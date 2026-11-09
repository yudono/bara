use crate::ast::{JsxAttrValue, JsxAttribute, JsxChild, JsxElement, ViewDef};
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Parse a Brui XML layout file into a `ViewDef`.
/// Expected format:
/// ```xml
/// <view name="LoginForm">
///   <Panel class="screen">
///     <Col class="card">
///       <Text class="title">Hello {name}</Text>
///       <Input bind="username" placeholder="Username" />
///       <Button onClick="login()">Login</Button>
///     </Col>
///   </Panel>
/// </view>
/// ```
/// If no outer `<view>` wrapper is present, the first Brui container element
/// is used as root and `default_name` becomes the view type name.
pub fn parse_brui_xml(content: &str, default_name: &str) -> Result<ViewDef, String> {
    let mut parser = XmlParser::new(content);
    let root = parser.parse_element()?;
    // Skip XML declarations / comments already handled; expect single root
    if root.tag == "view" {
        let type_name = root
            .attributes
            .iter()
            .find(|a| a.name == "name")
            .map(|a| match &a.value {
                JsxAttrValue::Lit(s) => s.clone(),
                JsxAttrValue::Expr(e) => format!("{e:?}"),
            })
            .unwrap_or_else(|| default_name.to_string());
        // view wrapper should contain exactly one Brui root element
        let mut inner_elems: Vec<JsxElement> = Vec::new();
        let mut inner_texts: Vec<String> = Vec::new();
        for child in root.children {
            match child {
                JsxChild::Element(e) => inner_elems.push(e),
                JsxChild::Text(t) => {
                    if !t.trim().is_empty() {
                        inner_texts.push(t);
                    }
                }
                JsxChild::Expr(e) => {
                    // wrap bare expression in a Text container for consistency
                    inner_elems.push(JsxElement {
                        tag: "Text".to_string(),
                        attributes: Vec::new(),
                        children: vec![JsxChild::Expr(e)],
                        span: (1, 1),
                    });
                }
                _ => {}
            }
        }
        let root_elem = if inner_elems.len() == 1 {
            inner_elems.into_iter().next().unwrap()
        } else if inner_elems.is_empty() {
            return Err("Brui XML <view> must contain at least one child element".to_string());
        } else {
            // Multiple roots: wrap in a Col
            JsxElement {
                tag: "Col".to_string(),
                attributes: Vec::new(),
                children: inner_elems.into_iter().map(JsxChild::Element).collect(),
                span: (1, 1),
            }
        };
        Ok(ViewDef {
            type_name: sanitize_type_name(&type_name, default_name),
            root: root_elem,
            span: (1, 1),
        })
    } else {
        Ok(ViewDef {
            type_name: sanitize_type_name(&root.tag, default_name).max_with_default(default_name),
            root,
            span: (1, 1),
        })
    }
}

fn sanitize_type_name(raw: &str, fallback: &str) -> String {
    let t = raw.trim();
    if t.is_empty() || t == "view" {
        return fallback.to_string();
    }
    // If raw looks like a tag name (Panel, Col...), it is not a type name -> use fallback
    let brui_tags = [
        "Panel", "Col", "Row", "Scroll", "Text", "Button", "Input", "TextField",
        "Checkbox", "Slider", "Separator", "Wrap", "Container", "Image", "Progress",
        "Spacer", "RadioButton", "Picker", "Dropdown", "Select", "Option",
        "Switch", "Toggle", "Modal", "Dialog", "Spinner", "ActivityIndicator",
        "BottomSheet", "Drawer", "SideMenu", "Stack", "VStack", "HStack",
    ];
    if brui_tags.contains(&t) {
        return fallback.to_string();
    }
    // Capitalize first letter for Rust type consistency
    let mut chars = t.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.collect::<String>()),
        None => fallback.to_string(),
    }
}

trait MaxWithDefault {
    fn max_with_default(self, default_name: &str) -> String;
}

impl MaxWithDefault for String {
    fn max_with_default(self, default_name: &str) -> String {
        if self.trim().is_empty() {
            default_name.to_string()
        } else {
            self
        }
    }
}

struct XmlParser<'a> {
    chars: Vec<char>,
    pos: usize,
    input: &'a str,
}

impl<'a> XmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            input,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn starts_with(&self, s: &str) -> bool {
        let v: Vec<char> = s.chars().collect();
        self.chars[self.pos..]
            .iter()
            .take(v.len())
            .collect::<Vec<_>>()
            == v.iter().collect::<Vec<_>>()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        Some(c)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse_element(&mut self) -> Result<JsxElement, String> {
        self.skip_ws();
        // Skip XML declaration <?xml ... ?> and comments <!-- ... -->
        loop {
            self.skip_ws();
            if self.starts_with("<?") {
                while self.pos < self.chars.len() && !self.starts_with("?>") {
                    self.advance();
                }
                for _ in 0..2 {
                    self.advance();
                }
                continue;
            }
            if self.starts_with("<!--") {
                for _ in 0..4 {
                    self.advance();
                }
                while self.pos < self.chars.len() && !self.starts_with("-->") {
                    self.advance();
                }
                for _ in 0..3 {
                    self.advance();
                }
                continue;
            }
            break;
        }
        self.skip_ws();
        if self.peek() != Some('<') {
            return Err(format!(
                "Brui XML: expected '<' at byte offset {} in: {}",
                self.pos,
                preview(self.input)
            ));
        }
        self.advance(); // <
        let tag = self.parse_tag_name()?;
        let attributes = self.parse_attributes()?;
        self.skip_ws();
        // Self-closing <Tag ... />
        if self.starts_with("/>") {
            self.advance();
            self.advance();
            return Ok(JsxElement {
                tag,
                attributes,
                children: Vec::new(),
                span: (1, 1),
            });
        }
        if self.peek() != Some('>') {
            return Err(format!("Brui XML: expected '>' after <{tag}"));
        }
        self.advance(); // >

        let mut children: Vec<JsxChild> = Vec::new();
        loop {
            self.skip_ws();
            if self.pos >= self.chars.len() {
                return Err(format!("Brui XML: unclosed tag <{tag}>"));
            }
            if self.starts_with("</") {
                self.advance();
                self.advance();
                let close_tag = self.parse_tag_name()?;
                self.skip_ws();
                if self.peek() != Some('>') {
                    return Err(format!("Brui XML: expected '>' in </{close_tag}>"));
                }
                self.advance();
                if close_tag != tag {
                    return Err(format!(
                        "Brui XML: mismatched closing tag </{close_tag}>, expected </{tag}>"
                    ));
                }
                break;
            }
            if self.starts_with("<!--") {
                for _ in 0..4 {
                    self.advance();
                }
                while self.pos < self.chars.len() && !self.starts_with("-->") {
                    self.advance();
                }
                for _ in 0..3 {
                    self.advance();
                }
                continue;
            }
            if self.peek() == Some('<') {
                // Nested element or {expr} in XML? Support <expr>{...}</expr>? No, just element.
                let child = self.parse_element()?;
                children.push(JsxChild::Element(child));
            } else if self.peek() == Some('{') {
                // {expr} expression child (consistent with .brx)
                self.advance();
                let mut expr_str = String::new();
                let mut depth = 1;
                while self.pos < self.chars.len() {
                    let c = self.advance().unwrap();
                    if c == '{' {
                        depth += 1;
                        expr_str.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_str.push(c);
                    } else {
                        expr_str.push(c);
                    }
                }
                let expr = parse_bara_expr(&expr_str)?;
                children.push(JsxChild::Expr(expr));
            } else {
                // Text node (may contain {expr} interpolation)
                let text = self.parse_text()?;
                if !text.trim().is_empty() {
                    // Split on {expr} segments
                    let mut rest = text.as_str();
                    while let Some(start) = rest.find('{') {
                        let end = rest.find('}');
                        if let Some(end_idx) = end {
                            if end_idx > start {
                                let lit = rest[..start].trim();
                                if !lit.is_empty() {
                                    children.push(JsxChild::Text(lit.to_string()));
                                }
                                let expr_str = &rest[start + 1..end_idx];
                                if !expr_str.trim().is_empty() {
                                    let expr = parse_bara_expr(expr_str)?;
                                    children.push(JsxChild::Expr(expr));
                                }
                                rest = &rest[end_idx + 1..];
                                continue;
                            }
                        }
                        break;
                    }
                    if !rest.trim().is_empty() {
                        children.push(JsxChild::Text(rest.trim().to_string()));
                    }
                }
            }
        }

        Ok(JsxElement {
            tag,
            attributes,
            children,
            span: (1, 1),
        })
    }

    fn parse_tag_name(&mut self) -> Result<String, String> {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err("Brui XML: expected tag name".to_string());
        }
        Ok(name)
    }

    fn parse_attributes(&mut self) -> Result<Vec<JsxAttribute>, String> {
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('>') | Some('/') | None => break,
                _ => {}
            }
            let name = self.parse_tag_name()?;
            self.skip_ws();
            let value = if self.peek() == Some('=') {
                self.advance();
                self.skip_ws();
                let quote = self.peek().ok_or("Brui XML: expected attribute value")?;
                if quote != '"' && quote != '\'' {
                    return Err(format!("Brui XML: attribute '{name}' value must be quoted"));
                }
                self.advance();
                let mut val = String::new();
                while let Some(c) = self.peek() {
                    if c == quote {
                        self.advance();
                        break;
                    }
                    val.push(c);
                    self.advance();
                }
                map_xml_attr(&name, &val)?
            } else {
                JsxAttrValue::Lit("true".to_string())
            };
            attrs.push(JsxAttribute {
                name: normalize_attr_name(&name),
                value,
                span: (1, 1),
            });
        }
        Ok(attrs)
    }

    fn parse_text(&mut self) -> Result<String, String> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == '<' || c == '{' {
                break;
            }
            s.push(c);
            self.advance();
        }
        Ok(s)
    }
}

fn normalize_attr_name(name: &str) -> String {
    match name {
        "className" | "class" => "class".to_string(),
        "on_click" | "onclick" | "onClick" => "onClick".to_string(),
        "bind" | "model" => "bind".to_string(),
        "hint" | "placeholder" => "placeholder".to_string(),
        "name" => "name".to_string(),
        other => other.to_string(),
    }
}

fn map_xml_attr(name: &str, val: &str) -> Result<JsxAttrValue, String> {
    let normalized = normalize_attr_name(name);
    match normalized.as_str() {
        "class" | "placeholder" | "name" | "value" | "progress" | "percent" | "size"
        | "amount" | "space" | "src" | "source" | "path" | "image" | "url" | "file" => {
            Ok(JsxAttrValue::Lit(val.to_string()))
        }
        "bind" => {
            // bind="field" -> Expr::Ident(field)
            let expr = parse_bara_expr(val)?;
            Ok(JsxAttrValue::Expr(expr))
        }
        "onClick" => {
            let expr = parse_bara_expr(val)?;
            Ok(JsxAttrValue::Expr(expr))
        }
        _ => {
            // Try to parse as Bara expression if wrapped in {..}, else literal
            let t = val.trim();
            if t.starts_with('{') && t.ends_with('}') && t.len() >= 2 {
                let inner = &t[1..t.len() - 1];
                Ok(JsxAttrValue::Expr(parse_bara_expr(inner)?))
            } else {
                Ok(JsxAttrValue::Lit(val.to_string()))
            }
        }
    }
}

fn parse_bara_expr(src: &str) -> Result<crate::ast::Expr, String> {
    let tokens = Lexer::new(src)
        .tokenize()
        .map_err(|e| format!("Brui XML expression '{src}': {e}"))?;
    let mut parser = Parser::new(tokens);
    parser
        .parse_expr()
        .map_err(|e| format!("Brui XML expression '{src}': {e}"))
}

fn preview(s: &str) -> String {
    s.chars().take(80).collect()
}

fn default_type_from_filename(path: &std::path::Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("BruiView");
    let mut chars = stem.chars();
    match chars.next() {
        Some(f) => format!("{}{}", f.to_ascii_uppercase(), chars.collect::<String>()),
        None => "BruiView".to_string(),
    }
}

/// Load all `.xml` Brui layouts under `project_root` (recursive) into views.
pub fn collect_xml_views(project_root: &std::path::Path) -> Vec<Result<ViewDef, String>> {
    let mut out = Vec::new();
    collect_xml_recursive(project_root, &mut out);
    out
}

fn collect_xml_recursive(dir: &std::path::Path, out: &mut Vec<Result<ViewDef, String>>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if fname == "target" || fname == ".bara_cache" || fname == ".git" {
                continue;
            }
            collect_xml_recursive(&p, out);
        } else if p.extension().map(|e| e == "xml").unwrap_or(false) {
            let content = match std::fs::read_to_string(&p) {
                Ok(c) => c,
                Err(e) => {
                    out.push(Err(format!("Failed to read {}: {e}", p.display())));
                    continue;
                }
            };
            let default_name = default_type_from_filename(&p);
            out.push(parse_brui_xml(&content, &default_name));
        }
    }
}
