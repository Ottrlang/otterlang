use chumsky::prelude::*;
use chumsky::Stream;

use crate::ast::nodes::{
    BinaryOp, Block, Expr, FStringPart, Function, Literal, Param, Program, Statement, Type, UnaryOp,
};
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

fn identifier_or_keyword_parser() -> impl Parser<TokenKind, String, Error = Simple<TokenKind>> {
    select! {
        TokenKind::Identifier(name) => name,
        TokenKind::Fn => "fn".to_string(),
        TokenKind::Let => "let".to_string(),
        TokenKind::Return => "return".to_string(),
        TokenKind::If => "if".to_string(),
        TokenKind::Else => "else".to_string(),
        TokenKind::Elif => "elif".to_string(),
        TokenKind::For => "for".to_string(),
        TokenKind::While => "while".to_string(),
        TokenKind::Break => "break".to_string(),
        TokenKind::Continue => "continue".to_string(),
        TokenKind::In => "in".to_string(),
        TokenKind::Use => "use".to_string(),
        TokenKind::From => "from".to_string(),
        TokenKind::As => "as".to_string(),
        TokenKind::Async => "async".to_string(),
        TokenKind::Await => "await".to_string(),
        TokenKind::Spawn => "spawn".to_string(),
        TokenKind::Match => "match".to_string(),
        TokenKind::Case => "case".to_string(),
        TokenKind::True => "true".to_string(),
        TokenKind::False => "false".to_string(),
        TokenKind::Print => "print".to_string(),
    }
}

fn type_parser() -> impl Parser<TokenKind, Type, Error = Simple<TokenKind>> {
    recursive(|ty| {
        identifier_parser()
            .then(
                ty.separated_by(just(TokenKind::Comma))
                    .allow_trailing()
                    .delimited_by(just(TokenKind::Lt), just(TokenKind::Gt))
                    .or_not()
            )
            .map(|(base, args)| match args {
                Some(args) => Type::Generic { base, args },
                None => Type::Simple(base),
            })
    })
}

fn parse_fstring(content: String) -> Expr {
    use chumsky::Parser;

    // Parse f-string by splitting on braces and parsing expressions
    let mut parts = Vec::new();
    let mut current_text = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                if let Some(&'{') = chars.peek() {
                    // Escaped {{
                    chars.next();
                    current_text.push('{');
                } else {
                    // Expression start
                    if !current_text.is_empty() {
                        parts.push(FStringPart::Text(current_text));
                        current_text = String::new();
                    }

                    // Parse expression until }
                    let mut expr_content = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == '}' {
                            break;
                        }
                        expr_content.push(ch);
                    }

                    if !expr_content.is_empty() {
                        // Parse the expression content using the full expression parser
                        let trimmed = expr_content.trim();
                        if !trimmed.is_empty() {
                            match crate::lexer::tokenize(trimmed) {
                                Ok(tokens) => {
                                    // Create a stream from tokens for the parser
                                    use chumsky::Stream;
                                    let end_span = tokens.last().map(|t| t.span.end()).unwrap_or(0);
                                    let stream = Stream::from_iter(
                                        end_span..end_span + 1,
                                        tokens
                                            .iter()
                                            .map(|token| (token.kind.clone(), token.span.into())),
                                    );
                                    match expr_parser().parse(stream) {
                                        Ok(expr) => {
                                            parts.push(FStringPart::Expr(Box::new(expr)));
                                        }
                                        Err(_) => {
                                            // Fallback to simple identifier if parsing fails
                                            parts.push(FStringPart::Expr(Box::new(
                                                Expr::Identifier(trimmed.to_string()),
                                            )));
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Fallback to simple identifier if tokenization fails
                                    parts.push(FStringPart::Expr(Box::new(Expr::Identifier(
                                        trimmed.to_string(),
                                    ))));
                                }
                            }
                        }
                    }
                }
            }
            '}' => {
                if let Some(&'}') = chars.peek() {
                    // Escaped }}
                    chars.next();
                    current_text.push('}');
                } else {
                    current_text.push('}');
                }
            }
            _ => current_text.push(ch),
        }
    }

    // Add remaining text
    if !current_text.is_empty() {
        parts.push(FStringPart::Text(current_text));
    }

    // If no expressions found, treat as regular string
    if parts
        .iter()
        .all(|part| matches!(part, FStringPart::Text(_)))
    {
        if let Some(FStringPart::Text(text)) = parts.first() {
            return Expr::Literal(Literal::String(text.clone()));
        }
    }

    Expr::FString { parts }
}

fn literal_expr_parser() -> impl Parser<TokenKind, Expr, Error = Simple<TokenKind>> {
    let string_lit =
        select! { TokenKind::StringLiteral(value) => Expr::Literal(Literal::String(value)) };
    let number_lit = select! { TokenKind::Number(value) => {
        // Remove underscores from the number
        let clean_value = value.replace('_', "");
        // Check if it contains a decimal point or is an integer
        if clean_value.contains('.') {
            Expr::Literal(Literal::Number(clean_value.parse().unwrap_or_default()))
        } else {
            // Parse as integer
            match clean_value.parse::<i64>() {
                Ok(int_val) => Expr::Literal(Literal::Number(int_val as f64)), // Store as float for now
                Err(_) => Expr::Literal(Literal::Number(0.0)), // fallback
            }
        }
    }};
    let bool_lit = select! {
        TokenKind::True => Expr::Literal(Literal::Bool(true)),
        TokenKind::False => Expr::Literal(Literal::Bool(false)),
    };
    let fstring_lit = select! { TokenKind::FString(content) => parse_fstring(content) };
    choice((fstring_lit, string_lit, number_lit, bool_lit))
}

fn expr_parser() -> impl Parser<TokenKind, Expr, Error = Simple<TokenKind>> {
    recursive(|expr| {
        let lambda_param = identifier_parser()
            .then(
                just(TokenKind::Colon)
                    .ignore_then(type_parser())
                    .or_not(),
            )
            .map(|(name, ty)| Param::new(name, ty));

        let lambda_params = lambda_param
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
            .or_not()
            .map(|params| params.unwrap_or_default());

        let lambda_ret_type = just(TokenKind::Arrow)
            .ignore_then(type_parser())
            .or_not();

        let lambda_block = recursive(|_block| {
            let lambda_stmt = recursive(|_stmt| {
                // Simplified statement parser for lambdas - just expressions and returns
                let lambda_return_stmt = just(TokenKind::Return)
                    .ignore_then(expr.clone().or_not())
                    .map(Statement::Return);

                choice((
                    lambda_return_stmt,
                    expr.clone().map(Statement::Expr),
                ))
                .then_ignore(just(TokenKind::Newline).or_not())
                .boxed()
            });

            lambda_stmt
                .repeated()
                .at_least(1)
                .map(Block::new)
        });

        let lambda_expr = just(TokenKind::Fn)
            .ignore_then(lambda_params)
            .then(lambda_ret_type)
            .then_ignore(just(TokenKind::Colon))
            .then(
                just(TokenKind::Newline)
                    .ignore_then(lambda_block)
                    .or(expr.clone().map(|expr| Block::new(vec![Statement::Expr(expr)])))
            )
            .map(|((params, ret_ty), body)| Expr::Lambda {
                params,
                ret_ty,
                body,
            });

        let atom = choice((
            literal_expr_parser(),
            lambda_expr,
            identifier_parser().map(Expr::Identifier),
            expr.clone()
                .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen)),
        ))
        .boxed();

        let member_access = atom
            .clone()
            .then(
                just(TokenKind::Dot)
                    .ignore_then(identifier_or_keyword_parser())
                    .repeated(),
            )
            .foldl(|object, field| Expr::Member {
                object: Box::new(object),
                field,
            })
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

        let call = member_access
            .clone()
            .then(call_suffix.repeated())
            .foldl(|func, args| Expr::Call {
                func: Box::new(func),
                args,
            })
            .boxed();

        let await_expr = just(TokenKind::Await)
            .ignore_then(call.clone())
            .map(|expr| Expr::Await(Box::new(expr)));

        let spawn_expr = just(TokenKind::Spawn)
            .ignore_then(call.clone())
            .map(|expr| Expr::Spawn(Box::new(expr)));

        let unary = choice((
            just(TokenKind::Minus).to(UnaryOp::Neg),
            just(TokenKind::Bang).to(UnaryOp::Not),
        ))
        .then(choice((
            await_expr.clone(),
            spawn_expr.clone(),
            call.clone(),
        )))
        .map(|(op, expr)| Expr::Unary {
            op,
            expr: Box::new(expr),
        })
        .or(await_expr)
        .or(spawn_expr)
        .or(call.clone())
        .boxed();

        let product = unary
            .clone()
            .then(
                choice((
                    just(TokenKind::Star).to(BinaryOp::Mul),
                    just(TokenKind::Slash).to(BinaryOp::Div),
                    just(TokenKind::Percent).to(BinaryOp::Mod),
                ))
                .then(unary.clone())
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

        let range = sum
            .clone()
            .then(just(TokenKind::DoubleDot).ignore_then(sum.clone()).or_not())
            .map(|(start, end)| {
                if let Some(end) = end {
                    Expr::Range {
                        start: Box::new(start),
                        end: Box::new(end),
                    }
                } else {
                    start
                }
            });

        let comparison = range
            .clone()
            .then(
                choice((
                    just(TokenKind::EqEq).to(BinaryOp::Eq),
                    just(TokenKind::Neq).to(BinaryOp::Ne),
                    just(TokenKind::Lt).to(BinaryOp::Lt),
                    just(TokenKind::Gt).to(BinaryOp::Gt),
                    just(TokenKind::LtEq).to(BinaryOp::LtEq),
                    just(TokenKind::GtEq).to(BinaryOp::GtEq),
                ))
                .then(range.clone())
                .repeated(),
            )
            .foldl(|left, (op, right)| Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });

        let logical = comparison
            .clone()
            .then(
                choice((
                    just(TokenKind::Amp).to(BinaryOp::And),
                    just(TokenKind::Pipe).to(BinaryOp::Or),
                ))
                .then(comparison)
                .repeated(),
            )
            .foldl(|left, (op, right)| Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });

        logical
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
        .map(|arg| {
            Statement::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("print".to_string())),
                args: vec![arg],
            })
        });

    let return_stmt = just(TokenKind::Return)
        .ignore_then(expr.clone().or_not())
        .map(Statement::Return);

    let let_stmt = just(TokenKind::Let)
        .ignore_then(identifier_parser())
        .then_ignore(just(TokenKind::Equals))
        .then(expr.clone())
        .map(|(name, expr)| Statement::Let { name, expr });

    let assignment_stmt = identifier_parser()
        .then(choice((
            just(TokenKind::Equals).to(None),
            just(TokenKind::PlusEq).to(Some(BinaryOp::Add)),
            just(TokenKind::MinusEq).to(Some(BinaryOp::Sub)),
            just(TokenKind::StarEq).to(Some(BinaryOp::Mul)),
            just(TokenKind::SlashEq).to(Some(BinaryOp::Div)),
        )))
        .then(expr.clone())
        .map(|((name, op), rhs)| {
            let expr = if let Some(op) = op {
                // Desugar: x += y becomes x = x + y
                Expr::Binary {
                    op,
                    left: Box::new(Expr::Identifier(name.clone())),
                    right: Box::new(rhs),
                }
            } else {
                rhs
            };
            Statement::Assignment { name, expr }
        });

    let use_stmt = just(TokenKind::Use)
        .ignore_then(identifier_parser())
        .then_ignore(just(TokenKind::Colon))
        .then(identifier_parser())
        .then(
            just(TokenKind::As)
                .ignore_then(identifier_parser())
                .or_not(),
        )
        .map(|((namespace, module), alias)| Statement::Use {
            module: format!("{}:{}", namespace, module),
            alias,
        });

    let break_stmt = just(TokenKind::Break).map(|_| Statement::Break);
    let continue_stmt = just(TokenKind::Continue).map(|_| Statement::Continue);

    // Create a recursive parser for statements
    let statement = recursive(|stmt| {
        let elif_block = just(TokenKind::Elif)
            .ignore_then(expr.clone())
            .then_ignore(just(TokenKind::Colon))
            .then_ignore(newline.clone())
            .then(
                stmt.clone()
                    .repeated()
                    .at_least(1)
                    .delimited_by(just(TokenKind::Indent), just(TokenKind::Dedent))
                    .map(Block::new),
            )
            .map(|(cond, block)| (cond, block));

        let if_stmt = just(TokenKind::If)
            .ignore_then(expr.clone())
            .then_ignore(just(TokenKind::Colon))
            .then_ignore(newline.clone())
            .then(
                stmt.clone()
                    .repeated()
                    .at_least(1)
                    .delimited_by(just(TokenKind::Indent), just(TokenKind::Dedent))
                    .map(Block::new),
            )
            .then(elif_block.repeated())
            .then(
                just(TokenKind::Else)
                    .ignore_then(just(TokenKind::Colon))
                    .ignore_then(newline.clone())
                    .then(
                        stmt.clone()
                            .repeated()
                            .at_least(1)
                            .delimited_by(just(TokenKind::Indent), just(TokenKind::Dedent))
                            .map(Block::new),
                    )
                    .or_not(),
            )
            .map(|(((cond, then_block), elif_blocks), else_block)| Statement::If {
                cond: Box::new(cond),
                then_block,
                elif_blocks,
                else_block: else_block.map(|(_, block)| block),
            });

        let for_stmt = just(TokenKind::For)
            .ignore_then(identifier_parser())
            .then_ignore(just(TokenKind::In))
            .then(expr.clone())
            .then_ignore(just(TokenKind::Colon))
            .then_ignore(newline.clone())
            .then(
                stmt.clone()
                    .repeated()
                    .at_least(1)
                    .delimited_by(just(TokenKind::Indent), just(TokenKind::Dedent))
                    .map(Block::new),
            )
            .map(|((var, iterable), body)| Statement::For {
                var,
                iterable,
                body,
            });

        let while_stmt = just(TokenKind::While)
            .ignore_then(expr.clone())
            .then_ignore(just(TokenKind::Colon))
            .then_ignore(newline.clone())
            .then(
                stmt.clone()
                    .repeated()
                    .at_least(1)
                    .delimited_by(just(TokenKind::Indent), just(TokenKind::Dedent))
                    .map(Block::new),
            )
            .map(|(cond, body)| Statement::While { cond, body });

        choice((
            print_stmt,
            return_stmt,
            let_stmt,
            assignment_stmt,
            use_stmt,
            if_stmt,
            for_stmt,
            while_stmt,
            break_stmt,
            continue_stmt,
            expr.map(Statement::Expr),
        ))
        .then_ignore(newline.clone().or_not())
        .boxed()
    });

    let block = statement
        .clone()
        .repeated()
        .at_least(1)
        .delimited_by(just(TokenKind::Indent), just(TokenKind::Dedent))
        .map(Block::new);

    let function_param = identifier_parser()
        .then(
            just(TokenKind::Colon)
                .ignore_then(type_parser())
                .or_not(),
        )
        .map(|(name, ty)| Param::new(name, ty));

    let function_params = function_param
        .separated_by(just(TokenKind::Comma))
        .allow_trailing()
        .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
        .or_not()
        .map(|params| params.unwrap_or_default());

    let function_ret_type = just(TokenKind::Arrow)
        .ignore_then(type_parser())
        .or_not();

    let function = just(TokenKind::Fn)
        .ignore_then(identifier_parser())
        .then(function_params)
        .then(function_ret_type)
        .then_ignore(just(TokenKind::Colon))
        .then_ignore(newline.clone())
        .then(block.clone())
        .map(|(((name, params), ret_ty), body)| Function::new(name, params, ret_ty, body))
        .map(Statement::Function)
        .then_ignore(newline.clone().or_not());

    newline
        .clone()
        .or_not()
        .ignore_then(choice((function, statement)).repeated())
        .then_ignore(newline.repeated().or_not())
        .then_ignore(just(TokenKind::Eof))
        .map(Program::new)
}
