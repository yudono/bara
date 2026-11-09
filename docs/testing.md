# Testing

```bash
cargo build --offline        # build the bara compiler
cargo test --offline         # full suite (unit + integration)
cargo test --offline --test <suite>   # one suite, e.g. integration_todo_app_test
```

Every test transpiles Bara source and asserts `syn::parse_file` succeeds, so
generated Rust is always validated as 100% syntactically valid.

## Suite list

| Suite | Tests | Scope |
|---|---|---|
| `math_algorithms_tests` | 12 | Fibonacci, factorial, GCD, sieve, fast pow, matrix, collatz, sqrt |
| `lexer_number_literal_test` | 10 | Numeric literal lexing |
| `backend_systems_tests` | 10 | HTTP types, routing, auth, rate limiting, cache, events, validation |
| `brui_css_tests` | 10 | Hex/RGB colors, margin/padding shorthands, borders, cascading, bindings |
| `objects_methods_tests` | 10 | Struct defaults, mutation, nesting, getters/setters, sibling calls |
| `memory_data_structures_tests` | 10 | Stack, queue, ring buffer, linked list, BST, heap, pool, dedup |
| `logic_boolean_tests` | 10 | De Morgan, truth tables, bit ops, if/elif/else, ternary, SAT |
| `looping_control_flow_tests` | 10 | While/for, break/continue, nesting, early return, filter/map |
| `scientific_numerical_tests` | 10 | Vectors, projectile, stats, regression, integration, Newton, complex |
| `lexer_comment_handling_test` | 9 | Comment lexing |
| `lexer_jsx_tag_test` | 9 | JSX tag lexing |
| `lexer_operator_precedence_test` | 9 | Operator lexing |
| `lexer_string_interpolation_test` | 9 | Interpolation lexing |
| `parser_expression_statement_test` | 15 | Expression/statement parsing |
| `rust_parity_tests` | 15 | Enum/match/Option/Result/tuple parity with Rust |
| `parser_control_flow_test` | 12 | Control-flow parsing |
| `parser_function_declaration_test` | 12 | Function declaration parsing |
| `integration_todo_app_test` | 11 | TodoApp struct, tambah/hapus/hitung, `eframe::App`, headless main, `CargoBridge` check |
| `integration_rest_api_test` | 11 | HttpRequest/Response/ApiServer, 200/401/404 dispatch, `CargoBridge` build |
| `parser_jsx_nesting_test` | 11 | JSX nesting parsing |
| `parser_struct_definition_test` | 10 | Struct definition parsing |
| `parser_pipeline_operator_test` | 9 | Pipeline operator parsing |
| `analyzer_tests` | 7 | Mutability, pipelines, borrowing, shadowing, dyn fallback |
| `analyzer_auto_borrowing_test` | 7 | Auto-borrowing analysis |
| `analyzer_lifetime_elision_test` | 7 | Lifetime elision |
| `analyzer_mutability_inference_test` | 7 | Mutability inference |
| `debugger_tests` | 7 | Undeclared vars, duplicates, div-by-zero, dead code, unknown components |
| `analyzer_last_use_clone_test` | 6 | Last-use clone analysis |
| `analyzer_scope_resolution_test` | 6 | Scope resolution |
| `analyzer_type_shadowing_test` | 6 | Type shadowing |
| `codegen_tests` | 6 | Raw Rust, mutability, pipelines, structs, dyn enums |
| `parser_tests` | 6 | Raw Rust, pipeline, types, functions, interpolation |
| `examples_integration_tests` | 5 | All 7 examples transpile + `check()` green |
| `cargo_bridge_tests` | 4 | Config parse, init/check, cache generation, error mapping |
| `lexer_tests` | 3 | Indent/dedent, raw Rust blocks, operators |
| `todo_transpile_tests` | 1 | Full multi-file todo `.br` + `.brx` transpile |

## The two integration suites

`tests/integration_todo_app_test.rs` (11 tests) transpiles
`examples/todo_app` through `CargoBridge` and asserts the output contains
`TodoApp`/`TodoItem` structs, `tambah_tugas(&mut self)`,
`hapus_tugas(&mut self, id: i64)`, `hitung_aktif(&self) -> i64`,
`impl eframe::App for TodoApp`, the headless `main`
(`BARA_HEADLESS`, `--headless`, `eframe::run_native`, window title), GUI
bindings (`TextEdit::singleline`, `.clicked()`, `ScrollArea`), plus
`syn` validity and a `CargoBridge::check` + cache-generation build check.

`tests/integration_rest_api_test.rs` (11 tests) transpiles
`examples/backend_api` and asserts `HttpRequest`/`HttpResponse`/`ApiServer`,
the `dispatch` signature, route coverage (`/health` 200, `/api/users` 200,
missing-token 401, unknown-path 404, create 201), `main` dispatch calls, `syn`
validity, `check()`, and a real `CargoBridge::build(..., offline=true)` whose
binary must exist.

## Conventions for new tests

- Transpile via `Lexer` → `Parser` → `Analyzer` → `Codegen`, or via
  `CargoBridge::new(example_dir())` for example-level tests.
- Always assert `syn::parse_file(&rust_code).is_ok()`.
- Assert on stable generated markers (`pub struct X`, `pub fn f(...)`,
  `impl eframe::App`, status codes) rather than exact formatting.
- GUI tests assert headless markers; never open a window in tests.
- CLI `build` tests pass `offline=true` so CI needs no network.
