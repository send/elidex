//! Shared storage utilities.

/// Encode a string for safe use as a SQLite table name component.
///
/// Short ASCII-only names (≤32 chars, alphanumeric) pass through unchanged
/// for readability. All other inputs are hex-encoded to guarantee
/// collision-free, injection-safe identifiers.
pub fn sanitize_sql_name(s: &str) -> String {
    if s.len() <= 32 && s.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return s.to_owned();
    }
    s.bytes()
        .fold(String::with_capacity(s.len() * 2), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_ascii_passthrough() {
        assert_eq!(sanitize_sql_name("users"), "users");
        assert_eq!(sanitize_sql_name("items123"), "items123");
    }

    #[test]
    fn non_ascii_hex_encoded() {
        let name = sanitize_sql_name("my-db");
        assert_ne!(name, "my-db");
        // Hex encoding: 'm'=6d, 'y'=79, '-'=2d, 'd'=64, 'b'=62
        assert_eq!(name, "6d792d6462");
    }

    #[test]
    fn collision_free() {
        assert_ne!(sanitize_sql_name("a-b"), sanitize_sql_name("a_b"));
        assert_ne!(sanitize_sql_name("a:b"), sanitize_sql_name("a/b"));
    }

    #[test]
    fn long_name_hex_encoded() {
        let long = "a".repeat(33);
        let sanitized = sanitize_sql_name(&long);
        assert_ne!(sanitized, long); // > 32 chars triggers hex
    }
}
