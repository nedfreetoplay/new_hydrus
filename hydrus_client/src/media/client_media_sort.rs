//! Sorting helpers from `hydrus/client/media/ClientMediaSort.py`.

use std::cmp::Ordering;
use std::sync::Arc;

use crate::client_constants;
use crate::media::client_media::MediaSingle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSort {
    pub sort_type: i32,
    pub sort_order: i32,
}

impl Default for MediaSort {
    fn default() -> Self {
        Self {
            sort_type: client_constants::SORT_FILES_BY_FILESIZE,
            sort_order: client_constants::SORT_ASC,
        }
    }
}

impl MediaSort {
    pub fn new(sort_type: i32, sort_order: i32) -> Self {
        Self {
            sort_type,
            sort_order,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}, {}",
            client_constants::sort_type_string(self.sort_type),
            client_constants::sort_order_string(self.sort_order)
        )
    }

    pub fn sort_media(&self, media: &mut [Arc<MediaSingle>]) {
        media.sort_by(|a, b| {
            let ordering = match self.sort_type {
                client_constants::SORT_FILES_BY_FILESIZE => a.get_size().cmp(&b.get_size()),
                client_constants::SORT_FILES_BY_MIME => {
                    (a.get_mime().as_int()).cmp(&b.get_mime().as_int())
                }
                client_constants::SORT_FILES_BY_HASH => a.get_hash().cmp(b.get_hash()),
                _ => Ordering::Equal,
            };

            match self.sort_order {
                client_constants::SORT_DESC => ordering.reverse(),
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

        MediaSort::new(
            client_constants::SORT_FILES_BY_FILESIZE,
            client_constants::SORT_ASC,
        )
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

        MediaSort::new(
            client_constants::SORT_FILES_BY_FILESIZE,
            client_constants::SORT_DESC,
        )
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
