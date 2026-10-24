pub mod client_content_updates;
pub mod client_metadata_conditional;
pub mod client_ratings;
pub mod client_tag_sorting;
pub mod client_tags;

pub use client_content_updates::{ContentAction, ContentUpdate, ContentUpdatePackage};
pub use client_metadata_conditional::MetadataConditional;
pub use client_ratings::{Rating, RatingsManager};
pub use client_tag_sorting::TagSort;
pub use client_tags::{ServiceKeysToTags, TagStatus};

pub fn register_serialisable_types() {
    client_metadata_conditional::register_serialisable_type();
}
