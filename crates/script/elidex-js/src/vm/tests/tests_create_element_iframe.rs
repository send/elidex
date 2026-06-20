//! `document.createElement('iframe')` must attach the `IframeData` component
//! at creation (WHATWG HTML §4.8.5), the same as the parser path — otherwise a
//! script-created iframe is invisible to the shell loader. Regression for the
//! `#11-createelement-element-init-derivation` slot: the createElement handler
//! and the parser now funnel through one shared derivation
//! (`elidex_custom_elements::derive_created_element_components`).

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity, IframeData, TagType};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

/// Find the single entity whose `TagType` equals `tag` (test DOMs are tiny).
fn find_by_tag(dom: &EcsDom, tag: &str) -> Option<Entity> {
    dom.world()
        .query::<(Entity, &TagType)>()
        .iter()
        .find_map(|(e, t)| (t.0 == tag).then_some(e))
}

#[test]
fn create_element_iframe_attaches_iframe_data() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // A bare createElement('iframe') (no attributes, detached) — the entity
    // still lives in the dom.
    vm.eval("document.createElement('iframe');").unwrap();
    vm.unbind();

    let iframe = find_by_tag(&dom, "iframe").expect("createElement created an iframe entity");
    assert!(
        dom.world().get::<&IframeData>(iframe).is_ok(),
        "createElement('iframe') must attach IframeData (else invisible to the shell loader)"
    );
    // Present-default: src/srcdoc unset until setAttribute fires.
    let data = dom.world().get::<&IframeData>(iframe).unwrap();
    assert!(data.src.is_none());
    assert!(data.srcdoc.is_none());
}

#[test]
fn create_element_div_does_not_attach_iframe_data() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("document.createElement('div');").unwrap();
    vm.unbind();

    let div = find_by_tag(&dom, "div").expect("createElement created a div entity");
    assert!(
        dom.world().get::<&IframeData>(div).is_err(),
        "a non-iframe element must not receive IframeData"
    );
}
