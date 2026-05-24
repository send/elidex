//! `CustomElementRegistry` interface (WHATWG HTML §4.13.4) — VM thin
//! binding to the engine-independent
//! [`elidex_custom_elements::CustomElementRegistry`] + reaction queue
//! drained at script-execution / event-dispatch checkpoints.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this module contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! WebIDL coercion (`observedAttributes` static getter → `Sequence<
//! DOMString>` → `Vec<String>`), DOMException mapping, and one-line
//! dispatch into the registry helpers. The validation / pending-
//! upgrade queue / state transitions live in
//! [`elidex_custom_elements`]; the shadow-including descendant walker
//! used by `customElements.upgrade(root)` lives in
//! [`elidex_dom_api::tree_nav::descendants_shadow_inclusive`].
//!
//! ## State storage
//!
//! Per-realm registry + reaction queue + `whenDefined` resolvers all
//! live on [`super::super::host_data::HostData`] (the binding-crate
//! exception (a) — per-VM identity handles) and are scrubbed on
//! `Vm::unbind`. See [`super::super::host_data::HostData::ce_registry`]
//! and siblings for the field-level rationale.
//!
//! The `CustomElementReactionConsumer` reads the same `ce_registry` +
//! `ce_reaction_queue` via cloned `Arc<Mutex<>>` handles plumbed in
//! `Vm::bind`.
//!
//! ## Lifecycle preconditions
//!
//! All `customElements.*` natives check `ctx.host_if_bound()` first
//! and silently no-op on retained references that outlived an
//! `unbind()` boundary — same convention as
//! [`super::mutation_observer`].

#![cfg(feature = "engine")]

pub(super) mod define;
pub(super) mod flush;
pub(super) mod lookup;
pub(super) mod upgrade;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `CustomElementRegistry.prototype` chained to
    /// `Object.prototype`, install the 4 method natives (`define` /
    /// `get` / `whenDefined` / `upgrade`), expose the
    /// `CustomElementRegistry` constructor stub on `globalThis`,
    /// eagerly construct the per-VM singleton, and install it as the
    /// `globalThis.customElements` data property.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean the
    /// call-order invariant from `register_globals()` was violated.
    pub(in crate::vm) fn register_custom_element_registry_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_custom_element_registry_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let wk = &self.well_known;
        let methods: [(_, NativeFn); 4] = [
            (wk.ce_define, define::native_ce_define as NativeFn),
            (wk.get, lookup::native_ce_get),
            (wk.ce_when_defined, lookup::native_ce_when_defined),
            (wk.ce_upgrade, lookup::native_ce_upgrade),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
        }

        self.custom_element_registry_prototype = Some(proto_id);

        // `CustomElementRegistry` constructor stub — throws on
        // call/construct per WebIDL §3.7 ("Illegal constructor" — HTML
        // §4.13.4 declares CustomElementRegistry with no exposed ctor). The
        // identifier still needs a global binding so `customElements
        // instanceof CustomElementRegistry` and `CustomElementRegistry
        // .prototype` parity work.
        let ctor = self.create_constructable_function(
            "CustomElementRegistry",
            native_ce_registry_illegal_ctor,
        );
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let ctor_name_sid = self.well_known.custom_element_registry_global;
        self.globals.insert(ctor_name_sid, JsValue::Object(ctor));

        // `globalThis.customElements` — install eagerly as a data
        // property, mirroring `globalThis.crypto`. Identity is stable
        // across reads via `custom_element_registry_instance`.
        let instance_id = self.alloc_or_cached_custom_element_registry();
        let cer_key = PropertyKey::String(self.well_known.custom_elements_accessor);
        self.define_shaped_property(
            self.global_object,
            cer_key,
            PropertyValue::Data(JsValue::Object(instance_id)),
            PropertyAttrs::WEBIDL_RO,
        );
    }

    /// Return the per-VM `CustomElementRegistry` singleton wrapper,
    /// allocating it on the first call. Re-allocates after
    /// `Vm::unbind` clears the slot.
    pub(in crate::vm) fn alloc_or_cached_custom_element_registry(&mut self) -> ObjectId {
        if let Some(id) = self.custom_element_registry_instance {
            return id;
        }
        let proto = self
            .custom_element_registry_prototype
            .expect("alloc_or_cached_custom_element_registry before register");
        let id = self.alloc_object(Object {
            kind: ObjectKind::CustomElementRegistry,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.custom_element_registry_instance = Some(id);
        id
    }
}

// ---------------------------------------------------------------------------
// Constructor stub — `new CustomElementRegistry()` throws per WebIDL §3.7
// (HTML §4.13.4: `CustomElementRegistry` has no exposed constructor)
// ---------------------------------------------------------------------------

fn native_ce_registry_illegal_ctor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error(
        "Failed to construct 'CustomElementRegistry': Illegal constructor",
    ))
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Confirm `this` is the `CustomElementRegistry` singleton. Returns
/// the canonical "Illegal invocation" TypeError otherwise — matches
/// every other `*.prototype.*` method's wording.
pub(super) fn require_ce_registry_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<(), VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'CustomElementRegistry': Illegal invocation"
        )));
    };
    if !matches!(
        ctx.vm.get_object(id).kind,
        ObjectKind::CustomElementRegistry
    ) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'CustomElementRegistry': Illegal invocation"
        )));
    }
    Ok(())
}
