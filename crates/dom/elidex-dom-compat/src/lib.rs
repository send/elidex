//! HTML/CSS compatibility layer for elidex.
//!
//! Provides normalization for legacy HTML and CSS patterns so the core engine
//! only needs to understand modern standards. This crate is linked by the
//! shell/integration layer, **not** by core crates (elidex-css, elidex-style,
//! elidex-html-parser).
//!
//! # Components
//!
//! - **Legacy UA stylesheet** ([`legacy_ua_stylesheet`]): rules for legacy tags
//!   like `<b>`, `<i>`, `<center>`, `<mark>`, form controls.
//! - **Presentational hints** ([`get_presentational_hints`]): converts HTML
//!   attributes (`bgcolor`, `width`, `border`, `color`, etc.) into CSS
//!   declarations that participate in the cascade.
//! - **Vendor prefix strip** ([`strip_vendor_prefixes`], [`parse_compat_stylesheet`]):
//!   removes `-webkit-`, `-moz-`, `-ms-`, `-o-` prefixes from CSS property names.
//! - **Legacy DOM API**: documents compat behavior for `document.all` and
//!   `document.write` (stubs in elidex-js).
//!
//! # Phase 4 deferred items
//!
//! - `font-style: oblique <angle>` (CSS Fonts Level 4, variable font support)
//! - `valign` attribute → `vertical-align` CSS property (not in `ComputedStyle`)
//! - `background` HTML attribute → `background-image: url()` (image pipeline)
//! - `document.write` full implementation (re-entrant parser)
//! - `document.all` as `HTMLAllCollection` (callable + typeof === "undefined")
//! - `document.images/forms/links` live collections
//! - `-webkit-text-size-adjust`, `-webkit-font-smoothing` (non-standard, dropped)
//! - `<font size="+2">` relative font sizes (requires parent reference in cascade)
//! - `cellpadding` nested table propagation (direct children only)
//! - `<marquee>`, `<blink>` (animation mechanism required)

mod legacy_dom;
mod legacy_ua;
mod presentational;
mod vendor_prefix;

#[cfg(test)]
mod tests;

pub use legacy_ua::legacy_ua_stylesheet;
pub use presentational::get_presentational_hints;
pub use vendor_prefix::{
    parse_compat_stylesheet, parse_compat_stylesheet_with_registry, strip_vendor_prefixes,
};
