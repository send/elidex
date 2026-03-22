//! CSS Paged Media Level 3 types.
//!
//! Types for `@page` rule representation: page selectors, page size,
//! margin boxes, and paged media layout context.

use crate::{ContentValue, EdgeSizes, PropertyDeclaration};

/// CSS Paged Media L3 `@page` rule.
#[derive(Clone, Debug, Default)]
pub struct PageRule {
    /// Page pseudo-class selectors (`:first`, `:left`, `:right`, `:blank`).
    pub selectors: Vec<PageSelector>,
    /// The `size` property value.
    pub size: Option<PageSize>,
    /// Page margin box definitions (CSS Paged Media L3 §4.2).
    pub margins: PageMargins,
    /// Other property declarations inside `@page`.
    pub properties: Vec<PropertyDeclaration>,
}

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

/// CSS Paged Media L3 §7 `size` property value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageSize {
    /// `size: auto` — UA default page size.
    Auto,
    /// `size: <width> <height>` — explicit dimensions in px.
    Explicit(f32, f32),
    /// `size: <named>` — named page size (portrait orientation).
    Named(NamedPageSize),
    /// `size: <named> landscape` — named page size with landscape orientation.
    LandscapeNamed(NamedPageSize),
    /// `size: <width> <height>` with explicit landscape dimensions.
    LandscapeExplicit(f32, f32),
    /// `size: <named> portrait` — named page size with explicit portrait orientation.
    PortraitNamed(NamedPageSize),
    /// `size: <width> <height>` with explicit portrait dimensions.
    PortraitExplicit(f32, f32),
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

/// 16 page-margin box types (CSS Paged Media L3 §4.2).
#[derive(Clone, Debug, Default)]
pub struct PageMargins {
    /// `@top-left-corner`
    pub top_left_corner: Option<MarginBoxContent>,
    /// `@top-left`
    pub top_left: Option<MarginBoxContent>,
    /// `@top-center`
    pub top_center: Option<MarginBoxContent>,
    /// `@top-right`
    pub top_right: Option<MarginBoxContent>,
    /// `@top-right-corner`
    pub top_right_corner: Option<MarginBoxContent>,
    /// `@right-top`
    pub right_top: Option<MarginBoxContent>,
    /// `@right-middle`
    pub right_middle: Option<MarginBoxContent>,
    /// `@right-bottom`
    pub right_bottom: Option<MarginBoxContent>,
    /// `@bottom-right-corner`
    pub bottom_right_corner: Option<MarginBoxContent>,
    /// `@bottom-right`
    pub bottom_right: Option<MarginBoxContent>,
    /// `@bottom-center`
    pub bottom_center: Option<MarginBoxContent>,
    /// `@bottom-left`
    pub bottom_left: Option<MarginBoxContent>,
    /// `@bottom-left-corner`
    pub bottom_left_corner: Option<MarginBoxContent>,
    /// `@left-bottom`
    pub left_bottom: Option<MarginBoxContent>,
    /// `@left-middle`
    pub left_middle: Option<MarginBoxContent>,
    /// `@left-top`
    pub left_top: Option<MarginBoxContent>,
}

/// Content and style for a single page-margin box.
#[derive(Clone, Debug)]
pub struct MarginBoxContent {
    /// The `content` property for this margin box.
    pub content: ContentValue,
    /// Additional property declarations.
    pub properties: Vec<PropertyDeclaration>,
}

/// Context for paged media layout.
#[derive(Clone, Debug)]
pub struct PagedMediaContext {
    /// Page width in px.
    pub page_width: f32,
    /// Page height in px.
    pub page_height: f32,
    /// Page margins (top, right, bottom, left) in px.
    pub page_margins: EdgeSizes,
    /// `@page` rules from stylesheets.
    pub page_rules: Vec<PageRule>,
}

impl PagedMediaContext {
    /// Compute the content area dimensions (page size minus margins).
    #[must_use]
    pub fn content_width(&self) -> f32 {
        (self.page_width - self.page_margins.left - self.page_margins.right).max(0.0)
    }

    /// Compute the content area height (page height minus margins).
    #[must_use]
    pub fn content_height(&self) -> f32 {
        (self.page_height - self.page_margins.top - self.page_margins.bottom).max(0.0)
    }

    /// Resolve the effective page size for a given page, considering `@page`
    /// rules with matching selectors. Returns `(width, height)` in px.
    ///
    /// Rules are applied in specificity order per CSS Paged Media L3 §4.1:
    /// fewer selectors first, more selectors later (higher specificity wins).
    /// Rules with equal specificity use source order (last wins).
    #[must_use]
    pub fn effective_page_size(&self, page_number: usize, is_blank: bool) -> (f32, f32) {
        let mut width = self.page_width;
        let mut height = self.page_height;

        let mut sorted: Vec<(usize, &PageRule)> = self
            .page_rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| selectors_match(&rule.selectors, page_number, is_blank))
            .collect();
        // Sort by (selector count, source order) — lower specificity first.
        sorted.sort_by_key(|(idx, rule)| (rule.selectors.len(), *idx));

        for (_, rule) in sorted {
            if let Some(size) = &rule.size {
                let (w, h) = resolve_page_size_dimensions(size);
                width = w;
                height = h;
            }
        }
        (width, height)
    }

    /// Resolve the effective page margins for a given page, considering `@page`
    /// rules with matching selectors.
    ///
    /// Rules are applied in specificity order per CSS Paged Media L3 §4.1:
    /// fewer selectors first, more selectors later (higher specificity wins).
    /// Rules with equal specificity use source order (last wins).
    #[must_use]
    pub fn effective_margins(&self, page_number: usize, is_blank: bool) -> EdgeSizes {
        let mut margins = self.page_margins;

        let mut sorted: Vec<(usize, &PageRule)> = self
            .page_rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| selectors_match(&rule.selectors, page_number, is_blank))
            .collect();
        // Sort by (selector count, source order) — lower specificity first.
        sorted.sort_by_key(|(idx, rule)| (rule.selectors.len(), *idx));

        for (_, rule) in sorted {
            // Apply margin declarations from the rule.
            for decl in &rule.properties {
                apply_margin_declaration(&mut margins, decl);
            }
        }
        margins
    }
}

/// Check whether page selectors match a given page.
///
/// Empty selectors match all pages (`.all()` on an empty iterator returns
/// `true`). Combined selectors use AND semantics per CSS Paged Media L3 §4.1.
#[must_use]
pub fn selectors_match(selectors: &[PageSelector], page_number: usize, is_blank: bool) -> bool {
    selectors.iter().all(|s| s.matches(page_number, is_blank))
}

/// Resolve a `PageSize` to `(width, height)` in px.
#[must_use]
fn resolve_page_size_dimensions(size: &PageSize) -> (f32, f32) {
    match *size {
        PageSize::Auto => (816.0, 1056.0), // Letter default
        PageSize::Explicit(w, h) | PageSize::PortraitExplicit(w, h) => (w, h),
        PageSize::Named(named) | PageSize::PortraitNamed(named) => named.dimensions(),
        PageSize::LandscapeNamed(named) => {
            let (w, h) = named.dimensions();
            (h, w)
        }
        PageSize::LandscapeExplicit(w, h) => (h, w),
    }
}

/// Apply a margin-related property declaration to `EdgeSizes`.
fn apply_margin_declaration(margins: &mut EdgeSizes, decl: &PropertyDeclaration) {
    use crate::CssValue;
    let px = match &decl.value {
        CssValue::Length(v, _unit) => *v,
        CssValue::Number(v) if *v == 0.0 => 0.0,
        _ => return,
    };
    match decl.property.as_str() {
        "margin-top" => margins.top = px,
        "margin-right" => margins.right = px,
        "margin-bottom" => margins.bottom = px,
        "margin-left" => margins.left = px,
        _ => {}
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
    fn page_rule_default() {
        let rule = PageRule::default();
        assert!(rule.selectors.is_empty());
        assert!(rule.size.is_none());
        assert!(rule.properties.is_empty());
    }

    #[test]
    fn effective_page_size_specificity_order() {
        // More specific selectors (more pseudo-classes) should win over
        // less specific ones, regardless of source order.

        let ctx = PagedMediaContext {
            page_width: 816.0,
            page_height: 1056.0,
            page_margins: EdgeSizes {
                top: 72.0,
                right: 72.0,
                bottom: 72.0,
                left: 72.0,
            },
            page_rules: vec![
                // More specific rule (:first:right) — listed first in source.
                PageRule {
                    selectors: vec![PageSelector::First, PageSelector::Right],
                    size: Some(PageSize::Explicit(500.0, 700.0)),
                    ..PageRule::default()
                },
                // Less specific rule (:first) — listed second in source.
                PageRule {
                    selectors: vec![PageSelector::First],
                    size: Some(PageSize::Explicit(600.0, 800.0)),
                    ..PageRule::default()
                },
            ],
        };

        // Page 1 is :first and :right. The :first:right rule (2 selectors)
        // has higher specificity and should win, even though :first appears
        // later in source order.
        let (w, h) = ctx.effective_page_size(1, false);
        assert_eq!(w, 500.0);
        assert_eq!(h, 700.0);
    }

    #[test]
    fn effective_margins_specificity_order() {
        use crate::{CssValue, LengthUnit};

        let ctx = PagedMediaContext {
            page_width: 816.0,
            page_height: 1056.0,
            page_margins: EdgeSizes {
                top: 72.0,
                right: 72.0,
                bottom: 72.0,
                left: 72.0,
            },
            page_rules: vec![
                // More specific: :first:right sets margin-top to 50.
                PageRule {
                    selectors: vec![PageSelector::First, PageSelector::Right],
                    size: None,
                    margins: PageMargins::default(),
                    properties: vec![PropertyDeclaration::new(
                        "margin-top",
                        CssValue::Length(50.0, LengthUnit::Px),
                    )],
                },
                // Less specific: :first sets margin-top to 100.
                PageRule {
                    selectors: vec![PageSelector::First],
                    size: None,
                    margins: PageMargins::default(),
                    properties: vec![PropertyDeclaration::new(
                        "margin-top",
                        CssValue::Length(100.0, LengthUnit::Px),
                    )],
                },
            ],
        };

        // :first:right (2 selectors) is more specific, applied last → wins.
        let margins = ctx.effective_margins(1, false);
        assert_eq!(margins.top, 50.0);
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
