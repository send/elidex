//! `Clients` + `Client` interfaces (WHATWG SW §4.2 / §4.3; D-19 PR-2).
//!
//! `Clients` is a stateless, brand-via-prototype façade (no `ObjectKind`)
//! reading the VM-level `sw_clients` snapshot.  `Client` is an own brand
//! (`ObjectKind::Client`) whose snapshot lives in `client_states` keyed by
//! the object id — the brand for `Client.postMessage` + the data behind the
//! `id` / `url` / `type` / `frameType` own attributes.

#![cfg(feature = "engine")]

use elidex_api_sw::{ClientSnapshot, ClientType, FrameType, SwToContent};

use super::super::super::shape::{self, PropertyAttrs};
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::super::{NativeFn, VmInner};
use super::install_interface;

// ---------------------------------------------------------------------------
// Interface registration
// ---------------------------------------------------------------------------

/// Allocate `Clients.prototype` (chains to `Object.prototype`), install
/// `get` / `matchAll` / `claim`, and expose the `Clients` interface.
pub(crate) fn register_clients_interface(vm: &mut VmInner) {
    let proto = alloc_object_proto(vm);
    let methods: &[(&str, NativeFn)] = &[
        ("get", native_clients_get),
        ("matchAll", native_clients_match_all),
        ("claim", native_clients_claim),
    ];
    vm.install_methods(proto, methods);
    vm.clients_prototype = Some(proto);
    install_interface(vm, proto, "Clients");
}

/// Allocate `Client.prototype` (chains to `Object.prototype`), install
/// `postMessage`, and expose the `Client` interface.  `id` / `url` / `type`
/// / `frameType` are own-data attributes on each instance (set at vend
/// time), not prototype accessors.
pub(crate) fn register_client_interface(vm: &mut VmInner) {
    let proto = alloc_object_proto(vm);
    let methods: &[(&str, NativeFn)] = &[("postMessage", native_client_post_message)];
    vm.install_methods(proto, methods);
    vm.client_prototype = Some(proto);
    install_interface(vm, proto, "Client");
}

/// Install the `clients` global singleton (SW §4.1.1) — the only way to
/// reach a `Clients`.
pub(crate) fn install_clients_singleton(vm: &mut VmInner) {
    let proto = vm.clients_prototype;
    let singleton = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let sid = vm.strings.intern("clients");
    vm.globals.insert(sid, JsValue::Object(singleton));
}

fn alloc_object_proto(vm: &mut VmInner) -> ObjectId {
    let parent = vm.object_prototype;
    vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: parent,
        extensible: true,
    })
}

// ---------------------------------------------------------------------------
// Client object construction
// ---------------------------------------------------------------------------

fn client_type_str(t: ClientType) -> &'static str {
    match t {
        ClientType::Window => "window",
        ClientType::Worker => "worker",
        ClientType::SharedWorker => "sharedworker",
    }
}

fn frame_type_str(f: FrameType) -> &'static str {
    match f {
        FrameType::TopLevel => "top-level",
        FrameType::Nested => "nested",
        FrameType::Auxiliary => "auxiliary",
        FrameType::None => "none",
    }
}

/// Allocate a `Client` (`ObjectKind::Client`) from a snapshot + register its
/// `client_states` entry (the brand + postMessage routing).  `id` / `url` /
/// `type` / `frameType` are readonly own-data props.
fn build_client_object(vm: &mut VmInner, snap: &ClientSnapshot) -> ObjectId {
    let proto = vm.client_prototype;
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Client,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // Root the freshly-allocated `Client` across its attribute installs:
    // `define_shaped_property` transitions the shape (and can GC) before the
    // object is reachable from any root, so an un-rooted `id` would be swept
    // mid-build and its slot recycled (the `build_response_from_entry`
    // precedent).
    let mut g = vm.push_temp_root(JsValue::Object(id));
    let id_sid = g.strings.intern(&snap.id);
    let url_sid = g.strings.intern(&snap.url);
    let type_sid = g.strings.intern(client_type_str(snap.client_type));
    let frame_sid = g.strings.intern(frame_type_str(snap.frame_type));
    define_ro(&mut g, id, "id", JsValue::String(id_sid));
    define_ro(&mut g, id, "url", JsValue::String(url_sid));
    define_ro(&mut g, id, "type", JsValue::String(type_sid));
    define_ro(&mut g, id, "frameType", JsValue::String(frame_sid));
    g.client_states.insert(id, snap.clone());
    drop(g);
    id
}

fn define_ro(vm: &mut VmInner, id: ObjectId, name: &str, value: JsValue) {
    let key = PropertyKey::String(vm.strings.intern(name));
    vm.define_shaped_property(
        id,
        key,
        PropertyValue::Data(value),
        PropertyAttrs::WEBIDL_RO,
    );
}

// ---------------------------------------------------------------------------
// Brand checks + Promise helper
// ---------------------------------------------------------------------------

/// Brand-check that `this` is the `clients` façade (prototype identity —
/// `Clients` has no `ObjectKind`, per §4.2/lesson #276).
fn require_clients(ctx: &NativeContext<'_>, this: JsValue) -> Result<(), VmError> {
    if let JsValue::Object(id) = this {
        if ctx.vm.get_object(id).prototype == ctx.vm.clients_prototype {
            return Ok(());
        }
    }
    Err(VmError::type_error(
        "Illegal invocation: receiver is not a Clients",
    ))
}

/// A fresh promise resolved with `value` (the `JsValue`-returning sugar over
/// [`super::promise_resolve`], which roots `value` across the GC-capable
/// `create_promise`).
fn resolved_promise(vm: &mut VmInner, value: JsValue) -> JsValue {
    JsValue::Object(super::promise_resolve(vm, value))
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

/// `clients.matchAll(options?)` → `Promise<sequence<Client>>` (SW §4.3.2).
/// Filters the snapshot by `options.type` (default `"window"`).
/// `includeUncontrolled` is read but does not widen the result — the SW
/// realm only tracks the clients it controls (production widening = D-26).
fn native_clients_match_all(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_clients(ctx, this)?;
    let options = args.first().copied().unwrap_or(JsValue::Undefined);
    let type_filter = read_type_option(ctx, options)?;

    // Clone the matching snapshots out before the mutable build borrow.
    let matched: Vec<ClientSnapshot> = ctx
        .vm
        .sw_clients
        .iter()
        .filter(|c| type_filter.matches(c.client_type))
        .cloned()
        .collect();

    // GC-safety: each `build_client_object` allocates (and can GC); root the
    // growing element set on the VM stack across the accumulation.
    let mut frame = ctx.vm.push_stack_scope();
    let base = frame.saved_len();
    for snap in &matched {
        let obj = build_client_object(&mut frame, snap);
        frame.stack.push(JsValue::Object(obj));
    }
    let elements: Vec<JsValue> = frame.stack[base..].to_vec();
    let arr = frame.create_array_object(elements);
    drop(frame);
    Ok(resolved_promise(ctx.vm, JsValue::Object(arr)))
}

/// `clients.get(id)` → `Promise<Client | undefined>` (SW §4.3.1).
fn native_clients_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_clients(ctx, this)?;
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let sid = super::super::super::coerce::to_string(ctx.vm, arg)?;
    let wanted = ctx.vm.strings.get_utf8(sid);
    let found = ctx.vm.sw_clients.iter().find(|c| c.id == wanted).cloned();
    let value = match found {
        Some(snap) => JsValue::Object(build_client_object(ctx.vm, &snap)),
        None => JsValue::Undefined,
    };
    Ok(resolved_promise(ctx.vm, value))
}

/// `clients.claim()` → `Promise<undefined>` (SW §4.3.4): stage a
/// `SwToContent::ClientsClaim` and resolve.
///
/// §4.3.4 step 1 rejects with `InvalidStateError` when the SW is not the
/// active worker; that gate needs the activation state the coordinator→VM
/// back-channel carries (PR-3 / D-26), so PR-2 resolves (the SW is running
/// and already activated by the coordinator before fetch traffic flows) and
/// sends the wire signal.
fn native_clients_claim(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_clients(ctx, this)?;
    ctx.vm.queue_sw_message(SwToContent::ClientsClaim);
    Ok(resolved_promise(ctx.vm, JsValue::Undefined))
}

/// `Client.postMessage(message)` (SW §4.2): stage a
/// `SwToContent::PostMessage` routed to *this* client's own id (F7 — not
/// boa's empty-string TODO).  Returns `undefined`.
///
/// The IPC wire is correct; production *delivery* to the content client
/// (the `SwToContent::PostMessage` consumer) is the PR-3 / D-26 back-channel.
fn native_client_post_message(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(
            "Illegal invocation: receiver is not a Client",
        ));
    };
    let Some(snap) = ctx.vm.client_states.get(&id) else {
        return Err(VmError::type_error(
            "Illegal invocation: receiver is not a Client",
        ));
    };
    let client_id = snap.id.clone();
    let data = args.first().copied().unwrap_or(JsValue::Undefined);
    let serialized = super::super::worker_scope::serialize_message(ctx, data)?;
    ctx.vm.queue_sw_message(SwToContent::PostMessage {
        client_id,
        data: serialized,
    });
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// matchAll options
// ---------------------------------------------------------------------------

/// `ClientQueryOptions.type` filter (SW §4.3.2): `window` (default) /
/// `worker` / `sharedworker` / `all`.
enum TypeFilter {
    Window,
    Worker,
    SharedWorker,
    All,
}

impl TypeFilter {
    fn matches(&self, t: ClientType) -> bool {
        match self {
            TypeFilter::All => true,
            TypeFilter::Window => matches!(t, ClientType::Window),
            TypeFilter::Worker => matches!(t, ClientType::Worker),
            TypeFilter::SharedWorker => matches!(t, ClientType::SharedWorker),
        }
    }
}

/// Read `options.type` (default `"window"`).  A non-nullish, non-object
/// `options` is tolerated as the default dictionary (the only consulted
/// member is `type`).
fn read_type_option(ctx: &mut NativeContext<'_>, options: JsValue) -> Result<TypeFilter, VmError> {
    let JsValue::Object(opts_id) = options else {
        return Ok(TypeFilter::Window);
    };
    let key = PropertyKey::String(ctx.vm.strings.intern("type"));
    let val = ctx.get_property_value(opts_id, key)?;
    if matches!(val, JsValue::Undefined) {
        return Ok(TypeFilter::Window);
    }
    let sid = super::super::super::coerce::to_string(ctx.vm, val)?;
    let s = ctx.vm.strings.get_utf8(sid);
    Ok(match s.as_str() {
        "all" => TypeFilter::All,
        "worker" => TypeFilter::Worker,
        "sharedworker" => TypeFilter::SharedWorker,
        _ => TypeFilter::Window,
    })
}
