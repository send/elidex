//! `FormData` interface (WHATWG XHR §4.3).
//!
//! `FormData` is a WebIDL interface rooted at `Object` — not an
//! `EventTarget`, not a `Node`.  Prototype chain:
//!
//! ```text
//! FormData instance (ObjectKind::FormData, payload-free)
//!   → FormData.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! ## State storage
//!
//! Per-instance state lives **out-of-band** in
//! [`super::super::VmInner::form_data_states`], keyed by the
//! instance's own `ObjectId`.  The variant
//! [`super::super::value::ObjectKind::FormData`] is payload-free so
//! the per-variant size discipline of
//! [`super::super::value::ObjectKind`] is preserved.
//!
//! ## GC contract
//!
//! Each entry's value is either a `StringId` (pool-permanent) or a
//! `Blob` `ObjectId` (GC-managed).  The trace step in
//! `gc::trace::trace_work_list` (private to the [`super::super::gc`] module) walks the
//! `form_data_states` entry whose key is a marked FormData wrapper
//! and re-marks every `FormDataValue::Blob(id)` so the Blobs
//! survive as long as the FormData itself is reachable.  The sweep
//! tail prunes dead-key entries from `form_data_states`.
//!
//! ## Implemented
//!
//! - `new FormData(form?)` — the optional `form` argument is not
//!   yet processed with HTML's form-submission control walk
//!   (WHATWG HTML §5.4 — out of scope for this PR).  Current
//!   behaviour:
//!   - missing / `undefined` / `null` → empty FormData.
//!   - any `Object` (form element or otherwise) → empty FormData
//!     (matches an `<form>` with no submittable controls; the
//!     element's controls are not yet enumerated).
//!   - any primitive (number / boolean / string / Symbol / BigInt)
//!     → `TypeError`, matching the WebIDL `optional HTMLFormElement
//!     form` coercion of a non-Object operand.
//! - `.append(name, value, filename?)` /
//!   `.delete(name)` / `.get(name)` / `.getAll(name)` /
//!   `.has(name)` / `.set(name, value, filename?)`.
//! - `.forEach(callback, thisArg?)` / `.keys()` / `.values()` /
//!   `.entries()` / `[@@iterator]` (aliased to `.entries()`).

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    ArrayIterState, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Entry value kind.  `String` carries a `StringId`; `Blob` carries
/// the backing Blob's `ObjectId` so the multipart encoder can read
/// its bytes + MIME type via [`super::blob::blob_bytes`] /
/// [`super::blob::blob_type`].
#[derive(Debug, Clone, Copy)]
pub(crate) enum FormDataValue {
    String(StringId),
    Blob(ObjectId),
}

/// One row in [`super::super::VmInner::form_data_states`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct FormDataEntry {
    pub(crate) name: StringId,
    pub(crate) value: FormDataValue,
    /// Optional filename — only meaningful for [`FormDataValue::Blob`]
    /// entries.  When `None` for a Blob entry, the multipart encoder
    /// substitutes the well-known `"blob"` default per WHATWG XHR
    /// §4.3 step "create an entry".
    pub(crate) filename: Option<StringId>,
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `FormData.prototype`, install its method suite +
    /// `[Symbol.iterator]`, and expose the `FormData` constructor
    /// on `globals`.
    ///
    /// Called from `register_globals()` after `register_blob_global`
    /// so that the Blob fast path inside `append` / `set` can rely
    /// on a fully-installed `Blob.prototype`.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean
    /// `register_prototypes` was skipped or the wrong order.
    pub(in crate::vm) fn register_form_data_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_form_data_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_form_data_members(proto_id);
        self.form_data_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("FormData", native_form_data_constructor);
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
        let name_sid = self.well_known.form_data_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_form_data_members(&mut self, proto_id: ObjectId) {
        let wk = &self.well_known;
        let entries = [
            (wk.append, native_fd_append as NativeFn),
            (wk.delete_str, native_fd_delete as NativeFn),
            (wk.get, native_fd_get as NativeFn),
            (wk.get_all, native_fd_get_all as NativeFn),
            (wk.has, native_fd_has as NativeFn),
            (wk.set, native_fd_set as NativeFn),
            (wk.for_each, native_fd_for_each as NativeFn),
            (wk.keys, native_fd_keys as NativeFn),
            (wk.values, native_fd_values as NativeFn),
            (wk.entries, native_fd_entries as NativeFn),
        ];
        let entries_sid = wk.entries;
        let mut entries_fn_id: Option<ObjectId> = None;
        for (name_sid, func) in entries {
            let fn_id = self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
            if name_sid == entries_sid {
                entries_fn_id = Some(fn_id);
            }
        }
        let entries_fn_id = entries_fn_id.expect("entries method id not captured during install");

        // `FormData.prototype[Symbol.iterator] === .entries` —
        // WHATWG XHR §4.3 IDL `iterable<USVString, FormDataEntryValue>`
        // mirrors `Map.prototype` precedent.
        let sym_iter_key = PropertyKey::Symbol(self.well_known_symbols.iterator);
        self.define_shaped_property(
            proto_id,
            sym_iter_key,
            PropertyValue::Data(JsValue::Object(entries_fn_id)),
            PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new FormData(form?)` (WHATWG XHR §4.3).
///
/// `form` argument: when present, the spec requires walking the
/// `<form>` element's submittable controls.  This PR initialises
/// the FormData empty even with a form argument (matches
/// `<form>` with no submittable controls); the full traversal lands
/// with the form-submission integration.  Passing a non-Object /
/// non-`HostObject` argument throws TypeError per WebIDL.
fn native_form_data_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'FormData': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    // WebIDL: `optional HTMLFormElement form` — undefined / missing
    // is fine, anything else must be an Object (plain objects fail
    // silently here since the form-element walk is deferred; a
    // primitive like `new FormData("foo")` rejects with TypeError
    // to match WebIDL nullable-interface coercion).
    // TODO: walk `<form>` controls when the form-submission
    // integration lands.  Until then we install an empty
    // entry list for `JsValue::Object(_)` — matches `<form>` with
    // no submittable controls, observably correct for tests that
    // construct `new FormData(formEl)` and immediately append.
    match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined | JsValue::Null | JsValue::Object(_) => {}
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'FormData': parameter 1 is not of type 'HTMLFormElement'",
            ));
        }
    }

    ctx.vm.get_object_mut(id).kind = ObjectKind::FormData;
    ctx.vm.form_data_states.insert(id, Vec::new());
    Ok(JsValue::Object(id))
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

/// `append(name, value, filename?)` — WHATWG XHR §4.3.  Always
/// appends to the entry list (no replace-and-append semantics, that
/// is `set`'s job).
fn native_fd_append(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "append")?;
    let entry = build_entry(ctx, args, "append")?;
    if let Some(state) = ctx.vm.form_data_states.get_mut(&id) {
        state.push(entry);
    }
    Ok(JsValue::Undefined)
}

fn native_fd_delete(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "delete")?;
    let name_sid = take_name_arg(ctx, args, "delete")?;
    if let Some(state) = ctx.vm.form_data_states.get_mut(&id) {
        state.retain(|e| e.name != name_sid);
    }
    Ok(JsValue::Undefined)
}

fn native_fd_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "get")?;
    let name_sid = take_name_arg(ctx, args, "get")?;
    let found = ctx
        .vm
        .form_data_states
        .get(&id)
        .and_then(|s| s.iter().find(|e| e.name == name_sid).map(|e| e.value));
    Ok(match found {
        Some(FormDataValue::String(sid)) => JsValue::String(sid),
        Some(FormDataValue::Blob(blob_id)) => JsValue::Object(blob_id),
        None => JsValue::Null,
    })
}

fn native_fd_get_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "getAll")?;
    let name_sid = take_name_arg(ctx, args, "getAll")?;
    let values: Vec<JsValue> = ctx
        .vm
        .form_data_states
        .get(&id)
        .map(|s| {
            s.iter()
                .filter(|e| e.name == name_sid)
                .map(|e| value_to_js(e.value))
                .collect()
        })
        .unwrap_or_default();
    Ok(JsValue::Object(ctx.vm.create_array_object(values)))
}

fn native_fd_has(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "has")?;
    let name_sid = take_name_arg(ctx, args, "has")?;
    let present = ctx
        .vm
        .form_data_states
        .get(&id)
        .is_some_and(|s| s.iter().any(|e| e.name == name_sid));
    Ok(JsValue::Boolean(present))
}

/// `set(name, value, filename?)` — WHATWG XHR §4.3 step "set":
/// replace the *first* matching entry's value (and filename), drop
/// every subsequent matching entry, append if none matched.
fn native_fd_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "set")?;
    let new_entry = build_entry(ctx, args, "set")?;
    if let Some(state) = ctx.vm.form_data_states.get_mut(&id) {
        let mut replaced = false;
        state.retain_mut(|e| {
            if e.name != new_entry.name {
                return true;
            }
            if replaced {
                false
            } else {
                e.value = new_entry.value;
                e.filename = new_entry.filename;
                replaced = true;
                true
            }
        });
        if !replaced {
            state.push(new_entry);
        }
    }
    Ok(JsValue::Undefined)
}

fn native_fd_for_each(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "forEach")?;
    let callback = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Object(cb_id) if ctx.vm.get_object(cb_id).kind.is_callable() => cb_id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'forEach' on 'FormData': \
                 parameter 1 is not of type 'Function'.",
            ));
        }
    };
    let this_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let entries: Vec<FormDataEntry> = ctx
        .vm
        .form_data_states
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let fd_val = JsValue::Object(id);
    for entry in entries {
        ctx.call_function(
            callback,
            this_arg,
            &[
                value_to_js(entry.value),
                JsValue::String(entry.name),
                fd_val,
            ],
        )?;
    }
    Ok(JsValue::Undefined)
}

fn native_fd_keys(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "keys")?;
    let arr: Vec<JsValue> = ctx
        .vm
        .form_data_states
        .get(&id)
        .map(|s| s.iter().map(|e| JsValue::String(e.name)).collect())
        .unwrap_or_default();
    Ok(wrap_in_array_iterator(ctx, arr))
}

fn native_fd_values(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "values")?;
    let arr: Vec<JsValue> = ctx
        .vm
        .form_data_states
        .get(&id)
        .map(|s| s.iter().map(|e| value_to_js(e.value)).collect())
        .unwrap_or_default();
    Ok(wrap_in_array_iterator(ctx, arr))
}

fn native_fd_entries(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_fd_this(ctx, this, "entries")?;
    let entries: Vec<FormDataEntry> = ctx
        .vm
        .form_data_states
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let pairs: Vec<JsValue> = entries
        .into_iter()
        .map(|e| {
            JsValue::Object(
                ctx.vm
                    .create_array_object(vec![JsValue::String(e.name), value_to_js(e.value)]),
            )
        })
        .collect();
    Ok(wrap_in_array_iterator(ctx, pairs))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_fd_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "FormData.prototype.{method} called on non-FormData"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::FormData) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "FormData.prototype.{method} called on non-FormData"
        )))
    }
}

fn take_name_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    method: &str,
) -> Result<StringId, VmError> {
    if args.is_empty() {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'FormData': 1 argument required, but only 0 present."
        )));
    }
    super::super::coerce::to_string(ctx.vm, args[0])
}

/// Build a `FormDataEntry` from an `(name, value, filename?)`
/// argument tuple, applying WHATWG XHR §4.3 "create an entry":
/// - `name` is `ToString`-coerced.
/// - `value` is a `Blob` instance ⇒ kept as `FormDataValue::Blob`,
///   and `filename` (when supplied) is `ToString`-coerced.  The
///   default filename `"blob"` is supplied lazily by the multipart
///   encoder when no explicit filename was set.
/// - Otherwise `value` is `ToString`-coerced into a string entry,
///   and `filename` is ignored (spec: filename is only meaningful
///   for Blob/File values).
fn build_entry(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    method: &str,
) -> Result<FormDataEntry, VmError> {
    if args.len() < 2 {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'FormData': 2 arguments required, but only {} present.",
            args.len()
        )));
    }
    let name_sid = super::super::coerce::to_string(ctx.vm, args[0])?;
    let value_arg = args[1];

    let (value, filename) = match value_arg {
        JsValue::Object(obj_id) if matches!(ctx.vm.get_object(obj_id).kind, ObjectKind::Blob) => {
            let filename = match args.get(2).copied() {
                Some(JsValue::Undefined) | None => None,
                Some(v) => Some(super::super::coerce::to_string(ctx.vm, v)?),
            };
            (FormDataValue::Blob(obj_id), filename)
        }
        _ => {
            // Per spec, filename is silently dropped for non-Blob
            // values — matches Chromium / Firefox.
            let sid = super::super::coerce::to_string(ctx.vm, value_arg)?;
            (FormDataValue::String(sid), None)
        }
    };
    Ok(FormDataEntry {
        name: name_sid,
        value,
        filename,
    })
}

#[inline]
fn value_to_js(v: FormDataValue) -> JsValue {
    match v {
        FormDataValue::String(sid) => JsValue::String(sid),
        FormDataValue::Blob(id) => JsValue::Object(id),
    }
}

/// Same shape + GC contract as
/// [`super::headers::wrap_in_array_iterator`] / the URLSearchParams
/// counterpart.  See those modules' rationale for the temp-root
/// bookkeeping; duplicated here because the helper is `fn`-private
/// to its module.
fn wrap_in_array_iterator(ctx: &mut NativeContext<'_>, elements: Vec<JsValue>) -> JsValue {
    let arr_id = ctx.vm.create_array_object(elements);
    let iter_proto = ctx.vm.array_iterator_prototype;
    let mut rooted = ctx.vm.push_temp_root(JsValue::Object(arr_id));
    let iter_id = rooted.alloc_object(Object {
        kind: ObjectKind::ArrayIterator(ArrayIterState {
            array_id: arr_id,
            index: 0,
            kind: 0, // Values
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: iter_proto,
        extensible: true,
    });
    drop(rooted);
    JsValue::Object(iter_id)
}
