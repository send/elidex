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
/// Returns `SyntaxError` for empty strings and strings containing whitespace.
fn validate_token(token: &str) -> Result<(), DomApiError> {
    if token.is_empty() {
        return Err(DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: "class name must not be empty".into(),
        });
    }
    if token.contains(char::is_whitespace) {
        return Err(DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: "class name must not contain whitespace".into(),
        });
    }
    Ok(())
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
        let current = get_class_string(this, dom)?;
        if current.split_whitespace().any(|c| c == class_name) {
            remove_class(this, &class_name, dom)?;
            Ok(JsValue::Bool(false))
        } else {
            add_class(this, &class_name, dom)?;
            Ok(JsValue::Bool(true))
        }
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
        validate_token(&class_name)?;
        let current = get_class_string(this, dom)?;
        let has = current.split_whitespace().any(|c| c == class_name);
        Ok(JsValue::Bool(has))
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
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
        assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
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
}
