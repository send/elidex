// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! Shared NodeFilter callback dispatch — converts an opaque
//! `ObjectId` bits handle to a typed callback, invokes it with the
//! current node wrapper, applies WebIDL `ToUint16` coercion to the
//! return value, and parses via
//! [`NodeFilterResult::from_unsigned_short`].
//!
//! Used by [`super::tree_walker_proto`] and
//! [`super::node_iterator_proto`] for `acceptNode` dispatch.

#![cfg(feature = "engine")]

use elidex_dom_api::traversal::{FilterAction, FilterError, NodeFilterResult};
use elidex_ecs::Entity;

use super::super::value::{JsValue, NativeContext, ObjectId, PropertyKey, VmError};
use super::super::VmInner;

/// A `FilterAction` impl that drives a JS `NodeFilter` callback for
/// each visited node.
///
/// Lifetime-tied to the caller's mutable `NativeContext` borrow —
/// each `accept` call must be able to call into the VM, hence the
/// borrowed reference rather than an owned context.
///
/// `filter_id` is the opaque ObjectId bits of the JS callback, or
/// `None` for "no filter" (every node ACCEPTed without callback).
pub(super) struct JsFilter<'ctx, 'a> {
    pub(super) ctx: &'ctx mut NativeContext<'a>,
    pub(super) filter_id: Option<u64>,
    /// Pending VmError surfaced through `FilterError::Throw` —
    /// returned to the caller via [`Self::take_pending_error`] so
    /// the original JS exception propagates without being masked.
    pub(super) pending_error: Option<VmError>,
}

impl<'ctx, 'a> JsFilter<'ctx, 'a> {
    pub(super) fn new(ctx: &'ctx mut NativeContext<'a>, filter_id: Option<u64>) -> Self {
        Self {
            ctx,
            filter_id,
            pending_error: None,
        }
    }

    pub(super) fn take_pending_error(&mut self) -> Option<VmError> {
        self.pending_error.take()
    }
}

impl FilterAction for JsFilter<'_, '_> {
    fn accept(&mut self, node: Entity) -> Result<NodeFilterResult, FilterError> {
        // Null filter — every node Accept.
        let Some(filter_bits) = self.filter_id else {
            return Ok(NodeFilterResult::Accept);
        };
        // Decode opaque bits.
        let filter_obj = ObjectId(filter_bits as u32);
        // Allocate a wrapper for the node.  Wrappers identity-cache
        // by entity, so repeated visits return the same `ObjectId`.
        let wrapper = self.ctx.vm.create_element_wrapper(node);
        let arg = JsValue::Object(wrapper);

        // Try `acceptNode` member if filter is an object with that
        // method; otherwise treat the filter itself as callable.
        // Copilot R2: route through `get_property_value` so accessor
        // getters on `acceptNode` resolve per WebIDL §3.10 `Get`
        // semantics (rather than treating accessor descriptors as
        // non-callable).
        let callable = match pick_callable(self.ctx.vm, filter_obj) {
            Ok(Some(c)) => c,
            Err(e) => {
                self.pending_error = Some(e);
                return Err(FilterError::Throw);
            }
            Ok(None) => {
                self.pending_error =
                    Some(VmError::type_error("NodeFilter callback is not callable."));
                return Err(FilterError::Throw);
            }
        };

        let result_val = match self
            .ctx
            .call_function(callable, JsValue::Object(filter_obj), &[arg])
        {
            Ok(v) => v,
            Err(e) => {
                self.pending_error = Some(e);
                return Err(FilterError::Throw);
            }
        };

        // WebIDL `unsigned short` coercion + classify.
        let n = super::super::coerce::to_uint16(self.ctx.vm, result_val).map_err(|e| {
            self.pending_error = Some(e);
            FilterError::Throw
        })?;
        Ok(NodeFilterResult::from_unsigned_short(n))
    }
}

/// Pick the actual callable for a JS NodeFilter — either the object
/// itself (if a Function) or its `acceptNode` member.
///
/// Per WHATWG §6.3 the NodeFilter callback interface accepts either
/// shape; browsers tolerate function-instance filters directly.  Copilot
/// R2 fix: lookup goes through `get_property_value` so accessor
/// getters on `acceptNode` resolve per WebIDL §3.10 `Get` semantics.
/// Returns `Err` if the getter itself threw; `Ok(None)` if the lookup
/// yielded a non-callable value (or filter is a non-callable plain
/// object); `Ok(Some(callable))` otherwise.
fn pick_callable(vm: &mut VmInner, filter_obj: ObjectId) -> Result<Option<ObjectId>, VmError> {
    if vm.get_object(filter_obj).kind.is_callable() {
        return Ok(Some(filter_obj));
    }
    let key = PropertyKey::String(vm.well_known.accept_node_method);
    let value = vm.get_property_value(filter_obj, key)?;
    let JsValue::Object(method_id) = value else {
        return Ok(None);
    };
    if vm.get_object(method_id).kind.is_callable() {
        Ok(Some(method_id))
    } else {
        Ok(None)
    }
}
