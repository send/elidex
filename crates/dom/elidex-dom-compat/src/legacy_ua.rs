//! Compat-only UA stylesheet for **obsolete / non-conforming** HTML elements.
//!
//! Holds default rendering only for the elements HTML §16.2 marks
//! non-conforming (`<strike>`, `<big>`, `<center>`). (`<tt>` is §16.2-obsolete too
//! but stays in the *core* UA sheet so its monospace reaches shadow trees — the
//! compat sheet is not applied inside shadow roots; see `elidex-style` `ua.rs`.)
//! Standard conforming-element
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
/// - `<strike>` (use `<s>`/`<del>`) — `<s>`/`<del>` are conforming and render in
///   the core UA sheet; only the obsolete `strike` alias is compat-gated.
/// - `<big>` — `larger` (relative) per the §15.3.4 sizing convention.
/// - `<center>` — block + centered text.
///
/// (`<tt>` is also §16.2-obsolete but is intentionally kept in the core sheet —
/// see the module doc — so it is NOT listed here.)
const LEGACY_UA_CSS: &str = r"
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
