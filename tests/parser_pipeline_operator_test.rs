use bara::analyzer::Analyzer;
use bara::ast::*;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn parse(input: &str) -> Program {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    Parser::new(tokens).parse_program().expect("parser failed")
}

fn parse_pipe_value(input: &str) -> Expr {
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1, "expected 1 item for: {input}");
    if let Item::TopLevelStmt(Stmt::Assign { value, .. }) = &prog.items[0] {
        value.clone()
    } else {
        panic!("expected Assign for: {input}");
    }
}

fn analyze(input: &str) -> Program {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    let mut prog = Parser::new(tokens).parse_program().expect("parser failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut prog).expect("analysis failed");
    prog
}

fn analyzed_assign_value(prog: &Program, index: usize) -> Expr {
    if let Item::TopLevelStmt(Stmt::Assign { value, .. }) = &prog.items[index] {
        value.clone()
    } else {
        panic!("expected Assign at item {index}");
    }
}

fn transpile_and_verify(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().expect("lexing failed");
    let mut program = Parser::new(tokens).parse_program().expect("parsing failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("analysis failed");
    let mut codegen = Codegen::new(&analyzer);
    let rust_code = codegen.generate(&program);
    syn::parse_file(&rust_code).expect("generated Rust must be valid (syn)");
    rust_code
}

#[test]
fn test_parse_pipe_to_bare_ident() {
    let value = parse_pipe_value("y = x |> f");
    if let Expr::Pipe { left, right, .. } = value {
        assert!(matches!(left.as_ref(), Expr::Ident(n, _) if n == "x"));
        assert!(matches!(right.as_ref(), Expr::Ident(n, _) if n == "f"));
    } else {
        panic!("expected Pipe expr");
    }
}

#[test]
fn test_parse_pipe_to_call() {
    let value = parse_pipe_value("y = x |> f()");
    if let Expr::Pipe { left, right, .. } = value {
        assert!(matches!(left.as_ref(), Expr::Ident(n, _) if n == "x"));
        assert!(matches!(right.as_ref(), Expr::Call { args, .. } if args.is_empty()));
    } else {
        panic!("expected Pipe expr");
    }
}

#[test]
fn test_parse_pipe_call_with_args() {
    let value = parse_pipe_value("y = x |> add(5)");
    if let Expr::Pipe { right, .. } = value {
        if let Expr::Call { callee, args, .. } = *right {
            assert!(matches!(callee.as_ref(), Expr::Ident(n, _) if n == "add"));
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected Call on pipe right side");
        }
    } else {
        panic!("expected Pipe expr");
    }
}

#[test]
fn test_parse_chained_pipe_shape() {
    let value = parse_pipe_value("z = x |> f() |> g()");
    if let Expr::Pipe { left, right, .. } = value {
        assert!(matches!(left.as_ref(), Expr::Pipe { .. }));
        assert!(matches!(right.as_ref(), Expr::Call { .. }));
    } else {
        panic!("expected outer Pipe expr");
    }
}

#[test]
fn test_lower_pipe_bare_ident() {
    let prog = analyze("double(x) => x * 2\ny = 10 |> double");
    let value = analyzed_assign_value(&prog, 1);
    if let Expr::Call { callee, args, .. } = value {
        assert!(matches!(callee.as_ref(), Expr::Ident(n, _) if n == "double"));
        assert_eq!(args.len(), 1);
        assert!(matches!(args[0], Expr::Literal(Literal::Int(10), _)));
    } else {
        panic!("expected lowered Call, got: {value:?}");
    }
}

#[test]
fn test_lower_pipe_call_no_args() {
    let prog = analyze("f(x) => x * 2\ny = 5 |> f()");
    let value = analyzed_assign_value(&prog, 1);
    if let Expr::Call { callee, args, .. } = value {
        assert!(matches!(callee.as_ref(), Expr::Ident(n, _) if n == "f"));
        assert_eq!(args.len(), 1);
        assert!(matches!(args[0], Expr::Literal(Literal::Int(5), _)));
    } else {
        panic!("expected lowered Call, got: {value:?}");
    }
}

#[test]
fn test_lower_pipe_prepends_to_existing_args() {
    let prog = analyze("add(a, b) => a + b\ny = 10 |> add(5)");
    let value = analyzed_assign_value(&prog, 1);
    if let Expr::Call { callee, args, .. } = value {
        assert!(matches!(callee.as_ref(), Expr::Ident(n, _) if n == "add"));
        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], Expr::Literal(Literal::Int(10), _)));
        assert!(matches!(args[1], Expr::Literal(Literal::Int(5), _)));
    } else {
        panic!("expected lowered Call, got: {value:?}");
    }
}

#[test]
fn test_lower_chained_pipe_nests_calls() {
    let prog = analyze("f(x) => x * 2\ng(x) => x + 1\nz = 1 |> f() |> g()");
    let value = analyzed_assign_value(&prog, 2);
    if let Expr::Call { callee, args, .. } = value {
        assert!(matches!(callee.as_ref(), Expr::Ident(n, _) if n == "g"));
        assert_eq!(args.len(), 1);
        if let Expr::Call { callee: inner_callee, args: inner_args, .. } = &args[0] {
            assert!(matches!(inner_callee.as_ref(), Expr::Ident(n, _) if n == "f"));
            assert_eq!(inner_args.len(), 1);
        } else {
            panic!("expected nested f() call as g() arg");
        }
    } else {
        panic!("expected lowered outer Call, got: {value:?}");
    }
}

#[test]
fn test_codegen_pipe_output_and_syn() {
    let rust = transpile_and_verify("double(x: int) -> int => x * 2\nval = 10 |> double()");
    assert!(rust.contains("double(10)"), "output was:\n{rust}");
}
