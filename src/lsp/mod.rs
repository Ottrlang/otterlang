use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::runtime::symbol_registry::SymbolRegistry;
use crate::typecheck::{TypeChecker, TypeError};
use common::Span;
use lexer::{tokenize, LexerError};
use parser::parse;
use utils::errors::{Diagnostic as OtterDiagnostic, DiagnosticSeverity as OtterDiagSeverity};

#[derive(Default, Debug)]
struct DocumentStore {
    documents: HashMap<Url, String>,
}

#[derive(Debug)]
pub struct Backend {
    client: Client,
    state: Arc<RwLock<DocumentStore>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(DocumentStore::default())),
        }
    }

    async fn upsert_document(&self, uri: Url, text: String) {
        {
            let mut state = self.state.write().await;
            state.documents.insert(uri.clone(), text);
        }
        self.publish_diagnostics(uri).await;
    }

    async fn remove_document(&self, uri: &Url) {
        {
            let mut state = self.state.write().await;
            state.documents.remove(uri);
        }
        let _ = self
            .client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
    }

    async fn publish_diagnostics(&self, uri: Url) {
        let text = {
            let state = self.state.read().await;
            state.documents.get(&uri).cloned()
        };

        if let Some(text) = text {
            let diagnostics = compute_lsp_diagnostics(&text);
            let _ = self
                .client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn document_text(&self, uri: &Url) -> Option<String> {
        let state = self.state.read().await;
        state.documents.get(uri).cloned()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "otterlang-lsp initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.upsert_document(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.upsert_document(params.text_document.uri, change.text)
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.remove_document(&params.text_document.uri).await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let text = self.document_text(&uri).await.unwrap_or_default();

        let mut items = vec![
            CompletionItem::new_simple("print".into(), "fn print(message: string)".into()),
            CompletionItem::new_simple("len".into(), "fn len(value)".into()),
            CompletionItem::new_simple("await".into(), "await expression".into()),
        ];

        items.extend(
            collect_identifiers(&text)
                .into_iter()
                .map(|ident| CompletionItem::new_simple(ident, "identifier".into())),
        );

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        if let Some(text) = self.document_text(&uri).await {
            if let Some(word) = word_at_position(&text, position) {
                let contents = HoverContents::Scalar(MarkedString::String(format!(
                    "symbol `{}` ({} chars)",
                    word,
                    word.len()
                )));
                return Ok(Some(Hover {
                    contents,
                    range: None,
                }));
            }
        }

        Ok(None)
    }
}

/// Run a standard I/O LSP server using the backend above.
pub async fn run_stdio_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn compute_lsp_diagnostics(text: &str) -> Vec<Diagnostic> {
    let source_id = "lsp";
    match tokenize(text) {
        Ok(tokens) => match parse(&tokens) {
            Ok(program) => {
                let mut checker = TypeChecker::new().with_registry(SymbolRegistry::global());
                if checker.check_program(&program).is_err() {
                    checker.errors().iter().map(type_error_to_lsp).collect()
                } else {
                    Vec::new()
                }
            }
            Err(errors) => errors
                .into_iter()
                .map(|err| otter_diag_to_lsp(&err.to_diagnostic(source_id), text))
                .collect(),
        },
        Err(errors) => errors
            .into_iter()
            .map(|err| otter_diag_to_lsp(&lexer_error_to_diag(source_id, &err), text))
            .collect(),
    }
}

fn word_at_position(text: &str, position: Position) -> Option<String> {
    let line = text.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let mut idx = position.character as isize;
    if idx as usize >= chars.len() {
        idx = chars.len() as isize - 1;
    }
    while idx >= 0 && !chars[idx as usize].is_alphanumeric() && chars[idx as usize] != '_' {
        idx -= 1;
    }
    if idx < 0 {
        return None;
    }
    let start = {
        let mut s = idx as usize;
        while s > 0 && (chars[s - 1].is_alphanumeric() || chars[s - 1] == '_') {
            s -= 1;
        }
        s
    };
    let mut end = idx as usize;
    while end + 1 < chars.len() && (chars[end + 1].is_alphanumeric() || chars[end + 1] == '_') {
        end += 1;
    }
    Some(chars[start..=end].iter().collect())
}

fn collect_identifiers(text: &str) -> Vec<String> {
    let mut set = BTreeSet::new();
    for token in text.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if token.len() > 1 && token.chars().next().map_or(false, |c| c.is_alphabetic()) {
            set.insert(token.to_string());
        }
    }
    set.into_iter().collect()
}

fn lexer_error_to_diag(source: &str, err: &LexerError) -> OtterDiagnostic {
    err.to_diagnostic(source)
}

fn type_error_to_lsp(err: &TypeError) -> Diagnostic {
    let mut message = err.message.clone();
    if let Some(hint) = &err.hint {
        message.push_str(&format!("\nHint: {}", hint));
    }
    if let Some(help) = &err.help {
        message.push_str(&format!("\nHelp: {}", help));
    }

    Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("otterlang".into()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

fn otter_diag_to_lsp(diag: &OtterDiagnostic, text: &str) -> Diagnostic {
    let range = span_to_range(diag.span(), text);
    let mut message = diag.message().to_string();
    if let Some(suggestion) = diag.suggestion() {
        message.push_str(&format!("\nSuggestion: {}", suggestion));
    }
    if let Some(help) = diag.help() {
        message.push_str(&format!("\nHelp: {}", help));
    }

    Diagnostic {
        range,
        severity: Some(match diag.severity() {
            OtterDiagSeverity::Error => DiagnosticSeverity::ERROR,
            OtterDiagSeverity::Warning => DiagnosticSeverity::WARNING,
            OtterDiagSeverity::Info => DiagnosticSeverity::INFORMATION,
            OtterDiagSeverity::Hint => DiagnosticSeverity::HINT,
        }),
        code: None,
        code_description: None,
        source: Some("otterlang".into()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

fn span_to_range(span: Span, text: &str) -> Range {
    Range {
        start: offset_to_position(text, span.start()),
        end: offset_to_position(text, span.end()),
    }
}

fn offset_to_position(text: &str, offset: usize) -> Position {
    let mut counted = 0usize;
    let mut line = 0u32;
    let mut character = 0u32;
    for ch in text.chars() {
        if counted >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
        counted += ch.len_utf8();
    }
    Position { line, character }
}
