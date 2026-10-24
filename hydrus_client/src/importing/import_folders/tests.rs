
use super::*;

#[test]
fn folders_become_due_after_their_period() {
    let mut folder = ImportFolder::new("inbox", "C:/inbox", 60);
    folder.mark_checked(100);

    assert!(!folder.is_due(159));
    assert!(folder.is_due(160));
}

#[test]
fn paused_folders_are_not_due() {
    let mut folder = ImportFolder::new("inbox", "C:/inbox", 60);
    folder.mark_checked(0);
    folder.pause_play();

    assert!(!folder.is_due(1000));
}

#[test]
fn manager_picks_the_earliest_due_folder() {
    let mut manager = ImportFoldersManager::new();
    let mut first = ImportFolder::new("first", "C:/a", 100);
    first.mark_checked(0);
    let mut second = ImportFolder::new("second", "C:/b", 50);
    second.mark_checked(0);
    manager.add_folder(first);
    manager.add_folder(second);

    assert_eq!(manager.get_folder_name_that_is_due(60), Some("second"));
    assert_eq!(manager.get_time_until_next_work(60), Some(0));
}

#[test]
fn folders_can_be_removed_by_name() {
    let mut manager = ImportFoldersManager::new();
    manager.add_folder(ImportFolder::new("inbox", "C:/inbox", 60));

    assert!(manager.remove_folder("inbox"));
    assert!(manager.is_empty());
    assert!(!manager.remove_folder("inbox"));
}

fn temp_folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "hydrus_client_hdd_{name}_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn hdd_import_scans_subdirectories() {
    let directory = temp_folder("scan");
    std::fs::write(directory.join("a.png"), b"a").unwrap();

    let nested = directory.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("b.png"), b"b").unwrap();

    let mut import = HddImport::new(ImportFolder::new("inbox", &directory, 60));
    assert_eq!(import.scan(i64::MAX / 2), 2);
    assert_eq!(import.len(), 2);

    import.folder.search_subdirectories = false;
    assert_eq!(import.scan(i64::MAX / 2), 1);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hdd_import_deletes_and_moves_files() {
    let directory = temp_folder("actions");
    let source = directory.join("a.png");
    std::fs::write(&source, b"a").unwrap();

    let mut delete = HddImport::new(ImportFolder::new("inbox", &directory, 60));
    delete.folder.actions = ImportFolderAction::Delete;
    delete.apply_action(&source).unwrap();
    assert!(!source.exists());

    std::fs::write(&source, b"a").unwrap();

    let destination = directory.join("done");
    let mut move_import = HddImport::new(ImportFolder::new("inbox", &directory, 60));
    move_import.folder.actions = ImportFolderAction::Move;
    move_import.folder.action_locations = Some(destination.clone());

    move_import.apply_action(&source).unwrap();
    assert!(!source.exists());
    assert!(destination.join("a.png").exists());

    let _ = std::fs::remove_dir_all(directory);
}
