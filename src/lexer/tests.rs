use super::Lexer;
use crate::tokens::TokenKind;

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
            TokenKind::FString,
            TokenKind::RightParen,
            TokenKind::Eof,
        ]
    )
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
            TokenKind::Illegal,
            TokenKind::Pass,
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
    let kinds = lex_kinds("3.14 0.5 10.0");
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
fn lexes_raw_multiline_strings() {
    let kinds = lex_kinds("r'''raw\ntriple''' r\"\"\"raw triple\"\"\"");
    assert_eq!(
        kinds,
        vec![TokenKind::String, TokenKind::String, TokenKind::Eof,]
    )
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
            TokenKind::Exclamation,
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
        vec![TokenKind::FString, TokenKind::FString, TokenKind::Eof,]
    )
}

#[test]
fn lexes_fstring_triple_quoted() {
    let kinds = lex_kinds("f'''multi\nline {x}'''");
    assert_eq!(kinds, vec![TokenKind::FString, TokenKind::Eof,])
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
