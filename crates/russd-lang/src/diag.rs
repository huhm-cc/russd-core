use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            hint: None,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render(&self, filename: &str, source: &str) -> String {
        let line = line_text(source, self.span.line);
        let col = self.span.col.max(1);
        let mut caret = String::new();
        for _ in 1..col {
            caret.push(' ');
        }
        caret.push('^');
        let mut out = format!(
            "{}:{}:{}: 錯誤：{}\n{}\n{}",
            filename, self.span.line, col, self.message, line, caret
        );
        if let Some(hint) = &self.hint {
            out.push_str("\n建議：");
            out.push_str(hint);
        }
        out
    }
}

fn line_text(source: &str, line: usize) -> String {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .to_string()
}

pub fn render_all(filename: &str, source: &str, diags: &[Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| d.render(filename, source))
        .collect::<Vec<_>>()
        .join("\n\n")
}
