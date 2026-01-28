//! Core utilities for Better GraphQL.
//!
//! This crate provides foundational types used throughout bgql:
//! - `span`: Source location tracking
//! - `text`: String interning
//! - `arena`: Arena allocation
//! - `diagnostics`: Error reporting

pub mod arena;
pub mod diagnostics;
pub mod span;
pub mod text;

pub use arena::Arena;
pub use diagnostics::{Diagnostic, DiagnosticBag, DiagnosticSeverity, Label};
pub use span::Span;
pub use text::{Interner, Text};
