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
    Bool(bool),
    Str(String),
    Unit,
    Vec(Vec<Value>),
    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),
    Range(i32, i32),
    Struct(String, HashMap<String, Value>),
    Enum(String, String, Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I32(n) => write!(f, "{n}"),
            Value::I64(n) => write!(f, "{n}"),
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
}

pub struct Vm {
    pub output: String,
    scopes: Vec<HashMap<String, Slot>>,
    functions: HashMap<String, FnDef>,
    methods: HashMap<String, HashMap<String, FnDef>>,
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
                    Flow::Value(v) => v,
                };
                self.define(name.clone(), v);
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::Return { expr, .. } => {
                let v = if let Some(e) = expr {
                    match self.eval(e)? {
                        Flow::Return(v) => v,
                        Flow::Value(v) => v,
                    }
                } else {
                    Value::Unit
                };
                Ok(Flow::Return(v))
            }
            Stmt::While { cond, body, span } => loop {
                match self.val(cond)? {
                    Value::Bool(true) => match self.eval(body)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Value(_) => {}
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
                        Flow::Value(_) => {}
                    }
                    self.pop();
                }
                Ok(Flow::Value(Value::Unit))
            }
            Stmt::Expr(e) => self.eval(e),
        }
    }

    fn eval_call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Result<Flow, Diagnostic> {
        if let Expr::Var { name, .. } = callee {
            if name == "print" {
                let mut parts = Vec::new();
                for a in args {
                    match self.eval(a)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
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
            if let Some(f) = self.functions.get(name).cloned() {
                let mut vs = Vec::new();
                for a in args {
                    match self.eval(a)? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
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
            }
            let recv = self.val(base)?;
            if name == "clone" {
                return Ok(Flow::Value(recv));
            }
            if name == "len" {
                let n = match &recv {
                    Value::Vec(xs) => xs.len() as i32,
                    Value::Str(s) => s.len() as i32,
                    _ => return Err(Diagnostic::new(span, "`len` 只能用在 Vec 或 str")),
                };
                return Ok(Flow::Value(Value::I32(n)));
            }
            if name == "push" {
                return Err(Diagnostic::new(
                    span,
                    "`push` 需要變數作為接收者，例如 `xs.push(1)`",
                ));
            }
            if name == "pop" {
                return Err(Diagnostic::new(
                    span,
                    "`pop` 需要變數作為接收者，例如 `xs.pop()`",
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
                _ => Err(Diagnostic::new(span, "負號只能用在整數")),
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
                (Value::Str(a), b) => Ok(Value::Str(format!("{a}{b}"))),
                (a, Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
                _ => Err(Diagnostic::new(span, "加法類型不支援")),
            },
            BinOp::Sub => int_binop(l, r, span, "減", i32::checked_sub, i64::checked_sub),
            BinOp::Mul => int_binop(l, r, span, "乘", i32::checked_mul, i64::checked_mul),
            BinOp::Div => match (l, r) {
                (Value::I32(_), Value::I32(0)) | (Value::I64(_), Value::I64(0)) => {
                    Err(Diagnostic::new(span, "除以零"))
                }
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
                _ => Err(Diagnostic::new(span, "取餘類型不支援")),
            },
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
        _ => return Err(Diagnostic::new(span, "比較類型不支援")),
    };
    Ok(Value::Bool(pred(ord)))
}

/// Evaluate a checked program; `print` output is collected on the VM and also written to stdout.
pub fn run(checked: &CheckedProgram) -> Result<(Value, String), Diagnostic> {
    let mut vm = Vm::new(checked);
    let v = vm.run_main()?;
    Ok((v, vm.output))
}
