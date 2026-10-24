//! Metadata routers from `hydrus/client/metadata/ClientMetadataMigration*.py`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::media::MediaResult;
use crate::metadata::TagStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataImporter {
    Tags { service_key: Vec<u8> },
    Namespace { namespace: String },
    Notes,
    URLs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataExporter {
    Txt,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleFileMetadataRouter {
    pub importers: Vec<MetadataImporter>,
    pub exporter: MetadataExporter,
}

impl SingleFileMetadataRouter {
    pub fn new(importers: Vec<MetadataImporter>, exporter: MetadataExporter) -> Self {
        Self {
            importers,
            exporter,
        }
    }

    /// Collects values per importer label.
    pub fn gather(&self, media_result: &MediaResult) -> BTreeMap<String, Vec<String>> {
        let mut gathered = BTreeMap::new();

        for importer in &self.importers {
            match importer {
                MetadataImporter::Tags { service_key } => {
                    let tags = media_result.tags_manager.get_tags(service_key);

                    if !tags.is_empty() {
                        gathered.insert(format!("tags:{}", hex_encode(service_key)), tags);
                    }
                }
                MetadataImporter::Namespace { namespace } => {
                    let prefix = format!("{namespace}:");

                    let mut values = Vec::new();

                    for service_key in media_result.tags_manager.tags.get_service_keys() {
                        for tag in media_result.tags_manager.get_tags(service_key) {
                            if let Some(value) = tag.strip_prefix(&prefix) {
                                values.push(value.to_string());
                            }
                        }
                    }

                    values.sort();
                    values.dedup();

                    if !values.is_empty() {
                        gathered.insert(format!("namespace:{namespace}"), values);
                    }
                }
                MetadataImporter::Notes => {
                    let notes: Vec<String> = media_result
                        .notes_manager
                        .notes()
                        .iter()
                        .map(|(name, note)| format!("{name}: {note}"))
                        .collect();

                    if !notes.is_empty() {
                        gathered.insert("notes".to_string(), notes);
                    }
                }
                MetadataImporter::URLs => {
                    // URLs are not part of MediaResult yet; keep the importer
                    // present so router definitions stay compatible.
                }
            }
        }

        gathered
    }

    pub fn export(&self, media_result: &MediaResult) -> String {
        let gathered = self.gather(media_result);

        match self.exporter {
            MetadataExporter::Txt => {
                let mut lines = Vec::new();

                for values in gathered.values() {
                    for value in values {
                        lines.push(value.clone());
                    }
                }

                lines.join("\n")
            }
            MetadataExporter::Json => {
                let value: Value = gathered
                    .into_iter()
                    .map(|(key, values)| (key, json!(values)))
                    .collect::<serde_json::Map<_, _>>()
                    .into();

                serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
            }
        }
    }

    /// Adds all gathered tags to the given service through a content update.
    pub fn build_tag_update(
        &self,
        service_key: Vec<u8>,
        media_result: &MediaResult,
    ) -> crate::metadata::ContentUpdatePackage {
        use hydrus_core::hydrus_constants::ContentType;

        let mut tags = Vec::new();

        for importer in &self.importers {
            if let MetadataImporter::Tags { service_key } = importer {
                tags.extend(media_result.tags_manager.get_tags(service_key));
            }
        }

        if tags.is_empty() {
            return crate::metadata::ContentUpdatePackage::new();
        }

        crate::metadata::ContentUpdatePackage::from_updates(vec![
            crate::metadata::ContentUpdate::new(
                crate::metadata::ContentAction::Add,
                service_key,
                ContentType::Mappings,
            )
            .with_hashes([media_result.get_hash().to_vec()])
            .with_tags(tags),
        ])
    }
}

/// Default text exporter used by the legacy "neighbouring txt" migration.
pub fn default_neighbouring_txt_router(tag_service_keys: Vec<Vec<u8>>) -> SingleFileMetadataRouter {
    SingleFileMetadataRouter::new(
        tag_service_keys
            .into_iter()
            .map(|service_key| MetadataImporter::Tags { service_key })
            .collect(),
        MetadataExporter::Txt,
    )
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::MediaResult;
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn media() -> MediaResult {
        let mut result = MediaResult::new(
            vec![1],
            FileInfo {
                size: 1,
                mime: Mime::Png,
                width: None,
                height: None,
                duration_ms: None,
                num_frames: None,
                has_audio: false,
                num_words: None,
            },
        );

        result.tags_manager.tags.add_tags(
            b"local tags",
            ["creator:someone", "blue eyes"],
            TagStatus::Current,
        );
        result.notes_manager.set_note("page", "hello");

        result
    }

    #[test]
    fn gathers_tags_notes_and_namespaces() {
        let router = SingleFileMetadataRouter::new(
            vec![
                MetadataImporter::Tags {
                    service_key: b"local tags".to_vec(),
                },
                MetadataImporter::Namespace {
                    namespace: "creator".to_string(),
                },
                MetadataImporter::Notes,
            ],
            MetadataExporter::Json,
        );

        let gathered = router.gather(&media());

        assert_eq!(gathered["tags:6c6f63616c2074616773"].len(), 2);
        assert_eq!(gathered["namespace:creator"], vec!["someone"]);
        assert_eq!(gathered["notes"], vec!["page: hello"]);
    }

    #[test]
    fn txt_export_is_one_value_per_line() {
        let router = SingleFileMetadataRouter::new(
            vec![MetadataImporter::Tags {
                service_key: b"local tags".to_vec(),
            }],
            MetadataExporter::Txt,
        );

        let text = router.export(&media());
        let mut lines: Vec<&str> = text.lines().collect();
        lines.sort();

        assert_eq!(lines, vec!["blue eyes", "creator:someone"]);
    }

    #[test]
    fn tag_update_contains_matching_hashes() {
        let router = SingleFileMetadataRouter::new(
            vec![MetadataImporter::Tags {
                service_key: b"local tags".to_vec(),
            }],
            MetadataExporter::Txt,
        );

        let package = router.build_tag_update(b"dest tags".to_vec(), &media());

        assert_eq!(package.updates.len(), 1);
        assert_eq!(package.updates[0].hashes, vec![vec![1]]);
        assert_eq!(package.updates[0].tags.len(), 2);
    }
}
