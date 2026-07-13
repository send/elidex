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
//! dispatch into the registry helpers. The validation /
//! state transitions live in
//! [`elidex_custom_elements`]; the shadow-including descendant walker
//! used by `customElements.upgrade(root)` lives in
//! [`elidex_dom_api::tree_nav::descendants_shadow_inclusive`].
//!
//! ## State storage
//!
//! Per-realm registry + reaction queue + `whenDefined` resolvers all
//! live on [`super::super::host_data::HostData`] (the binding-crate
//! exception (a) — per-VM identity handles). The **registry +
//! `whenDefined` resolvers + constructor maps are document-lifetime**:
//! cleared at `Vm::teardown_document`, so they SURVIVE a per-turn
//! (BATCH-BIND) `Vm::unbind` (`#11-per-batch-unbind-document-lifetime-state`).
//! The **reaction queue stays a per-turn scrub** (`Vm::unbind`) — it is a
//! transient checkpoint-drained queue holding `Entity` refs. See
//! [`super::super::host_data::HostData::ce_registry`] and siblings for the
//! field-level rationale.
//!
//! The `CustomElementReactionConsumer` reads the same `ce_registry` +
//! `ce_reaction_queue` via cloned `Arc<Mutex<>>` handles plumbed in
//! `Vm::bind`.
//!
//! ## Lifecycle preconditions
//!
//! `customElements.*` natives check `ctx.host_if_bound()` first and
//! silently no-op on retained references that outlived an `unbind()`
//! boundary — same convention as [`super::mutation_observer`]. One
//! exception: `whenDefined()` always returns a `Promise` because
//! script code expects a thenable shape; the unbound path returns a
//! REJECTED Promise rather than `undefined` so `then` chains still
//! type-check and route through the rejection handler.

#![cfg(feature = "engine")]

pub(super) mod creation;
pub(super) mod define;
pub(super) mod flush;
pub(super) mod html_element;
pub(super) mod lookup;
pub(super) mod records;
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

        // `CustomElementRegistry` declares no constructor operation, so
        // both call and construct throw "Illegal constructor" at the gate
        // (WebIDL §3.7.1 (Interface object) creation algorithm step 1.1;
        // HTML §4.13.4 declares CustomElementRegistry with no exposed ctor). The
        // identifier still needs a global binding so `customElements
        // instanceof CustomElementRegistry` and `CustomElementRegistry
        // .prototype` parity work.
        let ctor = self.create_illegal_constructor_function(
            "CustomElementRegistry",
            super::super::value::native_illegal_constructor_unreachable,
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
    /// allocating it on the first call. The slot is **realm-structural**
    /// (Codex #459 R3-1 + R4): once minted it is NEVER cleared — not on a
    /// per-turn `Vm::unbind` NOR at `Vm::teardown_document` — because
    /// `globalThis.customElements` is an install-once data property that keeps
    /// it rooted, and clearing the slot would let a rebind re-mint a duplicate
    /// and misclassify the page's own registry as `Foreign`. Only the backing
    /// `ce_registry` DATA is document-lifetime; after teardown the surviving
    /// wrapper reads an empty registry. Re-allocated lazily only on the first
    /// access of a freshly-constructed `Vm`.
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

/// Classification of a present `customElementRegistry` dictionary
/// member after WebIDL conversion — the member is NULLABLE
/// (`CustomElementRegistry?`) in both `ElementCreationOptions` and
/// `ShadowRootInit`, so explicit `null` is a present member carrying a
/// null registry, distinct from absence (`undefined`).
pub(super) enum RegistryMember {
    /// Explicit `null` — a null custom element registry.
    Null,
    /// The document's global registry singleton
    /// (`globalThis.customElements`).
    Document,
    /// A genuine `CustomElementRegistry` object that is NOT the
    /// document's singleton. Effectively unreachable today — the
    /// interface has no exposed constructor, so a second live registry
    /// cannot be minted, and the document's singleton wrapper now
    /// survives a `Vm::unbind`/rebind cycle (`custom_element_registry_
    /// instance` is document-lifetime, Codex #459 R3-1), so a retained
    /// `customElements` reference always re-classifies as `Document`.
    /// Kept for spec-completeness of the `CustomElementRegistry?` member.
    Foreign,
}

/// WebIDL conversion for a present (non-undefined)
/// `customElementRegistry` member: `null` and `CustomElementRegistry`
/// platform objects pass (classified for the caller's spec-step
/// sequencing), anything else is the standard conversion TypeError.
/// Dictionary members convert in lexicographic order, so this fires
/// before any algorithm step at both call sites — `createElement`'s
/// "flatten element creation options" (DOM §4.5) and `attachShadow`
/// (DOM §4.9 "Interface `Element`").
pub(super) fn convert_custom_element_registry_member(
    ctx: &mut NativeContext<'_>,
    raw: JsValue,
    prefix: &str,
) -> Result<RegistryMember, VmError> {
    match raw {
        JsValue::Null => Ok(RegistryMember::Null),
        JsValue::Object(id)
            if matches!(
                ctx.vm.get_object(id).kind,
                ObjectKind::CustomElementRegistry
            ) =>
        {
            if id == ctx.vm.alloc_or_cached_custom_element_registry() {
                Ok(RegistryMember::Document)
            } else {
                Ok(RegistryMember::Foreign)
            }
        }
        _ => Err(VmError::type_error(format!(
            "{prefix}: Failed to convert value to 'CustomElementRegistry'."
        ))),
    }
}

/// Shared registry-member acceptance gate: "flatten element creation
/// options" step 3.3 and `attachShadow` step 3 both throw
/// "NotSupportedError" for a **non-null**, non-scoped registry that is
/// not the document's custom element registry. A `null` member passes
/// — it is the spec's way to create a null-registry element / shadow
/// root (never upgraded); the caller threads
/// `RegistryMember::Null` through to the created node's
/// `RegistryAssociation`. This is an ALGORITHM step (after dictionary
/// conversion), so callers must run it after DOM §4.5 createElement
/// step 1's localName validation.
pub(super) fn reject_foreign_registry_member(
    ctx: &NativeContext<'_>,
    member: &RegistryMember,
    prefix: &str,
) -> Result<(), VmError> {
    if matches!(member, RegistryMember::Foreign) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_not_supported_error,
            format!(
                "{prefix}: the provided registry is not the document's custom element registry"
            ),
        ));
    }
    Ok(())
}
