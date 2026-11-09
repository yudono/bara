use std::path::PathBuf;

use bara::cargo_bridge::{BaraConfig, CargoBridge, SourceFile};

fn bridge_at(name: &str) -> (CargoBridge, BaraConfig) {
    let root = PathBuf::from(format!("/tmp/bara_dep_test_{name}"));
    let bridge = CargoBridge::new(root);
    let mut cfg = BaraConfig::default_for("my-app");
    cfg.version = "0.1.0".to_string();
    (bridge, cfg)
}

#[test]
fn deps_gui_preset_adds_egui_eframe() {
    let (bridge, mut cfg) = bridge_at("gui");
    cfg.presets.insert("gui".to_string());
    let toml = bridge.generate_cargo_toml(&cfg, "fn main() {}");
    // accesskit is OFF (upstream macOS abort); everything else mirrors default.
    assert!(toml.contains("eframe = { version = \"0.27\""), "{toml}");
    assert!(toml.contains("egui = \"0.27\""), "{toml}");
    assert!(!toml.contains("accesskit"), "{toml}");
}

#[test]
fn deps_autodetect_egui_in_rust_code() {
    let (bridge, cfg) = bridge_at("auto");
    let toml = bridge.generate_cargo_toml(&cfg, "fn update() { ui.label(egui::RichText::new(\"x\")); }");
    assert!(toml.contains("eframe = { version = \"0.27\""), "{toml}");
    assert!(toml.contains("egui = \"0.27\""), "{toml}");
    assert!(!toml.contains("accesskit"), "{toml}");
}

#[test]
fn deps_autodetect_tokio() {
    let (bridge, cfg) = bridge_at("tokio");
    let t1 = bridge.generate_cargo_toml(&cfg, "fn f() { tokio::spawn(async move { 1; }); }");
    assert!(t1.contains("tokio = { version = \"1\", features = [\"full\"] }"), "{t1}");
    let (bridge2, cfg2) = bridge_at("tokio2");
    let t2 = bridge2.generate_cargo_toml(&cfg2, "pub async fn f() {}");
    assert!(t2.contains("tokio ="), "{t2}");
}

#[test]
fn deps_autodetect_serde_pair() {
    let (bridge, cfg) = bridge_at("serde");
    let toml = bridge.generate_cargo_toml(&cfg, "fn f(x: serde::Serialize) {}");
    assert!(toml.contains("serde = { version = \"1.0\", features = [\"derive\"] }"), "{toml}");
    assert!(toml.contains("serde_json = \"1.0\""), "{toml}");
}

#[test]
fn deps_no_autodetect_for_plain_code() {
    let (bridge, cfg) = bridge_at("plain");
    let toml = bridge.generate_cargo_toml(&cfg, "fn main() { println!(\"hi\"); }");
    assert!(!toml.contains("eframe"), "{toml}");
    assert!(!toml.contains("egui"), "{toml}");
    assert!(!toml.contains("tokio"), "{toml}");
    assert!(!toml.contains("serde"), "{toml}");
}

#[test]
fn deps_custom_versions_and_name_sanitize() {
    let (bridge, mut cfg) = bridge_at("custom");
    cfg.dependencies.insert("anyhow".to_string(), "1".to_string());
    cfg.dependencies.insert("tokio".to_string(), "{ version = \"1\", features = [\"full\"] }".to_string());
    let toml = bridge.generate_cargo_toml(&cfg, "fn main() {}");
    assert!(toml.contains("name = \"my_app\""), "{toml}");
    assert!(toml.contains("edition = \"2021\""), "{toml}");
    assert!(toml.contains("anyhow = \"1\""), "{toml}");
    assert!(toml.contains("tokio = { version = \"1\", features = [\"full\"] }"), "{toml}");
}

#[test]
fn deps_translate_errors_single_source() {
    let (bridge, _) = bridge_at("err1");
    let sources = vec![SourceFile { path: PathBuf::from("main.br"), content: "x = 1".to_string() }];
    let stderr = "error[E0308]: mismatched types\n  --> .bara_cache/src/main.rs:42:15\n   |\n42 | let x = 1;\n";
    let out = bridge.translate_errors(stderr, &sources);
    assert!(out.contains("main.br:42:15"), "{out}");
}

#[test]
fn deps_translate_errors_multi_source_uses_entry() {
    let (bridge, _) = bridge_at("err2");
    let sources = vec![
        SourceFile { path: PathBuf::from("pages/auth/login.brx"), content: String::new() },
        SourceFile { path: PathBuf::from("main.br"), content: String::new() },
    ];
    let stderr = "  --> src/main.rs:7:3\n   |";
    let out = bridge.translate_errors(stderr, &sources);
    assert!(out.contains("main.br:7:3"), "{out}");
    let plain = "error: something broke\nno prefix here\n";
    assert_eq!(bridge.translate_errors(plain, &sources), "error: something broke\nno prefix here\n");
}
