use semver_tidy::normalize_line;

fn normalize(raw: &str) -> String {
    normalize_line(raw, 1).expect("expected successful parse")
}

fn fails(raw: &str) -> semver_tidy::error::SemverError {
    normalize_line(raw, 1).expect_err("expected a parse error")
}

#[test]
fn parses_plain_version() {
    assert_eq!(normalize("1.2.3"), "1.2.3");
    assert_eq!(normalize("0.0.0"), "0.0.0");
}

#[test]
fn strips_leading_v_prefix() {
    assert_eq!(normalize("v1.2.3"), "1.2.3");
    assert_eq!(normalize("V1.2.3"), "1.2.3");
}

#[test]
fn trims_surrounding_whitespace() {
    assert_eq!(normalize("  1.2.3  "), "1.2.3");
    assert_eq!(normalize("\t1.2.3\t"), "1.2.3");
}

#[test]
fn strips_leading_zeros_from_core_components() {
    assert_eq!(normalize("1.02.3"), "1.2.3");
    assert_eq!(normalize("01.02.03"), "1.2.3");
}

#[test]
fn strips_leading_zeros_from_numeric_prerelease_identifiers() {
    assert_eq!(normalize("1.2.3-Alpha.01"), "1.2.3-Alpha.1");
    assert_eq!(normalize("1.2.3-0"), "1.2.3-0");
}

#[test]
fn preserves_hyphenated_prerelease_identifiers() {
    assert_eq!(normalize("1.2.3-alpha-beta.1"), "1.2.3-alpha-beta.1");
}

#[test]
fn preserves_build_metadata_verbatim_including_leading_zeros() {
    assert_eq!(normalize("1.0.0+build.007"), "1.0.0+build.007");
    assert_eq!(
        normalize("1.2.3+001.exp.sha.5114f85"),
        "1.2.3+001.exp.sha.5114f85"
    );
}

#[test]
fn handles_multiple_dot_separated_identifiers() {
    assert_eq!(normalize("1.2.3-alpha.1.2"), "1.2.3-alpha.1.2");
}

#[test]
fn empty_line_is_an_error_at_column_one() {
    let err = fails("");
    assert_eq!(err.position.column, 1);
    assert!(err.message.contains("empty line"));
}

#[test]
fn whitespace_only_line_points_past_the_whitespace() {
    let err = fails("   ");
    assert_eq!(err.position.column, 4);
}

#[test]
fn missing_component_names_what_was_expected() {
    let err = fails("1.2");
    assert_eq!(err.position.column, 4);
    assert!(err.message.contains("patch"));
    assert!(err.message.contains("end of line"));
}

#[test]
fn non_numeric_major_names_the_offending_character() {
    let err = fails("a.2.3");
    assert_eq!(err.position.column, 1);
    assert!(err.message.contains("major"));
    assert!(err.message.contains('a'));
}

#[test]
fn empty_numeric_component_between_dots_is_an_error() {
    let err = fails("1..3");
    assert_eq!(err.position.column, 3);
    assert!(err.message.contains("minor"));
}

#[test]
fn dangling_prerelease_marker_is_an_error() {
    let err = fails("1.2.3-");
    assert_eq!(err.position.column, 7);
    assert!(err.message.contains("pre-release"));
}

#[test]
fn empty_identifier_between_dots_is_an_error() {
    let err = fails("1.2.3-alpha..beta");
    assert!(err.message.contains("pre-release"));
}

#[test]
fn dangling_build_marker_is_an_error() {
    let err = fails("1.2.3+");
    assert!(err.message.contains("build metadata"));
}

#[test]
fn trailing_garbage_after_a_valid_version_is_an_error() {
    let err = fails("1.2.3 abc");
    assert!(err.message.contains("unexpected character 'a'"));
}

#[test]
fn underscore_is_not_a_valid_identifier_character() {
    let err = fails("1.2.3-al_pha");
    assert!(err.message.contains("unexpected character '_'"));
}

#[test]
fn line_number_is_carried_through_to_the_error() {
    let err = normalize_line("bad", 5).unwrap_err();
    assert_eq!(err.position.line, 5);
}

#[test]
fn columns_count_characters_not_bytes() {
    // U+00A0 (no-break space) is one character but two bytes in UTF-8.
    // If columns were computed from byte offsets instead of char counts,
    // this would report column 5 instead of 4.
    let err = fails("\u{a0}1.2");
    assert_eq!(err.position.column, 4);
}
