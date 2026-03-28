mod mutations;
mod selectors;
mod validation;

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_script_session::{ComponentKind, SessionCore};

/// Create a simple DOM: doc > body > [div, span, p]
fn setup() -> (EcsDom, Entity, Entity, Entity, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let body = dom.create_element("body", Attributes::default());
    let div = dom.create_element("div", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    let p = dom.create_element("p", Attributes::default());
    dom.append_child(body, div);
    dom.append_child(body, span);
    dom.append_child(body, p);

    let mut session = SessionCore::new();
    session.get_or_create_wrapper(body, ComponentKind::Element);
    session.get_or_create_wrapper(div, ComponentKind::Element);
    session.get_or_create_wrapper(span, ComponentKind::Element);
    session.get_or_create_wrapper(p, ComponentKind::Element);

    (dom, body, div, span, p, session)
}
