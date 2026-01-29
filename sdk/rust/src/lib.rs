//! Better GraphQL SDK
//!
//! This crate provides high-level APIs for building GraphQL clients and servers
//! with strong type safety and inference.
//!
//! # Type-Safe Client
//!
//! ```ignore
//! use bgql_sdk::client::BgqlClient;
//! use bgql_sdk::typed::{TypedOperation, OperationKind};
//! use serde::{Serialize, Deserialize};
//!
//! // Define typed operation
//! #[derive(Serialize)]
//! struct GetUserVars { id: String }
//!
//! #[derive(Deserialize)]
//! struct GetUserData { user: Option<User> }
//!
//! struct GetUser;
//! impl TypedOperation for GetUser {
//!     type Variables = GetUserVars;
//!     type Response = GetUserData;
//!     const OPERATION: &'static str = "query GetUser($id: ID!) { user(id: $id) { id name } }";
//!     const OPERATION_NAME: &'static str = "GetUser";
//!     const KIND: OperationKind = OperationKind::Query;
//! }
//!
//! // Execute with full type safety
//! let response = client.execute_typed_ok::<GetUser>(GetUserVars { id: "1".into() }).await?;
//! ```
//!
//! # Type-Safe Server
//!
//! ```ignore
//! use bgql_sdk::server::BgqlServer;
//! use bgql_sdk::context::{TypedContext, ContextExt, data::CurrentUserId};
//! use bgql_sdk::typed::ResolverBuilder;
//!
//! #[derive(Deserialize)]
//! struct GetUserArgs { id: String }
//!
//! #[derive(Serialize)]
//! struct User { id: String, name: String }
//!
//! let resolvers = ResolverBuilder::<MySchema>::new()
//!     .query::<GetUserArgs, CurrentUserId, User, _, _>("user", |args, user_id| async move {
//!         // user_id is automatically extracted from context
//!         Ok(User { id: args.id, name: "Alice".into() })
//!     })
//!     .build();
//! ```

pub mod client;
pub mod context;
pub mod directives;
pub mod error;
pub(crate) mod http;
pub mod pubsub;
pub mod result;
pub mod server;
pub mod streaming;
pub mod typed;
pub mod validation;

// Re-export macros
pub use bgql_macros::{args, gql, graphql, resolver, resolvers, ContextKey, TypedOperation};

// Re-exports for convenience
pub use client::{BgqlClient, ClientConfig, GraphQLOperation, Request, Response};
pub use context::{ContextExt, SharedContext, TypedContext};
pub use error::{ErrorCode, ResultExt, SdkError, SdkResult};
pub use typed::{
    FromTypedContext, GraphQLArgs, GraphQLOutput, GraphQLParent, NoArgs, NoVariables,
    OperationKind, ResolverBuilder, Root, TypedOperation, TypedResolver, TypedResponse,
};

// Legacy re-exports (deprecated, use error module instead)
pub use result::{BgqlError, BgqlResult, Err, Ok, Result};

// Server re-exports
pub use server::{create_loader, BgqlServer, Context, DataLoader, Resolver, ServerConfig};

// Re-export runtime types that are commonly needed
pub use bgql_runtime::executor::{ExecutorConfig, FieldError};
pub use bgql_runtime::resolver::{
    ResolverArgs, ResolverError, ResolverInfo, ResolverMap, ResolverResult,
};
pub use bgql_runtime::schema::Schema;

// Streaming re-exports
pub use streaming::{
    Checkpoint, DeferPayload, DeferPayloadBuilder, ExecutionState, IncrementalEvent,
    IncrementalSender, IncrementalStream, MultipartWriter, PathSegment, StreamPayload,
    StreamPayloadBuilder, StreamingResponse,
};

// Directive re-exports
pub use directives::{
    BinaryDirective, BoundaryDirective, CacheDirective, CacheScope, CacheStrategy, DeferDirective,
    HydrateDirective, HydrationPriority, HydrationStrategy, IslandDirective, ParsedDirectives,
    PriorityDirective, RateLimitDirective, RequireAuthDirective, ResourceLevel, ResourcesDirective,
    ResumableDirective, ServerDirective, StreamDirective,
};

// Validation re-exports
pub use validation::{
    Validate, ValidationError, ValidationErrorCode, ValidationErrors, ValidationResult, Validator,
};

// PubSub re-exports
pub use pubsub::{PubSub, TypedPubSub, TypedReceiver};
