import type { Doc, Lang } from "./doc-types";

const hello = `fn greet(name: str) -> str {
    "Hello, " + name
}

fn main() {
    print(greet("Russd"))
}`;

const types = `fn main() {
    let n: i32 = 7
    let big: i64 = 10000000000i64
    let x: f64 = 3.14
    let ok: bool = true
    let s: str = "hi"
    let nothing: () = ()
    print(n, big, x, ok, s, nothing)
}`;

const vars = `fn main() {
    let a = 1
    let mut b = 2
    b = b + a
    b += 3
    print(b)
}`;

const ops = `fn main() {
    print(1 + 2 * 3)
    print(10 / 2)
    print(10 % 3)
    print(1 << 3)
    print(6 & 3)
    print(6 | 1)
    print(7 ^ 3)
    print(~0)
    print(true && false)
    print(!false)
}`;

const control = `fn main() {
    let n = 15
    if n % 15 == 0 {
        print("FizzBuzz")
    } else if n % 3 == 0 {
        print("Fizz")
    } else {
        print(n)
    }

    let mut i = 0
    while i < 3 {
        i += 1
    }

    for x in 1..4 {
        print(x)
    }

    loop {
        i += 1
        if i >= 6 {
            break
        }
        if i == 4 {
            continue
        }
        print(i)
    }
}`;

const functions = `fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn twice(n: i32) -> i32 {
    n + n
}

fn main() {
    print(twice(add(2, 3)))
}`;

const strings = `fn main() {
    let s = " Hello, Russd\\n "
    print(s.trim())
    print("Ab".to_lower())
    print("ab".to_upper())
    print("a,b,c".split(",").join("-"))
    print("hello".starts_with("he"))
    print("hello".contains("ell"))
    print("12".parse_i32())
}`;

const vecs = `fn main() {
    let mut xs: Vec[i32] = vec()
    xs.push(10)
    xs.push(20)
    xs.push(30)
    print(xs.len())
    print(xs[1])
    xs[1] = 21
    print(xs.contains(21))
    match xs.pop() {
        Some(v) => print(v)
        None => print("empty")
    }
    xs.reverse()
    print(xs)
}`;

const maps = `fn main() {
    let mut m: Map[i32] = map()
    m.set("score", 99)
    match m.get("score") {
        Some(v) => print("score", v)
        None => print("missing")
    }
    print(m.keys())
    print(m.contains("score"))
}`;

const structs = `struct Point {
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
    print(p.x, p.y, p.mag2())
}`;

const enums = `enum Color {
    Red
    Rgb(i32, i32, i32)
}

fn main() {
    let c = Color::Rgb(1, 2, 3)
    match (c) {
        Color::Red => print("red")
        Color::Rgb(r, g, b) => print(r, g, b)
    }
}`;

const result = `fn parse() -> Result[i32, str] {
    "41".parse_i32()
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
}`;

const ownership = `fn take(s: str) {
    print(s)
}

fn main() {
    let s = "hi"
    take(s)
    // take(s)  // 錯誤：已被移動。要用 s.clone()
    let t = "ok".clone()
    print(t)
}`;

const modules = `// app.rsd
mod math

fn main() {
    print(add(2, 3))
}

// math.rsd（同目錄）
fn add(a: i32, b: i32) -> i32 {
    a + b
}`;

const builtins = `fn worker() {
    print("thread")
}

fn main() {
    print(abs(-3))
    print(min(2, 9))
    print(sqrt(9.0))
    print(extern_call("sin", 0.0))
    print(now_ms())
    let id = spawn("worker")
    join(id)
}`;

const io = `fn main() {
    match write_file("out.txt", "hello") {
        Ok(_) => print("wrote")
        Err(e) => print(e)
    }
    match read_file("out.txt") {
        Ok(s) => print(s)
        Err(e) => print(e)
    }
    match http_get("http://example.com/") {
        Ok(body) => print(body.len())
        Err(e) => print(e)
    }
}`;

const tests = `fn test_add() {
    assert(1 + 2 == 3)
}

fn main() {}`;

const limitsSyntaxZh = [
  "沒有 `unsafe`、原始指標、生命週期 `'a`、`dyn`、`impl Trait`。",
  "沒有 `trait`、class、繼承、`super`、`virtual`。結構用欄位組合，行為寫在 `impl`。",
  "沒有泛型函式或泛型結構。只有內建 `Vec[T]`、`Map[V]`、`Option[T]`、`Result[T,E]` 帶型別參數，語法用 `[]` 不是 `<>`。",
  "沒有閉包、函式指標、把 `fn` 當值傳遞。`spawn` 只接受函式名稱字串。",
  "沒有 `use` / `pub` / `crate` / 路徑 `foo::bar`（列舉變體 `Color::Red` 除外）。模組是扁平合併，名稱會撞車。",
  "沒有 `const`、`static`、執行期全域可變狀態。",
  "沒有 tuple（只有 `()`）、沒有固定長度陣列、沒有 `char`、沒有 slice。",
  "沒有 `u8`/`u32`/`usize`/`f32`。整數預設 `i32`，超過請加 `i64` 後綴。沒有 `0x`/`0b` 字面值。",
  "識別字只能 ASCII 字母、數字、底線；不能用中文當變數名。",
  "註解只有 `//` 到行尾。沒有 `/* */`、沒有 `///` 文件註解、沒有屬性 `#[...]`、沒有巨集。",
  "字串只有 `\"...\"`，跳脫只允許 `\\n` `\\t` `\\\\` `\\\"`。沒有字串插值、沒有 `format!`、沒有原始字串 `r\"\"`。",
  "沒有 `else if` 關鍵字以外的 `if let` / `while let` / match guard（`if` 條件）。`else if` 可以用。",
  "沒有 `..=` 含端點範圍。`1..4` 是 1、2、3。",
  "複合賦值只有 `+=` `-=` `*=`，沒有 `/=` `%=` `&=` `|=`。",
  "`&mut` 目前只能取變數位址，不能對任意表達式取可變引用。",
  "結構欄位不能是引用（`x: &i32` 會被拒絕）。引用只活在區域變數與參數上。",
  "列舉變體只能是無欄位或元組欄位 `Rgb(i32,i32,i32)`，沒有結構體變體 `Rgb { r, g, b }`。",
  "沒有運算子多載、沒有 `Index` trait。只有 `Vec` 能用 `[]`。`Map` 必須 `.set` / `.get`，鍵必須是 `str`。",
  "`push`/`pop`/`set`/`clear`/`reverse` 的接收者必須是變數，不能寫 `vec().push(1)`。",
  "沒有型別別名、沒有 `as` 轉型（少數內建如 `sqrt` 會把 `i32` 當 `f64`）。",
  "沒有 `async`/`await`、channel、mutex、Rc/Arc、內部可變。並行只有 `spawn`/`join`。",
  "檔案頂層只能出現 `fn` `struct` `enum` `impl` `mod`。不能在頂層寫 `let`。",
  "`russd run` 需要 `fn main()`。測試函式名稱必須以 `test_` 開頭，沒有 `#[test]`。",
  "`join` 只接受 `Vec[str]`，不能對 `Vec[i32]` 呼叫。",
  "`match x { Color::Red => ... }` 會被解析成結構字面值。請寫 `match (x) { ... }`，或模式只寫 `Red` / `Rgb(r,g,b)`。",
  "賦值左邊只能是變數、結構欄位或 `Vec` 索引。不能寫 `*x = ...` 把值寫回 `&mut`。",
];

const limitsRuntimeZh = [
  "`i32` 加減乘溢位是錯誤，不是繞回。除零、向量越界也是錯誤，不是未定義行為。",
  "`str` 與 `Vec` 傳遞會移動；之後再用會編譯失敗。數字、`bool`、`()` 會複製。需要第二份就 `.clone()`。",
  "`read_file` / `write_file` 只能碰工作目錄內的相對路徑。絕對路徑、`..`、超出 cwd 一律拒絕。",
  "單次寫入上限 1 MB。",
  "`http_get` 只接受 `http://`（沒有 TLS、沒有 HTTPS）、只連埠 80、拒絕 localhost / 私有網段 / DNS 指到內網。URL 最長 2048。回應最多 256 KB。逾時約 5 秒。",
  "`sleep_ms` 最多睡 5000 毫秒，再大也會被截斷。",
  "`spawn(\"name\")` 的名稱必須是 ≤64 字元的 ASCII 識別字，對應現有無參數函式。不能傳參數進執行緒，也不能 spawn 作業系統指令。",
  "`extern_call` 白名單只有 `sin` `cos` `sqrt` `floor` `ceil` `pow` `abs`。不能 `dlopen`、不能呼叫任意 C 函式。",
  "沒有讀環境變數、沒有標準輸入 API、沒有開任意 TCP/UDP socket、沒有執行 shell。",
  "`str.len()` 是位元組數，不是 Unicode 字數。沒有字元迭代。",
  "`print` 把參數用空白接起來並換行。沒有 `println!` 格式化。",
  "執行器是最佳化編譯器 + 安全 VM，不是 LLVM 機器碼。不要期待原生碼那種「世界最快」。",
];

const limitsSyntaxEn = [
  "No `unsafe`, raw pointers, lifetimes `'a`, `dyn`, or `impl Trait`.",
  "No `trait`, classes, inheritance, `super`, or `virtual`. Compose structs; put behavior in `impl`.",
  "No generic functions or generic structs. Only built-in `Vec[T]`, `Map[V]`, `Option[T]`, `Result[T,E]` take type arguments, written with `[]` not `<>`.",
  "No closures, function pointers, or passing `fn` as a value. `spawn` takes a function-name string.",
  "No `use` / `pub` / `crate` / paths `foo::bar` (except enum variants like `Color::Red`). Modules flatten into one namespace.",
  "No `const` or `static`.",
  "No tuples except `()`, no fixed-size arrays, no `char`, no slices.",
  "No `u8`/`u32`/`usize`/`f32`. Integers default to `i32`; write `i64` with an `i64` suffix. No `0x`/`0b` literals.",
  "Identifiers are ASCII letters, digits, and `_` only. Chinese (or other non-ASCII) names are rejected.",
  "Comments are `//` to end of line only. No `/* */`, no `///`, no attributes `#[...]`, no macros.",
  "Strings are `\"...\"` with escapes `\\n` `\\t` `\\\\` `\\\"` only. No interpolation, no `format!`, no raw strings.",
  "No `if let`, `while let`, or match guards. `else if` is allowed.",
  "No inclusive range `..=`. `1..4` yields 1, 2, 3.",
  "Compound assignment is only `+=` `-=` `*=`. No `/=` `%=` `&=` `|=`.",
  "`&mut` currently applies to variables only, not arbitrary expressions.",
  "Struct fields cannot be references (`x: &i32` is rejected). References live on locals and parameters only.",
  "Enum variants are unit or tuple-like (`Rgb(i32,i32,i32)`), not struct-like `{ r, g, b }`.",
  "No operator overloading. Only `Vec` supports `[]`. Maps use `.set` / `.get` and keys must be `str`.",
  "`push`/`pop`/`set`/`clear`/`reverse` need a variable receiver: not `vec().push(1)`.",
  "No type aliases and almost no `as` casts (`sqrt` will take `i32` as `f64`).",
  "No `async`/`await`, channels, mutexes, Rc/Arc, or interior mutability. Concurrency is `spawn`/`join` only.",
  "File top level allows only `fn` `struct` `enum` `impl` `mod`. No top-level `let`.",
  "`russd run` needs `fn main()`. Tests must be named `test_*`; there is no `#[test]`.",
  "`join` accepts only `Vec[str]`, not `Vec[i32]`.",
  "`match x { Color::Red => ... }` is parsed as a struct literal. Write `match (x) { ... }`, or drop the prefix (`Red`, `Rgb(r,g,b)`).",
  "Assignment targets are variables, struct fields, or `Vec` indexes. You cannot write `*x = ...` through `&mut`.",
];

const limitsRuntimeEn = [
  "`i32` add/sub/mul overflow is an error, not wraparound. Division by zero and out-of-bounds indexes are errors, not undefined behavior.",
  "`str` and `Vec` move on pass; using them afterwards fails at check time. Numbers, `bool`, and `()` copy. Use `.clone()` for a second owned value.",
  "`read_file` / `write_file` only accept relative paths inside the working directory. Absolute paths and `..` are rejected.",
  "A single write is capped at 1 MB.",
  "`http_get` is `http://` only (no TLS/HTTPS), port 80 only, and it refuses localhost, private networks, and DNS that resolves private. URL max 2048 bytes, body max 256 KB, timeout about 5s.",
  "`sleep_ms` is capped at 5000 ms.",
  "`spawn(\"name\")` must be a ≤64-character ASCII identifier naming an existing zero-argument function. You cannot pass thread arguments or spawn OS processes.",
  "`extern_call` is a whitelist: `sin` `cos` `sqrt` `floor` `ceil` `pow` `abs`. No `dlopen`, no arbitrary C.",
  "No env vars, no stdin API, no raw TCP/UDP, no shell.",
  "`str.len()` is bytes, not Unicode scalar count. No character iteration.",
  "`print` joins arguments with spaces and adds a newline. No `println!` formatting.",
  "The engine is an optimizing compiler plus a safe VM, not LLVM native code. It is not “the world’s fastest language.”",
];

const limitsSyntaxCn = [
  "没有 `unsafe`、原始指针、生命周期 `'a`、`dyn`、`impl Trait`。",
  "没有 `trait`、class、继承、`super`。结构靠字段组合，行为写在 `impl`。",
  "没有泛型函数或泛型结构。只有内建 `Vec[T]`、`Map[V]`、`Option[T]`、`Result[T,E]` 带类型参数，写法是 `[]` 不是 `<>`。",
  "没有闭包、函数指针、把 `fn` 当值传递。`spawn` 只接受函数名字符串。",
  "没有 `use` / `pub` / `crate` / 路径 `foo::bar`（枚举变体 `Color::Red` 除外）。模块是扁平合并。",
  "没有 `const`、`static`。",
  "没有元组（只有 `()`）、没有定长数组、没有 `char`、没有 slice。",
  "没有 `u8`/`u32`/`usize`/`f32`。整数默认 `i32`，更大请加 `i64` 后缀。没有 `0x`/`0b` 字面量。",
  "标识符只能是 ASCII 字母、数字、下划线；不能用中文当变量名。",
  "注释只有 `//` 到行尾。没有 `/* */`、没有 `///`、没有属性 `#[...]`、没有宏。",
  "字符串只有 `\"...\"`，转义只允许 `\\n` `\\t` `\\\\` `\\\"`。没有插值、没有 `format!`。",
  "没有 `if let` / `while let` / match guard。可以用 `else if`。",
  "没有 `..=`。`1..4` 是 1、2、3。",
  "复合赋值只有 `+=` `-=` `*=`，没有 `/=` `%=` `&=`。",
  "`&mut` 目前只能取变量地址。",
  "结构字段不能是引用。引用只活在局部变量和参数上。",
  "枚举变体只能是无字段或元组字段，没有结构体变体。",
  "没有运算符重载。只有 `Vec` 能用 `[]`。`Map` 必须 `.set` / `.get`，键必须是 `str`。",
  "`push`/`pop`/`set`/`clear`/`reverse` 的接收者必须是变量。",
  "没有类型别名，几乎没有 `as` 转型。",
  "没有 `async`/`await`、channel、mutex、Rc/Arc。并发只有 `spawn`/`join`。",
  "文件顶层只能出现 `fn` `struct` `enum` `impl` `mod`。不能在顶层写 `let`。",
  "`russd run` 需要 `fn main()`。测试函数名必须以 `test_` 开头，没有 `#[test]`。",
  "`join` 只接受 `Vec[str]`，不能对 `Vec[i32]` 调用。",
  "`match x { Color::Red => ... }` 会被解析成结构字面量。请写 `match (x) { ... }`，或模式只写 `Red` / `Rgb(r,g,b)`。",
  "赋值左边只能是变量、结构字段或 `Vec` 下标。不能写 `*x = ...` 把值写回 `&mut`。",
];

const limitsRuntimeCn = [
  "`i32` 加减乘溢出是错误，不是回绕。除零、向量越界也是错误。",
  "`str` 与 `Vec` 传递会移动；之后再用会编译失败。数字、`bool`、`()` 会复制。需要第二份就 `.clone()`。",
  "`read_file` / `write_file` 只能碰工作目录内的相对路径。绝对路径、`..` 一律拒绝。",
  "单次写入上限 1 MB。",
  "`http_get` 只接受 `http://`（没有 TLS/HTTPS）、只连端口 80、拒绝 localhost / 私有网段。URL 最长 2048。响应最多 256 KB。超时约 5 秒。",
  "`sleep_ms` 最多睡 5000 毫秒。",
  "`spawn(\"name\")` 必须是 ≤64 字符的 ASCII 标识符，对应已有无参数函数。不能传参数进线程，也不能 spawn 操作系统命令。",
  "`extern_call` 白名单只有 `sin` `cos` `sqrt` `floor` `ceil` `pow` `abs`。不能 `dlopen`。",
  "没有读环境变量、没有标准输入 API、没有任意 TCP/UDP、没有执行 shell。",
  "`str.len()` 是字节数，不是 Unicode 字符数。",
  "`print` 用空格拼接参数并换行。没有 `println!` 格式化。",
  "执行器是优化编译器 + 安全 VM，不是 LLVM 机器码。",
];

export const howtoUse: Record<Lang, Doc> = {
  zh: {
    title: "如何使用、如何寫程式",
    lead: "這頁教 Russd 怎麼寫：語法、你實際能做的事，以及一長串刻意加上的限制。安裝與下載請看「下載核心」；專案設定請看「如何設定」。",
    sections: [
      {
        heading: "1. 程式長什麼樣子",
        body: [
          "原始檔副檔名是 `.rsd`。`russd run` 要有 `fn main()`。分號可寫可不寫。區塊最後一個表達式就是回傳值，不必處處 `return`。",
          "註解只有 `//`。關鍵字：`fn` `let` `mut` `if` `else` `while` `for` `in` `loop` `break` `continue` `struct` `enum` `match` `impl` `return` `true` `false` `self` `mod`。",
          "先跑起來：`cargo run -- examples/hello.rsd`（`--` 與檔名分開寫；不要寫成 `cargo run --run`，cargo 會說指令不存在）。參數再加一組 `--`，程式裡用 `args()` 讀成 `Vec[str]`：`cargo run -- examples/hello.rsd -- foo`。",
        ],
        code: { title: "hello.rsd", lang: "rsd", text: hello },
      },
      {
        heading: "編譯器會最佳化",
        body: [
          "Russd 是最佳化編譯器。預設 `-O2`：常數摺疊、`x+0`/`x*1` 這類代數簡化、`if true` 刪掉死分支、小型函式內聯、沒用到的 `let` 刪除、bytecode peephole。",
          "`russd run f.rsd -O0` 關掉最佳化。`russd opt f.rsd` 印摺疊/內聯/刪碼次數與 bytecode。溢位與除零不會被摺成繞回。後端仍是安全 VM，不是 LLVM 原生碼。",
        ],
        code: {
          title: "看最佳化",
          lang: "bash",
          text: `russd opt examples/hello.rsd
russd run examples/fizzbuzz.rsd -O2
russd run examples/fizzbuzz.rsd -O0`,
        },
      },
      {
        heading: "2. 型別",
        body: [
          "內建型別：`i32` `i64` `f64` `bool` `str` `()` `Vec[T]` `Map[V]` `Option[T]` `Result[T,E]`。泛型參數用方括號。",
          "整數字面值預設是 `i32`。超過 `i32` 要寫成 `10000000000i64`，否則型別檢查會擋。浮點要有小數點，例如 `3.14`。",
        ],
        code: { title: "型別", lang: "rsd", text: types },
      },
      {
        heading: "3. 變數",
        body: [
          "`let` 不可變；`let mut` 才能再賦值。可以寫 `let n: i32 = 1` 標型別，也可以靠推斷。",
          "賦值目標必須是變數、結構欄位、或 `Vec` 索引。不能對不可變綁定賦值。",
        ],
        code: { title: "let / let mut", lang: "rsd", text: vars },
      },
      {
        heading: "4. 運算子",
        body: [
          "算術 `+ - * / %`（`+` 也可接字串）。比較 `== != < <= > >=`。邏輯 `&& || !`。位元 `& | ^ ~ << >>`。",
          "運算子沒有多載：`+` 不能拿來加兩個結構。比較目前只允許數字。",
        ],
        code: { title: "運算", lang: "rsd", text: ops },
      },
      {
        heading: "5. 控制流程",
        body: [
          "`if` / `else if` / `else` 是表達式，分支要用 `{ }`。`while` 條件必須是 `bool`。",
          "`for x in xs` 可走 `Vec` 或範圍 `1..n`（不含 n）。`loop` 搭配 `break` / `continue`。`break` 不能帶值。",
        ],
        code: { title: "if / while / for / loop", lang: "rsd", text: control },
      },
      {
        heading: "6. 函式",
        body: [
          "寫法：`fn 名稱(參數: 型別) -> 回傳型別 { ... }`。省略 `->` 就是 `()`。",
          "參數可為值、`mut x: T`、`&T`、`&mut T`。`impl` 裡可用 `self`、`&self`、`&mut self`。沒有預設參數、沒有可變參數、沒有泛型參數。可變狀態主要靠 `let mut` 與 Vec/Map 方法；`*x = ...` 還不能用來寫回引用。",
        ],
        code: { title: "fn", lang: "rsd", text: functions },
      },
      {
        heading: "7. 字串",
        body: [
          "`str` 是擁有所有權的字串。方法：`len` `is_empty` `contains` `split` `replace` `trim` `to_upper` `to_lower` `starts_with` `ends_with` `parse_i32`。",
          "`parse_i32` 回傳 `Result[i32, str]`。`len` 是 UTF-8 位元組數。",
        ],
        code: { title: "字串方法", lang: "rsd", text: strings },
      },
      {
        heading: "8. 向量 Vec",
        body: [
          "`vec(...)` 建立向量，可帶初始元素。`push` `pop` `clear` `reverse` 會改原向量，接收者必須是變數。",
          "索引 `xs[i]` 只接受整數；越界執行期錯誤。`for x in xs` 會走過元素。`join(sep)` 只適用於 `Vec[str]`。",
        ],
        code: { title: "Vec", lang: "rsd", text: vecs },
      },
      {
        heading: "9. 對照表 Map",
        body: [
          "`map()` 不接受參數。值型別寫在 `Map[V]`。鍵永遠是 `str`，用 `.set(k, v)` 與 `.get(k)`（回 `Option[V]`），不能 `m[\"k\"]`。",
          "還有 `keys()` `contains` `len` `is_empty` `clear`。",
        ],
        code: { title: "Map", lang: "rsd", text: maps },
      },
      {
        heading: "10. 結構與 impl",
        body: [
          "具名欄位結構：`struct Point { x: i32 y: i32 }`。建構：`Point { x: 3, y: 4 }`。欄位必須一次給齊、名稱必須對。",
          "`impl Point { fn mag2(self) -> i32 { ... } }` 給方法。沒有繼承、沒有建構子關鍵字、沒有私有欄位。",
        ],
        code: { title: "struct / impl", lang: "rsd", text: structs },
      },
      {
        heading: "11. 列舉與 match",
        body: [
          "自訂列舉：`enum Color { Red Rgb(i32, i32, i32) }`。值寫 `Color::Red` 或 `Color::Rgb(1,2,3)`。",
          "`match` 用 `=>`。內建 `Some(x)` `None` `Ok(x)` `Err(e)` 可直接寫。也可用 `_`、整數、布林、字串模式。沒有 guard。對自訂列舉請寫 `match (值) { Color::Red => ... }`：若寫 `match 值 { Color::... }`，`Color:` 會被當成結構欄位。",
        ],
        code: { title: "enum / match", lang: "rsd", text: enums },
      },
      {
        heading: "12. Option、Result、?",
        body: [
          "`Option[T]`：`Some(v)` / `None`。`Result[T,E]`：`Ok(v)` / `Err(e)`。",
          "在回傳 `Result` 的函式裡，`expr?` 遇到 `Err` 會立刻回傳那個錯誤。`?` 不能用在 `Option`。失敗要處理就 `match`，或 `panic(\"...\")` / `assert(條件)` 直接停。",
        ],
        code: { title: "Result 與 ?", lang: "rsd", text: result },
      },
      {
        heading: "13. 所有權與借用",
        body: [
          "非 Copy 值（`str`、`Vec`、`Map`、結構、列舉）傳進函式會被移動，不能再用。Copy 的有 `i32` `i64` `f64` `bool` `()`。",
          "`&T` 可同時有多個；`&mut T` 同時只能一個，且不能再同時有共享借用。解引用用 `*`。沒有生命週期標註可寫——編譯器拒絕把引用塞進結構，問題直接不存在。",
        ],
        code: { title: "移動", lang: "rsd", text: ownership },
      },
      {
        heading: "14. 模組",
        body: [
          "`mod math` 會載入同目錄 `math.rsd` 或 `math/mod.rsd`，把裡面的項目合併進目前程式。沒有獨立命名空間、沒有 `pub`。循環載入會被跳過。",
        ],
        code: { title: "mod", lang: "rsd", text: modules },
      },
      {
        heading: "15. 你還能做的事（內建）",
        body: [
          "輸出：`print(...)`。數學：`abs` `min` `max` `sqrt`。時間：`now_ms()` `sleep_ms(n)`。測試：`assert(bool)`。中止：`panic(str)`。",
          "執行緒：`spawn(\"函式名\")` 回 `i32` id，再用 `join(id)`。FFI 白名單：`extern_call(\"sin\", x)` 等。複製：`.clone()`。",
        ],
        code: { title: "內建", lang: "rsd", text: builtins },
      },
      {
        heading: "16. 檔案與 HTTP",
        body: [
          "I/O 回 `Result`。路徑與網路都有沙盒，失敗是 `Err`，不是默默讀到系統檔。細節見下一節限制清單。",
        ],
        code: { title: "檔案 / HTTP", lang: "rsd", text: io },
      },
      {
        heading: "17. 測試",
        body: [
          "`russd test 檔案.rsd` 會執行所有名稱以 `test_` 開頭的函式。用 `assert` 檢查。不必有 `main` 邏輯，但檔案裡通常仍放一個空的 `fn main()`。",
        ],
        code: { title: "test_", lang: "rsd", text: tests },
      },
      {
        heading: "18. 語法限制（刻意沒有的東西）",
        body: [
          "Russd 看起來像簡化的 Rust，但下面這些你在 Rust 裡習慣的寫法，這裡都沒有。寫之前先看完，少踩編譯器。",
        ],
        bullets: limitsSyntaxZh,
      },
      {
        heading: "19. 執行期與安全限制",
        body: [
          "這些不是「還沒做完」的含糊承諾，而是執行期現在就會執行的規則。想做清單以外的事，目前就是不行。",
        ],
        bullets: limitsRuntimeZh,
      },
    ],
  },
  en: {
    title: "How to use and how to write Russd",
    lead: "This page is the language tutorial: syntax, what you can actually do, and a long list of limits. Install and clone live under Download core; project files live under How to configure.",
    sections: [
      {
        heading: "1. Shape of a program",
        body: [
          "Source files end in `.rsd`. `russd run` requires `fn main()`. Semicolons are optional. The last expression in a block is the return value.",
          "Comments are `//` only. Keywords: `fn` `let` `mut` `if` `else` `while` `for` `in` `loop` `break` `continue` `struct` `enum` `match` `impl` `return` `true` `false` `self` `mod`.",
          "Run with `cargo run -- examples/hello.rsd` (space after `--`; do not write `cargo run --run`, Cargo treats that as its own flag and reports not found). Extra program args go after another `--`: `cargo run -- examples/hello.rsd -- foo`, read with `args()` as `Vec[str]`.",
        ],
        code: { title: "hello.rsd", lang: "rsd", text: hello },
      },
      {
        heading: "The compiler optimizes",
        body: [
          "Russd is an optimizing compiler. Default `-O2`: constant folding, algebraic identities (`x+0`, `x*1`), dead `if` branches, small inlining, unused `let` removal, bytecode peephole.",
          "`russd run f.rsd -O0` turns passes off. `russd opt f.rsd` prints fold/inline/dce counts and bytecode. Overflow and division by zero are not folded into wraparound. The backend is still a safe VM, not LLVM native code.",
        ],
        code: {
          title: "inspect optimizations",
          lang: "bash",
          text: `russd opt examples/hello.rsd
russd run examples/fizzbuzz.rsd -O2
russd run examples/fizzbuzz.rsd -O0`,
        },
      },
      {
        heading: "2. Types",
        body: [
          "Built-ins: `i32` `i64` `f64` `bool` `str` `()` `Vec[T]` `Map[V]` `Option[T]` `Result[T,E]`. Type arguments use square brackets.",
          "Integer literals are `i32` by default. Values that do not fit need an `i64` suffix. Floats need a decimal point (`3.14`).",
        ],
        code: { title: "types", lang: "rsd", text: types },
      },
      {
        heading: "3. Variables",
        body: [
          "`let` is immutable; `let mut` can be assigned again. You may annotate `let n: i32 = 1` or let the checker infer.",
          "Assignment targets are variables, struct fields, or `Vec` indexes.",
        ],
        code: { title: "let / let mut", lang: "rsd", text: vars },
      },
      {
        heading: "4. Operators",
        body: [
          "Arithmetic `+ - * / %` (`+` also concatenates strings). Comparison `== != < <= > >=`. Logic `&& || !`. Bitwise `& | ^ ~ << >>`.",
          "No overloading: you cannot `+` two structs. Comparisons are for numbers.",
        ],
        code: { title: "operators", lang: "rsd", text: ops },
      },
      {
        heading: "5. Control flow",
        body: [
          "`if` / `else if` / `else` are expressions; branches need `{ }`. `while` conditions are `bool`.",
          "`for x in xs` walks a `Vec` or a range `1..n` (exclusive). `loop` with `break` / `continue`. `break` does not take a value.",
        ],
        code: { title: "if / while / for / loop", lang: "rsd", text: control },
      },
      {
        heading: "6. Functions",
        body: [
          "`fn name(param: Type) -> Type { ... }`. Omit `->` for `()`.",
          "Parameters may be values, `mut x: T`, `&T`, or `&mut T`. Methods may take `self`, `&self`, `&mut self`. No default args, no varargs, no type parameters on your functions. Mutation is mainly `let mut` plus Vec/Map methods; `*x = ...` cannot write back through a reference yet.",
        ],
        code: { title: "fn", lang: "rsd", text: functions },
      },
      {
        heading: "7. Strings",
        body: [
          "`str` is an owned string. Methods: `len` `is_empty` `contains` `split` `replace` `trim` `to_upper` `to_lower` `starts_with` `ends_with` `parse_i32`.",
          "`parse_i32` returns `Result[i32, str]`. `len` is UTF-8 bytes.",
        ],
        code: { title: "strings", lang: "rsd", text: strings },
      },
      {
        heading: "8. Vec",
        body: [
          "`vec(...)` builds a vector. `push` `pop` `clear` `reverse` mutate in place and need a variable receiver.",
          "`xs[i]` indexes with an integer; out of bounds is a runtime error. `join(sep)` works on `Vec[str]` only.",
        ],
        code: { title: "Vec", lang: "rsd", text: vecs },
      },
      {
        heading: "9. Map",
        body: [
          "`map()` takes no arguments. Write the value type as `Map[V]`. Keys are always `str`. Use `.set` and `.get` (an `Option`); not `m[\"k\"]`.",
          "Also `keys()` `contains` `len` `is_empty` `clear`.",
        ],
        code: { title: "Map", lang: "rsd", text: maps },
      },
      {
        heading: "10. Structs and impl",
        body: [
          "Named fields: `struct Point { x: i32 y: i32 }`. Build with `Point { x: 3, y: 4 }`. Every field is required.",
          "`impl Point { ... }` adds methods. No inheritance, no constructor keyword, no private fields.",
        ],
        code: { title: "struct / impl", lang: "rsd", text: structs },
      },
      {
        heading: "11. Enums and match",
        body: [
          "`enum Color { Red Rgb(i32, i32, i32) }`. Values: `Color::Red` or `Color::Rgb(1,2,3)`.",
          "`match` uses `=>`. Built-in `Some` `None` `Ok` `Err` can be written without a path. Patterns: `_`, ints, bools, strings. No guards. For a custom enum write `match (value) { Color::Red => ... }`. `match value { Color::... }` is parsed as a struct literal because of `Color:`.",
        ],
        code: { title: "enum / match", lang: "rsd", text: enums },
      },
      {
        heading: "12. Option, Result, ?",
        body: [
          "`Option[T]`: `Some(v)` / `None`. `Result[T,E]`: `Ok(v)` / `Err(e)`.",
          "In a function that returns `Result`, `expr?` forwards `Err`. `?` does not work on `Option`. Otherwise `match`, or stop with `panic` / `assert`.",
        ],
        code: { title: "Result and ?", lang: "rsd", text: result },
      },
      {
        heading: "13. Ownership and borrows",
        body: [
          "Non-Copy values (`str`, `Vec`, `Map`, structs, enums) move into calls. Copy types: `i32` `i64` `f64` `bool` `()`.",
          "Many `&T` at once; only one `&mut T`, and not together with shared borrows. Dereference with `*`. There is no lifetime syntax — storing a reference in a struct is rejected instead.",
        ],
        code: { title: "moves", lang: "rsd", text: ownership },
      },
      {
        heading: "14. Modules",
        body: [
          "`mod math` loads `math.rsd` or `math/mod.rsd` beside the current file and merges items. No separate namespace, no `pub`. Cycles are skipped.",
        ],
        code: { title: "mod", lang: "rsd", text: modules },
      },
      {
        heading: "15. What else you can do",
        body: [
          "Output: `print(...)`. Math: `abs` `min` `max` `sqrt`. Time: `now_ms()` `sleep_ms(n)`. Tests: `assert`. Abort: `panic(str)`.",
          "Threads: `spawn(\"name\")` returns an `i32` id; `join(id)` waits. FFI whitelist: `extern_call(\"sin\", x)`. Copy: `.clone()`.",
        ],
        code: { title: "built-ins", lang: "rsd", text: builtins },
      },
      {
        heading: "16. Files and HTTP",
        body: [
          "I/O returns `Result`. Paths and the network are sandboxed. Failures are `Err`. The next sections list the hard caps.",
        ],
        code: { title: "files / HTTP", lang: "rsd", text: io },
      },
      {
        heading: "17. Tests",
        body: [
          "`russd test file.rsd` runs every function whose name starts with `test_`. Use `assert`. A dummy `fn main()` is still typical.",
        ],
        code: { title: "test_", lang: "rsd", text: tests },
      },
      {
        heading: "18. Syntax limits (things that do not exist)",
        body: [
          "Russd looks like simplified Rust. These familiar pieces are missing on purpose. Read them before you fight the checker.",
        ],
        bullets: limitsSyntaxEn,
      },
      {
        heading: "19. Runtime and safety limits",
        body: ["These rules are enforced now. If it is not on the allowed list, you cannot do it yet."],
        bullets: limitsRuntimeEn,
      },
    ],
  },
  cn: {
    title: "如何使用、如何写程序",
    lead: "本页教 Russd 怎么写：语法、你实际能做的事，以及一长串刻意加上的限制。安装与下载见「下载核心」；项目配置见「如何配置」。",
    sections: [
      {
        heading: "1. 程序长什么样",
        body: [
          "源文件扩展名是 `.rsd`。`russd run` 需要 `fn main()`。分号可写可不写。块里最后一个表达式就是返回值。",
          "注释只有 `//`。关键字：`fn` `let` `mut` `if` `else` `while` `for` `in` `loop` `break` `continue` `struct` `enum` `match` `impl` `return` `true` `false` `self` `mod`。",
          "先跑起来：`cargo run -- examples/hello.rsd`（`--` 和文件名要分开；不要写成 `cargo run --run`，cargo 会报指令不存在）。程序参数再加一组 `--`，用 `args()` 读成 `Vec[str]`：`cargo run -- examples/hello.rsd -- foo`。",
        ],
        code: { title: "hello.rsd", lang: "rsd", text: hello },
      },
      {
        heading: "编译器会做优化",
        body: [
          "Russd 是优化编译器。默认 `-O2`：常数折叠、`x+0`/`x*1` 代数简化、`if true` 删死分支、小函数内联、没用到的 `let` 删除、bytecode peephole。",
          "`russd run f.rsd -O0` 关掉优化。`russd opt f.rsd` 打印折叠/内联/删码次数与 bytecode。溢出与除零不会折成回绕。后端仍是安全 VM，不是 LLVM 机器码。",
        ],
        code: {
          title: "看优化",
          lang: "bash",
          text: `russd opt examples/hello.rsd
russd run examples/fizzbuzz.rsd -O2
russd run examples/fizzbuzz.rsd -O0`,
        },
      },
      {
        heading: "2. 类型",
        body: [
          "内建类型：`i32` `i64` `f64` `bool` `str` `()` `Vec[T]` `Map[V]` `Option[T]` `Result[T,E]`。类型参数用方括号。",
          "整数字面量默认 `i32`。超出范围要写 `i64` 后缀。浮点要有小数点，例如 `3.14`。",
        ],
        code: { title: "类型", lang: "rsd", text: types },
      },
      {
        heading: "3. 变量",
        body: [
          "`let` 不可变；`let mut` 才能再赋值。可以写 `let n: i32 = 1`，也可以靠推断。",
          "赋值目标必须是变量、结构字段或 `Vec` 下标。",
        ],
        code: { title: "let / let mut", lang: "rsd", text: vars },
      },
      {
        heading: "4. 运算符",
        body: [
          "算术 `+ - * / %`（`+` 也可接字符串）。比较 `== != < <= > >=`。逻辑 `&& || !`。位运算 `& | ^ ~ << >>`。",
          "没有重载：不能把两个结构相加。比较目前只允许数字。",
        ],
        code: { title: "运算", lang: "rsd", text: ops },
      },
      {
        heading: "5. 控制流",
        body: [
          "`if` / `else if` / `else` 是表达式，分支要用 `{ }`。`while` 条件必须是 `bool`。",
          "`for x in xs` 可走 `Vec` 或范围 `1..n`（不含 n）。`loop` 搭配 `break` / `continue`。`break` 不能带值。",
        ],
        code: { title: "if / while / for / loop", lang: "rsd", text: control },
      },
      {
        heading: "6. 函数",
        body: [
          "写法：`fn 名称(参数: 类型) -> 返回类型 { ... }`。省略 `->` 就是 `()`。",
          "参数可为值、`mut x: T`、`&T`、`&mut T`。`impl` 里可用 `self`、`&self`、`&mut self`。没有默认参数、没有可变参数、没有泛型参数。可变状态主要靠 `let mut` 与 Vec/Map 方法；还不能用 `*x = ...` 写回引用。",
        ],
        code: { title: "fn", lang: "rsd", text: functions },
      },
      {
        heading: "7. 字符串",
        body: [
          "`str` 是拥有所有权的字符串。方法：`len` `is_empty` `contains` `split` `replace` `trim` `to_upper` `to_lower` `starts_with` `ends_with` `parse_i32`。",
          "`parse_i32` 返回 `Result[i32, str]`。`len` 是 UTF-8 字节数。",
        ],
        code: { title: "字符串方法", lang: "rsd", text: strings },
      },
      {
        heading: "8. 向量 Vec",
        body: [
          "`vec(...)` 建立向量。`push` `pop` `clear` `reverse` 会改原向量，接收者必须是变量。",
          "下标 `xs[i]` 只接受整数；越界是运行时错误。`join(sep)` 只适用于 `Vec[str]`。",
        ],
        code: { title: "Vec", lang: "rsd", text: vecs },
      },
      {
        heading: "9. 对照表 Map",
        body: [
          "`map()` 不接受参数。值类型写在 `Map[V]`。键永远是 `str`，用 `.set` 与 `.get`（返回 `Option`），不能 `m[\"k\"]`。",
          "还有 `keys()` `contains` `len` `is_empty` `clear`。",
        ],
        code: { title: "Map", lang: "rsd", text: maps },
      },
      {
        heading: "10. 结构与 impl",
        body: [
          "具名字段：`struct Point { x: i32 y: i32 }`。构造：`Point { x: 3, y: 4 }`。字段必须一次给齐。",
          "`impl Point { ... }` 给方法。没有继承、没有构造函数关键字、没有私有字段。",
        ],
        code: { title: "struct / impl", lang: "rsd", text: structs },
      },
      {
        heading: "11. 枚举与 match",
        body: [
          "`enum Color { Red Rgb(i32, i32, i32) }`。值写 `Color::Red` 或 `Color::Rgb(1,2,3)`。",
          "`match` 用 `=>`。内建 `Some` `None` `Ok` `Err` 可直接写。也可用 `_`、整数、布尔、字符串模式。没有 guard。自定义枚举请写 `match (值) { Color::Red => ... }`：`match 值 { Color::... }` 会被当成结构字面量。",
        ],
        code: { title: "enum / match", lang: "rsd", text: enums },
      },
      {
        heading: "12. Option、Result、?",
        body: [
          "`Option[T]`：`Some(v)` / `None`。`Result[T,E]`：`Ok(v)` / `Err(e)`。",
          "在返回 `Result` 的函数里，`expr?` 遇到 `Err` 会立刻返回。`?` 不能用在 `Option`。否则 `match`，或 `panic` / `assert`。",
        ],
        code: { title: "Result 与 ?", lang: "rsd", text: result },
      },
      {
        heading: "13. 所有权与借用",
        body: [
          "非 Copy 值（`str`、`Vec`、`Map`、结构、枚举）传进函数会被移动。Copy 的有 `i32` `i64` `f64` `bool` `()`。",
          "`&T` 可同时有多个；`&mut T` 同时只能一个。解引用用 `*`。没有生命周期语法——编译器直接禁止把引用放进结构。",
        ],
        code: { title: "移动", lang: "rsd", text: ownership },
      },
      {
        heading: "14. 模块",
        body: [
          "`mod math` 会加载同目录 `math.rsd` 或 `math/mod.rsd`，把里面的项目合并进当前程序。没有独立命名空间、没有 `pub`。",
        ],
        code: { title: "mod", lang: "rsd", text: modules },
      },
      {
        heading: "15. 你还能做的事（内建）",
        body: [
          "输出：`print(...)`。数学：`abs` `min` `max` `sqrt`。时间：`now_ms()` `sleep_ms(n)`。测试：`assert`。中止：`panic(str)`。",
          "线程：`spawn(\"函数名\")` 回 `i32` id，再用 `join(id)`。FFI 白名单：`extern_call(\"sin\", x)`。复制：`.clone()`。",
        ],
        code: { title: "内建", lang: "rsd", text: builtins },
      },
      {
        heading: "16. 文件与 HTTP",
        body: [
          "I/O 返回 `Result`。路径与网络都有沙盒。细节见下一节限制清单。",
        ],
        code: { title: "文件 / HTTP", lang: "rsd", text: io },
      },
      {
        heading: "17. 测试",
        body: [
          "`russd test 文件.rsd` 会执行所有名称以 `test_` 开头的函数。用 `assert` 检查。通常仍放一个空的 `fn main()`。",
        ],
        code: { title: "test_", lang: "rsd", text: tests },
      },
      {
        heading: "18. 语法限制（刻意没有的东西）",
        body: ["Russd 看起来像简化的 Rust，但下面这些写法这里都没有。写之前先看完。"],
        bullets: limitsSyntaxCn,
      },
      {
        heading: "19. 运行时与安全限制",
        body: ["这些规则现在就会执行。清单以外的事情，目前就是不行。"],
        bullets: limitsRuntimeCn,
      },
    ],
  },
};
