//! CSS Counter state machine (CSS Lists Level 3 §5–7).
//!
//! Tracks counter scopes during display list tree walk. Counters are created
//! by `counter-reset`, modified by `counter-increment` and `counter-set`,
//! and evaluated by `counter()` / `counters()` functions.

use std::collections::HashMap;

use elidex_plugin::{ComputedStyle, CounterResetEntry, ListStyleType};

/// A single counter instance in the counter stack.
#[derive(Clone, Debug)]
struct CounterInstance {
    scope_depth: usize,
    value: i32,
    /// Whether this counter counts in reverse (CSS Lists L3 §5.1).
    reversed: bool,
}

/// CSS Counter state machine.
///
/// Maintains a stack of counter instances per counter name,
/// allowing nested scopes to shadow outer counter instances.
pub struct CounterState {
    /// name → stack of counter instances.
    counters: HashMap<String, Vec<CounterInstance>>,
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
            while stack.last().is_some_and(|ci| ci.scope_depth == depth) {
                stack.pop();
            }
            !stack.is_empty()
        });
        self.scope_depth = self.scope_depth.saturating_sub(1);
    }

    /// Set a counter to a specific value at the current scope depth.
    ///
    /// If the counter already exists, its top value is overwritten.
    /// Otherwise, a new entry is created. Used by paged media layout
    /// to set the `page` and `pages` built-in counters.
    pub fn set_counter(&mut self, name: &str, value: i32) {
        let stack = self.counters.entry(name.to_string()).or_default();
        if let Some(top) = stack.last_mut() {
            top.value = value;
        } else {
            stack.push(CounterInstance {
                scope_depth: self.scope_depth,
                value,
                reversed: false,
            });
        }
    }

    /// Process counter properties from a computed style.
    ///
    /// Order of operations per CSS Lists L3 §5.4: reset → set → increment.
    ///
    /// `is_continuation` suppresses increment (used for fragmentation continuations).
    pub fn process_element(&mut self, style: &ComputedStyle, is_continuation: bool) {
        // Phase 1: counter-reset — creates new scope entries.
        for entry in &style.counter_reset {
            self.counters
                .entry(entry.name.clone())
                .or_default()
                .push(CounterInstance {
                    scope_depth: self.scope_depth,
                    value: entry.value,
                    reversed: entry.reversed,
                });
        }

        // Phase 2: counter-set — overwrites top value or creates if missing (§5.3).
        for (name, value) in &style.counter_set {
            let stack = self.counters.entry(name.clone()).or_default();
            if let Some(top) = stack.last_mut() {
                top.value = *value;
            } else {
                stack.push(CounterInstance {
                    scope_depth: self.scope_depth,
                    value: *value,
                    reversed: false,
                });
            }
        }

        // Phase 3: counter-increment — adds to top value or creates then increments (§5.2).
        // For reversed counters (CSS Lists L3 §5.1), the increment is negated.
        if !is_continuation {
            for (name, value) in &style.counter_increment {
                let stack = self.counters.entry(name.clone()).or_default();
                if let Some(top) = stack.last_mut() {
                    let effective = if top.reversed { -(*value) } else { *value };
                    top.value = top.value.saturating_add(effective);
                } else {
                    stack.push(CounterInstance {
                        scope_depth: self.scope_depth,
                        value: *value,
                        reversed: false,
                    });
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
            .map_or(0, |ci| ci.value);
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
                .map(|ci| format_counter_value(ci.value, style))
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
                // CSS Lists L3: decimal-leading-zero pads only non-negative values.
                // Negative values use plain decimal format.
                value.to_string()
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
/// Count `<li>` children of an element for reversed counter initialization.
///
/// Per HTML §4.4.8, a `<ol reversed>` without `start` has its counter set to
/// the number of owned `<li>` children.
#[must_use]
pub fn count_li_children(dom: &elidex_ecs::EcsDom, entity: elidex_ecs::Entity) -> usize {
    let mut count = 0;
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        if let Ok(tag) = dom.world().get::<&elidex_ecs::TagType>(c) {
            if tag.0 == "li" {
                count += 1;
            }
        }
        child = dom.get_next_sibling(c);
    }
    count
}

/// Add implicit `list-item` counter operations for `<ol>`, `<ul>`, `<li>`.
///
/// Per CSS Lists L3 §5: `<ol>` implicitly resets the `list-item` counter,
/// `<li>` implicitly increments it. The `reversed` attribute on `<ol>` creates
/// a reversed counter (CSS Lists L3 §5.1) with implicit decrement of -1.
///
/// `li_count` is the number of `<li>` children, used for reversed counter
/// initial value when no `start` attribute is present (HTML §4.4.8).
pub fn apply_implicit_list_counters(
    style: &mut ComputedStyle,
    tag: &str,
    attrs: &elidex_ecs::Attributes,
    li_count: usize,
) {
    match tag {
        "ol" => {
            let already_has = style
                .counter_reset
                .iter()
                .any(|e| e.name == "list-item");
            if !already_has {
                let is_reversed = attrs.contains("reversed");
                if is_reversed {
                    // Reversed counter: initial value = start attribute, or
                    // number of <li> children (HTML §4.4.8).
                    // Reversed counter: start_value + 1 so first <li> decrements
                    // to start_value. Without start attr, default = li_count.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                    let start_value = attrs
                        .get("start")
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(li_count.min(i32::MAX as usize) as i32)
                        .saturating_add(1);
                    style
                        .counter_reset
                        .push(CounterResetEntry::reversed("list-item", start_value));
                } else {
                    // Normal counter: first <li> increments to `start` (default 1).
                    let start_value = attrs
                        .get("start")
                        .and_then(|s| s.parse::<i32>().ok())
                        .map_or(0, |s| s - 1);
                    style
                        .counter_reset
                        .push(CounterResetEntry::new("list-item", start_value));
                }
            }
        }
        "ul" => {
            let already_has = style
                .counter_reset
                .iter()
                .any(|e| e.name == "list-item");
            if !already_has {
                style
                    .counter_reset
                    .push(CounterResetEntry::new("list-item", 0));
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
                    let already_set = style.counter_set.iter().any(|(n, _)| n == "list-item");
                    if !already_set {
                        style.counter_set.push(("list-item".to_string(), v));
                    }
                } else {
                    // Reversed counters use -1 increment; normal use +1.
                    // We check whether the parent <ol> is reversed by checking
                    // if the most recent list-item counter-reset is reversed.
                    // This info is passed via the `reversed` flag in CounterResetEntry.
                    // However, at this point we don't have parent info, so we
                    // always use +1. The reversed counter's higher initial value
                    // combined with +1 increment is wrong — we need -1 for reversed.
                    // We'll handle this in process_element by checking the counter
                    // stack's reversed flag.
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
        style.counter_reset.push(CounterResetEntry::new("c".to_string(), 0));
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
        style.counter_reset.push(CounterResetEntry::new("c".to_string(), 5));
        cs.push_scope();
        cs.process_element(&style, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "5");

        // Inner reset to 10 — creates a new scope entry, shadowing outer.
        let mut style2 = ComputedStyle::default();
        style2.counter_reset.push(CounterResetEntry::new("c".to_string(), 10));
        cs.push_scope();
        cs.process_element(&style2, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "10");
    }

    #[test]
    fn pop_scope_removes_counters() {
        let mut cs = CounterState::new();

        // Outer scope: reset to 5.
        let mut style = ComputedStyle::default();
        style.counter_reset.push(CounterResetEntry::new("c".to_string(), 5));
        cs.push_scope();
        cs.process_element(&style, false);

        // Inner scope: reset to 10.
        let mut style2 = ComputedStyle::default();
        style2.counter_reset.push(CounterResetEntry::new("c".to_string(), 10));
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
        ol_style.counter_reset.push(CounterResetEntry::new("list-item".to_string(), 0));
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
        inner_ol.counter_reset.push(CounterResetEntry::new("list-item".to_string(), 0));
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
        reset.counter_reset.push(CounterResetEntry::new("c".to_string(), 0));
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
        style.counter_reset.push(CounterResetEntry::new("c".to_string(), 0));
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
        style.counter_reset.push(CounterResetEntry::new("c".to_string(), 0));
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
        style.counter_reset.push(CounterResetEntry::new("a".to_string(), 10));
        style.counter_reset.push(CounterResetEntry::new("b".to_string(), 20));
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

        // Decimal-leading-zero: negative values use plain decimal (no zero-pad).
        assert_eq!(
            format_counter_value(-1, ListStyleType::DecimalLeadingZero),
            "-1"
        );
        assert_eq!(
            format_counter_value(-10, ListStyleType::DecimalLeadingZero),
            "-10"
        );
        // Zero still gets padded.
        assert_eq!(
            format_counter_value(0, ListStyleType::DecimalLeadingZero),
            "00"
        );

        // Non-positive alpha/roman fall back to decimal.
        assert_eq!(format_counter_value(0, ListStyleType::LowerAlpha), "0");
        assert_eq!(format_counter_value(-1, ListStyleType::LowerRoman), "-1");
    }

    #[test]
    fn reversed_counter_negates_increment() {
        let mut cs = CounterState::new();

        // Reset reversed counter to 5.
        let mut style = ComputedStyle::default();
        style
            .counter_reset
            .push(CounterResetEntry::reversed("c", 5));
        cs.push_scope();
        cs.process_element(&style, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "5");

        // Increment by 1 → reversed negates to -1 → value becomes 4.
        let mut inc = ComputedStyle::default();
        inc.counter_increment.push(("c".to_string(), 1));
        cs.push_scope();
        cs.process_element(&inc, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "4");
        cs.pop_scope();

        // Increment again → 3.
        cs.push_scope();
        cs.process_element(&inc, false);
        assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "3");
    }

    #[test]
    fn reversed_ol_implicit_counters() {
        // Simulate <ol reversed> with 3 <li> children (no start attribute).
        let mut cs = CounterState::new();

        // <ol reversed> → reversed counter reset to li_count (3).
        let mut ol_style = ComputedStyle::default();
        let mut attrs = elidex_ecs::Attributes::default();
        attrs.set("reversed", "");
        apply_implicit_list_counters(&mut ol_style, "ol", &attrs, 3);

        assert!(ol_style.counter_reset[0].reversed);
        // Initial value = li_count + 1 = 4, so first <li> decrements to 3.
        // Same pattern as normal: normal init=0, first <li> increments to 1.
        assert_eq!(ol_style.counter_reset[0].value, 4);

        cs.push_scope();
        cs.process_element(&ol_style, false);
        let mut li_style = ComputedStyle::default();
        li_style
            .counter_increment
            .push(("list-item".to_string(), 1));

        // First <li>
        cs.push_scope();
        cs.process_element(&li_style, false);
        assert_eq!(
            cs.evaluate_counter("list-item", ListStyleType::Decimal),
            "3",
            "first li in reversed ol with 3 items should show 3"
        );
        cs.pop_scope();

        // Second <li>
        cs.push_scope();
        cs.process_element(&li_style, false);
        assert_eq!(
            cs.evaluate_counter("list-item", ListStyleType::Decimal),
            "2"
        );
        cs.pop_scope();

        // Third <li>
        cs.push_scope();
        cs.process_element(&li_style, false);
        assert_eq!(
            cs.evaluate_counter("list-item", ListStyleType::Decimal),
            "1"
        );
    }
}
