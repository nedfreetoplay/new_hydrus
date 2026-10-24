use super::*;

#[test]
fn duplicate_seeds_are_not_added() {
    let mut cache = FileSeedCache::new();

    assert!(cache.add_seed(FileSeed::url("https://example.com/a", 0)));
    assert!(!cache.add_seed(FileSeed::url("https://example.com/a", 1)));
    assert_eq!(cache.get_num_seeds(), 1);
}

#[test]
fn status_changes_are_tracked() {
    let mut cache = FileSeedCache::new();
    cache.add_seed(FileSeed::url("https://example.com/a", 0));

    assert_eq!(cache.get_undone_seeds().len(), 1);

    cache.set_status(
        "https://example.com/a",
        client_constants::STATUS_SUCCESSFUL_AND_NEW,
        10,
    );

    assert_eq!(cache.get_num_done(), 1);
    assert_eq!(
        cache
            .get_seeds_with_status(client_constants::STATUS_SUCCESSFUL_AND_NEW)
            .len(),
        1
    );
}

#[test]
fn seeds_can_carry_hashes_and_notes() {
    let mut cache = FileSeedCache::new();
    cache.add_seed(FileSeed::url("https://example.com/a", 0));
    let seed = cache.get_seed_mut("https://example.com/a").unwrap();

    seed.set_hash(vec![1, 2], 5);
    seed.set_note("downloaded", 5);

    assert_eq!(seed.hash, Some(vec![1, 2]));
    assert_eq!(seed.note, "downloaded");
}

#[test]
fn hdd_seeds_have_the_right_type() {
    let seed = FileSeed::hdd("C:/inbox/a.png", 0);

    assert_eq!(seed.file_seed_type, FILE_SEED_TYPE_HDD);
    assert_eq!(seed.file_seed_data, "C:/inbox/a.png");
}
