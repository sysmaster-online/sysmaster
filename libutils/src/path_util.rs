use std::path::Path;

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
        assert_eq!(path_equal("/etc", "/etc"), true);
        assert_eq!(path_equal("//etc", "/etc"), true);
        assert_eq!(path_equal("/etc//", "/etc"), true);
        assert_eq!(path_equal("/etc", "./etc"), false);
        assert_eq!(path_equal("/x/./y", "/x/y"), true);
        assert_eq!(path_equal("/x/././y", "/x/y/./."), true);
        assert_eq!(path_equal("/etc", "/var"), false);
    }
}
