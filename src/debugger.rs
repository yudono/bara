use std::collections::HashSet;
use crate::ast::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub severity: Severity,
    pub span: (usize, usize),
    pub file: Option<String>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: (usize, usize)) -> Self {
        Self {
            code,
            message: message.into(),
            severity: Severity::Error,
            span,
            file: None,
            suggestion: None,
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>, span: (usize, usize)) -> Self {
        Self {
            code,
            message: message.into(),
            severity: Severity::Warning,
            span,
            file: None,
            suggestion: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Render a rustc-style diagnostic output with line excerpts and caret indicators
    pub fn render(&self, source_text: &str) -> String {
        let (line_num, col_num) = self.span;
        let sev_str = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };

        let file_str = self.file.as_deref().unwrap_or("source");
        let mut out = String::new();
        out.push_str(&format!(
            "{sev_str}[{}]: {}\n",
            self.code, self.message
        ));
        out.push_str(&format!(
            "  --> {}:{}:{}\n",
            file_str, line_num, col_num
        ));

        let lines: Vec<&str> = source_text.lines().collect();
        if line_num > 0 && line_num <= lines.len() {
            let line_content = lines[line_num - 1];
            let line_prefix = format!("{:4} | ", line_num);
            out.push_str("   |\n");
            out.push_str(&line_prefix);
            out.push_str(line_content);
            out.push('\n');

            let pad = " ".repeat(line_prefix.len() + col_num.saturating_sub(1));
            let caret = match self.severity {
                Severity::Error => "^",
                Severity::Warning => "^",
                Severity::Info => "-",
            };
            out.push_str(&format!("{pad}{caret} {}\n", self.message));
        }

        if let Some(ref sugg) = self.suggestion {
            out.push_str(&format!("   = help: {sugg}\n"));
        }

        out
    }
}

pub struct BaraDebugger {
    pub diagnostics: Vec<Diagnostic>,
    pub known_types: HashSet<String>,
    pub known_globals: HashSet<String>,
}

impl BaraDebugger {
    pub fn new() -> Self {
        let mut known_globals = HashSet::new();
        // Standard built-in Bara symbols & functions
        let builtins = [
            "print", "println", "len", "range", "abs", "min", "max",
            "true", "false", "None", "self", "brui", "app", "ctx", "ui",
            "int", "float", "str", "bool", "dyn"
        ];
        for b in builtins {
            known_globals.insert(b.to_string());
        }

        Self {
            diagnostics: Vec::new(),
            known_types: HashSet::new(),
            known_globals,
        }
    }

    pub fn inspect(&mut self, program: &Program, file_name: Option<&str>) {
        // Collect types and top-level function names
        let mut type_names = HashSet::new();
        for item in &program.items {
            match item {
                Item::Type(t) => {
                    if !type_names.insert(t.name.clone()) {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0002",
                                format!("Type '{}' is defined multiple times", t.name),
                                t.span,
                            )
                            .with_suggestion("Remove or rename the duplicate type definition"),
                        );
                    }
                    self.known_types.insert(t.name.clone());
                    self.known_globals.insert(t.name.clone());
                }
                Item::Fn(f) => {
                    self.known_globals.insert(f.name.clone());
                }
                _ => {}
            }
        }

        // Validate types and their methods
        let mut top_level_scope = self.known_globals.clone();
        for item in &program.items {
            match item {
                Item::Type(t) => {
                    self.check_type_def(t);
                }
                Item::Fn(f) => {
                    self.check_function(f, &HashSet::new());
                }
                Item::View(v) => {
                    self.check_view_def(v);
                }
                Item::TopLevelStmt(s) => {
                    self.check_stmt(s, &mut top_level_scope, &HashSet::new());
                }
                Item::Use(u) => {
                    // Stylesheet imports are not Rust modules: do not pollute scope.
                    if u.path.iter().any(|s| s.ends_with(".css") || s == "css")
                        || (u.path.len() == 1 && (u.path[0] == "styles" || u.path[0] == "style"))
                    {
                        continue;
                    }
                    if let Some(alias) = &u.alias {
                        self.known_globals.insert(alias.clone());
                        top_level_scope.insert(alias.clone());
                    } else if let Some(last) = u.path.last() {
                        self.known_globals.insert(last.clone());
                        top_level_scope.insert(last.clone());
                    }
                }
                _ => {}
            }
        }

        if let Some(fname) = file_name {
            for d in &mut self.diagnostics {
                if d.file.is_none() {
                    d.file = Some(fname.to_string());
                }
            }
        }
    }

    fn check_type_def(&mut self, t: &TypeDef) {
        let mut field_names = HashSet::new();
        for f in &t.fields {
            if !field_names.insert(f.name.clone()) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0003",
                        format!("Field '{}' is defined multiple times in type '{}'", f.name, t.name),
                        f.span,
                    )
                    .with_suggestion("Ensure field names in a type are unique"),
                );
            }
        }

        let mut method_names = HashSet::new();
        for m in &t.methods {
            if !method_names.insert(m.name.clone()) {
                self.diagnostics.push(
                    Diagnostic::error(
                        "E0003",
                        format!("Method '{}' is defined multiple times in type '{}'", m.name, t.name),
                        m.span,
                    )
                    .with_suggestion("Ensure method names in a type are unique"),
                );
            }
            self.check_function(m, &field_names);
        }
    }

    fn check_function(&mut self, f: &FnDef, struct_fields: &HashSet<String>) {
        let mut scope = self.known_globals.clone();
        for p in &f.params {
            scope.insert(p.name.clone());
        }

        self.check_block(&f.body, &mut scope, struct_fields);
    }

    fn check_block(&mut self, block: &Block, scope: &mut HashSet<String>, struct_fields: &HashSet<String>) {
        let mut terminated = false;
        let mut dead_span = None;

        for s in &block.stmts {
            if terminated {
                if dead_span.is_none() {
                    dead_span = Some(s.span());
                }
            }

            self.check_stmt(s, scope, struct_fields);

            match s {
                Stmt::Return(_, _) | Stmt::Break(_) | Stmt::Continue(_) => {
                    terminated = true;
                }
                _ => {}
            }
        }

        if let Some(span) = dead_span {
            self.diagnostics.push(
                Diagnostic::warning(
                    "W0003",
                    "Unreachable code detected after terminating statement",
                    span,
                )
                .with_suggestion("Remove dead statements following return, break, or continue"),
            );
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt, scope: &mut HashSet<String>, struct_fields: &HashSet<String>) {
        match stmt {
            Stmt::Assign { target, value, .. } => {
                self.check_expr(value, scope, struct_fields);
                if let Expr::Ident(name, _) = target {
                    scope.insert(name.clone());
                } else {
                    self.check_expr(target, scope, struct_fields);
                }
            }
            Stmt::Expr(e) => {
                self.check_expr(e, scope, struct_fields);
            }
            Stmt::If { cond, then_branch, elif_branches, else_branch, .. } => {
                self.check_expr(cond, scope, struct_fields);
                let mut then_scope = scope.clone();
                self.check_block(then_branch, &mut then_scope, struct_fields);

                for (el_cond, el_body) in elif_branches {
                    self.check_expr(el_cond, scope, struct_fields);
                    let mut el_scope = scope.clone();
                    self.check_block(el_body, &mut el_scope, struct_fields);
                }

                if let Some(eb) = else_branch {
                    let mut eb_scope = scope.clone();
                    self.check_block(eb, &mut eb_scope, struct_fields);
                }
            }
            Stmt::While { cond, body, span } => {
                self.check_expr(cond, scope, struct_fields);

                // Detect potential infinite loop: while cond where cond vars are never mutated and no break/return
                let mut cond_vars = HashSet::new();
                self.collect_idents(cond, &mut cond_vars);

                let mut body_mutations = HashSet::new();
                self.collect_mutations_in_block(body, &mut body_mutations);
                let has_terminator = self.block_has_terminator(body);

                let mut body_scope = scope.clone();
                self.check_block(body, &mut body_scope, struct_fields);

                if !cond_vars.is_empty()
                    && cond_vars.iter().all(|v| !body_mutations.contains(v))
                    && !has_terminator
                {
                    self.diagnostics.push(
                        Diagnostic::warning(
                            "W0004",
                            "While loop condition variables are never updated in the loop body",
                            *span,
                        )
                        .with_suggestion("Ensure loop condition variables are modified or include a break statement"),
                    );
                }
            }
            Stmt::For { item, iter, body, .. } => {
                self.check_expr(iter, scope, struct_fields);
                let mut for_scope = scope.clone();
                for_scope.insert(item.clone());
                self.check_block(body, &mut for_scope, struct_fields);
            }
            Stmt::Return(Some(val), _) => {
                self.check_expr(val, scope, struct_fields);
            }
            _ => {}
        }
    }

    fn check_expr(&mut self, expr: &Expr, scope: &HashSet<String>, struct_fields: &HashSet<String>) {
        match expr {
            Expr::Ident(name, span) => {
                if !scope.contains(name) && !struct_fields.contains(name) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0001",
                            format!("Undeclared variable or identifier '{name}'"),
                            *span,
                        )
                        .with_suggestion(format!("Define '{name}' before using it")),
                    );
                }
            }
            Expr::Binary { op, left, right, span } => {
                self.check_expr(left, scope, struct_fields);
                self.check_expr(right, scope, struct_fields);

                // Division by zero check
                if matches!(op, BinaryOp::Div | BinaryOp::Mod) {
                    if let Expr::Literal(Literal::Int(0), _) = right.as_ref() {
                        self.diagnostics.push(
                            Diagnostic::warning(
                                "W0002",
                                "Division or modulo by literal zero",
                                *span,
                            )
                            .with_suggestion("Avoid dividing by zero"),
                        );
                    }
                }
            }
            Expr::Unary { expr: inner, .. } => {
                self.check_expr(inner, scope, struct_fields);
            }
            Expr::Ternary { cond, then_expr, else_expr, .. } => {
                self.check_expr(cond, scope, struct_fields);
                self.check_expr(then_expr, scope, struct_fields);
                self.check_expr(else_expr, scope, struct_fields);
            }
            Expr::Call { callee, args, .. } => {
                self.check_expr(callee, scope, struct_fields);
                for a in args {
                    self.check_expr(a, scope, struct_fields);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                self.check_expr(receiver, scope, struct_fields);
                for a in args {
                    self.check_expr(a, scope, struct_fields);
                }
            }
            Expr::MemberAccess { object, .. } => {
                self.check_expr(object, scope, struct_fields);
            }
            Expr::Index { collection, index, .. } => {
                self.check_expr(collection, scope, struct_fields);
                self.check_expr(index, scope, struct_fields);
            }
            Expr::List(elems, _) => {
                for e in elems {
                    self.check_expr(e, scope, struct_fields);
                }
            }
            Expr::Map(pairs, _) => {
                for (k, v) in pairs {
                    self.check_expr(k, scope, struct_fields);
                    self.check_expr(v, scope, struct_fields);
                }
            }
            Expr::Closure { params, body, .. } => {
                let mut c_scope = scope.clone();
                for p in params {
                    c_scope.insert(p.clone());
                }
                self.check_expr(body, &c_scope, struct_fields);
            }
            Expr::Assign { target, value, .. } => {
                if !matches!(target.as_ref(), Expr::Ident(_, _)) {
                    self.check_expr(target, scope, struct_fields);
                }
                self.check_expr(value, scope, struct_fields);
            }
            Expr::Pipe { left, right, .. } => {
                self.check_expr(left, scope, struct_fields);
                self.check_expr(right, scope, struct_fields);
            }
            Expr::Try(inner, _)
            | Expr::Go(inner, _)
            | Expr::Clone(inner, _)
            | Expr::Borrow(inner, _, _)
            | Expr::Cast { expr: inner, .. } => {
                self.check_expr(inner, scope, struct_fields);
            }
            _ => {}
        }
    }

    fn check_view_def(&mut self, v: &ViewDef) {
        if !self.known_types.contains(&v.type_name) {
            self.diagnostics.push(
                Diagnostic::error(
                    "E0001",
                    format!("View binds to unknown type '{}'", v.type_name),
                    v.span,
                )
                .with_suggestion(format!("Declare 'type {} = ...' before defining its view", v.type_name)),
            );
        }

        self.check_jsx_element(&v.root);
    }

    fn check_jsx_element(&mut self, el: &JsxElement) {
        let valid_brui_tags = [
            "Panel", "Col", "Row", "Scroll", "Text", "Button",
            "Input", "TextField", "Checkbox", "Slider", "Separator",
            "Wrap", "Container", "Image", "Progress", "Spacer",
        ];

        if !valid_brui_tags.contains(&el.tag.as_str()) {
            self.diagnostics.push(
                Diagnostic::warning(
                    "E0003",
                    format!("Unknown Brui UI component '<{}>'", el.tag),
                    el.span,
                )
                .with_suggestion(format!(
                    "Valid Brui UI tags are: {}",
                    valid_brui_tags.join(", ")
                )),
            );
        }

        for child in &el.children {
            self.check_jsx_child(child);
        }
    }

    fn check_jsx_child(&mut self, child: &JsxChild) {
        match child {
            JsxChild::Element(e) => self.check_jsx_element(e),
            JsxChild::If { then_child, else_child, .. } => {
                self.check_jsx_child(then_child);
                if let Some(ec) = else_child {
                    self.check_jsx_child(ec);
                }
            }
            JsxChild::For { body, .. } => {
                self.check_jsx_child(body);
            }
            _ => {}
        }
    }

    fn collect_idents(&self, expr: &Expr, idents: &mut HashSet<String>) {
        match expr {
            Expr::Ident(name, _) => {
                idents.insert(name.clone());
            }
            Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
                self.collect_idents(left, idents);
                self.collect_idents(right, idents);
            }
            Expr::Unary { expr: inner, .. }
            | Expr::Try(inner, _)
            | Expr::Go(inner, _)
            | Expr::Clone(inner, _)
            | Expr::Borrow(inner, _, _)
            | Expr::Cast { expr: inner, .. } => {
                self.collect_idents(inner, idents);
            }
            Expr::Call { callee, args, .. } => {
                self.collect_idents(callee, idents);
                for a in args {
                    self.collect_idents(a, idents);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                self.collect_idents(receiver, idents);
                for a in args {
                    self.collect_idents(a, idents);
                }
            }
            Expr::Ternary { cond, then_expr, else_expr, .. } => {
                self.collect_idents(cond, idents);
                self.collect_idents(then_expr, idents);
                self.collect_idents(else_expr, idents);
            }
            _ => {}
        }
    }

    fn collect_mutations_in_block(&self, block: &Block, mutations: &mut HashSet<String>) {
        for s in &block.stmts {
            match s {
                Stmt::Assign { target, .. } => {
                    if let Expr::Ident(name, _) = target {
                        mutations.insert(name.clone());
                    }
                }
                Stmt::If { then_branch, elif_branches, else_branch, .. } => {
                    self.collect_mutations_in_block(then_branch, mutations);
                    for (_, b) in elif_branches {
                        self.collect_mutations_in_block(b, mutations);
                    }
                    if let Some(eb) = else_branch {
                        self.collect_mutations_in_block(eb, mutations);
                    }
                }
                Stmt::While { body, .. } | Stmt::For { body, .. } => {
                    self.collect_mutations_in_block(body, mutations);
                }
                _ => {}
            }
        }
    }

    fn block_has_terminator(&self, block: &Block) -> bool {
        for s in &block.stmts {
            match s {
                Stmt::Break(_) | Stmt::Return(_, _) => return true,
                Stmt::If { then_branch, elif_branches, else_branch, .. } => {
                    if self.block_has_terminator(then_branch) {
                        return true;
                    }
                    for (_, b) in elif_branches {
                        if self.block_has_terminator(b) {
                            return true;
                        }
                    }
                    if let Some(eb) = else_branch {
                        if self.block_has_terminator(eb) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
}
