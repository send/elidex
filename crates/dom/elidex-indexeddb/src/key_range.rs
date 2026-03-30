//! IDB key range — represents a continuous interval over the key space.
//!
//! W3C `IndexedDB` 3.0 §2.7 — used by cursors, `get_all`, `count`, `delete`.

use crate::IdbKey;

/// A key range representing a bounded or unbounded interval.
///
/// Both `lower` and `upper` may be `None` for unbounded ranges.
/// `lower_open` / `upper_open` control whether the bounds are exclusive.
#[derive(Debug, Clone, PartialEq)]
pub struct IdbKeyRange {
    pub lower: Option<IdbKey>,
    pub upper: Option<IdbKey>,
    pub lower_open: bool,
    pub upper_open: bool,
}

impl IdbKeyRange {
    /// Creates a range containing only the given key (`[key, key]`).
    pub fn only(key: IdbKey) -> Self {
        Self {
            lower: Some(key.clone()),
            upper: Some(key),
            lower_open: false,
            upper_open: false,
        }
    }

    /// Creates a range with a lower bound: `(lower, +inf)` or `[lower, +inf)`.
    pub fn lower_bound(lower: IdbKey, open: bool) -> Self {
        Self {
            lower: Some(lower),
            upper: None,
            lower_open: open,
            upper_open: false,
        }
    }

    /// Creates a range with an upper bound: `(-inf, upper)` or `(-inf, upper]`.
    pub fn upper_bound(upper: IdbKey, open: bool) -> Self {
        Self {
            lower: None,
            upper: Some(upper),
            lower_open: false,
            upper_open: open,
        }
    }

    /// Creates a range with both bounds.
    ///
    /// Returns `None` if `lower > upper`, or if `lower == upper` and either bound is open.
    pub fn bound(lower: IdbKey, upper: IdbKey, lower_open: bool, upper_open: bool) -> Option<Self> {
        use std::cmp::Ordering;
        match lower.cmp(&upper) {
            Ordering::Greater => return None,
            Ordering::Equal if lower_open || upper_open => return None,
            _ => {}
        }
        Some(Self {
            lower: Some(lower),
            upper: Some(upper),
            lower_open,
            upper_open,
        })
    }

    /// Returns `true` if the given key falls within this range.
    pub fn includes(&self, key: &IdbKey) -> bool {
        if let Some(lower) = &self.lower {
            let cmp = key.cmp(lower);
            if self.lower_open {
                if cmp != std::cmp::Ordering::Greater {
                    return false;
                }
            } else if cmp == std::cmp::Ordering::Less {
                return false;
            }
        }

        if let Some(upper) = &self.upper {
            let cmp = key.cmp(upper);
            if self.upper_open {
                if cmp != std::cmp::Ordering::Less {
                    return false;
                }
            } else if cmp == std::cmp::Ordering::Greater {
                return false;
            }
        }

        true
    }

    /// Generates a SQL WHERE clause fragment for key column comparisons.
    ///
    /// Returns `(clause, params)` where `clause` uses `?` placeholders
    /// and `params` are the serialized key bytes to bind.
    pub fn to_sql_clause(&self, col: &str) -> (String, Vec<Vec<u8>>) {
        let mut parts = Vec::new();
        let mut params = Vec::new();

        if let Some(lower) = &self.lower {
            let op = if self.lower_open { ">" } else { ">=" };
            parts.push(format!("{col} {op} ?"));
            params.push(lower.serialize());
        }

        if let Some(upper) = &self.upper {
            let op = if self.upper_open { "<" } else { "<=" };
            parts.push(format!("{col} {op} ?"));
            params.push(upper.serialize());
        }

        if parts.is_empty() {
            ("1=1".to_owned(), params)
        } else {
            (parts.join(" AND "), params)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_includes_exact_match() {
        let range = IdbKeyRange::only(IdbKey::Number(5.0));
        assert!(range.includes(&IdbKey::Number(5.0)));
        assert!(!range.includes(&IdbKey::Number(4.9)));
        assert!(!range.includes(&IdbKey::Number(5.1)));
    }

    #[test]
    fn lower_bound_closed() {
        let range = IdbKeyRange::lower_bound(IdbKey::Number(3.0), false);
        assert!(range.includes(&IdbKey::Number(3.0)));
        assert!(range.includes(&IdbKey::Number(100.0)));
        assert!(!range.includes(&IdbKey::Number(2.9)));
    }

    #[test]
    fn lower_bound_open() {
        let range = IdbKeyRange::lower_bound(IdbKey::Number(3.0), true);
        assert!(!range.includes(&IdbKey::Number(3.0)));
        assert!(range.includes(&IdbKey::Number(3.1)));
    }

    #[test]
    fn upper_bound_closed() {
        let range = IdbKeyRange::upper_bound(IdbKey::Number(10.0), false);
        assert!(range.includes(&IdbKey::Number(10.0)));
        assert!(range.includes(&IdbKey::Number(-100.0)));
        assert!(!range.includes(&IdbKey::Number(10.1)));
    }

    #[test]
    fn upper_bound_open() {
        let range = IdbKeyRange::upper_bound(IdbKey::Number(10.0), true);
        assert!(!range.includes(&IdbKey::Number(10.0)));
        assert!(range.includes(&IdbKey::Number(9.9)));
    }

    #[test]
    fn bound_closed_closed() {
        let range =
            IdbKeyRange::bound(IdbKey::Number(2.0), IdbKey::Number(5.0), false, false).unwrap();
        assert!(range.includes(&IdbKey::Number(2.0)));
        assert!(range.includes(&IdbKey::Number(3.5)));
        assert!(range.includes(&IdbKey::Number(5.0)));
        assert!(!range.includes(&IdbKey::Number(1.9)));
        assert!(!range.includes(&IdbKey::Number(5.1)));
    }

    #[test]
    fn bound_open_open() {
        let range =
            IdbKeyRange::bound(IdbKey::Number(2.0), IdbKey::Number(5.0), true, true).unwrap();
        assert!(!range.includes(&IdbKey::Number(2.0)));
        assert!(range.includes(&IdbKey::Number(3.0)));
        assert!(!range.includes(&IdbKey::Number(5.0)));
    }

    #[test]
    fn bound_invalid_lower_greater_than_upper() {
        assert!(
            IdbKeyRange::bound(IdbKey::Number(10.0), IdbKey::Number(5.0), false, false).is_none()
        );
    }

    #[test]
    fn bound_invalid_equal_but_open() {
        // equal bounds with either side open is invalid
        assert!(
            IdbKeyRange::bound(IdbKey::Number(5.0), IdbKey::Number(5.0), true, false).is_none()
        );
        assert!(
            IdbKeyRange::bound(IdbKey::Number(5.0), IdbKey::Number(5.0), false, true).is_none()
        );
        assert!(IdbKeyRange::bound(IdbKey::Number(5.0), IdbKey::Number(5.0), true, true).is_none());
    }

    #[test]
    fn bound_equal_closed_valid() {
        let range =
            IdbKeyRange::bound(IdbKey::Number(5.0), IdbKey::Number(5.0), false, false).unwrap();
        assert!(range.includes(&IdbKey::Number(5.0)));
        assert!(!range.includes(&IdbKey::Number(4.9)));
    }

    #[test]
    fn cross_type_range() {
        // range spanning from number to string
        let range = IdbKeyRange::bound(
            IdbKey::Number(0.0),
            IdbKey::String("z".into()),
            false,
            false,
        )
        .unwrap();
        assert!(range.includes(&IdbKey::Number(100.0)));
        assert!(range.includes(&IdbKey::Date(0.0))); // date is between number and string
        assert!(range.includes(&IdbKey::String("a".into())));
        assert!(!range.includes(&IdbKey::Array(vec![]))); // array is above string
    }

    #[test]
    fn to_sql_clause_both_bounds() {
        let range =
            IdbKeyRange::bound(IdbKey::Number(1.0), IdbKey::Number(10.0), true, false).unwrap();
        let (clause, params) = range.to_sql_clause("key_data");
        assert_eq!(clause, "key_data > ? AND key_data <= ?");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn to_sql_clause_unbounded() {
        let range = IdbKeyRange {
            lower: None,
            upper: None,
            lower_open: false,
            upper_open: false,
        };
        let (clause, params) = range.to_sql_clause("k");
        assert_eq!(clause, "1=1");
        assert!(params.is_empty());
    }
}
