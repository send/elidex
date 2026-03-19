//! CSS fragmentation keyword enums.

use std::fmt;

keyword_enum! {
    /// The CSS `break-before` / `break-after` property (CSS Fragmentation Level 3 §3.1).
    BreakValue {
        Auto => "auto",
        Avoid => "avoid",
        AvoidPage => "avoid-page",
        AvoidColumn => "avoid-column",
        Page => "page",
        Column => "column",
        Left => "left",
        Right => "right",
        Recto => "recto",
        Verso => "verso",
    }
}

keyword_enum! {
    /// The CSS `break-inside` property (CSS Fragmentation Level 3 §3.3).
    BreakInsideValue {
        Auto => "auto",
        Avoid => "avoid",
        AvoidPage => "avoid-page",
        AvoidColumn => "avoid-column",
    }
}

keyword_enum! {
    /// The CSS `box-decoration-break` property (CSS Fragmentation Level 3 §4).
    BoxDecorationBreak {
        Slice => "slice",
        Cloned => "clone",
    }
}
