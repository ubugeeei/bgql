//! Rust-like module system for GraphQL schemas.
//!
//! This module provides a simple module system for organizing schemas
//! across multiple files with mod/use syntax.
//!
//! # Example Schema Organization
//!
//! ```text
//! schema/
//! ├── mod.bgql          # Root module
//! ├── user/
//! │   ├── mod.bgql      # User module
//! │   ├── types.bgql    # Type definitions
//! │   └── queries.bgql  # Query definitions
//! └── posts/
//!     ├── mod.bgql
//!     └── types.bgql
//! ```
//!
//! # Syntax
//!
//! ```graphql
//! # mod.bgql (root)
//! mod user
//! mod posts
//!
//! use user::{User, UserInput}
//! use posts::Post
//!
//! type Query {
//!   user(id: ID!): User
//!   posts: [Post!]!
//! }
//! ```

use bgql_semantic::DefId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A module path (e.g., "user::types" or "posts").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModulePath {
    segments: Vec<String>,
}

impl ModulePath {
    /// Creates a root module path.
    pub fn root() -> Self {
        Self { segments: vec![] }
    }

    /// Creates a module path from segments.
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }

    /// Parses a module path from a string (e.g., "user::types").
    pub fn parse(path: &str) -> Self {
        if path.is_empty() {
            return Self::root();
        }
        Self {
            segments: path.split("::").map(|s| s.to_string()).collect(),
        }
    }

    /// Returns the parent path.
    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            None
        } else {
            Some(Self {
                segments: self.segments[..self.segments.len() - 1].to_vec(),
            })
        }
    }

    /// Returns the module name (last segment).
    pub fn name(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    /// Joins with a child segment.
    pub fn join(&self, child: &str) -> Self {
        let mut segments = self.segments.clone();
        segments.push(child.to_string());
        Self { segments }
    }

    /// Checks if this is the root path.
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Returns as string.
    pub fn as_string(&self) -> String {
        self.segments.join("::")
    }

    /// Returns segments.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
}

impl std::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.segments.join("::"))
    }
}

/// A module in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Module path.
    pub path: ModulePath,

    /// Child module declarations.
    pub submodules: Vec<ModuleDeclaration>,

    /// Import statements (use).
    pub imports: Vec<ModuleImport>,

    /// Exported items.
    pub exports: IndexMap<String, ModuleExport>,

    /// Module documentation.
    pub doc: Option<String>,
}

impl Module {
    /// Creates a new module.
    pub fn new(path: ModulePath) -> Self {
        Self {
            path,
            submodules: vec![],
            imports: vec![],
            exports: IndexMap::new(),
            doc: None,
        }
    }

    /// Creates a root module.
    pub fn root() -> Self {
        Self::new(ModulePath::root())
    }

    /// Adds a submodule declaration.
    pub fn add_submodule(&mut self, decl: ModuleDeclaration) {
        self.submodules.push(decl);
    }

    /// Adds an import.
    pub fn add_import(&mut self, import: ModuleImport) {
        self.imports.push(import);
    }

    /// Adds an export.
    pub fn add_export(&mut self, name: impl Into<String>, export: ModuleExport) {
        self.exports.insert(name.into(), export);
    }

    /// Gets an export by name.
    pub fn get_export(&self, name: &str) -> Option<&ModuleExport> {
        self.exports.get(name)
    }

    /// Returns all exports.
    pub fn all_exports(&self) -> impl Iterator<Item = (&String, &ModuleExport)> {
        self.exports.iter()
    }
}

/// Module declaration (mod statement).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDeclaration {
    /// Module name.
    pub name: String,

    /// File path (if external).
    pub path: Option<PathBuf>,
}

impl ModuleDeclaration {
    /// Creates a new module declaration.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
        }
    }

    /// Sets file path.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }
}

/// Import statement (use).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleImport {
    /// Source module path.
    pub path: ModulePath,

    /// Items to import.
    pub items: ImportItems,

    /// Alias for the import.
    pub alias: Option<String>,
}

impl ModuleImport {
    /// Creates a new import.
    pub fn new(path: ModulePath, items: ImportItems) -> Self {
        Self {
            path,
            items,
            alias: None,
        }
    }

    /// Creates a glob import (use module::*).
    pub fn glob(path: ModulePath) -> Self {
        Self {
            path,
            items: ImportItems::Glob,
            alias: None,
        }
    }

    /// Sets an alias.
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// Parses a use statement string.
    pub fn parse(use_stmt: &str) -> Option<Self> {
        let use_stmt = use_stmt.trim();

        // Handle glob import: use path::*
        if let Some(path) = use_stmt.strip_suffix("::*") {
            return Some(Self::glob(ModulePath::parse(path)));
        }

        // Handle grouped import: use path::{A, B, C}
        if let Some(brace_start) = use_stmt.find("::{") {
            if use_stmt.ends_with('}') {
                let path = &use_stmt[..brace_start];
                let items_str = &use_stmt[brace_start + 3..use_stmt.len() - 1];
                let items: Vec<ImportItem> = items_str
                    .split(',')
                    .map(|s| {
                        let s = s.trim();
                        if let Some((name, alias)) = s.split_once(" as ") {
                            ImportItem {
                                name: name.trim().to_string(),
                                alias: Some(alias.trim().to_string()),
                            }
                        } else {
                            ImportItem {
                                name: s.to_string(),
                                alias: None,
                            }
                        }
                    })
                    .collect();
                return Some(Self::new(
                    ModulePath::parse(path),
                    ImportItems::Named(items),
                ));
            }
        }

        // Handle single import: use path::Item or use path::Item as Alias
        if let Some(last_sep) = use_stmt.rfind("::") {
            let path = &use_stmt[..last_sep];
            let item_part = &use_stmt[last_sep + 2..];

            if let Some((name, alias)) = item_part.split_once(" as ") {
                return Some(Self::new(
                    ModulePath::parse(path),
                    ImportItems::Named(vec![ImportItem {
                        name: name.trim().to_string(),
                        alias: Some(alias.trim().to_string()),
                    }]),
                ));
            } else {
                return Some(Self::new(
                    ModulePath::parse(path),
                    ImportItems::Named(vec![ImportItem {
                        name: item_part.to_string(),
                        alias: None,
                    }]),
                ));
            }
        }

        None
    }
}

/// Items in an import statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportItems {
    /// Glob import (*).
    Glob,
    /// Named items.
    Named(Vec<ImportItem>),
    /// Self import (the module itself).
    SelfImport,
}

/// A single imported item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportItem {
    /// Original name.
    pub name: String,
    /// Alias (if any).
    pub alias: Option<String>,
}

impl ImportItem {
    /// Creates a new import item.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            alias: None,
        }
    }

    /// Sets an alias.
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// Returns the name to use in the importing module.
    pub fn local_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
}

/// An exported item from a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleExport {
    /// Item kind.
    pub kind: ExportKind,

    /// Original name.
    pub name: String,

    /// Definition ID in HIR (not serialized).
    #[serde(skip)]
    pub def_id: Option<DefId>,

    /// Documentation.
    pub doc: Option<String>,
}

impl ModuleExport {
    /// Creates a type export.
    pub fn type_def(name: impl Into<String>) -> Self {
        Self {
            kind: ExportKind::Type,
            name: name.into(),
            def_id: None,
            doc: None,
        }
    }

    /// Creates a directive export.
    pub fn directive(name: impl Into<String>) -> Self {
        Self {
            kind: ExportKind::Directive,
            name: name.into(),
            def_id: None,
            doc: None,
        }
    }

    /// Creates a fragment export.
    pub fn fragment(name: impl Into<String>) -> Self {
        Self {
            kind: ExportKind::Fragment,
            name: name.into(),
            def_id: None,
            doc: None,
        }
    }

    /// Sets the DefId.
    pub fn with_def_id(mut self, def_id: DefId) -> Self {
        self.def_id = Some(def_id);
        self
    }
}

/// Kind of exported item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportKind {
    /// Type definition.
    Type,
    /// Directive definition.
    Directive,
    /// Fragment definition.
    Fragment,
    /// Query operation.
    Query,
    /// Mutation operation.
    Mutation,
    /// Subscription operation.
    Subscription,
}

/// @use directive for GraphQL syntax.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseDirective {
    /// Module path to import from.
    pub path: String,

    /// Items to import (comma-separated).
    pub items: Option<String>,

    /// Glob import.
    #[serde(default)]
    pub glob: bool,
}

impl UseDirective {
    /// Creates a new use directive.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            items: None,
            glob: false,
        }
    }

    /// Creates a glob import.
    pub fn glob(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            items: None,
            glob: true,
        }
    }

    /// Sets items to import.
    pub fn with_items(mut self, items: impl Into<String>) -> Self {
        self.items = Some(items.into());
        self
    }

    /// Converts to ModuleImport.
    pub fn to_import(&self) -> ModuleImport {
        let path = ModulePath::parse(&self.path);

        if self.glob {
            return ModuleImport::glob(path);
        }

        if let Some(items) = &self.items {
            let items: Vec<ImportItem> = items
                .split(',')
                .map(|s| ImportItem::new(s.trim()))
                .collect();
            ModuleImport::new(path, ImportItems::Named(items))
        } else {
            ModuleImport::new(path, ImportItems::SelfImport)
        }
    }
}

/// Schema module (a module with its resolved definitions).
#[derive(Debug, Clone)]
pub struct SchemaModule {
    /// Module definition.
    pub module: Module,

    /// Resolved type IDs in this module.
    pub types: IndexMap<String, DefId>,

    /// Resolved directive IDs in this module.
    pub directives: IndexMap<String, DefId>,

    /// Child modules.
    pub children: HashMap<String, SchemaModule>,
}

impl SchemaModule {
    /// Creates a new schema module.
    pub fn new(module: Module) -> Self {
        Self {
            module,
            types: IndexMap::new(),
            directives: IndexMap::new(),
            children: HashMap::new(),
        }
    }

    /// Adds a type.
    pub fn add_type(&mut self, name: impl Into<String>, def_id: DefId) {
        let name = name.into();
        self.types.insert(name.clone(), def_id);
        self.module.add_export(
            name.clone(),
            ModuleExport::type_def(name).with_def_id(def_id),
        );
    }

    /// Adds a directive.
    pub fn add_directive(&mut self, name: impl Into<String>, def_id: DefId) {
        let name = name.into();
        self.directives.insert(name.clone(), def_id);
        self.module.add_export(
            name.clone(),
            ModuleExport::directive(name).with_def_id(def_id),
        );
    }

    /// Adds a child module.
    pub fn add_child(&mut self, name: impl Into<String>, child: SchemaModule) {
        let name = name.into();
        self.module
            .add_submodule(ModuleDeclaration::new(name.clone()));
        self.children.insert(name, child);
    }

    /// Resolves a path relative to this module.
    pub fn resolve_path(&self, path: &ModulePath) -> Option<&SchemaModule> {
        if path.is_root() {
            return Some(self);
        }

        let mut current = self;
        for segment in path.segments() {
            current = current.children.get(segment)?;
        }
        Some(current)
    }

    /// Gets a type DefId by path.
    pub fn get_type(&self, path: &str) -> Option<DefId> {
        if let Some((module_path, type_name)) = path.rsplit_once("::") {
            let module = self.resolve_path(&ModulePath::parse(module_path))?;
            module.types.get(type_name).copied()
        } else {
            self.types.get(path).copied()
        }
    }

    /// Gets a directive DefId by path.
    pub fn get_directive(&self, path: &str) -> Option<DefId> {
        if let Some((module_path, dir_name)) = path.rsplit_once("::") {
            let module = self.resolve_path(&ModulePath::parse(module_path))?;
            module.directives.get(dir_name).copied()
        } else {
            self.directives.get(path).copied()
        }
    }

    /// Flattens all type DefIds into a single namespace (for compatibility).
    pub fn flatten_types(&self) -> IndexMap<String, DefId> {
        let mut result = self.types.clone();
        for (child_name, child) in &self.children {
            for (type_name, def_id) in child.flatten_types() {
                result.insert(format!("{}::{}", child_name, type_name), def_id);
            }
        }
        result
    }
}

/// Module system for managing schema modules.
#[derive(Debug)]
pub struct ModuleSystem {
    /// Root module.
    root: SchemaModule,
}

impl ModuleSystem {
    /// Creates a new module system.
    pub fn new() -> Self {
        Self {
            root: SchemaModule::new(Module::root()),
        }
    }

    /// Gets the root module.
    pub fn root(&self) -> &SchemaModule {
        &self.root
    }

    /// Gets a mutable reference to the root module.
    pub fn root_mut(&mut self) -> &mut SchemaModule {
        &mut self.root
    }

    /// Resolves a type reference considering imports.
    pub fn resolve_type(&self, name: &str, from_module: &ModulePath) -> Option<DefId> {
        // First, check if it's a local type
        if let Some(module) = self.root.resolve_path(from_module) {
            if let Some(def_id) = module.types.get(name) {
                return Some(*def_id);
            }

            // Check imports
            for import in &module.module.imports {
                match &import.items {
                    ImportItems::Named(items) => {
                        for item in items {
                            if item.local_name() == name {
                                let full_path = format!("{}::{}", import.path, item.name);
                                return self.root.get_type(&full_path);
                            }
                        }
                    }
                    ImportItems::Glob => {
                        if let Some(imported_module) = self.root.resolve_path(&import.path) {
                            if let Some(def_id) = imported_module.types.get(name) {
                                return Some(*def_id);
                            }
                        }
                    }
                    ImportItems::SelfImport => {
                        if let Some(imported_module) = self.root.resolve_path(&import.path) {
                            if let Some(def_id) = imported_module.types.get(name) {
                                return Some(*def_id);
                            }
                        }
                    }
                }
            }
        }

        // Fall back to absolute path resolution
        self.root.get_type(name)
    }

    /// Resolves a directive reference considering imports.
    pub fn resolve_directive(&self, name: &str, from_module: &ModulePath) -> Option<DefId> {
        if let Some(module) = self.root.resolve_path(from_module) {
            if let Some(def_id) = module.directives.get(name) {
                return Some(*def_id);
            }
        }
        self.root.get_directive(name)
    }
}

impl Default for ModuleSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Module resolver for loading modules from files.
pub trait ModuleResolver {
    /// Resolves a module path to its content.
    fn resolve(&self, path: &ModulePath) -> Result<String, ModuleError>;

    /// Checks if a module exists.
    fn exists(&self, path: &ModulePath) -> bool;
}

/// File system module resolver.
#[derive(Debug)]
pub struct FileSystemResolver {
    /// Base directory.
    base_dir: PathBuf,

    /// File extension.
    extension: String,
}

impl FileSystemResolver {
    /// Creates a new file system resolver.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            extension: "bgql".to_string(),
        }
    }

    /// Sets the file extension.
    pub fn with_extension(mut self, ext: impl Into<String>) -> Self {
        self.extension = ext.into();
        self
    }

    /// Converts a module path to a file path.
    fn to_file_path(&self, path: &ModulePath) -> PathBuf {
        let mut file_path = self.base_dir.clone();

        for segment in path.segments() {
            file_path.push(segment);
        }

        // Try mod.bgql first, then {name}.bgql
        let mod_file = file_path.join(format!("mod.{}", self.extension));
        if mod_file.exists() {
            return mod_file;
        }

        file_path.set_extension(&self.extension);
        file_path
    }
}

impl ModuleResolver for FileSystemResolver {
    fn resolve(&self, path: &ModulePath) -> Result<String, ModuleError> {
        let file_path = self.to_file_path(path);
        std::fs::read_to_string(&file_path).map_err(|e| ModuleError::IoError {
            path: path.to_string(),
            message: e.to_string(),
        })
    }

    fn exists(&self, path: &ModulePath) -> bool {
        self.to_file_path(path).exists()
    }
}

/// Module error.
#[derive(Debug, Clone)]
pub enum ModuleError {
    /// Module not found.
    NotFound { path: String },
    /// Circular dependency.
    CircularDependency { path: String },
    /// I/O error.
    IoError { path: String, message: String },
    /// Parse error.
    ParseError { path: String, message: String },
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "Module not found: {}", path),
            Self::CircularDependency { path } => {
                write!(f, "Circular dependency detected: {}", path)
            }
            Self::IoError { path, message } => {
                write!(f, "I/O error loading {}: {}", path, message)
            }
            Self::ParseError { path, message } => {
                write!(f, "Parse error in {}: {}", path, message)
            }
        }
    }
}

impl std::error::Error for ModuleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_path() {
        let path = ModulePath::parse("user::types");
        assert_eq!(path.segments, vec!["user", "types"]);
        assert_eq!(path.name(), Some("types"));
        assert_eq!(path.as_string(), "user::types");

        let parent = path.parent().unwrap();
        assert_eq!(parent.as_string(), "user");

        let child = parent.join("queries");
        assert_eq!(child.as_string(), "user::queries");
    }

    #[test]
    fn test_import_parse() {
        // Single import
        let import = ModuleImport::parse("user::User").unwrap();
        assert_eq!(import.path.as_string(), "user");
        matches!(import.items, ImportItems::Named(_));

        // Grouped import
        let import = ModuleImport::parse("user::{User, UserInput}").unwrap();
        assert_eq!(import.path.as_string(), "user");
        if let ImportItems::Named(items) = import.items {
            assert_eq!(items.len(), 2);
        }

        // Glob import
        let import = ModuleImport::parse("user::*").unwrap();
        assert_eq!(import.path.as_string(), "user");
        matches!(import.items, ImportItems::Glob);
    }

    #[test]
    fn test_schema_module() {
        let mut root = SchemaModule::new(Module::root());

        let mut user_module = SchemaModule::new(Module::new(ModulePath::parse("user")));
        let user_def_id = DefId::from_raw(1);
        user_module.add_type("User", user_def_id);

        root.add_child("user", user_module);

        // Should be able to resolve
        assert!(root.get_type("user::User").is_some());
    }

    #[test]
    fn test_module_system() {
        let mut system = ModuleSystem::new();

        // Add a type to root
        let id_def_id = DefId::from_raw(1);
        system.root_mut().add_type("ID", id_def_id);

        // Should be able to resolve
        let resolved = system.resolve_type("ID", &ModulePath::root());
        assert_eq!(resolved, Some(id_def_id));
    }

    #[test]
    fn test_use_directive() {
        let use_dir = UseDirective::new("user").with_items("User, UserInput");
        let import = use_dir.to_import();

        assert_eq!(import.path.as_string(), "user");
        if let ImportItems::Named(items) = import.items {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].name, "User");
            assert_eq!(items[1].name, "UserInput");
        }
    }
}
