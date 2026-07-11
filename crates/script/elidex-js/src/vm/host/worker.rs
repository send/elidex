//! Main-side `Worker` interface (WHATWG HTML §10.2.6 — the parent's handle to
//! a dedicated worker).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file holds only the engine-bound
//! responsibilities: the `Worker` constructor + prototype install, the
//! brand check, `postMessage` / `terminate` marshalling, the `onmessage` /
//! `onerror` IDL-attribute install, and the per-tick message drain that
//! converts inbound [`WorkerToParent`] frames into DOM events. The pure
//! algorithm — worker-script URL resolution + option validation (WHATWG HTML
//! §10.2.6.3) and the cross-thread registry / spawn scaffolding — lives in the
//! engine-independent [`elidex_api_workers`] crate; the post-fetch runtime
//! harness ([`run_worker`](crate::vm::worker_thread::run_worker)) lives in
//! `vm/worker_thread.rs`.
//!
//! ## ECS-native storage
//!
//! A `Worker` object is a `HostObject` backed by a fresh
//! [`NodeKind::Worker`] entity in the main `EcsDom`, carrying a [`WorkerRef`]
//! component whose [`WorkerId`] is the brand-check key into the VM's
//! [`WorkerRegistry`] (the cross-thread transport handles — legitimately
//! non-ECS, see [`elidex_api_workers::WorkerRegistry`]). Listener state
//! (`onmessage` / `onerror` / `addEventListener`) lives in the engine-
//! independent `EventListeners` ECS component on that entity, dispatched
//! through the shared event-target walk — there is no VM-side reverse map
//! (mirrors the `WorkerGlobalScope` worker-side surface in
//! [`super::worker_scope`]).

#![cfg(feature = "engine")]

use elidex_api_workers::{
    resolve_worker_script_url, spawn_worker, validate_credentials, validate_worker_type, WorkerId,
    WorkerScriptError, WorkerToParent,
};
use elidex_ecs::{Entity, NodeKind};

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

/// ECS marker linking a main-side `Worker` `HostObject` entity to its
/// transport handle in the VM's [`WorkerRegistry`]. Presence of this component
/// on a [`NodeKind::Worker`] entity is the `Worker` brand check; the contained
/// [`WorkerId`] routes `postMessage` / `terminate` / the inbound drain to the
/// matching channel handle.
pub(in crate::vm) struct WorkerRef(pub(in crate::vm) WorkerId);

/// `Worker.prototype` methods (WHATWG HTML §10.2.6.3 — dedicated workers and
/// the `Worker` interface).
const WORKER_METHODS: &[(&str, NativeFn)] = &[
    ("postMessage", native_worker_post_message),
    ("terminate", native_worker_terminate),
];

impl VmInner {
    /// Install `Worker.prototype` (chaining `EventTarget.prototype` so
    /// `addEventListener` is inherited) + the `Worker` constructor on
    /// `globalThis` (WHATWG HTML §10.2.6). Window-scope only — a dedicated
    /// worker does not currently spawn nested workers (deferred with
    /// `#11-shared-worker-vm`'s sibling concerns).
    pub(in crate::vm) fn register_worker_global(&mut self) {
        let event_target_proto = self
            .event_target_prototype
            .expect("register_worker_global called before register_event_target_prototype");
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.install_methods(proto_id, WORKER_METHODS);
        // `onmessage` / `onmessageerror` / `onerror` over the shared
        // event-handler backend, backed by the `EventListeners` component on
        // the `Worker` entity (WHATWG HTML §10.2.6.1 / §8.1.8.1).
        self.install_worker_object_handler_attrs(proto_id);
        self.worker_prototype = Some(proto_id);

        let global_sid = self.strings.intern("Worker");
        super::events::install_ctor(
            self,
            proto_id,
            "Worker",
            native_worker_constructor,
            global_sid,
            super::super::value::CallShape::ConstructorOnly,
        );
    }

    /// Per-tick drain of every **live** worker's outbound channel (the parent's
    /// event-loop step of WHATWG HTML §10.2.4 "run a worker"). Iterates the
    /// [`worker_entities`](Self::worker_entities) live set (NOT a full
    /// `WorkerRef` world scan — terminated workers' entities are retained for
    /// the brand check but dropped from that set, so the per-frame cost stays
    /// O(live workers)): pulls all pending [`WorkerToParent`] frames and
    /// converts them to DOM events on the `Worker` object — `PostMessage` →
    /// `message`, `Error` → `error`, `MessageError` → `messageerror` — dropping
    /// the worker once it reports `Closed` / disconnects. The future shell main
    /// loop drives this each frame, like [`Vm::tick_network`](crate::vm::Vm::tick_network).
    pub(in crate::vm) fn drain_worker_messages(&mut self) {
        // Cheap per-frame guard: skip on the common no-worker page (mirrors
        // `tick_network`'s `network_handle`-absent skip).
        if self.worker_entities.is_empty() {
            return;
        }
        let workers: Vec<(WorkerId, Entity)> = self
            .worker_entities
            .iter()
            .map(|(&id, &entity)| (id, entity))
            .collect();

        for (worker_id, entity) in workers {
            let mut messages = Vec::new();
            let mut closed = false;
            if let Some(handle) = self.worker_registry.get(worker_id) {
                loop {
                    match handle.try_recv() {
                        Ok(msg) => messages.push(msg),
                        Err(crossbeam_channel::TryRecvError::Empty) => break,
                        Err(crossbeam_channel::TryRecvError::Disconnected) => {
                            closed = true;
                            break;
                        }
                    }
                }
            }

            // The `Worker` wrapper is cached + GC-rooted at construction, so the
            // lookup never misses for a live registered worker.
            let Some(target_wrapper) = self
                .host_data
                .as_deref()
                .and_then(|hd| hd.get_cached_wrapper(entity))
            else {
                continue;
            };

            for msg in messages {
                match msg {
                    // origin = "" per the message-port post-message steps —
                    // see `elidex_api_workers::ParentToWorker`.
                    WorkerToParent::PostMessage { data } => {
                        let type_sid = self.well_known.message;
                        self.dispatch_message_event_at(type_sid, entity, target_wrapper, &data, "");
                    }
                    WorkerToParent::Error {
                        message,
                        filename,
                        lineno,
                        colno,
                        error_value,
                    } => {
                        self.dispatch_worker_error_event_at(
                            entity,
                            &message,
                            &filename,
                            f64::from(lineno),
                            f64::from(colno),
                            &error_value,
                        );
                    }
                    // A `messageerror` is itself a `MessageEvent` with a `null`
                    // data payload — the *message port post message steps* fire
                    // it using `MessageEvent` (WHATWG HTML §9.4.4 step 7.4) —
                    // so route it through the same MessageEvent path, not a
                    // bare `Event`. Like `message` (step 7.7), its `origin`
                    // stays `""`.
                    WorkerToParent::MessageError => {
                        let type_sid = self.strings.intern("messageerror");
                        self.dispatch_message_event_at(
                            type_sid,
                            entity,
                            target_wrapper,
                            "null",
                            "",
                        );
                    }
                    WorkerToParent::Closed => closed = true,
                }
            }

            if closed {
                self.worker_registry.remove(worker_id);
                self.worker_entities.remove(&worker_id);
                // No further events will dispatch at this entity — drop the
                // cached `Worker` wrapper so it is no longer GC-rooted (it
                // survives only as long as live JS still references it). The
                // brand check reads `WorkerRef` off the still-live entity, so
                // `postMessage` after close stays a silent no-op (not a throw).
                if let Some(hd) = self.host_data.as_deref_mut() {
                    hd.remove_wrapper(entity);
                }
            }
        }
    }

    /// Terminate every registered dedicated worker and uncache its `Worker`
    /// wrapper (the document-teardown half of WHATWG HTML §10.2.4 "terminate a
    /// worker", driven from [`Vm::teardown_document`](crate::vm::Vm::teardown_document)). Uncaching
    /// runs **while still bound** — mirroring the in-session `terminate()` /
    /// close-drain cleanup — because the post-unbind drain early-returns on the
    /// now-empty registry and would otherwise leave the wrappers GC-rooted.
    pub(in crate::vm) fn teardown_workers(&mut self) {
        // Only live workers (still in the map) hold a cached wrapper —
        // terminated ones were uncached at `terminate()` / close-drain time.
        let live: Vec<Entity> = self.worker_entities.values().copied().collect();
        if let Some(hd) = self.host_data.as_deref_mut() {
            for entity in live {
                hd.remove_wrapper(entity);
            }
        }
        self.worker_entities.clear();
        self.worker_registry.terminate_all();
    }

    /// Fire a trusted `error` event (WHATWG HTML §10.2.5 — runtime script
    /// errors propagate to the worker's parent via AbstractWorker §10.2.6.1
    /// `onerror`). Builds an `ErrorEvent` carrying `message` / `filename` /
    /// `lineno` / `colno` and dispatches it at the `Worker` entity through the
    /// shared event-target walk.
    fn dispatch_worker_error_event_at(
        &mut self,
        target_entity: Entity,
        message: &str,
        filename: &str,
        lineno: f64,
        colno: f64,
        error_value: &str,
    ) {
        let type_sid = self.well_known.error;
        let message_sid = self.strings.intern(message);
        let filename_sid = self.strings.intern(filename);
        // `ErrorEvent.error` carries the thrown value; the real JS value cannot
        // cross the worker thread boundary, so the worker sends a string
        // representation (consistent with the JSON-messaging approximation) —
        // expose it as a JS string rather than the spec's `any` value.
        let error_sid = self.strings.intern(error_value);
        let shape_id = self
            .precomputed_event_shapes
            .as_ref()
            .expect("precomputed_event_shapes built during VM init")
            .error_event;
        let slots = vec![
            PropertyValue::Data(JsValue::String(message_sid)),
            PropertyValue::Data(JsValue::String(filename_sid)),
            PropertyValue::Data(JsValue::Number(lineno)),
            PropertyValue::Data(JsValue::Number(colno)),
            PropertyValue::Data(JsValue::String(error_sid)),
        ];
        let event_id = self.create_fresh_event_object(
            JsValue::Undefined,
            type_sid,
            super::events::EventInit::default(),
            shape_id,
            slots,
            true,
            // Worker error event is dispatched from VM-internal
            // task plumbing, not a user `new ErrorEvent(...)` call —
            // `mode = Call`. (Reparented to `ErrorEvent.prototype`
            // below regardless.)
            super::super::value::CallMode::Call,
        );
        // Reparent onto `ErrorEvent.prototype` so `e instanceof ErrorEvent`
        // holds (the call-mode `create_fresh_event_object` seeds
        // `Event.prototype`); the `message` / `filename` payload reads off the
        // shaped data slots regardless.
        if let Some(proto) = self.error_event_prototype {
            self.get_object_mut(event_id).prototype = Some(proto);
        }

        let mut g = self.push_temp_root(JsValue::Object(event_id));
        g.dispatched_events.insert(event_id);
        let mut ctx = NativeContext::new_call(&mut g);
        let _ =
            super::event_target_dispatch::dispatch_script_event(&mut ctx, event_id, target_entity);
        g.dispatched_events.remove(&event_id);
    }
}

/// Resolve `this` to the backing `Worker` `(entity, worker-id)` if it brands as
/// a `Worker` (a `HostObject` over a [`NodeKind::Worker`] entity carrying a
/// [`WorkerRef`]); a `TypeError` ("Illegal invocation") otherwise. Mirrors the
/// sibling host brand checks.
fn require_worker(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<(Entity, WorkerId), VmError> {
    let illegal = || {
        VmError::type_error(format!(
            "Failed to execute '{method}' on 'Worker': Illegal invocation"
        ))
    };
    let JsValue::Object(id) = this else {
        return Err(illegal());
    };
    let ObjectKind::HostObject { entity_bits } = ctx.vm.get_object(id).kind else {
        return Err(illegal());
    };
    let entity = Entity::from_bits(entity_bits).ok_or_else(illegal)?;
    let worker_id = ctx
        .host()
        .dom()
        .world()
        .get::<&WorkerRef>(entity)
        .ok()
        .map(|w| w.0)
        .ok_or_else(illegal)?;
    Ok((entity, worker_id))
}

/// `new Worker(scriptURL, options)` (WHATWG HTML §10.2.6.3). Resolves the
/// script URL same-origin against the document base URL, validates the `type` /
/// `credentials` options, mints a `Send` sibling `NetworkHandle` on this (main)
/// thread, spawns the worker thread, registers its transport handle, and
/// returns a `Worker` `HostObject` backed by a fresh `NodeKind::Worker` entity.
#[allow(clippy::needless_pass_by_value)]
fn native_worker_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let url_arg = args.first().copied().ok_or_else(|| {
        VmError::type_error(
            "Failed to construct 'Worker': 1 argument required, but only 0 present.",
        )
    })?;
    let url_sid = super::super::coerce::to_string(ctx.vm, url_arg)?;
    let url_str = ctx.vm.strings.get_utf8(url_sid);

    let options = parse_worker_options(ctx, args.get(1).copied())?;

    let base = ctx.vm.navigation.current_url.clone();
    let resolved = resolve_worker_script_url(&base, &url_str)
        .map_err(|e| marshal_worker_script_error(ctx.vm, &e))?;
    validate_worker_type(options.type_opt.as_deref())
        .map_err(|e| marshal_worker_script_error(ctx.vm, &e))?;
    let credentials = validate_credentials(options.credentials.as_deref())
        .map(|c| credentials_mode(&c))
        .map_err(|e| marshal_worker_script_error(ctx.vm, &e))?;

    // Mint a `Send` sibling network handle on the **main** thread (the
    // documented Web-Worker mint path, `NetworkHandle::create_sibling_handle`)
    // and move it into the worker thread; `NetworkHandle` is `Send` so the
    // by-value move across the spawn boundary is sound.
    let sibling = ctx
        .vm
        .network_handle
        .as_ref()
        .map(|h| h.create_sibling_handle());

    // Secure-context is inherited from the creator (this document), not derived
    // from the worker script URL (WHATWG HTML §8.1.3.5) — so a `data:` / `blob:`
    // worker spawned by a secure page is itself secure.
    let is_secure_context =
        super::worker_scope::url_is_secure_context(&ctx.vm.navigation.current_url);

    let name = options.name;
    let worker_name = name.clone();
    // The worker realm inherits this (the creator's) engine-wide mode — a
    // `BrowserCore`/`App` document's workers must not silently reset to the
    // default (they install the same policy-gated surface).
    let engine_mode = ctx.vm.engine_mode;
    let handle = spawn_worker(name, move |channel| {
        crate::vm::worker_thread::run_worker(
            &resolved,
            worker_name,
            is_secure_context,
            credentials,
            sibling,
            engine_mode,
            &channel,
        );
    });
    let worker_id = ctx.vm.worker_registry.register(handle);

    // Allocate the backing entity (ECS-native brand key + listener home) and
    // record the live `WorkerId` → entity mapping the drain iterates.
    let entity = ctx
        .host()
        .dom()
        .world_mut()
        .spawn((NodeKind::Worker, WorkerRef(worker_id)));
    ctx.vm.worker_entities.insert(worker_id, entity);

    // Promote the ctor's `this` (already prototyped to `Worker.prototype` by
    // `do_new`) to a `HostObject` over the entity, and cache it so the inbound
    // drain's `MessageEvent` target resolves to this same object.
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`")
    };
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::HostObject {
        entity_bits: entity.to_bits().get(),
    };
    ctx.host().cache_wrapper(entity, inst_id);
    Ok(JsValue::Object(inst_id))
}

/// Map a validated `WorkerOptions.credentials` value (WHATWG HTML §10.2.6.3 /
/// Fetch RequestCredentials) to the network [`CredentialsMode`]. The input is
/// already validated by [`validate_credentials`] to one of the three values.
fn credentials_mode(value: &str) -> elidex_net::CredentialsMode {
    match value {
        "omit" => elidex_net::CredentialsMode::Omit,
        "include" => elidex_net::CredentialsMode::Include,
        _ => elidex_net::CredentialsMode::SameOrigin,
    }
}

/// Parsed `WorkerOptions` (WHATWG HTML §10.2.6.3): `name` (default empty),
/// `type` (`"classic"` / `"module"`), `credentials` (RequestCredentials).
struct WorkerOptions {
    name: String,
    type_opt: Option<String>,
    credentials: Option<String>,
}

/// Read the optional `WorkerOptions` dictionary argument (WHATWG HTML
/// §10.2.6.3 + WebIDL §3.10 dictionary coercion): `undefined` / `null` →
/// defaults; an Object reads `name` / `type` / `credentials`; any other
/// primitive is a `TypeError`.
fn parse_worker_options(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<WorkerOptions, VmError> {
    let defaults = || WorkerOptions {
        name: String::new(),
        type_opt: None,
        credentials: None,
    };
    let Some(val) = arg else {
        return Ok(defaults());
    };
    match val {
        JsValue::Undefined | JsValue::Null => Ok(defaults()),
        JsValue::Object(opts_id) => {
            let name = read_opt_string(ctx, opts_id, "name")?.unwrap_or_default();
            let type_opt = read_opt_string(ctx, opts_id, "type")?;
            let credentials = read_opt_string(ctx, opts_id, "credentials")?;
            Ok(WorkerOptions {
                name,
                type_opt,
                credentials,
            })
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Worker': options is not an object.",
        )),
    }
}

/// Read `obj[key]` and `ToString`-coerce it, or `None` when the property is
/// absent / `undefined` (WebIDL dictionary member with no default).
fn read_opt_string(
    ctx: &mut NativeContext<'_>,
    obj_id: ObjectId,
    key: &str,
) -> Result<Option<String>, VmError> {
    let key_sid = ctx.vm.strings.intern(key);
    let raw = ctx
        .vm
        .get_property_value(obj_id, PropertyKey::String(key_sid))?;
    if matches!(raw, JsValue::Undefined) {
        return Ok(None);
    }
    let sid = super::super::coerce::to_string(ctx.vm, raw)?;
    Ok(Some(ctx.vm.strings.get_utf8(sid)))
}

/// Marshal a [`WorkerScriptError`] to the spec-mandated DOM exception
/// (WHATWG HTML §10.2.6.3): origin failures → `SecurityError`, everything else
/// → `SyntaxError` (invalid URL) / `TypeError`.
fn marshal_worker_script_error(vm: &VmInner, err: &WorkerScriptError) -> VmError {
    match err {
        WorkerScriptError::NotSameOrigin { .. } => {
            VmError::dom_exception(vm.well_known.dom_exc_security_error, err.to_string())
        }
        WorkerScriptError::InvalidUrl(_) => VmError::syntax_error(err.to_string()),
        _ => VmError::type_error(err.to_string()),
    }
}

/// `Worker.prototype.postMessage(message)` (WHATWG HTML §10.2.6.3): JSON-
/// serialize `message` and route it to the worker thread. Throws
/// `DataCloneError` when serialization fails (e.g. a circular reference) — the
/// JSON-shortcut deviation from full StructuredSerialize is tracked by the
/// defer slot `#11-worker-structured-serialize`.
fn native_worker_post_message(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (_entity, worker_id) = require_worker(ctx, this, "postMessage")?;
    let data = args.first().copied().unwrap_or(JsValue::Undefined);
    let serialized = super::worker_scope::serialize_message(ctx, data)?;
    // No origin travels with the message (S5-4e, closes slot
    // `#11-worker-port-message-no-origin`) — origin = "" per the message-port
    // post-message steps, see `elidex_api_workers::ParentToWorker`. Unlike
    // window.postMessage (§9.3.3), where the origin IS initialized.
    if let Some(handle) = ctx.vm.worker_registry.get(worker_id) {
        handle.post_message(serialized);
    }
    Ok(JsValue::Undefined)
}

/// `Worker.prototype.terminate()` (WHATWG HTML §10.2.6.3 / §10.2.4 "terminate a
/// worker"): signal the worker thread to exit and drop its transport handle.
/// Idempotent — a second call (or a call after the worker already exited) is a
/// no-op.
fn native_worker_terminate(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let (entity, worker_id) = require_worker(ctx, this, "terminate")?;
    ctx.vm.worker_registry.terminate(worker_id);
    // Drop from the live set so the drain stops visiting it.
    ctx.vm.worker_entities.remove(&worker_id);
    // `terminate` drops the registry handle immediately, so the drain never
    // sees `Closed` for this worker — un-root its cached wrapper here instead
    // (mirrors the close-drain path) so it does not leak for the VM's lifetime.
    // The entity + `WorkerRef` stay, so `postMessage` after `terminate` remains
    // a silent no-op rather than an "Illegal invocation" throw.
    if let Some(hd) = ctx.vm.host_data.as_deref_mut() {
        hd.remove_wrapper(entity);
    }
    Ok(JsValue::Undefined)
}
