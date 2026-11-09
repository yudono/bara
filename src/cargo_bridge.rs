use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::analyzer::Analyzer;
use crate::ast::{Expr, Item, Program, Stmt};
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

    /// Initialize a new Bara project template (blank hello-world, fully dynamic).
    /// Core never hardcodes any demo app: the template is an empty hello-world
    /// that works for CLI and GUI without bundling TodoApp or any other demo.
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

        // Blank hello-world template: no hardcoded demo app, no TodoApp.
        // Users add their own `type`, `view`, `layout.xml`, `styles.css` files.
        let main_br = r#"print("Hello from Bara!")
"#;
        fs::write(target_dir.join("main.br"), main_br)
            .map_err(|e| format!("Failed to write main.br: {e}"))?;

        Ok(())
    }

    /// Collect all source files from the project or an explicit entry file
    /// Scans recursively (excluding `target/`, `.bara_cache/`, `.git/`) and
    /// collects `.br`, `.brx`, `.xml` (Brui layout) and `.css` (Brui style).
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

        // Find all .br/.brx/.xml/.css files recursively (excluding build caches)
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
        // Recursive scan so `pages/auth/login.brx`, `pages/dashboard/*.brx`,
        // `layout.xml`, `styles.css`, etc. are all discovered consistently.
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name == "target" || name == ".bara_cache" || name == ".git" {
                        continue;
                    }
                }
                self.scan_dir_for_bara(&path, files)?;
                continue;
            }
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ext == "br" || ext == "brx" || ext == "xml" || ext == "css" {
                        if !files.contains(&path) {
                            files.push(path);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Transpiles source files into a unified valid Rust program string.
    /// `.css` stylesheets are EXPLICIT: a project only gets a stylesheet when
    /// some `.br`/`.brx` file contains `use "styles.css"`, `use styles.css`
    /// or shorthand `use styles`. Without such an import the `.css` file is
    /// ignored (never auto-loaded). This keeps core fully dynamic.
    pub fn transpile_sources(&self, sources: &[SourceFile]) -> Result<String, String> {
        // Pass 0: parse Bara sources first to discover explicit stylesheet imports.
        let mut bara_programs: Vec<(usize, Program)> = Vec::new();
        let mut css_imports: Vec<String> = Vec::new();
        for (idx, source) in sources.iter().enumerate() {
            let ext = source
                .path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if ext == "br" || ext == "brx" {
                let tokens = Lexer::new(&source.content).tokenize().map_err(|e| {
                    format!("{}: {e}", source.path.display())
                })?;
                let program = Parser::new(tokens).parse_program().map_err(|e| {
                    format!("{}: {e}", source.path.display())
                })?;
                for item in &program.items {
                    if let Item::Use(u) = item {
                        if let Some(css_file) = css_import_filename(&u.path) {
                            if !css_imports.contains(&css_file) {
                                css_imports.push(css_file);
                            }
                        }
                    }
                }
                bara_programs.push((idx, program));
            }
        }

        let mut all_items: Vec<Item> = Vec::new();

        // Boot order: the app boots the view named by `brui.load("*.brx")` in
        // the entry program. Move that file's program first so its view is the
        // `find_loaded_app_target` pick. Order-independent passes are unaffected.
        let mut load_order: Vec<usize> = (0..bara_programs.len()).collect();
        if let Some(loaded) = find_loaded_brx(&bara_programs) {
            load_order.sort_by_key(|&pos| {
                let (idx, _) = &bara_programs[pos];
                let stem_match = sources
                    .get(*idx)
                    .and_then(|s| s.path.file_name())
                    .and_then(|n| n.to_str())
                    == Some(loaded.as_str());
                if stem_match { 0 } else { 1 }
            });
        }
        let ordered_programs: Vec<(usize, Program)> = load_order
            .into_iter()
            .map(|pos| {
                let (idx, prog) = &bara_programs[pos];
                (*idx, prog.clone())
            })
            .collect();
        let bara_programs = ordered_programs;

        for source in sources {
            let ext = source
                .path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if ext == "css" {
                // Explicit-only: include iff some Bara file imports it.
                let fname = source
                    .path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let stem = source
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let wanted = css_imports.iter().any(|imp| {
                    imp == fname || imp == stem || imp == &format!("{stem}.css")
                });
                if wanted {
                    all_items.push(Item::StyleBlock(source.content.clone(), (1, 1)));
                }
                continue;
            }
            if ext == "xml" {
                // Brui XML layout: parse into a ViewDef
                let default_name = source
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("BruiView");
                let default_name = capitalize_name(default_name);
                match crate::brui_xml::parse_brui_xml(&source.content, &default_name) {
                    Ok(view) => all_items.push(Item::View(view)),
                    Err(e) => {
                        return Err(format!("{}: {e}", source.path.display()));
                    }
                }
                continue;
            }
            if ext == "br" || ext == "brx" {
                continue; // already parsed above; merged below
            }
            let tokens = Lexer::new(&source.content).tokenize().map_err(|e| {
                format!("{}: {e}", source.path.display())
            })?;
            let program = Parser::new(tokens).parse_program().map_err(|e| {
                format!("{}: {e}", source.path.display())
            })?;
            all_items.extend(program.items);
        }

        for (_, prog) in bara_programs {
            all_items.extend(prog.items);
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
    /// `.css` files are always valid stylesheets; `.xml` files are validated
    /// with the Brui XML parser so shared `styles.css` + multi-page `.brx`
    /// projects report accurate diagnostics.
    pub fn diagnose_sources(&self, sources: &[SourceFile]) -> Vec<Diagnostic> {
        let mut debugger = BaraDebugger::new();
        let mut parsed_sources = Vec::new();
        let mut all_diags = Vec::new();

        for source in sources {
            let fname = source.path.display().to_string();
            let ext = source
                .path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if ext == "css" {
                // Shared stylesheet: validate with CSS engine (never an error, just parse).
                let _ = crate::css_engine::CssStyleSheet::parse(&source.content);
                continue;
            }
            if ext == "xml" {
                let default_name = source
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("BruiView");
                let default_name = capitalize_name(default_name);
                match crate::brui_xml::parse_brui_xml(&source.content, &default_name) {
                    Ok(view) => {
                        debugger.known_types.insert(view.type_name.clone());
                        debugger.known_globals.insert(view.type_name.clone());
                        parsed_sources.push((
                            fname,
                            Program {
                                items: vec![Item::View(view)],
                            },
                        ));
                    }
                    Err(e) => {
                        all_diags.push(Diagnostic::error("E0000", e, (1, 1)).with_file(fname));
                    }
                }
                continue;
            }
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

            parsed_sources.push((fname, program));
        }

        // Pre-register global types, views and fields from ALL files first, so
        // multi-file projects (parent views using child components defined in
        // other files) resolve no matter the file order.
        for (_, program) in &parsed_sources {
            for item in &program.items {
                match item {
                    Item::Type(t) => {
                        debugger.known_types.insert(t.name.clone());
                        debugger.known_globals.insert(t.name.clone());
                        debugger.known_fields.insert(
                            t.name.clone(),
                            t.fields.iter().map(|f| f.name.clone()).collect(),
                        );
                    }
                    Item::Fn(f) => {
                        debugger.known_globals.insert(f.name.clone());
                    }
                    Item::View(v) => {
                        debugger.known_types.insert(v.type_name.clone());
                        debugger.known_globals.insert(v.type_name.clone());
                        debugger.known_views.insert(v.type_name.clone());
                    }
                    _ => {}
                }
            }
        }

        for (fname, program) in &parsed_sources {
            debugger.inspect(program, Some(fname));
        }

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
        self.setup_cache_with_sources(config, rust_code, &[])
    }

    /// Prepare `.bara_cache/` workspace directory with `Cargo.toml`, `src/main.rs`, and component transpiles
    pub fn setup_cache_with_sources(
        &self,
        config: &BaraConfig,
        rust_code: &str,
        sources: &[SourceFile],
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

        // Also transpile and write individual component/module files in .bara_cache/src/ (e.g. models.rs)
        for source in sources {
            let stem = source.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if !stem.is_empty() && stem != "main" {
                if let Ok(tokens) = Lexer::new(&source.content).tokenize() {
                    if let Ok(mut prog) = Parser::new(tokens).parse_program() {
                        let mut single_analyzer = Analyzer::new();
                        let _ = single_analyzer.analyze_program(&mut prog);
                        let mut single_codegen = Codegen::new(&single_analyzer);
                        let single_rs = single_codegen.generate(&prog);
                        let dest_path = src_dir.join(format!("{stem}.rs"));
                        let _ = fs::write(dest_path, single_rs);
                    }
                }
            }
        }

        Ok(main_rs_path)
    }

    /// Emits transpiled Rust files from .bara_cache/src to destination folder
    pub fn emit_rs_to(&self, out_dir: &Path, entry: Option<&Path>) -> Result<Vec<PathBuf>, String> {
        let (sources, config) = self.collect_sources(entry)?;
        let rust_code = self.transpile_sources(&sources)?;
        self.setup_cache_with_sources(&config, &rust_code, &sources)?;

        let dest_dir = if out_dir.is_absolute() {
            out_dir.to_path_buf()
        } else {
            self.project_root.join(out_dir)
        };
        fs::create_dir_all(&dest_dir).map_err(|e| {
            format!("Failed to create output directory {}: {e}", dest_dir.display())
        })?;

        let cache_src = self.cache_dir.join("src");
        let mut emitted = Vec::new();
        if let Ok(entries) = fs::read_dir(&cache_src) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().map(|e| e == "rs").unwrap_or(false) {
                    if let Some(fname) = p.file_name() {
                        let target = dest_dir.join(fname);
                        if fs::copy(&p, &target).is_ok() {
                            emitted.push(target);
                        }
                    }
                }
            }
        }
        emitted.sort();
        Ok(emitted)
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
        // NOTE: eframe's `accesskit` feature is deliberately OFF. It pulls
        // accesskit_macos -> objc2 0.3-beta, whose fast-enumeration encoding
        // (`countByEnumeratingWithState` expecting 'q') aborts at launch on
        // newer macOS against Swift-backed NSScreen arrays (expected 'Q').
        // Everything else mirrors eframe's default features. Users who need
        // screen-reader support can opt back in via bara.toml:
        //   eframe = { version = "0.27", features = ["accesskit", ...] }
        if config.presets.contains("gui")
            || rust_code.contains("eframe::App")
            || rust_code.contains("egui::")
            || rust_code.contains("brui")
        {
            deps.entry("eframe".to_string()).or_insert_with(|| {
                "{ version = \"0.27\", default-features = false, features = [\"default_fonts\", \"glow\", \"wayland\", \"web_screen_reader\", \"x11\"] }".to_string()
            });
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

        // GUI on macOS: objc2's message-send verification is `#[cfg(debug_assertions)]`
        // and false-positives on Swift-overlay classes (NSScreen array reports 'Q'
        // where the declared NSUInteger maps to 'q' in its table), aborting the app
        // during winit launch in DEV builds. Release builds skip the check and run
        // fine (the ABI itself is correct). Disable debug-assertions for generated
        // GUI projects so plain `bara run` opens a window instead of aborting.
        // Overflow checks stay on (separate profile flag). CLI projects are untouched.
        let is_gui = config.presets.contains("gui")
            || rust_code.contains("eframe::App")
            || rust_code.contains("egui::")
            || rust_code.contains("brui");
        if is_gui {
            out.push_str("\n[profile.dev]\ndebug-assertions = false\n");
            out.push_str("[profile.release]\ndebug-assertions = false\n");
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
        self.setup_cache_with_sources(&config, &rust_code, &sources)?;

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
        self.setup_cache_with_sources(&config, &rust_code, &sources)?;

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

        match status.code() {
            Some(code) => Ok(code),
            None => {
                // Killed by a signal (e.g. SIGABRT from a non-unwinding panic).
                // GUI apps on macOS can hit the upstream eframe 0.27 / winit 0.29 /
                // icrate 0.0.4 screen-enumeration abort; Bara-generated code is
                // equivalent to the eframe hello-world template.
                eprintln!(
                    "The app was terminated by an OS signal (no exit code). \
                    If this is a GUI app on macOS and you see `countByEnumeratingWithState` \
                    or `panic in a function that cannot unwind`, it is the known upstream \
                    eframe 0.27 / winit screen-enumeration abort, not Bara logic. \
                    Workarounds: run with `-- --headless` (or BARA_HEADLESS=1) to verify \
                    without opening a window, or re-run to retry."
                );
                Ok(134)
            }
        }
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

fn capitalize_name(s: &str) -> String {
  let mut c = s.chars();
  match c.next() { Some(f) => format!("{}{}", f.to_ascii_uppercase(), c.collect::<String>()), None => "BruiView".to_string() }
}

/// Find the `.brx` file named by `brui.load("*.brx")` in any program's
/// top-level statements, so the boot view can be ordered first.
/// Handles both `brui.load("x.brx")` and `app = brui.load("x.brx")` forms.
fn find_loaded_brx(bara_programs: &[(usize, Program)]) -> Option<String> {
    for (_, prog) in bara_programs {
        for item in &prog.items {
            let call_opt = match item {
                Item::TopLevelStmt(Stmt::Expr(Expr::MethodCall { method, args, .. })) => {
                    if method == "load" { Some(args) } else { None }
                }
                Item::TopLevelStmt(Stmt::Assign { value, .. }) => {
                    let inner: &Expr = value;
                    if let Expr::MethodCall { method, args, .. } = inner {
                        if method == "load" { Some(args) } else { None }
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(args) = call_opt {
                if let Some(first) = args.first() {
                    if let Expr::Literal(crate::ast::Literal::Str(name), _) = first {
                        if name.ends_with(".brx") {
                            return Some(name.clone());
                        }
                    }
                }
            }
        }
    }
    None
}
/// Returns `Some(css_filename)` when a `use` path is an explicit stylesheet
/// import (`use "styles.css"`, `use styles.css`, or shorthand `use styles`).
/// Otherwise returns `None` (a normal Rust/module import).
fn css_import_filename(path: &[String]) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    // String form: use "styles.css" -> path == ["styles.css"]
    if path.len() == 1 {
        let s = path[0].trim();
        if s.ends_with(".css") {
            // Keep only the file name (allow "pages/styles.css" too).
            let fname = s.rsplit('/').next().unwrap_or(s);
            return Some(fname.to_string());
        }
        if s == "styles" || s == "style" || s == "stylesheet" {
            return Some("styles.css".to_string());
        }
    }
    // Dotted form: use styles.css -> path == ["styles", "css"]
    if path.len() == 2 && path[1] == "css" {
        return Some(format!("{}.css", path[0]));
    }
    if path.last().map(|s| s == "css").unwrap_or(false) {
        let stem = path.first().cloned().unwrap_or_else(|| "styles".to_string());
        return Some(format!("{stem}.css"));
    }
    if path.iter().any(|s| s.ends_with(".css")) {
        let raw = path.iter().find(|s| s.ends_with(".css")).unwrap();
        let fname = raw.rsplit('/').next().unwrap_or(raw);
        return Some(fname.to_string());
    }
    None
}

/// True when a `use` statement refers to a stylesheet (never emitted as Rust).
pub fn is_stylesheet_use(path: &[String]) -> bool {
    css_import_filename(path).is_some()
}
