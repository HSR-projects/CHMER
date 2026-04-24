use crate::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Import(ImportStmt),
    Func(FuncDecl),
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct ImportStmt {
    pub module: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CtlModule {
    pub name: String,
    pub exports: Vec<FuncDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr, Span),
    Let { name: String, expr: Expr, span: Span },
    Assign { target: Expr, expr: Expr, span: Span },
    Return(Option<Expr>, Span),
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Vec<Stmt>,
        span: Span,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Block(Vec<Stmt>, Span),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Null(Span),
    Bool(bool, Span),
    Int(i64, Span),
    Float(f64, Span),
    Str(String, Span),
    Ident(String, Span),
    Array(Vec<Expr>, Span),
    Map(Vec<(Expr, Expr)>, Span),
    Unary {
        op: UnaryOp,
        rhs: Box<Expr>,
        span: Span,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Member {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Null(s)
            | Expr::Bool(_, s)
            | Expr::Int(_, s)
            | Expr::Float(_, s)
            | Expr::Str(_, s)
            | Expr::Ident(_, s)
            | Expr::Array(_, s)
            | Expr::Map(_, s) => *s,
            Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Call { span, .. }
            | Expr::Member { span, .. }
            | Expr::Index { span, .. } => *span,
        }
    }
}

