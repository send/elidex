//! Event-handler content attributes processed once per origin element
//! (umbrella `docs/plans/2026-10-event-handler-attrs-at-creation.md`,
//! slice a; per-PR plan `docs/plans/2026-10-eh-attrs-slice-a.md`).
//!
//! **Scaffold (plan §1.8 step 1).** Every public name the slice adds exists
//! here with a HEAD-equivalent body: both markers are declared and never
//! inserted, and [`process_before_handler_write`] is a no-op. The X-W
//! classifier is the one pure new function written in full, so plan cell P2
//! can pin it. The fix commit (plan §1.4–§1.6) replaces the bodies and moves
//! the attribute-change-steps helpers here from `event_handler_consumer.rs`.

use elidex_ecs::{EcsDom, Entity, TagType};

use crate::event_handler_consumer::{HandlerScope, EVENT_HANDLER_ATTRS};

/// The origin element's creation-time event-handler content attributes have
/// been processed (umbrella D-1). Unit, `Send + Sync`, sparse: inserted only
/// on an element that carries at least one handler content attribute of an
/// ordinary pair.
///
/// Scaffold: declared, never inserted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HandlerAttrsProcessed;

/// The bound Document's pre-bind tree has had its handler content attributes
/// processed by the bind pass (umbrella D-5). Unit, on the Document entity.
///
/// Scaffold: declared, never inserted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreBindHandlerAttrsProcessed;

/// WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and
/// Window objects (`#window-reflecting-body-element-event-handler-set`).
pub const WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET: [&str; 6] = [
    "onblur", "onerror", "onfocus", "onload", "onresize", "onscroll",
];

/// Steps 1–2 of *determining the target of an event handler* (WHATWG HTML
/// §8.1.8.1 Event handlers): `true` iff those steps do not return the
/// element. Reads the tag and the name only (umbrella X-W).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "scaffold: wired by the fix commit (plan §1.8)")
)]
pub(crate) fn is_window_delegating_pair(dom: &EcsDom, element: Entity, name: &str) -> bool {
    is_body_or_frameset(dom, element)
        && (EVENT_HANDLER_ATTRS
            .iter()
            .any(|(attr, scope)| *attr == name && *scope == HandlerScope::Window)
            || WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET.contains(&name))
}

/// `true` if `element` is a `body` or a `frameset` element, its tag compared
/// ASCII-case-insensitively as HEAD's `is_body_element` compares it.
pub(crate) fn is_body_or_frameset(dom: &EcsDom, element: Entity) -> bool {
    dom.world()
        .get::<&TagType>(element)
        .is_ok_and(|t| t.0.eq_ignore_ascii_case("body") || t.0.eq_ignore_ascii_case("frameset"))
}

/// Write-side entry, called before a handler write (umbrella D-4 / D-8).
///
/// Scaffold: a no-op, HEAD's behaviour (HEAD processes nothing at a write).
pub fn process_before_handler_write(
    dom: &mut EcsDom,
    element: Entity,
    name: &str,
    policy: elidex_plugin::SpecLevelPolicy,
) {
    let _ = (dom, element, name, policy);
}

/// Shared harness for the unit cells (plan §4.1 "Unit (session)").
#[cfg(test)]
mod test_support {
    use elidex_ecs::{Attributes, EcsDom, Entity, MutationDispatcher, MutationEvent};
    use elidex_plugin::SpecLevelPolicy;

    use super::HandlerAttrsProcessed;
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
        dom.world().get::<&HandlerAttrsProcessed>(e).is_ok()
    }

    pub(super) fn compat() -> SpecLevelPolicy {
        elidex_plugin::EngineMode::BrowserCompat.spec_level_policy()
    }
}

#[cfg(test)]
mod tests_bind;
#[cfg(test)]
mod tests_insertion_and_write;
