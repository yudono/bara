use crate::analyzer::Analyzer;
use crate::ast::*;
use crate::css_engine::CssStyleSheet;

pub struct Codegen<'a> {
    analyzer: &'a Analyzer,
    output: String,
    indent_level: usize,
}

impl<'a> Codegen<'a> {
    pub fn new(analyzer: &'a Analyzer) -> Self {
        Self {
            analyzer,
            output: String::new(),
            indent_level: 0,
        }
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn writeln(&mut self, s: &str) {
        if s.trim().is_empty() {
            self.output.push('\n');
        } else {
            self.output.push_str(&self.indent());
            self.output.push_str(s);
            self.output.push('\n');
        }
    }

    pub fn generate(&mut self, program: &Program) -> String {
        self.output.clear();

        // 1. Module attributes
        self.writeln("#![allow(unused_imports, unused_mut, unused_variables, dead_code, unused_parens)]");
        self.writeln("");

        // 2. BaraDyn enum if needed
        if self.analyzer.needs_bara_dyn {
            self.emit_bara_dyn();
        }

        // 3. Collect CSS stylesheets
        let mut stylesheet = CssStyleSheet::default();
        for item in &program.items {
            if let Item::StyleBlock(css, _) = item {
                let sheet = CssStyleSheet::parse(css);
                stylesheet.rules.extend(sheet.rules);
            }
        }

        // 4. User imports
        for item in &program.items {
            if let Item::Use(u) = item {
                let path = u.path.join("::");
                if path == "brui" || path == "TodoApp" {
                    continue;
                }
                if let Some(alias) = &u.alias {
                    self.writeln(&format!("use {path} as {alias};"));
                } else {
                    self.writeln(&format!("use {path};"));
                }
            }
        }
        self.writeln("");

        // 5. RawRust top-level items
        for item in &program.items {
            if let Item::RawRust(code, span) = item {
                self.writeln(&format!("// @line: {}", span.0));
                self.writeln(code);
                self.writeln("");
            }
        }

        // 6. Types (Structs + Default + Impl)
        for item in &program.items {
            if let Item::Type(t) = item {
                self.emit_type(t);
            }
        }

        // 7. Standalone functions
        for item in &program.items {
            if let Item::Fn(f) = item {
                self.emit_function(f, false);
            }
        }

        // 8. Views (eframe::App implementations for .brx)
        for item in &program.items {
            if let Item::View(v) = item {
                self.emit_view(v, &stylesheet);
            }
        }

        // 9. Top-level statements -> main()
        let top_stmts: Vec<&Stmt> = program
            .items
            .iter()
            .filter_map(|item| {
                if let Item::TopLevelStmt(s) = item {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();

        if !top_stmts.is_empty() {
            self.emit_main(&top_stmts, program);
        }

        // Try formatting with prettyplease for 100% idiomatic Rust
        let raw = self.output.clone();
        if let Ok(syntax_tree) = syn::parse_file(&raw) {
            prettyplease::unparse(&syntax_tree)
        } else {
            raw
        }
    }

    fn emit_bara_dyn(&mut self) {
        self.writeln("#[derive(Debug, Clone, PartialEq)]");
        self.writeln("pub enum BaraDyn {");
        self.writeln("    Int(i64),");
        self.writeln("    Float(f64),");
        self.writeln("    Str(String),");
        self.writeln("    Bool(bool),");
        self.writeln("    List(Vec<BaraDyn>),");
        self.writeln("    Map(std::collections::HashMap<String, BaraDyn>),");
        self.writeln("    None,");
        self.writeln("}");
        self.writeln("");
        self.writeln("impl std::fmt::Display for BaraDyn {");
        self.writeln("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {");
        self.writeln("        match self {");
        self.writeln("            BaraDyn::Int(i) => write!(f, \"{}\", i),");
        self.writeln("            BaraDyn::Float(fl) => write!(f, \"{}\", fl),");
        self.writeln("            BaraDyn::Str(s) => write!(f, \"{}\", s),");
        self.writeln("            BaraDyn::Bool(b) => write!(f, \"{}\", b),");
        self.writeln("            BaraDyn::List(l) => write!(f, \"{:?}\", l),");
        self.writeln("            BaraDyn::Map(m) => write!(f, \"{:?}\", m),");
        self.writeln("            BaraDyn::None => write!(f, \"None\"),");
        self.writeln("        }");
        self.writeln("    }");
        self.writeln("}");
        self.writeln("");
    }

    fn emit_type(&mut self, t: &TypeDef) {
        self.writeln(&format!("// @line: {}", t.span.0));
        self.writeln("#[derive(Debug, Clone, PartialEq)]");
        self.writeln(&format!("pub struct {} {{", t.name));
        self.indent_level += 1;
        for f in &t.fields {
            let ty_str = f
                .ty
                .as_ref()
                .map(|ty| ty.to_string())
                .unwrap_or_else(|| "String".to_string());
            self.writeln(&format!("pub {}: {},", f.name, ty_str));
        }
        self.indent_level -= 1;
        self.writeln("}");
        self.writeln("");

        // impl Default
        self.writeln(&format!("impl Default for {} {{", t.name));
        self.indent_level += 1;
        self.writeln("fn default() -> Self {");
        self.indent_level += 1;
        self.writeln("Self {");
        self.indent_level += 1;
        for f in &t.fields {
            if let Some(def_val) = &f.default_val {
                let val_code = self.expr_to_string(def_val);
                self.writeln(&format!("{}: {},", f.name, val_code));
            } else {
                let def_expr = match f.ty.as_ref() {
                    Some(TypeAnnotation::Int) => "0",
                    Some(TypeAnnotation::Float) => "0.0",
                    Some(TypeAnnotation::Bool) => "false",
                    Some(TypeAnnotation::Str) => "String::new()",
                    Some(TypeAnnotation::List(_)) => "Vec::new()",
                    Some(TypeAnnotation::Map(_, _)) => "std::collections::HashMap::new()",
                    _ => "Default::default()",
                };
                self.writeln(&format!("{}: {},", f.name, def_expr));
            }
        }
        self.indent_level -= 1;
        self.writeln("}");
        self.indent_level -= 1;
        self.writeln("}");
        self.indent_level -= 1;
        self.writeln("}");
        self.writeln("");

        // Constructor helper: TypeName(args)
        let constructor_params: Vec<String> = t
            .fields
            .iter()
            .map(|f| {
                let ty_str = f
                    .ty
                    .as_ref()
                    .map(|ty| ty.to_string())
                    .unwrap_or_else(|| "String".to_string());
                if ty_str == "String" {
                    format!("{}: impl Into<String>", f.name)
                } else {
                    format!("{}: {}", f.name, ty_str)
                }
            })
            .collect();

        let constructor_fields: Vec<String> = t
            .fields
            .iter()
            .map(|f| {
                let ty_str = f
                    .ty
                    .as_ref()
                    .map(|ty| ty.to_string())
                    .unwrap_or_else(|| "String".to_string());
                if ty_str == "String" {
                    format!("{}: {}.into()", f.name, f.name)
                } else {
                    format!("{}: {}", f.name, f.name)
                }
            })
            .collect();

        self.writeln(&format!("#[allow(non_snake_case)]"));
        self.writeln(&format!(
            "pub fn {}({}) -> {} {{",
            t.name,
            constructor_params.join(", "),
            t.name
        ));
        self.indent_level += 1;
        self.writeln(&format!("{} {{ {} }}", t.name, constructor_fields.join(", ")));
        self.indent_level -= 1;
        self.writeln("}");
        self.writeln("");

        // impl <TypeName>
        if !t.methods.is_empty() {
            self.writeln(&format!("impl {} {{", t.name));
            self.indent_level += 1;
            for m in &t.methods {
                self.emit_function(m, true);
            }
            self.indent_level -= 1;
            self.writeln("}");
            self.writeln("");
        }
    }

    fn emit_function(&mut self, f: &FnDef, is_method: bool) {
        self.writeln(&format!("// @line: {}", f.span.0));
        let async_prefix = if f.is_async { "async " } else { "" };

        let mut param_strs = Vec::new();

        if is_method {
            match f.receiver {
                ReceiverKind::RefSelf => param_strs.push("&self".to_string()),
                ReceiverKind::MutSelf => param_strs.push("&mut self".to_string()),
                ReceiverKind::ValueSelf => param_strs.push("self".to_string()),
                ReceiverKind::None => {}
            }
        }

        for p in &f.params {
            let ty_str = if let Some(ty) = &p.ty {
                ty.to_string()
            } else {
                "i64".to_string()
            };

            let prefix = match p.ownership {
                Some(ParamOwnership::MutRef) => "&mut ",
                Some(ParamOwnership::Ref) => {
                    if ty_str == "String" || ty_str.starts_with("Vec<") {
                        "&"
                    } else if ty_str == "i64" || ty_str == "f64" || ty_str == "bool" {
                        ""
                    } else {
                        "&"
                    }
                }
                Some(ParamOwnership::Value) | None => "",
            };

            param_strs.push(format!(
                "{}{}: {prefix}{ty_str}",
                if p.ownership == Some(ParamOwnership::MutRef) && prefix.is_empty() {
                    "mut "
                } else {
                    ""
                },
                p.name
            ));
        }

        let ret_str = if let Some(ret) = &f.return_ty {
            if f.is_async || f.name.ends_with('!') || self.block_contains_try(&f.body) {
                format!(" -> Result<{}, Box<dyn std::error::Error>>", ret)
            } else {
                format!(" -> {}", ret)
            }
        } else if f.is_async || self.block_contains_try(&f.body) {
            " -> Result<(), Box<dyn std::error::Error>>".to_string()
        } else {
            String::new()
        };

        self.writeln(&format!(
            "pub {async_prefix}fn {}({}){} {{",
            f.name,
            param_strs.join(", "),
            ret_str
        ));

        self.indent_level += 1;
        self.emit_block_body(&f.body, f.is_async || self.block_contains_try(&f.body));
        self.indent_level -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn block_contains_try(&self, block: &Block) -> bool {
        for s in &block.stmts {
            if self.stmt_contains_try(s) {
                return true;
            }
        }
        false
    }

    fn stmt_contains_try(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Assign { value, .. } | Stmt::Expr(value) => self.expr_contains_try(value),
            Stmt::Return(Some(e), _) => self.expr_contains_try(e),
            Stmt::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                if self.expr_contains_try(cond) || self.block_contains_try(then_branch) {
                    return true;
                }
                for (c, b) in elif_branches {
                    if self.expr_contains_try(c) || self.block_contains_try(b) {
                        return true;
                    }
                }
                if let Some(eb) = else_branch {
                    return self.block_contains_try(eb);
                }
                false
            }
            Stmt::For { iter, body, .. } => {
                self.expr_contains_try(iter) || self.block_contains_try(body)
            }
            Stmt::While { cond, body, .. } => {
                self.expr_contains_try(cond) || self.block_contains_try(body)
            }
            _ => false,
        }
    }

    fn expr_contains_try(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Try(_, _) => true,
            Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
                self.expr_contains_try(left) || self.expr_contains_try(right)
            }
            Expr::Assign { target, value, .. } => {
                self.expr_contains_try(target) || self.expr_contains_try(value)
            }
            Expr::Closure { body, .. } => self.expr_contains_try(body),
            Expr::Unary { expr: e, .. }
            | Expr::Go(e, _)
            | Expr::Clone(e, _)
            | Expr::Borrow(e, _, _)
            | Expr::Cast { expr: e, .. }
            | Expr::DynWrap { expr: e, .. } => self.expr_contains_try(e),
            Expr::Call { callee, args, .. } => {
                self.expr_contains_try(callee) || args.iter().any(|a| self.expr_contains_try(a))
            }
            Expr::MethodCall {
                receiver,
                args,
                ..
            } => self.expr_contains_try(receiver) || args.iter().any(|a| self.expr_contains_try(a)),
            Expr::Index {
                collection,
                index,
                ..
            } => self.expr_contains_try(collection) || self.expr_contains_try(index),
            _ => false,
        }
    }

    fn emit_view(&mut self, view: &ViewDef, css: &CssStyleSheet) {
        self.writeln(&format!("// @line: {}", view.span.0));
        self.writeln(&format!("impl eframe::App for {} {{", view.type_name));
        self.indent_level += 1;
        self.writeln("fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {");
        self.indent_level += 1;

        self.emit_jsx_element(&view.root, css, "ctx", "ui");

        self.indent_level -= 1;
        self.writeln("}");
        self.indent_level -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_jsx_element(
        &mut self,
        elem: &JsxElement,
        css: &CssStyleSheet,
        ctx_var: &str,
        ui_var: &str,
    ) {
        let tag = elem.tag.as_str();

        // Extract class attribute
        let mut class_names: Vec<String> = Vec::new();
        let mut on_click_expr: Option<String> = None;
        let mut bind_var: Option<String> = None;
        let mut placeholder: Option<String> = None;

        for attr in &elem.attributes {
            match attr.name.as_str() {
                "class" => match &attr.value {
                    JsxAttrValue::Lit(s) => {
                        for c in s.split_whitespace() {
                            class_names.push(c.to_string());
                        }
                    }
                    JsxAttrValue::Expr(e) => {
                        // Extract class names from literals or ternary
                        self.collect_class_names_from_expr(e, &mut class_names);
                    }
                },
                "onClick" => {
                    if let JsxAttrValue::Expr(e) = &attr.value {
                        on_click_expr = Some(self.expr_to_string(e));
                    }
                }
                "bind" => {
                    if let JsxAttrValue::Expr(e) = &attr.value {
                        bind_var = Some(self.expr_to_string(e));
                    }
                }
                "placeholder" => {
                    if let JsxAttrValue::Lit(s) = &attr.value {
                        placeholder = Some(s.clone());
                    }
                }
                _ => {}
            }
        }

        let class_refs: Vec<&str> = class_names.iter().map(|s| s.as_str()).collect();

        match tag {
            "Panel" => {
                let frame_opt = css.get_frame_code(&class_refs);
                let frame_str = if let Some(fc) = frame_opt {
                    format!(".frame({fc})")
                } else {
                    String::new()
                };
                self.writeln(&format!(
                    "egui::CentralPanel::default(){frame_str}.show({ctx_var}, |{ui_var}| {{"
                ));
                self.indent_level += 1;
                for child in &elem.children {
                    self.emit_jsx_child(child, css, ui_var);
                }
                self.indent_level -= 1;
                self.writeln("});");
            }
            "Col" => {
                let frame_opt = css.get_frame_code(&class_refs);
                let gap_opt = css.get_spacing(&class_refs);
                if let Some(fc) = &frame_opt {
                    self.writeln(&format!("{fc}.show({ui_var}, |{ui_var}| {{"));
                    self.indent_level += 1;
                }
                self.writeln(&format!("{ui_var}.vertical(|{ui_var}| {{"));
                self.indent_level += 1;
                if let Some(gap) = gap_opt {
                    self.writeln(&format!("{ui_var}.spacing_mut().item_spacing.y = {:.1};", gap));
                }
                for child in &elem.children {
                    self.emit_jsx_child(child, css, ui_var);
                }
                self.indent_level -= 1;
                self.writeln("});");
                if frame_opt.is_some() {
                    self.indent_level -= 1;
                    self.writeln("});");
                }
            }
            "Row" => {
                let frame_opt = css.get_frame_code(&class_refs);
                let gap_opt = css.get_spacing(&class_refs);
                if let Some(fc) = &frame_opt {
                    self.writeln(&format!("{fc}.show({ui_var}, |{ui_var}| {{"));
                    self.indent_level += 1;
                }
                self.writeln(&format!("{ui_var}.horizontal(|{ui_var}| {{"));
                self.indent_level += 1;
                if let Some(gap) = gap_opt {
                    self.writeln(&format!("{ui_var}.spacing_mut().item_spacing.x = {:.1};", gap));
                }
                for child in &elem.children {
                    self.emit_jsx_child(child, css, ui_var);
                }
                self.indent_level -= 1;
                self.writeln("});");
                if frame_opt.is_some() {
                    self.indent_level -= 1;
                    self.writeln("});");
                }
            }
            "Scroll" => {
                let max_h = css.get_max_height(&class_refs).unwrap_or(300.0);
                self.writeln(&format!(
                    "egui::ScrollArea::vertical().max_height({:.1}).show({ui_var}, |{ui_var}| {{",
                    max_h
                ));
                self.indent_level += 1;
                for child in &elem.children {
                    self.emit_jsx_child(child, css, ui_var);
                }
                self.indent_level -= 1;
                self.writeln("});");
            }
            "Text" => {
                let mods = css.get_rich_text_modifiers(&class_refs);
                let mod_chain = if mods.is_empty() {
                    String::new()
                } else {
                    format!(".{}", mods.join("."))
                };
                let content = self.jsx_children_to_rich_text_content(&elem.children);
                self.writeln(&format!(
                    "{ui_var}.label(egui::RichText::new({content}){mod_chain});"
                ));
            }
            "Button" => {
                let mods = css.get_rich_text_modifiers(&class_refs);
                let mod_chain = if mods.is_empty() {
                    String::new()
                } else {
                    format!(".{}", mods.join("."))
                };
                let content = self.jsx_children_to_rich_text_content(&elem.children);
                let action = on_click_expr.unwrap_or_default();
                self.writeln(&format!(
                    "if {ui_var}.add(egui::Button::new(egui::RichText::new({content}){mod_chain})).clicked() {{"
                ));
                self.indent_level += 1;
                if !action.is_empty() {
                    self.writeln(&format!("{action};"));
                }
                self.indent_level -= 1;
                self.writeln("}");
            }
            "Input" => {
                if let Some(var) = bind_var {
                    let hint = if let Some(ph) = placeholder {
                        format!(".hint_text(\"{}\")", ph.escape_default())
                    } else {
                        String::new()
                    };
                    self.writeln(&format!(
                        "{ui_var}.add(egui::TextEdit::singleline(&mut {var}){hint});"
                    ));
                }
            }
            "Checkbox" => {
                if let Some(var) = bind_var {
                    self.writeln(&format!("{ui_var}.checkbox(&mut {var}, \"\");"));
                }
            }
            "Slider" => {
                if let Some(var) = bind_var {
                    self.writeln(&format!("{ui_var}.add(egui::Slider::new(&mut {var}, 0..=100));"));
                }
            }
            "Separator" => {
                self.writeln(&format!("{ui_var}.separator();"));
            }
            _ => {
                // Generic fallback container
                self.writeln(&format!("{ui_var}.vertical(|{ui_var}| {{"));
                self.indent_level += 1;
                for child in &elem.children {
                    self.emit_jsx_child(child, css, ui_var);
                }
                self.indent_level -= 1;
                self.writeln("});");
            }
        }
    }

    fn emit_jsx_child(&mut self, child: &JsxChild, css: &CssStyleSheet, ui_var: &str) {
        match child {
            JsxChild::Element(elem) => {
                self.emit_jsx_element(elem, css, "ctx", ui_var);
            }
            JsxChild::Text(t) => {
                self.writeln(&format!("{ui_var}.label(\"{}\");", t.escape_default()));
            }
            JsxChild::Expr(e) => {
                let e_str = self.expr_to_string(e);
                self.writeln(&format!("{ui_var}.label(format!(\"{{}}\", {e_str}));"));
            }
            JsxChild::If {
                cond,
                then_child,
                else_child,
            } => {
                let c_str = self.expr_to_string(cond);
                self.writeln(&format!("if {c_str} {{"));
                self.indent_level += 1;
                self.emit_jsx_child(then_child, css, ui_var);
                self.indent_level -= 1;
                if let Some(ec) = else_child {
                    self.writeln("} else {");
                    self.indent_level += 1;
                    self.emit_jsx_child(ec, css, ui_var);
                    self.indent_level -= 1;
                }
                self.writeln("}");
            }
            JsxChild::For { item, iter, body } => {
                let iter_str = self.expr_to_string(iter);
                self.writeln(&format!("for {item} in &mut {iter_str} {{"));
                self.indent_level += 1;
                self.emit_jsx_child(body, css, ui_var);
                self.indent_level -= 1;
                self.writeln("}");
            }
        }
    }

    fn jsx_children_to_rich_text_content(&self, children: &[JsxChild]) -> String {
        let mut fmt_parts = Vec::new();
        let mut args = Vec::new();

        for child in children {
            match child {
                JsxChild::Text(t) => {
                    fmt_parts.push(t.replace('{', "{{").replace('}', "}}"));
                }
                JsxChild::Expr(e) => {
                    fmt_parts.push("{}".to_string());
                    args.push(self.expr_to_string(e));
                }
                _ => {}
            }
        }

        let full_fmt = fmt_parts.join(" ");
        if args.is_empty() {
            format!("\"{}\"", full_fmt.escape_default())
        } else {
            format!("format!(\"{}\", {})", full_fmt.escape_default(), args.join(", "))
        }
    }

    fn collect_class_names_from_expr(&self, expr: &Expr, classes: &mut Vec<String>) {
        match expr {
            Expr::Literal(Literal::Str(s), _) => {
                for c in s.split_whitespace() {
                    classes.push(c.to_string());
                }
            }
            Expr::Ternary { then_expr, else_expr, .. } => {
                self.collect_class_names_from_expr(then_expr, classes);
                self.collect_class_names_from_expr(else_expr, classes);
            }
            _ => {}
        }
    }

    fn emit_main(&mut self, stmts: &[&Stmt], program: &Program) {
        // Check if there is an app.run(...) call in top-level statements
        let mut app_run_target = None;
        let mut app_title = "Bara Desktop App".to_string();
        let mut app_width = 450.0f32;
        let mut app_height = 600.0f32;

        for s in stmts {
            if let Stmt::Expr(Expr::MethodCall { method, args, .. }) = s {
                if method == "run" {
                    // Look up which app was loaded
                    if let Some(target) = self.find_loaded_app_target(program) {
                        app_run_target = Some(target);
                    }
                    for arg in args {
                        match arg {
                            Expr::Literal(Literal::Str(t), _) => {
                                app_title = t.clone();
                            }
                            Expr::Assign { target, value, .. } => {
                                if let Expr::Ident(name, _) = target.as_ref() {
                                    if name == "width" {
                                        if let Expr::Literal(Literal::Int(w), _) = value.as_ref() {
                                            app_width = *w as f32;
                                        }
                                    } else if name == "height" {
                                        if let Expr::Literal(Literal::Int(h), _) = value.as_ref() {
                                            app_height = *h as f32;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if let Some(target_app) = app_run_target {
            self.writeln("fn main() -> Result<(), eframe::Error> {");
            self.indent_level += 1;
            self.writeln("let mut native_options = eframe::NativeOptions::default();");
            self.writeln(&format!(
                "native_options.viewport = egui::ViewportBuilder::default().with_inner_size([{:.1}, {:.1}]);",
                app_width, app_height
            ));
            self.writeln(&format!(
                "eframe::run_native(\"{}\", native_options, Box::new(|_cc| Ok(Box::new({target_app}::default()))))",
                app_title.escape_default()
            ));
            self.indent_level -= 1;
            self.writeln("}");
            return;
        }

        let is_async = self.analyzer.main_is_async;
        let has_try = self.analyzer.main_has_try || is_async;

        if is_async {
            self.writeln("#[tokio::main]");
            self.writeln("async fn main() -> Result<(), Box<dyn std::error::Error>> {");
        } else if has_try {
            self.writeln("fn main() -> Result<(), Box<dyn std::error::Error>> {");
        } else {
            self.writeln("fn main() {");
        }

        self.indent_level += 1;
        let block = Block {
            stmts: stmts.iter().map(|s| (*s).clone()).collect(),
            span: (1, 1),
        };
        self.emit_block_body(&block, has_try);
        self.indent_level -= 1;
        self.writeln("}");
    }

    fn find_loaded_app_target(&self, program: &Program) -> Option<String> {
        for item in &program.items {
            if let Item::View(v) = item {
                return Some(v.type_name.clone());
            }
        }
        for item in &program.items {
            if let Item::Type(t) = item {
                if t.name.ends_with("App") {
                    return Some(t.name.clone());
                }
            }
        }
        None
    }

    fn emit_block_body(&mut self, block: &Block, wrap_ok: bool) {
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == n - 1;

            if is_last && wrap_ok {
                match stmt {
                    Stmt::Expr(e) => {
                        let e_str = self.expr_to_string(e);
                        self.writeln(&format!("Ok({e_str})"));
                        return;
                    }
                    _ => {
                        self.emit_stmt(stmt, false);
                        self.writeln("Ok(())");
                        return;
                    }
                }
            }

            self.emit_stmt(stmt, is_last);
        }

        if block.stmts.is_empty() && wrap_ok {
            self.writeln("Ok(())");
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt, is_last_in_block: bool) {
        match stmt {
            Stmt::RawRust(code, span) => {
                self.writeln(&format!("// @line: {}", span.0));
                self.writeln(code);
            }
            Stmt::Assign {
                target,
                value,
                op,
                is_decl,
                is_mut,
                span,
                ..
            } => {
                self.writeln(&format!("// @line: {}", span.0));
                let val_code = self.expr_to_string(value);
                let target_code = self.expr_to_string(target);

                if *is_decl {
                    let mut_keyword = if *is_mut { "mut " } else { "" };
                    self.writeln(&format!("let {mut_keyword}{target_code} = {val_code};"));
                } else if let Some(bin_op) = op {
                    self.writeln(&format!("{target_code} {bin_op}= {val_code};"));
                } else {
                    self.writeln(&format!("{target_code} = {val_code};"));
                }
            }
            Stmt::Expr(e) => {
                self.writeln(&format!("// @line: {}", e.span().0));
                let e_code = self.expr_to_string(e);
                if is_last_in_block {
                    self.writeln(&e_code);
                } else {
                    self.writeln(&format!("{e_code};"));
                }
            }
            Stmt::Return(opt_e, span) => {
                self.writeln(&format!("// @line: {}", span.0));
                if let Some(e) = opt_e {
                    let e_code = self.expr_to_string(e);
                    self.writeln(&format!("return {e_code};"));
                } else {
                    self.writeln("return;");
                }
            }
            Stmt::Break(span) => {
                self.writeln(&format!("// @line: {}", span.0));
                self.writeln("break;");
            }
            Stmt::Continue(span) => {
                self.writeln(&format!("// @line: {}", span.0));
                self.writeln("continue;");
            }
            Stmt::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
                span,
            } => {
                self.writeln(&format!("// @line: {}", span.0));
                let cond_code = self.expr_to_string(cond);
                self.writeln(&format!("if {cond_code} {{"));
                self.indent_level += 1;
                self.emit_block_body(then_branch, false);
                self.indent_level -= 1;

                for (c, b) in elif_branches {
                    let c_code = self.expr_to_string(c);
                    self.writeln(&format!("}} else if {c_code} {{"));
                    self.indent_level += 1;
                    self.emit_block_body(b, false);
                    self.indent_level -= 1;
                }

                if let Some(eb) = else_branch {
                    self.writeln("} else {");
                    self.indent_level += 1;
                    self.emit_block_body(eb, false);
                    self.indent_level -= 1;
                }
                self.writeln("}");
            }
            Stmt::For {
                item,
                iter,
                body,
                span,
            } => {
                self.writeln(&format!("// @line: {}", span.0));
                let iter_code = self.expr_to_string(iter);
                self.writeln(&format!("for {item} in {iter_code} {{"));
                self.indent_level += 1;
                self.emit_block_body(body, false);
                self.indent_level -= 1;
                self.writeln("}");
            }
            Stmt::While { cond, body, span } => {
                self.writeln(&format!("// @line: {}", span.0));
                let cond_code = self.expr_to_string(cond);
                self.writeln(&format!("while {cond_code} {{"));
                self.indent_level += 1;
                self.emit_block_body(body, false);
                self.indent_level -= 1;
                self.writeln("}");
            }
        }
    }

    fn expr_to_string(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(i) => format!("{i}"),
                Literal::Float(fl) => {
                    let s = format!("{fl}");
                    if s.contains('.') {
                        s
                    } else {
                        format!("{s}.0")
                    }
                }
                Literal::Bool(b) => format!("{b}"),
                Literal::None => "None".to_string(),
                Literal::Str(s) => format!("\"{}\".to_string()", s.escape_default()),
                Literal::InterpolatedStr(parts) => {
                    let mut fmt_str = String::new();
                    let mut args = Vec::new();
                    for p in parts {
                        match p {
                            StringPart::Lit(l) => {
                                fmt_str.push_str(&l.replace('{', "{{").replace('}', "}}"))
                            }
                            StringPart::Expr(e) => {
                                fmt_str.push_str("{}");
                                args.push(self.expr_to_string(e));
                            }
                        }
                    }
                    if args.is_empty() {
                        format!("\"{}\".to_string()", fmt_str.escape_default())
                    } else {
                        format!("format!(\"{}\", {})", fmt_str.escape_default(), args.join(", "))
                    }
                }
            },
            Expr::Ident(name, _) => name.clone(),
            Expr::SelfAccess(name, _) => {
                if name == "self" || name.is_empty() {
                    "self".to_string()
                } else {
                    format!("self.{name}")
                }
            }
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
                ..
            } => {
                let c = self.expr_to_string(cond);
                let t = self.expr_to_string(then_expr);
                let e = self.expr_to_string(else_expr);
                format!("(if {c} {{ {t} }} else {{ {e} }})")
            }
            Expr::Binary { op, left, right, .. } => {
                let l = self.expr_to_string(left);
                let r = self.expr_to_string(right);
                format!("({l} {op} {r})")
            }
            Expr::Unary { op, expr: inner, .. } => {
                let s = self.expr_to_string(inner);
                format!("{op}{s}")
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "print" {
                        return self.emit_print(args);
                    }
                }

                let c = self.expr_to_string(callee);
                let arg_strs: Vec<String> = args
                    .iter()
                    .map(|a| self.call_arg_to_string(a))
                    .collect();
                format!("{c}({})", arg_strs.join(", "))
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                if method == "filter" {
                    let r = self.expr_to_string(receiver);
                    let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_string(a)).collect();
                    format!("{r}.iter().filter({})", arg_strs.join(", "))
                } else if method == "len" {
                    let r = self.expr_to_string(receiver);
                    if let Expr::MethodCall { method: inner_m, .. } = receiver.as_ref() {
                        if inner_m == "filter" {
                            format!("({r}.count() as i64)")
                        } else {
                            format!("({r}.len() as i64)")
                        }
                    } else {
                        format!("({r}.len() as i64)")
                    }
                } else {
                    let r = self.expr_to_string(receiver);
                    let arg_strs: Vec<String> = args
                        .iter()
                        .map(|a| self.call_arg_to_string(a))
                        .collect();
                    format!("{r}.{method}({})", arg_strs.join(", "))
                }
            }
            Expr::MemberAccess { object, member, .. } => {
                let obj = self.expr_to_string(object);
                format!("{obj}.{member}")
            }
            Expr::PathAccess { segments, .. } => segments.join("::"),
            Expr::Index {
                collection, index, ..
            } => {
                let c = self.expr_to_string(collection);
                let idx = self.expr_to_string(index);
                if idx.contains("as usize") {
                    format!("{c}[{idx}]")
                } else {
                    format!("{c}[({idx}) as usize]")
                }
            }
            Expr::List(elements, _) => {
                let el_strs: Vec<String> = elements.iter().map(|e| self.expr_to_string(e)).collect();
                format!("vec![{}]", el_strs.join(", "))
            }
            Expr::Map(pairs, _) => {
                let mut inserts = Vec::new();
                for (k, v) in pairs {
                    let ks = self.expr_to_string(k);
                    let vs = self.expr_to_string(v);
                    inserts.push(format!("__map.insert({ks}, {vs});"));
                }
                format!(
                    "{{ let mut __map = std::collections::HashMap::new(); {} __map }}",
                    inserts.join(" ")
                )
            }
            Expr::Pipe { left, right, .. } => {
                let l = self.expr_to_string(left);
                let r = self.expr_to_string(right);
                format!("{r}({l})")
            }
            Expr::Try(inner, _) => {
                let s = self.expr_to_string(inner);
                if self.analyzer.main_is_async || s.contains("get(") || s.contains("text(") {
                    format!("{s}.await?")
                } else {
                    format!("{s}?")
                }
            }
            Expr::SafeNav { object, member, .. } => {
                let obj = self.expr_to_string(object);
                format!("{obj}.as_ref().map(|x| &x.{member})")
            }
            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                let s = start.as_ref().map(|e| self.expr_to_string(e)).unwrap_or_default();
                let op = if *inclusive { "..=" } else { ".." };
                let e = end.as_ref().map(|e| self.expr_to_string(e)).unwrap_or_default();
                format!("{s}{op}{e}")
            }
            Expr::Go(inner, _) => {
                let s = self.expr_to_string(inner);
                format!("tokio::spawn(async move {{ {s}; }})")
            }
            Expr::Clone(inner, _) => {
                let s = self.expr_to_string(inner);
                format!("{s}.clone()")
            }
            Expr::Borrow(inner, is_mut, _) => {
                let s = self.expr_to_string(inner);
                if *is_mut {
                    format!("&mut {s}")
                } else {
                    format!("&{s}")
                }
            }
            Expr::Cast {
                expr: inner,
                target_ty,
                ..
            } => {
                let s = self.expr_to_string(inner);
                let rust_ty = match target_ty.as_str() {
                    "float" => "f64",
                    "int" => "i64",
                    "str" => "&str",
                    "bool" => "bool",
                    other => other,
                };
                format!("({s} as {rust_ty})")
            }
            Expr::DynWrap {
                variant,
                expr: inner,
                ..
            } => {
                let s = self.expr_to_string(inner);
                if variant == "Str" && !s.contains(".to_string()") {
                    format!("BaraDyn::{variant}({s}.to_string())")
                } else {
                    format!("BaraDyn::{variant}({s})")
                }
            }
            Expr::Assign { target, value, .. } => {
                let t = self.expr_to_string(target);
                let v = self.expr_to_string(value);
                format!("{t} = {v}")
            }
            Expr::Closure { params, body, .. } => {
                let p = params.join(", ");
                let b = self.expr_to_string(body);
                format!("|{p}| {b}")
            }
            Expr::Block(b, _) => {
                let mut blk_cg = Codegen::new(self.analyzer);
                blk_cg.indent_level = self.indent_level + 1;
                blk_cg.emit_block_body(b, false);
                format!("{{\n{}}}", blk_cg.output)
            }
            Expr::RawRust(code, _) => code.clone(),
        }
    }

    fn call_arg_to_string(&self, arg: &Expr) -> String {
        let actual = if let Expr::Assign { value, .. } = arg {
            value.as_ref()
        } else {
            arg
        };

        if let Expr::SelfAccess(name, _) = actual {
            format!("self.{name}.clone()")
        } else {
            self.expr_to_string(actual)
        }
    }

    fn emit_print(&self, args: &[Expr]) -> String {
        if args.is_empty() {
            return "println!()".to_string();
        }

        if args.len() == 1 {
            if let Expr::Literal(Literal::InterpolatedStr(parts), _) = &args[0] {
                let mut fmt_str = String::new();
                let mut fmt_args = Vec::new();
                for p in parts {
                    match p {
                        StringPart::Lit(l) => {
                            fmt_str.push_str(&l.replace('{', "{{").replace('}', "}}"))
                        }
                        StringPart::Expr(e) => {
                            fmt_str.push_str("{}");
                            fmt_args.push(self.expr_to_string(e));
                        }
                    }
                }
                if fmt_args.is_empty() {
                    return format!("println!(\"{}\")", fmt_str.escape_default());
                } else {
                    return format!(
                        "println!(\"{}\", {})",
                        fmt_str.escape_default(),
                        fmt_args.join(", ")
                    );
                }
            }

            let arg_s = self.expr_to_string(&args[0]);
            return format!("println!(\"{{}}\", {arg_s})");
        }

        let placeholders: Vec<&str> = args.iter().map(|_| "{}").collect();
        let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_string(a)).collect();
        format!(
            "println!(\"{}\", {})",
            placeholders.join(" "),
            arg_strs.join(", ")
        )
    }
}
