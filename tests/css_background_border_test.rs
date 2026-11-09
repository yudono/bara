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
fn bg_background_and_color_map_to_fill() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { background: #111827; }",
    ));
    assert!(rust.contains("fill(egui::Color32::from_rgb(17, 24, 39))"), "{rust}");
    let rust2 = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { background-color: #0b0f19; }",
    ));
    assert!(rust2.contains("fill(egui::Color32::from_rgb(11, 15, 25))"), "{rust2}");
}

#[test]
fn bg_image_position_size_repeat_ignored() {
    for (prop, v) in [
        ("background-image", "url('x.png')"),
        ("background-position", "center"),
        ("background-size", "cover"),
        ("background-repeat", "no-repeat"),
    ] {
        let s = CssStyleSheet::parse(&format!(".b {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["b"], prop).as_deref(), Some(v));
        assert!(s.get_frame_code(&["b"]).is_none());
        assert!(s.get_fill_code(&["b"]).is_none());
        let rust = transpile_ok(&view_shell(
            r#"<Row class="b"><Text class="t">hi</Text></Row>"#,
            &format!(".b {{ {prop}: {v}; }}"),
        ));
        assert!(!rust.contains("fill("), "{prop} {rust}");
    }
}

#[test]
fn bg_attachment_blend_clip_ignored() {
    for (prop, v) in [
        ("background-attachment", "fixed"),
        ("background-blend-mode", "multiply"),
        ("background-clip", "text"),
    ] {
        let s = CssStyleSheet::parse(&format!(".b {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["b"], prop).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Row class="b"><Text class="t">hi</Text></Row>"#,
            &format!(".b {{ {prop}: {v}; }}"),
        ));
        assert!(!rust.contains("fill("), "{prop} {rust}");
    }
}

#[test]
fn border_shorthand_and_pair_map_to_stroke() {
    let s = CssStyleSheet::parse(".b { border: 2px solid #3b82f6; }");
    let frame = s.get_frame_code(&["b"]).expect("frame");
    assert!(frame.contains("stroke(egui::Stroke::new(2.0_f32"), "{frame}");
    let rust = transpile_ok(&view_shell(
        r#"<Row class="b"><Text class="t">hi</Text></Row>"#,
        ".b { border-width: 2px; border-color: #1f2937; }",
    ));
    assert!(rust.contains("stroke(egui::Stroke::new(2.0_f32,"), "{rust}");
    let w_only = CssStyleSheet::parse(".b { border-width: 3px; }");
    assert!(w_only.get_frame_code(&["b"]).expect("frame").contains("3.0_f32"));
    let c_only = CssStyleSheet::parse(".b { border-color: #ff0000; }");
    assert!(c_only.get_frame_code(&["b"]).expect("frame").contains("1.0_f32"));
}

#[test]
fn border_side_longhands_ignored_fallback() {
    for (prop, v) in [
        ("border-top", "2px solid #ff0000"),
        ("border-right", "2px solid #ff0000"),
        ("border-bottom", "2px solid #ff0000"),
        ("border-left", "2px solid #ff0000"),
    ] {
        let s = CssStyleSheet::parse(&format!(".b {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["b"], prop).as_deref(), Some(v));
        // Only `border` / `border-width`+`border-color` are wired.
        assert!(s.get_frame_code(&["b"]).is_none(), "{prop}");
        assert!(s.get_stroke_code(&["b"]).is_none(), "{prop}");
        let rust = transpile_ok(&view_shell(
            r#"<Row class="b"><Text class="t">hi</Text></Row>"#,
            &format!(".b {{ {prop}: {v}; }}"),
        ));
        assert!(!rust.contains("stroke("), "{prop} {rust}");
    }
}

#[test]
fn border_style_width_color_edge_cases() {
    // border-style alone has no width/color mapping target.
    let s = CssStyleSheet::parse(".b { border-style: dashed; }");
    assert!(s.get_frame_code(&["b"]).is_none());
    let rust = transpile_ok(&view_shell(
        r#"<Row class="b"><Text class="t">hi</Text></Row>"#,
        ".b { border-style: dashed; border-width: 2px; }",
    ));
    // border-width supplies the stroke; style token is dropped.
    assert!(rust.contains("stroke(egui::Stroke::new(2.0_f32,"), "{rust}");
    let rust2 = transpile_ok(&view_shell(
        r#"<Row class="b"><Text class="t">hi</Text></Row>"#,
        ".b { border-style: solid; }",
    ));
    assert!(!rust2.contains("stroke("), "{rust2}");
}

#[test]
fn border_radius_maps_to_rounding() {
    let s = CssStyleSheet::parse(".r { border-radius: 8px; }");
    assert!(s.get_rounding_code(&["r"]).expect("r").contains("Rounding::same(8.0)"));
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { border-radius: 8px; }",
    ));
    assert!(rust.contains("rounding(egui::Rounding::same(8.0))"), "{rust}");
}

#[test]
fn outline_props_ignored_fallback() {
    for css in ["outline: 2px solid #ff0000;", "outline-offset: 4px;"] {
        let s = CssStyleSheet::parse(&format!(".b {{ {css} }}"));
        assert!(s.get_frame_code(&["b"]).is_none(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Row class="b"><Text class="t">hi</Text></Row>"#,
            &format!(".b {{ {css} }}"),
        ));
        assert!(!rust.contains("stroke("), "{css} {rust}");
    }
}

#[test]
fn shadow_none_maps_other_values_ignored() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { background: #111827; box-shadow: none; }",
    ));
    assert!(rust.contains("shadow(egui::epaint::Shadow::NONE)"), "{rust}");
    let s = CssStyleSheet::parse(".r { background: #111827; box-shadow: 0 1px 2px black; }");
    let frame = s.get_frame_code(&["r"]).expect("frame");
    // Real blur shadows have no egui::Frame equivalent; only `none` is wired.
    assert!(!frame.contains("shadow("), "{frame}");
    assert!(frame.contains("fill("), "{frame}");
}

#[test]
fn button_takes_fill_stroke_rounding() {
    let rust = transpile_ok(&view_shell(
        r#"<Button class="b">Go</Button>"#,
        ".b { background: #0284c7; border: 1px solid #000000; border-radius: 6px; }",
    ));
    assert!(rust.contains(".fill(egui::Color32::from_rgb(2, 132, 199))"), "{rust}");
    assert!(rust.contains(".stroke("), "{rust}");
    assert!(rust.contains(".rounding(egui::Rounding::same(6.0))"), "{rust}");
}
