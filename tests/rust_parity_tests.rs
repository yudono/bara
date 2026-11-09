use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile(input: &str) -> String {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    let mut program = Parser::new(tokens).parse_program().expect("parser failed");
    let mut analyzer = Analyzer::new();
    analyzer
        .analyze_program(&mut program)
        .expect("analyzer failed");
    let rust_code = Codegen::new(&analyzer).generate(&program);
    syn::parse_file(&rust_code).expect("generated Rust must be valid (syn)");
    rust_code
}

#[test]
fn test_parity_enum_simple() {
    let rs = transpile(
        r#"
enum Color =
    Red
    Green
    Blue
"#,
    );
    assert!(rs.contains("pub enum Color"), "Output was:\n{rs}");
    assert!(rs.contains("Red"), "Output was:\n{rs}");
    assert!(rs.contains("Green"), "Output was:\n{rs}");
    assert!(rs.contains("Blue"), "Output was:\n{rs}");
}

#[test]
fn test_parity_enum_with_fields() {
    let rs = transpile(
        r#"
enum Shape =
    Circle(float)
    Rect(int, int)
    Point
"#,
    );
    assert!(rs.contains("pub enum Shape"), "Output was:\n{rs}");
    assert!(rs.contains("Circle(f64)"), "Output was:\n{rs}");
    assert!(rs.contains("Rect(i64, i64)"), "Output was:\n{rs}");
    assert!(rs.contains("Point"), "Output was:\n{rs}");
}

#[test]
fn test_parity_match_expr_int_arms() {
    let rs = transpile(
        r#"
x = 2
desc = match x { 1 => "one", 2 => "two" }
print(desc)
"#,
    );
    assert!(rs.contains("match x"), "Output was:\n{rs}");
    assert!(rs.contains("1 =>"), "Output was:\n{rs}");
    assert!(rs.contains("2 =>"), "Output was:\n{rs}");
}

#[test]
fn test_parity_match_expr_enum_paths() {
    let rs = transpile(
        r#"
enum Color =
    Red
    Green
    Blue

c = Color::Red
desc = match c { Color::Red => "red", Color::Green => "green", Color::Blue => "blue" }
print(desc)
"#,
    );
    assert!(rs.contains("pub enum Color"), "Output was:\n{rs}");
    assert!(rs.contains("match c"), "Output was:\n{rs}");
    assert!(rs.contains("Color::Red =>"), "Output was:\n{rs}");
    assert!(rs.contains("Color::Blue =>"), "Output was:\n{rs}");
}

#[test]
fn test_parity_match_stmt() {
    let rs = transpile(
        r#"
x = 3
match x { 1 => print("one"), 2 => print("two"), 3 => print("three") }
"#,
    );
    assert!(rs.contains("match x"), "Output was:\n{rs}");
    assert!(rs.contains("println!"), "Output was:\n{rs}");
}

#[test]
fn test_parity_match_guard() {
    let rs = transpile(
        r#"
x = 5
desc = match x { 1 if x > 0 => "one", 2 => "two" }
print(desc)
"#,
    );
    assert!(rs.contains("match x"), "Output was:\n{rs}");
    assert!(rs.contains("if (x > 0)"), "Output was:\n{rs}");
}

#[test]
fn test_parity_pub_export() {
    let rs = transpile(
        r#"
pub double(x: int) -> int => x * 2
y = double(21)
print(y)
"#,
    );
    assert!(rs.contains("pub fn double"), "Output was:\n{rs}");
    assert!(rs.contains("-> i64"), "Output was:\n{rs}");
}

#[test]
fn test_parity_mod_decl() {
    let rs = transpile("pub mod utils\n");
    assert!(rs.contains("pub mod utils;"), "Output was:\n{rs}");
}

#[test]
fn test_parity_generic_option() {
    let rs = transpile(
        r#"
maybe(x: Option<int>) -> Option<int> => x
print("ok")
"#,
    );
    assert!(rs.contains("Option<"), "Output was:\n{rs}");
    assert!(rs.contains("Option<i64>"), "Output was:\n{rs}");
}

#[test]
fn test_parity_generic_result() {
    let rs = transpile(
        r#"
fetch(ok: bool) -> Result<str, str> =>
    if ok => return "data"
    return "err"
print("ok")
"#,
    );
    assert!(rs.contains("Result<"), "Output was:\n{rs}");
    assert!(rs.contains("Result<String, String>"), "Output was:\n{rs}");
}

#[test]
fn test_parity_generic_vec() {
    let rs = transpile(
        r#"
sum_all(xs: [int]) -> int =>
    total = 0
    i = 0
    while i < xs.len() =>
        total = total + xs[i]
        i = i + 1
    return total
print(sum_all([1, 2, 3]))
"#,
    );
    assert!(rs.contains("Vec<"), "Output was:\n{rs}");
    assert!(rs.contains("Vec<i64>"), "Output was:\n{rs}");
}

#[test]
fn test_parity_tuple_and_unit() {
    let rs = transpile(
        r#"
first(t: (int, str)) -> int => 1
nop() -> () => return
print(first((1, "one")))
nop()
"#,
    );
    assert!(rs.contains("(i64, String)"), "Output was:\n{rs}");
    assert!(rs.contains("-> ()"), "Output was:\n{rs}");
}

#[test]
fn test_parity_references() {
    let rs = transpile(
        r#"
peek(x: &int) -> int => x
bump(x: &mut int) => x = x + 1
n = 1
bump(n)
print(peek(1))
"#,
    );
    assert!(rs.contains("&i64"), "Output was:\n{rs}");
    assert!(rs.contains("&mut"), "Output was:\n{rs}");
}

#[test]
fn test_parity_use_path_and_alias() {
    let rs = transpile(
        r#"
use std::collections::HashMap
use std::collections::HashMap as Map
m = HashMap()
n = Map()
print("ok")
"#,
    );
    assert!(
        rs.contains("use std::collections::HashMap;"),
        "Output was:\n{rs}"
    );
    assert!(
        rs.contains("use std::collections::HashMap as Map;"),
        "Output was:\n{rs}"
    );
}

#[test]
fn test_parity_struct_methods_self() {
    let rs = transpile(
        r#"
type Counter =
    count: int = 0

    get() -> int =>
        return count

    bump() =>
        count = count + 1

c = Counter(count = 1)
g = c.get()
c.bump()
"#,
    );
    assert!(rs.contains("pub struct Counter"), "Output was:\n{rs}");
    assert!(rs.contains("pub fn get(&self) -> i64"), "Output was:\n{rs}");
    assert!(rs.contains("pub fn bump(&mut self)"), "Output was:\n{rs}");
    assert!(rs.contains("self.count"), "Output was:\n{rs}");
}
