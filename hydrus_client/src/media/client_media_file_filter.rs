//! Media filtering from `hydrus/client/media/ClientMediaFileFilter.py`.

use std::sync::Arc;

use crate::media::MediaSingle;
use crate::search::FileSearchContext;

#[derive(Debug, Clone, Default)]
pub struct MediaFileFilter {
    pub search_context: FileSearchContext,
}

impl MediaFileFilter {
    pub fn new(search_context: FileSearchContext) -> Self {
        Self { search_context }
    }

    pub fn allows(&self, media: &MediaSingle) -> bool {
        self.search_context
            .test_media_result(media.get_media_result())
    }

    pub fn filter(&self, media: &[Arc<MediaSingle>]) -> Vec<Arc<MediaSingle>> {
        media
            .iter()
            .filter(|media| self.allows(media))
            .cloned()
            .collect()
    }

    pub fn filter_hashes(&self, media: &[Arc<MediaSingle>]) -> Vec<Vec<u8>> {
        self.filter(media)
            .into_iter()
            .map(|media| media.get_hash().to_vec())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::MediaResult;
    use crate::search::predicate_constants::PREDICATE_TYPE_SYSTEM_MIME;
    use crate::search::{Predicate, PredicateValue};
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn media(hash: u8, mime: Mime) -> Arc<MediaSingle> {
        Arc::new(MediaSingle::new(MediaResult::new(
            vec![hash],
            FileInfo {
                size: 10,
                mime,
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
    fn empty_context_allows_everything() {
        let filter = MediaFileFilter::default();
        let media = [media(1, Mime::Png), media(2, Mime::Jpeg)];

        assert_eq!(filter.filter(&media).len(), 2);
    }

    #[test]
    fn mime_predicate_filters_media() {
        let mut context = FileSearchContext::new();
        context.set_predicates(vec![Predicate::new(
            PREDICATE_TYPE_SYSTEM_MIME,
            PredicateValue::Numbers(vec![Mime::Png.as_int() as i32]),
        )]);

        let filter = MediaFileFilter::new(context);
        let media = [media(1, Mime::Png), media(2, Mime::Jpeg)];

        assert_eq!(filter.filter_hashes(&media), vec![vec![1]]);
    }
}
