//! Strongly typed resolvers and operations.
//!
//! Provides compile-time type safety for GraphQL operations and resolvers.

use crate::context::TypedContext;
use crate::error::{ErrorCode, SdkError, SdkResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

// ============================================================================
// Typed GraphQL Operations (Client)
// ============================================================================

/// A strongly typed GraphQL operation.
///
/// This trait defines the contract for type-safe GraphQL queries and mutations.
///
/// # Example
///
/// ```ignore
/// use bgql_sdk::typed::{TypedOperation, OperationKind};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize)]
/// struct GetUserVariables {
///     id: String,
/// }
///
/// #[derive(Deserialize)]
/// struct GetUserResponse {
///     user: Option<User>,
/// }
///
/// #[derive(Deserialize)]
/// struct User {
///     id: String,
///     name: String,
/// }
///
/// struct GetUserQuery;
///
/// impl TypedOperation for GetUserQuery {
///     type Variables = GetUserVariables;
///     type Response = GetUserResponse;
///
///     const OPERATION: &'static str = "query GetUser($id: ID!) { user(id: $id) { id name } }";
///     const OPERATION_NAME: &'static str = "GetUser";
///     const KIND: OperationKind = OperationKind::Query;
/// }
/// ```
pub trait TypedOperation {
    /// The input variables type.
    type Variables: Serialize;

    /// The response data type.
    type Response: DeserializeOwned;

    /// The GraphQL operation string.
    const OPERATION: &'static str;

    /// The operation name (for multi-operation documents).
    const OPERATION_NAME: &'static str;

    /// The kind of operation.
    const KIND: OperationKind;
}

/// The kind of GraphQL operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Query,
    Mutation,
    Subscription,
}

/// Marker type for operations without variables.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct NoVariables;

/// Marker type for operations without a typed response (returns raw JSON).
#[derive(Debug, Clone, Deserialize)]
pub struct RawResponse(pub serde_json::Value);

impl std::ops::Deref for RawResponse {
    type Target = serde_json::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ============================================================================
// Typed Resolvers (Server)
// ============================================================================

/// A strongly typed resolver function.
///
/// # Type Parameters
///
/// - `Parent`: The parent object type (use `()` for root resolvers)
/// - `Args`: The resolver arguments type
/// - `Ctx`: Context data required by the resolver
/// - `Output`: The resolver output type
pub trait TypedResolver<Parent, Args, Ctx, Output>: Send + Sync + 'static {
    /// The future type returned by the resolver.
    type Future: Future<Output = SdkResult<Output>> + Send + 'static;

    /// Resolves the field.
    fn resolve(&self, parent: Parent, args: Args, ctx: &TypedContext) -> Self::Future;
}

/// Implements TypedResolver for async functions.
impl<F, Fut, Parent, Args, Ctx, Output> TypedResolver<Parent, Args, Ctx, Output> for F
where
    F: Fn(Parent, Args, Ctx) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = SdkResult<Output>> + Send + 'static,
    Parent: Send + 'static,
    Args: DeserializeOwned + Send + 'static,
    Ctx: FromTypedContext + Send + 'static,
    Output: Serialize + Send + 'static,
{
    type Future = Fut;

    fn resolve(&self, parent: Parent, args: Args, ctx: &TypedContext) -> Self::Future {
        let ctx_data = Ctx::from_context(ctx);
        (self)(parent, args, ctx_data)
    }
}

/// Trait for extracting typed data from context.
pub trait FromTypedContext: Sized {
    fn from_context(ctx: &TypedContext) -> Self;
}

/// Unit type extracts nothing.
impl FromTypedContext for () {
    fn from_context(_ctx: &TypedContext) -> Self {}
}

/// Option extracts if present.
impl<T: Clone + 'static> FromTypedContext for Option<T> {
    fn from_context(ctx: &TypedContext) -> Self {
        ctx.get::<T>().cloned()
    }
}

/// Tuple extraction for multiple context values.
impl<A, B> FromTypedContext for (A, B)
where
    A: Clone + 'static,
    B: Clone + 'static,
{
    fn from_context(ctx: &TypedContext) -> Self {
        (
            ctx.get::<A>().cloned().unwrap_or_else(|| panic!("Missing context type: {}", std::any::type_name::<A>())),
            ctx.get::<B>().cloned().unwrap_or_else(|| panic!("Missing context type: {}", std::any::type_name::<B>())),
        )
    }
}

impl<A, B, C> FromTypedContext for (A, B, C)
where
    A: Clone + 'static,
    B: Clone + 'static,
    C: Clone + 'static,
{
    fn from_context(ctx: &TypedContext) -> Self {
        (
            ctx.get::<A>().cloned().unwrap_or_else(|| panic!("Missing context type: {}", std::any::type_name::<A>())),
            ctx.get::<B>().cloned().unwrap_or_else(|| panic!("Missing context type: {}", std::any::type_name::<B>())),
            ctx.get::<C>().cloned().unwrap_or_else(|| panic!("Missing context type: {}", std::any::type_name::<C>())),
        )
    }
}

/// Marker for no arguments.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct NoArgs;

/// Marker for root resolvers (no parent).
pub type Root = ();

// ============================================================================
// Resolver Registration (for macro support)
// ============================================================================

/// Trait for resolver registration metadata (used by macros).
pub trait ResolverRegistration {
    const TYPE_NAME: &'static str;
    const FIELD_NAME: &'static str;
}

// ============================================================================
// Resolver Builder
// ============================================================================

/// Type-safe resolver registration.
pub struct ResolverBuilder<Schema> {
    resolvers: Vec<ResolverEntry>,
    _schema: PhantomData<Schema>,
}

struct ResolverEntry {
    type_name: String,
    field_name: String,
    resolver: BoxedResolver,
}

type BoxedResolver = Arc<
    dyn Fn(
            serde_json::Value,
            serde_json::Value,
            &TypedContext,
        ) -> Pin<Box<dyn Future<Output = SdkResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

impl<Schema> Default for ResolverBuilder<Schema> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Schema> ResolverBuilder<Schema> {
    /// Creates a new resolver builder.
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
            _schema: PhantomData,
        }
    }

    /// Registers a typed resolver for a field.
    ///
    /// # Type Safety
    ///
    /// The compiler ensures:
    /// - `Args` matches the schema's argument types
    /// - `Output` matches the schema's return type
    /// - `Ctx` types are available in the context
    pub fn resolver<Parent, Args, Ctx, Output, F, Fut>(
        mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        resolver: F,
    ) -> Self
    where
        Parent: DeserializeOwned + Send + 'static,
        Args: DeserializeOwned + Send + 'static,
        Ctx: FromTypedContext + Send + 'static,
        Output: Serialize + Send + 'static,
        F: Fn(Parent, Args, Ctx) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = SdkResult<Output>> + Send + 'static,
    {
        let type_name = type_name.into();
        let field_name = field_name.into();

        let boxed: BoxedResolver = Arc::new(move |parent_json, args_json, ctx| {
            let resolver = resolver.clone();

            // Extract context data before the async block (ctx is borrowed)
            let ctx_data = Ctx::from_context(ctx);

            Box::pin(async move {
                // Deserialize parent
                let parent: Parent = serde_json::from_value(parent_json)
                    .map_err(|e| SdkError::new(
                        ErrorCode::DeserializeError,
                        format!("Failed to deserialize parent: {}", e),
                    ))?;

                // Deserialize args
                let args: Args = serde_json::from_value(args_json)
                    .map_err(|e| SdkError::new(
                        ErrorCode::DeserializeError,
                        format!("Failed to deserialize arguments: {}", e),
                    ))?;

                // Call resolver
                let result = resolver(parent, args, ctx_data).await?;

                // Serialize result
                serde_json::to_value(result)
                    .map_err(|e| SdkError::new(
                        ErrorCode::SerializeError,
                        format!("Failed to serialize result: {}", e),
                    ))
            })
        });

        self.resolvers.push(ResolverEntry {
            type_name,
            field_name,
            resolver: boxed,
        });

        self
    }

    /// Registers a root query resolver (no parent).
    pub fn query<Args, Ctx, Output, F, Fut>(
        self,
        field_name: impl Into<String>,
        resolver: F,
    ) -> Self
    where
        Args: DeserializeOwned + Send + 'static,
        Ctx: FromTypedContext + Send + 'static,
        Output: Serialize + Send + 'static,
        F: Fn(Args, Ctx) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = SdkResult<Output>> + Send + 'static,
    {
        self.resolver::<(), Args, Ctx, Output, _, _>(
            "Query",
            field_name,
            move |_parent: (), args, ctx| resolver.clone()(args, ctx),
        )
    }

    /// Registers a root mutation resolver (no parent).
    pub fn mutation<Args, Ctx, Output, F, Fut>(
        self,
        field_name: impl Into<String>,
        resolver: F,
    ) -> Self
    where
        Args: DeserializeOwned + Send + 'static,
        Ctx: FromTypedContext + Send + 'static,
        Output: Serialize + Send + 'static,
        F: Fn(Args, Ctx) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = SdkResult<Output>> + Send + 'static,
    {
        self.resolver::<(), Args, Ctx, Output, _, _>(
            "Mutation",
            field_name,
            move |_parent: (), args, ctx| resolver.clone()(args, ctx),
        )
    }

    /// Returns all registered resolvers.
    pub fn build(self) -> Vec<(String, String, BoxedResolver)> {
        self.resolvers
            .into_iter()
            .map(|e| (e.type_name, e.field_name, e.resolver))
            .collect()
    }
}

// ============================================================================
// Field Definition Macros Support
// ============================================================================

/// Marker trait for types that can be used as GraphQL field arguments.
pub trait GraphQLArgs: DeserializeOwned + Send + 'static {}

impl<T: DeserializeOwned + Send + 'static> GraphQLArgs for T {}

/// Marker trait for types that can be returned from GraphQL fields.
pub trait GraphQLOutput: Serialize + Send + 'static {}

impl<T: Serialize + Send + 'static> GraphQLOutput for T {}

/// Marker trait for types that can be used as GraphQL parent objects.
pub trait GraphQLParent: DeserializeOwned + Send + 'static {}

impl<T: DeserializeOwned + Send + 'static> GraphQLParent for T {}

// ============================================================================
// Type-Safe Field Access
// ============================================================================

/// A typed wrapper for accessing GraphQL arguments.
#[derive(Debug)]
pub struct TypedArgs<T> {
    inner: T,
}

impl<T> TypedArgs<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> std::ops::Deref for TypedArgs<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: DeserializeOwned> TypedArgs<T> {
    /// Parses arguments from a JSON value.
    pub fn from_json(value: serde_json::Value) -> SdkResult<Self> {
        serde_json::from_value(value)
            .map(Self::new)
            .map_err(|e| SdkError::new(
                ErrorCode::DeserializeError,
                format!("Failed to parse arguments: {}", e),
            ))
    }
}

/// A typed wrapper for resolver results.
#[derive(Debug)]
pub struct TypedResult<T> {
    inner: T,
}

impl<T> TypedResult<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Serialize> TypedResult<T> {
    /// Converts the result to JSON.
    pub fn to_json(&self) -> SdkResult<serde_json::Value> {
        serde_json::to_value(&self.inner)
            .map_err(|e| SdkError::new(
                ErrorCode::SerializeError,
                format!("Failed to serialize result: {}", e),
            ))
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// A typed GraphQL response.
#[derive(Debug, Clone)]
pub struct TypedResponse<T> {
    pub data: Option<T>,
    pub errors: Vec<GraphQLError>,
}

impl<T> TypedResponse<T> {
    /// Returns true if the response has errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns the data if present and no errors occurred.
    pub fn into_result(self) -> SdkResult<T> {
        if !self.errors.is_empty() {
            return Err(SdkError::new(
                ErrorCode::ExecutionError,
                self.errors[0].message.clone(),
            ));
        }

        self.data.ok_or_else(|| SdkError::new(
            ErrorCode::NoData,
            "No data in response",
        ))
    }
}

impl<T: DeserializeOwned> TypedResponse<T> {
    /// Parses a typed response from a raw GraphQL response.
    pub fn from_raw(data: Option<serde_json::Value>, errors: Vec<GraphQLError>) -> SdkResult<Self> {
        let typed_data = match data {
            Some(v) => Some(serde_json::from_value(v).map_err(|e| {
                SdkError::new(
                    ErrorCode::DeserializeError,
                    format!("Failed to deserialize response: {}", e),
                )
            })?),
            None => None,
        };

        Ok(Self {
            data: typed_data,
            errors,
        })
    }
}

/// A GraphQL error.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(default)]
    pub path: Vec<serde_json::Value>,
    #[serde(default)]
    pub extensions: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct User {
        id: String,
        name: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    struct GetUserArgs {
        id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct GetUserResponse {
        user: Option<User>,
    }

    #[tokio::test]
    async fn test_resolver_builder() {
        let builder = ResolverBuilder::<()>::new()
            .query::<GetUserArgs, (), User, _, _>("user", |args, _ctx| async move {
                Ok(User {
                    id: args.id,
                    name: "Alice".to_string(),
                })
            });

        let resolvers = builder.build();
        assert_eq!(resolvers.len(), 1);
        assert_eq!(resolvers[0].0, "Query");
        assert_eq!(resolvers[0].1, "user");
    }

    #[test]
    fn test_typed_args() {
        let json = serde_json::json!({ "id": "123" });
        let args: TypedArgs<GetUserArgs> = TypedArgs::from_json(json).unwrap();
        assert_eq!(args.id, "123");
    }

    #[test]
    fn test_typed_response() {
        let response: TypedResponse<GetUserResponse> = TypedResponse {
            data: Some(GetUserResponse {
                user: Some(User {
                    id: "1".to_string(),
                    name: "Alice".to_string(),
                }),
            }),
            errors: vec![],
        };

        assert!(!response.has_errors());
        let result = response.into_result().unwrap();
        assert_eq!(result.user.unwrap().name, "Alice");
    }

    #[test]
    fn test_typed_response_with_errors() {
        let response: TypedResponse<GetUserResponse> = TypedResponse {
            data: None,
            errors: vec![GraphQLError {
                message: "User not found".to_string(),
                path: vec![],
                extensions: None,
            }],
        };

        assert!(response.has_errors());
        let result = response.into_result();
        assert!(result.is_err());
    }
}
