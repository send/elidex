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
sub { font-size: smaller; vertical-align: sub; }
sup { font-size: smaller; vertical-align: super; }
mark { background-color: yellow; color: black; }
center { display: block; text-align: center; }
input, textarea, select, button { display: inline-block; }
input:not([type=checkbox]):not([type=radio]), textarea {
    border-top-width: 1px; border-right-width: 1px;
    border-bottom-width: 1px; border-left-width: 1px;
    border-top-style: solid; border-right-style: solid;
    border-bottom-style: solid; border-left-style: solid;
    border-top-color: #767676; border-right-color: #767676;
    border-bottom-color: #767676; border-left-color: #767676;
    padding-top: 1px; padding-right: 2px;
    padding-bottom: 1px; padding-left: 2px;
    font-size: 13px;
    background-color: #ffffff;
}
button, input[type=submit], input[type=button], input[type=reset] {
    border-top-width: 1px; border-right-width: 1px;
    border-bottom-width: 1px; border-left-width: 1px;
    border-top-style: solid; border-right-style: solid;
    border-bottom-style: solid; border-left-style: solid;
    border-top-color: #767676; border-right-color: #767676;
    border-bottom-color: #767676; border-left-color: #767676;
    padding-top: 1px; padding-right: 6px;
    padding-bottom: 1px; padding-left: 6px;
    font-size: 13px;
    background-color: #efefef;
    text-align: center;
}
input:disabled, textarea:disabled, button:disabled, select:disabled {
    opacity: 0.5;
}
input[type=checkbox], input[type=radio] {
    border-top-width: 1px; border-right-width: 1px;
    border-bottom-width: 1px; border-left-width: 1px;
    border-top-style: solid; border-right-style: solid;
    border-bottom-style: solid; border-left-style: solid;
    border-top-color: #767676; border-right-color: #767676;
    border-bottom-color: #767676; border-left-color: #767676;
}
select {
    border-top-width: 1px; border-right-width: 1px;
    border-bottom-width: 1px; border-left-width: 1px;
    border-top-style: solid; border-right-style: solid;
    border-bottom-style: solid; border-left-style: solid;
    border-top-color: #767676; border-right-color: #767676;
    border-bottom-color: #767676; border-left-color: #767676;
    padding-top: 1px; padding-right: 2px;
    padding-bottom: 1px; padding-left: 2px;
    font-size: 13px;
    background-color: #ffffff;
}
fieldset {
    display: block;
    border-top-width: 2px; border-right-width: 2px;
    border-bottom-width: 2px; border-left-width: 2px;
    border-top-style: solid; border-right-style: solid;
    border-bottom-style: solid; border-left-style: solid;
    border-top-color: #c0c0c0; border-right-color: #c0c0c0;
    border-bottom-color: #c0c0c0; border-left-color: #c0c0c0;
    padding-top: 6px; padding-right: 10px;
    padding-bottom: 6px; padding-left: 10px;
}
legend {
    display: block;
    padding-top: 0; padding-right: 2px;
    padding-bottom: 0; padding-left: 2px;
}
form { display: block; }
slot { display: contents; }
";

/// Returns the legacy UA stylesheet (lazily initialized, cached).
#[must_use]
pub fn legacy_ua_stylesheet() -> &'static Stylesheet {
    static LEGACY_UA: OnceLock<Stylesheet> = OnceLock::new();
    LEGACY_UA.get_or_init(|| parse_stylesheet(LEGACY_UA_CSS, Origin::UserAgent))
}
