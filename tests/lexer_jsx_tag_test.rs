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
fn test_lt_token() {
    assert_eq!(kinds("<"), vec![TokenKind::Lt]);
}

#[test]
fn test_lt_slash_token() {
    assert_eq!(kinds("</"), vec![TokenKind::LtSlash]);
}

#[test]
fn test_slash_gt_token() {
    assert_eq!(kinds("/>"), vec![TokenKind::SlashGt]);
}

#[test]
fn test_panel_open_tag_start() {
    assert_eq!(
        kinds("<Panel"),
        vec![
            TokenKind::Lt,
            TokenKind::Ident("Panel".to_string()),
        ]
    );
}

#[test]
fn test_style_block_single_token() {
    let ks = kinds("<style>.a { color: red; }</style>");
    assert_eq!(ks.len(), 1, "expected single StyleBlock, got {ks:?}");
    match &ks[0] {
        TokenKind::StyleBlock(css) => {
            assert!(css.contains(".a"), "css should contain selector, got {css:?}");
            assert!(css.contains("color: red"), "css should contain rule, got {css:?}");
        }
        other => panic!("expected StyleBlock, got {other:?}"),
    }
}

#[test]
fn test_attribute_string_lexed_as_str() {
    let ks = kinds("<Panel title=\"hello\">");
    assert_eq!(
        ks,
        vec![
            TokenKind::Lt,
            TokenKind::Ident("Panel".to_string()),
            TokenKind::Ident("title".to_string()),
            TokenKind::Eq,
            TokenKind::Str("hello".to_string()),
            TokenKind::Gt,
        ]
    );
}

#[test]
fn test_jsx_open_close_pair() {
    let ks = kinds("<div> </div>");
    assert_eq!(
        ks,
        vec![
            TokenKind::Lt,
            TokenKind::Ident("div".to_string()),
            TokenKind::Gt,
            TokenKind::LtSlash,
            TokenKind::Ident("div".to_string()),
            TokenKind::Gt,
        ]
    );
}

#[test]
fn test_self_closing_tag() {
    let ks = kinds("<br />");
    assert_eq!(
        ks,
        vec![
            TokenKind::Lt,
            TokenKind::Ident("br".to_string()),
            TokenKind::SlashGt,
        ]
    );
}

#[test]
fn test_style_block_with_attributes() {
    let ks = kinds("<style type=\"text/css\">p { margin: 0; }</style>");
    assert_eq!(ks.len(), 1, "expected single StyleBlock, got {ks:?}");
    match &ks[0] {
        TokenKind::StyleBlock(css) => {
            assert!(css.contains("margin: 0"), "got {css:?}");
        }
        other => panic!("expected StyleBlock, got {other:?}"),
    }
}
