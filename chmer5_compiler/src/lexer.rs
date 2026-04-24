use crate::{ChmerError, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokKind {
    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,

    // operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Assign,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,

    // literals/idents
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),

    // keywords
    KwFunc,
    KwReturn,
    KwIf,
    KwElse,
    KwWhile,
    KwFor,
    KwBreak,
    KwContinue,
    KwConst,
    KwNull,
    KwTrue,
    KwFalse,
    KwTry,
    KwCatch,
    KwSwitch,
    KwCase,
    KwDefault,
    KwAsync,
    KwThread,
    KwClass,
    KwStruct,
    KwEnum,
    KwExport, // CTL only

    // special import syntax: "(#import)" followed by Ident then ";"
    KwImport,

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    source_name: &'a str,
    src: &'a str,
    bytes: &'a [u8],
    i: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source_name: &'a str, src: &'a str) -> Self {
        Self {
            source_name,
            src,
            bytes: src.as_bytes(),
            i: 0,
        }
    }

    pub fn lex(mut self) -> Result<Vec<Token>, ChmerError> {
        let mut out = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let start = self.i;
            if self.i >= self.bytes.len() {
                out.push(Token {
                    kind: TokKind::Eof,
                    span: Span::new(self.i, self.i),
                });
                return Ok(out);
            }

            // Special: "(#import)"
            if self.peek_bytes(b"(#import)") {
                self.i += b"(#import)".len();
                out.push(Token {
                    kind: TokKind::KwImport,
                    span: Span::new(start, self.i),
                });
                continue;
            }

            let b = self.bytes[self.i];
            match b {
                b'(' => {
                    self.i += 1;
                    out.push(tok(TokKind::LParen, start, self.i));
                }
                b')' => {
                    self.i += 1;
                    out.push(tok(TokKind::RParen, start, self.i));
                }
                b'{' => {
                    self.i += 1;
                    out.push(tok(TokKind::LBrace, start, self.i));
                }
                b'}' => {
                    self.i += 1;
                    out.push(tok(TokKind::RBrace, start, self.i));
                }
                b'[' => {
                    self.i += 1;
                    out.push(tok(TokKind::LBracket, start, self.i));
                }
                b']' => {
                    self.i += 1;
                    out.push(tok(TokKind::RBracket, start, self.i));
                }
                b',' => {
                    self.i += 1;
                    out.push(tok(TokKind::Comma, start, self.i));
                }
                b'.' => {
                    self.i += 1;
                    out.push(tok(TokKind::Dot, start, self.i));
                }
                b':' => {
                    self.i += 1;
                    out.push(tok(TokKind::Colon, start, self.i));
                }
                b';' => {
                    self.i += 1;
                    out.push(tok(TokKind::Semicolon, start, self.i));
                }
                b'+' => {
                    self.i += 1;
                    out.push(tok(TokKind::Plus, start, self.i));
                }
                b'-' => {
                    self.i += 1;
                    out.push(tok(TokKind::Minus, start, self.i));
                }
                b'*' => {
                    self.i += 1;
                    out.push(tok(TokKind::Star, start, self.i));
                }
                b'/' => {
                    self.i += 1;
                    out.push(tok(TokKind::Slash, start, self.i));
                }
                b'%' => {
                    self.i += 1;
                    out.push(tok(TokKind::Percent, start, self.i));
                }
                b'!' => {
                    self.i += 1;
                    if self.match_byte(b'=') {
                        out.push(tok(TokKind::BangEq, start, self.i));
                    } else {
                        out.push(tok(TokKind::Bang, start, self.i));
                    }
                }
                b'=' => {
                    self.i += 1;
                    if self.match_byte(b'=') {
                        out.push(tok(TokKind::EqEq, start, self.i));
                    } else {
                        out.push(tok(TokKind::Assign, start, self.i));
                    }
                }
                b'<' => {
                    self.i += 1;
                    if self.match_byte(b'=') {
                        out.push(tok(TokKind::LtEq, start, self.i));
                    } else {
                        out.push(tok(TokKind::Lt, start, self.i));
                    }
                }
                b'>' => {
                    self.i += 1;
                    if self.match_byte(b'=') {
                        out.push(tok(TokKind::GtEq, start, self.i));
                    } else {
                        out.push(tok(TokKind::Gt, start, self.i));
                    }
                }
                b'&' => {
                    self.i += 1;
                    if self.match_byte(b'&') {
                        out.push(tok(TokKind::AndAnd, start, self.i));
                    } else {
                        return Err(ChmerError::lex(
                            self.source_name,
                            self.src,
                            Span::new(start, self.i),
                            "Expected '&&'",
                        ));
                    }
                }
                b'|' => {
                    self.i += 1;
                    if self.match_byte(b'|') {
                        out.push(tok(TokKind::OrOr, start, self.i));
                    } else {
                        return Err(ChmerError::lex(
                            self.source_name,
                            self.src,
                            Span::new(start, self.i),
                            "Expected '||'",
                        ));
                    }
                }
                b'"' => {
                    let s = self.lex_string()?;
                    out.push(Token {
                        kind: TokKind::Str(s),
                        span: Span::new(start, self.i),
                    });
                }
                b'0'..=b'9' => {
                    let (kind, end) = self.lex_number()?;
                    out.push(Token {
                        kind,
                        span: Span::new(start, end),
                    });
                }
                _ => {
                    if is_ident_start(b) {
                        let ident = self.lex_ident();
                        let kind = keyword_or_ident(ident);
                        out.push(Token {
                            kind,
                            span: Span::new(start, self.i),
                        });
                    } else {
                        self.i += 1;
                        return Err(ChmerError::lex(
                            self.source_name,
                            self.src,
                            Span::new(start, self.i),
                            format!("Unexpected character: '{}'", b as char),
                        ));
                    }
                }
            }
        }
    }

    fn lex_string(&mut self) -> Result<String, ChmerError> {
        // assumes current is '"'
        let start = self.i;
        self.i += 1;
        let mut out = String::new();
        while self.i < self.bytes.len() {
            let b = self.bytes[self.i];
            match b {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.i += 1;
                    if self.i >= self.bytes.len() {
                        break;
                    }
                    let esc = self.bytes[self.i];
                    self.i += 1;
                    match esc {
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        _ => {
                            return Err(ChmerError::lex(
                                self.source_name,
                                self.src,
                                Span::new(start, self.i),
                                "Unknown escape in string",
                            ));
                        }
                    }
                }
                _ => {
                    out.push(b as char);
                    self.i += 1;
                }
            }
        }
        Err(ChmerError::lex(
            self.source_name,
            self.src,
            Span::new(start, self.i),
            "Unterminated string literal",
        ))
    }

    fn lex_number(&mut self) -> Result<(TokKind, usize), ChmerError> {
        let start = self.i;
        while self.i < self.bytes.len() && matches!(self.bytes[self.i], b'0'..=b'9') {
            self.i += 1;
        }
        let mut is_float = false;
        if self.i < self.bytes.len() && self.bytes[self.i] == b'.' {
            if self.i + 1 < self.bytes.len() && matches!(self.bytes[self.i + 1], b'0'..=b'9') {
                is_float = true;
                self.i += 1; // dot
                while self.i < self.bytes.len() && matches!(self.bytes[self.i], b'0'..=b'9') {
                    self.i += 1;
                }
            }
        }
        let s = &self.src[start..self.i];
        if is_float {
            let v = s.parse::<f64>().map_err(|_| {
                ChmerError::lex(
                    self.source_name,
                    self.src,
                    Span::new(start, self.i),
                    "Invalid float literal",
                )
            })?;
            Ok((TokKind::Float(v), self.i))
        } else {
            let v = s.parse::<i64>().map_err(|_| {
                ChmerError::lex(
                    self.source_name,
                    self.src,
                    Span::new(start, self.i),
                    "Invalid int literal",
                )
            })?;
            Ok((TokKind::Int(v), self.i))
        }
    }

    fn lex_ident(&mut self) -> String {
        let start = self.i;
        self.i += 1;
        while self.i < self.bytes.len() && is_ident_continue(self.bytes[self.i]) {
            self.i += 1;
        }
        self.src[start..self.i].to_string()
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self.i < self.bytes.len() {
                let b = self.bytes[self.i];
                if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                    self.i += 1;
                } else {
                    break;
                }
            }
            if self.i < self.bytes.len() && self.bytes[self.i] == b'#' {
                while self.i < self.bytes.len() && self.bytes[self.i] != b'\n' {
                    self.i += 1;
                }
                continue;
            }
            break;
        }
    }

    fn match_byte(&mut self, b: u8) -> bool {
        if self.i < self.bytes.len() && self.bytes[self.i] == b {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn peek_bytes(&self, pat: &[u8]) -> bool {
        self.bytes.get(self.i..self.i + pat.len()) == Some(pat)
    }
}

fn tok(kind: TokKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span::new(start, end),
    }
}

fn is_ident_start(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'_')
}

fn is_ident_continue(b: u8) -> bool {
    is_ident_start(b) || matches!(b, b'0'..=b'9' | b'-')
}

fn keyword_or_ident(s: String) -> TokKind {
    match s.as_str() {
        "func" => TokKind::KwFunc,
        "return" => TokKind::KwReturn,
        "if" => TokKind::KwIf,
        "else" => TokKind::KwElse,
        "while" => TokKind::KwWhile,
        "for" => TokKind::KwFor,
        "break" => TokKind::KwBreak,
        "continue" => TokKind::KwContinue,
        "const" => TokKind::KwConst,
        "null" => TokKind::KwNull,
        "true" => TokKind::KwTrue,
        "false" => TokKind::KwFalse,
        "try" => TokKind::KwTry,
        "catch" => TokKind::KwCatch,
        "switch" => TokKind::KwSwitch,
        "case" => TokKind::KwCase,
        "default" => TokKind::KwDefault,
        "async" => TokKind::KwAsync,
        "thread" => TokKind::KwThread,
        "class" => TokKind::KwClass,
        "struct" => TokKind::KwStruct,
        "enum" => TokKind::KwEnum,
        "export" => TokKind::KwExport,
        _ => TokKind::Ident(s),
    }
}

