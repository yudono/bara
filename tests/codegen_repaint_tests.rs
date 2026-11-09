use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile(input: &str) -> String {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("lexer failed");
    let mut parser = Parser::new(tokens);
    let mut prog = parser.parse_program().expect("parser failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut prog).expect("analyzer failed");
    let mut codegen = Codegen::new(&analyzer);
    codegen.generate(&prog)
}

const WITH_UPDATE_SRC: &str = r#"
type Clock =
    ticks: int = 0

    onUpdate() =>
        ticks = ticks + 1

view Clock =>
    <Panel class="s">
        <Text class="t">{ticks}</Text>
    </Panel>
"#;

const WITHOUT_UPDATE_SRC: &str = r#"
type Plain =
    title: str = "hi"

view Plain =>
    <Panel class="s">
        <Text class="t">{title}</Text>
    </Panel>
"#;

const CHILD_UPDATE_SRC: &str = r#"
type Card =
    count: int = 0

    onUpdate() =>
        count = count + 1

view Card =>
    <Panel class="c">
        <Text class="t">{count}</Text>
    </Panel>

type App =
    title = "T"

view App =>
    <Panel class="s">
        <Card />
    </Panel>
"#;

fn update_fn_body(rust: &str) -> &str {
    let start = rust.find("fn update(").expect("no update fn");
    &rust[start..]
}

fn child_fn_body_for_impl<'a>(rust: &'a str, impl_type: &str) -> &'a str {
    let marker = format!("impl {impl_type}");
    let impl_start = rust
        .find(marker.as_str())
        .unwrap_or_else(|| panic!("no {marker}"));
    let tail = &rust[impl_start..];
    let child_start = tail
        .find("__render_as_child")
        .expect("no __render_as_child");
    &tail[child_start..]
}

#[test]
fn view_with_onupdate_requests_repaint_in_update() {
    let rs = transpile(WITH_UPDATE_SRC);
    let body = update_fn_body(&rs);
    assert!(
        body.contains("ctx.request_repaint_after(std::time::Duration::from_millis(250));"),
        "update() must request repaint:\n{rs}"
    );
}

#[test]
fn view_without_onupdate_requests_no_repaint() {
    let rs = transpile(WITHOUT_UPDATE_SRC);
    assert!(
        !rs.contains("request_repaint_after"),
        "no repaint expected without onUpdate:\n{rs}"
    );
}

#[test]
fn child_with_onupdate_requests_repaint_in_render_as_child() {
    let rs = transpile(CHILD_UPDATE_SRC);
    let body = child_fn_body_for_impl(&rs, "Card");
    assert!(
        body.contains("ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));"),
        "Card::__render_as_child must request repaint:\n{rs}"
    );
}

#[test]
fn repaint_output_with_update_is_valid_rust() {
    let rs = transpile(WITH_UPDATE_SRC);
    syn::parse_file(&rs).expect("syn failed for onUpdate case");
}

#[test]
fn repaint_output_without_update_is_valid_rust() {
    let rs = transpile(WITHOUT_UPDATE_SRC);
    syn::parse_file(&rs).expect("syn failed for plain case");
    assert!(
        !rs.contains("request_repaint_after"),
        "no repaint expected without onUpdate:\n{rs}"
    );
}
