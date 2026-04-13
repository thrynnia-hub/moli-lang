// ARCH: Utility module providing diagnostic printing with colorized, line-accurate output.
// Span-based error reporting maps byte offsets to line/column positions.

/// A span in source code, represented as byte offsets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// A diagnostic error with message and source span
#[derive(Debug, Clone)]
pub struct MoliError {
    pub message: String,
    pub span: Span,
}

impl MoliError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for MoliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Converts a byte offset to (line, column), both 1-indexed
pub fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Returns the source line (1-indexed) as a string slice
pub fn get_source_line(source: &str, line_number: usize) -> &str {
    source.lines().nth(line_number - 1).unwrap_or("")
}

/// Colorized diagnostic printer that shows source context with carets
pub struct DiagnosticPrinter<'a> {
    source: &'a str,
    file: &'a str,
}

impl<'a> DiagnosticPrinter<'a> {
    pub fn new(source: &'a str, file: &'a str) -> Self {
        Self { source, file }
    }

    pub fn print_error(&self, message: &str, start: usize, end: usize) {
        let (line, col) = offset_to_line_col(self.source, start);
        let source_line = get_source_line(self.source, line);

        eprintln!(
            "\x1b[1;31merror\x1b[0m: {}",
            message
        );
        eprintln!(
            "  \x1b[1;34m-->\x1b[0m {}:{}:{}",
            self.file, line, col
        );
        eprintln!("   \x1b[1;34m|\x1b[0m");
        eprintln!(
            "\x1b[1;34m{:>3} |\x1b[0m {}",
            line, source_line
        );

        // ARCH: Underline the error span with carets
        let underline_start = col - 1;
        let underline_len = if end > start {
            let (end_line, end_col) = offset_to_line_col(self.source, end);
            if end_line == line {
                end_col - col
            } else {
                source_line.len().saturating_sub(underline_start)
            }
        } else {
            1
        };
        let underline_len = underline_len.max(1);

        eprintln!(
            "   \x1b[1;34m|\x1b[0m {}\x1b[1;31m{}\x1b[0m",
            " ".repeat(underline_start),
            "^".repeat(underline_len)
        );
        eprintln!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_line_col() {
        let src = "hello\nworld\nfoo";
        assert_eq!(offset_to_line_col(src, 0), (1, 1));
        assert_eq!(offset_to_line_col(src, 5), (1, 6));
        assert_eq!(offset_to_line_col(src, 6), (2, 1));
        assert_eq!(offset_to_line_col(src, 11), (2, 6));
        assert_eq!(offset_to_line_col(src, 12), (3, 1));
    }

    #[test]
    fn test_get_source_line() {
        let src = "line1\nline2\nline3";
        assert_eq!(get_source_line(src, 1), "line1");
        assert_eq!(get_source_line(src, 2), "line2");
        assert_eq!(get_source_line(src, 3), "line3");
    }

    #[test]
    fn test_span_merge() {
        let a = Span::new(5, 10);
        let b = Span::new(3, 12);
        let merged = a.merge(b);
        assert_eq!(merged.start, 3);
        assert_eq!(merged.end, 12);
    }
}
