#[cfg(test)]
mod tests;

use super::tokens::{Span, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexDiagKind {
    InvalidIndentation,
    UnterminatedString,
    InvalidNumber,
    UnexpectedCharacter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexDiag {
    pub kind: LexDiagKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StringPrefix {
    len: usize,
    raw: bool,
    bytes: bool,
    fstring: bool,
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

    pub fn next_token(&mut self) -> Token {
        if let Some(tok) = self.pending.pop() {
            return tok;
        }

        match self.mode_stack.last().copied().unwrap_or(Mode::Normal) {
            Mode::Normal => self.next_normal_token(),
        }
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
        while !matches!(self.current(), b'\n' | b'\r' | 0) {
            self.bump();
        }
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

    fn scan_operator(&mut self) -> Option<Token> {
        let start = self.pos as u32;

        match TokenKind::from_operator_bytes(&self.bytes[self.pos..]) {
            None => None,
            Some((kind, width)) => {
                for _ in 0..width {
                    self.bump();
                }

                let out_kind = match kind {
                    TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace => {
                        self.paren_depth += 1;
                        kind
                    }
                    TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                        if self.paren_depth > 0 {
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
    fn is_ident_start(&self, b: u8) -> bool {
        b.is_ascii_alphabetic() || b == b'_'
    }

    #[inline]
    fn is_ident_continue(&self, b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }

    fn scan_identifier_or_keyword(&mut self) -> Token {
        let start = self.pos as u32;

        self.bump();
        while self.is_ident_continue(self.current()) {
            self.bump();
        }

        let end = self.pos as u32;
        let text = &self.bytes[start as usize..end as usize];
        let kind = TokenKind::from_keyword(text).unwrap_or(TokenKind::Name);

        Token::new(kind, start, end)
    }

    fn skip_horizontal_ws(&mut self) {
        while matches!(self.current(), b' ' | b'\t' | b'\x0c') {
            self.bump();
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
            if self.current() == 0 {
                return TokenKind::UnterminatedString;
            }

            if self.current() == quote && self.peek(1) == quote && self.peek(2) == quote {
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
                    fstring: false,
                    unicode: false,
                    quote: b1,
                }),
                b'b' => Some(StringPrefix {
                    len: 1,
                    raw: false,
                    bytes: true,
                    fstring: false,
                    unicode: false,
                    quote: b1,
                }),
                b'f' => Some(StringPrefix {
                    len: 1,
                    raw: false,
                    bytes: false,
                    fstring: true,
                    unicode: false,
                    quote: b1,
                }),
                b'u' => Some(StringPrefix {
                    len: 1,
                    raw: false,
                    bytes: false,
                    fstring: false,
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
                    fstring: true,
                    unicode: false,
                    quote: b2,
                }),
                (b'r', b'b') | (b'b', b'r') => Some(StringPrefix {
                    len: 2,
                    raw: true,
                    bytes: true,
                    fstring: false,
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
                    self.bump();
                    self.bump();

                    let (saw_digit, bad) = self.scan_digits_with_underscores_hex();
                    if !saw_digit || bad {
                        self.push_diag(LexDiagKind::InvalidNumber, start, self.pos as u32);
                    }

                    return Token::new(TokenKind::Number, start, self.pos as u32);
                }
                b'o' | b'O' => {
                    self.bump();
                    self.bump();

                    let (saw_digit, bad) = self.scan_digits_with_underscores_octal();
                    if !saw_digit || bad {
                        self.push_diag(LexDiagKind::InvalidNumber, start, self.pos as u32);
                    }

                    return Token::new(TokenKind::Number, start, self.pos as u32);
                }
                b'b' | b'B' => {
                    self.bump();
                    self.bump();

                    let (saw_digit, bad) = self.scan_digits_with_underscores_bin();
                    if !saw_digit || bad {
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
            let exp_pos = self.pos;
            self.bump();

            if matches!(self.current(), b'+' | b'-') {
                self.bump();
            }

            let (saw_exp_digit, bad_exp) = self.scan_digits_with_underscores_dec();
            if !saw_exp_digit {
                invalid = true;
                self.pos = exp_pos;
            } else {
                invalid |= bad_exp;
            }
        }

        if matches!(self.current(), b'j' | b'J') {
            self.bump();
        }

        if invalid {
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
            if self.current() == 0 {
                return TokenKind::UnterminatedString;
            }

            if self.current() == quote
                && self.peek(1) == quote
                && self.peek(2) == quote
                && !self.has_odd_trailing_backslashes_before(self.pos)
            {
                self.bump();
                self.bump();
                self.bump();
                return TokenKind::String;
            }

            self.bump();
        }
    }

    fn scan_f_string(&mut self, quote: u8, raw: bool, triple: bool) -> TokenKind {
        if triple {
            self.bump();
            self.bump();
            self.bump();

            loop {
                if self.current() == 0 {
                    return TokenKind::UnterminatedString;
                }

                if self.current() == quote && self.peek(1) == quote && self.peek(2) == quote {
                    self.bump();
                    self.bump();
                    self.bump();
                    return TokenKind::FString;
                }

                if !raw && self.current() == b'\\' {
                    self.bump();
                    if self.current() == 0 {
                        return TokenKind::UnterminatedString;
                    }
                }

                self.bump();
            }
        }

        self.bump(); // opening quote

        while self.current() != 0 {
            if !raw && self.current() == b'\\' {
                self.bump();
                if self.current() == 0 {
                    return TokenKind::UnterminatedString;
                }
                self.bump();
                continue;
            }

            if self.current() == quote {
                self.bump();
                return TokenKind::FString;
            }

            match self.current() {
                b'\n' | b'\r' if !triple => return TokenKind::UnterminatedString,
                _ => self.bump(),
            }
        }

        TokenKind::UnterminatedString
    }

    fn scan_prefixed_string(&mut self, prefix: StringPrefix) -> Token {
        let start = self.pos as u32;

        for _ in 0..prefix.len {
            self.bump();
        }

        let is_triple = self.current() == prefix.quote
            && self.peek(1) == prefix.quote
            && self.peek(2) == prefix.quote;

        let mut kind = if prefix.fstring {
            self.scan_f_string(prefix.quote, prefix.raw, is_triple)
        } else if prefix.raw {
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
            kind = TokenKind::BString;
        }

        if kind == TokenKind::UnterminatedString {
            self.push_diag(LexDiagKind::UnterminatedString, start, self.pos as u32);
        }

        Token::new(kind, start, self.pos as u32)
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

            if self.current() == b'#' {
                self.skip_comment();
                continue;
            }

            if let Some(tok) = self.scan_operator() {
                return tok;
            }

            let b = self.current();

            if self.is_ident_start(b) {
                if let Some(prefix) = self.scan_string_prefix() {
                    return self.scan_prefixed_string(prefix);
                }

                return self.scan_identifier_or_keyword();
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
            self.bump();
            self.push_diag(LexDiagKind::UnexpectedCharacter, start, self.pos as u32);
            return Token::new(TokenKind::Illegal, start, self.pos as u32);
        }
    }
}
