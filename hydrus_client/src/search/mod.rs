pub mod client_file_search_context;
pub mod client_file_system_predicates;
pub mod client_number_test;
pub mod client_search_predicate;
pub mod client_search_tag_context;
pub mod predicate_constants;

pub use client_file_search_context::{
    FileSearchContext, SEARCH_TYPE_AND, SEARCH_TYPE_OR,
};
pub use client_file_system_predicates::FileSystemPredicates;
pub use client_number_test::NumberTest;
pub use client_search_predicate::{Predicate, PredicateCount, PredicateValue};
pub use client_search_tag_context::TagContext;

/// Registers the client search serialisable types with the core registry.
pub fn register_serialisable_types() {
    hydrus_core::hydrus_serialisable::register_serialisable_type(
        hydrus_core::hydrus_serialisable::SERIALISABLE_TYPE_NUMBER_TEST,
        || Box::new(NumberTest::default()),
    );
    hydrus_core::hydrus_serialisable::register_serialisable_type(
        hydrus_core::hydrus_serialisable::SERIALISABLE_TYPE_PREDICATE,
        || Box::new(Predicate::tag("")),
    );
    hydrus_core::hydrus_serialisable::register_serialisable_type(
        hydrus_core::hydrus_serialisable::SERIALISABLE_TYPE_TAG_CONTEXT,
        || Box::new(TagContext::default()),
    );
    hydrus_core::hydrus_serialisable::register_serialisable_type(
        hydrus_core::hydrus_serialisable::SERIALISABLE_TYPE_FILE_SEARCH_CONTEXT,
        || Box::new(FileSearchContext::default()),
    );
}
