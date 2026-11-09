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
fn clone_injected_when_value_used_later() {
    let code = "passthrough(x: str) -> str =>\n    return x\na = \"one\"\nb = passthrough(a)\nc = a";
    let rs = transpile(code);
    assert!(rs.contains("passthrough(a.clone())"), "output was:\n{rs}");
}

#[test]
fn clone_absent_on_last_use() {
    let code = "passthrough(x: str) -> str =>\n    return x\na = \"one\"\nb = passthrough(a)";
    let rs = transpile(code);
    assert!(rs.contains("passthrough(a)"), "output was:\n{rs}");
    assert!(!rs.contains(".clone()"), "output was:\n{rs}");
}

#[test]
fn clone_only_first_of_two_calls() {
    let code = "passthrough(x: str) -> str =>\n    return x\na = \"one\"\nb = passthrough(a)\nd = passthrough(a)";
    let rs = transpile(code);
    assert!(rs.contains("passthrough(a.clone())"), "output was:\n{rs}");
    assert!(rs.contains("passthrough(a);"), "output was:\n{rs}");
    assert_eq!(rs.matches(".clone()").count(), 1, "output was:\n{rs}");
}

#[test]
fn clone_int_value_param_used_later() {
    let code = "ident(x: int) -> int =>\n    return x\nm = 5\nn = ident(m)\np = m + 1";
    let rs = transpile(code);
    assert!(rs.contains("ident(m.clone())"), "output was:\n{rs}");
}

#[test]
fn clone_only_for_var_used_later() {
    let code = "id2(x: str) -> str =>\n    return x\na = \"A\"\nb = \"B\"\nc = id2(a)\nd = id2(b)\ne = a";
    let rs = transpile(code);
    assert!(rs.contains("id2(a.clone())"), "output was:\n{rs}");
    assert!(rs.contains("id2(b)"), "output was:\n{rs}");
    assert!(!rs.contains("id2(b.clone())"), "output was:\n{rs}");
}

#[test]
fn clone_not_used_for_borrowed_param() {
    let code = "peek(x: str) =>\n    x\na = \"A\"\nq = peek(a)\nz = a";
    let rs = transpile(code);
    assert!(rs.contains("peek(&a)"), "output was:\n{rs}");
    assert!(!rs.contains(".clone()"), "output was:\n{rs}");
}
