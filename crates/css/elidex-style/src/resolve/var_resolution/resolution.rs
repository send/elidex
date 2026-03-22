//! Recursive `var()` resolution and substitution.

use std::collections::{HashMap, HashSet};

use elidex_plugin::CssValue;

/// Maximum recursion depth for resolving `var()` references (cycle protection).
const MAX_VAR_DEPTH: usize = 32;

/// Maximum length (in bytes) for the output of `substitute_vars()`.
///
/// Prevents exponential blowup from patterns like
/// `--a: var(--b) var(--b); --b: var(--c) var(--c)` which would otherwise
/// produce 2^depth copies.
const MAX_SUBSTITUTED_LENGTH: usize = 65_536;

/// Resolve a single `CssValue::Var` to a concrete value.
///
/// Uses both depth limiting and a visited set for cycle detection.
/// If a custom property name is already in the visited set, the reference
/// is circular and resolution fails (returns `None`).
#[must_use]
pub(in crate::resolve) fn resolve_var_value(
    value: &CssValue,
    custom_props: &HashMap<String, String>,
    depth: usize,
    visited: &mut HashSet<String>,
) -> Option<CssValue> {
    if depth > MAX_VAR_DEPTH {
        return None;
    }

    let CssValue::Var(ref name, ref fallback) = value else {
        return Some(value.clone());
    };

    // Cycle detection: if we've already visited this property, bail out.
    if !visited.insert(name.clone()) {
        return None;
    }

    // Look up the custom property.
    if let Some(raw) = custom_props.get(name) {
        // The raw value itself might contain var() references.
        let parsed = parse_raw_value(raw);
        if let CssValue::Var(..) = &parsed {
            // Recursively resolve nested var().
            let result = resolve_var_value(&parsed, custom_props, depth + 1, visited);
            visited.remove(name);
            return result;
        }
        visited.remove(name);
        return Some(parsed);
    }

    visited.remove(name);

    // Property not found: use fallback if available.
    match fallback {
        Some(fb) => {
            if let CssValue::Var(..) = fb.as_ref() {
                resolve_var_value(fb, custom_props, depth + 1, visited)
            } else {
                Some(*fb.clone())
            }
        }
        None => None, // No fallback, var() unresolvable.
    }
}

/// Parse a raw token string into a typed [`CssValue`].
///
/// Delegates to `elidex_css::parse_raw_token_value()` which uses cssparser
/// internally.
pub(in crate::resolve) fn parse_raw_value(raw: &str) -> CssValue {
    elidex_css::parse_raw_token_value(raw)
}

/// Check whether a `CssValue` contains an unresolved `var()` reference.
pub(super) fn has_var_reference(value: &CssValue) -> bool {
    match value {
        CssValue::Var(..) => true,
        CssValue::RawTokens(raw) => raw.contains("var("),
        _ => false,
    }
}

/// Substitute all `var()` references in a raw CSS value string.
///
/// Returns `None` if any `var()` reference is unresolvable (no custom property
/// value and no fallback), which makes the property "invalid at computed-value
/// time" per CSS Variables Level 1 5.
///
/// Uses both depth limiting and a visited set for cycle detection, plus an
/// output size limit ([`MAX_SUBSTITUTED_LENGTH`]) to prevent exponential blowup.
pub(super) fn substitute_vars(
    raw: &str,
    custom_props: &HashMap<String, String>,
    depth: usize,
    visited: &mut HashSet<String>,
) -> Option<String> {
    if depth > MAX_VAR_DEPTH {
        return None;
    }
    if !raw.contains("var(") {
        return Some(raw.to_string());
    }

    let mut result = String::with_capacity(raw.len());
    let mut remaining = raw;

    // NOTE: Simple string search; does not skip var() inside CSS string literals.
    // This is acceptable since compound var() in string contexts is rare.
    loop {
        let Some(var_start) = remaining.find("var(") else {
            result.push_str(remaining);
            break;
        };

        // Copy text before var(
        result.push_str(&remaining[..var_start]);

        // Parse var() arguments
        let after_open = &remaining[var_start + 4..];
        let (name, fallback, consumed) = parse_var_args(after_open)?;

        // Look up the custom property
        let name = name.trim();

        // Cycle detection: if we've already visited this property, bail out.
        if visited.contains(name) {
            return None;
        }

        if let Some(value) = custom_props.get(name) {
            // The value might itself contain var() -- recurse.
            visited.insert(name.to_string());
            let resolved = substitute_vars(value.trim(), custom_props, depth + 1, visited)?;
            visited.remove(name);
            result.push_str(&resolved);
        } else if let Some(fb) = fallback {
            // Use fallback, which might contain var().
            let resolved = substitute_vars(fb.trim(), custom_props, depth + 1, visited)?;
            result.push_str(&resolved);
        } else {
            return None; // Unresolvable -- invalid at computed-value time.
        }

        // Check output size limit after each substitution.
        if result.len() > MAX_SUBSTITUTED_LENGTH {
            return None;
        }

        remaining = &after_open[consumed..];
    }

    Some(result)
}

/// Parse `var()` arguments from a string positioned after the opening `var(`.
///
/// Returns `(name, optional_fallback, bytes_consumed)` including the closing `)`.
/// Handles nested parentheses in the fallback value (e.g. `var(--x, calc(1px + 2px))`).
pub(super) fn parse_var_args(s: &str) -> Option<(&str, Option<&str>, usize)> {
    let mut depth: u32 = 1;
    let mut comma_pos = None;

    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let name_end = comma_pos.unwrap_or(i);
                    let name = &s[..name_end];
                    let fallback = comma_pos.map(|cp| &s[cp + 1..i]);
                    return Some((name, fallback, i + 1));
                }
            }
            ',' if depth == 1 && comma_pos.is_none() => {
                comma_pos = Some(i);
            }
            _ => {}
        }
    }
    None // Unclosed var()
}
