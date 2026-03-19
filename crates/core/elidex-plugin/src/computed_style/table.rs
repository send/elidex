//! CSS Table keyword enums.

use std::fmt;

keyword_enum! {
    /// The CSS `border-collapse` property (CSS 2.1 §17.6).
    BorderCollapse {
        Separate => "separate",
        Collapse => "collapse",
    }
}

keyword_enum! {
    /// The CSS `table-layout` property (CSS 2.1 §17.5.2).
    TableLayout {
        Auto => "auto",
        Fixed => "fixed",
    }
}

keyword_enum! {
    /// The CSS `caption-side` property (CSS 2.1 §17.4.1).
    CaptionSide {
        Top => "top",
        Bottom => "bottom",
    }
}

keyword_enum! {
    /// The CSS `empty-cells` property (CSS 2.1 §17.5.1).
    EmptyCells {
        Show => "show",
        Hide => "hide",
    }
}
