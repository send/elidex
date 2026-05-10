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

use crate::util::{not_found_error, require_string_arg};

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

/// Normalize whitespace in a class string: collapse multiple spaces, trim.
fn normalize_class_string(s: &str) -> String {
    s.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
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
    value: String,
) -> Result<(), DomApiError> {
    let mut attrs = dom
        .world_mut()
        .get::<&mut Attributes>(entity)
        .map_err(|_| not_found_error("element not found"))?;
    attrs.set(attr_name, value);
    Ok(())
}

/// Add a token to the attribute's whitespace-separated list if not already present.
fn add_token(
    entity: Entity,
    attr_name: &str,
    token: &str,
    dom: &mut EcsDom,
) -> Result<(), DomApiError> {
    let current = get_token_string(entity, dom, attr_name)?;
    if !current.split_ascii_whitespace().any(|c| c == token) {
        let normalized = normalize_class_string(&current);
        let new_value = if normalized.is_empty() {
            token.to_string()
        } else {
            format!("{normalized} {token}")
        };
        set_token_string(entity, dom, attr_name, new_value)?;
    }
    Ok(())
}

/// Remove a token from the attribute's whitespace-separated list.
fn remove_token(
    entity: Entity,
    attr_name: &str,
    token: &str,
    dom: &mut EcsDom,
) -> Result<(), DomApiError> {
    let current = get_token_string(entity, dom, attr_name)?;
    let new_tokens: Vec<&str> = current
        .split_ascii_whitespace()
        .filter(|c| *c != token)
        .collect();
    set_token_string(entity, dom, attr_name, new_tokens.join(" "))?;
    Ok(())
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
                let token = require_string_arg(args, 0)?;
                validate_token(&token)?;
                add_token(this, self.attr_name, &token, dom)?;
                dom.rev_version(this);
                Ok(JsValue::Undefined)
            }
            TokenListOp::Remove => {
                let token = require_string_arg(args, 0)?;
                validate_token(&token)?;
                remove_token(this, self.attr_name, &token, dom)?;
                dom.rev_version(this);
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
                let result = match force {
                    Some(true) => {
                        if !has {
                            add_token(this, self.attr_name, &token, dom)?;
                        }
                        true
                    }
                    Some(false) => {
                        if has {
                            remove_token(this, self.attr_name, &token, dom)?;
                        }
                        false
                    }
                    None => {
                        if has {
                            remove_token(this, self.attr_name, &token, dom)?;
                            false
                        } else {
                            add_token(this, self.attr_name, &token, dom)?;
                            true
                        }
                    }
                };
                dom.rev_version(this);
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
                let current = get_token_string(this, dom, self.attr_name)?;
                let tokens: Vec<&str> = current.split_ascii_whitespace().collect();
                if !tokens.contains(&old_token.as_str()) {
                    return Ok(JsValue::Bool(false));
                }
                // Infra ordered set "replace": replace first occurrence
                // of `old_token` with `new_token`, then drop subsequent
                // occurrences of `new_token` (dedup).
                let mut replaced = false;
                let mut result: Vec<&str> = Vec::with_capacity(tokens.len());
                for t in &tokens {
                    if !replaced && *t == old_token.as_str() {
                        result.push(new_token.as_str());
                        replaced = true;
                    } else if *t != new_token.as_str() {
                        result.push(t);
                    }
                }
                set_token_string(this, dom, self.attr_name, result.join(" "))?;
                dom.rev_version(this);
                Ok(JsValue::Bool(true))
            }
            TokenListOp::ValueGet => {
                let s = get_token_string(this, dom, self.attr_name)?;
                Ok(JsValue::String(s))
            }
            TokenListOp::ValueSet => {
                let value = require_string_arg(args, 0)?;
                set_token_string(this, dom, self.attr_name, value)?;
                dom.rev_version(this);
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
            TokenListOp::Supports => Err(DomApiError {
                kind: DomApiErrorKind::TypeError,
                message: format!(
                    "{} is not supported for this DOMTokenList",
                    self.method_name
                ),
            }),
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

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;

    fn setup() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "foo bar");
        let elem = dom.create_element("div", attrs);
        let session = SessionCore::new();
        (dom, elem, session)
    }

    #[test]
    fn validate_token_rejects_ascii_whitespace() {
        // Spec scope: each of the 5 ASCII whitespace bytes must error.
        for ws in ["\t", "\n", "\x0c", "\r", " "] {
            let token = format!("foo{ws}bar");
            let err = validate_token(&token).unwrap_err();
            assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
        }
    }

    #[test]
    fn validate_token_accepts_non_ascii_whitespace() {
        // PR178 R4 IMP regression — `char::is_whitespace` previously
        // rejected non-ASCII whitespace such as U+00A0 (NBSP), which
        // the spec considers a valid token character.
        for ch in ["\u{00A0}", "\u{2003}", "\u{3000}"] {
            let token = format!("foo{ch}bar");
            assert!(
                validate_token(&token).is_ok(),
                "token containing {ch:?} should be accepted (non-ASCII whitespace)"
            );
        }
    }

    #[test]
    fn add_then_contains_token_with_nbsp() {
        // PR178 R5 IMP regression — every tokenisation site (parse_ordered_set,
        // normalize_class_string, add_token, remove_token, Toggle / Contains /
        // Replace / Length / Item) was using `split_whitespace` (Unicode-aware),
        // which would break `contains`/`add` for tokens containing NBSP
        // (U+00A0) and other non-ASCII whitespace.  Switched to
        // `split_ascii_whitespace` so the membership check matches the
        // ASCII-whitespace parser used at insertion time.
        let (mut dom, elem, mut session) = setup();
        let nbsp_token = "foo\u{00A0}bar";
        CLASS_LIST_ADD
            .invoke(
                elem,
                &[JsValue::String(nbsp_token.into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = CLASS_LIST_CONTAINS
            .invoke(
                elem,
                &[JsValue::String(nbsp_token.into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(
            result,
            JsValue::Bool(true),
            "contains() must find an NBSP-containing token previously added"
        );
    }

    #[test]
    fn add_new_class() {
        let (mut dom, elem, mut session) = setup();
        CLASS_LIST_ADD
            .invoke(
                elem,
                &[JsValue::String("baz".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .collect();
        assert!(classes.contains(&"baz"));
        assert!(classes.contains(&"foo"));
    }

    #[test]
    fn add_existing_class_noop() {
        let (mut dom, elem, mut session) = setup();
        CLASS_LIST_ADD
            .invoke(
                elem,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let count = attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .filter(|c| *c == "foo")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn remove_class() {
        let (mut dom, elem, mut session) = setup();
        CLASS_LIST_REMOVE
            .invoke(
                elem,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert!(!attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .any(|c| c == "foo"));
    }

    #[test]
    fn toggle_adds_when_absent() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_TOGGLE
            .invoke(
                elem,
                &[JsValue::String("baz".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    #[test]
    fn toggle_removes_when_present() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_TOGGLE
            .invoke(
                elem,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    #[test]
    fn contains_true() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_CONTAINS
            .invoke(
                elem,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
    }

    #[test]
    fn contains_false() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_CONTAINS
            .invoke(
                elem,
                &[JsValue::String("missing".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    #[test]
    fn add_rejects_empty_token() {
        let (mut dom, elem, mut session) = setup();
        let err = CLASS_LIST_ADD
            .invoke(
                elem,
                &[JsValue::String(String::new())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn add_rejects_whitespace_token() {
        let (mut dom, elem, mut session) = setup();
        let err = CLASS_LIST_ADD
            .invoke(
                elem,
                &[JsValue::String("a b".into())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn add_normalizes_whitespace() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "  foo  bar  ");
        let elem = dom.create_element("div", attrs);
        let mut session = SessionCore::new();
        CLASS_LIST_ADD
            .invoke(
                elem,
                &[JsValue::String("baz".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert_eq!(attrs.get("class").unwrap(), "foo bar baz");
    }

    // -----------------------------------------------------------------------
    // toggle with force parameter
    // -----------------------------------------------------------------------

    #[test]
    fn toggle_force_true_adds() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_TOGGLE
            .invoke(
                elem,
                &[JsValue::String("baz".into()), JsValue::Bool(true)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert!(attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .any(|c| c == "baz"));
    }

    #[test]
    fn toggle_force_true_keeps_existing() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_TOGGLE
            .invoke(
                elem,
                &[JsValue::String("foo".into()), JsValue::Bool(true)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert!(attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .any(|c| c == "foo"));
    }

    #[test]
    fn toggle_force_false_removes() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_TOGGLE
            .invoke(
                elem,
                &[JsValue::String("foo".into()), JsValue::Bool(false)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        assert!(!attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .any(|c| c == "foo"));
    }

    #[test]
    fn toggle_force_false_noop_when_absent() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_TOGGLE
            .invoke(
                elem,
                &[JsValue::String("baz".into()), JsValue::Bool(false)],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // classList.replace
    // -----------------------------------------------------------------------

    #[test]
    fn replace_existing_class() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_REPLACE
            .invoke(
                elem,
                &[JsValue::String("foo".into()), JsValue::String("baz".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .collect();
        // "baz" should be in the position of "foo" (first).
        assert_eq!(classes, vec!["baz", "bar"]);
    }

    #[test]
    fn replace_missing_class() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_REPLACE
            .invoke(
                elem,
                &[
                    JsValue::String("missing".into()),
                    JsValue::String("baz".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
        // Class string unchanged.
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .collect();
        assert!(classes.contains(&"foo"));
        assert!(classes.contains(&"bar"));
    }

    #[test]
    fn replace_rejects_invalid_token() {
        let (mut dom, elem, mut session) = setup();
        let err = CLASS_LIST_REPLACE
            .invoke(
                elem,
                &[
                    JsValue::String(String::new()),
                    JsValue::String("baz".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
    }

    // -----------------------------------------------------------------------
    // classList.value getter/setter
    // -----------------------------------------------------------------------

    #[test]
    fn value_get() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_VALUE_GET
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("foo bar".into()));
    }

    #[test]
    fn value_set() {
        let (mut dom, elem, mut session) = setup();
        CLASS_LIST_VALUE_SET
            .invoke(
                elem,
                &[JsValue::String("a b c".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = CLASS_LIST_VALUE_GET
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("a b c".into()));
    }

    // -----------------------------------------------------------------------
    // classList.length
    // -----------------------------------------------------------------------

    #[test]
    fn length() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_LENGTH
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(2.0));
    }

    #[test]
    fn length_empty() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = CLASS_LIST_LENGTH
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(0.0));
    }

    // -----------------------------------------------------------------------
    // classList.item
    // -----------------------------------------------------------------------

    #[test]
    fn item_valid_index() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_ITEM
            .invoke(elem, &[JsValue::Number(0.0)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("foo".into()));

        let result = CLASS_LIST_ITEM
            .invoke(elem, &[JsValue::Number(1.0)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("bar".into()));
    }

    #[test]
    fn item_out_of_bounds() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_ITEM
            .invoke(elem, &[JsValue::Number(5.0)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    // -----------------------------------------------------------------------
    // classList.supports
    // -----------------------------------------------------------------------

    #[test]
    fn supports_throws() {
        let (mut dom, elem, mut session) = setup();
        let err = CLASS_LIST_SUPPORTS
            .invoke(
                elem,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::TypeError);
    }

    // -----------------------------------------------------------------------
    // Step 3 spec-compliance tests
    // -----------------------------------------------------------------------

    #[test]
    fn validate_token_whitespace_is_invalid_character_error() {
        let (mut dom, elem, mut session) = setup();
        let err = CLASS_LIST_ADD
            .invoke(
                elem,
                &[JsValue::String("a b".into())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::InvalidCharacterError);
    }

    #[test]
    fn validate_token_empty_is_syntax_error() {
        let (mut dom, elem, mut session) = setup();
        let err = CLASS_LIST_ADD
            .invoke(
                elem,
                &[JsValue::String(String::new())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn contains_no_validate_empty() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_CONTAINS
            .invoke(
                elem,
                &[JsValue::String(String::new())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    #[test]
    fn contains_no_validate_whitespace() {
        let (mut dom, elem, mut session) = setup();
        let result = CLASS_LIST_CONTAINS
            .invoke(
                elem,
                &[JsValue::String("a b".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(false));
    }

    #[test]
    fn length_dedup() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "foo bar foo");
        let elem = dom.create_element("div", attrs);
        let mut session = SessionCore::new();
        let result = CLASS_LIST_LENGTH
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(2.0)); // foo, bar (dedup)
    }

    #[test]
    fn item_dedup() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "foo bar foo");
        let elem = dom.create_element("div", attrs);
        let mut session = SessionCore::new();
        let result = CLASS_LIST_ITEM
            .invoke(elem, &[JsValue::Number(1.0)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("bar".into()));
    }

    #[test]
    fn replace_existing_new_token() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "foo bar baz");
        let elem = dom.create_element("div", attrs);
        let mut session = SessionCore::new();
        // Replace "foo" with "bar" — Infra ordered set "replace":
        // "foo" at index 0 becomes "bar", then existing "bar" at index 1 is removed.
        let result = CLASS_LIST_REPLACE
            .invoke(
                elem,
                &[JsValue::String("foo".into()), JsValue::String("bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .collect();
        assert_eq!(classes, vec!["bar", "baz"]);
    }

    #[test]
    fn replace_infra_ordered_set_position() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("class", "x foo y bar z");
        let elem = dom.create_element("div", attrs);
        let mut session = SessionCore::new();
        // Infra "replace": foo→bar at position 1, remove existing bar at position 3.
        let result = CLASS_LIST_REPLACE
            .invoke(
                elem,
                &[JsValue::String("foo".into()), JsValue::String("bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs
            .get("class")
            .unwrap()
            .split_ascii_whitespace()
            .collect();
        assert_eq!(classes, vec!["x", "bar", "y", "z"]);
    }

    #[test]
    fn supports_throws_type_error() {
        let (mut dom, elem, mut session) = setup();
        let err = CLASS_LIST_SUPPORTS
            .invoke(
                elem,
                &[JsValue::String("foo".into())],
                &mut session,
                &mut dom,
            )
            .unwrap_err();
        assert_eq!(err.kind, DomApiErrorKind::TypeError);
    }
}
