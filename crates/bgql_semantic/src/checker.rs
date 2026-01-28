//! Type checker for Better GraphQL.

use crate::hir::HirDatabase;
use crate::types::TypeRegistry;
use bgql_core::diagnostics::codes;
use bgql_core::{DiagnosticBag, Interner, Text};
use bgql_syntax::{
    Definition, Document, EnumTypeDefinition, EnumVariantData, FieldDefinition,
    InputEnumTypeDefinition, InputObjectTypeDefinition, InputUnionTypeDefinition,
    InputValueDefinition, InterfaceTypeDefinition, ObjectTypeDefinition, OpaqueTypeDefinition,
    Type, TypeDefinition, UnionTypeDefinition,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Interface field info for implementation checking.
#[derive(Clone)]
struct InterfaceFieldInfo {
    name: String,
    type_repr: String,
}

/// Generic type parameter info.
#[derive(Clone)]
struct GenericTypeParam {
    /// Parameter name (e.g., "T")
    name: String,
    /// Constraint interface names (e.g., ["Node", "Timestamped"] for `T extends Node & Timestamped`)
    constraints: Vec<String>,
}

/// Generic type definition info.
#[derive(Clone)]
struct GenericTypeInfo {
    /// Type parameters with their constraints
    params: Vec<GenericTypeParam>,
}

/// Type checker for Better GraphQL.
pub struct TypeChecker<'a> {
    #[allow(dead_code)]
    types: &'a TypeRegistry,
    #[allow(dead_code)]
    hir: &'a HirDatabase,
    interner: &'a Interner,
    diagnostics: DiagnosticBag,
    /// Set of all defined type names
    defined_types: FxHashSet<String>,
    /// Set of interface names
    interfaces: FxHashSet<String>,
    /// Set of input type names (for checking input unions)
    input_types: FxHashSet<String>,
    /// Interface fields for implementation checking
    interface_fields: FxHashMap<String, Vec<InterfaceFieldInfo>>,
    /// Generic type definitions with their type parameters
    generic_types: FxHashMap<String, GenericTypeInfo>,
    /// Map of type names to the interfaces they implement
    type_implements: FxHashMap<String, FxHashSet<String>>,
    /// Type parameters currently in scope (for checking generic type bodies)
    type_params_in_scope: FxHashSet<String>,
}

/// Result of type checking.
pub struct CheckResult {
    pub diagnostics: DiagnosticBag,
}

impl CheckResult {
    /// Returns true if type checking succeeded.
    pub fn is_ok(&self) -> bool {
        !self.diagnostics.has_errors()
    }
}

impl<'a> TypeChecker<'a> {
    /// Creates a new type checker.
    pub fn new(types: &'a TypeRegistry, hir: &'a HirDatabase, interner: &'a Interner) -> Self {
        Self {
            types,
            hir,
            interner,
            diagnostics: DiagnosticBag::new(),
            defined_types: FxHashSet::default(),
            interfaces: FxHashSet::default(),
            input_types: FxHashSet::default(),
            interface_fields: FxHashMap::default(),
            generic_types: FxHashMap::default(),
            type_implements: FxHashMap::default(),
            type_params_in_scope: FxHashSet::default(),
        }
    }

    /// Resolves a Text to a String.
    fn resolve(&self, text: Text) -> String {
        self.interner.get(text)
    }

    /// Converts a Type to a string representation for comparison.
    fn type_to_string(&self, ty: &Type<'_>) -> String {
        match ty {
            Type::Named(named) => self.interner.get(named.name),
            Type::Option(inner, _) => format!("Option<{}>", self.type_to_string(inner)),
            Type::List(inner, _) => format!("List<{}>", self.type_to_string(inner)),
            Type::Generic(generic) => {
                let args: Vec<String> = generic
                    .arguments
                    .iter()
                    .map(|arg| self.type_to_string(arg))
                    .collect();
                format!("{}<{}>", self.interner.get(generic.name), args.join(", "))
            }
            Type::Tuple(tuple) => {
                let elements: Vec<String> = tuple
                    .elements
                    .iter()
                    .map(|e| self.type_to_string(&e.ty))
                    .collect();
                format!("({})", elements.join(", "))
            }
            Type::_Phantom(_) => String::new(),
        }
    }

    /// Extracts constraint interface names from a constraint Type.
    /// For `T extends Node & Timestamped`, this extracts ["Node", "Timestamped"]
    fn extract_constraints_from_type(&self, ty: &Type<'_>) -> Vec<String> {
        match ty {
            Type::Named(named) => vec![self.interner.get(named.name)],
            Type::Generic(generic) => {
                // For intersection types represented as generics (e.g., Node & Timestamped)
                // The name would be the first interface
                let mut constraints = vec![self.interner.get(generic.name)];
                for arg in &generic.arguments {
                    constraints.extend(self.extract_constraints_from_type(arg));
                }
                constraints
            }
            _ => vec![],
        }
    }

    /// Collects type parameters from a list of TypeParameter AST nodes.
    fn collect_type_params(
        &self,
        type_params: &[bgql_syntax::TypeParameter<'_>],
    ) -> Vec<GenericTypeParam> {
        type_params
            .iter()
            .map(|param| {
                let name = self.resolve(param.name.value);
                let constraints = param
                    .constraint
                    .as_ref()
                    .map(|c| self.extract_constraints_from_type(c))
                    .unwrap_or_default();
                GenericTypeParam { name, constraints }
            })
            .collect()
    }

    /// Checks a document.
    pub fn check(&mut self, document: &Document<'_>) -> CheckResult {
        // Phase 1: Collect all type definitions
        self.collect_type_definitions(document);

        // Phase 2: Check all type references and semantic rules
        self.check_definitions(document);

        CheckResult {
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    /// Collects all type definitions (first pass).
    fn collect_type_definitions(&mut self, document: &Document<'_>) {
        // Register built-in scalars
        for name in ["Int", "Float", "String", "Boolean", "ID"] {
            self.defined_types.insert(name.to_string());
        }

        // Register built-in generic types
        for name in ["Option", "List"] {
            self.defined_types.insert(name.to_string());
        }

        for definition in &document.definitions {
            match definition {
                Definition::Type(type_def) => {
                    let (name, is_interface, is_input) = match type_def {
                        TypeDefinition::Object(obj) => (self.resolve(obj.name.value), false, false),
                        TypeDefinition::Interface(iface) => {
                            (self.resolve(iface.name.value), true, false)
                        }
                        TypeDefinition::Union(union_def) => {
                            (self.resolve(union_def.name.value), false, false)
                        }
                        TypeDefinition::Enum(enum_def) => {
                            (self.resolve(enum_def.name.value), false, false)
                        }
                        TypeDefinition::Input(input) => {
                            (self.resolve(input.name.value), false, true)
                        }
                        TypeDefinition::Scalar(scalar) => {
                            (self.resolve(scalar.name.value), false, false)
                        }
                        TypeDefinition::Opaque(opaque) => {
                            (self.resolve(opaque.name.value), false, false)
                        }
                        TypeDefinition::TypeAlias(alias) => {
                            (self.resolve(alias.name.value), false, false)
                        }
                        TypeDefinition::InputUnion(input_union) => {
                            (self.resolve(input_union.name.value), false, true)
                        }
                        TypeDefinition::InputEnum(input_enum) => {
                            (self.resolve(input_enum.name.value), false, true)
                        }
                    };

                    // Check for duplicate type definitions
                    if self.defined_types.contains(&name) {
                        let span = match type_def {
                            TypeDefinition::Object(obj) => obj.name.span,
                            TypeDefinition::Interface(iface) => iface.name.span,
                            TypeDefinition::Union(union_def) => union_def.name.span,
                            TypeDefinition::Enum(enum_def) => enum_def.name.span,
                            TypeDefinition::Input(input) => input.name.span,
                            TypeDefinition::Scalar(scalar) => scalar.name.span,
                            TypeDefinition::Opaque(opaque) => opaque.name.span,
                            TypeDefinition::TypeAlias(alias) => alias.name.span,
                            TypeDefinition::InputUnion(input_union) => input_union.name.span,
                            TypeDefinition::InputEnum(input_enum) => input_enum.name.span,
                        };
                        self.diagnostics.error(
                            codes::DUPLICATE_TYPE,
                            format!("Duplicate type definition `{name}`"),
                            span,
                            format!("Type `{name}` is already defined"),
                        );
                    } else {
                        self.defined_types.insert(name.clone());
                        if is_interface {
                            self.interfaces.insert(name.clone());
                            // Collect interface fields for implementation checking
                            if let TypeDefinition::Interface(iface) = type_def {
                                let fields: Vec<InterfaceFieldInfo> = iface
                                    .fields
                                    .iter()
                                    .map(|f| InterfaceFieldInfo {
                                        name: self.resolve(f.name.value),
                                        type_repr: self.type_to_string(&f.ty),
                                    })
                                    .collect();
                                self.interface_fields.insert(name.clone(), fields);

                                // Collect generic type parameters for interfaces
                                if !iface.type_params.is_empty() {
                                    let params = self.collect_type_params(&iface.type_params);
                                    self.generic_types
                                        .insert(name.clone(), GenericTypeInfo { params });
                                }
                            }
                        }
                        if is_input {
                            self.input_types.insert(name.clone());
                        }

                        // Collect generic type parameters for object types
                        if let TypeDefinition::Object(obj) = type_def {
                            if !obj.type_params.is_empty() {
                                let params = self.collect_type_params(&obj.type_params);
                                self.generic_types
                                    .insert(name.clone(), GenericTypeInfo { params });
                            }
                            // Collect which interfaces this type implements
                            if !obj.implements.is_empty() {
                                let implements: FxHashSet<String> = obj
                                    .implements
                                    .iter()
                                    .map(|iface| self.resolve(iface.value))
                                    .collect();
                                self.type_implements.insert(name.clone(), implements);
                            }
                        }
                    }
                }
                Definition::Module(module) => {
                    // Recursively collect from inline modules
                    if let Some(body) = &module.body {
                        let inner_doc = Document {
                            definitions: body.clone(),
                            span: module.span,
                        };
                        self.collect_type_definitions(&inner_doc);
                    }
                }
                _ => {}
            }
        }
    }

    /// Checks all definitions (second pass).
    fn check_definitions(&mut self, document: &Document<'_>) {
        for definition in &document.definitions {
            match definition {
                Definition::Type(type_def) => self.check_type_definition(type_def),
                Definition::Module(module) => {
                    if let Some(body) = &module.body {
                        let inner_doc = Document {
                            definitions: body.clone(),
                            span: module.span,
                        };
                        self.check_definitions(&inner_doc);
                    }
                }
                _ => {}
            }
        }
    }

    /// Checks a single type definition.
    fn check_type_definition(&mut self, type_def: &TypeDefinition<'_>) {
        match type_def {
            TypeDefinition::Object(obj) => self.check_object_type(obj),
            TypeDefinition::Interface(iface) => self.check_interface_type(iface),
            TypeDefinition::Union(union_def) => self.check_union_type(union_def),
            TypeDefinition::Enum(enum_def) => self.check_enum_type(enum_def),
            TypeDefinition::Input(input) => self.check_input_type(input),
            TypeDefinition::Scalar(_) => {} // Scalars have no type references
            TypeDefinition::Opaque(opaque) => self.check_opaque_type(opaque),
            TypeDefinition::TypeAlias(alias) => {
                self.check_type(&alias.aliased);
            }
            TypeDefinition::InputUnion(input_union) => self.check_input_union_type(input_union),
            TypeDefinition::InputEnum(input_enum) => self.check_input_enum_type(input_enum),
        }
    }

    /// Checks an object type definition.
    fn check_object_type(&mut self, obj: &ObjectTypeDefinition<'_>) {
        let type_name = self.resolve(obj.name.value);

        // Register type parameters in scope for checking field types
        let prev_type_params = std::mem::take(&mut self.type_params_in_scope);
        for param in &obj.type_params {
            self.type_params_in_scope
                .insert(self.resolve(param.name.value));
        }

        // Build a map of object fields for interface checking
        let obj_fields: FxHashMap<String, String> = obj
            .fields
            .iter()
            .map(|f| (self.resolve(f.name.value), self.type_to_string(&f.ty)))
            .collect();

        // Check implements clause
        for iface_name in &obj.implements {
            let name = self.resolve(iface_name.value);
            if !self.interfaces.contains(&name) {
                self.diagnostics.error(
                    codes::UNDEFINED_TYPE,
                    format!("Undefined interface `{name}`"),
                    iface_name.span,
                    format!("`{name}` is not a defined interface"),
                );
            } else {
                // Check that all interface fields are implemented
                if let Some(iface_fields) = self.interface_fields.get(&name).cloned() {
                    for iface_field in &iface_fields {
                        match obj_fields.get(&iface_field.name) {
                            None => {
                                self.diagnostics.error(
                                    codes::MISSING_INTERFACE_FIELD,
                                    format!(
                                        "Missing field `{}` from interface `{}`",
                                        iface_field.name, name
                                    ),
                                    obj.name.span,
                                    format!(
                                        "Type `{}` must implement field `{}` from interface `{}`",
                                        type_name, iface_field.name, name
                                    ),
                                );
                            }
                            Some(obj_type) => {
                                // Check type compatibility
                                if obj_type != &iface_field.type_repr {
                                    self.diagnostics.error(
                                        codes::INTERFACE_FIELD_TYPE_MISMATCH,
                                        format!(
                                            "Field `{}` has incompatible type",
                                            iface_field.name
                                        ),
                                        obj.name.span,
                                        format!(
                                            "Expected `{}` but found `{}`",
                                            iface_field.type_repr, obj_type
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check field duplicates
        self.check_field_duplicates(&obj.fields, &type_name);

        // Check field types
        for field in &obj.fields {
            self.check_field_definition(field);
        }

        // Restore previous type parameters scope
        self.type_params_in_scope = prev_type_params;
    }

    /// Checks an interface type definition.
    fn check_interface_type(&mut self, iface: &InterfaceTypeDefinition<'_>) {
        let type_name = self.resolve(iface.name.value);

        // Register type parameters in scope for checking field types
        let prev_type_params = std::mem::take(&mut self.type_params_in_scope);
        for param in &iface.type_params {
            self.type_params_in_scope
                .insert(self.resolve(param.name.value));
        }

        // Check extends clause
        for extends_name in &iface.implements {
            let name = self.resolve(extends_name.value);
            if !self.interfaces.contains(&name) {
                self.diagnostics.error(
                    codes::UNDEFINED_TYPE,
                    format!("Undefined interface `{name}`"),
                    extends_name.span,
                    format!("`{name}` is not a defined interface"),
                );
            }
        }

        // Check field duplicates
        self.check_field_duplicates(&iface.fields, &type_name);

        // Check field types
        for field in &iface.fields {
            self.check_field_definition(field);
        }

        // Restore previous type parameters scope
        self.type_params_in_scope = prev_type_params;
    }

    /// Checks a union type definition.
    fn check_union_type(&mut self, union_def: &UnionTypeDefinition<'_>) {
        if union_def.members.is_empty() {
            self.diagnostics.error(
                codes::INVALID_SYNTAX,
                "Empty union",
                union_def.span,
                "Union must have at least one member",
            );
            return;
        }

        for member in &union_def.members {
            let name = self.resolve(member.value);
            if !self.defined_types.contains(&name) {
                self.diagnostics.error(
                    codes::UNDEFINED_TYPE,
                    format!("Undefined type `{name}`"),
                    member.span,
                    format!("Union member `{name}` is not defined"),
                );
            }
        }
    }

    /// Checks an enum type definition.
    fn check_enum_type(&mut self, enum_def: &EnumTypeDefinition<'_>) {
        if enum_def.values.is_empty() {
            self.diagnostics.error(
                codes::INVALID_SYNTAX,
                "Empty enum",
                enum_def.span,
                "Enum must have at least one value",
            );
            return;
        }

        // Check for duplicate enum values
        let mut seen_values = FxHashSet::default();
        for value in &enum_def.values {
            let name = self.resolve(value.name.value);
            if seen_values.contains(&name) {
                self.diagnostics.error(
                    codes::DUPLICATE_FIELD,
                    format!("Duplicate enum value `{name}`"),
                    value.name.span,
                    format!("Enum value `{name}` is already defined"),
                );
            } else {
                seen_values.insert(name);
            }

            // Check types in variant data
            if let Some(data) = &value.data {
                match data {
                    EnumVariantData::Tuple(types, _) => {
                        for ty in types {
                            self.check_type(ty);
                        }
                    }
                    EnumVariantData::Struct(fields, _) => {
                        for field in fields {
                            self.check_input_value_definition(field);
                        }
                    }
                }
            }
        }
    }

    /// Checks an input object type definition.
    fn check_input_type(&mut self, input: &InputObjectTypeDefinition<'_>) {
        // Check for duplicate fields
        let mut seen_fields = FxHashSet::default();
        for field in &input.fields {
            let name = self.resolve(field.name.value);
            if seen_fields.contains(&name) {
                self.diagnostics.error(
                    codes::DUPLICATE_FIELD,
                    format!("Duplicate input field `{name}`"),
                    field.name.span,
                    format!("Field `{name}` is already defined"),
                );
            } else {
                seen_fields.insert(name);
            }
            self.check_input_value_definition(field);
        }
    }

    /// Checks an opaque type definition.
    fn check_opaque_type(&mut self, opaque: &OpaqueTypeDefinition<'_>) {
        self.check_type(&opaque.underlying);
    }

    /// Checks an input union type definition.
    fn check_input_union_type(&mut self, input_union: &InputUnionTypeDefinition<'_>) {
        if input_union.members.is_empty() {
            self.diagnostics.error(
                codes::INVALID_SYNTAX,
                "Empty input union",
                input_union.span,
                "Input union must have at least one member",
            );
            return;
        }

        for member in &input_union.members {
            let name = self.resolve(member.value);
            if !self.defined_types.contains(&name) {
                self.diagnostics.error(
                    codes::UNDEFINED_TYPE,
                    format!("Undefined type `{name}`"),
                    member.span,
                    format!("Input union member `{name}` is not defined"),
                );
            } else if !self.input_types.contains(&name) {
                self.diagnostics.error(
                    codes::TYPE_MISMATCH,
                    format!("Type `{name}` is not an input type"),
                    member.span,
                    "Input union members must be input types",
                );
            }
        }
    }

    /// Checks an input enum type definition.
    fn check_input_enum_type(&mut self, input_enum: &InputEnumTypeDefinition<'_>) {
        if input_enum.variants.is_empty() {
            self.diagnostics.error(
                codes::INVALID_SYNTAX,
                "Empty input enum",
                input_enum.span,
                "Input enum must have at least one variant",
            );
            return;
        }

        // Check for duplicate variants
        let mut seen_variants = FxHashSet::default();
        for variant in &input_enum.variants {
            let name = self.resolve(variant.name.value);
            if seen_variants.contains(&name) {
                self.diagnostics.error(
                    codes::DUPLICATE_FIELD,
                    format!("Duplicate variant `{name}`"),
                    variant.name.span,
                    format!("Variant `{name}` is already defined"),
                );
            } else {
                seen_variants.insert(name.clone());
            }

            // Check field types in struct variants
            if let Some(fields) = &variant.fields {
                let mut seen_fields = FxHashSet::default();
                for field in fields {
                    let field_name = self.resolve(field.name.value);
                    if seen_fields.contains(&field_name) {
                        self.diagnostics.error(
                            codes::DUPLICATE_FIELD,
                            format!("Duplicate field `{field_name}`"),
                            field.name.span,
                            format!("Field `{field_name}` is already defined in variant `{name}`"),
                        );
                    } else {
                        seen_fields.insert(field_name);
                    }
                    self.check_input_value_definition(field);
                }
            }
        }
    }

    /// Checks field duplicates.
    fn check_field_duplicates(&mut self, fields: &[FieldDefinition<'_>], type_name: &str) {
        let mut seen_fields = FxHashSet::default();
        for field in fields {
            let name = self.resolve(field.name.value);
            if seen_fields.contains(&name) {
                self.diagnostics.error(
                    codes::DUPLICATE_FIELD,
                    format!("Duplicate field `{name}`"),
                    field.name.span,
                    format!("Field `{name}` is already defined on type `{type_name}`"),
                );
            } else {
                seen_fields.insert(name);
            }
        }
    }

    /// Checks a field definition.
    fn check_field_definition(&mut self, field: &FieldDefinition<'_>) {
        let field_name = self.resolve(field.name.value);

        // Check field type
        self.check_type(&field.ty);

        // Check argument duplicates
        let mut seen_args = FxHashSet::default();
        for arg in &field.arguments {
            let name = self.resolve(arg.name.value);
            if seen_args.contains(&name) {
                self.diagnostics.error(
                    codes::DUPLICATE_FIELD,
                    format!("Duplicate argument `{name}`"),
                    arg.name.span,
                    format!("Argument `{name}` is already defined on field `{field_name}`"),
                );
            } else {
                seen_args.insert(name);
            }
            self.check_input_value_definition(arg);
        }
    }

    /// Checks an input value definition (argument or input field).
    fn check_input_value_definition(&mut self, input: &InputValueDefinition<'_>) {
        self.check_type(&input.ty);
    }

    /// Checks a type reference.
    fn check_type(&mut self, ty: &Type<'_>) {
        match ty {
            Type::Named(named) => {
                let name = self.interner.get(named.name);
                // Allow type parameters that are in scope
                if !self.defined_types.contains(&name)
                    && !self.type_params_in_scope.contains(&name)
                {
                    self.diagnostics.error(
                        codes::UNDEFINED_TYPE,
                        format!("Undefined type `{name}`"),
                        named.span,
                        format!("Type `{name}` is not defined"),
                    );
                }
            }
            Type::Option(inner, _) => self.check_type(inner),
            Type::List(inner, _) => self.check_type(inner),
            Type::Generic(generic) => {
                // Check the generic type name
                let name = self.interner.get(generic.name);
                if !self.defined_types.contains(&name) {
                    self.diagnostics.error(
                        codes::UNDEFINED_TYPE,
                        format!("Undefined type `{name}`"),
                        generic.span,
                        format!("Generic type `{name}` is not defined"),
                    );
                }

                // Check type arguments
                for arg in &generic.arguments {
                    self.check_type(arg);
                }

                // Check generic constraints
                self.check_generic_constraints(generic);
            }
            Type::Tuple(tuple) => {
                for element in &tuple.elements {
                    self.check_type(&element.ty);
                }
            }
            Type::_Phantom(_) => {}
        }
    }

    /// Checks generic constraints when a generic type is instantiated.
    fn check_generic_constraints(&mut self, generic: &bgql_syntax::GenericType<'_>) {
        let type_name = self.interner.get(generic.name);

        // Get the generic type info
        let generic_info = match self.generic_types.get(&type_name).cloned() {
            Some(info) => info,
            None => return, // Not a user-defined generic type (e.g., Option, List)
        };

        // Check each type argument against its parameter's constraint
        for (i, (param, arg)) in generic_info
            .params
            .iter()
            .zip(generic.arguments.iter())
            .enumerate()
        {
            if param.constraints.is_empty() {
                continue;
            }

            // Get the type name of the argument
            let arg_type_name = self.get_base_type_name(arg);
            if let Some(arg_type_name) = arg_type_name {
                // If the argument is a type parameter in scope, skip constraint checking
                // (the constraint would be validated at the instantiation site)
                if self.type_params_in_scope.contains(&arg_type_name) {
                    continue;
                }

                // Check if the argument type satisfies all constraints
                for constraint in &param.constraints {
                    if !self.type_satisfies_constraint(&arg_type_name, constraint) {
                        self.diagnostics.error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            format!(
                                "Type `{arg_type_name}` does not satisfy constraint `{constraint}`"
                            ),
                            generic.span,
                            format!(
                                "Type parameter {} at position {} requires type to implement `{}`",
                                param.name,
                                i + 1,
                                constraint
                            ),
                        );
                    }
                }
            }
        }
    }

    /// Gets the base type name from a Type (unwrapping Option/List).
    fn get_base_type_name(&self, ty: &Type<'_>) -> Option<String> {
        match ty {
            Type::Named(named) => Some(self.interner.get(named.name)),
            Type::Option(inner, _) => self.get_base_type_name(inner),
            Type::List(inner, _) => self.get_base_type_name(inner),
            Type::Generic(generic) => Some(self.interner.get(generic.name)),
            Type::Tuple(_) => None,
            Type::_Phantom(_) => None,
        }
    }

    /// Checks if a type satisfies a constraint (implements an interface).
    fn type_satisfies_constraint(&self, type_name: &str, constraint: &str) -> bool {
        // Check if the type is the constraint interface itself
        if type_name == constraint {
            return true;
        }

        // Check if the type implements the constraint interface
        if let Some(implements) = self.type_implements.get(type_name) {
            if implements.contains(constraint) {
                return true;
            }
        }

        // Check if the constraint is an interface (allow if constraint not defined)
        // This prevents false positives when the constraint interface doesn't exist
        // (a separate error will be reported for undefined interfaces)
        if !self.interfaces.contains(constraint) {
            return true;
        }

        false
    }
}

/// Type checks a document.
pub fn check(
    document: &Document<'_>,
    types: &TypeRegistry,
    hir: &HirDatabase,
    interner: &Interner,
) -> CheckResult {
    let mut checker = TypeChecker::new(types, hir, interner);
    checker.check(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::HirDatabase;
    use crate::types::TypeRegistry;
    use bgql_syntax::parser::Parser;

    fn check_source(source: &str) -> CheckResult {
        let interner = Interner::new();
        let mut parser = Parser::new(source, &interner);
        let doc = parser.parse_document();
        let types = TypeRegistry::new();
        let hir = HirDatabase::new();
        check(&doc, &types, &hir, &interner)
    }

    #[test]
    fn test_valid_object_type() {
        let result = check_source(
            r#"
            type User {
                id: ID
                name: String
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_undefined_type() {
        let result = check_source(
            r#"
            type User {
                id: ID
                profile: Profile
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::UNDEFINED_TYPE));
    }

    #[test]
    fn test_duplicate_field() {
        let result = check_source(
            r#"
            type User {
                id: ID
                name: String
                name: Int
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::DUPLICATE_FIELD));
    }

    #[test]
    fn test_duplicate_type() {
        let result = check_source(
            r#"
            type User {
                id: ID
            }
            type User {
                name: String
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::DUPLICATE_TYPE));
    }

    #[test]
    fn test_undefined_interface() {
        let result = check_source(
            r#"
            type User implements Node {
                id: ID
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::UNDEFINED_TYPE));
    }

    #[test]
    fn test_valid_interface_implementation() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            type User implements Node {
                id: ID
                name: String
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_union() {
        let result = check_source(
            r#"
            type User {
                id: ID
            }
            type Post {
                id: ID
            }
            union SearchResult = User | Post
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_undefined_union_member() {
        let result = check_source(
            r#"
            type User {
                id: ID
            }
            union SearchResult = User | Post
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::UNDEFINED_TYPE));
    }

    #[test]
    fn test_valid_input_type() {
        let result = check_source(
            r#"
            input CreateUserInput {
                name: String
                email: String
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_option_and_list_types() {
        let result = check_source(
            r#"
            type User {
                id: ID
                nickname: Option<String>
                friends: List<User>
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_enum_with_data() {
        let result = check_source(
            r#"
            enum Result {
                Ok(String)
                Err { message: String, code: Int }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_input_enum() {
        let result = check_source(
            r#"
            input enum LoginMethod {
                Email { email: String, password: String }
                OAuth { provider: String }
                Anonymous
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_opaque_type() {
        let result = check_source(
            r#"
            opaque Email = String
            type User {
                email: Email
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_interface_field() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            type User implements Node {
                name: String
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::MISSING_INTERFACE_FIELD));
    }

    #[test]
    fn test_interface_field_type_mismatch() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            type User implements Node {
                id: String
                name: String
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::INTERFACE_FIELD_TYPE_MISMATCH));
    }

    #[test]
    fn test_multiple_interface_implementation() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            interface Timestamped {
                createdAt: String
            }
            type User implements Node & Timestamped {
                id: ID
                createdAt: String
                name: String
            }
        "#,
        );
        assert!(result.is_ok());
    }

    // =========================================================================
    // Generic Constraint Tests
    // =========================================================================

    #[test]
    fn test_generic_type_without_constraint() {
        let result = check_source(
            r#"
            type Edge<T> {
                cursor: String
                node: T
            }
            type User {
                id: ID
                name: String
            }
            type Query {
                edge: Edge<User>
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_generic_type_with_constraint_satisfied() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            type Connection<T extends Node> {
                edges: List<T>
                totalCount: Int
            }
            type User implements Node {
                id: ID
                name: String
            }
            type Query {
                users: Connection<User>
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_generic_type_with_constraint_violated() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            type Connection<T extends Node> {
                edges: List<T>
                totalCount: Int
            }
            type Post {
                title: String
            }
            type Query {
                posts: Connection<Post>
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::GENERIC_CONSTRAINT_VIOLATION));
    }

    #[test]
    fn test_generic_interface_with_constraint() {
        // Test that generic interfaces with constraints work correctly
        // when used as field types
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            interface Repository<T extends Node> {
                findById(id: ID): Option<T>
                count: Int
            }
            type User implements Node {
                id: ID
                name: String
            }
            type Query {
                repo: Repository<User>
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_generic_with_constraint() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            type Edge<T> {
                cursor: String
                node: T
            }
            type Connection<T extends Node> {
                edges: List<Edge<T>>
                totalCount: Int
            }
            type User implements Node {
                id: ID
                name: String
            }
            type Query {
                users: Connection<User>
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_type_parameters_with_constraints() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            interface Error {
                message: String
            }
            type Result<T extends Node, E extends Error> {
                data: Option<T>
                error: Option<E>
            }
            type User implements Node {
                id: ID
                name: String
            }
            type NotFoundError implements Error {
                message: String
                resourceId: ID
            }
            type Query {
                user(id: ID): Result<User, NotFoundError>
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_partial_constraint_violation() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            interface Error {
                message: String
            }
            type Result<T extends Node, E extends Error> {
                data: Option<T>
                error: Option<E>
            }
            type User implements Node {
                id: ID
                name: String
            }
            type InvalidError {
                code: Int
            }
            type Query {
                user(id: ID): Result<User, InvalidError>
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::GENERIC_CONSTRAINT_VIOLATION));
    }

    #[test]
    fn test_generic_field_argument_with_constraint() {
        let result = check_source(
            r#"
            interface Node {
                id: ID
            }
            type Connection<T extends Node> {
                edges: List<T>
            }
            type User implements Node {
                id: ID
                name: String
            }
            type Query {
                users(first: Int): Connection<User>
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_constraint_with_marker_interface() {
        let result = check_source(
            r#"
            interface Persistable {}
            type Repository<T extends Persistable> {
                save(entity: T): T
            }
            type User implements Persistable {
                id: ID
                name: String
            }
            type Query {
                userRepo: Repository<User>
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_constraint_with_marker_interface_violated() {
        let result = check_source(
            r#"
            interface Persistable {}
            type Repository<T extends Persistable> {
                save(entity: T): T
            }
            type TempData {
                value: String
            }
            type Query {
                tempRepo: Repository<TempData>
            }
        "#,
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == codes::GENERIC_CONSTRAINT_VIOLATION));
    }
}
