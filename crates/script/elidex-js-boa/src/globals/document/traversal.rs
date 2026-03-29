//! TreeWalker, NodeIterator, and Range JS object builders.
//!
//! Extracted from `document.rs` to keep file sizes manageable.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use elidex_ecs::Entity;

use crate::bridge::HostBridge;

/// Hidden property key for the traversal object ID.
const TRAVERSAL_ID_KEY: &str = "__elidex_traversal_id__";

/// Build a JS object wrapping a `TreeWalker`.
#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]
pub(crate) fn build_tree_walker_object(
    tw_id: u64,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);

    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!(TRAVERSAL_ID_KEY),
        JsValue::from(tw_id as f64),
        Attribute::empty(),
    );

    macro_rules! tw_method {
        ($name:expr, $method:ident) => {{
            let b = bridge.clone();
            init.function(
                NativeFunction::from_copy_closure_with_captures(
                    |this, _args, bridge, ctx| {
                        let id = extract_traversal_id(this, ctx)?;
                        let result = bridge.with(|_session, dom| {
                            bridge.with_tree_walker(id, |tw| tw.$method(dom))
                        });
                        match result {
                            Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                            _ => Ok(JsValue::null()),
                        }
                    },
                    b,
                ),
                js_string!($name),
                0,
            );
        }};
    }

    tw_method!("nextNode", next_node);
    tw_method!("previousNode", previous_node);
    tw_method!("parentNode", parent_node);
    tw_method!("firstChild", first_child);
    tw_method!("lastChild", last_child);
    tw_method!("nextSibling", next_sibling);
    tw_method!("previousSibling", previous_sibling);

    Ok(init.build().into())
}

/// Build a JS object wrapping a `NodeIterator`.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn build_node_iterator_object(
    ni_id: u64,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);

    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!(TRAVERSAL_ID_KEY),
        JsValue::from(ni_id as f64),
        Attribute::empty(),
    );

    // nextNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge
                    .with(|_session, dom| bridge.with_node_iterator(id, |ni| ni.next_node(dom)));
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("nextNode"),
        0,
    );

    // previousNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let result = bridge.with(|_session, dom| {
                    bridge.with_node_iterator(id, |ni| ni.previous_node(dom))
                });
                match result {
                    Some(Some(entity)) => Ok(resolve_entity_to_js(entity, bridge, ctx)),
                    _ => Ok(JsValue::null()),
                }
            },
            b,
        ),
        js_string!("previousNode"),
        0,
    );

    Ok(init.build().into())
}

/// Build a JS object wrapping a Range.
#[allow(clippy::too_many_lines)]
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn build_range_object(
    range_id: u64,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut init = ObjectInitializer::new(ctx);

    #[allow(clippy::cast_precision_loss)]
    init.property(
        js_string!(TRAVERSAL_ID_KEY),
        JsValue::from(range_id as f64),
        Attribute::empty(),
    );

    // collapsed (getter)
    let b = bridge.clone();
    let realm = init.context().realm().clone();
    let collapsed_getter = NativeFunction::from_copy_closure_with_captures(
        |this, _args, bridge, ctx| {
            let id = extract_traversal_id(this, ctx)?;
            let result = bridge.with_range(id, |r| r.collapsed());
            Ok(JsValue::from(result.unwrap_or(true)))
        },
        b,
    )
    .to_js_function(&realm);
    init.accessor(
        js_string!("collapsed"),
        Some(collapsed_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // setStart(node, offset)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let node = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ().with_message("setStart: node required")
                    })?,
                    ctx,
                )?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let offset = args
                    .get(1)
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                bridge.with_range(id, |r| r.set_start(node, offset));
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("setStart"),
        2,
    );

    // setEnd(node, offset)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let node = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ().with_message("setEnd: node required")
                    })?,
                    ctx,
                )?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let offset = args
                    .get(1)
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                bridge.with_range(id, |r| r.set_end(node, offset));
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("setEnd"),
        2,
    );

    // collapse(toStart?)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let to_start = args.first().is_some_and(JsValue::to_boolean);
                bridge.with_range(id, |r| r.collapse(to_start));
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("collapse"),
        0,
    );

    // selectNode(node)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let node = crate::globals::element::extract_entity(
                    args.first().ok_or_else(|| {
                        JsNativeError::typ().with_message("selectNode: node required")
                    })?,
                    ctx,
                )?;
                bridge.with(|_session, dom| {
                    bridge.with_range(id, |r| r.select_node(node, dom));
                });
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("selectNode"),
        1,
    );

    // toString()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                let text = bridge.with(|_session, dom| bridge.with_range(id, |r| r.to_string(dom)));
                Ok(JsValue::from(js_string!(text.unwrap_or_default())))
            },
            b,
        ),
        js_string!("toString"),
        0,
    );

    // deleteContents()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let id = extract_traversal_id(this, ctx)?;
                bridge.with(|_session, dom| {
                    bridge.with_range(id, |r| r.delete_contents(dom));
                });
                Ok(JsValue::undefined())
            },
            b,
        ),
        js_string!("deleteContents"),
        0,
    );

    Ok(init.build().into())
}

/// Extract the traversal ID from a JS object's hidden property.
pub(super) fn extract_traversal_id(this: &JsValue, ctx: &mut Context) -> JsResult<u64> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("traversal method called on non-object")
    })?;
    let id_val = obj.get(js_string!(TRAVERSAL_ID_KEY), ctx)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let id = id_val
        .as_number()
        .ok_or_else(|| JsNativeError::typ().with_message("invalid traversal object"))?
        as u64;
    Ok(id)
}

/// Resolve an Entity to a JS element wrapper.
pub(super) fn resolve_entity_to_js(
    entity: Entity,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    let (obj_ref, is_iframe) = bridge.with(|session, dom| {
        let kind = dom.node_kind(entity).map_or(
            elidex_script_session::ComponentKind::Element,
            elidex_script_session::ComponentKind::from_node_kind,
        );
        let r = session.get_or_create_wrapper(entity, kind);
        let iframe = dom
            .world()
            .get::<&elidex_ecs::TagType>(entity)
            .ok()
            .is_some_and(|t| t.0 == "iframe");
        (r, iframe)
    });
    crate::globals::element::create_element_wrapper(entity, bridge, obj_ref, ctx, is_iframe)
}
