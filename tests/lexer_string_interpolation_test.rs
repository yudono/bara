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

fn first_str(src: &str) -> String {
    let ks = kinds(src);
    assert_eq!(ks.len(), 1, "expected single token for {src:?}, got {ks:?}");
    match &ks[0] {
        TokenKind::Str(s) => s.clone(),
        other => panic!("expected Str for {src:?}, got {other:?}"),
    }
}

#[test]
fn test_double_quoted_basic() {
    assert_eq!(first_str("\"hello\""), "hello".to_string());
}

#[test]
fn test_single_quoted_basic() {
    assert_eq!(first_str("'hello'"), "hello".to_string());
}

#[test]
fn test_empty_double_string() {
    assert_eq!(first_str("\"\""), String::new());
}

#[test]
fn test_empty_single_string() {
    assert_eq!(first_str("''"), String::new());
}

#[test]
fn test_escape_newline() {
    // bara source "a\nb" should decode to a real newline
    assert_eq!(first_str("\"a\\nb\""), "a\nb".to_string());
}

#[test]
fn test_escape_tab() {
    assert_eq!(first_str("\"a\\tb\""), "a\tb".to_string());
}

#[test]
fn test_escape_backslash() {
    assert_eq!(first_str("\"a\\\\b\""), "a\\b".to_string());
}

#[test]
fn test_interpolation_braces_preserved_double() {
    assert_eq!(
        first_str("\"hello {name}\""),
        "hello {name}".to_string()
    );
}

#[test]
fn test_interpolation_expr_preserved_single() {
    assert_eq!(
        first_str("'sum is {1 + 2}'"),
        "sum is {1 + 2}".to_string()
    );
}
