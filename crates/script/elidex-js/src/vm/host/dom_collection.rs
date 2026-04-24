//! Live DOM collections — `HTMLCollection` + `NodeList` (WHATWG DOM
//! §4.2.10 / §4.2.10.1).
//!
//! Both interfaces are implemented as payload-free `ObjectKind`
//! variants ([`ObjectKind::HtmlCollection`](super::super::value::ObjectKind::HtmlCollection)
//! / [`ObjectKind::NodeList`](super::super::value::ObjectKind::NodeList))
//! with their filter state held in
//! [`VmInner::live_collection_states`](super::super::VmInner::live_collection_states).
//! They share this single state table because every kind is
//! resolved by the same per-access ECS traversal — the only
//! distinction at read time is which variant of
//! [`LiveCollectionKind`] drives the filter predicate and (for
//! HTMLCollection only) whether `namedItem` is exposed.
//!
//! ## Liveness
//!
//! Per spec, `HTMLCollection` is always live and `NodeList` is
//! live *except* when returned by `querySelectorAll` (§4.2.6).
//! That single static case is represented by
//! [`LiveCollectionKind::Snapshot`], which carries a pre-captured
//! `Vec<Entity>`; every other kind re-traverses the ECS on each
//! read, so mutations made after the collection is obtained are
//! observable on subsequent `length` / `item(i)` / indexed
//! accesses.  No caching layer — the spec text *is* the algorithm.
//!
//! ## GC contract
//!
//! The prototypes are rooted via the `proto_roots` array (gc.rs).
//! `LiveCollectionKind` stores only `Entity`, `StringId`,
//! `Vec<StringId>`, and `Vec<Entity>` — **no `ObjectId` references**
//! — so the trace step has nothing to fan out.  The sweep tail
//! prunes `live_collection_states` entries whose key `ObjectId`
//! was collected, same pattern as `headers_states` / `blob_data`.
//!
//! ## Brand check
//!
//! Methods on both prototypes route through
//! [`require_collection_receiver`] which extracts the backing
//! filter kind and issues "Illegal invocation" TypeError for
//! non-collection receivers (so `HTMLCollection.prototype.item.call({})`
//! throws, matching WebIDL brand semantics).

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity, NodeKind};

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError, ARRAY_ITER_KIND_VALUES,
};
use super::super::{NativeFn, StringId, VmInner};

// -------------------------------------------------------------------------
// Filter discriminator
// -------------------------------------------------------------------------

/// The filter backing an `HTMLCollection` / `NodeList` wrapper.
///
/// HTMLCollection variants filter to **Element** nodes only
/// (WHATWG §4.2.10); NodeList variants include Text and Comment
/// nodes as well.  The `Snapshot` variant alone is static — every
/// other kind re-traverses on read.
pub(crate) enum LiveCollectionKind {
    // HTMLCollection filters (Element-only).
    ByTag {
        root: Entity,
        tag: StringId,
        all: bool,
    },
    ByClass {
        root: Entity,
        class_names: Vec<StringId>,
    },
    Children {
        parent: Entity,
    },
    Forms {
        doc: Entity,
    },
    Images {
        doc: Entity,
    },
    Links {
        doc: Entity,
    },
    // NodeList filters.
    ChildNodes {
        parent: Entity,
    },
    Snapshot {
        entities: Vec<Entity>,
    },
    ByName {
        doc: Entity,
        name: StringId,
    },
}

impl LiveCollectionKind {
    /// `true` if this variant backs an `HTMLCollection`; `false`
    /// for `NodeList` variants.  Drives prototype selection at
    /// wrapper construction and `namedItem` exposure at method
    /// install time.
    fn is_html_collection(&self) -> bool {
        matches!(
            self,
            Self::ByTag { .. }
                | Self::ByClass { .. }
                | Self::Children { .. }
                | Self::Forms { .. }
                | Self::Images { .. }
                | Self::Links { .. }
        )
    }
}

// -------------------------------------------------------------------------
// Entity resolution (per-access re-traversal)
// -------------------------------------------------------------------------

/// Re-resolve the entity list backing `kind`.  Every live variant
/// walks the ECS at this point; `Snapshot` clones the cached vec.
///
/// Takes resolved UTF-8 needles (tag / class-name / name-attr)
/// pre-materialised by the caller — avoids threading the
/// `StringPool` through here alongside the `&EcsDom` borrow, which
/// would otherwise fight the `NativeContext` split-field pattern.
///
/// Returning a `Vec` (not an iterator) is deliberate — callers
/// typically need the length up front, and the re-allocation
/// cost is dominated by the traversal anyway.
fn resolve_entities_with_needles(
    dom: &EcsDom,
    kind: &LiveCollectionKind,
    resolved_needles: &ResolvedNeedles,
) -> Vec<Entity> {
    match kind {
        LiveCollectionKind::ByTag { root, all, .. } => {
            let tag_str = resolved_needles.primary.as_deref().unwrap_or("");
            let mut out = Vec::new();
            dom.traverse_descendants(*root, |e| {
                if e == *root {
                    return true; // skip root itself (Element.getElementsByTagName semantics)
                }
                let matches = *all
                    || dom
                        .get_tag_name(e)
                        .is_some_and(|t| t.eq_ignore_ascii_case(tag_str));
                if matches && dom.node_kind_inferred(e) == Some(NodeKind::Element) {
                    out.push(e);
                }
                true
            });
            out
        }
        LiveCollectionKind::ByClass { root, .. } => {
            if resolved_needles.class_names.is_empty() {
                return Vec::new();
            }
            let mut out = Vec::new();
            dom.traverse_descendants(*root, |e| {
                if e == *root {
                    return true;
                }
                if dom.node_kind_inferred(e) != Some(NodeKind::Element) {
                    return true;
                }
                if let Some(class_attr) = dom.get_attribute(e, "class") {
                    let has_all = resolved_needles
                        .class_names
                        .iter()
                        .all(|n| class_attr.split_whitespace().any(|c| c == n));
                    if has_all {
                        out.push(e);
                    }
                }
                true
            });
            out
        }
        LiveCollectionKind::Children { parent } => dom
            .children_iter(*parent)
            .filter(|&e| dom.node_kind_inferred(e) == Some(NodeKind::Element))
            .collect(),
        LiveCollectionKind::Forms { doc } => collect_descendants_with_tag(dom, *doc, "form"),
        LiveCollectionKind::Images { doc } => collect_descendants_with_tag(dom, *doc, "img"),
        LiveCollectionKind::Links { doc } => {
            let mut out = Vec::new();
            dom.traverse_descendants(*doc, |e| {
                if e == *doc {
                    return true;
                }
                let tag_ok = dom
                    .get_tag_name(e)
                    .is_some_and(|t| t.eq_ignore_ascii_case("a") || t.eq_ignore_ascii_case("area"));
                if tag_ok && dom.get_attribute(e, "href").is_some() {
                    out.push(e);
                }
                true
            });
            out
        }
        LiveCollectionKind::ChildNodes { parent } => dom.children_iter(*parent).collect(),
        LiveCollectionKind::Snapshot { entities } => entities.clone(),
        LiveCollectionKind::ByName { doc, .. } => {
            let needle = resolved_needles.primary.as_deref().unwrap_or("");
            let mut out = Vec::new();
            dom.traverse_descendants(*doc, |e| {
                // Skip the doc root itself — `getElementsByName` is
                // a descendant-only query — and restrict to Element
                // nodes per WHATWG HTML §3.1.5 step 1 ("list of
                // elements with the given name").  Non-Element
                // nodes that happen to carry a `name` attribute
                // (possible via direct `EcsDom::set_attribute` on
                // any entity) must not leak into the result.
                if e == *doc {
                    return true;
                }
                if dom.node_kind_inferred(e) != Some(NodeKind::Element) {
                    return true;
                }
                if dom.get_attribute(e, "name").is_some_and(|v| v == needle) {
                    out.push(e);
                }
                true
            });
            out
        }
    }
}

/// Pre-materialised UTF-8 needles for entity resolution — filled
/// by [`resolve_needles`] from the filter's `StringId`s before
/// the EcsDom borrow is taken.
struct ResolvedNeedles {
    primary: Option<String>,
    class_names: Vec<String>,
}

/// Extract the UTF-8 strings a kind's filter consults so the
/// actual traversal can run with a disjoint `&EcsDom` borrow.
fn resolve_needles(vm: &VmInner, kind: &LiveCollectionKind) -> ResolvedNeedles {
    match kind {
        LiveCollectionKind::ByTag { tag, .. } => ResolvedNeedles {
            primary: Some(vm.strings.get_utf8(*tag)),
            class_names: Vec::new(),
        },
        LiveCollectionKind::ByClass { class_names, .. } => ResolvedNeedles {
            primary: None,
            class_names: class_names
                .iter()
                .map(|sid| vm.strings.get_utf8(*sid))
                .collect(),
        },
        LiveCollectionKind::ByName { name, .. } => ResolvedNeedles {
            primary: Some(vm.strings.get_utf8(*name)),
            class_names: Vec::new(),
        },
        _ => ResolvedNeedles {
            primary: None,
            class_names: Vec::new(),
        },
    }
}

/// Public resolver — materialises needles from the VM's string
/// pool, then runs the ECS traversal.  Uses a matched pair of
/// disjoint borrows (`&VmInner` for strings, then `&EcsDom` from
/// host data) to avoid NativeContext's aliasing constraint.
pub(super) fn resolve_entities_for(
    ctx: &mut NativeContext<'_>,
    kind: &LiveCollectionKind,
) -> Vec<Entity> {
    let needles = resolve_needles(ctx.vm, kind);
    resolve_entities_with_needles(ctx.host().dom(), kind, &needles)
}

/// Helper — collect every descendant Element with a given tag.
fn collect_descendants_with_tag(dom: &EcsDom, root: Entity, tag: &str) -> Vec<Entity> {
    let mut out = Vec::new();
    dom.traverse_descendants(root, |e| {
        if e == root {
            return true;
        }
        if dom
            .get_tag_name(e)
            .is_some_and(|t| t.eq_ignore_ascii_case(tag))
        {
            out.push(e);
        }
        true
    });
    out
}

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

        self.install_rw_accessor_once(proto_id, self.well_known.length, native_hc_length_get, None);
        self.install_method(proto_id, self.well_known.item, native_hc_item);
        self.install_method(
            proto_id,
            self.well_known.named_item,
            native_collection_named_item,
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

        self.install_rw_accessor_once(proto_id, self.well_known.length, native_nl_length_get, None);
        self.install_method(proto_id, self.well_known.item, native_nl_item);
        self.install_method(
            proto_id,
            self.well_known.for_each,
            native_node_list_for_each,
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

    fn install_rw_accessor_once(
        &mut self,
        proto_id: ObjectId,
        name_sid: StringId,
        getter: NativeFn,
        setter: Option<NativeFn>,
    ) {
        let display = self.strings.get_utf8(name_sid);
        let gid = self.create_native_function(&format!("get {display}"), getter);
        let sid = setter.map(|f| self.create_native_function(&format!("set {display}"), f));
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(name_sid),
            PropertyValue::Accessor {
                getter: Some(gid),
                setter: sid,
            },
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_method(&mut self, proto_id: ObjectId, name_sid: StringId, func: NativeFn) {
        let display = self.strings.get_utf8(name_sid);
        let fn_id = self.create_native_function(&display, func);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(name_sid),
            PropertyValue::Data(JsValue::Object(fn_id)),
            shape::PropertyAttrs::METHOD,
        );
    }

    /// Allocate a new collection wrapper backed by `kind`.  The
    /// returned `ObjectId` carries `ObjectKind::HtmlCollection` or
    /// `ObjectKind::NodeList` (chosen by the kind's discriminator)
    /// and points at the matching prototype.
    pub(crate) fn alloc_collection(&mut self, kind: LiveCollectionKind) -> ObjectId {
        let (object_kind, proto) = if kind.is_html_collection() {
            (
                ObjectKind::HtmlCollection,
                self.html_collection_prototype
                    .expect("alloc_collection before register_html_collection_prototype"),
            )
        } else {
            (
                ObjectKind::NodeList,
                self.node_list_prototype
                    .expect("alloc_collection before register_node_list_prototype"),
            )
        };
        let id = self.alloc_object(Object {
            kind: object_kind,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.live_collection_states.insert(id, kind);
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

/// Resolve the backing entity list for a live collection receiver.
///
/// The temporary remove / re-insert dance is deliberate: the
/// traversal needs a shared borrow of `ctx.vm.strings` (via
/// `resolve_needles`) **and** a shared borrow of `ctx.host().dom()`
/// on the same `NativeContext`, but Rust's split-field rules deny
/// two aliasing paths through `ctx.vm` in one expression.  Taking
/// the kind out of the map temporarily breaks that aliasing into
/// two sequential borrows.
fn resolve_receiver_entities(ctx: &mut NativeContext<'_>, id: ObjectId) -> Vec<Entity> {
    let Some(kind) = ctx.vm.live_collection_states.remove(&id) else {
        return Vec::new();
    };
    let entities = resolve_entities_for(ctx, &kind);
    ctx.vm.live_collection_states.insert(id, kind);
    entities
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
    let entities = resolve_receiver_entities(ctx, id);
    #[allow(clippy::cast_precision_loss)]
    Ok(JsValue::Number(entities.len() as f64))
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
    let entities = resolve_receiver_entities(ctx, id);
    Ok(match entities.get(index) {
        Some(&e) => JsValue::Object(ctx.vm.create_element_wrapper(e)),
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
        if dom.get_attribute(e, "id").as_deref() == Some(key.as_str()) {
            return Ok(JsValue::Object(ctx.vm.create_element_wrapper(e)));
        }
        if name_hit.is_none() {
            if let Some(tag) = dom.get_tag_name(e) {
                if tag_allows_name_lookup(&tag)
                    && dom.get_attribute(e, "name").as_deref() == Some(key.as_str())
                {
                    name_hit = Some(e);
                }
            }
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
/// reborrow chain used to obtain this `&EcsDom` from `HostData`;
/// splitting the phases eliminates the Stacked-Borrows violation.
pub(crate) fn try_indexed_get(
    vm: &mut VmInner,
    dom: &EcsDom,
    id: ObjectId,
    key: JsValue,
) -> Option<Entity> {
    let is_html_collection = matches!(vm.get_object(id).kind, ObjectKind::HtmlCollection);
    // Remove / re-insert mirrors `resolve_receiver_entities` so we
    // can take concurrent borrows of `vm.strings` (for the needle
    // lookup) and `dom` (for the traversal) without tripping the
    // aliasing rules.
    let kind = vm.live_collection_states.remove(&id)?;
    let needles = resolve_needles(vm, &kind);
    let entities = resolve_entities_with_needles(dom, &kind, &needles);
    vm.live_collection_states.insert(id, kind);

    match key {
        JsValue::Number(n) if n.is_finite() => {
            let trunc = n.trunc();
            if (trunc - n).abs() > f64::EPSILON || trunc < 0.0 {
                return None;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let idx = trunc as usize;
            entities.get(idx).copied()
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
                return entities.get(idx).copied();
            }
            let key_str = vm.strings.get_utf8(sid);
            // Non-canonical-index string on HTMLCollection → legacy
            // named property access (WHATWG HTML spec §4.2.10): `id`
            // attribute first, then `name` restricted to the tag
            // allowlist.  NodeList has no named-property path, so
            // return None for it.
            if !is_html_collection {
                return None;
            }
            let mut name_hit: Option<Entity> = None;
            for &e in &entities {
                if dom.get_attribute(e, "id").as_deref() == Some(key_str.as_str()) {
                    return Some(e);
                }
                if name_hit.is_none() {
                    if let Some(tag) = dom.get_tag_name(e) {
                        if tag_allows_name_lookup(&tag)
                            && dom.get_attribute(e, "name").as_deref() == Some(key_str.as_str())
                        {
                            name_hit = Some(e);
                        }
                    }
                }
            }
            name_hit
        }
        _ => None,
    }
}
