
use super::*;

#[test]
fn parses_simple_system_predicates() {
    let predicate = parse_system_predicate("system:inbox").unwrap();

    assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_INBOX);
}

#[test]
fn parses_width_with_operator() {
    let predicate = parse_system_predicate("system:width > 1000").unwrap();
    let PredicateValue::Number(number) = predicate.value else {
        panic!("expected a number test");
    };

    assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_WIDTH);
    assert_eq!(number.operator, NUMBER_TEST_OPERATOR_GREATER_THAN);
    assert_eq!(number.value, 1000.0);
}

#[test]
fn parses_filesize_units() {
    let predicate = parse_system_predicate("system:filesize > 1MB").unwrap();
    let PredicateValue::Number(number) = predicate.value else {
        panic!("expected a number test");
    };

    assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_SIZE);
    assert_eq!(number.value, 1024.0 * 1024.0);
}

#[test]
fn parses_filetype_lists() {
    let predicate = parse_system_predicate("system:filetype is png").unwrap();
    let PredicateValue::Numbers(mimes) = predicate.value else {
        panic!("expected mime numbers");
    };

    assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_MIME);
    assert!(mimes.contains(&(hydrus_core::hydrus_constants::Mime::Png.as_int() as i32)));
}

#[test]
fn parses_limit() {
    let predicate = parse_system_predicate("system:limit = 10").unwrap();
    let PredicateValue::Number(number) = predicate.value else {
        panic!("expected a number test");
    };

    assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_LIMIT);
    assert_eq!(number.value, 10.0);
}

#[test]
fn unsupported_service_predicates_are_errors() {
    assert!(parse_system_predicate("system:rating > 3").is_err());
}

fn services() -> (ServicesManager, std::path::PathBuf) {
    use crate::db::{ClientDB, services};
    use hydrus_core::hydrus_constants::ServiceType;

    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_predicate_services_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let db = ClientDB::new(&directory).unwrap();

    services::get_or_add_service(
        db.db(),
        b"my files",
        ServiceType::LocalFileDomain,
        "my files",
    )
    .unwrap();
    services::get_or_add_service(
        db.db(),
        b"my favourites",
        ServiceType::LocalRatingNumerical,
        "my favourites",
    )
    .unwrap();

    let manager = ServicesManager::load(db.db()).unwrap();

    (manager, directory)
}

#[test]
fn file_service_predicates_resolve_service_names() {
    let (services, directory) = services();

    let predicate = parse_system_predicate_with_services(
        "system:file service is currently in my files",
        &services,
    )
    .unwrap();

    assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_FILE_SERVICE);
    assert_eq!(
        predicate.value,
        PredicateValue::FileService {
            is_in: true,
            status: hydrus_core::hydrus_constants::ContentStatus::Current.as_int() as i32,
            service_key: b"my files".to_vec(),
        }
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn rating_predicates_resolve_service_names() {
    let (services, directory) = services();

    let predicate =
        parse_system_predicate_with_services("system:rating my favourites > 3/5", &services)
            .unwrap();

    assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_RATING);

    let PredicateValue::Service {
        service_key, value, ..
    } = predicate.value
    else {
        panic!("expected a rating service predicate");
    };

    assert_eq!(service_key, b"my favourites".to_vec());
    assert!(value.is_some());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn date_predicates_are_converted() {
    let predicate = parse_system_predicate("system:modified before 2020-01-02").unwrap();

    assert_eq!(
        predicate.predicate_type,
        PREDICATE_TYPE_SYSTEM_MODIFIED_TIME
    );

    let PredicateValue::Date {
        operator,
        date_type,
        values,
    } = predicate.value
    else {
        panic!("expected a date predicate");
    };

    assert_eq!(operator, "<");
    assert_eq!(date_type, "date");
    assert_eq!(&values[..3], &[2020, 1, 2]);
}
