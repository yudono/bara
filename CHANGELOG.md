# Changelog

All notable changes to the Bara compiler & CLI are documented here.
Versioning follows SemVer (`bara version` reads `Cargo.toml`).

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
