use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

use bara::cargo_bridge::{BaraConfig, CargoBridge};

#[test]
fn test_bara_config_parse_and_serialize() {
    let toml = r#"
[package]
name = "my_app"
version = "0.2.0"
entry = "app.br"

[presets]
gui = true
async = true

[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }
"#;

    let config = BaraConfig::parse(toml).expect("Parse failed");
    assert_eq!(config.name, "my_app");
    assert_eq!(config.version, "0.2.0");
    assert_eq!(config.entry, "app.br");
    assert!(config.presets.contains("gui"));
    assert!(config.presets.contains("async"));
    assert_eq!(config.dependencies.get("serde").unwrap(), "1.0");

    let serialized = config.to_toml_string();
    let re_parsed = BaraConfig::parse(&serialized).expect("Re-parse failed");
    assert_eq!(re_parsed.name, "my_app");
    assert_eq!(re_parsed.version, "0.2.0");
    assert_eq!(re_parsed.dependencies.get("serde").unwrap(), "1.0");
}

#[test]
fn test_cargo_bridge_init_and_check() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let project_dir = tmp.path().join("test_app");

    CargoBridge::init_project("test_app", &project_dir).expect("Init failed");

    // Blank hello-world template: core is fully dynamic, no hardcoded demo app.
    assert!(project_dir.join("bara.toml").exists());
    assert!(project_dir.join("main.br").exists());
    assert!(!project_dir.join("todo.br").exists());
    assert!(!project_dir.join("todo.brx").exists());

    let bridge = CargoBridge::new(project_dir.clone());
    let check_res = bridge.check(None);
    assert!(check_res.is_ok(), "Check failed: {:?}", check_res.err());

    let rust_code = bridge.transpile(None).expect("Transpile failed");
    assert!(rust_code.contains("Hello from Bara"));
    assert!(rust_code.contains("fn main()"));
    assert!(!rust_code.contains("TodoApp"));
}

#[test]
fn test_cargo_bridge_cache_generation() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let project_dir = tmp.path().join("cache_app");
    CargoBridge::init_project("cache_app", &project_dir).expect("Init failed");

    let bridge = CargoBridge::new(project_dir.clone());
    let (sources, config) = bridge.collect_sources(None).expect("Collect sources failed");
    let rust_code = bridge.transpile_sources(&sources).expect("Transpile failed");

    let main_rs = bridge.setup_cache(&config, &rust_code).expect("Setup cache failed");
    assert!(main_rs.exists());
    assert!(bridge.cache_dir.join("Cargo.toml").exists());

    let cargo_toml = fs::read_to_string(bridge.cache_dir.join("Cargo.toml")).expect("Read Cargo.toml failed");
    assert!(cargo_toml.contains("name = \"cache_app\""));
    // Blank hello-world has no GUI deps: eframe/egui are auto-detected only
    // when generated Rust actually uses them (fully dynamic core).
    assert!(!cargo_toml.contains("eframe"));
    assert!(!cargo_toml.contains("egui"));

    // GUI detection still works when Rust code uses eframe.
    let gui_toml = bridge.generate_cargo_toml(&config, "impl eframe::App for Foo {}");
    assert!(gui_toml.contains("eframe = { version = \"0.27\""));
    assert!(gui_toml.contains("egui = \"0.27\""));
    // No accesskit: its macOS screen-enumeration path aborts the app at launch.
    assert!(!gui_toml.contains("accesskit"));
}

#[test]
fn test_error_translation() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let project_dir = tmp.path().join("err_app");
    let bridge = CargoBridge::new(project_dir.clone());

    let fake_stderr = "error[E0308]: mismatched types\n  --> .bara_cache/src/main.rs:42:15\n   |\n42 | let x = 1;\n";
    let sources = vec![bara::cargo_bridge::SourceFile {
        path: PathBuf::from("main.br"),
        content: "x = 1".to_string(),
    }];

    let translated = bridge.translate_errors(fake_stderr, &sources);
    assert!(translated.contains("main.br:42:15"));
}
