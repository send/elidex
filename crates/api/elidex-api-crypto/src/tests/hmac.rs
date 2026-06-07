// ---------------------------------------------------------------------------
// HMAC vectors (RFC 4231 TC1: key = 0x0b×20, data = "Hi There")
// ---------------------------------------------------------------------------

use super::to_hex;
use crate::error::AlgorithmError;
use crate::hash::HashAlgorithm;
use crate::hmac;

#[test]
fn hmac_rfc4231_tc1() {
    let key = vec![0x0b_u8; 20];
    let data = b"Hi There";
    assert_eq!(
        to_hex(&hmac::sign(HashAlgorithm::Sha256, &key, data)),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
    assert_eq!(
        to_hex(&hmac::sign(HashAlgorithm::Sha384, &key, data)),
        "afd03944d84895626b0825f4ab46907f15f9dadbe4101ec682aa034c7cebc59c\
         faea9ea9076ede7f4af152e8b2fa9cb6"
    );
    assert_eq!(
        to_hex(&hmac::sign(HashAlgorithm::Sha512, &key, data)),
        "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde\
         daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
    );
}

#[test]
fn hmac_sha1_rfc2202_tc1() {
    // RFC 2202 TC1: key = 0x0b×20, data = "Hi There".
    let key = vec![0x0b_u8; 20];
    assert_eq!(
        to_hex(&hmac::sign(HashAlgorithm::Sha1, &key, b"Hi There")),
        "b617318655057264e28bc0b6fb378c8ef146be00"
    );
}

#[test]
fn hmac_sha256_rfc4231_tc2() {
    // key = "Jefe", data = "what do ya want for nothing?"
    assert_eq!(
        to_hex(&hmac::sign(
            HashAlgorithm::Sha256,
            b"Jefe",
            b"what do ya want for nothing?"
        )),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

#[test]
fn hmac_verify_constant_time_paths() {
    let key = vec![0x0b_u8; 20];
    let data = b"Hi There";
    let mac = hmac::sign(HashAlgorithm::Sha256, &key, data);
    assert!(hmac::verify(HashAlgorithm::Sha256, &key, &mac, data));
    // Tampered last byte.
    let mut bad = mac.clone();
    bad[31] ^= 0x01;
    assert!(!hmac::verify(HashAlgorithm::Sha256, &key, &bad, data));
    // Wrong length signature.
    assert!(!hmac::verify(HashAlgorithm::Sha256, &key, &mac[..31], data));
}

#[test]
fn hmac_block_size_defaults() {
    assert_eq!(HashAlgorithm::Sha1.block_size_bits(), 512);
    assert_eq!(HashAlgorithm::Sha256.block_size_bits(), 512);
    assert_eq!(HashAlgorithm::Sha384.block_size_bits(), 1024);
    assert_eq!(HashAlgorithm::Sha512.block_size_bits(), 1024);
    assert_eq!(
        hmac::generate_key_byte_len(HashAlgorithm::Sha256, None).unwrap(),
        64
    );
    assert_eq!(
        hmac::generate_key_byte_len(HashAlgorithm::Sha512, None).unwrap(),
        128
    );
    assert_eq!(
        hmac::generate_key_byte_len(HashAlgorithm::Sha256, Some(100)).unwrap(),
        13 // ceil(100/8)
    );
    assert!(matches!(
        hmac::generate_key_byte_len(HashAlgorithm::Sha256, Some(0)),
        Err(AlgorithmError::Operation(_))
    ));
}
