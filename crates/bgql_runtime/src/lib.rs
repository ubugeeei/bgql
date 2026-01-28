//! Runtime for Better GraphQL.
//!
//! This crate provides the GraphQL execution runtime:
//! - `schema`: Schema definition and building
//! - `executor`: Query execution
//! - `query`: Query planning
//! - `dataloader`: DataLoader for N+1 prevention
//! - `streaming`: @defer/@stream support

pub mod dataloader;
pub mod executor;
pub mod query;
pub mod schema;
pub mod streaming;

pub use dataloader::DataLoader;
pub use executor::{Context, Executor, ExecutorConfig, FieldError, Response};
pub use query::{PlannerConfig, QueryPlan, QueryPlanner};
pub use schema::{DirectiveDefinition, DirectiveLocation, Schema, SchemaBuilder};
pub use streaming::{DeferPayload, StreamPayload, StreamingResponse};
