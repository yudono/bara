use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Type,
    If,
    Elif,
    Else,
    For,
    In,
    While,
    Return,
    Break,
    Continue,
    Use,
    Go,
    View,
    Enum,
    Match,
    Pub,
    Mod,
    RawRust(String),
    StyleBlock(String),
    True,
    False,
    None,
    As,

    // Operators
    FatArrow,       // =>
    SkinnyArrow,    // ->
    Pipe,           // |>
    SafeNav,        // ?.
    RangeInclusive, // ..=
    Range,          // ..
    PlusEq,         // +=
    MinusEq,        // -=
    StarEq,         // *=
    SlashEq,        // /=
    PercentEq,      // %=
    EqEq,           // ==
    NotEq,          // !=
    LtEq,           // <=
    GtEq,           // >=
    AndAnd,         // && or and
    OrOr,           // || or or
    Eq,             // =
    Lt,             // <
    Gt,             // >
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %
    Bang,           // ! or not
    Question,       // ?
    Amp,            // &
    Bar,            // |
    Caret,          // ^
    Tilde,          // ~
    LtSlash,        // </
    SlashGt,        // />

    // Delimiters
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }
    Colon,    // :
    Comma,    // ,
    Dot,      // .
    Semi,     // ;

    // Literals
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),

    // Layout
    Newline,
    Indent,
    Dedent,
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Type => write!(f, "type"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Elif => write!(f, "elif"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Use => write!(f, "use"),
            TokenKind::Go => write!(f, "go"),
            TokenKind::View => write!(f, "view"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Pub => write!(f, "pub"),
            TokenKind::Mod => write!(f, "mod"),
            TokenKind::RawRust(_) => write!(f, "raw_rust"),
            TokenKind::StyleBlock(_) => write!(f, "<style>...</style>"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::None => write!(f, "none"),
            TokenKind::As => write!(f, "as"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::SkinnyArrow => write!(f, "->"),
            TokenKind::Pipe => write!(f, "|>"),
            TokenKind::SafeNav => write!(f, "?."),
            TokenKind::RangeInclusive => write!(f, "..="),
            TokenKind::Range => write!(f, ".."),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::StarEq => write!(f, "*="),
            TokenKind::SlashEq => write!(f, "/="),
            TokenKind::PercentEq => write!(f, "%="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::NotEq => write!(f, "!="),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::AndAnd => write!(f, "&&"),
            TokenKind::OrOr => write!(f, "||"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Amp => write!(f, "&"),
            TokenKind::Bar => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::LtSlash => write!(f, "</"),
            TokenKind::SlashGt => write!(f, "/>"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Semi => write!(f, ";"),
            TokenKind::Ident(s) => write!(f, "ident({s})"),
            TokenKind::Int(i) => write!(f, "{i}"),
            TokenKind::Float(fl) => write!(f, "{fl}"),
            TokenKind::Str(s) => write!(f, "\"{s}\""),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Indent => write!(f, "INDENT"),
            TokenKind::Dedent => write!(f, "DEDENT"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: (usize, usize), // (line, col) 1-based
}

pub struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    _input: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            _input: input,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace_except_newline(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut raw_tokens: Vec<Token> = Vec::new();
        let mut bracket_depth = 0usize;

        while self.pos < self.chars.len() {
            let start_line = self.line;
            let start_col = self.col;
            let ch = match self.peek() {
                Some(c) => c,
                None => break,
            };

            // Single line comment # or //
            if ch == '#' || (ch == '/' && self.peek_next() == Some('/')) {
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            // JSX Comment: {/* ... */}
            if ch == '{' && self.peek_next() == Some('/') {
                if self.chars.get(self.pos + 2) == Some(&'*') {
                    self.advance(); // {
                    self.advance(); // /
                    self.advance(); // *
                    while self.pos < self.chars.len() {
                        if self.peek() == Some('*')
                            && self.chars.get(self.pos + 1) == Some(&'/')
                            && self.chars.get(self.pos + 2) == Some(&'}')
                        {
                            self.advance(); // *
                            self.advance(); // /
                            self.advance(); // }
                            break;
                        }
                        self.advance();
                    }
                    continue;
                }
            }

            // Multi-line comment /* ... */
            if ch == '/' && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                while self.pos < self.chars.len() {
                    if self.peek() == Some('*') && self.peek_next() == Some('/') {
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            // Style block: <style> ... </style>
            if ch == '<' {
                let remaining: String = self.chars[self.pos..].iter().take(6).collect();
                if remaining == "<style" {
                    self.advance(); // <
                    while let Some(c) = self.peek() {
                        self.advance();
                        if c == '>' {
                            break;
                        }
                    }
                    let mut css = String::new();
                    while self.pos < self.chars.len() {
                        let lookahead: String = self.chars[self.pos..].iter().take(8).collect();
                        if lookahead == "</style>" {
                            for _ in 0..8 {
                                self.advance();
                            }
                            break;
                        }
                        css.push(self.advance().unwrap());
                    }
                    raw_tokens.push(Token {
                        kind: TokenKind::StyleBlock(css),
                        span: (start_line, start_col),
                    });
                    continue;
                }
            }

            // Spaces and tabs
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            // Newline
            if ch == '\n' {
                self.advance();
                if bracket_depth == 0 {
                    // Avoid duplicate consecutive newlines
                    if let Some(last) = raw_tokens.last() {
                        if last.kind != TokenKind::Newline {
                            raw_tokens.push(Token {
                                kind: TokenKind::Newline,
                                span: (start_line, start_col),
                            });
                        }
                    }
                }
                continue;
            }

            // Track bracket depth so newlines inside brackets/parentheses are ignored
            if ch == '(' || ch == '[' || ch == '{' {
                bracket_depth += 1;
            } else if ch == ')' || ch == ']' || ch == '}' {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }

            // Numbers
            if ch.is_ascii_digit() {
                let tok = self.read_number(start_line, start_col)?;
                raw_tokens.push(tok);
                continue;
            }

            // Strings
            if ch == '"' || ch == '\'' {
                let tok = self.read_string(ch, start_line, start_col)?;
                raw_tokens.push(tok);
                continue;
            }

            // Identifiers / Keywords
            if ch.is_alphabetic() || ch == '_' {
                let ident = self.read_ident();

                // Special handling for raw_rust { ... }
                if ident == "raw_rust" {
                    self.skip_whitespace_except_newline();
                    if self.peek() == Some('{') {
                        self.advance(); // consume '{'
                        let mut depth = 1;
                        let mut body = String::new();
                        while self.pos < self.chars.len() {
                            let c = self.advance().unwrap();
                            if c == '{' {
                                depth += 1;
                                body.push(c);
                            } else if c == '}' {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                body.push(c);
                            } else {
                                body.push(c);
                            }
                        }
                        raw_tokens.push(Token {
                            kind: TokenKind::RawRust(body),
                            span: (start_line, start_col),
                        });
                        continue;
                    }
                }

                let kind = match ident.as_str() {
                    "type" => TokenKind::Type,
                    "if" => TokenKind::If,
                    "elif" => TokenKind::Elif,
                    "else" => TokenKind::Else,
                    "for" => TokenKind::For,
                    "in" => TokenKind::In,
                    "while" => TokenKind::While,
                    "return" => TokenKind::Return,
                    "break" => TokenKind::Break,
                    "continue" => TokenKind::Continue,
                    "use" => TokenKind::Use,
                    "enum" => TokenKind::Enum,
                    "match" => TokenKind::Match,
                    "pub" => TokenKind::Pub,
                    "mod" => TokenKind::Mod,
                    "go" => TokenKind::Go,
                    "view" => TokenKind::View,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "none" => TokenKind::None,
                    "as" => TokenKind::As,
                    "and" => TokenKind::AndAnd,
                    "or" => TokenKind::OrOr,
                    "not" => TokenKind::Bang,
                    _ => TokenKind::Ident(ident),
                };
                raw_tokens.push(Token {
                    kind,
                    span: (start_line, start_col),
                });
                continue;
            }

            // Multi-character and single-character operators
            self.advance();
            let span = (start_line, start_col);
            let next = self.peek();

            let kind = match ch {
                '=' => {
                    if next == Some('>') {
                        self.advance();
                        TokenKind::FatArrow
                    } else if next == Some('=') {
                        self.advance();
                        TokenKind::EqEq
                    } else {
                        TokenKind::Eq
                    }
                }
                '-' => {
                    if next == Some('>') {
                        self.advance();
                        TokenKind::SkinnyArrow
                    } else if next == Some('=') {
                        self.advance();
                        TokenKind::MinusEq
                    } else {
                        TokenKind::Minus
                    }
                }
                '+' => {
                    if next == Some('=') {
                        self.advance();
                        TokenKind::PlusEq
                    } else {
                        TokenKind::Plus
                    }
                }
                '*' => {
                    if next == Some('=') {
                        self.advance();
                        TokenKind::StarEq
                    } else {
                        TokenKind::Star
                    }
                }
                '/' => {
                    if next == Some('>') {
                        self.advance();
                        TokenKind::SlashGt
                    } else if next == Some('=') {
                        self.advance();
                        TokenKind::SlashEq
                    } else {
                        TokenKind::Slash
                    }
                }
                '%' => {
                    if next == Some('=') {
                        self.advance();
                        TokenKind::PercentEq
                    } else {
                        TokenKind::Percent
                    }
                }
                '!' => {
                    if next == Some('=') {
                        self.advance();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }
                '<' => {
                    if next == Some('/') {
                        self.advance();
                        TokenKind::LtSlash
                    } else if next == Some('=') {
                        self.advance();
                        TokenKind::LtEq
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    if next == Some('=') {
                        self.advance();
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }
                '|' => {
                    if next == Some('>') {
                        self.advance();
                        TokenKind::Pipe
                    } else if next == Some('|') {
                        self.advance();
                        TokenKind::OrOr
                    } else {
                        TokenKind::Bar
                    }
                }
                '&' => {
                    if next == Some('&') {
                        self.advance();
                        TokenKind::AndAnd
                    } else {
                        TokenKind::Amp
                    }
                }
                '?' => {
                    if next == Some('.') {
                        self.advance();
                        TokenKind::SafeNav
                    } else {
                        TokenKind::Question
                    }
                }
                '.' => {
                    if next == Some('.') {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::RangeInclusive
                        } else {
                            TokenKind::Range
                        }
                    } else {
                        TokenKind::Dot
                    }
                }
                '^' => TokenKind::Caret,
                '~' => TokenKind::Tilde,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '[' => TokenKind::LBracket,
                ']' => TokenKind::RBracket,
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                ':' => TokenKind::Colon,
                ',' => TokenKind::Comma,
                ';' => TokenKind::Semi,
                _ => {
                    let mut s = String::new();
                    s.push(ch);
                    while let Some(c) = self.peek() {
                        if !c.is_whitespace()
                            && !matches!(
                                c,
                                '<' | '>'
                                    | '{'
                                    | '}'
                                    | '('
                                    | ')'
                                    | '['
                                    | ']'
                                    | ':'
                                    | ','
                                    | ';'
                                    | '='
                                    | '+'
                                    | '-'
                                    | '*'
                                    | '/'
                                    | '%'
                                    | '!'
                                    | '?'
                                    | '&'
                                    | '|'
                                    | '^'
                                    | '~'
                                    | '"'
                                    | '\''
                            )
                        {
                            s.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    TokenKind::Ident(s)
                }
            };

            raw_tokens.push(Token { kind, span });
        }

        // Apply indentation processing on the token stream
        self.apply_layout(raw_tokens)
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn read_number(&mut self, line: usize, col: usize) -> Result<Token, String> {
        let mut s = String::new();
        let mut is_float = false;

        if self.peek() == Some('0') {
            s.push(self.advance().unwrap());
            if self.peek() == Some('x') || self.peek() == Some('X') {
                s.push(self.advance().unwrap());
                while let Some(c) = self.peek() {
                    if c.is_ascii_hexdigit() || c == '_' {
                        s.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let clean = s[2..].replace('_', "");
                let val = i64::from_str_radix(&clean, 16)
                    .map_err(|e| format!("Invalid hex literal '{s}' at {line}:{col}: {e}"))?;
                return Ok(Token {
                    kind: TokenKind::Int(val),
                    span: (line, col),
                });
            } else if self.peek() == Some('b') || self.peek() == Some('B') {
                s.push(self.advance().unwrap());
                while let Some(c) = self.peek() {
                    if c == '0' || c == '1' || c == '_' {
                        s.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let clean = s[2..].replace('_', "");
                let val = i64::from_str_radix(&clean, 2)
                    .map_err(|e| format!("Invalid binary literal '{s}' at {line}:{col}: {e}"))?;
                return Ok(Token {
                    kind: TokenKind::Int(val),
                    span: (line, col),
                });
            }
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                s.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                // Peek ahead to ensure it's not `..` range operator
                if self.peek_next() == Some('.') {
                    break;
                }
                is_float = true;
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let clean = s.replace('_', "");
        if is_float {
            let val = clean
                .parse::<f64>()
                .map_err(|e| format!("Invalid float literal '{s}' at {line}:{col}: {e}"))?;
            Ok(Token {
                kind: TokenKind::Float(val),
                span: (line, col),
            })
        } else {
            let val = clean
                .parse::<i64>()
                .map_err(|e| format!("Invalid integer literal '{s}' at {line}:{col}: {e}"))?;
            Ok(Token {
                kind: TokenKind::Int(val),
                span: (line, col),
            })
        }
    }

    fn read_string(&mut self, quote: char, line: usize, col: usize) -> Result<Token, String> {
        self.advance(); // consume opening quote
        let mut s = String::new();

        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance(); // consume closing quote
                return Ok(Token {
                    kind: TokenKind::Str(s),
                    span: (line, col),
                });
            } else if ch == '\\' {
                self.advance();
                let escaped = match self.advance() {
                    Some('n') => '\n',
                    Some('r') => '\r',
                    Some('t') => '\t',
                    Some('\\') => '\\',
                    Some('\'') => '\'',
                    Some('"') => '"',
                    Some('{') => '{',
                    Some('}') => '}',
                    Some(other) => other,
                    None => return Err(format!("Unterminated escape sequence in string at {line}:{col}")),
                };
                s.push(escaped);
            } else {
                s.push(ch);
                self.advance();
            }
        }

        Err(format!("Unterminated string literal at {line}:{col}"))
    }

    /// Process indentation / layout from source lines and raw tokens
    fn apply_layout(&self, raw_tokens: Vec<Token>) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        let mut indent_stack: Vec<usize> = vec![0];

        // Group tokens by line
        // We look at the actual source line indents
        let lines: Vec<&str> = self._input.lines().collect();

        let mut tok_idx = 0;
        let mut current_line_num = 1;
        let mut bracket_depth = 0;

        while tok_idx < raw_tokens.len() {
            let tok = &raw_tokens[tok_idx];

            if tok.kind == TokenKind::Newline {
                tok_idx += 1;
                continue;
            }

            let tok_line = tok.span.0;

            // If we moved to a new line, check indentation (only when not inside brackets)
            if tok_line > current_line_num {
                if bracket_depth == 0 {
                    // Find line content
                    if tok_line - 1 < lines.len() {
                        let line_str = lines[tok_line - 1];
                        let indent = line_str
                            .chars()
                            .take_while(|c| *c == ' ' || *c == '\t')
                            .map(|c| if c == '\t' { 4 } else { 1 })
                            .sum::<usize>();

                        let current_indent = *indent_stack.last().unwrap_or(&0);
                        if indent > current_indent {
                            indent_stack.push(indent);
                            tokens.push(Token {
                                kind: TokenKind::Indent,
                                span: (tok_line, 1),
                            });
                        } else if indent < current_indent {
                            while let Some(&top) = indent_stack.last() {
                                if indent < top {
                                    indent_stack.pop();
                                    tokens.push(Token {
                                        kind: TokenKind::Dedent,
                                        span: (tok_line, 1),
                                    });
                                } else {
                                    break;
                                }
                            }
                        } else {
                            // Same indentation level on a new line: emit Newline between statements
                            if let Some(prev) = tokens.last() {
                                if prev.kind != TokenKind::Newline
                                    && prev.kind != TokenKind::Indent
                                    && prev.kind != TokenKind::Dedent
                                {
                                    tokens.push(Token {
                                        kind: TokenKind::Newline,
                                        span: (tok_line, 1),
                                    });
                                }
                            }
                        }
                    }
                }
                current_line_num = tok_line;
            }

            match &tok.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    bracket_depth += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    if bracket_depth > 0 {
                        bracket_depth -= 1;
                    }
                }
                _ => {}
            }

            tokens.push(tok.clone());
            tok_idx += 1;
        }

        // Pop remaining indentation
        while indent_stack.len() > 1 {
            indent_stack.pop();
            tokens.push(Token {
                kind: TokenKind::Dedent,
                span: (current_line_num, 1),
            });
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: (current_line_num, 1),
        });

        Ok(tokens)
    }
}
