/// Minimal mock extension for testing stderr handling.
///
/// Compiled with `rustc` (no deps) and used by `tests/extension_stderr.rs`.
/// Implements the JSON-RPC extension protocol: initialize, execute, shutdown.
/// Critically, it calls `eprintln!` during execute — the exact scenario that
/// crashes extensions when stderr is dropped after init.
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.is_empty() {
            continue;
        }

        let id = extract_id(&line);

        if line.contains("\"initialize\"") {
            writeln!(stdout, r#"{{"jsonrpc":"2.0","result":{{"ok":true}},"id":{id}}}"#).unwrap();
            stdout.flush().unwrap();
        } else if line.contains("\"execute\"") {
            // This is the critical test: eprintln! after init must not crash
            eprintln!("stderr-test: executing operation");
            eprintln!("stderr-test: second log line");

            writeln!(
                stdout,
                r#"{{"jsonrpc":"2.0","result":{{"success":true,"data":"ok","message":null}},"id":{id}}}"#
            )
            .unwrap();
            stdout.flush().unwrap();
        } else if line.contains("\"shutdown\"") {
            writeln!(stdout, r#"{{"jsonrpc":"2.0","result":{{"ok":true}},"id":{id}}}"#).unwrap();
            stdout.flush().unwrap();
            break;
        }
    }
}

/// Extract the numeric "id" field from a JSON string without a JSON parser.
fn extract_id(json: &str) -> u64 {
    // Find "id": or "id" : and parse the number that follows
    if let Some(pos) = json.find("\"id\"") {
        let rest = &json[pos + 4..];
        // Skip optional whitespace and colon
        let rest = rest.trim_start().strip_prefix(':').unwrap_or(rest);
        let rest = rest.trim_start();
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        num.parse().unwrap_or(0)
    } else {
        0
    }
}
