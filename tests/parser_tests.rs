use otterlang::ast::nodes::{Expr, Literal, Statement};
use otterlang::lexer::tokenize;
use otterlang::parser::parse;

#[test]
fn parse_print_function() {
    let source = "fn main:\n    print(\"Hello\")\n";
    let tokens = tokenize(source).expect("tokenization should succeed");
    let program = parse(&tokens).expect("parsing should succeed");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Function(function) => {
            assert_eq!(function.name, "main");
            assert_eq!(function.body.statements.len(), 1);
            match &function.body.statements[0] {
                Statement::Expr(expr) => match expr {
                    Expr::Call { func, args } => {
                        match &**func {
                            Expr::Identifier(name) => assert_eq!(name, "print"),
                            other => panic!("expected print identifier, got {:?}", other),
                        }
                        assert_eq!(args.len(), 1);
                        match &args[0] {
                            Expr::Literal(Literal::String(value)) => assert_eq!(value, "Hello"),
                            other => panic!("expected string literal, got {:?}", other),
                        }
                    }
                    other => panic!("expected function call, got {:?}", other),
                },
                stmt => panic!("expected expression statement, got {:?}", stmt),
            }
        }
        stmt => panic!("expected function statement, got {:?}", stmt),
    }
}

#[test]
fn parse_function_call_expression() {
    let source = "fn main:\n    x = add(2, 3)\n";
    let tokens = tokenize(source).expect("tokenization should succeed");
    let program = parse(&tokens).expect("parsing should succeed");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Function(function) => {
            assert_eq!(function.name, "main");
            assert_eq!(function.body.statements.len(), 1);
            match &function.body.statements[0] {
                Statement::Assignment { name, expr } => {
                    assert_eq!(name, "x");
                    match expr {
                        Expr::Call { func, args } => {
                            match &**func {
                                Expr::Identifier(name) => assert_eq!(name, "add"),
                                other => panic!("expected identifier func, got {:?}", other),
                            }
                            assert_eq!(args.len(), 2);
                        }
                        other => panic!("expected call expression, got {:?}", other),
                    }
                }
                other => panic!("expected assignment statement, got {:?}", other),
            }
        }
        other => panic!("expected function statement, got {:?}", other),
    }
}

#[test]
fn parse_if_with_elif() {
    let source = "fn main:\n    x = 10.0\n    if x > 5.0:\n        print(\"greater\")\n    elif x > 0.0:\n        print(\"positive\")\n    else:\n        print(\"zero or negative\")\n";
    let tokens = tokenize(source).expect("tokenization should succeed");
    let program = parse(&tokens).expect("parsing should succeed");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Function(function) => {
            assert_eq!(function.name, "main");
            assert_eq!(function.body.statements.len(), 2);
            
            // Check the if statement
            match &function.body.statements[1] {
                Statement::If {
                    cond: _,
                    then_block,
                    elif_blocks,
                    else_block,
                } => {
                    // Verify elif_blocks is not empty
                    assert_eq!(elif_blocks.len(), 1, "should have one elif block");
                    
                    // Verify else_block exists
                    assert!(else_block.is_some(), "should have else block");
                    
                    // Verify then_block has statements
                    assert_eq!(then_block.statements.len(), 1);
                    
                    // Verify elif block has statements
                    assert_eq!(elif_blocks[0].1.statements.len(), 1);
                }
                other => panic!("expected if statement, got {:?}", other),
            }
        }
        other => panic!("expected function statement, got {:?}", other),
    }
}

#[test]
fn parse_if_with_multiple_elif() {
    let source = "fn main:\n    x = 10.0\n    if x > 10.0:\n        print(\"greater\")\n    elif x > 5.0:\n        print(\"medium\")\n    elif x > 0.0:\n        print(\"small\")\n    else:\n        print(\"zero\")\n";
    let tokens = tokenize(source).expect("tokenization should succeed");
    let program = parse(&tokens).expect("parsing should succeed");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Function(function) => {
            match &function.body.statements[1] {
                Statement::If {
                    elif_blocks,
                    else_block,
                    ..
                } => {
                    assert_eq!(elif_blocks.len(), 2, "should have two elif blocks");
                    assert!(else_block.is_some(), "should have else block");
                }
                other => panic!("expected if statement, got {:?}", other),
            }
        }
        other => panic!("expected function statement, got {:?}", other),
    }
}

#[test]
fn parse_if_without_else() {
    let source = "fn main:\n    x = 10.0\n    if x > 5.0:\n        print(\"greater\")\n    elif x > 0.0:\n        print(\"positive\")\n";
    let tokens = tokenize(source).expect("tokenization should succeed");
    let program = parse(&tokens).expect("parsing should succeed");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Function(function) => {
            match &function.body.statements[1] {
                Statement::If {
                    elif_blocks,
                    else_block,
                    ..
                } => {
                    assert_eq!(elif_blocks.len(), 1, "should have one elif block");
                    assert!(else_block.is_none(), "should not have else block");
                }
                other => panic!("expected if statement, got {:?}", other),
            }
        }
        other => panic!("expected function statement, got {:?}", other),
    }
}
