use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};

pub fn safe_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_') && name.len() <= 64
}

pub fn sandbox_path(raw: &str) -> Result<PathBuf, String> {
    if raw.is_empty() || raw.len() > 4096 {
        return Err("路徑無效".into());
    }
    if raw.contains('\0') {
        return Err("路徑含有非法字元".into());
    }
    let p = Path::new(raw);
    if p.is_absolute() {
        return Err("禁止絕對路徑".into());
    }
    for c in p.components() {
        match c {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("禁止路徑逃逸".into());
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let joined = cwd.join(p);
    let canon_cwd = cwd.canonicalize().unwrap_or(cwd.clone());
    if let Ok(canon) = joined.canonicalize() {
        if !canon.starts_with(&canon_cwd) {
            return Err("路徑超出工作目錄".into());
        }
        return Ok(canon);
    }
    Ok(joined)
}

pub fn is_public_host(host: &str) -> Result<(), String> {
    let h = host.trim().to_ascii_lowercase();
    if h.is_empty() || h.len() > 253 {
        return Err("主機名稱無效".into());
    }
    if h == "localhost" || h.ends_with(".localhost") || h.ends_with(".local") {
        return Err("禁止連到本機名稱".into());
    }
    if h.parse::<IpAddr>().map(|ip| !is_global_ip(ip)).unwrap_or(false) {
        return Err("禁止連到私有或迴環位址".into());
    }
    let addrs = (h.as_str(), 80u16)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?;
    for a in addrs {
        if !is_global_ip(a.ip()) {
            return Err("DNS 解析到私有位址，已拒絕".into());
        }
    }
    Ok(())
}

fn is_global_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_broadcast()
                || v.is_unspecified()
                || v.octets()[0] == 0)
        }
        IpAddr::V6(v) => {
            let o = v.octets();
            !(v.is_loopback()
                || v.is_unspecified()
                || (o[0] & 0xfe) == 0xfc
                || (o[0] == 0xfe && (o[1] & 0xc0) == 0x80))
        }
    }
}

pub const MAX_SLEEP_MS: u64 = 5_000;
pub const MAX_WRITE_BYTES: usize = 1_048_576;
pub const MAX_HTTP_BYTES: usize = 262_144;
pub const MAX_HTTP_SERVE_REQUESTS: i32 = 10_000;
