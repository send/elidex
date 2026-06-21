//! Document-level DOM API handlers: querySelector, getElementById, createElement, createTextNode.

use elidex_css::{parse_selector_from_str, Selector};
use elidex_ecs::{Attributes, EcsDom, Entity, TagType};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiError, DomApiErrorKind, DomApiHandler, SessionCore,
};

use crate::util::require_string_arg;

// ---------------------------------------------------------------------------
// querySelector
// ---------------------------------------------------------------------------

/// `document.querySelector(selector)` — returns the first matching element or null.
pub struct QuerySelector;

impl DomApiHandler for QuerySelector {
    fn method_name(&self) -> &str {
        "querySelector"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let selector_str = require_string_arg(args, 0)?;
        let selectors = parse_selector_from_str(&selector_str).map_err(|()| DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: format!("Invalid selector: {selector_str}"),
        })?;
        reject_shadow_pseudos(&selectors)?;
        let mut matched = None;
        dom.traverse_descendants(this, |entity| {
            if dom.world().get::<&TagType>(entity).is_ok()
                && selectors.iter().any(|sel| sel.matches(entity, dom))
            {
                matched = Some(entity);
                false
            } else {
                true
            }
        });
        match matched {
            Some(entity) => {
                let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
                Ok(JsValue::ObjectRef(obj_ref.to_raw()))
            }
            None => Ok(JsValue::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// querySelectorAll (standalone helper — returns Vec<Entity>)
// ---------------------------------------------------------------------------

/// Find all elements matching any of the given selectors under `root`.
///
/// Returns entities in document order (pre-order DFS).
/// This is a standalone function (not a `DomApiHandler`) because it returns
/// multiple entities; the boa bridge converts the result to a JS array.
pub fn query_selector_all(
    root: Entity,
    selector_str: &str,
    dom: &EcsDom,
) -> Result<Vec<Entity>, DomApiError> {
    let selectors = parse_selector_from_str(selector_str).map_err(|()| DomApiError {
        kind: DomApiErrorKind::SyntaxError,
        message: format!("Invalid selector: {selector_str}"),
    })?;
    reject_shadow_pseudos(&selectors)?;
    let mut result = Vec::new();
    dom.traverse_descendants(root, |entity| {
        if dom.world().get::<&TagType>(entity).is_ok()
            && selectors.iter().any(|sel| sel.matches(entity, dom))
        {
            result.push(entity);
        }
        true
    });
    Ok(result)
}

// ---------------------------------------------------------------------------
// getElementById
// ---------------------------------------------------------------------------

/// `document.getElementById(id)` — returns the element with matching id or null.
pub struct GetElementById;

impl DomApiHandler for GetElementById {
    fn method_name(&self) -> &str {
        "getElementById"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let id = require_string_arg(args, 0)?;
        match dom.find_by_id(this, &id) {
            Some(entity) => {
                let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
                Ok(JsValue::ObjectRef(obj_ref.to_raw()))
            }
            None => Ok(JsValue::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// createElement
// ---------------------------------------------------------------------------

/// `document.createElement(tagName)` — creates a new element.
pub struct CreateElement;

/// Validates that a tag name contains only ASCII alphanumeric, `-`, `_`, and `.`.
///
/// `pub` because the engine bindings run the same check BEFORE the
/// flatten-algorithm NotSupportedError gates: DOM §4.5 `createElement`
/// step 1 throws `InvalidCharacterError` before step 3 "flatten
/// element creation options" raises its conflict / foreign-registry
/// errors (WebIDL *conversion* TypeErrors still precede both — they
/// happen at argument-conversion time). The handler below re-checks
/// (it stays self-contained for direct callers); this fn is the
/// single shared predicate.
pub fn is_valid_element_tag_name(name: &str) -> bool {
    is_valid_tag_name(name)
}

fn is_valid_tag_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

impl DomApiHandler for CreateElement {
    fn method_name(&self) -> &str {
        "createElement"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let tag = require_string_arg(args, 0)?;
        if !is_valid_tag_name(&tag) {
            // WHATWG DOM §4.5 "Interface Document" (`createElement`
            // method steps, step 1): an invalid local name throws
            // `InvalidCharacterError`, NOT `SyntaxError`.
            // `SyntaxError` is reserved for selector / URL / CSS-OM
            // parse failures (legacy code 12 per the WebIDL error
            // names table) and would surface the wrong
            // `DOMException.name` to JS
            // (`e.name === "InvalidCharacterError"` vs `"SyntaxError"`).
            return Err(DomApiError {
                kind: DomApiErrorKind::InvalidCharacterError,
                message: format!("Invalid tag name: {tag}"),
            });
        }
        // Optional second arg = the flattened creation options (WHATWG
        // DOM §4.5 createElement step 3 "flatten element creation
        // options") — the engine host MUST discriminate the WebIDL
        // `(DOMString or ElementCreationOptions)` union and pass only
        // the flattened result here; forwarding raw JS args verbatim
        // would let a DOMString `options` masquerade as an is value.
        // Encoding: `String` = the is value (no validity check — DOM
        // §4.9 step 6.3 marks on non-null `is` regardless of name
        // validity); `Null` = an explicit `customElementRegistry:
        // null` (mutually exclusive with `is` by the flatten step
        // 3.2.1 conflict, so one positional slot carries both).
        let is_value: Option<&str> = match args.get(1) {
            Some(JsValue::String(s)) => Some(s.as_str()),
            _ => None,
        };
        let null_registry = matches!(args.get(1), Some(JsValue::Null));
        // Single canonical local name: both the TagType and the
        // custom-element derivation read the same folded binding —
        // feeding the pre-fold `tag` into the derivation would skip
        // CE marking for `createElement('MY-EL')` (valid custom
        // element names are lowercase-first).
        let local_name = tag.to_ascii_lowercase();
        // Anchor the new node's "node document" (WHATWG DOM §4.4) to
        // the receiver Document so `newEl.ownerDocument` reports the
        // creating document even before insertion — critical for
        // clones and iframes where the bound global and the receiver
        // differ.  Pre-VM dispatch this lived in
        // `vm/host/document.rs::native_document_create_element` as an
        // explicit `create_element_with_owner` call; the handler now
        // owns the spec-precise behaviour so both boa and VM paths
        // observe the same ownerDocument semantics.
        // WHATWG DOM §4.9 "create an element" intrinsic-component derivation,
        // via the single canonical `derive_created_element_components` shared
        // with the parser's `attach_derived` (One-issue-one-way). Computed
        // before the entity spawn so `local_name` can move into
        // `create_element_with_owner`. `createElement` creates with NO
        // attributes (DOM §4.5 has no `is`/attribute step), so an `<iframe>`
        // derives a *present default* `IframeData` — the later
        // `setAttribute("src",…)` then populates it through the presence-gated
        // reconcile seam, and without it a script-created iframe is invisible
        // to the shell loader. The `is` value here comes from the options
        // argument; serialization compensates the absent `is` content
        // attribute via the HTML §13.3 is-value step in `serialize_node`.
        let components = elidex_custom_elements::derive_created_element_components(
            &local_name,
            elidex_ecs::Namespace::Html,
            is_value,
            &Attributes::default(),
        );
        let mut ce_state = components.custom_element_state;
        // DOM §4.9 "create an element internal" step 1: the created
        // element's custom element registry is the caller-supplied
        // one — an explicit null puts the element outside every
        // registry (never upgraded; the bindings' routing and the
        // define()/upgrade() walks all gate on this field).
        if null_registry {
            if let Some(state) = ce_state.as_mut() {
                state.registry = elidex_custom_elements::RegistryAssociation::Null;
            }
        }
        let is_template = local_name == "template";
        let entity = dom.create_element_with_owner(local_name, Attributes::default(), Some(this));
        if let Some(ce_state) = ce_state {
            let _ = dom.world_mut().insert_one(entity, ce_state);
        }
        // HTML §4.8.5: attach the derived `IframeData` (present-default on the
        // no-attrs createElement path) so the reconcile seam can populate
        // src/srcdoc and the iframe is *discoverable* as an iframe.
        //
        // NB: this is the DOM-side prerequisite, not the whole story for
        // *loading* a script-created iframe. The shell's `detect_iframe_mutations`
        // only loads iframes it sees in `SessionCore::flush()` mutation records,
        // but tree mutations (`appendChild`/`insertBefore`) bypass
        // `record_mutation` today (see `child_node/mutations.rs` module doc:
        // "tracked for a future milestone"), so `createElement('iframe')` +
        // `appendChild` produces no ChildList record and the iframe does not yet
        // load on connection. Routing tree mutations through `record_mutation`
        // (which also fires CE connected/disconnected callbacks) is the coupled
        // dependency tracked at `#11-tree-mutation-record-pipeline`.
        if let Some(iframe_data) = components.iframe_data {
            let _ = dom.world_mut().insert_one(entity, iframe_data);
        }
        // HTML §4.12.3: a `<template>` element has an associated content
        // `DocumentFragment` "created when the template element is created", so
        // `createElement('template')` gets an (empty) fragment up front — the
        // same eager attach the parser tiers do, via the one shared helper
        // (One-issue-one-way). `this` is the receiver document = the fragment's
        // owner document (C2 approximation; faithful inert owner document is
        // slot `#11-template-contents-owner-document`).
        if is_template {
            let _ = dom.attach_template_contents(entity, Some(this));
        }
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::Element);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

// ---------------------------------------------------------------------------
// createTextNode
// ---------------------------------------------------------------------------

/// `document.createTextNode(data)` — creates a new text node.
pub struct CreateTextNode;

impl DomApiHandler for CreateTextNode {
    fn method_name(&self) -> &str {
        "createTextNode"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = require_string_arg(args, 0)?;
        let entity = dom.create_text_with_owner(text, Some(this));
        let obj_ref = session.get_or_create_wrapper(entity, ComponentKind::TextNode);
        Ok(JsValue::ObjectRef(obj_ref.to_raw()))
    }
}

// ---------------------------------------------------------------------------
// DOM tree walk helpers
// ---------------------------------------------------------------------------

/// Check that no selector uses shadow-scoped pseudos (`:host`, `::slotted()`).
///
/// These are invalid in `querySelector` / `querySelectorAll` /
/// `Element.matches` / `Element.closest` per CSS Scoping §3 — the
/// pseudos are only valid inside a shadow tree's scoped style or
/// when the selector is being matched from a `ShadowRoot`.  Shared
/// across selector-accepting handlers in this crate (intentional
/// `pub(crate)` visibility — no `elidex-dom-api` external caller
/// has a reason to validate selector shape independently of a handler).
pub(crate) fn reject_shadow_pseudos(selectors: &[Selector]) -> Result<(), DomApiError> {
    if selectors.iter().any(Selector::has_shadow_pseudo) {
        return Err(DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            // API-neutral wording — the helper is shared across
            // `querySelector` / `querySelectorAll` / `matches` /
            // `closest`, so naming a single API in the message
            // misleads callers of the others.
            message: ":host and ::slotted() are only valid inside a shadow tree".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;

    fn setup_dom() -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc, html);
        dom.append_child(html, body);

        let mut div_attrs = Attributes::default();
        div_attrs.set("id", "target");
        div_attrs.set("class", "box");
        let div = dom.create_element("div", div_attrs);
        dom.append_child(body, div);

        let span = dom.create_element("span", Attributes::default());
        dom.append_child(div, span);

        let p = dom.create_element("p", Attributes::default());
        dom.append_child(body, p);

        (dom, doc)
    }

    #[test]
    fn query_selector_by_tag() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("span".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn query_selector_by_id() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("#target".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn query_selector_no_match() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("article".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn query_selector_invalid_selector() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler.invoke(
            doc,
            &[JsValue::String(">>>".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn query_selector_all_multiple() {
        let (dom, doc) = setup_dom();
        let matches = query_selector_all(doc, "div, p", &dom).unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn get_element_by_id_found() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = GetElementById;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("target".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn get_element_by_id_not_found() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = GetElementById;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("nonexistent".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::Null);
    }

    #[test]
    fn create_element_returns_ref() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = CreateElement;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("section".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn create_text_node_returns_ref() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = CreateTextNode;
        let result = handler
            .invoke(
                doc,
                &[JsValue::String("hello world".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::ObjectRef(_)));
    }

    #[test]
    fn query_selector_missing_arg() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler.invoke(doc, &[], &mut session, &mut dom);
        assert!(result.is_err());
    }

    // --- M2: Shadow pseudo rejection in querySelector ---

    #[test]
    fn query_selector_host_rejected() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = QuerySelector;
        let result = handler.invoke(
            doc,
            &[JsValue::String(":host".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn query_selector_slotted_rejected() {
        let (dom, doc) = setup_dom();
        let result = query_selector_all(doc, "::slotted(div)", &dom);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, DomApiErrorKind::SyntaxError);
    }

    #[test]
    fn create_element_invalid_tag_name_throws_invalid_character_error() {
        // WHATWG DOM §4.5 "Interface Document" (`createElement`
        // method steps, step 1): an invalid local name throws
        // `InvalidCharacterError` — *not* `SyntaxError`, which is
        // reserved for selector / URL / CSS-OM parse failures
        // (legacy code 12 per the WebIDL error names table).  A space
        // in the tag name is the simplest invalid-name input that
        // trips `is_valid_tag_name`.
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let handler = CreateElement;
        let result = handler.invoke(
            doc,
            &[JsValue::String("bad tag".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            DomApiErrorKind::InvalidCharacterError
        );
    }

    // -------------------------------------------------------------------
    // createElement — CustomElementState derivation (DOM §4.9 step 6.3)
    // -------------------------------------------------------------------

    use crate::test_util::entity_of as created_entity;

    #[test]
    fn create_element_autonomous_gets_undefined_ce_state() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let r = CreateElement
            .invoke(
                doc,
                &[JsValue::String("my-x".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let entity = created_entity(&r, &session);
        let ce = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
            .expect("autonomous custom element marked at creation");
        assert_eq!(ce.state, elidex_custom_elements::CEState::Undefined);
        assert_eq!(ce.definition_name, "my-x");
    }

    #[test]
    fn create_element_plain_builtin_unmarked() {
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let r = CreateElement
            .invoke(
                doc,
                &[JsValue::String("div".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let entity = created_entity(&r, &session);
        assert!(dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
            .is_err());
    }

    #[test]
    fn create_element_with_is_marks_customized_builtin_no_attr() {
        // The is value lands in the component only — DOM §4.5
        // createElement has no step that sets an `is` content
        // attribute (serialization compensates per HTML §13.3).
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let r = CreateElement
            .invoke(
                doc,
                &[
                    JsValue::String("button".into()),
                    JsValue::String("my-btn".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let entity = created_entity(&r, &session);
        let ce = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
            .expect("customized built-in marked at creation");
        assert_eq!(ce.definition_name, "my-btn");
        assert_eq!(dom.get_attribute(entity, "is"), None);
    }

    #[test]
    fn create_element_invalid_is_still_marks_undefined() {
        // Validity-free per step 6.3 — a host-side validity gate
        // filtering before the handler would turn this red.
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let r = CreateElement
            .invoke(
                doc,
                &[
                    JsValue::String("button".into()),
                    JsValue::String("notvalid".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let entity = created_entity(&r, &session);
        let ce = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
            .expect("non-null invalid is still marks Undefined");
        assert_eq!(ce.definition_name, "notvalid");
        assert_eq!(dom.get_attribute(entity, "is"), None);
    }

    #[test]
    fn create_element_case_folds_before_derivation() {
        // The folded local name feeds BOTH the TagType and the CE
        // derivation — `createElement('MY-EL')` must mark.
        let (mut dom, doc) = setup_dom();
        let mut session = SessionCore::new();
        let r = CreateElement
            .invoke(
                doc,
                &[JsValue::String("MY-EL".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        let entity = created_entity(&r, &session);
        let ce = dom
            .world()
            .get::<&elidex_custom_elements::CustomElementState>(entity)
            .expect("case-folded tag marks CE state");
        assert_eq!(ce.definition_name, "my-el");
    }
}
