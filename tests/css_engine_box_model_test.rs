use bara::css_engine::{CssStyleSheet, Margin};

fn sheet(css: &str) -> CssStyleSheet {
    CssStyleSheet::parse(css)
}

#[test]
fn box_uniform_padding_emits_same_margin() {
    let s = sheet(".p { padding: 16px; }");
    let frame = s.get_frame_code(&["p"]).expect("frame");
    assert!(frame.contains("inner_margin(egui::Margin::same(16.0))"), "{frame}");
}

#[test]
fn box_two_value_padding_emits_struct_margin() {
    let s = sheet(".p { padding: 8px 16px; }");
    let frame = s.get_frame_code(&["p"]).expect("frame");
    assert!(
        frame.contains("egui::Margin { top: 8.0, right: 16.0, bottom: 8.0, left: 16.0 }"),
        "{frame}"
    );
}

#[test]
fn box_four_value_padding_emits_struct_margin() {
    let s = sheet(".p { padding: 1px 2px 3px 4px; }");
    let frame = s.get_frame_code(&["p"]).expect("frame");
    assert!(
        frame.contains("egui::Margin { top: 1.0, right: 2.0, bottom: 3.0, left: 4.0 }"),
        "{frame}"
    );
}

#[test]
fn box_rem_padding_converts_to_px() {
    let s = sheet(".p { padding: 1.5rem; }");
    let frame = s.get_frame_code(&["p"]).expect("frame");
    assert!(frame.contains("egui::Margin::same(24.0)"), "{frame}");
}

#[test]
fn box_three_value_margin_parses() {
    let m = Margin::parse("5px 15px 25px").expect("margin");
    assert_eq!(m.top, 5.0);
    assert_eq!(m.right, 15.0);
    assert_eq!(m.bottom, 25.0);
    assert_eq!(m.left, 15.0);
}

#[test]
fn box_border_radius_emits_rounding() {
    let s = sheet(".r { border-radius: 8px; }");
    let frame = s.get_frame_code(&["r"]).expect("frame");
    assert!(frame.contains("rounding(egui::Rounding::same(8.0))"), "{frame}");
}

#[test]
fn box_border_shorthand_emits_stroke() {
    let s = sheet(".b { border: 2px solid #3b82f6; }");
    let frame = s.get_frame_code(&["b"]).expect("frame");
    assert!(frame.contains("stroke(egui::Stroke::new(2.0_f32"), "{frame}");
    assert!(frame.contains("egui::Color32::from_rgb(59, 130, 246)"), "{frame}");
}

#[test]
fn box_border_width_and_color_pair_emits_stroke() {
    let s = sheet(".b { border-width: 3px; border-color: #ff0000; }");
    let frame = s.get_frame_code(&["b"]).expect("frame");
    assert!(frame.contains("stroke(egui::Stroke::new(3.0_f32"), "{frame}");
    assert!(frame.contains("egui::Color32::from_rgb(255, 0, 0)"), "{frame}");
}

#[test]
fn box_shadow_none_emits_shadow_call() {
    let s = sheet(".c { background: #ffffff; box-shadow: none; }");
    let frame = s.get_frame_code(&["c"]).expect("frame");
    assert!(frame.contains("shadow(egui::epaint::Shadow::NONE)"), "{frame}");
}
