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
    coerce_first_arg_to_string, collect_descendants_by_class_name, collect_descendants_by_tag_name,
    parse_dom_selector, query_selector_in_subtree_all, query_selector_in_subtree_first,
    tree_nav_getter, wrap_entities_as_array, wrap_entity_or_null,
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
                native_element_insert_adjacent_element,
            ),
            (
                self.well_known.insert_adjacent_text,
                native_element_insert_adjacent_text,
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
// Accessor helpers
// ---------------------------------------------------------------------------

/// Collect direct children into a `Vec<Entity>`, optionally filtering
/// to elements only.  Returns a snapshot — mutations to the tree after
/// the call do not affect the returned vec.
fn collect_children(
    ctx: &mut NativeContext<'_>,
    entity: Entity,
    elements_only: bool,
) -> Vec<Entity> {
    let dom = ctx.host().dom();
    let mut out = Vec::new();
    for c in dom.children_iter(entity) {
        if elements_only && dom.world().get::<&TagType>(c).is_err() {
            continue;
        }
        out.push(c);
    }
    out
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
    let children = collect_children(ctx, entity, /*elements_only=*/ true);
    let elements: Vec<JsValue> = children
        .into_iter()
        .map(|e| JsValue::Object(ctx.vm.create_element_wrapper(e)))
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elements)))
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

/// `Element.prototype.querySelectorAll(selector)` — subtree-scoped,
/// returns a snapshot Array (live NodeList lands with Observers).
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
        // Unbound / non-HostObject receivers return `null`, matching
        // the other Element-side object-returning helpers
        // (`querySelector`, `closest`, `childNodes`).  HostObject
        // receivers of the wrong kind (e.g. a Text or Document
        // wrapper used via `Function.call`) throw TypeError via
        // `require_receiver` above.
        return Ok(JsValue::Null);
    };
    let selector_str = coerce_first_arg_to_string(ctx, args)?;
    let selectors = parse_dom_selector(&selector_str, "querySelectorAll")?;
    let matched = query_selector_in_subtree_all(ctx.host().dom(), entity, &selectors);
    Ok(wrap_entities_as_array(ctx.vm, &matched))
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
// insertAdjacentElement / insertAdjacentText — WHATWG §4.9
// ---------------------------------------------------------------------------

/// Which of the four WHATWG `where` positions (ASCII case-insensitive)
/// the caller passed to `insertAdjacent*`.
#[derive(Clone, Copy)]
enum InsertAdjacentWhere {
    BeforeBegin,
    AfterBegin,
    BeforeEnd,
    AfterEnd,
}

/// Parse the `where` argument into an [`InsertAdjacentWhere`], matching
/// ASCII case-insensitively against the four WHATWG literals.
fn parse_adjacent_position(raw: &str) -> Option<InsertAdjacentWhere> {
    // `eq_ignore_ascii_case` is O(n) on byte length; there are four
    // six-to-ten-byte literals so no optimisation is worthwhile.
    if raw.eq_ignore_ascii_case("beforebegin") {
        Some(InsertAdjacentWhere::BeforeBegin)
    } else if raw.eq_ignore_ascii_case("afterbegin") {
        Some(InsertAdjacentWhere::AfterBegin)
    } else if raw.eq_ignore_ascii_case("beforeend") {
        Some(InsertAdjacentWhere::BeforeEnd)
    } else if raw.eq_ignore_ascii_case("afterend") {
        Some(InsertAdjacentWhere::AfterEnd)
    } else {
        None
    }
}

/// TypeError thrown when `where` is not one of the four spec literals.
///
/// TODO(PR5a): upgrade `TypeError` → `DOMException("SyntaxError")`
/// when DOMException lands.  All callers embed the method name so the
/// message stays aligned with WHATWG `insertAdjacent*` step 1.
///
/// `where_value` is echoed into the message (matching Blink / Gecko)
/// so script debuggers see the exact literal the caller supplied.
fn adjacent_syntax_error(method: &str, where_value: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'Element': \
         the value provided ('{where_value}') is not one of \
         'beforebegin', 'afterbegin', 'beforeend', or 'afterend'."
    ))
}

/// True when `pos` is one of the two positions that require the
/// receiver to have a parent.  Used by `insertAdjacentText` to
/// pre-check before allocating a Text entity that would otherwise
/// leak into the ECS on early return.
fn position_requires_parent(pos: InsertAdjacentWhere) -> bool {
    matches!(
        pos,
        InsertAdjacentWhere::BeforeBegin | InsertAdjacentWhere::AfterEnd
    )
}

/// TypeError thrown when the second argument of `insertAdjacentElement`
/// is not an Element wrapper.  Matches the Blink / Gecko message form.
fn adjacent_element_arg_error() -> VmError {
    VmError::type_error(
        "Failed to execute 'insertAdjacentElement' on 'Element': \
         parameter 2 is not of type 'Element'."
            .to_owned(),
    )
}

/// TypeError thrown when `insertAdjacent*` fails pre-insertion validity
/// (cycle / ancestor self-insert / destroyed entity).
///
/// TODO(PR5a): upgrade `TypeError` → `DOMException("HierarchyRequestError")`
/// when DOMException lands.
fn adjacent_hierarchy_error(method: &str) -> VmError {
    VmError::type_error(format!(
        "Failed to execute '{method}' on 'Element': \
         the new child node cannot be inserted at this position."
    ))
}

/// TypeError thrown when `insertAdjacentElement`'s second argument
/// is a HostObject whose entity has been destroyed / recycled.
/// Separated from [`adjacent_element_arg_error`] so stale wrappers
/// surface the "detached" failure mode rather than being misreported
/// as non-Element (matches [`super::event_target::require_receiver`]
/// which also distinguishes destroyed vs. wrong-kind receivers).
fn adjacent_element_detached_error() -> VmError {
    VmError::type_error(
        "Failed to execute 'insertAdjacentElement' on 'Element': \
         parameter 2 is detached (invalid entity)."
            .to_owned(),
    )
}

/// Extract an Element [`Entity`] from a method argument, throwing a
/// WebIDL-style `TypeError` on any non-Element value (including
/// `null` / `undefined` / non-HostObject objects / HostObjects that
/// are not `NodeKind::Element`).  A HostObject whose entity has been
/// destroyed surfaces a distinct "detached" error so scripts can
/// distinguish stale wrappers from genuine type mismatches.
fn require_element_arg(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<Entity, VmError> {
    let JsValue::Object(id) = value else {
        return Err(adjacent_element_arg_error());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(adjacent_element_arg_error());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(adjacent_element_detached_error)?;
    // Stale-entity check BEFORE the kind lookup: a destroyed entity
    // has no components, so `node_kind_inferred` would return None
    // and masquerade as "wrong type".  Catching it here keeps the
    // error message aligned with `require_receiver` (which makes the
    // same split for stale receivers).
    if !ctx.host().dom().contains(entity) {
        return Err(adjacent_element_detached_error());
    }
    match ctx.host().dom().node_kind_inferred(entity) {
        Some(NodeKind::Element) => Ok(entity),
        _ => Err(adjacent_element_arg_error()),
    }
}

/// Perform the insertion step of `insertAdjacent*` (WHATWG §4.9).
/// `target` is the method receiver; `node` is the Element / Text to
/// insert.  Returns `Ok(Some(node))` when the insert succeeded,
/// `Ok(None)` when `where` is `beforebegin` / `afterend` but the
/// receiver has no parent (spec: return `null` without throwing), and
/// `Err` when `EcsDom` rejects the insertion (cycle / destroyed).
fn perform_adjacent_insert(
    ctx: &mut NativeContext<'_>,
    target: Entity,
    node: Entity,
    pos: InsertAdjacentWhere,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let dom = ctx.host().dom();
    // WHATWG `Node.insertBefore(x, x)` and its `x, x.nextSibling`
    // sibling form treat "insert a node before itself" as a no-op
    // that succeeds (§4.2.3 pre-insertion step 2).  `EcsDom::insert_before`
    // rejects `new_child == ref_child` as invalid, so every position
    // that would reduce to that edge case returns Ok(Some(node))
    // before the rejecting call — matching the ChildNode mixin's
    // `insert_before(parent, x, x)` accommodation in
    // `vm/host/childnode.rs`.
    match pos {
        InsertAdjacentWhere::BeforeBegin => {
            let Some(parent) = dom.get_parent(target) else {
                return Ok(None);
            };
            // `parent.insertBefore(target, target)` — no-op move.
            if node == target {
                return Ok(Some(node));
            }
            if !dom.insert_before(parent, node, target) {
                return Err(adjacent_hierarchy_error(method));
            }
            Ok(Some(node))
        }
        InsertAdjacentWhere::AfterBegin => {
            if let Some(first) = dom.children_iter(target).next() {
                // `target.insertBefore(first, first)` — no-op move.
                if node == first {
                    return Ok(Some(node));
                }
                if !dom.insert_before(target, node, first) {
                    return Err(adjacent_hierarchy_error(method));
                }
            } else if !dom.append_child(target, node) {
                return Err(adjacent_hierarchy_error(method));
            }
            Ok(Some(node))
        }
        InsertAdjacentWhere::BeforeEnd => {
            // No spec-allowed no-op here: `target.appendChild(target)`
            // is a genuine cycle and must fail.
            if !dom.append_child(target, node) {
                return Err(adjacent_hierarchy_error(method));
            }
            Ok(Some(node))
        }
        InsertAdjacentWhere::AfterEnd => {
            let Some(parent) = dom.get_parent(target) else {
                return Ok(None);
            };
            match dom.get_next_sibling(target) {
                Some(next) => {
                    // `parent.insertBefore(next, next)` — no-op move.
                    if node == next {
                        return Ok(Some(node));
                    }
                    if !dom.insert_before(parent, node, next) {
                        return Err(adjacent_hierarchy_error(method));
                    }
                }
                None => {
                    if !dom.append_child(parent, node) {
                        return Err(adjacent_hierarchy_error(method));
                    }
                }
            }
            Ok(Some(node))
        }
    }
}

/// `Element.prototype.insertAdjacentElement(where, element)` —
/// WHATWG DOM §4.9.
pub(super) fn native_element_insert_adjacent_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(target) = super::event_target::require_receiver(
        ctx,
        this,
        "Element",
        "insertAdjacentElement",
        |k| k == NodeKind::Element,
    )?
    else {
        return Ok(JsValue::Null);
    };
    let where_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let where_raw = super::super::coerce::to_string(ctx.vm, where_arg)?;
    let where_str = ctx.vm.strings.get_utf8(where_raw);
    let pos = parse_adjacent_position(&where_str)
        .ok_or_else(|| adjacent_syntax_error("insertAdjacentElement", &where_str))?;

    let element_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let node = require_element_arg(ctx, element_arg)?;
    // `node` is user-supplied — on `perform_adjacent_insert` failure
    // the caller still holds a JS handle to it, so we must NOT
    // destroy the entity here (that would invalidate live wrappers).
    match perform_adjacent_insert(ctx, target, node, pos, "insertAdjacentElement")? {
        Some(entity) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(entity))),
        None => Ok(JsValue::Null),
    }
}

/// `Element.prototype.insertAdjacentText(where, data)` —
/// WHATWG DOM §4.9.
pub(super) fn native_element_insert_adjacent_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(target) =
        super::event_target::require_receiver(ctx, this, "Element", "insertAdjacentText", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(JsValue::Undefined);
    };
    let where_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let where_raw = super::super::coerce::to_string(ctx.vm, where_arg)?;
    let where_str = ctx.vm.strings.get_utf8(where_raw);
    let pos = parse_adjacent_position(&where_str)
        .ok_or_else(|| adjacent_syntax_error("insertAdjacentText", &where_str))?;

    // Parent-less short-circuit: `beforebegin` / `afterend` require
    // the receiver to have a parent, and the spec treats the missing-
    // parent case as a silent no-op.  Check BEFORE allocating a Text
    // entity — otherwise the allocation leaks into the ECS because
    // no JS handle is returned and the entity never reaches GC.
    if position_requires_parent(pos) && ctx.host().dom().get_parent(target).is_none() {
        return Ok(JsValue::Undefined);
    }

    // `where` + parent-existence validity are already checked; allocate
    // the Text now.  WHATWG §4.9 step 2: the Text's node document is
    // *target's* node document, so thread the receiver's owner
    // document through the owner-aware creator (PR4f C2).
    let data_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let data_sid = super::super::coerce::to_string(ctx.vm, data_arg)?;
    let data = ctx.vm.strings.get_utf8(data_sid);
    let owner_doc = ctx.host().dom().owner_document(target);
    let text_entity = ctx.host().dom().create_text_with_owner(data, owner_doc);
    // Cycle / destroyed-receiver paths still fail inside
    // `perform_adjacent_insert` (parent exists but insertion is
    // otherwise invalid).  Destroy the unreferenced Text so the
    // error path does not leak an ECS entity — nothing outside this
    // function holds a handle to it.
    match perform_adjacent_insert(ctx, target, text_entity, pos, "insertAdjacentText") {
        Ok(_) => Ok(JsValue::Undefined),
        Err(e) => {
            let _ = ctx.host().dom().destroy_entity(text_entity);
            Err(e)
        }
    }
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
    // receiver should be a silent no-op.  Order matches
    // `querySelector*` / `matches` / `closest` in this same file.
    let Some(root) =
        super::event_target::require_receiver(ctx, this, "Element", "getElementsByTagName", |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(wrap_entities_as_array(ctx.vm, &[]));
    };
    let tag = coerce_first_arg_to_string(ctx, args)?;
    let entities = collect_descendants_by_tag_name(ctx.host().dom(), root, &tag);
    Ok(wrap_entities_as_array(ctx.vm, &entities))
}

/// `Element.prototype.getElementsByClassName(classNames)` —
/// WHATWG §4.2.6.2.  Descendant-only; empty-token-set yields an empty
/// array, and every class token must appear in the element's `class`
/// attribute (WHATWG "all classes in classes" AND semantics).
pub(super) fn native_element_get_elements_by_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Brand check before argument conversion — same WebIDL precedence
    // rule as `getElementsByTagName` above.
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
    let target_classes: Vec<&str> = class_str.split_whitespace().collect();
    let entities = collect_descendants_by_class_name(ctx.host().dom(), root, &target_classes);
    Ok(wrap_entities_as_array(ctx.vm, &entities))
}
