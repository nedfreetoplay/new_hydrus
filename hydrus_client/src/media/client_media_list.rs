//! `hydrus/client/media/ClientMediaList.py`

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::client_location::LocationContext;
use crate::media::client_media::MediaSingle;
use crate::media::client_media_result::MediaResult;
use crate::media::client_media_sort::MediaSort;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaList {
    pub location_context: LocationContext,
    media: Vec<Arc<MediaSingle>>,
    selected_hashes: BTreeSet<Vec<u8>>,
}

impl MediaList {
    pub fn new(location_context: LocationContext) -> Self {
        Self {
            location_context,
            media: Vec::new(),
            selected_hashes: BTreeSet::new(),
        }
    }

    pub fn add_media(&mut self, media: Arc<MediaSingle>) {
        self.media.push(media);
    }

    pub fn add_media_results(&mut self, results: impl IntoIterator<Item = MediaResult>) {
        for result in results {
            self.media.push(Arc::new(MediaSingle::new(result)));
        }
    }

    pub fn get_media(&self) -> &[Arc<MediaSingle>] {
        &self.media
    }

    pub fn get_first(&self) -> Option<&Arc<MediaSingle>> {
        self.media.first()
    }

    pub fn get_last(&self) -> Option<&Arc<MediaSingle>> {
        self.media.last()
    }

    pub fn get_next(&self, media: &MediaSingle) -> Option<&Arc<MediaSingle>> {
        self.position_of(media)
            .and_then(|index| self.media.get(index + 1))
    }

    pub fn get_previous(&self, media: &MediaSingle) -> Option<&Arc<MediaSingle>> {
        self.position_of(media)
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| self.media.get(index))
    }

    fn position_of(&self, media: &MediaSingle) -> Option<usize> {
        self.media
            .iter()
            .position(|candidate| candidate.get_hash() == media.get_hash())
    }

    pub fn get_num_files(&self) -> usize {
        self.media.len()
    }

    pub fn get_num_inbox(&self) -> usize {
        self.media.iter().filter(|media| media.has_inbox()).count()
    }

    pub fn get_num_archive(&self) -> usize {
        self.get_num_files() - self.get_num_inbox()
    }

    pub fn get_hashes(&self) -> Vec<Vec<u8>> {
        self.media.iter().map(|media| media.get_hash().to_vec()).collect()
    }

    pub fn get_media_results(&self) -> Vec<&MediaResult> {
        self.media
            .iter()
            .map(|media| media.get_media_result())
            .collect()
    }

    pub fn select(&mut self, hash: Vec<u8>) {
        self.selected_hashes.insert(hash);
    }

    pub fn deselect(&mut self, hash: &[u8]) {
        self.selected_hashes.remove(hash);
    }

    pub fn select_all(&mut self) {
        self.selected_hashes = self.get_hashes().into_iter().collect();
    }

    pub fn clear_selection(&mut self) {
        self.selected_hashes.clear();
    }

    pub fn get_selected_hashes(&self) -> &BTreeSet<Vec<u8>> {
        &self.selected_hashes
    }

    pub fn get_selected_media(&self) -> Vec<&Arc<MediaSingle>> {
        self.media
            .iter()
            .filter(|media| self.selected_hashes.contains(media.get_hash()))
            .collect()
    }

    pub fn get_sorted_media(&self, sort: &MediaSort) -> Vec<Arc<MediaSingle>> {
        let mut media = self.media.clone();
        sort.sort_media(&mut media);
        media
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydrus_core::hydrus_constants::Mime;
    use hydrus_core::files::hydrus_file_handling::FileInfo;

    fn result(hash: u8) -> MediaResult {
        MediaResult::new(
            vec![hash],
            FileInfo {
                size: hash as u64,
                mime: Mime::Png,
                width: None,
                height: None,
                duration_ms: None,
                num_frames: None,
                has_audio: false,
                num_words: None,
            },
        )
    }

    #[test]
    fn navigation_walks_the_list() {
        let mut list = MediaList::new(LocationContext::default());
        list.add_media_results([result(1), result(2), result(3)]);

        let first = list.get_first().unwrap().clone();
        let second = list.get_next(first.as_ref()).unwrap().clone();

        assert_eq!(second.get_hash(), &[2]);
        assert_eq!(list.get_previous(second.as_ref()).unwrap().get_hash(), &[1]);
    }

    #[test]
    fn selection_is_hash_based() {
        let mut list = MediaList::new(LocationContext::default());
        list.add_media_results([result(1), result(2)]);
        list.select(vec![2]);

        assert_eq!(list.get_selected_media().len(), 1);
        assert_eq!(list.get_selected_hashes().len(), 1);
    }

    #[test]
    fn sorted_media_does_not_mutate_the_list() {
        let mut list = MediaList::new(LocationContext::default());
        list.add_media_results([result(3), result(1), result(2)]);

        let sorted = list.get_sorted_media(&MediaSort::default());

        assert_eq!(sorted[0].get_size(), 1);
        assert_eq!(list.get_hashes()[0], vec![3]);
    }
}
