//! Element attribute-method tests — `tagName` / `getAttribute` /
//! `setAttribute` / `removeAttribute` / `hasAttribute` /
//! `getAttributeNames` / `toggleAttribute` + `id` / `className`.
//!
//! Split out of [`super::tests_element_methods`] to keep that file
//! under the project's 1000-line convention (PR5a C9).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;
use super::tests_element_methods::build_element_fixture;

// ---------------------------------------------------------------------------
// Attributes: tagName / getAttribute / setAttribute / …
// ---------------------------------------------------------------------------

#[test]
fn element_tag_name_is_upper_case() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm.eval("document.getElementById('root').tagName;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "BODY");

    vm.unbind();
}

#[test]
fn element_get_attribute_present_and_missing() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // body has id="root" and no class.
    let v = vm
        .eval("document.getElementById('root').getAttribute('id');")
        .unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "root");

    assert!(matches!(
        vm.eval("document.getElementById('root').getAttribute('nonexistent');")
            .unwrap(),
        JsValue::Null
    ));

    vm.unbind();
}

#[test]
fn element_set_attribute_then_get_and_has() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        "var el = document.getElementById('root'); \
         el.setAttribute('data-x', 'hello');",
    )
    .unwrap();

    let v = vm
        .eval("document.getElementById('root').getAttribute('data-x');")
        .unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "hello");

    assert!(matches!(
        vm.eval("document.getElementById('root').hasAttribute('data-x');")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').hasAttribute('missing');")
            .unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn element_remove_attribute_is_silent_when_missing() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Grab the wrapper before removing the id — id-based lookup
    // would fail after removal.  Run the whole scenario in one
    // script so locals survive between statements.
    let v = vm
        .eval(
            "var el = document.getElementById('root');\n\
             el.removeAttribute('id');\n\
             el.removeAttribute('missing');\n\
             el.hasAttribute('id') ? 'bug' : 'ok';",
        )
        .unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "ok");

    vm.unbind();
}

#[test]
fn element_get_attribute_names_is_array_in_insertion_order() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // div has class="box" (only one attr).  Add two more via setAttribute.
    let div_wrapper = vm.inner.create_element_wrapper(div);
    vm.set_global("_div", JsValue::Object(div_wrapper));
    vm.eval("_div.setAttribute('data-a', '1'); _div.setAttribute('data-b', '2');")
        .unwrap();
    let len = vm.eval("_div.getAttributeNames().length;").unwrap();
    let JsValue::Number(n) = len else { panic!() };
    assert!((n - 3.0).abs() < f64::EPSILON, "got {n}");

    // Each entry is a string.  Verify the first (original) slot.
    let first = vm.eval("_div.getAttributeNames()[0];").unwrap();
    let JsValue::String(sid) = first else {
        panic!()
    };
    assert_eq!(vm.get_string(sid), "class");

    vm.unbind();
}

#[test]
fn element_toggle_attribute_without_force_toggles_presence() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // First call: absent → add → returns true.  Value is empty string.
    let on = vm
        .eval("document.getElementById('root').toggleAttribute('hidden');")
        .unwrap();
    assert!(matches!(on, JsValue::Boolean(true)));
    assert!(matches!(
        vm.eval("document.getElementById('root').hasAttribute('hidden');")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    // Second call: present → remove → returns false.
    let off = vm
        .eval("document.getElementById('root').toggleAttribute('hidden');")
        .unwrap();
    assert!(matches!(off, JsValue::Boolean(false)));
    assert!(matches!(
        vm.eval("document.getElementById('root').hasAttribute('hidden');")
            .unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn element_toggle_attribute_with_force_is_idempotent() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // force=true both times — still present, returns true.
    assert!(matches!(
        vm.eval("document.getElementById('root').toggleAttribute('hidden', true);")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    assert!(matches!(
        vm.eval("document.getElementById('root').toggleAttribute('hidden', true);")
            .unwrap(),
        JsValue::Boolean(true)
    ));
    // force=false while present → remove.
    assert!(matches!(
        vm.eval("document.getElementById('root').toggleAttribute('hidden', false);")
            .unwrap(),
        JsValue::Boolean(false)
    ));
    // force=false while absent → still absent.
    assert!(matches!(
        vm.eval("document.getElementById('root').toggleAttribute('hidden', false);")
            .unwrap(),
        JsValue::Boolean(false)
    ));

    vm.unbind();
}

#[test]
fn element_id_reflected_getter_setter() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let v = vm.eval("document.getElementById('root').id;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "root");

    vm.eval("document.getElementById('root').id = 'new-id';")
        .unwrap();
    let v = vm.eval("document.getElementById('new-id').id;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "new-id");

    vm.unbind();
}

#[test]
fn element_class_name_reflects_class_attribute() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, p, _div, _span, _raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let p_wrapper = vm.inner.create_element_wrapper(p);
    vm.set_global("_p", JsValue::Object(p_wrapper));
    let v = vm.eval("_p.className;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "intro");

    vm.eval("_p.className = 'foo bar';").unwrap();
    assert!(matches!(
        vm.eval("_p.getAttribute('class');").unwrap(),
        JsValue::String(_)
    ));
    let v = vm.eval("_p.className;").unwrap();
    let JsValue::String(sid) = v else { panic!() };
    assert_eq!(vm.get_string(sid), "foo bar");

    vm.unbind();
}

#[test]
fn element_id_on_text_node_is_undefined() {
    // `id` / `className` live on Element.prototype, so Text wrappers
    // (which inherit via Node.prototype, not Element.prototype) must
    // NOT expose them.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let (doc, _body, _p, _div, _span, raw, _com) = build_element_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let raw_wrapper = vm.inner.create_element_wrapper(raw);
    vm.set_global("_raw", JsValue::Object(raw_wrapper));
    let t = vm.eval("typeof _raw.id;").unwrap();
    let JsValue::String(sid) = t else { panic!() };
    assert_eq!(vm.get_string(sid), "undefined");

    vm.unbind();
}
