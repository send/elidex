//! Live DOM collections — `HTMLCollection` + `NodeList` (WHATWG DOM
//! §4.2.10 / §4.2.10.1).
//!
//! Both interfaces are implemented as payload-free `ObjectKind`
//! variants ([`ObjectKind::HtmlCollection`](super::super::value::ObjectKind::HtmlCollection)
//! / [`ObjectKind::NodeList`](super::super::value::ObjectKind::NodeList))
//! with their filter state held in
//! [`VmInner::live_collection_states`](super::super::VmInner::live_collection_states)
//! as engine-independent
//! [`elidex_dom_api::LiveCollection`] values. The single
//! [`elidex_dom_api::CollectionKind`] discriminator drives prototype
//! selection at wrapper construction; `namedItem` exposure follows
//! the same `HtmlCollection` / `NodeList` split.
//!
//! ## Liveness
//!
//! Per spec, `HTMLCollection` is always live and `NodeList` is
//! live *except* when returned by `querySelectorAll` (§4.2.6).
//! That single static case is represented by
//! [`elidex_dom_api::CollectionFilter::Snapshot`] and constructed
//! via [`elidex_dom_api::LiveCollection::new_snapshot`]; every other
//! filter exposes the *observable* semantics that mutations made
//! after the collection is obtained are visible on subsequent
//! `length` / `item(i)` / indexed accesses.
//!
//! Cache validation against
//! [`EcsDom::inclusive_descendants_version`] lives entirely inside
//! the engine-independent [`elidex_dom_api::LiveCollection`]; this
//! module supplies the marshalling + brand-check + named-property
//! lookup glue and forwards `length` / `item` / iteration calls
//! straight to the API's [`snapshot`](elidex_dom_api::LiveCollection::snapshot)
//! / [`item`](elidex_dom_api::LiveCollection::item) /
//! [`length`](elidex_dom_api::LiveCollection::length) accessors.
//!
//! ## GC contract
//!
//! The prototypes are rooted via the `proto_roots` array (gc.rs).
//! [`elidex_dom_api::LiveCollection`] stores only `Entity`, owned
//! `String` (filter needles), `Vec<Entity>` (cached snapshot +
//! `Snapshot`-variant frozen list), and `u64` (cached subtree
//! version) — **no `ObjectId` references** — so the trace step has
//! nothing to fan out. The sweep tail prunes
//! `live_collection_states` entries whose key `ObjectId` was
//! collected, same pattern as `headers_states` / `blob_data`.
//!
//! ## Brand check
//!
//! Methods on both prototypes route through
//! [`require_collection_receiver`] which extracts the backing
//! filter kind and issues "Illegal invocation" TypeError for
//! non-collection receivers (so `HTMLCollection.prototype.item.call({})`
//! throws, matching WebIDL brand semantics).

#![cfg(feature = "engine")]

use elidex_dom_api::{CollectionKind, LiveCollection};
use elidex_ecs::Entity;

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError, ARRAY_ITER_KIND_VALUES,
};
use super::super::{NativeFn, VmInner};

// -------------------------------------------------------------------------
// Named item tag allowlist (HTMLCollection only)
// -------------------------------------------------------------------------

/// Tags whose `name` attribute contributes to
/// `HTMLCollection.namedItem` lookup per WHATWG §4.2.10.2.1
/// step 2.2.  `id` matching applies to every Element regardless of
/// tag; the `name` fallback is restricted to this list so
/// `<div name="foo">` does NOT surface via `namedItem("foo")`.
///
/// Compared via `eq_ignore_ascii_case` against static literals — no
/// owned lowercase copy is allocated per call (namedItem resolution
/// invokes this in its inner loop on every indexed access).
pub(super) fn tag_allows_name_lookup(tag: &str) -> bool {
    const NAMED_ITEM_TAGS: [&str; 15] = [
        "a", "area", "button", "form", "fieldset", "iframe", "img", "input", "object", "output",
        "select", "textarea", "map", "meta", "embed",
    ];
    NAMED_ITEM_TAGS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(tag))
}

// -------------------------------------------------------------------------
// Prototype registration
// -------------------------------------------------------------------------

impl VmInner {
    /// Allocate `HTMLCollection.prototype` and install `length` /
    /// `item` / `namedItem` / `[Symbol.iterator]`.  Shared methods
    /// (`length` / `item` / `@@iterator`) use HTMLCollection-tagged
    /// wrappers so brand-check failures surface `"HTMLCollection"`
    /// in the error message rather than the shared-native default.
    pub(in crate::vm) fn register_html_collection_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.html_collection_prototype = Some(proto_id);

        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_hc_length_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_native_method(
            proto_id,
            self.well_known.item,
            native_hc_item,
            shape::PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            self.well_known.named_item,
            native_collection_named_item,
            shape::PropertyAttrs::METHOD,
        );
        self.install_symbol_iterator(proto_id, native_hc_iterator);
    }

    /// Allocate `NodeList.prototype` and install `length` / `item` /
    /// `forEach` / `[Symbol.iterator]`.  Shared methods use
    /// NodeList-tagged wrappers so brand-check error messages
    /// say `"NodeList"` when reached via `NodeList.prototype.*`.
    pub(in crate::vm) fn register_node_list_prototype(&mut self) {
        let obj_proto = self.object_prototype;
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: obj_proto,
            extensible: true,
        });
        self.node_list_prototype = Some(proto_id);

        self.install_accessor_pair(
            proto_id,
            self.well_known.length,
            native_nl_length_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        self.install_native_method(
            proto_id,
            self.well_known.item,
            native_nl_item,
            shape::PropertyAttrs::METHOD,
        );
        self.install_native_method(
            proto_id,
            self.well_known.for_each,
            native_node_list_for_each,
            shape::PropertyAttrs::METHOD,
        );
        self.install_symbol_iterator(proto_id, native_nl_iterator);
    }

    /// Install `[Symbol.iterator]` pointing at the per-interface
    /// iterator wrapper so brand-check errors reflect the prototype
    /// the user reached through.
    fn install_symbol_iterator(&mut self, proto_id: ObjectId, iter_fn: NativeFn) {
        let fn_id = self.create_native_function("[Symbol.iterator]", iter_fn);
        let sym_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            sym_key,
            PropertyValue::Data(JsValue::Object(fn_id)),
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Allocate a new collection wrapper backed by `coll`. The
    /// returned `ObjectId` carries `ObjectKind::HtmlCollection` or
    /// `ObjectKind::NodeList` (chosen by the collection's
    /// [`CollectionKind`]) and points at the matching prototype.
    ///
    /// Convention: do **not** add new collection variants by
    /// extending [`ObjectKind`]; extend
    /// [`elidex_dom_api::CollectionFilter`] (and where needed
    /// [`elidex_dom_api::CollectionKind`]) instead. The two-enum
    /// split here is purely engine-bound (`ObjectKind` selects a
    /// prototype + brand) — collection *shape* belongs in the
    /// engine-independent crate.
    pub(crate) fn alloc_collection(&mut self, coll: LiveCollection) -> ObjectId {
        let (object_kind, proto) = match coll.kind() {
            CollectionKind::HtmlCollection => (
                ObjectKind::HtmlCollection,
                self.html_collection_prototype
                    .expect("alloc_collection before register_html_collection_prototype"),
            ),
            CollectionKind::NodeList => (
                ObjectKind::NodeList,
                self.node_list_prototype
                    .expect("alloc_collection before register_node_list_prototype"),
            ),
        };
        let id = self.alloc_object(Object {
            kind: object_kind,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.live_collection_states.insert(id, coll);
        id
    }
}

// -------------------------------------------------------------------------
// Native methods
// -------------------------------------------------------------------------

/// Brand-check helper — recover the `ObjectId` + whether the
/// receiver is an HTMLCollection (vs NodeList), enforcing per-method
/// interface contracts.
///
/// `interface` is the name of the prototype the method was installed
/// on (`"HTMLCollection"` or `"NodeList"`).  It drives both the
/// error message AND the accepted receiver kinds: HTMLCollection's
/// prototype accepts HTMLCollection receivers, NodeList's prototype
/// accepts NodeList receivers.  Shared methods (`length` / `item` /
/// `@@iterator`) are installed on BOTH prototypes via per-interface
/// wrapper natives so the error message reflects the prototype the
/// caller reached through — e.g.
/// `NodeList.prototype.length.call({})` reports
/// `"Failed to execute 'length' on 'NodeList'"`.
///
/// Returns a TypeError for non-collection receivers or mismatched
/// collection kinds so `.call({})` and cross-interface
/// `.call(otherKind)` both throw "Illegal invocation" (matches
/// browser behaviour).
fn require_collection_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
    interface: &'static str,
) -> Result<(ObjectId, bool), VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': Illegal invocation"
        )));
    };
    let kind_is_html = matches!(ctx.vm.get_object(id).kind, ObjectKind::HtmlCollection);
    let kind_is_node_list = matches!(ctx.vm.get_object(id).kind, ObjectKind::NodeList);
    // `interface == "NodeList"` accepts only NodeList receivers;
    // every other interface value (including the two wrapper-less
    // HTMLCollection-only natives below) accepts only HTMLCollection.
    let ok = if interface == "NodeList" {
        kind_is_node_list
    } else {
        kind_is_html
    };
    if !ok {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': Illegal invocation"
        )));
    }
    Ok((id, kind_is_html))
}

/// Run `f` against the receiver's `LiveCollection` and the bound DOM,
/// returning `fallback` when the VM is unbound or the receiver has no
/// entry in `live_collection_states`. Wraps the disjoint-borrow
/// accessor pattern that every collection method uses.
fn with_collection<R>(
    ctx: &mut NativeContext<'_>,
    id: ObjectId,
    fallback: R,
    f: impl FnOnce(&elidex_ecs::EcsDom, &mut LiveCollection) -> R,
) -> R {
    let Some((dom, states)) = ctx.dom_and_collection_states_if_bound() else {
        return fallback;
    };
    let Some(coll) = states.get_mut(&id) else {
        return fallback;
    };
    f(dom, coll)
}

/// Resolve the backing entity list for a live collection receiver as
/// an owned `Vec`. Used by methods that need to hand the slice to JS
/// (`@@iterator`, `forEach`) or iterate while holding `&mut ctx.vm`
/// (`namedItem`, which calls `create_element_wrapper`); fast-path
/// methods like `length` / `item(i)` should use [`with_collection`]
/// directly to avoid the per-call clone.
fn resolve_receiver_entities(ctx: &mut NativeContext<'_>, id: ObjectId) -> Vec<Entity> {
    with_collection(ctx, id, Vec::new(), |dom, coll| coll.snapshot(dom).to_vec())
}

// Per-interface `length` getter wrappers — shared body, per-
// interface brand-failure message.
fn native_hc_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_length_get_impl(ctx, this, "HTMLCollection")
}

fn native_nl_length_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_length_get_impl(ctx, this, "NodeList")
}

fn collection_length_get_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    interface: &'static str,
) -> Result<JsValue, VmError> {
    let (id, _) = require_collection_receiver(ctx, this, "length", interface)?;
    // Direct `LiveCollection::length` call avoids the per-access Vec
    // clone that `resolve_receiver_entities` performs.
    let len = with_collection(ctx, id, 0, |dom, coll| coll.length(dom));
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(len as f64))
}

// Per-interface `item` wrappers.
fn native_hc_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_item_impl(ctx, this, args, "HTMLCollection")
}

fn native_nl_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_item_impl(ctx, this, args, "NodeList")
}

fn collection_item_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    interface: &'static str,
) -> Result<JsValue, VmError> {
    let (id, _) = require_collection_receiver(ctx, this, "item", interface)?;
    let index = match args.first() {
        Some(JsValue::Number(n)) if n.is_finite() => {
            let trunc = n.trunc();
            if trunc < 0.0 {
                return Ok(JsValue::Null);
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let idx = trunc as usize;
            idx
        }
        Some(v) => {
            // WebIDL `unsigned long` — coerce; negatives / NaN map
            // to out-of-range (null).
            let n = super::super::coerce::to_int32(ctx.vm, *v)?;
            if n < 0 {
                return Ok(JsValue::Null);
            }
            #[allow(clippy::cast_sign_loss)]
            let idx = n as usize;
            idx
        }
        None => return Ok(JsValue::Null),
    };
    // Direct `LiveCollection::item` call avoids the per-access Vec
    // clone; the entity is copied out before `create_element_wrapper`
    // re-acquires `&mut ctx.vm`.
    let entity = with_collection(ctx, id, None, |dom, coll| coll.item(index, dom));
    Ok(match entity {
        Some(e) => JsValue::Object(ctx.vm.create_element_wrapper(e)),
        None => JsValue::Null,
    })
}

fn native_collection_named_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // HTMLCollection-only method — unconditionally brand-check
    // against HTMLCollection.  A NodeList receiver fails at the
    // require_collection_receiver call since `interface`=HTMLCollection
    // rejects non-HTMLCollection kinds.
    let (id, _is_html_collection) =
        require_collection_receiver(ctx, this, "namedItem", "HTMLCollection")?;
    let Some(arg) = args.first().copied() else {
        return Ok(JsValue::Null);
    };
    let key_sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let key = ctx.vm.strings.get_utf8(key_sid);
    if key.is_empty() {
        return Ok(JsValue::Null);
    }
    let entities = resolve_receiver_entities(ctx, id);
    // Pass 1 — id match wins over name match (WHATWG §4.2.10.2 step 2.1).
    let mut name_hit: Option<Entity> = None;
    for &e in &entities {
        let dom = ctx.host().dom();
        if dom.with_attribute(e, "id", |v| v == Some(key.as_str())) {
            return Ok(JsValue::Object(ctx.vm.create_element_wrapper(e)));
        }
        if name_hit.is_none()
            && dom.with_tag_name(e, |t| t.is_some_and(tag_allows_name_lookup))
            && dom.with_attribute(e, "name", |v| v == Some(key.as_str()))
        {
            name_hit = Some(e);
        }
    }
    Ok(match name_hit {
        Some(e) => JsValue::Object(ctx.vm.create_element_wrapper(e)),
        None => JsValue::Null,
    })
}

// Per-interface `@@iterator` wrappers.
fn native_hc_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_iterator_impl(ctx, this, args, "HTMLCollection")
}

fn native_nl_iterator(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    collection_iterator_impl(ctx, this, args, "NodeList")
}

fn collection_iterator_impl(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
    interface: &'static str,
) -> Result<JsValue, VmError> {
    let (id, _) = require_collection_receiver(ctx, this, "@@iterator", interface)?;
    let entities = resolve_receiver_entities(ctx, id);
    let values: Vec<JsValue> = entities
        .into_iter()
        .map(|e| JsValue::Object(ctx.vm.create_element_wrapper(e)))
        .collect();
    // Wrap the snapshot Array in an `ArrayIterator` so standard
    // `for ... of` consumption walks the snapshot.  The
    // `ARRAY_ITER_KIND_VALUES` discriminant selects the values-
    // yielding iterator (vs keys / entries).  Note: this iterator
    // reflects the *snapshot* at `@@iterator` time, not live DOM
    // changes that occur during iteration — consistent with
    // WHATWG §4.2.10 where the collection is live on access but
    // iteration uses IteratorProtocol over a captured Array.
    let array_id = ctx.vm.create_array_object(values);
    let proto = ctx.vm.array_iterator_prototype;
    let iter_obj = ctx.vm.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(super::super::value::ArrayIterState {
            array_id,
            index: 0,
            kind: ARRAY_ITER_KIND_VALUES,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    Ok(JsValue::Object(iter_obj))
}

fn native_node_list_for_each(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // NodeList-only method — brand-check against NodeList.
    let (id, _) = require_collection_receiver(ctx, this, "forEach", "NodeList")?;
    let callback = args.first().copied().unwrap_or(JsValue::Undefined);
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let callable = match callback {
        JsValue::Object(oid) => ctx.vm.get_object(oid).kind.is_callable(),
        _ => false,
    };
    if !callable {
        return Err(VmError::type_error(
            "Failed to execute 'forEach' on 'NodeList': callback is not callable".to_string(),
        ));
    }
    let entities = resolve_receiver_entities(ctx, id);
    for (idx, entity) in entities.into_iter().enumerate() {
        let value = JsValue::Object(ctx.vm.create_element_wrapper(entity));
        #[allow(clippy::cast_precision_loss)]
        let index_val = JsValue::Number(idx as f64);
        ctx.vm
            .call_value(callback, this_arg, &[value, index_val, this])?;
    }
    Ok(JsValue::Undefined)
}

// -------------------------------------------------------------------------
// Indexed property access (called from `ops_property::get_element`)
// -------------------------------------------------------------------------

/// Try to resolve `collection[index]` for HTMLCollection / NodeList
/// receivers.  Returns `Some(entity)` when the key is a valid
/// numeric index (or a numeric-string for HTMLCollection's legacy
/// named property fallback).  Returns `None` when the caller
/// should fall through to the regular property / prototype lookup
/// path (so non-index lookups like `coll.length` still see the
/// prototype accessor).
///
/// Returns the backing `Entity` rather than a wrapped `JsValue`
/// so the caller can drop the `&EcsDom` borrow before invoking
/// `create_element_wrapper`.  The wrapper path mutably borrows
/// `VmInner::host_data::wrapper_cache`, which aliases the shared
/// reborrow chain used to obtain this `&elidex_ecs::EcsDom` from
/// `HostData`; splitting the phases eliminates the Stacked-Borrows
/// violation.
pub(crate) fn try_indexed_get(
    vm: &mut VmInner,
    dom: &elidex_ecs::EcsDom,
    id: ObjectId,
    key: JsValue,
) -> Option<Entity> {
    let is_html_collection = matches!(vm.get_object(id).kind, ObjectKind::HtmlCollection);
    let coll = vm.live_collection_states.get_mut(&id)?;
    // The API LiveCollection handles every variant uniformly —
    // `Snapshot` returns its frozen list, every other filter
    // re-walks (or hits the cache) against `dom`. The Stacked-
    // Borrows-friendly slice borrow lives inside the API for the
    // duration of this call.
    let entities = coll.snapshot(dom);

    match key {
        JsValue::Number(n) if n.is_finite() => {
            let trunc = n.trunc();
            if (trunc - n).abs() > f64::EPSILON || trunc < 0.0 {
                None
            } else {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let idx = trunc as usize;
                entities.get(idx).copied()
            }
        }
        JsValue::String(sid) => {
            // Canonical array-index parse (ES §6.1.7 "array index" /
            // §7.1.21 CanonicalNumericIndexString): rejects
            // non-canonical forms like "01" / "+1" / "1.0" so
            // `coll['01']` does NOT alias `coll[1]`.  Reuses the
            // shared `parse_array_index_u32` helper that already
            // enforces the leading-zero + length constraints.
            let key_units = vm.strings.get(sid);
            if let Some(idx_u32) = super::super::coerce_format::parse_array_index_u32(key_units) {
                let idx = idx_u32 as usize;
                entities.get(idx).copied()
            } else if !is_html_collection {
                // Non-canonical-index string on a NodeList → no
                // named-property path (WHATWG; see HTMLCollection-
                // only `namedItem` below).
                None
            } else {
                // HTMLCollection legacy named property access
                // (WHATWG HTML §4.2.10): `id` attribute first, then
                // `name` restricted to the tag allowlist.
                let key_str = vm.strings.get_utf8(sid);
                let mut name_hit: Option<Entity> = None;
                let mut id_hit: Option<Entity> = None;
                for &e in entities {
                    if dom.with_attribute(e, "id", |v| v == Some(key_str.as_str())) {
                        id_hit = Some(e);
                        break;
                    }
                    if name_hit.is_none()
                        && dom.with_tag_name(e, |t| t.is_some_and(tag_allows_name_lookup))
                        && dom.with_attribute(e, "name", |v| v == Some(key_str.as_str()))
                    {
                        name_hit = Some(e);
                    }
                }
                id_hit.or(name_hit)
            }
        }
        _ => None,
    }
}
