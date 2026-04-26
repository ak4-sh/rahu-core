use super::{LexDiagKind, Lexer, Trivia};
use crate::tokens::{Token, TokenKind};

fn lex_kinds(src: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(src);
    let mut kinds = Vec::new();

    loop {
        let tok = lexer.next_token();
        kinds.push(tok.kind);
        if tok.kind == TokenKind::Eof {
            break;
        }
    }

    kinds
}

fn lex_tokens(src: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();

    loop {
        let tok = lexer.next_token();
        tokens.push(tok);
        if tok.kind == TokenKind::Eof {
            break;
        }
    }

    tokens
}

fn lex_tokens_and_diags(src: &str) -> (Vec<Token>, Vec<LexDiagKind>) {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();

    loop {
        let tok = lexer.next_token();
        tokens.push(tok);
        if tok.kind == TokenKind::Eof {
            break;
        }
    }

    let diags = lexer
        .take_diagnostics()
        .into_iter()
        .map(|diag| diag.kind)
        .collect();

    (tokens, diags)
}

fn lex_tokens_trivia_and_diags(src: &str) -> (Vec<Token>, Vec<Trivia>, Vec<LexDiagKind>) {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();

    loop {
        let tok = lexer.next_token();
        tokens.push(tok);
        if tok.kind == TokenKind::Eof {
            break;
        }
    }

    let trivia = lexer.take_trivia();
    let diags = lexer
        .take_diagnostics()
        .into_iter()
        .map(|diag| diag.kind)
        .collect();

    (tokens, trivia, diags)
}

#[test]
fn lexes_assignment() {
    let kinds = lex_kinds("x = 42\n");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Newline,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_comment() {
    let kinds = lex_kinds("#this is a dummy comment\nprint(sdf)");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Newline,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::Name,
            TokenKind::RightParen,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn captures_comment_only_line() {
    let src = "# hello\nx = 1";
    let (tokens, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].span.slice(src), "# hello");
    assert_eq!(trivia[0].precedes_token, 0);
    assert_eq!(tokens[0].kind, TokenKind::Newline);
}

#[test]
fn captures_inline_comment() {
    let src = "x = 1 # comment\n";
    let (tokens, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].span.slice(src), "# comment");
    assert_eq!(trivia[0].precedes_token, 3);
    assert_eq!(
        tokens[trivia[0].precedes_token as usize].kind,
        TokenKind::Newline
    );
}

#[test]
fn captures_inline_type_comment() {
    let src = "x = 1  # type: int\n";
    let (tokens, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].span.slice(src), "# type: int");
    assert!(!trivia[0].is_type_ignore(src));
    assert!(trivia[0].is_type_comment(src));
    assert_eq!(
        tokens[trivia[0].precedes_token as usize].kind,
        TokenKind::Newline
    );
}

#[test]
fn captures_comment_at_eof() {
    let src = "x = 1 # comment";
    let (tokens, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].span.slice(src), "# comment");
    assert_eq!(trivia[0].precedes_token, 3);
    assert_eq!(
        tokens[trivia[0].precedes_token as usize].kind,
        TokenKind::Eof
    );
}

#[test]
fn captures_comment_before_crlf_newline() {
    let src = "x = 1 # comment\r\n";
    let (tokens, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].span.slice(src), "# comment");
    assert_eq!(
        tokens[trivia[0].precedes_token as usize].kind,
        TokenKind::Newline
    );
}

#[test]
fn tracks_next_token_index_for_comment_attachment() {
    let src = "x = 1 # comment";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].precedes_token, 3);
}

#[test]
fn lexes_fstring_simple() {
    let kinds = lex_kinds("x = 10\nprint(f\"this is {x}\")");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Newline,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::FStringStart,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::RightParen,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn does_not_capture_hash_inside_fstring_text() {
    let src = "f'# not comment {x}'";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(trivia.is_empty());
}

#[test]
fn does_not_capture_hash_inside_double_quoted_fstring_text() {
    let src = "f\"# not trivia\"";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(trivia.is_empty());
}

#[test]
fn captures_comment_inside_fstring_expr() {
    let src = "f'''{x # comment\n}'''";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert_eq!(trivia.len(), 1);
    assert_eq!(trivia[0].span.slice(src), "# comment");
}

#[test]
fn detects_type_ignore_comment() {
    let src = "# type: ignore";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(trivia[0].is_type_ignore(src));
    assert!(!trivia[0].is_type_comment(src));
}

#[test]
fn detects_spaced_type_ignore_comment() {
    let src = "#    type: ignore";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(trivia[0].is_type_ignore(src));
    assert!(!trivia[0].is_type_comment(src));
}

#[test]
fn detects_bracketed_type_ignore_comment() {
    let src = "# type: ignore[attr-defined]";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(trivia[0].is_type_ignore(src));
    assert!(!trivia[0].is_type_comment(src));
}

#[test]
fn detects_type_comment() {
    let src = "# type: int";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(!trivia[0].is_type_ignore(src));
    assert!(trivia[0].is_type_comment(src));
}

#[test]
fn rejects_non_type_comment() {
    let src = "# not a type: comment";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(!trivia[0].is_type_ignore(src));
    assert!(!trivia[0].is_type_comment(src));
}

#[test]
fn rejects_empty_type_comment() {
    let src = "# type:";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(!trivia[0].is_type_ignore(src));
    assert!(!trivia[0].is_type_comment(src));
}

#[test]
fn rejects_whitespace_only_type_comment() {
    let src = "# type:   ";
    let (_, trivia, _) = lex_tokens_trivia_and_diags(src);

    assert!(!trivia[0].is_type_ignore(src));
    assert!(!trivia[0].is_type_comment(src));
}

#[test]
fn lexes_fstring_conversion_tokens() {
    let (tokens, diags) = lex_tokens_and_diags("f\"{x!r}\"");
    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Exclamation,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn lexes_fstring_format_spec_text_as_middle() {
    let kinds = lex_kinds("f\"{x:.2f}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::FStringMiddle,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_fstring_conversion_then_format_spec() {
    let kinds = lex_kinds("f\"{x!r:>10}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Exclamation,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::FStringMiddle,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_nested_expressions_inside_format_spec() {
    let kinds = lex_kinds("f\"{x:{width}.{precision}f}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_nested_format_spec_with_inner_format_spec() {
    let (tokens, diags) = lex_tokens_and_diags("f\"{x: .{precision:d}f}\"");

    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::FStringMiddle,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn lexes_fstring_debug_equal_before_format_spec() {
    let kinds = lex_kinds("f\"{x=:.2f}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Colon,
            TokenKind::FStringMiddle,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_fstring_debug_equal_without_format_spec() {
    let kinds = lex_kinds("f\"{x=}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn repeated_fstring_exclamation_tokens_are_left_for_parser() {
    let (tokens, diags) = lex_tokens_and_diags("f\"{x!!r}\"");
    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Exclamation,
            TokenKind::Exclamation,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn repeated_fstring_conversion_markers_are_left_for_parser() {
    let (tokens, diags) = lex_tokens_and_diags("f\"{x!r!s}\"");
    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Exclamation,
            TokenKind::Name,
            TokenKind::Exclamation,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn top_level_fstring_markers_do_not_override_nested_expression_tokens() {
    let kinds = lex_kinds("f\"{ {'a': 1} == y}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::LeftBrace,
            TokenKind::String,
            TokenKind::Colon,
            TokenKind::Number,
            TokenKind::RightBrace,
            TokenKind::EqualEqual,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn fstring_nested_not_equal_remains_normal_expression_token() {
    let kinds = lex_kinds("f\"{x != y}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::NotEqual,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn fstring_nested_bare_exclamation_stays_invalid() {
    let (tokens, diags) = lex_tokens_and_diags("f\"{foo(!x)}\"");
    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::Illegal,
            TokenKind::Name,
            TokenKind::RightParen,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert_eq!(diags, vec![LexDiagKind::UnexpectedCharacter]);
}

#[test]
fn lexes_while_loop() {
    let kinds = lex_kinds("x = 3\nfor i in range(0, x):\n    print(i)");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Newline,
            TokenKind::For,
            TokenKind::Name,
            TokenKind::In,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::Number,
            TokenKind::Comma,
            TokenKind::Name,
            TokenKind::RightParen,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::Name,
            TokenKind::RightParen,
            TokenKind::Dedent,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_mixed_indentation() {
    let kinds = lex_kinds("if x:\n\t pass");
    assert_eq!(
        kinds,
        vec![
            TokenKind::If,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::Pass,
            TokenKind::Dedent,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_hex_binary_octal_numbers() {
    let kinds = lex_kinds("0x1F 0b1010 0o755");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_float_numbers() {
    let kinds = lex_kinds("3.14 0.5 10.0 .5");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_simple_strings() {
    let kinds = lex_kinds("'hello' \"world\"");
    assert_eq!(
        kinds,
        vec![TokenKind::String, TokenKind::String, TokenKind::Eof,]
    )
}

#[test]
fn lexes_strings_with_escapes() {
    let kinds = lex_kinds("'hello\\nworld' \"tab\\there\"");
    assert_eq!(
        kinds,
        vec![TokenKind::String, TokenKind::String, TokenKind::Eof,]
    )
}

#[test]
fn lexes_raw_strings() {
    let kinds = lex_kinds("r'raw\\nstring' R\"double\"");
    assert_eq!(
        kinds,
        vec![TokenKind::String, TokenKind::String, TokenKind::Eof,]
    )
}

#[test]
fn lexes_byte_strings() {
    let kinds = lex_kinds("b'bytes' b\"data\"");
    assert_eq!(
        kinds,
        vec![TokenKind::BString, TokenKind::BString, TokenKind::Eof,]
    )
}

#[test]
fn lexes_multiline_strings() {
    let kinds = lex_kinds("'''triple\nquoted''' \"\"\"also triple\"\"\"");
    assert_eq!(
        kinds,
        vec![TokenKind::String, TokenKind::String, TokenKind::Eof,]
    )
}

#[test]
fn lexes_triple_string_with_internal_quote_candidates() {
    let kinds = lex_kinds("\"\"\"a\"b\"\"c\"\"\"");
    assert_eq!(kinds, vec![TokenKind::String, TokenKind::Eof]);
}

#[test]
fn triple_string_ignores_escaped_triple_quote() {
    let kinds = lex_kinds("\"\"\"abc \\\"\\\"\\\" def\"\"\"");
    assert_eq!(kinds, vec![TokenKind::String, TokenKind::Eof]);
}

#[test]
fn triple_string_closes_after_backslash() {
    let kinds = lex_kinds("\"\"\"abc \\\"\"\"");
    assert_eq!(kinds, vec![TokenKind::String, TokenKind::Eof]);
}

#[test]
fn lexes_raw_multiline_strings() {
    let kinds = lex_kinds("r'''raw\ntriple''' r\"\"\"raw triple\"\"\"");
    assert_eq!(
        kinds,
        vec![TokenKind::String, TokenKind::String, TokenKind::Eof,]
    )
}

#[test]
fn lexes_raw_triple_string_with_escaped_quote_candidates() {
    let kinds = lex_kinds("r\"\"\"keep \\\"\" not closed yet \"\"\"");
    assert_eq!(kinds, vec![TokenKind::String, TokenKind::Eof]);
}

#[test]
fn raw_triple_string_closes_after_backslash() {
    let kinds = lex_kinds("r\"\"\"abc \\\"\"\"");
    assert_eq!(kinds, vec![TokenKind::String, TokenKind::Eof]);
}

#[test]
fn lexes_unterminated_strings() {
    let kinds = lex_kinds("'unterminated");
    assert_eq!(kinds, vec![TokenKind::UnterminatedString, TokenKind::Eof,])
}

#[test]
fn lexes_compound_assignment_operators() {
    let kinds = lex_kinds("+= -= *= /= //= %= **=");
    assert_eq!(
        kinds,
        vec![
            TokenKind::PlusEqual,
            TokenKind::MinusEqual,
            TokenKind::StarEqual,
            TokenKind::SlashEqual,
            TokenKind::DoubleSlashEqual,
            TokenKind::PercentEqual,
            TokenKind::DoubleStarEqual,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_bitwise_operators() {
    let kinds = lex_kinds("& | ^ ~ << >> &= |= ^= <<= >>=");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Ampersand,
            TokenKind::Pipe,
            TokenKind::Circumflex,
            TokenKind::Tilde,
            TokenKind::LeftShift,
            TokenKind::RightShift,
            TokenKind::AmpersandEqual,
            TokenKind::PipeEqual,
            TokenKind::CircumflexEqual,
            TokenKind::LeftShiftEqual,
            TokenKind::RightShiftEqual,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_comparison_operators() {
    let kinds = lex_kinds("== != < > <= >=");
    assert_eq!(
        kinds,
        vec![
            TokenKind::EqualEqual,
            TokenKind::NotEqual,
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::LessEqual,
            TokenKind::GreaterEqual,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_other_operators() {
    let kinds = lex_kinds("// ** -> := @ @= ... !");
    assert_eq!(
        kinds,
        vec![
            TokenKind::DoubleSlash,
            TokenKind::DoubleStar,
            TokenKind::Arrow,
            TokenKind::ColonEqual,
            TokenKind::At,
            TokenKind::AtEqual,
            TokenKind::Ellipsis,
            TokenKind::Illegal,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_keywords() {
    let kinds = lex_kinds(
        "class def return if else elif for while break continue pass and or not in is None True False",
    );
    assert_eq!(
        kinds,
        vec![
            TokenKind::Class,
            TokenKind::Def,
            TokenKind::Return,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::Elif,
            TokenKind::For,
            TokenKind::While,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Pass,
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Not,
            TokenKind::In,
            TokenKind::Is,
            TokenKind::None,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_more_keywords() {
    let kinds = lex_kinds(
        "try except finally raise with import from as lambda yield global nonlocal assert del",
    );
    assert_eq!(
        kinds,
        vec![
            TokenKind::Try,
            TokenKind::Except,
            TokenKind::Finally,
            TokenKind::Raise,
            TokenKind::With,
            TokenKind::Import,
            TokenKind::From,
            TokenKind::As,
            TokenKind::Lambda,
            TokenKind::Yield,
            TokenKind::Global,
            TokenKind::Nonlocal,
            TokenKind::Assert,
            TokenKind::Del,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_multiple_dedents() {
    let kinds = lex_kinds("if x:\n    if y:\n        if z:\n            pass\nprint('done')");
    assert_eq!(
        kinds,
        vec![
            TokenKind::If,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::If,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::If,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::Pass,
            TokenKind::Newline,
            TokenKind::Dedent,
            TokenKind::Dedent,
            TokenKind::Dedent,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::String,
            TokenKind::RightParen,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_paren_continuation() {
    let kinds = lex_kinds("x = (1 +\n    2)");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::LeftParen,
            TokenKind::Number,
            TokenKind::Plus,
            TokenKind::Number,
            TokenKind::RightParen,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_bracket_brace_depth() {
    let kinds = lex_kinds("x = [1,\n    2, 3]\ny = {1:\n    2}");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::LeftBracket,
            TokenKind::Number,
            TokenKind::Comma,
            TokenKind::Number,
            TokenKind::Comma,
            TokenKind::Number,
            TokenKind::RightBracket,
            TokenKind::Newline,
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::LeftBrace,
            TokenKind::Number,
            TokenKind::Colon,
            TokenKind::Number,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_empty_input() {
    let kinds = lex_kinds("");
    assert_eq!(kinds, vec![TokenKind::Eof])
}

#[test]
fn lexes_only_whitespace() {
    let kinds = lex_kinds("   \n\n   \n");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Newline,
            TokenKind::Newline,
            TokenKind::Newline,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_illegal_character() {
    let kinds = lex_kinds("x @");
    assert_eq!(kinds, vec![TokenKind::Name, TokenKind::At, TokenKind::Eof,])
}

#[test]
fn lexes_function_def() {
    let kinds = lex_kinds("def foo(x, y):\n    return x + y");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Def,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::Name,
            TokenKind::Comma,
            TokenKind::Name,
            TokenKind::RightParen,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::Return,
            TokenKind::Name,
            TokenKind::Plus,
            TokenKind::Name,
            TokenKind::Dedent,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_class_def() {
    let kinds = lex_kinds("class Foo:\n    def bar(self):\n        pass");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Class,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::Def,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::Name,
            TokenKind::RightParen,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::Pass,
            TokenKind::Dedent,
            TokenKind::Dedent,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_fstring_with_raw() {
    let kinds = lex_kinds("f'value: {x}' fr'raw: {y}'");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::FStringStart,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn non_raw_fstring_text_keeps_escaped_quote() {
    let (tokens, diags) = lex_tokens_and_diags(r#"f"prefix \" value {x}""#);

    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn raw_fstring_text_keeps_backslash_quote_sequence() {
    let (tokens, diags) = lex_tokens_and_diags(r#"rf"prefix \" value {x}""#);

    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn raw_fstring_regex_from_pandas_script_lexes() {
    let (tokens, diags) = lex_tokens_and_diags(r#"rf"\b{word}\b""#);

    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn lexes_fstring_triple_quoted() {
    let kinds = lex_kinds("f'''multi\nline {x}'''");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_multiline_triple_fstring_from_pandas_script() {
    let src = concat!(
        "f\"\"\"{filename}:{line_number}:{err_msg} \"{title}\" to \"{\n",
        "    correct_title_capitalization(title)\n",
        "}\" \"\"\"",
    );
    let (tokens, diags) = lex_tokens_and_diags(src);

    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Newline,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::Name,
            TokenKind::RightParen,
            TokenKind::Newline,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn lexes_tail_of_pandas_validation_script() {
    let src = concat!(
        "number_of_errors: int = 0\n\n",
        "for filename in source_paths:\n",
        "    for title, line_number in find_titles(filename):\n",
        "        if title != correct_title_capitalization(title):\n",
        "            print(\n",
        "                f\"\"\"{filename}:{line_number}:{err_msg} \"{title}\" to \"{\n",
        "                    correct_title_capitalization(title)\n",
        "                }\" \"\"\"\n",
        "            )\n",
        "            number_of_errors += 1\n\n",
        "    return number_of_errors\n\n\n",
        "if __name__ == \"__main__\":\n",
        "    parser = argparse.ArgumentParser(description=\"Validate heading capitalization\")\n\n",
        "    parser.add_argument(\n",
        "        \"paths\", nargs=\"*\", help=\"Source paths of file/directory to check.\"\n",
        "    )\n\n",
        "    args = parser.parse_args()\n\n",
        "    sys.exit(main(args.paths))\n",
    );
    let (tokens, diags) = lex_tokens_and_diags(src);

    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    assert!(
        diags.is_empty(),
        "diags={diags:?} kinds={:?}",
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>()
    );
}

#[test]
fn lexes_tstring_with_raw() {
    let kinds = lex_kinds("t'value: {x}' tr'raw: {y}' rt'more: {z}'");
    assert_eq!(
        kinds,
        vec![
            TokenKind::TStringStart,
            TokenKind::TStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::TStringEnd,
            TokenKind::TStringStart,
            TokenKind::TStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::TStringEnd,
            TokenKind::TStringStart,
            TokenKind::TStringMiddle,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::TStringEnd,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_interpolated_double_left_brace_split() {
    let kinds = lex_kinds("f'{{x}'");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::FStringMiddle,
            TokenKind::Illegal,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_interpolated_double_right_brace_as_text() {
    let kinds = lex_kinds("f'{x}}}'");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringMiddle,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn interpolated_string_tokens_have_expected_spans() {
    let src = "f\"this is {x}\"";
    let tokens = lex_tokens(src);

    assert_eq!(tokens[0].kind, TokenKind::FStringStart);
    assert_eq!(tokens[0].span.slice(src), "f\"");

    assert_eq!(tokens[1].kind, TokenKind::FStringMiddle);
    assert_eq!(tokens[1].span.slice(src), "this is ");

    assert_eq!(tokens[2].kind, TokenKind::LeftBrace);
    assert_eq!(tokens[2].span.slice(src), "{");

    assert_eq!(tokens[3].kind, TokenKind::Name);
    assert_eq!(tokens[3].span.slice(src), "x");

    assert_eq!(tokens[4].kind, TokenKind::RightBrace);
    assert_eq!(tokens[4].span.slice(src), "}");

    assert_eq!(tokens[5].kind, TokenKind::FStringEnd);
    assert_eq!(tokens[5].span.slice(src), "\"");
}

#[test]
fn interpolated_double_left_brace_split_has_expected_spans() {
    let src = "f'{{x}'";
    let tokens = lex_tokens(src);

    assert_eq!(tokens[0].kind, TokenKind::FStringStart);
    assert_eq!(tokens[0].span.slice(src), "f'");

    assert_eq!(tokens[1].kind, TokenKind::FStringMiddle);
    assert_eq!(tokens[1].span.slice(src), "{{x");

    assert_eq!(tokens[2].kind, TokenKind::Illegal);
    assert_eq!(tokens[2].span.slice(src), "}");

    assert_eq!(tokens[3].kind, TokenKind::FStringEnd);
    assert_eq!(tokens[3].span.slice(src), "'");
}

#[test]
fn interpolated_double_right_brace_text_has_expected_spans() {
    let src = "f'{x}}}'";
    let tokens = lex_tokens(src);

    assert_eq!(tokens[0].kind, TokenKind::FStringStart);
    assert_eq!(tokens[0].span.slice(src), "f'");

    assert_eq!(tokens[1].kind, TokenKind::LeftBrace);
    assert_eq!(tokens[1].span.slice(src), "{");

    assert_eq!(tokens[2].kind, TokenKind::Name);
    assert_eq!(tokens[2].span.slice(src), "x");

    assert_eq!(tokens[3].kind, TokenKind::RightBrace);
    assert_eq!(tokens[3].span.slice(src), "}");

    assert_eq!(tokens[4].kind, TokenKind::FStringMiddle);
    assert_eq!(tokens[4].span.slice(src), "}}");

    assert_eq!(tokens[5].kind, TokenKind::FStringEnd);
    assert_eq!(tokens[5].span.slice(src), "'");
}

#[test]
fn lexes_interpolated_expr_with_nested_delimiters() {
    let kinds = lex_kinds("f\"{foo({1: [x]})}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::LeftParen,
            TokenKind::LeftBrace,
            TokenKind::Number,
            TokenKind::Colon,
            TokenKind::LeftBracket,
            TokenKind::Name,
            TokenKind::RightBracket,
            TokenKind::RightBrace,
            TokenKind::RightParen,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn raw_single_line_string_rejects_newline() {
    let (tokens, diags) = lex_tokens_and_diags("r'hello\nworld'");
    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::UnterminatedString,
            TokenKind::Newline,
            TokenKind::Name,
            TokenKind::UnterminatedString,
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        diags,
        vec![
            LexDiagKind::UnterminatedString,
            LexDiagKind::UnterminatedString
        ]
    );
}

#[test]
fn raw_single_line_fstring_rejects_newline_in_text() {
    let (tokens, diags) = lex_tokens_and_diags("fr'hello\n{x}'");
    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::UnterminatedString,
            TokenKind::Newline,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::UnterminatedString,
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        diags,
        vec![
            LexDiagKind::UnterminatedString,
            LexDiagKind::UnterminatedString
        ]
    );
}

#[test]
fn non_triple_fstring_rejects_newline_in_expr() {
    let (tokens, diags) = lex_tokens_and_diags("f'{x\n}'");
    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::UnterminatedString,
            TokenKind::Newline,
            TokenKind::RightBrace,
            TokenKind::UnterminatedString,
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        diags,
        vec![
            LexDiagKind::UnterminatedFstring,
            LexDiagKind::UnterminatedString,
        ]
    );
}

#[test]
fn triple_fstring_allows_newline_in_expr() {
    let kinds = lex_kinds("f'''{x\n}'''");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Newline,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn triple_fstring_newline_does_not_trigger_outer_indent_state() {
    let kinds = lex_kinds("if x:\n    y = f'''{foo\n}'''\n    z = 1\n");
    assert_eq!(
        kinds,
        vec![
            TokenKind::If,
            TokenKind::Name,
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Indent,
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::Newline,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Newline,
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Newline,
            TokenKind::Dedent,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn nested_interpolated_string_does_not_clobber_outer_expr_mode() {
    let kinds = lex_kinds("f\"{f'{x}' + y}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Plus,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn nested_interpolated_string_preserves_outer_delimiter_stack() {
    let kinds = lex_kinds("f\"{(f'{x}', y)}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::LeftParen,
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Comma,
            TokenKind::Name,
            TokenKind::RightParen,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn malformed_numbers_remain_single_tokens() {
    let (tokens, diags) = lex_tokens_and_diags("0b102 0x1g 1e+ 0123");
    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        tokens[..4]
            .iter()
            .map(|tok| tok.span.slice("0b102 0x1g 1e+ 0123"))
            .collect::<Vec<_>>(),
        vec!["0b102", "0x1g", "1e+", "0123"]
    );
    assert_eq!(
        diags,
        vec![
            LexDiagKind::InvalidNumber,
            LexDiagKind::InvalidNumber,
            LexDiagKind::InvalidNumber,
            LexDiagKind::InvalidNumber,
        ]
    );
}

#[test]
fn all_zero_prefixed_decimals_are_valid() {
    let (tokens, diags) = lex_tokens_and_diags("0 00 000 0_0 00_0");

    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
    assert!(diags.is_empty());
}

#[test]
fn nonzero_digits_after_leading_zero_are_invalid() {
    let (tokens, diags) = lex_tokens_and_diags("01 0123 00_1");

    assert_eq!(
        tokens.iter().map(|tok| tok.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        tokens[..3]
            .iter()
            .map(|tok| tok.span.slice("01 0123 00_1"))
            .collect::<Vec<_>>(),
        vec!["01", "0123", "00_1"]
    );
    assert_eq!(
        diags,
        vec![
            LexDiagKind::InvalidNumber,
            LexDiagKind::InvalidNumber,
            LexDiagKind::InvalidNumber,
        ]
    );
}

#[test]
fn lexes_dotted_names() {
    let kinds = lex_kinds("sys.path os.environ");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Dot,
            TokenKind::Name,
            TokenKind::Name,
            TokenKind::Dot,
            TokenKind::Name,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_semicolon_separator() {
    let kinds = lex_kinds("x = 1; y = 2");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Semicolon,
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_comment_at_eof() {
    let kinds = lex_kinds("x = 1 # comment");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_underscore_identifiers() {
    let kinds = lex_kinds("_foo __bar _ _123");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Name,
            TokenKind::Name,
            TokenKind::Name,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_unicode_identifier() {
    let kinds = lex_kinds("π = 3");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_non_latin_identifier() {
    let kinds = lex_kinds("变量 = 1");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_mixed_unicode_identifier() {
    let kinds = lex_kinds("café = 1");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    )
}

#[test]
fn lexes_unicode_identifier_inside_fstring_expr() {
    let kinds = lex_kinds("f\"{变量}\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::FStringStart,
            TokenKind::LeftBrace,
            TokenKind::Name,
            TokenKind::RightBrace,
            TokenKind::FStringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn unicode_identifier_span_is_byte_correct() {
    let src = "π_value = 3";
    let tokens = lex_tokens(src);

    assert_eq!(tokens[0].kind, TokenKind::Name);
    assert_eq!(tokens[0].span.slice(src), "π_value");
}

#[test]
fn emoji_is_not_identifier_start() {
    let (tokens, diags) = lex_tokens_and_diags("😀 = 1");
    assert_eq!(tokens[0].kind, TokenKind::Illegal);
    assert_eq!(tokens[0].span.slice("😀 = 1"), "😀");
    assert_eq!(diags, vec![LexDiagKind::UnexpectedCharacter]);
}

#[test]
fn lexes_backslash_line_continuation() {
    let kinds = lex_kinds("x = 1 + \\\n    2");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Plus,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_backslash_line_continuation_with_crlf() {
    let kinds = lex_kinds("x = 1 + \\\r\n    2");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Plus,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_backslash_crlf_line_continuation() {
    let kinds = lex_kinds("x = 1 + \\\r\n    2");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Name,
            TokenKind::Equal,
            TokenKind::Number,
            TokenKind::Plus,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
}
