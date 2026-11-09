# VM stack-safety guard — PR-1 plan (primitive, choke points, walkers, regexp front door)

Status: **plan (pre-implementation), revised after plan-review R1 and R2** (the R2 fold moves umbrella
I-3's consumer half, the CE-upgrade and async cases and the placement mutants to the P0 program, PM 502);
`/elidex-plan-review` required before code (edge-dense, umbrella §2). Parent:
[umbrella](2026-11-vm-stack-safety-guard.md). Siblings: [PR-1b](2026-11-vm-stack-safety-guard-pr1b.md) (the
JS→wasm entries, D-K) and the [P0 program](2026-11-vm-stack-safety-guard-p0.md) (the resource-limit `Err`
contract, its census and the choke-point placement). PR-1 does **not** close `#11-vm-call-depth-guard`: the
slot closes at PR-1b's landing (D-K, PM 483). Base: `47b6c934` (anchors below are at that commit). Rulings
of umbrella §0.1 are ratified and not re-opened here. Provenance root `H` is defined in the umbrella header;
every measurement names its log under `H/logs/`.
**Preconditions** (edges and their state: umbrella §4, the one home): the W-11 slot
`#11-abort-signal-any-dependent-flattening` (§2 A-1); the P0 program with an empty census `# pending`
section (P0 umbrella §7 L-1); the `interpreter.rs` prereq split (§5.1; discharged by #615); Slice F (§5.5); the CI/tools
`ci.yml` PRs, for the release leg (satisfied). NativeError E1 is not ordered against PR-1 (PM 490).

**Base-case reason (single PR).** PR-1 is a narrowly scoped per-PR slice under the approved
umbrella that passes its own plan-review; under CLAUDE.md's edge-dense rule (c) that makes it a
terminal unit, so it is not re-split for touching the same subsystem as its siblings.

**Relation to S1.** S1 (`elidex-wt-strrepl`, `#11-vm-string-replace-function-replacement`) rewrites
`natives_string.rs` :466-543, which shifts this plan's `natives_string.rs` anchors. PR-1 re-derives
every anchor at its own base with the commands given beside each table (§5.1, §6.1), not from the
numbers written here. Order (PM 476, restated by PM 483 P7): the guard is plan-reviewed first;
after S1 lands, PR-1's compile-site anchors (§6.1) are re-derived on PR-1's own base. The stale `§21.1.3.15` cite at `natives_string.rs:659` (search's lastIndex
comment) is not PR-1's: it belongs to the OD-2 / slice 9b re-tag, and PR-1 leaves it.

## §1. Scope

PR-1 delivers, in one PR:

1. The primitive `crate::stack_guard` (NEW) (§4), with its test-only seams (§4.5).
2. Probes at the two re-entry choke points CP-1 / CP-2 (§5.1). (The JS→wasm entries CP-3 and
   `MAX_WASM_STACK` are PR-1b's, D-K.)
3. Probes in every unbounded `elidex-js` walker (§5.2): JSON parse / internalize / stringify,
   structuredClone, `flat_into` (replacing `MAX_FLAT_DEPTH`), the parser / scope analyser /
   compiler recursive entries, the IndexedDB storable pre-check; the scope analyser's cap branch
   gets an error exit and stops leaking depth (W-8).
4. The regexp-compile front door, the routing of the five compile sites that stay compile sites
   (the sixth, structuredClone's re-compile, becomes a copy of the compiled matcher, §6.3), and the
   I-7 trip-wire (§6).
5. The PM 424 rewording of the `MAX_JSON_DEPTH` docstring (§7).
6. Cells, mutants, the explicit-thread rule, the hot-path measurement (§8–§10), the release
   test leg on the three CI OSes with its description in the `check` job's comment, and the local
   mise task `test-stack-guard-release` (NEW) (§8; PM 483 P5, PM 490).
7. The recursion cut check as a tracked CI check (§5.3) and the in-repo constant re-measurement
   driver (§6.2).

Not in PR-1: the JS→wasm entries CP-3a / CP-3b and `MAX_WASM_STACK` (PR-1b, D-K); the frame cap
(PR-2); everything in umbrella §1.2 — in particular the `AbortSignal.any` source-signal flattening
(the W-11 slot's, lands before PR-1, PM 454) and iterating the left spine of long expression chains
(§5.4, `#11-vm-frontend-left-spine-iterative`, registered by PM 483, scheduled immediately after
PR-1).

## §2. Coupled invariants

The umbrella §2 table (I-1 coverage, I-2 fail-safe bound, I-3 state integrity, I-4 semantic caps,
I-5 hot-path cost, and each pair) is this PR's invariant set; PR-1 is where all five are realised for the choke points CP-1 / CP-2
and the walkers. I-1 at the JS→wasm entries is PR-1b's (PR-1b §2); until PR-1b lands, I-1 does not
hold there (the band of PR-1b §4), which is why PR-1 does not close `#11-vm-call-depth-guard`.
**I-3** is the P0 program's: the placement of CP-1 / CP-2 inside `resource_limit::check_entry` (NEW: P0a-1) (P0a-1 §4.3,
cells and placement mutants P0a-1 §6–§7) and the contract that no consumer turns the error into a definitive
fact (P0 umbrella §4, census §5). The custom-element upgrade and lazy event-handler cases are P0b-L3's, the
async-resume case is P0 umbrella §6 item 5. PR-1's probes are that contract's producers (P0 umbrella §2
C-1 × I-1).

PR-local invariants:

- **I-6 One error shape per channel.** *Runtime channel*: every probe failure is
  `VmErrorKind::ResourceLimit` (NEW: P0a-1), materialised as a `RangeError` with `STACK_EXHAUSTED_MSG`
  (NEW: P0a-1), whatever the walker; cap failures keep their own messages (`DEPTH_EXCEEDED_MSG`; the regexp
  nesting message, §6.2). *Compile-time channel* (parser, scope analyser, compiler; no VM exists yet):
  every probe failure is one error kind, `JsParseErrorKind::StackExhausted` (NEW) in the parser and scope
  analyser and `CompileErrorKind::ResourceLimit` (NEW: P0a-1) in the compiler (`From<StackExhausted> for
  CompileError`); `compile_script` reports `CompileErrorKind::ResourceLimit` when any parse or scope error
  is `StackExhausted` (P0a-1 §4.1 item 4's rule; PR-1 adds the arm, cells `frontend_*` / `parser_*`). Cap
  failures are `JsParseErrorKind::NestingTooDeep` in the parser and (new, W-8) in the scope analyser. How §9 tells cap from probe: runtime cells compare the message; compile-time cells
  call `parse_script` / `analyze_scopes` / `compiler::compile` directly and compare the error
  **kind** (parser, scope) or the message (compiler), never the aggregated `compile_script` text.
- **I-7 Front door is the only compile path.** The `regress` crate is named only in the front-door
  module (§6.4 defines this as a property of the source, not as a list of call spellings).
- **A-1 Abort fan-out is depth 1** (assumption from the W-11 edge, PM 454) — once the W-11 slot has
  landed, a dependent signal's source signals are never dependent (DOM §3.2 "create a dependent
  abort signal", step 4.2 appends the input's source signals, not the input; dependent signals may
  still be inputs of `any()`), so `abort_signal`'s fan-out over dependent signals is depth 1 by
  construction and needs no probe. PR-1 checks A-1 two ways: (a) the PR body records
  `git merge-base --is-ancestor <W-11 slot landing sha> HEAD` → exit 0; (b) the cell
  `abort_any_chain_depth_one_precondition` (§8) builds a nested chain that aborts the process if
  the fan-out were still recursive, so a revert of the slot turns PR-1's suite red.

Pairs of I-6, I-7 and A-1 with every other invariant (18):

| Pair | Intersection |
|---|---|
| I-6 × I-1 | Each probe site belongs to exactly one channel (VM present → runtime; front end → compile-time), so the coverage audit (§5.3) can check site → channel mechanically. |
| I-6 × I-2 | A fallback-bound trip produces the same message as a real one; script cannot tell them apart, by design. Only the test-only bound-source accessor (§4.5) does, in the bound cell. |
| I-6 × I-3 | The kind's meaning (a probe failure, P0 umbrella §4 C-1) and where it is materialised are P0's (P0 umbrella §2 C-1 × C-3, C-3 × I-1); PR-1's part is that building it runs inside the reserve (I-1 × I-2). |
| I-6 × I-4 | Per channel, a cap and the probe give different messages / kinds, which is what lets the M-RX2 and M-W7 / M-W8a mutants (§9) see a cap silently answered by the probe. |
| I-6 × I-5 | The message is a `&'static str` constant; nothing is formatted unless the probe fails. |
| I-6 × I-7 | The front door's `TooDeep` / `Exhausted` arms map to the runtime channel's two messages; its `Syntax` arm keeps each site's current class and message (§6.3), so I-6 does not reach it. |
| I-6 × A-1 | Nothing on the abort path can raise I-6's error (no probe there); a regression of A-1 shows as a process abort in the A-1 cell, not as a `RangeError`. |
| I-7 × I-1 | A future compile site (the `RegExp` constructor, `matchAll`) is covered the moment it calls the front door, and §6.4's property makes any other path a CI failure. |
| I-7 × I-2 | The front door's ask is `WALKER_RESERVE + weight` against the same per-thread bound; under the fallback bound it rejects earlier (fail-early direction). |
| I-7 × I-3 | Every site compiles before it allocates the RegExp object (`load_constant`, the string natives), so a front-door failure leaves no half-built RegExp. `clone_regexp` does not compile (it copies the matcher, §6.3), so it has no front-door failure to order. |
| I-7 × I-4 | `MAX_REGEXP_DEPTH` has one home that both users import: `crate::regexp` (`regexp/mod.rs`, `pub(crate)`), used by the front door's cap and by `crate::regexp`'s own parser (`regexp/parser.rs` :21/:99/:323 today, where it is private). Moving it into the front door instead would leave that parser uncapped. |
| I-7 × I-5 | The weight pass is one scan of the pattern per compile, and a literal compiles on every evaluation (`load_constant`), so its cost is per evaluation; it is unmeasured today and §10 adds the workload. |
| I-7 × A-1 | Disjoint surfaces; no interaction. |
| A-1 × I-1 | The cut check (§5.3) treats `abort_signal` as cut on the strength of A-1, not of a probe. |
| A-1 × I-2 | Depth 1 needs no bound, so the abort path is independent of the bound's accuracy. |
| A-1 × I-3 | No probe means no half-propagated abort and no throw from `controller.abort()` — the reason A-1 is taken instead of a probe. |
| A-1 × I-4 | No cap either: depth is 1 by construction, so there is no semantic answer to keep. |
| A-1 × I-5 | No probe, no cost. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraphs 1–4 | no resource-limit deviation listed; paragraph 3 for the compile-time channel (§4.1) | `StackExhausted` → `RangeError` / `JsParseErrorKind::StackExhausted` (NEW) | ✓ | yes |
| ECMA-262 §23.1.3.13 Array.prototype.flat | depthNum = +∞ | `flat(Infinity)` no longer truncated at 100 | `native_array_flat` | ✓ | yes |
| ECMA-262 §23.1.3.13.1 FlattenIntoArray | recursive call, depth > 0 | +∞ / finite | `flat_into` | ✓ | yes |
| ECMA-262 §25.5.2.1 ParseJSON | step 1 validity, step 3 ParseText, step 6 `! Evaluation`; a cap / probe `RangeError` deviates from both (resource limit) | array / object | `parse_array` / `parse_object` | ✓ | yes |
| ECMA-262 §25.5.2.4 InternalizeJSONProperty | per element / key recursion | array / object | `internalize` | ✓ | yes |
| ECMA-262 §25.5.4.5 SerializeJSONObject | per key | nested object | `serialize_object` | ✓ | yes |
| ECMA-262 §25.5.4.6 SerializeJSONArray | per index | nested array | `serialize_array` | ✓ | yes |
| ECMA-262 §23.1.3.18.1 JoinIndexedProperties | step 4.d ToString per element, no cycle detection | cyclic array | CP-1 (re-entry through `toString`) | ✓ | yes |
| ECMA-262 §13.2.7.3 Runtime Semantics: Evaluation | step 3 `! RegExpCreate`: a cap / probe throw here deviates from the `!` (resource limit) | literal, compiled on every evaluation | `load_constant` | ✓ | yes |
| ECMA-262 §22.1.3.13 String.prototype.match | step 5 RegExpCreate (string argument; a non-RegExp object reaches it after step 3) | string / non-RegExp object | `native_string_match` ×2 | ✓ | yes |
| ECMA-262 §22.1.3.21 String.prototype.search | step 5 RegExpCreate (as for match) | string / non-RegExp object | `native_string_search` ×2 | ✓ | yes |
| ECMA-262 §22.2.3.1 RegExpCreate | step 2 RegExpInitialize | pattern | front door `compile_regexp` (NEW) | ✓ | yes |
| ECMA-262 §22.2.3.3 RegExpInitialize | step 14 ParsePattern, step 15 SyntaxError, step 22 CompilePattern | convergence point of RegExpCreate and the constructor | front door `compile_regexp` (NEW) | ✓ | yes |
| ECMA-262 §22.2.3.4 Static Semantics: ParsePattern | pattern parse | groups / disjunction | front door weight pass (NEW) | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal | step 28.4 recursion per property; step 13 RegExp copies `[[RegExpMatcher]]` (no compile step); step 18 Error | Array / ordinary / Error / RegExp | `clone_recursive`, `clone_regexp` (matcher copy, §6.3) | ✓ | yes |
| WHATWG HTML §2.7.7 StructuredDeserialize | step 25.3 recursion per property; step 12 RegExp from the copied matcher | Array / Object / RegExp | `clone_recursive` (fused walk), `clone_regexp` (the copied `regress::Regex`) | ✓ | yes |
| WHATWG HTML §2.7.11 Structured cloning API | steps 1–2 (§2.7.8 / §2.7.9) | — | `clone_recursive` entry | ✓ | yes |
| IndexedDB-3 §5.11 Clone a value | steps 3–4 (StructuredSerializeForStorage, StructuredDeserialize) | stored value | `reject_non_json_storable` | ✓ | yes |

**Breadth**: K=3 specs (ECMA-262, WHATWG HTML, IndexedDB-3), M=18 entries (`preflight.py` at this
revision: M=18, K=3, `IndexedDB-3` an unmapped label whose row is not webref-verified). The R1
revision's changes against the R0 table (which, like the umbrella's, said M=12 with different
rows): §25.5.2 → §25.5.2.1 ParseJSON; added JoinIndexedProperties, RegExpCreate, RegExpInitialize,
StructuredDeserialize, §2.7.11 re-labelled, IndexedDB-3 §5.11; rows PR-1 does not touch are in
umbrella §3.2. D-W's two WebAssembly JS API rows moved to PR-1b §3 with the split.
`preflight.py`'s own figures are recorded again at landing (§12 L-9).
**Split decision (D-K = split, PM 483)**: with the wasm rows, K=4 / M=20 crossed both
SPLIT-RECOMMENDED thresholds, and the D-W rows alone crossed them. They are split into PR-1b:
a different crate (`elidex-wasm-runtime`'s `Config`), a different spec (WebAssembly JS API), and
wasmtime's per-store limit is an invariant of its own (PR-1b §2 W-2). After the split K=3 / M=18:
below both thresholds, so PR-1 is a single PR without a breadth appeal.

### §3.1 User-input touch audit

All rows are script-controlled. Adjacent pre-existing code (measurements in umbrella §3.1 / §8 M-8):
- Left-deep chains: the compiler (no cap) aborts at 383 terms debug / 1677 release on 2 MiB today;
  the scope analyser's cap branch skips silently and leaks depth past 1025 terms, giving wrong code
  for everything after the chain. PR-1 gives the scope cap an error exit and the compiler a probe
  (§5.2 W-8, W-9); chain length is §5.4.
- The four `natives_string.rs` sites and the literal site map a `regress` syntax error to
  `TypeError` (spec: `SyntaxError` / early error). Unchanged; the front door returns the syntax
  error to the caller untouched (§6.3). Home: a child of VM P4 slice-plan umbrella 8 (RegExp
  completion), owner VM P4: `#11-vm-regexp-syntax-error-surface` (PM 483).
- `AbortSignal.any` chains whose intermediate composites are unreachable lose propagation after a
  GC today (chain of 10 000 with no references kept → `s.aborted === false` after `c.abort()`;
  §11 M-AB). Not PR-1's: the W-11 slot's flattening removes the intermediate links before PR-1
  (PM 454).

### §3.2 What each new surface's docstring cites

| Surface (NEW) | Cites |
|---|---|
| `crate::stack_guard` module doc / `ensure_stack` | ECMA-262 §17 (no resource-limit deviation: the limit is elidex's, PM 424); I-2's fail-early rule; the stack-switching constraint (§4.4) |
| `JsParseErrorKind::StackExhausted`, W-8's scope-analyser `NestingTooDeep` exit | ECMA-262 §17 paragraph 3: reported before evaluation, a stated resource-limit deviation (§4.1) |
| `vm/regexp_compile.rs` / `compile_regexp` | ECMA-262 §22.2.3.3 RegExpInitialize steps 14–15 / 22 (what is parsed and compiled) and §22.2.3.4 ParsePattern (what the weight pass approximates); the resource-limit status of `TooDeep` / `Exhausted` (no spec step throws them) |
| `From<StackExhausted> for VmError` / `CompileError`, `From<RegExpCompileError> for VmError` (`STACK_EXHAUSTED_MSG` and the kinds are P0a-1's) | none beyond the I-6 channel they belong to (they are mappings, not algorithms) |
| `MAX_REGEXP_DEPTH` (`crate::regexp`) | the semantic-cap role (I-4) and its two users; no spec section (no spec defines it) |
| `REGEXP_GROUP_COST` / `REGEXP_ALT_COST` | the ratchet cells and the in-repo driver that re-derive them (§6.2) |
| `clone_regexp` docstring (rewritten, its claims change) | HTML §2.7.4 StructuredSerializeInternal step 13 and §2.7.7 StructuredDeserialize step 12 (the matcher is copied; `lastIndex` is not among the copied slots) |

### §3.3 Stale cites seen in touched files (recorded, owner named, not fixed here)

| Site (at `47b6c934`) | Cite | Actual (webref) | Owner |
|---|---|---|---|
| `structured_clone.rs:63-64` | StructuredSerialize "§2.7.4", StructuredDeserialize "§2.7.6" | §2.7.5 / §2.7.7 | `#11-vm-structured-clone-layering-seam` (HTML §2.7.5 / §2.7.7 / §2.7.8 fidelity is one of its declared axes, `docs/plans/2026-07-vm-p4-slot-ledger.md` row :332) |
| `structured_clone.rs:153` | "§2.7.4 StructuredSerialize" | §2.7.4 is StructuredSerializeInternal | same |
| `structured_clone.rs:717–:872` | "§2.7.3 StructuredSerializeInternal step 13.2.x / 14.1" | §2.7.3 is "Restricting to the same agent cluster"; SSI is §2.7.4 | same |
| `structured_clone.rs` `clone_regexp` body comment (the own-property layout, "§21.2.5.10 / §21.2.5.3", and its "§2.9 step 15"), and `:933` ("spec §2.9") | "§2.9 step 15", "§21.2.5.10 / §21.2.5.3", "spec §2.9" | HTML §2.7.4 step 13 / §2.7.7 step 12; ECMA-262 RegExp prototype is §22.2.6; structuredClone is §2.7.11 | same. (The `clone_regexp` **docstring**, which carries "§2.9 step 15" too, is not in this row: D-SC makes its "re-compile" claim false, so PR-1 rewrites the whole docstring with webref-verified cites, §3.2.) |
| `natives_string.rs:659` | "§21.1.3.15" | not PR-1's (OD-2 / 9b re-tag) | VM P4 slice 9b |

PR-1's own new lines in `structured_clone.rs` cite webref-verified numbers; the old lines are left
for their owner, so the file carries both until the seam lands.

## §4. The primitive (NEW)

### §4.1 API

File `crates/script/elidex-js/src/stack_guard.rs` (NEW), `pub(crate) mod stack_guard` in
`lib.rs`. Crate root, not `vm/`, because the parser, scope analyser and compiler run without a
`VmInner`. (`pub(crate)` is the right visibility for one crate; the L2 slot's note in umbrella §1.2
covers the case where another crate needs it.)

```rust
/// Not enough Rust stack is left for the requested work. Error-type-independent:
/// each error domain converts it with `From`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StackExhausted;                       // (NEW)

/// Reserve every walker and choke point asks for (bytes). §4.3.
pub(crate) const WALKER_RESERVE: usize = 256 * 1024;    // (NEW)

/// Ok iff at least `need` bytes of this thread's stack remain below the current SP.
#[inline]
pub(crate) fn ensure_stack(need: usize) -> Result<(), StackExhausted>;  // (NEW)
```

Conversions (all NEW): `From<StackExhausted> for VmError` → `VmError::resource_limit()` (NEW: P0a-1; message
`STACK_EXHAUSTED_MSG`, P0a-1 §4.1); `From<StackExhausted> for CompileError` → kind
`CompileErrorKind::ResourceLimit` (NEW: P0a-1); a new `JsParseErrorKind::StackExhausted` (NEW) variant pushed
by the parser / scope probe branches.
There is exactly one primitive (PM 436 R1): walkers and choke points call
`ensure_stack(WALKER_RESERVE)?` (CP-1 / CP-2 inside P0a-1's `check_entry`, §5.1); the regexp front door calls `ensure_stack(WALKER_RESERVE +
weight_cost)` (§6.2). No second "check" function.

**The compile-time channel and §17 paragraph 3.** `compile_script` turns any parse or scope error
into `CompileError`, so a too-deep or too-long source is reported as the script's compile failure,
before any of it runs — the same channel `NestingTooDeep` uses today. §17 paragraph 3 forbids
treating "other kinds of errors" as early errors. This is a deviation, and it is stated rather
than argued away: no spec error exists here at all (the source is valid), so the whole outcome is
the resource-limit deviation, and reporting it before evaluation is part of that deviation. The
alternative — compile the script and raise the error when the over-deep construct is evaluated —
is not available: the analyser has not built the scopes of the unanalysed subtree, and `var` /
function declarations inside it hoist into scopes that enclose code which would run first, so no
partial program exists that evaluates the rest faithfully (umbrella §3.1 measures what today's
partial analysis produces: wrong code). **`eval` / `Function`**: when they land, `StackExhausted`
from the compile they perform maps to a `RangeError` thrown at the call (a runtime error of the
caller), which is §17-consistent. Home of that decision: `#11-vm-function-constructor-global`
(owns `Function` and `eval`); recorded here so it is not re-made.

### §4.2 Per-thread bound: where computed, where cached, unknown bound

- **Cached** in a `thread_local!` `const`-initialised `Cell<usize>` (`LOW`, 0 = not yet known) in
  `stack_guard.rs`. The stack is a per-thread resource, not a per-entity or per-VM fact, so a
  thread-local is its owner (not an ECS side-store question; nothing is entity-keyed).
- **Computed** lazily on a thread's first `ensure_stack` call, once:
  - macOS: `pthread_get_stackaddr_np(pthread_self())` (top) − `pthread_get_stacksize_np` → low.
    Measured: the reported size is the requested size + 12 KiB, and the guard faults at the
    reported low (umbrella §7, §8 M-5).
  - Linux: `pthread_getattr_np` + `pthread_attr_getstack` → low (the attr's address is the low end); `pthread_attr_destroy`.
  - Windows: `GetCurrentThreadStackLimits(&low, &high)` → low.
  - SP: address of a `black_box`ed local in `ensure_stack` itself.
  - `remaining = sp.saturating_sub(low)`; `Err` iff `remaining < need`.
- **Unknown bound** (other targets, or the OS call fails): **fail early**. `LOW` is set to the
  first probe's SP − `FALLBACK_SPAN` (`FALLBACK_SPAN` = 1 MiB, NEW). The two failure directions
  are a spurious catchable `RangeError` and a process abort; only the first is recoverable (I-2).
  **Assumption** behind "never late": the first probe on the thread runs near the thread's top, so
  first-probe SP − 1 MiB is above the real low on every thread at least 1 MiB + (top − first-probe
  SP) deep. True on every thread elidex creates (2 MiB plain `std::thread::spawn`; the first probe
  is the VM's first native call or first parse). **Residual case, stated**: an embedder on an
  unsupported target that creates the VM on a thread smaller than that, or whose first VM call
  happens deep in its own stack, gets a late bound — abort, not `RangeError`. Not engineered
  around because no supported target takes this branch (cell `stack_guard_bound_is_os_reported`).
  Measured once (umbrella §8 M-9): the forced fallback stops recursion with a `RangeError` at
  under half the OS-bound depth, both profiles.
- **Dependencies**: `libc = "0.2"` under `cfg(unix)` and `windows-sys = { version = "0.61", features =
  ["Win32_System_Threading"] }` under `cfg(windows)` become direct deps of `elidex-js`. Both are
  already in `Cargo.lock` (`libc 0.2.191`, `windows-sys 0.61.2`) and already direct deps of
  `elidex-sandbox` with that feature set, so no new crate and no `deny.toml` license change.
- **`unsafe`**: the workspace sets `unsafe_code = "warn"`. The OS queries are the only `unsafe` in
  the module: one `unsafe` block per platform function (`os_low` for macOS / Linux / Windows), each
  with `#[allow(unsafe_code)]` on that item only (the existing per-site precedent:
  `elidex-sandbox`'s `macos.rs` / `windows.rs`) and a `// SAFETY:` comment stating the FFI contract
  (calling-thread handle, out-pointers to live locals, `pthread_attr_destroy` on every path after a
  successful `pthread_getattr_np`). `ensure_stack` itself has no `unsafe` (the SP read is a
  `black_box`ed local's address).

### §4.3 Reserve sizing

`WALKER_RESERVE` must exceed the largest stack segment between two probes (I-1 × I-2), including
building the `RangeError` and unwinding to the catch.

| Measurement | debug | release | Command |
|---|---|---|---|
| Minimum reserve at which every walker, run at the callback floor (deepest point the choke-point probe allows), does not abort | ~31 KiB (flat with `MAX_FLAT_DEPTH` still in place: ~46 KiB) | ~15 KiB | `H/scripts/bisect.sh <debug\|release> <walker> <cap>` for walker ∈ none parse stringify internalize sc flat error (`H/logs/{dbg,rel}-bisect.log`) |
| Re-entry depth reached with 256 KiB | d≈30–42 | d≈250–409 | `H/scripts/run.sh <profile> ON256` |

(16 KiB resolution; prototype `H/prototype/stack_guard.rs`, env `SG_RESERVE` / `SG_OFF` /
`SG_FORCE_UNKNOWN`.)

256 KiB is 8× the debug minimum and 16× the release minimum. The margin is deliberate: the
out-of-scope DOM walkers (umbrella §1.2, `#11-dom-walkers-iterative-over-ecs-links`) and any host
native that recurses without a probe run inside that reserve, and it also absorbs an
under-estimate in the regexp cost constants (§6.2). It costs 12.5 % of a 2 MiB thread. One
constant for both profiles. The ratchet cell `stack_guard_reserve_margin_ratchet` (§8) re-runs the
floor cells with the test-only reserve of 64 KiB (2× the debug minimum); frame growth that eats
the margin fails that cell long before the production reserve is threatened. In release the
64 KiB ratchet has 4× headroom over the measured minimum, so it is weaker there than in debug; and
it runs no DOM walker, so it does not cover the headroom the L3 slot relies on (§12 L-6).

### §4.4 What else uses the VM thread's stack

- **Stack switching**: none compiled. `stacker` / `psm` / `corosensei` are absent from `Cargo.lock`;
  `wasmtime-internal-fiber` 50.0.1 is in `Cargo.lock` but not in the build graph (wasmtime's
  `async` feature is off; `cargo tree -i` prints nothing, umbrella §8 M-10). So the cached bound
  stays valid for the thread's lifetime. **Mechanism** (not prose): PR-1 adds `deny.toml` `[bans]`
  entries for `stacker`, `psm`, `corosensei` and `wasmtime-internal-fiber`, each with the reason
  "invalidates `crate::stack_guard`'s cached per-thread bound"; `cargo deny check bans` must be
  green at PR-1's base. If cargo-deny's graph does not list the inactive optional fiber crate (so a ban
  could not fire), that entry is replaced by a step in the same `deny` job running umbrella §8 M-10's
  command, `cargo tree --offline -p elidex-js --all-features -i wasmtime-internal-fiber`, which must print
  nothing; the other three stay bans. The PR says which branch it took. Adding any
  of them then fails `deny` until the guard is redesigned for it.
- **wasm** runs synchronously on the VM thread and is the one other consumer of it; its entries
  (CP-3a / CP-3b), `MAX_WASM_STACK` and the explicit `max_wasm_stack` are PR-1b's (D-K, PR-1b §4).
  PR-1's part is only the stack-switching ban above, which covers wasmtime's fiber crate too.

### §4.5 Test-only seams

Two seams, neither changing the production or benchmark shape of the hot path (§10 measures a
non-test release build):

- **Reserve override**: under `cfg(test)`, `WALKER_RESERVE` call sites read
  `stack_guard::walker_reserve()`, which returns a `thread_local!` override (default 256 KiB);
  under `cfg(not(test))` the same name is the `const`. The test build therefore has one extra TLS
  read per probe, the production build none. The 64 KiB ratchet sets the override on its thread.
- **Forced-unknown bound**: under `cfg(test)`, `stack_guard::force_unknown_bound()` (NEW: test-only) sets
  a `thread_local!` flag consulted only in the lazy init (once per thread, off the hot path), which makes
  the OS query report unknown. Because the bound is computed at the thread's first probe, the flag is set
  first thing inside `on_thread`'s closure, before the VM is created or anything parses
  (`on_thread_unknown_bound(bytes, f)`, NEW, does both). `stack_guard::bound_source()` (NEW: test-only)
  returns the bound's source (OS / fallback) and span for the bound cells.

## §5. Call sites

### §5.1 Choke points

The check sites and their placement are P0a-1's (`check_entry(Call)` first in `with_call_mode`,
`check_entry(Resume)` first in `resume_generator`; P0a-1 §4.3). PR-1 adds `ensure_stack(WALKER_RESERVE)` for
`Call` / `Resume` inside `check_entry`.

| Id | Site | Why this covers it |
|---|---|---|
| CP-1 | `VmInner::with_call_mode` (`crates/script/elidex-js/src/vm/interpreter.rs:160`) | every native→JS call (`call`, `call_construct_native`, `run_function`) and `construct_synchronous` (`crates/script/elidex-js/src/vm/dispatch_class.rs:323`) enter through it; the nested `run()` sites at `interpreter_call.rs:477` and `dispatch_class.rs:364` are inside it |
| CP-2 | `VmInner::resume_generator` (`crates/script/elidex-js/src/vm/natives_generator.rs:114`) | the nested `run()` at `natives_generator.rs:276`; reached both from the generator natives (`natives_generator.rs:374/404/432`) and, outside `with_call_mode`, from inline async calls (`push_js_call_frame` → `drive_async_coroutine` → `natives_generator.rs:472`) |

Enumeration of nested `run()` (re-derive at PR-1's base: `git grep -nE '(self|vm|inner|ctx\.vm)\.run\(' -- crates/script/elidex-js/src/vm ':!**/tests/**'`): 3 sites, lines `dispatch_class.rs:364` / `interpreter_call.rs:477` / `natives_generator.rs:276`; each is behind CP-1 or CP-2.
Callers of `with_call_mode` outside tests (`git grep -n 'with_call_mode(' -- crates/script/elidex-js/src/vm ':!**/tests/**'`): `interpreter.rs` lines 82/102/298 and `dispatch_class.rs:323`.

**`interpreter.rs` cohesion verdict (CLAUDE.md 1000-line rule; the split is discharged by #615: the
`with_call_mode` tests are `vm/tests/tests_with_call_mode.rs`, the call-op dispatch is
`vm/interpreter_call.rs`, and `interpreter.rs` keeps the entry half, 330 lines).** At the base: 1367 lines: source 1–1043, then an
in-file `#[cfg(test)] mod tests` (1044–1367, six cells all about `with_call_mode`'s panic safety).
Two real seams: (1) the test module, a single subject, moves to `vm/tests/`; (2) the remaining
source splits between the Rust-side entry points (`eval`, `run_script`, `call`,
`call_construct_native`, `with_call_mode`, `run_function`: lines 35–280 and 1004–1028) and the
call-op dispatch (`call_dispatch`, `call_internal`, `push_js_call_frame`: lines 280–1003) — "who
enters the interpreter from Rust" vs "how a call op is dispatched"; CP-1 lands in the first half.
Both are a standalone prereq split (umbrella §4), not part of PR-1. The file is on the VM P4
touch-time watch list (umbrellas 1a and 3, `docs/plans/2026-07-vm-p4-touch-time-split-record.md`);
the prereq PR records the verdict there.

**`vm/object_kind.rs` and `vm/mod.rs` cohesion verdicts (same rule, decided here).** PR-1 touches
each by one line. Both verdicts below are scoped to **the range PR-1 touches** (one line in each,
and the touch-time split it would require); they are not a whole-file judgement that no seam
exists. (The touch-time split record lists `vm/object_kind.rs` as *not on any watch list*,
with umbrellas 6, 7 and 10 owing it a verdict through their children, and does not list `vm/mod.rs`
at all, so neither verdict is owned elsewhere; this plan states both.)

- **`vm/object_kind.rs`, 1645 lines: no touch-time seam for the range PR-1 touches → no prereq split.** Shape at
  `47b6c934`: module doc and imports 1–52, two small helper enums `DomTokenListSource` /
  `ObserverKind` (53–110), the `ObjectKind` enum itself (113–1579, 106 variants, 1207 of those
  lines `///` docs, 76 `#[cfg]` gates), and one `impl ObjectKind` of two brand predicates
  (`is_callable`, `is_non_node_event_target`, 1581–1645). The file is one type definition: an enum
  cannot be divided between files, and its two predicates are the enum's own brand sets (the
  `is_constructor` predicate that had a different subject was already moved out, #588). The only
  ways to shrink it are (a) moving the ~60 lines of helper enums, which leaves a 1585-line file and
  is not a seam, or (b) moving variant payloads into named structs in their subsystems' modules,
  which changes `ObjectKind`'s representation — a design change for the umbrellas the record names,
  not a touch-time split. **The touch is not avoidable without a worse design**: the line is
  `compiled: Box<regress::Regex>` (`:132`), and I-7's property (§6.4) needs every file outside the
  front door to stop naming `regress`; the alternatives are an exemption for this file in the wire
  (a second file allowed to name the crate, which is the hole I-7 closes) or moving the matcher out
  of the variant into a side-store (an entity-of-the-heap fact moved off its object for a lint).
- **`vm/mod.rs`, 2785 lines: no touch-time seam for the range PR-1 touches → no prereq split.** Shape at `47b6c934`: the
  submodule declarations and two constants (1–183), the `VmInner` struct definition (184–2773:
  346 fields, 1937 of those lines `///` docs, 287 attributes, no function bodies), and the `Vm`
  wrapper struct (2775–2781) with a trailing comment. A struct definition cannot be divided between files; the only seam is
  grouping `VmInner`'s fields into subsystem sub-structs, which memory
  `vm-inner-substruct-deferral` records as an architectural refactor (split borrows in the GC mark
  phase) rather than a file split, deferred with no slot. That memory's trigger 1 ("`vm/mod.rs`
  grows past ~1500 LoC") has fired (it was written at 1116 lines). Recorded for the ledger (§12 L-17): a
  separate `VmInner` sub-struct proposal (`vm-inner-substruct-deferral` trigger 1) is owed after
  this plan's re-review; it is not designed here and is not a PR-1 prerequisite, since PR-1 adds no
  field. **The touch**: one `mod
  regexp_compile;` line. It is avoidable only by homing the front door outside `vm/` — e.g. as
  a `compile` submodule of `crate::regexp` beside `MAX_REGEXP_DEPTH` — and that is worse: `crate::regexp` is the
  front end's static-semantics module (the lexer's flag parser, the unconnected pattern parser),
  while the front door is a runtime-channel path (I-6) whose callers are all VM sites and whose
  `From<RegExpCompileError> for VmError` would make the front-end module import the VM's error
  type. So the front door stays at `vm/regexp_compile.rs` and the one `mod` line stays.

**host/ touch table.** PR-1 touches two `vm/host/` files (the `vm/host/wasm/` files are PR-1b's). PR-1's
change adds a stack check (and, in `structured_clone.rs`, replaces a re-compile with a copy, a net
deletion) and no algorithm. Whether the pre-existing algorithms belong in `vm/host/` is not claimed here.

| File | Change | Layering status |
|---|---|---|
| `crates/script/elidex-js/src/vm/host/structured_clone.rs` | W-6 probe at `clone_recursive`; `clone_regexp` copies the compiled matcher instead of re-compiling (§6.3) | the fused clone algorithm is not yet admitted by `vm/host/mod.rs:11-18`; its home is the derivation owned by `#11-vm-structured-clone-layering-seam`, which inherits W-6's probe and the front-door obligations (§5.5, PM 483) |
| `crates/script/elidex-js/src/vm/host/indexeddb/value.rs` | W-12 probe at `reject_non_json_storable` | a storability walk over `ObjectKind`; pre-existing, PR-1 adds the probe only |

No existing remaining-stack API to reuse: `git grep -nE 'remaining_stack|stack_guard|ensure_stack|stacker|pthread_get_stack|GetCurrentThreadStackLimits|pthread_getattr_np' -- crates`
at `47b6c934` → only `host/custom_elements/upgrade.rs`'s `_stack_guard` (the CE construction-stack
RAII guard, unrelated).

### §5.2 Walkers

Each probe is `crate::stack_guard::ensure_stack` (NEW) `(WALKER_RESERVE)?` (or the parse-error form) at the
walker's existing depth check, so cap and probe share one exit (I-3 × I-4).

| Id | Walker | Site | Existing semantic bound | Notes |
|---|---|---|---|---|
| W-1 | `parse_array` (`JsonParser`) | `crates/script/elidex-js/src/vm/natives_json/parse.rs:278` | `MAX_JSON_DEPTH` | probe before `self.depth += 1` |
| W-2 | `parse_object` (`JsonParser`) | `crates/script/elidex-js/src/vm/natives_json/parse.rs:321` | `MAX_JSON_DEPTH` | same |
| W-3 | `internalize` | `crates/script/elidex-js/src/vm/natives_json/parse.rs:410` | `MAX_JSON_DEPTH` | inside the `Object` arm, before the depth check |
| W-4 | `serialize_array` (method of `JsonSerializer`) | `crates/script/elidex-js/src/vm/natives_json/stringify.rs:144` | `MAX_JSON_DEPTH` | before `self.stack` push |
| W-5 | `serialize_object` (method of `JsonSerializer`) | `crates/script/elidex-js/src/vm/natives_json/stringify.rs:223` | `MAX_JSON_DEPTH` | same |
| W-6 | `clone_recursive` | `crates/script/elidex-js/src/vm/host/structured_clone.rs:88` | none today (PM 421 (b): the probe **is** its bound) | covers the `clone_array` / `clone_ordinary` / `clone_error` SCC; structuredClone has one fused serialize+deserialize walk (SSI step 28.4 and StructuredDeserialize step 25.3 in one recursion) |
| W-7 | Parser `enter_recursion` | `crates/script/elidex-js/src/parser/mod.rs:471` | `MAX_NESTING_DEPTH` | probe after the cap check; failure pushes `StackExhausted` and returns `false` (same contract as the cap) |
| W-7b | `parse_class_inner` | `crates/script/elidex-js/src/parser/function.rs:345` | **none today** (census hole, §5.3) | add `enter_recursion` / `leave_recursion` around the heritage parse, so the class-heritage chain counts toward `MAX_NESTING_DEPTH` and gets the probe |
| W-8 | Scope `enter_recursion` | `crates/script/elidex-js/src/scope/mod.rs:205` | `MAX_NESTING_DEPTH`, but **no error exit today** | (a) the cap branch pushes `NestingTooDeep` into the analysis errors and does not keep the increment (the parser's contract, `parser/mod.rs:471-480`), so the depth no longer leaks into later visits; (b) the probe branch pushes `StackExhausted`. Both return `false`; callers are unchanged (they already return early). One error exit, two kinds (I-6) |
| W-9 | Compiler entries `compile_expr` / `compile_stmt` / `compile_destructure_pattern` / `compile_nested_function` | `crates/script/elidex-js/src/compiler/expr.rs:74`, `compiler/stmt.rs:19`, `compiler/stmt_destructure.rs:23`, `compiler/expr_function.rs:137` | **none** (no cap; left-deep chains are bounded by neither the parser nor, past 1024, the scope pass) | `ensure_stack(WALKER_RESERVE)?` via `From<StackExhausted> for CompileError` |
| W-10 | `flat_into` | `crates/script/elidex-js/src/vm/natives_array_hof.rs:327` | `MAX_FLAT_DEPTH` = 100 (**deleted**) | `native_array_flat` passes `+∞` as `usize::MAX` and a finite depth unclamped; `flat(Infinity)` then gives FlattenIntoArray's result, and a cyclic array gives the probe's `RangeError` |
| W-11 | `abort_signal` composite fan-out | `crates/script/elidex-js/src/vm/host/abort.rs:607` / `abort_statics.rs` `native_abort_signal_static_any` | none | **not touched by PR-1** (PM 454): the W-11 slot implements DOM "create a dependent abort signal" step 4.2 (a dependent input contributes its source signals, not itself) before PR-1. Source signals are then never dependent, so the fan-out is depth 1 by construction (A-1, §2). **No probe**: one would leave a half-propagated abort and throw from `controller.abort()`, breaking I-3 |
| W-12 | `reject_non_json_storable` | `crates/script/elidex-js/src/vm/host/indexeddb/value.rs:242` | none (runs on the structuredClone output, so its depth is whatever the clone allowed) | probe; its per-level frame differs from `clone_recursive`'s, so the clone's probe does not bound it |

Under W-8 / W-9 the chain-length outcome per profile, on a 2 MiB thread at the top (from the
per-pass slopes, umbrella §8 M-8; the PR re-measures): **debug** — the compiler's probe fires near
(2048 − 256 − baseline) KiB / 5.3 KiB ≈ 330 terms, so a longer chain is a compile failure
(`StackExhausted`) instead of today's abort at 383; **release** — the scope cap fires first at
1025 terms (`NestingTooDeep`; the compiler's probe would only fire near 1450), instead of today's
wrong code from 1025 to 1676 and abort from 1677.

### §5.3 Enumeration method, the classified remainder, and the tracked check

Census (heuristic, name-based): `python3 -I H/scripts/recursion_census.py <wt> crates/script/elidex-js`
→ 22 direct self-recursions and 10 SCCs (unchanged between `efe8d05b` and `4728972c`). Cut check:
remove the probe set from the call graph and list every cycle that survives (R0 command, kept for
provenance):

```
python3 -I H/scripts/cutcheck.py <wt> crates/script/elidex-js \
  --marker 'enter_recursion\s*\(' --marker 'with_call_mode\s*\(' \
  --probe resume_generator --probe compile_expr --probe compile_stmt \
  --probe compile_destructure_pattern --probe compile_nested_function \
  --probe parse_array --probe parse_object --probe internalize \
  --probe serialize_array --probe serialize_object --probe clone_recursive \
  --probe flat_into --probe reject_non_json_storable --probe abort_signal \
  --probe drain_microtasks --probe parse_class_inner
```

`--probe abort_signal` marks a cut point, not a probe site: `abort_signal` is cut on A-1 (§2).
Result (`H/logs/cutcheck-pr1-v2.log`): 8 residual self-recursions, 2 residual SCCs, each classified:

| Residual | Class | Reason |
|---|---|---|
| `to_number`, `to_string` (`vm/coerce.rs`), `abstract_eq` (`vm/coerce.rs:753`), `abstract_relational` (`vm/coerce_ops.rs:131`) | bounded by argument | recurse once on a primitive produced by ToPrimitive / ToNumber |
| `write_number_es` (`vm/coerce_format.rs:132`) | bounded by argument | number formatting, fixed depth |
| `js_to_idb_key_depth` (`vm/host/indexeddb/value.rs:123`) | bounded by cap | `MAX_KEY_DEPTH` = 64 |
| `idb_key_to_js` (`vm/host/indexeddb/value.rs:196`) | bounded by construction | keys only enter the store through the 64-capped path |
| `disassemble_function` (`bytecode/disasm.rs:23`) | not reachable from script | only caller is `compiler/mod.rs` `#[cfg(test)]` |
| `crate::regexp` parser SCCs (`regexp/parser.rs`, `regexp/class_set.rs`) | not reachable from script **while unconnected** | `parse_pattern*` has no non-test caller (`git grep -n 'parse_pattern' -- crates/script/elidex-js/src ':!src/regexp/**'` → 0); the lexer calls only `crate::regexp::parse_flags` (`lexer/regexp_lit.rs:109`), so the module doc's "used by the lexer" is true of the flag parser and not of the pattern parser. Production compiles through `regress` (§6). The parser keeps its own `MAX_REGEXP_DEPTH` cap (§2 I-7 × I-4). **Trigger**: when umbrella 8's early-error child connects it (ECMA-262 §13.2.7.1 → §13.2.7.2 IsValidRegularExpressionLiteral → ParsePattern), its recursion must route through the probe and this row's class changes; the tracked check below turns that connection red until the allowlist row is re-classified. Deleting the module is umbrella 8's call, not PR-1's |
| event / microtask SCC (`drain_microtasks` … `walk_phase`) | cut by a re-entry flag | `drain_microtasks` returns at once when `microtask_drain_depth > 0` (`vm/natives_promise.rs:275`); every other cycle in the SCC calls JS, i.e. passes CP-1 |

Before cut, two more cycles survived and became W-7b and W-9's `compile_nested_function`:
`parse_class_expression_node | parse_class_inner | parse_primary | parse_primary_and_suffix`
(class heritage never calls `enter_recursion`) and `compile_nested_function` (self-recursive).
The class-heritage hole is real: guard off, release, `var C = class extends`×N `Object {}` + `{}`×(N−1)
parses at N=2000 on a 5.5 MiB thread (≈2.8 KiB/level, no cap) and aborts at N=5000 on 8 MiB;
debug aborts at N=300 on 2 MiB (`H/logs/rel-classext.log`, `H/logs/dbg-census-holes.log`).

**Tracked check (NEW, PR-1).** The one-off scripts become a CI check so that a new unprobed
recursion, or a removed probe, turns CI red:

- `.claude/tools/recursion_cutcheck.py` (NEW; the census and the cut check from
  `H/scripts/recursion_census.py` + `H/scripts/cutcheck.py`, merged), built on P0a-2's shared lexer and call
  graph `.claude/tools/_rust_callgraph.py` (one substrate for both checks). Each probe commit also deletes
  its function's name from the Err census's `PLANNED_PRODUCERS` (P0a-2 §6), which is red otherwise. Its probe set is **derived
  from the code**, not listed: a function is a probe site iff its body calls `ensure_stack` or
  `enter_recursion`, or it is `with_call_mode` / `resume_generator`; the cut points that are not
  probes (`abort_signal` on A-1, `drain_microtasks` on its re-entry flag) are rows of the allowlist
  with their class.
- `.claude/tools/recursion-cutcheck-allowlist.tsv` (NEW): one row per classified residual above
  (function or SCC, class, reason). The check fails on a residual cycle not in the file **and** on a
  file row whose cycle no longer exists, so the allowlist cannot drift in either direction.
- `.claude/tools/test_recursion_cutcheck.py` (NEW): a tracked `test_*.py`, so `scripts/python-suites.sh`
  collects it into the `tools` job by its step 2 (no registration). It runs the check on
  `crates/script/elidex-js` (rc 0 expected) and two controls on tempfile fixtures: an added
  unprobed self-recursive `fn` (→ red) and a stale allowlist row (→ red).
- **Stated open limits** (what the check cannot see), owner VM P4, re-evaluated when
  `#11-vm-native-callback-reentry-single-loop` is designed: recursion through trait-object
  dispatch, through closures passed across functions, and through function pointers; and the 32
  re-entry classes of umbrella §8 M-1 were chosen by hand (they demonstrate the choke points, they
  do not enumerate them — the choke-point argument is §5.1's `run()` enumeration). These are written
  in the check's header, in the PR body and on the single-loop slot's entry (§12 L-15).

### §5.4 Left-deep chains: they must compile (decision and home)

**Decision: a legitimate long left-deep chain must compile.** Generated code produces them
(string concatenation of a few thousand terms from a template, deep `a.b.c` namespace paths,
`f()()` builders), ECMA-262 bounds neither, and the parser already treats them iteratively
(constant parser depth). The depth the scope pass and compiler spend on them is an artefact of
walking the AST recursively, not nesting in the source; letting `MAX_NESTING_DEPTH` count it makes
the same constant mean "source nesting" in the parser and "chain length" in the scope pass, which
I-4 does not allow a cap to drift into.

**Remedy: iterate the left spine in the scope pass and the compiler** (walk down the left operand
of same-kind left-associative nodes — `Binary`, `Logical`, `Member`, `Call`, optional chains,
tagged templates — keeping an explicit stack, then emit upward), so chain length costs heap, not
Rust stack.

**Home: its own slot, `#11-vm-frontend-left-spine-iterative` (registered by PM 483, owner VM P4,
scheduled immediately after PR-1), not PR-1.** Reasons: it is a compiler-algorithm change with its own edge matrix (short-circuit jump
patching for `Logical`, optional-chain short-circuit to the chain end, call argument spread,
source-map order, assignment targets such as `a.b.c = x`), unrelated to the guard's invariants; and
PR-1 is correct without it (the probes stay as the safety bound after it, as for the single-loop
successor). Ordering (PM 483): the slot follows PR-1 immediately; it is not a PR-1 precondition.

**Regression PR-1 ships until that slot lands, stated, with its reason** (numbers from umbrella
§8 M-8: the largest `+` chain that does not abort today is 382 terms in debug and 1676 in release):

- **Release, 1025–1676 terms**: compile today, become a compile failure (`NestingTooDeep`, W-8's
  cap exit). **Reason this is the safe direction**: today the scope pass silently skips everything
  after such a chain and keeps the leaked depth, so the code after it is miscompiled without any
  error (a later `let` is unbound, a later function fails with an internal `CompileError`; umbrella
  §3.1). An explicit compile failure replaces silent wrong code. Only a chain that is the last thing
  in its script loses a working result. From 1677 terms the change is abort → `NestingTooDeep`.
- **Debug, ≈330–382 terms**: compile **correctly** today (the scope cap at 1025 is not reached and
  nothing is miscompiled), and become a compile failure (`StackExhausted`, W-9's probe, not
  `NestingTooDeep`). The miscompilation reason does **not** apply to this band: it is the cost of
  the probe being a safety bound, which must fire `WALKER_RESERVE` before the measured abort at
  383, so on a 2 MiB debug thread the last ≈50 terms below the abort point are given up. From 383
  terms the change is abort → `StackExhausted`. Per shape (umbrella §8 M-8 slopes): `a.b.c…` ≈330–380 (the `+` slope assumed; the PR re-measures),
  `f()()…` (≈6.4 KiB per level) ≈270–314.

When the slot lands, the `frontend_chain_*` cells (§8) move to nested-paren inputs of the same
depth, since chains no longer reach the probes.

### §5.5 Order against Slice F, and the hand-off to the layering seam (PM 483 P2)

**Ruling: Slice F → PR-1** (touch-time split). `structured_clone.rs` is 1077 lines at `47b6c934`;
Slice F (`#11-vm-structured-clone-file-split`) is its prereq split and lands first, so PR-1 makes
its W-6 probe and `clone_regexp` change in the post-split files and re-derives the `:88` / `:585`
anchors (§5.2, §6.1) on its own base with the commands given there. NativeError E1 also edits
`clone_recursive` (the ledger's edge F → E1); PM 483 does not order E1 against PR-1, and the two
have no content dependency, so whichever of the two lands second rebases onto the other (ratified,
PM 490).

**Hand-off obligations to the seam derivation.** The open umbrella
`#11-vm-structured-clone-layering-seam` (derivation triggered at Slice F's plan-review) may move the
clone algorithm out of `elidex-js`, where `pub(crate)` `stack_guard` and `regexp_compile` are
unreachable. The seam slice must **carry forward**, wherever the algorithm lands:

1. **W-6's probe**: one remaining-stack probe per clone level (the walker has no other bound,
   PM 421 (b)), with I-6's runtime message (`RangeError`, `STACK_EXHAUSTED_MSG`). If the algorithm
   leaves `elidex-js`, the primitive moves to a crate both can depend on rather than being
   duplicated (one primitive, PM 436 R1; the same rule as umbrella §1.2's L2 note).
2. **The front-door routing**: both RegExp branches stay engine-side (they read or allocate
   `ObjectKind::RegExp` and return `VmError`, i.e. marshalling, which the Layering mandate keeps in the
   engine): the in-process branch stays a matcher copy (§6.3, no compile), and a serializer that crosses a
   thread, process or storage boundary re-compiles on deserialize through `compile_regexp` (§6.3). A moved
   algorithm calls those branches across the seam; it adds no second compile path, so §6.4's property
   (no `regress` path outside `vm/regexp_compile.rs`) holds unchanged.
3. **The cut check**: the tracked check (§5.3) keeps `clone_recursive`'s cycle classified as
   probed; if the walker moves crates, the check's scope (`crates/script/elidex-js`) is widened to
   the new home in the same slice.

PR-1 records these on the seam's live entry at landing (§12 L-11).

## §6. Regexp-compile front door (PM 436 R1)

### §6.1 Compile sites

Re-derive at PR-1's base:
`git grep -nE 'regress::Regex|Regex::(with_flags|new|from_unicode)' -- crates/script/elidex-js/src ':!**/tests/**'`
— 6 compile sites at `47b6c934`: `natives_string.rs` 567/582/646/666, `dispatch_helpers.rs` 170, `structured_clone.rs` 585.
**Anchor order (PM 476 / PM 483 P7)**: guard-first — this plan is reviewed before S1 — and after S1
lands (it rewrites `natives_string.rs` :466-543) and Slice F lands (it splits `structured_clone.rs`),
these anchors are re-derived on PR-1's own base by the command above; the numbers here are
`47b6c934`'s and are not carried into the PR.
PR-1 routes **5** of them through the front door (the table); the sixth, `structured_clone.rs:585`
(`clone_regexp`'s re-compile), is not routed but **removed**: `clone_regexp` copies the compiled
matcher (D-SC, §6.3), so after PR-1 the same command reports no hit outside the front door:

| Site | Path |
|---|---|
| `crates/script/elidex-js/src/vm/natives_string.rs:567` | `String.prototype.match`, non-object argument |
| `crates/script/elidex-js/src/vm/natives_string.rs:582` | `match`, non-RegExp object argument (elidex does not perform step 3.a's `GetMethod(@@match)` for it; a known deviation, umbrella 8's dispatch charter) |
| `crates/script/elidex-js/src/vm/natives_string.rs:646` | `String.prototype.search`, non-object argument |
| `crates/script/elidex-js/src/vm/natives_string.rs:666` | `search`, non-RegExp object argument (same) |
| `crates/script/elidex-js/src/vm/dispatch_helpers.rs:170` | regexp literal (`Constant::RegExp` in `load_constant`), compiled on every evaluation |

There is no `RegExp` constructor global today, so "`RegExp(...)`" is not a current site; it is
covered by I-7 when it lands (`#11-vm-regexp-constructor-and-flags`, under umbrella 8). Neither is
`matchAll` (umbrella §3.2). Other crates use the `regex` crate (`elidex-api-sw` router,
`elidex-form-core` validation), which is not `regress` and not script-recursive; out of scope.
**Binding for umbrella 8**: the RegExp creation / construction child of umbrella 8
(`docs/plans/2026-07-vm-p4-slice-plan.md` row 8, `:976`) adopts `compile_regexp` as its compile
path (RegExpInitialize steps 14–15 / 22 are the front door), so there are never two compile paths.
PR-1 adds that pointer to row 8 (§12 L-8).

### §6.2 Weight model, cap and per-level cost

Ratified (PM 454): the two-axis weight (`groups × REGEXP_GROUP_COST + alts × REGEXP_ALT_COST`, cap
on `groups` only) is within PM 436 R1.

`regress` recurses on two axes, measured guard off (`H/scripts/needbin.sh <bin> <label>` with
`CODE="String('b'.match(<pattern>).index)"`, 16 KiB resolution):

| Axis | debug | release | Data |
|---|---|---|---|
| group nesting `(`…`)` (capturing, `(?:…)*`, lookahead all equal) | ≈5.1 KiB / level | ≈1.3 KiB / level | debug 100→200 levels: 576→1088 KiB; release 500→1000: 672→1312 KiB; `(?:` + `)*` and `(?=` at 250/500: 336/672 KiB |
| alternatives `|` within one disjunction (right-nested binary `Alt`, `regress` 0.11.1 `parse.rs` `make_alt`) | ≈0.83 KiB / alternative | ≈0.16 KiB / alternative | debug 250→500: 288→496 KiB; release 1000/2000/4000: 176/336/640 KiB (`H/logs/rx-concat-alt-rel.log` re-measured with an instrumented binary) |
| concatenation length, including of groups | none | none | `'a'.repeat(1e6)` compiles on 2 MiB (debug); `(?:a|b)` × N needs 80 KiB debug / 32 KiB release for N = 1000, 10 000, 50 000 (`H/logs/rx-concat-alt.log`, `H/logs/rx-concat-alt-rel.log`) |
| match execution length | none | none | `(?:a|b)*$` over 100 000 chars on 2 MiB (debug) |

Front door (NEW), `crates/script/elidex-js/src/vm/regexp_compile.rs`:

```rust
pub(crate) enum RegExpCompileError { Syntax(regress::Error), TooDeep, Exhausted(StackExhausted) } // (NEW)
pub(crate) fn compile_regexp(pattern: &str, flags: regress::Flags)
    -> Result<regress::Regex, RegExpCompileError>;                                       // (NEW)
```

1. **One iterative pass** over the pattern (skipping `\` escapes and `[...]` class contents) with
   a per-depth counter stack. Each open `(` pushes a frame `{ alts: 0, best: 0 }`; `|` increments
   the top frame's `alts`; a closing `)` pops its frame with `cost = max(best, alts × A)` (A =
   `REGEXP_ALT_COST`) and offers the parent `parent.alts × A + G + cost` (G = `REGEXP_GROUP_COST`),
   the parent keeping the maximum in `best`. At the end the top-level frame's
   `max(best, alts × A)` is `weight`, and the maximum stack height is `groups`. So `weight` is the
   cost of the costliest root-to-leaf path: the groups on it times G plus, at each level, the
   alternatives `regress`'s right-nested `Alt` puts before it times A. A concatenation of
   one-level groups `(?:a|b)(?:a|b)…` weighs `G + A` whatever its length, which the measurement
   above confirms is what `regress` spends.
2. `groups > MAX_REGEXP_DEPTH` (1024, the existing constant, homed in `crate::regexp` and shared
   with that module's parser, §2 I-7 × I-4) → `TooDeep` → `RangeError` "Maximum regular expression
   nesting depth exceeded" (NEW message; same class as the JSON cap). Profile-independent semantic
   bound.
3. `ensure_stack(WALKER_RESERVE + weight)` → `Exhausted` → `RangeError` `STACK_EXHAUSTED_MSG`.
4. `regress::Regex::with_flags(pattern, flags)`; a `regress::Error` is returned as `Syntax` for
   the caller to map as it does today.

Constants (NEW), `cfg(debug_assertions)`-split like `MAX_JSON_DEPTH`, each = measured slope × 1.25,
rounded up to 64 B: `REGEXP_GROUP_COST` debug 6592 B / release 1664 B; `REGEXP_ALT_COST` debug
1088 B / release 256 B.

**The `v` flag (PM 502 S-3).** Under `v` (UnicodeSets), ECMA-262 §22.2.1 Patterns lets a class nest
(`NestedClass` inside `ClassSetOperand`), and `regress` 0.11.1 recurses once per nested `[`
(`consume_class_set_operand` ↔ `consume_class_set_expression`, `parse.rs:1164-1175`). Step 1 skips class
contents and the cap counts groups only, so such a pattern weighs ≈0 and passes both: I-7 × I-1 does not
hold for it. It is latent: today `regress_flags_from_str` (`dispatch_helpers.rs:82-94`) drops `v`, itself
an unstated deviation homed in `#11-vm-regexp-v-flag-dropped` (PM 502). **Obligation on umbrella 8's `v`
child** (recorded on that slot, §12 L-8): before `v` is honoured, the weight pass parses classes with the
flags (a `v` class does not end at the first `]`), counts nested classes into the cap's depth and their
measured per-level cost into the weight, with the per-level cost measured like §6.2's axes.

**What fits, from the gate** (not from the slope). A compile at SP with `remaining` bytes passes
iff `remaining ≥ WALKER_RESERVE + weight`. At the top of a 2 MiB thread, `remaining` ≈ 2048 KiB −
baseline, where baseline is the stack already in use at the compile (≈80 KiB debug / ≈32 KiB
release for a trivial script, `H/logs/rx-concat-alt*.log`). One flat disjunction (groups = 0)
therefore fits ⌊(2048 − 256 − 32) KiB / 256 B⌋ ≈ 7 000 alternatives in release and
⌊(2048 − 256 − 80) KiB / 1088 B⌋ ≈ 1 600 in debug. Pure nesting fits ⌊1760 KiB / 1664 B⌋ ≈ 1 080
groups in release, so the cap (1024, step 2) answers first there; in debug the gate answers first,
from ⌊1712 KiB / 6592 B⌋ ≈ 265 groups. **Stated regressions** (patterns that compile today and get a
`RangeError` under PR-1, because the gate keeps the reserve free and carries the 1.25 factor, or because
the cap is newly applied to the `regress` path; slopes of the axis table, 2 MiB thread at the top):
flat disjunction ≈7 000–≈12 800 alternatives (release) and ≈1 600–≈2 370 (debug); group nesting
≈265–≈384 levels (debug, the gate) and 1025–≈1 550 (release, `MAX_REGEXP_DEPTH`, which today caps only the
unconnected `crate::regexp` parser). Alternation is not capped (only groups are); the gate is what bounds
it.

**Ratchet and re-measurement (the procedure is in the repo).** The cells `rx_group_cost_ratchet` /
`rx_alt_cost_ratchet` (§8) compile a pattern of D levels on a thread of exactly `baseline + R + D ×
cost`, where `R` is the test-only 64 KiB reserve (§4.5) and **`baseline`** is a recorded constant
beside each cell: the need of the same cell body at D = 1, 16 KiB resolution, recorded **per OS**
(`cfg(target_os)`-keyed; macOS from `H`, Linux / Windows from `scripts/stack-need.sh` run in PR-1's own
release CI leg before merge, the values written into the PR). The cell passes iff
the real need `baseline + D × slope + leaf` fits, i.e. it fails only when the real slope exceeds the
constant by more than `(R − leaf) / D`: at D = 1000 groups (release) that slack is ≈ 65 B per level
(4 % of `REGEXP_GROUP_COST`); at D = 200 (debug) ≈ 330 B (5 %); at D = 4000 / 500 alternatives
≈ 16 B / 130 B. (With `WALKER_RESERVE` in place of R, as R0 planned, the slack was 262 B / 1.3 KiB
per level, and "fails when the slope exceeds the constant" was an overclaim.) To **re-derive** a
constant when a ratchet goes red: `scripts/stack-need.sh` (NEW, tracked) bisects the thread size at
which the cell body survives, by re-running that one cell under nextest (one process per test, so
an abort is a failed run, not a lost driver) with the thread size passed in the test-only env
override `ELIDEX_SG_THREAD_KIB`; slope = Δneed / Δlevels at two depths; constant = slope × 1.25
rounded up to 64 B. The constants' docstrings cite the cells and this script, nothing outside the
repo.

### §6.3 Call-site mapping

Each of the five sites replaces its `regress::Regex::new` / `with_flags` call with
`compile_regexp`. `TooDeep` and `Exhausted` become `RangeError` via `From<RegExpCompileError>`
for `VmError` for those two arms; `Syntax(e)` keeps each site's current message and error class
(the class question belongs to umbrella 8's child `#11-vm-regexp-syntax-error-surface`, PM 483; §3.1).

- **Literal** (`load_constant`): §13.2.7.3 step 3 is `! RegExpCreate` — the spec says it cannot
  throw, because a bad pattern was already an early error (§13.2.7.1). A cap or probe throw here is
  a resource-limit deviation from that `!`, stated in the site's comment; the `Syntax` → `TypeError`
  path is the pre-existing early-error gap (umbrella §3.2), not this one. The throw is catchable because
  P0a-1 routes `Op::PushConst`'s error through `throw_error` (P0a-1 §5.2 B-1); `TooDeep` stays the cap's answer.
- **structuredClone** (`clone_regexp`, decision D-SC): **copy the matcher, do not re-compile.**
  HTML §2.7.4 StructuredSerializeInternal step 13 copies `[[RegExpMatcher]]` (with
  `[[OriginalSource]]` / `[[OriginalFlags]]`), and §2.7.7 StructuredDeserialize step 12 builds the
  new RegExp from that matcher; nothing parses, and no step can fail. `regress::Regex` is `Clone`
  (`regress` 0.11.1 `api.rs:373`, `#[derive(Debug, Clone)]`) over a flat `CompiledRegex`
  (`insn.rs:196`: `Vec<Insn>`, `Vec<BracketContents>`, counts), so the copy is heap work with no
  recursion and needs no probe. `clone_regexp` becomes: read `pattern` / `flags` / `compiled` from
  the source's `ObjectKind::RegExp`, allocate the new object with `compiled: compiled.clone()`,
  install `source` / `flags` / `lastIndex = 0` as today (`lastIndex` is not among the slots step 13
  copies). The `regress::Error` → `TypeError` defensive branch and the `regress_flags_from_str`
  call disappear with the re-compile, and `structured_clone.rs` names `regress` nowhere.
  **Paths that reach `clone_regexp`**, all in-process (re-derive:
  `git grep -n 'clone_value(' -- crates/script/elidex-js/src ':!**/tests/**'`):
  1. `structuredClone()` (`structured_clone.rs:962`): the clone is returned to the same realm.
  2. `window.postMessage` self-delivery (`pending_tasks.rs:592`): the cloned `JsValue` is queued
     as `PendingTask::PostMessage { data: cloned, .. }` on the same VM (`:611-617`).
  3. IndexedDB `value_to_json` (`indexeddb/value.rs:382`): the clone is checked by
     `reject_non_json_storable`, which rejects a RegExp (`:305`, `Kind::Reject("a RegExp")`), and only JSON
     text is stored; no RegExp reaches storage.
  The paths that do cross a boundary do **not** reach `clone_regexp`: worker / service-worker
  `postMessage` serialize through `worker_scope::serialize_message` (JSON shortcut, `worker.rs:498`,
  `worker_scope.rs:349`, `service_worker/worker.rs:95`, `service_worker/clients.rs:266`),
  `history.state` through `structured_serialize.rs` (the same JSON shortcut), and the iframe→parent
  `postMessage` through `ToString` (`pending_tasks.rs:543`); there is no `BroadcastChannel` or
  `MessageChannel`. So **no path keeps a re-compile**. When a serializer that crosses a thread,
  process or storage boundary gains RegExp support (`#11-history-state-structured-serialize-fidelity`,
  which folds the worker serializer, and `#11-idb-structured-clone-storage`), the live matcher cannot
  travel, so its deserialize step re-compiles from `[[OriginalSource]]` / `[[OriginalFlags]]`; I-7 makes
  `compile_regexp` its only path. HTML §2.7.7 StructuredDeserialize step 12 has no failure step, so a
  `RangeError` from that re-compile is elidex's resource limit, a stated deviation (§12 L-14).

### §6.4 Trip-wire (I-7), defined by the property

**Property**: outside `crates/script/elidex-js/src/vm/regexp_compile.rs`, no Rust source in
`crates/**` names the `regress` crate, and no `Cargo.toml` other than the workspace root's and
`elidex-js`'s depends on it. Rust names a crate only through a path (`regress::…`, `::regress::…`)
or a `use regress…` / `extern crate regress` item, so the source half is one pattern over the
grammar's forms, not over call spellings: `git grep -nP '\bregress\b\s*(::|as\b|;|\{)' -- crates` (Rust sources)
must report only the front-door module (`-P`, not `-E`: `git grep -E`'s `\b` matches nothing on
this git / macOS, see memory `reference_git-grep-E-word-boundary-silent-zero`). That catches
`regress::Regex::new`, `use regress::Regex` + `Regex::new`, `from_unicode`, a renamed import, and a
second crate taking the dependency. To make the property hold, the front door re-exports what other
modules use today (`pub(crate) use regress::{Flags, Match}`, and a `CompiledRegExp` type for
`ObjectKind::RegExp`'s `compiled` field, `object_kind.rs:132`), and `natives_regexp.rs`,
`natives_string.rs`, `dispatch_helpers.rs` and `object_kind.rs` switch to those names;
`structured_clone.rs` needs no new name, since D-SC deletes its only use (§6.3). A bare mention
of the word in prose is not matched (the pattern needs `::`, `as`, `;` or `{` after it), but a
comment that writes a path **is**: at `47b6c934` the pattern hits the doc comments
`dispatch_helpers.rs:81` (`` `regress::Flags` ``) and `natives_regexp.rs:4`
(`` `regress::Regex::find_from_utf16` ``), so PR-1 rewrites those two comments to the front
door's names too rather than exempting comments from the wire. The wire
lives in `.claude/tools/regress-front-door-trip-wire.sh` (NEW), registered in `scripts/trip-wires.sh`'s
`REQUIRED_WIRES`. Controls in the wire's self-check, as for the existing wires: a scratch file
outside the module with `use regress::Regex;` fails the wire, and so does a scratch `Cargo.toml`
naming `regress`.

## §7. PM 424 rewording (`crates/script/elidex-js/src/vm/natives_json/mod.rs`)

- Remove the sentence comparing the release value to another engine's (`natives_json/mod.rs:29`) and
  replace it with: "Release uses 1000, verified to hold for all three walkers (depth 990 passes,
  1000+ throws). The value is elidex's own resource limit; ECMA-262 defines none."
- Replace the "(the VM has no call-depth guard: `#11-vm-call-depth-guard`)" clause at line 27 with
  a pointer to `crate::stack_guard` (NEW): re-entry is now bounded by the choke-point probe, and the
  walkers carry their own probe; the cap stays as the semantic bound (PM 421 (a)).
- The "binding constraint is internalize … 2 MiB thread" paragraph stays as the derivation of the
  cap. Its ratchet sentence is updated, not removed: "The `json_cap_minus_10_on_production_thread_*`
  cells (cap − 10 must pass on a 2 MiB thread with the probe on, `tests_stack_guard.rs`) ratchet
  this against frame growth; the cap-boundary cells run on 8 MiB." Plus one sentence: past the cap
  the stack is the probe's concern, so the cap is no longer the only thing between the walker and
  an abort.

## §8. Cells (written first; red on `47b6c934` means abort or wrong result)

New file `crates/script/elidex-js/src/vm/tests/tests_stack_guard.rs` (NEW). Every cell runs its
body on an explicit thread via a helper `on_thread(bytes, f)` (NEW) using
`std::thread::Builder::stack_size`; probe cells use 2 MiB (the production thread), cap-boundary
cells 8 MiB, and the cap−10 derivation ratchet stays on 2 MiB with the probe on. A stack overflow
aborts the test process, which nextest reports as that test's failure. All cells run in **debug
and release**: `cargo nextest run -p elidex-js --all-features -E 'test(/tests_stack_guard/)'` and
the same with `--release`. CI: the `check` job runs the debug leg on all three OSes, and **PR-1 adds the
release leg on all three OSes** (PM 483 P5; umbrella §7, the one home of the leg's ruling and edges),
described with its reason (the release-only constants were measured on macOS aarch64 only) in the comment
of the `check` job in `.github/workflows/ci.yml` (PM 490; §12 L-12). PR-1b's cells join this file and both
legs. **Size**: ≈40 PR-1 cells and 5 PR-1b cells; if the file passes ≈800 lines while being written, it is
split by subject (`tests_stack_guard_runtime.rs` / `_frontend.rs` / `_regexp.rs`) in the same PR, with the
`test(/tests_stack_guard/)` filter still matching all three.

**Local release run (PM 490).** `mise run ci` does **not** gain the release leg. PR-1 adds the mise task
`test-stack-guard-release` (NEW): `cargo nextest run -p elidex-js --all-features --release -E
'test(/tests_stack_guard/)'`, not in `[tasks.ci]`'s `depends`, its reasons in its `mise.toml` comment (a
release build creates `target/release` in each worktree; the three-OS release check is CI's gate). PR-1's
author runs it **once** before the first push, after `mise run ci`, on the authoring host, as
`mise run test-stack-guard-release > log 2>&1`; pass iff the log has no `ERROR task failed` line (the
fixed marker of a failed mise task) and nextest's summary line reports 0 failed. The result is a PR-body
line. PR-1b's pre-push does the same.

| Cell | Body | Expect |
|---|---|---|
| `rt_reviver_recursion` | `function r(k, v) { return k === '' ? JSON.parse('[1]', r) : v } JSON.parse('[1]', r)` (declaration: a named function expression has no self-binding in elidex today) | kind `ResourceLimit` / `STACK_EXHAUSTED_MSG`: uncaught, the `Err` reaches the cell's top with its kind (`throw_error` returns the original error, P0a-1 §4.2), so the cell reads `is_resource_limit()` and the message, the rule P0a-1's `rl_nested_failure_keeps_kind` pins. Every `rt_*` cell, and every Rust-side `RangeError` expectation below, reads it so; a JS-side observation (`rt_inline_async`'s `.catch`) reads the name `vm_error_to_thrown` gives, `RangeError` |
| `rt_map_recursion` | `function f() { [1].map(f) } f()` | same |
| `rt_generator_resume` | `function* g() { g().next(); yield 1 } g().next()` | same |
| `rt_inline_async` | `async function f() { await f() } f().catch(e => globalThis.r = e.name)`, then drain microtasks | `r === 'RangeError'` |
| `rt_super_recursion` | `class B { constructor() { new C() } } class C extends B { constructor() { super() } } new C()` | `RangeError` |
| `rt_cyclic_join_range_error` | `const a = []; a[0] = a; String(a)` | `RangeError` (elidex resource limit; JoinIndexedProperties has no cycle detection; PM 424) |
| `walker_floor_{parse_array,parse_object,stringify_array,stringify_object,sc,flat}` | recurse through `[1].forEach` until the first `RangeError`, then run the walker at that depth on a cap−10 input that nests only the one container kind named | `RangeError` (probe), no abort |
| `walker_floor_internalize` | two levels above the floor, `JSON.parse('[[1],0]', function (k, v) { if (k === '0' && this.length === 2) this[1] = deep; return v })` with `deep` a cap−10-deep array built at the top: the text is shallow, so ParseText (W-1 / W-2) fits, and InternalizeJSONProperty then walks `deep` at index 1; control: the same call with a 2-deep `deep` | control returns; the cell `RangeError`, no abort (so W-3 answered: nothing else probes that walk) |
| `walker_floor_idb_storable` | two levels above the floor, `reject_non_json_storable` called directly (crate-internal cell) on a cap−10-deep value built at the top, so W-6 does not run first; control: a 2-deep value | control `Ok`; the cell `RangeError`, no abort |
| `json_cap_{parse,internalize,stringify}` (moved from `tests_json.rs` / `tests_string_complement.rs`, 8 MiB) | cap−10 passes, cap+100 throws | `DEPTH_EXCEEDED_MSG` |
| `json_cap_minus_10_on_production_thread_{parse,internalize,stringify}` (2 MiB, probe on) | cap−10 at the thread's top | passes. Measured in debug on the prototype: internalize at cap−10 leaves 397 807 B at its lowest probe, 135 KiB above the reserve (`H/logs/cap10-on-dbg.log`). If it goes red, frame growth has eaten that margin: that is `#11-vm-json-walkers-iterative`'s trigger 2 and a re-derivation of `MAX_JSON_DEPTH`, not a reason to move the cell to 8 MiB |
| `rx_nesting_repro` | `'a'.match('('.repeat(2000) + 'a' + ')'.repeat(2000))` | `RangeError`, nesting message (release aborts today, `H/logs/rel-strmatch2000.log`) |
| `rx_alternation_1e5` | `'b'.match('a\|'.repeat(100000) + 'b')` | `RangeError` / `STACK_EXHAUSTED_MSG` (debug aborts at 10 000 today) |
| `rx_concat_groups_depth_one` | `'b'.match('(?:a\|b)'.repeat(50000))` | compiles and matches (weight `G + A`; the R0 total-`|` weight rejected it) |
| `rx_literal_nesting` | a 2000-deep regexp literal in source | `RangeError` at literal evaluation (the stated deviation from `! RegExpCreate`) |
| `rx_group_cost_ratchet` / `rx_alt_cost_ratchet` | compile depth D on a thread of exactly `baseline + 64 KiB + D × cost` (test-only reserve), D = 1000 groups / 4000 alternatives (release), 200 / 500 (debug); `baseline` recorded beside the cell (§6.2) | compiles, no abort |
| `parser_parens_depth` | 2000 nested parens (cap) and 1000 nested parens at the callback floor | `NestingTooDeep`; `StackExhausted` (kinds compared, I-6) |
| `parser_class_heritage_chain` | `class extends` × 5000 | `StackExhausted` or `NestingTooDeep`, no abort (aborts today) |
| `frontend_chain_scope_cap` | `let s = 1+1+…` (1100 terms) `; let q = 5; q` through `analyze_scopes` | one `NestingTooDeep` error, no later visit skipped (red today: no error, and `q` unbound) |
| `frontend_chain_scope_probe` | 1000-term `+` chain: parse on the cell's thread, `analyze_scopes` on a thread of half its measured scope need plus the reserve, with the 64 KiB test reserve (§4.5): debug 384 + 64 KiB, release 64 + 64 KiB | `StackExhausted` in the analysis errors after at least one level was analysed, no abort |
| `frontend_chain_compile_probe` | 1000-term `+` chain: parse + scope on the cell's thread, `compiler::compile` on 2 MiB (debug; need 5.3 MiB) / 1 MiB (release; need 1.2 MiB) | `CompileErrorKind::ResourceLimit`, no abort (aborts today) |
| `frontend_compile_probe_{stmt,destructure}` | the same harness with 1000 nested blocks / nested destructuring patterns (below the parser's 1024 cap; no expression recursion on the path, so only the named entry probes it) | `CompileErrorKind::ResourceLimit`, no abort |
| `flat_infinity_deep` | `let a = [1]; for (let i = 0; i < 200; i++) a = [a]; a.flat(Infinity)` | `[1]` (FlattenIntoArray; today depth 100 leaves nesting) |
| `flat_cyclic` | `const a = []; a.push(a); a.flat(Infinity)` | `RangeError` |
| `abort_any_chain_depth_one_precondition` | on a 2 MiB thread, 10 000 chained `AbortSignal.any([s])` with intermediates kept, then `c.abort()` | `s.aborted === true`, no abort. Checks A-1 (§2): it aborts the process if the fan-out is still recursive (today: abort at 1000 kept in debug, §11 M-AB), so it is red on `47b6c934` and green only once the W-11 slot has landed. The unreferenced (GC) cell is the W-11 slot's own |
| `stack_guard_bound_is_os_reported` | on a 2 MiB thread, first probe | bound source = OS; span ∈ [2 MiB − 64 KiB, 2 MiB + 64 KiB] (macOS reports the request + 12 KiB, umbrella §7); prints span (the Linux / Windows measurement) |
| `stack_guard_unknown_bound_fails_early` | forced-unknown seam (§4.5) | first probe at top passes; recursion stops with `RangeError` at ≤ 1 MiB used |
| `stack_guard_reserve_margin_ratchet` | the `walker_floor_*` and `rt_*` bodies with the test-only reserve of 64 KiB | all `RangeError`, no abort; this is also the cell that turns a **late** bound (low below the real guard) into a red run |
| `sc_regexp_copies_matcher` | `const r = /(a)\|b/gi; r.lastIndex = 3; const c = structuredClone(r)` | `c !== r`, `c.source === r.source`, `c.flags === r.flags`, `c.lastIndex === 0`, `c.exec('xa')[1] === 'a'`, and `c` is a RegExp (`Object.prototype.toString.call(c) === '[object RegExp]'`); green today too (the behaviour is unchanged; this pins it across the rewrite) |
| `sc_regexp_needs_no_compile_room` | at the top, create `r` with `G` nested groups whose front-door weight is above half of what fits (debug G = 150, release G = 600); recurse through `[1].forEach` and at every depth call `structuredClone(r)`, recording the depth of its first failure | the clone never fails at a depth where CP-1 still admitted the callback, except at the floor level itself (where W-6's own probe may answer). With a re-compile through the front door it fails `WALKER_RESERVE + weight` above the floor, i.e. many levels earlier |

### §8.1 Tests that move to explicit `stack_size` threads

`git grep -n 'DEPTH_EXCEEDED_MSG\|NestingTooDeep\|MAX_JSON_DEPTH' ` over the `elidex-js` test files (pathspecs `crates/` + `**/tests*` and `**/` + `*_tests*.rs`; re-derive at PR-1's base):
`parse_reviver_deep_acyclic_chain_within_cap_ok` / `parse_reviver_deep_acyclic_chain_over_cap_throws` /
`parse_reviver_cyclic_graph_throws_range_error` (`tests_json.rs`), `json_parse_depth_cap` /
`json_stringify_depth_cap` (`tests_string_complement.rs`), `worker_post_message_hits_the_depth_cap_not_the_stack`
(`tests_json_shortcut_date.rs`), `deep_new_recursion_no_crash` (`parser/stmt_tests.rs`). Each
asserts a **cap** message, so on the libtest thread (whose size follows `RUST_MIN_STACK`) the probe
could fire first and turn a cap assertion into a probe failure. Their cap+N halves run on 8 MiB;
their cap−10 halves are replaced by the 2 MiB `json_cap_minus_10_on_production_thread_*` cells, so
the 2 MiB derivation ratchet that `natives_json/mod.rs:40-42` names is kept, not silently dropped.
The comment at `tests_json.rs:594` ("Cells run on the libtest thread (2 MiB)") is rewritten to say so.

## §9. Mutants

Each row removes one probe (or reverts one structural fix) and names the cell that must die.
Run on the final head only, debug and release. None has been run yet. Placement mutants (a check moved
past the state it must precede) are P0a-1 §7.

| Mutant | Removed | Dies |
|---|---|---|
| M-CP1 | CP-1 | `rt_reviver_recursion`, `rt_map_recursion`, `rt_super_recursion`, `rt_cyclic_join_range_error` (abort) |
| M-CP2 | CP-2 | `rt_generator_resume`, `rt_inline_async` |
| M-W1 / M-W2 | W-1 / W-2 | `walker_floor_parse_array` / `walker_floor_parse_object` |
| M-W3 | W-3 | `walker_floor_internalize` |
| M-W4 / M-W5 | W-4 / W-5 | `walker_floor_stringify_array` / `walker_floor_stringify_object` |
| M-W6 | W-6 | `walker_floor_sc` |
| M-W7 | W-7 | `parser_parens_depth` (floor half) |
| M-W7b | W-7b | `parser_class_heritage_chain` |
| M-W8 | W-8 probe branch | `frontend_chain_scope_probe` (abort) |
| M-W8a | W-8 cap branch reverted to the silent skip | `frontend_chain_scope_cap` (no error; `q` unbound) |
| M-W9e / M-W9s / M-W9d | W-9 at `compile_expr` / `compile_stmt` / `compile_destructure_pattern` | `frontend_chain_compile_probe` / `frontend_compile_probe_stmt` / `frontend_compile_probe_destructure` (abort) |
| M-W9f | W-9 at `compile_nested_function` | **equivalent** behaviourally (each nested-function level also passes `compile_expr` / `compile_stmt`); recorded; the cut check (M-CC) still sees the removal |
| M-W10 | W-10 | `flat_cyclic`; reverting to `MAX_FLAT_DEPTH` kills `flat_infinity_deep` |
| (no M-W11) | — | W-11 is not a PR-1 change (PM 454); reverting the slot's flattening kills `abort_any_chain_depth_one_precondition` (A-1), which is the check, not a PR-1 mutant |
| M-W12 | W-12 | `walker_floor_idb_storable` |
| M-RX1 | front-door `ensure_stack` | `rx_alternation_1e5` (and not `rx_nesting_repro`, which the cap answers: I-6 tells them apart) |
| M-RX2 | front-door cap | `rx_nesting_repro` changes message to `STACK_EXHAUSTED_MSG` |
| M-RX3 | `REGEXP_GROUP_COST` halved | `rx_group_cost_ratchet` |
| M-RX4 | one site bypasses the front door | trip-wire §6.4 |
| M-RX5 | weight = total `|` count (the R0 rule) | `rx_concat_groups_depth_one` |
| M-CC | one probe removed from a census cycle | the tracked cut check (§5.3), in addition to that probe's cell |
| M-SC | `clone_regexp` re-compiles through the front door instead of copying | `sc_regexp_needs_no_compile_room` |

## §10. Hot-path probe cost

Measured 2026-11-06 on the prototype, release, macOS aarch64, two builds of the same clone that
differ only in the probe body (`SG_MODE` compile-time switch in `H/prototype/stack_guard.rs`):
`none` = probe compiled out, `lean` = the PR-1 shape (one TLS read of the cached bound, SP read,
compare against a const reserve). **Scope of what was measured**: CP-1, CP-2, W-1…W-6 only. The
front-end probes (W-7…W-9), `flat_into` (W-10), the IndexedDB check (W-12) and the regexp front
door were not measured. Command: `H/scripts/hotpath.sh 7` (7 interleaved runs per build, in-script
`Date.now()` around the workload only, 8 MiB thread), log `H/logs/rel-hotpath.log`. The host was
loaded (load average 20–80, other sessions building).

| Workload | Probes hit | `none` min (ms) | `lean` min (ms) | Δ min | Run spread (max/min − 1), none / lean |
|---|---|---|---|---|---|
| `cb`: `forEach` over 10⁶ elements × 5 | 5·10⁶ × CP-1 | 497 | 499 | +0.4 % | 32 % / 30 % |
| `gen`: `it.next()` × 3·10⁶ | 3·10⁶ × CP-2 | 1371 | 1360 | −0.8 % | 62 % / 63 % |
| `json`: parse + stringify of 10⁵ × 4-deep records, × 5 | ≈4·10⁶ × W-1…W-5 | 999 | 1004 | +0.5 % | 42 % / 54 % |
| `sc`: `structuredClone` of the same, × 5 | ≈2·10⁶ × W-6 | 354 | 360 | +1.7 % | 64 % / 74 % |
| `plain` (control: inline JS calls, no probe on the path) | 0 | 2316 | 2255 | −2.6 % | 22 % / 33 % |

With run spreads of 22–74 %, the minima are the only usable statistic, and the control's −2.6 %
says differences of that size are noise on this host. Every probed Δ is inside it, so on this host
the cost is below what can be resolved; that is the whole claim, not "≤ 3 %".

**Acceptance for PR-1** (re-run on the final head, production probe vs a build with `ensure_stack`
stubbed to `Ok(())`, same method), recorded in the PR body:
- **Host condition**: no other build or test running; 1-minute load average ≤ 2 sampled before and
  after the run (both values recorded); ≥ 7 interleaved runs per build.
- **Workloads**: the five above plus `parse` (compile of a 10⁴-statement generated script, × 5;
  W-7…W-9), `flat` (`flat(Infinity)` over 10⁵ × 3-deep arrays; W-10) and `rxlit` (a regexp literal
  evaluated 10⁶ times; the front door's weight pass + gate). (`wasmcall`, CP-3a's workload, is
  PR-1b §8's.) W-12: an IndexedDB put of 10⁴ records if the harness runs it; otherwise W-12 is
  listed as unmeasured with the structural bound that stands in: one probe per level of a walk over the
  clone's output, which W-6 already probed per level, so W-12 at most doubles the `sc` row's probe count.
- **Band and verdict**: the noise band is the control row's |Δ min| on that run (floor 1 %). A
  probed row whose Δ min exceeds the band blocks; the PR then states the cost and the PM decides
  between accepting it and a fast path (I-5 says none is planned).

## §11. Measurements recorded for this plan

- **M-AB** `AbortSignal.any` chain, debug, guard off, 2 MiB: kept intermediates, N = 10 / 100 / 1000 /
  3000 → `true` / `true` / abort / abort; dropped intermediates, N = 10 000 → `false`
  (`H/logs/dbg-census-holes.log` and follow-up runs in the R0 session). Evidence for the W-11 slot and the
  A-1 cell; PR-1 does not change it.
- Regexp axes: `H/logs/{rel-rx-axes,rel-rx-axes2,dbg-rx-axes,rx-shapes-dbg,rx-concat-alt,rx-concat-alt-rel}.log`.
  Class heritage: `H/logs/rel-classext.log`, `H/logs/dbg-census-holes.log`. Cut check:
  `H/logs/cutcheck-pr1.log` (before W-7b / W-9), `H/logs/cutcheck-pr1-v2.log`.
- cap−10 on 2 MiB with the probe on (debug): `H/logs/cap10-on-dbg.log` (`WALK=<w> CAP=590 <debug cd_probe>
  walker 1 2097152`, w ∈ parse stringify internalize sc; lowest remaining at a probe 898 895 / 604 175 /
  397 807 / 371 519 B).
- Left-deep chains: umbrella §8 M-8. Forced fallback: M-9. Bound source and span: M-5. JS→wasm: PR-1b §9.

## §12. Landing obligations

Every row is done by the **PM / PMO** at PR-1's landing (PM 452: the ledger is the PM's) unless marked
*PR* (done by PR-1's PR itself, in the repo) or *PM* (a decision or a lane notice). This memo edits no
ledger.

- **L-1** (PMO). `#11-vm-call-depth-guard` gains: "Rust-stack choke points CP-1 / CP-2 and the walkers
  landed in PR-1 (sha); the JS→wasm entries remain open until PR-1b; the tracked cut check
  (`.claude/tools/recursion_cutcheck.py`) keeps the coverage claim." Closure: PR-1b §10 L-1.
- **L-2** (PMO). `#11-vm-json-walkers-iterative`: "trigger fired at PR-1 (sha); not taken: probe-only
  touch; trigger 2 stays live (the `json_cap_minus_10_on_production_thread_*` cells)". PR-1's body says so.
- **L-3** (PMO). `#11-vm-array-join-cycle-detection`: "under PR-1 (sha) a cyclic `join` gives the
  catchable `RangeError` (cell `rt_cyclic_join_range_error`)".
- **L-4** (PMO). `#11-vm-native-callback-reentry-single-loop`: "the ~35–47 debug / ~270–470 release figures
  were abort depths; under PR-1 the callback depth ends in a `RangeError` at d≈30–42 / 250–409 (umbrella
  §8 M-1), a figure that depends on `WALKER_RESERVE` as well as frame size".
- **L-5** (PMO). `#11-vm-js-frame-depth-limit` keeps only the plain-recursion facet; the Promise facet is
  `#11-vm-microtask-self-chaining-unbounded` (PM 483, PM 490). No PR-1 change.
- **L-6** (PM → L3; the entry is L3's). `#11-dom-walkers-iterative-over-ecs-links` gains: "until it lands,
  the DOM walkers' only headroom near the limit is PR-1's `WALKER_RESERVE` (256 KiB);
  `stack_guard_reserve_margin_ratchet` (64 KiB) runs no DOM walker, so a DOM-walker frame-size change is
  not ratcheted". Soft edge PR-1 → that slot; schedule: P0 umbrella §1.
- **L-7** (*PR*). Before landing, `git grep -n '§3\.1\.3' -- crates/script/elidex-js/src/vm/host/abort_statics.rs`
  → 0 lines (PM 458), beside the A-1 ancestry check; the PR body reports either result.
- **L-8** (*PR*). Slice-plan row 8 (`docs/plans/2026-07-vm-p4-slice-plan.md`, `:976`) gains the pointer
  "the RegExp creation / construction child adopts `vm/regexp_compile.rs`'s `compile_regexp` (stack-safety
  guard PR-1 §6.1)", the `crate::regexp` re-check trigger of §5.3, and the `v` obligation of §6.2 (also
  on `#11-vm-regexp-v-flag-dropped`, PMO). Then `python3 .claude/tools/plan-memo-umbrella-check.py
  docs/plans/2026-07-vm-p4-es-language-completeness.md` (rc 0) and `bash scripts/python-suites.sh`.
- **L-9** (*PR*). Breadth record: `python3 .claude/skills/elidex-plan-review/preflight.py` on the umbrella,
  this memo and PR-1b at the PR's head; rc and K / M as printed (hand counts: umbrella §3).
- **L-10** (PMO). `#11-vm-frontend-left-spine-iterative` gains the defect PR-1 leaves: "PR-1 makes long
  left-deep chains a compile failure instead of an abort or silent wrong code; the stated bands are PR-1
  §5.4 (release 1025–1676 `+` terms; debug ≈330–382 `+`, ≈270–314 `f()()`)".
- **L-11** (*PR*). `#11-vm-structured-clone-layering-seam` is a pointer row whose text is
  `docs/plans/2026-07-vm-p4-slot-ledger.md` row :332 (a VM P4 family memo). PR-1's PR appends there the
  stale-cite list of §3.3 and the three hand-offs of §5.5, then runs `python3
  .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md` (rc 0)
  and `python3 docs/plans/test_vm_p4_family_layout.py` (CLAUDE.md, VM-P4 checker paragraph).
- **L-12** (*PR*). The release step and its reason go in the `check` job's comment in
  `.github/workflows/ci.yml`, in the same commit as the step (PM 490); CLAUDE.md is not changed.
- **L-13.** Moved to PR-1b §10.
- **L-14** (PMO, and *PR* for the repo half). The defect, not a requirement: "a RegExp crossing a thread,
  process or storage boundary cannot carry its matcher; its deserialize must re-compile, and after
  stack-safety PR-1 the only compile path is `compile_regexp` (PR-1 §6.3); HTML §2.7.7 step 12 has no
  failure step, so a `RangeError` there is a stated deviation". Targets: `#11-idb-structured-clone-storage`
  (PMO); `#11-history-state-structured-serialize-fidelity`, a pointer row whose text is
  `docs/plans/2026-07-s5-5c-history-state-traversal.md:868` (PR-1's PR appends it there).
- **L-15** (PMO). `#11-vm-native-callback-reentry-single-loop` gains the cut check's stated limits (§5.3:
  trait-object, cross-function closure and function-pointer recursion; the hand-picked M-1 classes), beside
  P0's (P0 umbrella §7 L-7); Re-eval-date 2026-12-31.
- **L-16** (PMO). Pointer lines, decisions recorded here so they are not re-made:
  `#11-render-pipeline-recursion-depth-abort` "if a remaining-stack probe is chosen, it is the one
  primitive `crate::stack_guard`, moved to a crate both can depend on (umbrella §1.2)";
  `#11-vm-function-constructor-global` "`StackExhausted` from the compile `eval` / `Function` perform maps
  to a `RangeError` thrown at the call (stack-safety PR-1 §4.1)".
- **L-17** (PM). Slot `#11-vm-inner-substruct-decomposition` (registered PM 517; owner VM P4): the `VmInner`
  sub-struct proposal (memory `vm-inner-substruct-deferral`, trigger 1 fired: `vm/mod.rs` 2785 lines, §5.1).
