//! Streaming support for Better GraphQL.
//!
//! This module provides types and utilities for streaming GraphQL responses
//! using `@defer` and `@stream` directives.
//!
//! # Example
//!
//! ```ignore
//! use bgql_sdk::streaming::{StreamingResponse, DeferPayload, PathSegment};
//!
//! // Create a streaming response
//! let mut response = StreamingResponse::new(serde_json::json!({
//!     "user": {
//!         "id": "1",
//!         "name": "Alice"
//!     }
//! }));
//! response = response.with_pending();
//!
//! // Later, send deferred data
//! let defer_payload = DeferPayload {
//!     path: vec![PathSegment::Field("user".into())],
//!     data: serde_json::json!({"bio": "Hello!"}),
//!     label: Some("userBio".into()),
//!     has_next: false,
//!     errors: None,
//! };
//! ```

// Re-export streaming types from runtime
pub use bgql_runtime::streaming::{
    DeferPayload, IncrementalEvent, PathSegment, StreamPayload, StreamingResponse,
};

// Re-export execution state types for resumable queries
pub use bgql_runtime::state::{
    BinaryStreamPhase, BinaryStreamState, Checkpoint, ExecutionId, ExecutionPhase,
    ExecutionPosition, ExecutionState, ExecutionStats, ResumeToken, StreamCursor,
};

use tokio::sync::mpsc;

/// A stream of incremental GraphQL events.
///
/// This is used to deliver `@defer` and `@stream` payloads to clients
/// over chunked HTTP responses or WebSocket connections.
pub struct IncrementalStream {
    receiver: mpsc::Receiver<IncrementalEvent>,
    completed: bool,
}

impl IncrementalStream {
    /// Creates a new incremental stream with the given channel receiver.
    pub fn new(receiver: mpsc::Receiver<IncrementalEvent>) -> Self {
        Self {
            receiver,
            completed: false,
        }
    }

    /// Creates a new incremental stream and returns both the stream and sender.
    pub fn channel(buffer: usize) -> (Self, IncrementalSender) {
        let (tx, rx) = mpsc::channel(buffer);
        (Self::new(rx), IncrementalSender { sender: tx })
    }

    /// Checks if the stream has completed.
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Receives the next event from the stream.
    pub async fn next(&mut self) -> Option<IncrementalEvent> {
        if self.completed {
            return None;
        }

        match self.receiver.recv().await {
            Some(IncrementalEvent::Complete) => {
                self.completed = true;
                Some(IncrementalEvent::Complete)
            }
            Some(event) => Some(event),
            None => {
                self.completed = true;
                None
            }
        }
    }
}

/// Sender for incremental events.
pub struct IncrementalSender {
    sender: mpsc::Sender<IncrementalEvent>,
}

impl IncrementalSender {
    /// Sends a defer payload.
    pub async fn send_defer(&self, payload: DeferPayload) -> Result<(), SendError> {
        self.sender
            .send(IncrementalEvent::Defer(payload))
            .await
            .map_err(|_| SendError::Closed)
    }

    /// Sends a stream payload.
    pub async fn send_stream(&self, payload: StreamPayload) -> Result<(), SendError> {
        self.sender
            .send(IncrementalEvent::Stream(payload))
            .await
            .map_err(|_| SendError::Closed)
    }

    /// Signals that the stream is complete.
    pub async fn complete(&self) -> Result<(), SendError> {
        self.sender
            .send(IncrementalEvent::Complete)
            .await
            .map_err(|_| SendError::Closed)
    }

    /// Checks if the receiver has been dropped.
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl Clone for IncrementalSender {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

/// Error when sending incremental events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendError {
    /// The receiver has been dropped.
    Closed,
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::Closed => write!(f, "channel closed"),
        }
    }
}

impl std::error::Error for SendError {}

/// Builder for creating defer payloads.
#[derive(Debug, Clone, Default)]
pub struct DeferPayloadBuilder {
    path: Vec<PathSegment>,
    data: serde_json::Value,
    label: Option<String>,
    has_next: bool,
    errors: Option<Vec<serde_json::Value>>,
}

impl DeferPayloadBuilder {
    /// Creates a new defer payload builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path for this defer payload.
    pub fn path(mut self, path: Vec<PathSegment>) -> Self {
        self.path = path;
        self
    }

    /// Adds a path segment.
    pub fn at_field(mut self, field: impl Into<String>) -> Self {
        self.path.push(PathSegment::Field(field.into()));
        self
    }

    /// Adds an index path segment.
    pub fn at_index(mut self, index: usize) -> Self {
        self.path.push(PathSegment::Index(index));
        self
    }

    /// Sets the data for this defer payload.
    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Sets the label for this defer payload.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Marks that there are more payloads coming.
    pub fn has_next(mut self) -> Self {
        self.has_next = true;
        self
    }

    /// Marks this as the final payload.
    pub fn final_payload(mut self) -> Self {
        self.has_next = false;
        self
    }

    /// Adds an error to this payload.
    pub fn with_error(mut self, error: serde_json::Value) -> Self {
        self.errors.get_or_insert_with(Vec::new).push(error);
        self
    }

    /// Builds the defer payload.
    pub fn build(self) -> DeferPayload {
        DeferPayload {
            path: self.path,
            data: self.data,
            label: self.label,
            has_next: self.has_next,
            errors: self.errors,
        }
    }
}

/// Builder for creating stream payloads.
#[derive(Debug, Clone, Default)]
pub struct StreamPayloadBuilder {
    path: Vec<PathSegment>,
    items: Vec<serde_json::Value>,
    label: Option<String>,
    has_next: bool,
    errors: Option<Vec<serde_json::Value>>,
}

impl StreamPayloadBuilder {
    /// Creates a new stream payload builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path for this stream payload.
    pub fn path(mut self, path: Vec<PathSegment>) -> Self {
        self.path = path;
        self
    }

    /// Adds a path segment.
    pub fn at_field(mut self, field: impl Into<String>) -> Self {
        self.path.push(PathSegment::Field(field.into()));
        self
    }

    /// Adds an index path segment.
    pub fn at_index(mut self, index: usize) -> Self {
        self.path.push(PathSegment::Index(index));
        self
    }

    /// Sets the items for this stream payload.
    pub fn items(mut self, items: Vec<serde_json::Value>) -> Self {
        self.items = items;
        self
    }

    /// Adds an item to the stream.
    pub fn add_item(mut self, item: serde_json::Value) -> Self {
        self.items.push(item);
        self
    }

    /// Sets the label for this stream payload.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Marks that there are more items coming.
    pub fn has_next(mut self) -> Self {
        self.has_next = true;
        self
    }

    /// Marks this as the final batch.
    pub fn final_batch(mut self) -> Self {
        self.has_next = false;
        self
    }

    /// Adds an error to this payload.
    pub fn with_error(mut self, error: serde_json::Value) -> Self {
        self.errors.get_or_insert_with(Vec::new).push(error);
        self
    }

    /// Builds the stream payload.
    pub fn build(self) -> StreamPayload {
        StreamPayload {
            path: self.path,
            items: self.items,
            label: self.label,
            has_next: self.has_next,
            errors: self.errors,
        }
    }
}

/// Multipart response writer for streaming GraphQL responses.
///
/// Implements the `multipart/mixed` content type for incremental delivery.
#[derive(Debug)]
pub struct MultipartWriter {
    boundary: String,
    content_type: String,
}

impl MultipartWriter {
    /// Creates a new multipart writer with a random boundary.
    pub fn new() -> Self {
        let boundary = format!("----bgql{:x}", rand_boundary());
        Self {
            content_type: format!("multipart/mixed; boundary={}", boundary),
            boundary,
        }
    }

    /// Returns the Content-Type header value.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Returns the boundary string.
    pub fn boundary(&self) -> &str {
        &self.boundary
    }

    /// Formats the initial response part.
    pub fn format_initial(&self, data: &serde_json::Value, has_next: bool) -> String {
        let body = serde_json::json!({
            "data": data,
            "hasNext": has_next
        });
        format!(
            "--{}\r\nContent-Type: application/json\r\n\r\n{}\r\n",
            self.boundary,
            serde_json::to_string(&body).unwrap_or_default()
        )
    }

    /// Formats an incremental part.
    pub fn format_incremental(&self, event: &IncrementalEvent) -> String {
        match event {
            IncrementalEvent::Defer(payload) => {
                let body = serde_json::json!({
                    "incremental": [{
                        "path": payload.path,
                        "data": payload.data,
                        "label": payload.label,
                        "errors": payload.errors,
                    }],
                    "hasNext": payload.has_next
                });
                format!(
                    "--{}\r\nContent-Type: application/json\r\n\r\n{}\r\n",
                    self.boundary,
                    serde_json::to_string(&body).unwrap_or_default()
                )
            }
            IncrementalEvent::Stream(payload) => {
                let body = serde_json::json!({
                    "incremental": [{
                        "path": payload.path,
                        "items": payload.items,
                        "label": payload.label,
                        "errors": payload.errors,
                    }],
                    "hasNext": payload.has_next
                });
                format!(
                    "--{}\r\nContent-Type: application/json\r\n\r\n{}\r\n",
                    self.boundary,
                    serde_json::to_string(&body).unwrap_or_default()
                )
            }
            IncrementalEvent::Complete => {
                format!(
                    "--{}\r\nContent-Type: application/json\r\n\r\n{{\"hasNext\":false}}\r\n--{}--\r\n",
                    self.boundary, self.boundary
                )
            }
        }
    }
}

impl Default for MultipartWriter {
    fn default() -> Self {
        Self::new()
    }
}

fn rand_boundary() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defer_payload_builder() {
        let payload = DeferPayloadBuilder::new()
            .at_field("user")
            .at_field("bio")
            .data(serde_json::json!("Hello, World!"))
            .label("userBio")
            .final_payload()
            .build();

        assert_eq!(payload.path.len(), 2);
        assert!(matches!(&payload.path[0], PathSegment::Field(f) if f == "user"));
        assert!(matches!(&payload.path[1], PathSegment::Field(f) if f == "bio"));
        assert_eq!(payload.label, Some("userBio".to_string()));
        assert!(!payload.has_next);
    }

    #[test]
    fn test_stream_payload_builder() {
        let payload = StreamPayloadBuilder::new()
            .at_field("posts")
            .add_item(serde_json::json!({"id": "1", "title": "Post 1"}))
            .add_item(serde_json::json!({"id": "2", "title": "Post 2"}))
            .label("posts")
            .has_next()
            .build();

        assert_eq!(payload.items.len(), 2);
        assert!(payload.has_next);
    }

    #[tokio::test]
    async fn test_incremental_stream() {
        let (mut stream, sender) = IncrementalStream::channel(10);

        // Send a defer payload
        sender
            .send_defer(
                DeferPayloadBuilder::new()
                    .at_field("user")
                    .data(serde_json::json!({"name": "Alice"}))
                    .has_next()
                    .build(),
            )
            .await
            .unwrap();

        // Send complete signal
        sender.complete().await.unwrap();

        // Receive events
        let event1 = stream.next().await.unwrap();
        assert!(matches!(event1, IncrementalEvent::Defer(_)));

        let event2 = stream.next().await.unwrap();
        assert!(matches!(event2, IncrementalEvent::Complete));

        assert!(stream.is_completed());
    }

    #[test]
    fn test_multipart_writer() {
        let writer = MultipartWriter::new();

        assert!(writer.content_type().starts_with("multipart/mixed"));

        let initial = writer.format_initial(&serde_json::json!({"user": {"id": "1"}}), true);
        assert!(initial.contains("hasNext"));
        assert!(initial.contains("true"));

        let defer = IncrementalEvent::Defer(
            DeferPayloadBuilder::new()
                .at_field("user")
                .data(serde_json::json!({"bio": "Hello"}))
                .final_payload()
                .build(),
        );
        let formatted = writer.format_incremental(&defer);
        assert!(formatted.contains("incremental"));
    }
}
