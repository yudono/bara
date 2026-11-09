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
fn typo_color_maps_to_richtext() {
    let s = CssStyleSheet::parse(".t { color: #38bdf8; }");
    let mods = s.get_rich_text_modifiers(&["t"]).join(" ");
    assert!(mods.contains("from_rgb(56, 189, 248)"), "{mods}");
    let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, ".t { color: #38bdf8; }"));
    assert!(rust.contains("color(egui::Color32::from_rgb(56, 189, 248))"), "{rust}");
}

#[test]
fn typo_font_family_monospace_maps_others_ignored() {
    for fam in ["monospace", "Courier New", "Consolas"] {
        let s = CssStyleSheet::parse(&format!(".t {{ font-family: {fam}; }}"));
        assert!(s.get_rich_text_modifiers(&["t"]).contains(&"monospace()".to_string()), "{fam}");
    }
    let sans = CssStyleSheet::parse(".t { font-family: Inter, sans-serif; }");
    assert!(!sans.get_rich_text_modifiers(&["t"]).contains(&"monospace()".to_string()));
    let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, ".t { font-family: monospace; }"));
    assert!(rust.contains("monospace()"), "{rust}");
}

#[test]
fn typo_font_size_weight_style_mapped() {
    let rust = transpile_ok(&view_shell(
        r#"<Text class="t">hi</Text>"#,
        ".t { font-size: 20px; font-weight: bold; font-style: italic; }",
    ));
    assert!(rust.contains("size(20.0)"), "{rust}");
    assert!(rust.contains("strong()"), "{rust}");
    assert!(rust.contains("italics()"), "{rust}");
    for w in ["700", "800", "900", "bolder"] {
        let s = CssStyleSheet::parse(&format!(".t {{ font-weight: {w}; }}"));
        assert!(s.get_rich_text_modifiers(&["t"]).contains(&"strong()".to_string()), "{w}");
    }
    let normal = CssStyleSheet::parse(".t { font-weight: 400; }");
    assert!(!normal.get_rich_text_modifiers(&["t"]).contains(&"strong()".to_string()));
}

#[test]
fn typo_font_shorthand_parsed_but_ignored() {
    let s = CssStyleSheet::parse(".t { font: italic bold 16px/1.5 sans-serif; }");
    assert_eq!(
        s.get_raw_property(&["t"], "font").as_deref(),
        Some("italic bold 16px/1.5 sans-serif")
    );
    // Shorthand does not expand into font-size/weight/style modifiers.
    assert!(s.get_rich_text_modifiers(&["t"]).is_empty());
    let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, ".t { font: italic bold 16px/1.5 sans-serif; }"));
    assert!(!rust.contains("strong()"), "{rust}");
    assert!(!rust.contains("size("), "{rust}");
}

#[test]
fn typo_font_variant_line_height_ignored() {
    for (prop, v) in [("font-variant", "small-caps"), ("line-height", "1.5"), ("line-height", "24px")] {
        let s = CssStyleSheet::parse(&format!(".t {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["t"], prop).as_deref(), Some(v));
        assert!(s.get_rich_text_modifiers(&["t"]).is_empty());
        let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, &format!(".t {{ {prop}: {v}; }}")));
        assert!(!rust.contains("size(") || prop != "font-variant", "{rust}");
    }
}

#[test]
fn typo_text_align_parsed_but_ignored() {
    for v in ["left", "center", "right", "justify"] {
        let s = CssStyleSheet::parse(&format!(".t {{ text-align: {v}; }}"));
        assert_eq!(s.get_text_align(&["t"]).as_deref(), Some(v));
        assert!(s.get_rich_text_modifiers(&["t"]).is_empty());
        let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, &format!(".t {{ text-align: {v}; }}")));
        // egui labels are laid out by containers; no text-align emit.
        assert!(!rust.contains("text_align"), "{rust}");
    }
}

#[test]
fn typo_text_decoration_mapped_and_overline_ignored() {
    let rust = transpile_ok(&view_shell(
        r#"<Text class="t">hi</Text>"#,
        ".t { text-decoration: underline line-through; }",
    ));
    assert!(rust.contains("underline()"), "{rust}");
    assert!(rust.contains("strikethrough()"), "{rust}");
    let s = CssStyleSheet::parse(".t { text-decoration: overline; }");
    assert!(s.get_rich_text_modifiers(&["t"]).is_empty());
}

#[test]
fn typo_transform_indent_shadow_overflow_ignored() {
    for (prop, v) in [
        ("text-transform", "uppercase"),
        ("text-transform", "capitalize"),
        ("text-indent", "16px"),
        ("text-shadow", "1px 1px 2px black"),
        ("text-overflow", "ellipsis"),
    ] {
        let s = CssStyleSheet::parse(&format!(".t {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["t"], prop).as_deref(), Some(v));
        assert!(s.get_rich_text_modifiers(&["t"]).is_empty());
        let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, &format!(".t {{ {prop}: {v}; }}")));
        assert!(!rust.contains("strikethrough()"), "{prop} {rust}");
        assert!(!rust.contains("underline()"), "{prop} {rust}");
    }
}

#[test]
fn typo_spacing_white_break_writing_ignored() {
    for (prop, v) in [
        ("letter-spacing", "2px"),
        ("word-spacing", "4px"),
        ("white-space", "nowrap"),
        ("white-space", "pre-wrap"),
        ("word-break", "break-all"),
        ("overflow-wrap", "break-word"),
        ("writing-mode", "vertical-rl"),
    ] {
        let s = CssStyleSheet::parse(&format!(".t {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["t"], prop).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, &format!(".t {{ {prop}: {v}; }}")));
        // No egui RichText tracking / wrapping-mode mapping; ignored.
        assert!(syn::parse_file(&rust).is_ok());
        assert!(!rust.contains("monospace()") || prop == "letter-spacing" && false, "{prop} {rust}");
    }
    // white-space:nowrap has no wrap=false emit; label still plain.
    let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, ".t { white-space: nowrap; }"));
    assert!(rust.contains("label(egui::RichText::new"), "{rust}");
}

#[test]
fn typo_no_text_props_yields_empty_modifiers() {
    let s = CssStyleSheet::parse(".b { background: #ffffff; padding: 8px; line-height: 2; text-align: center; }");
    // line-height/text-align are ignored, background/padding are not text props.
    assert!(s.get_rich_text_modifiers(&["b"]).is_empty());
}
