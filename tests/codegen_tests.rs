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
    analyzer.analyze_program(&mut prog).expect("analyzer failed");
    let mut codegen = Codegen::new(&analyzer);
    codegen.generate(&prog)
}

#[test]
fn test_codegen_variables_mutability() {
    let input = r#"
name = "Bara"
score = 10
score += 5
"#;
    let rs = transpile(input);
    assert!(rs.contains("let name = \"Bara\".to_string();"), "Output was:\n{rs}");
    assert!(rs.contains("let mut score = 10;"), "Output was:\n{rs}");
    assert!(rs.contains("score += 5;"), "Output was:\n{rs}");
}

#[test]
fn test_codegen_function_and_borrow() {
    let input = r#"
proses(a, b) =>
    b.push(a)
    a * 2
"#;
    let rs = transpile(input);
    assert!(rs.contains("pub fn proses(a: i64, b: &mut Vec<i64>) -> i64"), "Output was:\n{rs}");
    assert!(rs.contains("b.push(a)"), "Output was:\n{rs}");
    assert!(rs.contains("(a * 2)"), "Output was:\n{rs}");
}

#[test]
fn test_codegen_type_struct_impl() {
    let input = r#"
type Player =
    name: str
    hp = 100

    hit(dmg: int) =>
        hp -= dmg
        if hp <= 0 => print("{name} defeated")

    is_alive() => hp > 0
"#;
    let rs = transpile(input);
    assert!(rs.contains("pub struct Player {"), "Output was:\n{rs}");
    assert!(rs.contains("impl Default for Player {"), "Output was:\n{rs}");
    assert!(rs.contains("impl Player {"), "Output was:\n{rs}");
    assert!(rs.contains("pub fn hit(&mut self, dmg: i64) {"), "Output was:\n{rs}");
    assert!(rs.contains("self.hp -= dmg;"), "Output was:\n{rs}");
    assert!(rs.contains("pub fn is_alive(&self) -> bool {"), "Output was:\n{rs}");
}

#[test]
fn test_codegen_heterogeneous_dyn() {
    let input = "data = [1, \"two\", true]";
    let rs = transpile(input);
    assert!(rs.contains("pub enum BaraDyn {"), "Output was:\n{rs}");
    assert!(rs.contains("BaraDyn::Int(1)"), "Output was:\n{rs}");
    assert!(rs.contains("BaraDyn::Str(\"two\".to_string())"), "Output was:\n{rs}");
    assert!(rs.contains("BaraDyn::Bool(true)"), "Output was:\n{rs}");
}

#[test]
fn test_codegen_pipeline_and_interpolation() {
    let input = r#"
double(x: int) -> int => x * 2
val = 10 |> double()
print("Result: {val}")
"#;
    let rs = transpile(input);
    assert!(rs.contains("double(10)"), "Output was:\n{rs}");
    assert!(rs.contains("println!(\"Result: {}\", val)"), "Output was:\n{rs}");
}

#[test]
fn test_codegen_raw_rust() {
    let input = r#"
raw_rust {
    let message = "Raw Rust block executed";
    println!("{}", message);
}
"#;
    let rs = transpile(input);
    assert!(rs.contains("Raw Rust block executed"), "Output was:\n{rs}");
}
