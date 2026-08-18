//! Parsing and normalization for semantic version strings.
//!
//! The parser is deliberately forgiving about *formatting* (a leading `v`,
//! extra leading zeros, stray whitespace) but strict about *structure*: a
//! missing component, a stray character, or an empty identifier is always
//! reported as an error with the exact line and column where parsing gave
//! up.

pub mod error;

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
        let mut out = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if !self.prerelease.is_empty() {
            out.push('-');
            out.push_str(&self.prerelease.join("."));
        }
        if !self.build.is_empty() {
            out.push('+');
            out.push_str(&self.build.join("."));
        }
        out
    }
}

/// Parses a single line of input and returns its normalized canonical form.
///
/// `line_no` is only used to annotate errors; it has no effect on parsing.
pub fn normalize_line(raw: &str, line_no: usize) -> Result<String, SemverError> {
    Parser::new(raw, line_no).parse().map(|v| v.to_canonical_string())
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
