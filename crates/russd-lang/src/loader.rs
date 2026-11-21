use crate::ast::{Item, Program};
use crate::check::{check, CheckedProgram};
use crate::diag::Diagnostic;
use crate::lexer::lex;
use crate::parser::parse;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn compile_path(path: &Path) -> Result<CheckedProgram, String> {
    let source =
        std::fs::read_to_string(path).map_err(|e| format!("無法讀取 {}: {e}", path.display()))?;
    match compile_path_inner(path, &source) {
        Ok(c) => Ok(c),
        Err(d) => Err(d.render(&path.display().to_string(), &source)),
    }
}

fn compile_path_inner(path: &Path, source: &str) -> Result<CheckedProgram, Diagnostic> {
    let tokens = lex(source)?;
    let program = parse(&tokens)?;
    let mut seen = HashSet::new();
    seen.insert(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
    let program = expand_mods(program, path, &mut seen)?;
    check(program)
}

fn expand_mods(
    program: Program,
    from: &Path,
    seen: &mut HashSet<PathBuf>,
) -> Result<Program, Diagnostic> {
    let dir = from.parent().unwrap_or(Path::new("."));
    let mut items = Vec::new();
    for item in program.items {
        match item {
            Item::Mod { name, span } => {
                let candidate = dir.join(format!("{name}.rsd"));
                let candidate2 = dir.join(&name).join("mod.rsd");
                let file = if candidate.exists() {
                    candidate
                } else if candidate2.exists() {
                    candidate2
                } else {
                    return Err(Diagnostic::new(
                        span,
                        format!("找不到模組 `{name}`（試過 {}）", candidate.display()),
                    ));
                };
                let canon = file.canonicalize().unwrap_or(file.clone());
                if !seen.insert(canon) {
                    continue;
                }
                let src = std::fs::read_to_string(&file)
                    .map_err(|e| Diagnostic::new(span, format!("讀取模組失敗：{e}")))?;
                let tokens = lex(&src)?;
                let sub = parse(&tokens)?;
                let sub = expand_mods(sub, &file, seen)?;
                items.extend(sub.items);
            }
            other => items.push(other),
        }
    }
    Ok(Program { items })
}
