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
fn layout_display_parsed_but_ignored() {
    for v in ["block", "flex", "grid", "none", "inline-block"] {
        let s = CssStyleSheet::parse(&format!(".d {{ display: {v}; }}"));
        assert_eq!(s.get_display(&["d"]).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Col class="d"><Text class="t">hi</Text></Col>"#,
            &format!(".d {{ display: {v}; }}"),
        ));
        // Tag (Row/Col/Panel) selects the egui layout; display is ignored.
        assert!(!rust.contains("display"), "{rust}");
    }
}

#[test]
fn layout_visibility_parsed_but_ignored() {
    for v in ["visible", "hidden", "collapse"] {
        let s = CssStyleSheet::parse(&format!(".d {{ visibility: {v}; }}"));
        assert_eq!(s.get_visibility(&["d"]).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Text class="d">hi</Text>"#,
            &format!(".d {{ visibility: {v}; }}"),
        ));
        // No egui visible/hidden toggle wired; hidden still emits label.
        assert!(rust.contains("label(egui::RichText::new"), "{rust}");
    }
}

#[test]
fn layout_overflow_parsed_scroll_still_vertical() {
    for v in ["hidden", "scroll", "auto", "visible"] {
        let s = CssStyleSheet::parse(&format!(".s {{ overflow: {v}; }}"));
        assert_eq!(s.get_overflow(&["s"]).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Scroll class="s"><Text class="t">hi</Text></Scroll>"#,
            &format!(".s {{ overflow: {v}; height: 200px; }}"),
        ));
        // Scroll always emits vertical area with max_height; overflow value ignored.
        assert!(rust.contains("ScrollArea::vertical()"), "{rust}");
        assert!(rust.contains("max_height(200.0)"), "{rust}");
    }
}

#[test]
fn layout_overflow_xy_fallback_to_overflow_getter() {
    let s = CssStyleSheet::parse(".s { overflow-x: hidden; }");
    assert_eq!(s.get_overflow(&["s"]).as_deref(), Some("hidden"));
    let s2 = CssStyleSheet::parse(".s { overflow-y: scroll; }");
    assert_eq!(s2.get_overflow(&["s"]).as_deref(), Some("scroll"));
    let rust = transpile_ok(&view_shell(
        r#"<Scroll class="s"><Text class="t">hi</Text></Scroll>"#,
        ".s { overflow-x: hidden; overflow-y: scroll; height: 150px; }",
    ));
    assert!(rust.contains("ScrollArea::vertical()"), "{rust}");
}

#[test]
fn layout_position_parsed_but_ignored() {
    for v in ["static", "relative", "absolute", "fixed", "sticky"] {
        let s = CssStyleSheet::parse(&format!(".p {{ position: {v}; }}"));
        assert_eq!(s.get_position(&["p"]).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Col class="p"><Text class="t">hi</Text></Col>"#,
            &format!(".p {{ position: {v}; }}"),
        ));
        assert!(!rust.contains("position"), "{rust}");
    }
}

#[test]
fn layout_offsets_inset_zindex_ignored() {
    let css = ".p { position: absolute; top: 10px; right: 20px; bottom: 30px; left: 40px; inset: 5px; z-index: 10; }";
    let s = CssStyleSheet::parse(css);
    for (prop, want) in [
        ("top", "10px"),
        ("right", "20px"),
        ("bottom", "30px"),
        ("left", "40px"),
        ("inset", "5px"),
        ("z-index", "10"),
    ] {
        assert_eq!(s.get_raw_property(&["p"], prop).as_deref(), Some(want), "{prop}");
    }
    let rust = transpile_ok(&view_shell(r#"<Col class="p"><Text class="t">hi</Text></Col>"#, css));
    // egui has no absolute box offsets / stacking context; all ignored.
    assert!(!rust.contains("z-index"), "{rust}");
    assert!(!rust.contains("available_pos"), "{rust}");
}

#[test]
fn layout_grid_shorthand_ignored() {
    for css in ["grid: 1fr 1fr;", "grid-template-columns: 1fr 2fr;", "grid-template-rows: auto 1fr;", "grid-template-areas: \"a b\";"] {
        let s = CssStyleSheet::parse(&format!(".g {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["g"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Col class="g"><Text class="t">hi</Text></Col>"#,
            &format!(".g {{ {css} }}"),
        ));
        // No grid track sizing in immediate mode; falls back to vertical layout.
        assert!(rust.contains("vertical("), "{css} {rust}");
    }
}

#[test]
fn layout_grid_auto_and_placement_ignored() {
    for css in [
        "grid-auto-columns: 100px;",
        "grid-auto-rows: 50px;",
        "grid-auto-flow: dense;",
        "grid-column: 1 / 3;",
        "grid-row: 2;",
        "grid-area: header;",
    ] {
        let s = CssStyleSheet::parse(&format!(".g {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["g"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Col class="g"><Text class="t">hi</Text></Col>"#,
            &format!(".g {{ {css} }}"),
        ));
        assert!(syn::parse_file(&rust).is_ok());
        assert!(!rust.contains("grid"), "{css} {rust}");
    }
}

#[test]
fn layout_justify_items_self_ignored() {
    for (prop, v) in [("justify-items", "center"), ("justify-items", "stretch"), ("justify-self", "center"), ("justify-self", "end")] {
        let s = CssStyleSheet::parse(&format!(".g {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["g"], prop).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Col class="g"><Text class="t">hi</Text></Col>"#,
            &format!(".g {{ {prop}: {v}; }}"),
        ));
        // Grid-axis alignment has no egui target; Row/Col cross-axis only.
        assert!(!rust.contains("vertical_centered") || prop == "justify-items" && false, "{prop} {rust}");
    }
}

#[test]
fn layout_no_layout_props_yields_default_containers() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { display: flex; position: relative; }",
    ));
    assert!(rust.contains("horizontal("), "{rust}");
    assert!(!rust.contains("horizontal_centered"), "{rust}");
}
