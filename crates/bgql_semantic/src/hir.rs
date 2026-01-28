//! High-level Intermediate Representation for Better GraphQL.

use bgql_core::Span;
use rustc_hash::FxHashMap;
use std::sync::atomic::{AtomicU32, Ordering};

/// A definition ID, uniquely identifying a definition in the HIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(u32);

impl DefId {
    /// Creates a DefId from a raw value.
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    pub const fn as_raw(self) -> u32 {
        self.0
    }
}

/// The HIR database.
#[derive(Debug, Default)]
pub struct HirDatabase {
    next_id: AtomicU32,
    definitions: FxHashMap<DefId, HirDefinition>,
}

impl HirDatabase {
    /// Creates a new HIR database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates a new definition ID.
    pub fn alloc_def_id(&self) -> DefId {
        DefId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Adds a definition.
    pub fn add_definition(&mut self, id: DefId, def: HirDefinition) {
        self.definitions.insert(id, def);
    }

    /// Gets a definition by ID.
    pub fn get(&self, id: DefId) -> Option<&HirDefinition> {
        self.definitions.get(&id)
    }
}

/// A HIR definition.
#[derive(Debug, Clone)]
pub enum HirDefinition {
    Type(HirTypeDef),
    Field(HirField),
    Directive(HirDirective),
}

/// A type definition in HIR.
#[derive(Debug, Clone)]
pub struct HirTypeDef {
    pub name: String,
    pub kind: HirTypeKind,
    pub fields: Vec<DefId>,
    pub implements: Vec<DefId>,
    pub span: Span,
}

/// The kind of a type definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirTypeKind {
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
    Scalar,
    Opaque,
}

/// A field in HIR.
#[derive(Debug, Clone)]
pub struct HirField {
    pub name: String,
    pub type_id: DefId,
    pub arguments: Vec<DefId>,
    pub span: Span,
}

/// A directive in HIR.
#[derive(Debug, Clone)]
pub struct HirDirective {
    pub name: String,
    pub arguments: Vec<DefId>,
    pub span: Span,
}

/// An operation in HIR.
#[derive(Debug, Clone)]
pub struct HirOperation {
    pub kind: HirOperationKind,
    pub name: Option<String>,
    pub variables: Vec<HirVariable>,
    pub selections: Vec<HirSelection>,
    pub span: Span,
}

/// The kind of operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirOperationKind {
    Query,
    Mutation,
    Subscription,
}

/// A variable in HIR.
#[derive(Debug, Clone)]
pub struct HirVariable {
    pub name: String,
    pub type_id: DefId,
    pub default_value: Option<HirValue>,
}

/// A selection in HIR.
#[derive(Debug, Clone)]
pub enum HirSelection {
    Field(HirFieldSelection),
    FragmentSpread(String),
    InlineFragment(HirInlineFragment),
}

/// A field selection in HIR.
#[derive(Debug, Clone)]
pub struct HirFieldSelection {
    pub alias: Option<String>,
    pub name: String,
    pub arguments: Vec<(String, HirValue)>,
    pub selections: Vec<HirSelection>,
}

/// An inline fragment in HIR.
#[derive(Debug, Clone)]
pub struct HirInlineFragment {
    pub type_condition: Option<String>,
    pub selections: Vec<HirSelection>,
}

/// A value in HIR.
#[derive(Debug, Clone)]
pub enum HirValue {
    Variable(String),
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Enum(String),
    List(Vec<HirValue>),
    Object(Vec<(String, HirValue)>),
}
