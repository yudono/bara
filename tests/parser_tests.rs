use bara::ast::*;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn parse(input: &str) -> Program {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexer failed");
    let mut parser = Parser::new(tokens);
    parser.parse_program().expect("parser failed")
}

#[test]
fn test_parse_variables_and_mutation() {
    let input = "name = \"Bara\"\nscore = 10\nscore += 5";
    let prog = parse(input);
    assert_eq!(prog.items.len(), 3);
}

#[test]
fn test_parse_function_block() {
    let input = "proses(a, b) =>\n    b.push(a)\n    a * 2";
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1);
    if let Item::Fn(f) = &prog.items[0] {
        assert_eq!(f.name, "proses");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.body.stmts.len(), 2);
    } else {
        panic!("expected Fn item");
    }
}

#[test]
fn test_parse_type_with_fields_and_methods() {
    let input = r#"
type Player =
    name: str
    hp = 100

    hit(dmg: int) =>
        hp -= dmg
        if hp <= 0 => print("defeated")

    is_alive() => hp > 0
"#;
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1);
    if let Item::Type(t) = &prog.items[0] {
        assert_eq!(t.name, "Player");
        assert_eq!(t.fields.len(), 2);
        assert_eq!(t.methods.len(), 2);
    } else {
        panic!("expected Type item");
    }
}

#[test]
fn test_parse_string_interpolation() {
    let input = "msg = \"Hello {name}, your score is {score + 5}\"";
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1);
    if let Item::TopLevelStmt(Stmt::Assign { value, .. }) = &prog.items[0] {
        if let Expr::Literal(Literal::InterpolatedStr(parts), _) = value {
            assert_eq!(parts.len(), 4); // "Hello ", expr(name), ", your score is ", expr(score + 5)
        } else {
            panic!("expected InterpolatedStr literal");
        }
    } else {
        panic!("expected Assign statement");
    }
}

#[test]
fn test_parse_pipeline() {
    let input = "res = x |> double() |> add(5)";
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1);
    if let Item::TopLevelStmt(Stmt::Assign { value, .. }) = &prog.items[0] {
        assert!(matches!(value, Expr::Pipe { .. }));
    } else {
        panic!("expected Assign statement with Pipe");
    }
}

#[test]
fn test_parse_raw_rust() {
    let input = "raw_rust { println!(\"Direct rust\"); }";
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1);
    assert!(matches!(prog.items[0], Item::RawRust(_, _)));
}
