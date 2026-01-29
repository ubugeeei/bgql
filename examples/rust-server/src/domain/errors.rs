//! Domain errors - Business rule violations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Post not found: {0}")]
    PostNotFound(String),

    #[error("Validation error: {field} - {message}")]
    ValidationError { field: String, message: String },
}

impl DomainError {
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }
}

pub type DomainResult<T> = Result<T, DomainError>;
