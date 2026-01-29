//! Better GraphQL Example Server
//!
//! Type-safe GraphQL server using generated resolvers.
//!
//! # Running
//! ```bash
//! cd examples/rust-server && cargo run --release
//! ```

mod application;
mod domain;
mod generated;
mod infrastructure;
mod presentation;

use bgql_sdk::server::{BgqlServer, ServerConfig};
use bgql_sdk::SdkResult;
use generated::ServerBuilderExt;
use infrastructure::*;
use presentation::*;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> SdkResult<()> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("bgql=info".parse().unwrap()))
        .with_target(false)
        .compact()
        .init();

    // Infrastructure
    let user_repo: Arc<dyn UserRepository> = Arc::new(InMemoryUserRepository::with_seed_data());
    let post_repo: Arc<dyn PostRepository> = Arc::new(InMemoryPostRepository::with_seed_data());
    let comment_repo: Arc<dyn CommentRepository> = Arc::new(InMemoryCommentRepository::new());
    info!("Repositories initialized");

    // Application
    let ctx = Arc::new(AppContext::new(user_repo, post_repo, comment_repo));
    info!("Application context created");

    // Type-safe resolvers
    let query = Arc::new(AppQueryResolvers::new(ctx.clone()));
    let mutation = Arc::new(AppMutationResolvers::new(ctx));

    // Server - no more manual clone() for each resolver!
    BgqlServer::builder()
        .config(ServerConfig::new().port(4000).host("0.0.0.0"))
        .schema_sdl(include_str!("../schema.bgql"))
        .query_resolvers(query)
        .mutation_resolvers(mutation)
        .build()?
        .listen()
        .await
}
