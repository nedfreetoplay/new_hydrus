
use super::*;

fn sample() -> Vec<String> {
    ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]
        .iter()
        .map(|value| value.to_string())
        .collect()
}

#[test]
fn splitter_honours_separator_and_max_splits() {
    let splitter = StringSplitter::new(", ");
    assert_eq!(splitter.split("1,2,3"), vec!["1,2,3"]);
    assert_eq!(splitter.split("1, 2, 3"), vec!["1", "2", "3"]);

    let splitter = StringSplitter::with_max_splits(", ", 2);
    assert_eq!(splitter.split("1, 2, 3, 4"), vec!["1", "2", "3, 4"]);
    assert_eq!(splitter.split("123"), vec!["123"]);
}

#[test]
fn splitter_unescapes_newlines() {
    assert_eq!(StringSplitter::new(r"\n").split("1\n2"), vec!["1", "2"]);
}

#[test]
fn slicer_uses_python_slice_semantics() {
    let slicer = StringSlicer::new(Some(3), Some(5));
    assert_eq!(slicer.slice(&sample()), vec!["d", "e"]);

    let slicer = StringSlicer::new(Some(-3), None);
    assert_eq!(slicer.slice(&sample()), vec!["h", "i", "j"]);

    let slicer = StringSlicer::new(None, Some(-3));
    assert_eq!(
        slicer.slice(&sample()),
        vec!["a", "b", "c", "d", "e", "f", "g"]
    );

    let slicer = StringSlicer::new(Some(5), Some(3));
    assert!(slicer.slice(&sample()).is_empty());
}

#[test]
fn lexicographic_sort_with_regex_keys() {
    let strings: Vec<String> = ["a 5", "b 2", "c 10", "d 7", "e"]
        .iter()
        .map(|value| value.to_string())
        .collect();

    let sorter = StringSorter::new(SortType::Lexicographic, true, Some(r"\d+".to_string()));
    assert_eq!(sorter.sort(&strings), ["c 10", "b 2", "a 5", "d 7", "e"]);

    let sorter = StringSorter::new(SortType::Lexicographic, false, Some(r"\d+".to_string()));
    assert_eq!(sorter.sort(&strings), ["d 7", "a 5", "b 2", "c 10", "e"]);
}

#[test]
fn human_sort_compares_numbers() {
    let strings: Vec<String> = ["a 5", "b 2", "c 10", "d 7", "e"]
        .iter()
        .map(|value| value.to_string())
        .collect();

    let sorter = StringSorter::new(SortType::HumanSort, true, Some(r"\d+".to_string()));

    assert_eq!(sorter.sort(&strings), ["b 2", "a 5", "d 7", "c 10", "e"]);
}
