//! Tag sorting from `hydrus/client/metadata/ClientTagSorting.py`.

use std::fmt;

use hydrus_core::hydrus_text;
use serde_json::{Value, json};

use crate::client_constants::SortOrder;

/// Tag sort type, mirroring the `SORT_BY_*` values from `ClientTagSorting.py`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagSortType {
    HumanTag = 0,
    HumanSubtag = 1,
    Count = 2,
}

impl TagSortType {
    pub const ALL: &'static [TagSortType] = &[
        TagSortType::HumanTag,
        TagSortType::HumanSubtag,
        TagSortType::Count,
    ];

    pub const fn as_int(self) -> i32 {
        self as i32
    }

    pub fn try_from_int(value: i32) -> Option<Self> {
        match value {
            0 => Some(TagSortType::HumanTag),
            1 => Some(TagSortType::HumanSubtag),
            2 => Some(TagSortType::Count),
            _ => None,
        }
    }
}

impl fmt::Display for TagSortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TagSortType::HumanTag => "human tag",
            TagSortType::HumanSubtag => "human subtag",
            TagSortType::Count => "count",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSort {
    pub sort_type: TagSortType,
    pub sort_order: SortOrder,
    pub use_siblings: bool,
    pub group_by: String,
}

impl Default for TagSort {
    fn default() -> Self {
        Self::static_get_text_asc_default()
    }
}

impl TagSort {
    pub fn new(
        sort_type: TagSortType,
        sort_order: SortOrder,
        use_siblings: bool,
        group_by: String,
    ) -> Self {
        Self {
            sort_type,
            sort_order,
            use_siblings,
            group_by,
        }
    }

    pub fn static_get_text_asc_default() -> Self {
        Self::new(
            TagSortType::HumanTag,
            SortOrder::Ascending,
            true,
            String::new(),
        )
    }

    pub fn static_get_text_asc_user_grouped_default() -> Self {
        Self::new(
            TagSortType::HumanSubtag,
            SortOrder::Ascending,
            true,
            "creator".to_string(),
        )
    }

    pub fn affected_by_count(&self) -> bool {
        self.sort_type == TagSortType::Count
    }

    /// `ToString()`
    pub fn to_string(&self) -> String {
        let direction = self.sort_order;

        if self.group_by.is_empty() {
            format!("{}, {direction}", self.sort_type)
        } else {
            format!(
                "{}, {direction}, grouped by {}",
                self.sort_type, self.group_by
            )
        }
    }

    pub fn to_dict_for_api(&self) -> Value {
        json!({
            "sort_type": self.sort_type.as_int(),
            "sort_order": self.sort_order.as_int(),
            "use_siblings": self.use_siblings,
            "group_by": self.group_by,
        })
    }

    /// Sorts tags in place using the same human sort keys as hydrus_core.
    pub fn sort_tags(&self, tags: &mut [String]) {
        match self.sort_type {
            TagSortType::HumanSubtag => tags.sort_by(|a, b| {
                human_subtag_key(a)
                    .cmp(&human_subtag_key(b))
                    .then_with(|| a.cmp(b))
            }),
            _ => hydrus_text::human_text_sort_strings(tags),
        }

        if self.sort_order == SortOrder::Descending {
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

        assert_eq!(sort.sort_type, TagSortType::HumanTag);
        assert_eq!(sort.sort_order, SortOrder::Ascending);
        assert!(sort.use_siblings);

        let grouped = TagSort::static_get_text_asc_user_grouped_default();

        assert_eq!(grouped.sort_type, TagSortType::HumanSubtag);
        assert_eq!(grouped.group_by, "creator");
    }

    #[test]
    fn descending_sort_reverses_order() {
        let mut ascending = tags();
        let mut descending = tags();

        TagSort::static_get_text_asc_default().sort_tags(&mut ascending);
        TagSort::new(
            TagSortType::HumanTag,
            SortOrder::Descending,
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

        assert_eq!(dict["sort_type"], json!(0));
        assert_eq!(dict["sort_order"], json!(0));
    }
}
