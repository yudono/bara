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

fn single(src: &str) -> TokenKind {
    let ks = kinds(src);
    assert_eq!(ks.len(), 1, "expected single token for {src:?}, got {ks:?}");
    ks.into_iter().next().unwrap()
}

#[test]
fn test_plain_int() {
    assert_eq!(single("42"), TokenKind::Int(42));
}

#[test]
fn test_zero_int() {
    assert_eq!(single("0"), TokenKind::Int(0));
}

#[test]
fn test_int_with_underscores() {
    assert_eq!(single("1_000"), TokenKind::Int(1000));
}

#[test]
fn test_large_underscored_int() {
    assert_eq!(single("1_000_000"), TokenKind::Int(1_000_000));
}

#[test]
fn test_simple_float() {
    match single("3.14") {
        TokenKind::Float(v) => assert!((v - 3.14).abs() < 1e-9, "got {v}"),
        other => panic!("expected Float, got {other:?}"),
    }
}

#[test]
fn test_float_with_underscores() {
    match single("1_000.5") {
        TokenKind::Float(v) => assert!((v - 1000.5).abs() < 1e-9, "got {v}"),
        other => panic!("expected Float, got {other:?}"),
    }
}

#[test]
fn test_hex_lower() {
    assert_eq!(single("0xff"), TokenKind::Int(255));
}

#[test]
fn test_hex_upper() {
    assert_eq!(single("0xFF"), TokenKind::Int(255));
}

#[test]
fn test_binary_literal() {
    assert_eq!(single("0b1010"), TokenKind::Int(10));
}

#[test]
fn test_binary_with_underscores() {
    assert_eq!(single("0b1010_0101"), TokenKind::Int(0b1010_0101));
}
