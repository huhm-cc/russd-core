# Russd 語言一覽（v0.2）

Russd 看起來像簡化的 Rust：`fn`、`let` / `let mut`、結構、列舉、`match`。

核心管線：`原始碼 → Token（lexer）→ AST（parser）→ 型別/所有權檢查 → bytecode 產物 + 執行期 VM`。

## 1. 基礎運算與控制流程

- 型別：`i32` `i64` `f64` `bool` `str` `()` `Vec[T]` `Map[V]` `Option[T]` `Result[T,E]`
- 控制：循序、`if/else`、`while`、`for x in xs`、`loop`/`break`/`continue`、`match`
- 運算：算術 `+ - * / %`、複合賦值 `+= -= *=`、比較、邏輯 `&& || !`、位元 `& | ^ ~ << >>`

## 2. 抽象化與模組化

- 函式、區域/參數作用域、`impl` 方法（結構當「類別」、方法當行為）
- 介面：以 `impl Type { fn ... }` 為契約；不提供 class 繼承，改用組合（欄位嵌套結構）
- 高階：函式當值尚在演進；`spawn("fn_name")` 把具名函式丟到另一條執行緒
- 模組：`mod math` 載入同目錄 `math.rsd`；套件清單 `russd.toml`，`russd pkg init`

## 3. 記憶體與執行期

- 所有權：移動、`&` / `&mut`、沒有 `unsafe`、沒有生命週期語法；結構不能存引用
- 溢位/除零/越界 = 錯誤；`Result`/`Option`/`?`/`panic`/`assert`
- 並行：`spawn` / `join`（OS thread）；非同步 I/O：`http_get`、`read_file`/`write_file`、`sleep_ms`

## 4. 編譯器核心

- lexer / parser / AST / 語意與型別檢查
- `russd disasm` 產出函式 bytecode；執行期是安全的堆疊/環境 VM（Rust 實作）

## 5. 標準庫與跨平台

- 字串：`len` `split` `replace` `trim` `to_upper` `to_lower` `starts_with` `ends_with` `parse_i32` `contains` `join`
- 集合：`vec`/`map`、`push` `pop` `set` `get` `keys` `clear` `reverse`
- 數學：`abs` `min` `max` `sqrt`；FFI：`extern_call("sin"|"cos"|"sqrt"|"pow"|...)`
- 時間：`now_ms` `sleep_ms`
- 檔案 / 網路：`read_file` `write_file` `http_get`（HTTP/1.0，無 TLS）
- 執行期可攜 Linux/macOS/Windows（只要有 Rust）；FFI 走宿主白名單而非任意 dlopen

## 6. 開發者工具

- `russd` REPL（會累積 `fn`/`struct`）
- `russd test`：執行名稱為 `test_*` 的函式
- `russd fmt` 格式化
- `russd complete` 給編輯器補全
- `editors/russd.tmLanguage.json` 語法突顯
- 套件：`russd pkg`

## 刻意還沒有

完整 trait 物件、class 繼承、LLVM 機器碼、TLS HTTPS、任意動態庫 FFI、圖形除錯器 GUI。
這些列在路線圖，不假裝已經是「世界最快」。
