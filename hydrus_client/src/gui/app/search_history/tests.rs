use super::*;

#[test]
fn recent_searches_are_deduped_and_capped() {
    let mut history = Vec::new();

    record(&mut history, "one");
    record(&mut history, "two");
    record(&mut history, "one");
    record(&mut history, "  ");

    assert_eq!(history, vec!["one", "two"]);

    for index in 0..(LIMIT + 5) {
        record(&mut history, &format!("query {index}"));
    }

    assert_eq!(history.len(), LIMIT);
    assert_eq!(history[0], format!("query {}", LIMIT + 4));
}
