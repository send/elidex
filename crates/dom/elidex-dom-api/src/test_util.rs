//! Crate-internal test fixtures shared across handler test modules.

use elidex_ecs::Entity;
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
