# Bara Language Transpiler & Toolchain

**Bara** (`.br`, `.brx`, `brui`) is a high-level, expressive systems language that compiles directly to 100% compatible native Rust, targeting `egui`/`eframe` for cross-platform GUI applications with compile-time CSS styling.

> Built with AI assistance.

---

## Architecture & Components

Bara is designed with a multi-pass pipeline:

1. **Lexer (`src/lexer.rs`)**:
   - Python-style off-side rule indentation tracking (`INDENT`, `DEDENT`, `NEWLINE`).
   - Suppresses indentation tokens inside parentheses, square brackets, and curly braces.
   - Supports raw Rust blocks (`raw_rust { ... }`), unicode symbols (e.g. `⚡`, `✕`), and embedded `<style>` blocks in `.brx` files.

2. **AST (`src/ast.rs`)**:
   - Strongly-typed Abstract Syntax Tree representing Bara programs, expressions, statements, types, methods, JSX elements, JSX control flow (`{if}`, `{for}`), ternary expressions (`cond ? a : b`), inline closures (`t => expr`), and assignment expressions.

3. **Parser (`src/parser.rs`)**:
   - Recursive-descent parser handling items, functions, types, methods, expressions, and JSX tags.
   - Supports pipeline operator (`|>`), string interpolation (`"Sisa: {hitung_aktif()} tugas"`), JSX child expressions, and conditional branches.

4. **Semantic Analyzer (`src/analyzer.rs`)**:
   - **Pass 1 (Type & Struct Registry)**: Registers all types, fields, and default values.
   - **Pass 2 (Function & Parameter Analysis)**: Infers borrowing (`&` vs `&mut`) and mutability based on whether variables are mutated or moved.
   - **Pass 3 (JSX & Self Resolution)**: Resolves unqualified identifiers in methods and JSX view definitions to `self.` fields.
   - **Pass 4 (Top-level Analysis & Last-Use)**: Tracks variables, mutability (`let` vs `let mut`), and moves/clones.
   - **Dynamic Fallback (`BaraDyn`)**: Automatically wraps heterogeneous list elements into `BaraDyn` variants (`Int`, `Float`, `Str`, `Bool`).

5. **Compile-Time CSS Engine (`src/css_engine.rs`)**:
   - Parses CSS declarations (background, color, font-size, font-weight, padding, border-radius, border/stroke, width, height, gap).
   - Maps styles directly to `egui::Frame`, `egui::Color32`, `egui::Margin`, `egui::Rounding`, `egui::Stroke`, and `egui::RichText`.

6. **Rust Code Generator (`src/codegen.rs`)**:
   - Emits Rust AST and pretty-prints it via `prettyplease`.
   - Generates struct definitions, `Default` implementations, constructor helper functions, method implementations, and `eframe::App` implementations for `.brx` views.
   - Generates the `fn main()` entry point for `app.run(...)` with native viewport window settings.

7. **Cargo Bridge (`src/cargo_bridge.rs`)**:
   - Project workspace manager creating and maintaining `.bara_cache/`.
   - Auto-generates `Cargo.toml` with presets for `egui`, `eframe`, `tokio`, and `serde`.
   - Maps compile errors back to original Bara source files and line numbers.

8. **CLI Toolchain (`src/main.rs`)**:
   - `bara init [name]`: Scaffolds a new Bara project.
   - `bara check [file]`: Runs full lexer, parser, analyzer, and codegen validation.
   - `bara transpile [file] [-o output.rs]`: Transpiles Bara source code to formatted Rust.
   - `bara build [file] [--release]`: Transpiles and invokes `cargo build`.
   - `bara run [file] [--release]`: Runs the Bara native application.
   - `bara add <crate> [version]`: Adds crate dependencies to `bara.toml`.

---

## 8 Core Transpilation Rules

1. **Zero-Boilerplate Variables**: Variables are declared directly without `let`. The analyzer tracks mutation and emits `let` or `let mut`.
2. **Fallback Dynamic Types (`BaraDyn`)**: Heterogeneous lists (e.g. `[1, "dua", 3.0]`) are typed as `Vec<BaraDyn>` with enum variants.
3. **Auto-Borrowing**: Function parameters default to borrowed references (`&T` or `&mut T`) unless ownership transfer is required.
4. **Last-Use Optimization**: Non-primitive variables used after a transfer are `.clone()`d, while the last use moves ownership.
5. **Unified Type System**: `type Name = ...` generates a Rust `struct`, `impl Default`, constructor function, and `impl` block with automatic `self.` field resolution.
6. **String Interpolation & Pipelines**: `"Nilai: {x}"` generates `format!("Nilai: {}", x)`, and `data |> process()` generates `process(data)`.
7. **Error & Async Inference**: Functions containing `!` or try expressions return `Result<T, Box<dyn std::error::Error>>`, and asynchronous blocks configure `tokio`.
8. **Crate Interop & Raw Rust**: `use crate_name` imports external crates, and `raw_rust { ... }` injects verbatim Rust code.

---

## Todo App Example

Located in `examples/todo_app/`:

### `todo.br` (Data Model & Logic)
```python
type TodoItem =
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
```

### `todo.brx` (Native GUI & CSS)
```jsx
use eframe
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
```

### `main.br` (App Entry Point)
```python
use brui
use TodoApp

brui.provider("egui")
app = brui.load("todo.brx")
app.run("Bara Todo Desktop", width = 450, height = 600)
```

---

## Project Structure

```
examples/
  todo_app/            # Todo list: single-file todo.brx (type + methods + view + inline <style>)
  dashboard_gui/     # Sidebar cloud dashboard: single-file dashboard.brx shell + pages/** components
    pages/
      auth/          # login.brx, register.brx (single-file auth components)
      dashboard/     # overview.brx (metrics/logs/search), users.brx (node CRUD)
  brui_xml_gui/        # XML-layout variant: app.br + layout.xml + shared styles.css
  react_tree/          # React-style tree: demo.brx + counter.brx (props, useEffect, lifecycle)
  cli_algorithms/      # Fibonacci, primes, sorting, binary search, GCD (CLI)
  backend_api/         # HttpRequest/Response + ApiServer dispatch demo (CLI)
  multi_lib/           # Multi-module imports: libs/{math_ops,string_utils,models}.br
  object_handling/     # Structs, methods, nested composition (models.br + main.br)
src/                   # Compiler: lexer, parser, analyzer, css_engine, codegen, cargo_bridge
tests/                 # Unit + integration suites (see docs/testing.md)
docs/                  # brui_components.md, brui_styling.md, examples.md, testing.md
```

Standalone rewrite project (sibling of this repo, not under `examples/`):
`../Solanum_bara/` — Solanum GNOME pomodoro timer rewritten in full Bara
(Timer/Settings/About components, `LapType` enum + `match`, frame-based
ticking via `onUpdate`). Proves Bara covers real-world Rust app patterns.

Every example ships a `bara.toml` with `[package]` + `[presets]` + `[dependencies]`
tables; `[presets]` selects toolchain behavior (`gui` pulls `eframe`/`egui`).

Generated GUI projects disable eframe's `accesskit` feature (its screen-reader
stack aborted at launch on newer macOS) and set `debug-assertions = false` for
the same reason (objc2's message verification is debug-only and false-positives
on Swift-overlay NSScreen arrays). Re-enable accessibility explicitly in
`bara.toml` if you need it:
`eframe = { version = "0.27", features = ["accesskit", ...] }`.

### Shared `styles.css` concept (explicit import)

A `.css` file is only compiled in when some `.br`/`.brx` file explicitly
imports it — core never auto-loads stylesheets. Add one line to `main.br`
(or `app.br`):

```python
use "styles.css"
```

`use styles.css` and shorthand `use styles` work too. `dashboard_gui/styles.css`
and `brui_xml_gui/styles.css` therefore style every `.brx` page and XML layout
in their project only because their `main.br` imports them — no per-`.brx`
`<style>` block needed. Inline `<style>` (as in `todo.brx`) merges into the
same sheet. Without the `use` line the `.css` file is ignored.
See `docs/brui_styling.md`.

### Headless GUI mode

GUI binaries skip window creation when there is no display, when
`BARA_HEADLESS` is set, or when `--headless` is passed — printing a line and
exiting 0 (used by CI and integration tests):

```bash
./target/debug/bara build examples/todo_app/main.br --offline
BARA_HEADLESS=1 ./examples/todo_app/target/debug/todo_app --headless
# Bara GUI 'Bara Todo Desktop': headless mode, skipping window.
```

`bara run` also accepts a compiled binary directly (`bara run
examples/todo_app/target/debug/todo_app -- --headless` executes it without
rebuilding); other subcommands given a binary path resolve to the enclosing
project via the nearest `bara.toml`.

---

## Running the Compiler & Tests

```bash
# Build the Bara CLI compiler
cargo build

# Run all test suites (unit + integration, offline-capable)
cargo test --offline

# One suite at a time
cargo test --offline --test integration_todo_app_test
cargo test --offline --test integration_rest_api_test

# Pre-compilation static debugger & diagnostic linter
./target/debug/bara debug examples/todo_app/main.br

# Check an example Bara project for syntax and semantics
./target/debug/bara check examples/todo_app/main.br

# Transpile Bara files to clean Rust
./target/debug/bara transpile examples/todo_app/main.br -o output.rs
```

---

## Exhaustive Test Coverage (Core Suites: 133 Tests, 100% Passing)

Bara enforces a zero-tolerance failure policy with automated `syn::parse_file` verification ensuring every generated line compiles to valid Rust:

| Test Suite | File | Tests | Coverage Scope |
|---|---|:---:|---|
| **Mathematical Algorithms** | `tests/math_algorithms_tests.rs` | 12 | Fibonacci (iterative/recursive), Factorials, GCD, LCM, Sieve of Eratosthenes, Trial Division Primes, Fast Exponentiation, Matrix 2x2, Collatz, Sqrt Binary Search |
| **Scientific & Numerical** | `tests/scientific_numerical_tests.rs` | 10 | 3D Vectors (dot/cross), Projectile Motion, Mean & Variance, Linear Regression, Trapezoidal Integration, Newton-Raphson Root Finding, Temp Conversions, Horner Poly, Complex Numbers, 3x3 Determinant |
| **Logic & Boolean Algebra** | `tests/logic_boolean_tests.rs` | 10 | De Morgan's Laws, Truth Tables, Bitwise Operations (`&`, `\|`, `^`), Multi-branch If/Elif/Else, Ternary Logic, Flag Masking, Leap Year, Short-Circuit Guards, 3-CNF SAT, Status Routing |
| **Backend & Systems** | `tests/backend_systems_tests.rs` | 10 | HTTP Request/Response, URL Dispatcher, Bearer Token Auth, Status Code Handlers, Token Bucket Rate Limiting, Query Parsing, Middleware Pipeline, KV Cache, Event Logger, Payload Validator |
| **Memory & Data Structures** | `tests/memory_data_structures_tests.rs` | 10 | Stack (LIFO), Queue (FIFO), Circular Ring Buffer, Singly Linked List, BST Search, Hash Bucketing, Min Heap Indexing, Memory Pool Allocator, Unique Deduplication, Dynamic Array |
| **Looping & Control Flow** | `tests/looping_control_flow_tests.rs` | 10 | While Accumulator, For-in Collections, Break, Continue, Nested While Loops, Nested For Loops, Early Return, Countdown, While-True Break, Filter & Double Mapping |
| **Objects & Methods** | `tests/objects_methods_tests.rs` | 10 | Struct Defaults, Immutable Field Access, Mutable Mutations, Nested Composition, Structs with Object Lists, Getters & Setters, Factory Constructors, Computed Fields, Sibling Method Calls, Struct Equality |
| **BRUI & Compile-Time CSS** | `tests/brui_css_tests.rs` | 10 | Hex (3/4/6/8-digit), RGB/RGBA, Margin/Padding Shorthands (1/2/3/4 values, rem/em), Borders & Strokes, CSS Cascading, Typography & RichText Modifiers, Spacing & Box Sizing, Layout Hierarchy, Control Bindings, Conditional/List Rendering |
| **Pre-Compilation Debugger** | `tests/debugger_tests.rs` | 7 | Undeclared Variables, Duplicate Types, Duplicate Fields/Methods, Zero Division, Dead/Unreachable Code, Unknown Brui Components, Rustc-style Diagnostics |
| **Semantic Analyzer** | `tests/analyzer_tests.rs` | 7 | Mutability Inference, Pipeline Lowering, Auto-borrowing, Shadowing, Struct Self-resolution, Heterogeneous List Dynamic Fallback, Index Casts |
| **Parser & Grammar** | `tests/parser_tests.rs` | 6 | Raw Rust, Pipeline, Types & Methods, Variables & Mutations, Functions, String Interpolation |
| **Rust Codegen** | `tests/codegen_tests.rs` | 6 | Raw Rust Inlining, Mutability Generation, Pipeline/Interpolation, Functions & Borrowing, Struct Impls, Heterogeneous Dyn Enums |
| **Cargo Bridge** | `tests/cargo_bridge_tests.rs` | 4 | Config Parsing & Serialization, Error Line Mapping, `.bara_cache` Generation, Project Init & Check |
| **Lexer & Tokens** | `tests/lexer_tests.rs` | 3 | Indent/Dedent, Raw Rust Block Lexing, Operators & Keywords |
| **Full Todo App Template** | `tests/todo_transpile_tests.rs` | 1 | Complete multi-file `.br` + `.brx` GUI app transpilation & syn validation |
| **Todo App Integration** | `tests/integration_todo_app_test.rs` | 11 | TodoApp struct, tambah/hapus/hitung methods, `eframe::App` impl, headless main, `CargoBridge` build check |
| **REST API Integration** | `tests/integration_rest_api_test.rs` | 11 | `HttpRequest`/`HttpResponse`/`ApiServer`, 200/401/404 dispatch routes, `CargoBridge` build check |
| **Examples Integration** | `tests/examples_integration_tests.rs` | 5 | All 7 examples (`cli_algorithms`, `backend_api`, `todo_app`, `dashboard_gui`, `brui_xml_gui`, `multi_lib`, `object_handling`) transpile + `check()` green |
| **TOTAL (core suites + new integration suites)** | **18 Test Suites** | **133** | **100% Passed (Semua Hijau)** |

Full per-suite list lives in `docs/testing.md`.
