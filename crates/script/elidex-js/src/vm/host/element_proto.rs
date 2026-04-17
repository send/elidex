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
