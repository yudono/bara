use std::collections::{HashMap, HashSet};
use crate::analyzer::Analyzer;
use crate::ast::*;
use crate::css_engine::CssStyleSheet;

pub struct Codegen<'a> {
    analyzer: &'a Analyzer,
    output: String,
    indent_level: usize,
    jsx_for_var: Option<String>,
    struct_defs: HashMap<String, TypeDef>,
    /// Type names that have a `view` (renderable child components).
    component_views: HashSet<String>,
    /// View currently being emitted (parent scope for component instances).
    current_view: Option<String>,
    /// Per (parent view, child tag) instance counters (match analyzer injection).
    component_counters: HashMap<(String, String), usize>,
    /// True while rendering inside `__render_as_child` (Panel becomes a frame).
    in_child: bool,
    /// Collected `useEffect` snippets, drained at the end of `update()`.
    pending_effects: Vec<String>,
}

impl<'a> Codegen<'a> {
    pub fn new(analyzer: &'a Analyzer) -> Self {
        Self {
            analyzer,
            output: String::new(),
            indent_level: 0,
            jsx_for_var: None,
            struct_defs: HashMap::new(),
            component_views: HashSet::new(),
            current_view: None,
            component_counters: HashMap::new(),
            in_child: false,
            pending_effects: Vec::new(),
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

        self.struct_defs.clear();
        for item in &program.items {
            if let Item::Type(t) = item {
                self.struct_defs.insert(t.name.clone(), t.clone());
            }
        }

        self.component_views.clear();
        for item in &program.items {
            if let Item::View(v) = item {
                self.component_views.insert(v.type_name.clone());
            }
        }

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

        // 3.5 Module declarations
        for item in &program.items {
            if let Item::Mod(m) = item {
                self.writeln(&format!("// @line: {}", m.span.0));
                let pub_prefix = if m.is_pub { "pub " } else { "" };
                self.writeln(&format!("{pub_prefix}mod {};", m.name));
            }
        }

        // 4. User imports
        let known_external_crates = [
            "std", "core", "alloc", "tokio", "serde", "serde_json", "egui", "eframe", "anyhow", "thiserror",
        ];
        let mut internal_names = HashSet::new();
        internal_names.insert("brui".to_string());
        for item in &program.items {
            match item {
                Item::Type(t) => {
                    internal_names.insert(t.name.clone());
                }
                Item::Enum(e) => {
                    internal_names.insert(e.name.clone());
                }
                Item::View(v) => {
                    internal_names.insert(v.type_name.clone());
                }
                _ => {}
            }
        }

        for item in &program.items {
            if let Item::Use(u) = item {
                // Explicit stylesheet imports (use "styles.css" / use styles.css /
                // use styles) are resolved at transpile time into StyleBlocks and
                // must never be emitted as Rust `use` items.
                if crate::cargo_bridge::is_stylesheet_use(&u.path) {
                    continue;
                }
                let path = u.path.join("::");
                if u.path.len() == 1 {
                    let first = &u.path[0];
                    if internal_names.contains(first)
                        || !known_external_crates.contains(&first.as_str())
                    {
                        continue;
                    }
                }
                let pub_prefix = if u.is_pub { "pub " } else { "" };
                if let Some(alias) = &u.alias {
                    self.writeln(&format!("{pub_prefix}use {path} as {alias};"));
                } else {
                    self.writeln(&format!("{pub_prefix}use {path};"));
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

        // 5.5 Enums
        for item in &program.items {
            if let Item::Enum(e) = item {
                self.emit_enum(e);
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

    fn emit_enum(&mut self, e: &EnumDef) {
        self.writeln(&format!("// @line: {}", e.span.0));
        self.writeln("#[derive(Debug, Clone, PartialEq)]");
        let pub_prefix = if e.is_pub { "pub " } else { "pub " };
        self.writeln(&format!("{pub_prefix}enum {} {{", e.name));
        self.indent_level += 1;
        for v in &e.variants {
            if v.fields.is_empty() {
                self.writeln(&format!("{},", v.name));
            } else {
                let f_strs: Vec<String> = v.fields.iter().map(|f| f.to_string()).collect();
                self.writeln(&format!("{}({}),", v.name, f_strs.join(", ")));
            }
        }
        self.indent_level -= 1;
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

        // Constructor helper: TypeName(args). Hidden React state fields
        // (`__child_*`, `__br_mounted`, `__fx_sig_*`) are never params; when
        // present we build via assignment so `Drop` types keep working
        // (struct-update syntax cannot move fields out of a Drop value).
        let visible_fields: Vec<&FieldDef> =
            t.fields.iter().filter(|f| !is_hidden_field(&f.name)).collect();
        let ctor_pairs: Vec<(String, String)> = visible_fields
            .iter()
            .map(|f| {
                let ty_str = f
                    .ty
                    .as_ref()
                    .map(|ty| ty.to_string())
                    .unwrap_or_else(|| "String".to_string());
                let expr = if ty_str == "String" {
                    format!("{}.into()", f.name)
                } else {
                    f.name.clone()
                };
                (f.name.clone(), expr)
            })
            .collect();

        let constructor_params: Vec<String> = visible_fields
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

        self.writeln(&format!("#[allow(non_snake_case)]"));
        self.writeln(&format!(
            "pub fn {}({}) -> {} {{",
            t.name,
            constructor_params.join(", "),
            t.name
        ));

        self.indent_level += 1;
        let has_hidden = t.fields.iter().any(|f| is_hidden_field(&f.name));
        if ctor_pairs.is_empty() && !has_hidden {
            self.writeln(&format!("{}::default()", t.name));
        } else if has_hidden {
            // Assignment form: works with Drop types (no partial moves).
            self.writeln(&format!("let mut __s = {}::default();", t.name));
            for (name, expr) in &ctor_pairs {
                self.writeln(&format!("__s.{name} = {expr};"));
            }
            self.writeln("__s");
        } else {
            let inits: Vec<String> = ctor_pairs
                .iter()
                .map(|(name, expr)| format!("{name}: {expr}"))
                .collect();
            self.writeln(&format!("{} {{ {} }}", t.name, inits.join(", ")));
        }
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
        self.current_view = Some(view.type_name.clone());
        self.component_counters.clear();

        let (has_mount, has_update, has_unmount) = self.lifecycle_flags(&view.type_name);
        let n_effects = Self::count_view_effects(&view.root);
        let needs_mounted = has_mount || has_update || n_effects > 0;

        self.writeln(&format!("impl eframe::App for {} {{", view.type_name));
        self.indent_level += 1;
        self.writeln("fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {");
        self.indent_level += 1;

        self.pending_effects.clear();
        self.in_child = false;
        self.emit_jsx_element(&view.root, css, "ctx", "ui");
        self.emit_react_epilogue(&view.type_name, has_mount, has_update, needs_mounted);

        self.indent_level -= 1;
        self.writeln("}");
        self.indent_level -= 1;
        self.writeln("}");
        self.writeln("");

        // Child-component renderer: parents embed `<Tag/>` via this method so
        // child state (persisted in `self.__child_*` on the PARENT) is kept.
        self.writeln(&format!("impl {} {{", view.type_name));
        self.indent_level += 1;
        self.writeln("pub fn __render_as_child(&mut self, ui: &mut egui::Ui) {");
        self.indent_level += 1;
        self.pending_effects.clear();
        // Instance indices must restart: same walk, same numbering as update().
        self.component_counters.clear();
        let prev_child = self.in_child;
        self.in_child = true;
        self.emit_jsx_element(&view.root, css, "ctx", "ui");
        self.emit_react_epilogue(&view.type_name, has_mount, has_update, needs_mounted);
        self.in_child = prev_child;
        self.indent_level -= 1;
        self.writeln("}");
        self.indent_level -= 1;
        self.writeln("}");
        self.writeln("");

        if has_unmount {
            self.writeln(&format!("impl Drop for {} {{", view.type_name));
            self.indent_level += 1;
            self.writeln("fn drop(&mut self) {");
            self.indent_level += 1;
            self.writeln("self.onUnmount();");
            self.indent_level -= 1;
            self.writeln("}");
            self.indent_level -= 1;
            self.writeln("}");
            self.writeln("");
        }

        self.current_view = None;
    }

    /// Emit deferred effects + lifecycle calls at the end of `update()` or
    /// `__render_as_child()`. Effects run in tree order, then `onUpdate()`
    /// (or `onMount()` on the first frame), then the mount flag is set.
    fn emit_react_epilogue(
        &mut self,
        type_name: &str,
        has_mount: bool,
        has_update: bool,
        needs_mounted: bool,
    ) {
        let _ = type_name;
        let effects = std::mem::take(&mut self.pending_effects);
        for snippet in effects {
            self.writeln(&snippet);
        }
        if has_mount && has_update {
            self.writeln("if !self.__br_mounted { self.onMount(); } else { self.onUpdate(); }");
        } else if has_mount {
            self.writeln("if !self.__br_mounted { self.onMount(); }");
        } else if has_update {
            self.writeln("if self.__br_mounted { self.onUpdate(); }");
        }
        if needs_mounted {
            self.writeln("self.__br_mounted = true;");
        }
    }

    fn lifecycle_flags(&self, type_name: &str) -> (bool, bool, bool) {
        if let Some(t) = self.struct_defs.get(type_name) {
            let mut mount = false;
            let mut update = false;
            let mut unmount = false;
            for m in &t.methods {
                match m.name.as_str() {
                    "onMount" => mount = true,
                    "onUpdate" => update = true,
                    "onUnmount" => unmount = true,
                    _ => {}
                }
            }
            (mount, update, unmount)
        } else {
            (false, false, false)
        }
    }

    fn count_view_effects(root: &JsxElement) -> usize {
        let mut n = 0;
        Self::count_effects_in_children(&root.children, &mut n);
        n
    }

    fn count_effects_in_children(children: &[JsxChild], n: &mut usize) {
        for child in children {
            match child {
                JsxChild::Effect { .. } => {
                    *n += 1;
                }
                JsxChild::Element(e) => Self::count_effects_in_children(&e.children, n),
                JsxChild::If { then_child, else_child, .. } => {
                    Self::count_effects_in_child(then_child, n);
                    if let Some(ec) = else_child {
                        Self::count_effects_in_child(ec, n);
                    }
                }
                JsxChild::For { body, .. } => {
                    Self::count_effects_in_child(body, n);
                }
                _ => {}
            }
        }
    }

    fn count_effects_in_child(child: &JsxChild, n: &mut usize) {
        match child {
            JsxChild::Effect { .. } => {
                *n += 1;
            }
            JsxChild::Element(e) => Self::count_effects_in_children(&e.children, n),
            JsxChild::If { then_child, else_child, .. } => {
                Self::count_effects_in_child(then_child, n);
                if let Some(ec) = else_child {
                    Self::count_effects_in_child(ec, n);
                }
            }
            JsxChild::For { body, .. } => {
                Self::count_effects_in_child(body, n);
            }
            _ => {}
        }
    }

    fn emit_jsx_element(
        &mut self,
        elem: &JsxElement,
        css: &CssStyleSheet,
        ctx_var: &str,
        ui_var: &str,
    ) {
        let tag = elem.tag.as_str();

        // Extract class / binding / widget attributes
        let mut class_names: Vec<String> = Vec::new();
        let mut on_click_expr: Option<String> = None;
        let mut bind_var: Option<String> = None;
        let mut placeholder: Option<String> = None;
        let mut image_src: Option<String> = None;
        let mut image_src_expr: Option<String> = None;
        let mut progress_lit: Option<f32> = None;
        let mut progress_expr: Option<String> = None;
        let mut spacer_lit: Option<f32> = None;
        let mut spacer_expr: Option<String> = None;

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
                "src" | "source" | "path" | "image" | "url" | "file" => match &attr.value {
                    JsxAttrValue::Lit(s) => {
                        image_src = Some(s.clone());
                    }
                    JsxAttrValue::Expr(e) => {
                        image_src_expr = Some(self.expr_to_string(e));
                    }
                },
                "value" | "progress" | "percent" => match &attr.value {
                    JsxAttrValue::Lit(s) => {
                        if let Ok(v) = s.trim().trim_end_matches('%').parse::<f32>() {
                            let v = if s.trim().ends_with('%') { v / 100.0 } else { v };
                            progress_lit = Some(if v > 1.0 { v / 100.0 } else { v });
                        }
                    }
                    JsxAttrValue::Expr(e) => {
                        progress_expr = Some(self.expr_to_string(e));
                    }
                },
                "size" | "amount" | "space" => match &attr.value {
                    JsxAttrValue::Lit(s) => {
                        if let Ok(v) = s.trim().trim_end_matches("px").parse::<f32>() {
                            spacer_lit = Some(v);
                        }
                    }
                    JsxAttrValue::Expr(e) => {
                        spacer_expr = Some(self.expr_to_string(e));
                    }
                },
                _ => {}
            }
        }

        let class_refs: Vec<&str> = class_names.iter().map(|s| s.as_str()).collect();

        // React-style child component: `<Tag .../>` where Tag names a struct
        // with a view. Props are assigned, then the persisted child instance
        // (kept in `self.__child_<Tag>_<n>` by the analyzer) renders itself.
        if !is_builtin_brui_tag(tag)
            && self.struct_defs.contains_key(tag)
            && self.component_views.contains(tag)
        {
            self.emit_component_element(elem, css, ui_var, &class_refs);
            return;
        }

        match tag {
            "Panel" => {
                let frame_opt = css.get_frame_code(&class_refs);
                if self.in_child {
                    // Nested render: a second CentralPanel is illegal in egui,
                    // so child roots render as framed vertical containers.
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
                    return;
                }
                let frame_str = if let Some(fc) = frame_opt {
                    format!(".frame({fc})")
                } else {
                    String::new()
                };
                let max_w = css.get_max_width(&class_refs);
                let max_h = css.get_max_height(&class_refs);
                let opacity = css.get_opacity(&class_refs);
                self.writeln(&format!(
                    "egui::CentralPanel::default(){frame_str}.show({ctx_var}, |{ui_var}| {{"
                ));
                self.indent_level += 1;
                if let Some(w) = max_w {
                    self.writeln(&format!("{ui_var}.set_max_width({:.1});", w));
                }
                if let Some(h) = max_h {
                    self.writeln(&format!("{ui_var}.set_max_height({:.1});", h));
                }
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                    }
                }
                for child in &elem.children {
                    self.emit_jsx_child(child, css, ui_var);
                }
                self.indent_level -= 1;
                self.writeln("});");
            }
            "Col" | "Container" => {
                let frame_opt = css.get_frame_code(&class_refs);
                let gap_opt = css.get_spacing(&class_refs);
                let max_w = css.get_max_width(&class_refs);
                let max_h = css.get_max_height(&class_refs);
                let opacity = css.get_opacity(&class_refs);
                if let Some(fc) = &frame_opt {
                    self.writeln(&format!("{fc}.show({ui_var}, |{ui_var}| {{"));
                    self.indent_level += 1;
                }
                self.writeln(&format!("{ui_var}.vertical(|{ui_var}| {{"));
                self.indent_level += 1;
                if let Some(gap) = gap_opt {
                    self.writeln(&format!("{ui_var}.spacing_mut().item_spacing.y = {:.1};", gap));
                }
                if let Some(w) = max_w {
                    self.writeln(&format!("{ui_var}.set_max_width({:.1});", w));
                }
                if let Some(h) = max_h {
                    self.writeln(&format!("{ui_var}.set_max_height({:.1});", h));
                }
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                    }
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
                let max_w = css.get_max_width(&class_refs);
                let max_h = css.get_max_height(&class_refs);
                let opacity = css.get_opacity(&class_refs);
                if let Some(fc) = &frame_opt {
                    self.writeln(&format!("{fc}.show({ui_var}, |{ui_var}| {{"));
                    self.indent_level += 1;
                }
                self.writeln(&format!("{ui_var}.horizontal(|{ui_var}| {{"));
                self.indent_level += 1;
                if let Some(gap) = gap_opt {
                    self.writeln(&format!("{ui_var}.spacing_mut().item_spacing.x = {:.1};", gap));
                }
                if let Some(w) = max_w {
                    self.writeln(&format!("{ui_var}.set_max_width({:.1});", w));
                }
                if let Some(h) = max_h {
                    self.writeln(&format!("{ui_var}.set_max_height({:.1});", h));
                }
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                    }
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
            "Wrap" => {
                let frame_opt = css.get_frame_code(&class_refs);
                let gap_opt = css.get_spacing(&class_refs);
                let max_w = css.get_max_width(&class_refs);
                let opacity = css.get_opacity(&class_refs);
                if let Some(fc) = &frame_opt {
                    self.writeln(&format!("{fc}.show({ui_var}, |{ui_var}| {{"));
                    self.indent_level += 1;
                }
                self.writeln(&format!("{ui_var}.horizontal_wrapped(|{ui_var}| {{"));
                self.indent_level += 1;
                if let Some(gap) = gap_opt {
                    self.writeln(&format!("{ui_var}.spacing_mut().item_spacing.x = {:.1};", gap));
                    self.writeln(&format!("{ui_var}.spacing_mut().item_spacing.y = {:.1};", gap));
                }
                if let Some(w) = max_w {
                    self.writeln(&format!("{ui_var}.set_max_width({:.1});", w));
                }
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                    }
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
                let max_w = css.get_max_width(&class_refs);
                let opacity = css.get_opacity(&class_refs);
                self.writeln(&format!(
                    "egui::ScrollArea::vertical().max_height({:.1}).show({ui_var}, |{ui_var}| {{",
                    max_h
                ));
                self.indent_level += 1;
                if let Some(w) = max_w {
                    self.writeln(&format!("{ui_var}.set_max_width({:.1});", w));
                }
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                    }
                }
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
                let opacity = css.get_opacity(&class_refs);
                let line = format!(
                    "{ui_var}.label(egui::RichText::new({content}){mod_chain});"
                );
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.scope(|{ui_var}| {{"));
                        self.indent_level += 1;
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                        self.writeln(&line);
                        self.indent_level -= 1;
                        self.writeln("});");
                    } else {
                        self.writeln(&line);
                    }
                } else {
                    self.writeln(&line);
                }
            }
            "Button" => {
                let mods = css.get_rich_text_modifiers(&class_refs);
                let mod_chain = if mods.is_empty() {
                    String::new()
                } else {
                    format!(".{}", mods.join("."))
                };
                let content = self.jsx_children_to_rich_text_content(&elem.children);
                let opacity = css.get_opacity(&class_refs);
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.scope(|{ui_var}| {{"));
                        self.indent_level += 1;
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                    }
                }
                let action = on_click_expr.unwrap_or_default();
                self.writeln(&format!(
                    "if {ui_var}.add(egui::Button::new(egui::RichText::new({content}){mod_chain})).clicked() {{"
                ));
                self.indent_level += 1;
                if !action.is_empty() {
                    if let Some(for_var) = &self.jsx_for_var {
                        let mut deferred = action.replace("self.", "s.");
                        deferred = deferred.replace(&format!("{for_var}."), &format!("__{for_var}."));
                        self.writeln(&format!("let __{for_var} = __{for_var}_clone.clone();"));
                        self.writeln("__actions.push(Box::new(move |s: &mut Self| {");
                        self.indent_level += 1;
                        self.writeln(&format!("{deferred};"));
                        self.indent_level -= 1;
                        self.writeln("}));");
                    } else {
                        self.writeln(&format!("{action};"));
                    }
                }
                self.indent_level -= 1;
                self.writeln("}");
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.indent_level -= 1;
                        self.writeln("});");
                    }
                }
            }
            "Input" | "TextField" => {
                if let Some(var) = bind_var {
                    let hint = if let Some(ph) = placeholder {
                        format!(".hint_text(\"{}\")", ph.escape_default())
                    } else {
                        String::new()
                    };
                    let opacity = css.get_opacity(&class_refs);
                    let line = format!(
                        "{ui_var}.add(egui::TextEdit::singleline(&mut {var}){hint});"
                    );
                    if let Some(o) = opacity {
                        if o < 1.0 {
                            self.writeln(&format!("{ui_var}.scope(|{ui_var}| {{"));
                            self.indent_level += 1;
                            self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                            self.writeln(&line);
                            self.indent_level -= 1;
                            self.writeln("});");
                        } else {
                            self.writeln(&line);
                        }
                    } else {
                        self.writeln(&line);
                    }
                }
            }
            "Checkbox" => {
                if let Some(var) = bind_var {
                    let label_expr = if elem.children.is_empty() {
                        "\"\"".to_string()
                    } else {
                        self.jsx_children_to_rich_text_content(&elem.children)
                    };
                    let opacity = css.get_opacity(&class_refs);
                    let line = format!("{ui_var}.checkbox(&mut {var}, {label_expr});");
                    if let Some(o) = opacity {
                        if o < 1.0 {
                            self.writeln(&format!("{ui_var}.scope(|{ui_var}| {{"));
                            self.indent_level += 1;
                            self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                            self.writeln(&line);
                            self.indent_level -= 1;
                            self.writeln("});");
                        } else {
                            self.writeln(&line);
                        }
                    } else {
                        self.writeln(&line);
                    }
                }
            }
            "Slider" => {
                if let Some(var) = bind_var {
                    let opacity = css.get_opacity(&class_refs);
                    let line = format!("{ui_var}.add(egui::Slider::new(&mut {var}, 0..=100));");
                    if let Some(o) = opacity {
                        if o < 1.0 {
                            self.writeln(&format!("{ui_var}.scope(|{ui_var}| {{"));
                            self.indent_level += 1;
                            self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                            self.writeln(&line);
                            self.indent_level -= 1;
                            self.writeln("});");
                        } else {
                            self.writeln(&line);
                        }
                    } else {
                        self.writeln(&line);
                    }
                }
            }
            "Separator" => {
                self.writeln(&format!("{ui_var}.separator();"));
            }
            "Image" => {
                let mods = css.get_rich_text_modifiers(&class_refs);
                let mod_chain = if mods.is_empty() {
                    String::new()
                } else {
                    format!(".{}", mods.join("."))
                };
                let opacity = css.get_opacity(&class_refs);
                let line = if let Some(src_expr) = &image_src_expr {
                    format!(
                        "{ui_var}.label(egui::RichText::new(format!(\"[Image: {{}}]\", {src_expr})){mod_chain});"
                    )
                } else {
                    let label_text = if let Some(src) = &image_src {
                        format!("[Image: {}]", src)
                    } else if !elem.children.is_empty() {
                        let inner = self.jsx_children_to_rich_text_content(&elem.children);
                        let _ = inner;
                        "[Image]".to_string()
                    } else {
                        "[Image]".to_string()
                    };
                    format!(
                        "{ui_var}.label(egui::RichText::new(\"{}\"){mod_chain});",
                        label_text.escape_default()
                    )
                };
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.scope(|{ui_var}| {{"));
                        self.indent_level += 1;
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                        self.writeln(&line);
                        self.indent_level -= 1;
                        self.writeln("});");
                    } else {
                        self.writeln(&line);
                    }
                } else {
                    self.writeln(&line);
                }
            }
            "Progress" => {
                let opacity = css.get_opacity(&class_refs);
                let bar_expr = if let Some(var) = &bind_var {
                    format!(
                        "{{ let __p = ({var} as f32); let __p = if __p > 1.0 {{ __p / 100.0 }} else {{ __p }}; {ui_var}.add(egui::ProgressBar::new(__p).show_percentage()); }}"
                    )
                } else if let Some(pe) = &progress_expr {
                    format!(
                        "{{ let __p = ({pe} as f32); let __p = if __p > 1.0 {{ __p / 100.0 }} else {{ __p }}; {ui_var}.add(egui::ProgressBar::new(__p).show_percentage()); }}"
                    )
                } else {
                    let v = progress_lit.unwrap_or(0.5).clamp(0.0, 1.0);
                    format!("{ui_var}.add(egui::ProgressBar::new({v:.3}).show_percentage());")
                };
                if let Some(o) = opacity {
                    if o < 1.0 {
                        self.writeln(&format!("{ui_var}.scope(|{ui_var}| {{"));
                        self.indent_level += 1;
                        self.writeln(&format!("{ui_var}.set_opacity({:.3});", o));
                        self.writeln(&bar_expr);
                        self.indent_level -= 1;
                        self.writeln("});");
                    } else {
                        self.writeln(&bar_expr);
                    }
                } else {
                    self.writeln(&bar_expr);
                }
            }
            "Spacer" => {
                if let Some(se) = &spacer_expr {
                    self.writeln(&format!("{ui_var}.add_space(({se}) as f32);"));
                } else if let Some(v) = spacer_lit {
                    self.writeln(&format!("{ui_var}.add_space({:.1});", v));
                } else if let Some(w) = css.get_max_width(&class_refs).or_else(|| css.get_max_height(&class_refs)) {
                    self.writeln(&format!("{ui_var}.add_space({:.1});", w));
                } else {
                    self.writeln(&format!("{ui_var}.add_space(8.0);"));
                }
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

    /// Render a React-style child component: assign props to the persisted
    /// `self.__child_<Tag>_<n>` instance, then let it render itself inline.
    fn emit_component_element(
        &mut self,
        elem: &JsxElement,
        css: &CssStyleSheet,
        ui_var: &str,
        class_refs: &[&str],
    ) {
        let tag = elem.tag.as_str();
        let parent = self.current_view.clone().unwrap_or_default();
        let key = (parent, tag.to_string());
        let idx = *self.component_counters.get(&key).unwrap_or(&0);
        self.component_counters.insert(key, idx + 1);
        let field = child_field_name(tag, idx);

        const NON_PROPS: &[&str] = &[
            "class", "bind", "onClick", "placeholder", "name", "value", "progress",
            "percent", "size", "amount", "space", "src", "source", "path", "image",
            "url", "file",
        ];
        for attr in &elem.attributes {
            if NON_PROPS.contains(&attr.name.as_str()) {
                continue;
            }
            match &attr.value {
                JsxAttrValue::Expr(e) => {
                    let v = self.expr_to_string(e);
                    self.writeln(&format!("self.{field}.{} = {v};", attr.name));
                }
                JsxAttrValue::Lit(s) => {
                    let is_str = self
                        .struct_defs
                        .get(tag)
                        .and_then(|t| t.fields.iter().find(|f| f.name == attr.name))
                        .map(|f| f.ty == Some(TypeAnnotation::Str))
                        .unwrap_or(true);
                    if is_str {
                        self.writeln(&format!(
                            "self.{field}.{} = \"{}\".to_string();",
                            attr.name,
                            s.escape_default()
                        ));
                    } else {
                        self.writeln(&format!("self.{field}.{} = {};", attr.name, s));
                    }
                }
            }
        }

        let frame_opt = css.get_frame_code(class_refs);
        if let Some(fc) = frame_opt {
            self.writeln(&format!("{fc}.show({ui_var}, |{ui_var}| {{"));
            self.indent_level += 1;
            self.writeln(&format!("self.{field}.__render_as_child({ui_var});"));
            self.indent_level -= 1;
            self.writeln("});");
        } else {
            self.writeln(&format!("self.{field}.__render_as_child({ui_var});"));
        }
    }

    fn emit_jsx_child(&mut self, child: &JsxChild, css: &CssStyleSheet, ui_var: &str) {
        match child {
            JsxChild::Element(elem) => {
                self.emit_jsx_element(elem, css, "ctx", ui_var);
            }
            JsxChild::Effect { deps, body, .. } => {
                // Deferred to the end of update(): run on mount and whenever
                // the debug signature of `deps` changes (React parity).
                let idx = self.pending_effects.len();
                let dep_codes: Vec<String> =
                    deps.iter().map(|d| self.expr_to_string(d)).collect();
                let sig_tuple = if dep_codes.is_empty() {
                    "()".to_string()
                } else {
                    format!("({})", dep_codes.join(", "))
                };
                let body_code = self.expr_to_string(body);
                let field = fx_sig_field_name(idx);
                self.pending_effects.push(format!(
                    "{{ let __sig = format!(\"{{:?}}\", {sig_tuple}); if !self.__br_mounted || self.{field} != __sig {{ {body_code}; self.{field} = __sig; }} }}"
                ));
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
                let prev_for = self.jsx_for_var.clone();
                self.jsx_for_var = Some(item.clone());
                self.writeln("let mut __actions: Vec<Box<dyn FnOnce(&mut Self)>> = Vec::new();");
                self.writeln(&format!("for {item} in &mut {iter_str} {{"));
                self.indent_level += 1;
                self.writeln(&format!("let __{item}_clone = {item}.clone();"));
                self.emit_jsx_child(body, css, ui_var);
                self.indent_level -= 1;
                self.writeln("}");
                self.writeln("for act in __actions {");
                self.indent_level += 1;
                self.writeln("act(self);");
                self.indent_level -= 1;
                self.writeln("}");
                self.jsx_for_var = prev_for;
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
            self.writeln("let headless = std::env::var(\"BARA_HEADLESS\").is_ok() || std::env::args().any(|a| a == \"--headless\");");
            self.writeln("#[cfg(target_os = \"linux\")]");
            self.writeln("let headless = headless || (std::env::var(\"DISPLAY\").is_err() && std::env::var(\"WAYLAND_DISPLAY\").is_err());");
            self.writeln(&format!("if headless {{ println!(\"Bara GUI '{{}}': headless mode, skipping window.\", \"{}\"); return Ok(()); }}", app_title.escape_default()));
            self.writeln("let mut native_options = eframe::NativeOptions::default();");
            self.writeln(&format!(
                "native_options.viewport = egui::ViewportBuilder::default().with_inner_size([{:.1}, {:.1}]);",
                app_width, app_height
            ));
            self.writeln(&format!(
                "eframe::run_native(\"{}\", native_options, Box::new(|_cc| Box::new({target_app}::default())))",
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
            Stmt::Match { expr, arms, span } => {
                self.writeln(&format!("// @line: {}", span.0));
                let target_code = self.expr_to_string(expr);
                self.writeln(&format!("match {target_code} {{"));
                self.indent_level += 1;
                for arm in arms {
                    let pat_code = self.expr_to_string(&arm.pattern);
                    let guard_str = if let Some(g) = &arm.guard {
                        format!(" if {}", self.expr_to_string(g))
                    } else {
                        String::new()
                    };
                    let body_code = self.expr_to_string(&arm.body);
                    if matches!(arm.body, Expr::Block(_, _)) {
                        self.writeln(&format!("{pat_code}{guard_str} => {body_code}"));
                    } else {
                        self.writeln(&format!("{pat_code}{guard_str} => {body_code},"));
                    }
                }
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
                    if name == "print" || name == "println" {
                        return self.emit_print(args);
                    }
                    if let Some(t) = self.struct_defs.get(name).cloned() {
                        return self.emit_struct_instantiation(&t, args);
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
            Expr::Tuple(elements, _) => {
                let el_strs: Vec<String> = elements.iter().map(|e| self.expr_to_string(e)).collect();
                if el_strs.len() == 1 {
                    format!("({},)", el_strs[0])
                } else {
                    format!("({})", el_strs.join(", "))
                }
            }
            Expr::Match { expr, arms, .. } => {
                let target_code = self.expr_to_string(expr);
                let mut arm_strs = Vec::new();
                for arm in arms {
                    let pat_code = self.expr_to_string(&arm.pattern);
                    let guard_str = if let Some(g) = &arm.guard {
                        format!(" if {}", self.expr_to_string(g))
                    } else {
                        String::new()
                    };
                    let body_code = self.expr_to_string(&arm.body);
                    arm_strs.push(format!("{pat_code}{guard_str} => {body_code}"));
                }
                format!("match {target_code} {{\n{}\n}}", arm_strs.join(",\n"))
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

    fn emit_struct_instantiation(&self, t: &TypeDef, args: &[Expr]) -> String {
        if args.is_empty() {
            return format!("{}::default()", t.name);
        }
        let mut field_pairs: Vec<(String, String)> = Vec::new();
        for (idx, arg) in args.iter().enumerate() {
            match arg {
                Expr::Assign { target, value, .. } => {
                    let field_name_opt = match target.as_ref() {
                        Expr::Ident(n, _) => Some(n.clone()),
                        Expr::SelfAccess(n, _) => Some(n.clone()),
                        _ => None,
                    };
                    if let Some(field_name) = field_name_opt {
                        let val_str = self.call_arg_to_string(value);
                        let is_str = t
                            .fields
                            .iter()
                            .any(|f| f.name == field_name && f.ty == Some(TypeAnnotation::Str));
                        let final_val = if is_str
                            && !val_str.ends_with(".to_string()")
                            && !val_str.ends_with(".into()")
                        {
                            format!("{val_str}.into()")
                        } else {
                            val_str
                        };
                        field_pairs.push((field_name, final_val));
                    }
                }
                _ => {
                    if idx < t.fields.len() {
                        let field_name = &t.fields[idx].name;
                        let val_str = self.call_arg_to_string(arg);
                        let is_str = t.fields[idx].ty == Some(TypeAnnotation::Str);
                        let final_val = if is_str
                            && !val_str.ends_with(".to_string()")
                            && !val_str.ends_with(".into()")
                        {
                            format!("{val_str}.into()")
                        } else {
                            val_str
                        };
                        field_pairs.push((field_name.clone(), final_val));
                    }
                }
            }
        }

        if field_pairs.is_empty() {
            return format!("{}::default()", t.name);
        }
        let has_drop = t.methods.iter().any(|m| m.name == "onUnmount");
        if field_pairs.len() == t.fields.len() {
            let assigns: Vec<String> = field_pairs
                .iter()
                .map(|(n, v)| format!("{n}: {v}"))
                .collect();
            return format!("{} {{ {} }}", t.name, assigns.join(", "));
        }
        if has_drop {
            // Assignment form: struct-update syntax cannot move fields out of
            // a Drop value, so build via assignment instead.
            let mut stmts = vec![format!("let mut __s = {}::default();", t.name)];
            for (n, v) in &field_pairs {
                stmts.push(format!("__s.{n} = {v};"));
            }
            stmts.push("__s".to_string());
            return format!("{{ {} }}", stmts.join(" "));
        }
        let assigns: Vec<String> = field_pairs
            .iter()
            .map(|(n, v)| format!("{n}: {v}"))
            .collect();
        format!(
            "{} {{ {}, ..Default::default() }}",
            t.name,
            assigns.join(", ")
        )
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
