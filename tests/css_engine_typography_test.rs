use bara::css_engine::CssStyleSheet;

fn sheet(css: &str) -> CssStyleSheet {
    CssStyleSheet::parse(css)
}

#[test]
fn typography_color_modifier() {
    let s = sheet(".t { color: #38bdf8; }");
    let mods = s.get_rich_text_modifiers(&["t"]);
    assert!(mods.contains(&"color(egui::Color32::from_rgb(56, 189, 248))".to_string()));
}

#[test]
fn typography_font_size_modifier() {
    let s = sheet(".a { font-size: 20px; } .b { font-size: 14px; }");
    assert!(s.get_rich_text_modifiers(&["a"]).contains(&"size(20.0)".to_string()));
    assert!(s.get_rich_text_modifiers(&["b"]).contains(&"size(14.0)".to_string()));
}

#[test]
fn typography_bold_weights_map_to_strong() {
    for w in ["bold", "700", "800", "900", "bolder"] {
        let s = sheet(&format!(".t {{ font-weight: {w}; }}"));
        assert!(s.get_rich_text_modifiers(&["t"]).contains(&"strong()".to_string()), "{w}");
    }
    let normal = sheet(".t { font-weight: 400; }");
    assert!(!normal.get_rich_text_modifiers(&["t"]).contains(&"strong()".to_string()));
}

#[test]
fn typography_italic_styles_map_to_italics() {
    for st in ["italic", "oblique"] {
        let s = sheet(&format!(".t {{ font-style: {st}; }}"));
        assert!(s.get_rich_text_modifiers(&["t"]).contains(&"italics()".to_string()), "{st}");
    }
    let normal = sheet(".t { font-style: normal; }");
    assert!(!normal.get_rich_text_modifiers(&["t"]).contains(&"italics()".to_string()));
}

#[test]
fn typography_text_decoration_maps() {
    let u = sheet(".t { text-decoration: underline; }");
    assert!(u.get_rich_text_modifiers(&["t"]).contains(&"underline()".to_string()));
    let st = sheet(".t { text-decoration: line-through; }");
    assert!(st.get_rich_text_modifiers(&["t"]).contains(&"strikethrough()".to_string()));
    let both = sheet(".t { text-decoration: underline line-through; }");
    let mods = both.get_rich_text_modifiers(&["t"]);
    assert!(mods.contains(&"underline()".to_string()));
    assert!(mods.contains(&"strikethrough()".to_string()));
}

#[test]
fn typography_monospace_families() {
    for fam in ["monospace", "Courier New", "Consolas"] {
        let s = sheet(&format!(".t {{ font-family: {fam}; }}"));
        assert!(s.get_rich_text_modifiers(&["t"]).contains(&"monospace()".to_string()), "{fam}");
    }
    let sans = sheet(".t { font-family: Inter, sans-serif; }");
    assert!(!sans.get_rich_text_modifiers(&["t"]).contains(&"monospace()".to_string()));
}

#[test]
fn typography_combined_title_rule() {
    let s = sheet(".title { color: #222222; font-size: 18px; font-weight: bold; font-style: italic; }");
    let mods = s.get_rich_text_modifiers(&["title"]);
    assert!(mods.contains(&"color(egui::Color32::from_rgb(34, 34, 34))".to_string()));
    assert!(mods.contains(&"size(18.0)".to_string()));
    assert!(mods.contains(&"strong()".to_string()));
    assert!(mods.contains(&"italics()".to_string()));
}

#[test]
fn typography_no_text_props_yields_empty() {
    let s = sheet(".box { background: #ffffff; padding: 8px; }");
    assert!(s.get_rich_text_modifiers(&["box"]).is_empty());
}
