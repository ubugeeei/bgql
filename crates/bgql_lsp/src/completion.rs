//! Completion provider.

use bgql_core::Interner;
use bgql_syntax::{Definition, Document, TypeDefinition};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat, Position};

use crate::symbols::position_to_offset;

/// Completion context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionContext {
    /// Top level of document
    TopLevel,
    /// Inside a type body (after `{`)
    TypeBody,
    /// After a colon (expecting type)
    TypePosition,
    /// After `@` (expecting directive)
    Directive,
    /// After `implements`
    Implements,
    /// Inside arguments
    Arguments,
    /// Unknown
    Unknown,
}

/// Get completions at the given position.
pub fn get_completions(
    content: &str,
    position: Position,
    document: &Document<'_>,
    interner: &Interner,
) -> Vec<CompletionItem> {
    let offset = position_to_offset(content, position);
    let context = determine_context(content, offset);

    let mut completions = Vec::new();

    match context {
        CompletionContext::TopLevel => {
            completions.extend(keyword_completions());
        }
        CompletionContext::TypePosition => {
            completions.extend(type_completions(document, interner));
            completions.extend(builtin_type_completions());
        }
        CompletionContext::Directive => {
            completions.extend(directive_completions(document, interner));
        }
        CompletionContext::Implements => {
            completions.extend(interface_completions(document, interner));
        }
        CompletionContext::TypeBody | CompletionContext::Arguments => {
            // Field names suggestions could go here
        }
        CompletionContext::Unknown => {
            // Provide all possible completions
            completions.extend(keyword_completions());
            completions.extend(builtin_type_completions());
            completions.extend(type_completions(document, interner));
        }
    }

    completions
}

fn determine_context(content: &str, offset: usize) -> CompletionContext {
    let before = &content[..offset.min(content.len())];

    // Check for directive context
    if before.ends_with('@') {
        return CompletionContext::Directive;
    }

    // Check for type position (after `:`)
    let trimmed = before.trim_end();
    if trimmed.ends_with(':') {
        return CompletionContext::TypePosition;
    }

    // Check for implements context
    if trimmed.ends_with("implements")
        || before.contains("implements")
            && before
                .rfind("implements")
                .map(|i| {
                    let after = &before[i + 10..];
                    !after.contains('{')
                })
                .unwrap_or(false)
    {
        return CompletionContext::Implements;
    }

    // Check if inside braces
    let open_braces = before.matches('{').count();
    let close_braces = before.matches('}').count();
    if open_braces > close_braces {
        // Check if we're in arguments
        let open_parens = before.matches('(').count();
        let close_parens = before.matches(')').count();
        if open_parens > close_parens {
            return CompletionContext::Arguments;
        }
        return CompletionContext::TypeBody;
    }

    // Check for top-level keywords
    let lines: Vec<_> = before.lines().collect();
    if let Some(last_line) = lines.last() {
        let trimmed_line = last_line.trim();
        if trimmed_line.is_empty() || !trimmed_line.contains('{') {
            return CompletionContext::TopLevel;
        }
    }

    CompletionContext::Unknown
}

fn keyword_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "type".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define an object type".to_string()),
            insert_text: Some("type ${1:Name} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "interface".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define an interface".to_string()),
            insert_text: Some("interface ${1:Name} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "enum".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define an enum".to_string()),
            insert_text: Some("enum ${1:Name} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "union".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a union type".to_string()),
            insert_text: Some("union ${1:Name} = $0".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "input".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define an input type".to_string()),
            insert_text: Some("input ${1:Name} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "scalar".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a scalar type".to_string()),
            insert_text: Some("scalar ${0:Name}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "opaque".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define an opaque (newtype) type".to_string()),
            insert_text: Some("opaque ${1:Name} = ${0:String}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "directive".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a directive".to_string()),
            insert_text: Some("directive @${1:name} on ${0:FIELD_DEFINITION}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "query".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a query operation".to_string()),
            insert_text: Some("query ${1:Name} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "mutation".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a mutation operation".to_string()),
            insert_text: Some("mutation ${1:Name} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "subscription".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a subscription operation".to_string()),
            insert_text: Some("subscription ${1:Name} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "fragment".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a fragment".to_string()),
            insert_text: Some("fragment ${1:Name} on ${2:Type} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "extend".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Extend an existing type".to_string()),
            insert_text: Some("extend type ${1:Name} {\n  $0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "schema".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define the schema".to_string()),
            insert_text: Some(
                "schema {\n  query: ${1:Query}\n  mutation: ${2:Mutation}\n}".to_string(),
            ),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
    ]
}

fn builtin_type_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "Int".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("32-bit signed integer".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "Float".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("Double-precision floating point".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "String".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("UTF-8 string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "Boolean".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("true or false".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "ID".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("Unique identifier".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "Option".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("Optional (nullable) type".to_string()),
            insert_text: Some("Option<${0:T}>".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "List".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("List type".to_string()),
            insert_text: Some("List<${0:T}>".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "DateTime".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("ISO 8601 date-time".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "JSON".to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some("Arbitrary JSON data".to_string()),
            ..Default::default()
        },
    ]
}

fn type_completions(document: &Document<'_>, interner: &Interner) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    for def in &document.definitions {
        if let Definition::Type(type_def) = def {
            if let Some(item) = type_def_to_completion(type_def, interner) {
                completions.push(item);
            }
        }
    }

    completions
}

fn type_def_to_completion(
    type_def: &TypeDefinition<'_>,
    interner: &Interner,
) -> Option<CompletionItem> {
    let (name, kind, detail) = match type_def {
        TypeDefinition::Object(obj) => {
            let name = interner.get(obj.name.value);
            (name, CompletionItemKind::CLASS, "object type")
        }
        TypeDefinition::Interface(iface) => {
            let name = interner.get(iface.name.value);
            (name, CompletionItemKind::INTERFACE, "interface")
        }
        TypeDefinition::Enum(e) => {
            let name = interner.get(e.name.value);
            (name, CompletionItemKind::ENUM, "enum")
        }
        TypeDefinition::Union(u) => {
            let name = interner.get(u.name.value);
            (name, CompletionItemKind::ENUM, "union")
        }
        TypeDefinition::Input(inp) => {
            let name = interner.get(inp.name.value);
            (name, CompletionItemKind::STRUCT, "input type")
        }
        TypeDefinition::Scalar(s) => {
            let name = interner.get(s.name.value);
            (name, CompletionItemKind::TYPE_PARAMETER, "scalar")
        }
        TypeDefinition::Opaque(o) => {
            let name = interner.get(o.name.value);
            (name, CompletionItemKind::CLASS, "opaque type")
        }
        TypeDefinition::TypeAlias(a) => {
            let name = interner.get(a.name.value);
            (name, CompletionItemKind::TYPE_PARAMETER, "type alias")
        }
        TypeDefinition::InputUnion(iu) => {
            let name = interner.get(iu.name.value);
            (name, CompletionItemKind::ENUM, "input union")
        }
    };

    Some(CompletionItem {
        label: name.to_string(),
        kind: Some(kind),
        detail: Some(detail.to_string()),
        ..Default::default()
    })
}

fn interface_completions(document: &Document<'_>, interner: &Interner) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    for def in &document.definitions {
        if let Definition::Type(TypeDefinition::Interface(iface)) = def {
            let name = interner.get(iface.name.value);
            completions.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::INTERFACE),
                detail: Some("interface".to_string()),
                ..Default::default()
            });
        }
    }

    completions
}

fn directive_completions(document: &Document<'_>, interner: &Interner) -> Vec<CompletionItem> {
    let mut completions = vec![
        // Built-in directives
        CompletionItem {
            label: "deprecated".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Mark as deprecated".to_string()),
            insert_text: Some("deprecated(reason: \"${0:reason}\")".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "skip".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Skip field conditionally".to_string()),
            insert_text: Some("skip(if: ${0:condition})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "include".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Include field conditionally".to_string()),
            insert_text: Some("include(if: ${0:condition})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
    ];

    // User-defined directives
    for def in &document.definitions {
        if let Definition::Directive(dir) = def {
            let name = interner.get(dir.name.value);
            completions.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: dir.description.as_ref().map(|d| d.value.to_string()),
                ..Default::default()
            });
        }
    }

    completions
}
