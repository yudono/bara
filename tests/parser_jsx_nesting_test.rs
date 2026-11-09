use bara::analyzer::Analyzer;
use bara::ast::*;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn parse(input: &str) -> Program {
    let tokens = Lexer::new(input).tokenize().expect("lexer failed");
    Parser::new(tokens).parse_program().expect("parser failed")
}

fn parse_view(input: &str) -> ViewDef {
    let prog = parse(input);
    assert_eq!(prog.items.len(), 1, "expected 1 item for: {input}");
    if let Item::View(v) = &prog.items[0] {
        v.clone()
    } else {
        panic!("expected View item for: {input}");
    }
}

fn transpile_and_verify(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().expect("lexing failed");
    let mut program = Parser::new(tokens).parse_program().expect("parsing failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("analysis failed");
    let mut codegen = Codegen::new(&analyzer);
    let rust_code = codegen.generate(&program);
    syn::parse_file(&rust_code).expect("generated Rust must be valid (syn)");
    rust_code
}

#[test]
fn test_minimal_view_panel_text() {
    let v = parse_view("view App =>\n    <Panel>\n        <Text>Hi</Text>\n    </Panel>");
    assert_eq!(v.type_name, "App");
    assert_eq!(v.root.tag, "Panel");
    assert_eq!(v.root.children.len(), 1);
    if let JsxChild::Element(el) = &v.root.children[0] {
        assert_eq!(el.tag, "Text");
        assert_eq!(el.children, vec![JsxChild::Text("Hi".to_string())]);
    } else {
        panic!("expected Text element child");
    }
}

#[test]
fn test_nested_panel_col_row() {
    let v = parse_view(
        "view App =>\n    <Panel>\n        <Col>\n            <Row>\n                <Text>Label</Text>\n            </Row>\n        </Col>\n    </Panel>",
    );
    assert_eq!(v.root.tag, "Panel");
    let col = match &v.root.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Col element, got: {other:?}"),
    };
    assert_eq!(col.tag, "Col");
    let row = match &col.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Row element, got: {other:?}"),
    };
    assert_eq!(row.tag, "Row");
    assert!(matches!(&row.children[0], JsxChild::Element(e) if e.tag == "Text"));
}

#[test]
fn test_self_closing_button_and_input() {
    let v = parse_view(
        "view App =>\n    <Panel>\n        <Col>\n            <Input />\n            <Button>Go</Button>\n        </Col>\n    </Panel>",
    );
    let col = match &v.root.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Col, got: {other:?}"),
    };
    assert_eq!(col.children.len(), 2);
    match &col.children[0] {
        JsxChild::Element(e) => {
            assert_eq!(e.tag, "Input");
            assert!(e.children.is_empty());
        }
        other => panic!("expected self-closing Input, got: {other:?}"),
    }
    assert!(matches!(&col.children[1], JsxChild::Element(e) if e.tag == "Button"));
}

#[test]
fn test_class_string_attr() {
    let v = parse_view(
        "view App =>\n    <Panel class=\"container\">\n        <Text>Hi</Text>\n    </Panel>",
    );
    assert_eq!(v.root.attributes.len(), 1);
    assert_eq!(v.root.attributes[0].name, "class");
    assert!(
        matches!(&v.root.attributes[0].value, JsxAttrValue::Lit(s) if s == "container")
    );
}

#[test]
fn test_bind_expr_attr() {
    let v = parse_view(
        "view App =>\n    <Panel>\n        <Input bind={username} />\n    </Panel>",
    );
    let col = match &v.root.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Input, got: {other:?}"),
    };
    assert_eq!(col.attributes.len(), 1);
    assert_eq!(col.attributes[0].name, "bind");
    assert!(
        matches!(&col.attributes[0].value, JsxAttrValue::Expr(Expr::Ident(n, _)) if n == "username")
    );
}

#[test]
fn test_onclick_assign_attr() {
    let v = parse_view(
        "view App =>\n    <Panel>\n        <Button onClick={count = count + 1}>Add</Button>\n    </Panel>",
    );
    let btn = match &v.root.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Button, got: {other:?}"),
    };
    assert_eq!(btn.attributes[0].name, "onClick");
    assert!(
        matches!(&btn.attributes[0].value, JsxAttrValue::Expr(Expr::Assign { .. })),
        "onClick must hold an Assign expr"
    );
}

#[test]
fn test_placeholder_attr() {
    let v = parse_view(
        "view App =>\n    <Panel>\n        <Input placeholder=\"Enter name...\" />\n    </Panel>",
    );
    let input = match &v.root.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Input, got: {other:?}"),
    };
    assert!(
        matches!(&input.attributes[0].value, JsxAttrValue::Lit(s) if s == "Enter name...")
    );
}

#[test]
fn test_ternary_class_attr() {
    let v = parse_view(
        "view App =>\n    <Panel>\n        <Button class={ok ? \"on\" : \"off\"}>T</Button>\n    </Panel>",
    );
    let btn = match &v.root.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Button, got: {other:?}"),
    };
    assert!(
        matches!(&btn.attributes[0].value, JsxAttrValue::Expr(Expr::Ternary { .. })),
        "class must hold a Ternary expr"
    );
}

#[test]
fn test_for_child_rendering() {
    let v = parse_view(
        "view App =>\n    <Panel>\n        <Col>\n            {for t in todos =>\n                <Row>\n                    <Text>{t}</Text>\n                </Row>\n            }\n        </Col>\n    </Panel>",
    );
    let col = match &v.root.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Col, got: {other:?}"),
    };
    match &col.children[0] {
        JsxChild::For { item, body, .. } => {
            assert_eq!(item, "t");
            assert!(matches!(body.as_ref(), JsxChild::Element(e) if e.tag == "Row"));
        }
        other => panic!("expected For child, got: {other:?}"),
    }
}

#[test]
fn test_if_else_child_rendering() {
    let v = parse_view(
        "view App =>\n    <Panel>\n        <Col>\n            {if is_admin =>\n                <Text>Admin</Text>\n            else =>\n                <Text>User</Text>\n            }\n        </Col>\n    </Panel>",
    );
    let col = match &v.root.children[0] {
        JsxChild::Element(e) => e,
        other => panic!("expected Col, got: {other:?}"),
    };
    match &col.children[0] {
        JsxChild::If { then_child, else_child, .. } => {
            assert!(matches!(then_child.as_ref(), JsxChild::Element(e) if e.tag == "Text"));
            assert!(matches!(else_child, Some(b) if matches!(b.as_ref(), JsxChild::Element(e) if e.tag == "Text")));
        }
        other => panic!("expected If child, got: {other:?}"),
    }
}

#[test]
fn test_full_view_transpile_syn() {
    let rust = transpile_and_verify(
        "type SettingsForm =\n    username: str = \"\"\n    count: int = 0\n\nview SettingsForm =>\n    <Panel class=\"container\">\n        <Col>\n            <Row>\n                <Text>Hello</Text>\n                <Button class=\"btn\" onClick={count = count + 1}>Add</Button>\n            </Row>\n            <Input bind={username} placeholder=\"Enter name...\" />\n        </Col>\n    </Panel>",
    );
    assert!(rust.contains("impl eframe::App for SettingsForm"), "output was:\n{rust}");
    assert!(rust.contains("egui::TextEdit::singleline"), "output was:\n{rust}");
}
