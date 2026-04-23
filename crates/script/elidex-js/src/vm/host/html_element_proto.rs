//! `HTMLElement.prototype` intrinsic — shared prototype for every
//! HTML-namespace element wrapper (WHATWG HTML §3.2.8).
//!
//! Chain:
//!
//! ```text
//! html element wrapper (e.g. <div>, <span>, <p>)
//!   → HTMLElement.prototype          (this intrinsic)
//!     → Element.prototype
//!       → Node.prototype
//!         → EventTarget.prototype
//!           → Object.prototype
//! ```
//!
//! Tag-specific prototypes (e.g. `HTMLIFrameElement.prototype`)
//! splice in between the wrapper and `HTMLElement.prototype`:
//!
//! ```text
//! iframe wrapper
//!   → HTMLIFrameElement.prototype
//!     → HTMLElement.prototype        (this intrinsic)
//!       → Element.prototype
//!         → …
//! ```
//!
//! Members installed here in this commit (PR5b §C1):
//!
//! - **`focus()`** / **`blur()`** — update `HostData::focused_entity`.
//!   Phase 2 simplification: no `FocusEvent` dispatch, no focusable-area
//!   check (follow-up commit adds `click()` which does dispatch).
//!
//! IDL attrs (accessKey / tabIndex / draggable / hidden / lang / dir /
//! title / translate / spellcheck / autocapitalize / inputMode /
//! enterKeyHint / nonce / contentEditable / isContentEditable /
//! autofocus) and `click()` install in follow-up commits (PR5b §C2).
//!
//! ## Receiver brand check
//!
//! `focus` / `blur` route through [`super::event_target::require_receiver`]
//! with a `PrototypeKind::Element` filter (every HTML-namespace element
//! inhabits `NodeKind::Element`; non-HTML XML elements are rejected at
//! `create_element_wrapper`'s dispatch step, never reaching this proto).
//!
//! ## GC contract
//!
//! The prototype itself is rooted via
//! [`VmInner::html_element_prototype`](super::super::VmInner::html_element_prototype)
//! in the `proto_roots` array (see `gc.rs`).  Installed method entries
//! carry `ObjectId` references to native function wrappers, which live
//! in `VmInner::objects` and are reachable from the prototype's shaped
//! storage — no separate root registration is needed.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `HTMLElement.prototype` with `Element.prototype` as
    /// its parent.  Must run after `register_element_prototype` and
    /// before `register_html_iframe_prototype` (which re-parents the
    /// iframe proto here).
    ///
    /// # Panics
    ///
    /// Panics if `element_prototype` has not been populated (would
    /// mean `register_element_prototype` was skipped or called in
    /// the wrong order).
    pub(in crate::vm) fn register_html_element_prototype(&mut self) {
        let parent = self
            .element_prototype
            .expect("register_html_element_prototype called before register_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_element_prototype = Some(proto_id);
        self.install_html_element_methods(proto_id);
    }

    /// Install `focus()` / `blur()` on `HTMLElement.prototype`.  Other
    /// methods (`click`) and the 16 IDL attrs land in follow-up
    /// commits — each as its own focused helper so review diffs stay
    /// narrow.
    fn install_html_element_methods(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (self.well_known.focus, native_html_element_focus as NativeFn),
            (self.well_known.blur, native_html_element_blur),
        ] {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                shape::PropertyAttrs::METHOD,
            );
        }
    }
}

/// `HTMLElement.prototype.focus()` (WHATWG HTML §6.7.1).
///
/// Phase 2 simplification: sets `HostData::focused_entity` without
/// running the "focusable area" algorithm (§6.6.3 step 3) or
/// dispatching `focus` / `focusin` / `blur` / `focusout` events.
/// Calling `.focus()` on a non-focusable element (disabled input,
/// content `tabindex` of -1 with no interactive intent) still marks
/// it as active — spec-polish land in a later tranche.
///
/// The `options` parameter (`{preventScroll, focusVisible}`) is
/// accepted and ignored; that too is spec polish.
fn native_html_element_focus(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Brand-check: receiver must be an HTML-namespace Element.
    // `require_receiver` returns `Ok(None)` when the receiver is
    // not a host-backed wrapper at all (e.g. a plain `{}`); WebIDL
    // brand checks for `HTMLElement` promote that to a synchronous
    // TypeError (browsers say "Illegal invocation") rather than a
    // silent no-op.
    let entity = require_html_element_receiver(ctx, this, "focus")?;
    if let Some(hd) = ctx.vm.host_data.as_deref_mut() {
        hd.set_focused_entity(entity);
    }
    Ok(JsValue::Undefined)
}

/// `HTMLElement.prototype.blur()` (WHATWG HTML §6.7.1).
///
/// Clears `HostData::focused_entity` **only if** the receiver is the
/// currently focused entity — blurring an unfocused element is a
/// no-op, matching browser behaviour.  No `blur` event dispatch in
/// Phase 2 (see `focus` rationale above).
fn native_html_element_blur(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "blur")?;
    if let Some(hd) = ctx.vm.host_data.as_deref_mut() {
        hd.invalidate_focus_if(entity);
    }
    Ok(JsValue::Undefined)
}

/// Brand-check for `HTMLElement.prototype` methods.  Wraps
/// [`super::event_target::require_receiver`] (which returns
/// `Ok(None)` for non-host receivers) and promotes the `None` case
/// to a TypeError so WebIDL brand semantics hold:
///
/// ```js
/// HTMLElement.prototype.focus.call({});          // TypeError
/// HTMLElement.prototype.focus.call(textNode);    // TypeError
/// HTMLElement.prototype.focus.call(divElement);  // OK
/// ```
fn require_html_element_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<elidex_ecs::Entity, VmError> {
    let entity_opt =
        super::event_target::require_receiver(ctx, this, "HTMLElement", method, |kind| {
            matches!(kind, elidex_ecs::NodeKind::Element)
        })?;
    entity_opt.ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLElement': Illegal invocation"
        ))
    })
}

/// Helper exposed to `document.rs` — read the currently focused
/// entity from `HostData`.  Returns `None` when no element is
/// focused **or** the focused entity has since been detached (the
/// detach hook clears focus before the entity is removed).
pub(super) fn focused_entity(ctx: &NativeContext<'_>) -> Option<elidex_ecs::Entity> {
    ctx.vm.host_data.as_deref()?.focused_entity()
}
