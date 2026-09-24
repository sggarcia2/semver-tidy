use std::io::Write;
use std::process::{Command, Stdio};

fn run(args: &[&str], stdin_input: &str) -> (String, String, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_semver-tidy"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start semver-tidy");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin_input.as_bytes())
        .expect("failed to write stdin");

    let output = child.wait_with_output().expect("failed to wait on child");
    (
        String::from_utf8(output.stdout).expect("stdout was not utf-8"),
        String::from_utf8(output.stderr).expect("stderr was not utf-8"),
        output.status.success(),
    )
}

#[test]
fn without_check_flag_normalized_output_is_printed() {
    let (stdout, _stderr, success) = run(&[], "v1.2.3\n1.02.3\n");
    assert_eq!(stdout, "1.2.3\n1.2.3\n");
    assert!(success);
}

#[test]
fn check_mode_suppresses_normalized_output_on_success() {
    let (stdout, stderr, success) = run(&["--check"], "v1.2.3\n1.02.3\n");
    assert_eq!(stdout, "");
    assert_eq!(stderr, "");
    assert!(success);
}

#[test]
fn check_mode_still_reports_errors_and_exits_nonzero() {
    let (stdout, stderr, success) = run(&["--check"], "v1.2.3\n1.2\n");
    assert_eq!(stdout, "");
    assert!(stderr.contains("expected '.' followed by the patch version"));
    assert!(!success);
}

#[test]
fn strip_build_flag_drops_build_metadata_from_output() {
    let (stdout, _stderr, success) = run(&["--strip-build"], "1.0.0+build.007\n1.2.3\n");
    assert_eq!(stdout, "1.0.0\n1.2.3\n");
    assert!(success);
}

#[test]
fn strip_build_flag_keeps_prerelease() {
    let (stdout, _stderr, success) = run(&["--strip-build"], "1.0.0-alpha.1+build.7\n");
    assert_eq!(stdout, "1.0.0-alpha.1\n");
    assert!(success);
}

#[test]
fn strip_build_flag_combines_with_check() {
    let (stdout, stderr, success) = run(&["--strip-build", "--check"], "1.0.0+build.007\n1.2\n");
    assert_eq!(stdout, "");
    assert!(stderr.contains("expected '.' followed by the patch version"));
    assert!(!success);
}

#[test]
fn compare_prints_less_than() {
    let (stdout, _stderr, success) = run(&["--compare", "1.2.3", "1.10.0"], "");
    assert_eq!(stdout, "<\n");
    assert!(success);
}

#[test]
fn compare_prints_equal_ignoring_build_metadata() {
    let (stdout, _stderr, success) = run(&["--compare", "1.0.0+a", "1.0.0+b"], "");
    assert_eq!(stdout, "=\n");
    assert!(success);
}

#[test]
fn compare_prints_greater_than() {
    let (stdout, _stderr, success) = run(&["--compare", "2.0.0", "1.9.9"], "");
    assert_eq!(stdout, ">\n");
    assert!(success);
}

#[test]
fn compare_rejects_wrong_argument_count() {
    let (stdout, stderr, success) = run(&["--compare", "1.0.0"], "");
    assert_eq!(stdout, "");
    assert!(stderr.contains("requires exactly two versions"));
    assert!(!success);
}

#[test]
fn compare_reports_which_argument_failed_to_parse() {
    let (stdout, stderr, success) = run(&["--compare", "1.0", "1.0.0"], "");
    assert_eq!(stdout, "");
    assert!(stderr.contains("first argument to --compare did not parse"));
    assert!(stderr.contains("expected '.' followed by the patch version"));
    assert!(!success);
}
