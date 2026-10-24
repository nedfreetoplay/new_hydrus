use super::*;

fn posts_json() -> String {
    serde_json::json!({
        "posts": [
            {"a": "blah1", "id": 123},
            {"a": "blah2", "id": 456},
            {"a": "blah7", "id": 789},
        ]
    })
    .to_string()
}

fn run(rules: Vec<JsonRule>, content: JsonContent, text: &str) -> Vec<String> {
    JsonFormula::new(rules, content).parse(&Default::default(), text, true)
}

#[test]
fn indexed_items_are_fetched() {
    let j_text = serde_json::json!({ "posts": [1, 2, 3] }).to_string();
    let rules = vec![
        JsonRule::DictKey(StringMatch::fixed("posts")),
        JsonRule::IndexedItem(1),
    ];

    assert_eq!(run(rules, JsonContent::String, &j_text), ["2"]);
}

#[test]
fn all_items_and_dict_keys_work() {
    let rules = vec![
        JsonRule::DictKey(StringMatch::fixed("posts")),
        JsonRule::AllItems,
        JsonRule::DictKey(StringMatch::fixed("id")),
    ];

    assert_eq!(
        run(rules, JsonContent::String, &posts_json()),
        ["123", "456", "789"]
    );

    let rules = vec![
        JsonRule::DictKey(StringMatch::fixed("posts")),
        JsonRule::IndexedItem(2),
    ];

    let keys = run(rules, JsonContent::DictKeys, &posts_json());
    assert_eq!(keys, ["a", "id"]);
}

#[test]
fn raw_json_objects_are_returned() {
    let rules = vec![
        JsonRule::DictKey(StringMatch::fixed("posts")),
        JsonRule::AllItems,
    ];

    let result = run(rules, JsonContent::Json, &posts_json());
    let loaded: Vec<Value> = result
        .iter()
        .map(|text| serde_json::from_str(text).unwrap())
        .collect();

    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0]["id"], 123);
}

#[test]
fn string_test_filters_items() {
    let rules = vec![
        JsonRule::DictKey(StringMatch::fixed("posts")),
        JsonRule::AllItems,
        JsonRule::DictKey(StringMatch::fixed("a")),
        JsonRule::TestStringItems(StringMatch::fixed("blah2")),
    ];

    assert_eq!(run(rules, JsonContent::String, &posts_json()), ["blah2"]);
}

#[test]
fn ascend_returns_to_the_parent_node() {
    let j_text = serde_json::json!({
        "posts": [
            {"a": "blah1", "id": 123},
            {"a": "blah2", "id": 456},
            {"b": "blah7", "id": 789},
        ]
    })
    .to_string();

    let rules = vec![
        JsonRule::DictKey(StringMatch::fixed("posts")),
        JsonRule::AllItems,
        JsonRule::DictKey(StringMatch::fixed("a")),
        JsonRule::Ascend(1),
        JsonRule::DictKey(StringMatch::fixed("id")),
    ];

    assert_eq!(run(rules, JsonContent::String, &j_text), ["123", "456"]);
}

struct DummyFormula(Vec<String>);

impl ParseFormula for DummyFormula {
    fn parse(
        &self,
        _context: &ParsingContext,
        _text: &str,
        _collapse_newlines: bool,
    ) -> Vec<String> {
        self.0.clone()
    }

    fn name(&self) -> &str {
        "dummy"
    }
}

#[test]
fn zipper_combines_formula_outputs() {
    let zipper = ZipperFormula::new(
        vec![
            Box::new(DummyFormula(vec!["test —” test".to_string()])),
            Box::new(DummyFormula(vec!["test ’“ test".to_string()])),
        ],
        "\\1 \\2",
    );

    assert_eq!(
        zipper.parse(&Default::default(), "gumpf", false),
        ["test —” test test ’“ test"]
    );
}
