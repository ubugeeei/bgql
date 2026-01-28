//! Query planning for Better GraphQL.

use crate::schema::Schema;
use bgql_semantic::hir::HirOperation;

/// Query planner configuration.
#[derive(Debug, Clone, Default)]
pub struct PlannerConfig {
    /// Maximum query depth.
    pub max_depth: usize,
    /// Maximum query complexity.
    pub max_complexity: usize,
}

/// The query planner.
#[derive(Debug)]
pub struct QueryPlanner {
    #[allow(dead_code)]
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
}

/// A query plan.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// The root node of the plan.
    pub root: PlanNode,
}

/// A node in the query plan.
#[derive(Debug, Clone)]
pub enum PlanNode {
    /// Sequential execution.
    Sequence(Vec<PlanNode>),
    /// Parallel execution.
    Parallel(Vec<PlanNode>),
    /// A leaf field to resolve.
    Leaf { field: String },
    /// A deferred node.
    Defer {
        node: Box<PlanNode>,
        label: Option<String>,
    },
    /// A streamed node.
    Stream {
        node: Box<PlanNode>,
        label: Option<String>,
        initial_count: usize,
    },
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
