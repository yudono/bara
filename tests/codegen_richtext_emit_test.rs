use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().expect("lexing failed");
    let mut program = Parser::new(tokens).parse_program().expect("parsing failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("analysis failed");
    let rust_code = Codegen::new(&analyzer).generate(&program);
    syn::parse_file(&rust_code).expect("generated Rust must be valid (syn)");
    rust_code
}

#[test]
fn richtext_plain_label() {
    let rs = transpile(
        r#"
type Card =
    dummy: str = "x"

view Card =>
    <Panel>
        <Text>Hi</Text>
    </Panel>
"#,
    );
    assert!(rs.contains("egui::RichText::new(\"Hi\")"), "{rs}");
}

#[test]
fn richtext_full_modifier_chain() {
    let rs = transpile(
        r#"
type Card =
    dummy: str = "x"

view Card =>
    <Panel>
        <Text class="t">Hi</Text>
    </Panel>

<style>
.t { color: #123456; font-size: 18px; font-weight: bold; font-style: italic; text-decoration: underline; font-family: monospace; }
</style>
"#,
    );
    assert!(rs.contains(".color(egui::Color32::from_rgb(18, 52, 86))"), "{rs}");
    assert!(rs.contains(".size(18.0)"), "{rs}");
    assert!(rs.contains(".strong()"), "{rs}");
    assert!(rs.contains(".italics()"), "{rs}");
    assert!(rs.contains(".underline()"), "{rs}");
    assert!(rs.contains(".monospace()"), "{rs}");
}

#[test]
fn richtext_strikethrough_modifier() {
    let rs = transpile(
        r#"
type Card =
    dummy: str = "x"

view Card =>
    <Panel>
        <Text class="old">Gone</Text>
    </Panel>

<style>
.old { text-decoration: line-through; }
</style>
"#,
    );
    assert!(rs.contains(".strikethrough()"), "{rs}");
}

#[test]
fn richtext_button_label_modifiers() {
    let rs = transpile(
        r#"
type Cta =
    n: int = 0

view Cta =>
    <Panel>
        <Button class="btn" onClick={n = n + 1}>Go</Button>
    </Panel>

<style>
.btn { color: #ffffff; font-weight: bold; }
</style>
"#,
    );
    assert!(rs.contains("egui::Button::new("), "{rs}");
    assert!(rs.contains("egui::RichText::new(\"Go\")"), "{rs}");
    assert!(rs.contains(".strong()"), "{rs}");
    assert!(rs.contains("egui::Color32::from_rgb(255, 255, 255)"), "{rs}");
}

#[test]
fn richtext_plain_button_has_no_modifier_chain() {
    let rs = transpile(
        r#"
type Cta =
    n: int = 0

view Cta =>
    <Panel>
        <Button onClick={n = n + 1}>Go</Button>
    </Panel>
"#,
    );
    assert!(rs.contains("RichText::new(\"Go\"))"), "{rs}");
}

#[test]
fn richtext_interpolation_uses_format() {
    let rs = transpile(
        r#"
type Card =
    name: str = "Ada"

view Card =>
    <Panel>
        <Text>{name}</Text>
    </Panel>
"#,
    );
    assert!(rs.contains("format!(\"{}\", self.name)"), "{rs}");
}

#[test]
fn richtext_mixed_static_and_expr() {
    let rs = transpile(
        r#"
type Card =
    name: str = "Ada"

view Card =>
    <Panel>
        <Text class="t">Halo {name}</Text>
    </Panel>

<style>
.t { font-size: 16px; }
</style>
"#,
    );
    assert!(rs.contains("format!("), "{rs}");
    assert!(rs.contains("self.name"), "{rs}");
    assert!(rs.contains(".size(16.0)"), "{rs}");
}
