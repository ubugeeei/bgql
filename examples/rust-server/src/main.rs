//! Better GraphQL Example Server
//!
//! A schema-first BGQL server demonstrating:
//! - Schema as the source of truth (schema.bgql)
//! - Type-safe resolvers using bgql_sdk
//! - DataLoader from bgql_sdk for N+1 prevention
//! - Typed errors
//! - HTTP API (curl-friendly)
//!
//! # Running the server
//!
//! ```bash
//! cd examples/rust-server
//! cargo run --release
//! ```
//!
//! # Example curl commands
//!
//! ```bash
//! # Query a user
//! curl -X POST http://localhost:4000/graphql \
//!   -H "Content-Type: application/json" \
//!   -d '{"query": "{ user(id: \"user_1\") { ... on User { id name } } }"}'
//!
//! # Query all posts
//! curl -X POST http://localhost:4000/graphql \
//!   -H "Content-Type: application/json" \
//!   -d '{"query": "{ posts { edges { node { id title } } } }"}'
//! ```

mod db;

use db::{Database, PostStatus};

use bgql_sdk::server::{BgqlServer, Context, ServerConfig};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task::LocalSet;

// =============================================================================
// GraphQL Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
struct GraphQLRequest {
    query: String,
    #[serde(default)]
    variables: Option<serde_json::Value>,
    #[serde(rename = "operationName")]
    #[allow(dead_code)]
    operation_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct GraphQLResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Serialize)]
struct GraphQLError {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<Vec<String>>,
}

// =============================================================================
// Server Builder
// =============================================================================

/// Creates the BGQL server with resolvers.
/// Uses bgql_sdk's BgqlServer builder pattern.
fn create_bgql_server(db: Arc<Database>) -> BgqlServer {
    let schema = include_str!("../schema.bgql");

    // Clone db for each resolver
    let db_for_user = db.clone();
    let db_for_users = db.clone();
    let db_for_post = db.clone();
    let db_for_posts = db.clone();
    let db_for_create = db.clone();
    let db_for_update = db.clone();
    let db_for_delete = db.clone();
    let db_for_publish = db.clone();

    BgqlServer::builder()
        .config(
            ServerConfig::new()
                .port(4000)
                .host("0.0.0.0"),
        )
        .schema_sdl(schema)

        // =====================================================================
        // Query Resolvers
        // =====================================================================

        // Query.user(id: UserId): UserResult
        .resolver("Query", "user", move |args, _ctx| {
            let db = db_for_user.clone();
            async move {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                match db.get_user(id).await {
                    Some(user) => {
                        let posts = db.get_posts_by_author(id, None).await;
                        let posts_count = posts.len();
                        let posts_json: Vec<serde_json::Value> = posts
                            .into_iter()
                            .map(|p| serde_json::json!({
                                "id": p.id,
                                "title": p.title,
                                "status": p.status.to_string()
                            }))
                            .collect();

                        Ok(serde_json::json!({
                            "__typename": "User",
                            "id": user.id,
                            "name": user.name,
                            "email": user.email,
                            "bio": user.bio,
                            "createdAt": user.created_at.to_rfc3339(),
                            "posts": posts_json,
                            "postsCount": posts_count
                        }))
                    }
                    None => Ok(serde_json::json!({
                        "__typename": "NotFoundError",
                        "message": format!("User '{}' not found", id),
                        "code": "NOT_FOUND",
                        "resourceType": "User",
                        "resourceId": id
                    })),
                }
            }
        })

        // Query.users(first: Int, after: String): UserConnection
        .resolver("Query", "users", move |args, _ctx| {
            let db = db_for_users.clone();
            async move {
                let first = args.get("first")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(10) as usize;
                let after = args.get("after")
                    .and_then(|v| v.as_str());

                let (users, has_next) = db.get_users(first, after).await;
                let total = db.get_user_count().await;

                let edges: Vec<serde_json::Value> = users
                    .iter()
                    .map(|u| serde_json::json!({
                        "cursor": u.id,
                        "node": {
                            "id": u.id,
                            "name": u.name,
                            "email": u.email,
                            "bio": u.bio,
                            "createdAt": u.created_at.to_rfc3339()
                        }
                    }))
                    .collect();

                let start_cursor = users.first().map(|u| u.id.clone());
                let end_cursor = users.last().map(|u| u.id.clone());

                Ok(serde_json::json!({
                    "edges": edges,
                    "pageInfo": {
                        "hasNextPage": has_next,
                        "hasPreviousPage": after.is_some(),
                        "startCursor": start_cursor,
                        "endCursor": end_cursor
                    },
                    "totalCount": total
                }))
            }
        })

        // Query.post(id: PostId): PostResult
        .resolver("Query", "post", move |args, _ctx| {
            let db = db_for_post.clone();
            async move {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                match db.get_post(id).await {
                    Some(post) => {
                        let author = db.get_user(&post.author_id).await;
                        Ok(serde_json::json!({
                            "__typename": "Post",
                            "id": post.id,
                            "title": post.title,
                            "content": post.content,
                            "status": post.status.to_string(),
                            "authorId": post.author_id,
                            "author": author.map(|a| serde_json::json!({
                                "id": a.id,
                                "name": a.name,
                                "email": a.email
                            })),
                            "createdAt": post.created_at.to_rfc3339(),
                            "updatedAt": post.updated_at.map(|t| t.to_rfc3339())
                        }))
                    }
                    None => Ok(serde_json::json!({
                        "__typename": "NotFoundError",
                        "message": format!("Post '{}' not found", id),
                        "code": "NOT_FOUND",
                        "resourceType": "Post",
                        "resourceId": id
                    })),
                }
            }
        })

        // Query.posts(first: Int, after: String, filter: PostFilter): PostConnection
        .resolver("Query", "posts", move |args, _ctx| {
            let db = db_for_posts.clone();
            async move {
                let first = args.get("first")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(10) as usize;
                let after = args.get("after")
                    .and_then(|v| v.as_str());

                let filter = args.get("filter");
                let status = filter
                    .and_then(|f| f.get("status"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "Draft" => Some(PostStatus::Draft),
                        "Published" => Some(PostStatus::Published),
                        "Archived" => Some(PostStatus::Archived),
                        _ => None,
                    });
                let author_id = filter
                    .and_then(|f| f.get("authorId"))
                    .and_then(|v| v.as_str());

                let (posts, has_next) = db.get_posts(first, after, status, author_id).await;
                let total = db.get_post_count(status, author_id).await;

                // Use DataLoader pattern: batch load all authors
                let author_ids: Vec<String> = posts.iter()
                    .map(|p| p.author_id.clone())
                    .collect();
                let authors = db.get_users_by_ids(author_ids).await;

                let edges: Vec<serde_json::Value> = posts
                    .iter()
                    .map(|p| {
                        let author = authors.get(&p.author_id);
                        serde_json::json!({
                            "cursor": p.id,
                            "node": {
                                "id": p.id,
                                "title": p.title,
                                "content": p.content,
                                "status": p.status.to_string(),
                                "authorId": p.author_id,
                                "author": author.map(|a| serde_json::json!({
                                    "id": a.id,
                                    "name": a.name
                                })),
                                "createdAt": p.created_at.to_rfc3339()
                            }
                        })
                    })
                    .collect();

                let start_cursor = posts.first().map(|p| p.id.clone());
                let end_cursor = posts.last().map(|p| p.id.clone());

                Ok(serde_json::json!({
                    "edges": edges,
                    "pageInfo": {
                        "hasNextPage": has_next,
                        "hasPreviousPage": after.is_some(),
                        "startCursor": start_cursor,
                        "endCursor": end_cursor
                    },
                    "totalCount": total
                }))
            }
        })

        // =====================================================================
        // Mutation Resolvers
        // =====================================================================

        // Mutation.createPost(input: CreatePostInput): CreatePostResult
        .resolver("Mutation", "createPost", move |args, _ctx| {
            let db = db_for_create.clone();
            async move {
                let input = args.get("input").cloned().unwrap_or(serde_json::Value::Null);

                let title = input.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let content = input.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if title.is_empty() {
                    return Ok(serde_json::json!({
                        "__typename": "ValidationError",
                        "message": "Title is required",
                        "code": "VALIDATION_ERROR",
                        "field": "title"
                    }));
                }

                let status = input.get("status")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "Draft" => Some(PostStatus::Draft),
                        "Published" => Some(PostStatus::Published),
                        _ => None,
                    })
                    .unwrap_or(PostStatus::Draft);

                let post = db.create_post(
                    "user_1".to_string(), // In real app, get from auth context
                    title.to_string(),
                    content.to_string(),
                    status,
                ).await;

                Ok(serde_json::json!({
                    "__typename": "Post",
                    "id": post.id,
                    "title": post.title,
                    "content": post.content,
                    "status": post.status.to_string(),
                    "createdAt": post.created_at.to_rfc3339()
                }))
            }
        })

        // Mutation.updatePost(id: PostId, input: UpdatePostInput): UpdatePostResult
        .resolver("Mutation", "updatePost", move |args, _ctx| {
            let db = db_for_update.clone();
            async move {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let input = args.get("input").cloned().unwrap_or(serde_json::Value::Null);
                let title = input.get("title").and_then(|v| v.as_str()).map(String::from);
                let content = input.get("content").and_then(|v| v.as_str()).map(String::from);
                let status = input.get("status")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "Draft" => Some(PostStatus::Draft),
                        "Published" => Some(PostStatus::Published),
                        "Archived" => Some(PostStatus::Archived),
                        _ => None,
                    });

                match db.update_post(id, title, content, status).await {
                    Some(post) => Ok(serde_json::json!({
                        "__typename": "Post",
                        "id": post.id,
                        "title": post.title,
                        "status": post.status.to_string()
                    })),
                    None => Ok(serde_json::json!({
                        "__typename": "NotFoundError",
                        "message": format!("Post '{}' not found", id),
                        "code": "NOT_FOUND"
                    })),
                }
            }
        })

        // Mutation.deletePost(id: PostId): DeleteResult
        .resolver("Mutation", "deletePost", move |args, _ctx| {
            let db = db_for_delete.clone();
            async move {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if db.delete_post(id).await {
                    Ok(serde_json::json!({
                        "__typename": "DeleteSuccess",
                        "success": true,
                        "deletedId": id
                    }))
                } else {
                    Ok(serde_json::json!({
                        "__typename": "NotFoundError",
                        "message": format!("Post '{}' not found", id),
                        "code": "NOT_FOUND"
                    }))
                }
            }
        })

        // Mutation.publishPost(id: PostId): UpdatePostResult
        .resolver("Mutation", "publishPost", move |args, _ctx| {
            let db = db_for_publish.clone();
            async move {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                match db.update_post(id, None, None, Some(PostStatus::Published)).await {
                    Some(post) => Ok(serde_json::json!({
                        "__typename": "Post",
                        "id": post.id,
                        "title": post.title,
                        "status": post.status.to_string()
                    })),
                    None => Ok(serde_json::json!({
                        "__typename": "NotFoundError",
                        "message": format!("Post '{}' not found", id),
                        "code": "NOT_FOUND"
                    }))
                }
            }
        })

        .build()
        .expect("Failed to build BGQL server")
}

// =============================================================================
// HTTP Handler (until SDK provides built-in HTTP server)
// =============================================================================
//
// Note: bgql_sdk::server::BgqlServer has a listen() method but it's currently
// a TODO stub. This HTTP layer will be removed once the SDK provides it.

type BoxBody = http_body_util::combinators::BoxBody<bytes::Bytes, hyper::Error>;

fn full<T: Into<bytes::Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

async fn handle_graphql_request(
    body_bytes: bytes::Bytes,
    server: &BgqlServer,
) -> Response<BoxBody> {
    let gql_request: GraphQLRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            let error_response = GraphQLResponse {
                data: None,
                errors: Some(vec![GraphQLError {
                    message: format!("Invalid JSON: {}", e),
                    path: None,
                }]),
            };
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(full(serde_json::to_string(&error_response).unwrap()))
                .unwrap();
        }
    };

    // Execute using bgql_sdk
    let ctx = Context::new();
    let result = server.execute(
        &gql_request.query,
        gql_request.variables,
        ctx,
    ).await;

    let response = match result {
        Ok(data) => {
            let data_value = data.get("data").cloned();
            let errors_value = data.get("errors")
                .and_then(|e| e.as_array())
                .map(|arr| arr.iter().map(|e| GraphQLError {
                    message: e.as_str().unwrap_or("Unknown error").to_string(),
                    path: None,
                }).collect());

            GraphQLResponse {
                data: data_value,
                errors: errors_value,
            }
        }
        Err(e) => {
            GraphQLResponse {
                data: None,
                errors: Some(vec![GraphQLError {
                    message: e.to_string(),
                    path: None,
                }]),
            }
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(full(serde_json::to_string(&response).unwrap()))
        .unwrap()
}

fn playground_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>BGQL Playground</title>
    <style>
        body { font-family: sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }
        h1 { color: #333; }
        .example { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
        code { background: #e0e0e0; padding: 2px 6px; border-radius: 3px; }
        pre { background: #2d2d2d; color: #f8f8f2; padding: 15px; border-radius: 5px; overflow-x: auto; font-size: 12px; }
    </style>
</head>
<body>
    <h1>BGQL Example Server</h1>
    <p>Schema-first BGQL server using <code>bgql_sdk</code>. POST to <code>/graphql</code>.</p>

    <h2>Example Queries</h2>

    <div class="example">
        <h3>Get a user</h3>
        <pre>curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ user(id: \"user_1\") { ... on User { id name posts { title } } } }"}' | jq</pre>
    </div>

    <div class="example">
        <h3>List users</h3>
        <pre>curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ users { edges { node { id name } } totalCount } }"}' | jq</pre>
    </div>

    <div class="example">
        <h3>List posts with authors</h3>
        <pre>curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ posts { edges { node { id title author { name } } } } }"}' | jq</pre>
    </div>

    <div class="example">
        <h3>Create a post</h3>
        <pre>curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "mutation { createPost(input: { title: \"Test\", content: \"Hello\" }) { ... on Post { id } } }"}' | jq</pre>
    </div>

    <div class="example">
        <h3>Publish a post</h3>
        <pre>curl -s http://localhost:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "mutation { publishPost(id: \"post_4\") { ... on Post { title status } } }"}' | jq</pre>
    </div>
</body>
</html>"#
}

// =============================================================================
// Main Entry Point
// =============================================================================

fn main() {
    println!("=================================================");
    println!("  BGQL Example Server - Schema-First GraphQL");
    println!("=================================================");
    println!();

    // Load schema (demonstrates schema-first approach)
    let schema_content = include_str!("../schema.bgql");
    println!("[schema] Loaded schema.bgql ({} bytes)", schema_content.len());
    println!();

    // Initialize database
    let db = Arc::new(Database::new_with_sample_data());
    println!("[db] Sample data: 3 users, 5 posts");
    println!();

    // Demonstrate DataLoader from SDK
    println!("[sdk] Using bgql_sdk::server::create_loader for DataLoader");
    println!("[sdk] Using bgql_sdk::server::BgqlServer for resolvers");
    println!();

    // Create BGQL server using the SDK
    let _server = create_bgql_server(db.clone());
    println!("[bgql] Server built with schema and resolvers");
    println!();

    // Note about HTTP layer
    println!("[http] Note: Using hyper for HTTP until SDK provides listen()");
    println!("       The SDK's BgqlServer.listen() is currently a TODO stub.");
    println!();

    // Run with single-threaded runtime (required because BgqlServer is not Send+Sync)
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let local = LocalSet::new();

    local.block_on(&rt, async move {
        let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
        let listener = TcpListener::bind(addr).await.expect("Failed to bind");

        println!("[http] Listening on http://{}", addr);
        println!();
        println!("Try: curl -s http://localhost:4000/graphql \\");
        println!("       -H 'Content-Type: application/json' \\");
        println!("       -d '{{\"query\": \"{{ users {{ totalCount }} }}\"}}' | jq");
        println!();
        println!("=================================================");

        loop {
            let (stream, _) = listener.accept().await.expect("Failed to accept");
            let io = TokioIo::new(stream);

            // Clone db for each connection
            let db_for_conn = db.clone();

            // Spawn locally (single-threaded) because BgqlServer is not Send
            tokio::task::spawn_local(async move {
                let db_for_service = db_for_conn.clone();
                let service = service_fn(move |req: Request<Incoming>| {
                    let db_for_req = db_for_service.clone();
                    async move {
                        let server = create_bgql_server(db_for_req);
                        let (parts, body) = req.into_parts();

                        let response: Response<BoxBody> = match (parts.method, parts.uri.path()) {
                            (Method::GET, "/health") => {
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .header("Content-Type", "application/json")
                                    .body(full(r#"{"status":"healthy"}"#))
                                    .unwrap()
                            }

                            (Method::POST, "/graphql") => {
                                let body_bytes = body.collect().await
                                    .map(|c| c.to_bytes())
                                    .unwrap_or_default();
                                handle_graphql_request(body_bytes, &server).await
                            }

                            (Method::OPTIONS, "/graphql") => {
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .header("Access-Control-Allow-Origin", "*")
                                    .header("Access-Control-Allow-Methods", "POST, OPTIONS")
                                    .header("Access-Control-Allow-Headers", "Content-Type")
                                    .body(full(""))
                                    .unwrap()
                            }

                            (Method::GET, "/graphql" | "/") => {
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .header("Content-Type", "text/html")
                                    .body(full(playground_html()))
                                    .unwrap()
                            }

                            _ => {
                                Response::builder()
                                    .status(StatusCode::NOT_FOUND)
                                    .header("Content-Type", "application/json")
                                    .body(full(r#"{"error":"Not Found"}"#))
                                    .unwrap()
                            }
                        };

                        Ok::<_, Infallible>(response)
                    }
                });

                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                {
                    eprintln!("Error: {:?}", err);
                }
            });
        }
    });
}
