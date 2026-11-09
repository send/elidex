# VM stack-safety guard — PR-1b plan (JS→wasm entries, `MAX_WASM_STACK`, the wasm overflow class)

Status: **plan (pre-implementation)**, split out of [PR-1](2026-11-vm-stack-safety-guard-pr1.md) by
the driver decision D-K (PM 483, umbrella §0.1); revised for PM 502 (S-4: the wasm stack overflow is a
`RangeError`; S-5: the store layout; the shallow-depth regression) and for the P0 program (placement and
conversions are P0a-1's). `/elidex-plan-review` required before code (edge-dense program, umbrella §2).
Parent: [umbrella](2026-11-vm-stack-safety-guard.md). **Closes `#11-vm-call-depth-guard`** (the slot
closes at this PR's landing, not PR-1's, PM 483). Base: `47b6c934` (anchors below are at that commit;
re-derive each at PR-1b's own base with the command beside it). Rulings of umbrella §0.1 are ratified and
not re-opened here; D-W (CP-3 + `MAX_WASM_STACK`) is carried over from PR-1 and corrected by PM 502 S-4
(§4.2). Provenance root `H` is defined in the umbrella header.
Preconditions: **PR-1 has landed** (edge PR-1 → PR-1b, umbrella §4): PR-1b uses its primitive
`crate::stack_guard::ensure_stack` (NEW: PR-1), `WALKER_RESERVE`, the test-only seams (PR-1 §4.5), the cell
file `tests_stack_guard.rs`, the `on_thread` helper and the release CI leg (PR-1 §8). The CP-3a / CP-3b
check sites, their placement and the CP-3b conversion are P0a-1's (P0a-1 §4.3); PR-1b fills them.

**Why a PR of its own (D-K).** (1) A different crate: the constant, the `Config` call and the overflow
classification live in `elidex-wasm-runtime`, while PR-1's Rust changes are all in `elidex-js`. (2) A
different spec: WebAssembly JS API. (3) wasmtime's per-store limit is an invariant of its own (§2 W-2).

**Base-case reason (single PR).** A narrowly scoped per-PR slice under the approved umbrella that
passes its own plan-review (CLAUDE.md edge-dense rule (c)).

## §1. Scope

1. CP-3a / CP-3b: `ensure_stack(WALKER_RESERVE + MAX_WASM_STACK)` inside P0a-1's
   `check_entry(EntryKind::WasmCall | EntryKind::WasmInstantiate)` (§5).
2. `elidex_wasm_runtime::MAX_WASM_STACK` (NEW, `pub const` = `512 * 1024`) and
   `config.max_wasm_stack(MAX_WASM_STACK)` set explicitly (§4.1).
3. The wasm stack overflow surfaces as the JS class, `RangeError` (WebAssembly JS API §7.1; §4.2).
4. The wasm cells (§6), mutants (§7), the `wasmcall` hot-path workload (§8), the wasm-entry trip-wire (§5)
   and the move of the wasm test harness out of `tests_wasm.rs` (§6).

Not in PR-1b: everything PR-1 delivers; wasm → JS imports (`#11-wasm-user-import-host-fn-builder`, §10
L-2); the structural remedy of the entry band (§4.3, slot request §10 L-4).

## §2. Coupled invariants

The umbrella's program invariants I-1…I-5 apply at CP-3 as at PR-1's choke points, and PR-1's I-6 (one
error shape per channel) applies to CP-3 as a runtime-channel site. **I-3 at CP-3** is P0a-1 §4.3 (the check
sits before the lazy `vm_wasm_runtime()` initialisation, `Store::new` and the start function for CP-3b, and
before the `HostState` bind and the `set_fuel` reset for CP-3a; placement mutants M-P-CP3a / M-P-CP3b are
P0a-1 §7). The one VM field written before CP-3a, `call_dispatch`'s `gc_enabled = false` in the
`WasmExportedFunction` arm (`interpreter_call.rs:240`; the prereq split is discharged by #615), is restored by
that arm on the `Err` return as on every other return.

CP-3a / CP-3b call `resource_limit::check_entry` (P0a-1 §4.2, the one check site) and propagate its error
(CP-3b through the named conversion of P0a-1 §4.3); there is no wrapper and no per-site mark (option β,
PM 590). A failure of the argument coercions before CP-3a (nested CP-1 entries, step 7) reaches the
enclosing CP-1 entry's caller as `ResourceLimit` and is reported or thrown there like any other probe
failure (P0 umbrella §4 C-4); CP-3a's position is unchanged by that.

PR-local invariants:

- **W-1 One constant, two readers.** The CP-3 ask and wasmtime's `max_wasm_stack` are both
  `MAX_WASM_STACK`; neither is written as a literal anywhere else.
- **W-2 Store layout and the wasm limit.** wasmtime measures `max_wasm_stack` from the SP at a store's
  outermost entry; a nested entry into the same store keeps the limit already set (`wasmtime` 50.0.1
  `runtime/func.rs:1535-1583`), and wasmtime never consults the thread's remaining stack. The spec model is
  one store per agent (WebAssembly JS API §4.1, "each agent has an associated store"). elidex today gives
  each `WasmInstance` its own `Store` (`runtime.rs:144`), an **interim deviation** the code itself records
  (`runtime.rs:120-128`, `:173-183`) until `#11-wasm-user-import-host-fn-builder` lands the shared-store
  wiring. Under the spec model a JS→wasm→JS→wasm chain keeps the outermost entry's limit, so wasmtime bounds
  every nested wasm frame within `MAX_WASM_STACK` of the outermost entry, and the JS frames between are
  bounded by CP-1; under the interim layout each store restarts its limit, so only CP-3 at each entry
  bounds the sum. In both, the outermost CP-3 is what makes the first entry safe.
- **A-W No wasm → JS re-entry at this base** (assumption): wasm can call only the `elidex` host functions
  (`host/funcs.rs`), which hold no VM handle; a non-empty `ImportObject` is rejected (`runtime.rs:130`).
  Breaking it is `#11-wasm-user-import-host-fn-builder`'s change (§10 L-2).

Pairs of W-1, W-2 and A-W with each other and with I-1…I-6 (21):

| Pair | Intersection |
|---|---|
| W-1 × I-1 | Coverage at CP-3 needs the ask to cover wasm's own maximum plus the unprobed excess (entry, trap unwinding, host functions); that holds only while the ask and the `Config` read one constant. Pinned by M-CP3c and M-WS (§7). |
| W-1 × I-2 | The ask is checked against the same per-thread bound as every other probe; under the fallback bound it rejects earlier (fail-early direction). |
| W-1 × I-3 | The constant is static; it adds no state. |
| W-1 × I-4 | 512 KiB is wasmtime's default, so wasm's own recursion depth at the top of the thread is unchanged; its overflow now surfaces as `RangeError` (§4.2); cell `wasm_start_recursion_overflow_at_top`. |
| W-1 × I-5 | The ask is a constant sum; the probe is the same TLS read + SP read + compare. |
| W-1 × I-6 | Both of CP-3's possible answers have I-6's runtime shape: CP-3's own failure (`VmError::resource_limit()`, C-1's kind, before the store is entered) and wasm's overflow trap (`RangeError` / `STACK_EXHAUSTED_MSG`, after wasm ran; §4.2). Under β no consumer tells the two apart: each is a throw completion of the step (P0 umbrella §4 C-4). |
| W-2 × I-1 | Under the interim layout nested entries into different stores each get 512 KiB; only a CP-3 at every JS→wasm entry bounds the total. Not constructible today (A-W); stated for the imports slot (§10 L-2). |
| W-2 × I-2 | wasmtime's limit is SP-relative from the entry and independent of the thread bound; the thread bound is only the guard's. |
| W-2 × I-3 | CP-3 runs before the store's outermost entry, so on `Err` wasmtime has set no limit and bound no `HostState`. |
| W-2 × I-4 | The per-store trap is wasm's own answer; CP-3 must not pre-empt it where the thread has room. At the top both surface as `RangeError`; the trap is the one that follows wasm execution (I-6 pair above), which changes which bound fired, not how the failure is consumed. |
| W-2 × I-5 | wasmtime's own stack check is unchanged; CP-3 adds one probe per entry, not per wasm call frame. |
| W-2 × I-6 | Under either layout, an overflow anywhere on a JS→wasm→JS→wasm chain is a `RangeError` (wasm's trap by §7.1, CP-1 or CP-3 by I-6), so the layout changes which bound fires, not the class. |
| A-W × I-1 | With no wasm → JS path, CP-1 is on no wasm path and CP-3 alone covers the wasm entries. |
| A-W × I-2 | Independent: A-W is a property of the import set, not of the bound. |
| A-W × I-3 | Host functions reached from wasm hold no VM handle, so a trap or `Err` inside wasm leaves no VM state to unwind. |
| A-W × I-4 | No cap involved. |
| A-W × I-5 | No cost. |
| A-W × I-6 | With no JS inside wasm, a wasm-side overflow has one source (wasmtime's trap) and one class. |
| W-1 × W-2 | The crate builds one `Config` (`git grep -n 'Config::new' -- crates/script/elidex-wasm-runtime/src` → `runtime.rs:76`), so every store gets `MAX_WASM_STACK`, and the ask matches the limit of whichever store an entry starts. |
| W-1 × A-W | When A-W is broken, the ask stays per entry; under the spec layout a nested CP-3 asks more than the nested entry needs (its store is already entered), which is the fail-early direction. |
| W-2 × A-W | The two together are why §10 L-2 records the store-layout dependence: with imports, which bound stops a JS→wasm→JS→wasm recursion depends on the layout. |

**State ownership (ECS-native check).** PR-1b writes no entity state. wasmtime's per-store limit is
wasmtime's; the agent store of the spec model is a session-level shared resource (side-store exception (b),
CLAUDE.md), not per-instance state; `MAX_WASM_STACK` is a constant.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WebAssembly JS API §5.6 Exported Functions | *call an Exported Function*: CP-3a runs after step 7's `ToWebAssemblyValue` coercions and before step 8's `func_invoke`, i.e. before the store is bound; step 10 maps a `func_invoke` error | exported function call | `call_wasm_exported_function` (CP-3a) | ✓ | yes |
| WebAssembly JS API §5 The WebAssembly Namespace | *instantiate the core of a WebAssembly module*: CP-3b runs before step 2's `module_instantiate`; step 3 maps a start-function error ("another error type if appropriate") | sync `Instance` ctor / async `instantiate` (rejection) | `instantiate_module` (CP-3b) | ✓ | yes |
| WebAssembly JS API §7.1 Stack Overflow | a stack overflow in WebAssembly code throws the same class as one in JavaScript | wasmtime's `Trap::StackOverflow` | `wasm_error_from_wasmtime`, `wasm_error_to_js_value` (§4.2) | ✓ | yes |

**Breadth**: K=1 spec (WebAssembly JS API), M=3 entries. `preflight.py` does not map the `WebAssembly JS
API` label, so no row is webref-verified by it; the anchors were looked up by hand: `webref dfn wasm-js-api-2
'call an Exported Function'` → §5.6 `#call-an-exported-function`; `webref dfn wasm-js-api-2 'instantiate
the core of a WebAssembly module'` → §5 `#instantiate-the-core-of-a-webassembly-module`; `webref body
wasm-js-api-2 errors` → §7 Error Condition Mappings to JavaScript, §7.1 Stack Overflow (2026-11-08).
Neither CP-3 throws "at" step 10 or step 3: CP-3 runs before `func_invoke` / `module_instantiate`, so its
`RangeError` is elidex's resource limit (PM 424) whose class §7.1 fixes, not a step's output. WebAssembly JS
API §8 Implementation-defined Limits defines no stack limit, so `MAX_WASM_STACK` is elidex's own resource
limit.

**Stated deviation (pre-existing, A4 R2).** *Asynchronously instantiate a WebAssembly module* step 6.1
queues a task that runs *instantiate the core*, so the start function runs at the bottom of a fresh task;
elidex calls `instantiate_module` synchronously inside `WebAssembly.instantiate` (`namespace.rs:576-583`), so
the caller's JS depth reaches the async path's CP-3b. The async half of the deep cell (§6) pins today's
behaviour, not the spec's; home: slot request §10 L-5.

### §3.1 What each new surface's docstring cites

| Surface (NEW) | Cites |
|---|---|
| `elidex_wasm_runtime::MAX_WASM_STACK` | wasmtime's `Config::max_wasm_stack` contract (wasm's own stack use, not the thread's); the CP-3 ask; the measurement that sizes it (§4.1); no spec section (§8 defines none for stack) |
| `WasmErrorKind::StackOverflow` (NEW) | WebAssembly JS API §7.1 (same class as a JS stack overflow; elidex's is `RangeError`, PM 421 (a)) |
| CP-3a / CP-3b site comments | the resource-limit status of CP-3's `RangeError` (§3); P0a-1 §4.3's placement rule |

## §4. The wasm entry

### §4.1 D-W: CP-3 and `MAX_WASM_STACK`

wasm runs synchronously on the VM thread: `elidex-wasm-runtime` calls wasmtime on the caller's stack in
exactly two places reachable from script, `WasmFunc::call` (`handle.rs:220`, an exported-function call) and
`WasmRuntime::instantiate` (`runtime.rs:165`, `Linker::instantiate`, which runs the module's start function);
no async, no fiber (re-derive: `git grep -nE '\.call\(|\.instantiate\(' -- crates/script/elidex-wasm-runtime/src`
→ those two plus `host/funcs.rs:62`, the `__alloc` call a host function makes from inside an activation that
is already running). wasmtime bounds wasm's own stack use with `max_wasm_stack` (W-2), and its
documentation (`config.rs`, `max_wasm_stack`) states that the thread is not checked: with less than that
remaining the process aborts. elidex leaves it at the default today (`runtime.rs:76-77`; the default is
`512 * 1024`, `config.rs:295`).
**Measured** (`H/scripts/wasm_entry.sh`, `H/logs/wasm-entry.log`; prototype guard on, 256 KiB, 2 MiB thread;
a module whose start function recurses without end, entered through `new WebAssembly.Instance` at callback
depth D): at the top both profiles give wasmtime's stack-overflow trap (today a `RuntimeError`); debug
D = 24 / 26 (lowest remaining at a probe 739 / 630 KiB) → trap, D = 28 / 30 / 32 → **process abort**, D = 33
(the CP-1 floor) → `RangeError` from CP-1; release D = 200 / 220 (731 / 595 KiB) → trap, D = 240 / 250 / 260
→ **abort**, D = 269 (floor) → `RangeError`. So without CP-3 a wasm entry in the band between the CP-1 floor
and ≈600 KiB remaining aborts the renderer: I-1 did not hold.
**Decision (D-W)**: CP-3 at both JS→wasm entries asks `ensure_stack(WALKER_RESERVE + MAX_WASM_STACK)`, and
`max_wasm_stack` is set explicitly from that constant. `MAX_WASM_STACK` (NEW) = `512 * 1024`, `pub const`
in `elidex-wasm-runtime` (`lib.rs` re-export; `runtime.rs:76-77`, the crate's one `Config::new()`, gains
`config.max_wasm_stack(MAX_WASM_STACK)`), imported by `elidex-js` under `feature = "engine"`. **Why 512 KiB**:
wasmtime's default, so wasm's own recursion depth is what it is today; an entry that runs to that limit needs
more than 512 KiB and at most ≈595 KiB (release) / ≈630 KiB (debug) of remaining stack, inside
`WALKER_RESERVE + 512 KiB` = 768 KiB. The excess over 512 KiB (entry, trap unwinding, and the host functions
reached from wasm) is what `WALKER_RESERVE` is for; the host functions include `get_text_content` and
`query_selector`, which run the out-of-scope recursive DOM walkers, so the wasm path leans on the reserve
exactly as the JS floor does (PR-1 §12 L-6; §10 L-3).

### §4.2 The wasm stack overflow is a `RangeError` (PM 502 S-4)

WebAssembly JS API §7.1: a stack overflow in WebAssembly code throws the same class as one in JavaScript.
elidex's is `RangeError` "Maximum call stack size exceeded" (PM 421 (a)), so the trap must not surface as
`WebAssembly.RuntimeError` as it does today.

- **Trap code**: `wasmtime::Trap::StackOverflow` ("call stack exhausted"), the trap wasmtime raises when a
  wasm prologue finds SP below the store's limit.
- **Classification site** (`elidex-wasm-runtime`): `engine_conv::wasm_error_from_wasmtime`
  (`engine_conv.rs:336-345`) maps an error whose chain holds `Trap::StackOverflow` to
  `WasmErrorKind::StackOverflow` (NEW variant of the `#[non_exhaustive]` enum, `error.rs:32`; the
  `Runtime` docstring stops claiming stack overflow). Every other trap stays `Runtime`.
- **Materialisation site** (`elidex-js`): `wasm_error_to_js_value` (`vm/host/wasm/errors.rs:179`) gains a
  `StackOverflow` arm that builds a `RangeError` object with `STACK_EXHAUSTED_MSG` (NEW: P0a-1), the object
  `vm_error_to_thrown` builds for C-1's kind. It is a thrown value, **not** `VmErrorKind::ResourceLimit`:
  the overflow is wasmtime's own fixed per-call budget (`MAX_WASM_STACK`), the same for the same module and
  input at any host depth, so it is a cap in P0's sense (P0 umbrella D-P0-6), not a probe failure (C-1).
- **Other traps** (`unreachable`, out-of-bounds, fuel exhaustion, …) stay `WebAssembly.RuntimeError` (§5.6
  *call an Exported Function* step 10, *instantiate the core* step 3; §7 maps only stack overflow and
  out-of-memory). §7.2 Out of Memory is not changed by this PR.
- Re-derive the sites at PR-1b's base: `git grep -n 'WasmErrorKind::' -- crates` (every match on the enum;
  the crate's own `Display` needs the arm, the `#[non_exhaustive]` matches in `elidex-js` need the explicit
  arm before their wildcard).

### §4.3 The shallow-depth regression (A3 R2; PM 502: fix in PR-1b first)

**The regression.** Every JS→wasm entry needs 768 KiB free, including a call into a trivial, non-recursive
export, which today runs anywhere above the abort point. On a 2 MiB thread such a call becomes a
`RangeError` once the remaining stack at CP-3 is below 768 KiB: from callback depth ≈23 (debug) / ≈195
(release) instead of the CP-1 floor's 33 / 269 (interpolated from the lowest remaining per depth in
`H/logs/wasm-entry.log`; the cell of §6 measures the edges).

**Can PR-1b fix it?** The fix would be to ask only for what wasm can use at this entry, i.e. let wasmtime's
own limit be `min(MAX_WASM_STACK, remaining − WALKER_RESERVE)`. wasmtime 50.0.1 does not allow it:
`max_wasm_stack` is an `Engine`-wide `Config` value (`config.rs:857`), applied SP-relative at the store's
outermost entry (`func.rs:1535-1583`), and no public API sets a per-store or per-entry stack limit (the
public stack knobs are `max_wasm_stack` and the `async_stack_*` family). At the entry a trivial export and an
unboundedly recursive one are indistinguishable. The other levers each move the regression elsewhere:

- an ask below `MAX_WASM_STACK` + the measured excess admits an entry that can overflow the thread before
  wasmtime's trap fires: an abort, breaking I-1;
- shrinking only the excess part (`WALKER_RESERVE` → the ≈83 / ≈118 KiB measured with a host-function-free
  module) gains ≈2 callback levels in debug and ≈25 in release (from the per-level slopes of the same log) and takes away the DOM walkers' headroom on
  the wasm path (§4.1), which the L3 slot relies on (PR-1 §12 L-6);
- a smaller `MAX_WASM_STACK` lowers wasm's own recursion depth at the top of the thread, a regression of
  other legitimate code;
- running wasm on its own stack (wasmtime's async mode, a fiber) removes the dependence on the caller's
  depth, but compiles the fiber crate that PR-1 §4.4 bans, because a stack switch invalidates the guard's
  cached bound; it is a guard redesign, not a PR-1b change.

**Outcome: not fixable inside PR-1b; the band is stated and pinned.** Remaining band: a JS→wasm entry with
less than `WALKER_RESERVE + MAX_WASM_STACK` (768 KiB) and more than the CP-1 floor (`WALKER_RESERVE`) of
remaining stack gets `RangeError` even when wasm would use little stack. Reason: the guard cannot bound
wasm's stack use below `MAX_WASM_STACK` at an entry, so the safety bound must reserve all of it. Direction:
fail-early (a catchable `RangeError`, never an abort). Pinned by `wasm_trivial_call_band` (§6). The
structural remedy is the registered slot `#11-wasm-entry-depth-band` (§10 L-4).

## §5. Call sites

| Id | Site | Placement (P0a-1 §4.3) | Why this covers it |
|---|---|---|---|
| CP-3a | `call_wasm_exported_function` (`crates/script/elidex-js/src/vm/host/wasm/exported_func.rs:268`) | P0a-1's `check_entry(WasmCall)`, after the argument coercions (*call an Exported Function* step 7, which may run user JS through CP-1) and the bound-session check, immediately before `host_data.with_session_and_dom(… func.call …)` (`:306`) | the only caller of `WasmFunc::call` in `elidex-js` (its one caller is `call_dispatch`'s `WasmExportedFunction` arm, `interpreter_call.rs:248`); every exported function, however obtained, is an `ObjectKind::WasmExportedFunction` dispatched there |
| CP-3b | `instantiate_module` (`crates/script/elidex-js/src/vm/host/wasm/instance.rs:151`) | P0a-1's `check_entry(WasmInstantiate)`, before `ctx.vm.vm_wasm_runtime()` (`:176`) and `runtime.instantiate` (`:180`); its `Err` converts by P0a-1 §4.3's named conversion (`vm_error_to_thrown`, never `wasm_error_to_js_value`) | the only caller of `WasmRuntime::instantiate` in `elidex-js`; both the sync `new WebAssembly.Instance` (`instance.rs:69`) and the async `WebAssembly.instantiate` (`namespace.rs:578`, which turns the `Err` into a rejection) go through it |

PR-1b's change at each site is the ask inside `check_entry` for its kind; the site itself is P0a-1's.
JS→wasm entries (re-derive: `git grep -nE 'func\.call\(|\.instantiate\(&' -- crates/script/elidex-js/src ':!**/tests/**'`):
`exported_func.rs:307` and `instance.rs:180`; no other crate depends on `elidex-wasm-runtime`.

**Trip-wire (NEW, `.claude/tools/wasm-entry-trip-wire.sh`, registered in `scripts/trip-wires.sh`'s
`REQUIRED_WIRES`).** Property: (1) no `Cargo.toml` other than the workspace root's, `elidex-js`'s and the
runtime's own names `elidex-wasm-runtime`; (2) inside `crates/script/elidex-js/src` (tests excluded), the
two entry calls `WasmFunc::call` and `WasmRuntime::instantiate` appear only in `exported_func.rs` and
`instance.rs`, by the pattern above. Half (2) matches call spellings, so a renamed binding can evade it; the
P0 census does not cover the gap (it derives producers, not wasm callers). Controls in the wire's self-check:
a scratch `Cargo.toml` naming the crate, and a scratch file with `runtime.instantiate(&m, &i)`, each fail it.

**host/ touch table.** Both files marshal JS values across the wasm boundary; the change adds a stack ask,
the error arm and no algorithm.

| File | Change | Engine-bound reason |
|---|---|---|
| `crates/script/elidex-js/src/vm/host/wasm/exported_func.rs` | CP-3a ask (inside P0a-1's check) | a stack check at the JS→wasm boundary |
| `crates/script/elidex-js/src/vm/host/wasm/instance.rs` | CP-3b ask (same) | same, for instantiation |
| `crates/script/elidex-js/src/vm/host/wasm/errors.rs` | `StackOverflow` arm | maps a runtime error kind to a JS object |

Outside `elidex-js`: `elidex-wasm-runtime/src/lib.rs` (re-export), `runtime.rs` (constant, `Config` call),
`error.rs` (variant), `engine_conv.rs` (classification). All touched files are under 1000 lines at
`47b6c934` (`exported_func.rs` 490, `instance.rs` 242, `errors.rs` 223, `lib.rs` 39, `runtime.rs` 806,
`error.rs` 127, `engine_conv.rs` 478).

## §6. Cells

Added to PR-1's `crates/script/elidex-js/src/vm/tests/tests_stack_guard.rs` (NEW: PR-1), same `on_thread`
rule, same debug and release legs (PR-1 §8; locally PR-1's task `test-stack-guard-release`, run once in
PR-1b's pre-push procedure, PR-1 §8). The bound-session harness (`setup_vm_for_binding`, private to
`vm/tests/tests_wasm.rs`, 998 lines at `ce603c74`) moves into `vm/tests/wasm_harness.rs` (NEW) in a
standalone first commit, so neither file is duplicated nor grown.

| Cell | Body | Expect |
|---|---|---|
| `wasm_start_recursion_overflow_at_top` | at the thread's top, `new WebAssembly.Instance(m)` where `m`'s start function recurses without end (`(module (func $f call $f) (start $f))`) | `RangeError` with `STACK_EXHAUSTED_MSG`, not `WebAssembly.RuntimeError` (§4.2), no abort. Red on `47b6c934` (`RuntimeError`, `H/logs/wasm-entry.log` `top`) |
| `wasm_other_trap_stays_runtime_error` | an export whose body is `unreachable` | `WebAssembly.RuntimeError` (§4.2: only stack overflow changes class) |
| `wasm_instantiate_deep_js_then_wasm` | recurse through `[1].forEach` to each depth D from the top to the CP-1 floor; at D, the same `new WebAssembly.Instance(m)` (CP-3b), and the async `WebAssembly.instantiate(m)` at the same depths (rejection; the synchronous-instantiate deviation of §3) | at every D: `RangeError` / `STACK_EXHAUSTED_MSG` (wasm's trap or CP-3b), never an abort, never `RuntimeError` (so a probe mapped to `RuntimeError` is red). Red on `47b6c934`: aborts for D in the band between the floor and ≈600 KiB remaining |
| `wasm_call_deep_js_then_wasm` | bound session (`wasm_harness.rs`); an instance exporting `f` that recurses without end; recurse through `[1].forEach` to each depth D up to the floor and call `instance.exports.f()` (CP-3a) | same as above, at every D |
| `wasm_trivial_call_band` | bound session; an export `g` returning `i32.const 1`; at each depth D up to the CP-1 floor call `g()`, recording the first D that throws | `g()` returns 1 at every D before that, `RangeError` at it and after, never an abort; that D is below the CP-1 floor depth (the stated band of §4.3 exists) and is printed with the floor depth. When the slot of §10 L-4 lands this cell flips to "returns 1 up to the floor" |
| (no JS→wasm→JS→wasm cell) | — | not constructible at this base: wasm cannot call JS (§4.1, A-W). §10 L-2 records the facts the imports slot needs |

## §7. Mutants

Run on the final head only, debug and release. None has been run yet. (CP-3 placement mutants are P0a-1 §7.)

| Mutant | Removed | Dies |
|---|---|---|
| M-CP3a | CP-3a's ask | `wasm_call_deep_js_then_wasm` (abort) |
| M-CP3b | CP-3b's ask | `wasm_instantiate_deep_js_then_wasm` (abort) |
| M-CP3c | CP-3's ask reduced to `WALKER_RESERVE` | the two deep cells (abort in the band); `wasm_trivial_call_band` (first throw moves to the floor) |
| M-WS | `config.max_wasm_stack(MAX_WASM_STACK)` replaced by a larger value (e.g. 4 MiB) | `wasm_start_recursion_overflow_at_top` (abort at the top of 2 MiB) and the two deep cells |
| M-SO | the `StackOverflow` classification removed (the trap is `Runtime` again) | `wasm_start_recursion_overflow_at_top`, the two deep cells (`RuntimeError`) |

## §8. Hot-path probe cost

Acceptance (same method, host condition and band as PR-1 §10): workload `wasmcall` — an exported wasm
function of an empty body called 10⁶ times from JS (CP-3a), production probe vs a build with
`ensure_stack` stubbed to `Ok(())`, plus PR-1's `plain` control row on the same run. CP-3b, once per
instantiation, is not on a hot path and is not measured. Recorded in the PR body.

## §9. Measurements

- JS→wasm entry near the floor: umbrella §8 M-11, `H/scripts/wasm_entry.sh`, log `H/logs/wasm-entry.log`
  (numbers in §4.1). Guard off, the same entry aborts from debug D = 30 / release D = 250 (versus the trap at
  25 / 200).
- wasm and stack switching: umbrella §8 M-10 (the fiber crate is not compiled; PR-1 §4.4 bans it).

## §10. Landing obligations

Every row is done by the PM / PMO at landing unless marked *PR* (PM 452: the ledger is the PM's).

- **L-1. Close `#11-vm-call-depth-guard`** (PMO, live ledger): with PR-1's and PR-1b's PR numbers and squash
  shas, and a pointer to PR-1's tracked cut check (PR-1 §5.3) as what keeps the closure true. The same edit
  adds to `#11-structured-clone-accessor-get` (whose entry names `#11-vm-call-depth-guard`): "the getter's
  native→JS re-entry passes CP-1 and each clone level passes W-6 (stack-safety PR-1 §5.1, §5.2)".
- **L-2. `#11-wasm-user-import-host-fn-builder`** (PMO): append the defect and the facts, not a
  requirement: "When JS imports land, a JS→wasm→JS→wasm recursion is bounded by CP-1 (`with_call_mode`) on
  the JS side and CP-3 at each JS→wasm entry; with the per-agent store (WebAssembly JS API §4.1) the
  outermost entry's wasmtime limit also bounds every nested wasm frame, with today's per-instance store
  only CP-3 does; every overflow on the path is a `RangeError` (wasm's by §7.1). Stack-safety PR-1b §2
  W-2." Ruled (PM 517): owner VM P4; trigger: wasm→JS user imports are accepted (the slot carries this L-2
  obligation); Re-eval-date 2027-01-31.
- **L-3. `#11-dom-walkers-iterative-over-ecs-links`** (PMO, via PM to L3): "wasm host functions
  (`get_text_content`, `query_selector`) run these walkers inside `WALKER_RESERVE` after wasm's own
  512 KiB, as the JS floor does (stack-safety PR-1b §4.1)".
- **L-4. `#11-wasm-entry-depth-band`** (registered, PM 517; owner VM P4): the entry band of §4.3. Defect "a
  JS→wasm entry with 256–768 KiB of stack left gets `RangeError` even when wasm would use little stack
  (callback depth ≈23–32 debug, ≈195–268 release on a 2 MiB thread)", remedy candidates of §4.3 (a
  per-entry wasm limit, which wasmtime 50.0.1 lacks; wasm on its own stack, which needs the guard redesigned
  for stack switching), trigger: wasmtime gains a per-store or per-entry stack limit, or a report lands in
  the band. The band is catchable (`RangeError`) and never aborts.
- **L-5. `#11-wasm-instantiate-sync-deviation`** (registered, PM 517; owner VM P4): the synchronous
  `WebAssembly.instantiate` deviation of §3 (*asynchronously instantiate a WebAssembly module* step 6.1
  queues a task).
- **L-6. Breadth record** (*PR*): `python3 .claude/skills/elidex-plan-review/preflight.py` on this memo at
  the PR's head, rc and K / M as printed (the label is unmapped; hand count K=1 / M=3).
