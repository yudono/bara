use std::path::PathBuf;

use bara::cargo_bridge::CargoBridge;

fn example_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn verify_example(name: &str) {
    let dir = example_dir(name);
    if !dir.is_dir() {
        eprintln!("Skipping missing example dir: {name}");
        return;
    }
    let bridge = CargoBridge::new(dir.clone());
    let (sources, _config) = bridge
        .collect_sources(None)
        .unwrap_or_else(|e| panic!("{name}: collect_sources failed: {e}"));
    assert!(
        !sources.is_empty(),
        "{name}: expected at least one source file"
    );
    let rust_code = bridge
        .transpile_sources(&sources)
        .unwrap_or_else(|e| panic!("{name}: transpile_sources failed: {e}"));
    assert!(
        !rust_code.is_empty(),
        "{name}: expected non-empty transpiled output"
    );
    syn::parse_file(&rust_code)
        .unwrap_or_else(|e| panic!("{name}: syn parse of transpiled code failed: {e}"));
    bridge
        .check(None)
        .unwrap_or_else(|e| panic!("{name}: check() failed: {e}"));
}

#[test]
fn test_example_cli_algorithms() {
    verify_example("cli_algorithms");
}

#[test]
fn test_example_backend_api() {
    verify_example("backend_api");
}

#[test]
fn test_example_todo_app() {
    verify_example("todo_app");
}

#[test]
fn test_example_dashboard_gui() {
    verify_example("dashboard_gui");
}

#[test]
fn test_all_known_examples_transpile() {
    for name in [
        "cli_algorithms",
        "backend_api",
        "todo_app",
        "dashboard_gui",
        "brui_xml_gui",
        "multi_lib",
        "object_handling",
    ] {
        verify_example(name);
    }
}

#[test]
fn test_cli_accepts_compiled_binary_path() {
    // Regression test: `bara check <target/debug/<binary>>` must resolve to the
    // enclosing project instead of failing with "No Bara source files found".
    let bara = PathBuf::from(env!("CARGO_BIN_EXE_bara"));
    let binary = example_dir("cli_algorithms").join("target").join("debug");
    let binary = binary.join("cli_algorithms");
    if !binary.is_file() {
        eprintln!("Skipping binary-path test: binary not built yet");
        return;
    }
    let out = std::process::Command::new(&bara)
        .arg("check")
        .arg(&binary)
        .output()
        .expect("failed to run bara check");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "bara check <binary> failed: {combined}");
    assert!(
        combined.contains("verified successfully"),
        "unexpected output: {combined}"
    );
}
