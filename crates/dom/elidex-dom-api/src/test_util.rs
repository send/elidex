//! Crate-internal test fixtures shared across handler test modules.

use std::sync::{Arc, Mutex};

use elidex_ecs::{EcsDom, Entity, MutationDispatcher, MutationEvent};
use elidex_plugin::JsValue;
use elidex_script_session::{JsObjectRef, SessionCore};

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
