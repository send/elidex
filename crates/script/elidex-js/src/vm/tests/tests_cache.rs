//! Cache API JS-surface tests (window realm; slot `#11-cache-api-vm` /
//! D-19 PR-1).
//!
//! The async model mirrors IndexedDB: every `caches.*` / `Cache.*` op
//! settles its Promise from a `PendingTask::CacheDeliver` task drained at
//! the `drain_tasks` tail (DR-A.1), never inline.  `Vm::eval` runs that
//! drain after the top-level script returns, and a `.then` chain of cache
//! ops all drains within the SAME eval (each settle enqueues the next
//! op's task, which the drain loop picks up).  So the pattern is: one
//! `eval` whose terminal `.then` writes the resolved value into
//! `globalThis.__out`, then read `__out` in a second `eval`.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_min_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

struct UnbindOnDrop<'a>(&'a mut Vm);

impl Drop for UnbindOnDrop<'_> {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

fn with_vm<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vm) -> R,
{
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_min_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let guard = UnbindOnDrop(&mut vm);
    f(guard.0)
}

/// Eval `setup` (whose terminal `.then` must assign the resolved value to
/// `globalThis.__out`); the `.then` chain drains at the eval tail.  Then
/// read `String(globalThis.__out)`.  An unset `__out` (a hung chain)
/// surfaces as `"undefined"`.
fn drive_string(vm: &mut Vm, setup: &str) -> String {
    vm.eval(setup).unwrap();
    match vm.eval("String(globalThis.__out)").unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string __out, got {other:?}"),
    }
}

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?} for `{source}`"),
    }
}

// ---------------------------------------------------------------------------
// CacheStorage: open / has / delete / keys
// ---------------------------------------------------------------------------

#[test]
fn open_has_delete_keys_roundtrip() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            globalThis.__log = [];
            caches.open('v1')
              .then(() => caches.has('v1'))
              .then(h => __log.push('has=' + h))
              .then(() => caches.keys())
              .then(k => __log.push('keys=' + k.join(',')))
              .then(() => caches.delete('v1'))
              .then(d => __log.push('del=' + d))
              .then(() => caches.has('v1'))
              .then(h => __log.push('has2=' + h))
              .then(() => { globalThis.__out = __log.join('|'); });
            ",
        );
        assert_eq!(out, "has=true|keys=v1|del=true|has2=false");
    });
}

#[test]
fn caches_keys_creation_order() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('first')
              .then(() => caches.open('second'))
              .then(() => caches.open('third'))
              .then(() => caches.keys())
              .then(k => { globalThis.__out = k.join(','); });
            ",
        );
        assert_eq!(out, "first,second,third");
    });
}

#[test]
fn has_missing_cache_is_false() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            "caches.has('nope').then(h => { globalThis.__out = 'has=' + h; });",
        );
        assert_eq!(out, "has=false");
    });
}

// ---------------------------------------------------------------------------
// Cache: put / match
// ---------------------------------------------------------------------------

#[test]
fn put_then_match_returns_response_body() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://example.com/', new Response('hello world'))
                .then(() => c.match('https://example.com/')))
              .then(resp => resp.text())
              .then(t => { globalThis.__out = t; });
            ",
        );
        assert_eq!(out, "hello world");
    });
}

#[test]
fn match_status_and_headers_preserved() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://example.com/x',
                  new Response('body', { status: 201, statusText: 'Created',
                                         headers: { 'content-type': 'text/plain' } }))
                .then(() => c.match('https://example.com/x')))
              .then(resp => { globalThis.__out = resp.status + '|' + resp.statusText + '|'
                            + resp.headers.get('content-type'); });
            ",
        );
        assert_eq!(out, "201|Created|text/plain");
    });
}

#[test]
fn match_miss_returns_undefined() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.match('https://example.com/absent'))
              .then(r => { globalThis.__out = r === undefined ? 'undefined' : 'found'; });
            ",
        );
        assert_eq!(out, "undefined");
    });
}

#[test]
fn put_overwrites_existing_entry() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/', new Response('first'))
                .then(() => c.put('https://e.com/', new Response('second')))
                .then(() => c.match('https://e.com/')))
              .then(r => r.text())
              .then(t => { globalThis.__out = t; });
            ",
        );
        assert_eq!(out, "second");
    });
}

// ---------------------------------------------------------------------------
// CacheQueryOptions
// ---------------------------------------------------------------------------

#[test]
fn match_ignore_search() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/p?v=1', new Response('cached'))
                .then(() => c.match('https://e.com/p?v=2', { ignoreSearch: true })))
              .then(r => r ? r.text() : 'miss')
              .then(t => { globalThis.__out = t; });
            ",
        );
        assert_eq!(out, "cached");
    });
}

#[test]
fn match_without_ignore_search_misses_on_query() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/p?v=1', new Response('cached'))
                .then(() => c.match('https://e.com/p?v=2')))
              .then(r => { globalThis.__out = r === undefined ? 'miss' : 'hit'; });
            ",
        );
        assert_eq!(out, "miss");
    });
}

#[test]
fn match_all_returns_every_response() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/a', new Response('A'))
                .then(() => c.put('https://e.com/b', new Response('B')))
                .then(() => c.matchAll()))
              .then(list => Promise.all(list.map(r => r.text())))
              .then(texts => { globalThis.__out = texts.sort().join(','); });
            ",
        );
        assert_eq!(out, "A,B");
    });
}

#[test]
fn cache_keys_returns_request_objects() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/a', new Response('A'))
                .then(() => c.put('https://e.com/b', new Response('B')))
                .then(() => c.keys()))
              .then(reqs => { globalThis.__out =
                  reqs.map(r => r.method + ' ' + r.url).sort().join(','); });
            ",
        );
        assert_eq!(out, "GET https://e.com/a,GET https://e.com/b");
    });
}

// ---------------------------------------------------------------------------
// Vary
// ---------------------------------------------------------------------------

#[test]
fn vary_header_matches_on_same_request_header() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => {
                const req = new Request('https://e.com/data',
                                        { headers: { accept: 'application/json' } });
                const resp = new Response('json', { headers: { vary: 'Accept' } });
                return c.put(req, resp).then(() => {
                  const same = new Request('https://e.com/data',
                                           { headers: { accept: 'application/json' } });
                  const diff = new Request('https://e.com/data',
                                           { headers: { accept: 'text/html' } });
                  return Promise.all([c.match(same), c.match(diff)]);
                });
              })
              .then(([a, b]) => { globalThis.__out =
                  (a ? 'hit' : 'miss') + ',' + (b ? 'hit' : 'miss'); });
            ",
        );
        assert_eq!(out, "hit,miss");
    });
}

#[test]
fn put_rejects_vary_star() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/', new Response('x', { headers: { vary: '*' } })))
              .then(() => { globalThis.__out = 'resolved'; },
                    e => { globalThis.__out = 'rejected:' + (e instanceof TypeError); });
            ",
        );
        assert_eq!(out, "rejected:true");
    });
}

// ---------------------------------------------------------------------------
// put rejections (§5.4.5)
// ---------------------------------------------------------------------------

#[test]
fn put_rejects_non_get_request() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put(new Request('https://e.com/', { method: 'POST' }),
                               new Response('x')))
              .then(() => { globalThis.__out = 'resolved'; },
                    e => { globalThis.__out = 'rejected:' + (e instanceof TypeError); });
            ",
        );
        assert_eq!(out, "rejected:true");
    });
}

#[test]
fn put_rejects_206_partial() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/', new Response('x', { status: 206 })))
              .then(() => { globalThis.__out = 'resolved'; },
                    e => { globalThis.__out = 'rejected:' + (e instanceof TypeError); });
            ",
        );
        assert_eq!(out, "rejected:true");
    });
}

#[test]
fn put_rejects_disturbed_body() {
    // §5.4.5 step 2: a Response whose body was already consumed (`.text()`
    // disturbs synchronously at call time) cannot be cached.
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => {
                const r = new Response('hi');
                r.text();
                return c.put('https://e.com/', r);
              })
              .then(() => { globalThis.__out = 'resolved'; },
                    e => { globalThis.__out = 'rejected:' + (e instanceof TypeError); });
            ",
        );
        assert_eq!(out, "rejected:true");
    });
}

#[test]
fn match_requires_a_request_argument() {
    // 0 args to a required-`request` op is a WebIDL TypeError (rejected).
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.match())
              .then(() => { globalThis.__out = 'resolved'; },
                    e => { globalThis.__out = 'rejected:' + (e instanceof TypeError); });
            ",
        );
        assert_eq!(out, "rejected:true");
    });
}

#[test]
fn put_requires_a_request_argument() {
    // §5.4.5: `request` is WebIDL-required; `c.put()` (0 args) must reject
    // with a TypeError, not coerce a missing `undefined` into a URL.
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put())
              .then(() => { globalThis.__out = 'resolved'; },
                    e => { globalThis.__out = 'rejected:' + (e instanceof TypeError); });
            ",
        );
        assert_eq!(out, "rejected:true");
    });
}

#[test]
fn caches_match_cache_name_null_restricts_to_literal_null() {
    // WebIDL: `{ cacheName: null }` is *present* and coerces to the
    // `DOMString` "null" (not treated as absent), so the cross-cache search
    // is restricted to a cache literally named "null" — a miss here.  (If
    // null were treated as absent, every cache would be searched and the
    // entry in cache 'a' would hit.)
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('a')
              .then(a => a.put('https://e.com/x', new Response('from-a')))
              .then(() => caches.match('https://e.com/x', { cacheName: null }))
              .then(r => { globalThis.__out = r === undefined ? 'miss' : 'hit'; });
            ",
        );
        assert_eq!(out, "miss");
    });
}

#[test]
fn match_of_synthetic_response_has_empty_url() {
    // Fetch §2.2.6: a synthetic `new Response(...)` has `url === ''`; the
    // Cache must not synthesize the request URL into the matched response.
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/page', new Response('x'))
                .then(() => c.match('https://e.com/page')))
              .then(resp => { globalThis.__out = 'url=[' + resp.url + ']'; });
            ",
        );
        assert_eq!(out, "url=[]");
    });
}

// ---------------------------------------------------------------------------
// delete
// ---------------------------------------------------------------------------

#[test]
fn cache_delete_removes_entry() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => c.put('https://e.com/', new Response('x'))
                .then(() => c.delete('https://e.com/'))
                .then(d => c.match('https://e.com/').then(m =>
                  { globalThis.__out = 'del=' + d + ',match='
                      + (m === undefined ? 'undefined' : 'found'); })));
            ",
        );
        assert_eq!(out, "del=true,match=undefined");
    });
}

// ---------------------------------------------------------------------------
// CacheStorage.match (cross-cache)
// ---------------------------------------------------------------------------

#[test]
fn caches_match_searches_all_caches() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('a')
              .then(a => a.put('https://e.com/x', new Response('from-a')))
              .then(() => caches.open('b'))
              .then(b => b.put('https://e.com/y', new Response('from-b')))
              .then(() => caches.match('https://e.com/y'))
              .then(r => r ? r.text() : 'miss')
              .then(t => { globalThis.__out = t; });
            ",
        );
        assert_eq!(out, "from-b");
    });
}

#[test]
fn caches_match_with_cache_name_restricts() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('a')
              .then(a => a.put('https://e.com/x', new Response('from-a')))
              .then(() => caches.open('b'))
              .then(() => caches.match('https://e.com/x', { cacheName: 'b' }))
              .then(r => { globalThis.__out = r === undefined ? 'miss' : 'hit'; });
            ",
        );
        assert_eq!(out, "miss");
    });
}

// ---------------------------------------------------------------------------
// Interface surface
// ---------------------------------------------------------------------------

#[test]
fn cache_and_cache_storage_are_illegal_constructors() {
    with_vm(|vm| {
        assert!(eval_bool(
            vm,
            "(() => { try { new Cache(); return false; } catch (e) { return e instanceof TypeError; } })()",
        ));
        assert!(eval_bool(
            vm,
            "(() => { try { new CacheStorage(); return false; } catch (e) { return e instanceof TypeError; } })()",
        ));
    });
}

#[test]
fn caches_is_a_cache_storage_instance() {
    with_vm(|vm| {
        assert!(eval_bool(vm, "caches instanceof CacheStorage"));
        assert!(eval_bool(vm, "typeof caches.open === 'function'"));
        assert!(eval_bool(vm, "typeof caches.match === 'function'"));
    });
}

#[test]
fn open_returns_a_cache_instance() {
    with_vm(|vm| {
        let out = drive_string(
            vm,
            "caches.open('v1').then(c => { globalThis.__out = c instanceof Cache; });",
        );
        assert_eq!(out, "true");
    });
}

#[test]
fn add_and_add_all_are_absent_pending_fetch_integration() {
    // Deferred to slot `#11-cache-add-fetch-integration` (needs fetch-broker
    // continuation).  Honest absence beats a cache-corrupting stub.
    with_vm(|vm| {
        let out = drive_string(
            vm,
            r"
            caches.open('v1')
              .then(c => { globalThis.__out = (typeof c.add) + ',' + (typeof c.addAll); });
            ",
        );
        assert_eq!(out, "undefined,undefined");
    });
}
