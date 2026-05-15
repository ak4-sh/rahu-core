#[allow(unused_imports)]
use crate::{
    interner::{LocalInterner, LocalSymbol},
    lexer,
    tokens::{Span, Token, TokenKind},
};
use std::collections::VecDeque;

use memchr::{memchr, memchr2, memchr3};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum LexDiagnosticKind {
    InvalidIndentation,
    UnterminatedString,
    InvalidNumber,
    UnexpectedCharacter,
    InvalidFstring,
    UnterminatedFstring,
}

#[derive(Clone, Eq, PartialEq, Copy)]
pub struct LexDiagnostic {
    pub kind: LexDiagnosticKind,
    pub span: Span,
}

impl LexDiagnostic {
    pub fn new(kind: LexDiagnosticKind, start: u32, end: u32) -> Self {
        Self {
            kind,
            span: Span::new(start, end),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TriviaKind {
    Comment,
    Docstring,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub span: Span,
}

impl Trivia {
    pub fn text<'src>(&self, src: &'src str) -> &'src str {
        debug_assert!(!src.is_empty());
        self.span.slice(src)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Brace {
    Paren,
    Bracket,
    Curly,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Mode {
    Normal,
}

pub struct Lexer<'src> {
    source: &'src str,
    bytes: &'src [u8],
    len: usize,
    pos: usize,
    interner: LocalInterner<'src>,
    indent_stack: Vec<u32>,
    pending: VecDeque<Token>,

    paren_depth: u32,
    at_beginning_of_line: bool,

    diagnostics: Vec<LexDiagnostic>,
    mode_stack: Vec<Mode>,
}

impl<'src> Lexer<'src> {
    pub fn new(input: &'src str) -> Self {
        Self {
            source: input,
            bytes: input.as_bytes(),
            len: input.len(),
            pos: 0,
            interner: LocalInterner::default(),
            indent_stack: Vec::new(),
            pending: VecDeque::new(),
            paren_depth: 0,
            at_beginning_of_line: true,
            diagnostics: Vec::new(),
            mode_stack: Vec::new(),
        }
    }

    #[inline]
    pub fn current_byte(&self) -> u8 {
        if self.pos < self.len {
            return self.bytes[self.pos];
        }
        0
    }

    #[inline]
    pub fn current_char(&self) -> Option<char> {
        if self.pos >= self.len {
            return None;
        }
        let b = self.bytes[self.pos];
        if b < 0x80 {
            return Some(b as char);
        }
        // SAFETY: self.pos is always on a UTF-8 boundary because we only
        // advance by len_utf8() on the slow path and by ASCII byte counts
        // on the fast path. self.source is &str, so bytes from pos onward
        // form valid UTF-8.
        Some(unsafe {
            std::str::from_utf8_unchecked(self.bytes.get_unchecked(self.pos..))
                .chars()
                .next()
                .unwrap_unchecked()
        })
    }

    #[inline]
    pub fn bump_char(&mut self) {
        if self.pos >= self.len {
            return;
        }
        let b = self.bytes[self.pos];
        if b < 0x80 {
            self.pos += 1;
            return;
        }
        self.bump_char_slow();
    }

    #[cold]
    #[inline(never)]
    fn bump_char_slow(&mut self) {
        // SAFETY: same as above — pos is on a UTF-8 boundary.
        let c = unsafe {
            std::str::from_utf8_unchecked(self.bytes.get_unchecked(self.pos..))
                .chars()
                .next()
                .unwrap_unchecked()
        };
        self.pos += c.len_utf8();
    }

    #[inline]
    pub fn bump_ascii(&mut self) {
        if self.pos < self.len {
            self.pos += 1;
        }
    }

    #[inline]
    pub fn peek_byte(&self, offset: usize) -> u8 {
        debug_assert!(offset > 0);
        let idx = self.pos + offset;
        if idx < self.len { self.bytes[idx] } else { 0 }
    }

    #[inline]
    fn measure_indent(&mut self) -> Option<(u32, u32)> {
        loop {
            let bytes = self.bytes;
            let line_start = self.pos;
            let rest = &bytes[line_start..];
            let space_count = rest.iter().position(|&b| b != b' ').unwrap_or(rest.len());
            let after_spaces = line_start + space_count;
            let next = bytes.get(after_spaces).copied();
            match next {
                Some(b'\t') | Some(0x0C) => {
                    return self.measure_indent_mixed(line_start);
                }
                Some(b'\n') => {
                    self.pos = after_spaces + 1;
                    continue;
                }
                Some(b'\r') => {
                    self.pos = after_spaces + 1;
                    if bytes.get(self.pos) == Some(&b'\n') {
                        self.pos += 1;
                    }
                    continue;
                }
                Some(b'#') => {
                    let nl = memchr(b'\n', &bytes[after_spaces..])
                        .map(|n| after_spaces + n + 1)
                        .unwrap_or(bytes.len());
                    self.pos = nl;
                    continue;
                }
                None => {
                    self.pos = after_spaces;
                    return None;
                }
                _ => {
                    self.pos = after_spaces;
                    return Some((line_start as u32, space_count as u32));
                }
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn measure_indent_mixed(&mut self, mut line_start: usize) -> Option<(u32, u32)> {
        let bytes = self.bytes;
        let mut col: u32 = 0;
        let mut i = line_start;
        loop {
            while i < bytes.len() {
                match bytes[i] {
                    b' ' => {
                        col += 1;
                        i += 1;
                    }
                    b'\t' => {
                        col = (col / 8 + 1) * 8;
                        i += 1;
                    }
                    0x0C => {
                        i += 1;
                    } // form feed: ignore
                    _ => break,
                }
            }
            match bytes.get(i).copied() {
                Some(b'\n') => {
                    self.pos = i + 1;
                    col = 0;
                    i = self.pos;
                    line_start = self.pos;
                    continue;
                }
                Some(b'\r') => {
                    self.pos = i + 1;
                    if bytes.get(self.pos) == Some(&b'\n') {
                        self.pos += 1;
                    }
                    col = 0;
                    i = self.pos;
                    line_start = self.pos;
                    continue;
                }
                Some(b'#') => {
                    let nl = memchr(b'\n', &bytes[i..])
                        .map(|n| i + n + 1)
                        .unwrap_or(bytes.len());
                    self.pos = nl;
                    col = 0;
                    i = self.pos;
                    line_start = self.pos;
                    continue;
                }
                None => {
                    self.pos = i;
                    return None;
                }
                _ => {
                    self.pos = i;
                    return Some((line_start as u32, col));
                }
            }
        }
    }
    #[inline]
    fn at_pos(&self, pos: usize) -> u8 {
        self.bytes[pos]
    }

    fn push_diag(&mut self, kind: LexDiagnosticKind, start: u32, end: u32) {
        self.diagnostics.push(LexDiagnostic::new(kind, start, end));
    }

    pub fn lex_simple_string(&mut self, quote_type: u8) -> Token {
        let start = self.pos;
        self.pos += 1; // past opening quote

        loop {
            let bytes = &self.bytes[self.pos..];
            let rel = match memchr3(quote_type, b'\n', b'\\', bytes) {
                Some(r) => r,
                None => {
                    self.pos = self.len;
                    self.push_diag(
                        LexDiagnosticKind::UnterminatedString,
                        start as u32,
                        self.pos as u32,
                    );
                    return Token::new(
                        start as u32,
                        self.pos as u32,
                        TokenKind::UnterminatedString,
                    );
                }
            };

            let b = bytes[rel];

            if b == quote_type {
                self.pos += rel + 1; // past closing quote
                return Token::new(start as u32, self.pos as u32, TokenKind::String);
            } else if b == b'\\' {
                self.pos += rel + 1;
                if self.pos < self.len {
                    self.bump_char();
                }
            } else {
                // newline
                self.pos += rel;
                self.push_diag(
                    LexDiagnosticKind::UnterminatedString,
                    start as u32,
                    self.pos as u32,
                );
                return Token::new(start as u32, self.pos as u32, TokenKind::UnterminatedString);
            }
        }
    }

    fn at_line_start(&mut self) {
        let (start, indent) = match self.measure_indent() {
            Some(p) => p,
            None => return,
        };

        let current = self.indent_stack.last().copied().unwrap_or(0);

        if indent > current {
            self.indent_stack.push(indent);
            self.pending
                .push_back(Token::indent(start, self.pos as u32))
        }

        if indent < current {
            while self.indent_stack.last().copied().unwrap_or(0) > indent {
                self.indent_stack.pop();
                self.pending
                    .push_back(Token::dedent(start, self.pos as u32));
            }

            if self.indent_stack.last().copied().unwrap_or(0) != indent {
                self.push_diag(
                    LexDiagnosticKind::InvalidIndentation,
                    start,
                    self.pos as u32,
                );
            }
        }

        self.at_beginning_of_line = false;
    }

    fn lex_name_or_keyword(&mut self) -> Token {
        let start = self.pos;
        let mut p = self.pos;
        while p < self.len {
            let b = self.bytes[p];
            if is_ascii_id_continue(b) {
                p += 1;
            } else if b < 0x80 {
                break;
            } else {
                self.pos = p;
                return self.lex_name_unicode(start);
            }
        }
        self.pos = p;

        let kind = TokenKind::from_keyword(&self.bytes[start..p]).unwrap_or(TokenKind::Name);
        if kind == TokenKind::Name && p - start > 1 {
            let sym = self.interner.intern(&self.source[start..p]);
            Token::with_symbol(start as u32, p as u32, kind, sym)
        } else {
            Token::new(start as u32, p as u32, kind)
        }
    }

    #[cold]
    #[inline(never)]
    fn lex_name_unicode(&mut self, start: usize) -> Token {
        while let Some(c) = self.current_char() {
            if unicode_ident::is_xid_continue(c) {
                self.bump_char();
            } else {
                break;
            }
        }
        // Non-ASCII identifiers can't be keywords (all keywords are ASCII).
        let sym = self.interner.intern(&self.source[start..self.pos]);
        Token::with_symbol(start as u32, self.pos as u32, TokenKind::Name, sym)
    }

    fn emit_newline(&mut self) -> Option<Token> {
        let start = self.pos as u32;
        match self.current_byte() {
            b'\r' => {
                self.bump_ascii();
                if self.current_byte() == b'\n' {
                    self.bump_ascii();
                }
            }
            b'\n' => {
                self.bump_ascii();
            }
            _ => unreachable!("emit_newline called on non-newline byte"),
        }

        if self.paren_depth > 0 {
            self.at_beginning_of_line = false;
            return None;
        }

        self.at_beginning_of_line = true;
        Some(Token::new(start, self.pos as u32, TokenKind::Newline))
    }

    fn drain_dedents_at_eof(&mut self) {
        let p = self.pos as u32;
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.pending.push_back(Token::dedent(p, p));
        }
        self.pending.push_back(Token::new(p, p, TokenKind::Eof));
    }

    fn lex_number(&mut self) -> Token {
        debug_assert!(self.current_byte().is_ascii_digit());

        let start = self.pos as u32;

        if self.current_byte() == b'0' {
            match self.peek_byte(1) {
                b'X' | b'x' => return self.lex_hex_number(start),
                b'o' | b'O' => return self.lex_octal_number(start),
                b'b' | b'B' => return self.lex_binary_number(start),
                _ => {}
            }
        }

        self.lex_decimal_number(start)
    }

    fn scan_digits<F: Fn(u8) -> bool>(&mut self, valid: F) -> (bool, bool) {
        let mut saw_digit = false;
        let mut prev_underscore = false;
        let mut invalid = false;
        loop {
            let b = self.current_byte();
            if valid(b) {
                saw_digit = true;
                prev_underscore = false;
                self.bump_ascii();
            } else if b == b'_' {
                if !saw_digit || prev_underscore {
                    invalid = true;
                }
                prev_underscore = true;
                self.bump_ascii();
            } else {
                break;
            }
        }
        if prev_underscore {
            invalid = true;
        }
        (saw_digit, invalid)
    }

    fn lex_decimal_number(&mut self, start: u32) -> Token {
        let mut invalid = false;

        let (saw_int, inv) = self.scan_digits(|b: u8| b.is_ascii_digit());
        invalid |= inv;

        if self.current_byte() == b'.' && self.peek_byte(1) != b'.' {
            self.bump_ascii();
            let (_, inv) = self.scan_digits(|b: u8| b.is_ascii_digit());
            invalid |= inv;
        } else if !saw_int {
            invalid = true;
        }

        if matches!(self.current_byte(), b'e' | b'E') {
            self.bump_ascii();
            if matches!(self.current_byte(), b'+' | b'-') {
                self.bump_ascii();
            }
            let (saw_exp, inv) = self.scan_digits(|b: u8| b.is_ascii_digit());
            invalid |= inv;
            if !saw_exp {
                invalid = true;
            }
        }

        if matches!(self.current_byte(), b'j' | b'J') {
            self.bump_ascii();
        }

        while matches!(
            self.current_byte(),
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_'
        ) {
            invalid = true;
            self.bump_ascii();
        }

        if invalid {
            self.push_diag(LexDiagnosticKind::InvalidNumber, start, self.pos as u32);
        }
        Token::new(start, self.pos as u32, TokenKind::Number)
    }

    fn lex_binary_number(&mut self, start: u32) -> Token {
        self.bump_ascii(); // 0
        self.bump_ascii(); // b
        let (saw_digit, mut invalid) = self.scan_digits(|b| matches!(b, b'0' | b'1'));
        if !saw_digit {
            invalid = true;
        }
        while matches!(
            self.current_byte(),
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_'
        ) {
            invalid = true;
            self.bump_ascii();
        }

        if invalid {
            self.push_diag(LexDiagnosticKind::InvalidNumber, start, self.pos as u32);
        }

        Token::new(start, self.pos as u32, TokenKind::Number)
    }

    fn lex_octal_number(&mut self, start: u32) -> Token {
        self.bump_ascii();
        self.bump_ascii();

        let (saw_digit, mut invalid) = self.scan_digits(|b| matches!(b, b'0'..=b'7'));

        if !saw_digit {
            invalid = true
        }

        while matches!(
            self.current_byte(),
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_'
        ) {
            invalid = true;
            self.bump_ascii();
        }

        if invalid {
            self.push_diag(LexDiagnosticKind::InvalidNumber, start, self.pos as u32);
        }

        Token::new(start, self.pos as u32, TokenKind::Number)
    }

    fn lex_hex_number(&mut self, start: u32) -> Token {
        self.bump_ascii();
        self.bump_ascii();

        let (saw_digit, mut invalid) = self.scan_digits(|b| b.is_ascii_hexdigit());

        if !saw_digit {
            invalid = true;
        }

        while matches!(
            self.current_byte(),
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_'
        ) {
            invalid = true;
            self.bump_ascii();
        }

        if invalid {
            self.push_diag(LexDiagnosticKind::InvalidNumber, start, self.pos as u32);
        }

        Token::new(start, self.pos as u32, TokenKind::Number)
    }

    pub fn next_token(&mut self) -> Token {
        todo!()
    }
}

#[inline]
const fn is_ascii_id_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
