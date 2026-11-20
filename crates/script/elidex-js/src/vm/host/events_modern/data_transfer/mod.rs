//! `DataTransfer` + `DataTransferItem` + `DataTransferItemList`
//! constructors + prototypes (HTML DnD §6.2 / §6.3).
//!
//! ## State storage
//!
//! All three interfaces share a single state map keyed by the
//! parent `DataTransfer` `ObjectId`:
//!
//! - `DataTransfer`     → [`super::DataTransferState`] (mutable
//!   container, holds the items Vec + enum values + wrapper caches).
//! - `DataTransferItem` → wrapper-only, carries
//!   `(parent_dt_id, index)` inline.  Identity-cached in the unified
//!   wrapper store under `WrapperKind::DataTransferItem`.
//! - `DataTransferItemList` → wrapper-only, single instance per
//!   parent; cached on `DataTransferState::items_wrapper`.
//!
//! ## Deferred (D-14 paired)
//!
//! - `DataTransfer.files` accessor returns an empty FileList stub
//!   (slot `#11-data-transfer-file-paired`).
//! - `DataTransferItem.getAsFile()` returns null.
//! - `DataTransferItemList.add(File)` overload throws TypeError.
//!
//! ## Deferred (paint pipeline)
//!
//! - `setDragImage(element, x, y)` stores entity_bits + coords;
//!   real paint integration deferred to slot
//!   `#11-data-transfer-drag-image-paint`.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, StringId, VmError,
};
use super::super::super::VmInner;
use super::super::events::install_ctor;
mod item;
mod item_list;

pub(in crate::vm) use item::register_data_transfer_item_global;
use item_list::invalidate_item_wrapper_cache_from;
pub(in crate::vm) use item_list::register_data_transfer_item_list_global;

use super::{DataTransferEntry, DataTransferState, DropEffect, EffectAllowed};

// ---------------------------------------------------------------------------
// Registration glue
// ---------------------------------------------------------------------------

pub(in crate::vm) fn register_data_transfer_global(vm: &mut VmInner) {
    let parent = vm
        .object_prototype
        .expect("register_data_transfer_global requires object_prototype");
    let proto_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(parent),
        extensible: true,
    });
    vm.data_transfer_prototype = Some(proto_id);
    install_ctor(
        vm,
        proto_id,
        "DataTransfer",
        native_data_transfer_constructor,
        vm.well_known.data_transfer_global,
        super::super::super::value::CallShape::ConstructorOnly,
    );
    install_data_transfer_accessors(vm, proto_id);
    install_data_transfer_methods(vm, proto_id);
}

fn install_data_transfer_accessors(vm: &mut VmInner, proto_id: ObjectId) {
    let k_drop_effect = vm.well_known.drop_effect;
    let k_effect_allowed = vm.well_known.effect_allowed;
    let k_items = vm.well_known.items;
    let k_files = vm.well_known.files;
    let k_types = vm.well_known.types;
    // `dropEffect` get + set (mutable enum, ASCII-CI input).
    vm.install_accessor_pair(
        proto_id,
        k_drop_effect,
        native_dt_get_drop_effect,
        Some(native_dt_set_drop_effect),
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_accessor_pair(
        proto_id,
        k_effect_allowed,
        native_dt_get_effect_allowed,
        Some(native_dt_set_effect_allowed),
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    // `items` / `files` / `types` readonly accessors.
    vm.install_accessor_pair(
        proto_id,
        k_items,
        native_dt_get_items,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_accessor_pair(
        proto_id,
        k_files,
        native_dt_get_files,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
    vm.install_accessor_pair(
        proto_id,
        k_types,
        native_dt_get_types,
        None,
        shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
}

#[allow(clippy::similar_names)] // spec method names share data_-prefix
fn install_data_transfer_methods(vm: &mut VmInner, proto_id: ObjectId) {
    let k_get_data = vm.well_known.get_data;
    let k_set_data = vm.well_known.set_data;
    let k_clear_data = vm.well_known.clear_data;
    let k_set_drag_image = vm.well_known.set_drag_image;
    vm.install_native_method(
        proto_id,
        k_get_data,
        native_dt_get_data,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        k_set_data,
        native_dt_set_data,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        k_clear_data,
        native_dt_clear_data,
        shape::PropertyAttrs::METHOD,
    );
    vm.install_native_method(
        proto_id,
        k_set_drag_image,
        native_dt_set_drag_image,
        shape::PropertyAttrs::METHOD,
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Brand-check kind for [`require_dt_brand`].  Mirrors the
/// `BrandCheckKind` enum in `events.rs` (lesson #211 source) — but
/// scoped local to D-9 because the DataTransfer brand check is a
/// single-variant `ObjectKind::DataTransfer` match, not a
/// prototype-chain walk like UIEvent-family receivers.  Splitting
/// the wording at the helper level keeps every call site
/// declarative.
#[derive(Clone, Copy)]
enum DtBrand<'a> {
    /// Attribute getter / setter — formats as
    /// `"Failed to {op} the '{member}' property from 'DataTransfer': Illegal invocation."`
    /// matching the wording already standardised by
    /// `events.rs::BrandCheckKind::Attribute`.
    Attribute { op: &'a str, member: &'a str },
    /// Operation method — formats as
    /// `"Failed to execute '{method}' on 'DataTransfer': Illegal invocation."`
    Operation { method: &'a str },
}

fn require_dt_brand(
    ctx: &NativeContext<'_>,
    this: JsValue,
    kind: DtBrand<'_>,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::DataTransfer) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(match kind {
        DtBrand::Attribute { op, member } => format!(
            "Failed to {op} the '{member}' property from 'DataTransfer': \
             Illegal invocation."
        ),
        DtBrand::Operation { method } => {
            format!("Failed to execute '{method}' on 'DataTransfer': Illegal invocation.")
        }
    }))
}

/// Const inert default returned by `dt_state` when the wrapper's
/// state entry is missing — survives `Vm::unbind()` clearing
/// `data_transfer_states`.  Matches `DataTransferState::empty()`
/// exactly so post-unbind reads behave like a fresh
/// `new DataTransfer()` would: no items, default enums, no drag
/// image.  `dt_state_mut` lazily reinserts a real entry on first
/// post-unbind write so subsequent reads see the mutation.
static EMPTY_DT_STATE: DataTransferState = DataTransferState {
    drop_effect: DropEffect::None,
    effect_allowed: EffectAllowed::None,
    items: Vec::new(),
    items_wrapper: None,
    files_wrapper: None,
    file_entries: Vec::new(),
    drag_image_entity: None,
    drag_image_x: 0,
    drag_image_y: 0,
};

fn dt_state(vm: &VmInner, id: ObjectId) -> &DataTransferState {
    vm.data_transfer_states.get(&id).unwrap_or(&EMPTY_DT_STATE)
}

pub(super) fn dt_state_mut(vm: &mut VmInner, id: ObjectId) -> &mut DataTransferState {
    vm.data_transfer_states
        .entry(id)
        .or_insert_with(DataTransferState::empty)
}

// ---------------------------------------------------------------------------
// DataTransfer constructor
// ---------------------------------------------------------------------------

fn native_data_transfer_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let proto = ctx
        .vm
        .data_transfer_prototype
        .expect("DataTransfer.prototype must be registered before ctor");
    let id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::DataTransfer,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    ctx.vm
        .data_transfer_states
        .insert(id, DataTransferState::empty());
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// dropEffect / effectAllowed accessors
// ---------------------------------------------------------------------------

fn native_dt_get_drop_effect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Attribute {
            op: "read",
            member: "dropEffect",
        },
    )?;
    let s = dt_state(ctx.vm, id).drop_effect.as_str();
    let sid = ctx.vm.strings.intern(s);
    Ok(JsValue::String(sid))
}

fn native_dt_set_drop_effect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Attribute {
            op: "set",
            member: "dropEffect",
        },
    )?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::super::coerce::to_string(ctx.vm, val)?;
    // Read the string back as Rust &str for ASCII-CI parse.
    let raw = ctx.vm.strings.get_utf8(sid);
    if let Some(new_val) = DropEffect::from_str_ascii_ci(&raw) {
        dt_state_mut(ctx.vm, id).drop_effect = new_val;
    }
    // Silent retain on invalid value per HTML §6.2.
    Ok(JsValue::Undefined)
}

fn native_dt_get_effect_allowed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Attribute {
            op: "read",
            member: "effectAllowed",
        },
    )?;
    let s = dt_state(ctx.vm, id).effect_allowed.as_str();
    let sid = ctx.vm.strings.intern(s);
    Ok(JsValue::String(sid))
}

fn native_dt_set_effect_allowed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Attribute {
            op: "set",
            member: "effectAllowed",
        },
    )?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::super::coerce::to_string(ctx.vm, val)?;
    let raw = ctx.vm.strings.get_utf8(sid);
    if let Some(new_val) = EffectAllowed::from_str_ascii_ci(&raw) {
        dt_state_mut(ctx.vm, id).effect_allowed = new_val;
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// items / files / types accessors
// ---------------------------------------------------------------------------

fn native_dt_get_items(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Attribute {
            op: "read",
            member: "items",
        },
    )?;
    // `[SameObject]` cache — return existing wrapper if present.
    if let Some(cached) = dt_state(ctx.vm, id).items_wrapper {
        return Ok(JsValue::Object(cached));
    }
    let proto = ctx
        .vm
        .data_transfer_item_list_prototype
        .expect("DataTransferItemList.prototype must be registered before items getter");
    let wrapper = ctx.vm.alloc_object(Object {
        kind: ObjectKind::DataTransferItemList { parent_dt_id: id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: Some(proto),
        extensible: true,
    });
    dt_state_mut(ctx.vm, id).items_wrapper = Some(wrapper);
    Ok(JsValue::Object(wrapper))
}

fn native_dt_get_files(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Attribute {
            op: "read",
            member: "files",
        },
    )?;
    if let Some(cached) = dt_state(ctx.vm, id).files_wrapper {
        return Ok(JsValue::Object(cached));
    }
    // D-14 `#11-file-api` Phase 5 — real FileList wrapper backed by
    // the DT's `file_entries` Vec (populated by `add(File)`).  Slot
    // `#11-data-transfer-file-paired` resolved here.  Identity-cache
    // so reads are `[SameObject]`-stable per spec.
    let file_ids = dt_state(ctx.vm, id).file_entries.clone();
    let file_list_id = super::super::file_list::create_file_list_from_ids(ctx.vm, file_ids);
    dt_state_mut(ctx.vm, id).files_wrapper = Some(file_list_id);
    Ok(JsValue::Object(file_list_id))
}

fn native_dt_get_types(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Attribute {
            op: "read",
            member: "types",
        },
    )?;
    // Build a fresh Array per `types` getter (HTML §6.2 step 3-5).
    // Each String entry contributes its format; any File-kind
    // entry contributes the literal `"Files"` (NOT yet reachable
    // since D-9 doesn't accept File entries, but the path is
    // wired in advance).
    //
    // WebIDL §3.2.27 specifies `FrozenArray<DOMString>`; this
    // returns a mutable Array because the VM's Array.prototype.push
    // bypasses `extensible: false` (`ObjectKind::Array { elements }`
    // mutates the Vec directly).  Calls return a fresh Array each
    // time, so script-side mutation cannot leak into the drag data
    // store — the observable contract (`dt.types !== dt.types`,
    // `dt.types.length` reflects current entries) holds.  Wiring
    // true descriptor-level freeze requires native_array_push +
    // `LoadElement` to honour extensible, which is a VM-wide change
    // beyond D-9's scope.
    // First-seen-order dedup via HashSet membership + Vec ordering —
    // keeps the build O(n) for scripts that grow `items` beyond a
    // handful of entries (Copilot R4 perf finding).
    let mut format_sids: Vec<StringId> = Vec::new();
    let mut seen: std::collections::HashSet<StringId> = std::collections::HashSet::new();
    let mut has_file = false;
    {
        let state = dt_state(ctx.vm, id);
        for entry in &state.items {
            match entry {
                DataTransferEntry::String { format, .. } => {
                    if seen.insert(*format) {
                        format_sids.push(*format);
                    }
                }
                DataTransferEntry::File { .. } => {
                    has_file = true;
                }
            }
        }
    }
    let mut elements: Vec<JsValue> = format_sids.into_iter().map(JsValue::String).collect();
    if has_file {
        let files_sid = ctx.vm.well_known.types_files_entry;
        elements.push(JsValue::String(files_sid));
    }
    let arr_id = ctx.vm.create_array_object(elements);
    Ok(JsValue::Object(arr_id))
}

// ---------------------------------------------------------------------------
// getData / setData / clearData methods
// ---------------------------------------------------------------------------

fn native_dt_get_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(ctx, this, DtBrand::Operation { method: "getData" })?;
    let format_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let format_sid = super::super::super::coerce::to_string(ctx.vm, format_val)?;
    // ASCII-CI format match on WTF-16 code units.  Concatenate
    // matching String entries (typical use is single-entry;
    // multi-entry concat matches HTML §6.2 step 4 "for each item in
    // the drag data store matching format").  Operating on the
    // WTF-16 pool slice preserves lone surrogates — `get_utf8`
    // would lossily replace them with `U+FFFD` and corrupt
    // round-trips for non-ASCII DOMString data.
    let mut buf: Vec<u16> = Vec::new();
    {
        let needle = ctx.vm.strings.get(format_sid).to_vec();
        let state = dt_state(ctx.vm, id);
        for entry in &state.items {
            if let DataTransferEntry::String { format, data } = entry {
                let entry_fmt = ctx.vm.strings.get(*format);
                if eq_ignore_ascii_case_wtf16(entry_fmt, &needle) {
                    buf.extend_from_slice(ctx.vm.strings.get(*data));
                }
            }
        }
    }
    let sid = ctx.vm.strings.intern_utf16(&buf);
    Ok(JsValue::String(sid))
}

/// ASCII case-insensitive equality on WTF-16 code units.  Folds
/// only ASCII-range upper-case into lower-case (`A-Z` → `a-z`);
/// surrogates and other Unicode code units compare bytewise per
/// WebIDL "ASCII case-insensitive match" rules.  Mirrors the
/// `str::eq_ignore_ascii_case` semantics on the WTF-16 plane.
#[inline]
fn eq_ignore_ascii_case_wtf16(a: &[u16], b: &[u16]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(&x, &y)| {
        let xl = if (0x41..=0x5A).contains(&x) {
            x | 0x20
        } else {
            x
        };
        let yl = if (0x41..=0x5A).contains(&y) {
            y | 0x20
        } else {
            y
        };
        xl == yl
    })
}

fn native_dt_set_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(ctx, this, DtBrand::Operation { method: "setData" })?;
    let format_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let data_val = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let format_sid = super::super::super::coerce::to_string(ctx.vm, format_val)?;
    let data_sid = super::super::super::coerce::to_string(ctx.vm, data_val)?;
    // Find the first matching String entry by ASCII-CI format.
    // Two-phase to avoid simultaneous &VmInner (string pool) +
    // &mut VmInner.data_transfer_states borrows.
    let match_idx = find_string_entry_index_ci(ctx.vm, id, format_sid);
    let state = dt_state_mut(ctx.vm, id);
    if let Some(i) = match_idx {
        if let DataTransferEntry::String { data, .. } = &mut state.items[i] {
            *data = data_sid;
        }
    } else {
        state.items.push(DataTransferEntry::String {
            format: format_sid,
            data: data_sid,
        });
    }
    Ok(JsValue::Undefined)
}

/// Locate the index of the first `DataTransferEntry::String` whose
/// format matches `format_sid` under ASCII case-insensitive
/// comparison on WTF-16 code units (preserves lone surrogates).
/// Returns `None` if no match.
fn find_string_entry_index_ci(vm: &VmInner, id: ObjectId, format_sid: StringId) -> Option<usize> {
    let needle = vm.strings.get(format_sid);
    let state = dt_state(vm, id);
    state.items.iter().position(|entry| match entry {
        DataTransferEntry::String { format, .. } => {
            eq_ignore_ascii_case_wtf16(vm.strings.get(*format), needle)
        }
        DataTransferEntry::File { .. } => false,
    })
}

fn native_dt_clear_data(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Operation {
            method: "clearData",
        },
    )?;
    let format_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    if matches!(format_arg, JsValue::Undefined) {
        // No argument — drain all String entries (File-kind entries
        // retained per HTML §6.2 step "Remove each item in the drag
        // data store item list whose kind is text").
        let had_removal = {
            let state = dt_state_mut(ctx.vm, id);
            let before = state.items.len();
            state
                .items
                .retain(|e| matches!(e, DataTransferEntry::File { .. }));
            state.items.len() != before
        };
        if had_removal {
            // Any String removal can shift File-entry indices and
            // invalidate every `(parent, index)` wrapper-cache key.
            invalidate_item_wrapper_cache_from(ctx.vm, id, 0);
        }
    } else {
        let format_sid = super::super::super::coerce::to_string(ctx.vm, format_arg)?;
        // Collect indices to remove (two-phase borrow split).
        // WTF-16 ASCII-CI compare preserves surrogates per the
        // `find_string_entry_index_ci` precedent.
        let needle = ctx.vm.strings.get(format_sid).to_vec();
        let to_remove: Vec<usize> = dt_state(ctx.vm, id)
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| match entry {
                DataTransferEntry::String { format, .. }
                    if eq_ignore_ascii_case_wtf16(ctx.vm.strings.get(*format), &needle) =>
                {
                    Some(i)
                }
                _ => None,
            })
            .collect();
        let smallest_removed = to_remove.first().copied();
        // Remove highest-index-first to keep earlier indices valid.
        let state = dt_state_mut(ctx.vm, id);
        for i in to_remove.into_iter().rev() {
            state.items.remove(i);
        }
        if let Some(from) = smallest_removed {
            // Indices ≥ smallest_removed shift; invalidate from there.
            invalidate_item_wrapper_cache_from(ctx.vm, id, from as u32);
        }
    }
    Ok(JsValue::Undefined)
}

/// `setDragImage(image)`-style Element argument coercion.  Returns
/// the entity bits or a WebIDL TypeError for non-Element values
/// (`null` / `undefined` / non-host objects / Document / Window /
/// Attr / Text / detached host wrappers).  Distinguishes
/// "detached entity" from "wrong type" per `require_element_arg`
/// precedent in `element_insert_adjacent.rs`.
fn require_element_arg_bits(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<u64, VmError> {
    let JsValue::Object(id) = value else {
        return Err(VmError::type_error(
            "Failed to execute 'setDragImage' on 'DataTransfer': \
             parameter 1 is not of type 'Element'.",
        ));
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(VmError::type_error(
            "Failed to execute 'setDragImage' on 'DataTransfer': \
             parameter 1 is not of type 'Element'.",
        ));
    };
    let Some(entity) = elidex_ecs::Entity::from_bits(entity_bits) else {
        return Err(VmError::type_error(
            "Failed to execute 'setDragImage' on 'DataTransfer': \
             parameter 1 is detached (invalid entity).",
        ));
    };
    // Post-unbind tolerance: when `Vm::unbind` has cleared host
    // pointers, `ctx.host().dom()` would panic.  Surface a TypeError
    // marking the receiver as detached instead — mirrors the
    // `EMPTY_DT_STATE` / `EMPTY_TOUCH_STATE` retained-wrapper
    // contract for `setDragImage` callers (Copilot R5 finding).
    let Some(host) = ctx.host_if_bound() else {
        return Err(VmError::type_error(
            "Failed to execute 'setDragImage' on 'DataTransfer': \
             parameter 1 is detached (invalid entity).",
        ));
    };
    let dom = host.dom();
    if !dom.contains(entity) {
        return Err(VmError::type_error(
            "Failed to execute 'setDragImage' on 'DataTransfer': \
             parameter 1 is detached (invalid entity).",
        ));
    }
    match dom.node_kind_inferred(entity) {
        Some(elidex_ecs::NodeKind::Element) => Ok(entity_bits),
        _ => Err(VmError::type_error(
            "Failed to execute 'setDragImage' on 'DataTransfer': \
             parameter 1 is not of type 'Element'.",
        )),
    }
}

fn native_dt_set_drag_image(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_dt_brand(
        ctx,
        this,
        DtBrand::Operation {
            method: "setDragImage",
        },
    )?;
    // `image: Element` — brand check.  WebIDL conversion is "any host
    // wrapper whose entity is `NodeKind::Element`"; Document / Window /
    // Attr / Text / non-host objects throw TypeError.
    let image_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let entity_bits = require_element_arg_bits(ctx, image_arg)?;
    let x_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let y_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
    let x_num = super::super::super::coerce::to_number(ctx.vm, x_arg)?;
    let y_num = super::super::super::coerce::to_number(ctx.vm, y_arg)?;
    let x = super::super::super::coerce::f64_to_int32(x_num);
    let y = super::super::super::coerce::f64_to_int32(y_num);

    let state = dt_state_mut(ctx.vm, id);
    state.drag_image_entity = Some(entity_bits);
    state.drag_image_x = x;
    state.drag_image_y = y;
    Ok(JsValue::Undefined)
}
