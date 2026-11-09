use bara::analyzer::Analyzer;
use bara::ast::*;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn parse(input: &str) -> Program {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    Parser::new(tokens).parse_program().expect("parser failed")
}

fn parse_type(input: &str) -> TypeDef {
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1, "expected 1 item for: {input}");
    if let Item::Type(t) = &prog.items[0] {
        t.clone()
    } else {
        panic!("expected Type item for: {input}");
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
fn test_basic_typed_fields() {
    let t = parse_type("type Point =\n    x: int\n    y: int");
    assert_eq!(t.name, "Point");
    assert_eq!(t.fields.len(), 2);
    assert_eq!(t.fields[0].name, "x");
    assert_eq!(t.fields[0].ty, Some(TypeAnnotation::Int));
    assert_eq!(t.fields[1].name, "y");
    assert!(t.methods.is_empty());
}

#[test]
fn test_fields_with_defaults() {
    let t = parse_type(
        "type Config =\n    title: str = \"Untitled\"\n    width: int = 800\n    active: bool = true",
    );
    assert_eq!(t.fields.len(), 3);
    assert!(
        matches!(&t.fields[0].default_val, Some(Expr::Literal(Literal::Str(s), _)) if s == "Untitled")
    );
    assert!(
        matches!(&t.fields[1].default_val, Some(Expr::Literal(Literal::Int(800), _)))
    );
    assert!(
        matches!(&t.fields[2].default_val, Some(Expr::Literal(Literal::Bool(true), _)))
    );
}

#[test]
fn test_untyped_field_with_default() {
    let t = parse_type("type Player =\n    hp = 100");
    assert_eq!(t.fields.len(), 1);
    assert_eq!(t.fields[0].name, "hp");
    assert!(t.fields[0].ty.is_none());
    assert!(
        matches!(&t.fields[0].default_val, Some(Expr::Literal(Literal::Int(100), _)))
    );
}

#[test]
fn test_pub_type_parses() {
    let t = parse_type("pub type Foo =\n    a: int");
    assert_eq!(t.name, "Foo");
    assert!(t.is_pub);
    assert_eq!(t.fields.len(), 1);
}

#[test]
fn test_type_with_methods() {
    let t = parse_type(
        "type Player =\n    name: str\n    hp = 100\n\n    hit(dmg: int) =>\n        hp -= dmg\n\n    is_alive() => hp > 0",
    );
    assert_eq!(t.name, "Player");
    assert_eq!(t.fields.len(), 2);
    assert_eq!(t.methods.len(), 2);
    assert_eq!(t.methods[0].name, "hit");
    assert_eq!(t.methods[0].params.len(), 1);
    assert_eq!(t.methods[1].name, "is_alive");
}

#[test]
fn test_method_typed_param_and_return() {
    let t = parse_type(
        "type Calc =\n    base: int\n\n    scaled(factor: int) -> int =>\n        return base * factor",
    );
    assert_eq!(t.methods.len(), 1);
    let m = &t.methods[0];
    assert_eq!(m.name, "scaled");
    assert_eq!(m.params[0].ty, Some(TypeAnnotation::Int));
    assert_eq!(m.return_ty, Some(TypeAnnotation::Int));
    assert!(matches!(m.body.stmts[0], Stmt::Return(Some(_), _)));
}

#[test]
fn test_list_field_type() {
    let t = parse_type("type TaskList =\n    tasks: [str]");
    assert_eq!(t.fields.len(), 1);
    assert!(
        matches!(&t.fields[0].ty, Some(TypeAnnotation::List(inner)) if **inner == TypeAnnotation::Str),
        "expected [str] field"
    );
}

#[test]
fn test_nested_construction_call() {
    let prog = parse("p = Point(x = 10, y = 15)");
    if let Item::TopLevelStmt(Stmt::Assign { target: _, value, .. }) = &prog.items[0] {
        if let Expr::Call { callee, args, .. } = value {
            assert!(matches!(callee.as_ref(), Expr::Ident(n, _) if n == "Point"));
            assert_eq!(args.len(), 2);
            for arg in args {
                assert!(matches!(arg, Expr::Assign { .. }), "ctor arg must be Assign");
            }
        } else {
            panic!("expected Call for struct construction");
        }
    } else {
        panic!("expected Assign statement");
    }
}

#[test]
fn test_construction_inside_list() {
    let prog = parse("items = [Point(x = 1, y = 2), Point(x = 3, y = 4)]");
    if let Item::TopLevelStmt(Stmt::Assign { value, .. }) = &prog.items[0] {
        if let Expr::List(elements, _) = value {
            assert_eq!(elements.len(), 2);
            for el in elements {
                assert!(matches!(el, Expr::Call { .. }));
            }
        } else {
            panic!("expected List value");
        }
    } else {
        panic!("expected Assign statement");
    }
}

#[test]
fn test_full_pipeline_struct_syn() {
    let rust = transpile_and_verify(
        "type WindowConfig =\n    title: str = \"Untitled\"\n    width: int = 800\n    height: int = 600\n\ncfg = WindowConfig(title = \"App\", width = 1280, height = 720)",
    );
    assert!(rust.contains("pub struct WindowConfig"), "output was:\n{rust}");
    assert!(rust.contains("impl Default for WindowConfig"), "output was:\n{rust}");
}
