//! `customElements.get(name)` / `customElements.whenDefined(name)` /
//! `customElements.upgrade(root)` — HTML §4.13.4.

#![cfg(feature = "engine")]

use elidex_custom_elements::{CEState, CustomElementState};

use super::super::super::natives_promise;
use super::super::super::value::{JsValue, NativeContext, VmError};
use super::require_ce_registry_receiver;

pub(crate) fn native_ce_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_ce_registry_receiver(ctx, this, "get")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let name = match args.first().copied() {
        Some(value) => coerce_to_string(ctx, value)?,
        None => return Ok(JsValue::Undefined),
    };
    let host = ctx.host();
    let constructor_id_u64 = {
        let registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
        match registry.get(&name) {
            Some(def) => def.constructor_id,
            None => return Ok(JsValue::Undefined),
        }
    };
    Ok(host
        .ce_constructors
        .get(&constructor_id_u64)
        .copied()
        .map_or(JsValue::Undefined, JsValue::Object))
}

pub(crate) fn native_ce_when_defined(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_ce_registry_receiver(ctx, this, "whenDefined")?;
    if ctx.host_if_bound().is_none() {
        // Promise still has to be observable as a Promise even on a
        // post-unbind retained `customElements` reference; rejecting
        // here would surprise scripts that rely on `whenDefined()`
        // returning a thenable. Hand back a rejected Promise.
        let promise = natives_promise::create_promise(ctx.vm);
        let reason = VmError::type_error(
            "Failed to execute 'whenDefined' on 'CustomElementRegistry': \
             host environment is not bound.",
        );
        let reason_value = ctx.vm.vm_error_to_thrown(&reason);
        let _ = natives_promise::settle_promise(ctx.vm, promise, true, reason_value);
        return Ok(JsValue::Object(promise));
    }

    let name = match args.first().copied() {
        Some(value) => coerce_to_string(ctx, value)?,
        None => String::new(),
    };

    // Invalid-name path returns a rejected Promise per HTML §4.13.4
    // step 2 — the spec uses a SyntaxError DOMException, not a
    // synchronous throw.
    if !elidex_custom_elements::is_valid_custom_element_name(&name) {
        let promise = natives_promise::create_promise(ctx.vm);
        let reason = VmError::dom_exception(
            ctx.vm.well_known.dom_exc_syntax_error,
            format!("'{name}' is not a valid custom element name"),
        );
        let reason_value = ctx.vm.vm_error_to_thrown(&reason);
        let _ = natives_promise::settle_promise(ctx.vm, promise, true, reason_value);
        return Ok(JsValue::Object(promise));
    }

    // Already-defined fast path: resolved Promise wrapping the
    // constructor.
    let ctor_id_opt: Option<super::super::super::value::ObjectId> = {
        let host = ctx.host();
        let registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
        registry
            .get(&name)
            .map(|d| d.constructor_id)
            .and_then(|cid| host.ce_constructors.get(&cid).copied())
    };
    if let Some(ctor_id) = ctor_id_opt {
        let promise = natives_promise::create_promise(ctx.vm);
        let _ = natives_promise::settle_promise(ctx.vm, promise, false, JsValue::Object(ctor_id));
        return Ok(JsValue::Object(promise));
    }

    // Pending — return the previously stored Promise if there is one
    // (per spec §4.13.4 step 3 "promise" is reused across calls), else
    // mint a fresh Promise + register a resolver.
    if let Some(cached) = ctx.host().ce_when_defined_promises.get(&name).copied() {
        return Ok(JsValue::Object(cached));
    }
    let promise = natives_promise::create_promise(ctx.vm);
    let (resolve, _reject) = natives_promise::create_resolver_pair(ctx.vm, promise);
    let host = ctx.host();
    host.ce_when_defined_promises.insert(name.clone(), promise);
    host.ce_when_defined_resolvers
        .entry(name)
        .or_default()
        .push(resolve);
    Ok(JsValue::Object(promise))
}

pub(crate) fn native_ce_upgrade(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_ce_registry_receiver(ctx, this, "upgrade")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // WebIDL `Node root` — required. A missing / non-Node arg throws
    // a TypeError matching Chrome / Firefox wording.
    let root_value = args.first().copied().ok_or_else(|| {
        VmError::type_error(
            "Failed to execute 'upgrade' on 'CustomElementRegistry': \
             1 argument required, but only 0 present.",
        )
    })?;
    let root_entity = super::super::node_proto::require_node_arg(ctx, root_value, "upgrade")?;

    // Walk shadow-including descendants, collect candidates, then
    // synchronously upgrade — `customElements.upgrade()` is a sync API
    // per HTML §4.13.4. Each candidate's constructor exception is
    // isolated (eprintln + continue) rather than propagated so a
    // single bad element does not abort the remaining candidates in
    // the subtree (matches Blink's batch-upgrade isolation).
    let mut candidates: Vec<elidex_ecs::Entity> = Vec::new();
    {
        let host = ctx.host();
        let registry_arc = std::sync::Arc::clone(&host.ce_registry);
        let dom = host.dom_shared();
        dom.for_each_shadow_inclusive_descendant(root_entity, &mut |e| {
            if let Ok(state) = dom.world().get::<&CustomElementState>(e) {
                if matches!(state.state, CEState::Undefined) {
                    // Only enqueue if the definition is registered —
                    // pure Undefined-with-no-definition entities stay
                    // pending until the matching `define()` lands.
                    let registry = registry_arc.lock().expect("CE registry mutex poisoned");
                    if registry.is_defined(&state.definition_name) {
                        candidates.push(e);
                    }
                }
            }
        });
    }
    // Each candidate failure is isolated per HTML §4.13.4 — a thrown
    // constructor marks that element Failed and the remaining
    // candidates still upgrade. (Without this isolation, the first
    // throw would propagate via `?` and silently abandon every later
    // element in the subtree.)
    for entity in candidates {
        if let Err(err) = super::upgrade::invoke_upgrade(ctx, entity) {
            eprintln!("[CE Upgrade Error] {}", err.message);
        }
    }
    Ok(JsValue::Undefined)
}

fn coerce_to_string(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<String, VmError> {
    let sid = super::super::super::coerce::to_string(ctx.vm, value)?;
    Ok(ctx.vm.strings.get_utf8(sid).clone())
}
