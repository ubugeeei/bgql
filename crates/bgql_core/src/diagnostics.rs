//! Diagnostic reporting for Better GraphQL.

use crate::span::Span;

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// An error that prevents compilation.
    Error,
    /// A warning that doesn't prevent compilation.
    Warning,
    /// An informational message.
    Info,
    /// A hint or suggestion.
    Hint,
}

/// A label attached to a diagnostic.
#[derive(Debug, Clone)]
pub struct Label {
    /// The span this label points to.
    pub span: Span,
    /// The label message.
    pub message: String,
}

impl Label {
    /// Creates a new label.
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// A diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Error code.
    pub code: String,
    /// Short title.
    pub title: String,
    /// Detailed message.
    pub message: Option<String>,
    /// Labels pointing to source locations.
    pub labels: Vec<Label>,
}

impl Diagnostic {
    /// Creates a new error diagnostic.
    pub fn error(code: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            code: code.into(),
            title: title.into(),
            message: None,
            labels: Vec::new(),
        }
    }

    /// Creates a new warning diagnostic.
    pub fn warning(code: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            code: code.into(),
            title: title.into(),
            message: None,
            labels: Vec::new(),
        }
    }

    /// Adds a message to the diagnostic.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Adds a label to the diagnostic.
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Adds a primary label at a span.
    pub fn with_span(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::new(span, message));
        self
    }

    /// Returns the primary span, if any.
    pub fn primary_span(&self) -> Option<Span> {
        self.labels.first().map(|l| l.span)
    }
}

/// A collection of diagnostics.
#[derive(Debug, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    /// Creates a new empty diagnostic bag.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a diagnostic.
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Adds an error diagnostic.
    pub fn error(
        &mut self,
        code: impl Into<String>,
        title: impl Into<String>,
        span: Span,
        message: impl Into<String>,
    ) {
        self.add(Diagnostic::error(code, title).with_span(span, message));
    }

    /// Adds a warning diagnostic.
    pub fn warning(
        &mut self,
        code: impl Into<String>,
        title: impl Into<String>,
        span: Span,
        message: impl Into<String>,
    ) {
        self.add(Diagnostic::warning(code, title).with_span(span, message));
    }

    /// Returns true if there are any errors.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }

    /// Returns the number of errors.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count()
    }

    /// Returns an iterator over all diagnostics.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    /// Returns an iterator over errors.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
    }

    /// Returns an iterator over warnings.
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
    }

    /// Returns true if there are no diagnostics.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Returns the number of diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }
}

/// Common diagnostic codes.
pub mod codes {
    pub const UNEXPECTED_TOKEN: &str = "E0001";
    pub const UNEXPECTED_EOF: &str = "E0002";
    pub const INVALID_SYNTAX: &str = "E0003";
    pub const UNDEFINED_TYPE: &str = "E0010";
    pub const UNDEFINED_FIELD: &str = "E0011";
    pub const DUPLICATE_TYPE: &str = "E0012";
    pub const DUPLICATE_FIELD: &str = "E0013";
    pub const TYPE_MISMATCH: &str = "E0020";
    pub const INVALID_DIRECTIVE: &str = "E0030";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_bag() {
        let mut bag = DiagnosticBag::new();
        bag.error("E001", "test error", Span::new(0, 10), "details");

        assert!(bag.has_errors());
        assert_eq!(bag.error_count(), 1);
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error("E001", "Test")
            .with_message("Details")
            .with_span(Span::new(0, 5), "here");

        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.primary_span(), Some(Span::new(0, 5)));
    }
}
