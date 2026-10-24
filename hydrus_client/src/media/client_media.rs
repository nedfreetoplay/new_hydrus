//! `hydrus/client/media/ClientMedia.py` and `ClientMediaSingle.py`.

use std::sync::Arc;

use crate::media::client_media_result::MediaResult;

#[derive(Debug, Clone, PartialEq)]
pub struct MediaSingle {
    pub media_result: MediaResult,
}

impl MediaSingle {
    pub fn new(media_result: MediaResult) -> Self {
        Self { media_result }
    }

    pub fn get_media_result(&self) -> &MediaResult {
        &self.media_result
    }

    pub fn get_hash(&self) -> &[u8] {
        self.media_result.get_hash()
    }

    pub fn get_size(&self) -> u64 {
        self.media_result.get_size()
    }

    pub fn get_mime(&self) -> hydrus_core::hydrus_constants::Mime {
        self.media_result.get_mime()
    }

    pub fn get_num_files(&self) -> usize {
        1
    }

    pub fn has_inbox(&self) -> bool {
        self.media_result.get_inbox()
    }
}

/// `MediaCollection`: a group of single media displayed as one item.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaCollection {
    pub media: Vec<Arc<MediaSingle>>,
}

impl MediaCollection {
    pub fn new(media: Vec<Arc<MediaSingle>>) -> Self {
        Self { media }
    }

    pub fn add_media(&mut self, media: Arc<MediaSingle>) {
        self.media.push(media);
    }

    pub fn get_num_files(&self) -> usize {
        self.media.len()
    }

    pub fn get_num_inbox(&self) -> usize {
        self.media.iter().filter(|media| media.has_inbox()).count()
    }

    pub fn get_display_media(&self) -> Option<&Arc<MediaSingle>> {
        self.media.first()
    }

    pub fn get_total_size(&self) -> u64 {
        self.media.iter().map(|media| media.get_size()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn single(size: u64) -> Arc<MediaSingle> {
        Arc::new(MediaSingle::new(MediaResult::new(
            vec![size as u8],
            FileInfo {
                size,
                mime: Mime::Png,
                width: Some(10),
                height: Some(10),
                duration_ms: None,
                num_frames: None,
                has_audio: false,
                num_words: None,
            },
        )))
    }

    #[test]
    fn single_media_counts_as_one_file() {
        assert_eq!(single(10).get_num_files(), 1);
    }

    #[test]
    fn collection_totals_its_members() {
        let mut collection = MediaCollection::default();
        collection.add_media(single(10));
        collection.add_media(single(20));

        assert_eq!(collection.get_num_files(), 2);
        assert_eq!(collection.get_total_size(), 30);
    }
}
