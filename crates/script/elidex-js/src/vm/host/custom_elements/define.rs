//! `customElements.define(name, constructor, options?)` — HTML §4.13.4
//! "Element definition" algorithm.

#![cfg(feature = "engine")]

use elidex_custom_elements::{CustomElementDefinition, CustomElementReaction, DefineError};

use super::super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::require_ce_registry_receiver;

const MAX_OBSERVED_ATTRIBUTES: usize = 1000;

pub(crate) fn native_ce_define(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_ce_registry_receiver(ctx, this, "define")?;
    // Post-unbind silent no-op (retained `customElements` reference).
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }

    // WebIDL signature requires 2 arguments (name, constructor) —
    // match Chrome / Firefox arg-count wording.
    if args.len() < 2 {
        return Err(VmError::type_error(format!(
            "Failed to execute 'define' on 'CustomElementRegistry': 2 arguments required, \
             but only {} present.",
            args.len()
        )));
    }

    // 1. ToString name (WebIDL DOMString) — `JsValue::String` is the
    //    only no-op path; everything else goes through the standard
    //    coercion.
    let name = coerce_to_string(ctx, args[0])?;

    // 2. Validate constructor is a callable construct-able object per
    //    WebIDL CustomElementConstructor (§4.13.4).
    let ctor_id = match args[1] {
        JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'define' on 'CustomElementRegistry': \
                 parameter 2 is not a constructor.",
            ));
        }
    };

    // 3. options.extends — v1 rejects customized built-in elements via
    //    NotSupportedError (`#11-customized-built-in-elements` defer
    //    slot).  Missing / undefined / null = autonomous custom element.
    if let Some(extends_value) = read_extends_option(ctx, args.get(2).copied())? {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_not_supported_error,
            format!(
                "Failed to execute 'define' on 'CustomElementRegistry': customized \
                 built-in elements (extends '{extends_value}') are not supported."
            ),
        ));
    }

    // 4. Read `constructor.observedAttributes` static getter and
    //    coerce as `Sequence<DOMString>` (HTML §4.13.4 step 14.6 +
    //    WebIDL §3.10.21).  An exception inside the getter / ToString
    //    coercion propagates out of define() per spec.
    let observed_attributes = read_observed_attributes(ctx, ctor_id)?;

    // 5. Register the definition (delegates name-validity check +
    //    duplicate-name check + pending-upgrade drain).  Allocate a
    //    fresh per-VM constructor ID, store the JS callable, and on
    //    `define` Err remove the orphaned constructor from the map.
    //    `ce_next_constructor_id` is NOT decremented — IDs may be
    //    skipped on failed defines; the counter is a unique-tag source,
    //    not a dense-index, so gaps are harmless (every live def holds
    //    its own ID, no consumer expects monotonic-no-gaps).
    let host = ctx.host();
    let constructor_id_u64 = host.ce_next_constructor_id;
    host.ce_next_constructor_id = host.ce_next_constructor_id.wrapping_add(1);
    host.ce_constructors.insert(constructor_id_u64, ctor_id);
    let definition = CustomElementDefinition {
        name: name.clone(),
        constructor_id: constructor_id_u64,
        observed_attributes,
        extends: None,
    };
    let pending = {
        let mut registry = host.ce_registry.lock().expect("CE registry mutex poisoned");
        registry.define(definition)
    };
    let pending_entities = match pending {
        Ok(entities) => entities,
        Err(err) => {
            host.ce_constructors.remove(&constructor_id_u64);
            return Err(define_error_to_vm_error(&ctx.vm.well_known, err));
        }
    };

    // 6. Enqueue Upgrade reactions for elements that were waiting in
    //    the pending-upgrade queue under this name. Retain the list so
    //    Step 7's world-walk can skip them — same entities re-found by
    //    the query would double-enqueue Upgrade (the dup would no-op
    //    via invoke_upgrade's Custom|Failed early-return, but it still
    //    inflates MAX_CE_DRAIN_ITERATIONS pressure).
    {
        let mut queue = host
            .ce_reaction_queue
            .lock()
            .expect("CE reaction queue mutex poisoned");
        for entity in &pending_entities {
            queue.push_back(CustomElementReaction::Upgrade(*entity));
        }
    }

    // 7. Walk every entity in the world carrying
    //    `CustomElementState::undefined(name)` and enqueue Upgrade —
    //    covers parser-baked elements (state attached but never queued
    //    because the parser cannot reach the per-VM registry) AND
    //    detached `createElement`-baked elements not in the
    //    pending_upgrade queue. Skips the entities already enqueued in
    //    Step 6 to avoid double-enqueue.
    enqueue_upgrade_walk(ctx, &name, &pending_entities);

    // 8. Resolve any pending whenDefined() Promises for this name.
    resolve_when_defined(ctx, &name, ctor_id);

    // 9. Flush queued reactions (Upgrades for the just-defined name
    //    fire synchronously per HTML §4.13.4 step 16 + §4.13.3 — the
    //    spec gates the entire define() body inside a CE reactions
    //    stack frame).
    ctx.vm.flush_ce_reactions();

    Ok(JsValue::Undefined)
}

/// ToString coercion (WebIDL DOMString). Strings short-circuit; other
/// kinds go through the standard `ToString` algorithm.
fn coerce_to_string(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<String, VmError> {
    let sid = super::super::super::coerce::to_string(ctx.vm, value)?;
    Ok(ctx.vm.strings.get_utf8(sid).clone())
}

/// Read `options.extends` if `options` is provided. Returns the
/// extends string when present (rejected as NotSupportedError by the
/// caller), `Ok(None)` when omitted / `undefined` / `null`.
fn read_extends_option(
    ctx: &mut NativeContext<'_>,
    opts: Option<JsValue>,
) -> Result<Option<String>, VmError> {
    // WebIDL §3.10.20 "dictionary": null and undefined become an
    // empty dictionary; non-object non-null values throw TypeError.
    // (Chrome accepts non-object loosely; spec-strict rejects.)
    let Some(opts_value) = opts else {
        return Ok(None);
    };
    let opts_id = match opts_value {
        JsValue::Undefined | JsValue::Null => return Ok(None),
        JsValue::Object(id) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'define' on 'CustomElementRegistry': \
                 parameter 3 is not an object.",
            ));
        }
    };
    let key = super::super::super::value::PropertyKey::String(ctx.vm.well_known.extends);
    let extends_val = ctx.vm.get_property_value(opts_id, key)?;
    match extends_val {
        JsValue::Undefined | JsValue::Null => Ok(None),
        other => Ok(Some(coerce_to_string(ctx, other)?)),
    }
}

/// Read the static `constructor.observedAttributes` accessor and
/// coerce its result into a `Vec<String>` per WebIDL
/// `Sequence<DOMString>` (§3.10.16). Preserves order, deduplicates,
/// ASCII-case-folds to lowercase (matches Chrome / Firefox + the
/// boa-side baseline). Errors inside the getter / iterator protocol
/// propagate out of `define()` per HTML §4.13.4.
fn read_observed_attributes(
    ctx: &mut NativeContext<'_>,
    ctor_id: ObjectId,
) -> Result<Vec<String>, VmError> {
    let observed_key =
        super::super::super::value::PropertyKey::String(ctx.vm.well_known.observed_attributes);
    let observed_val = ctx.vm.get_property_value(ctor_id, observed_key)?;
    if matches!(observed_val, JsValue::Undefined | JsValue::Null) {
        return Ok(Vec::new());
    }
    let msgs = super::super::super::webidl_sequence::SeqMessages {
        not_iterable: "Failed to execute 'define' on 'CustomElementRegistry': \
             observedAttributes must be an iterable of strings.",
        iter_not_object: "Failed to execute 'define' on 'CustomElementRegistry': \
             observedAttributes iterator did not yield an object.",
        cap_exceeded: "Failed to execute 'define' on 'CustomElementRegistry': \
             observedAttributes exceeds the implementation limit.",
    };
    let items = super::super::super::webidl_sequence::webidl_sequence_to_vec(
        ctx,
        observed_val,
        MAX_OBSERVED_ATTRIBUTES,
        &msgs,
        |ctx, _idx, value| coerce_to_string(ctx, value).map(|s| s.to_ascii_lowercase()),
    )?;
    // HashSet for O(N) dedup while preserving first-seen ordering.
    let mut seen: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(items.len());
    let mut deduped: Vec<String> = Vec::with_capacity(items.len());
    for s in items {
        if seen.insert(s.clone()) {
            deduped.push(s);
        }
    }
    Ok(deduped)
}

/// Map [`DefineError`] to the canonical DOMException name per HTML §4.13.4.
fn define_error_to_vm_error(
    well_known: &super::super::super::well_known::WellKnownStrings,
    err: DefineError,
) -> VmError {
    match err {
        DefineError::InvalidName(name) => VmError::dom_exception(
            well_known.dom_exc_syntax_error,
            format!("'{name}' is not a valid custom element name"),
        ),
        DefineError::AlreadyDefined(name) => VmError::dom_exception(
            well_known.dom_exc_not_supported_error,
            format!("'{name}' has already been defined as a custom element"),
        ),
    }
}

/// Enqueue Upgrade reactions for every entity in the world carrying
/// `CustomElementState::Undefined` for `name`. Covers the parser /
/// innerHTML path (where the parser attached the state component but
/// could not reach the per-VM registry to call `queue_for_upgrade`)
/// AND the detached-pre-define path (entities created via
/// `createElement` before define then orphaned — `pending_upgrade`
/// drain in step 6 covers those, this walk dedups but skips entities
/// already covered by `skip_already_pending`).
///
/// Uses a world-wide hecs query rather than a tree walk so detached
/// elements (orphans + DocumentFragment subtrees + future
/// multi-document) are not silently missed (the document-rooted walk
/// would have skipped them).
fn enqueue_upgrade_walk(
    ctx: &mut NativeContext<'_>,
    name: &str,
    skip_already_pending: &[elidex_ecs::Entity],
) {
    let Some(host) = ctx.host_if_bound() else {
        return;
    };
    let to_upgrade = {
        let dom = host.dom_shared();
        elidex_custom_elements::collect_undefined_entities(dom.world(), name, skip_already_pending)
    };
    if to_upgrade.is_empty() {
        return;
    }
    let mut queue = host
        .ce_reaction_queue
        .lock()
        .expect("CE reaction queue mutex poisoned");
    for entity in to_upgrade {
        queue.push_back(CustomElementReaction::Upgrade(entity));
    }
}

/// Resolve every pending `whenDefined(name)` Promise with `ctor_id`.
fn resolve_when_defined(ctx: &mut NativeContext<'_>, name: &str, ctor_id: ObjectId) {
    let promise_to_resolve = {
        let host = ctx.host();
        host.ce_when_defined_resolvers.remove(name);
        host.ce_when_defined_promises.remove(name)
    };
    // The resolver-callbacks map entries hold `PromiseResolver`
    // function ObjectIds bound to the SAME `promise_to_resolve`
    // (`lookup.rs::native_ce_when_defined` mints both at once and
    // never adds a second resolver per name). Settling the promise
    // directly is sufficient — the resolver function's effect is
    // identical to `settle_promise` and would no-op via the
    // already_resolved idempotency flag anyway. We drop the resolvers
    // map entry above purely to release the GC root on the resolver
    // function.
    if let Some(promise_id) = promise_to_resolve {
        let _ = super::super::super::natives_promise::settle_promise(
            ctx.vm,
            promise_id,
            false,
            JsValue::Object(ctor_id),
        );
    }
}
