use std::fs;

use tempfile::tempdir;

use bara::cargo_bridge::{BaraConfig, CargoBridge};
use bara::debugger::Severity;

fn write_project(root: &std::path::Path) {
    fs::create_dir_all(root.join("pages/auth")).expect("mkdir pages/auth");
    fs::write(
        root.join("bara.toml"),
        "[package]\nname = \"sync_app\"\nversion = \"0.1.0\"\nentry = \"main.br\"\n",
    )
    .expect("write bara.toml");
    fs::write(root.join("main.br"), "use \"styles.css\"\nprint(\"boot\")\n").expect("write main.br");
    fs::write(
        root.join("pages/auth/login.brx"),
        "type Login =\n    username: str = \"\"\n\nview Login =>\n    <Panel class=\"login\">\n        <Text class=\"title\">Login</Text>\n    </Panel>\n",
    )
    .expect("write login.brx");
    fs::write(
        root.join("styles.css"),
        ".login {\n    background: #ffffff;\n    padding: 16px;\n}\n.title {\n    color: #000000;\n    font-size: 16px;\n}\n",
    )
    .expect("write styles.css");
}

#[test]
fn sync_config_parses_package_and_deps() {
    let toml = "[package]\nname = \"sync_app\"\nversion = \"0.3.0\"\nentry = \"main.br\"\n\n[dependencies]\nserde = \"1.0\"\n";
    let cfg = BaraConfig::parse(toml).expect("parse");
    assert_eq!(cfg.name, "sync_app");
    assert_eq!(cfg.version, "0.3.0");
    assert_eq!(cfg.entry, "main.br");
    assert_eq!(cfg.dependencies.get("serde").unwrap(), "1.0");
}

#[test]
fn sync_config_default_and_save_roundtrip() {
    let tmp = tempdir().expect("tempdir");
    let cfg = BaraConfig::default_for("sync_app");
    assert_eq!(cfg.version, "0.1.0");
    assert_eq!(cfg.entry, "main.br");
    let path = tmp.path().join("bara.toml");
    cfg.save(&path).expect("save");
    let re = BaraConfig::from_file(&path).expect("reload");
    assert_eq!(re.name, "sync_app");
    assert_eq!(re.version, "0.1.0");
    assert_eq!(re.entry, "main.br");
}

#[test]
fn sync_collect_finds_nested_brx_and_css() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("proj");
    write_project(&root);
    let bridge = CargoBridge::new(root.clone());
    let (sources, config) = bridge.collect_sources(None).expect("collect");
    assert_eq!(config.name, "sync_app");
    assert_eq!(sources.len(), 3);
    let names: Vec<String> = sources.iter().map(|s| s.path.display().to_string()).collect();
    assert!(names.iter().any(|p| p.ends_with("pages/auth/login.brx")), "{names:?}");
    assert!(names.iter().any(|p| p.ends_with("styles.css")), "{names:?}");
    assert!(names.iter().any(|p| p.ends_with("main.br")), "{names:?}");
}

#[test]
fn sync_collect_places_entry_last() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("proj");
    write_project(&root);
    let bridge = CargoBridge::new(root.clone());
    let (sources, _) = bridge.collect_sources(None).expect("collect");
    let last = sources.last().expect("entry");
    assert!(last.path.ends_with("main.br"), "{}", last.path.display());
}

#[test]
fn sync_transpile_merges_css_and_brx() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("proj");
    write_project(&root);
    let bridge = CargoBridge::new(root.clone());
    let (sources, _) = bridge.collect_sources(None).expect("collect");
    let rust = bridge.transpile_sources(&sources).expect("transpile");
    assert!(rust.contains("pub struct Login"), "{rust}");
    assert!(rust.contains("impl eframe::App for Login"), "{rust}");
    assert!(rust.contains("egui::Color32::from_rgb(255, 255, 255)"), "{rust}");
    assert!(rust.contains(".size(16.0)"), "{rust}");
    assert!(rust.contains("fn main()"), "{rust}");
}

#[test]
fn sync_diagnose_clean_project_has_no_errors() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("proj");
    write_project(&root);
    let bridge = CargoBridge::new(root.clone());
    let (sources, _) = bridge.collect_sources(None).expect("collect");
    let diags = bridge.diagnose_sources(&sources);
    assert!(!diags.iter().any(|d| d.severity == Severity::Error), "{diags:?}");
}

#[test]
fn sync_collect_empty_dir_errors() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("empty");
    fs::create_dir_all(&root).expect("mkdir");
    let bridge = CargoBridge::new(root);
    assert!(bridge.collect_sources(None).is_err());
}

#[test]
fn sync_css_requires_explicit_import() {
    // Stylesheets are explicit-only: without `use "styles.css"` the .css
    // file is collected but NOT merged into the transpiled output.
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path().join("proj_noimport");
    fs::create_dir_all(root.join("pages/auth")).expect("mkdir pages/auth");
    fs::write(
        root.join("bara.toml"),
        "[package]\nname = \"sync_app\"\nversion = \"0.1.0\"\nentry = \"main.br\"\n",
    )
    .expect("write bara.toml");
    fs::write(root.join("main.br"), "print(\"boot\")\n").expect("write main.br");
    fs::write(
        root.join("pages/auth/login.brx"),
        "type Login =\n    username: str = \"\"\n\nview Login =>\n    <Panel class=\"login\">\n        <Text class=\"title\">Login</Text>\n    </Panel>\n",
    )
    .expect("write login.brx");
    fs::write(
        root.join("styles.css"),
        ".login {\n    background: #ffffff;\n    padding: 16px;\n}\n",
    )
    .expect("write styles.css");
    let bridge = CargoBridge::new(root.clone());
    let (sources, _) = bridge.collect_sources(None).expect("collect");
    let rust = bridge.transpile_sources(&sources).expect("transpile");
    assert!(rust.contains("pub struct Login"), "{rust}");
    assert!(
        !rust.contains("egui::Color32::from_rgb(255, 255, 255)"),
        "css must NOT be auto-loaded without explicit use: {rust}"
    );
}
