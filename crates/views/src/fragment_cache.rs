//! Byte-bounded value caching for legacy JSON and shared API facts.
pub use campfire_presentation::cache_keys::{cache_version, cache_key_with_version};

use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub mod keys;

/// The store's default limit: `MemoryStore`'s default `size`, 32 MB.
pub const DEFAULT_MAX_BYTES: usize = 32 * 1024 * 1024;

/// What an entry costs beyond its key and payload (`MemoryStore::PER_ENTRY_OVERHEAD`).
pub const PER_ENTRY_OVERHEAD: usize = 240;

/// The bytes a cached value accounts for, like the `bytesize` of the payload `MemoryStore` and
/// Redis keep. Shared parts (an `Arc`) count in full: an entry is charged for what it holds alive.
pub trait CacheSize {
    fn cache_size(&self) -> usize;
}

/// A string holds its capacity, not just its length.
impl CacheSize for String {
    fn cache_size(&self) -> usize {
        self.capacity()
    }
}

impl<T: CacheSize + ?Sized> CacheSize for Arc<T> {
    fn cache_size(&self) -> usize {
        T::cache_size(self)
    }
}

/// The length of `value`'s JSON: the payload size of a Jbuilder value.
pub fn serialized_size(value: &impl serde::Serialize) -> usize {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, value).map_or(0, |()| count.0)
}

type Value = Arc<dyn Any + Send + Sync>;

/// A byte-bounded in-process value store.
pub struct FragmentCache {
    max_bytes: usize,
    entries: Mutex<Entries>,
}

#[derive(Default)]
struct Entries {
    values: HashMap<Arc<str>, Entry>,
    /// Last use → key, oldest first.
    recency: BTreeMap<u64, Arc<str>>,
    clock: u64,
    /// The sum of every entry's `size`.
    bytes: usize,
}

struct Entry {
    /// The map's key, shared with `recency`.
    key: Arc<str>,
    value: Value,
    used: u64,
    size: usize,
}

impl Entry {
    /// Marks the entry used at `now`.
    fn touch(&mut self, recency: &mut BTreeMap<u64, Arc<str>>, now: u64) {
        recency.remove(&self.used);
        self.used = now;
        recency.insert(now, self.key.clone());
    }
}

impl FragmentCache {
    /// A store that keeps at most `max_bytes` of entries (as [`CacheSize`] and
    /// [`PER_ENTRY_OVERHEAD`] count them).
    pub fn new(max_bytes: usize) -> Arc<Self> {
        Arc::new(Self {
            max_bytes,
            entries: Mutex::default(),
        })
    }

    /// `Rails.cache.fetch(key) { value }` for any cloneable value (Jbuilder caches the hash it
    /// built, not its JSON).
    pub fn fetch_value<T: CacheSize + Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        compute: impl FnOnce() -> T,
    ) -> T {
        match self.try_fetch_value(key, || Ok::<T, std::convert::Infallible>(compute())) {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }

    /// [`Self::fetch_value`] where computing can fail: nothing is stored then. When two computations
    /// of a key race, the first one stored is what both return.
    pub fn try_fetch_value<T: CacheSize + Clone + Send + Sync + 'static, E>(
        &self,
        key: &str,
        compute: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        if let Some(value) = self.read::<T>(key) {
            return Ok(value);
        }
        // Compute outside the lock so nested cache reads can proceed.
        let value = compute()?;
        let size = key.len() + value.cache_size() + PER_ENTRY_OVERHEAD;
        Ok(self.write(key, value, size))
    }

    /// The value `key` holds, if any (a use, for eviction).
    pub fn get<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        self.read(key)
    }

    pub fn len(&self) -> usize {
        self.lock().values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bytes the entries account for.
    pub fn bytes(&self) -> usize {
        self.lock().bytes
    }

    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    pub fn clear(&self) {
        *self.lock() = Entries::default();
    }

    /// Stores `value` unless `key` already holds one of its type, and returns what `key` holds.
    fn read<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        let mut entries = self.lock();
        let Entries {
            values,
            recency,
            clock,
            ..
        } = &mut *entries;
        let entry = values.get_mut(key)?;
        let value = entry.value.downcast_ref::<T>()?.clone();
        *clock += 1;
        entry.touch(recency, *clock);
        Some(value)
    }

    fn write<T: Clone + Send + Sync + 'static>(&self, key: &str, value: T, size: usize) -> T {
        let mut entries = self.lock();
        let Entries {
            values,
            recency,
            clock,
            bytes,
        } = &mut *entries;
        *clock += 1;
        if let Some(entry) = values.get_mut(key) {
            if let Some(stored) = entry.value.downcast_ref::<T>() {
                let stored = stored.clone();
                entry.touch(recency, *clock);
                return stored;
            }
            if let Some(replaced) = values.remove(key) {
                recency.remove(&replaced.used);
                *bytes -= replaced.size;
            }
        }
        if size > self.max_bytes / 4 {
            return value;
        }
        let key: Arc<str> = key.into();
        values.insert(
            key.clone(),
            Entry {
                key: key.clone(),
                value: Arc::new(value.clone()),
                used: *clock,
                size,
            },
        );
        recency.insert(*clock, key);
        *bytes += size;
        if *bytes > self.max_bytes {
            // `MemoryStore#prune(@max_size * 0.75)`
            let target = self.max_bytes / 4 * 3;
            while *bytes > target {
                let Some((_, oldest)) = recency.pop_first() else {
                    break;
                };
                if let Some(entry) = values.remove(&oldest) {
                    *bytes -= entry.size;
                }
            }
        }
        value
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Entries> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

thread_local! {
    static CURRENT: RefCell<Option<Arc<FragmentCache>>> = const { RefCell::new(None) };
}

/// Runs `f` with `cache` as this thread's current store.
pub fn with<R>(cache: &Arc<FragmentCache>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Arc<FragmentCache>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            CURRENT.with(|current| *current.borrow_mut() = previous);
        }
    }
    let _restore = Restore(CURRENT.with(|current| current.borrow_mut().replace(cache.clone())));
    f()
}

/// This thread's current store, if any.
pub fn current() -> Option<Arc<FragmentCache>> {
    CURRENT.with(|current| current.borrow().clone())
}

/// `json.cache! key do ... end` against the current store (uncached without one).
pub fn try_fetch_value<T: CacheSize + Clone + Send + Sync + 'static, E>(
    key: impl FnOnce() -> String,
    compute: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    match current() {
        Some(cache) => cache.try_fetch_value(&key(), compute),
        None => compute(),
    }
}

/// A future that has `cache` as the current store whenever it's polled: a request's handler,
/// whose synchronous reads (on whichever worker thread polls it) then see the store.
pub struct Scoped<F> {
    cache: Arc<FragmentCache>,
    future: Pin<Box<F>>,
}

impl<F: Future> Scoped<F> {
    pub fn new(cache: Arc<FragmentCache>, future: F) -> Self {
        Self {
            cache,
            future: Box::pin(future),
        }
    }
}

impl<F: Future> Future for Scoped<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = &mut *self;
        with(&this.cache, || this.future.as_mut().poll(cx))
    }
}

impl CacheSize for campfire_presentation::messages::json::UserJson {
    fn cache_size(&self) -> usize { serialized_size(self) }
}
impl CacheSize for campfire_presentation::messages::json::MessageJson {
    fn cache_size(&self) -> usize { serialized_size(self) }
}
impl CacheSize for campfire_presentation::messages::json::BoostJson {
    fn cache_size(&self) -> usize { serialized_size(self) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIG: usize = 1 << 20;

    impl CacheSize for i32 {
        fn cache_size(&self) -> usize {
            4
        }
    }

    /// What a one-letter key holding `payload` bytes costs.
    fn entry(payload: usize) -> usize {
        1 + payload + PER_ENTRY_OVERHEAD
    }

    #[test]
    fn the_first_rendering_is_reused() {
        let cache = FragmentCache::new(BIG);
        assert_eq!(cache.fetch_value("a", || "first".to_string()), "first");
        assert_eq!(cache.fetch_value("a", || "second".to_string()), "first");
        assert_eq!(cache.fetch_value("b", || "other".to_string()), "other");
    }

    #[test]
    fn entries_count_their_key_payload_and_overhead() {
        let cache = FragmentCache::new(BIG);
        cache.fetch_value("a", || "x".repeat(100));
        cache.fetch_value("bb", || "y".repeat(10));
        assert_eq!(
            cache.bytes(),
            (1 + 100 + PER_ENTRY_OVERHEAD) + (2 + 10 + PER_ENTRY_OVERHEAD)
        );
        cache.fetch_value::<String>("a", || unreachable!());
        assert_eq!(
            cache.bytes(),
            entry(100) + (2 + 10 + PER_ENTRY_OVERHEAD),
            "a hit costs nothing"
        );
        cache.clear();
        assert_eq!(cache.bytes(), 0);
    }

    #[test]
    fn the_byte_limit_is_enforced() {
        let max = 10 * entry(1000);
        let cache = FragmentCache::new(max);
        for i in 0..1000 {
            cache.fetch_value(&format!("{}", i % 3), || "x".repeat(1000));
            cache.fetch_value(&format!("k{i}"), || "x".repeat(1000));
            assert!(
                cache.bytes() <= max,
                "{} bytes after {i} writes",
                cache.bytes()
            );
        }
        assert!(cache.len() < 20);
        assert!(
            cache.bytes() > max / 2,
            "pruning stops at three quarters, not empty"
        );
        for hot in 0..3 {
            assert!(
                cache.get::<String>(&hot.to_string()).is_some(),
                "entry {hot} is used every sixth write"
            );
        }
    }

    #[test]
    fn going_over_prunes_to_three_quarters_least_recently_used_first() {
        let cache = FragmentCache::new(4 * entry(100));
        for key in ["a", "b", "c", "d"] {
            cache.fetch_value(key, || "x".repeat(100));
        }
        cache.fetch_value::<String>("a", || unreachable!("a is still stored"));
        assert_eq!(cache.len(), 4);
        // Five entries is over; three quarters of the limit holds three.
        cache.fetch_value("e", || "x".repeat(100));
        assert_eq!(cache.len(), 3);
        assert!(cache.bytes() <= 3 * entry(100));
        assert_eq!(
            cache.get::<String>("b"),
            None,
            "b was least recently used"
        );
        assert_eq!(cache.get::<String>("c"), None, "c went next");
        for key in ["a", "d", "e"] {
            assert!(
                cache.get::<String>(key).is_some(),
                "{key} survives (a was used after d)"
            );
        }
    }

    #[test]
    fn a_value_larger_than_the_store_is_returned_but_not_kept() {
        let cache = FragmentCache::new(entry(10));
        assert_eq!(cache.fetch_value("a", || "x".repeat(100)), "x".repeat(100));
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.bytes(), 0);
    }

    #[test]
    fn a_value_larger_than_a_quarter_of_the_store_doesnt_evict_the_rest() {
        let cache = FragmentCache::new(8 * entry(100));
        for key in ["a", "b", "c"] {
            cache.fetch_value(key, || "x".repeat(100));
        }
        let big = "y".repeat(3 * entry(100));
        assert_eq!(cache.fetch_value("big", || big.clone()), big);
        assert_eq!(cache.len(), 3, "the big value isn't kept");
        assert_eq!(cache.get::<String>("big"), None);
        for key in ["a", "b", "c"] {
            assert!(
                cache.get::<String>(key).is_some(),
                "{key} is still stored"
            );
        }
        assert_eq!(cache.bytes(), 3 * entry(100));
    }

    #[test]
    fn concurrent_use_stays_within_the_limit() {
        let max = 64 * entry(200);
        let cache = FragmentCache::new(max);
        std::thread::scope(|scope| {
            for t in 0..8 {
                let cache = &cache;
                scope.spawn(move || {
                    for i in 0..5000 {
                        let hot = cache.fetch_value(&format!("{}", i % 8), || "x".repeat(200));
                        assert_eq!(hot, "x".repeat(200));
                        cache.fetch_value(&format!("cold/{t}/{i}"), || "y".repeat(200));
                        assert!(cache.bytes() <= max);
                    }
                });
            }
        });
        let len = cache.len();
        assert!(len > 0 && cache.bytes() <= max);
        assert_eq!(
            cache.bytes(),
            cache
                .lock()
                .values
                .values()
                .map(|entry| entry.size)
                .sum::<usize>()
        );
        assert_eq!(
            cache.lock().recency.len(),
            len,
            "every entry is in the recency order once"
        );
    }

    #[test]
    fn racing_renders_all_return_the_first_stored_fragment() {
        let cache = FragmentCache::new(BIG);
        let barrier = std::sync::Barrier::new(4);
        let values: Vec<Arc<String>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|t| {
                    let (cache, barrier) = (&cache, &barrier);
                    scope.spawn(move || {
                        cache.fetch_value("k", || {
                            barrier.wait();
                            Arc::new(t.to_string())
                        })
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect()
        });
        assert!(
            values.iter().all(|value| Arc::ptr_eq(value, &values[0])),
            "{values:?}"
        );
        assert!(Arc::ptr_eq(
            &cache.get::<Arc<String>>("k").unwrap(),
            &values[0]
        ));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn jbuilder_values_count_their_json() {
        assert_eq!(serialized_size(&vec!["ab", "c"]), r#"["ab","c"]"#.len());
    }

    #[test]
    fn failures_are_not_stored() {
        let cache = FragmentCache::new(BIG);
        assert!(
            cache
                .try_fetch_value::<i32, _>("k", || Err("boom"))
                .is_err()
        );
        assert_eq!(cache.try_fetch_value::<i32, &str>("k", || Ok(1)), Ok(1));
        assert_eq!(cache.try_fetch_value::<i32, &str>("k", || Ok(2)), Ok(1));
    }

    #[test]
    fn keys_use_usec_versions() {
        let time: jiff::Timestamp = "2024-06-01T12:00:00.000123Z".parse().unwrap();
        assert_eq!(
            cache_key_with_version("messages", 1, time),
            "messages/1-20240601120000000123"
        );
    }
}
