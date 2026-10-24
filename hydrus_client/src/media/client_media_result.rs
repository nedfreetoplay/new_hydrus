//! `hydrus/client/media/ClientMediaResult.py`

use hydrus_core::files::hydrus_file_handling::FileInfo;

use crate::media::client_media_managers::{
    FileInfoManager, FileViewingStatsManager, LocationsManager, NotesManager, TagsManager,
    TimesManager,
};
use crate::metadata::RatingsManager;

#[derive(Debug, Clone, PartialEq)]
pub struct MediaResult {
    pub hash: Vec<u8>,
    pub file_info_manager: FileInfoManager,
    pub times_manager: TimesManager,
    pub locations_manager: LocationsManager,
    pub notes_manager: NotesManager,
    pub ratings_manager: RatingsManager,
    pub tags_manager: TagsManager,
    pub viewing_stats: FileViewingStatsManager,
}

impl MediaResult {
    pub fn new(hash: Vec<u8>, file_info: FileInfo) -> Self {
        Self {
            file_info_manager: FileInfoManager::new(hash.clone(), file_info),
            hash,
            times_manager: TimesManager::default(),
            locations_manager: LocationsManager::default(),
            notes_manager: NotesManager::default(),
            ratings_manager: RatingsManager::default(),
            tags_manager: TagsManager::default(),
            viewing_stats: FileViewingStatsManager::default(),
        }
    }

    pub fn get_hash(&self) -> &[u8] {
        &self.hash
    }

    pub fn get_size(&self) -> u64 {
        self.file_info_manager.get_size()
    }

    pub fn get_mime(&self) -> hydrus_core::hydrus_constants::Mime {
        self.file_info_manager.get_mime()
    }

    pub fn get_resolution(&self) -> Option<(u32, u32)> {
        self.file_info_manager.get_resolution()
    }

    pub fn get_duration_ms(&self) -> Option<i64> {
        self.file_info_manager.file_info.duration_ms
    }

    pub fn get_num_frames(&self) -> Option<u32> {
        self.file_info_manager.file_info.num_frames
    }

    pub fn get_num_words(&self) -> Option<i64> {
        self.file_info_manager.file_info.num_words
    }

    pub fn has_duration(&self) -> bool {
        self.get_duration_ms().is_some()
    }

    pub fn has_audio(&self) -> bool {
        self.file_info_manager.has_audio()
    }

    pub fn has_notes(&self) -> bool {
        self.notes_manager.has_notes()
    }

    pub fn has_useful_resolution(&self) -> bool {
        self.get_resolution().is_some()
    }

    pub fn get_inbox(&self) -> bool {
        self.locations_manager.inbox
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydrus_core::hydrus_constants::Mime;

    fn result() -> MediaResult {
        MediaResult::new(
            vec![0xaa],
            FileInfo {
                size: 4096,
                mime: Mime::Mp4Video,
                width: Some(1280),
                height: Some(720),
                duration_ms: Some(5000),
                num_frames: Some(150),
                has_audio: true,
                num_words: None,
            },
        )
    }

    #[test]
    fn media_result_exposes_file_info() {
        let media = result();

        assert_eq!(media.get_hash(), &[0xaa]);
        assert_eq!(media.get_size(), 4096);
        assert_eq!(media.get_resolution(), Some((1280, 720)));
        assert_eq!(media.get_duration_ms(), Some(5000));
        assert!(media.has_duration());
        assert!(media.has_audio());
    }

    #[test]
    fn inbox_comes_from_locations_manager() {
        let mut media = result();
        assert!(!media.get_inbox());

        media.locations_manager.inbox = true;

        assert!(media.get_inbox());
    }
}
