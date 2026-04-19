//! `CharacterData.prototype` tests — `data` / `length` accessors and
//! the `appendData` / `insertData` / `deleteData` / `replaceData` /
//! `substringData` methods, plus the prototype-chain invariant
//! that Text and Comment inherit these members.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, eval_num, eval_str};
use super::super::value::JsValue;
use super::super::Vm;

fn setup() -> (Vm, SessionCore, EcsDom, elidex_ecs::Entity) {
    let vm = Vm::new();
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (vm, session, dom, doc)
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

#[test]
fn text_data_get_and_set() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('hello');")
        .unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "hello");
    vm.eval("t.data = 'updated';").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "updated");
    vm.unbind();
}

#[test]
fn comment_data_get_and_set() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.c = document.createComment('foo');")
        .unwrap();
    assert_eq!(eval_str(&mut vm, "c.data;"), "foo");
    vm.eval("c.data = 'bar';").unwrap();
    assert_eq!(eval_str(&mut vm, "c.data;"), "bar");
    vm.unbind();
}

#[test]
fn text_length_ascii() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('abcde');")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "t.length;"), 5.0);
    vm.unbind();
}

#[test]
fn text_length_counts_utf16_code_units_for_emoji() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // 🎉 (U+1F389) encodes as a surrogate pair (2 UTF-16 units) per
    // spec; JS String `'🎉'.length === 2` — CharacterData.length must
    // match.
    vm.eval("globalThis.t = document.createTextNode('\\uD83C\\uDF89');")
        .unwrap();
    assert_eq!(eval_num(&mut vm, "t.length;"), 2.0);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

#[test]
fn text_append_data() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('foo');")
        .unwrap();
    vm.eval("t.appendData('bar');").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "foobar");
    vm.unbind();
}

#[test]
fn text_insert_data_at_offset() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('ab');")
        .unwrap();
    vm.eval("t.insertData(1, 'XYZ');").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "aXYZb");
    vm.unbind();
}

#[test]
fn text_delete_data_removes_range() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('abcdef');")
        .unwrap();
    vm.eval("t.deleteData(1, 3);").unwrap(); // remove 'bcd'
    assert_eq!(eval_str(&mut vm, "t.data;"), "aef");
    vm.unbind();
}

#[test]
fn text_replace_data_substitutes() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('abcdef');")
        .unwrap();
    vm.eval("t.replaceData(1, 3, 'XY');").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "aXYef");
    vm.unbind();
}

#[test]
fn text_substring_data_returns_slice() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('abcdef');")
        .unwrap();
    assert_eq!(eval_str(&mut vm, "t.substringData(1, 3);"), "bcd");
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn text_insert_data_at_zero_is_prepend() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('z');")
        .unwrap();
    vm.eval("t.insertData(0, 'abc');").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "abcz");
    vm.unbind();
}

#[test]
fn text_insert_data_at_length_is_append() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('abc');")
        .unwrap();
    vm.eval("t.insertData(3, 'XYZ');").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "abcXYZ");
    vm.unbind();
}

#[test]
fn text_delete_data_offset_exceeds_length_throws() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let threw = vm
        .eval(
            "var t = document.createTextNode('abc');\n\
             var err = null;\n\
             try { t.deleteData(10, 1); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Prototype chain
// ---------------------------------------------------------------------------

#[test]
fn text_is_instance_of_character_data_prototype() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // getPrototypeOf(text) — after C5 the immediate parent is
    // CharacterData.prototype (C5.5 inserts Text.prototype in
    // between).  We check the `data` method is found, which exercises
    // the prototype chain without depending on prototype identity.
    let JsValue::Boolean(b) = vm
        .eval(
            "var t = document.createTextNode('x');\n\
             typeof t.appendData === 'function';",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}

// ---------------------------------------------------------------------------
// Text.prototype.splitText (PR4e C5.5)
// ---------------------------------------------------------------------------

#[test]
fn text_split_text_mid_offset_returns_tail() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.p = document.createElement('p');")
        .unwrap();
    vm.eval("globalThis.t = document.createTextNode('hello world');")
        .unwrap();
    vm.eval("p.appendChild(t);").unwrap();
    vm.eval("globalThis.rest = t.splitText(5);").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "hello");
    assert_eq!(eval_str(&mut vm, "rest.data;"), " world");
    // New text inserted as next sibling of original.
    assert_eq!(eval_num(&mut vm, "p.childNodes.length;"), 2.0);
    vm.unbind();
}

#[test]
fn text_split_text_at_zero_leaves_original_empty() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('abc');")
        .unwrap();
    vm.eval("globalThis.r = t.splitText(0);").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "");
    assert_eq!(eval_str(&mut vm, "r.data;"), "abc");
    vm.unbind();
}

#[test]
fn text_split_text_at_length_returns_empty() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval("globalThis.t = document.createTextNode('abc');")
        .unwrap();
    vm.eval("globalThis.r = t.splitText(3);").unwrap();
    assert_eq!(eval_str(&mut vm, "t.data;"), "abc");
    assert_eq!(eval_str(&mut vm, "r.data;"), "");
    vm.unbind();
}

#[test]
fn text_split_text_beyond_length_throws() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let threw = vm
        .eval(
            "var t = document.createTextNode('abc');\n\
             var err = null;\n\
             try { t.splitText(100); } catch (e) { err = e; }\n\
             err !== null;",
        )
        .unwrap();
    assert!(matches!(threw, JsValue::Boolean(true)));
    vm.unbind();
}

#[test]
fn comment_inherits_character_data_members() {
    let (mut vm, mut session, mut dom, doc) = setup();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let JsValue::Boolean(b) = vm
        .eval(
            "var c = document.createComment('x');\n\
             typeof c.appendData === 'function' && typeof c.substringData === 'function';",
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(b);
    vm.unbind();
}
