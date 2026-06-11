//! Node interface methods and ChildNode/ParentNode mixin methods.

use boa_engine::{js_string, JsValue, NativeFunction};
use elidex_plugin::JsValue as ElidexJsValue;

use crate::bridge::HostBridge;
use crate::globals::element::core::extract_entity;
use crate::globals::{
    boa_arg_to_elidex, boa_args_to_elidex, invoke_dom_handler, invoke_dom_handler_ref,
    invoke_dom_handler_void,
};

use boa_engine::object::ObjectInitializer;

/// Register node methods that take entity arguments.
pub(in crate::globals::element) fn register_node_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
) {
    // contains(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("contains", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("contains"),
        1,
    );

    // compareDocumentPosition(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("compareDocumentPosition", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("compareDocumentPosition"),
        1,
    );

    install_clone_node(init, bridge);

    // normalize()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_void("normalize", entity, &[], bridge)
            },
            b,
        ),
        js_string!("normalize"),
        0,
    );

    // getRootNode()
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_ref("getRootNode", entity, &[], bridge, ctx)
            },
            b,
        ),
        js_string!("getRootNode"),
        0,
    );

    // isSameNode(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("isSameNode", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("isSameNode"),
        1,
    );

    // isEqualNode(other)
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let other =
                    boa_arg_to_elidex(args.first().unwrap_or(&JsValue::null()), bridge, ctx)?;
                invoke_dom_handler("isEqualNode", entity, &[other], bridge)
            },
            b,
        ),
        js_string!("isEqualNode"),
        1,
    );
}

/// Register variadic ChildNode/ParentNode mixin methods (before, after, remove, etc.).
pub(in crate::globals::element) fn register_child_parent_mixin_methods(
    init: &mut ObjectInitializer<'_>,
    bridge: &HostBridge,
) {
    // Variadic methods: before, after, replaceWith, prepend, append, replaceChildren.
    static VARIADIC_METHODS: &[&str] = &[
        "before",
        "after",
        "replaceWith",
        "prepend",
        "append",
        "replaceChildren",
    ];
    for &method_name in VARIADIC_METHODS {
        let b = bridge.clone();
        init.function(
            NativeFunction::from_copy_closure_with_captures(
                move |this, args, bridge, ctx| {
                    let entity = extract_entity(this, ctx)?;
                    let elidex_args = boa_args_to_elidex(args, bridge, ctx)?;
                    invoke_dom_handler_void(method_name, entity, &elidex_args, bridge)
                },
                b,
            ),
            js_string!(method_name),
            0,
        );
    }

    // remove() — no args.
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, _args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                invoke_dom_handler_void("remove", entity, &[], bridge)
            },
            b,
        ),
        js_string!("remove"),
        0,
    );

    // Namespace API (WHATWG DOM §4.4) — HTML documents always return null/xhtml.
    init.function(
        NativeFunction::from_fn_ptr(|_this, _args, _ctx| Ok(JsValue::null())),
        js_string!("lookupPrefix"),
        1,
    );
    init.function(
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let prefix = args
                .first()
                .filter(|v| !v.is_undefined() && !v.is_null())
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped());
            match prefix.as_deref() {
                None | Some("") => Ok(JsValue::from(js_string!("http://www.w3.org/1999/xhtml"))),
                _ => Ok(JsValue::null()),
            }
        }),
        js_string!("lookupNamespaceURI"),
        1,
    );
    init.function(
        NativeFunction::from_copy_closure(|_this, args, ctx| {
            let ns = args
                .first()
                .filter(|v| !v.is_undefined() && !v.is_null())
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped());
            Ok(JsValue::from(
                ns.as_deref() == Some("http://www.w3.org/1999/xhtml"),
            ))
        }),
        js_string!("isDefaultNamespace"),
        1,
    );
}

/// `cloneNode(deep?)` — split out of `register_node_methods` (clippy
/// fn-length) and carrying the clone-time upgrade-reaction seam.
fn install_clone_node(init: &mut ObjectInitializer<'_>, bridge: &HostBridge) {
    let b = bridge.clone();
    init.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let entity = extract_entity(this, ctx)?;
                let deep = args.first().is_some_and(JsValue::to_boolean);
                let result = invoke_dom_handler_ref(
                    "cloneNode",
                    entity,
                    &[ElidexJsValue::Bool(deep)],
                    bridge,
                    ctx,
                )?;
                // DOM §4.4 "clone a single node": a clone whose
                // definition lookup is non-null gets an upgrade
                // reaction enqueued at clone time (it must not stay
                // `Undefined` until a later insertion / upgrade()
                // call). The walk is registry- and local-name-gated.
                if let Ok(clone_root) = extract_entity(&result, ctx) {
                    bridge.with(|_session, dom| {
                        bridge.apply_clone_ce_creation_pass(dom, clone_root);
                    });
                }
                Ok(result)
            },
            b,
        ),
        js_string!("cloneNode"),
        1,
    );
}
