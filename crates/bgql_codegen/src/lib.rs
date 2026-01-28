//! Code generation for Better GraphQL SDKs.
//!
//! This crate generates type-safe SDK code from bgql schemas for:
//! - TypeScript (client and server)
//! - Rust (client and server)
//! - Go (client and server)
//!
//! # Example
//!
//! ```ignore
//! use bgql_codegen::{CodeGenerator, Language};
//!
//! let generator = CodeGenerator::new(&document, &interner);
//! let code = generator.generate(Language::TypeScript);
//! ```

mod go;
mod rust;
mod typescript;

pub use go::GoGenerator;
pub use rust::RustGenerator;
pub use typescript::TypeScriptGenerator;

use bgql_core::Interner;
use bgql_syntax::{Definition, Document, OperationDefinition, Type, TypeDefinition};

/// Target language for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    TypeScript,
    Rust,
    Go,
}

/// Code generation options.
#[derive(Debug, Clone)]
pub struct CodegenOptions {
    /// Generate client SDK code.
    pub client: bool,
    /// Generate server SDK code.
    pub server: bool,
    /// Generate operation types from queries/mutations.
    pub operations: bool,
    /// Package/module name.
    pub package_name: String,
    /// Add runtime imports.
    pub include_runtime: bool,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            client: true,
            server: true,
            operations: true,
            package_name: "generated".to_string(),
            include_runtime: true,
        }
    }
}

/// Main code generator.
pub struct CodeGenerator<'a> {
    document: &'a Document<'a>,
    interner: &'a Interner,
    options: CodegenOptions,
}

impl<'a> CodeGenerator<'a> {
    /// Creates a new code generator.
    pub fn new(document: &'a Document<'a>, interner: &'a Interner) -> Self {
        Self {
            document,
            interner,
            options: CodegenOptions::default(),
        }
    }

    /// Creates a new code generator with options.
    pub fn with_options(
        document: &'a Document<'a>,
        interner: &'a Interner,
        options: CodegenOptions,
    ) -> Self {
        Self {
            document,
            interner,
            options,
        }
    }

    /// Generates code for the specified language.
    pub fn generate(&self, language: Language) -> String {
        match language {
            Language::TypeScript => {
                TypeScriptGenerator::new(self.document, self.interner, &self.options).generate()
            }
            Language::Rust => {
                RustGenerator::new(self.document, self.interner, &self.options).generate()
            }
            Language::Go => {
                GoGenerator::new(self.document, self.interner, &self.options).generate()
            }
        }
    }

    /// Sets code generation options.
    pub fn options(mut self, options: CodegenOptions) -> Self {
        self.options = options;
        self
    }
}

/// Helper trait for type conversion.
pub(crate) trait TypeConverter {
    fn convert_type(&self, ty: &Type<'_>, interner: &Interner) -> String;
    fn convert_scalar(&self, name: &str) -> String;
}

/// Extract type definitions from document.
pub(crate) fn extract_types<'a>(document: &'a Document<'a>) -> Vec<&'a TypeDefinition<'a>> {
    document
        .definitions
        .iter()
        .filter_map(|def| {
            if let Definition::Type(type_def) = def {
                Some(type_def)
            } else {
                None
            }
        })
        .collect()
}

/// Extract operation definitions from document.
#[allow(dead_code)]
pub(crate) fn extract_operations<'a>(
    document: &'a Document<'a>,
) -> Vec<&'a OperationDefinition<'a>> {
    document
        .definitions
        .iter()
        .filter_map(|def| {
            if let Definition::Operation(op) = def {
                Some(op)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_options_default() {
        let options = CodegenOptions::default();
        assert!(options.client);
        assert!(options.server);
        assert!(options.operations);
    }
}
