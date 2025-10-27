use chumsky::prelude::*;
use chumsky::Stream;

use crate::ast::{BinaryOp, Expr, Function, Literal, Program, Statement};
use crate::lexer::token::{Span, Token, TokenKind};
use crate::utils::errors::{Diagnostic, DiagnosticSeverity};

#[derive(Debug, Clone)]
pub struct ParserError {
    pub message: String,
    pub span: Span,
}

impl ParserError {
    pub fn to_diagnostic(&self, source_id: &str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticSeverity::Error,
            source_id,
            self.span,
            self.message.clone(),
        )
    }
}

impl From<Simple<TokenKind>> for ParserError {
    fn from(value: Simple<TokenKind>) -> Self {
        let span_range = value.span();
        let span = Span::new(span_range.start, span_range.end);
        let message = if let Some(found) = value.found() {
            format!("unexpected token: {:?}", found)
        } else {
            "unexpected end of input".to_string()
        };
        Self { message, span }
    }
}

pub fn parse(tokens: &[Token]) -> Result<Program, Vec<ParserError>> {
    let parser = program_parser();
    let eof_span = tokens
        .last()
        .map(|token| token.span)
        .unwrap_or_else(|| Span::new(0, 0));

    let end = eof_span.end();
    let stream = Stream::from_iter(
        end..end + 1,
        tokens
            .iter()
            .cloned()
            .map(|token| (token.kind, token.span.into())),
    );

    parser
        .parse(stream)
        .map_err(|errors| errors.into_iter().map(ParserError::from).collect())
}

fn identifier_parser() -> impl Parser<TokenKind, String, Error = Simple<TokenKind>> {
    select! { TokenKind::Identifier(name) => name }
}

fn literal_expr_parser() -> impl Parser<TokenKind, Expr, Error = Simple<TokenKind>> {
    let string_lit =
        select! { TokenKind::StringLiteral(value) => Expr::Literal(Literal::String(value)) };
    let number_lit = select! { TokenKind::Number(value) => Expr::Literal(Literal::Number(value.parse().unwrap_or_default())) };
    choice((string_lit, number_lit))
}

fn expr_parser() -> impl Parser<TokenKind, Expr, Error = Simple<TokenKind>> {
    recursive(|expr| {
        let atom = choice((
            literal_expr_parser(),
            identifier_parser().map(Expr::Identifier),
            expr.clone()
                .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen)),
        ))
        .boxed();

        let call_suffix = just(TokenKind::LParen)
            .ignore_then(
                expr.clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing()
                    .or_not()
                    .map(|args| args.unwrap_or_default()),
            )
            .then_ignore(just(TokenKind::RParen));

        let call = atom
            .clone()
            .then(call_suffix.repeated())
            .foldl(|callee, args| Expr::Call {
                callee: Box::new(callee),
                args,
            })
            .boxed();

        let product = call
            .clone()
            .then(
                choice((
                    just(TokenKind::Star).to(BinaryOp::Mul),
                    just(TokenKind::Slash).to(BinaryOp::Div),
                ))
                .then(call.clone())
                .repeated(),
            )
            .foldl(|left, (op, right)| Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });

        let sum = product
            .clone()
            .then(
                choice((
                    just(TokenKind::Plus).to(BinaryOp::Add),
                    just(TokenKind::Minus).to(BinaryOp::Sub),
                ))
                .then(product)
                .repeated(),
            )
            .foldl(|left, (op, right)| Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });

        sum
    })
}

fn program_parser() -> impl Parser<TokenKind, Program, Error = Simple<TokenKind>> {
    let newline = just(TokenKind::Newline).repeated().at_least(1);
    let expr = expr_parser().boxed();

    let print_stmt = just(TokenKind::Print)
        .ignore_then(
            expr.clone()
                .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen)),
        )
        .map(Statement::Print);

    let return_stmt = just(TokenKind::Return)
        .ignore_then(expr.clone().or_not())
        .map(Statement::Return);

    let assignment_stmt = identifier_parser()
        .then_ignore(just(TokenKind::Equals))
        .then(expr.clone())
        .map(|(name, expr)| Statement::Assignment { name, expr });

    let statement = choice((print_stmt, return_stmt, assignment_stmt))
        .then_ignore(newline.clone())
        .boxed();

    let block = statement
        .clone()
        .repeated()
        .at_least(1)
        .delimited_by(just(TokenKind::Indent), just(TokenKind::Dedent));

    let function = just(TokenKind::Fn)
        .ignore_then(identifier_parser())
        .then_ignore(just(TokenKind::Colon))
        .then_ignore(newline.clone())
        .then(block)
        .map(|(name, body)| Function::new(name, body))
        .then_ignore(newline.clone().or_not());

    newline
        .clone()
        .or_not()
        .ignore_then(function.repeated())
        .then_ignore(newline.repeated().or_not())
        .then_ignore(just(TokenKind::Eof))
        .map(Program::new)
}
