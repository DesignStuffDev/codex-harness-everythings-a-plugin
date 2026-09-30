use std::io::{self, BufRead, Write};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut lines = io::stdin().lock().lines();
    lines.next().unwrap().unwrap();
    println!(r#"{{"type":"ready","api_version":1}}"#);
    io::stdout().flush().unwrap();
    let request = lines.next().unwrap().unwrap();
    let mut log = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&args[1])
        .unwrap();
    writeln!(log, "{}", request).unwrap();
    if request.contains(r#""block":true"#) {
        std::fs::write(&args[2], b"started").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1200));
        std::fs::write(format!("{}.finished", args[2]), b"finished").unwrap();
    }
    let result = if request.contains(r#""method":"invoke"#) {
        if request.contains(r#""malformed":true"#) {
            r#"{"text":false,"success":true}"#.to_owned()
        } else if request.contains(r#""large":true"#) {
            format!(r#"{{"text":"{}","success":false}}"#, "é".repeat(20000))
        } else {
            r#"{"text":"native external tool executed","success":true}"#.to_owned()
        }
    } else if request.contains(r#""method":"contribute"#) {
        format!(r#"{{"text":"{}"}}"#, "🌱".repeat(2000))
    } else {
        "{}".to_owned()
    };
    println!(r#"{{"type":"result","id":1,"result":{}}}"#, result);
    io::stdout().flush().unwrap();
    let shutdown = lines.next().unwrap().unwrap();
    assert!(shutdown.contains("shutdown"));
}
