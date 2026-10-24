
use super::*;

fn base() -> FilesStorageBaseLocation {
    FilesStorageBaseLocation::new("C:/hydrus/files", 10, None)
}

#[test]
fn unlimited_locations_accept_everything() {
    assert!(base().able_to_accept_subfolders(u64::MAX - 1, 1));
    assert!(!base().needs_to_remove_subfolders(u64::MAX));
    assert!(base().has_no_upper_limit());
}

#[test]
fn limited_locations_respect_the_ceiling() {
    let location = FilesStorageBaseLocation::new("C:/hydrus/files", 10, Some(100));

    assert!(location.able_to_accept_subfolders(50, 50));
    assert!(!location.able_to_accept_subfolders(50, 51));
    assert!(location.needs_to_remove_subfolders(101));
}

#[test]
fn ideal_weights_are_proportional() {
    let locations = [
        FilesStorageBaseLocation::new("a", 1, None),
        FilesStorageBaseLocation::new("b", 3, None),
    ];

    let weights = FilesStorageBaseLocation::static_get_ideal_weights(400, &locations);

    assert_eq!(weights[Path::new("a")], 100.0);
    assert_eq!(weights[Path::new("b")], 300.0);
}

#[test]
fn subfolder_paths_match_hydrus_layout() {
    let subfolder = FilesStorageSubfolder::new("fa", base());
    let path = subfolder.get_file_path("abc");

    assert!(path.ends_with("fa/abc"));
    assert!(subfolder.is_for_files());
    assert_eq!(subfolder.get_prefix_directories_longest_first().len(), 2);
}

#[test]
fn multi_character_prefixes_nest_like_python() {
    let subfolder = FilesStorageSubfolder::new("fab3", base());

    assert_eq!(
        subfolder.path(),
        Path::new("C:/hydrus/files").join("fab").join("3")
    );
    assert!(subfolder.get_file_path("abc").ends_with("fab/3/abc"));
}

#[test]
fn python_weight_rules_match() {
    let subfolder_size = 1_048_567u64 * 122;
    let weight_of_subfolder = 1.0 / 256.0;

    let unlimited = FilesStorageBaseLocation::new("C:/hydrus/files", 2, None);
    let current = weight_of_subfolder * 16.0;

    assert!(!unlimited.needs_to_remove_subfolders(subfolder_size * 16));
    assert!(!unlimited.would_like_to_remove_subfolders(current, 5, weight_of_subfolder));
    assert!(unlimited.eager_to_accept_subfolders(
        current,
        5,
        weight_of_subfolder,
        subfolder_size * 16,
        subfolder_size
    ));

    let current = weight_of_subfolder * 128.0;

    assert!(unlimited.would_like_to_remove_subfolders(current, 5, weight_of_subfolder));
    assert!(!unlimited.eager_to_accept_subfolders(
        current,
        5,
        weight_of_subfolder,
        subfolder_size * 128,
        subfolder_size
    ));
    assert!(unlimited.able_to_accept_subfolders(subfolder_size * 128, subfolder_size));

    let limited = FilesStorageBaseLocation::new("C:/hydrus/files", 2, Some(subfolder_size * 20));

    assert!(!limited.needs_to_remove_subfolders(subfolder_size * 16));
    assert!(limited.needs_to_remove_subfolders(subfolder_size * 32));
    assert!(!limited.able_to_accept_subfolders(subfolder_size * 32, subfolder_size));
}

#[test]
fn zero_weight_rejects_and_wants_removal() {
    let location = FilesStorageBaseLocation::new("C:/hydrus/files", 0, None);

    assert!(!location.able_to_accept_subfolders(0, 1));
    assert!(location.needs_to_remove_subfolders(0));
    assert!(location.would_like_to_remove_subfolders(0.0, 5, 0.1));
}

#[test]
fn hashed_file_path_uses_core_prefix() {
    let path = get_file_path(Path::new("C:/hydrus/files"), &[0xab, 0xcd], 2, "file");

    assert!(path.ends_with("fab/file"));
}
