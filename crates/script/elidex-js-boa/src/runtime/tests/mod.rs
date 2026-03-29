use super::*;
use elidex_ecs::{Attributes, EcsDom};
use elidex_plugin::{EventPayload, MouseEventInit};

fn setup() -> (JsRuntime, SessionCore, EcsDom, Entity) {
    let runtime = JsRuntime::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (runtime, session, dom, doc)
}

mod custom_elements;
mod dom_api;
mod events;
mod observers;
mod web_apis;
