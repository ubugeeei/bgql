//! Recursive descent parser for Better GraphQL.

use crate::ast::*;
use crate::lexer::Lexer;
use crate::token::{DirectiveLocation, Token, TokenKind};
use bgql_core::{diagnostics::codes, DiagnosticBag, Interner, Span, Text};

/// Parser for Better GraphQL.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    #[allow(dead_code)]
    source: &'a str,
    #[allow(dead_code)]
    interner: &'a Interner,
    current: Token,
    diagnostics: DiagnosticBag,
}

/// Result of parsing.
pub struct ParseResult<'a> {
    pub document: Document<'a>,
    pub diagnostics: DiagnosticBag,
}

/// Parses a source string into a document.
pub fn parse<'a>(source: &'a str, interner: &'a Interner) -> ParseResult<'a> {
    let mut parser = Parser::new(source, interner);
    let document = parser.parse_document();
    ParseResult {
        document,
        diagnostics: parser.diagnostics,
    }
}

impl<'a> Parser<'a> {
    /// Creates a new parser.
    pub fn new(source: &'a str, interner: &'a Interner) -> Self {
        let mut lexer = Lexer::new(source, interner);
        let current = lexer.next_token();
        Self {
            lexer,
            source,
            interner,
            current,
            diagnostics: DiagnosticBag::new(),
        }
    }

    /// Returns the current token kind.
    #[inline]
    fn at(&self) -> TokenKind {
        self.current.kind
    }

    /// Returns true if at the given kind.
    #[inline]
    fn at_kind(&self, kind: TokenKind) -> bool {
        self.current.kind == kind
    }

    /// Advances to the next token.
    fn advance(&mut self) {
        self.current = self.lexer.next_token();
    }

    /// Expects a specific token kind.
    fn expect(&mut self, kind: TokenKind) -> bool {
        if self.at_kind(kind) {
            self.advance();
            true
        } else {
            self.error_expected(kind);
            false
        }
    }

    /// Gets the text of the current token.
    fn current_text(&self) -> &'a str {
        self.lexer.span_text(self.current.span)
    }

    /// Interns the current token's text.
    fn intern_current(&self) -> Text {
        self.lexer.intern_span(self.current.span)
    }

    /// Reports an error.
    fn error(&mut self, message: &str) {
        self.diagnostics.error(
            codes::INVALID_SYNTAX,
            message,
            self.current.span,
            message.to_string(),
        );
    }

    /// Reports an expected token error.
    fn error_expected(&mut self, expected: TokenKind) {
        self.diagnostics.error(
            codes::UNEXPECTED_TOKEN,
            "unexpected token",
            self.current.span,
            format!("expected {}, found {}", expected, self.at()),
        );
    }

    /// Parses a document.
    pub fn parse_document(&mut self) -> Document<'a> {
        let start = self.current.span.start;
        let mut definitions = Vec::new();

        while !self.at_kind(TokenKind::Eof) {
            if let Some(def) = self.parse_definition() {
                definitions.push(def);
            } else {
                // Recovery: skip to next definition
                self.advance();
            }
        }

        let end = self.current.span.end;
        Document {
            definitions,
            span: Span::new(start, end),
        }
    }

    /// Parses a definition.
    fn parse_definition(&mut self) -> Option<Definition<'a>> {
        // Skip description for now
        let description = self.try_parse_description();

        // Check for visibility modifier
        let visibility = self.parse_visibility();

        match self.at() {
            TokenKind::Schema => Some(Definition::Schema(
                self.parse_schema_definition(description),
            )),
            TokenKind::Type => Some(Definition::Type(TypeDefinition::Object(
                self.parse_object_type_with_visibility(description, visibility),
            ))),
            TokenKind::Interface => Some(Definition::Type(TypeDefinition::Interface(
                self.parse_interface_type_with_visibility(description, visibility),
            ))),
            TokenKind::Union => Some(Definition::Type(TypeDefinition::Union(
                self.parse_union_type_with_visibility(description, visibility),
            ))),
            TokenKind::Enum => Some(Definition::Type(TypeDefinition::Enum(
                self.parse_enum_type_with_visibility(description, visibility),
            ))),
            TokenKind::Input => {
                // Could be input union or input enum
                if self.peek_next() == TokenKind::Union {
                    Some(Definition::Type(TypeDefinition::InputUnion(
                        self.parse_input_union_type_with_visibility(description, visibility),
                    )))
                } else if self.peek_next() == TokenKind::Enum {
                    Some(Definition::Type(TypeDefinition::InputEnum(
                        self.parse_input_enum_type_with_visibility(description, visibility),
                    )))
                } else {
                    Some(Definition::Type(TypeDefinition::Input(
                        self.parse_input_object_type_with_visibility(description, visibility),
                    )))
                }
            }
            TokenKind::Scalar => Some(Definition::Type(TypeDefinition::Scalar(
                self.parse_scalar_type_with_visibility(description, visibility),
            ))),
            TokenKind::Opaque => Some(Definition::Type(TypeDefinition::Opaque(
                self.parse_opaque_type_with_visibility(description, visibility),
            ))),
            TokenKind::Alias => Some(Definition::Type(TypeDefinition::TypeAlias(
                self.parse_type_alias(description),
            ))),
            TokenKind::Directive => Some(Definition::Directive(
                self.parse_directive_definition(description),
            )),
            TokenKind::Query
            | TokenKind::Mutation
            | TokenKind::Subscription
            | TokenKind::LBrace => Some(Definition::Operation(self.parse_operation())),
            TokenKind::Fragment => Some(Definition::Fragment(self.parse_fragment_definition())),
            TokenKind::Mod => Some(Definition::Module(self.parse_module_declaration(visibility))),
            TokenKind::Use => Some(Definition::Use(self.parse_use_statement(visibility))),
            _ => {
                self.error("expected definition");
                None
            }
        }
    }

    /// Parses visibility modifier.
    fn parse_visibility(&mut self) -> Visibility {
        if self.at_kind(TokenKind::Pub) {
            self.advance();
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    /// Parses a module declaration.
    ///
    /// ```graphql
    /// mod users;
    /// mod auth { ... }
    /// ```
    fn parse_module_declaration(&mut self, visibility: Visibility) -> ModuleDeclaration<'a> {
        let start = self.current.span.start;
        self.advance(); // mod

        let name = self.parse_name();

        let body = if self.at_kind(TokenKind::LBrace) {
            self.advance();
            let mut definitions = Vec::new();
            while !self.at_kind(TokenKind::RBrace) && !self.at_kind(TokenKind::Eof) {
                if let Some(def) = self.parse_definition() {
                    definitions.push(def);
                } else {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace);
            Some(definitions)
        } else {
            // External module: mod name;
            // Semicolon is optional
            if self.at_kind(TokenKind::Semicolon) {
                self.advance();
            }
            None
        };

        let end = self.current.span.start;
        ModuleDeclaration {
            name,
            body,
            visibility,
            span: Span::new(start, end),
        }
    }

    /// Parses a use statement.
    ///
    /// ```graphql
    /// use::users::{User, UserResult}
    /// use::common::*
    /// use::external::User as ExternalUser
    /// ```
    fn parse_use_statement(&mut self, visibility: Visibility) -> UseStatement<'a> {
        let start = self.current.span.start;
        self.advance(); // use

        // Expect ::
        self.expect(TokenKind::ColonColon);

        // Parse module path
        let mut path = Vec::new();
        path.push(self.parse_name());

        while self.at_kind(TokenKind::ColonColon) {
            self.advance();

            // Check for glob import
            if self.at_kind(TokenKind::Star) {
                self.advance();
                let end = self.current.span.start;
                return UseStatement {
                    path,
                    items: UseItems::Glob,
                    visibility,
                    span: Span::new(start, end),
                    _phantom: std::marker::PhantomData,
                };
            }

            // Check for named imports { A, B }
            if self.at_kind(TokenKind::LBrace) {
                let items = self.parse_use_items();
                let end = self.current.span.start;
                return UseStatement {
                    path,
                    items: UseItems::Named(items),
                    visibility,
                    span: Span::new(start, end),
                    _phantom: std::marker::PhantomData,
                };
            }

            // Continue path
            path.push(self.parse_name());
        }

        // Check for alias: use::module::Item as Alias
        if self.at_kind(TokenKind::As) {
            self.advance();
            let alias = self.parse_name();
            let last_name = path.pop().unwrap();
            let item = UseItem {
                name: last_name,
                alias: Some(alias),
                span: Span::new(last_name.span.start, alias.span.end),
            };
            let end = self.current.span.start;
            return UseStatement {
                path,
                items: UseItems::Named(vec![item]),
                visibility,
                span: Span::new(start, end),
                _phantom: std::marker::PhantomData,
            };
        }

        // Single import: use::module::Item
        let end = self.current.span.start;
        UseStatement {
            path,
            items: UseItems::Single,
            visibility,
            span: Span::new(start, end),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Parses use items: { A, B as C, D }
    fn parse_use_items(&mut self) -> Vec<UseItem> {
        let mut items = Vec::new();
        self.advance(); // {

        while !self.at_kind(TokenKind::RBrace) && !self.at_kind(TokenKind::Eof) {
            let item_start = self.current.span.start;
            let name = self.parse_name();

            let alias = if self.at_kind(TokenKind::As) {
                self.advance();
                Some(self.parse_name())
            } else {
                None
            };

            let item_end = self.current.span.start;
            items.push(UseItem {
                name,
                alias,
                span: Span::new(item_start, item_end),
            });

            if self.at_kind(TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RBrace);
        items
    }

    /// Peeks at the next token kind.
    fn peek_next(&mut self) -> TokenKind {
        let _saved_pos = self.lexer.pos();
        let saved_current = self.current;
        self.advance();
        let next = self.at();
        // Restore - hacky but works
        self.current = saved_current;
        next
    }

    /// Tries to parse a description.
    fn try_parse_description(&mut self) -> Option<Description<'a>> {
        if matches!(
            self.at(),
            TokenKind::StringLiteral | TokenKind::BlockStringLiteral
        ) {
            let span = self.current.span;
            let text = self.current_text();
            // Strip quotes
            let value = if text.starts_with("\"\"\"") {
                &text[3..text.len() - 3]
            } else {
                &text[1..text.len() - 1]
            };
            self.advance();
            Some(Description::new(value, span))
        } else {
            None
        }
    }

    /// Parses a name.
    fn parse_name(&mut self) -> Name {
        let span = self.current.span;
        let value = self.intern_current();
        if self.at_kind(TokenKind::Ident) || self.at().is_keyword() {
            self.advance();
        } else {
            self.error("expected name");
        }
        Name::new(value, span)
    }

    /// Parses schema definition.
    fn parse_schema_definition(
        &mut self,
        description: Option<Description<'a>>,
    ) -> SchemaDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // schema

        let directives = self.parse_directives();
        self.expect(TokenKind::LBrace);

        let mut operations = Vec::new();
        while !self.at_kind(TokenKind::RBrace) && !self.at_kind(TokenKind::Eof) {
            let op_start = self.current.span.start;
            let operation = match self.at() {
                TokenKind::Query => {
                    self.advance();
                    OperationType::Query
                }
                TokenKind::Mutation => {
                    self.advance();
                    OperationType::Mutation
                }
                TokenKind::Subscription => {
                    self.advance();
                    OperationType::Subscription
                }
                _ => {
                    self.error("expected operation type");
                    self.advance();
                    continue;
                }
            };
            self.expect(TokenKind::Colon);
            let type_name = self.intern_current();
            self.advance();
            let op_end = self.current.span.start;
            operations.push(OperationTypeDefinition {
                operation,
                type_name,
                span: Span::new(op_start, op_end),
            });
        }
        self.expect(TokenKind::RBrace);

        let end = self.current.span.start;
        SchemaDefinition {
            description,
            directives,
            operations,
            span: Span::new(start, end),
        }
    }

    /// Parses object type definition (legacy, no visibility).
    fn parse_object_type(
        &mut self,
        description: Option<Description<'a>>,
    ) -> ObjectTypeDefinition<'a> {
        self.parse_object_type_with_visibility(description, Visibility::Private)
    }

    /// Parses object type definition with visibility.
    fn parse_object_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> ObjectTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // type

        let name = self.parse_name();
        let type_params = self.parse_type_parameters();
        let implements = self.parse_implements();
        let directives = self.parse_directives();

        self.expect(TokenKind::LBrace);
        let fields = self.parse_field_definitions();
        self.expect(TokenKind::RBrace);

        let end = self.current.span.start;
        ObjectTypeDefinition {
            description,
            visibility,
            name,
            implements,
            directives,
            fields,
            type_params,
            span: Span::new(start, end),
        }
    }

    /// Parses interface type definition (legacy, no visibility).
    fn parse_interface_type(
        &mut self,
        description: Option<Description<'a>>,
    ) -> InterfaceTypeDefinition<'a> {
        self.parse_interface_type_with_visibility(description, Visibility::Private)
    }

    /// Parses interface type definition with visibility.
    fn parse_interface_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> InterfaceTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // interface

        let name = self.parse_name();
        let type_params = self.parse_type_parameters();
        let implements = self.parse_implements();
        let directives = self.parse_directives();

        self.expect(TokenKind::LBrace);
        let fields = self.parse_field_definitions();
        self.expect(TokenKind::RBrace);

        let end = self.current.span.start;
        InterfaceTypeDefinition {
            description,
            visibility,
            name,
            implements,
            directives,
            fields,
            type_params,
            span: Span::new(start, end),
        }
    }

    /// Parses union type definition (legacy, no visibility).
    fn parse_union_type(
        &mut self,
        description: Option<Description<'a>>,
    ) -> UnionTypeDefinition<'a> {
        self.parse_union_type_with_visibility(description, Visibility::Private)
    }

    /// Parses union type definition with visibility.
    fn parse_union_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> UnionTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // union

        let name = self.parse_name();
        let directives = self.parse_directives();

        self.expect(TokenKind::Eq);

        let mut members = Vec::new();
        if self.at_kind(TokenKind::Pipe) {
            self.advance();
        }
        members.push(self.parse_name());
        while self.at_kind(TokenKind::Pipe) {
            self.advance();
            members.push(self.parse_name());
        }

        let end = self.current.span.start;
        UnionTypeDefinition {
            description,
            visibility,
            name,
            directives,
            members,
            span: Span::new(start, end),
        }
    }

    /// Parses enum type definition (legacy, no visibility).
    fn parse_enum_type(&mut self, description: Option<Description<'a>>) -> EnumTypeDefinition<'a> {
        self.parse_enum_type_with_visibility(description, Visibility::Private)
    }

    /// Parses enum type definition with visibility.
    fn parse_enum_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> EnumTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // enum

        let name = self.parse_name();
        let directives = self.parse_directives();

        self.expect(TokenKind::LBrace);
        let values = self.parse_enum_values();
        self.expect(TokenKind::RBrace);

        let end = self.current.span.start;
        EnumTypeDefinition {
            description,
            visibility,
            name,
            directives,
            values,
            span: Span::new(start, end),
        }
    }

    /// Parses enum values.
    fn parse_enum_values(&mut self) -> Vec<EnumValueDefinition<'a>> {
        let mut values = Vec::new();
        while !self.at_kind(TokenKind::RBrace) && !self.at_kind(TokenKind::Eof) {
            let description = self.try_parse_description();
            let value_start = self.current.span.start;
            let name = self.parse_name();

            // Check for Rust-style enum data
            let data = if self.at_kind(TokenKind::LParen) {
                // Tuple variant
                let tuple_start = self.current.span.start;
                self.advance();
                let mut types = Vec::new();
                if !self.at_kind(TokenKind::RParen) {
                    types.push(self.parse_type());
                    while self.at_kind(TokenKind::Comma) {
                        self.advance();
                        if !self.at_kind(TokenKind::RParen) {
                            types.push(self.parse_type());
                        }
                    }
                }
                self.expect(TokenKind::RParen);
                let tuple_end = self.current.span.start;
                Some(EnumVariantData::Tuple(
                    types,
                    Span::new(tuple_start, tuple_end),
                ))
            } else if self.at_kind(TokenKind::LBrace) {
                // Struct variant
                let struct_start = self.current.span.start;
                self.advance();
                let fields = self.parse_input_value_definitions();
                self.expect(TokenKind::RBrace);
                let struct_end = self.current.span.start;
                Some(EnumVariantData::Struct(
                    fields,
                    Span::new(struct_start, struct_end),
                ))
            } else {
                None
            };

            let directives = self.parse_directives();
            let value_end = self.current.span.start;

            values.push(EnumValueDefinition {
                description,
                name,
                directives,
                data,
                span: Span::new(value_start, value_end),
            });
        }
        values
    }

    /// Parses input object type definition (legacy, no visibility).
    fn parse_input_object_type(
        &mut self,
        description: Option<Description<'a>>,
    ) -> InputObjectTypeDefinition<'a> {
        self.parse_input_object_type_with_visibility(description, Visibility::Private)
    }

    /// Parses input object type definition with visibility.
    fn parse_input_object_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> InputObjectTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // input

        let name = self.parse_name();
        let directives = self.parse_directives();

        self.expect(TokenKind::LBrace);
        let fields = self.parse_input_value_definitions();
        self.expect(TokenKind::RBrace);

        let end = self.current.span.start;
        InputObjectTypeDefinition {
            description,
            visibility,
            name,
            directives,
            fields,
            span: Span::new(start, end),
        }
    }

    /// Parses scalar type definition (legacy, no visibility).
    fn parse_scalar_type(
        &mut self,
        description: Option<Description<'a>>,
    ) -> ScalarTypeDefinition<'a> {
        self.parse_scalar_type_with_visibility(description, Visibility::Private)
    }

    /// Parses scalar type definition with visibility.
    fn parse_scalar_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> ScalarTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // scalar

        let name = self.parse_name();
        let directives = self.parse_directives();

        let end = self.current.span.start;
        ScalarTypeDefinition {
            description,
            visibility,
            name,
            directives,
            span: Span::new(start, end),
        }
    }

    /// Parses opaque type definition (legacy, no visibility).
    fn parse_opaque_type(
        &mut self,
        description: Option<Description<'a>>,
    ) -> OpaqueTypeDefinition<'a> {
        self.parse_opaque_type_with_visibility(description, Visibility::Private)
    }

    /// Parses opaque type definition with visibility.
    fn parse_opaque_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> OpaqueTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // opaque

        let name = self.parse_name();
        self.expect(TokenKind::Eq);
        let underlying = self.parse_type();
        let directives = self.parse_directives();

        let end = self.current.span.start;
        OpaqueTypeDefinition {
            description,
            visibility,
            name,
            underlying,
            directives,
            span: Span::new(start, end),
        }
    }

    /// Parses type alias definition.
    fn parse_type_alias(
        &mut self,
        description: Option<Description<'a>>,
    ) -> TypeAliasDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // alias

        let name = self.parse_name();
        self.expect(TokenKind::Eq);
        let aliased = self.parse_type();

        let end = self.current.span.start;
        TypeAliasDefinition {
            description,
            name,
            aliased,
            span: Span::new(start, end),
        }
    }

    /// Parses input union type definition (legacy, no visibility).
    fn parse_input_union_type(
        &mut self,
        description: Option<Description<'a>>,
    ) -> InputUnionTypeDefinition<'a> {
        self.parse_input_union_type_with_visibility(description, Visibility::Private)
    }

    /// Parses input union type definition with visibility.
    fn parse_input_union_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> InputUnionTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // input
        self.advance(); // union

        let name = self.parse_name();
        let directives = self.parse_directives();

        self.expect(TokenKind::Eq);

        let mut members = Vec::new();
        if self.at_kind(TokenKind::Pipe) {
            self.advance();
        }
        members.push(self.parse_name());
        while self.at_kind(TokenKind::Pipe) {
            self.advance();
            members.push(self.parse_name());
        }

        let end = self.current.span.start;
        InputUnionTypeDefinition {
            description,
            visibility,
            name,
            directives,
            members,
            span: Span::new(start, end),
        }
    }

    /// Parses input enum type definition (legacy, no visibility).
    fn parse_input_enum_type(
        &mut self,
        description: Option<Description<'a>>,
    ) -> InputEnumTypeDefinition<'a> {
        self.parse_input_enum_type_with_visibility(description, Visibility::Private)
    }

    /// Parses input enum type definition with visibility.
    fn parse_input_enum_type_with_visibility(
        &mut self,
        description: Option<Description<'a>>,
        visibility: Visibility,
    ) -> InputEnumTypeDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // input
        self.advance(); // enum

        let name = self.parse_name();
        let directives = self.parse_directives();

        self.expect(TokenKind::LBrace);
        let variants = self.parse_input_enum_variants();
        self.expect(TokenKind::RBrace);

        let end = self.current.span.start;
        InputEnumTypeDefinition {
            description,
            visibility,
            name,
            directives,
            variants,
            span: Span::new(start, end),
        }
    }

    /// Parses input enum variants.
    fn parse_input_enum_variants(&mut self) -> Vec<InputEnumVariant<'a>> {
        let mut variants = Vec::new();
        while !self.at_kind(TokenKind::RBrace) && !self.at_kind(TokenKind::Eof) {
            let description = self.try_parse_description();
            let variant_start = self.current.span.start;
            let name = self.parse_name();

            // Check for variant data (struct fields)
            let fields = if self.at_kind(TokenKind::LBrace) {
                self.advance();
                let fields = self.parse_input_value_definitions();
                self.expect(TokenKind::RBrace);
                Some(fields)
            } else if self.at_kind(TokenKind::LParen) {
                // Tuple-style variant referencing existing input type
                self.advance();
                let type_name = self.parse_name();
                self.expect(TokenKind::RParen);
                // Create a single unnamed field for the tuple variant
                Some(vec![InputValueDefinition {
                    description: None,
                    name: Name::new(self.lexer.intern_span(type_name.span), type_name.span),
                    ty: Type::Named(NamedType {
                        name: type_name.value,
                        span: type_name.span,
                    }),
                    default_value: None,
                    directives: vec![],
                    span: type_name.span,
                }])
            } else {
                None
            };

            let directives = self.parse_directives();
            let variant_end = self.current.span.start;

            variants.push(InputEnumVariant {
                description,
                name,
                directives,
                fields,
                span: Span::new(variant_start, variant_end),
            });
        }
        variants
    }

    /// Parses directive definition.
    fn parse_directive_definition(
        &mut self,
        description: Option<Description<'a>>,
    ) -> DirectiveDefinitionNode<'a> {
        let start = self.current.span.start;
        self.advance(); // directive
        self.expect(TokenKind::At);

        let name = self.parse_name();
        let arguments = if self.at_kind(TokenKind::LParen) {
            self.advance();
            let args = self.parse_input_value_definitions();
            self.expect(TokenKind::RParen);
            args
        } else {
            Vec::new()
        };

        let repeatable = if self.at_kind(TokenKind::Repeatable) {
            self.advance();
            true
        } else {
            false
        };

        // Parse "on"
        if self.at_kind(TokenKind::On) {
            self.advance();
        }

        let mut locations = Vec::new();
        if self.at_kind(TokenKind::Pipe) {
            self.advance();
        }
        if let Some(loc) = DirectiveLocation::parse(self.current_text()) {
            locations.push(loc);
            self.advance();
        }
        while self.at_kind(TokenKind::Pipe) {
            self.advance();
            if let Some(loc) = DirectiveLocation::parse(self.current_text()) {
                locations.push(loc);
                self.advance();
            }
        }

        let end = self.current.span.start;
        DirectiveDefinitionNode {
            description,
            name,
            arguments,
            repeatable,
            locations,
            span: Span::new(start, end),
        }
    }

    /// Parses implements clause.
    fn parse_implements(&mut self) -> Vec<Name> {
        let mut implements = Vec::new();
        if self.at_kind(TokenKind::Implements) {
            self.advance();
            if self.at_kind(TokenKind::Amp) {
                self.advance();
            }
            implements.push(self.parse_name());
            while self.at_kind(TokenKind::Amp) {
                self.advance();
                implements.push(self.parse_name());
            }
        }
        implements
    }

    /// Parses type parameters.
    fn parse_type_parameters(&mut self) -> Vec<TypeParameter<'a>> {
        let mut params = Vec::new();
        if self.at_kind(TokenKind::LAngle) {
            self.advance();
            if !self.at_kind(TokenKind::RAngle) {
                params.push(self.parse_type_parameter());
                while self.at_kind(TokenKind::Comma) {
                    self.advance();
                    params.push(self.parse_type_parameter());
                }
            }
            self.expect(TokenKind::RAngle);
        }
        params
    }

    /// Parses a type parameter.
    fn parse_type_parameter(&mut self) -> TypeParameter<'a> {
        let start = self.current.span.start;
        let name = self.parse_name();
        let constraint = if self.at_kind(TokenKind::Extends) {
            self.advance();
            Some(self.parse_type())
        } else {
            None
        };
        let end = self.current.span.start;
        TypeParameter {
            name,
            constraint,
            span: Span::new(start, end),
        }
    }

    /// Parses field definitions.
    fn parse_field_definitions(&mut self) -> Vec<FieldDefinition<'a>> {
        let mut fields = Vec::new();
        while !self.at_kind(TokenKind::RBrace) && !self.at_kind(TokenKind::Eof) {
            let description = self.try_parse_description();
            fields.push(self.parse_field_definition(description));
        }
        fields
    }

    /// Parses a field definition.
    fn parse_field_definition(
        &mut self,
        description: Option<Description<'a>>,
    ) -> FieldDefinition<'a> {
        let start = self.current.span.start;
        let name = self.parse_name();

        let arguments = if self.at_kind(TokenKind::LParen) {
            self.advance();
            let args = self.parse_input_value_definitions();
            self.expect(TokenKind::RParen);
            args
        } else {
            Vec::new()
        };

        self.expect(TokenKind::Colon);
        let ty = self.parse_type();
        let directives = self.parse_directives();

        let end = self.current.span.start;
        FieldDefinition {
            description,
            name,
            arguments,
            ty,
            directives,
            span: Span::new(start, end),
        }
    }

    /// Parses input value definitions.
    fn parse_input_value_definitions(&mut self) -> Vec<InputValueDefinition<'a>> {
        let mut fields = Vec::new();
        while !self.at_kind(TokenKind::RParen)
            && !self.at_kind(TokenKind::RBrace)
            && !self.at_kind(TokenKind::Eof)
        {
            // Skip optional commas between fields
            while self.at_kind(TokenKind::Comma) {
                self.advance();
            }
            if self.at_kind(TokenKind::RParen) || self.at_kind(TokenKind::RBrace) {
                break;
            }
            let description = self.try_parse_description();
            fields.push(self.parse_input_value_definition(description));
        }
        fields
    }

    /// Parses an input value definition.
    fn parse_input_value_definition(
        &mut self,
        description: Option<Description<'a>>,
    ) -> InputValueDefinition<'a> {
        let start = self.current.span.start;
        let name = self.parse_name();
        self.expect(TokenKind::Colon);
        let ty = self.parse_type();

        let default_value = if self.at_kind(TokenKind::Eq) {
            self.advance();
            Some(self.parse_value())
        } else {
            None
        };

        let directives = self.parse_directives();

        let end = self.current.span.start;
        InputValueDefinition {
            description,
            name,
            ty,
            default_value,
            directives,
            span: Span::new(start, end),
        }
    }

    /// Parses a type.
    fn parse_type(&mut self) -> Type<'a> {
        let start = self.current.span.start;

        // Check for Option/List wrappers
        if self.at_kind(TokenKind::Option) {
            self.advance();
            self.expect(TokenKind::LAngle);
            let inner = self.parse_type();
            self.expect(TokenKind::RAngle);
            let end = self.current.span.start;
            return Type::Option(Box::new(inner), Span::new(start, end));
        }

        if self.at_kind(TokenKind::List) {
            self.advance();
            self.expect(TokenKind::LAngle);
            let inner = self.parse_type();
            self.expect(TokenKind::RAngle);
            let end = self.current.span.start;
            return Type::List(Box::new(inner), Span::new(start, end));
        }

        // Check for tuple type
        if self.at_kind(TokenKind::LParen) {
            return self.parse_tuple_type();
        }

        // Named or generic type
        let name = self.intern_current();
        self.advance();

        if self.at_kind(TokenKind::LAngle) {
            // Generic type
            self.advance();
            let mut arguments = Vec::new();
            if !self.at_kind(TokenKind::RAngle) {
                arguments.push(self.parse_type());
                while self.at_kind(TokenKind::Comma) {
                    self.advance();
                    arguments.push(self.parse_type());
                }
            }
            self.expect(TokenKind::RAngle);
            let end = self.current.span.start;
            Type::Generic(GenericType {
                name,
                arguments,
                span: Span::new(start, end),
            })
        } else {
            let end = self.current.span.start;
            Type::Named(NamedType {
                name,
                span: Span::new(start, end),
            })
        }
    }

    /// Parses a tuple type.
    fn parse_tuple_type(&mut self) -> Type<'a> {
        let start = self.current.span.start;
        self.advance(); // (

        let mut elements = Vec::new();
        if !self.at_kind(TokenKind::RParen) {
            elements.push(self.parse_tuple_element());
            while self.at_kind(TokenKind::Comma) {
                self.advance();
                if !self.at_kind(TokenKind::RParen) {
                    elements.push(self.parse_tuple_element());
                }
            }
        }
        self.expect(TokenKind::RParen);

        let end = self.current.span.start;
        Type::Tuple(TupleType {
            elements,
            span: Span::new(start, end),
        })
    }

    /// Parses a tuple element.
    fn parse_tuple_element(&mut self) -> TupleElement<'a> {
        let start = self.current.span.start;

        // Check if it's a named element
        let saved = self.current;
        let name = if self.at_kind(TokenKind::Ident) {
            let potential_name = self.parse_name();
            if self.at_kind(TokenKind::Colon) {
                self.advance();
                Some(potential_name)
            } else {
                // Not a named element, restore
                self.current = saved;
                None
            }
        } else {
            None
        };

        let ty = self.parse_type();

        let end = self.current.span.start;
        TupleElement {
            name,
            ty,
            span: Span::new(start, end),
        }
    }

    /// Parses directives.
    fn parse_directives(&mut self) -> Vec<Directive<'a>> {
        let mut directives = Vec::new();
        while self.at_kind(TokenKind::At) {
            directives.push(self.parse_directive());
        }
        directives
    }

    /// Parses a directive.
    fn parse_directive(&mut self) -> Directive<'a> {
        let start = self.current.span.start;
        self.advance(); // @

        let name = self.parse_name();
        let arguments = if self.at_kind(TokenKind::LParen) {
            self.advance();
            let args = self.parse_arguments();
            self.expect(TokenKind::RParen);
            args
        } else {
            Vec::new()
        };

        let end = self.current.span.start;
        Directive {
            name,
            arguments,
            span: Span::new(start, end),
        }
    }

    /// Parses arguments.
    fn parse_arguments(&mut self) -> Vec<Argument<'a>> {
        let mut args = Vec::new();
        while !self.at_kind(TokenKind::RParen) && !self.at_kind(TokenKind::Eof) {
            args.push(self.parse_argument());
        }
        args
    }

    /// Parses an argument.
    fn parse_argument(&mut self) -> Argument<'a> {
        let start = self.current.span.start;
        let name = self.parse_name();
        self.expect(TokenKind::Colon);
        let value = self.parse_value();
        let end = self.current.span.start;
        Argument {
            name,
            value,
            span: Span::new(start, end),
        }
    }

    /// Parses a value.
    fn parse_value(&mut self) -> Value<'a> {
        let start = self.current.span.start;

        match self.at() {
            TokenKind::Dollar => {
                self.advance();
                let name = self.parse_name();
                Value::Variable(name)
            }
            TokenKind::IntLiteral => {
                let text = self.current_text();
                let value = text.parse().unwrap_or(0);
                self.advance();
                Value::Int(value, Span::new(start, self.current.span.start))
            }
            TokenKind::FloatLiteral => {
                let text = self.current_text();
                let value = text.parse().unwrap_or(0.0);
                self.advance();
                Value::Float(value, Span::new(start, self.current.span.start))
            }
            TokenKind::StringLiteral | TokenKind::BlockStringLiteral => {
                let text = self.current_text();
                let value = if text.starts_with("\"\"\"") {
                    text[3..text.len() - 3].to_string()
                } else {
                    text[1..text.len() - 1].to_string()
                };
                self.advance();
                Value::String(value, Span::new(start, self.current.span.start))
            }
            TokenKind::True => {
                self.advance();
                Value::Boolean(true, Span::new(start, self.current.span.start))
            }
            TokenKind::False => {
                self.advance();
                Value::Boolean(false, Span::new(start, self.current.span.start))
            }
            TokenKind::Null => {
                self.advance();
                Value::Null(Span::new(start, self.current.span.start))
            }
            TokenKind::LBracket => {
                self.advance();
                let mut values = Vec::new();
                while !self.at_kind(TokenKind::RBracket) && !self.at_kind(TokenKind::Eof) {
                    values.push(self.parse_value());
                }
                self.expect(TokenKind::RBracket);
                Value::List(values, Span::new(start, self.current.span.start))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut fields = Vec::new();
                while !self.at_kind(TokenKind::RBrace) && !self.at_kind(TokenKind::Eof) {
                    let name = self.parse_name();
                    self.expect(TokenKind::Colon);
                    let value = self.parse_value();
                    fields.push((name, value));
                }
                self.expect(TokenKind::RBrace);
                Value::Object(fields, Span::new(start, self.current.span.start))
            }
            TokenKind::Ident => {
                let name = self.parse_name();
                Value::Enum(name)
            }
            _ => {
                self.error("expected value");
                self.advance();
                Value::Null(Span::new(start, self.current.span.start))
            }
        }
    }

    /// Parses an operation definition.
    fn parse_operation(&mut self) -> OperationDefinition<'a> {
        let start = self.current.span.start;

        let (operation, name) = if self.at_kind(TokenKind::LBrace) {
            // Anonymous query
            (OperationType::Query, None)
        } else {
            let op = match self.at() {
                TokenKind::Query => OperationType::Query,
                TokenKind::Mutation => OperationType::Mutation,
                TokenKind::Subscription => OperationType::Subscription,
                _ => OperationType::Query,
            };
            self.advance();

            let name = if self.at_kind(TokenKind::Ident) {
                Some(self.parse_name())
            } else {
                None
            };

            (op, name)
        };

        let variables = if self.at_kind(TokenKind::LParen) {
            self.advance();
            let vars = self.parse_variable_definitions();
            self.expect(TokenKind::RParen);
            vars
        } else {
            Vec::new()
        };

        let directives = self.parse_directives();
        let selection_set = self.parse_selection_set();

        let end = self.current.span.start;
        OperationDefinition {
            operation,
            name,
            variables,
            directives,
            selection_set,
            span: Span::new(start, end),
        }
    }

    /// Parses variable definitions.
    fn parse_variable_definitions(&mut self) -> Vec<VariableDefinition<'a>> {
        let mut vars = Vec::new();
        while !self.at_kind(TokenKind::RParen) && !self.at_kind(TokenKind::Eof) {
            vars.push(self.parse_variable_definition());
        }
        vars
    }

    /// Parses a variable definition.
    fn parse_variable_definition(&mut self) -> VariableDefinition<'a> {
        let start = self.current.span.start;
        self.expect(TokenKind::Dollar);
        let name = self.parse_name();
        self.expect(TokenKind::Colon);
        let ty = self.parse_type();

        let default_value = if self.at_kind(TokenKind::Eq) {
            self.advance();
            Some(self.parse_value())
        } else {
            None
        };

        let directives = self.parse_directives();

        let end = self.current.span.start;
        VariableDefinition {
            name,
            ty,
            default_value,
            directives,
            span: Span::new(start, end),
        }
    }

    /// Parses a fragment definition.
    fn parse_fragment_definition(&mut self) -> FragmentDefinition<'a> {
        let start = self.current.span.start;
        self.advance(); // fragment

        let name = self.parse_name();
        self.expect(TokenKind::On);
        let type_condition = self.parse_name();
        let directives = self.parse_directives();
        let selection_set = self.parse_selection_set();

        let end = self.current.span.start;
        FragmentDefinition {
            name,
            type_condition,
            directives,
            selection_set,
            span: Span::new(start, end),
        }
    }

    /// Parses a selection set.
    fn parse_selection_set(&mut self) -> SelectionSet<'a> {
        let start = self.current.span.start;
        self.expect(TokenKind::LBrace);

        let mut selections = Vec::new();
        while !self.at_kind(TokenKind::RBrace) && !self.at_kind(TokenKind::Eof) {
            selections.push(self.parse_selection());
        }
        self.expect(TokenKind::RBrace);

        let end = self.current.span.start;
        SelectionSet {
            selections,
            span: Span::new(start, end),
        }
    }

    /// Parses a selection.
    fn parse_selection(&mut self) -> Selection<'a> {
        if self.at_kind(TokenKind::Spread) {
            self.advance();
            if self.at_kind(TokenKind::On) {
                // Inline fragment
                self.advance();
                let type_condition = Some(self.parse_name());
                let directives = self.parse_directives();
                let selection_set = self.parse_selection_set();
                Selection::InlineFragment(InlineFragment {
                    type_condition,
                    directives,
                    selection_set,
                    span: self.current.span,
                })
            } else if self.at_kind(TokenKind::LBrace) || self.at_kind(TokenKind::At) {
                // Inline fragment without type condition
                let directives = self.parse_directives();
                let selection_set = self.parse_selection_set();
                Selection::InlineFragment(InlineFragment {
                    type_condition: None,
                    directives,
                    selection_set,
                    span: self.current.span,
                })
            } else {
                // Fragment spread
                let name = self.parse_name();
                let directives = self.parse_directives();
                Selection::FragmentSpread(FragmentSpread {
                    name,
                    directives,
                    span: self.current.span,
                })
            }
        } else {
            // Field
            Selection::Field(self.parse_field_selection())
        }
    }

    /// Parses a field selection.
    fn parse_field_selection(&mut self) -> FieldSelection<'a> {
        let start = self.current.span.start;

        // Check for alias
        let first_name = self.parse_name();
        let (alias, name) = if self.at_kind(TokenKind::Colon) {
            self.advance();
            (Some(first_name), self.parse_name())
        } else {
            (None, first_name)
        };

        let arguments = if self.at_kind(TokenKind::LParen) {
            self.advance();
            let args = self.parse_arguments();
            self.expect(TokenKind::RParen);
            args
        } else {
            Vec::new()
        };

        let directives = self.parse_directives();

        let selection_set = if self.at_kind(TokenKind::LBrace) {
            Some(self.parse_selection_set())
        } else {
            None
        };

        let end = self.current.span.start;
        FieldSelection {
            alias,
            name,
            arguments,
            directives,
            selection_set,
            span: Span::new(start, end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_type() {
        let interner = Interner::new();
        let result = parse("type Query { hello: String }", &interner);
        assert!(!result.diagnostics.has_errors());
        assert_eq!(result.document.definitions.len(), 1);
    }

    #[test]
    fn test_parse_opaque_type() {
        let interner = Interner::new();
        let result = parse("opaque Email = String", &interner);
        assert!(!result.diagnostics.has_errors());
    }

    #[test]
    fn test_parse_generic_type() {
        let interner = Interner::new();
        let result = parse("type Connection<T> { edges: List<T> }", &interner);
        assert!(!result.diagnostics.has_errors());
    }

    #[test]
    fn test_parse_rust_enum() {
        let interner = Interner::new();
        let result = parse(
            r#"
            enum Result {
                Ok(String)
                Err { message: String, code: Int }
            }
        "#,
            &interner,
        );
        assert!(!result.diagnostics.has_errors());
    }

    #[test]
    fn test_parse_mod_external() {
        let interner = Interner::new();
        let result = parse("mod users;", &interner);
        assert!(!result.diagnostics.has_errors());
        assert_eq!(result.document.definitions.len(), 1);
        match &result.document.definitions[0] {
            Definition::Module(m) => {
                assert_eq!(interner.get(m.name.value), "users");
                assert!(m.body.is_none());
            }
            _ => panic!("expected module definition"),
        }
    }

    #[test]
    fn test_parse_mod_inline() {
        let interner = Interner::new();
        let result = parse(
            r#"
            mod auth {
                pub type Token {
                    value: String
                }
            }
        "#,
            &interner,
        );
        assert!(!result.diagnostics.has_errors());
        assert_eq!(result.document.definitions.len(), 1);
        match &result.document.definitions[0] {
            Definition::Module(m) => {
                assert_eq!(interner.get(m.name.value), "auth");
                assert!(m.body.is_some());
                assert_eq!(m.body.as_ref().unwrap().len(), 1);
            }
            _ => panic!("expected module definition"),
        }
    }

    #[test]
    fn test_parse_pub_type() {
        let interner = Interner::new();
        let result = parse("pub type User { id: ID }", &interner);
        assert!(!result.diagnostics.has_errors());
        match &result.document.definitions[0] {
            Definition::Type(TypeDefinition::Object(obj)) => {
                assert_eq!(obj.visibility, Visibility::Public);
            }
            _ => panic!("expected object type definition"),
        }
    }

    #[test]
    fn test_parse_use_statement() {
        let interner = Interner::new();
        let result = parse("use::users::{User, UserResult}", &interner);
        assert!(!result.diagnostics.has_errors());
        assert_eq!(result.document.definitions.len(), 1);
        match &result.document.definitions[0] {
            Definition::Use(u) => {
                assert_eq!(u.path.len(), 1);
                assert_eq!(interner.get(u.path[0].value), "users");
                match &u.items {
                    UseItems::Named(items) => {
                        assert_eq!(items.len(), 2);
                    }
                    _ => panic!("expected named items"),
                }
            }
            _ => panic!("expected use statement"),
        }
    }

    #[test]
    fn test_parse_use_glob() {
        let interner = Interner::new();
        let result = parse("use::common::*", &interner);
        assert!(!result.diagnostics.has_errors());
        match &result.document.definitions[0] {
            Definition::Use(u) => {
                assert!(matches!(u.items, UseItems::Glob));
            }
            _ => panic!("expected use statement"),
        }
    }

    #[test]
    fn test_parse_use_alias() {
        let interner = Interner::new();
        let result = parse("use::external::User as ExternalUser", &interner);
        assert!(!result.diagnostics.has_errors());
        match &result.document.definitions[0] {
            Definition::Use(u) => {
                match &u.items {
                    UseItems::Named(items) => {
                        assert_eq!(items.len(), 1);
                        assert!(items[0].alias.is_some());
                    }
                    _ => panic!("expected named items with alias"),
                }
            }
            _ => panic!("expected use statement"),
        }
    }
}
