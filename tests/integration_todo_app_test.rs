use std::path::PathBuf;

use bara::cargo_bridge::CargoBridge;

fn example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join("todo_app")
}

fn todo_rust_code() -> String {
    let bridge = CargoBridge::new(example_dir());
    let (sources, _config) = bridge
        .collect_sources(None)
        .expect("todo_app: collect_sources failed");
    bridge
        .transpile_sources(&sources)
        .expect("todo_app: transpile_sources failed")
}

#[test]
fn test_todo_example_files_exist() {
    let dir = example_dir();
    assert!(dir.is_dir(), "examples/todo_app missing");
    for f in ["main.br", "todo.br", "todo.brx", "bara.toml"] {
        assert!(dir.join(f).exists(), "examples/todo_app/{f} missing");
    }
}

#[test]
fn test_todo_bara_toml_tables() {
    let content = std::fs::read_to_string(example_dir().join("bara.toml"))
        .expect("read bara.toml failed");
    assert!(content.contains("[package]"), "missing [package]");
    assert!(content.contains("[presets]"), "missing [presets]");
    assert!(content.contains("[dependencies]"), "missing [dependencies]");
}

#[test]
fn test_todo_transpile_has_todo_app_struct() {
    let rust = todo_rust_code();
    assert!(rust.contains("pub struct TodoApp"), "missing TodoApp struct");
    assert!(rust.contains("impl Default for TodoApp"), "missing Default impl");
    assert!(rust.contains("pub struct TodoItem"), "missing TodoItem struct");
    assert!(rust.contains("impl Default for TodoItem"), "missing TodoItem Default");
}

#[test]
fn test_todo_transpile_has_tambah_method() {
    let rust = todo_rust_code();
    assert!(
        rust.contains("pub fn tambah_tugas(&mut self)"),
        "missing tambah_tugas method"
    );
    assert!(rust.contains("TodoItem"), "tambah should construct TodoItem");
}

#[test]
fn test_todo_transpile_has_hapus_method() {
    let rust = todo_rust_code();
    assert!(
        rust.contains("pub fn hapus_tugas(&mut self, id: i64)"),
        "missing hapus_tugas method"
    );
}

#[test]
fn test_todo_transpile_has_hitung_method() {
    let rust = todo_rust_code();
    assert!(
        rust.contains("pub fn hitung_aktif(&self) -> i64"),
        "missing hitung_aktif method"
    );
}

#[test]
fn test_todo_transpile_has_eframe_app_impl() {
    let rust = todo_rust_code();
    assert!(
        rust.contains("impl eframe::App for TodoApp"),
        "missing eframe::App impl"
    );
    assert!(rust.contains("fn update("), "missing update fn");
    assert!(rust.contains("CentralPanel"), "missing CentralPanel");
}

#[test]
fn test_todo_transpile_headless_main() {
    let rust = todo_rust_code();
    assert!(rust.contains("fn main() -> Result<(), eframe::Error>"), "missing GUI main");
    assert!(rust.contains("BARA_HEADLESS"), "missing BARA_HEADLESS check");
    assert!(rust.contains("--headless"), "missing --headless flag");
    assert!(rust.contains("headless mode"), "missing headless message");
    assert!(rust.contains("eframe::run_native"), "missing run_native");
    assert!(rust.contains("Bara Todo Desktop"), "missing window title");
}

#[test]
fn test_todo_transpile_gui_bindings() {
    let rust = todo_rust_code();
    assert!(
        rust.contains("egui::TextEdit::singleline(&mut self.input_tugas)"),
        "missing input binding"
    );
    assert!(rust.contains(".clicked()"), "missing button click handler");
    assert!(rust.contains("ScrollArea"), "missing scroll area");
}

#[test]
fn test_todo_syn_parses() {
    let rust = todo_rust_code();
    syn::parse_file(&rust).expect("todo transpiled Rust must parse with syn");
}

#[test]
fn test_todo_build_check_via_cargo_bridge() {
    let bridge = CargoBridge::new(example_dir());
    bridge.check(None).expect("todo_app: check() failed");
    let (sources, config) = bridge.collect_sources(None).expect("collect failed");
    let rust_code = bridge.transpile_sources(&sources).expect("transpile failed");
    let cargo_toml = bridge.generate_cargo_toml(&config, &rust_code);
    assert!(cargo_toml.contains("eframe"), "generated Cargo.toml must include eframe");
    assert!(cargo_toml.contains("egui"), "generated Cargo.toml must include egui");
    let main_rs = bridge
        .setup_cache(&config, &rust_code)
        .expect("setup_cache failed");
    assert!(main_rs.exists(), "cached main.rs must exist");
}
