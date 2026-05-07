//! `HTMLFormElement.prototype` intrinsic — per-tag prototype layer
//! for `<form>` wrappers (HTML §4.10.3).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", form submission and reset
//! algorithms live in [`elidex_form::submit`] (`reset_form` /
//! `read_form_attrs` / `find_form_ancestor`).  This module reflects
//! content attributes, dispatches the cancelable `reset` event, and
//! delegates the reset side-effect to elidex-form.
//!
//! ## Members installed
//!
//! Reflected DOMString attributes: `action`, `method`, `enctype`,
//! `encoding` (alias of `enctype`), `target`, `name`, `acceptCharset`,
//! `autocomplete`, `rel`.
//!
//! Reflected boolean: `noValidate`.
//!
//! Read-only stubs:
//! - `elements` — empty NodeList snapshot (Phase 4 stub; full
//!   `HTMLFormControlsCollection` lands in Phase 7).
//! - `length` — 0 (matches the empty stub above).
//!
//! Methods:
//! - `submit()` / `requestSubmit()` — `NotSupportedError` stub
//!   (defer slot `#11-form-submission`, navigation infra).
//! - `reset()` — dispatches a cancelable `reset` event then, if not
//!   default-prevented, calls `elidex_form::reset_form` to roll
//!   each form control back to its `default_value` /
//!   `default_checked` (Group β F-7 + F-8 fold).
//! - `checkValidity()` / `reportValidity()` — Phase 9 mixin.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::{NativeFn, VmInner};

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    pub(in crate::vm) fn register_html_form_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_form_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_form_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        // String reflect pairs: (idl_sid, html_attr_name).
        let pairs: [(super::super::StringId, &'static str); 8] = [
            (self.well_known.action, "action"),
            (self.well_known.method_attr, "method"),
            (self.well_known.enctype, "enctype"),
            (self.well_known.target, "target"),
            (self.well_known.name, "name"),
            (self.well_known.accept_charset, "accept-charset"),
            (self.well_known.autocomplete, "autocomplete"),
            (self.well_known.rel, "rel"),
        ];
        for (name_sid, attr_name) in pairs {
            let getter = string_reflect_getter_for(attr_name);
            let setter = string_reflect_setter_for(attr_name);
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // `encoding` is a legacy alias for `enctype` (HTML §4.10.3).
        self.install_accessor_pair(
            proto_id,
            self.well_known.encoding,
            form_get_enctype,
            Some(form_set_enctype),
            attrs,
        );

        // noValidate boolean reflect.
        self.install_accessor_pair(
            proto_id,
            self.well_known.no_validate,
            native_form_get_no_validate,
            Some(native_form_set_no_validate),
            attrs,
        );

        // length / elements — Phase 4 stubs.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_form_get_length,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.elements_attr,
            native_form_get_elements,
            None,
            attrs,
        );

        // Methods.
        let method_attrs = shape::PropertyAttrs::METHOD;
        self.install_native_method(
            proto_id,
            self.well_known.submit_method,
            native_form_submit,
            method_attrs,
        );
        self.install_native_method(
            proto_id,
            self.well_known.request_submit,
            native_form_request_submit,
            method_attrs,
        );
        self.install_native_method(
            proto_id,
            self.well_known.reset_method,
            native_form_reset,
            method_attrs,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_form_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLFormElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "form") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLFormElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String reflect helpers
// ---------------------------------------------------------------------------

fn string_reflect_getter_for(attr_name: &'static str) -> NativeFn {
    match attr_name {
        "action" => form_get_action,
        "method" => form_get_method,
        "enctype" => form_get_enctype,
        "target" => form_get_target,
        "name" => form_get_name,
        "accept-charset" => form_get_accept_charset,
        "autocomplete" => form_get_autocomplete,
        "rel" => form_get_rel,
        _ => unreachable!("string_reflect_getter_for called with unsupported attr {attr_name}"),
    }
}

fn string_reflect_setter_for(attr_name: &'static str) -> NativeFn {
    match attr_name {
        "action" => form_set_action,
        "method" => form_set_method,
        "enctype" => form_set_enctype,
        "target" => form_set_target,
        "name" => form_set_name,
        "accept-charset" => form_set_accept_charset,
        "autocomplete" => form_set_autocomplete,
        "rel" => form_set_rel,
        _ => unreachable!("string_reflect_setter_for called with unsupported attr {attr_name}"),
    }
}

macro_rules! form_string_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let empty = ctx.vm.well_known.empty;
            let Some(entity) = require_form_receiver(ctx, this, $label)? else {
                return Ok(JsValue::String(empty));
            };
            let sid = match ctx.dom_and_strings_if_bound() {
                Some((dom, strings)) => {
                    dom.with_attribute(entity, $attr, |v| v.map_or(empty, |s| strings.intern(s)))
                }
                None => empty,
            };
            Ok(JsValue::String(sid))
        }

        fn $set(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_form_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Undefined);
            };
            let val = args.first().copied().unwrap_or(JsValue::Undefined);
            let sid = super::super::coerce::to_string(ctx.vm, val)?;
            let s = ctx.vm.strings.get_utf8(sid);
            ctx.host().dom().set_attribute(entity, $attr, s);
            Ok(JsValue::Undefined)
        }
    };
}

form_string_attr!(form_get_action, form_set_action, "action", "action");
form_string_attr!(form_get_method, form_set_method, "method", "method");
form_string_attr!(form_get_enctype, form_set_enctype, "enctype", "enctype");
form_string_attr!(form_get_target, form_set_target, "target", "target");
form_string_attr!(form_get_name, form_set_name, "name", "name");
form_string_attr!(
    form_get_accept_charset,
    form_set_accept_charset,
    "accept-charset",
    "acceptCharset"
);
form_string_attr!(
    form_get_autocomplete,
    form_set_autocomplete,
    "autocomplete",
    "autocomplete"
);
form_string_attr!(form_get_rel, form_set_rel, "rel", "rel");

// ---------------------------------------------------------------------------
// noValidate boolean reflect
// ---------------------------------------------------------------------------

fn native_form_get_no_validate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "noValidate")? else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "novalidate"),
    ))
}

fn native_form_set_no_validate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "noValidate")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "novalidate", String::new());
    } else {
        super::element_attrs::attr_remove(ctx, entity, "novalidate");
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// length / elements stubs
// ---------------------------------------------------------------------------

fn native_form_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "length")? else {
        return Ok(JsValue::Number(0.0));
    };
    // HTML §4.10.3: form.length returns the number of listed elements
    // in form.elements.  Build a transient FormControls collection,
    // count, and discard.
    let mut coll = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::FormControls,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let len = coll.length(ctx.host().dom());
    Ok(JsValue::Number(
        u32::try_from(len).unwrap_or(u32::MAX).into(),
    ))
}

fn native_form_get_elements(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "elements")? else {
        let id = ctx
            .vm
            .alloc_collection(elidex_dom_api::LiveCollection::new_snapshot(
                Vec::new(),
                elidex_dom_api::CollectionKind::NodeList,
            ));
        return Ok(JsValue::Object(id));
    };
    let coll = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::FormControls,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let id = ctx.vm.alloc_collection(coll);
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// submit / requestSubmit / reset
// ---------------------------------------------------------------------------

fn native_form_submit(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_form_receiver(ctx, this, "submit")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "form.submit() is not yet supported (slot #11-form-submission)",
    ))
}

fn native_form_request_submit(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_form_receiver(ctx, this, "requestSubmit")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "form.requestSubmit() is not yet supported (slot #11-form-submission)",
    ))
}

/// `form.reset()` — HTML §4.10.21.5.
///
/// Group β F-7 + F-8 fold:
///
/// 1. Construct a cancelable `reset` event (bubbles=true,
///    cancelable=true) at the precomputed core-9 shape.
/// 2. Dispatch through the script-event walk; if a listener calls
///    `preventDefault()` the reset is cancelled per spec.
/// 3. On non-cancelled dispatch call
///    [`elidex_form::reset_form`] to roll each descendant
///    form-control's `FormControlState` back to its
///    `default_value` / `default_checked`.
fn native_form_reset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "reset")? else {
        return Ok(JsValue::Undefined);
    };
    let cancelled = dispatch_reset_event(ctx, entity)?;
    if cancelled {
        return Ok(JsValue::Undefined);
    }
    elidex_form::reset_form(ctx.host().dom(), entity);
    Ok(JsValue::Undefined)
}

/// Construct a `reset` Event and dispatch it on `target_entity`.
/// Returns `true` when the dispatch was cancelled
/// (default-prevented).
fn dispatch_reset_event(
    ctx: &mut NativeContext<'_>,
    target_entity: elidex_ecs::Entity,
) -> Result<bool, VmError> {
    use super::super::value::{ObjectKind, PropertyValue};

    let type_sid = ctx.vm.well_known.reset_event;
    let event_proto = ctx.vm.event_prototype;
    let target_wrapper = ctx.vm.create_element_wrapper(target_entity);
    let core_shape = ctx
        .vm
        .precomputed_event_shapes
        .as_ref()
        .expect("precomputed_event_shapes built during VM init")
        .core;

    // Allocate Event with cancelable=true, bubbles=true.  The
    // freshly-returned event_id is rooted via the
    // `dispatched_events` insert immediately below — the trace
    // step doesn't visit the membership directly, but the Event's
    // shape-resident slots (`target` / `currentTarget`) install
    // before any subsequent allocation can trigger GC, and the
    // dispatched_events sweep tail prevents stale-id retention if
    // dispatch panics.  Same lifetime contract as
    // `super::pending_tasks::deliver_post_message` — the scope
    // between `alloc_object` and the slot install is narrow enough
    // that GC cannot run within it.
    let event_id = ctx.vm.alloc_object(super::super::value::Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable: true,
            passive: false,
            type_sid,
            bubbles: true,
            composed: false,
            composed_path: None,
        },
        storage: super::super::value::PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: event_proto,
        extensible: true,
    });

    let timestamp_ms = ctx.vm.start_instant.elapsed().as_secs_f64() * 1000.0;
    let slots: Vec<PropertyValue> = vec![
        PropertyValue::Data(JsValue::String(type_sid)),
        PropertyValue::Data(JsValue::Boolean(true)), // bubbles
        PropertyValue::Data(JsValue::Boolean(true)), // cancelable
        PropertyValue::Data(JsValue::Number(0.0)),   // eventPhase
        PropertyValue::Data(JsValue::Object(target_wrapper)), // target
        PropertyValue::Data(JsValue::Object(target_wrapper)), // currentTarget
        PropertyValue::Data(JsValue::Number(timestamp_ms)),
        PropertyValue::Data(JsValue::Boolean(false)), // defaultPrevented
        PropertyValue::Data(JsValue::Boolean(false)), // composed
    ];
    ctx.vm
        .define_with_precomputed_shape(event_id, core_shape, slots);

    // Bracket dispatched_events around the walk per the
    // `dispatch_script_event` contract.
    ctx.vm.dispatched_events.insert(event_id);
    let result = super::event_target_dispatch::dispatch_script_event(ctx, event_id, target_entity);
    ctx.vm.dispatched_events.remove(&event_id);

    // dispatch_script_event returns Ok(!default_prevented), so
    // `Ok(false)` means the dispatch was cancelled.
    Ok(matches!(result, Ok(false)))
}
