mod ast;
mod check;
mod diag;
mod interp;
mod lexer;
mod parser;
mod span;
mod token;
mod types;

pub use check::{check, CheckedProgram};
pub use diag::{render_all, Diagnostic};
pub use interp::{run, Value, Vm};
pub use lexer::lex;
pub use parser::{parse_repl_line, ReplInput};

use crate::parser::parse;

pub fn compile(source: &str) -> Result<CheckedProgram, Diagnostic> {
    let tokens = lex(source)?;
    let program = parse(&tokens)?;
    check(program)
}

pub fn compile_without_main(source: &str) -> Result<CheckedProgram, Diagnostic> {
    let tokens = lex(source)?;
    let mut program = parse(&tokens)?;
    if !program
        .items
        .iter()
        .any(|it| matches!(it, ast::Item::Fn(f) if f.name == "main"))
    {
        program.items.push(ast::Item::Fn(ast::FnDef {
            name: "main".into(),
            params: vec![],
            ret: ast::TypeExpr::Unit(span::Span::dummy()),
            body: ast::Expr::Block {
                stmts: vec![],
                span: span::Span::dummy(),
            },
            span: span::Span::dummy(),
        }));
    }
    check(program)
}

pub fn run_source(filename: &str, source: &str) -> Result<String, String> {
    match compile(source) {
        Ok(checked) => match run(&checked) {
            Ok((_val, output)) => Ok(output),
            Err(d) => Err(d.render(filename, source)),
        },
        Err(d) => Err(d.render(filename, source)),
    }
}

pub fn check_source(filename: &str, source: &str) -> Result<(), String> {
    compile(source)
        .map(|_| ())
        .map_err(|d| d.render(filename, source))
}
