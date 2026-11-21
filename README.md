# Russd

開源、語法接近 Rust 的靜態型別語言。核心在 [huhm-cc/russd-core](https://github.com/huhm-cc/russd-core)（**GPL-3.0**）。

v0.3 是最佳化編譯器：型別檢查之後做常數摺疊、內聯、死碼刪除，再產出 bytecode，由安全 VM 執行。它**不是** rustc/LLVM 等級的原生碼編譯器。

## 安裝

```bash
git clone https://github.com/huhm-cc/russd-core
cd russd-core
cargo test --workspace
cargo run -- run examples/hello.rsd
cargo run -- run examples/app.rsd
cargo run -- test examples/lang_test.rsd
cargo run -- opt examples/hello.rsd
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
| `russd run f.rsd [-O0|-O1|-O2] -- args` | 執行（預設 O2 最佳化） |
| `russd opt f.rsd` | 最佳化統計 + bytecode |
| `russd check` | 只做 lex/parse/type/borrow |
| `russd test` | 跑 `test_*` |
| `russd fmt` | 格式化 |
| `russd disasm` | 印最佳化後 bytecode |
| `russd complete` | 符號補全 |
| `russd pkg init` | 寫 `russd.toml` |

編輯器：把 `editors/russd.tmLanguage.json` 加到支援 TextMate grammar 的 IDE。

## 說明網站

文件站在 `web/`（Next.js）。

- `/howto-use/{zh,en,cn}` 如何使用（語法、能做的事、限制）
- `/howto-configuration/{zh,en,cn}` 如何設定
- `/download-core/{zh,en,cn}` 下載核心
- `/opensource` 開源授權

本機：

```bash
cd web
npm install
npm run dev
```
