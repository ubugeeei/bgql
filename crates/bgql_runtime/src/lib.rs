//! Runtime for Better GraphQL.
//!
//! This crate provides the GraphQL execution runtime:
//! - `schema`: Schema definition and building
//! - `executor`: Query execution
//! - `query`: Query planning
<<<<<<< HEAD
=======
//! - `resolver`: Field resolution system
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
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
<<<<<<< HEAD
=======
pub mod resolver;
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
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
<<<<<<< HEAD
pub use executor::{Context, Executor, ExecutorConfig, FieldError, Response};
pub use hls::{HlsManifest, HlsPlaylist, HlsSegment, HlsStreamGenerator};
pub use query::{PlannerConfig, QueryPlan, QueryPlanner};
=======
pub use executor::{Context, Executor, ExecutorConfig, FieldError, PathSegment, Response};
pub use hls::{HlsManifest, HlsPlaylist, HlsSegment, HlsStreamGenerator};
pub use query::{FieldInfo, PlanError, PlanNode, PlannerConfig, QueryPlan, QueryPlanner};
pub use resolver::{
    AsyncFnResolver, DefaultResolver, FnResolver, Resolver, ResolverArgs, ResolverError,
    ResolverFuture, ResolverInfo, ResolverMap, ResolverResult,
};
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
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
