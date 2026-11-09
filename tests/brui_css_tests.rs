use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::css_engine::{Color, CssStyleSheet, Margin, Stroke};
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile_and_verify(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().expect("Lexing failed");
    let mut program = Parser::new(tokens).parse_program().expect("Parsing failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("Analysis failed");
    let mut codegen = Codegen::new(&analyzer);
    let rust_code = codegen.generate(&program);
    let parsed_syn = syn::parse_file(&rust_code);
    assert!(
        parsed_syn.is_ok(),
        "Generated Rust code failed syn validation:\n{rust_code}\nError: {:?}",
        parsed_syn.err()
    );
    rust_code
}

#[test]
fn test_css_color_parsing_all_formats() {
    // 3-digit hex
    let c3 = Color::parse("#f0a").expect("3-digit hex");
    assert_eq!(c3, Color { r: 255, g: 0, b: 170, a: 255 });

    // 4-digit hex with alpha
    let c4 = Color::parse("#f0a8").expect("4-digit hex");
    assert_eq!(c4, Color { r: 255, g: 0, b: 170, a: 136 });

    // 6-digit hex
    let c6 = Color::parse("#1f2937").expect("6-digit hex");
    assert_eq!(c6, Color { r: 31, g: 41, b: 55, a: 255 });

    // 8-digit hex
    let c8 = Color::parse("#1f293780").expect("8-digit hex");
    assert_eq!(c8, Color { r: 31, g: 41, b: 55, a: 128 });

    // rgb()
    let crgb = Color::parse("rgb(59, 130, 246)").expect("rgb func");
    assert_eq!(crgb, Color { r: 59, g: 130, b: 246, a: 255 });

    // rgba()
    let crgba = Color::parse("rgba(59, 130, 246, 0.5)").expect("rgba func");
    assert_eq!(crgba, Color { r: 59, g: 130, b: 246, a: 127 });

    // named colors
    let c_red = Color::parse("red").expect("red");
    assert_eq!(c_red, Color { r: 239, g: 68, b: 68, a: 255 });

    let c_trans = Color::parse("transparent").expect("transparent");
    assert_eq!(c_trans, Color { r: 0, g: 0, b: 0, a: 0 });
}

#[test]
fn test_css_margin_and_padding_shorthands() {
    // 1-value: all sides
    let m1 = Margin::parse("16px").expect("1-val");
    assert_eq!(m1, Margin { top: 16.0, right: 16.0, bottom: 16.0, left: 16.0 });
    assert_eq!(m1.to_egui_code(), "egui::Margin::same(16.0)");

    // 2-value: vertical horizontal
    let m2 = Margin::parse("8px 16px").expect("2-val");
    assert_eq!(m2, Margin { top: 8.0, right: 16.0, bottom: 8.0, left: 16.0 });

    // 3-value: top horizontal bottom
    let m3 = Margin::parse("5px 15px 25px").expect("3-val");
    assert_eq!(m3, Margin { top: 5.0, right: 15.0, bottom: 25.0, left: 15.0 });

    // 4-value: top right bottom left
    let m4 = Margin::parse("1px 2px 3px 4px").expect("4-val");
    assert_eq!(m4, Margin { top: 1.0, right: 2.0, bottom: 3.0, left: 4.0 });

    // rem units (1rem = 16px)
    let m_rem = Margin::parse("1.5rem").expect("rem");
    assert_eq!(m_rem, Margin { top: 24.0, right: 24.0, bottom: 24.0, left: 24.0 });
}

#[test]
fn test_css_stroke_and_border_parsing() {
    let s1 = Stroke::parse("1px solid #3b82f6").expect("stroke px");
    assert_eq!(s1.width, 1.0);
    assert_eq!(s1.color, Color { r: 59, g: 130, b: 246, a: 255 });

    let s2 = Stroke::parse("2.5rem solid red").expect("stroke rem");
    assert_eq!(s2.width, 40.0);
    assert_eq!(s2.color, Color { r: 239, g: 68, b: 68, a: 255 });
}

#[test]
fn test_css_stylesheet_rule_matching_and_cascading() {
    let css = r#"
.card {
    background: #1e293b;
    padding: 16px;
    border-radius: 8px;
}

.card.highlighted {
    border: 2px solid #3b82f6;
}
"#;
    let sheet = CssStyleSheet::parse(css);

    // Matching single class
    let frame1 = sheet.get_frame_code(&["card"]).expect("card frame");
    assert!(frame1.contains("egui::Color32::from_rgb(30, 41, 59)"));
    assert!(frame1.contains("egui::Margin::same(16.0)"));
    assert!(frame1.contains("egui::Rounding::same(8.0)"));

    // Matching compound classes (.card.highlighted)
    let frame2 = sheet.get_frame_code(&["card", "highlighted"]).expect("highlighted frame");
    assert!(frame2.contains("egui::Stroke::new(2.0"));
    assert!(frame2.contains("egui::Color32::from_rgb(59, 130, 246)"));
}

#[test]
fn test_css_rich_text_modifiers() {
    let css = r#"
.title {
    color: #f8fafc;
    font-size: 20px;
    font-weight: bold;
}

.code_snippet {
    font-family: monospace;
    text-decoration: underline;
}
"#;
    let sheet = CssStyleSheet::parse(css);

    let mods1 = sheet.get_rich_text_modifiers(&["title"]);
    assert!(mods1.contains(&"color(egui::Color32::from_rgb(248, 250, 252))".to_string()));
    assert!(mods1.contains(&"size(20.0)".to_string()));
    assert!(mods1.contains(&"strong()".to_string()));

    let mods2 = sheet.get_rich_text_modifiers(&["code_snippet"]);
    assert!(mods2.contains(&"monospace()".to_string()));
    assert!(mods2.contains(&"underline()".to_string()));
}

#[test]
fn test_css_spacing_and_dimensions() {
    let css = r#"
.flex_row {
    gap: 12px;
    height: 48px;
    width: 320px;
}
"#;
    let sheet = CssStyleSheet::parse(css);
    assert_eq!(sheet.get_spacing(&["flex_row"]), Some(12.0));
    assert_eq!(sheet.get_max_height(&["flex_row"]), Some(48.0));
    assert_eq!(sheet.get_max_width(&["flex_row"]), Some(320.0));
}

#[test]
fn test_brui_view_with_style_block() {
    let code = r#"
type Dashboard =
    title: str = "Bara Dashboard"
    counter: int = 0

<style>
.container {
    background: #0f172a;
    padding: 24px;
    border-radius: 12px;
}
.header {
    color: #38bdf8;
    font-size: 24px;
    font-weight: bold;
}
.btn_primary {
    background: #2563eb;
    color: #ffffff;
}
</style>

view Dashboard =>
    <Panel class="container">
        <Col>
            <Text class="header">{title}</Text>
            <Button class="btn_primary" onClick={counter = counter + 1}>Click Me</Button>
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Dashboard"));
    assert!(rust.contains("impl eframe::App for Dashboard"));
    assert!(rust.contains("egui::Frame::none()"));
    assert!(rust.contains("egui::Color32::from_rgb(15, 23, 42)"));
    assert!(rust.contains("egui::Rounding::same(12.0)"));
    assert!(rust.contains("self.counter = (self.counter + 1);"));
}

#[test]
fn test_brui_nested_layout_hierarchy() {
    let code = r#"
type AppLayout =
    status: str = "Ready"

view AppLayout =>
    <Panel>
        <Col>
            <Row>
                <Text>Header Left</Text>
                <Separator />
                <Text>Header Right</Text>
            </Row>
            <Scroll>
                <Text>Scrollable Content Area</Text>
            </Scroll>
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.vertical(|ui|"));
    assert!(rust.contains("ui.horizontal(|ui|"));
    assert!(rust.contains("egui::ScrollArea::vertical()"));
    assert!(rust.contains(".max_height(300.0)"));
    assert!(rust.contains("ui.separator()"));
}

#[test]
fn test_brui_input_and_controls_binding() {
    let code = r#"
type SettingsForm =
    username: str = ""
    dark_mode: bool = true
    volume: int = 75

view SettingsForm =>
    <Panel>
        <Col>
            <Input bind={username} placeholder="Enter name..." />
            <Checkbox bind={dark_mode} />
            <Slider bind={volume} />
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::TextEdit::singleline(&mut self.username)"));
    assert!(rust.contains(".hint_text(\"Enter name...\")"));
    assert!(rust.contains("ui.checkbox(&mut self.dark_mode, \"\");"));
    assert!(rust.contains("ui.add(egui::Slider::new(&mut self.volume, 0..=100));"));
}

#[test]
fn test_brui_conditional_and_list_rendering() {
    let code = r#"
type TodoApp =
    todos: [str] = []
    is_admin: bool = false

view TodoApp =>
    <Panel>
        <Col>
            {if is_admin =>
                <Text>Admin Dashboard</Text>
            else =>
                <Text>User View</Text>
            }
            {for t in todos =>
                <Row>
                    <Text>{t}</Text>
                </Row>
            }
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("if self.is_admin {"));
    assert!(rust.contains("for t in &mut self.todos {"));
}
