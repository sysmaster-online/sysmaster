//! Functions that perform string escaping and unescaping.

/// Escape a string, typically representing a path.
pub(crate) fn escape<S: AsRef<str>>(input: S) -> String {
    let mut result = String::new();
    let mut prev = false;
    for char in input
        .as_ref()
        .trim_start_matches('/')
        .trim_end_matches('/')
        .chars()
    {
        match char {
            '/' => {
                if prev {
                    continue;
                } else {
                    prev = true;
                    result.push('-');
                }
            }
            _ => {
                prev = false;
                result.push(char);
            }
        }
    }
    result
}

/// Unescape a string that represents a path.
pub(crate) fn _unescape_path<S: AsRef<str>>(input: S) -> String {
    format!("/{}", input.as_ref().replace('-', "/"))
}

/// Unescape a string that does not represent a path.
pub(crate) fn _unescape_non_path<S: AsRef<str>>(input: S) -> String {
    input.as_ref().replace('-', "/")
}

#[cfg(test)]
mod tests {
    use crate::escape::{_unescape_non_path, _unescape_path, escape};

    #[test]
    fn test_escape() {
        assert_eq!(escape("/dev//sda"), "dev-sda".to_string());
        assert_eq!(escape("/foo//bar/baz/"), "foo-bar-baz".to_string());
    }

    #[test]
    fn test_unescape_path() {
        assert_eq!(_unescape_path("dev-sda"), "/dev/sda".to_string());
    }

    #[test]
    fn test_unescape_non_path() {
        assert_eq!(
            _unescape_non_path("normal-escaped-string"),
            "normal/escaped/string".to_string()
        );
    }
}
