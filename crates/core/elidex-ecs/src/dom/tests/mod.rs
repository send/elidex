use super::*;
use crate::components::{Attributes, ShadowRootMode, SlotAssignment, TextContent};

fn elem(dom: &mut EcsDom, tag: &'static str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

mod creation;
mod destroy;
mod shadow_dom;
mod tree_ops;
