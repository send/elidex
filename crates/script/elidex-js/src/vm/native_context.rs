//! Re-entrant native-function helper methods hanging off `NativeContext`.
//!
//! Extracted from `mod.rs` to keep that file under the 1000-line project
//! convention.  `NativeContext<'a>` itself is defined in `value.rs`; this
//! file hosts the convenience API it exposes to native builtins
//! (intern, alloc_object, call_function, host access, etc.).

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
    #[inline]
    pub fn to_number(&self, val: JsValue) -> Result<f64, VmError> {
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
