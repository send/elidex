//! ECDSA known-answer tests from RFC 6979 Appendix A.2.5 (P-256), A.2.6
//! (P-384) and A.2.7 (P-521): the private key, message `"sample"` and the
//! deterministic `r‖s`, driven through the crate's public `sign` / `verify`
//! entry points (the same path WebCrypto uses).  They pin the RFC 6979
//! deterministic output and the digest-length behaviour.
//!
//! **New behaviour since ecdsa 0.17:** a digest shorter than the curve order
//! (P-384 + SHA-1 / SHA-256, P-521 + SHA-1 / SHA-256) is accepted, as WebCrypto
//! §23.7.1 imposes no minimum digest length.  Those cases are the
//! `short_digest` tests below.  For those pairs the RFC `r‖s` is only checked
//! to *verify* (ecdsa seeds RFC 6979 with the curve's digest, not the signing
//! hash, so our deterministic signature differs from the RFC's).

use super::no_rng;
use crate::algorithm::NamedCurve;
use crate::key::KeyUsage;
use crate::ops::{import_key, sign, verify, KeyData, KeyFormat};
use crate::{normalize, CryptoKeyData, JsonWebKey, NormalizedAlgorithm, Operation};
use base64::Engine;

// RFC 6979 A.2 vectors, P-256
const P256_D: &str = "C9AFA9D845BA75166B5C215767B1D6934E50C3DB36E89B127B8A622B120F6721";
const P256_UX: &str = "60FED4BA255A9D31C961EB74C6356D68C049B8923B61FA6CE669622E60F29FB6";
const P256_UY: &str = "7903FE1008B8BC99A41AE9E95628BC64F2F1B20C2D7E9F5177A3C294D4462299";
const P256_SHA256_RS: &str = "EFD48B2AACB6A8FD1140DD9CD45E81D69D2C877B56AAF991C34D0EA84EAF3716F7CB1C942D657C41D436C7A1B6E29F65F3E900DBB9AFF4064DC4AB2F843ACDA8";
// RFC 6979 A.2 vectors, P-384
const P384_D: &str = "6B9D3DAD2E1B8C1C05B19875B6659F4DE23C3B667BF297BA9AA47740787137D896D5724E4C70A825F872C9EA60D2EDF5";
const P384_UX: &str = "EC3A4E415B4E19A4568618029F427FA5DA9A8BC4AE92E02E06AAE5286B300C64DEF8F0EA9055866064A254515480BC13";
const P384_UY: &str = "8015D9B72D7D57244EA8EF9AC0C621896708A59367F9DFB9F54CA84B3F1C9DB1288B231C3AE0D4FE7344FD2533264720";
const P384_SHA1_RS: &str = "EC748D839243D6FBEF4FC5C4859A7DFFD7F3ABDDF72014540C16D73309834FA37B9BA002899F6FDA3A4A9386790D4EB2A3BCFA947BEEF4732BF247AC17F71676CB31A847B9FF0CBC9C9ED4C1A5B3FACF26F49CA031D4857570CCB5CA4424A443";
const P384_SHA256_RS: &str = "21B13D1E013C7FA1392D03C5F99AF8B30C570C6F98D4EA8E354B63A21D3DAA33BDE1E888E63355D92FA2B3C36D8FB2CDF3AA443FB107745BF4BD77CB3891674632068A10CA67E3D45DB2266FA7D1FEEBEFDC63ECCD1AC42EC0CB8668A4FA0AB0";
const P384_SHA384_RS: &str = "94EDBB92A5ECB8AAD4736E56C691916B3F88140666CE9FA73D64C4EA95AD133C81A648152E44ACF96E36DD1E80FABE4699EF4AEB15F178CEA1FE40DB2603138F130E740A19624526203B6351D0A3A94FA329C145786E679E7B82C71A38628AC8";
// RFC 6979 A.2 vectors, P-521
const P521_D: &str = "00FAD06DAA62BA3B25D2FB40133DA757205DE67F5BB0018FEE8C86E1B68C7E75CAA896EB32F1F47C70855836A6D16FCC1466F6D8FBEC67DB89EC0C08B0E996B83538";
const P521_UX: &str = "01894550D0785932E00EAA23B694F213F8C3121F86DC97A04E5A7167DB4E5BCD371123D46E45DB6B5D5370A7F20FB633155D38FFA16D2BD761DCAC474B9A2F5023A4";
const P521_UY: &str = "00493101C962CD4D2FDDF782285E64584139C2F91B47F87FF82354D6630F746A28A0DB25741B5B34A828008B22ACC23F924FAAFBD4D33F81EA66956DFEAA2BFDFCF5";
const P521_SHA1_RS: &str = "00343B6EC45728975EA5CBA6659BBB6062A5FF89EEA58BE3C80B619F322C87910FE092F7D45BB0F8EEE01ED3F20BABEC079D202AE677B243AB40B5431D497C55D75D00E7B0E675A9B24413D448B8CC119D2BF7B2D2DF032741C096634D6D65D0DBE3D5694625FB9E8104D3B842C1B0E2D0B98BEA19341E8676AEF66AE4EBA3D5475D5D16";
const P521_SHA256_RS: &str = "01511BB4D675114FE266FC4372B87682BAECC01D3CC62CF2303C92B3526012659D16876E25C7C1E57648F23B73564D67F61C6F14D527D54972810421E7D87589E1A7004A171143A83163D6DF460AAF61522695F207A58B95C0644D87E52AA1A347916E4F7A72930B1BC06DBE22CE3F58264AFD23704CBB63B29B931F7DE6C9D949A7ECFC";
const P521_SHA512_RS: &str = "00C328FAFCBD79DD77850370C46325D987CB525569FB63C5D3BC53950E6D4C5F174E25A1EE9017B5D450606ADD152B534931D7D4E8455CC91F9B15BF05EC36E377FA00617CCE7CF5064806C467F678D3B4080D6F1CC50AF26CA209417308281B68AF282623EAA63E5B5C0723D8B8C37FF0777B1A20F8CCB1DCCC43997F1EE0E44DA4A67A";

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn b64(hex: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(unhex(hex))
}

fn ecdsa_alg(op: Operation, curve: NamedCurve, hash: &str) -> NormalizedAlgorithm {
    let mut raw = crate::RawAlgorithm::from_name("ECDSA");
    raw.named_curve = Some(curve.as_str().to_string());
    raw.hash = Some(Box::new(crate::RawAlgorithm::from_name(hash)));
    normalize(op, raw).expect("ECDSA algorithm normalizes")
}

/// Import the RFC private key (JWK) and the matching public point (raw).
fn rfc_keys(curve: NamedCurve, d: &str, ux: &str, uy: &str) -> (CryptoKeyData, CryptoKeyData) {
    let jwk = JsonWebKey {
        kty: Some("EC".to_string()),
        crv: Some(curve.as_str().to_string()),
        x: Some(b64(ux)),
        y: Some(b64(uy)),
        d: Some(b64(d)),
        ..Default::default()
    };
    let import_alg = || {
        let mut raw = crate::RawAlgorithm::from_name("ECDSA");
        raw.named_curve = Some(curve.as_str().to_string());
        normalize(Operation::ImportKey, raw).expect("import algorithm")
    };
    let private = import_key(
        KeyFormat::Jwk,
        import_alg(),
        true,
        vec![KeyUsage::Sign],
        KeyData::Jwk(Box::new(jwk)),
    )
    .expect("RFC private key imports");
    let mut point = vec![0x04];
    point.extend(unhex(ux));
    point.extend(unhex(uy));
    let public = import_key(
        KeyFormat::Raw,
        import_alg(),
        true,
        vec![KeyUsage::Verify],
        KeyData::Raw(point),
    )
    .expect("RFC public key imports");
    (private, public)
}

fn check(
    curve: NamedCurve,
    hash: &str,
    d: &str,
    ux: &str,
    uy: &str,
    expected_rs: &str,
    rfc_nonce: bool,
) {
    let (private, public) = rfc_keys(curve, d, ux, uy);
    let msg = b"sample";
    let expected = unhex(expected_rs);
    assert_eq!(expected.len(), curve.signature_len(), "vector length");
    let got = sign(
        ecdsa_alg(Operation::Sign, curve, hash),
        &private,
        msg,
        no_rng,
    )
    .expect("sign");
    if rfc_nonce {
        assert_eq!(got, expected, "{} / {hash}: RFC 6979 r‖s", curve.as_str());
    } else {
        // ecdsa's RFC 6979 HMAC-DRBG runs over the *curve's* digest (SHA-256 /
        // SHA-384 / SHA-512), not the signing hash, so for a mismatched pair the
        // nonce — hence r‖s — differs from the RFC vector (still a valid,
        // deterministic ECDSA signature).  Pin that it is deterministic and
        // verifies instead of asserting RFC equality.
        assert_eq!(got.len(), expected.len());
        let again = sign(
            ecdsa_alg(Operation::Sign, curve, hash),
            &private,
            msg,
            no_rng,
        )
        .expect("sign");
        assert_eq!(got, again, "{} / {hash}: deterministic", curve.as_str());
        assert!(
            verify(
                ecdsa_alg(Operation::Verify, curve, hash),
                &public,
                &got,
                msg
            )
            .expect("verify"),
            "{} / {hash}: own signature verifies",
            curve.as_str()
        );
    }
    let vfy = |sig: &[u8]| {
        verify(ecdsa_alg(Operation::Verify, curve, hash), &public, sig, msg).expect("verify")
    };
    assert!(
        vfy(&expected),
        "{} / {hash}: RFC signature verifies",
        curve.as_str()
    );
    let mut tampered = expected;
    tampered[0] ^= 0x01;
    assert!(
        !vfy(&tampered),
        "{} / {hash}: tampered signature rejected",
        curve.as_str()
    );
}

#[test]
fn rfc6979_p256_sha256() {
    check(
        NamedCurve::P256,
        "SHA-256",
        P256_D,
        P256_UX,
        P256_UY,
        P256_SHA256_RS,
        true,
    );
}

#[test]
fn rfc6979_p384_sha384() {
    check(
        NamedCurve::P384,
        "SHA-384",
        P384_D,
        P384_UX,
        P384_UY,
        P384_SHA384_RS,
        true,
    );
}

#[test]
fn rfc6979_p384_sha256_short_digest() {
    // New behaviour since ecdsa 0.17: short digests are accepted per WebCrypto
    // §23.7.1 (no minimum digest length).
    check(
        NamedCurve::P384,
        "SHA-256",
        P384_D,
        P384_UX,
        P384_UY,
        P384_SHA256_RS,
        false,
    );
}

#[test]
fn rfc6979_p384_sha1_short_digest() {
    // New behaviour since ecdsa 0.17: short digests are accepted per WebCrypto
    // §23.7.1 (no minimum digest length).
    check(
        NamedCurve::P384,
        "SHA-1",
        P384_D,
        P384_UX,
        P384_UY,
        P384_SHA1_RS,
        false,
    );
}

#[test]
fn rfc6979_p521_sha512() {
    check(
        NamedCurve::P521,
        "SHA-512",
        P521_D,
        P521_UX,
        P521_UY,
        P521_SHA512_RS,
        true,
    );
}

#[test]
fn rfc6979_p521_sha256_short_digest() {
    // New behaviour since ecdsa 0.17: short digests are accepted per WebCrypto
    // §23.7.1 (no minimum digest length).
    check(
        NamedCurve::P521,
        "SHA-256",
        P521_D,
        P521_UX,
        P521_UY,
        P521_SHA256_RS,
        false,
    );
}

#[test]
fn rfc6979_p521_sha1_short_digest() {
    // New behaviour since ecdsa 0.17: short digests are accepted per WebCrypto
    // §23.7.1 (no minimum digest length).
    check(
        NamedCurve::P521,
        "SHA-1",
        P521_D,
        P521_UX,
        P521_UY,
        P521_SHA1_RS,
        false,
    );
}
