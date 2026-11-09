use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile(input: &str) -> String {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexer failed");
    let mut parser = Parser::new(tokens);
    let mut prog = parser.parse_program().expect("parser failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut prog).expect("analysis failed");
    let mut codegen = Codegen::new(&analyzer);
    let rs = codegen.generate(&prog);
    assert!(
        syn::parse_file(&rs).is_ok(),
        "generated Rust must parse:\n{rs}"
    );
    rs
}

#[test]
fn borrow_str_param_gets_ref() {
    let code = "greet(name: str) =>\n    print(\"hi {name}\")";
    let rs = transpile(code);
    assert!(rs.contains("name: &String"), "output was:\n{rs}");
}

#[test]
fn borrow_vec_param_gets_ref() {
    let code = "sum_all(xs: [int]) -> int =>\n    s = 0\n    for x in xs =>\n        s = s + x\n    return s";
    let rs = transpile(code);
    assert!(rs.contains("xs: &Vec<i64>"), "output was:\n{rs}");
}

#[test]
fn borrow_int_params_not_borrowed() {
    let code = "add(a: int, b: int) -> int => a + b";
    let rs = transpile(code);
    assert!(rs.contains("a: i64"), "output was:\n{rs}");
    assert!(rs.contains("b: i64"), "output was:\n{rs}");
    assert!(!rs.contains("a: &"), "output was:\n{rs}");
    assert!(!rs.contains("b: &"), "output was:\n{rs}");
}

#[test]
fn borrow_float_params_not_borrowed() {
    let code = "mix(a: float, b: float) -> float => a + b";
    let rs = transpile(code);
    assert!(rs.contains("a: f64"), "output was:\n{rs}");
    assert!(rs.contains("b: f64"), "output was:\n{rs}");
    assert!(!rs.contains("a: &"), "output was:\n{rs}");
    assert!(!rs.contains("b: &"), "output was:\n{rs}");
}

#[test]
fn borrow_bool_params_not_borrowed() {
    let code = "both(a: bool, b: bool) -> bool => a and b";
    let rs = transpile(code);
    assert!(rs.contains("a: bool"), "output was:\n{rs}");
    assert!(rs.contains("b: bool"), "output was:\n{rs}");
    assert!(!rs.contains("a: &"), "output was:\n{rs}");
    assert!(!rs.contains("b: &"), "output was:\n{rs}");
    assert!(rs.contains("&&"), "output was:\n{rs}");
}

#[test]
fn borrow_mutating_param_gets_mut_ref() {
    let code = "push_it(xs: [int], v: int) =>\n    xs.push(v)";
    let rs = transpile(code);
    assert!(rs.contains("xs: &mut Vec<i64>"), "output was:\n{rs}");
    assert!(rs.contains("v: i64"), "output was:\n{rs}");
    assert!(!rs.contains("v: &"), "output was:\n{rs}");
}

#[test]
fn borrow_call_site_inserts_borrows() {
    let code = "greet(name: str) =>\n    print(\"hi {name}\")\n\npush_it(xs: [int], v: int) =>\n    xs.push(v)\nn = \"Bo\"\ngreet(n)\na = [1, 2]\npush_it(a, 9)";
    let rs = transpile(code);
    assert!(rs.contains("greet(&n)"), "output was:\n{rs}");
    assert!(rs.contains("push_it(&mut a, 9)"), "output was:\n{rs}");
}
