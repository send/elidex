//! CSS Paged Media Level 3 page rules.
mod body;
mod selectors;
mod size;

pub(crate) use body::parse_body;
pub use body::PageRule;
use elidex_plugin::{ContentItem, ContentValue, CssValue, ListStyleType};
pub(crate) use selectors::alternatives;
pub use selectors::PageAlternative;
pub use size::parse_page_size;

/// Margin box at-rule names (CSS Paged Media L3 §4.2).
pub const MARGIN_BOX_NAMES: &[&str] = &[
    "top-left-corner",
    "top-left",
    "top-center",
    "top-right",
    "top-right-corner",
    "right-top",
    "right-middle",
    "right-bottom",
    "bottom-right-corner",
    "bottom-right",
    "bottom-center",
    "bottom-left",
    "bottom-left-corner",
    "left-bottom",
    "left-middle",
    "left-top",
];

/// Convert a `CssValue` to a `ContentValue`.
pub fn content_value_from_css_value(value: &CssValue) -> ContentValue {
    match value {
        CssValue::Keyword(k) if k == "normal" => ContentValue::Normal,
        CssValue::Keyword(k) if k == "none" => ContentValue::None,
        CssValue::String(s) => ContentValue::Items(vec![ContentItem::String(s.clone())]),
        CssValue::Keyword(k) => {
            // The declaration parser encodes counter()/counters() as keyword strings:
            //   "counter:name:style" or "counters:name:separator:style"
            if let Some(item) = parse_counter_keyword(k) {
                ContentValue::Items(vec![item])
            } else {
                ContentValue::Normal
            }
        }
        CssValue::List(items) => {
            let content_items: Vec<ContentItem> =
                items.iter().filter_map(css_value_to_content_item).collect();
            if content_items.is_empty() {
                ContentValue::Normal
            } else {
                ContentValue::Items(content_items)
            }
        }
        _ => ContentValue::Normal,
    }
}

/// Convert a single `CssValue` to a `ContentItem`.
fn css_value_to_content_item(value: &CssValue) -> Option<ContentItem> {
    match value {
        CssValue::String(s) => Some(ContentItem::String(s.clone())),
        CssValue::Keyword(k) => parse_counter_keyword(k),
        _ => None,
    }
}

/// Parse an encoded counter keyword string into a `ContentItem`.
///
/// The declaration parser encodes `counter(name, style)` as `"counter:name:style"`
/// and `counters(name, sep, style)` as `"counters:name:sep:style"`.
fn parse_counter_keyword(k: &str) -> Option<ContentItem> {
    if let Some(rest) = k.strip_prefix("counter:") {
        // "counter:name:style"
        let mut parts = rest.splitn(2, ':');
        let name = parts.next()?.to_string();
        let style_str = parts.next().unwrap_or("decimal");
        let style = ListStyleType::from_keyword(style_str).unwrap_or_default();
        Some(ContentItem::Counter { name, style })
    } else if let Some(rest) = k.strip_prefix("counters:") {
        // "counters:name:separator:style"
        let mut parts = rest.splitn(3, ':');
        let name = parts.next()?.to_string();
        let separator = parts.next()?.to_string();
        let style_str = parts.next().unwrap_or("decimal");
        let style = ListStyleType::from_keyword(style_str).unwrap_or_default();
        Some(ContentItem::Counters {
            name,
            separator,
            style,
        })
    } else {
        None
    }
}
