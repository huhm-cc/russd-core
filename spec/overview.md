# Russd 語言一覽（v0）

Russd 看起來像簡化的 Rust：`fn`、`let` / `let mut`、結構、列舉、`match`。

## 與 Rust 的差別

- **沒有生命週期標註** `'a`。借用只活在當下這句；不能把 `&T` 放進結構或當函式回傳值。
- **沒有 `unsafe`**。沒有未定義行為入口。
- **溢位與除零是錯誤**（不是靜默 wrap）。
- **越界索引是錯誤**。
- 分號可省略，換行即可分隔語句。

## 型別

`i32` `i64` `bool` `str` `()` `Vec[T]` `Option[T]` `Result[T, E]`

## 所有權

- 預設移動（`str`、`Vec`、結構、列舉）。
- `i32` `i64` `bool` `()` 是 Copy。
- `&x` 共享借用、`&mut x` 獨佔借用；不能同時存在。
- `.clone()` 複製擁有值。

## 內建

- `print(...)`
- `vec(...)` / `let xs: Vec[i32] = vec()`
- `xs.push` `xs.pop` `xs.len` `s.len`

## 還沒有（有意延後）

trait、async、巨集、crate 生態、LLVM 原生後端。v0 是解釋器，**不是**世界上最快的實作；IR 預留給之後的 LLVM 階段。
