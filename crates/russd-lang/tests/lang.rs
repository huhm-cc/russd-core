use russd_lang::{compile, run_source};

fn run_ok(src: &str) -> String {
    run_source("<test>", src).unwrap_or_else(|e| panic!("{e}"))
}

fn run_err(src: &str) -> String {
    run_source("<test>", src).expect_err("expected error")
}

#[test]
fn hello() {
    let out = run_ok(
        r#"
fn greet(name: str) -> str {
    "Hello, " + name
}
fn main() {
    print(greet("Russd"))
}
"#,
    );
    assert_eq!(out, "Hello, Russd\n");
}

#[test]
fn arithmetic_and_while() {
    let out = run_ok(
        r#"
fn main() {
    let mut n = 0
    let mut s = 0
    while n < 5 {
        s = s + n
        n = n + 1
    }
    print(s)
}
"#,
    );
    assert_eq!(out, "10\n");
}

#[test]
fn vec_for_and_index() {
    let out = run_ok(
        r#"
fn main() {
    let mut xs: Vec[i32] = vec()
    xs.push(1)
    xs.push(2)
    xs.push(3)
    let mut sum = 0
    for x in xs {
        sum = sum + x
    }
    print(sum)
}
"#,
    );
    assert_eq!(out, "6\n");
}

#[test]
fn option_match() {
    let out = run_ok(
        r#"
fn main() {
    let v: Option[i32] = Some(7)
    match v {
        Some(x) => print(x)
        None => print(0)
    }
}
"#,
    );
    assert_eq!(out, "7\n");
}

#[test]
fn struct_impl() {
    let out = run_ok(
        r#"
struct Point {
    x: i32
    y: i32
}
impl Point {
    fn mag2(self) -> i32 {
        self.x * self.x + self.y * self.y
    }
}
fn main() {
    let p = Point { x: 3, y: 4 }
    print(p.mag2())
}
"#,
    );
    assert_eq!(out, "25\n");
}

#[test]
fn overflow_is_error() {
    let err = run_err(
        r#"
fn main() {
    print(2147483647 + 1)
}
"#,
    );
    assert!(err.contains("溢位"), "{err}");
}

#[test]
fn move_error() {
    let err = run_err(
        r#"
fn take(s: str) {}
fn main() {
    let s = "hi"
    take(s)
    print(s)
}
"#,
    );
    assert!(err.contains("已被移動"), "{err}");
}

#[test]
fn mut_borrow_conflict() {
    let src = r#"
fn two(a: &mut i32, b: &mut i32) {}
fn main() {
    let mut n = 1
    two(&mut n, &mut n)
}
"#;
    let err = compile(src).expect_err("should fail");
    assert!(
        err.message.contains("借用") || err.message.contains("&mut"),
        "{}",
        err.message
    );
}

#[test]
fn no_ref_fields() {
    let src = r#"
struct Bad {
    x: &i32
}
fn main() {}
"#;
    let err = compile(src).expect_err("should fail");
    assert!(err.message.contains("引用"), "{}", err.message);
}

#[test]
fn result_try() {
    let out = run_ok(
        r#"
fn parse() -> Result[i32, str] {
    Ok(42)
}
fn go() -> Result[i32, str] {
    let n = parse()?
    Ok(n + 1)
}
fn main() {
    match go() {
        Ok(n) => print(n)
        Err(e) => print(e)
    }
}
"#,
    );
    assert_eq!(out, "43\n");
}

#[test]
fn bitwise_and_loop() {
    let out = run_ok(
        r#"
fn main() {
    print(1 << 3)
    print(6 & 3)
    print(6 | 1)
    let mut n = 0
    loop {
        n += 1
        if n >= 2 {
            break
        }
    }
    print(n)
}
"#,
    );
    assert_eq!(out, "8\n2\n7\n2\n");
}

#[test]
fn map_and_string() {
    let out = run_ok(
        r#"
fn main() {
    let mut m: Map[i32] = map()
    m.set("a", 1)
    match m.get("a") {
        Some(v) => print(v)
        None => print(0)
    }
    print("Ab".to_lower())
    print((7 ^ 3))
}
"#,
    );
    assert_eq!(out, "1\nab\n4\n");
}

#[test]
fn sandbox_rejects_escape() {
    let out = run_ok(
        r#"
fn main() {
    match read_file("../Cargo.toml") {
        Ok(_) => print("leaked")
        Err(_) => print("blocked")
    }
    match read_file("/etc/passwd") {
        Ok(_) => print("abs")
        Err(_) => print("noabs")
    }
}
"#,
    );
    assert!(out.contains("blocked"), "{out}");
    assert!(out.contains("noabs"), "{out}");
}

#[test]
fn fizz_first_lines() {
    let out = run_ok(
        r#"
fn main() {
    for n in 1..6 {
        if n % 3 == 0 {
            print("Fizz")
        } else {
            print(n)
        }
    }
}
"#,
    );
    assert_eq!(out, "1\n2\nFizz\n4\n5\n");
}
