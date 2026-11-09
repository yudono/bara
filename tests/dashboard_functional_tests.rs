use bara::cargo_bridge::CargoBridge;
use std::path::PathBuf;

fn dashboard_rust() -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/dashboard_gui");
    assert!(dir.is_dir(), "dashboard_gui example missing");
    let bridge = CargoBridge::new(dir);
    let (sources, _) = bridge.collect_sources(None).expect("collect");
    assert!(!sources.is_empty(), "expected dashboard sources");
    let rust = bridge.transpile_sources(&sources).expect("transpile");
    syn::parse_file(&rust).expect("syn");
    rust
}

#[test]
fn dashboard_boots_shell_app() {
    let rust = dashboard_rust();
    assert!(
        rust.contains("Box::new(DashboardApp::default())"),
        "boot target must be DashboardApp:\n{rust}"
    );
}

#[test]
fn dashboard_auth_methods_exist() {
    let rust = dashboard_rust();
    for sig in [
        "pub fn login(&mut self)",
        "pub fn logout(&mut self)",
        "pub fn register(&mut self)",
        "pub fn sign_out(&mut self)",
        "pub fn navigate(",
    ] {
        assert!(rust.contains(sig), "missing {sig}:\n{rust}");
    }
}

#[test]
fn dashboard_node_crud_methods_exist() {
    let rust = dashboard_rust();
    for sig in [
        "pub fn add_or_save(",
        "pub fn edit_user(&mut self, id: i64)",
        "pub fn cancel_edit(&mut self)",
        "pub fn remove_user(&mut self, id: i64)",
    ] {
        assert!(rust.contains(sig), "missing {sig}:\n{rust}");
    }
}

#[test]
fn dashboard_pages_are_child_components() {
    let rust = dashboard_rust();
    for tag in [
        "__child_LoginForm_0",
        "__child_RegisterForm_0",
        "__child_OverviewPage_0",
        "__child_UsersPage_0",
    ] {
        assert!(rust.contains(tag), "missing persisted child {tag}:\n{rust}");
    }
    assert!(rust.contains("__render_as_child"), "{rust}");
}
