use crate::error::CompileError;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    At,
    LBracket,
    RBracket,
    Colon,
    Equal,
    Arrow,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    AndAnd,
    OrOr,
    String(String),
    Number(String),
    Identifier(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
                continue;
            }

            let line = self.line;
            let column = self.column;
            let kind = match ch {
                '@' => self.single(TokenKind::At),
                '[' => self.single(TokenKind::LBracket),
                ']' => self.single(TokenKind::RBracket),
                ':' => self.single(TokenKind::Colon),
                '-' if self.peek_next() == Some('>') => self.double(TokenKind::Arrow),
                '{' => self.single(TokenKind::LBrace),
                '}' => self.single(TokenKind::RBrace),
                '(' => self.single(TokenKind::LParen),
                ')' => self.single(TokenKind::RParen),
                ',' => self.single(TokenKind::Comma),
                '+' => self.single(TokenKind::Plus),
                '-' => self.single(TokenKind::Minus),
                '*' => self.single(TokenKind::Star),
                '%' => self.single(TokenKind::Percent),
                '!' if self.peek_next() == Some('=') => self.double(TokenKind::BangEqual),
                '!' => self.single(TokenKind::Bang),
                '=' if self.peek_next() == Some('=') => self.double(TokenKind::EqualEqual),
                '=' => self.single(TokenKind::Equal),
                '<' if self.peek_next() == Some('=') => self.double(TokenKind::LessEqual),
                '<' => self.single(TokenKind::Less),
                '>' if self.peek_next() == Some('=') => self.double(TokenKind::GreaterEqual),
                '>' => self.single(TokenKind::Greater),
                '&' if self.peek_next() == Some('&') => self.double(TokenKind::AndAnd),
                '|' if self.peek_next() == Some('|') => self.double(TokenKind::OrOr),
                '/' if self.peek_next() == Some('/') => {
                    self.skip_line_comment();
                    continue;
                }
                '/' => self.single(TokenKind::Slash),
                '"' => TokenKind::String(self.read_string()?),
                c if c.is_ascii_digit() => TokenKind::Number(self.read_number()),
                c if is_identifier_start(c) => TokenKind::Identifier(self.read_identifier()),
                _ => {
                    return Err(CompileError::lexer(
                        line,
                        column,
                        format!("Unexpected character '{ch}'"),
                    ));
                }
            };

            tokens.push(Token { kind, line, column });
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
            column: self.column,
        });
        Ok(tokens)
    }

    fn single(&mut self, kind: TokenKind) -> TokenKind {
        self.advance();
        kind
    }

    fn double(&mut self, kind: TokenKind) -> TokenKind {
        self.advance();
        self.advance();
        kind
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.current).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.current + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.current += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn read_string(&mut self) -> Result<String, CompileError> {
        let start_line = self.line;
        let start_column = self.column;
        self.advance();
        let mut value = String::new();

        loop {
            match self.advance() {
                Some('"') => return Ok(value),
                Some('\\') => {
                    let escaped = self.advance().ok_or_else(|| {
                        CompileError::lexer(start_line, start_column, "Unterminated escape sequence")
                    })?;
                    match escaped {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '0' => value.push('\0'),
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        other => {
                            return Err(CompileError::lexer(
                                self.line,
                                self.column.saturating_sub(1),
                                format!("Unsupported escape sequence \\{other}"),
                            ));
                        }
                    }
                }
                Some('\n') => {
                    return Err(CompileError::lexer(
                        start_line,
                        start_column,
                        "A string literal cannot contain an unescaped newline",
                    ));
                }
                Some(ch) => value.push(ch),
                None => {
                    return Err(CompileError::lexer(
                        start_line,
                        start_column,
                        "Unterminated string literal",
                    ));
                }
            }
        }
    }

    fn read_number(&mut self) -> String {
        let mut value = String::new();
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            value.push(self.advance().expect("peeked character"));
        }
        if self.peek() == Some('.') && self.peek_next().is_some_and(|ch| ch.is_ascii_digit()) {
            value.push(self.advance().expect("peeked decimal point"));
            while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                value.push(self.advance().expect("peeked character"));
            }
        }
        value
    }

    fn read_identifier(&mut self) -> String {
        let mut value = String::new();
        while self.peek().is_some_and(is_identifier_continue) {
            value.push(self.advance().expect("peeked character"));
        }
        value
    }

    fn skip_line_comment(&mut self) {
        self.advance();
        self.advance();
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}
