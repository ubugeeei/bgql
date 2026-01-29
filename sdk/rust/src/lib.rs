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
pub use bgql_macros::{TypedOperation, ContextKey, resolver, graphql, gql, args, resolvers};

// Re-exports for convenience
pub use client::{BgqlClient, ClientConfig, GraphQLOperation, Request, Response};
pub use context::{TypedContext, ContextExt, SharedContext};
pub use error::{ErrorCode, SdkError, SdkResult, ResultExt};
pub use typed::{
    TypedOperation, OperationKind, TypedResponse,
    TypedResolver, ResolverBuilder, FromTypedContext,
    NoVariables, NoArgs, Root,
    GraphQLArgs, GraphQLOutput, GraphQLParent,
};

// Legacy re-exports (deprecated, use error module instead)
pub use result::{BgqlError, BgqlResult, Err, Ok, Result};

// Server re-exports
pub use server::{BgqlServer, Context, DataLoader, Resolver, ServerConfig, create_loader};

// Re-export runtime types that are commonly needed
pub use bgql_runtime::schema::Schema;
pub use bgql_runtime::executor::{ExecutorConfig, FieldError};
pub use bgql_runtime::resolver::{ResolverArgs, ResolverInfo, ResolverMap, ResolverResult, ResolverError};

// Streaming re-exports
pub use streaming::{
    StreamingResponse, DeferPayload, StreamPayload, PathSegment, IncrementalEvent,
    ExecutionState, Checkpoint, IncrementalStream, IncrementalSender,
    DeferPayloadBuilder, StreamPayloadBuilder, MultipartWriter,
};

// Directive re-exports
pub use directives::{
    DeferDirective, StreamDirective, BinaryDirective, ServerDirective, BoundaryDirective,
    PriorityDirective, ResourcesDirective, ResumableDirective, IslandDirective, HydrateDirective,
    CacheDirective, RateLimitDirective, RequireAuthDirective, ParsedDirectives,
    CacheStrategy, CacheScope, ResourceLevel, HydrationStrategy, HydrationPriority,
};

// Validation re-exports
pub use validation::{
    Validate, Validator, ValidationError, ValidationErrors, ValidationErrorCode, ValidationResult,
};

// PubSub re-exports
pub use pubsub::{PubSub, TypedPubSub, TypedReceiver};
