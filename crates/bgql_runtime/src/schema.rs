//! Schema definition for Better GraphQL.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A GraphQL schema.
#[derive(Debug, Clone, Default)]
pub struct Schema {
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

    /// Builds the schema.
    pub fn build(self) -> Schema {
        self.schema
    }
}
