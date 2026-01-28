//! Document state management.

use std::collections::HashMap;
use tower_lsp::lsp_types::Url;

/// State for a single document.
#[derive(Debug, Clone, Default)]
pub struct DocumentState {
    pub content: String,
    pub version: i32,
}

impl DocumentState {
    pub fn new(content: String, version: i32) -> Self {
        Self { content, version }
    }

    pub fn update(&mut self, content: String, version: i32) {
        self.content = content;
        self.version = version;
    }
}

/// Server state containing all open documents.
#[derive(Debug, Default)]
pub struct ServerState {
    pub documents: HashMap<Url, DocumentState>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn open_document(&mut self, uri: Url, content: String, version: i32) {
        self.documents
            .insert(uri, DocumentState::new(content, version));
    }

    pub fn update_document(&mut self, uri: &Url, content: String, version: i32) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.update(content, version);
        }
    }

    pub fn close_document(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn get_document(&self, uri: &Url) -> Option<&DocumentState> {
        self.documents.get(uri)
    }
}
