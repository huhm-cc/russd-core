use crate::span::Span;
use crate::token::{Token, TokenKind};

pub fn lex(source: &str) -> Result<Vec<Token>, crate::diag::Diagnostic> {
    let mut lx = Lexer {
        src: source,
        pos: 0,
        line: 1,
        col: 1,
    };
    let mut tokens = Vec::new();
    loop {
        let tok = lx.next_token()?;
        let eof = matches!(tok.kind, TokenKind::Eof);
        tokens.push(tok);
        if eof {
            break;
        }
    }
    Ok(tokens)
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut chars = self.src[self.pos..].chars();
        chars.next()?;
        chars.next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn span_from(&self, start: usize, line: usize, col: usize) -> Span {
        Span {
            start,
            end: self.pos,
            line,
            col,
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.bump();
            }
            if self.peek() == Some('/') && self.peek2() == Some('/') {
                while self.peek().is_some() && self.peek() != Some('\n') {
                    self.bump();
                }
                continue;
            }
            break;
        }
    }

    fn error(&self, message: impl Into<String>) -> crate::diag::Diagnostic {
        crate::diag::Diagnostic::new(
            Span {
                start: self.pos,
                end: self.pos,
                line: self.line,
                col: self.col,
            },
            message,
        )
    }

    fn next_token(&mut self) -> Result<Token, crate::diag::Diagnostic> {
        self.skip_ws_and_comments();
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        let Some(ch) = self.peek() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: self.span_from(start, line, col),
            });
        };

        if ch.is_ascii_alphabetic() || ch == '_' {
            return Ok(self.ident_or_kw(start, line, col));
        }
        if ch.is_ascii_digit() {
            return self.number(start, line, col);
        }
        if ch == '"' {
            return self.string(start, line, col);
        }

        let kind = match ch {
            '+' => {
                self.bump();
                TokenKind::Plus
            }
            '-' => {
                self.bump();
                if self.peek() == Some('>') {
                    self.bump();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                self.bump();
                TokenKind::Star
            }
            '/' => {
                self.bump();
                TokenKind::Slash
            }
            '%' => {
                self.bump();
                TokenKind::Percent
            }
            '=' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::EqEq
                } else if self.peek() == Some('>') {
                    self.bump();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                self.bump();
                if self.peek() == Some('&') {
                    self.bump();
                    TokenKind::AndAnd
                } else {
                    TokenKind::And
                }
            }
            '|' => {
                self.bump();
                if self.peek() == Some('|') {
                    self.bump();
                    TokenKind::OrOr
                } else {
                    return Err(self.error("Russd 目前不支援單個 `|`，請用 `||` 或 match 分支"));
                }
            }
            '(' => {
                self.bump();
                TokenKind::LParen
            }
            ')' => {
                self.bump();
                TokenKind::RParen
            }
            '{' => {
                self.bump();
                TokenKind::LBrace
            }
            '}' => {
                self.bump();
                TokenKind::RBrace
            }
            '[' => {
                self.bump();
                TokenKind::LBracket
            }
            ']' => {
                self.bump();
                TokenKind::RBracket
            }
            ',' => {
                self.bump();
                TokenKind::Comma
            }
            '.' => {
                self.bump();
                if self.peek() == Some('.') {
                    self.bump();
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }
            ':' => {
                self.bump();
                TokenKind::Colon
            }
            ';' => {
                self.bump();
                TokenKind::Semi
            }
            '?' => {
                self.bump();
                TokenKind::Question
            }
            _ => {
                return Err(self.error(format!("無法辨識的字元 `{ch}`")));
            }
        };
        Ok(Token {
            kind,
            span: self.span_from(start, line, col),
        })
    }

    fn ident_or_kw(&mut self, start: usize, line: usize, col: usize) -> Token {
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            self.bump();
        }
        let word = &self.src[start..self.pos];
        let kind = TokenKind::keyword(word).unwrap_or_else(|| TokenKind::Ident(word.to_string()));
        Token {
            kind,
            span: self.span_from(start, line, col),
        }
    }

    fn number(
        &mut self,
        start: usize,
        line: usize,
        col: usize,
    ) -> Result<Token, crate::diag::Diagnostic> {
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        let mut suffix_i64 = false;
        if self.peek() == Some('i') {
            let rest = &self.src[self.pos..];
            if rest.starts_with("i64") {
                self.bump();
                self.bump();
                self.bump();
                suffix_i64 = true;
            } else if rest.starts_with("i32") {
                self.bump();
                self.bump();
                self.bump();
            }
        }
        let digits: String = self.src[start..self.pos]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let value: i64 = digits
            .parse()
            .map_err(|_| self.error("整數太大，超出 i64"))?;
        Ok(Token {
            kind: TokenKind::Int { value, suffix_i64 },
            span: self.span_from(start, line, col),
        })
    }

    fn string(
        &mut self,
        start: usize,
        line: usize,
        col: usize,
    ) -> Result<Token, crate::diag::Diagnostic> {
        self.bump(); // "
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(self.error("字串沒有結尾的 `\"`")),
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some(c) => return Err(self.error(format!("未知的跳脫序列 `\\{c}`"))),
                    None => return Err(self.error("字串沒有結尾的 `\"`")),
                },
                Some(c) => out.push(c),
            }
        }
        Ok(Token {
            kind: TokenKind::String(out),
            span: self.span_from(start, line, col),
        })
    }
}
