//! Integration tests for streaming execution.

use bgql_runtime::{
    binary_transport::{BinaryChunk, ChunkFlags},
    streaming::{DeferPayload, IncrementalEvent, PathSegment, StreamPayload, StreamingResponse},
};

/// Test streaming response creation.
#[test]
fn test_streaming_response() {
    let initial_data = serde_json::json!({
        "user": {
            "id": "1",
            "name": "Alice"
        }
    });

    let response = StreamingResponse::new(initial_data.clone()).with_pending();

    assert_eq!(response.initial, initial_data);
    assert!(response.has_next);
}

/// Test defer payload creation.
#[test]
fn test_defer_payload() {
    let defer = DeferPayload {
        label: Some("profile".to_string()),
        path: vec![PathSegment::from("user"), PathSegment::from("profile")],
        data: serde_json::json!({
            "bio": "Software Engineer",
            "location": "Tokyo"
        }),
        errors: None,
        has_next: true,
    };

    assert_eq!(defer.label, Some("profile".to_string()));
    assert!(defer.has_next);
    assert!(defer.errors.is_none());
}

/// Test stream payload accumulation.
#[test]
fn test_stream_payload() {
    let payload = StreamPayload {
        label: Some("feed".to_string()),
        path: vec![PathSegment::from("feed")],
        items: vec![
            serde_json::json!({"id": 1, "content": "Post 1"}),
            serde_json::json!({"id": 2, "content": "Post 2"}),
        ],
        errors: None,
        has_next: true,
    };

    assert_eq!(payload.items.len(), 2);
    assert!(payload.has_next);
}

/// Test incremental events.
#[test]
fn test_incremental_events() {
    let defer_event = IncrementalEvent::Defer(DeferPayload {
        label: Some("test".to_string()),
        path: vec![PathSegment::from("data")],
        data: serde_json::json!({"value": 42}),
        errors: None,
        has_next: false,
    });

    let stream_event = IncrementalEvent::Stream(StreamPayload {
        label: Some("items".to_string()),
        path: vec![PathSegment::from("list")],
        items: vec![serde_json::json!(1), serde_json::json!(2)],
        errors: None,
        has_next: true,
    });

    let complete_event = IncrementalEvent::Complete;

    // Pattern matching works
    match defer_event {
        IncrementalEvent::Defer(d) => assert_eq!(d.data["value"], 42),
        _ => panic!("Expected defer event"),
    }

    match stream_event {
        IncrementalEvent::Stream(s) => assert_eq!(s.items.len(), 2),
        _ => panic!("Expected stream event"),
    }

    match complete_event {
        IncrementalEvent::Complete => {}
        _ => panic!("Expected complete event"),
    }
}

/// Test binary chunk creation.
#[test]
fn test_binary_chunk_creation() {
    let data = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    let chunk = BinaryChunk::new(42, data.clone());

    assert_eq!(chunk.sequence, 42);
    assert_eq!(chunk.payload, data);
    assert!(!chunk.flags.is_final());
    assert!(chunk.validate());
}

/// Test binary chunk with final flag.
#[test]
fn test_binary_chunk_final() {
    let data = vec![1, 2, 3];
    let chunk = BinaryChunk::final_chunk(10, data.clone());

    assert!(chunk.flags.is_final());
    assert_eq!(chunk.sequence, 10);
    assert!(chunk.validate());
}

/// Test binary chunk serialization roundtrip.
#[test]
fn test_binary_chunk_roundtrip() {
    let original = BinaryChunk::new(100, vec![10, 20, 30, 40, 50]);
    let bytes = original.to_bytes();
    let restored = BinaryChunk::from_bytes(&bytes).expect("should deserialize");

    assert_eq!(restored.sequence, original.sequence);
    assert_eq!(restored.payload, original.payload);
    assert_eq!(restored.flags.as_u8(), original.flags.as_u8());
}

/// Test path segment conversion.
#[test]
fn test_path_segment_conversion() {
    let field: PathSegment = "field_name".into();
    let index: PathSegment = 42usize.into();

    match field {
        PathSegment::Field(s) => assert_eq!(s, "field_name"),
        _ => panic!("Expected field"),
    }

    match index {
        PathSegment::Index(i) => assert_eq!(i, 42),
        _ => panic!("Expected index"),
    }
}

/// Test streaming with errors.
#[test]
fn test_streaming_with_errors() {
    let defer_with_error = DeferPayload {
        label: Some("profile".to_string()),
        path: vec![PathSegment::from("user")],
        data: serde_json::Value::Null,
        errors: Some(vec![serde_json::json!({
            "message": "User not found",
            "path": ["user", "profile"]
        })]),
        has_next: false,
    };

    assert!(defer_with_error.errors.is_some());
    assert_eq!(defer_with_error.errors.as_ref().unwrap().len(), 1);
}

/// Test chunk flags.
#[test]
fn test_chunk_flags() {
    let flags = ChunkFlags::new();
    assert!(!flags.is_final());
    assert!(!flags.is_error());

    let final_flags = ChunkFlags::new().with_final();
    assert!(final_flags.is_final());

    let error_flags = ChunkFlags::new().with_error();
    assert!(error_flags.is_error());

    let combined = ChunkFlags::new().with_final().with_metadata();
    assert!(combined.is_final());
    assert!(combined.is_metadata());
}

/// Test error chunk creation.
#[test]
fn test_error_chunk() {
    let chunk = BinaryChunk::error_chunk(1, "Something went wrong");

    assert!(chunk.flags.is_error());
    assert_eq!(
        String::from_utf8_lossy(&chunk.payload),
        "Something went wrong"
    );
}
