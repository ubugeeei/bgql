//! Built-in HTTP server for BGQL.
//!
//! This module provides a complete HTTP server that handles:
//! - POST /bgql - GraphQL queries and mutations
//! - GET /bgql - Playground UI
//! - GET /health - Health check
//! - GET /.well-known/bgql - Server capabilities

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info};

use crate::error::SdkResult;
use crate::server::{BgqlServer, Context, ServerConfig};

#[derive(Debug, Deserialize)]
pub(crate) struct GraphQLRequest {
    pub query: String,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(rename = "operationName")]
    #[allow(dead_code)]
    pub operation_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GraphQLResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GraphQLError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<String>>,
}

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn json_response<T: Serialize>(data: &T) -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(full(serde_json::to_string(data).unwrap()))
        .unwrap()
}

fn error_response(status: StatusCode, message: &str) -> Response<BoxBody> {
    let error = GraphQLResponse {
        data: None,
        errors: Some(vec![GraphQLError {
            message: message.to_string(),
            path: None,
        }]),
    };
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(full(serde_json::to_string(&error).unwrap()))
        .unwrap()
}

pub(crate) async fn handle_graphql_request(
    body_bytes: Bytes,
    server: &BgqlServer,
) -> Response<BoxBody> {
    let gql_request: GraphQLRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e));
        }
    };

    debug!(
        "Executing query: {}",
        gql_request.query.chars().take(100).collect::<String>()
    );

    let ctx = Context::new();
    let result = server
        .execute(&gql_request.query, gql_request.variables, ctx)
        .await;

    match result {
        Ok(data) => {
            let data_value = data.get("data").cloned();
            let errors_value = data.get("errors").and_then(|e| e.as_array()).map(|arr| {
                arr.iter()
                    .map(|e| GraphQLError {
                        message: e.as_str().unwrap_or("Unknown error").to_string(),
                        path: None,
                    })
                    .collect()
            });

            json_response(&GraphQLResponse {
                data: data_value,
                errors: errors_value,
            })
        }
        Err(e) => {
            error!("Query execution error: {}", e);
            json_response(&GraphQLResponse {
                data: None,
                errors: Some(vec![GraphQLError {
                    message: e.to_string(),
                    path: None,
                }]),
            })
        }
    }
}

pub(crate) fn playground_html(endpoint: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>BGQL Playground</title>
    <style>
        * {{ box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 900px; margin: 0 auto; padding: 2rem; background: #fafafa; }}
        h1 {{ color: #1a1a1a; border-bottom: 2px solid #10b981; padding-bottom: 0.5rem; }}
        .info {{ background: #ecfdf5; padding: 1rem 1.5rem; border-radius: 8px; border-left: 4px solid #10b981; margin: 1.5rem 0; }}
        .info h3 {{ margin-top: 0; color: #065f46; }}
        .info code {{ background: #d1fae5; padding: 2px 6px; border-radius: 4px; }}
        pre {{ background: #1e293b; color: #e2e8f0; padding: 1rem; border-radius: 6px; overflow-x: auto; font-size: 0.85rem; }}
        .example {{ background: white; padding: 1rem 1.5rem; margin: 1rem 0; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
        .example h3 {{ color: #1f2937; margin-top: 0; font-size: 1rem; }}
    </style>
</head>
<body>
    <h1>BGQL Server</h1>
    <p>Schema-first GraphQL server powered by <code>bgql_sdk</code>.</p>

    <div class="info">
        <h3>Endpoints</h3>
        <ul>
            <li><code>POST {endpoint}</code> - GraphQL queries and mutations</li>
            <li><code>GET {endpoint}</code> - This playground</li>
            <li><code>GET /health</code> - Health check</li>
        </ul>
    </div>

    <h2>Example</h2>
    <div class="example">
        <pre>curl -s http://localhost:4000{endpoint} \
  -H "Content-Type: application/json" \
  -d '{{\"query\": \"{{ __typename }}\"}}' | jq</pre>
    </div>
</body>
</html>"#,
        endpoint = endpoint
    )
}

pub(crate) fn well_known_bgql(config: &ServerConfig) -> String {
    serde_json::json!({
        "version": "1.0",
        "endpoints": {
            "graphql": "/bgql",
            "health": "/health"
        },
        "features": {
            "introspection": config.introspection,
            "playground": config.playground
        },
        "limits": {
            "maxDepth": config.max_depth,
            "maxComplexity": config.max_complexity
        }
    })
    .to_string()
}

pub(crate) fn health_response() -> &'static str {
    r#"{"status":"healthy"}"#
}

/// Starts the HTTP server.
pub(crate) async fn run_server(server: Arc<BgqlServer>) -> SdkResult<()> {
    let config = server.config();
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e| crate::error::SdkError::server(format!("Invalid address: {}", e)))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::SdkError::server(format!("Failed to bind: {}", e)))?;

    println!();
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║                    BGQL Server                         ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();
    info!("Listening on http://{}", addr);
    if config.playground {
        info!("Playground: http://{}/bgql", addr);
    }
    println!();

    // Process requests sequentially to avoid Send requirement on BgqlServer
    // TODO: Make BgqlServer Send+Sync for parallel request processing
    loop {
        let (stream, _addr) = listener
            .accept()
            .await
            .map_err(|e| crate::error::SdkError::server(format!("Failed to accept: {}", e)))?;

        let io = TokioIo::new(stream);
        let server_ref = &server;

        let service = service_fn(|req: Request<Incoming>| {
            let config = server_ref.config();
            async move {
                let (parts, body) = req.into_parts();

                let response: Response<BoxBody> = match (parts.method.clone(), parts.uri.path()) {
                    (Method::GET, "/health") => Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .body(full(health_response()))
                        .unwrap(),

                    (Method::GET, "/.well-known/bgql") => Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .body(full(well_known_bgql(config)))
                        .unwrap(),

                    (Method::POST, "/bgql") => {
                        let body_bytes = body
                            .collect()
                            .await
                            .map(|c| c.to_bytes())
                            .unwrap_or_default();
                        handle_graphql_request(body_bytes, server_ref).await
                    }

                    (Method::GET, "/bgql") | (Method::GET, "/") if config.playground => {
                        Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "text/html; charset=utf-8")
                            .body(full(playground_html("/bgql")))
                            .unwrap()
                    }

                    (Method::OPTIONS, "/bgql") => Response::builder()
                        .status(StatusCode::OK)
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                        .header(
                            "Access-Control-Allow-Headers",
                            "Content-Type, Authorization",
                        )
                        .body(full(""))
                        .unwrap(),

                    _ => Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header("Content-Type", "application/json")
                        .body(full(r#"{"error":"Not Found"}"#))
                        .unwrap(),
                };

                Ok::<_, Infallible>(response)
            }
        });

        if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
            if !err.to_string().contains("connection closed") {
                error!("Connection error: {:?}", err);
            }
        }
    }
}
