use bara::analyzer::Analyzer;
use bara::ast::{Item, JsxChild};
use bara::cargo_bridge::CargoBridge;
use bara::codegen::Codegen;
use bara::debugger::BaraDebugger;
use bara::lexer::Lexer;
use bara::parser::Parser;
use std::path::PathBuf;

fn parse_ok(src: &str) -> bara::ast::Program {
    let tokens = Lexer::new(src).tokenize().expect("lex");
    Parser::new(tokens).parse_program().expect("parse")
}

fn transpile_ok(src: &str) -> String {
    let mut prog = parse_ok(src);
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut prog).expect("analyze");
    let mut codegen = Codegen::new(&analyzer);
    let rust = codegen.generate(&prog);
    syn::parse_file(&rust).expect("syn");
    rust
}

fn diags_for(src: &str) -> Vec<bara::debugger::Diagnostic> {
    let prog = parse_ok(src);
    let mut dbg = BaraDebugger::new();
    dbg.inspect(&prog, None);
    dbg.diagnostics
}

fn has_code(diags: &[bara::debugger::Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code == code)
}

const PARENT_CHILD_SRC: &str = r#"
type Card =
    label: str = "Hi"
    count: int = 0
    ready = false

    bump() =>
        count = count + 1

    sync() =>
        ready = true

    onMount() =>
        ready = true

view Card =>
    <Panel class="card">
        <Text class="t">{label} {count}</Text>
        <Button class="b" onClick={bump()}>
            Go
        </Button>
        {useEffect(count) => sync()}
    </Panel>

type App =
    title = "T"
    clicks: int = 0

    poke() =>
        clicks = clicks + 1

    onUpdate() =>
        clicks = clicks + 1

    onUnmount() =>
        clicks = 0

view App =>
    <Panel class="screen">
        <Text class="t">{title}</Text>
        <Card label="Hello" />
        {useEffect(clicks) => poke()}
    </Panel>
"#;

#[test]
fn react_parses_use_effect_hook() {
    let prog = parse_ok(PARENT_CHILD_SRC);
    let mut found = 0;
    for item in &prog.items {
        if let Item::View(v) = item {
            for child in &v.root.children {
                if let JsxChild::Element(e) = child {
                    for sub in &e.children {
                        if let JsxChild::Effect { deps, .. } = sub {
                            found += deps.len();
                        }
                    }
                }
            }
            for child in &v.root.children {
                if let JsxChild::Effect { deps, .. } = child {
                    found += deps.len();
                }
            }
        }
    }
    // App effect has 1 dep (clicks); Card effect has 1 dep (count).
    assert_eq!(found, 2, "expected two useEffect deps total");
}

#[test]
fn react_valid_tree_has_no_component_errors() {
    let diags = diags_for(PARENT_CHILD_SRC);
    for bad in ["E0004", "E0005", "E0006", "E0007"] {
        assert!(!has_code(&diags, bad), "unexpected {bad}: {diags:?}");
    }
}

#[test]
fn react_nested_effect_is_rejected() {
    let src = r#"
type App =
    items: [int] = []

view App =>
    <Panel class="s">
        {for x in items =>
            {useEffect(x) => poke()}
        }
    </Panel>
"#;
    let diags = diags_for(src);
    assert!(has_code(&diags, "E0006"), "expected E0006, got {diags:?}");
}

#[test]
fn react_unknown_prop_is_rejected() {
    let src = r#"
type Card =
    label: str = "x"

view Card =>
    <Panel class="c">
        <Text class="t">{label}</Text>
    </Panel>

type App =
    dummy = 1

view App =>
    <Panel class="s">
        <Card nope="x" />
    </Panel>
"#;
    let diags = diags_for(src);
    assert!(has_code(&diags, "E0005"), "expected E0005, got {diags:?}");
}

#[test]
fn react_self_recursion_is_rejected() {
    let src = r#"
type Loop =
    n: int = 0

view Loop =>
    <Panel class="s">
        <Loop />
    </Panel>
"#;
    let diags = diags_for(src);
    assert!(has_code(&diags, "E0007"), "expected E0007, got {diags:?}");
}

#[test]
fn react_component_without_backing_type_is_rejected() {
    let src = r#"
type App =
    dummy = 1

view Ghost =>
    <Panel class="g">
        <Text class="t">boo</Text>
    </Panel>

view App =>
    <Panel class="s">
        <Ghost />
    </Panel>
"#;
    let diags = diags_for(src);
    assert!(has_code(&diags, "E0004"), "expected E0004, got {diags:?}");
}

#[test]
fn react_codegen_renders_child_with_state_and_effects() {
    let rust = transpile_ok(PARENT_CHILD_SRC);
    // Persisted child instance + props on the parent.
    assert!(rust.contains("__child_Card_0"), "{rust}");
    assert!(
        rust.contains("self.__child_Card_0.label = \"Hello\".to_string();"),
        "{rust}"
    );
    assert!(rust.contains("__render_as_child"), "{rust}");
    // Effect signature + mount flag plumbing.
    assert!(rust.contains("__br_mounted"), "{rust}");
    assert!(rust.contains("__fx_sig_0"), "{rust}");
    // Lifecycle wiring.
    assert!(rust.contains("fn onMount"), "{rust}");
    assert!(rust.contains("fn onUpdate"), "{rust}");
    assert!(rust.contains("impl Drop for App"), "{rust}");
    // Hidden fields never leak into constructors (assignment form keeps
    // Drop types working).
    assert!(rust.contains("let mut __s = Card::default();"), "{rust}");
}

#[test]
fn react_child_type_dot_syntax_resolves_to_instance() {
    // User-facing sugar: `Card.count` inside the parent means
    // `self.__child_Card_0.count` — no `__child_` identifiers in user code.
    let src = r#"
type Card =
    label: str = "x"
    count: int = 0

    bump() =>
        count = count + 1

view Card =>
    <Panel class="c">
        <Text class="t">{label} {count}</Text>
    </Panel>

type App =
    title = "T"

    reset_card() =>
        Card.count = 0

view App =>
    <Panel class="s">
        <Card label="hi" />
        {if Card.count == 0 =>
            <Text class="t">fresh</Text>
        }
    </Panel>
"#;
    let rust = transpile_ok(src);
    assert!(
        rust.contains("self.__child_Card_0.count"),
        "sugar must lower to instance access:\n{rust}"
    );
    assert!(
        !rust.contains("Card.count"),
        "no raw type-level access may remain:\n{rust}"
    );
}

#[test]
fn react_example_transpiles_and_boots_parent() {    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/react_tree");
    if !dir.is_dir() {
        eprintln!("Skipping: react_tree example missing");
        return;
    }
    let bridge = CargoBridge::new(dir);
    let (sources, _) = bridge.collect_sources(None).expect("collect");
    let rust = bridge.transpile_sources(&sources).expect("transpile");
    syn::parse_file(&rust).expect("syn");
    assert!(rust.contains("Box::new(ReactDemoApp::default())"), "{rust}");
    assert!(rust.contains("__child_Clicker_0"), "{rust}");
    assert!(rust.contains("__child_Clicker_1"), "{rust}");
}
