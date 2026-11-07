# VM stack-safety guard — PR-1b plan (JS→wasm entries, `MAX_WASM_STACK`)

Status: **plan (pre-implementation)**, split out of [PR-1](2026-11-vm-stack-safety-guard-pr1.md) by
the driver decision D-K (PM 483, umbrella §0.1); `/elidex-plan-review` required before code
(edge-dense program, umbrella §2). Parent: [umbrella](2026-11-vm-stack-safety-guard.md).
**Closes `#11-vm-call-depth-guard`** (the slot closes at this PR's landing, not PR-1's, PM 483).
Base: `47b6c934` (anchors below are at that commit; re-derive each at PR-1b's own base with the
command beside it). Rulings of umbrella §0.1 are ratified and not re-opened here; the decision D-W
(CP-3 + `MAX_WASM_STACK`) is carried over from PR-1 unchanged. Provenance root `H` is defined in
the umbrella header.
Preconditions: **PR-1 has landed** (edge PR-1 → PR-1b): PR-1b uses its primitive
`crate::stack_guard::ensure_stack` (NEW: PR-1), `WALKER_RESERVE`, the test-only seams (PR-1 §4.5), the cell file
`tests_stack_guard.rs`, the `on_thread` helper and the release CI leg (PR-1 §8).

**Why a PR of its own (D-K).** (1) A different crate: the constant and the `Config` call live in
`elidex-wasm-runtime`, while PR-1's Rust changes are all in `elidex-js`. (2) A different spec: WebAssembly JS API
(PR-1 cites ECMA-262 / HTML / IndexedDB). (3) wasmtime's per-store limit is an invariant of its
own (§2 W-2), unrelated to PR-1's choke points and walkers. With the wasm rows PR-1's map was
K=4 / M=20 (SPLIT-RECOMMENDED by both thresholds, crossed by these rows alone).

**Base-case reason (single PR).** A narrowly scoped per-PR slice under the approved umbrella that
passes its own plan-review (CLAUDE.md edge-dense rule (c)).

## §1. Scope

1. CP-3a / CP-3b: `ensure_stack(WALKER_RESERVE + MAX_WASM_STACK)` at the two JS→wasm entries (§5).
2. `elidex_wasm_runtime::MAX_WASM_STACK` (NEW, `pub const` = `512 * 1024`) and
   `config.max_wasm_stack(MAX_WASM_STACK)` set explicitly (§4).
3. The wasm cells (§6), mutants (§7) and the `wasmcall` hot-path workload (§8).

Not in PR-1b: everything PR-1 delivers; wasm → JS imports (`#11-wasm-user-import-host-fn-builder`,
which inherits the obligation of §10 L-2).

## §2. Coupled invariants

The umbrella's program invariants I-1 (coverage), I-2 (fail-safe bound), I-3 (state integrity),
I-4 (semantic caps unchanged) and I-5 (hot-path cost) apply at CP-3 as at PR-1's choke points.
**I-3 at CP-3**: the probe sits before the lazy `vm_wasm_runtime()` initialisation, `Store::new` and
the start function (CP-3b), and before the `HostState` bind and the `set_fuel` reset (CP-3a). The
one VM field written before CP-3a, `call_dispatch`'s `gc_enabled = false` in the
`WasmExportedFunction` arm (`interpreter.rs:501` at `47b6c934`; re-derive after the `interpreter.rs`
prereq split), is restored by that arm on the `Err` return as on every other return.

PR-local invariants:

- **W-1 One constant, two readers.** The CP-3 ask and wasmtime's `max_wasm_stack` are both
  `MAX_WASM_STACK`; neither is written as a literal anywhere else.
- **W-2 Per-store limit.** wasmtime measures `max_wasm_stack` from the SP at a store's outermost
  entry; a nested entry into the same store keeps the limit already set, and an entry into another
  store (each `WasmInstance` has its own, `runtime.rs:144`) gets a fresh full limit. wasmtime never
  consults the thread's remaining stack, so across entries only CP-3 bounds the sum.
- **A-W No wasm → JS re-entry at this base** (assumption): wasm can call only the `elidex` host
  functions (`host/funcs.rs`), which hold no VM handle; a non-empty `ImportObject` is rejected
  (`runtime.rs:130`). Breaking it is `#11-wasm-user-import-host-fn-builder`'s change (§10 L-2).

Pairs of W-1, W-2 and A-W with each other and with I-1…I-5 (18):

| Pair | Intersection |
|---|---|
| W-1 × I-1 | Coverage at CP-3 needs the ask to cover wasm's own maximum plus the unprobed excess (entry, trap unwinding, host functions); that holds only while the ask and the `Config` read one constant. Pinned by M-CP3c and M-WS (§7). |
| W-1 × I-2 | The ask is checked against the same per-thread bound as every other probe; under the fallback bound it rejects earlier (fail-early direction). |
| W-1 × I-3 | The constant is static; it adds no state. |
| W-1 × I-4 | 512 KiB is wasmtime's default, so wasm's own recursion depth and its trap (`RuntimeError`) at the top of the thread are unchanged; cell `wasm_start_recursion_traps_at_top`. |
| W-1 × I-5 | The ask is a constant sum; the probe is the same TLS read + SP read + compare. |
| W-2 × I-1 | Nested entries into different stores each get 512 KiB; only a CP-3 at every JS→wasm entry bounds the total. Not constructible today (A-W); required when imports land (§10 L-2). |
| W-2 × I-2 | wasmtime's limit is SP-relative from the entry and independent of the thread bound; the thread bound is only the guard's, so its accuracy matters at CP-3 as anywhere. |
| W-2 × I-3 | CP-3 runs before the store's outermost entry, so on `Err` wasmtime has set no limit and bound no `HostState`. |
| W-2 × I-4 | The per-store trap is wasm's own answer; CP-3 must not pre-empt it where the thread has room (top of thread → `RuntimeError`, not `RangeError`). |
| W-2 × I-5 | wasmtime's own stack check is unchanged; CP-3 adds one probe per entry, not per wasm call frame. |
| A-W × I-1 | With no wasm → JS path, CP-1 is on no wasm path and CP-3 alone covers the wasm entries. |
| A-W × I-2 | Independent: A-W is a property of the import set, not of the bound. |
| A-W × I-3 | Host functions reached from wasm hold no VM handle, so a trap or `Err` inside wasm leaves no VM state to unwind. |
| A-W × I-4 | No cap involved. |
| A-W × I-5 | No cost. |
| W-1 × W-2 | The crate builds one `Config` (`git grep -n 'Config::new' -- crates/script/elidex-wasm-runtime/src` → `runtime.rs:76`), so every store gets `MAX_WASM_STACK`, and the ask matches the limit of whichever store an entry starts. |
| W-1 × A-W | When A-W is broken, the ask stays per entry (each JS→wasm entry asks `WALKER_RESERVE + MAX_WASM_STACK` again), which is what W-2 needs. |
| W-2 × A-W | The two together are why §10 L-2 is required, not optional: with imports, JS→wasm→JS→wasm recursion through different instances is bounded only by CP-1 + CP-3 at each step. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WebAssembly JS API §5.6 Exported Functions | *call an Exported Function* step 8 `func_invoke` (CP-3a sits after step 7's `ToWebAssemblyValue` coercions, before the store is bound); step 10's exception is elidex's resource limit, not a trap | exported function call | `call_wasm_exported_function` (CP-3a) | ✓ | yes |
| WebAssembly JS API §5 The WebAssembly Namespace | *instantiate the core of a WebAssembly module* step 2 `module_instantiate` (runs the start function); step 3's third bullet ("another error type if appropriate") is where the `RangeError` lands | sync `Instance` ctor / async `instantiate` (rejection) | `instantiate_module` (CP-3b) | ✓ | yes |

**Breadth**: K=1 spec (WebAssembly JS API), M=2 entries. `preflight.py` does not map the
`WebAssembly JS API` label, so neither row is webref-verified by it; the anchors were looked up by
hand (carried over from PR-1's R1 revision): `webref dfn wasm-js-api-2 'call an Exported Function'`
→ §5.6 `#call-an-exported-function`, `webref dfn wasm-js-api-2 'instantiate the core of a
WebAssembly module'` → §5 `#instantiate-the-core-of-a-webassembly-module`. WebAssembly JS API §8
Implementation-defined Limits defines no stack limit ("an implementation may run out of resources
for valid modules below these limits"), so `MAX_WASM_STACK` is elidex's own resource limit
(PM 424).

### §3.1 What each new surface's docstring cites

| Surface (NEW) | Cites |
|---|---|
| `elidex_wasm_runtime::MAX_WASM_STACK` | wasmtime's `Config::max_wasm_stack` contract (wasm's own stack use, not the thread's); the CP-3 probe that asks `WALKER_RESERVE + MAX_WASM_STACK`; the measurement that sizes it (§4); no spec section (WebAssembly JS API §8 defines none for stack) |
| CP-3a / CP-3b site comments | WebAssembly JS API §5.6 *call an Exported Function* step 8 / §5 *instantiate the core of a WebAssembly module* step 3 (where the resource-limit `RangeError` lands), and I-3's placement rule (§2) |

## §4. The wasm entry (decision D-W)

wasm runs synchronously on the VM thread: `elidex-wasm-runtime` calls
wasmtime on the caller's stack in exactly two places reachable from script, `WasmFunc::call`
(`handle.rs:220`, an exported-function call) and `WasmRuntime::instantiate` (`runtime.rs:165`,
`Linker::instantiate`, which runs the module's start function); no async, no fiber (re-derive:
`git grep -nE '\.call\(|\.instantiate\(' -- crates/script/elidex-wasm-runtime/src` → those two
plus `host/funcs.rs:62`, the `__alloc` call a host function makes from inside an activation that
is already running). wasmtime bounds wasm's own stack use with `max_wasm_stack`, measured from
the SP at the store's outermost entry (`wasmtime` 50.0.1 `runtime/func.rs:1535-1583`: a nested
entry into the same store keeps the limit already set), and its documentation (`config.rs`,
`max_wasm_stack`) states that the thread is not checked: with less than that remaining the
process aborts. elidex leaves it at the default today (`runtime.rs:76-77`: `Config::new()` then only `consume_fuel`;
the default is `512 * 1024`, `config.rs:295`).
**Measured** (`H/scripts/wasm_entry.sh`, `H/logs/wasm-entry.log`; prototype guard on, 256 KiB,
2 MiB thread; a module whose start function recurses without end, entered through
`new WebAssembly.Instance` at callback depth D): at the top both profiles give a catchable
`RuntimeError` (wasmtime's stack-overflow trap); debug D = 24 / 26 (lowest remaining at a probe
739 / 630 KiB) → `RuntimeError`, D = 28 / 30 / 32 → **process abort**, D = 33 (the CP-1 floor) →
`RangeError` from CP-1; release D = 200 / 220 (731 / 595 KiB) → `RuntimeError`, D = 240 / 250 /
260 → **abort**, D = 269 (floor) → `RangeError`. So without CP-3 (PR-1 alone, or PR-1 as planned before D-W), a wasm entry
in the band between the CP-1 floor and ≈600 KiB remaining aborts the renderer: I-1 did not hold.
**Decision (D-W)**: the choke point **CP-3** at both JS→wasm entries (§5), asking
`ensure_stack(WALKER_RESERVE + MAX_WASM_STACK)`, and sets `max_wasm_stack` explicitly from that
constant. **`MAX_WASM_STACK`** (NEW) = `512 * 1024`, `pub const` in `elidex-wasm-runtime`
(`lib.rs` re-export; `runtime.rs:76-77`, the crate's one `Config::new()`, gains `config.max_wasm_stack(MAX_WASM_STACK)`), imported by
`elidex-js` under `feature = "engine"` (the dependency direction that exists; `elidex-js`'s
`pub(crate)` `crate::stack_guard` (NEW: PR-1) is not exported, and does not need to be). One constant, two
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
constructible at this base, and CP-1 is not on any wasm path yet. When
`#11-wasm-user-import-host-fn-builder` lands JS imports, the import trampoline must enter JS
through `with_call_mode` (CP-1), and each JS→wasm entry it leads to passes CP-3 again; this is
required, not optional, because each `WasmInstance` has its own `Store` (`runtime.rs:144`) and
wasmtime sets a fresh `max_wasm_stack` limit at each store's outermost entry, so a nested entry
into another instance gets a full 512 KiB of its own (§10 L-2).

## §5. Call sites

| Id | Site | Placement | Why this covers it |
|---|---|---|---|
| CP-3a | `call_wasm_exported_function` (`crates/script/elidex-js/src/vm/host/wasm/exported_func.rs:268`), asking `WALKER_RESERVE + MAX_WASM_STACK` | after the argument coercions (*call an Exported Function* step 7, which may run user JS through CP-1) and the bound-session check, immediately before `host_data.with_session_and_dom(… func.call …)` (`:306`), i.e. before the `HostState` bind and `set_fuel` reset in `WasmFunc::call` | the only caller of `WasmFunc::call` in `elidex-js` (its one caller is `call_dispatch`'s `WasmExportedFunction` arm, `interpreter.rs:511`); every exported function, however obtained (`instance.exports`, a funcref read from a table), is an `ObjectKind::WasmExportedFunction` dispatched there |
| CP-3b | `instantiate_module` (`crates/script/elidex-js/src/vm/host/wasm/instance.rs:151`), same ask | after `coerce_import_object` (*Read the imports*, `:162`), before `ctx.vm.vm_wasm_runtime()` (`:176`, lazy runtime init) and `runtime.instantiate` (`:180`, new `Store` + start function) | the only caller of `WasmRuntime::instantiate` in `elidex-js`; both the sync `new WebAssembly.Instance` (`instance.rs:69`) and the async `WebAssembly.instantiate` (`namespace.rs:578`, which turns the `Err` into a rejection) go through it |

JS→wasm entries (re-derive: `git grep -nE 'func\.call\(|\.instantiate\(&' -- crates/script/elidex-js/src ':!**/tests/**'`): `exported_func.rs:307` and `instance.rs:180`, i.e. CP-3a and CP-3b; no other crate calls into `elidex-wasm-runtime` (`git grep -ln elidex_wasm_runtime -- crates ':!crates/script/elidex-js' ':!crates/script/elidex-wasm-runtime'` → nothing). Both entries are inside `elidex-js`, so `crate::stack_guard` reaches them; the only `elidex-wasm-runtime` change is the `MAX_WASM_STACK` constant and the `Config` call (§4). CP-3 is not a re-entry choke point (no nested `run()`): it guards a foreign stack consumer whose own limit is a fixed size, which is why its ask is that size plus the reserve.

**host/ touch table.** Both files are engine-bound per the Layering mandate: they marshal JS values
across the wasm boundary, and the change adds a stack check, no algorithm.

| File | Change | Engine-bound reason |
|---|---|---|
| `crates/script/elidex-js/src/vm/host/wasm/exported_func.rs` | CP-3a probe | a stack check at the JS→wasm boundary; the call itself stays in `elidex-wasm-runtime` |
| `crates/script/elidex-js/src/vm/host/wasm/instance.rs` | CP-3b probe | same, for instantiation |

Outside `elidex-js`: `crates/script/elidex-wasm-runtime/src/lib.rs` (re-export of
`MAX_WASM_STACK`) and `crates/script/elidex-wasm-runtime/src/runtime.rs` (the constant and the
`Config` call). All four touched files are under 1000 lines at `47b6c934` (`exported_func.rs` 490,
`instance.rs` 242, `lib.rs` 39, `runtime.rs` 806), so no touch-time split applies.

## §6. Cells

Added to PR-1's `crates/script/elidex-js/src/vm/tests/tests_stack_guard.rs` (NEW: PR-1), same `on_thread`
rule, same debug and release CI legs (PR-1 §8). Locally the release cells run through PR-1's mise
task `test-stack-guard-release` (NEW: PR-1), once in PR-1b's pre-push procedure, as in PR-1 §8
(PM 490); `mise run ci` has no release leg.

| Cell | Body | Expect |
|---|---|---|
| `wasm_start_recursion_traps_at_top` | at the thread's top, `new WebAssembly.Instance(m)` where `m`'s start function recurses without end (`(module (func $f call $f) (start $f))`) | `RuntimeError` (wasmtime's own limit, which `MAX_WASM_STACK` sets), no abort; green today too (`H/logs/wasm-entry.log` `top`). Pins that wasm's limit is below the thread's room at the top |
| `wasm_instantiate_deep_js_then_wasm` | recurse through `[1].forEach` to each depth D from the top to the CP-1 floor; at D, the same `new WebAssembly.Instance(m)` (CP-3b), and the async `WebAssembly.instantiate(m)` at the same depths (rejection) | at every D: `RuntimeError` (enough room) or `RangeError` / `STACK_EXHAUSTED_MSG` (CP-3b), never an abort. Red on `47b6c934`: aborts for D in the band between the floor and ≈600 KiB remaining (debug 28–32, release 240–260 on the prototype, `H/logs/wasm-entry.log`) |
| `wasm_call_deep_js_then_wasm` | bound session (the `tests_wasm.rs` harness: `setup_vm_for_binding` + `test_helpers::bind_vm`); an instance exporting `f` that recurses without end; recurse through `[1].forEach` to each depth D up to the floor and call `instance.exports.f()` (CP-3a) | same as above, at every D. Red on `47b6c934` in the same band (same mechanism as CP-3b, whose band is the measured one; the exported-function band is measured when the cell is written) |
| (no JS→wasm→JS→wasm cell) | — | not constructible at this base: wasm cannot call JS (§4). It is the obligation PR-1b hands `#11-wasm-user-import-host-fn-builder` (§10 L-2), whose acceptance gains that cell |

## §7. Mutants

Run on the final head only, debug and release. None has been run yet.

| Mutant | Removed | Dies |
|---|---|---|
| M-CP3a | CP-3a | `wasm_call_deep_js_then_wasm` (abort) |
| M-CP3b | CP-3b | `wasm_instantiate_deep_js_then_wasm` (abort) |
| M-CP3c | CP-3's ask reduced to `WALKER_RESERVE` (constant dropped from the ask) | `wasm_instantiate_deep_js_then_wasm` and `wasm_call_deep_js_then_wasm` (abort in the band) |
| M-WS | `config.max_wasm_stack(MAX_WASM_STACK)` replaced by a larger value (e.g. 4 MiB), i.e. wasmtime's limit no longer the constant the probe asks for | `wasm_start_recursion_traps_at_top` (abort at the top of 2 MiB) and the two deep cells |

## §8. Hot-path probe cost

Acceptance (same method, host condition and band as PR-1 §10): workload `wasmcall` — an exported
wasm function of an empty body called 10⁶ times from JS (CP-3a), production probe vs a build with
`ensure_stack` stubbed to `Ok(())`, plus PR-1's `plain` control row on the same run. CP-3b, once per
instantiation, is not on a hot path and is not measured. Recorded in the PR body.

## §9. Measurements

- JS→wasm entry near the floor: umbrella §8 M-11, `H/scripts/wasm_entry.sh`, log
  `H/logs/wasm-entry.log` (numbers in §4). Guard off, the same entry aborts from debug D = 30 /
  release D = 250 (versus `RuntimeError` at 25 / 200).
- wasm and stack switching: umbrella §8 M-10 (the fiber crate is not compiled; PR-1 §4.4 bans it).

## §10. Landing obligations

- **L-1. Close `#11-vm-call-depth-guard`** (PM / PMO, live ledger): with PR-1's and PR-1b's PR
  numbers and squash shas, and a pointer to PR-1's tracked cut check (PR-1 §5.3) as what keeps the
  closure true.
- **L-2. `#11-wasm-user-import-host-fn-builder` inherits a guard obligation** (live entry, PM /
  PMO): "JS imports called from wasm enter JS through `with_call_mode` (CP-1), and every JS→wasm
  entry they lead to passes CP-3; acceptance gains a JS→wasm→JS→wasm recursion cell that ends in a
  `RangeError` on a 2 MiB thread in both profiles (each instance has its own store, so wasmtime's
  limit restarts at every nested entry; only CP-3 bounds the sum)".
- **L-3. Breadth record**: `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo at
  the PR's head, rc and K / M as printed (the label is unmapped; hand count K=1 / M=2).
