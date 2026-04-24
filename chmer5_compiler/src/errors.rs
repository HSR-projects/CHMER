use std::fmt;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone)]
pub struct SourcePos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct SourceLoc {
    pub name: String,
    pub pos: SourcePos,
}

#[derive(Debug, Error)]
pub enum ChmerError {
    #[error("{kind}")]
    Diagnostic { kind: ErrorKind, loc: Option<SourceLoc> },
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    SyntaxError(String),
    LexError(String),
    ImportError(String),
    CompileError(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::SyntaxError(msg) => write!(f, "SyntaxError: {msg}"),
            ErrorKind::LexError(msg) => write!(f, "LexError: {msg}"),
            ErrorKind::ImportError(msg) => write!(f, "ImportError: {msg}"),
            ErrorKind::CompileError(msg) => write!(f, "CompileError: {msg}"),
        }
    }
}

pub fn byte_offset_to_line_col(src: &str, offset: usize) -> SourcePos {
    let mut line = 1usize;
    let mut col = 1usize;
    let mut i = 0usize;
    for ch in src.chars() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        i += ch.len_utf8();
    }
    SourcePos { line, col }
}

impl ChmerError {
    pub fn syntax(source_name: &str, src: &str, span: Span, msg: impl Into<String>) -> Self {
        let pos = byte_offset_to_line_col(src, span.start);
        ChmerError::Diagnostic {
            kind: ErrorKind::SyntaxError(msg.into()),
            loc: Some(SourceLoc {
                name: source_name.to_string(),
                pos,
            }),
        }
    }

    pub fn lex(source_name: &str, src: &str, span: Span, msg: impl Into<String>) -> Self {
        let pos = byte_offset_to_line_col(src, span.start);
        ChmerError::Diagnostic {
            kind: ErrorKind::LexError(msg.into()),
            loc: Some(SourceLoc {
                name: source_name.to_string(),
                pos,
            }),
        }
    }

    pub fn compile(source_name: &str, src: &str, span: Span, msg: impl Into<String>) -> Self {
        let pos = byte_offset_to_line_col(src, span.start);
        ChmerError::Diagnostic {
            kind: ErrorKind::CompileError(msg.into()),
            loc: Some(SourceLoc {
                name: source_name.to_string(),
                pos,
            }),
        }
    }
}

