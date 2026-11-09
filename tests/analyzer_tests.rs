use bara::analyzer::Analyzer;
use bara::ast::*;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn analyze(input: &str) -> (Program, Analyzer) {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexer failed");
    let mut parser = Parser::new(tokens);
    let mut prog = parser.parse_program().expect("parser failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut prog).expect("analysis failed");
    (prog, analyzer)
}

#[test]
fn test_mutability_inference() {
    let input = "x = 10\ny = 20\ny += 5";
    let (prog, _) = analyze(input);
    // Stmt 0: x = 10 -> is_decl: true, is_mut: false
    if let Item::TopLevelStmt(Stmt::Assign { is_decl, is_mut, .. }) = &prog.items[0] {
        assert!(is_decl);
        assert!(!is_mut);
    } else {
        panic!("expected assign 0");
    }

    // Stmt 1: y = 20 -> is_decl: true, is_mut: true
    if let Item::TopLevelStmt(Stmt::Assign { is_decl, is_mut, .. }) = &prog.items[1] {
        assert!(is_decl);
        assert!(is_mut);
    } else {
        panic!("expected assign 1");
    }
}

#[test]
fn test_variable_shadowing() {
    let input = "x = 10\nx = \"hello\"";
    let (prog, _) = analyze(input);
    // Stmt 0: x = 10 (int) -> is_decl: true
    if let Item::TopLevelStmt(Stmt::Assign { is_decl, .. }) = &prog.items[0] {
        assert!(is_decl);
    }
    // Stmt 1: x = "hello" (str) -> is_decl: true (Shadowing!)
    if let Item::TopLevelStmt(Stmt::Assign { is_decl, .. }) = &prog.items[1] {
        assert!(is_decl, "Expected variable shadowing to trigger is_decl: true");
    }
}

#[test]
fn test_function_auto_borrowing() {
    let input = "proses(a, b) =>\n    b.push(a)\n    a * 2";
    let (prog, _) = analyze(input);
    if let Item::Fn(f) = &prog.items[0] {
        assert_eq!(f.params[0].ownership, Some(ParamOwnership::Ref));
        assert_eq!(f.params[1].ownership, Some(ParamOwnership::MutRef));
    } else {
        panic!("expected Fn item");
    }
}

#[test]
fn test_struct_self_resolution_and_receiver() {
    let input = r#"
type Player =
    name: str
    hp = 100

    hit(dmg: int) =>
        hp -= dmg

    is_alive() => hp > 0
"#;
    let (prog, _) = analyze(input);
    if let Item::Type(t) = &prog.items[0] {
        assert_eq!(t.methods[0].name, "hit");
        assert_eq!(t.methods[0].receiver, ReceiverKind::MutSelf);

        assert_eq!(t.methods[1].name, "is_alive");
        assert_eq!(t.methods[1].receiver, ReceiverKind::RefSelf);
    } else {
        panic!("expected Type item");
    }
}

#[test]
fn test_heterogeneous_list_dyn() {
    let input = "data = [1, \"two\", true]";
    let (prog, analyzer) = analyze(input);
    assert!(analyzer.needs_bara_dyn, "Should flag needs_bara_dyn");
    if let Item::TopLevelStmt(Stmt::Assign { value, .. }) = &prog.items[0] {
        if let Expr::List(elements, _) = value {
            assert!(matches!(elements[0], Expr::DynWrap { .. }));
            assert!(matches!(elements[1], Expr::DynWrap { .. }));
            assert!(matches!(elements[2], Expr::DynWrap { .. }));
        } else {
            panic!("expected list");
        }
    }
}

#[test]
fn test_pipeline_lowering() {
    let input = "f(x) => x * 2\ny = 5 |> f()";
    let (prog, _) = analyze(input);
    if let Item::TopLevelStmt(Stmt::Assign { value, .. }) = &prog.items[1] {
        if let Expr::Call { callee, args, .. } = value {
            assert!(matches!(callee.as_ref(), Expr::Ident(name, _) if name == "f"));
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected lowered Call");
        }
    }
}

#[test]
fn test_index_auto_cast_usize() {
    let input = "arr = [10, 20, 30]\ni = 1\nval = arr[i]";
    let (prog, _) = analyze(input);
    if let Item::TopLevelStmt(Stmt::Assign { value, .. }) = &prog.items[2] {
        if let Expr::Index { index, .. } = value {
            if let Expr::Cast { target_ty, .. } = index.as_ref() {
                assert_eq!(target_ty, "usize");
            } else {
                panic!("expected index cast to usize");
            }
        } else {
            panic!("expected Index expr");
        }
    }
}
