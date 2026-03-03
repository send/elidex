//! Legacy UA stylesheet for HTML tags not covered by the core UA stylesheet.
//!
//! These rules provide default rendering for legacy formatting elements
//! (`<b>`, `<i>`, `<u>`, `<center>`, `<mark>`, etc.) and form controls.

use std::sync::OnceLock;

use elidex_css::{parse_stylesheet, Origin, Stylesheet};

/// CSS source for legacy tag styling.
///
/// Note: `<tt>` is already covered by the core UA stylesheet's
/// `code, kbd, samp, tt { font-family: monospace; }` rule.
/// `<h1>`–`<h6>` font-weight:bold is covered by the core UA stylesheet
/// (no explicit rule needed since their inherited weight from `<body>` is 400,
/// but they render bold in real browsers — adding it here for correctness).
/// WHATWG HTML §15.3.1 rendering rules for phrasing content.
///
/// Key spec differences from naive implementations:
/// - `b`/`strong` use `bolder` (relative), not `bold` (absolute 700).
/// - `small`/`sub`/`sup` use `smaller` (relative), not a fixed px value.
/// - `big` uses `larger` (relative), not a fixed px value.
const LEGACY_UA_CSS: &str = r"
b, strong { font-weight: bolder; }
i, em, cite, var, dfn, address { font-style: italic; }
u, ins { text-decoration-line: underline; }
s, strike, del { text-decoration-line: line-through; }
small { font-size: smaller; }
big { font-size: larger; }
/* Phase 4 TODO: sub/sup need vertical-align: sub/super (property not yet in ComputedStyle) */
sub { font-size: smaller; }
sup { font-size: smaller; }
mark { background-color: yellow; color: black; }
center { display: block; text-align: center; }
input, textarea, select, button { display: inline-block; }
";

/// Returns the legacy UA stylesheet (lazily initialized, cached).
#[must_use]
pub fn legacy_ua_stylesheet() -> &'static Stylesheet {
    static LEGACY_UA: OnceLock<Stylesheet> = OnceLock::new();
    LEGACY_UA.get_or_init(|| parse_stylesheet(LEGACY_UA_CSS, Origin::UserAgent))
}
