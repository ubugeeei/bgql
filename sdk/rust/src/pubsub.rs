//! Publish/Subscribe system for BGQL subscriptions.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

const DEFAULT_CAPACITY: usize = 256;

/// A publish/subscribe hub for GraphQL subscriptions.
#[derive(Clone)]
pub struct PubSub {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<serde_json::Value>>>>,
    capacity: usize,
}

impl Default for PubSub {
    fn default() -> Self {
        Self::new()
    }
}

impl PubSub {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            capacity: DEFAULT_CAPACITY,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            capacity,
        }
    }

    pub async fn publish(&self, topic: impl AsRef<str>, event: serde_json::Value) -> usize {
        let topic = topic.as_ref();
        let channels = self.channels.read().await;

        if let Some(sender) = channels.get(topic) {
            match sender.send(event) {
                Ok(count) => count,
                Err(_) => 0,
            }
        } else {
            0
        }
    }

    pub async fn subscribe(
        &self,
        topic: impl Into<String>,
    ) -> broadcast::Receiver<serde_json::Value> {
        let topic = topic.into();
        let mut channels = self.channels.write().await;

        if let Some(sender) = channels.get(&topic) {
            sender.subscribe()
        } else {
            let (sender, receiver) = broadcast::channel(self.capacity);
            channels.insert(topic, sender);
            receiver
        }
    }

    pub async fn topic_count(&self) -> usize {
        self.channels.read().await.len()
    }

    pub async fn has_subscribers(&self, topic: &str) -> bool {
        let channels = self.channels.read().await;
        if let Some(sender) = channels.get(topic) {
            sender.receiver_count() > 0
        } else {
            false
        }
    }

    pub async fn cleanup(&self) {
        let mut channels = self.channels.write().await;
        channels.retain(|_, sender| sender.receiver_count() > 0);
    }
}

/// A typed wrapper around PubSub for type-safe event publishing.
pub struct TypedPubSub<T> {
    inner: PubSub,
    topic: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: serde::Serialize + serde::de::DeserializeOwned + Send + 'static> TypedPubSub<T> {
    pub fn new(pubsub: PubSub, topic: impl Into<String>) -> Self {
        Self {
            inner: pubsub,
            topic: topic.into(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn publish(&self, event: T) -> usize {
        if let Ok(value) = serde_json::to_value(&event) {
            self.inner.publish(&self.topic, value).await
        } else {
            0
        }
    }

    pub async fn subscribe(&self) -> TypedReceiver<T> {
        let receiver = self.inner.subscribe(&self.topic).await;
        TypedReceiver {
            inner: receiver,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// A typed receiver for subscription events.
pub struct TypedReceiver<T> {
    inner: broadcast::Receiver<serde_json::Value>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: serde::de::DeserializeOwned> TypedReceiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        match self.inner.recv().await {
            Ok(value) => serde_json::from_value(value).ok(),
            Err(_) => None,
        }
    }
}
