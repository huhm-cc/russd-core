use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_russd"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_args(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("spawn russd");
    let code = out.status.code().unwrap_or(1);
    (
        code,
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn run_hello_subcommand() {
    let (code, stdout, stderr) = run_args(&["run", "examples/hello.rsd"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(stdout.contains("Hello, Russd"), "stdout={stdout}");
}

#[test]
fn run_hello_double_dash_alias() {
    let (code, stdout, stderr) = run_args(&["--run", "examples/hello.rsd"]);
    assert_eq!(code, 0, "stderr={stderr}\nstdout={stdout}");
    assert!(stdout.contains("Hello, Russd"), "stdout={stdout}");
}

#[test]
fn run_hello_path_only() {
    let (code, stdout, stderr) = run_args(&["examples/hello.rsd"]);
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(stdout.contains("Hello, Russd"), "stdout={stdout}");
}

#[test]
fn unknown_command_says_not_found() {
    let (code, _stdout, stderr) = run_args(&["nonesuch"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("找不到指令") || stderr.contains("nonesuch"),
        "stderr={stderr}"
    );
}
