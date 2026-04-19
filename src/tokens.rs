/// Span represents the start pos and end pos of a lexer token
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self {
            start: start,
            end: end,
        }
    }
    pub fn len(self) -> u32 {
        self.end - self.start
    }
    pub fn slice<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start as usize..self.end as usize]
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, start: u32, end: u32) -> Self {
        Self {
            kind: kind,
            span: Span {
                start: start,
                end: end,
            },
        }
    }

    pub fn new_with_span(kind: TokenKind, span: Span) -> Self {
        Self {
            kind: kind,
            span: span,
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
    FString,
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
    /// Returns the keyword token kind for `s`, if `s` is a Python keyword.
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s {
            "False" => Some(Self::False),
            "None" => Some(Self::None),
            "True" => Some(Self::True),
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "not" => Some(Self::Not),
            "in" => Some(Self::In),
            "is" => Some(Self::Is),
            "if" => Some(Self::If),
            "elif" => Some(Self::Elif),
            "else" => Some(Self::Else),
            "for" => Some(Self::For),
            "while" => Some(Self::While),
            "break" => Some(Self::Break),
            "continue" => Some(Self::Continue),
            "pass" => Some(Self::Pass),
            "def" => Some(Self::Def),
            "class" => Some(Self::Class),
            "return" => Some(Self::Return),
            "import" => Some(Self::Import),
            "from" => Some(Self::From),
            "as" => Some(Self::As),
            "try" => Some(Self::Try),
            "except" => Some(Self::Except),
            "finally" => Some(Self::Finally),
            "raise" => Some(Self::Raise),
            "with" => Some(Self::With),
            "lambda" => Some(Self::Lambda),
            "assert" => Some(Self::Assert),
            "global" => Some(Self::Global),
            "nonlocal" => Some(Self::Nonlocal),
            "del" => Some(Self::Del),
            "yield" => Some(Self::Yield),
            _ => None,
        }
    }

    pub fn from_single_char(c: char) -> Option<Self> {
        match c {
            '(' => Some(Self::LeftParen),
            ')' => Some(Self::RightParen),
            '[' => Some(Self::LeftBracket),
            ']' => Some(Self::RightBracket),
            ':' => Some(Self::Colon),
            ',' => Some(Self::Comma),
            ';' => Some(Self::Semicolon),
            '+' => Some(Self::Plus),
            '=' => Some(Self::Equal),
            '-' => Some(Self::Minus),
            '*' => Some(Self::Star),
            '/' => Some(Self::Slash),
            '|' => Some(Self::Pipe),
            '&' => Some(Self::Ampersand),
            '<' => Some(Self::Less),
            '>' => Some(Self::Greater),
            '.' => Some(Self::Dot),
            '%' => Some(Self::Percent),
            '{' => Some(Self::LeftBrace),
            '}' => Some(Self::RightBrace),
            '~' => Some(Self::Tilde),
            '^' => Some(Self::Circumflex),
            '@' => Some(Self::At),
            '!' => Some(Self::Exclamation),
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

            [c, ..] => Self::from_single_char(*c as char).map(|kind| (kind, 1)),
            [] => None,
        }
    }
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            Self::False
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
