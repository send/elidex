//! Fixtures shared by the element handler test modules.

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_script_session::{ComponentKind, SessionCore};

/// A `div` parent and a detached `span` child, both registered with the
/// session so tests can pass them as `ObjectRef` arguments.
pub(super) fn setup() -> (EcsDom, Entity, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    let mut session = SessionCore::new();
    // Pre-register entities so we can pass ObjectRef args.
    session.get_or_create_wrapper(parent, ComponentKind::Element);
    session.get_or_create_wrapper(child, ComponentKind::Element);
    (dom, parent, child, session)
}
