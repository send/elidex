//! `Element.prototype` intrinsic (WHATWG DOM §4.9).
//!
//! Holds Element-only members — tree navigation
//! (`parentElement`, `children`, `firstElementChild`, …), attribute
//! manipulation (`getAttribute`, `setAttribute`, …), and mutation
//! (`appendChild`, `removeChild`, …) that do not apply to Text or
//! Comment nodes.
//!
//! ## Prototype chain
//!
//! ```text
//! element wrapper (HostObject)
//!   → Element.prototype        (this intrinsic)
//!     → EventTarget.prototype  (PR3 C0 — includes Node-common accessors)
//!       → Object.prototype     (bootstrap)
//! ```
//!
//! Text and Comment wrappers skip `Element.prototype` — they chain
//! straight to `EventTarget.prototype`.  This keeps Element-specific
//! names off Text instances (`textNode.getAttribute` is `undefined`,
//! matching browsers).
//!
//! At C2 the prototype is allocated empty; per-feature methods are
//! installed by later PR4c commits (C3 tree nav, C4 attributes,
//! C5 mutation, C6 matches/closest).
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
use super::event_target::entity_from_this;

use elidex_ecs::{Entity, TagType};

impl VmInner {
    /// Allocate `Element.prototype` whose parent is
    /// `EventTarget.prototype`.
    ///
    /// Called from `register_globals()` after
    /// `register_event_target_prototype` — the latter's result is
    /// what the chain climbs to.
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` has not been populated
    /// (would mean `register_event_target_prototype` was skipped or
    /// called in the wrong order).
    pub(in crate::vm) fn register_element_prototype(&mut self) {
        let event_target_proto = self
            .event_target_prototype
            .expect("register_element_prototype called before register_event_target_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.element_prototype = Some(proto_id);
        self.install_element_tree_nav(proto_id);
        self.install_element_attributes(proto_id);
        self.install_element_mutation(proto_id);
    }

    /// Install Element-only tree-navigation accessors + `contains` /
    /// `hasChildNodes` methods on `proto_id` (= `Element.prototype`).
    fn install_element_tree_nav(&mut self, proto_id: ObjectId) {
        // Read-only accessors — every getter computes from live DOM
        // state so there is no data slot to cache the value in.
        for (name_sid, getter) in [
            (
                self.well_known.parent_element,
                native_element_get_parent_element as NativeFn,
            ),
            (self.well_known.first_child, native_element_get_first_child),
            (self.well_known.last_child, native_element_get_last_child),
            (
                self.well_known.first_element_child,
                native_element_get_first_element_child,
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
            (self.well_known.child_nodes, native_element_get_child_nodes),
            (self.well_known.children, native_element_get_children),
            (
                self.well_known.child_element_count,
                native_element_get_child_element_count,
            ),
            (
                self.well_known.is_connected,
                native_element_get_is_connected,
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
        // Methods.
        for (name_sid, func) in [
            (
                self.well_known.has_child_nodes,
                native_element_has_child_nodes as NativeFn,
            ),
            (self.well_known.contains, native_element_contains),
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
        let rw_attrs = shape::PropertyAttrs {
            writable: false,
            enumerable: true,
            configurable: true,
            is_accessor: true,
        };
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

    /// Install DOM-mutation methods (`appendChild`, `removeChild`,
    /// `insertBefore`, `replaceChild`, `remove`) on `proto_id`.
    fn install_element_mutation(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (
                self.well_known.append_child,
                native_element_append_child as NativeFn,
            ),
            (self.well_known.remove_child, native_element_remove_child),
            (self.well_known.insert_before, native_element_insert_before),
            (self.well_known.replace_child, native_element_replace_child),
            (self.well_known.remove, native_element_remove),
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

/// Wrap `Option<Entity>` as a wrapper JsValue, or Null.
fn wrap_or_null(ctx: &mut NativeContext<'_>, entity: Option<Entity>) -> JsValue {
    match entity {
        Some(e) => JsValue::Object(ctx.vm.create_element_wrapper(e)),
        None => JsValue::Null,
    }
}

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

fn native_element_get_parent_element(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let parent = {
        let dom = ctx.host().dom();
        match dom.get_parent(entity) {
            // parentElement returns parent only if it is itself an Element.
            Some(p) if dom.world().get::<&TagType>(p).is_ok() => Some(p),
            _ => None,
        }
    };
    Ok(wrap_or_null(ctx, parent))
}

fn native_element_get_first_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let child = ctx.host().dom().get_first_child(entity);
    Ok(wrap_or_null(ctx, child))
}

fn native_element_get_last_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let child = ctx.host().dom().get_last_child(entity);
    Ok(wrap_or_null(ctx, child))
}

fn native_element_get_first_element_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let child = ctx.host().dom().first_element_child(entity);
    Ok(wrap_or_null(ctx, child))
}

fn native_element_get_last_element_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let child = ctx.host().dom().last_element_child(entity);
    Ok(wrap_or_null(ctx, child))
}

fn native_element_get_next_element_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let sib = ctx.host().dom().next_element_sibling(entity);
    Ok(wrap_or_null(ctx, sib))
}

fn native_element_get_previous_element_sibling(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let sib = ctx.host().dom().prev_element_sibling(entity);
    Ok(wrap_or_null(ctx, sib))
}

fn native_element_get_child_nodes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    // Phase 2: return a plain JS array (static snapshot) rather than
    // a live NodeList.  Full NodeList semantics land with Observers
    // / CE lifecycle (PR5b).
    let children = collect_children(ctx, entity, /*elements_only=*/ false);
    let elements: Vec<JsValue> = children
        .into_iter()
        .map(|e| JsValue::Object(ctx.vm.create_element_wrapper(e)))
        .collect();
    Ok(JsValue::Object(ctx.vm.create_array_object(elements)))
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

fn native_element_get_is_connected(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    // WHATWG §4.4: connected iff shadow-including root is the document.
    // We approximate with the non-composed root: if the walk reaches
    // the bound `document_entity`, the node is connected.  Shadow
    // boundaries are handled by `find_tree_root`, which stops at a
    // shadow root — the shell's shadow-aware check will be layered
    // on top in PR5b when Custom Elements land.
    let dom = ctx.host().dom();
    let root = dom.find_tree_root_composed(entity);
    let connected = ctx
        .vm
        .host_data
        .as_deref()
        .and_then(|hd| hd.document_entity_opt())
        .is_some_and(|doc| root == doc);
    Ok(JsValue::Boolean(connected))
}

// ---------------------------------------------------------------------------
// Natives: hasChildNodes() / contains(other)
// ---------------------------------------------------------------------------

fn native_element_has_child_nodes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(
        ctx.host().dom().get_first_child(entity).is_some(),
    ))
}

fn native_element_contains(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(self_entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Boolean(false));
    };
    // WHATWG §4.4.2 contains(other):
    //   "returns true if other is an inclusive descendant of this,
    //    and false otherwise (including when other is null)."
    let other_entity = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Null | JsValue::Undefined => return Ok(JsValue::Boolean(false)),
        JsValue::Object(id) => match ctx.vm.get_object(id).kind {
            ObjectKind::HostObject { entity_bits } => match Entity::from_bits(entity_bits) {
                Some(e) => e,
                None => return Ok(JsValue::Boolean(false)),
            },
            _ => return Ok(JsValue::Boolean(false)),
        },
        _ => return Ok(JsValue::Boolean(false)),
    };
    if self_entity == other_entity {
        return Ok(JsValue::Boolean(true));
    }
    Ok(JsValue::Boolean(
        ctx.host()
            .dom()
            .is_ancestor_or_self(self_entity, other_entity),
    ))
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
fn attr_get(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) -> Option<String> {
    let dom = ctx.host().dom();
    dom.world()
        .get::<&elidex_ecs::Attributes>(entity)
        .ok()
        .and_then(|attrs| attrs.get(name).map(str::to_owned))
}

/// Set attribute `name` = `value` on `entity`, inserting an
/// `Attributes` component if one does not exist.  Returns `false`
/// when the entity has been destroyed.
fn attr_set(ctx: &mut NativeContext<'_>, entity: Entity, name: &str, value: String) -> bool {
    let dom = ctx.host().dom();
    let has_component = dom.world().get::<&elidex_ecs::Attributes>(entity).is_ok();
    if has_component {
        if let Ok(mut attrs) = dom.world_mut().get::<&mut elidex_ecs::Attributes>(entity) {
            attrs.set(name, value);
            return true;
        }
        return false;
    }
    let mut attrs = elidex_ecs::Attributes::default();
    attrs.set(name, value);
    dom.world_mut().insert_one(entity, attrs).is_ok()
}

fn attr_remove(ctx: &mut NativeContext<'_>, entity: Entity, name: &str) {
    let dom = ctx.host().dom();
    if let Ok(mut attrs) = dom.world_mut().get::<&mut elidex_ecs::Attributes>(entity) {
        attrs.remove(name);
    }
}

fn native_element_get_attribute(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Null);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let name = ctx.vm.strings.get_utf8(sid);
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
    let name_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let value_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let name_sid = super::super::coerce::to_string(ctx.vm, name_arg)?;
    let value_sid = super::super::coerce::to_string(ctx.vm, value_arg)?;
    let name = ctx.vm.strings.get_utf8(name_sid);
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
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let name = ctx.vm.strings.get_utf8(sid);
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
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let name = ctx.vm.strings.get_utf8(sid);
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
    let name_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let name_sid = super::super::coerce::to_string(ctx.vm, name_arg)?;
    let name = ctx.vm.strings.get_utf8(name_sid);

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

fn native_element_get_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    // WHATWG §4.9 reflected `id` / `className`: missing attribute
    // returns the empty string (not `null` like `getAttribute`).
    let val = attr_get(ctx, entity, "id").unwrap_or_default();
    if val.is_empty() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = ctx.vm.strings.intern(&val);
    Ok(JsValue::String(sid))
}

fn native_element_set_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let value = ctx.vm.strings.get_utf8(sid);
    attr_set(ctx, entity, "id", value);
    Ok(JsValue::Undefined)
}

fn native_element_get_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let val = attr_get(ctx, entity, "class").unwrap_or_default();
    if val.is_empty() {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    }
    let sid = ctx.vm.strings.intern(&val);
    Ok(JsValue::String(sid))
}

fn native_element_set_class_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, arg)?;
    let value = ctx.vm.strings.get_utf8(sid);
    attr_set(ctx, entity, "class", value);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Natives: DOM mutation (C5)
// ---------------------------------------------------------------------------

/// Extract an entity from a `JsValue` that is expected to be a DOM
/// node HostObject.  Returns an error with a WebIDL-style message
/// when the value is null / not an object / not a HostObject, matching
/// the spec-required `TypeError` for
/// `(new Document()).appendChild(1)` and friends.
fn require_node_arg(
    ctx: &NativeContext<'_>,
    value: JsValue,
    method: &str,
) -> Result<Entity, VmError> {
    let id = match value {
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(format!(
                "Failed to execute '{method}' on 'Node': parameter is not of type 'Node'."
            )));
        }
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::HostObject { entity_bits } => Entity::from_bits(entity_bits).ok_or_else(|| {
            VmError::type_error(format!(
                "Failed to execute '{method}' on 'Node': the node is detached (invalid entity)."
            ))
        }),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Node': parameter is not of type 'Node'."
        ))),
    }
}

fn native_element_append_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let child_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let child = require_node_arg(ctx, child_arg, "appendChild")?;
    let ok = ctx.host().dom().append_child(parent, child);
    if !ok {
        // WHATWG §4.5 pre-insertion validity — we model the lifecycle
        // violations EcsDom rejects (self-append, cycle, destroyed
        // entity) as HierarchyRequestError via TypeError with a
        // descriptive message.  Shell integrators that need the
        // spec-correct DOMException family wire it in PR5b.
        return Err(VmError::type_error(
            "Failed to execute 'appendChild' on 'Node': the new child element cannot be inserted.",
        ));
    }
    // Spec: return the appended node.
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(child)))
}

fn native_element_remove_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let child_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let child = require_node_arg(ctx, child_arg, "removeChild")?;
    let ok = ctx.host().dom().remove_child(parent, child);
    if !ok {
        // `NotFoundError` in the spec — surfaced as TypeError here
        // per the DOMException deferral above.
        return Err(VmError::type_error(
            "Failed to execute 'removeChild' on 'Node': \
             The node to be removed is not a child of this node.",
        ));
    }
    // Spec returns the removed node.
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(child)))
}

fn native_element_insert_before(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let new_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let new_node = require_node_arg(ctx, new_arg, "insertBefore")?;
    // `ref_node` may be `null` → append at end.
    let ref_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    match ref_arg {
        JsValue::Null | JsValue::Undefined => {
            if !ctx.host().dom().append_child(parent, new_node) {
                return Err(VmError::type_error(
                    "Failed to execute 'insertBefore' on 'Node': \
                     the new child element cannot be inserted.",
                ));
            }
        }
        _ => {
            let ref_node = require_node_arg(ctx, ref_arg, "insertBefore")?;
            if !ctx.host().dom().insert_before(parent, new_node, ref_node) {
                return Err(VmError::type_error(
                    "Failed to execute 'insertBefore' on 'Node': \
                     the reference node is not a child of this node.",
                ));
            }
        }
    }
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_node)))
}

fn native_element_replace_child(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(parent) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let new_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let new_node = require_node_arg(ctx, new_arg, "replaceChild")?;
    let old_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let old_node = require_node_arg(ctx, old_arg, "replaceChild")?;
    if !ctx.host().dom().replace_child(parent, new_node, old_node) {
        return Err(VmError::type_error(
            "Failed to execute 'replaceChild' on 'Node': \
             the node to be replaced is not a child of this node.",
        ));
    }
    // Spec: returns the *replaced* (old) node.
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(old_node)))
}

fn native_element_remove(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = entity_from_this(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    // WHATWG ChildNode §5.2.2 `remove()`: if the node has no parent,
    // do nothing.
    let dom = ctx.host().dom();
    if let Some(parent) = dom.get_parent(entity) {
        let _ = dom.remove_child(parent, entity);
    }
    Ok(JsValue::Undefined)
}
