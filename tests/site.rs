use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_russd"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn get(path: &str) -> String {
    let req = format!("GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n");
    for _ in 0..80 {
        if let Ok(mut s) = TcpStream::connect("127.0.0.1:43719") {
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
            if s.write_all(req.as_bytes()).is_err() {
                thread::sleep(Duration::from_millis(25));
                continue;
            }
            let mut buf = String::new();
            let _ = s.read_to_string(&mut buf);
            return buf;
        }
        thread::sleep(Duration::from_millis(25));
    }
    String::new()
}

#[test]
fn workshop_serves_home() {
    let client = thread::spawn(|| get("/?name=Ada"));
    let out = Command::new(bin())
        .args(["run", "examples/site/main.rsd", "--", "43719", "1"])
        .current_dir(repo_root())
        .output()
        .expect("run site");
    let page = client.join().expect("client");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "site failed: {}\n{}",
        stderr,
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(page.contains("Ada"), "page={page}");
    assert!(page.contains("木質工作臺"), "page={page}");
}
