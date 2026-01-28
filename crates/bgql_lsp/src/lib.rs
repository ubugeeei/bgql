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
use bgql_syntax::{format, parse, Definition, TypeDefinition};
use std::sync::Arc;
use symbols::{
    offset_to_position, position_to_offset, span_to_range, symbol_to_document_symbol, SymbolTable,
};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
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

        result
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
            .collect()
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
                    ]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
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
