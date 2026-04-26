//! `HTMLIFrameElement.prototype` intrinsic — per-tag prototype
//! layer for `<iframe>` wrappers (HTML §4.8.5).
//!
//! Chain (PR5b):
//!
//! ```text
//! iframe wrapper
//!   → HTMLIFrameElement.prototype    (this intrinsic)
//!     → HTMLElement.prototype
//!       → Element.prototype
//!         → Node.prototype
//!           → EventTarget.prototype
//!             → Object.prototype
//! ```
//!
//! The parent link was re-pointed from `Element.prototype` to
//! `HTMLElement.prototype` in PR5b §C1 so that
//! `iframe instanceof HTMLElement === true`; the rest of the chain
//! climbs unchanged through `Element → Node → EventTarget`.
//!
//! Members installed here:
//!
//! - **String reflect RW attrs** (WHATWG "DOMString reflect" —
//!   getter is `getAttribute(name) ?? ""`, setter is
//!   `setAttribute(name, ToString(v))`): `src`, `srcdoc`, `name`,
//!   `referrerPolicy`, `allow`, `width`, `height`, `loading`,
//!   `sandbox`.  `width` / `height` are kept as string-reflect
//!   instead of the `long` variant so `"100px"` round-trips faithfully.
//! - **Boolean reflect** (WHATWG "boolean reflect" — attribute
//!   presence ↔ IDL boolean): `allowFullscreen`.
//! - **Parity null stubs** — `contentDocument`, `contentWindow`.
//!   The legacy boa binding returns `null` here too because each
//!   iframe runs in its own `JsRuntime` and objects do not cross
//!   `Context` boundaries (`elidex-js-boa/src/globals/iframe.rs`).
//!   Real cross-context Document / Window proxies require multi-
//!   document support inside one VM (a separate sub-frame
//!   browsing-context entity model) and are tracked in the M4-12
//!   cutover residual roadmap, not this PR.  The `null` return
//!   here keeps the JS surface stable across the boa removal in
//!   PR7.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::{NativeFn, VmInner};

use elidex_ecs::{Entity, NodeKind};

impl VmInner {
    /// Allocate `HTMLIFrameElement.prototype` with
    /// `HTMLElement.prototype` as its parent so
    /// `iframe instanceof HTMLElement === true` (WHATWG §4.8.5 / §3.2.8).
    /// Must run after `register_html_element_prototype`.
    pub(in crate::vm) fn register_html_iframe_prototype(&mut self) {
        let parent = self
            .html_element_prototype
            .expect("register_html_iframe_prototype called before register_html_element_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(parent),
            extensible: true,
        });
        self.html_iframe_prototype = Some(proto_id);

        self.install_html_iframe_string_attrs(proto_id);
        self.install_html_iframe_bool_attr(proto_id);
        self.install_html_iframe_parity_stubs(proto_id);
    }

    fn install_html_iframe_string_attrs(&mut self, proto_id: ObjectId) {
        // (IDL property name SID, HTML attribute lowercase name).
        //
        // HTML content attributes are lowercase; the IDL property
        // name often is (e.g. `src`) but may be camelCase
        // (`referrerPolicy` ↔ `referrerpolicy`).  The split below
        // keeps both views explicit.
        let pairs: [(super::super::StringId, &'static str); 9] = [
            (self.well_known.src, "src"),
            (self.well_known.srcdoc, "srcdoc"),
            (self.well_known.name, "name"),
            (self.well_known.referrer_policy, "referrerpolicy"),
            (self.well_known.allow, "allow"),
            (self.well_known.width, "width"),
            (self.well_known.height, "height"),
            (self.well_known.loading, "loading"),
            (self.well_known.sandbox, "sandbox"),
        ];
        for (name_sid, attr_name) in pairs {
            let getter = string_reflect_getter_for(attr_name);
            let setter = string_reflect_setter_for(attr_name);
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                Some(setter),
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    fn install_html_iframe_bool_attr(&mut self, proto_id: ObjectId) {
        self.install_accessor_pair(
            proto_id,
            self.well_known.allow_fullscreen,
            native_iframe_get_allow_fullscreen,
            Some(native_iframe_set_allow_fullscreen),
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_html_iframe_parity_stubs(&mut self, proto_id: ObjectId) {
        for (name_sid, getter) in [
            (
                self.well_known.content_document,
                native_iframe_get_content_document as NativeFn,
            ),
            (
                self.well_known.content_window,
                native_iframe_get_content_window,
            ),
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
// Natives
// ---------------------------------------------------------------------------

/// Brand check for `<iframe>` receivers — rejects non-Element and
/// non-iframe tag entities with "Illegal invocation".
fn require_iframe_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<Entity>, VmError> {
    let Some(entity) =
        super::event_target::require_receiver(ctx, this, "HTMLIFrameElement", method, |k| {
            k == NodeKind::Element
        })?
    else {
        return Ok(None);
    };
    // Inside the element cohort, reject non-iframe tags — a plain
    // <div> would otherwise satisfy the kind filter but has no
    // business on HTMLIFrameElement.prototype methods.
    //
    // Use `HostData::tag_matches_ascii_case` directly rather than
    // `EcsDom::get_tag_name(...).eq_ignore_ascii_case(...)` — the
    // latter clones the TagType String on every accessor call, and
    // this brand check fires on every `iframe.src` read.  The helper
    // walks the ECS component in place and never allocates.
    let is_iframe = ctx.host().tag_matches_ascii_case(entity, "iframe");
    if !is_iframe {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'HTMLIFrameElement': Illegal invocation"
        )));
    }
    Ok(Some(entity))
}

/// Return a native getter that reads `attr_name` as a DOMString
/// reflect (empty string when absent).
fn string_reflect_getter_for(attr_name: &'static str) -> NativeFn {
    // Nine attribute names resolve through a small match so the
    // getter stays a plain function pointer (no trait objects / no
    // per-entity allocation).  Same machinery as the setters below.
    match attr_name {
        "src" => iframe_get_src,
        "srcdoc" => iframe_get_srcdoc,
        "name" => iframe_get_name,
        "referrerpolicy" => iframe_get_referrer_policy,
        "allow" => iframe_get_allow,
        "width" => iframe_get_width,
        "height" => iframe_get_height,
        "loading" => iframe_get_loading,
        "sandbox" => iframe_get_sandbox,
        _ => unreachable!("string_reflect_getter_for called with unsupported attr {attr_name}"),
    }
}

fn string_reflect_setter_for(attr_name: &'static str) -> NativeFn {
    match attr_name {
        "src" => iframe_set_src,
        "srcdoc" => iframe_set_srcdoc,
        "name" => iframe_set_name,
        "referrerpolicy" => iframe_set_referrer_policy,
        "allow" => iframe_set_allow,
        "width" => iframe_set_width,
        "height" => iframe_set_height,
        "loading" => iframe_set_loading,
        "sandbox" => iframe_set_sandbox,
        _ => unreachable!("string_reflect_setter_for called with unsupported attr {attr_name}"),
    }
}

macro_rules! iframe_string_attr {
    ($get:ident, $set:ident, $attr:expr, $label:expr) => {
        fn $get(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            _args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let empty = ctx.vm.well_known.empty;
            let Some(entity) = require_iframe_receiver(ctx, this, $label)? else {
                return Ok(JsValue::String(empty));
            };
            // Split borrow: `with_attribute` on `&EcsDom` + `intern`
            // on `&mut StringPool`, no per-call `String::from` clone.
            let sid = match ctx.dom_and_strings_if_bound() {
                Some((dom, strings)) => {
                    dom.with_attribute(entity, $attr, |v| v.map_or(empty, |s| strings.intern(s)))
                }
                None => empty,
            };
            Ok(JsValue::String(sid))
        }

        fn $set(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            let Some(entity) = require_iframe_receiver(ctx, this, $label)? else {
                return Ok(JsValue::Undefined);
            };
            let val = args.first().copied().unwrap_or(JsValue::Undefined);
            let sid = super::super::coerce::to_string(ctx.vm, val)?;
            let s = ctx.vm.strings.get_utf8(sid);
            ctx.host().dom().set_attribute(entity, $attr, s);
            Ok(JsValue::Undefined)
        }
    };
}

iframe_string_attr!(iframe_get_src, iframe_set_src, "src", "src");
iframe_string_attr!(iframe_get_srcdoc, iframe_set_srcdoc, "srcdoc", "srcdoc");
iframe_string_attr!(iframe_get_name, iframe_set_name, "name", "name");
iframe_string_attr!(
    iframe_get_referrer_policy,
    iframe_set_referrer_policy,
    "referrerpolicy",
    "referrerPolicy"
);
iframe_string_attr!(iframe_get_allow, iframe_set_allow, "allow", "allow");
iframe_string_attr!(iframe_get_width, iframe_set_width, "width", "width");
iframe_string_attr!(iframe_get_height, iframe_set_height, "height", "height");
iframe_string_attr!(iframe_get_loading, iframe_set_loading, "loading", "loading");
iframe_string_attr!(iframe_get_sandbox, iframe_set_sandbox, "sandbox", "sandbox");

/// `allowFullscreen` — WHATWG "boolean reflect".  Attribute presence
/// ↔ IDL boolean.  `ToBoolean(v)` drives the setter so
/// `iframe.allowFullscreen = "0"` is true (a non-empty string is
/// truthy) — counter-intuitive, but the conversion rule the spec
/// demands.
fn native_iframe_get_allow_fullscreen(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_iframe_receiver(ctx, this, "allowFullscreen")? else {
        return Ok(JsValue::Boolean(false));
    };
    let present = ctx.host().dom().has_attribute(entity, "allowfullscreen");
    Ok(JsValue::Boolean(present))
}

fn native_iframe_set_allow_fullscreen(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(entity) = require_iframe_receiver(ctx, this, "allowFullscreen")? else {
        return Ok(JsValue::Undefined);
    };
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let flag = super::super::coerce::to_boolean(ctx.vm, val);
    if flag {
        ctx.host()
            .dom()
            .set_attribute(entity, "allowfullscreen", String::new());
    } else {
        ctx.host().dom().remove_attribute(entity, "allowfullscreen");
    }
    Ok(JsValue::Undefined)
}

/// `contentDocument` — parity stub matching the legacy boa binding.
/// A real cross-context Document proxy needs the VM to host the
/// child frame's document in its own browsing context (see the
/// module docstring); until that lands the getter returns `null`,
/// which is also what the spec requires for cross-origin frames so
/// feature-detection code (`if (iframe.contentDocument)`) keeps
/// working.
fn native_iframe_get_content_document(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_iframe_receiver(ctx, this, "contentDocument")?;
    Ok(JsValue::Null)
}

/// `contentWindow` — parity stub matching the legacy boa binding.
/// See `native_iframe_get_content_document`.
fn native_iframe_get_content_window(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let _ = require_iframe_receiver(ctx, this, "contentWindow")?;
    Ok(JsValue::Null)
}
