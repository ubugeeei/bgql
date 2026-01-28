//! Better GraphQL Client SDK.
//!
//! Provides a type-safe GraphQL client with:
//! - Automatic retries
//! - Request caching
//! - Middleware support
//! - Type-safe queries

use crate::result::{BgqlError, BgqlResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Client configuration.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the GraphQL endpoint.
    pub url: String,
    /// Default timeout.
    pub timeout: Duration,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Default headers.
    pub headers: HashMap<String, String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            headers: HashMap::new(),
        }
    }
}

impl ClientConfig {
    /// Creates a new config with a URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Sets the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the max retries.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Adds a default header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// Middleware function type.
pub type Middleware = Arc<
    dyn Fn(
            Request,
            Next,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = BgqlResult<Response>> + Send>>
        + Send
        + Sync,
>;

/// Next middleware in the chain.
pub type Next = Arc<
    dyn Fn(
            Request,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = BgqlResult<Response>> + Send>>
        + Send
        + Sync,
>;

/// A GraphQL request.
#[derive(Debug, Clone, Serialize)]
pub struct Request {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    #[serde(skip)]
    pub headers: HashMap<String, String>,
}

/// A GraphQL response.
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    pub data: Option<serde_json::Value>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// A GraphQL error.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(default)]
    pub path: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

/// The Better GraphQL client.
#[derive(Clone)]
pub struct BgqlClient {
    config: ClientConfig,
    middlewares: Vec<Middleware>,
}

impl BgqlClient {
    /// Creates a new client with the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            config: ClientConfig::new(url),
            middlewares: Vec::new(),
        }
    }

    /// Creates a new client with configuration.
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            config,
            middlewares: Vec::new(),
        }
    }

    /// Adds a middleware.
    pub fn use_middleware<F>(mut self, middleware: F) -> Self
    where
        F: Fn(
                Request,
                Next,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = BgqlResult<Response>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    /// Creates a query builder.
    pub fn query<T: DeserializeOwned>(&self, query: impl Into<String>) -> QueryBuilder<T> {
        QueryBuilder {
            client: self.clone(),
            query: query.into(),
            variables: None,
            operation_name: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a mutation builder.
    pub fn mutate<T: DeserializeOwned>(&self, mutation: impl Into<String>) -> QueryBuilder<T> {
        self.query(mutation)
    }

    /// Executes a raw request.
    pub async fn execute_raw(&self, _request: Request) -> BgqlResult<Response> {
        // TODO: Implement actual HTTP request
        // For now, return a placeholder
        Err(BgqlError::new(
            "NOT_IMPLEMENTED",
            "HTTP client not yet implemented",
        ))
    }
}

/// A query builder.
pub struct QueryBuilder<T> {
    client: BgqlClient,
    query: String,
    variables: Option<serde_json::Value>,
    operation_name: Option<String>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned> QueryBuilder<T> {
    /// Sets the variables.
    pub fn variables<V: Serialize>(mut self, variables: V) -> Self {
        self.variables = serde_json::to_value(variables).ok();
        self
    }

    /// Sets the operation name.
    pub fn operation_name(mut self, name: impl Into<String>) -> Self {
        self.operation_name = Some(name.into());
        self
    }

    /// Executes the query.
    pub async fn execute(self) -> BgqlResult<T> {
        let request = Request {
            query: self.query,
            variables: self.variables,
            operation_name: self.operation_name,
            headers: self.client.config.headers.clone(),
        };

        let response = self.client.execute_raw(request).await?;

        if let Some(errors) = response.errors {
            if !errors.is_empty() {
                return Err(BgqlError::new("GRAPHQL_ERROR", errors[0].message.clone()));
            }
        }

        match response.data {
            Some(data) => serde_json::from_value(data).map_err(|e| BgqlError::parse(e.to_string())),
            None => Err(BgqlError::new("NO_DATA", "No data in response")),
        }
    }
}

/// Typed GraphQL operation trait.
pub trait GraphQLOperation {
    type Variables: Serialize;
    type Response: DeserializeOwned;

    fn query() -> &'static str;
    fn operation_name() -> Option<&'static str> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new("http://localhost:4000/graphql")
            .timeout(Duration::from_secs(10))
            .max_retries(5)
            .header("Authorization", "Bearer token");

        assert_eq!(config.url, "http://localhost:4000/graphql");
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 5);
        assert!(config.headers.contains_key("Authorization"));
    }
}
