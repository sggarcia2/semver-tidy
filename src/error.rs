use std::fmt;

/// A 1-indexed line and column within the original input.
///
/// Columns count characters, not bytes, so the caret in [`SemverError`]'s
/// `Display` impl lines up correctly for non-ASCII input too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// A parse failure, carrying enough context to render a caret pointing at
/// the exact character that caused it.
#[derive(Debug, Clone)]
pub struct SemverError {
    pub position: Position,
    pub message: String,
    pub line_text: String,
}

impl fmt::Display for SemverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line_no = self.position.line.to_string();
        let gutter = line_no.len();

        writeln!(f, "error: {}", self.message)?;
        writeln!(
            f,
            "{:gutter$}--> line {}, column {}",
            "",
            self.position.line,
            self.position.column,
        )?;
        writeln!(f, "{:gutter$} |", "")?;
        writeln!(f, "{line_no} | {}", self.line_text)?;
        let caret_offset = self.position.column.saturating_sub(1);
        writeln!(f, "{:gutter$} | {:caret_offset$}^", "", "")
    }
}

impl std::error::Error for SemverError {}
