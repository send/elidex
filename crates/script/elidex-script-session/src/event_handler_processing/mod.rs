//! Event-handler content attributes processed once per origin element
//! (umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md`,
//! slice a; per-PR plan `docs/plans/2026-10-eh-attrs-slice-a.md`).
//!
//! The realisation of the *attribute change steps* (WHATWG HTML §8.1.8.1
//! Event handlers) for event-handler content attributes, at three timings
//! ([`ProcessingTiming`]): the bound document's first bind, insertion, and
//! the first handler write. The consumer's arms
//! (`event_handler_consumer.rs`) and the VM's IDL setters call into here.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, TemplateContentsWalk};
use elidex_plugin::SpecLevelPolicy;

use crate::event_handler_consumer::{
    event_handler_attr_spec_level, HandlerScope, EVENT_HANDLER_ATTRS,
};
use crate::EventListeners;

/// The origin element's creation-time event-handler content attributes have
/// been processed (umbrella D-1). Unit, `Send + Sync`, sparse: inserted only
/// on an element that carries at least one handler content attribute of an
/// ordinary pair.
///
/// WHATWG DOM §4.9 Interface Element, *append an attribute* step 4 (each
/// creation-time attribute runs the attribute change steps once); WHATWG HTML
/// §8.1.8.1 Event handlers, *attribute change steps*.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HandlerAttrsProcessed;

/// The bound Document's pre-bind tree has had its handler content attributes
/// processed by the bind pass (umbrella D-5). Unit, on the Document entity.
///
/// WHATWG HTML §13.2.6.1 Creating and inserting nodes, *create an element for
/// a token* step 17.2 (the parser appended the attributes before any bind).
/// The once-per-document bind point itself has no spec step: it is an elidex
/// mechanism (D-5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreBindHandlerAttrsProcessed;

/// WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and
/// Window objects (`#window-reflecting-body-element-event-handler-set`).
pub const WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET: [&str; 6] = [
    "onblur", "onerror", "onfocus", "onload", "onresize", "onscroll",
];

/// The timing at which [`process_handler_content_attributes`] runs (umbrella
/// K-0's T-a1 / T-a2 / T-a3). No spec step defines it: an elidex mechanism,
/// recorded by slot `#11-eh-attrs-creation-time-call-sites`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessingTiming {
    /// T-a1: the bound document's first bind.
    Bind,
    /// T-a2: the consumer's Insert arm.
    Insertion,
    /// T-a3: the first handler write that passes the D-8 restriction.
    HandlerWrite,
}

/// Run the attribute change steps for `element`'s event-handler content
/// attributes (umbrella D-2), in `Attributes` order. WHATWG HTML §8.1.8.1 Event
/// handlers, *attribute change steps* 1–5.6, per attribute; WHATWG DOM §4.9
/// Interface Element, *append an attribute* step 4.
///
/// Not realised: step 3 (a null target, which `determine the target of an
/// event handler` step 3 yields when the node document is not an active
/// document, is not modelled; target determination is slot
/// `#11-event-handler-target-determination`, T3) and step 5.1 (CSP — not
/// modelled).
///
/// Ordinary pairs are processed once (K-1): skipped when `element` is
/// already marked. Window-delegating pairs (X-W) bypass the once rule: they
/// are delivered at [`ProcessingTiming::Bind`] and
/// [`ProcessingTiming::Insertion`] and never at
/// [`ProcessingTiming::HandlerWrite`]. `element` is marked iff it carries at
/// least one ordinary handler content attribute, policy-excluded or not.
pub(crate) fn process_handler_content_attributes(
    dom: &mut EcsDom,
    element: Entity,
    policy: SpecLevelPolicy,
    timing: ProcessingTiming,
) {
    let marked = is_processed(dom, element);
    // Marked: ordinary pairs are skipped, and only `body` / `frameset` have
    // Window-delegating pairs, so nothing would be delivered.
    if marked && !is_body_or_frameset(dom, element) {
        return;
    }
    // Snapshot (name, event_type, scope, value); no `Attributes` → nothing to do
    // (Document, Window, text nodes, shadow-root entities).
    let snapshot: Vec<(String, String, HandlerScope, String)> = {
        let Ok(attrs) = dom.world().get::<&Attributes>(element) else {
            return;
        };
        attrs
            .iter()
            .filter_map(|(name, value)| {
                event_handler_attr_lookup(name).map(|(event_type, scope)| {
                    (
                        name.to_owned(),
                        event_type.to_owned(),
                        scope,
                        value.to_owned(),
                    )
                })
            })
            .collect()
    };
    let mut carries_ordinary = false;
    let mut writes: Vec<(HandlerScope, String, String)> = Vec::new();
    for (name, event_type, scope, value) in snapshot {
        let window_delegating = is_window_delegating_pair(dom, element, &name);
        if !window_delegating {
            // Counted before the policy filter: an element whose only ordinary
            // attributes are policy-excluded is still marked.
            carries_ordinary = true;
        }
        if !policy.installs(event_handler_attr_spec_level(&name)) {
            continue;
        }
        let deliver = match (window_delegating, timing) {
            (true, ProcessingTiming::Bind | ProcessingTiming::Insertion) => true,
            (true, ProcessingTiming::HandlerWrite) => false,
            (false, _) => !marked,
        };
        if deliver {
            writes.push((scope, event_type, value));
        }
    }
    for (scope, event_type, value) in writes {
        let target = resolve_handler_target(element, scope, dom);
        set_inline_handler(target, &event_type, &value, dom);
    }
    if carries_ordinary && !marked {
        let _ = dom.world_mut().insert_one(element, HandlerAttrsProcessed);
    }
}

/// Write-side entry, called before a handler write (umbrella D-4 / D-8): by
/// the consumer's AttributeChange arm after its lookup and policy check, and
/// by both VM element setters after receiver resolution (D-6).
///
/// Processes `element` only if it is unmarked, the written pair is not
/// Window-delegating, and `name` is `element`'s only deliverable ordinary
/// handler content attribute (D-8 (a)). The restriction is recorded by slot
/// `#11-eh-attrs-creation-time-call-sites`. No spec step defines this entry: an
/// elidex mechanism (D-8), recorded by the same slot.
pub fn process_before_handler_write(
    dom: &mut EcsDom,
    element: Entity,
    name: &str,
    policy: SpecLevelPolicy,
) {
    if is_processed(dom, element) {
        return;
    }
    if is_window_delegating_pair(dom, element, name) {
        return;
    }
    if !is_only_deliverable_ordinary_name(dom, element, name, policy) {
        return;
    }
    process_handler_content_attributes(dom, element, policy, ProcessingTiming::HandlerWrite);
}

/// The bind pass (umbrella D-5 / D-11): process every element of the bound
/// `document`'s shadow-including tree, uncapped, template contents excluded,
/// once per document.
pub(crate) fn process_bound_document(dom: &mut EcsDom, document: Entity, policy: SpecLevelPolicy) {
    if !dom.contains(document)
        || dom
            .world()
            .get::<&PreBindHandlerAttrsProcessed>(document)
            .is_ok()
    {
        return;
    }
    let mut nodes = Vec::new();
    dom.for_each_uncapped_shadow_inclusive(document, TemplateContentsWalk::Exclude, &mut |e| {
        nodes.push(e);
    });
    for node in nodes {
        process_handler_content_attributes(dom, node, policy, ProcessingTiming::Bind);
    }
    let _ = dom
        .world_mut()
        .insert_one(document, PreBindHandlerAttrsProcessed);
}

fn is_processed(dom: &EcsDom, element: Entity) -> bool {
    dom.world().get::<&HandlerAttrsProcessed>(element).is_ok()
}

/// `true` iff `name` is the one and only handler content attribute of
/// `element` that is ordinary (not Window-delegating) and installed by
/// `policy`. Stops at the first mismatch or second hit.
fn is_only_deliverable_ordinary_name(
    dom: &EcsDom,
    element: Entity,
    name: &str,
    policy: SpecLevelPolicy,
) -> bool {
    let Ok(attrs) = dom.world().get::<&Attributes>(element) else {
        return false;
    };
    let mut deliverable = attrs.iter().map(|(n, _)| n).filter(|n| {
        event_handler_attr_lookup(n).is_some()
            && policy.installs(event_handler_attr_spec_level(n))
            && !is_window_delegating_pair(dom, element, n)
    });
    deliverable.next() == Some(name) && deliverable.next().is_none()
}

/// Steps 1–2 of *determining the target of an event handler* (WHATWG HTML
/// §8.1.8.1 Event handlers): `true` iff those steps do not return the
/// element. Reads the tag and the name only (umbrella X-W).
pub(crate) fn is_window_delegating_pair(dom: &EcsDom, element: Entity, name: &str) -> bool {
    is_body_or_frameset(dom, element)
        && (matches!(
            event_handler_attr_lookup(name),
            Some((_, HandlerScope::Window))
        ) || WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET.contains(&name))
}

/// `true` if `element` is a `body` or a `frameset` element, its tag compared
/// ASCII-case-insensitively as [`is_body_element`] compares it. The class test
/// of steps 1–2 of *determining the target of an event handler* (WHATWG HTML
/// §8.1.8.1 Event handlers).
fn is_body_or_frameset(dom: &EcsDom, element: Entity) -> bool {
    dom.world()
        .get::<&TagType>(element)
        .is_ok_and(|t| t.0.eq_ignore_ascii_case("body") || t.0.eq_ignore_ascii_case("frameset"))
}

/// If `name` is a known event-handler content attribute, return its event
/// type (name minus `on`) and [`HandlerScope`]. Linear scan, off the hot
/// path. The scope drives `<body>` WindowEventHandlers delegation (below).
pub(crate) fn event_handler_attr_lookup(name: &str) -> Option<(&str, HandlerScope)> {
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
pub(crate) fn resolve_handler_target(origin: Entity, scope: HandlerScope, dom: &EcsDom) -> Entity {
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

/// Ensure an event-handler listener exists for `(entity, event_type)` and
/// record its uncompiled inline source.
pub(crate) fn set_inline_handler(entity: Entity, event_type: &str, source: &str, dom: &mut EcsDom) {
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
pub(crate) fn clear_inline_handler(entity: Entity, event_type: &str, dom: &mut EcsDom) {
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

/// Shared harness for the unit cells (plan §4.1 "Unit (session)").
#[cfg(test)]
mod test_support {
    use elidex_ecs::{Attributes, EcsDom, Entity, MutationDispatcher, MutationEvent};
    use elidex_plugin::SpecLevelPolicy;

    use crate::{EventHandlerAttributeConsumer, EventListeners};

    /// Test-local [`MutationDispatcher`] wrapping only the consumer under test.
    struct ConsumerHook(EventHandlerAttributeConsumer);

    impl MutationDispatcher for ConsumerHook {
        fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
            self.0.handle(event, dom);
        }
    }

    /// Install a dispatcher holding a fresh consumer built with `policy`.
    pub(super) fn install(dom: &mut EcsDom, policy: SpecLevelPolicy) {
        let displaced = dom.set_mutation_dispatcher(Box::new(ConsumerHook(
            EventHandlerAttributeConsumer::with_policy(policy),
        )));
        assert!(displaced.is_none(), "one dispatcher per test DOM");
    }

    /// Run the bind pass with a **fresh** consumer, as production rebuilds the
    /// consumer at every bind (umbrella H13).
    pub(super) fn init_fresh(dom: &mut EcsDom, document: Entity, policy: SpecLevelPolicy) {
        EventHandlerAttributeConsumer::with_policy(policy).initialize_from_tree(dom, document);
    }

    /// Spawn `tag` with `attrs` baked into `Attributes` (no `AttributeChange`,
    /// the parser / clone shape), not inserted.
    pub(super) fn elem(dom: &mut EcsDom, tag: &str, attrs: &[(&str, &str)]) -> Entity {
        let mut a = Attributes::default();
        for (name, value) in attrs {
            a.set(*name, *value);
        }
        dom.create_element(tag, a)
    }

    /// Append, asserting the tree operation succeeded.
    pub(super) fn append(dom: &mut EcsDom, parent: Entity, child: Entity) {
        assert!(dom.append_child(parent, child), "append_child failed");
    }

    /// Pre-bind page: `document_root()` → `html` → `body`, built with no
    /// dispatcher installed. Returns `(document, body)`.
    pub(super) fn page(dom: &mut EcsDom) -> (Entity, Entity) {
        let doc = dom.create_document_root();
        let html = elem(dom, "html", &[]);
        let body = elem(dom, "body", &[]);
        append(dom, doc, html);
        append(dom, html, body);
        (doc, body)
    }

    /// A handler entry's store state, as the unit oracles read it.
    #[derive(Debug, PartialEq, Eq)]
    pub(super) enum Entry {
        /// No handler entry for the event type on the target.
        Absent,
        /// The entry exists with no uncompiled source (compiled or IDL-written).
        NoSource,
        /// The entry holds this raw uncompiled source.
        Raw(String),
    }

    pub(super) fn raw(src: &str) -> Entry {
        Entry::Raw(src.to_owned())
    }

    /// The `event_type` handler entry's store state on `target`.
    pub(super) fn entry(dom: &EcsDom, target: Entity, event_type: &str) -> Entry {
        let Ok(listeners) = dom.world().get::<&EventListeners>(target) else {
            return Entry::Absent;
        };
        let Some(id) = listeners.find_event_handler(event_type) else {
            return Entry::Absent;
        };
        listeners
            .uncompiled_source(id)
            .map_or(Entry::NoSource, |s| Entry::Raw(s.to_owned()))
    }

    /// Drop the entry's uncompiled source (the lazy compile's store effect).
    pub(super) fn take_uncompiled(dom: &mut EcsDom, target: Entity, event_type: &str) {
        let mut listeners = dom
            .world_mut()
            .get::<&mut EventListeners>(target)
            .expect("target has EventListeners");
        let id = listeners
            .find_event_handler(event_type)
            .expect("handler entry exists");
        assert!(
            listeners.take_uncompiled(id).is_some(),
            "entry had a source"
        );
    }

    pub(super) fn is_marked(dom: &EcsDom, e: Entity) -> bool {
        super::is_processed(dom, e)
    }

    pub(super) fn compat() -> SpecLevelPolicy {
        elidex_plugin::EngineMode::BrowserCompat.spec_level_policy()
    }
}

#[cfg(test)]
mod tests_bind;
#[cfg(test)]
mod tests_insertion_and_write;
