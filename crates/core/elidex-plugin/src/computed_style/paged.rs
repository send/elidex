//! CSS Paged Media Level 3 types.
//!
//! Types for `@page` rule representation: page selectors, page size,
//! margin boxes, and paged media layout context.

use crate::{ContentValue, EdgeSizes, PropertyDeclaration};

/// CSS Paged Media L3 §4.1 page pseudo-classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageSelector {
    /// `:first` — matches the first page.
    First,
    /// `:left` — matches left pages (even pages in LTR).
    Left,
    /// `:right` — matches right pages (odd pages in LTR).
    Right,
    /// `:blank` — matches intentionally blank pages.
    Blank,
}

impl PageSelector {
    /// Parse a page pseudo-class name (without the leading colon).
    #[must_use]
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "first" => Some(Self::First),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "blank" => Some(Self::Blank),
            _ => None,
        }
    }

    /// Check whether this selector matches the given page.
    ///
    /// Page numbers are 1-based. `:left` matches even pages, `:right`
    /// matches odd pages (LTR convention per CSS Paged Media L3 §4.1).
    #[must_use]
    pub fn matches(self, page_number: usize, is_blank: bool) -> bool {
        match self {
            Self::First => page_number == 1,
            Self::Left => page_number.is_multiple_of(2),
            Self::Right => !page_number.is_multiple_of(2),
            Self::Blank => is_blank,
        }
    }
}

/// Named page sizes (CSS Paged Media L3 §7.1).
///
/// Dimensions follow ISO 216 / ANSI standards at 96 DPI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedPageSize {
    /// A5: 148mm × 210mm.
    A5,
    /// A4: 210mm × 297mm.
    A4,
    /// A3: 297mm × 420mm.
    A3,
    /// B5 (ISO): 176mm × 250mm.
    B5,
    /// B4 (ISO): 250mm × 353mm.
    B4,
    /// Letter: 8.5in × 11in.
    Letter,
    /// Legal: 8.5in × 14in.
    Legal,
    /// Ledger: 11in × 17in.
    Ledger,
}

impl NamedPageSize {
    /// Returns (width\_px, height\_px) at 96 DPI (portrait orientation).
    ///
    /// Conversion: 1 in = 96 px, 1 mm = 96/25.4 px ≈ 3.7795 px.
    #[must_use]
    pub fn dimensions(self) -> (f32, f32) {
        // mm_to_px: value_mm * 96.0 / 25.4, rounded to nearest integer
        match self {
            Self::A5 => (559.0, 794.0),       // 148mm × 210mm
            Self::A4 => (794.0, 1123.0),      // 210mm × 297mm
            Self::A3 => (1123.0, 1587.0),     // 297mm × 420mm
            Self::B5 => (665.0, 945.0),       // 176mm × 250mm
            Self::B4 => (945.0, 1334.0),      // 250mm × 353mm
            Self::Letter => (816.0, 1056.0),  // 8.5in × 11in
            Self::Legal => (816.0, 1344.0),   // 8.5in × 14in
            Self::Ledger => (1056.0, 1632.0), // 11in × 17in
        }
    }

    /// Parse a named page size keyword (case-insensitive).
    #[must_use]
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "a5" => Some(Self::A5),
            "a4" => Some(Self::A4),
            "a3" => Some(Self::A3),
            "b5" => Some(Self::B5),
            "b4" => Some(Self::B4),
            "letter" => Some(Self::Letter),
            "legal" => Some(Self::Legal),
            "ledger" => Some(Self::Ledger),
            _ => None,
        }
    }
}

/// Content and style for a single page-margin box.
#[derive(Clone, Debug)]
pub struct MarginBoxContent {
    /// The `content` property for this margin box.
    pub content: ContentValue,
    /// Additional property declarations.
    pub properties: Vec<PropertyDeclaration>,
}

/// Device sheet and default margins, also used to represent selected sheet geometry.
#[derive(Clone, Copy, Debug)]
pub struct PageGeometry {
    pub sheet: crate::Size,
    pub margins: EdgeSizes,
}

impl PageGeometry {
    #[must_use]
    pub fn area(self) -> crate::Rect {
        crate::Rect::new(
            self.margins.left,
            self.margins.top,
            (self.sheet.width - self.margins.left - self.margins.right).max(0.0),
            (self.sheet.height - self.margins.top - self.margins.bottom).max(0.0),
        )
    }
}

/// Selected page properties, prepared before paginated layout.
#[derive(Clone, Debug)]
pub struct PreparedPage {
    pub geometry: PageGeometry,
    pub matched_selectors: Vec<PageSelector>,
    pub boxes: Vec<(String, MarginBoxContent)>,
}

/// Prepared page identities for the first page and both sides of subsequent spreads.
#[derive(Clone, Debug)]
pub struct PreparedPages {
    first: PagePair,
    even: PagePair,
    odd: PagePair,
}

#[derive(Clone, Debug)]
struct PagePair {
    content: PreparedPage,
    blank: PreparedPage,
}

impl PreparedPages {
    /// The ordinary first page is already prepared because its area establishes
    /// the document viewport. Request each remaining identity exactly once.
    #[must_use]
    pub fn from_first(
        first: PreparedPage,
        mut prepare: impl FnMut(usize, bool) -> PreparedPage,
    ) -> Self {
        let first = PagePair {
            content: first,
            blank: prepare(1, true),
        };
        let mut pair = |number| PagePair {
            content: prepare(number, false),
            blank: prepare(number, true),
        };
        Self {
            first,
            even: pair(2),
            odd: pair(3),
        }
    }

    #[must_use]
    pub fn select(&self, number: usize, blank: bool) -> &PreparedPage {
        let pair = if number == 1 {
            &self.first
        } else if number.is_multiple_of(2) {
            &self.even
        } else {
            &self.odd
        };
        if blank {
            &pair.blank
        } else {
            &pair.content
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_page_size_a4_dimensions() {
        let (w, h) = NamedPageSize::A4.dimensions();
        assert_eq!(w, 794.0);
        assert_eq!(h, 1123.0);
    }

    #[test]
    fn named_page_size_letter_dimensions() {
        let (w, h) = NamedPageSize::Letter.dimensions();
        assert_eq!(w, 816.0);
        assert_eq!(h, 1056.0);
    }

    #[test]
    fn named_page_size_from_keyword() {
        assert_eq!(NamedPageSize::from_keyword("A4"), Some(NamedPageSize::A4));
        assert_eq!(NamedPageSize::from_keyword("a4"), Some(NamedPageSize::A4));
        assert_eq!(
            NamedPageSize::from_keyword("letter"),
            Some(NamedPageSize::Letter)
        );
        assert_eq!(NamedPageSize::from_keyword("unknown"), None);
    }

    #[test]
    fn page_selector_from_keyword() {
        assert_eq!(
            PageSelector::from_keyword("first"),
            Some(PageSelector::First)
        );
        assert_eq!(PageSelector::from_keyword("LEFT"), Some(PageSelector::Left));
        assert_eq!(
            PageSelector::from_keyword("Right"),
            Some(PageSelector::Right)
        );
        assert_eq!(
            PageSelector::from_keyword("blank"),
            Some(PageSelector::Blank)
        );
        assert_eq!(PageSelector::from_keyword("other"), None);
    }

    #[test]
    fn all_named_sizes_have_valid_dimensions() {
        let sizes = [
            NamedPageSize::A5,
            NamedPageSize::A4,
            NamedPageSize::A3,
            NamedPageSize::B5,
            NamedPageSize::B4,
            NamedPageSize::Letter,
            NamedPageSize::Legal,
            NamedPageSize::Ledger,
        ];
        for size in &sizes {
            let (w, h) = size.dimensions();
            assert!(w > 0.0, "{size:?} width should be positive");
            assert!(h > 0.0, "{size:?} height should be positive");
            assert!(h > w, "{size:?} should be portrait (h > w)");
        }
    }
}
