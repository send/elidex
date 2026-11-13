# `#11-webcrypto-rsa-keygen-param-validation` — WebCrypto §20.5 at Generate Key step 2 (small plan-memo)

Status: **plan (pre-implementation)**. Owner: CI/tools (PM 706). Due: 2026-11-22. Not edge-dense (PM 706),
so no `/elidex-plan-review`; the push gate is the full `/pre-push`. Base: `06bdfbbd`; every `file:line`
below is at that commit. Paths are relative to `crates/api/elidex-api-crypto/src/` unless they start with
`crates/` or `docs/`.

**The defect, stated as a defect.** WebCrypto §20.5 "RSA key generation parameter validation" is not
implemented. `generateKey` for RSASSA-PKCS1-v1_5, RSA-PSS and RSA-OAEP passes `publicExponent` straight to
`rsa::RsaPrivateKey::new_with_exp` (`rsa.rs:522-525`). With an even exponent the rsa crate's prime search
never finds a prime `p` with `gcd(e, p − 1) = 1`, so the call does not return: at modulusLength 1024, the
exponents 2, 4 and 65536 did not return within 90 s (measured 2026-11-13, debug build, direct
`rsa::generate`). The keygen runs on the VM thread, so this is a hang that any script can trigger. The other
§20.5 cases (modulusLength 0, 3; modulusLength 16 with e = 65537; exponent 1, which at 2048 first runs
the full prime search, 11 s in a debug build, F3) end in OperationError from the rsa crate, but only by
accident of that crate. §20.5 alone is not the whole fix: an odd exponent with a tiny modulusLength hangs the
same crate loop (D6). Two comments state the opposite of
the spec ("WebCrypto does not constrain its value", `rsa.rs:479-481`; "the exponent value are the
rsa-crate's OperationError … honored as-is", `algorithm/normalize.rs:118-119`), each with a
`⚠ Contradicts WebCrypto §20.5 step 3` pointer to this slot (`rsa.rs:482-483`, `algorithm/normalize.rs:120-121`).

## §1. Spec (webref, webcrypto-2)

Each pair below was looked up with `.claude/tools/webref heading webcrypto-2 <n>` and the prose with
`.claude/tools/webref body webcrypto-2 <anchor>`.

| § = title | anchor | what this PR uses |
|---|---|---|
| §20.5 = RSA key generation parameter validation | `#rsa-key-generation-parameter-validation` | Steps 1–3. Step 3: if modulusLength is less than 4, or publicExponent is less than 3, is even, or is greater than or equal to 2^modulusLength − 1, throw an OperationError. |
| §20.9.3 = Generate Key (RSASSA-PKCS1-v1_5) | `#rsassa-pkcs1-operations-generate-key` | Step 1 usages SyntaxError; step 2 "Perform the validate RSA key generation parameters algorithm with normalizedAlgorithm"; steps 3–4 generate, failure → OperationError. |
| §21.4.3 = Generate Key (RSA-PSS) | `#rsa-pss-operations-generate-key` | Same steps 1–4. |
| §22.4.3 = Generate Key (RSA-OAEP) | `#rsa-oaep-operations-generate-key` | Same steps 1–4 (OAEP usage set in step 1). |
| §16 = BigInteger | `#big-integer` | Unsigned big-endian. Input may have any number of leading zero bits, "including the empty array, which represents zero". This defines §20.5 step 2's "converting … to a non-negative integer". |
| §14.3.6 = The generateKey method | `#SubtleCrypto-method-generateKey` | Step 2 normalizes; step 8 runs the generate key operation; after it, a CryptoKeyPair whose privateKey has empty usages is a SyntaxError. |

## §2. Decisions

**D1. Where the validation lives: a new submodule `rsa/keygen_params.rs` holding one function,
`pub(crate) fn validate_rsa_keygen_params(modulus_length: u32, public_exponent: &[u8]) -> Result<(), AlgorithmError>`,
called once from `rsa::generate`.** Reason: §20.5 is an algorithm of the RSA family chapter, invoked from
each variant's Generate Key step 2, so it belongs to the `rsa` module next to `generate`, the one function
all three variants already share (`ops.rs:99-112` is the only call site of `rsa::generate`); one function
called from that one place is the single canonical form.
- Not `normalize`: normalization runs at §14.3.6 step 2, before Generate Key step 1, so a check there would
  report the OperationError ahead of the usages SyntaxError (an observable inversion), and it would run for
  a step that is not normalization.
- Not the `algorithm` module: it holds the dictionary model and normalization, not operation steps.
- A submodule, not inline in `rsa.rs`: `rsa.rs` is 896 lines; the function plus its doc is about 50
  lines, a self-contained unit with no dependency on `rsa.rs` internals beyond the `operation` message
  helper, and it follows the existing `rsa/oaep.rs` / `rsa/signing.rs` pattern. `rsa.rs` itself changes by
  the call, the `mod` / `use` lines and the doc rewrite (§4), so it stays well under 1000 and needs no split.
- The function takes the two members, not a `NormalizedAlgorithm`: `generate` already receives them
  destructured (`ops.rs:99-112`), and the spec's step 1–2 only read those two members.
- Engine-independent: pure byte arithmetic in `elidex-api-crypto`, no rsa-crate type, no VM type.

**D2. Order: usages SyntaxError (step 1) → §20.5 (step 2) → the D6 floor and exponent cap → the 8192 generate ceiling and the OAEP range →
`new_with_exp` (steps 3–4) → the §14.3.6 empty-private-usages SyntaxError.** The call is inserted at
`rsa.rs:499`, between `validate_generate_usages` (`:498`) and the ceiling (`:507`). Reason: follow the spec
step order wherever it is observable. The ceiling and the OAEP range are elidex's own limits on step 3
(key generation that the engine refuses to attempt or whose key would be unusable), so they report a
step-4 failure and belong after step 2.
- What changes in error *type*: nothing observable that was not already wrong. Every §20.5 failure is an
  OperationError, as are the ceiling, the OAEP range and the rsa-crate failure; the usages SyntaxError
  stays first; the empty-private-usages SyntaxError stays after the operation (§14.3.6), so
  `usages = ['verify']` with an even exponent is an OperationError (it was a hang).
- What changes in *message only*: an input that fails both §20.5 and a later limit now carries the §20.5
  message. Examples: modulusLength 16384 with an even exponent (was the ceiling message), RSA-OAEP 1024
  with an even exponent (was the OAEP range message), exponent 1 or modulusLength 3 (was "RSA key
  generation failed" from the rsa crate).
- Inside step 3 the four conditions form one "or" with one error type, so their order is not observable;
  the function tests them as modulusLength < 4, e < 3, e even, e ≥ 2^ml − 1, and each has its own message
  naming the condition (`modulusLength` / `publicExponent`).

**D3. "e ≥ 2^modulusLength − 1" by bit length, without building either number.** Reason: modulusLength is
script-controlled up to `u32::MAX` (the ceiling check comes after §20.5, per D2), so `2^ml − 1` must never
be materialized; the exponent is already in memory as input bytes, so a scan of it is bounded by the input.
- Conversion (§20.5 step 2 with §16): strip leading zero bytes; the remaining slice `e` is the minimal
  big-endian encoding; an empty slice (empty input or all zeros) is 0. No `BigUint` is built (the existing
  `BigUint::from_bytes_be` at `rsa.rs:522` stays for the rsa crate call). The existing code passes the input
  bytes unchanged and the key's `[[algorithm]]` keeps them byte-identical (`rsa.rs` step-8 comment,
  `generate_preserves_non_minimal_public_exponent`); the validation reads the stripped view only and does
  not change what is stored.
- e < 3: `e` is empty, or `e.len() == 1 && e[0] < 3`.
- even: `e[last] & 1 == 0` (reached only with `e` non-empty).
- `bits(e) = 8 · e.len() − e[0].leading_zeros()`, computed in `u64` so that a long input cannot overflow.
  `2^ml − 1` is exactly `ml` one-bits, so e ≥ 2^ml − 1 ⟺ `bits(e) > ml`, or `bits(e) == ml` and every bit
  of `e` is 1 (`e[0] == 0xFF >> e[0].leading_zeros()` and `e[1..]` all `0xFF`).
- Cost: O(input length), no allocation.

**D4. Tests: crate unit tests in `src/tests/rsa/keygen.rs` (197 lines now, about 330 after), plus one
VM-level JS test in `crates/script/elidex-js/src/vm/tests/tests_crypto/rsa.rs` (364 lines now).** Neither
file approaches the 1000-line threshold. Reason for the VM test: the defect report is "script-reachable",
and only a VM test shows that the bytes a script passes reach §20.5 through the marshaller
(`crates/script/elidex-js/src/vm/host/subtle_crypto/marshal/params.rs`) and come back as a DOMException
named `OperationError` (`crates/script/elidex-js/src/vm/host/subtle_crypto/ops.rs:74-91` maps
`AlgorithmError::Operation`).
- The hang guard is the crate test, not the VM test. The crate tests that reach `generate` with an input
  §20.5 rejects pass a `fill_random` that **panics** on its first call. If the validation regresses, the rsa
  crate's prime search calls the RNG and the test fails at once with that panic instead of hanging.
  It must panic, not return `Err`: `ClosureRng` latches an error and continues with its SplitMix64 fallback
  stream (`rng.rs`), so an `Err` seam would let the prime search loop on.
- The VM test cannot inject the RNG, so it must not use an input that hangs pre-fix. It uses
  modulusLength 3 with e = 3 (`new Uint8Array([3])`, RSA-PSS), which fails fast both before and after the
  fix (F3), and tells them apart by the message: it asserts `e.name === 'OperationError'` and that
  `e.message` contains `modulusLength` (the §20.5 message), which the rsa crate's "RSA key generation
  failed" does not. nextest's `terminate-after` (8 × 60 s, `.config/nextest.toml`) stays a backstop only;
  no test in this PR relies on it.

**D5. Each new test has a named mutation that must turn it red (§3.3); the mutations are run once, on
the final head, in a debug build, and the result goes in the PR body.**

**D6. Two capability bounds in `generate`, after §20.5 (PM 726).** Measured 2026-11-13 (debug build, base
`06bdfbbd` + `4e91e0ab`): modulusLength 12 with e = 39 (RSASSA-PKCS1-v1_5 and RSA-PSS) does not return within
60 s, although §20.5 accepts it (odd, ≥ 3, < 2^12 − 1). Controls: ml 12 / e = 3 generates in 0.26 ms,
ml 64 / e = 65537 in 2.2 ms. Cause: only three 6-bit candidate primes exist (top two bits set: 53, 59, 61), every
φ shares a factor with 39 = 3·13, and the rsa crate's `generate_multi_prime_key_with_exp` `'next: loop` never ends;
the crate checks `MAX_PUB_EXPONENT` (2^33 − 1) only after that loop. RSA-OAEP is unaffected (its range is ≥ 2048).
- Derivation: with e ≤ 2^33 − 1, e has at most 9 distinct odd prime factors (3·5·…·29 < 2^33 < 3·5·…·31); a b-bit
  prime p with p − 1 = 2q (q prime, q > 2^33) gives gcd(e, p − 1) = 1 for every such e; such primes exist among the
  candidates when b ≥ 35, so the loop ends with probability 1; each prime is modulusLength / 2 bits, so
  modulusLength ≥ 2·35 = 70.
- Decision: `MIN_RSA_GENERATE_MODULUS_BITS = 70` (unrounded: a rounded number has no derivation) and an exponent
  cap at the crate's `RsaPublicKey::MAX_PUB_EXPONENT`, compared on the bytes (no `BigUint`), both OperationError
  (Generate Key's "generation fails" path, like the 8192 ceiling), in `rsa/keygen_params.rs`
  `validate_rsa_generate_capability`, called right after §20.5.
- Rejected: a floor of 256 (a browser's value, not a derivation); no floor (the hang stays).
- (The D5 above is the mutation-run decision; this one is D6 because D5 was taken.)

## §3. Tests

Helper: `keygen_alg` (`tests/rsa/mod.rs:48`) fixes the exponent at `[1, 0, 1]`; add a sibling that takes
the exponent bytes (or give `keygen_alg` an exponent parameter and update its callers), and a
`panicking_fill` that panics with "rsa crate reached".

### §3.1 Boundary table — `validate_rsa_keygen_params` directly (DoD (i))

Each failing row trips exactly one condition, and each condition has a row on each side of its boundary.

| # | modulusLength | publicExponent bytes | value | expect | condition exercised |
|---|---|---|---|---|---|
| B1 | 3 | `[3]` | 3 | Err | ml < 4 (3 < 2^3 − 1 = 7 is false, so only this) |
| B2 | 4 | `[3]` | 3 | Ok | ml < 4, pass side (3 < 15) |
| B3 | 2048 | `[1]` | 1 | Err | e < 3 (odd, so only this) |
| B4 | 2048 | `[3]` | 3 | Ok | e < 3, pass side |
| B5 | 2048 | `[]` | 0 | Err | empty array = 0 (§16) |
| B6 | 2048 | `[0, 0]` | 0 | Err | all-zero encoding = 0 |
| B7 | 2048 | `[1, 0, 0]` | 65536 | Err | even (only this) |
| B8 | 2048 | `[1, 0, 1]` | 65537 | Ok | even, pass side |
| B9 | 16 | `[0xFF, 0xFF]` | 65535 = 2^16 − 1 | Err | e ≥ 2^ml − 1, equality (bits == ml, all ones) |
| B10 | 16 | `[0xFF, 0xFD]` | 65533 | Ok | bits == ml, not all ones |
| B11 | 16 | `[1, 0, 1]` | 65537 | Err | bits(e) = 17 > ml |
| B12 | 17 | `[1, 0, 1]` | 65537 | Ok | bits(e) < ml |
| B13 | 8 | `[0, 0, 0xFF]` | 255 = 2^8 − 1 | Err | leading zeros stripped before the equality test |
| B14 | 17 | `[0, 0, 1, 0, 1]` | 65537 | Ok | leading zeros not counted as bits |
| B15 | `u32::MAX` | `[1, 0, 1]` | 65537 | Ok | no allocation at the largest ml (returns at once) |

### §3.1b Capability table — `validate_rsa_generate_capability` directly

| # | ml | exponent | expect |
|---|---|---|---|
| C1 | 69 | 65537 | Err (`modulusLength`) |
| C2 | 70 | 65537 | Ok |
| C3 | 70 | 2^33 − 1 = `[1,FF,FF,FF,FF]` | Ok (also accepted by §20.5) |
| C4 | 70 | 2^33 + 1 = `[2,0,0,0,1]` | Err (`publicExponent`; odd, so §20.5 does not stop it first) |
| C5 | 70 | 2^33 − 1 with 4 leading zero bytes (9 bytes) | Ok (strip before the length test) |
| C6 | 70 | 9-byte value above 2^64 | Err |
| C7 | `u32::MAX` | 65537 | Ok |

The boundary tables carry an expected-message column (Ok, or the substring the OperationError must contain).

### §3.2 Through `generate_key` (DoD (ii), (iii)) and the VM

`generate_rejects_before_the_rsa_crate`: one table over the panic seam; each row's error must equal the
function's own error for that input (`validate_rsa_keygen_params` or `validate_rsa_generate_capability`):
G1 (1024, e = 2), G2–G4 (even exponent, RSASSA / PSS / OAEP at 2048), G6 (OAEP 1024, §20.5 before the OAEP
range), G6b (16384, §20.5 before the ceiling), G8 (ml 3, §20.5 before the floor), G9 / G10 (ml 12, e = 39,
RSASSA / PSS: the D6 regression), G11 (ml 69), G12 (e = 2^33 + 1), G13 / G14 (cap before the ceiling / the OAEP
range). `generate_check_order_kinds`: G5 (usages SyntaxError before §20.5), G6c (empty private usages is
OperationError, §14.3.6 is after the operation), G6d (same at ml 12 / e = 39).
G7 (VM, RSA-PSS, ml 3, `[3]`): rejects with `OperationError` and a message containing `modulusLength` (D4; F3).
Existing tests use 65537 (or non-minimal `[0,1,0,1]`) at ml 512 / 1024 / 2048 / 16384, which both checks accept.

### §3.3 Mutations (D5)

Run on the final head (debug build); "red" = the named test fails.

| # | mutation | red test(s) |
|---|---|---|
| M1 | delete `ml < 4` | `keygen_params_boundary_table` (B1), `generate_rejects_before_the_rsa_crate` (G8) |
| M2 | `ml < 4` → `ml <= 4` | boundary table (B2) |
| M3 | delete the whole `e < 3` condition | boundary table (B3) |
| M3b | delete only `e.len() == 1 && e[0] < 3` | boundary table (B3) |
| M4 | `e[0] < 3` → `e[0] <= 3` | boundary table (B2, via e = 3) |
| M5 | delete the "even" condition | boundary table, `generate_rejects_before_the_rsa_crate` (panic seam), `generate_check_order_kinds` |
| M6 | drop the all-ones equality branch | boundary table (B9) |
| M7 | no leading-zero strip in §20.5 | boundary table, `generate_preserves_non_minimal_public_exponent` (both by panic: shift overflow on a zero lead byte) |
| M8 | `bits == ml` always fails | boundary table (B10) |
| M9 | delete the §20.5 call | `generate_rejects_before_the_rsa_crate`, `generate_check_order_kinds` (panic seam). G7 stays green (the floor's message also contains `modulusLength`) |
| M9c | delete the §20.5 call and the capability call | same two (panic seam), and G7 (VM) |
| M10 | §20.5 call before the usages check | `generate_check_order_kinds` (G5) |
| M11 | §20.5 call after the ceiling / OAEP range | `generate_rejects_before_the_rsa_crate` (G6) |
| M12 | floor 70 → 69 | `keygen_capability_boundary_table` (C1), `generate_rejects_before_the_rsa_crate` (G11) |
| M13 | drop the floor check | capability table (C1), `generate_rejects_before_the_rsa_crate` (G9), `generate_check_order_kinds` |
| M14 | cap `>` → `>=` | capability table (C3) |
| M15 | drop the cap check | capability table (C4), `generate_rejects_before_the_rsa_crate` (G12) |
| M16 | no leading-zero strip in the cap | capability table (C5) |
| M17 | delete the capability call | `generate_rejects_before_the_rsa_crate`, `generate_check_order_kinds` (panic seam) |

`e[0] < 3` → `e[0] < 2` is an equivalent mutant (the only value it lets through, 2, is rejected as even).
With §20.5, the floor and the cap calls all removed, G9 fails at once by the seam's panic ("rsa crate reached"),
about 0.5 s wall including the cargo invocation.

## §4. Comment rewrites (DoD (iv))

- `rsa.rs:473-483` (the `generate` doc): replace "WebCrypto does not constrain its value, so an even /
  `< 3` exponent (or an unusable modulus length) surfaces as the §20.9.3 step-4 OperationError from the rsa
  crate (honored as-is)" and the `⚠ Contradicts …` pointer with: Generate Key step 2 runs §20.5
  ([`validate_rsa_keygen_params`]) before any key generation, so an even exponent, an exponent below 3 or
  at least 2^modulusLength − 1, or a modulusLength below 4 is an OperationError there; the rsa crate is
  reached only with parameters §20.5 accepts, and its failures are steps 3–4's OperationError.
- `rsa.rs:470`: the section banner names §20.9.3 / §21.4.3 only; add §22.4.3 (the function serves all
  three).
- `algorithm/normalize.rs:118-121`: replace the "rsa-crate's OperationError … honored as-is" sentence and
  the pointer with: modulusLength and publicExponent values are not checked by normalization; Generate
  Key step 2 (§20.5) checks them in `rsa::validate_rsa_keygen_params`.
- Sweep: `git grep -n 'keygen-param-validation' -- crates` must return 0 lines after the edit, and
  `git grep -n -i 'honored as-is\|does not constrain' -- crates/api/elidex-api-crypto` must show no
  remaining claim about the exponent (at `06bdfbbd` it returns only the two sites above).

## §5. DoD

- (i) Each of the four §20.5 step-3 conditions, the floor (69 / 70) and the exponent cap (2^33 − 1 / 2^33 + 1) is tested on both sides of its boundary (§3.1, §3.1b), and ml 12 / e = 39 (RSASSA and RSA-PSS) is rejected at once (G9, G10).
- (ii) An even exponent fails at once with OperationError, by a test that cannot hang if the fix regresses
  (G1–G4, G9–G10 with the panic seam).
- (iii) The RSASSA-PKCS1-v1_5, RSA-PSS and RSA-OAEP variants all go through the same validation (G2–G4).
- (iv) The two false claims are rewritten to match §20.5 and their slot pointers removed (§4).
- (v) Every WebCrypto cite added or changed in this PR is checked against webref (§1 table; re-run the
  `heading` lookups on the final head).
- (vi) On landing: close the slot, and give PM the landing text as `<path>:<lines>` on the merge commit.
- Plus: each §3.3 mutation recorded red on the final head (M-row → test name), `cargo test -p
  elidex-api-crypto --all-features` and the `tests_crypto` filter of `cargo test -p elidex-js
  --all-features` green, then the full `/pre-push`.

## §6. Facts to confirm at implementation (before the test is written)

- F1. `new_with_exp` draws from the RNG before it can loop (so the panic seam fires instead of the hang).
  Shown by M9: with the call deleted, G1 must fail by the seam's panic within seconds.
- F2. The VM marshaller passes the `publicExponent` bytes unchanged (no stripping), so G7's input reaches
  §20.5 as `[3]` (`crates/script/elidex-js/src/vm/host/subtle_crypto/marshal/params.rs:309-322` snapshots the bytes).
- F3. **Measured 2026-11-13 on `06bdfbbd`, debug build:** exponent 1 at modulusLength 2048 is NOT fast
  pre-fix. It runs the prime search and then fails: `Operation("RSA key generation failed")` in 11.1 s.
  The 65537 control at 2048 generates in 11.0 s. So G7 uses modulusLength 3 with e = 3 (measured fast:
  `Operation(...)` at once) and asserts the message contains `modulusLength`.
- F4. **Measured 2026-11-13 (same build):** an odd exponent above the rsa crate's public-exponent limit
  (2^33 + 1 and 2^40 + 1, at modulusLength 1024) fails fast with `Operation("RSA key generation failed")`,
  in 0.74 s / 0.75 s. No hang. §20.5 lets these through (they are odd, ≥ 3 and < 2^1024 − 1), and the
  failure is Generate Key's own "generation fails → OperationError" step, so they are outside this slot.
