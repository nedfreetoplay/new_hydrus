//! Background import and download workers.
//!
//! The Qt GUI is single-threaded, so every long operation is submitted to one
//! worker thread and reported back through a channel. The application state
//! drains that channel from its Qt timer tick; widgets never touch worker
//! state directly.

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;

use hydrus_core::files::hydrus_file_handling as core_files;
use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_db_base::DBBase;
use reqwest::blocking::Client;

use crate::client_options::ClientOptions;
use crate::controller::Controller;
use crate::db::{master, url_map};
use crate::files::ClientFilesManager;
use crate::gui::app::import::ImportContext;
use crate::networking::NetworkContext;
use crate::networking::client_networking_bandwidth::NetworkBandwidthManager;
use crate::networking::download_pools::{DownloadPoolSettings, domain_of, group_by_domain};
use crate::parsing::gallery::{
    is_media_url, parse_gallery_page, parse_post_page, parser_by_id, parser_for_url,
};

const MAX_GALLERY_PAGES: usize = 120;
const MAX_GALLERY_FILES: usize = 2000;

#[derive(Debug)]
pub(crate) enum AsyncJob {
    ImportPaths {
        paths: Vec<PathBuf>,
        /// Set when the batch came from an import folder, so post-import
        /// move/delete actions can be applied to that folder.
        folder: Option<String>,
        /// Tag service key -> filename tagging options, from the import folder
        /// or local import dialog.
        filename_tagging:
            std::collections::BTreeMap<Vec<u8>, crate::importing::FilenameTaggingOptions>,
        /// Delete each source file after it imports successfully (the review
        /// panel's "delete original files" option).
        delete_after_success: bool,
    },
    DownloadUrls {
        job_id: String,
        urls: Vec<String>,
        /// Effective import options for this page or subscription.
        import_options: crate::importing::ImportOptionsContainer,
    },
    ProcessPages {
        job_id: String,
        urls: Vec<String>,
        filter: String,
        site: String,
        /// Effective import options for this page or subscription.
        import_options: crate::importing::ImportOptionsContainer,
    },
    UpdateOptions(ClientOptions),
    UpdateNetworkHeaders(Vec<(String, String)>),
    UpdatePools(DownloadPoolSettings),
    ExportFolder {
        name: String,
        folder: super::export::ExportFolderEntry,
        hashes: Option<Vec<Vec<u8>>>,
    },
}

#[derive(Debug)]
pub(crate) enum AsyncEvent {
    Started {
        job_id: String,
        total: usize,
    },
    Discovered {
        job_id: String,
        urls: Vec<String>,
    },
    ItemStarted {
        job_id: String,
        url: String,
    },
    Message {
        job_id: String,
        message: String,
    },
    /// One gallery or post URL has been processed; feeds the gallery seed log.
    GallerySeed {
        job_id: String,
        url: String,
        status: String,
        note: String,
    },
    Item {
        job_id: String,
        url: String,
        ok: bool,
        note: String,
        hash: Option<Vec<u8>>,
    },
    Finished {
        job_id: String,
        imported: usize,
        failed: usize,
        message: String,
        hashes: Vec<Vec<u8>>,
    },
    ImportProgress {
        done: usize,
        total: usize,
        imported: usize,
        failed: usize,
        message: String,
    },
    ImportFinished {
        imported: usize,
        skipped: usize,
        errors: Vec<String>,
        hashes: Vec<Vec<u8>>,
        folder: Option<String>,
        results: Vec<(PathBuf, bool)>,
    },
    ExportFinished {
        name: String,
        result: super::export::ExportResult,
        checked_at: i64,
    },
}

pub(crate) struct AsyncRuntime {
    sender: Sender<AsyncJob>,
    receiver: Receiver<AsyncEvent>,
}

impl AsyncRuntime {
    pub(crate) fn new(controller: std::sync::Arc<Controller>) -> Self {
        let (job_sender, job_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();
        let context = WorkerContext::from_controller(controller.clone());

        thread::Builder::new()
            .name("hydrus-import-download".to_string())
            .spawn(move || worker_loop(context, job_receiver, event_sender))
            .expect("could not start the import/download worker");

        Self {
            sender: job_sender,
            receiver: event_receiver,
        }
    }

    pub(crate) fn submit(&self, job: AsyncJob) -> Result<(), String> {
        self.sender
            .send(job)
            .map_err(|_| "the import/download worker stopped".to_string())
    }

    pub(crate) fn update_options(&self, options: ClientOptions) {
        let _ = self.sender.send(AsyncJob::UpdateOptions(options));
    }

    pub(crate) fn update_network_headers(&self, headers: Vec<(String, String)>) {
        let _ = self.sender.send(AsyncJob::UpdateNetworkHeaders(headers));
    }

    pub(crate) fn update_pools(&self, pools: DownloadPoolSettings) {
        let _ = self.sender.send(AsyncJob::UpdatePools(pools));
    }

    pub(crate) fn drain(&self) -> Vec<AsyncEvent> {
        self.receiver.try_iter().collect()
    }
}

#[derive(Clone)]
struct WorkerContext {
    controller: Arc<Controller>,
    db: DBBase,
    files_manager: ClientFilesManager,
    options: ClientOptions,
    headers: Vec<(String, String)>,
    paused: Arc<AtomicBool>,
    bandwidth: Arc<RwLock<NetworkBandwidthManager>>,
    pools: DownloadPoolSettings,
}

impl WorkerContext {
    fn from_controller(controller: Arc<Controller>) -> Self {
        Self {
            controller: controller.clone(),
            db: controller.db().db().clone(),
            files_manager: controller.files_manager(),
            options: controller.options(),
            headers: controller
                .domain_manager()
                .read()
                .map(|manager| manager.get_custom_header_pairs())
                .unwrap_or_default(),
            paused: controller.network_paused_flag(),
            bandwidth: controller.bandwidth_manager_arc(),
            pools: controller.download_pool_settings(),
        }
    }

    fn importer(&self) -> Result<ImportContext, String> {
        let manager = crate::importing::ImportOptionsManager::load_from_options(&self.options);
        let import_options = manager.get_effective("local_import", None);

        ImportContext::new_with_options(self.db.clone(), self.files_manager.clone(), import_options)
    }

    fn importer_with_options(
        &self,
        import_options: &crate::importing::ImportOptionsContainer,
    ) -> Result<ImportContext, String> {
        ImportContext::new_with_options(
            self.db.clone(),
            self.files_manager.clone(),
            import_options.clone(),
        )
    }

    fn client(&self) -> Result<Client, String> {
        let timeout = self.options.get_integer("network_timeout").max(5) as u64;

        let mut builder = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .user_agent(crate::client_defaults::DEFAULT_USER_AGENT);

        let mut header_map = reqwest::header::HeaderMap::new();

        for (name, value) in &self.headers {
            let (Ok(name), Ok(value)) = (
                reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) else {
                continue;
            };

            header_map.insert(name, value);
        }

        if !header_map.is_empty() {
            builder = builder.default_headers(header_map);
        }

        if let Some(proxy) = self
            .options
            .get_noneable_string("https_proxy")
            .or_else(|| self.options.get_noneable_string("http_proxy"))
            .filter(|proxy| !proxy.trim().is_empty())
        {
            let proxy = reqwest::Proxy::all(proxy.trim())
                .map_err(|error| format!("invalid proxy: {error}"))?;
            builder = builder.proxy(proxy);
        }

        builder
            .build()
            .map_err(|error| format!("could not build the network client: {error}"))
    }
}

fn worker_loop(mut context: WorkerContext, jobs: Receiver<AsyncJob>, events: Sender<AsyncEvent>) {
    while let Ok(job) = jobs.recv() {
        match job {
            AsyncJob::ImportPaths {
                paths,
                folder,
                filename_tagging,
                delete_after_success,
            } => run_import(
                &context,
                &events,
                paths,
                folder,
                filename_tagging,
                delete_after_success,
            ),
            AsyncJob::DownloadUrls {
                job_id,
                urls,
                import_options,
            } => run_download_urls(&context, &events, &job_id, urls, &import_options),
            AsyncJob::ProcessPages {
                job_id,
                urls,
                filter,
                site,
                import_options,
            } => run_process_pages(
                &context,
                &events,
                &job_id,
                urls,
                &filter,
                &site,
                &import_options,
            ),
            AsyncJob::UpdateOptions(options) => {
                context.options = options;
            }
            AsyncJob::UpdateNetworkHeaders(headers) => {
                context.headers = headers;
            }
            AsyncJob::UpdatePools(pools) => {
                context.pools = pools;
            }
            AsyncJob::ExportFolder {
                name,
                folder,
                hashes,
            } => {
                let result = super::export::execute_selected(
                    &context.controller,
                    &folder,
                    hashes.as_deref(),
                );

                let _ = events.send(AsyncEvent::ExportFinished {
                    name,
                    result,
                    checked_at: hydrus_core::hydrus_time::get_now(),
                });
            }
        }
    }
}

fn run_import(
    context: &WorkerContext,
    events: &Sender<AsyncEvent>,
    paths: Vec<PathBuf>,
    folder: Option<String>,
    filename_tagging: std::collections::BTreeMap<Vec<u8>, crate::importing::FilenameTaggingOptions>,
    delete_after_success: bool,
) {
    let total = paths.len();
    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();
    let mut hashes = Vec::new();
    let mut results = Vec::with_capacity(total);

    let importer = match context
        .importer()
        .map(|importer| importer.with_filename_tagging(filename_tagging))
    {
        Ok(importer) => importer,
        Err(error) => {
            let _ = events.send(AsyncEvent::ImportFinished {
                imported: 0,
                skipped: total,
                errors: vec![error],
                hashes: Vec::new(),
                folder,
                results: paths.into_iter().map(|path| (path, false)).collect(),
            });
            return;
        }
    };

    for (index, path) in paths.iter().enumerate() {
        let message = format!(
            "importing {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        let known_before = delete_after_success
            && core_files::get_hash_from_path(path)
                .ok()
                .is_some_and(|hash| {
                    master::find_hash_id(importer.db(), &hash)
                        .ok()
                        .flatten()
                        .is_some()
                });

        match importer.import_file(path) {
            Ok(hash) => {
                imported += 1;
                hashes.push(hash);
                results.push((path.clone(), true));

                if delete_after_success && !known_before {
                    if let Err(error) = std::fs::remove_file(path) {
                        errors.push(format!(
                            "{}: could not delete the original: {error}",
                            path.display()
                        ));
                    }
                }
            }
            Err(error) => {
                skipped += 1;
                errors.push(format!("{}: {error}", path.display()));
                results.push((path.clone(), false));
            }
        }

        let _ = events.send(AsyncEvent::ImportProgress {
            done: index + 1,
            total,
            imported,
            failed: skipped,
            message,
        });
    }

    let _ = events.send(AsyncEvent::ImportFinished {
        imported,
        skipped,
        errors,
        hashes,
        folder,
        results,
    });
}

fn run_download_urls(
    context: &WorkerContext,
    events: &Sender<AsyncEvent>,
    job_id: &str,
    urls: Vec<String>,
    import_options: &crate::importing::ImportOptionsContainer,
) {
    let _ = events.send(AsyncEvent::Started {
        job_id: job_id.to_string(),
        total: urls.len(),
    });

    let (imported, failed, hashes) = run_pool(context, events, job_id, urls, import_options);

    let _ = events.send(AsyncEvent::Finished {
        job_id: job_id.to_string(),
        imported,
        failed,
        message: format!("{imported} imported, {failed} failed"),
        hashes,
    });
}

fn run_process_pages(
    context: &WorkerContext,
    events: &Sender<AsyncEvent>,
    job_id: &str,
    urls: Vec<String>,
    filter: &str,
    site: &str,
    import_options: &crate::importing::ImportOptionsContainer,
) {
    let client = match context.client() {
        Ok(client) => client,
        Err(error) => {
            let _ = events.send(AsyncEvent::Finished {
                job_id: job_id.to_string(),
                imported: 0,
                failed: 1,
                message: error,
                hashes: Vec::new(),
            });
            return;
        }
    };

    let mut file_urls: BTreeSet<String> = BTreeSet::new();
    let mut page_count = 0usize;
    let mut page_queue: VecDeque<String> = urls.into_iter().collect();
    let mut seen_pages: BTreeSet<String> = BTreeSet::new();

    while let Some(page_url) = page_queue.pop_front() {
        if page_count >= MAX_GALLERY_PAGES || file_urls.len() >= MAX_GALLERY_FILES {
            break;
        }

        if context.paused.load(Ordering::Relaxed) {
            let _ = events.send(AsyncEvent::Message {
                job_id: job_id.to_string(),
                message: "network is paused".to_string(),
            });
            break;
        }

        if is_media_url(&page_url) {
            if matches_filter(&page_url, filter) {
                file_urls.insert(page_url);
            }
            continue;
        }

        let key = normalise_page_url(&page_url);

        if !seen_pages.insert(key) {
            continue;
        }

        page_count += 1;

        let _ = events.send(AsyncEvent::Message {
            job_id: job_id.to_string(),
            message: format!("fetching page {page_url}"),
        });
        let _ = events.send(AsyncEvent::GallerySeed {
            job_id: job_id.to_string(),
            url: page_url.clone(),
            status: "working".to_string(),
            note: "fetching".to_string(),
        });

        let Ok((body, _)) = http_get(context, &client, &page_url) else {
            let _ = events.send(AsyncEvent::Message {
                job_id: job_id.to_string(),
                message: format!("could not fetch {page_url}"),
            });
            let _ = events.send(AsyncEvent::GallerySeed {
                job_id: job_id.to_string(),
                url: page_url.clone(),
                status: "failed".to_string(),
                note: "could not fetch".to_string(),
            });
            continue;
        };

        let html = String::from_utf8_lossy(&body);
        let parser = if site.trim().is_empty() || site.eq_ignore_ascii_case("auto") {
            parser_for_url(&page_url)
        } else {
            parser_by_id(site)
        };
        let parsed = parse_gallery_page(&html, &page_url, parser);
        let page_file_count = parsed.files.len();

        let _ = events.send(AsyncEvent::GallerySeed {
            job_id: job_id.to_string(),
            url: page_url.clone(),
            status: "success".to_string(),
            note: format!("{page_file_count} file(s) found"),
        });

        if parsed.is_post || parsed.posts.is_empty() {
            for link in parsed.files {
                if file_urls.len() >= MAX_GALLERY_FILES {
                    break;
                }

                if matches_filter(&link, filter) {
                    file_urls.insert(link);
                }
            }
        } else if parser.id == crate::parsing::gallery::GalleryParserId::Generic {
            for link in parsed.files {
                if file_urls.len() >= MAX_GALLERY_FILES {
                    break;
                }

                if matches_filter(&link, filter) {
                    file_urls.insert(link);
                }
            }
        }

        if !parsed.is_post && !parsed.posts.is_empty() {
            for post_url in parsed.posts {
                if page_count >= MAX_GALLERY_PAGES || file_urls.len() >= MAX_GALLERY_FILES {
                    break;
                }

                if context.paused.load(Ordering::Relaxed) {
                    break;
                }

                let post_key = normalise_page_url(&post_url);

                if !seen_pages.insert(post_key) {
                    continue;
                }

                page_count += 1;

                let _ = events.send(AsyncEvent::Message {
                    job_id: job_id.to_string(),
                    message: format!("fetching post {post_url}"),
                });
                let _ = events.send(AsyncEvent::GallerySeed {
                    job_id: job_id.to_string(),
                    url: post_url.clone(),
                    status: "working".to_string(),
                    note: "fetching".to_string(),
                });

                let Ok((post_body, _)) = http_get(context, &client, &post_url) else {
                    let _ = events.send(AsyncEvent::GallerySeed {
                        job_id: job_id.to_string(),
                        url: post_url.clone(),
                        status: "failed".to_string(),
                        note: "could not fetch".to_string(),
                    });
                    continue;
                };

                let post_links =
                    parse_post_page(&String::from_utf8_lossy(&post_body), &post_url, parser);
                let post_file_count = post_links.len();

                let _ = events.send(AsyncEvent::GallerySeed {
                    job_id: job_id.to_string(),
                    url: post_url.clone(),
                    status: "success".to_string(),
                    note: format!("{post_file_count} file(s) found"),
                });

                for link in post_links {
                    if file_urls.len() >= MAX_GALLERY_FILES {
                        break;
                    }

                    if matches_filter(&link, filter) {
                        file_urls.insert(link);
                    }
                }
            }
        }

        for next_page in parsed.next_pages {
            if !seen_pages.contains(&normalise_page_url(&next_page)) {
                page_queue.push_back(next_page);
            }
        }
    }

    let file_urls: Vec<String> = file_urls.into_iter().take(MAX_GALLERY_FILES).collect();

    let _ = events.send(AsyncEvent::Discovered {
        job_id: job_id.to_string(),
        urls: file_urls.clone(),
    });
    let _ = events.send(AsyncEvent::Started {
        job_id: job_id.to_string(),
        total: file_urls.len(),
    });

    let (imported, failed, hashes) = run_pool(context, events, job_id, file_urls, import_options);

    let _ = events.send(AsyncEvent::Finished {
        job_id: job_id.to_string(),
        imported,
        failed,
        message: format!("{imported} imported, {failed} failed from {page_count} pages"),
        hashes,
    });
}

/// Runs file downloads through per-domain pools. Each domain gets its own
/// queue and up to its configured number of concurrent workers; a shared gate
/// enforces the global `max_network_jobs` option.
fn run_pool(
    context: &WorkerContext,
    events: &Sender<AsyncEvent>,
    job_id: &str,
    urls: Vec<String>,
    import_options: &crate::importing::ImportOptionsContainer,
) -> (usize, usize, Vec<Vec<u8>>) {
    let groups = group_by_domain(urls);
    let global = Arc::new(PoolGate::new(
        context.options.get_integer("max_network_jobs").max(1) as usize,
    ));
    let default_domain = context
        .options
        .get_integer("max_network_jobs_per_domain")
        .max(1) as usize;
    let imported = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let hashes = Arc::new(Mutex::new(Vec::new()));

    thread::scope(|scope| {
        for (domain, urls) in groups {
            let limit = context
                .pools
                .limit_for(&domain, default_domain)
                .min(urls.len().max(1));
            let queue = Arc::new(Mutex::new(VecDeque::from(urls)));

            for _ in 0..limit {
                let queue = queue.clone();
                let global = global.clone();
                let imported = imported.clone();
                let failed = failed.clone();
                let hashes = hashes.clone();
                let events = events.clone();
                let context = context.clone();
                let job_id = job_id.to_string();
                let import_options = import_options.clone();

                scope.spawn(move || {
                    loop {
                        let next = {
                            let mut queue = queue.lock().unwrap();
                            queue.pop_front()
                        };

                        let Some(url) = next else {
                            return;
                        };

                        if context.paused.load(Ordering::Relaxed) {
                            let _ = events.send(AsyncEvent::Item {
                                job_id: job_id.clone(),
                                url,
                                ok: false,
                                note: "network is paused".to_string(),
                                hash: None,
                            });
                            failed.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }

                        let _ = events.send(AsyncEvent::ItemStarted {
                            job_id: job_id.clone(),
                            url: url.clone(),
                        });

                        global.acquire();
                        let (ok, note, hash) = download_url(&context, &url, &import_options);
                        global.release();

                        if ok {
                            imported.fetch_add(1, Ordering::Relaxed);
                        } else {
                            failed.fetch_add(1, Ordering::Relaxed);
                        }

                        if let Some(hash) = &hash {
                            hashes.lock().unwrap().push(hash.clone());
                        }

                        let _ = events.send(AsyncEvent::Item {
                            job_id: job_id.clone(),
                            url,
                            ok,
                            note,
                            hash,
                        });
                    }
                });
            }
        }
    });

    let hashes = hashes
        .lock()
        .map(|mut hashes| std::mem::take(&mut *hashes))
        .unwrap_or_default();

    (
        imported.load(Ordering::Relaxed),
        failed.load(Ordering::Relaxed),
        hashes,
    )
}

struct PoolGate {
    active: Mutex<usize>,
    wake: Condvar,
    max: usize,
}

impl PoolGate {
    fn new(max: usize) -> Self {
        Self {
            active: Mutex::new(0),
            wake: Condvar::new(),
            max: max.max(1),
        }
    }

    fn acquire(&self) {
        let mut active = self.active.lock().unwrap();

        while *active >= self.max {
            active = self.wake.wait(active).unwrap();
        }

        *active += 1;
    }

    fn release(&self) {
        let mut active = self.active.lock().unwrap();
        *active = active.saturating_sub(1);
        self.wake.notify_one();
    }
}

fn normalise_page_url(url: &str) -> String {
    url.split('#')
        .next()
        .unwrap_or(url)
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn download_url(
    context: &WorkerContext,
    url: &str,
    import_options: &crate::importing::ImportOptionsContainer,
) -> (bool, String, Option<Vec<u8>>) {
    if context.paused.load(Ordering::Relaxed) {
        return (false, "network is paused".to_string(), None);
    }

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return (false, "not an http(s) url".to_string(), None);
    }

    let client = match context.client() {
        Ok(client) => client,
        Err(error) => return (false, error, None),
    };

    let (bytes, _content_type) = match http_get(context, &client, url) {
        Ok(value) => value,
        Err(error) => return (false, error, None),
    };

    let (mut file, path) = match hydrus_core::hydrus_temp::get_temp_path("url_import", ".bin", None)
    {
        Ok(value) => value,
        Err(error) => return (false, error.to_string(), None),
    };

    use std::io::Write;

    if let Err(error) = file.write_all(&bytes) {
        return (false, error.to_string(), None);
    }

    drop(file);

    let importer = match context.importer_with_options(import_options) {
        Ok(importer) => importer,
        Err(error) => {
            let _ = std::fs::remove_file(&path);
            return (false, error, None);
        }
    };

    let already_known = hydrus_core::files::hydrus_file_handling::get_hash_from_path(&path)
        .ok()
        .and_then(|hash| master::find_hash_id(&context.db, &hash).ok().flatten())
        .is_some();

    let result = importer.import_file_from(&path, Some(url));
    let _ = std::fs::remove_file(path);

    match result {
        Ok(hash) => {
            let note = if already_known {
                "already in database".to_string()
            } else {
                format!(
                    "downloaded and imported ({})",
                    mime_note(&context.db, &hash)
                )
            };

            (true, note, Some(hash))
        }
        Err(error) => (false, error, None),
    }
}

fn mime_note(db: &DBBase, hash: &[u8]) -> String {
    let Some(hash_id) = master::find_hash_id(db, hash).ok().flatten() else {
        return "file".to_string();
    };

    crate::db::files_metadata::get_file_info(db, hash_id)
        .ok()
        .flatten()
        .map(|info| info.mime.as_str().to_string())
        .unwrap_or_else(|| "file".to_string())
}

fn http_get(
    context: &WorkerContext,
    client: &Client,
    url: &str,
) -> Result<(Vec<u8>, String), String> {
    if !wait_for_bandwidth(context, url) {
        return Err("network is paused".to_string());
    }

    let response = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status());

    report_request_used(context, url);

    let response = response.map_err(|error| error.to_string())?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();

    let bytes = response
        .bytes()
        .map_err(|error| error.to_string())?
        .to_vec();

    report_data_used(context, url, bytes.len());

    Ok((bytes, content_type))
}

fn bandwidth_context(url: &str) -> NetworkContext {
    let domain = domain_of(url);

    if domain.is_empty() {
        NetworkContext::default_context()
    } else {
        NetworkContext::for_domain(domain)
    }
}

fn wait_for_bandwidth(context: &WorkerContext, url: &str) -> bool {
    let network_context = bandwidth_context(url);

    loop {
        if context.paused.load(Ordering::Relaxed) {
            return false;
        }

        let (allowed, waiting) = {
            let mut manager = context.bandwidth.write().unwrap();

            (
                manager.can_start_request(&network_context, 5),
                manager.get_waiting_estimate(&network_context),
            )
        };

        if allowed {
            return true;
        }

        thread::sleep(std::time::Duration::from_secs(waiting.clamp(1, 30) as u64));
    }
}

fn report_request_used(context: &WorkerContext, url: &str) {
    if let Ok(mut manager) = context.bandwidth.write() {
        manager.report_request_used(&bandwidth_context(url), 1);
    }
}

fn report_data_used(context: &WorkerContext, url: &str, bytes: usize) {
    if let Ok(mut manager) = context.bandwidth.write() {
        manager.report_data_used(&bandwidth_context(url), bytes as i64);
    }
}

fn matches_filter(url: &str, filter: &str) -> bool {
    let filter = filter.trim();

    filter.is_empty()
        || url
            .to_ascii_lowercase()
            .contains(&filter.to_ascii_lowercase())
}

/// Replaces the `{query}` placeholder in a gallery URL. When there is no
/// placeholder, the search text is ignored.
pub fn gallery_url(template: &str, query: &str) -> String {
    let template = template.trim();

    if !template.contains("{query}") {
        return template.to_string();
    }

    let encoded: String = query
        .trim()
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => character.to_string(),
            ' ' => "+".to_string(),
            other => {
                let mut buffer = [0u8; 4];
                let text = other.encode_utf8(&mut buffer);
                text.as_bytes()
                    .iter()
                    .map(|byte| format!("%{byte:02X}"))
                    .collect::<String>()
            }
        })
        .collect();

    template.replace("{query}", &encoded)
}

/// Writes a URL mapping after an import. Used by tests and by the async path.
pub fn record_source_url(db: &DBBase, hash: &[u8], url: &str) {
    let Ok(Some(hash_id)) = master::find_hash_id(db, hash) else {
        return;
    };

    let _ = url_map::add_url(db, hash_id, url);
}

/// Converts a mime to the extension the downloaded temp file should use.
pub fn extension_for_mime(mime: Mime) -> &'static str {
    mime.extension()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_media_extensions() {
        assert!(is_media_url("https://x/a.JPG?size=1"));
        assert!(is_media_url("https://x/a.webm"));
        assert!(!is_media_url("https://x/a.css"));
        assert!(!is_media_url("https://x/post/1"));
    }

    #[test]
    fn pool_gate_limits_concurrency() {
        let gate = PoolGate::new(1);
        gate.acquire();
        gate.release();
        gate.acquire();
        gate.release();
    }

    #[test]
    fn gallery_query_is_percent_encoded() {
        assert_eq!(
            gallery_url("https://e/search?q={query}&page=1", "blue eyes"),
            "https://e/search?q=blue+eyes&page=1"
        );
        assert_eq!(
            gallery_url("https://e/gallery", "ignored"),
            "https://e/gallery"
        );
    }

    #[test]
    fn filters_are_case_insensitive() {
        assert!(matches_filter("https://x/BLUE/1.jpg", "blue"));
        assert!(!matches_filter("https://x/red/1.jpg", "blue"));
        assert!(matches_filter("https://x/red/1.jpg", ""));
    }
}
