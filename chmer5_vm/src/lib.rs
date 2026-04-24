use std::{collections::HashMap, fmt, rc::Rc, sync::Mutex};

use chmer5_compiler::{Chunk, ConstVal, Function, Op};
use thiserror::Error;
use tiny_http::{Header, Method, Response, Server};
use eframe::egui;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<String>),
    Array(Rc<Vec<Value>>),
    Object(Rc<HashMap<String, Value>>),
    NativeFunc(NativeFunc),
    BoundMethod { recv: Rc<HashMap<String, Value>>, func: NativeFunc },
    Func(usize), // index into loaded functions table; entry is special-cased
}

pub type NativeFunc = fn(&mut Vm, Vec<Value>) -> Result<Value, VmError>;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Array(a) => write!(f, "{:?}", a),
            Value::Object(_) => write!(f, "[object]"),
            Value::NativeFunc(_) => write!(f, "[native func]"),
            Value::BoundMethod { .. } => write!(f, "[method]"),
            Value::Func(_) => write!(f, "[func]"),
        }
    }
}

#[derive(Debug, Error)]
pub enum VmError {
    #[error("RuntimeError: {0}")]
    Runtime(String),
    #[error("ImportError: {0}")]
    Import(String),
}

#[derive(Debug)]
pub struct Vm {
    globals: HashMap<String, Value>,
    functions: Vec<Function>,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    natives: HashMap<String, NativeFunc>,
}

#[derive(Debug)]
struct Frame {
    func: Function,
    ip: usize,
    locals: Vec<Value>,
    global_id_to_name: Vec<String>,
}

impl Vm {
    pub fn new() -> Self {
        let mut vm = Self {
            globals: HashMap::new(),
            functions: Vec::new(),
            stack: Vec::new(),
            frames: Vec::new(),
            natives: HashMap::new(),
        };
        vm.install_builtins();
        vm
    }

    fn install_builtins(&mut self) {
        self.natives.insert("print".to_string(), native_print as NativeFunc);
        self.natives
            .insert("inet_server".to_string(), native_inet_server as NativeFunc);
        self.natives
            .insert("inet_route".to_string(), native_inet_route as NativeFunc);
        self.natives
            .insert("inet_route_text".to_string(), native_inet_route_text as NativeFunc);
        self.natives
            .insert("inet_start".to_string(), native_inet_start as NativeFunc);
        self.natives
            .insert("chess_board".to_string(), native_chess_board_id as NativeFunc);
        self.natives
            .insert("chess_board_load".to_string(), native_chess_board_load as NativeFunc);
        self.natives.insert(
            "chess_board_legalmoves".to_string(),
            native_chess_board_legalmoves as NativeFunc,
        );
        self.natives.insert(
            "chess_render_board_html".to_string(),
            native_chess_render_board_html_id as NativeFunc,
        );
    }

    pub fn run(&mut self, chunk: Chunk) -> Result<Value, VmError> {
        self.load_globals(&chunk.entry);
        self.functions = chunk.functions;
        let entry = chunk.entry;
        self.push_frame(entry, vec![])?;
        self.exec_loop()
    }

    fn load_globals(&mut self, f: &Function) {
        self.globals.clear();

        // bind native functions
        for (name, n) in self.natives.iter() {
            self.globals.insert(name.clone(), Value::NativeFunc(*n));
        }

        // prebind builtin module objects into globals if present
        // modules are injected lazily by name lookup.
        let _ = f;
        self.globals.insert("chess".to_string(), module_chess_object());
        self.globals.insert("inet".to_string(), module_inet_object());
        self.globals.insert("gui".to_string(), module_gui_object());
        self.globals
            .insert("sys".to_string(), Value::Object(Rc::new(HashMap::new())));
    }

    fn push_frame(&mut self, func: Function, args: Vec<Value>) -> Result<(), VmError> {
        if args.len() != func.arity as usize {
            return Err(VmError::Runtime(format!(
                "Arity mismatch calling {} (expected {}, got {})",
                func.name,
                func.arity,
                args.len()
            )));
        }
        let mut id_to_name = vec![String::new(); func.globals.len()];
        for (k, v) in func.globals.iter() {
            id_to_name[*v as usize] = k.clone();
        }
        self.frames.push(Frame {
            func,
            ip: 0,
            locals: args,
            global_id_to_name: id_to_name,
        });
        Ok(())
    }

    fn exec_loop(&mut self) -> Result<Value, VmError> {
        loop {
            if self.frames.is_empty() {
                return Ok(Value::Null);
            }
            let op = {
                let frame = self.frames.last_mut().ok_or_else(|| {
                    VmError::Runtime("Internal VM error: no active frame".to_string())
                })?;
                if frame.ip >= frame.func.code.len() {
                    return Err(VmError::Runtime("Instruction pointer out of bounds".to_string()));
                }
                let op = frame.func.code[frame.ip].clone();
                frame.ip += 1;
                op
            };

            match op {
                Op::Const(i) => {
                    let c = {
                        let frame = self.frames.last().unwrap();
                        frame.func.consts[i as usize].clone()
                    };
                    self.stack.push(const_to_val(&c));
                }
                Op::MakeFunc(idx) => {
                    self.stack.push(Value::Func(idx as usize));
                }
                Op::Pop => {
                    self.stack.pop();
                }
                Op::LoadGlobal(i) => {
                    let name = {
                        let frame = self.frames.last().unwrap();
                        frame
                            .global_id_to_name
                            .get(i as usize)
                            .cloned()
                            .unwrap_or_default()
                    };
                    let v = self.globals.get(&name).cloned().unwrap_or(Value::Null);
                    self.stack.push(v);
                }
                Op::StoreGlobal(i) => {
                    let v = self.pop()?;
                    let name = {
                        let frame = self.frames.last().unwrap();
                        frame
                            .global_id_to_name
                            .get(i as usize)
                            .cloned()
                            .unwrap_or_default()
                    };
                    if !name.is_empty() {
                        self.globals.insert(name, v.clone());
                    }
                    self.stack.push(v);
                }
                Op::LoadLocal(i) => {
                    let v = {
                        let frame = self.frames.last().unwrap();
                        frame.locals.get(i as usize).cloned().unwrap_or(Value::Null)
                    };
                    self.stack.push(v);
                }
                Op::StoreLocal(i) => {
                    let v = self.pop()?;
                    let idx = i as usize;
                    {
                        let frame = self.frames.last_mut().unwrap();
                        if frame.locals.len() <= idx {
                            frame.locals.resize(idx + 1, Value::Null);
                        }
                        frame.locals[idx] = v.clone();
                    }
                    self.stack.push(v);
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(bin_num(op, a, b)?);
                }
                Op::Eq | Op::Neq | Op::Lt | Op::LtEq | Op::Gt | Op::GtEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(cmp(op, a, b)?);
                }
                Op::Not => {
                    let v = self.pop()?;
                    self.stack.push(Value::Bool(!truthy(&v)));
                }
                Op::Neg => {
                    let v = self.pop()?;
                    match v {
                        Value::Int(i) => self.stack.push(Value::Int(-i)),
                        Value::Float(x) => self.stack.push(Value::Float(-x)),
                        _ => return Err(VmError::Runtime("Unary '-' on non-number".into())),
                    }
                }
                Op::And => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Bool(truthy(&a) && truthy(&b)));
                }
                Op::Or => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Bool(truthy(&a) || truthy(&b)));
                }
                Op::JumpIfFalse(target) => {
                    let v = self.pop()?;
                    if !truthy(&v) {
                        let frame = self.frames.last_mut().unwrap();
                        frame.ip = target as usize;
                    }
                }
                Op::Jump(target) => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.ip = target as usize;
                }
                Op::Call(argc) => {
                    let mut args = Vec::with_capacity(argc as usize);
                    for _ in 0..argc {
                        args.push(self.pop()?);
                    }
                    args.reverse();
                    let callee = self.pop()?;
                    let ret = self.call_value(callee, args)?;
                    self.stack.push(ret);
                }
                Op::Return => {
                    let ret = self.pop().unwrap_or(Value::Null);
                    self.frames.pop();
                    if self.frames.is_empty() {
                        return Ok(ret);
                    } else {
                        self.stack.push(ret);
                    }
                }
                Op::MakeArray(n) => {
                    let mut elems = Vec::with_capacity(n as usize);
                    for _ in 0..n {
                        elems.push(self.pop()?);
                    }
                    elems.reverse();
                    self.stack.push(Value::Array(Rc::new(elems)));
                }
                Op::IndexGet => {
                    let idx = self.pop()?;
                    let obj = self.pop()?;
                    self.stack.push(index_get(obj, idx)?);
                }
                Op::IndexSet => {
                    let val = self.pop()?;
                    let idx = self.pop()?;
                    let obj = self.pop()?;
                    self.stack.push(index_set(obj, idx, val)?);
                }
                Op::GetField(fid) => {
                    let obj = self.pop()?;
                    let field = {
                        let frame = self.frames.last().unwrap();
                        frame
                            .global_id_to_name
                            .get(fid as usize)
                            .cloned()
                            .unwrap_or_default()
                    };
                    self.stack.push(get_field(obj, &field)?);
                }
                Op::SetField(fid) => {
                    let val = self.pop()?;
                    let obj = self.pop()?;
                    let field = {
                        let frame = self.frames.last().unwrap();
                        frame
                            .global_id_to_name
                            .get(fid as usize)
                            .cloned()
                            .unwrap_or_default()
                    };
                    self.stack.push(set_field(obj, &field, val)?);
                }
            }
        }
    }

    fn call_value(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, VmError> {
        match callee {
            Value::NativeFunc(f) => f(self, args),
            Value::BoundMethod { recv, func } => {
                let mut a = Vec::with_capacity(args.len() + 1);
                a.push(Value::Object(recv));
                a.extend(args);
                func(self, a)
            }
            Value::Func(idx) => {
                let func = self
                    .functions
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| VmError::Runtime("Unknown function".into()))?;
                let depth = self.frames.len();
                let stack_len = self.stack.len();
                self.push_frame(func, args)?;
                let ret = self.exec_until_depth(depth);
                self.stack.truncate(stack_len);
                ret
            }
            Value::Object(obj) => {
                // calling object: look for __call
                if let Some(v) = obj.get("__call") {
                    self.call_value(v.clone(), args)
                } else {
                    Err(VmError::Runtime("Value is not callable".into()))
                }
            }
            _ => Err(VmError::Runtime("Value is not callable".into())),
        }
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack
            .pop()
            .ok_or_else(|| VmError::Runtime("Stack underflow".into()))
    }

    fn exec_until_depth(&mut self, depth: usize) -> Result<Value, VmError> {
        // Run until the VM returns to `depth` frames, then return the callee's return value.
        while self.frames.len() > depth {
            let _ = self.exec_one()?;
        }
        self.pop()
    }

    fn exec_one(&mut self) -> Result<(), VmError> {
        if self.frames.is_empty() {
            return Ok(());
        }
        let op = {
            let frame = self.frames.last_mut().unwrap();
            if frame.ip >= frame.func.code.len() {
                return Err(VmError::Runtime("Instruction pointer out of bounds".to_string()));
            }
            let op = frame.func.code[frame.ip].clone();
            frame.ip += 1;
            op
        };
        match op {
            Op::Const(i) => {
                let c = {
                    let frame = self.frames.last().unwrap();
                    frame.func.consts[i as usize].clone()
                };
                self.stack.push(const_to_val(&c));
            }
            Op::MakeFunc(idx) => self.stack.push(Value::Func(idx as usize)),
            Op::Pop => {
                self.stack.pop();
            }
            Op::LoadGlobal(i) => {
                let name = {
                    let frame = self.frames.last().unwrap();
                    frame
                        .global_id_to_name
                        .get(i as usize)
                        .cloned()
                        .unwrap_or_default()
                };
                let v = self.globals.get(&name).cloned().unwrap_or(Value::Null);
                self.stack.push(v);
            }
            Op::StoreGlobal(i) => {
                let v = self.pop()?;
                let name = {
                    let frame = self.frames.last().unwrap();
                    frame
                        .global_id_to_name
                        .get(i as usize)
                        .cloned()
                        .unwrap_or_default()
                };
                if !name.is_empty() {
                    self.globals.insert(name, v.clone());
                }
                self.stack.push(v);
            }
            Op::LoadLocal(i) => {
                let v = {
                    let frame = self.frames.last().unwrap();
                    frame.locals.get(i as usize).cloned().unwrap_or(Value::Null)
                };
                self.stack.push(v);
            }
            Op::StoreLocal(i) => {
                let v = self.pop()?;
                let idx = i as usize;
                let frame = self.frames.last_mut().unwrap();
                if frame.locals.len() <= idx {
                    frame.locals.resize(idx + 1, Value::Null);
                }
                frame.locals[idx] = v.clone();
                self.stack.push(v);
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(bin_num(op, a, b)?);
            }
            Op::Eq | Op::Neq | Op::Lt | Op::LtEq | Op::Gt | Op::GtEq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(cmp(op, a, b)?);
            }
            Op::Not => {
                let v = self.pop()?;
                self.stack.push(Value::Bool(!truthy(&v)));
            }
            Op::Neg => {
                let v = self.pop()?;
                match v {
                    Value::Int(i) => self.stack.push(Value::Int(-i)),
                    Value::Float(x) => self.stack.push(Value::Float(-x)),
                    _ => return Err(VmError::Runtime("Unary '-' on non-number".into())),
                }
            }
            Op::And => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(Value::Bool(truthy(&a) && truthy(&b)));
            }
            Op::Or => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(Value::Bool(truthy(&a) || truthy(&b)));
            }
            Op::JumpIfFalse(target) => {
                let v = self.pop()?;
                if !truthy(&v) {
                    let frame = self.frames.last_mut().unwrap();
                    frame.ip = target as usize;
                }
            }
            Op::Jump(target) => {
                let frame = self.frames.last_mut().unwrap();
                frame.ip = target as usize;
            }
            Op::Call(argc) => {
                let mut args = Vec::with_capacity(argc as usize);
                for _ in 0..argc {
                    args.push(self.pop()?);
                }
                args.reverse();
                let callee = self.pop()?;
                let ret = self.call_value(callee, args)?;
                self.stack.push(ret);
            }
            Op::Return => {
                let ret = self.pop().unwrap_or(Value::Null);
                self.frames.pop();
                if self.frames.is_empty() {
                    self.stack.push(ret);
                } else {
                    self.stack.push(ret);
                }
            }
            Op::GetField(fid) => {
                let obj = self.pop()?;
                let field = {
                    let frame = self.frames.last().unwrap();
                    frame
                        .global_id_to_name
                        .get(fid as usize)
                        .cloned()
                        .unwrap_or_default()
                };
                self.stack.push(get_field(obj, &field)?);
            }
            Op::SetField(fid) => {
                let val = self.pop()?;
                let obj = self.pop()?;
                let field = {
                    let frame = self.frames.last().unwrap();
                    frame
                        .global_id_to_name
                        .get(fid as usize)
                        .cloned()
                        .unwrap_or_default()
                };
                self.stack.push(set_field(obj, &field, val)?);
            }
            Op::MakeArray(n) => {
                let mut elems = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    elems.push(self.pop()?);
                }
                elems.reverse();
                self.stack.push(Value::Array(Rc::new(elems)));
            }
            Op::IndexGet => {
                let idx = self.pop()?;
                let obj = self.pop()?;
                self.stack.push(index_get(obj, idx)?);
            }
            Op::IndexSet => {
                let val = self.pop()?;
                let idx = self.pop()?;
                let obj = self.pop()?;
                self.stack.push(index_set(obj, idx, val)?);
            }
        }
        Ok(())
    }
}

// ---------------- inet module (minimal but working) ----------------

#[derive(Debug, Clone)]
struct InetRoute {
    handler_func_index: Option<usize>,
    static_body: Option<String>,
}

#[derive(Debug)]
struct InetServerState {
    port: u16,
    routes: HashMap<String, InetRoute>,
}

static INET_SERVERS: Mutex<Vec<InetServerState>> = Mutex::new(Vec::new());

fn module_inet_object() -> Value {
    let mut m = HashMap::<String, Value>::new();
    m.insert("__module".to_string(), Value::Bool(true));
    m.insert("server".to_string(), Value::NativeFunc(native_inet_server as NativeFunc));
    m.insert("get".to_string(), Value::NativeFunc(native_inet_get as NativeFunc));
    Value::Object(Rc::new(m))
}

fn native_inet_server(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::Runtime("inet.server expects (port)".into()));
    }
    let port = match args[0] {
        Value::Int(p) => p as u16,
        _ => return Err(VmError::Runtime("port must be int".into())),
    };
    let mut servers = INET_SERVERS
        .lock()
        .map_err(|_| VmError::Runtime("inet server store poisoned".into()))?;
    servers.push(InetServerState {
        port,
        routes: HashMap::new(),
    });
    let id = (servers.len() - 1) as i64;

    let mut obj = HashMap::<String, Value>::new();
    obj.insert("__inet_id".to_string(), Value::Int(id));
    obj.insert("route".to_string(), Value::NativeFunc(native_inet_route as NativeFunc));
    obj.insert(
        "routeText".to_string(),
        Value::NativeFunc(native_inet_route_text as NativeFunc),
    );
    obj.insert("start".to_string(), Value::NativeFunc(native_inet_start as NativeFunc));
    Ok(Value::Object(Rc::new(obj)))
}

// ---------------- gui module (desktop) ----------------

fn module_gui_object() -> Value {
    let mut m = HashMap::<String, Value>::new();
    m.insert("__module".to_string(), Value::Bool(true));
    m.insert("run".to_string(), Value::NativeFunc(native_gui_run as NativeFunc));
    Value::Object(Rc::new(m))
}

fn native_gui_run(vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    // gui.run(title, width, height, drawFn)
    if args.len() != 4 {
        return Err(VmError::Runtime(
            "gui.run expects (title, width, height, drawFn)".into(),
        ));
    }
    let title = match &args[0] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::Runtime("title must be string".into())),
    };
    let w = match args[1] {
        Value::Int(i) => i as f32,
        _ => return Err(VmError::Runtime("width must be int".into())),
    };
    let h = match args[2] {
        Value::Int(i) => i as f32,
        _ => return Err(VmError::Runtime("height must be int".into())),
    };
    let draw_idx = match args[3] {
        Value::Func(i) => i,
        _ => {
            return Err(VmError::Runtime(
                "drawFn must be a function (define `func draw(ui){...}` and pass `draw`)".into(),
            ));
        }
    };

    let globals = vm.globals.clone();
    let functions = vm.functions.clone();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([w, h])
            .with_title(title),
        ..Default::default()
    };

    let app = DesktopApp::new(globals, functions, draw_idx);
    eframe::run_native("CHMER", native_options, Box::new(|_cc| Ok(Box::new(app))))
        .map_err(|e| VmError::Runtime(format!("gui.run failed: {e}")))?;
    Ok(Value::Null)
}

struct DesktopApp {
    globals: HashMap<String, Value>,
    functions: Vec<Function>,
    draw_idx: usize,

    // chess interaction state
    board_id: Option<i64>,
    selected: Option<usize>,
    last_move: Option<String>,
}

impl DesktopApp {
    fn new(globals: HashMap<String, Value>, functions: Vec<Function>, draw_idx: usize) -> Self {
        Self {
            globals,
            functions,
            draw_idx,
            board_id: None,
            selected: None,
            last_move: None,
        }
    }

    fn ui_value(&self) -> Value {
        // Expose a minimal UI object to CHMER drawFn.
        let mut m = HashMap::<String, Value>::new();
        m.insert("__module".to_string(), Value::Bool(false));
        m.insert("text".to_string(), Value::NativeFunc(gui_ui_text as NativeFunc));
        m.insert(
            "chessboard".to_string(),
            Value::NativeFunc(gui_ui_chessboard as NativeFunc),
        );
        Value::Object(Rc::new(m))
    }
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Run CHMER draw function each frame.
        let mut child = Vm::new();
        child.globals = self.globals.clone();
        child.functions = self.functions.clone();

        // Inject a shared GUI context into globals so native UI funcs can draw.
        child.globals.insert(
            "__gui_ctx".to_string(),
            Value::Object(Rc::new(HashMap::new())),
        );
        GUI_CTX.with(|g| *g.borrow_mut() = Some(ctx.clone()));

        let ui_obj = self.ui_value();
        let _ = child.call_value(Value::Func(self.draw_idx), vec![ui_obj]);

        // Extract board interaction state (kept in Rust; CHMER can call ui.chessboard(board))
        let _ = ctx;
    }
}

thread_local! {
    static GUI_CTX: std::cell::RefCell<Option<egui::Context>> = const { std::cell::RefCell::new(None) };
}

fn gui_ui_text(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    // ui.text("hello")
    if args.len() != 2 {
        return Err(VmError::Runtime("ui.text expects (ui, text)".into()));
    }
    let text = match &args[1] {
        Value::Str(s) => s.as_str(),
        _ => return Err(VmError::Runtime("text must be string".into())),
    };
    GUI_CTX.with(|g| {
        if let Some(ctx) = g.borrow().clone() {
            egui::CentralPanel::default().show(&ctx, |ui| {
                ui.label(text);
            });
        }
    });
    Ok(Value::Null)
}

fn gui_ui_chessboard(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    // ui.chessboard(board) -> lastMoveStr|null
    if args.len() != 2 {
        return Err(VmError::Runtime("ui.chessboard expects (ui, board)".into()));
    }
    let board_id = match &args[1] {
        Value::Object(o) => match o.get("__board_id") {
            Some(Value::Int(i)) => *i,
            _ => return Err(VmError::Runtime("Invalid board object".into())),
        },
        _ => return Err(VmError::Runtime("Invalid board object".into())),
    };

    let mut last_move: Option<String> = None;
    let mut legal_to: Vec<usize> = Vec::new();

    GUI_CTX.with(|g| {
        if let Some(ctx) = g.borrow().clone() {
            egui::CentralPanel::default().show(&ctx, |ui| {
                ui.heading("CHMER Desktop Chess");
                ui.add_space(8.0);

                let sq_size = 56.0;
                let (rect, _resp) =
                    ui.allocate_exact_size(egui::vec2(sq_size * 8.0, sq_size * 8.0), egui::Sense::click());

                let painter = ui.painter_at(rect);

                // precompute legal moves for selection highlight
                let legal = chmer5_chess::board_legalmoves(board_id).unwrap_or_default();
                let sel = SELECTED_SQ.with(|s| *s.borrow());
                if let Some(from) = sel {
                    legal_to = legal
                        .iter()
                        .filter_map(|m| {
                            if m.len() >= 4 && &m[0..2] == sq_to_alg(from) {
                                Some(m[2..4].to_string())
                            } else {
                                None
                            }
                        })
                        .filter_map(|a| chmer5_chess_sq_from_alg(&a))
                        .collect();
                }

                // click -> square
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    if rect.contains(pos) && ui.input(|i| i.pointer.any_click()) {
                        let rel = pos - rect.min;
                        let file = (rel.x / sq_size).floor() as i32;
                        let rank_from_top = (rel.y / sq_size).floor() as i32;
                        if (0..8).contains(&file) && (0..8).contains(&rank_from_top) {
                            let rank = 7 - rank_from_top; // convert to 0..7 bottom
                            let sq = (rank * 8 + file) as usize;

                            // Simple two-click move: first select, second move.
                            SELECTED_SQ.with(|s| {
                                let mut sel = s.borrow_mut();
                                if let Some(from) = *sel {
                                    if from != sq {
                                        let mv = format!("{}{}", sq_to_alg(from), sq_to_alg(sq));
                                        match chmer5_chess::board_apply_uci(board_id, &mv) {
                                            Ok(()) => last_move = Some(mv),
                                            Err(_) => {}
                                        }
                                    }
                                    *sel = None;
                                } else {
                                    *sel = Some(sq);
                                }
                            });
                        }
                    }
                }

                // draw squares + pieces
                for r in 0..8 {
                    for f in 0..8 {
                        let sq = r * 8 + f;
                        let x = rect.min.x + f as f32 * sq_size;
                        let y = rect.min.y + (7 - r) as f32 * sq_size;
                        let sq_rect = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(sq_size, sq_size),
                        );
                        let dark = ((r + f) & 1) == 1;
                        let col = if dark {
                            egui::Color32::from_rgb(181, 136, 99)
                        } else {
                            egui::Color32::from_rgb(240, 217, 181)
                        };
                        painter.rect_filled(sq_rect, 0.0, col);

                        let sel = SELECTED_SQ.with(|s| *s.borrow());
                        if sel == Some(sq) {
                            painter.rect_stroke(
                                sq_rect.shrink(2.0),
                                0.0,
                                egui::Stroke::new(2.0, egui::Color32::YELLOW),
                            );
                        }
                        if legal_to.contains(&sq) {
                            painter.rect_stroke(
                                sq_rect.shrink(4.0),
                                0.0,
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 200, 120)),
                            );
                        }

                        if let Ok(p) = chmer5_chess::board_piece_at(board_id, sq) {
                            if p != b'.' {
                                let glyph = piece_to_unicode(p);
                                painter.text(
                                    sq_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    glyph,
                                    egui::FontId::proportional(32.0),
                                    egui::Color32::BLACK,
                                );
                            }
                        }
                    }
                }

                ui.add_space(8.0);
                if let Some(m) = &last_move {
                    ui.label(format!("Last move: {m}"));
                } else {
                    ui.label("Click a piece, then click destination.");
                }
            });
        }
    });

    Ok(match last_move {
        Some(m) => Value::Str(Rc::new(m)),
        None => Value::Null,
    })
}

fn chmer5_chess_sq_from_alg(a: &str) -> Option<usize> {
    let bytes = a.as_bytes();
    if bytes.len() != 2 {
        return None;
    }
    let f = bytes[0];
    let r = bytes[1];
    if !(b'a'..=b'h').contains(&f) || !(b'1'..=b'8').contains(&r) {
        return None;
    }
    let file = (f - b'a') as usize;
    let rank = (r - b'1') as usize;
    Some(rank * 8 + file)
}

thread_local! {
    static SELECTED_SQ: std::cell::RefCell<Option<usize>> = const { std::cell::RefCell::new(None) };
}

fn piece_to_unicode(p: u8) -> &'static str {
    match p {
        b'P' => "♙",
        b'N' => "♘",
        b'B' => "♗",
        b'R' => "♖",
        b'Q' => "♕",
        b'K' => "♔",
        b'p' => "♟",
        b'n' => "♞",
        b'b' => "♝",
        b'r' => "♜",
        b'q' => "♛",
        b'k' => "♚",
        _ => "",
    }
}

fn sq_to_alg(sq: usize) -> String {
    let file = (sq % 8) as u8;
    let rank = (sq / 8) as u8;
    let f = (b'a' + file) as char;
    let r = (b'1' + rank) as char;
    format!("{f}{r}")
}

fn inet_id_from_obj(v: &Value) -> Result<i64, VmError> {
    match v {
        Value::Object(o) => match o.get("__inet_id") {
            Some(Value::Int(i)) => Ok(*i),
            _ => Err(VmError::Runtime("Invalid server object".into())),
        },
        _ => Err(VmError::Runtime("Invalid server object".into())),
    }
}

fn native_inet_route(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::Runtime("server.route expects (server, path, handler)".into()));
    }
    let id = inet_id_from_obj(&args[0])?;
    let path = match &args[1] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::Runtime("path must be string".into())),
    };
    let handler_func_index = match &args[2] {
        Value::Func(i) => *i,
        _ => {
            return Err(VmError::Runtime(
                "handler must be a function (define `func handler(req){...}` and pass `handler`)"
                    .into(),
            ));
        }
    };

    let mut servers = INET_SERVERS
        .lock()
        .map_err(|_| VmError::Runtime("inet server store poisoned".into()))?;
    let s = servers
        .get_mut(id as usize)
        .ok_or_else(|| VmError::Runtime("inet server id out of range".into()))?;
    s.routes.insert(
        path,
        InetRoute {
            handler_func_index: Some(handler_func_index),
            static_body: None,
        },
    );
    Ok(Value::Null)
}

fn native_inet_route_text(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 3 {
        return Err(VmError::Runtime(
            "server.routeText expects (server, path, text)".into(),
        ));
    }
    let id = inet_id_from_obj(&args[0])?;
    let path = match &args[1] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::Runtime("path must be string".into())),
    };
    let body = match &args[2] {
        Value::Str(s) => s.as_str().to_string(),
        _ => return Err(VmError::Runtime("text must be string".into())),
    };

    let mut servers = INET_SERVERS
        .lock()
        .map_err(|_| VmError::Runtime("inet server store poisoned".into()))?;
    let s = servers
        .get_mut(id as usize)
        .ok_or_else(|| VmError::Runtime("inet server id out of range".into()))?;
    s.routes.insert(
        path,
        InetRoute {
            handler_func_index: None,
            static_body: Some(body),
        },
    );
    Ok(Value::Null)
}

fn native_inet_start(vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::Runtime("server.start expects (server)".into()));
    }
    let id = inet_id_from_obj(&args[0])?;
    let (port, routes) = {
        let servers = INET_SERVERS
            .lock()
            .map_err(|_| VmError::Runtime("inet server store poisoned".into()))?;
        let s = servers
            .get(id as usize)
            .ok_or_else(|| VmError::Runtime("inet server id out of range".into()))?;
        (s.port, s.routes.clone())
    };

    let addr = format!("0.0.0.0:{port}");
    let server = Server::http(&addr).map_err(|e| VmError::Runtime(format!("inet server: {e}")))?;
    println!("CHMER inet server listening on http://{addr}");

    for req in server.incoming_requests() {
        let path = req.url().to_string();
        let method = req.method().clone();
        let body = String::new();

        let route = routes.get(&path).cloned();
        let resp_body = if let Some(route) = route {
            if let Some(body) = route.static_body {
                body
            } else if let Some(hidx) = route.handler_func_index {
            let req_obj = Value::Object(Rc::new(HashMap::from([
                ("path".to_string(), Value::Str(Rc::new(path.clone()))),
                ("method".to_string(), Value::Str(Rc::new(match method {
                    Method::Get => "GET",
                    Method::Post => "POST",
                    Method::Put => "PUT",
                    Method::Delete => "DELETE",
                    _ => "OTHER",
                }.to_string()))),
                ("body".to_string(), Value::Str(Rc::new(body))),
            ])));

            // Run handler in an isolated VM instance to avoid re-entrancy issues.
            let mut child = Vm::new();
            child.globals = vm.globals.clone();
            child.functions = vm.functions.clone();
            match child.call_value(Value::Func(hidx), vec![req_obj]) {
                Ok(v) => v.to_string(),
                Err(e) => format!("{e}"),
            }
            } else {
                "500".to_string()
            }
        } else {
            "404".to_string()
        };

        let mut response = Response::from_string(resp_body);
        response.add_header(
            Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                .map_err(|e| VmError::Runtime(format!("header error: {:?}", e)))?,
        );
        let _ = req.respond(response);
    }

    Ok(Value::Null)
}

fn native_inet_get(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::Runtime("inet.get expects (url)".into()));
    }
    let url = match &args[0] {
        Value::Str(s) => s.as_str(),
        _ => return Err(VmError::Runtime("url must be string".into())),
    };
    let resp = ureq::get(url)
        .call()
        .map_err(|e| VmError::Runtime(format!("inet.get failed: {e}")))?;
    let text = resp
        .into_string()
        .map_err(|e| VmError::Runtime(format!("inet.get read failed: {e}")))?;
    Ok(Value::Str(Rc::new(text)))
}

fn const_to_val(c: &ConstVal) -> Value {
    match c {
        ConstVal::Null => Value::Null,
        ConstVal::Bool(b) => Value::Bool(*b),
        ConstVal::Int(i) => Value::Int(*i),
        ConstVal::Float(x) => Value::Float(*x),
        ConstVal::Str(s) => Value::Str(Rc::new(s.clone())),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(x) => *x != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(_) => true,
        Value::NativeFunc(_) => true,
        Value::BoundMethod { .. } => true,
        Value::Func(_) => true,
    }
}

fn bin_num(op: Op, a: Value, b: Value) -> Result<Value, VmError> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(match op {
            Op::Add => Value::Int(x + y),
            Op::Sub => Value::Int(x - y),
            Op::Mul => Value::Int(x * y),
            Op::Div => Value::Int(x / y),
            Op::Mod => Value::Int(x % y),
            _ => unreachable!(),
        }),
        (Value::Float(x), Value::Float(y)) => Ok(match op {
            Op::Add => Value::Float(x + y),
            Op::Sub => Value::Float(x - y),
            Op::Mul => Value::Float(x * y),
            Op::Div => Value::Float(x / y),
            Op::Mod => Value::Float(x % y),
            _ => unreachable!(),
        }),
        (Value::Int(x), Value::Float(y)) => bin_num(op, Value::Float(x as f64), Value::Float(y)),
        (Value::Float(x), Value::Int(y)) => bin_num(op, Value::Float(x), Value::Float(y as f64)),
        (Value::Str(x), y) if matches!(op, Op::Add) => Ok(Value::Str(Rc::new(format!("{x}{y}")))),
        (x, Value::Str(y)) if matches!(op, Op::Add) => Ok(Value::Str(Rc::new(format!("{x}{y}")))),
        _ => Err(VmError::Runtime("Numeric operation on invalid types".into())),
    }
}

fn cmp(op: Op, a: Value, b: Value) -> Result<Value, VmError> {
    let res = match op {
        Op::Eq => eq_val(&a, &b),
        Op::Neq => !eq_val(&a, &b),
        Op::Lt | Op::LtEq | Op::Gt | Op::GtEq => {
            let (x, y) = match (a, b) {
                (Value::Int(x), Value::Int(y)) => (x as f64, y as f64),
                (Value::Float(x), Value::Float(y)) => (x, y),
                (Value::Int(x), Value::Float(y)) => (x as f64, y),
                (Value::Float(x), Value::Int(y)) => (x, y as f64),
                _ => return Err(VmError::Runtime("Comparison on invalid types".into())),
            };
            match op {
                Op::Lt => x < y,
                Op::LtEq => x <= y,
                Op::Gt => x > y,
                Op::GtEq => x >= y,
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    };
    Ok(Value::Bool(res))
}

fn eq_val(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        _ => false,
    }
}

fn get_field(obj: Value, field: &str) -> Result<Value, VmError> {
    match obj {
        Value::Object(map) => {
            let v = map.get(field).cloned().unwrap_or(Value::Null);
            let is_module = matches!(map.get("__module"), Some(Value::Bool(true)));
            match v {
                Value::NativeFunc(f) if !is_module => Ok(Value::BoundMethod { recv: map, func: f }),
                _ => Ok(v),
            }
        }
        _ => Err(VmError::Runtime("GetField on non-object".into())),
    }
}

fn set_field(obj: Value, field: &str, val: Value) -> Result<Value, VmError> {
    match obj {
        Value::Object(map) => {
            let mut m = (*map).clone();
            m.insert(field.to_string(), val.clone());
            Ok(Value::Object(Rc::new(m)))
        }
        _ => Err(VmError::Runtime("SetField on non-object".into())),
    }
}

fn index_get(obj: Value, idx: Value) -> Result<Value, VmError> {
    match (obj, idx) {
        (Value::Array(a), Value::Int(i)) => Ok(a.get(i as usize).cloned().unwrap_or(Value::Null)),
        _ => Err(VmError::Runtime("Indexing not supported".into())),
    }
}

fn index_set(obj: Value, idx: Value, val: Value) -> Result<Value, VmError> {
    match (obj, idx) {
        (Value::Array(a), Value::Int(i)) => {
            let mut v = (*a).clone();
            let ui = i as usize;
            if ui >= v.len() {
                return Err(VmError::Runtime("Index out of bounds".into()));
            }
            v[ui] = val.clone();
            Ok(Value::Array(Rc::new(v)))
        }
        _ => Err(VmError::Runtime("IndexSet not supported".into())),
    }
}

fn native_print(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.is_empty() {
        println!();
        return Ok(Value::Null);
    }
    let mut first = true;
    for a in args {
        if !first {
            print!(" ");
        }
        first = false;
        print!("{a}");
    }
    println!();
    Ok(Value::Null)
}

fn module_chess_object() -> Value {
    let mut m = HashMap::<String, Value>::new();
    m.insert("__module".to_string(), Value::Bool(true));
    m.insert("board".to_string(), Value::NativeFunc(native_chess_board as NativeFunc));
    m.insert(
        "renderBoardHtml".to_string(),
        Value::NativeFunc(native_chess_render_board_html as NativeFunc),
    );
    Value::Object(Rc::new(m))
}

fn native_chess_board(_vm: &mut Vm, _args: Vec<Value>) -> Result<Value, VmError> {
    let id = chmer5_chess::board_new();
    let mut obj = HashMap::<String, Value>::new();
    obj.insert("__board_id".to_string(), Value::Int(id));
    obj.insert("load".to_string(), Value::NativeFunc(native_board_load as NativeFunc));
    obj.insert(
        "legalmoves".to_string(),
        Value::NativeFunc(native_board_legalmoves as NativeFunc),
    );
    Ok(Value::Object(Rc::new(obj)))
}

fn native_board_load(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::Runtime("board.load expects (board, fen)".into()));
    }
    let board_id = match &args[0] {
        Value::Object(o) => match o.get("__board_id") {
            Some(Value::Int(i)) => *i,
            _ => return Err(VmError::Runtime("Invalid board object".into())),
        },
        _ => return Err(VmError::Runtime("Invalid board object".into())),
    };
    let fen = match &args[1] {
        Value::Str(s) => s.as_str(),
        _ => return Err(VmError::Runtime("fen must be string".into())),
    };
    chmer5_chess::board_load(board_id, fen)
        .map_err(|e| VmError::Runtime(format!("FEN error: {e}")))?;
    Ok(Value::Null)
}

fn native_board_legalmoves(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::Runtime("board.legalmoves expects (board)".into()));
    }
    let board_id = match &args[0] {
        Value::Object(o) => match o.get("__board_id") {
            Some(Value::Int(i)) => *i,
            _ => return Err(VmError::Runtime("Invalid board object".into())),
        },
        _ => return Err(VmError::Runtime("Invalid board object".into())),
    };
    let list = chmer5_chess::board_legalmoves(board_id)
        .map_err(|e| VmError::Runtime(format!("MoveGen error: {e}")))?;
    Ok(Value::Array(Rc::new(
        list.into_iter().map(|s| Value::Str(Rc::new(s))).collect(),
    )))
}

fn native_chess_board_id(_vm: &mut Vm, _args: Vec<Value>) -> Result<Value, VmError> {
    Ok(Value::Int(chmer5_chess::board_new()))
}

fn native_chess_board_load(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 2 {
        return Err(VmError::Runtime(
            "chess_board_load expects (boardId, fen)".into(),
        ));
    }
    let id = match args[0] {
        Value::Int(i) => i,
        _ => return Err(VmError::Runtime("boardId must be int".into())),
    };
    let fen = match &args[1] {
        Value::Str(s) => s.as_str(),
        _ => return Err(VmError::Runtime("fen must be string".into())),
    };
    chmer5_chess::board_load(id, fen).map_err(|e| VmError::Runtime(format!("FEN error: {e}")))?;
    Ok(Value::Null)
}

fn native_chess_board_legalmoves(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::Runtime(
            "chess_board_legalmoves expects (boardId)".into(),
        ));
    }
    let id = match args[0] {
        Value::Int(i) => i,
        _ => return Err(VmError::Runtime("boardId must be int".into())),
    };
    let list = chmer5_chess::board_legalmoves(id)
        .map_err(|e| VmError::Runtime(format!("MoveGen error: {e}")))?;
    Ok(Value::Array(Rc::new(
        list.into_iter().map(|s| Value::Str(Rc::new(s))).collect(),
    )))
}

fn native_chess_render_board_html_id(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    if args.len() != 1 {
        return Err(VmError::Runtime(
            "chess_render_board_html expects (boardId)".into(),
        ));
    }
    let id = match args[0] {
        Value::Int(i) => i,
        _ => return Err(VmError::Runtime("boardId must be int".into())),
    };
    let moves = chmer5_chess::board_legalmoves(id)
        .map_err(|e| VmError::Runtime(format!("MoveGen error: {e}")))?;
    let html = format!(
        "<html><body><h2>CHMER Chess</h2><div>Moves: {}</div></body></html>",
        serde_json::to_string(&moves).unwrap_or("[]".to_string())
    );
    Ok(Value::Str(Rc::new(html)))
}

fn native_chess_render_board_html(_vm: &mut Vm, args: Vec<Value>) -> Result<Value, VmError> {
    // chess.renderBoardHtml(board)
    if args.len() != 1 {
        return Err(VmError::Runtime("chess.renderBoardHtml expects (board)".into()));
    }
    let board_id = match &args[0] {
        Value::Object(o) => match o.get("__board_id") {
            Some(Value::Int(i)) => *i,
            _ => return Err(VmError::Runtime("Invalid board object".into())),
        },
        _ => return Err(VmError::Runtime("Invalid board object".into())),
    };

    // Minimal HTML: renders an 8x8 grid and a small move list.
    let moves = chmer5_chess::board_legalmoves(board_id)
        .map_err(|e| VmError::Runtime(format!("MoveGen error: {e}")))?;
    let html = format!(
        r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8"/>
  <title>CHMER Chess</title>
  <style>
    body {{ font-family: system-ui, sans-serif; padding: 20px; }}
    .row {{ display:flex; }}
    .sq {{ width:60px; height:60px; display:flex; align-items:center; justify-content:center; font-size:34px; }}
    .w {{ background:#f0d9b5; }}
    .b {{ background:#b58863; }}
    .wrap {{ display:flex; gap:24px; }}
    .moves {{ max-width: 320px; }}
    button {{ margin: 4px 4px 0 0; }}
  </style>
</head>
<body>
  <h2>CHMER Chess (prototype)</h2>
  <div class="wrap">
    <div id="board"></div>
    <div class="moves">
      <div><b>Legal moves (stub)</b></div>
      <div id="moves"></div>
    </div>
  </div>
  <script>
    const moves = {moves_json};
    document.getElementById("moves").innerHTML = moves.map(m => `<button onclick="alert('move '+m)">${{m}}</button>`).join("");

    // Board rendering is placeholder until full piece access is exposed from runtime.
    let board = "";
    for (let r=7;r>=0;r--) {{
      board += '<div class="row">';
      for (let f=0;f<8;f++) {{
        const isBlack = ((r+f)&1)===1;
        board += `<div class="sq ${{isBlack?'b':'w'}}"></div>`;
      }}
      board += '</div>';
    }}
    document.getElementById("board").innerHTML = board;
  </script>
</body>
</html>"#,
        moves_json = serde_json::to_string(&moves).unwrap_or("[]".to_string())
    );
    Ok(Value::Str(Rc::new(html)))
}
