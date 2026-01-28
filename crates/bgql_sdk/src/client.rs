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
    /// Retry delay base (in milliseconds) - exponential backoff will be applied.
    pub retry_delay_ms: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            headers: HashMap::new(),
            retry_delay_ms: 100,
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

    /// Sets the retry delay base in milliseconds.
    pub fn retry_delay_ms(mut self, delay: u64) -> Self {
        self.retry_delay_ms = delay;
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

/// HTTP client implementation using simple TCP sockets.
/// This avoids the need for external HTTP client dependencies.
struct HttpClient {
    timeout: Duration,
}

impl HttpClient {
    fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    async fn post(&self, url: &str, body: &str, headers: &HashMap<String, String>) -> BgqlResult<String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use tokio::time::timeout;

        // Parse URL
        let (host, port, path) = parse_url(url)?;

        // Connect with timeout
        let connect_future = TcpStream::connect(format!("{}:{}", host, port));
        let mut stream = timeout(self.timeout, connect_future)
            .await
            .map_err(|_| BgqlError::timeout())?
            .map_err(|e| BgqlError::network(format!("Connection failed: {}", e)))?;

        // Build HTTP request
        let mut request = format!(
            "POST {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n",
            path,
            host,
            body.len()
        );

        for (key, value) in headers {
            request.push_str(&format!("{}: {}\r\n", key, value));
        }
        request.push_str("\r\n");
        request.push_str(body);

        // Send request
        let write_future = stream.write_all(request.as_bytes());
        timeout(self.timeout, write_future)
            .await
            .map_err(|_| BgqlError::timeout())?
            .map_err(|e| BgqlError::network(format!("Write failed: {}", e)))?;

        // Read response
        let mut response_bytes = Vec::new();
        let read_future = stream.read_to_end(&mut response_bytes);
        timeout(self.timeout, read_future)
            .await
            .map_err(|_| BgqlError::timeout())?
            .map_err(|e| BgqlError::network(format!("Read failed: {}", e)))?;

        let response_str = String::from_utf8_lossy(&response_bytes);

        // Parse HTTP response
        parse_http_response(&response_str)
    }
}

/// Parses a URL into host, port, and path.
fn parse_url(url: &str) -> BgqlResult<(String, u16, String)> {
    let url = url.trim();

    // Remove protocol
    let without_protocol = if url.starts_with("https://") {
        return Err(BgqlError::new(
            "HTTPS_NOT_SUPPORTED",
            "HTTPS is not supported in the simple HTTP client. Use a proxy or configure your server for HTTP.",
        ));
    } else if url.starts_with("http://") {
        &url[7..]
    } else {
        url
    };

    // Split host:port and path
    let (host_port, path) = if let Some(slash_pos) = without_protocol.find('/') {
        (&without_protocol[..slash_pos], &without_protocol[slash_pos..])
    } else {
        (without_protocol, "/")
    };

    // Split host and port
    let (host, port) = if let Some(colon_pos) = host_port.rfind(':') {
        let host = &host_port[..colon_pos];
        let port_str = &host_port[colon_pos + 1..];
        let port = port_str
            .parse()
            .map_err(|_| BgqlError::new("INVALID_URL", format!("Invalid port: {}", port_str)))?;
        (host.to_string(), port)
    } else {
        (host_port.to_string(), 80)
    };

    Ok((host, port, path.to_string()))
}

/// Parses an HTTP response and extracts the body.
fn parse_http_response(response: &str) -> BgqlResult<String> {
    // Find the status line
    let lines: Vec<&str> = response.lines().collect();
    if lines.is_empty() {
        return Err(BgqlError::network("Empty response"));
    }

    // Check status
    let status_line = lines[0];
    if !status_line.contains("200") && !status_line.contains("201") {
        // Check for other successful status codes
        if status_line.contains("4") || status_line.contains("5") {
            return Err(BgqlError::network(format!("HTTP error: {}", status_line)));
        }
    }

    // Find the body (after the empty line)
    if let Some(body_start) = response.find("\r\n\r\n") {
        let body = &response[body_start + 4..];
        // Handle chunked transfer encoding
        if response.contains("Transfer-Encoding: chunked") {
            return parse_chunked_body(body);
        }
        Ok(body.to_string())
    } else if let Some(body_start) = response.find("\n\n") {
        let body = &response[body_start + 2..];
        Ok(body.to_string())
    } else {
        Err(BgqlError::network("Could not find response body"))
    }
}

/// Parses a chunked transfer encoding body.
fn parse_chunked_body(body: &str) -> BgqlResult<String> {
    let mut result = String::new();
    let mut remaining = body;

    loop {
        // Find chunk size line
        let size_end = remaining.find("\r\n").or_else(|| remaining.find('\n'));
        if size_end.is_none() {
            break;
        }
        let size_end = size_end.unwrap();
        let size_str = remaining[..size_end].trim();

        // Parse chunk size (hex)
        let chunk_size = usize::from_str_radix(size_str, 16).unwrap_or(0);
        if chunk_size == 0 {
            break;
        }

        // Skip to chunk data
        let data_start = if remaining.contains("\r\n") {
            size_end + 2
        } else {
            size_end + 1
        };

        if data_start + chunk_size > remaining.len() {
            result.push_str(&remaining[data_start..]);
            break;
        }

        result.push_str(&remaining[data_start..data_start + chunk_size]);
        remaining = &remaining[data_start + chunk_size..];

        // Skip trailing CRLF
        if remaining.starts_with("\r\n") {
            remaining = &remaining[2..];
        } else if remaining.starts_with('\n') {
            remaining = &remaining[1..];
        }
    }

    Ok(result)
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

    /// Executes a raw request with retry logic.
    pub async fn execute_raw(&self, request: Request) -> BgqlResult<Response> {
        let mut last_error = BgqlError::network("No attempts made");
        let http_client = HttpClient::new(self.config.timeout);

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                // Exponential backoff
                let delay = self.config.retry_delay_ms * (2_u64.pow(attempt - 1));
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            // Merge headers
            let mut headers = self.config.headers.clone();
            for (k, v) in &request.headers {
                headers.insert(k.clone(), v.clone());
            }

            // Serialize request body
            let body = serde_json::json!({
                "query": request.query,
                "variables": request.variables,
                "operationName": request.operation_name,
            });
            let body_str = serde_json::to_string(&body)
                .map_err(|e| BgqlError::new("SERIALIZE_ERROR", e.to_string()))?;

            // Execute HTTP request
            match http_client.post(&self.config.url, &body_str, &headers).await {
                Ok(response_body) => {
                    // Parse JSON response
                    match serde_json::from_str::<Response>(&response_body) {
                        Ok(response) => return Ok(response),
                        Err(e) => {
                            last_error = BgqlError::parse(format!(
                                "Failed to parse response: {}. Body: {}",
                                e,
                                &response_body[..response_body.len().min(200)]
                            ));
                        }
                    }
                }
                Err(e) => {
                    last_error = e;
                    // Only retry on network/timeout errors
                    if last_error.code != "NETWORK_ERROR" && last_error.code != "TIMEOUT" {
                        return Err(last_error);
                    }
                }
            }
        }

        Err(last_error)
    }

    /// Executes a request through the middleware chain.
    async fn execute_with_middleware(&self, request: Request) -> BgqlResult<Response> {
        if self.middlewares.is_empty() {
            return self.execute_raw(request).await;
        }

        // Build middleware chain from the end
        let client = self.clone();
        let final_handler: Next = Arc::new(move |req| {
            let client = client.clone();
            Box::pin(async move { client.execute_raw(req).await })
        });

        // Chain middlewares in reverse order
        let mut next = final_handler;
        for middleware in self.middlewares.iter().rev() {
            let mw = middleware.clone();
            let current_next = next;
            next = Arc::new(move |req| {
                let mw = mw.clone();
                let next = current_next.clone();
                Box::pin(async move { mw(req, next).await })
            });
        }

        next(request).await
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

        let response = self.client.execute_with_middleware(request).await?;

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

    /// Executes the query and returns the raw response.
    pub async fn execute_raw(self) -> BgqlResult<Response> {
        let request = Request {
            query: self.query,
            variables: self.variables,
            operation_name: self.operation_name,
            headers: self.client.config.headers.clone(),
        };

        self.client.execute_with_middleware(request).await
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

/// Extension trait for executing typed operations.
impl BgqlClient {
    /// Executes a typed operation.
    pub async fn execute<Op: GraphQLOperation>(
        &self,
        variables: Op::Variables,
    ) -> BgqlResult<Op::Response> {
        let mut builder = self.query::<Op::Response>(Op::query());
        builder = builder.variables(variables);
        if let Some(name) = Op::operation_name() {
            builder = builder.operation_name(name);
        }
        builder.execute().await
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

    #[test]
    fn test_parse_url() {
        let (host, port, path) = parse_url("http://localhost:4000/graphql").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 4000);
        assert_eq!(path, "/graphql");

        let (host, port, path) = parse_url("http://example.com/api/graphql").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
        assert_eq!(path, "/api/graphql");
    }

    #[test]
    fn test_parse_http_response() {
        let response = "HTTP/1.1 200 OK\r\n\
                       Content-Type: application/json\r\n\
                       \r\n\
                       {\"data\":{\"hello\":\"world\"}}";
        let body = parse_http_response(response).unwrap();
        assert_eq!(body, "{\"data\":{\"hello\":\"world\"}}");
    }

    #[test]
    fn test_request_serialization() {
        let request = Request {
            query: "query { hello }".to_string(),
            variables: Some(serde_json::json!({"id": 1})),
            operation_name: Some("HelloQuery".to_string()),
            headers: HashMap::new(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("query { hello }"));
        assert!(json.contains("HelloQuery"));
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = BgqlClient::new("http://localhost:4000/graphql");
        assert_eq!(client.config.url, "http://localhost:4000/graphql");
    }

    #[tokio::test]
    async fn test_query_builder() {
        let client = BgqlClient::new("http://localhost:4000/graphql");
        let builder = client
            .query::<serde_json::Value>("query { hello }")
            .variables(serde_json::json!({"id": 1}))
            .operation_name("HelloQuery");

        assert_eq!(builder.query, "query { hello }");
        assert_eq!(builder.operation_name, Some("HelloQuery".to_string()));
    }

    #[test]
    fn test_chunked_body_parsing() {
        let chunked = "5\r\nhello\r\n5\r\nworld\r\n0\r\n\r\n";
        let result = parse_chunked_body(chunked).unwrap();
        assert_eq!(result, "helloworld");
    }
}
