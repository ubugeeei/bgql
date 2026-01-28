//! Semantic analysis for Better GraphQL.
//!
//! This crate provides:
//! - `hir`: High-level intermediate representation
//! - `types`: Type system
//! - `checker`: Type checking

pub mod checker;
pub mod hir;
pub mod types;

pub use hir::{DefId, HirDatabase};
pub use types::{Type, TypeRegistry};
