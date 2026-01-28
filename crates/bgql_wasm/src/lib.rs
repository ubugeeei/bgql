//! WebAssembly bindings for Better GraphQL.
//!
//! # JavaScript Usage
//!
//! ```javascript
//! import init, { BetterGraphQL } from '@bgql/wasm';
//!
//! await init();
//!
//! const bgql = new BetterGraphQL();
//! const result = bgql.parse(`
//!     type Query {
//!         hello: String
//!     }
//! `);
//!
//! if (result.success) {
//!     console.log('Types:', result.types);
//! } else {
//!     console.log('Errors:', result.diagnostics);
//! }
//! ```

use bgql_core::Interner;
use bgql_syntax::{format, parse, Definition, Type, TypeDefinition};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Clone)]
pub struct Diagnostic {
    pub severity: String,
    pub message: String,
    pub code: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Serialize, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub kind: String,
    pub description: Option<String>,
    pub fields: Vec<FieldInfo>,
    pub implements: Vec<String>,
    pub values: Vec<String>,
    pub members: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub type_name: String,
    pub description: Option<String>,
    pub arguments: Vec<ArgumentInfo>,
}

#[derive(Serialize, Clone)]
pub struct ArgumentInfo {
    pub name: String,
    pub type_name: String,
}

#[derive(Serialize)]
pub struct ParseResult {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub types: Vec<TypeInfo>,
}

#[derive(Serialize)]
pub struct ValidateResult {
    pub valid: bool,
    pub diagnostics: Vec<Diagnostic>,
}

/// The main Better GraphQL WebAssembly API.
#[wasm_bindgen]
pub struct BetterGraphQL {}

#[wasm_bindgen]
impl BetterGraphQL {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[wasm_bindgen]
    pub fn parse(&self, source: &str) -> JsValue {
        let interner = Interner::new();
        let result = parse(source, &interner);

        let mut diagnostics = Vec::new();
        let mut types = Vec::new();

        // Collect diagnostics
        for diag in result.diagnostics.iter() {
            let (start_line, start_col, end_line, end_col) = if let Some(span) = diag.primary_span()
            {
                let before = &source[..span.start as usize];
                let start_line = before.lines().count() as u32;
                let start_col = before
                    .lines()
                    .last()
                    .map(|l| l.len() as u32 + 1)
                    .unwrap_or(1);
                let end_before = &source[..span.end as usize];
                let end_line = end_before.lines().count() as u32;
                let end_col = end_before
                    .lines()
                    .last()
                    .map(|l| l.len() as u32 + 1)
                    .unwrap_or(1);
                (start_line, start_col, end_line, end_col)
            } else {
                (1, 1, 1, 1)
            };

            diagnostics.push(Diagnostic {
                severity: if diag.severity == bgql_core::DiagnosticSeverity::Error {
                    "error".to_string()
                } else {
                    "warning".to_string()
                },
                message: diag.title.clone(),
                code: diag.code.clone(),
                start_line,
                start_column: start_col,
                end_line,
                end_column: end_col,
            });
        }

        // Extract types
        for def in &result.document.definitions {
            if let Definition::Type(type_def) = def {
                types.push(extract_type_info(type_def, &interner));
            }
        }

        let parse_result = ParseResult {
            success: !result.diagnostics.has_errors(),
            diagnostics,
            types,
        };

        serde_wasm_bindgen::to_value(&parse_result).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn format(&self, source: &str) -> Result<String, JsValue> {
        let interner = Interner::new();
        let result = parse(source, &interner);

        if result.diagnostics.has_errors() {
            let errors: Vec<String> = result
                .diagnostics
                .errors()
                .map(|e| e.title.clone())
                .collect();
            return Err(JsValue::from_str(&errors.join("; ")));
        }

        Ok(format(&result.document, &interner))
    }

    #[wasm_bindgen]
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[wasm_bindgen]
    pub fn validate(&self, source: &str) -> JsValue {
        let interner = Interner::new();
        let result = parse(source, &interner);

        let mut diagnostics = Vec::new();

        for diag in result.diagnostics.iter() {
            let (start_line, start_col, end_line, end_col) = if let Some(span) = diag.primary_span()
            {
                let before = &source[..span.start as usize];
                let start_line = before.lines().count() as u32;
                let start_col = before
                    .lines()
                    .last()
                    .map(|l| l.len() as u32 + 1)
                    .unwrap_or(1);
                let end_before = &source[..span.end as usize];
                let end_line = end_before.lines().count() as u32;
                let end_col = end_before
                    .lines()
                    .last()
                    .map(|l| l.len() as u32 + 1)
                    .unwrap_or(1);
                (start_line, start_col, end_line, end_col)
            } else {
                (1, 1, 1, 1)
            };

            diagnostics.push(Diagnostic {
                severity: if diag.severity == bgql_core::DiagnosticSeverity::Error {
                    "error".to_string()
                } else {
                    "warning".to_string()
                },
                message: diag.title.clone(),
                code: diag.code.clone(),
                start_line,
                start_column: start_col,
                end_line,
                end_column: end_col,
            });
        }

        let validate_result = ValidateResult {
            valid: !result.diagnostics.has_errors(),
            diagnostics,
        };

        serde_wasm_bindgen::to_value(&validate_result).unwrap_or(JsValue::NULL)
    }
}

impl Default for BetterGraphQL {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_type_info(type_def: &TypeDefinition, interner: &Interner) -> TypeInfo {
    match type_def {
        TypeDefinition::Object(obj) => TypeInfo {
            name: interner.get(obj.name.value),
            kind: "OBJECT".to_string(),
            description: obj.description.as_ref().map(|d| d.value.to_string()),
            fields: obj
                .fields
                .iter()
                .map(|f| FieldInfo {
                    name: interner.get(f.name.value),
                    type_name: format_type(&f.ty, interner),
                    description: f.description.as_ref().map(|d| d.value.to_string()),
                    arguments: f
                        .arguments
                        .iter()
                        .map(|a| ArgumentInfo {
                            name: interner.get(a.name.value),
                            type_name: format_type(&a.ty, interner),
                        })
                        .collect(),
                })
                .collect(),
            implements: obj
                .implements
                .iter()
                .map(|i| interner.get(i.value))
                .collect(),
            values: vec![],
            members: vec![],
        },
        TypeDefinition::Interface(iface) => TypeInfo {
            name: interner.get(iface.name.value),
            kind: "INTERFACE".to_string(),
            description: iface.description.as_ref().map(|d| d.value.to_string()),
            fields: iface
                .fields
                .iter()
                .map(|f| FieldInfo {
                    name: interner.get(f.name.value),
                    type_name: format_type(&f.ty, interner),
                    description: f.description.as_ref().map(|d| d.value.to_string()),
                    arguments: vec![],
                })
                .collect(),
            implements: iface
                .implements
                .iter()
                .map(|i| interner.get(i.value))
                .collect(),
            values: vec![],
            members: vec![],
        },
        TypeDefinition::Enum(e) => TypeInfo {
            name: interner.get(e.name.value),
            kind: "ENUM".to_string(),
            description: e.description.as_ref().map(|d| d.value.to_string()),
            fields: vec![],
            implements: vec![],
            values: e
                .values
                .iter()
                .map(|v| interner.get(v.name.value))
                .collect(),
            members: vec![],
        },
        TypeDefinition::Union(u) => TypeInfo {
            name: interner.get(u.name.value),
            kind: "UNION".to_string(),
            description: u.description.as_ref().map(|d| d.value.to_string()),
            fields: vec![],
            implements: vec![],
            values: vec![],
            members: u.members.iter().map(|m| interner.get(m.value)).collect(),
        },
        TypeDefinition::Input(inp) => TypeInfo {
            name: interner.get(inp.name.value),
            kind: "INPUT_OBJECT".to_string(),
            description: inp.description.as_ref().map(|d| d.value.to_string()),
            fields: inp
                .fields
                .iter()
                .map(|f| FieldInfo {
                    name: interner.get(f.name.value),
                    type_name: format_type(&f.ty, interner),
                    description: f.description.as_ref().map(|d| d.value.to_string()),
                    arguments: vec![],
                })
                .collect(),
            implements: vec![],
            values: vec![],
            members: vec![],
        },
        TypeDefinition::Scalar(s) => TypeInfo {
            name: interner.get(s.name.value),
            kind: "SCALAR".to_string(),
            description: s.description.as_ref().map(|d| d.value.to_string()),
            fields: vec![],
            implements: vec![],
            values: vec![],
            members: vec![],
        },
        TypeDefinition::Opaque(o) => TypeInfo {
            name: interner.get(o.name.value),
            kind: "OPAQUE".to_string(),
            description: o.description.as_ref().map(|d| d.value.to_string()),
            fields: vec![],
            implements: vec![],
            values: vec![],
            members: vec![format_type(&o.underlying, interner)],
        },
        TypeDefinition::TypeAlias(a) => TypeInfo {
            name: interner.get(a.name.value),
            kind: "TYPE_ALIAS".to_string(),
            description: a.description.as_ref().map(|d| d.value.to_string()),
            fields: vec![],
            implements: vec![],
            values: vec![],
            members: vec![format_type(&a.aliased, interner)],
        },
        TypeDefinition::InputUnion(iu) => TypeInfo {
            name: interner.get(iu.name.value),
            kind: "INPUT_UNION".to_string(),
            description: iu.description.as_ref().map(|d| d.value.to_string()),
            fields: vec![],
            implements: vec![],
            values: vec![],
            members: iu.members.iter().map(|m| interner.get(m.value)).collect(),
        },
        TypeDefinition::InputEnum(ie) => TypeInfo {
            name: interner.get(ie.name.value),
            kind: "INPUT_ENUM".to_string(),
            description: ie.description.as_ref().map(|d| d.value.to_string()),
            fields: vec![],
            implements: vec![],
            values: ie
                .variants
                .iter()
                .map(|v| interner.get(v.name.value))
                .collect(),
            members: vec![],
        },
    }
}

fn format_type(ty: &Type, interner: &Interner) -> String {
    match ty {
        Type::Named(named) => interner.get(named.name),
        Type::Option(inner, _) => format!("Option<{}>", format_type(inner, interner)),
        Type::List(inner, _) => format!("List<{}>", format_type(inner, interner)),
        Type::Generic(gen) => {
            let name = interner.get(gen.name);
            let args: Vec<String> = gen
                .arguments
                .iter()
                .map(|a| format_type(a, interner))
                .collect();
            format!("{}<{}>", name, args.join(", "))
        }
        Type::Tuple(tuple) => {
            let elements: Vec<String> = tuple
                .elements
                .iter()
                .map(|e| format_type(&e.ty, interner))
                .collect();
            format!("({})", elements.join(", "))
        }
        Type::_Phantom(_) => String::new(),
    }
}

// Legacy API
#[wasm_bindgen]
pub fn parse_schema(source: &str) -> JsValue {
    let bgql = BetterGraphQL::new();
    bgql.parse(source)
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
