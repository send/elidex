# VM stack-safety guard — umbrella (`#11-vm-call-depth-guard`)

Status: **plan (pre-implementation), revised after plan-review R1** (findings at
`H/review/axis1.md` … `axis5.md`, `aggregate.md`) **and the driver decisions D-W / D-SC and the
split verdicts** (PR-1 §6.3, §5.1; PR-1b §4), **and the PM 483 rulings** (§0.1). Owner: VM P4 lane.
PR-level plans: [PR-1](2026-11-vm-stack-safety-guard-pr1.md),
[PR-1b](2026-11-vm-stack-safety-guard-pr1b.md).

**Provenance root.** `H` = `/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/stackguard-census`
(outside the repo, durable). `H/scripts/` holds every measurement script, `H/logs/` every log,
`H/prototype/` the prototype sources (`stack_guard.rs`, the driver examples `cd_probe.rs` /
`pass_need.rs`). The scripts hard-code the binaries of the prototype clone (a scratch clone at
`efe8d05b` with `H/prototype/stack_guard.rs` applied), which lives in the VM P4 session scratchpad
and is **not** durable; rebuilding it is `H/scripts/build.sh`. Every number below names its log.
Code anchors are at `47b6c934` (origin/main, 2026-11-08) unless a row says otherwise. The
prototype base `efe8d05b` differs from `47b6c934` in the front end only by #602
(`compiler/resolve.rs`, closure upvalue resolution); the recursion census is byte-identical on
both (§8 M-0). The procedure that re-derives the constants in the repo is a ratchet cell plus a
tracked driver (PR-1 §6.2, §8), not a script under `H`.

## §0. Charter

A script must not be able to abort or hang the renderer by exhausting the Rust stack. Today it can:
every path that nests the Rust interpreter (`run()` re-entered through a native callback, a
generator resume or a synchronous construct), and every recursive walker in `elidex-js`
(JSON, structuredClone, `Array.prototype.flat`, the parser / scope analyser / compiler, regexp
compilation, `AbortSignal.any` propagation), recurses on the host stack with no remaining-stack
check. (`AbortSignal.any` propagation is not removed by this program: the W-11 slot removes it
structurally before PR-1 lands, PM 454; §1.2, §4.) On a 2 MiB thread (the size every production
VM thread gets, see `MAX_JSON_DEPTH`'s thread survey in
`crates/script/elidex-js/src/vm/natives_json/mod.rs`) 28 of 32 measured re-entry classes abort the
process (§8 M-1), and a left-deep expression chain of 383 terms (`1+1+…`) aborts the compiler in a
debug build (§8 M-8).

The program installs **one remaining-stack primitive** in `elidex-js` and calls it at the
re-entry choke points and inside every unbounded `elidex-js` walker. Exhaustion becomes a
catchable `RangeError` ("Maximum call stack size exceeded") at run time, and a compile failure of
one error kind at parse / compile time (PR-1 §2 I-6). This is **elidex's own resource limit**
(PM 424); no other implementation's number is being reproduced. What ECMA-262 says about it:

- §17 Error Handling and Language Extensions lists the permitted deviations from "report all errors
  as specified"; none is a resource limit. A probe failure is therefore a stated deviation, never a
  spec step.
- §15.10.3 PrepareForTailCall **is** a resource requirement: a tail position call must release or
  reuse the caller's execution-context resources. elidex is strict-only, so every tail position
  call is in scope of it. The guard's Rust-stack probes do not touch JS→JS calls (they push heap
  frames, PR-1 §5.1), so PR-1 is unaffected; PR-2's frame cap is not: under a `frames.len()` cap,
  `function f(n) { return n ? f(n - 1) : 0 } f(1e6)` becomes a `RangeError` where §15.10.3 requires
  constant resource use. **Ruling (PM 483)**: core is strict-only, so PrepareForTailCall is a
  normative requirement on core, and not implementing it is a **spec deviation**, homed in the open
  slot `#11-vm-proper-tail-calls` (owner VM P4; re-evaluated at PR-2's plan). The VM P4 coverage map's
  former "n/a (no tail calls in elidex)" (`docs/plans/2026-07-vm-p4-es-language-completeness.md`, row
  ECMA-262 §13.3.6.2 EvaluateCall step 6) now points at that slot. Today the tail call runs without
  bound until heap OOM (ledger `#11-vm-js-frame-depth-limit`: 1e7 frames reach 4 GB), so §15.10.3 is
  not met today either. PR-2's relation to the slot: once PTC lands, tail frames are released or
  reused and are **not counted** by the frame cap; until then, `f(1e6)` in tail position throwing
  `RangeError` under PR-2 is a **known deviation**, recorded as such in PR-2's plan and in the
  slot.

### §0.1 Ratified rulings (decided; not re-opened here)

| Id | Ruling |
|---|---|
| PM 421 (a) | Walker-level probes: one remaining-stack primitive, called at the re-entry choke points and at each walker's depth check. `MAX_JSON_DEPTH` stays as the **semantic** bound; the probe is the **safety** bound. libc / windows-sys, no new crate. Error = catchable `RangeError` "Maximum call stack size exceeded". |
| PM 421 (b) | The structuredClone depth bound is absorbed into the guard as a walker probe. |
| PM 424 | Never phrase a limit as reproducing another implementation. A limit is spec or elidex's own resource limit. A cyclic `join` giving a catchable `RangeError` is correct, not a defect. (Attribution, restated after R1 without changing the ruling: JoinIndexedProperties §23.1.3.18.1 has no cycle detection, so the spec's own evaluation of a cyclic `join` recurses without end; the `RangeError` is elidex's resource limit stopping that recursion, not a spec step.) |
| PM 436 R1 = (a)+(b) | One regexp-compile front door: nesting checked against the cap, then `ensure_stack(nesting × measured per-level cost)`. One primitive taking a size; walkers pass the default reserve. Every compile site routes through it. No regress-replacement slot. |
| PM 436 R2 | Recursive DOM walkers outside `elidex-js` are out; new L3 slot `#11-dom-walkers-iterative-over-ecs-links`. |
| PM 436 R3 | Style / layout / paint recursion is out; new L2 slot `#11-render-pipeline-recursion-depth-abort`. |
| PM 436 extra | `MAX_FLAT_DEPTH` is replaced by the probe in PR-1; the parser / compiler recursive entries join PR-1's walker set; JSON cells run on an explicit `stack_size` thread. |
| PM 436 split | PR-1 = primitive + choke points + walkers + regexp front door + PM 424 rewording. PR-2 = the frame cap (`#11-vm-js-frame-depth-limit`). PR-1 measures the hot-path probe cost and the Linux / Windows bounds (or names the CI cell that does). |
| PM 454 (W-11) | `AbortSignal.any` chain recursion (PR-1's former row W-11) is its **own terminal slot**, not part of PR-1, and it **lands before PR-1**. The fix is DOM "create a dependent abort signal" source flattening (DOM §3.2, step 4.2), which also repairs the GC defect (unreferenced intermediates are collected and the abort never propagates). Owner **L3**; per the Layering mandate the algorithm goes in an engine-independent crate (the `elidex-dom-api` side), not in `vm/host/`. The `abort_statics.rs` citation drift is fixed on that slot's first touch — all seven `§3.1.3`-family cites in that file, one touch (PM 458, ledger entry of `#11-abort-signal-any-dependent-flattening`). Slot id: `#11-abort-signal-any-dependent-flattening` (registered by PM 454). |
| PM 454 (weight) | The regexp front door's two-axis weight `groups × REGEXP_GROUP_COST + alts × REGEXP_ALT_COST`, with the cap on `groups` only, is within PM 436 R1 (ratified). (R1 revision: `alts` is now the alternatives on the costliest path, not the pattern's total `|` count — PR-1 §6.2; the two-axis form and the groups-only cap are unchanged.) |
| PM 454 (homes) | Regexp syntax errors surfacing as `TypeError` (spec: `SyntaxError`) → a child of VM P4 slice-plan umbrella 8 (RegExp completion), owner VM P4. The missing `RegExp` constructor → under umbrella 8, owner VM P4; the live ledger homes it in `#11-vm-regexp-constructor-and-flags` (PM 454 note on that entry). |
| PM 476 | S1 (`#11-vm-string-replace-function-replacement`) stays a single PR; the guard is plan-reviewed first. After S1 lands, PR-1's compile-site anchors are re-derived on PR-1's own base (PM 483 P7, PR-1 §6.1). |
| PM 483 | P1: PrepareForTailCall is a normative requirement on strict-only core; not implementing it is a spec deviation homed in `#11-vm-proper-tail-calls` (VM P4, re-eval = PR-2's plan); §0. P2: Slice F → PR-1 (touch-time split); W-6's probe and the front-door obligations are hand-offs to the `#11-vm-structured-clone-layering-seam` derivation (PR-1 §5.5). P4: regexp `SyntaxError`-as-`TypeError` home = `#11-vm-regexp-syntax-error-surface` (child of umbrella 8, VM P4). P5: a release test leg on all three CI OSes, ordered after the CI/tools lane's `ci.yml` PRs `ci-ecs-mission-ratchet` and `ci-default-features-check`; the CLAUDE.md §CI wording goes to the PM for user approval (§7). P6: the self-chaining-Promise facet is its own slot `#11-vm-microtask-self-chaining-unbounded`. P8: `#11-vm-frontend-left-spine-iterative` registered (VM P4, immediately after PR-1). D-K: CP-3 and `MAX_WASM_STACK` split into PR-1b; `#11-vm-call-depth-guard` closes at PR-1b's landing. |

## §1. Scope

### §1.1 In

- The primitive (`crate::stack_guard` (NEW), PR-1 §4) and its per-thread bound.
- The two re-entry choke points: `VmInner::with_call_mode` entry and `VmInner::resume_generator`
  entry (PR-1 §5.1). Every nested `run()` is reached through one of them (§2 I-1).
- The two JS→wasm entries (CP-3: exported-function call and instantiation, PR-1b §5), which hand
  wasmtime up to `max_wasm_stack` of the same thread's stack: the probe asks `WALKER_RESERVE +
  MAX_WASM_STACK`, and `max_wasm_stack` is set from that one constant (PR-1b §4; measured, §8 M-11).
- Every unbounded walker in `elidex-js` (PR-1 §5.2, enumerated by the census in §8 M-0 and the
  cut check in §8 M-6, which PR-1 turns into a tracked CI check, PR-1 §5.3).
- The scope analyser's cap branch, which today skips silently and leaks depth (§3.1): PR-1 makes it
  record an error (PR-1 §5.2 W-8).
- The regexp-compile front door and the routing of the five `regress` compile sites that stay
  compile sites through it; the sixth (structuredClone's re-compile) becomes a copy of the compiled
  matcher, which is HTML StructuredSerializeInternal step 13 as written (PR-1 §6.1, §6.3).
- The frame cap for plain JS→JS recursion, which never touches the Rust stack (PR-2; its relation
  to §15.10.3 and `#11-vm-proper-tail-calls` is §0's ruling: a known deviation until PTC lands).
- The PM 424 rewording of the `MAX_JSON_DEPTH` docstring.

### §1.2 Out (with the slot that owns each)

| Out-of-scope surface | Why out | Owner slot |
|---|---|---|
| Recursive DOM walkers outside `elidex-js` (`elidex-dom-api` serialize / text collection / normalize, `elidex-form`, `elidex-ecs` shadow walks, `elidex-html-parser` convert, …; census `H/census-host.txt`, §8 M-0) | PM 436 R2. They run on the VM thread, so the guard's reserve gives them headroom near the limit but no guarantee: `innerHTML` / `outerHTML` on a 3000-deep tree abort in debug today (§8 M-7). PR-1's 256 KiB reserve is sized with this in mind (PR-1 §4.3), so that slot depends on the reserve until it lands (PR-1 §12 L-6; a soft edge, PR-1 → the slot, not a precondition of PR-1). Schedule (L3 estimate via PM, 11-08): L3 folds it with the capped-walker slot into one umbrella, start ≈11-11, slice 1 (`elidex-ecs` `tree/walkers.rs` + the capped walker's 8 callers) landing ≈11-14, the whole umbrella ≈end of November — i.e. after PR-1, so L-6 holds in that window | `#11-dom-walkers-iterative-over-ecs-links` (L3) |
| Style / layout / paint recursion (`elidex-css` selector matching, calc / media parse-eval, layout tree walks) | PM 436 R3. The primitive is `pub(crate)` in `elidex-js` (PR-1 §4.1); if that slot's derivation picks a remaining-stack probe, the primitive must move to a crate both can depend on rather than be duplicated (one primitive, PM 436 R1) — recorded so the slot does not grow a second one | `#11-render-pipeline-recursion-depth-abort` (L2) |
| Removing the Rust recursion of native→JS callbacks altogether (one dispatch loop) | Successor ideal, T3. The guard stays as the safety bound either way | `#11-vm-native-callback-reentry-single-loop` |
| Iterative JSON walkers | T3 engineering ideal. A4's `MAX_JSON_DEPTH` already prevents each walker's *own* recursion from aborting; PR-1 adds the probe for re-entry. What stays is the frame-size dependence of `MAX_JSON_DEPTH` (debug 600 vs release 1000) | `#11-vm-json-walkers-iterative` |
| Returning `""` for a cyclic `join` | A compat behaviour; core gives the catchable `RangeError` (PM 424; elidex resource limit, §0.1) | `#11-vm-array-join-cycle-detection` (compat) |
| `AbortSignal.any` chain recursion and its GC defect (DOM "create a dependent abort signal" source flattening) | PM 454: own terminal slot, lands **before** PR-1 (§4); the algorithm belongs in an engine-independent crate, not `vm/host/` | `#11-abort-signal-any-dependent-flattening` (L3) |
| Regexp syntax errors surfacing as `TypeError` (spec: `SyntaxError` / early error) at the literal / `match` / `search` compile sites | PM 454; PR-1's front door passes `Syntax` through unchanged. (structuredClone is **not** in this set: after PR-1 it does not compile at all, it copies the matcher, §3.) | `#11-vm-regexp-syntax-error-surface` (child of VM P4 slice-plan umbrella 8, RegExp completion; owner VM P4; PM 483). Until now carried as a sub-item of `#11-vm-regexp-constructor-and-flags`'s live entry ("regex syntax errors surface as TypeError", PM 454 note) |
| The missing `RegExp` constructor | PM 454; PR-1 covers it through I-7 once it calls the front door | `#11-vm-regexp-constructor-and-flags` (under umbrella 8, VM P4) |
| Long left-deep expression chains (`a+b+…`, `a.b.c…`, `f()()…`) compiling at any length | Decided in PR-1 §5.4: they must compile; the remedy (iterating the left spine in scope + compiler) is its own slot, not PR-1 | `#11-vm-frontend-left-spine-iterative` (registered, PM 483; VM P4, scheduled immediately after PR-1) |
| Proper tail calls (ECMA-262 §15.10.3 PrepareForTailCall) | A normative requirement on strict-only core, not implemented: a spec deviation (PM 483, §0). The guard does not implement it; PR-2's frame cap states its relation (§0) | `#11-vm-proper-tail-calls` (VM P4; re-eval at PR-2's plan) |
| A self-chaining Promise inside one `drain_microtasks` (unbounded) | Each job runs to completion and pops its frames, so neither a probe nor a `frames.len()` cap bounds it (PM 483) | `#11-vm-microtask-self-chaining-unbounded` (split from `#11-vm-js-frame-depth-limit`) |
| The `eval` / `Function` mapping of `StackExhausted` (compile-time → `RangeError` at the call) | No `eval` / `Function` today; the decision is recorded in PR-1 §4.1 so it is not re-made | `#11-vm-function-constructor-global` (owns `Function` and `eval`, VM P4 ledger) |

## §2. Coupled invariants

Five program invariants hold at once; PR-1 adds I-6, I-7 and A-1 (PR-1 §2, with their 18 pairs);
PR-1b adds W-1 and W-2 at the JS→wasm entries (PR-1b §2). The CP-3 cells of the table below are
realised by PR-1b, the rest by PR-1.
Each pair's intersection is stated with what pins it; the **Basis** column says whether that is a
measurement (with its log), an assertion PR-1 still has to measure, or structural (true by the
placement, nothing to measure).

- **I-1 Coverage** — every Rust-stack recursion reachable from script passes a probe (or is
  bounded by construction) before it can consume more than the reserve.
- **I-2 Fail-safe bound** — the per-thread lower stack bound is the OS-reported one; when it is
  unknown the guard trips early (spurious `RangeError`), never late (abort). The fallback's
  "never late" rests on an assumption (PR-1 §4.2): the first probe runs near the thread's top.
- **I-3 State integrity** — a probe that fails leaves **VM** state as if the guarded operation never
  started (no half-entered frame, no generator marked Running). State outside the VM that a caller
  mutated before entering a choke point is the caller's: the one such caller today is custom-element
  upgrade, which follows HTML's "the above steps threw" branch (PR-1 §2 I-3). (Abort propagation
  carries no probe at all: after the W-11 slot it is depth 1 by construction, PM 454.)
- **I-4 Semantic caps unchanged** — `MAX_JSON_DEPTH` (cfg-split: 600 debug / 1000 release),
  `MAX_NESTING_DEPTH` (1024) and `MAX_REGEXP_DEPTH` (1024) keep their values and stay the semantic
  answers; the probe fires only where the cap would not save the stack. Only `MAX_JSON_DEPTH` is
  profile-dependent, and it stays so.
- **I-5 Hot-path cost** — the probe on `with_call_mode` / `resume_generator` / per-walker-level is
  cheap enough not to need a fast-path exemption.

| Pair | Intersection | Pinned by | Basis |
|---|---|---|---|
| I-1 × I-2 | Coverage is only as good as the bound: the reserve must exceed the largest **unprobed** stack segment between two probes, on the bound the OS reports. At CP-3 the segment after the probe is wasm's, so the ask is `WALKER_RESERVE + MAX_WASM_STACK`: an entry that runs to wasmtime's 512 KiB limit needs more than 512 KiB and at most ≈595 KiB (release) / ≈630 KiB (debug) of remaining stack (`H/logs/wasm-entry.log`, §8 M-11). | Minimum reserve with walker probes ~31 KiB debug / ~15 KiB release (16 KiB resolution, `H/logs/dbg-bisect.log`, `H/logs/rel-bisect.log`); 256 KiB chosen (PR-1 §4.3). macOS: a 2 MiB thread reports `requested + 12 KiB` and a C recursion faults 204 B below the reported low, so the reported low is the guard boundary (§8 M-5). Linux / Windows: asserted only, CI cell `stack_guard_bound_is_os_reported` + the 64 KiB ratchet (PR-1 §8). | measured (macOS); asserted (Linux, Windows) |
| I-1 × I-3 | Each choke-point probe sits before the first VM state mutation of the guarded operation, so the failing path returns `Err` with nothing to unwind; for CP-3 also before the first wasm store / instance mutation. | `with_call_mode`: probe before the `entry_frames` snapshot. `resume_generator`: before the generator is taken / marked Running. CP-3a: before the `HostState` bind and `set_fuel` (the arm's `gc_enabled` toggle is restored on `Err`); CP-3b: before the lazy `vm_wasm_runtime()` init, `Store::new` and the start function (PR-1 §2, §5.1; CP-3: PR-1b §2, §5). Mutants M-CP1 / M-CP2 (PR-1 §9) and M-CP3a / M-CP3b (PR-1b §7) are planned, **not run**. | structural (placement); mutants to run in PR-1 |
| I-1 × I-4 | A cap that the stack cannot reach on the target thread hides a missing probe; a probe that fires before the cap changes the semantic answer. | JSON walkers on a 2 MiB thread, guard OFF, need at cap−10: parse 1184 / 368 KiB, stringify 1472 / 480, internalize 1712 / 368, sc 1696 / 480 (debug / release, `H/logs/dbg-need.log`, `H/logs/rel-need.log`). With the probe ON (256 KiB) cap−10 still passes on 2 MiB in debug, the lowest remaining at a probe being 397 807 B (internalize; `H/logs/cap10-on-dbg.log`). So the cap cells run on 8 MiB (they test the cap), and separate cap−10 cells stay on 2 MiB with the probe on (they ratchet the cap's derivation, PR-1 §8). | measured (macOS, debug + release) |
| I-1 × I-5 | Probing every walker level and every callback entry is what makes coverage local and auditable; it is affordable only if one probe is a TLS read + SP read + compare. | Release, lean probe vs probe compiled out, min of 7 on a loaded host, CP-1 / CP-2 / W-1…W-6 only (PR-1 §10 lists what is unmeasured; CP-3a's `wasmcall` workload is PR-1b §8's acceptance run). | measured (partial scope, macOS release) |
| I-2 × I-3 | An unknown bound fails early, which surfaces as a `RangeError` from an operation that has not started; I-3 makes the early failure harmless. | Forced fallback (`SG_FORCE_UNKNOWN`, 2 MiB): `map` / `json` / `generator` stop with a catchable `RangeError` at d = 15 / 13 / 15 debug and 115 / 107 / 137 release, vs 34 / 31 / 36 and 267 / 250 / 320 on the OS bound; bound = first-probe SP − 1 MiB (`H/logs/force-unknown.log`). | measured (macOS) |
| I-2 × I-4 | The cap must be reachable on the smallest supported thread or the cap cell silently tests the probe instead. | The explicit-thread rule for cap cells (PR-1 §8). | structural |
| I-2 × I-5 | The bound is computed once per thread (lazy, cached in a `thread_local!` `Cell`), so the OS query is never on the hot path. | One `pthread_*` / `GetCurrentThreadStackLimits` call per thread. | structural |
| I-3 × I-4 | Hitting a cap and hitting the probe must both be clean errors at the same depth-check site, so a walker has one error exit. | JSON: probe at the existing `depth > MAX_JSON_DEPTH` site. Scope: today the cap branch has **no** error exit (silent skip + depth leak, §3.1); PR-1 gives it one (PR-1 §5.2 W-8). | structural (design); the scope half is a PR-1 change |
| I-3 × I-5 | The probe is a pure read; it adds no state to roll back. | `ensure_stack` mutates only the lazily-initialised bound cell. | structural |
| I-4 × I-5 | Keeping the caps means the common shallow case never reaches the probe's error branch. | — | structural |

The edge-dense trigger fires (5 intersecting axes; no canonical algorithm, since no spec defines
the limit), so each PR goes through `/elidex-plan-review` before implementation.

## §3. Spec coverage map

Spec coverage is thin: no spec defines a stack or nesting limit. The rows are the algorithms whose
recursion the program bounds or whose outcome it changes, plus §17 (what it does not contain) and
§15.10.3 (the one resource requirement, PR-2's). A probe or cap failure is never a spec step; it is
elidex's resource limit (PM 424), and the "Step" cell says where it lands relative to the spec.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | paragraphs 1–4 (paragraph 3: no other kinds of errors as early errors) | permitted deviations (none is a resource limit) | `crate::stack_guard` (NEW) `StackExhausted` → `RangeError` / parse error kind (NEW) | ✓ | yes |
| ECMA-262 §15.10.3 PrepareForTailCall | steps 1–2 + the closing requirement (release or reuse resources) | tail position call | PR-2 frame cap; not implemented (spec deviation, `#11-vm-proper-tail-calls`, PM 483): after PTC lands tail frames are not counted by the cap, until then a tail-position `RangeError` is a known deviation (§0) | ✓ | yes |
| ECMA-262 §23.1.3.13 Array.prototype.flat | depthNum = +∞ | `flat(Infinity)` no longer truncated at 100 | `native_array_flat` | ✓ | yes |
| ECMA-262 §23.1.3.13.1 FlattenIntoArray | recursive call when depth > 0 | depth = +∞ / finite | `flat_into` | ✓ | yes |
| ECMA-262 §25.5.2.1 ParseJSON | step 1 (validity), step 3 ParseText, step 6 `! Evaluation` | nested arrays / objects | `parse_array` / `parse_object` (methods of `JsonParser`) | ✓ | yes |
| ECMA-262 §25.5.2.4 InternalizeJSONProperty | recursive call per element / key | array / object holder | `internalize` | ✓ | yes |
| ECMA-262 §25.5.4.5 SerializeJSONObject | SerializeJSONProperty per key | nested object | `serialize_object` (method of `JsonSerializer`) | ✓ | yes |
| ECMA-262 §25.5.4.6 SerializeJSONArray | SerializeJSONProperty per index | nested array | `serialize_array` (method of `JsonSerializer`) | ✓ | yes |
| ECMA-262 §23.1.3.18.1 JoinIndexedProperties | step 4.d (ToString / Invoke per element; no cycle detection) | cyclic array | CP-1 (via `toString` re-entry); cell `rt_cyclic_join_range_error` | ✓ | yes |
| ECMA-262 §13.2.7.3 Runtime Semantics: Evaluation | step 3 `! RegExpCreate` | RegularExpressionLiteral, compiled per evaluation | `load_constant` `Constant::RegExp` | ✓ | yes |
| ECMA-262 §22.1.3.13 String.prototype.match | step 5 RegExpCreate (string argument; non-RegExp object after step 3) | string / non-RegExp object | `native_string_match` (`natives_string.rs` lines 567/582 at `47b6c934`) | ✓ | yes |
| ECMA-262 §22.1.3.21 String.prototype.search | step 5 RegExpCreate (string argument; non-RegExp object after step 3) | string / non-RegExp object | `native_string_search` (`natives_string.rs` lines 646/666 at `47b6c934`) | ✓ | yes |
| ECMA-262 §22.2.3.1 RegExpCreate | step 2 RegExpInitialize | pattern parse + compile | front door `compile_regexp` (NEW) | ✓ | yes |
| ECMA-262 §22.2.3.3 RegExpInitialize | step 14 ParsePattern, step 15 SyntaxError, step 22 CompilePattern | the convergence point of RegExpCreate and the `RegExp` constructor | front door `compile_regexp` (NEW) | ✓ | yes |
| ECMA-262 §22.2.3.4 Static Semantics: ParsePattern | pattern parse | groups / disjunction | front door weight pass (NEW) | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal | step 28.4 per-property recursion; step 13 RegExp (copies `[[RegExpMatcher]]`, no compile) | Array / ordinary / Error / RegExp | `clone_recursive`, `clone_regexp` (matcher copy) | ✓ | yes |
| WHATWG HTML §2.7.7 StructuredDeserialize | step 25.3 per-property recursion; step 12 RegExp (from the copied matcher) | Array / Object / RegExp | `clone_recursive` (fused walk), `clone_regexp` (the copied matcher) | ✓ | yes |
| WHATWG HTML §2.7.11 Structured cloning API | step 1 StructuredSerializeWithTransfer (§2.7.8), step 2 StructuredDeserializeWithTransfer (§2.7.9) | — | `clone_recursive` entry | ✓ | yes |
| IndexedDB-3 §5.11 Clone a value | step 3 StructuredSerializeForStorage, step 4 StructuredDeserialize | stored value | `reject_non_json_storable` (W-12) | ✓ | yes |
| WebAssembly JS API §5.6 Exported Functions | *call an Exported Function* step 8 `func_invoke`; the probe's `RangeError` is elidex's resource limit (step 10's "unless otherwise indicated") | exported function call | CP-3a `call_wasm_exported_function` (PR-1b) | ✓ | yes |
| WebAssembly JS API §5 The WebAssembly Namespace | *instantiate the core of a WebAssembly module* step 2 (start function); step 3 third bullet ("another error type if appropriate") | sync ctor / async `instantiate` | CP-3b `instantiate_module` (PR-1b) | ✓ | yes |

**Breadth**: K=4 specs (ECMA-262, WHATWG HTML, IndexedDB-3, WebAssembly JS API), M=21 entries
(verified 2026-11-08 by `preflight.py`: M=21, K=4, with `IndexedDB-3` and `WebAssembly JS API` reported as
unmapped labels and their rows not webref-verified; the wasm anchors are hand-checked in PR-1b §3;
re-recorded at landing, PR-1 §12 L-9 and PR-1b §10). The R1 revision added §15.10.3, §25.5.2.1 (replacing the
§25.5.2 row), JoinIndexedProperties, RegExpInitialize, ParsePattern, StructuredDeserialize and
IndexedDB-3 §5.11, re-labelled §2.7.11, and moved the rows the program does not touch to §3.2;
D-W added the two WebAssembly JS API rows.
**Split decision**: K=4 and M=21 → SPLIT-RECOMMENDED by breadth. The program is already split (the
edge-dense reason, §2, and the PM 436 split). **D-K = split**: CP-3 and `MAX_WASM_STACK` are PR-1b,
not PR-1 — a different crate (`elidex-wasm-runtime`'s `Config`), a different spec (WebAssembly JS
API), and wasmtime's per-store limit is an invariant of its own (PR-1b §2 W-2). After the split
PR-1's map is K=3 / M=18 and PR-1b's K=1 / M=2 (PR-1 §3, PR-1b §3).

### §3.1 User-input touch audit

Every touched site takes script-controlled depth: JSON text, object graphs, array nesting, regexp
pattern text, script source nesting and chain length, callback recursion. (`AbortSignal.any`
chains are the W-11 slot's, PM 454.) Adjacent pre-existing code whose exposure changes:

- **Left-deep chains reach the scope and compiler recursion unbounded today.** The parser's Pratt
  infix loop (`parse_expr_bp_from`, `crates/script/elidex-js/src/parser/expr.rs:93`) keeps the
  parser's depth constant on `1+1+…`, `a.b.c…`, `f()()…`, while the AST is N deep on the left; the
  scope analyser's `visit_expr` (`crates/script/elidex-js/src/scope/visitor.rs:234`, `Binary` arm
  :288) and the compiler's `compile_expr` (`crates/script/elidex-js/src/compiler/expr.rs:74`,
  `Binary` arm :92-94) recurse on the left operand. Measured (2 MiB, guard off, §8 M-8): the
  compiler aborts the process at 383 terms in debug (`compile_expr` overflow) and 1677 in release.
  The compiler has no cap. **Exposure today: a short script aborts a debug renderer; a 1677-term
  chain aborts a release one.**
- `ScopeState::enter_recursion` (`crates/script/elidex-js/src/scope/mod.rs:205`) returns `false`
  past `MAX_NESTING_DEPTH` without recording an error, and its callers return without
  `leave_recursion`, so the depth **leaks**: after the first over-depth visit every later visit in
  the program fails too. Reachable through a left-deep chain of more than 1025 terms (release, where
  the compiler survives that far). Measured (§8 M-8): code **after** a 1100-term chain is not
  analysed — a later `let q = 5; String(q)` throws `ReferenceError: q is not defined`, a later
  function or arrow fails to compile with the internal "no function scope found" `CompileError`;
  the chain itself and code before it evaluate correctly. So the silent skip produces wrong code,
  not a panic. PR-1 gives the cap branch one error exit and removes the leak (PR-1 §5.2 W-8).
- The literal / `match` / `search` compile sites map a `regress` syntax error to `TypeError`
  (spec: early error / `SyntaxError`). Unchanged by this program; home = a child of VM P4
  slice-plan umbrella 8 (RegExp completion), owner VM P4: `#11-vm-regexp-syntax-error-surface` (PM 483).

### §3.2 Cited, not touched by this program

| Spec section | Why cited | Owner |
|---|---|---|
| ECMA-262 §13.2.7.1 Static Semantics: Early Errors (RegularExpressionLiteral) | A pattern syntax error is an early error; elidex compiles at evaluation instead. When the early-error check lands it connects `crate::regexp`'s recursive parser, which must then route through the probe (PR-1 §5.3) | `#11-vm-regexp-syntax-error-surface` (umbrella 8's SyntaxError child, PM 483) |
| ECMA-262 §13.2.7.2 Static Semantics: IsValidRegularExpressionLiteral | The predicate §13.2.7.1 calls (→ ParsePattern) | same |
| ECMA-262 §22.2.4.1 RegExp ( patternOrRegexp, flags ) | Steps 7–8 RegExpAlloc + RegExpInitialize: the constructor converges on RegExpInitialize steps 14 / 15, i.e. on the front door | `#11-vm-regexp-constructor-and-flags` |
| ECMA-262 §22.1.3.14 String.prototype.matchAll | Step 5 RegExpCreate: a future compile site, covered by I-7 when it lands | umbrella 8 |

## §4. Per-PR split

| PR | Content | Depends on | Unblocks |
|---|---|---|---|
| W-11 slot (not part of this program's PRs; PM 454) | DOM "create a dependent abort signal" source flattening in an engine-independent crate; GC repair; `abort_statics.rs` citation drift fix (all seven cites, PM 458) | — | PR-1 |
| `interpreter.rs` prereq split (standalone, CLAUDE.md 1000-line rule; PR-1 §5.1) | Move the in-file `mod tests` of `vm/interpreter.rs` out; split the entry half from the dispatch half | — | PR-1 |
| Slice F (`#11-vm-structured-clone-file-split`, standalone, touch-time split; PM 483 P2) | Split `structured_clone.rs` (1077 lines) before PR-1 touches it | — | PR-1 |
| CI/tools `ci.yml` PRs `ci-ecs-mission-ratchet`, `ci-default-features-check` (CI/tools lane; not this program's) | Their own `ci.yml` changes; the release test leg is added on top of them (PM 483 P5, §7) | — | PR-1 (the release leg only) |
| PR-1 | Primitive; choke points CP-1, CP-2; walker probes (JSON ×5 sites — `parse.rs` lines 278/321/410, `stringify.rs` lines 144/223 — structuredClone, `flat_into` replacing `MAX_FLAT_DEPTH`, parser / scope / compiler entries, the scope cap's error exit, IndexedDB storable check); regexp front door + 5 compile sites routed + structuredClone's re-compile replaced by a matcher copy + trip-wire; PM 424 rewording; explicit-thread JSON cells; the tracked recursion cut check; the release test leg on the three CI OSes (§7) | the W-11 slot (landed); the `interpreter.rs` prereq split; Slice F (landed, PM 483); the two CI/tools `ci.yml` PRs (landed, for the release leg) | PR-1b; PR-2; `#11-vm-frontend-left-spine-iterative`; the successor T3 slot |
| PR-1b | CP-3a / CP-3b at the two JS→wasm entries asking `WALKER_RESERVE + MAX_WASM_STACK`; `elidex_wasm_runtime::MAX_WASM_STACK` (NEW) and the explicit `max_wasm_stack`; the wasm cells and mutants (PR-1b) | PR-1 (the primitive, `WALKER_RESERVE`, `tests_stack_guard.rs`, the release leg) | closes `#11-vm-call-depth-guard` |
| PR-2 | Frame cap: `frames.len()` cap mapped to the same `RangeError`, for plain JS→JS recursion (heap `CallFrame`s, no Rust recursion). Relation to `#11-vm-proper-tail-calls` (PM 483): after PTC lands, tail frames are not counted by the cap; until then a tail-position `f(1e6)` throwing `RangeError` is a known deviation, recorded in PR-2's plan. The self-chaining-Promise facet is **not** PR-2's (`#11-vm-microtask-self-chaining-unbounded`, PM 483) | PR-1 (shares the error constructor and message) | — |

Edges: **W-11 slot → PR-1 is a hard edge** (PM 454). PR-1 cannot land correctly without it:
PR-1 puts no probe on `abort_signal` propagation and treats it as cut in the census (PR-1 §5.3), so
without the slot the `AbortSignal.any` chain stays an unprobed script-reachable Rust recursion
(guard off today, debug, 2 MiB: a kept chain of 1000 aborts the process, PR-1 §11 M-AB), I-1 does
not hold, and the program could not close `#11-vm-call-depth-guard`. Adding a probe there instead is not
an option (it would throw from `controller.abort()` with a half-propagated abort, breaking I-3).
PR-1's check of the edge is in PR-1 §2 (A-1). Its home (`elidex-dom-api`) is not where the abort
state lives today (`VmInner`'s `abort_signal_states` / `any_composite_map`, traced by the GC).
**Schedule** (L3 estimate via PM, 11-08): a separate small PR, not part of L3's walker umbrella;
L3 starts it right after #607 Rev 4 — plan and plan-review 11-09, implementation and pre-push
11-09 to 11-10, PR around 11-10, landing expected 11-10 to 11-11 after Codex review. Surfaces:
`abort_statics.rs` and a new engine-independent DOM §3.2 "create a dependent abort signal". PR-1's
landing condition is unchanged: after this slot lands.

**Slice F → PR-1** (PM 483 P2, touch-time split): the split moves the `structured_clone.rs` anchors
(W-6 `:88`, the `clone_regexp` re-compile `:585`), so PR-1 re-derives them on its own base. The
ledger edge F → NativeError E1 stands; PM 483 orders F before PR-1 and does not order E1 against
PR-1 (both edit `clone_recursive` after F; whichever lands second rebases onto the other). The
layering seam's derivation inherits PR-1's obligations (PR-1 §5.5).

**PR-1 → PR-1b** (D-K): PR-1b uses PR-1's primitive, reserve and cell file. Between the two
landings the JS→wasm entry band of PR-1b §4 (an entry between the CP-1 floor and ≈600 KiB remaining
aborts) is still open, which is why `#11-vm-call-depth-guard` closes at PR-1b, not PR-1.

**CI/tools `ci.yml` PRs → PR-1** (PM 483 P5): the release leg is the first release-profile CI
step of the program and PR-1 is the first guard PR whose cells carry release-only constants
(`MAX_JSON_DEPTH` 1000, `REGEXP_*_COST` release), so PR-1 carries it, on top of the two `ci.yml`
PRs (§7).

PR-2 reuses PR-1's `StackExhausted`→`RangeError` mapping so the two limits produce one error shape;
PR-2 does not touch the probe. The successor `#11-vm-native-callback-reentry-single-loop` removes
nested `run()`; after it, the choke-point probes become walker-only, which is a deletion, not a
redesign.

## §5. Slot homing

| Slot | State after this program | Home |
|---|---|---|
| `#11-abort-signal-any-dependent-flattening` | **Closed before PR-1** (its own terminal slot, PM 454) | L3 |
| `#11-vm-call-depth-guard` | **Closed by PR-1b** (D-K, PM 483), with PR-1b's sha (PR-1b §10 L-1); PR-1 alone does not close it (the JS→wasm band, §4). Its closure rests on I-1, whose coverage claim is only as complete as the census; PR-1's tracked cut check (PR-1 §5.3) is what keeps it true after landing, and the census's declared blind spots stay open with an owner (PR-1 §5.3) | this umbrella |
| `#11-vm-js-frame-depth-limit` | PR-2 closes it (the plain-recursion facet). Its second facet, "a self-chaining Promise is unbounded inside one `drain_microtasks`", which a `frames.len()` cap does not bound (each job runs to completion and pops its frames), is split out to `#11-vm-microtask-self-chaining-unbounded` (PM 483) | this umbrella |
| `#11-vm-microtask-self-chaining-unbounded` | **New** (PM 483), split from `#11-vm-js-frame-depth-limit`. Open; not bounded by anything in this program | as registered by PM 483 |
| `#11-vm-proper-tail-calls` | Open (PM 483): PrepareForTailCall not implemented on strict-only core, a spec deviation. Re-evaluated at PR-2's plan; after it lands, tail frames are not counted by PR-2's cap | VM P4 |
| `#11-vm-regexp-syntax-error-surface` | Open (PM 483), child of umbrella 8: regexp syntax errors surface as `TypeError` at the literal / `match` / `search` sites. PR-1's front door passes `Syntax` through unchanged | VM P4 |
| `#11-vm-json-walkers-iterative` | Stays open (T3 engineering ideal; A4's cap already prevents the walker-own abort). Its trigger "the next PR touching a JSON walker" fires on PR-1; the handling is PR-1 §12 L-2 | VM P4 |
| `#11-vm-native-callback-reentry-single-loop` | Stays open, successor (T3). Its depth figures change meaning under PR-1 (PR-1 §12 L-4) | VM P4 |
| `#11-vm-array-join-cycle-detection` | Stays open, compat. Its premise is realised by PR-1: a cyclic `join` gives the catchable `RangeError` (elidex resource limit; cell `rt_cyclic_join_range_error`, PR-1 §8; PR-1 §12 L-3) | compat (no lane) |
| `#11-dom-walkers-iterative-over-ecs-links` | Open (PM 436 R2); depends on PR-1's reserve until it lands (PR-1 §12 L-6; soft edge PR-1 → the slot, not a PR-1 precondition). L3 umbrella with the capped-walker slot, ≈11-11 to ≈end of November (L3 estimate via PM, 11-08; §1.2) | L3 |
| `#11-render-pipeline-recursion-depth-abort` | Open (PM 436 R3); one-primitive note in §1.2 | L2 |
| `#11-vm-frontend-left-spine-iterative` | **Registered** (PM 483; PR-1 §5.4), scheduled immediately after PR-1 | VM P4 |

## §6. Security relevance

A renderer abort (SIGABRT from Rust's stack-overflow handler) or a hang is a denial of service on
the tab, reachable from any page with a few lines of script (`[1].map(f)` recursion, a deep
`JSON.parse`, one 2000-deep regexp literal, a 1677-term `+` chain). Under the process model the
blast radius is the renderer process, so every tab sharing it is lost too. The guard turns each of
these into a catchable exception inside the page's own realm (or a compile failure of the one
script, PR-1 §2 I-6). This is a "security by structure" fix in the sense of the design philosophy:
the bound is enforced by one primitive at choke points, not by per-feature review, and the census
cut check keeps the choke-point set complete in CI (PR-1 §5.3).

## §7. Platform coverage

| Platform | Bound source | Measured where |
|---|---|---|
| macOS (aarch64) | `pthread_get_stackaddr_np` / `pthread_get_stacksize_np` | Locally. The reported size of a thread created with N bytes is N + 12 KiB (C probe, N = 1 / 2 / 8 MiB, `H/logs/bound-span-c.log`), and a C recursion on a 2 MiB thread faults 204 B below `top − reported size` (`H/logs/bound-deep-c.log`): the reported low is the guard boundary, so the bound is exact. The VM prototype's first-probe span is 2 095 263 B (debug) / 2 108 328 B (release) on a 2 MiB thread (`H/logs/bound-span.log`) — above 2 MiB in release, which is why PR-1's bound cell checks a band around the request, not an upper bound at it (PR-1 §8) |
| Linux (glibc) | `pthread_getattr_np` + `pthread_attr_getstack` | CI only (no Linux toolchain or image on the authoring host): cell `stack_guard_bound_is_os_reported` (PR-1 §8) asserts source = OS and prints the span; the late direction is pinned by `stack_guard_reserve_margin_ratchet` (it aborts if the bound is late by more than the 64 KiB test reserve minus the largest measured segment) |
| Windows (MSVC) | `GetCurrentThreadStackLimits` (windows-sys `Win32_System_Threading`, already enabled in the workspace by `elidex-sandbox`) | CI only: the same two cells |
| Anything else (no OS query implemented) | Fallback: first-probe SP − 1 MiB (fail early, I-2) | the bound cell asserts the fallback is **not** taken on the three CI OSes; the fallback itself is pinned by `stack_guard_unknown_bound_fails_early` and was run once (§8 M-9) |

**Release legs (PM 483 P5).** The release-profile constants (`MAX_JSON_DEPTH` 1000,
`REGEXP_*_COST` release values) were measured on macOS aarch64 only, Windows release has never been
run, and the CI `check` job runs nextest in debug on three OSes. Ruling: a **release test leg on
all three CI OSes** (`cargo nextest run -p elidex-js --all-features --release -E
'test(/tests_stack_guard/)'` in the `check` job), ordered **after** the CI/tools lane's `ci.yml`
PRs `ci-ecs-mission-ratchet` and `ci-default-features-check`. Carrier: **PR-1**, the first guard
PR whose cells carry release-only constants, with the edge from those two PRs (§4); PR-1b's wasm
cells join the same leg by living in `tests_stack_guard.rs`. CLAUDE.md §CI describes the `check`
job's steps, so the new step makes that text incomplete: the wording change is **not** made in
this memo or by PR-1 on its own — it goes to the PM for user approval, and PR-1 applies the text
the user approves (PR-1 §12 L-12). Whether `mise run ci` gains the release leg is part of the same
proposal.

## §8. Measurements (provenance)

All under `H` (defined in the header). Thread size 2 MiB unless stated. Prototype base `efe8d05b`.

- **M-0 census**: `python3 -I H/scripts/recursion_census.py <wt> crates/script/elidex-js` → 22 direct
  self-recursions, 10 SCCs; identical at `efe8d05b` and `4728972c` (`H/census-js.txt`). Host crates:
  `H/census-host.txt`. PR-1 moves the script into the repo (PR-1 §5.3).
- **M-1 re-entry classes**: `H/scripts/run.sh <debug|release> <tag> [SG_OFF=1]`. Guard OFF: 28 of 32 abort
  (`plain`, `bound_new`, `plain_new`, `replace` do not recurse on the Rust stack). Guard ON (256 KiB):
  all 28 give a catchable `RangeError` (`async_sync` as a rejected promise); debug stops at d≈30–42,
  release at d≈250–409 (`H/logs/{dbg,rel}-{off,on256}.log`).
- **M-2 walker thread need** (guard OFF): `H/scripts/need.sh <profile> <walker> <cap>`; numbers in §2.
- **M-3 minimum reserve**: `H/scripts/bisect.sh <profile> <walker> <cap>` → ~31 KiB debug / ~15 KiB release.
- **M-4 regexp / parser stack**: `H/scripts/needcode.sh` / `H/scripts/needbin.sh` with `CODE=…`; debug ≈5.0 KiB per
  regexp group level, release ≈1.3 KiB; parser ≈5.3 KiB per paren level in debug. Alternation is a
  second axis (PR-1 §6.2). A concatenation of N one-level groups `(?:a|b)` is flat: 80 KiB debug /
  32 KiB release for N = 1000, 10 000 and 50 000 (`H/logs/rx-concat-alt.log` debug rows,
  `H/logs/rx-concat-alt-rel.log` release; the release rows of the first log are from a lean-mode
  binary whose probe floors every run at 272 KiB, and are superseded).
- **M-5 bound source**: `BOUND src=1` line of any `cd_probe` run (`H/logs/bound-span.log`); the macOS
  C probes `H/scripts/span.c` / `H/scripts/deep.c` (`clang -O0`), logs `H/logs/bound-span-c.log`,
  `H/logs/bound-deep-c.log`.
- **M-6 cut check**: `python3 -I H/scripts/cutcheck.py <wt> crates/script/elidex-js --marker … --probe …`
  (exact command in PR-1 §5.3) → no residual cycle outside the classified set (`H/logs/cutcheck-pr1-v2.log`).
- **M-7 DOM walkers** (out of scope, evidence for R2): `H/logs/dbg-dom.log`.
- **M-8 left-deep chains** (guard off; the prototype has no front-end probes):
  `H/scripts/chainbisect.sh <bin> <label> <plus|member|call> 2097152` → largest N that does not
  abort: debug 382 / 380 / 314, release 1676 / 1674 / 1674 (`H/logs/chain-dbg.log`,
  `H/logs/chain-rel.log`); the debug overflow frame is `compile_expr` (`lldb` backtrace,
  `H/logs/chain-lldb-dbg600.log`). Silent skip at N = 1000 / 1100: `H/scripts/chain.sh <bin> <shape> <N>
  <stack>` (`H/logs/chain-skip.log`; debug on a 64 MiB thread) and `H/scripts/skip2.sh <bin>`
  (`H/logs/chain-skip2.log`, release): chains evaluate correctly, code after them does not (§3.1).
  Per-pass need, source parsed on the main thread and one pass on a thread of the given size:
  `H/scripts/passneed.sh <pass_need bin> <label> <scope|compile> <shape> <N>` (example
  `H/prototype/pass_need.rs`) → scope 752 KiB debug / 128 KiB release, compile 5312 KiB debug /
  1216 KiB release, for a 1000-term `+` chain (`H/logs/passneed.log`; ≈0.75 / 0.13 KiB and
  ≈5.3 / 1.2 KiB per level; `f()()…` compiles at ≈6.4 KiB per level in debug).
- **M-9 forced-unknown fallback**: `SG_FORCE_UNKNOWN=1 <cd_probe> <case> 100000 2097152` for
  `map` / `json` / `generator`, both profiles, with the OS-bound runs beside them
  (`H/logs/force-unknown.log`); numbers in §2 I-2 × I-3.
- **M-10 wasm and stack switching**: `cargo tree --offline -p elidex-js --all-features -i
  wasmtime-internal-fiber` and the same with `--workspace` → "nothing to print"
  (`H/logs/tree-fiber.log`, `H/logs/tree-fiber-ws.log`): the fiber crate is in `Cargo.lock` but
  not compiled (wasmtime's `async` feature is off). Reading of PR-1 §4.4 (stack switching).
- **M-11 JS→wasm entry near the floor**: `H/scripts/wasm_entry.sh <debug|release> <top|depth> [D] [SG_OFF=1]`
  (a module whose start function recurses without end, instantiated at callback depth D through
  `[1].forEach`; prototype guard on at 256 KiB unless `SG_OFF=1`), log `H/logs/wasm-entry.log`:
  top → `RuntimeError` both profiles; guard on, debug D = 24 / 26 → `RuntimeError`, 28–32 → abort,
  33 (floor) → `RangeError`; release 200 / 220 → `RuntimeError`, 240–260 → abort, 269 →
  `RangeError`. Reading and the `MAX_WASM_STACK` sizing: PR-1b §4.
