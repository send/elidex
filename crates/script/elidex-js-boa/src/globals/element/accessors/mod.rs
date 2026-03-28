//! Element accessor registration — Node methods, Element methods, property accessors,
//! layout queries, dataset, and classList.

mod collections;
mod element_methods;
mod layout_queries;
mod node_methods;
mod properties;

pub(crate) use collections::{create_class_list_object, register_cached_accessor};

use boa_engine::object::ObjectInitializer;

use crate::bridge::HostBridge;

/// Register all element accessors from sub-modules.
pub(crate) fn register_element_accessors(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
    realm: &boa_engine::realm::Realm,
) {
    node_methods::register_node_methods(init, bridge);
    node_methods::register_child_parent_mixin_methods(init, bridge);
    element_methods::register_element_extra_methods(init, bridge);
    properties::register_element_extra_accessors(init, bridge, realm);
    layout_queries::register_layout_query_accessors(init, bridge, realm);
    collections::register_dataset_accessor(init, bridge, realm);
}
