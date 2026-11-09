# Changelog

All notable changes to the Bara compiler & CLI are documented here.
Versioning follows SemVer (`bara version` reads `Cargo.toml`).

## [0.1.5] - 2026-11-09

### Added
- Inline React-style `style={{...}}` per-widget overrides (camelCase or
  kebab-case keys, literal values, wins over class styles; `W0007` on
  unknown keys).
- Preflight contract documented: unstyled widgets inherit the active egui
  theme (no forced colors); `color-scheme: light` selects the light base.
- `#[allow(non_snake_case)]` on generated `onMount`/`onUpdate`/`onUnmount`
  so React lifecycle naming never warns.
- 40-test CSS suite incl. preflight guarantees; COMPONENT.md + CSS.md
  component/property matrices.

### Fixed
- `Margin::parse` / `Stroke::parse` are strict: invalid declarations are
  ignored instead of silently becoming zero/black.
- `opacity` modulates stroke color, consistent with fill/text.

## [0.1.4] - 2026-11-09

### Changed / Added
- Injected child instances are now snake_case (`__child_login_form_0`);
  user code always writes the `ChildType.field` / `ChildType.method()`
  sugar — never `__child_*` — and typos still error as undeclared
  identifiers.
- Boot view follows `brui.load("*.brx")` in the entry program.
- `onUpdate` auto-repaint: a viewed type defining `onUpdate` gets
  `request_repaint_after(250ms)` for per-frame logic; types without it stay
  event-driven.
- New example `../Solanum_bara/` (standalone pomodoro timer rewrite:
  Timer/Settings/About components, `LapType` enum + `match`, frame-based
  ticking via `onUpdate`, light theme via `color-scheme: light`, explicit
  `styles.css`).

### Added (components)
- New native tags: `RadioButton`, `Picker`/`Dropdown`/`Select` (with
  `<Option value="..">` children), `Switch`/`Toggle` (emulated),
  `Modal`/`Dialog`, `Spinner`/`ActivityIndicator`, `BottomSheet`,
  `Drawer`/`SideMenu`, `Stack`/`VStack`/`HStack` aliases, `tooltip="..."`
  attribute on Text/Button. See `COMPONENT.md` for the full matrix and
  the documented skips (`ZStack`, video, webview, gestures...).
- New tests: `tests/css_box_model_test.rs`, `tests/css_flexbox_test.rs`,
  `tests/css_typography_test.rs`, `tests/css_background_border_test.rs`,
  `tests/css_effects_test.rs`, `tests/css_layout_position_test.rs`,
  `tests/css_misc_test.rs`, `tests/bara_composition_test.rs`,
  `tests/brui_extra_tags_test.rs`.
  See `CSS.md` for per-property support status and reasons.

### Fixed (CSS correctness)
- Invalid declarations are now ignored instead of silently becoming
  zero/black: strict unit parsing for margin/padding (`Margin::parse`
  returns `None` on garbage; unitless non-zero rejected) and strict
  `Stroke::parse` (needs a real width or color).
- `opacity` now modulates stroke colors too, consistent with fill/text.
- `color-scheme: light` on the root panel class opts a view into
  `egui::Visuals::light()`.
- Single-file React components: `type` + `view` (+ optional inline
  `<style>`) may live together in one `.brx`; all GUI examples merged to
  this shape (`todo.brx`, `dashboard.brx` + `pages/**`, `demo.brx`).
- Inline `style={{...}}` per-widget overrides (React syntax, camelCase or
  kebab-case keys, literal values, `W0007` on unknown keys); wins over
  class styles.
- Preflight contract documented: unstyled widgets inherit the active egui
  theme (no forced dark colors); invalid CSS is ignored, never coerced.

## [0.1.3] - 2026-11-09

### Added (React-style UI model)
- Child components: `<Tag .../>` where `Tag` names a `type` + `view` renders
  inline with props; per-parent persisted instances keep child state.
- Clean component references: write `ChildType.field` / `ChildType.method()`
  in parent code — the analyzer maps them to the embedded instance
  (no `__child_*` identifiers in user code; those stay compiler-internal).
- `useEffect(dep, ...) => body` hooks (top-level only, `E0006` otherwise),
  deferred to end of `update()` with signature-change detection.
- Lifecycle methods `onMount` / `onUpdate` / `onUnmount` (via `Drop`).
- Diagnostics: unknown prop `E0005`, missing backing type `E0004`,
  self-recursion `E0007`.
- Boot order: the app boots the view named by `brui.load("*.brx")` in the
  entry program instead of the alphabetically first view.
- New example `examples/react_tree/` (props, effects, lifecycle, tree).
- Dashboard rewrite: `shell.brx` → `dashboard.brx` sidebar shell, pages are
  real child components, login/register session + full node CRUD, all UI
  functional.

## [0.1.2] - 2026-11-09

### Fixed (macOS GUI abort)
- Root cause found: the `countByEnumeratingWithState` abort comes from objc2's
  `msg_send_check`, which is `#[cfg(debug_assertions)]` — a debug-only
  verification that false-positives on Swift-overlay NSScreen arrays. It cannot
  fire in release builds.
- Generated GUI projects now set `[profile.dev] debug-assertions = false`
  (overflow checks stay on), so plain `bara run` opens a window instead of
  aborting. Verified: the abort string is absent from generated binaries
  (`strings ... | grep "invalid message send"` → 0), and all three GUI examples
  stay alive with real windows.
- Removed eframe's `accesskit` default feature from the generated manifest
  (it dragged in `accesskit_macos -> objc2 0.3-beta`, gone from the lock now).
  Opt back in via `bara.toml`: `eframe = { version = "0.27", features = [...] }`.

## [0.1.1] - 2026-11-09

## [0.1.1] - 2026-11-09

### Fixed
- `bara check/build/transpile/debug/run` now accept a compiled binary path
  (e.g. `bara run examples/todo_app/target/debug/todo_app`) by resolving to
  the enclosing project via the nearest `bara.toml` instead of failing with
  "No Bara source files found".
- `bara run <compiled-binary>` executes the binary directly without rebuilding.
- `bridge.run` prints a friendly hint when the child process dies by an OS
  signal (covers the upstream eframe 0.27 / winit screen-enumeration abort on
  macOS, which is outside Bara-generated code).
- Removed stale `bara_app` binaries left over from the old `[project]` config;
  every `examples/*/target/debug/` now holds exactly one correctly named binary.
- Core is fully dynamic: `bara init` scaffolds a blank hello-world project
  (`print("Hello from Bara!")`) with no hardcoded TodoApp/demo content.
- Stylesheets are explicit-only: a `.css` file is compiled in only when some
  `.br`/`.brx` file contains `use "styles.css"` (also `use styles.css` /
  `use styles`); never auto-loaded.

### Added
- Brui XML layouts (`layout.xml` + `parse_brui_xml`) as an alternative to `.brx`.
- 16 supported Brui components: Panel, Col, Row, Scroll, Text, Button, Input,
  TextField, Checkbox, Slider, Separator, Wrap, Container, Image, Progress, Spacer.
- Extended CSS engine: opacity, margin/outer_margin, width/height/max limits,
  align/justify helpers.
- Multi-page `dashboard_gui` example (`pages/auth/*`, `pages/dashboard/*`)
  sharing one explicitly imported `styles.css`.
- New examples: `brui_xml_gui`, `multi_lib` (imports/exports, enums, generics),
  `object_handling` (methods, ownership).
- Regression test `test_cli_accepts_compiled_binary_path` plus
  `sync_css_requires_explicit_import`.
- Headless GUI mode (`BARA_HEADLESS=1` / `--headless`) for CI and windowless runs.

## [0.1.0] - 2026-11-09
- Initial release: lexer, parser, analyzer, CSS engine, codegen, cargo bridge,
  CLI toolchain, and the first test suites.
