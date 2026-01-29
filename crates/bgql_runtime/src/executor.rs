//! Query execution for Better GraphQL.

<<<<<<< HEAD
use crate::query::QueryPlan;
use crate::schema::Schema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Executor configuration.
#[derive(Debug, Clone, Default)]
=======
use crate::query::{FieldInfo, PlanNode, QueryPlan};
use crate::resolver::{ResolverArgs, ResolverInfo, ResolverMap};
use crate::schema::Schema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Executor configuration.
#[derive(Debug, Clone)]
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
pub struct ExecutorConfig {
    /// Maximum parallel execution depth.
    pub max_parallel_depth: usize,
    /// Enable tracing.
    pub tracing: bool,
<<<<<<< HEAD
}

/// The query executor.
#[derive(Debug)]
pub struct Executor {
    #[allow(dead_code)]
    config: ExecutorConfig,
=======
    /// Maximum concurrent field resolutions.
    pub max_concurrent_fields: usize,
    /// Timeout for field resolution in milliseconds.
    pub field_timeout_ms: u64,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_parallel_depth: 10,
            tracing: false,
            max_concurrent_fields: 100,
            field_timeout_ms: 30000,
        }
    }
}

/// The query executor.
pub struct Executor {
    config: ExecutorConfig,
    resolvers: Arc<ResolverMap>,
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

<<<<<<< HEAD
=======
impl std::fmt::Debug for Executor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Executor")
            .field("config", &self.config)
            .finish()
    }
}

>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
impl Executor {
    /// Creates a new executor.
    pub fn new() -> Self {
        Self {
            config: ExecutorConfig::default(),
<<<<<<< HEAD
=======
            resolvers: Arc::new(ResolverMap::new()),
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
        }
    }

    /// Creates an executor with configuration.
    pub fn with_config(config: ExecutorConfig) -> Self {
<<<<<<< HEAD
        Self { config }
    }

    /// Executes a query plan.
    pub async fn execute(&self, _plan: &QueryPlan, _schema: &Schema, _ctx: &Context) -> Response {
        // TODO: Implement actual execution
        Response {
            data: None,
            errors: None,
=======
        Self {
            config,
            resolvers: Arc::new(ResolverMap::new()),
        }
    }

    /// Creates an executor with resolvers.
    pub fn with_resolvers(resolvers: ResolverMap) -> Self {
        Self {
            config: ExecutorConfig::default(),
            resolvers: Arc::new(resolvers),
        }
    }

    /// Creates an executor with config and resolvers.
    pub fn new_with(config: ExecutorConfig, resolvers: ResolverMap) -> Self {
        Self {
            config,
            resolvers: Arc::new(resolvers),
        }
    }

    /// Gets a reference to the resolvers.
    pub fn resolvers(&self) -> &ResolverMap {
        &self.resolvers
    }

    /// Executes a query plan.
    pub async fn execute(&self, plan: &QueryPlan, schema: &Schema, ctx: &Context) -> Response {
        let exec_ctx = ExecutionContext {
            schema: schema.clone(),
            ctx: ctx.clone(),
            resolvers: Arc::clone(&self.resolvers),
            config: self.config.clone(),
            errors: Arc::new(RwLock::new(Vec::new())),
        };

        // Get root value (empty object for Query/Mutation)
        let root_value = Value::Object(serde_json::Map::new());

        // Execute the plan
        let data = execute_node(&plan.root, root_value, Vec::new(), &exec_ctx).await;

        // Collect errors
        let errors = exec_ctx.errors.read().await;
        let errors = if errors.is_empty() {
            None
        } else {
            Some(errors.clone())
        };

        Response {
            data: Some(data),
            errors,
        }
    }
}

/// Executes a plan node.
fn execute_node<'a>(
    node: &'a PlanNode,
    parent: Value,
    path: Vec<PathSegment>,
    ctx: &'a ExecutionContext,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Value> + Send + 'a>> {
    Box::pin(async move {
        match node {
            PlanNode::Sequence(nodes) => execute_sequence(nodes, parent, path, ctx).await,
            PlanNode::Parallel(nodes) => execute_parallel(nodes, parent, path, ctx).await,
            PlanNode::Field {
                info,
                response_name,
                children,
            } => execute_field(info, response_name, children, parent, path, ctx).await,
            PlanNode::Leaf { field } => execute_leaf(field, parent, path, ctx).await,
            PlanNode::TypeCondition { type_name, node } => {
                // Check if parent matches type condition
                if let Some(typename) = parent.get("__typename").and_then(|v| v.as_str()) {
                    if typename == type_name {
                        return execute_node(node, parent, path, ctx).await;
                    }
                }
                // If no __typename, assume it matches
                execute_node(node, parent, path, ctx).await
            }
            PlanNode::FragmentSpread { name: _ } => {
                // Fragment spreads should be resolved during planning
                Value::Null
            }
            PlanNode::Defer { node, label: _ } => {
                // For now, execute deferred nodes synchronously
                execute_node(node, parent, path, ctx).await
            }
            PlanNode::Stream {
                node,
                label: _,
                initial_count: _,
            } => {
                // For now, execute streamed nodes synchronously
                execute_node(node, parent, path, ctx).await
            }
            PlanNode::Conditional { condition, node } => {
                if *condition {
                    execute_node(node, parent, path, ctx).await
                } else {
                    Value::Null
                }
            }
        }
    })
}

/// Executes nodes sequentially.
async fn execute_sequence(
    nodes: &[PlanNode],
    parent: Value,
    path: Vec<PathSegment>,
    ctx: &ExecutionContext,
) -> Value {
    let mut result = serde_json::Map::new();

    for node in nodes {
        let value = execute_node(node, parent.clone(), path.clone(), ctx).await;

        // Merge result into the object
        if let Value::Object(map) = value {
            for (k, v) in map {
                result.insert(k, v);
            }
        }
    }

    Value::Object(result)
}

/// Executes nodes in parallel.
async fn execute_parallel(
    nodes: &[PlanNode],
    parent: Value,
    path: Vec<PathSegment>,
    ctx: &ExecutionContext,
) -> Value {
    let mut handles = Vec::with_capacity(nodes.len());

    for node in nodes {
        let parent = parent.clone();
        let path = path.clone();
        let resolvers = Arc::clone(&ctx.resolvers);
        let errors = Arc::clone(&ctx.errors);
        let config = ctx.config.clone();
        let schema = ctx.schema.clone();
        let user_ctx = ctx.ctx.clone();
        let node = node.clone();

        handles.push(tokio::spawn(async move {
            let local_ctx = ExecutionContext {
                schema,
                ctx: user_ctx,
                resolvers,
                config,
                errors,
            };
            execute_node(&node, parent, path, &local_ctx).await
        }));
    }

    let mut result = serde_json::Map::new();

    for handle in handles {
        match handle.await {
            Ok(value) => {
                if let Value::Object(map) = value {
                    for (k, v) in map {
                        result.insert(k, v);
                    }
                }
            }
            Err(e) => {
                let mut errors = ctx.errors.write().await;
                errors.push(FieldError::new(format!("Parallel execution failed: {}", e)));
            }
        }
    }

    Value::Object(result)
}

/// Executes a field with nested selections.
async fn execute_field(
    info: &FieldInfo,
    response_name: &str,
    children: &PlanNode,
    parent: Value,
    path: Vec<PathSegment>,
    ctx: &ExecutionContext,
) -> Value {
    // Resolve the field value
    let field_value = resolve_field(info, &parent, path.clone(), ctx).await;

    // If the field resolved to an array, we need to execute children for each item
    let result = match field_value {
        Value::Array(items) => {
            let mut results = Vec::with_capacity(items.len());
            for (i, item) in items.into_iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(PathSegment::Index(i));
                let child_result = execute_node(children, item, child_path, ctx).await;
                results.push(child_result);
            }
            Value::Array(results)
        }
        Value::Null => Value::Null,
        other => {
            // Execute children with the resolved value as parent
            execute_node(children, other, path, ctx).await
        }
    };

    // Create an object with the response name
    let mut obj = serde_json::Map::new();
    obj.insert(response_name.to_string(), result);
    Value::Object(obj)
}

/// Executes a leaf field.
async fn execute_leaf(
    info: &FieldInfo,
    parent: Value,
    path: Vec<PathSegment>,
    ctx: &ExecutionContext,
) -> Value {
    let response_key = info.response_key();
    let value = resolve_field(info, &parent, path, ctx).await;

    let mut obj = serde_json::Map::new();
    obj.insert(response_key.to_string(), value);
    Value::Object(obj)
}

/// Resolves a single field.
async fn resolve_field(
    info: &FieldInfo,
    parent: &Value,
    mut path: Vec<PathSegment>,
    ctx: &ExecutionContext,
) -> Value {
    // Handle __typename specially
    if info.is_introspection && info.name == "__typename" {
        return Value::String(info.parent_type.clone());
    }

    // Build resolver args
    let args = ResolverArgs::from_pairs(info.arguments.clone());

    // Add field to path
    path.push(PathSegment::Field(info.response_key().to_string()));

    // Build resolver info
    let resolver_info = ResolverInfo::new(&info.name, &info.parent_type)
        .with_return_type(&info.return_type)
        .with_path(path.clone());

    // Get the resolver
    let resolver = ctx.resolvers.get(&info.parent_type, &info.name);

    match resolver {
        Some(r) => {
            let result = r.resolve(parent, &args, &ctx.ctx, &resolver_info).await;

            match result {
                Ok(value) => value,
                Err(e) => {
                    let mut errors = ctx.errors.write().await;
                    errors.push(FieldError::new(e.to_string()).with_path(path));
                    Value::Null
                }
            }
        }
        None => {
            // No resolver found, try default property access
            parent.get(&info.name).cloned().unwrap_or(Value::Null)
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
        }
    }
}

/// Execution context.
<<<<<<< HEAD
#[derive(Debug)]
pub struct Context {
    /// Request-scoped data.
    pub data: HashMap<String, serde_json::Value>,
=======
#[derive(Clone)]
struct ExecutionContext {
    schema: Schema,
    ctx: Context,
    resolvers: Arc<ResolverMap>,
    config: ExecutorConfig,
    errors: Arc<RwLock<Vec<FieldError>>>,
}

/// Execution context.
#[derive(Debug, Clone)]
pub struct Context {
    /// Request-scoped data.
    pub data: HashMap<String, serde_json::Value>,
    /// Variables from the request.
    pub variables: HashMap<String, serde_json::Value>,
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Creates a new context.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
<<<<<<< HEAD
=======
            variables: HashMap::new(),
        }
    }

    /// Creates a context with variables.
    pub fn with_variables(variables: HashMap<String, serde_json::Value>) -> Self {
        Self {
            data: HashMap::new(),
            variables,
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
        }
    }

    /// Sets a value in the context.
    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
    }

    /// Gets a value from the context.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
<<<<<<< HEAD
=======

    /// Gets a variable by name.
    pub fn variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// Gets a variable as a specific type.
    pub fn variable_as<T: for<'de> Deserialize<'de>>(&self, name: &str) -> Option<T> {
        self.variables
            .get(name)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
}

/// A GraphQL response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// The data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// The errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

<<<<<<< HEAD
=======
impl Response {
    /// Creates a successful response with data.
    pub fn data(data: serde_json::Value) -> Self {
        Self {
            data: Some(data),
            errors: None,
        }
    }

    /// Creates an error response.
    pub fn error(error: FieldError) -> Self {
        Self {
            data: None,
            errors: Some(vec![error]),
        }
    }

    /// Creates an error response with multiple errors.
    pub fn errors(errors: Vec<FieldError>) -> Self {
        Self {
            data: None,
            errors: Some(errors),
        }
    }

    /// Returns true if the response has errors.
    pub fn has_errors(&self) -> bool {
        self.errors.as_ref().map(|e| !e.is_empty()).unwrap_or(false)
    }

    /// Returns true if the response has data.
    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }
}

>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
/// A field error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    /// The error message.
    pub message: String,
    /// The path to the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<PathSegment>>,
    /// Error extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

/// A path segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathSegment {
    Field(String),
    Index(usize),
}

impl FieldError {
    /// Creates a new field error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
            extensions: None,
        }
    }

    /// Adds a path to the error.
    pub fn with_path(mut self, path: Vec<PathSegment>) -> Self {
        self.path = Some(path);
        self
    }
<<<<<<< HEAD
=======

    /// Adds an extension.
    pub fn with_extension(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extensions
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value);
        self
    }

    /// Sets the error code extension.
    pub fn with_code(self, code: impl Into<String>) -> Self {
        self.with_extension("code", serde_json::Value::String(code.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{FieldInfo, PlanNode, QueryPlan};
    use crate::resolver::{FnResolver, ResolverMap};
    use crate::schema::{FieldDef, ObjectDef, SchemaBuilder, TypeDef, TypeRef};
    use bgql_semantic::hir::HirOperationKind;
    use indexmap::IndexMap;

    fn create_test_schema() -> Schema {
        let mut user_fields = IndexMap::new();
        user_fields.insert(
            "id".to_string(),
            FieldDef {
                name: "id".to_string(),
                description: None,
                ty: TypeRef::Named("ID".to_string()),
                arguments: IndexMap::new(),
                deprecated: false,
                deprecation_reason: None,
            },
        );
        user_fields.insert(
            "name".to_string(),
            FieldDef {
                name: "name".to_string(),
                description: None,
                ty: TypeRef::Named("String".to_string()),
                arguments: IndexMap::new(),
                deprecated: false,
                deprecation_reason: None,
            },
        );

        let mut query_fields = IndexMap::new();
        query_fields.insert(
            "user".to_string(),
            FieldDef {
                name: "user".to_string(),
                description: None,
                ty: TypeRef::Named("User".to_string()),
                arguments: IndexMap::new(),
                deprecated: false,
                deprecation_reason: None,
            },
        );

        SchemaBuilder::new()
            .query_type("Query")
            .add_type(TypeDef::Object(ObjectDef {
                name: "Query".to_string(),
                description: None,
                fields: query_fields,
                implements: Vec::new(),
            }))
            .add_type(TypeDef::Object(ObjectDef {
                name: "User".to_string(),
                description: None,
                fields: user_fields,
                implements: Vec::new(),
            }))
            .build()
    }

    #[tokio::test]
    async fn test_execute_simple_query() {
        let mut resolvers = ResolverMap::new();

        resolvers.register(
            "Query",
            "user",
            FnResolver::new(|_parent, _args, _ctx, _info| {
                Ok(serde_json::json!({"id": "1", "name": "Alice"}))
            }),
        );

        let executor = Executor::with_resolvers(resolvers);
        let schema = create_test_schema();
        let ctx = Context::new();

        let plan = QueryPlan {
            root: PlanNode::Field {
                info: FieldInfo {
                    name: "user".to_string(),
                    alias: None,
                    parent_type: "Query".to_string(),
                    return_type: "User".to_string(),
                    arguments: Vec::new(),
                    is_introspection: false,
                },
                response_name: "user".to_string(),
                children: Box::new(PlanNode::Parallel(vec![
                    PlanNode::Leaf {
                        field: FieldInfo {
                            name: "id".to_string(),
                            alias: None,
                            parent_type: "User".to_string(),
                            return_type: "ID".to_string(),
                            arguments: Vec::new(),
                            is_introspection: false,
                        },
                    },
                    PlanNode::Leaf {
                        field: FieldInfo {
                            name: "name".to_string(),
                            alias: None,
                            parent_type: "User".to_string(),
                            return_type: "String".to_string(),
                            arguments: Vec::new(),
                            is_introspection: false,
                        },
                    },
                ])),
            },
            operation_name: None,
            operation_kind: HirOperationKind::Query,
            complexity: 0,
            max_depth: 0,
        };

        let response = executor.execute(&plan, &schema, &ctx).await;

        assert!(response.data.is_some());
        assert!(!response.has_errors());

        let data = response.data.unwrap();
        assert_eq!(data["user"]["id"], "1");
        assert_eq!(data["user"]["name"], "Alice");
    }

    #[tokio::test]
    async fn test_execute_typename() {
        let resolvers = ResolverMap::new();
        let executor = Executor::with_resolvers(resolvers);
        let schema = create_test_schema();
        let ctx = Context::new();

        let plan = QueryPlan {
            root: PlanNode::Leaf {
                field: FieldInfo {
                    name: "__typename".to_string(),
                    alias: None,
                    parent_type: "Query".to_string(),
                    return_type: "String".to_string(),
                    arguments: Vec::new(),
                    is_introspection: true,
                },
            },
            operation_name: None,
            operation_kind: HirOperationKind::Query,
            complexity: 0,
            max_depth: 0,
        };

        let response = executor.execute(&plan, &schema, &ctx).await;

        assert!(response.data.is_some());
        let data = response.data.unwrap();
        assert_eq!(data["__typename"], "Query");
    }

    #[tokio::test]
    async fn test_execute_with_arguments() {
        let mut resolvers = ResolverMap::new();

        resolvers.register_fn("Query", "user", |_parent, args, _ctx, _info| {
            let id: String = args.require("id")?;
            Ok(serde_json::json!({"id": id, "name": "User"}))
        });

        let executor = Executor::with_resolvers(resolvers);
        let schema = create_test_schema();
        let ctx = Context::new();

        let plan = QueryPlan {
            root: PlanNode::Leaf {
                field: FieldInfo {
                    name: "user".to_string(),
                    alias: None,
                    parent_type: "Query".to_string(),
                    return_type: "User".to_string(),
                    arguments: vec![("id".to_string(), serde_json::json!("42"))],
                    is_introspection: false,
                },
            },
            operation_name: None,
            operation_kind: HirOperationKind::Query,
            complexity: 0,
            max_depth: 0,
        };

        let response = executor.execute(&plan, &schema, &ctx).await;

        assert!(response.data.is_some());
        let data = response.data.unwrap();
        assert_eq!(data["user"]["id"], "42");
    }

    #[tokio::test]
    async fn test_execute_with_error() {
        let mut resolvers = ResolverMap::new();

        resolvers.register_fn("Query", "user", |_parent, _args, _ctx, _info| {
            Err(crate::resolver::ResolverError::Custom(
                "User not found".to_string(),
            ))
        });

        let executor = Executor::with_resolvers(resolvers);
        let schema = create_test_schema();
        let ctx = Context::new();

        let plan = QueryPlan {
            root: PlanNode::Leaf {
                field: FieldInfo {
                    name: "user".to_string(),
                    alias: None,
                    parent_type: "Query".to_string(),
                    return_type: "User".to_string(),
                    arguments: Vec::new(),
                    is_introspection: false,
                },
            },
            operation_name: None,
            operation_kind: HirOperationKind::Query,
            complexity: 0,
            max_depth: 0,
        };

        let response = executor.execute(&plan, &schema, &ctx).await;

        assert!(response.has_errors());
        let errors = response.errors.unwrap();
        assert!(errors[0].message.contains("User not found"));
    }

    #[tokio::test]
    async fn test_execute_list_field() {
        let mut resolvers = ResolverMap::new();

        resolvers.register_fn("Query", "users", |_parent, _args, _ctx, _info| {
            Ok(serde_json::json!([
                {"id": "1", "name": "Alice"},
                {"id": "2", "name": "Bob"}
            ]))
        });

        let executor = Executor::with_resolvers(resolvers);
        let schema = create_test_schema();
        let ctx = Context::new();

        let plan = QueryPlan {
            root: PlanNode::Field {
                info: FieldInfo {
                    name: "users".to_string(),
                    alias: None,
                    parent_type: "Query".to_string(),
                    return_type: "User".to_string(),
                    arguments: Vec::new(),
                    is_introspection: false,
                },
                response_name: "users".to_string(),
                children: Box::new(PlanNode::Parallel(vec![
                    PlanNode::Leaf {
                        field: FieldInfo {
                            name: "id".to_string(),
                            alias: None,
                            parent_type: "User".to_string(),
                            return_type: "ID".to_string(),
                            arguments: Vec::new(),
                            is_introspection: false,
                        },
                    },
                    PlanNode::Leaf {
                        field: FieldInfo {
                            name: "name".to_string(),
                            alias: None,
                            parent_type: "User".to_string(),
                            return_type: "String".to_string(),
                            arguments: Vec::new(),
                            is_introspection: false,
                        },
                    },
                ])),
            },
            operation_name: None,
            operation_kind: HirOperationKind::Query,
            complexity: 0,
            max_depth: 0,
        };

        let response = executor.execute(&plan, &schema, &ctx).await;

        assert!(response.data.is_some());
        let data = response.data.unwrap();
        let users = data["users"].as_array().unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0]["id"], "1");
        assert_eq!(users[1]["name"], "Bob");
    }

    #[test]
    fn test_context() {
        let mut ctx = Context::new();
        ctx.set("user_id", "123");

        assert_eq!(ctx.get::<String>("user_id"), Some("123".to_string()));
        assert_eq!(ctx.get::<String>("missing"), None);
    }

    #[test]
    fn test_context_with_variables() {
        let mut vars = HashMap::new();
        vars.insert("id".to_string(), serde_json::json!("42"));

        let ctx = Context::with_variables(vars);
        assert_eq!(ctx.variable("id"), Some(&serde_json::json!("42")));
        assert_eq!(ctx.variable_as::<String>("id"), Some("42".to_string()));
    }

    #[test]
    fn test_field_error() {
        let error = FieldError::new("Something went wrong")
            .with_path(vec![
                PathSegment::Field("user".to_string()),
                PathSegment::Field("name".to_string()),
            ])
            .with_code("NOT_FOUND");

        assert_eq!(error.message, "Something went wrong");
        assert!(error.path.is_some());
        assert!(error.extensions.is_some());
    }

    #[test]
    fn test_response() {
        let data_response = Response::data(serde_json::json!({"hello": "world"}));
        assert!(data_response.has_data());
        assert!(!data_response.has_errors());

        let error_response = Response::error(FieldError::new("Error"));
        assert!(!error_response.has_data());
        assert!(error_response.has_errors());
    }
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
}
