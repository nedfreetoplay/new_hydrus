use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct MappingCountsCache {
    entries: BTreeMap<(i64, i64, i64), (i64, i64)>,
    dirty: bool,
}

impl MappingCountsCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, file_service_id: i64, tag_service_id: i64, tag_id: i64) -> (i64, i64) {
        self.entries
            .get(&(file_service_id, tag_service_id, tag_id))
            .copied()
            .unwrap_or((0, 0))
    }

    pub fn set(
        &mut self,
        file_service_id: i64,
        tag_service_id: i64,
        tag_id: i64,
        current: i64,
        pending: i64,
    ) {
        self.entries.insert(
            (file_service_id, tag_service_id, tag_id),
            (current, pending),
        );
        self.dirty = true;
    }

    pub fn add(
        &mut self,
        file_service_id: i64,
        tag_service_id: i64,
        tag_id: i64,
        current_delta: i64,
        pending_delta: i64,
    ) {
        let (current, pending) = self.get(file_service_id, tag_service_id, tag_id);
        self.set(
            file_service_id,
            tag_service_id,
            tag_id,
            current + current_delta,
            pending + pending_delta,
        );
    }

    pub fn remove(&mut self, file_service_id: i64, tag_service_id: i64, tag_id: i64) -> bool {
        self.dirty = true;
        self.entries
            .remove(&(file_service_id, tag_service_id, tag_id))
            .is_some()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_clean(&mut self) {
        self.dirty = false;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
