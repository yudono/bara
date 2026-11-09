#[cfg(test)]
mod tests {
    use bara::lexer::{Lexer, TokenKind};

    #[test]
    fn test_lex_basic() {
        let input = "x = 10\nx += 5";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("failed to tokenize");
        assert_eq!(tokens[0].kind, TokenKind::Ident("x".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Eq);
        assert_eq!(tokens[2].kind, TokenKind::Int(10));
        assert_eq!(tokens[3].kind, TokenKind::Newline);
        assert_eq!(tokens[4].kind, TokenKind::Ident("x".to_string()));
        assert_eq!(tokens[5].kind, TokenKind::PlusEq);
        assert_eq!(tokens[6].kind, TokenKind::Int(5));
    }

    #[test]
    fn test_lex_indentation() {
        let input = "proses(a, b) =>\n    b.push(a)\n    a * 2";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("failed to tokenize");
        // Verify Indent and Dedent are emitted
        let has_indent = tokens.iter().any(|t| t.kind == TokenKind::Indent);
        let has_dedent = tokens.iter().any(|t| t.kind == TokenKind::Dedent);
        assert!(has_indent, "Expected INDENT token");
        assert!(has_dedent, "Expected DEDENT token");
    }

    #[test]
    fn test_lex_raw_rust() {
        let input = "raw_rust { let mut foo = 42; println!(\"{}\", foo); }";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("failed to tokenize");
        assert!(matches!(tokens[0].kind, TokenKind::RawRust(_)));
    }
}
