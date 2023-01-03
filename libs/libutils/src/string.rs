//! Common used string functions

/// Add "\n" to s.
/// This can be used when generating a multi-line string.
/// Use this function before you write a new line.
pub fn new_line_break(s: &mut String) {
    if !s.is_empty() {
        *s += "\n";
    }
}
