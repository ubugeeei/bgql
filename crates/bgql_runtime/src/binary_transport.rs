//! Binary streaming transport protocol.
//!
//! This module implements the binary streaming protocol for
//! efficient media and file transfer over GraphQL.

use crate::state::{BinaryStreamPhase, BinaryStreamState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, RwLock};

/// Protocol version.
pub const PROTOCOL_VERSION: u8 = 1;

/// Default chunk size (64KB).
pub const DEFAULT_CHUNK_SIZE: u32 = 65536;

/// Maximum chunk size (1MB).
pub const MAX_CHUNK_SIZE: u32 = 1024 * 1024;

/// Content type for binary streams.
pub const CONTENT_TYPE_BINARY_STREAM: &str = "application/vnd.bgql.binary-stream";

/// Binary chunk flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkFlags(u8);

impl ChunkFlags {
    /// No special flags.
    pub const NONE: Self = Self(0);

    /// This is the final chunk.
    pub const FINAL: Self = Self(1 << 0);

    /// An error occurred.
    pub const ERROR: Self = Self(1 << 1);

    /// Contains metadata.
    pub const METADATA: Self = Self(1 << 2);

    /// Chunk is compressed.
    pub const COMPRESSED: Self = Self(1 << 3);

    /// Chunk is a keyframe (for video).
    pub const KEYFRAME: Self = Self(1 << 4);

    /// Creates new flags.
    pub fn new() -> Self {
        Self::NONE
    }

    /// Sets the final flag.
    pub fn with_final(mut self) -> Self {
        self.0 |= Self::FINAL.0;
        self
    }

    /// Sets the error flag.
    pub fn with_error(mut self) -> Self {
        self.0 |= Self::ERROR.0;
        self
    }

    /// Sets the metadata flag.
    pub fn with_metadata(mut self) -> Self {
        self.0 |= Self::METADATA.0;
        self
    }

    /// Sets the compressed flag.
    pub fn with_compressed(mut self) -> Self {
        self.0 |= Self::COMPRESSED.0;
        self
    }

    /// Sets the keyframe flag.
    pub fn with_keyframe(mut self) -> Self {
        self.0 |= Self::KEYFRAME.0;
        self
    }

    /// Checks if this is the final chunk.
    pub fn is_final(&self) -> bool {
        self.0 & Self::FINAL.0 != 0
    }

    /// Checks if this is an error chunk.
    pub fn is_error(&self) -> bool {
        self.0 & Self::ERROR.0 != 0
    }

    /// Checks if this contains metadata.
    pub fn is_metadata(&self) -> bool {
        self.0 & Self::METADATA.0 != 0
    }

    /// Checks if this is compressed.
    pub fn is_compressed(&self) -> bool {
        self.0 & Self::COMPRESSED.0 != 0
    }

    /// Checks if this is a keyframe.
    pub fn is_keyframe(&self) -> bool {
        self.0 & Self::KEYFRAME.0 != 0
    }

    /// Gets the raw value.
    pub fn as_u8(&self) -> u8 {
        self.0
    }

    /// Creates from raw value.
    pub fn from_u8(value: u8) -> Self {
        Self(value)
    }
}

impl Default for ChunkFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// A binary chunk in the stream.
///
/// Frame format:
/// ```text
/// [4 bytes: sequence number (big-endian)]
/// [4 bytes: payload length (big-endian)]
/// [1 byte: flags]
/// [payload bytes]
/// [4 bytes: CRC32 (big-endian)]
/// ```
#[derive(Debug, Clone)]
pub struct BinaryChunk {
    /// Sequence number.
    pub sequence: u32,

    /// Chunk flags.
    pub flags: ChunkFlags,

    /// Payload data.
    pub payload: Vec<u8>,

    /// CRC32 checksum.
    pub checksum: u32,
}

impl BinaryChunk {
    /// Creates a new chunk.
    pub fn new(sequence: u32, payload: Vec<u8>) -> Self {
        let checksum = crc32(&payload);
        Self {
            sequence,
            flags: ChunkFlags::NONE,
            payload,
            checksum,
        }
    }

    /// Creates a final chunk.
    pub fn final_chunk(sequence: u32, payload: Vec<u8>) -> Self {
        let checksum = crc32(&payload);
        Self {
            sequence,
            flags: ChunkFlags::FINAL,
            payload,
            checksum,
        }
    }

    /// Creates an error chunk.
    pub fn error_chunk(sequence: u32, message: &str) -> Self {
        let payload = message.as_bytes().to_vec();
        let checksum = crc32(&payload);
        Self {
            sequence,
            flags: ChunkFlags::ERROR,
            payload,
            checksum,
        }
    }

    /// Creates a metadata chunk.
    pub fn metadata_chunk(sequence: u32, metadata: &BinaryStreamMetadata) -> Self {
        let payload = serde_json::to_vec(metadata).unwrap_or_default();
        let checksum = crc32(&payload);
        Self {
            sequence,
            flags: ChunkFlags::METADATA,
            payload,
            checksum,
        }
    }

    /// Validates the checksum.
    pub fn validate(&self) -> bool {
        crc32(&self.payload) == self.checksum
    }

    /// Serializes the chunk to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(13 + self.payload.len());
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        buf.push(self.flags.as_u8());
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(&self.checksum.to_be_bytes());
        buf
    }

    /// Deserializes a chunk from bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 13 {
            return None;
        }

        let sequence = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let payload_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let flags = ChunkFlags::from_u8(data[8]);

        if data.len() < 13 + payload_len {
            return None;
        }

        let payload = data[9..9 + payload_len].to_vec();
        let checksum_start = 9 + payload_len;
        let checksum = u32::from_be_bytes([
            data[checksum_start],
            data[checksum_start + 1],
            data[checksum_start + 2],
            data[checksum_start + 3],
        ]);

        let chunk = Self {
            sequence,
            flags,
            payload,
            checksum,
        };

        if chunk.validate() {
            Some(chunk)
        } else {
            None
        }
    }

    /// Async write to a writer.
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.to_bytes();
        writer.write_all(&bytes).await
    }

    /// Async read from a reader.
    pub async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<Self> {
        let mut header = [0u8; 9];
        reader.read_exact(&mut header).await?;

        let sequence = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let payload_len = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;
        let flags = ChunkFlags::from_u8(header[8]);

        let mut payload = vec![0u8; payload_len];
        reader.read_exact(&mut payload).await?;

        let mut checksum_bytes = [0u8; 4];
        reader.read_exact(&mut checksum_bytes).await?;
        let checksum = u32::from_be_bytes(checksum_bytes);

        let chunk = Self {
            sequence,
            flags,
            payload,
            checksum,
        };

        if !chunk.validate() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Checksum validation failed",
            ));
        }

        Ok(chunk)
    }
}

/// Simple CRC32 implementation.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFF_u32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB88320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Metadata for a binary stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStreamMetadata {
    /// Stream ID.
    pub id: String,

    /// Content type (MIME).
    pub content_type: String,

    /// Total size in bytes (if known).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_size: Option<u64>,

    /// Chunk size.
    pub chunk_size: u32,

    /// Whether range requests are supported.
    pub supports_range: bool,

    /// Whether pause is supported.
    pub supports_pause: bool,

    /// Additional metadata.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl BinaryStreamMetadata {
    /// Creates new metadata.
    pub fn new(id: String, content_type: String) -> Self {
        Self {
            id,
            content_type,
            total_size: None,
            chunk_size: DEFAULT_CHUNK_SIZE,
            supports_range: true,
            supports_pause: true,
            extra: HashMap::new(),
        }
    }

    /// Sets total size.
    pub fn with_total_size(mut self, size: u64) -> Self {
        self.total_size = Some(size);
        self
    }

    /// Sets chunk size.
    pub fn with_chunk_size(mut self, size: u32) -> Self {
        self.chunk_size = size.min(MAX_CHUNK_SIZE);
        self
    }

    /// Adds extra metadata.
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}

/// Handle for controlling a binary stream.
#[derive(Debug, Clone)]
pub struct BinaryStreamHandle {
    /// Stream ID.
    pub id: String,

    /// Content type.
    pub content_type: String,

    /// Total size (if known).
    pub total_size: Option<u64>,

    /// Chunk size.
    pub chunk_size: u32,

    /// Whether range requests are supported.
    pub supports_range: bool,

    /// Whether pause is supported.
    pub supports_pause: bool,

    /// Internal state.
    state: Arc<RwLock<BinaryStreamState>>,

    /// Control channel.
    control_tx: mpsc::Sender<StreamControl>,
}

/// Stream control commands.
#[derive(Debug)]
pub enum StreamControl {
    /// Pause the stream.
    Pause,
    /// Resume the stream.
    Resume,
    /// Seek to offset.
    Seek(u64),
    /// Stop the stream.
    Stop,
}

impl BinaryStreamHandle {
    /// Creates a new stream handle.
    pub fn new(metadata: BinaryStreamMetadata) -> (Self, mpsc::Receiver<StreamControl>) {
        let (control_tx, control_rx) = mpsc::channel(16);
        let state = BinaryStreamState::new(metadata.id.clone(), metadata.content_type.clone())
            .with_chunk_size(metadata.chunk_size);

        let state = if let Some(size) = metadata.total_size {
            state.with_total_size(size)
        } else {
            state
        };

        let handle = Self {
            id: metadata.id,
            content_type: metadata.content_type,
            total_size: metadata.total_size,
            chunk_size: metadata.chunk_size,
            supports_range: metadata.supports_range,
            supports_pause: metadata.supports_pause,
            state: Arc::new(RwLock::new(state)),
            control_tx,
        };

        (handle, control_rx)
    }

    /// Pauses the stream.
    pub async fn pause(&self) -> Result<(), StreamError> {
        if !self.supports_pause {
            return Err(StreamError::NotSupported("pause".to_string()));
        }
        self.control_tx
            .send(StreamControl::Pause)
            .await
            .map_err(|_| StreamError::Closed)?;
        Ok(())
    }

    /// Resumes the stream.
    pub async fn resume(&self) -> Result<(), StreamError> {
        self.control_tx
            .send(StreamControl::Resume)
            .await
            .map_err(|_| StreamError::Closed)?;
        Ok(())
    }

    /// Seeks to an offset.
    pub async fn seek(&self, offset: u64) -> Result<(), StreamError> {
        if !self.supports_range {
            return Err(StreamError::NotSupported("seek".to_string()));
        }
        self.control_tx
            .send(StreamControl::Seek(offset))
            .await
            .map_err(|_| StreamError::Closed)?;
        Ok(())
    }

    /// Stops the stream.
    pub async fn stop(&self) -> Result<(), StreamError> {
        self.control_tx
            .send(StreamControl::Stop)
            .await
            .map_err(|_| StreamError::Closed)?;
        Ok(())
    }

    /// Gets current progress (0.0 - 1.0).
    pub async fn progress(&self) -> Option<f64> {
        let state = self.state.read().await;
        state.progress().map(|p| p / 100.0)
    }

    /// Gets current offset.
    pub async fn offset(&self) -> u64 {
        let state = self.state.read().await;
        state.offset
    }

    /// Gets bytes transferred.
    pub async fn bytes_transferred(&self) -> u64 {
        let state = self.state.read().await;
        state.bytes_transferred
    }

    /// Gets the stream phase.
    pub async fn phase(&self) -> BinaryStreamPhase {
        let state = self.state.read().await;
        state.phase
    }

    /// Updates internal state.
    #[allow(dead_code)]
    pub(crate) async fn update_state<F>(&self, f: F)
    where
        F: FnOnce(&mut BinaryStreamState),
    {
        let mut state = self.state.write().await;
        f(&mut state);
    }
}

/// Stream error.
#[derive(Debug, Clone)]
pub enum StreamError {
    /// Operation not supported.
    NotSupported(String),
    /// Stream is closed.
    Closed,
    /// I/O error.
    Io(String),
    /// Invalid data.
    InvalidData(String),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSupported(op) => write!(f, "Operation not supported: {}", op),
            Self::Closed => write!(f, "Stream is closed"),
            Self::Io(msg) => write!(f, "I/O error: {}", msg),
            Self::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl std::error::Error for StreamError {}

/// Binary protocol for encoding/decoding streams.
pub struct BinaryProtocol;

impl BinaryProtocol {
    /// Creates HTTP headers for a binary stream response.
    pub fn headers(metadata: &BinaryStreamMetadata) -> Vec<(String, String)> {
        let mut headers = vec![
            (
                "Content-Type".to_string(),
                CONTENT_TYPE_BINARY_STREAM.to_string(),
            ),
            ("X-BGQL-Stream-Id".to_string(), metadata.id.clone()),
            (
                "X-BGQL-Content-Type".to_string(),
                metadata.content_type.clone(),
            ),
            (
                "X-BGQL-Chunk-Size".to_string(),
                metadata.chunk_size.to_string(),
            ),
            ("Transfer-Encoding".to_string(), "chunked".to_string()),
        ];

        if let Some(size) = metadata.total_size {
            headers.push(("X-BGQL-Total-Size".to_string(), size.to_string()));
        }

        if metadata.supports_range {
            headers.push(("Accept-Ranges".to_string(), "bytes".to_string()));
        }

        headers
    }

    /// Parses headers to extract stream metadata.
    pub fn parse_headers(headers: &[(String, String)]) -> Option<BinaryStreamMetadata> {
        let mut id = None;
        let mut content_type = None;
        let mut total_size = None;
        let mut chunk_size = DEFAULT_CHUNK_SIZE;
        let mut supports_range = false;

        for (key, value) in headers {
            match key.as_str() {
                "X-BGQL-Stream-Id" | "x-bgql-stream-id" => id = Some(value.clone()),
                "X-BGQL-Content-Type" | "x-bgql-content-type" => content_type = Some(value.clone()),
                "X-BGQL-Total-Size" | "x-bgql-total-size" => {
                    total_size = value.parse().ok();
                }
                "X-BGQL-Chunk-Size" | "x-bgql-chunk-size" => {
                    chunk_size = value.parse().unwrap_or(DEFAULT_CHUNK_SIZE);
                }
                "Accept-Ranges" | "accept-ranges" if value == "bytes" => {
                    supports_range = true;
                }
                _ => {}
            }
        }

        let id = id?;
        let content_type = content_type?;

        let mut metadata = BinaryStreamMetadata::new(id, content_type).with_chunk_size(chunk_size);

        if let Some(size) = total_size {
            metadata = metadata.with_total_size(size);
        }

        metadata.supports_range = supports_range;

        Some(metadata)
    }

    /// Encodes a stream from a reader.
    pub async fn encode_stream<R, W>(
        reader: &mut R,
        writer: &mut W,
        chunk_size: u32,
        mut control_rx: mpsc::Receiver<StreamControl>,
    ) -> Result<u64, StreamError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut sequence = 0u32;
        let mut total_bytes = 0u64;
        let mut paused = false;

        loop {
            // Check for control messages
            match control_rx.try_recv() {
                Ok(StreamControl::Pause) => paused = true,
                Ok(StreamControl::Resume) => paused = false,
                Ok(StreamControl::Stop) => break,
                Ok(StreamControl::Seek(_)) => {
                    // Seek not supported in this basic implementation
                }
                Err(_) => {}
            }

            if paused {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }

            let mut buf = vec![0u8; chunk_size as usize];
            let n = reader
                .read(&mut buf)
                .await
                .map_err(|e| StreamError::Io(e.to_string()))?;

            if n == 0 {
                // End of stream
                let chunk = BinaryChunk::final_chunk(sequence, vec![]);
                chunk
                    .write_to(writer)
                    .await
                    .map_err(|e| StreamError::Io(e.to_string()))?;
                break;
            }

            buf.truncate(n);
            let chunk = BinaryChunk::new(sequence, buf);
            chunk
                .write_to(writer)
                .await
                .map_err(|e| StreamError::Io(e.to_string()))?;

            sequence += 1;
            total_bytes += n as u64;
        }

        writer
            .flush()
            .await
            .map_err(|e| StreamError::Io(e.to_string()))?;

        Ok(total_bytes)
    }

    /// Decodes a stream from a reader.
    pub async fn decode_stream<R, W>(reader: &mut R, writer: &mut W) -> Result<u64, StreamError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut total_bytes = 0u64;

        loop {
            let chunk = BinaryChunk::read_from(reader)
                .await
                .map_err(|e| StreamError::Io(e.to_string()))?;

            if chunk.flags.is_error() {
                let msg = String::from_utf8_lossy(&chunk.payload).to_string();
                return Err(StreamError::InvalidData(msg));
            }

            if !chunk.payload.is_empty() {
                writer
                    .write_all(&chunk.payload)
                    .await
                    .map_err(|e| StreamError::Io(e.to_string()))?;
                total_bytes += chunk.payload.len() as u64;
            }

            if chunk.flags.is_final() {
                break;
            }
        }

        writer
            .flush()
            .await
            .map_err(|e| StreamError::Io(e.to_string()))?;

        Ok(total_bytes)
    }
}

/// Progress tracker for binary streams.
pub struct ProgressTracker {
    /// Total size (if known).
    total_size: Option<u64>,

    /// Bytes transferred.
    transferred: AtomicU64,

    /// Progress callback.
    callback: Option<Box<dyn Fn(f64) + Send + Sync>>,
}

impl std::fmt::Debug for ProgressTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressTracker")
            .field("total_size", &self.total_size)
            .field("transferred", &self.transferred)
            .field("has_callback", &self.callback.is_some())
            .finish()
    }
}

impl ProgressTracker {
    /// Creates a new progress tracker.
    pub fn new(total_size: Option<u64>) -> Self {
        Self {
            total_size,
            transferred: AtomicU64::new(0),
            callback: None,
        }
    }

    /// Sets a progress callback.
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(f64) + Send + Sync + 'static,
    {
        self.callback = Some(Box::new(callback));
        self
    }

    /// Updates progress.
    pub fn update(&self, bytes: u64) {
        let total = self.transferred.fetch_add(bytes, Ordering::Relaxed) + bytes;
        if let (Some(size), Some(callback)) = (self.total_size, &self.callback) {
            let progress = total as f64 / size as f64;
            callback(progress);
        }
    }

    /// Gets current progress (0.0 - 1.0).
    pub fn progress(&self) -> Option<f64> {
        self.total_size
            .map(|size| self.transferred.load(Ordering::Relaxed) as f64 / size as f64)
    }

    /// Gets bytes transferred.
    pub fn bytes_transferred(&self) -> u64 {
        self.transferred.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_flags() {
        let flags = ChunkFlags::new().with_final().with_keyframe();
        assert!(flags.is_final());
        assert!(flags.is_keyframe());
        assert!(!flags.is_error());
    }

    #[test]
    fn test_chunk_serialization() {
        let chunk = BinaryChunk::new(42, b"Hello, World!".to_vec());
        let bytes = chunk.to_bytes();
        let restored = BinaryChunk::from_bytes(&bytes).unwrap();

        assert_eq!(restored.sequence, 42);
        assert_eq!(restored.payload, b"Hello, World!");
        assert!(restored.validate());
    }

    #[test]
    fn test_crc32() {
        // Known CRC32 value for "123456789"
        let data = b"123456789";
        let crc = crc32(data);
        assert_eq!(crc, 0xCBF43926);
    }

    #[test]
    fn test_metadata() {
        let metadata = BinaryStreamMetadata::new("stream-1".into(), "video/mp4".into())
            .with_total_size(1024 * 1024)
            .with_chunk_size(65536)
            .with_extra("duration", serde_json::json!(120.5));

        assert_eq!(metadata.id, "stream-1");
        assert_eq!(metadata.total_size, Some(1024 * 1024));
        assert!(metadata.extra.contains_key("duration"));
    }

    #[test]
    fn test_headers() {
        let metadata =
            BinaryStreamMetadata::new("stream-1".into(), "video/mp4".into()).with_total_size(1000);

        let headers = BinaryProtocol::headers(&metadata);
        let header_map: HashMap<_, _> = headers.into_iter().collect();

        assert_eq!(
            header_map.get("Content-Type"),
            Some(&CONTENT_TYPE_BINARY_STREAM.to_string())
        );
        assert_eq!(
            header_map.get("X-BGQL-Stream-Id"),
            Some(&"stream-1".to_string())
        );
        assert_eq!(
            header_map.get("X-BGQL-Total-Size"),
            Some(&"1000".to_string())
        );
    }

    #[test]
    fn test_parse_headers() {
        let headers = vec![
            ("X-BGQL-Stream-Id".to_string(), "stream-2".to_string()),
            ("X-BGQL-Content-Type".to_string(), "audio/mp3".to_string()),
            ("X-BGQL-Total-Size".to_string(), "5000".to_string()),
            ("Accept-Ranges".to_string(), "bytes".to_string()),
        ];

        let metadata = BinaryProtocol::parse_headers(&headers).unwrap();
        assert_eq!(metadata.id, "stream-2");
        assert_eq!(metadata.content_type, "audio/mp3");
        assert_eq!(metadata.total_size, Some(5000));
        assert!(metadata.supports_range);
    }

    #[test]
    fn test_progress_tracker() {
        let tracker = ProgressTracker::new(Some(100));

        tracker.update(25);
        assert_eq!(tracker.progress(), Some(0.25));

        tracker.update(25);
        assert_eq!(tracker.progress(), Some(0.5));

        assert_eq!(tracker.bytes_transferred(), 50);
    }

    #[tokio::test]
    async fn test_stream_handle() {
        let metadata =
            BinaryStreamMetadata::new("test-stream".into(), "application/octet-stream".into());
        let (handle, mut control_rx) = BinaryStreamHandle::new(metadata);

        assert_eq!(handle.id, "test-stream");
        assert!(handle.supports_pause);

        // Test pause
        handle.pause().await.unwrap();
        let cmd = control_rx.recv().await.unwrap();
        matches!(cmd, StreamControl::Pause);
    }
}
