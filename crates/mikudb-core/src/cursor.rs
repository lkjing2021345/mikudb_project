//! 游标模块
//!
//! 提供查询结果的迭代器接口，支持批量获取、流式处理和游标管理。

use crate::boml::Document;
use crate::common::{MikuError, MikuResult};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

static CURSOR_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct CursorOptions {
    pub batch_size: u32,
    pub timeout: Option<Duration>,
    pub no_cursor_timeout: bool,
    pub allow_partial_results: bool,
    pub max_await_time: Option<Duration>,
}

impl Default for CursorOptions {
    fn default() -> Self {
        Self {
            batch_size: 101,
            timeout: Some(Duration::from_secs(600)),
            no_cursor_timeout: false,
            allow_partial_results: false,
            max_await_time: None,
        }
    }
}

pub struct Cursor<T = Document> {
    id: u64,
    collection: String,
    buffer: Mutex<VecDeque<T>>,
    exhausted: AtomicBool,
    options: CursorOptions,
    created_at: Instant,
    last_accessed: Mutex<Instant>,
    total_returned: AtomicU64,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Cursor<T> {
    pub fn new(collection: impl Into<String>, options: CursorOptions) -> Self {
        let id = CURSOR_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let now = Instant::now();

        Self {
            id,
            collection: collection.into(),
            buffer: Mutex::new(VecDeque::new()),
            exhausted: AtomicBool::new(false),
            options,
            created_at: now,
            last_accessed: Mutex::new(now),
            total_returned: AtomicU64::new(0),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn from_vec(collection: impl Into<String>, items: Vec<T>) -> Self {
        let cursor = Self::new(collection, CursorOptions::default());
        cursor.buffer.lock().extend(items);
        cursor.exhausted.store(true, Ordering::SeqCst);
        cursor
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn collection(&self) -> &str {
        &self.collection
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted.load(Ordering::SeqCst) && self.buffer.lock().is_empty()
    }

    pub fn is_timed_out(&self) -> bool {
        if self.options.no_cursor_timeout {
            return false;
        }
        if let Some(timeout) = self.options.timeout {
            return self.last_accessed.lock().elapsed() > timeout;
        }
        false
    }

    pub fn total_returned(&self) -> u64 {
        self.total_returned.load(Ordering::SeqCst)
    }

    fn touch(&self) {
        *self.last_accessed.lock() = Instant::now();
    }

    pub fn buffered_count(&self) -> usize {
        self.buffer.lock().len()
    }

    pub(crate) fn add_batch(&self, batch: Vec<T>, is_last: bool) {
        self.touch();
        let mut buffer = self.buffer.lock();
        buffer.extend(batch);
        if is_last {
            self.exhausted.store(true, Ordering::SeqCst);
        }
    }

    pub(crate) fn mark_exhausted(&self) {
        self.exhausted.store(true, Ordering::SeqCst);
    }
}

impl<T: Clone> Cursor<T> {
    pub fn next(&self) -> Option<T> {
        self.touch();
        let item = self.buffer.lock().pop_front();
        if item.is_some() {
            self.total_returned.fetch_add(1, Ordering::SeqCst);
        }
        item
    }

    pub fn try_next(&self) -> MikuResult<Option<T>> {
        if self.is_timed_out() {
            return Err(MikuError::Query("Cursor timed out".to_string()));
        }
        Ok(self.next())
    }

    pub fn peek(&self) -> Option<T> {
        self.touch();
        self.buffer.lock().front().cloned()
    }

    pub fn take(&self, n: usize) -> Vec<T> {
        self.touch();
        let mut buffer = self.buffer.lock();
        let count = n.min(buffer.len());
        let items: Vec<T> = buffer.drain(..count).collect();
        self.total_returned.fetch_add(items.len() as u64, Ordering::SeqCst);
        items
    }

    pub fn collect_all(&self) -> Vec<T> {
        self.touch();
        let mut buffer = self.buffer.lock();
        let items: Vec<T> = buffer.drain(..).collect();
        self.total_returned.fetch_add(items.len() as u64, Ordering::SeqCst);
        items
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.collect_all()
    }
}

impl Cursor<Document> {
    pub fn map_documents<F, U>(&self, f: F) -> Vec<U>
    where
        F: Fn(&Document) -> U,
    {
        self.touch();
        let buffer = self.buffer.lock();
        buffer.iter().map(f).collect()
    }

    pub fn filter_documents<F>(&self, predicate: F) -> Vec<Document>
    where
        F: Fn(&Document) -> bool,
    {
        self.touch();
        let mut buffer = self.buffer.lock();
        let (matching, remaining): (Vec<_>, Vec<_>) =
            buffer.drain(..).partition(|d| predicate(d));
        buffer.extend(remaining);
        self.total_returned.fetch_add(matching.len() as u64, Ordering::SeqCst);
        matching
    }
}

impl<T: Clone> Iterator for &Cursor<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        Cursor::next(self)
    }
}

pub struct CursorIterator<T> {
    cursor: Arc<Cursor<T>>,
}

impl<T: Clone> CursorIterator<T> {
    pub fn new(cursor: Arc<Cursor<T>>) -> Self {
        Self { cursor }
    }
}

impl<T: Clone> Iterator for CursorIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next()
    }
}

pub struct CursorManager {
    cursors: Mutex<std::collections::HashMap<u64, Arc<Cursor<Document>>>>,
    cleanup_interval: Duration,
    last_cleanup: Mutex<Instant>,
}

impl CursorManager {
    pub fn new() -> Self {
        Self {
            cursors: Mutex::new(std::collections::HashMap::new()),
            cleanup_interval: Duration::from_secs(60),
            last_cleanup: Mutex::new(Instant::now()),
        }
    }

    pub fn register(&self, cursor: Cursor<Document>) -> Arc<Cursor<Document>> {
        self.maybe_cleanup();
        let cursor = Arc::new(cursor);
        self.cursors.lock().insert(cursor.id(), cursor.clone());
        cursor
    }

    pub fn get(&self, id: u64) -> Option<Arc<Cursor<Document>>> {
        self.cursors.lock().get(&id).cloned()
    }

    pub fn remove(&self, id: u64) -> Option<Arc<Cursor<Document>>> {
        self.cursors.lock().remove(&id)
    }

    pub fn kill(&self, ids: &[u64]) -> Vec<u64> {
        let mut cursors = self.cursors.lock();
        ids.iter()
            .filter(|id| cursors.remove(id).is_some())
            .copied()
            .collect()
    }

    pub fn kill_all(&self) -> usize {
        let mut cursors = self.cursors.lock();
        let count = cursors.len();
        cursors.clear();
        count
    }

    fn maybe_cleanup(&self) {
        let mut last = self.last_cleanup.lock();
        if last.elapsed() > self.cleanup_interval {
            *last = Instant::now();
            drop(last);
            self.cleanup_expired();
        }
    }

    pub fn cleanup_expired(&self) {
        let mut cursors = self.cursors.lock();
        let expired: Vec<u64> = cursors
            .iter()
            .filter(|(_, c)| c.is_timed_out() || c.is_exhausted())
            .map(|(id, _)| *id)
            .collect();

        for id in expired {
            cursors.remove(&id);
        }
    }

    pub fn active_count(&self) -> usize {
        self.cursors.lock().len()
    }

    pub fn list_cursors(&self) -> Vec<CursorInfo> {
        self.cursors
            .lock()
            .values()
            .map(|c| CursorInfo {
                id: c.id(),
                collection: c.collection().to_string(),
                buffered: c.buffered_count(),
                total_returned: c.total_returned(),
                exhausted: c.is_exhausted(),
                created_at: c.created_at,
            })
            .collect()
    }
}

impl Default for CursorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CursorInfo {
    pub id: u64,
    pub collection: String,
    pub buffered: usize,
    pub total_returned: u64,
    pub exhausted: bool,
    pub created_at: Instant,
}

pub struct CursorBuilder {
    collection: String,
    options: CursorOptions,
}

impl CursorBuilder {
    pub fn new(collection: impl Into<String>) -> Self {
        Self {
            collection: collection.into(),
            options: CursorOptions::default(),
        }
    }

    pub fn batch_size(mut self, size: u32) -> Self {
        self.options.batch_size = size;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = Some(timeout);
        self
    }

    pub fn no_timeout(mut self) -> Self {
        self.options.no_cursor_timeout = true;
        self
    }

    pub fn allow_partial_results(mut self, allow: bool) -> Self {
        self.options.allow_partial_results = allow;
        self
    }

    pub fn max_await_time(mut self, time: Duration) -> Self {
        self.options.max_await_time = Some(time);
        self
    }

    pub fn build(self) -> Cursor<Document> {
        Cursor::new(self.collection, self.options)
    }

    pub fn build_with_data(self, data: Vec<Document>) -> Cursor<Document> {
        let cursor = self.build();
        cursor.add_batch(data, true);
        cursor
    }
}

#[cfg(feature = "async")]
pub mod async_cursor {
    use super::*;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio_stream::Stream;

    pub struct AsyncCursor<T> {
        inner: Arc<Cursor<T>>,
    }

    impl<T: Clone + Send + Sync + 'static> AsyncCursor<T> {
        pub fn new(cursor: Arc<Cursor<T>>) -> Self {
            Self { inner: cursor }
        }

        pub async fn next(&self) -> Option<T> {
            self.inner.next()
        }

        pub async fn try_next(&self) -> MikuResult<Option<T>> {
            self.inner.try_next()
        }

        pub async fn collect(&self) -> Vec<T> {
            self.inner.collect_all()
        }

        pub fn id(&self) -> u64 {
            self.inner.id()
        }

        pub fn is_exhausted(&self) -> bool {
            self.inner.is_exhausted()
        }
    }

    impl<T: Clone + Send + Sync + 'static> Stream for AsyncCursor<T> {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.inner.next())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_basic() {
        let cursor: Cursor<i32> = Cursor::from_vec("test", vec![1, 2, 3, 4, 5]);

        assert_eq!(cursor.id(), 1);
        assert_eq!(cursor.collection(), "test");
        assert_eq!(cursor.buffered_count(), 5);
    }

    #[test]
    fn test_cursor_iteration() {
        let cursor: Cursor<i32> = Cursor::from_vec("test", vec![1, 2, 3]);

        assert_eq!(cursor.next(), Some(1));
        assert_eq!(cursor.next(), Some(2));
        assert_eq!(cursor.next(), Some(3));
        assert_eq!(cursor.next(), None);
        assert!(cursor.is_exhausted());
    }

    #[test]
    fn test_cursor_take() {
        let cursor: Cursor<i32> = Cursor::from_vec("test", vec![1, 2, 3, 4, 5]);

        let batch = cursor.take(3);
        assert_eq!(batch, vec![1, 2, 3]);
        assert_eq!(cursor.buffered_count(), 2);
    }

    #[test]
    fn test_cursor_peek() {
        let cursor: Cursor<i32> = Cursor::from_vec("test", vec![1, 2, 3]);

        assert_eq!(cursor.peek(), Some(1));
        assert_eq!(cursor.peek(), Some(1));
        assert_eq!(cursor.next(), Some(1));
        assert_eq!(cursor.peek(), Some(2));
    }

    #[test]
    fn test_cursor_collect_all() {
        let cursor: Cursor<i32> = Cursor::from_vec("test", vec![1, 2, 3, 4, 5]);

        let all = cursor.collect_all();
        assert_eq!(all, vec![1, 2, 3, 4, 5]);
        assert!(cursor.is_exhausted());
        assert_eq!(cursor.total_returned(), 5);
    }

    #[test]
    fn test_cursor_builder() {
        let cursor = CursorBuilder::new("users")
            .batch_size(50)
            .timeout(Duration::from_secs(300))
            .build();

        assert_eq!(cursor.collection(), "users");
        assert_eq!(cursor.options.batch_size, 50);
    }

    #[test]
    fn test_cursor_manager() {
        let manager = CursorManager::new();

        let cursor1 = Cursor::from_vec("test1", vec![Document::new()]);
        let cursor2 = Cursor::from_vec("test2", vec![Document::new()]);

        let c1 = manager.register(cursor1);
        let c2 = manager.register(cursor2);

        assert_eq!(manager.active_count(), 2);

        manager.remove(c1.id());
        assert_eq!(manager.active_count(), 1);

        manager.kill_all();
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_cursor_timeout() {
        let mut options = CursorOptions::default();
        options.timeout = Some(Duration::from_millis(1));

        let cursor: Cursor<i32> = Cursor::new("test", options);
        cursor.add_batch(vec![1, 2, 3], true);

        std::thread::sleep(Duration::from_millis(10));

        assert!(cursor.is_timed_out());
    }

    #[test]
    fn test_cursor_no_timeout() {
        let mut options = CursorOptions::default();
        options.no_cursor_timeout = true;
        options.timeout = Some(Duration::from_millis(1));

        let cursor: Cursor<i32> = Cursor::new("test", options);

        std::thread::sleep(Duration::from_millis(10));

        assert!(!cursor.is_timed_out());
    }
}
