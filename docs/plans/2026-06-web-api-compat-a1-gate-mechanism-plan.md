# A1 — Web-API Core/Compat Gate Mechanism (plan-memo)

Plan date: 2026-06-20 JST
Status: **PLAN / DESIGN — pre-implementation. No `.rs` change in this PR-of-record yet.**
Program: `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` → Program A, **PR A1**.
Parent design (SSoT, locked): `docs/plans/2026-06-web-api-compat-split-design.md` (A0, merged `723b30ed`, PR #368).
Gate: this memo must pass `/elidex-plan-review` (RULE — edge-dense subsystem, ≥3 intersecting
invariant axes: registration-seam × engine-wide-mode × compile-feature × realm-scope ×
construction-ordering) **before** any implementation commit. Per CLAUDE.md
"Edge-dense work = multi-PR program + 実装前 plan-review 必須" and the umbrella.

> A1 builds **only the gate mechanism**: the carrier (level at every install seam) +
> enforcement (a policy consulted by every installer) + the mode selector (engine-wide
> `EngineMode` → derived `SpecLevelPolicy`, fixed at VM construction) + the
> `compat-webapi` cargo feature. **It moves no API and changes no behavior** — the shell
> supplies `BrowserCompat`, so every Modern + Legacy API installs exactly as today. The
> *real* demotion of storage (A2) / cookie (A3) / live-collections (B0) is downstream and
> reuses this gate. A1's proof-of-mechanism is a **test-only** marked-`Legacy` API
> exercising all four seams.

---

## §A. Spec coverage map (preflight hard-gate)

> A1 is an **infrastructure / mechanism** PR — it implements **no spec algorithm** and
> dispatches **no new web-platform surface**. The "spec" it is faithful to is the elidex
> **design contract** for the core/compat boundary and the engine-mode storage
> precondition, not a WHATWG/W3C algorithm. The map therefore names the *contract*
> sections the mechanism must honor + the *carrier site* each gate seam attaches to (no
> dispatch site is edited to change observable behavior). All citations webref-verified (§8).

| Spec section | Step | Branch | Touch (carrier/seam site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Design `14-script-engines-webapi.md` §14.4.2 (Web API core/compat) | gate vocabulary load-bearing | mechanism (no demotion) | `elidex-plugin/spec_level.rs` (enums) + 4 install seams | ✓ (4 seams closed, §3.3) | no (A1 marks nothing real) |
| Design §14.4.3 (engine-mode storage contract: core/app ⇒ async storage, no sync) | mode-enum doc pins precondition | mechanism + doc | `EngineMode` doc comment (§3.2) | ✓ (3 modes closed) | no |
| Design `12-dom-cssom.md` §12.1.2 (DOM core/compat) | gate must *be able to* express DOM `Legacy` | mechanism (B0 demotes) | `DomApiHandler` registry resolve seam (§3.3 seam-4) | n/a (B0-owned) | no |
| WHATWG HTML §12.2 / §12.2.4 (Web Storage / `StorageEvent`) | *named only* as the eventual A2 client of seam-2 | not demoted by A1 | none (A2) | n/a (A2) | n/a |

**Breadth**: K=2 design specs (14, 12) + 1 named-only HTML contract; M=4 entries (the 4
table rows above); the *observable* surface touched = **0 web-platform APIs** (A1 marks
nothing real `Legacy`). This is a mechanism PR → single-PR scope is correct (it is the
umbrella-approved base-case slice for the gate); the preflight's SPLIT-RECOMMENDED verdict
(K-inflated by three distinct in-repo design-doc labels, not three independent spec
surfaces) is overridden on that basis. A2/A3/B0/B1/E0 each re-run their own coverage map at
their plan time.

> **Threading precedent (not a spec row):** the `EngineMode` construction-param → VM-field →
> installer-read path reuses the existing `global_scope_kind` realm-gating idiom
> (`init.rs:87/728`, `mod.rs:2529`, `globals.rs:503` — verified 2026-06-20, §1). It carries
> no `§`-number so it is documented here rather than as a coverage-map entry.

**User-input flow**: none. A1 introduces no new untrusted-input parse and no new
value sanitization site. The `EngineMode` value is **embedder-supplied at construction**
(trusted host config, like `global_scope_kind`), not page-controlled.

---

## §0. Decisions this memo commits to (the A0-delegated calls)

A0 delegated the *exact mechanism* to this plan-review (A0 §1 R5 discipline: "A0 states the
requirement + constraint and delegates the exact implementation to the implementing PR's
plan-review"). The lens-determined calls:

1. **Carrier form = leveled *install call*, default-`Modern`, NO table-literal edits
   (§2.1).** The level travels at the **install call site / via a leveled helper variant**,
   not as a third tuple element on every `(&str, NativeFn)` table row. Decisive because:
   (a) editing the ~95 % Modern surface's table consts is mechanical noise that buys
   nothing (the default is `Modern`); (b) it keeps A1 **out of `window.rs` / `document.rs`
   entirely**, dissolving the #372 (media Slice 2b) collision for A1 — only A2 touches the
   `window.rs` storage table, and A2 is already sequenced *after* Slice 2b (A0 §6); (c) it
   mirrors the existing `global_scope_kind` gating idiom, which already wraps installs in a
   mode check (`globals.rs:503`) rather than tagging table rows. *One issue, one way.*

2. **`EngineMode` + `SpecLevelPolicy` live in `elidex-plugin`, threaded as a
   VM-construction parameter exactly like `global_scope_kind` (§2.2 / §3.2).** Not
   `bind_session` (A0 R3-7: installers run at construction, before any bind). The public
   engine/VM constructors default to `BrowserCompat` ⇒ zero behavior change.

3. **One policy, four seams, uniform (§3.3).** `install_methods`/`install_ro_accessors`/
   `install_rw_accessors` (tables) + direct `register_*_global()` installers +
   `install_event_handler_attrs` (the `onstorage` seam) + the `DomApiHandler` registry
   resolve path. A1 makes **all four** consult the same derived policy. No fifth bespoke gate.

4. **`compat-webapi` is an independent cargo feature, declared but NOT yet load-bearing on
   the backend (§2.3 / §3.4).** A1 declares it and wires the policy/install plumbing to
   read it; A1 does **not** cfg-gate the storage backend dependency (that is A2 — "no API
   moves" means `engine`-without-`compat-webapi` need not yet *compile* the storage code
   out; A1 only lands the mechanism). `compat-webapi` is **not** implied-by `engine`
   (A0 R5-1: features are additive — implied-by would make the app exclusion
   unimplementable).

5. **Proof = a test-only `Legacy` marker across all four seams (§5).** A1 marks **no real**
   Web API `Legacy`. Storage/cookie demotion is A2/A3.

These are lens-collapsed (One-issue-one-way + Ideal-over-pragmatic + collision-avoidance);
they are *recommendations to the plan-review*, surfaced as Open Questions in §7 for the
5-agent gate to falsify — not unilateral closes.

---

## §1. Verified anchors (re-grepped at HEAD `2f4a9d5a`, 2026-06-20)

Every site below was grepped against `main` HEAD in this session, not transcribed from A0.
**Two drifts vs. the A0 doc are recorded** (A0 §2-table cited stale coordinates):

| Symbol / site | Verified location | Notes / A0-drift |
|---|---|---|
| `WebApiSpecLevel { Modern, Legacy, Deprecated }` | `crates/core/elidex-plugin/src/spec_level.rs:68` (`#[non_exhaustive]`) | matches A0 |
| `DomSpecLevel { Living, Legacy, Deprecated }` | `spec_level.rs:25` | matches A0 |
| `EngineMode` / `SpecLevelPolicy` | **absent** (grep: only `ProcessModel` `lib.rs:86`, `AutoRepeatMode`) | confirms new types needed |
| `register_globals()` **call** | `vm/init.rs:740` | **A0 DRIFT** — A0 §2/§3.2b cites `:734`; actual `:740` |
| `fn register_globals` body | `vm/globals.rs:72` | — |
| `install_methods` / `install_ro_accessors` / `install_rw_accessors` | `vm/globals.rs:962` / `:988` / `:1013` | all `&[(&str, NativeFn[, NativeFn])]`; ro/rw `#[cfg(feature="engine")]` |
| `register_window_prototype` | `vm/host/window.rs:411` | A1 does **not** edit (decision §2.1) |
| storage accessor install call | `window.rs:438` `install_ro_accessors(proto_id, WINDOW_STORAGE_ACCESSORS)` | A2 site, not A1 |
| `install_event_handler_attrs` **call** | `window.rs:445` | A1 makes the *def* policy-aware, not this call |
| `WINDOW_STORAGE_ACCESSORS` const | `window.rs:525` | A2 site |
| `fn install_event_handler_attrs` | `vm/host/event_handler_attrs.rs:73`; shared loop `install_handler_attrs_from_list:145` (sibling `install_handler_attr_family:215`); `EVENT_HANDLER_ATTRS` import `:50` | seam-3 (line re-verified 2026-06-20) |
| `register_storage_global` def | `vm/host/storage.rs:237` | A2 site (named here for seam-2 shape) |
| `register_storage_global` call | `vm/globals.rs:483` (**unconditional** — outside the `GlobalScopeKind::Window` branch at `:503`) | corroborates A0/A2 R5-6 worker over-exposure |
| `register_storage_event_global` call | `vm/globals.rs:658` | A2 site |
| `global_scope_kind` field | `vm/mod.rs:2529`; set `init.rs:728`; param `new_with_scope(global_scope_kind)` `init.rs:87` | **threading precedent for `EngineMode`** |
| public VM constructors | `Vm::new` `init.rs:22→23`; `new_worker` `:43`; `new_service_worker` `:71` — all `→ new_with_scope` | each gains a mode default |
| `ElidexJsEngine::new` | `engine.rs:35` → `vm: Vm::new()` `:39` | engine-level entry for mode param |
| `NetworkMiddleware::spec_level -> WebApiSpecLevel::Modern` | `elidex-plugin/src/traits.rs:271-272` | **the only existing `WebApiSpecLevel` carrier** — precedent for the trait-default form |
| `DomApiHandler::spec_level` (default `Living`) | emitted by `define_api_handler!` (invocation `dom_api.rs:3`, `fn spec_level` default body `macros.rs:21`, default arg `Living` `dom_api.rs:12`) | seam-4 carrier already exists |
| `PluginRegistry::resolve` | `elidex-plugin/src/registry.rs:32` (pure name→handler; **ignores `spec_level`**) | enforcement gap to close |
| live `DomApiHandler` dispatch | `invoke_dom_api` `vm/host/dom_bridge.rs:475`, resolves via `ctx.vm.dom_registry.resolve(handler_name)` `:490` | **A0 DRIFT** — A0 §2-table located `invoke_dom_api` under `elidex-script-session` (`dom_bridge.rs:475`); it actually lives in `elidex-js/src/vm/host/dom_bridge.rs`. Line `:475` correct, crate wrong. |
| `dom_registry` field | `vm/mod.rs:277` `Rc<elidex_dom_api::registry::DomHandlerRegistry>` | seam-4 enforcement attaches here |

**Drift consequence**: both A0 drifts are *citation* errors, not design errors — the
mechanism is unchanged. Recorded so the implementation uses `:740` / the correct crate, and
so a follow-up can fix the A0 doc's two coordinates (low priority; fold into the F2 sweep or
a one-line A0 amend — **not** A1's scope).

---

## §2. Design decisions (detail)

### 2.1 Carrier form — leveled install, default-Modern (vs. per-row tuple)

The install helpers take flat tables `&[(&str, NativeFn)]` (`globals.rs:962/988/1013`).
Two carrier shapes were considered:

- **(rejected) Per-row level** — widen every tuple to `(&str, NativeFn, WebApiSpecLevel)`.
  Forces editing **every** `WINDOW_*` / `DOCUMENT_*` / global table literal in `window.rs`,
  `document.rs`, `navigator.rs`, … (the entire Modern surface) to append `Modern` — pure
  noise, and it puts A1 squarely on top of #372's `window.rs` edits.
- **(chosen) Leveled install call** — the *caller* states the level; tables are untouched.
  Concretely, A1 adds level-aware install entry points (e.g. a `level: WebApiSpecLevel`
  parameter on a new `install_*_leveled` variant, or a thin wrapper the existing helper
  delegates into) that consult the policy and **no-op the install when the policy excludes
  the level**. The existing zero-arg-level helpers keep their signatures and default to
  `Modern` (so every current call is unchanged and installs exactly as today). A2/A3 later
  switch the *storage/cookie* install calls to the leveled variant with `Legacy`.

This mirrors the established idiom: realm gating is already expressed by **wrapping the
install in a mode check at the call site** (`globals.rs:503` `if matches!(self.global_scope_kind,
Window) { register_custom_element_registry_global() }`), not by tagging rows. The gate is the
same shape, one axis over (spec-level instead of realm). *One issue, one way.*

**Net A1 file touch for the carrier**: `globals.rs` (helper variants + policy read inside
them) — **not** `window.rs` / `document.rs` / `navigator.rs`. Collision with #372 avoided.

### 2.2 `EngineMode` placement + threading

- **Types in `elidex-plugin`** next to the enums (`spec_level.rs`): `EngineMode {
  BrowserCompat, BrowserCore, App }`, `SpecLevelPolicy` (the per-layer derived policy:
  "is this `WebApiSpecLevel`/`DomSpecLevel` installed?"), and a derive fn
  `EngineMode → SpecLevelPolicy`. Home rationale: every layer (Web-API install plumbing in
  `elidex-js`, style/E0, DOM/B1) must *name* the same authority (A0 R3-6 whole-engine
  consistency); `elidex-plugin` is the shared vocabulary crate all of them already depend on.
- **Threading = construction param, mirroring `global_scope_kind`.** `new_with_scope`
  (`init.rs:87`) gains an `engine_mode` (or pre-derived `SpecLevelPolicy`) parameter; the VM
  stores it as a field beside `global_scope_kind` (`mod.rs:2529`); `register_globals`
  (called `init.rs:740`, *before* `bind_session`) reads it via the leveled install helpers.
  The three public constructors (`Vm::new` `:22`, `new_worker` `:43`, `new_service_worker`
  `:71`) default to `BrowserCompat`. `ElidexJsEngine::new` (`engine.rs:35`) likewise gains a
  mode param with a `BrowserCompat` default (or an additive `new_with_mode`), so the shell —
  the embedder — supplies the mode; **default = `BrowserCompat` = zero behavior change**.
- **Why not `bind_session`** (A0 R3-7): installers run at construction. A bind-time mode
  could not *prevent* an install; it would need a second *removal* path = a strangler. The
  construction-param form prevents the install up front — the clean single-writer shape.

### 2.3 `compat-webapi` feature

A runtime policy still *links* compat shim code. The `App` build (design §14.4.3 "コンパイル
時除外") must be able to ship without the sync-storage shim in the binary. So:

- A1 **declares** `compat-webapi` in `elidex-js/Cargo.toml` as an **independent** feature
  (NOT `engine`-implied — A0 R5-1: additive features mean an `engine`-implied `compat-webapi`
  would force it on for the app's `--features engine` build, making the exclusion
  impossible). The browser/default profile turns on `engine` + `compat-webapi` *separately*;
  the app profile selects `engine` alone.
- A1 wires the policy/install plumbing to **read** the cfg (the compile-time half of the duo:
  "is it in the binary?" vs. the runtime mode's "is it reachable now?").
- A1 does **NOT** cfg-gate `WebStorageManager`/`SessionStorageState` or the
  `elidex-storage-core` dependency. "No API moves" ⇒ `engine`-without-`compat-webapi` need
  not yet compile the storage code out; that drop is **A2's** AC (A0 R7-1/R9-1: gate the Web
  Storage *code*, never the shared `storage-core` crate which Cache/SW need). A1 lands only
  the *mechanism* + the feature *declaration*.

**Not two authorities — a hard ceiling + a soft selector (F5, §7 Q5 closed).** The
`compat-webapi` cfg and the runtime `EngineMode` answer **different, ordered** questions and
cannot contradict: the cfg is the **hard presence ceiling** ("is the compat shim *linked* in
this binary?") and `EngineMode` is the **soft per-session selector** *within what is present*
("of the linked surface, what does this VM install?"). The ordering is total — the cfg
dominates: with `compat-webapi` **off** (the app profile), the `Legacy` install arms are
compiled out, so a VM constructed with **any** mode (even `BrowserCompat`) simply installs
`Modern` only — asking for an absent `Legacy` is a **compile-time no-op, not a contradiction
or panic**. So `BrowserCompat` under cfg-off degrades gracefully to `BrowserCore` behaviour
rather than half-installing. `EngineMode` remains the single *runtime* mode authority
(requirement 5); the cfg is a compile-time ceiling above it, never an independent second
runtime switch (no code path reads the cfg to *override* a runtime-chosen mode). The browser/
default profile sets `engine` + `compat-webapi` **on** and supplies `BrowserCompat`, the
zero-behavior-change baseline.

---

## §3. Mechanism — the four seams

### 3.1 Why four (A0 §3.2a)

A table-only gate is incomplete: legacy top-level globals (`StorageEvent`, a future
`XMLHttpRequest`) install via flat `register_*_global()` calls, `onstorage` installs via the
event-handler-attr seam, and bridge-dispatched DOM methods resolve through the
`DomApiHandler` registry. One policy must reach all four, else A2/A3/B1 each grow a one-off
gate (the "new seam + N legacy" anti-pattern).

### 3.2 The single authority

`EngineMode` (construction param) → `SpecLevelPolicy` (derived once, stored on the VM). Every
seam asks the *same* policy `policy.installs(level) -> bool`. `BrowserCompat` ⇒ installs
`Modern + Legacy` (current behavior). `BrowserCore`/`App` ⇒ `Modern` only (⚠ not selectable
for a real session until `#11-async-core-storage-cookiestore` lands — §6; A1 exercises these
modes by **unit test only**).

### 3.3 Seam-by-seam

| # | Seam | Site | A1 change |
|---|---|---|---|
| 1 | Method/accessor tables | `install_methods`/`install_ro_accessors`/`install_rw_accessors` `globals.rs:962/988/1013` | Add leveled variants (§2.1) that consult `policy.installs(level)`; existing zero-level callers default `Modern` (no-op for the 95 %). |
| 2 | Direct global installers | the `register_*_global()` sequence in `register_globals` `globals.rs:72…` (e.g. `register_storage_global` call `:483`, `register_storage_event_global` `:658`) | **Demotable-installer routing (F1, committed):** A1 routes the **demotable** `register_*_global` calls — the installers a future mode will exclude: **today `register_storage_global` (`:483`) + `register_storage_event_global` (`:658`); future XHR** — through the policy-aware install path **now**, at level `Modern` (no behavior change — `policy.installs(Modern)` is always true under `BrowserCompat`). A2/A3 then change **only the level argument** (`Modern → Legacy`) at those call sites — they do **not** re-touch the seam wiring (§7 Q6 resolved). Permanently-`Modern` globals (`crypto`/`websocket`/`fetch`/…) need **not** route through the gate (they are never excluded — gating them would be churn, not One-issue-one-way: the anti-pattern is *two ways to gate `Legacy`*, not gating `Modern`). A1 marks nothing real `Legacy`; the test API is the only excluded entry. |
| 3 | Event-handler IDL attrs | `install_event_handler_attrs` def `event_handler_attrs.rs:73` → shared loop `install_handler_attrs_from_list:145` (`install_handler_attr_family:215` is a sibling scope-filtered loop) | Make the install loop level-aware so a `Legacy`-classified handler attr (the future `onstorage`, A2) is skipped when excluded. A1 adds the capability + a test handler attr; it does **not** classify `onstorage` (A2). |
| 4 | `DomApiHandler` registry | enforcement home = **`elidex-dom-api::registry::DomHandlerRegistry`** (`= PluginRegistry<dyn DomApiHandler>`, `mod.rs:277` `Rc<…DomHandlerRegistry>`); carrier `DomApiHandler::spec_level` (`define_api_handler!`, default body `macros.rs:21`, default `Living`); live dispatch `invoke_dom_api` `dom_bridge.rs:475`/resolve `:490` | **Layering-pinned (F3, committed):** enforce by **withholding `Legacy` handlers at registration** into `DomHandlerRegistry` (the elidex-dom-api crate, where the concrete `DomApiHandler` trait + its `spec_level()` are visible) — **not** in the generic `elidex-plugin::PluginRegistry::resolve` (`registry.rs:32`; it is `T: ?Sized`-generic and cannot read `spec_level` without coupling the foundational crate to a DOM trait — a layering inversion), and **not** as a per-call check in the `invoke_dom_api` hot path (runs per DOM mutation). Registration-time withholding keeps resolve a pure map lookup and the policy a single-writer construction-time decision. (Live-collection getters that allocate directly in `document.rs` and bypass `invoke_dom_api` are gated at **seam-1**, not here — A0 §3.4; A1 only makes seam-4 *capable*, B0/B1 demote.) |

**Uniformity check**: all four end at one predicate `policy.installs(level)`. No seam grows a
bespoke `if mode == …` branch (requirement: One issue, one way).

### 3.4 Enforcement vs. classification (A1 vs A2/A3/B)

A1 = **carrier + enforcement + selector**. A1 classifies **nothing real** as `Legacy` (the
`SpecLevelPolicy` default leaves the whole surface `Modern`). The real classification —
storage/`StorageEvent`/`onstorage` → `Legacy` (A2), `document.cookie` → `Legacy` (A3),
live-collection family (B0) — flips specific install calls to the leveled variant with
`Legacy`. A1's only `Legacy`-marked entity is the **test API** (§5).

---

## §4. File-level change plan (implementation, for the post-review commit)

> Listed so the plan-review can scope blast-radius. No code is written until review passes.

1. **`crates/core/elidex-plugin/src/spec_level.rs`** — add `EngineMode { BrowserCompat,
   BrowserCore, App }` (`#[non_exhaustive]`, `Default = BrowserCompat`), `SpecLevelPolicy`
   (carries which `WebApiSpecLevel`/`DomSpecLevel` install), `EngineMode::web_api_policy()` /
   `dom_policy()` derive fns, and `SpecLevelPolicy::installs(WebApiSpecLevel) -> bool` /
   `installs_dom(DomSpecLevel) -> bool`. Doc comment pins the §14.4.3 precondition
   (`BrowserCore`/`App` not selectable for a real session until
   `#11-async-core-storage-cookiestore`).
2. **`elidex-js/Cargo.toml`** — declare `compat-webapi` feature (independent of `engine`).
3. **`elidex-js/src/vm/mod.rs`** — add the policy field beside `global_scope_kind` (`:2529`).
   **Invariant comment required (F4):** annotate the field "*set before any installer runs;
   `register_globals` (`init.rs:740`) is the sole install entry and is called last in the
   ctor — if a future installer is added that runs earlier in construction it MUST observe a
   set policy, so keep this field's initialization at/near the top of the struct literal.*"
   This is the single failure mode that would make the gate **silently no-op** (install
   `Legacy` while the policy meant to exclude it), so it is pinned, not left implicit.
4. **`elidex-js/src/vm/init.rs`** — `new_with_scope` (`:87`) gains a mode/policy param; field
   set near `:728` (the policy field set **precedes** the `vm.inner.register_globals()` call
   at `:740` — verified ordering, the F4 invariant); the three public ctors (`:22/:43/:71`)
   default `BrowserCompat`.
5. **`elidex-js/src/engine.rs`** — `ElidexJsEngine::new` (`:35`) mode param (default
   `BrowserCompat`) or additive `new_with_mode`; pass through to `Vm`.
6. **`elidex-js/src/vm/globals.rs`** — leveled install-helper variants (seam-1); route the
   **demotable** `register_*_global` calls (`register_storage_global` `:483` /
   `register_storage_event_global` `:658`) through the policy-aware path in `register_globals`
   (seam-2, level `Modern` for now); read the `compat-webapi` cfg.
7. **`elidex-js/src/vm/host/event_handler_attrs.rs`** — level-aware `install_handler_attrs_from_list`
   (`:145`) (seam-3).
8. **`elidex-dom-api::create_dom_registry` (F3-pinned, feasibility-verified)** — the registry
   is populated by `create_dom_registry()` (`elidex-dom-api/src/registry.rs:18`) via a sequence
   of `register_static(name, Box::new(Handler))` calls (each handler is `Box<dyn DomApiHandler>`,
   so `.spec_level()` is callable), and the VM builds it at `init.rs:156`
   (`Rc::new(create_dom_registry())`) — a construction-time point **before** `register_globals`
   (`:740`). A1 makes `create_dom_registry` **policy-aware** (takes the policy / a post-filter)
   and **withholds handlers whose `spec_level()` the policy excludes** — registration-time, so
   resolve (`registry.rs:32`) stays a pure map lookup. **Not** `elidex-plugin::PluginRegistry::resolve`
   (generic `T: ?Sized`, cannot read `spec_level` without coupling the foundational crate to a
   DOM trait) and **not** a per-call check in `elidex-js`'s `invoke_dom_api` hot path
   (`dom_bridge.rs:490`). Pinning the home to `elidex-dom-api` (rather than
   `elidex-script-session`) also keeps A1 off the `elidex-script-session/src/engine.rs` file
   that #372 edits (§6).
9. **Tests** — §5.

Files A1 **does not** touch: `window.rs`, `document.rs`, `navigator.rs`, `storage.rs`
(beyond possibly reading the new helper) — all are A2/A3 sites — **and**
`elidex-script-session/src/engine.rs` (seam-4 is pinned to `elidex-dom-api` by F3, so A1's
DOM-registry enforcement does not enter `elidex-script-session`). This keeps A1 off the #372
`window.rs` path and off the `elidex-script-session/src/engine.rs` file #372 edits; the one
remaining #372 overlap is `elidex-js/src/engine.rs` (§6).

---

## §5. Testing / Acceptance criteria

From A0 §5 A1 row, made concrete:

1. **No behavior change under `BrowserCompat`** — existing VM/engine test suites pass
   unchanged (the default mode installs `Modern + Legacy` exactly as today). This is the
   primary regression guard.
2. **Four-seam exclusion proof** — a **test-only** API marked `Legacy`, installed via **each
   seam**, is:
   - present under `BrowserCompat`,
   - **absent** under `BrowserCore` **and** `App`,
   for: (a) a method/accessor **table** entry (seam-1), (b) a **`register_*_global`** installer
   (seam-2), (c) an **`install_event_handler_attrs`** handler attr — the `onstorage` seam
   shape (seam-3, A0 R8-1), (d) a **`DomApiHandler`** registry handler (seam-4).
3. **`compat-webapi` declared + read** — feature is independent of `engine` (additive); the
   install plumbing reads the cfg. Both profiles compile: browser (`engine` +
   `compat-webapi`) and a profile with `engine` alone. *A1 does NOT require the storage
   backend to drop under `engine`-alone* (that is A2 — A0 R7-1); A1's compile check only
   proves the feature wiring + mechanism build cleanly both ways.
4. **`BrowserCore`/`App` are test-only** — no production code path selects them (the shell
   supplies `BrowserCompat`); asserted by leaving the shell default unchanged.
5. **Mode-enum doc** states the §14.4.3 precondition (§4 item 1).
6. **Write-before-install ordering pinned (F4)** — the policy field carries the invariant
   comment (§4 item 3) and the seam-2/4 exclusion test (item 2) is the live guard: because the
   test `Legacy` API is installed *through* `register_globals` (called last, `init.rs:740`), a
   test that observes it **absent** under `BrowserCore` is exactly a test that the policy was
   set *before* the installer ran. A regression that moved an installer ahead of the field-set
   would flip that test, so no separate ordering assertion is needed beyond the seam-exclusion
   test + the field comment.

`mise run ci` green; per-crate `cargo test -p elidex-plugin -p elidex-js -p
elidex-script-session --all-features`.

---

## §6. Collision / sequencing (re-confirmed)

- **#372 (media Slice 2b-ii, branch `media-query-slice2bii`) is OPEN** (verified this
  session: state OPEN/UNSTABLE, Codex `/external-converge` R1 in flight). #372's edited files
  are (verified `gh pr view 372 --json files`): **`crates/script/elidex-js/src/vm/host/window.rs`**,
  **`crates/script/elidex-js/src/engine.rs`**, **`crates/script/elidex-script-session/src/engine.rs`**.
  **A1's chosen carrier form (§2.1) keeps A1 out of `window.rs` entirely** — but the collision
  is **not fully dissolved** (F2 correction): A1's `engine.rs` touch (§4 item 5,
  `ElidexJsEngine::new` mode param) **overlaps `elidex-js/src/engine.rs`**, which #372 also
  edits (different region — A1 ctor `:35`; #372 `HostDriver` impl `~:497`). Overlap-surfaces
  to re-check at open-time, **enumerated**:
  - `crates/script/elidex-js/src/engine.rs` — **overlaps #372** (A1 ctor vs #372 `HostDriver` impl; LOW, distinct regions).
  - `crates/script/elidex-script-session/src/engine.rs` — #372 edits it; **A1 no longer touches it** (F3 pins seam-4 to `elidex-dom-api`, not `elidex-script-session` — overlap removed by the F3 fix).
  - `crates/script/elidex-js/src/vm/globals.rs` — A1-only (not on the Slice 2b path).

  So A1 *implementation* proceeds once #372 lands and `main` is rebased, with **low** residual
  collision risk concentrated on `elidex-js/src/engine.rs`. **A2** (which *does* edit the
  `window.rs` storage table) remains hard-sequenced *after* Slice 2b (A0 §6).
- **`#11-async-core-storage-cookiestore` precondition** — A1 must not let any real session
  select `BrowserCore`/`App` (those modes have no storage API until the async core lands,
  design §14.4.3). Enforced by: shell default = `BrowserCompat`; the two non-compat modes
  exercised by unit test only; the mode-enum doc carries the warning (§4 item 1).
- **Worktree isolation** — implementation builds in a dedicated worktree off `origin/main`
  (`git worktree add -b webapi-compat-a1 <dir> origin/main`). This plan-memo PR is doc-only on
  `docs/plans/`.

---

## §7. Open questions + resolutions

> **Status after `/elidex-plan-review` (2026-06-20, 5-agent, 0 CRIT / 2 IMP / 6 MIN).** The
> review **resolved** Q4/Q5/Q6 (folded into the body as F1/F3/F5); Q1/Q2/Q3 were confirmed by
> the review (no divergence found — recorded as standing design decisions, not open). Only Q7
> (re-grep discipline) remains a live instruction to the implementer.

1. **Carrier form (§2.1) — RESOLVED (review-confirmed).** Leveled-install-call, default-Modern,
   no-table-edit is the chosen form. Axis-1/Axis-2 confirmed it is a faithful realization of A0
   §3.2a's "level on the registration entry" (the level rides the install *call*, the seam the
   table feeds) and keeps algorithm out of `vm/host/`. No further question.
2. **Policy vs. trait (A0 §8 Q1) — RESOLVED (review-confirmed).** "Level + `SpecLevelPolicy` at
   install" stands; storage/cookie are thin backend bindings needing no new `WebApiHandler`
   dispatch trait. `NetworkMiddleware::spec_level` (`traits.rs:271`) is the precedent for the
   trait-default form *where a trait already exists*.
3. **`EngineMode` home + threading (§2.2) — RESOLVED (review-confirmed).** `elidex-plugin` next
   to the enums (Axis-1 verified the crate has zero `elidex-*` deps, so no inversion);
   construction param, not `bind_session`; one authority feeding Web-API + style + DOM. Both
   `EngineMode` and the derived `SpecLevelPolicy` live in `elidex-plugin`.
4. **Seam-4 enforcement locus — RESOLVED (F3).** Withhold `Legacy` handlers **at registration**
   into `elidex-dom-api::DomHandlerRegistry` (where `dyn DomApiHandler`/`spec_level` are
   visible) — **not** `elidex-plugin::PluginRegistry::resolve` (generic, would couple the
   foundational crate to a DOM trait) and **not** a per-call check on the `invoke_dom_api` hot
   path. Keeps resolve a pure map lookup; single-writer at construction. (§3.3 seam-4, §4 item 8.)
5. **Feature/runtime duo — RESOLVED (F5).** One mechanism, two faces: cfg = binary presence,
   `EngineMode` = per-session reachability; consistent by construction (build profile sets both
   coherently), not two independent authorities. No strangler. (§2.3 closing paragraph.)
6. **Seam-2 gating scope — RESOLVED (F1, One-issue-one-way committed).** A1 routes the
   **demotable** `register_*_global` installers (today `register_storage_global` /
   `register_storage_event_global`; future XHR) through the policy-aware install path now (level
   `Modern`, no behavior change), so A2/A3 change **only the level argument** and never re-touch
   the seam wiring. Permanently-`Modern` globals are not gated (never excluded). The seam is
   built once; the classification flips once. (§3.3 seam-2.)
7. **Re-grep discipline (Axis 5) — LIVE instruction.** This is a 2026-06-20 snapshot; the
   implementer re-greps §1 and re-confirms #372's branch state + edited-file set at open-time.
   Two A0 citation drifts are recorded (§1): `register_globals` call `:740` (not `:734`);
   `invoke_dom_api` in `elidex-js/vm/host/dom_bridge.rs` (not `elidex-script-session`). The A0
   doc's two stale coordinates should be corrected via a one-line A0 amend or the F2 clerical
   sweep — **not** A1's scope.

---

## §8. Citation appendix (webref-verified)

| Concept | Source | Anchor |
|---|---|---|
| Web API core/compat boundary | design `14-script-engines-webapi.md` §14.4.2 | (in-repo) |
| Engine-mode storage contract (core/app ⇒ async, no sync) | design §14.4.3 | (in-repo) |
| DOM core/compat | design `12-dom-cssom.md` §12.1.2 | (in-repo) |
| Web Storage / `StorageEvent` (named-only; A2's client) | WHATWG HTML §12.2 / §12.2.4 | `#storage` / `#the-storageevent-interface` |
| `document.cookie` (named-only; A3's client) | WHATWG HTML §3.1.4 | `#dom-document-cookie` |

> A1 cites no new WHATWG algorithm prose (it implements no algorithm). The HTML rows are the
> downstream clients of the seams A1 builds, named for traceability only. WHATWG anchors carried
> verbatim from A0 §7 (already webref-verified there).
