use bara::debugger::{BaraDebugger, Severity};
use bara::lexer::Lexer;
use bara::parser::Parser;

fn parse_and_diagnose(code: &str) -> Vec<bara::debugger::Diagnostic> {
    let tokens = Lexer::new(code).tokenize().expect("Lexing failed");
    let program = Parser::new(tokens).parse_program().expect("Parsing failed");
    let mut debugger = BaraDebugger::new();
    debugger.inspect(&program, Some("test.br"));
    debugger.diagnostics
}

#[test]
fn test_detect_undeclared_variable() {
    let code = r#"
fn compute(a: int) =>
    b = a + unknown_var
    return b
"#;
    let diags = parse_and_diagnose(code);
    assert!(diags.iter().any(|d| d.code == "E0001" && d.message.contains("unknown_var")));
}

#[test]
fn test_detect_duplicate_type() {
    let code = r#"
type User =
    id: int
    name: str

type User =
    id: int
"#;
    let diags = parse_and_diagnose(code);
    assert!(diags.iter().any(|d| d.code == "E0002" && d.message.contains("User")));
}

#[test]
fn test_detect_duplicate_field_and_method() {
    let code = r#"
type Account =
    balance: int
    balance: int

    deposit(amount: int) =>
        balance = balance + amount

    deposit(amount: int) =>
        balance = balance + amount
"#;
    let diags = parse_and_diagnose(code);
    assert!(diags.iter().any(|d| d.code == "E0003" && d.message.contains("balance")));
    assert!(diags.iter().any(|d| d.code == "E0003" && d.message.contains("deposit")));
}

#[test]
fn test_detect_division_by_zero() {
    let code = r#"
fn calculate(x: int) =>
    y = x / 0
    return y
"#;
    let diags = parse_and_diagnose(code);
    assert!(diags.iter().any(|d| d.code == "W0002" && d.severity == Severity::Warning));
}

#[test]
fn test_detect_unreachable_code() {
    let code = r#"
fn test_dead(x: int) =>
    if x > 10 =>
        return 100
        y = 200
        z = y + 1
    return 0
"#;
    let diags = parse_and_diagnose(code);
    assert!(diags.iter().any(|d| d.code == "W0003" && d.severity == Severity::Warning));
}

#[test]
fn test_detect_unknown_brui_tag() {
    let code = r#"
type MyApp =
    count = 0

view MyApp =>
    <MagicFloatingWindow>
        <Text>Hello</Text>
    </MagicFloatingWindow>
"#;
    let diags = parse_and_diagnose(code);
    assert!(diags.iter().any(|d| d.code == "E0003" && d.message.contains("MagicFloatingWindow")));
}

#[test]
fn test_diagnostic_render_output() {
    let code = "fn test() =>\n    x = missing_val\n";
    let diags = parse_and_diagnose(code);
    let rendered = diags[0].render(code);
    println!("RENDERED:\n{rendered}");
    assert!(rendered.contains("error[E0001]"));
    assert!(rendered.contains("test.br:"));
    assert!(rendered.contains("x = missing_val"));
    assert!(rendered.contains("^"));
}
