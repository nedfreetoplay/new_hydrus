//! Import option value objects from `hydrus/client/importing/options/`.
//!
//! A container keeps one optional entry per option type. Missing entries mean
//! "inherit the default", which is how the manager builds the effective options
//! for a caller (global -> caller default -> specific importer).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use hydrus_core::hydrus_constants::{
    ANIMATIONS, APPLICATIONS, ARCHIVES, AUDIO, GENERAL_CLASSES_OF_FILETYPE, IMAGE_PROJECT_FILES,
    IMAGES, Mime, VIDEO,
};
use hydrus_core::hydrus_tags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Canonical order from `ImportOptionsConstants.py`.
pub const IMPORT_OPTIONS_TYPES_CANONICAL_ORDER: &[&str] = &[
    "prefetch",
    "file_filtering",
    "tag_filtering",
    "locations",
    "tags",
    "notes",
    "external_programs",
    "presentation",
];

pub const IMPORT_OPTIONS_TYPE_LABELS: &[(&str, &str)] = &[
    ("prefetch", "prefetch logic"),
    ("file_filtering", "file filtering"),
    ("tag_filtering", "tag filtering"),
    ("locations", "locations"),
    ("tags", "tags"),
    ("notes", "notes"),
    ("external_programs", "external programs"),
    ("presentation", "presentation"),
];

pub const IMPORT_OPTIONS_TYPE_DESCRIPTIONS: &[(&str, &str)] = &[
    (
        "prefetch",
        "Hydrus can skip a file or its metadata when it already knows the hash or URL. \
         Turning these checks off wastes bandwidth and is only useful for forced rechecks.",
    ),
    (
        "file_filtering",
        "Before a file is imported it is checked against these rules. A file that fails \
         gets an ignored status.",
    ),
    (
        "tag_filtering",
        "Parsed tags are checked against a blacklist and an optional whitelist. A blacklisted \
         tag, or no whitelisted tag, makes the import ignored.",
    ),
    (
        "locations",
        "Choose where incoming files are placed and whether they are archived automatically.",
    ),
    (
        "tags",
        "Choose which tag services receive tags parsed during the import.",
    ),
    (
        "notes",
        "Choose what to do with notes parsed during the import.",
    ),
    (
        "external_programs",
        "Run a program for every new or already-known file import. Be careful with this.",
    ),
    (
        "presentation",
        "Filter which imported files are shown in the importer UI.",
    ),
];

pub const CALLER_TYPES: &[(&str, &str, &str)] = &[
    (
        "global",
        "global",
        "The base that every importer inherits unless it sets something more specific.",
    ),
    (
        "client_api",
        "client api",
        "Files posted directly to the Client API, with no downloader page involved.",
    ),
    (
        "local_import",
        "local hard drive import",
        "All imports from a local hard drive, via an import page or an import folder.",
    ),
    (
        "local_import_folder",
        "import folder",
        "Import folders; useful for quieter presentation settings.",
    ),
    (
        "watcher_urls",
        "watchable urls",
        "Thread watcher work, if it needs different metadata filtering.",
    ),
    (
        "post_urls",
        "gallery/post urls",
        "Any gallery search or post URL in a downloader or subscription.",
    ),
    (
        "subscription",
        "subscription",
        "All subscription imports; a good place for quieter presentation.",
    ),
    (
        "url_class",
        "url class",
        "Defaults for URLs of a particular class; overrides most other defaults.",
    ),
    (
        "specific_importer",
        "specific importer",
        "Attached to one importer only and always wins over other defaults.",
    ),
];

/// Option types that do not make sense for a pure local import.
pub const DOWNLOADER_ONLY_TYPES: &[&str] = &["prefetch", "tag_filtering", "tags", "notes"];

pub const PREFETCH_DO_NOT_CHECK: i32 = 0;
pub const PREFETCH_DO_CHECK: i32 = 1;
pub const PREFETCH_DO_CHECK_AND_MATCHES_ARE_DISPOSITIVE: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrefetchImportOptions {
    pub preimport_hash_check_type: i32,
    pub preimport_url_check_type: i32,
    pub preimport_url_check_looks_for_neighbour_spam: bool,
    pub fetch_metadata_even_if_url_known: bool,
    pub fetch_metadata_even_if_hash_known: bool,
}

impl Default for PrefetchImportOptions {
    fn default() -> Self {
        Self {
            preimport_hash_check_type: PREFETCH_DO_CHECK_AND_MATCHES_ARE_DISPOSITIVE,
            preimport_url_check_type: PREFETCH_DO_CHECK,
            preimport_url_check_looks_for_neighbour_spam: true,
            fetch_metadata_even_if_url_known: false,
            fetch_metadata_even_if_hash_known: false,
        }
    }
}

impl PrefetchImportOptions {
    pub fn checks_hashes(&self) -> bool {
        self.preimport_hash_check_type != PREFETCH_DO_NOT_CHECK
    }

    pub fn checks_urls(&self) -> bool {
        self.preimport_url_check_type != PREFETCH_DO_NOT_CHECK
    }

    pub fn get_summary(&self, caller: &str) -> String {
        if is_local_caller(caller) {
            return String::new();
        }

        let mut statements = Vec::new();

        if !self.checks_hashes() && !self.checks_urls() {
            statements.push("WARNING: always redownloads files!".to_string());
        } else if !self.checks_hashes() {
            statements.push("WARNING: ignores hashes for file redownload checks!".to_string());
        } else if !self.checks_urls() {
            statements.push("WARNING: ignores URLs for file redownload checks!".to_string());
        }

        if self.fetch_metadata_even_if_hash_known && self.fetch_metadata_even_if_url_known {
            statements.push("always redownloads metadata!".to_string());
        } else if self.fetch_metadata_even_if_hash_known {
            statements.push("ignores hashes for metadata redownload checks".to_string());
        } else if self.fetch_metadata_even_if_url_known {
            statements.push("ignores URLs for metadata redownload checks".to_string());
        }

        if statements.is_empty() {
            return "looks good".to_string();
        }

        statements.join(", ")
    }
}

fn is_local_caller(caller: &str) -> bool {
    matches!(
        caller,
        "local_import" | "local_import_folder" | "client_api" | "favourites"
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileFilteringImportOptions {
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_resolution: Option<(u32, u32)>,
    pub max_resolution: Option<(u32, u32)>,
    pub max_gif_size: Option<u64>,
    #[serde(with = "mime_list")]
    pub allowed_mimes: Vec<Mime>,
    pub excludes_deleted: bool,
    pub allows_decompression_bombs: bool,
}

impl Default for FileFilteringImportOptions {
    fn default() -> Self {
        Self {
            min_size: None,
            max_size: None,
            min_resolution: None,
            max_resolution: None,
            max_gif_size: None,
            allowed_mimes: Vec::new(),
            excludes_deleted: true,
            allows_decompression_bombs: false,
        }
    }
}

impl FileFilteringImportOptions {
    /// `CheckFileIsValid( size, mime, width, height )`
    pub fn check_file_is_valid(
        &self,
        size: u64,
        mime: Mime,
        width: Option<u32>,
        height: Option<u32>,
    ) -> bool {
        if let Some(min_size) = self.min_size {
            if size < min_size {
                return false;
            }
        }

        if let Some(max_size) = self.max_size {
            if size > max_size {
                return false;
            }
        }

        if let Some(max_gif_size) = self.max_gif_size {
            if matches!(
                mime,
                Mime::AnimationGif | Mime::StaticGif | Mime::UndeterminedGif
            ) && size > max_gif_size
            {
                return false;
            }
        }

        if let Some((min_width, min_height)) = self.min_resolution {
            match (width, height) {
                (Some(width), Some(height)) if width >= min_width && height >= min_height => {}
                _ => return false,
            }
        }

        if let Some((max_width, max_height)) = self.max_resolution {
            match (width, height) {
                (Some(width), Some(height)) if width <= max_width && height <= max_height => {}
                _ => return false,
            }
        }

        if !self.allowed_mimes.is_empty()
            && !self
                .allowed_mimes
                .iter()
                .any(|allowed| mime_is_in_class(mime, *allowed))
        {
            return false;
        }

        true
    }

    pub fn get_summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(min_size) = self.min_size {
            parts.push(format!("min size {min_size}"));
        }
        if let Some(max_size) = self.max_size {
            parts.push(format!("max size {max_size}"));
        }
        if let Some(resolution) = self.min_resolution {
            parts.push(format!("min resolution {}x{}", resolution.0, resolution.1));
        }
        if let Some(resolution) = self.max_resolution {
            parts.push(format!("max resolution {}x{}", resolution.0, resolution.1));
        }
        if let Some(max_gif_size) = self.max_gif_size {
            parts.push(format!("max gif size {max_gif_size}"));
        }
        if !self.allowed_mimes.is_empty() {
            parts.push(format!(
                "filetypes {}",
                self.allowed_mimes
                    .iter()
                    .map(|mime| mime.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if self.excludes_deleted {
            parts.push("excludes previously deleted".to_string());
        }

        if parts.is_empty() {
            "allows all files".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Mime classes are stored as their general class (image, video, ...), so a
/// single checkbox covers every filetype in that class.
fn mime_is_in_class(mime: Mime, class: Mime) -> bool {
    if mime == class {
        return true;
    }

    let members: &[Mime] = match class {
        Mime::GeneralImage => IMAGES,
        Mime::GeneralAnimation => ANIMATIONS,
        Mime::GeneralVideo => VIDEO,
        Mime::GeneralAudio => AUDIO,
        Mime::GeneralApplication => APPLICATIONS,
        Mime::GeneralAppArchive => ARCHIVES,
        Mime::GeneralImageProject => IMAGE_PROJECT_FILES,
        Mime::GeneralFile => GENERAL_CLASSES_OF_FILETYPE,
        _ => return false,
    };

    members.contains(&mime)
}

mod mime_list {
    use super::*;

    pub fn serialize<S: Serializer>(mimes: &[Mime], serializer: S) -> Result<S::Ok, S::Error> {
        let names: Vec<&str> = mimes.iter().map(|mime| mime.as_str()).collect();

        names.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Mime>, D::Error> {
        let names: Vec<String> = Vec::deserialize(deserializer)?;

        Ok(names
            .iter()
            .filter_map(|name| Mime::from_str(name))
            .collect())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TagFilteringImportOptions {
    /// Tag slice -> `blacklist` or `whitelist`.
    pub rules: BTreeMap<String, String>,
    pub whitelist: Vec<String>,
}

impl TagFilteringImportOptions {
    pub fn allows_everything(&self) -> bool {
        self.rules.values().all(|rule| rule != "blacklist") && self.whitelist.is_empty()
    }

    pub fn get_summary(&self) -> String {
        let blacklists = self
            .rules
            .values()
            .filter(|rule| rule.as_str() == "blacklist")
            .count();

        if self.allows_everything() {
            return "allows all tags".to_string();
        }

        let mut statements = Vec::new();

        if blacklists > 0 {
            statements.push(format!("{blacklists} blacklist rule(s)"));
        }

        if !self.whitelist.is_empty() {
            statements.push(format!("{} whitelist tag(s)", self.whitelist.len()));
        }

        statements.join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocationImportOptions {
    /// Local file service names. Empty means the default local file service.
    pub destination_services: Vec<String>,
    pub automatically_archives: bool,
    pub associate_primary_urls: bool,
    pub associate_source_urls: bool,
    pub do_automatic_archive_on_already_in_db_files: bool,
    pub do_import_destinations_on_already_in_db_files: bool,
}

impl Default for LocationImportOptions {
    fn default() -> Self {
        Self {
            destination_services: Vec::new(),
            automatically_archives: false,
            associate_primary_urls: true,
            associate_source_urls: true,
            do_automatic_archive_on_already_in_db_files: true,
            do_import_destinations_on_already_in_db_files: false,
        }
    }
}

impl LocationImportOptions {
    pub fn get_summary(&self) -> String {
        let destinations = if self.destination_services.is_empty() {
            "the default local file service".to_string()
        } else {
            self.destination_services.join(", ")
        };

        let mut statements = vec![format!("imports to {destinations}")];

        if self.automatically_archives {
            statements.push("automatically archives".to_string());
        }

        if self.do_import_destinations_on_already_in_db_files {
            statements.push("adds destinations for files already in db".to_string());
        }

        statements.join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServiceTagImportOptions {
    pub get_tags: bool,
    pub additional_tags: Vec<String>,
    pub to_new_files: bool,
    pub to_already_in_inbox: bool,
    pub to_already_in_archive: bool,
    pub only_add_existing_tags: bool,
}

impl Default for ServiceTagImportOptions {
    fn default() -> Self {
        Self {
            get_tags: false,
            additional_tags: Vec::new(),
            to_new_files: true,
            to_already_in_inbox: true,
            to_already_in_archive: true,
            only_add_existing_tags: false,
        }
    }
}

impl ServiceTagImportOptions {
    pub fn get_additional_tags(&self) -> &[String] {
        &self.additional_tags
    }

    pub fn get_summary(&self) -> String {
        if !self.get_tags {
            return "no tags".to_string();
        }

        let mut statements = vec!["get tags".to_string()];

        if !self.additional_tags.is_empty() {
            statements.push(format!("{} additional tag(s)", self.additional_tags.len()));
        }

        if self.only_add_existing_tags {
            statements.push("only existing tags".to_string());
        }

        statements.join(", ")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagImportOptions {
    service_options: BTreeMap<Vec<u8>, ServiceTagImportOptions>,
}

impl TagImportOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_service_options(
        &mut self,
        service_key: impl Into<Vec<u8>>,
        options: ServiceTagImportOptions,
    ) {
        self.service_options.insert(service_key.into(), options);
    }

    pub fn get_service_options(&self, service_key: &[u8]) -> Option<&ServiceTagImportOptions> {
        self.service_options.get(service_key)
    }

    pub fn get_service_options_mut(
        &mut self,
        service_key: &[u8],
    ) -> Option<&mut ServiceTagImportOptions> {
        self.service_options.get_mut(service_key)
    }

    pub fn service_keys(&self) -> Vec<&[u8]> {
        self.service_options.keys().map(Vec::as_slice).collect()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&Vec<u8>, &ServiceTagImportOptions)> {
        self.service_options.iter()
    }

    pub fn remove_service(&mut self, service_key: &[u8]) -> bool {
        self.service_options.remove(service_key).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.service_options.is_empty()
    }

    pub fn get_summary(&self) -> String {
        if self.service_options.is_empty() {
            return "no tag services".to_string();
        }

        let mut statements: Vec<String> = self
            .service_options
            .values()
            .filter(|options| options.get_tags || !options.additional_tags.is_empty())
            .map(ServiceTagImportOptions::get_summary)
            .collect();

        statements.sort();

        if statements.is_empty() {
            "no tags".to_string()
        } else {
            statements.join(", ")
        }
    }
}

impl Serialize for TagImportOptions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let map: BTreeMap<String, &ServiceTagImportOptions> = self
            .service_options
            .iter()
            .map(|(key, options)| (to_hex(key), options))
            .collect();

        map.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TagImportOptions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: BTreeMap<String, ServiceTagImportOptions> = BTreeMap::deserialize(deserializer)?;
        let mut options = Self::default();

        for (key, service_options) in map {
            if let Some(key) = from_hex(&key) {
                options.set_service_options(key, service_options);
            }
        }

        Ok(options)
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn from_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoteImportOptions {
    pub get_notes: bool,
    pub extend_existing_note_if_possible: bool,
    /// `replace`, `ignore`, `append` or `rename`.
    pub conflict_resolution: String,
    pub name_whitelist: Vec<String>,
    pub all_name_override: Option<String>,
    pub names_to_name_overrides: BTreeMap<String, String>,
}

impl Default for NoteImportOptions {
    fn default() -> Self {
        Self {
            get_notes: true,
            extend_existing_note_if_possible: true,
            conflict_resolution: "rename".to_string(),
            name_whitelist: Vec::new(),
            all_name_override: None,
            names_to_name_overrides: BTreeMap::new(),
        }
    }
}

impl NoteImportOptions {
    pub fn get_summary(&self) -> String {
        if !self.get_notes {
            return "not adding notes".to_string();
        }

        let mut statements = vec!["adding notes".to_string()];

        if !self.extend_existing_note_if_possible {
            statements.push("duplicating rather than extending".to_string());
        }

        statements.push(format!("conflict resolution: {}", self.conflict_resolution));

        if self.all_name_override.is_some() || !self.names_to_name_overrides.is_empty() {
            statements.push("with renames".to_string());
        }

        statements.join(", ")
    }
}

pub const PRESENTATION_STATUS_ANY_GOOD: &str = "all";
pub const PRESENTATION_STATUS_NEW_ONLY: &str = "new";
pub const PRESENTATION_STATUS_NONE: &str = "none";

pub const PRESENTATION_INBOX_AGNOSTIC: &str = "agnostic";
pub const PRESENTATION_INBOX_REQUIRE_INBOX: &str = "require_inbox";
pub const PRESENTATION_INBOX_AND_INCLUDE_ALL_INBOX: &str = "include_all_inbox";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresentationImportOptions {
    pub presentation_status: String,
    pub presentation_inbox: String,
}

impl Default for PresentationImportOptions {
    fn default() -> Self {
        Self {
            presentation_status: PRESENTATION_STATUS_ANY_GOOD.to_string(),
            presentation_inbox: PRESENTATION_INBOX_AGNOSTIC.to_string(),
        }
    }
}

impl PresentationImportOptions {
    pub fn presents_all(&self) -> bool {
        self.presentation_status == PRESENTATION_STATUS_ANY_GOOD
            && self.presentation_inbox == PRESENTATION_INBOX_AGNOSTIC
    }

    pub fn get_summary(&self) -> String {
        if self.presentation_status == PRESENTATION_STATUS_NONE {
            return "not presenting any files".to_string();
        }

        let status = match self.presentation_status.as_str() {
            PRESENTATION_STATUS_NEW_ONLY => "new files",
            _ => "all files",
        };

        match self.presentation_inbox.as_str() {
            PRESENTATION_INBOX_REQUIRE_INBOX => {
                if status == "all files" {
                    "inbox files".to_string()
                } else {
                    "new inbox files".to_string()
                }
            }
            PRESENTATION_INBOX_AND_INCLUDE_ALL_INBOX => {
                if status == "new files" {
                    "new or inbox files".to_string()
                } else {
                    "all files, including inbox".to_string()
                }
            }
            _ => status.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExternalProgramEntry {
    pub name: String,
    pub path: String,
    pub arguments: String,
    pub do_it_on_new: bool,
    pub do_it_on_already_in: bool,
}

impl ExternalProgramEntry {
    pub fn get_summary(&self) -> String {
        let import_types = match (self.do_it_on_new, self.do_it_on_already_in) {
            (true, true) => "all file imports",
            (true, false) => "all new file imports",
            (false, true) => "all \"already in db\" imports",
            (false, false) => "no imports (will do nothing!)",
        };

        format!("call {} on {import_types}", self.name)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExternalProgramsImportOptions {
    pub entries: Vec<ExternalProgramEntry>,
}

impl ExternalProgramsImportOptions {
    pub fn get_summary(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut statements: Vec<String> = self
            .entries
            .iter()
            .map(ExternalProgramEntry::get_summary)
            .collect();
        statements.sort();

        statements.join(", ")
    }
}

/// One of the six directory slots a filename tagging rule can read. The
/// indexes are `0, 1, 2` from the start of the path and `-3, -2, -1` from the
/// end, matching Python's `_directories_dict`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DirectoryTaggingRule {
    pub enabled: bool,
    /// Empty means an unnamespaced tag; otherwise the directory name is added
    /// as `namespace:directory`.
    pub namespace: String,
}

impl Default for DirectoryTaggingRule {
    fn default() -> Self {
        Self {
            enabled: false,
            namespace: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuickNamespaceRule {
    pub namespace: String,
    pub regex: String,
}

impl Default for QuickNamespaceRule {
    fn default() -> Self {
        Self {
            namespace: String::new(),
            regex: String::new(),
        }
    }
}

/// `FilenameTaggingOptions.py`: turns a file's path into tags.
///
/// Python keys this object by tag service key on an import folder and on a
/// local import page; here the map lives on the folder entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FilenameTaggingOptions {
    pub tags_for_all: Vec<String>,
    pub add_filename: bool,
    pub filename_namespace: String,
    /// Keys are the six directory slots described on [`DirectoryTaggingRule`].
    pub directories: BTreeMap<i32, DirectoryTaggingRule>,
    pub quick_namespaces: Vec<QuickNamespaceRule>,
    pub regexes: Vec<String>,
}

pub const FILENAME_TAGGING_DIRECTORIES: &[i32] = &[0, 1, 2, -3, -2, -1];

impl Default for FilenameTaggingOptions {
    fn default() -> Self {
        let mut directories = BTreeMap::new();

        for index in FILENAME_TAGGING_DIRECTORIES {
            directories.insert(*index, DirectoryTaggingRule::default());
        }

        Self {
            tags_for_all: Vec::new(),
            add_filename: false,
            filename_namespace: "filename".to_string(),
            directories,
            quick_namespaces: Vec::new(),
            regexes: Vec::new(),
        }
    }
}

impl FilenameTaggingOptions {
    /// `GetTags( service_key, path )`
    ///
    /// Tags are cleaned and de-duplicated; display filtering is applied by the
    /// caller when it writes the mappings.
    pub fn get_tags(&self, path: &Path) -> Vec<String> {
        let mut tags: Vec<String> = self.tags_for_all.clone();

        let filename = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();

        if self.add_filename && !filename.is_empty() {
            tags.push(namespaced_tag(&self.filename_namespace, &filename));
        }

        let directories = directory_components(path);

        for (index, rule) in &self.directories {
            if !rule.enabled {
                continue;
            }

            let Some(directory) = directory_at(&directories, *index) else {
                continue;
            };

            tags.push(namespaced_tag(&rule.namespace, directory));
        }

        let path_text = path.to_string_lossy();

        for pattern in &self.regexes {
            collect_regex_matches(&mut tags, pattern, &path_text, None);
        }

        for rule in &self.quick_namespaces {
            collect_regex_matches(
                &mut tags,
                &rule.regex,
                &path_text,
                Some(rule.namespace.as_str()),
            );
        }

        clean_tag_list(tags)
    }

    pub fn get_summary(&self) -> String {
        let mut statements = Vec::new();

        if !self.tags_for_all.is_empty() {
            statements.push(format!("{} tag(s) for all", self.tags_for_all.len()));
        }

        if self.add_filename {
            statements.push(format!("filename as '{}'", self.filename_namespace));
        }

        let directories = self
            .directories
            .values()
            .filter(|rule| rule.enabled)
            .count();

        if directories > 0 {
            statements.push(format!("{directories} directory rule(s)"));
        }

        if !self.quick_namespaces.is_empty() {
            statements.push(format!(
                "{} namespace regex(es)",
                self.quick_namespaces.len()
            ));
        }

        if !self.regexes.is_empty() {
            statements.push(format!("{} regex(es)", self.regexes.len()));
        }

        if statements.is_empty() {
            "no tags".to_string()
        } else {
            statements.join(", ")
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tags_for_all.is_empty()
            && !self.add_filename
            && self.directories.values().all(|rule| !rule.enabled)
            && self.quick_namespaces.is_empty()
            && self.regexes.is_empty()
    }
}

fn namespaced_tag(namespace: &str, subtag: &str) -> String {
    let namespace = namespace.trim();

    if namespace.is_empty() {
        subtag.to_string()
    } else {
        format!("{namespace}:{subtag}")
    }
}

/// Path directories without the drive/root and the filename, so index `0` is
/// the first real directory and `-1` the immediate parent.
fn directory_components(path: &Path) -> Vec<String> {
    let mut components: Vec<String> = path
        .parent()
        .map(|parent| {
            parent
                .components()
                .filter_map(|component| match component {
                    std::path::Component::Normal(part) => Some(part.to_string_lossy().to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    components.retain(|component| !component.is_empty());

    components
}

fn directory_at(directories: &[String], index: i32) -> Option<&str> {
    let len = directories.len() as i32;
    let resolved = if index < 0 { len + index } else { index };

    if resolved < 0 || resolved >= len {
        return None;
    }

    directories.get(resolved as usize).map(String::as_str)
}

/// Mimics `re.findall`: capture groups win when the pattern has any, otherwise
/// the whole match is used.
fn collect_regex_matches(
    tags: &mut Vec<String>,
    pattern: &str,
    text: &str,
    namespace: Option<&str>,
) {
    let Ok(regex) = regex::Regex::new(pattern) else {
        return;
    };

    if regex.captures_len() > 1 {
        for captures in regex.captures_iter(text) {
            for group in captures.iter().skip(1).flatten() {
                let tag = match namespace {
                    Some(namespace) => namespaced_tag(namespace, group.as_str()),
                    None => group.as_str().to_string(),
                };

                tags.push(tag);
            }
        }
    } else {
        for found in regex.find_iter(text) {
            let tag = match namespace {
                Some(namespace) => namespaced_tag(namespace, found.as_str()),
                None => found.as_str().to_string(),
            };

            tags.push(tag);
        }
    }
}

fn clean_tag_list(tags: Vec<String>) -> Vec<String> {
    let mut clean: BTreeSet<String> = BTreeSet::new();

    for tag in tags {
        let tag = tag.trim();

        if tag.is_empty() {
            continue;
        }

        match hydrus_tags::clean_tag(tag) {
            Ok(tag) if hydrus_tags::check_tag_not_empty(&tag).is_ok() => {
                clean.insert(tag);
            }
            _ => {}
        }
    }

    clean.into_iter().collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportOptionsContainer {
    pub prefetch: Option<PrefetchImportOptions>,
    pub file_filtering: Option<FileFilteringImportOptions>,
    pub tag_filtering: Option<TagFilteringImportOptions>,
    pub locations: Option<LocationImportOptions>,
    pub tags: Option<TagImportOptions>,
    pub notes: Option<NoteImportOptions>,
    pub external_programs: Option<ExternalProgramsImportOptions>,
    pub presentation: Option<PresentationImportOptions>,
    /// Option type -> where this entry came from (`global`, `url class: x`, ...).
    pub source_labels: BTreeMap<String, String>,
}

impl ImportOptionsContainer {
    /// A container with every option type set to its default value. This is the
    /// global default the manager starts with.
    pub fn with_defaults() -> Self {
        let mut container = Self::default();

        container.prefetch = Some(PrefetchImportOptions::default());
        container.file_filtering = Some(FileFilteringImportOptions::default());
        container.tag_filtering = Some(TagFilteringImportOptions::default());
        container.locations = Some(LocationImportOptions::default());
        container.tags = Some(TagImportOptions::default());
        container.notes = Some(NoteImportOptions::default());
        container.external_programs = Some(ExternalProgramsImportOptions::default());
        container.presentation = Some(PresentationImportOptions::default());

        container
    }

    pub fn has(&self, option_type: &str) -> bool {
        match option_type {
            "prefetch" => self.prefetch.is_some(),
            "file_filtering" => self.file_filtering.is_some(),
            "tag_filtering" => self.tag_filtering.is_some(),
            "locations" => self.locations.is_some(),
            "tags" => self.tags.is_some(),
            "notes" => self.notes.is_some(),
            "external_programs" => self.external_programs.is_some(),
            "presentation" => self.presentation.is_some(),
            _ => false,
        }
    }

    pub fn delete(&mut self, option_type: &str) {
        match option_type {
            "prefetch" => self.prefetch = None,
            "file_filtering" => self.file_filtering = None,
            "tag_filtering" => self.tag_filtering = None,
            "locations" => self.locations = None,
            "tags" => self.tags = None,
            "notes" => self.notes = None,
            "external_programs" => self.external_programs = None,
            "presentation" => self.presentation = None,
            _ => return,
        }

        self.source_labels.remove(option_type);
    }

    pub fn is_empty(&self) -> bool {
        IMPORT_OPTIONS_TYPES_CANONICAL_ORDER
            .iter()
            .all(|option_type| !self.has(option_type))
    }

    pub fn is_full(&self) -> bool {
        IMPORT_OPTIONS_TYPES_CANONICAL_ORDER
            .iter()
            .all(|option_type| self.has(option_type))
    }

    /// Copies entries this container does not have yet.
    pub fn fill_in_with_slice(&mut self, other: &Self, source_label: Option<&str>) {
        if other.is_empty() {
            return;
        }

        for option_type in IMPORT_OPTIONS_TYPES_CANONICAL_ORDER {
            if self.has(option_type) || !other.has(option_type) {
                continue;
            }

            self.copy_option_type_from(other, option_type, source_label);
        }
    }

    /// Copies every entry the other container defines.
    pub fn overwrite_with_slice(&mut self, other: &Self, source_label: Option<&str>) {
        if other.is_empty() {
            return;
        }

        for option_type in IMPORT_OPTIONS_TYPES_CANONICAL_ORDER {
            if other.has(option_type) {
                self.copy_option_type_from(other, option_type, source_label);
            }
        }
    }

    pub fn get_duplicate_with_just_these_option_types(&self, option_types: &[&str]) -> Self {
        let mut result = Self::default();

        for option_type in option_types {
            if self.has(option_type) {
                result.copy_option_type_from(
                    self,
                    option_type,
                    self.source_labels.get(*option_type).map(String::as_str),
                );
            }
        }

        result
    }

    fn copy_option_type_from(
        &mut self,
        other: &Self,
        option_type: &str,
        source_label: Option<&str>,
    ) {
        match option_type {
            "prefetch" => self.prefetch = other.prefetch.clone(),
            "file_filtering" => self.file_filtering = other.file_filtering.clone(),
            "tag_filtering" => self.tag_filtering = other.tag_filtering.clone(),
            "locations" => self.locations = other.locations.clone(),
            "tags" => self.tags = other.tags.clone(),
            "notes" => self.notes = other.notes.clone(),
            "external_programs" => self.external_programs = other.external_programs.clone(),
            "presentation" => self.presentation = other.presentation.clone(),
            _ => return,
        }

        match source_label {
            Some(label) => {
                self.source_labels
                    .insert(option_type.to_string(), label.to_string());
            }
            None => {
                self.source_labels.remove(option_type);
            }
        }
    }

    pub fn get_source_label(&self, option_type: &str) -> String {
        self.source_labels
            .get(option_type)
            .cloned()
            .unwrap_or_else(|| "global".to_string())
    }

    pub fn import_options_differ(&self, other: &Self, option_type: &str) -> bool {
        match (self.has(option_type), other.has(option_type)) {
            (false, false) => false,
            (true, true) => {
                let left = serde_json::to_value(self).unwrap_or_default();
                let right = serde_json::to_value(other).unwrap_or_default();

                left.get(option_type) != right.get(option_type)
            }
            _ => true,
        }
    }

    pub fn file_filtering_or_default(&self) -> FileFilteringImportOptions {
        self.file_filtering.clone().unwrap_or_default()
    }

    pub fn locations_or_default(&self) -> LocationImportOptions {
        self.locations.clone().unwrap_or_default()
    }

    pub fn presentation_or_default(&self) -> PresentationImportOptions {
        self.presentation.clone().unwrap_or_default()
    }

    pub fn prefetch_or_default(&self) -> PrefetchImportOptions {
        self.prefetch.clone().unwrap_or_default()
    }

    pub fn notes_or_default(&self) -> NoteImportOptions {
        self.notes.clone().unwrap_or_default()
    }

    pub fn tags_or_default(&self) -> TagImportOptions {
        self.tags.clone().unwrap_or_default()
    }

    pub fn external_programs_or_default(&self) -> ExternalProgramsImportOptions {
        self.external_programs.clone().unwrap_or_default()
    }

    pub fn tag_filtering_or_default(&self) -> TagFilteringImportOptions {
        self.tag_filtering.clone().unwrap_or_default()
    }

    pub fn get_summary(&self, caller: &str) -> String {
        let mut short = Vec::new();
        let mut long = Vec::new();

        for option_type in IMPORT_OPTIONS_TYPES_CANONICAL_ORDER {
            if !self.has(option_type) {
                continue;
            }

            short.push(option_type_label(option_type).to_string());

            let summary = match *option_type {
                "prefetch" => self
                    .prefetch
                    .as_ref()
                    .map(|options| options.get_summary(caller))
                    .unwrap_or_default(),
                "file_filtering" => self
                    .file_filtering
                    .as_ref()
                    .map(FileFilteringImportOptions::get_summary)
                    .unwrap_or_default(),
                "tag_filtering" => self
                    .tag_filtering
                    .as_ref()
                    .map(TagFilteringImportOptions::get_summary)
                    .unwrap_or_default(),
                "locations" => self
                    .locations
                    .as_ref()
                    .map(LocationImportOptions::get_summary)
                    .unwrap_or_default(),
                "tags" => self
                    .tags
                    .as_ref()
                    .map(TagImportOptions::get_summary)
                    .unwrap_or_default(),
                "notes" => self
                    .notes
                    .as_ref()
                    .map(NoteImportOptions::get_summary)
                    .unwrap_or_default(),
                "external_programs" => self
                    .external_programs
                    .as_ref()
                    .map(ExternalProgramsImportOptions::get_summary)
                    .unwrap_or_default(),
                "presentation" => self
                    .presentation
                    .as_ref()
                    .map(PresentationImportOptions::get_summary)
                    .unwrap_or_default(),
                _ => String::new(),
            };

            if !summary.is_empty() {
                long.push(summary);
            }
        }

        if short.is_empty() {
            return String::new();
        }

        if short.len() <= 2 && (1..=2).contains(&long.len()) {
            format!("{}: {}", short.join(", "), long.join(" | "))
        } else {
            short.join(", ")
        }
    }
}

pub fn option_type_label(option_type: &str) -> &'static str {
    IMPORT_OPTIONS_TYPE_LABELS
        .iter()
        .find(|(key, _)| *key == option_type)
        .map(|(_, label)| *label)
        .unwrap_or("unknown option")
}

pub fn option_types_for_caller(caller: &str) -> Vec<&'static str> {
    match caller {
        "local_import" => vec![
            "file_filtering",
            "locations",
            "external_programs",
            "presentation",
        ],
        "local_import_folder" => vec!["locations", "external_programs", "presentation"],
        "subscription" => vec!["locations", "presentation"],
        "post_urls" | "watcher_urls" => vec![
            "file_filtering",
            "tag_filtering",
            "locations",
            "tags",
            "notes",
            "presentation",
        ],
        "url_class" => vec![
            "prefetch",
            "file_filtering",
            "tag_filtering",
            "locations",
            "tags",
            "notes",
        ],
        "client_api" => vec!["file_filtering", "locations"],
        _ => IMPORT_OPTIONS_TYPES_CANONICAL_ORDER.to_vec(),
    }
}

/// A set of option-type keys for the editor, including any type a container
/// already sets even if the caller type normally hides it.
pub fn editor_option_types_for_caller(
    caller: &str,
    container: &ImportOptionsContainer,
) -> Vec<&'static str> {
    let mut types = option_types_for_caller(caller);

    for option_type in IMPORT_OPTIONS_TYPES_CANONICAL_ORDER {
        if container.has(option_type) && !types.contains(option_type) {
            types.push(option_type);
        }
    }

    types
}

/// The mime classes offered by the file filtering panel.
pub fn file_type_classes() -> Vec<Mime> {
    GENERAL_CLASSES_OF_FILETYPE.to_vec()
}

/// File extensions of the mime classes above, for the GUI labels.
pub fn mime_class_extensions(mime: Mime) -> String {
    let mut extensions = BTreeSet::new();

    let members: &[Mime] = match mime {
        Mime::GeneralImage => IMAGES,
        Mime::GeneralAnimation => ANIMATIONS,
        Mime::GeneralVideo => VIDEO,
        Mime::GeneralAudio => AUDIO,
        Mime::GeneralApplication => APPLICATIONS,
        Mime::GeneralAppArchive => ARCHIVES,
        Mime::GeneralImageProject => IMAGE_PROJECT_FILES,
        Mime::GeneralFile => GENERAL_CLASSES_OF_FILETYPE,
        _ => &[],
    };

    for member in members {
        if *member == Mime::GeneralFile {
            continue;
        }

        extensions.insert(member.extension().to_string());
    }

    let mut extensions: Vec<String> = extensions.into_iter().collect();
    extensions.sort();

    extensions.join(" ")
}

#[cfg(test)]
mod tests;
