//! HTTP Live Streaming (HLS) support.
//!
//! This module provides HLS manifest generation and segment streaming
//! capabilities for adaptive bitrate streaming over GraphQL.

use crate::state::BinaryStreamPhase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// HLS protocol version supported.
pub const HLS_VERSION: u8 = 7;

/// Default segment duration in seconds.
pub const DEFAULT_SEGMENT_DURATION: f64 = 6.0;

/// Content type for HLS master playlist.
pub const CONTENT_TYPE_HLS_PLAYLIST: &str = "application/vnd.apple.mpegurl";

/// Content type for HLS segments.
pub const CONTENT_TYPE_HLS_SEGMENT: &str = "video/mp2t";

/// Content type for fMP4 segments.
pub const CONTENT_TYPE_FMP4_SEGMENT: &str = "video/mp4";

/// HLS playlist type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaylistType {
    /// Live playlist (can grow).
    Live,
    /// Event playlist (can grow, no removal).
    Event,
    /// VOD playlist (complete, static).
    #[default]
    Vod,
}

/// Media type for HLS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaType {
    Audio,
    Video,
    Subtitles,
    ClosedCaptions,
}

/// Video codec information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCodec {
    /// Codec identifier (e.g., "avc1.64001f").
    pub codec: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Frame rate.
    pub frame_rate: f64,
    /// Bitrate in bits per second.
    pub bitrate: u32,
}

impl VideoCodec {
    /// Creates a new video codec.
    pub fn new(codec: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            codec: codec.into(),
            width,
            height,
            frame_rate: 30.0,
            bitrate: 0,
        }
    }

    /// Common H.264 profiles.
    pub fn h264_baseline(width: u32, height: u32, bitrate: u32) -> Self {
        Self {
            codec: "avc1.42E01E".to_string(), // Baseline Profile Level 3.0
            width,
            height,
            frame_rate: 30.0,
            bitrate,
        }
    }

    pub fn h264_main(width: u32, height: u32, bitrate: u32) -> Self {
        Self {
            codec: "avc1.4D401F".to_string(), // Main Profile Level 3.1
            width,
            height,
            frame_rate: 30.0,
            bitrate,
        }
    }

    pub fn h264_high(width: u32, height: u32, bitrate: u32) -> Self {
        Self {
            codec: "avc1.64001F".to_string(), // High Profile Level 3.1
            width,
            height,
            frame_rate: 30.0,
            bitrate,
        }
    }

    /// Sets frame rate.
    pub fn with_frame_rate(mut self, fps: f64) -> Self {
        self.frame_rate = fps;
        self
    }

    /// Sets bitrate.
    pub fn with_bitrate(mut self, bitrate: u32) -> Self {
        self.bitrate = bitrate;
        self
    }

    /// Returns resolution string (e.g., "1920x1080").
    pub fn resolution(&self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

/// Audio codec information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCodec {
    /// Codec identifier (e.g., "mp4a.40.2").
    pub codec: String,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u8,
    /// Bitrate in bits per second.
    pub bitrate: u32,
    /// Language code (ISO 639-1).
    pub language: Option<String>,
}

impl AudioCodec {
    /// AAC-LC codec.
    pub fn aac_lc(sample_rate: u32, channels: u8, bitrate: u32) -> Self {
        Self {
            codec: "mp4a.40.2".to_string(),
            sample_rate,
            channels,
            bitrate,
            language: None,
        }
    }

    /// AAC-HE v2 codec.
    pub fn aac_he_v2(sample_rate: u32, channels: u8, bitrate: u32) -> Self {
        Self {
            codec: "mp4a.40.29".to_string(),
            sample_rate,
            channels,
            bitrate,
            language: None,
        }
    }

    /// Sets language.
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }
}

/// HLS variant stream (quality level).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsVariant {
    /// Unique identifier.
    pub id: String,
    /// Video codec.
    pub video: Option<VideoCodec>,
    /// Audio codec.
    pub audio: Option<AudioCodec>,
    /// Combined bandwidth.
    pub bandwidth: u32,
    /// Average bandwidth.
    pub average_bandwidth: Option<u32>,
    /// URI for the variant playlist.
    pub uri: String,
}

impl HlsVariant {
    /// Creates a video-only variant.
    pub fn video(id: impl Into<String>, video: VideoCodec, uri: impl Into<String>) -> Self {
        let bandwidth = video.bitrate;
        Self {
            id: id.into(),
            video: Some(video),
            audio: None,
            bandwidth,
            average_bandwidth: None,
            uri: uri.into(),
        }
    }

    /// Creates a video+audio variant.
    pub fn video_audio(
        id: impl Into<String>,
        video: VideoCodec,
        audio: AudioCodec,
        uri: impl Into<String>,
    ) -> Self {
        let bandwidth = video.bitrate + audio.bitrate;
        Self {
            id: id.into(),
            video: Some(video),
            audio: Some(audio),
            bandwidth,
            average_bandwidth: None,
            uri: uri.into(),
        }
    }

    /// Generates the #EXT-X-STREAM-INF line.
    pub fn stream_inf(&self) -> String {
        let mut attrs = vec![format!("BANDWIDTH={}", self.bandwidth)];

        if let Some(avg) = self.average_bandwidth {
            attrs.push(format!("AVERAGE-BANDWIDTH={}", avg));
        }

        let mut codecs = Vec::new();
        if let Some(v) = &self.video {
            codecs.push(v.codec.clone());
            attrs.push(format!("RESOLUTION={}", v.resolution()));
            attrs.push(format!("FRAME-RATE={:.3}", v.frame_rate));
        }
        if let Some(a) = &self.audio {
            codecs.push(a.codec.clone());
        }

        if !codecs.is_empty() {
            attrs.push(format!("CODECS=\"{}\"", codecs.join(",")));
        }

        format!("#EXT-X-STREAM-INF:{}", attrs.join(","))
    }
}

/// HLS segment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsSegment {
    /// Segment sequence number.
    pub sequence: u64,
    /// Duration in seconds.
    pub duration: f64,
    /// URI for the segment.
    pub uri: String,
    /// Byte range (offset, length) if applicable.
    pub byte_range: Option<(u64, u64)>,
    /// Whether this is a discontinuity.
    pub discontinuity: bool,
    /// Program date time.
    pub program_date_time: Option<String>,
    /// Encryption key info.
    pub key: Option<SegmentKey>,
}

impl HlsSegment {
    /// Creates a new segment.
    pub fn new(sequence: u64, duration: f64, uri: impl Into<String>) -> Self {
        Self {
            sequence,
            duration,
            uri: uri.into(),
            byte_range: None,
            discontinuity: false,
            program_date_time: None,
            key: None,
        }
    }

    /// Sets byte range for byte-range requests.
    pub fn with_byte_range(mut self, offset: u64, length: u64) -> Self {
        self.byte_range = Some((offset, length));
        self
    }

    /// Marks as discontinuity.
    pub fn with_discontinuity(mut self) -> Self {
        self.discontinuity = true;
        self
    }

    /// Generates segment lines for playlist.
    pub fn to_playlist_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        if self.discontinuity {
            lines.push("#EXT-X-DISCONTINUITY".to_string());
        }

        if let Some(ref key) = self.key {
            lines.push(key.to_playlist_line());
        }

        if let Some(ref pdt) = self.program_date_time {
            lines.push(format!("#EXT-X-PROGRAM-DATE-TIME:{}", pdt));
        }

        lines.push(format!("#EXTINF:{:.6},", self.duration));

        if let Some((offset, length)) = self.byte_range {
            lines.push(format!("#EXT-X-BYTERANGE:{}@{}", length, offset));
        }

        lines.push(self.uri.clone());

        lines
    }
}

/// Segment encryption key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentKey {
    /// Encryption method.
    pub method: EncryptionMethod,
    /// Key URI.
    pub uri: Option<String>,
    /// Initialization vector.
    pub iv: Option<String>,
    /// Key format.
    pub key_format: Option<String>,
}

/// Encryption method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionMethod {
    None,
    Aes128,
    SampleAes,
    SampleAesCtr,
}

impl SegmentKey {
    /// No encryption.
    pub fn none() -> Self {
        Self {
            method: EncryptionMethod::None,
            uri: None,
            iv: None,
            key_format: None,
        }
    }

    /// AES-128 encryption.
    pub fn aes128(uri: impl Into<String>) -> Self {
        Self {
            method: EncryptionMethod::Aes128,
            uri: Some(uri.into()),
            iv: None,
            key_format: None,
        }
    }

    /// Sets IV.
    pub fn with_iv(mut self, iv: impl Into<String>) -> Self {
        self.iv = Some(iv.into());
        self
    }

    fn to_playlist_line(&self) -> String {
        let method = match self.method {
            EncryptionMethod::None => return "#EXT-X-KEY:METHOD=NONE".to_string(),
            EncryptionMethod::Aes128 => "AES-128",
            EncryptionMethod::SampleAes => "SAMPLE-AES",
            EncryptionMethod::SampleAesCtr => "SAMPLE-AES-CTR",
        };

        let mut attrs = vec![format!("METHOD={}", method)];

        if let Some(ref uri) = self.uri {
            attrs.push(format!("URI=\"{}\"", uri));
        }
        if let Some(ref iv) = self.iv {
            attrs.push(format!("IV={}", iv));
        }
        if let Some(ref kf) = self.key_format {
            attrs.push(format!("KEYFORMAT=\"{}\"", kf));
        }

        format!("#EXT-X-KEY:{}", attrs.join(","))
    }
}

/// HLS media playlist (variant playlist).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsPlaylist {
    /// Playlist version.
    pub version: u8,
    /// Target segment duration.
    pub target_duration: u32,
    /// Media sequence number.
    pub media_sequence: u64,
    /// Discontinuity sequence.
    pub discontinuity_sequence: u64,
    /// Playlist type.
    pub playlist_type: Option<PlaylistType>,
    /// Segments.
    pub segments: Vec<HlsSegment>,
    /// Whether the playlist is complete.
    pub end_list: bool,
    /// Independent segments flag.
    pub independent_segments: bool,
}

impl Default for HlsPlaylist {
    fn default() -> Self {
        Self {
            version: HLS_VERSION,
            target_duration: DEFAULT_SEGMENT_DURATION.ceil() as u32,
            media_sequence: 0,
            discontinuity_sequence: 0,
            playlist_type: None,
            segments: Vec::new(),
            end_list: false,
            independent_segments: true,
        }
    }
}

impl HlsPlaylist {
    /// Creates a new playlist.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets target duration.
    pub fn with_target_duration(mut self, duration: u32) -> Self {
        self.target_duration = duration;
        self
    }

    /// Sets playlist type.
    pub fn with_type(mut self, playlist_type: PlaylistType) -> Self {
        self.playlist_type = Some(playlist_type);
        self
    }

    /// Adds a segment.
    pub fn add_segment(&mut self, segment: HlsSegment) {
        self.segments.push(segment);
    }

    /// Marks playlist as complete.
    pub fn end(&mut self) {
        self.end_list = true;
    }

    /// Generates playlist content as string.
    pub fn render(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for HlsPlaylist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "#EXTM3U")?;
        writeln!(f, "#EXT-X-VERSION:{}", self.version)?;
        writeln!(f, "#EXT-X-TARGETDURATION:{}", self.target_duration)?;
        writeln!(f, "#EXT-X-MEDIA-SEQUENCE:{}", self.media_sequence)?;

        if self.discontinuity_sequence > 0 {
            writeln!(
                f,
                "#EXT-X-DISCONTINUITY-SEQUENCE:{}",
                self.discontinuity_sequence
            )?;
        }

        if let Some(pt) = &self.playlist_type {
            // Live playlists don't include EXT-X-PLAYLIST-TYPE
            if *pt != PlaylistType::Live {
                let type_str = match pt {
                    PlaylistType::Live => unreachable!(),
                    PlaylistType::Event => "EVENT",
                    PlaylistType::Vod => "VOD",
                };
                writeln!(f, "#EXT-X-PLAYLIST-TYPE:{}", type_str)?;
            }
        }

        if self.independent_segments {
            writeln!(f, "#EXT-X-INDEPENDENT-SEGMENTS")?;
        }

        for segment in &self.segments {
            for line in segment.to_playlist_lines() {
                writeln!(f, "{}", line)?;
            }
        }

        if self.end_list {
            writeln!(f, "#EXT-X-ENDLIST")?;
        }

        Ok(())
    }
}

impl HlsPlaylist {
    /// Parses playlist from string.
    pub fn parse(content: &str) -> Option<Self> {
        let mut playlist = Self::new();
        let mut current_segment_duration = 0.0;
        let mut sequence = 0u64;

        for line in content.lines() {
            let line = line.trim();

            if let Some(v) = line.strip_prefix("#EXT-X-VERSION:") {
                playlist.version = v.parse().unwrap_or(HLS_VERSION);
            } else if let Some(v) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
                playlist.target_duration = v.parse().unwrap_or(6);
            } else if let Some(v) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
                playlist.media_sequence = v.parse().unwrap_or(0);
                sequence = playlist.media_sequence;
            } else if let Some(v) = line.strip_prefix("#EXT-X-PLAYLIST-TYPE:") {
                playlist.playlist_type = match v {
                    "VOD" => Some(PlaylistType::Vod),
                    "EVENT" => Some(PlaylistType::Event),
                    _ => None,
                };
            } else if let Some(v) = line.strip_prefix("#EXTINF:") {
                let duration_str = v.trim_end_matches(',');
                current_segment_duration = duration_str.parse().unwrap_or(0.0);
            } else if line == "#EXT-X-ENDLIST" {
                playlist.end_list = true;
            } else if !line.starts_with('#') && !line.is_empty() {
                playlist.add_segment(HlsSegment::new(sequence, current_segment_duration, line));
                sequence += 1;
            }
        }

        Some(playlist)
    }

    /// Returns total duration.
    pub fn total_duration(&self) -> f64 {
        self.segments.iter().map(|s| s.duration).sum()
    }

    /// Returns segment count.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
}

/// HLS master playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsManifest {
    /// Playlist version.
    pub version: u8,
    /// Variants (quality levels).
    pub variants: Vec<HlsVariant>,
    /// Independent segments flag.
    pub independent_segments: bool,
    /// Session data.
    pub session_data: HashMap<String, String>,
}

impl Default for HlsManifest {
    fn default() -> Self {
        Self {
            version: HLS_VERSION,
            variants: Vec::new(),
            independent_segments: true,
            session_data: HashMap::new(),
        }
    }
}

impl HlsManifest {
    /// Creates a new manifest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a variant.
    pub fn add_variant(&mut self, variant: HlsVariant) {
        self.variants.push(variant);
    }

    /// Adds session data.
    pub fn add_session_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.session_data.insert(key.into(), value.into());
    }

    /// Generates manifest content as string.
    pub fn render(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for HlsManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "#EXTM3U")?;
        writeln!(f, "#EXT-X-VERSION:{}", self.version)?;

        if self.independent_segments {
            writeln!(f, "#EXT-X-INDEPENDENT-SEGMENTS")?;
        }

        for (key, value) in &self.session_data {
            writeln!(
                f,
                "#EXT-X-SESSION-DATA:DATA-ID=\"{}\",VALUE=\"{}\"",
                key, value
            )?;
        }

        // Sort variants by bandwidth (ascending) for adaptive streaming
        let mut variants = self.variants.clone();
        variants.sort_by(|a, b| a.bandwidth.cmp(&b.bandwidth));

        for variant in &variants {
            writeln!(f, "{}", variant.stream_inf())?;
            writeln!(f, "{}", variant.uri)?;
        }

        Ok(())
    }
}

impl HlsManifest {
    /// Creates a simple manifest with standard quality levels.
    pub fn standard_qualities(base_uri: &str, source_video: &VideoCodec) -> Self {
        let mut manifest = Self::new();

        // Define standard quality levels based on source resolution
        let qualities = [
            (1920, 1080, 5000000, "1080p"),
            (1280, 720, 2800000, "720p"),
            (854, 480, 1400000, "480p"),
            (640, 360, 800000, "360p"),
            (426, 240, 400000, "240p"),
        ];

        for (width, height, bitrate, label) in qualities {
            if width <= source_video.width && height <= source_video.height {
                let video = VideoCodec::h264_main(width, height, bitrate)
                    .with_frame_rate(source_video.frame_rate.min(30.0));

                let variant = HlsVariant::video(
                    label,
                    video,
                    format!("{}/{}/playlist.m3u8", base_uri, label),
                );

                manifest.add_variant(variant);
            }
        }

        manifest
    }
}

/// HLS stream generator for live/VOD content.
pub struct HlsStreamGenerator {
    /// Stream ID.
    pub id: String,
    /// Target segment duration.
    pub segment_duration: f64,
    /// Current playlist.
    playlist: Arc<RwLock<HlsPlaylist>>,
    /// Current sequence number.
    sequence: Arc<std::sync::atomic::AtomicU64>,
    /// Segment sender.
    segment_tx: mpsc::Sender<GeneratedSegment>,
    /// Segment receiver.
    segment_rx: Option<mpsc::Receiver<GeneratedSegment>>,
    /// State.
    state: Arc<RwLock<StreamGeneratorState>>,
}

/// Generated segment with data.
#[derive(Debug)]
pub struct GeneratedSegment {
    /// Segment info.
    pub segment: HlsSegment,
    /// Segment data.
    pub data: Vec<u8>,
}

/// Stream generator state.
#[derive(Debug, Clone)]
struct StreamGeneratorState {
    phase: BinaryStreamPhase,
    total_duration: f64,
    segments_generated: u64,
}

impl HlsStreamGenerator {
    /// Creates a new stream generator.
    pub fn new(id: impl Into<String>, segment_duration: f64) -> Self {
        let (segment_tx, segment_rx) = mpsc::channel(16);

        Self {
            id: id.into(),
            segment_duration,
            playlist: Arc::new(RwLock::new(
                HlsPlaylist::new()
                    .with_target_duration(segment_duration.ceil() as u32)
                    .with_type(PlaylistType::Vod),
            )),
            sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            segment_tx,
            segment_rx: Some(segment_rx),
            state: Arc::new(RwLock::new(StreamGeneratorState {
                phase: BinaryStreamPhase::Pending,
                total_duration: 0.0,
                segments_generated: 0,
            })),
        }
    }

    /// Creates for live streaming.
    pub fn live(id: impl Into<String>, segment_duration: f64) -> Self {
        let gen = Self::new(id, segment_duration);
        let playlist = gen.playlist.clone();
        tokio::spawn(async move {
            let mut playlist = playlist.write().await;
            playlist.playlist_type = Some(PlaylistType::Live);
        });
        gen
    }

    /// Takes the segment receiver.
    pub fn take_receiver(&mut self) -> Option<mpsc::Receiver<GeneratedSegment>> {
        self.segment_rx.take()
    }

    /// Adds a segment.
    pub async fn add_segment(&self, duration: f64, data: Vec<u8>) -> HlsSegment {
        let seq = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let segment = HlsSegment::new(seq, duration, format!("segment_{}.ts", seq));

        {
            let mut playlist = self.playlist.write().await;
            playlist.add_segment(segment.clone());
        }

        {
            let mut state = self.state.write().await;
            state.total_duration += duration;
            state.segments_generated += 1;
        }

        let _ = self
            .segment_tx
            .send(GeneratedSegment {
                segment: segment.clone(),
                data,
            })
            .await;

        segment
    }

    /// Finalizes the stream.
    pub async fn finalize(&self) {
        {
            let mut playlist = self.playlist.write().await;
            playlist.end();
        }
        {
            let mut state = self.state.write().await;
            state.phase = BinaryStreamPhase::Completed;
        }
    }

    /// Gets current playlist.
    pub async fn playlist(&self) -> HlsPlaylist {
        self.playlist.read().await.clone()
    }

    /// Gets playlist as string.
    pub async fn playlist_string(&self) -> String {
        self.playlist.read().await.to_string()
    }

    /// Gets total duration.
    pub async fn total_duration(&self) -> f64 {
        self.state.read().await.total_duration
    }

    /// Gets segment count.
    pub async fn segment_count(&self) -> u64 {
        self.state.read().await.segments_generated
    }
}

/// HLS segment fetcher for client-side consumption.
pub struct HlsSegmentFetcher {
    /// Base URL for segments.
    base_url: String,
    /// Current playlist.
    playlist: HlsPlaylist,
    /// Current segment index.
    current_index: usize,
}

impl HlsSegmentFetcher {
    /// Creates a new fetcher.
    pub fn new(base_url: impl Into<String>, playlist: HlsPlaylist) -> Self {
        Self {
            base_url: base_url.into(),
            playlist,
            current_index: 0,
        }
    }

    /// Gets the next segment URL.
    pub fn next_segment_url(&mut self) -> Option<String> {
        if self.current_index >= self.playlist.segments.len() {
            return None;
        }

        let segment = &self.playlist.segments[self.current_index];
        self.current_index += 1;

        Some(format!("{}/{}", self.base_url, segment.uri))
    }

    /// Gets segment URL by index.
    pub fn segment_url(&self, index: usize) -> Option<String> {
        self.playlist
            .segments
            .get(index)
            .map(|s| format!("{}/{}", self.base_url, s.uri))
    }

    /// Resets to beginning.
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Seeks to a time position.
    pub fn seek_to_time(&mut self, time: f64) {
        let mut accumulated = 0.0;
        for (i, segment) in self.playlist.segments.iter().enumerate() {
            if accumulated + segment.duration > time {
                self.current_index = i;
                return;
            }
            accumulated += segment.duration;
        }
        self.current_index = self.playlist.segments.len();
    }

    /// Gets remaining segments count.
    pub fn remaining(&self) -> usize {
        self.playlist
            .segments
            .len()
            .saturating_sub(self.current_index)
    }

    /// Updates the playlist (for live streams).
    pub fn update_playlist(&mut self, playlist: HlsPlaylist) {
        self.playlist = playlist;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_codec() {
        let codec = VideoCodec::h264_high(1920, 1080, 5000000);
        assert_eq!(codec.resolution(), "1920x1080");
        assert_eq!(codec.codec, "avc1.64001F");
    }

    #[test]
    fn test_hls_segment() {
        let segment = HlsSegment::new(0, 6.0, "segment_0.ts");
        let lines = segment.to_playlist_lines();
        assert!(lines.iter().any(|l| l.contains("EXTINF:6.0")));
        assert!(lines.iter().any(|l| l == "segment_0.ts"));
    }

    #[test]
    fn test_hls_playlist_generation() {
        let mut playlist = HlsPlaylist::new()
            .with_target_duration(6)
            .with_type(PlaylistType::Vod);

        playlist.add_segment(HlsSegment::new(0, 6.0, "segment_0.ts"));
        playlist.add_segment(HlsSegment::new(1, 6.0, "segment_1.ts"));
        playlist.add_segment(HlsSegment::new(2, 4.5, "segment_2.ts"));
        playlist.end();

        let content = playlist.to_string();
        assert!(content.contains("#EXTM3U"));
        assert!(content.contains("#EXT-X-VERSION:7"));
        assert!(content.contains("#EXT-X-TARGETDURATION:6"));
        assert!(content.contains("#EXT-X-PLAYLIST-TYPE:VOD"));
        assert!(content.contains("#EXT-X-ENDLIST"));
        assert!(content.contains("segment_0.ts"));
        assert!(content.contains("segment_1.ts"));
        assert!(content.contains("segment_2.ts"));
    }

    #[test]
    fn test_hls_manifest_generation() {
        let mut manifest = HlsManifest::new();

        manifest.add_variant(HlsVariant::video(
            "1080p",
            VideoCodec::h264_high(1920, 1080, 5000000),
            "1080p/playlist.m3u8",
        ));

        manifest.add_variant(HlsVariant::video(
            "720p",
            VideoCodec::h264_main(1280, 720, 2800000),
            "720p/playlist.m3u8",
        ));

        let content = manifest.to_string();
        assert!(content.contains("#EXTM3U"));
        assert!(content.contains("#EXT-X-STREAM-INF"));
        assert!(content.contains("BANDWIDTH=5000000"));
        assert!(content.contains("1080p/playlist.m3u8"));
    }

    #[test]
    fn test_playlist_parse() {
        let content = r#"#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:0
#EXT-X-PLAYLIST-TYPE:VOD
#EXTINF:6.000000,
segment_0.ts
#EXTINF:6.000000,
segment_1.ts
#EXT-X-ENDLIST
"#;

        let playlist = HlsPlaylist::parse(content).unwrap();
        assert_eq!(playlist.version, 7);
        assert_eq!(playlist.target_duration, 6);
        assert_eq!(playlist.segments.len(), 2);
        assert!(playlist.end_list);
        assert_eq!(playlist.playlist_type, Some(PlaylistType::Vod));
    }

    #[test]
    fn test_segment_fetcher() {
        let mut playlist = HlsPlaylist::new();
        playlist.add_segment(HlsSegment::new(0, 6.0, "segment_0.ts"));
        playlist.add_segment(HlsSegment::new(1, 6.0, "segment_1.ts"));
        playlist.add_segment(HlsSegment::new(2, 6.0, "segment_2.ts"));

        let mut fetcher = HlsSegmentFetcher::new("https://example.com/stream", playlist);

        assert_eq!(
            fetcher.next_segment_url(),
            Some("https://example.com/stream/segment_0.ts".to_string())
        );
        assert_eq!(
            fetcher.next_segment_url(),
            Some("https://example.com/stream/segment_1.ts".to_string())
        );

        fetcher.seek_to_time(0.0);
        assert_eq!(
            fetcher.next_segment_url(),
            Some("https://example.com/stream/segment_0.ts".to_string())
        );
    }

    #[tokio::test]
    async fn test_stream_generator() {
        let mut gen = HlsStreamGenerator::new("test-stream", 6.0);
        let _rx = gen.take_receiver().unwrap();

        gen.add_segment(6.0, vec![0u8; 100]).await;
        gen.add_segment(6.0, vec![0u8; 100]).await;
        gen.finalize().await;

        let playlist = gen.playlist().await;
        assert_eq!(playlist.segments.len(), 2);
        assert!(playlist.end_list);
    }

    #[test]
    fn test_standard_qualities() {
        let source = VideoCodec::h264_high(1920, 1080, 10000000).with_frame_rate(60.0);

        let manifest = HlsManifest::standard_qualities("/video", &source);

        // Should have multiple quality levels
        assert!(!manifest.variants.is_empty());

        // All variants should have frame_rate capped at 30
        for variant in &manifest.variants {
            if let Some(v) = &variant.video {
                assert!(v.frame_rate <= 30.0);
            }
        }
    }
}
