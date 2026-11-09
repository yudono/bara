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
fn scope_reads_field_as_self() {
    let code = r#"
type Rectangle =
    width: int
    height: int

    area() -> int =>
        return width * height
"#;
    let rs = transpile(code);
    assert!(rs.contains("pub fn area(&self) -> i64"), "output was:\n{rs}");
    assert!(rs.contains("(self.width * self.height)"), "output was:\n{rs}");
}

#[test]
fn scope_writes_field_as_self_mut() {
    let code = r#"
type Counter =
    count: int = 0
    step: int = 1

    increment() =>
        count = count + step
"#;
    let rs = transpile(code);
    assert!(rs.contains("pub fn increment(&mut self)"), "output was:\n{rs}");
    assert!(
        rs.contains("self.count = (self.count + self.step);"),
        "output was:\n{rs}"
    );
}

#[test]
fn scope_param_shadows_field() {
    let code = r#"
type Crate =
    val: int = 0

    pick(val: int) -> int =>
        return val + 1
"#;
    let rs = transpile(code);
    assert!(rs.contains("val: i64"), "output was:\n{rs}");
    assert!(rs.contains("return (val + 1);"), "output was:\n{rs}");
    assert!(!rs.contains("self.val"), "output was:\n{rs}");
}

#[test]
fn scope_for_item_shadows_field() {
    let code = r#"
type Basket =
    item: int = 0
    total: int = 0

    sum(items: [int]) -> int =>
        for item in items =>
            total = total + item
        return total
"#;
    let rs = transpile(code);
    assert!(rs.contains("for item in items"), "output was:\n{rs}");
    assert!(rs.contains("self.total"), "output was:\n{rs}");
    assert!(!rs.contains("self.item"), "output was:\n{rs}");
    assert!(rs.contains("&mut self"), "output was:\n{rs}");
}

#[test]
fn scope_method_call_becomes_self_call() {
    let code = r#"
type MathHelper =
    square(x: int) -> int =>
        return x * x

    cube(x: int) -> int =>
        return square(x) * x
"#;
    let rs = transpile(code);
    assert!(rs.contains("self.square(x)"), "output was:\n{rs}");
}

#[test]
fn scope_field_in_interpolated_string() {
    let code = r#"
type Player =
    name: str
    hp = 100

    hit(dmg: int) =>
        hp -= dmg
        if hp <= 0 => print("{name} defeated")

    is_alive() => hp > 0
"#;
    let rs = transpile(code);
    assert!(rs.contains("pub fn hit(&mut self, dmg: i64)"), "output was:\n{rs}");
    assert!(rs.contains("self.hp -= dmg;"), "output was:\n{rs}");
    assert!(rs.contains("self.name"), "output was:\n{rs}");
    assert!(rs.contains("pub fn is_alive(&self)"), "output was:\n{rs}");
}
