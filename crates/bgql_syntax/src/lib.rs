//! Syntax layer for Better GraphQL.
//!
//! This crate provides:
//! - `token`: Token kinds and token structures
//! - `lexer`: Tokenization
//! - `ast`: Abstract syntax tree types
//! - `parser`: Recursive descent parser
//! - `formatter`: Code formatting

pub mod ast;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod token;

pub use ast::*;
pub use formatter::{format, format_with_options, FormatOptions, Formatter};
pub use lexer::Lexer;
pub use parser::{parse, ParseResult};
pub use token::{DirectiveLocation, Token, TokenKind};
