//! Input validation support for Better GraphQL.
//!
//! This module provides validation primitives that correspond to BGQL's
//! validation directives:
//!
//! - `@minLength`, `@maxLength` - String length constraints
//! - `@min`, `@max` - Numeric range constraints
//! - `@email`, `@url` - Format validators
//! - `@pattern` - Regex pattern matching
//! - `@trim`, `@lowercase`, `@sanitize` - Input transformations
//!
//! # Example
//!
//! ```ignore
//! use bgql_sdk::validation::{Validate, ValidationResult, Validator};
//!
//! // Simple validation
//! let mut validator = Validator::new();
//! validator.validate_string("name", name, |v| v.min_length(2).max_length(100));
//! validator.validate_string("email", email, |v| v.email());
//! let result = validator.finish();
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// A validation error for a specific field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// The field that failed validation.
    pub field: String,
    /// Human-readable error message.
    pub message: String,
    /// Machine-readable error code.
    pub code: ValidationErrorCode,
    /// The constraint that was violated (for min/max etc).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint: Option<String>,
}

impl ValidationError {
    /// Creates a new validation error.
    pub fn new(
        field: impl Into<String>,
        code: ValidationErrorCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code,
            constraint: None,
        }
    }

    /// Sets the constraint value.
    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraint = Some(constraint.into());
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validation error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationErrorCode {
    /// Value is required but missing.
    Required,
    /// String is too short.
    MinLength,
    /// String is too long.
    MaxLength,
    /// Number is too small.
    Min,
    /// Number is too large.
    Max,
    /// Invalid email format.
    InvalidEmail,
    /// Invalid URL format.
    InvalidUrl,
    /// Pattern mismatch.
    PatternMismatch,
    /// Invalid format.
    InvalidFormat,
    /// Custom validation failed.
    Custom,
}

impl fmt::Display for ValidationErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Required => write!(f, "REQUIRED"),
            Self::MinLength => write!(f, "MIN_LENGTH"),
            Self::MaxLength => write!(f, "MAX_LENGTH"),
            Self::Min => write!(f, "MIN"),
            Self::Max => write!(f, "MAX"),
            Self::InvalidEmail => write!(f, "INVALID_EMAIL"),
            Self::InvalidUrl => write!(f, "INVALID_URL"),
            Self::PatternMismatch => write!(f, "PATTERN_MISMATCH"),
            Self::InvalidFormat => write!(f, "INVALID_FORMAT"),
            Self::Custom => write!(f, "CUSTOM"),
        }
    }
}

/// Result type for validation operations.
pub type ValidationResult<T> = Result<T, ValidationErrors>;

/// Collection of validation errors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationErrors {
    /// List of validation errors.
    pub errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Creates an empty error collection.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Creates errors from a single error.
    pub fn single(error: ValidationError) -> Self {
        Self {
            errors: vec![error],
        }
    }

    /// Adds an error.
    pub fn push(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Returns true if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns true if there are no errors.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns errors for a specific field.
    pub fn for_field(&self, field: &str) -> Vec<&ValidationError> {
        self.errors.iter().filter(|e| e.field == field).collect()
    }

    /// Merges another set of errors.
    pub fn merge(&mut self, other: ValidationErrors) {
        self.errors.extend(other.errors);
    }

    /// Converts to a GraphQL-compatible error response.
    pub fn to_graphql_error(&self) -> serde_json::Value {
        serde_json::json!({
            "__typename": "ValidationError",
            "message": self.to_string(),
            "code": "VALIDATION_ERROR",
            "field": self.errors.first().map(|e| &e.field),
            "constraint": self.errors.first().and_then(|e| e.constraint.as_ref()),
        })
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<_> = self.errors.iter().map(|e| e.to_string()).collect();
        write!(f, "{}", messages.join("; "))
    }
}

impl std::error::Error for ValidationErrors {}

impl From<ValidationError> for ValidationErrors {
    fn from(error: ValidationError) -> Self {
        Self::single(error)
    }
}

/// Trait for validatable types.
pub trait Validate {
    /// Validates the value and returns errors if invalid.
    fn validate(&self) -> ValidationResult<()>;

    /// Returns true if the value is valid.
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Builder for validating input fields.
#[derive(Debug, Default)]
pub struct Validator {
    errors: ValidationErrors,
}

impl Validator {
    /// Creates a new validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates a string field with the given checks.
    pub fn string(&mut self, name: &str, value: &str) -> StringValidator<'_> {
        StringValidator {
            errors: &mut self.errors,
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    /// Validates a numeric field with the given checks.
    pub fn number<T: NumericValue>(&mut self, name: &str, value: T) -> NumericValidator<'_, T> {
        NumericValidator {
            errors: &mut self.errors,
            name: name.to_string(),
            value,
        }
    }

    /// Adds a custom validation error.
    pub fn add_error(&mut self, field: &str, code: ValidationErrorCode, message: &str) {
        self.errors.push(ValidationError::new(field, code, message));
    }

    /// Finishes validation and returns the result.
    pub fn finish(self) -> ValidationResult<()> {
        if self.errors.has_errors() {
            Err(self.errors)
        } else {
            Ok(())
        }
    }
}

/// Validator for string fields.
pub struct StringValidator<'a> {
    errors: &'a mut ValidationErrors,
    name: String,
    value: String,
}

impl<'a> StringValidator<'a> {
    /// Validates minimum length.
    pub fn min_length(self, min: usize) -> Self {
        if self.value.len() < min {
            self.errors.push(
                ValidationError::new(
                    &self.name,
                    ValidationErrorCode::MinLength,
                    format!("must be at least {} characters", min),
                )
                .with_constraint(min.to_string()),
            );
        }
        self
    }

    /// Validates maximum length.
    pub fn max_length(self, max: usize) -> Self {
        if self.value.len() > max {
            self.errors.push(
                ValidationError::new(
                    &self.name,
                    ValidationErrorCode::MaxLength,
                    format!("must be at most {} characters", max),
                )
                .with_constraint(max.to_string()),
            );
        }
        self
    }

    /// Validates email format.
    pub fn email(self) -> Self {
        if !is_valid_email(&self.value) {
            self.errors.push(ValidationError::new(
                &self.name,
                ValidationErrorCode::InvalidEmail,
                "must be a valid email address",
            ));
        }
        self
    }

    /// Validates URL format.
    pub fn url(self) -> Self {
        if !is_valid_url(&self.value) {
            self.errors.push(ValidationError::new(
                &self.name,
                ValidationErrorCode::InvalidUrl,
                "must be a valid URL",
            ));
        }
        self
    }

    /// Validates against a regex pattern.
    pub fn pattern(self, pattern: &str) -> Self {
        if let Ok(re) = regex::Regex::new(pattern) {
            if !re.is_match(&self.value) {
                self.errors.push(
                    ValidationError::new(
                        &self.name,
                        ValidationErrorCode::PatternMismatch,
                        "does not match the required pattern",
                    )
                    .with_constraint(pattern.to_string()),
                );
            }
        }
        self
    }

    /// Validates that the value is not empty after trimming.
    pub fn required(self) -> Self {
        if self.value.trim().is_empty() {
            self.errors.push(ValidationError::new(
                &self.name,
                ValidationErrorCode::Required,
                "is required",
            ));
        }
        self
    }
}

/// Trait for numeric values.
pub trait NumericValue: PartialOrd + Copy + fmt::Display {}
impl NumericValue for i32 {}
impl NumericValue for i64 {}
impl NumericValue for u32 {}
impl NumericValue for u64 {}
impl NumericValue for f32 {}
impl NumericValue for f64 {}

/// Validator for numeric fields.
pub struct NumericValidator<'a, T: NumericValue> {
    errors: &'a mut ValidationErrors,
    name: String,
    value: T,
}

impl<'a, T: NumericValue> NumericValidator<'a, T> {
    /// Validates minimum value.
    pub fn min(self, min: T) -> Self {
        if self.value < min {
            self.errors.push(
                ValidationError::new(
                    &self.name,
                    ValidationErrorCode::Min,
                    format!("must be at least {}", min),
                )
                .with_constraint(min.to_string()),
            );
        }
        self
    }

    /// Validates maximum value.
    pub fn max(self, max: T) -> Self {
        if self.value > max {
            self.errors.push(
                ValidationError::new(
                    &self.name,
                    ValidationErrorCode::Max,
                    format!("must be at most {}", max),
                )
                .with_constraint(max.to_string()),
            );
        }
        self
    }
}

/// Input transformation helpers.
pub mod transform {
    /// Trims whitespace from a string.
    pub fn trim(s: &str) -> String {
        s.trim().to_string()
    }

    /// Converts string to lowercase.
    pub fn lowercase(s: &str) -> String {
        s.to_lowercase()
    }

    /// Converts string to uppercase.
    pub fn uppercase(s: &str) -> String {
        s.to_uppercase()
    }

    /// Sanitizes HTML content.
    pub fn sanitize_html(s: &str) -> String {
        s.replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// Applies trim and lowercase.
    pub fn normalize_email(s: &str) -> String {
        lowercase(&trim(s))
    }
}

/// Simple email validation (basic check).
fn is_valid_email(email: &str) -> bool {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Basic email validation
    let parts: Vec<&str> = trimmed.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local = parts[0];
    let domain = parts[1];

    if local.is_empty() || domain.is_empty() {
        return false;
    }

    if !domain.contains('.') {
        return false;
    }

    let domain_parts: Vec<&str> = domain.split('.').collect();
    if domain_parts.iter().any(|p| p.is_empty()) {
        return false;
    }

    true
}

/// Simple URL validation.
fn is_valid_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Basic URL validation
    (trimmed.starts_with("http://") || trimmed.starts_with("https://")) && trimmed.len() > 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_validation() {
        let mut validator = Validator::new();
        validator.string("name", "Al").min_length(3).max_length(100);
        let result = validator.finish();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.errors.len(), 1);
        assert_eq!(errors.errors[0].code, ValidationErrorCode::MinLength);
    }

    #[test]
    fn test_email_validation() {
        // Valid email
        let mut validator = Validator::new();
        validator.string("email", "test@example.com").email();
        let result = validator.finish();
        assert!(result.is_ok());

        // Invalid email
        let mut validator = Validator::new();
        validator.string("email", "not-an-email").email();
        let result = validator.finish();
        assert!(result.is_err());
    }

    #[test]
    fn test_numeric_validation() {
        let mut validator = Validator::new();
        validator.number("age", 15).min(18).max(120);
        let result = validator.finish();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.errors[0].code, ValidationErrorCode::Min);
    }

    #[test]
    fn test_multiple_fields() {
        let mut validator = Validator::new();
        validator.string("name", "").required().min_length(2);
        validator.string("email", "invalid").email();
        let result = validator.finish();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.errors.len() >= 2);
    }

    #[test]
    fn test_transform() {
        assert_eq!(transform::trim("  hello  "), "hello");
        assert_eq!(transform::lowercase("HELLO"), "hello");
        assert_eq!(transform::normalize_email("  TEST@Example.COM  "), "test@example.com");
        assert_eq!(transform::sanitize_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
    }

    #[test]
    fn test_validation_errors_to_graphql() {
        let errors = ValidationErrors::single(
            ValidationError::new("email", ValidationErrorCode::InvalidEmail, "Invalid email")
                .with_constraint("email format"),
        );

        let json = errors.to_graphql_error();
        assert_eq!(json["__typename"], "ValidationError");
        assert_eq!(json["field"], "email");
    }
}
