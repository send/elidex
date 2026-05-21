//! `#11-wrapper-identity-seam` regression tests.
//!
//! The seam folds every `[SameObject]` wrapper-identity cache in the VM
//! into one `HostData::wrapper_store` keyed by [`WrapperKey`], with the
//! GC mark / sweep behaviour dispatched per [`WrapperKind`].  These
//! tests exercise each of the five distinct mark/retain behaviours the
//! seam must preserve (one per `MarkAgent`), plus the kind-discriminator
//! invariant that lets two different wrapper kinds share one owner
//! without colliding:
//!
//! - **A. `StrongRoot`** (the primary node wrapper): survives GC even
//!   with no JS reference — the UAF-critical path.  Pruned only on
//!   entity despawn, never value-swept.
//! - **B. `WeakViaOwnerEntity`** (classList / dataset / Attr / …): a
//!   secondary wrapper survives GC while its owner element wrapper is
//!   still reachable (weak-through-owner).
//! - **C. `WeakViaOwnerEntityAndRuleId`** (CSSOM rule wrappers): the
//!   owner gate plus rule_id liveness — covered by
//!   `tests_cssom_sheet::deleted_rule_wrapper_caches_drop_after_gc`.
//! - **D. `ViaOwnerTrace`** (`<input>.files` FileList): marked by the
//!   owning `<input>` `HostObject`'s trace fan-out, so it survives
//!   while the input is reachable even with no direct JS reference.
//! - **E. `NoProactiveMark`** (`DataTransferItem`): nothing marks it
//!   proactively — it is collected once its own JS reference is gone,
//!   *even while the parent `DataTransfer` is alive* (the leak a naive
//!   "mark-iff-parent" rule would have caused).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, ObjectId};
use super::super::Vm;

/// Build a minimal `html > body` document and return its root.
fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

/// Evaluate `expr`, asserting it yields an object, and return its id.
fn eval_object_id(vm: &mut Vm, expr: &str) -> ObjectId {
    match vm.eval(expr).unwrap() {
        JsValue::Object(id) => id,
        other => panic!("expected object from `{expr}`, got {other:?}"),
    }
}

/// Whether the object slot is still live (not collected).
fn is_live(vm: &Vm, id: ObjectId) -> bool {
    vm.inner.objects[id.0 as usize].is_some()
}

// ── A. StrongRoot — node wrapper survives GC with no JS reference ──

#[test]
fn node_wrapper_survives_gc_without_js_ref() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Allocate a detached element wrapper, capture its id, then drop
    // every JS reference (clear the completion value with a second
    // eval) so only the seam's `StrongRoot` mark can keep it alive.
    let div_id = eval_object_id(&mut vm, "document.createElement('div')");
    vm.eval("undefined").unwrap();
    vm.inner.collect_garbage();
    assert!(
        is_live(&vm, div_id),
        "node wrapper must survive GC via MarkAgent::StrongRoot (UAF guard)"
    );
    vm.unbind();
}

// ── B. WeakViaOwnerEntity — classList survives while owner alive ──

#[test]
fn classlist_survives_while_owner_alive() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Keep the owner element reachable via a global; drop the
    // classList JS reference.  The weak-through-owner gate marks the
    // classList wrapper because the owner's primary `Node` wrapper is
    // still cached.
    vm.eval("globalThis.el = document.createElement('div'); undefined")
        .unwrap();
    let cl_id = eval_object_id(&mut vm, "el.classList");
    vm.eval("undefined").unwrap();
    vm.inner.collect_garbage();
    assert!(
        is_live(&vm, cl_id),
        "classList must survive GC while owner element is alive (WeakViaOwnerEntity)"
    );
    vm.unbind();
}

// ── D. ViaOwnerTrace — FileList survives while <input> alive ──

#[test]
fn input_files_survives_while_input_alive() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // The `<input>` is reachable via a global; the FileList JS
    // reference is dropped.  The input `HostObject`'s trace fan-out
    // marks the cached FileList through the seam store.
    vm.eval("globalThis.inp = document.createElement('input'); inp.type = 'file'; undefined")
        .unwrap();
    let files_id = eval_object_id(&mut vm, "inp.files");
    vm.eval("undefined").unwrap();
    vm.inner.collect_garbage();
    assert!(
        is_live(&vm, files_id),
        "FileList must survive GC while the <input> is alive (ViaOwnerTrace)"
    );
    // Identity is stable across reads + a GC cycle.
    let files_again = eval_object_id(&mut vm, "inp.files");
    assert_eq!(
        files_id, files_again,
        "input.files === input.files [SameObject]"
    );
    vm.unbind();
}

// ── E. NoProactiveMark — DataTransferItem NOT kept alive by parent ──

#[test]
fn data_transfer_item_not_promoted_by_parent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Parent DataTransfer stays reachable via a global; the only JS
    // reference to the item wrapper is then dropped.  Because the item
    // has no proactive mark agent, it must be collected even though the
    // parent (and its items list) are alive — the leak a naive
    // "mark-iff-parent" rule would have caused.
    // Indexed access (`dt.items[0]`) is deferred to
    // `#11-events-modern-indexed-exotic`; `add()` returns the same
    // interned item wrapper, so capture the id from there.
    vm.eval("globalThis.dt = new DataTransfer(); undefined")
        .unwrap();
    let item_id = eval_object_id(&mut vm, "dt.items.add('hi', 'text/plain')");
    vm.eval("undefined").unwrap();
    vm.inner.collect_garbage();
    assert!(
        !is_live(&vm, item_id),
        "DataTransferItem must be collected once unreferenced, even with parent alive \
         (NoProactiveMark — leak guard)"
    );
    vm.unbind();
}

// ── Kind discriminator — two kinds on one entity don't collide ──

#[test]
fn distinct_kinds_on_same_entity_do_not_collide() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // classList and dataset share the owner `Entity` but are different
    // `WrapperKind`s: each must be `[SameObject]`-stable yet distinct
    // from the other (the kind field is what the single key uses to
    // keep them apart now that they no longer have separate maps).
    let result = vm
        .eval(
            "globalThis.el = document.createElement('div'); \
             el.classList === el.classList \
               && el.dataset === el.dataset \
               && el.classList !== el.dataset",
        )
        .unwrap();
    assert_eq!(
        result,
        JsValue::Boolean(true),
        "classList / dataset must be per-kind stable and mutually distinct on one entity"
    );
    vm.unbind();
}
