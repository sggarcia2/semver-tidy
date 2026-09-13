//! Parsing and normalization for semantic version strings.
//!
//! The parser is deliberately forgiving about *formatting* (a leading `v`,
//! extra leading zeros, stray whitespace) but strict about *structure*: a
//! missing component, a stray character, or an empty identifier is always
//! reported as an error with the exact line and column where parsing gave
//! up.

pub mod error;

use std::cmp::Ordering;

use error::{Position, SemverError};

/// A parsed version, split into its normalized parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: String,
    pub minor: String,
    pub patch: String,
    pub prerelease: Vec<String>,
    pub build: Vec<String>,
}

impl Version {
    /// Renders the version back to its canonical `major.minor.patch` form.
    pub fn to_canonical_string(&self) -> String {
        self.render(true)
    }

    /// Renders the version back to canonical form, omitting build metadata
    /// even if the input had some. Build metadata never affects precedence,
    /// so callers that only care about comparable identity can drop it.
    pub fn to_canonical_string_without_build(&self) -> String {
        self.render(false)
    }

    fn render(&self, include_build: bool) -> String {
        let mut out = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if !self.prerelease.is_empty() {
            out.push('-');
            out.push_str(&self.prerelease.join("."));
        }
        if include_build && !self.build.is_empty() {
            out.push('+');
            out.push_str(&self.build.join("."));
        }
        out
    }

    /// Orders two versions by semver 2.0.0 precedence (spec item 11):
    /// major, then minor, then patch, then pre-release identifiers
    /// compared left to right. A version with no pre-release outranks one
    /// that has one. Build metadata never affects precedence, so two
    /// versions differing only in build metadata compare as equal here
    /// even though [`PartialEq`] would consider them different.
    ///
    /// This is a plain method rather than an [`Ord`] impl because that
    /// build-metadata exception would make it inconsistent with the
    /// derived [`Eq`], which does compare build metadata.
    pub fn compare_precedence(&self, other: &Version) -> Ordering {
        compare_numeric_str(&self.major, &other.major)
            .then_with(|| compare_numeric_str(&self.minor, &other.minor))
            .then_with(|| compare_numeric_str(&self.patch, &other.patch))
            .then_with(|| compare_prerelease(&self.prerelease, &other.prerelease))
    }
}

fn compare_prerelease(a: &[String], b: &[String]) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }

    for (x, y) in a.iter().zip(b.iter()) {
        let ord = compare_identifier(x, y);
        if ord != Ordering::Equal {
            return ord;
        }
    }
    a.len().cmp(&b.len())
}

fn compare_identifier(a: &str, b: &str) -> Ordering {
    let a_numeric = a.chars().all(|c| c.is_ascii_digit());
    let b_numeric = b.chars().all(|c| c.is_ascii_digit());
    match (a_numeric, b_numeric) {
        (true, true) => compare_numeric_str(a, b),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.cmp(b),
    }
}

/// Compares two non-negative decimal integer strings by numeric value
/// without parsing them into an integer type. Both the version core and
/// numeric pre-release identifiers are normalized to have no leading
/// zeros (aside from a lone "0") by the time they reach here, so ordering
/// by length and then lexicographically gives the correct numeric result
/// for values of any size, including ones too large for a u64.
fn compare_numeric_str(a: &str, b: &str) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

/// Parses a single line of input into its structured, normalized form.
///
/// `line_no` is only used to annotate errors; it has no effect on parsing.
pub fn parse_line(raw: &str, line_no: usize) -> Result<Version, SemverError> {
    Parser::new(raw, line_no).parse()
}

/// Parses a single line of input and returns its normalized canonical form.
///
/// `line_no` is only used to annotate errors; it has no effect on parsing.
pub fn normalize_line(raw: &str, line_no: usize) -> Result<String, SemverError> {
    parse_line(raw, line_no).map(|v| v.to_canonical_string())
}

/// Like [`normalize_line`], but the returned string never has a `+build`
/// suffix, regardless of whether the input carried build metadata.
///
/// `line_no` is only used to annotate errors; it has no effect on parsing.
pub fn normalize_line_without_build(raw: &str, line_no: usize) -> Result<String, SemverError> {
    parse_line(raw, line_no).map(|v| v.to_canonical_string_without_build())
}

struct Parser<'a> {
    raw: &'a str,
    chars: Vec<char>,
    pos: usize,
    line_no: usize,
}

impl<'a> Parser<'a> {
    fn new(raw: &'a str, line_no: usize) -> Self {
        Parser {
            raw,
            chars: raw.chars().collect(),
            pos: 0,
            line_no,
        }
    }

    fn column(&self) -> usize {
        self.pos + 1
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn error(&self, message: impl Into<String>) -> SemverError {
        SemverError {
            position: Position { line: self.line_no, column: self.column() },
            message: message.into(),
            line_text: self.raw.to_string(),
        }
    }

    fn parse(mut self) -> Result<Version, SemverError> {
        self.skip_whitespace();

        if self.peek().is_none() {
            return Err(self.error("expected a version, found an empty line"));
        }

        if matches!(self.peek(), Some('v') | Some('V')) {
            self.advance();
        }

        let major = self.parse_numeric_component("major")?;
        self.expect_dot("minor")?;
        let minor = self.parse_numeric_component("minor")?;
        self.expect_dot("patch")?;
        let patch = self.parse_numeric_component("patch")?;

        let mut prerelease = Vec::new();
        if self.peek() == Some('-') {
            self.advance();
            prerelease = self.parse_dot_identifiers(true)?;
        }

        let mut build = Vec::new();
        if self.peek() == Some('+') {
            self.advance();
            build = self.parse_dot_identifiers(false)?;
        }

        self.skip_whitespace();

        if let Some(c) = self.peek() {
            return Err(self.error(format!("unexpected character '{c}'")));
        }

        Ok(Version { major, minor, patch, prerelease, build })
    }

    fn expect_dot(&mut self, next_component: &str) -> Result<(), SemverError> {
        match self.peek() {
            Some('.') => {
                self.advance();
                Ok(())
            }
            Some(c) => Err(self.error(format!(
                "expected '.' before the {next_component} version, found '{c}'"
            ))),
            None => Err(self.error(format!(
                "expected '.' followed by the {next_component} version, found end of line"
            ))),
        }
    }

    fn parse_numeric_component(&mut self, name: &str) -> Result<String, SemverError> {
        let mut digits = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            digits.push(self.advance().unwrap());
        }

        if digits.is_empty() {
            return match self.peek() {
                Some(c) => {
                    Err(self.error(format!("expected a numeric {name} version, found '{c}'")))
                }
                None => Err(self.error(format!(
                    "expected a numeric {name} version, found end of line"
                ))),
            };
        }

        Ok(strip_leading_zeros(&digits))
    }

    fn parse_dot_identifiers(&mut self, is_prerelease: bool) -> Result<Vec<String>, SemverError> {
        let mut identifiers = Vec::new();
        loop {
            identifiers.push(self.parse_identifier(is_prerelease)?);
            if self.peek() == Some('.') {
                self.advance();
                continue;
            }
            break;
        }
        Ok(identifiers)
    }

    fn parse_identifier(&mut self, is_prerelease: bool) -> Result<String, SemverError> {
        let mut ident = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '-') {
            ident.push(self.advance().unwrap());
        }

        if ident.is_empty() {
            let kind = if is_prerelease { "pre-release" } else { "build metadata" };
            return match self.peek() {
                Some(c) => {
                    Err(self.error(format!("expected a {kind} identifier, found '{c}'")))
                }
                None => {
                    Err(self.error(format!("expected a {kind} identifier, found end of line")))
                }
            };
        }

        // Pre-release identifiers that are purely numeric have semantic
        // ordering (they compare as numbers), so a leading zero is just
        // messiness to clean up. Build metadata is opaque and preserved
        // verbatim.
        if is_prerelease && ident.chars().all(|c| c.is_ascii_digit()) {
            ident = strip_leading_zeros(&ident);
        }

        Ok(ident)
    }
}

fn strip_leading_zeros(digits: &str) -> String {
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}
