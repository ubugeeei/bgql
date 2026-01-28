//! Built-in directives for BGQL streaming-first architecture.
//!
//! This module defines the custom directives for:
//! - Server Fragments (@server, @boundary, @island)
//! - Scheduling (@priority, @resources)
//! - Binary streaming (@binary)
//! - Hydration (@hydrate)
//! - Resumable queries (@resumable)

use crate::resource::ResourceLevel;
use crate::schema::{DirectiveDefinition, DirectiveLocation, InputFieldDef, TypeRef};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

// =============================================================================
// @server directive
// =============================================================================

/// Cache strategy for server fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CacheStrategy {
    /// No caching.
    #[default]
    None,
    /// Cache per request.
    Request,
    /// Cache per user.
    User,
    /// Global cache.
    Global,
}

/// Arguments for @server directive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDirective {
    /// Whether to isolate execution to server only.
    #[serde(default = "default_true")]
    pub isolate: bool,

    /// Cache strategy.
    #[serde(default)]
    pub cache: CacheStrategy,

    /// Whether the fragment can be prerendered.
    #[serde(default)]
    pub prerender: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ServerDirective {
    fn default() -> Self {
        Self {
            isolate: true,
            cache: CacheStrategy::None,
            prerender: false,
        }
    }
}

impl ServerDirective {
    /// Creates a new @server directive with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables isolation.
    pub fn with_isolate(mut self, isolate: bool) -> Self {
        self.isolate = isolate;
        self
    }

    /// Sets cache strategy.
    pub fn with_cache(mut self, cache: CacheStrategy) -> Self {
        self.cache = cache;
        self
    }

    /// Enables prerendering.
    pub fn with_prerender(mut self, prerender: bool) -> Self {
        self.prerender = prerender;
        self
    }
}

// =============================================================================
// @boundary directive
// =============================================================================

/// Serialization strategy for boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SerializeStrategy {
    /// Standard JSON serialization.
    #[default]
    Json,
    /// Binary serialization.
    Binary,
    /// Only serialize as ID reference (fetch later).
    Reference,
    /// Never serialize (server-only data).
    Never,
}

/// Arguments for @boundary directive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoundaryDirective {
    /// Mark as server-only field/type.
    #[serde(default)]
    pub server: bool,

    /// Mark as client-only field/type.
    #[serde(default)]
    pub client: bool,

    /// Serialization strategy.
    #[serde(default)]
    pub serialize: SerializeStrategy,
}

impl BoundaryDirective {
    /// Creates a server-only boundary.
    pub fn server_only() -> Self {
        Self {
            server: true,
            client: false,
            serialize: SerializeStrategy::Never,
        }
    }

    /// Creates a client-only boundary.
    pub fn client_only() -> Self {
        Self {
            server: false,
            client: true,
            serialize: SerializeStrategy::Json,
        }
    }

    /// Checks if this marks sensitive data (should never leave server).
    pub fn is_sensitive(&self) -> bool {
        self.server && matches!(self.serialize, SerializeStrategy::Never)
    }
}

// =============================================================================
// @island directive
// =============================================================================

/// Hydration strategy for islands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HydrationStrategy {
    /// Hydrate immediately on load.
    Immediate,
    /// Hydrate during browser idle time.
    #[default]
    Idle,
    /// Hydrate when visible (IntersectionObserver).
    Visible,
    /// Hydrate on user interaction.
    Interaction,
    /// Never hydrate (static content).
    Never,
}

/// Hydration priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HydrationPriority {
    /// Critical - hydrate first.
    Critical,
    /// High priority.
    High,
    /// Normal priority.
    #[default]
    Normal,
    /// Low priority.
    Low,
}

/// Arguments for @island directive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandDirective {
    /// Island name (component identifier).
    pub name: String,

    /// Hydration strategy.
    #[serde(default)]
    pub hydrate: HydrationStrategy,

    /// Client-side bundle to load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_bundle: Option<String>,
}

impl IslandDirective {
    /// Creates a new island.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            hydrate: HydrationStrategy::default(),
            client_bundle: None,
        }
    }

    /// Sets hydration strategy.
    pub fn with_hydrate(mut self, strategy: HydrationStrategy) -> Self {
        self.hydrate = strategy;
        self
    }

    /// Sets the client bundle.
    pub fn with_client_bundle(mut self, bundle: impl Into<String>) -> Self {
        self.client_bundle = Some(bundle.into());
        self
    }
}

// =============================================================================
// @hydrate directive
// =============================================================================

/// Arguments for @hydrate directive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HydrateDirective {
    /// Hydration strategy.
    #[serde(default)]
    pub strategy: HydrationStrategy,

    /// Hydration priority.
    #[serde(default)]
    pub priority: HydrationPriority,
}

impl HydrateDirective {
    /// Creates a new hydrate directive.
    pub fn new(strategy: HydrationStrategy) -> Self {
        Self {
            strategy,
            priority: HydrationPriority::Normal,
        }
    }

    /// Sets priority.
    pub fn with_priority(mut self, priority: HydrationPriority) -> Self {
        self.priority = priority;
        self
    }
}

// =============================================================================
// @priority directive
// =============================================================================

/// Arguments for @priority directive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityDirective {
    /// Priority level (1 = highest, 10 = lowest).
    #[serde(default = "default_priority")]
    pub level: u8,

    /// Deadline for completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,

    /// Whether the query can be preempted.
    #[serde(default = "default_true")]
    pub preemptible: bool,
}

fn default_priority() -> u8 {
    5
}

impl Default for PriorityDirective {
    fn default() -> Self {
        Self {
            level: 5,
            deadline: None,
            preemptible: true,
        }
    }
}

impl PriorityDirective {
    /// Creates a critical priority.
    pub fn critical() -> Self {
        Self {
            level: 1,
            deadline: None,
            preemptible: false,
        }
    }

    /// Creates a high priority.
    pub fn high() -> Self {
        Self {
            level: 2,
            deadline: None,
            preemptible: true,
        }
    }

    /// Creates a normal priority.
    pub fn normal() -> Self {
        Self::default()
    }

    /// Creates a low priority.
    pub fn low() -> Self {
        Self {
            level: 8,
            deadline: None,
            preemptible: true,
        }
    }

    /// Creates a background priority.
    pub fn background() -> Self {
        Self {
            level: 10,
            deadline: None,
            preemptible: true,
        }
    }

    /// Sets deadline.
    pub fn with_deadline(mut self, deadline: impl Into<String>) -> Self {
        self.deadline = Some(deadline.into());
        self
    }
}

// =============================================================================
// @resources directive
// =============================================================================

/// Arguments for @resources directive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesDirective {
    /// CPU usage estimate (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<f64>,

    /// Memory usage estimate in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,

    /// I/O intensity level.
    #[serde(default)]
    pub io: ResourceLevel,

    /// Network intensity level.
    #[serde(default)]
    pub network: ResourceLevel,
}

impl ResourcesDirective {
    /// Creates resource hints for CPU-intensive operations.
    pub fn cpu_intensive(cpu: f64) -> Self {
        Self {
            cpu: Some(cpu),
            memory: None,
            io: ResourceLevel::Low,
            network: ResourceLevel::Low,
        }
    }

    /// Creates resource hints for I/O-intensive operations.
    pub fn io_intensive() -> Self {
        Self {
            cpu: Some(0.1),
            memory: None,
            io: ResourceLevel::High,
            network: ResourceLevel::Low,
        }
    }

    /// Creates resource hints for network-intensive operations.
    pub fn network_intensive() -> Self {
        Self {
            cpu: Some(0.1),
            memory: None,
            io: ResourceLevel::Low,
            network: ResourceLevel::High,
        }
    }
}

// =============================================================================
// @binary directive
// =============================================================================

/// Arguments for @binary directive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryDirective {
    /// Enable progressive streaming.
    #[serde(default)]
    pub progressive: bool,

    /// Chunk size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<u32>,

    /// Enable HLS output.
    #[serde(default)]
    pub hls: bool,

    /// HLS segment duration (if hls=true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_duration: Option<f64>,
}

impl BinaryDirective {
    /// Creates a progressive streaming directive.
    pub fn progressive() -> Self {
        Self {
            progressive: true,
            chunk_size: None,
            hls: false,
            segment_duration: None,
        }
    }

    /// Creates an HLS streaming directive.
    pub fn hls(segment_duration: f64) -> Self {
        Self {
            progressive: true,
            chunk_size: None,
            hls: true,
            segment_duration: Some(segment_duration),
        }
    }

    /// Sets chunk size.
    pub fn with_chunk_size(mut self, size: u32) -> Self {
        self.chunk_size = Some(size);
        self
    }
}

// =============================================================================
// @resumable directive
// =============================================================================

/// Arguments for @resumable directive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumableDirective {
    /// Time-to-live for resume state in seconds.
    #[serde(default = "default_ttl")]
    pub ttl: u64,

    /// Checkpoint interval (items between checkpoints).
    #[serde(default = "default_checkpoint_interval")]
    pub checkpoint_interval: u32,
}

fn default_ttl() -> u64 {
    3600 // 1 hour
}

fn default_checkpoint_interval() -> u32 {
    50
}

impl Default for ResumableDirective {
    fn default() -> Self {
        Self {
            ttl: 3600,
            checkpoint_interval: 50,
        }
    }
}

impl ResumableDirective {
    /// Creates with custom TTL.
    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl = ttl_seconds;
        self
    }

    /// Creates with custom checkpoint interval.
    pub fn with_checkpoint_interval(mut self, interval: u32) -> Self {
        self.checkpoint_interval = interval;
        self
    }
}

// =============================================================================
// @defer directive (extended)
// =============================================================================

/// Extended @defer directive with fallback support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferDirective {
    /// Label for identifying this defer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Condition for when to defer.
    #[serde(rename = "if", default = "default_true")]
    pub condition: bool,

    /// Fallback data while loading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<serde_json::Value>,
}

impl Default for DeferDirective {
    fn default() -> Self {
        Self {
            label: None,
            condition: true,
            fallback: None,
        }
    }
}

impl DeferDirective {
    /// Creates a labeled defer.
    pub fn labeled(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            condition: true,
            fallback: None,
        }
    }

    /// Sets fallback data.
    pub fn with_fallback(mut self, fallback: serde_json::Value) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// Sets condition.
    pub fn with_condition(mut self, condition: bool) -> Self {
        self.condition = condition;
        self
    }
}

// =============================================================================
// @stream directive (extended)
// =============================================================================

/// Extended @stream directive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDirective {
    /// Label for identifying this stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Condition for when to stream.
    #[serde(rename = "if", default = "default_true")]
    pub condition: bool,

    /// Initial items to return before streaming.
    #[serde(default)]
    pub initial_count: u32,
}

impl Default for StreamDirective {
    fn default() -> Self {
        Self {
            label: None,
            condition: true,
            initial_count: 0,
        }
    }
}

impl StreamDirective {
    /// Creates a labeled stream.
    pub fn labeled(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            condition: true,
            initial_count: 0,
        }
    }

    /// Sets initial count.
    pub fn with_initial_count(mut self, count: u32) -> Self {
        self.initial_count = count;
        self
    }
}

// =============================================================================
// Directive Definitions (for schema introspection)
// =============================================================================

/// Creates all BGQL streaming directives.
pub fn create_streaming_directives() -> Vec<DirectiveDefinition> {
    vec![
        create_server_directive(),
        create_boundary_directive(),
        create_island_directive(),
        create_hydrate_directive(),
        create_priority_directive(),
        create_resources_directive(),
        create_binary_directive(),
        create_resumable_directive(),
        create_defer_directive(),
        create_stream_directive(),
    ]
}

fn create_server_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "isolate".to_string(),
        InputFieldDef {
            name: "isolate".to_string(),
            description: Some("Server-side only execution".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("true".to_string()),
        },
    );

    arguments.insert(
        "cache".to_string(),
        InputFieldDef {
            name: "cache".to_string(),
            description: Some("Cache strategy".to_string()),
            ty: TypeRef::Named("CacheStrategy".to_string()),
            default_value: Some("NONE".to_string()),
        },
    );

    arguments.insert(
        "prerender".to_string(),
        InputFieldDef {
            name: "prerender".to_string(),
            description: Some("Enable prerendering".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("false".to_string()),
        },
    );

    DirectiveDefinition {
        name: "server".to_string(),
        description: Some("Marks a fragment as server-only".to_string()),
        arguments,
        locations: vec![DirectiveLocation::FragmentDefinition],
        repeatable: false,
    }
}

fn create_boundary_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "server".to_string(),
        InputFieldDef {
            name: "server".to_string(),
            description: Some("Server-only field".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("false".to_string()),
        },
    );

    arguments.insert(
        "client".to_string(),
        InputFieldDef {
            name: "client".to_string(),
            description: Some("Client-only field".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("false".to_string()),
        },
    );

    arguments.insert(
        "serialize".to_string(),
        InputFieldDef {
            name: "serialize".to_string(),
            description: Some("Serialization strategy".to_string()),
            ty: TypeRef::Named("SerializeStrategy".to_string()),
            default_value: Some("JSON".to_string()),
        },
    );

    DirectiveDefinition {
        name: "boundary".to_string(),
        description: Some("Defines client-server boundary for a field or type".to_string()),
        arguments,
        locations: vec![
            DirectiveLocation::Object,
            DirectiveLocation::FieldDefinition,
        ],
        repeatable: false,
    }
}

fn create_island_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "name".to_string(),
        InputFieldDef {
            name: "name".to_string(),
            description: Some("Island component name".to_string()),
            ty: TypeRef::Named("String".to_string()),
            default_value: None,
        },
    );

    arguments.insert(
        "hydrate".to_string(),
        InputFieldDef {
            name: "hydrate".to_string(),
            description: Some("Hydration strategy".to_string()),
            ty: TypeRef::Named("HydrationStrategy".to_string()),
            default_value: Some("VISIBLE".to_string()),
        },
    );

    arguments.insert(
        "clientBundle".to_string(),
        InputFieldDef {
            name: "clientBundle".to_string(),
            description: Some("Client-side bundle path".to_string()),
            ty: TypeRef::option(TypeRef::Named("String".to_string())),
            default_value: None,
        },
    );

    DirectiveDefinition {
        name: "island".to_string(),
        description: Some("Marks a fragment as an interactive island".to_string()),
        arguments,
        locations: vec![DirectiveLocation::FragmentDefinition],
        repeatable: false,
    }
}

fn create_hydrate_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "strategy".to_string(),
        InputFieldDef {
            name: "strategy".to_string(),
            description: Some("Hydration strategy".to_string()),
            ty: TypeRef::Named("HydrationStrategy".to_string()),
            default_value: Some("IDLE".to_string()),
        },
    );

    arguments.insert(
        "priority".to_string(),
        InputFieldDef {
            name: "priority".to_string(),
            description: Some("Hydration priority".to_string()),
            ty: TypeRef::Named("HydrationPriority".to_string()),
            default_value: Some("NORMAL".to_string()),
        },
    );

    DirectiveDefinition {
        name: "hydrate".to_string(),
        description: Some("Controls hydration behavior".to_string()),
        arguments,
        locations: vec![
            DirectiveLocation::FragmentSpread,
            DirectiveLocation::InlineFragment,
        ],
        repeatable: false,
    }
}

fn create_priority_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "level".to_string(),
        InputFieldDef {
            name: "level".to_string(),
            description: Some("Priority level (1=highest, 10=lowest)".to_string()),
            ty: TypeRef::Named("Int".to_string()),
            default_value: Some("5".to_string()),
        },
    );

    arguments.insert(
        "deadline".to_string(),
        InputFieldDef {
            name: "deadline".to_string(),
            description: Some("Completion deadline".to_string()),
            ty: TypeRef::option(TypeRef::Named("DateTime".to_string())),
            default_value: None,
        },
    );

    arguments.insert(
        "preemptible".to_string(),
        InputFieldDef {
            name: "preemptible".to_string(),
            description: Some("Whether query can be preempted".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("true".to_string()),
        },
    );

    DirectiveDefinition {
        name: "priority".to_string(),
        description: Some("Sets execution priority for a query or field".to_string()),
        arguments,
        locations: vec![
            DirectiveLocation::Query,
            DirectiveLocation::Mutation,
            DirectiveLocation::Field,
        ],
        repeatable: false,
    }
}

fn create_resources_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "cpu".to_string(),
        InputFieldDef {
            name: "cpu".to_string(),
            description: Some("CPU usage estimate (0.0-1.0)".to_string()),
            ty: TypeRef::option(TypeRef::Named("Float".to_string())),
            default_value: None,
        },
    );

    arguments.insert(
        "memory".to_string(),
        InputFieldDef {
            name: "memory".to_string(),
            description: Some("Memory usage in bytes".to_string()),
            ty: TypeRef::option(TypeRef::Named("Int".to_string())),
            default_value: None,
        },
    );

    arguments.insert(
        "io".to_string(),
        InputFieldDef {
            name: "io".to_string(),
            description: Some("I/O intensity level".to_string()),
            ty: TypeRef::Named("ResourceLevel".to_string()),
            default_value: Some("LOW".to_string()),
        },
    );

    arguments.insert(
        "network".to_string(),
        InputFieldDef {
            name: "network".to_string(),
            description: Some("Network intensity level".to_string()),
            ty: TypeRef::Named("ResourceLevel".to_string()),
            default_value: Some("LOW".to_string()),
        },
    );

    DirectiveDefinition {
        name: "resources".to_string(),
        description: Some("Resource hints for scheduling".to_string()),
        arguments,
        locations: vec![DirectiveLocation::FieldDefinition],
        repeatable: false,
    }
}

fn create_binary_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "progressive".to_string(),
        InputFieldDef {
            name: "progressive".to_string(),
            description: Some("Enable progressive streaming".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("false".to_string()),
        },
    );

    arguments.insert(
        "chunkSize".to_string(),
        InputFieldDef {
            name: "chunkSize".to_string(),
            description: Some("Chunk size in bytes".to_string()),
            ty: TypeRef::option(TypeRef::Named("Int".to_string())),
            default_value: None,
        },
    );

    arguments.insert(
        "hls".to_string(),
        InputFieldDef {
            name: "hls".to_string(),
            description: Some("Enable HLS output".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("false".to_string()),
        },
    );

    arguments.insert(
        "segmentDuration".to_string(),
        InputFieldDef {
            name: "segmentDuration".to_string(),
            description: Some("HLS segment duration in seconds".to_string()),
            ty: TypeRef::option(TypeRef::Named("Float".to_string())),
            default_value: None,
        },
    );

    DirectiveDefinition {
        name: "binary".to_string(),
        description: Some("Marks a field as returning binary stream data".to_string()),
        arguments,
        locations: vec![DirectiveLocation::FieldDefinition],
        repeatable: false,
    }
}

fn create_resumable_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "ttl".to_string(),
        InputFieldDef {
            name: "ttl".to_string(),
            description: Some("Time-to-live for resume state in seconds".to_string()),
            ty: TypeRef::Named("Int".to_string()),
            default_value: Some("3600".to_string()),
        },
    );

    arguments.insert(
        "checkpointInterval".to_string(),
        InputFieldDef {
            name: "checkpointInterval".to_string(),
            description: Some("Items between checkpoints".to_string()),
            ty: TypeRef::Named("Int".to_string()),
            default_value: Some("50".to_string()),
        },
    );

    DirectiveDefinition {
        name: "resumable".to_string(),
        description: Some("Enables pause/resume for a query".to_string()),
        arguments,
        locations: vec![DirectiveLocation::Query],
        repeatable: false,
    }
}

fn create_defer_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "label".to_string(),
        InputFieldDef {
            name: "label".to_string(),
            description: Some("Label for this defer".to_string()),
            ty: TypeRef::option(TypeRef::Named("String".to_string())),
            default_value: None,
        },
    );

    arguments.insert(
        "if".to_string(),
        InputFieldDef {
            name: "if".to_string(),
            description: Some("Condition for deferring".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("true".to_string()),
        },
    );

    DirectiveDefinition {
        name: "defer".to_string(),
        description: Some("Defers delivery of fragment data".to_string()),
        arguments,
        locations: vec![
            DirectiveLocation::FragmentSpread,
            DirectiveLocation::InlineFragment,
        ],
        repeatable: false,
    }
}

fn create_stream_directive() -> DirectiveDefinition {
    let mut arguments = IndexMap::new();

    arguments.insert(
        "label".to_string(),
        InputFieldDef {
            name: "label".to_string(),
            description: Some("Label for this stream".to_string()),
            ty: TypeRef::option(TypeRef::Named("String".to_string())),
            default_value: None,
        },
    );

    arguments.insert(
        "if".to_string(),
        InputFieldDef {
            name: "if".to_string(),
            description: Some("Condition for streaming".to_string()),
            ty: TypeRef::Named("Boolean".to_string()),
            default_value: Some("true".to_string()),
        },
    );

    arguments.insert(
        "initialCount".to_string(),
        InputFieldDef {
            name: "initialCount".to_string(),
            description: Some("Initial items before streaming".to_string()),
            ty: TypeRef::Named("Int".to_string()),
            default_value: Some("0".to_string()),
        },
    );

    DirectiveDefinition {
        name: "stream".to_string(),
        description: Some("Streams list items incrementally".to_string()),
        arguments,
        locations: vec![DirectiveLocation::Field],
        repeatable: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_directive() {
        let directive = ServerDirective::new()
            .with_cache(CacheStrategy::User)
            .with_prerender(true);

        assert_eq!(directive.cache, CacheStrategy::User);
        assert!(directive.prerender);
        assert!(directive.isolate);
    }

    #[test]
    fn test_boundary_directive() {
        let sensitive = BoundaryDirective::server_only();
        assert!(sensitive.is_sensitive());

        let client = BoundaryDirective::client_only();
        assert!(!client.is_sensitive());
    }

    #[test]
    fn test_island_directive() {
        let island = IslandDirective::new("comments")
            .with_hydrate(HydrationStrategy::Visible)
            .with_client_bundle("comments.js");

        assert_eq!(island.name, "comments");
        assert_eq!(island.hydrate, HydrationStrategy::Visible);
        assert_eq!(island.client_bundle, Some("comments.js".to_string()));
    }

    #[test]
    fn test_priority_directive() {
        let critical = PriorityDirective::critical();
        assert_eq!(critical.level, 1);
        assert!(!critical.preemptible);

        let background = PriorityDirective::background();
        assert_eq!(background.level, 10);
        assert!(background.preemptible);
    }

    #[test]
    fn test_binary_directive() {
        let hls = BinaryDirective::hls(6.0);
        assert!(hls.progressive);
        assert!(hls.hls);
        assert_eq!(hls.segment_duration, Some(6.0));
    }

    #[test]
    fn test_create_directives() {
        let directives = create_streaming_directives();
        assert_eq!(directives.len(), 10);

        let names: Vec<_> = directives.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"server"));
        assert!(names.contains(&"boundary"));
        assert!(names.contains(&"island"));
        assert!(names.contains(&"hydrate"));
        assert!(names.contains(&"priority"));
        assert!(names.contains(&"resources"));
        assert!(names.contains(&"binary"));
        assert!(names.contains(&"resumable"));
        assert!(names.contains(&"defer"));
        assert!(names.contains(&"stream"));
    }
}
