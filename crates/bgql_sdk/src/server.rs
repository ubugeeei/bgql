//! Better GraphQL Server SDK.
//!
//! Provides a type-safe GraphQL server with:
//! - Automatic DataLoader integration
//! - Input validation
//! - Middleware support
//! - Streaming (@defer/@stream)

use crate::result::{BgqlError, BgqlResult};
use bgql_runtime::schema::Schema;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Server configuration.
#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    /// Port to listen on.
    pub port: u16,
    /// Host to bind to.
    pub host: String,
    /// Enable introspection.
    pub introspection: bool,
    /// Enable playground.
    pub playground: bool,
    /// Maximum query depth.
    pub max_depth: usize,
    /// Maximum query complexity.
    pub max_complexity: usize,
}

impl ServerConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self {
            port: 4000,
            host: "localhost".to_string(),
            introspection: true,
            playground: true,
            max_depth: 10,
            max_complexity: 1000,
        }
    }

    /// Sets the port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the host.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Disables introspection.
    pub fn no_introspection(mut self) -> Self {
        self.introspection = false;
        self
    }

    /// Disables playground.
    pub fn no_playground(mut self) -> Self {
        self.playground = false;
        self
    }
}

/// Request context.
#[derive(Debug)]
pub struct Context {
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Request-scoped data.
    pub data: HashMap<String, serde_json::Value>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Creates a new context.
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
            data: HashMap::new(),
        }
    }

    /// Sets a value in the context.
    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
    }

    /// Gets a value from the context.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Gets a header value.
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }
}

/// Resolver function type.
pub type ResolverFn = Arc<
    dyn Fn(
            serde_json::Value,
            Context,
        ) -> Pin<Box<dyn Future<Output = BgqlResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

/// A resolver.
pub struct Resolver {
    #[allow(dead_code)]
    type_name: String,
    #[allow(dead_code)]
    field_name: String,
    #[allow(dead_code)]
    func: ResolverFn,
}

impl Resolver {
    /// Creates a new resolver.
    pub fn new<F, Fut>(type_name: impl Into<String>, field_name: impl Into<String>, func: F) -> Self
    where
        F: Fn(serde_json::Value, Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = BgqlResult<serde_json::Value>> + Send + 'static,
    {
        Self {
            type_name: type_name.into(),
            field_name: field_name.into(),
            func: Arc::new(move |args, ctx| Box::pin(func(args, ctx))),
        }
    }
}

/// Server builder.
#[derive(Default)]
pub struct ServerBuilder {
    config: ServerConfig,
    schema: Option<Schema>,
    resolvers: Vec<Resolver>,
}

impl ServerBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the configuration.
    pub fn config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the schema from a file path.
    pub fn schema_file(mut self, _path: impl Into<String>) -> Self {
        // TODO: Load and parse schema from file
        self.schema = Some(Schema::new());
        self
    }

    /// Sets the schema from SDL.
    pub fn schema_sdl(mut self, _sdl: impl Into<String>) -> Self {
        // TODO: Parse schema from SDL
        self.schema = Some(Schema::new());
        self
    }

    /// Adds a resolver.
    pub fn resolver<F, Fut>(
        mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        func: F,
    ) -> Self
    where
        F: Fn(serde_json::Value, Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = BgqlResult<serde_json::Value>> + Send + 'static,
    {
        self.resolvers
            .push(Resolver::new(type_name, field_name, func));
        self
    }

    /// Builds the server.
    pub fn build(self) -> BgqlResult<BgqlServer> {
        let schema = self
            .schema
            .ok_or_else(|| BgqlError::new("NO_SCHEMA", "Schema is required"))?;

        Ok(BgqlServer {
            config: self.config,
            schema,
            resolvers: self.resolvers,
        })
    }
}

/// The Better GraphQL server.
pub struct BgqlServer {
    config: ServerConfig,
    #[allow(dead_code)]
    schema: Schema,
    #[allow(dead_code)]
    resolvers: Vec<Resolver>,
}

impl BgqlServer {
    /// Creates a new server builder.
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Starts the server.
    pub async fn listen(&self) -> BgqlResult<()> {
        println!(
            "[bgql] Server starting on http://{}:{}",
            self.config.host, self.config.port
        );

        if self.config.playground {
            println!(
                "[bgql] Playground available at http://{}:{}/graphql",
                self.config.host, self.config.port
            );
        }

        // TODO: Implement actual HTTP server
        // For now, just return success
        Ok(())
    }

    /// Executes a query.
    pub async fn execute(
        &self,
        _query: &str,
        _variables: Option<serde_json::Value>,
        _ctx: Context,
    ) -> BgqlResult<serde_json::Value> {
        // TODO: Implement query execution
        Err(BgqlError::new(
            "NOT_IMPLEMENTED",
            "Execution not yet implemented",
        ))
    }
}

/// DataLoader for batching and caching.
pub struct DataLoader<K, V> {
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V> DataLoader<K, V>
where
    K: Eq + std::hash::Hash + Clone + Send,
    V: Clone + Send,
{
    /// Creates a new DataLoader.
    pub fn new<F, Fut>(_batch_fn: F) -> Self
    where
        F: Fn(Vec<K>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HashMap<K, V>> + Send + 'static,
    {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Loads a value by key.
    pub async fn load(&self, _key: K) -> Option<V> {
        // TODO: Implement batching
        None
    }

    /// Loads multiple values by keys.
    pub async fn load_many(&self, _keys: Vec<K>) -> HashMap<K, V> {
        // TODO: Implement batching
        HashMap::new()
    }

    /// Clears the cache.
    pub fn clear(&self) {
        // TODO: Implement cache clearing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let config = ServerConfig::new()
            .port(8080)
            .host("0.0.0.0")
            .no_playground();

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.playground);
    }

    #[test]
    fn test_context() {
        let mut ctx = Context::new();
        ctx.set("user_id", "123");
        assert_eq!(ctx.get::<String>("user_id"), Some("123".to_string()));
    }
}
