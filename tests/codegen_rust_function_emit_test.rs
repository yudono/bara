use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile(input: &str) -> String {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    let mut program = Parser::new(tokens).parse_program().expect("parser failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("analyzer failed");
    let rust_code = Codegen::new(&analyzer).generate(&program);
    syn::parse_file(&rust_code).expect("generated Rust must be valid (syn)");
    rust_code
}

#[test]
fn function_typed_params_and_return() {
    let rs = transpile("double(x: int) -> int => x * 2\nprint(double(21))\n");
    assert!(rs.contains("pub fn double(x: i64) -> i64"), "{rs}");
    assert!(rs.contains("(x * 2)"), "{rs}");
}

#[test]
fn function_untyped_params_default_i64() {
    let rs = transpile("add(a, b) => a + b\nprint(add(1, 2))\n");
    assert!(rs.contains("pub fn add(a: i64, b: i64)"), "{rs}");
}

#[test]
fn function_mut_borrow_inferred() {
    let rs = transpile("push_all(a, b) =>\n    b.push(a)\n    a * 2\nprint(\"ok\")\n");
    assert!(rs.contains("pub fn push_all(a: i64, b: &mut Vec<i64>) -> i64"), "{rs}");
    assert!(rs.contains("b.push(a)"), "{rs}");
}

#[test]
fn function_string_return_with_interpolation() {
    let rs = transpile("greet(name: str) -> str => \"hi {name}\"\nprint(greet(\"bo\"))\n");
    assert!(rs.contains("pub fn greet(name: &String) -> String"), "{rs}");
    assert!(rs.contains("format!(\"hi {}\", name)"), "{rs}");
}

#[test]
fn function_ref_self_getter() {
    let rs = transpile(
        r#"
type Counter =
    count: int = 0

    get() -> int =>
        return count

c = Counter(count = 1)
print(c.get())
"#,
    );
    assert!(rs.contains("pub fn get(&self) -> i64"), "{rs}");
    assert!(rs.contains("self.count"), "{rs}");
}

#[test]
fn function_mut_self_bumper() {
    let rs = transpile(
        r#"
type Counter =
    count: int = 0

    bump() =>
        count = count + 1

c = Counter(count = 1)
c.bump()
print(c)
"#,
    );
    assert!(rs.contains("pub fn bump(&mut self)"), "{rs}");
    assert!(rs.contains("self.count"), "{rs}");
}

#[test]
fn function_try_wraps_return_in_result() {
    let rs = transpile(
        r#"
load_name() -> str =>
    v = fetch_name()!
    return v
fetch_name() -> str => "bob"
print(load_name())
"#,
    );
    assert!(rs.contains("pub fn load_name() -> Result<String, Box<dyn std::error::Error>>"), "{rs}");
    assert!(rs.contains("fetch_name()?"), "{rs}");
}
