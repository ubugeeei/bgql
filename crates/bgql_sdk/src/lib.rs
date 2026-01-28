//! Better GraphQL SDK
//!
//! This crate provides high-level APIs for building GraphQL clients and servers.
//!
//! # Rust Client
//!
//! ```ignore
//! use bgql_sdk::client::{BgqlClient, ClientConfig};
//!
//! let client = BgqlClient::new("http://localhost:4000/graphql");
//! let result = client.query::<UserQuery>()
//!     .variables(UserQueryVariables { id: "1".into() })
//!     .execute()
//!     .await?;
//! ```
//!
//! # Rust Server
//!
//! ```ignore
//! use bgql_sdk::server::{BgqlServer, ServerConfig};
//!
//! let server = BgqlServer::builder()
//!     .schema_sdl("type Query { hello: String }")
//!     .resolver("Query", "hello", |_args, _ctx| async {
//!         Ok(serde_json::json!("Hello, World!"))
//!     })
//!     .build()?;
//!
//! // Execute a query directly
//! let result = server.execute("query { hello }", None, Context::new()).await?;
//!
//! // Or start the HTTP server
//! server.listen().await?;
//! ```

pub mod client;
pub mod result;
pub mod server;

// Re-exports
pub use client::{BgqlClient, ClientConfig, GraphQLOperation, Request, Response};
pub use result::{BgqlError, BgqlResult, Err, Ok, Result};
pub use server::{BgqlServer, Context, DataLoader, Resolver, ServerConfig, create_loader};

// Re-export runtime types that are commonly needed
pub use bgql_runtime::schema::Schema;
pub use bgql_runtime::executor::{ExecutorConfig, FieldError};
pub use bgql_runtime::resolver::{ResolverArgs, ResolverInfo, ResolverMap, ResolverResult, ResolverError};
