use std::cmp::Ordering;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use semver_tidy::{normalize_line, normalize_line_without_build, parse_line};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "--compare") {
        return run_compare(&args);
    }

    let check_only = args.iter().any(|a| a == "--check");
    let strip_build = args.iter().any(|a| a == "--strip-build");
    let paths: Vec<&String> = args
        .iter()
        .filter(|a| a.as_str() != "--check" && a.as_str() != "--strip-build")
        .collect();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;

    if paths.is_empty() {
        let lines: Vec<String> = io::stdin().lock().lines().filter_map(Result::ok).collect();
        had_error |= process_lines(&lines, None, check_only, strip_build, &mut out);
    } else {
        // Only prefix errors with the source file once there's more than
        // one, so the single-file case keeps its plain error output.
        let show_filename = paths.len() > 1;
        for path in paths {
            match fs::read_to_string(path) {
                Ok(contents) => {
                    let lines: Vec<String> = contents.lines().map(str::to_string).collect();
                    let label = show_filename.then(|| path.as_str());
                    had_error |= process_lines(&lines, label, check_only, strip_build, &mut out);
                }
                Err(e) => {
                    eprintln!("error: could not read '{path}': {e}");
                    had_error = true;
                }
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Normalizes each line and writes the result to `out`, returning whether
/// any line failed to parse. `filename`, when set, is printed ahead of
/// that file's errors so failures stay traceable when multiple files are
/// given on the command line (line numbers alone reset per file).
fn process_lines(
    lines: &[String],
    filename: Option<&str>,
    check_only: bool,
    strip_build: bool,
    out: &mut impl Write,
) -> bool {
    let mut had_error = false;

    for (idx, raw_line) in lines.iter().enumerate() {
        let line_no = idx + 1;

        if raw_line.trim().is_empty() {
            continue;
        }

        let result = if strip_build {
            normalize_line_without_build(raw_line, line_no)
        } else {
            normalize_line(raw_line, line_no)
        };

        match result {
            Ok(normalized) => {
                if !check_only {
                    let _ = writeln!(out, "{normalized}");
                }
            }
            Err(e) => {
                had_error = true;
                if let Some(name) = filename {
                    eprintln!("{name}:");
                }
                eprint!("{e}");
            }
        }
    }

    had_error
}

/// Handles `--compare <a> <b>`, printing `<`, `=`, or `>` for how `a` orders
/// against `b` under semver precedence (build metadata ignored).
fn run_compare(args: &[String]) -> ExitCode {
    let versions: Vec<&String> = args.iter().filter(|a| a.as_str() != "--compare").collect();
    if versions.len() != 2 {
        eprintln!(
            "error: --compare requires exactly two versions, e.g. semver-tidy --compare 1.2.3 1.3.0"
        );
        return ExitCode::FAILURE;
    }

    let left = match parse_line(versions[0], 1) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: first argument to --compare did not parse");
            eprint!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let right = match parse_line(versions[1], 1) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: second argument to --compare did not parse");
            eprint!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let symbol = match left.compare_precedence(&right) {
        Ordering::Less => "<",
        Ordering::Equal => "=",
        Ordering::Greater => ">",
    };
    println!("{symbol}");
    ExitCode::SUCCESS
}
