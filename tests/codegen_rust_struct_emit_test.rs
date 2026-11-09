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
fn struct_field_types_emit() {
    let rs = transpile(
        r#"
type User =
    name: str
    age: int
    score: float
    active: bool
print("ok")
"#,
    );
    assert!(rs.contains("pub struct User {"), "{rs}");
    assert!(rs.contains("pub name: String,"), "{rs}");
    assert!(rs.contains("pub age: i64,"), "{rs}");
    assert!(rs.contains("pub score: f64,"), "{rs}");
    assert!(rs.contains("pub active: bool,"), "{rs}");
}

#[test]
fn struct_default_zero_values() {
    let rs = transpile(
        r#"
type User =
    name: str
    age: int
    score: float
    active: bool
print("ok")
"#,
    );
    assert!(rs.contains("impl Default for User {"), "{rs}");
    assert!(rs.contains("name: String::new(),"), "{rs}");
    assert!(rs.contains("age: 0,"), "{rs}");
    assert!(rs.contains("score: 0.0,"), "{rs}");
    assert!(rs.contains("active: false,"), "{rs}");
}

#[test]
fn struct_explicit_defaults_kept() {
    let rs = transpile(
        r#"
type Player =
    nick: str = "bob"
    hp: int = 100
print("ok")
"#,
    );
    assert!(rs.contains("nick: \"bob\".to_string(),"), "{rs}");
    assert!(rs.contains("hp: 100,"), "{rs}");
}

#[test]
fn struct_constructor_uses_into_for_strings() {
    let rs = transpile(
        r#"
type User =
    name: str
    age: int
print("ok")
"#,
    );
    assert!(rs.contains("pub fn User(name: impl Into<String>, age: i64) -> User"), "{rs}");
    assert!(rs.contains("name: name.into()"), "{rs}");
    assert!(rs.contains("age: age"), "{rs}");
}

#[test]
fn struct_vec_and_map_field_defaults() {
    let rs = transpile(
        r#"
type Bag =
    tags: [str]
    meta: {str: int}
print("ok")
"#,
    );
    assert!(rs.contains("pub tags: Vec<String>,"), "{rs}");
    assert!(rs.contains("pub meta: std::collections::HashMap<String, i64>,"), "{rs}");
    assert!(rs.contains("tags: Vec::new(),"), "{rs}");
    assert!(rs.contains("meta: std::collections::HashMap::new(),"), "{rs}");
}

#[test]
fn struct_instantiation_full_named_args() {
    let rs = transpile(
        r#"
type User =
    name: str
    age: int

u = User(name = "A", age = 3)
print(u)
"#,
    );
    assert!(rs.contains("name: \"A\".to_string(),"), "{rs}");
    assert!(rs.contains("age: 3"), "{rs}");
}

#[test]
fn struct_instantiation_partial_uses_default() {
    let rs = transpile(
        r#"
type User =
    name: str
    age: int

u = User(name = "B")
print(u)
"#,
    );
    assert!(rs.contains("..Default::default()"), "{rs}");
    assert!(rs.contains("name: \"B\".to_string(),"), "{rs}");
}

#[test]
fn struct_instantiation_empty_is_default() {
    let rs = transpile(
        r#"
type User =
    name: str
    age: int

u = User()
print(u)
"#,
    );
    assert!(rs.contains("User::default()"), "{rs}");
}
