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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::dom_bridge::{
    coerce_first_arg_to_string, parse_dom_selector, query_selector_in_subtree_all,
    query_selector_in_subtree_first, tree_nav_getter, wrap_entities_as_array, wrap_entity_or_null,
};
use super::event_target::entity_from_this;

use elidex_ecs::{Entity, NodeKind, TagType};

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
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    /// Install Element attribute-manipulation methods + `id` /
    /// `className` / `tagName` accessors on `proto_id`.
    fn install_element_attributes(&mut self, proto_id: ObjectId) {
        // `tagName` — read-only accessor (WHATWG §4.9, uppercase for
        // HTML).
        let tag_name_sid = self.well_known.tag_name;
        let tag_name_name = self.strings.get_utf8(tag_name_sid);
        let tag_gid = self
            .create_native_function(&format!("get {tag_name_name}"), native_element_get_tag_name);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(tag_name_sid),
            PropertyValue::Accessor {
                getter: Some(tag_gid),
                setter: None,
            },
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        // `id` / `className` — read/write accessors (WHATWG §3.5).
        // `WEBIDL_RO_ACCESSOR`'s `writable` bit is meaningless for
        // accessors; the RW-ness comes from the setter slot below.
        let rw_attrs = shape::PropertyAttrs::WEBIDL_RO_ACCESSOR;
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
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            let sid = self.create_native_function(&format!("set {name}"), setter);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: Some(sid),
                },
                rw_attrs,
            );
        }

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
        ] {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                shape::PropertyAttrs::METHOD,
            );
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
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                shape::PropertyAttrs::METHOD,
            );
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
    tree_nav_getter(ctx, this, |dom, e| dom.first_element_child(e))
}

fn native_element_get_last_element_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, |dom, e| dom.last_element_child(e))
}

fn native_element_get_next_element_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, |dom, e| dom.next_element_sibling(e))
}

fn native_element_get_previous_element_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    tree_nav_getter(ctx, this, |dom, e| dom.prev_element_sibling(e))
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
    Ok(JsValue::Number(count as f64))
}

// ---------------------------------------------------------------------------
// Natives: attribute manipulation + id / className / tagName
// ---------------------------------------------------------------------------

fn native_element_get_tag_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    // WHATWG DOM §4.9 tagName: HTML elements are uppercase.  Every
    // document we bind is treated as HTML in Phase 2.
    let tag = ctx.host().dom().get_tag_name(entity);
    match tag {
        Some(t) => {
            let upper = t.to_ascii_uppercase();
            let sid = ctx.vm.strings.intern(&upper);
            Ok(JsValue::String(sid))
        }
        None => Ok(JsValue::String(ctx.vm.well_known.empty)),
    }
}

/// Read attribute `name` on `entity` as a String, or `None` if absent.
///
/// Thin shim around [`elidex_ecs::EcsDom::get_attribute`]; retained here
/// to keep call sites terse and to enforce the `NativeContext` borrow
/// discipline.
fn attr_get(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) -> Option<String> {
    ctx.host().dom().get_attribute(entity, name)
}

/// Set attribute `name` = `value` on `entity`.  Shim around
/// [`elidex_ecs::EcsDom::set_attribute`].  Returns `false` when the
/// entity has been destroyed.
fn attr_set(ctx: &mut NativeContext<'_>, entity: Entity, name: &str, value: String) -> bool {
    ctx.host().dom().set_attribute(entity, name, value)
}

/// Remove attribute `name` from `entity`.  Shim around
/// [`elidex_ecs::EcsDom::remove_attribute`].
fn attr_remove(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) {
    ctx.host().dom().remove_attribute(entity, name);
}

fn native_element_get_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let name = coerce_first_arg_to_string(ctx, args)?;
    match attr_get(ctx, entity, &name) {
        Some(v) => {
            let sid = ctx.vm.strings.intern(&v);
            Ok(JsValue::String(sid))
        }
        None => Ok(JsValue::Null),
    }
}

fn native_element_set_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // Coerce BOTH args (name then value) per WebIDL even though the
    // spec name-validation step runs on a qualified name; we accept
    // any string here and defer validation to a future HTML5 parser
    // upgrade.
    let name = coerce_first_arg_to_string(ctx, args)?;
    let value_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let value_sid = super::super::coerce::to_string(ctx.vm, value_arg)?;
    let value = ctx.vm.strings.get_utf8(value_sid);
    attr_set(ctx, entity, &name, value);
    Ok(JsValue::Undefined)
}

fn native_element_remove_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let name = coerce_first_arg_to_string(ctx, args)?;
    attr_remove(ctx, entity, &name);
    Ok(JsValue::Undefined)
}

fn native_element_has_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let name = coerce_first_arg_to_string(ctx, args)?;
    let has = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&elidex_ecs::Attributes>(entity)
            .ok()
            .is_some_and(|attrs| attrs.contains(&name))
    };
    Ok(JsValue::Boolean(has))
}

fn native_element_get_attribute_names(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        // WHATWG §4.9.2 getAttributeNames — returns a list; we return
        // an empty Array for unbound / non-HostObject receivers.
        return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
    };
    let names: Vec<String> = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&elidex_ecs::Attributes>(entity)
            .ok()
            .map(|attrs| attrs.iter().map(|(k, _)| k.to_owned()).collect())
            .unwrap_or_default()
    };
    let values: Vec<JsValue> = names
        .into_iter()
        .map(|n| {
            let sid = ctx.vm.strings.intern(&n);
            JsValue::String(sid)
        })
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(values)))
}

fn native_element_toggle_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    let name = coerce_first_arg_to_string(ctx, args)?;

    // `force` (second arg): undefined = toggle, true = ensure present,
    // false = ensure absent.  WHATWG §4.9.2 toggleAttribute.
    let force: Option<bool> = match args.get(1).copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => None,
        v => Some(super::super::coerce::to_boolean(ctx.vm, v)),
    };

    let currently_present = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&elidex_ecs::Attributes>(entity)
            .ok()
            .is_some_and(|attrs| attrs.contains(&name))
    };

    let final_present = match force {
        Some(true) => {
            if !currently_present {
                // WHATWG §4.9.2: when force=true and absent, set value to
                // empty string.
                attr_set(ctx, entity, &name, String::new());
            }
            true
        }
        Some(false) => {
            if currently_present {
                attr_remove(ctx, entity, &name);
            }
            false
        }
        None => {
            if currently_present {
                attr_remove(ctx, entity, &name);
                false
            } else {
                attr_set(ctx, entity, &name, String::new());
                true
            }
        }
    };
    Ok(JsValue::Boolean(final_present))
}

// ---------------------------------------------------------------------------
// id / className (reflected as the underlying attribute)
// ---------------------------------------------------------------------------

/// Shared body for reflected-string-attribute getters (`id`,
/// `className`).  Missing attribute returns the empty string (not
/// `null` like `getAttribute`) per WHATWG §4.9.
fn reflected_string_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    attr_name: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let val = attr_get(ctx, entity, attr_name).unwrap_or_default();
    if val.is_empty() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = ctx.vm.strings.intern(&val);
    Ok(JsValue::String(sid))
}

/// Shared body for reflected-string-attribute setters.
fn reflected_string_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    attr_name: &str,
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let value = coerce_first_arg_to_string(ctx, args)?;
    attr_set(ctx, entity, attr_name, value);
    Ok(JsValue::Undefined)
}

fn native_element_get_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    reflected_string_get(ctx, this, "id")
}

fn native_element_set_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    reflected_string_set(ctx, this, args, "id")
}

fn native_element_get_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    reflected_string_get(ctx, this, "class")
}

fn native_element_set_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    reflected_string_set(ctx, this, args, "class")
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
    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    let selectors = parse_dom_selector(&selector_str, "matches/closest")?;
    let dom = ctx.host().dom();
    let matched = selectors.iter().any(|s| s.matches(entity, dom));
    Ok(JsValue::Boolean(matched))
}

/// `Element.prototype.querySelector(selector)` (WHATWG §4.2.6).
/// Subtree-scoped — `this` itself is never a match candidate, only
/// its descendants.
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
    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    let selectors = parse_dom_selector(&selector_str, "querySelector")?;
    let matched = query_selector_in_subtree_first(ctx.host().dom(), entity, &selectors);
    Ok(wrap_entity_or_null(ctx.vm, matched))
}

/// `Element.prototype.querySelectorAll(selector)` — subtree-scoped.
///
/// WHATWG §4.2.6: returns a **static** NodeList.  The selector is
/// evaluated once, the matching entities are captured in a
/// `Snapshot` kind, and subsequent reads serve from that frozen
/// list.  Live collection kinds (ByTag / ByClass) are reserved for
/// `getElementsBy*` and `element.children`.
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
    let selectors = parse_dom_selector(&selector_str, "querySelectorAll")?;
    let entities = query_selector_in_subtree_all(ctx.host().dom(), entity, &selectors);
    let id = ctx
        .vm
        .alloc_collection(super::dom_collection::LiveCollectionKind::Snapshot { entities });
    Ok(JsValue::Object(id))
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
    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    let selectors = parse_dom_selector(&selector_str, "matches/closest")?;

    // Walk self → parent ancestors, returning the first matching
    // Element.  WHATWG §4.9 closest() is inclusive and stops at the
    // first non-Element parent (or at the root).
    let matched: Option<Entity> = {
        let dom = ctx.host().dom();
        let mut current = Some(entity);
        let mut out = None;
        while let Some(e) = current {
            if dom.world().get::<&TagType>(e).is_ok() && selectors.iter().any(|s| s.matches(e, dom))
            {
                out = Some(e);
                break;
            }
            // Walk only to Element ancestors, matching
            // `parentElement` semantics — this naturally stops at
            // the `ShadowRoot` (no TagType) so closest() does not
            // cross the shadow boundary to the host.  Document root
            // also has no TagType, so the walk stops there in the
            // normal case too.
            current = dom
                .get_parent(e)
                .filter(|p| dom.world().get::<&TagType>(*p).is_ok());
        }
        out
    };
    Ok(wrap_entity_or_null(ctx.vm, matched))
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
