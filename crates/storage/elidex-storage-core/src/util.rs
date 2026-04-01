//! Shared storage utilities.

/// Encode a string for safe use as a SQLite table name or filesystem path component.
///
/// Short ASCII-only names (≤32 chars, alphanumeric) pass through unchanged
/// for readability. All other inputs are hex-encoded with an `x_` prefix
/// to guarantee collision-free, injection-safe identifiers.
///
/// Used for both SQLite table names and origin directory names on disk.
pub fn sanitize_sql_name(s: &str) -> String {
    if !s.is_empty()
        && s.len() <= 32
        && s.bytes().all(|b| b.is_ascii_alphanumeric())
        && !is_windows_reserved(s)
    {
        return s.to_owned();
    }
    // Prefix hex output with "x_" to avoid collision with alphanumeric passthrough.
    let mut result = String::with_capacity(2 + s.len() * 2);
    result.push_str("x_");
    for b in s.bytes() {
        use std::fmt::Write;
        let _ = write!(result, "{b:02x}");
    }
    result
}

/// Check if a name is a Windows reserved device name (case-insensitive).
fn is_windows_reserved(s: &str) -> bool {
    matches!(
        s.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
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
        // Hex encoding with "x_" prefix to avoid collision with passthrough.
        assert_eq!(name, "x_6d792d6462");
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
