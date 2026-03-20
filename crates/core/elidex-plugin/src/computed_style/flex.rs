//! Flexbox keyword enums.

use std::fmt;

keyword_enum! {
    /// The CSS `flex-direction` property.
    FlexDirection {
        Row => "row",
        RowReverse => "row-reverse",
        Column => "column",
        ColumnReverse => "column-reverse",
    }
}

keyword_enum! {
    /// The CSS `flex-wrap` property.
    FlexWrap {
        Nowrap => "nowrap",
        Wrap => "wrap",
        WrapReverse => "wrap-reverse",
    }
}

keyword_enum! {
    /// The CSS `justify-content` property.
    ///
    /// `Normal` is the CSS initial value (CSS Box Alignment L3 §6.5).
    /// In flex contexts it behaves as `flex-start`; in grid contexts
    /// it behaves as `stretch`.
    JustifyContent {
        Normal => "normal",
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        SpaceBetween => "space-between",
        SpaceAround => "space-around",
        SpaceEvenly => "space-evenly",
        Stretch => "stretch",
    }
}

keyword_enum! {
    /// The CSS `align-items` property.
    AlignItems {
        Stretch => "stretch",
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        Baseline => "baseline",
    }
}

keyword_enum! {
    /// The CSS `align-self` property.
    AlignSelf {
        Auto => "auto",
        Stretch => "stretch",
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        Baseline => "baseline",
    }
}

keyword_enum! {
    /// The CSS `align-content` property.
    ///
    /// `Normal` is the CSS initial value (CSS Box Alignment L3 §6.5).
    /// In flex contexts it behaves as `stretch`; in grid contexts
    /// it behaves as `stretch`.
    AlignContent {
        Normal => "normal",
        Stretch => "stretch",
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        SpaceBetween => "space-between",
        SpaceAround => "space-around",
        SpaceEvenly => "space-evenly",
    }
}

/// The CSS `flex-basis` property (CSS Flexbox §7.3).
///
/// Represents the initial main size of a flex item before free space distribution.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexBasis {
    /// `auto` — uses the item's main-size property as the basis.
    #[default]
    Auto,
    /// `content` — uses the item's max-content size as the basis.
    Content,
    /// A fixed length in pixels.
    Length(f32),
    /// A percentage of the flex container's main size.
    Percentage(f32),
}

/// CSS Box Alignment Level 3 `safe`/`unsafe` modifier.
///
/// When `Safe`, alignment falls back to `start` if the aligned content
/// overflows the alignment container (preventing data loss).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AlignmentSafety {
    /// No overflow safety — items may overflow (CSS default).
    #[default]
    Unsafe,
    /// Fall back to `start` alignment when items would overflow.
    Safe,
}
