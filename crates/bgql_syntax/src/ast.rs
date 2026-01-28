//! Abstract Syntax Tree types for Better GraphQL.

use crate::token::DirectiveLocation;
use bgql_core::{Span, Text};

/// A complete document.
#[derive(Debug, Clone)]
pub struct Document<'a> {
    pub definitions: Vec<Definition<'a>>,
    pub span: Span,
}

/// A top-level definition.
#[derive(Debug, Clone)]
pub enum Definition<'a> {
    Schema(SchemaDefinition<'a>),
    Type(TypeDefinition<'a>),
    Directive(DirectiveDefinitionNode<'a>),
    Operation(OperationDefinition<'a>),
    Fragment(FragmentDefinition<'a>),
}

/// Schema definition.
#[derive(Debug, Clone)]
pub struct SchemaDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub directives: Vec<Directive<'a>>,
    pub operations: Vec<OperationTypeDefinition>,
    pub span: Span,
}

/// Operation type in schema (query/mutation/subscription).
#[derive(Debug, Clone)]
pub struct OperationTypeDefinition {
    pub operation: OperationType,
    pub type_name: Text,
    pub span: Span,
}

/// Type of operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Query,
    Mutation,
    Subscription,
}

/// Type definitions.
#[derive(Debug, Clone)]
pub enum TypeDefinition<'a> {
    Object(ObjectTypeDefinition<'a>),
    Interface(InterfaceTypeDefinition<'a>),
    Union(UnionTypeDefinition<'a>),
    Enum(EnumTypeDefinition<'a>),
    Input(InputObjectTypeDefinition<'a>),
    Scalar(ScalarTypeDefinition<'a>),
    Opaque(OpaqueTypeDefinition<'a>),
    TypeAlias(TypeAliasDefinition<'a>),
    InputUnion(InputUnionTypeDefinition<'a>),
}

/// Object type definition.
#[derive(Debug, Clone)]
pub struct ObjectTypeDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub implements: Vec<Name>,
    pub directives: Vec<Directive<'a>>,
    pub fields: Vec<FieldDefinition<'a>>,
    pub type_params: Vec<TypeParameter<'a>>,
    pub span: Span,
}

/// Interface type definition.
#[derive(Debug, Clone)]
pub struct InterfaceTypeDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub implements: Vec<Name>,
    pub directives: Vec<Directive<'a>>,
    pub fields: Vec<FieldDefinition<'a>>,
    pub type_params: Vec<TypeParameter<'a>>,
    pub span: Span,
}

/// Union type definition.
#[derive(Debug, Clone)]
pub struct UnionTypeDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub directives: Vec<Directive<'a>>,
    pub members: Vec<Name>,
    pub span: Span,
}

/// Enum type definition.
#[derive(Debug, Clone)]
pub struct EnumTypeDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub directives: Vec<Directive<'a>>,
    pub values: Vec<EnumValueDefinition<'a>>,
    pub span: Span,
}

/// Enum value definition.
#[derive(Debug, Clone)]
pub struct EnumValueDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub directives: Vec<Directive<'a>>,
    pub data: Option<EnumVariantData<'a>>,
    pub span: Span,
}

/// Rust-style enum variant data.
#[derive(Debug, Clone)]
pub enum EnumVariantData<'a> {
    /// Tuple variant: Variant(Type1, Type2)
    Tuple(Vec<Type<'a>>, Span),
    /// Struct variant: Variant { field: Type }
    Struct(Vec<InputValueDefinition<'a>>, Span),
}

/// Input object type definition.
#[derive(Debug, Clone)]
pub struct InputObjectTypeDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub directives: Vec<Directive<'a>>,
    pub fields: Vec<InputValueDefinition<'a>>,
    pub span: Span,
}

/// Scalar type definition.
#[derive(Debug, Clone)]
pub struct ScalarTypeDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub directives: Vec<Directive<'a>>,
    pub span: Span,
}

/// Opaque type definition (nominal typing wrapper).
#[derive(Debug, Clone)]
pub struct OpaqueTypeDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub underlying: Type<'a>,
    pub directives: Vec<Directive<'a>>,
    pub span: Span,
}

/// Type alias definition.
#[derive(Debug, Clone)]
pub struct TypeAliasDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub aliased: Type<'a>,
    pub span: Span,
}

/// Input union type definition.
#[derive(Debug, Clone)]
pub struct InputUnionTypeDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub directives: Vec<Directive<'a>>,
    pub members: Vec<Name>,
    pub span: Span,
}

/// Field definition.
#[derive(Debug, Clone)]
pub struct FieldDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub arguments: Vec<InputValueDefinition<'a>>,
    pub ty: Type<'a>,
    pub directives: Vec<Directive<'a>>,
    pub span: Span,
}

/// Input value definition (arguments, input fields).
#[derive(Debug, Clone)]
pub struct InputValueDefinition<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub ty: Type<'a>,
    pub default_value: Option<Value<'a>>,
    pub directives: Vec<Directive<'a>>,
    pub span: Span,
}

/// Type parameter for generics.
#[derive(Debug, Clone)]
pub struct TypeParameter<'a> {
    pub name: Name,
    pub constraint: Option<Type<'a>>,
    pub span: Span,
}

/// Type reference.
#[derive(Debug, Clone)]
pub enum Type<'a> {
    /// Named type: User
    Named(NamedType),
    /// Option type: Option<User>
    Option(Box<Type<'a>>, Span),
    /// List type: List<User>
    List(Box<Type<'a>>, Span),
    /// Generic type: Connection<User>
    Generic(GenericType<'a>),
    /// Tuple type: (Int, String)
    Tuple(TupleType<'a>),
    /// Phantom variant for lifetime
    _Phantom(std::marker::PhantomData<&'a ()>),
}

/// Named type.
#[derive(Debug, Clone)]
pub struct NamedType {
    pub name: Text,
    pub span: Span,
}

/// Generic type instantiation.
#[derive(Debug, Clone)]
pub struct GenericType<'a> {
    pub name: Text,
    pub arguments: Vec<Type<'a>>,
    pub span: Span,
}

/// Tuple type.
#[derive(Debug, Clone)]
pub struct TupleType<'a> {
    pub elements: Vec<TupleElement<'a>>,
    pub span: Span,
}

/// Tuple element.
#[derive(Debug, Clone)]
pub struct TupleElement<'a> {
    pub name: Option<Name>,
    pub ty: Type<'a>,
    pub span: Span,
}

/// Directive definition.
#[derive(Debug, Clone)]
pub struct DirectiveDefinitionNode<'a> {
    pub description: Option<Description<'a>>,
    pub name: Name,
    pub arguments: Vec<InputValueDefinition<'a>>,
    pub repeatable: bool,
    pub locations: Vec<DirectiveLocation>,
    pub span: Span,
}

/// Directive usage.
#[derive(Debug, Clone)]
pub struct Directive<'a> {
    pub name: Name,
    pub arguments: Vec<Argument<'a>>,
    pub span: Span,
}

/// Argument.
#[derive(Debug, Clone)]
pub struct Argument<'a> {
    pub name: Name,
    pub value: Value<'a>,
    pub span: Span,
}

/// Operation definition.
#[derive(Debug, Clone)]
pub struct OperationDefinition<'a> {
    pub operation: OperationType,
    pub name: Option<Name>,
    pub variables: Vec<VariableDefinition<'a>>,
    pub directives: Vec<Directive<'a>>,
    pub selection_set: SelectionSet<'a>,
    pub span: Span,
}

/// Variable definition.
#[derive(Debug, Clone)]
pub struct VariableDefinition<'a> {
    pub name: Name,
    pub ty: Type<'a>,
    pub default_value: Option<Value<'a>>,
    pub directives: Vec<Directive<'a>>,
    pub span: Span,
}

/// Fragment definition.
#[derive(Debug, Clone)]
pub struct FragmentDefinition<'a> {
    pub name: Name,
    pub type_condition: Name,
    pub directives: Vec<Directive<'a>>,
    pub selection_set: SelectionSet<'a>,
    pub span: Span,
}

/// Selection set.
#[derive(Debug, Clone)]
pub struct SelectionSet<'a> {
    pub selections: Vec<Selection<'a>>,
    pub span: Span,
}

/// Selection.
#[derive(Debug, Clone)]
pub enum Selection<'a> {
    Field(FieldSelection<'a>),
    FragmentSpread(FragmentSpread<'a>),
    InlineFragment(InlineFragment<'a>),
}

/// Field selection.
#[derive(Debug, Clone)]
pub struct FieldSelection<'a> {
    pub alias: Option<Name>,
    pub name: Name,
    pub arguments: Vec<Argument<'a>>,
    pub directives: Vec<Directive<'a>>,
    pub selection_set: Option<SelectionSet<'a>>,
    pub span: Span,
}

/// Fragment spread.
#[derive(Debug, Clone)]
pub struct FragmentSpread<'a> {
    pub name: Name,
    pub directives: Vec<Directive<'a>>,
    pub span: Span,
}

/// Inline fragment.
#[derive(Debug, Clone)]
pub struct InlineFragment<'a> {
    pub type_condition: Option<Name>,
    pub directives: Vec<Directive<'a>>,
    pub selection_set: SelectionSet<'a>,
    pub span: Span,
}

/// Value.
#[derive(Debug, Clone)]
pub enum Value<'a> {
    Variable(Name),
    Int(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Boolean(bool, Span),
    Null(Span),
    Enum(Name),
    List(Vec<Value<'a>>, Span),
    Object(Vec<(Name, Value<'a>)>, Span),
    /// Phantom variant for lifetime
    _Phantom(std::marker::PhantomData<&'a ()>),
}

/// Name with span.
#[derive(Debug, Clone, Copy)]
pub struct Name {
    pub value: Text,
    pub span: Span,
}

impl Name {
    pub fn new(value: Text, span: Span) -> Self {
        Self { value, span }
    }
}

/// Description (documentation string).
#[derive(Debug, Clone)]
pub struct Description<'a> {
    pub value: &'a str,
    pub span: Span,
}

impl<'a> Description<'a> {
    pub fn new(value: &'a str, span: Span) -> Self {
        Self { value, span }
    }
}
