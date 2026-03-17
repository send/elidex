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
    /// The CSS `overflow` property (per-axis value).
    Overflow {
        Visible => "visible",
        Hidden => "hidden",
        Scroll => "scroll",
        Auto => "auto",
        Clip => "clip",
    }
}

impl Overflow {
    /// Returns `true` if this value creates a scroll container (CSS Overflow L3 §2).
    #[must_use]
    pub fn is_scroll_container(self) -> bool {
        matches!(self, Self::Scroll | Self::Auto)
    }

    /// Returns `true` if programmatic scrolling is allowed.
    #[must_use]
    pub fn allows_programmatic_scroll(self) -> bool {
        matches!(self, Self::Hidden | Self::Scroll | Self::Auto)
    }

    /// Returns `true` if this value clips overflow content.
    #[must_use]
    pub fn clips(self) -> bool {
        self != Self::Visible
    }
}

/// Viewport-level overflow propagated from root/body (CSS Overflow L3 §3.1).
///
/// Default is `Auto/Auto` (viewport `visible` is treated as `auto` — §3.1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewportOverflow {
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
}

impl Default for ViewportOverflow {
    fn default() -> Self {
        // CSS Overflow L3 §3.1: visible on the viewport is interpreted as auto.
        Self {
            overflow_x: Overflow::Auto,
            overflow_y: Overflow::Auto,
        }
    }
}

impl ViewportOverflow {
    /// CSS Overflow L3 §3.1: Normalize overflow values for the viewport.
    /// - `visible` → `auto` (viewport cannot be visible)
    /// - `clip` → `hidden` (clip not supported on viewport)
    fn normalize_for_viewport(overflow: Overflow) -> Overflow {
        match overflow {
            Overflow::Visible => Overflow::Auto,
            Overflow::Clip => Overflow::Hidden,
            other => other,
        }
    }

    /// Returns `true` if viewport scrolling is allowed on either axis.
    #[must_use]
    pub fn allows_scroll(&self) -> bool {
        self.overflow_x.is_scroll_container() || self.overflow_y.is_scroll_container()
    }

    /// Create from propagated overflow values, applying viewport normalization.
    #[must_use]
    pub fn from_propagated(overflow_x: Overflow, overflow_y: Overflow) -> Self {
        Self {
            overflow_x: Self::normalize_for_viewport(overflow_x),
            overflow_y: Self::normalize_for_viewport(overflow_y),
        }
    }
}
