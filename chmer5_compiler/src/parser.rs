use crate::{
    ast::*,
    lexer::{TokKind, Token},
    ChmerError, Span,
};

pub struct Parser<'a> {
    source_name: &'a str,
    src: &'a str,
    toks: Vec<Token>,
    i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(source_name: &'a str, src: &'a str, toks: Vec<Token>) -> Self {
        Self {
            source_name,
            src,
            toks,
            i: 0,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ChmerError> {
        let mut items = Vec::new();
        while !self.at(TokKind::Eof) {
            if self.at(TokKind::KwImport) {
                items.push(Item::Import(self.parse_import()?));
            } else if self.at(TokKind::KwFunc) {
                items.push(Item::Func(self.parse_func_decl(false)?));
            } else {
                let st = self.parse_stmt()?;
                items.push(Item::Stmt(st));
            }
        }
        Ok(Program { items })
    }

    pub fn parse_ctl_module(&mut self) -> Result<CtlModule, ChmerError> {
        // CTL is a small language: `module <ident>; export func ...`
        let start = self.peek_span().start;
        let module_kw = self.expect_ident_like("module")?;
        if module_kw != "module" {
            return Err(self.err_here("CTL expected 'module'"));
        }
        let name = self.expect_ident()?;
        self.expect(TokKind::Semicolon, "Expected ';' after module name")?;

        let mut exports = Vec::new();
        while !self.at(TokKind::Eof) {
            self.expect(TokKind::KwExport, "CTL expected 'export'")?;
            self.expect(TokKind::KwFunc, "CTL expected 'func'")?;
            exports.push(self.parse_func_decl(true)?);
        }
        Ok(CtlModule {
            name,
            exports,
            span: Span::new(start, self.prev_span().end),
        })
    }

    fn parse_import(&mut self) -> Result<ImportStmt, ChmerError> {
        let kw = self.bump();
        let module = self.expect_ident()?;
        self.expect(TokKind::Semicolon, "Expected ';' after import")?;
        Ok(ImportStmt {
            module,
            span: kw.span,
        })
    }

    fn parse_func_decl(&mut self, already_saw_func_kw: bool) -> Result<FuncDecl, ChmerError> {
        let start = if already_saw_func_kw {
            self.prev_span().start
        } else {
            self.expect(TokKind::KwFunc, "Expected 'func'")?.start
        };
        let name = self.expect_ident()?;
        self.expect(TokKind::LParen, "Expected '(' after function name")?;
        let mut params = Vec::new();
        if !self.at(TokKind::RParen) {
            loop {
                params.push(self.expect_ident()?);
                if self.at(TokKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect(TokKind::RParen, "Expected ')' after parameters")?;
        let body = self.parse_block()?;
        Ok(FuncDecl {
            name,
            params,
            body,
            span: Span::new(start, self.prev_span().end),
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ChmerError> {
        if self.at(TokKind::KwReturn) {
            let kw = self.bump();
            if self.at(TokKind::Semicolon) {
                self.bump();
                return Ok(Stmt::Return(None, kw.span));
            }
            let e = self.parse_expr()?;
            self.expect(TokKind::Semicolon, "Expected ';' after return")?;
            return Ok(Stmt::Return(Some(e), kw.span));
        }

        if self.at(TokKind::KwIf) {
            return self.parse_if();
        }
        if self.at(TokKind::KwWhile) {
            return self.parse_while();
        }
        if self.at(TokKind::LBrace) {
            let start = self.peek_span().start;
            let body = self.parse_block()?;
            let end = self.prev_span().end;
            return Ok(Stmt::Block(body, Span::new(start, end)));
        }

        // assignment vs expr statement
        let expr = self.parse_expr()?;
        if self.at(TokKind::Assign) {
            let eq = self.bump();
            let rhs = self.parse_expr()?;
            self.expect(TokKind::Semicolon, "Expected ';' after assignment")?;
            if !is_assignable(&expr) {
                return Err(self.err_here("Invalid assignment target"));
            }
            if let Expr::Ident(name, _) = expr.clone() {
                return Ok(Stmt::Let {
                    name,
                    expr: rhs,
                    span: eq.span,
                });
            }
            return Ok(Stmt::Assign {
                target: expr,
                expr: rhs,
                span: eq.span,
            });
        }

        let sp = expr.span();
        self.expect(TokKind::Semicolon, "Expected ';'")?;
        Ok(Stmt::Expr(expr, sp))
    }

    fn parse_if(&mut self) -> Result<Stmt, ChmerError> {
        let kw = self.bump();
        self.expect(TokKind::LParen, "Expected '(' after if")?;
        let cond = self.parse_expr()?;
        self.expect(TokKind::RParen, "Expected ')' after if condition")?;
        let then_block = self.parse_block()?;
        let else_block = if self.at(TokKind::KwElse) {
            self.bump();
            self.parse_block()?
        } else {
            Vec::new()
        };
        Ok(Stmt::If {
            cond,
            then_block,
            else_block,
            span: kw.span,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ChmerError> {
        let kw = self.bump();
        self.expect(TokKind::LParen, "Expected '(' after while")?;
        let cond = self.parse_expr()?;
        self.expect(TokKind::RParen, "Expected ')' after while condition")?;
        let body = self.parse_block()?;
        Ok(Stmt::While {
            cond,
            body,
            span: kw.span,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ChmerError> {
        self.expect(TokKind::LBrace, "Expected '{'")?;
        let mut out = Vec::new();
        while !self.at(TokKind::RBrace) && !self.at(TokKind::Eof) {
            out.push(self.parse_stmt()?);
        }
        self.expect(TokKind::RBrace, "Expected '}'")?;
        Ok(out)
    }

    fn parse_expr(&mut self) -> Result<Expr, ChmerError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ChmerError> {
        let mut e = self.parse_and()?;
        while self.at(TokKind::OrOr) {
            let op = self.bump().span;
            let rhs = self.parse_and()?;
            e = Expr::Binary {
                lhs: Box::new(e),
                op: BinaryOp::Or,
                rhs: Box::new(rhs),
                span: op,
            };
        }
        Ok(e)
    }

    fn parse_and(&mut self) -> Result<Expr, ChmerError> {
        let mut e = self.parse_equality()?;
        while self.at(TokKind::AndAnd) {
            let op = self.bump().span;
            let rhs = self.parse_equality()?;
            e = Expr::Binary {
                lhs: Box::new(e),
                op: BinaryOp::And,
                rhs: Box::new(rhs),
                span: op,
            };
        }
        Ok(e)
    }

    fn parse_equality(&mut self) -> Result<Expr, ChmerError> {
        let mut e = self.parse_compare()?;
        loop {
            if self.at(TokKind::EqEq) {
                let s = self.bump().span;
                let rhs = self.parse_compare()?;
                e = Expr::Binary {
                    lhs: Box::new(e),
                    op: BinaryOp::Eq,
                    rhs: Box::new(rhs),
                    span: s,
                };
            } else if self.at(TokKind::BangEq) {
                let s = self.bump().span;
                let rhs = self.parse_compare()?;
                e = Expr::Binary {
                    lhs: Box::new(e),
                    op: BinaryOp::Neq,
                    rhs: Box::new(rhs),
                    span: s,
                };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_compare(&mut self) -> Result<Expr, ChmerError> {
        let mut e = self.parse_term()?;
        loop {
            let op = if self.at(TokKind::Lt) {
                Some(BinaryOp::Lt)
            } else if self.at(TokKind::LtEq) {
                Some(BinaryOp::LtEq)
            } else if self.at(TokKind::Gt) {
                Some(BinaryOp::Gt)
            } else if self.at(TokKind::GtEq) {
                Some(BinaryOp::GtEq)
            } else {
                None
            };
            if let Some(op) = op {
                let s = self.bump().span;
                let rhs = self.parse_term()?;
                e = Expr::Binary {
                    lhs: Box::new(e),
                    op,
                    rhs: Box::new(rhs),
                    span: s,
                };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_term(&mut self) -> Result<Expr, ChmerError> {
        let mut e = self.parse_factor()?;
        loop {
            let op = if self.at(TokKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.at(TokKind::Minus) {
                Some(BinaryOp::Sub)
            } else {
                None
            };
            if let Some(op) = op {
                let s = self.bump().span;
                let rhs = self.parse_factor()?;
                e = Expr::Binary {
                    lhs: Box::new(e),
                    op,
                    rhs: Box::new(rhs),
                    span: s,
                };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_factor(&mut self) -> Result<Expr, ChmerError> {
        let mut e = self.parse_unary()?;
        loop {
            let op = if self.at(TokKind::Star) {
                Some(BinaryOp::Mul)
            } else if self.at(TokKind::Slash) {
                Some(BinaryOp::Div)
            } else if self.at(TokKind::Percent) {
                Some(BinaryOp::Mod)
            } else {
                None
            };
            if let Some(op) = op {
                let s = self.bump().span;
                let rhs = self.parse_unary()?;
                e = Expr::Binary {
                    lhs: Box::new(e),
                    op,
                    rhs: Box::new(rhs),
                    span: s,
                };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> Result<Expr, ChmerError> {
        if self.at(TokKind::Bang) {
            let s = self.bump().span;
            let rhs = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                rhs: Box::new(rhs),
                span: s,
            });
        }
        if self.at(TokKind::Minus) {
            let s = self.bump().span;
            let rhs = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                rhs: Box::new(rhs),
                span: s,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ChmerError> {
        let mut e = self.parse_primary()?;
        loop {
            if self.at(TokKind::LParen) {
                let start = e.span().start;
                self.bump();
                let mut args = Vec::new();
                if !self.at(TokKind::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.at(TokKind::Comma) {
                            self.bump();
                            continue;
                        }
                        break;
                    }
                }
                let end = self.expect(TokKind::RParen, "Expected ')'")?.end;
                e = Expr::Call {
                    callee: Box::new(e),
                    args,
                    span: Span::new(start, end),
                };
                continue;
            }
            if self.at(TokKind::Dot) {
                self.bump();
                let field = self.expect_ident()?;
                let span = Span::new(e.span().start, self.prev_span().end);
                e = Expr::Member {
                    object: Box::new(e),
                    field,
                    span,
                };
                continue;
            }
            if self.at(TokKind::LBracket) {
                let start = e.span().start;
                self.bump();
                let idx = self.parse_expr()?;
                let end = self.expect(TokKind::RBracket, "Expected ']'")?.end;
                e = Expr::Index {
                    object: Box::new(e),
                    index: Box::new(idx),
                    span: Span::new(start, end),
                };
                continue;
            }
            break;
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> Result<Expr, ChmerError> {
        let t = self.peek().clone();
        match t.kind {
            TokKind::KwNull => {
                self.bump();
                Ok(Expr::Null(t.span))
            }
            TokKind::KwTrue => {
                self.bump();
                Ok(Expr::Bool(true, t.span))
            }
            TokKind::KwFalse => {
                self.bump();
                Ok(Expr::Bool(false, t.span))
            }
            TokKind::Int(v) => {
                self.bump();
                Ok(Expr::Int(v, t.span))
            }
            TokKind::Float(v) => {
                self.bump();
                Ok(Expr::Float(v, t.span))
            }
            TokKind::Str(s) => {
                self.bump();
                Ok(Expr::Str(s, t.span))
            }
            TokKind::Ident(name) => {
                self.bump();
                Ok(Expr::Ident(name, t.span))
            }
            TokKind::LParen => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect(TokKind::RParen, "Expected ')'")?;
                Ok(e)
            }
            TokKind::LBracket => self.parse_array(),
            TokKind::LBrace => self.parse_map_literal(),
            _ => Err(self.err_here("Expected expression")),
        }
    }

    fn parse_array(&mut self) -> Result<Expr, ChmerError> {
        let start = self.expect(TokKind::LBracket, "Expected '['")?.start;
        let mut elems = Vec::new();
        if !self.at(TokKind::RBracket) {
            loop {
                elems.push(self.parse_expr()?);
                if self.at(TokKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let end = self.expect(TokKind::RBracket, "Expected ']'")?.end;
        Ok(Expr::Array(elems, Span::new(start, end)))
    }

    fn parse_map_literal(&mut self) -> Result<Expr, ChmerError> {
        // map literal uses `{ key: value, ... }` in CHMER.
        let start = self.expect(TokKind::LBrace, "Expected '{'")?.start;
        let mut pairs = Vec::new();
        if !self.at(TokKind::RBrace) {
            loop {
                let key = self.parse_expr()?;
                self.expect(TokKind::Colon, "Expected ':' in map literal key/value")?;
                let val = self.parse_expr()?;
                pairs.push((key, val));
                if self.at(TokKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let end = self.expect(TokKind::RBrace, "Expected '}'")?.end;
        Ok(Expr::Map(pairs, Span::new(start, end)))
    }

    // -------------- helpers --------------

    fn at(&self, kind: TokKind) -> bool {
        use TokKind::*;
        match (&self.peek().kind, kind) {
            (LParen, LParen)
            | (RParen, RParen)
            | (LBrace, LBrace)
            | (RBrace, RBrace)
            | (LBracket, LBracket)
            | (RBracket, RBracket)
            | (Comma, Comma)
            | (Dot, Dot)
            | (Colon, Colon)
            | (Semicolon, Semicolon)
            | (Plus, Plus)
            | (Minus, Minus)
            | (Star, Star)
            | (Slash, Slash)
            | (Percent, Percent)
            | (Bang, Bang)
            | (Assign, Assign)
            | (EqEq, EqEq)
            | (BangEq, BangEq)
            | (Lt, Lt)
            | (LtEq, LtEq)
            | (Gt, Gt)
            | (GtEq, GtEq)
            | (AndAnd, AndAnd)
            | (OrOr, OrOr)
            | (KwFunc, KwFunc)
            | (KwReturn, KwReturn)
            | (KwIf, KwIf)
            | (KwElse, KwElse)
            | (KwWhile, KwWhile)
            | (KwFor, KwFor)
            | (KwBreak, KwBreak)
            | (KwContinue, KwContinue)
            | (KwConst, KwConst)
            | (KwNull, KwNull)
            | (KwTrue, KwTrue)
            | (KwFalse, KwFalse)
            | (KwTry, KwTry)
            | (KwCatch, KwCatch)
            | (KwSwitch, KwSwitch)
            | (KwCase, KwCase)
            | (KwDefault, KwDefault)
            | (KwAsync, KwAsync)
            | (KwThread, KwThread)
            | (KwClass, KwClass)
            | (KwStruct, KwStruct)
            | (KwEnum, KwEnum)
            | (KwExport, KwExport)
            | (KwImport, KwImport)
            | (Eof, Eof) => true,
            _ => false,
        }
    }

    fn peek(&self) -> &Token {
        &self.toks[self.i]
    }

    fn peek_span(&self) -> Span {
        self.peek().span
    }

    fn prev_span(&self) -> Span {
        self.toks[self.i.saturating_sub(1)].span
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.i].clone();
        self.i += 1;
        t
    }

    fn expect(&mut self, kind: TokKind, msg: &str) -> Result<Span, ChmerError> {
        if self.at(kind) {
            Ok(self.bump().span)
        } else {
            Err(self.err_here(msg))
        }
    }

    fn expect_ident(&mut self) -> Result<String, ChmerError> {
        match self.peek().kind.clone() {
            TokKind::Ident(s) => {
                self.bump();
                Ok(s)
            }
            _ => Err(self.err_here("Expected identifier")),
        }
    }

    fn expect_ident_like(&mut self, _example: &str) -> Result<String, ChmerError> {
        self.expect_ident()
    }

    fn err_here(&self, msg: impl Into<String>) -> ChmerError {
        ChmerError::syntax(self.source_name, self.src, self.peek_span(), msg.into())
    }
}

fn is_assignable(expr: &Expr) -> bool {
    matches!(expr, Expr::Ident(_, _) | Expr::Member { .. } | Expr::Index { .. })
}

