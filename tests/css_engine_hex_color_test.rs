use bara::css_engine::Color;

#[test]
fn hex_short_rgb_expands_nibbles() {
    assert_eq!(Color::parse("#f0a"), Some(Color::new(255, 0, 170, 255)));
}

#[test]
fn hex_short_rgba_expands_nibbles() {
    assert_eq!(Color::parse("#f0a8"), Some(Color::new(255, 0, 170, 136)));
}

#[test]
fn hex_long_rrggbb() {
    assert_eq!(Color::parse("#1f2937"), Some(Color::new(31, 41, 55, 255)));
}

#[test]
fn hex_long_rrggbbaa() {
    assert_eq!(Color::parse("#1f293780"), Some(Color::new(31, 41, 55, 128)));
}

#[test]
fn hex_uppercase_digits() {
    assert_eq!(Color::parse("#FF0000"), Some(Color::new(255, 0, 0, 255)));
}

#[test]
fn func_rgb_and_rgba_forms() {
    assert_eq!(Color::parse("rgb(59, 130, 246)"), Some(Color::new(59, 130, 246, 255)));
    assert_eq!(Color::parse("rgba(59, 130, 246, 0.5)"), Some(Color::new(59, 130, 246, 127)));
    assert_eq!(Color::parse("rgba(255, 0, 0, 50%)"), Some(Color::new(255, 0, 0, 127)));
    assert_eq!(Color::parse("rgba(10, 20, 30, 1.0)"), Some(Color::new(10, 20, 30, 255)));
}

#[test]
fn named_colors_resolve() {
    assert_eq!(Color::parse("red"), Some(Color::new(239, 68, 68, 255)));
    assert_eq!(Color::parse("white"), Some(Color::new(255, 255, 255, 255)));
    assert_eq!(Color::parse("black"), Some(Color::new(0, 0, 0, 255)));
    assert_eq!(Color::parse("transparent"), Some(Color::new(0, 0, 0, 0)));
    assert_eq!(Color::parse("RED"), Some(Color::new(239, 68, 68, 255)));
}

#[test]
fn to_egui_code_forms() {
    assert_eq!(Color::new(255, 0, 170, 255).to_egui_code(), "egui::Color32::from_rgb(255, 0, 170)");
    assert_eq!(Color::new(0, 0, 0, 0).to_egui_code(), "egui::Color32::TRANSPARENT");
    assert_eq!(
        Color::new(31, 41, 55, 128).to_egui_code(),
        "egui::Color32::from_rgba_premultiplied(31, 41, 55, 128)"
    );
}

#[test]
fn invalid_colors_return_none() {
    assert_eq!(Color::parse("#xyz"), None);
    assert_eq!(Color::parse("#12345"), None);
    assert_eq!(Color::parse("notacolor"), None);
    assert_eq!(Color::parse(""), None);
}
