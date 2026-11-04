//! Crate-internal test fixtures shared across handler test modules.

use std::sync::{Arc, Mutex};

use elidex_ecs::{Attributes, CloneScope, EcsDom, Entity, MutationDispatcher, MutationEvent};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, JsObjectRef, SessionCore};

/// Unwrap a handler's `ObjectRef` result back to its [`Entity`].
pub(crate) fn entity_of(r: &JsValue, session: &SessionCore) -> Entity {
    let JsValue::ObjectRef(ref_id) = r else {
        panic!("expected ObjectRef");
    };
    session
        .identity_map()
        .get(JsObjectRef::from_raw(*ref_id))
        .unwrap()
        .0
}

/// A [`MutationDispatcher`] that counts [`MutationEvent::AttributeChange`]
/// dispatches — shared by the attribute-write chokepoint-routing tests
/// (className/id/dataset setters + the DOMTokenList family) that assert each
/// write fires exactly one `AttributeChange` record.
#[derive(Default, Clone)]
pub(crate) struct AttrChangeCounter {
    pub(crate) count: Arc<Mutex<usize>>,
}

impl MutationDispatcher for AttrChangeCounter {
    fn dispatch(&mut self, event: &MutationEvent<'_>, _dom: &mut EcsDom) {
        if matches!(*event, MutationEvent::AttributeChange { .. }) {
            *self.count.lock().unwrap() += 1;
        }
    }
}

/// A [`MutationDispatcher`] that records every
/// [`MutationEvent::CloneCreated`] as `(root, scope)`, and — at dispatch —
/// whether the root already had its shadow root and its light children (the
/// "event fires after the whole copy exists" probe).
#[derive(Default, Clone)]
pub(crate) struct CloneCreatedProbe {
    pub(crate) events: Arc<Mutex<Vec<(Entity, CloneScope)>>>,
    /// `(root has a shadow root, root has light children)` per event.
    pub(crate) shape_at_dispatch: Arc<Mutex<Vec<(bool, bool)>>>,
}

impl MutationDispatcher for CloneCreatedProbe {
    fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        if let MutationEvent::CloneCreated { root, scope } = *event {
            self.events.lock().unwrap().push((root, scope));
            self.shape_at_dispatch.lock().unwrap().push((
                dom.get_shadow_root(root).is_some(),
                !dom.children(root).is_empty(),
            ));
        }
    }
}

/// A `div` parent and a detached `span` child, both registered with the
/// session so tests can pass them as `ObjectRef` arguments.
pub(crate) fn setup() -> (EcsDom, Entity, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    let mut session = SessionCore::new();
    // Pre-register entities so we can pass ObjectRef args.
    session.get_or_create_wrapper(parent, ComponentKind::Element);
    session.get_or_create_wrapper(child, ComponentKind::Element);
    (dom, parent, child, session)
}
