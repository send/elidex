//! `Element.prototype` intrinsic (WHATWG DOM §4.9).
//!
//! Holds **Element-only** members — element-scoped tree navigation
//! (`firstElementChild`, `children`, `childElementCount`, …),
//! attribute manipulation (`getAttribute`, `setAttribute`, …),
//! selector helpers (`matches`, `closest`), `tagName` / `id` /
//! `className`, and the `ChildNode` mixin method `remove()`.
//!
//! Node-common members — `parentNode`, `parentElement`,
//! `firstChild`, `nodeType`, `textContent`, `appendChild`,
//! `removeChild`, etc. — live on `Node.prototype` and so apply to
//! Text / Comment / Document / DocumentFragment wrappers too.
//!
//! ## Prototype chain
//!
//! ```text
//! element wrapper (HostObject)
//!   → Element.prototype        (this intrinsic)
//!     → Node.prototype         (Node-common accessors + mutation)
//!       → EventTarget.prototype
//!         → Object.prototype   (bootstrap)
//! ```
//!
//! Text and Comment wrappers skip `Element.prototype` and chain
//! directly to `Node.prototype` → `EventTarget.prototype`.  This
//! keeps Element-specific names off Text instances
//! (`textNode.getAttribute` is `undefined`, matching browsers)
//! while still exposing Node-common members on them.
//!
//! ## Why a shared prototype?
//!
//! The alternative — installing methods directly on each element
//! wrapper — would allocate one native-function per method per
//! element (tens of methods × thousands of elements).  A single
//! shared prototype matches browser engines (V8's `HTMLElement`
//! prototype chain, SpiderMonkey's `ElementProto`) and aligns with
//! how other intrinsics (`Array.prototype`, `Window.prototype`) are
//! structured elsewhere in the VM.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{
    coerce_first_arg_to_string, coerce_first_arg_to_string_id, invoke_dom_api,
    query_selector_all_snapshot, tree_nav_getter, wrap_entities_as_array,
};
use super::element_attrs::{
    native_element_get_attribute, native_element_get_attribute_names,
    native_element_get_attribute_node, native_element_get_attributes,
    native_element_get_class_name, native_element_get_id, native_element_get_tag_name,
    native_element_has_attribute, native_element_remove_attribute,
    native_element_remove_attribute_node, native_element_set_attribute,
    native_element_set_attribute_node, native_element_set_class_name, native_element_set_id,
    native_element_toggle_attribute,
};
use super::event_target::entity_from_this;

use elidex_ecs::{NodeKind, TagType};

impl VmInner {
    /// Allocate `Element.prototype` whose parent is
    /// `Node.prototype`.
    ///
    /// Called from `register_globals()` **after**
    /// `register_node_prototype` so the chain can climb through
    /// `Node.prototype` → `EventTarget.prototype` → `Object.prototype`.
    ///
    /// # Panics
    ///
    /// Panics if `node_prototype` has not been populated (would mean
    /// `register_node_prototype` was skipped or called in the wrong
    /// order).
    pub(in crate::vm) fn register_element_prototype(&mut self) {
        let node_proto = self
            .node_prototype
            .expect("register_element_prototype called before register_node_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(node_proto),
            extensible: true,
        });
        self.element_prototype = Some(proto_id);
        self.install_element_tree_nav(proto_id);
        self.install_element_attributes(proto_id);
        // ChildNode mixin (WHATWG §5.2.2) — `before` / `after` /
        // `replaceWith` / `remove`.  The same native fns are installed
        // on `CharacterData.prototype` from
        // `register_character_data_prototype`, matching WHATWG's
        // mixin-on-multiple-interfaces pattern.
        self.install_child_node_mixin(proto_id);
        // ParentNode mixin (WHATWG §5.2.4) — `prepend` / `append` /
        // `replaceChildren`.  The document wrapper gets its own copy
        // patched per-bind in `install_document_methods_if_needed`.
        // DocumentFragment wrappers currently chain via Node.prototype
        // and so do not see these members yet.
        self.install_parent_node_mixin(proto_id);
        self.install_element_matches(proto_id);
    }

    /// Install Element-only tree-navigation accessors from the
    /// ParentNode / NonDocumentTypeChildNode mixins defined on
    /// `Element.prototype` (WHATWG DOM §4.4 / §4.9).  Node-level
    /// accessors (`parentNode`, `parentElement`, `firstChild`,
    /// `childNodes`, …) live on `Node.prototype`.
    fn install_element_tree_nav(&mut self, proto_id: ObjectId) {
        for (name_sid, getter) in [
            (
                self.well_known.first_element_child,
                native_element_get_first_element_child as NativeFn,
            ),
            (
                self.well_known.last_element_child,
                native_element_get_last_element_child,
            ),
            (
                self.well_known.next_element_sibling,
                native_element_get_next_element_sibling,
            ),
            (
                self.well_known.previous_element_sibling,
                native_element_get_previous_element_sibling,
            ),
            (self.well_known.children, native_element_get_children),
            (
                self.well_known.child_element_count,
                native_element_get_child_element_count,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    /// Install Element attribute-manipulation methods + `id` /
    /// `className` / `tagName` accessors on `proto_id`.
    fn install_element_attributes(&mut self, proto_id: ObjectId) {
        // `tagName` — read-only accessor (WHATWG §4.9, uppercase for
        // HTML).
        self.install_accessor_pair(
            proto_id,
            self.well_known.tag_name,
            native_element_get_tag_name,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // `id` / `className` — read/write accessors (WHATWG §3.5).
        // `WEBIDL_RO_ACCESSOR`'s `writable` bit is meaningless for
        // accessors; the RW-ness comes from the setter slot below.
        for (name_sid, getter, setter) in [
            (
                self.well_known.id,
                native_element_get_id as NativeFn,
                native_element_set_id as NativeFn,
            ),
            (
                self.well_known.class_name,
                native_element_get_class_name,
                native_element_set_class_name,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        // `attributes` accessor — returns a live `NamedNodeMap`
        // backed by the element's `Attributes` component
        // (WHATWG §4.9).
        self.install_accessor_pair(
            proto_id,
            self.well_known.attributes,
            native_element_get_attributes,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // Attribute methods.
        for (name_sid, func) in [
            (
                self.well_known.get_attribute,
                native_element_get_attribute as NativeFn,
            ),
            (self.well_known.set_attribute, native_element_set_attribute),
            (
                self.well_known.remove_attribute,
                native_element_remove_attribute,
            ),
            (self.well_known.has_attribute, native_element_has_attribute),
            (
                self.well_known.get_attribute_names,
                native_element_get_attribute_names,
            ),
            (
                self.well_known.toggle_attribute,
                native_element_toggle_attribute,
            ),
            // Attr-typed methods — WHATWG §4.9.2.
            (
                self.well_known.get_attribute_node,
                native_element_get_attribute_node,
            ),
            (
                self.well_known.set_attribute_node,
                native_element_set_attribute_node,
            ),
            (
                self.well_known.remove_attribute_node,
                native_element_remove_attribute_node,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }

    /// Install `matches(selector)` / `closest(selector)` +
    /// `querySelector(selector)` / `querySelectorAll(selector)` +
    /// `insertAdjacentElement` / `insertAdjacentText` on
    /// `proto_id`.  The querySelector family is subtree-scoped
    /// (WHATWG §4.2.6) — `this` itself is not a match candidate,
    /// only its light-tree descendants.
    fn install_element_matches(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (self.well_known.matches, native_element_matches as NativeFn),
            (self.well_known.closest, native_element_closest),
            (
                self.well_known.query_selector,
                native_element_query_selector,
            ),
            (
                self.well_known.query_selector_all,
                native_element_query_selector_all,
            ),
            (
                self.well_known.insert_adjacent_element,
                super::element_insert_adjacent::native_element_insert_adjacent_element,
            ),
            (
                self.well_known.insert_adjacent_text,
                super::element_insert_adjacent::native_element_insert_adjacent_text,
            ),
            (
                self.well_known.get_elements_by_tag_name,
                native_element_get_elements_by_tag_name,
            ),
            (
                self.well_known.get_elements_by_class_name,
                native_element_get_elements_by_class_name,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }
}

// ---------------------------------------------------------------------------
// Natives: tree-navigation accessors
// ---------------------------------------------------------------------------

fn native_element_get_first_element_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, elidex_ecs::EcsDom::first_element_child)
}

fn native_element_get_last_element_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, elidex_ecs::EcsDom::last_element_child)
}

fn native_element_get_next_element_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, elidex_ecs::EcsDom::next_element_sibling)
}

fn native_element_get_previous_element_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, elidex_ecs::EcsDom::prev_element_sibling)
}

fn native_element_get_children(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // `element.children` is a live `HTMLCollection` — every access
    // re-traverses the parent's children to include concurrent
    // mutations (WHATWG §4.2.10).
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Children { parent: entity });
    Ok(JsValue::Object(id))
}

fn native_element_get_child_element_count(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Number(0.0));
    };
    let dom = ctx.host().dom();
    let count = dom
        .children_iter(entity)
        .filter(|c| dom.world().get::<&TagType>(*c).is_ok())
        .count();
    #[allow(clippy::cast_precision_loss)]
    // child counts in practice fit in u32, well within f64 mantissa
    let count_f = count as f64;
    Ok(JsValue::Number(count_f))
}

// ---------------------------------------------------------------------------
// Natives: matches / closest
// ---------------------------------------------------------------------------

fn native_element_matches(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "Element", "matches", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Boolean(false));
    };
    let target_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "matches", entity, &[JsValue::String(target_sid)])
}

/// `Element.prototype.querySelector(selector)` (WHATWG §4.2.6).
/// Subtree-scoped — `this` itself is never a match candidate, only
/// its descendants.  The `QuerySelector` handler in `elidex-dom-api`
/// runs the same DFS regardless of whether the receiver is a
/// Document or an Element.
fn native_element_query_selector(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "Element", "querySelector", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Null);
    };
    let target_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "querySelector", entity, &[JsValue::String(target_sid)])
}

/// `Element.prototype.querySelectorAll(selector)` — subtree-scoped.
///
/// WHATWG §4.2.6: returns a **static** NodeList.  The selector is
/// evaluated once, the matching entities are captured in a
/// `Snapshot` kind, and subsequent reads serve from that frozen
/// list.  Live collection kinds (ByTag / ByClass) are reserved for
/// `getElementsBy*` and `element.children`.
///
/// Uses the engine-independent `elidex_dom_api::query_selector_all`
/// free function (handler architecture cannot return `Vec<Entity>`).
fn native_element_query_selector_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "Element", "querySelectorAll", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Null);
    };
    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    query_selector_all_snapshot(ctx, entity, &selector_str)
}

fn native_element_closest(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "Element", "closest", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Null);
    };
    let target_sid = coerce_first_arg_to_string_id(ctx, args)?;
    invoke_dom_api(ctx, "closest", entity, &[JsValue::String(target_sid)])
}

// ---------------------------------------------------------------------------
// Element.prototype.getElementsByTagName / getElementsByClassName — WHATWG §4.2.6
// ---------------------------------------------------------------------------

/// `Element.prototype.getElementsByTagName(qualifiedName)` — WHATWG §4.2.6.2.
///
/// Scope is **descendants of the receiver only**; the receiver is
/// never a match candidate.  Shares
/// [`collect_descendants_by_tag_name`] with the Document-level form
/// so `*` handling and ASCII case-folding cannot drift.
pub(super) fn native_element_get_elements_by_tag_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WebIDL brand check runs BEFORE argument conversion — otherwise
    // `Element.prototype.getElementsByTagName.call({}, {toString(){ ... }})`
    // would trigger user code via ToString even though the invalid
    // receiver should be a silent no-op.
    let Some(root) =
        super::event_target::require_receiver(ctx, this, "Element", "getElementsByTagName", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    };
    let tag = coerce_first_arg_to_string(ctx, args)?;
    let tag_sid = ctx.vm.strings.intern(&tag);
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::ByTag {
            root,
            tag: tag_sid,
            all: tag == "*",
        });
    Ok(JsValue::Object(id))
}

/// `Element.prototype.getElementsByClassName(classNames)` —
/// WHATWG §4.2.6.2.  Descendant-only, live.
pub(super) fn native_element_get_elements_by_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(root) = super::event_target::require_receiver(
        ctx,
        this,
        "Element",
        "getElementsByClassName",
        |k| k == NodeKind::Element,
    )?
    else {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    };
    let class_str = coerce_first_arg_to_string(ctx, args)?;
    let class_names: Vec<_> = class_str
        .split_whitespace()
        .map(|c| ctx.vm.strings.intern(c))
        .collect();
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::ByClass { root, class_names });
    Ok(JsValue::Object(id))
}
