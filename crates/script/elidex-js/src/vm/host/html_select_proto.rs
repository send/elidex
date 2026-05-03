//! `HTMLSelectElement.prototype` intrinsic — per-tag prototype layer
//! for `<select>` wrappers (HTML §4.10.7 — slot #11-tags-T1 Phase 7).
//!
//! Chain:
//!
//! ```text
//! select wrapper
//!   → HTMLSelectElement.prototype
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! Members installed here:
//!
//! - **Reflected attrs**: `autocomplete` / `disabled` (boolean) /
//!   `multiple` (boolean) / `name` / `required` (boolean) / `size`
//!   (`unsigned long`).
//! - **`type`** — derived getter returning `"select-multiple"` if
//!   the `multiple` content attribute is present, otherwise
//!   `"select-one"` (HTML §4.10.7.1 step 2).
//! - **`length`** — RW alias for `options.length`.  Setter delegates
//!   to `HTMLOptionsCollection.length` setter so the same DOM
//!   mutation routes apply.
//! - **`options`** — HTMLOptionsCollection (`Options { select }`).
//! - **`selectedOptions`** — read-only HTMLCollection (Snapshot
//!   variant) of currently selected options.
//! - **`selectedIndex`** — RW.  Getter returns the index of the
//!   first selected `<option>` or -1.  Setter clears all
//!   `selected` content attributes and sets the one at the new
//!   index.
//! - **`value`** — RW.  Getter returns the first selected option's
//!   value (or "" if none); setter sets the option whose value
//!   matches first.
//! - **`add(opt, before?)`** / **`remove(idx?)`** / **`item(i)`** /
//!   **`namedItem(name)`** — proxy to the underlying
//!   HTMLOptionsCollection.
//! - **`form`** / **`labels`** — derived getters.
//!
//! ConstraintValidation methods (Phase 9) and the popover API stay
//! out of scope here.

#![cfg(feature = "engine")]

use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

const INTERFACE: &str = "HTMLSelectElement";

impl VmInner {
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

        // String reflects.
        for &(prop_sid, getter, setter) in &[
            (
                self.well_known.autocomplete_attr,
                sel_get_autocomplete as super::super::NativeFn,
                sel_set_autocomplete as super::super::NativeFn,
            ),
            (self.well_known.name, sel_get_name, sel_set_name),
        ] {
            self.install_accessor_pair(
                proto_id,
                prop_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // Boolean reflects.
        for &(prop_sid, getter, setter) in &[
            (
                self.well_known.disabled,
                sel_get_disabled as super::super::NativeFn,
                sel_set_disabled as super::super::NativeFn,
            ),
            (
                self.well_known.multiple_attr,
                sel_get_multiple,
                sel_set_multiple,
            ),
            (self.well_known.required, sel_get_required, sel_set_required),
        ] {
            self.install_accessor_pair(
                proto_id,
                prop_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // size — `unsigned long` reflect with default that depends
        // on `multiple` presence (1 for select-one, 4 for
        // select-multiple per HTML §4.10.7.4 step 5).
        self.install_accessor_pair(
            proto_id,
            self.well_known.size_attr,
            sel_get_size,
            Some(sel_set_size),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // type — derived from `multiple` content attribute.
        self.install_accessor_pair(
            proto_id,
            self.well_known.type_attr,
            sel_get_type,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // length — RW.  Reads / writes go through the
        // HTMLOptionsCollection so add()/remove() observers see the
        // same mutation surface.
        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            sel_get_length,
            Some(sel_set_length),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // options / selectedOptions / selectedIndex / value.
        self.install_accessor_pair(
            proto_id,
            self.well_known.options_attr,
            sel_get_options,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selected_options,
            sel_get_selected_options,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.selected_index,
            sel_get_selected_index,
            Some(sel_set_selected_index),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.value,
            sel_get_value,
            Some(sel_set_value),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // form / labels.
        self.install_accessor_pair(
            proto_id,
            self.well_known.form_attr,
            sel_get_form,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_accessor_pair(
            proto_id,
            self.well_known.labels_attr,
            sel_get_labels,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // ConstraintValidation mixin (Phase 9).
        super::validity_state::install_constraint_validation_methods(self, proto_id);

        // add / remove / item / namedItem — proxy to options.
        for &(name_sid, native) in &[
            (
                self.well_known.add_method,
                sel_add as super::super::NativeFn,
            ),
            (self.well_known.remove_method, sel_remove),
            (self.well_known.item, sel_item),
            (self.well_known.named_item, sel_named_item),
        ] {
            self.install_native_method(proto_id, name_sid, native, shape::PropertyAttrs::METHOD);
        }
    }
}

fn require_select_receiver(
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
    if !ctx.host().tag_matches_ascii_case(entity, "select") {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{INTERFACE}': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

// --- String reflects ----------------------------------------------

macro_rules! select_string_attr {
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
            ctx.host().dom().set_attribute(entity, $attr, s);
            Ok(JsValue::Undefined)
        }
    };
}

select_string_attr!(
    sel_get_autocomplete,
    sel_set_autocomplete,
    "autocomplete",
    "autocomplete"
);
select_string_attr!(sel_get_name, sel_set_name, "name", "name");

// --- Boolean reflects ---------------------------------------------

macro_rules! select_bool_attr {
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
                ctx.host().dom().set_attribute(entity, $attr, String::new());
            } else {
                super::element_attrs::attr_remove(ctx, entity, $attr);
            }
            Ok(JsValue::Undefined)
        }
    };
}

select_bool_attr!(sel_get_disabled, sel_set_disabled, "disabled", "disabled");
select_bool_attr!(sel_get_multiple, sel_set_multiple, "multiple", "multiple");
select_bool_attr!(sel_get_required, sel_set_required, "required", "required");

// --- size (unsigned long, default 1 / 4 by `multiple`) ------------

fn sel_get_size(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "size")? else {
        return Ok(JsValue::Number(0.0));
    };
    let dom = ctx.host().dom();
    let size_attr = dom.with_attribute(entity, "size", |v| v.and_then(|s| s.parse::<u32>().ok()));
    let value = match size_attr {
        Some(n) if n > 0 => n,
        _ => {
            if dom.has_attribute(entity, "multiple") {
                4
            } else {
                1
            }
        }
    };
    Ok(JsValue::Number(f64::from(value)))
}

fn sel_set_size(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "size")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_uint32(ctx.vm, val)?;
    ctx.host()
        .dom()
        .set_attribute(entity, "size", n.to_string());
    Ok(JsValue::Undefined)
}

// --- type (derived getter) ----------------------------------------

fn sel_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "type")? else {
        return Ok(JsValue::String(ctx.vm.well_known.select_one_str));
    };
    let sid = if ctx.host().dom().has_attribute(entity, "multiple") {
        ctx.vm.well_known.select_multiple_str
    } else {
        ctx.vm.well_known.select_one_str
    };
    Ok(JsValue::String(sid))
}

// --- length (alias for options.length) ----------------------------

fn collect_options(ctx: &mut NativeContext<'_>, select: Entity) -> Vec<Entity> {
    let dom = ctx.host().dom();
    let mut out = Vec::new();
    dom.traverse_descendants(select, |e| {
        if e == select {
            return true;
        }
        if dom.node_kind_inferred(e) != Some(NodeKind::Element) {
            return true;
        }
        if dom.with_tag_name(e, |t| t.is_some_and(|s| s.eq_ignore_ascii_case("option"))) {
            out.push(e);
        }
        true
    });
    out
}

fn sel_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "length")? else {
        return Ok(JsValue::Number(0.0));
    };
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(collect_options(ctx, entity).len() as f64))
}

fn sel_set_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "length")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let new_len = super::super::coerce::to_uint32(ctx.vm, val)?;
    let current = collect_options(ctx, entity);
    let cur_len = u32::try_from(current.len()).unwrap_or(u32::MAX);
    if new_len < cur_len {
        let dom = ctx.host().dom();
        for &option in current.iter().skip(new_len as usize).rev() {
            if let Some(parent) = dom.get_parent(option) {
                let _ = dom.remove_child(parent, option);
            }
        }
    } else if new_len > cur_len {
        let dom = ctx.host().dom();
        for _ in 0..(new_len - cur_len) {
            let opt = dom.create_element("option", elidex_ecs::Attributes::default());
            let _ = dom.append_child(entity, opt);
        }
    }
    Ok(JsValue::Undefined)
}

// --- options / selectedOptions ------------------------------------

fn sel_get_options(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "options")? else {
        return Ok(JsValue::Null);
    };
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Options { select: entity });
    Ok(JsValue::Object(id))
}

fn sel_get_selected_options(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "selectedOptions")? else {
        return Ok(JsValue::Null);
    };
    let options = collect_options(ctx, entity);
    let dom = ctx.host().dom();
    let selected: Vec<Entity> = options
        .into_iter()
        .filter(|&o| dom.has_attribute(o, "selected"))
        .collect();
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Snapshot {
            entities: selected,
        });
    Ok(JsValue::Object(id))
}

// --- selectedIndex (RW) -------------------------------------------

fn sel_get_selected_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "selectedIndex")? else {
        return Ok(JsValue::Number(-1.0));
    };
    let options = collect_options(ctx, entity);
    let dom = ctx.host().dom();
    for (idx, &option) in options.iter().enumerate() {
        if dom.has_attribute(option, "selected") {
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
            let v = idx as i64 as f64;
            return Ok(JsValue::Number(v));
        }
    }
    Ok(JsValue::Number(-1.0))
}

fn sel_set_selected_index(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "selectedIndex")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    let options = collect_options(ctx, entity);
    // Clear `selected` on every option; then set on the target if
    // in range (HTML §4.10.7.4).
    for &opt in &options {
        super::element_attrs::attr_remove(ctx, opt, "selected");
    }
    if n >= 0 {
        let idx = n as usize;
        if let Some(&opt) = options.get(idx) {
            ctx.host()
                .dom()
                .set_attribute(opt, "selected", String::new());
        }
    }
    Ok(JsValue::Undefined)
}

// --- value (RW: first-selected option's value, or "") -------------

fn sel_get_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let empty = ctx.vm.well_known.empty;
    let Some(entity) = require_select_receiver(ctx, this, "value")? else {
        return Ok(JsValue::String(empty));
    };
    let options = collect_options(ctx, entity);
    for option in options {
        let dom = ctx.host().dom();
        if dom.has_attribute(option, "selected") {
            // Per HTML §4.10.10.4: option.value defaults to its
            // textContent when the `value` content attribute is
            // absent.
            let val = dom.with_attribute(option, "value", |v| v.map(String::from));
            let text = match val {
                Some(s) => s,
                None => option_text_content(ctx, option),
            };
            if text.is_empty() {
                return Ok(JsValue::String(empty));
            }
            let sid = ctx.vm.strings.intern(&text);
            return Ok(JsValue::String(sid));
        }
    }
    Ok(JsValue::String(empty))
}

fn sel_set_value(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "value")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let needle = ctx.vm.strings.get_utf8(sid);
    let options = collect_options(ctx, entity);
    // Clear all selected attrs.
    for &opt in &options {
        super::element_attrs::attr_remove(ctx, opt, "selected");
    }
    // Set selected on first matching option.
    for option in options {
        let matches = {
            let dom = ctx.host().dom();
            let value_attr = dom.with_attribute(option, "value", |v| v.map(String::from));
            match value_attr {
                Some(s) => s == needle,
                None => option_text_content(ctx, option) == needle,
            }
        };
        if matches {
            ctx.host()
                .dom()
                .set_attribute(option, "selected", String::new());
            break;
        }
    }
    Ok(JsValue::Undefined)
}

/// Walk the option's descendant Text data — mirrors HTMLOptionElement
/// text-default-value semantics (HTML §4.10.10.4 step 1).
fn option_text_content(ctx: &mut NativeContext<'_>, option: Entity) -> String {
    let dom = ctx.host().dom();
    let mut buf = String::new();
    dom.traverse_descendants(option, |e| {
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(e) {
            buf.push_str(&text.0);
        }
        true
    });
    buf
}

// --- form / labels ------------------------------------------------

fn sel_get_form(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "form")? else {
        return Ok(JsValue::Null);
    };
    match super::form_assoc::resolve_form_association(ctx, entity) {
        Some(f) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(f))),
        None => Ok(JsValue::Null),
    }
}

fn sel_get_labels(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "labels")? else {
        return Ok(JsValue::Null);
    };
    let labels = super::form_assoc::collect_labels_for(ctx, entity);
    let kind = super::dom_collection::LiveCollectionKind::Snapshot { entities: labels };
    let id = ctx.vm.alloc_collection(kind);
    Ok(JsValue::Object(id))
}

// --- add / remove / item / namedItem (proxy to options) -----------

fn sel_add(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "add")? else {
        return Ok(JsValue::Undefined);
    };
    // Allocate a transient HTMLOptionsCollection wrapper for the
    // select and dispatch the original args through the shared
    // `native_options_add` impl — no need to round-trip through
    // prototype property lookup.  The wrapper is GC-rooted via
    // `live_collection_states` until the next sweep, after which
    // the un-referenced wrapper is reclaimed.
    let coll_id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Options { select: entity });
    super::dom_collection::native_options_add(ctx, JsValue::Object(coll_id), args)
}

fn sel_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "remove")? else {
        return Ok(JsValue::Undefined);
    };
    // No-arg form: `select.remove()` falls back to
    // `ChildNode.remove` on the receiver itself per HTML §4.10.7.4
    // (legacy `remove()` dispatch).
    if args.is_empty() {
        if let Some(parent) = ctx.host().dom().get_parent(entity) {
            let _ = ctx.host().dom().remove_child(parent, entity);
        }
        return Ok(JsValue::Undefined);
    }
    let coll_id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Options { select: entity });
    super::dom_collection::native_options_remove(ctx, JsValue::Object(coll_id), args)
}

fn sel_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "item")? else {
        return Ok(JsValue::Null);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    if n < 0 {
        return Ok(JsValue::Null);
    }
    let idx = n as usize;
    let options = collect_options(ctx, entity);
    Ok(match options.get(idx) {
        Some(&opt) => JsValue::Object(ctx.vm.create_element_wrapper(opt)),
        None => JsValue::Null,
    })
}

fn sel_named_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_select_receiver(ctx, this, "namedItem")? else {
        return Ok(JsValue::Null);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let key = ctx.vm.strings.get_utf8(sid);
    if key.is_empty() {
        return Ok(JsValue::Null);
    }
    let options = collect_options(ctx, entity);
    let dom = ctx.host().dom();
    // id-match wins over name-match per HTMLCollection §4.2.10.2 step 2.1.
    let mut name_hit: Option<Entity> = None;
    for &option in &options {
        if dom.with_attribute(option, "id", |v| v == Some(key.as_str())) {
            return Ok(JsValue::Object(ctx.vm.create_element_wrapper(option)));
        }
        if name_hit.is_none() && dom.with_attribute(option, "name", |v| v == Some(key.as_str())) {
            name_hit = Some(option);
        }
    }
    Ok(match name_hit {
        Some(e) => JsValue::Object(ctx.vm.create_element_wrapper(e)),
        None => JsValue::Null,
    })
}
