//! Symbol table and document indexing.

use bgql_core::{Interner, Span};
use bgql_syntax::{
    Definition, Document, EnumTypeDefinition, FieldDefinition, InputEnumTypeDefinition,
    InputObjectTypeDefinition, InputUnionTypeDefinition, InterfaceTypeDefinition,
    ObjectTypeDefinition, OpaqueTypeDefinition, ScalarTypeDefinition, TypeAliasDefinition,
    TypeDefinition, UnionTypeDefinition,
};
use std::collections::HashMap;
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};

/// A symbol in the document.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolType,
    pub span: Span,
    pub description: Option<String>,
    pub children: Vec<Symbol>,
}

/// Type of symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    Type,
    Interface,
    Enum,
    EnumValue,
    Union,
    Input,
    Scalar,
    Opaque,
    Alias,
    InputUnion,
    InputEnum,
    Field,
    Argument,
    Directive,
    Query,
    Mutation,
    Subscription,
    Fragment,
    Module,
}

impl SymbolType {
    pub fn to_lsp_kind(self) -> SymbolKind {
        match self {
            SymbolType::Type | SymbolType::Opaque | SymbolType::Alias => SymbolKind::CLASS,
            SymbolType::Interface => SymbolKind::INTERFACE,
            SymbolType::Enum => SymbolKind::ENUM,
            SymbolType::EnumValue => SymbolKind::ENUM_MEMBER,
            SymbolType::Union | SymbolType::InputUnion | SymbolType::InputEnum => SymbolKind::ENUM,
            SymbolType::Input => SymbolKind::STRUCT,
            SymbolType::Scalar => SymbolKind::TYPE_PARAMETER,
            SymbolType::Field => SymbolKind::FIELD,
            SymbolType::Argument => SymbolKind::VARIABLE,
            SymbolType::Directive => SymbolKind::FUNCTION,
            SymbolType::Query | SymbolType::Mutation | SymbolType::Subscription => {
                SymbolKind::METHOD
            }
            SymbolType::Fragment => SymbolKind::FUNCTION,
            SymbolType::Module => SymbolKind::MODULE,
        }
    }
}

/// Symbol table for a document.
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// All symbols by name.
    pub symbols: HashMap<String, Symbol>,
    /// Root-level symbols in document order.
    pub root_symbols: Vec<Symbol>,
}

impl SymbolTable {
    /// Build a symbol table from a parsed document.
    pub fn from_document(document: &Document<'_>, interner: &Interner) -> Self {
        let mut table = Self::default();

        for def in &document.definitions {
            if let Some(symbol) = table.index_definition(def, interner) {
                table.symbols.insert(symbol.name.clone(), symbol.clone());
                table.root_symbols.push(symbol);
            }
        }

        table
    }

    fn index_definition(&mut self, def: &Definition<'_>, interner: &Interner) -> Option<Symbol> {
        match def {
            Definition::Type(type_def) => self.index_type_definition(type_def, interner),
            Definition::Directive(dir) => {
                let name = interner.get(dir.name.value).to_string();
                Some(Symbol {
                    name,
                    kind: SymbolType::Directive,
                    span: dir.span,
                    description: dir.description.as_ref().map(|d| d.value.to_string()),
                    children: Vec::new(),
                })
            }
            Definition::Operation(op) => {
                let name = op
                    .name
                    .as_ref()
                    .map(|n| interner.get(n.value).to_string())
                    .unwrap_or_else(|| "anonymous".to_string());
                let kind = match op.operation {
                    bgql_syntax::OperationType::Query => SymbolType::Query,
                    bgql_syntax::OperationType::Mutation => SymbolType::Mutation,
                    bgql_syntax::OperationType::Subscription => SymbolType::Subscription,
                };
                Some(Symbol {
                    name,
                    kind,
                    span: op.span,
                    description: None,
                    children: Vec::new(),
                })
            }
            Definition::Fragment(frag) => {
                let name = interner.get(frag.name.value).to_string();
                Some(Symbol {
                    name,
                    kind: SymbolType::Fragment,
                    span: frag.span,
                    description: None,
                    children: Vec::new(),
                })
            }
            Definition::Schema(_) => None,
            Definition::Module(m) => {
                let name = interner.get(m.name.value).to_string();
                let children = m
                    .body
                    .as_ref()
                    .map(|body| {
                        body.iter()
                            .filter_map(|def| self.index_definition(def, interner))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(Symbol {
                    name,
                    kind: SymbolType::Module,
                    span: m.span,
                    description: None,
                    children,
                })
            }
            Definition::Use(_) => None, // Use statements don't create symbols
        }
    }

    fn index_type_definition(
        &mut self,
        type_def: &TypeDefinition<'_>,
        interner: &Interner,
    ) -> Option<Symbol> {
        match type_def {
            TypeDefinition::Object(obj) => Some(self.index_object_type(obj, interner)),
            TypeDefinition::Interface(iface) => Some(self.index_interface_type(iface, interner)),
            TypeDefinition::Enum(e) => Some(self.index_enum_type(e, interner)),
            TypeDefinition::Union(u) => Some(self.index_union_type(u, interner)),
            TypeDefinition::Input(inp) => Some(self.index_input_type(inp, interner)),
            TypeDefinition::Scalar(s) => Some(self.index_scalar_type(s, interner)),
            TypeDefinition::Opaque(o) => Some(self.index_opaque_type(o, interner)),
            TypeDefinition::TypeAlias(a) => Some(self.index_type_alias(a, interner)),
            TypeDefinition::InputUnion(iu) => Some(self.index_input_union_type(iu, interner)),
            TypeDefinition::InputEnum(ie) => Some(self.index_input_enum_type(ie, interner)),
        }
    }

    fn index_object_type(&mut self, obj: &ObjectTypeDefinition<'_>, interner: &Interner) -> Symbol {
        let name = interner.get(obj.name.value).to_string();
        let children = obj
            .fields
            .iter()
            .map(|f| self.index_field(f, interner))
            .collect();

        Symbol {
            name,
            kind: SymbolType::Type,
            span: obj.span,
            description: obj.description.as_ref().map(|d| d.value.to_string()),
            children,
        }
    }

    fn index_interface_type(
        &mut self,
        iface: &InterfaceTypeDefinition<'_>,
        interner: &Interner,
    ) -> Symbol {
        let name = interner.get(iface.name.value).to_string();
        let children = iface
            .fields
            .iter()
            .map(|f| self.index_field(f, interner))
            .collect();

        Symbol {
            name,
            kind: SymbolType::Interface,
            span: iface.span,
            description: iface.description.as_ref().map(|d| d.value.to_string()),
            children,
        }
    }

    fn index_enum_type(&mut self, e: &EnumTypeDefinition<'_>, interner: &Interner) -> Symbol {
        let name = interner.get(e.name.value).to_string();
        let children = e
            .values
            .iter()
            .map(|v| {
                let value_name = interner.get(v.name.value).to_string();
                Symbol {
                    name: value_name,
                    kind: SymbolType::EnumValue,
                    span: v.span,
                    description: v.description.as_ref().map(|d| d.value.to_string()),
                    children: Vec::new(),
                }
            })
            .collect();

        Symbol {
            name,
            kind: SymbolType::Enum,
            span: e.span,
            description: e.description.as_ref().map(|d| d.value.to_string()),
            children,
        }
    }

    fn index_union_type(&mut self, u: &UnionTypeDefinition<'_>, interner: &Interner) -> Symbol {
        let name = interner.get(u.name.value).to_string();
        Symbol {
            name,
            kind: SymbolType::Union,
            span: u.span,
            description: u.description.as_ref().map(|d| d.value.to_string()),
            children: Vec::new(),
        }
    }

    fn index_input_type(
        &mut self,
        inp: &InputObjectTypeDefinition<'_>,
        interner: &Interner,
    ) -> Symbol {
        let name = interner.get(inp.name.value).to_string();
        let children = inp
            .fields
            .iter()
            .map(|f| {
                let field_name = interner.get(f.name.value).to_string();
                Symbol {
                    name: field_name,
                    kind: SymbolType::Field,
                    span: f.span,
                    description: f.description.as_ref().map(|d| d.value.to_string()),
                    children: Vec::new(),
                }
            })
            .collect();

        Symbol {
            name,
            kind: SymbolType::Input,
            span: inp.span,
            description: inp.description.as_ref().map(|d| d.value.to_string()),
            children,
        }
    }

    fn index_scalar_type(&mut self, s: &ScalarTypeDefinition<'_>, interner: &Interner) -> Symbol {
        let name = interner.get(s.name.value).to_string();
        Symbol {
            name,
            kind: SymbolType::Scalar,
            span: s.span,
            description: s.description.as_ref().map(|d| d.value.to_string()),
            children: Vec::new(),
        }
    }

    fn index_opaque_type(&mut self, o: &OpaqueTypeDefinition<'_>, interner: &Interner) -> Symbol {
        let name = interner.get(o.name.value).to_string();
        Symbol {
            name,
            kind: SymbolType::Opaque,
            span: o.span,
            description: o.description.as_ref().map(|d| d.value.to_string()),
            children: Vec::new(),
        }
    }

    fn index_type_alias(&mut self, a: &TypeAliasDefinition<'_>, interner: &Interner) -> Symbol {
        let name = interner.get(a.name.value).to_string();
        Symbol {
            name,
            kind: SymbolType::Alias,
            span: a.span,
            description: a.description.as_ref().map(|d| d.value.to_string()),
            children: Vec::new(),
        }
    }

    fn index_input_union_type(
        &mut self,
        iu: &InputUnionTypeDefinition<'_>,
        interner: &Interner,
    ) -> Symbol {
        let name = interner.get(iu.name.value).to_string();
        Symbol {
            name,
            kind: SymbolType::InputUnion,
            span: iu.span,
            description: iu.description.as_ref().map(|d| d.value.to_string()),
            children: Vec::new(),
        }
    }

    fn index_input_enum_type(
        &mut self,
        ie: &InputEnumTypeDefinition<'_>,
        interner: &Interner,
    ) -> Symbol {
        let name = interner.get(ie.name.value).to_string();
        let children = ie
            .variants
            .iter()
            .map(|v| {
                let variant_name = interner.get(v.name.value).to_string();
                Symbol {
                    name: variant_name,
                    kind: SymbolType::EnumValue,
                    span: v.span,
                    description: v.description.as_ref().map(|d| d.value.to_string()),
                    children: Vec::new(),
                }
            })
            .collect();

        Symbol {
            name,
            kind: SymbolType::InputEnum,
            span: ie.span,
            description: ie.description.as_ref().map(|d| d.value.to_string()),
            children,
        }
    }

    fn index_field(&mut self, field: &FieldDefinition<'_>, interner: &Interner) -> Symbol {
        let name = interner.get(field.name.value).to_string();
        let children = field
            .arguments
            .iter()
            .map(|arg| {
                let arg_name = interner.get(arg.name.value).to_string();
                Symbol {
                    name: arg_name,
                    kind: SymbolType::Argument,
                    span: arg.span,
                    description: arg.description.as_ref().map(|d| d.value.to_string()),
                    children: Vec::new(),
                }
            })
            .collect();

        Symbol {
            name,
            kind: SymbolType::Field,
            span: field.span,
            description: field.description.as_ref().map(|d| d.value.to_string()),
            children,
        }
    }

    /// Find a symbol at the given offset.
    pub fn find_symbol_at(&self, offset: u32) -> Option<&Symbol> {
        for symbol in &self.root_symbols {
            if symbol.span.start <= offset && offset <= symbol.span.end {
                // Check children first (more specific)
                for child in &symbol.children {
                    if child.span.start <= offset && offset <= child.span.end {
                        return Some(child);
                    }
                }
                return Some(symbol);
            }
        }
        None
    }

    /// Get symbol by name.
    #[allow(dead_code)]
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }
}

/// Convert a Symbol to LSP DocumentSymbol.
pub fn symbol_to_document_symbol(symbol: &Symbol, content: &str) -> DocumentSymbol {
    let range = span_to_range(symbol.span, content);
    let selection_range = range; // Could be more specific to just the name

    #[allow(deprecated)]
    DocumentSymbol {
        name: symbol.name.clone(),
        detail: symbol.description.clone(),
        kind: symbol.kind.to_lsp_kind(),
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children: if symbol.children.is_empty() {
            None
        } else {
            Some(
                symbol
                    .children
                    .iter()
                    .map(|c| symbol_to_document_symbol(c, content))
                    .collect(),
            )
        },
    }
}

/// Convert a Span to LSP Range.
pub fn span_to_range(span: Span, content: &str) -> Range {
    Range {
        start: offset_to_position(content, span.start as usize),
        end: offset_to_position(content, span.end as usize),
    }
}

/// Convert a byte offset to LSP Position.
pub fn offset_to_position(content: &str, offset: usize) -> Position {
    let offset = offset.min(content.len());
    let before = &content[..offset];
    let line = before.matches('\n').count() as u32;
    let col = before
        .rfind('\n')
        .map(|pos| offset - pos - 1)
        .unwrap_or(offset) as u32;
    Position::new(line, col)
}

/// Convert LSP Position to byte offset.
pub fn position_to_offset(content: &str, position: Position) -> usize {
    let mut offset = 0;
    for (i, line) in content.lines().enumerate() {
        if i == position.line as usize {
            return offset + (position.character as usize).min(line.len());
        }
        offset += line.len() + 1; // +1 for newline
    }
    content.len()
}
