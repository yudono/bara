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
fn flex_gap_row_gap_column_gap_map_to_spacing() {
    let s = CssStyleSheet::parse(".f { gap: 12px; }");
    assert_eq!(s.get_spacing(&["f"]), Some(12.0));
    let r = CssStyleSheet::parse(".f { row-gap: 10px; }");
    assert_eq!(r.get_spacing(&["f"]), Some(10.0));
    let c = CssStyleSheet::parse(".f { column-gap: 6px; }");
    assert_eq!(c.get_spacing(&["f"]), Some(6.0));
    let rust = transpile_ok(&view_shell(
        r#"<Col class="f"><Text class="t">hi</Text></Col>"#,
        ".f { gap: 12px; }",
    ));
    assert!(rust.contains("item_spacing.y = 12.0"), "{rust}");
    let rust2 = transpile_ok(&view_shell(
        r#"<Row class="f"><Text class="t">hi</Text></Row>"#,
        ".f { column-gap: 10px; }",
    ));
    assert!(rust2.contains("item_spacing.x = 10.0"), "{rust2}");
}

#[test]
fn flex_justify_space_between_uses_main_justify() {
    let s = CssStyleSheet::parse(".r { justify-content: space-between; }");
    assert_eq!(
        s.get_justify_content(&["r"]).as_deref(),
        Some("space-between")
    );
    let rust = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="a">x</Text><Text class="b">y</Text></Row>"#,
        ".r { justify-content: space-between; }",
    ));
    assert!(rust.contains("with_main_justify(true)"), "{rust}");
    assert!(rust.contains("left_to_right"), "{rust}");
}

#[test]
fn flex_justify_other_values_ignored_fallback() {
    for v in ["center", "flex-start", "flex-end", "space-around", "space-evenly", "start", "end"] {
        let s = CssStyleSheet::parse(&format!(".r {{ justify-content: {v}; }}"));
        assert_eq!(s.get_justify_content(&["r"]).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Row class="r"><Text class="a">x</Text></Row>"#,
            &format!(".r {{ justify-content: {v}; }}"),
        ));
        // Only space-between is wired; all others parse but emit no justify.
        assert!(!rust.contains("with_main_justify(true)"), "value {v}: {rust}");
    }
}

#[test]
fn flex_align_center_row_and_col() {
    let s = CssStyleSheet::parse(".r { align-items: center; }");
    assert_eq!(s.get_align_items(&["r"]).as_deref(), Some("center"));
    let rust_row = transpile_ok(&view_shell(
        r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
        ".r { align-items: center; }",
    ));
    assert!(rust_row.contains("horizontal_centered"), "{rust_row}");
    let rust_col = transpile_ok(&view_shell(
        r#"<Col class="c"><Text class="t">hi</Text></Col>"#,
        ".c { align-items: center; }",
    ));
    assert!(rust_col.contains("vertical_centered"), "{rust_col}");
}

#[test]
fn flex_align_end_maps_to_max_layout() {
    for v in ["flex-end", "end"] {
        let rust = transpile_ok(&view_shell(
            r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
            &format!(".r {{ align-items: {v}; }}"),
        ));
        assert!(rust.contains("Align::Max"), "value {v}: {rust}");
    }
}

#[test]
fn flex_align_other_values_default_to_min_fallback() {
    for v in ["stretch", "flex-start", "start", "baseline"] {
        let s = CssStyleSheet::parse(&format!(".r {{ align-items: {v}; }}"));
        assert_eq!(s.get_align_items(&["r"]).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Row class="r"><Text class="t">hi</Text></Row>"#,
            &format!(".r {{ align-items: {v}; }}"),
        ));
        // Min is the default Row layout: no centered wrapper.
        assert!(rust.contains("horizontal("), "value {v}: {rust}");
        assert!(!rust.contains("horizontal_centered"), "value {v}: {rust}");
    }
}

#[test]
fn flex_direction_parsed_but_ignored() {
    for v in ["row", "row-reverse", "column", "column-reverse"] {
        let s = CssStyleSheet::parse(&format!(".f {{ flex-direction: {v}; }}"));
        assert_eq!(s.get_flex_direction(&["f"]).as_deref(), Some(v));
        assert_eq!(s.get_raw_property(&["f"], "flex-direction").as_deref(), Some(v));
        // Direction is conveyed by Row vs Col tags, not the CSS property.
        let rust = transpile_ok(&view_shell(
            r#"<Row class="f"><Text class="t">hi</Text></Row>"#,
            &format!(".f {{ flex-direction: {v}; }}"),
        ));
        assert!(rust.contains("horizontal("), "{rust}");
    }
}

#[test]
fn flex_wrap_parsed_wrap_widget_always_wrapped() {
    for v in ["wrap", "nowrap", "wrap-reverse"] {
        let s = CssStyleSheet::parse(&format!(".w {{ flex-wrap: {v}; }}"));
        assert_eq!(s.get_flex_wrap(&["w"]).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Wrap class="w"><Text class="t">hi</Text></Wrap>"#,
            &format!(".w {{ flex-wrap: {v}; gap: 8px; }}"),
        ));
        // <Wrap> always emits horizontal_wrapped regardless of flex-wrap value.
        assert!(rust.contains("horizontal_wrapped"), "{rust}");
    }
}

#[test]
fn flex_shorthand_grow_shrink_basis_flow_order_ignored() {
    let css = ".f { flex: 1 1 auto; flex-grow: 2; flex-shrink: 0; flex-basis: 100px; flex-flow: row wrap; order: 3; }";
    let s = CssStyleSheet::parse(css);
    for prop in ["flex", "flex-grow", "flex-shrink", "flex-basis", "flex-flow", "order"] {
        assert!(s.get_raw_property(&["f"], prop).is_some(), "{prop}");
    }
    assert_eq!(s.get_raw_property(&["f"], "flex-grow").as_deref(), Some("2"));
    assert_eq!(s.get_raw_property(&["f"], "order").as_deref(), Some("3"));
    // No egui flex-factor / ordering primitive: nearest equivalent is gap only.
    assert_eq!(s.get_spacing(&["f"]), None);
    let rust = transpile_ok(&view_shell(
        r#"<Row class="f"><Text class="t">hi</Text></Row>"#,
        css,
    ));
    assert!(!rust.contains("flex"), "{rust}");
}

#[test]
fn flex_align_content_and_self_ignored() {
    for (prop, v) in [
        ("align-content", "center"),
        ("align-content", "space-between"),
        ("align-content", "stretch"),
        ("align-self", "center"),
        ("align-self", "flex-end"),
        ("align-self", "stretch"),
    ] {
        let s = CssStyleSheet::parse(&format!(".f {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["f"], prop).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Row class="f"><Text class="t">hi</Text></Row>"#,
            &format!(".f {{ {prop}: {v}; }}"),
        ));
        // Per-item / multi-line alignment has no declarative target; ignored.
        assert!(!rust.contains("with_main_justify"), "{prop}:{v} {rust}");
    }
    // align-self:center on a Row does not center the row itself (no cross hint).
    let rust = transpile_ok(&view_shell(
        r#"<Row class="f"><Text class="t">hi</Text></Row>"#,
        ".f { align-self: center; }",
    ));
    assert!(!rust.contains("horizontal_centered"), "{rust}");
}

#[test]
fn flex_row_gap_takes_row_axis_spacing() {
    let rust = transpile_ok(&view_shell(
        r#"<Row class="f"><Text class="t">hi</Text></Row>"#,
        ".f { row-gap: 14px; }",
    ));
    // get_spacing falls back to row-gap; Row emits it on the x axis.
    assert!(rust.contains("item_spacing.x = 14.0"), "{rust}");
}
