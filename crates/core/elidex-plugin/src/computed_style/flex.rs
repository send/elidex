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
    JustifyContent {
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        SpaceBetween => "space-between",
        SpaceAround => "space-around",
        SpaceEvenly => "space-evenly",
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
    AlignContent {
        Stretch => "stretch",
        FlexStart => "flex-start",
        FlexEnd => "flex-end",
        Center => "center",
        SpaceBetween => "space-between",
        SpaceAround => "space-around",
        SpaceEvenly => "space-evenly",
    }
}
