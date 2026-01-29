//! Integration tests for bgql_sdk

use bgql_sdk::server::{BgqlServer, Context, ServerConfig, create_loader};
use bgql_sdk::client::ClientConfig;
use bgql_sdk::error::{ErrorCode, SdkError};
use std::time::Duration;

/// Test schema parsing and query execution
#[tokio::test]
async fn test_complete_schema_parsing() {
    let schema = r#"
        type Query {
            user(id: ID): User
            users: List<User>
        }

        type User {
            id: ID
            name: String
            email: String
            posts: List<Post>
        }

        type Post {
            id: ID
            title: String
            content: String
            author: User
        }

        type Mutation {
            createUser(name: String, email: String): User
        }
    "#;

    let server = BgqlServer::builder()
        .schema_sdl(schema)
        .resolver("Query", "user", |args, _ctx| async move {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("1");
            Ok(serde_json::json!({
                "id": id,
                "name": "Alice",
                "email": "alice@example.com"
            }))
        })
        .resolver("Query", "users", |_args, _ctx| async move {
            Ok(serde_json::json!([
                {"id": "1", "name": "Alice", "email": "alice@example.com"},
                {"id": "2", "name": "Bob", "email": "bob@example.com"}
            ]))
        })
        .build();

    assert!(server.is_ok(), "Server should build successfully");
    let server = server.unwrap();

    // Test simple query
    let result = server.execute(
        "query { user(id: \"1\") { id name email } }",
        None,
        Context::new(),
    ).await;

    assert!(result.is_ok(), "Query should execute successfully");
    let data = result.unwrap();
    let user = &data["data"]["user"];
    assert_eq!(user["id"], "1");
    assert_eq!(user["name"], "Alice");
}

/// Test query with variables
#[tokio::test]
async fn test_query_with_variables() {
    let server = BgqlServer::builder()
        .schema_sdl(r#"
            type Query {
                user(id: ID): User
            }
            type User {
                id: ID
                name: String
            }
        "#)
        .resolver("Query", "user", |args, _ctx| async move {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("default");
            Ok(serde_json::json!({
                "id": id,
                "name": format!("User {}", id)
            }))
        })
        .build()
        .unwrap();

    let result = server.execute(
        "query GetUser($id: ID) { user(id: $id) { id name } }",
        Some(serde_json::json!({"id": "42"})),
        Context::new(),
    ).await;

    assert!(result.is_ok());
}

/// Test context data passing
#[tokio::test]
async fn test_context_data() {
    let mut ctx = Context::new();
    ctx.set("user_id", "123");
    ctx.set("roles", vec!["admin", "user"]);

    assert_eq!(ctx.get::<String>("user_id"), Some("123".to_string()));
    assert_eq!(
        ctx.get::<Vec<String>>("roles"),
        Some(vec!["admin".to_string(), "user".to_string()])
    );
    assert_eq!(ctx.get::<String>("missing"), None);
}

/// Test server configuration
#[tokio::test]
async fn test_server_configuration() {
    let config = ServerConfig::new()
        .port(8080)
        .host("0.0.0.0")
        .no_introspection()
        .no_playground();

    assert_eq!(config.port, 8080);
    assert_eq!(config.host, "0.0.0.0");
    assert!(!config.introspection);
    assert!(!config.playground);
    assert_eq!(config.max_depth, 10);
    assert_eq!(config.max_complexity, 1000);
}

/// Test DataLoader batching
#[tokio::test]
async fn test_dataloader_batching() {
    let loader = create_loader(|keys: Vec<String>| async move {
        keys.into_iter()
            .map(|k| {
                let value = format!("Value for {}", k);
                (k, value)
            })
            .collect()
    });

    // Load multiple values at once
    let results = loader.load_many(vec![
        "key1".to_string(),
        "key2".to_string(),
        "key3".to_string(),
    ]).await;

    assert_eq!(results.get("key1"), Some(&"Value for key1".to_string()));
    assert_eq!(results.get("key2"), Some(&"Value for key2".to_string()));
    assert_eq!(results.get("key3"), Some(&"Value for key3".to_string()));
}

/// Test client configuration
#[test]
fn test_client_configuration() {
    let config = ClientConfig::new("http://localhost:4000/bgql")
        .timeout(Duration::from_secs(60))
        .max_retries(5)
        .retry_delay_ms(200)
        .header("Authorization", "Bearer token")
        .header("X-Request-Id", "123");

    assert_eq!(config.url, "http://localhost:4000/bgql");
    assert_eq!(config.timeout, Duration::from_secs(60));
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.retry_delay_ms, 200);
    assert_eq!(config.headers.get("Authorization"), Some(&"Bearer token".to_string()));
    assert_eq!(config.headers.get("X-Request-Id"), Some(&"123".to_string()));
}

/// Test error handling
#[tokio::test]
async fn test_error_handling() {
    let server = BgqlServer::builder()
        .schema_sdl(r#"
            type Query {
                fail: String
            }
        "#)
        .build()
        .unwrap();

    // Query for a field that doesn't have a resolver should use default resolver
    let result = server.execute(
        "query { fail }",
        None,
        Context::new(),
    ).await;

    assert!(result.is_ok());
    let data = result.unwrap();
    // The default resolver returns null for missing fields
    assert!(data.get("data").is_some());
}

/// Test parse error handling
#[tokio::test]
async fn test_parse_error() {
    let server = BgqlServer::builder()
        .schema_sdl(r#"
            type Query {
                hello: String
            }
        "#)
        .build()
        .unwrap();

    // Invalid query syntax
    let result = server.execute(
        "query { hello",  // Missing closing brace
        None,
        Context::new(),
    ).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, ErrorCode::ParseError);
}

/// Test nested query execution
#[tokio::test]
async fn test_nested_query() {
    let server = BgqlServer::builder()
        .schema_sdl(r#"
            type Query {
                user: User
            }
            type User {
                id: ID
                name: String
                profile: Profile
            }
            type Profile {
                bio: String
                avatar: String
            }
        "#)
        .resolver("Query", "user", |_args, _ctx| async move {
            Ok(serde_json::json!({
                "id": "1",
                "name": "Alice",
                "profile": {
                    "bio": "Software Engineer",
                    "avatar": "https://example.com/avatar.png"
                }
            }))
        })
        .build()
        .unwrap();

    let result = server.execute(
        "query { user { id name profile { bio avatar } } }",
        None,
        Context::new(),
    ).await;

    assert!(result.is_ok());
    let data = result.unwrap();
    let user = &data["data"]["user"];
    assert_eq!(user["name"], "Alice");
    assert_eq!(user["profile"]["bio"], "Software Engineer");
}

/// Test list field execution
#[tokio::test]
async fn test_list_field() {
    let server = BgqlServer::builder()
        .schema_sdl(r#"
            type Query {
                users: List<User>
            }
            type User {
                id: ID
                name: String
            }
        "#)
        .resolver("Query", "users", |_args, _ctx| async move {
            Ok(serde_json::json!([
                {"id": "1", "name": "Alice"},
                {"id": "2", "name": "Bob"},
                {"id": "3", "name": "Charlie"}
            ]))
        })
        .build()
        .unwrap();

    let result = server.execute(
        "query { users { id name } }",
        None,
        Context::new(),
    ).await;

    assert!(result.is_ok());
    let data = result.unwrap();
    let users = data["data"]["users"].as_array().unwrap();
    assert_eq!(users.len(), 3);
    assert_eq!(users[0]["name"], "Alice");
    assert_eq!(users[1]["name"], "Bob");
    assert_eq!(users[2]["name"], "Charlie");
}

/// Test __typename introspection
#[tokio::test]
async fn test_typename_introspection() {
    let server = BgqlServer::builder()
        .schema_sdl(r#"
            type Query {
                user: User
            }
            type User {
                id: ID
                name: String
            }
        "#)
        .resolver("Query", "user", |_args, _ctx| async move {
            Ok(serde_json::json!({
                "id": "1",
                "name": "Alice"
            }))
        })
        .build()
        .unwrap();

    let result = server.execute(
        "query { user { __typename id name } }",
        None,
        Context::new(),
    ).await;

    assert!(result.is_ok());
    let data = result.unwrap();
    let user = &data["data"]["user"];
    assert_eq!(user["__typename"], "User");
    assert_eq!(user["id"], "1");
}

/// Test SdkError construction
#[test]
fn test_sdk_error() {
    let error = SdkError::new(ErrorCode::Custom, "Something went wrong");
    assert_eq!(error.code, ErrorCode::Custom);
    assert_eq!(error.message, "Something went wrong");

    let network_err = SdkError::network("Connection refused");
    assert_eq!(network_err.code, ErrorCode::NetworkError);

    let parse_err = SdkError::parse("Invalid syntax");
    assert_eq!(parse_err.code, ErrorCode::ParseError);

    let timeout_err = SdkError::timeout();
    assert_eq!(timeout_err.code, ErrorCode::Timeout);

    let not_found = SdkError::not_found("User");
    assert_eq!(not_found.code, ErrorCode::NotFound);
    assert!(not_found.message.contains("User"));

    // Test error code properties
    assert!(ErrorCode::Timeout.is_retryable());
    assert!(ErrorCode::NetworkError.is_retryable());
    assert!(!ErrorCode::ParseError.is_retryable());

    assert!(ErrorCode::ValidationError.is_client_error());
    assert!(!ErrorCode::InternalError.is_client_error());

    assert!(ErrorCode::ExecutionError.is_server_error());
}
