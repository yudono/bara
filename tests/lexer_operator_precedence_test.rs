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
fn test_arithmetic_operators() {
    assert_eq!(
        kinds("+ - * / %"),
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
        ]
    );
}

#[test]
fn test_equality_operators() {
    assert_eq!(kinds("== !="), vec![TokenKind::EqEq, TokenKind::NotEq]);
}

#[test]
fn test_comparison_operators() {
    assert_eq!(
        kinds("< <= > >="),
        vec![
            TokenKind::Lt,
            TokenKind::LtEq,
            TokenKind::Gt,
            TokenKind::GtEq,
        ]
    );
}

#[test]
fn test_logical_operators() {
    assert_eq!(
        kinds("&& || & | ^"),
        vec![
            TokenKind::AndAnd,
            TokenKind::OrOr,
            TokenKind::Amp,
            TokenKind::Bar,
            TokenKind::Caret,
        ]
    );
}

#[test]
fn test_compound_assignment_operators() {
    assert_eq!(
        kinds("+= -= *= /= %="),
        vec![
            TokenKind::PlusEq,
            TokenKind::MinusEq,
            TokenKind::StarEq,
            TokenKind::SlashEq,
            TokenKind::PercentEq,
        ]
    );
}

#[test]
fn test_arrow_pipe_operators() {
    assert_eq!(
        kinds("=> -> |>"),
        vec![
            TokenKind::FatArrow,
            TokenKind::SkinnyArrow,
            TokenKind::Pipe,
        ]
    );
}

#[test]
fn test_range_and_safe_nav_operators() {
    assert_eq!(
        kinds(".. ..= ?."),
        vec![
            TokenKind::Range,
            TokenKind::RangeInclusive,
            TokenKind::SafeNav,
        ]
    );
}

#[test]
fn test_bang_question_as_operators() {
    assert_eq!(
        kinds("! ? as"),
        vec![TokenKind::Bang, TokenKind::Question, TokenKind::As]
    );
}

#[test]
fn test_full_operator_sequence_in_order() {
    let src = "+ - * / % == != < <= > >= && || & | ^ += -= *= /= %= => -> |> .. ..= ?. ! as";
    assert_eq!(
        kinds(src),
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::EqEq,
            TokenKind::NotEq,
            TokenKind::Lt,
            TokenKind::LtEq,
            TokenKind::Gt,
            TokenKind::GtEq,
            TokenKind::AndAnd,
            TokenKind::OrOr,
            TokenKind::Amp,
            TokenKind::Bar,
            TokenKind::Caret,
            TokenKind::PlusEq,
            TokenKind::MinusEq,
            TokenKind::StarEq,
            TokenKind::SlashEq,
            TokenKind::PercentEq,
            TokenKind::FatArrow,
            TokenKind::SkinnyArrow,
            TokenKind::Pipe,
            TokenKind::Range,
            TokenKind::RangeInclusive,
            TokenKind::SafeNav,
            TokenKind::Bang,
            TokenKind::As,
        ]
    );
}
