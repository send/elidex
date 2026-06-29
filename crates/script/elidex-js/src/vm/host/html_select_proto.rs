//! `HTMLSelectElement.prototype` intrinsic — per-tag prototype layer
//! for `<select>` wrappers (HTML §4.10.7).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate".  Form association resolves
//! through [`elidex_form::find_form_ancestor`].  The options live
//! collection is backed by
//! [`elidex_dom_api::CollectionFilter::Options`].
//!
//! ## Members installed
//!
//! Reflected DOMString attrs: `name`, `autocomplete`.
//! Reflected boolean attrs: `disabled`, `multiple`, `required`,
//! `autofocus`.
//! Reflected long: `size` (default 0).
//!
//! Read-only:
//! - `type` returns `"select-multiple"` if `multiple` else
//!   `"select-one"` (HTML §4.10.7).
//! - `form` resolves via `find_form_ancestor`.
//! - `labels` empty NodeList stub (label collection algorithm
//!   pending elidex-form `collect_labels_for` exposure).
//! - `options` returns an `HTMLCollection` filtered by
//!   `CollectionFilter::Options` (live across DOM mutation).
//! - `length` mirrors `options.length`.
//! - `selectedOptions` live `HTMLCollection` of currently-selected
//!   `<option>`s, backed by `CollectionFilter::SelectedOptions`.
//! - `selectedIndex` / `value` reflect the current selection.
//!
//! Methods:
//! - `add(opt, before?)` / `remove(idx?)` — thin marshalling
//!   dispatchers (B1.2b-2-select convergence): brand-check the
//!   receiver + WebIDL unions, then route to the engine-independent
//!   `options.add` / `options.remove` dom-api handlers (HTML §4.10.7
//!   "act like" §2.6.4.3), which own the algorithm + `MutationRecord`
//!   production.  `remove()` no-arg falls through to the converged
//!   `ChildNode.remove` handler (detach the select itself).
//! - `item(idx)` / `namedItem(name)` — proxy to the options live
//!   collection.

#![cfg(feature = "engine")]
// Cast-sign-loss / cast-truncation: every `as usize` / `as i32` /
// `as i64` cast in this module is preceded by an explicit
// non-negative / fits-in-i32 guard.  Module-wide allow keeps the
// reflected-attr setters readable.
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::{NativeFn, VmInner};

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    #[allow(clippy::too_many_lines)] // Phase 7 install — 5 read-only + 4 mutable accessors + 4 methods, single-purpose.
    pub(in crate::vm) fn register_html_select_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_select_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_select_prototype = Some(proto_id);

        let attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;

        // String reflects.
        for (name_sid, getter, setter) in [
            (
                self.well_known.name,
                native_select_get_name as NativeFn,
                native_select_set_name as NativeFn,
            ),
            (
                self.well_known.autocomplete,
                native_select_get_autocomplete as NativeFn,
                native_select_set_autocomplete as NativeFn,
            ),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // Boolean reflects.
        for (name_sid, getter, setter) in [
            (
                self.well_known.disabled,
                native_select_get_disabled as NativeFn,
                native_select_set_disabled as NativeFn,
            ),
            (
                self.well_known.multiple,
                native_select_get_multiple as NativeFn,
                native_select_set_multiple as NativeFn,
            ),
            (
                self.well_known.required,
                native_select_get_required as NativeFn,
                native_select_set_required as NativeFn,
            ),
            (
                self.well_known.autofocus,
                native_select_get_autofocus as NativeFn,
                native_select_set_autofocus as NativeFn,
            ),
        ] {
            self.install_accessor_pair(proto_id, name_sid, getter, Some(setter), attrs);
        }
        // Long reflect: size.
        self.install_accessor_pair(
            proto_id,
            self.well_known.size_attr,
            native_select_get_size,
            Some(native_select_set_size),
            attrs,
        );
        // Read-only.
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            native_select_get_type,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            native_select_get_form,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.labels,
            native_select_get_labels,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.options,
            native_select_get_options,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_select_get_length,
            None,
            attrs,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selected_options,
            native_select_get_selected_options,
            None,
            attrs,
        );
        // selectedIndex (read-write).
        self.install_accessor_pair(
            proto_id,
            self.well_known.selected_index,
            native_select_get_selected_index,
            Some(native_select_set_selected_index),
            attrs,
        );
        // value (read-write — reflects selected option's value).
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            native_select_get_value,
            Some(native_select_set_value),
            attrs,
        );
        // Methods.
        let m = shape::PropertyAttrs::METHOD;
        self.install_native_method(proto_id, self.well_known.item, native_select_item, m);
        self.install_native_method(
            proto_id,
            self.well_known.named_item,
            native_select_named_item,
            m,
        );
        self.install_native_method(proto_id, self.well_known.add, native_select_add, m);
        // `select.remove(idx)` (HTML §4.10.7 `#dom-select-remove`) overrides
        // `ChildNode.remove()`.  Spec says when called with no args
        // it falls through to ChildNode.remove (detach this element);
        // with a numeric arg it detaches the option at that index.
        self.install_native_method(proto_id, self.well_known.remove, native_select_remove, m);
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

fn require_select_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLSelectElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    if !ctx.host().tag_matches_ascii_case(entity, "select") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLSelectElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// ---------------------------------------------------------------------------
// String / boolean reflect macros
// ---------------------------------------------------------------------------

macro_rules! sel_string_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let empty = ctx.vm.well_known.empty;
            let Some(entity) = require_select_receiver(ctx, this, $label)? else {
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
            let Some(entity) = require_select_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Undefined);
            };
            let val = args.first().copied().unwrap_or(JsValue::Undefined);
            let sid = super::super::coerce::to_string(ctx.vm, val)?;
            let s = ctx.vm.strings.get_utf8(sid);
            super::element_attrs::attr_set(ctx, entity, $attr, &s);
            Ok(JsValue::Undefined)
        }
    };
}

sel_string_attr!(
    native_select_get_name,
    native_select_set_name,
    "name",
    "name"
);
sel_string_attr!(
    native_select_get_autocomplete,
    native_select_set_autocomplete,
    "autocomplete",
    "autocomplete"
);

macro_rules! sel_bool_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_select_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Boolean(false));
            };
            Ok(JsValue::Boolean(
                ctx.host().dom().has_attribute(entity, $attr),
            ))
        }

        fn $set(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_select_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Undefined);
            };
            let val = args.first().copied().unwrap_or(JsValue::Undefined);
            let flag = super::super::coerce::to_boolean(ctx.vm, val);
            if flag {
                super::element_attrs::attr_set(ctx, entity, $attr, "");
            } else {
                super::element_attrs::attr_remove(ctx, entity, $attr);
            }
            Ok(JsValue::Undefined)
        }
    };
}

sel_bool_attr!(
    native_select_get_disabled,
    native_select_set_disabled,
    "disabled",
    "disabled"
);
sel_bool_attr!(
    native_select_get_multiple,
    native_select_set_multiple,
    "multiple",
    "multiple"
);
sel_bool_attr!(
    native_select_get_required,
    native_select_set_required,
    "required",
    "required"
);
sel_bool_attr!(
    native_select_get_autofocus,
    native_select_set_autofocus,
    "autofocus",
    "autofocus"
);

// size — long reflect, default 0 (HTML §4.10.7).  Default rendering
// size is browser-dependent; the IDL value mirrors the attribute.
fn native_select_get_size(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "size")? else {
        return Ok(JsValue::Number(0.0));
    };
    let v = ctx
        .host()
        .dom()
        .get_attribute(entity, "size")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    Ok(JsValue::Number(f64::from(v)))
}

fn native_select_set_size(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "size")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    super::element_attrs::attr_set(ctx, entity, "size", &n.to_string());
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// type / form / labels
// ---------------------------------------------------------------------------

fn native_select_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "type")? else {
        let sid = ctx.vm.strings.intern("select-one");
        return Ok(JsValue::String(sid));
    };
    let multiple = ctx.host().dom().has_attribute(entity, "multiple");
    let canonical = if multiple {
        "select-multiple"
    } else {
        "select-one"
    };
    let sid = ctx.vm.strings.intern(canonical);
    Ok(JsValue::String(sid))
}

fn native_select_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    let form = elidex_form::find_form_ancestor(ctx.host().dom(), entity);
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, form))
}

fn native_select_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_select_receiver(ctx, this, "labels")?;
    Ok(JsValue::Object(
        super::dom_collection::empty_labels_collection(ctx.vm),
    ))
}

// ---------------------------------------------------------------------------
// options / length / selectedOptions / selectedIndex / value
// ---------------------------------------------------------------------------

/// Build a fresh live `Options` collection rooted at `select_entity`.
fn native_select_get_options(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_select_receiver(ctx, this, "options")?;
    // `[SameObject]` cache — return the same collection wrapper
    // across reads.  Sweep tail prunes when the select wrapper dies.
    let id = super::dom_collection::cached_form_collection(
        ctx.vm,
        entity,
        elidex_dom_api::CollectionFilter::Options,
        super::dom_collection::FormCollectionCache::Options,
    );
    Ok(JsValue::Object(id))
}

fn native_select_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "length")? else {
        return Ok(JsValue::Number(0.0));
    };
    let mut coll = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::Options,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let len = coll.length(ctx.host().dom());
    Ok(JsValue::Number(
        u32::try_from(len).unwrap_or(u32::MAX).into(),
    ))
}

fn native_select_get_selected_options(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "selectedOptions")? else {
        let id = ctx
            .vm
            .alloc_collection(elidex_dom_api::LiveCollection::new_snapshot(
                Vec::new(),
                elidex_dom_api::CollectionKind::HtmlCollection,
            ));
        return Ok(JsValue::Object(id));
    };
    // Live `HTMLCollection` of options whose effective selectedness
    // is true per HTML §4.10.10.2 — backed by
    // `CollectionFilter::SelectedOptions`, which expresses the full
    // selectedness algorithm: any explicit `selected` attribute
    // wins, otherwise (non-multiple, display size <= 1) the first
    // non-disabled option is the implicit default.  Multi-selects
    // and listbox selects (size > 1) only surface options with
    // explicit `selected`.  No `[SameObject]` cache: the spec
    // doesn't require identity preservation here, and the wrapper
    // is cheap (filter walks only on length/item access, snapshot
    // via the inclusive-descendants-version cache like other live
    // collections).
    let coll = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::SelectedOptions,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let id = ctx.vm.alloc_collection(coll);
    Ok(JsValue::Object(id))
}

fn native_select_get_selected_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "selectedIndex")? else {
        return Ok(JsValue::Number(-1.0));
    };
    // HTML §4.10.10.2 selectedness fallback hoisted to elidex-form
    // (slot #11-tags-T1-v2-drift-hoist D-3) — vm/host/ retains only
    // brand check + JsValue marshalling.
    let value = elidex_form::select_selected_index(ctx.host().dom(), entity);
    Ok(JsValue::Number(value))
}

fn native_select_set_selected_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "selectedIndex")? else {
        return Ok(JsValue::Undefined);
    };
    select_set_selected_index_impl(ctx, entity, args)
}

/// HTML §4.10.10 `selectedIndex` setter algorithm shared by
/// `HTMLSelectElement.prototype.selectedIndex` and
/// `HTMLOptionsCollection.prototype.selectedIndex`.  The setter
/// itself is hoisted to elidex-form (drift-hoist D-4); this wrapper
/// adds the `attr_wrapper_cache` invalidation step that elidex-form
/// can't perform (it's a VM-bound concern — `getAttributeNode("selected")`
/// identity semantics survive `selectedIndex = N` mutations).
pub(super) fn select_set_selected_index_impl(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    // Snapshot the option list before the mutation so we can
    // invalidate every `(option, "selected")` cache entry afterwards.
    let selected_sid = ctx.vm.strings.intern("selected");
    let affected: Vec<Entity> = {
        let dom = ctx.host().dom();
        let mut opts = elidex_dom_api::LiveCollection::new(
            entity,
            elidex_dom_api::CollectionFilter::Options,
            elidex_dom_api::CollectionKind::HtmlCollection,
        );
        opts.snapshot(dom).to_vec()
    };
    elidex_form::select_set_selected_index(ctx.host().dom(), entity, n);
    for opt in affected {
        ctx.vm.invalidate_attr_cache_entry(opt, selected_sid);
    }
    Ok(JsValue::Undefined)
}

fn native_select_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    // HTML §4.10.10 select.value resolution hoisted to elidex-form
    // (slot #11-tags-T1-v2-drift-hoist D-4).  `intern("")` lands on
    // the same `StringId` as `well_known.empty` per the invariant in
    // `WellKnownStrings::intern_all`, so we always intern unconditionally
    // rather than re-encoding the empty short-circuit at every call site.
    let value = elidex_form::select_get_value(ctx.host().dom(), entity);
    let sid = ctx.vm.strings.intern(&value);
    Ok(JsValue::String(sid))
}

fn native_select_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let target_sid = super::super::coerce::to_string(ctx.vm, val)?;
    let target = ctx.vm.strings.get_utf8(target_sid);
    // HTML §4.10.7.4 value setter hoisted to elidex-form
    // (slot #11-tags-T1-v2-drift-hoist D-4).  Pre-PR semantics: only
    // options whose `selected` attribute is *removed* by the setter
    // need their `attr_wrapper_cache` entry invalidated; the matching
    // option that retains `selected` keeps its `Attr` wrapper identity.
    // Snapshot the options that had `selected` before the mutation,
    // then invalidate only those that have lost it after.
    let selected_sid = ctx.vm.strings.intern("selected");
    let was_selected: Vec<Entity> = {
        let dom = ctx.host().dom();
        let mut opts = elidex_dom_api::LiveCollection::new(
            entity,
            elidex_dom_api::CollectionFilter::Options,
            elidex_dom_api::CollectionKind::HtmlCollection,
        );
        opts.snapshot(dom)
            .iter()
            .copied()
            .filter(|opt| {
                dom.world()
                    .get::<&elidex_ecs::Attributes>(*opt)
                    .is_ok_and(|a| a.contains("selected"))
            })
            .collect()
    };
    elidex_form::select_set_value(ctx.host().dom(), entity, &target);
    for opt in was_selected {
        let still_selected = ctx
            .host()
            .dom()
            .world()
            .get::<&elidex_ecs::Attributes>(opt)
            .is_ok_and(|a| a.contains("selected"));
        if !still_selected {
            ctx.vm.invalidate_attr_cache_entry(opt, selected_sid);
        }
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Methods — item / namedItem / add / remove
// ---------------------------------------------------------------------------

fn native_select_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "item")? else {
        return Ok(JsValue::Null);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    let Ok(idx) = usize::try_from(n) else {
        return Ok(JsValue::Null);
    };
    let mut opts = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::Options,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let target = opts.item(idx, ctx.host().dom());
    Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, target))
}

fn native_select_named_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "namedItem")? else {
        return Ok(JsValue::Null);
    };
    let target_str = super::dom_bridge::coerce_first_arg_to_string(ctx, args)?;
    let mut opts = elidex_dom_api::LiveCollection::new(
        entity,
        elidex_dom_api::CollectionFilter::Options,
        elidex_dom_api::CollectionKind::HtmlCollection,
    );
    let snap = opts.snapshot(ctx.host().dom()).to_vec();
    for opt in &snap {
        let id_match =
            ctx.host().dom().get_attribute(*opt, "id").as_deref() == Some(target_str.as_str());
        let name_match =
            ctx.host().dom().get_attribute(*opt, "name").as_deref() == Some(target_str.as_str());
        if id_match || name_match {
            return Ok(super::dom_bridge::wrap_entity_or_null(ctx.vm, Some(*opt)));
        }
    }
    Ok(JsValue::Null)
}

/// `select.add(option, before?)` — HTML §4.10.7 (`#dom-select-add`),
/// which "acts like" `HTMLOptionsCollection.add` (§2.6.4.3).
///
/// Thin marshalling dispatcher (B1.2b-2-select convergence): brand-check the
/// receiver + the `(HTMLOptionElement or HTMLOptGroupElement)` element arg, then
/// normalise the `(HTMLElement or long)?` `before` union and route to the
/// engine-independent `options.add` dom-api handler. The algorithm (validity
/// ordering + `MutationRecord` production) lives in
/// `elidex_dom_api::element::select::OptionsAdd`.
fn native_select_add(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "add")? else {
        return Ok(JsValue::Undefined);
    };
    dispatch_options_add(ctx, entity, args)
}

/// `select.remove(index?)` — HTML §4.10.7 (`#dom-select-remove`).  No-arg /
/// `undefined` form falls through to `ChildNode.remove()` (detach the select
/// itself); a numeric form removes the option at the given index.
///
/// Thin marshalling dispatcher: the no-arg path routes to the converged `remove`
/// (ChildNode) handler (One-issue-one-way — no inline detach duplicate); the
/// numeric path `ToInt32`-coerces and routes to `options.remove`.
fn native_select_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "remove")? else {
        return Ok(JsValue::Undefined);
    };
    // No-arg / undefined: ChildNode.remove() — detach `entity` via the converged
    // record-producing handler (#393), NOT an inline `remove_child` duplicate.
    if args.is_empty() || matches!(args[0], JsValue::Undefined) {
        return super::dom_bridge::invoke_dom_api(ctx, "remove", entity, &[]);
    }
    dispatch_options_remove_index(ctx, entity, args[0])
}

// ---------------------------------------------------------------------------
// Shared marshalling dispatchers (HTMLSelectElement + HTMLOptionsCollection)
// ---------------------------------------------------------------------------
//
// `select.add`/`remove(idx)` and `options.add`/`remove`/`length` route through
// the same engine-independent dom-api handlers (One handler per algorithm, both
// receivers — HTML §4.10.7 "act like" §2.6.4.3). These helpers own only the
// engine-bound marshalling: receiver-resolved `select` entity in, WebIDL union /
// `ToInt32` / `ToUint32` coercion, then `invoke_dom_api`.

/// TypeError for an `add` `element` argument that is not an
/// `HTMLOptionElement`/`HTMLOptGroupElement` wrapper (WebIDL union-conversion
/// failure). Distinct from the detached form so a stale wrapper surfaces as
/// "detached", mirroring `element_insert_adjacent::require_element_arg`.
fn option_arg_type_error() -> VmError {
    VmError::type_error(
        "Failed to execute 'add' on 'HTMLSelectElement': The element provided is not an \
         HTMLOptionElement or HTMLOptGroupElement."
            .to_owned(),
    )
}

/// TypeError for an `add` `element` argument whose backing entity has been
/// destroyed / recycled.
fn option_arg_detached_error() -> VmError {
    VmError::type_error(
        "Failed to execute 'add' on 'HTMLSelectElement': the element provided is detached \
         (invalid entity)."
            .to_owned(),
    )
}

/// WebIDL `(HTMLOptionElement or HTMLOptGroupElement)` union brand-check
/// (marshalling): the `element` arg must be a live `<option>`/`<optgroup>`
/// wrapper. Kept VM-side — like `require_element_arg` (#399) — because (a) it
/// distinguishes a *detached* wrapper (recycled entity) from a *wrong-type* one,
/// a `JsValue`/`ObjectKind`-identity distinction the engine-independent handler
/// cannot make (the bridge's `materialize` rejects a destroyed entity
/// generically before the handler runs), and (b) running the full union check
/// here keeps the WebIDL conversion order correct — a wrong-type `element` throws
/// before `before`'s `ToInt32` side effects. The `options.add` handler re-guards
/// the tag for boa/wasm parity + defense-in-depth.
fn require_option_or_optgroup_arg(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Result<(), VmError> {
    let JsValue::Object(id) = value else {
        return Err(option_arg_type_error());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(option_arg_type_error());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(option_arg_detached_error)?;
    // Stale-entity check before the tag lookup: a destroyed entity has no
    // components, so a tag probe would masquerade as "wrong type".
    if !ctx.host().dom().contains(entity) {
        return Err(option_arg_detached_error());
    }
    let is_member = ctx.host().tag_matches_ascii_case(entity, "option")
        || ctx.host().tag_matches_ascii_case(entity, "optgroup");
    if is_member {
        Ok(())
    } else {
        Err(option_arg_type_error())
    }
}

/// Normalise the `add` `before` argument `(HTMLElement or long)?` to what the
/// `options.add` handler consumes: `Null` (append), the element `Object` (a node
/// reference), or a `Number` (the `ToInt32` index). The element-vs-long
/// discrimination inspects `ObjectKind`/Element identity, so it is engine-bound
/// marshalling and stays VM-side.
fn normalize_before_arg(
    ctx: &mut NativeContext<'_>,
    before_arg: JsValue,
) -> Result<JsValue, VmError> {
    let coerce_long = |ctx: &mut NativeContext<'_>, v: JsValue| -> Result<JsValue, VmError> {
        // ToInt32 (which calls ToNumber first per ECMA-262 §7.1.4); a non-Number
        // Object goes through ToPrimitive → typically NaN → 0.
        Ok(JsValue::Number(f64::from(super::super::coerce::to_int32(
            ctx.vm, v,
        )?)))
    };
    match before_arg {
        JsValue::Null | JsValue::Undefined => Ok(JsValue::Null),
        JsValue::Object(obj_id) => match ctx.vm.get_object(obj_id).kind {
            ObjectKind::HostObject { entity_bits } => {
                // The HostObject enters the HTMLElement arm only when its backing
                // entity is actually an Element node; Text / Comment / Document
                // wrappers fall through to the `long` branch.
                let is_element = Entity::from_bits(entity_bits).is_some_and(|e| {
                    ctx.host().dom().node_kind_inferred(e) == Some(NodeKind::Element)
                });
                if is_element {
                    Ok(before_arg)
                } else {
                    coerce_long(ctx, before_arg)
                }
            }
            // Non-HostObject (plain JS object) falls into the `long` branch.
            _ => coerce_long(ctx, before_arg),
        },
        // Number / String / Boolean / BigInt all coerce through ToInt32.
        _ => coerce_long(ctx, before_arg),
    }
}

/// HTML §2.6.4.3 / §4.10.7 `add(element, before?)` marshalling shared by
/// `HTMLSelectElement.add` and `HTMLOptionsCollection.add`. `select` is the
/// receiver-resolved `<select>` entity.
pub(super) fn dispatch_options_add(
    ctx: &mut NativeContext<'_>,
    select: Entity,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL conversion order: element union first, then before.
    let opt_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    require_option_or_optgroup_arg(ctx, opt_arg)?;
    let before_arg = args.get(1).copied().unwrap_or(JsValue::Null);
    let before_norm = normalize_before_arg(ctx, before_arg)?;
    super::dom_bridge::invoke_dom_api(ctx, "options.add", select, &[opt_arg, before_norm])
}

/// HTML §2.6.4.3 / §4.10.7 `remove(index)` marshalling: `ToInt32`-coerce the
/// index and route to the `options.remove` handler. `select` is the
/// receiver-resolved `<select>` entity.
pub(super) fn dispatch_options_remove_index(
    ctx: &mut NativeContext<'_>,
    select: Entity,
    idx_arg: JsValue,
) -> Result<JsValue, VmError> {
    let n = super::super::coerce::to_int32(ctx.vm, idx_arg)?;
    super::dom_bridge::invoke_dom_api(
        ctx,
        "options.remove",
        select,
        &[JsValue::Number(f64::from(n))],
    )
}

/// HTML §2.6.4.3 `length` setter marshalling: `ToUint32`-coerce the value and
/// route to the `options.length.set` handler. `select` is the receiver-resolved
/// `<select>` entity.
pub(super) fn dispatch_options_set_length(
    ctx: &mut NativeContext<'_>,
    select: Entity,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let value = super::super::coerce::to_uint32(ctx.vm, val)?;
    super::dom_bridge::invoke_dom_api(
        ctx,
        "options.length.set",
        select,
        &[JsValue::Number(f64::from(value))],
    )
}
