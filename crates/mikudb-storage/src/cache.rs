//! 缓存模块
//!
//! 本模块实现多级缓存系统:
//! - **LRU 缓存**: 通用 LRU 淘汰策略缓存
//! - **文档缓存**: 缓存文档数据,提升热点数据访问性能
//! - **查询缓存**: 缓存查询结果,减少重复查询开销
//!
//! 特性:
//! - 线程安全:使用 DashMap 和 parking_lot::Mutex 保证并发访问
//! - 容量控制:基于字节大小限制,自动淘汰最久未使用的条目
//! - 统计信息:记录命中率、缓存大小等指标
//! - 集合失效:支持按集合批量失效缓存

use dashmap::DashMap;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// LRU 缓存
///
/// 实现 Least Recently Used (最近最少使用) 淘汰策略的线程安全缓存。
/// 使用 DashMap 提供高并发读写能力,使用 VecDeque 维护访问顺序。
pub struct LruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// 缓存数据:键 -> 缓存条目
    map: DashMap<K, CacheEntry<V>>,
    /// 访问顺序:队首为最久未使用,队尾为最近使用
    order: Mutex<VecDeque<K>>,
    /// 容量限制(字节)
    capacity: usize,
    /// 缓存命中次数
    hits: AtomicU64,
    /// 缓存未命中次数
    misses: AtomicU64,
    /// 当前缓存大小(字节)
    size: AtomicUsize,
}

/// 缓存条目
///
/// 封装缓存值和其占用的字节大小。
struct CacheEntry<V> {
    /// 缓存的值
    value: V,
    /// 条目大小(字节)
    size: usize,
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// # Brief
    /// 创建 LRU 缓存
    ///
    /// # Arguments
    /// * `capacity` - 缓存容量限制(字节)
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

    /// # Brief
    /// 获取缓存值
    ///
    /// 命中时更新访问顺序,将键移到队尾(标记为最近使用)。
    ///
    /// # Arguments
    /// * `key` - 缓存键
    ///
    /// # Returns
    /// 缓存值(如果存在)
    pub fn get(&self, key: &K) -> Option<V> {
        if let Some(entry) = self.map.get(key) {
            // 命中,增加命中计数
            self.hits.fetch_add(1, Ordering::Relaxed);
            // 更新访问顺序
            self.touch(key.clone());
            Some(entry.value.clone())
        } else {
            // 未命中,增加未命中计数
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// # Brief
    /// 插入缓存条目
    ///
    /// 如果键已存在,更新值并调整大小。
    /// 如果插入后超过容量限制,自动淘汰最久未使用的条目。
    ///
    /// # Arguments
    /// * `key` - 缓存键
    /// * `value` - 缓存值
    /// * `size` - 条目大小(字节)
    pub fn insert(&self, key: K, value: V, size: usize) {
        let entry = CacheEntry { value, size };

        if let Some(old) = self.map.insert(key.clone(), entry) {
            // 键已存在,减去旧值的大小
            self.size.fetch_sub(old.size, Ordering::Relaxed);
        } else {
            // 新键,添加到访问顺序队列
            let mut order = self.order.lock();
            order.push_back(key.clone());
        }

        // 增加新值的大小
        self.size.fetch_add(size, Ordering::Relaxed);

        // 如果超过容量限制,淘汰最久未使用的条目
        while self.size.load(Ordering::Relaxed) > self.capacity {
            self.evict_one();
        }
    }

    /// # Brief
    /// 移除缓存条目
    ///
    /// # Arguments
    /// * `key` - 缓存键
    ///
    /// # Returns
    /// 移除的值(如果存在)
    pub fn remove(&self, key: &K) -> Option<V> {
        if let Some((_, entry)) = self.map.remove(key) {
            // 减小缓存大小
            self.size.fetch_sub(entry.size, Ordering::Relaxed);

            // 从访问顺序队列中移除
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

    /// # Brief
    /// 更新访问顺序
    ///
    /// 将键移到队尾,标记为最近使用。
    ///
    /// # Arguments
    /// * `key` - 缓存键
    fn touch(&self, key: K) {
        let mut order = self.order.lock();
        // 从队列中移除键
        order.retain(|k| k != &key);
        // 添加到队尾(最近使用)
        order.push_back(key);
    }

    /// # Brief
    /// 淘汰一个条目
    ///
    /// 移除队首(最久未使用)的条目。
    fn evict_one(&self) {
        let key = {
            let mut order = self.order.lock();
            // 弹出队首(最久未使用)
            order.pop_front()
        };

        if let Some(key) = key {
            if let Some((_, entry)) = self.map.remove(&key) {
                // 减小缓存大小
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

    /// # Brief
    /// 获取缓存统计信息
    ///
    /// # Returns
    /// 缓存统计信息(命中率、大小、条目数等)
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        // 计算命中率
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

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 命中率 (0.0 ~ 1.0)
    pub hit_rate: f64,
    /// 当前缓存大小(字节)
    pub size: usize,
    /// 缓存容量限制(字节)
    pub capacity: usize,
    /// 缓存条目数
    pub entries: usize,
}

/// 文档缓存
///
/// 缓存文档数据,使用 "集合名:键" 作为缓存键。
pub struct DocumentCache {
    /// 内部 LRU 缓存
    cache: LruCache<Vec<u8>, Vec<u8>>,
}

impl DocumentCache {
    /// # Brief
    /// 创建文档缓存
    ///
    /// # Arguments
    /// * `capacity_bytes` - 缓存容量限制(字节)
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            cache: LruCache::new(capacity_bytes),
        }
    }

    /// # Brief
    /// 获取文档
    ///
    /// # Arguments
    /// * `collection` - 集合名
    /// * `key` - 文档键
    ///
    /// # Returns
    /// 文档数据(如果存在)
    pub fn get(&self, collection: &str, key: &[u8]) -> Option<Vec<u8>> {
        let cache_key = Self::make_key(collection, key);
        self.cache.get(&cache_key)
    }

    /// # Brief
    /// 插入文档到缓存
    ///
    /// # Arguments
    /// * `collection` - 集合名
    /// * `key` - 文档键
    /// * `value` - 文档数据
    pub fn insert(&self, collection: &str, key: &[u8], value: Vec<u8>) {
        let cache_key = Self::make_key(collection, key);
        // 缓存大小 = 键大小 + 值大小
        let size = cache_key.len() + value.len();
        self.cache.insert(cache_key, value, size);
    }

    pub fn remove(&self, collection: &str, key: &[u8]) {
        let cache_key = Self::make_key(collection, key);
        self.cache.remove(&cache_key);
    }

    /// # Brief
    /// 失效集合的所有缓存
    ///
    /// 移除指定集合的所有文档缓存。
    ///
    /// # Arguments
    /// * `collection` - 集合名
    pub fn invalidate_collection(&self, collection: &str) {
        let prefix = format!("{}:", collection);
        // 收集所有匹配前缀的键
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

        // 批量删除
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

    /// # Brief
    /// 构造缓存键
    ///
    /// 格式: "集合名:文档键"
    ///
    /// # Arguments
    /// * `collection` - 集合名
    /// * `key` - 文档键
    ///
    /// # Returns
    /// 缓存键(字节数组)
    fn make_key(collection: &str, key: &[u8]) -> Vec<u8> {
        let mut cache_key = Vec::with_capacity(collection.len() + 1 + key.len());
        cache_key.extend_from_slice(collection.as_bytes());
        cache_key.push(b':');  // 使用冒号分隔集合名和键
        cache_key.extend_from_slice(key);
        cache_key
    }
}

/// 查询缓存
///
/// 缓存查询结果,使用查询哈希值作为键。
pub struct QueryCache {
    /// 内部 LRU 缓存
    cache: LruCache<u64, Vec<u8>>,
}

impl QueryCache {
    /// # Brief
    /// 创建查询缓存
    ///
    /// # Arguments
    /// * `capacity_bytes` - 缓存容量限制(字节)
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            cache: LruCache::new(capacity_bytes),
        }
    }

    /// # Brief
    /// 获取查询结果
    ///
    /// # Arguments
    /// * `query_hash` - 查询哈希值
    ///
    /// # Returns
    /// 查询结果(如果存在)
    pub fn get(&self, query_hash: u64) -> Option<Vec<u8>> {
        self.cache.get(&query_hash)
    }

    /// # Brief
    /// 插入查询结果到缓存
    ///
    /// # Arguments
    /// * `query_hash` - 查询哈希值
    /// * `result` - 查询结果
    pub fn insert(&self, query_hash: u64, result: Vec<u8>) {
        // 缓存大小 = 哈希值大小(8字节) + 结果大小
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
