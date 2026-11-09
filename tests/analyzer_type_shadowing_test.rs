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
fn shadow_int_to_str_emits_new_let() {
    let rs = transpile("x = 10\nx = \"hello\"");
    assert_eq!(rs.matches("let ").count(), 2, "output was:\n{rs}");
    assert!(rs.contains("let mut x = 10;"), "output was:\n{rs}");
    assert!(rs.contains("\"hello\".to_string()"), "output was:\n{rs}");
}

#[test]
fn shadow_str_to_int_emits_new_let() {
    let rs = transpile("s = \"a\"\ns = 5");
    assert_eq!(rs.matches("let ").count(), 2, "output was:\n{rs}");
    assert!(rs.contains("let mut s = 5;"), "output was:\n{rs}");
}

#[test]
fn shadow_same_type_reassigns_without_new_let() {
    let rs = transpile("y = 1\ny = 2");
    assert_eq!(rs.matches("let ").count(), 1, "output was:\n{rs}");
    assert!(rs.contains("y = 2;"), "output was:\n{rs}");
}

#[test]
fn shadow_int_to_float_emits_new_let() {
    let rs = transpile("n = 7\nn = 2.5");
    assert_eq!(rs.matches("let ").count(), 2, "output was:\n{rs}");
    assert!(rs.contains("let mut n = 7;"), "output was:\n{rs}");
    assert!(rs.contains("2.5"), "output was:\n{rs}");
}

#[test]
fn shadow_int_to_bool_emits_new_let() {
    let rs = transpile("f = 1\nf = true");
    assert_eq!(rs.matches("let ").count(), 2, "output was:\n{rs}");
    assert!(rs.contains("let mut f = 1;"), "output was:\n{rs}");
    assert!(rs.contains("true"), "output was:\n{rs}");
}

#[test]
fn shadow_list_to_int_emits_new_let() {
    let rs = transpile("xs = [1, 2]\nxs = 42");
    assert_eq!(rs.matches("let ").count(), 2, "output was:\n{rs}");
    assert!(rs.contains("vec![1, 2]"), "output was:\n{rs}");
    assert!(rs.contains("xs = 42;"), "output was:\n{rs}");
}
