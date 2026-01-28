//! Query execution for Better GraphQL.

use crate::query::QueryPlan;
use crate::schema::Schema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Executor configuration.
#[derive(Debug, Clone, Default)]
pub struct ExecutorConfig {
    /// Maximum parallel execution depth.
    pub max_parallel_depth: usize,
    /// Enable tracing.
    pub tracing: bool,
}

/// The query executor.
#[derive(Debug)]
pub struct Executor {
    #[allow(dead_code)]
    config: ExecutorConfig,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    /// Creates a new executor.
    pub fn new() -> Self {
        Self {
            config: ExecutorConfig::default(),
        }
    }

    /// Creates an executor with configuration.
    pub fn with_config(config: ExecutorConfig) -> Self {
        Self { config }
    }

    /// Executes a query plan.
    pub async fn execute(&self, _plan: &QueryPlan, _schema: &Schema, _ctx: &Context) -> Response {
        // TODO: Implement actual execution
        Response {
            data: None,
            errors: None,
        }
    }
}

/// Execution context.
#[derive(Debug)]
pub struct Context {
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
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// A GraphQL response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// The data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// The errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

/// A field error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    /// The error message.
    pub message: String,
    /// The path to the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<PathSegment>>,
    /// Error extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

/// A path segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathSegment {
    Field(String),
    Index(usize),
}

impl FieldError {
    /// Creates a new field error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
            extensions: None,
        }
    }

    /// Adds a path to the error.
    pub fn with_path(mut self, path: Vec<PathSegment>) -> Self {
        self.path = Some(path);
        self
    }
}
