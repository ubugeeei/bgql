//! Runtime for Better GraphQL.
//!
//! This crate provides the GraphQL execution runtime:
//! - `schema`: Schema definition and building
//! - `executor`: Query execution
//! - `query`: Query planning
//! - `dataloader`: DataLoader for N+1 prevention
//! - `streaming`: @defer/@stream support
//! - `state`: Execution state management for pause/resume
//! - `resource`: Resource management for scheduling
//! - `scheduler`: Priority-based query scheduling
//! - `binary_transport`: Binary streaming protocol
//! - `hls`: HTTP Live Streaming support
//! - `directives`: Built-in streaming directives

pub mod binary_transport;
pub mod dataloader;
pub mod directives;
pub mod executor;
pub mod hls;
pub mod query;
pub mod resource;
pub mod scheduler;
pub mod schema;
pub mod state;
pub mod streaming;

pub use binary_transport::{BinaryChunk, BinaryProtocol, BinaryStreamHandle};
pub use dataloader::DataLoader;
pub use directives::{
    create_streaming_directives, BinaryDirective, BoundaryDirective, CacheStrategy, DeferDirective,
    HydrateDirective, HydrationPriority, HydrationStrategy, IslandDirective, PriorityDirective,
    ResourcesDirective, ResumableDirective, SerializeStrategy, ServerDirective, StreamDirective,
};
pub use executor::{Context, Executor, ExecutorConfig, FieldError, Response};
pub use hls::{HlsManifest, HlsPlaylist, HlsSegment, HlsStreamGenerator};
pub use query::{PlannerConfig, QueryPlan, QueryPlanner};
pub use resource::{ResourceLimits, ResourceManager, ResourceRequirements, ResourceUsage};
pub use scheduler::{ExecutionHandle, QueryScheduler, SchedulerConfig, TaskPriority, TaskStatus};
pub use schema::{
    DirectiveDefinition, DirectiveLocation, EndpointConfig, Schema, SchemaBuilder, SchemaMetadata,
    SchemaVersion,
};
pub use state::{
    BinaryStreamPhase, BinaryStreamState, Checkpoint, ExecutionPhase, ExecutionPosition,
    ExecutionState, StreamCursor,
};
pub use streaming::{DeferPayload, StreamPayload, StreamingResponse};
