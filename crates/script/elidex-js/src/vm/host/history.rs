//! `history` global — a subset of the `History` interface
//! (WHATWG HTML §7.4).
//!
//! # Phase 2 scope
//!
//! All state lives on [`VmInner::navigation`] (see C3).  Methods
//! update the in-memory history stack synchronously; no `popstate`
//! firing, no structured-clone serialisation, and no shell-side
//! navigation — those land when the shell integration bridge ships
//! (PR6).  `scrollRestoration` is stubbed to `"auto"` for feature
//! detection parity.

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::VmInner;

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

pub(super) fn native_history_get_length(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `history.length` is a non-negative integer.  Clamp via `u32` to
    // satisfy clippy::cast_lossless and convert via `From<f64>`.
    let len = u32::try_from(ctx.vm.navigation.history_entries.len()).unwrap_or(u32::MAX);
    Ok(JsValue::Number(f64::from(len)))
}

pub(super) fn native_history_get_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let nav = &ctx.vm.navigation;
    Ok(nav
        .history_entries
        .get(nav.history_index)
        .map_or(JsValue::Null, |e| e.state))
}

pub(super) fn native_history_get_scroll_restoration(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 2: always `"auto"`.  A writable setter arrives with the
    // scroll-anchoring work in PR5+.
    let sid = ctx.vm.strings.intern("auto");
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// Navigation methods (back / forward / go)
// ---------------------------------------------------------------------------

/// Shared body for `back` / `forward` / `go` — advance the
/// `history_index` by `delta`, clamping to `[0, len)`.  Updates
/// `current_url` to match the resulting entry.  WHATWG HTML §7.4.2
/// says out-of-range deltas silently no-op (no throw, no scroll).
fn traverse(ctx: &mut NativeContext<'_>, delta: i64) {
    let nav = &mut ctx.vm.navigation;
    let Some(len) = i64::try_from(nav.history_entries.len()).ok() else {
        return;
    };
    let Some(cur) = i64::try_from(nav.history_index).ok() else {
        return;
    };
    let target = cur + delta;
    if target < 0 || target >= len {
        return;
    }
    let new_index = usize::try_from(target).unwrap_or(0);
    nav.history_index = new_index;
    if let Some(entry) = nav.history_entries.get(new_index) {
        nav.current_url = entry.url.clone();
    }
}

pub(super) fn native_history_back(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    traverse(ctx, -1);
    Ok(JsValue::Undefined)
}

pub(super) fn native_history_forward(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    traverse(ctx, 1);
    Ok(JsValue::Undefined)
}

pub(super) fn native_history_go(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // §7.4.2: `go(delta=0)` reloads.  With no shell-side reload, that
    // collapses to a no-op too (matches the `reload()` stub).
    let delta_f = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 0.0,
        other => coerce::to_number(ctx.vm, other)?,
    };
    #[allow(clippy::cast_possible_truncation)]
    let delta = if delta_f.is_finite() {
        delta_f.trunc() as i64
    } else {
        0
    };
    traverse(ctx, delta);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// State-mutation methods (pushState / replaceState)
// ---------------------------------------------------------------------------

/// Shared body for `pushState` / `replaceState`.
///
/// WHATWG HTML §7.4.3 requires the URL argument to parse against the
/// current document's URL (same-origin enforcement).  Phase 2 skips
/// the same-origin check — the shell will perform the check when it
/// owns the navigation.  We still accept `undefined` ⇒ keep current
/// URL unchanged (matches browsers).
fn state_mutate(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    replace_index: bool,
) -> Result<(), VmError> {
    let state = args.first().copied().unwrap_or(JsValue::Undefined);
    // `title` (args[1]) is ignored per §7.4.3 "title is intentionally
    // unused" — browsers collectively agreed to deprecate it.

    let url_arg = args.get(2).copied().unwrap_or(JsValue::Undefined);
    let new_url = if matches!(url_arg, JsValue::Undefined | JsValue::Null) {
        ctx.vm.navigation.current_url.clone()
    } else {
        let sid = coerce::to_string(ctx.vm, url_arg)?;
        ctx.vm.strings.get_utf8(sid)
    };

    let nav = &mut ctx.vm.navigation;
    if replace_index {
        if let Some(entry) = nav.history_entries.get_mut(nav.history_index) {
            entry.url.clone_from(&new_url);
            entry.state = state;
        }
    } else {
        // Push: truncate forward history, then append.
        nav.history_entries.truncate(nav.history_index + 1);
        nav.history_entries.push(super::navigation::HistoryEntry {
            url: new_url.clone(),
            state,
        });
        nav.history_index = nav.history_entries.len() - 1;
    }
    nav.current_url = new_url;
    Ok(())
}

pub(super) fn native_history_push_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    state_mutate(ctx, args, false)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_history_replace_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    state_mutate(ctx, args, true)?;
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Installation
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install `globalThis.history` (WHATWG HTML §7.4).
    pub(in crate::vm) fn register_history_global(&mut self) {
        let obj_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });

        // Methods.
        let methods: &[(&str, super::super::NativeFn)] = &[
            ("back", native_history_back),
            ("forward", native_history_forward),
            ("go", native_history_go),
            ("pushState", native_history_push_state),
            ("replaceState", native_history_replace_state),
        ];
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

        // Read-only accessors.
        let ro_accessors: &[(&str, super::super::NativeFn)] = &[
            ("length", native_history_get_length),
            ("state", native_history_get_state),
            ("scrollRestoration", native_history_get_scroll_restoration),
        ];
        for &(name, getter) in ro_accessors {
            let getter_name = format!("get {name}");
            let gid = self.create_native_function(&getter_name, getter);
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

        let name = self.well_known.history;
        self.globals.insert(name, JsValue::Object(obj_id));
    }
}
