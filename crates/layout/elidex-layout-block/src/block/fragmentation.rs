//! CSS Fragmentation Level 3 helper functions.
//!
//! Pure helper functions for fragmentation decisions — forced break detection,
//! avoid-break checks, monolithic element classification, and best-break
//! selection. No side effects; used by `stack_block_children` and
//! `layout_block_inner`.

use elidex_plugin::{BreakInsideValue, BreakValue, ComputedStyle, Overflow};

use crate::FragmentationType;

// ---------------------------------------------------------------------------
// Forced break detection (§3.1)
// ---------------------------------------------------------------------------

/// CSS Fragmentation L3 §3.1: check if a break value forces a break.
///
/// Page/Left/Right/Recto/Verso force in all contexts (§3.1: "A page break
/// opportunity also makes a column break"). Column forces only in column
/// contexts, not standalone page contexts.
#[must_use]
pub fn is_forced_break(value: BreakValue, frag_type: FragmentationType) -> bool {
    match value {
        BreakValue::Page
        | BreakValue::Left
        | BreakValue::Right
        | BreakValue::Recto
        | BreakValue::Verso => true,
        BreakValue::Column => matches!(frag_type, FragmentationType::Column),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Avoid-break checks (§3.3-§3.4)
// ---------------------------------------------------------------------------

/// Check if `break-inside` requests avoiding breaks.
#[must_use]
pub fn is_avoid_break_inside(inside: BreakInsideValue, frag_type: FragmentationType) -> bool {
    match inside {
        BreakInsideValue::Avoid => true,
        BreakInsideValue::AvoidPage => matches!(frag_type, FragmentationType::Page),
        BreakInsideValue::AvoidColumn => matches!(frag_type, FragmentationType::Column),
        BreakInsideValue::Auto => false,
    }
}

/// Check if `break-before`/`break-after` requests avoiding breaks.
///
/// Used for break candidate penalty scoring (§3.4).
#[must_use]
pub fn is_avoid_break_value(value: BreakValue, frag_type: FragmentationType) -> bool {
    match value {
        BreakValue::Avoid => true,
        BreakValue::AvoidPage => matches!(frag_type, FragmentationType::Page),
        BreakValue::AvoidColumn => matches!(frag_type, FragmentationType::Column),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Monolithic element detection (§4)
// ---------------------------------------------------------------------------

/// CSS Fragmentation L3 §4: monolithic elements cannot be fragmented.
///
/// Replaced elements, elements with `overflow != visible`, and transformed
/// elements are monolithic.
#[must_use]
pub fn is_monolithic(style: &ComputedStyle, has_intrinsic: bool) -> bool {
    has_intrinsic
        || style.overflow_x != Overflow::Visible
        || style.overflow_y != Overflow::Visible
        || style.has_transform
}

// ---------------------------------------------------------------------------
// Break candidate selection (§3.3-§3.4)
// ---------------------------------------------------------------------------

/// CSS Fragmentation L3 §3.3: break point classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub enum BreakClass {
    /// Class A: between block-level siblings, or between line boxes.
    A,
    /// Class B: before first / after last child of a container.
    B,
    /// Class C: before/after the content of a box (least preferred).
    C,
}

/// A candidate break point within a fragmentation context.
pub struct BreakCandidate {
    /// Index of the child after which to break (or before which, for Class B).
    pub child_index: usize,
    /// Break class (A, B, or C).
    pub class: BreakClass,
    /// Block-axis cursor position at this candidate.
    pub cursor_block: f32,
    /// Whether breaking here violates an `avoid` constraint.
    pub violates_avoid: bool,
    /// Whether breaking here violates orphan/widow constraints.
    pub orphan_widow_penalty: bool,
}

/// CSS Fragmentation L3 §3.4: select the best break point.
///
/// Among candidates that fit within `available` space:
/// 1. Non-avoid over avoid
/// 2. No orphan/widow penalty over penalty
/// 3. Class A > B > C (lower enum ordinal = preferred)
///
/// If NO candidate fits, returns the first candidate (break as early as
/// possible to minimize overflow).
///
/// Returns the index into `candidates`, or `None` if empty.
#[must_use]
pub fn find_best_break(candidates: &[BreakCandidate], available: f32) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }

    // Partition into fitting and non-fitting candidates.
    let fitting: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.cursor_block <= available)
        .map(|(i, _)| i)
        .collect();

    if fitting.is_empty() {
        // Last-resort: first candidate (break as early as possible).
        return Some(0);
    }

    // Sort fitting candidates by preference:
    // 1. violates_avoid: false < true
    // 2. orphan_widow_penalty: false < true
    // 3. class: A < B < C
    // Among equal preference, prefer later position (more content in this fragment).
    let mut best = fitting[0];
    for &idx in &fitting[1..] {
        let cur = &candidates[best];
        let cand = &candidates[idx];
        let cur_key = (cur.violates_avoid, cur.orphan_widow_penalty, cur.class);
        let cand_key = (cand.violates_avoid, cand.orphan_widow_penalty, cand.class);
        if cand_key < cur_key || (cand_key == cur_key && cand.cursor_block >= cur.cursor_block) {
            best = idx;
        }
    }
    Some(best)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forced_break_page_forces_in_all_contexts() {
        assert!(is_forced_break(BreakValue::Page, FragmentationType::Page));
        assert!(is_forced_break(BreakValue::Page, FragmentationType::Column));
        assert!(is_forced_break(BreakValue::Left, FragmentationType::Page));
        assert!(is_forced_break(
            BreakValue::Right,
            FragmentationType::Column
        ));
        assert!(is_forced_break(BreakValue::Recto, FragmentationType::Page));
        assert!(is_forced_break(
            BreakValue::Verso,
            FragmentationType::Column
        ));
    }

    #[test]
    fn forced_break_column_only_in_column_context() {
        assert!(is_forced_break(
            BreakValue::Column,
            FragmentationType::Column
        ));
        assert!(!is_forced_break(
            BreakValue::Column,
            FragmentationType::Page
        ));
    }

    #[test]
    fn forced_break_auto_and_avoid_are_not_forced() {
        assert!(!is_forced_break(BreakValue::Auto, FragmentationType::Page));
        assert!(!is_forced_break(BreakValue::Avoid, FragmentationType::Page));
        assert!(!is_forced_break(
            BreakValue::AvoidPage,
            FragmentationType::Page
        ));
        assert!(!is_forced_break(
            BreakValue::AvoidColumn,
            FragmentationType::Column
        ));
    }

    #[test]
    fn avoid_break_inside_checks() {
        assert!(is_avoid_break_inside(
            BreakInsideValue::Avoid,
            FragmentationType::Page
        ));
        assert!(is_avoid_break_inside(
            BreakInsideValue::Avoid,
            FragmentationType::Column
        ));
        assert!(is_avoid_break_inside(
            BreakInsideValue::AvoidPage,
            FragmentationType::Page
        ));
        assert!(!is_avoid_break_inside(
            BreakInsideValue::AvoidPage,
            FragmentationType::Column
        ));
        assert!(is_avoid_break_inside(
            BreakInsideValue::AvoidColumn,
            FragmentationType::Column
        ));
        assert!(!is_avoid_break_inside(
            BreakInsideValue::AvoidColumn,
            FragmentationType::Page
        ));
        assert!(!is_avoid_break_inside(
            BreakInsideValue::Auto,
            FragmentationType::Page
        ));
    }

    #[test]
    fn avoid_break_value_checks() {
        assert!(is_avoid_break_value(
            BreakValue::Avoid,
            FragmentationType::Page
        ));
        assert!(is_avoid_break_value(
            BreakValue::Avoid,
            FragmentationType::Column
        ));
        assert!(is_avoid_break_value(
            BreakValue::AvoidPage,
            FragmentationType::Page
        ));
        assert!(!is_avoid_break_value(
            BreakValue::AvoidPage,
            FragmentationType::Column
        ));
        assert!(!is_avoid_break_value(
            BreakValue::Auto,
            FragmentationType::Page
        ));
        assert!(!is_avoid_break_value(
            BreakValue::Page,
            FragmentationType::Page
        ));
    }

    #[test]
    fn monolithic_replaced() {
        let style = ComputedStyle::default();
        assert!(is_monolithic(&style, true));
        assert!(!is_monolithic(&style, false));
    }

    #[test]
    fn monolithic_overflow() {
        let style = ComputedStyle {
            overflow_x: Overflow::Hidden,
            ..ComputedStyle::default()
        };
        assert!(is_monolithic(&style, false));

        let style2 = ComputedStyle {
            overflow_y: Overflow::Scroll,
            ..ComputedStyle::default()
        };
        assert!(is_monolithic(&style2, false));
    }

    #[test]
    fn monolithic_transform() {
        let style = ComputedStyle {
            has_transform: true,
            ..ComputedStyle::default()
        };
        assert!(is_monolithic(&style, false));
    }

    #[test]
    fn find_best_break_empty() {
        assert_eq!(find_best_break(&[], 100.0), None);
    }

    #[test]
    fn find_best_break_prefers_non_avoid() {
        let candidates = vec![
            BreakCandidate {
                child_index: 0,
                class: BreakClass::A,
                cursor_block: 30.0,
                violates_avoid: true,
                orphan_widow_penalty: false,
            },
            BreakCandidate {
                child_index: 1,
                class: BreakClass::A,
                cursor_block: 60.0,
                violates_avoid: false,
                orphan_widow_penalty: false,
            },
        ];
        assert_eq!(find_best_break(&candidates, 100.0), Some(1));
    }

    #[test]
    fn find_best_break_orphan_widow_penalty() {
        let candidates = vec![
            BreakCandidate {
                child_index: 0,
                class: BreakClass::A,
                cursor_block: 30.0,
                violates_avoid: false,
                orphan_widow_penalty: true,
            },
            BreakCandidate {
                child_index: 1,
                class: BreakClass::A,
                cursor_block: 60.0,
                violates_avoid: false,
                orphan_widow_penalty: false,
            },
        ];
        assert_eq!(find_best_break(&candidates, 100.0), Some(1));
    }

    #[test]
    fn find_best_break_last_resort_when_none_fit() {
        let candidates = vec![
            BreakCandidate {
                child_index: 0,
                class: BreakClass::A,
                cursor_block: 150.0,
                violates_avoid: false,
                orphan_widow_penalty: false,
            },
            BreakCandidate {
                child_index: 1,
                class: BreakClass::A,
                cursor_block: 200.0,
                violates_avoid: false,
                orphan_widow_penalty: false,
            },
        ];
        // None fit, so pick first candidate.
        assert_eq!(find_best_break(&candidates, 100.0), Some(0));
    }

    #[test]
    fn find_best_break_prefers_class_a_over_b() {
        let candidates = vec![
            BreakCandidate {
                child_index: 0,
                class: BreakClass::B,
                cursor_block: 30.0,
                violates_avoid: false,
                orphan_widow_penalty: false,
            },
            BreakCandidate {
                child_index: 1,
                class: BreakClass::A,
                cursor_block: 60.0,
                violates_avoid: false,
                orphan_widow_penalty: false,
            },
        ];
        assert_eq!(find_best_break(&candidates, 100.0), Some(1));
    }
}
