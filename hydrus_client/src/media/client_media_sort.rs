//! Sorting helpers from `hydrus/client/media/ClientMediaSort.py`.

use std::cmp::Ordering;
use std::sync::Arc;

use crate::client_constants::{self, SortOrder, SortType};
use crate::media::client_media::MediaSingle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSort {
    pub sort_type: SortType,
    pub sort_order: SortOrder,
}

impl Default for MediaSort {
    fn default() -> Self {
        Self {
            sort_type: SortType::FileSize,
            sort_order: client_constants::SortOrder::Ascending,
        }
    }
}

impl MediaSort {
    pub fn new(sort_type: SortType, sort_order: SortOrder) -> Self {
        Self {
            sort_type,
            sort_order,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}, {}", self.sort_type, self.sort_order)
    }

    pub fn sort_media(&self, media: &mut [Arc<MediaSingle>]) {
        media.sort_by(|a, b| {
            let ordering = match self.sort_type {
                SortType::FileSize => a.get_size().cmp(&b.get_size()),
                SortType::Mime => (a.get_mime().as_int()).cmp(&b.get_mime().as_int()),
                SortType::Hash => a.get_hash().cmp(b.get_hash()),
                _ => Ordering::Equal,
            };

            match self.sort_order {
                SortOrder::Descending => ordering.reverse(),
                _ => ordering,
            }
        });
    }
}

/// `MediaCollect`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaCollect {
    pub collect_by: Vec<i32>,
}

impl MediaCollect {
    pub fn is_empty(&self) -> bool {
        self.collect_by.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::client_media_result::MediaResult;
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn media(size: u64, timestamps: u8) -> Arc<MediaSingle> {
        Arc::new(MediaSingle::new(MediaResult::new(
            vec![timestamps],
            FileInfo {
                size,
                mime: Mime::Png,
                width: None,
                height: None,
                duration_ms: None,
                num_frames: None,
                has_audio: false,
                num_words: None,
            },
        )))
    }

    #[test]
    fn filesize_sort_ascending() {
        let mut values = vec![media(30, 1), media(10, 2), media(20, 3)];

        MediaSort::new(SortType::FileSize, client_constants::SortOrder::Ascending)
            .sort_media(&mut values);

        assert_eq!(
            values
                .iter()
                .map(|media| media.get_size())
                .collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
    }

    #[test]
    fn filesize_sort_descending() {
        let mut values = vec![media(30, 1), media(10, 2), media(20, 3)];

        MediaSort::new(SortType::FileSize, client_constants::SortOrder::Descending)
            .sort_media(&mut values);

        assert_eq!(
            values
                .iter()
                .map(|media| media.get_size())
                .collect::<Vec<_>>(),
            vec![30, 20, 10]
        );
    }
}
