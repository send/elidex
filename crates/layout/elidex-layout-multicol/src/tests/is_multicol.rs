//! `is_multicol` predicate tests.

use super::*;

#[test]
fn is_multicol_block_with_count() {
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(3),
        ..ComputedStyle::default()
    };
    assert!(is_multicol(&style));
}

#[test]
fn is_multicol_block_with_width() {
    let style = ComputedStyle {
        display: Display::Block,
        column_width: Dimension::Length(200.0),
        ..ComputedStyle::default()
    };
    assert!(is_multicol(&style));
}

#[test]
fn is_multicol_block_without_columns() {
    let style = ComputedStyle::default(); // display: block, no column props
    assert!(!is_multicol(&style));
}

#[test]
fn is_multicol_flex_false() {
    let style = ComputedStyle {
        display: Display::Flex,
        column_count: Some(3),
        ..ComputedStyle::default()
    };
    assert!(!is_multicol(&style));
}

#[test]
fn is_multicol_grid_false() {
    let style = ComputedStyle {
        display: Display::Grid,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    assert!(!is_multicol(&style));
}

#[test]
fn is_multicol_inline_block_true() {
    let style = ComputedStyle {
        display: Display::InlineBlock,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    assert!(is_multicol(&style));
}
