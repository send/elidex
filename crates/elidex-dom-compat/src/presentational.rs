//! HTML presentational attribute → CSS declaration conversion.
//!
//! Converts HTML attributes like `bgcolor`, `width`, `border`, `color`, `face`,
//! `size` into CSS declarations that participate in the cascade at author-origin,
//! specificity (0,0,0).
//!
//! # Phase 4 TODO
//!
//! - `valign` attribute → `vertical-align` (not in `ComputedStyle`)
//! - `background` attribute → `background-image: url()` (image pipeline)
//! - `<font size="+2">` relative sizes (needs parent reference in cascade)
//! - `cellpadding` nested table propagation

use elidex_css::Declaration;
use elidex_ecs::{Attributes, EcsDom, Entity, TagType};
use elidex_plugin::{CssColor, CssValue, LengthUnit};

/// CSS box sides, used for border and padding property generation.
const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

/// CSS absolute-size keywords for `<font size="N">` (WHATWG §15.3.1).
///
/// HTML-specific mapping (1–7 only); CSS absolute sizes are in `elidex-css`.
const FONT_SIZE_KEYWORDS: [&str; 7] = [
    "x-small",
    "small",
    "medium",
    "large",
    "x-large",
    "xx-large",
    "xxx-large",
];

/// Generate presentational hint declarations for an entity.
///
/// Reads the entity's tag name and attributes, and returns CSS declarations
/// corresponding to HTML presentational attributes. These declarations
/// participate in the cascade at author-origin, specificity (0,0,0), ordered
/// before all author stylesheet rules.
///
/// # Performance
///
/// `Vec::new()` does not heap-allocate (capacity = 0), so the early returns
/// for non-element or attribute-less entities are allocation-free.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn get_presentational_hints(entity: Entity, dom: &EcsDom) -> Vec<Declaration> {
    let Ok(tag_type) = dom.world().get::<&TagType>(entity) else {
        return Vec::new();
    };
    let tag = tag_type.0.as_str();

    let Ok(attrs) = dom.world().get::<&Attributes>(entity) else {
        return Vec::new();
    };

    let mut decls = Vec::new();

    // dir attribute → direction CSS property (WHATWG §15.3.6).
    // Applies to all HTML elements, not just specific tags.
    push_dir_attr(&attrs, &mut decls);

    // Fast path: skip tags that never have other presentational hints.
    if !matches!(
        tag,
        "img"
            | "table"
            | "td"
            | "th"
            | "hr"
            | "canvas"
            | "video"
            | "embed"
            | "object"
            | "iframe"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "body"
            | "div"
            | "p"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "caption"
            | "font"
    ) {
        return decls;
    }

    // width/height on replaced and table elements.
    // WHATWG: table/td/th width and td/th height use "ignoring zero" semantics.
    // hr only maps width (not height — hr uses the size attribute for height).
    match tag {
        "table" => {
            push_nonzero_dimension_attr(&attrs, "width", "width", &mut decls);
            push_dimension_attr(&attrs, "height", "height", &mut decls);
        }
        "td" | "th" => {
            push_nonzero_dimension_attr(&attrs, "width", "width", &mut decls);
            push_nonzero_dimension_attr(&attrs, "height", "height", &mut decls);
        }
        "hr" => {
            // WHATWG §15.3.11: only width is a presentational hint for <hr>.
            push_dimension_attr(&attrs, "width", "width", &mut decls);
        }
        "img" | "canvas" | "video" | "embed" | "object" | "iframe" => {
            push_dimension_attr(&attrs, "width", "width", &mut decls);
            push_dimension_attr(&attrs, "height", "height", &mut decls);
        }
        _ => {}
    }

    // bgcolor on table/body elements
    if matches!(
        tag,
        "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" | "body"
    ) {
        push_color_attr(&attrs, "bgcolor", "background-color", &mut decls);
    }

    // align on block elements → text-align (WHATWG §15.3.2)
    if matches!(
        tag,
        "div"
            | "p"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "caption"
            | "td"
            | "th"
            | "tr"
            | "thead"
            | "tbody"
            | "tfoot"
    ) {
        push_align_attr(&attrs, &mut decls);
    }

    // table[align] → float/margin centering (WHATWG §15.3.10, not text-align)
    if tag == "table" {
        push_table_align_attr(&attrs, &mut decls);
    }

    // border on table → border-*-width + border-*-style (WHATWG §15.3.10)
    if tag == "table" {
        push_table_border_attr(&attrs, &mut decls);
    }
    // border on img → only when border > 0 (WHATWG §15.4.3)
    if tag == "img" {
        push_img_border_attr(&attrs, &mut decls);
    }

    // cellspacing on table → border-spacing
    if tag == "table" {
        push_cellspacing_attr(&attrs, &mut decls);
    }

    // <font> attributes
    if tag == "font" {
        push_font_attrs(&attrs, &mut decls);
    }

    // cellpadding: td/th inherits padding from parent table's cellpadding attribute
    if matches!(tag, "td" | "th") {
        decls.extend(get_cellpadding_hints(entity, dom));
    }

    decls
}

/// Push a dimension attribute (width/height) as a CSS declaration.
fn push_dimension_attr(
    attrs: &Attributes,
    attr_name: &str,
    css_prop: &str,
    decls: &mut Vec<Declaration>,
) {
    if let Some(val) = attrs.get(attr_name) {
        if let Some(css_val) = parse_dimension_value(val) {
            decls.push(Declaration::new(css_prop, css_val));
        }
    }
}

/// Push a dimension attribute, ignoring zero values.
///
/// WHATWG specifies "maps to the dimension property (ignoring zero)" for
/// table/td/th width and td/th height — `width="0"` or `height="0"`
/// should produce no presentational hint.
fn push_nonzero_dimension_attr(
    attrs: &Attributes,
    attr_name: &str,
    css_prop: &str,
    decls: &mut Vec<Declaration>,
) {
    if let Some(val) = attrs.get(attr_name) {
        if let Some(css_val) = parse_dimension_value(val) {
            // Reject zero values.
            let is_zero = matches!(
                css_val,
                CssValue::Length(v, _) | CssValue::Percentage(v) if v == 0.0
            );
            if !is_zero {
                decls.push(Declaration::new(css_prop, css_val));
            }
        }
    }
}

/// Parse a non-negative dimension value: bare digits → px, ends with "%" → percentage.
///
/// Returns `None` for empty, unparseable, negative, or non-finite values
/// (NaN/Infinity). Per WHATWG spec, HTML `width`/`height` attributes use
/// "rules for parsing non-negative integers/dimension values".
#[must_use]
fn parse_dimension_value(value: &str) -> Option<CssValue> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(pct) = trimmed.strip_suffix('%') {
        if let Ok(n) = pct.trim().parse::<f32>() {
            if n.is_finite() && n >= 0.0 {
                return Some(CssValue::Percentage(n));
            }
        }
    }
    // Bare digits → px
    if let Ok(n) = trimmed.parse::<f32>() {
        if n.is_finite() && n >= 0.0 {
            return Some(CssValue::Length(n, LengthUnit::Px));
        }
    }
    None
}

/// Push a color attribute as a CSS color declaration.
fn push_color_attr(
    attrs: &Attributes,
    attr_name: &str,
    css_prop: &str,
    decls: &mut Vec<Declaration>,
) {
    if let Some(val) = attrs.get(attr_name) {
        if let Some(color) = parse_html_color(val) {
            decls.push(Declaration::new(css_prop, CssValue::Color(color)));
        }
    }
}

/// Parse an HTML color value (named colors + hex).
///
/// Handles `#RRGGBB`, `#RGB`, bare hex (`RRGGBB`/`RGB`), and 16 named colors.
/// This is a subset of the HTML spec's "rules for parsing a legacy colour value"
/// (§2.4.6) which also handles arbitrary-length hex, non-hex → 0, etc. The exotic
/// cases (e.g. `bgcolor="chucknorris"`) are extremely rare in practice.
/// Phase 4 TODO: full legacy color parsing algorithm if compat testing reveals gaps.
#[must_use]
fn parse_html_color(value: &str) -> Option<CssColor> {
    let trimmed = value.trim();
    // Named colors (most common subset) — case-insensitive without allocation.
    let named = [
        ("black", CssColor::BLACK),
        ("white", CssColor::WHITE),
        ("red", CssColor::RED),
        ("green", CssColor::new(0, 128, 0, 255)),
        ("blue", CssColor::BLUE),
        ("yellow", CssColor::new(255, 255, 0, 255)),
        ("aqua", CssColor::new(0, 255, 255, 255)),
        ("cyan", CssColor::new(0, 255, 255, 255)),
        ("fuchsia", CssColor::new(255, 0, 255, 255)),
        ("magenta", CssColor::new(255, 0, 255, 255)),
        ("silver", CssColor::new(192, 192, 192, 255)),
        ("gray", CssColor::new(128, 128, 128, 255)),
        ("grey", CssColor::new(128, 128, 128, 255)),
        ("maroon", CssColor::new(128, 0, 0, 255)),
        ("olive", CssColor::new(128, 128, 0, 255)),
        ("purple", CssColor::new(128, 0, 128, 255)),
        ("teal", CssColor::new(0, 128, 128, 255)),
        ("navy", CssColor::new(0, 0, 128, 255)),
        ("orange", CssColor::new(255, 165, 0, 255)),
    ];
    for (name, color) in &named {
        if trimmed.eq_ignore_ascii_case(name) {
            return Some(*color);
        }
    }

    // Hex color (#RGB or #RRGGBB)
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if !hex.is_ascii() {
        return None;
    }
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            Some(CssColor::new(r * 17, g * 17, b * 17, 255))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(CssColor::new(r, g, b, 255))
        }
        _ => None,
    }
}

/// Push dir attribute → direction + unicode-bidi CSS declarations (WHATWG §15.3.6).
///
/// `dir="ltr"` → `direction: ltr; unicode-bidi: isolate`,
/// `dir="rtl"` → `direction: rtl; unicode-bidi: isolate`.
fn push_dir_attr(attrs: &Attributes, decls: &mut Vec<Declaration>) {
    if let Some(val) = attrs.get("dir") {
        let dir = match val.trim().to_ascii_lowercase().as_str() {
            "ltr" => "ltr",
            "rtl" => "rtl",
            // TODO(Phase 4): `dir="auto"` requires the first-strong-character
            // algorithm (HTML §15.3.6) to determine direction from content.
            _ => return,
        };
        decls.push(Declaration::new(
            "direction",
            CssValue::Keyword(dir.to_string()),
        ));
        decls.push(Declaration::new(
            "unicode-bidi",
            CssValue::Keyword("isolate".to_string()),
        ));
    }
}

/// Push align attribute → text-align CSS declaration.
fn push_align_attr(attrs: &Attributes, decls: &mut Vec<Declaration>) {
    if let Some(val) = attrs.get("align") {
        let css_val = match val.trim().to_ascii_lowercase().as_str() {
            "left" => "left",
            "center" | "middle" => "center",
            "right" => "right",
            "justify" => "justify",
            _ => return,
        };
        decls.push(Declaration::new(
            "text-align",
            CssValue::Keyword(css_val.to_string()),
        ));
    }
}

/// Push table align attribute → float or margin auto centering (WHATWG §15.3.10).
///
/// Unlike block elements where align → text-align, on `<table>` the spec maps:
/// - `align="left"` → `float: left` (approximated with margin until float is supported)
/// - `align="right"` → `float: right` (approximated with margin until float is supported)
/// - `align="center"` → `margin-left: auto; margin-right: auto`
fn push_table_align_attr(attrs: &Attributes, decls: &mut Vec<Declaration>) {
    if let Some(val) = attrs.get("align") {
        match val.trim().to_ascii_lowercase().as_str() {
            "center" => {
                decls.push(Declaration::new("margin-left", CssValue::Auto));
                decls.push(Declaration::new("margin-right", CssValue::Auto));
            }
            // Phase 4 TODO: these should emit `float: left/right` (float not in ComputedStyle).
            // Margin approximation provides partial alignment behavior for now.
            "left" => {
                decls.push(Declaration::new(
                    "margin-left",
                    CssValue::Length(0.0, LengthUnit::Px),
                ));
                decls.push(Declaration::new("margin-right", CssValue::Auto));
            }
            "right" => {
                decls.push(Declaration::new("margin-left", CssValue::Auto));
                decls.push(Declaration::new(
                    "margin-right",
                    CssValue::Length(0.0, LengthUnit::Px),
                ));
            }
            _ => {}
        }
    }
}

/// Parse a non-negative finite `f32` from an HTML attribute value.
///
/// Returns `fallback` for empty, non-numeric, NaN, or Infinity values.
#[must_use]
fn parse_attr_f32(value: &str, fallback: f32) -> f32 {
    let parsed = value.trim().parse::<f32>().unwrap_or(fallback);
    if parsed.is_finite() {
        parsed.max(0.0)
    } else {
        fallback
    }
}

/// Push table border attribute → border-*-width + border-*-style.
///
/// Per WHATWG §15.3.10:
/// - Parse failure (empty string, non-numeric) → default 1px outset
/// - `border="0"` → border-*-width: 0 + border-*-style: none
/// - `border="N"` (N > 0) → border-*-width: Npx + border-*-style: outset
fn push_table_border_attr(attrs: &Attributes, decls: &mut Vec<Declaration>) {
    if let Some(val) = attrs.get("border") {
        let trimmed = val.trim();
        let width = if trimmed.is_empty() {
            1.0
        } else {
            parse_attr_f32(trimmed, 1.0)
        };
        // WHATWG §15.3.10: table border-style is outset (not solid).
        let style = if width > 0.0 { "outset" } else { "none" };
        for side in &SIDES {
            decls.push(Declaration::new(
                format!("border-{side}-width"),
                CssValue::Length(width, LengthUnit::Px),
            ));
            decls.push(Declaration::new(
                format!("border-{side}-style"),
                CssValue::Keyword(style.to_string()),
            ));
        }
    }
}

/// Push img border attribute → border-*-width + border-*-style.
///
/// Per WHATWG §15.4.3: only when the parsed border value is greater than zero.
/// `border="0"` and `border=""` produce no presentational hints for `<img>`.
fn push_img_border_attr(attrs: &Attributes, decls: &mut Vec<Declaration>) {
    if let Some(val) = attrs.get("border") {
        let trimmed = val.trim();
        if trimmed.is_empty() {
            return; // No hint for empty border attribute on img.
        }
        let width = parse_attr_f32(trimmed, 0.0);
        if width <= 0.0 {
            return; // No hint for border="0" on img.
        }
        for side in &SIDES {
            decls.push(Declaration::new(
                format!("border-{side}-width"),
                CssValue::Length(width, LengthUnit::Px),
            ));
            decls.push(Declaration::new(
                format!("border-{side}-style"),
                CssValue::Keyword("solid".to_string()),
            ));
        }
    }
}

/// Push cellspacing attribute → border-spacing-h/v.
fn push_cellspacing_attr(attrs: &Attributes, decls: &mut Vec<Declaration>) {
    if let Some(val) = attrs.get("cellspacing") {
        let spacing = parse_attr_f32(val, 0.0);
        decls.push(Declaration::new(
            "border-spacing-h",
            CssValue::Length(spacing, LengthUnit::Px),
        ));
        decls.push(Declaration::new(
            "border-spacing-v",
            CssValue::Length(spacing, LengthUnit::Px),
        ));
    }
}

/// Push `<font>` attributes: color, face, size.
fn push_font_attrs(attrs: &Attributes, decls: &mut Vec<Declaration>) {
    // color → CSS color
    push_color_attr(attrs, "color", "color", decls);

    // face → font-family
    if let Some(face) = attrs.get("face") {
        let families: Vec<CssValue> = face
            .split(',')
            .map(|f| CssValue::Keyword(f.trim().to_string()))
            .filter(|v| !matches!(v, CssValue::Keyword(k) if k.is_empty()))
            .collect();
        if !families.is_empty() {
            decls.push(Declaration::new("font-family", CssValue::List(families)));
        }
    }

    // size → font-size keyword (absolute only; relative "+N"/"-N" is Phase 4)
    if let Some(size_str) = attrs.get("size") {
        let trimmed = size_str.trim();
        // Phase 4 TODO: handle relative sizes (+N, -N)
        if !trimmed.starts_with('+') && !trimmed.starts_with('-') {
            if let Ok(n) = trimmed.parse::<usize>() {
                // WHATWG §15.3.1: clamp to [1, 7].
                let clamped = n.clamp(1, 7);
                let keyword = FONT_SIZE_KEYWORDS[clamped - 1];
                decls.push(Declaration::new(
                    "font-size",
                    CssValue::Keyword(keyword.to_string()),
                ));
            }
        }
    }
}

/// Check if a td/th's parent table has a cellpadding attribute,
/// and if so, generate padding declarations.
///
/// Called from `get_presentational_hints` for td/th elements.
fn get_cellpadding_hints(entity: Entity, dom: &EcsDom) -> Vec<Declaration> {
    // Walk up: td/th → tr → (thead/tbody/tfoot)? → table
    let mut ancestor = dom.get_parent(entity);
    for _ in 0..4 {
        let Some(parent) = ancestor else { break };
        let Ok(parent_tag) = dom.world().get::<&TagType>(parent) else {
            break;
        };
        if parent_tag.0.as_str() == "table" {
            let Ok(parent_attrs) = dom.world().get::<&Attributes>(parent) else {
                return Vec::new();
            };
            if let Some(val) = parent_attrs.get("cellpadding") {
                let padding = parse_attr_f32(val, 0.0);
                // Emit padding even for 0 — overrides UA default `td { padding: 1px }`.
                return SIDES
                    .iter()
                    .map(|side| {
                        Declaration::new(
                            format!("padding-{side}"),
                            CssValue::Length(padding, LengthUnit::Px),
                        )
                    })
                    .collect();
            }
            return Vec::new();
        }
        ancestor = dom.get_parent(parent);
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::CssColor;

    // --- parse_html_color safety ---

    #[test]
    fn parse_html_color_non_ascii_no_panic() {
        // 3-byte UTF-8 chars that make byte len 3 or 6 but are not ASCII hex.
        assert!(parse_html_color("あ").is_none()); // 3 bytes
        assert!(parse_html_color("ああ").is_none()); // 6 bytes
        assert!(parse_html_color("🔥").is_none()); // 4 bytes
    }

    #[test]
    fn parse_html_color_valid_hex() {
        assert_eq!(
            parse_html_color("#f00"),
            Some(CssColor::new(255, 0, 0, 255))
        );
        assert_eq!(
            parse_html_color("#ff0000"),
            Some(CssColor::new(255, 0, 0, 255))
        );
    }

    #[test]
    fn parse_html_color_named() {
        assert_eq!(parse_html_color("red"), Some(CssColor::RED));
        assert_eq!(parse_html_color("WHITE"), Some(CssColor::WHITE));
    }

    // --- parse_dimension_value NaN/Infinity ---

    #[test]
    fn parse_dimension_rejects_nan() {
        assert!(parse_dimension_value("NaN").is_none());
    }

    #[test]
    fn parse_dimension_rejects_infinity() {
        assert!(parse_dimension_value("Infinity").is_none());
        assert!(parse_dimension_value("-Infinity").is_none());
    }

    #[test]
    fn parse_dimension_valid_number() {
        assert_eq!(
            parse_dimension_value("100"),
            Some(CssValue::Length(100.0, LengthUnit::Px))
        );
        assert_eq!(
            parse_dimension_value("50%"),
            Some(CssValue::Percentage(50.0))
        );
    }

    // --- parse_attr_f32 ---

    #[test]
    fn parse_attr_f32_nan_returns_fallback() {
        assert_eq!(parse_attr_f32("NaN", 1.0), 1.0);
        assert_eq!(parse_attr_f32("NaN", 0.0), 0.0);
    }

    #[test]
    fn parse_attr_f32_infinity_returns_fallback() {
        assert_eq!(parse_attr_f32("Infinity", 1.0), 1.0);
        assert_eq!(parse_attr_f32("-Infinity", 0.0), 0.0);
    }

    #[test]
    fn parse_attr_f32_negative_clamped_to_zero() {
        assert_eq!(parse_attr_f32("-5", 0.0), 0.0);
    }

    #[test]
    fn parse_attr_f32_valid() {
        assert_eq!(parse_attr_f32("10", 0.0), 10.0);
        assert_eq!(parse_attr_f32("  5  ", 0.0), 5.0);
    }
}
