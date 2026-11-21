use crate::ast::*;
use crate::check::CheckedProgram;
use crate::diag::Diagnostic;
use crate::span::Span;
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug)]
pub enum Value {
    I32(i32),
    I64(i64),
    F64(f64),
    Bool(bool),
    Str(String),
    Unit,
    Vec(Vec<Value>),
    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),
    Range(i32, i32),
    Map(HashMap<String, Value>),
    Struct(String, HashMap<String, Value>),
    Enum(String, String, Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I32(n) => write!(f, "{n}"),
            Value::I64(n) => write!(f, "{n}"),
            Value::F64(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Unit => write!(f, "()"),
            Value::Vec(xs) => {
                write!(f, "[")?;
                for (i, x) in xs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{x}")?;
                }
                write!(f, "]")
            }
            Value::Option(None) => write!(f, "None"),
            Value::Option(Some(v)) => write!(f, "Some({v})"),
            Value::Result(Ok(v)) => write!(f, "Ok({v})"),
            Value::Result(Err(e)) => write!(f, "Err({e})"),
            Value::Range(a, b) => write!(f, "{a}..{b}"),
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Value::Struct(name, fields) => {
                write!(f, "{name} {{ ")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, " }}")
            }
            Value::Enum(en, var, args) => {
                write!(f, "{en}::{var}")?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{a}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
enum Flow {
    Value(Value),
    Return(Value),
    Break,
    Continue,
}

pub struct Vm {
    pub output: String,
    scopes: Vec<HashMap<String, Slot>>,
    functions: HashMap<String, FnDef>,
    methods: HashMap<String, HashMap<String, FnDef>>,
    argv: Vec<String>,
    threads: HashMap<i32, std::thread::JoinHandle<Result<String, Diagnostic>>>,
    next_tid: i32,
}

struct Slot {
    value: Value,
}

impl Vm {
    pub fn new(checked: &CheckedProgram) -> Self {
        Self {
            output: String::new(),
            scopes: vec![HashMap::new()],
            functions: checked.functions.clone(),
            methods: checked.methods.clone(),
            argv: Vec::new(),
            threads: HashMap::new(),
            next_tid: 1,
        }
    }

    pub fn with_args(mut self, argv: Vec<String>) -> Self {
        self.argv = argv;
        self
    }

    pub fn run_named(&mut self, name: &str) -> Result<Value, Diagnostic> {
        let f = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| Diagnostic::new(Span::dummy(), format!("找不到 `{name}`")))?;
        match self.call_fn(&f, vec![])? {
            Flow::Value(v) | Flow::Return(v) => Ok(v),
            Flow::Break => Err(Diagnostic::new(Span::dummy(), "在函式頂層 `break`")),
            Flow::Continue => Err(Diagnostic::new(Span::dummy(), "在函式頂層 `continue`")),
        }
    }

    pub fn run_main(&mut self) -> Result<Value, Diagnostic> {
        let main = self
            .functions
            .get("main")
            .cloned()
            .ok_or_else(|| Diagnostic::new(Span::dummy(), "找不到 `main`"))?;
        match self.call_fn(&main, vec![])? {
            Flow::Value(v) | Flow::Return(v) => Ok(v),
            Flow::Break => Err(Diagnostic::new(Span::dummy(), "在函式頂層 `break`")),
            Flow::Continue => Err(Diagnostic::new(Span::dummy(), "在函式頂層 `continue`")),
        }
    }

    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: String, value: Value) {
        self.scopes.last_mut().unwrap().insert(name, Slot { value });
    }

    fn get(&self, name: &str, span: Span) -> Result<Value, Diagnostic> {
        for s in self.scopes.iter().rev() {
            if let Some(slot) = s.get(name) {
                return Ok(slot.value.clone());
            }
        }
        Err(Diagnostic::new(span, format!("執行期找不到 `{name}`")))
    }

    fn get_mut(&mut self, name: &str, span: Span) -> Result<&mut Value, Diagnostic> {
        for s in self.scopes.iter_mut().rev() {
            if s.contains_key(name) {
                return Ok(&mut s.get_mut(name).unwrap().value);
            }
        }
        Err(Diagnostic::new(span, format!("執行期找不到 `{name}`")))
    }

    fn call_fn(&mut self, f: &FnDef, args: Vec<Value>) -> Result<Flow, Diagnostic> {
        self.push();
        for (p, v) in f.params.iter().zip(args.into_iter()) {
            self.define(p.name.clone(), v);
        }
        let flow = self.eval(&f.body)?;
        self.pop();
        Ok(match flow {
            Flow::Return(v) => Flow::Value(v),
            other => other,
        })
    }

    fn eval(&mut self, expr: &Expr) -> Result<Flow, Diagnostic> {
        match expr {
            Expr::Int { value, ty_i64, .. } => {
                if *ty_i64 {
                    Ok(Flow::Value(Value::I64(*value)))
                } else {
                    Ok(Flow::Value(Value::I32(*value as i32)))
                }
            }
            Expr::Float { value, .. } => Ok(Flow::Value(Value::F64(*value))),
            Expr::Bool { value, .. } => Ok(Flow::Value(Value::Bool(*value))),
            Expr::Str { value, .. } => Ok(Flow::Value(Value::Str(value.clone()))),
            Expr::Unit { .. } => Ok(Flow::Value(Value::Unit)),
            Expr::Var { name, span } => Ok(Flow::Value(self.get(name, *span)?)),
            Expr::Unary { op, expr, span } => {
                let v = self.val(expr)?;
                Ok(Flow::Value(self.unary(*op, v, *span)?))
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let l = self.val(left)?;
                let r = self.val(right)?;
                Ok(Flow::Value(self.binary(*op, l, r, *span)?))
            }
            Expr::Assign { place, value, span } => {
                let v = self.val(value)?;
                self.assign(place, v, *span)?;
                Ok(Flow::Value(Value::Unit))
            }
            Expr::Call { callee, args, span } => self.eval_call(callee, args, *span),
            Expr::Field { base, name, span } => {
                let v = self.val(base)?;
                Ok(Flow::Value(self.field(&v, name, *span)?))
            }
            Expr::Index { base, index, span } => {
                let b = self.val(base)?;
                let i = self.val(index)?;
                Ok(Flow::Value(self.index(&b, &i, *span)?))
            }
            Expr::StructLit { name, fields, .. } => {
                let mut map = HashMap::new();
                for (k, e) in fields {
                    map.insert(k.clone(), self.val(e)?);
                }
                Ok(Flow::Value(Value::Struct(name.clone(), map)))
            }
            Expr::EnumLit {
                enum_name,
                variant,
                args,
                ..
            } => {
                let mut vs = Vec::new();
                for a in args {
                    vs.push(self.val(a)?);
                }
                match variant.as_str() {
                    "Some" => Ok(Flow::Value(Value::Option(Some(Box::new(
                        vs.into_iter().next().unwrap(),
                    ))))),
                    "None" => Ok(Flow::Value(Value::Option(None))),
                    "Ok" => Ok(Flow::Value(Value::Result(Ok(Box::new(
                        vs.into_iter().next().unwrap(),
                    ))))),
                    "Err" => Ok(Flow::Value(Value::Result(Err(Box::new(
                        vs.into_iter().next().unwrap(),
                    ))))),
                    _ => Ok(Flow::Value(Value::Enum(
                        enum_name.clone().unwrap_or_default(),
                        variant.clone(),
                        vs,
                    ))),
                }
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => match self.val(cond)? {
                Value::Bool(true) => self.eval(then_branch),
                Value::Bool(false) => {
                    if let Some(e) = else_branch {
                        self.eval(e)
                    } else {
                        Ok(Flow::Value(Value::Unit))
                    }
                }
                _ => Err(Diagnostic::new(*span, "`if` 條件必須是 bool")),
            },
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => {
                let v = self.val(scrutinee)?;
                for arm in arms {
                    self.push();
                    if self.match_pat(&arm.pat, &v)? {
                        let r = self.eval(&arm.body)?;
                        self.pop();
                        return Ok(r);
                    }
                    self.pop();
                }
                Err(Diagnostic::new(*span, "`match` 沒有對應的分支"))
            }
            Expr::Block { stmts, .. } => self.eval_block(stmts),
            Expr::Try { expr, span } => match self.val(expr)? {
                Value::Result(Ok(v)) => Ok(Flow::Value(*v)),
                Value::Result(Err(e)) => Ok(Flow::Return(Value::Result(Err(e)))),
                Value::Option(Some(v)) => Ok(Flow::Value(*v)),
                Value::Option(None) => Ok(Flow::Return(Value::Option(None))),
                _ => Err(Diagnostic::new(*span, "`?` 只能用在 Result 或 Option")),
            },
            Expr::Range { start, end, .. } => {
                let a = match self.val(start)? {
                    Value::I32(n) => n,
                    _ => return Err(Diagnostic::new(start.span(), "範圍起點必須是 i32")),
                };
                let b = match self.val(end)? {
                    Value::I32(n) => n,
                    _ => return Err(Diagnostic::new(end.span(), "範圍終點必須是 i32")),
                };
                Ok(Flow::Value(Value::Range(a, b)))
            }
        }
    }

    fn val(&mut self, expr: &Expr) -> Result<Value, Diagnostic> {
        match self.eval(expr)? {
            Flow::Value(v) => Ok(v),
            Flow::Return(v) => Err(Diagnostic::new(
                expr.span(),
                format!("內部 early-return: {v}"),
            )),
            Flow::Break => Err(Diagnostic::new(expr.span(), "`break` 出現在無效位置")),
            Flow::Continue => Err(Diagnostic::new(expr.span(), "`continue` 出現在無效位置")),
        }
    }

    fn eval_block(&mut self, stmts: &[Stmt]) -> Result<Flow, Diagnostic> {
        self.push();
        let mut last = Value::Unit;
        for stmt in stmts {
            match self.eval_stmt(stmt)? {
                Flow::Return(v) => {
                    self.pop();
                    return Ok(Flow::Return(v));
                }
                Flow::Break => {
                    self.pop();
                    return Ok(Flow::Break);
                }
                Flow::Continue => {
                    self.pop();
                    return Ok(Flow::Continue);
                }
                Flow::Value(v) => last = v,
            }
        }
        self.pop();
        Ok(Flow::Value(last))
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> Result<Flow, Diagnostic> {
        match stmt {
            Stmt::Let { name, init, .. } => {
                let v = match self.eval(init)? {
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Break => return Ok(Flow::Break),
                    Flow::Continue => return Ok(Flow::Continue),
                    Flow::Value(v) => v,
                };
                self.define(name.clone(), v);
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::Return { expr, .. } => {
                let v = if let Some(e) = expr {
                    match self.eval(e)? {
                        Flow::Return(v) | Flow::Value(v) => v,
                        Flow::Break => {
                            return Err(Diagnostic::new(e.span(), "不能在 return 裡 break"))
                        }
                        Flow::Continue => {
                            return Err(Diagnostic::new(e.span(), "不能在 return 裡 continue"))
                        }
                    }
                } else {
                    Value::Unit
                };
                Ok(Flow::Return(v))
            }
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::Continue),
            Stmt::Loop { body, .. } => loop {
                match self.eval(body)? {
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Break => return Ok(Flow::Value(Value::Unit)),
                    Flow::Continue | Flow::Value(_) => {}
                }
            },
            Stmt::While { cond, body, span } => loop {
                match self.val(cond)? {
                    Value::Bool(true) => match self.eval(body)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Break => return Ok(Flow::Value(Value::Unit)),
                        Flow::Continue | Flow::Value(_) => {}
                    },
                    Value::Bool(false) => return Ok(Flow::Value(Value::Unit)),
                    _ => return Err(Diagnostic::new(*span, "`while` 條件必須是 bool")),
                }
            },
            Stmt::For {
                name,
                iter,
                body,
                span,
            } => {
                let it = self.val(iter)?;
                let items: Vec<Value> = match it {
                    Value::Vec(xs) => xs,
                    Value::Range(a, b) => (a..b).map(Value::I32).collect(),
                    _ => {
                        return Err(Diagnostic::new(*span, "`for` 只能遍歷 Vec 或範圍"));
                    }
                };
                for item in items {
                    self.push();
                    self.define(name.clone(), item);
                    match self.eval(body)? {
                        Flow::Return(v) => {
                            self.pop();
                            return Ok(Flow::Return(v));
                        }
                        Flow::Break => {
                            self.pop();
                            return Ok(Flow::Value(Value::Unit));
                        }
                        Flow::Continue | Flow::Value(_) => {}
                    }
                    self.pop();
                }
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::Expr(e) => self.eval(e),
        }
    }

    fn http_serve(&mut self, args: &[Expr], span: Span) -> Result<Flow, Diagnostic> {
        let port = match self.val(&args[0])? {
            Value::I32(n) => n,
            Value::I64(n) => n as i32,
            _ => return Err(Diagnostic::new(span, "`http_serve` 連接埠必須是整數")),
        };
        let handler = match self.val(&args[1])? {
            Value::Str(s) => s,
            _ => return Err(Diagnostic::new(span, "`http_serve` 需要處理函式名稱")),
        };
        if !crate::sandbox::safe_ident(&handler) {
            return Err(Diagnostic::new(span, "`http_serve` 函式名稱不合法"));
        }
        let mut max_req = 0i32;
        if args.len() >= 3 {
            max_req = match self.val(&args[2])? {
                Value::I32(n) => n,
                Value::I64(n) => n as i32,
                _ => return Err(Diagnostic::new(span, "`http_serve` 次數必須是整數")),
            };
        }
        if max_req < 0 {
            max_req = 0;
        }
        if max_req > crate::sandbox::MAX_HTTP_SERVE_REQUESTS {
            max_req = crate::sandbox::MAX_HTTP_SERVE_REQUESTS;
        }
        let f = self
            .functions
            .get(&handler)
            .cloned()
            .ok_or_else(|| Diagnostic::new(span, format!("找不到處理函式 `{handler}`")))?;
        if f.params.len() != 4 {
            return Err(Diagnostic::new(
                span,
                format!("`{handler}` 必須是 fn(method: str, path: str, query: str, body: str) -> str"),
            ));
        }
        let listener = match crate::http_server::bind_local(port) {
            Ok(l) => l,
            Err(e) => {
                return Ok(Flow::Value(Value::Result(Err(Box::new(Value::Str(e))))))
            }
        };
        println!("Russd 網站 http://127.0.0.1:{port}/");
        self.output
            .push_str(&format!("Russd 網站 http://127.0.0.1:{port}/\n"));
        let mut served = 0i32;
        loop {
            if max_req > 0 && served >= max_req {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    match crate::http_server::read_request(&mut stream) {
                        Ok(req) => {
                            if req.method == "HEAD" || req.path == "/favicon.ico" {
                                let _ = crate::http_server::write_response(
                                    &mut stream,
                                    &req.path,
                                    "",
                                );
                            } else {
                                let args = vec![
                                    Value::Str(req.method),
                                    Value::Str(req.path.clone()),
                                    Value::Str(req.query),
                                    Value::Str(req.body),
                                ];
                                match self.call_fn(&f, args) {
                                    Ok(Flow::Value(Value::Str(body)))
                                    | Ok(Flow::Return(Value::Str(body))) => {
                                        let _ = crate::http_server::write_response(
                                            &mut stream, &req.path, &body,
                                        );
                                    }
                                    Ok(_) => {
                                        crate::http_server::write_error(
                                            &mut stream,
                                            "處理函式必須回傳 str",
                                        );
                                    }
                                    Err(d) => {
                                        crate::http_server::write_error(&mut stream, &d.message);
                                    }
                                }
                            }
                            served += 1;
                        }
                        Err(e) => crate::http_server::write_error(&mut stream, &e),
                    }
                }
                Err(e) => {
                    return Ok(Flow::Value(Value::Result(Err(Box::new(Value::Str(
                        e.to_string(),
                    ))))))
                }
            }
        }
        Ok(Flow::Value(Value::Result(Ok(Box::new(Value::Str(format!(
            "served {served}"
        )))))))
    }

    fn eval_call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Result<Flow, Diagnostic> {
        if let Expr::Var { name, .. } = callee {
            if name == "print" {
                let mut parts = Vec::new();
                for a in args {
                    match self.eval(a)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Break => return Ok(Flow::Break),
                        Flow::Continue => return Ok(Flow::Continue),
                        Flow::Value(v) => parts.push(v.to_string()),
                    }
                }
                let line = parts.join(" ");
                self.output.push_str(&line);
                self.output.push('\n');
                println!("{line}");
                return Ok(Flow::Value(Value::Unit));
            }
            if name == "vec" {
                let mut xs = Vec::new();
                for a in args {
                    xs.push(self.val(a)?);
                }
                return Ok(Flow::Value(Value::Vec(xs)));
            }
            if name == "map" {
                return Ok(Flow::Value(Value::Map(HashMap::new())));
            }
            if name == "args" {
                return Ok(Flow::Value(Value::Vec(
                    self.argv.iter().cloned().map(Value::Str).collect(),
                )));
            }
            if name == "read_file" {
                let path = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`read_file` 路徑必須是 str")),
                };
                let path = match crate::sandbox::sandbox_path(&path) {
                    Ok(p) => p,
                    Err(e) => {
                        return Ok(Flow::Value(Value::Result(Err(Box::new(Value::Str(e))))))
                    }
                };
                return Ok(Flow::Value(match std::fs::read_to_string(&path) {
                    Ok(s) => Value::Result(Ok(Box::new(Value::Str(s)))),
                    Err(e) => Value::Result(Err(Box::new(Value::Str(e.to_string())))),
                }));
            }
            if name == "write_file" {
                let path = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`write_file` 路徑必須是 str")),
                };
                let content = self.val(&args[1])?.to_string();
                if content.len() > crate::sandbox::MAX_WRITE_BYTES {
                    return Ok(Flow::Value(Value::Result(Err(Box::new(Value::Str(
                        "寫入超過 1MB 上限".into(),
                    ))))));
                }
                let path = match crate::sandbox::sandbox_path(&path) {
                    Ok(p) => p,
                    Err(e) => {
                        return Ok(Flow::Value(Value::Result(Err(Box::new(Value::Str(e))))))
                    }
                };
                return Ok(Flow::Value(match std::fs::write(&path, content) {
                    Ok(()) => Value::Result(Ok(Box::new(Value::Unit))),
                    Err(e) => Value::Result(Err(Box::new(Value::Str(e.to_string())))),
                }));
            }
            if name == "assert" {
                match self.val(&args[0])? {
                    Value::Bool(true) => return Ok(Flow::Value(Value::Unit)),
                    Value::Bool(false) => {
                        return Err(Diagnostic::new(span, "assert 失敗").hint("條件為 false"))
                    }
                    _ => return Err(Diagnostic::new(span, "`assert` 需要 bool")),
                }
            }
            if name == "panic" {
                let msg = self.val(&args[0])?.to_string();
                return Err(Diagnostic::new(span, format!("panic: {msg}")));
            }
            if name == "abs" {
                return Ok(Flow::Value(match self.val(&args[0])? {
                    Value::I32(n) => Value::I32(n.saturating_abs()),
                    Value::I64(n) => Value::I64(n.saturating_abs()),
                    Value::F64(n) => Value::F64(n.abs()),
                    _ => return Err(Diagnostic::new(span, "`abs` 類型錯誤")),
                }));
            }
            if name == "min" || name == "max" {
                let a = self.val(&args[0])?;
                let b = self.val(&args[1])?;
                let pick_min = name == "min";
                return Ok(Flow::Value(match (a, b, pick_min) {
                    (Value::I32(x), Value::I32(y), true) => Value::I32(x.min(y)),
                    (Value::I32(x), Value::I32(y), false) => Value::I32(x.max(y)),
                    (Value::I64(x), Value::I64(y), true) => Value::I64(x.min(y)),
                    (Value::I64(x), Value::I64(y), false) => Value::I64(x.max(y)),
                    (Value::F64(x), Value::F64(y), true) => Value::F64(x.min(y)),
                    (Value::F64(x), Value::F64(y), false) => Value::F64(x.max(y)),
                    _ => return Err(Diagnostic::new(span, "`min`/`max` 類型錯誤")),
                }));
            }
            if name == "sqrt" {
                return Ok(Flow::Value(match self.val(&args[0])? {
                    Value::F64(n) => Value::F64(n.sqrt()),
                    Value::I32(n) => Value::F64((n as f64).sqrt()),
                    _ => return Err(Diagnostic::new(span, "`sqrt` 需要數字")),
                }));
            }
            if name == "now_ms" {
                use std::time::{SystemTime, UNIX_EPOCH};
                let ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                return Ok(Flow::Value(Value::I64(ms)));
            }
            if name == "sleep_ms" {
                let n = match self.val(&args[0])? {
                    Value::I32(n) => n.max(0) as u64,
                    Value::I64(n) => n.max(0) as u64,
                    _ => return Err(Diagnostic::new(span, "`sleep_ms` 需要整數")),
                };
                let n = n.min(crate::sandbox::MAX_SLEEP_MS);
                std::thread::sleep(std::time::Duration::from_millis(n));
                return Ok(Flow::Value(Value::Unit));
            }
            if name == "http_get" {
                let url = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`http_get` 需要 URL 字串")),
                };
                return Ok(Flow::Value(http_get(&url)));
            }
            if name == "url_decode" {
                let s = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`url_decode` 需要 str")),
                };
                return Ok(Flow::Value(Value::Str(crate::http_server::url_decode(&s))));
            }
            if name == "http_serve" {
                return self.http_serve(args, span);
            }
            if name == "spawn" {
                let fname = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`spawn` 需要函式名稱")),
                };
                if !crate::sandbox::safe_ident(&fname) {
                    return Err(Diagnostic::new(span, "`spawn` 函式名稱不合法"));
                }
                let f = self
                    .functions
                    .get(&fname)
                    .cloned()
                    .ok_or_else(|| Diagnostic::new(span, format!("找不到函式 `{fname}`")))?;
                let functions = self.functions.clone();
                let methods = self.methods.clone();
                let handle = std::thread::spawn(move || {
                    let mut vm = Vm {
                        output: String::new(),
                        scopes: vec![HashMap::new()],
                        functions,
                        methods,
                        argv: vec![],
                        threads: HashMap::new(),
                        next_tid: 1,
                    };
                    vm.call_fn(&f, vec![]).map(|_| vm.output)
                });
                let id = self.next_tid;
                self.next_tid += 1;
                self.threads.insert(id, handle);
                return Ok(Flow::Value(Value::I32(id)));
            }
            if name == "join" {
                let id = match self.val(&args[0])? {
                    Value::I32(n) => n,
                    _ => return Err(Diagnostic::new(span, "`join` 需要 i32")),
                };
                match self.threads.remove(&id) {
                    Some(h) => match h.join() {
                        Ok(Ok(out)) => {
                            self.output.push_str(&out);
                            return Ok(Flow::Value(Value::Unit));
                        }
                        Ok(Err(d)) => return Err(d),
                        Err(_) => return Err(Diagnostic::new(span, "執行緒 panic")),
                    },
                    None => return Err(Diagnostic::new(span, "未知的 thread id")),
                }
            }
            if name == "extern_call" {
                let n = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`extern_call` 第一個參數是名稱")),
                };
                let mut rest = Vec::new();
                for a in args.iter().skip(1) {
                    rest.push(self.val(a)?);
                }
                return Ok(Flow::Value(host_ffi(&n, &rest, span)?));
            }
            if let Some(f) = self.functions.get(name).cloned() {
                let mut vs = Vec::new();
                for a in args {
                    match self.eval(a)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Break => return Ok(Flow::Break),
                        Flow::Continue => return Ok(Flow::Continue),
                        Flow::Value(v) => vs.push(v),
                    }
                }
                return self.call_fn(&f, vs);
            }
        }
        if let Expr::Field { base, name, .. } = callee {
            // Method call: prefer mutating in place for push/pop when base is a var
            if let Expr::Var {
                name: recv,
                span: rsp,
            } = base.as_ref()
            {
                if name == "push" {
                    let arg = self.val(&args[0])?;
                    match self.get_mut(recv, *rsp)? {
                        Value::Vec(xs) => {
                            xs.push(arg);
                            return Ok(Flow::Value(Value::Unit));
                        }
                        _ => return Err(Diagnostic::new(span, "`push` 只能用在 Vec")),
                    }
                }
                if name == "pop" {
                    match self.get_mut(recv, *rsp)? {
                        Value::Vec(xs) => {
                            return Ok(Flow::Value(Value::Option(xs.pop().map(Box::new))));
                        }
                        _ => return Err(Diagnostic::new(span, "`pop` 只能用在 Vec")),
                    }
                }
                if name == "clear" {
                    match self.get_mut(recv, *rsp)? {
                        Value::Vec(xs) => xs.clear(),
                        Value::Map(m) => m.clear(),
                        _ => return Err(Diagnostic::new(span, "`clear` 只能用在 Vec 或 Map")),
                    }
                    return Ok(Flow::Value(Value::Unit));
                }
                if name == "reverse" {
                    match self.get_mut(recv, *rsp)? {
                        Value::Vec(xs) => xs.reverse(),
                        _ => return Err(Diagnostic::new(span, "`reverse` 只能用在 Vec")),
                    }
                    return Ok(Flow::Value(Value::Unit));
                }
                if name == "set" {
                    let k = match self.val(&args[0])? {
                        Value::Str(s) => s,
                        _ => return Err(Diagnostic::new(span, "Map 鍵必須是 str")),
                    };
                    let v = self.val(&args[1])?;
                    match self.get_mut(recv, *rsp)? {
                        Value::Map(m) => {
                            m.insert(k, v);
                            return Ok(Flow::Value(Value::Unit));
                        }
                        _ => return Err(Diagnostic::new(span, "`set` 只能用在 Map")),
                    }
                }
            }
            let recv = self.val(base)?;
            if name == "clone" {
                return Ok(Flow::Value(recv));
            }
            if name == "len" {
                let n = match &recv {
                    Value::Vec(xs) => xs.len() as i32,
                    Value::Str(s) => s.len() as i32,
                    Value::Map(m) => m.len() as i32,
                    _ => return Err(Diagnostic::new(span, "`len` 只能用在 Vec、Map 或 str")),
                };
                return Ok(Flow::Value(Value::I32(n)));
            }
            if name == "is_empty" {
                let b = match &recv {
                    Value::Vec(xs) => xs.is_empty(),
                    Value::Str(s) => s.is_empty(),
                    Value::Map(m) => m.is_empty(),
                    _ => return Err(Diagnostic::new(span, "`is_empty` 類型錯誤")),
                };
                return Ok(Flow::Value(Value::Bool(b)));
            }
            if name == "contains" {
                let arg = self.val(&args[0])?;
                let b = match (&recv, arg) {
                    (Value::Vec(xs), a) => xs.iter().any(|x| eq(x, &a)),
                    (Value::Str(s), Value::Str(p)) => s.contains(&p),
                    (Value::Map(m), Value::Str(k)) => m.contains_key(&k),
                    _ => return Err(Diagnostic::new(span, "`contains` 類型錯誤")),
                };
                return Ok(Flow::Value(Value::Bool(b)));
            }
            if name == "get" {
                let k = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "Map 鍵必須是 str")),
                };
                return Ok(Flow::Value(match recv {
                    Value::Map(m) => Value::Option(m.get(&k).cloned().map(Box::new)),
                    _ => return Err(Diagnostic::new(span, "`get` 只能用在 Map")),
                }));
            }
            if name == "keys" {
                return Ok(Flow::Value(match recv {
                    Value::Map(m) => Value::Vec(m.keys().cloned().map(Value::Str).collect()),
                    _ => return Err(Diagnostic::new(span, "`keys` 只能用在 Map")),
                }));
            }
            if name == "split" {
                let sep = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`split` 需要字串")),
                };
                return Ok(Flow::Value(match recv {
                    Value::Str(s) => {
                        Value::Vec(s.split(&sep).map(|p| Value::Str(p.to_string())).collect())
                    }
                    _ => return Err(Diagnostic::new(span, "`split` 只能用在 str")),
                }));
            }
            if name == "replace" {
                let from = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`replace` 需要字串")),
                };
                let to = match self.val(&args[1])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`replace` 需要字串")),
                };
                return Ok(Flow::Value(match recv {
                    Value::Str(s) => Value::Str(s.replace(&from, &to)),
                    _ => return Err(Diagnostic::new(span, "`replace` 只能用在 str")),
                }));
            }
            if name == "trim" {
                return Ok(Flow::Value(match recv {
                    Value::Str(s) => Value::Str(s.trim().to_string()),
                    _ => return Err(Diagnostic::new(span, "`trim` 只能用在 str")),
                }));
            }
            if name == "to_upper" {
                return Ok(Flow::Value(match recv {
                    Value::Str(s) => Value::Str(s.to_uppercase()),
                    _ => return Err(Diagnostic::new(span, "`to_upper` 只能用在 str")),
                }));
            }
            if name == "to_lower" {
                return Ok(Flow::Value(match recv {
                    Value::Str(s) => Value::Str(s.to_lowercase()),
                    _ => return Err(Diagnostic::new(span, "`to_lower` 只能用在 str")),
                }));
            }
            if name == "starts_with" || name == "ends_with" {
                let p = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, format!("`{name}` 需要字串"))),
                };
                return Ok(Flow::Value(match recv {
                    Value::Str(s) => Value::Bool(if name == "starts_with" {
                        s.starts_with(&p)
                    } else {
                        s.ends_with(&p)
                    }),
                    _ => return Err(Diagnostic::new(span, format!("`{name}` 只能用在 str"))),
                }));
            }
            if name == "parse_i32" {
                return Ok(Flow::Value(match recv {
                    Value::Str(s) => match s.parse::<i32>() {
                        Ok(n) => Value::Result(Ok(Box::new(Value::I32(n)))),
                        Err(e) => Value::Result(Err(Box::new(Value::Str(e.to_string())))),
                    },
                    _ => return Err(Diagnostic::new(span, "`parse_i32` 只能用在 str")),
                }));
            }
            if name == "join" {
                let sep = match self.val(&args[0])? {
                    Value::Str(s) => s,
                    _ => return Err(Diagnostic::new(span, "`join` 需要字串")),
                };
                return Ok(Flow::Value(match recv {
                    Value::Vec(xs) => Value::Str(
                        xs.iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(&sep),
                    ),
                    _ => return Err(Diagnostic::new(span, "`join` 只能用在 Vec")),
                }));
            }
            if name == "push"
                || name == "pop"
                || name == "set"
                || name == "clear"
                || name == "reverse"
            {
                return Err(Diagnostic::new(
                    span,
                    format!("`{name}` 需要變數作為接收者"),
                ));
            }
            let type_name = match &recv {
                Value::Struct(n, _) => n.clone(),
                Value::Enum(n, _, _) => n.clone(),
                _ => return Err(Diagnostic::new(span, format!("值沒有方法 `{name}`"))),
            };
            if let Some(m) = self
                .methods
                .get(&type_name)
                .and_then(|ms| ms.get(name))
                .cloned()
            {
                let mut vs = vec![recv];
                for a in args {
                    vs.push(self.val(a)?);
                }
                return self.call_fn(&m, vs);
            }
            return Err(Diagnostic::new(
                span,
                format!("`{type_name}` 沒有方法 `{name}`"),
            ));
        }
        Err(Diagnostic::new(span, "無法呼叫"))
    }

    fn unary(&self, op: UnOp, v: Value, span: Span) -> Result<Value, Diagnostic> {
        match op {
            UnOp::Neg => match v {
                Value::I32(n) => n
                    .checked_neg()
                    .map(Value::I32)
                    .ok_or_else(|| Diagnostic::new(span, "i32 溢位")),
                Value::I64(n) => n
                    .checked_neg()
                    .map(Value::I64)
                    .ok_or_else(|| Diagnostic::new(span, "i64 溢位")),
                Value::F64(n) => Ok(Value::F64(-n)),
                _ => Err(Diagnostic::new(span, "負號只能用在數字")),
            },
            UnOp::BitNot => match v {
                Value::I32(n) => Ok(Value::I32(!n)),
                Value::I64(n) => Ok(Value::I64(!n)),
                _ => Err(Diagnostic::new(span, "`~` 只能用在整數")),
            },
            UnOp::Not => match v {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                _ => Err(Diagnostic::new(span, "`!` 只能用在 bool")),
            },
            UnOp::Ref | UnOp::RefMut | UnOp::Deref => Ok(v),
        }
    }

    fn binary(&self, op: BinOp, l: Value, r: Value, span: Span) -> Result<Value, Diagnostic> {
        match op {
            BinOp::And => match (l, r) {
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
                _ => Err(Diagnostic::new(span, "`&&` 需要 bool")),
            },
            BinOp::Or => match (l, r) {
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
                _ => Err(Diagnostic::new(span, "`||` 需要 bool")),
            },
            BinOp::Eq => Ok(Value::Bool(eq(&l, &r))),
            BinOp::Ne => Ok(Value::Bool(!eq(&l, &r))),
            BinOp::Add => match (l, r) {
                (Value::I32(a), Value::I32(b)) => a
                    .checked_add(b)
                    .map(Value::I32)
                    .ok_or_else(|| Diagnostic::new(span, "i32 加法溢位")),
                (Value::I64(a), Value::I64(b)) => a
                    .checked_add(b)
                    .map(Value::I64)
                    .ok_or_else(|| Diagnostic::new(span, "i64 加法溢位")),
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a + b)),
                (Value::Str(a), b) => Ok(Value::Str(format!("{a}{b}"))),
                (a, Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
                _ => Err(Diagnostic::new(span, "加法類型不支援")),
            },
            BinOp::Sub => match (l, r) {
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a - b)),
                (l, r) => int_binop(l, r, span, "減", i32::checked_sub, i64::checked_sub),
            },
            BinOp::Mul => match (l, r) {
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a * b)),
                (l, r) => int_binop(l, r, span, "乘", i32::checked_mul, i64::checked_mul),
            },
            BinOp::Div => match (l, r) {
                (Value::I32(_), Value::I32(0)) | (Value::I64(_), Value::I64(0)) => {
                    Err(Diagnostic::new(span, "除以零"))
                }
                (Value::F64(_), Value::F64(b)) if b == 0.0 => Err(Diagnostic::new(span, "除以零")),
                (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a / b)),
                (Value::I32(a), Value::I32(b)) => a
                    .checked_div(b)
                    .map(Value::I32)
                    .ok_or_else(|| Diagnostic::new(span, "i32 除法溢位")),
                (Value::I64(a), Value::I64(b)) => a
                    .checked_div(b)
                    .map(Value::I64)
                    .ok_or_else(|| Diagnostic::new(span, "i64 除法溢位")),
                _ => Err(Diagnostic::new(span, "除法類型不支援")),
            },
            BinOp::Mod => match (l, r) {
                (_, Value::I32(0)) | (_, Value::I64(0)) => Err(Diagnostic::new(span, "除以零")),
                (Value::I32(a), Value::I32(b)) => Ok(Value::I32(a % b)),
                (Value::I64(a), Value::I64(b)) => Ok(Value::I64(a % b)),
                (Value::F64(a), Value::F64(b)) if b != 0.0 => Ok(Value::F64(a % b)),
                _ => Err(Diagnostic::new(span, "取餘類型不支援")),
            },
            BinOp::BitAnd => bit_binop(l, r, span, |a, b| a & b, |a, b| a & b),
            BinOp::BitOr => bit_binop(l, r, span, |a, b| a | b, |a, b| a | b),
            BinOp::BitXor => bit_binop(l, r, span, |a, b| a ^ b, |a, b| a ^ b),
            BinOp::Shl => bit_binop(
                l,
                r,
                span,
                |a, b| a.wrapping_shl(b as u32),
                |a, b| a.wrapping_shl(b as u32),
            ),
            BinOp::Shr => bit_binop(
                l,
                r,
                span,
                |a, b| a.wrapping_shr(b as u32),
                |a, b| a.wrapping_shr(b as u32),
            ),
            BinOp::Lt => cmp(l, r, span, |o| o.is_lt()),
            BinOp::Le => cmp(l, r, span, |o| !o.is_gt()),
            BinOp::Gt => cmp(l, r, span, |o| o.is_gt()),
            BinOp::Ge => cmp(l, r, span, |o| !o.is_lt()),
        }
    }

    fn field(&self, v: &Value, name: &str, span: Span) -> Result<Value, Diagnostic> {
        match v {
            Value::Struct(_, fields) => fields
                .get(name)
                .cloned()
                .ok_or_else(|| Diagnostic::new(span, format!("沒有欄位 `{name}`"))),
            Value::Vec(xs) if name == "len" => Ok(Value::I32(xs.len() as i32)),
            Value::Str(s) if name == "len" => Ok(Value::I32(s.len() as i32)),
            _ => Err(Diagnostic::new(span, format!("值沒有欄位 `{name}`"))),
        }
    }

    fn index(&self, b: &Value, i: &Value, span: Span) -> Result<Value, Diagnostic> {
        let idx = match i {
            Value::I32(n) => *n,
            _ => return Err(Diagnostic::new(span, "索引必須是 i32")),
        };
        match b {
            Value::Vec(xs) => {
                if idx < 0 || idx as usize >= xs.len() {
                    return Err(Diagnostic::new(
                        span,
                        format!("索引 {idx} 超出範圍（長度 {}）", xs.len()),
                    ));
                }
                Ok(xs[idx as usize].clone())
            }
            _ => Err(Diagnostic::new(span, "只能索引 Vec")),
        }
    }

    fn assign(&mut self, place: &Expr, value: Value, span: Span) -> Result<(), Diagnostic> {
        match place {
            Expr::Var { name, span } => {
                *self.get_mut(name, *span)? = value;
                Ok(())
            }
            Expr::Field { base, name, span } => {
                if let Expr::Var {
                    name: recv,
                    span: rsp,
                } = base.as_ref()
                {
                    match self.get_mut(recv, *rsp)? {
                        Value::Struct(_, fields) => {
                            fields.insert(name.clone(), value);
                            return Ok(());
                        }
                        _ => return Err(Diagnostic::new(*span, "只能對結構欄位賦值")),
                    }
                }
                Err(Diagnostic::new(*span, "欄位賦值目前只支援變數.欄位"))
            }
            Expr::Index { base, index, span } => {
                let i = match self.val(index)? {
                    Value::I32(n) => n,
                    _ => return Err(Diagnostic::new(*span, "索引必須是 i32")),
                };
                if let Expr::Var {
                    name: recv,
                    span: rsp,
                } = base.as_ref()
                {
                    match self.get_mut(recv, *rsp)? {
                        Value::Vec(xs) => {
                            if i < 0 || i as usize >= xs.len() {
                                return Err(Diagnostic::new(*span, "索引超出範圍"));
                            }
                            xs[i as usize] = value;
                            return Ok(());
                        }
                        _ => return Err(Diagnostic::new(*span, "只能對 Vec 索引賦值")),
                    }
                }
                Err(Diagnostic::new(*span, "索引賦值目前只支援變數[i]"))
            }
            Expr::Unary {
                op: UnOp::Deref,
                expr,
                ..
            } => self.assign(expr, value, span),
            _ => Err(Diagnostic::new(span, "無效的賦值位置")),
        }
    }

    fn match_pat(&mut self, pat: &Pat, v: &Value) -> Result<bool, Diagnostic> {
        match pat {
            Pat::Wildcard => Ok(true),
            Pat::Binding(name) => {
                self.define(name.clone(), v.clone());
                Ok(true)
            }
            Pat::Int(n) => Ok(matches!(v, Value::I32(x) if *x as i64 == *n)
                || matches!(v, Value::I64(x) if *x == *n)),
            Pat::Bool(b) => Ok(matches!(v, Value::Bool(x) if x == b)),
            Pat::Str(s) => Ok(matches!(v, Value::Str(x) if x == s)),
            Pat::Variant { name, args, .. } => match (name.as_str(), v) {
                ("Some", Value::Option(Some(inner))) => {
                    if args.len() == 1 {
                        self.match_pat(&args[0], inner)
                    } else {
                        Ok(false)
                    }
                }
                ("None", Value::Option(None)) => Ok(true),
                ("Ok", Value::Result(Ok(inner))) => {
                    if args.len() == 1 {
                        self.match_pat(&args[0], inner)
                    } else {
                        Ok(false)
                    }
                }
                ("Err", Value::Result(Err(inner))) => {
                    if args.len() == 1 {
                        self.match_pat(&args[0], inner)
                    } else {
                        Ok(false)
                    }
                }
                (_, Value::Enum(_, var, vs)) if var == name => {
                    if args.len() != vs.len() {
                        return Ok(false);
                    }
                    for (p, val) in args.iter().zip(vs) {
                        if !self.match_pat(p, val)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                _ => Ok(false),
            },
        }
    }
}

fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::I32(x), Value::I32(y)) => x == y,
        (Value::I64(x), Value::I64(y)) => x == y,
        (Value::F64(x), Value::F64(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Unit, Value::Unit) => true,
        (Value::Option(x), Value::Option(y)) => match (x, y) {
            (None, None) => true,
            (Some(a), Some(b)) => eq(a, b),
            _ => false,
        },
        _ => false,
    }
}

fn int_binop(
    l: Value,
    r: Value,
    span: Span,
    name: &str,
    f32: fn(i32, i32) -> Option<i32>,
    f64: fn(i64, i64) -> Option<i64>,
) -> Result<Value, Diagnostic> {
    match (l, r) {
        (Value::I32(a), Value::I32(b)) => f32(a, b)
            .map(Value::I32)
            .ok_or_else(|| Diagnostic::new(span, format!("i32 {name}法溢位"))),
        (Value::I64(a), Value::I64(b)) => f64(a, b)
            .map(Value::I64)
            .ok_or_else(|| Diagnostic::new(span, format!("i64 {name}法溢位"))),
        _ => Err(Diagnostic::new(span, format!("{name}法類型不支援"))),
    }
}

fn cmp(
    l: Value,
    r: Value,
    span: Span,
    pred: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<Value, Diagnostic> {
    let ord = match (l, r) {
        (Value::I32(a), Value::I32(b)) => a.cmp(&b),
        (Value::I64(a), Value::I64(b)) => a.cmp(&b),
        (Value::F64(a), Value::F64(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        _ => return Err(Diagnostic::new(span, "比較類型不支援")),
    };
    Ok(Value::Bool(pred(ord)))
}

fn bit_binop(
    l: Value,
    r: Value,
    span: Span,
    fi: fn(i32, i32) -> i32,
    fl: fn(i64, i64) -> i64,
) -> Result<Value, Diagnostic> {
    match (l, r) {
        (Value::I32(a), Value::I32(b)) => Ok(Value::I32(fi(a, b))),
        (Value::I64(a), Value::I64(b)) => Ok(Value::I64(fl(a, b))),
        _ => Err(Diagnostic::new(span, "位元運算只能用在整數")),
    }
}

fn host_ffi(name: &str, args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let f64_arg = |i: usize| match args.get(i) {
        Some(Value::F64(n)) => Ok(*n),
        Some(Value::I32(n)) => Ok(*n as f64),
        Some(Value::I64(n)) => Ok(*n as f64),
        _ => Err(Diagnostic::new(span, format!("FFI `{name}` 參數錯誤"))),
    };
    match name {
        "sin" => Ok(Value::F64(f64_arg(0)?.sin())),
        "cos" => Ok(Value::F64(f64_arg(0)?.cos())),
        "sqrt" => Ok(Value::F64(f64_arg(0)?.sqrt())),
        "floor" => Ok(Value::F64(f64_arg(0)?.floor())),
        "ceil" => Ok(Value::F64(f64_arg(0)?.ceil())),
        "pow" => Ok(Value::F64(f64_arg(0)?.powf(f64_arg(1)?))),
        "abs" => match args.first() {
            Some(Value::I32(n)) => Ok(Value::I32(n.saturating_abs())),
            Some(Value::F64(n)) => Ok(Value::F64(n.abs())),
            _ => Err(Diagnostic::new(span, "FFI abs 參數錯誤")),
        },
        other => Err(Diagnostic::new(
            span,
            format!("未知的 FFI `{other}`。內建：sin, cos, sqrt, floor, ceil, pow, abs"),
        )),
    }
}

fn http_get(url: &str) -> Value {
    match http_get_inner(url) {
        Ok(s) => Value::Result(Ok(Box::new(Value::Str(s)))),
        Err(e) => Value::Result(Err(Box::new(Value::Str(e)))),
    }
}

fn http_get_inner(url: &str) -> Result<String, String> {
    if url.len() > 2048 {
        return Err("URL 過長".into());
    }
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "只支援 http:// URL（不內建 TLS，且拒絕其他協定）".to_string())?;
    let (hostport, path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (rest, "/".to_string()),
    };
    if path.contains('\n') || path.contains('\r') {
        return Err("URL 含有非法字元".into());
    }
    let (host, port) = if let Some((h, p)) = hostport.split_once(':') {
        (h, p.parse::<u16>().map_err(|e| e.to_string())?)
    } else {
        (hostport, 80)
    };
    if port != 80 {
        return Err("只允許連接埠 80".into());
    }
    crate::sandbox::is_public_host(host)?;
    use std::io::{Read, Write};
    use std::net::ToSocketAddrs;
    use std::time::Duration;
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| "無法解析主機".to_string())?;
    let mut stream = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let req = format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() > crate::sandbox::MAX_HTTP_BYTES {
                    return Err("回應超過大小上限".into());
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    let text = String::from_utf8_lossy(&buf).into_owned();
    if let Some((_, body)) = text.split_once("\r\n\r\n") {
        Ok(body.to_string())
    } else {
        Ok(text)
    }
}

pub fn run(checked: &CheckedProgram) -> Result<(Value, String), Diagnostic> {
    run_with_args(checked, vec![])
}

pub fn run_with_args(
    checked: &CheckedProgram,
    argv: Vec<String>,
) -> Result<(Value, String), Diagnostic> {
    let mut vm = Vm::new(checked).with_args(argv);
    let v = vm.run_main()?;
    Ok((v, vm.output))
}
