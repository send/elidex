// ---------------------------------------------------------------------------
// digest (relocated from the VM host)
// ---------------------------------------------------------------------------

use super::to_hex;
use crate::hash::HashAlgorithm;

#[test]
fn digest_sha256_abc() {
    // FIPS 180-4 SHA-256("abc").
    assert_eq!(
        to_hex(&HashAlgorithm::Sha256.digest(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn digest_sha1_abc() {
    assert_eq!(
        to_hex(&HashAlgorithm::Sha1.digest(b"abc")),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
}

#[test]
fn digest_lengths() {
    assert_eq!(HashAlgorithm::Sha1.digest(b"").len(), 20);
    assert_eq!(HashAlgorithm::Sha256.digest(b"").len(), 32);
    assert_eq!(HashAlgorithm::Sha384.digest(b"").len(), 48);
    assert_eq!(HashAlgorithm::Sha512.digest(b"").len(), 64);
}
