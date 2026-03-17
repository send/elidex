//! classList DOM API handlers: add, remove, toggle, contains.
//!
//! Class names are split via `split_whitespace()` and re-joined with single
//! spaces. This implicitly normalizes the `class` attribute (tabs, newlines,
//! and consecutive spaces become a single space). This matches browser behavior
//! where `classList` operations normalize whitespace.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind, DomApiHandler, SessionCore};

use crate::util::{not_found_error, require_string_arg};

/// Validate a class token per the `DOMTokenList` spec.
///
/// Returns `SyntaxError` for empty strings and `InvalidCharacterError` for
/// strings containing whitespace.
fn validate_token(token: &str) -> Result<(), DomApiError> {
    if token.is_empty() {
        return Err(DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: "class name must not be empty".into(),
        });
    }
    if token.contains(char::is_whitespace) {
        return Err(DomApiError {
            kind: DomApiErrorKind::InvalidCharacterError,
            message: "class name must not contain whitespace".into(),
        });
    }
    Ok(())
}

/// Parse a class string as an ordered set: split on whitespace, deduplicate
/// while preserving first-occurrence order.
fn parse_ordered_set(class_str: &str) -> Vec<&str> {
    let mut seen = Vec::new();
    for token in class_str.split_whitespace() {
        if !seen.contains(&token) {
            seen.push(token);
        }
    }
    seen
}

/// Normalize whitespace in a class string: collapse multiple spaces, trim.
fn normalize_class_string(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn get_class_string(entity: Entity, dom: &EcsDom) -> Result<String, DomApiError> {
    let attrs = dom
        .world()
        .get::<&Attributes>(entity)
        .map_err(|_| not_found_error("element not found"))?;
    Ok(attrs.get("class").unwrap_or("").to_string())
}

fn set_class_string(entity: Entity, dom: &mut EcsDom, value: String) -> Result<(), DomApiError> {
    let mut attrs = dom
        .world_mut()
        .get::<&mut Attributes>(entity)
        .map_err(|_| not_found_error("element not found"))?;
    attrs.set("class", value);
    Ok(())
}

/// Add a class name to the class string if not already present.
fn add_class(entity: Entity, class_name: &str, dom: &mut EcsDom) -> Result<(), DomApiError> {
    let current = get_class_string(entity, dom)?;
    if !current.split_whitespace().any(|c| c == class_name) {
        let normalized = normalize_class_string(&current);
        let new_class = if normalized.is_empty() {
            class_name.to_string()
        } else {
            format!("{normalized} {class_name}")
        };
        set_class_string(entity, dom, new_class)?;
    }
    Ok(())
}

/// Remove a class name from the class string.
fn remove_class(entity: Entity, class_name: &str, dom: &mut EcsDom) -> Result<(), DomApiError> {
    let current = get_class_string(entity, dom)?;
    let new_class: Vec<&str> = current
        .split_whitespace()
        .filter(|c| *c != class_name)
        .collect();
    set_class_string(entity, dom, new_class.join(" "))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// classList.add
// ---------------------------------------------------------------------------

/// `element.classList.add(className)` — adds a class if not present.
pub struct ClassListAdd;

impl DomApiHandler for ClassListAdd {
    fn method_name(&self) -> &str {
        "classList.add"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let class_name = require_string_arg(args, 0)?;
        validate_token(&class_name)?;
        add_class(this, &class_name, dom)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// classList.remove
// ---------------------------------------------------------------------------

/// `element.classList.remove(className)` — removes a class.
pub struct ClassListRemove;

impl DomApiHandler for ClassListRemove {
    fn method_name(&self) -> &str {
        "classList.remove"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let class_name = require_string_arg(args, 0)?;
        validate_token(&class_name)?;
        remove_class(this, &class_name, dom)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// classList.toggle
// ---------------------------------------------------------------------------

/// `element.classList.toggle(className)` — toggles a class and returns the new state.
pub struct ClassListToggle;

impl DomApiHandler for ClassListToggle {
    fn method_name(&self) -> &str {
        "classList.toggle"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let class_name = require_string_arg(args, 0)?;
        validate_token(&class_name)?;

        // Optional force parameter (second argument).
        let force = match args.get(1) {
            Some(JsValue::Bool(b)) => Some(*b),
            _ => None,
        };

        let current = get_class_string(this, dom)?;
        let has = current.split_whitespace().any(|c| c == class_name);

        let result = match force {
            Some(true) => {
                if !has {
                    add_class(this, &class_name, dom)?;
                }
                true
            }
            Some(false) => {
                if has {
                    remove_class(this, &class_name, dom)?;
                }
                false
            }
            None => {
                if has {
                    remove_class(this, &class_name, dom)?;
                    false
                } else {
                    add_class(this, &class_name, dom)?;
                    true
                }
            }
        };
        dom.rev_version(this);
        Ok(JsValue::Bool(result))
    }
}

// ---------------------------------------------------------------------------
// classList.contains
// ---------------------------------------------------------------------------

/// `element.classList.contains(className)` — checks if a class is present.
pub struct ClassListContains;

impl DomApiHandler for ClassListContains {
    fn method_name(&self) -> &str {
        "classList.contains"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let class_name = require_string_arg(args, 0)?;
        let current = get_class_string(this, dom)?;
        let has = current.split_whitespace().any(|c| c == class_name);
        Ok(JsValue::Bool(has))
    }
}

// ---------------------------------------------------------------------------
// classList.replace
// ---------------------------------------------------------------------------

/// `element.classList.replace(old, new)` — replaces a class token, preserving order.
///
/// Returns `true` if the old token was found and replaced, `false` otherwise.
pub struct ClassListReplace;

impl DomApiHandler for ClassListReplace {
    fn method_name(&self) -> &str {
        "classList.replace"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let old_token = require_string_arg(args, 0)?;
        let new_token = require_string_arg(args, 1)?;
        validate_token(&old_token)?;
        validate_token(&new_token)?;

        let current = get_class_string(this, dom)?;
        let tokens: Vec<&str> = current.split_whitespace().collect();
        if !tokens.contains(&old_token.as_str()) {
            return Ok(JsValue::Bool(false));
        }

        // Infra ordered set "replace" algorithm:
        // 1. Replace the first occurrence of old_token with new_token.
        // 2. Remove all subsequent occurrences of new_token (dedup).
        let mut replaced = false;
        let mut result: Vec<&str> = Vec::with_capacity(tokens.len());
        for t in &tokens {
            if !replaced && *t == old_token.as_str() {
                result.push(new_token.as_str());
                replaced = true;
            } else if *t != new_token.as_str() {
                result.push(t);
            }
            // Skip duplicate new_token (either pre-existing or from replacement).
        }
        set_class_string(this, dom, result.join(" "))?;
        dom.rev_version(this);
        Ok(JsValue::Bool(true))
    }
}

// ---------------------------------------------------------------------------
// classList.value getter/setter
// ---------------------------------------------------------------------------

/// `element.classList.value` getter — returns the raw class string.
pub struct ClassListValueGet;

impl DomApiHandler for ClassListValueGet {
    fn method_name(&self) -> &str {
        "classList.value.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let s = get_class_string(this, dom)?;
        Ok(JsValue::String(s))
    }
}

/// `element.classList.value` setter — sets the class attribute directly.
pub struct ClassListValueSet;

impl DomApiHandler for ClassListValueSet {
    fn method_name(&self) -> &str {
        "classList.value.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = require_string_arg(args, 0)?;
        set_class_string(this, dom, value)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// classList.length
// ---------------------------------------------------------------------------

/// `element.classList.length` — returns the number of class tokens.
pub struct ClassListLength;

impl DomApiHandler for ClassListLength {
    fn method_name(&self) -> &str {
        "classList.length"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let s = get_class_string(this, dom)?;
        let count = parse_ordered_set(&s).len();
        #[allow(clippy::cast_precision_loss)] // DOM IDL uses f64 for all numeric values
        Ok(JsValue::Number(count as f64))
    }
}

// ---------------------------------------------------------------------------
// classList.item
// ---------------------------------------------------------------------------

/// `element.classList.item(index)` — returns the token at the given index,
/// or `Null` if out of bounds.
pub struct ClassListItem;

impl DomApiHandler for ClassListItem {
    fn method_name(&self) -> &str {
        "classList.item"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let index = match args.first() {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // DOM IDL index is unsigned; JS Number → usize is intentional
            Some(JsValue::Number(n)) => *n as usize,
            _ => {
                return Err(DomApiError {
                    kind: DomApiErrorKind::TypeError,
                    message: "classList.item: argument 0 must be a number".into(),
                });
            }
        };
        let s = get_class_string(this, dom)?;
        match parse_ordered_set(&s).get(index) {
            Some(token) => Ok(JsValue::String((*token).to_string())),
            None => Ok(JsValue::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// classList.supports
// ---------------------------------------------------------------------------

/// `element.classList.supports()` — always throws `TypeError`.
///
/// `DOMTokenList.supports()` is only meaningful for specific interfaces
/// (e.g., `<link rel>`, `<iframe sandbox>`), not for `classList`.
pub struct ClassListSupports;

impl DomApiHandler for ClassListSupports {
    fn method_name(&self) -> &str {
        "classList.supports"
    }

    fn invoke(
        &self,
        _this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        _dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        Err(DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: "classList.supports() is not supported for classList".into(),
        })
    }
}

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
    fn add_new_class() {
        let (mut dom, elem, mut session) = setup();
        ClassListAdd
            .invoke(
                elem,
                &[JsValue::String("baz".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs.get("class").unwrap().split_whitespace().collect();
        assert!(classes.contains(&"baz"));
        assert!(classes.contains(&"foo"));
    }

    #[test]
    fn add_existing_class_noop() {
        let (mut dom, elem, mut session) = setup();
        ClassListAdd
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
            .split_whitespace()
            .filter(|c| *c == "foo")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn remove_class() {
        let (mut dom, elem, mut session) = setup();
        ClassListRemove
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
            .split_whitespace()
            .any(|c| c == "foo"));
    }

    #[test]
    fn toggle_adds_when_absent() {
        let (mut dom, elem, mut session) = setup();
        let result = ClassListToggle
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
        let result = ClassListToggle
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
        let result = ClassListContains
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
        let result = ClassListContains
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
        let err = ClassListAdd
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
        let err = ClassListAdd
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
        ClassListAdd
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
        let result = ClassListToggle
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
            .split_whitespace()
            .any(|c| c == "baz"));
    }

    #[test]
    fn toggle_force_true_keeps_existing() {
        let (mut dom, elem, mut session) = setup();
        let result = ClassListToggle
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
            .split_whitespace()
            .any(|c| c == "foo"));
    }

    #[test]
    fn toggle_force_false_removes() {
        let (mut dom, elem, mut session) = setup();
        let result = ClassListToggle
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
            .split_whitespace()
            .any(|c| c == "foo"));
    }

    #[test]
    fn toggle_force_false_noop_when_absent() {
        let (mut dom, elem, mut session) = setup();
        let result = ClassListToggle
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
        let result = ClassListReplace
            .invoke(
                elem,
                &[JsValue::String("foo".into()), JsValue::String("baz".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs.get("class").unwrap().split_whitespace().collect();
        // "baz" should be in the position of "foo" (first).
        assert_eq!(classes, vec!["baz", "bar"]);
    }

    #[test]
    fn replace_missing_class() {
        let (mut dom, elem, mut session) = setup();
        let result = ClassListReplace
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
        let classes: Vec<&str> = attrs.get("class").unwrap().split_whitespace().collect();
        assert!(classes.contains(&"foo"));
        assert!(classes.contains(&"bar"));
    }

    #[test]
    fn replace_rejects_invalid_token() {
        let (mut dom, elem, mut session) = setup();
        let err = ClassListReplace
            .invoke(
                elem,
                &[JsValue::String("".into()), JsValue::String("baz".into())],
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
        let result = ClassListValueGet
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("foo bar".into()));
    }

    #[test]
    fn value_set() {
        let (mut dom, elem, mut session) = setup();
        ClassListValueSet
            .invoke(
                elem,
                &[JsValue::String("a b c".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let result = ClassListValueGet
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
        let result = ClassListLength
            .invoke(elem, &[], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::Number(2.0));
    }

    #[test]
    fn length_empty() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = ClassListLength
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
        let result = ClassListItem
            .invoke(elem, &[JsValue::Number(0.0)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("foo".into()));

        let result = ClassListItem
            .invoke(elem, &[JsValue::Number(1.0)], &mut session, &mut dom)
            .unwrap();
        assert_eq!(result, JsValue::String("bar".into()));
    }

    #[test]
    fn item_out_of_bounds() {
        let (mut dom, elem, mut session) = setup();
        let result = ClassListItem
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
        let err = ClassListSupports
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
        let err = ClassListAdd
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
        let err = ClassListAdd
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
        let result = ClassListContains
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
        let result = ClassListContains
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
        let result = ClassListLength
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
        let result = ClassListItem
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
        let result = ClassListReplace
            .invoke(
                elem,
                &[JsValue::String("foo".into()), JsValue::String("bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs.get("class").unwrap().split_whitespace().collect();
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
        let result = ClassListReplace
            .invoke(
                elem,
                &[JsValue::String("foo".into()), JsValue::String("bar".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Bool(true));
        let attrs = dom.world().get::<&Attributes>(elem).unwrap();
        let classes: Vec<&str> = attrs.get("class").unwrap().split_whitespace().collect();
        assert_eq!(classes, vec!["x", "bar", "y", "z"]);
    }

    #[test]
    fn supports_throws_type_error() {
        let (mut dom, elem, mut session) = setup();
        let err = ClassListSupports
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
