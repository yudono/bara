use std::path::PathBuf;

use bara::cargo_bridge::CargoBridge;

fn example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join("backend_api")
}

fn backend_rust_code() -> String {
    let bridge = CargoBridge::new(example_dir());
    let (sources, _config) = bridge
        .collect_sources(None)
        .expect("backend_api: collect_sources failed");
    bridge
        .transpile_sources(&sources)
        .expect("backend_api: transpile_sources failed")
}

#[test]
fn test_backend_example_files_exist() {
    let dir = example_dir();
    assert!(dir.is_dir(), "examples/backend_api missing");
    for f in ["main.br", "server.br", "bara.toml"] {
        assert!(dir.join(f).exists(), "examples/backend_api/{f} missing");
    }
}

#[test]
fn test_backend_bara_toml_tables() {
    let content = std::fs::read_to_string(example_dir().join("bara.toml"))
        .expect("read bara.toml failed");
    assert!(content.contains("[package]"), "missing [package]");
    assert!(content.contains("[presets]"), "missing [presets]");
    assert!(content.contains("[dependencies]"), "missing [dependencies]");
}

#[test]
fn test_backend_transpile_has_http_request() {
    let rust = backend_rust_code();
    assert!(rust.contains("pub struct HttpRequest"), "missing HttpRequest");
    assert!(rust.contains("pub method:"), "HttpRequest missing method field");
    assert!(rust.contains("pub path:"), "HttpRequest missing path field");
    assert!(rust.contains("pub token:"), "HttpRequest missing token field");
    assert!(rust.contains("pub body:"), "HttpRequest missing body field");
}

#[test]
fn test_backend_transpile_has_http_response() {
    let rust = backend_rust_code();
    assert!(rust.contains("pub struct HttpResponse"), "missing HttpResponse");
    assert!(rust.contains("pub status_code:"), "missing status_code field");
}

#[test]
fn test_backend_transpile_has_api_server() {
    let rust = backend_rust_code();
    assert!(rust.contains("pub struct ApiServer"), "missing ApiServer");
    assert!(
        rust.contains("pub fn dispatch(&mut self, req: &HttpRequest) -> HttpResponse"),
        "missing dispatch method"
    );
}

#[test]
fn test_backend_dispatch_routes_200() {
    let rust = backend_rust_code();
    assert!(rust.contains("/health"), "missing /health route");
    assert!(rust.contains("/api/users"), "missing /api/users route");
    assert!(rust.contains("status_code: 200"), "missing 200 status");
}

#[test]
fn test_backend_dispatch_routes_401() {
    let rust = backend_rust_code();
    assert!(rust.contains("status_code: 401"), "missing 401 status");
    assert!(rust.contains("Unauthorized"), "missing Unauthorized body");
}

#[test]
fn test_backend_dispatch_routes_404() {
    let rust = backend_rust_code();
    assert!(rust.contains("status_code: 404"), "missing 404 status");
    assert!(rust.contains("Not Found"), "missing Not Found body");
}

#[test]
fn test_backend_transpile_main_dispatches() {
    let rust = backend_rust_code();
    assert!(rust.contains("fn main()"), "missing main fn");
    assert!(rust.contains("server.dispatch"), "main must call dispatch");
    assert!(rust.contains("status_code"), "main must read status_code");
}

#[test]
fn test_backend_syn_parses() {
    let rust = backend_rust_code();
    syn::parse_file(&rust).expect("backend transpiled Rust must parse with syn");
}

#[test]
fn test_backend_build_check_via_cargo_bridge() {
    let bridge = CargoBridge::new(example_dir());
    bridge.check(None).expect("backend_api: check() failed");
    let bin = bridge
        .build(None, false, true)
        .expect("backend_api: build(--offline) failed");
    assert!(bin.exists(), "built backend_api binary must exist");
}
