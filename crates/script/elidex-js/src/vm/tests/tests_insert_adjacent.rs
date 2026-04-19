//! PR4f C4: `Element.prototype.insertAdjacentElement` /
//! `insertAdjacentText` — WHATWG DOM §4.9.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_pair_in_parent(dom: &mut EcsDom) -> elidex_ecs::Entity {
    // `<section>` wrapper contains a `<div id="t"/>` target and a
    // sibling `<span id="sib"/>` to the right, so `beforebegin` /
    // `afterend` have something to land next to.
    let doc = dom.create_document_root();
    let section = dom.create_element("section", Attributes::default());
    let target = dom.create_element("div", {
        let mut a = Attributes::default();
        a.set("id", "t");
        a
    });
    let sib = dom.create_element("span", {
        let mut a = Attributes::default();
        a.set("id", "sib");
        a
    });
    assert!(dom.append_child(doc, section));
    assert!(dom.append_child(section, target));
    assert!(dom.append_child(section, sib));
    doc
}

fn build_detached_target(dom: &mut EcsDom) -> elidex_ecs::Entity {
    dom.create_document_root()
}

fn run(script: &str, fixture: impl FnOnce(&mut EcsDom) -> elidex_ecs::Entity) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}");
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

#[test]
fn insert_adjacent_element_beforebegin() {
    let out = run(
        "var t = document.getElementById('t');\
         var p = document.createElement('p');\
         var r = t.insertAdjacentElement('beforebegin', p);\
         r === p ? 'ok:' + t.parentNode.firstChild.tagName : 'fail';",
        build_pair_in_parent,
    );
    assert_eq!(out, "ok:P");
}

#[test]
fn insert_adjacent_element_afterbegin() {
    let out = run(
        "var t = document.getElementById('t');\
         t.appendChild(document.createElement('em'));\
         var p = document.createElement('p');\
         t.insertAdjacentElement('afterbegin', p);\
         t.firstChild.tagName + '|' + t.lastChild.tagName;",
        build_pair_in_parent,
    );
    assert_eq!(out, "P|EM");
}

#[test]
fn insert_adjacent_element_beforeend() {
    let out = run(
        "var t = document.getElementById('t');\
         t.appendChild(document.createElement('em'));\
         var p = document.createElement('p');\
         t.insertAdjacentElement('beforeend', p);\
         t.firstChild.tagName + '|' + t.lastChild.tagName;",
        build_pair_in_parent,
    );
    assert_eq!(out, "EM|P");
}

#[test]
fn insert_adjacent_element_afterend() {
    let out = run(
        "var t = document.getElementById('t');\
         var p = document.createElement('p');\
         t.insertAdjacentElement('afterend', p);\
         t.nextSibling.tagName + '|' + t.parentNode.lastChild.tagName;",
        build_pair_in_parent,
    );
    // After `afterend` with an existing trailing sibling:
    //   section > div#t, p, span#sib   ← p inserted between t and sib
    assert_eq!(out, "P|SPAN");
}

#[test]
fn insert_adjacent_element_beforebegin_no_parent_returns_null() {
    let out = run(
        "var t = document.createElement('div');\
         var p = document.createElement('p');\
         var r = t.insertAdjacentElement('beforebegin', p);\
         r === null && t.childNodes.length === 0 ? 'ok' : 'fail';",
        build_detached_target,
    );
    assert_eq!(out, "ok");
}

#[test]
fn insert_adjacent_element_afterend_no_parent_returns_null() {
    let out = run(
        "var t = document.createElement('div');\
         var p = document.createElement('p');\
         var r = t.insertAdjacentElement('afterend', p);\
         r === null && t.childNodes.length === 0 ? 'ok' : 'fail';",
        build_detached_target,
    );
    assert_eq!(out, "ok");
}

#[test]
fn insert_adjacent_element_rejects_bogus_where() {
    // CallMethod coerces VmError into its Display string (separate
    // pre-existing VM quirk from Op::Call's Error-object wrap), so we
    // assert on the string contents rather than e.name / instanceof.
    let out = run(
        "var t = document.getElementById('t');\
         var p = document.createElement('p');\
         try { t.insertAdjacentElement('sideways', p); 'no-throw'; } \
         catch (e) { \
           var isType = (typeof e === 'string' && e.indexOf('TypeError') >= 0);\
           var unchanged = t.parentNode.childNodes.length === 2;\
           isType + ':' + unchanged; }",
        build_pair_in_parent,
    );
    assert_eq!(out, "true:true");
}

#[test]
fn insert_adjacent_element_rejects_non_element_arg() {
    let out = run(
        "var t = document.getElementById('t');\
         try { t.insertAdjacentElement('beforeend', null); 'no-throw'; } \
         catch (e) { (typeof e === 'string' && e.indexOf('TypeError') >= 0) ? 'threw' : 'bad'; }",
        build_pair_in_parent,
    );
    assert_eq!(out, "threw");
}

#[test]
fn insert_adjacent_element_where_is_ascii_case_insensitive() {
    // Spec requires ASCII case-insensitive match on the where literal.
    let out = run(
        "var t = document.getElementById('t');\
         var p = document.createElement('p');\
         t.insertAdjacentElement('BEFOREbeGIN', p);\
         t.parentNode.firstChild.tagName;",
        build_pair_in_parent,
    );
    assert_eq!(out, "P");
}

#[test]
fn insert_adjacent_text_afterbegin_creates_text() {
    let out = run(
        "var t = document.getElementById('t');\
         t.appendChild(document.createElement('em'));\
         var r = t.insertAdjacentText('afterbegin', 42);\
         typeof r + '|' + t.firstChild.nodeType + '|' + t.firstChild.data;",
        build_pair_in_parent,
    );
    // nodeType === 3 is Text.
    assert_eq!(out, "undefined|3|42");
}

#[test]
fn insert_adjacent_text_afterend_creates_text_sibling() {
    let out = run(
        "var t = document.getElementById('t');\
         t.insertAdjacentText('afterend', 'hi');\
         t.nextSibling.data;",
        build_pair_in_parent,
    );
    assert_eq!(out, "hi");
}

#[test]
fn insert_adjacent_text_no_parent_is_noop_returns_undefined() {
    let out = run(
        "var t = document.createElement('div');\
         var r = t.insertAdjacentText('beforebegin', 'hi');\
         typeof r + '|' + t.childNodes.length;",
        build_detached_target,
    );
    assert_eq!(out, "undefined|0");
}

#[test]
fn insert_adjacent_text_rejects_bogus_where_before_allocating_text() {
    // S6: position-parse failure is checked BEFORE the Text is created
    // so we don't leak detached Text nodes into the ECS on misuse.
    let out = run(
        "var t = document.getElementById('t');\
         try { t.insertAdjacentText('middle', 'x'); 'no-throw'; } \
         catch (e) { 'threw'; }",
        build_pair_in_parent,
    );
    assert_eq!(out, "threw");
}

#[test]
fn insert_adjacent_element_afterbegin_first_child_is_noop_success() {
    // Copilot R2 F4 lock-in: WHATWG treats "insert a node before
    // itself" as a no-op success, but `EcsDom::insert_before`
    // rejects `new_child == ref_child` as invalid.  The native must
    // short-circuit on that edge case so scripts like
    // `el.insertAdjacentElement('afterbegin', el.firstChild)` do not
    // throw — they are a common pattern for "ensure x is the first
    // child".
    let out = run(
        "var t = document.getElementById('t');\
         var kid = document.createElement('em');\
         t.appendChild(kid);\
         var r = t.insertAdjacentElement('afterbegin', kid);\
         r === kid && t.firstChild === kid ? 'ok' : 'fail';",
        build_pair_in_parent,
    );
    assert_eq!(out, "ok");
}

#[test]
fn insert_adjacent_element_beforebegin_self_is_noop_success() {
    // Copilot R2 F4 lock-in: `el.insertAdjacentElement('beforebegin', el)`
    // reduces to `parent.insertBefore(el, el)` which WHATWG treats as
    // a no-op success.
    let out = run(
        "var t = document.getElementById('t');\
         var parent = t.parentNode;\
         var r = t.insertAdjacentElement('beforebegin', t);\
         r === t && parent.childNodes[0] === t ? 'ok' : 'fail';",
        build_pair_in_parent,
    );
    assert_eq!(out, "ok");
}

#[test]
fn insert_adjacent_element_afterend_next_sibling_is_noop_success() {
    // Same WHATWG invariant applied to `afterend` with
    // `this.nextSibling` as the inserted node.
    let out = run(
        "var t = document.getElementById('t');\
         var sib = document.getElementById('sib');\
         var r = t.insertAdjacentElement('afterend', sib);\
         r === sib && t.nextSibling === sib ? 'ok' : 'fail';",
        build_pair_in_parent,
    );
    assert_eq!(out, "ok");
}

#[test]
fn insert_adjacent_text_parent_less_short_circuit_does_not_leak_text() {
    // Copilot R1 F2 lock-in — `beforebegin` / `afterend` on a
    // parent-less receiver used to allocate a Text entity before
    // realising the insertion was a no-op, leaking an orphan into
    // ECS.  Count Text entities before and after to confirm no new
    // Text survives.
    use elidex_ecs::TextContent;
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let script = "var t = document.createElement('div'); \
                  t.insertAdjacentText('beforebegin', 'ghost'); \
                  t.insertAdjacentText('afterend', 'ghost2'); \
                  t.childNodes.length;";
    let result = vm.eval(script).unwrap();
    assert!(matches!(result, JsValue::Number(n) if n == 0.0));
    vm.unbind();

    // No Text entity should exist in the ECS — the receiver had no
    // parent, so both insertAdjacentText calls are silent no-ops.
    let text_count = dom.world().query::<&TextContent>().iter().count();
    assert_eq!(
        text_count, 0,
        "insertAdjacentText on parent-less receiver leaked Text entities into ECS"
    );
}
