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
fn fx_opacity_maps_to_set_opacity_and_modulates() {
    let s = CssStyleSheet::parse(".t { color: #ffffff; opacity: 0.5; }");
    assert_eq!(s.get_opacity(&["t"]), Some(0.5));
    let mods = s.get_rich_text_modifiers(&["t"]).join(" ");
    assert!(mods.contains("from_rgba_premultiplied(255, 255, 255, 127)"), "{mods}");
    let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, ".t { color: #ffffff; opacity: 0.5; }"));
    assert!(rust.contains("set_opacity(0.500)"), "{rust}");
}

#[test]
fn fx_filter_backdrop_ignored() {
    for (prop, v) in [
        ("filter", "blur(4px)"),
        ("filter", "brightness(0.5)"),
        ("backdrop-filter", "blur(8px)"),
    ] {
        let s = CssStyleSheet::parse(&format!(".f {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["f"], prop).as_deref(), Some(v));
        assert!(s.get_frame_code(&["f"]).is_none());
        let rust = transpile_ok(&view_shell(
            r#"<Col class="f"><Text class="t">hi</Text></Col>"#,
            &format!(".f {{ {prop}: {v}; }}"),
        ));
        assert!(!rust.contains("blur"), "{prop} {rust}");
    }
}

#[test]
fn fx_clip_mask_ignored() {
    for (prop, v) in [
        ("clip-path", "circle(50%)"),
        ("mask", "url('m.png')"),
        ("mask-image", "linear-gradient(black, transparent)"),
    ] {
        let s = CssStyleSheet::parse(&format!(".f {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["f"], prop).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Col class="f"><Text class="t">hi</Text></Col>"#,
            &format!(".f {{ {prop}: {v}; }}"),
        ));
        assert!(!rust.contains("clip"), "{prop} {rust}");
    }
}

#[test]
fn fx_cursor_pointer_events_user_select_appearance_resize_ignored() {
    for (prop, v) in [
        ("cursor", "pointer"),
        ("pointer-events", "none"),
        ("user-select", "none"),
        ("appearance", "none"),
        ("resize", "both"),
    ] {
        let s = CssStyleSheet::parse(&format!(".f {{ {prop}: {v}; }}"));
        assert_eq!(s.get_raw_property(&["f"], prop).as_deref(), Some(v));
        let rust = transpile_ok(&view_shell(
            r#"<Button class="f">Go</Button>"#,
            &format!(".f {{ {prop}: {v}; }}"),
        ));
        // Immediate-mode widgets have no CSS cursor/hit-test/selectivity hooks.
        assert!(!rust.contains(v), "{prop} {rust}");
    }
}

#[test]
fn fx_transition_shorthands_ignored() {
    for css in [
        "transition: all 0.3s ease;",
        "transition-property: opacity;",
        "transition-duration: 200ms;",
        "transition-timing-function: ease-in-out;",
        "transition-delay: 100ms;",
    ] {
        let s = CssStyleSheet::parse(&format!(".a {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["a"], prop).is_some(), "{css}");
        assert!(s.get_frame_code(&["a"]).is_none(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Col class="a"><Text class="t">hi</Text></Col>"#,
            &format!(".a {{ {css} }}"),
        ));
        assert!(!rust.contains("transition"), "{css} {rust}");
    }
}

#[test]
fn fx_animation_shorthands_ignored() {
    for css in [
        "animation: fade 1s infinite;",
        "animation-name: fade;",
        "animation-duration: 2s;",
        "animation-timing-function: linear;",
        "animation-delay: 100ms;",
        "animation-iteration-count: infinite;",
        "animation-direction: alternate;",
    ] {
        let s = CssStyleSheet::parse(&format!(".a {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["a"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Col class="a"><Text class="t">hi</Text></Col>"#,
            &format!(".a {{ {css} }}"),
        ));
        // egui is immediate-mode: no retained keyframe timeline; ignored.
        assert!(!rust.contains("animation"), "{css} {rust}");
    }
}

#[test]
fn fx_transform_origin_ignored() {
    for css in ["transform: rotate(45deg);", "transform: translateX(10px);", "transform-origin: center;"] {
        let s = CssStyleSheet::parse(&format!(".a {{ {css} }}"));
        let prop = css.split(':').next().unwrap().trim();
        assert!(s.get_raw_property(&["a"], prop).is_some(), "{css}");
        let rust = transpile_ok(&view_shell(
            r#"<Col class="a"><Text class="t">hi</Text></Col>"#,
            &format!(".a {{ {css} }}"),
        ));
        assert!(!rust.contains("rotate"), "{css} {rust}");
        assert!(!rust.contains("transform"), "{css} {rust}");
    }
}

#[test]
fn fx_opacity_edge_values_clamped() {
    let s = CssStyleSheet::parse(".a { opacity: 0; }");
    assert_eq!(s.get_opacity(&["a"]), Some(0.0));
    let rust = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, ".t { opacity: 0; }"));
    assert!(rust.contains("set_opacity(0.000)"), "{rust}");
    let s2 = CssStyleSheet::parse(".a { opacity: 1; }");
    assert_eq!(s2.get_opacity(&["a"]), Some(1.0));
    let rust2 = transpile_ok(&view_shell(r#"<Text class="t">hi</Text>"#, ".t { opacity: 1; }"));
    // Opacity 1 emits no set_opacity wrapper (default).
    assert!(!rust2.contains("set_opacity"), "{rust2}");
}
