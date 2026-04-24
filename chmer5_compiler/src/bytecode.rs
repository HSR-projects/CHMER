use std::collections::HashMap;

use indexmap::IndexMap;

use crate::{ast::*, ChmerError, Span};

#[derive(Debug, Clone)]
pub enum ConstVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Debug, Clone)]
pub enum Op {
    Const(u32),
    MakeFunc(u32),
    LoadGlobal(u32),
    StoreGlobal(u32),
    LoadLocal(u32),
    StoreLocal(u32),
    Pop,

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
    Not,
    Neg,
    And,
    Or,

    JumpIfFalse(u32),
    Jump(u32),

    Call(u32),
    Return,

    GetField(u32),
    SetField(u32),

    MakeArray(u32),
    IndexGet,
    IndexSet,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub arity: u32,
    pub code: Vec<Op>,
    pub consts: Vec<ConstVal>,
    pub globals: IndexMap<String, u32>,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub entry: Function,
    pub functions: Vec<Function>,
    pub imports: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleArtifact {
    pub name: String,
    pub chunk: Chunk,
    pub exports: Vec<String>,
}

pub struct Compiler<'a> {
    source_name: &'a str,
}

impl<'a> Compiler<'a> {
    pub fn new(source_name: &'a str) -> Self {
        Self { source_name }
    }

    pub fn compile_program(&mut self, p: &Program) -> Result<Chunk, ChmerError> {
        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut func_name_to_index = HashMap::<String, u32>::new();

        // gather top-level functions
        for item in &p.items {
            match item {
                Item::Import(im) => imports.push(im.module.clone()),
                Item::Func(f) => {
                    let idx = functions.len() as u32;
                    func_name_to_index.insert(f.name.clone(), idx);
                    functions.push(self.compile_func(f, &IndexMap::new())?);
                }
                Item::Stmt(_) => {}
            }
        }

        // entry function is top-level statements executed in order
        let mut entry = FunctionBuilder::new("<main>");
        // bind top-level functions as global values
        for (name, idx) in func_name_to_index.iter() {
            entry.emit(Op::MakeFunc(*idx));
            let gid = entry.global_id(name);
            entry.emit(Op::StoreGlobal(gid));
            entry.emit(Op::Pop);
        }
        for item in &p.items {
            match item {
                Item::Import(_) | Item::Func(_) => {}
                Item::Stmt(s) => entry.compile_stmt(self.source_name, s, &mut functions)?,
            }
        }
        let nul = entry.add_const(ConstVal::Null);
        entry.emit(Op::Const(nul));
        entry.emit(Op::Return);

        Ok(Chunk {
            entry: entry.finish(),
            functions,
            imports,
        })
    }

    pub fn compile_ctl_module(&mut self, m: &CtlModule) -> Result<ModuleArtifact, ChmerError> {
        let mut functions = Vec::new();
        let mut exports = Vec::new();
        for f in &m.exports {
            exports.push(f.name.clone());
            functions.push(self.compile_func(f, &IndexMap::new())?);
        }

        // CTL modules have empty entry; exports are invoked by importers.
        let mut entry = FunctionBuilder::new("<module_init>");
        let nul = entry.add_const(ConstVal::Null);
        entry.emit(Op::Const(nul));
        entry.emit(Op::Return);

        Ok(ModuleArtifact {
            name: m.name.clone(),
            chunk: Chunk {
                entry: entry.finish(),
                functions,
                imports: Vec::new(),
            },
            exports,
        })
    }

    fn compile_func(
        &mut self,
        f: &FuncDecl,
        _globals: &IndexMap<String, u32>,
    ) -> Result<Function, ChmerError> {
        let mut fb = FunctionBuilder::new(&f.name);
        fb.arity = f.params.len() as u32;
        for (idx, p) in f.params.iter().enumerate() {
            fb.locals.insert(p.clone(), idx as u32);
        }
        let mut functions = Vec::new();
        for st in &f.body {
            fb.compile_stmt(self.source_name, st, &mut functions)?;
        }
        let nul = fb.add_const(ConstVal::Null);
        fb.emit(Op::Const(nul));
        fb.emit(Op::Return);
        Ok(fb.finish())
    }
}

struct FunctionBuilder {
    name: String,
    arity: u32,
    code: Vec<Op>,
    consts: Vec<ConstVal>,
    globals: IndexMap<String, u32>,
    locals: HashMap<String, u32>,
    next_local: u32,
}

impl FunctionBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            arity: 0,
            code: Vec::new(),
            consts: Vec::new(),
            globals: IndexMap::new(),
            locals: HashMap::new(),
            next_local: 0,
        }
    }

    fn finish(self) -> Function {
        Function {
            name: self.name,
            arity: self.arity,
            code: self.code,
            consts: self.consts,
            globals: self.globals,
        }
    }

    fn add_const(&mut self, c: ConstVal) -> u32 {
        self.consts.push(c);
        (self.consts.len() - 1) as u32
    }

    fn emit(&mut self, op: Op) -> u32 {
        self.code.push(op);
        (self.code.len() - 1) as u32
    }

    fn patch_jump(&mut self, at: u32, target: u32) {
        match &mut self.code[at as usize] {
            Op::JumpIfFalse(t) | Op::Jump(t) => *t = target,
            _ => {}
        }
    }

    fn global_id(&mut self, name: &str) -> u32 {
        if let Some(v) = self.globals.get(name) {
            *v
        } else {
            let id = self.globals.len() as u32;
            self.globals.insert(name.to_string(), id);
            id
        }
    }

    fn compile_stmt(
        &mut self,
        source_name: &str,
        s: &Stmt,
        functions: &mut Vec<Function>,
    ) -> Result<(), ChmerError> {
        match s {
            Stmt::Expr(e, _) => {
                self.compile_expr(source_name, e, functions)?;
                self.emit(Op::Pop);
            }
            Stmt::Assign { target, expr, .. } => {
                // assign supports `ident = expr` and `obj.field = expr` and `obj[idx] = expr`.
                match target {
                    Expr::Ident(name, _) => {
                        self.compile_expr(source_name, expr, functions)?;
                        let gid = self.global_id(name);
                        self.emit(Op::StoreGlobal(gid));
                    }
                    Expr::Member { object, field, .. } => {
                        self.compile_expr(source_name, object, functions)?;
                        self.compile_expr(source_name, expr, functions)?;
                        let fid = self.global_id(field);
                        self.emit(Op::SetField(fid));
                    }
                    Expr::Index { object, index, .. } => {
                        self.compile_expr(source_name, object, functions)?;
                        self.compile_expr(source_name, index, functions)?;
                        self.compile_expr(source_name, expr, functions)?;
                        self.emit(Op::IndexSet);
                    }
                    _ => {
                        return Err(ChmerError::compile(
                            source_name,
                            "",
                            Span::new(0, 0),
                            "Invalid assignment target",
                        ));
                    }
                }
            }
            Stmt::Return(opt, _) => {
                if let Some(e) = opt {
                    self.compile_expr(source_name, e, functions)?;
                } else {
                    let n = self.add_const(ConstVal::Null);
                    self.emit(Op::Const(n));
                }
                self.emit(Op::Return);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.compile_expr(source_name, cond, functions)?;
                let jf = self.emit(Op::JumpIfFalse(0));
                for st in then_block {
                    self.compile_stmt(source_name, st, functions)?;
                }
                let jend = self.emit(Op::Jump(0));
                let else_start = self.code.len() as u32;
                self.patch_jump(jf, else_start);
                for st in else_block {
                    self.compile_stmt(source_name, st, functions)?;
                }
                let end = self.code.len() as u32;
                self.patch_jump(jend, end);
            }
            Stmt::While { cond, body, .. } => {
                let loop_start = self.code.len() as u32;
                self.compile_expr(source_name, cond, functions)?;
                let jf = self.emit(Op::JumpIfFalse(0));
                for st in body {
                    self.compile_stmt(source_name, st, functions)?;
                }
                self.emit(Op::Jump(loop_start));
                let end = self.code.len() as u32;
                self.patch_jump(jf, end);
            }
            Stmt::Block(body, _) => {
                for st in body {
                    self.compile_stmt(source_name, st, functions)?;
                }
            }
            Stmt::Let { name, expr, .. } => {
                self.compile_expr(source_name, expr, functions)?;
                let gid = self.global_id(name);
                self.emit(Op::StoreGlobal(gid));
            }
        }
        Ok(())
    }

    fn compile_expr(
        &mut self,
        source_name: &str,
        e: &Expr,
        functions: &mut Vec<Function>,
    ) -> Result<(), ChmerError> {
        match e {
            Expr::Null(_) => {
                let n = self.add_const(ConstVal::Null);
                self.emit(Op::Const(n));
            }
            Expr::Bool(v, _) => {
                let n = self.add_const(ConstVal::Bool(*v));
                self.emit(Op::Const(n));
            }
            Expr::Int(v, _) => {
                let n = self.add_const(ConstVal::Int(*v));
                self.emit(Op::Const(n));
            }
            Expr::Float(v, _) => {
                let n = self.add_const(ConstVal::Float(*v));
                self.emit(Op::Const(n));
            }
            Expr::Str(s, _) => {
                let n = self.add_const(ConstVal::Str(s.clone()));
                self.emit(Op::Const(n));
            }
            Expr::Ident(name, _) => {
                if let Some(lid) = self.locals.get(name) {
                    self.emit(Op::LoadLocal(*lid));
                } else {
                    let gid = self.global_id(name);
                    self.emit(Op::LoadGlobal(gid));
                }
            }
            Expr::Unary { op, rhs, .. } => {
                self.compile_expr(source_name, rhs, functions)?;
                match op {
                    UnaryOp::Not => self.emit(Op::Not),
                    UnaryOp::Neg => self.emit(Op::Neg),
                };
            }
            Expr::Binary { lhs, op, rhs, .. } => {
                self.compile_expr(source_name, lhs, functions)?;
                self.compile_expr(source_name, rhs, functions)?;
                self.emit(match op {
                    BinaryOp::Add => Op::Add,
                    BinaryOp::Sub => Op::Sub,
                    BinaryOp::Mul => Op::Mul,
                    BinaryOp::Div => Op::Div,
                    BinaryOp::Mod => Op::Mod,
                    BinaryOp::Eq => Op::Eq,
                    BinaryOp::Neq => Op::Neq,
                    BinaryOp::Lt => Op::Lt,
                    BinaryOp::LtEq => Op::LtEq,
                    BinaryOp::Gt => Op::Gt,
                    BinaryOp::GtEq => Op::GtEq,
                    BinaryOp::And => Op::And,
                    BinaryOp::Or => Op::Or,
                });
            }
            Expr::Call { callee, args, .. } => {
                self.compile_expr(source_name, callee, functions)?;
                for a in args {
                    self.compile_expr(source_name, a, functions)?;
                }
                self.emit(Op::Call(args.len() as u32));
            }
            Expr::Member { object, field, .. } => {
                self.compile_expr(source_name, object, functions)?;
                let fid = self.global_id(field);
                self.emit(Op::GetField(fid));
            }
            Expr::Array(elems, _) => {
                for el in elems {
                    self.compile_expr(source_name, el, functions)?;
                }
                self.emit(Op::MakeArray(elems.len() as u32));
            }
            Expr::Index { object, index, .. } => {
                self.compile_expr(source_name, object, functions)?;
                self.compile_expr(source_name, index, functions)?;
                self.emit(Op::IndexGet);
            }
            Expr::Map(_, span) => {
                return Err(ChmerError::compile(
                    source_name,
                    "",
                    *span,
                    "Map literals not yet supported in VM runtime (coming next)",
                ));
            }
        }
        Ok(())
    }
}

