//! Better GraphQL Server SDK.
//!
//! Provides a type-safe GraphQL server with:
//! - Automatic DataLoader integration
//! - Input validation
//! - Middleware support
//! - Streaming (@defer/@stream)
//! - Type-safe resolvers with automatic context extraction

use crate::context::TypedContext;
use crate::error::{ErrorCode, SdkError, SdkResult};

// Legacy re-exports for backwards compatibility
pub use crate::result::{BgqlError, BgqlResult};
use bgql_core::Interner;
use bgql_runtime::executor::{Context as RuntimeContext, Executor, ExecutorConfig};
use bgql_runtime::query::{QueryPlanner, PlannerConfig};
use bgql_runtime::resolver::ResolverMap;
use bgql_runtime::schema::{
    Schema, SchemaBuilder, TypeDef, ObjectDef, FieldDef, TypeRef, InputFieldDef,
    ScalarDef, InterfaceDef, UnionDef, EnumDef, EnumValueDef, InputObjectDef,
};
use bgql_semantic::hir::{HirOperation, HirOperationKind, HirSelection, HirFieldSelection, HirValue};
use bgql_syntax::{parse, Definition, TypeDefinition, OperationType};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use indexmap::IndexMap;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to listen on.
    pub port: u16,
    /// Host to bind to.
    pub host: String,
    /// Enable introspection.
    pub introspection: bool,
    /// Enable playground.
    pub playground: bool,
    /// Maximum query depth.
    pub max_depth: usize,
    /// Maximum query complexity.
    pub max_complexity: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self {
            port: 4000,
            host: "localhost".to_string(),
            introspection: true,
            playground: true,
            max_depth: 10,
            max_complexity: 1000,
        }
    }

    /// Sets the port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the host.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Disables introspection.
    pub fn no_introspection(mut self) -> Self {
        self.introspection = false;
        self
    }

    /// Disables playground.
    pub fn no_playground(mut self) -> Self {
        self.playground = false;
        self
    }
}

/// Request context (legacy API, prefer TypedContext for new code).
#[derive(Debug)]
pub struct Context {
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Request-scoped data.
    pub data: HashMap<String, serde_json::Value>,
    /// Type-safe data storage.
    typed: TypedContext,
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
            headers: HashMap::new(),
            data: HashMap::new(),
            typed: TypedContext::new(),
        }
    }

    /// Creates a context from a TypedContext.
    pub fn from_typed(typed: TypedContext) -> Self {
        Self {
            headers: typed.headers().clone(),
            data: HashMap::new(),
            typed,
        }
    }

    /// Returns the typed context.
    pub fn typed(&self) -> &TypedContext {
        &self.typed
    }

    /// Returns a mutable typed context.
    pub fn typed_mut(&mut self) -> &mut TypedContext {
        &mut self.typed
    }

    /// Sets a value in the context (legacy string-keyed API).
    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
    }

    /// Gets a value from the context (legacy string-keyed API).
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Inserts a typed value (type-safe API).
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.typed.insert(value);
    }

    /// Gets a typed value by type (type-safe API).
    pub fn get_typed<T: 'static>(&self) -> Option<&T> {
        self.typed.get::<T>()
    }

    /// Gets a header value.
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }

    /// Converts to runtime context.
    fn to_runtime_context(&self, variables: Option<serde_json::Value>) -> RuntimeContext {
        let mut ctx = RuntimeContext::new();

        // Copy data to runtime context
        for (key, value) in &self.data {
            ctx.data.insert(key.clone(), value.clone());
        }

        // Add variables if provided
        if let Some(serde_json::Value::Object(vars)) = variables {
            for (key, value) in vars {
                ctx.variables.insert(key, value);
            }
        }

        ctx
    }
}

/// Resolver function type.
pub type ResolverFn = Arc<
    dyn Fn(
            serde_json::Value,
            Context,
        ) -> Pin<Box<dyn Future<Output = SdkResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

/// A resolver.
pub struct Resolver {
    type_name: String,
    field_name: String,
    func: ResolverFn,
}

impl Resolver {
    /// Creates a new resolver.
    pub fn new<F, Fut>(type_name: impl Into<String>, field_name: impl Into<String>, func: F) -> Self
    where
        F: Fn(serde_json::Value, Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = SdkResult<serde_json::Value>> + Send + 'static,
    {
        Self {
            type_name: type_name.into(),
            field_name: field_name.into(),
            func: Arc::new(move |args, ctx| Box::pin(func(args, ctx))),
        }
    }
}

/// Server builder.
#[derive(Default)]
pub struct ServerBuilder {
    config: ServerConfig,
    #[allow(dead_code)]
    schema: Option<Schema>,
    sdl: Option<String>,
    resolvers: Vec<Resolver>,
    interner: Interner,
}

/// Server builder with shared state.
pub struct StatefulServerBuilder<S: Clone + Send + Sync + 'static> {
    inner: ServerBuilder,
    state: Arc<S>,
}

impl<S: Clone + Send + Sync + 'static> StatefulServerBuilder<S> {
    /// Adds a resolver with automatic state cloning.
    pub fn resolver<F, Fut>(
        mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        func: F,
    ) -> Self
    where
        F: Fn(Arc<S>, serde_json::Value, Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = SdkResult<serde_json::Value>> + Send + 'static,
    {
        let state = self.state.clone();
        let type_name = type_name.into();
        let field_name = field_name.into();

        self.inner.resolvers.push(Resolver {
            type_name,
            field_name,
            func: Arc::new(move |args, ctx| {
                let s = state.clone();
                let f = func;
                Box::pin(async move { f(s, args, ctx).await })
            }),
        });
        self
    }

    /// Sets the configuration.
    pub fn config(mut self, config: ServerConfig) -> Self {
        self.inner.config = config;
        self
    }

    /// Sets the schema from SDL.
    pub fn schema_sdl(mut self, sdl: impl Into<String>) -> Self {
        self.inner.sdl = Some(sdl.into());
        self
    }

    /// Sets the schema from a file path.
    pub fn schema_file(mut self, path: impl Into<String>) -> Self {
        self.inner = self.inner.schema_file(path);
        self
    }

    /// Builds the server.
    pub fn build(self) -> SdkResult<BgqlServer> {
        self.inner.build()
    }
}

impl ServerBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the configuration.
    pub fn config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the schema from a file path.
    pub fn schema_file(mut self, path: impl Into<String>) -> Self {
        let path_str = path.into();
        match std::fs::read_to_string(&path_str) {
            Ok(content) => {
                self.sdl = Some(content);
            }
            Err(e) => {
                eprintln!("[bgql] Warning: Could not read schema file '{}': {}", path_str, e);
            }
        }
        self
    }

    /// Sets the schema from SDL.
    pub fn schema_sdl(mut self, sdl: impl Into<String>) -> Self {
        self.sdl = Some(sdl.into());
        self
    }

    /// Adds a resolver.
    pub fn resolver<F, Fut>(
        mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        func: F,
    ) -> Self
    where
        F: Fn(serde_json::Value, Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = SdkResult<serde_json::Value>> + Send + 'static,
    {
        self.resolvers
            .push(Resolver::new(type_name, field_name, func));
        self
    }

    /// Builds the server.
    pub fn build(mut self) -> SdkResult<BgqlServer> {
        // Parse schema from SDL if provided
        let schema = if let Some(sdl) = &self.sdl {
            parse_sdl_to_schema(sdl, &self.interner)?
        } else {
            return Err(SdkError::new(ErrorCode::NoSchema, "Schema is required"));
        };

        // Build resolver map from provided resolvers
        let mut resolver_map = ResolverMap::new();
        for resolver in std::mem::take(&mut self.resolvers) {
            let func = resolver.func.clone();
            resolver_map.register_async(
                resolver.type_name.clone(),
                resolver.field_name.clone(),
                move |parent, args, _ctx, _info| {
                    let func = func.clone();
                    let args_json = serde_json::to_value(args.all()).unwrap_or(serde_json::Value::Null);
                    let _parent = parent.clone();
                    async move {
                        // Create SDK context from args
                        let sdk_ctx = Context::new();
                        match func(args_json, sdk_ctx).await {
                            Ok(value) => Ok(value),
                            Err(e) => Err(bgql_runtime::resolver::ResolverError::Custom(e.message)),
                        }
                    }
                },
            );
        }

        let executor_config = ExecutorConfig {
            max_parallel_depth: self.config.max_depth,
            tracing: false,
            max_concurrent_fields: 100,
            field_timeout_ms: 30000,
        };

        let executor = Executor::new_with(executor_config, resolver_map);

        let planner_config = PlannerConfig {
            max_depth: self.config.max_depth,
            max_complexity: self.config.max_complexity,
            enable_parallel: true,
            parallel_threshold: 2,
        };

        let planner = QueryPlanner::with_config(planner_config);

        Ok(BgqlServer {
            config: self.config,
            schema,
            executor,
            planner,
            interner: self.interner,
        })
    }
}

/// The Better GraphQL server.
pub struct BgqlServer {
    config: ServerConfig,
    schema: Schema,
    executor: Executor,
    planner: QueryPlanner,
    interner: Interner,
}

impl BgqlServer {
    /// Creates a new server builder.
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Returns a reference to the schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Starts the server and blocks until shutdown.
    ///
    /// Handles:
    /// - POST /bgql - GraphQL queries and mutations
    /// - GET /bgql - Playground UI (if enabled)
    /// - GET /health - Health check endpoint
    /// - GET /.well-known/bgql - Server capabilities
    pub async fn listen(self) -> SdkResult<()> {
        crate::http::run_server(Arc::new(self)).await
    }

    /// Executes a query.
    pub async fn execute(
        &self,
        query: &str,
        variables: Option<serde_json::Value>,
        ctx: Context,
    ) -> SdkResult<serde_json::Value> {
        // Parse the query
        let parse_result = parse(query, &self.interner);

        if parse_result.diagnostics.has_errors() {
            return Err(SdkError::parse(format!(
                "Parse errors: {:?}",
                parse_result.diagnostics
            )));
        }

        // Find the operation definition
        let operation_def = parse_result.document.definitions
            .iter()
            .find_map(|def| {
                if let Definition::Operation(op) = def {
                    Some(op)
                } else {
                    None
                }
            })
            .ok_or_else(|| SdkError::new(ErrorCode::NoOperation, "No operation found in query"))?;

        // Convert AST operation to HIR operation
        let hir_operation = ast_operation_to_hir(operation_def, &self.interner);

        // Plan the query
        let plan = self.planner.plan(&hir_operation, &self.schema)
            .map_err(|e| SdkError::new(ErrorCode::PlanError, e.message))?;

        // Execute the plan
        let runtime_ctx = ctx.to_runtime_context(variables);
        let response = self.executor.execute(&plan, &self.schema, &runtime_ctx).await;

        // Convert response to JSON
        if response.has_errors() {
            let errors: Vec<String> = response.errors
                .unwrap_or_default()
                .iter()
                .map(|e| e.message.clone())
                .collect();

            let mut result = serde_json::Map::new();
            if let Some(data) = response.data {
                result.insert("data".to_string(), data);
            }
            result.insert("errors".to_string(), serde_json::json!(errors));
            Ok(serde_json::Value::Object(result))
        } else {
            let mut result = serde_json::Map::new();
            if let Some(data) = response.data {
                result.insert("data".to_string(), data);
            }
            Ok(serde_json::Value::Object(result))
        }
    }
}

/// Parses SDL string to Schema.
fn parse_sdl_to_schema(sdl: &str, interner: &Interner) -> SdkResult<Schema> {
    let parse_result = parse(sdl, interner);

    if parse_result.diagnostics.has_errors() {
        return Err(SdkError::new(
            ErrorCode::SchemaError,
            format!("Schema parse errors: {:?}", parse_result.diagnostics),
        ));
    }

    let mut builder = SchemaBuilder::new();
    let mut query_type = None;
    let mut mutation_type = None;
    let mut subscription_type = None;

    for definition in &parse_result.document.definitions {
        match definition {
            Definition::Schema(schema_def) => {
                for op in &schema_def.operations {
                    let type_name = interner.get(op.type_name).to_string();
                    match op.operation {
                        OperationType::Query => query_type = Some(type_name),
                        OperationType::Mutation => mutation_type = Some(type_name),
                        OperationType::Subscription => subscription_type = Some(type_name),
                    }
                }
            }
            Definition::Type(type_def) => {
                let type_def = convert_type_definition(type_def, interner);
                builder = builder.add_type(type_def);
            }
            _ => {}
        }
    }

    // Set root types (default to Query if not specified)
    if let Some(qt) = query_type {
        builder = builder.query_type(qt);
    } else {
        // Check if there's a type named "Query"
        for def in &parse_result.document.definitions {
            if let Definition::Type(TypeDefinition::Object(obj)) = def {
                let name = interner.get(obj.name.value);
                if name == "Query" {
                    builder = builder.query_type("Query");
                    break;
                }
            }
        }
    }

    if let Some(mt) = mutation_type {
        builder = builder.mutation_type(mt);
    }

    if let Some(st) = subscription_type {
        builder = builder.subscription_type(st);
    }

    Ok(builder.build())
}

/// Converts AST type definition to runtime TypeDef.
fn convert_type_definition(type_def: &TypeDefinition, interner: &Interner) -> TypeDef {
    match type_def {
        TypeDefinition::Scalar(scalar) => {
            TypeDef::Scalar(ScalarDef {
                name: interner.get(scalar.name.value).to_string(),
                description: scalar.description.as_ref().map(|d| d.value.to_string()),
            })
        }
        TypeDefinition::Object(obj) => {
            let mut fields = IndexMap::new();
            for field in &obj.fields {
                let field_name = interner.get(field.name.value).to_string();
                let mut arguments = IndexMap::new();
                for arg in &field.arguments {
                    let arg_name = interner.get(arg.name.value).to_string();
                    arguments.insert(arg_name.clone(), InputFieldDef {
                        name: arg_name,
                        description: arg.description.as_ref().map(|d| d.value.to_string()),
                        ty: convert_type(&arg.ty, interner),
                        default_value: None,
                    });
                }
                fields.insert(field_name.clone(), FieldDef {
                    name: field_name,
                    description: field.description.as_ref().map(|d| d.value.to_string()),
                    ty: convert_type(&field.ty, interner),
                    arguments,
                    deprecated: false,
                    deprecation_reason: None,
                });
            }
            TypeDef::Object(ObjectDef {
                name: interner.get(obj.name.value).to_string(),
                description: obj.description.as_ref().map(|d| d.value.to_string()),
                fields,
                implements: obj.implements.iter().map(|n| interner.get(n.value).to_string()).collect(),
            })
        }
        TypeDefinition::Interface(iface) => {
            let mut fields = IndexMap::new();
            for field in &iface.fields {
                let field_name = interner.get(field.name.value).to_string();
                let mut arguments = IndexMap::new();
                for arg in &field.arguments {
                    let arg_name = interner.get(arg.name.value).to_string();
                    arguments.insert(arg_name.clone(), InputFieldDef {
                        name: arg_name,
                        description: arg.description.as_ref().map(|d| d.value.to_string()),
                        ty: convert_type(&arg.ty, interner),
                        default_value: None,
                    });
                }
                fields.insert(field_name.clone(), FieldDef {
                    name: field_name,
                    description: field.description.as_ref().map(|d| d.value.to_string()),
                    ty: convert_type(&field.ty, interner),
                    arguments,
                    deprecated: false,
                    deprecation_reason: None,
                });
            }
            TypeDef::Interface(InterfaceDef {
                name: interner.get(iface.name.value).to_string(),
                description: iface.description.as_ref().map(|d| d.value.to_string()),
                fields,
                implements: iface.implements.iter().map(|n| interner.get(n.value).to_string()).collect(),
            })
        }
        TypeDefinition::Union(union_def) => {
            TypeDef::Union(UnionDef {
                name: interner.get(union_def.name.value).to_string(),
                description: union_def.description.as_ref().map(|d| d.value.to_string()),
                members: union_def.members.iter().map(|n| interner.get(n.value).to_string()).collect(),
            })
        }
        TypeDefinition::Enum(enum_def) => {
            let values = enum_def.values.iter().map(|v| EnumValueDef {
                name: interner.get(v.name.value).to_string(),
                description: v.description.as_ref().map(|d| d.value.to_string()),
                deprecated: false,
                deprecation_reason: None,
            }).collect();
            TypeDef::Enum(EnumDef {
                name: interner.get(enum_def.name.value).to_string(),
                description: enum_def.description.as_ref().map(|d| d.value.to_string()),
                values,
            })
        }
        TypeDefinition::Input(input) => {
            let mut fields = IndexMap::new();
            for field in &input.fields {
                let field_name = interner.get(field.name.value).to_string();
                fields.insert(field_name.clone(), InputFieldDef {
                    name: field_name,
                    description: field.description.as_ref().map(|d| d.value.to_string()),
                    ty: convert_type(&field.ty, interner),
                    default_value: None,
                });
            }
            TypeDef::InputObject(InputObjectDef {
                name: interner.get(input.name.value).to_string(),
                description: input.description.as_ref().map(|d| d.value.to_string()),
                fields,
            })
        }
        // Handle other type definitions as scalars for now
        TypeDefinition::Opaque(opaque) => {
            TypeDef::Scalar(ScalarDef {
                name: interner.get(opaque.name.value).to_string(),
                description: opaque.description.as_ref().map(|d| d.value.to_string()),
            })
        }
        TypeDefinition::TypeAlias(alias) => {
            TypeDef::Scalar(ScalarDef {
                name: interner.get(alias.name.value).to_string(),
                description: alias.description.as_ref().map(|d| d.value.to_string()),
            })
        }
        TypeDefinition::InputUnion(input_union) => {
            TypeDef::Union(UnionDef {
                name: interner.get(input_union.name.value).to_string(),
                description: input_union.description.as_ref().map(|d| d.value.to_string()),
                members: input_union.members.iter().map(|n| interner.get(n.value).to_string()).collect(),
            })
        }
        TypeDefinition::InputEnum(input_enum) => {
            let values = input_enum.variants.iter().map(|v| EnumValueDef {
                name: interner.get(v.name.value).to_string(),
                description: v.description.as_ref().map(|d| d.value.to_string()),
                deprecated: false,
                deprecation_reason: None,
            }).collect();
            TypeDef::Enum(EnumDef {
                name: interner.get(input_enum.name.value).to_string(),
                description: input_enum.description.as_ref().map(|d| d.value.to_string()),
                values,
            })
        }
    }
}

/// Converts AST type to runtime TypeRef.
fn convert_type(ty: &bgql_syntax::Type, interner: &Interner) -> TypeRef {
    match ty {
        bgql_syntax::Type::Named(named) => {
            TypeRef::Named(interner.get(named.name).to_string())
        }
        bgql_syntax::Type::Option(inner, _) => {
            TypeRef::Option(Box::new(convert_type(inner, interner)))
        }
        bgql_syntax::Type::List(inner, _) => {
            TypeRef::List(Box::new(convert_type(inner, interner)))
        }
        bgql_syntax::Type::Generic(generic) => {
            // Treat generic types as named types for now
            TypeRef::Named(interner.get(generic.name).to_string())
        }
        bgql_syntax::Type::Tuple(_) => {
            // Treat tuple types as named types for now
            TypeRef::Named("Tuple".to_string())
        }
        bgql_syntax::Type::_Phantom(_) => {
            // Phantom variant, should not occur in practice
            TypeRef::Named("Unknown".to_string())
        }
    }
}

/// Converts AST operation to HIR operation.
fn ast_operation_to_hir(
    op: &bgql_syntax::OperationDefinition,
    interner: &Interner,
) -> HirOperation {
    let kind = match op.operation {
        OperationType::Query => HirOperationKind::Query,
        OperationType::Mutation => HirOperationKind::Mutation,
        OperationType::Subscription => HirOperationKind::Subscription,
    };

    let name = op.name.as_ref().map(|n| interner.get(n.value).to_string());

    let selections = op.selection_set.selections
        .iter()
        .map(|sel| convert_selection(sel, interner))
        .collect();

    HirOperation {
        kind,
        name,
        variables: vec![], // TODO: convert variables
        selections,
        span: op.span,
    }
}

/// Converts AST selection to HIR selection.
fn convert_selection(
    sel: &bgql_syntax::Selection,
    interner: &Interner,
) -> HirSelection {
    match sel {
        bgql_syntax::Selection::Field(field) => {
            let alias = field.alias.as_ref().map(|a| interner.get(a.value).to_string());
            let name = interner.get(field.name.value).to_string();
            let arguments: Vec<(String, HirValue)> = field.arguments
                .iter()
                .map(|arg| {
                    let arg_name = interner.get(arg.name.value).to_string();
                    let arg_value = convert_value(&arg.value, interner);
                    (arg_name, arg_value)
                })
                .collect();
            let selections = field.selection_set
                .as_ref()
                .map(|ss| ss.selections.iter().map(|s| convert_selection(s, interner)).collect())
                .unwrap_or_default();

            HirSelection::Field(HirFieldSelection {
                alias,
                name,
                arguments,
                selections,
            })
        }
        bgql_syntax::Selection::FragmentSpread(spread) => {
            HirSelection::FragmentSpread(interner.get(spread.name.value).to_string())
        }
        bgql_syntax::Selection::InlineFragment(inline) => {
            let type_condition = inline.type_condition.as_ref()
                .map(|tc| interner.get(tc.value).to_string());
            let selections = inline.selection_set.selections
                .iter()
                .map(|s| convert_selection(s, interner))
                .collect();

            HirSelection::InlineFragment(bgql_semantic::hir::HirInlineFragment {
                type_condition,
                selections,
            })
        }
    }
}

/// Converts AST value to HIR value.
fn convert_value(value: &bgql_syntax::Value, interner: &Interner) -> HirValue {
    match value {
        bgql_syntax::Value::Variable(name) => {
            HirValue::Variable(interner.get(name.value).to_string())
        }
        bgql_syntax::Value::Int(n, _) => HirValue::Int(*n),
        bgql_syntax::Value::Float(n, _) => HirValue::Float(*n),
        bgql_syntax::Value::String(s, _) => HirValue::String(s.clone()),
        bgql_syntax::Value::Boolean(b, _) => HirValue::Boolean(*b),
        bgql_syntax::Value::Null(_) => HirValue::Null,
        bgql_syntax::Value::Enum(name) => {
            HirValue::Enum(interner.get(name.value).to_string())
        }
        bgql_syntax::Value::List(items, _) => {
            HirValue::List(items.iter().map(|v| convert_value(v, interner)).collect())
        }
        bgql_syntax::Value::Object(fields, _) => {
            HirValue::Object(
                fields.iter()
                    .map(|(name, value)| {
                        (interner.get(name.value).to_string(), convert_value(value, interner))
                    })
                    .collect()
            )
        }
        bgql_syntax::Value::_Phantom(_) => {
            // Phantom variant, should not occur in practice
            HirValue::Null
        }
    }
}

/// DataLoader for batching and caching.
pub struct DataLoader<K, V, F>
where
    K: Eq + std::hash::Hash + Clone + Send,
    V: Clone + Send,
    F: Fn(Vec<K>) -> Pin<Box<dyn Future<Output = HashMap<K, V>> + Send>> + Send + Sync,
{
    inner: bgql_runtime::DataLoader<K, V, F>,
}

impl<K, V, F> DataLoader<K, V, F>
where
    K: Eq + std::hash::Hash + Clone + Send + 'static,
    V: Clone + Send + 'static,
    F: Fn(Vec<K>) -> Pin<Box<dyn Future<Output = HashMap<K, V>> + Send>> + Send + Sync + 'static,
{
    /// Creates a new DataLoader.
    pub fn new(batch_fn: F) -> Self {
        Self {
            inner: bgql_runtime::DataLoader::new(batch_fn),
        }
    }

    /// Loads a value by key.
    pub async fn load(&self, key: K) -> Option<V> {
        self.inner.load(key).await
    }

    /// Loads multiple values by keys.
    pub async fn load_many(&self, keys: Vec<K>) -> HashMap<K, V> {
        self.inner.load_many(keys).await
    }

    /// Clears the cache.
    pub async fn clear(&self) {
        self.inner.clear().await
    }
}

/// Creates a DataLoader with the given batch function.
pub fn create_loader<K, V, F, Fut>(
    batch_fn: F,
) -> DataLoader<
    K,
    V,
    impl Fn(Vec<K>) -> Pin<Box<dyn Future<Output = HashMap<K, V>> + Send>> + Send + Sync,
>
where
    K: Eq + std::hash::Hash + Clone + Send + 'static,
    V: Clone + Send + 'static,
    F: Fn(Vec<K>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = HashMap<K, V>> + Send + 'static,
{
    DataLoader {
        inner: bgql_runtime::dataloader::create_loader(batch_fn),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let config = ServerConfig::new()
            .port(8080)
            .host("0.0.0.0")
            .no_playground();

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.playground);
    }

    #[test]
    fn test_context() {
        let mut ctx = Context::new();
        ctx.set("user_id", "123");
        assert_eq!(ctx.get::<String>("user_id"), Some("123".to_string()));
    }

    #[tokio::test]
    async fn test_server_builder_with_sdl() {
        let server = BgqlServer::builder()
            .schema_sdl(r#"
                type Query {
                    hello: String
                }
            "#)
            .build();

        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_execute_simple_query() {
        let server = BgqlServer::builder()
            .schema_sdl(r#"
                type Query {
                    hello: String
                }
            "#)
            .resolver("Query", "hello", |_args, _ctx| async {
                Ok(serde_json::json!("Hello, World!"))
            })
            .build()
            .unwrap();

        let result = server.execute(
            "query { hello }",
            None,
            Context::new(),
        ).await;

        assert!(result.is_ok(), "Query execution failed: {:?}", result.err());
        let data = result.unwrap();
        assert!(data.get("data").is_some());
        assert_eq!(data["data"]["hello"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_dataloader() {
        let loader = create_loader(|keys: Vec<i32>| async move {
            keys.into_iter().map(|k| (k, k * 2)).collect()
        });

        let results = loader.load_many(vec![1, 2, 3]).await;
        assert_eq!(results.get(&1), Some(&2));
        assert_eq!(results.get(&2), Some(&4));
        assert_eq!(results.get(&3), Some(&6));
    }
}
