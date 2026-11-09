# Examples

All examples build offline and run to exit code 0. GUI examples render with
`eframe`/`egui` and support `--headless` (see below).

```
examples/
  todo_app/          # Todo list: types + methods (.br) + native GUI (.brx)
  dashboard_gui/     # Sidebar cloud dashboard: dashboard.brx shell + pages/** components
    pages/
      auth/          # login, register (.br + .brx each, auth session state)
      dashboard/     # overview (metrics/logs/search), users (node CRUD) (.br + .brx each)
  brui_xml_gui/      # XML layout + shared CSS variant (layout.xml + styles.css)
  ../Solanum_bara/     # Pomodoro timer rewrite (standalone project): Timer/Settings/About + LapType enum + onUpdate ticking
  cli_algorithms/    # Fibonacci, primes, sorting, binary search, GCD
  backend_api/       # HttpRequest/Response + ApiServer dispatch demo
  multi_lib/         # Multi-module imports (libs/*.br), enums, Option/Result
  object_handling/   # Structs, &self/&mut self methods, nested composition
```

## GUI examples

| Example | Entry | UI definition | Notes |
|---|---|---|---|
| `todo_app` | `main.br` | `todo.brx` + inline `<style>` | `TodoApp`: `tambah_tugas`, `hapus_tugas`, `hitung_aktif`; `eframe::App` impl |
| `dashboard_gui` | `main.br` | `dashboard.brx` sidebar shell + `pages/**` child components + shared `styles.css` | `DashboardApp` boots first (matches `brui.load("dashboard.brx")`); sidebar after login, auth-gated pages; login/register with session, node CRUD (add/edit/remove/filter), KPI + log search |
| `brui_xml_gui` | `main.br` | `layout.xml` + shared `styles.css` | `XmlDemoApp` (`increment`, `reset`, `login`); XML `bind="..."` / `onClick="..."` string form |
| `react_tree` | `main.br` | `demo.brx` + `counter.brx` + shared `styles.css` | `ReactDemoApp` parent + `Clicker` children; props, `useEffect`, `onMount`/`onUpdate`/`onUnmount`, persisted child state |
| `Solanum_bara` | `main.br` | `Timer`/`Settings`/`About` child components + shared `styles.css` | Pomodoro timer rewrite: `LapType` enum + `match`, frame-based ticking via `onUpdate` (250ms repaint), explicit stylesheet import |

Window sizes: todo `450x600`, dashboard `850x620`, brui XML demo `500x400`
(set via `app.run("Title", width = ..., height = ...)` in each `main.br`).

## CLI examples

| Example | Entry | What it does |
|---|---|---|
| `cli_algorithms` | `main.br` + `algorithms.br` | Fibonacci(10/20), sieve primes to 30, bubble sort, binary search, GCD(48, 18) |
| `backend_api` | `main.br` + `server.br` | `GET /health` → 200, missing token → 401, `GET /api/users` → 200, `POST /api/users` → 201, unknown → 404 |
| `multi_lib` | `main.br` + `libs/*.br` | Cross-module functions/types, `TextStatus`/`Shape` enums with `match`, `Option`/`Result` handling |
| `object_handling` | `main.br` + `models.br` | `BankAccount` deposit/withdraw, `Counter`, nested `Pair`, list filter/iterate |

## Shared `styles.css` concept (explicit import)

`.css` files are explicit-only: a project only gets a stylesheet when some
`.br`/`.brx` file contains `use "styles.css"` (also accepts `use styles.css`
or shorthand `use styles`). `dashboard_gui/main.br`,
`brui_xml_gui/main.br`, and `Solanum_bara/main.br` therefore import their shared `styles.css`, which then
styles every `.brx` page and XML layout in the project; individual `.brx`
files need no `<style>` block. Inline `<style>` (as in `todo.brx`) merges
into the same sheet. Without the `use` line the `.css` file is collected but
ignored (see `sync_css_requires_explicit_import`). See
`docs/brui_styling.md` for the supported property list.

## `bara.toml` layout

Every example uses the same three-table shape:

```toml
[package]
name = "todo_app"
version = "0.1.0"
entry = "main.br"

[presets]
gui = true

[dependencies]
```

`[presets]` selects toolchain behavior (`gui` pulls `eframe`/`egui`,
`serde`/`async` pull their crates into the generated `Cargo.toml`).

## `--headless` usage

GUI binaries detect headless environments automatically (no `DISPLAY` /
`WAYLAND_DISPLAY` on Linux) and also honor an explicit flag or env var:

```bash
./target/debug/bara build examples/todo_app/main.br --offline
BARA_HEADLESS=1 ./examples/todo_app/target/debug/todo_app --headless
# Bara GUI 'Bara Todo Desktop': headless mode, skipping window.
```

In headless mode the app prints the line above and exits 0 without opening a
window — used by CI and by the integration tests.

```bash
# Check, transpile, build, run (all offline-capable)
./target/debug/bara check examples/todo_app/main.br
./target/debug/bara transpile examples/todo_app/main.br -o /tmp/todo.rs
./target/debug/bara build examples/dashboard_gui/main.br --offline
./target/debug/bara run examples/cli_algorithms/main.br --offline
```
