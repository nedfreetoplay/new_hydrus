//! Rust port of `hydrus/core/HydrusDBPopulateCache.py`.
//!
//! Three cache shapes: an object cache that holds weak references plus a
//! bounded strong LRU, and two primitive caches (one with a TTL, one without).
//!
//! Python's `KeyError` becomes `None`; callers that know a key must be present
//! can treat `None` as the "this should never happen" case it is in Python.

use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use indexmap::IndexMap;

/// Move an existing key to the back of the insertion order.
fn move_to_end<K: std::hash::Hash + Eq, V>(map: &mut IndexMap<K, V>, key: &K) {
    if let Some((key, value)) = map.shift_remove_entry(key) {
        map.insert(key, value);
    }
}

/// Python's `IdToObjectCacheTTL`.
///
/// Ids map to weakly held objects; the same objects are also held strongly in a
/// bounded, TTL-ed LRU so a recently used object survives a gap in its real
/// owner's references.
pub struct IdToObjectCacheTTL<T> {
    name: String,
    ttl: Duration,
    max_strong_size: usize,
    ids_to_values_weak: HashMap<i64, Weak<T>>,
    ids_to_values_strong: IndexMap<i64, (Instant, Arc<T>)>,
}

impl<T> IdToObjectCacheTTL<T> {
    pub fn new(name: impl Into<String>, ttl_seconds: u64, max_strong_size: usize) -> Self {
        Self {
            name: name.into(),
            ttl: Duration::from_secs(ttl_seconds),
            max_strong_size,
            ids_to_values_weak: HashMap::new(),
            ids_to_values_strong: IndexMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// `key in self._ids_to_values_weak`. Dead weak references count as absent.
    pub fn contains(&self, key: i64) -> bool {
        self.ids_to_values_weak
            .get(&key)
            .and_then(Weak::upgrade)
            .is_some()
    }

    /// `self[ key ]`, returning `None` where Python raises `KeyError`.
    pub fn get(&mut self, key: i64) -> Option<Arc<T>> {
        let object = self.ids_to_values_weak.get(&key)?.upgrade()?;

        self.touch(key, object.clone());

        Some(object)
    }

    fn touch(&mut self, key: i64, object: Arc<T>) {
        let timeout = Instant::now() + self.ttl;

        if let Some(entry) = self.ids_to_values_strong.get_mut(&key) {
            *entry = (timeout, object);
        } else {
            self.ids_to_values_strong.insert(key, (timeout, object));
        }

        move_to_end(&mut self.ids_to_values_strong, &key);
    }

    fn touch_many(&mut self, keys_to_objects: impl IntoIterator<Item = (i64, Arc<T>)>) {
        let timeout = Instant::now() + self.ttl;

        for (key, object) in keys_to_objects {
            if let Some(entry) = self.ids_to_values_strong.get_mut(&key) {
                *entry = (timeout, object);
            } else {
                self.ids_to_values_strong.insert(key, (timeout, object));
            }

            move_to_end(&mut self.ids_to_values_strong, &key);
        }
    }

    /// `self[ key ] = obj`
    pub fn set(&mut self, key: i64, object: Arc<T>) {
        self.ids_to_values_weak.insert(key, Arc::downgrade(&object));

        self.touch(key, object);
    }

    /// `clear`
    pub fn clear(&mut self) {
        self.ids_to_values_weak.clear();
        self.ids_to_values_strong.clear();
    }

    /// `maintain_touch_record`
    pub fn maintain_touch_record(&mut self) {
        let max_deletes_this_call = std::cmp::max(1, self.ids_to_values_weak.len() / 16);

        let now = Instant::now();

        let mut num_to_remove_for_timeout = 0usize;

        for (current_timeout, _) in self.ids_to_values_strong.values() {
            if *current_timeout > now {
                break;
            }

            num_to_remove_for_timeout += 1;

            if num_to_remove_for_timeout >= max_deletes_this_call {
                break;
            }
        }

        for _ in 0..num_to_remove_for_timeout {
            self.ids_to_values_strong.shift_remove_index(0);
        }

        let oversize_quantity =
            self.ids_to_values_strong.len().saturating_sub(self.max_strong_size);

        let num_to_remove_for_oversize =
            std::cmp::min(oversize_quantity, max_deletes_this_call - num_to_remove_for_timeout);

        for _ in 0..num_to_remove_for_oversize {
            self.ids_to_values_strong.shift_remove_index(0);
        }
    }

    /// `touch_many_if_have( keys )`
    pub fn touch_many_if_have(&mut self, keys: impl IntoIterator<Item = i64>) {
        let mut to_touch = Vec::new();

        for key in keys {
            if let Some(object) = self.ids_to_values_weak.get(&key).and_then(Weak::upgrade) {
                to_touch.push((key, object));
            }
        }

        self.touch_many(to_touch);
    }

    /// `update( keys_to_objs )`
    pub fn update(&mut self, keys_to_objects: impl IntoIterator<Item = (i64, Arc<T>)>) {
        let objects = keys_to_objects.into_iter().collect::<Vec<_>>();

        for (key, object) in &objects {
            self.ids_to_values_weak
                .insert(*key, Arc::downgrade(object));
        }

        self.touch_many(objects);
    }
}

/// Python's `IdToPrimitiveCacheTTL`.
pub struct IdToPrimitiveCacheTTL<T> {
    name: String,
    ttl: Duration,
    min_size: usize,
    max_size: usize,
    ids_to_values: IndexMap<i64, (Instant, T)>,
}

impl<T> IdToPrimitiveCacheTTL<T> {
    pub fn new(name: impl Into<String>, ttl_seconds: u64, min_size: usize, max_size: usize) -> Self {
        Self {
            name: name.into(),
            ttl: Duration::from_secs(ttl_seconds),
            min_size,
            max_size,
            ids_to_values: IndexMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn contains(&self, key: i64) -> bool {
        self.ids_to_values.contains_key(&key)
    }

    pub fn len(&self) -> usize {
        self.ids_to_values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids_to_values.is_empty()
    }

    /// `self[ key ]`, returning `None` where Python raises `KeyError`.
    pub fn get(&mut self, key: i64) -> Option<&T> {
        let value = self.ids_to_values.shift_remove(&key)?;

        let timeout = Instant::now() + self.ttl;

        self.ids_to_values.insert(key, (timeout, value.1));

        self.ids_to_values.get(&key).map(|(_, value)| value)
    }

    fn touch(&mut self, key: i64, value: T) {
        let timeout = Instant::now() + self.ttl;

        if let Some(entry) = self.ids_to_values.get_mut(&key) {
            *entry = (timeout, value);
        } else {
            self.ids_to_values.insert(key, (timeout, value));
        }

        move_to_end(&mut self.ids_to_values, &key);
    }

    fn touch_many(&mut self, keys_to_values: impl IntoIterator<Item = (i64, T)>) {
        let timeout = Instant::now() + self.ttl;

        for (key, value) in keys_to_values {
            if let Some(entry) = self.ids_to_values.get_mut(&key) {
                *entry = (timeout, value);
            } else {
                self.ids_to_values.insert(key, (timeout, value));
            }

            move_to_end(&mut self.ids_to_values, &key);
        }
    }

    /// `self[ key ] = obj`
    pub fn set(&mut self, key: i64, value: T) {
        self.touch(key, value);
    }

    pub fn clear(&mut self) {
        self.ids_to_values.clear();
    }

    /// `maintain_touch_record`
    pub fn maintain_touch_record(&mut self) {
        if self.ids_to_values.len() <= self.min_size {
            return;
        }

        let max_deletes_this_call = std::cmp::max(1, self.ids_to_values.len() / 16);

        let now = Instant::now();

        let mut num_to_remove_for_timeout = 0usize;

        for (current_timeout, _) in self.ids_to_values.values() {
            if *current_timeout > now {
                break;
            }

            num_to_remove_for_timeout += 1;

            if num_to_remove_for_timeout >= max_deletes_this_call {
                break;
            }
        }

        for _ in 0..num_to_remove_for_timeout {
            self.ids_to_values.shift_remove_index(0);
        }

        let oversize_quantity = self.ids_to_values.len().saturating_sub(self.max_size);

        let num_to_remove_for_oversize =
            std::cmp::min(oversize_quantity, max_deletes_this_call - num_to_remove_for_timeout);

        for _ in 0..num_to_remove_for_oversize {
            self.ids_to_values.shift_remove_index(0);
        }
    }

    /// `update( keys_to_objs )`
    pub fn update(&mut self, keys_to_values: impl IntoIterator<Item = (i64, T)>) {
        self.touch_many(keys_to_values);
    }
}

/// Python's `IdToPrimitiveCache`.
pub struct IdToPrimitiveCache<T> {
    name: String,
    max_size: usize,
    ids_to_values: IndexMap<i64, T>,
}

impl<T> IdToPrimitiveCache<T> {
    pub fn new(name: impl Into<String>, max_size: usize) -> Self {
        Self {
            name: name.into(),
            max_size,
            ids_to_values: IndexMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn contains(&self, key: i64) -> bool {
        self.ids_to_values.contains_key(&key)
    }

    pub fn len(&self) -> usize {
        self.ids_to_values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids_to_values.is_empty()
    }

    /// `del self[ key ]`
    pub fn remove(&mut self, key: i64) -> Option<T> {
        self.ids_to_values.shift_remove(&key)
    }

    /// `self[ key ]`, returning `None` where Python raises `KeyError`.
    pub fn get(&mut self, key: i64) -> Option<&T> {
        if !self.ids_to_values.contains_key(&key) {
            return None;
        }

        move_to_end(&mut self.ids_to_values, &key);

        self.ids_to_values.get(&key)
    }

    /// `self[ key ] = obj`. Python ignores the new value for an existing key
    /// and only moves it to the end, which the port preserves.
    pub fn set(&mut self, key: i64, value: T) {
        if self.ids_to_values.contains_key(&key) {
            move_to_end(&mut self.ids_to_values, &key);
        } else {
            self.ids_to_values.insert(key, value);
        }
    }

    pub fn clear(&mut self) {
        self.ids_to_values.clear();
    }

    /// `maintain_touch_record`
    pub fn maintain_touch_record(&mut self) {
        let current_size = self.ids_to_values.len();

        if current_size > self.max_size {
            let max_deletes_this_call = std::cmp::max(1, current_size / 16);

            let num_to_remove = std::cmp::min(
                current_size - self.max_size,
                max_deletes_this_call,
            );

            for _ in 0..num_to_remove {
                self.ids_to_values.shift_remove_index(0);
            }
        }
    }

    /// `update( keys_to_objs )`
    pub fn update(&mut self, keys_to_values: impl IntoIterator<Item = (i64, T)>) {
        for (key, value) in keys_to_values {
            if self.ids_to_values.contains_key(&key) {
                move_to_end(&mut self.ids_to_values, &key);
            } else {
                self.ids_to_values.insert(key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_cache_holds_weak_and_strong_references() {
        let mut cache = IdToObjectCacheTTL::new("objects", 300, 4);

        let first = Arc::new(String::from("first"));

        cache.set(1, first.clone());

        assert!(cache.contains(1));
        assert_eq!(cache.get(1).as_deref().map(String::as_str), Some("first"));

        drop(first);

        // The strong LRU keeps it alive.
        assert!(cache.contains(1));

        cache.clear();

        assert!(!cache.contains(1));
        assert!(cache.get(1).is_none());
    }

    #[test]
    fn object_cache_timeout_and_oversize_culling() {
        let mut cache = IdToObjectCacheTTL::new("objects", 0, 2);

        let values = (0..6)
            .map(|key| (key, Arc::new(key as i64)))
            .collect::<Vec<_>>();

        cache.update(values);

        // With a zero TTL every entry is already expired; one call may only
        // delete max(1, weak.len()/16) strong entries.
        cache.maintain_touch_record();

        assert_eq!(cache.ids_to_values_strong.len(), 5);

        for _ in 0..10 {
            cache.maintain_touch_record();
        }

        assert!(cache.ids_to_values_strong.is_empty());
    }

    #[test]
    fn object_cache_touches_existing_keys() {
        let mut cache = IdToObjectCacheTTL::new("objects", 300, 4);

        let object = Arc::new(7i64);

        cache.set(1, object.clone());
        cache.set(2, Arc::new(8i64));

        cache.touch_many_if_have([1, 3]);

        assert!(cache.get(2).is_some());
        assert!(cache.get(1).is_some());
    }

    #[test]
    fn primitive_ttl_cache_orders_and_expires() {
        let mut cache = IdToPrimitiveCacheTTL::new("tags", 300, 2, 8);

        cache.set(1, "one".to_string());
        cache.set(2, "two".to_string());
        cache.set(3, "three".to_string());

        assert_eq!(cache.get(1).map(String::as_str), Some("one"));

        // Key 1 was moved to the end, so key 2 is now the oldest.
        let keys = cache.ids_to_values.keys().copied().collect::<Vec<_>>();

        assert_eq!(keys, vec![2, 3, 1]);

        cache.update([(4, "four".to_string()), (5, "five".to_string())]);

        assert_eq!(cache.len(), 5);
        assert!(cache.contains(5));

        cache.clear();

        assert!(cache.is_empty());
    }

    #[test]
    fn primitive_ttl_cache_obeys_min_and_max_sizes() {
        let mut cache = IdToPrimitiveCacheTTL::new("tags", 0, 100, 100);

        for key in 0..50 {
            cache.set(key, key);
        }

        cache.maintain_touch_record();

        assert_eq!(cache.len(), 50, "below min_size, nothing is culled");

        let mut cache = IdToPrimitiveCacheTTL::new("tags", 0, 0, 4);

        for key in 0..40 {
            cache.set(key, key);
        }

        cache.maintain_touch_record();

        assert_eq!(cache.len(), 38);
    }

    #[test]
    fn primitive_cache_moves_and_culls() {
        let mut cache = IdToPrimitiveCache::new("hashes", 8);

        cache.set(1, "one".to_string());
        cache.set(2, "two".to_string());
        cache.set(3, "three".to_string());

        // Existing keys keep their original value.
        cache.set(2, "ignored".to_string());

        assert_eq!(cache.get(2).map(String::as_str), Some("two"));

        let keys = cache.ids_to_values.keys().copied().collect::<Vec<_>>();

        assert_eq!(keys, vec![1, 3, 2]);

        assert_eq!(cache.remove(1).as_deref(), Some("one"));
        assert!(!cache.contains(1));

        let mut cache = IdToPrimitiveCache::new("hashes", 4);

        for key in 0..40 {
            cache.set(key, key);
        }

        cache.maintain_touch_record();

        assert_eq!(cache.len(), 38);
        assert!(!cache.contains(0));
        assert!(!cache.contains(1));
    }
}
