//! CSS Multi-column keyword enums.

use std::fmt;

keyword_enum! {
    /// The CSS `column-fill` property (CSS Multi-column Layout Level 1 §7).
    ColumnFill {
        Balance => "balance",
        Auto => "auto",
    }
}

keyword_enum! {
    /// The CSS `column-span` property (CSS Multi-column Layout Level 1 §6).
    ColumnSpan {
        None => "none",
        All => "all",
    }
}
