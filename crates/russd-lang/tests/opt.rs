use russd_lang::{compile_bytecode, compile_with_opt, run, OptLevel};

fn run_o(level: OptLevel, src: &str) -> String {
    let c = compile_with_opt(src, level).unwrap_or_else(|e| panic!("{e:?}"));
    run(&c).unwrap_or_else(|e| panic!("{}", e.message)).1
}

#[test]
fn const_fold_arithmetic() {
    let src = r#"
fn main() {
    print(1 + 2 * 3)
    print(1 << 3)
}
"#;
    assert_eq!(run_o(OptLevel::O2, src), "7\n8\n");
    let c = compile_with_opt(src, OptLevel::O2).unwrap();
    let dis = compile_bytecode(&c).disassemble();
    assert!(dis.contains("Const(I32(7))"), "{dis}");
    assert!(dis.contains("Const(I32(8))"), "{dis}");
}

#[test]
fn fold_true_branch_and_inline() {
    let src = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
fn main() {
    if true {
        print(add(2, 3))
    } else {
        print(0)
    }
}
"#;
    assert_eq!(run_o(OptLevel::O2, src), "5\n");
    let c = compile_with_opt(src, OptLevel::O2).unwrap();
    let dis = compile_bytecode(&c).disassemble();
    let main = dis.split("fn main:").nth(1).unwrap_or(&dis);
    assert!(
        main.contains("Const(I32(5))"),
        "expected inlined/folded 5 in main:\n{dis}"
    );
    assert!(!main.contains("JumpIfFalse"), "{dis}");
}

#[test]
fn dce_unused_let() {
    let src = r#"
fn main() {
    let dead = 1 + 2
    print(4)
}
"#;
    assert_eq!(run_o(OptLevel::O2, src), "4\n");
}

#[test]
fn o0_still_correct() {
    let src = r#"
fn main() {
    print(1 + 2 * 3)
}
"#;
    assert_eq!(run_o(OptLevel::O0, src), "7\n");
}

#[test]
fn overflow_not_folded_into_wrap() {
    let src = r#"
fn main() {
    print(2147483647 + 1)
}
"#;
    let err = compile_with_opt(src, OptLevel::O2)
        .ok()
        .and_then(|c| run(&c).err())
        .map(|d| d.message)
        .unwrap_or_else(|| "no runtime error".into());
    assert!(err.contains("溢位"), "{err}");
}
