use bara::css_engine::CssStyleSheet;

fn sheet(css: &str) -> CssStyleSheet {
    CssStyleSheet::parse(css)
}

#[test]
fn selector_single_class_maps_frame() {
    let s = sheet(".btn { background: #ff0000; padding: 8px; }");
    let frame = s.get_frame_code(&["btn"]).expect("frame for .btn");
    assert!(frame.contains("egui::Color32::from_rgb(255, 0, 0)"), "{frame}");
    assert!(frame.contains("egui::Margin::same(8.0)"), "{frame}");
}

#[test]
fn selector_compound_requires_all_classes() {
    let s = sheet(".btn.active { border: 2px solid #00ff00; }");
    assert!(s.get_frame_code(&["btn"]).is_none());
    assert!(s.get_frame_code(&["active"]).is_none());
    let frame = s.get_frame_code(&["btn", "active"]).expect("compound frame");
    assert!(frame.contains("egui::Stroke::new(2.0_f32"), "{frame}");
    assert!(frame.contains("egui::Color32::from_rgb(0, 255, 0)"), "{frame}");
}

#[test]
fn selector_cascade_later_rule_overrides_color() {
    let s = sheet(".btn { color: #111111; } .btn.active { color: #ffffff; }");
    let base = s.get_rich_text_modifiers(&["btn"]);
    assert!(base.contains(&"color(egui::Color32::from_rgb(17, 17, 17))".to_string()));
    let active = s.get_rich_text_modifiers(&["btn", "active"]);
    assert!(active.contains(&"color(egui::Color32::from_rgb(255, 255, 255))".to_string()));
    assert!(!active.contains(&"color(egui::Color32::from_rgb(17, 17, 17))".to_string()));
}

#[test]
fn selector_multiple_classes_merge_properties() {
    let s = sheet(".a { color: #ff0000; } .b { font-size: 20px; }");
    let mods = s.get_rich_text_modifiers(&["a", "b"]);
    assert!(mods.contains(&"color(egui::Color32::from_rgb(255, 0, 0))".to_string()));
    assert!(mods.contains(&"size(20.0)".to_string()));
}

#[test]
fn selector_base_rule_applies_to_compound_element() {
    let s = sheet(
        ".card { background: #1e293b; padding: 16px; } .card.highlighted { border: 2px solid #3b82f6; }",
    );
    let frame = s.get_frame_code(&["card", "highlighted"]).expect("frame");
    assert!(frame.contains("egui::Color32::from_rgb(30, 41, 59)"), "{frame}");
    assert!(frame.contains("egui::Margin::same(16.0)"), "{frame}");
    assert!(frame.contains("egui::Stroke::new(2.0_f32"), "{frame}");
}

#[test]
fn selector_later_padding_rule_wins() {
    let s = sheet(".x { padding: 4px; } .x { padding: 12px; }");
    let frame = s.get_frame_code(&["x"]).expect("frame");
    assert!(frame.contains("same(12.0)"), "{frame}");
    assert!(!frame.contains("same(4.0)"), "{frame}");
}

#[test]
fn selector_no_match_yields_none() {
    let s = sheet(".foo { background: #ffffff; }");
    assert!(s.get_frame_code(&["bar"]).is_none());
    assert!(s.get_frame_code(&[]).is_none());
    assert!(s.get_rich_text_modifiers(&["bar"]).is_empty());
}
