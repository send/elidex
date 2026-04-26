//! `DocumentType.prototype` intrinsic (WHATWG DOM §4.7).
//!
//! Intermediate prototype layer for DocumentType wrappers:
//!
//! ```text
//! doctype wrapper
//!   → DocumentType.prototype     (this intrinsic)
//!     → Node.prototype
//!       → EventTarget.prototype
//!         → Object.prototype
//! ```
//!
//! Members:
//!
//! - `name` — DOCTYPE name (e.g. `"html"`).
//! - `publicId` — PUBLIC identifier, **empty string when absent**
//!   (WHATWG: non-nullable DOMString).
//! - `systemId` — SYSTEM identifier, same empty-string semantics.
//!
//! ChildNode mixin methods (`before` / `after` / `remove` /
//! `replaceWith`) are not installed here — WHATWG §4.7 DocumentType
//! does include the mixin, but the existing `childnode` / `parentnode`
//! mixin installers run on Element.prototype and CharacterData.prototype;
//! DocumentType routing is deferred to PR5b alongside HTMLCollection /
//! NamedNodeMap (see `m4-12-pr4f-plan` defer list).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{JsValue, NativeContext, Object, ObjectKind, PropertyStorage, VmError};
use super::super::VmInner;

use elidex_ecs::{DocTypeData, NodeKind};

impl VmInner {
    /// Allocate `DocumentType.prototype` with `Node.prototype` as its
    /// parent.  Must run after `register_node_prototype`.
    pub(in crate::vm) fn register_document_type_prototype(&mut self) {
        let parent = self
            .node_prototype
            .expect("register_document_type_prototype called before register_node_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.document_type_prototype = Some(proto_id);

        for (name_sid, getter) in [
            (
                self.well_known.name,
                native_doctype_get_name as super::super::NativeFn,
            ),
            (self.well_known.public_id, native_doctype_get_public_id),
            (self.well_known.system_id, native_doctype_get_system_id),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Natives (WHATWG §4.7 DocumentType)
// ---------------------------------------------------------------------------

/// `require_document_type_receiver` — brand check for DocumentType
/// wrappers.  Matches the receiver-validation pattern used elsewhere
/// (`require_receiver` with a kind filter); returned `None` means
/// unbound / non-HostObject receivers no-op with the field's default
/// (empty string).
fn require_document_type_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<elidex_ecs::Entity>, VmError> {
    super::event_target::require_receiver(ctx, this, "DocumentType", method, |k| {
        k == NodeKind::DocumentType
    })
}

fn read_doctype_field(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
    pick: impl FnOnce(&DocTypeData) -> String,
) -> Result<JsValue, VmError> {
    let Some(entity) = require_document_type_receiver(ctx, this, method)? else {
        return Ok(JsValue::String(ctx.vm.well_known.empty));
    };
    let value = {
        let dom = ctx.host().dom();
        dom.world()
            .get::<&DocTypeData>(entity)
            .ok()
            .map(|d| pick(&d))
    };
    match value {
        Some(v) => {
            let sid = ctx.vm.strings.intern(&v);
            Ok(JsValue::String(sid))
        }
        None => Ok(JsValue::String(ctx.vm.well_known.empty)),
    }
}

/// `DocumentType.prototype.name` — WHATWG §4.7.  Never null —
/// absent DOCTYPEs return an empty string.
pub(super) fn native_doctype_get_name(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    read_doctype_field(ctx, this, "name", |d| d.name.clone())
}

/// `DocumentType.prototype.publicId` — WHATWG §4.7.  Empty-string
/// sentinel for absence matches the DocTypeData default (`""`).
pub(super) fn native_doctype_get_public_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    read_doctype_field(ctx, this, "publicId", |d| d.public_id.clone())
}

/// `DocumentType.prototype.systemId` — WHATWG §4.7.  Empty-string
/// sentinel for absence.
pub(super) fn native_doctype_get_system_id(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    read_doctype_field(ctx, this, "systemId", |d| d.system_id.clone())
}
