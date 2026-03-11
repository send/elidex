//! Display, position, and overflow keyword enums.

use std::fmt;

keyword_enum! {
    /// The CSS `display` property.
    Display {
        Inline => "inline",
        Block => "block",
        InlineBlock => "inline-block",
        None => "none",
        Flex => "flex",
        InlineFlex => "inline-flex",
        ListItem => "list-item",
        Grid => "grid",
        InlineGrid => "inline-grid",
        Table => "table",
        InlineTable => "inline-table",
        TableCaption => "table-caption",
        TableRow => "table-row",
        TableCell => "table-cell",
        TableRowGroup => "table-row-group",
        TableHeaderGroup => "table-header-group",
        TableFooterGroup => "table-footer-group",
        TableColumn => "table-column",
        TableColumnGroup => "table-column-group",
        Contents => "contents",
    }
}

impl Display {
    /// CSS 2.1 §11.2: table-internal display types where `visibility: collapse`
    /// hides the entire row/column (rather than just hiding the content).
    #[must_use]
    pub fn is_table_internal(self) -> bool {
        matches!(
            self,
            Self::TableRow
                | Self::TableColumn
                | Self::TableRowGroup
                | Self::TableHeaderGroup
                | Self::TableFooterGroup
                | Self::TableColumnGroup
        )
    }
}

keyword_enum! {
    /// The CSS `position` property.
    Position {
        Static => "static",
        Relative => "relative",
        Absolute => "absolute",
        Fixed => "fixed",
        Sticky => "sticky",
    }
}

keyword_enum! {
    /// The CSS `overflow` property.
    ///
    /// CSS `scroll` and `auto` are mapped to `Hidden` during parsing
    /// (scrollbar rendering is deferred to Phase 4).
    Overflow {
        Visible => "visible",
        Hidden => "hidden",
    }
}
