//! Rust port of `hydrus/core/HydrusLists.py`.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::time::{Duration, Instant};

use rand::Rng;
use rand::seq::SliceRandom;

use crate::hydrus_exceptions::{ErrorKind, HydrusException};

/// `dedupe_list`: keep first occurrences, preserve order.
pub fn dedupe_list<T, I>(xs: I) -> Vec<T>
where
    T: Eq + Hash + Clone,
    I: IntoIterator<Item = T>,
{
    let mut seen: HashSet<T> = HashSet::new();
    let mut result: Vec<T> = Vec::new();

    for x in xs {
        if seen.insert(x.clone()) {
            result.push(x);
        }
    }

    result
}

/// `FastIndexUniqueList`: a list with a fast `index_of`.
///
/// Python's version uses a dict from item to index and marks it dirty on
/// structural edits. We keep the same trade-off: cheap lookups, lazy rebuild.
#[derive(Debug, Clone, Default)]
pub struct FastIndexUniqueList<T> {
    items: Vec<T>,
    indices: HashMap<T, usize>,
    indices_dirty: bool,
}

impl<T> FastIndexUniqueList<T>
where
    T: Eq + Hash + Clone,
{
    pub fn new(initial_items: impl IntoIterator<Item = T>) -> Self {
        let items: Vec<T> = initial_items.into_iter().collect();

        Self {
            items,
            indices: HashMap::new(),
            indices_dirty: true,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn contains(&mut self, item: &T) -> bool {
        self.ensure_indices();
        self.indices.contains_key(item)
    }

    pub fn index_of(&mut self, item: &T) -> Option<usize> {
        self.ensure_indices();
        self.indices.get(item).copied()
    }

    /// Python raises `DataMissing` here.
    pub fn index_or_error(&mut self, item: &T) -> Result<usize, HydrusException> {
        self.index_of(item)
            .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "item not in list"))
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    pub fn into_vec(self) -> Vec<T> {
        self.items
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.mark_dirty();
    }

    pub fn append(&mut self, item: T) {
        self.extend([item]);
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        self.ensure_indices();

        let start = self.items.len();

        for (offset, item) in items.into_iter().enumerate() {
            self.indices.insert(item.clone(), start + offset);
            self.items.push(item);
        }
    }

    pub fn set(&mut self, index: usize, value: T) -> Option<T> {
        let old = std::mem::replace(self.items.get_mut(index)?, value.clone());
        self.indices.remove(&old);
        self.indices.insert(value, index);
        Some(old)
    }

    pub fn insert_items<I: IntoIterator<Item = T>>(&mut self, items: I, insertion_index: usize) {
        for (offset, item) in items.into_iter().enumerate() {
            self.items
                .insert((insertion_index + offset).min(self.items.len()), item);
        }

        self.mark_dirty();
    }

    /// Python's `insert_items( items )` with no index: append then sort.
    pub fn insert_items_sorted<I: IntoIterator<Item = T>>(&mut self, items: I)
    where
        T: Ord,
    {
        self.extend(items);
        self.sort();
    }

    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.items.sort();
        self.mark_dirty();
    }

    /// Move the given items so they sit at `insertion_index` (the Python
    /// selection-page reorder helper). Missing items are skipped.
    pub fn move_items(&mut self, new_items: &[T], insertion_index: usize) {
        let mut items_to_move: Vec<T> = Vec::new();

        for item in new_items {
            if self.index_of(item).is_some() {
                items_to_move.push(item.clone());
            }
        }

        if items_to_move.is_empty() {
            return;
        }

        let index = insertion_index.min(self.items.len());

        self.remove_items(&items_to_move);
        self.insert_items(items_to_move, index);
    }

    pub fn remove_items(&mut self, items: &[T]) {
        let mut deletee_indices: Vec<usize> = items
            .iter()
            .filter_map(|item| self.index_of(item))
            .collect();

        deletee_indices.sort_unstable();
        deletee_indices.dedup();
        deletee_indices.reverse();

        for index in deletee_indices {
            self.items.remove(index);
        }

        self.mark_dirty();
    }

    pub fn remove(&mut self, item: &T) {
        self.remove_items(std::slice::from_ref(item));
    }

    pub fn pop(&mut self, index: Option<usize>) -> Option<T> {
        let index = index.unwrap_or_else(|| self.items.len().saturating_sub(1));

        if index >= self.items.len() {
            return None;
        }

        let item = self.items.remove(index);
        self.mark_dirty();
        Some(item)
    }

    pub fn reverse(&mut self) {
        self.items.reverse();
        self.mark_dirty();
    }

    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.items.sort_by(compare);
        self.mark_dirty();
    }

    pub fn random_sort(&mut self) {
        self.items.shuffle(&mut rand::thread_rng());
        self.mark_dirty();
    }

    fn mark_dirty(&mut self) {
        self.indices_dirty = true;
        self.indices.clear();
    }

    fn ensure_indices(&mut self) {
        if self.indices_dirty {
            self.indices = self
                .items
                .iter()
                .enumerate()
                .map(|(index, item)| (item.clone(), index))
                .collect();

            self.indices_dirty = false;
        }
    }
}

impl<T: Eq + Hash + Clone> FromIterator<T> for FastIndexUniqueList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(iter)
    }
}

/// `intelligent_mass_intersect`
pub fn intelligent_mass_intersect<T, I, S>(sets_to_reduce: I) -> HashSet<T>
where
    T: Eq + Hash + Clone,
    S: std::borrow::Borrow<HashSet<T>>,
    I: IntoIterator<Item = S>,
{
    let mut answer: Option<HashSet<T>> = None;

    for set_to_reduce in sets_to_reduce {
        let set_to_reduce = set_to_reduce.borrow();

        if set_to_reduce.is_empty() {
            return HashSet::new();
        }

        match &mut answer {
            None => answer = Some(set_to_reduce.clone()),
            Some(current) => {
                if current.is_empty() {
                    return HashSet::new();
                }

                current.retain(|item| set_to_reduce.contains(item));
            }
        }
    }

    answer.unwrap_or_default()
}

/// `iterate_list_randomly_and_fast`
pub fn iterate_list_randomly_and_fast<T: Clone>(xs: &[T]) -> Vec<T> {
    let mut ys = xs.to_vec();
    ys.shuffle(&mut rand::thread_rng());
    ys
}

/// `mass_extend`
pub fn mass_extend<T, I, J>(iterables: I) -> Vec<T>
where
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = T>,
{
    iterables
        .into_iter()
        .flat_map(IntoIterator::into_iter)
        .collect()
}

/// `mass_union`
pub fn mass_union<T, I, J>(iterables: I) -> HashSet<T>
where
    T: Eq + Hash,
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = T>,
{
    iterables.into_iter().flatten().collect()
}

/// `median_pop`: assumes a sorted, non-empty population.
pub fn median_pop<T>(population: &mut Vec<T>) -> T {
    let median_index = population.len() / 2;

    population.remove(median_index)
}

/// `partition_iterator_into_lists`: `( falsy, truthy )`.
pub fn partition_iterator_into_lists<T, I, F>(pred: F, stream: I) -> (Vec<T>, Vec<T>)
where
    I: IntoIterator<Item = T>,
    F: Fn(&T) -> bool,
{
    let mut no: Vec<T> = Vec::new();
    let mut yes: Vec<T> = Vec::new();

    for item in stream {
        if pred(&item) {
            yes.push(item);
        } else {
            no.push(item);
        }
    }

    (no, yes)
}

/// `pull_n_from_iterator`
pub fn pull_n_from_iterator<T, I>(iterator: I, n: usize) -> Vec<T>
where
    I: IntoIterator<Item = T>,
{
    iterator.into_iter().take(n).collect()
}

/// `randomise_list_by_chunks`
pub fn randomise_list_by_chunks<T: Clone>(xs: &[T], n: usize) -> Vec<T> {
    let mut blocks: Vec<&[T]> = xs.chunks(n.max(1)).collect();
    blocks.shuffle(&mut rand::thread_rng());

    blocks.into_iter().flatten().cloned().collect()
}

/// `random_pop`
pub fn random_pop<T>(population: &mut Vec<T>) -> Option<T> {
    if population.is_empty() {
        return None;
    }

    let random_index = rand::thread_rng().gen_range(0..population.len());

    Some(population.remove(random_index))
}

/// `sample_set_by_getting_first`: not truly random, but cheap for big sets.
pub fn sample_set_by_getting_first<T: Clone + Eq + Hash>(s: &HashSet<T>, n: usize) -> HashSet<T> {
    s.iter().take(n).cloned().collect()
}

/// `sets_intersect`
pub fn sets_intersect<T, I, J>(a: I, b: J) -> bool
where
    T: Eq + Hash,
    I: IntoIterator<Item = T>,
    J: IntoIterator<Item = T>,
{
    let a: HashSet<T> = a.into_iter().collect();

    b.into_iter().any(|item| a.contains(&item))
}

/// `smooth_out_mapping_iterator`: split `( tag, big list of hashes )` up.
pub fn smooth_out_mapping_iterator<T, U, I>(xs: I, n: usize) -> Vec<(T, Vec<U>)>
where
    I: IntoIterator<Item = (T, Vec<U>)>,
    T: Clone,
    U: Clone,
{
    let mut result: Vec<(T, Vec<U>)> = Vec::new();

    for (tag_item, hash_items) in xs {
        for chunk in hash_items.chunks(n.max(1)) {
            result.push((tag_item.clone(), chunk.to_vec()));
        }
    }

    result
}

/// `splay_list_for_db`
pub fn splay_list_for_db<T: std::fmt::Display>(xs: &[T]) -> String {
    let inner: Vec<String> = xs.iter().map(ToString::to_string).collect();

    format!("({})", inner.join(","))
}

/// `split_iterator_into_chunks`
pub fn split_iterator_into_chunks<T, I>(iterator: I, n: usize) -> Vec<Vec<T>>
where
    I: IntoIterator<Item = T>,
{
    let n = n.max(1);
    let mut chunks: Vec<Vec<T>> = Vec::new();
    let mut chunk: Vec<T> = Vec::with_capacity(n);

    for item in iterator {
        chunk.push(item);

        if chunk.len() == n {
            chunks.push(std::mem::replace(&mut chunk, Vec::with_capacity(n)));
        }
    }

    if !chunk.is_empty() {
        chunks.push(chunk);
    }

    chunks
}

/// `split_list_into_chunks`
pub fn split_list_into_chunks<T: Clone>(xs: &[T], n: usize) -> Vec<Vec<T>> {
    xs.chunks(n.max(1)).map(|chunk| chunk.to_vec()).collect()
}

/// `split_list_into_chunks_rich`: `( index, num_to_do, chunk )`.
pub fn split_list_into_chunks_rich<T: Clone>(xs: &[T], n: usize) -> Vec<(usize, usize, Vec<T>)> {
    let num_to_do = xs.len();

    xs.chunks(n.max(1))
        .enumerate()
        .map(|(chunk_index, chunk)| (chunk_index * n.max(1), num_to_do, chunk.to_vec()))
        .collect()
}

/// `split_iterator_into_autothrottled_chunks`.
///
/// Python yields chunks lazily and grows the chunk size as it measures how
/// fast the consumer is. Rust callers get the chunks up front; the sizing
/// algorithm is the same.
pub fn split_iterator_into_autothrottled_chunks<T, I>(
    iterator: I,
    starting_n: usize,
    precise_time_to_stop: Instant,
) -> Vec<Vec<T>>
where
    I: IntoIterator<Item = T>,
{
    let mut iterator = iterator.into_iter();

    let mut n = starting_n.max(1);
    let mut chunks: Vec<Vec<T>> = Vec::new();

    loop {
        let chunk: Vec<T> = iterator.by_ref().take(n).collect();

        if chunk.is_empty() {
            break;
        }

        let time_work_started = Instant::now();

        chunks.push(chunk);

        let actual_work_period = time_work_started.elapsed().as_secs_f64();
        let items_per_second = if actual_work_period > 0.0 {
            n as f64 / actual_work_period
        } else {
            n as f64
        };

        if Instant::now() >= precise_time_to_stop {
            n = 1;
        } else {
            let time_remaining = precise_time_to_stop
                .saturating_duration_since(Instant::now())
                .as_secs_f64();

            let expected_items_in_remaining_time =
                ((time_remaining * items_per_second) as usize).max(1);

            n = (n * 4).min(expected_items_in_remaining_time);
        }
    }

    chunks
}

/// `split_mapping_iterator_into_autothrottled_chunks`.
pub fn split_mapping_iterator_into_autothrottled_chunks<K, V, I>(
    iterator: I,
    starting_n: usize,
    precise_time_to_stop: Instant,
) -> Vec<Vec<(K, Vec<V>)>>
where
    I: IntoIterator<Item = (K, Vec<V>)>,
{
    let mut n = starting_n.max(1);
    let mut chunk_weight = 0usize;
    let mut chunk: Vec<(K, Vec<V>)> = Vec::new();
    let mut chunks: Vec<Vec<(K, Vec<V>)>> = Vec::new();

    for (tag_item, hash_items) in iterator {
        chunk_weight += hash_items.len();
        chunk.push((tag_item, hash_items));

        if chunk_weight >= n {
            let time_work_started = Instant::now();

            chunks.push(std::mem::take(&mut chunk));

            let actual_work_period = time_work_started.elapsed().as_secs_f64();
            chunk_weight = 0;

            let items_per_second = if actual_work_period > 0.0 {
                n as f64 / actual_work_period
            } else {
                n as f64
            };

            if Instant::now() >= precise_time_to_stop {
                n = 1;
            } else {
                let time_remaining = precise_time_to_stop
                    .saturating_duration_since(Instant::now())
                    .as_secs_f64();

                let expected_items_in_remaining_time =
                    ((time_remaining * items_per_second) as usize).max(1);

                n = (n * 4).min(expected_items_in_remaining_time);
            }
        }
    }

    if !chunk.is_empty() {
        chunks.push(chunk);
    }

    chunks
}

/// Convenience for the common "run for at most this long" case.
pub fn deadline_in(duration: Duration) -> Instant {
    Instant::now() + duration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_preserves_first_order() {
        assert_eq!(dedupe_list(vec![3, 1, 3, 2, 1]), vec![3, 1, 2]);
    }

    #[test]
    fn fast_index_list_tracks_edits() {
        let mut list = FastIndexUniqueList::new(["a", "b", "c"]);

        assert_eq!(list.index_of(&"b"), Some(1));

        list.insert_items(["x"], 0);
        assert_eq!(list.index_of(&"x"), Some(0));
        assert_eq!(list.index_of(&"c"), Some(3));

        list.remove_items(&["b"]);
        assert_eq!(list.as_slice(), &["x", "a", "c"]);
        assert_eq!(list.index_of(&"c"), Some(2));
    }

    #[test]
    fn intersect_works() {
        let a: HashSet<i32> = [1, 2, 3].into_iter().collect();
        let b: HashSet<i32> = [2, 3, 4].into_iter().collect();
        let c: HashSet<i32> = [3, 2, 9].into_iter().collect();

        assert_eq!(
            intelligent_mass_intersect([&a, &b, &c]),
            [2, 3].into_iter().collect()
        );

        let empty: HashSet<i32> = HashSet::new();
        assert!(intelligent_mass_intersect([&a, &empty]).is_empty());
    }

    #[test]
    fn chunking() {
        assert_eq!(
            split_list_into_chunks(&[1, 2, 3, 4, 5], 2),
            vec![vec![1, 2], vec![3, 4], vec![5]]
        );

        assert_eq!(
            split_list_into_chunks_rich(&[1, 2, 3], 2),
            vec![(0, 3, vec![1, 2]), (2, 3, vec![3])]
        );

        assert_eq!(
            split_iterator_into_chunks(vec![1, 2, 3], 2),
            vec![vec![1, 2], vec![3]]
        );
    }

    #[test]
    fn splay_for_db() {
        assert_eq!(splay_list_for_db(&[1, 2, 3]), "(1,2,3)");
    }

    #[test]
    fn partition_splits() {
        let (evens, odds) = partition_iterator_into_lists(|x: &i32| x % 2 == 0, vec![1, 2, 3, 4]);

        assert_eq!(evens, vec![1, 3]);
        assert_eq!(odds, vec![2, 4]);
    }
}
