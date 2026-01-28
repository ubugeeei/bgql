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
//!     .schema("schema.bgql")
//!     .resolver("Query", "users", |ctx| async { ... })
//!     .build()?;
//!
//! server.listen(4000).await?;
//! ```

pub mod client;
pub mod result;
pub mod server;

// Re-exports
pub use client::{BgqlClient, ClientConfig};
pub use result::{BgqlError, BgqlResult, Err, Ok, Result};
pub use server::{BgqlServer, Resolver, ServerConfig};
