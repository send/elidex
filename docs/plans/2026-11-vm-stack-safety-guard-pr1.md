# VM stack-safety guard — PR-1 plan (primitive, choke points, walkers, regexp front door)

Status: **plan (pre-implementation)**; `/elidex-plan-review` required before code (edge-dense,
umbrella §2). Parent: [umbrella](2026-11-vm-stack-safety-guard.md). Closes the Rust-stack half of
`#11-vm-call-depth-guard`. Base: `4728972c`. Rulings PM 421 (a)/(b), PM 424, PM 436 R1/R2/R3/extra/split,
PM 454 are ratified (umbrella §0.1) and not re-opened here. Precondition: the W-11 slot `#11-abort-signal-any-dependent-flattening`
(registered by PM 454) has landed (umbrella §4 edge; §2 A-1).

## §1. Scope

PR-1 delivers, in one PR (the umbrella's base-case slice):

1. The primitive `crate::stack_guard` (NEW) (§4).
2. Probes at the two re-entry choke points (§5.1).
3. Probes in every unbounded `elidex-js` walker (§5.2): JSON parse / internalize / stringify,
   structuredClone, `flat_into` (replacing `MAX_FLAT_DEPTH`), the parser / scope analyser /
   compiler recursive entries, the IndexedDB storable pre-check.
4. The regexp-compile front door and the routing of all six compile sites (§6).
5. The PM 424 rewording of the `MAX_JSON_DEPTH` docstring (§7).
6. Cells, mutants, the explicit-thread rule and the hot-path measurement (§8–§10).

Not in PR-1: the frame cap (PR-2), and everything in umbrella §1.2 — in particular the
`AbortSignal.any` source-signal flattening, which is the W-11 slot's and lands before PR-1 (PM 454).

## §2. Coupled invariants

The umbrella §2 table (I-1 coverage, I-2 fail-safe bound, I-3 state integrity, I-4 semantic caps,
I-5 hot-path cost, and each pair) is this PR's invariant set; PR-1 is where all five are realised.
PR-local additions:

- **I-6 One error shape** — every probe failure is the same `RangeError` message, whatever the
  walker; cap failures keep their own messages (`DEPTH_EXCEEDED_MSG`, `NestingTooDeep`, the regexp
  nesting message). I-6 × I-4 = a cell can tell "cap" from "probe" by message alone (used by the
  mutants, §9).
- **I-7 Front door is the only compile path** — no `regress::Regex::new` / `with_flags` outside the
  front-door module. I-7 × I-1 = a future `RegExp` constructor is covered the moment it calls the
  front door, and a trip-wire (§6.4) makes the other path a CI failure.

Assumption from the W-11 edge (PM 454):

- **A-1 Abort fan-out is depth 1** — once the W-11 slot has landed, a dependent signal's source
  signals are never dependent (DOM §3.2 "create a dependent abort signal", step 4.2 appends the
  input's source signals, not the input), so `abort_signal`'s fan-out over dependent signals is
  depth 1 by construction and needs no probe (§5.2 W-11). PR-1 checks A-1 two ways: (a) the PR
  body records `git merge-base --is-ancestor <W-11 slot landing sha> HEAD` → exit 0; (b) the cell
  `abort_any_chain_depth_one_precondition` (§8) builds a nested chain that aborts the process if
  the fan-out were still recursive, so a revert of the slot turns PR-1's suite red. A-1 × I-1 = the
  cut check (§5.3) treats `abort_signal` as cut on the strength of A-1, not of a probe.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| ECMA-262 §17 Error Handling and Language Extensions | prose | no resource-limit clause | `StackExhausted` → `RangeError` (NEW) | ✓ | yes |
| ECMA-262 §23.1.3.13 Array.prototype.flat ( [ depth ] ) | depthNum = +∞ | `flat(Infinity)` no longer truncated at 100 | `native_array_flat` | ✓ | yes |
| ECMA-262 §23.1.3.13.1 FlattenIntoArray | recursive call, depth > 0 | +∞ / finite | `flat_into` | ✓ | yes |
| ECMA-262 §25.5.2 JSON.parse | text parse | array / object | `parse_array` / `parse_object` | ✓ | yes |
| ECMA-262 §25.5.2.4 InternalizeJSONProperty | per element / key recursion | array / object | `internalize` | ✓ | yes |
| ECMA-262 §25.5.4.5 SerializeJSONObject | per key | nested object | `serialize_object` | ✓ | yes |
| ECMA-262 §25.5.4.6 SerializeJSONArray | per index | nested array | `serialize_array` | ✓ | yes |
| ECMA-262 §13.2.7.3 Runtime Semantics: Evaluation | RegularExpressionLiteral | compile on first load | `load_constant` | ✓ | yes |
| ECMA-262 §22.1.3.13 String.prototype.match | RegExpCreate | string / non-RegExp object | `native_string_match` ×2 | ✓ | yes |
| ECMA-262 §22.1.3.21 String.prototype.search | RegExpCreate | string / non-RegExp object | `native_string_search` ×2 | ✓ | yes |
| ECMA-262 §22.2.3.4 Static Semantics: ParsePattern | pattern parse | groups / disjunction | front door `compile_regexp` (NEW) | ✓ | yes |
| WHATWG HTML §2.7.4 StructuredSerializeInternal | recursion per property / element; RegExp re-compile | Array / ordinary / Error / RegExp | `clone_recursive`, `clone_regexp` | ✓ | yes |

**Breadth**: K=2 specs, M=12 entries (verified 2026-11-06 by `preflight.py`'s own row count, after PM 454 moved the DOM `AbortSignal` row to the
W-11 slot).
**Split decision**: K<4 and M<20 → single PR (the umbrella's base-case slice).

### §3.1 User-input touch audit

All rows are script-controlled. Adjacent pre-existing code:
- `ScopeState::enter_recursion` silently skips a subtree past `MAX_NESTING_DEPTH` (no error).
  Exposure unchanged: the parser errors first at the same constant. The probe branch added here
  records an error (§5.2 W-8), so it does not inherit the silent skip.
- The four `natives_string.rs` sites and the literal site map a `regress` syntax error to
  `TypeError` (spec: `SyntaxError` / early error). Unchanged; the front door returns the syntax
  error to the caller untouched (§6.3). Home: a child of VM P4 slice-plan umbrella 8 (RegExp
  completion), owner VM P4 (PM 454).
- `AbortSignal.any` chains whose intermediate composites are unreachable lose propagation after a
  GC today (measured: chain of 10 000 with no references kept → `s.aborted === false` after
  `c.abort()`; §11 M-AB). Not PR-1's: the W-11 slot's flattening removes the intermediate links and
  fixes this before PR-1 (PM 454).

## §4. The primitive (NEW)

### §4.1 API

File `crates/script/elidex-js/src/stack_guard.rs` (NEW), `pub(crate) mod stack_guard` in
`lib.rs`. Crate root, not `vm/`, because the parser, scope analyser and compiler run without a
`VmInner`.

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
`From<StackExhausted> for CompileError`; a new `JsParseErrorKind::StackExhausted` (NEW) variant
pushed by the parser / scope probe branches. There is exactly one primitive (PM 436 R1): walkers
and choke points call `ensure_stack(WALKER_RESERVE)?`; the regexp front door calls
`ensure_stack(WALKER_RESERVE + weight_cost)` (§6.2). No second "check" function.

How the parse / compile error surfaces: `compile_script` turns any parse or scope error into
`CompileError` today, and elidex has no `eval` / `Function` constructor, so a too-deep source is
reported as the script's compile failure (the same channel as `NestingTooDeep`). When `eval` /
`Function` land, `StackExhausted` maps to `RangeError` there; that mapping belongs to those
features, recorded here so it is not re-decided.

### §4.2 Per-thread bound: where computed, where cached, unknown bound

- **Cached** in a `thread_local!` `const`-initialised `Cell<usize>` (`LOW`, 0 = not yet known) in
  `stack_guard.rs`. The stack is a per-thread resource, not a per-entity or per-VM fact, so a
  thread-local is its owner (not an ECS side-store question; nothing is entity-keyed).
- **Computed** lazily on a thread's first `ensure_stack` call, once:
  - macOS: `pthread_get_stackaddr_np(pthread_self())` (top) − `pthread_get_stacksize_np` → low.
  - Linux: `pthread_getattr_np` + `pthread_attr_getstack` → low (the attr's address is the low end); `pthread_attr_destroy`.
  - Windows: `GetCurrentThreadStackLimits(&low, &high)` → low.
  - SP: address of a `black_box`ed local in `ensure_stack` itself.
  - `remaining = sp.saturating_sub(low)`; `Err` iff `remaining < need`.
- **Unknown bound** (other targets, or the OS call fails): **fail early**. `LOW` is set to the
  first probe's SP − `FALLBACK_SPAN` (`FALLBACK_SPAN` = 1 MiB, NEW). Justification (I-2): the
  two failure directions are a spurious catchable `RangeError` (guard trips while stack remains)
  and a process abort (guard trips after the stack is gone). Only the first is recoverable, so an
  unknown bound must fall on that side. 1 MiB is half the smallest production thread (2 MiB, plain
  `std::thread::spawn`), and the first probe runs near the top of the thread (the VM's first
  native call or first parse), so the assumed span is below what is actually there on every
  thread elidex creates. A thread created smaller than `FALLBACK_SPAN` by an embedder on an
  unsupported target is the residual risk; it is stated, not engineered around, because no
  supported target takes this branch (cell `stack_guard_bound_is_os_reported`, §8).
- Stack switching: none in the workspace (no `stacker` / `psm` / `corosensei` in `Cargo.lock`,
  checked 2026-11-06), so the cached bound stays valid for the thread's lifetime. A future
  stack-switching dependency invalidates the cache; the cell above does not catch that, so the
  constraint is written in the module doc.
- Dependencies: `libc = "0.2"` under `cfg(unix)` and `windows-sys = { version = "0.61", features =
  ["Win32_System_Threading"] }` under `cfg(windows)` become direct deps of `elidex-js`. Both are
  already in `Cargo.lock` (`libc 0.2.191`, `windows-sys 0.61.2`) and already direct deps of
  `elidex-sandbox` with that feature set, so no new crate and no `deny.toml` change.

### §4.3 Reserve sizing

`WALKER_RESERVE` must exceed the largest stack segment between two probes (I-1 × I-2), including
building the `RangeError` and unwinding to the catch.

| Measurement | debug | release | Command |
|---|---|---|---|
| Minimum reserve at which every walker, run at the callback floor (deepest point the choke-point probe allows), does not abort | ~31 KiB (flat with `MAX_FLAT_DEPTH` still in place: ~46 KiB) | ~15 KiB | `S/bisect.sh <debug\|release> <walker> <cap>` for walker ∈ none parse stringify internalize sc flat error |
| Re-entry depth reached with 256 KiB | d≈30–42 | d≈250–409 | `S/run.sh <profile> ON256` |

(16 KiB resolution; `S` = the session scratchpad `stackguard/` directory; prototype at
the clone's `vm/stack_guard.rs` (prototype only), env `SG_RESERVE` / `SG_OFF` /
`SG_FORCE_UNKNOWN`.)

256 KiB is 8× the debug minimum and 16× the release minimum. The margin is deliberate: the
out-of-scope DOM walkers (umbrella §1.2, `#11-dom-walkers-iterative-over-ecs-links`) and any host
native that recurses without a probe run inside that reserve. It costs 12.5 % of a 2 MiB thread.
One constant for both profiles: the measured minimum is far below it in either. The ratchet cell
`stack_guard_reserve_margin_ratchet` (§8) re-runs the floor cells with a test-only reserve of
64 KiB (2× the debug minimum); frame growth that eats the margin fails that cell long before the
production reserve is threatened.

## §5. Call sites

### §5.1 Choke points

| Id | Site | Placement | Why this covers it |
|---|---|---|---|
| CP-1 | `VmInner::with_call_mode` (`crates/script/elidex-js/src/vm/interpreter.rs:160`) | first statement, before the `entry_frames` / `entry_stack_depth` snapshot | every native→JS call (`call`, `call_construct_native`, `run_function`) and `construct_synchronous` (`crates/script/elidex-js/src/vm/dispatch_class.rs:323`) enter through it; the nested `run()` sites at `interpreter.rs:740` and `dispatch_class.rs:364` are inside it |
| CP-2 | `VmInner::resume_generator` (`crates/script/elidex-js/src/vm/natives_generator.rs:114`) | first statement, before the generator state is taken and marked Running | the nested `run()` at `natives_generator.rs:276`; reached both from the generator natives (`natives_generator.rs:374/404/432`) and, outside `with_call_mode`, from inline async calls (`push_js_call_frame` → `drive_async_coroutine` → `natives_generator.rs:472`) |

Enumeration of nested `run()` (verified 2026-11-06 via `git grep -nE '(self|vm|inner|ctx\.vm)\.run\(' -- crates/script/elidex-js/src/vm ':!**/tests/**'`): 3 sites, lines `dispatch_class.rs:364` / `interpreter.rs:740` / `natives_generator.rs:276`; each is behind CP-1 or CP-2.
Callers of `with_call_mode` outside tests (verified via `git grep -n 'with_call_mode(' -- crates/script/elidex-js/src/vm`): `interpreter.rs` lines 82/102/1010 and `dispatch_class.rs:323`.

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
| W-6 | `clone_recursive` | `crates/script/elidex-js/src/vm/host/structured_clone.rs:88` | none today (PM 421 (b): the probe **is** its bound) | covers the `clone_array` / `clone_ordinary` / `clone_error` SCC; structuredClone has one fused serialize+deserialize walk, not two |
| W-7 | Parser `enter_recursion` | `crates/script/elidex-js/src/parser/mod.rs:471` | `MAX_NESTING_DEPTH` | probe after the cap check; failure pushes `StackExhausted` and returns `false` (same contract as the cap) |
| W-7b | `parse_class_inner` | `crates/script/elidex-js/src/parser/function.rs:345` | **none today** (census hole, §5.3) | add `enter_recursion` / `leave_recursion` around the heritage parse, so the class-heritage chain counts toward `MAX_NESTING_DEPTH` and gets the probe |
| W-8 | Scope `enter_recursion` | `crates/script/elidex-js/src/scope/mod.rs:205` | `MAX_NESTING_DEPTH` | probe failure records `StackExhausted` in the analysis errors before returning `false` |
| W-9 | Compiler entries `compile_expr` / `compile_stmt` / `compile_destructure_pattern` / `compile_nested_function` | `crates/script/elidex-js/src/compiler/expr.rs:74`, `compiler/stmt.rs:19`, `compiler/stmt_destructure.rs:23`, `compiler/expr_function.rs:137` | parse-depth (indirect) | `ensure_stack(WALKER_RESERVE)?` via `From<StackExhausted> for CompileError` |
| W-10 | `flat_into` | `crates/script/elidex-js/src/vm/natives_array_hof.rs:327` | `MAX_FLAT_DEPTH` = 100 (**deleted**) | `native_array_flat` passes `+∞` as `usize::MAX` and a finite depth unclamped; `flat(Infinity)` then gives FlattenIntoArray's result, and a cyclic array gives the probe's `RangeError` |
| W-11 | `abort_signal` composite fan-out | `crates/script/elidex-js/src/vm/host/abort.rs:607` / `abort_statics.rs` `native_abort_signal_static_any` | none | **not touched by PR-1** (PM 454): the W-11 slot implements DOM "create a dependent abort signal" step 4.2 (a dependent input contributes its source signals, not itself) in an engine-independent crate before PR-1. Composites are then never inputs, so the fan-out is depth 1 by construction (A-1, §2). **No probe**: one would leave a half-propagated abort and throw from `controller.abort()`, breaking I-3 |
| W-12 | `reject_non_json_storable` | `crates/script/elidex-js/src/vm/host/indexeddb/value.rs:242` | none (runs on the structuredClone output, so its depth is whatever the clone allowed) | probe; its per-level frame differs from `clone_recursive`'s, so the clone's probe does not bound it |

### §5.3 Enumeration method and the classified remainder

Census (heuristic, name-based; blind to trait dispatch and closures across functions):
`python3 -I S/recursion_census.py <wt> crates/script/elidex-js` → 22 direct self-recursions and
10 SCCs (unchanged between `efe8d05b` and `4728972c`). Cut check: remove the probe set from the
call graph and list every cycle that survives:

```
python3 -I S/cutcheck.py <wt> crates/script/elidex-js \
  --marker 'enter_recursion\s*\(' --marker 'with_call_mode\s*\(' \
  --probe resume_generator --probe compile_expr --probe compile_stmt \
  --probe compile_destructure_pattern --probe compile_nested_function \
  --probe parse_array --probe parse_object --probe internalize \
  --probe serialize_array --probe serialize_object --probe clone_recursive \
  --probe flat_into --probe reject_non_json_storable --probe abort_signal \
  --probe drain_microtasks --probe parse_class_inner
```

`--probe abort_signal` marks a cut point, not a probe site: `abort_signal` is cut on A-1 (§2), i.e.
on the W-11 slot having landed, and carries no probe.

Result (`S/cutcheck-pr1-v2.log`): 8 residual self-recursions, 2 residual SCCs, each classified:

| Residual | Class | Reason |
|---|---|---|
| `to_number`, `to_string` (`vm/coerce.rs`), `abstract_eq` (`vm/coerce.rs:753`), `abstract_relational` (`vm/coerce_ops.rs:131`) | bounded by argument | recurse once on a primitive produced by ToPrimitive / ToNumber |
| `write_number_es` (`vm/coerce_format.rs:132`) | bounded by argument | number formatting, fixed depth |
| `js_to_idb_key_depth` (`vm/host/indexeddb/value.rs:123`) | bounded by cap | `MAX_KEY_DEPTH` = 64 |
| `idb_key_to_js` (`vm/host/indexeddb/value.rs:196`) | bounded by construction | keys only enter the store through the 64-capped path |
| `disassemble_function` (`bytecode/disasm.rs:23`) | not reachable from script | only caller is `compiler/mod.rs` `#[cfg(test)]` |
| `crate::regexp` parser SCCs (`regexp/parser.rs`, `regexp/class_set.rs`) | not reachable from script | `parse_pattern*` has no non-test caller (`git grep -n 'parse_pattern' -- crates/script/elidex-js/src ':!src/regexp/**'` → 0 hits outside tests); production compiles through `regress` (§6) |
| event / microtask SCC (`drain_microtasks` … `walk_phase`) | cut by a re-entry flag | `drain_microtasks` returns at once when `microtask_drain_depth > 0` (`vm/natives_promise.rs:275`); every other cycle in the SCC calls JS, i.e. passes CP-1 |

Before cut, two more cycles survived and became W-7b and W-9's `compile_nested_function`:
`parse_class_expression_node | parse_class_inner | parse_primary | parse_primary_and_suffix`
(class heritage never calls `enter_recursion`) and `compile_nested_function` (self-recursive).
The class-heritage hole is real: guard off, release, `var C = class extends`×N `Object {}` + `{}`×(N−1)
parses at N=2000 on a 5.5 MiB thread (≈2.8 KiB/level, no cap) and aborts at N=5000 on 8 MiB;
debug aborts at N=300 on 2 MiB (`S/rel-classext.log`, `S/dbg-census-holes.log`).

## §6. Regexp-compile front door (PM 436 R1)

### §6.1 Compile sites

Verified 2026-11-06 via
`git grep -nE 'regress::Regex|Regex::(with_flags|new|from_unicode)' -- crates/script/elidex-js/src ':!**/tests/**'`
— 6 compile sites (verified 2026-11-06), lines `natives_string.rs` 569/584/648/668, `dispatch_helpers.rs` 170, `structured_clone.rs` 585:

| Site | Path |
|---|---|
| `crates/script/elidex-js/src/vm/natives_string.rs:569` | `String.prototype.match`, non-object argument |
| `crates/script/elidex-js/src/vm/natives_string.rs:584` | `match`, non-RegExp object argument |
| `crates/script/elidex-js/src/vm/natives_string.rs:648` | `String.prototype.search`, non-object argument |
| `crates/script/elidex-js/src/vm/natives_string.rs:668` | `search`, non-RegExp object argument |
| `crates/script/elidex-js/src/vm/dispatch_helpers.rs:170` | regexp literal (`Constant::RegExp` in `load_constant`) |
| `crates/script/elidex-js/src/vm/host/structured_clone.rs:585` | structuredClone RegExp re-compile |

There is no `RegExp` constructor global today (`RegExp is not defined` in the prototype runs), so
"`RegExp(...)`" is not a current site; it is covered by I-7 when it lands. Home: under VM P4
slice-plan umbrella 8 (RegExp completion), owner VM P4 (PM 454; registration being checked by the PM). Other crates use the
`regex` crate (`elidex-api-sw` router, `elidex-form-core` validation), which is not `regress` and
not script-recursive; out of scope.

### §6.2 Weight model, cap and per-level cost

Ratified (PM 454): the two-axis weight (`groups × REGEXP_GROUP_COST + alts × REGEXP_ALT_COST`, cap
on `groups` only) is within PM 436 R1.

`regress` recurses on two axes, measured guard off (`S/needbin.sh <bin> <label>` with
`CODE="String('b'.match(<pattern>).index)"`, 16 KiB resolution):

| Axis | debug | release | Data |
|---|---|---|---|
| group nesting `(`…`)` (capturing, `(?:…)*`, lookahead all equal) | ≈5.1 KiB / level | ≈1.3 KiB / level | debug 100→200 levels: 576→1088 KiB; release 500→1000: 672→1312 KiB; `(?:` + `)*` and `(?=` at 250/500: 336/672 KiB |
| alternatives `|` (right-nested binary `Alt` in `regress` 0.11.1 `make_alt`) | ≈0.83 KiB / alternative | ≈0.16 KiB / alternative | debug 250→500: 288→496 KiB; release 1000/2000/4000: 176/336/640 KiB |
| concatenation length | none | none | `'a'.repeat(1e6)` compiles on 2 MiB (debug) |
| match execution length | none | none | `(?:a|b)*$` over 100 000 chars on 2 MiB (debug) |

So the nesting the brief describes is one of two axes; a pattern of 10 000 alternatives aborts a
debug build on 2 MiB with no group nesting at all (`S/rx-shapes-dbg.log`).

Front door (NEW), `crates/script/elidex-js/src/vm/regexp_compile.rs`:

```rust
pub(crate) enum RegExpCompileError { Syntax(regress::Error), TooDeep, Exhausted(StackExhausted) } // (NEW)
pub(crate) fn compile_regexp(pattern: &str, flags: regress::Flags)
    -> Result<regress::Regex, RegExpCompileError>;                                       // (NEW)
```

1. One iterative pass over the pattern (skipping `\` escapes and `[...]` class contents) yields
   `groups` = maximum `(` nesting and `alts` = total `|` count. `alts` over-approximates the
   alternatives on the deepest path, which is the fail-early direction.
2. `groups > MAX_REGEXP_DEPTH` (1024, the existing constant, moved to the front door as its single
   home) → `TooDeep` → `RangeError` "Maximum regular expression nesting depth exceeded" (NEW
   message; same class as the JSON cap). Profile-independent semantic bound.
3. `ensure_stack(WALKER_RESERVE + groups × REGEXP_GROUP_COST + alts × REGEXP_ALT_COST)` →
   `Exhausted` → `RangeError` `STACK_EXHAUSTED_MSG`.
4. `regress::Regex::with_flags(pattern, flags)`; a `regress::Error` is returned as `Syntax` for
   the caller to map as it does today.

Constants (NEW), `cfg(debug_assertions)`-split like `MAX_JSON_DEPTH`, each = measured slope × 1.25,
rounded up to 64 B: `REGEXP_GROUP_COST` debug 6592 B / release 1664 B; `REGEXP_ALT_COST` debug
1088 B / release 256 B. Re-measurement procedure (in the constant's docstring): run
`S/needbin.sh` at two depths per axis per profile, slope = Δneed / Δlevels; the ratchet cells
`rx_group_cost_ratchet` / `rx_alt_cost_ratchet` (§8) fail when the real slope exceeds the constant.

Alternation is not capped (only groups are): a list of thousands of alternatives is legitimate
content, and the stack ask already bounds it. Release fits ~12 000 alternatives on a 2 MiB thread.

### §6.3 Call-site mapping

Each of the six sites replaces its `regress::Regex::new` / `with_flags` call with
`compile_regexp`. `TooDeep` and `Exhausted` become `RangeError` via `From<RegExpCompileError>`
for `VmError` for those two arms; `Syntax(e)` keeps each site's current message and error class
(the class question belongs to a child of VM P4 slice-plan umbrella 8, PM 454; §3.1). The structuredClone site keeps its
`DataCloneError`-free `TypeError` for `Syntax` and propagates the two `RangeError` arms.

### §6.4 Trip-wire (I-7)

A wire in `scripts/trip-wires.sh`: `git grep -nE 'regress::Regex::(new|with_flags)' -- crates/script/elidex-js/src`
must report only `vm/regexp_compile.rs`. Control (in the wire's own self-check, as for the
existing wires): a scratch line outside the module makes the wire fail.

## §7. PM 424 rewording (`crates/script/elidex-js/src/vm/natives_json/mod.rs`)

- Remove the sentence comparing the release value to another engine's (`natives_json/mod.rs:29`) and
  replace it with: "Release uses 1000, verified to hold for all three walkers (depth 990 passes,
  1000+ throws). The value is elidex's own resource limit; ECMA-262 defines none."
- Replace the "(the VM has no call-depth guard: `#11-vm-call-depth-guard`)" clause at line 27 with
  a pointer to `crate::stack_guard` (NEW): re-entry is now bounded by the choke-point probe, and the
  walkers carry their own probe; the cap stays as the semantic bound (PM 421 (a)).
- The "binding constraint is internalize … 2 MiB thread" paragraph stays as the derivation of the
  cap, plus one sentence: past the cap the stack is the probe's concern, so the cap is no longer the
  only thing between the walker and an abort.

## §8. Cells (written first; red on `4728972c` means abort or wrong result)

New file `crates/script/elidex-js/src/vm/tests/tests_stack_guard.rs` (NEW). Every cell runs its
body on an explicit thread via a helper `on_thread(bytes, f)` (NEW) using
`std::thread::Builder::stack_size`; probe cells use 2 MiB (the production thread), cap cells 8 MiB.
A stack overflow aborts the test process, which nextest reports as that test's failure. All cells
run in **debug and release**: `cargo nextest run -p elidex-js --all-features -E 'test(/tests_stack_guard/)'`
and the same with `--release`. CI: the `check` job runs the debug leg on all three OSes; PR-1 adds
a release step for this filter on the `ubuntu-latest` leg (cost measured in the PR; the constants
are profile-split, so a release-only constant is otherwise never exercised).

| Cell | Body | Expect |
|---|---|---|
| `rt_reviver_recursion` | `function r(k, v) { return k === '' ? JSON.parse('[1]', r) : v } JSON.parse('[1]', r)` (declaration: a named function expression has no self-binding in elidex today) | `RangeError` / `STACK_EXHAUSTED_MSG` |
| `rt_map_recursion` | `function f() { [1].map(f) } f()` | same |
| `rt_generator_resume` | `function* g() { g().next(); yield 1 } g().next()` | same |
| `rt_inline_async` | `async function f() { await f() } f().catch(e => globalThis.r = e.name)`, then drain microtasks | `r === 'RangeError'` |
| `rt_super_recursion` | `class B { constructor() { new C() } } class C extends B { constructor() { super() } } new C()` | `RangeError` |
| `rt_cyclic_join_range_error` | `const a = []; a[0] = a; String(a)` | `RangeError` (spec; PM 424) |
| `walker_floor_{parse,internalize,stringify,sc,flat,idb_storable}` | recurse through `[1].forEach` until the first `RangeError`, then run the walker on a cap−10 input at that depth | `RangeError` (probe), no abort |
| `json_cap_{parse,internalize,stringify}` (moved from `tests_json.rs` / `tests_string_complement.rs`, 8 MiB) | cap−10 passes, cap+100 throws | `DEPTH_EXCEEDED_MSG` |
| `rx_nesting_repro` | `'a'.match('('.repeat(2000) + 'a' + ')'.repeat(2000))` | `RangeError`, nesting message (release aborts today, `S/rel-strmatch2000.log`) |
| `rx_alternation_1e5` | `'b'.match('a\|'.repeat(100000) + 'b')` | `RangeError` / `STACK_EXHAUSTED_MSG` (debug aborts at 10 000 today) |
| `rx_literal_nesting` | a 2000-deep regexp literal in source | `RangeError` at literal evaluation |
| `rx_group_cost_ratchet` / `rx_alt_cost_ratchet` | compile depth D on a thread of exactly `baseline + WALKER_RESERVE + D × cost`, D = 1000 groups / 4000 alternatives (release), 200 / 500 (debug) | compiles, no abort |
| `parser_parens_depth` | 2000 nested parens (cap) and 1000 nested parens at the callback floor | `NestingTooDeep`; `StackExhausted` |
| `parser_class_heritage_chain` | `class extends` × 5000 | `StackExhausted` or `NestingTooDeep`, no abort (aborts today) |
| `flat_infinity_deep` | `let a = [1]; for (let i = 0; i < 200; i++) a = [a]; a.flat(Infinity)` | `[1]` (FlattenIntoArray; today depth 100 leaves nesting) |
| `flat_cyclic` | `const a = []; a.push(a); a.flat(Infinity)` | `RangeError` |
| `abort_any_chain_depth_one_precondition` | on a 2 MiB thread, 10 000 chained `AbortSignal.any([s])` with intermediates kept, then `c.abort()` | `s.aborted === true`, no abort. Checks A-1 (§2): it aborts the process if the fan-out is still recursive (today: abort at 1000 kept in debug, §11 M-AB), so it is red on `4728972c` and green only once the W-11 slot has landed. The unreferenced (GC) cell is the W-11 slot's own |
| `stack_guard_bound_is_os_reported` | on a 2 MiB thread, first probe | bound source = OS; span ∈ [2 MiB − 64 KiB, 2 MiB]; prints span (the Linux / Windows measurement, umbrella §7) |
| `stack_guard_unknown_bound_fails_early` | test-only forced-unknown path | first probe at top passes; recursion stops with `RangeError` at ≤ 1 MiB used |
| `stack_guard_reserve_margin_ratchet` | the `walker_floor_*` and `rt_*` bodies with a test-only reserve of 64 KiB | all `RangeError`, no abort |

### §8.1 Tests that move to explicit `stack_size` threads

Verified 2026-11-06 via `git grep -n 'DEPTH_EXCEEDED_MSG\|NestingTooDeep\|MAX_JSON_DEPTH' ` over the `elidex-js` test files (pathspecs `crates/` + `**/tests*` and `**/` + `*_tests*.rs`):
`parse_reviver_deep_acyclic_chain_within_cap_ok` / `parse_reviver_deep_acyclic_chain_over_cap_throws` /
`parse_reviver_cyclic_graph_throws_range_error` (`tests_json.rs`), `json_parse_depth_cap` /
`json_stringify_depth_cap` (`tests_string_complement.rs`), `worker_post_message_hits_the_depth_cap_not_the_stack`
(`tests_json_shortcut_date.rs`), `deep_new_recursion_no_crash` (`parser/stmt_tests.rs`). Each
asserts a **cap** message, so on the libtest thread (whose size follows `RUST_MIN_STACK`) the probe
could fire first and turn a cap assertion into a probe failure. They run on 8 MiB. The comment at
`tests_json.rs:594` ("Cells run on the libtest thread (2 MiB)") is rewritten to say so.

## §9. Mutants

Each row removes one probe (or reverts one structural fix) and names the cell that must die.
Run on the final head only, debug and release.

| Mutant | Removed | Dies |
|---|---|---|
| M-CP1 | CP-1 | `rt_reviver_recursion`, `rt_map_recursion`, `rt_super_recursion`, `rt_cyclic_join_range_error` (abort) |
| M-CP2 | CP-2 | `rt_generator_resume`, `rt_inline_async` |
| M-W1/2 | W-1, W-2 | `walker_floor_parse` |
| M-W3 | W-3 | `walker_floor_internalize` |
| M-W4/5 | W-4, W-5 | `walker_floor_stringify` |
| M-W6 | W-6 | `walker_floor_sc` |
| M-W7 | W-7 | `parser_parens_depth` (floor half) |
| M-W7b | W-7b | `parser_class_heritage_chain` |
| M-W8 / M-W9 | W-8, W-9 | **expected to survive** if scope / compiler frames are smaller than the parser's per level (the parser probe then fires first). PR-1 measures per-level stack for the three passes (`S/needbin.sh` on parens N, parse-only vs full compile); if a pass's slope is below the parser's, its probe stays (it bounds a different frame size) and the mutant's survival is recorded with that measurement, not hidden; if above, a cell compiling a pre-parsed program on a thread between the two needs is added |
| M-W10 | W-10 | `flat_cyclic`; reverting to `MAX_FLAT_DEPTH` kills `flat_infinity_deep` |
| (no M-W11) | — | W-11 is not a PR-1 change (PM 454); reverting the slot's flattening kills `abort_any_chain_depth_one_precondition` (A-1), which is the check, not a PR-1 mutant |
| M-W12 | W-12 | `walker_floor_idb_storable` |
| M-RX1 | front-door `ensure_stack` | `rx_alternation_1e5` (and not `rx_nesting_repro`, which the cap answers: I-6 tells them apart) |
| M-RX2 | front-door cap | `rx_nesting_repro` changes message to `STACK_EXHAUSTED_MSG` |
| M-RX3 | `REGEXP_GROUP_COST` halved | `rx_group_cost_ratchet` |
| M-RX4 | one site bypasses the front door | trip-wire §6.4 |

## §10. Hot-path probe cost

Measured 2026-11-06 on the prototype, release, macOS aarch64, two builds of the same clone that
differ only in the probe body (`SG_MODE` compile-time switch in the prototype `stack_guard.rs`):
`none` = probe compiled out, `lean` = the PR-1 shape (one TLS read of the cached bound, SP read,
compare against a const reserve) at CP-1, CP-2, W-1…W-6. Command: `S/hotpath.sh 7` (7 interleaved
runs per build, in-script `Date.now()` around the workload only, 8 MiB thread), log
`S/rel-hotpath.log`. The host was loaded (load average 20–80, other sessions building), so the
minimum of 7 is reported and the unprobed control row gives the noise band.

| Workload | Probes hit | `none` min (ms) | `lean` min (ms) | Δ |
|---|---|---|---|---|
| `cb`: `forEach` over 10⁶ elements × 5 | 5·10⁶ × CP-1 | 497 | 499 | +0.4 % |
| `gen`: `it.next()` × 3·10⁶ | 3·10⁶ × CP-2 | 1371 | 1360 | −0.8 % |
| `json`: parse + stringify of 10⁵ × 4-deep records, × 5 | ≈4·10⁶ × W-1…W-5 | 999 | 1004 | +0.5 % |
| `sc`: `structuredClone` of the same, × 5 | ≈2·10⁶ × W-6 | 354 | 360 | +1.7 % |
| `plain` (control: inline JS calls, no probe on the path) | 0 | 2316 | 2255 | −2.6 % |

Every probed row is inside the control's ±3 % band, so the cost is below what this host can
resolve; no fast-path exemption is needed (I-5). Acceptance for PR-1: re-run `hotpath.sh` on the
final head (production probe vs a build with `ensure_stack` stubbed to `Ok(())`, same method) and
record it in the PR body; a probed row outside the control band on a quiet host blocks.

## §11. Measurements recorded for this plan

- **M-AB** `AbortSignal.any` chain, debug, guard off, 2 MiB: kept intermediates, N = 10 / 100 / 1000 /
  3000 → `true` / `true` / abort / abort; dropped intermediates, N = 10 000 → `false`
  (`S/dbg-census-holes.log` and the follow-up runs in this session). Evidence for the W-11 slot
  (PM 454) and for the A-1 precondition cell; PR-1 does not change it.
- Regexp axes: `S/rel-rx-axes.log`, `S/rel-rx-axes2.log`, `S/dbg-rx-axes.log`, `S/rx-shapes-dbg.log`.
- Class heritage: `S/rel-classext.log`, `S/dbg-census-holes.log`.
- Cut check: `S/cutcheck-pr1.log` (before W-7b / W-9 additions), `S/cutcheck-pr1-v2.log`.
