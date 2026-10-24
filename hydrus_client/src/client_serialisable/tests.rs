use super::*;
use crate::client_options::ClientOptions;

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "hydrus_client_{name}_{}_{}.png",
        std::process::id(),
        rand::random::<u64>()
    ))
}

#[test]
fn strings_round_trip_through_png() {
    let path = temp_path("string");
    let payload = "hello, hydrus";

    dump_string_to_png(&path, payload, 64).unwrap();

    assert_eq!(load_string_from_png(&path).unwrap(), payload);

    let _ = std::fs::remove_file(path);
}

#[test]
fn bytes_round_trip_through_png() {
    let path = temp_path("bytes");
    let payload = b"\x00\x01\x02hello";

    dump_bytes_to_png(&path, payload, 32).unwrap();

    assert_eq!(load_bytes_from_png(&path).unwrap(), payload);

    let _ = std::fs::remove_file(path);
}

#[test]
fn serialisable_objects_round_trip() {
    let path = temp_path("object");
    let object = ClientOptions::new();

    dump_object_to_png(&path, &object, 64).unwrap();

    let text = load_string_from_png(&path).unwrap();

    assert!(text.contains("\"advanced_mode\""));
    assert!(text.starts_with("[22, 8"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn invalid_png_is_an_error() {
    let path = temp_path("invalid");
    std::fs::write(&path, b"not a png").unwrap();

    assert!(load_compressed_bytes_from_png(&path).is_err());

    let _ = std::fs::remove_file(path);
}
