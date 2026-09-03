use std::cmp::Ordering;

use semver_tidy::parse_line;

fn version(raw: &str) -> semver_tidy::Version {
    parse_line(raw, 1).expect("expected successful parse")
}

fn cmp(a: &str, b: &str) -> Ordering {
    version(a).compare_precedence(&version(b))
}

#[test]
fn equal_versions_compare_equal() {
    assert_eq!(cmp("1.2.3", "1.2.3"), Ordering::Equal);
    assert_eq!(cmp("v1.2.3", "1.2.3"), Ordering::Equal);
}

#[test]
fn major_minor_patch_compare_numerically_not_lexically() {
    // A naive string comparison would rank "9" above "10".
    assert_eq!(cmp("1.9.0", "1.10.0"), Ordering::Less);
    assert_eq!(cmp("9.0.0", "10.0.0"), Ordering::Less);
    assert_eq!(cmp("1.2.9", "1.2.10"), Ordering::Less);
}

#[test]
fn leading_zeros_do_not_affect_precedence() {
    assert_eq!(cmp("1.02.3", "1.2.3"), Ordering::Equal);
}

#[test]
fn version_without_prerelease_outranks_one_with_prerelease() {
    assert_eq!(cmp("1.0.0", "1.0.0-alpha"), Ordering::Greater);
    assert_eq!(cmp("1.0.0-alpha", "1.0.0"), Ordering::Less);
}

#[test]
fn prerelease_identifiers_compare_left_to_right() {
    assert_eq!(cmp("1.0.0-alpha.1", "1.0.0-alpha.2"), Ordering::Less);
    assert_eq!(cmp("1.0.0-alpha.2", "1.0.0-beta"), Ordering::Less);
}

#[test]
fn numeric_prerelease_identifiers_always_rank_below_alphanumeric_ones() {
    assert_eq!(cmp("1.0.0-1", "1.0.0-alpha"), Ordering::Less);
    assert_eq!(cmp("1.0.0-alpha", "1.0.0-1"), Ordering::Greater);
}

#[test]
fn numeric_prerelease_identifiers_compare_by_value_not_text() {
    assert_eq!(cmp("1.0.0-9", "1.0.0-10"), Ordering::Less);
    assert_eq!(cmp("1.0.0-alpha.09", "1.0.0-alpha.10"), Ordering::Less);
}

#[test]
fn a_longer_prerelease_field_list_outranks_a_shared_prefix() {
    assert_eq!(cmp("1.0.0-alpha", "1.0.0-alpha.1"), Ordering::Less);
}

#[test]
fn build_metadata_never_affects_precedence() {
    assert_eq!(cmp("1.0.0+build.1", "1.0.0+build.2"), Ordering::Equal);
    assert_eq!(cmp("1.0.0-rc.1+build.1", "1.0.0-rc.1+build.9"), Ordering::Equal);
}

#[test]
fn spec_precedence_example_orders_correctly() {
    // The worked example from semver.org's spec, item 11.
    let ordered = [
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-alpha.beta",
        "1.0.0-beta",
        "1.0.0-beta.2",
        "1.0.0-beta.11",
        "1.0.0-rc.1",
        "1.0.0",
    ];
    for pair in ordered.windows(2) {
        assert_eq!(
            cmp(pair[0], pair[1]),
            Ordering::Less,
            "expected {} < {}",
            pair[0],
            pair[1]
        );
    }
}
