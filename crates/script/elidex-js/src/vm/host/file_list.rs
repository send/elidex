//! `FileList` interface (File API §5).
//!
//! ```text
//! FileList instance (ObjectKind::FileList, payload-free)
//!   → FileList.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! ## Scope
//!
//! - `length` IDL readonly attr (unsigned long).
//! - `item(index)` method (out-of-range returns `null` per WebIDL
//!   nullable return).
//!
//! ## Indexed-property exotic — NOT implemented
//!
//! `fileList[0]` returns `undefined`; callers must use `.item(i)`.
//! Defer to slot `#11-filelist-indexed-exotic` keyed to the general
//! `#11-events-modern-indexed-exotic` infrastructure also pending for
//! `TouchList` / `DataTransferItemList` / `CSSRuleList`.  When the
//! indexed-exotic infra lands, FileList joins the same `[[Get]]`
//! dispatch path.
//!
//! ## Construction
//!
//! `FileList` is NOT exposed as a JS constructor — there is no
//! `new FileList()`.  Instances are allocated by VM-internal callers
//! (`<input type=file>.files` getter [Phase 3] / `DataTransfer.files`
//! [Phase 5]) via [`create_file_list_from_ids`].  The interface name
//! is bound on globals (as a brand-check identifier) but its
//! `[[Construct]]` slot throws TypeError.

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::events::install_ctor;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Per-`FileList` out-of-band state, keyed in
/// [`super::super::VmInner::file_list_data`] by the instance's
/// `ObjectId`.
///
/// Holds the ordered list of File wrapper `ObjectId`s.  Live mutation
/// of the underlying list is NOT supported per spec — FileList is
/// conceptually a snapshot, recreated by callers (e.g.
/// `<input type=file>.files` builds a fresh FileList per staged set;
/// `DataTransfer.files` builds from `dt_state.file_entries`).
#[derive(Debug, Default)]
pub(crate) struct FileListSideData {
    /// File wrapper `ObjectId`s in order.  GC trace fans out so each
    /// File (and transitively each backing Blob) survives while the
    /// FileList is reachable.
    pub(crate) file_ids: Vec<ObjectId>,
}

// ---------------------------------------------------------------------------
// Construction helper
// ---------------------------------------------------------------------------

/// Allocate a fresh `FileList` wrapper backed by the given file IDs.
/// Used by `<input type=file>.files` and `DataTransfer.files`.
///
/// `file_ids` must be non-FileList ObjectIds (typically `File`
/// instances).  Caller takes responsibility for ensuring those File
/// wrappers exist + the surrounding GC-rooting strategy keeps them
/// reachable through the FileList side-data fan-out.
pub(crate) fn create_file_list_from_ids(vm: &mut VmInner, file_ids: Vec<ObjectId>) -> ObjectId {
    let proto = vm.file_list_prototype;
    let id = vm.alloc_object(Object {
        kind: ObjectKind::FileList,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    vm.file_list_data.insert(id, FileListSideData { file_ids });
    id
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `FileList.prototype` (chains to `Object.prototype`)
    /// and install the `length` getter + `item(i)` method.
    ///
    /// The `FileList` global identifier is bound to a non-constructable
    /// native function whose `[[Construct]]` slot throws TypeError —
    /// matches Chrome / Firefox behaviour for the FileAPI §5 interface
    /// (no public ctor per IDL).
    pub(in crate::vm) fn register_file_list_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_file_list_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_file_list_members(proto_id);
        self.file_list_prototype = Some(proto_id);

        // FileList has no public IDL ctor — `new FileList()` throws
        // "Illegal constructor".  Use the shared `install_ctor` helper
        // for consistency with DataTransferItemList / TouchList.
        let global_sid = self.well_known.file_list_global;
        install_ctor(
            self,
            proto_id,
            "FileList",
            native_file_list_illegal_constructor,
            global_sid,
            super::super::value::CallShape::Ordinary,
        );
    }

    fn install_file_list_members(&mut self, proto_id: ObjectId) {
        let length_sid = self.well_known.length;
        self.install_accessor_pair(
            proto_id,
            length_sid,
            native_file_list_get_length as NativeFn,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        let item_sid: StringId = self.well_known.item;
        self.install_native_method(
            proto_id,
            item_sid,
            native_file_list_item as NativeFn,
            PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check + helpers
// ---------------------------------------------------------------------------

fn require_file_list_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "FileList.prototype.{method} called on non-FileList"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::FileList) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "FileList.prototype.{method} called on non-FileList"
        )))
    }
}

// ---------------------------------------------------------------------------
// Members
// ---------------------------------------------------------------------------

#[allow(clippy::needless_pass_by_value)]
fn native_file_list_illegal_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error("Illegal constructor"))
}

fn native_file_list_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_file_list_this(ctx, this, "length")?;
    #[allow(clippy::cast_precision_loss)]
    let len = ctx
        .vm
        .file_list_data
        .get(&id)
        .map_or(0, |d| d.file_ids.len()) as f64;
    Ok(JsValue::Number(len))
}

fn native_file_list_item(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_file_list_this(ctx, this, "item")?;
    let index_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    // WebIDL `unsigned long` ToUint32 per §3.10.10 — NaN / ±∞ → 0,
    // negatives wrap mod 2^32 (so `-1` → 0xFFFFFFFF).  Matches Chrome
    // / Firefox `fileList.item(NaN)` returning the index-0 entry on
    // non-empty lists rather than null.
    let n = super::super::coerce::to_number(ctx.vm, index_arg)?;
    let index = super::super::coerce::f64_to_uint32(n) as usize;
    let file_id_opt = ctx
        .vm
        .file_list_data
        .get(&id)
        .and_then(|d| d.file_ids.get(index).copied());
    Ok(file_id_opt.map_or(JsValue::Null, JsValue::Object))
}
