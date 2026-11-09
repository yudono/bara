use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::css_engine::CssStyleSheet;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn sheet(css: &str) -> CssStyleSheet {
    CssStyleSheet::parse(css)
}

fn transpile(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().expect("lexing failed");
    let mut program = Parser::new(tokens).parse_program().expect("parsing failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("analysis failed");
    let mut codegen = Codegen::new(&analyzer);
    let rust_code = codegen.generate(&program);
    syn::parse_file(&rust_code).expect("generated Rust must be valid (syn)");
    rust_code
}

#[test]
fn flex_gap_spacing() {
    let s = sheet(".row { gap: 12px; }");
    assert_eq!(s.get_spacing(&["row"]), Some(12.0));
}

#[test]
fn flex_row_and_column_gap_fallbacks() {
    let r = sheet(".v { row-gap: 10px; }");
    assert_eq!(r.get_spacing(&["v"]), Some(10.0));
    let c = sheet(".h { column-gap: 6px; }");
    assert_eq!(c.get_spacing(&["h"]), Some(6.0));
}

#[test]
fn flex_gap_takes_priority_over_row_gap() {
    let s = sheet(".f { gap: 12px; row-gap: 4px; }");
    assert_eq!(s.get_spacing(&["f"]), Some(12.0));
}

#[test]
fn flex_no_gap_yields_none() {
    let s = sheet(".f { padding: 8px; }");
    assert_eq!(s.get_spacing(&["f"]), None);
}

#[test]
fn flex_height_and_max_height() {
    let h = sheet(".a { height: 48px; }");
    assert_eq!(h.get_max_height(&["a"]), Some(48.0));
    let mh = sheet(".b { max-height: 200px; }");
    assert_eq!(mh.get_max_height(&["b"]), Some(200.0));
    let both = sheet(".c { height: 48px; max-height: 200px; }");
    assert_eq!(both.get_max_height(&["c"]), Some(48.0));
}

#[test]
fn flex_width_and_max_width() {
    let w = sheet(".a { width: 320px; }");
    assert_eq!(w.get_max_width(&["a"]), Some(320.0));
    let mw = sheet(".b { max-width: 640px; }");
    assert_eq!(mw.get_max_width(&["b"]), Some(640.0));
}

#[test]
fn flex_frame_fill_background_variants() {
    let bg = sheet(".a { background: #0f172a; }");
    let f1 = bg.get_frame_code(&["a"]).expect("frame");
    assert!(f1.contains("fill(egui::Color32::from_rgb(15, 23, 42))"), "{f1}");
    let bgc = sheet(".b { background-color: #ffffff; }");
    let f2 = bgc.get_frame_code(&["b"]).expect("frame");
    assert!(f2.contains("fill(egui::Color32::from_rgb(255, 255, 255))"), "{f2}");
}

#[test]
fn flex_col_gap_emits_vertical_spacing() {
    let rs = transpile(
        r#"
type Wrap =
    label: str = "hi"

view Wrap =>
    <Panel>
        <Col class="wrap">
            <Text>{label}</Text>
        </Col>
    </Panel>

<style>
.wrap { gap: 12px; }
</style>
"#,
    );
    assert!(rs.contains("item_spacing.y = 12.0;"), "{rs}");
}

#[test]
fn flex_row_gap_emits_horizontal_spacing() {
    let rs = transpile(
        r#"
type Bar =
    label: str = "hi"

view Bar =>
    <Panel>
        <Row class="bar">
            <Text>{label}</Text>
        </Row>
    </Panel>

<style>
.bar { column-gap: 10px; }
</style>
"#,
    );
    assert!(rs.contains("item_spacing.x = 10.0;"), "{rs}");
}

#[test]
fn flex_scroll_custom_height() {
    let rs = transpile(
        r#"
type Feed =
    label: str = "hi"

view Feed =>
    <Panel>
        <Scroll class="area">
            <Text>{label}</Text>
        </Scroll>
    </Panel>

<style>
.area { height: 220px; }
</style>
"#,
    );
    assert!(rs.contains(".max_height(220.0)"), "{rs}");
}
