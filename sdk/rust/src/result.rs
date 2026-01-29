//! Result type for Better GraphQL SDK.
//!
//! Uses the "errors as values" pattern instead of exceptions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A Result type that represents either success (Ok) or failure (Err).
#[derive(Debug, Clone)]
pub enum Result<T, E = BgqlError> {
    Ok(T),
    Err(E),
}

/// Alias for Ok variant.
pub type Ok<T> = Result<T, std::convert::Infallible>;

/// Alias for Err variant.
pub type Err<E> = Result<std::convert::Infallible, E>;

impl<T, E> Result<T, E> {
    /// Returns true if the result is Ok.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    /// Returns true if the result is Err.
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    /// Converts to std::result::Result.
    pub fn into_std(self) -> std::result::Result<T, E> {
        match self {
            Self::Ok(v) => std::result::Result::Ok(v),
            Self::Err(e) => std::result::Result::Err(e),
        }
    }

    /// Maps the Ok value.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Result<U, E> {
        match self {
            Self::Ok(v) => Result::Ok(f(v)),
            Self::Err(e) => Result::Err(e),
        }
    }

    /// Maps the Err value.
    pub fn map_err<F, G: FnOnce(E) -> F>(self, f: G) -> Result<T, F> {
        match self {
            Self::Ok(v) => Result::Ok(v),
            Self::Err(e) => Result::Err(f(e)),
        }
    }

    /// Unwraps the Ok value, panicking if Err.
    pub fn unwrap(self) -> T
    where
        E: std::fmt::Debug,
    {
        match self {
            Self::Ok(v) => v,
            Self::Err(e) => panic!("called unwrap on Err: {:?}", e),
        }
    }

    /// Returns the Ok value or a default.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Ok(v) => v,
            Self::Err(_) => default,
        }
    }

    /// Returns the Ok value or computes it from a closure.
    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, f: F) -> T {
        match self {
            Self::Ok(v) => v,
            Self::Err(e) => f(e),
        }
    }
}

impl<T, E> From<std::result::Result<T, E>> for Result<T, E> {
    fn from(r: std::result::Result<T, E>) -> Self {
        match r {
            std::result::Result::Ok(v) => Self::Ok(v),
            std::result::Result::Err(e) => Self::Err(e),
        }
    }
}

/// Type alias for BgqlResult (uses std::result::Result for `?` operator support).
pub type BgqlResult<T> = std::result::Result<T, BgqlError>;

/// A Better GraphQL error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgqlError {
    /// Error code.
    pub code: String,
    /// Error message.
    pub message: String,
    /// Additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
    /// Cause of the error.
    #[serde(skip)]
    pub cause: Option<Box<BgqlError>>,
}

impl BgqlError {
    /// Creates a new error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            cause: None,
        }
    }

    /// Adds details to the error.
    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        self.details = Some(details);
        self
    }

    /// Adds a cause to the error.
    pub fn with_cause(mut self, cause: BgqlError) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    // Common error constructors

    /// Creates a network error.
    pub fn network(message: impl Into<String>) -> Self {
        Self::new("NETWORK_ERROR", message)
    }

    /// Creates a parse error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::new("PARSE_ERROR", message)
    }

    /// Creates a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new("VALIDATION_ERROR", message)
    }

    /// Creates an authentication error.
    pub fn auth(message: impl Into<String>) -> Self {
        Self::new("AUTH_ERROR", message)
    }

    /// Creates a not found error.
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", format!("{} not found", resource.into()))
    }

    /// Creates a timeout error.
    pub fn timeout() -> Self {
        Self::new("TIMEOUT", "Request timed out")
    }
}

impl std::fmt::Display for BgqlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for BgqlError {}
