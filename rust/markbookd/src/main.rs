mod backup;
mod calc;
mod db;
mod ipc;
mod legacy;

use std::io::{self, BufRead, Write};

fn main() {
    // Keep this binary dependency-light for now. Use simple error mapping.
    let mut state = ipc::AppState {
        workspace: None,
        db: None,
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(v) => v,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let req: ipc::Request = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                // Can't reply without id; ignore.
                let _ = writeln!(
                    stdout,
                    "{{\"ok\":false,\"error\":{{\"code\":\"bad_json\",\"message\":\"{}\"}}}}",
                    e
                );
                let _ = stdout.flush();
                continue;
            }
        };

        let resp = ipc::handle_request(&mut state, req);
        let _ = writeln!(
            stdout,
            "{}",
            serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".to_string())
        );
        let _ = stdout.flush();
    }
}
