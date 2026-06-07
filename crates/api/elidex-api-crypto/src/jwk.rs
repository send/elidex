//! JSON Web Key (WebCrypto §15 `JsonWebKey`, RFC 7517) — the `oct`
//! symmetric-key subset used by HMAC / AES import/export and the
//! `wrapKey` / `unwrapKey` JWK round-trip.
//!
//! For `importKey` / `exportKey` the VM marshals the live JS object into
//! [`JsonWebKey`] fields (no JSON parse — `keyData` arrives as a JS object).
//! For `wrapKey` / `unwrapKey` the JWK is serialized to / parsed from JSON
//! **bytes** here ([`to_json_bytes`] / [`from_json_bytes`]) — WebCrypto
//! §14.3.11 step 14 / §9 "parse a JWK" require the JSON representation to be
//! produced "in the context of a new global object", i.e. **isolated from the
//! page realm** (no page-defined `Object.prototype.toJSON`, no caller-mutated
//! prototypes).  Doing it in this engine-independent crate over the
//! `JsonWebKey` struct (never a JS object) is exactly that isolation, so a
//! page that pollutes `Object.prototype` cannot observe or hijack a wrap /
//! unwrap.  This module validates the `oct` shape and decodes `k` (base64url,
//! no padding, per RFC 7515 §2).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::algorithm::AesVariant;
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::key::{CryptoKeyData, KeyUsage};

/// A JSON Web Key (the members relevant to symmetric `oct` keys).
/// `None` means the member was absent in the source object.
///
/// `Serialize` covers the `wrapKey` JSON serialization ([`to_json_bytes`]):
/// absent members are omitted.  The `unwrapKey` parse ([`from_json_bytes`])
/// does **not** derive `Deserialize` — it converts a parsed
/// [`serde_json::Value`] per the WebIDL `JsonWebKey` dictionary rules so a
/// present explicit `null` is distinguished from an absent member (a derived
/// `Deserialize` would collapse both to `None`, diverging from the spec
/// `importKey` conversion of `ext: null` / `key_ops: null`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct JsonWebKey {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alg: Option<String>,
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_ops: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<bool>,
}

/// Serialize an `oct` [`JsonWebKey`] to JSON bytes for `wrapKey` (WebCrypto
/// §14.3.11 step 14, the `jwk` branch).
///
/// The serialization runs entirely over the Rust struct — never a JS object —
/// so it is isolated from the page realm (no `Object.prototype.toJSON` is
/// invoked, satisfying the "new global object" requirement).  A plain struct
/// of strings / bools / string sequences is always serializable, so this is
/// infallible.
///
/// The output is padded to a multiple of 8 bytes with trailing ASCII spaces.
/// §14.3.11 step 14's Note explicitly allows adapting a flexible key-format
/// serialization to the wrapping algorithm's size constraints ("JSON.stringify
/// is not normatively required"); AES-KW requires the wrapped payload be a
/// multiple of 64 bits (8 bytes), so the padding lets a `jwk` key be
/// AES-KW-wrapped (as browsers / WPT expect) instead of failing §30.3.1 step 1.
/// Trailing whitespace is valid JSON ignored by [`from_json_bytes`], so it is
/// harmless for the AES-GCM/CBC/CTR fallback (any length).
pub fn to_json_bytes(jwk: &JsonWebKey) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(jwk).expect("an oct JsonWebKey is always serializable");
    let rem = bytes.len() % AES_KW_BLOCK;
    if rem != 0 {
        bytes.resize(bytes.len() + (AES_KW_BLOCK - rem), b' ');
    }
    bytes
}

/// The AES-KW semiblock size in bytes (64 bits) — the wrap payload granularity
/// [`to_json_bytes`] pads to.
const AES_KW_BLOCK: usize = 8;

/// Parse JSON bytes into a [`JsonWebKey`] for `unwrapKey` (WebCrypto §9 "parse
/// a JWK", reached from §14.3.12 step 15).
///
/// The parse runs over the bytes directly — never via a JS object in the page
/// realm — so caller-mutated `Object.prototype` / `Array.prototype` cannot run
/// during the conversion (the "new global object" isolation).
///
/// After `JSON.parse`, the result is converted to the `JsonWebKey` IDL
/// dictionary: each member is read by **presence** (so an explicit `null` is a
/// *present* member, not an absent one) and converted per WebIDL — `DOMString`
/// members via ECMAScript `ToString` (an explicit `null` becomes `"null"`),
/// `ext` via `ToBoolean` (`null` → `false`), and `key_ops` as a
/// `sequence<DOMString>` (a present non-array, including `null`, is not a
/// sequence → `DataError`).  This matches the live `importKey` marshalling, so
/// a wrapped `{ "ext": null }` / `{ "key_ops": null }` is rejected here exactly
/// as the normal JWK import path rejects it (rather than silently accepted).
/// Unknown JWK members (the EC / RSA fields of a non-`oct` key) are ignored;
/// malformed JSON or a non-object document is a `DataError`.
pub fn from_json_bytes(bytes: &[u8]) -> Result<JsonWebKey, AlgorithmError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| data("unwrapped key data is not valid JSON"))?;
    // §9 step 5 / WebIDL §3.2.17 dictionary conversion of the `JSON.parse`
    // result: an object converts directly; `null` (and an array — an Object with
    // no named JWK members) converts to a dictionary with all members absent (no
    // error — the missing `kty` is caught at step 6 below); any other primitive
    // (string / number / boolean) is not an object → `TypeError`.
    let map = match value {
        Value::Object(map) => map,
        Value::Null | Value::Array(_) => Map::new(),
        Value::String(_) | Value::Number(_) | Value::Bool(_) => {
            return Err(AlgorithmError::Type(
                "unwrapped key data is not a JSON Web Key dictionary".to_string(),
            ));
        }
    };
    // Step 5 dictionary conversion: the only members that can raise a conversion
    // error for a JSON value are `key_ops` and `oth` (both `sequence`s); the
    // `DOMString` / `ext` members always convert.  They are converted in WebIDL
    // member order (`key_ops` before `oth`), matching the live `importKey`
    // marshaller, so the error precedence agrees.  The `oth` values are unused
    // for `oct` keys — only its validation matters.
    let key_ops = member_string_sequence(&map, "key_ops")?;
    validate_oth(&map)?;
    let jwk = JsonWebKey {
        kty: member_domstring(&map, "kty"),
        k: member_domstring(&map, "k"),
        alg: member_domstring(&map, "alg"),
        use_: member_domstring(&map, "use"),
        key_ops,
        ext: member_boolean(&map, "ext"),
    };
    // §9 "parse a JWK" step 6: the `kty` member must be defined (a present member,
    // even if its value is the empty string) — a `DataError` here, BEFORE the
    // §14.3.12 step-16 import, so the error precedence does not depend on the
    // requested import algorithm (e.g. an HKDF unwrap of a `kty`-less JWK rejects
    // with this DataError, not HKDF's later "jwk unsupported" NotSupportedError).
    if jwk.kty.is_none() {
        return Err(data("JWK 'kty' member is missing"));
    }
    Ok(jwk)
}

/// Convert (for validation only) the `oth` member
/// (`sequence<RsaOtherPrimesInfo>`), matching the live `importKey` marshaller:
/// absent → ok; a present non-array is not a sequence → `TypeError`; each entry
/// must be an object (a JS array is also an object, with no `d`/`r`/`t`) or
/// `null` (an empty dictionary) — a primitive entry is not a dictionary →
/// `TypeError`.  The `d`/`r`/`t` members are optional `DOMString`s that always
/// convert, so they need no per-entry check; the values are discarded.
fn validate_oth(map: &Map<String, Value>) -> Result<(), AlgorithmError> {
    let Some(oth) = map.get("oth") else {
        return Ok(());
    };
    let Value::Array(entries) = oth else {
        return Err(AlgorithmError::Type(
            "JWK 'oth' member is not a sequence".to_string(),
        ));
    };
    for entry in entries {
        match entry {
            Value::Null | Value::Object(_) | Value::Array(_) => {}
            Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                return Err(AlgorithmError::Type(
                    "JWK 'oth' entry is not an RsaOtherPrimesInfo dictionary".to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// Read a `DOMString` JWK member by presence (WebIDL): absent → `None`; a
/// present value (incl. explicit `null`) → its ECMAScript `ToString`.
fn member_domstring(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key).map(json_to_domstring)
}

/// Read the `boolean ext` JWK member by presence (WebIDL): absent → `None`; a
/// present value (incl. explicit `null` → `false`) → its ECMAScript `ToBoolean`.
fn member_boolean(map: &Map<String, Value>, key: &str) -> Option<bool> {
    map.get(key).map(json_to_boolean)
}

/// Read the `sequence<DOMString> key_ops` JWK member by presence (WebIDL):
/// absent → `None`; a present array → each element `ToString`-ed; a present
/// non-array is not a sequence → `TypeError`.
///
/// The `TypeError` (not `DataError`) matches the live `importKey` path: a JSON
/// `null` / string / number / boolean is not an `Object` (Web IDL §3.2.21
/// step 1) and a JSON object has no `@@iterator`, so the sequence conversion
/// throws a `TypeError` in both cases — exactly what
/// `webidl_sequence_to_vec` raises for the same inputs.
fn member_string_sequence(
    map: &Map<String, Value>,
    key: &str,
) -> Result<Option<Vec<String>>, AlgorithmError> {
    match map.get(key) {
        None => Ok(None),
        Some(Value::Array(items)) => Ok(Some(items.iter().map(json_to_domstring).collect())),
        Some(_) => Err(AlgorithmError::Type(
            "JWK 'key_ops' member is not a sequence".to_string(),
        )),
    }
}

/// ECMAScript `ToString` of a parsed JSON value (the WebIDL `DOMString`
/// conversion in "parse a JWK"), matching the live `importKey` marshalling's
/// `ToString` so an array / object member coerces identically (e.g.
/// `["oct"]` → `"oct"`) rather than failing on JSON text.
fn json_to_domstring(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        // JS parses every JSON number as an `f64` and `ToString`s the `f64`
        // value, so `1`, `1.0`, and `1.00` all stringify to `"1"`.
        // `serde_json::Number::to_string` would instead preserve the source
        // spelling (`"1.0"`), which would make e.g. `key_ops:[1, 1.0]` miss a
        // duplicate the WebIDL conversion catches — so go through `f64`.
        Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), |f| f.to_string()),
        Value::String(s) => s.clone(),
        // `Array.prototype.toString` = `join(",")`: each element is `ToString`-ed,
        // except `null` / `undefined` elements, which join renders as the empty
        // string (JSON has no `undefined`).
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                other => json_to_domstring(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        // A plain object (a `JSON.parse` result has the pristine
        // `Object.prototype` in the "new global object") stringifies to
        // `"[object Object]"`.
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// ECMAScript `ToBoolean` of a parsed JSON value (the WebIDL `boolean`
/// conversion in "parse a JWK"): `null` / `false` / `0` / `""` are falsy.
fn json_to_boolean(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) | Value::Array(_) | Value::Object(_) => true,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
    }
}

/// Validate an `oct` JWK for HMAC import and return the decoded key
/// material (WebCrypto §31 HMAC "Import Key", `jwk` branch).
///
/// All failures map to `DataError` per the HMAC jwk-import "throw a
/// DataError" branches.
pub fn import_oct_hmac(
    jwk: &JsonWebKey,
    hash: HashAlgorithm,
    extractable: bool,
    usages: &[KeyUsage],
) -> Result<Vec<u8>, AlgorithmError> {
    // kty must be "oct".
    if jwk.kty.as_deref() != Some("oct") {
        return Err(data("JWK 'kty' member must be 'oct' for HMAC"));
    }

    // k is required and must be valid base64url (no padding). A
    // zero-length decode is rejected by the caller (`ops::import_key`)
    // per the WebCrypto §31.6.4 shared "if length is zero, throw a
    // DataError" step, so it is not special-cased here.
    let Some(k) = jwk.k.as_deref() else {
        return Err(data("JWK 'k' member is missing"));
    };
    let material = URL_SAFE_NO_PAD
        .decode(k)
        .map_err(|_| data("JWK 'k' member is not valid base64url"))?;

    // alg, if present, must match the requested hash.
    if let Some(alg) = jwk.alg.as_deref() {
        if alg != hash.jwk_hmac_alg() {
            return Err(data("JWK 'alg' member does not match the requested hash"));
        }
    }

    // use, if present, must be "sig" for a signing/verifying key — but
    // only when usages is non-empty (WebCrypto §31.6.4 step 7).  With
    // empty usages the later generic empty-secret-usages SyntaxError
    // (§14.3.9) is the correct rejection, so this DataError must not
    // pre-empt it.
    if !usages.is_empty() {
        if let Some(use_) = jwk.use_.as_deref() {
            if use_ != "sig" {
                return Err(data("JWK 'use' member must be 'sig'"));
            }
        }
    }

    // key_ops, if present, must be a valid superset of the requested
    // usages with no duplicate entries.
    if let Some(key_ops) = &jwk.key_ops {
        validate_key_ops(key_ops, usages)?;
    }

    // ext false cannot satisfy an extractable=true import.
    if let Some(false) = jwk.ext {
        if extractable {
            return Err(data(
                "JWK 'ext' member is false but an extractable key was requested",
            ));
        }
    }

    Ok(material)
}

/// Serialize an HMAC `CryptoKey` to an `oct` JWK (WebCrypto §31 HMAC
/// "Export Key", `jwk` branch).
pub fn export_oct_hmac(key: &CryptoKeyData, hash: HashAlgorithm) -> JsonWebKey {
    JsonWebKey {
        kty: Some("oct".to_string()),
        k: Some(URL_SAFE_NO_PAD.encode(key.material.as_bytes())),
        alg: Some(hash.jwk_hmac_alg().to_string()),
        use_: None,
        key_ops: Some(key.usages.iter().map(|u| u.as_str().to_string()).collect()),
        ext: Some(key.extractable),
    }
}

/// Validate an `oct` JWK for AES import and return the decoded key
/// material (WebCrypto §27.7.4 / §28.4.4 / §29.4.4 `jwk` branch — the
/// three AES modes share the step shape; only the `alg` prefix differs by
/// `variant` + key length).  All failures map to `DataError`.
pub fn import_oct_aes(
    jwk: &JsonWebKey,
    variant: AesVariant,
    extractable: bool,
    usages: &[KeyUsage],
) -> Result<Vec<u8>, AlgorithmError> {
    // jwk substep 2: kty must be "oct".
    if jwk.kty.as_deref() != Some("oct") {
        return Err(data("JWK 'kty' member must be 'oct' for AES"));
    }
    // jwk substep 4: decode the k field (base64url, no padding).
    let Some(k) = jwk.k.as_deref() else {
        return Err(data("JWK 'k' member is missing"));
    };
    let material = URL_SAFE_NO_PAD
        .decode(k)
        .map_err(|_| data("JWK 'k' member is not valid base64url"))?;

    // jwk substep 5: the key length in bits must be 128/192/256, and a
    // present `alg` must match the variant's value for that length (e.g.
    // 256-bit AES-GCM → "A256GCM"); any other length is a DataError.
    let bits = bit_len(material.len())?;
    let Some(expected_alg) = variant.jwk_alg(bits) else {
        return Err(data("JWK 'k' must decode to a 128, 192 or 256-bit AES key"));
    };
    if let Some(alg) = jwk.alg.as_deref() {
        if alg != expected_alg {
            return Err(data(
                "JWK 'alg' member does not match the AES key length / mode",
            ));
        }
    }

    // jwk substep 6: a present `use` must be "enc" (AES is an encryption
    // key) — but only when usages is non-empty, so the later generic
    // empty-secret-usages SyntaxError (§14.3.9) is not pre-empted.
    if !usages.is_empty() {
        if let Some(use_) = jwk.use_.as_deref() {
            if use_ != "enc" {
                return Err(data("JWK 'use' member must be 'enc'"));
            }
        }
    }

    // jwk substep 7: a present `key_ops` must be a valid superset of the
    // requested usages.
    if let Some(key_ops) = &jwk.key_ops {
        validate_key_ops(key_ops, usages)?;
    }

    // jwk substep 8: ext=false cannot satisfy an extractable=true import.
    if let Some(false) = jwk.ext {
        if extractable {
            return Err(data(
                "JWK 'ext' member is false but an extractable key was requested",
            ));
        }
    }

    Ok(material)
}

/// Serialize an AES `CryptoKey` to an `oct` JWK (WebCrypto §27.7.5 /
/// §28.4.5 / §29.4.5 `jwk` branch).
pub fn export_oct_aes(key: &CryptoKeyData, variant: AesVariant, length_bits: u32) -> JsonWebKey {
    JsonWebKey {
        kty: Some("oct".to_string()),
        k: Some(URL_SAFE_NO_PAD.encode(key.material.as_bytes())),
        // length is 128/192/256 for any stored AES key, so `jwk_alg` is Some.
        alg: variant.jwk_alg(length_bits).map(str::to_string),
        use_: None,
        key_ops: Some(key.usages.iter().map(|u| u.as_str().to_string()).collect()),
        ext: Some(key.extractable),
    }
}

/// The bit length of a byte sequence, or `DataError` if it would overflow
/// `u32` (defensive — AES material is always ≤ 32 bytes).
fn bit_len(byte_len: usize) -> Result<u32, AlgorithmError> {
    u32::try_from(byte_len)
        .ok()
        .and_then(|n| n.checked_mul(8))
        .ok_or_else(|| data("JWK 'k' member is too large"))
}

/// `key_ops` must be a valid JWK key-operations array containing every
/// requested usage (WebCrypto §31.6.4 step 8).
///
/// Validity is per JWK [RFC 7517 §4.3]: entries are arbitrary strings
/// ("Other values MAY be used"), but duplicate values MUST NOT be present.
/// So this checks duplicates + the requested-usage superset at the
/// **string** level — extension / private operations (e.g. a custom
/// `"derive-foo"` alongside `"sign"`) are ignored, not rejected.
fn validate_key_ops(key_ops: &[String], usages: &[KeyUsage]) -> Result<(), AlgorithmError> {
    // RFC 7517 §4.3: no duplicate key operation values.
    for (i, op) in key_ops.iter().enumerate() {
        if key_ops[i + 1..].iter().any(|other| other == op) {
            return Err(data("JWK 'key_ops' member contains a duplicate entry"));
        }
    }
    // §31.6.4 step 8: key_ops must contain all requested usages.
    for usage in usages {
        if !key_ops.iter().any(|op| op == usage.as_str()) {
            return Err(data(
                "JWK 'key_ops' member is not a superset of the requested usages",
            ));
        }
    }
    Ok(())
}

fn data(msg: &str) -> AlgorithmError {
    AlgorithmError::Data(msg.to_string())
}
