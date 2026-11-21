# Russd

開源、語法接近 Rust、刻意更好學的程式語言。核心編譯器與執行器都在這個倉庫：[huhm-cc/russd-core](https://github.com/huhm-cc/russd-core)。

> v0 是**帶靜態檢查的解釋器**。它比草稿腳本嚴謹，但**還沒有** LLVM 原生後端，所以不會比 rustc 更快。安全預設（無 `unsafe`、無生命週期語法、溢位/越界即錯誤）是現在就能驗收的部分。

授權：**GNU GPL v3**（與本倉庫 `LICENSE` 相同，完全開源、copyleft）。

## 快速開始

```bash
git clone https://github.com/huhm-cc/russd-core
cd russd-core
cargo run -- run examples/hello.rsd
```

其他指令：

```bash
cargo run -- check examples/hello.rsd
cargo run -- run examples/fizzbuzz.rsd
cargo run --            # REPL
cargo test
```

安裝 CLI：

```bash
cargo install --path .
russd run examples/structs.rsd
```

## 語言長什麼樣子

```rsd
fn greet(name: str) -> str {
    "Hello, " + name
}

fn main() {
    let mut n = 0
    while n < 3 {
        print(greet("Russd"))
        n = n + 1
    }
}
```

更多說明見 [spec/overview.md](spec/overview.md)。

## 設計原則

| 目標 | v0 怎麼做 |
|------|-----------|
| 易懂、好學 | 語法像簡化 Rust；診斷用中文 + 修法建議 |
| 更安全 | 沒有 `unsafe`、結構不能存引用、溢位/除零/越界都是錯誤 |
| 更好用 | 不寫 `'a`；分號可省略 |
| 更快 | 解釋器先能跑；HIR 預留給之後的 LLVM 後端 |

## 倉庫結構

- `crates/russd-lang` — lexer、parser、類型/所有權檢查、解釋器
- `src/main.rs` — `russd` CLI
- `examples/` — 可執行範例
- `spec/` — 給學習者的一頁說明

## 貢獻

歡迎開 issue 與 pull request。請維持 GPL-3.0，不要加入 `unsafe` 逃生艙或生命週期語法，除非先有完整設計討論。
