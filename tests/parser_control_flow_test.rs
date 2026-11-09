use bara::analyzer::Analyzer;
use bara::ast::*;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn parse(input: &str) -> Program {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    Parser::new(tokens).parse_program().expect("parser failed")
}

fn parse_single_stmt(input: &str) -> Stmt {
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1, "expected 1 item for: {input}");
    if let Item::TopLevelStmt(s) = &prog.items[0] {
        s.clone()
    } else {
        panic!("expected TopLevelStmt for: {input}");
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
fn test_if_else_statement() {
    let stmt = parse_single_stmt("if x > 0 =>\n    y = 1\nelse =>\n    y = 2");
    if let Stmt::If { cond, then_branch, elif_branches, else_branch, .. } = stmt {
        assert!(matches!(cond, Expr::Binary { op: BinaryOp::Gt, .. }));
        assert_eq!(then_branch.stmts.len(), 1);
        assert!(elif_branches.is_empty());
        assert!(else_branch.is_some());
        assert_eq!(else_branch.unwrap().stmts.len(), 1);
    } else {
        panic!("expected If stmt");
    }
}

#[test]
fn test_elif_chain() {
    let stmt = parse_single_stmt(
        "if s >= 90 =>\n    g = \"A\"\nelif s >= 80 =>\n    g = \"B\"\nelif s >= 70 =>\n    g = \"C\"\nelse =>\n    g = \"F\"",
    );
    if let Stmt::If { elif_branches, else_branch, .. } = stmt {
        assert_eq!(elif_branches.len(), 2);
        assert!(else_branch.is_some());
    } else {
        panic!("expected If stmt with elif chain");
    }
}

#[test]
fn test_nested_if() {
    let stmt = parse_single_stmt("if a =>\n    if b =>\n        c = 1");
    if let Stmt::If { then_branch, .. } = stmt {
        assert_eq!(then_branch.stmts.len(), 1);
        assert!(matches!(then_branch.stmts[0], Stmt::If { .. }));
    } else {
        panic!("expected outer If stmt");
    }
}

#[test]
fn test_for_in_loop() {
    let stmt = parse_single_stmt("for x in xs =>\n    print(x)");
    if let Stmt::For { item, iter, body, .. } = stmt {
        assert_eq!(item, "x");
        assert!(matches!(iter, Expr::Ident(ref n, _) if n == "xs"));
        assert_eq!(body.stmts.len(), 1);
    } else {
        panic!("expected For stmt");
    }
}

#[test]
fn test_nested_for_loops() {
    let stmt = parse_single_stmt("for x in a =>\n    for y in b =>\n        n = n + 1");
    if let Stmt::For { item, body, .. } = stmt {
        assert_eq!(item, "x");
        assert_eq!(body.stmts.len(), 1);
        if let Stmt::For { item: inner, .. } = &body.stmts[0] {
            assert_eq!(inner, "y");
        } else {
            panic!("expected inner For stmt");
        }
    } else {
        panic!("expected outer For stmt");
    }
}

#[test]
fn test_while_loop() {
    let stmt = parse_single_stmt("while i < n =>\n    i = i + 1");
    if let Stmt::While { cond, body, .. } = stmt {
        assert!(matches!(cond, Expr::Binary { op: BinaryOp::Lt, .. }));
        assert_eq!(body.stmts.len(), 1);
    } else {
        panic!("expected While stmt");
    }
}

#[test]
fn test_match_statement() {
    let stmt =
        parse_single_stmt("match x { 1 => print(\"one\"), 2 => print(\"two\"), 3 => print(\"three\") }");
    if let Stmt::Match { expr, arms, .. } = stmt {
        assert!(matches!(expr, Expr::Ident(ref n, _) if n == "x"));
        assert_eq!(arms.len(), 3);
        assert!(arms.iter().all(|a| a.guard.is_none()));
    } else {
        panic!("expected Match stmt");
    }
}

#[test]
fn test_match_statement_with_guard() {
    let stmt =
        parse_single_stmt("match x { 1 if x > 0 => print(\"big one\"), 2 => print(\"two\") }");
    if let Stmt::Match { arms, .. } = stmt {
        assert_eq!(arms.len(), 2);
        assert!(arms[0].guard.is_some());
        assert!(arms[1].guard.is_none());
    } else {
        panic!("expected Match stmt with guard");
    }
}

#[test]
fn test_match_expr_with_guard() {
    let stmt = parse_single_stmt("desc = match x { 1 if x > 0 => \"one\", 2 => \"two\" }");
    if let Stmt::Assign { value, .. } = stmt {
        if let Expr::Match { arms, .. } = value {
            assert_eq!(arms.len(), 2);
            assert!(arms[0].guard.is_some());
        } else {
            panic!("expected Match expr value");
        }
    } else {
        panic!("expected Assign stmt");
    }
}

#[test]
fn test_break_and_continue_in_while() {
    let stmt = parse_single_stmt(
        "while i < 10 =>\n    i = i + 1\n    if i == 5 =>\n        continue\n    if i == 8 =>\n        break",
    );
    if let Stmt::While { body, .. } = stmt {
        assert_eq!(body.stmts.len(), 3);
        let has_continue = matches!(&body.stmts[1], Stmt::If { then_branch, .. }
            if matches!(then_branch.stmts[0], Stmt::Continue(_)));
        let has_break = matches!(&body.stmts[2], Stmt::If { then_branch, .. }
            if matches!(then_branch.stmts[0], Stmt::Break(_)));
        assert!(has_continue, "expected continue in second if");
        assert!(has_break, "expected break in third if");
    } else {
        panic!("expected While stmt");
    }
}

#[test]
fn test_return_with_value_and_bare_return() {
    let with_val = parse("fn f(x: int) =>\n    return x * 2");
    if let Item::Fn(f) = &with_val.items[0] {
        assert!(matches!(&f.body.stmts[0], Stmt::Return(Some(_), _)));
    } else {
        panic!("expected Fn item");
    }
    let bare = parse("fn g() =>\n    print(\"hi\")\n    return");
    if let Item::Fn(g) = &bare.items[0] {
        assert!(matches!(&g.body.stmts[1], Stmt::Return(None, _)));
    } else {
        panic!("expected Fn item");
    }
}

#[test]
fn test_full_control_flow_transpile_syn() {
    let rust = transpile_and_verify(
        "classify(s: int) -> str =>\n    if s >= 90 =>\n        return \"A\"\n    elif s >= 80 =>\n        return \"B\"\n    else =>\n        return \"F\"\n\ntotal(xs: [int]) -> int =>\n    sum = 0\n    for x in xs =>\n        sum = sum + x\n    i = 0\n    while i < 3 =>\n        sum = sum + i\n        i = i + 1\n    return sum\n\ng = classify(95)\nt = total([1, 2, 3])",
    );
    assert!(rust.contains("if (s >= 90)"), "output was:\n{rust}");
    assert!(rust.contains("else if (s >= 80)"), "output was:\n{rust}");
    assert!(rust.contains("for x in"), "output was:\n{rust}");
    assert!(rust.contains("while (i < 3)"), "output was:\n{rust}");
}
