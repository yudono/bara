use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile_and_verify(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().expect("Lexing failed");
    let mut program = Parser::new(tokens).parse_program().expect("Parsing failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("Analysis failed");
    let mut codegen = Codegen::new(&analyzer);
    let rust_code = codegen.generate(&program);
    let parsed_syn = syn::parse_file(&rust_code);
    assert!(
        parsed_syn.is_ok(),
        "Generated Rust code failed syn validation:\n{rust_code}\nError: {:?}",
        parsed_syn.err()
    );
    rust_code
}

#[test]
fn composition_safe_area_view_panel_padding() {
    let code = r#"
type SafeScreen =
    msg: str = "hello"

view SafeScreen =>
    <Panel class="safe">
        <Text>{msg}</Text>
    </Panel>

<style>
.safe {
    background: #0f172a;
    padding: 24px;
}
</style>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::CentralPanel::default()"), "{rust}");
    assert!(rust.contains("inner_margin"), "{rust}");
    assert!(rust.contains("egui::Margin::same(24.0)"), "{rust}");
}

#[test]
fn composition_flat_list_scroll_for_with_delete() {
    let code = r#"
type FeedItem =
    id: int
    title: str

type Feed =
    items = [
        FeedItem(id = 1, title = "one"),
        FeedItem(id = 2, title = "two")
    ]

    remove(id: int) =>
        items.retain(x => x.id != id)

view Feed =>
    <Panel>
        <Scroll>
            <Col>
                {for item in items =>
                    <Row>
                        <Text>{item.title}</Text>
                        <Button onClick={remove(item.id)}>X</Button>
                    </Row>
                }
            </Col>
        </Scroll>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::ScrollArea::vertical()"), "{rust}");
    assert!(rust.contains("__actions"), "{rust}");
    assert!(rust.contains("retain"), "{rust}");
}

#[test]
fn composition_section_list_two_groups() {
    let code = r#"
type Sections =
    fruits: [str] = ["Apple", "Banana"]
    veggies: [str] = ["Carrot", "Peas"]

view Sections =>
    <Panel>
        <Scroll>
            <Col>
                <Text class="h">Fruits</Text>
                {for f in fruits =>
                    <Text>{f}</Text>
                }
                <Text class="h">Veggies</Text>
                {for v in veggies =>
                    <Text>{v}</Text>
                }
            </Col>
        </Scroll>
    </Panel>

<style>
.h {
    font-size: 18px;
    font-weight: bold;
    color: #f8fafc;
}
</style>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("Fruits"), "{rust}");
    assert!(rust.contains("Veggies"), "{rust}");
    assert!(rust.contains("egui::ScrollArea::vertical()"), "{rust}");
}

#[test]
fn composition_card_col_frame() {
    let code = r#"
type CardDemo =
    title: str = "Card title"
    body: str = "Card body"

view CardDemo =>
    <Panel>
        <Col class="card">
            <Text class="t">{title}</Text>
            <Text>{body}</Text>
        </Col>
    </Panel>

<style>
.card {
    background: #111827;
    border: 1px solid #1f2937;
    padding: 12px;
    border-radius: 10px;
}
.t {
    color: #f8fafc;
    font-size: 16px;
    font-weight: bold;
}
</style>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::Frame::none()"), "{rust}");
    assert!(rust.contains("fill(egui::Color32::from_rgb(17, 24, 39))"), "{rust}");
    assert!(rust.contains("rounding(egui::Rounding::same(10.0))"), "{rust}");
}

#[test]
fn composition_header_appbar_space_between() {
    let code = r#"
type TopBar =
    title: str = "MyApp"
    clicks: int = 0

view TopBar =>
    <Panel>
        <Row class="bar">
            <Text class="title">{title}</Text>
            <Button onClick={clicks = clicks + 1}>Go</Button>
        </Row>
    </Panel>

<style>
.bar {
    justify-content: space-between;
    align-items: center;
}
.title {
    font-size: 20px;
    color: #f8fafc;
    font-weight: bold;
}
</style>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("with_main_justify(true)"), "{rust}");
    assert!(rust.contains("left_to_right"), "{rust}");
}

#[test]
fn composition_bottom_tab_bar_switching() {
    let code = r#"
type Tabs =
    tab: str = "home"

view Tabs =>
    <Panel>
        <Col>
            <Text>{tab}</Text>
            <Row class="bar">
                <Button onClick={tab = "home"}>Home</Button>
                <Button onClick={tab = "search"}>Search</Button>
                <Button onClick={tab = "profile"}>Profile</Button>
            </Row>
        </Col>
    </Panel>

<style>
.bar {
    gap: 8px;
}
</style>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("self.tab ="), "{rust}");
    assert!(rust.contains(".clicked()"), "{rust}");
}

#[test]
fn composition_badge_text_count() {
    let code = r#"
type Inbox =
    count: int = 5

view Inbox =>
    <Panel>
        <Row class="badge_wrap">
            <Text class="badge_text">{count}</Text>
        </Row>
    </Panel>

<style>
.badge_wrap {
    background: #ef4444;
    padding: 4px 8px;
    border-radius: 6px;
}
.badge_text {
    color: #ffffff;
    font-size: 13px;
    font-weight: bold;
}
</style>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("RichText"), "{rust}");
    assert!(rust.contains("fill(egui::Color32::from_rgb(239, 68, 68))"), "{rust}");
    assert!(rust.contains(".label("), "{rust}");
}

#[test]
fn composition_icon_text_glyph() {
    // Vector icons have no native tag at this commit; the Bara-idiomatic
    // composition is a Text glyph/emoji. Documented in COMPONENT.md.
    let code = r#"
type IconDemo =
    label: str = "home"

view IconDemo =>
    <Panel>
        <Row>
            <Text class="ic">★</Text>
            <Text>{label}</Text>
        </Row>
    </Panel>

<style>
.ic {
    font-size: 20px;
    color: #f8fafc;
}
</style>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains(".label("), "{rust}");
    assert!(rust.contains("RichText"), "{rust}");
}

#[test]
fn composition_basics_view_text_image_smoke() {
    let code = r#"
type Basics =
    msg: str = "hi"

view Basics =>
    <Panel>
        <Col>
            <Text>{msg}</Text>
            <Image src="logo.png" />
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::CentralPanel"), "{rust}");
    assert!(rust.contains(".label("), "{rust}");
}
