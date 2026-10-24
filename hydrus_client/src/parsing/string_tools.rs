//! Splitter, slicer and sorter from `hydrus/client/ClientStrings.py`.

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringSplitter {
    pub separator: String,
    pub max_splits: Option<usize>,
}

impl StringSplitter {
    pub fn new(separator: impl Into<String>) -> Self {
        Self {
            separator: unescape_python(separator.into()),
            max_splits: None,
        }
    }

    pub fn with_max_splits(separator: impl Into<String>, max_splits: usize) -> Self {
        Self {
            separator: unescape_python(separator.into()),
            max_splits: Some(max_splits),
        }
    }

    pub fn split(&self, text: &str) -> Vec<String> {
        if self.separator.is_empty() {
            return vec![text.to_string()];
        }

        let parts: Vec<String> = match self.max_splits {
            Some(max_splits) => text
                .splitn(max_splits + 1, self.separator.as_str())
                .map(str::to_string)
                .collect(),
            None => text
                .split(self.separator.as_str())
                .map(str::to_string)
                .collect(),
        };

        parts
    }

    pub fn to_string(&self) -> String {
        match self.max_splits {
            Some(max_splits) => format!(
                "splitting on \"{}\" into at most {} parts",
                self.separator, max_splits
            ),
            None => format!("splitting on \"{}\"", self.separator),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StringSlicer {
    pub index_start: Option<isize>,
    pub index_end: Option<isize>,
}

impl StringSlicer {
    pub fn new(index_start: Option<isize>, index_end: Option<isize>) -> Self {
        Self {
            index_start,
            index_end,
        }
    }

    pub fn slice<'a>(&self, strings: &'a [String]) -> Vec<&'a str> {
        let length = strings.len() as isize;
        let start = self
            .index_start
            .map(|index| resolve_index(index, length))
            .unwrap_or(0);
        let end = self
            .index_end
            .map(|index| resolve_index(index, length))
            .unwrap_or(length);

        if start >= end {
            return Vec::new();
        }

        strings[start as usize..end as usize]
            .iter()
            .map(String::as_str)
            .collect()
    }

    pub fn to_string(&self) -> String {
        match (self.index_start, self.index_end) {
            (None, None) => "selecting all strings".to_string(),
            (Some(start), None) => {
                format!("selecting {} and onwards", ordinal_start(start))
            }
            (None, Some(end)) => {
                format!("selecting up to and including {}", ordinal_end(end))
            }
            (Some(start), Some(end)) => {
                if start >= end {
                    "selecting nothing".to_string()
                } else {
                    format!(
                        "selecting {} up to and including {}",
                        ordinal_start(start),
                        ordinal_end(end)
                    )
                }
            }
        }
    }
}

fn resolve_index(index: isize, length: isize) -> isize {
    let resolved = if index < 0 { length + index } else { index };

    resolved.clamp(0, length)
}

fn ordinal_start(index: isize) -> String {
    if index < 0 {
        return format!("{} from last string", ordinal_word(-index));
    }

    format!("the {} string", ordinal_word(index + 1))
}

fn ordinal_end(index: isize) -> String {
    if index < 0 {
        return format!("{} from last string", ordinal_word(-index + 1));
    }

    format!("the {} string", ordinal_word(index))
}

fn ordinal_word(value: isize) -> String {
    let suffix = match value % 100 {
        11..=13 => "th",
        _ => match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };

    format!("{value}{suffix}")
}

mod sorter;
use sorter::unescape_python;
pub use sorter::*;

#[cfg(test)]
mod tests;
