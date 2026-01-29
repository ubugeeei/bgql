//! Resolver system for Better GraphQL.
//!
//! This module provides the resolver trait and infrastructure for field resolution.

use crate::executor::{Context, FieldError, PathSegment};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Arguments passed to a resolver.
#[derive(Debug, Clone, Default)]
pub struct ResolverArgs {
    args: HashMap<String, Value>,
}

impl ResolverArgs {
    /// Creates new resolver args.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates resolver args from a list of (name, value) pairs.
    pub fn from_pairs(pairs: Vec<(String, Value)>) -> Self {
        Self {
            args: pairs.into_iter().collect(),
        }
    }

    /// Gets an argument by name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.args.get(name)
    }

    /// Gets an argument as a specific type.
    pub fn get_as<T: serde::de::DeserializeOwned>(&self, name: &str) -> Option<T> {
        self.args
            .get(name)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Gets a required argument, returning an error if not found.
    pub fn require<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<T, ResolverError> {
        self.args
            .get(name)
            .ok_or_else(|| ResolverError::MissingArgument(name.to_string()))
            .and_then(|v| {
                serde_json::from_value(v.clone())
                    .map_err(|e| ResolverError::ArgumentParseError(name.to_string(), e.to_string()))
            })
    }

    /// Returns all arguments.
    pub fn all(&self) -> &HashMap<String, Value> {
        &self.args
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    /// Sets an argument.
    pub fn set(&mut self, name: impl Into<String>, value: Value) {
        self.args.insert(name.into(), value);
    }
}

/// Info about the field being resolved.
#[derive(Debug, Clone)]
pub struct ResolverInfo {
    /// The field name being resolved.
    pub field_name: String,

    /// The return type name.
    pub return_type: String,

    /// The parent type name.
    pub parent_type: String,

    /// Path to this field.
    pub path: Vec<PathSegment>,

    /// Selected sub-fields (for object types).
    pub selected_fields: Vec<String>,
}

impl ResolverInfo {
    /// Creates new resolver info.
    pub fn new(field_name: impl Into<String>, parent_type: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            return_type: String::new(),
            parent_type: parent_type.into(),
            path: Vec::new(),
            selected_fields: Vec::new(),
        }
    }

    /// Sets the return type.
    pub fn with_return_type(mut self, ty: impl Into<String>) -> Self {
        self.return_type = ty.into();
        self
    }

    /// Sets the path.
    pub fn with_path(mut self, path: Vec<PathSegment>) -> Self {
        self.path = path;
        self
    }

    /// Sets the selected fields.
    pub fn with_selected_fields(mut self, fields: Vec<String>) -> Self {
        self.selected_fields = fields;
        self
    }
}

/// Result type for resolvers.
pub type ResolverResult = Result<Value, ResolverError>;

/// Future type for async resolvers.
pub type ResolverFuture<'a> = Pin<Box<dyn Future<Output = ResolverResult> + Send + 'a>>;

/// Error from a resolver.
#[derive(Debug, Clone)]
pub enum ResolverError {
    /// Field not found.
    FieldNotFound(String),

    /// Missing required argument.
    MissingArgument(String),

    /// Argument parse error.
    ArgumentParseError(String, String),

    /// Null value for non-nullable field.
    NullValue(String),

    /// Custom error.
    Custom(String),

    /// Internal error.
    Internal(String),
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FieldNotFound(field) => write!(f, "Field not found: {}", field),
            Self::MissingArgument(arg) => write!(f, "Missing required argument: {}", arg),
            Self::ArgumentParseError(arg, err) => {
                write!(f, "Failed to parse argument '{}': {}", arg, err)
            }
            Self::NullValue(field) => write!(f, "Null value for non-nullable field: {}", field),
            Self::Custom(msg) => write!(f, "{}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ResolverError {}

impl From<ResolverError> for FieldError {
    fn from(error: ResolverError) -> Self {
        FieldError::new(error.to_string())
    }
}

/// Trait for field resolvers.
pub trait Resolver: Send + Sync {
    /// Resolves a field value.
    fn resolve<'a>(
        &'a self,
        parent: &'a Value,
        args: &'a ResolverArgs,
        ctx: &'a Context,
        info: &'a ResolverInfo,
    ) -> ResolverFuture<'a>;
}

/// A boxed resolver.
pub type BoxedResolver = Box<dyn Resolver>;

/// A sync resolver function.
pub type SyncResolverFn =
    Arc<dyn Fn(&Value, &ResolverArgs, &Context, &ResolverInfo) -> ResolverResult + Send + Sync>;

/// A wrapper for sync resolver functions.
pub struct FnResolver {
    func: SyncResolverFn,
}

impl FnResolver {
    /// Creates a new function resolver.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&Value, &ResolverArgs, &Context, &ResolverInfo) -> ResolverResult
            + Send
            + Sync
            + 'static,
    {
        Self { func: Arc::new(f) }
    }
}

impl Resolver for FnResolver {
    fn resolve<'a>(
        &'a self,
        parent: &'a Value,
        args: &'a ResolverArgs,
        ctx: &'a Context,
        info: &'a ResolverInfo,
    ) -> ResolverFuture<'a> {
        let result = (self.func)(parent, args, ctx, info);
        Box::pin(async move { result })
    }
}

/// An async resolver function type.
pub type AsyncResolverFn = Arc<
    dyn Fn(Value, ResolverArgs, Context, ResolverInfo) -> ResolverFuture<'static> + Send + Sync,
>;

/// A wrapper for async resolver functions.
pub struct AsyncFnResolver {
    func: AsyncResolverFn,
}

impl AsyncFnResolver {
    /// Creates a new async function resolver.
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn(Value, ResolverArgs, Context, ResolverInfo) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ResolverResult> + Send + 'static,
    {
        Self {
            func: Arc::new(move |parent, args, ctx, info| Box::pin(f(parent, args, ctx, info))),
        }
    }
}

impl Resolver for AsyncFnResolver {
    fn resolve<'a>(
        &'a self,
        parent: &'a Value,
        args: &'a ResolverArgs,
        ctx: &'a Context,
        info: &'a ResolverInfo,
    ) -> ResolverFuture<'a> {
        let parent = parent.clone();
        let args = args.clone();
        let ctx = ctx.clone();
        let info = info.clone();
        let func = Arc::clone(&self.func);
        Box::pin(async move { func(parent, args, ctx, info).await })
    }
}

/// Default resolver that accesses properties from the parent object.
pub struct DefaultResolver;

impl Resolver for DefaultResolver {
    fn resolve<'a>(
        &'a self,
        parent: &'a Value,
        _args: &'a ResolverArgs,
        _ctx: &'a Context,
        info: &'a ResolverInfo,
    ) -> ResolverFuture<'a> {
        let field_name = &info.field_name;
        let result = match parent {
            Value::Object(map) => {
                if let Some(value) = map.get(field_name) {
                    Ok(value.clone())
                } else {
                    // Try snake_case version
                    let snake_case = to_snake_case(field_name);
                    if let Some(value) = map.get(&snake_case) {
                        Ok(value.clone())
                    } else {
                        Ok(Value::Null)
                    }
                }
            }
            Value::Null => Ok(Value::Null),
            _ => Err(ResolverError::FieldNotFound(field_name.clone())),
        };
        Box::pin(async move { result })
    }
}

/// Converts camelCase to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Storage for resolvers organized by type and field.
#[derive(Default)]
pub struct ResolverMap {
    /// Resolvers indexed by "TypeName.fieldName".
    resolvers: HashMap<String, BoxedResolver>,

    /// Default resolver for unregistered fields.
    default_resolver: Option<BoxedResolver>,
}

impl ResolverMap {
    /// Creates a new resolver map.
    pub fn new() -> Self {
        Self {
            resolvers: HashMap::new(),
            default_resolver: Some(Box::new(DefaultResolver)),
        }
    }

    /// Registers a resolver for a specific type and field.
    pub fn register<R: Resolver + 'static>(
        &mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        resolver: R,
    ) {
        let key = format!("{}.{}", type_name.into(), field_name.into());
        self.resolvers.insert(key, Box::new(resolver));
    }

    /// Registers a sync function as a resolver.
    pub fn register_fn<F>(
        &mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        f: F,
    ) where
        F: Fn(&Value, &ResolverArgs, &Context, &ResolverInfo) -> ResolverResult
            + Send
            + Sync
            + 'static,
    {
        self.register(type_name, field_name, FnResolver::new(f));
    }

    /// Registers an async function as a resolver.
    pub fn register_async<F, Fut>(
        &mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        f: F,
    ) where
        F: Fn(Value, ResolverArgs, Context, ResolverInfo) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ResolverResult> + Send + 'static,
    {
        self.register(type_name, field_name, AsyncFnResolver::new(f));
    }

    /// Gets a resolver for a type and field.
    pub fn get(&self, type_name: &str, field_name: &str) -> Option<&dyn Resolver> {
        let key = format!("{}.{}", type_name, field_name);
        self.resolvers
            .get(&key)
            .map(|r| r.as_ref())
            .or(self.default_resolver.as_ref().map(|r| r.as_ref()))
    }

    /// Sets the default resolver.
    pub fn set_default<R: Resolver + 'static>(&mut self, resolver: R) {
        self.default_resolver = Some(Box::new(resolver));
    }

    /// Removes the default resolver.
    pub fn remove_default(&mut self) {
        self.default_resolver = None;
    }
}

impl Debug for ResolverMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolverMap")
            .field("resolver_count", &self.resolvers.len())
            .field("has_default", &self.default_resolver.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_args() {
        let mut args = ResolverArgs::new();
        args.set("id", serde_json::json!(123));
        args.set("name", serde_json::json!("test"));

        assert_eq!(args.get_as::<i64>("id"), Some(123));
        assert_eq!(args.get_as::<String>("name"), Some("test".to_string()));
        assert_eq!(args.get_as::<i64>("missing"), None);
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("firstName"), "first_name");
        assert_eq!(to_snake_case("lastName"), "last_name");
        assert_eq!(to_snake_case("id"), "id");
        assert_eq!(to_snake_case("ID"), "i_d");
    }

    #[tokio::test]
    async fn test_default_resolver() {
        let resolver = DefaultResolver;
        let parent = serde_json::json!({"name": "Alice", "age": 30});
        let args = ResolverArgs::new();
        let ctx = Context::new();
        let info = ResolverInfo::new("name", "User");

        let result = resolver.resolve(&parent, &args, &ctx, &info).await;
        assert_eq!(result.unwrap(), serde_json::json!("Alice"));
    }

    #[tokio::test]
    async fn test_fn_resolver() {
        let resolver = FnResolver::new(|_parent, args, _ctx, _info| {
            let id: i64 = args.require("id")?;
            Ok(serde_json::json!({"id": id, "name": "User"}))
        });

        let parent = serde_json::json!({});
        let mut args = ResolverArgs::new();
        args.set("id", serde_json::json!(42));
        let ctx = Context::new();
        let info = ResolverInfo::new("user", "Query");

        let result = resolver.resolve(&parent, &args, &ctx, &info).await;
        assert_eq!(
            result.unwrap(),
            serde_json::json!({"id": 42, "name": "User"})
        );
    }

    #[tokio::test]
    async fn test_resolver_map() {
        let mut map = ResolverMap::new();

        map.register_fn("Query", "hello", |_parent, _args, _ctx, _info| {
            Ok(serde_json::json!("Hello, World!"))
        });

        let resolver = map.get("Query", "hello").unwrap();
        let parent = serde_json::json!({});
        let args = ResolverArgs::new();
        let ctx = Context::new();
        let info = ResolverInfo::new("hello", "Query");

        let result = resolver.resolve(&parent, &args, &ctx, &info).await;
        assert_eq!(result.unwrap(), serde_json::json!("Hello, World!"));
    }

    #[tokio::test]
    async fn test_resolver_map_default_fallback() {
        let map = ResolverMap::new();

        // Should use default resolver for unregistered fields
        let resolver = map.get("User", "name").unwrap();
        let parent = serde_json::json!({"name": "Bob"});
        let args = ResolverArgs::new();
        let ctx = Context::new();
        let info = ResolverInfo::new("name", "User");

        let result = resolver.resolve(&parent, &args, &ctx, &info).await;
        assert_eq!(result.unwrap(), serde_json::json!("Bob"));
    }
}
