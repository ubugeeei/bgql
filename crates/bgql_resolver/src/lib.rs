//! Name resolution for Better GraphQL.
//!
//! This crate handles resolving names to their definitions,
//! including Rust-like module system support for schema organization.

pub mod module_system;

use bgql_core::{diagnostics::codes, DiagnosticBag, Span};
use bgql_semantic::{DefId, HirDatabase, TypeRegistry};
use rustc_hash::FxHashMap;

pub use module_system::{
    ExportKind, FileSystemResolver, ImportItem, ImportItems, Module, ModuleDeclaration,
    ModuleError, ModuleExport, ModuleImport, ModulePath, ModuleResolver, ModuleSystem,
    SchemaModule, UseDirective,
};

/// A scope for name resolution.
#[derive(Debug, Default)]
pub struct Scope {
    names: FxHashMap<String, DefId>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Scope) -> Self {
        Self {
            names: FxHashMap::default(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, name: String, id: DefId) {
        self.names.insert(name, id);
    }

    pub fn lookup(&self, name: &str) -> Option<DefId> {
        self.names
            .get(name)
            .copied()
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }
}

/// Context for name resolution.
#[derive(Debug)]
pub struct ResolverContext {
    pub types: TypeRegistry,
    pub hir: HirDatabase,
    pub diagnostics: DiagnosticBag,
    scopes: Vec<Scope>,
}

impl Default for ResolverContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolverContext {
    pub fn new() -> Self {
        let mut types = TypeRegistry::new();
        types.register_builtin_scalars();

        Self {
            types,
            hir: HirDatabase::new(),
            diagnostics: DiagnosticBag::new(),
            scopes: vec![Scope::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn current_scope(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("scope stack is never empty")
    }

    pub fn define(&mut self, name: String, id: DefId) {
        self.current_scope().define(name, id);
    }

    pub fn lookup(&self, name: &str) -> Option<DefId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.lookup(name) {
                return Some(id);
            }
        }
        None
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }
}

/// The name resolver.
pub struct Resolver<'a> {
    ctx: &'a mut ResolverContext,
}

impl<'a> Resolver<'a> {
    pub fn new(ctx: &'a mut ResolverContext) -> Self {
        Self { ctx }
    }

    pub fn resolve_type(&self, name: &str) -> Option<DefId> {
        self.ctx.lookup(name)
    }

    pub fn report_undefined_type(&mut self, name: &str, span: Span) {
        self.ctx.diagnostics.error(
            codes::UNDEFINED_TYPE,
            "undefined type",
            span,
            format!("type `{name}` is not defined"),
        );
    }
}

/// Result of resolution.
pub struct ResolverResult {
    pub hir: HirDatabase,
    pub types: TypeRegistry,
    pub diagnostics: DiagnosticBag,
}

impl ResolverResult {
    pub fn is_ok(&self) -> bool {
        !self.diagnostics.has_errors()
    }
}

/// Resolves names in a document.
pub fn resolve(_document: &bgql_syntax::Document<'_>) -> ResolverResult {
    let ctx = ResolverContext::new();

    ResolverResult {
        hir: ctx.hir,
        types: ctx.types,
        diagnostics: ctx.diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope() {
        let mut scope = Scope::new();
        let id = DefId::from_raw(1);
        scope.define("User".to_string(), id);
        assert_eq!(scope.lookup("User"), Some(id));
    }
}
