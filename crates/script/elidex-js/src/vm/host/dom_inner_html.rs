//! `Element` / `ShadowRoot` `innerHTML`, `outerHTML`, `setHTMLUnsafe`,
//! and `getHTML` natives (WHATWG HTML §4.4.5 / §4.4.6 / §4.4.7).
//!
//! The four members shared between Element and ShadowRoot routes through
//! shared `*_for` helpers parameterised by a brand-check function
//! pointer; only Element exposes `outerHTML` (per spec ShadowRoot has
//! none — it is a DocumentFragment-rooted tree without a containing
//! element to serialize).  The actual mutation / serialization is
//! implemented engine-indep in [`elidex_script_session`] /
//! [`elidex_dom_api`] — VM bodies here perform brand-checks, argument
//! coercion, and the borrow choreography between `HostData`, the DOM,
//! and the string pool.

#![cfg(feature = "engine")]

use std::collections::HashSet;

use elidex_dom_api::{
    serialize_inner_html, serialize_inner_html_with_options, serialize_outer_html, SerializeOptions,
};
use elidex_ecs::Entity;
use elidex_script_session::{apply_set_inner_html, apply_set_outer_html, SetInnerHtmlOptions};

use super::super::value::{JsValue, NativeContext, ObjectKind, VmError};
use super::super::VmInner;

/// Brand-check predicate for the shared `*_for` helpers. `fn` pointer
/// rather than a generic so the four flavours (Element/ShadowRoot ×
/// inner/setHTMLUnsafe/getHTML) share one monomorphisation.
type BrandCheck = fn(&VmInner, Entity) -> bool;

fn brand_element(vm: &VmInner, entity: Entity) -> bool {
    vm.host_data
        .as_deref()
        .is_some_and(|hd| hd.is_element_entity(entity))
}

fn brand_shadow_root(vm: &VmInner, entity: Entity) -> bool {
    vm.host_data
        .as_deref()
        .is_some_and(|hd| hd.is_shadow_root_entity(entity))
}

/// WebIDL branded-receiver gate.
///
/// Returns `Ok(None)` only for the elidex unbound-receiver case — the
/// receiver is a `HostObject` wrapper but the VM has been `unbind`'d,
/// so `entity_from_this` declines to decode entity bits (silent no-op
/// matches retained-wrapper policy elsewhere). Throws TypeError for
/// non-wrapper receivers ("Illegal invocation"), for wrappers whose
/// backing entity has been destroyed in the live DOM ("the node is
/// detached (invalid entity)"), and for wrappers whose brand does not
/// match ("Illegal invocation"). The detached / wrong-brand split
/// mirrors [`event_target::require_receiver`] so debugger messages
/// line up across the receiver-helper surface.
// Argument order matches the WebIDL error-message form `Failed to
// execute '<accessor>' on '<interface>'`: accessor (the member name
// being invoked) comes first, interface (the receiver brand) second —
// so every call site reads naturally as `require_brand(ctx, this,
// "innerHTML", "Element", check)`. The earlier draft had these
// parameters swapped, which produced TypeError messages reading
// "Failed to execute 'Element' on 'innerHTML'" (PR201 Copilot R2).
fn require_brand(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    accessor: &'static str,
    interface: &'static str,
    check: BrandCheck,
) -> Result<Option<Entity>, VmError> {
    if !super::event_target::this_is_node_wrapper(ctx.vm, this) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{accessor}' on '{interface}': Illegal invocation"
        )));
    }
    let Some(entity) = super::event_target::entity_from_this(ctx, this) else {
        return Ok(None);
    };
    // Differentiate destroyed entity from wrong-brand receiver so the
    // surfaced error message matches the actual failure mode.
    let is_live = ctx
        .host_if_bound()
        .is_some_and(|hd| hd.dom().contains(entity));
    if !is_live {
        return Err(VmError::type_error(format!(
            "Failed to execute '{accessor}' on '{interface}': \
             the node is detached (invalid entity)."
        )));
    }
    if !check(ctx.vm, entity) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{accessor}' on '{interface}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// innerHTML getter (shared between Element and ShadowRoot)
// ---------------------------------------------------------------------------

fn get_inner_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    interface: &'static str,
    check: BrandCheck,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_brand(ctx, this, "innerHTML", interface, check)? else {
        let empty = ctx.vm.strings.intern("");
        return Ok(JsValue::String(empty));
    };
    let Some((dom, strings)) = ctx.dom_and_strings_if_bound() else {
        let empty = ctx.vm.strings.intern("");
        return Ok(JsValue::String(empty));
    };
    let html = serialize_inner_html(entity, dom);
    let sid = strings.intern(&html);
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// innerHTML / setHTMLUnsafe setter (shared)
// ---------------------------------------------------------------------------

fn set_inner_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    interface: &'static str,
    accessor: &'static str,
    opts: SetInnerHtmlOptions,
    check: BrandCheck,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_brand(ctx, this, accessor, interface, check)? else {
        return Ok(JsValue::Undefined);
    };
    let raw_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let html_sid = super::super::coerce::to_string(ctx.vm, raw_arg)?;
    let html = ctx.vm.strings.get_utf8(html_sid);
    let host_data = ctx
        .vm
        .host_data
        .as_deref_mut()
        .expect("bound by require_brand");
    let record = host_data
        .with_session_and_dom(|_session, dom| apply_set_inner_html(dom, entity, &html, opts));
    if let Some(rec) = record {
        ctx.vm.deliver_mutation_records(&[rec]);
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// getHTML (shared) — WHATWG HTML §4.4.6
// ---------------------------------------------------------------------------

/// Parsed `GetHTMLOptions` dictionary. The spec defines two fields:
/// `serializableShadowRoots` (default false) and `shadowRoots`
/// (default empty sequence). Missing / undefined `options` argument
/// produces the defaults.
struct GetHtmlOptions {
    serializable: bool,
    explicit: HashSet<Entity>,
}

impl GetHtmlOptions {
    fn defaults() -> Self {
        Self {
            serializable: false,
            explicit: HashSet::new(),
        }
    }
}

fn parse_get_html_options(
    ctx: &mut NativeContext<'_>,
    raw: JsValue,
    interface: &'static str,
) -> Result<GetHtmlOptions, VmError> {
    if matches!(raw, JsValue::Undefined | JsValue::Null) {
        return Ok(GetHtmlOptions::defaults());
    }
    // WebIDL §3.10.16 dictionary conversion: non-Object / non-null /
    // non-undefined argument throws TypeError. Mirrors the
    // `attachShadow` init-dict precedent at `element_shadow.rs::parse_shadow_init`.
    let JsValue::Object(opts_id) = raw else {
        return Err(VmError::type_error(format!(
            "Failed to execute 'getHTML' on '{interface}': \
             parameter 1 is not of type 'GetHTMLOptions'."
        )));
    };
    // `serializableShadowRoots` — WebIDL boolean (ToBoolean coercion).
    let sid_serializable = ctx.vm.strings.intern("serializableShadowRoots");
    let key_serializable = super::super::value::PropertyKey::String(sid_serializable);
    let raw_serializable = ctx.vm.get_property_value(opts_id, key_serializable)?;
    let serializable = if matches!(raw_serializable, JsValue::Undefined) {
        false
    } else {
        super::super::coerce::to_boolean(ctx.vm, raw_serializable)
    };
    // `shadowRoots` — WebIDL sequence<ShadowRoot>; an array-like with
    // `length` indexed by integer property names.
    let sid_shadow_roots = ctx.vm.strings.intern("shadowRoots");
    let key_shadow_roots = super::super::value::PropertyKey::String(sid_shadow_roots);
    let raw_shadow_roots = ctx.vm.get_property_value(opts_id, key_shadow_roots)?;
    let explicit = if matches!(raw_shadow_roots, JsValue::Undefined | JsValue::Null) {
        HashSet::new()
    } else {
        parse_shadow_root_sequence(ctx, raw_shadow_roots, interface)?
    };
    Ok(GetHtmlOptions {
        serializable,
        explicit,
    })
}

fn parse_shadow_root_sequence(
    ctx: &mut NativeContext<'_>,
    raw: JsValue,
    interface: &'static str,
) -> Result<HashSet<Entity>, VmError> {
    let JsValue::Object(seq_id) = raw else {
        return Err(VmError::type_error(format!(
            "Failed to execute 'getHTML' on '{interface}': \
             'shadowRoots' is not iterable."
        )));
    };
    // Mirror `natives_function.rs::collect_array_like`: dense
    // `Array { elements }` storage does not surface through string
    // property keys, so probe the variant first and fall back to the
    // `length`-driven indexed walk only for ordinary array-like
    // objects (e.g. NodeList wrappers).
    let elements: Vec<JsValue> =
        if let ObjectKind::Array { elements } = &ctx.vm.get_object(seq_id).kind {
            elements.iter().map(|v| v.or_undefined()).collect()
        } else {
            let length_key = super::super::value::PropertyKey::String(ctx.vm.well_known.length);
            let length_val = ctx.vm.get_property_value(seq_id, length_key)?;
            let length = super::super::coerce::to_uint32(ctx.vm, length_val)? as usize;
            let mut out = Vec::with_capacity(length);
            for i in 0..length {
                let index_sid = ctx.vm.strings.intern(&i.to_string());
                let index_key = super::super::value::PropertyKey::String(index_sid);
                out.push(ctx.vm.get_property_value(seq_id, index_key)?);
            }
            out
        };
    let mut out = HashSet::with_capacity(elements.len());
    for (i, elem) in elements.into_iter().enumerate() {
        let JsValue::Object(obj_id) = elem else {
            return Err(VmError::type_error(format!(
                "Failed to execute 'getHTML' on '{interface}': \
                 'shadowRoots[{i}]' is not a ShadowRoot."
            )));
        };
        let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(obj_id).kind else {
            return Err(VmError::type_error(format!(
                "Failed to execute 'getHTML' on '{interface}': \
                 'shadowRoots[{i}]' is not a ShadowRoot."
            )));
        };
        let Some(entity) = Entity::from_bits(entity_bits) else {
            return Err(VmError::type_error(format!(
                "Failed to execute 'getHTML' on '{interface}': \
                 'shadowRoots[{i}]' is detached (invalid entity)."
            )));
        };
        // Separate "wrapper points at a destroyed entity" from "wrong
        // brand" so the error message matches the actual failure mode
        // — same split `require_brand` performs above and
        // `event_target::require_receiver` performs for receivers.
        let is_live = ctx
            .host_if_bound()
            .is_some_and(|hd| hd.dom().contains(entity));
        if !is_live {
            return Err(VmError::type_error(format!(
                "Failed to execute 'getHTML' on '{interface}': \
                 'shadowRoots[{i}]' is detached (invalid entity)."
            )));
        }
        if !super::event_target::is_shadow_root_entity(ctx.vm, entity) {
            return Err(VmError::type_error(format!(
                "Failed to execute 'getHTML' on '{interface}': \
                 'shadowRoots[{i}]' is not a ShadowRoot."
            )));
        }
        out.insert(entity);
    }
    Ok(out)
}

fn get_html_for(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    interface: &'static str,
    check: BrandCheck,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_brand(ctx, this, "getHTML", interface, check)? else {
        let empty = ctx.vm.strings.intern("");
        return Ok(JsValue::String(empty));
    };
    let raw_opts = args.first().copied().unwrap_or(JsValue::Undefined);
    let opts = parse_get_html_options(ctx, raw_opts, interface)?;
    let serialize_opts = SerializeOptions {
        serializable_shadow_roots: opts.serializable,
        explicit_shadow_roots: opts.explicit,
    };
    let Some((dom, strings)) = ctx.dom_and_strings_if_bound() else {
        let empty = ctx.vm.strings.intern("");
        return Ok(JsValue::String(empty));
    };
    let html = serialize_inner_html_with_options(entity, dom, &serialize_opts);
    let sid = strings.intern(&html);
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// Element-only natives — outerHTML getter / setter
// ---------------------------------------------------------------------------

pub(super) fn native_element_get_outer_html(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_brand(ctx, this, "outerHTML", "Element", brand_element)? else {
        let empty = ctx.vm.strings.intern("");
        return Ok(JsValue::String(empty));
    };
    let Some((dom, strings)) = ctx.dom_and_strings_if_bound() else {
        let empty = ctx.vm.strings.intern("");
        return Ok(JsValue::String(empty));
    };
    let html = serialize_outer_html(entity, dom);
    let sid = strings.intern(&html);
    Ok(JsValue::String(sid))
}

pub(super) fn native_element_set_outer_html(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_brand(ctx, this, "outerHTML", "Element", brand_element)? else {
        return Ok(JsValue::Undefined);
    };
    let raw_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let html_sid = super::super::coerce::to_string(ctx.vm, raw_arg)?;
    let html = ctx.vm.strings.get_utf8(html_sid);
    let host_data = ctx
        .vm
        .host_data
        .as_deref_mut()
        .expect("bound by require_brand");
    let result =
        host_data.with_session_and_dom(|_session, dom| apply_set_outer_html(dom, entity, &html));
    match result {
        Ok(rec) => {
            ctx.vm.deliver_mutation_records(&[rec]);
            Ok(JsValue::Undefined)
        }
        // Only variant today; the enum is `#[non_exhaustive]` so this
        // matches future spec-derived rejections (e.g. fragment parse
        // errors) into the same DOMException slot until they earn
        // their own message.
        Err(_) => Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_no_modification_allowed_error,
            "Failed to set 'outerHTML' on 'Element': \
             This element has no parent, or its parent is the Document."
                .to_string(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Element natives — thin wrappers over the shared helpers
// ---------------------------------------------------------------------------

pub(super) fn native_element_get_inner_html(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    get_inner_html_for(ctx, this, "Element", brand_element)
}

pub(super) fn native_element_set_inner_html(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_inner_html_for(
        ctx,
        this,
        args,
        "Element",
        "innerHTML",
        SetInnerHtmlOptions::default(),
        brand_element,
    )
}

pub(super) fn native_element_set_html_unsafe(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_inner_html_for(
        ctx,
        this,
        args,
        "Element",
        "setHTMLUnsafe",
        SetInnerHtmlOptions {
            allow_declarative_shadow: true,
        },
        brand_element,
    )
}

pub(super) fn native_element_get_html(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    get_html_for(ctx, this, args, "Element", brand_element)
}

// ---------------------------------------------------------------------------
// ShadowRoot natives — thin wrappers over the shared helpers
// ---------------------------------------------------------------------------

pub(super) fn native_shadow_root_get_inner_html(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    get_inner_html_for(ctx, this, "ShadowRoot", brand_shadow_root)
}

pub(super) fn native_shadow_root_set_inner_html(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_inner_html_for(
        ctx,
        this,
        args,
        "ShadowRoot",
        "innerHTML",
        SetInnerHtmlOptions::default(),
        brand_shadow_root,
    )
}

pub(super) fn native_shadow_root_set_html_unsafe(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    set_inner_html_for(
        ctx,
        this,
        args,
        "ShadowRoot",
        "setHTMLUnsafe",
        SetInnerHtmlOptions {
            allow_declarative_shadow: true,
        },
        brand_shadow_root,
    )
}

pub(super) fn native_shadow_root_get_html(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    get_html_for(ctx, this, args, "ShadowRoot", brand_shadow_root)
}
