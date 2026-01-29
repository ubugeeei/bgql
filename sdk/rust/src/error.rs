//! Strongly typed error system for Better GraphQL SDK.
//!
//! Provides compile-time guarantees for error handling with typed error codes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// Typed error codes for compile-time safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ErrorCode {
    // Network errors
    NetworkError,
    Timeout,
    ConnectionRefused,
    DnsResolution,

    // Protocol errors
    HttpError,
    HttpsNotSupported,
    InvalidUrl,
    InvalidResponse,

    // GraphQL errors
    ParseError,
    ValidationError,
    ExecutionError,
    NoOperation,
    NoData,

    // Schema errors
    SchemaError,
    NoSchema,
    TypeNotFound,
    FieldNotFound,

    // Resolver errors
    ResolverError,
    ResolverNotFound,
    ResolverTimeout,

    // Serialization errors
    SerializeError,
    DeserializeError,

    // Auth errors
    AuthError,
    Unauthorized,
    Forbidden,

    // Resource errors
    NotFound,
    Conflict,

    // Internal errors
    InternalError,
    PlanError,

    // Custom error (escape hatch, but tracked)
    Custom,
}

impl ErrorCode {
    /// Returns the string representation of the error code.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NetworkError => "NETWORK_ERROR",
            Self::Timeout => "TIMEOUT",
            Self::ConnectionRefused => "CONNECTION_REFUSED",
            Self::DnsResolution => "DNS_RESOLUTION",
            Self::HttpError => "HTTP_ERROR",
            Self::HttpsNotSupported => "HTTPS_NOT_SUPPORTED",
            Self::InvalidUrl => "INVALID_URL",
            Self::InvalidResponse => "INVALID_RESPONSE",
            Self::ParseError => "PARSE_ERROR",
            Self::ValidationError => "VALIDATION_ERROR",
            Self::ExecutionError => "EXECUTION_ERROR",
            Self::NoOperation => "NO_OPERATION",
            Self::NoData => "NO_DATA",
            Self::SchemaError => "SCHEMA_ERROR",
            Self::NoSchema => "NO_SCHEMA",
            Self::TypeNotFound => "TYPE_NOT_FOUND",
            Self::FieldNotFound => "FIELD_NOT_FOUND",
            Self::ResolverError => "RESOLVER_ERROR",
            Self::ResolverNotFound => "RESOLVER_NOT_FOUND",
            Self::ResolverTimeout => "RESOLVER_TIMEOUT",
            Self::SerializeError => "SERIALIZE_ERROR",
            Self::DeserializeError => "DESERIALIZE_ERROR",
            Self::AuthError => "AUTH_ERROR",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::InternalError => "INTERNAL_ERROR",
            Self::PlanError => "PLAN_ERROR",
            Self::Custom => "CUSTOM",
        }
    }

    /// Returns true if this is a retryable error.
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::NetworkError | Self::Timeout | Self::ConnectionRefused | Self::DnsResolution
        )
    }

    /// Returns true if this is a client error (4xx equivalent).
    pub const fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::ParseError
                | Self::ValidationError
                | Self::AuthError
                | Self::Unauthorized
                | Self::Forbidden
                | Self::NotFound
                | Self::Conflict
                | Self::InvalidUrl
                | Self::NoOperation
        )
    }

    /// Returns true if this is a server error (5xx equivalent).
    pub const fn is_server_error(&self) -> bool {
        matches!(
            self,
            Self::InternalError
                | Self::ExecutionError
                | Self::ResolverError
                | Self::ResolverTimeout
        )
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Strongly typed SDK error.
#[derive(Error, Debug, Clone)]
#[error("[{code}] {message}")]
pub struct SdkError {
    /// Typed error code.
    pub code: ErrorCode,
    /// Human-readable error message.
    pub message: String,
    /// Additional structured details.
    #[source]
    pub source: Option<Box<SdkError>>,
    /// Extension data for debugging.
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

impl SdkError {
    /// Creates a new error with the given code and message.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
            extensions: None,
        }
    }

    /// Adds a source error.
    pub fn with_source(mut self, source: SdkError) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Adds extension data.
    pub fn with_extension(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        let extensions = self.extensions.get_or_insert_with(HashMap::new);
        if let Ok(v) = serde_json::to_value(value) {
            extensions.insert(key.into(), v);
        }
        self
    }

    /// Adds multiple extensions.
    pub fn with_extensions(mut self, ext: HashMap<String, serde_json::Value>) -> Self {
        self.extensions = Some(ext);
        self
    }

    // Convenience constructors

    /// Creates a network error.
    pub fn network(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NetworkError, message)
    }

    /// Creates a timeout error.
    pub fn timeout() -> Self {
        Self::new(ErrorCode::Timeout, "Request timed out")
    }

    /// Creates a parse error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ParseError, message)
    }

    /// Creates a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationError, message)
    }

    /// Creates an auth error.
    pub fn auth(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthError, message)
    }

    /// Creates a not found error.
    pub fn not_found(resource: impl fmt::Display) -> Self {
        Self::new(ErrorCode::NotFound, format!("{} not found", resource))
    }

    /// Creates a serialization error.
    pub fn serialize(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::SerializeError, message)
    }

    /// Creates a deserialization error.
    pub fn deserialize(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DeserializeError, message)
    }

    /// Creates an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// Creates a server error.
    pub fn server(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        self.code.is_retryable()
    }

    /// Returns true if this is a client error.
    pub fn is_client_error(&self) -> bool {
        self.code.is_client_error()
    }

    /// Returns true if this is a server error.
    pub fn is_server_error(&self) -> bool {
        self.code.is_server_error()
    }
}

impl Serialize for SdkError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("SdkError", 3)?;
        state.serialize_field("code", &self.code)?;
        state.serialize_field("message", &self.message)?;
        if let Some(ref ext) = self.extensions {
            state.serialize_field("extensions", ext)?;
        }
        state.end()
    }
}

/// Type alias for SDK results.
pub type SdkResult<T> = std::result::Result<T, SdkError>;

/// Extension trait for converting other errors to SdkError.
pub trait IntoSdkError {
    fn into_sdk_error(self, code: ErrorCode) -> SdkError;
}

impl<E: std::error::Error> IntoSdkError for E {
    fn into_sdk_error(self, code: ErrorCode) -> SdkError {
        SdkError::new(code, self.to_string())
    }
}

/// Result extension for mapping errors with context.
pub trait ResultExt<T> {
    /// Maps the error to an SdkError with the given code.
    fn map_sdk_err(self, code: ErrorCode) -> SdkResult<T>;

    /// Maps the error to an SdkError with the given code and message.
    fn map_sdk_err_with(self, code: ErrorCode, message: impl Into<String>) -> SdkResult<T>;
}

impl<T, E: std::error::Error> ResultExt<T> for std::result::Result<T, E> {
    fn map_sdk_err(self, code: ErrorCode) -> SdkResult<T> {
        self.map_err(|e| e.into_sdk_error(code))
    }

    fn map_sdk_err_with(self, code: ErrorCode, message: impl Into<String>) -> SdkResult<T> {
        self.map_err(|e| {
            SdkError::new(code, message).with_extension("original_error", e.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_properties() {
        assert!(ErrorCode::Timeout.is_retryable());
        assert!(!ErrorCode::ParseError.is_retryable());

        assert!(ErrorCode::ValidationError.is_client_error());
        assert!(!ErrorCode::InternalError.is_client_error());

        assert!(ErrorCode::ExecutionError.is_server_error());
        assert!(!ErrorCode::NotFound.is_server_error());
    }

    #[test]
    fn test_error_construction() {
        let err =
            SdkError::new(ErrorCode::NotFound, "User not found").with_extension("user_id", "123");

        assert_eq!(err.code, ErrorCode::NotFound);
        assert_eq!(err.message, "User not found");
        assert!(err.extensions.is_some());
    }

    #[test]
    fn test_error_serialization() {
        let err = SdkError::network("Connection failed");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("NETWORK_ERROR"));
        assert!(json.contains("Connection failed"));
    }

    #[test]
    fn test_result_ext() {
        let result: Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));

        let sdk_result = result.map_sdk_err(ErrorCode::NotFound);
        assert!(sdk_result.is_err());
        assert_eq!(sdk_result.unwrap_err().code, ErrorCode::NotFound);
    }
}
