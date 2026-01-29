//! Language Server Protocol implementation for Better GraphQL.
//!
//! This crate provides:
//! - LSP server implementation using tower-lsp
//! - Diagnostics publishing
//! - Completion
//! - Hover information
//! - Go to definition
//! - Find references
//! - Document symbols
//! - Formatting
//! - Rename

mod completion;
mod hover;
mod state;
mod symbols;

use async_trait::async_trait;
use bgql_core::Interner;
use bgql_semantic::{checker, hir::HirDatabase, types::TypeRegistry};
use bgql_syntax::{format, parse, Definition, TypeDefinition};
use std::sync::Arc;
use symbols::{
    offset_to_position, position_to_offset, span_to_range, symbol_to_document_symbol, SymbolTable,
};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
#[allow(unused_imports)]
use tracing::info;

use crate::state::ServerState;

/// The Better GraphQL language server.
pub struct BgqlLanguageServer {
    client: Client,
    state: Arc<RwLock<ServerState>>,
}

impl BgqlLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(ServerState::new())),
        }
    }

    async fn validate(&self, uri: &Url) {
        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return;
        };

        let diagnostics = self.get_diagnostics(&content);

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn get_diagnostics(&self, content: &str) -> Vec<Diagnostic> {
        let interner = Interner::new();
        let result = parse(content, &interner);

        // Collect parser diagnostics
        let mut diagnostics: Vec<Diagnostic> = result
            .diagnostics
            .iter()
            .filter_map(|diag| {
                let span = diag.primary_span()?;
                let start = offset_to_position(content, span.start as usize);
                let end = offset_to_position(content, span.end as usize);

                Some(Diagnostic {
                    range: Range { start, end },
                    severity: Some(match diag.severity {
                        bgql_core::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
                        bgql_core::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
                        bgql_core::DiagnosticSeverity::Info => DiagnosticSeverity::INFORMATION,
                        bgql_core::DiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
                    }),
                    message: diag.title.clone(),
                    source: Some("bgql".to_string()),
                    ..Default::default()
                })
            })
            .collect();

        // Run type checker if no parser errors
        if !result.diagnostics.has_errors() {
            let types = TypeRegistry::new();
            let hir = HirDatabase::new();
            let check_result = checker::check(&result.document, &types, &hir, &interner);

            for diag in check_result.diagnostics.iter() {
                if let Some(span) = diag.primary_span() {
                    let start = offset_to_position(content, span.start as usize);
                    let end = offset_to_position(content, span.end as usize);

                    diagnostics.push(Diagnostic {
                        range: Range { start, end },
                        severity: Some(match diag.severity {
                            bgql_core::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
                            bgql_core::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
                            bgql_core::DiagnosticSeverity::Info => DiagnosticSeverity::INFORMATION,
                            bgql_core::DiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
                        }),
                        message: diag.title.clone(),
                        source: Some("bgql".to_string()),
                        ..Default::default()
                    });
                }
            }
        }

        diagnostics
    }

    fn find_definition_location(
        &self,
        content: &str,
        position: Position,
        interner: &Interner,
        document: &bgql_syntax::Document<'_>,
    ) -> Option<Location> {
        let offset = position_to_offset(content, position);
        let word = get_word_at_offset(content, offset)?;

        for def in &document.definitions {
            if let Definition::Type(type_def) = def {
                let (name, span) = match type_def {
                    TypeDefinition::Object(obj) => (interner.get(obj.name.value), obj.span),
                    TypeDefinition::Interface(iface) => {
                        (interner.get(iface.name.value), iface.span)
                    }
                    TypeDefinition::Enum(e) => (interner.get(e.name.value), e.span),
                    TypeDefinition::Union(u) => (interner.get(u.name.value), u.span),
                    TypeDefinition::Input(inp) => (interner.get(inp.name.value), inp.span),
                    TypeDefinition::Scalar(s) => (interner.get(s.name.value), s.span),
                    TypeDefinition::Opaque(o) => (interner.get(o.name.value), o.span),
                    TypeDefinition::TypeAlias(a) => (interner.get(a.name.value), a.span),
                    TypeDefinition::InputUnion(iu) => (interner.get(iu.name.value), iu.span),
                    TypeDefinition::InputEnum(ie) => (interner.get(ie.name.value), ie.span),
                };

                if name == word {
                    return Some(Location {
                        uri: Url::parse("file:///").ok()?,
                        range: span_to_range(span, content),
                    });
                }
            }
        }

        None
    }
}

fn get_word_at_offset(content: &str, offset: usize) -> Option<String> {
    let bytes = content.as_bytes();

    let mut start = offset;
    while start > 0 && is_identifier_char(bytes[start - 1]) {
        start -= 1;
    }

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

const fn is_identifier_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

#[async_trait]
impl LanguageServer for BgqlLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        info!("Better GraphQL Language Server initializing");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "@".to_string(),
                        "{".to_string(),
                        ":".to_string(),
                        "(".to_string(),
                        "<".to_string(),
                        " ".to_string(),
                        ",".to_string(),
                    ]),
                    resolve_provider: Some(true),
                    ..Default::default()
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![",".to_string()]),
                    work_done_progress_options: Default::default(),
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::CLASS,
                                    SemanticTokenType::ENUM,
                                    SemanticTokenType::INTERFACE,
                                    SemanticTokenType::STRUCT,
                                    SemanticTokenType::TYPE_PARAMETER,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::PROPERTY,
                                    SemanticTokenType::ENUM_MEMBER,
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::METHOD,
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::COMMENT,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::OPERATOR,
                                    SemanticTokenType::DECORATOR,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DECLARATION,
                                    SemanticTokenModifier::DEFINITION,
                                    SemanticTokenModifier::DEPRECATED,
                                    SemanticTokenModifier::READONLY,
                                ],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            ..Default::default()
                        },
                    ),
                ),
                inlay_hint_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "bgql-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("Better GraphQL Language Server initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Better GraphQL Language Server shutting down");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        {
            let mut state = self.state.write().await;
            state.open_document(
                uri.clone(),
                params.text_document.text,
                params.text_document.version,
            );
        }

        self.validate(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        {
            let mut state = self.state.write().await;
            for change in params.content_changes {
                state.update_document(&uri, change.text, params.text_document.version);
            }
        }

        self.validate(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut state = self.state.write().await;
        state.close_document(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let interner = Interner::new();
        let result = parse(&content, &interner);

        let completions =
            completion::get_completions(&content, position, &result.document, &interner);
        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let interner = Interner::new();
        let result = parse(&content, &interner);

        Ok(hover::get_hover(
            &content,
            position.line,
            position.character,
            &result.document,
            &interner,
        ))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let interner = Interner::new();
        let result = parse(&content, &interner);

        if let Some(mut location) =
            self.find_definition_location(&content, position, &interner, &result.document)
        {
            location.uri = uri.clone();
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let offset = position_to_offset(&content, position);
        let word = match get_word_at_offset(&content, offset) {
            Some(w) => w,
            None => return Ok(None),
        };

        let mut locations = Vec::new();
        let mut search_offset = 0;

        while let Some(pos) = content[search_offset..].find(&word) {
            let abs_pos = search_offset + pos;

            let before_ok = abs_pos == 0 || !is_identifier_char(content.as_bytes()[abs_pos - 1]);
            let after_ok = abs_pos + word.len() >= content.len()
                || !is_identifier_char(content.as_bytes()[abs_pos + word.len()]);

            if before_ok && after_ok {
                let start = offset_to_position(&content, abs_pos);
                let end = offset_to_position(&content, abs_pos + word.len());
                locations.push(Location {
                    uri: uri.clone(),
                    range: Range { start, end },
                });
            }

            search_offset = abs_pos + word.len();
        }

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let interner = Interner::new();
        let result = parse(&content, &interner);

        let symbol_table = SymbolTable::from_document(&result.document, &interner);
        let symbols: Vec<DocumentSymbol> = symbol_table
            .root_symbols
            .iter()
            .map(|s| symbol_to_document_symbol(s, &content))
            .collect();

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let interner = Interner::new();
        let result = parse(&content, &interner);

        if result.diagnostics.has_errors() {
            return Ok(None);
        }

        let formatted = format(&result.document, &interner);

        let lines: Vec<_> = content.lines().collect();
        let end_line = lines.len().saturating_sub(1) as u32;
        let end_char = lines.last().map(|l| l.len() as u32).unwrap_or(0);

        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(end_line, end_char),
            },
            new_text: formatted,
        }]))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let offset = position_to_offset(&content, position);
        let word = match get_word_at_offset(&content, offset) {
            Some(w) => w,
            None => return Ok(None),
        };

        let mut edits = Vec::new();
        let mut search_offset = 0;

        while let Some(pos) = content[search_offset..].find(&word) {
            let abs_pos = search_offset + pos;

            let before_ok = abs_pos == 0 || !is_identifier_char(content.as_bytes()[abs_pos - 1]);
            let after_ok = abs_pos + word.len() >= content.len()
                || !is_identifier_char(content.as_bytes()[abs_pos + word.len()]);

            if before_ok && after_ok {
                let start = offset_to_position(&content, abs_pos);
                let end = offset_to_position(&content, abs_pos + word.len());
                edits.push(TextEdit {
                    range: Range { start, end },
                    new_text: new_name.clone(),
                });
            }

            search_offset = abs_pos + word.len();
        }

        if edits.is_empty() {
            return Ok(None);
        }

        let mut changes = std::collections::HashMap::new();
        changes.insert(uri.clone(), edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let offset = position_to_offset(&content, position);
        let signatures = get_signature_help(&content, offset);

        if signatures.is_empty() {
            return Ok(None);
        }

        Ok(Some(SignatureHelp {
            signatures,
            active_signature: Some(0),
            active_parameter: None,
        }))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let interner = Interner::new();
        let result = parse(&content, &interner);

        let tokens = compute_semantic_tokens(&result.document, &content, &interner);

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let interner = Interner::new();
        let result = parse(&content, &interner);

        let hints = compute_inlay_hints(&result.document, &content, &interner);

        Ok(Some(hints))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;

        let content = {
            let state = self.state.read().await;
            state.get_document(uri).map(|d| d.content.clone())
        };

        let Some(content) = content else {
            return Ok(None);
        };

        let mut actions = Vec::new();

        // Generate quick fixes for diagnostics
        for diag in &params.context.diagnostics {
            if let Some(action) = generate_quick_fix(&content, diag, uri) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }
}

// =============================================================================
// Signature Help
// =============================================================================

fn get_signature_help(content: &str, offset: usize) -> Vec<SignatureInformation> {
    let before = &content[..offset.min(content.len())];

    // Find the directive or field call context
    if let Some(at_pos) = before.rfind('@') {
        let directive_text = &before[at_pos + 1..];
        // Find directive name
        let name_end = directive_text
            .find(|c: char| !c.is_alphanumeric())
            .unwrap_or(directive_text.len());
        let directive_name = &directive_text[..name_end];

        return get_directive_signatures(directive_name);
    }

    Vec::new()
}

fn get_directive_signatures(name: &str) -> Vec<SignatureInformation> {
    match name {
        "deprecated" => vec![SignatureInformation {
            label: "@deprecated(reason: String)".to_string(),
            documentation: Some(Documentation::String(
                "Mark a field or type as deprecated".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("reason: String".to_string()),
                documentation: Some(Documentation::String(
                    "Explanation of why it's deprecated".to_string(),
                )),
            }]),
            active_parameter: None,
        }],
        "minLength" => vec![SignatureInformation {
            label: "@minLength(value: Int)".to_string(),
            documentation: Some(Documentation::String(
                "Validate minimum string length".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("value: Int".to_string()),
                documentation: Some(Documentation::String("Minimum length".to_string())),
            }]),
            active_parameter: None,
        }],
        "maxLength" => vec![SignatureInformation {
            label: "@maxLength(value: Int)".to_string(),
            documentation: Some(Documentation::String(
                "Validate maximum string length".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("value: Int".to_string()),
                documentation: Some(Documentation::String("Maximum length".to_string())),
            }]),
            active_parameter: None,
        }],
        "min" => vec![SignatureInformation {
            label: "@min(value: Int | Float)".to_string(),
            documentation: Some(Documentation::String(
                "Validate minimum numeric value".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("value: Int | Float".to_string()),
                documentation: Some(Documentation::String("Minimum value".to_string())),
            }]),
            active_parameter: None,
        }],
        "max" => vec![SignatureInformation {
            label: "@max(value: Int | Float)".to_string(),
            documentation: Some(Documentation::String(
                "Validate maximum numeric value".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("value: Int | Float".to_string()),
                documentation: Some(Documentation::String("Maximum value".to_string())),
            }]),
            active_parameter: None,
        }],
        "pattern" => vec![SignatureInformation {
            label: "@pattern(regex: String)".to_string(),
            documentation: Some(Documentation::String(
                "Validate against a regular expression".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("regex: String".to_string()),
                documentation: Some(Documentation::String("Regular expression pattern".to_string())),
            }]),
            active_parameter: None,
        }],
        "hasRole" => vec![SignatureInformation {
            label: "@hasRole(role: Role)".to_string(),
            documentation: Some(Documentation::String(
                "Require a specific user role".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("role: Role".to_string()),
                documentation: Some(Documentation::String("Required role".to_string())),
            }]),
            active_parameter: None,
        }],
        "cacheControl" => vec![SignatureInformation {
            label: "@cacheControl(maxAge: Int, scope: CacheScope = PUBLIC)".to_string(),
            documentation: Some(Documentation::String("Set cache control hints".to_string())),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("maxAge: Int".to_string()),
                    documentation: Some(Documentation::String(
                        "Cache duration in seconds".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("scope: CacheScope = PUBLIC".to_string()),
                    documentation: Some(Documentation::String(
                        "Cache scope (PUBLIC or PRIVATE)".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }],
        "defer" => vec![SignatureInformation {
            label: "@defer(label: String, if: Boolean = true)".to_string(),
            documentation: Some(Documentation::String(
                "Defer field resolution for incremental delivery".to_string(),
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("label: String".to_string()),
                    documentation: Some(Documentation::String(
                        "Label for identifying the deferred fragment".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("if: Boolean = true".to_string()),
                    documentation: Some(Documentation::String(
                        "Condition to enable deferring".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }],
        "stream" => vec![SignatureInformation {
            label: "@stream(initialCount: Int = 0, label: String, if: Boolean = true)".to_string(),
            documentation: Some(Documentation::String(
                "Stream list items incrementally".to_string(),
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("initialCount: Int = 0".to_string()),
                    documentation: Some(Documentation::String(
                        "Number of items to include in initial response".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("label: String".to_string()),
                    documentation: Some(Documentation::String(
                        "Label for identifying the stream".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("if: Boolean = true".to_string()),
                    documentation: Some(Documentation::String(
                        "Condition to enable streaming".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }],
        "rateLimit" => vec![SignatureInformation {
            label: "@rateLimit(requests: Int, window: String)".to_string(),
            documentation: Some(Documentation::String("Apply rate limiting".to_string())),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("requests: Int".to_string()),
                    documentation: Some(Documentation::String(
                        "Maximum requests allowed".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("window: String".to_string()),
                    documentation: Some(Documentation::String(
                        "Time window (e.g., \"1h\", \"1m\")".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }],
        _ => Vec::new(),
    }
}

// =============================================================================
// Semantic Tokens
// =============================================================================

fn compute_semantic_tokens(
    document: &bgql_syntax::Document<'_>,
    content: &str,
    interner: &Interner,
) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for def in &document.definitions {
        match def {
            Definition::Type(type_def) => {
                let (span, _name) = match type_def {
                    TypeDefinition::Object(obj) => (obj.name.span, interner.get(obj.name.value)),
                    TypeDefinition::Interface(iface) => {
                        (iface.name.span, interner.get(iface.name.value))
                    }
                    TypeDefinition::Enum(e) => (e.name.span, interner.get(e.name.value)),
                    TypeDefinition::Union(u) => (u.name.span, interner.get(u.name.value)),
                    TypeDefinition::Input(i) => (i.name.span, interner.get(i.name.value)),
                    TypeDefinition::Scalar(s) => (s.name.span, interner.get(s.name.value)),
                    TypeDefinition::Opaque(o) => (o.name.span, interner.get(o.name.value)),
                    TypeDefinition::TypeAlias(a) => (a.name.span, interner.get(a.name.value)),
                    TypeDefinition::InputUnion(iu) => (iu.name.span, interner.get(iu.name.value)),
                    TypeDefinition::InputEnum(ie) => (ie.name.span, interner.get(ie.name.value)),
                };

                let pos = offset_to_position(content, span.start as usize);
                let length = (span.end - span.start) as u32;

                let delta_line = pos.line - prev_line;
                let delta_start = if delta_line == 0 {
                    pos.character - prev_start
                } else {
                    pos.character
                };

                let token_type = match type_def {
                    TypeDefinition::Interface(_) => 3, // INTERFACE
                    TypeDefinition::Enum(_) => 2,     // ENUM
                    TypeDefinition::Input(_) => 4,    // STRUCT
                    _ => 1,                           // CLASS
                };

                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length,
                    token_type,
                    token_modifiers_bitset: 1, // DECLARATION
                });

                prev_line = pos.line;
                prev_start = pos.character;
            }
            _ => {}
        }
    }

    tokens
}

// =============================================================================
// Inlay Hints
// =============================================================================

fn compute_inlay_hints(
    document: &bgql_syntax::Document<'_>,
    content: &str,
    interner: &Interner,
) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    for def in &document.definitions {
        if let Definition::Type(TypeDefinition::Object(obj)) = def {
            // Show interface count
            if !obj.implements.is_empty() {
                let iface_names: Vec<_> = obj
                    .implements
                    .iter()
                    .map(|i| interner.get(i.value))
                    .collect();
                let hint_text = format!(" impl {}", iface_names.len());
                let pos = offset_to_position(content, obj.name.span.end as usize);

                hints.push(InlayHint {
                    position: pos,
                    label: InlayHintLabel::String(hint_text),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String(format!(
                        "Implements: {}",
                        iface_names.join(", ")
                    ))),
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                });
            }

            // Show field count
            if obj.fields.len() > 3 {
                let hint_text = format!(" {} fields", obj.fields.len());
                // Position after the opening brace - find it
                let obj_text = &content[obj.span.start as usize..obj.span.end as usize];
                if let Some(brace_pos) = obj_text.find('{') {
                    let abs_pos = obj.span.start as usize + brace_pos + 1;
                    let pos = offset_to_position(content, abs_pos);
                    hints.push(InlayHint {
                        position: pos,
                        label: InlayHintLabel::String(hint_text),
                        kind: Some(InlayHintKind::TYPE),
                        text_edits: None,
                        tooltip: None,
                        padding_left: Some(true),
                        padding_right: None,
                        data: None,
                    });
                }
            }
        }
    }

    hints
}

// =============================================================================
// Code Actions (Quick Fixes)
// =============================================================================

fn generate_quick_fix(
    content: &str,
    diagnostic: &Diagnostic,
    uri: &Url,
) -> Option<CodeAction> {
    // Check for "Undefined type" errors - suggest adding the type
    if diagnostic.message.contains("Undefined type") {
        // Extract the type name from the message
        if let Some(start) = diagnostic.message.find('`') {
            if let Some(end) = diagnostic.message[start + 1..].find('`') {
                let type_name = &diagnostic.message[start + 1..start + 1 + end];

                // Create a quick fix to add the type definition
                let insert_text = format!("\ntype {} {{\n  \n}}\n", type_name);

                // Find the end of the document
                let lines: Vec<_> = content.lines().collect();
                let end_line = lines.len() as u32;

                let mut changes = std::collections::HashMap::new();
                changes.insert(
                    uri.clone(),
                    vec![TextEdit {
                        range: Range {
                            start: Position::new(end_line, 0),
                            end: Position::new(end_line, 0),
                        },
                        new_text: insert_text,
                    }],
                );

                return Some(CodeAction {
                    title: format!("Create type `{}`", type_name),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    }),
                    is_preferred: Some(true),
                    ..Default::default()
                });
            }
        }
    }

    // Check for "Missing field" errors - suggest adding the field
    if diagnostic.message.contains("Missing field") {
        if let Some(start) = diagnostic.message.find('`') {
            if let Some(end) = diagnostic.message[start + 1..].find('`') {
                let field_name = &diagnostic.message[start + 1..start + 1 + end];

                return Some(CodeAction {
                    title: format!("Add field `{}`", field_name),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    // Note: actual edit would need more context about the interface
                    ..Default::default()
                });
            }
        }
    }

    None
}

/// Runs the language server.
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(BgqlLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_word_at_offset() {
        let content = "type User { id: ID }";
        assert_eq!(get_word_at_offset(content, 5), Some("User".to_string()));
        assert_eq!(get_word_at_offset(content, 12), Some("id".to_string()));
        assert_eq!(get_word_at_offset(content, 16), Some("ID".to_string()));
    }

    #[test]
    fn test_is_identifier_char() {
        assert!(is_identifier_char(b'a'));
        assert!(is_identifier_char(b'Z'));
        assert!(is_identifier_char(b'0'));
        assert!(is_identifier_char(b'_'));
        assert!(!is_identifier_char(b' '));
        assert!(!is_identifier_char(b':'));
    }
}
