//! Type-safe context for request-scoped data.
//!
//! Uses `TypeId` for compile-time type safety instead of string keys.

use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// A type-safe storage for request-scoped data.
///
/// Unlike `HashMap<String, Value>`, this provides compile-time type safety.
///
/// # Example
///
/// ```
/// use bgql_sdk::context::TypedContext;
///
/// #[derive(Clone)]
/// struct UserId(String);
///
/// #[derive(Clone)]
/// struct UserRoles(Vec<String>);
///
/// let mut ctx = TypedContext::new();
/// ctx.insert(UserId("123".into()));
/// ctx.insert(UserRoles(vec!["admin".into()]));
///
/// // Type-safe retrieval
/// let user_id: Option<&UserId> = ctx.get();
/// assert_eq!(user_id.unwrap().0, "123");
///
/// // Wrong type returns None (compile-time safe)
/// let roles: Option<&UserRoles> = ctx.get();
/// assert!(roles.is_some());
/// ```
#[derive(Default)]
pub struct TypedContext {
    data: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    headers: HashMap<String, String>,
}

impl TypedContext {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a value into the context.
    ///
    /// If a value of the same type already exists, it is replaced.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        self.data
            .insert(TypeId::of::<T>(), Box::new(value))
            .and_then(|boxed| boxed.downcast().ok().map(|b| *b))
    }

    /// Gets a reference to a value by type.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref())
    }

    /// Gets a mutable reference to a value by type.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.data
            .get_mut(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut())
    }

    /// Removes a value by type.
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.data
            .remove(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast().ok().map(|b| *b))
    }

    /// Returns true if the context contains a value of the given type.
    pub fn contains<T: 'static>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }

    /// Clears all values from the context.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Sets a header value.
    pub fn set_header(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(key.into(), value.into());
    }

    /// Gets a header value.
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }

    /// Returns all headers.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Returns mutable headers.
    pub fn headers_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.headers
    }
}

impl fmt::Debug for TypedContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypedContext")
            .field("data_count", &self.data.len())
            .field("headers", &self.headers)
            .finish()
    }
}

/// A shareable, thread-safe context wrapper.
pub type SharedContext = Arc<TypedContext>;

/// Extension trait for building contexts with fluent API.
pub trait ContextExt {
    /// Adds a value to the context and returns self.
    fn with<T: Send + Sync + 'static>(self, value: T) -> Self;

    /// Adds a header and returns self.
    fn with_header(self, key: impl Into<String>, value: impl Into<String>) -> Self;
}

impl ContextExt for TypedContext {
    fn with<T: Send + Sync + 'static>(mut self, value: T) -> Self {
        self.insert(value);
        self
    }

    fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.set_header(key, value);
        self
    }
}

/// Marker trait for context-extractable types.
///
/// Types implementing this trait can be extracted from context in resolvers.
pub trait FromContext: Sized {
    /// Extracts the value from the context.
    fn from_context(ctx: &TypedContext) -> Option<Self>;
}

impl<T: Clone + 'static> FromContext for T {
    fn from_context(ctx: &TypedContext) -> Option<Self> {
        ctx.get::<T>().cloned()
    }
}

/// A reference wrapper that implements FromContext.
pub struct ContextRef<'a, T>(&'a T);

impl<'a, T> std::ops::Deref for ContextRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Common context data types.
pub mod data {
    use super::*;

    /// Current user ID.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct CurrentUserId(pub String);

    impl CurrentUserId {
        pub fn new(id: impl Into<String>) -> Self {
            Self(id.into())
        }

        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    /// User roles for authorization.
    #[derive(Debug, Clone, Default)]
    pub struct UserRoles(pub Vec<String>);

    impl UserRoles {
        pub fn new(roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
            Self(roles.into_iter().map(|r| r.into()).collect())
        }

        pub fn has_role(&self, role: &str) -> bool {
            self.0.iter().any(|r| r == role)
        }

        pub fn has_any_role(&self, roles: &[&str]) -> bool {
            roles.iter().any(|r| self.has_role(r))
        }

        pub fn has_all_roles(&self, roles: &[&str]) -> bool {
            roles.iter().all(|r| self.has_role(r))
        }
    }

    /// Request ID for tracing.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct RequestId(pub String);

    impl RequestId {
        pub fn new(id: impl Into<String>) -> Self {
            Self(id.into())
        }

        pub fn generate() -> Self {
            use std::time::{SystemTime, UNIX_EPOCH};
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            Self(format!("req_{:x}", timestamp))
        }
    }

    /// Request start time for timing.
    #[derive(Debug, Clone, Copy)]
    pub struct RequestStartTime(pub std::time::Instant);

    impl Default for RequestStartTime {
        fn default() -> Self {
            Self(std::time::Instant::now())
        }
    }

    impl RequestStartTime {
        pub fn elapsed(&self) -> std::time::Duration {
            self.0.elapsed()
        }
    }

    /// Database connection pool reference.
    #[derive(Clone)]
    pub struct DbPool<T>(pub Arc<T>);

    impl<T> DbPool<T> {
        pub fn new(pool: T) -> Self {
            Self(Arc::new(pool))
        }

        pub fn inner(&self) -> &T {
            &self.0
        }
    }

    impl<T> std::ops::Deref for DbPool<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::data::*;
    use super::*;

    #[test]
    fn test_typed_context_basic() {
        let mut ctx = TypedContext::new();

        ctx.insert(CurrentUserId::new("user_123"));
        ctx.insert(UserRoles::new(["admin", "user"]));

        let user_id = ctx.get::<CurrentUserId>().unwrap();
        assert_eq!(user_id.as_str(), "user_123");

        let roles = ctx.get::<UserRoles>().unwrap();
        assert!(roles.has_role("admin"));
        assert!(!roles.has_role("superadmin"));
    }

    #[test]
    fn test_typed_context_replace() {
        let mut ctx = TypedContext::new();

        ctx.insert(CurrentUserId::new("old_user"));
        let old = ctx.insert(CurrentUserId::new("new_user"));

        assert_eq!(old.unwrap().0, "old_user");
        assert_eq!(ctx.get::<CurrentUserId>().unwrap().0, "new_user");
    }

    #[test]
    fn test_typed_context_remove() {
        let mut ctx = TypedContext::new();
        ctx.insert(CurrentUserId::new("user_123"));

        let removed = ctx.remove::<CurrentUserId>();
        assert!(removed.is_some());
        assert!(ctx.get::<CurrentUserId>().is_none());
    }

    #[test]
    fn test_context_ext_fluent() {
        let ctx = TypedContext::new()
            .with(CurrentUserId::new("123"))
            .with(UserRoles::new(["admin"]))
            .with_header("Authorization", "Bearer token");

        assert!(ctx.contains::<CurrentUserId>());
        assert!(ctx.contains::<UserRoles>());
        assert_eq!(ctx.header("Authorization"), Some("Bearer token"));
    }

    #[test]
    fn test_user_roles() {
        let roles = UserRoles::new(["admin", "editor", "viewer"]);

        assert!(roles.has_role("admin"));
        assert!(roles.has_any_role(&["superadmin", "admin"]));
        assert!(roles.has_all_roles(&["admin", "editor"]));
        assert!(!roles.has_all_roles(&["admin", "superadmin"]));
    }

    #[test]
    fn test_request_id_generate() {
        let id1 = RequestId::generate();
        let id2 = RequestId::generate();
        assert_ne!(id1, id2);
        assert!(id1.0.starts_with("req_"));
    }
}
