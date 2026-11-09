use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::css_engine::CssStyleSheet;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile_ok(src: &str) -> String {
    let tokens = Lexer::new(src).tokenize().expect("lex");
    let mut prog = Parser::new(tokens).parse_program().expect("parse");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut prog).expect("analyze");
    let mut codegen = Codegen::new(&analyzer);
    let rust = codegen.generate(&prog);
    syn::parse_file(&rust).expect("syn");
    rust
}

fn view_shell(inner: &str, style: &str) -> String {
    format!(
        r#"
type T =
    title: str = "x"
    n: int = 0

view T =>
    <Panel class="screen">
        {inner}
    </Panel>

<style>
{style}
</style>
"#
    )
}

#[test]
fn box_width_exact_maps_to_set_width() {
    let s = CssStyleSheet::parse(".w { width: 200px; }");
    assert_eq!(s.get_width(&["w"]), Some(200.0));
    assert_eq!(s.get_max_width(&["w"]), Some(200.0));
    let rust = transpile_ok(&view_shell(
        r#"<Col class="w"><Text class="t">hi</Text></Col>"#,
        ".w { width: 200px; }",
    ));
    assert!(rust.contains("set_width(200.0)"), "{rust}");
}

#[test]
fn box_max_width_maps_to_set_max_width() {
    let s = CssStyleSheet::parse(".w { max-width: 240px; }");
    assert_eq!(s.get_max_width(&["w"]), Some(240.0));
    assert_eq!(s.get_width(&["w"]), None);
    let rust = transpile_ok(&view_shell(
        r#"<Col class="w"><Text class="t">hi</Text></Col>"#,
        ".w { max-width: 240px; }",
    ));
    assert!(rust.contains("set_max_width(240.0)"), "{rust}");
}

#[test]
fn box_min_width_parsed_but_ignored_fallback() {
    let s = CssStyleSheet::parse(".w { min-width: 120px; }");
    assert_eq!(s.get_min_width(&["w"]), Some(120.0));
    // No existing getter wires min-width into layout.
    assert_eq!(s.get_width(&["w"]), None);
    assert_eq!(s.get_max_width(&["w"]), None);
    assert!(s.get_frame_code(&["w"]).is_none());
    let rust = transpile_ok(&view_shell(
        r#"<Col class="w"><Text class="t">hi</Text></Col>"#,
        ".w { min-width: 120px; }",
    ));
    assert!(!rust.contains("set_width(120.0)"), "{rust}");
    assert!(!rust.contains("set_max_width(120.0)"), "{rust}");
}

#[test]
fn box_height_exact_and_max_maps() {
    let s = CssStyleSheet::parse(".h { height: 100px; }");
    assert_eq!(s.get_height(&["h"]), Some(100.0));
    assert_eq!(s.get_max_height(&["h"]), Some(100.0));
    let rust = transpile_ok(&view_shell(
        r#"<Col class="h"><Text class="t">hi</Text></Col>"#,
        ".h { height: 100px; }",
    ));
    assert!(rust.contains("set_height(100.0)"), "{rust}");
    let s2 = CssStyleSheet::parse(".h { max-height: 300px; }");
    assert_eq!(s2.get_max_height(&["h"]), Some(300.0));
}

#[test]
fn box_min_height_parsed_but_ignored_fallback() {
    let s = CssStyleSheet::parse(".h { min-height: 80px; }");
    assert_eq!(s.get_min_height(&["h"]), Some(80.0));
    assert_eq!(s.get_height(&["h"]), None);
    assert_eq!(s.get_max_height(&["h"]), None);
    let rust = transpile_ok(&view_shell(
        r#"<Col class="h"><Text class="t">hi</Text></Col>"#,
        ".h { min-height: 80px; }",
    ));
    assert!(!rust.contains("set_height(80.0)"), "{rust}");
}

#[test]
fn box_percent_and_auto_treated_as_unset() {
    let s = CssStyleSheet::parse(".w { width: 50%; height: auto; }");
    assert_eq!(s.get_width(&["w"]), None);
    assert_eq!(s.get_height(&["w"]), None);
    assert_eq!(s.get_max_width(&["w"]), None);
    assert_eq!(s.get_max_height(&["w"]), None);
    let rust = transpile_ok(&view_shell(
        r#"<Col class="w"><Text class="t">hi</Text></Col>"#,
        ".w { width: 50%; height: auto; }",
    ));
    assert!(!rust.contains("set_width("), "{rust}");
    assert!(!rust.contains("set_height("), "{rust}");
}

#[test]
fn box_margin_shorthand_maps_to_outer_margin() {
    let s = CssStyleSheet::parse(".m { margin: 4px 8px; }");
    let frame = s.get_frame_code(&["m"]).expect("frame");
    assert!(frame.contains("outer_margin"), "{frame}");
    let rust = transpile_ok(&view_shell(
        r#"<Col class="m"><Text class="t">hi</Text></Col>"#,
        ".m { margin: 12px; }",
    ));
    assert!(rust.contains("outer_margin"), "{rust}");
}

#[test]
fn box_margin_longhands_parsed_but_ignored() {
    let s = CssStyleSheet::parse(
        ".m { margin-top: 4px; margin-right: 8px; margin-bottom: 12px; margin-left: 16px; }",
    );
    assert_eq!(s.get_margin_top(&["m"]), Some(4.0));
    assert_eq!(s.get_margin_right(&["m"]), Some(8.0));
    assert_eq!(s.get_margin_bottom(&["m"]), Some(12.0));
    assert_eq!(s.get_margin_left(&["m"]), Some(16.0));
    // Codegen only reads `margin` shorthand: longhand-only yields no frame.
    assert!(s.get_frame_code(&["m"]).is_none());
    let rust = transpile_ok(&view_shell(
        r#"<Col class="m"><Text class="t">hi</Text></Col>"#,
        ".m { margin-top: 4px; margin-right: 8px; margin-bottom: 12px; margin-left: 16px; }",
    ));
    assert!(!rust.contains("outer_margin"), "{rust}");
}

#[test]
fn box_effective_margin_merges_shorthand_and_longhand() {
    let s = CssStyleSheet::parse(".m { margin: 8px; margin-top: 20px; }");
    let eff = s.get_effective_margin(&["m"]).expect("effective");
    assert!((eff.top - 20.0).abs() < 0.01);
    assert!((eff.right - 8.0).abs() < 0.01);
    assert!((eff.bottom - 8.0).abs() < 0.01);
    assert!((eff.left - 8.0).abs() < 0.01);
    // Shorthand still drives the frame; override is parsed for future wiring.
    let frame = s.get_frame_code(&["m"]).expect("frame");
    assert!(frame.contains("outer_margin(egui::Margin::same(8.0))"), "{frame}");
}

#[test]
fn box_padding_shorthand_maps_to_inner_margin() {
    let s = CssStyleSheet::parse(".p { padding: 16px; }");
    let frame = s.get_frame_code(&["p"]).expect("frame");
    assert!(frame.contains("inner_margin(egui::Margin::same(16.0))"), "{frame}");
    let rust = transpile_ok(&view_shell(
        r#"<Col class="p"><Text class="t">hi</Text></Col>"#,
        ".p { padding: 16px; }",
    ));
    assert!(rust.contains("inner_margin"), "{rust}");
}

#[test]
fn box_padding_longhands_parsed_but_ignored() {
    let s = CssStyleSheet::parse(
        ".p { padding-top: 4px; padding-right: 8px; padding-bottom: 12px; padding-left: 16px; }",
    );
    assert_eq!(s.get_padding_top(&["p"]), Some(4.0));
    assert_eq!(s.get_padding_right(&["p"]), Some(8.0));
    assert_eq!(s.get_padding_bottom(&["p"]), Some(12.0));
    assert_eq!(s.get_padding_left(&["p"]), Some(16.0));
    assert!(s.get_frame_code(&["p"]).is_none());
    let rust = transpile_ok(&view_shell(
        r#"<Col class="p"><Text class="t">hi</Text></Col>"#,
        ".p { padding-top: 4px; }",
    ));
    assert!(!rust.contains("inner_margin"), "{rust}");
}

#[test]
fn box_effective_padding_merges_shorthand_and_longhand() {
    let s = CssStyleSheet::parse(".p { padding: 8px; padding-left: 24px; }");
    let eff = s.get_effective_padding(&["p"]).expect("effective");
    assert!((eff.left - 24.0).abs() < 0.01);
    assert!((eff.top - 8.0).abs() < 0.01);
}

#[test]
fn box_sizing_parsed_but_ignored() {
    for v in ["border-box", "content-box"] {
        let s = CssStyleSheet::parse(&format!(".b {{ box-sizing: {v}; }}"));
        assert_eq!(s.get_box_sizing(&["b"]).as_deref(), Some(v));
        assert_eq!(s.get_raw_property(&["b"], "box-sizing").as_deref(), Some(v));
        assert!(s.get_frame_code(&["b"]).is_none());
        let rust = transpile_ok(&view_shell(
            r#"<Col class="b"><Text class="t">hi</Text></Col>"#,
            &format!(".b {{ box-sizing: {v}; }}"),
        ));
        // No egui box-model switch exists; parse succeeds, ignored.
        assert!(!rust.contains("box-sizing"), "{rust}");
    }
}

#[test]
fn box_aspect_ratio_parsed_but_ignored() {
    let s = CssStyleSheet::parse(".b { aspect-ratio: 16 / 9; }");
    assert!(s.get_aspect_ratio(&["b"]).is_some());
    assert!(s.get_frame_code(&["b"]).is_none());
    let rust = transpile_ok(&view_shell(
        r#"<Col class="b"><Text class="t">hi</Text></Col>"#,
        ".b { aspect-ratio: 16 / 9; width: 160px; }",
    ));
    assert!(rust.contains("set_width(160.0)"), "{rust}");
    assert!(!rust.contains("aspect"), "{rust}");
}
