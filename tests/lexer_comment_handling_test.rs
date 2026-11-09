use bara::lexer::{Lexer, TokenKind};

fn kinds(src: &str) -> Vec<TokenKind> {
    let mut lex = Lexer::new(src);
    let toks = lex.tokenize().expect("failed to tokenize");
    toks.into_iter()
        .map(|t| t.kind)
        .filter(|k| {
            !matches!(
                k,
                TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::Eof
            )
        })
        .collect()
}

#[test]
fn test_hash_line_comment_skipped() {
    assert_eq!(
        kinds("# just a comment"),
        Vec::<TokenKind>::new()
    );
}

#[test]
fn test_slashslash_line_comment_skipped() {
    assert_eq!(
        kinds("// just a comment"),
        Vec::<TokenKind>::new()
    );
}

#[test]
fn test_hash_comment_after_code() {
    assert_eq!(
        kinds("x = 1 # trailing comment"),
        vec![
            TokenKind::Ident("x".to_string()),
            TokenKind::Eq,
            TokenKind::Int(1),
        ]
    );
}

#[test]
fn test_slashslash_comment_after_code() {
    assert_eq!(
        kinds("x = 1 // trailing comment"),
        vec![
            TokenKind::Ident("x".to_string()),
            TokenKind::Eq,
            TokenKind::Int(1),
        ]
    );
}

#[test]
fn test_block_comment_inline_skipped() {
    assert_eq!(
        kinds("x = /* hidden */ 1"),
        vec![
            TokenKind::Ident("x".to_string()),
            TokenKind::Eq,
            TokenKind::Int(1),
        ]
    );
}

#[test]
fn test_block_comment_multiline_skipped() {
    let src = "x = 1 /* multi\nline\ncomment */ y = 2";
    let ks = kinds(src);
    assert_eq!(
        ks,
        vec![
            TokenKind::Ident("x".to_string()),
            TokenKind::Eq,
            TokenKind::Int(1),
            TokenKind::Ident("y".to_string()),
            TokenKind::Eq,
            TokenKind::Int(2),
        ]
    );
}

#[test]
fn test_jsx_comment_only_skipped() {
    assert_eq!(
        kinds("{/* JSX comment */}"),
        Vec::<TokenKind>::new()
    );
}

#[test]
fn test_jsx_comment_between_code_skipped() {
    let src = "x = 1\n{/* jsx comment */}\ny = 2";
    assert_eq!(
        kinds(src),
        vec![
            TokenKind::Ident("x".to_string()),
            TokenKind::Eq,
            TokenKind::Int(1),
            TokenKind::Ident("y".to_string()),
            TokenKind::Eq,
            TokenKind::Int(2),
        ]
    );
}

#[test]
fn test_comment_only_lines_leave_code_intact() {
    let src = "# first\ny = 2\n// third";
    assert_eq!(
        kinds(src),
        vec![
            TokenKind::Ident("y".to_string()),
            TokenKind::Eq,
            TokenKind::Int(2),
        ]
    );
}
