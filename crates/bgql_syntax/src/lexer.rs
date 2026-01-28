//! Lexer for Better GraphQL.

use crate::token::{Token, TokenKind};
use bgql_core::{Interner, Span, Text};

/// A lexer for Better GraphQL source code.
pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: u32,
    interner: &'a Interner,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer.
    pub fn new(source: &'a str, interner: &'a Interner) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            interner,
        }
    }

    /// Returns the current position.
    #[inline]
    pub fn pos(&self) -> u32 {
        self.pos
    }

    /// Peeks at the current byte without consuming.
    #[inline]
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos as usize).copied()
    }

    /// Peeks at the byte at offset from current position.
    #[inline]
    fn peek_at(&self, offset: u32) -> Option<u8> {
        self.bytes.get((self.pos + offset) as usize).copied()
    }

    /// Advances by one byte.
    #[inline]
    fn advance(&mut self) {
        self.pos += 1;
    }

    /// Advances by n bytes.
    #[inline]
    fn advance_by(&mut self, n: u32) {
        self.pos += n;
    }

    /// Returns true if at end of input.
    #[inline]
    #[allow(dead_code)]
    fn is_eof(&self) -> bool {
        self.pos as usize >= self.bytes.len()
    }

    /// Gets the slice from start to current position.
    #[inline]
    fn slice_from(&self, start: u32) -> &'a str {
        &self.source[start as usize..self.pos as usize]
    }

    /// Scans the next token.
    pub fn next_token(&mut self) -> Token {
        self.skip_trivia();

        let start = self.pos;

        let Some(c) = self.peek() else {
            return Token::new(TokenKind::Eof, Span::new(start, start));
        };

        let kind = match c {
            // Punctuation
            b'{' => {
                self.advance();
                TokenKind::LBrace
            }
            b'}' => {
                self.advance();
                TokenKind::RBrace
            }
            b'(' => {
                self.advance();
                TokenKind::LParen
            }
            b')' => {
                self.advance();
                TokenKind::RParen
            }
            b'[' => {
                self.advance();
                TokenKind::LBracket
            }
            b']' => {
                self.advance();
                TokenKind::RBracket
            }
            b'<' => {
                self.advance();
                TokenKind::LAngle
            }
            b'>' => {
                self.advance();
                TokenKind::RAngle
            }
            b':' => {
                self.advance();
                if self.peek() == Some(b':') {
                    self.advance();
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }
            b',' => {
                self.advance();
                TokenKind::Comma
            }
            b'.' => {
                if self.peek_at(1) == Some(b'.') && self.peek_at(2) == Some(b'.') {
                    self.advance_by(3);
                    TokenKind::Spread
                } else {
                    self.advance();
                    TokenKind::Dot
                }
            }
            b'=' => {
                self.advance();
                TokenKind::Eq
            }
            b'|' => {
                self.advance();
                TokenKind::Pipe
            }
            b'&' => {
                self.advance();
                TokenKind::Amp
            }
            b'@' => {
                self.advance();
                TokenKind::At
            }
            b'!' => {
                self.advance();
                TokenKind::Bang
            }
            b'?' => {
                self.advance();
                TokenKind::Question
            }
            b'$' => {
                self.advance();
                TokenKind::Dollar
            }
            b';' => {
                self.advance();
                TokenKind::Semicolon
            }
            b'*' => {
                self.advance();
                TokenKind::Star
            }

            // String literals
            b'"' => self.scan_string(),

            // Numbers
            b'-' | b'0'..=b'9' => self.scan_number(),

            // Identifiers and keywords
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.scan_identifier(),

            _ => {
                self.advance();
                TokenKind::Error
            }
        };

        Token::new(kind, Span::new(start, self.pos))
    }

    /// Skips whitespace, newlines, and comments.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => {
                    self.advance();
                }
                Some(b'#') => {
                    // Comment - skip to end of line
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some(0xEF) if self.peek_at(1) == Some(0xBB) && self.peek_at(2) == Some(0xBF) => {
                    // UTF-8 BOM
                    self.advance_by(3);
                }
                _ => break,
            }
        }
    }

    /// Scans an identifier or keyword.
    fn scan_identifier(&mut self) -> TokenKind {
        let start = self.pos;

        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let text = self.slice_from(start);

        // Check for keywords
        TokenKind::from_keyword(text).unwrap_or(TokenKind::Ident)
    }

    /// Scans a number literal.
    fn scan_number(&mut self) -> TokenKind {
        let mut is_float = false;

        // Optional negative sign
        if self.peek() == Some(b'-') {
            self.advance();
        }

        // Integer part
        if self.peek() == Some(b'0') {
            self.advance();
        } else {
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Fractional part
        if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.advance(); // .
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Exponent part
        if let Some(b'e' | b'E') = self.peek() {
            is_float = true;
            self.advance();
            if let Some(b'+' | b'-') = self.peek() {
                self.advance();
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if is_float {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        }
    }

    /// Scans a string literal.
    fn scan_string(&mut self) -> TokenKind {
        self.advance(); // Opening quote

        // Check for block string
        if self.peek() == Some(b'"') && self.peek_at(1) == Some(b'"') {
            self.advance_by(2);
            return self.scan_block_string();
        }

        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    return TokenKind::Error;
                }
                Some(b'"') => {
                    self.advance();
                    return TokenKind::StringLiteral;
                }
                Some(b'\\') => {
                    self.advance();
                    self.advance(); // Escaped char
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Scans a block string literal.
    fn scan_block_string(&mut self) -> TokenKind {
        loop {
            match self.peek() {
                None => {
                    return TokenKind::Error;
                }
                Some(b'"') if self.peek_at(1) == Some(b'"') && self.peek_at(2) == Some(b'"') => {
                    self.advance_by(3);
                    return TokenKind::BlockStringLiteral;
                }
                Some(b'\\')
                    if self.peek_at(1) == Some(b'"')
                        && self.peek_at(2) == Some(b'"')
                        && self.peek_at(3) == Some(b'"') =>
                {
                    self.advance_by(4); // Escaped triple quote
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Interns the text at the given span.
    pub fn intern_span(&self, span: Span) -> Text {
        let text = &self.source[span.start as usize..span.end as usize];
        self.interner.intern(text)
    }

    /// Gets the text at the given span.
    pub fn span_text(&self, span: Span) -> &'a str {
        &self.source[span.start as usize..span.end as usize]
    }
}

/// Tokenizes the entire source.
pub fn tokenize(source: &str, interner: &Interner) -> Vec<Token> {
    let mut lexer = Lexer::new(source, interner);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(token);
        if is_eof {
            break;
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_punctuation() {
        let interner = Interner::new();
        let tokens = tokenize("{ } ( ) [ ] < > : :: , . ... = | & @ ! ? $", &interner);

        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::LAngle,
                TokenKind::RAngle,
                TokenKind::Colon,
                TokenKind::ColonColon,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Spread,
                TokenKind::Eq,
                TokenKind::Pipe,
                TokenKind::Amp,
                TokenKind::At,
                TokenKind::Bang,
                TokenKind::Question,
                TokenKind::Dollar,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let interner = Interner::new();
        let tokens = tokenize(
            "type interface enum input scalar opaque Option List",
            &interner,
        );

        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Type,
                TokenKind::Interface,
                TokenKind::Enum,
                TokenKind::Input,
                TokenKind::Scalar,
                TokenKind::Opaque,
                TokenKind::Option,
                TokenKind::List,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_numbers() {
        let interner = Interner::new();
        let tokens = tokenize("42 -17 3.14 1e10 2.5e-3", &interner);

        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::IntLiteral,
                TokenKind::IntLiteral,
                TokenKind::FloatLiteral,
                TokenKind::FloatLiteral,
                TokenKind::FloatLiteral,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_strings() {
        let interner = Interner::new();
        let tokens = tokenize(r#""hello" "world" """block string""""#, &interner);

        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::StringLiteral,
                TokenKind::StringLiteral,
                TokenKind::BlockStringLiteral,
                TokenKind::Eof,
            ]
        );
    }
}
