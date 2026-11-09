use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile(input: &str) -> String {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    let mut program = Parser::new(tokens).parse_program().expect("parser failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("analyzer failed");
    let rust_code = Codegen::new(&analyzer).generate(&program);
    syn::parse_file(&rust_code).expect("generated Rust must be valid (syn)");
    rust_code
}

#[test]
fn async_go_emits_tokio_spawn() {
    let rs = transpile("work() => print(\"w\")\ngo work()\n");
    assert!(rs.contains("tokio::spawn(async move"), "{rs}");
}

#[test]
fn async_fn_with_go_body_is_async() {
    let rs = transpile("run_bg() =>\n    go helper()\n    print(\"spawned\")\nhelper() => print(\"h\")\n");
    assert!(rs.contains("pub async fn run_bg()"), "{rs}");
    assert!(rs.contains("tokio::spawn(async move"), "{rs}");
}

#[test]
fn async_top_level_go_makes_tokio_main() {
    let rs = transpile("bg() => print(\"b\")\ngo bg()\nprint(\"up\")\n");
    assert!(rs.contains("#[tokio::main]"), "{rs}");
    assert!(rs.contains("async fn main()"), "{rs}");
    assert!(rs.contains("tokio::spawn(async move"), "{rs}");
}

#[test]
fn async_try_emits_question_mark_and_result() {
    let rs = transpile(
        "load_name() -> str =>\n    v = fetch_name()!\n    return v\nfetch_name() -> str => \"bob\"\nprint(load_name())\n",
    );
    assert!(rs.contains("fetch_name()?"), "{rs}");
    assert!(rs.contains("pub fn load_name() -> Result<String, Box<dyn std::error::Error>>"), "{rs}");
}

#[test]
fn async_try_in_async_main_emits_await() {
    let rs = transpile(
        "go bg()\nv = inner()!\nprint(v)\nbg() => print(\"booted\")\ninner() -> str => \"data\"\n",
    );
    assert!(rs.contains("#[tokio::main]"), "{rs}");
    assert!(rs.contains("inner().await?"), "{rs}");
}

#[test]
fn async_fn_with_return_wraps_result() {
    let rs = transpile(
        "fetch_num() -> int =>\n    go helper()\n    return 42\nhelper() => print(\"h\")\nprint(fetch_num())\n",
    );
    assert!(rs.contains("pub async fn fetch_num() -> Result<i64, Box<dyn std::error::Error>>"), "{rs}");
}
