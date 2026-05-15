use crate::interner::LocalSymbol;
/// Representing a span inside the source text
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Span {
    start: u32,
    end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn as_range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }

    pub fn slice<'a>(&self, src: &'a str) -> &'a str {
        let range = self.as_range();

        debug_assert!(range.start <= range.end);
        debug_assert!(range.start <= src.len());
        debug_assert!(src.is_char_boundary(range.start));
        debug_assert!(src.is_char_boundary(range.end));
        &src[range]
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Token {
    span: Span,
    kind: TokenKind,
    symbol: Option<LocalSymbol>,
}

impl Token {
    pub fn new(start: u32, end: u32, kind: TokenKind) -> Self {
        Self {
            span: Span { start, end },
            kind,
            symbol: None,
        }
    }

    pub fn with_span(span: Span, kind: TokenKind) -> Self {
        Self {
            span,
            kind,
            symbol: None,
        }
    }

    pub fn with_span_and_symbol(span: Span, kind: TokenKind, sym: LocalSymbol) -> Self {
        Self {
            span,
            kind,
            symbol: Some(sym),
        }
    }

    pub fn with_symbol(start: u32, end: u32, kind: TokenKind, sym: LocalSymbol) -> Self {
        Self {
            span: Span { start, end },
            kind,
            symbol: Some(sym),
        }
    }

    pub fn dedent(start: u32, end: u32) -> Self {
        Self {
            span: Span { start, end },
            symbol: None,
            kind: TokenKind::Dedent,
        }
    }

    pub fn indent(start: u32, end: u32) -> Self {
        Self {
            span: Span { start, end },
            symbol: None,
            kind: TokenKind::Indent,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TokenKind {
    Eof,
    Illegal,

    Name,
    Number,
    String,

    // f-string
    FStringStart,
    FStringMiddle,
    FStringEnd,

    // t-string
    TStringStart,
    TStringMiddle,
    TStringEnd,

    BString,

    Newline,
    Indent,
    Dedent,

    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Colon,
    Semicolon,
    Comma,
    Dot,

    Plus,
    Minus,
    Star,
    Slash,
    DoubleSlash,
    Percent,

    Pipe,
    Ampersand,
    Circumflex,
    Tilde,
    At,

    Less,
    Greater,
    Equal,

    LeftBrace,
    RightBrace,

    EqualEqual,
    NotEqual,
    LessEqual,
    GreaterEqual,
    LeftShift,
    RightShift,
    DoubleStar,

    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    DoubleSlashEqual,
    PercentEqual,
    AmpersandEqual,
    PipeEqual,
    CircumflexEqual,
    LeftShiftEqual,
    RightShiftEqual,
    DoubleStarEqual,
    AtEqual,

    Arrow,
    ColonEqual,
    Ellipsis,
    Exclamation,

    Async,
    Await,
    False,
    None,
    True,
    And,
    As,
    Assert,
    Break,
    Class,
    Continue,
    Def,
    Del,
    Elif,
    Else,
    Except,
    Finally,
    For,
    From,
    Global,
    If,
    Import,
    In,
    Is,
    Lambda,
    Nonlocal,
    Not,
    Or,
    Pass,
    Raise,
    Return,
    Try,
    While,
    With,
    Yield,

    UnterminatedString,
}

impl TokenKind {
    /// match u8 slice to python keyword
    pub fn from_keyword(s: &[u8]) -> Option<Self> {
        match s.len() {
            2..=8 => {}
            _ => return None,
        }

        match s {
            b"False" => Some(Self::False),
            b"None" => Some(Self::None),
            b"True" => Some(Self::True),
            b"async" => Some(Self::Async),
            b"await" => Some(Self::Await),
            b"and" => Some(Self::And),
            b"or" => Some(Self::Or),
            b"not" => Some(Self::Not),
            b"in" => Some(Self::In),
            b"is" => Some(Self::Is),
            b"if" => Some(Self::If),
            b"elif" => Some(Self::Elif),
            b"else" => Some(Self::Else),
            b"for" => Some(Self::For),
            b"while" => Some(Self::While),
            b"break" => Some(Self::Break),
            b"continue" => Some(Self::Continue),
            b"pass" => Some(Self::Pass),
            b"def" => Some(Self::Def),
            b"class" => Some(Self::Class),
            b"return" => Some(Self::Return),
            b"import" => Some(Self::Import),
            b"from" => Some(Self::From),
            b"as" => Some(Self::As),
            b"try" => Some(Self::Try),
            b"except" => Some(Self::Except),
            b"finally" => Some(Self::Finally),
            b"raise" => Some(Self::Raise),
            b"with" => Some(Self::With),
            b"lambda" => Some(Self::Lambda),
            b"assert" => Some(Self::Assert),
            b"global" => Some(Self::Global),
            b"nonlocal" => Some(Self::Nonlocal),
            b"del" => Some(Self::Del),
            b"yield" => Some(Self::Yield),
            _ => None,
        }
    }

    /// match byte to token
    pub fn from_single_byte(b: u8) -> Option<Self> {
        match b {
            b'(' => Some(Self::LeftParen),
            b')' => Some(Self::RightParen),
            b'[' => Some(Self::LeftBracket),
            b']' => Some(Self::RightBracket),
            b':' => Some(Self::Colon),
            b',' => Some(Self::Comma),
            b';' => Some(Self::Semicolon),
            b'+' => Some(Self::Plus),
            b'=' => Some(Self::Equal),
            b'-' => Some(Self::Minus),
            b'*' => Some(Self::Star),
            b'/' => Some(Self::Slash),
            b'|' => Some(Self::Pipe),
            b'&' => Some(Self::Ampersand),
            b'<' => Some(Self::Less),
            b'>' => Some(Self::Greater),
            b'.' => Some(Self::Dot),
            b'%' => Some(Self::Percent),
            b'{' => Some(Self::LeftBrace),
            b'}' => Some(Self::RightBrace),
            b'~' => Some(Self::Tilde),
            b'^' => Some(Self::Circumflex),
            b'@' => Some(Self::At),
            b'!' => Some(Self::Exclamation),
            _ => None,
        }
    }

    pub fn from_operator_bytes(rest: &[u8]) -> Option<(Self, usize)> {
        match rest {
            [b'<', b'<', b'=', ..] => Some((Self::LeftShiftEqual, 3)),
            [b'>', b'>', b'=', ..] => Some((Self::RightShiftEqual, 3)),
            [b'*', b'*', b'=', ..] => Some((Self::DoubleStarEqual, 3)),
            [b'/', b'/', b'=', ..] => Some((Self::DoubleSlashEqual, 3)),
            [b'.', b'.', b'.', ..] => Some((Self::Ellipsis, 3)),

            [b'=', b'=', ..] => Some((Self::EqualEqual, 2)),
            [b'!', b'=', ..] => Some((Self::NotEqual, 2)),
            [b'<', b'=', ..] => Some((Self::LessEqual, 2)),
            [b'>', b'=', ..] => Some((Self::GreaterEqual, 2)),
            [b'>', b'>', ..] => Some((Self::RightShift, 2)),
            [b'<', b'<', ..] => Some((Self::LeftShift, 2)),
            [b'*', b'*', ..] => Some((Self::DoubleStar, 2)),
            [b'+', b'=', ..] => Some((Self::PlusEqual, 2)),
            [b'-', b'=', ..] => Some((Self::MinusEqual, 2)),
            [b'*', b'=', ..] => Some((Self::StarEqual, 2)),
            [b'/', b'=', ..] => Some((Self::SlashEqual, 2)),
            [b'%', b'=', ..] => Some((Self::PercentEqual, 2)),
            [b'&', b'=', ..] => Some((Self::AmpersandEqual, 2)),
            [b'|', b'=', ..] => Some((Self::PipeEqual, 2)),
            [b'/', b'/', ..] => Some((Self::DoubleSlash, 2)),
            [b'@', b'=', ..] => Some((Self::AtEqual, 2)),
            [b'-', b'>', ..] => Some((Self::Arrow, 2)),
            [b':', b'=', ..] => Some((Self::ColonEqual, 2)),
            [b'^', b'=', ..] => Some((Self::CircumflexEqual, 2)),

            [c, ..] => Self::from_single_byte(*c).map(|kind| (kind, 1)),
            [] => None,
        }
    }

    /// Check if current token is a Python keyword
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            Self::Async
                | Self::Await
                | Self::False
                | Self::None
                | Self::True
                | Self::And
                | Self::As
                | Self::Assert
                | Self::Break
                | Self::Class
                | Self::Continue
                | Self::Def
                | Self::Del
                | Self::Elif
                | Self::Else
                | Self::Except
                | Self::Finally
                | Self::For
                | Self::From
                | Self::Global
                | Self::If
                | Self::Import
                | Self::In
                | Self::Is
                | Self::Lambda
                | Self::Nonlocal
                | Self::Not
                | Self::Or
                | Self::Pass
                | Self::Raise
                | Self::Return
                | Self::Try
                | Self::While
                | Self::With
                | Self::Yield
        )
    }
}

#[cfg(test)]
mod test;
