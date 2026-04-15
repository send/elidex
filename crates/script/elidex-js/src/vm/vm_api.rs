//! Public `Vm` API — thin wrappers that delegate to `VmInner`.
//!
//! Split out of `mod.rs` to keep that file under the project's
//! 1000-line convention.  All business logic lives in `VmInner`; this
//! file owns nothing but delegation.

use crate::bytecode::compiled::CompiledFunction;

use super::value::{self, FuncId, JsValue, Object, ObjectId, StringId, UpvalueId, VmError};
use super::{host_data, Vm};

impl Vm {
    // -- Public API: all delegate to VmInner --------------------------------

    /// Parse, compile, and execute JavaScript source code.
    pub fn eval(&mut self, source: &str) -> Result<JsValue, VmError> {
        self.inner.eval(source)
    }

    /// Load and execute a compiled script.
    pub fn run_script(
        &mut self,
        script: crate::bytecode::compiled::CompiledScript,
    ) -> Result<JsValue, VmError> {
        self.inner.run_script(script)
    }

    /// Call a JS function object with the given `this` and arguments.
    pub fn call(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.inner.call(func_obj_id, this, args)
    }

    /// Push `value` onto the VM stack as a temporary GC root and
    /// return an RAII guard that restores the stack on drop.
    ///
    /// Thin wrapper over [`VmInner::push_temp_root`] — see that for
    /// the rooting contract (RAII Drop + length/slot-identity
    /// asserts + panic-safe).
    ///
    /// Use this when an allocation has just produced a `JsValue` not
    /// yet reachable from any other root (a freshly created event
    /// object, a one-shot intermediate before being installed into a
    /// property, etc.) and you need it to survive a GC cycle
    /// triggered by user JS that runs while the guard is alive.
    ///
    /// ```rust,ignore
    /// let mut g = vm.push_temp_root(JsValue::Object(id));
    /// let _ = g.call(func_id, this, &[arg]);
    /// // g drops here; stack restored to pre-push length
    /// ```
    #[cfg(feature = "engine")]
    pub(crate) fn push_temp_root(&mut self, value: JsValue) -> super::VmTempRoot<'_> {
        self.inner.push_temp_root(value)
    }

    /// Intern a string, returning its `StringId`.
    #[inline]
    pub fn intern(&mut self, s: &str) -> StringId {
        self.inner.strings.intern(s)
    }

    /// Look up an interned string by its ID, returning WTF-16 code units.
    #[inline]
    pub fn get_string_u16(&self, id: StringId) -> &[u16] {
        self.inner.strings.get(id)
    }

    /// Look up an interned string by its ID, returning a UTF-8 `String`.
    #[inline]
    pub fn get_string(&self, id: StringId) -> String {
        self.inner.strings.get_utf8(id)
    }

    /// Allocate an object, returning its `ObjectId`.
    pub fn alloc_object(&mut self, obj: Object) -> ObjectId {
        self.inner.alloc_object(obj)
    }

    /// Get a reference to an object.
    #[inline]
    pub fn get_object(&self, id: ObjectId) -> &Object {
        self.inner.get_object(id)
    }

    /// Get a mutable reference to an object.
    #[inline]
    pub fn get_object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.inner.get_object_mut(id)
    }

    /// Register a compiled function in the VM, returning its `FuncId`.
    pub fn register_function(&mut self, func: CompiledFunction) -> FuncId {
        self.inner.register_function(func)
    }

    /// Get a reference to a compiled function.
    #[inline]
    pub fn get_compiled(&self, id: FuncId) -> &CompiledFunction {
        self.inner.get_compiled(id)
    }

    /// Allocate an upvalue, returning its `UpvalueId`.
    pub fn alloc_upvalue(&mut self, uv: value::Upvalue) -> UpvalueId {
        self.inner.alloc_upvalue(uv)
    }

    /// Install a `HostData` instance for browser shell integration.
    /// Call once, typically at `ElidexJsEngine` construction.
    ///
    /// # Panics
    ///
    /// Panics if a `HostData` is already installed, to prevent accidentally
    /// dropping caches (listener_store, wrapper_cache) from a prior bind.
    pub fn install_host_data(&mut self, hd: host_data::HostData) {
        assert!(
            self.inner.host_data.is_none(),
            "HostData already installed; use host_data() to access or a fresh Vm to reinstall"
        );
        self.inner.host_data = Some(Box::new(hd));
    }

    /// Access the host data (if installed).
    pub fn host_data(&mut self) -> Option<&mut host_data::HostData> {
        self.inner.host_data.as_deref_mut()
    }

    /// Bind host pointers for a JS execution call.  No-op if `HostData` is absent.
    ///
    /// # Safety
    ///
    /// See [`host_data::HostData::bind`]: pointers must remain valid (and not
    /// be aliased via any Rust reference) until `unbind()` is called.
    #[cfg(feature = "engine")]
    #[allow(unsafe_code)]
    pub unsafe fn bind(
        &mut self,
        session: *mut elidex_script_session::SessionCore,
        dom: *mut elidex_ecs::EcsDom,
        document: elidex_ecs::Entity,
    ) {
        if let Some(hd) = self.inner.host_data.as_deref_mut() {
            unsafe { hd.bind(session, dom, document) };
            // Refresh the `document` global so JS code (and listener
            // bodies) sees the just-bound document entity.  Wrapper
            // identity is preserved across bind/unbind cycles via
            // `HostData::wrapper_cache` — repeated binds with the
            // same document entity return the same ObjectId.
            self.install_document_global();
        }
    }

    /// Resolve an ECS `Entity` to its shared JS wrapper `ObjectId`,
    /// allocating on the first lookup and reusing the cached wrapper
    /// on every subsequent call.  See `vm/host/elements.rs` module
    /// doc for the identity contract.
    ///
    /// Public hook for benches and shell integration (PR6) that need
    /// to surface a DOM Entity to JS without routing through a full
    /// dispatch.  Requires `HostData` to be installed (the cache
    /// lives there).
    #[cfg(feature = "engine")]
    pub fn create_element_wrapper(&mut self, entity: elidex_ecs::Entity) -> ObjectId {
        self.inner.create_element_wrapper(entity)
    }

    /// Build a JS event object for a single listener invocation.
    /// Thin wrapper over the internal builder (see
    /// `vm/host/events.rs::create_event_object`) — exposed pub so
    /// benches and shell dispatch code can construct event objects
    /// outside of the internal dispatch loop.
    ///
    /// The caller must pass pre-resolved HostObject wrappers for
    /// target / currentTarget (use [`Vm::create_element_wrapper`]).
    /// Returned ObjectId is unrooted — root it immediately before
    /// any operation that may allocate or run user JS.
    #[cfg(feature = "engine")]
    pub fn create_event_object(
        &mut self,
        event: &elidex_script_session::event_dispatch::DispatchEvent,
        target: ObjectId,
        current_target: ObjectId,
        passive: bool,
    ) -> ObjectId {
        self.inner
            .create_event_object(event, target, current_target, passive)
    }

    /// Clear host pointers after JS execution.  No-op if unbound.
    pub fn unbind(&mut self) {
        if let Some(hd) = self.inner.host_data.as_deref_mut() {
            hd.unbind();
        }
    }

    /// Install a new global variable.
    ///
    /// Reusing a name is normally a bug — shell host globals and JS-visible
    /// built-ins must not collide — so this convenience method ignores any
    /// previous value.  Use [`Vm::set_global_checked`] if the caller needs
    /// to detect replacement explicitly.
    pub fn set_global(&mut self, name: &str, value: JsValue) {
        let _ = self.set_global_checked(name, value);
    }

    /// Install a new global variable and return the previous value, if any.
    pub fn set_global_checked(&mut self, name: &str, value: JsValue) -> Option<JsValue> {
        let id = self.inner.strings.intern(name);
        self.inner.globals.insert(id, value)
    }

    /// Get a global variable.
    pub fn get_global(&self, name: &str) -> Option<JsValue> {
        let sid = self.inner.strings.lookup(name)?;
        self.inner.globals.get(&sid).copied()
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}
