//! D-15 PR-B `#11-shadow-innerhtml-mixin` — JS-facing tests for
//! `Element` / `ShadowRoot` `innerHTML`, `outerHTML`, `setHTMLUnsafe`,
//! `getHTML` (WHATWG HTML §4.4.5 / §4.4.6 / §4.4.7) plus `cloneNode`
//! shadow-tree honouring (HTML §4.7.10 step 5).
//!
//! Shadow-encapsulation invariants (closed-mode opacity, default
//! serializer skipping shadow content) plus the declarative-shadow
//! parser hook (`<template shadowrootmode>` → attachShadow) all
//! intersect this surface — a fresh module keeps the lock-set
//! organised.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

// =====================================================================
// P0 tests — lock spec contracts that, if broken, would silently leak
// shadow content or mis-route mutation observers.  Always run first.
// =====================================================================

#[test]
fn element_inner_html_get_serializes_children() {
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.appendChild(document.createElement('p')); \
         d.innerHTML.indexOf('<p>') === 0 ? 'ok' : ('fail:' + d.innerHTML);");
    assert_eq!(out, "ok");
}

#[test]
fn element_inner_html_set_parses_fragment_and_replaces() {
    let out = run(
        "var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.appendChild(document.createElement('span')); \
         d.innerHTML = '<p>new</p>'; \
         (d.children.length === 1 && d.firstElementChild.tagName === 'P') \
             ? 'ok' : ('fail:' + d.children.length + ':' + (d.firstElementChild && d.firstElementChild.tagName));",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_inner_html_does_not_leak_shadow_content() {
    // Encapsulation lock — host.innerHTML must NOT expose the shadow
    // tree even when the shadow root has `serializable: true`.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open', serializable: true}); \
         sr.innerHTML = '<p>secret</p>'; \
         host.innerHTML.indexOf('secret') === -1 ? 'ok' : ('fail:' + host.innerHTML);");
    assert_eq!(out, "ok");
}

#[test]
fn element_outer_html_get_includes_self_tag() {
    let out = run(
        "var d = document.createElement('section'); \
         document.body.appendChild(d); \
         d.appendChild(document.createElement('p')); \
         d.outerHTML.startsWith('<section') && d.outerHTML.endsWith('</section>') ? 'ok' : ('fail:' + d.outerHTML);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_outer_html_set_replaces_self() {
    let out = run("var parent = document.createElement('div'); \
         document.body.appendChild(parent); \
         var target = document.createElement('span'); \
         parent.appendChild(target); \
         target.outerHTML = '<p>new</p>'; \
         (parent.children.length === 1 && parent.firstElementChild.tagName === 'P' \
            && target.parentNode === null) \
             ? 'ok' : ('fail:' + parent.children.length + ':' + parent.innerHTML);");
    assert_eq!(out, "ok");
}

#[test]
fn element_outer_html_set_throws_when_no_parent() {
    // NoModificationAllowedError when the entity has no parent.
    let out = run("var orphan = document.createElement('div'); \
         var caught = ''; \
         try { orphan.outerHTML = '<p></p>'; } \
         catch (e) { caught = e.name; } \
         caught === 'NoModificationAllowedError' ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_inner_html_get_serializes_shadow_children() {
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open'}); \
         sr.appendChild(document.createElement('p')); \
         sr.innerHTML.indexOf('<p>') === 0 ? 'ok' : ('fail:' + sr.innerHTML);");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_inner_html_set_replaces_shadow_children() {
    // `sr.children` exercises the ParentNode mixin reader inherited
    // via the ShadowRoot → DocumentFragment.prototype chain.
    let out = run(
        "var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open'}); \
         sr.appendChild(document.createElement('span')); \
         sr.innerHTML = '<p>new</p>'; \
         (sr.children.length === 1 && sr.firstElementChild.tagName === 'P') \
             ? 'ok' : ('fail:' + sr.children.length + ':' + (sr.firstElementChild && sr.firstElementChild.tagName));",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_set_html_unsafe_parses_template_shadowrootmode_as_shadow_root() {
    // Declarative shadow root attaches via setHTMLUnsafe (HTML §8.5.2).
    // `firstElementChild` reaches ShadowRoot via the ParentNode mixin
    // installed on `DocumentFragment.prototype`.
    let out = run(
        "var host = document.createElement('div'); \
         document.body.appendChild(host); \
         host.setHTMLUnsafe('<template shadowrootmode=\"open\"><p>x</p></template>'); \
         (host.shadowRoot !== null && host.shadowRoot.firstElementChild && host.shadowRoot.firstElementChild.tagName === 'P') \
             ? 'ok' : ('fail:' + (host.shadowRoot === null ? 'no-shadow' : host.shadowRoot.innerHTML));",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_set_html_unsafe_closed_mode_hides_shadow_root_from_getter() {
    // Closed declarative shadow root attaches but is opaque to
    // `element.shadowRoot` per §4.8 encapsulation.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         host.setHTMLUnsafe('<template shadowrootmode=\"closed\"></template>'); \
         host.shadowRoot === null ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn element_set_html_unsafe_on_host_with_existing_shadow_does_not_reattach() {
    // B1 silent fallback: a second declarative shadow attach on the same
    // host is rejected by `attach_shadow` and the existing shadow is
    // preserved; the template lands as a normal child.
    let out = run(
        "var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var first = host.attachShadow({mode: 'open'}); \
         first.appendChild(document.createElement('span')); \
         host.setHTMLUnsafe('<template shadowrootmode=\"open\"><p>x</p></template>'); \
         (host.shadowRoot === first && host.shadowRoot.firstChild && host.shadowRoot.firstChild.tagName === 'SPAN') \
             ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_set_html_unsafe_template_shadowroot_attrs_parse_as_booleans() {
    // Boolean attribute convention — value is irrelevant once the
    // attribute is present.  Verify all three optional boolean attrs
    // round-trip into the ShadowRoot init fields.
    let out = run(
        "var host = document.createElement('div'); \
         document.body.appendChild(host); \
         host.setHTMLUnsafe('<template shadowrootmode=\"open\" \
                                       shadowrootdelegatesfocus \
                                       shadowrootclonable \
                                       shadowrootserializable></template>'); \
         var sr = host.shadowRoot; \
         (sr.delegatesFocus === true && sr.clonable === true && sr.serializable === true) \
             ? 'ok' : ('fail:df=' + sr.delegatesFocus + ':cl=' + sr.clonable + ':sr=' + sr.serializable);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_inner_html_getter_throws_on_alien_receiver() {
    // WebIDL brand check — innerHTML getter called on a non-Element
    // receiver throws "Illegal invocation". The interface name globals
    // (Element / ShadowRoot) are not exposed yet — walk the prototype
    // chain from a sample instance to locate the descriptor.
    let out = run(
        "var elemProto = Object.getPrototypeOf(document.createElement('div')); \
         while (elemProto && !Object.getOwnPropertyDescriptor(elemProto, 'innerHTML')) { \
             elemProto = Object.getPrototypeOf(elemProto); \
         } \
         var getter = Object.getOwnPropertyDescriptor(elemProto, 'innerHTML').get; \
         var caught = ''; \
         try { getter.call({}); } catch (e) { caught = e.name; } \
         (caught === 'TypeError') ? 'ok' : ('fail:' + caught);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_inner_html_setter_error_message_uses_correct_interface_and_accessor() {
    // PR201 Copilot R2 / F1 regression: `require_brand` parameter
    // order was previously swapped so TypeError messages for alien
    // receivers read "Failed to execute 'Element' on 'innerHTML'"
    // instead of "Failed to execute 'innerHTML' on 'Element'". Lock
    // the corrected order from JS.
    let out = run(
        "var elemProto = Object.getPrototypeOf(document.createElement('div')); \
         while (elemProto && !Object.getOwnPropertyDescriptor(elemProto, 'innerHTML')) { \
             elemProto = Object.getPrototypeOf(elemProto); \
         } \
         var setter = Object.getOwnPropertyDescriptor(elemProto, 'innerHTML').set; \
         var msg = ''; \
         try { setter.call({}, 'x'); } catch (e) { msg = e.message; } \
         (msg.indexOf(\"'innerHTML' on 'Element'\") !== -1) ? 'ok' : ('fail:' + msg);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_inner_html_getter_distinguishes_detached_from_wrong_brand() {
    // R1 fix: `require_brand` now mirrors `event_target::require_receiver`
    // — a wrapper whose backing entity has been destroyed surfaces the
    // "detached (invalid entity)" message rather than "Illegal
    // invocation", and a wrapper of the wrong brand still surfaces
    // "Illegal invocation". The split keeps debug output aligned with
    // the rest of the receiver-helper surface.
    let out = run(
        "var elemProto = Object.getPrototypeOf(document.createElement('div')); \
         while (elemProto && !Object.getOwnPropertyDescriptor(elemProto, 'innerHTML')) { \
             elemProto = Object.getPrototypeOf(elemProto); \
         } \
         var getter = Object.getOwnPropertyDescriptor(elemProto, 'innerHTML').get; \
         var alien = ''; \
         try { getter.call({}); } catch (e) { alien = e.message; } \
         (alien.indexOf('Illegal invocation') !== -1) ? 'ok' : ('fail:' + alien);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_inner_html_getter_throws_on_element_receiver() {
    // Post-H-migration discriminator: ShadowRoot.prototype.innerHTML
    // accessor uses the ECS-component brand check, not ObjectKind.
    // Calling it on an Element receiver must throw TypeError.
    let out = run("var host = document.createElement('div'); \
         var sr = host.attachShadow({mode: 'open'}); \
         var srProto = Object.getPrototypeOf(sr); \
         while (srProto && !Object.getOwnPropertyDescriptor(srProto, 'innerHTML')) { \
             srProto = Object.getPrototypeOf(srProto); \
         } \
         var getter = Object.getOwnPropertyDescriptor(srProto, 'innerHTML').get; \
         var elem = document.createElement('div'); \
         var caught = ''; \
         try { getter.call(elem); } catch (e) { caught = e.name; } \
         (caught === 'TypeError') ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

// =====================================================================
// P1 tests — broader spec coverage; surface getHTML / cloneNode /
// brand-check edge cases.
// =====================================================================

#[test]
fn element_get_html_default_excludes_shadow_roots() {
    // `getHTML()` with no options behaves like `innerHTML` — shadow
    // content is skipped per §4.4.6.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open', serializable: true}); \
         sr.innerHTML = '<p>secret</p>'; \
         host.appendChild(document.createElement('span')); \
         host.getHTML().indexOf('secret') === -1 ? 'ok' : ('fail:' + host.getHTML());");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_with_serializable_shadow_roots_emits_template_for_serializable_only() {
    // Two hosts — one shadow with `serializable: true`, one without.
    // `getHTML({serializableShadowRoots:true})` only emits the
    // template for the serializable one.
    let out = run("var hostA = document.createElement('div'); \
         document.body.appendChild(hostA); \
         var srA = hostA.attachShadow({mode: 'open', serializable: true}); \
         srA.innerHTML = '<p>A</p>'; \
         var hostB = document.createElement('div'); \
         document.body.appendChild(hostB); \
         var srB = hostB.attachShadow({mode: 'open', serializable: false}); \
         srB.innerHTML = '<p>B</p>'; \
         var aOut = hostA.getHTML({serializableShadowRoots: true}); \
         var bOut = hostB.getHTML({serializableShadowRoots: true}); \
         (aOut.indexOf('shadowrootmode') !== -1 \
            && aOut.indexOf('<p>A</p>') !== -1 \
            && bOut.indexOf('shadowrootmode') === -1) \
             ? 'ok' : ('fail:a=' + aOut + ' b=' + bOut);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_explicit_list_emits_closed_shadow_root() {
    // B3 — `shadowRoots: [sr]` force-emits regardless of mode/serializable.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'closed'}); \
         sr.appendChild(document.createElement('p')); \
         var got = host.getHTML({shadowRoots: [sr]}); \
         (got.indexOf('shadowrootmode=\"closed\"') !== -1 \
            && got.indexOf('<p>') !== -1) \
             ? 'ok' : ('fail:' + got);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_shadow_roots_string_iterates_per_codepoint() {
    // PR201 Copilot R12 regression: WebIDL `sequence<T>` conversion
    // accepts any iterable primitive — strings carry
    // `String.prototype[@@iterator]` (code-point iteration). The
    // earlier `JsValue::Object` guard up-front rejected strings with
    // "not iterable", diverging from spec. The fix routes everything
    // through `resolve_iterator`, so a string now iterates per
    // code-point; each iteration yields a single-char string which
    // is not a ShadowRoot → spec-correct TypeError saying
    // "shadowRoots[0] is not a ShadowRoot" instead of the misleading
    // "shadowRoots is not iterable". Both throw TypeError (so
    // error.name checks work), but the message refines per spec.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var caught = ''; \
         try { host.getHTML({shadowRoots: 'abc'}); } catch (e) { caught = e.message; } \
         (caught.indexOf(\"'shadowRoots[0]' is not a ShadowRoot\") !== -1) \
             ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_shadow_roots_consumes_custom_iterable() {
    // PR201 Copilot R8 / F1 regression: WebIDL `sequence<T>` consumes
    // `@@iterator`. A custom iterable (or generator, or `new Set`)
    // must produce the same explicit-list emission as the literal
    // array form. The earlier draft only handled dense `Array` +
    // `{length, 0..}` indexed walk and silently dropped iterables to
    // an empty list. `Set` isn't exposed in the test VM, so the test
    // hand-rolls an iterable via `[Symbol.iterator]()` returning a
    // single-shot iterator.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'closed'}); \
         sr.appendChild(document.createElement('p')); \
         var iterable = { \
             [Symbol.iterator]() { \
                 var done = false; \
                 return { next() { \
                     if (done) return { done: true, value: undefined }; \
                     done = true; \
                     return { done: false, value: sr }; \
                 } }; \
             } \
         }; \
         var got = host.getHTML({shadowRoots: iterable}); \
         (got.indexOf('shadowrootmode=\"closed\"') !== -1 \
            && got.indexOf('<p>') !== -1) \
             ? 'ok' : ('fail:' + got);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_shadow_roots_plain_array_like_throws_typeerror() {
    // Companion R8 lock: a plain `{length, 0..}` array-like (no
    // `@@iterator`) is NOT iterable per spec, so the sequence
    // converter must throw TypeError instead of silently treating it
    // as a length-bounded list (which was the pre-R8 behaviour).
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var bogus = { length: 1, 0: document.createElement('div') }; \
         var caught = ''; \
         try { host.getHTML({shadowRoots: bogus}); } catch (e) { caught = e.name; } \
         (caught === 'TypeError') ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_shadow_roots_null_throws_typeerror() {
    // PR201 Copilot R6 / F1 regression: per WebIDL §3.10.16, a
    // dictionary member's default is applied ONLY when the value is
    // `undefined`. `null` is passed through to the sequence converter,
    // which rejects it because `sequence<ShadowRoot>` is not nullable.
    // The earlier draft conflated null and undefined, silently
    // accepting `{shadowRoots: null}` as the default empty sequence.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var caught = ''; \
         try { host.getHTML({shadowRoots: null}); } catch (e) { caught = e.name; } \
         (caught === 'TypeError') ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_shadow_roots_array_like_with_invalid_first_short_circuits() {
    // PR201 Copilot R6 / F2 regression: a hostile `{length: 1<<27, 0: bogus}`
    // would previously iterate the full ~134M length interning index
    // strings and pushing into a Vec before validating, even though
    // index 0 is invalid. The fix validates inline so the first
    // bogus element terminates the loop immediately, AND the
    // length is capped at `SHADOW_ROOTS_SEQ_CAP` (4096) as defence
    // in depth. Test asserts a `{length: 1<<27}` invocation
    // terminates synchronously with the expected TypeError.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var bogus = { length: 134217728, 0: document.createElement('div') }; \
         var caught = ''; \
         try { host.getHTML({shadowRoots: bogus}); } catch (e) { caught = e.name; } \
         (caught === 'TypeError') ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_dense_array_shadow_roots_capped_safely() {
    // DoS bound: `{shadowRoots: new Array(5000)}` terminates
    // synchronously with TypeError (sparse hole → undefined → validator
    // brand-check fails at index 0).
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var arr = new Array(5000); \
         var caught = ''; \
         try { host.getHTML({shadowRoots: arr}); } catch (e) { caught = e.name; } \
         (caught === 'TypeError') ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_shadow_roots_honours_array_iterator_override() {
    // WebIDL §3.10.16 step 4: an Array whose `[Symbol.iterator]` is
    // overridden must use the override, not dense storage.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'closed'}); \
         sr.appendChild(document.createElement('p')); \
         var arr = [sr, sr, sr]; \
         arr[Symbol.iterator] = function() { \
             var i = 0; \
             return { next: function() { \
                 if (i >= 1) return { value: undefined, done: true }; \
                 i++; \
                 return { value: sr, done: false }; \
             } }; \
         }; \
         var got = host.getHTML({shadowRoots: arr}); \
         /* override emits 1 entry; dense walk would emit 3 */ \
         (got.indexOf('shadowrootmode=\"closed\"') !== -1 \
            && got.indexOf('<p>') !== -1) \
             ? 'ok' : ('fail:' + got);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_shadow_roots_iter_close_runs_on_validator_throw() {
    // §7.4.11: validator throw triggers IteratorClose; custom .return()
    // must fire before the TypeError propagates.
    let out = run(
        "var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var bogus = document.createElement('div'); \
         var returned = false; \
         var iterable = { \
             [Symbol.iterator]() { \
                 return { \
                     next() { return { value: bogus, done: false }; }, \
                     return() { returned = true; return { value: undefined, done: true }; }, \
                 }; \
             } \
         }; \
         var caught = ''; \
         try { host.getHTML({shadowRoots: iterable}); } catch (e) { caught = e.name; } \
         (caught === 'TypeError' && returned === true) ? 'ok' : ('fail:' + caught + '/' + returned);",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_huge_shadow_roots_length_is_capped_safely() {
    // PR201 Copilot R4 / F1 regression: a malicious caller passing
    // `{length: 2**31}` (or any value beyond `DENSE_ARRAY_LEN_LIMIT`)
    // would previously force a multi-GB `Vec::with_capacity` allocation
    // and exhaust process memory. The cap mirrors
    // `natives_function.rs::collect_array_like`. The probe completes
    // synchronously and produces a TypeError ("not a ShadowRoot") on
    // the first iteration — no OOM.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var bogus = { length: 2147483648, 0: document.createElement('div') }; \
         var caught = ''; \
         try { host.getHTML({shadowRoots: bogus}); } catch (e) { caught = e.name; } \
         (caught === 'TypeError') ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_throws_typeerror_on_non_object_primitive_options() {
    // PR201 Copilot R3 / F1 regression: WebIDL §3.10.16 dictionary
    // conversion throws TypeError for non-Object / non-null /
    // non-undefined arguments. The earlier draft silently fell back to
    // the default dictionary, which masked spec violations in caller
    // code. Locked here with three representative primitives.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         function tryCall(arg) { \
             try { host.getHTML(arg); return null; } \
             catch (e) { return e.name; } \
         } \
         var n = tryCall(42); \
         var s = tryCall('opts'); \
         var b = tryCall(true); \
         (n === 'TypeError' && s === 'TypeError' && b === 'TypeError') \
             ? 'ok' : ('fail:n=' + n + ' s=' + s + ' b=' + b);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_explicit_shadow_root_brand_check_throws_on_non_shadow_root() {
    // Per WebIDL sequence<ShadowRoot> — a non-ShadowRoot element in
    // the sequence throws TypeError.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var bogus = document.createElement('div'); \
         var caught = ''; \
         try { host.getHTML({shadowRoots: [bogus]}); } catch (e) { caught = e.name; } \
         caught === 'TypeError' ? 'ok' : ('fail:' + caught);");
    assert_eq!(out, "ok");
}

#[test]
fn clone_node_deep_clones_shadow_tree_when_clonable_true() {
    let out = run(
        "var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open', clonable: true}); \
         sr.appendChild(document.createElement('p')); \
         var clone = host.cloneNode(true); \
         (clone.shadowRoot !== null && clone.shadowRoot.firstChild && clone.shadowRoot.firstChild.tagName === 'P' \
            && clone.shadowRoot !== sr) \
             ? 'ok' : ('fail:' + (clone.shadowRoot === null ? 'no-shadow' : 'has-shadow'));",
    );
    assert_eq!(out, "ok");
}

#[test]
fn clone_node_deep_skips_shadow_tree_when_clonable_false() {
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open', clonable: false}); \
         sr.appendChild(document.createElement('p')); \
         var clone = host.cloneNode(true); \
         clone.shadowRoot === null ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn clone_node_shallow_does_not_clone_shadow_tree_regardless_of_clonable() {
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open', clonable: true}); \
         sr.appendChild(document.createElement('p')); \
         var clone = host.cloneNode(false); \
         clone.shadowRoot === null ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn shadow_root_set_html_unsafe_parses_inner_shadow_template_as_nested_shadow() {
    // Nested declarative shadow within a ShadowRoot's setHTMLUnsafe:
    // the outer ShadowRoot itself is not a shadow host, but a
    // descendant <div> inside its content can be — and that descendant
    // receives a declarative shadow attached from a nested template.
    let out = run(
        "var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open'}); \
         sr.setHTMLUnsafe('<div><template shadowrootmode=\"open\"><p>inner</p></template></div>'); \
         var inner = sr.firstChild; \
         (inner && inner.tagName === 'DIV' && inner.shadowRoot && inner.shadowRoot.firstChild.tagName === 'P') \
             ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_no_args_uses_default_dict_excludes_shadows() {
    // `getHTML()` (no argument) defaults to `{serializableShadowRoots:
    // false, shadowRoots: []}` per WebIDL dictionary defaults.
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         var sr = host.attachShadow({mode: 'open', serializable: true}); \
         sr.appendChild(document.createElement('p')); \
         host.getHTML().indexOf('shadowrootmode') === -1 ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_round_trip_with_set_html_unsafe() {
    // Round-trip discriminator: getHTML output passed back to
    // setHTMLUnsafe must reattach an equivalent declarative shadow
    // root (HTML §13.5 serialization → §4.12.3 `<template shadowrootmode>` parser hook).
    let out = run(
        "var src = document.createElement('div'); \
         document.body.appendChild(src); \
         var srA = src.attachShadow({mode: 'open', serializable: true}); \
         srA.appendChild(document.createElement('p')); \
         var html = src.getHTML({serializableShadowRoots: true}); \
         var dst = document.createElement('div'); \
         document.body.appendChild(dst); \
         dst.setHTMLUnsafe(html); \
         (dst.shadowRoot !== null && dst.shadowRoot.firstChild && dst.shadowRoot.firstChild.tagName === 'P') \
             ? 'ok' : ('fail:html=' + html + ':shadow=' + (dst.shadowRoot === null ? 'no' : 'yes'));",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_emits_shadowrootslotassignment_for_manual_mode() {
    // Round-trip lock for `slotAssignment: 'manual'` — the serializer
    // must emit `shadowrootslotassignment="manual"` so a subsequent
    // `setHTMLUnsafe(getHTML(...))` preserves the manual mode (HTML
    // §4.12.3 / §13.5). Named mode is the default and is intentionally
    // omitted from the serialised attribute set to keep the round-trip
    // terse for the common case.
    let out = run("var named = document.createElement('div'); \
         document.body.appendChild(named); \
         named.attachShadow({mode: 'open', slotAssignment: 'named', serializable: true}); \
         var manual = document.createElement('div'); \
         document.body.appendChild(manual); \
         manual.attachShadow({mode: 'open', slotAssignment: 'manual', serializable: true}); \
         var namedOut = named.getHTML({serializableShadowRoots: true}); \
         var manualOut = manual.getHTML({serializableShadowRoots: true}); \
         (namedOut.indexOf('shadowrootslotassignment') === -1 \
            && manualOut.indexOf('shadowrootslotassignment=\"manual\"') !== -1) \
             ? 'ok' : ('fail:named=' + namedOut + ' manual=' + manualOut);");
    assert_eq!(out, "ok");
}

#[test]
fn element_get_html_manual_slot_assignment_round_trip() {
    // Round-trip discriminator: getHTML output of a manual-mode shadow
    // host fed back through setHTMLUnsafe must produce a shadow root
    // whose `slotAssignment` is again `'manual'`.
    let out = run(
        "var src = document.createElement('div'); \
         document.body.appendChild(src); \
         src.attachShadow({mode: 'open', slotAssignment: 'manual', serializable: true}); \
         var html = src.getHTML({serializableShadowRoots: true}); \
         var dst = document.createElement('div'); \
         document.body.appendChild(dst); \
         dst.setHTMLUnsafe(html); \
         (dst.shadowRoot !== null && dst.shadowRoot.slotAssignment === 'manual') \
             ? 'ok' : ('fail:html=' + html + ':mode=' + (dst.shadowRoot && dst.shadowRoot.slotAssignment));",
    );
    assert_eq!(out, "ok");
}

#[test]
fn element_inner_html_setter_does_not_attach_declarative_shadow() {
    // innerHTML default opts → `<template shadowrootmode>` stays as a
    // plain `<template>` element (no declarative attach).
    let out = run("var host = document.createElement('div'); \
         document.body.appendChild(host); \
         host.innerHTML = '<template shadowrootmode=\"open\"></template>'; \
         host.shadowRoot === null ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}
