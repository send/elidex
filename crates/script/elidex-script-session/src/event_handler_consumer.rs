//! [`EventHandlerAttributeConsumer`] — inline event-handler content
//! attribute detector (WHATWG HTML §8.1.8.1).
//!
//! Dual-arm [`MutationDispatcher`](elidex_ecs::MutationDispatcher)
//! consumer (mirrors the `FormControlReconciler` AttributeChange +
//! Insert shape):
//!
//! - **Arm 1 — [`MutationEvent::AttributeChange`]**: dynamic
//!   `setAttribute` / `removeAttribute` on a live element. Fires at the
//!   `EcsDom::set_attribute` chokepoint.
//! - **Arm 2 — [`MutationEvent::Insert`]**: parser / `innerHTML` /
//!   `outerHTML` / `setHTMLUnsafe` bake attributes into the `Attributes`
//!   component at `create_element` time and do **not** fire
//!   `AttributeChange` (WHATWG DOM §4.2.3 insert + §4.9 create-an-element
//!   / HTML §8.5 DOM-parsing APIs + §13.2.6 tree construction). Arm 2
//!   walks the inserted subtree's baked `on*` attributes. `Insert` fires
//!   per inserted node (no descendants slice — unlike `Remove`), so the
//!   arm must walk descendants itself.
//!
//! # Layering
//!
//! Engine-independent: records the *uncompiled* source string into the
//! [`EventListeners`] component's
//! [`ListenerKind::EventHandler`](crate::ListenerKind::EventHandler)
//! entry (`set_uncompiled`); it never compiles (no VM / `NativeContext`
//! access — `MutationDispatcher::dispatch` receives only `EcsDom`).
//! Compilation is lazy, performed VM-side at first read / dispatch
//! (WHATWG HTML §8.1.8.1 "get the current value"). This consumer is
//! placed in `elidex-script-session` (not `elidex-dom-api`/`elidex-form`
//! like the other consumers) by **write-locality**: [`EventListeners`]
//! is defined here, and hosting the consumer in `elidex-form` would
//! introduce a new `elidex-form → elidex-script-session` dependency
//! edge. It is the first `MutationDispatcher` impl in this crate.

use elidex_ecs::{Attributes, EcsDom, Entity, MutationEvent, TagType};

use crate::EventListeners;

/// Which mixin / interface an event-handler IDL attribute belongs to
/// (WHATWG HTML §8.1.8.2 / §8.1.8.2.1). Drives VM-side prototype
/// installation (`elidex-js`) — kept in the engine-independent crate so
/// the attribute name list has a single source of truth shared by inline
/// detection (here) and IDL accessor install (VM-side).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerScope {
    /// `GlobalEventHandlers` — Element + Document + Window.
    Global,
    /// `DocumentAndElementEventHandlers` — Document + Element.
    DocumentElement,
    /// `WindowEventHandlers` — Window (+ HTMLBodyElement delegation).
    Window,
    /// Document partial interface only (`onreadystatechange` etc.).
    DocumentOnly,
}

/// Canonical event-handler IDL attribute table (WHATWG HTML §8.1.8.2.1).
/// Event type = attribute name without the leading `on` (uniform across
/// the whole table — `onclick` → `click`, `onbeforeunload` →
/// `beforeunload`, etc.). Single SoT: inline detection (this module) and
/// VM-side accessor install both consume this list.
pub const EVENT_HANDLER_ATTRS: &[(&str, HandlerScope)] = &[
    // GlobalEventHandlers
    ("onabort", HandlerScope::Global),
    ("onauxclick", HandlerScope::Global),
    ("onbeforeinput", HandlerScope::Global),
    ("onbeforematch", HandlerScope::Global),
    ("onbeforetoggle", HandlerScope::Global),
    ("onblur", HandlerScope::Global),
    ("oncancel", HandlerScope::Global),
    ("oncanplay", HandlerScope::Global),
    ("oncanplaythrough", HandlerScope::Global),
    ("onchange", HandlerScope::Global),
    ("onclick", HandlerScope::Global),
    ("onclose", HandlerScope::Global),
    ("oncontextlost", HandlerScope::Global),
    ("oncontextmenu", HandlerScope::Global),
    ("oncontextrestored", HandlerScope::Global),
    ("oncuechange", HandlerScope::Global),
    ("ondblclick", HandlerScope::Global),
    ("ondrag", HandlerScope::Global),
    ("ondragend", HandlerScope::Global),
    ("ondragenter", HandlerScope::Global),
    ("ondragleave", HandlerScope::Global),
    ("ondragover", HandlerScope::Global),
    ("ondragstart", HandlerScope::Global),
    ("ondrop", HandlerScope::Global),
    ("ondurationchange", HandlerScope::Global),
    ("onemptied", HandlerScope::Global),
    ("onended", HandlerScope::Global),
    ("onerror", HandlerScope::Global),
    ("onfocus", HandlerScope::Global),
    ("onformdata", HandlerScope::Global),
    ("ongotpointercapture", HandlerScope::Global),
    ("oninput", HandlerScope::Global),
    ("oninvalid", HandlerScope::Global),
    ("onkeydown", HandlerScope::Global),
    ("onkeypress", HandlerScope::Global),
    ("onkeyup", HandlerScope::Global),
    ("onload", HandlerScope::Global),
    ("onloadeddata", HandlerScope::Global),
    ("onloadedmetadata", HandlerScope::Global),
    ("onloadstart", HandlerScope::Global),
    ("onlostpointercapture", HandlerScope::Global),
    ("onmousedown", HandlerScope::Global),
    ("onmouseenter", HandlerScope::Global),
    ("onmouseleave", HandlerScope::Global),
    ("onmousemove", HandlerScope::Global),
    ("onmouseout", HandlerScope::Global),
    ("onmouseover", HandlerScope::Global),
    ("onmouseup", HandlerScope::Global),
    ("onpause", HandlerScope::Global),
    ("onplay", HandlerScope::Global),
    ("onplaying", HandlerScope::Global),
    ("onpointercancel", HandlerScope::Global),
    ("onpointerdown", HandlerScope::Global),
    ("onpointerenter", HandlerScope::Global),
    ("onpointerleave", HandlerScope::Global),
    ("onpointermove", HandlerScope::Global),
    ("onpointerout", HandlerScope::Global),
    ("onpointerover", HandlerScope::Global),
    ("onpointerup", HandlerScope::Global),
    ("onprogress", HandlerScope::Global),
    ("onratechange", HandlerScope::Global),
    ("onreset", HandlerScope::Global),
    ("onresize", HandlerScope::Global),
    ("onscroll", HandlerScope::Global),
    ("onscrollend", HandlerScope::Global),
    ("onsecuritypolicyviolation", HandlerScope::Global),
    ("onseeked", HandlerScope::Global),
    ("onseeking", HandlerScope::Global),
    ("onselect", HandlerScope::Global),
    ("onslotchange", HandlerScope::Global),
    ("onstalled", HandlerScope::Global),
    ("onsubmit", HandlerScope::Global),
    ("onsuspend", HandlerScope::Global),
    ("ontimeupdate", HandlerScope::Global),
    ("ontoggle", HandlerScope::Global),
    ("ontransitioncancel", HandlerScope::Global),
    ("ontransitionend", HandlerScope::Global),
    ("ontransitionrun", HandlerScope::Global),
    ("ontransitionstart", HandlerScope::Global),
    ("onvolumechange", HandlerScope::Global),
    ("onwaiting", HandlerScope::Global),
    ("onwheel", HandlerScope::Global),
    // DocumentAndElementEventHandlers
    ("oncopy", HandlerScope::DocumentElement),
    ("oncut", HandlerScope::DocumentElement),
    ("onpaste", HandlerScope::DocumentElement),
    // WindowEventHandlers (Window + HTMLBodyElement delegation)
    ("onafterprint", HandlerScope::Window),
    ("onbeforeprint", HandlerScope::Window),
    ("onbeforeunload", HandlerScope::Window),
    ("onhashchange", HandlerScope::Window),
    ("onlanguagechange", HandlerScope::Window),
    ("onmessage", HandlerScope::Window),
    ("onmessageerror", HandlerScope::Window),
    ("onoffline", HandlerScope::Window),
    ("ononline", HandlerScope::Window),
    ("onpagehide", HandlerScope::Window),
    ("onpagereveal", HandlerScope::Window),
    ("onpageshow", HandlerScope::Window),
    ("onpageswap", HandlerScope::Window),
    ("onpopstate", HandlerScope::Window),
    ("onrejectionhandled", HandlerScope::Window),
    ("onstorage", HandlerScope::Window),
    ("onunhandledrejection", HandlerScope::Window),
    ("onunload", HandlerScope::Window),
    // Document partial interface
    ("onreadystatechange", HandlerScope::DocumentOnly),
    ("onvisibilitychange", HandlerScope::DocumentOnly),
];

/// Event-handler IDL attributes exposed on `WorkerGlobalScope` /
/// `DedicatedWorkerGlobalScope` (WHATWG HTML §10.2.1.1: `onerror` from
/// WorkerGlobalScope, `onmessage` / `onmessageerror` from
/// DedicatedWorkerGlobalScope, and the WindowOrWorkerGlobalScope shared
/// handlers). Disjoint from the `HandlerScope`-keyed [`EVENT_HANDLER_ATTRS`]
/// table because the worker set is a hand-picked subset that overlaps the
/// `Window` / `Global` scopes (single-scope rows cannot be dual-tagged); kept
/// here as the single source of truth so the VM-side install reads one list.
/// Every entry also appears in [`EVENT_HANDLER_ATTRS`], so
/// [`event_handler_attr_event_type`] resolves each.
pub const WORKER_EVENT_HANDLER_ATTRS: &[&str] = &[
    "onmessage",
    "onmessageerror",
    "onerror",
    "onlanguagechange",
    "onoffline",
    "ononline",
    "onrejectionhandled",
    "onunhandledrejection",
];

/// Event-handler IDL attributes exposed on the main-side `Worker` object (the
/// parent's handle): `onerror` from the AbstractWorker mixin (WHATWG HTML
/// §10.2.6.1) plus `onmessage` / `onmessageerror` from the dedicated `Worker`
/// interface (§10.2.6.3). A strict subset of [`WORKER_EVENT_HANDLER_ATTRS`] —
/// the WindowOrWorkerGlobalScope shared handlers belong only to the worker
/// *scope*, not to the `Worker` object. Kept here as the single source of
/// truth so the VM-side install reads one list; every entry also appears in
/// [`EVENT_HANDLER_ATTRS`], so [`event_handler_attr_event_type`] resolves each.
pub const WORKER_OBJECT_EVENT_HANDLER_ATTRS: &[&str] = &["onmessage", "onmessageerror", "onerror"];

/// If `name` is a known event-handler content attribute, return its
/// event type (the name with the leading `on` stripped). Linear scan —
/// only on attribute mutations / element inserts, not a hot path.
#[must_use]
pub fn event_handler_attr_event_type(name: &str) -> Option<&str> {
    if EVENT_HANDLER_ATTRS.iter().any(|(attr, _)| *attr == name) {
        // Every entry's event type is the name minus "on".
        Some(&name[2..])
    } else {
        None
    }
}

/// The spec level of the **Web Storage family** — the SINGLE classification
/// source every Web Storage install seam reads (Codex R7).
///
/// The family spans several install surfaces — the `localStorage` /
/// `sessionStorage` accessors (`window.rs`), the `Storage` / `StorageEvent`
/// globals (`register_globals`), and the `window.onstorage` handler attribute
/// (via [`event_handler_attr_spec_level`]). Routing them all through this one
/// source means the family is demoted by **one edit** rather than N independent
/// `Modern` literals that must be flipped in lockstep (a missed one would leave a
/// split surface — `StorageEvent` without `localStorage`, accessors without the
/// constructors). A1 = [`Modern`](elidex_plugin::WebApiSpecLevel::Modern)
/// (installs in every mode — no behavior change); **A2 flips this to
/// [`Legacy`](elidex_plugin::WebApiSpecLevel::Legacy) here, in one place** (HTML
/// §12.2). The non-install storage surfaces — the `<body onstorage="…">`
/// content-attribute path and `StorageEvent` delivery — are A2's broader
/// suppression scope (A0 §5 A2 row, which spans the VM *and* the shell tab/IPC
/// plumbing); A2 wires them to read this same source.
#[must_use]
pub fn web_storage_spec_level() -> elidex_plugin::WebApiSpecLevel {
    elidex_plugin::WebApiSpecLevel::Modern
}

/// The spec level of `document.cookie` — the single classification source for the
/// cookie surface (single install surface, but kept a source for uniformity so
/// no demotable family is gated by a bare literal). A1 = `Modern`; **A3 flips
/// this to `Legacy`** (HTML §3.1.4).
#[must_use]
pub fn document_cookie_spec_level() -> elidex_plugin::WebApiSpecLevel {
    elidex_plugin::WebApiSpecLevel::Modern
}

/// The spec level of the **live-collection family** — the SINGLE classification
/// source every live-collection install seam reads (Codex R7).
///
/// The family spans many surfaces: the `Document` getters
/// `getElementsByTagName` / `getElementsByClassName` / `getElementsByName`
/// (DOM §4.5 / HTML §3.1.7), the `forms` / `images` / `links` accessors, the
/// ParentNode `children` mixin, plus `Element.prototype` getters / `table.rows` /
/// `select.options` in sibling files. Routing them all through this one source
/// makes the family a **one-edit demotion**. A1 = [`Living`](elidex_plugin::DomSpecLevel::Living)
/// (installs in every mode); **B0/B1 flip this to
/// [`Legacy`](elidex_plugin::DomSpecLevel::Legacy) here, in one place** (design
/// §12.1.2). A1 wires the `Document` getters as the representative seam; the
/// **full surface sweep** (`forms`/`images`/`links`/`children` + the cross-file
/// surfaces) is B0's classification work (A0 §5 B0 row), routed through this same
/// source — not a new gate.
#[must_use]
pub fn live_collection_spec_level() -> elidex_plugin::DomSpecLevel {
    elidex_plugin::DomSpecLevel::Living
}

/// The Web-API spec level of an event-handler IDL attribute — seam-3 of the A1
/// Web-API core/compat gate (the VM's `install_handler_attr_family` loop gates
/// each row by `installs(level)`).
///
/// **Total over [`EVENT_HANDLER_ATTRS`]** (sibling of
/// [`event_handler_attr_event_type`]): every attr — known or not — maps to a
/// level, so a future row added to the family table can never silently
/// mis-classify. `"onstorage"` reads [`web_storage_spec_level`] (it fires
/// `StorageEvent`, part of the Web Storage surface, HTML §12.2.4), so it is
/// withheld together with the rest of that family when A2 flips the one source;
/// every other attr is [`Modern`](elidex_plugin::WebApiSpecLevel::Modern). In A1
/// the source is `Modern`, so this installs in every mode (no behavior change).
#[must_use]
pub fn event_handler_attr_spec_level(name: &str) -> elidex_plugin::WebApiSpecLevel {
    match name {
        // `window.onstorage` is part of the Web Storage family — tie it to the
        // family's single source so A2's one flip hides it too.
        "onstorage" => web_storage_spec_level(),
        // Every other handler attr is Modern (total default — no silent
        // mis-classification when the family table grows).
        _ => elidex_plugin::WebApiSpecLevel::Modern,
    }
}

/// If `name` is a known event-handler content attribute, return its event
/// type (name minus `on`) and [`HandlerScope`]. Linear scan, off the hot
/// path. The scope drives `<body>` WindowEventHandlers delegation (below).
fn event_handler_attr_lookup(name: &str) -> Option<(&str, HandlerScope)> {
    EVENT_HANDLER_ATTRS
        .iter()
        .find(|(attr, _)| *attr == name)
        .map(|(attr, scope)| (&attr[2..], *scope))
}

/// Resolve the entity whose [`EventListeners`] component should hold an
/// inline handler for `(origin, scope)`. WindowEventHandlers content
/// attributes on a `<body>` element delegate to the Window object (WHATWG
/// HTML §8.1.8.2) — the IDL accessors read/write the Window, so the inline
/// source must land there too. Everything else stays on the origin entity.
fn resolve_handler_target(origin: Entity, scope: HandlerScope, dom: &EcsDom) -> Entity {
    if scope == HandlerScope::Window && is_body_element(origin, dom) {
        dom.window_entity().unwrap_or(origin)
    } else {
        origin
    }
}

/// `true` if `entity` is an HTML `<body>` element.
fn is_body_element(entity: Entity, dom: &EcsDom) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .is_ok_and(|t| t.0.eq_ignore_ascii_case("body"))
}

/// Inline event-handler content attribute consumer. Unit struct — all
/// state lives in the [`EventListeners`] ECS component.
pub struct EventHandlerAttributeConsumer;

impl EventHandlerAttributeConsumer {
    /// Dispatch entry invoked by the binding-layer composer.
    pub fn handle(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        match *event {
            MutationEvent::AttributeChange {
                node,
                name,
                old_value,
                new_value,
            } => handle_attribute_change(node, name, old_value, new_value, dom),
            MutationEvent::Insert { node, .. } => handle_insert(node, dom),
            _ => {}
        }
    }
}

/// Arm 1 — dynamic `setAttribute` / `removeAttribute` (WHATWG DOM §4.9
/// attribute change steps + HTML §8.1.8.1).
fn handle_attribute_change(
    node: Entity,
    name: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    dom: &mut EcsDom,
) {
    let Some((event_type, scope)) = event_handler_attr_lookup(name) else {
        return;
    };
    let event_type = event_type.to_string();
    let target = resolve_handler_target(node, scope, dom);
    match new_value {
        Some(src) => set_inline_handler(target, &event_type, src, dom),
        // Only a genuine content-attribute removal (an attribute that
        // actually existed) clears the handler. `remove_attribute` fires
        // unconditionally with `old_value = None` for an absent attribute
        // (EcsDom DOM §4.3.2 record semantics); such a no-op must NOT
        // disturb an IDL-set handler (`el.onclick = fn` creates no content
        // attribute), per WHATWG HTML §8.1.8.1.
        None if old_value.is_some() => clear_inline_handler(target, &event_type, dom),
        None => {}
    }
}

/// Arm 2 — parser / innerHTML baked-attr spawn (WHATWG DOM §4.2.3 insert).
/// `Insert` fires per inserted root only (no descendants slice), and
/// [`EcsDom::traverse_descendants`] excludes the root, so process the
/// inserted `node` itself plus every descendant.
fn handle_insert(node: Entity, dom: &mut EcsDom) {
    // Collect the subtree's entities first (closure captures only the
    // Vec), then read attributes + mutate EventListeners — avoids
    // overlapping borrows of `dom`.
    let mut entities: Vec<Entity> = vec![node];
    dom.traverse_descendants(node, |descendant| {
        entities.push(descendant);
        true
    });
    // (origin entity, scope, event_type, source). Target resolution is
    // deferred to after the `Attributes` borrow is released to avoid
    // overlapping `dom` borrows.
    let mut pending: Vec<(Entity, HandlerScope, String, String)> = Vec::new();
    for entity in entities {
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            for (attr_name, attr_value) in attrs.iter() {
                if let Some((event_type, scope)) = event_handler_attr_lookup(attr_name) {
                    pending.push((
                        entity,
                        scope,
                        event_type.to_string(),
                        attr_value.to_string(),
                    ));
                }
            }
        }
    }
    for (origin, scope, event_type, source) in pending {
        let target = resolve_handler_target(origin, scope, dom);
        set_inline_handler(target, &event_type, &source, dom);
    }
}

/// Ensure an event-handler listener exists for `(entity, event_type)` and
/// record its uncompiled inline source.
fn set_inline_handler(entity: Entity, event_type: &str, source: &str, dom: &mut EcsDom) {
    ensure_event_listeners(entity, dom);
    if let Ok(mut listeners) = dom.world_mut().get::<&mut EventListeners>(entity) {
        let id = listeners
            .find_event_handler(event_type)
            .unwrap_or_else(|| listeners.add_event_handler(event_type.to_string()));
        listeners.set_uncompiled(id, source);
    }
}

/// Content-attribute removal (`removeAttribute('onclick')`): mark the
/// handler cleared (WHATWG HTML §8.1.8.1 — the handler value becomes
/// null). The listener entry is kept for registration-order stability;
/// the `cleared` flag makes the VM-side getter/dispatch drop any already-
/// compiled callable (which this engine-independent crate cannot reach)
/// and treat the handler as null until it is reactivated.
fn clear_inline_handler(entity: Entity, event_type: &str, dom: &mut EcsDom) {
    if let Ok(mut listeners) = dom.world_mut().get::<&mut EventListeners>(entity) {
        if let Some(id) = listeners.find_event_handler(event_type) {
            listeners.mark_cleared(id);
        }
    }
}

/// Attach an empty [`EventListeners`] component if absent (so the
/// subsequent `&mut` borrow succeeds).
fn ensure_event_listeners(entity: Entity, dom: &mut EcsDom) {
    if dom.world().get::<&EventListeners>(entity).is_err() {
        let _ = dom.world_mut().insert_one(entity, EventListeners::new());
    }
}

#[cfg(test)]
mod spec_level_tests {
    use super::{
        document_cookie_spec_level, event_handler_attr_spec_level, live_collection_spec_level,
        web_storage_spec_level, EVENT_HANDLER_ATTRS,
    };
    use elidex_plugin::{DomSpecLevel, WebApiSpecLevel};

    #[test]
    fn a1_classifies_every_family_modern() {
        // A1 marks nothing Legacy — every family's single source is Modern/Living
        // so the install seams install in every mode (no behavior change). A2/A3/B
        // flip exactly one source each; these assertions are the canaries that
        // catch an accidental early demotion.
        assert_eq!(web_storage_spec_level(), WebApiSpecLevel::Modern);
        assert_eq!(document_cookie_spec_level(), WebApiSpecLevel::Modern);
        assert_eq!(live_collection_spec_level(), DomSpecLevel::Living);
    }

    #[test]
    fn onstorage_is_tied_to_the_web_storage_source() {
        // `window.onstorage` is part of the Web Storage family, so it MUST read the
        // family's single source — otherwise A2's one flip would hide the
        // accessors / `Storage` / `StorageEvent` but leave `onstorage` exposed
        // (Codex R7). This binds them so the tie cannot silently break.
        assert_eq!(
            event_handler_attr_spec_level("onstorage"),
            web_storage_spec_level()
        );
    }

    #[test]
    fn spec_level_is_total_over_the_handler_attr_table() {
        // Sibling of `event_handler_attr_event_type`'s totality: every row of
        // `EVENT_HANDLER_ATTRS` resolves to a level (no panic / no fall-through to
        // an unintended default), and only `onstorage` is non-Modern-by-source.
        for (attr, _scope) in EVENT_HANDLER_ATTRS {
            let level = event_handler_attr_spec_level(attr);
            if *attr == "onstorage" {
                assert_eq!(level, web_storage_spec_level());
            } else {
                assert_eq!(level, WebApiSpecLevel::Modern, "{attr} must be Modern");
            }
        }
    }
}
