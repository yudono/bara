use bara::ast::*;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn parse(input: &str) -> Program {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    Parser::new(tokens).parse_program().expect("parser failed")
}

fn parse_assign(input: &str) -> (Expr, Expr, Option<BinaryOp>) {
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1, "expected 1 item for: {input}");
    if let Item::TopLevelStmt(Stmt::Assign { target, value, op, .. }) = &prog.items[0] {
        (target.clone(), value.clone(), *op)
    } else {
        panic!("expected TopLevelStmt Assign for: {input}");
    }
}

#[test]
fn test_simple_int_assignment() {
    let (target, value, op) = parse_assign("x = 42");
    assert!(matches!(target, Expr::Ident(ref n, _) if n == "x"));
    assert!(matches!(value, Expr::Literal(Literal::Int(42), _)));
    assert_eq!(op, None);
}

#[test]
fn test_string_assignment() {
    let (target, value, _) = parse_assign("name = \"Bara\"");
    assert!(matches!(target, Expr::Ident(ref n, _) if n == "name"));
    assert!(matches!(value, Expr::Literal(Literal::Str(ref s), _) if s == "Bara"));
}

#[test]
fn test_compound_add_assign() {
    let prog = parse("score = 10\nscore += 5");
    assert_eq!(prog.items.len(), 2);
    if let Item::TopLevelStmt(Stmt::Assign { target, value, op, .. }) = &prog.items[1] {
        assert!(matches!(target, Expr::Ident(ref n, _) if n == "score"));
        assert!(matches!(value, Expr::Literal(Literal::Int(5), _)));
        assert_eq!(*op, Some(BinaryOp::Add));
    } else {
        panic!("expected compound Assign statement");
    }
}

#[test]
fn test_compound_all_ops() {
    for (src, expected) in [
        ("x -= 1", BinaryOp::Sub),
        ("x *= 2", BinaryOp::Mul),
        ("x /= 2", BinaryOp::Div),
        ("x %= 2", BinaryOp::Mod),
    ] {
        let (_, _, op) = parse_assign(src);
        assert_eq!(op, Some(expected), "wrong op for: {src}");
    }
}

#[test]
fn test_function_call() {
    let (_, value, _) = parse_assign("res = add(1, 2)");
    if let Expr::Call { callee, args, .. } = value {
        assert!(matches!(callee.as_ref(), Expr::Ident(n, _) if n == "add"));
        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], Expr::Literal(Literal::Int(1), _)));
        assert!(matches!(args[1], Expr::Literal(Literal::Int(2), _)));
    } else {
        panic!("expected Call expr");
    }
}

#[test]
fn test_nested_function_call() {
    let (_, value, _) = parse_assign("res = add(mul(2, 3), 4)");
    if let Expr::Call { args, .. } = value {
        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], Expr::Call { .. }));
        assert!(matches!(args[1], Expr::Literal(Literal::Int(4), _)));
    } else {
        panic!("expected outer Call expr");
    }
}

#[test]
fn test_index_access() {
    let (_, value, _) = parse_assign("v = arr[1]");
    if let Expr::Index { collection, index, .. } = value {
        assert!(matches!(collection.as_ref(), Expr::Ident(n, _) if n == "arr"));
        assert!(matches!(index.as_ref(), Expr::Literal(Literal::Int(1), _)));
    } else {
        panic!("expected Index expr");
    }
}

#[test]
fn test_member_access() {
    let (_, value, _) = parse_assign("n = user.name");
    if let Expr::MemberAccess { object, member, .. } = value {
        assert!(matches!(object.as_ref(), Expr::Ident(n, _) if n == "user"));
        assert_eq!(member, "name");
    } else {
        panic!("expected MemberAccess expr");
    }
}

#[test]
fn test_method_call() {
    let prog = parse("user.greet(\"hi\")");
    assert_eq!(prog.items.len(), 1);
    if let Item::TopLevelStmt(Stmt::Expr(Expr::MethodCall { receiver, method, args, .. })) =
        &prog.items[0]
    {
        assert!(matches!(receiver.as_ref(), Expr::Ident(n, _) if n == "user"));
        assert_eq!(method, "greet");
        assert_eq!(args.len(), 1);
    } else {
        panic!("expected Expr MethodCall statement");
    }
}

#[test]
fn test_cast_as() {
    let (_, value, _) = parse_assign("v = x as int");
    if let Expr::Cast { expr, target_ty, .. } = value {
        assert!(matches!(expr.as_ref(), Expr::Ident(n, _) if n == "x"));
        assert_eq!(target_ty, "int");
    } else {
        panic!("expected Cast expr");
    }
}

#[test]
fn test_try_bang_call_and_ident() {
    let (_, call_value, _) = parse_assign("v = load()!");
    assert!(
        matches!(call_value, Expr::Try(inner, _) if matches!(inner.as_ref(), Expr::Call { .. })),
        "expected Try(Call)"
    );
    let (_, ident_value, _) = parse_assign("w = x!");
    assert!(
        matches!(ident_value, Expr::Try(inner, _) if matches!(inner.as_ref(), Expr::Ident(n, _) if n == "x")),
        "expected Try(Ident)"
    );
}

#[test]
fn test_ternary_expression() {
    let (_, value, _) = parse_assign("m = a > b ? a : b");
    if let Expr::Ternary { cond, then_expr, else_expr, .. } = value {
        assert!(matches!(cond.as_ref(), Expr::Binary { op: BinaryOp::Gt, .. }));
        assert!(matches!(then_expr.as_ref(), Expr::Ident(n, _) if n == "a"));
        assert!(matches!(else_expr.as_ref(), Expr::Ident(n, _) if n == "b"));
    } else {
        panic!("expected Ternary expr");
    }
}

#[test]
fn test_closure_expression() {
    let (_, value, _) = parse_assign("f = x => x * 2");
    if let Expr::Closure { params, body, .. } = value {
        assert_eq!(params, vec!["x".to_string()]);
        assert!(matches!(body.as_ref(), Expr::Binary { op: BinaryOp::Mul, .. }));
    } else {
        panic!("expected Closure expr");
    }
}

#[test]
fn test_tuple_expression() {
    let (_, value, _) = parse_assign("t = (1, \"two\")");
    if let Expr::Tuple(items, _) = value {
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], Expr::Literal(Literal::Int(1), _)));
        assert!(matches!(items[1], Expr::Literal(Literal::Str(_), _)));
    } else {
        panic!("expected Tuple expr");
    }
}

#[test]
fn test_match_expression() {
    let (_, value, _) = parse_assign("desc = match x { 1 => \"one\", 2 => \"two\" }");
    if let Expr::Match { expr, arms, .. } = value {
        assert!(matches!(expr.as_ref(), Expr::Ident(n, _) if n == "x"));
        assert_eq!(arms.len(), 2);
        assert!(arms[0].guard.is_none());
        assert!(arms[1].guard.is_none());
    } else {
        panic!("expected Match expr");
    }
}
