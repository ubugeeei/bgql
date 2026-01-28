//! Schema definition for Better GraphQL.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Schema version using semantic versioning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major version (breaking changes).
    pub major: u32,
    /// Minor version (backwards-compatible additions).
    pub minor: u32,
    /// Patch version (backwards-compatible fixes).
    pub patch: u32,
    /// Optional prerelease identifier (e.g., "alpha", "beta.1").
    pub prerelease: Option<String>,
}

impl SchemaVersion {
    /// Creates a new schema version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
        }
    }

    /// Parses a version string (e.g., "1.2.3" or "2.0.0-beta.1").
    pub fn parse(version: &str) -> Option<Self> {
        let (version_part, prerelease) = if let Some((v, pre)) = version.split_once('-') {
            (v, Some(pre.to_string()))
        } else {
            (version, None)
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() < 2 {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);

        Some(Self {
            major,
            minor,
            patch,
            prerelease,
        })
    }

    /// Sets prerelease identifier.
    pub fn with_prerelease(mut self, prerelease: impl Into<String>) -> Self {
        self.prerelease = Some(prerelease.into());
        self
    }

    /// Checks if this version is compatible with another.
    pub fn is_compatible_with(&self, other: &SchemaVersion) -> bool {
        // Major version must match for compatibility
        self.major == other.major
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.prerelease {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

/// Endpoint configuration for the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Base path for the GraphQL endpoint.
    pub path: String,

    /// WebSocket path for subscriptions.
    pub ws_path: Option<String>,

    /// Binary stream endpoint path.
    pub binary_path: Option<String>,

    /// HLS streaming endpoint path.
    pub hls_path: Option<String>,

    /// Enable introspection.
    pub introspection: bool,

    /// Enable GraphQL Playground/GraphiQL.
    pub playground: bool,

    /// Maximum query depth.
    pub max_depth: Option<u32>,

    /// Maximum query complexity.
    pub max_complexity: Option<u32>,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            path: "/graphql".to_string(),
            ws_path: Some("/graphql/ws".to_string()),
            binary_path: Some("/graphql/binary".to_string()),
            hls_path: Some("/graphql/hls".to_string()),
            introspection: true,
            playground: true,
            max_depth: None,
            max_complexity: None,
        }
    }
}

impl EndpointConfig {
    /// Creates a new endpoint config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Disables introspection (recommended for production).
    pub fn disable_introspection(mut self) -> Self {
        self.introspection = false;
        self
    }

    /// Disables playground (recommended for production).
    pub fn disable_playground(mut self) -> Self {
        self.playground = false;
        self
    }

    /// Sets maximum query depth.
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Sets maximum query complexity.
    pub fn with_max_complexity(mut self, complexity: u32) -> Self {
        self.max_complexity = Some(complexity);
        self
    }
}

/// Schema metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Schema name.
    pub name: Option<String>,

    /// Schema description.
    pub description: Option<String>,

    /// Schema version.
    pub version: SchemaVersion,

    /// Schema authors.
    pub authors: Vec<String>,

    /// Schema license.
    pub license: Option<String>,

    /// Schema repository URL.
    pub repository: Option<String>,

    /// Custom metadata.
    pub custom: IndexMap<String, serde_json::Value>,
}

impl SchemaMetadata {
    /// Creates new schema metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the schema name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the schema version.
    pub fn with_version(mut self, version: SchemaVersion) -> Self {
        self.version = version;
        self
    }

    /// Sets the schema description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Adds an author.
    pub fn add_author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
        self
    }
}

/// A GraphQL schema.
#[derive(Debug, Clone, Default)]
pub struct Schema {
    /// Schema metadata (version, name, etc.).
    pub metadata: SchemaMetadata,

    /// Endpoint configuration.
    pub endpoint: EndpointConfig,

    pub query_type: Option<String>,
    pub mutation_type: Option<String>,
    pub subscription_type: Option<String>,
    pub types: IndexMap<String, TypeDef>,
    pub directives: IndexMap<String, DirectiveDefinition>,
}

impl Schema {
    /// Creates a new empty schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a type by name.
    pub fn get_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
    }

    /// Returns all types.
    pub fn types(&self) -> impl Iterator<Item = (&String, &TypeDef)> {
        self.types.iter()
    }
}

/// A type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeDef {
    Scalar(ScalarDef),
    Object(ObjectDef),
    Interface(InterfaceDef),
    Union(UnionDef),
    Enum(EnumDef),
    InputObject(InputObjectDef),
}

/// Scalar type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarDef {
    pub name: String,
    pub description: Option<String>,
}

/// Object type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectDef {
    pub name: String,
    pub description: Option<String>,
    pub fields: IndexMap<String, FieldDef>,
    pub implements: Vec<String>,
}

/// Interface type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDef {
    pub name: String,
    pub description: Option<String>,
    pub fields: IndexMap<String, FieldDef>,
    pub implements: Vec<String>,
}

/// Union type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnionDef {
    pub name: String,
    pub description: Option<String>,
    pub members: Vec<String>,
}

/// Enum type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub description: Option<String>,
    pub values: Vec<EnumValueDef>,
}

/// Enum value definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValueDef {
    pub name: String,
    pub description: Option<String>,
    pub deprecated: bool,
    pub deprecation_reason: Option<String>,
}

/// Input object type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputObjectDef {
    pub name: String,
    pub description: Option<String>,
    pub fields: IndexMap<String, InputFieldDef>,
}

/// Field definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub description: Option<String>,
    pub ty: TypeRef,
    pub arguments: IndexMap<String, InputFieldDef>,
    pub deprecated: bool,
    pub deprecation_reason: Option<String>,
}

/// Input field definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputFieldDef {
    pub name: String,
    pub description: Option<String>,
    pub ty: TypeRef,
    pub default_value: Option<String>,
}

/// Type reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeRef {
    Named(String),
    Option(Box<TypeRef>),
    List(Box<TypeRef>),
}

impl TypeRef {
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    pub fn option(inner: TypeRef) -> Self {
        Self::Option(Box::new(inner))
    }

    pub fn list(inner: TypeRef) -> Self {
        Self::List(Box::new(inner))
    }
}

/// Directive definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveDefinition {
    pub name: String,
    pub description: Option<String>,
    pub arguments: IndexMap<String, InputFieldDef>,
    pub locations: Vec<DirectiveLocation>,
    pub repeatable: bool,
}

/// Directive location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectiveLocation {
    Query,
    Mutation,
    Subscription,
    Field,
    FragmentDefinition,
    FragmentSpread,
    InlineFragment,
    VariableDefinition,
    Schema,
    Scalar,
    Object,
    FieldDefinition,
    ArgumentDefinition,
    Interface,
    Union,
    Enum,
    EnumValue,
    InputObject,
    InputFieldDefinition,
}

/// Schema builder.
#[derive(Debug, Default)]
pub struct SchemaBuilder {
    schema: Schema,
}

impl SchemaBuilder {
    /// Creates a new schema builder.
    pub fn new() -> Self {
        let mut builder = Self::default();
        // Add built-in scalars
        for name in ["Int", "Float", "String", "Boolean", "ID"] {
            builder.schema.types.insert(
                name.to_string(),
                TypeDef::Scalar(ScalarDef {
                    name: name.to_string(),
                    description: Some(format!("Built-in {name} scalar")),
                }),
            );
        }
        builder
    }

    /// Sets the schema name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.schema.metadata.name = Some(name.into());
        self
    }

    /// Sets the schema version.
    pub fn version(mut self, version: SchemaVersion) -> Self {
        self.schema.metadata.version = version;
        self
    }

    /// Sets the schema version from string.
    pub fn version_str(mut self, version: &str) -> Self {
        if let Some(v) = SchemaVersion::parse(version) {
            self.schema.metadata.version = v;
        }
        self
    }

    /// Sets the schema description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.schema.metadata.description = Some(desc.into());
        self
    }

    /// Sets the endpoint configuration.
    pub fn endpoint(mut self, config: EndpointConfig) -> Self {
        self.schema.endpoint = config;
        self
    }

    /// Sets the endpoint path.
    pub fn endpoint_path(mut self, path: impl Into<String>) -> Self {
        self.schema.endpoint.path = path.into();
        self
    }

    /// Sets the query type.
    pub fn query_type(mut self, name: impl Into<String>) -> Self {
        self.schema.query_type = Some(name.into());
        self
    }

    /// Sets the mutation type.
    pub fn mutation_type(mut self, name: impl Into<String>) -> Self {
        self.schema.mutation_type = Some(name.into());
        self
    }

    /// Sets the subscription type.
    pub fn subscription_type(mut self, name: impl Into<String>) -> Self {
        self.schema.subscription_type = Some(name.into());
        self
    }

    /// Adds a type.
    pub fn add_type(mut self, type_def: TypeDef) -> Self {
        let name = match &type_def {
            TypeDef::Scalar(s) => s.name.clone(),
            TypeDef::Object(o) => o.name.clone(),
            TypeDef::Interface(i) => i.name.clone(),
            TypeDef::Union(u) => u.name.clone(),
            TypeDef::Enum(e) => e.name.clone(),
            TypeDef::InputObject(i) => i.name.clone(),
        };
        self.schema.types.insert(name, type_def);
        self
    }

    /// Adds a directive definition.
    pub fn add_directive(mut self, directive: DirectiveDefinition) -> Self {
        self.schema
            .directives
            .insert(directive.name.clone(), directive);
        self
    }

    /// Builds the schema.
    pub fn build(self) -> Schema {
        self.schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version_parse() {
        let v = SchemaVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.to_string(), "1.2.3");

        let v = SchemaVersion::parse("2.0.0-beta.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.prerelease, Some("beta.1".to_string()));
        assert_eq!(v.to_string(), "2.0.0-beta.1");
    }

    #[test]
    fn test_schema_version_compatibility() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 5, 0);
        let v3 = SchemaVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));
    }

    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new()
            .name("MyAPI")
            .version_str("1.2.3")
            .description("My GraphQL API")
            .endpoint_path("/api/graphql")
            .query_type("Query")
            .build();

        assert_eq!(schema.metadata.name, Some("MyAPI".to_string()));
        assert_eq!(schema.metadata.version.major, 1);
        assert_eq!(schema.endpoint.path, "/api/graphql");
        assert_eq!(schema.query_type, Some("Query".to_string()));
    }
}
