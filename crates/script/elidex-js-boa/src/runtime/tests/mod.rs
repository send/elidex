use super::*;
use elidex_ecs::{Attributes, EcsDom};
use elidex_plugin::{EventPayload, MouseEventInit};

fn setup() -> (JsRuntime, SessionCore, EcsDom, Entity) {
    // Use a real broker so Worker constructor and fetch() work in tests.
    let np = elidex_net::broker::spawn_network_process(elidex_net::NetClient::new());
    let nh = std::rc::Rc::new(np.create_renderer_handle());
    std::mem::forget(np); // Keep broker alive for test duration.
    let runtime = JsRuntime::with_network(Some(nh));
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
