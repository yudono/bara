use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::analyzer::Analyzer;
use crate::ast::{Item, Program};
use crate::codegen::Codegen;
use crate::debugger::{BaraDebugger, Diagnostic, Severity};
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Represents the configuration from `bara.toml`
#[derive(Debug, Clone)]
pub struct BaraConfig {
    pub name: String,
    pub version: String,
    pub entry: String,
    pub dependencies: HashMap<String, String>,
    pub presets: HashSet<String>,
}

impl BaraConfig {
    pub fn default_for(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            entry: "main.br".to_string(),
            dependencies: HashMap::new(),
            presets: HashSet::new(),
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self, String> {
        let mut name = "bara_app".to_string();
        let mut version = "0.1.0".to_string();
        let mut entry = "main.br".to_string();
        let mut dependencies = HashMap::new();
        let mut presets = HashSet::new();

        let mut current_section = "";

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].trim();
                continue;
            }

            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                let val = v.trim().trim_matches('"').trim_matches('\'');

                match current_section {
                    "package" => match key {
                        "name" => name = val.to_string(),
                        "version" => version = val.to_string(),
                        "entry" => entry = val.to_string(),
                        _ => {}
                    },
                    "dependencies" => {
                        dependencies.insert(key.to_string(), val.to_string());
                    }
                    "presets" => {
                        if val == "true" {
                            presets.insert(key.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            name,
            version,
            entry,
            dependencies,
            presets,
        })
    }

    pub fn to_toml_string(&self) -> String {
        let mut out = String::new();
        out.push_str("[package]\n");
        out.push_str(&format!("name = \"{}\"\n", self.name));
        out.push_str(&format!("version = \"{}\"\n", self.version));
        out.push_str(&format!("entry = \"{}\"\n\n", self.entry));

        if !self.presets.is_empty() {
            out.push_str("[presets]\n");
            for p in &self.presets {
                out.push_str(&format!("{p} = true\n"));
            }
            out.push('\n');
        }

        out.push_str("[dependencies]\n");
        for (k, v) in &self.dependencies {
            if v.starts_with('{') {
                out.push_str(&format!("{k} = {v}\n"));
            } else {
                out.push_str(&format!("{k} = \"{v}\"\n"));
            }
        }

        out
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        fs::write(path, self.to_toml_string())
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))
    }
}

/// Represents a Bara source file
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
}

/// Cargo bridge and project manager for Bara
pub struct CargoBridge {
    pub project_root: PathBuf,
    pub cache_dir: PathBuf,
}

impl CargoBridge {
    pub fn new(project_root: PathBuf) -> Self {
        let cache_dir = project_root.join(".bara_cache");
        Self {
            project_root,
            cache_dir,
        }
    }

    /// Initialize a new Bara project template
    pub fn init_project(name: &str, target_dir: &Path) -> Result<(), String> {
        if !target_dir.exists() {
            fs::create_dir_all(target_dir)
                .map_err(|e| format!("Failed to create directory {}: {e}", target_dir.display()))?;
        }

        let config = BaraConfig::default_for(name);
        config.save(&target_dir.join("bara.toml"))?;

        let gitignore = ".bara_cache/\ntarget/\n";
        fs::write(target_dir.join(".gitignore"), gitignore)
            .map_err(|e| format!("Failed to write .gitignore: {e}"))?;

        let todo_br = r#"type TodoItem =
    id: int
    teks: str
    selesai = false

type TodoApp =
    input_tugas = ""
    filter_status = "Semua"
    daftar_tugas = [
        TodoItem(id = 1, teks = "Bikin compiler Bara", selesai = true),
        TodoItem(id = 2, teks = "Integrasi parser JSX .brx", selesai = false),
        TodoItem(id = 3, teks = "Compile ke egui native", selesai = false)
    ]

    tambah_tugas() =>
        if input_tugas != "" =>
            id_baru = daftar_tugas.len() + 1
            daftar_tugas.push(TodoItem(id = id_baru, teks = input_tugas, selesai = false))
            input_tugas = ""

    hapus_tugas(id: int) =>
        daftar_tugas.retain(t => t.id != id)

    hitung_aktif() =>
        daftar_tugas.filter(t => !t.selesai).len()
"#;
        fs::write(target_dir.join("todo.br"), todo_br)
            .map_err(|e| format!("Failed to write todo.br: {e}"))?;

        let todo_brx = r#"use eframe
use egui
use TodoApp

view TodoApp =>
    <Panel class="layar_utama">
        <Col class="container_utama">
            
            {/* Header Aplikasi */}
            <Row class="header_box">
                <Text class="judul">⚡ Bara Todo App</Text>
                <Text class="badge">Sisa: {hitung_aktif()} tugas</Text>
            </Row>

            {/* Form Input Tugas Baru */}
            <Row class="form_box">
                <Input 
                    bind={input_tugas} 
                    placeholder="Apa rencana kerjamu hari ini?..." 
                />
                <Button class="btn_tambah" onClick={tambah_tugas()}>
                    Tambah
                </Button>
            </Row>

            {/* Filter Tab */}
            <Row class="filter_box">
                <Button class={filter_status == "Semua" ? "btn_filter aktif" : "btn_filter"} onClick={filter_status = "Semua"}>
                    Semua
                </Button>
                <Button class={filter_status == "Aktif" ? "btn_filter aktif" : "btn_filter"} onClick={filter_status = "Aktif"}>
                    Aktif
                </Button>
                <Button class={filter_status == "Selesai" ? "btn_filter aktif" : "btn_filter"} onClick={filter_status = "Selesai"}>
                    Selesai
                </Button>
            </Row>

            <Separator />

            {/* List Tugas (Scrollable Area) */}
            <Scroll class="scroll_area">
                <Col class="list_col">
                    {for item in daftar_tugas =>
                        {if filter_status == "Semua" or (filter_status == "Aktif" and !item.selesai) or (filter_status == "Selesai" and item.selesai) =>
                            <Row class={item.selesai ? "item_tugas selesai" : "item_tugas"}>
                                <Checkbox bind={item.selesai} />
                                <Text class="teks_tugas">{item.teks}</Text>
                                <Button class="btn_hapus" onClick={hapus_tugas(item.id)}>
                                    ✕
                                </Button>
                            </Row>
                        }
                    }
                </Col>
            </Scroll>

        </Col>
    </Panel>

<style>
    .layar_utama {
        background: #090d16;
        padding: 24px;
    }
    .container_utama {
        gap: 16px;
    }
    .header_box {
        justify-content: space-between;
        align-items: center;
    }
    .judul {
        font-size: 22px;
        color: #f8fafc;
        font-weight: bold;
    }
    .badge {
        font-size: 13px;
        color: #38bdf8;
        background: #0369a133;
        padding: 4px 8px;
        border-radius: 6px;
    }
    .form_box {
        gap: 10px;
    }
    .btn_tambah {
        background: #6366f1;
        color: #ffffff;
        border-radius: 8px;
        padding: 8px 16px;
    }
    .filter_box {
        gap: 8px;
    }
    .btn_filter {
        background: #1e293b;
        color: #94a3b8;
        border-radius: 6px;
        padding: 4px 12px;
    }
    .btn_filter.aktif {
        background: #334155;
        color: #f8fafc;
        font-weight: bold;
    }
    .scroll_area {
        height: 350px;
    }
    .list_col {
        gap: 8px;
    }
    .item_tugas {
        background: #111827;
        border: 1px solid #1f2937;
        padding: 12px;
        border-radius: 10px;
        align-items: center;
        justify-content: space-between;
    }
    .item_tugas.selesai {
        opacity: 0.5;
    }
    .teks_tugas {
        color: #e2e8f0;
        font-size: 14px;
    }
    .btn_hapus {
        background: transparent;
        color: #ef4444;
        font-weight: bold;
    }
</style>
"#;
        fs::write(target_dir.join("todo.brx"), todo_brx)
            .map_err(|e| format!("Failed to write todo.brx: {e}"))?;

        let main_br = r#"use brui
use TodoApp

brui.provider("egui")
app = brui.load("todo.brx")
app.run("Bara Todo Desktop", width = 450, height = 600)
"#;
        fs::write(target_dir.join("main.br"), main_br)
            .map_err(|e| format!("Failed to write main.br: {e}"))?;

        Ok(())
    }

    /// Collect all source files from the project or an explicit entry file
    pub fn collect_sources(&self, entry: Option<&Path>) -> Result<(Vec<SourceFile>, BaraConfig), String> {
        let toml_path = self.project_root.join("bara.toml");
        let config = if toml_path.exists() {
            BaraConfig::from_file(&toml_path)?
        } else {
            let name = self
                .project_root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "bara_app".to_string());
            BaraConfig::default_for(&name)
        };

        let mut sources = Vec::new();
        let mut entry_path = None;

        if let Some(e) = entry {
            let full_e = if e.is_absolute() {
                e.to_path_buf()
            } else {
                self.project_root.join(e)
            };
            if full_e.exists() {
                entry_path = Some(full_e);
            }
        }

        if entry_path.is_none() {
            let default_entry = self.project_root.join(&config.entry);
            if default_entry.exists() {
                entry_path = Some(default_entry);
            }
        }

        // Find all .br and .brx files in project_root (and src/ if exists)
        let mut scanned_files = Vec::new();
        self.scan_dir_for_bara(&self.project_root, &mut scanned_files)?;

        let src_dir = self.project_root.join("src");
        if src_dir.exists() && src_dir.is_dir() {
            self.scan_dir_for_bara(&src_dir, &mut scanned_files)?;
        }

        // Separate definitions from entry file
        let mut def_files = Vec::new();
        let mut entry_file_source = None;

        for path in scanned_files {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

            let is_entry = if let Some(ref ep) = entry_path {
                &path == ep
            } else {
                path.file_name()
                    .map(|f| f == "main.br")
                    .unwrap_or(false)
            };

            if is_entry {
                entry_file_source = Some(SourceFile { path, content });
            } else {
                def_files.push(SourceFile { path, content });
            }
        }

        // Sort definitions alphabetically for determinism
        def_files.sort_by(|a, b| a.path.cmp(&b.path));

        sources.extend(def_files);
        if let Some(ef) = entry_file_source {
            sources.push(ef);
        }

        if sources.is_empty() {
            return Err(format!(
                "No Bara source files (.br or .brx) found in {}",
                self.project_root.display()
            ));
        }

        Ok((sources, config))
    }

    fn scan_dir_for_bara(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "br" || ext == "brx" {
                        if !files.contains(&path) {
                            files.push(path);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Transpiles source files into a unified valid Rust program string
    pub fn transpile_sources(&self, sources: &[SourceFile]) -> Result<String, String> {
        let mut all_items: Vec<Item> = Vec::new();

        for source in sources {
            let tokens = Lexer::new(&source.content).tokenize().map_err(|e| {
                format!("{}: {e}", source.path.display())
            })?;
            let program = Parser::new(tokens).parse_program().map_err(|e| {
                format!("{}: {e}", source.path.display())
            })?;
            all_items.extend(program.items);
        }

        let mut unified_program = Program { items: all_items };

        let mut analyzer = Analyzer::new();
        analyzer
            .analyze_program(&mut unified_program)
            .map_err(|e| format!("Semantic error: {e}"))?;

        let mut codegen = Codegen::new(&analyzer);
        let rust_code = codegen.generate(&unified_program);

        // Verify that the generated code is 100% valid Rust syntax
        if let Err(e) = syn::parse_file(&rust_code) {
            return Err(format!("Generated Rust code syn validation failed: {e}"));
        }

        Ok(rust_code)
    }

    /// Runs static diagnostic analysis on Bara source files
    pub fn diagnose_sources(&self, sources: &[SourceFile]) -> Vec<Diagnostic> {
        let mut unified_program = Program { items: Vec::new() };
        let mut all_diags = Vec::new();

        for source in sources {
            let fname = source.path.display().to_string();
            let tokens = match Lexer::new(&source.content).tokenize() {
                Ok(t) => t,
                Err(e) => {
                    all_diags.push(Diagnostic::error("E0000", e, (1, 1)).with_file(fname));
                    continue;
                }
            };
            let program = match Parser::new(tokens).parse_program() {
                Ok(p) => p,
                Err(e) => {
                    all_diags.push(Diagnostic::error("E0000", e, (1, 1)).with_file(fname));
                    continue;
                }
            };
            unified_program.items.extend(program.items);
        }

        let mut debugger = BaraDebugger::new();
        debugger.inspect(&unified_program, None);
        all_diags.extend(debugger.diagnostics);
        all_diags
    }

    /// Run `bara check` on the project
    pub fn check(&self, entry: Option<&Path>) -> Result<(), String> {
        let (sources, _config) = self.collect_sources(entry)?;
        println!("Checking {} Bara source file(s)...", sources.len());

        let diags = self.diagnose_sources(&sources);
        let mut has_errors = false;
        for d in &diags {
            let source_content = sources
                .iter()
                .find(|s| d.file.as_ref().map(|f| f == &s.path.display().to_string()).unwrap_or(true))
                .map(|s| s.content.as_str())
                .unwrap_or("");
            eprintln!("{}", d.render(source_content));
            if d.severity == Severity::Error {
                has_errors = true;
            }
        }

        if has_errors {
            return Err("Bara pre-compilation check failed with errors.".to_string());
        }

        let rust_code = self.transpile_sources(&sources)?;

        // Ensure syntax parses
        syn::parse_file(&rust_code)
            .map_err(|e| format!("Rust syntax verification failed: {e}"))?;

        println!("✓ All syntax, types, and semantics verified successfully.");
        Ok(())
    }

    /// Transpile and return the Rust code
    pub fn transpile(&self, entry: Option<&Path>) -> Result<String, String> {
        let (sources, _config) = self.collect_sources(entry)?;
        self.transpile_sources(&sources)
    }

    /// Prepare `.bara_cache/` workspace directory with `Cargo.toml` and `src/main.rs`
    pub fn setup_cache(
        &self,
        config: &BaraConfig,
        rust_code: &str,
    ) -> Result<PathBuf, String> {
        let src_dir = self.cache_dir.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| {
            format!("Failed to create cache directory {}: {e}", src_dir.display())
        })?;

        let cargo_toml_content = self.generate_cargo_toml(config, rust_code);
        fs::write(self.cache_dir.join("Cargo.toml"), cargo_toml_content).map_err(|e| {
            format!("Failed to write {}: {e}", self.cache_dir.join("Cargo.toml").display())
        })?;

        let main_rs_path = src_dir.join("main.rs");
        fs::write(&main_rs_path, rust_code).map_err(|e| {
            format!("Failed to write {}: {e}", main_rs_path.display())
        })?;

        Ok(main_rs_path)
    }

    /// Generate `.bara_cache/Cargo.toml` with presets and dependencies
    pub fn generate_cargo_toml(&self, config: &BaraConfig, rust_code: &str) -> String {
        let mut out = String::new();
        out.push_str("[package]\n");
        out.push_str(&format!("name = \"{}\"\n", config.name.replace('-', "_")));
        out.push_str(&format!("version = \"{}\"\n", config.version));
        out.push_str("edition = \"2021\"\n\n");

        out.push_str("[dependencies]\n");

        let mut deps = config.dependencies.clone();

        // Auto-detect GUI presets (egui, eframe)
        if config.presets.contains("gui")
            || rust_code.contains("eframe::App")
            || rust_code.contains("egui::")
            || rust_code.contains("brui")
        {
            deps.entry("eframe".to_string())
                .or_insert_with(|| "\"0.27\"".to_string());
            deps.entry("egui".to_string())
                .or_insert_with(|| "\"0.27\"".to_string());
        }

        // Auto-detect async / tokio preset
        if config.presets.contains("async")
            || rust_code.contains("tokio::")
            || rust_code.contains("async fn")
        {
            deps.entry("tokio".to_string())
                .or_insert_with(|| "{ version = \"1\", features = [\"full\"] }".to_string());
        }

        // Auto-detect serde preset
        if config.presets.contains("serde") || rust_code.contains("serde::") {
            deps.entry("serde".to_string())
                .or_insert_with(|| "{ version = \"1.0\", features = [\"derive\"] }".to_string());
            deps.entry("serde_json".to_string())
                .or_insert_with(|| "\"1.0\"".to_string());
        }

        for (k, v) in &deps {
            if v.starts_with('{') {
                out.push_str(&format!("{k} = {v}\n"));
            } else if v.starts_with('"') {
                out.push_str(&format!("{k} = {v}\n"));
            } else {
                out.push_str(&format!("{k} = \"{v}\"\n"));
            }
        }

        out
    }

    /// Build the project with `cargo build`
    pub fn build(
        &self,
        entry: Option<&Path>,
        release: bool,
        offline: bool,
    ) -> Result<PathBuf, String> {
        let (sources, config) = self.collect_sources(entry)?;
        let rust_code = self.transpile_sources(&sources)?;
        self.setup_cache(&config, &rust_code)?;

        println!("Compiling Bara project '{}' via Cargo Bridge...", config.name);

        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.current_dir(&self.cache_dir);

        if release {
            cmd.arg("--release");
        }
        if offline {
            cmd.arg("--offline");
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to invoke cargo: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let translated = self.translate_errors(&stderr, &sources);
            return Err(format!("Cargo build failed:\n{translated}"));
        }

        let mode = if release { "release" } else { "debug" };
        let binary_name = config.name.replace('-', "_");
        let bin_path = self.cache_dir.join("target").join(mode).join(&binary_name);

        // Copy binary to project target/ or root
        let out_dir = self.project_root.join("target").join(mode);
        let _ = fs::create_dir_all(&out_dir);
        let dest = out_dir.join(&binary_name);
        if bin_path.exists() {
            let _ = fs::copy(&bin_path, &dest);
        }

        println!("✓ Build succeeded: {}", dest.display());
        Ok(dest)
    }

    /// Run the project
    pub fn run(
        &self,
        entry: Option<&Path>,
        release: bool,
        offline: bool,
        app_args: &[String],
    ) -> Result<i32, String> {
        let (sources, config) = self.collect_sources(entry)?;
        let rust_code = self.transpile_sources(&sources)?;
        self.setup_cache(&config, &rust_code)?;

        println!("Running Bara project '{}'...", config.name);

        let mut cmd = Command::new("cargo");
        cmd.arg("run");
        cmd.current_dir(&self.cache_dir);

        if release {
            cmd.arg("--release");
        }
        if offline {
            cmd.arg("--offline");
        }

        if !app_args.is_empty() {
            cmd.arg("--");
            for a in app_args {
                cmd.arg(a);
            }
        }

        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to run cargo: {e}"))?;

        let status = child
            .wait()
            .map_err(|e| format!("Failed to wait for process: {e}"))?;

        Ok(status.code().unwrap_or(0))
    }

    /// Add a dependency to `bara.toml`
    pub fn add_dependency(&self, crate_name: &str, version: Option<&str>) -> Result<(), String> {
        let toml_path = self.project_root.join("bara.toml");
        let mut config = if toml_path.exists() {
            BaraConfig::from_file(&toml_path)?
        } else {
            let name = self
                .project_root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "bara_app".to_string());
            BaraConfig::default_for(&name)
        };

        let ver = version.unwrap_or("*");
        config.dependencies.insert(crate_name.to_string(), ver.to_string());
        config.save(&toml_path)?;

        println!("✓ Added dependency '{crate_name} = \"{ver}\"' to bara.toml");
        Ok(())
    }

    /// Translates cargo/rustc error outputs back to original Bara source lines
    pub fn translate_errors(&self, stderr: &str, sources: &[SourceFile]) -> String {
        let mut translated = String::new();

        for line in stderr.lines() {
            let mut matched = false;
            for prefix in &[".bara_cache/src/main.rs:", "src/main.rs:"] {
                if let Some(idx) = line.find(prefix) {
                    let after = &line[idx + prefix.len()..];
                    if let Some((line_num_str, rest)) = after.split_once(':') {
                        if let Ok(line_num) = line_num_str.parse::<usize>() {
                            let mapped = self.find_source_origin(line_num, sources);
                            translated.push_str(&line[..idx]);
                            translated.push_str(&mapped);
                            translated.push(':');
                            translated.push_str(line_num_str);
                            translated.push(':');
                            translated.push_str(rest);
                            translated.push('\n');
                            matched = true;
                            break;
                        }
                    }
                }
            }
            if !matched {
                translated.push_str(line);
                translated.push('\n');
            }
        }

        translated
    }

    fn find_source_origin(&self, _gen_line: usize, sources: &[SourceFile]) -> String {
        // If single source, point to that source
        if sources.len() == 1 {
            return sources[0].path.display().to_string();
        }
        // If multiple sources, default to the entry file or first source
        if let Some(entry) = sources.last() {
            entry.path.display().to_string()
        } else {
            "bara_source".to_string()
        }
    }
}
