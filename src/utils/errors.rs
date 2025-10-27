use crate::lexer::token::Span;
use ariadne::{Color, Label, Report, ReportKind, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Clone)]
pub struct Diagnostic {
    severity: DiagnosticSeverity,
    source_id: String,
    span: Span,
    message: String,
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
        }
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

    pub fn report_kind(&self) -> ReportKind<'_> {
        match self.severity {
            DiagnosticSeverity::Error => ReportKind::Error,
            DiagnosticSeverity::Warning => ReportKind::Warning,
        }
    }
}

pub fn emit_diagnostics(diagnostics: &[Diagnostic], source: &str) {
    for diagnostic in diagnostics {
        let color = match diagnostic.severity {
            DiagnosticSeverity::Error => Color::Red,
            DiagnosticSeverity::Warning => Color::Yellow,
        };

        let span: std::ops::Range<usize> = diagnostic.span().into();
        let report = Report::build(
            diagnostic.report_kind(),
            diagnostic.source_id().to_string(),
            span.start,
        )
        .with_message(diagnostic.message())
        .with_label(
            Label::new((diagnostic.source_id().to_string(), span.clone()))
                .with_message(diagnostic.message())
                .with_color(color),
        )
        .with_note("For more information, re-run with --debug to inspect tokens and AST.")
        .finish();

        let _ = report.print((diagnostic.source_id().to_string(), Source::from(source)));
    }
}
