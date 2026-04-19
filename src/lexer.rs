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

impl<'src> Lexer<'src> {
    pub fn new(input: &'src str) -> Self {
        let mut lexer = Self {
            input: input,
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
        if next >= self.bytes.len(){
            0
        } else{
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
}
