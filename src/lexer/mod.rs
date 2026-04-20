#[cfg(test)]
mod tests;

use super::tokens::{Token, TokenKind};
pub struct Lexer<'src> {
    input: &'src str,
    bytes: &'src [u8],
    position: usize,
    read_position: usize,
    ch: u8,

    indent_stack: Vec<u32>,
    at_line_start: bool,
    pending_dedents: u32,
    indent_char: Option<u8>,
    paren_depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexError {
    MixedTabsAndSpaces,
    InconsistentIndentation,
}

impl<'src> Lexer<'src> {
    pub fn new(input: &'src str) -> Self {
        let mut lexer = Self {
            input,
            bytes: input.as_bytes(),
            position: 0,
            read_position: 0,
            ch: 0,
            indent_stack: vec![0],
            at_line_start: true,
            pending_dedents: 0,
            indent_char: None,
            paren_depth: 0,
        };
        lexer.read_char();
        lexer
    }

    fn read_char(&mut self) {
        if self.read_position >= self.bytes.len() {
            self.ch = 0;
        } else {
            self.ch = self.bytes[self.read_position];
        }
        self.position = self.read_position;
        self.read_position += 1;
    }

    fn peek(&self) -> u8 {
        if self.read_position >= self.bytes.len() {
            0
        } else {
            self.bytes[self.read_position]
        }
    }

    fn peek_ahead(&self, delta: usize) -> u8 {
        let next = self.read_position.saturating_add(delta);
        if next >= self.bytes.len() {
            0
        } else {
            self.bytes[next]
        }
    }

    fn skip_comment(&mut self) {
        while self.ch != b'\n' && self.ch != 0 {
            self.read_char();
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.ch {
                b' ' => self.read_char(),
                b'#' => self.skip_comment(),
                _ => break,
            }
        }
    }

    fn is_ident_start(&self) -> bool {
        self.ch.is_ascii_alphabetic() || self.ch == b'_'
    }

    fn is_ident_continuation(&self) -> bool {
        self.ch.is_ascii_alphanumeric() || self.ch == b'_'
    }

    fn lex_number(&mut self) -> Token {
        let start = self.position as u32;

        if self.ch == b'0' {
            match self.peek() {
                b'x' | b'X' => {
                    self.read_char();
                    self.read_char();
                    while self.ch != 0 && self.ch.is_ascii_hexdigit() {
                        self.read_char();
                    }
                    return Token::new(TokenKind::Number, start, self.position as u32);
                }
                b'b' | b'B' => {
                    self.read_char();
                    self.read_char();
                    while matches!(self.ch, b'0' | b'1') {
                        self.read_char();
                    }
                    return Token::new(TokenKind::Number, start, self.position as u32);
                }
                b'o' | b'O' => {
                    self.read_char();
                    self.read_char();
                    while matches!(self.ch, b'0'..=b'7') {
                        self.read_char();
                    }
                    return Token::new(TokenKind::Number, start, self.position as u32);
                }
                _ => {}
            }
        }

        while self.ch.is_ascii_digit() {
            self.read_char();
        }

        if self.ch == b'.' && self.peek().is_ascii_digit() {
            self.read_char();
            while self.ch.is_ascii_digit() {
                self.read_char();
            }
        }

        Token::new(TokenKind::Number, start, self.position as u32)
    }

    fn consume_leading_indent(&mut self) -> Result<(u32, u8, bool), LexError> {
        let mut count: u32 = 0u32;
        let mut seen_space = false;
        let mut seen_tab = false;

        loop {
            match self.ch {
                b' ' => {
                    seen_space = true;
                    count += 1;
                    self.read_char();
                }
                b'\t' => {
                    seen_tab = true;
                    count += 1;
                    self.read_char();
                }

                first_non_indent => {
                    if seen_space && seen_tab {
                        return Err(LexError::MixedTabsAndSpaces);
                    }

                    if count > 0 {
                        match self.indent_char {
                            Some(b' ') if seen_tab => {
                                return Err(LexError::InconsistentIndentation);
                            }
                            Some(b'\t') if seen_space => {
                                return Err(LexError::InconsistentIndentation);
                            }
                            None if seen_tab => {
                                self.indent_char = Some(b'\t');
                            }
                            None if seen_space => {
                                self.indent_char = Some(b' ');
                            }
                            _ => {}
                        }
                    }
                    return Ok((count, first_non_indent, count > 0));
                }
            }
        }
    }

    fn read_multiline_string(&mut self, quote_type: u8) -> TokenKind {
        for _ in 0..3 {
            self.read_char();
        }

        loop {
            if self.ch == 0 {
                return TokenKind::UnterminatedString;
            }

            if self.ch == quote_type
                && self.peek() == quote_type
                && self.peek_ahead(1) == quote_type
            {
                for _ in 0..3 {
                    self.read_char();
                }
                return TokenKind::String;
            }

            self.read_char();
        }
    }

    fn read_simple_string(&mut self, quote_type: u8) -> TokenKind {
        self.read_char();

        while self.ch != 0 {
            if self.ch == quote_type {
                break;
            }

            if self.ch == b'\\' {
                self.read_char();

                if self.ch == 0 {
                    return TokenKind::UnterminatedString;
                }

                self.read_char();
                continue;
            }

            self.read_char();
        }

        if self.ch != quote_type {
            return TokenKind::UnterminatedString;
        }

        self.read_char();
        TokenKind::String
    }

    fn string_prefix_length(&self) -> Option<(usize, bool, bool, bool)> {
        let ch = self.ch;
        let next = self.peek();

        match (ch, next) {
            (b'f' | b'F', b'\'' | b'"') => Some((1, false, true, false)),
            (b'r' | b'R', b'\'' | b'"') => Some((1, true, false, false)),
            (b'r' | b'R', b'f' | b'F') => Some((2, true, true, false)),
            (b'f' | b'F', b'r' | b'R') => Some((2, true, true, false)),
            (b'b' | b'B', b'\'' | b'"') => Some((1, false, false, true)),
            (b'b' | b'B', b'r' | b'R') => Some((2, true, false, true)),
            (b'r' | b'R', b'b' | b'B') => Some((2, true, false, true)),
            _ => None,
        }
    }

    fn read_raw_string(&mut self, quote_type: u8) -> TokenKind {
        self.read_char();

        while self.ch != 0 {
            if self.ch == quote_type && !self.has_odd_trailing_backslashes() {
                break;
            }

            self.read_char();
        }

        if self.ch != quote_type {
            return TokenKind::UnterminatedString;
        }

        self.read_char();
        TokenKind::String
    }

    fn read_raw_multiline_string(&mut self, quote_type: u8) -> TokenKind {
        for _ in 0..3 {
            self.read_char();
        }

        loop {
            if self.ch == 0 {
                return TokenKind::UnterminatedString;
            }

            if self.ch == quote_type
                && self.peek() == quote_type
                && self.peek_ahead(1) == quote_type
                && !self.has_odd_trailing_backslashes()
            {
                for _ in 0..3 {
                    self.read_char();
                }
                return TokenKind::String;
            }

            self.read_char();
        }
    }

    fn read_f_string(&mut self, raw: bool) -> TokenKind {
        let quote_type = self.ch;
        let is_triple = self.peek() == quote_type && self.peek_ahead(1) == quote_type;

        if is_triple {
            for _ in 0..3 {
                self.read_char();
            }

            loop {
                if self.ch == 0 {
                    return TokenKind::UnterminatedString;
                }

                if self.ch == quote_type
                    && self.peek() == quote_type
                    && self.peek_ahead(1) == quote_type
                {
                    for _ in 0..3 {
                        self.read_char();
                    }
                    return TokenKind::FString;
                }

                if !raw && self.ch == b'\\' {
                    self.read_char();
                    if self.ch == 0 {
                        return TokenKind::UnterminatedString;
                    }
                }

                self.read_char();
            }
        }

        self.read_char();

        while self.ch != 0 {
            if !raw && self.ch == b'\\' {
                self.read_char();
                if self.ch == 0 {
                    return TokenKind::UnterminatedString;
                }
                self.read_char();
                continue;
            }

            if self.ch == quote_type {
                self.read_char();
                return TokenKind::FString;
            }

            self.read_char();
        }

        TokenKind::UnterminatedString
    }

    fn has_odd_trailing_backslashes(&self) -> bool {
        if self.position == 0 {
            return false;
        }

        let mut i = self.position;
        let mut count = 0usize;

        while i > 0 {
            let b = self.bytes[i - 1];
            if b == b'\\' {
                count += 1;
                i -= 1;
            } else {
                break;
            }
        }

        count % 2 == 1
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            if self.pending_dedents > 0 {
                self.pending_dedents -= 1;
                let pos = self.position as u32;
                return Token::new(TokenKind::Dedent, pos, pos);
            }

            if self.at_line_start && self.paren_depth == 0 {
                let pos = self.position as u32;

                match self.consume_leading_indent() {
                    Err(_) => {
                        return Token::new(TokenKind::Illegal, pos, pos);
                    }
                    Ok((spaces, first_non_indent, consumed)) => {
                        if first_non_indent != b'\n'
                            && first_non_indent != b'#'
                            && first_non_indent != 0
                        {
                            let current = *self.indent_stack.last().unwrap();

                            if spaces > current {
                                self.indent_stack.push(spaces);
                                self.at_line_start = false;
                                return Token::new(TokenKind::Indent, pos, pos);
                            }

                            if spaces < current {
                                let mut dedent_count = 0u32;

                                while self.indent_stack.len() > 1
                                    && *self.indent_stack.last().unwrap() > spaces
                                {
                                    self.indent_stack.pop();
                                    dedent_count += 1;
                                }

                                if *self.indent_stack.last().unwrap() != spaces {
                                    return Token::new(TokenKind::Illegal, pos, pos);
                                }

                                if dedent_count > 1 {
                                    self.pending_dedents = dedent_count - 1;
                                }

                                self.at_line_start = false;
                                return Token::new(TokenKind::Dedent, pos, pos);
                            }

                            self.at_line_start = false;
                        } else if consumed {
                            self.at_line_start = false;
                        }
                    }
                }
            }

            if self.at_line_start && self.paren_depth > 0 {
                self.at_line_start = false;
            }

            self.skip_whitespace_and_comments();

            if self.ch == 0 {
                let pos = self.position as u32;

                if self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    return Token::new(TokenKind::Dedent, pos, pos);
                }

                return Token::new(TokenKind::Eof, pos, pos);
            }

            if self.ch == b'\n' {
                let start = self.position as u32;
                self.read_char();

                if self.paren_depth > 0 {
                    self.at_line_start = false;
                    continue;
                }

                self.at_line_start = true;
                return Token::new(TokenKind::Newline, start, self.position as u32);
            }

            break;
        }

        if let Some((kind, width)) = TokenKind::from_operator_bytes(&self.bytes[self.position..]) {
            let start = self.position as u32;

            for _ in 0..width {
                self.read_char();
            }

            match kind {
                TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace => {
                    self.paren_depth += 1;
                }
                TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                    if self.paren_depth > 0 {
                        self.paren_depth -= 1;
                    }
                }
                _ => {}
            }

            return Token::new(kind, start, self.position as u32);
        }

        if self.ch.is_ascii_digit() {
            return self.lex_number();
        }

        if self.is_ident_start() {
            if let Some((prefix_len, raw, fstring, bstring)) = self.string_prefix_length() {
                let quote_pos = self.position + prefix_len;

                if quote_pos < self.bytes.len()
                    && (self.bytes[quote_pos] == b'\'' || self.bytes[quote_pos] == b'"')
                {
                    let start = self.position as u32;
                    let quote = self.bytes[quote_pos];
                    let is_triple = quote_pos + 2 < self.bytes.len()
                        && self.bytes[quote_pos + 1] == quote
                        && self.bytes[quote_pos + 2] == quote;

                    for _ in 0..prefix_len {
                        self.read_char();
                    }

                    let typ = if fstring {
                        self.read_f_string(raw)
                    } else if raw {
                        let base = if is_triple {
                            self.read_raw_multiline_string(quote)
                        } else {
                            self.read_raw_string(quote)
                        };

                        if bstring && base == TokenKind::String {
                            TokenKind::BString
                        } else {
                            base
                        }
                    } else {
                        let base = if is_triple {
                            self.read_multiline_string(quote)
                        } else {
                            self.read_simple_string(quote)
                        };

                        if bstring && base == TokenKind::String {
                            TokenKind::BString
                        } else {
                            base
                        }
                    };

                    return Token::new(typ, start, self.position as u32);
                }
            }

            let start = self.position as u32;

            while self.is_ident_continuation() {
                self.read_char();
            }

            let text = &self.input[start as usize..self.position];
            let kind = TokenKind::from_keyword(text).unwrap_or(TokenKind::Name);

            return Token::new(kind, start, self.position as u32);
        }

        if self.ch == b'\"' || self.ch == b'\'' {
            let start = self.position as u32;

            let typ = if self.ch == b'\'' && self.peek() == b'\'' && self.peek_ahead(1) == b'\'' {
                self.read_multiline_string(b'\'')
            } else if self.ch == b'"' && self.peek() == b'"' && self.peek_ahead(1) == b'"' {
                self.read_multiline_string(b'"')
            } else {
                self.read_simple_string(self.ch)
            };
            return Token::new(typ, start, self.position as u32);
        }

        let start = self.position as u32;
        self.read_char();
        Token::new(TokenKind::Illegal, start, self.position as u32)
    }
}
