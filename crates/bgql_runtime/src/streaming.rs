//! Streaming support for Better GraphQL (@defer/@stream).

use serde::{Deserialize, Serialize};

/// A streaming response that supports incremental delivery.
#[derive(Debug, Clone)]
pub struct StreamingResponse {
    /// Initial payload.
    pub initial: serde_json::Value,
    /// Whether there are more payloads coming.
    pub has_next: bool,
}

impl StreamingResponse {
    /// Creates a new streaming response.
    pub fn new(initial: serde_json::Value) -> Self {
        Self {
            initial,
            has_next: false,
        }
    }

    /// Marks that there are more payloads.
    pub fn with_pending(mut self) -> Self {
        self.has_next = true;
        self
    }
}

/// A deferred payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferPayload {
    /// Path to where this data should be merged.
    pub path: Vec<PathSegment>,
    /// The deferred data.
    pub data: serde_json::Value,
    /// Label for this defer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Whether there are more payloads.
    pub has_next: bool,
    /// Errors in this payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<serde_json::Value>>,
}

/// A streamed payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPayload {
    /// Path to where this data should be appended.
    pub path: Vec<PathSegment>,
    /// The streamed items.
    pub items: Vec<serde_json::Value>,
    /// Label for this stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Whether there are more items.
    pub has_next: bool,
    /// Errors in this payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<serde_json::Value>>,
}

/// A path segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathSegment {
    Field(String),
    Index(usize),
}

impl From<String> for PathSegment {
    fn from(s: String) -> Self {
        Self::Field(s)
    }
}

impl From<&str> for PathSegment {
    fn from(s: &str) -> Self {
        Self::Field(s.to_string())
    }
}

impl From<usize> for PathSegment {
    fn from(i: usize) -> Self {
        Self::Index(i)
    }
}

/// Incremental delivery event.
#[derive(Debug, Clone)]
pub enum IncrementalEvent {
    /// A deferred payload.
    Defer(DeferPayload),
    /// A streamed payload.
    Stream(StreamPayload),
    /// End of stream.
    Complete,
}
