use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens.last().expect("token stream must not be empty")
        })
    }

    fn peek(&self) -> &TokenKind {
        &self.current().kind
    }

    fn peek_next(&self) -> &TokenKind {
        if self.pos + 1 < self.tokens.len() {
            &self.tokens[self.pos + 1].kind
        } else {
            &TokenKind::Eof
        }
    }

    #[allow(dead_code)]
    fn peek_at(&self, offset: usize) -> &TokenKind {
        if self.pos + offset < self.tokens.len() {
            &self.tokens[self.pos + offset].kind
        } else {
            &TokenKind::Eof
        }
    }

    fn advance(&mut self) -> Token {
        let tok = self.current().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let cur = self.current();
            Err(format!(
                "Expected token '{kind}', found '{cur_kind}' at {line}:{col}",
                cur_kind = cur.kind,
                line = cur.span.0,
                col = cur.span.1
            ))
        }
    }

    fn skip_newlines(&mut self) {
        while self.match_token(&TokenKind::Newline) {}
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenKind::Eof) {
            let item = self.parse_item()?;
            items.push(item);
            self.skip_newlines();
        }

        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        self.skip_newlines();

        // 1. RawRust
        if let TokenKind::RawRust(body) = self.peek() {
            let span = self.current().span;
            let code = body.clone();
            self.advance();
            return Ok(Item::RawRust(code, span));
        }

        // 1.5 StyleBlock (<style> ... </style>)
        if let TokenKind::StyleBlock(css) = self.peek() {
            let span = self.current().span;
            let c = css.clone();
            self.advance();
            return Ok(Item::StyleBlock(c, span));
        }

        let is_pub = self.match_token(&TokenKind::Pub);

        // 2. Mod declaration
        if self.check(&TokenKind::Mod) {
            let mod_def = self.parse_mod_def(true)?;
            return Ok(Item::Mod(mod_def));
        }

        // 2.2 Use statement
        if self.check(&TokenKind::Use) {
            let use_stmt = self.parse_use_stmt(is_pub)?;
            return Ok(Item::Use(use_stmt));
        }

        // 2.5 View definition
        if self.check(&TokenKind::View) {
            let view = self.parse_view_def()?;
            return Ok(Item::View(view));
        }

        // 3. Enum definition
        if self.check(&TokenKind::Enum) {
            let enum_def = self.parse_enum_def(true)?;
            return Ok(Item::Enum(enum_def));
        }

        // 3.5 Type definition
        if self.check(&TokenKind::Type) {
            let type_def = self.parse_type_def(true)?;
            return Ok(Item::Type(type_def));
        }

        // 4. Function definition: [pub] [fn] name(params) [-> ret] => body
        if self.is_function_def() {
            let fn_def = self.parse_fn_def(true)?;
            return Ok(Item::Fn(fn_def));
        }

        // 5. Top-level statement
        let stmt = self.parse_stmt()?;
        Ok(Item::TopLevelStmt(stmt))
    }

    fn is_function_def(&self) -> bool {
        let mut cur = self.pos;
        if cur < self.tokens.len() && self.tokens[cur].kind == TokenKind::Pub {
            cur += 1;
        }
        let start_pos = if cur < self.tokens.len() {
            if let TokenKind::Ident(s) = &self.tokens[cur].kind {
                if s == "fn" {
                    cur + 1
                } else {
                    cur
                }
            } else {
                return false;
            }
        } else {
            return false;
        };

        if start_pos < self.tokens.len() {
            if let TokenKind::Ident(_) = &self.tokens[start_pos].kind {
                if start_pos + 1 < self.tokens.len() && self.tokens[start_pos + 1].kind == TokenKind::LParen {
                    // Look ahead past the matching RParen to see if there is `=>` or `->`
                    let mut idx = start_pos + 2;
                    let mut depth = 1;
                    while idx < self.tokens.len() {
                        match &self.tokens[idx].kind {
                            TokenKind::LParen => depth += 1,
                            TokenKind::RParen => {
                                depth -= 1;
                                if depth == 0 {
                                    idx += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        idx += 1;
                    }
                    while idx < self.tokens.len() && self.tokens[idx].kind == TokenKind::Newline {
                        idx += 1;
                    }
                    if idx < self.tokens.len() {
                        let k = &self.tokens[idx].kind;
                        if *k == TokenKind::FatArrow || *k == TokenKind::SkinnyArrow {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn parse_mod_def(&mut self, is_pub: bool) -> Result<ModDef, String> {
        let span = self.current().span;
        self.expect(&TokenKind::Mod)?;
        let name = self.parse_ident_name()?;
        self.match_token(&TokenKind::Semi);
        Ok(ModDef { name, is_pub, span })
    }

    fn parse_use_stmt(&mut self, is_pub: bool) -> Result<UseStmt, String> {
        let span = self.current().span;
        self.expect(&TokenKind::Use)?;

        // Explicit stylesheet import: `use "styles.css"` (preferred, exact file).
        if let TokenKind::Str(s) = self.peek() {
            let lit = s.clone();
            self.advance();
            self.match_token(&TokenKind::Semi);
            return Ok(UseStmt {
                path: vec![lit],
                alias: None,
                is_pub,
                span,
            });
        }

        let mut path = Vec::new();
        let first_seg = if self.match_token(&TokenKind::Star) {
            "*".to_string()
        } else {
            self.parse_ident_name()?
        };
        path.push(first_seg);

        loop {
            let is_colons = self.check(&TokenKind::Colon) && self.peek_next() == &TokenKind::Colon;
            let is_dot = self.check(&TokenKind::Dot);

            if is_colons {
                self.advance();
                self.advance();
            } else if is_dot {
                self.advance();
            } else {
                break;
            }

            if self.match_token(&TokenKind::Star) {
                path.push("*".to_string());
                break;
            } else if self.match_token(&TokenKind::LBrace) {
                let mut group = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                    group.push(self.parse_ident_name()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                path.push(format!("{{{}}}", group.join(", ")));
                break;
            } else {
                let seg = self.parse_ident_name()?;
                path.push(seg);
            }
        }

        let alias = if self.match_token(&TokenKind::As) {
            Some(self.parse_ident_name()?)
        } else {
            None
        };

        self.match_token(&TokenKind::Semi);
        Ok(UseStmt { path, alias, is_pub, span })
    }

    fn parse_enum_def(&mut self, is_pub: bool) -> Result<EnumDef, String> {
        let span = self.current().span;
        self.expect(&TokenKind::Enum)?;
        let name = self.parse_ident_name()?;

        self.match_token(&TokenKind::Eq);
        self.match_token(&TokenKind::Colon);
        self.skip_newlines();

        let mut variants = Vec::new();
        let is_braced = self.match_token(&TokenKind::LBrace);
        let is_indented = if !is_braced {
            self.match_token(&TokenKind::Indent)
        } else {
            false
        };

        while !self.check(&TokenKind::Eof) {
            self.skip_newlines();
            if is_braced && self.check(&TokenKind::RBrace) {
                break;
            }
            if is_indented && self.check(&TokenKind::Dedent) {
                break;
            }
            if !is_braced && !is_indented && (self.check(&TokenKind::Newline) || self.check(&TokenKind::Eof)) {
                break;
            }

            let var_span = self.current().span;
            let var_name = self.parse_ident_name()?;
            let mut fields = Vec::new();

            if self.match_token(&TokenKind::LParen) {
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    fields.push(self.parse_type_annotation()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
            }

            variants.push(EnumVariant {
                name: var_name,
                fields,
                span: var_span,
            });

            self.match_token(&TokenKind::Comma);
            self.skip_newlines();

            if !is_braced && !is_indented {
                break;
            }
        }

        if is_braced {
            self.expect(&TokenKind::RBrace)?;
        } else if is_indented {
            self.expect(&TokenKind::Dedent)?;
        }

        Ok(EnumDef {
            name,
            variants,
            is_pub,
            span,
        })
    }

    fn parse_type_def(&mut self, is_pub: bool) -> Result<TypeDef, String> {
        let span = self.current().span;
        self.expect(&TokenKind::Type)?;
        let name = self.parse_ident_name()?;

        // Expect '=' or just block
        self.match_token(&TokenKind::Eq);
        self.skip_newlines();

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        let is_braced = self.match_token(&TokenKind::LBrace);
        let is_indented = if !is_braced {
            self.match_token(&TokenKind::Indent)
        } else {
            false
        };

        if !is_braced && !is_indented {
            return Err(format!(
                "Expected indented block or '{{' for type '{name}' at {line}:{col}",
                line = span.0,
                col = span.1
            ));
        }

        self.skip_newlines();

        while !self.check(&TokenKind::Eof) {
            if is_braced && self.check(&TokenKind::RBrace) {
                self.advance();
                break;
            }
            if is_indented && self.check(&TokenKind::Dedent) {
                self.advance();
                break;
            }

            self.skip_newlines();
            if (is_braced && self.check(&TokenKind::RBrace))
                || (is_indented && self.check(&TokenKind::Dedent))
            {
                continue;
            }

            // Check if it's a method
            if self.is_function_def() {
                let m = self.parse_fn_def(true)?;
                methods.push(m);
            } else {
                // It's a field
                let f_span = self.current().span;
                let f_name = self.parse_ident_name()?;
                let mut ty = None;
                let mut default_val = None;

                if self.match_token(&TokenKind::Colon) {
                    ty = Some(self.parse_type_annotation()?);
                }

                if self.match_token(&TokenKind::Eq) {
                    default_val = Some(self.parse_expr()?);
                }

                self.match_token(&TokenKind::Semi);
                self.skip_newlines();

                fields.push(FieldDef {
                    name: f_name,
                    ty,
                    default_val,
                    span: f_span,
                });
            }

            self.skip_newlines();
        }

        Ok(TypeDef {
            name,
            fields,
            methods,
            is_pub,
            span,
        })
    }

    fn parse_view_def(&mut self) -> Result<ViewDef, String> {
        let span = self.current().span;
        self.expect(&TokenKind::View)?;
        let type_name = self.parse_ident_name()?;
        self.expect(&TokenKind::FatArrow)?;
        self.skip_newlines();

        let had_indent = self.match_token(&TokenKind::Indent);
        self.skip_newlines();

        let root = self.parse_jsx_element()?;
        self.skip_newlines();

        if had_indent {
            self.match_token(&TokenKind::Dedent);
        }

        Ok(ViewDef {
            type_name,
            root,
            span,
        })
    }

    fn skip_jsx_whitespace(&mut self) {
        while self.match_token(&TokenKind::Newline)
            || self.match_token(&TokenKind::Indent)
            || self.match_token(&TokenKind::Dedent)
        {}
    }

    pub fn parse_jsx_element(&mut self) -> Result<JsxElement, String> {
        let span = self.current().span;
        self.expect(&TokenKind::Lt)?;
        let tag = self.parse_ident_name()?;

        let mut attributes = Vec::new();
        while !self.check(&TokenKind::Gt)
            && !self.check(&TokenKind::SlashGt)
            && !self.check(&TokenKind::Eof)
        {
            if self.match_token(&TokenKind::Newline)
                || self.match_token(&TokenKind::Indent)
                || self.match_token(&TokenKind::Dedent)
            {
                continue;
            }

            let attr_span = self.current().span;
            let attr_name = self.parse_ident_name()?;
            let attr_val = if self.match_token(&TokenKind::Eq) {
                if let TokenKind::Str(s) = self.peek() {
                    let st = s.clone();
                    self.advance();
                    JsxAttrValue::Lit(st)
                } else if self.match_token(&TokenKind::LBrace) {
                    let expr = self.parse_expr()?;
                    self.expect(&TokenKind::RBrace)?;
                    JsxAttrValue::Expr(expr)
                } else {
                    return Err(format!(
                        "Expected string or '{{' for attribute '{attr_name}' at {line}:{col}",
                        line = attr_span.0,
                        col = attr_span.1
                    ));
                }
            } else {
                JsxAttrValue::Lit("true".to_string())
            };

            attributes.push(JsxAttribute {
                name: attr_name,
                value: attr_val,
                span: attr_span,
            });
        }

        if self.match_token(&TokenKind::SlashGt) {
            return Ok(JsxElement {
                tag,
                attributes,
                children: Vec::new(),
                span,
            });
        }

        self.expect(&TokenKind::Gt)?;

        let mut children = Vec::new();
        loop {
            if self.check(&TokenKind::LtSlash) {
                self.advance();
                let close_tag = self.parse_ident_name()?;
                if close_tag != tag {
                    return Err(format!(
                        "Mismatched closing tag '</{close_tag}>', expected '</{tag}>' at {line}:{col}",
                        line = span.0,
                        col = span.1
                    ));
                }
                self.expect(&TokenKind::Gt)?;
                break;
            }

            if self.check(&TokenKind::Eof) {
                return Err(format!(
                    "Unclosed JSX tag '<{tag}>' at {line}:{col}",
                    line = span.0,
                    col = span.1
                ));
            }

            if self.match_token(&TokenKind::Newline)
                || self.match_token(&TokenKind::Indent)
                || self.match_token(&TokenKind::Dedent)
            {
                continue;
            }

            if self.match_token(&TokenKind::LBrace) {
                if self.match_token(&TokenKind::If) {
                    let cond = self.parse_ternary()?;
                    self.expect(&TokenKind::FatArrow)?;
                    let child = self.parse_jsx_child_item()?;
                    let mut else_child = None;
                    if self.match_token(&TokenKind::Else) {
                        self.expect(&TokenKind::FatArrow)?;
                        else_child = Some(Box::new(self.parse_jsx_child_item()?));
                    }
                    self.skip_jsx_whitespace();
                    self.expect(&TokenKind::RBrace)?;
                    children.push(JsxChild::If {
                        cond,
                        then_child: Box::new(child),
                        else_child,
                    });
                } else if self.match_token(&TokenKind::For) {
                    let item = self.parse_ident_name()?;
                    self.expect(&TokenKind::In)?;
                    let iter = self.parse_ternary()?;
                    self.expect(&TokenKind::FatArrow)?;
                    let body = self.parse_jsx_child_item()?;
                    self.skip_jsx_whitespace();
                    self.expect(&TokenKind::RBrace)?;
                    children.push(JsxChild::For {
                        item,
                        iter,
                        body: Box::new(body),
                    });
                } else if matches!(self.peek(), TokenKind::Ident(s) if s == "useEffect") {
                    let eff_span = self.current().span;
                    self.advance(); // consume `useEffect`
                    self.expect(&TokenKind::LParen)?;
                    let mut deps = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        deps.push(self.parse_expr()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    self.expect(&TokenKind::FatArrow)?;
                    let body = self.parse_expr()?;
                    self.skip_jsx_whitespace();
                    self.expect(&TokenKind::RBrace)?;
                    children.push(JsxChild::Effect {
                        deps,
                        body,
                        span: eff_span,
                    });
                } else {
                    let expr = self.parse_expr()?;
                    self.skip_jsx_whitespace();
                    self.expect(&TokenKind::RBrace)?;
                    children.push(JsxChild::Expr(expr));
                }
                continue;
            }

            if self.check(&TokenKind::Lt) {
                let child_elem = self.parse_jsx_element()?;
                children.push(JsxChild::Element(child_elem));
                continue;
            }

            let mut text = String::new();
            while !self.check(&TokenKind::Lt)
                && !self.check(&TokenKind::LtSlash)
                && !self.check(&TokenKind::LBrace)
                && !self.check(&TokenKind::Newline)
                && !self.check(&TokenKind::Indent)
                && !self.check(&TokenKind::Dedent)
                && !self.check(&TokenKind::Eof)
            {
                let tok = self.advance();
                let piece = match &tok.kind {
                    TokenKind::Str(s) => s.clone(),
                    TokenKind::Ident(s) => s.clone(),
                    other => format!("{other}"),
                };
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&piece);
            }

            if !text.trim().is_empty() {
                children.push(JsxChild::Text(text.trim().to_string()));
            }
        }

        Ok(JsxElement {
            tag,
            attributes,
            children,
            span,
        })
    }

    fn parse_jsx_child_item(&mut self) -> Result<JsxChild, String> {
        self.skip_jsx_whitespace();
        if self.check(&TokenKind::Lt) {
            let el = self.parse_jsx_element()?;
            Ok(JsxChild::Element(el))
        } else if self.match_token(&TokenKind::LBrace) {
            if self.match_token(&TokenKind::If) {
                let cond = self.parse_ternary()?;
                self.expect(&TokenKind::FatArrow)?;
                let child = self.parse_jsx_child_item()?;
                let mut else_child = None;
                if self.match_token(&TokenKind::Else) {
                    self.expect(&TokenKind::FatArrow)?;
                    else_child = Some(Box::new(self.parse_jsx_child_item()?));
                }
                self.skip_jsx_whitespace();
                self.expect(&TokenKind::RBrace)?;
                Ok(JsxChild::If {
                    cond,
                    then_child: Box::new(child),
                    else_child,
                })
            } else if self.match_token(&TokenKind::For) {
                let item = self.parse_ident_name()?;
                self.expect(&TokenKind::In)?;
                let iter = self.parse_ternary()?;
                self.expect(&TokenKind::FatArrow)?;
                let body = self.parse_jsx_child_item()?;
                self.skip_jsx_whitespace();
                self.expect(&TokenKind::RBrace)?;
                Ok(JsxChild::For {
                    item,
                    iter,
                    body: Box::new(body),
                })
            } else if matches!(self.peek(), TokenKind::Ident(s) if s == "useEffect") {
                let eff_span = self.current().span;
                self.advance(); // consume `useEffect`
                self.expect(&TokenKind::LParen)?;
                let mut deps = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    deps.push(self.parse_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::FatArrow)?;
                let body = self.parse_expr()?;
                self.skip_jsx_whitespace();
                self.expect(&TokenKind::RBrace)?;
                Ok(JsxChild::Effect {
                    deps,
                    body,
                    span: eff_span,
                })
            } else {
                let e = self.parse_expr()?;
                self.skip_jsx_whitespace();
                self.expect(&TokenKind::RBrace)?;
                Ok(JsxChild::Expr(e))
            }
        } else {
            let mut text = String::new();
            while !self.check(&TokenKind::Lt)
                && !self.check(&TokenKind::LtSlash)
                && !self.check(&TokenKind::LBrace)
                && !self.check(&TokenKind::RBrace)
                && !self.check(&TokenKind::Newline)
                && !self.check(&TokenKind::Indent)
                && !self.check(&TokenKind::Dedent)
                && !self.check(&TokenKind::Eof)
            {
                let tok = self.advance();
                let piece = match &tok.kind {
                    TokenKind::Str(s) => s.clone(),
                    TokenKind::Ident(s) => s.clone(),
                    other => format!("{other}"),
                };
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&piece);
            }
            Ok(JsxChild::Text(text.trim().to_string()))
        }
    }

    fn parse_fn_def(&mut self, is_pub: bool) -> Result<FnDef, String> {
        let span = self.current().span;
        if let TokenKind::Ident(s) = self.peek() {
            if s == "fn" {
                self.advance();
            }
        }
        let name = self.parse_ident_name()?;

        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let p_span = self.current().span;
            let p_name = self.parse_ident_name()?;
            let mut ty = None;
            let mut default_val = None;

            if self.match_token(&TokenKind::Colon) {
                ty = Some(self.parse_type_annotation()?);
            }

            if self.match_token(&TokenKind::Eq) {
                default_val = Some(self.parse_expr()?);
            }

            params.push(Param {
                name: p_name,
                ty,
                ownership: None,
                default_val,
                span: p_span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen)?;

        // Return type
        let return_ty = if self.match_token(&TokenKind::SkinnyArrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        self.expect(&TokenKind::FatArrow)?;
        let body = self.parse_block()?;

        Ok(FnDef {
            name,
            params,
            return_ty,
            body,
            is_async: false,
            receiver: ReceiverKind::None,
            is_pub,
            span,
        })
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, String> {
        // References: &T, &mut T
        if self.match_token(&TokenKind::Amp) {
            let is_mut = if let TokenKind::Ident(s) = self.peek() {
                if s == "mut" {
                    self.advance();
                    true
                } else {
                    false
                }
            } else {
                false
            };
            let inner = self.parse_type_annotation()?;
            let prefix = if is_mut { "&mut " } else { "&" };
            return Ok(TypeAnnotation::Custom(format!("{prefix}{inner}")));
        }

        match self.peek() {
            TokenKind::Ident(name) => {
                let mut n = name.clone();
                self.advance();

                while (self.check(&TokenKind::Colon) && self.peek_next() == &TokenKind::Colon)
                    || self.check(&TokenKind::Dot)
                {
                    if self.match_token(&TokenKind::Dot) {
                        let seg = self.parse_ident_name()?;
                        n.push_str("::");
                        n.push_str(&seg);
                    } else {
                        self.advance();
                        self.advance();
                        let seg = self.parse_ident_name()?;
                        n.push_str("::");
                        n.push_str(&seg);
                    }
                }

                // Check for generic type arguments: e.g. Option<int>, Result<str, int>, Arc<Mutex<int>>
                if self.match_token(&TokenKind::Lt) {
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::Gt) && !self.check(&TokenKind::Eof) {
                        args.push(self.parse_type_annotation()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::Gt)?;
                    let args_str = args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
                    return Ok(TypeAnnotation::Custom(format!("{n}<{args_str}>")));
                }

                match n.as_str() {
                    "int" => Ok(TypeAnnotation::Int),
                    "float" => Ok(TypeAnnotation::Float),
                    "str" => Ok(TypeAnnotation::Str),
                    "bool" => Ok(TypeAnnotation::Bool),
                    "dyn" => Ok(TypeAnnotation::Dyn),
                    _ => Ok(TypeAnnotation::Custom(n)),
                }
            }
            TokenKind::LBracket => {
                self.advance();
                let inner = self.parse_type_annotation()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(TypeAnnotation::List(Box::new(inner)))
            }
            TokenKind::LBrace => {
                self.advance();
                let k = self.parse_type_annotation()?;
                self.expect(&TokenKind::Colon)?;
                let v = self.parse_type_annotation()?;
                self.expect(&TokenKind::RBrace)?;
                Ok(TypeAnnotation::Map(Box::new(k), Box::new(v)))
            }
            TokenKind::LParen => {
                self.advance();
                if self.match_token(&TokenKind::RParen) {
                    return Ok(TypeAnnotation::Unit);
                }
                let first = self.parse_type_annotation()?;
                if self.match_token(&TokenKind::Comma) {
                    let mut items = vec![first];
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        items.push(self.parse_type_annotation()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(TypeAnnotation::Tuple(items))
                } else {
                    self.expect(&TokenKind::RParen)?;
                    Ok(first)
                }
            }
            other => Err(format!(
                "Expected type annotation, found '{other}' at {line}:{col}",
                line = self.current().span.0,
                col = self.current().span.1
            )),
        }
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        let span = self.current().span;

        // 1. Indented block
        if self.match_token(&TokenKind::Indent) {
            let mut stmts = Vec::new();
            self.skip_newlines();
            while !self.check(&TokenKind::Dedent) && !self.check(&TokenKind::Eof) {
                let s = self.parse_stmt()?;
                stmts.push(s);
                self.skip_newlines();
            }
            self.expect(&TokenKind::Dedent)?;
            return Ok(Block { stmts, span });
        }

        // 2. Braced block '{ ... }'
        if self.match_token(&TokenKind::LBrace) {
            let mut stmts = Vec::new();
            self.skip_newlines();
            while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                let s = self.parse_stmt()?;
                stmts.push(s);
                self.skip_newlines();
            }
            self.expect(&TokenKind::RBrace)?;
            return Ok(Block { stmts, span });
        }

        // 3. Newline followed by indent
        if self.match_token(&TokenKind::Newline) {
            if self.match_token(&TokenKind::Indent) {
                let mut stmts = Vec::new();
                self.skip_newlines();
                while !self.check(&TokenKind::Dedent) && !self.check(&TokenKind::Eof) {
                    let s = self.parse_stmt()?;
                    stmts.push(s);
                    self.skip_newlines();
                }
                self.expect(&TokenKind::Dedent)?;
                return Ok(Block { stmts, span });
            }
        }

        // 4. Inline statement or expression
        let s = self.parse_stmt()?;
        Ok(Block {
            stmts: vec![s],
            span,
        })
    }

    pub fn parse_stmt(&mut self) -> Result<Stmt, String> {
        self.skip_newlines();
        let span = self.current().span;

        // RawRust
        if let TokenKind::RawRust(code) = self.peek() {
            let c = code.clone();
            self.advance();
            return Ok(Stmt::RawRust(c, span));
        }

        // Return
        if self.match_token(&TokenKind::Return) {
            let val = if self.check(&TokenKind::Newline)
                || self.check(&TokenKind::Semi)
                || self.check(&TokenKind::Dedent)
                || self.check(&TokenKind::RBrace)
            {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.match_token(&TokenKind::Semi);
            return Ok(Stmt::Return(val, span));
        }

        // Break
        if self.match_token(&TokenKind::Break) {
            self.match_token(&TokenKind::Semi);
            return Ok(Stmt::Break(span));
        }

        // Continue
        if self.match_token(&TokenKind::Continue) {
            self.match_token(&TokenKind::Semi);
            return Ok(Stmt::Continue(span));
        }

        // If statement
        if self.match_token(&TokenKind::If) {
            return self.parse_if_stmt(span);
        }

        // For loop
        if self.match_token(&TokenKind::For) {
            let item = self.parse_ident_name()?;
            self.expect(&TokenKind::In)?;
            let iter = self.parse_ternary()?;
            self.expect(&TokenKind::FatArrow)?;
            let body = self.parse_block()?;
            return Ok(Stmt::For {
                item,
                iter,
                body,
                span,
            });
        }

        // While loop
        if self.match_token(&TokenKind::While) {
            let cond = self.parse_ternary()?;
            self.expect(&TokenKind::FatArrow)?;
            let body = self.parse_block()?;
            return Ok(Stmt::While { cond, body, span });
        }

        // Match statement
        if self.check(&TokenKind::Match) {
            let (target, arms) = self.parse_match_common()?;
            return Ok(Stmt::Match { expr: target, arms, span });
        }

        // Expression or Assignment
        let expr = self.parse_expr()?;

        if let Expr::Assign { target, value, span: s } = expr {
            self.match_token(&TokenKind::Semi);
            return Ok(Stmt::Assign {
                target: *target,
                value: *value,
                op: None,
                is_decl: false,
                is_mut: false,
                ty: None,
                span: s,
            });
        }

        // Check for compound assignment operators: +=, -=, *=, /=, %=
        let op = match self.peek() {
            TokenKind::PlusEq => Some(BinaryOp::Add),
            TokenKind::MinusEq => Some(BinaryOp::Sub),
            TokenKind::StarEq => Some(BinaryOp::Mul),
            TokenKind::SlashEq => Some(BinaryOp::Div),
            TokenKind::PercentEq => Some(BinaryOp::Mod),
            _ => None,
        };

        if let Some(bin_op) = op {
            self.advance();
            let value = self.parse_expr()?;
            self.match_token(&TokenKind::Semi);
            return Ok(Stmt::Assign {
                target: expr,
                value,
                op: Some(bin_op),
                is_decl: false,
                is_mut: false,
                ty: None,
                span,
            });
        }

        self.match_token(&TokenKind::Semi);
        Ok(Stmt::Expr(expr))
    }

    fn parse_if_stmt(&mut self, span: (usize, usize)) -> Result<Stmt, String> {
        let cond = self.parse_ternary()?;
        self.expect(&TokenKind::FatArrow)?;
        let then_branch = self.parse_block()?;

        let mut elif_branches = Vec::new();
        let mut else_branch = None;

        self.skip_newlines();

        while self.match_token(&TokenKind::Elif) {
            let elif_cond = self.parse_ternary()?;
            self.expect(&TokenKind::FatArrow)?;
            let elif_body = self.parse_block()?;
            elif_branches.push((elif_cond, elif_body));
            self.skip_newlines();
        }

        if self.match_token(&TokenKind::Else) {
            self.expect(&TokenKind::FatArrow)?;
            else_branch = Some(self.parse_block()?);
        }

        Ok(Stmt::If {
            cond,
            then_branch,
            elif_branches,
            else_branch,
            span,
        })
    }

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment_expr()
    }

    fn parse_assignment_expr(&mut self) -> Result<Expr, String> {
        let left = self.parse_closure()?;

        if self.match_token(&TokenKind::Eq) {
            let span = left.span();
            let value = self.parse_assignment_expr()?;
            return Ok(Expr::Assign {
                target: Box::new(left),
                value: Box::new(value),
                span,
            });
        }

        Ok(left)
    }

    fn parse_closure(&mut self) -> Result<Expr, String> {
        if let TokenKind::Ident(param_name) = self.peek() {
            if self.peek_next() == &TokenKind::FatArrow {
                let span = self.current().span;
                let p = param_name.clone();
                self.advance(); // consume ident
                self.advance(); // consume =>
                let body = self.parse_assignment_expr()?;
                return Ok(Expr::Closure {
                    params: vec![p],
                    body: Box::new(body),
                    span,
                });
            }
        }

        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let cond = self.parse_pipeline()?;

        if self.match_token(&TokenKind::Question) {
            let span = cond.span();
            let then_expr = self.parse_expr()?;
            self.expect(&TokenKind::Colon)?;
            let else_expr = self.parse_expr()?;
            return Ok(Expr::Ternary {
                cond: Box::new(cond),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                span,
            });
        }

        Ok(cond)
    }

    fn parse_pipeline(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_range()?;

        while self.match_token(&TokenKind::Pipe) {
            let span = self.current().span;
            let right = self.parse_range()?;
            left = Expr::Pipe {
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_range(&mut self) -> Result<Expr, String> {
        let left = self.parse_logical_or()?;

        if self.match_token(&TokenKind::Range) {
            let span = left.span();
            let right = if self.is_expr_start() {
                Some(Box::new(self.parse_logical_or()?))
            } else {
                None
            };
            return Ok(Expr::Range {
                start: Some(Box::new(left)),
                end: right,
                inclusive: false,
                span,
            });
        }

        if self.match_token(&TokenKind::RangeInclusive) {
            let span = left.span();
            let right = Some(Box::new(self.parse_logical_or()?));
            return Ok(Expr::Range {
                start: Some(Box::new(left)),
                end: right,
                inclusive: true,
                span,
            });
        }

        Ok(left)
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_logical_and()?;

        while self.match_token(&TokenKind::OrOr) {
            let span = self.current().span;
            let right = self.parse_logical_and()?;
            left = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_or()?;

        while self.match_token(&TokenKind::AndAnd) {
            let span = self.current().span;
            let right = self.parse_bitwise_or()?;
            left = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_xor()?;

        while self.match_token(&TokenKind::Bar) {
            let span = self.current().span;
            let right = self.parse_bitwise_xor()?;
            left = Expr::Binary {
                op: BinaryOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_and()?;

        while self.match_token(&TokenKind::Caret) {
            let span = self.current().span;
            let right = self.parse_bitwise_and()?;
            left = Expr::Binary {
                op: BinaryOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;

        while self.match_token(&TokenKind::Amp) {
            let span = self.current().span;
            let right = self.parse_equality()?;
            left = Expr::Binary {
                op: BinaryOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;

        while let Some(op) = match self.peek() {
            TokenKind::EqEq => Some(BinaryOp::Eq),
            TokenKind::NotEq => Some(BinaryOp::Neq),
            _ => None,
        } {
            self.advance();
            let span = self.current().span;
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;

        while let Some(op) = match self.peek() {
            TokenKind::Lt => Some(BinaryOp::Lt),
            TokenKind::LtEq => Some(BinaryOp::Lte),
            TokenKind::Gt => Some(BinaryOp::Gt),
            TokenKind::GtEq => Some(BinaryOp::Gte),
            _ => None,
        } {
            self.advance();
            let span = self.current().span;
            let right = self.parse_additive()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;

        while let Some(op) = match self.peek() {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Sub),
            _ => None,
        } {
            self.advance();
            let span = self.current().span;
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        while let Some(op) = match self.peek() {
            TokenKind::Star => Some(BinaryOp::Mul),
            TokenKind::Slash => Some(BinaryOp::Div),
            TokenKind::Percent => Some(BinaryOp::Mod),
            _ => None,
        } {
            self.advance();
            let span = self.current().span;
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        let span = self.current().span;

        if self.match_token(&TokenKind::Minus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
                span,
            });
        }

        if self.match_token(&TokenKind::Bang) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                span,
            });
        }

        // Concurrency: `go expr`
        if self.match_token(&TokenKind::Go) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Go(Box::new(expr), span));
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            let span = self.current().span;

            // Postfix '!' error / await operator
            if self.match_token(&TokenKind::Bang) {
                expr = Expr::Try(Box::new(expr), span);
                continue;
            }

            // Postfix '?.' safe navigation
            if self.match_token(&TokenKind::SafeNav) {
                let member = self.parse_ident_name()?;
                expr = Expr::SafeNav {
                    object: Box::new(expr),
                    member,
                    span,
                };
                continue;
            }

            // Function Call: expr(...)
            if self.match_token(&TokenKind::LParen) {
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    args.push(self.parse_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
                continue;
            }

            // Indexing: expr[...]
            if self.match_token(&TokenKind::LBracket) {
                let index = self.parse_expr()?;
                self.expect(&TokenKind::RBracket)?;
                expr = Expr::Index {
                    collection: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
                continue;
            }

            // Type Cast: expr as target_type
            if self.match_token(&TokenKind::As) {
                let target_ty = self.parse_ident_name()?;
                expr = Expr::Cast {
                    expr: Box::new(expr),
                    target_ty,
                    span,
                };
                continue;
            }

            // Member access or method call: expr.member
            if self.match_token(&TokenKind::Dot) {
                let member = self.parse_ident_name()?;

                // If followed by '(', it's a method call
                if self.match_token(&TokenKind::LParen) {
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        args.push(self.parse_expr()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    expr = Expr::MethodCall {
                        receiver: Box::new(expr),
                        method: member,
                        args,
                        span,
                    };
                } else {
                    expr = Expr::MemberAccess {
                        object: Box::new(expr),
                        member,
                        span,
                    };
                }
                continue;
            }

            // Path access: expr::member
            if self.check(&TokenKind::Colon) && self.peek_next() == &TokenKind::Colon {
                self.advance();
                self.advance();
                let member = self.parse_ident_name()?;
                let mut segments = match expr {
                    Expr::PathAccess { segments, .. } => segments,
                    Expr::Ident(name, _) => vec![name],
                    _ => vec![],
                };
                segments.push(member);
                expr = Expr::PathAccess {
                    segments,
                    span,
                };
                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn parse_match_common(&mut self) -> Result<(Expr, Vec<MatchArm>), String> {
        let span = self.current().span;
        self.expect(&TokenKind::Match)?;
        let target = self.parse_expr()?;

        self.match_token(&TokenKind::FatArrow);
        self.skip_newlines();

        let is_braced = self.match_token(&TokenKind::LBrace);
        let is_indented = if !is_braced {
            self.match_token(&TokenKind::Indent)
        } else {
            false
        };

        let mut arms = Vec::new();
        while !self.check(&TokenKind::Eof) {
            self.skip_newlines();
            if is_braced && self.check(&TokenKind::RBrace) {
                break;
            }
            if is_indented && self.check(&TokenKind::Dedent) {
                break;
            }

            let arm_span = self.current().span;
            let pattern = self.parse_expr()?;

            let guard = if self.match_token(&TokenKind::If) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            self.expect(&TokenKind::FatArrow)?;
            self.skip_newlines();

            let body = if self.match_token(&TokenKind::Indent) {
                let mut stmts = Vec::new();
                while !self.check(&TokenKind::Dedent) && !self.check(&TokenKind::Eof) {
                    stmts.push(self.parse_stmt()?);
                    self.skip_newlines();
                }
                self.expect(&TokenKind::Dedent)?;
                Expr::Block(Block { stmts, span: arm_span }, arm_span)
            } else {
                self.parse_expr()?
            };

            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: arm_span,
            });

            self.match_token(&TokenKind::Comma);
            self.skip_newlines();

            if !is_braced && !is_indented {
                break;
            }
        }

        if is_braced {
            self.expect(&TokenKind::RBrace)?;
        } else if is_indented {
            self.expect(&TokenKind::Dedent)?;
        }

        Ok((target, arms))
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let span = self.current().span;

        match self.peek() {
            TokenKind::Match => {
                let (target, arms) = self.parse_match_common()?;
                Ok(Expr::Match {
                    expr: Box::new(target),
                    arms,
                    span,
                })
            }
            TokenKind::Int(val) => {
                let v = *val;
                self.advance();
                Ok(Expr::Literal(Literal::Int(v), span))
            }
            TokenKind::Float(val) => {
                let v = *val;
                self.advance();
                Ok(Expr::Literal(Literal::Float(v), span))
            }
            TokenKind::Str(val) => {
                let s = val.clone();
                self.advance();
                let lit = self.parse_string_interpolation(&s, span)?;
                Ok(Expr::Literal(lit, span))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true), span))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false), span))
            }
            TokenKind::None => {
                self.advance();
                Ok(Expr::Literal(Literal::None, span))
            }
            TokenKind::Ident(name) => {
                let n = name.clone();
                self.advance();
                Ok(Expr::Ident(n, span))
            }
            TokenKind::LParen => {
                self.advance();
                if self.match_token(&TokenKind::RParen) {
                    return Ok(Expr::Tuple(Vec::new(), span));
                }
                let first = self.parse_expr()?;
                if self.match_token(&TokenKind::Comma) {
                    let mut items = vec![first];
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        items.push(self.parse_expr()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(Expr::Tuple(items, span))
                } else {
                    self.expect(&TokenKind::RParen)?;
                    Ok(first)
                }
            }
            TokenKind::LBracket => {
                // List literal [a, b, c]
                self.advance();
                let mut elements = Vec::new();
                while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                    elements.push(self.parse_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::List(elements, span))
            }
            TokenKind::LBrace => {
                // Map literal {"a": 1, "b": 2} or Block
                self.advance();
                if self.match_token(&TokenKind::RBrace) {
                    // Empty map
                    return Ok(Expr::Map(Vec::new(), span));
                }

                // Check if this looks like a map or a block
                let key_expr = self.parse_expr()?;
                if self.match_token(&TokenKind::Colon) {
                    let val_expr = self.parse_expr()?;
                    let mut entries = vec![(key_expr, val_expr)];
                    while self.match_token(&TokenKind::Comma) {
                        if self.check(&TokenKind::RBrace) {
                            break;
                        }
                        let k = self.parse_expr()?;
                        self.expect(&TokenKind::Colon)?;
                        let v = self.parse_expr()?;
                        entries.push((k, v));
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Expr::Map(entries, span))
                } else {
                    // It's a block expression
                    let mut stmts = vec![Stmt::Expr(key_expr)];
                    self.skip_newlines();
                    while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                        stmts.push(self.parse_stmt()?);
                        self.skip_newlines();
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Expr::Block(Block { stmts, span }, span))
                }
            }
            TokenKind::RawRust(code) => {
                let c = code.clone();
                self.advance();
                Ok(Expr::RawRust(c, span))
            }
            other => Err(format!(
                "Unexpected token '{other}' in expression at {line}:{col}",
                line = span.0,
                col = span.1
            )),
        }
    }

    fn is_expr_start(&self) -> bool {
        matches!(
            self.peek(),
            TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::Str(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::None
                | TokenKind::Ident(_)
                | TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::LBrace
                | TokenKind::Minus
                | TokenKind::Bang
                | TokenKind::Go
        )
    }

    fn parse_ident_name(&mut self) -> Result<String, String> {
        let cur = self.current().clone();
        if let TokenKind::Ident(name) = cur.kind {
            self.advance();
            Ok(name)
        } else {
            Err(format!(
                "Expected identifier, found '{kind}' at {line}:{col}",
                kind = cur.kind,
                line = cur.span.0,
                col = cur.span.1
            ))
        }
    }

    fn parse_string_interpolation(
        &self,
        s: &str,
        _span: (usize, usize),
    ) -> Result<Literal, String> {
        if !s.contains('{') {
            return Ok(Literal::Str(s.to_string()));
        }

        let mut parts = Vec::new();
        let mut cur_lit = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // Check if escaped: \{ was unescaped to { during read_string, but if user wrote {{
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    cur_lit.push('{');
                    i += 2;
                    continue;
                }

                if !cur_lit.is_empty() {
                    parts.push(StringPart::Lit(cur_lit.clone()));
                    cur_lit.clear();
                }

                i += 1; // skip '{'
                let mut expr_str = String::new();
                let mut brace_depth = 1;

                while i < chars.len() {
                    if chars[i] == '{' {
                        brace_depth += 1;
                        expr_str.push(chars[i]);
                    } else if chars[i] == '}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            i += 1; // skip '}'
                            break;
                        }
                        expr_str.push(chars[i]);
                    } else {
                        expr_str.push(chars[i]);
                    }
                    i += 1;
                }

                // Lex & parse expr_str
                let mut sub_lexer = Lexer::new(&expr_str);
                let sub_tokens = sub_lexer.tokenize().map_err(|e| {
                    format!("Error tokenizing interpolated expression '{expr_str}' in string: {e}")
                })?;
                let mut sub_parser = Parser::new(sub_tokens);
                let parsed_expr = sub_parser.parse_expr().map_err(|e| {
                    format!("Error parsing interpolated expression '{expr_str}' in string: {e}")
                })?;
                parts.push(StringPart::Expr(parsed_expr));
            } else if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
                cur_lit.push('}');
                i += 2;
            } else {
                cur_lit.push(chars[i]);
                i += 1;
            }
        }

        if !cur_lit.is_empty() {
            parts.push(StringPart::Lit(cur_lit));
        }

        Ok(Literal::InterpolatedStr(parts))
    }
}
