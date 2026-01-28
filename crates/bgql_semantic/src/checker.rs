//! Type checker for Better GraphQL.

use crate::hir::HirDatabase;
use crate::types::TypeRegistry;
use bgql_core::DiagnosticBag;
use bgql_syntax::Document;

/// Type checker for Better GraphQL.
pub struct TypeChecker<'a> {
    #[allow(dead_code)]
    types: &'a TypeRegistry,
    #[allow(dead_code)]
    hir: &'a HirDatabase,
    diagnostics: DiagnosticBag,
}

/// Result of type checking.
pub struct CheckResult {
    pub diagnostics: DiagnosticBag,
}

impl CheckResult {
    /// Returns true if type checking succeeded.
    pub fn is_ok(&self) -> bool {
        !self.diagnostics.has_errors()
    }
}

impl<'a> TypeChecker<'a> {
    /// Creates a new type checker.
    pub fn new(types: &'a TypeRegistry, hir: &'a HirDatabase) -> Self {
        Self {
            types,
            hir,
            diagnostics: DiagnosticBag::new(),
        }
    }

    /// Checks a document.
    pub fn check(&mut self, _document: &Document<'_>) -> CheckResult {
        // TODO: Implement type checking
        // 1. Check all type references are valid
        // 2. Check field types match
        // 3. Check directive arguments
        // 4. Check generic constraints

        CheckResult {
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }
}

/// Type checks a document.
pub fn check(document: &Document<'_>, types: &TypeRegistry, hir: &HirDatabase) -> CheckResult {
    let mut checker = TypeChecker::new(types, hir);
    checker.check(document)
}
