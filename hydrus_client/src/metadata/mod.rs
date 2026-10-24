pub mod client_content_updates;
pub mod client_file_migration;
pub mod client_metadata_conditional;
pub mod client_ratings;
pub mod client_tag_sorting;
pub mod client_tags;
pub mod metadata_migration;

pub use client_content_updates::{ContentAction, ContentUpdate, ContentUpdatePackage};
pub use client_file_migration::{MoveAction, build_local_file_migration_package};
pub use client_metadata_conditional::MetadataConditional;
pub use client_ratings::{Rating, RatingsManager};
pub use client_tag_sorting::TagSort;
pub use client_tags::{ServiceKeysToTags, TagStatus};
pub use metadata_migration::{MetadataExporter, MetadataImporter, SingleFileMetadataRouter};

pub fn register_serialisable_types() {
    client_metadata_conditional::register_serialisable_type();
}
