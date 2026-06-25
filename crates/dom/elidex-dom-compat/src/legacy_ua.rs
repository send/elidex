//! Compat-only UA stylesheet for **obsolete / non-conforming** HTML elements.
//!
//! Holds default rendering only for the elements HTML §16.2 marks
//! non-conforming (`<tt>`, `<strike>`, `<big>`, `<center>`). Standard conforming-element
//! rendering — `<b>`/`<strong>`/`<em>`/`<mark>`/form controls/etc. (HTML §15.3) —
//! is part of the modern UA baseline and lives in the core UA stylesheet
//! (`elidex-style` `ua_stylesheet`), applied in **every** engine mode. Only this
//! obsolete-element sheet is gated to compat mode (`EngineMode::BrowserCompat`).

use std::sync::OnceLock;

use elidex_css::{parse_stylesheet, Origin, Stylesheet};

/// CSS source for obsolete-element rendering (WHATWG HTML §16.2 non-conforming
/// features).
///
/// Only the non-conforming elements live here:
/// - `<tt>` — monospace; obsolete (use `<code>`/`<kbd>`/`<samp>` or CSS). Shares
///   the conforming monospace elements' rendering but is itself §16.2-obsolete,
///   so it is compat-gated rather than in the core `pre, code, kbd, samp` rule.
/// - `<strike>` (use `<s>`/`<del>`) — `<s>`/`<del>` are conforming and render in
///   the core UA sheet; only the obsolete `strike` alias is compat-gated.
/// - `<big>` — `larger` (relative) per the §15.3.4 sizing convention.
/// - `<center>` — block + centered text.
const LEGACY_UA_CSS: &str = r"
tt { font-family: monospace; }
strike { text-decoration-line: line-through; }
big { font-size: larger; }
center { display: block; text-align: center; }
";

/// Returns the legacy UA stylesheet (lazily initialized, cached).
#[must_use]
pub fn legacy_ua_stylesheet() -> &'static Stylesheet {
    static LEGACY_UA: OnceLock<Stylesheet> = OnceLock::new();
    LEGACY_UA.get_or_init(|| parse_stylesheet(LEGACY_UA_CSS, Origin::UserAgent))
}
