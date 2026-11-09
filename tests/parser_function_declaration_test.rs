use bara::analyzer::Analyzer;
use bara::ast::*;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn parse(input: &str) -> Program {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    Parser::new(tokens).parse_program().expect("parser failed")
}

fn parse_fn(input: &str) -> FnDef {
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1, "expected 1 item for: {input}");
    if let Item::Fn(f) = &prog.items[0] {
        f.clone()
    } else {
        panic!("expected Fn item for: {input}");
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
fn test_bare_fn_inline_body() {
    let f = parse_fn("add(a, b) => a + b");
    assert_eq!(f.name, "add");
    assert_eq!(f.params.len(), 2);
    assert!(f.params.iter().all(|p| p.ty.is_none()));
    assert_eq!(f.body.stmts.len(), 1);
}

#[test]
fn test_fn_keyword_form() {
    let f = parse_fn("fn greet(name) => name");
    assert_eq!(f.name, "greet");
    assert_eq!(f.params.len(), 1);
    assert_eq!(f.params[0].name, "name");
}

#[test]
fn test_pub_fn_flag() {
    let f = parse_fn("pub fn double(x) => x * 2");
    assert_eq!(f.name, "double");
    assert!(f.is_pub);
}

#[test]
fn test_return_type_arrow() {
    let f = parse_fn("fn double(x: int) -> int => x * 2");
    assert_eq!(f.name, "double");
    assert_eq!(f.return_ty, Some(TypeAnnotation::Int));
}

#[test]
fn test_typed_params_int_str_bool() {
    let f = parse_fn("fn f(name: str, age: int, active: bool) => age");
    assert_eq!(f.params.len(), 3);
    assert_eq!(f.params[0].ty, Some(TypeAnnotation::Str));
    assert_eq!(f.params[1].ty, Some(TypeAnnotation::Int));
    assert_eq!(f.params[2].ty, Some(TypeAnnotation::Bool));
}

#[test]
fn test_list_param_type() {
    let f = parse_fn("fn sum(xs: [int]) => xs");
    assert_eq!(f.params.len(), 1);
    assert!(
        matches!(&f.params[0].ty, Some(TypeAnnotation::List(inner)) if **inner == TypeAnnotation::Int),
        "expected [int] param"
    );
}

#[test]
fn test_tuple_param_type() {
    let f = parse_fn("fn first(t: (int, str)) -> int => 1");
    assert_eq!(f.params.len(), 1);
    assert!(
        matches!(&f.params[0].ty, Some(TypeAnnotation::Tuple(items)) if *items == vec![TypeAnnotation::Int, TypeAnnotation::Str]),
        "expected (int, str) param"
    );
    assert_eq!(f.return_ty, Some(TypeAnnotation::Int));
}

#[test]
fn test_option_param_and_return() {
    let f = parse_fn("fn maybe(x: Option<int>) -> Option<int> => x");
    let param_ty = f.params[0].ty.as_ref().expect("param needs type");
    assert_eq!(param_ty.to_string(), "Option<i64>");
    assert_eq!(
        f.return_ty.as_ref().expect("needs return type").to_string(),
        "Option<i64>"
    );
}

#[test]
fn test_default_param_value() {
    let f = parse_fn("fn greet(name = \"World\") => name");
    assert_eq!(f.params.len(), 1);
    assert!(
        matches!(&f.params[0].default_val, Some(Expr::Literal(Literal::Str(s), _)) if s == "World"),
        "expected string default"
    );
}

#[test]
fn test_typed_params_with_defaults() {
    let f = parse_fn("fn add(a: int = 1, b: int = 2) => a + b");
    assert_eq!(f.params.len(), 2);
    assert_eq!(f.params[0].ty, Some(TypeAnnotation::Int));
    assert!(
        matches!(&f.params[0].default_val, Some(Expr::Literal(Literal::Int(1), _))),
        "expected default 1"
    );
    assert!(
        matches!(&f.params[1].default_val, Some(Expr::Literal(Literal::Int(2), _))),
        "expected default 2"
    );
}

#[test]
fn test_multiline_block_body() {
    let f = parse_fn("fn calc(a: int, b: int) -> int =>\n    s = a + b\n    return s");
    assert_eq!(f.params.len(), 2);
    assert_eq!(f.return_ty, Some(TypeAnnotation::Int));
    assert_eq!(f.body.stmts.len(), 2);
    assert!(matches!(f.body.stmts[0], Stmt::Assign { .. }));
    assert!(matches!(f.body.stmts[1], Stmt::Return(Some(_), _)));
}

#[test]
fn test_full_pipeline_fn_syn() {
    let rust = transpile_and_verify("pub double(x: int) -> int => x * 2\ny = double(21)");
    assert!(rust.contains("pub fn double"), "output was:\n{rust}");
    assert!(rust.contains("-> i64"), "output was:\n{rust}");
}
