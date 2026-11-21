# Russd

開源、語法接近 Rust 的靜態型別語言。核心在 [huhm-cc/russd-core](https://github.com/huhm-cc/russd-core)（**GPL-3.0**）。

v0.3 是最佳化編譯器：型別檢查之後做常數摺疊、內聯、死碼刪除，再產出 bytecode，由安全 VM 執行。它**不是** rustc/LLVM 等級的原生碼編譯器。

## 安裝

`cargo` 一定要在 **倉庫根目錄**（有 `Cargo.toml` 的那個資料夾）執行。`git clone` 之後會多一層 `russd-core/`；若提示 `could not find Cargo.toml`，代表還停在上一層（例如 `D:\test`），請先 `cd russd-core`。

PowerShell / cmd / bash 都一樣：

```powershell
git clone https://github.com/huhm-cc/russd-core
cd russd-core
cargo test --workspace
cargo run -- examples/hello.rsd
cargo run -- examples/app.rsd
cargo run -- test examples/lang_test.rsd
cargo run -- opt examples/hello.rsd
cargo run -- disasm examples/hello.rsd
```

`cargo run` 後面的 `--` 與檔名之間要有空白。不要寫 `cargo run --run`（Cargo 會當成自己的旗標）。
`cargo run -- run examples/hello.rsd` 也可以，但必須先 `cd` 進這個倉庫。

用 Russd 寫的小網站（路由、FizzBuzz、留言板、JSON）：

```powershell
cd russd-core
cargo run -- examples/site/main.rsd
```

然後用瀏覽器開 `http://127.0.0.1:4371/`。只聽本機。換埠：`cargo run -- examples/site/main.rsd -- 4371`。

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
