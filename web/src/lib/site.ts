import type { Doc, Lang } from "./doc-types";
export type { Doc, Lang } from "./doc-types";

export const LANGS: { id: Lang; label: string }[] = [
  { id: "zh", label: "繁體中文" },
  { id: "en", label: "English" },
  { id: "cn", label: "简体中文" },
];

export function isLang(s: string): s is Lang {
  return s === "zh" || s === "en" || s === "cn";
}

export const GITHUB_CORE = "https://github.com/huhm-cc/russd-core";
export const ORIGIN_CORE = "https://cursor.com/codebase/lan-b3w0h/russd-core";

export const NAV = [
  { href: "/howto-use", key: "use" as const },
  { href: "/howto-configuration", key: "config" as const },
  { href: "/download-core", key: "download" as const },
  { href: "/opensource", key: "oss" as const },
];

export const ui = {
  zh: {
    brand: "Russd",
    tagline: "開源、靜態型別、刻意更安全的 Rust 風格語言",
    use: "如何使用",
    config: "如何設定",
    download: "下載核心",
    oss: "開源",
    home: "首頁",
    copy: "複製",
    copied: "已複製",
    lang: "語言",
  },
  en: {
    brand: "Russd",
    tagline: "An open-source, statically typed, Rust-like language with safer defaults",
    use: "How to use",
    config: "How to configure",
    download: "Download core",
    oss: "Open source",
    home: "Home",
    copy: "Copy",
    copied: "Copied",
    lang: "Language",
  },
  cn: {
    brand: "Russd",
    tagline: "开源、静态类型、刻意更安全的 Rust 风格语言",
    use: "如何使用",
    config: "如何配置",
    download: "下载核心",
    oss: "开源",
    home: "首页",
    copy: "复制",
    copied: "已复制",
    lang: "语言",
  },
};

export { howtoUse } from "./howto-use";

export const howtoConfig: Record<Lang, Doc> = {
  zh: {
    title: "如何設定 Russd",
    lead: "專案用 russd.toml 描述套件；編輯器可用 complete 與 TextMate 語法檔。執行期會限制檔案與網路，避免腳本任意讀寫系統。",
    sections: [
      {
        heading: "russd.toml",
        body: ["在倉庫根目錄執行 russd pkg init 會寫入套件清單。這是套件名稱、版本與依賴的起點。"],
        code: {
          title: "russd.toml",
          lang: "toml",
          text: `[package]
name = "russd-core"
version = "0.2.0"
license = "GPL-3.0-only"

[dependencies]`,
        },
      },
      {
        heading: "編輯器",
        body: [
          "把 editors/russd.tmLanguage.json 加到支援 TextMate grammar 的編輯器，即可突顯 .rsd。",
          "russd complete path.rsd 會列出關鍵字與符號，可接到自動補全。",
        ],
      },
      {
        heading: "安全預設（執行期）",
        body: [
          "read_file / write_file 只能存取目前工作目錄之內的路徑，拒絕 .. 與絕對路徑逃逸。",
          "http_get 只允許 http://，拒絕私有網段（SSRF），並限制逾時與回應大小。",
          "spawn 只接受合法函式識別字。sleep_ms 有上限。寫入檔案有大小上限。",
          "語言沒有 unsafe。結構不能存放引用。溢位預設是錯誤。",
        ],
      },
      {
        heading: "說明網站",
        body: [
          "說明網站在 web/，以 Next.js 建置。回應帶 Content-Security-Policy、X-Content-Type-Options、Referrer-Policy 等安全標頭。",
        ],
      },
    ],
  },
  en: {
    title: "How to configure Russd",
    lead: "Describe packages with russd.toml, wire the editor, and rely on runtime sandboxing for files and HTTP.",
    sections: [
      {
        heading: "russd.toml",
        body: ["Run russd pkg init at the repo root to write a package manifest."],
        code: {
          title: "russd.toml",
          lang: "toml",
          text: `[package]
name = "russd-core"
version = "0.2.0"
license = "GPL-3.0-only"

[dependencies]`,
        },
      },
      {
        heading: "Editor",
        body: [
          "Add editors/russd.tmLanguage.json to a TextMate-compatible editor for .rsd highlighting.",
          "russd complete path.rsd prints keywords and symbols for autocomplete.",
        ],
      },
      {
        heading: "Safe defaults",
        body: [
          "read_file / write_file stay inside the current working directory; .. and absolute escapes are rejected.",
          "http_get allows only http://, blocks private networks (SSRF), and caps timeout and body size.",
          "spawn accepts identifier names only. sleep_ms and file writes are capped.",
          "No unsafe. Structs cannot store references. Overflow is an error by default.",
        ],
      },
      {
        heading: "Website",
        body: [
          "Docs live in web/ (Next.js). Responses include CSP and related security headers.",
        ],
      },
    ],
  },
  cn: {
    title: "如何配置 Russd",
    lead: "用 russd.toml 描述套件；编辑器可用 complete 与 TextMate 语法文件。运行时会限制文件与网络。",
    sections: [
      {
        heading: "russd.toml",
        body: ["在仓库根目录运行 russd pkg init 会写入套件清单。"],
        code: {
          title: "russd.toml",
          lang: "toml",
          text: `[package]
name = "russd-core"
version = "0.2.0"
license = "GPL-3.0-only"

[dependencies]`,
        },
      },
      {
        heading: "编辑器",
        body: ["把 editors/russd.tmLanguage.json 加到支持 TextMate 的编辑器。russd complete 可对接补全。"],
      },
      {
        heading: "安全默认",
        body: [
          "文件读写限制在工作目录内。http_get 拒绝私有网段。spawn 只接受合法函数名。",
          "语言没有 unsafe。溢出默认是错误。",
        ],
      },
      {
        heading: "说明网站",
        body: ["文档站点在 web/，以 Next.js 建置，回应带安全标头。"],
      },
    ],
  },
};

export const downloadCore: Record<Lang, Doc> = {
  zh: {
    title: "下載 Russd 核心",
    lead: "核心原始碼在 huhm-cc/russd-core。請用 git 取得原始碼並自行編譯，這是目前唯一支援的發行方式。",
    sections: [
      {
        heading: "GitHub（原始碼）",
        body: [
          "倉庫：https://github.com/huhm-cc/russd-core",
          "授權：GNU GPL v3。克隆後請閱讀 LICENSE。",
          "`git clone` 會在目前目錄下建立 `russd-core/`。接下來的 `cargo` 必須在那個資料夾裡跑。若出現 `could not find Cargo.toml`，代表還在上一層（例如 `D:\\test`），請先 `cd russd-core`。",
        ],
        code: {
          title: "git clone（PowerShell / bash）",
          lang: "bash",
          text: `git clone https://github.com/huhm-cc/russd-core
cd russd-core
cargo test --workspace
cargo run -- examples/hello.rsd
cargo build --release
.\\target\\release\\russd.exe run examples/hello.rsd`,
        },
      },
      {
        heading: "Origin CLI",
        body: ["若你使用 Cursor Origin 上的副本，可用 Origin CLI 克隆。安裝後若找不到 origin，把 ~/.local/bin 加進 PATH。"],
        code: {
          title: "Origin",
          lang: "bash",
          text: `curl -fsSL https://downloads.cursor.com/origin/install.sh | sh
origin auth login
origin repo clone lan-b3w0h/russd-core`,
        },
      },
      {
        heading: "驗證",
        body: ["編譯完成後請跑 cargo test --workspace，並執行 examples/hello.rsd。不要執行來路不明的 .rsd 腳本去讀寫家目錄以外的路徑——執行期已阻擋，但仍請只跑你信任的程式。"],
      },
    ],
  },
  en: {
    title: "Download Russd core",
    lead: "Source lives at huhm-cc/russd-core. Clone and build it yourself — that is the supported release channel.",
    sections: [
      {
        heading: "GitHub (source)",
        body: [
          "Repository: https://github.com/huhm-cc/russd-core. License: GNU GPL v3.",
          "`git clone` creates a `russd-core/` folder. Run every `cargo` command inside that folder. `could not find Cargo.toml` means you are still in the parent directory (for example `D:\\test`); `cd russd-core` first.",
        ],
        code: {
          title: "git clone (PowerShell / bash)",
          lang: "bash",
          text: `git clone https://github.com/huhm-cc/russd-core
cd russd-core
cargo test --workspace
cargo run -- examples/hello.rsd
cargo build --release
.\\target\\release\\russd.exe run examples/hello.rsd`,
        },
      },
      {
        heading: "Origin CLI",
        body: ["A Cursor Origin copy can be cloned with the Origin CLI. If origin is missing after install, add ~/.local/bin to PATH."],
        code: {
          title: "Origin",
          lang: "bash",
          text: `curl -fsSL https://downloads.cursor.com/origin/install.sh | sh
origin auth login
origin repo clone lan-b3w0h/russd-core`,
        },
      },
      {
        heading: "Verify",
        body: ["Run cargo test --workspace and examples/hello.rsd. Only run .rsd programs you trust."],
      },
    ],
  },
  cn: {
    title: "下载 Russd 核心",
    lead: "核心源码在 huhm-cc/russd-core。请用 git 获取并自行编译，这是目前唯一支持的发行方式。",
    sections: [
      {
        heading: "GitHub（源码）",
        body: [
          "仓库：https://github.com/huhm-cc/russd-core。许可证：GNU GPL v3。",
          "`git clone` 会在当前目录下创建 `russd-core/`。之后的 `cargo` 必须在那个文件夹里运行。若出现 `could not find Cargo.toml`，说明还停在上一层（例如 `D:\\test`），请先 `cd russd-core`。",
        ],
        code: {
          title: "git clone（PowerShell / bash）",
          lang: "bash",
          text: `git clone https://github.com/huhm-cc/russd-core
cd russd-core
cargo test --workspace
cargo run -- examples/hello.rsd
cargo build --release
.\\target\\release\\russd.exe run examples/hello.rsd`,
        },
      },
      {
        heading: "Origin CLI",
        body: ["Cursor Origin 上的副本可用 Origin CLI 克隆。"],
        code: {
          title: "Origin",
          lang: "bash",
          text: `curl -fsSL https://downloads.cursor.com/origin/install.sh | sh
origin auth login
origin repo clone lan-b3w0h/russd-core`,
        },
      },
      {
        heading: "验证",
        body: ["请运行 cargo test --workspace，并执行 examples/hello.rsd。"],
      },
    ],
  },
};

export const opensource = {
  title: "Open source",
  license: "GNU General Public License v3.0",
  repo: GITHUB_CORE,
  points: [
    {
      zh: "核心編譯器、標準庫行為與本說明站原始碼都在同一倉庫，授權 GPL-3.0。",
      en: "The compiler, runtime behavior, and this documentation site live in the same repository under GPL-3.0.",
      cn: "核心编译器、运行时与本文档站点源码都在同一仓库，许可证为 GPL-3.0。",
    },
    {
      zh: "歡迎 issue 與 pull request。請不要加入 unsafe 逃生艙或生命週期語法，除非先有設計討論。",
      en: "Issues and pull requests are welcome. Do not add unsafe escapes or lifetime syntax without a design discussion.",
      cn: "欢迎 issue 与 pull request。请不要在没有设计讨论的情况下加入 unsafe 或生命周期语法。",
    },
    {
      zh: "原始碼在 huhm-cc/russd-core。歡迎閱讀 LICENSE 後再改、再發佈。",
      en: "Source is at huhm-cc/russd-core. Read LICENSE before modifying or redistributing.",
      cn: "源码在 huhm-cc/russd-core。请先阅读 LICENSE 再修改或再发布。",
    },
  ],
};
