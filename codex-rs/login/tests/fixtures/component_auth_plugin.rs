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
    let response = if request.contains("auth.refresh") {
        &args[3]
    } else {
        &args[2]
    };
    println!("{}", std::fs::read_to_string(response).unwrap());
    io::stdout().flush().unwrap();
    let _ = lines.next();
}
