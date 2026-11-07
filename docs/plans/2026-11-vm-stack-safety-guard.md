# VM stack-safety guard — umbrella (`#11-vm-call-depth-guard`)

Status: **plan (pre-implementation)**. Owner: VM P4 lane. PR-level plan: [PR-1](2026-11-vm-stack-safety-guard-pr1.md).
Measured at `4728972c` (origin/main, 2026-11-06) unless a row says otherwise; the prototype that
produced the numbers is a scratch clone at `efe8d05b` with the probe patch on top (§8 lists every
command). The two commits differ by #601 (CI tooling) and #602 (closure upvalue resolution); the
recursion census is byte-identical on both (§8 M-0).

## §0. Charter

A script must not be able to abort or hang the renderer by exhausting the Rust stack. Today it can:
every path that nests the Rust interpreter (`run()` re-entered through a native callback, a
generator resume or a synchronous construct), and every recursive walker in `elidex-js`
(JSON, structuredClone, `Array.prototype.flat`, the parser / scope analyser / compiler, regexp
compilation, `AbortSignal.any` propagation), recurses on the host stack with no remaining-stack
check. (`AbortSignal.any` propagation is not removed by this program: the W-11 slot removes it
structurally before PR-1 lands, PM 454; §1.2, §4.) On a 2 MiB thread (the size every production VM thread gets, see `MAX_JSON_DEPTH`'s thread
survey in `crates/script/elidex-js/src/vm/natives_json/mod.rs`) 28 of 32 measured re-entry classes
abort the process (§8 M-1).

The program installs **one remaining-stack primitive** in `elidex-js` and calls it at the
re-entry choke points and inside every unbounded `elidex-js` walker. Exhaustion becomes a
catchable `RangeError` ("Maximum call stack size exceeded"). This is **elidex's own resource
limit**: ECMA-262 has no resource-limit clause (§17 Error Handling and Language Extensions lists
the permitted deviations and none of them is a resource limit), so no spec step is being
implemented and no other implementation's number is being reproduced (PM 424).

### §0.1 Ratified rulings (decided; not re-opened here)

| Id | Ruling |
|---|---|
| PM 421 (a) | Walker-level probes: one remaining-stack primitive, called at the re-entry choke points and at each walker's depth check. `MAX_JSON_DEPTH` stays as the **semantic** bound; the probe is the **safety** bound. libc / windows-sys, no new crate. Error = catchable `RangeError` "Maximum call stack size exceeded". |
| PM 421 (b) | The structuredClone depth bound is absorbed into the guard as a walker probe. |
| PM 424 | Never phrase a limit as reproducing another implementation. A limit is spec or elidex's own resource limit. A cyclic `join` giving the spec `RangeError` is correct, not a defect. |
| PM 436 R1 = (a)+(b) | One regexp-compile front door: nesting checked against the cap, then `ensure_stack(nesting × measured per-level cost)`. One primitive taking a size; walkers pass the default reserve. Every compile site routes through it. No regress-replacement slot. |
| PM 436 R2 | Recursive DOM walkers outside `elidex-js` are out; new L3 slot `#11-dom-walkers-iterative-over-ecs-links`. |
| PM 436 R3 | Style / layout / paint recursion is out; new L2 slot `#11-render-pipeline-recursion-depth-abort`. |
| PM 436 extra | `MAX_FLAT_DEPTH` is replaced by the probe in PR-1; the parser / compiler recursive entries join PR-1's walker set; JSON cells run on an explicit `stack_size` thread. |
| PM 436 split | PR-1 = primitive + choke points + walkers + regexp front door + PM 424 rewording. PR-2 = the frame cap (`#11-vm-js-frame-depth-limit`). PR-1 measures the hot-path probe cost and the Linux / Windows bounds (or names the CI cell that does). |
| PM 454 (W-11) | `AbortSignal.any` chain recursion (PR-1's former row W-11) is its **own terminal slot**, not part of PR-1, and it **lands before PR-1**. The fix is DOM "create a dependent abort signal" source flattening (DOM §3.2, step 4.2), which also repairs the GC defect (unreferenced intermediates are collected and the abort never propagates). Owner **L3**; per the Layering mandate the algorithm goes in an engine-independent crate (the `elidex-dom-api` side), not in `vm/host/`. The `abort_statics.rs` "WHATWG §3.1.3.3" → DOM §3.2 citation drift is fixed on that slot's first touch. Slot id: the W-11 slot `#11-abort-signal-any-dependent-flattening` (registered by PM 454). |
| PM 454 (weight) | The regexp front door's two-axis weight `groups × REGEXP_GROUP_COST + alts × REGEXP_ALT_COST`, with the cap on `groups` only, is within PM 436 R1 (ratified). |
| PM 454 (homes) | Regexp syntax errors surfacing as `TypeError` (spec: `SyntaxError`) → a child of VM P4 slice-plan umbrella 8 (RegExp completion), owner VM P4. The missing `RegExp` constructor → under umbrella 8, owner VM P4 (registration being checked by the PM). |

## §1. Scope

### §1.1 In

- The primitive (`crate::stack_guard` (NEW), PR-1 §4) and its per-thread bound.
- The two re-entry choke points: `VmInner::with_call_mode` entry and `VmInner::resume_generator`
  entry (PR-1 §5.1). Every nested `run()` is reached through one of them (§2 I-1).
- Every unbounded walker in `elidex-js` (PR-1 §5.2, enumerated by the census in §8 M-0 and the
  cut check in §8 M-6).
- The regexp-compile front door and the routing of all six `regress` compile sites through it
  (PR-1 §6).
- The frame cap for plain JS→JS recursion, which never touches the Rust stack (PR-2).
- The PM 424 rewording of the `MAX_JSON_DEPTH` docstring.

### §1.2 Out (with the slot that owns each)

| Out-of-scope surface | Why out | Owner slot |
|---|---|---|
| Recursive DOM walkers outside `elidex-js` (`elidex-dom-api` serialize / text collection / normalize, `elidex-form`, `elidex-ecs` shadow walks, `elidex-html-parser` convert, …; census `census-host.txt`, §8 M-0) | PM 436 R2. They run on the VM thread, so the guard's reserve gives them headroom near the limit but no guarantee: `innerHTML` / `outerHTML` on a 3000-deep tree abort in debug today (§8 M-7) | `#11-dom-walkers-iterative-over-ecs-links` (L3, NEW) |
| Style / layout / paint recursion (`elidex-css` selector matching, calc / media parse-eval, layout tree walks) | PM 436 R3 | `#11-render-pipeline-recursion-depth-abort` (L2, NEW) |
| Removing the Rust recursion of native→JS callbacks altogether (one dispatch loop) | Successor ideal, T3. The guard stays as the safety bound either way | `#11-vm-native-callback-reentry-single-loop` |
| Iterative JSON walkers | The probe makes the abort impossible; iterative walkers remove the frame-size dependence of `MAX_JSON_DEPTH` | `#11-vm-json-walkers-iterative` |
| Returning `""` for a cyclic `join` | A compat behaviour; core gives the spec `RangeError` (PM 424) | `#11-vm-array-join-cycle-detection` (compat) |
| `AbortSignal.any` chain recursion and its GC defect (DOM "create a dependent abort signal" source flattening) | PM 454: own terminal slot, lands **before** PR-1 (§4); the algorithm belongs in an engine-independent crate, not `vm/host/` | the W-11 slot `#11-abort-signal-any-dependent-flattening` (registered by PM 454) (L3) |
| Regexp syntax errors surfacing as `TypeError` (spec: `SyntaxError` / early error) at the literal / `match` / `search` / structuredClone compile sites | PM 454; PR-1's front door passes `Syntax` through unchanged | child of VM P4 slice-plan umbrella 8 (RegExp completion) (VM P4) |
| The missing `RegExp` constructor | PM 454; PR-1 covers it through I-7 once it calls the front door | under VM P4 slice-plan umbrella 8 (VM P4; registration being checked by the PM) |

## §2. Coupled invariants

Five invariants hold at once. Each pair's intersection is stated with the measurement that pins it.

- **I-1 Coverage** — every Rust-stack recursion reachable from script passes a probe (or is
  bounded by construction) before it can consume more than the reserve.
- **I-2 Fail-safe bound** — the per-thread lower stack bound is the OS-reported one; when it is
  unknown the guard trips early (spurious `RangeError`), never late (abort).
- **I-3 State integrity** — a probe that fails leaves VM state as if the guarded operation never
  started (no half-entered frame, no generator marked Running). (Abort propagation carries no
  probe at all: after the W-11 slot it is depth 1 by construction, PM 454.)
- **I-4 Semantic caps unchanged** — `MAX_JSON_DEPTH`, `MAX_NESTING_DEPTH`, the regexp nesting cap
  stay the profile-stable semantic answers; the probe fires only where the cap would not save the stack.
- **I-5 Hot-path cost** — the probe on `with_call_mode` / `resume_generator` / per-walker-level is
  cheap enough not to need a fast-path exemption.

| Pair | Intersection | Measured |
|---|---|---|
| I-1 × I-2 | Coverage is only as good as the bound: the reserve must exceed the largest **unprobed** stack segment between two probes, on the bound the OS reports. | Minimum reserve with walker probes: ~31 KiB debug / ~15 KiB release (16 KiB resolution); 256 KiB chosen (PR-1 §4.3). macOS bound = OS-reported (`BOUND src=1`, span 2 095 263 B from the first probe on a 2 MiB thread). Linux / Windows: CI cell `stack_guard_bound_is_os_reported` (§7). |
| I-1 × I-3 | Each choke-point probe sits before the first state mutation of the guarded operation, so the failing path returns `Err` with nothing to unwind. | `with_call_mode`: probe before the `entry_frames` snapshot. `resume_generator`: before the generator is taken / marked Running. Mutants M-CP1 / M-CP2 (PR-1 §9) show the corresponding cells abort when the probe is removed. |
| I-1 × I-4 | A cap that the stack cannot reach on the target thread hides a missing probe; a probe that fires before the cap changes the semantic answer. | JSON walkers on a 2 MiB thread, guard OFF, need: parse 1184 / 368 KiB, stringify 1472 / 480, internalize 1712 / 368, sc 1696 / 480 (debug / release, at cap−10). The cap-boundary cells therefore run on an explicit 8 MiB thread (PR-1 §8) so they test the cap, and separate cells on 2 MiB test the probe. |
| I-1 × I-5 | Probing every walker level and every callback entry is what makes coverage local and auditable; it is affordable only if one probe is a TLS read + SP read + compare. | Release, lean probe vs probe compiled out, min of 7 on a loaded host: `forEach` 5·10⁶ callbacks +0.4 %, 3·10⁶ generator resumes −0.8 %, JSON parse+stringify +0.5 %, structuredClone +1.7 %; unprobed control −2.6 % (noise band ±3 %). PR-1 §10. |
| I-2 × I-3 | An unknown bound fails early, which surfaces as a `RangeError` from an operation that has not started; I-3 makes the early failure harmless. | Fallback path forced (`SG_FORCE_UNKNOWN`): bound = first-probe SP − 1 MiB (PR-1 §4.2). |
| I-2 × I-4 | The cap must be reachable on the smallest supported thread or the cap cell silently tests the probe instead. | Covered by the explicit-thread rule for cap cells (PR-1 §8). |
| I-2 × I-5 | The bound is computed once per thread (lazy, cached in a `thread_local!` `Cell`), so the OS query is never on the hot path. | One `pthread_*` / `GetCurrentThreadStackLimits` call per thread. |
| I-3 × I-4 | Hitting a cap and hitting the probe must both be clean errors at the same depth-check site, so a walker has one error exit. | JSON: probe sits at the existing `depth > MAX_JSON_DEPTH` site; two messages, one exit (PR-1 §5.2). |
| I-3 × I-5 | The probe is a pure read; it adds no state to roll back. | `ensure_stack` mutates only the lazily-initialised bound cell. |
| I-4 × I-5 | Keeping the caps means the common shallow case never reaches the probe's error branch. | — (structural) |

The edge-dense trigger fires (5 intersecting axes; no canonical algorithm, since no spec defines
the limit), so each PR goes through `/elidex-plan-review` before implementation.

## §3. Spec coverage map

Spec coverage is thin: no spec defines a stack or nesting limit. The rows below are the
algorithms whose recursion the guard bounds, plus §17, which is cited for what it does **not**
contain. A probe failure is never a spec step; it is the elidex resource limit (PM 424).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | prose | permitted deviations (none is a resource limit) | `crate::stack_guard` (NEW) `StackExhausted` → `RangeError` (NEW) | ✓ | yes |
| ECMA-262 §23.1.3.13.1 FlattenIntoArray | recursive call when depth > 0 | depth = +∞ / finite | `flat_into` | ✓ | yes |
| ECMA-262 §25.5.2 JSON.parse | parse of text | nested arrays / objects | `parse_array` / `parse_object` (methods of `JsonParser`) | ✓ | yes |
| ECMA-262 §25.5.2.4 InternalizeJSONProperty | recursive call per element / key | array / object holder | `internalize` | ✓ | yes |
| ECMA-262 §25.5.4.5 SerializeJSONObject | SerializeJSONProperty per key | nested object | `serialize_object` (method of `JsonSerializer`) | ✓ | yes |
| ECMA-262 §25.5.4.6 SerializeJSONArray | SerializeJSONProperty per index | nested array | `serialize_array` (method of `JsonSerializer`) | ✓ | yes |
| ECMA-262 §13.2.7.3 Runtime Semantics: Evaluation | RegularExpressionLiteral | pattern compile | `load_constant` `Constant::RegExp` | ✓ | yes |
| ECMA-262 §22.1.3.13 String.prototype.match | RegExpCreate for a non-RegExp argument | string / non-RegExp object | `native_string_match` (2 sites, verified 2026-11-06: `natives_string.rs` lines 569/584) | ✓ | yes |
| ECMA-262 §22.1.3.21 String.prototype.search | RegExpCreate for a non-RegExp argument | string / non-RegExp object | `native_string_search` (2 sites, verified 2026-11-06: `natives_string.rs` lines 648/668) | ✓ | yes |
| ECMA-262 §22.2.3.1 RegExpCreate | pattern parse + compile | (future `RegExp` constructor) | front door (NEW) | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal | recursive serialize per property / element | Array / ordinary / Error / RegExp re-compile | `clone_recursive` | ✓ | yes |
| WHATWG HTML §2.7.11 Structured cloning API | structuredClone(value) | — | `clone_recursive` entry | ✓ | yes |

**Breadth**: K=2 specs, M=12 entries (verified 2026-11-06 by `preflight.py`'s own row count, after PM 454 removed the DOM `AbortSignal` row; the
row now belongs to the W-11 slot).
**Split decision**: K<4 and M<20 → single PR by breadth; the program is split for the edge-dense
reason (§2) and the PM 436 split, not by breadth.

### §3.1 User-input touch audit

Every touched site takes script-controlled depth: JSON text, object graphs, array nesting, regexp
pattern text, script source nesting, callback recursion. (`AbortSignal.any` chains are the W-11
slot's, PM 454.) Adjacent
pre-existing code whose exposure changes:

- `ScopeState::enter_recursion` returns `false` past `MAX_NESTING_DEPTH` without recording an
  error (silent subtree skip). Exposure today: none, because the parser's own
  `enter_recursion` errors first at the same constant. PR-1 makes the scope probe failure record an
  error (PR-1 §5.2), so the silent skip is never reached through the probe.
- The literal / `match` / `search` compile sites map a `regress` syntax error to `TypeError`
  (spec: early error / `SyntaxError`). Unchanged by this program; home = a child of VM P4
  slice-plan umbrella 8 (RegExp completion), owner VM P4 (PM 454).

## §4. Per-PR split

| PR | Content | Depends on | Unblocks |
|---|---|---|---|
| W-11 slot (not part of this program's PRs; PM 454) | DOM "create a dependent abort signal" source flattening in an engine-independent crate; GC repair; `abort_statics.rs` citation drift fix | — | PR-1 |
| PR-1 | Primitive; choke points; walker probes (JSON ×5 sites — verified 2026-11-06: `parse.rs` lines 278/321/410, `stringify.rs` lines 144/223 — structuredClone, `flat_into` replacing `MAX_FLAT_DEPTH`, parser / scope / compiler entries, IndexedDB storable check); regexp front door + 6 compile sites; PM 424 rewording; explicit-thread JSON cells | the W-11 slot (landed) | PR-2; the successor T3 slot |
| PR-2 | Frame cap: `frames.len()` cap mapped to the same `RangeError`, for plain JS→JS recursion (heap `CallFrame`s, no Rust recursion) | PR-1 (shares the error constructor and message) | — |

Edges: **W-11 slot → PR-1 is a hard edge** (PM 454). PR-1 cannot land correctly without it:
PR-1 puts no probe on `abort_signal` propagation and treats it as cut in the census (PR-1 §5.3), so
without the slot the `AbortSignal.any` chain stays an unprobed script-reachable Rust recursion
(guard off today, debug, 2 MiB: a kept chain of 1000 aborts the process, PR-1 §11 M-AB), I-1 does
not hold, and PR-1 could not close `#11-vm-call-depth-guard`. Adding a probe there instead is not
an option (it would throw from `controller.abort()` with a half-propagated abort, breaking I-3).
PR-1's check of the edge is in PR-1 §2 (A-1). PR-2 reuses PR-1's `StackExhausted`→`RangeError` mapping so the two limits produce one
error shape; PR-2 does not touch the probe. The successor `#11-vm-native-callback-reentry-single-loop`
removes nested `run()`; after it, the choke-point probes become walker-only, which is a deletion,
not a redesign.

## §5. Slot homing

| Slot | State after this program | Home |
|---|---|---|
| the W-11 slot `#11-abort-signal-any-dependent-flattening` (registered by PM 454) | **Closed before PR-1** (its own terminal slot, PM 454) | L3 |
| `#11-vm-call-depth-guard` | **Closed by PR-1** (the Rust-stack half) | this umbrella |
| `#11-vm-js-frame-depth-limit` | **Closed by PR-2** | this umbrella |
| `#11-vm-json-walkers-iterative` | Stays open (T3 engineering ideal). Its premise changes: the abort is gone, the remaining defect is the frame-size dependence of `MAX_JSON_DEPTH` | VM P4 |
| `#11-vm-native-callback-reentry-single-loop` | Stays open, successor (T3) | VM P4 |
| `#11-vm-array-join-cycle-detection` | Stays open, compat. Under PR-1 a cyclic `join` gives the catchable spec `RangeError` (cell `rt_cyclic_join_range_error`, PR-1 §8) | compat (no lane) |
| `#11-dom-walkers-iterative-over-ecs-links` | **NEW**, PM 436 R2 | L3 |
| `#11-render-pipeline-recursion-depth-abort` | **NEW**, PM 436 R3 | L2 |

## §6. Security relevance

A renderer abort (SIGABRT from Rust's stack-overflow handler) or a hang is a denial of service on
the tab, reachable from any page with a few lines of script (`[1].map(f)` recursion, a deep
`JSON.parse`, one 2000-deep regexp literal). Under the process model the blast radius is the
renderer process, so every tab sharing it is lost too. The guard turns each of these into a
catchable exception inside the page's own realm. This is a "security by structure" fix in the
sense of the design philosophy: the bound is enforced by one primitive at choke points, not by
per-feature review.

## §7. Platform coverage

| Platform | Bound source | Measured where |
|---|---|---|
| macOS (aarch64) | `pthread_get_stackaddr_np` / `pthread_get_stacksize_np` | locally: OS-reported (`src=1`), span from first probe 2 095 263 B on a 2 MiB thread (§8 M-5) |
| Linux (glibc) | `pthread_getattr_np` + `pthread_attr_getstack` | CI `check` job, `ubuntu-latest` leg: cell `stack_guard_bound_is_os_reported` (PR-1 §8) asserts source = OS and span within [thread size − 64 KiB, thread size] on an explicit 2 MiB thread, and prints the span. Not measurable locally (no Linux toolchain or image on the authoring host) |
| Windows (MSVC) | `GetCurrentThreadStackLimits` (windows-sys `Win32_System_Threading`, already enabled in the workspace by `elidex-sandbox`) | CI `check` job, `windows-latest` leg: the same cell |
| Anything else (no OS query implemented) | Fallback: first-probe SP − 1 MiB (fail early, I-2) | the same cell asserts the fallback is **not** taken on the three CI OSes |

## §8. Measurements (provenance)

All under the session scratchpad `stackguard/` directory (`S` below); the prototype clone is
`S/proto` (prototype probe = the clone's `vm/stack_guard.rs` (prototype only)), driver
example `examples/cd_probe.rs` in the clone. Thread size 2 MiB unless stated.

- **M-0 census**: `python3 -I S/recursion_census.py <wt> crates/script/elidex-js` → 22 direct
  self-recursions, 10 SCCs; identical at `efe8d05b` and `4728972c`. Host crates: `census-host.txt`.
- **M-1 re-entry classes**: `S/run.sh <debug|release> <tag> [SG_OFF=1]`. Guard OFF: 28 of 32 abort
  (`plain`, `bound_new`, `plain_new`, `replace` do not recurse on the Rust stack). Guard ON (256 KiB):
  all 28 give a catchable `RangeError` (`async_sync` as a rejected promise); debug stops at d≈30–42,
  release at d≈250–409.
- **M-2 walker thread need** (guard OFF): `S/need.sh <profile> <walker> <cap>`; numbers in §2.
- **M-3 minimum reserve**: `S/bisect.sh <profile> <walker> <cap>` → ~31 KiB debug / ~15 KiB release.
- **M-4 regexp / parser stack**: `S/needcode.sh` with `CODE=…`; debug ≈5.0 KiB per regexp group
  level, release ≈1.3 KiB; parser ≈5.3 KiB per paren level in debug. Alternation is a second axis
  (PR-1 §6.2).
- **M-5 bound source**: `BOUND src=1` line of any `cd_probe` run (`S/rx.log`).
- **M-6 cut check**: `python3 -I S/cutcheck.py <wt> crates/script/elidex-js --marker … --probe …`
  (exact command in PR-1 §5.3) → no residual cycle outside the classified set.
- **M-7 DOM walkers** (out of scope, evidence for R2): `S/dbg-dom.log`.
