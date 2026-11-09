use std::collections::{HashMap, HashSet};
use crate::ast::*;

#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub known_crates: HashSet<String>,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        let mut known_crates = HashSet::new();
        known_crates.insert("std".to_string());
        known_crates.insert("reqwest".to_string());
        known_crates.insert("tokio".to_string());
        known_crates.insert("serde".to_string());
        known_crates.insert("serde_json".to_string());
        known_crates.insert("fs".to_string());
        known_crates.insert("env".to_string());
        known_crates.insert("io".to_string());
        known_crates.insert("collections".to_string());
        Self { known_crates }
    }
}

pub struct Analyzer {
    pub config: AnalyzerConfig,
    pub structs: HashMap<String, TypeDef>,
    pub enums: HashMap<String, EnumDef>,
    pub functions: HashMap<String, FnDef>,
    pub needs_bara_dyn: bool,
    pub main_is_async: bool,
    pub main_has_try: bool,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            config: AnalyzerConfig::default(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            functions: HashMap::new(),
            needs_bara_dyn: false,
            main_is_async: false,
            main_has_try: false,
        }
    }

    pub fn analyze_program(&mut self, program: &mut Program) -> Result<(), String> {
        // Collect imports into known_crates (only external crates with path > 1)
        for item in &program.items {
            if let Item::Use(u) = item {
                if u.path.len() > 1 {
                    if let Some(first) = u.path.first() {
                        self.config.known_crates.insert(first.clone());
                    }
                }
            }
        }

        // Pass 1A: Collect struct & enum definitions & infer field types
        for item in &mut program.items {
            match item {
                Item::Type(t) => {
                    for f in &mut t.fields {
                        if f.ty.is_none() {
                            if let Some(def_val) = &f.default_val {
                                f.ty = Some(self.infer_expr_type(def_val));
                            } else {
                                f.ty = Some(TypeAnnotation::Str);
                            }
                        }
                    }
                    self.structs.insert(t.name.clone(), t.clone());
                }
                Item::Enum(e) => {
                    self.enums.insert(e.name.clone(), e.clone());
                }
                _ => {}
            }
        }

        // Pass 1B: Collect standalone function signatures
        for item in &program.items {
            if let Item::Fn(f) = item {
                self.functions.insert(f.name.clone(), f.clone());
            }
        }

        // Pass 2: Analyze methods within structs (self. field resolution, mutability, receiver)
        for item in &mut program.items {
            if let Item::Type(t) = item {
                let field_names: HashSet<String> = t.fields.iter().map(|f| f.name.clone()).collect();
                let method_names: HashSet<String> = t.methods.iter().map(|m| m.name.clone()).collect();
                for m in &mut t.methods {
                    self.analyze_method(m, &field_names, &method_names)?;
                }
                // Update stored struct definition
                self.structs.insert(t.name.clone(), t.clone());
            }
        }

        // Pass 3: Analyze standalone functions (parameter borrowing, mutability, return Ok)
        for item in &mut program.items {
            if let Item::Fn(f) = item {
                self.analyze_function(f)?;
                self.functions.insert(f.name.clone(), f.clone());
            }
        }

        // Pass 3.5: Analyze Views (resolve fields in JSX to self.field)
        for item in &mut program.items {
            if let Item::View(v) = item {
                let field_names = if let Some(t) = self.structs.get(&v.type_name) {
                    let mut s: HashSet<String> = t.fields.iter().map(|f| f.name.clone()).collect();
                    for m in &t.methods {
                        s.insert(m.name.clone());
                    }
                    s
                } else {
                    HashSet::new()
                };
                self.resolve_fields_in_jsx_element(&mut v.root, &field_names, &HashSet::new());
            }
        }

        // Pass 4: Analyze top-level statements (main function body)
        let mut top_stmts = Vec::new();
        for item in &program.items {
            if let Item::TopLevelStmt(s) = item {
                top_stmts.push(s.clone());
            }
        }

        if !top_stmts.is_empty() {
            let mut main_block = Block {
                stmts: top_stmts,
                span: (1, 1),
            };
            self.analyze_block(&mut main_block, &HashSet::new(), true)?;

            // Replace top-level statements with analyzed ones
            let mut new_items = Vec::new();
            let mut main_idx = 0;
            for item in &program.items {
                match item {
                    Item::TopLevelStmt(_) => {
                        if main_idx < main_block.stmts.len() {
                            new_items.push(Item::TopLevelStmt(main_block.stmts[main_idx].clone()));
                            main_idx += 1;
                        }
                    }
                    other => new_items.push(other.clone()),
                }
            }
            program.items = new_items;
        }

        Ok(())
    }

    fn analyze_method(
        &mut self,
        m: &mut FnDef,
        field_names: &HashSet<String>,
        method_names: &HashSet<String>,
    ) -> Result<(), String> {
        let mut local_vars = HashSet::new();
        for p in &m.params {
            local_vars.insert(p.name.clone());
        }

        // Check whether body accesses or mutates any field
        let mut field_accessed = false;
        let mut field_mutated = false;

        self.resolve_fields_in_block(
            &mut m.body,
            field_names,
            method_names,
            &mut local_vars,
            &mut field_accessed,
            &mut field_mutated,
        );

        // Determine method receiver kind: methods on a struct default to RefSelf unless mutated
        if field_mutated {
            m.receiver = ReceiverKind::MutSelf;
        } else {
            m.receiver = ReceiverKind::RefSelf;
        }

        // Now run general function analysis on this method
        self.analyze_function_common(m)?;

        Ok(())
    }

    fn resolve_fields_in_block(
        &self,
        block: &mut Block,
        field_names: &HashSet<String>,
        method_names: &HashSet<String>,
        local_vars: &mut HashSet<String>,
        field_accessed: &mut bool,
        field_mutated: &mut bool,
    ) {
        for stmt in &mut block.stmts {
            match stmt {
                Stmt::Assign { target, value, op, .. } => {
                    // Check if target is a field name
                    if let Expr::Ident(name, span) = target {
                        if field_names.contains(name) && !local_vars.contains(name) {
                            *field_accessed = true;
                            *field_mutated = true;
                            *target = Expr::SelfAccess(name.clone(), *span);
                        } else {
                            local_vars.insert(name.clone());
                        }
                    } else {
                        let mut target_field_accessed = false;
                        self.resolve_fields_in_expr(target, field_names, method_names, local_vars, &mut target_field_accessed, field_mutated);
                        if target_field_accessed {
                            *field_accessed = true;
                            *field_mutated = true;
                        }
                    }
                    if op.is_some() {
                        *field_mutated = true;
                    }
                    self.resolve_fields_in_expr(value, field_names, method_names, local_vars, field_accessed, field_mutated);
                }
                Stmt::Expr(e) => {
                    self.resolve_fields_in_expr(e, field_names, method_names, local_vars, field_accessed, field_mutated);
                }
                Stmt::Return(opt_e, _) => {
                    if let Some(e) = opt_e {
                        self.resolve_fields_in_expr(e, field_names, method_names, local_vars, field_accessed, field_mutated);
                    }
                }
                Stmt::If { cond, then_branch, elif_branches, else_branch, .. } => {
                    self.resolve_fields_in_expr(cond, field_names, method_names, local_vars, field_accessed, field_mutated);
                    let mut then_locals = local_vars.clone();
                    self.resolve_fields_in_block(then_branch, field_names, method_names, &mut then_locals, field_accessed, field_mutated);
                    for (c, b) in elif_branches {
                        self.resolve_fields_in_expr(c, field_names, method_names, local_vars, field_accessed, field_mutated);
                        let mut elif_locals = local_vars.clone();
                        self.resolve_fields_in_block(b, field_names, method_names, &mut elif_locals, field_accessed, field_mutated);
                    }
                    if let Some(eb) = else_branch {
                        let mut else_locals = local_vars.clone();
                        self.resolve_fields_in_block(eb, field_names, method_names, &mut else_locals, field_accessed, field_mutated);
                    }
                }
                Stmt::For { item, iter, body, .. } => {
                    self.resolve_fields_in_expr(iter, field_names, method_names, local_vars, field_accessed, field_mutated);
                    let mut for_locals = local_vars.clone();
                    for_locals.insert(item.clone());
                    self.resolve_fields_in_block(body, field_names, method_names, &mut for_locals, field_accessed, field_mutated);
                }
                Stmt::While { cond, body, .. } => {
                    self.resolve_fields_in_expr(cond, field_names, method_names, local_vars, field_accessed, field_mutated);
                    let mut while_locals = local_vars.clone();
                    self.resolve_fields_in_block(body, field_names, method_names, &mut while_locals, field_accessed, field_mutated);
                }
                Stmt::Match { expr, arms, .. } => {
                    self.resolve_fields_in_expr(expr, field_names, method_names, local_vars, field_accessed, field_mutated);
                    for arm in arms {
                        let arm_locals = local_vars.clone();
                        self.resolve_fields_in_expr(&mut arm.pattern, field_names, method_names, &arm_locals, field_accessed, field_mutated);
                        if let Some(guard) = &mut arm.guard {
                            self.resolve_fields_in_expr(guard, field_names, method_names, &arm_locals, field_accessed, field_mutated);
                        }
                        self.resolve_fields_in_expr(&mut arm.body, field_names, method_names, &arm_locals, field_accessed, field_mutated);
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_fields_in_expr(
        &self,
        expr: &mut Expr,
        field_names: &HashSet<String>,
        method_names: &HashSet<String>,
        local_vars: &HashSet<String>,
        field_accessed: &mut bool,
        field_mutated: &mut bool,
    ) {
        match expr {
            Expr::Literal(Literal::InterpolatedStr(parts), _) => {
                for p in parts {
                    if let StringPart::Expr(e) = p {
                        self.resolve_fields_in_expr(e, field_names, method_names, local_vars, field_accessed, field_mutated);
                    }
                }
            }
            Expr::Ident(name, span) => {
                if field_names.contains(name) && !local_vars.contains(name) {
                    *field_accessed = true;
                    *expr = Expr::SelfAccess(name.clone(), *span);
                }
            }
            Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
                self.resolve_fields_in_expr(left, field_names, method_names, local_vars, field_accessed, field_mutated);
                self.resolve_fields_in_expr(right, field_names, method_names, local_vars, field_accessed, field_mutated);
            }
            Expr::Ternary { cond, then_expr, else_expr, .. } => {
                self.resolve_fields_in_expr(cond, field_names, method_names, local_vars, field_accessed, field_mutated);
                self.resolve_fields_in_expr(then_expr, field_names, method_names, local_vars, field_accessed, field_mutated);
                self.resolve_fields_in_expr(else_expr, field_names, method_names, local_vars, field_accessed, field_mutated);
            }
            Expr::Assign { target, value, .. } => {
                self.resolve_fields_in_expr(target, field_names, method_names, local_vars, field_accessed, field_mutated);
                self.resolve_fields_in_expr(value, field_names, method_names, local_vars, field_accessed, field_mutated);
            }
            Expr::Closure { params, body, .. } => {
                let mut closure_locals = local_vars.clone();
                for p in params {
                    closure_locals.insert(p.clone());
                }
                self.resolve_fields_in_expr(body, field_names, method_names, &closure_locals, field_accessed, field_mutated);
            }
            Expr::Unary { expr: inner, .. }
            | Expr::Try(inner, _)
            | Expr::Go(inner, _)
            | Expr::Clone(inner, _)
            | Expr::Borrow(inner, _, _)
            | Expr::Cast { expr: inner, .. }
            | Expr::DynWrap { expr: inner, .. } => {
                self.resolve_fields_in_expr(inner, field_names, method_names, local_vars, field_accessed, field_mutated);
            }
            Expr::Call { callee, args, span } => {
                if let Expr::Ident(name, s) = callee.as_ref() {
                    if method_names.contains(name) && !local_vars.contains(name) {
                        *field_accessed = true;
                        let self_expr = Box::new(Expr::SelfAccess("self".to_string(), *s));
                        let mut new_args = std::mem::take(args);
                        for a in &mut new_args {
                            self.resolve_fields_in_expr(a, field_names, method_names, local_vars, field_accessed, field_mutated);
                        }
                        *expr = Expr::MethodCall {
                            receiver: self_expr,
                            method: name.clone(),
                            args: new_args,
                            span: *span,
                        };
                        return;
                    }
                }
                self.resolve_fields_in_expr(callee, field_names, method_names, local_vars, field_accessed, field_mutated);
                for a in args {
                    self.resolve_fields_in_expr(a, field_names, method_names, local_vars, field_accessed, field_mutated);
                }
            }
            Expr::MethodCall { receiver, args, method, .. } => {
                // If receiver is a field, mutating methods like .push() mutate the field
                if let Expr::Ident(name, _) = receiver.as_ref() {
                    if field_names.contains(name) && !local_vars.contains(name) {
                        if is_mutating_method(method) {
                            *field_mutated = true;
                        }
                    }
                }
                self.resolve_fields_in_expr(receiver, field_names, method_names, local_vars, field_accessed, field_mutated);
                for a in args {
                    self.resolve_fields_in_expr(a, field_names, method_names, local_vars, field_accessed, field_mutated);
                }
            }
            Expr::MemberAccess { object, .. } | Expr::SafeNav { object, .. } => {
                self.resolve_fields_in_expr(object, field_names, method_names, local_vars, field_accessed, field_mutated);
            }
            Expr::Index { collection, index, .. } => {
                self.resolve_fields_in_expr(collection, field_names, method_names, local_vars, field_accessed, field_mutated);
                self.resolve_fields_in_expr(index, field_names, method_names, local_vars, field_accessed, field_mutated);
            }
            Expr::List(elements, _) => {
                for el in elements {
                    self.resolve_fields_in_expr(el, field_names, method_names, local_vars, field_accessed, field_mutated);
                }
            }
            Expr::Map(pairs, _) => {
                for (k, v) in pairs {
                    self.resolve_fields_in_expr(k, field_names, method_names, local_vars, field_accessed, field_mutated);
                    self.resolve_fields_in_expr(v, field_names, method_names, local_vars, field_accessed, field_mutated);
                }
            }
            Expr::Block(b, _) => {
                let mut blk_locals = local_vars.clone();
                self.resolve_fields_in_block(b, field_names, method_names, &mut blk_locals, field_accessed, field_mutated);
            }
            Expr::Tuple(items, _) => {
                for item in items {
                    self.resolve_fields_in_expr(item, field_names, method_names, local_vars, field_accessed, field_mutated);
                }
            }
            Expr::Match { expr: target, arms, .. } => {
                self.resolve_fields_in_expr(target, field_names, method_names, local_vars, field_accessed, field_mutated);
                for arm in arms {
                    let arm_locals = local_vars.clone();
                    self.resolve_fields_in_expr(&mut arm.pattern, field_names, method_names, &arm_locals, field_accessed, field_mutated);
                    if let Some(guard) = &mut arm.guard {
                        self.resolve_fields_in_expr(guard, field_names, method_names, &arm_locals, field_accessed, field_mutated);
                    }
                    self.resolve_fields_in_expr(&mut arm.body, field_names, method_names, &arm_locals, field_accessed, field_mutated);
                }
            }
            _ => {}
        }
    }

    fn resolve_fields_in_jsx_element(
        &self,
        elem: &mut JsxElement,
        field_names: &HashSet<String>,
        local_vars: &HashSet<String>,
    ) {
        let mut dummy_acc = false;
        let mut dummy_mut = false;
        let empty_methods = HashSet::new();
        for attr in &mut elem.attributes {
            if let JsxAttrValue::Expr(e) = &mut attr.value {
                self.resolve_fields_in_expr(e, field_names, &empty_methods, local_vars, &mut dummy_acc, &mut dummy_mut);
            }
        }
        for child in &mut elem.children {
            self.resolve_fields_in_jsx_child(child, field_names, local_vars);
        }
    }

    fn resolve_fields_in_jsx_child(
        &self,
        child: &mut JsxChild,
        field_names: &HashSet<String>,
        local_vars: &HashSet<String>,
    ) {
        let mut dummy_acc = false;
        let mut dummy_mut = false;
        let empty_methods = HashSet::new();
        match child {
            JsxChild::Element(e) => {
                self.resolve_fields_in_jsx_element(e, field_names, local_vars);
            }
            JsxChild::Expr(e) => {
                self.resolve_fields_in_expr(e, field_names, &empty_methods, local_vars, &mut dummy_acc, &mut dummy_mut);
            }
            JsxChild::If { cond, then_child, else_child } => {
                self.resolve_fields_in_expr(cond, field_names, &empty_methods, local_vars, &mut dummy_acc, &mut dummy_mut);
                self.resolve_fields_in_jsx_child(then_child, field_names, local_vars);
                if let Some(ec) = else_child {
                    self.resolve_fields_in_jsx_child(ec, field_names, local_vars);
                }
            }
            JsxChild::For { iter, body, item } => {
                self.resolve_fields_in_expr(iter, field_names, &empty_methods, local_vars, &mut dummy_acc, &mut dummy_mut);
                let mut inner_locals = local_vars.clone();
                inner_locals.insert(item.clone());
                self.resolve_fields_in_jsx_child(body, field_names, &inner_locals);
            }
            _ => {}
        }
    }

    fn analyze_function(&mut self, f: &mut FnDef) -> Result<(), String> {
        self.analyze_function_common(f)
    }

    fn analyze_function_common(&mut self, f: &mut FnDef) -> Result<(), String> {
        // Step 1: Infer parameter types and borrowing / ownership from body usage
        for p in &mut f.params {
            if p.ty.is_none() {
                p.ty = Some(self.infer_param_type(&p.name, &f.body));
            }
            let usage = self.inspect_param_usage(&p.name, &f.body);
            p.ownership = Some(usage);
        }

        // Step 2: Analyze block statements, mutability, calls, etc.
        let mut initial_vars = HashSet::new();
        for p in &f.params {
            initial_vars.insert(p.name.clone());
        }

        self.analyze_block(&mut f.body, &initial_vars, false)?;

        // Step 2.5: Infer function return type if not specified
        if f.return_ty.is_none() {
            if let Some(last_stmt) = f.body.stmts.last() {
                match last_stmt {
                    Stmt::Expr(e) => {
                        let inf = self.infer_expr_type(e);
                        if inf != TypeAnnotation::Inferred {
                            f.return_ty = Some(inf);
                        }
                    }
                    Stmt::Return(Some(e), _) => {
                        let inf = self.infer_expr_type(e);
                        if inf != TypeAnnotation::Inferred {
                            f.return_ty = Some(inf);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Step 3: Check if function uses '!' (error handling) or async
        let (has_try, is_async) = self.check_try_and_async(&f.body);
        f.is_async = f.is_async || is_async;

        if has_try {
            // If return_ty is not specified, wrap in Result<T, Box<dyn std::error::Error>>
            let inner_ret = f.return_ty.clone().unwrap_or(TypeAnnotation::Unit);
            // We flag that it has try so codegen can wrap in Result and append Ok(...)
            if f.return_ty.is_none() {
                f.return_ty = Some(inner_ret);
            }
        }

        Ok(())
    }

    fn inspect_param_usage(&self, param_name: &str, body: &Block) -> ParamOwnership {
        let mut mutated = false;
        let mut moved = false;

        self.scan_param_in_block(param_name, body, &mut mutated, &mut moved);

        if mutated {
            ParamOwnership::MutRef
        } else if moved {
            ParamOwnership::Value
        } else {
            ParamOwnership::Ref
        }
    }

    fn scan_param_in_block(&self, name: &str, block: &Block, mutated: &mut bool, moved: &mut bool) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Assign { target, value, op, .. } => {
                    if let Expr::Ident(id, _) = target {
                        if id == name {
                            *mutated = true;
                        }
                    }
                    if op.is_some() {
                        if let Expr::Ident(id, _) = target {
                            if id == name {
                                *mutated = true;
                            }
                        }
                    }
                    self.scan_param_in_expr(name, value, mutated, moved);
                }
                Stmt::Expr(e) => {
                    self.scan_param_in_expr(name, e, mutated, moved);
                }
                Stmt::Return(Some(e), _) => {
                    if let Expr::Ident(id, _) = e {
                        if id == name {
                            *moved = true;
                        }
                    }
                    self.scan_param_in_expr(name, e, mutated, moved);
                }
                Stmt::If { cond, then_branch, elif_branches, else_branch, .. } => {
                    self.scan_param_in_expr(name, cond, mutated, moved);
                    self.scan_param_in_block(name, then_branch, mutated, moved);
                    for (c, b) in elif_branches {
                        self.scan_param_in_expr(name, c, mutated, moved);
                        self.scan_param_in_block(name, b, mutated, moved);
                    }
                    if let Some(eb) = else_branch {
                        self.scan_param_in_block(name, eb, mutated, moved);
                    }
                }
                Stmt::For { iter, body, .. } => {
                    self.scan_param_in_expr(name, iter, mutated, moved);
                    self.scan_param_in_block(name, body, mutated, moved);
                }
                Stmt::While { cond, body, .. } => {
                    self.scan_param_in_expr(name, cond, mutated, moved);
                    self.scan_param_in_block(name, body, mutated, moved);
                }
                _ => {}
            }
        }
    }

    fn scan_param_in_expr(&self, name: &str, expr: &Expr, mutated: &mut bool, moved: &mut bool) {
        match expr {
            Expr::MethodCall { receiver, method, args, .. } => {
                if let Expr::Ident(id, _) = receiver.as_ref() {
                    if id == name && is_mutating_method(method) {
                        *mutated = true;
                    }
                }
                self.scan_param_in_expr(name, receiver, mutated, moved);
                for a in args {
                    self.scan_param_in_expr(name, a, mutated, moved);
                }
            }
            Expr::Go(inner, _) => {
                if let Expr::Ident(id, _) = inner.as_ref() {
                    if id == name {
                        *moved = true;
                    }
                }
                self.scan_param_in_expr(name, inner, mutated, moved);
            }
            Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
                self.scan_param_in_expr(name, left, mutated, moved);
                self.scan_param_in_expr(name, right, mutated, moved);
            }
            Expr::Ternary { cond, then_expr, else_expr, .. } => {
                self.scan_param_in_expr(name, cond, mutated, moved);
                self.scan_param_in_expr(name, then_expr, mutated, moved);
                self.scan_param_in_expr(name, else_expr, mutated, moved);
            }
            Expr::Assign { target, value, .. } => {
                self.scan_param_in_expr(name, target, mutated, moved);
                self.scan_param_in_expr(name, value, mutated, moved);
            }
            Expr::Closure { body, .. } => {
                self.scan_param_in_expr(name, body, mutated, moved);
            }
            Expr::Unary { expr: inner, .. }
            | Expr::Try(inner, _)
            | Expr::Clone(inner, _)
            | Expr::Borrow(inner, _, _)
            | Expr::Cast { expr: inner, .. }
            | Expr::DynWrap { expr: inner, .. } => {
                self.scan_param_in_expr(name, inner, mutated, moved);
            }
            Expr::Call { callee, args, .. } => {
                self.scan_param_in_expr(name, callee, mutated, moved);
                for a in args {
                    self.scan_param_in_expr(name, a, mutated, moved);
                }
            }
            Expr::Index { collection, index, .. } => {
                self.scan_param_in_expr(name, collection, mutated, moved);
                self.scan_param_in_expr(name, index, mutated, moved);
            }
            Expr::List(elements, _) => {
                for el in elements {
                    self.scan_param_in_expr(name, el, mutated, moved);
                }
            }
            Expr::Map(pairs, _) => {
                for (k, v) in pairs {
                    self.scan_param_in_expr(name, k, mutated, moved);
                    self.scan_param_in_expr(name, v, mutated, moved);
                }
            }
            Expr::Block(b, _) => {
                self.scan_param_in_block(name, b, mutated, moved);
            }
            _ => {}
        }
    }

    /// Block-level analysis:
    /// 1. Track variable declarations, mutability, and type changes (shadowing)
    /// 2. Perform Last-Use Analysis on variables
    /// 3. Inject auto-borrowing (&, &mut) and auto-cloning at call sites
    /// 4. Auto-cast array index expressions to usize
    /// 5. Transform pipeline operator |> to function calls
    /// 6. Wrap heterogeneous lists into BaraDyn
    /// 7. Resolve dots to :: for crate/module paths
    fn analyze_block(
        &mut self,
        block: &mut Block,
        parent_scope_vars: &HashSet<String>,
        is_top_level: bool,
    ) -> Result<(), String> {
        // Collect variable mutability and types in this block
        let mut var_types: HashMap<String, TypeAnnotation> = HashMap::new();
        let mut var_decl_seen: HashSet<String> = parent_scope_vars.clone();
        let mut mutated_vars: HashSet<String> = HashSet::new();

        // Pass 1: Scan mutations across all statements in block
        let mut declared_scan = parent_scope_vars.clone();
        for stmt in &block.stmts {
            self.collect_mutations_in_stmt(stmt, &mut declared_scan, &mut mutated_vars);
        }

        // Pass 2: Lower expressions and assign let / let mut / shadowing
        let total_stmts = block.stmts.len();
        for i in 0..total_stmts {
            // Find last uses of variables from statement i onward
            let mut remaining_uses: HashSet<String> = HashSet::new();
            for j in (i + 1)..total_stmts {
                self.collect_used_vars_in_stmt(&block.stmts[j], &mut remaining_uses);
            }

            match &mut block.stmts[i] {
                Stmt::Assign {
                    target,
                    value,
                    op,
                    is_decl,
                    is_mut,
                    ty,
                    ..
                } => {
                    // Lower value expression first
                    self.lower_expr(value, &remaining_uses)?;

                    if let Expr::Ident(var_name, _) = target {
                        let inferred_ty = self.infer_expr_type(value);

                        if op.is_none() {
                            // Check if already declared with a different type (shadowing)
                            if let Some(prev_ty) = var_types.get(var_name) {
                                if *prev_ty != inferred_ty && inferred_ty != TypeAnnotation::Inferred {
                                    // Variable Shadowing: emit a new `let`
                                    *is_decl = true;
                                    *is_mut = mutated_vars.contains(var_name);
                                    *ty = Some(inferred_ty.clone());
                                    var_types.insert(var_name.clone(), inferred_ty);
                                } else {
                                    // Normal reassignment
                                    *is_decl = false;
                                }
                            } else if !var_decl_seen.contains(var_name) {
                                // First declaration in this scope
                                *is_decl = true;
                                *is_mut = mutated_vars.contains(var_name);
                                *ty = Some(inferred_ty.clone());
                                var_types.insert(var_name.clone(), inferred_ty);
                                var_decl_seen.insert(var_name.clone());
                            } else {
                                // Variable was declared in parent scope, so this is a mutation/reassignment
                                *is_decl = false;
                            }
                        } else {
                            // Mutation operator like +=, -=
                            *is_decl = false;
                        }
                    } else {
                        self.lower_expr(target, &remaining_uses)?;
                    }
                }
                Stmt::Expr(e) => {
                    self.lower_expr(e, &remaining_uses)?;
                }
                Stmt::Return(opt_e, _) => {
                    if let Some(e) = opt_e {
                        self.lower_expr(e, &remaining_uses)?;
                    }
                }
                Stmt::If {
                    cond,
                    then_branch,
                    elif_branches,
                    else_branch,
                    ..
                } => {
                    self.lower_expr(cond, &remaining_uses)?;
                    self.analyze_block(then_branch, &var_decl_seen, false)?;
                    for (c, b) in elif_branches {
                        self.lower_expr(c, &remaining_uses)?;
                        self.analyze_block(b, &var_decl_seen, false)?;
                    }
                    if let Some(eb) = else_branch {
                        self.analyze_block(eb, &var_decl_seen, false)?;
                    }
                }
                Stmt::For { iter, body, .. } => {
                    self.lower_expr(iter, &remaining_uses)?;
                    self.analyze_block(body, &var_decl_seen, false)?;
                }
                Stmt::While { cond, body, .. } => {
                    self.lower_expr(cond, &remaining_uses)?;
                    self.analyze_block(body, &var_decl_seen, false)?;
                }
                Stmt::Match { expr, arms, .. } => {
                    self.lower_expr(expr, &remaining_uses)?;
                    for arm in arms {
                        self.lower_expr(&mut arm.pattern, &remaining_uses)?;
                        if let Some(guard) = &mut arm.guard {
                            self.lower_expr(guard, &remaining_uses)?;
                        }
                        self.lower_expr(&mut arm.body, &remaining_uses)?;
                    }
                }
                _ => {}
            }
        }

        if is_top_level {
            let (has_try, is_async) = self.check_try_and_async(block);
            if has_try {
                self.main_has_try = true;
            }
            if is_async {
                self.main_is_async = true;
            }
        }

        Ok(())
    }

    fn collect_mutations_in_stmt(&self, stmt: &Stmt, declared: &mut HashSet<String>, mutated: &mut HashSet<String>) {
        match stmt {
            Stmt::Assign { target, op, value, .. } => {
                if let Expr::Ident(name, _) = target {
                    if op.is_some() || declared.contains(name) {
                        mutated.insert(name.clone());
                    } else {
                        declared.insert(name.clone());
                    }
                } else if let Some(root) = self.get_root_ident(target) {
                    mutated.insert(root.to_string());
                }
                self.collect_mutations_in_expr(value, mutated);
            }
            Stmt::Expr(e) => {
                self.collect_mutations_in_expr(e, mutated);
            }
            Stmt::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.collect_mutations_in_expr(cond, mutated);
                for s in &then_branch.stmts {
                    self.collect_mutations_in_stmt(s, declared, mutated);
                }
                for (c, b) in elif_branches {
                    self.collect_mutations_in_expr(c, mutated);
                    for s in &b.stmts {
                        self.collect_mutations_in_stmt(s, declared, mutated);
                    }
                }
                if let Some(eb) = else_branch {
                    for s in &eb.stmts {
                        self.collect_mutations_in_stmt(s, declared, mutated);
                    }
                }
            }
            Stmt::For { iter, body, .. } => {
                self.collect_mutations_in_expr(iter, mutated);
                for s in &body.stmts {
                    self.collect_mutations_in_stmt(s, declared, mutated);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.collect_mutations_in_expr(cond, mutated);
                for s in &body.stmts {
                    self.collect_mutations_in_stmt(s, declared, mutated);
                }
            }
            Stmt::Match { expr, arms, .. } => {
                self.collect_mutations_in_expr(expr, mutated);
                for arm in arms {
                    self.collect_mutations_in_expr(&arm.pattern, mutated);
                    if let Some(guard) = &arm.guard {
                        self.collect_mutations_in_expr(guard, mutated);
                    }
                    self.collect_mutations_in_expr(&arm.body, mutated);
                }
            }
            Stmt::Return(Some(val), _) => {
                self.collect_mutations_in_expr(val, mutated);
            }
            _ => {}
        }
    }

    fn collect_mutations_in_expr(&self, expr: &Expr, mutated: &mut HashSet<String>) {
        match expr {
            Expr::MethodCall { receiver, method, args, .. } => {
                let is_mutating = is_mutating_method(method)
                    || self.structs.values().any(|t| {
                        t.methods
                            .iter()
                            .any(|m| m.name == *method && m.receiver == ReceiverKind::MutSelf)
                    });
                if is_mutating {
                    if let Some(root) = self.get_root_ident(receiver) {
                        mutated.insert(root.to_string());
                    }
                }
                self.collect_mutations_in_expr(receiver, mutated);
                for a in args {
                    self.collect_mutations_in_expr(a, mutated);
                }
            }
            Expr::Assign { target, value, .. } => {
                if let Some(root) = self.get_root_ident(target) {
                    mutated.insert(root.to_string());
                }
                self.collect_mutations_in_expr(target, mutated);
                self.collect_mutations_in_expr(value, mutated);
            }
            Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
                self.collect_mutations_in_expr(left, mutated);
                self.collect_mutations_in_expr(right, mutated);
            }
            Expr::Unary { expr: inner, .. }
            | Expr::Try(inner, _)
            | Expr::Go(inner, _)
            | Expr::Clone(inner, _)
            | Expr::Borrow(inner, _, _)
            | Expr::Cast { expr: inner, .. }
            | Expr::SafeNav { object: inner, .. } => {
                self.collect_mutations_in_expr(inner, mutated);
            }
            Expr::Index { collection, index, .. } => {
                self.collect_mutations_in_expr(collection, mutated);
                self.collect_mutations_in_expr(index, mutated);
            }
            Expr::Call { callee, args, .. } => {
                self.collect_mutations_in_expr(callee, mutated);
                for a in args {
                    self.collect_mutations_in_expr(a, mutated);
                }
            }
            Expr::Ternary { cond, then_expr, else_expr, .. } => {
                self.collect_mutations_in_expr(cond, mutated);
                self.collect_mutations_in_expr(then_expr, mutated);
                self.collect_mutations_in_expr(else_expr, mutated);
            }
            Expr::List(items, _) => {
                for item in items {
                    self.collect_mutations_in_expr(item, mutated);
                }
            }
            Expr::Tuple(items, _) => {
                for item in items {
                    self.collect_mutations_in_expr(item, mutated);
                }
            }
            Expr::Match { expr: target, arms, .. } => {
                self.collect_mutations_in_expr(target, mutated);
                for arm in arms {
                    self.collect_mutations_in_expr(&arm.pattern, mutated);
                    if let Some(guard) = &arm.guard {
                        self.collect_mutations_in_expr(guard, mutated);
                    }
                    self.collect_mutations_in_expr(&arm.body, mutated);
                }
            }
            _ => {}
        }
    }

    fn get_root_ident<'a>(&self, expr: &'a Expr) -> Option<&'a str> {
        match expr {
            Expr::Ident(name, _) => Some(name.as_str()),
            Expr::Index { collection, .. } => self.get_root_ident(collection),
            Expr::SafeNav { object, .. } => self.get_root_ident(object),
            Expr::MemberAccess { object, .. } => self.get_root_ident(object),
            Expr::PathAccess { segments, .. } => segments.first().map(|s| s.as_str()),
            _ => None,
        }
    }

    fn collect_used_vars_in_stmt(&self, stmt: &Stmt, used: &mut HashSet<String>) {
        match stmt {
            Stmt::Assign { target, value, .. } => {
                self.collect_used_vars_in_expr(target, used);
                self.collect_used_vars_in_expr(value, used);
            }
            Stmt::Expr(e) => self.collect_used_vars_in_expr(e, used),
            Stmt::Return(opt_e, _) => {
                if let Some(e) = opt_e {
                    self.collect_used_vars_in_expr(e, used);
                }
            }
            Stmt::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.collect_used_vars_in_expr(cond, used);
                for s in &then_branch.stmts {
                    self.collect_used_vars_in_stmt(s, used);
                }
                for (c, b) in elif_branches {
                    self.collect_used_vars_in_expr(c, used);
                    for s in &b.stmts {
                        self.collect_used_vars_in_stmt(s, used);
                    }
                }
                if let Some(eb) = else_branch {
                    for s in &eb.stmts {
                        self.collect_used_vars_in_stmt(s, used);
                    }
                }
            }
            Stmt::For { iter, body, .. } => {
                self.collect_used_vars_in_expr(iter, used);
                for s in &body.stmts {
                    self.collect_used_vars_in_stmt(s, used);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.collect_used_vars_in_expr(cond, used);
                for s in &body.stmts {
                    self.collect_used_vars_in_stmt(s, used);
                }
            }
            Stmt::Match { expr, arms, .. } => {
                self.collect_used_vars_in_expr(expr, used);
                for arm in arms {
                    self.collect_used_vars_in_expr(&arm.pattern, used);
                    if let Some(guard) = &arm.guard {
                        self.collect_used_vars_in_expr(guard, used);
                    }
                    self.collect_used_vars_in_expr(&arm.body, used);
                }
            }
            _ => {}
        }
    }

    fn collect_used_vars_in_expr(&self, expr: &Expr, used: &mut HashSet<String>) {
        match expr {
            Expr::Ident(name, _) => {
                used.insert(name.clone());
            }
            Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
                self.collect_used_vars_in_expr(left, used);
                self.collect_used_vars_in_expr(right, used);
            }
            Expr::Ternary { cond, then_expr, else_expr, .. } => {
                self.collect_used_vars_in_expr(cond, used);
                self.collect_used_vars_in_expr(then_expr, used);
                self.collect_used_vars_in_expr(else_expr, used);
            }
            Expr::Assign { target, value, .. } => {
                self.collect_used_vars_in_expr(target, used);
                self.collect_used_vars_in_expr(value, used);
            }
            Expr::Closure { body, .. } => {
                self.collect_used_vars_in_expr(body, used);
            }
            Expr::Unary { expr: inner, .. }
            | Expr::Try(inner, _)
            | Expr::Go(inner, _)
            | Expr::Clone(inner, _)
            | Expr::Borrow(inner, _, _)
            | Expr::Cast { expr: inner, .. }
            | Expr::DynWrap { expr: inner, .. } => {
                self.collect_used_vars_in_expr(inner, used);
            }
            Expr::Call { callee, args, .. } => {
                self.collect_used_vars_in_expr(callee, used);
                for a in args {
                    self.collect_used_vars_in_expr(a, used);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                self.collect_used_vars_in_expr(receiver, used);
                for a in args {
                    self.collect_used_vars_in_expr(a, used);
                }
            }
            Expr::MemberAccess { object, .. } | Expr::SafeNav { object, .. } => {
                self.collect_used_vars_in_expr(object, used);
            }
            Expr::Index { collection, index, .. } => {
                self.collect_used_vars_in_expr(collection, used);
                self.collect_used_vars_in_expr(index, used);
            }
            Expr::List(elements, _) => {
                for el in elements {
                    self.collect_used_vars_in_expr(el, used);
                }
            }
            Expr::Map(pairs, _) => {
                for (k, v) in pairs {
                    self.collect_used_vars_in_expr(k, used);
                    self.collect_used_vars_in_expr(v, used);
                }
            }
            Expr::Block(b, _) => {
                for s in &b.stmts {
                    self.collect_used_vars_in_stmt(s, used);
                }
            }
            Expr::Tuple(elements, _) => {
                for el in elements {
                    self.collect_used_vars_in_expr(el, used);
                }
            }
            Expr::Match { expr: target, arms, .. } => {
                self.collect_used_vars_in_expr(target, used);
                for arm in arms {
                    self.collect_used_vars_in_expr(&arm.pattern, used);
                    if let Some(guard) = &arm.guard {
                        self.collect_used_vars_in_expr(guard, used);
                    }
                    self.collect_used_vars_in_expr(&arm.body, used);
                }
            }
            _ => {}
        }
    }

    fn lower_expr(&mut self, expr: &mut Expr, remaining_uses: &HashSet<String>) -> Result<(), String> {
        // 1. Pipeline lowering: a |> f(b) -> f(a, b)
        if let Expr::Pipe { left, right, span } = expr {
            let sp = *span;
            self.lower_expr(left, remaining_uses)?;
            self.lower_expr(right, remaining_uses)?;

            match right.as_mut() {
                Expr::Call { args, .. } => {
                    let mut new_args = vec![*left.clone()];
                    new_args.append(args);
                    *args = new_args;
                    *expr = *right.clone();
                }
                Expr::Ident(name, s) => {
                    *expr = Expr::Call {
                        callee: Box::new(Expr::Ident(name.clone(), *s)),
                        args: vec![*left.clone()],
                        span: sp,
                    };
                }
                _ => {
                    *expr = Expr::Call {
                        callee: right.clone(),
                        args: vec![*left.clone()],
                        span: sp,
                    };
                }
            }
            return Ok(());
        }

        // 2. Heterogeneous List check
        if let Expr::List(elements, span) = expr {
            let sp = *span;
            for el in elements.iter_mut() {
                self.lower_expr(el, remaining_uses)?;
            }

            let mut has_diff_types = false;
            if elements.len() > 1 {
                let first_ty = self.infer_expr_type(&elements[0]);
                for el in elements.iter().skip(1) {
                    let t = self.infer_expr_type(el);
                    if t != first_ty && first_ty != TypeAnnotation::Inferred && t != TypeAnnotation::Inferred {
                        has_diff_types = true;
                        break;
                    }
                }
            }

            if has_diff_types {
                self.needs_bara_dyn = true;
                let mut wrapped = Vec::new();
                for el in elements.drain(..) {
                    let variant = match self.infer_expr_type(&el) {
                        TypeAnnotation::Int => "Int",
                        TypeAnnotation::Float => "Float",
                        TypeAnnotation::Str => "Str",
                        TypeAnnotation::Bool => "Bool",
                        TypeAnnotation::List(_) => "List",
                        _ => "Str",
                    };
                    wrapped.push(Expr::DynWrap {
                        variant: variant.to_string(),
                        expr: Box::new(el),
                        span: sp,
                    });
                }
                *elements = wrapped;
            }
            return Ok(());
        }

        // 3. Array Index auto-cast to usize
        if let Expr::Index { collection, index, span } = expr {
            let sp = *span;
            self.lower_expr(collection, remaining_uses)?;
            self.lower_expr(index, remaining_uses)?;

            // If index is not already a cast, cast it to usize
            if !matches!(index.as_ref(), Expr::Cast { target_ty, .. } if target_ty == "usize") {
                let idx = *index.clone();
                *index = Box::new(Expr::Cast {
                    expr: Box::new(idx),
                    target_ty: "usize".to_string(),
                    span: sp,
                });
            }
            return Ok(());
        }

        // 4. Dot-to-Path Resolution for method calls and member accesses
        if let Expr::MethodCall { receiver, method, args, span } = expr {
            let sp = *span;
            self.lower_expr(receiver, remaining_uses)?;
            for a in args.iter_mut() {
                self.lower_expr(a, remaining_uses)?;
            }

            // Check if receiver is a known crate or module or struct
            if let Some(path_str) = self.get_path_prefix(receiver) {
                // Transform into PathAccess + Call: path::method(args)
                let full_path = format!("{path_str}::{method}");
                *expr = Expr::Call {
                    callee: Box::new(Expr::Ident(full_path, sp)),
                    args: args.clone(),
                    span: sp,
                };
                return Ok(());
            }

            // Auto-borrowing for method arguments
            let mut matched_params: Option<Vec<Param>> = None;
            for t in self.structs.values() {
                if let Some(m) = t.methods.iter().find(|m| m.name == *method) {
                    matched_params = Some(m.params.clone());
                    break;
                }
            }

            if let Some(params) = matched_params {
                for (idx, arg) in args.iter_mut().enumerate() {
                    if idx < params.len() {
                        let p = &params[idx];
                        if let Expr::Ident(arg_name, arg_span) = arg.clone() {
                            match p.ownership {
                                Some(ParamOwnership::MutRef) => {
                                    *arg = Expr::Borrow(Box::new(Expr::Ident(arg_name, arg_span)), true, arg_span);
                                }
                                Some(ParamOwnership::Ref) => {
                                    let is_copy = matches!(
                                        p.ty,
                                        Some(TypeAnnotation::Int | TypeAnnotation::Float | TypeAnnotation::Bool)
                                    );
                                    if !is_copy {
                                        *arg = Expr::Borrow(Box::new(Expr::Ident(arg_name, arg_span)), false, arg_span);
                                    }
                                }
                                Some(ParamOwnership::Value) | None => {
                                    if remaining_uses.contains(&arg_name) {
                                        *arg = Expr::Clone(Box::new(Expr::Ident(arg_name, arg_span)), arg_span);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. Call site injection: Auto-borrowing (&, &mut) and last-use .clone()
        if let Expr::Call { callee, args, .. } = expr {
            self.lower_expr(callee, remaining_uses)?;
            for a in args.iter_mut() {
                self.lower_expr(a, remaining_uses)?;
            }

            // If callee is an identifier of a known function, inspect parameter ownership
            if let Expr::Ident(fn_name, _) = callee.as_ref() {
                if let Some(sig) = self.functions.get(fn_name).cloned() {
                    for (idx, arg) in args.iter_mut().enumerate() {
                        if idx < sig.params.len() {
                            let p = &sig.params[idx];
                            if let Expr::Ident(arg_name, arg_span) = arg.clone() {
                                match p.ownership {
                                    Some(ParamOwnership::MutRef) => {
                                        *arg = Expr::Borrow(Box::new(Expr::Ident(arg_name, arg_span)), true, arg_span);
                                    }
                                    Some(ParamOwnership::Ref) => {
                                        let is_copy = matches!(
                                            p.ty,
                                            Some(TypeAnnotation::Int | TypeAnnotation::Float | TypeAnnotation::Bool)
                                        );
                                        if !is_copy {
                                            *arg = Expr::Borrow(Box::new(Expr::Ident(arg_name, arg_span)), false, arg_span);
                                        }
                                    }
                                    Some(ParamOwnership::Value) | None => {
                                        // Last-use check: if arg_name is used later in caller scope, clone it!
                                        if remaining_uses.contains(&arg_name) {
                                            *arg = Expr::Clone(Box::new(Expr::Ident(arg_name, arg_span)), arg_span);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            return Ok(());
        }

        // Recurse into other sub-expressions
        match expr {
            Expr::Binary { left, right, .. } => {
                self.lower_expr(left, remaining_uses)?;
                self.lower_expr(right, remaining_uses)?;
            }
            Expr::Ternary { cond, then_expr, else_expr, .. } => {
                self.lower_expr(cond, remaining_uses)?;
                self.lower_expr(then_expr, remaining_uses)?;
                self.lower_expr(else_expr, remaining_uses)?;
            }
            Expr::Assign { target, value, .. } => {
                self.lower_expr(target, remaining_uses)?;
                self.lower_expr(value, remaining_uses)?;
            }
            Expr::Closure { body, .. } => {
                self.lower_expr(body, remaining_uses)?;
            }
            Expr::Unary { expr: inner, .. }
            | Expr::Try(inner, _)
            | Expr::Go(inner, _)
            | Expr::Clone(inner, _)
            | Expr::Borrow(inner, _, _)
            | Expr::Cast { expr: inner, .. }
            | Expr::DynWrap { expr: inner, .. } => {
                self.lower_expr(inner, remaining_uses)?;
            }
            Expr::MemberAccess { object, member, span } => {
                if let Some(prefix) = self.get_path_prefix(object) {
                    *expr = Expr::PathAccess {
                        segments: vec![prefix, member.clone()],
                        span: *span,
                    };
                } else {
                    self.lower_expr(object, remaining_uses)?;
                }
            }
            Expr::SafeNav { object, .. } => {
                self.lower_expr(object, remaining_uses)?;
            }
            Expr::Tuple(elements, _) => {
                for el in elements {
                    self.lower_expr(el, remaining_uses)?;
                }
            }
            Expr::Match { expr: target, arms, .. } => {
                self.lower_expr(target, remaining_uses)?;
                for arm in arms {
                    self.lower_expr(&mut arm.pattern, remaining_uses)?;
                    if let Some(guard) = &mut arm.guard {
                        self.lower_expr(guard, remaining_uses)?;
                    }
                    self.lower_expr(&mut arm.body, remaining_uses)?;
                }
            }
            Expr::Map(pairs, _) => {
                for (k, v) in pairs {
                    self.lower_expr(k, remaining_uses)?;
                    self.lower_expr(v, remaining_uses)?;
                }
            }
            Expr::Block(b, _) => {
                self.analyze_block(b, remaining_uses, false)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn get_path_prefix(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name, _) => {
                if self.config.known_crates.contains(name)
                    || self.structs.contains_key(name)
                    || self.enums.contains_key(name)
                    || name.chars().next().map_or(false, |c| c.is_uppercase())
                {
                    Some(name.clone())
                } else {
                    None
                }
            }
            Expr::MemberAccess { object, member, .. } => {
                if let Some(prefix) = self.get_path_prefix(object) {
                    Some(format!("{prefix}::{member}"))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn check_try_and_async(&self, block: &Block) -> (bool, bool) {
        let mut has_try = false;
        let mut is_async = false;

        for stmt in &block.stmts {
            self.scan_try_async_stmt(stmt, &mut has_try, &mut is_async);
        }

        (has_try, is_async)
    }

    fn scan_try_async_stmt(&self, stmt: &Stmt, has_try: &mut bool, is_async: &mut bool) {
        match stmt {
            Stmt::Assign { value, .. } | Stmt::Expr(value) => {
                self.scan_try_async_expr(value, has_try, is_async);
            }
            Stmt::Return(Some(e), _) => {
                self.scan_try_async_expr(e, has_try, is_async);
            }
            Stmt::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.scan_try_async_expr(cond, has_try, is_async);
                for s in &then_branch.stmts {
                    self.scan_try_async_stmt(s, has_try, is_async);
                }
                for (c, b) in elif_branches {
                    self.scan_try_async_expr(c, has_try, is_async);
                    for s in &b.stmts {
                        self.scan_try_async_stmt(s, has_try, is_async);
                    }
                }
                if let Some(eb) = else_branch {
                    for s in &eb.stmts {
                        self.scan_try_async_stmt(s, has_try, is_async);
                    }
                }
            }
            Stmt::For { iter, body, .. } => {
                self.scan_try_async_expr(iter, has_try, is_async);
                for s in &body.stmts {
                    self.scan_try_async_stmt(s, has_try, is_async);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.scan_try_async_expr(cond, has_try, is_async);
                for s in &body.stmts {
                    self.scan_try_async_stmt(s, has_try, is_async);
                }
            }
            Stmt::Match { expr, arms, .. } => {
                self.scan_try_async_expr(expr, has_try, is_async);
                for arm in arms {
                    self.scan_try_async_expr(&arm.pattern, has_try, is_async);
                    if let Some(guard) = &arm.guard {
                        self.scan_try_async_expr(guard, has_try, is_async);
                    }
                    self.scan_try_async_expr(&arm.body, has_try, is_async);
                }
            }
            _ => {}
        }
    }

    fn scan_try_async_expr(&self, expr: &Expr, has_try: &mut bool, is_async: &mut bool) {
        match expr {
            Expr::Try(inner, _) => {
                *has_try = true;
                // If it's reqwest or tokio call, mark as async
                if self.is_async_expr(inner) {
                    *is_async = true;
                }
                self.scan_try_async_expr(inner, has_try, is_async);
            }
            Expr::Go(inner, _) => {
                *is_async = true;
                self.scan_try_async_expr(inner, has_try, is_async);
            }
            Expr::Call { callee, args, .. } => {
                if self.is_async_expr(callee) {
                    *is_async = true;
                }
                self.scan_try_async_expr(callee, has_try, is_async);
                for a in args {
                    self.scan_try_async_expr(a, has_try, is_async);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                if self.is_async_expr(receiver) {
                    *is_async = true;
                }
                self.scan_try_async_expr(receiver, has_try, is_async);
                for a in args {
                    self.scan_try_async_expr(a, has_try, is_async);
                }
            }
            Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
                self.scan_try_async_expr(left, has_try, is_async);
                self.scan_try_async_expr(right, has_try, is_async);
            }
            Expr::Ternary { cond, then_expr, else_expr, .. } => {
                self.scan_try_async_expr(cond, has_try, is_async);
                self.scan_try_async_expr(then_expr, has_try, is_async);
                self.scan_try_async_expr(else_expr, has_try, is_async);
            }
            Expr::Assign { target, value, .. } => {
                self.scan_try_async_expr(target, has_try, is_async);
                self.scan_try_async_expr(value, has_try, is_async);
            }
            Expr::Closure { body, .. } => {
                self.scan_try_async_expr(body, has_try, is_async);
            }
            Expr::Unary { expr: inner, .. }
            | Expr::Clone(inner, _)
            | Expr::Borrow(inner, _, _)
            | Expr::Cast { expr: inner, .. }
            | Expr::DynWrap { expr: inner, .. } => {
                self.scan_try_async_expr(inner, has_try, is_async);
            }
            Expr::Index { collection, index, .. } => {
                self.scan_try_async_expr(collection, has_try, is_async);
                self.scan_try_async_expr(index, has_try, is_async);
            }
            Expr::List(elements, _) => {
                for el in elements {
                    self.scan_try_async_expr(el, has_try, is_async);
                }
            }
            Expr::Block(b, _) => {
                for s in &b.stmts {
                    self.scan_try_async_stmt(s, has_try, is_async);
                }
            }
            Expr::Tuple(elements, _) => {
                for el in elements {
                    self.scan_try_async_expr(el, has_try, is_async);
                }
            }
            Expr::Match { expr: target, arms, .. } => {
                self.scan_try_async_expr(target, has_try, is_async);
                for arm in arms {
                    self.scan_try_async_expr(&arm.pattern, has_try, is_async);
                    if let Some(guard) = &arm.guard {
                        self.scan_try_async_expr(guard, has_try, is_async);
                    }
                    self.scan_try_async_expr(&arm.body, has_try, is_async);
                }
            }
            _ => {}
        }
    }

    fn is_async_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Ident(name, _) => name.contains("reqwest") || name.contains("tokio"),
            Expr::MethodCall { receiver, method, .. } => {
                method == "text" || method == "json" || method == "send" || self.is_async_expr(receiver)
            }
            Expr::Call { callee, .. } => self.is_async_expr(callee),
            Expr::MemberAccess { object, .. } => self.is_async_expr(object),
            _ => false,
        }
    }

    fn infer_param_type(&self, param_name: &str, body: &Block) -> TypeAnnotation {
        let mut is_list = false;
        self.scan_param_for_list(param_name, body, &mut is_list);
        if is_list {
            TypeAnnotation::List(Box::new(TypeAnnotation::Int))
        } else {
            TypeAnnotation::Int
        }
    }

    fn scan_param_for_list(&self, name: &str, block: &Block, is_list: &mut bool) {
        for s in &block.stmts {
            match s {
                Stmt::Expr(e) | Stmt::Assign { value: e, .. } => {
                    self.scan_expr_for_list(name, e, is_list);
                }
                Stmt::If { cond, then_branch, elif_branches, else_branch, .. } => {
                    self.scan_expr_for_list(name, cond, is_list);
                    self.scan_param_for_list(name, then_branch, is_list);
                    for (c, b) in elif_branches {
                        self.scan_expr_for_list(name, c, is_list);
                        self.scan_param_for_list(name, b, is_list);
                    }
                    if let Some(eb) = else_branch {
                        self.scan_param_for_list(name, eb, is_list);
                    }
                }
                _ => {}
            }
        }
    }

    fn scan_expr_for_list(&self, name: &str, expr: &Expr, is_list: &mut bool) {
        if let Expr::MethodCall { receiver, method, .. } = expr {
            if let Expr::Ident(id, _) = receiver.as_ref() {
                if id == name && matches!(method.as_str(), "push" | "pop" | "retain" | "len" | "clear") {
                    *is_list = true;
                }
            }
        }
    }

    fn infer_expr_type(&self, expr: &Expr) -> TypeAnnotation {
        match expr {
            Expr::Literal(lit, _) => match lit {
                Literal::Int(_) => TypeAnnotation::Int,
                Literal::Float(_) => TypeAnnotation::Float,
                Literal::Str(_) | Literal::InterpolatedStr(_) => TypeAnnotation::Str,
                Literal::Bool(_) => TypeAnnotation::Bool,
                Literal::None => TypeAnnotation::Unit,
            },
            Expr::Binary { op, .. } => match op {
                BinaryOp::Eq
                | BinaryOp::Neq
                | BinaryOp::Lt
                | BinaryOp::Lte
                | BinaryOp::Gt
                | BinaryOp::Gte
                | BinaryOp::And
                | BinaryOp::Or => TypeAnnotation::Bool,
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor => TypeAnnotation::Int,
            },
            Expr::Unary { op, expr: inner, .. } => match op {
                UnaryOp::Not => TypeAnnotation::Bool,
                UnaryOp::Neg => self.infer_expr_type(inner),
            },
            Expr::Ternary { then_expr, .. } => self.infer_expr_type(then_expr),
            Expr::Assign { value, .. } => self.infer_expr_type(value),
            Expr::Closure { .. } => TypeAnnotation::Inferred,
            Expr::List(elements, _) => {
                if let Some(first) = elements.first() {
                    TypeAnnotation::List(Box::new(self.infer_expr_type(first)))
                } else {
                    TypeAnnotation::List(Box::new(TypeAnnotation::Inferred))
                }
            }
            Expr::Map(pairs, _) => {
                if let Some((k, v)) = pairs.first() {
                    TypeAnnotation::Map(
                        Box::new(self.infer_expr_type(k)),
                        Box::new(self.infer_expr_type(v)),
                    )
                } else {
                    TypeAnnotation::Map(
                        Box::new(TypeAnnotation::Str),
                        Box::new(TypeAnnotation::Inferred),
                    )
                }
            }
            Expr::Call { callee, .. } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if self.structs.contains_key(name) {
                        return TypeAnnotation::Custom(name.clone());
                    }
                }
                TypeAnnotation::Inferred
            }
            Expr::MethodCall { method, .. } => {
                if method == "len" || method == "count" {
                    TypeAnnotation::Int
                } else if method == "is_empty" {
                    TypeAnnotation::Bool
                } else {
                    TypeAnnotation::Inferred
                }
            }
            Expr::Cast { target_ty, .. } => {
                if target_ty == "usize" || target_ty == "i64" {
                    TypeAnnotation::Int
                } else if target_ty == "f64" {
                    TypeAnnotation::Float
                } else {
                    TypeAnnotation::Custom(target_ty.clone())
                }
            }
            Expr::DynWrap { .. } => TypeAnnotation::Dyn,
            Expr::Tuple(elements, _) => {
                TypeAnnotation::Tuple(elements.iter().map(|e| self.infer_expr_type(e)).collect())
            }
            Expr::Match { arms, .. } => {
                if let Some(first) = arms.first() {
                    self.infer_expr_type(&first.body)
                } else {
                    TypeAnnotation::Inferred
                }
            }
            _ => TypeAnnotation::Inferred,
        }
    }
}

fn is_mutating_method(method: &str) -> bool {
    matches!(
        method,
        "push"
            | "pop"
            | "insert"
            | "remove"
            | "clear"
            | "extend"
            | "sort"
            | "reverse"
            | "dedup"
            | "append"
            | "retain"
            | "truncate"
    )
}
