//! CSS Counter state machine (CSS Lists Level 3 §5–7).
//!
//! Tracks counter scopes during display list tree walk. Counters are created
//! by `counter-reset`, modified by `counter-increment` and `counter-set`,
//! and evaluated by `counter()` / `counters()` functions.

use std::collections::HashMap;

use elidex_plugin::{ComputedStyle, ListStyleType};

/// CSS Counter state machine.
///
/// Maintains a stack of `(scope_depth, value)` pairs per counter name,
/// allowing nested scopes to shadow outer counter instances.
pub struct CounterState {
    /// name → stack of `(scope_depth, value)` pairs.
    counters: HashMap<String, Vec<(usize, i32)>>,
    /// Current nesting depth (incremented on element entry, decremented on exit).
    scope_depth: usize,
}

impl CounterState {
    /// Create a new empty counter state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
            scope_depth: 0,
        }
    }

    /// Increment scope depth on element entry.
    pub fn push_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Decrement scope depth on element exit, removing any counter entries
    /// created at this depth (CSS Lists L3 §5.1).
    pub fn pop_scope(&mut self) {
        let depth = self.scope_depth;
        self.counters.retain(|_, stack| {
            while stack.last().is_some_and(|&(d, _)| d == depth) {
                stack.pop();
            }
            !stack.is_empty()
        });
        self.scope_depth = self.scope_depth.saturating_sub(1);
    }

    /// Process counter properties from a computed style.
    ///
    /// Order of operations per CSS Lists L3 §5.4: reset → set → increment.
    ///
    /// `is_continuation` suppresses increment (used for fragmentation continuations).
    pub fn process_element(&mut self, style: &ComputedStyle, is_continuation: bool) {
        // Phase 1: counter-reset — creates new scope entries.
        for (name, value) in &style.counter_reset {
            self.counters
                .entry(name.clone())
                .or_default()
                .push((self.scope_depth, *value));
        }

        // Phase 2: counter-set — overwrites top value or creates if missing (§5.3).
        for (name, value) in &style.counter_set {
            let stack = self.counters.entry(name.clone()).or_default();
            if let Some(top) = stack.last_mut() {
                top.1 = *value;
            } else {
                stack.push((self.scope_depth, *value));
            }
        }

        // Phase 3: counter-increment — adds to top value or creates then increments (§5.2).
        if !is_continuation {
            for (name, value) in &style.counter_increment {
                let stack = self.counters.entry(name.clone()).or_default();
                if let Some(top) = stack.last_mut() {
                    top.1 += value;
                } else {
                    stack.push((self.scope_depth, *value));
                }
            }
        }
    }

    /// Evaluate a single `counter(name, style)` function.
    ///
    /// Returns the formatted top value from the counter's stack, or "0" if
    /// the counter does not exist.
    #[must_use]
    pub fn evaluate_counter(&self, name: &str, style: ListStyleType) -> String {
        let value = self
            .counters
            .get(name)
            .and_then(|stack| stack.last())
            .map_or(0, |&(_, v)| v);
        format_counter_value(value, style)
    }

    /// Evaluate a `counters(name, separator, style)` function.
    ///
    /// Returns all values from the counter's stack, formatted and joined
    /// with `separator`.
    #[must_use]
    pub fn evaluate_counters(&self, name: &str, separator: &str, style: ListStyleType) -> String {
        match self.counters.get(name) {
            Some(stack) if !stack.is_empty() => stack
                .iter()
                .map(|&(_, v)| format_counter_value(v, style))
                .collect::<Vec<_>>()
                .join(separator),
            _ => format_counter_value(0, style),
        }
    }
}

impl Default for CounterState {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a counter value according to the list-style-type.
#[must_use]
pub fn format_counter_value(value: i32, style: ListStyleType) -> String {
    match style {
        ListStyleType::Decimal => value.to_string(),
        ListStyleType::DecimalLeadingZero => {
            if value < 0 {
                format!("-{:02}", value.unsigned_abs())
            } else {
                format!("{value:02}")
            }
        }
        ListStyleType::LowerRoman => to_roman(value, false),
        ListStyleType::UpperRoman => to_roman(value, true),
        ListStyleType::LowerAlpha | ListStyleType::LowerLatin => to_alpha(value, b'a'),
        ListStyleType::UpperAlpha | ListStyleType::UpperLatin => to_alpha(value, b'A'),
        ListStyleType::Disc => "\u{2022}".to_string(),
        ListStyleType::Circle => "\u{25E6}".to_string(),
        ListStyleType::Square => "\u{25AA}".to_string(),
        ListStyleType::None => String::new(),
    }
}

/// Roman numeral value-to-string table: (threshold, lowercase, uppercase).
const ROMAN_TABLE: &[(i32, &str, &str)] = &[
    (1000, "m", "M"),
    (900, "cm", "CM"),
    (500, "d", "D"),
    (400, "cd", "CD"),
    (100, "c", "C"),
    (90, "xc", "XC"),
    (50, "l", "L"),
    (40, "xl", "XL"),
    (10, "x", "X"),
    (9, "ix", "IX"),
    (5, "v", "V"),
    (4, "iv", "IV"),
    (1, "i", "I"),
];

/// Convert a value to a Roman numeral string. Returns the decimal
/// representation for non-positive values (CSS Lists L3 §7).
fn to_roman(value: i32, upper: bool) -> String {
    if value <= 0 {
        return value.to_string();
    }

    let mut result = String::new();
    let mut remaining = value;
    for &(threshold, lower, upper_str) in ROMAN_TABLE {
        while remaining >= threshold {
            result.push_str(if upper { upper_str } else { lower });
            remaining -= threshold;
        }
    }
    result
}

/// Convert a value to alphabetic representation (1=a, 26=z, 27=aa, ...).
/// Non-positive values fall back to decimal (CSS Lists L3 §7).
fn to_alpha(value: i32, base: u8) -> String {
    if value <= 0 {
        return value.to_string();
    }
    let mut result = Vec::new();
    let mut n = value.cast_unsigned();
    while n > 0 {
        n -= 1;
        result.push((base + (n % 26) as u8) as char);
        n /= 26;
    }
    result.reverse();
    result.into_iter().collect()
}

/// Add implicit `list-item` counter operations for `<ol>` and `<li>` elements.
///
/// Per CSS Lists L3 §5: `<ol>` implicitly resets the `list-item` counter,
/// and `<li>` implicitly increments it by 1. Explicit `counter-reset` /
/// `counter-increment` on these elements suppresses the implicit behavior.
pub fn apply_implicit_list_counters(
    style: &mut ComputedStyle,
    tag: &str,
    attrs: &elidex_ecs::Attributes,
) {
    match tag {
        "ol" => {
            let already_has = style.counter_reset.iter().any(|(n, _)| n == "list-item");
            if !already_has {
                // Use the `start` attribute value minus 1 (so first <li> increments to `start`),
                // defaulting to 0 (first item = 1).
                let start_value = attrs
                    .get("start")
                    .and_then(|s| s.parse::<i32>().ok())
                    .map_or(0, |s| s - 1);
                style
                    .counter_reset
                    .push(("list-item".to_string(), start_value));
            }
        }
        "ul" => {
            let already_has = style.counter_reset.iter().any(|(n, _)| n == "list-item");
            if !already_has {
                style.counter_reset.push(("list-item".to_string(), 0));
            }
        }
        "li" => {
            let already_has = style
                .counter_increment
                .iter()
                .any(|(n, _)| n == "list-item");
            if !already_has {
                let inc = attrs.get("value").and_then(|s| s.parse::<i32>().ok());
                if let Some(v) = inc {
                    // `value` attribute sets the counter to this value via counter-set.
                    let already_set = style.counter_set.iter().any(|(n, _)| n == "list-item");
                    if !already_set {
                        style.counter_set.push(("list-item".to_string(), v));
                    }
                } else {
                    style.counter_increment.push(("list-item".to_string(), 1));
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::ComputedStyle;

    #[test]
    fn basic_counter_reset_and_increment() {
        let mut cs = CounterState::new();

        // Simulate: element resets counter "c" to 0.
        let mut style = ComputedStyle::default();
        style.counter_reset.push(("c".to_string(), 0));
        cs.push_scope();
        cs.process_element(&style, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "0");

        // Simulate: child increments counter "c" by 1.
        let mut style2 = ComputedStyle::default();
        style2.counter_increment.push(("c".to_string(), 1));
        cs.push_scope();
        cs.process_element(&style2, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "1");
    }

    #[test]
    fn counter_reset_creates_scope() {
        let mut cs = CounterState::new();

        // Outer reset to 5.
        let mut style = ComputedStyle::default();
        style.counter_reset.push(("c".to_string(), 5));
        cs.push_scope();
        cs.process_element(&style, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "5");

        // Inner reset to 10 — creates a new scope entry, shadowing outer.
        let mut style2 = ComputedStyle::default();
        style2.counter_reset.push(("c".to_string(), 10));
        cs.push_scope();
        cs.process_element(&style2, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "10");
    }

    #[test]
    fn pop_scope_removes_counters() {
        let mut cs = CounterState::new();

        // Outer scope: reset to 5.
        let mut style = ComputedStyle::default();
        style.counter_reset.push(("c".to_string(), 5));
        cs.push_scope();
        cs.process_element(&style, false);

        // Inner scope: reset to 10.
        let mut style2 = ComputedStyle::default();
        style2.counter_reset.push(("c".to_string(), 10));
        cs.push_scope();
        cs.process_element(&style2, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "10");

        // Pop inner scope — should restore to 5.
        cs.pop_scope();
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "5");
    }

    #[test]
    fn nested_counter_scopes() {
        let mut cs = CounterState::new();

        // ol resets list-item to 0.
        let mut ol_style = ComputedStyle::default();
        ol_style.counter_reset.push(("list-item".to_string(), 0));
        cs.push_scope();
        cs.process_element(&ol_style, false);

        // li increments.
        let mut li_style = ComputedStyle::default();
        li_style
            .counter_increment
            .push(("list-item".to_string(), 1));
        cs.push_scope();
        cs.process_element(&li_style, false);
        assert_eq!(
            cs.evaluate_counter("list-item", ListStyleType::Decimal),
            "1"
        );
        cs.pop_scope();

        // Nested ol resets again.
        let mut inner_ol = ComputedStyle::default();
        inner_ol.counter_reset.push(("list-item".to_string(), 0));
        cs.push_scope();
        cs.process_element(&inner_ol, false);

        // Inner li increments.
        cs.push_scope();
        cs.process_element(&li_style, false);
        assert_eq!(
            cs.evaluate_counter("list-item", ListStyleType::Decimal),
            "1"
        );
        cs.pop_scope();

        // Pop inner ol scope.
        cs.pop_scope();

        // Second outer li — increments the outer counter.
        cs.push_scope();
        cs.process_element(&li_style, false);
        assert_eq!(
            cs.evaluate_counter("list-item", ListStyleType::Decimal),
            "2"
        );
        cs.pop_scope();
    }

    #[test]
    fn counters_concatenation() {
        let mut cs = CounterState::new();

        // Outer reset.
        let mut reset = ComputedStyle::default();
        reset.counter_reset.push(("c".to_string(), 0));
        cs.push_scope();
        cs.process_element(&reset, false);

        // Increment to 1.
        let mut inc = ComputedStyle::default();
        inc.counter_increment.push(("c".to_string(), 1));
        cs.push_scope();
        cs.process_element(&inc, false);

        // Nested reset.
        cs.push_scope();
        cs.process_element(&reset, false);

        // Nested increment to 1.
        cs.push_scope();
        cs.process_element(&inc, false);

        // counters() should show "1.1".
        assert_eq!(
            cs.evaluate_counters("c", ".", ListStyleType::Decimal),
            "1.1"
        );

        cs.pop_scope();
        cs.pop_scope();
        cs.pop_scope();

        // After popping inner scopes, only outer "1" remains.
        assert_eq!(cs.evaluate_counters("c", ".", ListStyleType::Decimal), "1");
    }

    #[test]
    fn counter_set_overwrites_value() {
        let mut cs = CounterState::new();

        // Reset to 0.
        let mut style = ComputedStyle::default();
        style.counter_reset.push(("c".to_string(), 0));
        cs.push_scope();
        cs.process_element(&style, false);

        // Set to 42.
        let mut set_style = ComputedStyle::default();
        set_style.counter_set.push(("c".to_string(), 42));
        cs.push_scope();
        cs.process_element(&set_style, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "42");
    }

    #[test]
    fn counter_set_creates_if_not_exists() {
        let mut cs = CounterState::new();

        // counter-set on a non-existent counter creates it (§5.3).
        let mut style = ComputedStyle::default();
        style.counter_set.push(("newc".to_string(), 7));
        cs.push_scope();
        cs.process_element(&style, false);
        assert_eq!(cs.evaluate_counter("newc", ListStyleType::Decimal), "7");
    }

    #[test]
    fn increment_skipped_on_continuation() {
        let mut cs = CounterState::new();

        let mut style = ComputedStyle::default();
        style.counter_reset.push(("c".to_string(), 0));
        cs.push_scope();
        cs.process_element(&style, false);

        // Normal increment.
        let mut inc = ComputedStyle::default();
        inc.counter_increment.push(("c".to_string(), 1));
        cs.push_scope();
        cs.process_element(&inc, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "1");
        cs.pop_scope();

        // Continuation — increment skipped.
        cs.push_scope();
        cs.process_element(&inc, true);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "1");
        cs.pop_scope();

        // Normal increment again.
        cs.push_scope();
        cs.process_element(&inc, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "2");
    }

    #[test]
    fn multiple_counters_on_one_element() {
        let mut cs = CounterState::new();

        let mut style = ComputedStyle::default();
        style.counter_reset.push(("a".to_string(), 10));
        style.counter_reset.push(("b".to_string(), 20));
        style.counter_increment.push(("a".to_string(), 5));
        cs.push_scope();
        cs.process_element(&style, false);

        // a was reset to 10 then incremented by 5.
        assert_eq!(cs.evaluate_counter("a", ListStyleType::Decimal), "15");
        // b was reset to 20, no increment.
        assert_eq!(cs.evaluate_counter("b", ListStyleType::Decimal), "20");
    }

    #[test]
    fn format_decimal_and_alpha() {
        assert_eq!(format_counter_value(1, ListStyleType::Decimal), "1");
        assert_eq!(format_counter_value(42, ListStyleType::Decimal), "42");
        assert_eq!(
            format_counter_value(1, ListStyleType::DecimalLeadingZero),
            "01"
        );
        assert_eq!(
            format_counter_value(10, ListStyleType::DecimalLeadingZero),
            "10"
        );

        // Alpha: 1=a, 26=z, 27=aa.
        assert_eq!(format_counter_value(1, ListStyleType::LowerAlpha), "a");
        assert_eq!(format_counter_value(26, ListStyleType::LowerAlpha), "z");
        assert_eq!(format_counter_value(27, ListStyleType::LowerAlpha), "aa");
        assert_eq!(format_counter_value(1, ListStyleType::UpperAlpha), "A");

        // Roman.
        assert_eq!(format_counter_value(4, ListStyleType::LowerRoman), "iv");
        assert_eq!(format_counter_value(9, ListStyleType::UpperRoman), "IX");
        assert_eq!(
            format_counter_value(2024, ListStyleType::LowerRoman),
            "mmxxiv"
        );

        // Bullets.
        assert_eq!(format_counter_value(1, ListStyleType::Disc), "\u{2022}");
        assert_eq!(format_counter_value(1, ListStyleType::Circle), "\u{25E6}");
        assert_eq!(format_counter_value(1, ListStyleType::Square), "\u{25AA}");

        // None.
        assert_eq!(format_counter_value(1, ListStyleType::None), "");

        // Non-positive alpha/roman fall back to decimal.
        assert_eq!(format_counter_value(0, ListStyleType::LowerAlpha), "0");
        assert_eq!(format_counter_value(-1, ListStyleType::LowerRoman), "-1");
    }
}
