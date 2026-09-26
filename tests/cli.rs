use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Writes `contents` to a uniquely named file under the OS temp dir so
/// concurrently running tests never collide, returning the path to clean
/// up afterward.
fn write_temp_file(name: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("semver-tidy-test-{}-{name}", std::process::id()));
    fs::write(&path, contents).expect("failed to write temp file");
    path
}

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

#[test]
fn single_file_argument_is_read_without_a_filename_prefix() {
    let path = write_temp_file("single_file", "v1.2.3\n1.02.3\n");
    let (stdout, stderr, success) = run(&[path.to_str().unwrap()], "");
    fs::remove_file(&path).ok();

    assert_eq!(stdout, "1.2.3\n1.2.3\n");
    assert_eq!(stderr, "");
    assert!(success);
}

#[test]
fn multiple_files_are_each_normalized_in_order() {
    let first = write_temp_file("multi_first", "1.2.3\n");
    let second = write_temp_file("multi_second", "2.0.0\n");
    let (stdout, _stderr, success) = run(
        &[first.to_str().unwrap(), second.to_str().unwrap()],
        "",
    );
    fs::remove_file(&first).ok();
    fs::remove_file(&second).ok();

    assert_eq!(stdout, "1.2.3\n2.0.0\n");
    assert!(success);
}

#[test]
fn errors_from_multiple_files_are_labeled_with_the_source_file() {
    let first = write_temp_file("multi_err_first", "1.2\n");
    let second = write_temp_file("multi_err_second", "3.0.0\n");
    let (stdout, stderr, success) = run(
        &[first.to_str().unwrap(), second.to_str().unwrap()],
        "",
    );
    fs::remove_file(&first).ok();
    fs::remove_file(&second).ok();

    assert_eq!(stdout, "3.0.0\n");
    assert!(stderr.contains(&format!("{}:", first.display())));
    assert!(stderr.contains("expected '.' followed by the patch version"));
    assert!(!success);
}

#[test]
fn a_missing_file_among_several_does_not_stop_the_others_from_being_processed() {
    let second = write_temp_file("multi_missing_second", "1.2.3\n");
    let missing = std::env::temp_dir().join("semver-tidy-test-this-file-does-not-exist");
    let (stdout, stderr, success) = run(
        &[missing.to_str().unwrap(), second.to_str().unwrap()],
        "",
    );
    fs::remove_file(&second).ok();

    assert_eq!(stdout, "1.2.3\n");
    assert!(stderr.contains("could not read"));
    assert!(!success);
}
