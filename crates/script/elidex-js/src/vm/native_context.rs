//! Re-entrant native-function helper methods hanging off `NativeContext`.
//!
//! `NativeContext<'a>` itself is defined in `value.rs`; this file hosts
//! the convenience API it exposes to native builtins (intern, alloc_object,
//! call_function, host access, etc.).

use super::coerce;
use super::host_data;
use super::value::{self, JsValue, NativeContext, Object, ObjectId, StringId, VmError};

// ---------------------------------------------------------------------------
// Native function helpers
// ---------------------------------------------------------------------------

impl NativeContext<'_> {
    /// Intern a string from UTF-8.
    #[inline]
    pub fn intern(&mut self, s: &str) -> StringId {
        self.vm.strings.intern(s)
    }

    /// Intern a string from raw WTF-16 code units.
    #[inline]
    pub fn intern_utf16(&mut self, units: &[u16]) -> StringId {
        self.vm.strings.intern_utf16(units)
    }

    /// Look up an interned string as WTF-16.
    #[inline]
    pub fn get_u16(&self, id: StringId) -> &[u16] {
        self.vm.strings.get(id)
    }

    /// Look up an interned string as UTF-8 (lossy for lone surrogates).
    #[inline]
    pub fn get_utf8(&self, id: StringId) -> String {
        self.vm.strings.get_utf8(id)
    }

    /// Allocate an object.
    pub fn alloc_object(&mut self, obj: Object) -> ObjectId {
        self.vm.alloc_object(obj)
    }

    /// Get a reference to an object.
    #[inline]
    pub fn get_object(&self, id: ObjectId) -> &Object {
        self.vm.get_object(id)
    }

    /// Get a mutable reference to an object.
    #[inline]
    pub fn get_object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.vm.get_object_mut(id)
    }

    /// Convert a value to f64 using ES2020 ToNumber.
    /// Returns `Err(VmError)` for Symbol values (ES2020 §7.1.4).
    ///
    /// Takes `&mut self` because the §7.1.4 step 4 Object path delegates
    /// to `ToPrimitive(val, "number")`, which may invoke user-defined
    /// `valueOf` / `toString` and through them arbitrary JS.
    #[inline]
    pub fn to_number(&mut self, val: JsValue) -> Result<f64, VmError> {
        coerce::to_number(self.vm, val)
    }

    /// Convert a value to an interned string using ES2020 ToString.
    /// Returns `Err(VmError)` for Symbol values (ES2020 §7.1.12).
    #[inline]
    pub fn to_string_val(&mut self, val: JsValue) -> Result<StringId, VmError> {
        coerce::to_string(self.vm, val)
    }

    /// Convert a value to bool using ES2020 ToBoolean.
    #[inline]
    pub fn to_boolean(&self, val: JsValue) -> bool {
        coerce::to_boolean(self.vm, val)
    }

    /// Call a JS function object by `ObjectId` (e.g. getter/setter invoke).
    ///
    /// Enables native functions to call back into the VM for re-entrant
    /// execution (e.g. invoking accessor getters from `Object.values`).
    pub fn call_function(
        &mut self,
        callee: ObjectId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.vm.call(callee, this, args)
    }

    /// Call a value as a function (type-checked: must be an object).
    pub fn call_value(
        &mut self,
        callee: JsValue,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.vm.call_value(callee, this, args)
    }

    /// Resolve a `PropertyValue` slot to a `JsValue`, invoking the getter
    /// if the slot is an accessor.
    pub fn resolve_slot(
        &mut self,
        slot: value::PropertyValue,
        this: JsValue,
    ) -> Result<JsValue, VmError> {
        self.vm.resolve_slot(slot, this)
    }

    /// Perform a fresh `Get` (§7.3.1) on an object by `PropertyKey`.
    /// Looks up the property (own + prototype chain), resolves accessors.
    /// Returns `JsValue::Undefined` when the property does not exist.
    pub fn get_property_value(
        &mut self,
        obj_id: value::ObjectId,
        key: value::PropertyKey,
    ) -> Result<JsValue, VmError> {
        self.vm.get_property_value(obj_id, key)
    }

    /// Returns `true` if the current native function is being invoked via
    /// `[[Construct]]` (i.e. `new F(...)`).  Used by constructors to choose
    /// between wrapper-object and primitive return paths.
    #[inline]
    pub fn is_construct(&self) -> bool {
        self.vm.in_construct
    }

    /// Access the host data.
    ///
    /// # Panics
    ///
    /// Panics if no `HostData` has been installed on the VM.
    pub fn host(&mut self) -> &mut host_data::HostData {
        self.vm
            .host_data
            .as_deref_mut()
            .expect("NativeContext::host() called without HostData installed")
    }

    /// Access the host data, returning `None` if not installed.
    pub fn host_opt(&mut self) -> Option<&mut host_data::HostData> {
        self.vm.host_data.as_deref_mut()
    }

    /// Access the host data only when it is both installed **and
    /// currently bound** (i.e. `session`/`dom` pointers are valid).
    /// Returns `None` in all unbound states.
    ///
    /// Use this from native functions that need to perform a DOM
    /// operation via `host.dom()`: a post-unbind caller (JS retained a
    /// wrapper across an `unbind()` boundary) gets a safe `None`
    /// instead of panicking.
    pub fn host_if_bound(&mut self) -> Option<&mut host_data::HostData> {
        self.vm.host_data.as_deref_mut().filter(|h| h.is_bound())
    }

    /// Borrow the bound DOM (shared) and the string pool (exclusive)
    /// simultaneously via disjoint field projection on
    /// `VmInner`.
    ///
    /// Lets a native function call
    /// [`elidex_ecs::EcsDom::with_attribute`] (or any other
    /// `&EcsDom`-only accessor) to read a borrowed `&str` AND
    /// `pool.intern(s)` it within the same closure — without
    /// the `&ctx.host().dom()` / `&mut ctx.vm.strings.intern()`
    /// borrow conflict that the per-method `host()` accessor
    /// triggers.  Returns `None` when the VM is unbound (matches
    /// [`Self::host_if_bound`]'s contract so post-unbind callers
    /// can fall through without panicking).
    ///
    /// # Safety
    ///
    /// Implemented via `HostData::dom_shared` (the `&self` aliasing
    /// view of `dom_ptr`).  The returned `&EcsDom` aliases the
    /// exclusive borrow held by the `bind`-time pointer; callers
    /// must not invoke any sibling `host()` / `host().dom()` path
    /// (which would yield `&mut EcsDom`) while either of the
    /// returned references is live.  In practice this is
    /// straightforward: the closure form returns the projected
    /// `R` and drops both borrows before the call site continues.
    #[cfg(feature = "engine")]
    pub fn dom_and_strings_if_bound(
        &mut self,
    ) -> Option<(&elidex_ecs::EcsDom, &mut super::pools::StringPool)> {
        let vm = &mut *self.vm;
        let host = vm.host_data.as_deref().filter(|h| h.is_bound())?;
        Some((host.dom_shared(), &mut vm.strings))
    }

    /// Borrow the bound DOM (shared) and the live-collection state map
    /// (exclusive) simultaneously via disjoint field projection on
    /// `VmInner`.
    ///
    /// Mirrors [`Self::dom_and_strings_if_bound`] for the
    /// `live_collection_states` map: lets a native function pass
    /// `&EcsDom` to [`elidex_dom_api::LiveCollection::snapshot`]
    /// while keeping `&mut LiveCollection` borrowed out of the map,
    /// avoiding the `ctx.host()` / `ctx.vm.live_collection_states`
    /// aliasing conflict that an unsplit `&mut ctx` borrow would hit.
    ///
    /// # Safety
    ///
    /// Same `dom_ptr` aliasing contract as
    /// [`Self::dom_and_strings_if_bound`] — callers must not invoke
    /// any sibling `host()` / `host().dom()` path while either of
    /// the returned references is live.
    #[cfg(feature = "engine")]
    pub fn dom_and_collection_states_if_bound(
        &mut self,
    ) -> Option<(
        &elidex_ecs::EcsDom,
        &mut std::collections::HashMap<value::ObjectId, elidex_dom_api::LiveCollection>,
    )> {
        let vm = &mut *self.vm;
        let host = vm.host_data.as_deref().filter(|h| h.is_bound())?;
        Some((host.dom_shared(), &mut vm.live_collection_states))
    }

    /// `HasProperty` + `Get` (§7.3.1): returns `None` if the property does
    /// not exist anywhere on the prototype chain, `Some(value)` otherwise.
    pub fn try_get_property_value(
        &mut self,
        obj_id: value::ObjectId,
        key: value::PropertyKey,
    ) -> Result<Option<JsValue>, VmError> {
        let exists = coerce::get_property(self.vm, obj_id, key).is_some();
        if !exists {
            return Ok(None);
        }
        Ok(Some(self.vm.get_property_value(obj_id, key)?))
    }
}
