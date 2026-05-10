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
    if !current.split_whitespace().any(|c| c == token) {
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
    let new_tokens: Vec<&str> = current.split_whitespace().filter(|c| *c != token).collect();
    set_token_string(entity, dom, attr_name, new_tokens.join(" "))?;
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
        add_token(this, "class", &class_name, dom)?;
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
        remove_token(this, "class", &class_name, dom)?;
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

        let current = get_token_string(this, dom, "class")?;
        let has = current.split_whitespace().any(|c| c == class_name);

        let result = match force {
            Some(true) => {
                if !has {
                    add_token(this, "class", &class_name, dom)?;
                }
                true
            }
            Some(false) => {
                if has {
                    remove_token(this, "class", &class_name, dom)?;
                }
                false
            }
            None => {
                if has {
                    remove_token(this, "class", &class_name, dom)?;
                    false
                } else {
                    add_token(this, "class", &class_name, dom)?;
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
        let current = get_token_string(this, dom, "class")?;
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

        let current = get_token_string(this, dom, "class")?;
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
        set_token_string(this, dom, "class", result.join(" "))?;
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
        let s = get_token_string(this, dom, "class")?;
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
        set_token_string(this, dom, "class", value)?;
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
        let s = get_token_string(this, dom, "class")?;
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
        let s = get_token_string(this, dom, "class")?;
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

// ===========================================================================
// relList — `<a>.relList` / `<area>.relList` / `<link>.relList`
// (HTML §4.6.5).  Same DOMTokenList algorithms backed by the `rel`
// content attribute instead of `class`.
// ===========================================================================

/// `element.relList.add(token)` — adds a rel token if not present.
pub struct RelListAdd;
impl DomApiHandler for RelListAdd {
    fn method_name(&self) -> &str {
        "relList.add"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let token = require_string_arg(args, 0)?;
        validate_token(&token)?;
        add_token(this, "rel", &token, dom)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `element.relList.remove(token)` — removes a rel token.
pub struct RelListRemove;
impl DomApiHandler for RelListRemove {
    fn method_name(&self) -> &str {
        "relList.remove"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let token = require_string_arg(args, 0)?;
        validate_token(&token)?;
        remove_token(this, "rel", &token, dom)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `element.relList.toggle(token, force?)` — toggles a rel token and
/// returns the new state.
pub struct RelListToggle;
impl DomApiHandler for RelListToggle {
    fn method_name(&self) -> &str {
        "relList.toggle"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let token = require_string_arg(args, 0)?;
        validate_token(&token)?;
        let force = match args.get(1) {
            Some(JsValue::Bool(b)) => Some(*b),
            _ => None,
        };
        let current = get_token_string(this, dom, "rel")?;
        let has = current.split_whitespace().any(|c| c == token);
        let result = match force {
            Some(true) => {
                if !has {
                    add_token(this, "rel", &token, dom)?;
                }
                true
            }
            Some(false) => {
                if has {
                    remove_token(this, "rel", &token, dom)?;
                }
                false
            }
            None => {
                if has {
                    remove_token(this, "rel", &token, dom)?;
                    false
                } else {
                    add_token(this, "rel", &token, dom)?;
                    true
                }
            }
        };
        dom.rev_version(this);
        Ok(JsValue::Bool(result))
    }
}

/// `element.relList.contains(token)` — checks if a rel token is present.
pub struct RelListContains;
impl DomApiHandler for RelListContains {
    fn method_name(&self) -> &str {
        "relList.contains"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let token = require_string_arg(args, 0)?;
        let current = get_token_string(this, dom, "rel")?;
        let has = current.split_whitespace().any(|c| c == token);
        Ok(JsValue::Bool(has))
    }
}

/// `element.relList.replace(old, new)` — replaces a rel token in
/// place.  Returns `true` if the old token was found.
pub struct RelListReplace;
impl DomApiHandler for RelListReplace {
    fn method_name(&self) -> &str {
        "relList.replace"
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
        let current = get_token_string(this, dom, "rel")?;
        let tokens: Vec<&str> = current.split_whitespace().collect();
        if !tokens.contains(&old_token.as_str()) {
            return Ok(JsValue::Bool(false));
        }
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
        set_token_string(this, dom, "rel", result.join(" "))?;
        dom.rev_version(this);
        Ok(JsValue::Bool(true))
    }
}

/// `element.relList.value` getter — returns the raw rel string.
pub struct RelListValueGet;
impl DomApiHandler for RelListValueGet {
    fn method_name(&self) -> &str {
        "relList.value.get"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let s = get_token_string(this, dom, "rel")?;
        Ok(JsValue::String(s))
    }
}

/// `element.relList.value` setter — sets the rel attribute directly.
pub struct RelListValueSet;
impl DomApiHandler for RelListValueSet {
    fn method_name(&self) -> &str {
        "relList.value.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = require_string_arg(args, 0)?;
        set_token_string(this, dom, "rel", value)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `element.relList.length` — returns the number of rel tokens.
pub struct RelListLength;
impl DomApiHandler for RelListLength {
    fn method_name(&self) -> &str {
        "relList.length"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let s = get_token_string(this, dom, "rel")?;
        let count = parse_ordered_set(&s).len();
        #[allow(clippy::cast_precision_loss)]
        Ok(JsValue::Number(count as f64))
    }
}

/// `element.relList.item(index)` — returns the token at the index or
/// `Null` if out of bounds.
pub struct RelListItem;
impl DomApiHandler for RelListItem {
    fn method_name(&self) -> &str {
        "relList.item"
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
            Some(JsValue::Number(n)) => *n as usize,
            _ => {
                return Err(DomApiError {
                    kind: DomApiErrorKind::TypeError,
                    message: "relList.item: argument 0 must be a number".into(),
                });
            }
        };
        let s = get_token_string(this, dom, "rel")?;
        match parse_ordered_set(&s).get(index) {
            Some(token) => Ok(JsValue::String((*token).to_string())),
            None => Ok(JsValue::Null),
        }
    }
}

// ===========================================================================
// linkSizes — `<link>.sizes` (HTML §4.6.7,
// `[SameObject, PutForwards=value] DOMTokenList`).  Backed by the
// `sizes` content attribute.  Slot `#11-tags-T2a-url-bearing` (D-4).
// ===========================================================================

/// `<link>.sizes.add(token)` — adds a sizes token if not present.
pub struct LinkSizesAdd;
impl DomApiHandler for LinkSizesAdd {
    fn method_name(&self) -> &str {
        "linkSizes.add"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let token = require_string_arg(args, 0)?;
        validate_token(&token)?;
        add_token(this, "sizes", &token, dom)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `<link>.sizes.remove(token)` — removes a sizes token.
pub struct LinkSizesRemove;
impl DomApiHandler for LinkSizesRemove {
    fn method_name(&self) -> &str {
        "linkSizes.remove"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let token = require_string_arg(args, 0)?;
        validate_token(&token)?;
        remove_token(this, "sizes", &token, dom)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `<link>.sizes.toggle(token, force?)` — toggles a sizes token.
pub struct LinkSizesToggle;
impl DomApiHandler for LinkSizesToggle {
    fn method_name(&self) -> &str {
        "linkSizes.toggle"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let token = require_string_arg(args, 0)?;
        validate_token(&token)?;
        let force = match args.get(1) {
            Some(JsValue::Bool(b)) => Some(*b),
            _ => None,
        };
        let current = get_token_string(this, dom, "sizes")?;
        let has = current.split_whitespace().any(|c| c == token);
        let result = match force {
            Some(true) => {
                if !has {
                    add_token(this, "sizes", &token, dom)?;
                }
                true
            }
            Some(false) => {
                if has {
                    remove_token(this, "sizes", &token, dom)?;
                }
                false
            }
            None => {
                if has {
                    remove_token(this, "sizes", &token, dom)?;
                    false
                } else {
                    add_token(this, "sizes", &token, dom)?;
                    true
                }
            }
        };
        dom.rev_version(this);
        Ok(JsValue::Bool(result))
    }
}

/// `<link>.sizes.contains(token)` — checks if a sizes token is present.
pub struct LinkSizesContains;
impl DomApiHandler for LinkSizesContains {
    fn method_name(&self) -> &str {
        "linkSizes.contains"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let token = require_string_arg(args, 0)?;
        let current = get_token_string(this, dom, "sizes")?;
        let has = current.split_whitespace().any(|c| c == token);
        Ok(JsValue::Bool(has))
    }
}

/// `<link>.sizes.replace(old, new)`.
pub struct LinkSizesReplace;
impl DomApiHandler for LinkSizesReplace {
    fn method_name(&self) -> &str {
        "linkSizes.replace"
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
        let current = get_token_string(this, dom, "sizes")?;
        let tokens: Vec<&str> = current.split_whitespace().collect();
        if !tokens.contains(&old_token.as_str()) {
            return Ok(JsValue::Bool(false));
        }
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
        set_token_string(this, dom, "sizes", result.join(" "))?;
        dom.rev_version(this);
        Ok(JsValue::Bool(true))
    }
}

/// `<link>.sizes.value` getter.
pub struct LinkSizesValueGet;
impl DomApiHandler for LinkSizesValueGet {
    fn method_name(&self) -> &str {
        "linkSizes.value.get"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let s = get_token_string(this, dom, "sizes")?;
        Ok(JsValue::String(s))
    }
}

/// `<link>.sizes.value` setter.
pub struct LinkSizesValueSet;
impl DomApiHandler for LinkSizesValueSet {
    fn method_name(&self) -> &str {
        "linkSizes.value.set"
    }
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = require_string_arg(args, 0)?;
        set_token_string(this, dom, "sizes", value)?;
        dom.rev_version(this);
        Ok(JsValue::Undefined)
    }
}

/// `<link>.sizes.length`.
pub struct LinkSizesLength;
impl DomApiHandler for LinkSizesLength {
    fn method_name(&self) -> &str {
        "linkSizes.length"
    }
    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let s = get_token_string(this, dom, "sizes")?;
        let count = parse_ordered_set(&s).len();
        #[allow(clippy::cast_precision_loss)]
        Ok(JsValue::Number(count as f64))
    }
}

/// `<link>.sizes.item(index)`.
pub struct LinkSizesItem;
impl DomApiHandler for LinkSizesItem {
    fn method_name(&self) -> &str {
        "linkSizes.item"
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
            Some(JsValue::Number(n)) => *n as usize,
            _ => {
                return Err(DomApiError {
                    kind: DomApiErrorKind::TypeError,
                    message: "linkSizes.item: argument 0 must be a number".into(),
                });
            }
        };
        let s = get_token_string(this, dom, "sizes")?;
        match parse_ordered_set(&s).get(index) {
            Some(token) => Ok(JsValue::String((*token).to_string())),
            None => Ok(JsValue::Null),
        }
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
