use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

pub const MIN_PORT: i32 = 1024;
pub const MAX_PORT: i32 = 49151;
pub const MAX_HEADER_BYTES: usize = 8192;
pub const MAX_BODY_BYTES: usize = 65_536;

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: String,
    pub body: String,
}

pub fn url_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let h = hex_nibble(bytes[i + 1]);
                let l = hex_nibble(bytes[i + 2]);
                if let (Some(h), Some(l)) = (h, l) {
                    out.push(char::from(h * 16 + l));
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub fn bind_local(port: i32) -> Result<TcpListener, String> {
    if port < MIN_PORT || port > MAX_PORT {
        return Err(format!(
            "http_serve 只接受連接埠 {MIN_PORT}–{MAX_PORT}（拒絕特權埠與動態埠）"
        ));
    }
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).map_err(|e| format!("無法綁定 {addr}: {e}"))?;
    let _ = listener.set_nonblocking(false);
    Ok(listener)
}

pub fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > MAX_HEADER_BYTES + MAX_BODY_BYTES {
            return Err("請求太大".into());
        }
        if let Some(pos) = find_headers_end(&buf) {
            let header_text = String::from_utf8_lossy(&buf[..pos]).into_owned();
            let mut content_len = 0usize;
            for line in header_text.split('\n') {
                let line = line.trim_end_matches('\r');
                let lower = line.to_ascii_lowercase();
                if let Some(rest) = lower.strip_prefix("content-length:") {
                    content_len = rest.trim().parse().unwrap_or(0);
                }
            }
            if content_len > MAX_BODY_BYTES {
                return Err("請求本文太大".into());
            }
            let mut body = buf[pos + 4..].to_vec();
            while body.len() < content_len {
                let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&tmp[..n]);
                if body.len() > MAX_BODY_BYTES {
                    return Err("請求本文太大".into());
                }
            }
            body.truncate(content_len);
            return parse_request(&header_text, &body);
        }
        if n < tmp.len() && buf.len() > MAX_HEADER_BYTES {
            return Err("標頭不完整或過長".into());
        }
    }
    Err("連線在讀完請求前關閉".into())
}

fn find_headers_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_request(headers: &str, body: &[u8]) -> Result<HttpRequest, String> {
    let first = headers.lines().next().unwrap_or("").trim();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_ascii_uppercase();
    if method != "GET" && method != "POST" && method != "HEAD" {
        return Err(format!("不支援的 HTTP 方法 {method}"));
    }
    let raw_path = parts.next().unwrap_or("/");
    if raw_path.len() > 1024 || raw_path.contains('\0') {
        return Err("路徑無效".into());
    }
    let (path, query) = match raw_path.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (raw_path.to_string(), String::new()),
    };
    if !path.starts_with('/') {
        return Err("路徑必須以 / 開頭".into());
    }
    let body = String::from_utf8_lossy(body).into_owned();
    Ok(HttpRequest {
        method,
        path,
        query,
        body,
    })
}

pub fn write_response(stream: &mut TcpStream, path: &str, body: &str) -> Result<(), String> {
    let status = if path == "/favicon.ico" {
        "404 Not Found"
    } else {
        "200 OK"
    };
    let ctype = if path.starts_with("/api/") {
        "application/json; charset=utf-8"
    } else {
        "text/html; charset=utf-8"
    };
    let bytes = body.as_bytes();
    let header = format!(
        "HTTP/1.0 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\n\r\n",
        bytes.len()
    );
    stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
    if path != "/favicon.ico" {
        stream.write_all(bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn write_error(stream: &mut TcpStream, message: &str) {
    let body = format!("<pre>{}</pre>", html_escape_min(message));
    let _ = write_response(stream, "/error", &body);
}

fn html_escape_min(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_plus_and_percent() {
        assert_eq!(url_decode("Ada+Lovelace"), "Ada Lovelace");
        assert_eq!(url_decode("%3Cscript%3E"), "<script>");
    }
}
