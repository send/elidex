//! `CSS` namespace static methods (CSSOM §6.7) — `CSS.escape` and a 2-argument
//! form of `CSS.supports`.
//!
//! Both handlers ignore `this` (CSS namespace methods are pure functions on a
//! namespace object).  The dispatch contract uses a sentinel root entity for
//! `this` since [`elidex_script_session::DomApiHandler::invoke`] requires
//! one; the body never reads it.
//!
//! ## `CSS.supports(condition)` — 1-arg form deferred
//!
//! The single-argument `supports(<supports-condition>)` form requires a
//! recursive-descent parser over the CSS supports grammar
//! (`<supports-condition>` → `not <s-cond>` / `<s-cond> and <s-cond>` /
//! `<s-cond> or <s-cond>` / `<supports-feature>`).  Out of scope for PR-A
//! (deferred to slot `#11-css-supports-condition`).  PR-A returns `false`
//! for any 1-arg call so framework feature-detect calls fail closed rather
//! than throwing.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiHandler, SessionCore};

use crate::util::require_string_arg;

// ---------------------------------------------------------------------------
// CSS.escape(ident)
// ---------------------------------------------------------------------------

/// `CSS.escape(ident)` — CSSOM §6.7.2 "serialize an identifier" wrapper.
/// Pure string transformation; ignores `this` and `dom`.
pub struct CssEscape;

impl DomApiHandler for CssEscape {
    fn method_name(&self) -> &str {
        "CSS.escape"
    }

    fn invoke(
        &self,
        _this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let ident = require_string_arg(args, 0)?;
        Ok(JsValue::String(elidex_css::escape_ident(&ident)))
    }
}

// ---------------------------------------------------------------------------
// CSS.supports(property, value)
// ---------------------------------------------------------------------------

/// `CSS.supports(property, value)` — 2-argument feature query.  Returns
/// `true` when the engine recognises the `(property, value)` pair (the
/// declaration parser produces a non-empty result), `false` otherwise.
///
/// 1-argument `<supports-condition>` form is deferred (returns `false`
/// pending the dedicated parser).  Detected via `args.len() < 2`.
pub struct CssSupports;

impl DomApiHandler for CssSupports {
    fn method_name(&self) -> &str {
        "CSS.supports"
    }

    fn invoke(
        &self,
        _this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        // WebIDL: `CSS.supports()` with zero arguments throws TypeError
        // (the first parameter is required in both the 1-arg
        // `<supports-condition>` and 2-arg `(property, value)` overloads).
        // Match the VM-side `native_css_supports` shape so the standalone
        // handler stays consistent with the engine binding.
        if args.is_empty() {
            return Err(elidex_script_session::DomApiError {
                kind: elidex_script_session::DomApiErrorKind::TypeError,
                message:
                    "Failed to execute 'supports' on 'CSS': 1 argument required, but 0 present."
                        .into(),
            });
        }
        if args.len() < 2 {
            // 1-arg <supports-condition> form deferred — see module docs.
            return Ok(JsValue::Bool(false));
        }
        let property = require_string_arg(args, 0)?;
        let value = require_string_arg(args, 1)?;
        let css = format!("{property}: {value};");
        let supported = !elidex_css::parse_declaration_block(&css).is_empty();
        Ok(JsValue::Bool(supported))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn setup() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let session = SessionCore::new();
        (dom, elem, session)
    }

    #[test]
    fn escape_basic_ident() {
        let (mut dom, this, mut session) = setup();
        let result = CssEscape
            .invoke(
                this,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("foo".into()));
    }

    #[test]
    fn escape_special_chars() {
        let (mut dom, this, mut session) = setup();
        let result = CssEscape
            .invoke(
                this,
                &[JsValue::String("foo bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("foo\\ bar".into()));
    }

    #[test]
    fn supports_known_property() {
        let (mut dom, this, mut session) = setup();
        let result = CssSupports
            .invoke(
                this,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    #[test]
    fn supports_one_arg_returns_false() {
        let (mut dom, this, mut session) = setup();
        let result = CssSupports
            .invoke(
                this,
                &[JsValue::String("(display: flex)".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    #[test]
    fn supports_unknown_property() {
        let (mut dom, this, mut session) = setup();
        // `parse_declaration_block` returns empty Vec for unknown
        // properties (see `parse_property_value` contract), so
        // `supports` returns `false`.
        let result = CssSupports
            .invoke(
                this,
                &[
                    JsValue::String("definitely-not-a-real-css-property-xyz".into()),
                    JsValue::String("anything".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }
}
