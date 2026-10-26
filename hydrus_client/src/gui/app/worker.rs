//! Background import and download workers.
//!
//! The Qt GUI is single-threaded, so every long operation is submitted to one
//! worker thread and reported back through a channel. The application state
//! drains that channel from its Qt timer tick; widgets never touch worker
//! state directly.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_db_base::DBBase;
use regex::Regex;
use reqwest::Url;
use reqwest::blocking::Client;

use crate::client_options::ClientOptions;
use crate::controller::Controller;
use crate::db::{master, url_map};
use crate::files::ClientFilesManager;
use crate::gui::app::import::ImportContext;

const MAX_GALLERY_PAGES: usize = 120;
const MAX_GALLERY_FILES: usize = 2000;

#[derive(Debug)]
pub(crate) enum AsyncJob {
    ImportPaths {
        paths: Vec<PathBuf>,
    },
    DownloadUrls {
        job_id: String,
        urls: Vec<String>,
    },
    ProcessPages {
        job_id: String,
        urls: Vec<String>,
        filter: String,
    },
    UpdateOptions(ClientOptions),
    UpdateNetworkHeaders(Vec<(String, String)>),
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
        let context = WorkerContext::from_controller(&controller);

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

    pub(crate) fn drain(&self) -> Vec<AsyncEvent> {
        self.receiver.try_iter().collect()
    }
}

#[derive(Clone)]
struct WorkerContext {
    db: DBBase,
    files_manager: ClientFilesManager,
    options: ClientOptions,
    headers: Vec<(String, String)>,
    paused: Arc<AtomicBool>,
}

impl WorkerContext {
    fn from_controller(controller: &Controller) -> Self {
        Self {
            db: controller.db().db().clone(),
            files_manager: controller.files_manager(),
            options: controller.options(),
            headers: controller
                .domain_manager()
                .read()
                .map(|manager| manager.get_custom_header_pairs())
                .unwrap_or_default(),
            paused: controller.network_paused_flag(),
        }
    }

    fn importer(&self) -> Result<ImportContext, String> {
        ImportContext::new(self.db.clone(), self.files_manager.clone())
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
            AsyncJob::ImportPaths { paths } => run_import(&context, &events, paths),
            AsyncJob::DownloadUrls { job_id, urls } => {
                run_download_urls(&context, &events, &job_id, urls)
            }
            AsyncJob::ProcessPages {
                job_id,
                urls,
                filter,
            } => run_process_pages(&context, &events, &job_id, urls, &filter),
            AsyncJob::UpdateOptions(options) => {
                context.options = options;
            }
            AsyncJob::UpdateNetworkHeaders(headers) => {
                context.headers = headers;
            }
        }
    }
}

fn run_import(context: &WorkerContext, events: &Sender<AsyncEvent>, paths: Vec<PathBuf>) {
    let total = paths.len();
    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();
    let mut hashes = Vec::new();

    let importer = match context.importer() {
        Ok(importer) => importer,
        Err(error) => {
            let _ = events.send(AsyncEvent::ImportFinished {
                imported: 0,
                skipped: total,
                errors: vec![error],
                hashes: Vec::new(),
            });
            return;
        }
    };

    for (index, path) in paths.iter().enumerate() {
        let message = format!(
            "importing {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );

        match importer.import_file(path) {
            Ok(hash) => {
                imported += 1;
                hashes.push(hash);
            }
            Err(error) => {
                skipped += 1;
                errors.push(format!("{}: {error}", path.display()));
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
    });
}

fn run_download_urls(
    context: &WorkerContext,
    events: &Sender<AsyncEvent>,
    job_id: &str,
    urls: Vec<String>,
) {
    let _ = events.send(AsyncEvent::Started {
        job_id: job_id.to_string(),
        total: urls.len(),
    });

    let mut imported = 0;
    let mut failed = 0;
    let mut hashes = Vec::new();

    for url in urls {
        let _ = events.send(AsyncEvent::ItemStarted {
            job_id: job_id.to_string(),
            url: url.clone(),
        });

        if context.paused.load(Ordering::Relaxed) {
            let _ = events.send(AsyncEvent::Item {
                job_id: job_id.to_string(),
                url,
                ok: false,
                note: "network is paused".to_string(),
                hash: None,
            });
            failed += 1;
            continue;
        }

        let (ok, note, hash) = download_url(context, &url);

        if ok {
            imported += 1;
        } else {
            failed += 1;
        }

        if let Some(hash) = &hash {
            hashes.push(hash.clone());
        }

        let _ = events.send(AsyncEvent::Item {
            job_id: job_id.to_string(),
            url,
            ok,
            note,
            hash,
        });
    }

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

    for url in urls {
        if context.paused.load(Ordering::Relaxed) {
            let _ = events.send(AsyncEvent::Message {
                job_id: job_id.to_string(),
                message: "network is paused".to_string(),
            });
            break;
        }

        if is_media_url(&url) {
            if matches_filter(&url, filter) {
                file_urls.insert(url);
            }
            continue;
        }

        let _ = events.send(AsyncEvent::Message {
            job_id: job_id.to_string(),
            message: format!("fetching page {url}"),
        });

        let Ok((body, _)) = http_get(&client, &url) else {
            let _ = events.send(AsyncEvent::Message {
                job_id: job_id.to_string(),
                message: format!("could not fetch {url}"),
            });
            continue;
        };

        let html = String::from_utf8_lossy(&body);
        let page_links = extract_links(&html, &url);

        let direct: Vec<String> = page_links
            .iter()
            .filter(|link| is_media_url(link) && matches_filter(link, filter))
            .cloned()
            .collect();

        if !direct.is_empty() {
            for link in direct {
                file_urls.insert(link);
            }
            continue;
        }

        for page_link in page_links {
            if page_count >= MAX_GALLERY_PAGES || file_urls.len() >= MAX_GALLERY_FILES {
                break;
            }

            if should_follow_page_link(&page_link, &url) {
                page_count += 1;

                let _ = events.send(AsyncEvent::Message {
                    job_id: job_id.to_string(),
                    message: format!("fetching gallery page {page_link}"),
                });

                let Ok((page_body, _)) = http_get(&client, &page_link) else {
                    continue;
                };

                for link in extract_links(&String::from_utf8_lossy(&page_body), &page_link) {
                    if is_media_url(&link) && matches_filter(&link, filter) {
                        file_urls.insert(link);
                    }
                }
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

    let mut imported = 0;
    let mut failed = 0;
    let mut hashes = Vec::new();

    for url in file_urls {
        if context.paused.load(Ordering::Relaxed) {
            let _ = events.send(AsyncEvent::Message {
                job_id: job_id.to_string(),
                message: "network is paused".to_string(),
            });
            break;
        }

        let _ = events.send(AsyncEvent::ItemStarted {
            job_id: job_id.to_string(),
            url: url.clone(),
        });

        let (ok, note, hash) = download_url(context, &url);

        if ok {
            imported += 1;
        } else {
            failed += 1;
        }

        if let Some(hash) = &hash {
            hashes.push(hash.clone());
        }

        let _ = events.send(AsyncEvent::Item {
            job_id: job_id.to_string(),
            url,
            ok,
            note,
            hash,
        });
    }

    let _ = events.send(AsyncEvent::Finished {
        job_id: job_id.to_string(),
        imported,
        failed,
        message: format!("{imported} imported, {failed} failed from {page_count} pages"),
        hashes,
    });
}

fn download_url(context: &WorkerContext, url: &str) -> (bool, String, Option<Vec<u8>>) {
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

    let (bytes, _content_type) = match http_get(&client, url) {
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

    let importer = match context.importer() {
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

fn http_get(client: &Client, url: &str) -> Result<(Vec<u8>, String), String> {
    let response = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?;

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

    Ok((bytes, content_type))
}

/// Extracts href/src links from a page and resolves them against its URL.
pub fn extract_links(html: &str, base: &str) -> Vec<String> {
    static LINK: OnceLock<Regex> = OnceLock::new();

    let regex = LINK.get_or_init(|| {
        Regex::new(r#"(?is)(?:href|src|data-src|data-original)\s*=\s*["']([^"']+)["']"#)
            .expect("valid link regex")
    });

    let Ok(base) = Url::parse(base) else {
        return Vec::new();
    };

    let mut links = Vec::new();
    let mut seen = BTreeSet::new();

    for capture in regex.captures_iter(html) {
        let Some(raw) = capture.get(1).map(|value| value.as_str().trim()) else {
            continue;
        };

        if raw.is_empty()
            || raw.starts_with('#')
            || raw.starts_with("javascript:")
            || raw.starts_with("mailto:")
            || raw.starts_with("data:")
        {
            continue;
        }

        let raw = raw
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'");

        let Ok(mut url) = base.join(&raw) else {
            continue;
        };

        if !matches!(url.scheme(), "http" | "https") {
            continue;
        }

        url.set_fragment(None);

        let text = url.to_string();

        if seen.insert(text.clone()) {
            links.push(text);
        }
    }

    links
}

pub fn is_media_url(url: &str) -> bool {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();

    let extension = path.rsplit('.').next().unwrap_or_default();

    matches!(
        extension,
        "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "webp"
            | "bmp"
            | "tif"
            | "tiff"
            | "ico"
            | "qoi"
            | "avif"
            | "heic"
            | "jxl"
            | "svg"
            | "mp4"
            | "webm"
            | "mkv"
            | "mov"
            | "m4v"
            | "avi"
            | "mp3"
            | "flac"
            | "wav"
            | "ogg"
            | "opus"
            | "m4a"
            | "pdf"
            | "zip"
            | "rar"
            | "7z"
    )
}

fn should_follow_page_link(link: &str, gallery_url: &str) -> bool {
    if link == gallery_url || is_media_url(link) {
        return false;
    }

    let (Ok(link_url), Ok(gallery)) = (Url::parse(link), Url::parse(gallery_url)) else {
        return false;
    };

    if link_url
        .host_str()
        .map(|host| host.trim_start_matches("www."))
        != gallery
            .host_str()
            .map(|host| host.trim_start_matches("www."))
    {
        return false;
    }

    let path = link
        .split(['?', '#'])
        .next()
        .unwrap_or(link)
        .to_ascii_lowercase();

    let extension = path.rsplit('.').next().unwrap_or_default();

    !matches!(
        extension,
        "css" | "js" | "json" | "xml" | "rss" | "ico" | "woff" | "woff2" | "ttf" | "otf"
    )
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
    fn extracts_and_resolves_links() {
        let links = extract_links(
            r#"<a href="/post/1">one</a><img src="https://cdn.example.com/a.jpg"><a href='javascript:x'>no</a>"#,
            "https://example.com/gallery",
        );

        assert_eq!(
            links,
            vec![
                "https://example.com/post/1".to_string(),
                "https://cdn.example.com/a.jpg".to_string()
            ]
        );
    }

    #[test]
    fn recognises_media_extensions() {
        assert!(is_media_url("https://x/a.JPG?size=1"));
        assert!(is_media_url("https://x/a.webm"));
        assert!(!is_media_url("https://x/a.css"));
        assert!(!is_media_url("https://x/post/1"));
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
