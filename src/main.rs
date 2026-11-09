use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use bara::cargo_bridge::CargoBridge;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        r#"Bara - The Unified Native GUI & Systems Language (v{VERSION})

USAGE:
    bara <SUBCOMMAND> [OPTIONS] [ARGS]

SUBCOMMANDS:
    init [NAME]              Initialize a new Bara project with templates
    check [FILE]             Check Bara program for syntax and semantic errors
    debug [FILE]             Run Bara diagnostic engine and linter
    build [FILE] [OPTIONS]   Compile Bara program to a native binary
    run [FILE] [OPTIONS]     Run a Bara application
    transpile [FILE] [-o]    Transpile Bara code to Rust without building
    add <CRATE> [VERSION]    Add a dependency to bara.toml
    help                     Print this help message
    version                  Print version information

OPTIONS:
    --release                Build in release mode (optimizations enabled)
    --offline                Run cargo in offline mode
    -o, --output <PATH>      Output path for binary or transpiled Rust file
    -h, --help               Print help information
    -V, --version            Print version information

EXAMPLES:
    bara init my_todo
    cd my_todo && bara check
    bara transpile main.br -o output.rs
    bara run main.br
    bara add serde "1.0"
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "-h" | "--help" | "help" => {
            print_help();
        }
        "-V" | "--version" | "version" => {
            println!("bara {VERSION}");
        }
        "init" => {
            let path_arg = if args.len() > 2 && !args[2].starts_with('-') {
                args[2].clone()
            } else {
                "bara_project".to_string()
            };

            let target_dir = PathBuf::from(&path_arg);
            let name = target_dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "bara_project".to_string());

            println!("Initializing new Bara project in '{}'...", target_dir.display());
            if let Err(e) = CargoBridge::init_project(&name, &target_dir) {
                eprintln!("Error: {e}");
                process::exit(1);
            }
            println!("✓ Project '{}' created successfully!", name);
            println!("To get started:");
            println!("  cd {}", target_dir.display());
            println!("  bara check");
            println!("  bara run");
        }
        "check" => {
            let entry = parse_entry_arg(&args, 2);
            let root = current_project_root(entry.as_deref());
            let bridge = CargoBridge::new(root);

            if let Err(e) = bridge.check(entry.as_deref()) {
                eprintln!("Check failed:\n{e}");
                process::exit(1);
            }
        }
        "debug" | "lint" => {
            let entry = parse_entry_arg(&args, 2);
            let root = current_project_root(entry.as_deref());
            let bridge = CargoBridge::new(root);

            match bridge.collect_sources(entry.as_deref()) {
                Ok((sources, _)) => {
                    println!("Debugging {} source file(s) with Bara Diagnostic Engine...", sources.len());
                    let diags = bridge.diagnose_sources(&sources);
                    let mut error_count = 0;
                    let mut warn_count = 0;

                    for d in &diags {
                        let source_content = sources
                            .iter()
                            .find(|s| d.file.as_ref().map(|f| f == &s.path.display().to_string()).unwrap_or(true))
                            .map(|s| s.content.as_str())
                            .unwrap_or("");
                        eprintln!("{}", d.render(source_content));
                        match d.severity {
                            bara::debugger::Severity::Error => error_count += 1,
                            bara::debugger::Severity::Warning => warn_count += 1,
                            bara::debugger::Severity::Info => {}
                        }
                    }

                    if error_count > 0 {
                        eprintln!("Found {error_count} error(s), {warn_count} warning(s).");
                        process::exit(1);
                    } else if warn_count > 0 {
                        println!("✓ No errors found. ({warn_count} warning(s))");
                    } else {
                        println!("✓ Zero diagnostics: code is completely clean!");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read sources: {e}");
                    process::exit(1);
                }
            }
        }
        "transpile" => {
            let entry = parse_entry_arg(&args, 2);
            let output_path = parse_flag_val(&args, "-o").or_else(|| parse_flag_val(&args, "--output"));

            let root = current_project_root(entry.as_deref());
            let bridge = CargoBridge::new(root);

            match bridge.transpile(entry.as_deref()) {
                Ok(rust_code) => {
                    if let Some(out_p) = output_path {
                        if let Err(e) = fs::write(&out_p, &rust_code) {
                            eprintln!("Error writing output file {out_p}: {e}");
                            process::exit(1);
                        }
                        println!("✓ Transpiled Rust code written to {out_p}");
                    } else {
                        println!("{rust_code}");
                    }
                }
                Err(e) => {
                    eprintln!("Transpilation failed:\n{e}");
                    process::exit(1);
                }
            }
        }
        "build" => {
            let entry = parse_entry_arg(&args, 2);
            let release = has_flag(&args, "--release");
            let offline = has_flag(&args, "--offline");
            let output_path = parse_flag_val(&args, "-o").or_else(|| parse_flag_val(&args, "--output"));

            let root = current_project_root(entry.as_deref());
            let bridge = CargoBridge::new(root);

            match bridge.build(entry.as_deref(), release, offline) {
                Ok(bin_path) => {
                    if let Some(out_p) = output_path {
                        let dest = PathBuf::from(out_p);
                        if let Err(e) = fs::copy(&bin_path, &dest) {
                            eprintln!("Failed to copy binary to {}: {e}", dest.display());
                        } else {
                            println!("✓ Binary copied to {}", dest.display());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Build failed:\n{e}");
                    process::exit(1);
                }
            }
        }
        "run" => {
            let entry = parse_entry_arg(&args, 2);
            let release = has_flag(&args, "--release");
            let offline = has_flag(&args, "--offline");

            // Extract app arguments after `--`
            let mut app_args = Vec::new();
            if let Some(pos) = args.iter().position(|a| a == "--") {
                app_args.extend(args[pos + 1..].iter().cloned());
            }

            let root = current_project_root(entry.as_deref());
            let bridge = CargoBridge::new(root);

            match bridge.run(entry.as_deref(), release, offline, &app_args) {
                Ok(code) => process::exit(code),
                Err(e) => {
                    eprintln!("Run failed:\n{e}");
                    process::exit(1);
                }
            }
        }
        "add" => {
            if args.len() < 3 {
                eprintln!("Error: 'bara add' requires a crate name.");
                eprintln!("Usage: bara add <crate> [version]");
                process::exit(1);
            }
            let crate_name = &args[2];
            let version = if args.len() > 3 && !args[3].starts_with('-') {
                Some(args[3].as_str())
            } else {
                None
            };

            let root = current_project_root(None);
            let bridge = CargoBridge::new(root);
            if let Err(e) = bridge.add_dependency(crate_name, version) {
                eprintln!("Failed to add dependency: {e}");
                process::exit(1);
            }
        }
        unknown => {
            eprintln!("Unknown command: '{unknown}'");
            print_help();
            process::exit(1);
        }
    }
}

fn parse_entry_arg(args: &[String], start: usize) -> Option<PathBuf> {
    for arg in &args[start..] {
        if arg == "--" {
            break;
        }
        if !arg.starts_with('-') {
            return Some(PathBuf::from(arg));
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn parse_flag_val(args: &[String], flag: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

fn current_project_root(entry: Option<&Path>) -> PathBuf {
    if let Some(e) = entry {
        if e.is_file() {
            if let Some(parent) = e.parent() {
                if !parent.as_os_str().is_empty() {
                    return parent.to_path_buf();
                }
            }
        } else if e.is_dir() {
            return e.to_path_buf();
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
