use otterlang::ast::{Expr, Literal, Statement};
use otterlang::lexer::tokenize;
use otterlang::parser::parse;

#[test]
fn parse_print_function() {
    let source = "fn main:\n    print(\"Hello\")\n";
    let tokens = tokenize(source).expect("tokenization should succeed");
    let program = parse(&tokens).expect("parsing should succeed");

    assert_eq!(program.functions.len(), 1);
    let function = &program.functions[0];
    assert_eq!(function.name, "main");
    assert_eq!(function.body.len(), 1);

    match &function.body[0] {
        Statement::Print(expr) => match expr {
            Expr::Literal(Literal::String(value)) => assert_eq!(value, "Hello"),
            other => panic!("expected string literal, got {:?}", other),
        },
        stmt => panic!("expected print statement, got {:?}", stmt),
    }
}

#[test]
fn parse_function_call_expression() {
    let source = "fn main:\n    x = add(2, 3)\n";
    let tokens = tokenize(source).expect("tokenization should succeed");
    let program = parse(&tokens).expect("parsing should succeed");

    assert_eq!(program.functions.len(), 1);
    let function = &program.functions[0];
    assert_eq!(function.body.len(), 1);

    match &function.body[0] {
        Statement::Assignment { name, expr } => {
            assert_eq!(name, "x");
            match expr {
                Expr::Call { callee, args } => {
                    match &**callee {
                        Expr::Identifier(name) => assert_eq!(name, "add"),
                        other => panic!("expected identifier callee, got {:?}", other),
                    }
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected call expression, got {:?}", other),
            }
        }
        other => panic!("expected assignment statement, got {:?}", other),
    }
}
