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
fn frame_panel_emits_central_panel() {
    let rs = transpile(
        r#"
type Home =
    title: str = "Hi"

view Home =>
    <Panel>
        <Text>{title}</Text>
    </Panel>
"#,
    );
    assert!(rs.contains("egui::CentralPanel::default()"), "{rs}");
    assert!(rs.contains("impl eframe::App for Home"), "{rs}");
}

#[test]
fn frame_panel_with_background_class() {
    let rs = transpile(
        r#"
type Home =
    title: str = "Hi"

view Home =>
    <Panel class="screen">
        <Text>{title}</Text>
    </Panel>

<style>
.screen { background: #090d16; padding: 24px; }
</style>
"#,
    );
    assert!(rs.contains(".frame("), "{rs}");
    assert!(rs.contains("egui::Frame::none()"), "{rs}");
    assert!(rs.contains("egui::Color32::from_rgb(9, 13, 22)"), "{rs}");
    assert!(rs.contains("egui::Margin::same(24.0)"), "{rs}");
}

#[test]
fn frame_col_and_row_containers() {
    let rs = transpile(
        r#"
type Layout =
    a: str = "x"
    b: str = "y"

view Layout =>
    <Panel>
        <Col>
            <Text>{a}</Text>
        </Col>
        <Row>
            <Text>{b}</Text>
        </Row>
    </Panel>
"#,
    );
    assert!(rs.contains("vertical(|ui|"), "{rs}");
    assert!(rs.contains("horizontal(|ui|"), "{rs}");
}

#[test]
fn frame_scroll_default_max_height() {
    let rs = transpile(
        r#"
type Feed =
    label: str = "hi"

view Feed =>
    <Panel>
        <Scroll>
            <Text>{label}</Text>
        </Scroll>
    </Panel>
"#,
    );
    assert!(rs.contains("egui::ScrollArea::vertical()"), "{rs}");
    assert!(rs.contains(".max_height(300.0)"), "{rs}");
}

#[test]
fn frame_separator_emits() {
    let rs = transpile(
        r#"
type Page =
    label: str = "hi"

view Page =>
    <Panel>
        <Col>
            <Text>{label}</Text>
            <Separator />
        </Col>
    </Panel>
"#,
    );
    assert!(rs.contains("ui.separator();"), "{rs}");
}

#[test]
fn frame_button_click_action() {
    let rs = transpile(
        r#"
type Counter =
    n: int = 0

view Counter =>
    <Panel>
        <Button onClick={n = n + 1}>Add</Button>
    </Panel>
"#,
    );
    assert!(rs.contains("egui::Button::new"), "{rs}");
    assert!(rs.contains(".clicked()"), "{rs}");
    assert!(rs.contains("self.n = (self.n + 1);"), "{rs}");
}

#[test]
fn frame_input_binding_with_hint() {
    let rs = transpile(
        r#"
type Form =
    name: str = ""

view Form =>
    <Panel>
        <Input bind={name} placeholder="Hi" />
    </Panel>
"#,
    );
    assert!(rs.contains("egui::TextEdit::singleline(&mut self.name)"), "{rs}");
    assert!(rs.contains(".hint_text(\"Hi\")"), "{rs}");
}

#[test]
fn frame_checkbox_binding() {
    let rs = transpile(
        r#"
type Prefs =
    flag: bool = false

view Prefs =>
    <Panel>
        <Checkbox bind={flag} />
    </Panel>
"#,
    );
    assert!(rs.contains("ui.checkbox(&mut self.flag, \"\");"), "{rs}");
}
