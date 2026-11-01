//! Sorting, grouping and summary cells for WS-15.3.2d (R2: pure, tested).
//! Inputs are slices/primitives, outputs are `Vec<usize>`-as-`Vec<i64>` indices
//! or `String`. No Qt/FS/network. Covers audit № 11 (comparator numbers live
//! in `status_texts::comparator_number_text`, re-exported here), № 36
//! (files/checking cells delegate to `render`/`status_texts`), seed cache
//! ordering/filtering and generic row grouping.

/// Stable order of string keys. Returns indices into `keys`.
pub fn sorted_order_strings(keys: &[String], ascending: bool) -> Vec<i64> {
    let mut idx: Vec<usize> = (0..keys.len()).collect();
    idx.sort_by(|&a, &b| {
        let ord = keys[a].cmp(&keys[b]);
        if ascending { ord } else { ord.reverse() }
    });
    idx.into_iter().map(|i| i as i64).collect()
}

/// Stable order of integer keys. Returns indices into `keys`.
pub fn sorted_order_ints(keys: &[i64], ascending: bool) -> Vec<i64> {
    let mut idx: Vec<usize> = (0..keys.len()).collect();
    idx.sort_by(|&a, &b| {
        let ord = keys[a].cmp(&keys[b]);
        if ascending { ord } else { ord.reverse() }
    });
    idx.into_iter().map(|i| i as i64).collect()
}

/// Sort + dedup selected row numbers (review dialog pending selection).
pub fn sorted_selected_rows(rows: &[i64]) -> Vec<i64> {
    let mut out: Vec<i64> = rows.to_vec();
    out.sort_unstable();
    out.dedup();
    out
}

/// Partition `count` rows into consecutive groups of `group_size`.
/// Empty for `count == 0` or `group_size == 0`. Last group may be shorter.
pub fn group_rows(count: usize, group_size: usize) -> Vec<Vec<i64>> {
    if count == 0 || group_size == 0 {
        return Vec::new();
    }
    (0..count)
        .collect::<Vec<_>>()
        .chunks(group_size)
        .map(|c| c.iter().map(|&i| i as i64).collect())
        .collect()
}

/// Visible seed-cache row indices after status filter + url/note needle
/// (case-insensitive substring, empty needle matches all). Stable: input order.
pub fn seed_visible_order(
    statuses: &[String],
    urls: &[String],
    notes: &[String],
    filter: &str,
    needle: &str,
) -> Vec<i64> {
    let needle = needle.trim().to_lowercase();
    let n = statuses.len().min(urls.len()).min(notes.len());
    (0..n)
        .filter(|&i| {
            if filter != "all" && statuses[i] != filter {
                return false;
            }
            if needle.is_empty() {
                return true;
            }
            urls[i].to_lowercase().contains(&needle) || notes[i].to_lowercase().contains(&needle)
        })
        .map(|i| i as i64)
        .collect()
}

/// Seed-cache timestamp cell: empty for non-positive, else `yyyy-MM-dd HH:mm:ss` (UTC).
pub fn seed_timestamp_text(seconds: i64) -> String {
    if seconds <= 0 {
        return String::new();
    }
    let days = seconds.div_euclid(86_400);
    let secs = seconds.rem_euclid(86_400);
    // civil-from-days (Howard Hinnant), days since 1970-01-01.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    y += if m <= 2 { 1 } else { 0 };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y,
        m,
        d,
        secs / 3_600,
        (secs % 3_600) / 60,
        secs % 60
    )
}

/// № 11 canonical: comparator number cell (same as `QString::number(double)` for
/// the values the editor produces).
pub fn comparator_number_text(value: f64) -> String {
    super::status_texts::comparator_number_text(value)
}

/// № 36: files/checking summary cells delegate to the single source of truth.
pub fn files_status_cell(done: i64, total: i64) -> String {
    super::render::downloader_files_status(done, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_single() {
        let e: Vec<String> = vec![];
        assert_eq!(sorted_order_strings(&e, true), Vec::<i64>::new());
        assert_eq!(sorted_order_ints(&[], false), Vec::<i64>::new());
        assert_eq!(sorted_selected_rows(&[]), Vec::<i64>::new());
        assert_eq!(group_rows(0, 3), Vec::<Vec<i64>>::new());
        assert_eq!(group_rows(1, 0), Vec::<Vec<i64>>::new());
        assert_eq!(
            sorted_order_strings(&["b".to_string()], true),
            vec![0]
        );
        assert_eq!(sorted_order_ints(&[7], false), vec![0]);
    }

    #[test]
    fn stable_equal_keys() {
        let keys = vec!["a".to_string(), "a".to_string(), "a".to_string()];
        assert_eq!(sorted_order_strings(&keys, true), vec![0, 1, 2]);
        assert_eq!(sorted_order_strings(&keys, false), vec![0, 1, 2]);
        assert_eq!(sorted_order_ints(&[5, 5, 5], true), vec![0, 1, 2]);
    }

    #[test]
    fn reverse_order() {
        let keys = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(sorted_order_strings(&keys, false), vec![2, 1, 0]);
        assert_eq!(sorted_order_ints(&[1, 2, 3], false), vec![2, 1, 0]);
        assert_eq!(sorted_order_ints(&[3, 1, 2], true), vec![1, 2, 0]);
    }

    #[test]
    fn selected_rows_sort_dedup() {
        assert_eq!(sorted_selected_rows(&[3, 1, 3, 2, 1]), vec![1, 2, 3]);
        assert_eq!(sorted_selected_rows(&[5]), vec![5]);
    }

    #[test]
    fn nested_groups() {
        assert_eq!(
            group_rows(7, 3),
            vec![vec![0, 1, 2], vec![3, 4, 5], vec![6]]
        );
        assert_eq!(group_rows(2, 5), vec![vec![0, 1]]);
        assert_eq!(group_rows(3, 1), vec![vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn seed_filter() {
        let st = vec!["ok".to_string(), "failed".to_string(), "ok".to_string()];
        let urls = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let notes = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        assert_eq!(
            seed_visible_order(&st, &urls, &notes, "all", ""),
            vec![0, 1, 2]
        );
        assert_eq!(
            seed_visible_order(&st, &urls, &notes, "ok", ""),
            vec![0, 2]
        );
        assert_eq!(seed_visible_order(&st, &urls, &notes, "all", "B"), vec![1]);
        assert_eq!(
            seed_visible_order(&st, &urls, &notes, "failed", "zzz"),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn seed_time() {
        assert_eq!(seed_timestamp_text(0), "");
        assert_eq!(seed_timestamp_text(-5), "");
        assert_eq!(seed_timestamp_text(1_700_000_000), "2023-11-14 22:13:20");
    }

    #[test]
    fn cells_delegate() {
        assert_eq!(comparator_number_text(2.0), "2");
        assert!(!files_status_cell(1, 2).is_empty());
    }
}
