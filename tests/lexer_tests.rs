use otterlang::lexer::{tokenize, TokenKind};

#[test]
fn tokenize_simple_function() {
    let source = "fn main:\n    print(\"Hello\")\n";
    let tokens = tokenize(source).expect("tokenization should succeed");

    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
    let expected = vec![
        TokenKind::Fn,
        TokenKind::Identifier("main".to_string()),
        TokenKind::Colon,
        TokenKind::Newline,
        TokenKind::Indent,
        TokenKind::Print,
        TokenKind::LParen,
        TokenKind::StringLiteral("Hello".to_string()),
        TokenKind::RParen,
        TokenKind::Newline,
        TokenKind::Dedent,
        TokenKind::Eof,
    ];

    assert_eq!(kinds, expected);
}
