//! Import option value objects from `hydrus/client/importing/options/`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::Mime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFilteringImportOptions {
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_resolution: Option<(u32, u32)>,
    pub max_resolution: Option<(u32, u32)>,
    pub max_gif_size: Option<u64>,
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

        if !self.allowed_mimes.is_empty() && !self.allowed_mimes.contains(&mime) {
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
        if !self.allowed_mimes.is_empty() {
            parts.push(format!(
                "mimes {}",
                self.allowed_mimes
                    .iter()
                    .map(|mime| mime.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if parts.is_empty() {
            "allows all files".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn is_empty(&self) -> bool {
        self.service_options.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportOptionsContainer {
    pub file_filtering: FileFilteringImportOptions,
    pub tag_import_options: TagImportOptions,
}

#[cfg(test)]
mod tests;
