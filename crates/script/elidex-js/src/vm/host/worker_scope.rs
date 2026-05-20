//! `WorkerGlobalScope` surface for dedicated-worker VMs (WHATWG HTML §10.2.1.1
//! / §10.2.1.2 + the §10.3 worker APIs).
//!
//! The worker realm's analog of the Window prototype block in `host/window.rs`:
//! it installs the worker-only globals (`self` / `name` / `postMessage` /
//! `close` / `importScripts` / `WorkerLocation` / `WorkerNavigator` /
//! `isSecureContext`) and the `WorkerGlobalScope.prototype` (chaining to
//! `EventTarget.prototype` so `addEventListener` is inherited). Worker-side
//! `postMessage` enqueues onto `VmInner::worker_outgoing` (drained by the
//! worker thread loop into `WorkerToParent::PostMessage`) rather than the
//! Window `pending_tasks` queue. Listener state lives in the engine-bound
//! `EventListeners` ECS component on the worker-scope entity (see
//! `event_handler_attrs::install_worker_handler_attrs`).

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::{GlobalScopeKind, NativeFn, VmInner};
use super::event_target_dispatch::dispatch_script_event;

/// `WorkerGlobalScope` prototype methods shared by every worker `globalThis`
/// (WHATWG HTML §10.2.1.2 + §10.3.1).
const WORKER_SCOPE_METHODS: &[(&str, NativeFn)] = &[
    ("postMessage", native_worker_post_message),
    ("close", native_worker_close),
    ("importScripts", native_import_scripts),
];

impl VmInner {
    /// Install `WorkerGlobalScope.prototype` (WHATWG HTML §10.2.1.1) chaining to
    /// `EventTarget.prototype`, then splice it as `globalThis`'s prototype. The
    /// worker analog of [`register_window_prototype`](Self::register_window_prototype).
    pub(in crate::vm) fn register_worker_global_scope_prototype(&mut self) {
        let event_target_proto = self.event_target_prototype.expect(
            "register_worker_global_scope_prototype called before register_event_target_prototype",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.install_methods(proto_id, WORKER_SCOPE_METHODS);
        // `onmessage` / `onerror` / `onmessageerror` + the WindowOrWorker
        // shared handler attrs — backed by the `EventListeners` component on
        // the worker-scope entity (WHATWG HTML §8.1.8.1).
        self.install_worker_handler_attrs(proto_id);
        self.worker_scope_prototype = Some(proto_id);
    }

    /// Install `self.navigator` as a `WorkerNavigator` (WHATWG HTML §10.3.2):
    /// the worker-appropriate subset of the Navigator surface.
    pub(in crate::vm) fn register_worker_navigator_global(&mut self) {
        let obj_id = self.create_object_with_methods(&[]);

        let hw = std::thread::available_parallelism()
            .map_or(1u32, |n| u32::try_from(n.get()).unwrap_or(u32::MAX));
        let string_fields: &[(&str, &str)] = &[
            ("userAgent", "Mozilla/5.0 (compatible; Elidex/0.1)"),
            ("appName", "Netscape"),
            ("appVersion", "5.0 (compatible; Elidex/0.1)"),
            ("product", "Gecko"),
            ("platform", std::env::consts::OS),
            ("language", "en-US"),
        ];
        for &(name, value) in string_fields {
            let key = PropertyKey::String(self.strings.intern(name));
            let sid = self.strings.intern(value);
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::String(sid)),
                PropertyAttrs::WEBIDL_RO,
            );
        }

        let key = PropertyKey::String(self.strings.intern("onLine"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Boolean(true)),
            PropertyAttrs::WEBIDL_RO,
        );

        let key = PropertyKey::String(self.strings.intern("hardwareConcurrency"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Number(f64::from(hw))),
            PropertyAttrs::WEBIDL_RO,
        );

        let en_us = self.strings.intern("en-US");
        let en = self.strings.intern("en");
        let lang_arr = self.create_array_object(vec![JsValue::String(en_us), JsValue::String(en)]);
        let key = PropertyKey::String(self.strings.intern("languages"));
        self.define_shaped_property(
            obj_id,
            key,
            PropertyValue::Data(JsValue::Object(lang_arr)),
            PropertyAttrs::WEBIDL_RO,
        );

        let nav = self.well_known.navigator;
        self.globals.insert(nav, JsValue::Object(obj_id));
    }

    /// Install `self.location` as a `WorkerLocation` (WHATWG HTML §10.3.3): a
    /// read-only view of the worker script URL.
    pub(in crate::vm) fn register_worker_location_global(&mut self, url: &url::Url) {
        let obj_id =
            self.create_object_with_methods(&[("toString", native_worker_location_to_string)]);

        let host = url.host_str().map_or_else(String::new, |h| {
            url.port()
                .map_or_else(|| h.to_string(), |port| format!("{h}:{port}"))
        });
        let fields: &[(&str, String)] = &[
            ("href", url.to_string()),
            ("origin", url.origin().ascii_serialization()),
            ("protocol", format!("{}:", url.scheme())),
            ("host", host),
            ("hostname", url.host_str().unwrap_or("").to_string()),
            (
                "port",
                url.port().map_or_else(String::new, |p| p.to_string()),
            ),
            ("pathname", url.path().to_string()),
            (
                "search",
                url.query().map_or_else(String::new, |q| format!("?{q}")),
            ),
            (
                "hash",
                url.fragment().map_or_else(String::new, |f| format!("#{f}")),
            ),
        ];
        for (name, value) in fields {
            let key = PropertyKey::String(self.strings.intern(name));
            let sid = self.strings.intern(value);
            self.define_shaped_property(
                obj_id,
                key,
                PropertyValue::Data(JsValue::String(sid)),
                PropertyAttrs::WEBIDL_RO,
            );
        }

        let loc = self.well_known.location;
        self.globals.insert(loc, JsValue::Object(obj_id));
    }

    /// Install the remaining worker globals: `self` (the §10.2.1.1
    /// self-reference making `self === globalThis`), the read-only worker
    /// `name`, and `isSecureContext` (WindowOrWorkerGlobalScope mixin; W3C
    /// Secure Contexts). `is_secure_context` is inherited from the **creator's**
    /// environment — NOT derived from the worker script URL, so a `data:` /
    /// `blob:` worker spawned by a secure parent is itself secure.
    pub(in crate::vm) fn register_worker_globals(&mut self, name: &str, is_secure_context: bool) {
        let self_sid = self.strings.intern("self");
        self.globals
            .insert(self_sid, JsValue::Object(self.global_object));

        let name_key = self.strings.intern("name");
        let name_val = self.strings.intern(name);
        self.globals.insert(name_key, JsValue::String(name_val));

        let secure_key = self.strings.intern("isSecureContext");
        self.globals
            .insert(secure_key, JsValue::Boolean(is_secure_context));
    }
}

/// Whether a URL denotes a secure context (WHATWG HTML §8.1.3.5 / W3C Secure
/// Contexts — the "potentially trustworthy URL" essence): HTTPS / WSS, `file:`,
/// or a localhost host. The main-side `Worker` constructor applies this to the
/// **creator's** URL to derive the worker's inherited secure-context flag.
pub(in crate::vm) fn url_is_secure_context(url: &url::Url) -> bool {
    url.scheme() == "https"
        || url.scheme() == "wss"
        || url.scheme() == "file"
        || matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"))
}

impl VmInner {
    /// Deliver an inbound `postMessage` from the parent to the worker scope
    /// (WHATWG HTML §10.2.1.2 step "fire an event named `message`"). Resolves
    /// the worker-scope entity (the global object's backing entity) and
    /// delegates to the shared [`dispatch_message_event_at`](Self::dispatch_message_event_at)
    /// helper, with the worker's `globalThis` as the event target wrapper.
    pub(in crate::vm) fn dispatch_worker_message(&mut self, data_json: &str, origin: &str) {
        let global_id = self.global_object;
        let ObjectKind::HostObject {
            entity_bits: target_bits,
        } = self.get_object(global_id).kind
        else {
            return;
        };
        let Some(target_entity) = elidex_ecs::Entity::from_bits(target_bits) else {
            return;
        };
        let message_type_sid = self.well_known.message;
        self.dispatch_message_event_at(
            message_type_sid,
            target_entity,
            global_id,
            data_json,
            origin,
        );
    }

    /// Fire a trusted `MessageEvent` of type `type_sid` carrying the JSON
    /// `data_json` payload at `target_entity`, with `target` / `srcElement` set
    /// to `target_wrapper` (WHATWG HTML §9.1 MessageEvent + the dedicated-worker
    /// §10.2.1.2 / AbstractWorker §10.2.6.1 delivery steps). `type_sid` is
    /// `message` for normal delivery or `messageerror` for a payload that failed
    /// to deserialize (§10.2.1.2 / §10.2.6.1 — both are MessageEvents). Shared
    /// by the worker-side inbound delivery
    /// ([`dispatch_worker_message`](Self::dispatch_worker_message), target = the
    /// worker's `globalThis`) and the main-side per-tick drain
    /// ([`drain_worker_messages`](Self::drain_worker_messages), target = the
    /// `Worker` object). `source` is always `null` — neither realm exposes the
    /// peer as a worker-visible object. Dispatched through the shared
    /// `dispatch_script_event` walker so the matching `on*` handler and every
    /// `addEventListener` listener fire with correct `{once}` / `{signal}`
    /// handling.
    pub(in crate::vm) fn dispatch_message_event_at(
        &mut self,
        type_sid: super::super::value::StringId,
        target_entity: elidex_ecs::Entity,
        target_wrapper: super::super::value::ObjectId,
        data_json: &str,
        origin: &str,
    ) {
        // JSON.parse the payload via the non-interning `&str` entry — these
        // transient cross-thread blobs must not accrete in the permanent
        // `StringPool`. A parse failure (e.g. the `undefined`→`null` shortcut)
        // degrades to `undefined`, matching the structured-clone-via-JSON
        // approximation.
        let data = super::super::natives_json::parse_json_str(self, data_json)
            .unwrap_or(JsValue::Undefined);

        let message_type_sid = type_sid;
        let origin_sid = self.strings.intern(origin);
        let last_event_id_sid = self.well_known.empty;

        // Root `data` before allocating the event (GC is enabled in the worker
        // loop, unlike inside native dispatch) — mirrors `dispatch_post_message`.
        let mut g_data = self.push_temp_root(data);
        let event_proto = g_data.message_event_prototype.or(g_data.event_prototype);
        let event_id = g_data.alloc_object(Object {
            kind: ObjectKind::Event {
                default_prevented: false,
                propagation_stopped: false,
                immediate_propagation_stopped: false,
                cancelable: false,
                passive: false,
                type_sid: message_type_sid,
                bubbles: false,
                composed: false,
                composed_path: None,
            },
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: event_proto,
            extensible: true,
        });

        let mut g = g_data.push_temp_root(JsValue::Object(event_id));
        let message_shape = g
            .precomputed_event_shapes
            .as_ref()
            .expect("precomputed_event_shapes built during VM init")
            .message;
        let timestamp_ms = g.start_instant.elapsed().as_secs_f64() * 1000.0;
        let ports_arr = g.create_array_object(Vec::new());
        let slots: Vec<PropertyValue> = vec![
            PropertyValue::Data(JsValue::String(message_type_sid)),
            PropertyValue::Data(JsValue::Boolean(false)),
            PropertyValue::Data(JsValue::Boolean(false)),
            PropertyValue::Data(JsValue::Number(0.0)),
            PropertyValue::Data(JsValue::Object(target_wrapper)),
            PropertyValue::Data(JsValue::Object(target_wrapper)),
            PropertyValue::Data(JsValue::Number(timestamp_ms)),
            PropertyValue::Data(JsValue::Boolean(false)),
            PropertyValue::Data(JsValue::Boolean(true)),
            PropertyValue::Data(data),
            PropertyValue::Data(JsValue::String(origin_sid)),
            PropertyValue::Data(JsValue::String(last_event_id_sid)),
            PropertyValue::Data(JsValue::Null),
            PropertyValue::Data(JsValue::Object(ports_arr)),
        ];
        g.define_with_precomputed_shape(event_id, message_shape, slots);

        g.dispatched_events.insert(event_id);
        let mut ctx = NativeContext { vm: &mut g };
        let _ = dispatch_script_event(&mut ctx, event_id, target_entity);
        g.dispatched_events.remove(&event_id);
    }
}

/// `self.postMessage(data)` (WHATWG HTML §10.2.1.2): JSON-serialize `data` and
/// enqueue it for the worker thread loop to forward to the parent. Throws
/// `DataCloneError` when serialization fails (e.g. a circular reference) — the
/// JSON shortcut deviation from full StructuredSerialize is tracked by the
/// defer slot `#11-worker-structured-serialize`.
fn native_worker_post_message(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let data = args.first().copied().unwrap_or(JsValue::Undefined);
    let Ok(json) =
        super::super::natives_json::native_json_stringify(ctx, JsValue::Undefined, &[data])
    else {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_data_clone_error,
            "Failed to serialize message",
        ));
    };
    // `JSON.stringify` of a function / symbol yields `undefined`; encode it as
    // JSON `null` so it round-trips through the parent's `JSON.parse` (matching
    // the structured-clone-via-JSON approximation).
    let serialized = match json {
        JsValue::String(sid) => ctx.vm.strings.get_utf8(sid),
        _ => "null".to_string(),
    };
    ctx.vm.worker_outgoing.push(serialized);
    Ok(JsValue::Undefined)
}

/// `self.close()` (WHATWG HTML §10.2.1.2): request worker shutdown. The worker
/// thread loop observes the flag after the current tick and exits.
fn native_worker_close(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    ctx.vm.worker_close_requested = true;
    Ok(JsValue::Undefined)
}

/// `self.importScripts(...urls)` (WHATWG HTML §10.3.1): synchronously fetch,
/// validate, and evaluate each classic script in order, resolved against the
/// worker script URL.
fn native_import_scripts(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let base = match &ctx.vm.global_scope_kind {
        GlobalScopeKind::DedicatedWorker { script_url, .. } => script_url.clone(),
        GlobalScopeKind::Window => {
            return Err(VmError::type_error(
                "importScripts is only available in workers",
            ))
        }
    };
    let Some(handle) = ctx.vm.network_handle.clone() else {
        return Err(VmError::type_error(
            "NetworkError: no network handle installed on this worker",
        ));
    };

    for arg in args {
        let url_sid = super::super::coerce::to_string(ctx.vm, *arg)?;
        let url_str = ctx.vm.strings.get_utf8(url_sid);
        let resolved = base
            .join(&url_str)
            .map_err(|e| VmError::type_error(format!("importScripts: invalid URL: {e}")))?;

        let request = elidex_net::Request {
            method: "GET".to_string(),
            url: resolved.clone(),
            ..Default::default()
        };
        let response = handle.fetch_blocking(request).map_err(|e| {
            VmError::type_error(format!("NetworkError: failed to fetch {resolved}: {e}"))
        })?;

        let content_type = response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.as_str());
        let source = elidex_api_workers::validate_worker_script_response(
            content_type,
            response.status,
            &response.body,
            &resolved,
        )
        .map_err(|e| VmError::type_error(format!("NetworkError: {e}")))?;

        let script = crate::compiler::compile_script(&source).map_err(|e| {
            VmError::type_error(format!(
                "importScripts: failed to compile {resolved}: {e:?}"
            ))
        })?;
        ctx.vm.run_script(script)?;
    }

    Ok(JsValue::Undefined)
}

/// `WorkerLocation.prototype.toString()` / stringifier (WHATWG HTML §10.3.3):
/// returns `href`, read from the receiver's own data property.
fn native_worker_location_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        let key = PropertyKey::String(ctx.vm.strings.intern("href"));
        return ctx.vm.get_property_value(id, key);
    }
    Ok(JsValue::Undefined)
}
