//! `HTMLFormElement.prototype` intrinsic — per-tag prototype layer
//! for `<form>` wrappers (HTML §4.10.3).
//!
//! Chain (slot #11-tags-T1 Phase 4):
//!
//! ```text
//! form wrapper
//!   → HTMLFormElement.prototype  (this intrinsic)
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! Members installed here:
//!
//! - **Reflected attrs** (DOMString reflect unless noted):
//!   `acceptCharset` (`accept-charset` content attribute) /
//!   `action` / `autocomplete` / `enctype` / `encoding` (alias of
//!   enctype) / `method` / `name` / `noValidate` (boolean reflect of
//!   `novalidate`) / `target` / `rel`.
//! - **`length`** getter — equal to `elements.length`.
//! - **`elements`** getter — live HTMLFormControlsCollection scoped
//!   to the form's descendants.  Reuses the FCC machinery added in
//!   Phase 3 (cache opt-out per
//!   [`super::dom_collection::LiveCollectionKind::is_cacheable`]).
//! - **`reset()`** — currently a no-op.  Per HTML §4.10.21.2, the
//!   full algorithm fires a cancelable `reset` event then resets
//!   each form control's value.  Both halves are deferred:
//!   - **`reset` event dispatch** → slot
//!     `#11-tags-T1-followup-reset-event`.  Firing a script-visible
//!     event from a native requires the
//!     [`super::event_target_dispatch::dispatch_script_event`]
//!     pipeline + a `DispatchEvent` payload + a manual
//!     `create_event_object` allocation, none of which are wired
//!     up for any other prototype yet.  Trigger: a real-world site
//!     or WPT test relies on the reset event.
//!   - **FormControlState reset** → slot `#11c-followup-reset-form`
//!     (depends on the `elidex-form` Cargo dep landing with #11c —
//!     `elidex_form::reset_form` is the helper invoked there).
//! - **`checkValidity()`** / **`reportValidity()`** — currently
//!   return `true`.  Per HTML §4.10.18.5 the proper walk visits each
//!   listed form control and dispatches an `invalid` event on
//!   failures.  Full implementation lands in **Phase 9** alongside
//!   the ValidityState exposure + ConstraintValidation mixin (plan
//!   §B "ValidityState + ConstraintValidation").
//! - **`submit()`** / **`requestSubmit(submitter?)`** — throw
//!   `DOMException("NotSupportedError")`.  The full form-submission
//!   algorithm (HTML §4.10.21.3) is deferred to slot
//!   **#11-form-submission** (plan §F-2).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{Entity, NodeKind};

const INTERFACE: &str = "HTMLFormElement";

impl VmInner {
    /// Allocate `HTMLFormElement.prototype`.  Must run after
    /// `register_html_element_prototype`.
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

        // 9 string-reflect IDL properties.  Each pair points at a
        // dedicated getter/setter native; `encoding` aliases
        // `enctype` (reads/writes the same `enctype` content
        // attribute) but is installed under its own IDL property
        // name with a dedicated function pair so the dispatch table
        // is direct (no name → attr → fn indirection).
        for &(prop_sid, getter, setter) in &[
            (
                self.well_known.accept_charset,
                form_get_accept_charset as super::super::NativeFn,
                form_set_accept_charset as super::super::NativeFn,
            ),
            (
                self.well_known.action_attr,
                form_get_action,
                form_set_action,
            ),
            (
                self.well_known.autocomplete_attr,
                form_get_autocomplete,
                form_set_autocomplete,
            ),
            (
                self.well_known.enctype_attr,
                form_get_enctype,
                form_set_enctype,
            ),
            (
                self.well_known.encoding_attr,
                form_get_encoding,
                form_set_encoding,
            ),
            (
                self.well_known.method_attr,
                form_get_method,
                form_set_method,
            ),
            (self.well_known.name, form_get_name, form_set_name),
            (
                self.well_known.target_attr,
                form_get_target,
                form_set_target,
            ),
            (self.well_known.rel_attr, form_get_rel, form_set_rel),
        ] {
            self.install_accessor_pair(
                proto_id,
                prop_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // noValidate — boolean reflect of `novalidate` content attr.
        self.install_accessor_pair(
            proto_id,
            self.well_known.no_validate,
            native_form_get_no_validate,
            Some(native_form_set_no_validate),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // length / elements — read-only.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_form_get_length,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.elements_attr,
            native_form_get_elements,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // reset / checkValidity / reportValidity / submit / requestSubmit
        for &(name_sid, native) in &[
            (
                self.well_known.reset_method,
                native_form_reset as super::super::NativeFn,
            ),
            (self.well_known.check_validity, native_form_check_validity),
            (self.well_known.report_validity, native_form_report_validity),
            (self.well_known.submit_method, native_form_submit),
            (self.well_known.request_submit, native_form_request_submit),
        ] {
            self.install_native_method(proto_id, name_sid, native, shape::PropertyAttrs::METHOD);
        }
    }
}

fn require_form_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) = super::event_target::require_receiver(ctx, this, INTERFACE, method, |k| {
        k == NodeKind::Element
    })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "form") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// --- String reflect helper — function-pointer pair generated by ---
// the `iframe_string_attr!`-style macro family.  Inlined here for
// the form attrs since the iframe macro is module-private.

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

form_string_attr!(
    form_get_accept_charset,
    form_set_accept_charset,
    "accept-charset",
    "acceptCharset"
);
form_string_attr!(form_get_action, form_set_action, "action", "action");
form_string_attr!(
    form_get_autocomplete,
    form_set_autocomplete,
    "autocomplete",
    "autocomplete"
);
form_string_attr!(form_get_enctype, form_set_enctype, "enctype", "enctype");
// `encoding` IDL property aliases enctype — same backing attribute.
form_string_attr!(form_get_encoding, form_set_encoding, "enctype", "encoding");
form_string_attr!(form_get_method, form_set_method, "method", "method");
form_string_attr!(form_get_name, form_set_name, "name", "name");
form_string_attr!(form_get_target, form_set_target, "target", "target");
form_string_attr!(form_get_rel, form_set_rel, "rel", "rel");

// --- noValidate (boolean reflect) -----------------------------------

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

// --- length / elements -----------------------------------------------

fn native_form_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "length")? else {
        return Ok(JsValue::Number(0.0));
    };
    // `length === elements.length` is the spec relationship (HTML
    // §4.10.3.1 step 4 — "the length attribute must return the
    // number of nodes represented by the elements collection").
    // Single source of truth: route through the same FormControls
    // walker that `elements` uses, ensuring the predicate stays in
    // one place.  The transient wrapper allocation is amortised by
    // GC and bounded by typical-form size (<50 controls).
    let kind = super::dom_collection::LiveCollectionKind::FormControls { scope: entity };
    let coll_id = ctx.vm.alloc_collection(kind);
    let entities = super::dom_collection::resolve_receiver_entities(ctx, coll_id);
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(entities.len() as f64))
}

fn native_form_get_elements(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "elements")? else {
        return Ok(JsValue::Null);
    };
    let kind = super::dom_collection::LiveCollectionKind::FormControls { scope: entity };
    let id = ctx.vm.alloc_collection(kind);
    Ok(JsValue::Object(id))
}

// --- reset / checkValidity / reportValidity (deferrals documented in the module docstring)

fn native_form_reset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _entity = require_form_receiver(ctx, this, "reset")?;
    // Reset event dispatch + FormControlState reset both deferred —
    // see module docstring for slot bindings.
    Ok(JsValue::Undefined)
}

fn native_form_check_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "checkValidity")? else {
        return Ok(JsValue::Boolean(true));
    };
    Ok(JsValue::Boolean(form_statically_validate(ctx, entity)))
}

fn native_form_report_validity(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "reportValidity")? else {
        return Ok(JsValue::Boolean(true));
    };
    // Headless mode — same as checkValidity.  UI popup lands with
    // slot #11-validation-ui.
    Ok(JsValue::Boolean(form_statically_validate(ctx, entity)))
}

/// HTML §4.10.18.5 statically-validate-the-constraints — walk every
/// submittable element whose form owner is `form_entity` and apply
/// the per-control `checkValidity` predicate (Phase 9 approximation:
/// `customError` empty + not exempt via `willValidate`).  Returns
/// `true` iff every submittable control reports valid.
fn form_statically_validate(ctx: &mut NativeContext<'_>, form_entity: Entity) -> bool {
    let kind = super::dom_collection::LiveCollectionKind::FormControls { scope: form_entity };
    let coll_id = ctx.vm.alloc_collection(kind);
    let entities = super::dom_collection::resolve_receiver_entities(ctx, coll_id);
    for e in entities {
        // Submittable subset of listed elements per HTML §4.10.2:
        // button / input / select / textarea (fieldset / output /
        // object are listed but not submittable).
        let is_submittable = ctx.host().dom().with_tag_name(e, |t| {
            t.is_some_and(|s| {
                s.eq_ignore_ascii_case("button")
                    || s.eq_ignore_ascii_case("input")
                    || s.eq_ignore_ascii_case("select")
                    || s.eq_ignore_ascii_case("textarea")
            })
        });
        if !is_submittable {
            continue;
        }
        if !super::validity_state::entity_check_validity(ctx, e) {
            return false;
        }
    }
    true
}

// --- submit / requestSubmit (deferred to slot #11-form-submission) ---

fn native_form_submit(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _entity = require_form_receiver(ctx, this, "submit")?;
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "form.submit() is deferred to slot #11-form-submission \
         (HTML §4.10.21.3 form-submission algorithm + navigation \
         integration not yet implemented)",
    ))
}

fn native_form_request_submit(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_form_receiver(ctx, this, "requestSubmit")? else {
        return Ok(JsValue::Undefined);
    };
    // Spec validation — submitter argument when present must be a
    // submit button descendant of this form (HTML §4.10.21.3 step 1).
    // We perform the check here even though the actual submission
    // path throws below; a TypeError on a bad submitter must beat
    // the NotSupportedError.
    if let Some(submitter_val) = args.first().copied() {
        if !matches!(submitter_val, JsValue::Undefined | JsValue::Null) {
            // Use the shared `entity_from_this` helper which combines
            // the JsValue::Object check + HostObject extraction +
            // bound-VM gate into one call.
            let Some(submitter_entity) = super::event_target::entity_from_this(ctx, submitter_val)
            else {
                return Err(VmError::type_error(
                    "Failed to execute 'requestSubmit' on 'HTMLFormElement': \
                     submitter is not an Element"
                        .to_string(),
                ));
            };
            if !is_submit_button_descendant_of(ctx, submitter_entity, entity) {
                return Err(VmError::type_error(
                    "Failed to execute 'requestSubmit' on 'HTMLFormElement': \
                     The specified element is not a submit button or \
                     is not owned by this form element"
                        .to_string(),
                ));
            }
        }
    }
    Err(VmError::dom_exception(
        ctx.vm.well_known.dom_exc_not_supported_error,
        "form.requestSubmit() is deferred to slot #11-form-submission \
         (HTML §4.10.21.3 form-submission algorithm + navigation \
         integration not yet implemented)",
    ))
}

/// HTML §4.10.21.3 step 1.x — a valid submitter must be:
/// 1. A submit-button-eligible element (button[type=submit] /
///    input[type=submit] / input[type=image]); and
/// 2. Form-associated with `form` (form attribute IDREF or ancestor
///    walk).
fn is_submit_button_descendant_of(
    ctx: &mut NativeContext<'_>,
    submitter: Entity,
    form: Entity,
) -> bool {
    let dom = ctx.host().dom();
    // `eq_ignore_ascii_case` against the static tag literals avoids
    // the per-call `to_ascii_lowercase` allocation that an explicit
    // owned-String round-trip would incur.
    let is_button = dom.with_tag_name(submitter, |t| {
        t.is_some_and(|s| s.eq_ignore_ascii_case("button"))
    });
    let is_input = dom.with_tag_name(submitter, |t| {
        t.is_some_and(|s| s.eq_ignore_ascii_case("input"))
    });
    let is_submit_button = if is_button {
        // <button type=submit> or unspecified type (default is submit).
        dom.with_attribute(submitter, "type", |v| match v {
            None => true,
            Some(s) => s.eq_ignore_ascii_case("submit"),
        })
    } else if is_input {
        dom.with_attribute(submitter, "type", |v| {
            v.is_some_and(|s| s.eq_ignore_ascii_case("submit") || s.eq_ignore_ascii_case("image"))
        })
    } else {
        false
    };
    if !is_submit_button {
        return false;
    }
    let owner = super::form_assoc::resolve_form_association(ctx, submitter);
    owner == Some(form)
}
