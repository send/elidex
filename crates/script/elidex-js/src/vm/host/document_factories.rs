//! `document.createElement` / `createTextNode` / `createComment` /
//! `createDocumentFragment` natives (WHATWG DOM §4.5 Interface Document
//! factory methods).  Pure-move split from `document.rs` (C0 D-14 (b));
//! the method table stays in `document.rs`.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, VmError};
use super::document::document_receiver;
use super::dom_bridge::{coerce_first_arg_to_string_id, invoke_dom_api};

// ---------------------------------------------------------------------------
// createElement / createTextNode
// ---------------------------------------------------------------------------
//
// Unbound callers get `null` (not a throw) to match the no-op
// behaviour of `getElementById` / `body` / `head` etc.  In practice
// the only way to reach these natives while unbound is to retain a
// `document` reference across an `unbind()` boundary; throwing
// TypeError would crash the listener instead of producing the same
// "document is detached" semantics that other methods already surface.

pub(super) fn native_document_create_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc_entity) = document_receiver(ctx, this, "createElement")? else {
        return Ok(JsValue::Null);
    };
    // WebIDL ARGUMENT CONVERSION first (ToString of the tag, then the
    // options union — conversion TypeErrors precede every method
    // step), then DOM §4.5 method step 1 (localName validity →
    // InvalidCharacterError), then step 3 flatten (conflict / foreign
    // NotSupportedErrors). The handler re-validates and lowercases;
    // the pre-check here exists purely for the spec-mandated error
    // ORDER (shared predicate:
    // `elidex_dom_api::document::is_valid_element_tag_name`).
    let tag_sid = coerce_first_arg_to_string_id(ctx, args)?;
    let converted =
        super::custom_elements::creation::convert_create_element_options(ctx, args.get(1))?;
    let tag = ctx.vm.strings.get_utf8(tag_sid);
    if !elidex_dom_api::document::is_valid_element_tag_name(&tag) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_invalid_character_error,
            format!("Failed to execute 'createElement' on 'Document': Invalid tag name: {tag}"),
        ));
    }
    let flattened = super::custom_elements::creation::flatten_converted_options(ctx, converted)?;
    let mut handler_args = vec![JsValue::String(tag_sid)];
    if let Some(is_sid) = flattened.is {
        handler_args.push(JsValue::String(is_sid));
    } else if flattened.null_registry {
        // `Null` = explicit `customElementRegistry: null` (mutually
        // exclusive with `is` per flatten step 3.2.1) — the handler
        // marks the created element's `RegistryAssociation::Null`.
        handler_args.push(JsValue::Null);
    }
    let result = invoke_dom_api(ctx, "createElement", doc_entity, &handler_args)?;
    // Post-handler hook: ask `elidex_form::create_form_control_state`
    // to attach a `FormControlState` component if the just-created
    // element is a form-control tag (`<input>`, `<button>`,
    // `<textarea>`, `<select>`, `<output>`, `<meter>`, `<progress>`).
    // The helper inspects TagType / Attributes itself and returns
    // without inserting for non-form-control tags, so this call is
    // O(tag-lookup) on the no-op path; we don't pre-filter here to
    // keep the tag list canonically owned by elidex-form.
    // Marshalling-layer wiring — the algorithm itself lives in
    // `elidex_form::create_form_control_state`.  Necessary so
    // JS-created form controls behave correctly under label
    // association, validation, Selection API, and form reset.
    if let JsValue::Object(obj_id) = result {
        if let crate::vm::value::ObjectKind::HostObject { entity_bits } =
            ctx.vm.get_object(obj_id).kind
        {
            if let Some(entity) = elidex_ecs::Entity::from_bits(entity_bits) {
                let _ = elidex_form::create_form_control_state(ctx.host().dom(), entity);
                // D-17 `#11-custom-elements-vm` — per-VM upgrade
                // routing for the `CustomElementState` the engine-
                // indep createElement handler just derived (DOM §4.9
                // "create an element" step 6.3 via
                // `CustomElementState::for_created_element`): an
                // Upgrade reaction fires if the matching definition
                // is already registered, else the entity is queued
                // pending the next `customElements.define()`.
                super::custom_elements::creation::route_custom_element_upgrade(ctx, entity);
            }
        }
    }
    Ok(result)
}

pub(super) fn native_document_create_text_node(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc_entity) = document_receiver(ctx, this, "createTextNode")? else {
        return Ok(JsValue::Null);
    };
    let data_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(
        ctx,
        "createTextNode",
        doc_entity,
        &[JsValue::String(data_sid)],
    )
}

/// `document.createComment(data)` — WHATWG DOM §4.5.  Allocates a
/// Comment entity with the coerced `data` string and returns its
/// wrapper.  Unbound receiver → `null` (matches the rest of the
/// document method family).
pub(super) fn native_document_create_comment(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc_entity) = document_receiver(ctx, this, "createComment")? else {
        return Ok(JsValue::Null);
    };
    let data_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(
        ctx,
        "createComment",
        doc_entity,
        &[JsValue::String(data_sid)],
    )
}

/// `document.createDocumentFragment()` — WHATWG DOM §4.5.  Allocates
/// a `DocumentFragment` entity that is **not** linked into any tree
/// and returns its wrapper.  Unbound / non-Document receiver → `null`.
pub(super) fn native_document_create_document_fragment(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Null);
    }
    let Some(doc_entity) = document_receiver(ctx, this, "createDocumentFragment")? else {
        return Ok(JsValue::Null);
    };
    invoke_dom_api(ctx, "createDocumentFragment", doc_entity, &[])
}
