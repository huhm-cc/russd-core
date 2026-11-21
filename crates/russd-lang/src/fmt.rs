use crate::lexer::lex;
use crate::token::TokenKind;

/// Lightweight formatter: keeps tokens, inserts newlines after `{` / `;` semantics
/// and indents by brace depth.
pub fn format_source(source: &str) -> Result<String, String> {
    let tokens = lex(source).map_err(|d| d.message)?;
    let mut out = String::new();
    let mut depth = 0i32;
    let mut need_nl = false;
    for tok in tokens {
        if matches!(tok.kind, TokenKind::Eof) {
            break;
        }
        if matches!(tok.kind, TokenKind::RBrace) {
            depth -= 1;
            out.push('\n');
            for _ in 0..depth.max(0) {
                out.push_str("    ");
            }
            out.push('}');
            need_nl = true;
            continue;
        }
        if need_nl {
            out.push('\n');
            for _ in 0..depth.max(0) {
                out.push_str("    ");
            }
            need_nl = false;
        } else if !out.is_empty() && !out.ends_with(' ') && !out.ends_with('(') {
            match tok.kind {
                TokenKind::Comma | TokenKind::Semi | TokenKind::Dot | TokenKind::RParen => {}
                _ => {
                    if !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
            }
        }
        match &tok.kind {
            TokenKind::LBrace => {
                out.push('{');
                depth += 1;
                need_nl = true;
            }
            TokenKind::Semi => {
                out.push(';');
                need_nl = true;
            }
            other => out.push_str(&display_token(other)),
        }
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn display_token(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Ident(s) => s.clone(),
        TokenKind::Int { value, suffix_i64 } => {
            if *suffix_i64 {
                format!("{value}i64")
            } else {
                format!("{value}")
            }
        }
        TokenKind::Float(f) => format!("{f}"),
        TokenKind::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        TokenKind::Fn => "fn".into(),
        TokenKind::Let => "let".into(),
        TokenKind::Mut => "mut".into(),
        TokenKind::If => "if".into(),
        TokenKind::Else => "else".into(),
        TokenKind::While => "while".into(),
        TokenKind::For => "for".into(),
        TokenKind::In => "in".into(),
        TokenKind::Struct => "struct".into(),
        TokenKind::Enum => "enum".into(),
        TokenKind::Match => "match".into(),
        TokenKind::Impl => "impl".into(),
        TokenKind::Return => "return".into(),
        TokenKind::True => "true".into(),
        TokenKind::False => "false".into(),
        TokenKind::SelfValue => "self".into(),
        TokenKind::Mod => "mod".into(),
        TokenKind::Break => "break".into(),
        TokenKind::Continue => "continue".into(),
        TokenKind::Loop => "loop".into(),
        TokenKind::Plus => "+".into(),
        TokenKind::PlusEq => "+=".into(),
        TokenKind::Minus => "-".into(),
        TokenKind::MinusEq => "-=".into(),
        TokenKind::Star => "*".into(),
        TokenKind::StarEq => "*=".into(),
        TokenKind::Slash => "/".into(),
        TokenKind::Percent => "%".into(),
        TokenKind::Eq => "=".into(),
        TokenKind::EqEq => "==".into(),
        TokenKind::Bang => "!".into(),
        TokenKind::BangEq => "!=".into(),
        TokenKind::Lt => "<".into(),
        TokenKind::LtEq => "<=".into(),
        TokenKind::Gt => ">".into(),
        TokenKind::GtEq => ">=".into(),
        TokenKind::And => "&".into(),
        TokenKind::AndAnd => "&&".into(),
        TokenKind::Pipe => "|".into(),
        TokenKind::OrOr => "||".into(),
        TokenKind::Caret => "^".into(),
        TokenKind::Tilde => "~".into(),
        TokenKind::Shl => "<<".into(),
        TokenKind::Shr => ">>".into(),
        TokenKind::LParen => "(".into(),
        TokenKind::RParen => ")".into(),
        TokenKind::LBrace => "{".into(),
        TokenKind::RBrace => "}".into(),
        TokenKind::LBracket => "[".into(),
        TokenKind::RBracket => "]".into(),
        TokenKind::Comma => ",".into(),
        TokenKind::Dot => ".".into(),
        TokenKind::Colon => ":".into(),
        TokenKind::Semi => ";".into(),
        TokenKind::Arrow => "->".into(),
        TokenKind::FatArrow => "=>".into(),
        TokenKind::Question => "?".into(),
        TokenKind::DotDot => "..".into(),
        TokenKind::Eof => "".into(),
    }
}
