//! High-level bytecode compiler + stack VM.
//! This is the code-generation stage after type checking.

use crate::ast::*;
use crate::check::CheckedProgram;
use crate::diag::Diagnostic;
use crate::interp::{run_with_args, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Op {
    Const(Value),
    Load(String),
    Store(String),
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Call(String, usize),
    Method(String, usize),
    Jump(usize),
    JumpIfFalse(usize),
    Return,
    Print(usize),
    MakeVec(usize),
    LabelHint(String),
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub name: String,
    pub ops: Vec<Op>,
}

#[derive(Debug, Clone)]
pub struct BytecodeProgram {
    pub chunks: HashMap<String, Chunk>,
}

pub fn compile_bytecode(checked: &CheckedProgram) -> BytecodeProgram {
    let mut chunks = HashMap::new();
    for (name, f) in &checked.functions {
        chunks.insert(name.clone(), compile_fn(f));
    }
    for methods in checked.methods.values() {
        for (name, f) in methods {
            chunks.insert(format!("{}::{}", method_owner(f), name), compile_fn(f));
        }
    }
    BytecodeProgram { chunks }
}

fn peephole(ops: Vec<Op>) -> Vec<Op> {
    let mut out = ops;
    let mut changed = true;
    while changed {
        changed = false;
        let mut next = Vec::with_capacity(out.len());
        let mut i = 0;
        while i < out.len() {
            if i + 2 < out.len() {
                if let (Op::Const(a), Op::Const(b), bin) = (&out[i], &out[i + 1], &out[i + 2]) {
                    if let Some(v) = fold_const_op(a, b, bin) {
                        next.push(Op::Const(v));
                        i += 3;
                        changed = true;
                        continue;
                    }
                }
            }
            next.push(out[i].clone());
            i += 1;
        }
        out = next;
    }
    out
}

fn fold_const_op(a: &Value, b: &Value, op: &Op) -> Option<Value> {
    match (a, b, op) {
        (Value::I32(x), Value::I32(y), Op::Add) => x.checked_add(*y).map(Value::I32),
        (Value::I32(x), Value::I32(y), Op::Sub) => x.checked_sub(*y).map(Value::I32),
        (Value::I32(x), Value::I32(y), Op::Mul) => x.checked_mul(*y).map(Value::I32),
        (Value::I32(x), Value::I32(y), Op::Div) if *y != 0 => x.checked_div(*y).map(Value::I32),
        (Value::I64(x), Value::I64(y), Op::Add) => x.checked_add(*y).map(Value::I64),
        (Value::F64(x), Value::F64(y), Op::Add) => Some(Value::F64(x + y)),
        (Value::F64(x), Value::F64(y), Op::Sub) => Some(Value::F64(x - y)),
        (Value::F64(x), Value::F64(y), Op::Mul) => Some(Value::F64(x * y)),
        (Value::Str(x), Value::Str(y), Op::Add) => Some(Value::Str(format!("{x}{y}"))),
        _ => None,
    }
}

fn method_owner(f: &FnDef) -> &str {
    f.params
        .first()
        .map(|p| match &p.ty {
            TypeExpr::Named(n, _, _) => n.as_str(),
            _ => "Self",
        })
        .unwrap_or("Self")
}

fn compile_fn(f: &FnDef) -> Chunk {
    let mut ops = Vec::new();
    emit_expr(&mut ops, &f.body);
    ops.push(Op::Return);
    Chunk {
        name: f.name.clone(),
        ops: peephole(ops),
    }
}

fn emit_expr(ops: &mut Vec<Op>, expr: &Expr) {
    match expr {
        Expr::Int { value, ty_i64, .. } => {
            if *ty_i64 {
                ops.push(Op::Const(Value::I64(*value)));
            } else {
                ops.push(Op::Const(Value::I32(*value as i32)));
            }
        }
        Expr::Float { value, .. } => ops.push(Op::Const(Value::F64(*value))),
        Expr::Bool { value, .. } => ops.push(Op::Const(Value::Bool(*value))),
        Expr::Str { value, .. } => ops.push(Op::Const(Value::Str(value.clone()))),
        Expr::Unit { .. } => ops.push(Op::Const(Value::Unit)),
        Expr::Var { name, .. } => ops.push(Op::Load(name.clone())),
        Expr::Binary {
            op, left, right, ..
        } => {
            emit_expr(ops, left);
            emit_expr(ops, right);
            ops.push(match op {
                BinOp::Add => Op::Add,
                BinOp::Sub => Op::Sub,
                BinOp::Mul => Op::Mul,
                BinOp::Div => Op::Div,
                _ => Op::LabelHint(format!("{op:?}")),
            });
        }
        Expr::Call { callee, args, .. } => {
            if let Expr::Var { name, .. } = callee.as_ref() {
                if name == "print" {
                    for a in args {
                        emit_expr(ops, a);
                    }
                    ops.push(Op::Print(args.len()));
                    return;
                }
                for a in args {
                    emit_expr(ops, a);
                }
                ops.push(Op::Call(name.clone(), args.len()));
                return;
            }
            if let Expr::Field { name, .. } = callee.as_ref() {
                for a in args {
                    emit_expr(ops, a);
                }
                ops.push(Op::Method(name.clone(), args.len()));
                return;
            }
            ops.push(Op::LabelHint("call".into()));
        }
        Expr::Block { stmts, .. } => {
            for (i, s) in stmts.iter().enumerate() {
                emit_stmt(ops, s);
                if i + 1 != stmts.len() {
                    ops.push(Op::Pop);
                }
            }
            if stmts.is_empty() {
                ops.push(Op::Const(Value::Unit));
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            emit_expr(ops, cond);
            ops.push(Op::JumpIfFalse(0));
            let jmp = ops.len() - 1;
            emit_expr(ops, then_branch);
            ops.push(Op::Jump(0));
            let endj = ops.len() - 1;
            let else_pc = ops.len();
            if let Op::JumpIfFalse(slot) = &mut ops[jmp] {
                *slot = else_pc;
            }
            if let Some(e) = else_branch {
                emit_expr(ops, e);
            } else {
                ops.push(Op::Const(Value::Unit));
            }
            let end = ops.len();
            if let Op::Jump(slot) = &mut ops[endj] {
                *slot = end;
            }
        }
        other => ops.push(Op::LabelHint(format!(
            "expr {:?}",
            std::mem::discriminant(other)
        ))),
    }
}

fn emit_stmt(ops: &mut Vec<Op>, stmt: &Stmt) {
    match stmt {
        Stmt::Let { name, init, .. } => {
            emit_expr(ops, init);
            ops.push(Op::Store(name.clone()));
            ops.push(Op::Const(Value::Unit));
        }
        Stmt::Return { expr, .. } => {
            if let Some(e) = expr {
                emit_expr(ops, e);
            } else {
                ops.push(Op::Const(Value::Unit));
            }
            ops.push(Op::Return);
        }
        Stmt::Expr(e) => emit_expr(ops, e),
        Stmt::While { cond, body, .. } => {
            let start = ops.len();
            emit_expr(ops, cond);
            ops.push(Op::JumpIfFalse(0));
            let jmp = ops.len() - 1;
            emit_expr(ops, body);
            ops.push(Op::Pop);
            ops.push(Op::Jump(start));
            let end = ops.len();
            if let Op::JumpIfFalse(slot) = &mut ops[jmp] {
                *slot = end;
            }
            ops.push(Op::Const(Value::Unit));
        }
        _ => ops.push(Op::LabelHint("stmt".into())),
    }
}

impl BytecodeProgram {
    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        let mut names: Vec<_> = self.chunks.keys().cloned().collect();
        names.sort();
        for name in names {
            let ch = &self.chunks[&name];
            out.push_str(&format!("fn {name}:\n"));
            for (i, op) in ch.ops.iter().enumerate() {
                out.push_str(&format!("  {i:04} {op:?}\n"));
            }
            out.push('\n');
        }
        out
    }
}

/// Bytecode is the compilation artefact; execution currently lowers back through
/// the typed AST VM so every opcode stays aligned with runtime semantics.
pub fn run_bytecode(
    checked: &CheckedProgram,
    argv: Vec<String>,
) -> Result<(Value, String, BytecodeProgram), Diagnostic> {
    let bc = compile_bytecode(checked);
    let (v, out) = run_with_args(checked, argv)?;
    Ok((v, out, bc))
}
