//! Code formatting for Better GraphQL.

use crate::ast::*;
use bgql_core::Interner;

/// Formatting options.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Number of spaces for indentation.
    pub indent_size: usize,
    /// Use tabs instead of spaces.
    pub use_tabs: bool,
    /// Maximum line width.
    pub max_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_size: 2,
            use_tabs: false,
            max_width: 80,
        }
    }
}

/// Code formatter.
pub struct Formatter<'a> {
    options: FormatOptions,
    interner: &'a Interner,
    output: String,
    indent: usize,
}

impl<'a> Formatter<'a> {
    /// Creates a new formatter.
    pub fn new(interner: &'a Interner, options: FormatOptions) -> Self {
        Self {
            options,
            interner,
            output: String::new(),
            indent: 0,
        }
    }

    /// Formats a document.
    pub fn format(&mut self, document: &Document<'_>) -> String {
        self.output.clear();

        for (i, def) in document.definitions.iter().enumerate() {
            if i > 0 {
                self.output.push_str("\n\n");
            }
            self.format_definition(def);
        }

        self.output.clone()
    }

    fn format_definition(&mut self, def: &Definition<'_>) {
        match def {
            Definition::Schema(s) => self.format_schema(s),
            Definition::Type(t) => self.format_type_definition(t),
            Definition::Directive(d) => self.format_directive_definition(d),
            Definition::Operation(o) => self.format_operation(o),
            Definition::Fragment(f) => self.format_fragment(f),
            Definition::Module(m) => self.format_module(m),
            Definition::Use(u) => self.format_use(u),
        }
    }

    fn format_visibility(&mut self, visibility: &Visibility) {
        if *visibility == Visibility::Public {
            self.output.push_str("pub ");
        }
    }

    fn format_module(&mut self, module: &ModuleDeclaration<'_>) {
        self.format_visibility(&module.visibility);
        self.output.push_str("mod ");
        self.output.push_str(&self.interner.get(module.name.value));

        if let Some(body) = &module.body {
            self.output.push_str(" {\n");
            self.indent += 1;
            for (i, def) in body.iter().enumerate() {
                if i > 0 {
                    self.output.push_str("\n\n");
                }
                self.push_indent();
                self.format_definition(def);
            }
            self.output.push('\n');
            self.indent -= 1;
            self.push_indent();
            self.output.push('}');
        } else {
            self.output.push(';');
        }
    }

    fn format_use(&mut self, use_stmt: &UseStatement<'_>) {
        self.format_visibility(&use_stmt.visibility);
        self.output.push_str("use::");

        // Format path
        for (i, segment) in use_stmt.path.iter().enumerate() {
            if i > 0 {
                self.output.push_str("::");
            }
            self.output.push_str(&self.interner.get(segment.value));
        }

        // Format items
        match &use_stmt.items {
            UseItems::Glob => {
                self.output.push_str("::*");
            }
            UseItems::Named(items) => {
                self.output.push_str("::{");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&self.interner.get(item.name.value));
                    if let Some(alias) = &item.alias {
                        self.output.push_str(" as ");
                        self.output.push_str(&self.interner.get(alias.value));
                    }
                }
                self.output.push('}');
            }
            UseItems::Single => {
                // Path already contains the item
            }
        }
    }

    fn format_schema(&mut self, schema: &SchemaDefinition<'_>) {
        if let Some(desc) = &schema.description {
            self.format_description(desc);
        }
        self.output.push_str("schema");
        self.format_directives(&schema.directives);
        self.output.push_str(" {\n");
        self.indent += 1;

        for op in &schema.operations {
            self.push_indent();
            let op_name = match op.operation {
                OperationType::Query => "query",
                OperationType::Mutation => "mutation",
                OperationType::Subscription => "subscription",
            };
            self.output.push_str(op_name);
            self.output.push_str(": ");
            self.output.push_str(&self.interner.get(op.type_name));
            self.output.push('\n');
        }

        self.indent -= 1;
        self.output.push('}');
    }

    fn format_type_definition(&mut self, type_def: &TypeDefinition<'_>) {
        match type_def {
            TypeDefinition::Object(obj) => self.format_object_type(obj),
            TypeDefinition::Interface(iface) => self.format_interface_type(iface),
            TypeDefinition::Union(u) => self.format_union_type(u),
            TypeDefinition::Enum(e) => self.format_enum_type(e),
            TypeDefinition::Input(inp) => self.format_input_type(inp),
            TypeDefinition::Scalar(s) => self.format_scalar_type(s),
            TypeDefinition::Opaque(o) => self.format_opaque_type(o),
            TypeDefinition::TypeAlias(a) => self.format_type_alias(a),
            TypeDefinition::InputUnion(iu) => self.format_input_union_type(iu),
            TypeDefinition::InputEnum(ie) => self.format_input_enum_type(ie),
        }
    }

    fn format_object_type(&mut self, obj: &ObjectTypeDefinition<'_>) {
        if let Some(desc) = &obj.description {
            self.format_description(desc);
        }
        self.output.push_str("type ");
        self.output.push_str(&self.interner.get(obj.name.value));
        self.format_type_params(&obj.type_params);
        self.format_implements(&obj.implements);
        self.format_directives(&obj.directives);
        self.output.push_str(" {\n");
        self.indent += 1;
        self.format_fields(&obj.fields);
        self.indent -= 1;
        self.output.push('}');
    }

    fn format_interface_type(&mut self, iface: &InterfaceTypeDefinition<'_>) {
        if let Some(desc) = &iface.description {
            self.format_description(desc);
        }
        self.output.push_str("interface ");
        self.output.push_str(&self.interner.get(iface.name.value));
        self.format_type_params(&iface.type_params);
        self.format_implements(&iface.implements);
        self.format_directives(&iface.directives);
        self.output.push_str(" {\n");
        self.indent += 1;
        self.format_fields(&iface.fields);
        self.indent -= 1;
        self.output.push('}');
    }

    fn format_union_type(&mut self, u: &UnionTypeDefinition<'_>) {
        if let Some(desc) = &u.description {
            self.format_description(desc);
        }
        self.output.push_str("union ");
        self.output.push_str(&self.interner.get(u.name.value));
        self.format_directives(&u.directives);
        self.output.push_str(" = ");
        for (i, member) in u.members.iter().enumerate() {
            if i > 0 {
                self.output.push_str(" | ");
            }
            self.output.push_str(&self.interner.get(member.value));
        }
    }

    fn format_enum_type(&mut self, e: &EnumTypeDefinition<'_>) {
        if let Some(desc) = &e.description {
            self.format_description(desc);
        }
        self.output.push_str("enum ");
        self.output.push_str(&self.interner.get(e.name.value));
        self.format_directives(&e.directives);
        self.output.push_str(" {\n");
        self.indent += 1;
        for value in &e.values {
            self.push_indent();
            if let Some(desc) = &value.description {
                self.format_description(desc);
                self.push_indent();
            }
            self.output.push_str(&self.interner.get(value.name.value));
            if let Some(data) = &value.data {
                self.format_enum_variant_data(data);
            }
            self.format_directives(&value.directives);
            self.output.push('\n');
        }
        self.indent -= 1;
        self.output.push('}');
    }

    fn format_enum_variant_data(&mut self, data: &EnumVariantData<'_>) {
        match data {
            EnumVariantData::Tuple(types, _) => {
                self.output.push('(');
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_type(ty);
                }
                self.output.push(')');
            }
            EnumVariantData::Struct(fields, _) => {
                self.output.push_str(" {\n");
                self.indent += 1;
                for field in fields {
                    self.push_indent();
                    self.output.push_str(&self.interner.get(field.name.value));
                    self.output.push_str(": ");
                    self.format_type(&field.ty);
                    self.output.push('\n');
                }
                self.indent -= 1;
                self.push_indent();
                self.output.push('}');
            }
        }
    }

    fn format_input_type(&mut self, inp: &InputObjectTypeDefinition<'_>) {
        if let Some(desc) = &inp.description {
            self.format_description(desc);
        }
        self.output.push_str("input ");
        self.output.push_str(&self.interner.get(inp.name.value));
        self.format_directives(&inp.directives);
        self.output.push_str(" {\n");
        self.indent += 1;
        for field in &inp.fields {
            self.push_indent();
            self.output.push_str(&self.interner.get(field.name.value));
            self.output.push_str(": ");
            self.format_type(&field.ty);
            self.format_directives(&field.directives);
            self.output.push('\n');
        }
        self.indent -= 1;
        self.output.push('}');
    }

    fn format_scalar_type(&mut self, s: &ScalarTypeDefinition<'_>) {
        if let Some(desc) = &s.description {
            self.format_description(desc);
        }
        self.output.push_str("scalar ");
        self.output.push_str(&self.interner.get(s.name.value));
        self.format_directives(&s.directives);
    }

    fn format_opaque_type(&mut self, o: &OpaqueTypeDefinition<'_>) {
        if let Some(desc) = &o.description {
            self.format_description(desc);
        }
        self.output.push_str("opaque ");
        self.output.push_str(&self.interner.get(o.name.value));
        self.output.push_str(" = ");
        self.format_type(&o.underlying);
        self.format_directives(&o.directives);
    }

    fn format_type_alias(&mut self, a: &TypeAliasDefinition<'_>) {
        if let Some(desc) = &a.description {
            self.format_description(desc);
        }
        self.output.push_str("alias ");
        self.output.push_str(&self.interner.get(a.name.value));
        self.output.push_str(" = ");
        self.format_type(&a.aliased);
    }

    fn format_input_union_type(&mut self, iu: &InputUnionTypeDefinition<'_>) {
        if let Some(desc) = &iu.description {
            self.format_description(desc);
        }
        self.output.push_str("input union ");
        self.output.push_str(&self.interner.get(iu.name.value));
        self.format_directives(&iu.directives);
        self.output.push_str(" = ");
        for (i, member) in iu.members.iter().enumerate() {
            if i > 0 {
                self.output.push_str(" | ");
            }
            self.output.push_str(&self.interner.get(member.value));
        }
    }

    fn format_input_enum_type(&mut self, ie: &InputEnumTypeDefinition<'_>) {
        if let Some(desc) = &ie.description {
            self.format_description(desc);
        }
        self.output.push_str("input enum ");
        self.output.push_str(&self.interner.get(ie.name.value));
        self.format_directives(&ie.directives);
        self.output.push_str(" {\n");
        self.indent += 1;

        for variant in &ie.variants {
            self.push_indent();
            if let Some(desc) = &variant.description {
                self.format_description(desc);
                self.push_indent();
            }
            self.output.push_str(&self.interner.get(variant.name.value));
            self.format_directives(&variant.directives);

            if let Some(fields) = &variant.fields {
                self.output.push_str(" { ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&self.interner.get(field.name.value));
                    self.output.push_str(": ");
                    self.format_type(&field.ty);
                }
                self.output.push_str(" }");
            }
            self.output.push('\n');
        }

        self.indent -= 1;
        self.output.push('}');
    }

    fn format_directive_definition(&mut self, _d: &DirectiveDefinitionNode<'_>) {
        // TODO: Implement
    }

    fn format_operation(&mut self, _o: &OperationDefinition<'_>) {
        // TODO: Implement
    }

    fn format_fragment(&mut self, _f: &FragmentDefinition<'_>) {
        // TODO: Implement
    }

    fn format_fields(&mut self, fields: &[FieldDefinition<'_>]) {
        for field in fields {
            if let Some(desc) = &field.description {
                self.push_indent();
                self.format_description(desc);
            }
            self.push_indent();
            self.output.push_str(&self.interner.get(field.name.value));
            if !field.arguments.is_empty() {
                self.output.push('(');
                for (i, arg) in field.arguments.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&self.interner.get(arg.name.value));
                    self.output.push_str(": ");
                    self.format_type(&arg.ty);
                }
                self.output.push(')');
            }
            self.output.push_str(": ");
            self.format_type(&field.ty);
            self.format_directives(&field.directives);
            self.output.push('\n');
        }
    }

    fn format_type(&mut self, ty: &Type<'_>) {
        match ty {
            Type::Named(named) => {
                self.output.push_str(&self.interner.get(named.name));
            }
            Type::Option(inner, _) => {
                self.output.push_str("Option<");
                self.format_type(inner);
                self.output.push('>');
            }
            Type::List(inner, _) => {
                self.output.push_str("List<");
                self.format_type(inner);
                self.output.push('>');
            }
            Type::Generic(gen) => {
                self.output.push_str(&self.interner.get(gen.name));
                self.output.push('<');
                for (i, arg) in gen.arguments.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_type(arg);
                }
                self.output.push('>');
            }
            Type::Tuple(tuple) => {
                self.output.push('(');
                for (i, elem) in tuple.elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    if let Some(name) = &elem.name {
                        self.output.push_str(&self.interner.get(name.value));
                        self.output.push_str(": ");
                    }
                    self.format_type(&elem.ty);
                }
                self.output.push(')');
            }
            Type::_Phantom(_) => {}
        }
    }

    fn format_type_params(&mut self, params: &[TypeParameter<'_>]) {
        if params.is_empty() {
            return;
        }
        self.output.push('<');
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&self.interner.get(param.name.value));
            if let Some(constraint) = &param.constraint {
                self.output.push_str(" extends ");
                self.format_type(constraint);
            }
        }
        self.output.push('>');
    }

    fn format_implements(&mut self, implements: &[Name]) {
        if implements.is_empty() {
            return;
        }
        self.output.push_str(" implements ");
        for (i, name) in implements.iter().enumerate() {
            if i > 0 {
                self.output.push_str(" & ");
            }
            self.output.push_str(&self.interner.get(name.value));
        }
    }

    fn format_directives(&mut self, directives: &[Directive<'_>]) {
        for directive in directives {
            self.output.push_str(" @");
            self.output
                .push_str(&self.interner.get(directive.name.value));
            if !directive.arguments.is_empty() {
                self.output.push('(');
                for (i, arg) in directive.arguments.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&self.interner.get(arg.name.value));
                    self.output.push_str(": ");
                    self.format_value(&arg.value);
                }
                self.output.push(')');
            }
        }
    }

    fn format_value(&mut self, value: &Value<'_>) {
        match value {
            Value::Variable(name) => {
                self.output.push('$');
                self.output.push_str(&self.interner.get(name.value));
            }
            Value::Int(n, _) => {
                self.output.push_str(&n.to_string());
            }
            Value::Float(n, _) => {
                self.output.push_str(&n.to_string());
            }
            Value::String(s, _) => {
                self.output.push('"');
                self.output.push_str(s);
                self.output.push('"');
            }
            Value::Boolean(b, _) => {
                self.output.push_str(if *b { "true" } else { "false" });
            }
            Value::Null(_) => {
                self.output.push_str("null");
            }
            Value::Enum(name) => {
                self.output.push_str(&self.interner.get(name.value));
            }
            Value::List(items, _) => {
                self.output.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.format_value(item);
                }
                self.output.push(']');
            }
            Value::Object(fields, _) => {
                self.output.push('{');
                for (i, (name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&self.interner.get(name.value));
                    self.output.push_str(": ");
                    self.format_value(value);
                }
                self.output.push('}');
            }
            Value::_Phantom(_) => unreachable!(),
        }
    }

    fn format_description(&mut self, desc: &Description<'_>) {
        if desc.value.contains('\n') {
            self.output.push_str("\"\"\"\n");
            for line in desc.value.lines() {
                self.push_indent();
                self.output.push_str(line);
                self.output.push('\n');
            }
            self.push_indent();
            self.output.push_str("\"\"\"\n");
        } else {
            self.output.push('"');
            self.output.push_str(desc.value);
            self.output.push_str("\"\n");
        }
    }

    fn push_indent(&mut self) {
        if self.options.use_tabs {
            for _ in 0..self.indent {
                self.output.push('\t');
            }
        } else {
            for _ in 0..(self.indent * self.options.indent_size) {
                self.output.push(' ');
            }
        }
    }
}

/// Formats a document with default options.
pub fn format(document: &Document<'_>, interner: &Interner) -> String {
    let mut formatter = Formatter::new(interner, FormatOptions::default());
    formatter.format(document)
}

/// Formats a document with custom options.
pub fn format_with_options(
    document: &Document<'_>,
    interner: &Interner,
    options: FormatOptions,
) -> String {
    let mut formatter = Formatter::new(interner, options);
    formatter.format(document)
}
