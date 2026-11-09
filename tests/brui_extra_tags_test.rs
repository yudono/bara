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
    // Collapse all whitespace runs: prettyplease reflows long lines, so
    // assertions use normalized code and stay formatting-robust.
    let mut flat = rust_code.split_whitespace().collect::<Vec<_>>().join(" ");
    for (a, b) in [
        (" .", "."),
        ("( ", "("),
        (" )", ")"),
        (" ,", ","),
        (" ;", ";"),
        (",)", ")"),
    ] {
        flat = flat.replace(a, b);
    }
    flat
}

// ---- RadioButton (egui::Ui::radio_value, ui.rs) ----

#[test]
fn test_radio_button_str_field() {
    let code = r#"
type Choice =
    choice: str = "a"

view Choice =>
    <Panel>
        <Col>
            <RadioButton bind={choice} value="a">Option A</RadioButton>
            <RadioButton bind={choice} value="b">Option B</RadioButton>
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.radio_value(&mut self.choice, \"a\".to_string(), \"Option A\");"), "missing radio A:\n{rust}");
    assert!(rust.contains("ui.radio_value(&mut self.choice, \"b\".to_string(), \"Option B\");"), "missing radio B:\n{rust}");
}

#[test]
fn test_radio_button_expr_value_int_field() {
    let code = r#"
type Pick =
    num: int = 1

view Pick =>
    <Panel>
        <Col>
            <RadioButton bind={num} value={1}>One</RadioButton>
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.radio_value(&mut self.num, 1, \"One\");"), "missing int radio:\n{rust}");
}

// ---- Picker / Dropdown / Select (egui::ComboBox::from_label + selected_text + show_ui + Ui::selectable_value) ----

#[test]
fn test_picker_with_options() {
    let code = r#"
type Fruit =
    fruit: str = "apple"

view Fruit =>
    <Panel>
        <Col>
            <Picker bind={fruit}>
                <Option value="apple">Apple</Option>
                <Option value="banana">Banana</Option>
            </Picker>
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::ComboBox::from_label(\"\").selected_text(self.fruit.clone()).show_ui(ui, |ui| {"), "missing combo:\n{rust}");
    assert!(rust.contains("ui.selectable_value(&mut self.fruit, \"apple\".to_string(), \"Apple\");"), "missing option apple:\n{rust}");
    assert!(rust.contains("ui.selectable_value(&mut self.fruit, \"banana\".to_string(), \"Banana\");"), "missing option banana:\n{rust}");
}

#[test]
fn test_dropdown_alias() {
    let code = r#"
type Fruit =
    fruit: str = "apple"

view Fruit =>
    <Panel>
        <Dropdown bind={fruit}>
            <Option value="apple">Apple</Option>
        </Dropdown>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::ComboBox::from_label(\"\").selected_text(self.fruit.clone())"), "missing combo:\n{rust}");
    assert!(rust.contains("ui.selectable_value(&mut self.fruit, \"apple\".to_string(), \"Apple\");"), "missing option:\n{rust}");
}

#[test]
fn test_select_alias() {
    let code = r#"
type Fruit =
    fruit: str = "apple"

view Fruit =>
    <Panel>
        <Select bind={fruit}>
            <Option value="apple">Apple</Option>
        </Select>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::ComboBox::from_label(\"\").selected_text(self.fruit.clone())"), "missing combo:\n{rust}");
    assert!(rust.contains("ui.selectable_value(&mut self.fruit, \"apple\".to_string(), \"Apple\");"), "missing option:\n{rust}");
}

// ---- Switch / Toggle (emulated Button flipping a bool; no native switch widget in egui 0.27) ----

#[test]
fn test_switch_toggles_bool() {
    let code = r#"
type Flags =
    enabled: bool = false

view Flags =>
    <Panel>
        <Col>
            <Switch bind={enabled}>Notifications</Switch>
        </Col>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::Button::new(format!(\"{} [{}]\", \"Notifications\", (if self.enabled { \"On\" } else { \"Off\" })))"), "missing switch button:\n{rust}");
    assert!(rust.contains("self.enabled = !self.enabled;"), "missing toggle:\n{rust}");
}

#[test]
fn test_switch_no_label() {
    let code = r#"
type Flags =
    enabled: bool = false

view Flags =>
    <Panel>
        <Switch bind={enabled} />
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::Button::new((if self.enabled { \"On\" } else { \"Off\" }))"), "missing bare switch:\n{rust}");
    assert!(rust.contains("self.enabled = !self.enabled;"), "missing toggle:\n{rust}");
}

#[test]
fn test_toggle_alias() {
    let code = r#"
type Flags =
    dark: bool = true

view Flags =>
    <Panel>
        <Toggle bind={dark}>Dark mode</Toggle>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::Button::new(format!(\"{} [{}]\", \"Dark mode\", (if self.dark { \"On\" } else { \"Off\" })))"), "missing toggle button:\n{rust}");
    assert!(rust.contains("self.dark = !self.dark;"), "missing toggle:\n{rust}");
}

// ---- Modal / Dialog (egui::Window::new + open + show; no egui::Modal in 0.27) ----

#[test]
fn test_modal_with_open_flag() {
    let code = r#"
type App =
    show_help: bool = false

view App =>
    <Panel>
        <Modal title="Help" open={show_help}>
            <Text>Body text</Text>
        </Modal>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::Window::new(\"Help\").open(&mut self.show_help).show(&__bara_ctx, |ui| {"), "missing modal window:\n{rust}");
    assert!(rust.contains("ui.label(egui::RichText::new(\"Body text\"));"), "missing modal child:\n{rust}");
}

#[test]
fn test_modal_without_open_flag() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <Modal title="Always">
            <Text>Hi</Text>
        </Modal>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::Window::new(\"Always\").show(&__bara_ctx, |ui| {"), "missing always-open modal:\n{rust}");
}

#[test]
fn test_dialog_alias() {
    let code = r#"
type App =
    show_about: bool = true

view App =>
    <Panel>
        <Dialog title="About" open={show_about}>
            <Text>About body</Text>
        </Dialog>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::Window::new(\"About\").open(&mut self.show_about).show(&__bara_ctx, |ui| {"), "missing dialog window:\n{rust}");
}

// ---- Spinner / ActivityIndicator (egui::Spinner::new) ----

#[test]
fn test_spinner_default() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <Spinner />
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.add(egui::Spinner::new());"), "missing spinner:\n{rust}");
}

#[test]
fn test_spinner_with_size() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <Spinner size="24" />
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.add(egui::Spinner::new().size(24.0));"), "missing sized spinner:\n{rust}");
}

#[test]
fn test_activity_indicator_alias() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <ActivityIndicator />
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.add(egui::Spinner::new());"), "missing activity indicator:\n{rust}");
}

// ---- BottomSheet (egui::TopBottomPanel::bottom) ----

#[test]
fn test_bottom_sheet() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <BottomSheet>
            <Text>Sheet content</Text>
        </BottomSheet>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::TopBottomPanel::bottom(\"bara_bottom_sheet\").show(&__bara_ctx, |ui| {"), "missing bottom sheet:\n{rust}");
    assert!(rust.contains("ui.label(egui::RichText::new(\"Sheet content\"));"), "missing sheet child:\n{rust}");
}

#[test]
fn test_bottom_sheet_multiple_children() {
    let code = r#"
type App =
    count: int = 0

view App =>
    <Panel>
        <BottomSheet>
            <Text>First</Text>
            <Text>Second</Text>
        </BottomSheet>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::TopBottomPanel::bottom(\"bara_bottom_sheet\")"), "missing bottom sheet:\n{rust}");
    assert!(rust.contains("ui.label(egui::RichText::new(\"First\"));"), "missing first child:\n{rust}");
    assert!(rust.contains("ui.label(egui::RichText::new(\"Second\"));"), "missing second child:\n{rust}");
}

// ---- Drawer / SideMenu (egui::SidePanel::left) ----

#[test]
fn test_drawer() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <Drawer>
            <Text>Nav item</Text>
        </Drawer>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::SidePanel::left(\"bara_drawer_left\").show(&__bara_ctx, |ui| {"), "missing drawer:\n{rust}");
    assert!(rust.contains("ui.label(egui::RichText::new(\"Nav item\"));"), "missing drawer child:\n{rust}");
}

#[test]
fn test_side_menu_alias() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <SideMenu>
            <Text>Menu item</Text>
        </SideMenu>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("egui::SidePanel::left(\"bara_drawer_left\").show(&__bara_ctx, |ui| {"), "missing side menu:\n{rust}");
    assert!(rust.contains("ui.label(egui::RichText::new(\"Menu item\"));"), "missing menu child:\n{rust}");
}

// ---- Stack / VStack / HStack (aliases of Col / Row) ----

#[test]
fn test_stack_is_vertical() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <Stack>
            <Text>A</Text>
        </Stack>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.vertical(|ui| {"), "stack should be vertical:\n{rust}");
    assert!(rust.contains("ui.label(egui::RichText::new(\"A\"));"), "missing stack child:\n{rust}");
}

#[test]
fn test_vstack_is_vertical() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <VStack>
            <Text>B</Text>
        </VStack>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.vertical(|ui| {"), "vstack should be vertical:\n{rust}");
}

#[test]
fn test_hstack_is_horizontal() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <HStack>
            <Text>C</Text>
        </HStack>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("ui.horizontal(|ui| {"), "hstack should be horizontal:\n{rust}");
}

// ---- tooltip attribute (Response::on_hover_text) on Text + Button ----

#[test]
fn test_text_tooltip() {
    let code = r#"
type App =
    x: str = ""

view App =>
    <Panel>
        <Text tooltip="Helpful hint">Hello</Text>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains(".on_hover_text(\"Helpful hint\")"), "missing text tooltip:\n{rust}");
    assert!(rust.contains("ui.label(egui::RichText::new(\"Hello\")).on_hover_text(\"Helpful hint\");"), "missing tooltip chain:\n{rust}");
}

#[test]
fn test_button_tooltip_with_onclick() {
    let code = r#"
type App =
    count: int = 0

view App =>
    <Panel>
        <Button tooltip="Click me" onClick={count = count + 1}>Go</Button>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains(".on_hover_text(\"Click me\").clicked()"), "missing button tooltip chain:\n{rust}");
    assert!(rust.contains("self.count = (self.count + 1);"), "missing onclick action:\n{rust}");
}

#[test]
fn test_button_without_tooltip_unchanged() {
    let code = r#"
type App =
    count: int = 0

view App =>
    <Panel>
        <Button onClick={count = count + 1}>Go</Button>
    </Panel>
"#;
    let rust = transpile_and_verify(code);
    assert!(!rust.contains("on_hover_text"), "unexpected tooltip:\n{rust}");
    assert!(rust.contains(".clicked()"), "missing clicked:\n{rust}");
}
