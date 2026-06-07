//! AES-GCM / AES-CBC / AES-CTR encryption + decryption (WebCrypto §29
//! AES-GCM / §28 AES-CBC / §27 AES-CTR `Operations`).
//!
//! ## Why these are composed, not pulled from `aes-gcm` / `ctr`
//!
//! WebCrypto requires *runtime* flexibility the fixed-generic RustCrypto
//! AEAD/stream crates cannot express:
//!
//! - AES-GCM `tagLength` ∈ {32, 64, 96, 104, 112, 120, 128} bits and an
//!   `iv` of any length — `aes-gcm`'s `TagSize` (≥ 96 bits) and `NonceSize`
//!   are compile-time generics, so 32/64-bit tags and non-96-bit IVs are
//!   unreachable.
//! - AES-CTR `length` ∈ [1, 128] counter bits — the `ctr` crate only offers
//!   the fixed `Ctr32/64/128BE` counter widths.
//!
//! So this module composes the **vetted** RustCrypto primitives — the
//! `aes` block cipher and the `ghash` universal hash — into the cipher
//! modes per NIST SP 800-38A (CBC/CTR) and SP 800-38D (GCM), exactly as
//! `aes-gcm` does internally but with the spec's runtime sizing.  The
//! security-critical math (the block cipher, GHASH, constant-time tag
//! compare via `subtle`) stays in the audited crates; only the mode glue
//! is local, validated against the NIST/RFC known-answer vectors in
//! `tests.rs`.  AES-CBC uses the `cbc` crate directly (its `Encryptor` /
//! `Decryptor` carry the full mode + PKCS#7 padding).
//!
//! All entry points dispatch on the AES key byte length (16/24/32 →
//! AES-128/192/256); the length is validated upstream
//! (`ops::generate_key` / `ops::import_key` enforce 128/192/256), so a
//! wrong length here is an internal invariant violation, not a user error.

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{
    Block, BlockCipherEncrypt, BlockModeDecrypt, BlockModeEncrypt, KeyInit, KeyIvInit,
};
use aes::{Aes128, Aes192, Aes256};
use ghash::universal_hash::UniversalHash as _;
use ghash::GHash;
use subtle::ConstantTimeEq as _;

use crate::error::AlgorithmError;

/// The AES block size in bytes (128 bits).
const BLOCK: usize = 16;

/// The valid AES-GCM authentication-tag lengths in bits (WebCrypto §29.4.1
/// step 4 / §29.4.2 step 1).  All are byte-aligned, so truncation /
/// comparison is on whole octets.
const VALID_GCM_TAG_LENGTHS: [u32; 7] = [32, 64, 96, 104, 112, 120, 128];

fn operation(msg: impl Into<String>) -> AlgorithmError {
    AlgorithmError::Operation(msg.into())
}

// ===========================================================================
// AES-GCM (WebCrypto §29.4.1 / §29.4.2 — composed per NIST SP 800-38D)
// ===========================================================================

/// AES-GCM encrypt (§29.4.1): returns `ciphertext || tag`, the tag
/// truncated to `tag_length_bits`.  `tag_length_bits` must be one of the
/// valid GCM tag lengths (else `OperationError`, §29.4.1 step 4).
pub fn encrypt_gcm(
    key: &[u8],
    iv: &[u8],
    additional_data: &[u8],
    plaintext: &[u8],
    tag_length_bits: u32,
) -> Result<Vec<u8>, AlgorithmError> {
    // §29.4.1 step 1: plaintext longer than 2^39 - 256 bytes → OperationError.
    // (The IV / additionalData > 2^64-1 byte limits of steps 2-3 are
    // unreachable — a `Vec` cannot hold that many bytes.)
    const MAX_GCM_PLAINTEXT: u64 = (1u64 << 39) - 256;
    if plaintext.len() as u64 > MAX_GCM_PLAINTEXT {
        return Err(operation(
            "AES-GCM plaintext exceeds the maximum length (2^39 - 256 bytes)",
        ));
    }
    let tag_len = gcm_tag_len_bytes(tag_length_bits)?;
    let (mut ciphertext, full_tag) = match key.len() {
        16 => gcm_seal::<Aes128>(key, iv, additional_data, plaintext),
        24 => gcm_seal::<Aes192>(key, iv, additional_data, plaintext),
        32 => gcm_seal::<Aes256>(key, iv, additional_data, plaintext),
        _ => unreachable!("AES key length validated to 16/24/32 by ops"),
    };
    // §29.4.1 step 7: ciphertext = C | T, with T the leading `tagLength`
    // bits of the full 128-bit tag.
    ciphertext.extend_from_slice(&full_tag[..tag_len]);
    Ok(ciphertext)
}

/// AES-GCM decrypt (§29.4.2): splits the trailing `tag_length_bits` tag,
/// recomputes the tag over the ciphertext, and verifies it in constant
/// time.  `OperationError` on a bad `tagLength`, a too-short input, or a
/// tag mismatch ("the indication of inauthenticity").
pub fn decrypt_gcm(
    key: &[u8],
    iv: &[u8],
    additional_data: &[u8],
    ciphertext: &[u8],
    tag_length_bits: u32,
) -> Result<Vec<u8>, AlgorithmError> {
    let tag_len = gcm_tag_len_bytes(tag_length_bits)?;
    // §29.4.2 step 2: ciphertext shorter than the tag cannot carry one.
    if ciphertext.len() < tag_len {
        return Err(operation("AES-GCM ciphertext is shorter than the tag"));
    }
    let (actual_ct, provided_tag) = ciphertext.split_at(ciphertext.len() - tag_len);
    let plaintext = match key.len() {
        16 => gcm_open::<Aes128>(key, iv, additional_data, actual_ct, provided_tag),
        24 => gcm_open::<Aes192>(key, iv, additional_data, actual_ct, provided_tag),
        32 => gcm_open::<Aes256>(key, iv, additional_data, actual_ct, provided_tag),
        _ => unreachable!("AES key length validated to 16/24/32 by ops"),
    };
    // §29.4.2 step 8 authenticated-decryption "FAIL" → OperationError.
    plaintext.ok_or_else(|| operation("AES-GCM authentication tag mismatch"))
}

/// `tagLength` bits → tag byte count, rejecting an out-of-set value
/// (§29.4.1 step 4 / §29.4.2 step 1).
fn gcm_tag_len_bytes(tag_length_bits: u32) -> Result<usize, AlgorithmError> {
    if VALID_GCM_TAG_LENGTHS.contains(&tag_length_bits) {
        Ok((tag_length_bits / 8) as usize)
    } else {
        Err(operation(
            "AES-GCM tagLength must be one of 32, 64, 96, 104, 112, 120 or 128",
        ))
    }
}

/// GCM authenticated encryption: returns `(ciphertext, full_128bit_tag)`.
fn gcm_seal<C: BlockCipherEncrypt + KeyInit>(
    key: &[u8],
    iv: &[u8],
    aad: &[u8],
    plaintext: &[u8],
) -> (Vec<u8>, [u8; BLOCK]) {
    let cipher = new_cipher::<C>(key);
    let h = encrypt_block(&cipher, [0u8; BLOCK]); // H = E_K(0^128)
    let j0 = gcm_j0(&h, iv);
    let mut ciphertext = plaintext.to_vec();
    gctr(&cipher, inc32(j0), &mut ciphertext); // C = GCTR_K(inc32(J0), P)
    let tag = gcm_tag(&cipher, &h, &j0, aad, &ciphertext);
    (ciphertext, tag)
}

/// GCM authenticated decryption.  Recomputes the tag over the ciphertext
/// (GHASH is independent of the plaintext) and constant-time compares its
/// leading `provided_tag.len()` bytes against `provided_tag`; only on a
/// match is the ciphertext decrypted (don't process unauthenticated data).
/// `None` ⇒ the tag did not verify.
fn gcm_open<C: BlockCipherEncrypt + KeyInit>(
    key: &[u8],
    iv: &[u8],
    aad: &[u8],
    ciphertext: &[u8],
    provided_tag: &[u8],
) -> Option<Vec<u8>> {
    // The caller splits `provided_tag` as the trailing `tagLength/8` ≤ 16
    // bytes, so the leading-bytes slice below is in bounds.
    debug_assert!(provided_tag.len() <= BLOCK, "GCM tag is at most 16 bytes");
    let cipher = new_cipher::<C>(key);
    let h = encrypt_block(&cipher, [0u8; BLOCK]);
    let j0 = gcm_j0(&h, iv);
    let full_tag = gcm_tag(&cipher, &h, &j0, aad, ciphertext);
    // Verify the leading `tagLength` bits before decrypting (§29.4.2 step 8).
    if !bool::from(full_tag[..provided_tag.len()].ct_eq(provided_tag)) {
        return None;
    }
    let mut plaintext = ciphertext.to_vec();
    gctr(&cipher, inc32(j0), &mut plaintext);
    Some(plaintext)
}

/// Derive the GCM pre-counter block J0 (NIST SP 800-38D §7.1 step 2):
/// `IV || 0^31 || 1` for a 96-bit IV, else `GHASH_H(IV padded || [len(IV)])`.
fn gcm_j0(h: &[u8; BLOCK], iv: &[u8]) -> [u8; BLOCK] {
    if iv.len() == 12 {
        let mut j0 = [0u8; BLOCK];
        j0[..12].copy_from_slice(iv);
        j0[BLOCK - 1] = 1;
        j0
    } else {
        let hk: &ghash::Key = h.into();
        let mut gh = GHash::new(hk);
        gh.update_padded(iv);
        let mut len_block = [0u8; BLOCK];
        len_block[8..].copy_from_slice(&((iv.len() as u64) * 8).to_be_bytes());
        gh.update(core::slice::from_ref((&len_block).into()));
        gh.finalize().into()
    }
}

/// The full 128-bit GCM tag (NIST SP 800-38D §7.1 steps 5-6):
/// `E_K(J0) XOR GHASH_H(A || 0* || C || 0* || [len(A)] || [len(C)])`.
fn gcm_tag<C: BlockCipherEncrypt>(
    cipher: &C,
    h: &[u8; BLOCK],
    j0: &[u8; BLOCK],
    aad: &[u8],
    ciphertext: &[u8],
) -> [u8; BLOCK] {
    let hk: &ghash::Key = h.into();
    let mut gh = GHash::new(hk);
    gh.update_padded(aad);
    gh.update_padded(ciphertext);
    let mut len_block = [0u8; BLOCK];
    len_block[..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
    len_block[8..].copy_from_slice(&((ciphertext.len() as u64) * 8).to_be_bytes());
    gh.update(core::slice::from_ref((&len_block).into()));
    let s: [u8; BLOCK] = gh.finalize().into();
    let ek_j0 = encrypt_block(cipher, *j0);
    let mut tag = [0u8; BLOCK];
    for i in 0..BLOCK {
        tag[i] = ek_j0[i] ^ s[i];
    }
    tag
}

/// GCM counter mode (NIST SP 800-38D §6.5): XOR `data` in place with the
/// keystream `E_K(counter), E_K(inc32(counter)), …`, the rightmost 32 bits
/// of the counter block incrementing per block.
fn gctr<C: BlockCipherEncrypt>(cipher: &C, mut counter: [u8; BLOCK], data: &mut [u8]) {
    for chunk in data.chunks_mut(BLOCK) {
        let keystream = encrypt_block(cipher, counter);
        for (d, k) in chunk.iter_mut().zip(keystream.iter()) {
            *d ^= *k;
        }
        inc32_in_place(&mut counter);
    }
}

/// `inc32` (NIST SP 800-38D §6.2): increment the rightmost 32 bits of the
/// block as a big-endian integer, mod 2^32 (the upper 96 bits are fixed).
fn inc32(mut block: [u8; BLOCK]) -> [u8; BLOCK] {
    inc32_in_place(&mut block);
    block
}

fn inc32_in_place(block: &mut [u8; BLOCK]) {
    let next = u32::from_be_bytes([block[12], block[13], block[14], block[15]]).wrapping_add(1);
    block[12..].copy_from_slice(&next.to_be_bytes());
}

// ===========================================================================
// AES-CBC (WebCrypto §28.4.1 / §28.4.2 — `cbc` crate + PKCS#7)
// ===========================================================================

/// AES-CBC encrypt (§28.4.1): `iv` must be 16 bytes (else `OperationError`),
/// PKCS#7 padding (RFC 2315 §10.3) — always adds 1-16 padding bytes.
pub fn encrypt_cbc(key: &[u8], iv: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    require_iv_16(iv, "AES-CBC")?;
    Ok(match key.len() {
        16 => cbc::Encryptor::<Aes128>::new_from_slices(key, iv)
            .expect("validated key/iv length")
            .encrypt_padded_vec::<Pkcs7>(plaintext),
        24 => cbc::Encryptor::<Aes192>::new_from_slices(key, iv)
            .expect("validated key/iv length")
            .encrypt_padded_vec::<Pkcs7>(plaintext),
        32 => cbc::Encryptor::<Aes256>::new_from_slices(key, iv)
            .expect("validated key/iv length")
            .encrypt_padded_vec::<Pkcs7>(plaintext),
        _ => unreachable!("AES key length validated to 16/24/32 by ops"),
    })
}

/// AES-CBC decrypt (§28.4.2): `iv` 16 bytes; ciphertext non-zero and a
/// multiple of the block size (§28.4.2 step 2); invalid PKCS#7 padding
/// (§28.4.2 step 5) → `OperationError`.
pub fn decrypt_cbc(key: &[u8], iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, AlgorithmError> {
    require_iv_16(iv, "AES-CBC")?;
    // §28.4.2 step 2: a zero-length or non-block-aligned ciphertext cannot
    // be a valid PKCS#7-padded CBC output.
    if ciphertext.is_empty() || !ciphertext.len().is_multiple_of(BLOCK) {
        return Err(operation(
            "AES-CBC ciphertext length must be a non-zero multiple of 16 bytes",
        ));
    }
    let bad_padding = || operation("AES-CBC decryption failed (invalid padding)");
    match key.len() {
        16 => cbc::Decryptor::<Aes128>::new_from_slices(key, iv)
            .expect("validated key/iv length")
            .decrypt_padded_vec::<Pkcs7>(ciphertext)
            .map_err(|_| bad_padding()),
        24 => cbc::Decryptor::<Aes192>::new_from_slices(key, iv)
            .expect("validated key/iv length")
            .decrypt_padded_vec::<Pkcs7>(ciphertext)
            .map_err(|_| bad_padding()),
        32 => cbc::Decryptor::<Aes256>::new_from_slices(key, iv)
            .expect("validated key/iv length")
            .decrypt_padded_vec::<Pkcs7>(ciphertext)
            .map_err(|_| bad_padding()),
        _ => unreachable!("AES key length validated to 16/24/32 by ops"),
    }
}

fn require_iv_16(iv: &[u8], algo: &str) -> Result<(), AlgorithmError> {
    if iv.len() == BLOCK {
        Ok(())
    } else {
        Err(operation(format!("{algo} iv must be exactly 16 bytes")))
    }
}

// ===========================================================================
// AES-CTR (WebCrypto §27.7.1 / §27.7.2 — composed per NIST SP 800-38A)
// ===========================================================================

/// AES-CTR encrypt (§27.7.1).  CTR is symmetric, so decrypt is identical.
pub fn encrypt_ctr(
    key: &[u8],
    counter: &[u8],
    length_bits: u32,
    plaintext: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    ctr_xor(key, counter, length_bits, plaintext)
}

/// AES-CTR decrypt (§27.7.2) — the same keystream XOR as encrypt.
pub fn decrypt_ctr(
    key: &[u8],
    counter: &[u8],
    length_bits: u32,
    ciphertext: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    ctr_xor(key, counter, length_bits, ciphertext)
}

/// Core AES-CTR transform: validate the counter block (16 bytes) and the
/// counter width (`length_bits` ∈ [1, 128]) per §27.7.1/.2 steps 1-2, then
/// XOR the data with the AES keystream, incrementing only the rightmost
/// `length_bits` bits of the counter block.
fn ctr_xor(
    key: &[u8],
    counter: &[u8],
    length_bits: u32,
    data: &[u8],
) -> Result<Vec<u8>, AlgorithmError> {
    // §27.7.1/.2 step 1: the counter block is the AES block size.
    if counter.len() != BLOCK {
        return Err(operation("AES-CTR counter must be exactly 16 bytes"));
    }
    // §27.7.1/.2 step 2: the counter length is 1..=128 bits.
    if length_bits == 0 || length_bits > 128 {
        return Err(operation("AES-CTR length must be between 1 and 128 bits"));
    }
    // §27.7.1/.2 step 3 delegates to NIST SP 800-38A §6.5 with the App. B.1
    // incrementing function over the rightmost `length_bits` bits — period
    // 2^length_bits.  A message of more than 2^length_bits blocks wraps the
    // counter and reuses keystream under the same key (catastrophic; NIST
    // requires distinct counter blocks), so reject it rather than emit
    // insecure output.  The guard only bites narrow counter widths — for
    // length_bits ≥ 64 the capacity dwarfs any in-memory message.
    if length_bits < 128 {
        let num_blocks = data.len().div_ceil(BLOCK) as u128;
        if num_blocks > (1u128 << length_bits) {
            return Err(operation(
                "AES-CTR data is too large for the counter length (would reuse counter blocks)",
            ));
        }
    }
    let mut counter_block = [0u8; BLOCK];
    counter_block.copy_from_slice(counter);
    Ok(match key.len() {
        16 => ctr_apply::<Aes128>(key, counter_block, length_bits, data),
        24 => ctr_apply::<Aes192>(key, counter_block, length_bits, data),
        32 => ctr_apply::<Aes256>(key, counter_block, length_bits, data),
        _ => unreachable!("AES key length validated to 16/24/32 by ops"),
    })
}

fn ctr_apply<C: BlockCipherEncrypt + KeyInit>(
    key: &[u8],
    mut counter: [u8; BLOCK],
    length_bits: u32,
    data: &[u8],
) -> Vec<u8> {
    let cipher = new_cipher::<C>(key);
    let mut out = data.to_vec();
    for chunk in out.chunks_mut(BLOCK) {
        let keystream = encrypt_block(&cipher, counter);
        for (d, k) in chunk.iter_mut().zip(keystream.iter()) {
            *d ^= *k;
        }
        ctr_increment(&mut counter, length_bits);
    }
    out
}

/// Increment the rightmost `length_bits` bits of the 128-bit counter block
/// as a big-endian integer, modulo 2^`length_bits` (NIST SP 800-38A
/// Appendix B.1 with m = `length_bits`).  The upper `128 - length_bits`
/// bits are the nonce and are preserved on wrap.
fn ctr_increment(block: &mut [u8; BLOCK], length_bits: u32) {
    debug_assert!((1..=128).contains(&length_bits));
    let full_bytes = (length_bits / 8) as usize;
    let partial_bits = length_bits % 8;
    // Add 1 to the counter, propagating the carry across the full counter
    // bytes (block[15], [14], …): each iteration adds the carry-in (1) and
    // returns early once a byte does not overflow.
    for i in 0..full_bytes {
        let idx = BLOCK - 1 - i;
        let (next, overflow) = block[idx].overflowing_add(1);
        block[idx] = next;
        if !overflow {
            return; // no carry into the next byte
        }
    }
    // Reached only if every full counter byte overflowed (or there are
    // none): the +1 lands in the partial top byte's low `partial_bits` bits,
    // wrapping within them and preserving the high (nonce) bits.  A
    // multiple-of-8 width has no partial byte, so the final carry is
    // discarded (mod 2^length_bits).
    if partial_bits > 0 {
        let idx = BLOCK - 1 - full_bytes;
        let mask: u8 = (1u8 << partial_bits) - 1;
        let nonce_part = block[idx] & !mask;
        let counter_part = block[idx].wrapping_add(1) & mask;
        block[idx] = nonce_part | counter_part;
    }
}

// ===========================================================================
// Shared block-cipher helpers
// ===========================================================================

fn new_cipher<C: KeyInit>(key: &[u8]) -> C {
    C::new_from_slice(key).expect("AES key length validated to 16/24/32 by ops")
}

/// Encrypt a single 16-byte block with the AES block cipher.  Built via
/// `Block::<C>::default()` + `copy_from_slice` rather than `From<[u8; 16]>`
/// so it stays generic over the cipher without pinning `C::BlockSize` to a
/// `typenum` constant (AES's block size is 16, asserted in debug).
fn encrypt_block<C: BlockCipherEncrypt>(cipher: &C, input: [u8; BLOCK]) -> [u8; BLOCK] {
    let mut block = Block::<C>::default();
    debug_assert_eq!(block.len(), BLOCK, "AES block size is 16 bytes");
    block.copy_from_slice(&input);
    cipher.encrypt_block(&mut block);
    let mut out = [0u8; BLOCK];
    out.copy_from_slice(&block);
    out
}
