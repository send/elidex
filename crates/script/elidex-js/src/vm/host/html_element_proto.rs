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
//! Members installed here (PR5b §C1 + §C2):
//!
//! - **Methods**: `focus()` / `blur()` — reconcile the canonical
//!   `ElementState::FOCUS` component (the single focus source of truth)
//!   via `elidex_dom_api::focus::{is_focusable, current_focus, set_focus_bit}`.
//!   No `FocusEvent` dispatch yet (a VM host method cannot fire a DOM event
//!   through the listener walk — slot `#11-vm-host-synthetic-dom-event-dispatch`,
//!   shared with `click()`).
//! - **Plain DOMString reflect**: `accessKey` / `lang` / `title` /
//!   `nonce`.
//! - **Enumerated (limited-to-known-values)**: `dir` (`ltr`/`rtl`/
//!   `auto`), `autocapitalize`, `inputMode`, `enterKeyHint`,
//!   `contentEditable` (`true`/`false`/`plaintext-only`/`inherit`).
//! - **Boolean presence**: `hidden` (tri-state with `"until-found"`
//!   surfacing as a DOMString), `autofocus`.
//! - **Boolean with attr value mapping**: `draggable` (per-element
//!   default for `<img>` / `<a href>` / `<area href>`), `translate`
//!   (`"yes"`/`"no"`, default true), `spellcheck` (`"true"`/`"false"`,
//!   default true).
//! - **Long with per-element default**: `tabIndex` — 0 for link /
//!   form-control / embed / contenteditable elements, -1 otherwise
//!   (WHATWG §6.6.3).
//! - **Readonly derived**: `isContentEditable` — walks ancestors for
//!   the first explicit `contenteditable` state.
//!
//! ## Receiver brand check
//!
//! Every installed accessor / method routes through
//! [`require_html_element_receiver`], which wraps
//! [`super::event_target::require_receiver`] with a
//! `NodeKind::Element` filter and **promotes the non-host-object
//! case to TypeError** (vanilla `require_receiver` silently no-ops
//! for `{}` receivers, which is wrong for the WebIDL brand semantics
//! HTMLElement attrs need).  Non-HTML XML elements are rejected at
//! `create_element_wrapper`'s dispatch step, never reaching this proto.
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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
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
        self.install_html_element_idl_attrs(proto_id);
        // `dataset` accessor — returns an identity-preserving
        // `DOMStringMap` wrapper backed by the element's `data-*`
        // attributes (WHATWG HTML §3.2.6).
        self.install_accessor_pair(
            proto_id,
            self.well_known.dataset,
            native_html_element_get_dataset,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `style` accessor — returns an identity-preserving Inline
        // `CSSStyleDeclaration` wrapper backed by the element's
        // `InlineStyle` ECS component (CSSOM §6.6).  Read-only: per
        // spec, `el.style = "color: red"` is a no-op (the style
        // attribute setter shape lives at the IDL layer below the
        // declaration block, and CSSOM does not surface a setter on
        // `Element.style` itself — `style.cssText = "..."` is the
        // intended write path).
        self.install_accessor_pair(
            proto_id,
            self.well_known.style,
            super::css_style_declaration::native_html_element_get_style,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // Event-handler IDL attributes (WHATWG HTML §8.1.8.2.1):
        // GlobalEventHandlers + DocumentAndElementEventHandlers
        // (`oncopy`/`oncut`/`onpaste`) are mixed into HTMLElement /
        // Element, so every HTML element inherits them here.
        // WindowEventHandlers are installed on Window / delegated from
        // `HTMLBodyElement.prototype` separately.
        self.install_event_handler_attrs(
            proto_id,
            &[
                elidex_script_session::HandlerScope::Global,
                elidex_script_session::HandlerScope::DocumentElement,
            ],
        );
    }

    /// Install `focus()` / `blur()` on `HTMLElement.prototype`.
    /// `click()` follows in the PR5b §C6 MouseEvent dispatch
    /// tranche.
    fn install_html_element_methods(&mut self, proto_id: ObjectId) {
        for (name_sid, func) in [
            (self.well_known.focus, native_html_element_focus as NativeFn),
            (self.well_known.blur, native_html_element_blur),
        ] {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
    }

    /// Install the 16 HTMLElement IDL attribute accessors (§3.2.8 /
    /// §6.6 / §6.7).  Read/write attrs install as `WEBIDL_RO_ACCESSOR`
    /// (accessor pair, configurable=true, enumerable=true matching
    /// WebIDL `[Unscopable]` defaults — same shape category as
    /// HTMLIFrameElement attrs).  The single read-only derived
    /// attribute (`isContentEditable`) installs with `setter: None`.
    fn install_html_element_idl_attrs(&mut self, proto_id: ObjectId) {
        // Plain DOMString reflect — getter returns the attribute
        // value or `""` when absent; setter writes `ToString(v)`.
        for (name_sid, getter, setter) in [
            (
                self.well_known.access_key,
                native_access_key_get as NativeFn,
                native_access_key_set as NativeFn,
            ),
            (self.well_known.lang, native_lang_get, native_lang_set),
            (self.well_known.title, native_title_get, native_title_set),
            (self.well_known.nonce, native_nonce_get, native_nonce_set),
            // Enumerated / limited-to-known-values — getter
            // canonicalises (empty / lowercase), setter is verbatim.
            (self.well_known.dir, native_dir_get, native_dir_set),
            (
                self.well_known.autocapitalize,
                native_autocapitalize_get,
                native_autocapitalize_set,
            ),
            (
                self.well_known.input_mode,
                native_input_mode_get,
                native_input_mode_set,
            ),
            (
                self.well_known.enter_key_hint,
                native_enter_key_hint_get,
                native_enter_key_hint_set,
            ),
            (
                self.well_known.content_editable,
                native_content_editable_get,
                native_content_editable_set,
            ),
            // Boolean reflect (attr presence ↔ IDL boolean).
            (self.well_known.hidden, native_hidden_get, native_hidden_set),
            (
                self.well_known.autofocus,
                native_autofocus_get,
                native_autofocus_set,
            ),
            // Boolean attrs driven by content value (not presence).
            (
                self.well_known.draggable,
                native_draggable_get,
                native_draggable_set,
            ),
            (
                self.well_known.translate,
                native_translate_get,
                native_translate_set,
            ),
            (
                self.well_known.spellcheck,
                native_spellcheck_get,
                native_spellcheck_set,
            ),
            // Long reflect with per-element default (link / form
            // controls / contenteditable → 0; others → −1).
            (
                self.well_known.tab_index,
                native_tab_index_get,
                native_tab_index_set,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
        // Read-only derived attribute — no backing content attr.
        self.install_accessor_pair(
            proto_id,
            self.well_known.is_content_editable,
            native_is_content_editable_get,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }
}

/// `HTMLElement.prototype.focus()` (WHATWG HTML §6.6.6 Focus management
/// APIs; the focusing steps are §6.6.4 Processing model).
///
/// Sets the canonical `ElementState::FOCUS` component (the single focus
/// source of truth) on the receiver via the engine-independent
/// `elidex_dom_api::focus::set_focus_bit`, which clears any prior holder
/// so the single-focus invariant holds by construction across the VM and
/// shell focus writers. Only a focusable area (§6.6.2, `is_focusable`)
/// receives focus; `.focus()` on a non-focusable element is a no-op.
///
/// Focus EVENT dispatch (`focus` / `focusin`) is deferred: a VM host
/// method cannot fire a DOM event through the 3-phase listener walk yet
/// (the same primitive that blocks `el.click()` — slot
/// `#11-vm-host-synthetic-dom-event-dispatch`). The same deferral covers the
/// OLD focused area's unfocusing steps when this `focus()` moves focus off a
/// previously-focused element: its `focusout` / `blur` and — for a user-edited
/// text control — the §4.10.5.5 change-on-blur event are NOT fired here (the
/// shell UA reconciler `content::focus::set_focus` dispatches those, but only
/// on its own input paths). The old element's `FocusValueSnapshot` is
/// deliberately left intact (not consumed) rather than discarded, so that once
/// the slot lands the deferred dispatch can still observe the focus-time value
/// and fire `change`. The `options` parameter (`{preventScroll, focusVisible}`)
/// is accepted and ignored (spec polish).
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
    let doc = ctx.host().document();
    let dom = ctx.host().dom();
    // Already this document's focused element ⇒ no-op (WHATWG HTML §6.6: focusing
    // the current focused area does nothing). Crucially this must return BEFORE
    // re-seeding the change-on-blur snapshot below: a redundant `focus()` after
    // the user has edited the control would otherwise refresh the snapshot
    // baseline to the edited value, suppressing the later §4.10.5.5 `change`
    // event. Mirrors the shell `set_focus` reconciler's `old == Some(entity)`
    // early return.
    if elidex_dom_api::focus::current_focus(dom, doc) == Some(entity) {
        return Ok(JsValue::Undefined);
    }
    // Focus is the *active* document's focused area (WHATWG HTML §6.6): only an
    // element in the bound document can take it. An element in a non-active
    // document — e.g. a `document.cloneNode()` subtree, which `is_connected`
    // reports connected because its root *is* a `Document` — must NOT steal the
    // live document's focus: `set_focus_bit`'s world-wide sweep would clear the
    // real holder and `blur()` could not restore it (`current_focus` is scoped
    // to the bound document). The active-document gate (`is_in_document`) is
    // engine-bound — which document is bound is the VM's fact — so it lives here
    // rather than in the engine-independent `is_focusable`.
    if elidex_dom_api::focus::is_focusable(dom, entity)
        && elidex_dom_api::focus::is_in_document(dom, entity, doc)
    {
        elidex_dom_api::focus::set_focus_bit(dom, Some(entity));
        // Seed the change-on-blur snapshot for text controls, just like the
        // shell `set_focus` reconciler — so a script `input.focus()` followed
        // by user editing + a user blur still fires the HTML §4.10.5.5 `change`
        // event (the snapshot follows the canonical FOCUS bit, not only the UA
        // path).
        elidex_form::record_focus_snapshot(dom, entity);
    }
    Ok(JsValue::Undefined)
}

/// `HTMLElement.prototype.dataset` getter — return an
/// identity-preserving `DOMStringMap` wrapper backed by the
/// element's `data-*` attributes (WHATWG HTML §3.2.6).  Repeated
/// reads return the same `ObjectId` via
/// [`crate::vm::VmInner::alloc_or_cached_dataset`].
fn native_html_element_get_dataset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "dataset")?;
    let id = ctx.vm.alloc_or_cached_dataset(entity);
    Ok(JsValue::Object(id))
}

/// `HTMLElement.prototype.blur()` (WHATWG HTML §6.6.6 Focus management
/// APIs; the unfocusing steps are §6.6.4 Processing model).
///
/// Delegates to the engine-independent [`elidex_dom_api::focus::blur`], which
/// clears the `ElementState::FOCUS` bit iff the receiver is the **raw** focus
/// holder (the focus SoT). Blurring an unfocused element is a no-op. Operating on
/// the raw holder makes `el.focus(); el.hidden = true; el.blur()` actually
/// unfocus `el` even though the same-turn `hidden` only schedules the
/// *asynchronous* render-time fixup (the bit lingers, so `el` is still the
/// focused area when `blur()` runs); the explicit clear stops a later un-hide
/// from resurrecting `document.activeElement` (Codex S2 R6). No `blur` /
/// `focusout` event dispatch yet (deferred with `focus`;
/// slot `#11-vm-host-synthetic-dom-event-dispatch`).
fn native_html_element_blur(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "blur")?;
    let dom = ctx.host().dom();
    elidex_dom_api::focus::blur(dom, entity);
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

// =========================================================================
// IDL attributes — WHATWG HTML §3.2.8 / §6.6 / §6.7
// =========================================================================
//
// Accessor naming convention: `native_<idl_property>_get` /
// `native_<idl_property>_set`.  All receivers go through
// [`require_html_element_receiver`] so `.call({})` uniformly throws
// TypeError.  Empty-string return on missing attr matches the DOMString
// reflect semantics every HTMLElement attr shares.

/// Plain DOMString reflect — getter returns attr value or `""`.
///
/// Used for: `accessKey` / `lang` / `title` / `nonce`.
fn string_reflect_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    idl_name: &'static str,
    attr_name: &'static str,
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, idl_name)?;
    let empty = ctx.vm.well_known.empty;
    // Snapshot `well_known.empty` first, then split the dom + strings
    // borrow so `with_attribute`'s closure can intern the borrowed
    // `&str` without the per-method `host()` / `vm.strings` borrow
    // conflict — saves one `String::from` clone per call.
    // `require_html_element_receiver` already promotes the unbound
    // case to TypeError above, so the `None` arm of
    // `dom_and_strings_if_bound` is a defensive fallback only.
    let sid = match ctx.dom_and_strings_if_bound() {
        Some((dom, strings)) => dom.with_attribute(entity, attr_name, |v| {
            v.map_or(empty, |s| strings.intern(s))
        }),
        None => empty,
    };
    Ok(JsValue::String(sid))
}

/// Plain DOMString reflect setter — writes `ToString(v)`.
fn string_reflect_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    idl_name: &'static str,
    attr_name: &'static str,
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, idl_name)?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    ctx.host().dom().set_attribute(entity, attr_name, &s);
    Ok(JsValue::Undefined)
}

/// Limited-to-known-values DOMString reflect — getter lowercases
/// and returns the attribute value when it matches one of
/// `allowed`, otherwise `default` (typically `""`).  Setter is
/// verbatim (spec §3.2.8.1: "On setting, the content attribute
/// must be set to the specified value", i.e. no validation).
fn enumerated_reflect_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    idl_name: &'static str,
    attr_name: &'static str,
    allowed: &[&str],
    default: &'static str,
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, idl_name)?;
    let resolved = ctx
        .host()
        .dom()
        .with_attribute(entity, attr_name, |raw| {
            raw.map(str::to_ascii_lowercase)
                .filter(|v| allowed.iter().any(|a| a == v))
        })
        .unwrap_or_else(|| default.to_string());
    let sid = if resolved.is_empty() {
        ctx.vm.well_known.empty
    } else {
        ctx.vm.strings.intern(&resolved)
    };
    Ok(JsValue::String(sid))
}

// ---- accessKey ----
fn native_access_key_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_get(ctx, this, "accessKey", "accesskey")
}
fn native_access_key_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "accessKey", "accesskey")
}

// ---- lang ----
fn native_lang_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_get(ctx, this, "lang", "lang")
}
fn native_lang_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "lang", "lang")
}

// ---- title ----
fn native_title_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_get(ctx, this, "title", "title")
}
fn native_title_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "title", "title")
}

// ---- nonce ----
fn native_nonce_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_get(ctx, this, "nonce", "nonce")
}
fn native_nonce_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "nonce", "nonce")
}

// ---- dir (limited to ltr/rtl/auto) ----
//
// WHATWG §3.2.8.1: getter returns the canonical form when attr is
// set to one of the known values, otherwise `""`.  Setter stores
// verbatim.
fn native_dir_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enumerated_reflect_get(ctx, this, "dir", "dir", &["ltr", "rtl", "auto"], "")
}
fn native_dir_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "dir", "dir")
}

// ---- autocapitalize ----
//
// WHATWG §6.8.7: allowed values `off` / `none` / `on` / `sentences`
// / `words` / `characters`; `off` is the canonical form of the
// "none" state (getter returns `"none"`) — kept as a separate
// recognised token below and canonicalised via the `none` fallback.
fn native_autocapitalize_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enumerated_reflect_get(
        ctx,
        this,
        "autocapitalize",
        "autocapitalize",
        &["off", "none", "on", "sentences", "words", "characters"],
        "",
    )
}
fn native_autocapitalize_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "autocapitalize", "autocapitalize")
}

// ---- inputMode ----
fn native_input_mode_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enumerated_reflect_get(
        ctx,
        this,
        "inputMode",
        "inputmode",
        &[
            "none", "text", "tel", "url", "email", "numeric", "decimal", "search",
        ],
        "",
    )
}
fn native_input_mode_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "inputMode", "inputmode")
}

// ---- enterKeyHint ----
fn native_enter_key_hint_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enumerated_reflect_get(
        ctx,
        this,
        "enterKeyHint",
        "enterkeyhint",
        &["enter", "done", "go", "next", "previous", "search", "send"],
        "",
    )
}
fn native_enter_key_hint_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "enterKeyHint", "enterkeyhint")
}

// ---- contentEditable / isContentEditable ----
//
// WHATWG §6.7.3: `contentEditable` is a DOMString enumerated
// reflecting the content attribute; missing value is `"inherit"`,
// invalid is also treated as the missing-state per spec (§6.7.3.2).
// `isContentEditable` is a readonly boolean that resolves the
// effective state by walking ancestors — spec inherits from
// `<html>` (which defaults to `false`).
fn native_content_editable_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enumerated_reflect_get(
        ctx,
        this,
        "contentEditable",
        "contenteditable",
        &["true", "false", "plaintext-only", "inherit"],
        "inherit",
    )
}
fn native_content_editable_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    string_reflect_set(ctx, this, args, "contentEditable", "contenteditable")
}

fn native_is_content_editable_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "isContentEditable")?;
    // HTML §6.7.3 ancestor-walk algorithm hoisted to elidex-dom-api
    // per the architectural Layering mandate (slot
    // `#11-tags-T1-v2-drift-hoist` D-5).
    let value = elidex_dom_api::element::is_content_editable(ctx.host().dom(), entity);
    Ok(JsValue::Boolean(value))
}

// ---- hidden (tri-state) ----
//
// WHATWG §6.6: IDL type is `(boolean or DOMString)`.  Getter returns
// `true` when the content attribute is present (except `until-found`
// which surfaces as the string `"until-found"`), `false` when
// absent.  Setter accepts `true` / `false` / `"until-found"` /
// `""` — any non-string non-boolean coerces via ToBoolean.
fn native_hidden_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    enum HiddenKind {
        Absent,
        Present,
        UntilFound,
    }
    let entity = require_html_element_receiver(ctx, this, "hidden")?;
    let kind = ctx
        .host()
        .dom()
        .with_attribute(entity, "hidden", |v| match v {
            None => HiddenKind::Absent,
            Some(s) if s.eq_ignore_ascii_case("until-found") => HiddenKind::UntilFound,
            Some(_) => HiddenKind::Present,
        });
    Ok(match kind {
        HiddenKind::Absent => JsValue::Boolean(false),
        HiddenKind::UntilFound => JsValue::String(ctx.vm.strings.intern("until-found")),
        HiddenKind::Present => JsValue::Boolean(true),
    })
}
fn native_hidden_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "hidden")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    // Distinguish `"until-found"` from other strings so that
    // `el.hidden = "until-found"` sets the content attribute to
    // that literal rather than the empty string (presence-only).
    if let JsValue::String(sid) = val {
        let s = ctx.vm.strings.get_utf8(sid);
        if s.eq_ignore_ascii_case("until-found") {
            ctx.host()
                .dom()
                .set_attribute(entity, "hidden", "until-found");
            return Ok(JsValue::Undefined);
        }
        if s.is_empty() {
            super::element_attrs::attr_remove(ctx, entity, "hidden");
            return Ok(JsValue::Undefined);
        }
    }
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host().dom().set_attribute(entity, "hidden", "");
    } else {
        super::element_attrs::attr_remove(ctx, entity, "hidden");
    }
    Ok(JsValue::Undefined)
}

// ---- autofocus (boolean reflect, presence = true) ----
fn native_autofocus_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "autofocus")?;
    Ok(JsValue::Boolean(
        ctx.host().dom().has_attribute(entity, "autofocus"),
    ))
}
fn native_autofocus_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "autofocus")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    if super::super::coerce::to_boolean(ctx.vm, val) {
        ctx.host().dom().set_attribute(entity, "autofocus", "");
    } else {
        super::element_attrs::attr_remove(ctx, entity, "autofocus");
    }
    Ok(JsValue::Undefined)
}

// ---- draggable (plain boolean IDL over tri-state content attr) ----
//
// WHATWG §6.11.1: IDL getter returns `true` if content is `"true"`,
// `false` if `"false"`, otherwise per-element default.  Setter
// writes `"true"` or `"false"` (never `"auto"` — spec-defined).
fn native_draggable_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "draggable")?;
    let dom = ctx.host().dom();
    let result = dom.with_attribute(entity, "draggable", |raw| {
        match raw.map(str::to_ascii_lowercase).as_deref() {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        }
    });
    let result = result.unwrap_or_else(|| draggable_default_for(dom, entity));
    Ok(JsValue::Boolean(result))
}
fn native_draggable_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "draggable")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let literal = if super::super::coerce::to_boolean(ctx.vm, val) {
        "true"
    } else {
        "false"
    };
    ctx.host().dom().set_attribute(entity, "draggable", literal);
    Ok(JsValue::Undefined)
}

/// Per-element `draggable` default (WHATWG §6.11.1 step 4).
/// `<img>` and `<a href>` default to true; everything else false.
fn draggable_default_for(dom: &elidex_ecs::EcsDom, entity: elidex_ecs::Entity) -> bool {
    dom.with_tag_name(entity, |tag| match tag {
        Some(t) if t.eq_ignore_ascii_case("img") => true,
        Some(t)
            if (t.eq_ignore_ascii_case("a") || t.eq_ignore_ascii_case("area"))
                && dom.has_attribute(entity, "href") =>
        {
            true
        }
        _ => false,
    })
}

// ---- translate (yes / no, defaults to true) ----
fn native_translate_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "translate")?;
    // §6.9 step 4: missing / "" / "yes" → true; "no" → false.
    let result = ctx.host().dom().with_attribute(entity, "translate", |raw| {
        !matches!(raw.map(str::to_ascii_lowercase).as_deref(), Some("no"))
    });
    Ok(JsValue::Boolean(result))
}
fn native_translate_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "translate")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let literal = if super::super::coerce::to_boolean(ctx.vm, val) {
        "yes"
    } else {
        "no"
    };
    ctx.host().dom().set_attribute(entity, "translate", literal);
    Ok(JsValue::Undefined)
}

// ---- spellcheck (true / false, defaults to true unless inherited false) ----
fn native_spellcheck_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "spellcheck")?;
    // §6.8.6 default-true: attr "true" / "" → true, "false" → false,
    // missing / other → true (Phase 2 simplification; inheritance
    // from ancestors is the spec rule but browsers diverge).
    let result = ctx
        .host()
        .dom()
        .with_attribute(entity, "spellcheck", |raw| {
            !matches!(raw.map(str::to_ascii_lowercase).as_deref(), Some("false"))
        });
    Ok(JsValue::Boolean(result))
}
fn native_spellcheck_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "spellcheck")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let literal = if super::super::coerce::to_boolean(ctx.vm, val) {
        "true"
    } else {
        "false"
    };
    ctx.host()
        .dom()
        .set_attribute(entity, "spellcheck", literal);
    Ok(JsValue::Undefined)
}

// ---- tabIndex (long with per-element default) ----
//
// WHATWG §6.6.3: default depends on the element — link (`a[href]`
// / `area[href]`), form control (`button`, `input:not([type=hidden])`,
// `select`, `textarea`), iframe / object / embed, and elements with
// `contenteditable` default to 0; everything else defaults to -1.
fn native_tab_index_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "tabIndex")?;
    let dom = ctx.host().dom();
    let parsed = dom.with_attribute(entity, "tabindex", |raw| {
        raw.and_then(elidex_dom_api::focus::parse_tab_index_value)
    });
    let value = parsed.unwrap_or_else(|| elidex_dom_api::focus::tab_index_default_for(dom, entity));
    Ok(JsValue::Number(f64::from(value)))
}
fn native_tab_index_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let entity = require_html_element_receiver(ctx, this, "tabIndex")?;
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    // WebIDL `long` — truncate via `ToInt32`.
    let n = super::super::coerce::to_int32(ctx.vm, val)?;
    let s = n.to_string();
    ctx.host().dom().set_attribute(entity, "tabindex", &s);
    Ok(JsValue::Undefined)
}

// The `tabindex` attribute parse (WHATWG §6.6.3) and the per-element
// `tabIndex` default now both live in the engine-independent
// `elidex_dom_api::focus` (`parse_tab_index_value` / `tab_index_default_for`),
// shared by the VM `tabIndex` getter, the dom-api `is_focusable` predicate, and
// the shell's focus reconciler (one focusable-area algorithm, one home — the
// Layering mandate; one tabindex parse, one home — one-issue-one-way).
