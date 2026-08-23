use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use semver_tidy::normalize_line;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let check_only = args.iter().any(|a| a == "--check");
    let path = args.iter().find(|a| a.as_str() != "--check");

    let lines: Vec<String> = match path {
        Some(path) => match fs::read_to_string(path) {
            Ok(contents) => contents.lines().map(str::to_string).collect(),
            Err(e) => {
                eprintln!("error: could not read '{path}': {e}");
                return ExitCode::FAILURE;
            }
        },
        None => io::stdin().lock().lines().filter_map(Result::ok).collect(),
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;

    for (idx, raw_line) in lines.iter().enumerate() {
        let line_no = idx + 1;

        if raw_line.trim().is_empty() {
            continue;
        }

        match normalize_line(raw_line, line_no) {
            Ok(normalized) => {
                if !check_only {
                    let _ = writeln!(out, "{normalized}");
                }
            }
            Err(e) => {
                had_error = true;
                eprint!("{e}");
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
