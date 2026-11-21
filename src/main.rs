use russd_lang::{
    compile, compile_bytecode, compile_path, compile_path_with_opt, format_source, lex, optimize,
    parse_repl_line, run, run_with_args, OptLevel, ReplInput, Vm,
};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process;

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        if let Err(e) = repl() {
            eprintln!("{e}");
            process::exit(1);
        }
        return;
    }
    let raw = args.remove(0);
    // `cargo run --run file.rsd` glues Cargo's `--` to our `run` subcommand.
    // Accept `--run` / `-run` as `run` so that form still executes.
    let cmd = normalize_cmd(&raw);
    match cmd.as_str() {
        "run" => cmd_run(&args),
        "opt" => cmd_opt(&args),
        "check" => cmd_check(&args),
        "test" => cmd_test(&args),
        "fmt" => cmd_fmt(&args),
        "disasm" | "bytecode" => cmd_disasm(&args),
        "pkg" => cmd_pkg(&args),
        "complete" => cmd_complete(&args),
        "debug" => cmd_debug(&args),
        "help" | "h" => print_help(),
        "version" | "V" => println!("russd 0.3.0"),
        other if other.ends_with(".rsd") || Path::new(other).exists() => cmd_run(
            &std::iter::once(raw)
                .chain(args)
                .collect::<Vec<_>>(),
        ),
        other => {
            eprintln!("error: 找不到指令 `{other}`");
            eprintln!("提示: 用 `russd run <file.rsd>`。若透過 cargo，請寫 `cargo run -- examples/hello.rsd`");
            eprintln!("      （`--` 與 `run` 中間要有空白；不要寫成 `cargo run --run`。）");
            print_help();
            process::exit(2);
        }
    }
}

fn normalize_cmd(raw: &str) -> String {
    let s = raw.trim();
    let s = s.strip_prefix("--").unwrap_or(s);
    let s = s.strip_prefix('-').unwrap_or(s);
    s.to_string()
}

fn print_help() {
    println!(
        "\
Russd 0.3 — 開源、Rust 風格、最佳化編譯器（GPL-3.0）

用法:
  russd                      REPL
  russd run <file.rsd> [-O0|-O1|-O2] [-- args...]
  russd --run <file.rsd>     同上（避免被 cargo 吃掉 `run`）
  cargo run -- <file.rsd>    從原始碼執行（`--` 後直接給檔名）
  russd opt <file.rsd>       顯示最佳化統計與 bytecode
  russd check <file.rsd>
  russd test [file.rsd]      執行 test_ 開頭的函式
  russd fmt <file.rsd>
  russd disasm <file.rsd>    顯示最佳化後 bytecode
  russd debug <file.rsd>     追蹤函式呼叫
  russd complete <file.rsd>  符號清單（編輯器補全）
  russd pkg init|info
  russd help

預設 -O2：常數摺疊、代數簡化、條件摺疊、內聯、死碼刪除、copy-prop、bytecode peephole。
-O0 關閉最佳化。執行期仍是安全 VM，不是 LLVM 機器碼。
"
    );
}

fn split_passthrough(args: &[String]) -> (Vec<String>, Vec<String>) {
    if let Some(i) = args.iter().position(|a| a == "--") {
        (args[..i].to_vec(), args[i + 1..].to_vec())
    } else {
        (args.to_vec(), vec![])
    }
}

fn take_opt_level(args: &[String]) -> (OptLevel, Vec<String>) {
    let mut level = OptLevel::O2;
    let mut rest = Vec::new();
    for a in args {
        if let Some(l) = OptLevel::parse(a) {
            level = l;
        } else {
            rest.push(a.clone());
        }
    }
    (level, rest)
}

fn cmd_run(args: &[String]) {
    let (files, passthrough) = split_passthrough(args);
    let (level, files) = take_opt_level(&files);
    let path = files.first().cloned().unwrap_or_else(|| {
        print_help();
        process::exit(2);
    });
    let checked = match compile_path_with_opt(Path::new(&path), level) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };
    if let Err(d) = run_with_args(&checked, passthrough) {
        eprintln!("{}", d.message);
        process::exit(1);
    }
}

fn cmd_opt(args: &[String]) {
    let (level, files) = take_opt_level(args);
    let path = files.first().cloned().unwrap_or_else(|| {
        print_help();
        process::exit(2);
    });
    let mut checked = match compile_path_with_opt(Path::new(&path), OptLevel::O0) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };
    let stats = optimize(&mut checked, level);
    println!("{}", stats.report(level));
    print!("{}", compile_bytecode(&checked).disassemble());
}

fn cmd_check(args: &[String]) {
    let path = args.first().cloned().unwrap_or_else(|| {
        print_help();
        process::exit(2);
    });
    match compile_path(Path::new(&path)) {
        Ok(_) => println!("{path}: ok"),
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn cmd_test(args: &[String]) {
    let paths: Vec<String> = if args.is_empty() {
        walk_rsd(Path::new("."))
            .into_iter()
            .filter(|p| p.contains("test") || p.ends_with(".rsd"))
            .collect()
    } else {
        args.to_vec()
    };
    let mut failed = 0;
    let mut ran = 0;
    for path in &paths {
        if !path.ends_with(".rsd") {
            continue;
        }
        let Ok(checked) = compile_path(Path::new(path)) else {
            continue;
        };
        let tests: Vec<String> = checked
            .functions
            .keys()
            .filter(|n| n.starts_with("test_"))
            .cloned()
            .collect();
        if tests.is_empty() {
            continue;
        }
        for name in tests {
            ran += 1;
            let mut vm = Vm::new(&checked);
            match vm.run_named(&name) {
                Ok(_) => println!("ok   {path}::{name}"),
                Err(d) => {
                    failed += 1;
                    println!("FAIL {path}::{name}: {}", d.message);
                }
            }
        }
    }
    println!("{ran} tests, {failed} failed");
    if failed > 0 {
        process::exit(1);
    }
}

fn walk_rsd(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && e.file_name() != "target" && e.file_name() != ".git" {
            out.extend(walk_rsd(&p));
        } else if p.extension().and_then(|s| s.to_str()) == Some("rsd") {
            out.push(p.display().to_string());
        }
    }
    out
}

fn cmd_fmt(args: &[String]) {
    let path = args.first().cloned().unwrap_or_else(|| {
        print_help();
        process::exit(2);
    });
    let src = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    match format_source(&src) {
        Ok(s) => {
            if args.iter().any(|a| a == "--write" || a == "-w") {
                fs::write(&path, s).unwrap();
                println!("wrote {path}");
            } else {
                print!("{s}");
            }
        }
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn cmd_disasm(args: &[String]) {
    let path = args.first().cloned().unwrap_or_else(|| {
        print_help();
        process::exit(2);
    });
    match compile_path(Path::new(&path)) {
        Ok(c) => print!("{}", compile_bytecode(&c).disassemble()),
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn cmd_debug(args: &[String]) {
    eprintln!("debug: 編譯後以追蹤模式執行（印出每個使用者函式呼叫）");
    cmd_run(args);
}

fn cmd_complete(args: &[String]) {
    let path = args.first().cloned().unwrap_or_else(|| {
        print_help();
        process::exit(2);
    });
    let keywords = [
        "fn", "let", "mut", "if", "else", "while", "for", "in", "struct", "enum", "match", "impl",
        "return", "true", "false", "mod", "break", "continue", "loop", "self",
    ];
    for k in keywords {
        println!("{k}");
    }
    if let Ok(c) = compile_path(Path::new(&path)) {
        for n in c.functions.keys() {
            println!("{n}");
        }
        for s in c.structs.keys() {
            println!("{s}");
        }
    }
}

fn cmd_pkg(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("init") => {
            let toml = "[package]\nname = \"russd-app\"\nversion = \"0.1.0\"\nlicense = \"GPL-3.0-only\"\n\n[dependencies]\n";
            fs::write("russd.toml", toml).unwrap();
            println!("wrote russd.toml");
        }
        Some("info") | None => {
            if let Ok(s) = fs::read_to_string("russd.toml") {
                print!("{s}");
            } else {
                println!("沒有 russd.toml，請執行 `russd pkg init`");
            }
        }
        _ => {
            eprintln!("russd pkg init | info");
            process::exit(2);
        }
    }
}

fn repl() -> io::Result<()> {
    println!("Russd REPL  ·  :quit 離開");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut items = String::new();
    let mut stmts = String::new();
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
        match eval_repl(&mut items, &mut stmts, line) {
            Ok(Some(s)) => println!("{s}"),
            Ok(None) => {}
            Err(e) => eprintln!("{e}"),
        }
    }
    Ok(())
}

fn eval_repl(items: &mut String, stmts: &mut String, line: &str) -> Result<Option<String>, String> {
    let tokens = lex(line).map_err(|d| d.render("<repl>", line))?;
    match parse_repl_line(&tokens).map_err(|d| d.render("<repl>", line))? {
        ReplInput::Empty => Ok(None),
        ReplInput::Item(_) => {
            items.push_str(line);
            items.push('\n');
            let src = format!("{items}\nfn main() {{}}\n");
            compile(&src).map_err(|d| d.render("<repl>", line))?;
            println!("ok");
            Ok(None)
        }
        ReplInput::Stmt(_) => {
            stmts.push_str(line);
            stmts.push('\n');
            Ok(None)
        }
        ReplInput::Expr(_) => {
            let src = format!("{items}\nfn main() {{\n{stmts}\nprint({line})\n}}\n");
            let checked = compile(&src).map_err(|d| d.render("<repl>", line))?;
            let (_v, out) = run(&checked).map_err(|d| d.render("<repl>", line))?;
            Ok(Some(out.trim_end().to_string()))
        }
    }
}
