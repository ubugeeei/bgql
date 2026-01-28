//! Type system for Better GraphQL.

use crate::hir::DefId;
use rustc_hash::FxHashMap;

/// A type in the type system.
#[derive(Debug, Clone)]
pub enum Type {
    /// Named type reference
    Named(String),
    /// `Option<T>`
    Option(Box<Type>),
    /// `List<T>`
    List(Box<Type>),
    /// Generic type instantiation
    Generic(String, Vec<Type>),
    /// Opaque type
    Opaque(String, Box<Type>),
    /// Tuple type
    Tuple(Vec<Type>),
}

/// A scalar type.
#[derive(Debug, Clone)]
pub struct ScalarType {
    pub name: String,
    pub description: Option<String>,
}

/// An object type.
#[derive(Debug, Clone)]
pub struct ObjectType {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<Field>,
    pub implements: Vec<String>,
}

/// A field.
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub description: Option<String>,
    pub ty: Type,
    pub arguments: Vec<InputValue>,
}

/// An input value (argument or input field).
#[derive(Debug, Clone)]
pub struct InputValue {
    pub name: String,
    pub description: Option<String>,
    pub ty: Type,
    pub default_value: Option<String>,
}

/// The type registry.
#[derive(Debug, Default)]
pub struct TypeRegistry {
    scalars: FxHashMap<String, ScalarType>,
    objects: FxHashMap<String, ObjectType>,
    type_ids: FxHashMap<String, DefId>,
}

impl TypeRegistry {
    /// Creates a new type registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the built-in scalar types.
    pub fn register_builtin_scalars(&mut self) {
        for name in ["Int", "Float", "String", "Boolean", "ID"] {
            self.scalars.insert(
                name.to_string(),
                ScalarType {
                    name: name.to_string(),
                    description: Some(format!("Built-in {name} scalar")),
                },
            );
        }
    }

    /// Registers a scalar type.
    pub fn register_scalar(&mut self, scalar: ScalarType) {
        self.scalars.insert(scalar.name.clone(), scalar);
    }

    /// Registers an object type.
    pub fn register_object(&mut self, object: ObjectType) {
        self.objects.insert(object.name.clone(), object);
    }

    /// Associates a type name with a DefId.
    pub fn register_type_id(&mut self, name: String, id: DefId) {
        self.type_ids.insert(name, id);
    }

    /// Looks up a type by name.
    pub fn lookup(&self, name: &str) -> Option<DefId> {
        self.type_ids.get(name).copied()
    }

    /// Gets a scalar type by name.
    pub fn get_scalar(&self, name: &str) -> Option<&ScalarType> {
        self.scalars.get(name)
    }

    /// Gets an object type by name.
    pub fn get_object(&self, name: &str) -> Option<&ObjectType> {
        self.objects.get(name)
    }

    /// Returns true if a type exists.
    pub fn has_type(&self, name: &str) -> bool {
        self.scalars.contains_key(name) || self.objects.contains_key(name)
    }
}

/// Type reference.
#[derive(Debug, Clone)]
pub struct TypeRef {
    pub inner: Type,
}

impl TypeRef {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            inner: Type::Named(name.into()),
        }
    }

    pub fn option(inner: TypeRef) -> Self {
        Self {
            inner: Type::Option(Box::new(inner.inner)),
        }
    }

    pub fn list(inner: TypeRef) -> Self {
        Self {
            inner: Type::List(Box::new(inner.inner)),
        }
    }
}
