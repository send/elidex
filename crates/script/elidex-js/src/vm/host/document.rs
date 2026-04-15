//! Document host globals — `document.getElementById`,
//! `document.createElement`, `document.createTextNode`, and the
//! getters for `body` / `head` / `documentElement` / `title` / `URL` /
//! `readyState` (WHATWG DOM §4.5 + HTML §3.2.9, §7.1, §12.2.8).
//!
//! # Scope
//!
//! - `getElementById(id)` — O(n) scan of all entities with
//!   `Attributes` component; the first match wins (WHATWG DOM §4.5
//!   "If the list has no items, return null").
//! - `createElement(tag)` / `createTextNode(data)` — allocate ECS
//!   entities; the Element form returns a freshly-cached wrapper,
//!   the Text form returns a host object keyed on the text entity
//!   (no text-specific wrapper methods yet — PR4c lands textContent,
//!   splitText, etc.).
//! - `body` / `head` / `documentElement` — tree walk from the
//!   document root looking for the first `<html>` child, then within
//!   that for `<head>` / `<body>`.  Phase 2 returns `null` when the
//!   structure is missing rather than synthesising fallback nodes.
//! - `title` (get) — concatenates text children of the first
//!   `<title>` element descending from `<head>`; setter lands in PR4c
//!   alongside the rest of the attribute-manipulation surface.
//! - `URL` / `documentURI` — `VmInner::navigation.current_url`.
//! - `readyState` — stub returning `"complete"` (the VM has no notion
//!   of document loading state yet; the shell owns that in PR6).

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{JsValue, NativeContext, ObjectId, PropertyKey, PropertyValue, VmError};
use super::super::VmInner;
use super::super::{coerce, Vm};

use elidex_ecs::{Attributes, Entity, TagType};

// ---------------------------------------------------------------------------
// Helpers — tree walk from the document root.
// ---------------------------------------------------------------------------

/// Find the first element child of `parent` whose tag (lowercased)
/// equals `tag`.  Children are walked in document order.  Text /
/// comment children are skipped without recursion.
fn first_child_with_tag(dom: &elidex_ecs::EcsDom, parent: Entity, tag: &str) -> Option<Entity> {
    for child in dom.children_iter(parent) {
        if let Ok(tag_comp) = dom.world().get::<&TagType>(child) {
            if tag_comp.0.eq_ignore_ascii_case(tag) {
                return Some(child);
            }
        }
    }
    None
}

/// Locate the `<html>` root child of the bound document.  Returns
/// `None` if there is no document entity or the document has no
/// `<html>` child (e.g. empty tree).
fn find_html_root(ctx: &mut NativeContext<'_>) -> Option<Entity> {
    let doc = ctx.vm.host_data.as_deref()?.document_entity_opt()?;
    let dom = ctx.host().dom();
    first_child_with_tag(dom, doc, "html")
}

// ---------------------------------------------------------------------------
// getElementById
// ---------------------------------------------------------------------------

pub(super) fn native_document_get_element_by_id(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    let target = ctx.vm.strings.get_utf8(sid);

    if !ctx
        .vm
        .host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
    {
        return Ok(JsValue::Null);
    }

    // O(n) scan.  A per-document id→entity index is tracked as a
    // separate performance follow-up; the shell page crawler will
    // surface real hot paths.
    let matched: Option<Entity> = {
        let dom = ctx.host().dom();
        dom.world()
            .query::<(Entity, &Attributes)>()
            .iter()
            .find_map(|(e, attrs)| {
                if attrs.get("id") == Some(&*target) {
                    Some(e)
                } else {
                    None
                }
            })
    };
    match matched {
        Some(e) => Ok(JsValue::Object(ctx.vm.create_element_wrapper(e))),
        None => Ok(JsValue::Null),
    }
}

// ---------------------------------------------------------------------------
// createElement / createTextNode
// ---------------------------------------------------------------------------

pub(super) fn native_document_create_element(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    let raw_tag = ctx.vm.strings.get_utf8(sid);

    // WHATWG DOM §4.5 createElement normalises to lowercase in the
    // "HTML document" branch.  We treat every bind as HTML.
    let tag = raw_tag.to_ascii_lowercase();

    if !ctx
        .vm
        .host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
    {
        return Err(VmError::type_error(
            "document.createElement called without a bound DOM",
        ));
    }

    let new_entity = {
        let dom = ctx.host().dom();
        dom.create_element(tag, Attributes::default())
    };
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

pub(super) fn native_document_create_text_node(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = coerce::to_string(ctx.vm, arg)?;
    let data = ctx.vm.strings.get_utf8(sid);

    if !ctx
        .vm
        .host_data
        .as_deref()
        .is_some_and(super::super::host_data::HostData::is_bound)
    {
        return Err(VmError::type_error(
            "document.createTextNode called without a bound DOM",
        ));
    }

    let new_entity = {
        let dom = ctx.host().dom();
        dom.create_text(data)
    };
    // Text nodes share the same HostObject wrapper surface as
    // elements — the prototype chain climbs through
    // `EventTarget.prototype` either way.  PR4c will install
    // text-specific own-properties (`data`, `length`, `splitText`,
    // etc.) on a dedicated `Text.prototype`; until then the wrapper
    // simply identifies the underlying entity.
    Ok(JsValue::Object(ctx.vm.create_element_wrapper(new_entity)))
}

// ---------------------------------------------------------------------------
// Getters: documentElement / head / body / title / URL / readyState
// ---------------------------------------------------------------------------

fn wrap_entity_or_null(vm: &mut VmInner, entity: Option<Entity>) -> JsValue {
    match entity {
        Some(e) => JsValue::Object(vm.create_element_wrapper(e)),
        None => JsValue::Null,
    }
}

pub(super) fn native_document_get_document_element(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let html = find_html_root(ctx);
    Ok(wrap_entity_or_null(ctx.vm, html))
}

pub(super) fn native_document_get_head(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let head = find_html_root(ctx).and_then(|html| {
        let dom = ctx.host().dom();
        first_child_with_tag(dom, html, "head")
    });
    Ok(wrap_entity_or_null(ctx.vm, head))
}

pub(super) fn native_document_get_body(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let body = find_html_root(ctx).and_then(|html| {
        let dom = ctx.host().dom();
        first_child_with_tag(dom, html, "body")
    });
    Ok(wrap_entity_or_null(ctx.vm, body))
}

pub(super) fn native_document_get_title(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let title_text: String = {
        let Some(html) = find_html_root(ctx) else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        let dom = ctx.host().dom();
        let Some(head) = first_child_with_tag(dom, html, "head") else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        let Some(title) = first_child_with_tag(dom, head, "title") else {
            return Ok(JsValue::String(ctx.vm.well_known.empty));
        };
        // Concat all text-node children (matches WHATWG §3.2.9
        // "descendant text content", but for title we only walk
        // immediate children — title cannot legally contain nested
        // elements).
        let mut out = String::new();
        for child in dom.children_iter(title) {
            if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(child) {
                out.push_str(&text.0);
            }
        }
        out
    };
    let sid = ctx.vm.strings.intern(&title_text);
    Ok(JsValue::String(sid))
}

pub(super) fn native_document_get_url(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let url = ctx.vm.navigation.current_url.clone();
    let sid = ctx.vm.strings.intern(&url);
    Ok(JsValue::String(sid))
}

pub(super) fn native_document_get_ready_state(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Phase 2: scripts run after parse completes (eval is synchronous
    // from the shell's perspective), so `complete` is honest.  The
    // shell wires real lifecycle transitions in PR6.
    let sid = ctx.vm.strings.intern("complete");
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// Installation — add the own-properties to the document wrapper at bind time.
// ---------------------------------------------------------------------------

impl Vm {
    /// Re-install the document-only own-properties each bind cycle.
    /// Called by `install_document_global` after the HostObject
    /// wrapper has been refreshed.
    ///
    /// We re-install rather than once-at-VM-init because the
    /// document wrapper is cached per-Entity in
    /// `HostData::wrapper_cache` — successive binds with the same
    /// document entity return the same wrapper ObjectId, so
    /// properties survive without any work, but the first bind
    /// *must* populate it.  The "already installed" check below
    /// keeps rebind cycles O(1).
    pub(in crate::vm) fn install_document_methods_if_needed(&mut self, doc_wrapper: ObjectId) {
        // Fast path: properties already installed on this wrapper.
        let method_key = PropertyKey::String(
            self.inner
                .strings
                .lookup("getElementById")
                .unwrap_or(self.inner.strings.intern("getElementById")),
        );
        if crate::vm::coerce::get_property(&self.inner, doc_wrapper, method_key).is_some() {
            return;
        }

        // Methods.
        let methods: &[(&str, super::super::NativeFn)] = &[
            ("getElementById", native_document_get_element_by_id),
            ("createElement", native_document_create_element),
            ("createTextNode", native_document_create_text_node),
        ];
        for &(name, func) in methods {
            let fn_id = self.inner.create_native_function(name, func);
            let key = PropertyKey::String(self.inner.strings.intern(name));
            self.inner.define_shaped_property(
                doc_wrapper,
                key,
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }

        // Read-only accessors.
        let ro_accessors: &[(&str, super::super::NativeFn)] = &[
            ("documentElement", native_document_get_document_element),
            ("head", native_document_get_head),
            ("body", native_document_get_body),
            ("title", native_document_get_title),
            ("URL", native_document_get_url),
            ("documentURI", native_document_get_url),
            ("readyState", native_document_get_ready_state),
        ];
        for &(name, getter) in ro_accessors {
            let getter_name = format!("get {name}");
            let gid = self.inner.create_native_function(&getter_name, getter);
            let key = PropertyKey::String(self.inner.strings.intern(name));
            self.inner.define_shaped_property(
                doc_wrapper,
                key,
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }
}
