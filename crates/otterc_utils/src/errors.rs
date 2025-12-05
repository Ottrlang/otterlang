use ariadne::{Color, Label, Report, ReportKind, Source};
use otterc_span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Clone)]
pub struct Diagnostic {
    severity: DiagnosticSeverity,
    source_id: String,
    span: Span,
    message: String,
    label: Option<String>,
    suggestion: Option<String>,
    help: Option<String>,
}

impl Diagnostic {
    pub fn new<S: Into<String>>(
        severity: DiagnosticSeverity,
        source_id: S,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            source_id: source_id.into(),
            span,
            message: message.into(),
            label: None,
            suggestion: None,
            help: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn suggestion(&self) -> Option<&str> {
        self.suggestion.as_deref()
    }

    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    pub fn report_kind(&self) -> ReportKind<'_> {
        match self.severity {
            DiagnosticSeverity::Error => ReportKind::Error,
            DiagnosticSeverity::Warning => ReportKind::Warning,
            DiagnosticSeverity::Info | DiagnosticSeverity::Hint => ReportKind::Advice,
        }
    }

    /// Create an info diagnostic
    pub fn info(source_id: impl Into<String>, span: Span, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Info, source_id, span, message)
    }

    /// Create a hint diagnostic
    pub fn hint(source_id: impl Into<String>, span: Span, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Hint, source_id, span, message)
    }

    /// Create an error diagnostic
    pub fn error(source_id: impl Into<String>, span: Span, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Error, source_id, span, message)
    }

    /// Create a warning diagnostic
    pub fn warning(source_id: impl Into<String>, span: Span, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Warning, source_id, span, message)
    }
}

pub fn emit_diagnostics(diagnostics: &[Diagnostic], source: &str) {
    for diagnostic in diagnostics {
        let color = match diagnostic.severity {
            DiagnosticSeverity::Error => Color::Red,
            DiagnosticSeverity::Warning => Color::Yellow,
            DiagnosticSeverity::Info => Color::Blue,
            DiagnosticSeverity::Hint => Color::Cyan,
        };

        let span: std::ops::Range<usize> = diagnostic.span().into();
        let mut report = Report::build(
            diagnostic.report_kind(),
            diagnostic.source_id().to_string(),
            span.start,
        )
        .with_message(diagnostic.message());

        // Only add a label if there is specific label text, or if we want to point to the span
        // without repeating the main error message.
        if let Some(label_text) = diagnostic.label() {
            report = report.with_label(
                Label::new((diagnostic.source_id().to_string(), span.clone()))
                    .with_message(label_text)
                    .with_color(color),
            );
        } else {
            // Just highlight the span without text if no specific label is provided
            // to avoid duplicating the main message
            report = report.with_label(
                Label::new((diagnostic.source_id().to_string(), span.clone())).with_color(color),
            );
        }

        // Add suggestion if available
        if let Some(suggestion) = diagnostic.suggestion() {
            // TODO: We could use `ariadne::Config` to make this fancier, but for now
            // just improving the text format.
            report = report.with_note(format!("Suggestion: {}", suggestion));
        }

        // Add help text if available
        if let Some(help) = diagnostic.help() {
            report = report.with_note(help);
        } else {
            // Only show generic help for errors/warnings
            if matches!(
                diagnostic.severity,
                DiagnosticSeverity::Error | DiagnosticSeverity::Warning
            ) {
                report = report.with_note(
                    "For more information, re-run with --debug to inspect tokens and AST.",
                );
            }
        }

        let _ = report
            .finish()
            .print((diagnostic.source_id().to_string(), Source::from(source)));
    }
}

/// Emit a single diagnostic
pub fn emit_diagnostic(diagnostic: &Diagnostic, source: &str) {
    emit_diagnostics(std::slice::from_ref(diagnostic), source);
}
