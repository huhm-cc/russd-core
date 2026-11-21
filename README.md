# Russd

開源、語法接近 Rust 的靜態型別語言。核心在 [huhm-cc/russd-core](https://github.com/huhm-cc/russd-core)（**GPL-3.0**）。

v0.2 覆蓋一門「能認真寫程式」的語言該有的六層（運算與控制流、模組化、執行期安全、編譯器管線、標準庫、開發工具）。它**不是** rustc/LLVM 等級的最佳化編譯器。

## 安裝

```bash
git clone https://github.com/huhm-cc/russd-core
cd russd-core
cargo test --workspace
cargo run -- run examples/hello.rsd
cargo run -- run examples/app.rsd
cargo run -- test examples/lang_test.rsd
cargo run -- disasm examples/hello.rsd
```

```bash
cargo install --path .
russd run examples/fizzbuzz.rsd
russd --help
```

## 你能寫什麼

```rsd
mod math

fn main() {
    let mut xs: Vec[i32] = vec()
    xs.push(1)
    xs.push(2)
    print("sum", add(xs[0], xs[1]))
    print(1 << 3, 6 & 3, ~0)
}
```

完整對照見 [spec/overview.md](spec/overview.md)。

## 指令

| 指令 | 用途 |
|------|------|
| `russd run f.rsd -- args` | 執行（支援 `mod`、命令列參數 `args()`） |
| `russd check` | 只做 lex/parse/type/borrow |
| `russd test` | 跑 `test_*` |
| `russd fmt` | 格式化 |
| `russd disasm` | 印 bytecode |
| `russd complete` | 符號補全 |
| `russd pkg init` | 寫 `russd.toml` |

編輯器：把 `editors/russd.tmLanguage.json` 加到支援 TextMate grammar 的 IDE。
