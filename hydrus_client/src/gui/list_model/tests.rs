
use super::*;

fn model() -> ListModel {
    let mut model = ListModel::new(vec![
        ListColumn::new("name", true),
        ListColumn::new("size", true),
    ]);
    model.add_row(ListRow::new([
        ("name".to_string(), "beta".to_string()),
        ("size".to_string(), "20".to_string()),
    ]));
    model.add_row(ListRow::new([
        ("name".to_string(), "alpha".to_string()),
        ("size".to_string(), "10".to_string()),
    ]));
    model
}

#[test]
fn sorting_respects_direction() {
    let mut model = model();

    assert!(model.sort_by("name", true));
    assert_eq!(model.get_row(0).unwrap().get("name"), Some("alpha"));

    model.sort_by("name", false);
    assert_eq!(model.get_row(0).unwrap().get("name"), Some("beta"));

    assert_eq!(model.get_sort(), Some(("name", false)));
}

#[test]
fn unknown_or_unsortable_columns_are_rejected() {
    let mut model = ListModel::new(vec![ListColumn::new("name", false)]);
    model.add_row(ListRow::new([("name".to_string(), "x".to_string())]));

    assert!(!model.sort_by("name", true));
    assert!(!model.sort_by("missing", true));
}

#[test]
fn selection_can_be_additive() {
    let mut model = model();

    model.select(0, false);
    model.select(1, true);

    assert_eq!(model.get_selected_indices().len(), 2);

    model.select(0, false);
    assert_eq!(model.get_selected_indices().len(), 1);
}

#[test]
fn filtering_matches_any_cell() {
    let model = model();

    assert_eq!(model.filter_rows("alp"), vec![1]);
    assert_eq!(model.filter_rows("2").len(), 1);
}

#[test]
fn list_box_tracks_checked_items() {
    let mut list_box = ListBox::new();
    let first = list_box.add_item("one");
    list_box.add_item("two");
    list_box.set_checked(first, true);

    assert!(list_box.is_checked(first));
    assert_eq!(list_box.get_checked_items(), vec![&"one".to_string()]);

    list_box.remove_item(first);
    assert!(!list_box.is_checked(first));
}
