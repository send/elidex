//! `customElements.define(name, constructor, options?)` — HTML §4.13.4
//! "Element definition" algorithm.

#![cfg(feature = "engine")]

use std::sync::PoisonError;

use elidex_custom_elements::{CustomElementDefinition, CustomElementReaction, DefineError};

use super::super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::require_ce_registry_receiver;

const MAX_OBSERVED_ATTRIBUTES: usize = 1000;

#[allow(clippy::too_many_lines)] // step-by-step HTML §4.13.4 algorithm walk; splitting would obscure the spec correspondence
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

    // 2. Validate constructor has [[Construct]] per WebIDL
    //    `CustomElementConstructor` (HTML §4.13.4). Routed via the
    //    project-wide `IsConstructor` abstract-op helper so arrow
    //    functions / `Function.prototype.bind` of non-ctor targets /
    //    Promise resolver objects / generator functions all reject
    //    here (callable ≠ constructable).
    let ctor_id = match args[1] {
        JsValue::Object(id) if super::super::super::object_kind::is_constructor(ctx.vm, id) => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'define' on 'CustomElementRegistry': \
                 parameter 2 is not a constructor.",
            ));
        }
    };

    // 2. HTML §4.13.4 step 2: validate `name` against the custom
    // element production. Runs BEFORE the brand check (2b) and
    // duplicate-constructor check (2d) so an invalid name throws
    // SyntaxError regardless of constructor shape (D-17b R14 G14-1
    // spec-ordering fix — the brand check would otherwise preempt
    // and throw TypeError on `define('x', class {})`).
    if !elidex_custom_elements::is_valid_custom_element_name(&name) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_syntax_error,
            format!(
                "Failed to execute 'define' on 'CustomElementRegistry': \
                 \"{name}\" is not a valid custom element name."
            ),
        ));
    }

    // 2a. HTML §4.13.4 step 3: name already registered → NotSupportedError.
    // Early peek via `CustomElementRegistry::is_defined` so this fires
    // BEFORE the brand check (per spec ordering); the registry's own
    // `AlreadyDefined` check at `registry.define()` below is the
    // authoritative gate (defense-in-depth — won't fire after this
    // early peek + the host-bound lock-once discipline).
    let already_defined = {
        let registry = ctx
            .host()
            .ce_registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        registry.is_defined(&name)
    };
    if already_defined {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_not_supported_error,
            format!(
                "Failed to execute 'define' on 'CustomElementRegistry': \
                 the name \"{name}\" has already been defined as a custom element."
            ),
        ));
    }

    // 2b. HTML §4.13.4 step 4: reject re-use of the same constructor
    // with this registry. The reverse map `ce_constructor_to_id`
    // (D-17b R2 G1) is the SoT for "is this ctor already registered?"
    // — used by `native_html_element_ctor` to resolve `new.target` →
    // `constructor_id`, so an overwrite here would silently alias
    // `new.target` from the FIRST define call to the SECOND
    // definition. Fires BEFORE any host bookkeeping (no rollback
    // needed). Mirrors the registry's own name-uniqueness check
    // (`DefineError::AlreadyDefined`) for the constructor axis.
    if ctx.host().ce_constructor_to_id.contains_key(&ctor_id) {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_not_supported_error,
            "Failed to execute 'define' on 'CustomElementRegistry': \
             this constructor has already been used with this registry.",
        ));
    }

    // 2c. HTMLConstructor brand check ([C1] §3.2.3 — invoked from
    // [C3] §4.13.4 `define` algorithm). The ctor's `[[Prototype]]`
    // chain must reach `globalThis.HTMLElement`; otherwise the
    // sync-construct + upgrade paths skip the prototype splice and
    // the resulting wrapper's chain is broken (Test #2
    // `instanceof_post_upgrade` would fail at upgrade-time despite
    // define succeeding). Runs AFTER spec steps 2-4 so name /
    // dup-name / dup-ctor errors surface with the spec-mandated
    // SyntaxError / NotSupportedError instead of being preempted by
    // a TypeError (D-17b R14 G14-1). Cycle-safe via the depth-bound
    // walk in `html_element::validate_html_element_constructor_chain`.
    super::html_element::validate_html_element_constructor_chain(ctx.vm, ctor_id)?;

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
    //    duplicate-name check).  Allocate a fresh per-VM constructor
    //    ID, store the JS callable, and on
    //    `define` Err remove the orphaned constructor from the map.
    //    `ce_next_constructor_id` is NOT decremented — IDs may be
    //    skipped on failed defines; the counter is a unique-tag source,
    //    not a dense-index, so gaps are harmless (every live def holds
    //    its own ID, no consumer expects monotonic-no-gaps).
    let host = ctx.host();
    let constructor_id_u64 = host.ce_next_constructor_id;
    // checked_add (not wrapping) — wrapping at 2^64 would re-use a
    // live constructor_id and alias the wrong constructor onto an
    // existing CustomElementDefinition. Practically unreachable, but
    // an explicit overflow error beats silent corruption.
    host.ce_next_constructor_id = host
        .ce_next_constructor_id
        .checked_add(1)
        .expect("CE constructor ID counter overflow (2^64 defines in one VM)");
    host.ce_constructors.insert(constructor_id_u64, ctor_id);
    // Reverse map for native_html_element_ctor's `new.target → constructor_id`
    // resolution. Populated + rolled back in lockstep with the forward map
    // so the bijection holds by construction (D-17b R2 G1 — replaces the
    // earlier JS-visible symbol brand to remove the spoofing surface).
    host.ce_constructor_to_id
        .insert(ctor_id, constructor_id_u64);
    let definition =
        CustomElementDefinition::new(name.clone(), constructor_id_u64, observed_attributes, None);
    let define_result = {
        let mut registry = host
            .ce_registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        registry.define(definition)
    };
    if let Err(err) = define_result {
        host.ce_constructors.remove(&constructor_id_u64);
        host.ce_constructor_to_id.remove(&ctor_id);
        return Err(define_error_to_vm_error(&ctx.vm.well_known, err));
    }

    // 6. Walk every entity in the world carrying
    //    `CustomElementState::undefined(name)` and enqueue Upgrade —
    //    the per-entity component is the single source of truth for
    //    "awaiting upgrade", so parser-baked elements (the parser
    //    cannot reach the per-VM registry) and `createElement`-baked
    //    elements (attached or detached) are all discovered by the
    //    same query.
    enqueue_upgrade_walk(ctx, &name);

    // 7. Resolve any pending whenDefined() Promises for this name.
    resolve_when_defined(ctx, &name, ctor_id);

    // 8. Flush queued reactions (Upgrades for the just-defined name
    //    fire synchronously: §4.13.4 define() step 18 enqueues them
    //    via "upgrade particular elements within a document", and
    //    §4.13.6 "Custom element reactions" gates the entire define()
    //    body inside a CE reactions stack frame).
    ctx.vm.flush_ce_reactions();

    Ok(JsValue::Undefined)
}

/// ToString coercion (WebIDL DOMString). Strings short-circuit; other
/// kinds go through the standard `ToString` algorithm.
fn coerce_to_string(ctx: &mut NativeContext<'_>, value: JsValue) -> Result<String, VmError> {
    let sid = super::super::super::coerce::to_string(ctx.vm, value)?;
    Ok(ctx.vm.strings.get_utf8(sid))
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

/// Enqueue Upgrade reactions for `name`'s upgrade candidates — the
/// single define()-time candidate-discovery mechanism, shared with
/// the boa engine via the engine-indep
/// `elidex_custom_elements::collect_upgrade_candidates` (HTML §4.13.4
/// "upgrade particular elements within a document": shadow-including
/// descendants of the document, in tree order, with the Undefined +
/// document-registry + local-name/is-value match). Parser- and
/// `createElement`-baked elements connected to the document are all
/// discovered here off the per-entity component; *detached* elements
/// awaiting upgrade are caught later by the insertion-time "try to
/// upgrade" path, not at define() time (per spec).
///
/// Holding the registry lock across the shared `dom` borrow follows
/// the `invoke_upgrade` → `prepare_upgrade` precedent (only a MUTABLE
/// DOM borrow must not overlap the registry guard).
fn enqueue_upgrade_walk(ctx: &mut NativeContext<'_>, name: &str) {
    let Some(host) = ctx.host_if_bound() else {
        return;
    };
    // No document bound → no document descendants to upgrade (the
    // §4.13.4 AO is per-document); detached elements upgrade on
    // insertion regardless.
    let Some(document) = host.document_entity_opt() else {
        return;
    };
    let to_upgrade = {
        let registry = host
            .ce_registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let dom = host.dom_shared();
        elidex_custom_elements::collect_upgrade_candidates(dom, document, &registry, name)
    };
    if to_upgrade.is_empty() {
        return;
    }
    let mut queue = host
        .ce_reaction_queue
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    for entity in to_upgrade {
        queue.push_back(CustomElementReaction::Upgrade(entity));
    }
}

/// Resolve every pending `whenDefined(name)` Promise with `ctor_id`.
fn resolve_when_defined(ctx: &mut NativeContext<'_>, name: &str, ctor_id: ObjectId) {
    let promise_to_resolve = ctx.host().ce_when_defined_promises.remove(name);
    if let Some(promise_id) = promise_to_resolve {
        let _ = super::super::super::natives_promise::settle_promise(
            ctx.vm,
            promise_id,
            false,
            JsValue::Object(ctor_id),
        );
    }
}
