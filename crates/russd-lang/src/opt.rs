//! Optimizing compiler middle-end: rewrite typed AST, then bytecode peephole.
//! Default pipeline is O2. Semantics stay aligned with the runtime (no wrap-on-overflow).

use crate::ast::*;
use crate::check::CheckedProgram;
use crate::span::Span;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
}

impl OptLevel {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "-O0" | "O0" => Some(Self::O0),
            "-O1" | "O1" => Some(Self::O1),
            "-O2" | "O2" => Some(Self::O2),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OptStats {
    pub folds: u32,
    pub inlines: u32,
    pub dce: u32,
    pub copies: u32,
}

impl OptStats {
    pub fn report(&self, level: OptLevel) -> String {
        let lv = match level {
            OptLevel::O0 => "O0",
            OptLevel::O1 => "O1",
            OptLevel::O2 => "O2",
        };
        format!(
            "russd opt {lv}: const-fold {folds}, inline {inlines}, dce {dce}, copy-prop {copies}",
            folds = self.folds,
            inlines = self.inlines,
            dce = self.dce,
            copies = self.copies
        )
    }
}

pub fn optimize(prog: &mut CheckedProgram, level: OptLevel) -> OptStats {
    if level == OptLevel::O0 {
        return OptStats::default();
    }
    let mut stats = OptStats::default();
    let spawn_keep = collect_spawn_targets(prog);
    for _ in 0..8 {
        let before = stats.clone();
        let fns: Vec<String> = prog.functions.keys().cloned().collect();
        for name in fns {
            if let Some(mut f) = prog.functions.remove(&name) {
                opt_fn(&mut f, prog, level, &mut stats, &spawn_keep);
                prog.functions.insert(name, f);
            }
        }
        let owners: Vec<String> = prog.methods.keys().cloned().collect();
        for owner in owners {
            if let Some(mut map) = prog.methods.remove(&owner) {
                let names: Vec<String> = map.keys().cloned().collect();
                for n in names {
                    if let Some(mut m) = map.remove(&n) {
                        opt_fn(&mut m, prog, level, &mut stats, &spawn_keep);
                        map.insert(n, m);
                    }
                }
                prog.methods.insert(owner, map);
            }
        }
        if stats.folds == before.folds
            && stats.inlines == before.inlines
            && stats.dce == before.dce
            && stats.copies == before.copies
        {
            break;
        }
    }
    stats
}

fn collect_spawn_targets(prog: &CheckedProgram) -> HashSet<String> {
    let mut s = HashSet::new();
    for f in prog.functions.values() {
        walk_spawn(&f.body, &mut s);
    }
    for ms in prog.methods.values() {
        for f in ms.values() {
            walk_spawn(&f.body, &mut s);
        }
    }
    s
}

fn walk_spawn(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::Call { callee, args, .. } => {
            if let Expr::Var { name, .. } = callee.as_ref() {
                if name == "spawn" {
                    if let Some(Expr::Str { value, .. }) = args.first() {
                        out.insert(value.clone());
                    }
                }
            }
            walk_spawn(callee, out);
            for a in args {
                walk_spawn(a, out);
            }
        }
        Expr::Binary { left, right, .. } => {
            walk_spawn(left, out);
            walk_spawn(right, out);
        }
        Expr::Unary { expr, .. } | Expr::Try { expr, .. } | Expr::Field { base: expr, .. } => {
            walk_spawn(expr, out)
        }
        Expr::Index { base, index, .. } => {
            walk_spawn(base, out);
            walk_spawn(index, out);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            walk_spawn(cond, out);
            walk_spawn(then_branch, out);
            if let Some(e) = else_branch {
                walk_spawn(e, out);
            }
        }
        Expr::Match { scrutinee, arms, .. } => {
            walk_spawn(scrutinee, out);
            for a in arms {
                walk_spawn(&a.body, out);
            }
        }
        Expr::Block { stmts, .. } => {
            for st in stmts {
                walk_stmt_spawn(st, out);
            }
        }
        Expr::Assign { place, value, .. } => {
            walk_spawn(place, out);
            walk_spawn(value, out);
        }
        Expr::Range { start, end, .. } => {
            walk_spawn(start, out);
            walk_spawn(end, out);
        }
        Expr::StructLit { fields, .. } => {
            for (_, v) in fields {
                walk_spawn(v, out);
            }
        }
        Expr::EnumLit { args, .. } => {
            for a in args {
                walk_spawn(a, out);
            }
        }
        _ => {}
    }
}

fn walk_stmt_spawn(st: &Stmt, out: &mut HashSet<String>) {
    match st {
        Stmt::Let { init, .. } => walk_spawn(init, out),
        Stmt::Return { expr, .. } => {
            if let Some(e) = expr {
                walk_spawn(e, out);
            }
        }
        Stmt::While { cond, body, .. } | Stmt::For { iter: cond, body, .. } => {
            walk_spawn(cond, out);
            walk_spawn(body, out);
        }
        Stmt::Loop { body, .. } => walk_spawn(body, out),
        Stmt::Expr(e) => walk_spawn(e, out),
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

fn opt_fn(
    f: &mut FnDef,
    prog: &CheckedProgram,
    level: OptLevel,
    stats: &mut OptStats,
    _spawn_keep: &HashSet<String>,
) {
    f.body = opt_expr(&f.body, prog, Some(&f.name), level, stats);
}

fn opt_expr(
    e: &Expr,
    prog: &CheckedProgram,
    current: Option<&str>,
    level: OptLevel,
    stats: &mut OptStats,
) -> Expr {
    match e {
        Expr::Binary {
            op,
            left,
            right,
            span,
        } => {
            let left = opt_expr(left, prog, current, level, stats);
            let right = opt_expr(right, prog, current, level, stats);
            if let Some(folded) = fold_binary(*op, &left, &right, *span) {
                stats.folds += 1;
                return folded;
            }
            if let Some(alg) = algebraic(*op, &left, &right, *span) {
                stats.folds += 1;
                return alg;
            }
            Expr::Binary {
                op: *op,
                left: Box::new(left),
                right: Box::new(right),
                span: *span,
            }
        }
        Expr::Unary { op, expr, span } => {
            let inner = opt_expr(expr, prog, current, level, stats);
            if let Some(f) = fold_unary(*op, &inner, *span) {
                stats.folds += 1;
                return f;
            }
            Expr::Unary {
                op: *op,
                expr: Box::new(inner),
                span: *span,
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            span,
        } => {
            let cond = opt_expr(cond, prog, current, level, stats);
            let then_branch = opt_expr(then_branch, prog, current, level, stats);
            let else_branch = else_branch
                .as_ref()
                .map(|e| opt_expr(e, prog, current, level, stats));
            if let Expr::Bool { value, .. } = cond {
                stats.folds += 1;
                if value {
                    return then_branch;
                }
                return else_branch.unwrap_or(Expr::Unit { span: *span });
            }
            Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: else_branch.map(Box::new),
                span: *span,
            }
        }
        Expr::Call { callee, args, span } => {
            let callee = opt_expr(callee, prog, current, level, stats);
            let args: Vec<Expr> = args
                .iter()
                .map(|a| opt_expr(a, prog, current, level, stats))
                .collect();
            if level == OptLevel::O2 {
                if let Expr::Var { name, .. } = &callee {
                    if current != Some(name.as_str()) {
                        if let Some(f) = prog.functions.get(name) {
                            if let Some(inlined) = try_inline(f, &args, *span) {
                                stats.inlines += 1;
                                return opt_expr(&inlined, prog, current, level, stats);
                            }
                        }
                    }
                }
            }
            Expr::Call {
                callee: Box::new(callee),
                args,
                span: *span,
            }
        }
        Expr::Block { stmts, span } => {
            let stmts: Vec<Stmt> = stmts
                .iter()
                .map(|s| opt_stmt(s, prog, current, level, stats))
                .collect();
            let stmts = if level == OptLevel::O2 {
                dce_block(stmts, stats)
            } else {
                dce_after_return(stmts, stats)
            };
            Expr::Block {
                stmts,
                span: *span,
            }
        }
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let scrutinee = opt_expr(scrutinee, prog, current, level, stats);
            let arms = arms
                .iter()
                .map(|a| MatchArm {
                    pat: a.pat.clone(),
                    body: opt_expr(&a.body, prog, current, level, stats),
                })
                .collect();
            Expr::Match {
                scrutinee: Box::new(scrutinee),
                arms,
                span: *span,
            }
        }
        Expr::Assign { place, value, span } => Expr::Assign {
            place: Box::new(opt_expr(place, prog, current, level, stats)),
            value: Box::new(opt_expr(value, prog, current, level, stats)),
            span: *span,
        },
        Expr::Field { base, name, span } => Expr::Field {
            base: Box::new(opt_expr(base, prog, current, level, stats)),
            name: name.clone(),
            span: *span,
        },
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(opt_expr(base, prog, current, level, stats)),
            index: Box::new(opt_expr(index, prog, current, level, stats)),
            span: *span,
        },
        Expr::Try { expr, span } => Expr::Try {
            expr: Box::new(opt_expr(expr, prog, current, level, stats)),
            span: *span,
        },
        Expr::Range { start, end, span } => Expr::Range {
            start: Box::new(opt_expr(start, prog, current, level, stats)),
            end: Box::new(opt_expr(end, prog, current, level, stats)),
            span: *span,
        },
        Expr::StructLit { name, fields, span } => Expr::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), opt_expr(v, prog, current, level, stats)))
                .collect(),
            span: *span,
        },
        Expr::EnumLit {
            enum_name,
            variant,
            args,
            span,
        } => Expr::EnumLit {
            enum_name: enum_name.clone(),
            variant: variant.clone(),
            args: args
                .iter()
                .map(|a| opt_expr(a, prog, current, level, stats))
                .collect(),
            span: *span,
        },
        other => other.clone(),
    }
}

fn opt_stmt(
    s: &Stmt,
    prog: &CheckedProgram,
    current: Option<&str>,
    level: OptLevel,
    stats: &mut OptStats,
) -> Stmt {
    match s {
        Stmt::Let {
            name,
            mutable,
            ty,
            init,
            span,
        } => Stmt::Let {
            name: name.clone(),
            mutable: *mutable,
            ty: ty.clone(),
            init: opt_expr(init, prog, current, level, stats),
            span: *span,
        },
        Stmt::Return { expr, span } => Stmt::Return {
            expr: expr
                .as_ref()
                .map(|e| opt_expr(e, prog, current, level, stats)),
            span: *span,
        },
        Stmt::While { cond, body, span } => {
            let cond = opt_expr(cond, prog, current, level, stats);
            let body = opt_expr(body, prog, current, level, stats);
            if matches!(cond, Expr::Bool { value: false, .. }) {
                stats.folds += 1;
                return Stmt::Expr(Expr::Unit { span: *span });
            }
            Stmt::While {
                cond,
                body,
                span: *span,
            }
        }
        Stmt::For {
            name,
            iter,
            body,
            span,
        } => Stmt::For {
            name: name.clone(),
            iter: opt_expr(iter, prog, current, level, stats),
            body: opt_expr(body, prog, current, level, stats),
            span: *span,
        },
        Stmt::Loop { body, span } => Stmt::Loop {
            body: opt_expr(body, prog, current, level, stats),
            span: *span,
        },
        Stmt::Expr(e) => Stmt::Expr(opt_expr(e, prog, current, level, stats)),
        other => other.clone(),
    }
}

fn dce_after_return(stmts: Vec<Stmt>, stats: &mut OptStats) -> Vec<Stmt> {
    let mut out = Vec::new();
    for s in stmts {
        let is_ret = matches!(s, Stmt::Return { .. });
        out.push(s);
        if is_ret {
            if !out.is_empty() {
                stats.dce += 1;
            }
            break;
        }
    }
    out
}

fn dce_block(stmts: Vec<Stmt>, stats: &mut OptStats) -> Vec<Stmt> {
    let stmts = dce_after_return(stmts, stats);
    let mut used = HashSet::new();
    for s in &stmts {
        collect_uses_stmt(s, &mut used);
    }
    let mut out = Vec::new();
    for s in stmts {
        if let Stmt::Let { name, init, .. } = &s {
            if !used.contains(name) && !has_side_effect(init) {
                stats.dce += 1;
                continue;
            }
        }
        out.push(s);
    }
    copy_prop_block(out, stats)
}

fn copy_prop_block(stmts: Vec<Stmt>, stats: &mut OptStats) -> Vec<Stmt> {
    let mut consts: HashMap<String, Expr> = HashMap::new();
    let mut assigned = HashSet::new();
    for s in &stmts {
        if let Stmt::Let {
            name,
            mutable: false,
            init,
            ..
        } = s
        {
            if is_const(init) {
                consts.insert(name.clone(), init.clone());
            }
        }
        collect_assigns(s, &mut assigned);
    }
    for n in &assigned {
        consts.remove(n);
    }
    if consts.is_empty() {
        return stmts;
    }
    stmts
        .into_iter()
        .map(|s| subst_stmt(s, &consts, stats))
        .collect()
}

fn collect_assigns(s: &Stmt, out: &mut HashSet<String>) {
    match s {
        Stmt::Expr(Expr::Assign { place, .. }) => {
            if let Expr::Var { name, .. } = place.as_ref() {
                out.insert(name.clone());
            }
        }
        Stmt::While { body, .. } | Stmt::For { body, .. } | Stmt::Loop { body, .. } => {
            if let Expr::Block { stmts, .. } = body {
                for st in stmts {
                    collect_assigns(st, out);
                }
            }
        }
        _ => {}
    }
}

fn subst_stmt(s: Stmt, map: &HashMap<String, Expr>, stats: &mut OptStats) -> Stmt {
    match s {
        Stmt::Let {
            name,
            mutable,
            ty,
            init,
            span,
        } => Stmt::Let {
            name,
            mutable,
            ty,
            init: subst_expr(init, map, stats),
            span,
        },
        Stmt::Return { expr, span } => Stmt::Return {
            expr: expr.map(|e| subst_expr(e, map, stats)),
            span,
        },
        Stmt::While { cond, body, span } => Stmt::While {
            cond: subst_expr(cond, map, stats),
            body: subst_expr(body, map, stats),
            span,
        },
        Stmt::For {
            name,
            iter,
            body,
            span,
        } => Stmt::For {
            name,
            iter: subst_expr(iter, map, stats),
            body: subst_expr(body, map, stats),
            span,
        },
        Stmt::Loop { body, span } => Stmt::Loop {
            body: subst_expr(body, map, stats),
            span,
        },
        Stmt::Expr(e) => Stmt::Expr(subst_expr(e, map, stats)),
        other => other,
    }
}

fn subst_expr(e: Expr, map: &HashMap<String, Expr>, stats: &mut OptStats) -> Expr {
    match e {
        Expr::Var { name, span } => {
            if let Some(c) = map.get(&name) {
                stats.copies += 1;
                let mut v = c.clone();
                set_span(&mut v, span);
                return v;
            }
            Expr::Var { name, span }
        }
        Expr::Binary {
            op,
            left,
            right,
            span,
        } => Expr::Binary {
            op,
            left: Box::new(subst_expr(*left, map, stats)),
            right: Box::new(subst_expr(*right, map, stats)),
            span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op,
            expr: Box::new(subst_expr(*expr, map, stats)),
            span,
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: Box::new(subst_expr(*callee, map, stats)),
            args: args
                .into_iter()
                .map(|a| subst_expr(a, map, stats))
                .collect(),
            span,
        },
        Expr::If {
            cond,
            then_branch,
            else_branch,
            span,
        } => Expr::If {
            cond: Box::new(subst_expr(*cond, map, stats)),
            then_branch: Box::new(subst_expr(*then_branch, map, stats)),
            else_branch: else_branch.map(|e| Box::new(subst_expr(*e, map, stats))),
            span,
        },
        Expr::Block { stmts, span } => Expr::Block {
            stmts: stmts
                .into_iter()
                .map(|s| subst_stmt(s, map, stats))
                .collect(),
            span,
        },
        Expr::Assign { place, value, span } => Expr::Assign {
            place: Box::new(subst_expr(*place, map, stats)),
            value: Box::new(subst_expr(*value, map, stats)),
            span,
        },
        Expr::Field { base, name, span } => Expr::Field {
            base: Box::new(subst_expr(*base, map, stats)),
            name,
            span,
        },
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(subst_expr(*base, map, stats)),
            index: Box::new(subst_expr(*index, map, stats)),
            span,
        },
        other => other,
    }
}

fn set_span(e: &mut Expr, span: Span) {
    match e {
        Expr::Int { span: s, .. }
        | Expr::Float { span: s, .. }
        | Expr::Bool { span: s, .. }
        | Expr::Str { span: s, .. }
        | Expr::Unit { span: s }
        | Expr::Var { span: s, .. } => *s = span,
        _ => {}
    }
}

fn collect_uses_stmt(s: &Stmt, used: &mut HashSet<String>) {
    match s {
        Stmt::Let { init, .. } => collect_uses(init, used),
        Stmt::Return { expr, .. } => {
            if let Some(e) = expr {
                collect_uses(e, used);
            }
        }
        Stmt::While { cond, body, .. } | Stmt::For { iter: cond, body, .. } => {
            collect_uses(cond, used);
            collect_uses(body, used);
        }
        Stmt::Loop { body, .. } => collect_uses(body, used),
        Stmt::Expr(e) => collect_uses(e, used),
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

fn collect_uses(e: &Expr, used: &mut HashSet<String>) {
    match e {
        Expr::Var { name, .. } => {
            used.insert(name.clone());
        }
        Expr::Binary { left, right, .. } => {
            collect_uses(left, used);
            collect_uses(right, used);
        }
        Expr::Unary { expr, .. } | Expr::Try { expr, .. } | Expr::Field { base: expr, .. } => {
            collect_uses(expr, used)
        }
        Expr::Call { callee, args, .. } => {
            collect_uses(callee, used);
            for a in args {
                collect_uses(a, used);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_uses(base, used);
            collect_uses(index, used);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_uses(cond, used);
            collect_uses(then_branch, used);
            if let Some(e) = else_branch {
                collect_uses(e, used);
            }
        }
        Expr::Match { scrutinee, arms, .. } => {
            collect_uses(scrutinee, used);
            for a in arms {
                collect_uses(&a.body, used);
            }
        }
        Expr::Block { stmts, .. } => {
            for s in stmts {
                collect_uses_stmt(s, used);
            }
        }
        Expr::Assign { place, value, .. } => {
            collect_uses(place, used);
            collect_uses(value, used);
        }
        Expr::Range { start, end, .. } => {
            collect_uses(start, used);
            collect_uses(end, used);
        }
        Expr::StructLit { fields, .. } => {
            for (_, v) in fields {
                collect_uses(v, used);
            }
        }
        Expr::EnumLit { args, .. } => {
            for a in args {
                collect_uses(a, used);
            }
        }
        _ => {}
    }
}

fn try_inline(f: &FnDef, args: &[Expr], span: Span) -> Option<Expr> {
    if f.params.len() != args.len() {
        return None;
    }
    if f.params.iter().any(|p| p.name == "self") {
        return None;
    }
    if complexity(&f.body) > 12 {
        return None;
    }
    if contains_loop(&f.body) {
        return None;
    }
    for (p, a) in f.params.iter().zip(args) {
        let uses = count_var(&f.body, &p.name);
        if uses > 1 && has_side_effect(a) {
            return None;
        }
    }
    let mut map = HashMap::new();
    for (p, a) in f.params.iter().zip(args) {
        map.insert(p.name.clone(), a.clone());
    }
    let mut dummy = OptStats::default();
    let mut body = subst_expr(f.body.clone(), &map, &mut dummy);
    set_span(&mut body, span);
    Some(body)
}

fn complexity(e: &Expr) -> u32 {
    match e {
        Expr::Block { stmts, .. } => stmts.iter().map(stmt_complexity).sum::<u32>() + 1,
        Expr::Binary { left, right, .. } => 1 + complexity(left) + complexity(right),
        Expr::Call { args, .. } => 2 + args.iter().map(complexity).sum::<u32>(),
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            1 + complexity(cond)
                + complexity(then_branch)
                + else_branch.as_ref().map(|e| complexity(e)).unwrap_or(0)
        }
        _ => 1,
    }
}

fn stmt_complexity(s: &Stmt) -> u32 {
    match s {
        Stmt::Let { init, .. } => 1 + complexity(init),
        Stmt::Expr(e) | Stmt::Loop { body: e, .. } => complexity(e),
        Stmt::Return { expr, .. } => expr.as_ref().map(complexity).unwrap_or(1),
        Stmt::While { cond, body, .. } | Stmt::For { iter: cond, body, .. } => {
            2 + complexity(cond) + complexity(body)
        }
        _ => 1,
    }
}

fn contains_loop(e: &Expr) -> bool {
    match e {
        Expr::Block { stmts, .. } => stmts.iter().any(|s| match s {
            Stmt::While { .. } | Stmt::For { .. } | Stmt::Loop { .. } => true,
            Stmt::Let { init, .. } => contains_loop(init),
            Stmt::Expr(e) | Stmt::Return { expr: Some(e), .. } => contains_loop(e),
            _ => false,
        }),
        Expr::If {
            then_branch,
            else_branch,
            ..
        } => contains_loop(then_branch) || else_branch.as_ref().is_some_and(|e| contains_loop(e)),
        _ => false,
    }
}

fn count_var(e: &Expr, name: &str) -> u32 {
    let mut set = HashSet::new();
    collect_uses(e, &mut set);
    if set.contains(name) {
        // approximate: walk again counting
        count_var_inner(e, name)
    } else {
        0
    }
}

fn count_var_inner(e: &Expr, name: &str) -> u32 {
    match e {
        Expr::Var { name: n, .. } => u32::from(n == name),
        Expr::Binary { left, right, .. } => count_var_inner(left, name) + count_var_inner(right, name),
        Expr::Unary { expr, .. } | Expr::Field { base: expr, .. } | Expr::Try { expr, .. } => {
            count_var_inner(expr, name)
        }
        Expr::Call { callee, args, .. } => {
            count_var_inner(callee, name) + args.iter().map(|a| count_var_inner(a, name)).sum::<u32>()
        }
        Expr::Block { stmts, .. } => stmts
            .iter()
            .map(|s| match s {
                Stmt::Let { init, .. } => count_var_inner(init, name),
                Stmt::Expr(e) => count_var_inner(e, name),
                Stmt::Return { expr: Some(e), .. } => count_var_inner(e, name),
                _ => 0,
            })
            .sum(),
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            count_var_inner(cond, name)
                + count_var_inner(then_branch, name)
                + else_branch
                    .as_ref()
                    .map(|e| count_var_inner(e, name))
                    .unwrap_or(0)
        }
        _ => 0,
    }
}

fn is_const(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Str { .. }
            | Expr::Unit { .. }
    )
}

fn has_side_effect(e: &Expr) -> bool {
    match e {
        Expr::Call { .. } | Expr::Assign { .. } => true,
        Expr::Binary { left, right, .. } => has_side_effect(left) || has_side_effect(right),
        Expr::Unary { expr, .. } | Expr::Field { base: expr, .. } | Expr::Try { expr, .. } => {
            has_side_effect(expr)
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            has_side_effect(cond)
                || has_side_effect(then_branch)
                || else_branch.as_ref().is_some_and(|e| has_side_effect(e))
        }
        Expr::Block { stmts, .. } => stmts.iter().any(|s| match s {
            Stmt::Expr(e) | Stmt::Let { init: e, .. } => has_side_effect(e),
            Stmt::While { .. } | Stmt::For { .. } | Stmt::Loop { .. } => true,
            _ => false,
        }),
        _ => false,
    }
}

fn fold_binary(op: BinOp, left: &Expr, right: &Expr, span: Span) -> Option<Expr> {
    match (op, left, right) {
        (BinOp::Add, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) => {
            let sum = a.checked_add(*b)?;
            if !*ty_i64 && (sum < i32::MIN as i64 || sum > i32::MAX as i64) {
                return None;
            }
            Some(Expr::Int {
                value: sum,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::Sub, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) => {
            let v = a.checked_sub(*b)?;
            Some(Expr::Int {
                value: v,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::Mul, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) => {
            let v = a.checked_mul(*b)?;
            if !*ty_i64 && (v < i32::MIN as i64 || v > i32::MAX as i64) {
                return None;
            }
            Some(Expr::Int {
                value: v,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::Div, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) if *b != 0 => {
            Some(Expr::Int {
                value: a / b,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::Mod, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) if *b != 0 => {
            Some(Expr::Int {
                value: a % b,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::BitAnd, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Int {
                value: a & b,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::BitOr, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Int {
                value: a | b,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::BitXor, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Int {
                value: a ^ b,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::Shl, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Int {
                value: a.wrapping_shl(*b as u32),
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::Shr, Expr::Int { value: a, ty_i64, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Int {
                value: a.wrapping_shr(*b as u32),
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::Eq, Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Bool { value: a == b, span })
        }
        (BinOp::Ne, Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Bool { value: a != b, span })
        }
        (BinOp::Lt, Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Bool { value: a < b, span })
        }
        (BinOp::Le, Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Bool { value: a <= b, span })
        }
        (BinOp::Gt, Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Bool { value: a > b, span })
        }
        (BinOp::Ge, Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => {
            Some(Expr::Bool { value: a >= b, span })
        }
        (BinOp::Add, Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => {
            Some(Expr::Float { value: a + b, span })
        }
        (BinOp::Sub, Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => {
            Some(Expr::Float { value: a - b, span })
        }
        (BinOp::Mul, Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => {
            Some(Expr::Float { value: a * b, span })
        }
        (BinOp::And, Expr::Bool { value: a, .. }, Expr::Bool { value: b, .. }) => {
            Some(Expr::Bool {
                value: *a && *b,
                span,
            })
        }
        (BinOp::Or, Expr::Bool { value: a, .. }, Expr::Bool { value: b, .. }) => Some(Expr::Bool {
            value: *a || *b,
            span,
        }),
        (BinOp::Add, Expr::Str { value: a, .. }, Expr::Str { value: b, .. }) => Some(Expr::Str {
            value: format!("{a}{b}"),
            span,
        }),
        _ => None,
    }
}

fn algebraic(op: BinOp, left: &Expr, right: &Expr, span: Span) -> Option<Expr> {
    match (op, left, right) {
        (BinOp::Add, e, Expr::Int { value: 0, .. }) | (BinOp::Add, Expr::Int { value: 0, .. }, e) => {
            Some(e.clone())
        }
        (BinOp::Mul, _, Expr::Int { value: 0, ty_i64, .. })
        | (BinOp::Mul, Expr::Int { value: 0, ty_i64, .. }, _)
            if !has_side_effect(left) && !has_side_effect(right) =>
        {
            Some(Expr::Int {
                value: 0,
                ty_i64: *ty_i64,
                span,
            })
        }
        (BinOp::Mul, e, Expr::Int { value: 1, .. }) | (BinOp::Mul, Expr::Int { value: 1, .. }, e) => {
            Some(e.clone())
        }
        (BinOp::And, Expr::Bool { value: true, .. }, e)
        | (BinOp::And, e, Expr::Bool { value: true, .. }) => Some(e.clone()),
        (BinOp::And, Expr::Bool { value: false, .. }, _)
        | (BinOp::And, _, Expr::Bool { value: false, .. })
            if !has_side_effect(left) && !has_side_effect(right) =>
        {
            Some(Expr::Bool { value: false, span })
        }
        (BinOp::Or, Expr::Bool { value: false, .. }, e)
        | (BinOp::Or, e, Expr::Bool { value: false, .. }) => Some(e.clone()),
        (BinOp::Or, Expr::Bool { value: true, .. }, _)
        | (BinOp::Or, _, Expr::Bool { value: true, .. })
            if !has_side_effect(left) && !has_side_effect(right) =>
        {
            Some(Expr::Bool { value: true, span })
        }
        _ => None,
    }
}

fn fold_unary(op: UnOp, inner: &Expr, span: Span) -> Option<Expr> {
    match (op, inner) {
        (UnOp::Not, Expr::Bool { value, .. }) => Some(Expr::Bool {
            value: !*value,
            span,
        }),
        (UnOp::Neg, Expr::Int { value, ty_i64, .. }) => Some(Expr::Int {
            value: -value,
            ty_i64: *ty_i64,
            span,
        }),
        (UnOp::Neg, Expr::Float { value, .. }) => Some(Expr::Float {
            value: -value,
            span,
        }),
        (UnOp::BitNot, Expr::Int { value, ty_i64, .. }) => Some(Expr::Int {
            value: !value,
            ty_i64: *ty_i64,
            span,
        }),
        _ => None,
    }
}
