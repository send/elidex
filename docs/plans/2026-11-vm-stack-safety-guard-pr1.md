# VM stack-safety guard — PR-1 plan (primitive, choke points, walkers, regexp front door)

Status: **plan (pre-implementation), revised after plan-review R1 and the driver decisions D-W
(wasm entry, §4.4), D-SC (structuredClone matcher copy, §6.3) and the split verdicts (§5.1)**; `/elidex-plan-review`
required before code (edge-dense, umbrella §2). Parent: [umbrella](2026-11-vm-stack-safety-guard.md).
Closes the Rust-stack half of `#11-vm-call-depth-guard`. Base: `47b6c934` (anchors below are at
that commit). Rulings PM 421 (a)/(b), PM 424, PM 436 R1/R2/R3/extra/split, PM 454, PM 476 are
ratified (umbrella §0.1) and not re-opened here. Provenance root `H` is defined in the umbrella
header; every measurement names its log under `H/logs/`.
Preconditions: the W-11 slot `#11-abort-signal-any-dependent-flattening` has landed (umbrella §4
edge; §2 A-1), and the `interpreter.rs` prereq split has landed (§5.1). Ordering against Slice F
(`#11-vm-structured-clone-file-split`), NativeError E1, and the open umbrella
`#11-vm-structured-clone-layering-seam`: ⟨PM pending: P2⟩ (facts in §5.5).

**Base-case reason (single PR).** PR-1 is a narrowly scoped per-PR slice under the approved
umbrella that passes its own plan-review; under CLAUDE.md's edge-dense rule (c) that makes it a
terminal unit, so it is not re-split for touching the same subsystem as its siblings.

**Relation to S1.** S1 (`elidex-wt-strrepl`, `#11-vm-string-replace-function-replacement`) rewrites
`natives_string.rs` :466-543, which shifts this plan's `natives_string.rs` anchors. PR-1 re-derives
every anchor at its own base with the commands given beside each table (§5.1, §6.1), not from the
numbers written here. The stale `§21.1.3.15` cite at `natives_string.rs:659` (search's lastIndex
comment) is not PR-1's: it belongs to the OD-2 / slice 9b re-tag, and PR-1 leaves it.

## §1. Scope

PR-1 delivers, in one PR:

1. The primitive `crate::stack_guard` (NEW) (§4), with its test-only seams (§4.5).
2. Probes at the two re-entry choke points and at the two JS→wasm entries (CP-3), and wasmtime's
   `max_wasm_stack` set from the one constant the CP-3 probe asks for (§4.4, §5.1).
3. Probes in every unbounded `elidex-js` walker (§5.2): JSON parse / internalize / stringify,
   structuredClone, `flat_into` (replacing `MAX_FLAT_DEPTH`), the parser / scope analyser /
   compiler recursive entries, the IndexedDB storable pre-check; the scope analyser's cap branch
   gets an error exit and stops leaking depth (W-8).
4. The regexp-compile front door, the routing of the five compile sites that stay compile sites
   (the sixth, structuredClone's re-compile, becomes a copy of the compiled matcher, §6.3), and the
   I-7 trip-wire (§6).
5. The PM 424 rewording of the `MAX_JSON_DEPTH` docstring (§7).
6. Cells, mutants, the explicit-thread rule, the hot-path measurement (§8–§10).
7. The recursion cut check as a tracked CI check (§5.3) and the in-repo constant re-measurement
   driver (§6.2).

Not in PR-1: the frame cap (PR-2); everything in umbrella §1.2 — in particular the
`AbortSignal.any` source-signal flattening (the W-11 slot's, lands before PR-1, PM 454) and
iterating the left spine of long expression chains (§5.4, its own slot; registration
⟨PM pending: P8⟩).

## §2. Coupled invariants

The umbrella §2 table (I-1 coverage, I-2 fail-safe bound, I-3 state integrity, I-4 semantic caps,
I-5 hot-path cost, and each pair) is this PR's invariant set; PR-1 is where all five are realised.
**I-3, scoped**: it holds for **VM** state (`call`, `call_construct_native`, `run_function`,
`construct_synchronous`, `dispatch_super_inner` mutate nothing before `with_call_mode`, R1 Axis 2)
and for the **wasm store / instance** behind CP-3 (§5.1): the probe sits before the lazy
`vm_wasm_runtime()` initialisation, `Store::new` and the start function (CP-3b), and before the
`HostState` bind and the `set_fuel` reset (CP-3a). The one VM field written before CP-3a,
`call_dispatch`'s `gc_enabled = false` in the `WasmExportedFunction` arm (`interpreter.rs:501`), is
restored by that arm on the `Err` return as on every other return.
Two non-VM consequences are accepted and stated:

- *Custom-element upgrade.* `host/custom_elements/upgrade.rs` sets the element's state to
  precustomized, creates the wrapper and pushes the construction-stack guard before it calls
  `construct_synchronous` (`upgrade.rs:277`). A CP-1 failure there returns `Err` into the upgrade,
  which runs `finalize_failure_shim` and leaves the element Failed. That is HTML §4.13.5 *upgrade an
  element* step 10's "if the above steps threw an exception" branch (definition → null, reaction
  queue emptied, rethrow; state stays "failed" or "precustomized") — the same path a throwing
  constructor takes today, whose remaining divergence is `#11-ce-upgrade-returned-object-typeerror`.
  The cause differs (caller stack depth, so a `customElements.define()` reached from deep recursion
  can fail a whole upgrade batch), and the outcome is the spec's for an exception. Cell
  `ce_upgrade_probe_failure_takes_threw_branch` (§8).
- *Async functions.* On the inline async path CP-2 fails inside `drive_async_coroutine`
  (`natives_generator.rs:460`), which turns the `Err` into a rejection of the wrapper promise and
  returns `Ok(())` (`natives_generator.rs:518-526`); the generator stays SuspendedStart / SuspendedYield.
  On a later `AsyncDriverStep` resume that fails the same way, the `RangeError` settles the wrapper
  but is not injected into the body, so a `try` / `finally` in the body does not run. Reachability
  is low (the microtask drain that runs the step starts shallow) and the wrapper's settlement is
  correct; recorded, not engineered around. Cell `rt_inline_async` covers the first case.

PR-local invariants:

- **I-6 One error shape per channel.** *Runtime channel*: every probe failure is a `RangeError`
  with `STACK_EXHAUSTED_MSG`, whatever the walker; cap failures keep their own messages
  (`DEPTH_EXCEEDED_MSG`; the regexp nesting message, §6.2). *Compile-time channel* (parser, scope
  analyser, compiler; no VM exists yet): every probe failure is one error kind,
  `JsParseErrorKind::StackExhausted` (NEW) in the parser and scope analyser, and a `CompileError`
  whose message is `STACK_EXHAUSTED_MSG` in the compiler (`From<StackExhausted> for CompileError`);
  cap failures are `JsParseErrorKind::NestingTooDeep` in the parser and (new, W-8) in the scope
  analyser. How §9 tells cap from probe: runtime cells compare the message; compile-time cells
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
| I-6 × I-3 | The error object is built after `ensure_stack` returned `Err` and before any VM mutation of the guarded operation; its construction runs inside the reserve (I-1 × I-2). |
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
| WebAssembly JS API §5.6 Exported Functions | *call an Exported Function* step 8 `func_invoke` (CP-3a sits after step 7's `ToWebAssemblyValue` coercions, before the store is bound); step 10's exception is elidex's resource limit, not a trap | exported function call | `call_wasm_exported_function` (CP-3a) | ✓ | yes |
| WebAssembly JS API §5 The WebAssembly Namespace | *instantiate the core of a WebAssembly module* step 2 `module_instantiate` (runs the start function); step 3's third bullet ("another error type if appropriate") is where the `RangeError` lands | sync `Instance` ctor / async `instantiate` (rejection) | `instantiate_module` (CP-3b) | ✓ | yes |

**Breadth**: K=4 specs (ECMA-262, WHATWG HTML, IndexedDB-3, WebAssembly JS API), M=20 entries
(verified 2026-11-08 by `preflight.py`: M=20, K=4, `IndexedDB-3` and `WebAssembly JS API` unmapped labels whose
rows are not webref-verified; the two wasm rows' anchors were looked up by hand with
`webref dfn wasm-js-api-2 'call an Exported Function'` → §5.6 `#call-an-exported-function` and
`webref dfn wasm-js-api-2 'instantiate the core of a WebAssembly module'` → §5
`#instantiate-the-core-of-a-webassembly-module`). The R1 revision's changes against the R0 table
(which, like the umbrella's, said M=12 with different rows): §25.5.2 → §25.5.2.1 ParseJSON; added
JoinIndexedProperties, RegExpCreate, RegExpInitialize, StructuredDeserialize, §2.7.11 re-labelled,
IndexedDB-3 §5.11; D-W added the two WebAssembly JS API rows; rows PR-1 does not touch are in
umbrella §3.2. `preflight.py`'s own figures are recorded again at landing (§12 L-9).
**Split decision**: K=4 and M=20 → `preflight.py` reports SPLIT-RECOMMENDED (K≥4 or M≥20). Both
thresholds are crossed by the D-W rows alone (without them K=3, M=18). ⟨driver pending: D-K — split
CP-3 + `MAX_WASM_STACK` into a PR of its own, or keep PR-1 single with a positive single-PR
appeal written here⟩

### §3.1 User-input touch audit

All rows are script-controlled. Adjacent pre-existing code (measurements in umbrella §3.1 / §8 M-8):
- Left-deep chains: the compiler (no cap) aborts at 383 terms debug / 1677 release on 2 MiB today;
  the scope analyser's cap branch skips silently and leaks depth past 1025 terms, giving wrong code
  for everything after the chain. PR-1 gives the scope cap an error exit and the compiler a probe
  (§5.2 W-8, W-9); chain length is §5.4.
- The four `natives_string.rs` sites and the literal site map a `regress` syntax error to
  `TypeError` (spec: `SyntaxError` / early error). Unchanged; the front door returns the syntax
  error to the caller untouched (§6.3). Home: a child of VM P4 slice-plan umbrella 8 (RegExp
  completion), owner VM P4 (PM 454; slot id ⟨PM pending: P4⟩).
- `AbortSignal.any` chains whose intermediate composites are unreachable lose propagation after a
  GC today (chain of 10 000 with no references kept → `s.aborted === false` after `c.abort()`;
  §11 M-AB). Not PR-1's: the W-11 slot's flattening removes the intermediate links before PR-1
  (PM 454).

### §3.2 What each new surface's docstring cites

| Surface (NEW) | Cites |
|---|---|
| `crate::stack_guard` module doc / `ensure_stack` | ECMA-262 §17 (no resource-limit deviation: the limit is elidex's, PM 424); I-2's fail-early rule; the stack-switching constraint (§4.4) |
| `STACK_EXHAUSTED_MSG` | none (an elidex message) |
| `vm/regexp_compile.rs` / `compile_regexp` | ECMA-262 §22.2.3.3 RegExpInitialize steps 14–15 / 22 (what is parsed and compiled) and §22.2.3.4 ParsePattern (what the weight pass approximates); the resource-limit status of `TooDeep` / `Exhausted` (no spec step throws them) |
| `From<StackExhausted> for VmError` / `CompileError`, `From<RegExpCompileError> for VmError` | none beyond the I-6 channel they belong to (they are mappings, not algorithms) |
| `MAX_REGEXP_DEPTH` (`crate::regexp`) | the semantic-cap role (I-4) and its two users; no spec section (no spec defines it) |
| `REGEXP_GROUP_COST` / `REGEXP_ALT_COST` | the ratchet cells and the in-repo driver that re-derive them (§6.2) |
| `elidex_wasm_runtime::MAX_WASM_STACK` | wasmtime's `Config::max_wasm_stack` contract (wasm's own stack use, not the thread's); the CP-3 probe that asks `WALKER_RESERVE + MAX_WASM_STACK`; the measurement that sizes it (§4.4); no spec section (WebAssembly JS API §8 Implementation-defined Limits defines none for stack: "an implementation may run out of resources for valid modules below these limits") |
| CP-3a / CP-3b site comments | WebAssembly JS API §5.6 *call an Exported Function* step 8 / §5 *instantiate the core of a WebAssembly module* step 3 (where the resource-limit `RangeError` lands), and I-3's placement rule |
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

/// Message shared by every conversion (I-6).
pub(crate) const STACK_EXHAUSTED_MSG: &str = "Maximum call stack size exceeded"; // (NEW)
```

Conversions (all NEW): `From<StackExhausted> for VmError` → `VmError::range_error(STACK_EXHAUSTED_MSG)`;
`From<StackExhausted> for CompileError` (message `STACK_EXHAUSTED_MSG`); a new
`JsParseErrorKind::StackExhausted` (NEW) variant pushed by the parser / scope probe branches.
There is exactly one primitive (PM 436 R1): walkers and choke points call
`ensure_stack(WALKER_RESERVE)?`; the regexp front door calls `ensure_stack(WALKER_RESERVE +
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
  green at PR-1's base (if cargo-deny's graph lists the inactive optional fiber crate, that entry is
  replaced by a `wasmtime` feature-set check in the same job, and the PR says which). Adding any
  of them then fails `deny` until the guard is redesigned for it.
- **wasm** (decision D-W). wasm runs synchronously on the VM thread: `elidex-wasm-runtime` calls
  wasmtime on the caller's stack in exactly two places reachable from script, `WasmFunc::call`
  (`handle.rs:220`, an exported-function call) and `WasmRuntime::instantiate` (`runtime.rs:165`,
  `Linker::instantiate`, which runs the module's start function); no async, no fiber (re-derive:
  `git grep -nE '\.call\(|\.instantiate\(' -- crates/script/elidex-wasm-runtime/src` → those two
  plus `host/funcs.rs:62`, the `__alloc` call a host function makes from inside an activation that
  is already running). wasmtime bounds wasm's own stack use with `max_wasm_stack`, measured from
  the SP at the store's outermost entry (`wasmtime` 50.0.1 `runtime/func.rs:1535-1583`: a nested
  entry into the same store keeps the limit already set), and its documentation (`config.rs`,
  `max_wasm_stack`) states that the thread is not checked: with less than that remaining the
  process aborts. elidex leaves it at the default today (`runtime.rs:76` sets only `consume_fuel`;
  the default is `512 * 1024`, `config.rs:295`).
  **Measured** (`H/scripts/wasm_entry.sh`, `H/logs/wasm-entry.log`; prototype guard on, 256 KiB,
  2 MiB thread; a module whose start function recurses without end, entered through
  `new WebAssembly.Instance` at callback depth D): at the top both profiles give a catchable
  `RuntimeError` (wasmtime's stack-overflow trap); debug D = 24 / 26 (lowest remaining at a probe
  739 / 630 KiB) → `RuntimeError`, D = 28 / 30 / 32 → **process abort**, D = 33 (the CP-1 floor) →
  `RangeError` from CP-1; release D = 200 / 220 (731 / 595 KiB) → `RuntimeError`, D = 240 / 250 /
  260 → **abort**, D = 269 (floor) → `RangeError`. So with PR-1 as planned before D-W, a wasm entry
  in the band between the CP-1 floor and ≈600 KiB remaining aborts the renderer: I-1 did not hold.
  **Decision**: PR-1 adds the choke point **CP-3** at both JS→wasm entries (§5.1), asking
  `ensure_stack(WALKER_RESERVE + MAX_WASM_STACK)`, and sets `max_wasm_stack` explicitly from that
  constant. **`MAX_WASM_STACK`** (NEW) = `512 * 1024`, `pub const` in `elidex-wasm-runtime`
  (`lib.rs` re-export; `runtime.rs:76` calls `config.max_wasm_stack(MAX_WASM_STACK)`), imported by
  `elidex-js` under `feature = "engine"` (the dependency direction that exists; `elidex-js`'s
  `pub(crate)` `crate::stack_guard` is not exported, and does not need to be). One constant, two
  readers, so the ask and wasmtime's limit cannot drift apart. **Why 512 KiB**: it is wasmtime's
  default, so wasm's own recursion depth is what it is today, and the measurement above brackets
  the real need of an entry that runs to wasmtime's limit at more than 512 KiB and at most
  ≈ 595 KiB (release) / ≈ 630 KiB (debug) of remaining stack — inside `WALKER_RESERVE + 512 KiB`
  = 768 KiB, the excess over 512 KiB (entry, trap unwinding, host functions reached from wasm,
  which wasmtime does not limit) being what `WALKER_RESERVE` is for. **Cost**: a JS→wasm call needs
  768 KiB free, so on a 2 MiB thread it gets the `RangeError` from callback depth ≈ 23 (debug) /
  ≈ 195 (release) instead of the CP-1 floor's 33 / 269 (interpolated from the same log's lowest
  remaining per depth). A smaller constant would buy JS depth by cutting wasm's recursion depth
  below today's; that is a tuning decision for whoever has a wasm workload that needs it, not a
  guard decision.
  **wasm → JS re-entry does not exist today**: `WasmRuntime::instantiate` rejects a non-empty
  `ImportObject` (`runtime.rs:130`), `instance.rs` `coerce_import_object` builds an empty one for
  every input, and the only functions wasm can call are the `elidex` host functions
  (`host/funcs.rs`), which reach the DOM through `DomHandlerRegistry` and hold no VM handle
  (`elidex-wasm-runtime` does not depend on `elidex-js`). So no JS→wasm→JS→wasm recursion is
  constructible at PR-1's base, and CP-1 is not on any wasm path yet. When
  `#11-wasm-user-import-host-fn-builder` lands JS imports, the import trampoline must enter JS
  through `with_call_mode` (CP-1), and each JS→wasm entry it leads to passes CP-3 again; this is
  required, not optional, because each `WasmInstance` has its own `Store` (`runtime.rs:144`) and
  wasmtime sets a fresh `max_wasm_stack` limit at each store's outermost entry, so a nested entry
  into another instance gets a full 512 KiB of its own (§12 L-13).

### §4.5 Test-only seams

Two seams, neither changing the production or benchmark shape of the hot path (§10 measures a
non-test release build):

- **Reserve override**: under `cfg(test)`, `WALKER_RESERVE` call sites read
  `stack_guard::walker_reserve()`, which returns a `thread_local!` override (default 256 KiB);
  under `cfg(not(test))` the same name is the `const`. The test build therefore has one extra TLS
  read per probe, the production build none. The 64 KiB ratchet sets the override on its thread.
- **Forced-unknown bound**: under `cfg(test)`, a `thread_local!` flag consulted only in the lazy
  init (once per thread, off the hot path) makes the OS query report unknown. A test-only accessor
  returns the bound's source (OS / fallback) and span for the bound cells.

## §5. Call sites

### §5.1 Choke points

| Id | Site | Placement | Why this covers it |
|---|---|---|---|
| CP-1 | `VmInner::with_call_mode` (`crates/script/elidex-js/src/vm/interpreter.rs:160`) | first statement, before the `entry_frames` / `entry_stack_depth` snapshot | every native→JS call (`call`, `call_construct_native`, `run_function`) and `construct_synchronous` (`crates/script/elidex-js/src/vm/dispatch_class.rs:323`) enter through it; the nested `run()` sites at `interpreter.rs:740` and `dispatch_class.rs:364` are inside it |
| CP-2 | `VmInner::resume_generator` (`crates/script/elidex-js/src/vm/natives_generator.rs:114`) | first statement, before the generator state is taken and marked Running | the nested `run()` at `natives_generator.rs:276`; reached both from the generator natives (`natives_generator.rs:374/404/432`) and, outside `with_call_mode`, from inline async calls (`push_js_call_frame` → `drive_async_coroutine` → `natives_generator.rs:472`) |
| CP-3a | `call_wasm_exported_function` (`crates/script/elidex-js/src/vm/host/wasm/exported_func.rs:268`), asking `WALKER_RESERVE + MAX_WASM_STACK` | after the argument coercions (*call an Exported Function* step 7, which may run user JS through CP-1) and the bound-session check, immediately before `host_data.with_session_and_dom(… func.call …)` (`:306`), i.e. before the `HostState` bind and `set_fuel` reset in `WasmFunc::call` | the only caller of `WasmFunc::call` in `elidex-js` (its one caller is `call_dispatch`'s `WasmExportedFunction` arm, `interpreter.rs:511`); every exported function, however obtained (`instance.exports`, a funcref read from a table), is an `ObjectKind::WasmExportedFunction` dispatched there |
| CP-3b | `instantiate_module` (`crates/script/elidex-js/src/vm/host/wasm/instance.rs:151`), same ask | after `coerce_import_object` (*Read the imports*, `:162`), before `ctx.vm.vm_wasm_runtime()` (`:176`, lazy runtime init) and `runtime.instantiate` (`:180`, new `Store` + start function) | the only caller of `WasmRuntime::instantiate` in `elidex-js`; both the sync `new WebAssembly.Instance` (`instance.rs:69`) and the async `WebAssembly.instantiate` (`namespace.rs:578`, which turns the `Err` into a rejection) go through it |

Enumeration of nested `run()` (re-derive at PR-1's base: `git grep -nE '(self|vm|inner|ctx\.vm)\.run\(' -- crates/script/elidex-js/src/vm ':!**/tests/**'`): 3 sites, lines `dispatch_class.rs:364` / `interpreter.rs:740` / `natives_generator.rs:276`; each is behind CP-1 or CP-2.
Callers of `with_call_mode` outside tests (`git grep -n 'with_call_mode(' -- crates/script/elidex-js/src/vm`): `interpreter.rs` lines 82/102/1010 and `dispatch_class.rs:323`.
JS→wasm entries (re-derive: `git grep -nE 'func\.call\(|\.instantiate\(&' -- crates/script/elidex-js/src ':!**/tests/**'`): `exported_func.rs:307` and `instance.rs:180`, i.e. CP-3a and CP-3b; no other crate calls into `elidex-wasm-runtime` (`git grep -ln elidex_wasm_runtime -- crates ':!crates/script/elidex-js' ':!crates/script/elidex-wasm-runtime'` → nothing). Both entries are inside `elidex-js`, so `crate::stack_guard` reaches them; the only `elidex-wasm-runtime` change is the `MAX_WASM_STACK` constant and the `Config` call (§4.4). CP-3 is not a re-entry choke point (no nested `run()`): it guards a foreign stack consumer whose own limit is a fixed size, which is why its ask is that size plus the reserve.

**`interpreter.rs` cohesion verdict (CLAUDE.md 1000-line rule).** 1367 lines: source 1–1043, then an
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
each by one line. (The touch-time split record lists `vm/object_kind.rs` as *not on any watch list*,
with umbrellas 6, 7 and 10 owing it a verdict through their children, and does not list `vm/mod.rs`
at all, so neither verdict is owned elsewhere; this plan states both.)

- **`vm/object_kind.rs`, 1645 lines: no real cohesion seam → no prereq split.** Shape at
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
- **`vm/mod.rs`, 2785 lines: no real cohesion seam → no prereq split.** Shape at `47b6c934`: the
  submodule declarations and two constants (1–183), the `VmInner` struct definition (184–2773:
  346 fields, 1937 of those lines `///` docs, 287 attributes, no function bodies), and the `Vm`
  wrapper struct (2775–2781) with a trailing comment. A struct definition cannot be divided between files; the only seam is
  grouping `VmInner`'s fields into subsystem sub-structs, which memory
  `vm-inner-substruct-deferral` records as an architectural refactor (split borrows in the GC mark
  phase) rather than a file split, deferred with no slot. That memory's trigger 1 ("`vm/mod.rs`
  grows past ~1500 LoC") has fired (it was written at 1116 lines); that is a fact for the PM's
  ledger, not a PR-1 prerequisite, since PR-1 adds no field. **The touch**: one `mod
  regexp_compile;` line. It is avoidable only by homing the front door outside `vm/` — e.g. as
  a `compile` submodule of `crate::regexp` beside `MAX_REGEXP_DEPTH` — and that is worse: `crate::regexp` is the
  front end's static-semantics module (the lexer's flag parser, the unconnected pattern parser),
  while the front door is a runtime-channel path (I-6) whose callers are all VM sites and whose
  `From<RegExpCompileError> for VmError` would make the front-end module import the VM's error
  type. So the front door stays at `vm/regexp_compile.rs` and the one `mod` line stays.

**host/ touch table.** PR-1 touches four `vm/host/` files. All are engine-bound per the Layering
mandate: the first two walk VM `ObjectId` graphs, the last two marshal JS values across the
wasm boundary, and the change adds a stack check (and, in `structured_clone.rs`, replaces a
re-compile with a copy), no algorithm.

| File | Change | Engine-bound reason |
|---|---|---|
| `crates/script/elidex-js/src/vm/host/structured_clone.rs` | W-6 probe at `clone_recursive`; `clone_regexp` copies the compiled matcher instead of re-compiling (§6.3) | the clone walks `VmInner` objects; where the algorithm half ends up is the layering seam's (⟨PM pending: P2⟩) |
| `crates/script/elidex-js/src/vm/host/indexeddb/value.rs` | W-12 probe at `reject_non_json_storable` | walks the VM value produced by the clone |
| `crates/script/elidex-js/src/vm/host/wasm/exported_func.rs` | CP-3a probe | a stack check at the JS→wasm boundary; the call itself stays in `elidex-wasm-runtime` |
| `crates/script/elidex-js/src/vm/host/wasm/instance.rs` | CP-3b probe | same, for instantiation |

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
  `H/scripts/recursion_census.py` + `H/scripts/cutcheck.py`, merged). Its probe set is **derived
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
  in the check's header and in the PR body.

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

**Home: its own slot, `#11-vm-frontend-left-spine-iterative` (proposed; registration
⟨PM pending: P8⟩), not PR-1.** Reasons: it is a compiler-algorithm change with its own edge matrix (short-circuit jump
patching for `Logical`, optional-chain short-circuit to the chain end, call argument spread,
source-map order, assignment targets such as `a.b.c = x`), unrelated to the guard's invariants; and
PR-1 is correct without it (the probes stay as the safety bound after it, as for the single-loop
successor). No ordering edge between the two.

**Regression PR-1 ships until that slot lands, stated**: release chains of 1025–1676 terms, which
compile today, become a compile failure (`NestingTooDeep`). Today those programs already analyse
nothing after the chain (wrong code, umbrella §3.1), so only a chain that is the last thing in its
script loses a working result. Debug chains of ~330–382 terms change from compiling to a compile
failure; above 382 they change from abort to a compile failure. When the slot (⟨PM pending: P8⟩) lands, the
`frontend_chain_*` cells (§8) move to nested-paren inputs of the same depth, since chains no longer
reach the probes.

### §5.5 Facts for ⟨PM pending: P2⟩

`structured_clone.rs` is 1077 lines; Slice F (`#11-vm-structured-clone-file-split`) is its
prereq split, and the Slice F memo hard-codes the file's length and line ranges, which PR-1's W-6
probe and `clone_regexp` change would move. NativeError E1 also edits `clone_recursive` (the
ledger's edge F → E1). The open umbrella `#11-vm-structured-clone-layering-seam` (derivation
triggered at Slice F's plan-review) may move the clone algorithm out of `elidex-js`; then
`pub(crate)` `stack_guard` and `regexp_compile` are unreachable from it, §6.4's trip-wire (scoped
to `crates/**`) still sees it but the front door does not, and the W-6 / `:88` / `:585` anchors
move. Obligations PR-1 hands to wherever the algorithm lands: a probe per clone level (W-6), the
RegExp branch as a matcher copy (§6.3; a serializer that crosses a thread, process or storage
boundary re-compiles on deserialize through the front door instead), and I-6's runtime message.

## §6. Regexp-compile front door (PM 436 R1)

### §6.1 Compile sites

Re-derive at PR-1's base:
`git grep -nE 'regress::Regex|Regex::(with_flags|new|from_unicode)' -- crates/script/elidex-js/src ':!**/tests/**'`
— 6 compile sites at `47b6c934`: `natives_string.rs` 567/582/646/666, `dispatch_helpers.rs` 170, `structured_clone.rs` 585.
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

**What fits, from the gate** (not from the slope). A compile at SP with `remaining` bytes passes
iff `remaining ≥ WALKER_RESERVE + weight`. At the top of a 2 MiB thread, `remaining` ≈ 2048 KiB −
baseline, where baseline is the stack already in use at the compile (≈80 KiB debug / ≈32 KiB
release for a trivial script, `H/logs/rx-concat-alt*.log`). One flat disjunction (groups = 0)
therefore fits ⌊(2048 − 256 − 32) KiB / 256 B⌋ ≈ 7 000 alternatives in release and
⌊(2048 − 256 − 80) KiB / 1088 B⌋ ≈ 1 600 in debug. Pure nesting fits ⌊1760 KiB / 1664 B⌋ ≈ 1 080
groups in release, so the cap (1024, step 2) answers first there; in debug the gate answers first,
from ⌊1712 KiB / 6592 B⌋ ≈ 265 groups. **Stated regression**: release compiles a flat disjunction of up to ≈12 800 alternatives
today (by the measured slope); from ≈7 000 to ≈12 800 PR-1 gives a `RangeError` instead, because the
gate keeps the reserve free and carries the 1.25 factor. Alternation is not capped (only groups
are); the gate is what bounds it.

**Ratchet and re-measurement (the procedure is in the repo).** The cells `rx_group_cost_ratchet` /
`rx_alt_cost_ratchet` (§8) compile a pattern of D levels on a thread of exactly `baseline + R + D ×
cost`, where `R` is the test-only 64 KiB reserve (§4.5) and **`baseline`** is a recorded constant
beside each cell: the need of the same cell body at D = 1, 16 KiB resolution. The cell passes iff
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
(the class question belongs to umbrella 8's child, ⟨PM pending: P4⟩; §3.1).

- **Literal** (`load_constant`): §13.2.7.3 step 3 is `! RegExpCreate` — the spec says it cannot
  throw, because a bad pattern was already an early error (§13.2.7.1). A cap or probe throw here is
  a resource-limit deviation from that `!`, stated in the site's comment; the `Syntax` → `TypeError`
  path is the pre-existing early-error gap (umbrella §3.2), not this one.
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
  process or storage boundary gains RegExp support (`#11-worker-structured-serialize`,
  `#11-history-state-structured-serialize-fidelity`, `#11-idb-structured-clone-storage`), the live
  matcher cannot travel, so its deserialize step re-compiles from `[[OriginalSource]]` /
  `[[OriginalFlags]]` through `compile_regexp` (I-7 makes any other compile a CI failure; §12 L-14).

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
the same with `--release`. CI: the `check` job runs the debug leg on all three OSes; which OS ×
profile legs run the release leg, and its coordination with the CI/tools lane's in-flight work
(and the CLAUDE.md §CI text and `mise run ci`), is ⟨PM pending: P5⟩ (facts: umbrella §7). Until
P5 is decided, the release constants are exercised only where someone runs `--release` by hand.

| Cell | Body | Expect |
|---|---|---|
| `rt_reviver_recursion` | `function r(k, v) { return k === '' ? JSON.parse('[1]', r) : v } JSON.parse('[1]', r)` (declaration: a named function expression has no self-binding in elidex today) | `RangeError` / `STACK_EXHAUSTED_MSG` |
| `rt_map_recursion` | `function f() { [1].map(f) } f()` | same |
| `rt_generator_resume` | `function* g() { g().next(); yield 1 } g().next()` | same |
| `rt_inline_async` | `async function f() { await f() } f().catch(e => globalThis.r = e.name)`, then drain microtasks | `r === 'RangeError'` |
| `rt_super_recursion` | `class B { constructor() { new C() } } class C extends B { constructor() { super() } } new C()` | `RangeError` |
| `rt_cyclic_join_range_error` | `const a = []; a[0] = a; String(a)` | `RangeError` (elidex resource limit; JoinIndexedProperties has no cycle detection; PM 424) |
| `ce_upgrade_probe_failure_takes_threw_branch` | define a custom element from inside recursion that reaches the CP-1 floor, with an un-upgraded instance in the document | `RangeError` reaches the caller; the element's state is "failed", its definition null, its reaction queue empty (HTML §4.13.5 step 10's threw branch); no abort |
| `walker_floor_{parse,internalize,stringify,sc,flat,idb_storable}` | recurse through `[1].forEach` until the first `RangeError`, then run the walker on a cap−10 input at that depth | `RangeError` (probe), no abort |
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
| `frontend_chain_scope_probe` | 1000-term `+` chain: parse on the cell's thread, `analyze_scopes` on a thread of half its measured scope need (debug 384 KiB, release 64 KiB) | `StackExhausted` in the analysis errors, no abort |
| `frontend_chain_compile_probe` | 1000-term `+` chain: parse + scope on the cell's thread, `compiler::compile` on 2 MiB (debug; need 5.3 MiB) / 1 MiB (release; need 1.2 MiB) | `CompileError` with `STACK_EXHAUSTED_MSG`, no abort (aborts today) |
| `flat_infinity_deep` | `let a = [1]; for (let i = 0; i < 200; i++) a = [a]; a.flat(Infinity)` | `[1]` (FlattenIntoArray; today depth 100 leaves nesting) |
| `flat_cyclic` | `const a = []; a.push(a); a.flat(Infinity)` | `RangeError` |
| `abort_any_chain_depth_one_precondition` | on a 2 MiB thread, 10 000 chained `AbortSignal.any([s])` with intermediates kept, then `c.abort()` | `s.aborted === true`, no abort. Checks A-1 (§2): it aborts the process if the fan-out is still recursive (today: abort at 1000 kept in debug, §11 M-AB), so it is red on `47b6c934` and green only once the W-11 slot has landed. The unreferenced (GC) cell is the W-11 slot's own |
| `stack_guard_bound_is_os_reported` | on a 2 MiB thread, first probe | bound source = OS; span ∈ [2 MiB − 64 KiB, 2 MiB + 64 KiB] (macOS reports the request + 12 KiB, umbrella §7); prints span (the Linux / Windows measurement) |
| `stack_guard_unknown_bound_fails_early` | forced-unknown seam (§4.5) | first probe at top passes; recursion stops with `RangeError` at ≤ 1 MiB used |
| `stack_guard_reserve_margin_ratchet` | the `walker_floor_*` and `rt_*` bodies with the test-only reserve of 64 KiB | all `RangeError`, no abort; this is also the cell that turns a **late** bound (low below the real guard) into a red run |
| `wasm_start_recursion_traps_at_top` | at the thread's top, `new WebAssembly.Instance(m)` where `m`'s start function recurses without end (`(module (func $f call $f) (start $f))`) | `RuntimeError` (wasmtime's own limit, which `MAX_WASM_STACK` sets), no abort; green today too (`H/logs/wasm-entry.log` `top`). Pins that wasm's limit is below the thread's room at the top |
| `wasm_instantiate_deep_js_then_wasm` | recurse through `[1].forEach` to each depth D from the top to the CP-1 floor; at D, the same `new WebAssembly.Instance(m)` (CP-3b), and the async `WebAssembly.instantiate(m)` at the same depths (rejection) | at every D: `RuntimeError` (enough room) or `RangeError` / `STACK_EXHAUSTED_MSG` (CP-3b), never an abort. Red on `47b6c934`: aborts for D in the band between the floor and ≈600 KiB remaining (debug 28–32, release 240–260 on the prototype, `H/logs/wasm-entry.log`) |
| `wasm_call_deep_js_then_wasm` | bound session (the `tests_wasm.rs` harness: `setup_vm_for_binding` + `test_helpers::bind_vm`); an instance exporting `f` that recurses without end; recurse through `[1].forEach` to each depth D up to the floor and call `instance.exports.f()` (CP-3a) | same as above, at every D. Red on `47b6c934` in the same band (same mechanism as CP-3b, whose band is the measured one; the exported-function band is measured when the cell is written) |
| (no JS→wasm→JS→wasm cell) | — | not constructible at PR-1's base: wasm cannot call JS (§4.4). It is the obligation PR-1 hands `#11-wasm-user-import-host-fn-builder` (§12 L-13), whose acceptance gains that cell |
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
Run on the final head only, debug and release. None has been run yet.

| Mutant | Removed | Dies |
|---|---|---|
| M-CP1 | CP-1 | `rt_reviver_recursion`, `rt_map_recursion`, `rt_super_recursion`, `rt_cyclic_join_range_error`, `ce_upgrade_probe_failure_takes_threw_branch` (abort) |
| M-CP2 | CP-2 | `rt_generator_resume`, `rt_inline_async` |
| M-W1/2 | W-1, W-2 | `walker_floor_parse` |
| M-W3 | W-3 | `walker_floor_internalize` |
| M-W4/5 | W-4, W-5 | `walker_floor_stringify` |
| M-W6 | W-6 | `walker_floor_sc` |
| M-W7 | W-7 | `parser_parens_depth` (floor half) |
| M-W7b | W-7b | `parser_class_heritage_chain` |
| M-W8 | W-8 probe branch | `frontend_chain_scope_probe` (abort) |
| M-W8a | W-8 cap branch reverted to the silent skip | `frontend_chain_scope_cap` (no error; `q` unbound) |
| M-W9 | W-9 | `frontend_chain_compile_probe` (abort) |
| M-W10 | W-10 | `flat_cyclic`; reverting to `MAX_FLAT_DEPTH` kills `flat_infinity_deep` |
| (no M-W11) | — | W-11 is not a PR-1 change (PM 454); reverting the slot's flattening kills `abort_any_chain_depth_one_precondition` (A-1), which is the check, not a PR-1 mutant |
| M-W12 | W-12 | `walker_floor_idb_storable` |
| M-RX1 | front-door `ensure_stack` | `rx_alternation_1e5` (and not `rx_nesting_repro`, which the cap answers: I-6 tells them apart) |
| M-RX2 | front-door cap | `rx_nesting_repro` changes message to `STACK_EXHAUSTED_MSG` |
| M-RX3 | `REGEXP_GROUP_COST` halved | `rx_group_cost_ratchet` |
| M-RX4 | one site bypasses the front door | trip-wire §6.4 |
| M-RX5 | weight = total `|` count (the R0 rule) | `rx_concat_groups_depth_one` |
| M-CC | one probe removed from a census cycle | the tracked cut check (§5.3), in addition to that probe's cell |
| M-CP3a | CP-3a | `wasm_call_deep_js_then_wasm` (abort) |
| M-CP3b | CP-3b | `wasm_instantiate_deep_js_then_wasm` (abort) |
| M-CP3c | CP-3's ask reduced to `WALKER_RESERVE` (constant dropped from the ask) | `wasm_instantiate_deep_js_then_wasm` and `wasm_call_deep_js_then_wasm` (abort in the band) |
| M-WS | `config.max_wasm_stack(MAX_WASM_STACK)` replaced by a larger value (e.g. 4 MiB), i.e. wasmtime's limit no longer the constant the probe asks for | `wasm_start_recursion_traps_at_top` (abort at the top of 2 MiB) and the two deep cells |
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
  evaluated 10⁶ times; the front door's weight pass + gate) and `wasmcall` (an exported wasm
  function of an empty body called 10⁶ times from JS; CP-3a. CP-3b, once per instantiation, is not on
  a hot path and is not measured). W-12 is measured only if an IndexedDB
  put of 10⁴ records is runnable in the test harness; otherwise it is listed as unmeasured.
- **Band and verdict**: the noise band is the control row's |Δ min| on that run (floor 1 %). A
  probed row whose Δ min exceeds the band blocks; the PR then states the cost and the PM decides
  between accepting it and a fast path (I-5 says none is planned).

## §11. Measurements recorded for this plan

- **M-AB** `AbortSignal.any` chain, debug, guard off, 2 MiB: kept intermediates, N = 10 / 100 / 1000 /
  3000 → `true` / `true` / abort / abort; dropped intermediates, N = 10 000 → `false`
  (`H/logs/dbg-census-holes.log` and follow-up runs in the R0 session). Evidence for the W-11 slot
  (PM 454) and for the A-1 precondition cell; PR-1 does not change it.
- Regexp axes: `H/logs/rel-rx-axes.log`, `H/logs/rel-rx-axes2.log`, `H/logs/dbg-rx-axes.log`,
  `H/logs/rx-shapes-dbg.log`, `H/logs/rx-concat-alt.log`, `H/logs/rx-concat-alt-rel.log`.
- Class heritage: `H/logs/rel-classext.log`, `H/logs/dbg-census-holes.log`.
- Cut check: `H/logs/cutcheck-pr1.log` (before W-7b / W-9 additions), `H/logs/cutcheck-pr1-v2.log`.
- Left-deep chains, silent skip, per-pass need: umbrella §8 M-8.
- cap−10 on 2 MiB with the probe on (debug): `H/logs/cap10-on-dbg.log`
  (`WALK=<w> CAP=590 <debug cd_probe> walker 1 2097152`, w ∈ parse stringify internalize sc;
  lowest remaining at a probe 898 895 / 604 175 / 397 807 / 371 519 B).
- Forced fallback: umbrella §8 M-9. Bound source and span: umbrella §8 M-5.
- JS→wasm entry near the floor (D-W): umbrella §8 M-11, `H/scripts/wasm_entry.sh`, log
  `H/logs/wasm-entry.log` (numbers in §4.4). Guard off, the same entry aborts from debug D = 30 /
  release D = 250 (versus `RuntimeError` at 25 / 200).

## §12. Landing obligations

What PR-1's implementation PR (or, where marked, the PM / PMO at its landing) must do. This memo
records them; it edits neither the ledgers nor the slice-plan.

- **L-1. Close `#11-vm-call-depth-guard`** (PM / PMO, live ledger): with PR-1's PR number and squash
  sha, and a pointer to the tracked cut check (§5.3) as what keeps the closure true.
- **L-2. `#11-vm-json-walkers-iterative`'s trigger fires on PR-1** ("the next PR touching a JSON
  walker"). Handling: acknowledged and not taken. PR-1 touches the walkers only to add a probe at
  their existing depth-check site; making them iterative is that slot's own derivation (three
  walkers, explicit stacks, the debug / release cap unification), not a guard change. The PR body
  says so; the live entry records "trigger fired at PR-1 (sha); not taken: probe-only touch" and
  keeps trigger 2 live (the 2 MiB `json_cap_minus_10_on_production_thread_*` cells, §8). Its
  premise wording is aligned with the ledger: A4's cap already prevents the walker-own abort (the
  umbrella's R0 "the abort is gone" is withdrawn).
- **L-3. `#11-vm-array-join-cycle-detection`: premise realised.** The live entry gains: "under PR-1
  (sha) a cyclic `join` gives the catchable `RangeError` (cell `rt_cyclic_join_range_error`)".
- **L-4. `#11-vm-native-callback-reentry-single-loop`: its numbers change meaning.** "~35–47 debug,
  ~270–470 release" were abort depths; under PR-1 the callback depth ends in a `RangeError` at
  d≈30–42 / 250–409 (256 KiB reserve, umbrella §8 M-1), a figure that now depends on
  `WALKER_RESERVE` as well as frame size. The live entry is updated to say so.
- **L-5. `#11-vm-js-frame-depth-limit`'s Promise facet**: ⟨PM pending: P6⟩; PR-1 does not change
  it.
- **L-6. `#11-dom-walkers-iterative-over-ecs-links` depends on the reserve.** Its live entry gains:
  "until it lands, the DOM walkers' only headroom near the limit is PR-1's `WALKER_RESERVE`
  (256 KiB); `stack_guard_reserve_margin_ratchet` (64 KiB) runs no DOM walker, so a DOM-walker
  frame-size change is not ratcheted".
- **L-7. PM 458's extension**: before landing, PR-1 checks that the W-11 slot removed all seven
  `§3.1.3`-family cites in `abort_statics.rs` (`git grep -n '§3\.1\.3' -- crates/script/elidex-js/src/vm/host/abort_statics.rs`
  → 0 lines) alongside the A-1 ancestry check, and reports either result.
- **L-8. Slice-plan row 8** (`docs/plans/2026-07-vm-p4-slice-plan.md`, `:976`): its cell gains the
  pointer "the RegExp creation / construction child adopts `vm/regexp_compile.rs`'s
  `compile_regexp` as its compile path (stack-safety guard PR-1 §6.1)", and the
  `crate::regexp` re-check trigger of §5.3. Then `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md`
  (rc 0) and `bash scripts/python-suites.sh` (the VM P4 family guard runs there).
- **L-9. Breadth record**: `python3 .claude/skills/elidex-plan-review/preflight.py` on both memos at
  the PR's head, rc and K / M as printed (it does not map the `IndexedDB-3` or `WebAssembly JS API`
  labels; the hand count in §3 is K=4 / M=20 here and K=4 / M=21 in the umbrella).
- **L-10. New slot**: `#11-vm-frontend-left-spine-iterative` (§5.4): ⟨PM pending: P8⟩ (proposal:
  owner VM P4, acceptance = the `frontend_chain_*` cells compile a 10⁴-term chain on 2 MiB in both
  profiles; or the PM re-homes it).
- **L-11. Stale cites** (§3.3): the live entry of `#11-vm-structured-clone-layering-seam` gains the
  list, so the seam's derivation re-cites them.
- **L-12. CLAUDE.md §CI and `mise run ci`**: updated in the same PR if P5 adds a release step.
- **L-13. `#11-wasm-user-import-host-fn-builder` inherits a guard obligation** (live entry, PM /
  PMO): "JS imports called from wasm enter JS through `with_call_mode` (CP-1), and every JS→wasm
  entry they lead to passes CP-3; acceptance gains a JS→wasm→JS→wasm recursion cell that ends in a
  `RangeError` on a 2 MiB thread in both profiles (each instance has its own store, so wasmtime's
  limit restarts at every nested entry; only CP-3 bounds the sum)".
- **L-14. Boundary-crossing serializers re-compile through the front door** (live entries of
  `#11-worker-structured-serialize`, `#11-history-state-structured-serialize-fidelity`,
  `#11-idb-structured-clone-storage`, PM / PMO): "a RegExp crossing the boundary is deserialized by
  re-compiling `[[OriginalSource]]` / `[[OriginalFlags]]` through `compile_regexp` (stack-safety
  guard PR-1 §6.3); the in-process `clone_regexp` copies the matcher and is not the model".
