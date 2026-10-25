//! Default URL classes, downloader formulae and favourite searches.
//!
//! `ClientDefaults.py` loads these from serialised PNGs. The Rust port ships a
//! built-in, representative set so a fresh client has useful defaults without
//! needing the Python static directory at runtime.

use crate::client_constants;
use crate::networking::client_networking_domain::DomainManager;
use crate::networking::client_networking_url_class::UrlClass;
use crate::parsing::formula::ParseFormula;
use crate::parsing::html_formula::{HtmlContent, HtmlFormula};
use crate::search::client_file_search_context::FileSearchContext;
use crate::search::client_number_test::{NUMBER_TEST_OPERATOR_EQUAL, NumberTest};
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::predicate_constants::*;
use hydrus_core::hydrus_constants::Mime;

use super::network::{CustomHeader, default_custom_headers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleDownloaderFormula {
    pub name: String,
    pub url_class_names: Vec<String>,
    pub formula: HtmlFormula,
}

impl SimpleDownloaderFormula {
    pub fn parse_links(&self, html: &str) -> Vec<String> {
        self.formula.parse(&Default::default(), html, true)
    }
}

pub fn default_url_classes() -> Vec<UrlClass> {
    vec![
        UrlClass::new("safebooru post", "safebooru.org")
            .with_fixed_path(["index.php"])
            .with_variable("page_id"),
        UrlClass::new("danbooru post", "danbooru.donmai.us").with_fixed_path(["posts"]),
        UrlClass::new("pixiv artwork", "www.pixiv.net")
            .with_fixed_path(["artworks"])
            .with_variable("artwork_id"),
        UrlClass::new("twitter tweet", "twitter.com")
            .with_variable("user")
            .with_fixed_path(["status"])
            .with_variable("tweet_id"),
    ]
}

pub fn default_simple_downloader_formulae() -> Vec<SimpleDownloaderFormula> {
    vec![
        SimpleDownloaderFormula {
            name: "all files linked by images in page".to_string(),
            url_class_names: vec!["safebooru post".to_string()],
            formula: HtmlFormula::descending("a", HtmlContent::Attribute("href".to_string())),
        },
        SimpleDownloaderFormula {
            name: "all img src links in page".to_string(),
            url_class_names: Vec::new(),
            formula: HtmlFormula::descending("img", HtmlContent::Attribute("src".to_string())),
        },
    ]
}

/// `set_default_domain_manager_data`
pub fn apply_default_domain_manager_data(manager: &mut DomainManager) {
    for url_class in default_url_classes() {
        manager.add_url_class(url_class);
    }

    for (name, header) in default_custom_headers() {
        manager.set_custom_header(name, header);
    }

    // The Python default is "get tags into the local downloader tags service".
    manager.set_custom_header(
        "X-Hydrus-Default-Tag-Service",
        CustomHeader {
            value: "local downloader tags".to_string(),
            approved: true,
            note: "Default service for imported tags.".to_string(),
        },
    );
}

#[derive(Debug, Clone, PartialEq)]
pub struct FavouriteSearchRow {
    pub folder_name: String,
    pub name: String,
    pub file_search_context: FileSearchContext,
    pub synchronised: bool,
    pub sort_type: client_constants::SortType,
    pub sort_order: client_constants::SortOrder,
}

impl FavouriteSearchRow {
    pub fn new(
        folder_name: impl Into<String>,
        name: impl Into<String>,
        predicates: Vec<Predicate>,
    ) -> Self {
        let mut file_search_context = FileSearchContext::new();
        file_search_context.set_location_context(
            crate::client_location::LocationContext::static_create_simple(
                client_constants::LOCAL_FILE_SERVICE_KEY.to_vec(),
            ),
        );
        file_search_context.set_predicates(predicates);

        Self {
            folder_name: folder_name.into(),
            name: name.into(),
            file_search_context,
            synchronised: true,
            sort_type: client_constants::SortType::FileSize,
            sort_order: client_constants::SortOrder::Descending,
        }
    }
}

/// `set_default_favourite_search_manager_data`
pub fn default_favourite_searches() -> Vec<FavouriteSearchRow> {
    let predicates = vec![
        Predicate::new(PredicateType::SystemInbox, PredicateValue::None),
        Predicate::system_number(
            PredicateType::SystemLimit,
            NumberTest::new(NUMBER_TEST_OPERATOR_EQUAL, 256.0, None),
        ),
        Predicate::new(
            PredicateType::SystemMime,
            PredicateValue::Numbers(
                [Mime::Png, Mime::Jpeg, Mime::Mp4Video]
                    .into_iter()
                    .map(|mime| mime.as_int() as i32)
                    .collect(),
            ),
        ),
    ];

    vec![FavouriteSearchRow::new(
        "example search",
        "inbox filter",
        predicates,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::client_networking_url_class::PathComponent;

    #[test]
    fn default_url_classes_cover_common_sites() {
        let classes = default_url_classes();

        assert!(classes.len() >= 4);
        assert!(
            classes
                .iter()
                .any(|class| class.netloc == "danbooru.donmai.us")
        );
        assert!(classes.iter().any(|class| {
            class
                .path_components
                .iter()
                .any(|component| matches!(component, PathComponent::Variable { .. }))
        }));
    }

    #[test]
    fn downloader_formulae_extract_links() {
        let formula = &default_simple_downloader_formulae()[1];
        let links = formula.parse_links(r#"<div><img src="https://example.com/a.png"></div>"#);

        assert_eq!(links, ["https://example.com/a.png"]);
    }

    #[test]
    fn domain_defaults_install_classes_and_headers() {
        let mut manager = DomainManager::new();
        apply_default_domain_manager_data(&mut manager);

        assert!(!manager.get_url_classes().is_empty());
        assert!(manager.get_custom_header("User-Agent").is_some());
        assert!(manager.get_custom_header("Accept").is_some());
    }

    #[test]
    fn favourite_search_matches_python_default() {
        let rows = default_favourite_searches();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].folder_name, "example search");
        assert_eq!(rows[0].name, "inbox filter");
        assert!(rows[0].synchronised);
        assert_eq!(rows[0].file_search_context.get_predicates().len(), 3);
    }
}
