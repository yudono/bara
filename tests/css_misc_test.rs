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
fn misc_list_style_ignored() {
    for css in [
        "list-style: disc inside;",
        "list-style-type: square;",
        "list-style-position: inside;",
        "list-style-image: url('b.png');",
    ] {
        let s = CssStyleSheet::parse(&format!(".l {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["l"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Col class="l"><Text class="t">hi</Text></Col>"#,
            &format!(".l {{ {css} }}"),
        ));
        assert!(!rust.contains("list"), "{css} {rust}");
    }
}

#[test]
fn misc_table_borders_ignored() {
    for css in ["border-collapse: collapse;", "border-collapse: separate;", "border-spacing: 8px;"] {
        let s = CssStyleSheet::parse(&format!(".t {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["t"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Col class="t"><Text class="x">hi</Text></Col>"#,
            &format!(".t {{ {css} }}"),
        ));
        assert!(syn::parse_file(&rust).is_ok());
        assert!(!rust.contains("collapse"), "{css} {rust}");
    }
}

#[test]
fn misc_content_ignored() {
    let s = CssStyleSheet::parse(".c { content: \"hi\"; }");
    assert_eq!(s.get_raw_property(&["c"], "content").as_deref(), Some("\"hi\""));
    let rust = transpile_ok(&view_shell(r#"<Text class="c">hi</Text>"#, ".c { content: \"hi\"; }"));
    // Pseudo-element content has no label mapping; children text is used.
    assert!(rust.contains("label(egui::RichText::new"), "{rust}");
}

#[test]
fn misc_object_fit_position_ignored() {
    for css in ["object-fit: cover;", "object-fit: contain;", "object-position: center;"] {
        let s = CssStyleSheet::parse(&format!(".i {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["i"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Image class="i" src="a.png" />"#,
            &format!(".i {{ {css} }}"),
        ));
        // Images render as placeholder labels; no fit/position mapping.
        assert!(rust.contains("[Image:"), "{css} {rust}");
    }
}

#[test]
fn misc_scroll_behavior_overscroll_ignored() {
    for css in [
        "scroll-behavior: smooth;",
        "overscroll-behavior: contain;",
        "overscroll-behavior: none;",
    ] {
        let s = CssStyleSheet::parse(&format!(".s {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["s"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Scroll class="s"><Text class="t">hi</Text></Scroll>"#,
            &format!(".s {{ {css} height: 200px; }}"),
        ));
        assert!(rust.contains("ScrollArea::vertical()"), "{css} {rust}");
        assert!(rust.contains("max_height(200.0)"), "{css} {rust}");
    }
}

#[test]
fn misc_scroll_snap_ignored() {
    for css in [
        "scroll-snap-type: x mandatory;",
        "scroll-snap-align: start;",
        "scroll-snap-stop: always;",
    ] {
        let s = CssStyleSheet::parse(&format!(".s {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["s"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Scroll class="s"><Text class="t">hi</Text></Scroll>"#,
            &format!(".s {{ {css} }}"),
        ));
        assert!(rust.contains("ScrollArea::vertical()"), "{css} {rust}");
    }
}

#[test]
fn misc_unknown_property_never_crashes() {
    let s = CssStyleSheet::parse(".x { frobnicate: 123; not-a-prop: xyz; }");
    assert_eq!(s.get_raw_property(&["x"], "frobnicate").as_deref(), Some("123"));
    assert!(s.get_frame_code(&["x"]).is_none());
    assert!(s.get_rich_text_modifiers(&["x"]).is_empty());
    let rust = transpile_ok(&view_shell(
        r#"<Col class="x"><Text class="t">hi</Text></Col>"#,
        ".x { frobnicate: 123; }",
    ));
    assert!(rust.contains("vertical("), "{rust}");
}
