//! Execution state management for resumable queries.
//!
//! This module provides state management for streaming-first execution,
//! including checkpoints for pause/resume functionality.

use crate::streaming::PathSegment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Unique identifier for an execution.
pub type ExecutionId = String;

/// Unique identifier for a checkpoint.
pub type CheckpointId = String;

/// Token used to resume execution.
pub type ResumeToken = String;

/// Execution state for a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    /// Unique identifier for this execution.
    pub id: ExecutionId,

    /// Hash of the query for validation on resume.
    pub query_hash: String,

    /// Variables used in the query.
    pub variables: HashMap<String, serde_json::Value>,

    /// Current execution phase.
    pub phase: ExecutionPhase,

    /// Current position in the execution.
    pub current_position: ExecutionPosition,

    /// Checkpoints created during execution.
    pub checkpoints: Vec<Checkpoint>,

    /// Partial data accumulated so far.
    pub partial_data: serde_json::Value,

    /// Pending @defer labels.
    pub pending_defers: Vec<String>,

    /// Active @stream cursors.
    pub active_streams: HashMap<String, StreamCursor>,

    /// Token for resuming this execution.
    pub resume_token: Option<ResumeToken>,

    /// When this execution state expires.
    pub expires_at: Option<SystemTime>,

    /// Execution statistics.
    pub stats: ExecutionStats,

    /// Creation timestamp.
    pub created_at: SystemTime,

    /// Last update timestamp.
    pub updated_at: SystemTime,
}

impl ExecutionState {
    /// Creates a new execution state.
    pub fn new(id: ExecutionId, query_hash: String) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            query_hash,
            variables: HashMap::new(),
            phase: ExecutionPhase::Pending,
            current_position: ExecutionPosition::default(),
            checkpoints: Vec::new(),
            partial_data: serde_json::Value::Null,
            pending_defers: Vec::new(),
            active_streams: HashMap::new(),
            resume_token: None,
            expires_at: None,
            stats: ExecutionStats::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the TTL for this execution state.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.expires_at = Some(SystemTime::now() + ttl);
        self
    }

    /// Sets the variables.
    pub fn with_variables(mut self, variables: HashMap<String, serde_json::Value>) -> Self {
        self.variables = variables;
        self
    }

    /// Checks if the execution state has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Checks if execution can be resumed.
    pub fn can_resume(&self) -> bool {
        !self.is_expired()
            && matches!(
                self.phase,
                ExecutionPhase::Paused | ExecutionPhase::Streaming
            )
    }

    /// Creates a checkpoint at the current position.
    pub fn create_checkpoint(&mut self) -> Checkpoint {
        let checkpoint = Checkpoint {
            id: format!("{}-{}", self.id, self.checkpoints.len()),
            position: self.current_position.clone(),
            data_snapshot: self.partial_data.clone(),
            pending_defers: self.pending_defers.clone(),
            active_streams: self.active_streams.clone(),
            timestamp: SystemTime::now(),
        };
        self.checkpoints.push(checkpoint.clone());
        self.updated_at = SystemTime::now();
        checkpoint
    }

    /// Restores state from a checkpoint.
    pub fn restore_from_checkpoint(&mut self, checkpoint_id: &str) -> Option<&Checkpoint> {
        if let Some(checkpoint) = self.checkpoints.iter().find(|c| c.id == checkpoint_id) {
            self.current_position = checkpoint.position.clone();
            self.partial_data = checkpoint.data_snapshot.clone();
            self.pending_defers = checkpoint.pending_defers.clone();
            self.active_streams = checkpoint.active_streams.clone();
            self.phase = ExecutionPhase::Streaming;
            self.updated_at = SystemTime::now();
            Some(checkpoint)
        } else {
            None
        }
    }

    /// Generates a resume token.
    pub fn generate_resume_token(&mut self) -> ResumeToken {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.query_hash.hash(&mut hasher);
        self.updated_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);

        let token = format!("{:x}", hasher.finish());
        self.resume_token = Some(token.clone());
        token
    }

    /// Transitions to a new phase.
    pub fn transition_to(&mut self, phase: ExecutionPhase) {
        self.phase = phase;
        self.updated_at = SystemTime::now();
    }

    /// Marks a @defer as resolved.
    pub fn resolve_defer(&mut self, label: &str) {
        self.pending_defers.retain(|l| l != label);
        self.stats.defers_resolved += 1;
        self.updated_at = SystemTime::now();
    }

    /// Adds a pending @defer.
    pub fn add_pending_defer(&mut self, label: String) {
        if !self.pending_defers.contains(&label) {
            self.pending_defers.push(label);
        }
    }

    /// Updates stream cursor.
    pub fn update_stream_cursor(&mut self, label: &str, cursor: StreamCursor) {
        self.active_streams.insert(label.to_string(), cursor);
        self.updated_at = SystemTime::now();
    }

    /// Merges partial data at a path.
    pub fn merge_data(&mut self, path: &[PathSegment], data: serde_json::Value) {
        merge_at_path(&mut self.partial_data, path, data);
        self.updated_at = SystemTime::now();
    }
}

/// Phase of query execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPhase {
    /// Waiting to start.
    Pending,
    /// Currently executing.
    Running,
    /// Streaming incremental data.
    Streaming,
    /// Paused (can resume).
    Paused,
    /// Completed successfully.
    Completed,
    /// Failed with error.
    Failed,
    /// Cancelled by client.
    Cancelled,
}

/// Current position in execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionPosition {
    /// Path in the query being executed.
    pub path: Vec<PathSegment>,

    /// Index in current list (for @stream).
    pub list_index: Option<usize>,

    /// Cursor for stream pagination.
    pub stream_cursor: Option<String>,

    /// Offset in binary data.
    pub binary_offset: Option<u64>,

    /// Current depth in query.
    pub depth: usize,
}

impl ExecutionPosition {
    /// Creates a new position at a path.
    pub fn at_path(path: Vec<PathSegment>) -> Self {
        Self {
            depth: path.len(),
            path,
            ..Default::default()
        }
    }

    /// Creates a position for binary streaming.
    pub fn binary(offset: u64) -> Self {
        Self {
            binary_offset: Some(offset),
            ..Default::default()
        }
    }
}

/// A checkpoint for resumable execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique identifier.
    pub id: CheckpointId,

    /// Position at checkpoint.
    pub position: ExecutionPosition,

    /// Data accumulated up to this point.
    pub data_snapshot: serde_json::Value,

    /// Pending @defer labels at checkpoint.
    pub pending_defers: Vec<String>,

    /// Active stream cursors at checkpoint.
    pub active_streams: HashMap<String, StreamCursor>,

    /// When checkpoint was created.
    pub timestamp: SystemTime,
}

/// Cursor state for a @stream directive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamCursor {
    /// Label of the stream.
    pub label: String,

    /// Path to the stream field.
    pub path: Vec<PathSegment>,

    /// Number of items delivered so far.
    pub items_delivered: usize,

    /// Total items (if known).
    pub total_items: Option<usize>,

    /// Opaque cursor for pagination.
    pub cursor: Option<String>,

    /// Whether the stream has completed.
    pub completed: bool,
}

impl StreamCursor {
    /// Creates a new stream cursor.
    pub fn new(label: String, path: Vec<PathSegment>) -> Self {
        Self {
            label,
            path,
            items_delivered: 0,
            total_items: None,
            cursor: None,
            completed: false,
        }
    }

    /// Returns true if more items are available.
    pub fn has_more(&self) -> bool {
        !self.completed
            && self
                .total_items
                .map(|total| self.items_delivered < total)
                .unwrap_or(true)
    }

    /// Advances the cursor.
    pub fn advance(&mut self, count: usize, cursor: Option<String>) {
        self.items_delivered += count;
        self.cursor = cursor;
    }

    /// Marks the stream as completed.
    pub fn complete(&mut self) {
        self.completed = true;
    }
}

/// Statistics for query execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Total fields resolved.
    pub fields_resolved: usize,

    /// Total @defer payloads sent.
    pub defers_resolved: usize,

    /// Total @stream items sent.
    pub stream_items_sent: usize,

    /// Total bytes transferred (for binary streams).
    pub bytes_transferred: u64,

    /// Number of checkpoints created.
    pub checkpoints_created: usize,

    /// Number of times paused.
    pub pause_count: usize,

    /// Number of times resumed.
    pub resume_count: usize,

    /// Total execution duration (excluding paused time).
    pub execution_duration: Duration,
}

/// State for a binary stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStreamState {
    /// Unique identifier for the stream.
    pub id: String,

    /// Content type (MIME type).
    pub content_type: String,

    /// Total size in bytes (if known).
    pub total_size: Option<u64>,

    /// Chunk size for streaming.
    pub chunk_size: u32,

    /// Current offset.
    pub offset: u64,

    /// Bytes transferred so far.
    pub bytes_transferred: u64,

    /// Whether range requests are supported.
    pub supports_range: bool,

    /// Whether pause is supported.
    pub supports_pause: bool,

    /// Stream phase.
    pub phase: BinaryStreamPhase,
}

impl BinaryStreamState {
    /// Creates a new binary stream state.
    pub fn new(id: String, content_type: String) -> Self {
        Self {
            id,
            content_type,
            total_size: None,
            chunk_size: 65536, // 64KB default
            offset: 0,
            bytes_transferred: 0,
            supports_range: true,
            supports_pause: true,
            phase: BinaryStreamPhase::Pending,
        }
    }

    /// Sets the total size.
    pub fn with_total_size(mut self, size: u64) -> Self {
        self.total_size = Some(size);
        self
    }

    /// Sets the chunk size.
    pub fn with_chunk_size(mut self, size: u32) -> Self {
        self.chunk_size = size;
        self
    }

    /// Returns progress as percentage (0-100).
    pub fn progress(&self) -> Option<f64> {
        self.total_size
            .map(|total| (self.bytes_transferred as f64 / total as f64) * 100.0)
    }

    /// Advances the stream position.
    pub fn advance(&mut self, bytes: u64) {
        self.offset += bytes;
        self.bytes_transferred += bytes;
    }

    /// Checks if stream is complete.
    pub fn is_complete(&self) -> bool {
        if let Some(total) = self.total_size {
            self.bytes_transferred >= total
        } else {
            matches!(self.phase, BinaryStreamPhase::Completed)
        }
    }
}

/// Phase of binary stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryStreamPhase {
    /// Waiting to start.
    Pending,
    /// Streaming data.
    Streaming,
    /// Paused.
    Paused,
    /// Completed successfully.
    Completed,
    /// Failed with error.
    Failed,
}

/// Helper function to merge data at a path.
fn merge_at_path(target: &mut serde_json::Value, path: &[PathSegment], data: serde_json::Value) {
    if path.is_empty() {
        *target = data;
        return;
    }

    // Ensure target is an object if null
    if target.is_null() {
        *target = serde_json::json!({});
    }

    let mut current = target;
    for (i, segment) in path.iter().enumerate() {
        let is_last = i == path.len() - 1;

        match segment {
            PathSegment::Field(field) => {
                if !current.is_object() {
                    *current = serde_json::json!({});
                }
                let obj = current.as_object_mut().unwrap();
                if is_last {
                    obj.insert(field.clone(), data.clone());
                    return;
                }
                if !obj.contains_key(field) {
                    obj.insert(field.clone(), serde_json::json!({}));
                }
                current = obj.get_mut(field).unwrap();
            }
            PathSegment::Index(idx) => {
                if !current.is_array() {
                    *current = serde_json::json!([]);
                }
                let arr = current.as_array_mut().unwrap();
                while arr.len() <= *idx {
                    arr.push(serde_json::Value::Null);
                }
                if is_last {
                    arr[*idx] = data.clone();
                    return;
                }
                current = &mut arr[*idx];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_state_lifecycle() {
        let mut state = ExecutionState::new("exec-1".into(), "hash123".into());
        assert_eq!(state.phase, ExecutionPhase::Pending);

        state.transition_to(ExecutionPhase::Running);
        assert_eq!(state.phase, ExecutionPhase::Running);

        state.add_pending_defer("bio".into());
        state.add_pending_defer("stats".into());
        assert_eq!(state.pending_defers.len(), 2);

        state.resolve_defer("bio");
        assert_eq!(state.pending_defers.len(), 1);
        assert!(!state.pending_defers.contains(&"bio".to_string()));
    }

    #[test]
    fn test_checkpoint_creation_and_restore() {
        let mut state = ExecutionState::new("exec-2".into(), "hash456".into());
        state.partial_data = serde_json::json!({"user": {"name": "Alice"}});
        state.pending_defers = vec!["bio".into()];

        let checkpoint = state.create_checkpoint();
        assert!(checkpoint.id.starts_with("exec-2-"));

        // Modify state
        state.partial_data = serde_json::json!({"user": {"name": "Bob"}});
        state.pending_defers.clear();

        // Restore
        state.restore_from_checkpoint(&checkpoint.id);
        assert_eq!(
            state.partial_data,
            serde_json::json!({"user": {"name": "Alice"}})
        );
        assert_eq!(state.pending_defers.len(), 1);
    }

    #[test]
    fn test_merge_at_path() {
        let mut target = serde_json::json!({
            "user": {
                "name": "Alice"
            }
        });

        merge_at_path(
            &mut target,
            &[PathSegment::Field("user".into())],
            serde_json::json!({"name": "Alice", "bio": "Hello!"}),
        );

        assert_eq!(target["user"]["bio"], "Hello!");
    }

    #[test]
    fn test_stream_cursor() {
        let mut cursor =
            StreamCursor::new("posts".into(), vec![PathSegment::Field("posts".into())]);
        assert!(cursor.has_more());
        assert_eq!(cursor.items_delivered, 0);

        cursor.advance(10, Some("cursor123".into()));
        assert_eq!(cursor.items_delivered, 10);
        assert_eq!(cursor.cursor, Some("cursor123".into()));

        cursor.complete();
        assert!(!cursor.has_more());
    }

    #[test]
    fn test_binary_stream_state() {
        let mut stream = BinaryStreamState::new("stream-1".into(), "video/mp4".into())
            .with_total_size(1000)
            .with_chunk_size(100);

        assert_eq!(stream.progress(), Some(0.0));

        stream.advance(250);
        assert_eq!(stream.progress(), Some(25.0));
        assert!(!stream.is_complete());

        stream.advance(750);
        assert!(stream.is_complete());
    }

    #[test]
    fn test_resume_token() {
        let mut state = ExecutionState::new("exec-3".into(), "hash789".into());
        let token1 = state.generate_resume_token();
        let token2 = state.generate_resume_token();

        // Same state should produce same token
        assert_eq!(token1, token2);
    }
}
