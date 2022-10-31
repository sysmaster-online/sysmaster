//! the utils of the path operation
//!
use std::path::Path;

/// return true if the path of a and b equaled.
pub fn path_equal(a: &str, b: &str) -> bool {
    let p_a = Path::new(a);
    let p_b = Path::new(b);
    p_a == p_b
}

#[cfg(test)]
mod tests {
    use crate::path_util::path_equal;

    #[test]
    fn test_path_equal() {
        assert!(path_equal("/etc", "/etc"));
        assert!(path_equal("//etc", "/etc"));
        assert!(path_equal("/etc//", "/etc"));
        assert!(!path_equal("/etc", "./etc"));
        assert!(path_equal("/x/./y", "/x/y"));
        assert!(path_equal("/x/././y", "/x/y/./."));
        assert!(!path_equal("/etc", "/var"));
    }
}
