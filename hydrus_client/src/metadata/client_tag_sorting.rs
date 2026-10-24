//! Tag sorting from `hydrus/client/metadata/ClientTagSorting.py`.

use hydrus_core::hydrus_text;
use serde_json::{Value, json};

use crate::client_constants;

pub const SORT_BY_HUMAN_TAG: i32 = 0;
pub const SORT_BY_HUMAN_SUBTAG: i32 = 1;
pub const SORT_BY_COUNT: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSort {
    pub sort_type: i32,
    pub sort_order: i32,
    pub use_siblings: bool,
    pub group_by: String,
}

impl Default for TagSort {
    fn default() -> Self {
        Self::static_get_text_asc_default()
    }
}

impl TagSort {
    pub fn new(sort_type: i32, sort_order: i32, use_siblings: bool, group_by: String) -> Self {
        Self {
            sort_type,
            sort_order,
            use_siblings,
            group_by,
        }
    }

    pub fn static_get_text_asc_default() -> Self {
        Self::new(SORT_BY_HUMAN_TAG, client_constants::SORT_ASC, true, String::new())
    }

    pub fn static_get_text_asc_user_grouped_default() -> Self {
        Self::new(
            SORT_BY_HUMAN_SUBTAG,
            client_constants::SORT_ASC,
            true,
            "creator".to_string(),
        )
    }

    pub fn affected_by_count(&self) -> bool {
        self.sort_type == SORT_BY_COUNT
    }

    /// `ToString()`
    pub fn to_string(&self) -> String {
        let sort_type = match self.sort_type {
            SORT_BY_HUMAN_TAG => "human tag",
            SORT_BY_HUMAN_SUBTAG => "human subtag",
            SORT_BY_COUNT => "count",
            _ => "unknown",
        };

        let direction = client_constants::sort_order_string(self.sort_order);

        if self.group_by.is_empty() {
            format!("{sort_type}, {direction}")
        } else {
            format!("{sort_type}, {direction}, grouped by {}", self.group_by)
        }
    }

    pub fn to_dict_for_api(&self) -> Value {
        json!({
            "sort_type": self.sort_type,
            "sort_order": self.sort_order,
            "use_siblings": self.use_siblings,
            "group_by": self.group_by,
        })
    }

    /// Sorts tags in place using the same human sort keys as hydrus_core.
    pub fn sort_tags(&self, tags: &mut [String]) {
        match self.sort_type {
            SORT_BY_HUMAN_SUBTAG => tags.sort_by(|a, b| {
                human_subtag_key(a)
                    .cmp(&human_subtag_key(b))
                    .then_with(|| a.cmp(b))
            }),
            _ => hydrus_text::human_text_sort_strings(tags),
        }

        if self.sort_order == client_constants::SORT_DESC {
            tags.reverse();
        }
    }
}

fn human_subtag_key(tag: &str) -> String {
    tag.split_once(':')
        .map(|(_, subtag)| subtag.trim_start().to_string())
        .unwrap_or_else(|| tag.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags() -> Vec<String> {
        vec![
            "blue eyes".to_string(),
            "artist:someone".to_string(),
            "1girl".to_string(),
        ]
    }

    #[test]
    fn defaults_match_python_constructors() {
        let sort = TagSort::static_get_text_asc_default();

        assert_eq!(sort.sort_type, SORT_BY_HUMAN_TAG);
        assert_eq!(sort.sort_order, client_constants::SORT_ASC);
        assert!(sort.use_siblings);

        let grouped = TagSort::static_get_text_asc_user_grouped_default();

        assert_eq!(grouped.sort_type, SORT_BY_HUMAN_SUBTAG);
        assert_eq!(grouped.group_by, "creator");
    }

    #[test]
    fn descending_sort_reverses_order() {
        let mut ascending = tags();
        let mut descending = tags();

        TagSort::static_get_text_asc_default().sort_tags(&mut ascending);
        TagSort::new(
            SORT_BY_HUMAN_TAG,
            client_constants::SORT_DESC,
            true,
            String::new(),
        )
        .sort_tags(&mut descending);

        assert_eq!(descending, ascending.into_iter().rev().collect::<Vec<_>>());
    }

    #[test]
    fn subtag_sort_ignores_namespace_prefix() {
        let mut values = vec![
            "creator:zebra".to_string(),
            "series:alpha".to_string(),
            "character:beta".to_string(),
        ];

        TagSort::static_get_text_asc_user_grouped_default().sort_tags(&mut values);

        assert_eq!(values[0], "series:alpha");
        assert_eq!(values[1], "character:beta");
        assert_eq!(values[2], "creator:zebra");
    }

    #[test]
    fn api_dict_has_python_keys() {
        let dict = TagSort::static_get_text_asc_default().to_dict_for_api();

        assert_eq!(dict["sort_type"], json!(SORT_BY_HUMAN_TAG));
        assert_eq!(dict["sort_order"], json!(client_constants::SORT_ASC));
    }
}
