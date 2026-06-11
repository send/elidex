mod clone;
mod contains_compare;
mod equality_owner;
mod normalize_text;

use super::*;
use elidex_ecs::{
    Attributes, CommentData, DocTypeData, EcsDom, Entity, InlineStyle, TagType, TextContent,
};
use elidex_plugin::JsValue;
use elidex_script_session::{
    ComponentKind, DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore,
};

fn setup() -> (EcsDom, SessionCore) {
    (EcsDom::new(), SessionCore::new())
}

fn wrap(entity: Entity, session: &mut SessionCore) -> u64 {
    session
        .get_or_create_wrapper(entity, ComponentKind::Element)
        .to_raw()
}

fn obj_ref_arg(entity: Entity, session: &mut SessionCore) -> JsValue {
    JsValue::ObjectRef(wrap(entity, session))
}
