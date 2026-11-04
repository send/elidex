//! Property-install primitives shared by every `register_*` global installer:
//! `register_global_function`, `create_object_with_methods`,
//! `wire_constructor_global`, `install_methods`, the RO / RW accessor
//! installers, and the spec-level install gates (`installs` / `installs_dom`).
//!
//! Split from `globals.rs` (pure move) to keep that file under the project's
//! 1000-line convention.

use super::shape::{self, PropertyAttrs};
use super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::{NativeFn, VmInner};
#[cfg(feature = "engine")]
use elidex_plugin::{DomSpecLevel, WebApiSpecLevel};

impl VmInner {
    /// Helper: register a native function as a global.
    pub(super) fn register_global_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) {
        let fn_id = self.create_native_function(name, func);
        let name_id = self.strings.intern(name);
        self.globals.insert(name_id, JsValue::Object(fn_id));
    }

    /// Helper: create a global object with named native methods.
    pub(super) fn create_object_with_methods(
        &mut self,
        methods: &[(&str, NativeFn)],
    ) -> super::value::ObjectId {
        let obj_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });
        self.install_methods(obj_id, methods);
        obj_id
    }

    /// Wire an already-created constructor object into the global namespace:
    /// sets `Ctor.prototype = proto` (BUILTIN), `proto.constructor = Ctor`
    /// (METHOD), and installs `Ctor` as the `name` global. This is the shared
    /// wiring for the constructable primitive globals (String, Number, Boolean);
    /// the caller owns ctor creation (constructable vs plain) plus any
    /// statics/constants.
    pub(super) fn wire_constructor_global(
        &mut self,
        name: &str,
        ctor_id: super::value::ObjectId,
        proto_id: super::value::ObjectId,
    ) {
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor_id,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor_id)),
            PropertyAttrs::METHOD,
        );
        let name_id = self.strings.intern(name);
        self.globals.insert(name_id, JsValue::Object(ctor_id));
    }

    /// Install each `(name, native)` as a `DATA`/`METHOD`-attributed
    /// property on `obj_id`.  Shared by `create_object_with_methods`,
    /// the Window prototype registration (`host/window.rs`), and the
    /// per-bind document-method installer (`host/document.rs`) —
    /// everywhere we need to attach a batch of methods to an object
    /// that already exists.
    pub(crate) fn install_methods(
        &mut self,
        obj_id: super::value::ObjectId,
        methods: &[(&str, NativeFn)],
    ) {
        for &(name, func) in methods {
            let fn_id = self.create_native_function(name, func);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
    }

    /// Install each `(name, getter)` as a read-only accessor on
    /// `obj_id` with the WebIDL-default attrs (non-writable,
    /// enumerable, configurable).  The getter's WebIDL name
    /// (`"get foo"`) is derived from `name`.
    ///
    /// Engine-only — every call site lives behind `#[cfg(feature =
    /// "engine")]` (host globals: `location`, `history`, `window`,
    /// `document`).  Non-engine builds omit the helper entirely.
    #[cfg(feature = "engine")]
    pub(crate) fn install_ro_accessors(
        &mut self,
        obj_id: super::value::ObjectId,
        accessors: &[(&str, NativeFn)],
    ) {
        for &(name, getter) in accessors {
            let gid = self.create_native_function(&format!("get {name}"), getter);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    /// Install each `(name, getter, setter)` as an RW accessor pair
    /// with WebIDL-default attrs.  The underlying flags mirror
    /// `install_ro_accessors`; the `setter` slot is what makes the
    /// accessor writable.
    #[cfg(feature = "engine")]
    pub(crate) fn install_rw_accessors(
        &mut self,
        obj_id: super::value::ObjectId,
        accessors: &[(&str, NativeFn, NativeFn)],
    ) {
        for &(name, getter, setter) in accessors {
            let gid = self.create_native_function(&format!("get {name}"), getter);
            let sid = self.create_native_function(&format!("set {name}"), setter);
            let key = PropertyKey::String(self.strings.intern(name));
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: Some(sid),
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    /// Whether a Web API classified at `level` installs for this VM — the single
    /// **family-neutral** gate every demotable Web-API install seam consults
    /// (storage accessors / `Storage` / `StorageEvent` globals, `document.cookie`,
    /// `window.onstorage`). A1 routes every such site at [`WebApiSpecLevel::Modern`]
    /// so it installs in all modes — byte-identical to the pre-gate engine; A2/A3
    /// demote a family by flipping the level literal at its site to
    /// [`WebApiSpecLevel::Legacy`] (no per-site lockstep, no new gate — One issue,
    /// one way). Delegates to the engine-wide `elidex_plugin::SpecLevelPolicy`
    /// derived from the construction-fixed [`EngineMode`](elidex_plugin::EngineMode).
    #[cfg(feature = "engine")]
    pub(crate) fn installs(&self, level: WebApiSpecLevel) -> bool {
        self.spec_level_policy.installs(level)
    }

    /// DOM analogue of [`Self::installs`] — whether a DOM API classified at
    /// `level` installs. The `Document` live-collection getters route here at
    /// [`DomSpecLevel::Living`] (B0/B1 flip to [`DomSpecLevel::Legacy`]); the
    /// `DomApiHandler` registry applies the *same* policy at registration as
    /// dispatch-level defense-in-depth (`create_dom_registry_with_policy`).
    #[cfg(feature = "engine")]
    pub(crate) fn installs_dom(&self, level: DomSpecLevel) -> bool {
        self.spec_level_policy.installs_dom(level)
    }
}
