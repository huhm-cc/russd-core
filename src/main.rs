use russd_lang::{check_source, compile, lex, parse_repl_line, run, ReplInput};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process;

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        if let Err(e) = repl() {
            eprintln!("{e}");
            process::exit(1);
        }
        return;
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "run" => {
            let path = args.first().cloned().unwrap_or_else(usage_exit);
            run_file(&path);
        }
        "check" => {
            let path = args.first().cloned().unwrap_or_else(usage_exit);
            check_file(&path);
        }
        "--help" | "-h" | "help" => print_help(),
        other if other.ends_with(".rsd") => run_file(other),
        _ => {
            usage_exit();
        }
    }
}

fn usage_exit() -> String {
    print_help();
    process::exit(2);
}

fn print_help() {
    println!(
        "\
Russd — 簡化、更安全的 Rust 風格語言（開源，GPL-3.0）

用法:
  russd                 啟動 REPL
  russd run <file.rsd>  執行程式
  russd check <file.rsd>
  russd <file.rsd>
"
    );
}

fn run_file(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("無法讀取 {path}: {e}");
            process::exit(1);
        }
    };
    match compile(&source) {
        Ok(checked) => match run(&checked) {
            Ok(_) => {}
            Err(d) => {
                eprintln!("{}", d.render(path, &source));
                process::exit(1);
            }
        },
        Err(d) => {
            eprintln!("{}", d.render(path, &source));
            process::exit(1);
        }
    }
}

fn check_file(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("無法讀取 {path}: {e}");
            process::exit(1);
        }
    };
    match check_source(path, &source) {
        Ok(()) => println!("{path}: ok"),
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn repl() -> io::Result<()> {
    println!("Russd REPL  ·  輸入 :quit 離開");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        write!(stdout, "rsd> ")?;
        stdout.flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == ":quit" || line == ":exit" {
            break;
        }
        match eval_repl(line) {
            Ok(Some(s)) => println!("{s}"),
            Ok(None) => {}
            Err(e) => eprintln!("{e}"),
        }
    }
    Ok(())
}

fn eval_repl(line: &str) -> Result<Option<String>, String> {
    let tokens = lex(line).map_err(|d| d.render("<repl>", line))?;
    match parse_repl_line(&tokens).map_err(|d| d.render("<repl>", line))? {
        ReplInput::Empty => Ok(None),
        ReplInput::Expr(_) => {
            let wrapped = format!("fn main() {{ print({line}) }}\n");
            let checked = compile(&wrapped).map_err(|d| d.render("<repl>", line))?;
            let (_val, out) = run(&checked).map_err(|d| d.render("<repl>", line))?;
            Ok(Some(out.trim_end().to_string()))
        }
        ReplInput::Stmt(_) | ReplInput::Item(_) => {
            let wrapped = format!("{line}\nfn main() {{}}\n");
            compile(&wrapped).map_err(|d| d.render("<repl>", line))?;
            println!("ok");
            Ok(None)
        }
    }
}
