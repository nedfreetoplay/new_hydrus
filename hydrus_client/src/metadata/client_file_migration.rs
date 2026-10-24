//! File migration helpers from `hydrus/client/metadata/ClientFileMigration.py`.

use hydrus_core::hydrus_constants::ContentType;

use crate::metadata::{ContentAction, ContentUpdate, ContentUpdatePackage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveAction {
    Move,
    Duplicate,
}

impl MoveAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Duplicate => "duplicate",
        }
    }
}

/// Builds the content updates for moving/duplicating files into a local file
/// domain, mirroring `DoMoveOrDuplicateLocalFiles` before it talks to the DB.
pub fn build_local_file_migration_package(
    dest_service_key: Vec<u8>,
    action: MoveAction,
    hashes: impl IntoIterator<Item = Vec<u8>>,
    source_service_key: Option<Vec<u8>>,
) -> ContentUpdatePackage {
    let hashes: Vec<Vec<u8>> = hashes.into_iter().collect();
    let mut updates = Vec::new();

    if hashes.is_empty() {
        return ContentUpdatePackage::new();
    }

    updates.push(
        ContentUpdate::new(ContentAction::Add, dest_service_key, ContentType::Files)
            .with_hashes(hashes.clone()),
    );

    if action == MoveAction::Move {
        if let Some(source_service_key) = source_service_key {
            updates.push(
                ContentUpdate::new(ContentAction::Delete, source_service_key, ContentType::Files)
                    .with_hashes(hashes),
            );
        }
    }

    ContentUpdatePackage::from_updates(updates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_adds_to_destination_and_deletes_from_source() {
        let package = build_local_file_migration_package(
            b"dest".to_vec(),
            MoveAction::Move,
            [vec![1], vec![2]],
            Some(b"source".to_vec()),
        );

        assert_eq!(package.updates.len(), 2);
        assert_eq!(package.updates[0].action, ContentAction::Add);
        assert_eq!(package.updates[1].action, ContentAction::Delete);
        assert_eq!(package.updates[1].hashes.len(), 2);
    }

    #[test]
    fn duplicate_only_adds_to_destination() {
        let package = build_local_file_migration_package(
            b"dest".to_vec(),
            MoveAction::Duplicate,
            [vec![1]],
            Some(b"source".to_vec()),
        );

        assert_eq!(package.updates.len(), 1);
        assert_eq!(package.updates[0].action, ContentAction::Add);
    }

    #[test]
    fn empty_hash_list_produces_no_updates() {
        let package = build_local_file_migration_package(
            b"dest".to_vec(),
            MoveAction::Move,
            Vec::new(),
            None,
        );

        assert!(package.is_empty());
    }
}
