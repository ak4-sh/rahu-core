use super::{Span, Token, TokenKind};

#[test]
fn span_empty_when_start_equals_end() {
    assert!(Span::new(3, 3).is_empty());
}

#[test]
fn token_new_sets_kind() {
    let token = Token::new(0, 1, TokenKind::Name);
    let expected = Token::with_span(Span::new(0, 1), TokenKind::Name);

    assert_eq!(token, expected);
}
