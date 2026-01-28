//! Hover information provider.

use bgql_core::Interner;
use bgql_syntax::{Definition, Document, Type, TypeDefinition};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::symbols::{position_to_offset, SymbolTable, SymbolType};

/// Get hover information at the given position.
pub fn get_hover(
    content: &str,
    line: u32,
    character: u32,
    document: &Document<'_>,
    interner: &Interner,
) -> Option<Hover> {
    let offset = position_to_offset(
        content,
        tower_lsp::lsp_types::Position::new(line, character),
    );

    // Build symbol table
    let symbols = SymbolTable::from_document(document, interner);

    // Find symbol at position
    if let Some(symbol) = symbols.find_symbol_at(offset as u32) {
        let markdown = build_hover_markdown(symbol, &symbols, document, interner);
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: None,
        });
    }

    // Try to find a type reference at position
    if let Some(hover) = find_type_at_position(content, offset, document, interner) {
        return Some(hover);
    }

    None
}

fn build_hover_markdown(
    symbol: &crate::symbols::Symbol,
    _symbols: &SymbolTable,
    _document: &Document<'_>,
    _interner: &Interner,
) -> String {
    let mut markdown = String::new();

    // Add code block with symbol signature
    markdown.push_str("```bgql\n");
    match symbol.kind {
        SymbolType::Type => {
            markdown.push_str(&format!("type {}", symbol.name));
        }
        SymbolType::Interface => {
            markdown.push_str(&format!("interface {}", symbol.name));
        }
        SymbolType::Enum => {
            markdown.push_str(&format!("enum {}", symbol.name));
        }
        SymbolType::EnumValue => {
            markdown.push_str(&symbol.name);
        }
        SymbolType::Union => {
            markdown.push_str(&format!("union {}", symbol.name));
        }
        SymbolType::Input => {
            markdown.push_str(&format!("input {}", symbol.name));
        }
        SymbolType::Scalar => {
            markdown.push_str(&format!("scalar {}", symbol.name));
        }
        SymbolType::Opaque => {
            markdown.push_str(&format!("opaque {}", symbol.name));
        }
        SymbolType::Alias => {
            markdown.push_str(&format!("alias {}", symbol.name));
        }
        SymbolType::InputUnion => {
            markdown.push_str(&format!("input union {}", symbol.name));
        }
        SymbolType::Field => {
            markdown.push_str(&format!("{}: ...", symbol.name));
        }
        SymbolType::Argument => {
            markdown.push_str(&format!("(parameter) {}", symbol.name));
        }
        SymbolType::Directive => {
            markdown.push_str(&format!("directive @{}", symbol.name));
        }
        SymbolType::Query => {
            markdown.push_str(&format!("query {}", symbol.name));
        }
        SymbolType::Mutation => {
            markdown.push_str(&format!("mutation {}", symbol.name));
        }
        SymbolType::Subscription => {
            markdown.push_str(&format!("subscription {}", symbol.name));
        }
        SymbolType::Fragment => {
            markdown.push_str(&format!("fragment {}", symbol.name));
        }
    }
    markdown.push_str("\n```\n");

    // Add description if present
    if let Some(desc) = &symbol.description {
        markdown.push_str("\n---\n\n");
        markdown.push_str(desc);
    }

    markdown
}

fn find_type_at_position(
    content: &str,
    offset: usize,
    document: &Document<'_>,
    interner: &Interner,
) -> Option<Hover> {
    // Get the word at the cursor position
    let word = get_word_at_offset(content, offset)?;

    // Check if it's a built-in type
    if let Some(hover) = get_builtin_type_hover(&word) {
        return Some(hover);
    }

    // Look up the type in the document
    for def in &document.definitions {
        if let Definition::Type(type_def) = def {
            let type_name = get_type_name(type_def, interner)?;
            if type_name == word {
                return Some(build_type_hover(type_def, interner));
            }
        }
    }

    None
}

fn get_word_at_offset(content: &str, offset: usize) -> Option<String> {
    let bytes = content.as_bytes();

    // Find word start
    let mut start = offset;
    while start > 0 && is_identifier_char(bytes[start - 1]) {
        start -= 1;
    }

    // Find word end
    let mut end = offset;
    while end < bytes.len() && is_identifier_char(bytes[end]) {
        end += 1;
    }

    if start < end {
        Some(content[start..end].to_string())
    } else {
        None
    }
}

fn is_identifier_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn get_builtin_type_hover(name: &str) -> Option<Hover> {
    let description = match name {
        "Int" => "A signed 32-bit integer.",
        "Float" => "A signed double-precision floating-point value.",
        "String" => "A UTF-8 character sequence.",
        "Boolean" => "A `true` or `false` value.",
        "ID" => "A unique identifier, serialized as a string.",
        "Option" => "An optional value that may be null.\n\n```bgql\nOption<T>\n```",
        "List" => "A list of values.\n\n```bgql\nList<T>\n```",
        "DateTime" => "An ISO 8601 date-time string.",
        "JSON" => "Arbitrary JSON data.",
        _ => return None,
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```bgql\n{}\n```\n\n---\n\n{}", name, description),
        }),
        range: None,
    })
}

fn get_type_name<'a>(type_def: &'a TypeDefinition<'a>, interner: &'a Interner) -> Option<String> {
    let name = match type_def {
        TypeDefinition::Object(obj) => interner.get(obj.name.value),
        TypeDefinition::Interface(iface) => interner.get(iface.name.value),
        TypeDefinition::Enum(e) => interner.get(e.name.value),
        TypeDefinition::Union(u) => interner.get(u.name.value),
        TypeDefinition::Input(inp) => interner.get(inp.name.value),
        TypeDefinition::Scalar(s) => interner.get(s.name.value),
        TypeDefinition::Opaque(o) => interner.get(o.name.value),
        TypeDefinition::TypeAlias(a) => interner.get(a.name.value),
        TypeDefinition::InputUnion(iu) => interner.get(iu.name.value),
    };
    Some(name.to_string())
}

fn build_type_hover(type_def: &TypeDefinition<'_>, interner: &Interner) -> Hover {
    let mut markdown = String::new();

    markdown.push_str("```bgql\n");
    match type_def {
        TypeDefinition::Object(obj) => {
            let name = interner.get(obj.name.value);
            markdown.push_str(&format!("type {}", name));
            if !obj.implements.is_empty() {
                let impls: Vec<_> = obj
                    .implements
                    .iter()
                    .map(|i| interner.get(i.value))
                    .collect();
                markdown.push_str(&format!(" implements {}", impls.join(" & ")));
            }
            markdown.push_str(" { ... }");
        }
        TypeDefinition::Interface(iface) => {
            let name = interner.get(iface.name.value);
            markdown.push_str(&format!("interface {} {{ ... }}", name));
        }
        TypeDefinition::Enum(e) => {
            let name = interner.get(e.name.value);
            let values: Vec<_> = e
                .values
                .iter()
                .take(5)
                .map(|v| interner.get(v.name.value))
                .collect();
            let suffix = if e.values.len() > 5 { ", ..." } else { "" };
            markdown.push_str(&format!(
                "enum {} {{ {}{} }}",
                name,
                values.join(", "),
                suffix
            ));
        }
        TypeDefinition::Union(u) => {
            let name = interner.get(u.name.value);
            let members: Vec<_> = u.members.iter().map(|m| interner.get(m.value)).collect();
            markdown.push_str(&format!("union {} = {}", name, members.join(" | ")));
        }
        TypeDefinition::Input(inp) => {
            let name = interner.get(inp.name.value);
            markdown.push_str(&format!("input {} {{ ... }}", name));
        }
        TypeDefinition::Scalar(s) => {
            let name = interner.get(s.name.value);
            markdown.push_str(&format!("scalar {}", name));
        }
        TypeDefinition::Opaque(o) => {
            let name = interner.get(o.name.value);
            let underlying = format_type(&o.underlying, interner);
            markdown.push_str(&format!("opaque {} = {}", name, underlying));
        }
        TypeDefinition::TypeAlias(a) => {
            let name = interner.get(a.name.value);
            let aliased = format_type(&a.aliased, interner);
            markdown.push_str(&format!("alias {} = {}", name, aliased));
        }
        TypeDefinition::InputUnion(iu) => {
            let name = interner.get(iu.name.value);
            let members: Vec<_> = iu.members.iter().map(|m| interner.get(m.value)).collect();
            markdown.push_str(&format!("input union {} = {}", name, members.join(" | ")));
        }
    }
    markdown.push_str("\n```");

    // Add description
    let desc = match type_def {
        TypeDefinition::Object(obj) => obj.description.as_ref().map(|d| d.value),
        TypeDefinition::Interface(iface) => iface.description.as_ref().map(|d| d.value),
        TypeDefinition::Enum(e) => e.description.as_ref().map(|d| d.value),
        TypeDefinition::Union(u) => u.description.as_ref().map(|d| d.value),
        TypeDefinition::Input(inp) => inp.description.as_ref().map(|d| d.value),
        TypeDefinition::Scalar(s) => s.description.as_ref().map(|d| d.value),
        TypeDefinition::Opaque(o) => o.description.as_ref().map(|d| d.value),
        TypeDefinition::TypeAlias(a) => a.description.as_ref().map(|d| d.value),
        TypeDefinition::InputUnion(iu) => iu.description.as_ref().map(|d| d.value),
    };

    if let Some(desc) = desc {
        markdown.push_str("\n\n---\n\n");
        markdown.push_str(desc);
    }

    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: None,
    }
}

fn format_type(ty: &Type<'_>, interner: &Interner) -> String {
    match ty {
        Type::Named(named) => interner.get(named.name).to_string(),
        Type::Option(inner, _) => format!("Option<{}>", format_type(inner, interner)),
        Type::List(inner, _) => format!("List<{}>", format_type(inner, interner)),
        Type::Generic(gen) => {
            let args: Vec<_> = gen
                .arguments
                .iter()
                .map(|a| format_type(a, interner))
                .collect();
            format!("{}<{}>", interner.get(gen.name), args.join(", "))
        }
        Type::Tuple(tuple) => {
            let elements: Vec<_> = tuple
                .elements
                .iter()
                .map(|e| format_type(&e.ty, interner))
                .collect();
            format!("({})", elements.join(", "))
        }
        Type::_Phantom(_) => String::new(),
    }
}
