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
fn elide_str_ref_has_no_lifetime() {
    let code = "greet(name: str) =>\n    print(\"hi {name}\")\nn = \"Bo\"\ngreet(n)";
    let rs = transpile(code);
    assert!(rs.contains("name: &String"), "output was:\n{rs}");
    assert!(rs.contains("greet(&n)"), "output was:\n{rs}");
    assert!(!rs.contains("&'"), "output was:\n{rs}");
}

#[test]
fn elide_vec_ref_has_no_lifetime() {
    let code = "sum_all(xs: [int]) -> int =>\n    s = 0\n    for x in xs =>\n        s = s + x\n    return s";
    let rs = transpile(code);
    assert!(rs.contains("xs: &Vec<i64>"), "output was:\n{rs}");
    assert!(!rs.contains("&'"), "output was:\n{rs}");
}

#[test]
fn elide_mut_ref_borrow() {
    let code = "push_it(xs: [int], v: int) =>\n    xs.push(v)\na = [1, 2]\npush_it(a, 9)";
    let rs = transpile(code);
    assert!(rs.contains("xs: &mut Vec<i64>"), "output was:\n{rs}");
    assert!(rs.contains("push_it(&mut a, 9)"), "output was:\n{rs}");
    assert!(!rs.contains("&'"), "output was:\n{rs}");
}

#[test]
fn elide_inferred_int_return() {
    let rs = transpile("double_it(x: int) => x * 2");
    assert!(rs.contains("pub fn double_it(x: i64) -> i64"), "output was:\n{rs}");
}

#[test]
fn elide_inferred_bool_return() {
    let rs = transpile("is_big(x: int) => x > 100");
    assert!(rs.contains("pub fn is_big(x: i64) -> bool"), "output was:\n{rs}");
}

#[test]
fn elide_inferred_str_return() {
    let rs = transpile("hello() => \"hi\"");
    assert!(rs.contains("pub fn hello() -> String"), "output was:\n{rs}");
}

#[test]
fn elide_inferred_float_return() {
    let rs = transpile("pi() => 3.14");
    assert!(rs.contains("pub fn pi() -> f64"), "output was:\n{rs}");
}
