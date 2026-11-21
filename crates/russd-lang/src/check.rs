use crate::ast::*;
use crate::diag::Diagnostic;
use crate::span::Span;
use crate::types::{Binding, Type};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CheckedProgram {
    pub program: Program,
    pub structs: HashMap<String, StructDef>,
    pub enums: HashMap<String, EnumDef>,
    pub functions: HashMap<String, FnDef>,
    pub methods: HashMap<String, HashMap<String, FnDef>>,
}

struct Infer {
    next: u32,
    subst: HashMap<u32, Type>,
}

impl Infer {
    fn new() -> Self {
        Self {
            next: 0,
            subst: HashMap::new(),
        }
    }

    fn fresh(&mut self) -> Type {
        let id = self.next;
        self.next += 1;
        Type::Infer(id)
    }

    fn resolve(&self, ty: &Type) -> Type {
        match ty {
            Type::Infer(id) => {
                if let Some(t) = self.subst.get(id) {
                    self.resolve(t)
                } else {
                    ty.clone()
                }
            }
            Type::Vec(t) => Type::Vec(Box::new(self.resolve(t))),
            Type::Map(t) => Type::Map(Box::new(self.resolve(t))),
            Type::Option(t) => Type::Option(Box::new(self.resolve(t))),
            Type::Result(t, e) => {
                Type::Result(Box::new(self.resolve(t)), Box::new(self.resolve(e)))
            }
            Type::Ref(t, m) => Type::Ref(Box::new(self.resolve(t)), *m),
            _ => ty.clone(),
        }
    }

    fn unify(&mut self, a: &Type, b: &Type, span: Span) -> Result<Type, Diagnostic> {
        let a = self.resolve(a);
        let b = self.resolve(b);
        if a == b {
            return Ok(a);
        }
        match (&a, &b) {
            (Type::Never, t) | (t, Type::Never) => Ok(t.clone()),
            (Type::Infer(id), t) | (t, Type::Infer(id)) => {
                self.subst.insert(*id, t.clone());
                Ok(t.clone())
            }
            (Type::Vec(x), Type::Vec(y)) => Ok(Type::Vec(Box::new(self.unify(x, y, span)?))),
            (Type::Map(x), Type::Map(y)) => Ok(Type::Map(Box::new(self.unify(x, y, span)?))),
            (Type::Option(x), Type::Option(y)) => {
                Ok(Type::Option(Box::new(self.unify(x, y, span)?)))
            }
            (Type::Result(x1, e1), Type::Result(x2, e2)) => Ok(Type::Result(
                Box::new(self.unify(x1, x2, span)?),
                Box::new(self.unify(e1, e2, span)?),
            )),
            (Type::Ref(x, m1), Type::Ref(y, m2)) if m1 == m2 => {
                Ok(Type::Ref(Box::new(self.unify(x, y, span)?), *m1))
            }
            _ => Err(Diagnostic::new(
                span,
                format!("類型不符：預期 `{}`，實際是 `{}`", a.display(), b.display()),
            )
            .hint("檢查變數型別、函式回傳值，或加上明確的型別標註")),
        }
    }
}

struct Ctx<'a> {
    structs: &'a HashMap<String, StructDef>,
    enums: &'a HashMap<String, EnumDef>,
    functions: &'a HashMap<String, FnDef>,
    methods: &'a HashMap<String, HashMap<String, FnDef>>,
    scopes: Vec<HashMap<String, Binding>>,
    infer: Infer,
    ret: Type,
    impl_ty: Option<String>,
    in_loop: u32,
}

impl<'a> Ctx<'a> {
    fn error_moved(name: &str, span: Span) -> Diagnostic {
        Diagnostic::new(span, format!("`{name}` 已被移動，無法再使用"))
            .hint(format!("改成 `{name}.clone()`，或改傳 `&{name}`"))
    }

    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: String, b: Binding) {
        self.scopes.last_mut().unwrap().insert(name, b);
    }

    fn lookup(&mut self, name: &str) -> Option<&mut Binding> {
        for s in self.scopes.iter_mut().rev() {
            if s.contains_key(name) {
                return s.get_mut(name);
            }
        }
        None
    }

    fn get(&self, name: &str) -> Option<&Binding> {
        for s in self.scopes.iter().rev() {
            if let Some(b) = s.get(name) {
                return Some(b);
            }
        }
        None
    }

    fn resolve_type_expr(&mut self, te: &TypeExpr) -> Result<Type, Diagnostic> {
        match te {
            TypeExpr::Unit(_) => Ok(Type::Unit),
            TypeExpr::Ref {
                inner,
                mutable,
                span: _,
            } => {
                let t = self.resolve_type_expr(inner)?;
                Ok(Type::Ref(Box::new(t), *mutable))
            }
            TypeExpr::Named(name, args, span) => {
                if name == "Self" {
                    if let Some(t) = &self.impl_ty {
                        return Ok(Type::Named(t.clone()));
                    }
                    return Err(Diagnostic::new(*span, "`Self` 只能用在 `impl` 裡"));
                }
                match name.as_str() {
                    "i32" => Ok(Type::I32),
                    "i64" => Ok(Type::I64),
                    "f64" => Ok(Type::F64),
                    "bool" => Ok(Type::Bool),
                    "str" => Ok(Type::Str),
                    "Range" => Ok(Type::Range),
                    "Map" => {
                        let inner = if args.len() == 1 {
                            self.resolve_type_expr(&args[0])?
                        } else if args.len() == 2 {
                            let k = self.resolve_type_expr(&args[0])?;
                            if k != Type::Str {
                                return Err(Diagnostic::new(
                                    *span,
                                    "`Map` 的鍵必須是 `str`（寫成 `Map[V]` 或 `Map[str, V]`）",
                                ));
                            }
                            self.resolve_type_expr(&args[1])?
                        } else if args.is_empty() {
                            self.infer.fresh()
                        } else {
                            return Err(Diagnostic::new(*span, "`Map` 寫成 `Map[V]`"));
                        };
                        Ok(Type::Map(Box::new(inner)))
                    }
                    "Vec" => {
                        let inner = if args.is_empty() {
                            self.infer.fresh()
                        } else if args.len() == 1 {
                            self.resolve_type_expr(&args[0])?
                        } else {
                            return Err(Diagnostic::new(*span, "`Vec` 只接受一個型別參數"));
                        };
                        Ok(Type::Vec(Box::new(inner)))
                    }
                    "Option" => {
                        let inner = if args.is_empty() {
                            self.infer.fresh()
                        } else if args.len() == 1 {
                            self.resolve_type_expr(&args[0])?
                        } else {
                            return Err(Diagnostic::new(*span, "`Option` 只接受一個型別參數"));
                        };
                        Ok(Type::Option(Box::new(inner)))
                    }
                    "Result" => {
                        let (ok, err) = if args.is_empty() {
                            (self.infer.fresh(), self.infer.fresh())
                        } else if args.len() == 2 {
                            (
                                self.resolve_type_expr(&args[0])?,
                                self.resolve_type_expr(&args[1])?,
                            )
                        } else {
                            return Err(Diagnostic::new(
                                *span,
                                "`Result` 需要兩個型別參數，例如 `Result[i32, str]`",
                            ));
                        };
                        Ok(Type::Result(Box::new(ok), Box::new(err)))
                    }
                    _ => {
                        if self.structs.contains_key(name) || self.enums.contains_key(name) {
                            Ok(Type::Named(name.clone()))
                        } else {
                            Err(Diagnostic::new(*span, format!("未知類型 `{name}`")))
                        }
                    }
                }
            }
        }
    }

    fn use_var(
        &mut self,
        name: &str,
        span: Span,
        by_ref: bool,
        mut_ref: bool,
    ) -> Result<Type, Diagnostic> {
        if self.functions.contains_key(name) {
            return Err(Diagnostic::new(
                span,
                format!("`{name}` 是函式，請用 `{name}(...)` 呼叫"),
            ));
        }
        let copy;
        let ty;
        {
            let b = self
                .get(name)
                .ok_or_else(|| Diagnostic::new(span, format!("找不到名稱 `{name}`")))?;
            if b.moved {
                return Err(Self::error_moved(name, span));
            }
            copy = b.ty.is_copy();
            ty = b.ty.clone();
        }
        if by_ref {
            let b = self.lookup(name).unwrap();
            if mut_ref {
                if b.shared > 0 || b.unique {
                    return Err(Diagnostic::new(
                        span,
                        format!("無法對 `{name}` 取得 `&mut`：它已經被借用"),
                    )
                    .hint("同一時間只能有一個 `&mut`，且不能同時存在 `&`"));
                }
                if !b.mutable {
                    return Err(Diagnostic::new(
                        span,
                        format!("`{name}` 不是 `let mut`，不能 `&mut`"),
                    )
                    .hint(format!("改成 `let mut {name} = ...`")));
                }
                b.unique = true;
                return Ok(Type::Ref(Box::new(ty), true));
            }
            if b.unique {
                return Err(Diagnostic::new(
                    span,
                    format!("無法對 `{name}` 取得 `&`：它已經被獨佔借用"),
                ));
            }
            b.shared += 1;
            return Ok(Type::Ref(Box::new(ty), false));
        }
        if !copy {
            let b = self.lookup(name).unwrap();
            if b.shared > 0 || b.unique {
                return Err(
                    Diagnostic::new(span, format!("`{name}` 正在被借用，不能移動"))
                        .hint("等借用結束，或改用 `.clone()`"),
                );
            }
            b.moved = true;
        }
        Ok(ty)
    }

    fn peek_place_ty(&mut self, expr: &Expr) -> Result<Type, Diagnostic> {
        match expr {
            Expr::Var { name, span } => {
                if self.functions.contains_key(name) {
                    return Err(Diagnostic::new(*span, format!("`{name}` 是函式")));
                }
                let b = self
                    .get(name)
                    .ok_or_else(|| Diagnostic::new(*span, format!("找不到名稱 `{name}`")))?;
                if b.moved {
                    return Err(Self::error_moved(name, *span));
                }
                Ok(b.ty.clone())
            }
            Expr::Field { base, name, span } => {
                let ty = self.peek_place_ty(base)?;
                self.field_type(&ty, name, *span)
            }
            Expr::Index { base, index, span } => {
                let ty = self.peek_place_ty(base)?;
                self.check_expr(index, Some(&Type::I32))?;
                match ty.peel_ref() {
                    Type::Vec(inner) => Ok((**inner).clone()),
                    _ => Err(Diagnostic::new(*span, "只有 `Vec` 可以索引")),
                }
            }
            _ => self.check_expr(expr, None),
        }
    }

    fn borrow_receiver(
        &mut self,
        expr: &Expr,
        mutable: bool,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match expr {
            Expr::Var { name, span: vspan } => {
                let _ = self.use_var(name, *vspan, true, mutable)?;
                Ok(())
            }
            _ => {
                let _ = self.check_expr(expr, None)?;
                if mutable {
                    Err(Diagnostic::new(
                        span,
                        "可變方法目前只能呼叫在變數上，例如 `xs.push(1)`",
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn check_place_assign(&mut self, expr: &Expr) -> Result<Type, Diagnostic> {
        match expr {
            Expr::Var { name, span } => {
                let b = self
                    .lookup(name)
                    .ok_or_else(|| Diagnostic::new(*span, format!("找不到名稱 `{name}`")))?;
                if b.moved {
                    return Err(Self::error_moved(name, *span));
                }
                if !b.mutable {
                    return Err(Diagnostic::new(*span, format!("`{name}` 不可變，不能賦值"))
                        .hint(format!("改成 `let mut {name} = ...`")));
                }
                if b.shared > 0 || b.unique {
                    return Err(Diagnostic::new(
                        *span,
                        format!("`{name}` 正在被借用，不能賦值"),
                    ));
                }
                Ok(b.ty.clone())
            }
            Expr::Field { base, name, span } => {
                let ty = self.check_place_assign(base)?;
                self.field_type(&ty, name, *span)
            }
            Expr::Index { base, index, span } => {
                let ty = self.check_place_assign(base)?;
                self.check_expr(index, Some(&Type::I32))?;
                match ty.peel_ref() {
                    Type::Vec(inner) => Ok((**inner).clone()),
                    _ => Err(Diagnostic::new(*span, "只有 `Vec` 可以用 `[]` 賦值")),
                }
            }
            Expr::Unary {
                op: UnOp::Deref,
                expr,
                span,
            } => {
                let ty = self.check_expr(expr, None)?;
                match ty {
                    Type::Ref(inner, true) => Ok(*inner),
                    Type::Ref(_, false) => Err(Diagnostic::new(
                        *span,
                        "不能透過共享借用 `&T` 賦值，需要 `&mut T`",
                    )),
                    _ => Err(Diagnostic::new(*span, "只能解引用 `&mut T`")),
                }
            }
            _ => Err(Diagnostic::new(
                expr.span(),
                "賦值左邊必須是變數、欄位或索引",
            )),
        }
    }

    fn field_type(&self, ty: &Type, name: &str, span: Span) -> Result<Type, Diagnostic> {
        let ty = ty.peel_ref();
        match ty {
            Type::Named(s) => {
                if let Some(st) = self.structs.get(s) {
                    for (fnm, te) in &st.fields {
                        if fnm == name {
                            return type_from_struct_field(te);
                        }
                    }
                    Err(Diagnostic::new(span, format!("`{s}` 沒有欄位 `{name}`")))
                } else {
                    Err(Diagnostic::new(span, format!("`{s}` 不是結構，沒有欄位")))
                }
            }
            Type::Vec(_) | Type::Map(_) | Type::Str if name == "len" => Ok(Type::I32),
            _ => Err(Diagnostic::new(
                span,
                format!("類型 `{}` 沒有欄位 `{name}`", ty.display()),
            )),
        }
    }

    fn check_expr(&mut self, expr: &Expr, expected: Option<&Type>) -> Result<Type, Diagnostic> {
        let ty = match expr {
            Expr::Int {
                value,
                ty_i64,
                span,
            } => {
                if *ty_i64 {
                    Type::I64
                } else if *value > i32::MAX as i64 || *value < i32::MIN as i64 {
                    return Err(Diagnostic::new(*span, "整數超出 i32，請加 `i64` 後綴"));
                } else if matches!(expected, Some(Type::I64)) {
                    Type::I64
                } else {
                    Type::I32
                }
            }
            Expr::Float { .. } => Type::F64,
            Expr::Bool { .. } => Type::Bool,
            Expr::Str { .. } => Type::Str,
            Expr::Unit { .. } => Type::Unit,
            Expr::Var { name, span } => self.use_var(name, *span, false, false)?,
            Expr::Unary { op, expr, span } => match op {
                UnOp::Neg => {
                    let t = self.check_expr(expr, expected)?;
                    match t.peel_ref() {
                        Type::I32 | Type::I64 | Type::F64 => t.peel_ref().clone(),
                        _ => {
                            return Err(Diagnostic::new(*span, "只能對數字使用負號"));
                        }
                    }
                }
                UnOp::Not => {
                    self.check_expr(expr, Some(&Type::Bool))?;
                    Type::Bool
                }
                UnOp::BitNot => {
                    let t = self.check_expr(expr, None)?;
                    match t.peel_ref() {
                        Type::I32 | Type::I64 => t.peel_ref().clone(),
                        _ => return Err(Diagnostic::new(*span, "`~` 只能用在整數")),
                    }
                }
                UnOp::Ref => {
                    if let Expr::Var { name, span } = expr.as_ref() {
                        self.use_var(name, *span, true, false)?
                    } else {
                        let t = self.check_expr(expr, None)?;
                        Type::Ref(Box::new(t), false)
                    }
                }
                UnOp::RefMut => {
                    if let Expr::Var { name, span } = expr.as_ref() {
                        self.use_var(name, *span, true, true)?
                    } else {
                        return Err(Diagnostic::new(*span, "`&mut` 目前只能用在變數上"));
                    }
                }
                UnOp::Deref => {
                    let t = self.check_expr(expr, None)?;
                    match t {
                        Type::Ref(inner, _) => *inner,
                        _ => return Err(Diagnostic::new(*span, "只能解引用引用類型")),
                    }
                }
            },
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => self.check_binary(*op, left, right, *span)?,
            Expr::Assign { place, value, span } => {
                let lty = self.check_place_assign(place)?;
                let rty = self.check_expr(value, Some(&lty))?;
                self.infer.unify(&lty, &rty, *span)?;
                Type::Unit
            }
            Expr::Call { callee, args, span } => self.check_call(callee, args, *span, expected)?,
            Expr::Field { base, name, span } => {
                let ty = self.peek_place_ty(base)?;
                self.field_type(&ty, name, *span)?
            }
            Expr::Index { base, index, span } => {
                let bty = self.peek_place_ty(base)?;
                self.check_expr(index, Some(&Type::I32))?;
                match bty.peel_ref() {
                    Type::Vec(inner) => (**inner).clone(),
                    Type::Str => Type::I32, // not really, keep as error
                    _ => {
                        return Err(Diagnostic::new(
                            *span,
                            format!("不能索引類型 `{}`", bty.display()),
                        ));
                    }
                }
            }
            Expr::StructLit { name, fields, span } => {
                let st = self
                    .structs
                    .get(name)
                    .ok_or_else(|| Diagnostic::new(*span, format!("未知結構 `{name}`")))?
                    .clone();
                if fields.len() != st.fields.len() {
                    return Err(Diagnostic::new(*span, format!("`{name}` 欄位數量不對")));
                }
                for (fname, fexpr) in fields {
                    let fty = st.fields.iter().find(|(n, _)| n == fname).ok_or_else(|| {
                        Diagnostic::new(*span, format!("`{name}` 沒有欄位 `{fname}`"))
                    })?;
                    let want = self.resolve_type_expr(&fty.1)?;
                    if want.is_ref() {
                        return Err(Diagnostic::new(
                            *span,
                            "結構不能存放引用。Russd 沒有生命週期標註，請改存擁有值或之後用 Share",
                        ));
                    }
                    let got = self.check_expr(fexpr, Some(&want))?;
                    self.infer.unify(&want, &got, fexpr.span())?;
                }
                Type::Named(name.clone())
            }
            Expr::EnumLit {
                enum_name,
                variant,
                args,
                span,
            } => self.check_enum_lit(enum_name, variant, args, *span, expected)?,
            Expr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                self.check_expr(cond, Some(&Type::Bool))?;
                let t1 = self.check_expr(then_branch, expected)?;
                if let Some(e) = else_branch {
                    let t2 = self.check_expr(e, expected)?;
                    self.infer.unify(&t1, &t2, *span)?
                } else {
                    self.infer.unify(&t1, &Type::Unit, *span)?
                }
            }
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => self.check_match(scrutinee, arms, *span, expected)?,
            Expr::Block { stmts, .. } => self.check_block(stmts, expected)?,
            Expr::Try { expr, span } => {
                let t = self.check_expr(expr, None)?;
                match self.infer.resolve(&t) {
                    Type::Result(ok, err) => {
                        let ret = self.infer.resolve(&self.ret);
                        match ret {
                            Type::Result(_, e2) => {
                                self.infer.unify(&err, &e2, *span)?;
                            }
                            _ => {
                                return Err(Diagnostic::new(
                                    *span,
                                    "`?` 只能用在回傳 `Result` 的函式裡",
                                ));
                            }
                        }
                        *ok
                    }
                    Type::Option(inner) => {
                        let ret = self.infer.resolve(&self.ret);
                        match ret {
                            Type::Option(_) => {}
                            _ => {
                                return Err(Diagnostic::new(
                                    *span,
                                    "`?` 用在 `Option` 上時，函式也要回傳 `Option`",
                                ));
                            }
                        }
                        *inner
                    }
                    other => {
                        return Err(Diagnostic::new(
                            *span,
                            format!(
                                "`?` 只能用在 `Result` 或 `Option`，不是 `{}`",
                                other.display()
                            ),
                        ));
                    }
                }
            }
            Expr::Range { start, end, span } => {
                let a = self.check_expr(start, Some(&Type::I32))?;
                let b = self.check_expr(end, Some(&Type::I32))?;
                self.infer.unify(&a, &Type::I32, *span)?;
                self.infer.unify(&b, &Type::I32, *span)?;
                Type::Range
            }
        };
        let ty = self.infer.resolve(&ty);
        if let Some(exp) = expected {
            self.infer.unify(&ty, exp, expr.span())
        } else {
            Ok(ty)
        }
    }

    fn check_binary(
        &mut self,
        op: BinOp,
        left: &Expr,
        right: &Expr,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let l = self.check_expr(left, None)?;
        let r = self.check_expr(right, None)?;
        let l = l.peel_ref().clone();
        let r = r.peel_ref().clone();
        match op {
            BinOp::And | BinOp::Or => {
                self.infer.unify(&l, &Type::Bool, span)?;
                self.infer.unify(&r, &Type::Bool, span)?;
                Ok(Type::Bool)
            }
            BinOp::Eq | BinOp::Ne => {
                self.infer.unify(&l, &r, span)?;
                Ok(Type::Bool)
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                match l {
                    Type::I32 | Type::I64 | Type::F64 => {}
                    _ => {
                        return Err(Diagnostic::new(span, "比較運算只能用在數字"));
                    }
                }
                self.infer.unify(&l, &r, span)?;
                Ok(Type::Bool)
            }
            BinOp::Add => {
                if matches!(l, Type::Str) || matches!(r, Type::Str) {
                    return Ok(Type::Str);
                }
                self.infer.unify(&l, &r, span)?;
                match self.infer.resolve(&l) {
                    Type::I32 | Type::I64 | Type::F64 => Ok(self.infer.resolve(&l)),
                    other => Err(Diagnostic::new(
                        span,
                        format!("不能對 `{}` 做加法", other.display()),
                    )),
                }
            }
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                self.infer.unify(&l, &r, span)?;
                match self.infer.resolve(&l) {
                    Type::I32 | Type::I64 | Type::F64 => Ok(self.infer.resolve(&l)),
                    other => Err(Diagnostic::new(
                        span,
                        format!("算術運算不能用在 `{}`", other.display()),
                    )),
                }
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                self.infer.unify(&l, &r, span)?;
                match self.infer.resolve(&l) {
                    Type::I32 | Type::I64 => Ok(self.infer.resolve(&l)),
                    other => Err(Diagnostic::new(
                        span,
                        format!("位元運算不能用在 `{}`", other.display()),
                    )),
                }
            }
        }
    }

    fn check_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
        expected: Option<&Type>,
    ) -> Result<Type, Diagnostic> {
        if let Expr::Var { name, .. } = callee {
            if name == "print" {
                for a in args {
                    self.check_expr(a, None)?;
                }
                return Ok(Type::Unit);
            }
            if name == "vec" {
                let inner = match expected {
                    Some(Type::Vec(t)) => (**t).clone(),
                    _ => self.infer.fresh(),
                };
                if !args.is_empty() {
                    for a in args {
                        let t = self.check_expr(a, Some(&inner))?;
                        self.infer.unify(&inner, &t, a.span())?;
                    }
                }
                return Ok(Type::Vec(Box::new(inner)));
            }
            if name == "map" {
                if !args.is_empty() {
                    return Err(Diagnostic::new(span, "`map()` 不接受參數，請再用 `.set`"));
                }
                let inner = match expected {
                    Some(Type::Map(t)) => (**t).clone(),
                    _ => self.infer.fresh(),
                };
                return Ok(Type::Map(Box::new(inner)));
            }
            if name == "args" {
                if !args.is_empty() {
                    return Err(Diagnostic::new(span, "`args()` 不接受參數"));
                }
                return Ok(Type::Vec(Box::new(Type::Str)));
            }
            if name == "read_file" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`read_file` 需要路徑字串"));
                }
                self.check_expr(&args[0], Some(&Type::Str))?;
                return Ok(Type::Result(Box::new(Type::Str), Box::new(Type::Str)));
            }
            if name == "write_file" {
                if args.len() != 2 {
                    return Err(Diagnostic::new(span, "`write_file(path, content)`"));
                }
                self.check_expr(&args[0], Some(&Type::Str))?;
                self.check_expr(&args[1], Some(&Type::Str))?;
                return Ok(Type::Result(Box::new(Type::Unit), Box::new(Type::Str)));
            }
            if name == "assert" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`assert` 需要一個 bool"));
                }
                self.check_expr(&args[0], Some(&Type::Bool))?;
                return Ok(Type::Unit);
            }
            if name == "panic" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`panic` 需要一個字串"));
                }
                self.check_expr(&args[0], Some(&Type::Str))?;
                return Ok(Type::Never);
            }
            if name == "abs" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`abs` 需要一個數字"));
                }
                let t = self.check_expr(&args[0], None)?;
                match t.peel_ref() {
                    Type::I32 | Type::I64 | Type::F64 => return Ok(t.peel_ref().clone()),
                    other => {
                        return Err(Diagnostic::new(
                            span,
                            format!("`abs` 不能用在 `{}`", other.display()),
                        ))
                    }
                }
            }
            if name == "min" || name == "max" {
                if args.len() != 2 {
                    return Err(Diagnostic::new(span, format!("`{name}` 需要兩個數字")));
                }
                let a = self.check_expr(&args[0], None)?;
                let b = self.check_expr(&args[1], Some(&a))?;
                self.infer.unify(&a, &b, span)?;
                match self.infer.resolve(&a) {
                    Type::I32 | Type::I64 | Type::F64 => return Ok(self.infer.resolve(&a)),
                    other => {
                        return Err(Diagnostic::new(
                            span,
                            format!("`{name}` 不能用在 `{}`", other.display()),
                        ))
                    }
                }
            }
            if name == "sqrt" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`sqrt` 需要一個數字"));
                }
                let t = self.check_expr(&args[0], None)?;
                match t.peel_ref() {
                    Type::F64 | Type::I32 | Type::I64 => return Ok(Type::F64),
                    other => {
                        return Err(Diagnostic::new(
                            span,
                            format!("`sqrt` 不能用在 `{}`", other.display()),
                        ))
                    }
                }
            }
            if name == "now_ms" {
                if !args.is_empty() {
                    return Err(Diagnostic::new(span, "`now_ms()` 不接受參數"));
                }
                return Ok(Type::I64);
            }
            if name == "sleep_ms" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`sleep_ms` 需要毫秒數"));
                }
                self.check_expr(&args[0], None)?;
                return Ok(Type::Unit);
            }
            if name == "http_get" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`http_get` 需要 URL"));
                }
                self.check_expr(&args[0], Some(&Type::Str))?;
                return Ok(Type::Result(Box::new(Type::Str), Box::new(Type::Str)));
            }
            if name == "url_decode" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`url_decode` 需要一個字串"));
                }
                self.check_expr(&args[0], Some(&Type::Str))?;
                return Ok(Type::Str);
            }
            if name == "http_serve" {
                if args.len() != 2 && args.len() != 3 {
                    return Err(Diagnostic::new(
                        span,
                        "`http_serve(port, handler)` 或 `http_serve(port, handler, max_requests)`",
                    ));
                }
                self.check_expr(&args[0], Some(&Type::I32))?;
                self.check_expr(&args[1], Some(&Type::Str))?;
                if args.len() == 3 {
                    self.check_expr(&args[2], Some(&Type::I32))?;
                }
                return Ok(Type::Result(Box::new(Type::Str), Box::new(Type::Str)));
            }
            if name == "spawn" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`spawn` 需要函式名稱字串"));
                }
                self.check_expr(&args[0], Some(&Type::Str))?;
                return Ok(Type::I32);
            }
            if name == "join" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`join` 需要 thread id"));
                }
                self.check_expr(&args[0], Some(&Type::I32))?;
                return Ok(Type::Unit);
            }
            if name == "extern_call" {
                if args.is_empty() {
                    return Err(Diagnostic::new(span, "`extern_call(name, ...)`"));
                }
                self.check_expr(&args[0], Some(&Type::Str))?;
                for a in args.iter().skip(1) {
                    self.check_expr(a, None)?;
                }
                return Ok(expected.cloned().unwrap_or(Type::I32));
            }
            if let Some(f) = self.functions.get(name).cloned() {
                return self.check_fn_call(&f, args, span, None);
            }
        }
        if let Expr::Field { base, name, .. } = callee {
            if name == "clone" {
                if !args.is_empty() {
                    return Err(Diagnostic::new(span, "`clone` 不接受參數"));
                }
                return Ok(self.peek_place_ty(base)?.peel_ref().clone());
            }
            if name == "len" {
                if !args.is_empty() {
                    return Err(Diagnostic::new(span, "`len` 不接受參數"));
                }
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Vec(_) | Type::Str | Type::Map(_) => return Ok(Type::I32),
                    other => {
                        return Err(Diagnostic::new(
                            span,
                            format!(
                                "`len` 只能用在 `Vec`、`Map` 或 `str`，不是 `{}`",
                                other.display()
                            ),
                        ));
                    }
                }
            }
            if name == "push" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`push` 需要一個參數"));
                }
                self.borrow_receiver(base, true, span)?;
                let recv = self.peek_place_ty(base)?;
                match recv.peel_ref() {
                    Type::Vec(inner) => {
                        let inner = (**inner).clone();
                        self.check_expr(&args[0], Some(&inner))?;
                        return Ok(Type::Unit);
                    }
                    _ => return Err(Diagnostic::new(span, "`push` 只能用在 `Vec`")),
                }
            }
            if name == "pop" {
                if !args.is_empty() {
                    return Err(Diagnostic::new(span, "`pop` 不接受參數"));
                }
                self.borrow_receiver(base, true, span)?;
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Vec(inner) => return Ok(Type::Option(Box::new((**inner).clone()))),
                    _ => return Err(Diagnostic::new(span, "`pop` 只能用在 `Vec`")),
                }
            }
            if name == "clear" {
                self.borrow_receiver(base, true, span)?;
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Vec(_) | Type::Map(_) => return Ok(Type::Unit),
                    _ => return Err(Diagnostic::new(span, "`clear` 只能用在 `Vec` 或 `Map`")),
                }
            }
            if name == "reverse" {
                self.borrow_receiver(base, true, span)?;
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Vec(_) => return Ok(Type::Unit),
                    _ => return Err(Diagnostic::new(span, "`reverse` 只能用在 `Vec`")),
                }
            }
            if name == "is_empty" {
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Vec(_) | Type::Str | Type::Map(_) => return Ok(Type::Bool),
                    _ => {
                        return Err(Diagnostic::new(
                            span,
                            "`is_empty` 只能用在 `Vec`/`Map`/`str`",
                        ))
                    }
                }
            }
            if name == "contains" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`contains` 需要一個參數"));
                }
                let recv = self.peek_place_ty(base)?.peel_ref().clone();
                match recv {
                    Type::Vec(inner) => {
                        self.check_expr(&args[0], Some(&inner))?;
                        return Ok(Type::Bool);
                    }
                    Type::Str => {
                        self.check_expr(&args[0], Some(&Type::Str))?;
                        return Ok(Type::Bool);
                    }
                    Type::Map(_) => {
                        self.check_expr(&args[0], Some(&Type::Str))?;
                        return Ok(Type::Bool);
                    }
                    _ => return Err(Diagnostic::new(span, "`contains` 類型不支援")),
                }
            }
            if name == "set" {
                if args.len() != 2 {
                    return Err(Diagnostic::new(span, "`set(key, value)`"));
                }
                self.borrow_receiver(base, true, span)?;
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Map(inner) => {
                        let inner = (**inner).clone();
                        self.check_expr(&args[0], Some(&Type::Str))?;
                        self.check_expr(&args[1], Some(&inner))?;
                        return Ok(Type::Unit);
                    }
                    _ => return Err(Diagnostic::new(span, "`set` 只能用在 `Map`")),
                }
            }
            if name == "get" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`get` 需要鍵"));
                }
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Map(inner) => {
                        self.check_expr(&args[0], Some(&Type::Str))?;
                        return Ok(Type::Option(Box::new((**inner).clone())));
                    }
                    _ => return Err(Diagnostic::new(span, "`get` 只能用在 `Map`")),
                }
            }
            if name == "keys" {
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Map(_) => return Ok(Type::Vec(Box::new(Type::Str))),
                    _ => return Err(Diagnostic::new(span, "`keys` 只能用在 `Map`")),
                }
            }
            if name == "split" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`split` 需要分隔字串"));
                }
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Str => {
                        self.check_expr(&args[0], Some(&Type::Str))?;
                        return Ok(Type::Vec(Box::new(Type::Str)));
                    }
                    _ => return Err(Diagnostic::new(span, "`split` 只能用在 `str`")),
                }
            }
            if name == "replace" {
                if args.len() != 2 {
                    return Err(Diagnostic::new(span, "`replace(from, to)`"));
                }
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Str => {
                        self.check_expr(&args[0], Some(&Type::Str))?;
                        self.check_expr(&args[1], Some(&Type::Str))?;
                        return Ok(Type::Str);
                    }
                    _ => return Err(Diagnostic::new(span, "`replace` 只能用在 `str`")),
                }
            }
            if name == "trim" || name == "to_upper" || name == "to_lower" {
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Str => return Ok(Type::Str),
                    _ => return Err(Diagnostic::new(span, format!("`{name}` 只能用在 `str`"))),
                }
            }
            if name == "starts_with" || name == "ends_with" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, format!("`{name}` 需要一個字串")));
                }
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Str => {
                        self.check_expr(&args[0], Some(&Type::Str))?;
                        return Ok(Type::Bool);
                    }
                    _ => return Err(Diagnostic::new(span, format!("`{name}` 只能用在 `str`"))),
                }
            }
            if name == "parse_i32" {
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Str => return Ok(Type::Result(Box::new(Type::I32), Box::new(Type::Str))),
                    _ => return Err(Diagnostic::new(span, "`parse_i32` 只能用在 `str`")),
                }
            }
            if name == "join" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`join` 需要分隔字串"));
                }
                match self.peek_place_ty(base)?.peel_ref() {
                    Type::Vec(inner) if **inner == Type::Str => {
                        self.check_expr(&args[0], Some(&Type::Str))?;
                        return Ok(Type::Str);
                    }
                    _ => return Err(Diagnostic::new(span, "`join` 只能用在 `Vec[str]`")),
                }
            }
            let recv = self.peek_place_ty(base)?;
            let recv_inner = recv.peel_ref().clone();
            let type_name = match &recv_inner {
                Type::Named(n) => n.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        span,
                        format!("類型 `{}` 沒有方法 `{name}`", recv_inner.display()),
                    ))
                }
            };
            if let Some(methods) = self.methods.get(&type_name) {
                if let Some(m) = methods.get(name).cloned() {
                    if let Some(first) = m.params.first() {
                        match first.kind {
                            ParamKind::SelfValue => {
                                let _ = self.check_expr(base, None)?;
                            }
                            ParamKind::SelfRef => self.borrow_receiver(base, false, span)?,
                            ParamKind::SelfRefMut => self.borrow_receiver(base, true, span)?,
                            ParamKind::Value => {}
                        }
                    }
                    return self.check_fn_call(&m, args, span, Some(&recv));
                }
            }
            return Err(Diagnostic::new(
                span,
                format!("`{type_name}` 沒有方法 `{name}`"),
            ));
        }
        Err(Diagnostic::new(span, "無法呼叫這個表達式"))
    }

    fn check_fn_call(
        &mut self,
        f: &FnDef,
        args: &[Expr],
        span: Span,
        receiver: Option<&Type>,
    ) -> Result<Type, Diagnostic> {
        let mut params = f.params.clone();
        if receiver.is_some() && params.first().map(|p| p.name == "self").unwrap_or(false) {
            params.remove(0);
        }
        if params.len() != args.len() {
            return Err(Diagnostic::new(
                span,
                format!(
                    "`{}` 需要 {} 個參數，實際給了 {}",
                    f.name,
                    params.len(),
                    args.len()
                ),
            ));
        }
        for (p, a) in params.iter().zip(args) {
            let want = self.resolve_type_expr(&p.ty)?;
            let got = self.check_expr(a, Some(&want))?;
            self.infer.unify(&want, &got, a.span())?;
        }
        self.resolve_type_expr(&f.ret)
    }

    fn check_enum_lit(
        &mut self,
        enum_name: &Option<String>,
        variant: &str,
        args: &[Expr],
        span: Span,
        expected: Option<&Type>,
    ) -> Result<Type, Diagnostic> {
        match variant {
            "Some" => {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`Some` 需要一個值"));
                }
                let inner = match expected {
                    Some(Type::Option(t)) => (**t).clone(),
                    _ => self.infer.fresh(),
                };
                let g = self.check_expr(&args[0], Some(&inner))?;
                Ok(Type::Option(Box::new(self.infer.unify(&inner, &g, span)?)))
            }
            "None" => {
                let inner = match expected {
                    Some(Type::Option(t)) => (**t).clone(),
                    _ => self.infer.fresh(),
                };
                Ok(Type::Option(Box::new(inner)))
            }
            "Ok" => {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`Ok` 需要一個值"));
                }
                let (ok, err) = match expected {
                    Some(Type::Result(o, e)) => ((**o).clone(), (**e).clone()),
                    _ => (self.infer.fresh(), self.infer.fresh()),
                };
                let g = self.check_expr(&args[0], Some(&ok))?;
                Ok(Type::Result(
                    Box::new(self.infer.unify(&ok, &g, span)?),
                    Box::new(err),
                ))
            }
            "Err" => {
                if args.len() != 1 {
                    return Err(Diagnostic::new(span, "`Err` 需要一個值"));
                }
                let (ok, err) = match expected {
                    Some(Type::Result(o, e)) => ((**o).clone(), (**e).clone()),
                    _ => (self.infer.fresh(), self.infer.fresh()),
                };
                let g = self.check_expr(&args[0], Some(&err))?;
                Ok(Type::Result(
                    Box::new(ok),
                    Box::new(self.infer.unify(&err, &g, span)?),
                ))
            }
            _ => {
                let ename = enum_name.clone().or_else(|| {
                    expected.and_then(|t| match t {
                        Type::Named(n) if self.enums.contains_key(n) => Some(n.clone()),
                        _ => None,
                    })
                });
                let Some(ename) = ename else {
                    return Err(Diagnostic::new(
                        span,
                        format!("未知變體 `{variant}`，請寫成 `Enum::{variant}`"),
                    ));
                };
                let en = self
                    .enums
                    .get(&ename)
                    .ok_or_else(|| Diagnostic::new(span, format!("未知列舉 `{ename}`")))?;
                let v = en
                    .variants
                    .iter()
                    .find(|v| v.name == variant)
                    .ok_or_else(|| {
                        Diagnostic::new(span, format!("`{ename}` 沒有變體 `{variant}`"))
                    })?;
                if v.fields.len() != args.len() {
                    return Err(Diagnostic::new(
                        span,
                        format!("`{ename}::{variant}` 參數數量不對"),
                    ));
                }
                for (ft, a) in v.fields.iter().zip(args) {
                    let want = self.resolve_type_expr(ft)?;
                    let got = self.check_expr(a, Some(&want))?;
                    self.infer.unify(&want, &got, a.span())?;
                }
                Ok(Type::Named(ename))
            }
        }
    }

    fn check_match(
        &mut self,
        scrutinee: &Expr,
        arms: &[MatchArm],
        span: Span,
        expected: Option<&Type>,
    ) -> Result<Type, Diagnostic> {
        let st = self.check_expr(scrutinee, None)?;
        if arms.is_empty() {
            return Err(Diagnostic::new(span, "`match` 至少要有一個分支"));
        }
        let mut result: Option<Type> = expected.cloned();
        for arm in arms {
            self.push();
            self.bind_pat(&arm.pat, &st)?;
            let t = self.check_expr(&arm.body, expected)?;
            self.pop();
            result = Some(match result {
                None => t,
                Some(prev) => self.infer.unify(&prev, &t, arm.body.span())?,
            });
        }
        Ok(result.unwrap_or(Type::Never))
    }

    fn bind_pat(&mut self, pat: &Pat, ty: &Type) -> Result<(), Diagnostic> {
        let ty = self.infer.resolve(ty);
        match pat {
            Pat::Wildcard | Pat::Int(_) | Pat::Bool(_) | Pat::Str(_) => Ok(()),
            Pat::Binding(name) => {
                self.define(
                    name.clone(),
                    Binding {
                        ty,
                        mutable: false,
                        moved: false,
                        shared: 0,
                        unique: false,
                        span: Span::dummy(),
                    },
                );
                Ok(())
            }
            Pat::Variant { name, args, .. } => match (name.as_str(), &ty) {
                ("Some", Type::Option(inner)) => {
                    if args.len() != 1 {
                        return Err(Diagnostic::new(Span::dummy(), "`Some` 模式需要一個綁定"));
                    }
                    self.bind_pat(&args[0], inner)
                }
                ("None", Type::Option(_)) => Ok(()),
                ("Ok", Type::Result(ok, _)) => {
                    if args.len() != 1 {
                        return Err(Diagnostic::new(Span::dummy(), "`Ok` 模式需要一個綁定"));
                    }
                    self.bind_pat(&args[0], ok)
                }
                ("Err", Type::Result(_, err)) => {
                    if args.len() != 1 {
                        return Err(Diagnostic::new(Span::dummy(), "`Err` 模式需要一個綁定"));
                    }
                    self.bind_pat(&args[0], err)
                }
                (_, Type::Named(en)) => {
                    if let Some(ed) = self.enums.get(en) {
                        if let Some(v) = ed.variants.iter().find(|v| &v.name == name) {
                            if v.fields.len() != args.len() {
                                return Err(Diagnostic::new(Span::dummy(), "模式參數數量不對"));
                            }
                            for (ft, p) in v.fields.iter().zip(args) {
                                let want = self.resolve_type_expr(ft)?;
                                self.bind_pat(p, &want)?;
                            }
                            return Ok(());
                        }
                    }
                    Err(Diagnostic::new(
                        Span::dummy(),
                        format!("模式 `{name}` 與類型 `{}` 不符", ty.display()),
                    ))
                }
                _ => Err(Diagnostic::new(
                    Span::dummy(),
                    format!("模式 `{name}` 與類型 `{}` 不符", ty.display()),
                )),
            },
        }
    }

    fn check_block(&mut self, stmts: &[Stmt], expected: Option<&Type>) -> Result<Type, Diagnostic> {
        self.push();
        let mut last = Type::Unit;
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i + 1 == stmts.len();
            last = self.check_stmt(stmt, if is_last { expected } else { None })?;
            // end of statement: drop temporary borrows by resetting unique/shared on copies?
            // Simple model: borrows last until end of the full function. Too conservative but safe.
            let _ = is_last;
        }
        self.pop();
        // Dropping the scope ends borrows from inner lets; outer vars' unique flags may linger.
        // Reset unique/shared at the end of each statement for a more usable checker.
        Ok(last)
    }

    fn check_stmt(&mut self, stmt: &Stmt, expected: Option<&Type>) -> Result<Type, Diagnostic> {
        // Release statement-level borrows after each non-let use: reset unique/shared
        // on all bindings at the start of the next stmt except those still in nested expr.
        self.clear_borrows();
        match stmt {
            Stmt::Let {
                name,
                mutable,
                ty,
                init,
                span,
            } => {
                let want = if let Some(t) = ty {
                    Some(self.resolve_type_expr(t)?)
                } else {
                    expected.cloned()
                };
                if let Some(Type::Ref(_, _)) = &want {
                    return Err(Diagnostic::new(
                        *span,
                        "不能把引用存進 `let` 綁定以外的長期位置；區域借用請在同一表達式內使用",
                    ));
                }
                let got = self.check_expr(init, want.as_ref())?;
                let final_ty = if let Some(w) = want {
                    self.infer.unify(&w, &got, *span)?
                } else {
                    got
                };
                if final_ty.is_ref() {
                    return Err(Diagnostic::new(
                        *span,
                        "不能把借用存進變數。Russd 沒有生命週期，`&` / `&mut` 只能當下使用",
                    )
                    .hint("改成在呼叫時寫 `foo(&x)`，或 clone 成擁有值"));
                }
                self.define(
                    name.clone(),
                    Binding {
                        ty: final_ty,
                        mutable: *mutable,
                        moved: false,
                        shared: 0,
                        unique: false,
                        span: *span,
                    },
                );
                Ok(Type::Unit)
            }
            Stmt::Return { expr, span } => {
                let t = if let Some(e) = expr {
                    self.check_expr(e, Some(&self.ret.clone()))?
                } else {
                    Type::Unit
                };
                self.infer.unify(&t, &self.ret.clone(), *span)?;
                Ok(Type::Never)
            }
            Stmt::While { cond, body, span } => {
                self.check_expr(cond, Some(&Type::Bool))?;
                self.in_loop += 1;
                let bt = self.check_expr(body, Some(&Type::Unit))?;
                self.in_loop -= 1;
                self.infer.unify(&bt, &Type::Unit, *span)?;
                Ok(Type::Unit)
            }
            Stmt::Loop { body, span } => {
                self.in_loop += 1;
                let bt = self.check_expr(body, Some(&Type::Unit))?;
                self.in_loop -= 1;
                self.infer.unify(&bt, &Type::Unit, *span)?;
                Ok(Type::Unit)
            }
            Stmt::Break { span } => {
                if self.in_loop == 0 {
                    return Err(Diagnostic::new(*span, "`break` 只能寫在迴圈裡"));
                }
                Ok(Type::Never)
            }
            Stmt::Continue { span } => {
                if self.in_loop == 0 {
                    return Err(Diagnostic::new(*span, "`continue` 只能寫在迴圈裡"));
                }
                Ok(Type::Never)
            }
            Stmt::For {
                name,
                iter,
                body,
                span,
            } => {
                let it = self.check_expr(iter, None)?;
                let elem = match it.peel_ref() {
                    Type::Vec(inner) => (**inner).clone(),
                    Type::Range => Type::I32,
                    other => {
                        return Err(Diagnostic::new(
                            *span,
                            format!("`for` 只能遍歷 `Vec` 或範圍，不是 `{}`", other.display()),
                        ));
                    }
                };
                self.push();
                self.define(
                    name.clone(),
                    Binding {
                        ty: elem,
                        mutable: false,
                        moved: false,
                        shared: 0,
                        unique: false,
                        span: *span,
                    },
                );
                self.in_loop += 1;
                self.check_expr(body, Some(&Type::Unit))?;
                self.in_loop -= 1;
                self.pop();
                Ok(Type::Unit)
            }
            Stmt::Expr(e) => self.check_expr(e, expected),
        }
    }

    fn clear_borrows(&mut self) {
        for s in &mut self.scopes {
            for b in s.values_mut() {
                b.shared = 0;
                b.unique = false;
            }
        }
    }

    fn check_fn(&mut self, f: &FnDef) -> Result<(), Diagnostic> {
        self.ret = self.resolve_type_expr(&f.ret)?;
        if self.ret.is_ref() {
            return Err(Diagnostic::new(
                f.span,
                "函式不能回傳引用。Russd 沒有生命週期語法，請回傳擁有的值",
            )
            .hint("把資料 clone 出去，或改傳擁有型別"));
        }
        self.scopes.clear();
        self.push();
        let impl_ty = self.impl_ty.clone();
        for p in &f.params {
            let mut ty = self.resolve_type_expr(&p.ty)?;
            if p.name == "self" {
                if let Some(t) = &impl_ty {
                    ty = match p.kind {
                        ParamKind::SelfRef => Type::Ref(Box::new(Type::Named(t.clone())), false),
                        ParamKind::SelfRefMut => Type::Ref(Box::new(Type::Named(t.clone())), true),
                        _ => Type::Named(t.clone()),
                    };
                }
            }
            if let Type::Ref(_, _) = &ty {
                // params may be refs (local to the call)
            }
            self.define(
                p.name.clone(),
                Binding {
                    ty,
                    mutable: p.is_mut || p.kind == ParamKind::SelfRefMut,
                    moved: false,
                    shared: 0,
                    unique: false,
                    span: p.span,
                },
            );
        }
        let body_ty = self.check_expr(&f.body, Some(&self.ret.clone()))?;
        self.infer
            .unify(&body_ty, &self.ret.clone(), f.body.span())?;
        Ok(())
    }
}

fn type_from_struct_field(te: &TypeExpr) -> Result<Type, Diagnostic> {
    // Used only for field_type without infer holes for named builtins
    match te {
        TypeExpr::Unit(_) => Ok(Type::Unit),
        TypeExpr::Ref { span, .. } => Err(Diagnostic::new(*span, "結構欄位不能是引用類型")),
        TypeExpr::Named(name, args, span) => match name.as_str() {
            "i32" => Ok(Type::I32),
            "i64" => Ok(Type::I64),
            "f64" => Ok(Type::F64),
            "bool" => Ok(Type::Bool),
            "str" => Ok(Type::Str),
            "Map" if args.len() == 1 => Ok(Type::Map(Box::new(type_from_struct_field(&args[0])?))),
            "Vec" if args.len() == 1 => Ok(Type::Vec(Box::new(type_from_struct_field(&args[0])?))),
            "Option" if args.len() == 1 => {
                Ok(Type::Option(Box::new(type_from_struct_field(&args[0])?)))
            }
            "Result" if args.len() == 2 => Ok(Type::Result(
                Box::new(type_from_struct_field(&args[0])?),
                Box::new(type_from_struct_field(&args[1])?),
            )),
            n if args.is_empty() => Ok(Type::Named(n.to_string())),
            _ => Err(Diagnostic::new(*span, "無法解析欄位類型")),
        },
    }
}

pub fn check(program: Program) -> Result<CheckedProgram, Diagnostic> {
    let mut structs = HashMap::new();
    let mut enums = HashMap::new();
    let mut functions = HashMap::new();
    let mut methods: HashMap<String, HashMap<String, FnDef>> = HashMap::new();

    for item in &program.items {
        match item {
            Item::Struct(s) => {
                for (_, ty) in &s.fields {
                    if matches!(ty, TypeExpr::Ref { .. }) {
                        return Err(Diagnostic::new(
                            ty.span(),
                            "結構不能存放引用。Russd 刻意沒有生命週期標註",
                        )
                        .hint("改存擁有的值，例如 `str` 而不是 `&str`"));
                    }
                }
                if structs.insert(s.name.clone(), s.clone()).is_some() {
                    return Err(Diagnostic::new(
                        s.span,
                        format!("重複定義結構 `{}`", s.name),
                    ));
                }
            }
            Item::Enum(e) => {
                if enums.insert(e.name.clone(), e.clone()).is_some() {
                    return Err(Diagnostic::new(
                        e.span,
                        format!("重複定義列舉 `{}`", e.name),
                    ));
                }
            }
            Item::Fn(f) => {
                if functions.insert(f.name.clone(), f.clone()).is_some() {
                    return Err(Diagnostic::new(
                        f.span,
                        format!("重複定義函式 `{}`", f.name),
                    ));
                }
            }
            Item::Mod { name, span } => {
                return Err(Diagnostic::new(
                    *span,
                    format!("`mod {name}` 還沒展開。請用 `russd run 檔案.rsd` 從檔案編譯"),
                ));
            }
            Item::Impl(im) => {
                let map = methods.entry(im.ty.clone()).or_default();
                for m in &im.methods {
                    if map.insert(m.name.clone(), m.clone()).is_some() {
                        return Err(Diagnostic::new(
                            m.span,
                            format!("`{}` 重複定義方法 `{}`", im.ty, m.name),
                        ));
                    }
                }
            }
        }
    }

    if !functions.contains_key("main") {
        return Err(
            Diagnostic::new(Span::dummy(), "程式需要 `fn main()` 作為入口")
                .hint("加上 `fn main() { ... }`"),
        );
    }

    let mut ctx = Ctx {
        structs: &structs,
        enums: &enums,
        functions: &functions,
        methods: &methods,
        scopes: Vec::new(),
        infer: Infer::new(),
        ret: Type::Unit,
        impl_ty: None,
        in_loop: 0,
    };

    for item in &program.items {
        match item {
            Item::Fn(f) => {
                ctx.impl_ty = None;
                ctx.check_fn(f)?;
            }
            Item::Impl(im) => {
                if !structs.contains_key(&im.ty) && !enums.contains_key(&im.ty) {
                    return Err(Diagnostic::new(
                        im.span,
                        format!("無法為未知類型 `{}` 寫 `impl`", im.ty),
                    ));
                }
                ctx.impl_ty = Some(im.ty.clone());
                for m in &im.methods {
                    ctx.check_fn(m)?;
                }
            }
            _ => {}
        }
    }

    Ok(CheckedProgram {
        program,
        structs,
        enums,
        functions,
        methods,
    })
}
