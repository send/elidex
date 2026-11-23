# R-0d — the WebSocket protocol-list validator moves to `elidex-api-ws` (per-PR plan memo, native-construct umbrella §4.1)

## §0. Frame

**Umbrella**: the native-construct protocol (R) umbrella, `docs/plans/2026-11-vm-native-construct-contract.md`
(ratified; on main since #643 `b46a030c`; its code anchors are at its base `4b723d4b`). This memo is the per-PR plan of
its §4.1 **R-0d** bullet. §4.1's plan-review paragraph asks for "a short plan memo and `/elidex-plan-review`" because
R-0d is "a cross-crate move with the dependency decision above and a SyntaxError surface that must not change". Lane:
**L3** (PM 951; an earlier PM handoff said VM P4).

**Settled by the umbrella, not re-opened here** (§4.1, §7's `vm/host/websocket.rs` row, §9.1 step 9, M-10, the §5.2
row for `#11-http-token-predicate-unification`, Changelog Rev 1.2 F1, Rev 1.3 G1, Rev 1.4 PM 784 (2) / (3) / (6)):
the protocol-list checks leave `vm/host/` for `crates/api/elidex-api-ws/src/websocket.rs` as one NEW function
`&[String] -> Result<(), String>` beside `validate_ws_url`. `parse_protocols` keeps the marshalling and maps `Err` to
SyntaxError the same way the constructor maps `validate_ws_url`'s `Err`. `is_valid_protocol_token_char` is deleted. The
validator mints no token predicate: it calls `elidex_api_fetch::is_valid_header_name`, and `elidex-api-ws` gains a
dependency on `elidex-api-fetch` (PM 784 (2); a predicate in `elidex-plugin` is rejected under Plugin-first). No new
`host/` file. The host header copy (`vm/host/headers/validation.rs`) and `elidex-net`'s `validate_method` matcher belong
to `#11-http-token-predicate-unification` (PM 786, owner VM P4), not to R-0d. Whichever lands second rebases.

**Dates** (umbrella §9.1 step 9, PM 750; §4.1, PM 784 (3)): target **11-24**, hard edge **12-04**. A date that would
pass 11-26 goes to the PM. Order condition "after R-0c, one at a time": met, since R-0c landed 11-21 (#654 `3cbd5b88`).
Sequence (PM 960): the prerequisite **R-0d-pre** (§1) lands 11-23, then this plan PR, then the code PR, expected
**11-25**: one day past the target and inside 11-26 (PM 960 OK).

**Base**: `origin/main` at **`c3c78070`** (#664, R-0b). Every `file:line` is at `c3c78070` unless it names another
sha. `H` = `crates/script/elidex-js/src/vm/host`; `W` = `crates/api/elidex-api-ws/src`. The code PR's base is main
after R-0d-pre, which moves only test files, so the `H` / `W` / `headers.rs` anchors hold there.

### §0.1 Premises re-verified (at `c3c78070`)

- `H/websocket.rs` is **1001** lines (`git show c3c78070:crates/script/elidex-js/src/vm/host/websocket.rs | wc -l`). It
  is also unchanged in the moved ranges since `4b723d4b`: `git diff 4b723d4b c3c78070 -- <file>` shows three
  comment-only hunks, at `:643`, `:785` and `:887`, all below the ranges.
- The moved ranges are the same as the umbrella's (`grep -n` on their first and last lines). In `parse_protocols`
  (`fn` at `:460`), the post-collect block runs from `// RFC 7230 §3.2.6 token-ABNF validation` (`:496`) through the
  closing `}` of the duplicate loop (`:523`), 28 lines. `Ok(out)` (`:524`) stays. Then `is_valid_protocol_token_char`:
  its docstring at `:527`, `fn` at `:536`, closing `}` at `:558`, and the blank line `:559`, 33 lines in all.
  `parse_protocols`' docstring sentence at `:457-459`, the constructor docstring's item 4 (`:306-308`) and the module
  doc's `:47` describe the moved checks. They are rewritten in place (§1).
- `validate_ws_url` is `W/websocket.rs:80`, `pub fn validate_ws_url(url: &url::Url) -> Result<(), String>`,
  re-exported by `W/lib.rs:10-13`. `W/websocket.rs` is 338 lines, and its `#[cfg(test)] mod tests` starts at `:175`.
- `is_valid_header_name` is `crates/api/elidex-api-fetch/src/headers.rs:69`:
  `#[must_use] pub fn is_valid_header_name(name: &str) -> bool`. It is public through
  `crates/api/elidex-api-fetch/src/lib.rs:10-12` (`pub use headers::{is_valid_header_name, …}`).
- `crates/api/elidex-api-fetch/Cargo.toml`'s `[dependencies]` is empty. So the new `elidex-api-ws` →
  `elidex-api-fetch` edge closes no cycle. No crate depends on `elidex-api-fetch` today:
  `git grep -n 'elidex-api-fetch\|elidex_api_fetch' c3c78070 -- . ':!Cargo.lock' ':!docs'` prints only the workspace
  manifest (`Cargo.toml:32`, `:231`) and the crate's own `name =` line. Those hits are the positive control. R-0d makes
  `elidex-api-ws` its first dependent.
- `elidex-api-ws`'s dependents: `elidex-js` only (`crates/script/elidex-js/Cargo.toml:121`, optional, in the feature
  list at `:49`), by `git grep -n 'elidex-api-ws\|elidex_api_ws' c3c78070 -- '*Cargo.toml'`.
- The `token` spellings are as listed in M-10. `git grep -n -P 'fn is_valid_header_name|fn is_tchar|fn
  is_valid_protocol_token_char|HTTP token:' c3c78070 -- crates` prints 5 lines in 4 files. A second search by property,
  over the tchar alternation itself,
  `git grep -n -P "b'\^' \| b'_'|b'!' \| b'#'|\[!#\\\$%&" c3c78070 -- crates`, finds the same three byte-matchers
  (`headers.rs:73-74`, `transport.rs:300-304`, `headers/validation.rs:23`) and nothing else. The websocket predicate is
  written as an exclusion list, so that second search does not match it. It is the known fourth spelling.

### §0.2 Premise errors and drift

- None in the umbrella's R-0d cites: every line above matches `4b723d4b`. One error is in the brief (a claim, not the
  umbrella's): the docstring rule for NEW functions is umbrella §4.3's bullet, not "§4.6" (§6).
- **Found while measuring; not R-0d's to fix** (§5, O-1; PM 958 → `#11-host-syntaxerror-domexception`). The constructor's SyntaxErrors are ECMAScript `SyntaxError`s,
  not `DOMException`s. `VmError::syntax_error` builds `VmErrorKind::SyntaxError`, and `vm/ops.rs:185` turns that into
  the ECMAScript `SyntaxError` constructor's object. Step 9 throws "a "SyntaxError" `DOMException`" (§2). The host's
  other DOM paths already use `dom_exc_syntax_error` (`H/dom_bridge.rs:376-395`).
- `is_valid_header_name`'s docstring (`headers.rs:64-67`) cites "Fetch spec §2.2.1" and "RFC 7230 §3.2.6". In webref,
  §2.2.1 is Methods and the header name dfn is in §2.2.2 Headers (`webref dfn fetch 'header name'` →
  `§2.2.2 Headers #header-name`). RFC 7230 is obsoleted by RFC 9110. R-0d rewrites that docstring (§1.2).
- **Rev 1 errors** (R1): the `headers.rs` range is `:64-67` (`git grep -n '' c3c78070 -- <headers.rs>`), not `:63-66`,
  and RFC 9110 §5.6.2 is `token`, not "field-name" (§1.2). R1's other premises match `c3c78070`.
- Found in Rev 2: the `:354` comment `// Step 4: protocols coercion + token-ABNF validation.` names the docstring's own
  step 4, not a spec step, and stays true. It is unchanged.

## §1. Scope

**In**:
- `W/websocket.rs`: a NEW `pub fn validate_ws_protocols(protocols: &[String]) -> Result<(), String>`, placed after
  `validate_ws_url`, and its unit cells (§3). `W/lib.rs` re-exports it.
- `crates/api/elidex-api-ws/Cargo.toml`: `elidex-api-fetch.workspace = true`. This gives one new line in `Cargo.lock`
  (the `elidex-api-ws` entry's `dependencies`).
- `H/websocket.rs`: `:496-523` is replaced by one call,
  `elidex_api_ws::validate_ws_protocols(&out).map_err(|e| VmError::syntax_error(format!("Failed to construct 'WebSocket': {e}")))?;`.
  This is the same form as the constructor's `:350` / `:352`. `:527-559` is deleted. The docstrings and comments that
  describe the moved checks (`:47`, `:306-308`, `:457-459`) now name the validator and cite step 9 (§1.2). No other line
  of the file changes.
- `crates/api/elidex-api-fetch/src/headers.rs:64-67`: the docstring is corrected (§0.2) to the string in §1.2. It is
  comment only, and R-0d's new docstring links to it.
- **R-0d-pre** (standalone prerequisite PR, branch `vm-tests-websocket-split`, PM 960): `vm/tests/tests_websocket.rs`
  (1442 lines, 78 tests) is split at its section banners into the directory module `vm/tests/tests_websocket/`. It is
  a pure move, as `vm/tests/tests_mutation_observer/` was. Plan-review is skipped as a trivial pure move, like R-0c,
  and its landing memo records why. The constructor URL and protocols sections (`:100-299`) go together into one file:
  "the constructor file" below (its name is not final; likely `ctor.rs`).
- The constructor file: the four protocol-rejecting `ctor_*` cells move from `contains` matching to exact checks, and
  two cells are added (§3). There is no second, loosely overlapping set elsewhere (One issue, one way).

**Out**: no change to what is accepted or rejected, to the error type, to the check order or to any message (§2). The
ECMAScript-vs-`DOMException` type of the error (§0.2) is out of scope: slot `#11-host-syntaxerror-domexception` owns it (PM 958). The WebIDL union handling of the sequence arm
(`parse_protocols`' Array fast path and the ToString fallback for other objects) is marshalling and is not touched.
The host header copy and `validate_method` are untouched (§0). The stale `§9.3` / `§9.3.1` cites R-0d does not
rewrite (`git grep -n -P '§9\.3' c3c78070 -- <H/websocket.rs> <W/websocket.rs>` → 24 lines) stay with the HTML
section-renumber sweep (memory `project_html-section-renumber-sweep`, the WebSocket item). This includes the
neighbours of R-0d's lines: items 1–3 at `:300-305` and `W/websocket.rs:66`. R-0d writes correct cites only on the
lines it adds or rewrites (§1.2), as #440 did.

**Out, homed** (PM 960 Q2): non-Array iterables. A `Set`, a `Map` or a user iterable falls to the DOMString arm, so
`new WebSocket(u, new Set(['chat']))` stringifies to `"[object Set]"` and throws SyntaxError (docstring `:449-452`,
"Phase 1 accepts Array literals + DOMString only"). It predates R-0d, is unchanged by it, and belongs to the existing
slot `#11-webidl-sequence-dense-array-fast-path` (PM 960 adds the site). Same root: the conversion skips WebIDL
§3.2.25 Union types step 11.2.1, "Let method be ? GetMethod(V, %Symbol.iterator%)." (`webref body webidl js-union`,
source `# https://webidl.spec.whatwg.org/#js-union`). R-0d rewrites the `:457-459` sentence only; `:449-452` stays.

### §1.1 The validator

The validator moves the body of `:496-523` unchanged in order, with the predicate swapped:

```rust
/// Validate the `protocols` list of the `new WebSocket(url, protocols)` constructor, WebSockets §3.1 constructor
/// step 9 (`webref body websockets dom-websocket-websocket`): no value may occur more than once, and each value must
/// be a non-empty HTTP `token`. That is what "the requirements for elements that comprise the value of
/// `Sec-WebSocket-Protocol` fields" come to under The WebSocket Protocol [WSP]. The `token` test is
/// [`elidex_api_fetch::is_valid_header_name`] (Fetch §2.2.2 *header name*: the `field-name` production of RFC 9110's
/// field names, `field-name = token`, whose `token` is RFC 9110 §5.6.2). Values are compared by exact string
/// equality. Checks run per value in list order (empty, then token), then for duplicates over the whole list.
/// The first failure is returned as the `SyntaxError` message without its "Failed to construct" prefix, which the
/// caller adds.
pub fn validate_ws_protocols(protocols: &[String]) -> Result<(), String>
```

The body has the same two loops: the per-value loop (`is_empty` → `"empty sub-protocol is not allowed"`, then
`!is_valid_header_name(s)` → the token message), then the O(n²) duplicate loop with its comment. The empty check stays
**before** the predicate, and it must: `is_valid_header_name("")` is false, so without the empty check an empty value
would get the token message instead (mutant M1, §3).

**ECS-native judgement** (R1 F1): `validate_ws_protocols` is a pure `&[String] -> Result<(), String>` function. It
adds no component, query, side-store, `ObjectKind` or event. The only state the list reaches is the existing per-VM
WebSocket state and the `RendererToNetwork::WebSocketOpen` message, and R-0d does not touch either.

### §1.2 Rewritten lines and their cite strings

| Line (at `c3c78070`) | After R-0d (cite string exact) |
|---|---|
| `H/websocket.rs:47` (module doc) | "… + protocols coercion and validation (`elidex_api_ws::validate_ws_protocols`, WebSockets §3.1 constructor step 9) + …" |
| `H/websocket.rs:306-308` (constructor docstring item 4) | "Parse `protocols` arg (`DOMString \| sequence<DOMString>`) and validate it with `elidex_api_ws::validate_ws_protocols` (WebSockets §3.1 constructor step 9). Empty / non-token / duplicate → SyntaxError." |
| `H/websocket.rs:457-459` (`parse_protocols` docstring) | "The collected list is validated by `elidex_api_ws::validate_ws_protocols` (WebSockets §3.1 constructor step 9); its `Err` becomes a `SyntaxError`." |
| `H/websocket.rs:354` (comment) | unchanged (§0.2) |
| new validator docstring (`W/websocket.rs`) | "WebSockets §3.1 constructor step 9" (§1.1) |
| `headers.rs:64-67` | "Validate a header name: Fetch §2.2.2 *header name*, the `field-name` production of RFC 9110's field names (`field-name = token`), whose `token` is RFC 9110 §5.6.2. It must not be empty." |

"§3.1": `webref dfn websockets 'WebSocket(url, protocols)'` → `§3.1 Interface definition #dom-websocket-websocket`.
RFC 9110's field-names section is named, not numbered (RFCs are outside webref); §5.6.2 is not webref-checked (§2).

## §2. The SyntaxError surface (behaviour-neutral) and spec map

Step 9, from `webref body websockets dom-websocket-websocket` (source line
`# https://websockets.spec.whatwg.org/#dom-websocket-websocket`): "If any of the values in protocols occur more than
once or otherwise fail to match the requirements for elements that comprise the value of `Sec-WebSocket-Protocol`
fields as defined by The WebSocket protocol, then throw a "SyntaxError" DOMException. [WSP]". The step defines one
condition and one exception. It fixes no check order and no message. The element requirement is RFC 6455 §4.1 (client
requirements) item 10 (`1#token`), and `token` today is RFC 9110 §5.6.2. RFCs are outside webref, so those two numbers
are carried from the umbrella §4.1 (Rev 1.5 MIN) and are not machine-checked here. Fetch §2.2.2 *header name*
(`webref body fetch header-name`, source `# https://fetch.spec.whatwg.org/#header-name`) prints: "header name is a
byte sequence that matches the field-name token production." (no leading "A"; `webref body fetch terminology-headers`
prints the sentence with it). Step 9 says "occur more than once" and gives no folding rule, and the code compares
exactly. R-0d keeps that comparison (the table's duplicate row).

What R-0d preserves, observable from script, with the cells that pin each in §3:

| Part | Today (`H/websocket.rs` at `c3c78070`) | After R-0d |
|---|---|---|
| Error kind | `VmError::syntax_error` (ECMAScript `SyntaxError`; §0.2): `e.name` is `"SyntaxError"` and `e instanceof SyntaxError` is true | the same constructor, at the `map_err` in `parse_protocols` |
| Collect before validate | every value is coerced by ToString first (`:461-494`); a throwing `toString` wins over an invalid earlier value | unchanged (the call follows the collect) |
| Order | per value in list order: empty, then token; then duplicates over the list | the same, inside the validator |
| Messages | `Failed to construct 'WebSocket': ` + one of `empty sub-protocol is not allowed` / `sub-protocol '<s>' contains a character not allowed by RFC 7230 token ABNF` / `duplicate sub-protocol '<a>'` | byte-identical: the validator's `Err` is the suffix, and the host adds the prefix with the same `format!` as `:350` / `:352` |
| Accepted set | per `char`, 0x21..=0x7E minus 17 delimiters | `is_valid_header_name`: equivalent on every input (§2.1) |
| Duplicate comparison | exact string equality (`a == b`) | the same `a == b` |

The message keeps "RFC 7230", although RFC 7230 is obsolete. A message refresh would change what scripts see, so it
is not done here (`#11-host-syntaxerror-domexception`, PM 958).

### §2.1 Equivalence of the predicate (the core of the neutrality claim)

The old test is `s.chars().all(is_valid_protocol_token_char)`, reached only for a non-empty `s`. The new test is
`is_valid_header_name(s)`, reached only for a non-empty `s`, which is `!s.is_empty() && s.bytes().all(<77-byte
matcher>)`. **Verdict: equal on every non-empty `&str`.** On `""` the two differ (old: vacuously true; new: false),
and the empty check that runs first in both versions makes that input unreachable.

- **By argument**: `all` over chars is a conjunction over the string's scalar values, and so is `all` over bytes,
  grouped by the scalar each byte encodes. An ASCII scalar is one UTF-8 byte equal to its code point. A non-ASCII scalar
  is encoded only with bytes ≥ 0x80. The old predicate rejects every non-ASCII scalar (`c as u32` > 0x7E), and the new
  matcher rejects every byte ≥ 0x80. So equality on single scalars is enough.
- **Measured, exhaustively**: both functions were copied verbatim from `c3c78070` (`sed -n 536,558p` of
  `H/websocket.rs`, `sed -n 68,77p` of `headers.rs`) into a scratch program and run over all 1,112,064 scalar values,
  each as a one-character string. Result: `scalars=1112064 accepted=77 diffs=0` (rustc 1.99.1, printed in the same
  run). Control: a mutant old predicate that also rejects `'!'` gives `control_mutant_diffs=1`. Strings checked: `""`,
  `"chat"`, `"a b"`, `"a,b"`, `"café"`, `"é"`, `"x😀y"`, `"a\u{7F}"`, `"\u{FFFD}"`. Every one agrees once the empty
  check is applied. A JS lone surrogate reaches both as U+FFFD through `get_utf8`: it is the same `String` and is
  rejected by both.
- Bytes vs chars and the size of the accepted set: both accept exactly the 77 tchars (15 symbols, 10 digits, 52
  letters). The old docstring's "the 16 separators" lists and matches 17 (DQUOTE plus `(),/:;<=>?@[\]{}`). That
  docstring is deleted.

No open question on equivalence.

### §2.2 Spec coverage map

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WebSockets §3.1 `new WebSocket(url, protocols)` | 9 | empty / non-token (each delimiter, SP, HTAB, controls, DEL, non-ASCII) / duplicate (adjacent, non-adjacent; case-distinct is not one) / accept | `validate_ws_protocols` (NEW, `W/websocket.rs`); caller `parse_protocols` (`H/websocket.rs:460`) | ✓ (§3 U7: all 128 ASCII bytes) | yes (script supplies `protocols`) |

**Breadth**: K=1, M=1. **User-input touch audit**: no new script-reachable site. The protocols reach the broker as
before (`RendererToNetwork::WebSocketOpen`). The broker's re-validation gap belongs to slot
`#11-ws-broker-protocol-validation` (§5).

## §3. Tests and mutants

**Existing VM cells** (`tests_websocket.rs:226-298` at `c3c78070`; `git grep -n -i 'protocol' c3c78070 --
crates/script/elidex-js/src/vm/tests` gives 28 lines, all in this file): `ctor_accepts_single_string_protocol`,
`ctor_accepts_array_of_protocols`, `ctor_rejects_empty_protocol`, `ctor_rejects_protocol_with_separator`,
`ctor_rejects_protocol_with_space`, `ctor_rejects_duplicate_protocols`. The four `ctor_rejects_*` match
`format!("{err:?}")` with `contains` (`_with_space` only `"Syntax"`), so they do not pin the bytes. The two
`ctor_accepts_*` already assert exactly (`Ok`) and stay. `elidex-api-ws` has no protocol cell today.

**New unit cells** (`W/websocket.rs` `mod tests`; exact `Err` strings):

| Cell | Input → expected |
|---|---|
| U1 accept | `[]`, `["chat"]`, `["chat", "superchat"]` → `Ok` |
| U2 empty | `[""]` → `Err("empty sub-protocol is not allowed")` |
| U3 delimiter | `["bad,protocol"]` and `["a b"]` → the token message naming the value |
| U4 non-ASCII | `["café"]`, `["é"]` → the token message (the value is in the message byte-for-byte) |
| U5 duplicate | `["chat", "chat"]` and `["a", "b", "a"]` → `Err("duplicate sub-protocol 'chat'")` / `'a'` |
| U6 order | `["chat", "", "chat"]` → empty; `["a b", "a b"]` → token; `["a b", ""]` → token (the first value's token check comes before the second value's empty check) |
| U7 ASCII table | every byte 0x00..=0x7F as a one-character value → `Ok` exactly for a literal 77-character tchar string written in the test (not derived from either predicate) |
| U8 case | `["chat", "Chat"]` → `Ok` (exact equality, no ASCII case folding) |

**End-to-end cells** (the constructor file, its `with_ws_vm(false, …)` harness: an `http:` page, so `ws://` passes
the mixed-content gate; no new harness). Each runs
`try { new WebSocket('ws://example.com/socket', P) } catch (e) { globalThis.r = <expr> }` and checks `r` exactly with
`assert_eval_string` (`<expr>` = `e.name + '|' + e.message`) or `assert_eval_bool`.

| Cell | `P` → expected `r` |
|---|---|
| V1 `ctor_rejects_empty_protocol` (tightened) | `''` → `SyntaxError\|Failed to construct 'WebSocket': empty sub-protocol is not allowed` |
| V2 `ctor_rejects_protocol_with_separator` (tightened) | `'bad,protocol'` → `SyntaxError\|Failed to construct 'WebSocket': sub-protocol 'bad,protocol' contains a character not allowed by RFC 7230 token ABNF` |
| V2′ `ctor_rejects_protocol_with_space` (tightened) | `'bad protocol'` → the same message naming `'bad protocol'` |
| V3 `ctor_rejects_duplicate_protocols` (tightened) | `['chat', 'chat']` → `SyntaxError\|Failed to construct 'WebSocket': duplicate sub-protocol 'chat'` |
| V4 new, collect before validate | `['', {toString() { throw new TypeError('t') }}]` → `TypeError\|t` |
| V5 new, error kind | `''`, with `<expr>` = `e instanceof SyntaxError` → `true` |

V1–V3 check `e.name`, which a `DOMException` also gives, so they stay green across the kind fix. V5 is the cell that
`#11-host-syntaxerror-domexception`'s PR must flip, and it pins the ECMAScript kind until then.

**Commits** (as R-0b §6): (1) the four tightened cells, V4, V5 and nothing else, green on the pre-move code. The PR
body records that commit's sha, the command `cargo test -p elidex-js --all-features -- vm::tests::tests_websocket`,
and its `test result:` line. (2) The move, U1–U8, and the `headers.rs` docstring. The same command is green, recorded
the same way, plus `cargo test -p elidex-api-ws`. The commit-(1) cells staying green is the measured neutrality
evidence.

**Mutants** (run locally on the final head only, at most 2 concurrent `cargo test` processes; record which cell goes red):

| # | Mutant | Must turn red |
|---|---|---|
| M1 | drop the empty check (an empty value then gets the token message) | U2, V1 |
| M2 | drop the duplicate loop | U5, V3 |
| M3 | predicate → always true | U3, U4, U7, V2, V2′ |
| M4 | inner loop `&out[i..]` (an element compared with itself) | U1, `ctor_accepts_array_of_protocols` |
| M5 | inner loop `&out[i + 2..]`, or adjacent pairs only (`windows(2)`) | U5 (adjacent), U5 (non-adjacent) respectively |
| M6 | duplicate loop before the per-value loop | U6 |
| M7 | host `map_err` drops or changes the prefix | V1–V3 |
| M8 | host maps to `type_error` | V1–V3 (`e.name`), V5 |
| M9 | validation moved inside the collect loop | V4 |
| M10 | host maps to `dom_exc_syntax_error` (as `H/dom_bridge.rs:395`) | V5 (V1–V3 stay green: same `name` and `message`) |
| M11 | duplicate test `a.eq_ignore_ascii_case(b)` | U8 |

Before push: `cargo fmt --all`, `cargo test -p elidex-api-ws`, `cargo test -p elidex-js --all-features`,
`mise run ci`, `/pre-push`.

## §4. 1000-line verdicts (`wc -l` at `c3c78070`; after = estimate)

| File | Now | After | Verdict |
|---|---|---|---|
| `H/websocket.rs` | 1001 | ≈944 (1001 − 28 − 33 + about 4 for the call; the docstring rewrites are in place) | R-0d is the seam (umbrella §7). It ends under 1000, so no host-side split follows. The PR re-measures it. |
| `W/websocket.rs` | 338 | ≈440 | under 1000 |
| `W/lib.rs` | 13 | 13–14 | — |
| `crates/api/elidex-api-fetch/src/headers.rs` | 141 | 141 | — |
| `vm/tests/tests_websocket.rs` | 1442 | split by R-0d-pre | The touch-time split is the standalone prereq R-0d-pre (§1). R-0d edits only the constructor file, ≈200 lines after the split plus ≈40; the PR re-measures it. `tests_realtime.rs` is not touched. |

## §5. Coupled sites and open questions

- `#11-http-token-predicate-unification` (PM 786, owner VM P4). It shares no source file with R-0d. If that slot makes
  the host or `elidex-net` call `is_valid_header_name`, both PRs touch `headers.rs`'s docstring region or `Cargo.lock`.
  The later one rebases (umbrella §5.2).
- R-NAT host chapter (umbrella §4.4, L3). It rewrites the constructor (`:329`) after R-0d and cites post-move anchors.
  It also re-measures `H/websocket.rs` (umbrella §7).
- **O-1 (decided, PM 958)**: `#11-host-syntaxerror-domexception` owns the host's `VmError::syntax_error` sites where
  the spec throws a "SyntaxError" `DOMException`, incl. the "RFC 7230" message text. PM 960 Q1: after R-0d the "RFC 7230 token ABNF" message text lives in
  the validator's `Err` in `W/websocket.rs`, not in `H/`. The PM adds that site to the slot's scope. The slot recounts
  its sites by `git grep` at its own PR's head, so this memo fixes no count. That PR also flips V5 (§3).
- `#11-ws-broker-protocol-validation` (new, PM 960 Q3; owner VM P4; trigger: R-0d's landing).
  `crates/net/elidex-net/src/ws/io_loop.rs:216` joins the renderer's protocols with `", "` into `Sec-WebSocket-Protocol`
  without checking them. The slot's PR decides the crate dependency direction. Basis: Security by structure.
- `#11-webidl-sequence-dense-array-fast-path` gains the `:449-452` site (§1 Out; PM 960 Q2).
- The Codex CS cite-sweep inventory (`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/codex-lanes/cs/sweep-inventory.md`,
  its `vm/host/websocket.rs` rows). R-0d rewrites `:459` and moves the lines after the moved block (`:560` on) by about −57.
  Whichever lands second re-derives the lines. (R-0d-pre moves its `tests_websocket.rs` rows the same way.)

## §6. Acceptance

- `git grep -n -w is_valid_protocol_token_char -- crates` returns no hit. At `c3c78070` the same command returns
  `H/websocket.rs:458`, `:505` and `:536` (control).
- `git grep -n -P 'is_empty\(\)|duplicate sub-protocol' -- crates/script/elidex-js/src/vm/host/websocket.rs` returns
  no hit. Control at `c3c78070`: `:500` and `:518`, both inside the moved block, and no other line of the file.
- Every NEW function has a docstring naming its algorithm. The rule is the docstring bullet in umbrella §4.3 (the
  brief's "§4.6" is wrong: §4.6 is wire #5). `validate_ws_protocols` names WebSockets §3.1 constructor step 9 (§1.1).
- §3's two runs are recorded in the PR body, and every §3 mutant turns its cells red.
- `python3 .claude/tools/ecs_mission_ratchet.py` reports 0 new.

## §7. Plan-review scope

`/elidex-plan-review` on this memo (umbrella §4.1). In scope:
- Layering: the algorithm moves out of `host/`, and the host keeps only marshalling and the error mapping.
- The dependency edge, `elidex-api-ws` → `elidex-api-fetch`.
- The equivalence argument and its measurement (§2.1).
- Whether §3's cells and mutants pin every part of §2's table.
- The cites.

Out of scope, because the umbrella settled them (§0): where the validator lives, which predicate it calls, the
rejected `elidex-plugin` home, and the other two `token` spellings. O-1 was a question about homing the defect, decided by PM 958.

**Changelog.**

| Rev | Change | Decision |
|---|---|---|
| 1 | 2026-11-23. First draft at `c3c78070`. O-1 homed: `#11-host-syntaxerror-domexception` (§0.2, §1, §5). | PM 958 |
| 2 | 2026-11-23. plan-review R1 at `18524ebb` (CRIT 0 / IMP 3 / MIN 9 / FP 2); F1–F12. | PM 960 |
