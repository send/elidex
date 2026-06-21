//! classList DOM API handlers: add, remove, toggle, contains.
//!
//! Class names are split via `split_ascii_whitespace()` and re-joined with
//! single spaces.  This implicitly normalises the `class` attribute (tabs,
//! newlines, and consecutive ASCII spaces become a single space) while
//! treating non-ASCII whitespace (U+00A0 NBSP etc.) as ordinary token
//! content per WHATWG DOM §6 ("ASCII whitespace" tokenisation rule shared
//! across `classList` / `relList` / `linkSizes`).

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind, DomApiHandler, SessionCore};

use crate::util::{not_found_error, require_live_element, require_string_arg};

/// Validate a token per the `DOMTokenList` spec.
///
/// Returns `SyntaxError` for empty strings and `InvalidCharacterError` for
/// strings containing **ASCII whitespace** (U+0009, U+000A, U+000C,
/// U+000D, U+0020) — WHATWG DOM §6 defines DOMTokenList validation in
/// terms of ASCII whitespace, so non-ASCII whitespace such as U+00A0
/// (NBSP) is a valid token character and must NOT throw.  Used by every
/// DOMTokenList family (`classList` / `relList` / `linkSizes`), so the
/// messages are kept generic ("token must …") rather than naming any
/// specific attribute.
fn validate_token(token: &str) -> Result<(), DomApiError> {
    if token.is_empty() {
        return Err(DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: "token must not be empty".into(),
        });
    }
    if token.bytes().any(|b| b.is_ascii_whitespace()) {
        return Err(DomApiError {
            kind: DomApiErrorKind::InvalidCharacterError,
            message: "token must not contain ASCII whitespace".into(),
        });
    }
    Ok(())
}

/// Parse a class string as an ordered set: split on whitespace, deduplicate
/// while preserving first-occurrence order.
fn parse_ordered_set(class_str: &str) -> Vec<&str> {
    let mut seen = Vec::new();
    for token in class_str.split_ascii_whitespace() {
        if !seen.contains(&token) {
            seen.push(token);
        }
    }
    seen
}

/// Read the raw token string for a given attribute name (e.g. `"class"`,
/// `"rel"`, `"sizes"`).  Generalised in slot `#11-tags-T2a-url-bearing` so
/// the same DOMTokenList algorithms back `Element.classList`,
/// `HTMLAnchorElement.relList`, `HTMLLinkElement.sizes`, etc.
fn get_token_string(entity: Entity, dom: &EcsDom, attr_name: &str) -> Result<String, DomApiError> {
    let attrs = dom
        .world()
        .get::<&Attributes>(entity)
        .map_err(|_| not_found_error("element not found"))?;
    Ok(attrs.get(attr_name).unwrap_or("").to_string())
}

fn set_token_string(
    entity: Entity,
    dom: &mut EcsDom,
    attr_name: &str,
    value: &str,
) -> Result<(), DomApiError> {
    // Route through the canonical `EcsDom::set_attribute` chokepoint (not a
    // direct `Attributes` write) so a DOMTokenList write
    // (`classList`/`relList`/`linkSizes` add/remove/toggle/replace/`value=`)
    // bumps `rev_version` AND dispatches `MutationEvent::AttributeChange`
    // (DOM §4.9 → §4.3.2) — the prior direct path skipped the mutation event.
    // `require_live_element` preserves the "stale / non-Element receiver →
    // NotFoundError" contract for the `value=` op, which (unlike
    // add/remove/toggle/replace) does not pre-read via `get_token_string`.
    // The chokepoint owns `rev_version`, so callers drop their manual bump.
    require_live_element(dom, entity)?;
    dom.set_attribute(entity, attr_name, value);
    Ok(())
}

/// Whether the backing content attribute is currently *present* (vs absent).
/// Distinguishes the update-steps "get an attribute … returns null" case from a
/// present-but-empty attribute (`class=""`): the latter must still be written.
fn attribute_present(entity: Entity, dom: &EcsDom, attr_name: &str) -> bool {
    dom.world()
        .get::<&Attributes>(entity)
        .is_ok_and(|a| a.get(attr_name).is_some())
}

/// Run the DOMTokenList **update steps** (DOM §7.1 `#concept-dtl-update`) for a
/// method (`add`/`remove`/`toggle`/`replace`) that has just recomputed the
/// token set into `serialized`:
/// - **step 1** — if the backing attribute is absent AND `serialized` is empty,
///   return: a method that nets no change on an attribute-less element is a
///   no-op and must NOT create an empty attribute or dispatch `AttributeChange`
///   (e.g. `div.classList.remove("x")` on a `class`-less `<div>`).
/// - **step 2** — otherwise set the attribute value to `serialized`.
///
/// The `value=` setter is NOT an update-steps caller (DOM: its setter does an
/// unconditional "set an attribute value"), so it bypasses this gate and calls
/// [`set_token_string`] directly.
fn run_update_steps(
    entity: Entity,
    dom: &mut EcsDom,
    attr_name: &str,
    serialized: &str,
) -> Result<(), DomApiError> {
    if serialized.is_empty() && !attribute_present(entity, dom, attr_name) {
        return Ok(());
    }
    set_token_string(entity, dom, attr_name, serialized)
}

/// Collect every positional argument of a variadic DOMTokenList method
/// (`add`/`remove`) as a string, in order — coerced via the same
/// [`require_string_arg`] path each single-token op already uses.
fn collect_string_args(args: &[JsValue]) -> Result<Vec<String>, DomApiError> {
    (0..args.len())
        .map(|i| require_string_arg(args, i))
        .collect()
}

/// Parse the backing attribute into its **ordered token set** — the working
/// unit of every DOMTokenList mutator (DOM §7.1: ordered-set parser, split on
/// ASCII whitespace + dedup preserving order). Mutators operate on this `Vec`
/// then serialize it back via [`run_update_steps`], so serialization +
/// deduplication + the update-steps gate are correct by construction for every
/// method (`add`/`remove`/`toggle`/`replace`) — no method manipulates the raw
/// attribute string.
fn token_set(entity: Entity, dom: &EcsDom, attr_name: &str) -> Result<Vec<String>, DomApiError> {
    let current = get_token_string(entity, dom, attr_name)?;
    Ok(parse_ordered_set(&current)
        .into_iter()
        .map(str::to_string)
        .collect())
}

/// Serialize an ordered token set (DOM §7.1 ordered-set serializer — join with
/// U+0020). The single canonical form every mutator writes through the update
/// steps; the `value=` setter is the only path that writes a raw value.
fn serialize_token_set(set: &[String]) -> String {
    set.join(" ")
}

/// `add(tokens…)` — append each token to the ordered set if absent, then run
/// the update steps **once** (DOM §7.1). Variadic: one `AttributeChange` for the
/// whole call (per-token routing would fire one per token); the serialized set
/// is written, never the raw attribute string. `toggle` reuses this with a
/// single-element slice.
fn add_tokens(
    entity: Entity,
    attr_name: &str,
    tokens: &[String],
    dom: &mut EcsDom,
) -> Result<(), DomApiError> {
    let mut set = token_set(entity, dom, attr_name)?;
    for token in tokens {
        if !set.iter().any(|t| t == token) {
            set.push(token.clone());
        }
    }
    run_update_steps(entity, dom, attr_name, &serialize_token_set(&set))
}

/// `remove(tokens…)` — drop every token in `tokens` from the ordered set, then
/// run the update steps **once** (DOM §7.1). See [`add_tokens`]. `toggle` reuses
/// this with a single-element slice.
fn remove_tokens(
    entity: Entity,
    attr_name: &str,
    tokens: &[String],
    dom: &mut EcsDom,
) -> Result<(), DomApiError> {
    let mut set = token_set(entity, dom, attr_name)?;
    set.retain(|t| !tokens.iter().any(|x| x == t));
    run_update_steps(entity, dom, attr_name, &serialize_token_set(&set))
}

// ===========================================================================
// `TokenListHandler` factory — single struct backs all DOMTokenList
// operations across `Element.classList` / `<a,area,link>.relList` /
// `<link>.sizes`.  Parameterised by `(method_name, attr_name, op)`,
// dispatched in `invoke` via a `match self.op` (collapses the
// pre-/simplify 27 unit-struct copy-paste of slot
// `#11-tags-T2a-url-bearing` to one type).
// ===========================================================================

/// DOMTokenList operation discriminator — selects which method
/// (add / remove / toggle / contains / replace / value.{get,set} /
/// length / item / supports) the handler executes against the
/// underlying token-set algorithms.  Shared with VM-side
/// `dispatch_method` so a single op enum drives both routing and
/// handler dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenListOp {
    Add,
    Remove,
    Toggle,
    Contains,
    Replace,
    ValueGet,
    ValueSet,
    Length,
    Item,
    Supports,
}

/// Generic DOMTokenList handler.  One instance per (method_name,
/// attr_name, op) triple; the registry registers 28 instances spanning
/// classList / relList / linkSizes families.
pub struct TokenListHandler {
    /// Method name surfaced to `DomApiHandler::method_name` (matches
    /// the registry key, e.g. `"classList.add"` / `"relList.add"`).
    pub method_name: &'static str,
    /// HTML content attribute name backing the wrapper
    /// (`"class"` / `"rel"` / `"sizes"`).
    pub attr_name: &'static str,
    /// Which operation to perform.
    pub op: TokenListOp,
}

/// `<link>` `rel` supported tokens — HTML §4.2.4 defines these as the possible
/// keywords **intersected with the keywords whose processing model the user
/// agent implements**, so `supports()` is an honest feature-detection signal
/// rather than a keyword-recognition one. elidex implements:
/// - `stylesheet` — the external CSS load + cascade pipeline (`LinkStylesheet`),
/// - `manifest` — Web App Manifest discovery + resolution
///   (`elidex-navigation` loader / `elidex-api-sw`).
///
/// The remaining possible `<link>` keywords (alternate, dns-prefetch, expect,
/// icon, modulepreload, next, pingback, preconnect, prefetch, preload, search)
/// have **no** processing model here — advertising them would make
/// `link.relList.supports(…)` lie. Add a keyword here when its processing model
/// lands (and not before).
const LINK_REL_IMPLEMENTED: &[&str] = &["manifest", "stylesheet"];

/// `<a>` / `<area>` `rel` supported tokens — the possible hyperlink keywords are
/// `noopener` / `noreferrer` / `opener` (HTML §4.6.2 `#attr-hyperlink-rel`), but
/// elidex implements **none** of their processing models: `target=_blank` never
/// consults `rel`, and `window.opener` is inert. The implemented subset is
/// therefore empty — `relList.supports(…)` returns `false` (not a throw, since
/// hyperlink `rel` *does* define supported tokens; the UA-implemented subset is
/// just empty). Grows as the processing models land.
const HYPERLINK_REL_IMPLEMENTED: &[&str] = &[];

/// Resolve the DOM §7.1 *supported tokens* set for a DOMTokenList — the
/// UA-implemented subset (see [`LINK_REL_IMPLEMENTED`] /
/// [`HYPERLINK_REL_IMPLEMENTED`]) — or `None` when the backing attribute defines
/// no supported tokens at all (then `supports()` throws). Only `rel` defines
/// supported tokens, and the set depends on the owning element. `class` /
/// `sizes` define none.
///
/// `<form>` `rel` also defines supported tokens (HTML §4.10.3), but
/// `HTMLFormElement.relList` is not yet wired (no `relList` accessor), so the
/// branch is unreachable from web content and intentionally omitted; add
/// `form => Some(HYPERLINK_REL_IMPLEMENTED)` here when that surface is wired.
fn rel_supported_tokens(
    attr_name: &str,
    entity: Entity,
    dom: &EcsDom,
) -> Option<&'static [&'static str]> {
    if attr_name != "rel" {
        return None;
    }
    dom.with_tag_name(entity, |tag| match tag {
        Some(t) if t.eq_ignore_ascii_case("link") => Some(LINK_REL_IMPLEMENTED),
        Some(t) if t.eq_ignore_ascii_case("a") || t.eq_ignore_ascii_case("area") => {
            Some(HYPERLINK_REL_IMPLEMENTED)
        }
        _ => None,
    })
}

impl DomApiHandler for TokenListHandler {
    fn method_name(&self) -> &str {
        self.method_name
    }

    #[allow(clippy::too_many_lines)]
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        match self.op {
            TokenListOp::Add => {
                // §7.1 `add(tokens…)` is variadic: validate ALL tokens first,
                // then append all + run the update steps once (one
                // `AttributeChange` for the whole call, not one per token).
                let tokens = collect_string_args(args)?;
                for token in &tokens {
                    validate_token(token)?;
                }
                add_tokens(this, self.attr_name, &tokens, dom)?;
                Ok(JsValue::Undefined)
            }
            TokenListOp::Remove => {
                let tokens = collect_string_args(args)?;
                for token in &tokens {
                    validate_token(token)?;
                }
                remove_tokens(this, self.attr_name, &tokens, dom)?;
                Ok(JsValue::Undefined)
            }
            TokenListOp::Toggle => {
                let token = require_string_arg(args, 0)?;
                validate_token(&token)?;
                let force = match args.get(1) {
                    Some(JsValue::Bool(b)) => Some(*b),
                    _ => None,
                };
                let current = get_token_string(this, dom, self.attr_name)?;
                let has = current.split_ascii_whitespace().any(|c| c == token);
                // `toggle` is single-token; reuse the variadic helpers with a
                // one-element slice so the update steps still run once.
                let one = std::slice::from_ref(&token);
                let result = match force {
                    Some(true) => {
                        if !has {
                            add_tokens(this, self.attr_name, one, dom)?;
                        }
                        true
                    }
                    Some(false) => {
                        if has {
                            remove_tokens(this, self.attr_name, one, dom)?;
                        }
                        false
                    }
                    None => {
                        if has {
                            remove_tokens(this, self.attr_name, one, dom)?;
                            false
                        } else {
                            add_tokens(this, self.attr_name, one, dom)?;
                            true
                        }
                    }
                };
                Ok(JsValue::Bool(result))
            }
            TokenListOp::Contains => {
                let token = require_string_arg(args, 0)?;
                let current = get_token_string(this, dom, self.attr_name)?;
                let has = current.split_ascii_whitespace().any(|c| c == token);
                Ok(JsValue::Bool(has))
            }
            TokenListOp::Replace => {
                let old_token = require_string_arg(args, 0)?;
                let new_token = require_string_arg(args, 1)?;
                validate_token(&old_token)?;
                validate_token(&new_token)?;
                let set = token_set(this, dom, self.attr_name)?;
                if !set.iter().any(|t| t == &old_token) {
                    return Ok(JsValue::Bool(false));
                }
                // Infra "replace within an ordered set" (Infra §5.1.3, DOM §7.1
                // replace step 4): put `new` at the **first instance of *either*
                // `old` or `new`** and drop all other instances of both — so
                // `replace("a","c")` on both « a b c » and « c b a » yields
                // « c b ». (Replacing only at `old`'s position diverges when
                // `new` precedes `old`.)
                let mut replaced = false;
                let mut result: Vec<String> = Vec::with_capacity(set.len());
                for t in set {
                    if t == old_token || t == new_token {
                        if !replaced {
                            result.push(new_token.clone());
                            replaced = true;
                        }
                    } else {
                        result.push(t);
                    }
                }
                run_update_steps(this, dom, self.attr_name, &serialize_token_set(&result))?;
                Ok(JsValue::Bool(true))
            }
            TokenListOp::ValueGet => {
                let s = get_token_string(this, dom, self.attr_name)?;
                Ok(JsValue::String(s))
            }
            TokenListOp::ValueSet => {
                let value = require_string_arg(args, 0)?;
                set_token_string(this, dom, self.attr_name, &value)?;
                Ok(JsValue::Undefined)
            }
            TokenListOp::Length => {
                let s = get_token_string(this, dom, self.attr_name)?;
                let count = parse_ordered_set(&s).len();
                #[allow(clippy::cast_precision_loss)]
                Ok(JsValue::Number(count as f64))
            }
            TokenListOp::Item => {
                let index = match args.first() {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    Some(JsValue::Number(n)) => *n as usize,
                    _ => {
                        return Err(DomApiError {
                            kind: DomApiErrorKind::TypeError,
                            message: format!("{}: argument 0 must be a number", self.method_name),
                        });
                    }
                };
                let s = get_token_string(this, dom, self.attr_name)?;
                match parse_ordered_set(&s).get(index) {
                    Some(token) => Ok(JsValue::String((*token).to_string())),
                    None => Ok(JsValue::Null),
                }
            }
            TokenListOp::Supports => {
                // DOM §7.1 `supports(token)` = the attribute's *validation
                // steps*: if the backing attribute defines supported tokens,
                // return whether `token` is one of them (ASCII case-insensitive,
                // no token-syntax validation); otherwise throw a `TypeError`.
                let token = require_string_arg(args, 0)?;
                match rel_supported_tokens(self.attr_name, this, dom) {
                    Some(set) => Ok(JsValue::Bool(
                        set.iter().any(|t| t.eq_ignore_ascii_case(&token)),
                    )),
                    None => Err(DomApiError {
                        kind: DomApiErrorKind::TypeError,
                        message: format!(
                            "{} is not supported for this DOMTokenList",
                            self.method_name
                        ),
                    }),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Static handler instances — one per (family, op) pair.  The registry
// references these directly and tests use them in place of the
// pre-refactor unit struct types.  Each is a `pub const` so callers
// can `INSTANCE.invoke(...)` without any Box / allocation.
// ---------------------------------------------------------------------------

macro_rules! handler {
    ($name:ident, $method:literal, $attr:literal, $op:ident) => {
        pub const $name: TokenListHandler = TokenListHandler {
            method_name: $method,
            attr_name: $attr,
            op: TokenListOp::$op,
        };
    };
}

handler!(CLASS_LIST_ADD, "classList.add", "class", Add);
handler!(CLASS_LIST_REMOVE, "classList.remove", "class", Remove);
handler!(CLASS_LIST_TOGGLE, "classList.toggle", "class", Toggle);
handler!(CLASS_LIST_CONTAINS, "classList.contains", "class", Contains);
handler!(CLASS_LIST_REPLACE, "classList.replace", "class", Replace);
handler!(
    CLASS_LIST_VALUE_GET,
    "classList.value.get",
    "class",
    ValueGet
);
handler!(
    CLASS_LIST_VALUE_SET,
    "classList.value.set",
    "class",
    ValueSet
);
handler!(CLASS_LIST_LENGTH, "classList.length", "class", Length);
handler!(CLASS_LIST_ITEM, "classList.item", "class", Item);
handler!(CLASS_LIST_SUPPORTS, "classList.supports", "class", Supports);

handler!(REL_LIST_ADD, "relList.add", "rel", Add);
handler!(REL_LIST_REMOVE, "relList.remove", "rel", Remove);
handler!(REL_LIST_TOGGLE, "relList.toggle", "rel", Toggle);
handler!(REL_LIST_CONTAINS, "relList.contains", "rel", Contains);
handler!(REL_LIST_REPLACE, "relList.replace", "rel", Replace);
handler!(REL_LIST_VALUE_GET, "relList.value.get", "rel", ValueGet);
handler!(REL_LIST_VALUE_SET, "relList.value.set", "rel", ValueSet);
handler!(REL_LIST_LENGTH, "relList.length", "rel", Length);
handler!(REL_LIST_ITEM, "relList.item", "rel", Item);
handler!(REL_LIST_SUPPORTS, "relList.supports", "rel", Supports);

handler!(LINK_SIZES_ADD, "linkSizes.add", "sizes", Add);
handler!(LINK_SIZES_REMOVE, "linkSizes.remove", "sizes", Remove);
handler!(LINK_SIZES_TOGGLE, "linkSizes.toggle", "sizes", Toggle);
handler!(LINK_SIZES_CONTAINS, "linkSizes.contains", "sizes", Contains);
handler!(LINK_SIZES_REPLACE, "linkSizes.replace", "sizes", Replace);
handler!(
    LINK_SIZES_VALUE_GET,
    "linkSizes.value.get",
    "sizes",
    ValueGet
);
handler!(
    LINK_SIZES_VALUE_SET,
    "linkSizes.value.set",
    "sizes",
    ValueSet
);
handler!(LINK_SIZES_LENGTH, "linkSizes.length", "sizes", Length);
handler!(LINK_SIZES_ITEM, "linkSizes.item", "sizes", Item);
handler!(LINK_SIZES_SUPPORTS, "linkSizes.supports", "sizes", Supports);

handler!(OUTPUT_HTML_FOR_ADD, "outputHtmlFor.add", "for", Add);
handler!(
    OUTPUT_HTML_FOR_REMOVE,
    "outputHtmlFor.remove",
    "for",
    Remove
);
handler!(
    OUTPUT_HTML_FOR_TOGGLE,
    "outputHtmlFor.toggle",
    "for",
    Toggle
);
handler!(
    OUTPUT_HTML_FOR_CONTAINS,
    "outputHtmlFor.contains",
    "for",
    Contains
);
handler!(
    OUTPUT_HTML_FOR_REPLACE,
    "outputHtmlFor.replace",
    "for",
    Replace
);
handler!(
    OUTPUT_HTML_FOR_VALUE_GET,
    "outputHtmlFor.value.get",
    "for",
    ValueGet
);
handler!(
    OUTPUT_HTML_FOR_VALUE_SET,
    "outputHtmlFor.value.set",
    "for",
    ValueSet
);
handler!(
    OUTPUT_HTML_FOR_LENGTH,
    "outputHtmlFor.length",
    "for",
    Length
);
handler!(OUTPUT_HTML_FOR_ITEM, "outputHtmlFor.item", "for", Item);
handler!(
    OUTPUT_HTML_FOR_SUPPORTS,
    "outputHtmlFor.supports",
    "for",
    Supports
);
#[cfg(test)]
#[path = "class_list_tests.rs"]
mod tests;
