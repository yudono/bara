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
fn mut_plain_let_stays_immutable() {
    let rs = transpile("x = 10\ny = \"hi\"");
    assert!(rs.contains("let x = 10;"), "output was:\n{rs}");
    assert!(rs.contains("let y = \"hi\".to_string();"), "output was:\n{rs}");
    assert!(!rs.contains("let mut x"), "output was:\n{rs}");
    assert!(!rs.contains("let mut y"), "output was:\n{rs}");
}

#[test]
fn mut_plus_equals_makes_let_mut() {
    let rs = transpile("score = 10\nscore += 5");
    assert!(rs.contains("let mut score = 10;"), "output was:\n{rs}");
    assert!(rs.contains("score += 5;"), "output was:\n{rs}");
}

#[test]
fn mut_minus_equals_makes_let_mut() {
    let rs = transpile("hp = 100\nhp -= 20");
    assert!(rs.contains("let mut hp = 100;"), "output was:\n{rs}");
    assert!(rs.contains("hp -= 20;"), "output was:\n{rs}");
}

#[test]
fn mut_reassignment_makes_let_mut() {
    let rs = transpile("x = 1\nx = 2");
    assert!(rs.contains("let mut x = 1;"), "output was:\n{rs}");
    assert!(rs.contains("x = 2;"), "output was:\n{rs}");
}

#[test]
fn mut_mutating_call_makes_let_mut() {
    let rs = transpile("arr = [1, 2]\narr.push(3)");
    assert!(rs.contains("let mut arr"), "output was:\n{rs}");
    assert!(rs.contains("arr.push(3)"), "output was:\n{rs}");
}

#[test]
fn mut_method_mutation_detects_mut_self() {
    let code = r#"
type Counter =
    count: int = 0
    step: int = 1

    increment() =>
        count = count + step
"#;
    let rs = transpile(code);
    assert!(rs.contains("pub fn increment(&mut self)"), "output was:\n{rs}");
    assert!(rs.contains("self.count"), "output was:\n{rs}");
}

#[test]
fn mut_readonly_method_detects_ref_self() {
    let code = r#"
type Rectangle =
    width: int
    height: int

    area() -> int =>
        return width * height
"#;
    let rs = transpile(code);
    assert!(rs.contains("pub fn area(&self) -> i64"), "output was:\n{rs}");
    assert!(!rs.contains("&mut self"), "output was:\n{rs}");
}
