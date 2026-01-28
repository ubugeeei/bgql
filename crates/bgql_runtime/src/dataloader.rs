//! DataLoader for Better GraphQL.
//!
//! Provides batching and caching to prevent N+1 queries.

use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// A DataLoader that batches and caches loads.
pub struct DataLoader<K, V, F>
where
    K: Eq + Hash + Clone + Send,
    V: Clone + Send,
    F: Fn(Vec<K>) -> std::pin::Pin<Box<dyn Future<Output = HashMap<K, V>> + Send>> + Send + Sync,
{
    batch_fn: Arc<F>,
    cache: Arc<RwLock<FxHashMap<K, V>>>,
    batch: Arc<Mutex<Vec<K>>>,
    batch_size: usize,
}

impl<K, V, F> DataLoader<K, V, F>
where
    K: Eq + Hash + Clone + Send + 'static,
    V: Clone + Send + 'static,
    F: Fn(Vec<K>) -> std::pin::Pin<Box<dyn Future<Output = HashMap<K, V>> + Send>>
        + Send
        + Sync
        + 'static,
{
    /// Creates a new DataLoader.
    pub fn new(batch_fn: F) -> Self {
        Self {
            batch_fn: Arc::new(batch_fn),
            cache: Arc::new(RwLock::new(FxHashMap::default())),
            batch: Arc::new(Mutex::new(Vec::new())),
            batch_size: 100,
        }
    }

    /// Sets the maximum batch size.
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Loads a value by key.
    pub async fn load(&self, key: K) -> Option<V> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(value) = cache.get(&key) {
                return Some(value.clone());
            }
        }

        // Add to batch and trigger batch load
        let keys = {
            let mut batch = self.batch.lock().await;
            batch.push(key.clone());
            if batch.len() >= self.batch_size {
                std::mem::take(&mut *batch)
            } else {
                return None; // Will be loaded in batch
            }
        };

        // Execute batch
        let results = (self.batch_fn)(keys).await;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            for (k, v) in results.iter() {
                cache.insert(k.clone(), v.clone());
            }
        }

        // Return the requested value
        results.get(&key).cloned()
    }

    /// Loads multiple values.
    pub async fn load_many(&self, keys: Vec<K>) -> HashMap<K, V> {
        let results = (self.batch_fn)(keys).await;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            for (k, v) in results.iter() {
                cache.insert(k.clone(), v.clone());
            }
        }

        results
    }

    /// Clears the cache.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Clears a specific key from the cache.
    pub async fn clear_key(&self, key: &K) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    /// Primes the cache with a value.
    pub async fn prime(&self, key: K, value: V) {
        let mut cache = self.cache.write().await;
        cache.insert(key, value);
    }
}

/// Creates a simple DataLoader with a batch function.
#[allow(clippy::type_complexity)]
pub fn create_loader<K, V, F, Fut>(
    batch_fn: F,
) -> DataLoader<
    K,
    V,
    impl Fn(Vec<K>) -> std::pin::Pin<Box<dyn Future<Output = HashMap<K, V>> + Send>> + Send + Sync,
>
where
    K: Eq + Hash + Clone + Send + 'static,
    V: Clone + Send + 'static,
    F: Fn(Vec<K>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = HashMap<K, V>> + Send + 'static,
{
    DataLoader::new(move |keys| Box::pin(batch_fn(keys)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dataloader() {
        let loader = create_loader(|keys: Vec<i32>| async move {
            keys.into_iter().map(|k| (k, k * 2)).collect()
        });

        let result = loader.load_many(vec![1, 2, 3]).await;
        assert_eq!(result.get(&1), Some(&2));
        assert_eq!(result.get(&2), Some(&4));
        assert_eq!(result.get(&3), Some(&6));
    }
}
