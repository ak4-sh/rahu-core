#[cfg(test)]
mod tests;

use super::tokens::{Span, Token, TokenKind};
use memchr::{memchr, memchr2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexDiagKind {
    InvalidIndentation,
    UnterminatedString,
    InvalidNumber,
    UnexpectedCharacter,
    InvalidFstring,
    UnterminatedFstring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Brace {
    Paren,   // (
    Bracket, // [
    Curly,   // {
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexDiag {
    pub kind: LexDiagKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriviaKind {
    Comment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub span: Span,
    pub precedes_token: u32,
}

impl Trivia {
    pub fn text<'src>(&self, src: &'src str) -> &'src str {
        self.span.slice(src)
    }

    fn comment_payload<'src>(&self, src: &'src str) -> &'src str {
        let text = self.text(src);
        text.strip_prefix('#').unwrap_or(text).trim_start()
    }

    pub fn is_type_ignore(&self, src: &str) -> bool {
        let payload = self.comment_payload(src);
        if payload == "type: ignore" {
            return true;
        }

        let Some(rest) = payload.strip_prefix("type: ignore") else {
            return false;
        };

        let rest = rest.trim();
        rest.starts_with('[') && rest.ends_with(']')
    }

    pub fn is_type_comment(&self, src: &str) -> bool {
        let payload = self.comment_payload(src);
        payload
            .strip_prefix("type:")
            .is_some_and(|rest| !rest.trim().is_empty())
            && !self.is_type_ignore(src)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Normal,
    InterpolatedText {
        kind: InterpolatedKind,
        quote: u8,
        triple: bool,
        raw: bool,
    },
    InterpolatedFormatSpec {
        kind: InterpolatedKind,
        quote: u8,
        triple: bool,
        raw: bool,
        return_to: ExprReturn,
    },
    InterpolatedExpr {
        kind: InterpolatedKind,
        quote: u8,
        triple: bool,
        raw: bool,
        brace_stack: Vec<Brace>,
        return_to: ExprReturn,
        phase: FieldPhase,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterpolatedKind {
    F,
    T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldPhase {
    Expr,
    Conversion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprReturn {
    Text,
    FormatSpec,
}

enum ExprStep {
    Token(Token),
    Unterminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StringPrefix {
    len: usize,
    raw: bool,
    bytes: bool,
    interpolated: Option<InterpolatedKind>,
    unicode: bool,
    quote: u8,
}

pub struct Lexer<'src> {
    bytes: &'src [u8],
    len: usize,
    pos: usize,

    indent_stack: Vec<u32>,
    pending: Vec<Token>,
    mode_stack: Vec<Mode>,
    diagnostics: Vec<LexDiag>,
    trivia: Vec<Trivia>,
    next_token_index: u32,

    paren_depth: u32,
    at_bol: bool,
}

impl<'src> Lexer<'src> {
    pub fn new(input: &'src str) -> Self {
        let bytes = input.as_bytes();
        Self {
            bytes,
            len: bytes.len(),
            pos: 0,
            indent_stack: vec![0],
            pending: Vec::new(),
            diagnostics: Vec::new(),
            trivia: Vec::new(),
            next_token_index: 0,
            mode_stack: vec![Mode::Normal],
            paren_depth: 0,
            at_bol: true,
        }
    }

    #[inline]
    fn current(&self) -> u8 {
        if self.pos < self.len {
            self.bytes[self.pos]
        } else {
            0
        }
    }

    #[inline]
    fn peek(&self, i: usize) -> u8 {
        let idx = self.pos.saturating_add(i);
        if idx < self.len { self.bytes[idx] } else { 0 }
    }

    #[inline]
    fn bump(&mut self) {
        if self.pos < self.len {
            self.pos += 1
        }
    }

    fn push_diag(&mut self, kind: LexDiagKind, start: u32, end: u32) {
        self.diagnostics.push(LexDiag {
            kind,
            span: Span { start, end },
        });
    }

    pub fn take_diagnostics(&mut self) -> Vec<LexDiag> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn take_trivia(&mut self) -> Vec<Trivia> {
        std::mem::take(&mut self.trivia)
    }

    fn emit_token(&mut self, tok: Token) -> Token {
        self.next_token_index += 1;
        tok
    }

    pub fn next_token(&mut self) -> Token {
        if let Some(tok) = self.pending.pop() {
            return self.emit_token(tok);
        }

        enum NextMode {
            Normal,
            InterpolatedText {
                kind: InterpolatedKind,
                quote: u8,
                triple: bool,
                raw: bool,
            },
            InterpolatedFormatSpec {
                kind: InterpolatedKind,
                quote: u8,
                triple: bool,
                raw: bool,
                return_to: ExprReturn,
            },
            InterpolatedExpr {
                kind: InterpolatedKind,
                quote: u8,
                triple: bool,
                raw: bool,
                return_to: ExprReturn,
                phase: FieldPhase,
            },
        }

        let mode = match self.mode_stack.last() {
            Some(Mode::Normal) | None => NextMode::Normal,
            Some(Mode::InterpolatedText {
                kind,
                quote,
                triple,
                raw,
            }) => NextMode::InterpolatedText {
                kind: *kind,
                quote: *quote,
                triple: *triple,
                raw: *raw,
            },
            Some(Mode::InterpolatedFormatSpec {
                kind,
                quote,
                triple,
                raw,
                return_to,
            }) => NextMode::InterpolatedFormatSpec {
                kind: *kind,
                quote: *quote,
                triple: *triple,
                raw: *raw,
                return_to: *return_to,
            },
            Some(Mode::InterpolatedExpr {
                kind,
                quote,
                triple,
                raw,
                return_to,
                phase,
                ..
            }) => NextMode::InterpolatedExpr {
                kind: *kind,
                quote: *quote,
                triple: *triple,
                raw: *raw,
                return_to: *return_to,
                phase: *phase,
            },
        };

        let tok = match mode {
            NextMode::Normal => self.next_normal_token(),
            NextMode::InterpolatedText {
                kind,
                quote,
                triple,
                raw,
            } => self.next_interpolated_text_token(kind, quote, triple, raw),
            NextMode::InterpolatedFormatSpec {
                kind,
                quote,
                triple,
                raw,
                return_to,
            } => self.next_interpolated_format_spec_token(kind, quote, triple, raw, return_to),
            NextMode::InterpolatedExpr {
                kind,
                quote,
                triple,
                raw,
                return_to,
                phase,
            } => self.lex_interpolated_expr(kind, quote, triple, raw, return_to, phase),
        };

        self.emit_token(tok)
    }

    fn scan_indent(&mut self) {
        let start = self.pos as u32;
        let mut column: u32 = 0;
        let pending_start = self.pending.len();

        loop {
            match self.current() {
                b' ' => {
                    column += 1;
                    self.bump();
                }
                b'\t' => {
                    column = ((column / 8) + 1) * 8;
                    self.bump();
                }
                b'\x0c' => {
                    self.bump();
                }
                _ => break,
            }
        }

        let first = self.current();

        if matches!(first, b'\n' | b'\r' | b'#' | 0) {
            return;
        }

        let current_indent = *self.indent_stack.last().unwrap();

        if column > current_indent {
            self.indent_stack.push(column);
            self.pending
                .push(Token::new(TokenKind::Indent, start, self.pos as u32));
        } else if column < current_indent {
            let mut dedents = 0usize;

            while self.indent_stack.len() > 1 && *self.indent_stack.last().unwrap() > column {
                self.indent_stack.pop();
                dedents += 1;
            }

            for _ in 0..dedents {
                self.pending
                    .push(Token::new(TokenKind::Dedent, start, start));
            }

            if *self.indent_stack.last().unwrap() != column {
                self.push_diag(LexDiagKind::InvalidIndentation, start, self.pos as u32);
            }
        }

        self.pending[pending_start..].reverse();
        self.at_bol = false;
    }

    fn skip_comment(&mut self) {
        let start = self.pos as u32;
        let rest = &self.bytes[self.pos..];
        if let Some(offset) = memchr2(b'\n', b'\r', rest) {
            self.pos += offset;
        } else {
            self.pos = self.len;
        }

        self.trivia.push(Trivia {
            kind: TriviaKind::Comment,
            span: Span {
                start,
                end: self.pos as u32,
            },
            precedes_token: self.next_token_index,
        });
    }

    fn scan_newline(&mut self) -> Option<Token> {
        let start = self.pos as u32;

        match self.current() {
            b'\r' => {
                self.bump();
                if self.current() == b'\n' {
                    self.bump();
                }
            }
            b'\n' => {
                self.bump();
            }
            _ => unreachable!(),
        }

        // if we're inside an expression, then just go lex next token
        if self.paren_depth > 0 {
            self.at_bol = false;
            return None;
        }

        self.at_bol = true;
        Some(Token::new(TokenKind::Newline, start, self.pos as u32))
    }

    fn skip_line_continuation(&mut self) -> bool {
        if self.current() != b'\\' {
            return false;
        }

        match self.peek(1) {
            b'\n' => {
                self.bump();
                self.bump();
            }
            b'\r' => {
                self.bump();
                self.bump();
                if self.current() == b'\n' {
                    self.bump();
                }
            }
            _ => return false,
        }

        self.at_bol = false;
        true
    }

    fn scan_operator(&mut self, track_paren_depth: bool) -> Option<Token> {
        let start = self.pos as u32;

        match TokenKind::from_operator_bytes(&self.bytes[self.pos..]) {
            None => None,
            Some((kind, width)) => {
                for _ in 0..width {
                    self.bump();
                }

                let out_kind = match kind {
                    TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace => {
                        if track_paren_depth {
                            self.paren_depth += 1;
                        }
                        kind
                    }
                    TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                        if track_paren_depth && self.paren_depth > 0 {
                            self.paren_depth -= 1;
                        }
                        kind
                    }
                    TokenKind::Exclamation => {
                        self.push_diag(LexDiagKind::UnexpectedCharacter, start, self.pos as u32);
                        TokenKind::Illegal
                    }
                    _ => kind,
                };

                Some(Token::new(out_kind, start, self.pos as u32))
            }
        }
    }

    #[inline]
    fn current_non_ascii_char(&self) -> Option<char> {
        debug_assert!(self.pos < self.len);
        debug_assert!(!self.current().is_ascii());

        unsafe { std::str::from_utf8_unchecked(&self.bytes[self.pos..]).chars().next() }
    }

    #[inline]
    fn bump_char(&mut self) -> Option<char> {
        if self.pos >= self.len {
            return None;
        }

        let ch = unsafe { std::str::from_utf8_unchecked(&self.bytes[self.pos..]).chars().next() }?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    #[inline]
    fn is_ident_start(&self, ch: char) -> bool {
        ch == '_' || unicode_ident::is_xid_start(ch)
    }

    #[inline]
    fn is_ident_continue(&self, ch: char) -> bool {
        ch == '_' || unicode_ident::is_xid_continue(ch)
    }

    #[inline]
    fn current_starts_identifier(&self) -> bool {
        let b = self.current();
        if b.is_ascii() {
            self.is_ident_start(b as char)
        } else {
            self.current_non_ascii_char()
                .is_some_and(|ch| self.is_ident_start(ch))
        }
    }

    fn scan_identifier_or_keyword(&mut self) -> Token {
        debug_assert!(self.current_starts_identifier());

        let start = self.pos as u32;
        let mut ascii_only = self.current().is_ascii();

        if ascii_only {
            self.bump();
            while self.pos < self.len {
                let b = self.current();
                if b.is_ascii_alphanumeric() || b == b'_' {
                    self.bump();
                    continue;
                }
                if b.is_ascii() {
                    let end = self.pos as u32;
                    let text = &self.bytes[start as usize..end as usize];
                    let kind = TokenKind::from_keyword(text).unwrap_or(TokenKind::Name);
                    return Token::new(kind, start, end);
                }
                ascii_only = false;
                break;
            }

            if ascii_only {
                let end = self.pos as u32;
                let text = &self.bytes[start as usize..end as usize];
                let kind = TokenKind::from_keyword(text).unwrap_or(TokenKind::Name);
                return Token::new(kind, start, end);
            }
        }

        if !ascii_only {
            if start == self.pos as u32 {
                self.bump_char();
            }
            while self.pos < self.len {
                let b = self.current();

                if b.is_ascii() {
                    if b.is_ascii_alphanumeric() || b == b'_' {
                        self.bump();
                        continue;
                    }
                    break;
                }

                let Some(ch) = self.current_non_ascii_char() else {
                    break;
                };
                if !self.is_ident_continue(ch) {
                    break;
                }
                self.pos += ch.len_utf8();
            }
        }

        let end = self.pos as u32;

        Token::new(TokenKind::Name, start, end)
    }

    fn skip_horizontal_ws(&mut self) {
        while self.pos < self.len {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\x0c' => self.pos += 1,
                _ => break,
            }
        }
    }

    #[inline]
    fn is_dec_digit(b: u8) -> bool {
        b.is_ascii_digit()
    }

    #[inline]
    fn is_bin_digit(b: u8) -> bool {
        matches!(b, b'0' | b'1')
    }

    #[inline]
    fn is_oct_digit(b: u8) -> bool {
        matches!(b, b'0'..=b'7')
    }

    #[inline]
    fn is_hex_digit(b: u8) -> bool {
        b.is_ascii_hexdigit()
    }

    fn scan_digits_with_underscores_dec(&mut self) -> (bool, bool) {
        let mut saw_digit = false;
        let mut prev_underscore = false;
        let mut invalid = false;

        loop {
            let b = self.current();

            if Self::is_dec_digit(b) {
                saw_digit = true;
                prev_underscore = false;
                self.bump();
            } else if b == b'_' {
                if !saw_digit || prev_underscore {
                    invalid = true;
                }
                prev_underscore = true;
                self.bump();
            } else {
                break;
            }
        }

        if prev_underscore {
            invalid = true;
        }

        (saw_digit, invalid)
    }

    fn scan_digits_with_underscores_bin(&mut self) -> (bool, bool) {
        let mut saw_digit = false;
        let mut prev_underscore = false;
        let mut invalid = false;

        loop {
            let b = self.current();

            if Self::is_bin_digit(b) {
                saw_digit = true;
                prev_underscore = false;
                self.bump();
            } else if b == b'_' {
                if !saw_digit || prev_underscore {
                    invalid = true;
                }
                prev_underscore = true;
                self.bump();
            } else {
                break;
            }
        }

        if prev_underscore {
            invalid = true;
        }

        (saw_digit, invalid)
    }

    fn scan_digits_with_underscores_octal(&mut self) -> (bool, bool) {
        let mut saw_digit = false;
        let mut prev_underscore = false;
        let mut invalid = false;

        loop {
            let b = self.current();

            if Self::is_oct_digit(b) {
                saw_digit = true;
                prev_underscore = false;
                self.bump();
            } else if b == b'_' {
                if !saw_digit || prev_underscore {
                    invalid = true;
                }
                prev_underscore = true;
                self.bump();
            } else {
                break;
            }
        }

        if prev_underscore {
            invalid = true;
        }

        (saw_digit, invalid)
    }

    fn scan_digits_with_underscores_hex(&mut self) -> (bool, bool) {
        let mut saw_digit = false;
        let mut prev_underscore = false;
        let mut invalid = false;

        loop {
            let b = self.current();

            if Self::is_hex_digit(b) {
                saw_digit = true;
                prev_underscore = false;
                self.bump();
            } else if b == b'_' {
                if !saw_digit || prev_underscore {
                    invalid = true;
                }
                prev_underscore = true;
                self.bump();
            } else {
                break;
            }
        }

        if prev_underscore {
            invalid = true;
        }

        (saw_digit, invalid)
    }

    #[inline]
    fn is_number_tail(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }

    fn consume_number_tail(&mut self) {
        while Self::is_number_tail(self.current()) {
            self.bump();
        }
    }

    fn scan_radix_number(
        &mut self,
        start: u32,
        scan_digits: fn(&mut Self) -> (bool, bool),
        is_valid_digit: fn(u8) -> bool,
    ) -> Token {
        self.bump();
        self.bump();

        let (saw_digit, mut invalid) = scan_digits(self);
        if !saw_digit {
            invalid = true;
        }

        while Self::is_number_tail(self.current()) {
            let b = self.current();
            if b != b'_' && !is_valid_digit(b) {
                invalid = true;
            }
            self.bump();
        }

        if invalid {
            self.push_diag(LexDiagKind::InvalidNumber, start, self.pos as u32);
        }

        Token::new(TokenKind::Number, start, self.pos as u32)
    }

    fn scan_string(&mut self) -> Token {
        let start = self.pos as u32;
        let quote = self.current();

        let kind = if self.peek(1) == quote && self.peek(2) == quote {
            self.scan_triple_string(quote)
        } else {
            self.scan_simple_string(quote)
        };

        Token::new(kind, start, self.pos as u32)
    }

    fn scan_simple_string(&mut self, quote: u8) -> TokenKind {
        self.bump();

        while self.current() != 0 {
            match self.current() {
                b'\n' | b'\r' => return TokenKind::UnterminatedString,
                b'\\' => {
                    self.bump();
                    if self.current() == 0 {
                        return TokenKind::UnterminatedString;
                    }
                    self.bump();
                }
                b if b == quote => {
                    self.bump();
                    return TokenKind::String;
                }
                _ => self.bump(),
            }
        }

        TokenKind::UnterminatedString
    }

    fn scan_triple_string(&mut self, quote: u8) -> TokenKind {
        self.bump();
        self.bump();
        self.bump();

        loop {
            let rest = &self.bytes[self.pos..];
            let Some(offset) = memchr(quote, rest) else {
                self.pos = self.len;
                return TokenKind::UnterminatedString;
            };

            self.pos += offset;
            if self.peek(1) == quote && self.peek(2) == quote {
                self.bump();
                self.bump();
                self.bump();
                return TokenKind::String;
            }

            self.bump();
        }
    }

    fn scan_string_prefix(&self) -> Option<StringPrefix> {
        let b0 = self.current();
        let b1 = self.peek(1);
        let b2 = self.peek(2);

        let is_quote = |b: u8| b == b'\'' || b == b'"';
        let lower = |b: u8| b.to_ascii_lowercase();

        // one-letter prefixes: r, b, f, u
        if is_quote(b1) {
            return match lower(b0) {
                b'r' => Some(StringPrefix {
                    len: 1,
                    raw: true,
                    bytes: false,
                    interpolated: None,
                    unicode: false,
                    quote: b1,
                }),
                b'b' => Some(StringPrefix {
                    len: 1,
                    raw: false,
                    bytes: true,
                    interpolated: None,
                    unicode: false,
                    quote: b1,
                }),
                b'f' => Some(StringPrefix {
                    len: 1,
                    raw: false,
                    bytes: false,
                    interpolated: Some(InterpolatedKind::F),
                    unicode: false,
                    quote: b1,
                }),
                b't' => Some(StringPrefix {
                    len: 1,
                    raw: false,
                    bytes: false,
                    interpolated: Some(InterpolatedKind::T),
                    unicode: false,
                    quote: b1,
                }),
                b'u' => Some(StringPrefix {
                    len: 1,
                    raw: false,
                    bytes: false,
                    interpolated: None,
                    unicode: true,
                    quote: b1,
                }),
                _ => None,
            };
        }

        // two-letter prefixes: rf, fr, rb, br
        if is_quote(b2) {
            let a = lower(b0);
            let b = lower(b1);

            return match (a, b) {
                (b'r', b'f') | (b'f', b'r') => Some(StringPrefix {
                    len: 2,
                    raw: true,
                    bytes: false,
                    interpolated: Some(InterpolatedKind::F),
                    unicode: false,
                    quote: b2,
                }),
                (b'r', b't') | (b't', b'r') => Some(StringPrefix {
                    len: 2,
                    raw: true,
                    bytes: false,
                    interpolated: Some(InterpolatedKind::T),
                    unicode: false,
                    quote: b2,
                }),
                (b'r', b'b') | (b'b', b'r') => Some(StringPrefix {
                    len: 2,
                    raw: true,
                    bytes: true,
                    interpolated: None,
                    unicode: false,
                    quote: b2,
                }),
                _ => None,
            };
        }

        None
    }

    fn scan_number(&mut self) -> Token {
        let start = self.pos as u32;
        let mut invalid = false;

        if self.current() == b'0' {
            match self.peek(1) {
                b'x' | b'X' => {
                    return self.scan_radix_number(
                        start,
                        Self::scan_digits_with_underscores_hex,
                        Self::is_hex_digit,
                    );
                }
                b'o' | b'O' => {
                    return self.scan_radix_number(
                        start,
                        Self::scan_digits_with_underscores_octal,
                        Self::is_oct_digit,
                    );
                }
                b'b' | b'B' => {
                    return self.scan_radix_number(
                        start,
                        Self::scan_digits_with_underscores_bin,
                        Self::is_bin_digit,
                    );
                }
                b'0'..=b'9' | b'_' => {
                    let mut zero_prefixed_invalid = self.scan_zero_prefixed_decimal_integer();
                    if matches!(self.current(), b'.' | b'e' | b'E' | b'j' | b'J') {
                        if self.current() == b'.' {
                            self.bump();
                            let (_, bad_frac) = self.scan_digits_with_underscores_dec();
                            zero_prefixed_invalid |= bad_frac;
                        }

                        if matches!(self.current(), b'e' | b'E') {
                            self.bump();
                            if matches!(self.current(), b'+' | b'-') {
                                self.bump();
                            }

                            let (saw_exp_digit, bad_exp) = self.scan_digits_with_underscores_dec();
                            if !saw_exp_digit {
                                zero_prefixed_invalid = true;
                            } else {
                                zero_prefixed_invalid |= bad_exp;
                            }
                        }

                        if matches!(self.current(), b'j' | b'J') {
                            self.bump();
                        }
                    }

                    if zero_prefixed_invalid {
                        self.consume_number_tail();
                        self.push_diag(LexDiagKind::InvalidNumber, start, self.pos as u32);
                    }
                    return Token::new(TokenKind::Number, start, self.pos as u32);
                }
                _ => {}
            }
        }

        let (_, bad_int) = self.scan_digits_with_underscores_dec();
        invalid |= bad_int;

        if self.current() == b'.' && self.peek(1) != b'.' {
            self.bump();
            let (_, bad_frac) = self.scan_digits_with_underscores_dec();
            invalid |= bad_frac;
        }

        if matches!(self.current(), b'e' | b'E') {
            self.bump();

            if matches!(self.current(), b'+' | b'-') {
                self.bump();
            }

            let (saw_exp_digit, bad_exp) = self.scan_digits_with_underscores_dec();
            if !saw_exp_digit {
                invalid = true;
            } else {
                invalid |= bad_exp;
            }
        }

        if matches!(self.current(), b'j' | b'J') {
            self.bump();
        }

        if invalid {
            self.consume_number_tail();
        }

        if invalid {
            self.push_diag(LexDiagKind::InvalidNumber, start, self.pos as u32);
        }

        Token::new(TokenKind::Number, start, self.pos as u32)
    }

    fn scan_dot_number(&mut self) -> Token {
        let start = self.pos as u32;
        self.bump();

        let (_, bad_frac) = self.scan_digits_with_underscores_dec();
        let mut invalid = bad_frac;

        if matches!(self.current(), b'e' | b'E') {
            self.bump();
            if matches!(self.current(), b'+' | b'-') {
                self.bump();
            }

            let (saw_exp_digit, bad_exp) = self.scan_digits_with_underscores_dec();
            invalid |= !saw_exp_digit || bad_exp;
        }

        if matches!(self.current(), b'j' | b'J') {
            self.bump();
        }

        if invalid {
            self.consume_number_tail();
            self.push_diag(LexDiagKind::InvalidNumber, start, self.pos as u32);
        }

        Token::new(TokenKind::Number, start, self.pos as u32)
    }

    #[inline]
    fn has_odd_trailing_backslashes_before(&self, idx: usize) -> bool {
        if idx == 0 {
            return false;
        }

        let mut i = idx;
        let mut count = 0usize;

        while i > 0 && self.bytes[i - 1] == b'\\' {
            count += 1;
            i -= 1;
        }

        count % 2 == 1
    }

    fn scan_raw_string(&mut self, quote: u8) -> TokenKind {
        self.bump(); // opening quote

        while self.current() != 0 {
            if matches!(self.current(), b'\n' | b'\r') {
                return TokenKind::UnterminatedString;
            }

            if self.current() == quote && !self.has_odd_trailing_backslashes_before(self.pos) {
                self.bump();
                return TokenKind::String;
            }

            self.bump();
        }

        TokenKind::UnterminatedString
    }

    fn scan_raw_triple_string(&mut self, quote: u8) -> TokenKind {
        self.bump();
        self.bump();
        self.bump();

        loop {
            let rest = &self.bytes[self.pos..];
            let Some(offset) = memchr(quote, rest) else {
                self.pos = self.len;
                return TokenKind::UnterminatedString;
            };

            self.pos += offset;
            if self.peek(1) == quote && self.peek(2) == quote {
                self.bump();
                self.bump();
                self.bump();
                return TokenKind::String;
            }

            self.bump();
        }
    }

    fn scan_zero_prefixed_decimal_integer(&mut self) -> bool {
        debug_assert!(self.current() == b'0');

        self.bump();

        let mut invalid = false;
        let mut prev_underscore = false;
        let mut saw_nonzero_digit = false;

        loop {
            match self.current() {
                b'0' => {
                    prev_underscore = false;
                    self.bump();
                }
                b'1'..=b'9' => {
                    saw_nonzero_digit = true;
                    prev_underscore = false;
                    self.bump();
                }
                b'_' => {
                    if prev_underscore {
                        invalid = true;
                    }
                    prev_underscore = true;
                    self.bump();
                }
                _ => break,
            }
        }

        if prev_underscore {
            invalid = true;
        }

        invalid || saw_nonzero_digit
    }

    fn scan_prefixed_string(&mut self, prefix: StringPrefix) -> Token {
        debug_assert!(prefix.interpolated.is_none());

        let start = self.pos as u32;

        for _ in 0..prefix.len {
            self.bump();
        }

        let is_triple = self.current() == prefix.quote
            && self.peek(1) == prefix.quote
            && self.peek(2) == prefix.quote;

        let mut kind = if prefix.raw {
            if is_triple {
                self.scan_raw_triple_string(prefix.quote)
            } else {
                self.scan_raw_string(prefix.quote)
            }
        } else if is_triple {
            self.scan_triple_string(prefix.quote)
        } else {
            self.scan_simple_string(prefix.quote)
        };

        if kind == TokenKind::String && prefix.bytes {
            kind = TokenKind::BString
        }

        if kind == TokenKind::UnterminatedString {
            self.push_diag(LexDiagKind::UnterminatedString, start, self.pos as u32);
        }
        Token::new(kind, start, self.pos as u32)
    }

    fn interpolated_boundary_kind(&self, kind: InterpolatedKind, is_start: bool) -> TokenKind {
        match (kind, is_start) {
            (InterpolatedKind::F, true) => TokenKind::FStringStart,
            (InterpolatedKind::F, false) => TokenKind::FStringEnd,
            (InterpolatedKind::T, true) => TokenKind::TStringStart,
            (InterpolatedKind::T, false) => TokenKind::TStringEnd,
        }
    }

    fn interpolated_middle_kind(&self, kind: InterpolatedKind) -> TokenKind {
        match kind {
            InterpolatedKind::F => TokenKind::FStringMiddle,
            InterpolatedKind::T => TokenKind::TStringMiddle,
        }
    }

    fn enter_interpolated_string(&mut self, prefix: StringPrefix, kind: InterpolatedKind) -> Token {
        let start = self.pos as u32;

        for _ in 0..prefix.len {
            self.bump();
        }

        let triple = self.current() == prefix.quote
            && self.peek(1) == prefix.quote
            && self.peek(2) == prefix.quote;

        // consume opening delimiter
        if triple {
            self.bump();
            self.bump();
            self.bump();
        } else {
            self.bump();
        }

        self.mode_stack.push(Mode::InterpolatedText {
            kind,
            quote: prefix.quote,
            triple,
            raw: prefix.raw,
        });

        Token::new(
            self.interpolated_boundary_kind(kind, true),
            start,
            self.pos as u32,
        )
    }

    fn next_normal_token(&mut self) -> Token {
        loop {
            if self.at_bol && self.paren_depth == 0 {
                self.scan_indent();

                if let Some(tok) = self.pending.pop() {
                    return tok;
                }
            }

            if self.current() == 0 {
                let pos = self.pos as u32;

                if self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    return Token::new(TokenKind::Dedent, pos, pos);
                }

                return Token::new(TokenKind::Eof, pos, pos);
            }

            if matches!(self.current(), b'\n' | b'\r') {
                if let Some(tok) = self.scan_newline() {
                    return tok;
                }
                continue;
            }

            if self.skip_line_continuation() {
                continue;
            }

            self.skip_horizontal_ws();

            if self.current() == 0 {
                continue;
            }

            if matches!(self.current(), b'\n' | b'\r') {
                if let Some(tok) = self.scan_newline() {
                    return tok;
                }
                continue;
            }

            if self.skip_line_continuation() {
                continue;
            }

            if self.current() == b'#' {
                self.skip_comment();
                continue;
            }

            if self.current() == b'.' && self.peek(1).is_ascii_digit() {
                return self.scan_dot_number();
            }

            if let Some(tok) = self.scan_operator(true) {
                return tok;
            }

            let b = self.current();

            if b.is_ascii() && self.is_ident_start(b as char) {
                if let Some(prefix) = self.scan_string_prefix() {
                    if let Some(kind) = prefix.interpolated {
                        return self.enter_interpolated_string(prefix, kind);
                    }
                    return self.scan_prefixed_string(prefix);
                }

                return self.scan_identifier_or_keyword();
            }

            if !b.is_ascii() {
                if self.current_starts_identifier() {
                    return self.scan_identifier_or_keyword();
                }
            }

            if b.is_ascii_digit() {
                return self.scan_number();
            }

            if b == b'\'' || b == b'"' {
                let tok = self.scan_string();
                if tok.kind == TokenKind::UnterminatedString {
                    self.push_diag(
                        LexDiagKind::UnterminatedString,
                        tok.span.start,
                        tok.span.end,
                    );
                }
                return tok;
            }

            let start = self.pos as u32;
            if self.current().is_ascii() {
                self.bump();
            } else {
                let _ = self.bump_char();
            }
            self.push_diag(LexDiagKind::UnexpectedCharacter, start, self.pos as u32);
            return Token::new(TokenKind::Illegal, start, self.pos as u32);
        }
    }

    /// Lexing just the text part of an interpolated string.
    fn next_interpolated_text_token(
        &mut self,
        kind: InterpolatedKind,
        quote: u8,
        triple: bool,
        raw: bool,
    ) -> Token {
        let start = self.pos as u32;

        loop {
            let b = self.current();

            if b == 0 {
                self.push_diag(LexDiagKind::UnterminatedString, start, self.pos as u32);
                self.mode_stack.pop();
                return Token::new(TokenKind::UnterminatedString, start, self.pos as u32);
            }

            if b == b'{' {
                if self.peek(1) == b'{' {
                    self.bump();
                    self.bump();
                    continue;
                }

                if self.pos > start as usize {
                    return Token::new(self.interpolated_middle_kind(kind), start, self.pos as u32);
                }

                self.bump();
                *self.mode_stack.last_mut().unwrap() = Mode::InterpolatedExpr {
                    kind,
                    quote,
                    triple,
                    raw,
                    brace_stack: Vec::new(),
                    return_to: ExprReturn::Text,
                    phase: FieldPhase::Expr,
                };
                return Token::new(TokenKind::LeftBrace, start, self.pos as u32);
            }

            if b == b'}' {
                if self.peek(1) == b'}' {
                    self.bump();
                    self.bump();
                    continue;
                }

                if self.pos > start as usize {
                    return Token::new(self.interpolated_middle_kind(kind), start, self.pos as u32);
                }

                self.bump();
                self.push_diag(LexDiagKind::UnexpectedCharacter, start, self.pos as u32);
                return Token::new(TokenKind::Illegal, start, self.pos as u32);
            }

            if triple {
                if b == quote
                    && self.peek(1) == quote
                    && self.peek(2) == quote
                    && !self.has_odd_trailing_backslashes_before(self.pos)
                {
                    if self.pos > start as usize {
                        return Token::new(
                            self.interpolated_middle_kind(kind),
                            start,
                            self.pos as u32,
                        );
                    }

                    self.bump();
                    self.bump();
                    self.bump();
                    self.mode_stack.pop();
                    return Token::new(
                        self.interpolated_boundary_kind(kind, false),
                        start,
                        self.pos as u32,
                    );
                }
            } else {
                if b == quote && !self.has_odd_trailing_backslashes_before(self.pos) {
                    if self.pos > start as usize {
                        return Token::new(
                            self.interpolated_middle_kind(kind),
                            start,
                            self.pos as u32,
                        );
                    }

                    self.bump();
                    self.mode_stack.pop();
                    return Token::new(
                        self.interpolated_boundary_kind(kind, false),
                        start,
                        self.pos as u32,
                    );
                }

                if matches!(b, b'\n' | b'\r') {
                    self.push_diag(LexDiagKind::UnterminatedString, start, self.pos as u32);
                    self.mode_stack.pop();
                    return Token::new(TokenKind::UnterminatedString, start, self.pos as u32);
                }
            }

            if !raw && b == b'\\' {
                self.bump();
                if self.current() == 0 {
                    self.push_diag(LexDiagKind::UnterminatedString, start, self.pos as u32);
                    self.mode_stack.pop();
                    return Token::new(TokenKind::UnterminatedString, start, self.pos as u32);
                }
                self.bump();
                continue;
            }

            self.bump();
        }
    }

    fn next_interpolated_format_spec_token(
        &mut self,
        kind: InterpolatedKind,
        quote: u8,
        triple: bool,
        raw: bool,
        return_to: ExprReturn,
    ) -> Token {
        let start = self.pos as u32;

        loop {
            let b = self.current();

            if b == 0 {
                self.push_diag(LexDiagKind::UnterminatedString, start, self.pos as u32);
                self.mode_stack.pop();
                return Token::new(TokenKind::UnterminatedString, start, self.pos as u32);
            }

            if !triple && matches!(b, b'\n' | b'\r') {
                self.push_diag(LexDiagKind::UnterminatedString, start, self.pos as u32);
                self.mode_stack.pop();
                return Token::new(TokenKind::UnterminatedString, start, self.pos as u32);
            }

            if b == b'{' {
                if self.peek(1) == b'{' {
                    self.bump();
                    self.bump();
                    continue;
                }

                if self.pos > start as usize {
                    return Token::new(self.interpolated_middle_kind(kind), start, self.pos as u32);
                }

                self.bump();
                self.mode_stack.push(Mode::InterpolatedExpr {
                    kind,
                    quote,
                    triple,
                    raw,
                    brace_stack: Vec::new(),
                    return_to: ExprReturn::FormatSpec,
                    phase: FieldPhase::Expr,
                });
                return Token::new(TokenKind::LeftBrace, start, self.pos as u32);
            }

            if b == b'}' {
                if self.peek(1) == b'}' {
                    self.bump();
                    self.bump();
                    continue;
                }

                if self.pos > start as usize {
                    return Token::new(self.interpolated_middle_kind(kind), start, self.pos as u32);
                }

                self.bump();
                match return_to {
                    ExprReturn::Text => {
                        *self.mode_stack.last_mut().unwrap() = Mode::InterpolatedText {
                            kind,
                            quote,
                            triple,
                            raw,
                        };
                    }
                    ExprReturn::FormatSpec => {
                        self.mode_stack.pop();
                    }
                }
                return Token::new(TokenKind::RightBrace, start, self.pos as u32);
            }

            self.bump();
        }
    }

    fn update_interpolated_brace_stack(brace_stack: &mut Vec<Brace>, kind: TokenKind) {
        match kind {
            TokenKind::LeftParen => brace_stack.push(Brace::Paren),
            TokenKind::LeftBracket => brace_stack.push(Brace::Bracket),
            TokenKind::LeftBrace => brace_stack.push(Brace::Curly),
            TokenKind::RightParen => {
                if matches!(brace_stack.last(), Some(Brace::Paren)) {
                    brace_stack.pop();
                }
            }
            TokenKind::RightBracket => {
                if matches!(brace_stack.last(), Some(Brace::Bracket)) {
                    brace_stack.pop();
                }
            }
            TokenKind::RightBrace => {
                if matches!(brace_stack.last(), Some(Brace::Curly)) {
                    brace_stack.pop();
                }
            }
            _ => {}
        }
    }

    fn interpolated_expr_brace_stack_empty(&self, expr_index: usize) -> bool {
        matches!(
            self.mode_stack.get(expr_index),
            Some(Mode::InterpolatedExpr { brace_stack, .. }) if brace_stack.is_empty()
        )
    }

    fn set_interpolated_expr_phase(&mut self, expr_index: usize, phase: FieldPhase) {
        if let Some(Mode::InterpolatedExpr {
            phase: current_phase,
            ..
        }) = self.mode_stack.get_mut(expr_index)
        {
            *current_phase = phase;
        }
    }

    fn interpolated_expr_phase(&self, expr_index: usize) -> Option<FieldPhase> {
        match self.mode_stack.get(expr_index) {
            Some(Mode::InterpolatedExpr { phase, .. }) => Some(*phase),
            _ => None,
        }
    }

    fn resume_interpolated_expr_target(
        &mut self,
        expr_index: usize,
        kind: InterpolatedKind,
        quote: u8,
        triple: bool,
        raw: bool,
    ) {
        let return_to = match self.mode_stack.get(expr_index) {
            Some(Mode::InterpolatedExpr { return_to, .. }) => *return_to,
            _ => return,
        };

        match return_to {
            ExprReturn::Text => {
                if let Some(slot) = self.mode_stack.get_mut(expr_index) {
                    *slot = Mode::InterpolatedText {
                        kind,
                        quote,
                        triple,
                        raw,
                    };
                }
            }
            ExprReturn::FormatSpec => {
                self.mode_stack.pop();
            }
        }
    }

    fn lex_mode_expr_token(&mut self, expr_index: usize, triple: bool) -> ExprStep {
        loop {
            if self.current() == 0 {
                return ExprStep::Unterminated;
            }

            if matches!(self.current(), b' ' | b'\t' | b'\x0c') {
                self.skip_horizontal_ws();
                continue;
            }

            if self.skip_line_continuation() {
                continue;
            }

            if matches!(self.current(), b'\n' | b'\r') {
                if !triple {
                    return ExprStep::Unterminated;
                }

                let start = self.pos as u32;
                match self.current() {
                    b'\r' => {
                        self.bump();
                        if self.current() == b'\n' {
                            self.bump();
                        }
                    }
                    b'\n' => self.bump(),
                    _ => unreachable!(),
                }

                if self.interpolated_expr_brace_stack_empty(expr_index) {
                    return ExprStep::Token(Token::new(TokenKind::Newline, start, self.pos as u32));
                }

                continue;
            }

            if self.current() == b'#' {
                self.skip_comment();
                continue;
            }

            if self.current() == b'.' && self.peek(1).is_ascii_digit() {
                return ExprStep::Token(self.scan_dot_number());
            }

            if let Some(tok) = self.scan_operator(false) {
                return ExprStep::Token(tok);
            }

            let b = self.current();

            if b.is_ascii() && self.is_ident_start(b as char) {
                if let Some(prefix) = self.scan_string_prefix() {
                    if let Some(kind) = prefix.interpolated {
                        return ExprStep::Token(self.enter_interpolated_string(prefix, kind));
                    }
                    return ExprStep::Token(self.scan_prefixed_string(prefix));
                }

                return ExprStep::Token(self.scan_identifier_or_keyword());
            }

            if !b.is_ascii() {
                if self.current_starts_identifier() {
                    return ExprStep::Token(self.scan_identifier_or_keyword());
                }
            }

            if b.is_ascii_digit() {
                return ExprStep::Token(self.scan_number());
            }

            if b == b'\'' || b == b'"' {
                let tok = self.scan_string();
                if tok.kind == TokenKind::UnterminatedString {
                    self.push_diag(
                        LexDiagKind::UnterminatedString,
                        tok.span.start,
                        tok.span.end,
                    );
                }
                return ExprStep::Token(tok);
            }

            let start = self.pos as u32;
            if self.current().is_ascii() {
                self.bump();
            } else {
                let _ = self.bump_char();
            }
            self.push_diag(LexDiagKind::UnexpectedCharacter, start, self.pos as u32);
            return ExprStep::Token(Token::new(TokenKind::Illegal, start, self.pos as u32));
        }
    }

    /// Lexer when we're in an interpolated-string expression.
    fn lex_interpolated_expr(
        &mut self,
        kind: InterpolatedKind,
        quote: u8,
        triple: bool,
        raw: bool,
        _return_to: ExprReturn,
        phase: FieldPhase,
    ) -> Token {
        let start = self.pos as u32;
        let expr_index = self.mode_stack.len().saturating_sub(1);

        if !matches!(self.mode_stack.last(), Some(Mode::InterpolatedExpr { .. })) {
            self.push_diag(LexDiagKind::InvalidFstring, start, start);
            return Token::new(TokenKind::Illegal, start, start);
        }

        if self.current() == 0 {
            self.push_diag(LexDiagKind::UnterminatedFstring, start, self.pos as u32);
            self.mode_stack.pop();
            return Token::new(TokenKind::UnterminatedString, start, self.pos as u32);
        }

        if self.interpolated_expr_brace_stack_empty(expr_index) {
            self.skip_horizontal_ws();
        }

        if self.current() == b'}' && self.interpolated_expr_brace_stack_empty(expr_index) {
            self.bump();
            self.resume_interpolated_expr_target(expr_index, kind, quote, triple, raw);
            return Token::new(TokenKind::RightBrace, start, self.pos as u32);
        }

        if self.interpolated_expr_brace_stack_empty(expr_index) {
            if self.current() == b'!' && self.peek(1) != b'=' {
                self.bump();
                if matches!(phase, FieldPhase::Expr) {
                    self.set_interpolated_expr_phase(expr_index, FieldPhase::Conversion);
                }
                return Token::new(TokenKind::Exclamation, start, self.pos as u32);
            }

            if self.current() == b':' && self.peek(1) != b'=' {
                self.bump();
                let return_to = match self.mode_stack.get(expr_index) {
                    Some(Mode::InterpolatedExpr { return_to, .. }) => *return_to,
                    _ => {
                        self.push_diag(LexDiagKind::InvalidFstring, start, start);
                        return Token::new(TokenKind::Illegal, start, start);
                    }
                };
                if let Some(slot) = self.mode_stack.get_mut(expr_index) {
                    *slot = Mode::InterpolatedFormatSpec {
                        kind,
                        quote,
                        triple,
                        raw,
                        return_to,
                    };
                }
                return Token::new(TokenKind::Colon, start, self.pos as u32);
            }

            if matches!(phase, FieldPhase::Expr) && self.current() == b'=' && self.peek(1) != b'=' {
                self.bump();
                return Token::new(TokenKind::Equal, start, self.pos as u32);
            }
        }

        let tok = match self.lex_mode_expr_token(expr_index, triple) {
            ExprStep::Token(tok) => tok,
            ExprStep::Unterminated => {
                self.push_diag(LexDiagKind::UnterminatedFstring, start, self.pos as u32);
                self.mode_stack.pop();
                return Token::new(TokenKind::UnterminatedString, start, self.pos as u32);
            }
        };

        if self.mode_stack.len() != expr_index + 1 {
            return tok;
        }

        if matches!(
            self.interpolated_expr_phase(expr_index),
            Some(FieldPhase::Conversion)
        ) && self.interpolated_expr_brace_stack_empty(expr_index)
            && !matches!(tok.kind, TokenKind::Colon | TokenKind::RightBrace)
        {
            self.set_interpolated_expr_phase(expr_index, FieldPhase::Expr);
        }

        let Some(slot) = self.mode_stack.get_mut(expr_index) else {
            self.push_diag(LexDiagKind::InvalidFstring, start, start);
            return Token::new(TokenKind::Illegal, start, start);
        };

        match slot {
            Mode::InterpolatedExpr { brace_stack, .. } => {
                Self::update_interpolated_brace_stack(brace_stack, tok.kind);
            }
            _ => {
                self.push_diag(LexDiagKind::InvalidFstring, start, start);
                return Token::new(TokenKind::Illegal, start, start);
            }
        }

        tok
    }
}
