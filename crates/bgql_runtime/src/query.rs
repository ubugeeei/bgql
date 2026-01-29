//! Query planning for Better GraphQL.

<<<<<<< HEAD
use crate::schema::Schema;
use bgql_semantic::hir::HirOperation;

/// Query planner configuration.
#[derive(Debug, Clone, Default)]
=======
use crate::schema::{FieldDef, ObjectDef, Schema, TypeDef, TypeRef};
use bgql_semantic::hir::{HirFieldSelection, HirOperation, HirOperationKind, HirSelection, HirValue};
use std::collections::HashSet;

/// Query planner configuration.
#[derive(Debug, Clone)]
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
pub struct PlannerConfig {
    /// Maximum query depth.
    pub max_depth: usize,
    /// Maximum query complexity.
    pub max_complexity: usize,
<<<<<<< HEAD
=======
    /// Enable parallel execution of sibling fields.
    pub enable_parallel: bool,
    /// Minimum fields to parallelize.
    pub parallel_threshold: usize,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_depth: 20,
            max_complexity: 1000,
            enable_parallel: true,
            parallel_threshold: 2,
        }
    }
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
}

/// The query planner.
#[derive(Debug)]
pub struct QueryPlanner {
<<<<<<< HEAD
    #[allow(dead_code)]
=======
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
    config: PlannerConfig,
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryPlanner {
    /// Creates a new query planner.
    pub fn new() -> Self {
        Self {
            config: PlannerConfig::default(),
        }
    }

    /// Creates a query planner with configuration.
    pub fn with_config(config: PlannerConfig) -> Self {
        Self { config }
    }

    /// Plans a query.
    pub fn plan(
        &self,
<<<<<<< HEAD
        _operation: &HirOperation,
        _schema: &Schema,
    ) -> Result<QueryPlan, PlanError> {
        // TODO: Implement query planning
        Ok(QueryPlan {
            root: PlanNode::Leaf {
                field: "query".to_string(),
            },
        })
    }
=======
        operation: &HirOperation,
        schema: &Schema,
    ) -> Result<QueryPlan, PlanError> {
        let root_type_name = match operation.kind {
            HirOperationKind::Query => schema.query_type.as_deref(),
            HirOperationKind::Mutation => schema.mutation_type.as_deref(),
            HirOperationKind::Subscription => schema.subscription_type.as_deref(),
        };

        let root_type_name = root_type_name.ok_or_else(|| PlanError {
            message: format!(
                "No {} type defined in schema",
                match operation.kind {
                    HirOperationKind::Query => "Query",
                    HirOperationKind::Mutation => "Mutation",
                    HirOperationKind::Subscription => "Subscription",
                }
            ),
        })?;

        let root_type = schema.get_type(root_type_name).ok_or_else(|| PlanError {
            message: format!("Root type '{}' not found in schema", root_type_name),
        })?;

        let object_def = match root_type {
            TypeDef::Object(obj) => obj,
            _ => {
                return Err(PlanError {
                    message: format!("Root type '{}' must be an Object type", root_type_name),
                })
            }
        };

        let mut context = PlanningContext {
            schema,
            config: &self.config,
            depth: 0,
            complexity: 0,
            visited_fragments: HashSet::new(),
        };

        let root_node = self.plan_selections(
            &operation.selections,
            object_def,
            root_type_name,
            &mut context,
        )?;

        Ok(QueryPlan {
            root: root_node,
            operation_name: operation.name.clone(),
            operation_kind: operation.kind,
            complexity: context.complexity,
            max_depth: context.depth,
        })
    }

    /// Plans a selection set.
    fn plan_selections(
        &self,
        selections: &[HirSelection],
        parent_type: &ObjectDef,
        parent_type_name: &str,
        ctx: &mut PlanningContext<'_>,
    ) -> Result<PlanNode, PlanError> {
        if ctx.depth > self.config.max_depth {
            return Err(PlanError {
                message: format!(
                    "Query depth {} exceeds maximum allowed depth {}",
                    ctx.depth, self.config.max_depth
                ),
            });
        }

        let mut field_nodes = Vec::new();

        for selection in selections {
            match selection {
                HirSelection::Field(field_sel) => {
                    let node = self.plan_field(field_sel, parent_type, parent_type_name, ctx)?;
                    field_nodes.push(node);
                }
                HirSelection::FragmentSpread(name) => {
                    // Skip already visited fragments to prevent cycles
                    if ctx.visited_fragments.contains(name) {
                        continue;
                    }
                    ctx.visited_fragments.insert(name.clone());
                    // Fragment spreading would be resolved during execution
                    // For now, we create a placeholder
                    field_nodes.push(PlanNode::FragmentSpread {
                        name: name.clone(),
                    });
                }
                HirSelection::InlineFragment(inline) => {
                    // Handle inline fragments
                    if let Some(type_condition) = &inline.type_condition {
                        if let Some(TypeDef::Object(cond_type)) = ctx.schema.get_type(type_condition)
                        {
                            let inner =
                                self.plan_selections(&inline.selections, cond_type, type_condition, ctx)?;
                            field_nodes.push(PlanNode::TypeCondition {
                                type_name: type_condition.clone(),
                                node: Box::new(inner),
                            });
                        }
                    } else {
                        // Inline fragment without type condition
                        let inner = self.plan_selections(
                            &inline.selections,
                            parent_type,
                            parent_type_name,
                            ctx,
                        )?;
                        field_nodes.push(inner);
                    }
                }
            }
        }

        // Determine if we should parallelize
        if self.config.enable_parallel && field_nodes.len() >= self.config.parallel_threshold {
            Ok(PlanNode::Parallel(field_nodes))
        } else if field_nodes.len() == 1 {
            Ok(field_nodes.pop().unwrap())
        } else {
            Ok(PlanNode::Sequence(field_nodes))
        }
    }

    /// Plans a single field.
    fn plan_field(
        &self,
        field: &HirFieldSelection,
        parent_type: &ObjectDef,
        parent_type_name: &str,
        ctx: &mut PlanningContext<'_>,
    ) -> Result<PlanNode, PlanError> {
        // Handle __typename
        if field.name == "__typename" {
            ctx.complexity += 1;
            return Ok(PlanNode::Leaf {
                field: FieldInfo {
                    name: "__typename".to_string(),
                    alias: field.alias.clone(),
                    parent_type: parent_type_name.to_string(),
                    return_type: "String".to_string(),
                    arguments: Vec::new(),
                    is_introspection: true,
                },
            });
        }

        // Find field definition
        let field_def = parent_type.fields.get(&field.name).ok_or_else(|| PlanError {
            message: format!(
                "Field '{}' not found on type '{}'",
                field.name, parent_type_name
            ),
        })?;

        ctx.complexity += self.calculate_field_complexity(field_def, &field.arguments);

        if ctx.complexity > self.config.max_complexity {
            return Err(PlanError {
                message: format!(
                    "Query complexity {} exceeds maximum allowed complexity {}",
                    ctx.complexity, self.config.max_complexity
                ),
            });
        }

        // Convert arguments
        let arguments: Vec<(String, serde_json::Value)> = field
            .arguments
            .iter()
            .map(|(name, value)| (name.clone(), hir_value_to_json(value)))
            .collect();

        let return_type_name = get_base_type_name(&field_def.ty);
        let response_name = field.alias.as_ref().unwrap_or(&field.name).clone();

        // Check if we need to resolve nested selections
        if !field.selections.is_empty() {
            // Get the return type
            if let Some(return_type) = ctx.schema.get_type(&return_type_name) {
                if let TypeDef::Object(obj) = return_type {
                    ctx.depth += 1;
                    let max_depth = ctx.depth;
                    let nested = self.plan_selections(&field.selections, obj, &return_type_name, ctx)?;
                    if ctx.depth == max_depth {
                        ctx.depth = max_depth;
                    }

                    // Check for @defer directive
                    let is_deferred = has_defer_directive(&field.arguments);
                    let defer_label = get_defer_label(&field.arguments);

                    if is_deferred {
                        return Ok(PlanNode::Defer {
                            node: Box::new(PlanNode::Field {
                                info: FieldInfo {
                                    name: field.name.clone(),
                                    alias: field.alias.clone(),
                                    parent_type: parent_type_name.to_string(),
                                    return_type: return_type_name,
                                    arguments,
                                    is_introspection: false,
                                },
                                response_name,
                                children: Box::new(nested),
                            }),
                            label: defer_label,
                        });
                    }

                    // Check for @stream directive
                    let is_streamed = has_stream_directive(&field.arguments);
                    let stream_label = get_stream_label(&field.arguments);
                    let initial_count = get_stream_initial_count(&field.arguments);

                    if is_streamed {
                        return Ok(PlanNode::Stream {
                            node: Box::new(PlanNode::Field {
                                info: FieldInfo {
                                    name: field.name.clone(),
                                    alias: field.alias.clone(),
                                    parent_type: parent_type_name.to_string(),
                                    return_type: return_type_name,
                                    arguments,
                                    is_introspection: false,
                                },
                                response_name,
                                children: Box::new(nested),
                            }),
                            label: stream_label,
                            initial_count,
                        });
                    }

                    return Ok(PlanNode::Field {
                        info: FieldInfo {
                            name: field.name.clone(),
                            alias: field.alias.clone(),
                            parent_type: parent_type_name.to_string(),
                            return_type: return_type_name,
                            arguments,
                            is_introspection: false,
                        },
                        response_name,
                        children: Box::new(nested),
                    });
                }
            }
        }

        // Leaf field
        Ok(PlanNode::Leaf {
            field: FieldInfo {
                name: field.name.clone(),
                alias: field.alias.clone(),
                parent_type: parent_type_name.to_string(),
                return_type: return_type_name,
                arguments,
                is_introspection: false,
            },
        })
    }

    /// Calculates field complexity.
    fn calculate_field_complexity(
        &self,
        field_def: &FieldDef,
        arguments: &[(String, HirValue)],
    ) -> usize {
        let mut complexity = 1;

        // Check for multiplier arguments (first, last, limit)
        for (name, value) in arguments {
            if matches!(name.as_str(), "first" | "last" | "limit") {
                if let HirValue::Int(n) = value {
                    complexity *= (*n).max(1) as usize;
                }
            }
        }

        // List types have higher base complexity
        if matches!(field_def.ty, TypeRef::List(_)) {
            complexity *= 10;
        }

        complexity
    }
}

/// Context for query planning.
struct PlanningContext<'a> {
    schema: &'a Schema,
    #[allow(dead_code)]
    config: &'a PlannerConfig,
    depth: usize,
    complexity: usize,
    visited_fragments: HashSet<String>,
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
}

/// A query plan.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// The root node of the plan.
    pub root: PlanNode,
<<<<<<< HEAD
=======
    /// Operation name.
    pub operation_name: Option<String>,
    /// Operation kind.
    pub operation_kind: HirOperationKind,
    /// Total complexity score.
    pub complexity: usize,
    /// Maximum depth.
    pub max_depth: usize,
}

impl QueryPlan {
    /// Creates a simple plan with a root node.
    pub fn simple(root: PlanNode) -> Self {
        Self {
            root,
            operation_name: None,
            operation_kind: HirOperationKind::Query,
            complexity: 0,
            max_depth: 0,
        }
    }
}

/// Information about a field to resolve.
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Field name.
    pub name: String,
    /// Field alias (if any).
    pub alias: Option<String>,
    /// Parent type name.
    pub parent_type: String,
    /// Return type name.
    pub return_type: String,
    /// Field arguments.
    pub arguments: Vec<(String, serde_json::Value)>,
    /// Whether this is an introspection field.
    pub is_introspection: bool,
}

impl FieldInfo {
    /// Returns the response key (alias or name).
    pub fn response_key(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
}

/// A node in the query plan.
#[derive(Debug, Clone)]
pub enum PlanNode {
    /// Sequential execution.
    Sequence(Vec<PlanNode>),
<<<<<<< HEAD
    /// Parallel execution.
    Parallel(Vec<PlanNode>),
    /// A leaf field to resolve.
    Leaf { field: String },
=======

    /// Parallel execution.
    Parallel(Vec<PlanNode>),

    /// A field with nested selections.
    Field {
        info: FieldInfo,
        response_name: String,
        children: Box<PlanNode>,
    },

    /// A leaf field to resolve.
    Leaf { field: FieldInfo },

    /// A fragment spread (to be resolved).
    FragmentSpread { name: String },

    /// A type condition (inline fragment or fragment).
    TypeCondition { type_name: String, node: Box<PlanNode> },

>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
    /// A deferred node.
    Defer {
        node: Box<PlanNode>,
        label: Option<String>,
    },
<<<<<<< HEAD
=======

>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
    /// A streamed node.
    Stream {
        node: Box<PlanNode>,
        label: Option<String>,
        initial_count: usize,
    },
<<<<<<< HEAD
=======

    /// Conditional node (for @skip/@include).
    Conditional {
        condition: bool,
        node: Box<PlanNode>,
    },
}

impl PlanNode {
    /// Returns true if this is a leaf node.
    pub fn is_leaf(&self) -> bool {
        matches!(self, PlanNode::Leaf { .. })
    }

    /// Returns the number of fields in this plan node.
    pub fn field_count(&self) -> usize {
        match self {
            PlanNode::Sequence(nodes) | PlanNode::Parallel(nodes) => {
                nodes.iter().map(|n| n.field_count()).sum()
            }
            PlanNode::Field { children, .. } => 1 + children.field_count(),
            PlanNode::Leaf { .. } => 1,
            PlanNode::FragmentSpread { .. } => 0,
            PlanNode::TypeCondition { node, .. } => node.field_count(),
            PlanNode::Defer { node, .. } | PlanNode::Stream { node, .. } => node.field_count(),
            PlanNode::Conditional { node, condition } => {
                if *condition {
                    node.field_count()
                } else {
                    0
                }
            }
        }
    }
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
}

/// A planning error.
#[derive(Debug, Clone)]
pub struct PlanError {
    pub message: String,
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PlanError {}
<<<<<<< HEAD
=======

/// Gets the base type name from a TypeRef.
fn get_base_type_name(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Named(name) => name.clone(),
        TypeRef::Option(inner) | TypeRef::List(inner) => get_base_type_name(inner),
    }
}

/// Converts a HIR value to JSON.
fn hir_value_to_json(value: &HirValue) -> serde_json::Value {
    match value {
        HirValue::Variable(name) => serde_json::json!({"$var": name}),
        HirValue::Int(n) => serde_json::json!(n),
        HirValue::Float(n) => serde_json::json!(n),
        HirValue::String(s) => serde_json::json!(s),
        HirValue::Boolean(b) => serde_json::json!(b),
        HirValue::Null => serde_json::Value::Null,
        HirValue::Enum(name) => serde_json::json!(name),
        HirValue::List(items) => {
            serde_json::Value::Array(items.iter().map(hir_value_to_json).collect())
        }
        HirValue::Object(fields) => {
            let map: serde_json::Map<String, serde_json::Value> = fields
                .iter()
                .map(|(k, v)| (k.clone(), hir_value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

/// Checks if field has @defer directive.
fn has_defer_directive(_arguments: &[(String, HirValue)]) -> bool {
    // In a real implementation, we'd check the directives on the field
    // For now, return false
    false
}

/// Gets the label from @defer directive.
fn get_defer_label(_arguments: &[(String, HirValue)]) -> Option<String> {
    None
}

/// Checks if field has @stream directive.
fn has_stream_directive(_arguments: &[(String, HirValue)]) -> bool {
    false
}

/// Gets the label from @stream directive.
fn get_stream_label(_arguments: &[(String, HirValue)]) -> Option<String> {
    None
}

/// Gets the initial count from @stream directive.
fn get_stream_initial_count(_arguments: &[(String, HirValue)]) -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldDef, ObjectDef, SchemaBuilder, TypeDef, TypeRef};
    use bgql_core::Span;
    use bgql_semantic::hir::{HirFieldSelection, HirOperation, HirOperationKind, HirSelection};
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
        user_fields.insert(
            "email".to_string(),
            FieldDef {
                name: "email".to_string(),
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
        query_fields.insert(
            "users".to_string(),
            FieldDef {
                name: "users".to_string(),
                description: None,
                ty: TypeRef::List(Box::new(TypeRef::Named("User".to_string()))),
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

    fn create_test_operation() -> HirOperation {
        HirOperation {
            kind: HirOperationKind::Query,
            name: Some("GetUser".to_string()),
            variables: Vec::new(),
            selections: vec![HirSelection::Field(HirFieldSelection {
                alias: None,
                name: "user".to_string(),
                arguments: Vec::new(),
                selections: vec![
                    HirSelection::Field(HirFieldSelection {
                        alias: None,
                        name: "id".to_string(),
                        arguments: Vec::new(),
                        selections: Vec::new(),
                    }),
                    HirSelection::Field(HirFieldSelection {
                        alias: None,
                        name: "name".to_string(),
                        arguments: Vec::new(),
                        selections: Vec::new(),
                    }),
                ],
            })],
            span: Span::empty(0),
        }
    }

    #[test]
    fn test_plan_simple_query() {
        let schema = create_test_schema();
        let operation = create_test_operation();
        let planner = QueryPlanner::new();

        let plan = planner.plan(&operation, &schema).unwrap();

        assert_eq!(plan.operation_name, Some("GetUser".to_string()));
        assert!(plan.root.field_count() > 0);
    }

    #[test]
    fn test_plan_parallel_fields() {
        let schema = create_test_schema();
        let planner = QueryPlanner::with_config(PlannerConfig {
            enable_parallel: true,
            parallel_threshold: 2,
            ..Default::default()
        });

        let operation = HirOperation {
            kind: HirOperationKind::Query,
            name: None,
            variables: Vec::new(),
            selections: vec![HirSelection::Field(HirFieldSelection {
                alias: None,
                name: "user".to_string(),
                arguments: Vec::new(),
                selections: vec![
                    HirSelection::Field(HirFieldSelection {
                        alias: None,
                        name: "id".to_string(),
                        arguments: Vec::new(),
                        selections: Vec::new(),
                    }),
                    HirSelection::Field(HirFieldSelection {
                        alias: None,
                        name: "name".to_string(),
                        arguments: Vec::new(),
                        selections: Vec::new(),
                    }),
                    HirSelection::Field(HirFieldSelection {
                        alias: None,
                        name: "email".to_string(),
                        arguments: Vec::new(),
                        selections: Vec::new(),
                    }),
                ],
            })],
            span: Span::empty(0),
        };

        let plan = planner.plan(&operation, &schema).unwrap();

        // Check that we have parallel execution for the nested fields
        if let PlanNode::Field { children, .. } = &plan.root {
            assert!(matches!(children.as_ref(), PlanNode::Parallel(_)));
        } else {
            panic!("Expected Field node at root");
        }
    }

    #[test]
    fn test_plan_depth_limit() {
        let schema = create_test_schema();
        let planner = QueryPlanner::with_config(PlannerConfig {
            max_depth: 0,
            ..Default::default()
        });

        // The test operation has depth 1 (user -> id/name), so max_depth=0 should fail
        let operation = create_test_operation();
        let result = planner.plan(&operation, &schema);

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("depth"));
    }

    #[test]
    fn test_plan_typename() {
        let schema = create_test_schema();
        let planner = QueryPlanner::new();

        let operation = HirOperation {
            kind: HirOperationKind::Query,
            name: None,
            variables: Vec::new(),
            selections: vec![HirSelection::Field(HirFieldSelection {
                alias: None,
                name: "user".to_string(),
                arguments: Vec::new(),
                selections: vec![HirSelection::Field(HirFieldSelection {
                    alias: None,
                    name: "__typename".to_string(),
                    arguments: Vec::new(),
                    selections: Vec::new(),
                })],
            })],
            span: Span::empty(0),
        };

        let plan = planner.plan(&operation, &schema).unwrap();
        assert!(plan.root.field_count() > 0);
    }

    #[test]
    fn test_field_info_response_key() {
        let info = FieldInfo {
            name: "userName".to_string(),
            alias: Some("name".to_string()),
            parent_type: "User".to_string(),
            return_type: "String".to_string(),
            arguments: Vec::new(),
            is_introspection: false,
        };

        assert_eq!(info.response_key(), "name");

        let info_no_alias = FieldInfo {
            name: "userName".to_string(),
            alias: None,
            parent_type: "User".to_string(),
            return_type: "String".to_string(),
            arguments: Vec::new(),
            is_introspection: false,
        };

        assert_eq!(info_no_alias.response_key(), "userName");
    }
}
>>>>>>> 703747c251d776e50c5464e836b0be66b7f8ebc9
