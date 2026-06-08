//! Differential equivalence test for the spec-forced JWK mirror
//! (`#11-crypto-subtle-full`).
//!
//! `importKey('jwk', …)` reads the JWK from a **live JS object** via
//! [`super::marshal::marshal_jwk`] (page realm), while `unwrapKey('jwk', …)`
//! reads it from **JSON bytes** via [`elidex_api_crypto::jwk::from_json_bytes`]
//! (realm-isolated, WebCrypto §9 "a new global object").  The two
//! independently re-implement the WebIDL `JsonWebKey` dictionary conversion
//! over different substrates, so they MUST produce identical [`JsonWebKey`]
//! structs for the same logical JWK — else wrap↔import coercion / error
//! precedence diverge.
//!
//! This test pins them in lockstep across the member matrix.  It is the
//! forcing function the PR-4 EC members create: a developer who adds a member
//! to one reader but not the other (or coerces it differently) fails here.

#![cfg(all(test, feature = "engine"))]

use elidex_api_crypto::jwk::from_json_bytes;
use elidex_api_crypto::JsonWebKey;

use super::super::super::value::NativeContext;
use super::super::super::Vm;
use super::marshal::marshal_jwk;

/// Marshal `js_literal` (a JS object literal) via the live `marshal_jwk` and
/// `json` (the equivalent JSON text) via `from_json_bytes`, asserting the two
/// `JsonWebKey` structs are identical.
fn assert_mirror(js_literal: &str, json: &str) {
    let mut vm = Vm::new();
    let obj = vm
        .eval(&format!("({js_literal})"))
        .expect("the JWK object literal evaluates");
    let via_object: JsonWebKey = {
        let mut ctx = NativeContext::new_call(&mut vm.inner);
        marshal_jwk(&mut ctx, obj).expect("marshal_jwk converts the live object")
    };
    let via_bytes = from_json_bytes(json.as_bytes()).expect("from_json_bytes parses the JSON");
    assert_eq!(
        via_object, via_bytes,
        "JWK mirror diverged for object {js_literal} vs JSON {json}"
    );
}

#[test]
fn jwk_mirror_ec_public_all_curves() {
    for crv in ["P-256", "P-384", "P-521"] {
        assert_mirror(
            &format!("{{kty:'EC',crv:'{crv}',x:'eHh4',y:'eXl5'}}"),
            &format!(r#"{{"kty":"EC","crv":"{crv}","x":"eHh4","y":"eXl5"}}"#),
        );
    }
}

#[test]
fn jwk_mirror_ec_private_with_d_and_metadata() {
    assert_mirror(
        "{kty:'EC',crv:'P-256',x:'eHh4',y:'eXl5',d:'ZGRk',ext:true,key_ops:['sign']}",
        r#"{"kty":"EC","crv":"P-256","x":"eHh4","y":"eXl5","d":"ZGRk","ext":true,"key_ops":["sign"]}"#,
    );
}

#[test]
fn jwk_mirror_ec_array_coerced_members() {
    // An array member is `ToString`-ed (`['EC']` → "EC", `['P','256']` →
    // "P,256") identically by both halves.
    assert_mirror(
        "{kty:['EC'],crv:'P-256',x:['eHh4'],y:'eXl5'}",
        r#"{"kty":["EC"],"crv":"P-256","x":["eHh4"],"y":"eXl5"}"#,
    );
}

#[test]
fn jwk_mirror_ec_present_null_members() {
    // A present `null` (vs an absent member) coerces to "null" for DOMString
    // members and `false` for `ext` — identically in both halves.
    assert_mirror(
        "{kty:'EC',crv:'P-256',x:'eHh4',y:'eXl5',d:null,alg:null,ext:null}",
        r#"{"kty":"EC","crv":"P-256","x":"eHh4","y":"eXl5","d":null,"alg":null,"ext":null}"#,
    );
}

#[test]
fn jwk_mirror_retains_rsa_members_and_ignores_unknown_identically() {
    // RSA members (n / e / …) are retained by both halves (PR-5a); unknown
    // members are read (for getter side effects) but retained by neither — so
    // the structs still match.
    assert_mirror(
        "{kty:'EC',crv:'P-256',x:'eHh4',y:'eXl5',n:'big',e:'AQAB',unknown:'x'}",
        r#"{"kty":"EC","crv":"P-256","x":"eHh4","y":"eXl5","n":"big","e":"AQAB","unknown":"x"}"#,
    );
}

#[test]
fn jwk_mirror_rsa_public() {
    assert_mirror(
        "{kty:'RSA',n:'bbbb',e:'AQAB',alg:'RS256',ext:true,key_ops:['verify']}",
        r#"{"kty":"RSA","n":"bbbb","e":"AQAB","alg":"RS256","ext":true,"key_ops":["verify"]}"#,
    );
}

#[test]
fn jwk_mirror_rsa_private_all_crt_members() {
    // The full RSA private member set (n / e / d / p / q / dp / dq / qi) is
    // read + retained identically by the live and bytes halves.
    assert_mirror(
        "{kty:'RSA',n:'bbbb',e:'AQAB',d:'ZGRk',p:'cHA',q:'cXE',dp:'ZHA',dq:'ZHE',qi:'cWk',alg:'RS384'}",
        r#"{"kty":"RSA","n":"bbbb","e":"AQAB","d":"ZGRk","p":"cHA","q":"cXE","dp":"ZHA","dq":"ZHE","qi":"cWk","alg":"RS384"}"#,
    );
}

#[test]
fn jwk_mirror_rsa_oth_multiprime() {
    // The `oth` (otherPrimeInfos) sequence + its per-entry r / d / t members
    // mirror identically (multi-prime import is later rejected, but the
    // marshalled struct must still match for the wrap↔import lockstep).
    assert_mirror(
        "{kty:'RSA',n:'bbbb',e:'AQAB',d:'ZGRk',oth:[{r:'cjE',d:'ZDE',t:'dDE'},{r:'cjI'}]}",
        r#"{"kty":"RSA","n":"bbbb","e":"AQAB","d":"ZGRk","oth":[{"r":"cjE","d":"ZDE","t":"dDE"},{"r":"cjI"}]}"#,
    );
}

#[test]
fn jwk_mirror_oct_symmetric_unchanged() {
    // The pre-EC `oct` subset still mirrors (regression guard for the existing
    // HMAC / AES path).
    assert_mirror(
        "{kty:'oct',k:'a2V5',alg:'A256GCM',ext:true,key_ops:['encrypt','decrypt']}",
        r#"{"kty":"oct","k":"a2V5","alg":"A256GCM","ext":true,"key_ops":["encrypt","decrypt"]}"#,
    );
}
