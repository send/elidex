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

use elidex_ecs::{Attributes, EcsDom, Entity, MutationEvent};

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
                new_value,
                ..
            } => handle_attribute_change(node, name, new_value, dom),
            MutationEvent::Insert { node, .. } => handle_insert(node, dom),
            _ => {}
        }
    }
}

/// Arm 1 — dynamic `setAttribute` / `removeAttribute` (WHATWG DOM §4.9
/// attribute change steps + HTML §8.1.8.1).
fn handle_attribute_change(node: Entity, name: &str, new_value: Option<&str>, dom: &mut EcsDom) {
    let Some(event_type) = event_handler_attr_event_type(name) else {
        return;
    };
    let event_type = event_type.to_string();
    match new_value {
        Some(src) => set_inline_handler(node, &event_type, src, dom),
        None => clear_inline_handler(node, &event_type, dom),
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
    let mut pending: Vec<(Entity, String, String)> = Vec::new();
    for entity in entities {
        if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
            for (attr_name, attr_value) in attrs.iter() {
                if let Some(event_type) = event_handler_attr_event_type(attr_name) {
                    pending.push((entity, event_type.to_string(), attr_value.to_string()));
                }
            }
        }
    }
    for (entity, event_type, source) in pending {
        set_inline_handler(entity, &event_type, &source, dom);
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

/// Content-attribute removal: clear the uncompiled source (the listener
/// entry is kept for registration-order stability; getter/dispatch will
/// see no compiled callable + no uncompiled source → no-op).
fn clear_inline_handler(entity: Entity, event_type: &str, dom: &mut EcsDom) {
    if let Ok(mut listeners) = dom.world_mut().get::<&mut EventListeners>(entity) {
        if let Some(id) = listeners.find_event_handler(event_type) {
            listeners.clear_uncompiled(id);
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
