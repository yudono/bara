use bara::analyzer::Analyzer;
use bara::ast::Item;
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

// ---------- A. Parsing ----------

#[test]
fn css_parses_hex_rgb_rgba_named_colors() {
    for (input, expect_some) in [
        ("#f00", true),
        ("#ff0000", true),
        ("#ff000080", true),
        ("rgb(255, 0, 0)", true),
        ("rgba(255, 0, 0, 0.5)", true),
        ("red", true),
        ("ReD", true),
        ("transparent", true),
        ("notacolor", false),
        ("", false),
    ] {
        assert_eq!(
            bara::css_engine::Color::parse(input).is_some(),
            expect_some,
            "color {input:?}"
        );
    }
}

#[test]
fn css_parses_size_units() {
    let cases = [("16px", 16.0), ("1.5rem", 24.0), ("2em", 32.0), ("10", 10.0)];
    for (input, want) in cases {
        let got = CssStyleSheet::parse_size(input).expect(input);
        assert!((got - want).abs() < 0.01, "{input} -> {got}");
    }
    assert!(CssStyleSheet::parse_size("abc").is_none());
}

#[test]
fn css_parses_margin_shorthands() {
    let m1 = bara::css_engine::Margin::parse("8px").unwrap();
    assert_eq!((m1.top, m1.right, m1.bottom, m1.left), (8.0, 8.0, 8.0, 8.0));
    let m2 = bara::css_engine::Margin::parse("4px 8px").unwrap();
    assert_eq!((m2.top, m2.right, m2.bottom, m2.left), (4.0, 8.0, 4.0, 8.0));
    let m4 = bara::css_engine::Margin::parse("1px 2px 3px 4px").unwrap();
    assert_eq!((m4.top, m4.right, m4.bottom, m4.left), (1.0, 2.0, 3.0, 4.0));
    assert!(bara::css_engine::Margin::parse("").is_none());
}

#[test]
fn css_parses_border_shorthand_and_opacity() {
    let sheet = CssStyleSheet::parse(".b { border: 2px solid #ff0000; opacity: 0.5; }");
    let code = sheet.get_frame_code(&["b"]).expect("frame");
    assert!(code.contains("stroke(egui::Stroke::new(2.0_f32,"), "{code}");
    assert!(code.contains("from_rgba_premultiplied(255, 0, 0, 127)"), "{code}");
    // Invalid declarations are ignored, never fatal.
    let sheet2 = CssStyleSheet::parse(".b { border: nonsense; padding: nope; }");
    assert!(sheet2.get_frame_code(&["b"]).is_none());
}

// ---------- B. Cascade ----------

#[test]
fn css_multi_class_selectors_require_all_classes() {
    let sheet = CssStyleSheet::parse(".a.b { color: #ffffff; }");
    assert!(sheet.get_rich_text_modifiers(&["a"]).is_empty());
    assert!(sheet.get_rich_text_modifiers(&["b"]).is_empty());
    assert!(!sheet.get_rich_text_modifiers(&["a", "b"]).is_empty());
}

#[test]
fn css_later_rules_win_and_merge() {
    let sheet = CssStyleSheet::parse(
        ".t { color: #ff0000; font-size: 12px; } .t { color: #00ff00; }",
    );
    let mods = sheet.get_rich_text_modifiers(&["t"]).join(" ");
    assert!(mods.contains("from_rgb(0, 255, 0)"), "{mods}");
    assert!(mods.contains("size(12.0)"), "{mods}");
}

#[test]
fn css_unmatched_classes_yield_no_code() {
    let sheet = CssStyleSheet::parse(".t { color: #ffffff; background: #000000; }");
    assert!(sheet.get_frame_code(&["other"]).is_none());
    assert!(sheet.get_rich_text_modifiers(&["other"]).is_empty());
}

// ---------- C. Frame mapping ----------

#[test]
fn css_background_maps_to_fill() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { background: #111827; }",
    ));
    assert!(rust.contains("fill(egui::Color32::from_rgb(17, 24, 39))"), "{rust}");
}

#[test]
fn css_background_color_alias_works() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { background-color: #0b0f19; }",
    ));
    assert!(rust.contains("fill(egui::Color32::from_rgb(11, 15, 25))"), "{rust}");
}

#[test]
fn css_padding_maps_to_inner_margin() {
    let rust = transpile_ok(&view_shell(
        r#"<Col class="c"><Text class="t">hi</Text></Col>"#,
        ".c { padding: 24px; }",
    ));
    assert!(rust.contains("inner_margin(egui::Margin::same(24.0))"), "{rust}");
}

#[test]
fn css_margin_maps_to_outer_margin() {
    let rust = transpile_ok(&view_shell(
        r#"<Col class="c"><Text class="t">hi</Text></Col>"#,
        ".c { margin: 4px 8px; }",
    ));
    assert!(rust.contains("outer_margin"), "{rust}");
    assert!(rust.contains("egui::Margin"), "{rust}");
}

#[test]
fn css_border_radius_maps_to_rounding() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { border-radius: 8px; }",
    ));
    assert!(rust.contains("rounding(egui::Rounding::same(8.0))"), "{rust}");
}

#[test]
fn css_border_pair_maps_to_stroke() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { border-width: 2px; border-color: #1f2937; }",
    ));
    assert!(rust.contains("stroke(egui::Stroke::new(2.0_f32,"), "{rust}");
}

#[test]
fn css_box_shadow_none_maps() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { background: #111827; box-shadow: none; }",
    ));
    assert!(rust.contains("shadow(egui::epaint::Shadow::NONE)"), "{rust}");
}

// ---------- D. Text mapping ----------

#[test]
fn css_text_color_maps() {
    let rust = transpile_ok(&view_shell(
        r#"<Text class="t">hi</Text>"#,
        ".t { color: #f8fafc; }",
    ));
    assert!(rust.contains("color(egui::Color32::from_rgb(248, 250, 252))"), "{rust}");
}

#[test]
fn css_font_size_maps() {
    let rust = transpile_ok(&view_shell(
        r#"<Text class="t">hi</Text>"#,
        ".t { font-size: 22px; }",
    ));
    assert!(rust.contains("size(22.0)"), "{rust}");
}

#[test]
fn css_font_weight_bold_maps() {
    let rust = transpile_ok(&view_shell(
        r#"<Text class="t">hi</Text>"#,
        ".t { font-weight: bold; }",
    ));
    assert!(rust.contains("strong()"), "{rust}");
}

#[test]
fn css_font_style_and_decoration_map() {
    let rust = transpile_ok(&view_shell(
        r#"<Text class="t">hi</Text>"#,
        ".t { font-style: italic; text-decoration: underline line-through; }",
    ));
    assert!(rust.contains("italics()"), "{rust}");
    assert!(rust.contains("underline()"), "{rust}");
    assert!(rust.contains("strikethrough()"), "{rust}");
}

#[test]
fn css_monospace_family_maps() {
    let rust = transpile_ok(&view_shell(
        r#"<Text class="t">hi</Text>"#,
        ".t { font-family: monospace; }",
    ));
    assert!(rust.contains("monospace()"), "{rust}");
}

// ---------- E. Layout mapping ----------

#[test]
fn css_gap_maps_to_item_spacing() {
    let rust = transpile_ok(&view_shell(
        r#"<Col class="c"><Text class="t">hi</Text></Col>"#,
        ".c { gap: 16px; }",
    ));
    assert!(rust.contains("item_spacing.y = 16.0"), "{rust}");
    let rust_row = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { gap: 10px; }",
    ));
    assert!(rust_row.contains("item_spacing.x = 10.0"), "{rust_row}");
}

#[test]
fn css_max_width_height_map() {
    let rust = transpile_ok(&view_shell(
        r#"<Col class="c"><Text class="t">hi</Text></Col>"#,
        ".c { max-width: 240px; max-height: 300px; }",
    ));
    assert!(rust.contains("set_max_width(240.0)"), "{rust}");
    assert!(rust.contains("set_max_height(300.0)"), "{rust}");
}

#[test]
fn css_exact_width_height_map_to_set_size() {
    let rust = transpile_ok(&view_shell(
        r#"<Col class="c"><Text class="t">hi</Text></Col>"#,
        ".c { width: 200px; height: 100px; }",
    ));
    assert!(rust.contains("set_width(200.0)"), "{rust}");
    assert!(rust.contains("set_height(100.0)"), "{rust}");
}

#[test]
fn css_row_align_center_uses_centered() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { align-items: center; }",
    ));
    assert!(rust.contains("horizontal_centered"), "{rust}");
}

#[test]
fn css_row_space_between_uses_main_justify() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="a">x</Text><Text class="b">y</Text></Row>"#,
        ".r { justify-content: space-between; }",
    ));
    assert!(rust.contains("with_main_justify(true)"), "{rust}");
    assert!(rust.contains("left_to_right"), "{rust}");
}

#[test]
fn css_col_align_center_uses_vertical_centered() {
    let rust = transpile_ok(&view_shell(
        r#"<Col class="c"><Text class="t">hi</Text></Col>"#,
        ".c { align-items: center; }",
    ));
    assert!(rust.contains("vertical_centered"), "{rust}");
}

#[test]
fn css_wrap_renders_wrapping_row() {
    let rust = transpile_ok(&view_shell(
        r#"<Wrap class="w"><Text class="t">hi</Text></Wrap>"#,
        ".w { gap: 8px; }",
    ));
    assert!(rust.contains("horizontal_wrapped"), "{rust}");
}

#[test]
fn css_scroll_uses_max_height() {
    let rust = transpile_ok(&view_shell(
        r#"<Scroll class="s"><Text class="t">hi</Text></Scroll>"#,
        ".s { height: 350px; }",
    ));
    assert!(rust.contains("max_height(350.0)"), "{rust}");
}

// ---------- F. Widget mapping ----------

#[test]
fn css_button_takes_fill_stroke_rounding() {
    let rust = transpile_ok(&view_shell(
        r#"<Button class="b">Go</Button>"#,
        ".b { background: #0284c7; border: 1px solid #000000; border-radius: 6px; }",
    ));
    assert!(rust.contains(".fill(egui::Color32::from_rgb(2, 132, 199))"), "{rust}");
    assert!(rust.contains(".stroke("), "{rust}");
    assert!(rust.contains(".rounding(egui::Rounding::same(6.0))"), "{rust}");
}

#[test]
fn css_button_size_maps_to_add_sized() {
    let rust = transpile_ok(&view_shell(
        r#"<Button class="b">Go</Button>"#,
        ".b { width: 120px; height: 40px; }",
    ));
    assert!(rust.contains("add_sized"), "{rust}");
    assert!(rust.contains("egui::vec2(120.0, 40.0)"), "{rust}");
}

#[test]
fn css_input_takes_text_color_and_font() {
    let rust = transpile_ok(&view_shell(
        r#"<Input class="fld" bind={title} />"#,
        ".fld { color: #f8fafc; font-size: 14px; background: #1e293b; }",
    ));
    assert!(rust.contains("text_color(egui::Color32::from_rgb(248, 250, 252))"), "{rust}");
    assert!(rust.contains("FontId::proportional(14.0)"), "{rust}");
    assert!(rust.contains("fill(egui::Color32::from_rgb(30, 41, 59))"), "{rust}");
}

#[test]
fn css_opacity_maps_to_set_opacity() {
    let rust = transpile_ok(&view_shell(
        r#"<Text class="t">hi</Text>"#,
        ".t { color: #ffffff; opacity: 0.5; }",
    ));
    assert!(rust.contains("set_opacity(0.500)"), "{rust}");
}

#[test]
fn css_checkbox_slider_separator_emit_widgets() {
    let rust = transpile_ok(&view_shell(
        "<Checkbox bind={title} />",
        ".x { color: #ffffff; }",
    ));
    assert!(rust.contains("checkbox(&mut self.title"), "{rust}");
    let rust2 = transpile_ok(&view_shell(
        "<Separator />",
        ".x { color: #ffffff; }",
    ));
    assert!(rust2.contains("separator()"), "{rust2}");
}
