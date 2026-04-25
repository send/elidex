//! VM heap allocation + object access methods on [`super::VmInner`].
//!
//! Split from [`super`] (the file `vm/mod.rs` itself) to keep
//! that file below the 1000-line convention (cleanup tranche 2).
//! `vm/mod.rs` retains the [`super::VmInner`] / [`super::Vm`]
//! struct definitions, the module-tree wiring, and the
//! workspace-level constants; this sibling holds the
//! `impl VmInner` block of allocation / promotion / receiver
//! coercion / object-access utility methods.
//!
//! ## Method groups
//!
//! - **Listener cleanup**:
//!   [`super::VmInner::remove_listener_and_prune_back_ref`] (engine-only)
//! - **Allocation**: [`super::VmInner::alloc_symbol`] /
//!   [`super::VmInner::alloc_object`] /
//!   [`super::VmInner::create_array_object`] /
//!   [`super::VmInner::create_string_wrapper`]
//! - **Receiver coercion + promotion**:
//!   [`super::VmInner::ensure_instance_or_alloc`] /
//!   [`super::VmInner::promote_to_string_wrapper`] /
//!   [`super::VmInner::promote_to_array`]
//! - **Object access**: [`super::VmInner::get_object`] /
//!   [`super::VmInner::get_object_mut`]

use super::shape;
use super::value::{self, JsValue, Object, ObjectId, ObjectKind, StringId, SymbolId, SymbolRecord};
use super::VmInner;

impl VmInner {
    /// Drop a `ListenerId` from `HostData::listener_store` AND prune
    /// any `AbortSignal` back-ref to it.
    ///
    /// This is the canonical retirement path — both
    /// `removeEventListener` and the `{once}` auto-removal that
    /// `event_dispatch` triggers via `Engine::remove_listener` route
    /// through this helper so the back-ref index stays bounded
    /// regardless of how the listener was retired.  Skipping the
    /// back-ref scrub would let `abort_listener_back_refs` and
    /// `abort_signal_states[…].bound_listener_removals` grow
    /// unbounded across `addEventListener({signal}, {once: true})`
    /// dispatch cycles.
    ///
    /// Engine-only: `abort_signal_states` /
    /// `abort_listener_back_refs` only exist behind the `engine`
    /// feature; without it, the helper just defers to
    /// `host_data.remove_listener`.
    #[cfg(feature = "engine")]
    pub(crate) fn remove_listener_and_prune_back_ref(
        &mut self,
        listener_id: elidex_script_session::ListenerId,
    ) {
        if let Some(host) = self.host_data.as_deref_mut() {
            host.remove_listener(listener_id);
        }
        if let Some(signal_id) = self.abort_listener_back_refs.remove(&listener_id) {
            if let Some(state) = self.abort_signal_states.get_mut(&signal_id) {
                state.bound_listener_removals.remove(&listener_id);
            }
        }
    }

    /// Allocate a new symbol, returning its `SymbolId`.
    pub(crate) fn alloc_symbol(&mut self, description: Option<StringId>) -> SymbolId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(SymbolRecord { description });
        id
    }

    /// Allocate an object, returning its `ObjectId`.
    ///
    /// May trigger a GC cycle if the allocation pressure threshold is exceeded.
    /// GC runs **before** the new object is placed in the heap, so the new
    /// object cannot be prematurely collected.
    /// Estimated byte cost per object allocation (struct size + inline overhead).
    const OBJECT_ALLOC_ESTIMATE: usize = std::mem::size_of::<Object>() + 64;

    pub(crate) fn alloc_object(&mut self, obj: Object) -> ObjectId {
        // GC trigger BEFORE insertion.  Callers must ensure that any
        // ObjectIds reachable only through `obj`'s fields (prototype,
        // array elements, property slots) are already rooted on the VM
        // stack or otherwise reachable from GC roots.  Prototype ObjectIds
        // from VmInner fields (e.g., `self.object_prototype`) are always
        // rooted.  For complex cases (e.g., `create_closure`, `do_new`),
        // callers temporarily push values onto the stack or disable GC.
        if self.gc_enabled
            && self
                .gc_bytes_since_last
                .saturating_add(Self::OBJECT_ALLOC_ESTIMATE)
                >= self.gc_threshold
        {
            self.collect_garbage();
        }
        // Increment AFTER potential GC so the current allocation is still
        // counted towards the next cycle's threshold.
        self.gc_bytes_since_last += Self::OBJECT_ALLOC_ESTIMATE;

        if let Some(idx) = self.free_objects.pop() {
            self.objects[idx as usize] = Some(obj);
            ObjectId(idx)
        } else {
            let id = ObjectId(self.objects.len() as u32);
            self.objects.push(Some(obj));
            id
        }
    }

    /// Resolve a constructor's receiver for both `new`-mode and
    /// call-mode invocations.
    ///
    /// - `new F(...)`: native dispatch sets `self.in_construct = true`
    ///   and `do_new` supplies a pre-allocated object receiver — we
    ///   must reuse `this` as-is so the constructor initializes the
    ///   same instance the caller will receive.
    /// - `F(...)` (call-mode): `in_construct = false`; allocate a
    ///   fresh Ordinary with `prototype`.  An explicit receiver
    ///   passed via `F.call(obj, ...)` / `F.apply(obj, ...)` is *not*
    ///   reused — spec §19.5.1.1 step 2 (OrdinaryCreateFromConstructor)
    ///   always yields a new object.
    ///
    /// Implements the "callable constructor" shape of §19.5.1.1
    /// step 1-2.
    pub(crate) fn ensure_instance_or_alloc(
        &mut self,
        this: JsValue,
        prototype: Option<ObjectId>,
    ) -> JsValue {
        if self.in_construct {
            if let JsValue::Object(_) = this {
                return this;
            }
        }
        let obj = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype,
            extensible: true,
        });
        JsValue::Object(obj)
    }

    /// Allocate an `ObjectKind::Array` with the standard prototype.
    pub(crate) fn create_array_object(&mut self, elements: Vec<JsValue>) -> ObjectId {
        // `alloc_object` can trigger GC *before* the new object is
        // inserted into `self.objects`.  At that point `elements` lives
        // only in the Rust-local `Object` struct — not a GC root — so
        // any `JsValue::Object` entries could be collected mid-call.
        // Push a temporary rooted copy onto the VM stack for the
        // allocation window; GC scans `self.stack`, so every element
        // stays alive.  After the new object is installed in
        // `self.objects`, its elements are reachable via the object
        // and the stack copy can go.
        let stack_root = self.stack.len();
        self.stack.extend_from_slice(&elements);
        let obj = self.alloc_object(Object {
            kind: ObjectKind::Array { elements },
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.array_prototype,
            extensible: true,
        });
        self.stack.truncate(stack_root);
        obj
    }

    /// Allocate a `StringWrapper` with `length` stored as a non-writable data
    /// property (immutable inner string → no accessor needed).
    pub(crate) fn create_string_wrapper(&mut self, sid: StringId) -> ObjectId {
        #[allow(clippy::cast_precision_loss)]
        let len = self.strings.get(sid).len() as f64;
        let obj = self.alloc_object(Object {
            kind: ObjectKind::StringWrapper(sid),
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.string_prototype,
            extensible: true,
        });
        self.install_string_wrapper_length(obj, len);
        obj
    }

    /// Promote an existing Ordinary instance (typically pre-allocated by
    /// `do_new` for a native constructor) into a StringWrapper in place,
    /// reusing the object slot to avoid a second allocation.
    pub(crate) fn promote_to_string_wrapper(&mut self, obj_id: ObjectId, sid: StringId) {
        #[allow(clippy::cast_precision_loss)]
        let len = self.strings.get(sid).len() as f64;
        {
            let obj = self.get_object_mut(obj_id);
            obj.kind = ObjectKind::StringWrapper(sid);
        }
        self.install_string_wrapper_length(obj_id, len);
    }

    /// Promote an existing Ordinary instance into an Array in place.  Same
    /// motivation as `promote_to_string_wrapper`: reuse the object slot
    /// pre-allocated by `do_new` instead of allocating a fresh array.
    pub(crate) fn promote_to_array(&mut self, obj_id: ObjectId, elements: Vec<JsValue>) {
        let obj = self.get_object_mut(obj_id);
        obj.kind = ObjectKind::Array { elements };
    }

    fn install_string_wrapper_length(&mut self, obj_id: ObjectId, len: f64) {
        let length_key = value::PropertyKey::String(self.well_known.length);
        self.define_shaped_property(
            obj_id,
            length_key,
            value::PropertyValue::Data(JsValue::Number(len)),
            shape::PropertyAttrs {
                writable: false,
                enumerable: false,
                configurable: false,
                is_accessor: false,
            },
        );
    }

    /// Get a reference to an object.
    ///
    /// # Panics
    /// Panics if the object has been freed.
    #[inline]
    pub(crate) fn get_object(&self, id: ObjectId) -> &Object {
        self.objects[id.0 as usize]
            .as_ref()
            .expect("object already freed")
    }

    /// Get a mutable reference to an object.
    ///
    /// # Panics
    /// Panics if the object has been freed.
    #[inline]
    pub(crate) fn get_object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.objects[id.0 as usize]
            .as_mut()
            .expect("object already freed")
    }
}
