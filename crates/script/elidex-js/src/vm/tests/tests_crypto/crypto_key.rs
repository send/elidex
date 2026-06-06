//! `CryptoKey` accessors (type / extractable / algorithm / usages),
//! the `[[algorithm]]` / `[[usages]]` §13.4 cached-object semantics,
//! the §18.4.4 step-5/6 getter-firing invariants, and the GC /
//! side-store pruning invariants for `crypto_key_states` /
//! `crypto_key_js_cache`.

use super::super::super::value::JsValue;
use super::super::super::Vm;
use super::{assert_typeerror, eval_err, eval_global_string};

#[test]
fn crypto_key_accessors() {
    // type / extractable / algorithm.name / algorithm.hash.name / usages.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-384'}, true, ['sign','verify']) \
           .then(k => { globalThis.r = [k.type, k.extractable, k.algorithm.name, \
                        k.algorithm.hash.name, k.usages.join(','), \
                        k.algorithm.length].join('|'); });";
    assert_eq!(
        eval_global_string(src, "r"),
        // SHA-384 HMAC default length = block size 1024 bits.
        "secret|true|HMAC|SHA-384|sign,verify|1024"
    );
}

#[test]
fn crypto_key_constructor_is_illegal() {
    let err = eval_err("new CryptoKey();");
    assert_typeerror(&err);
}

#[test]
fn import_cyclic_algorithm_object_does_not_recurse() {
    // C1 regression: a self-referential `hash` member must NOT recurse
    // (the nested `hash` is marshalled as a name-only leaf). It rejects
    // (`hash` "HMAC" is not a recognized digest), it does not crash.
    let src = "globalThis.r = 'pending'; \
         const a = {name:'HMAC'}; a.hash = a; \
         crypto.subtle.importKey('raw', new Uint8Array(20), a, true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn sign_does_not_read_hash_member_getter() {
    // C6 regression: sign's algorithm is name-only (the spec never reads
    // `hash`/`length` for sign), so a throwing `hash` getter must NOT fire.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.importKey('raw', new Uint8Array(20), {name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(k => crypto.subtle.sign( \
              {name:'HMAC', get hash(){ throw new Error('should not read'); }}, \
              k, new Uint8Array(1))) \
           .then(() => { globalThis.r = 'signed'; }, e => { globalThis.r = 'err:' + e.message; });";
    assert_eq!(eval_global_string(src, "r"), "signed");
}

#[test]
fn generate_key_unsupported_name_does_not_read_hash_getter() {
    // §18.4.4 step 5/6 ordering: an unregistered `(generateKey, name)`
    // pair is rejected as NotSupportedError at step 5 — *before* step 6's
    // params-dictionary conversion reads `hash` — so a throwing `hash`
    // getter on an unsupported algorithm must NOT fire.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey( \
            {name:'AES-Magic', get hash(){ throw new Error('should not read'); }}, \
            true, ['sign']) \
           .then(() => { globalThis.r = 'resolved'; }, e => { globalThis.r = e.name; });";
    assert_eq!(eval_global_string(src, "r"), "NotSupportedError");
}

#[test]
fn crypto_key_algorithm_and_usages_are_cached_objects() {
    // §13.4: the `algorithm` / `usages` getters return the *cached*
    // ECMAScript object (`[[algorithm_cached]]` / `[[usages_cached]]`), so
    // identity is stable across reads — `key.algorithm === key.algorithm`
    // and `key.usages === key.usages` are both `true`.
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(k => { globalThis.r = [k.algorithm === k.algorithm, \
                        k.usages === k.usages, \
                        k.algorithm.hash === k.algorithm.hash].join(','); });";
    assert_eq!(eval_global_string(src, "r"), "true,true,true");
}

#[test]
fn crypto_key_cached_algorithm_mutation_persists() {
    // A consequence of caching (§13.4): because the same object is
    // returned each read, a property written onto `key.algorithm` is
    // observable on the next read (it is not rebuilt fresh).
    let src = "globalThis.r = 'pending'; \
         crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(k => { k.algorithm.marker = 42; globalThis.r = String(k.algorithm.marker); });";
    assert_eq!(eval_global_string(src, "r"), "42");
}

#[test]
fn crypto_key_states_pruned_on_gc() {
    // I1 correctness invariant: a CryptoKey unreachable from any root is
    // pruned from `crypto_key_states` on collection (ObjectId slots are
    // reused, so a stale entry would bind another wrapper's material).
    use elidex_api_crypto::key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
    use elidex_api_crypto::HashAlgorithm;

    let mut vm = Vm::new();
    // Trigger global registration so `crypto_key_prototype` is set.
    vm.eval("void crypto.subtle;").unwrap();

    let data = CryptoKeyData {
        key_type: KeyType::Secret,
        extractable: true,
        algorithm: KeyAlgorithm::Hmac {
            hash: HashAlgorithm::Sha256,
            length: 160,
        },
        usages: vec![KeyUsage::Sign],
        material: KeyMaterial::Raw(vec![0xab; 20]),
    };
    let id = vm.inner.alloc_crypto_key(data);
    assert_eq!(vm.inner.crypto_key_states.len(), 1);

    // Root it via a global; GC keeps it.
    let key = vm.inner.strings.intern("rootedKey");
    vm.inner.globals.insert(key, JsValue::Object(id));
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.crypto_key_states.len(),
        1,
        "rooted key survives GC"
    );

    // Drop the only root; GC prunes the side-store entry.
    vm.inner.globals.insert(key, JsValue::Undefined);
    vm.inner.collect_garbage();
    assert_eq!(
        vm.inner.crypto_key_states.len(),
        0,
        "unreachable key pruned from side-store"
    );
}

#[test]
fn crypto_key_cached_algorithm_survives_gc_via_trace_arm() {
    // The cached `[[algorithm_cached]]` object (§13.4) is reachable ONLY
    // through `crypto_key_js_cache` after the callback returns — no JS var
    // holds it.  A GC with the key still rooted must keep it alive via the
    // `ObjectKind::CryptoKey` trace arm; otherwise the tagged property
    // would be lost (the getter would rebuild a fresh object).
    let mut vm = Vm::new();
    vm.eval(
        "crypto.subtle.generateKey({name:'HMAC', hash:'SHA-256'}, true, ['sign']) \
           .then(key => { globalThis.k = key; key.algorithm.marker = 7; });",
    )
    .unwrap();
    vm.inner.collect_garbage();
    let r = vm.eval("String(globalThis.k.algorithm.marker)").unwrap();
    match r {
        JsValue::String(id) => assert_eq!(vm.get_string(id), "7", "cached object survived GC"),
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn crypto_key_js_cache_pruned_on_gc() {
    // The `algorithm` / `usages` cache (`crypto_key_js_cache`) is pruned
    // alongside `crypto_key_states` when the key is collected — `ObjectId`
    // slots are reused, so a stale cache entry would alias another
    // wrapper's accessors.  Root the key directly via a global (not via a
    // settled `generateKey` Promise, whose `[[PromiseResult]]` would keep
    // the key reachable past the global drop).
    use elidex_api_crypto::key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
    use elidex_api_crypto::HashAlgorithm;

    let mut vm = Vm::new();
    // First eval registers the globals so `crypto_key_prototype` is set.
    vm.eval("void crypto.subtle;").unwrap();
    let id = vm.inner.alloc_crypto_key(CryptoKeyData {
        key_type: KeyType::Secret,
        extractable: true,
        algorithm: KeyAlgorithm::Hmac {
            hash: HashAlgorithm::Sha256,
            length: 160,
        },
        usages: vec![KeyUsage::Sign],
        material: KeyMaterial::Raw(vec![0xab; 20]),
    });
    let k = vm.inner.strings.intern("k");
    vm.inner.globals.insert(k, JsValue::Object(id));

    // Read both accessors (via JS, so the real getter populates the cache).
    vm.eval("void globalThis.k.algorithm; void globalThis.k.usages;")
        .unwrap();
    assert!(
        vm.inner.crypto_key_js_cache.contains_key(&id),
        "both accessors populated the cache"
    );

    // Rooted → cache entry survives.
    vm.inner.collect_garbage();
    assert!(
        vm.inner.crypto_key_js_cache.contains_key(&id),
        "cache survives while key rooted"
    );

    // Drop the root → cache + key state both pruned.
    vm.inner.globals.insert(k, JsValue::Undefined);
    vm.inner.collect_garbage();
    assert!(
        !vm.inner.crypto_key_js_cache.contains_key(&id),
        "cache pruned with collected key"
    );
    assert!(
        !vm.inner.crypto_key_states.contains_key(&id),
        "key state pruned with collected key"
    );
}

#[test]
fn crypto_key_accessor_with_missing_side_store_entry_is_illegal_invocation() {
    // Copilot #1 regression: a `CryptoKey` brand surviving WITHOUT its
    // side-store entry (e.g. a reference retained across `Vm::unbind`,
    // which clears the side-store) must surface as a TypeError, not a
    // panic / stale-material read.
    use elidex_api_crypto::key::{CryptoKeyData, KeyAlgorithm, KeyMaterial, KeyType, KeyUsage};
    use elidex_api_crypto::HashAlgorithm;

    let mut vm = Vm::new();
    vm.eval("void crypto.subtle;").unwrap();
    let data = CryptoKeyData {
        key_type: KeyType::Secret,
        extractable: true,
        algorithm: KeyAlgorithm::Hmac {
            hash: HashAlgorithm::Sha256,
            length: 160,
        },
        usages: vec![KeyUsage::Sign],
        material: KeyMaterial::Raw(vec![0xab; 20]),
    };
    let id = vm.inner.alloc_crypto_key(data);
    let key = vm.inner.strings.intern("k");
    vm.inner.globals.insert(key, JsValue::Object(id));
    // Simulate the invariant violation (entry gone, wrapper retained).
    vm.inner.crypto_key_states.remove(&id);

    let r = vm
        .eval("(() => { try { globalThis.k.type; return 'no-throw'; } catch (e) { return e.name; } })();")
        .unwrap();
    match r {
        JsValue::String(sid) => assert_eq!(vm.get_string(sid), "TypeError"),
        other => panic!("expected TypeError name, got {other:?}"),
    }
}
