use dashmap::DashMap;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub struct LruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    map: DashMap<K, CacheEntry<V>>,
    order: Mutex<VecDeque<K>>,
    capacity: usize,
    hits: AtomicU64,
    misses: AtomicU64,
    size: AtomicUsize,
}

struct CacheEntry<V> {
    value: V,
    size: usize,
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            map: DashMap::new(),
            order: Mutex::new(VecDeque::new()),
            capacity,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            size: AtomicUsize::new(0),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        if let Some(entry) = self.map.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            self.touch(key.clone());
            Some(entry.value.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn insert(&self, key: K, value: V, size: usize) {
        let entry = CacheEntry { value, size };

        if let Some(old) = self.map.insert(key.clone(), entry) {
            self.size.fetch_sub(old.size, Ordering::Relaxed);
        } else {
            let mut order = self.order.lock();
            order.push_back(key.clone());
        }

        self.size.fetch_add(size, Ordering::Relaxed);

        while self.size.load(Ordering::Relaxed) > self.capacity {
            self.evict_one();
        }
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        if let Some((_, entry)) = self.map.remove(key) {
            self.size.fetch_sub(entry.size, Ordering::Relaxed);

            let mut order = self.order.lock();
            order.retain(|k| k != key);

            Some(entry.value)
        } else {
            None
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn clear(&self) {
        self.map.clear();
        self.order.lock().clear();
        self.size.store(0, Ordering::Relaxed);
    }

    fn touch(&self, key: K) {
        let mut order = self.order.lock();
        order.retain(|k| k != &key);
        order.push_back(key);
    }

    fn evict_one(&self) {
        let key = {
            let mut order = self.order.lock();
            order.pop_front()
        };

        if let Some(key) = key {
            if let Some((_, entry)) = self.map.remove(&key) {
                self.size.fetch_sub(entry.size, Ordering::Relaxed);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        CacheStats {
            hits,
            misses,
            hit_rate,
            size: self.size.load(Ordering::Relaxed),
            capacity: self.capacity,
            entries: self.map.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: usize,
    pub capacity: usize,
    pub entries: usize,
}

pub struct DocumentCache {
    cache: LruCache<Vec<u8>, Vec<u8>>,
}

impl DocumentCache {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            cache: LruCache::new(capacity_bytes),
        }
    }

    pub fn get(&self, collection: &str, key: &[u8]) -> Option<Vec<u8>> {
        let cache_key = Self::make_key(collection, key);
        self.cache.get(&cache_key)
    }

    pub fn insert(&self, collection: &str, key: &[u8], value: Vec<u8>) {
        let cache_key = Self::make_key(collection, key);
        let size = cache_key.len() + value.len();
        self.cache.insert(cache_key, value, size);
    }

    pub fn remove(&self, collection: &str, key: &[u8]) {
        let cache_key = Self::make_key(collection, key);
        self.cache.remove(&cache_key);
    }

    pub fn invalidate_collection(&self, collection: &str) {
        let prefix = format!("{}:", collection);
        let keys_to_remove: Vec<Vec<u8>> = self
            .cache
            .map
            .iter()
            .filter(|entry| {
                entry
                    .key()
                    .starts_with(prefix.as_bytes())
            })
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            self.cache.remove(&key);
        }
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn stats(&self) -> CacheStats {
        self.cache.stats()
    }

    fn make_key(collection: &str, key: &[u8]) -> Vec<u8> {
        let mut cache_key = Vec::with_capacity(collection.len() + 1 + key.len());
        cache_key.extend_from_slice(collection.as_bytes());
        cache_key.push(b':');
        cache_key.extend_from_slice(key);
        cache_key
    }
}

pub struct QueryCache {
    cache: LruCache<u64, Vec<u8>>,
}

impl QueryCache {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            cache: LruCache::new(capacity_bytes),
        }
    }

    pub fn get(&self, query_hash: u64) -> Option<Vec<u8>> {
        self.cache.get(&query_hash)
    }

    pub fn insert(&self, query_hash: u64, result: Vec<u8>) {
        let size = 8 + result.len();
        self.cache.insert(query_hash, result, size);
    }

    pub fn invalidate(&self, query_hash: u64) {
        self.cache.remove(&query_hash);
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn stats(&self) -> CacheStats {
        self.cache.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache() {
        let cache: LruCache<String, String> = LruCache::new(100);

        cache.insert("key1".to_string(), "value1".to_string(), 10);
        cache.insert("key2".to_string(), "value2".to_string(), 10);

        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(cache.get(&"key2".to_string()), Some("value2".to_string()));
        assert_eq!(cache.get(&"key3".to_string()), None);
    }

    #[test]
    fn test_lru_eviction() {
        let cache: LruCache<String, String> = LruCache::new(25);

        cache.insert("key1".to_string(), "value1".to_string(), 10);
        cache.insert("key2".to_string(), "value2".to_string(), 10);
        cache.insert("key3".to_string(), "value3".to_string(), 10);

        assert!(cache.get(&"key1".to_string()).is_none());
        assert!(cache.get(&"key2".to_string()).is_some());
        assert!(cache.get(&"key3".to_string()).is_some());
    }

    #[test]
    fn test_document_cache() {
        let cache = DocumentCache::new(1024);

        cache.insert("test", b"doc1", vec![1, 2, 3]);
        assert_eq!(cache.get("test", b"doc1"), Some(vec![1, 2, 3]));

        cache.remove("test", b"doc1");
        assert_eq!(cache.get("test", b"doc1"), None);
    }
}
