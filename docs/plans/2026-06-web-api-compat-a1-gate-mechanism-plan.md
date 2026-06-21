# A1 — Web-API Core/Compat Gate Mechanism (plan-memo)

Plan date: 2026-06-20 JST · **Revised 2026-06-21 (post-Codex-R6 → option-A general gate)**
Status: **PLAN / DESIGN — re-opened for re-implementation.** The first implementation
(landed `b8237374`…`87e33a0d`, PR [#376](https://github.com/send/elidex/pull/376)) built a
**storage-specific point solution**; Codex `/external-converge` R6 (4×P2, with R1/R5 realm
findings = a 3rd+ round of gate-mechanism critique) showed it is **not** the general
"level-gated *install* at every seam" mechanism A0/§0.3 promised. User chose **option A:
redesign A1 into the general gate**. This memo is the revised design; it must re-pass
`/elidex-plan-review` (the redesign **reverses** the prior plan-reviewed "inline
storage-specific guard" decision — edge-dense RULE) before the re-implementation commit.
Program: `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` → Program A, **PR A1**.
Parent design (SSoT, locked): `docs/plans/2026-06-web-api-compat-split-design.md` (A0, merged `723b30ed`, PR #368).
Gate: edge-dense subsystem, ≥3 intersecting invariant axes (registration-seam ×
engine-wide-mode × compile-feature × realm-scope × construction-ordering). Per CLAUDE.md
"Edge-dense work = multi-PR program + 実装前 plan-review 必須" and the umbrella.

> A1 builds **only the gate mechanism**: (a) the **one general predicate**
> `SpecLevelPolicy::installs(level)` / `installs_dom(level)`, (b) **every demotable-destined
> install site routed through it at level `Modern`** — so A2 (storage) / A3 (cookie) /
> B (live-collections) become **pure level-flips** (`Modern → Legacy` at the API's
> classification point, no seam re-wiring), (c) the engine-wide `EngineMode` → derived
> `SpecLevelPolicy` selector fixed at VM construction, and (d) the `compat-webapi` cargo
> feature as the compile-time hard ceiling. **It moves no API and changes no behavior** —
> A1 classifies **nothing** `Legacy` (the whole surface stays `Modern`), the shell supplies
> `BrowserCompat`, so every API installs exactly as today. The gate reaches **all four
> install seams** (method/accessor tables · direct `register_*_global` · event-handler IDL
> attrs · the `DomApiHandler` registry); A1 wires a real `Modern` caller at each so none of
> the routing is dead code and downstream PRs only flip a level.

---

## §R. Why this is a redesign (Codex R6 → option A)

The first A1 was correct in *types* (`EngineMode`/`SpecLevelPolicy` in `elidex-plugin`,
construction-param threading, realm inheritance) but **storage-specific in its wiring**: a
family-named `VmInner::installs_web_storage()` predicate gated only the Web Storage seams.
Codex R6 (`87e33a0d`, 4×P2) is a coherent structural critique — not four nits — and with
R1 (worker realm) + R5 (wasm realm) it is the **3rd+ round of gate-mechanism findings**
(cross-round structural root). The root: **A1-as-built is a point solution, not the general
mechanism A0 §3.2 promised** ("a level at *every* install seam + one policy consulted by
*every* installer"). The earlier "drop the leveled-install helpers as dead code" decision
over-corrected into storage-specificity — and the dead-code objection **dissolves** once A1
routes **all** demotable-destined install sites through the one predicate at `Modern` (then
the sites *are* the callers, and A2/A3/B become pure level-flips). The four R6 drivers:

- **F8** (`elidex-dom-api/src/registry.rs:57`) — seam-4 registry-withholding makes a Legacy
  DOM method a **`TypeError` at call**, not **absent**. The JS-**property install** (the host
  table, seam-1) is the absence lever; the registry is **dispatch-level defense-in-depth**.
  Fixed by §3.3 D4 (lever hierarchy) + wiring the live-collection *table* install (seam-1).
- **F9** (`vm/globals.rs:1059`) — `installs_web_storage` is **storage-specific**; cookie
  (A3) / live-collections (B) would each need their **own** guard → not the one-way
  `installs(level)` flip the mechanism promised. **Mechanism incomplete for direct table /
  global installs.** Fixed by §0.1/§3.3 (one general predicate + all demotable sites pre-wired).
- **F10** (`engine.rs:58`) — `new_with_mode` is **public**; an embedder can select
  `BrowserCore`/`App` for a real session **before** `#11-async-core-storage-cookiestore`
  lands → a contract-violating no-storage session. A doc comment does not enforce the
  test-only invariant. Fixed by §3.5 D5 (`#[cfg(test)]`-gate the mode constructor).
- **F11** (`vm/globals.rs:1051` / `event_handler_attrs.rs`) — `window.onstorage` (the
  deferred seam-3) installs via the shared `EVENT_HANDLER_ATTRS` family loop with **no
  policy-aware seam**, so when A2 flips storage to Legacy, `onstorage` stays exposed. Fixed
  by §3.3 seam-3 (per-attr level in the family loop) — **wired now, at `Modern`**.

`seen_findings` carried into the resumed loop = {F1, F2, F3(FP), F4, F5, F6, F7, F8, F9,
F10, F11}. F1 (worker realm) + F7 (wasm realm) realm-inheritance fixes are **kept** (§3.6).

### §R.1 — R7 refinement (per-family classification source)

Codex **R7** (3× P2 on `0320d0c8`) is the same structural theme one layer deeper than R6: a
*single level flip must make the whole family absent*, but the first redesign's **per-site
`Modern` literals** captured only some surfaces of each family. The root (a missing
abstraction) and its fix:

- **R7-2 (window.rs — collapse Web Storage gates to one decision):** FIXED — introduced a
  **single classification source per family** (`web_storage_spec_level()` /
  `document_cookie_spec_level()` / `live_collection_spec_level()` in `elidex-script-session`,
  beside `event_handler_attr_spec_level`); every install seam reads its family's source, so a
  family demotes by flipping **one source** (§0.2 / §2.1). Enforcement stays the one general
  `installs`/`installs_dom` predicate (F9); classification is now per-family-source — complementary.
- **R7-1 (onstorage content-attribute path):** the `onstorage` *accessor* now reads
  `web_storage_spec_level()` (tied to the family). The `<body onstorage="…">` /
  `setAttribute("onstorage",…)` content-attribute registration in `EventHandlerAttributeConsumer`
  + `StorageEvent` delivery are **A2's broader suppression scope** (A0 §5 A2 row — spans the VM
  *and* the shell tab/IPC plumbing); A2 wires them to read the same source. Genuine cross-PR
  boundary, not an edge-defer.
- **R7-3 (live-collection family completeness):** the single source `live_collection_spec_level()`
  is established and seam-1c reads it; the **full surface sweep** (`forms`/`images`/`links`/
  `children` + `Element.prototype`/`table.rows`/`select.options` across sibling files) is **B0's**
  classification work (A0 §5 B0 row / §1.3 / plan-review Q6), routed through the **same** source —
  not a new gate. Genuine cross-PR boundary.

Net: the per-family-source abstraction (the structural root) is fully landed in A1; each family's
*surface-capture completeness* is its downstream owner's scope, now flowing through the one source.

---

## §A. Spec coverage map (preflight hard-gate)

> A1 is an **infrastructure / mechanism** PR — it implements **no spec algorithm** and
> dispatches **no new web-platform surface**. The "spec" it is faithful to is the elidex
> **design contract** for the core/compat boundary + the engine-mode storage precondition.
> The map names the *contract* sections the mechanism must honor + the *carrier site* each
> gate seam attaches to. A1 edits no dispatch site to change observable behavior. All
> citations webref-verified (§8).

| Spec section | Step | Branch | Touch (carrier/seam site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Design `14-script-engines-webapi.md` §14.4.2 (Web API core/compat) | gate vocabulary load-bearing | mechanism (no demotion) | `elidex-plugin/spec_level.rs` (enums + policy) + **all 4 install seams routed** | ✓ (4 seams closed, §3.3) | no (A1 marks nothing real) |
| Design §14.4.3 (engine-mode storage contract: core/app ⇒ async storage, no sync) | mode-enum doc pins precondition + **ctor enforces it** (F10) | mechanism + doc + cfg-gate | `EngineMode` doc (§3.2) + `new_with_mode` `#[cfg(test)]` (§3.5) | ✓ (3 modes closed) | no |
| Design `12-dom-cssom.md` §12.1.2 (DOM core/compat) | gate must *be able to* express DOM `Legacy` at the **property** seam | mechanism (B0 demotes) | `DOCUMENT_METHODS` live-collection sub-table install (seam-1) + `DomHandlerRegistry` (seam-4, defense-in-depth) | n/a (B0-owned) | no |
| WHATWG HTML §12.2 / §12.2.4 (Web Storage / `StorageEvent`) | seam-2/3 routed at `Modern` (A2's client) | not demoted by A1 | `register_storage*_global` + `onstorage` row (routed, `Modern`) | n/a (A2 flips) | n/a |
| WHATWG HTML §3.1.4 (`document.cookie`) | seam-1 cookie accessor routed at `Modern` (A3's client) | not demoted by A1 | `DOCUMENT_COOKIE_RW_ACCESSOR` install (routed, `Modern`) | n/a (A3 flips) | n/a |

**Breadth**: K=2 design specs (14, 12) + 2 named HTML contracts (the routed-at-`Modern`
clients); the *observable* surface touched = **0 web-platform APIs** (A1 marks nothing real
`Legacy` — every routed site installs at `Modern`). This is a mechanism PR → single-PR scope
is correct (the umbrella-approved base-case slice for the gate). A2/A3/B0/B1/E0 each re-run
their own coverage map at their plan time and own the *classification* (the `Legacy` flip).

> **Threading precedent (not a spec row):** the `EngineMode` construction-param → VM-field →
> installer-read path reuses the existing `global_scope_kind` realm-gating idiom
> (`init.rs`, `mod.rs`, `globals.rs` — §1). It carries no `§`-number so it is documented here.

**User-input flow**: none. A1 introduces no new untrusted-input parse and no new value
sanitization site. The `EngineMode` value is **embedder-supplied at construction** (trusted
host config, like `global_scope_kind`), not page-controlled.

---

## §0. Decisions this memo commits to (option-A general gate)

These realize A0 §3.2 ("a level at every install seam + one policy") through the
philosophy lenses (One-issue-one-way + Ideal-over-pragmatic). They are *recommendations to
the plan-review*, surfaced as Open Questions in §7 for the 5-agent gate to falsify.

1. **One general predicate, no family-named helper (F9).** Drop
   `VmInner::installs_web_storage()`. Every demotable install site consults the single
   family-neutral predicate `SpecLevelPolicy::installs(WebApiSpecLevel)` /
   `installs_dom(DomSpecLevel)` (already on `elidex-plugin`), via thin VM forwarders
   `VmInner::installs(level)` / `installs_dom(level)`. The predicate names no API family —
   so cookie (A3) and live-collections (B) reuse the **same** predicate, never a new
   `installs_cookie()` / `installs_live_collection()`. *One issue, one way.*

2. **Carrier = inline level-guard reading a per-family classification source (A0 §2.1 +
   Codex R7 refinement).** Each demotable install seam is `if self.installs(<family>_spec_level())
   { install_*(proto, TABLE) }` — the general predicate (the call-site-wrap idiom A0 §2.1
   blessed, mirroring `global_scope_kind`; *not* a per-row tuple, *not* an `install_*_leveled`
   helper set) reading the **single classification source for that API's family**:
   `web_storage_spec_level()` (accessors + `Storage`/`StorageEvent` globals + `onstorage`),
   `document_cookie_spec_level()`, `live_collection_spec_level()` (the `Document` getters +,
   downstream, `forms`/`images`/`links`/`children`/…). **Codex R7 correction:** the first
   redesign put an independent `Modern` *literal* at each site; for a family that spans several
   install surfaces that means N literals to flip in lockstep — a missed one leaves a split
   surface (`StorageEvent` without `localStorage`, the live-collection getters gone but
   `forms`/`images`/`links` still live). So the level lives in **one source per family**; A2/A3/B
   demote a whole family by flipping **one source**, and every surface of that family (including
   ones a downstream PR adds) routes through the same source. *Enforcement stays the one general
   `installs`/`installs_dom` predicate (F9); classification is per-family-source (R7) — the two
   are complementary, not the storage-specific predicate F9 rejected.* Leveled-helper variants
   stay rejected (parallel helper zoo for the demotable minority).

3. **All four install seams routed, with a real `Modern` caller each (F8/F9/F11).** The gate
   spans four seam *kinds*; A1 wires **every demotable-destined site it can identify** at
   `Modern` so the routing is non-dead and downstream is level-flip-only:
   - **seam-1 (method/accessor tables):** storage accessors (`window.rs`), **`document.cookie`**
     (extracted into its own 1-row accessor sub-table — A3 target), **Document
     live-collection getters** (extracted from `DOCUMENT_METHODS` — B target).
   - **seam-2 (direct `register_*_global`):** `Storage` + `StorageEvent` globals (`globals.rs`).
   - **seam-3 (event-handler IDL attrs):** **`window.onstorage`** — gated *within* the shared
     `install_handler_attr_family` loop by a per-attr level (F11) — wired now, at `Modern`.
   - **seam-4 (`DomApiHandler` registry):** `create_dom_registry_with_policy` (already built),
     reframed as **dispatch-level defense-in-depth**, not the property-absence lever (D4/F8).

4. **A1 classifies NOTHING `Legacy` — pre-judging no membership (B0-ownership preserved).**
   Every routed site stays `Modern`/`Living`. For the live-collection getters A1 only
   *routes-at-`Modern`* the currently-installed `Document` getters; it asserts **neither**
   that they are `Legacy` **nor** that they are the complete family. B0 owns the
   classification (the `Legacy` flip) **and** the full-family sweep (Element.prototype /
   `table.rows` / `form.elements` / … sites outside A1's view) per A0 §1.3. A1 makes the
   gate *capable*; B0 decides + completes.

   **"Every demotable site A1 can identify" is binding, not optional (closes the F9
   anti-pattern at the root).** `document.cookie` is a demotable-destined site A1 *can*
   identify (A3's target) — so A1 routes it **now**, at `Modern`. Leaving it un-routed
   "for A3" would make A3 perform a structural `document.rs` sub-table extraction rather
   than the promised one-literal flip — i.e. exactly the "sibling install site left
   un-routed → downstream grows its own structural edit" shape that R6/F9 rejected the
   first A1 for. Routing cookie now is therefore *required by §0.3*, not a scope choice
   (§7 Q2 confirms the route-at-`Modern`/classify-nothing **boundary**, no longer asks
   whether to defer).

5. **F10 — the non-compat modes are unreachable for a real session (cfg, not doc).** A doc
   warning does not enforce the §14.4.3 precondition. `Vm::new_with_mode` /
   `ElidexJsEngine::new_with_mode` are `#[cfg(test)]`-gated (production embedders cannot name
   `BrowserCore`/`App`; the only callers today are tests; the async-core PR un-gates them).
   `new()` (BrowserCompat) stays the sole public constructor — zero behavior change. (§3.5.)

6. **`compat-webapi` = compile-time hard ceiling above the runtime mode (F5, kept).** Declared
   independent of `engine` (additive). When off (the app profile), the derived policy is
   lowered once at construction via `SpecLevelPolicy::with_legacy_excluded()`, so every seam
   inherits Legacy-exclusion with no per-seam `cfg`. A1 does **not** cfg-gate the storage
   backend (that is A2). Latent today (A1 marks nothing `Legacy`). (§2.3.)

7. **Layering + ECS-native check (consolidated, F5/F6).** Crate homes by responsibility: the
   general predicate + `EngineMode`/`SpecLevelPolicy` → **`elidex-plugin`** (foundational,
   zero `elidex-*` deps → no inversion); the per-attr level classifier
   `event_handler_attr_spec_level` → **`elidex-script-session`** beside
   `event_handler_attr_event_type` (engine-independent SoT, no `ObjectId`/marshalling leak);
   the policy registry-filter → **`elidex-dom-api`** (where `dyn DomApiHandler`/`spec_level`
   are visible — *not* `PluginRegistry`); the install-guards → **`vm/host`** (engine-bound
   install plumbing only — no DOM algorithm; the live-collection *walkers* stay untouched in
   their native bodies). **ECS-native:** the gate state is **whole-VM construction config**
   (`spec_level_policy` field, derived once, never mutated, not entity-keyed) — the "shared
   cross-cutting state" exception (CLAUDE.md side-store rule (b)), **not** a per-entity
   side-store that should be an ECS component; the predicate is a flat `match level` fn,
   **not** an OO observer/subscriber registry.

---

## §1. Verified anchors (re-grep at impl-open — Q7 LIVE)

Re-grepped at PR-#376 HEAD `87e33a0d` (the redesign re-implements on top of the landed A1,
so the as-built A1 sites are the new baseline). **The implementer re-greps at open-time**
(Q7) — line numbers drift across the #372 rebase + A1's own additions; navigate by symbol.

| Symbol / site | Verified location (87e33a0d) | Role in the redesign |
|---|---|---|
| `WebApiSpecLevel {Modern,Legacy,Deprecated}` / `DomSpecLevel {Living,Legacy,Deprecated}` | `elidex-plugin/src/spec_level.rs:68` / `:25` | classification vocabulary (unchanged) |
| `EngineMode` / `SpecLevelPolicy` + `installs` / `installs_dom` / `with_legacy_excluded` | `spec_level.rs:94` / `:136` / `:145` / `:155` / `:171` | **the general predicate already exists** — D1 only removes the family-named VM wrapper |
| `VmInner::installs_web_storage` | `vm/globals.rs:1059` | **DELETE (D1)** → replace with `installs(level)` / `installs_dom(level)` forwarders |
| storage-accessor guard (seam-1) | `vm/host/window.rs:465` `if self.installs_web_storage() { install_ro_accessors(.., WINDOW_STORAGE_ACCESSORS) }` | re-express via `self.installs(Modern)` |
| `Storage` / `StorageEvent` global guards (seam-2) | `vm/globals.rs:489` / `:670` | re-express via `self.installs(Modern)` |
| `DOCUMENT_METHODS` (live-collection getters rows) | `vm/host/document.rs:1017` (`getElementsByTagName` `:1021`, `getElementsByClassName` `:1025`, `getElementsByName` `:1029`) | **extract** `DOCUMENT_LIVE_COLLECTION_METHODS` sub-table; gate install at `Modern` (seam-1) |
| `DOCUMENT_RW_ACCESSORS` (cookie row + `title`) | `vm/host/document.rs:1101`; install `:989` | **extract** `DOCUMENT_COOKIE_RW_ACCESSOR` 1-row; gate install at `Modern` (seam-1) |
| `install_handler_attr_family` loop over `EVENT_HANDLER_ATTRS` | `vm/host/event_handler_attrs.rs:215` (loop `:222`) | **F11** — add per-attr level; gate `onstorage` row at `Modern` (seam-3) |
| `EVENT_HANDLER_ATTRS` (incl. `onstorage`) + `event_handler_attr_event_type` SoT | `elidex-script-session` (imported `event_handler_attrs.rs:50`) | add sibling `event_handler_attr_spec_level(attr) -> WebApiSpecLevel` (Modern default; A2 flips onstorage) |
| `create_dom_registry_with_policy` (seam-4) | `elidex-dom-api/src/registry.rs:51` (gate `:57`) | **keep** — reframe as defense-in-depth (D4/F8); doc the lever hierarchy |
| `Vm::new_with_mode` / `new_with_scope(engine_mode)` | `vm/init.rs:42` / `:132` | **F10** — `#[cfg(test)]`-gate `new_with_mode` (Vm + Engine) |
| `ElidexJsEngine::new` / `new_with_mode` | `engine.rs:41` / `:58` | `new()` public (BrowserCompat); `new_with_mode` `#[cfg(test)]` |
| `spec_level_policy` field on `VmInner` (set in `new_with_scope`, before `register_globals`) | `vm/mod.rs:2568`; set `init.rs:146`/`:154`/`:794` | the F4 write-before-install invariant — single shared ctor, so worker/SW/wasm inherit (D6) |
| worker / SW / wasm mode propagation | `vm/host/worker.rs`, `vm/sw_thread.rs`, `vm/host/wasm/mod.rs:83` (R1/R5) | **F1/F7 kept** — verify the general predicate threads (D6) |

---

## §2. Design decisions (detail)

### 2.1 Carrier form — inline level-guard at the call site (vs. per-row tuple / leveled helper)

The install helpers take flat tables `&[(&str, NativeFn)]`. Three carrier shapes:

- **(rejected) Per-row level** — widen every tuple to `(&str, NativeFn, WebApiSpecLevel)`.
  Forces editing every `WINDOW_*`/`DOCUMENT_*` table to append `Modern` — pure noise for the
  ~95% Modern surface (A0 §3.2a).
- **(rejected) Leveled install helper** — a parallel `install_*_leveled(table, level)` set.
  The demotable *minority* would call them while the Modern 95% keeps the bare helpers, so
  the gate is **not** uniformly inside one install path — the helper buys no forget-safety
  the inline guard lacks, and adds a parallel helper zoo. (The first A1 dropped these as dead
  code; the redesign does **not** restore them — the inline guard is the cleaner realization
  of the *same* "route every demotable site through the one predicate" goal. Q1.)
- **(chosen) Inline level-guard at the call site** — `if self.installs(level) {
  install_*(proto, TABLE) }`, level literal at the site. Identical in shape to the existing
  realm gate (`if matches!(self.global_scope_kind, Window) { … }`). A2/A3/B flip the literal.

**Why the demotable APIs need sub-table extraction.** Storage accessors already live in their
own `WINDOW_STORAGE_ACCESSORS` const, so one guard gates the whole call. But `document.cookie`
shares `DOCUMENT_RW_ACCESSORS` with `title` (Modern), and the live-collection getters share
`DOCUMENT_METHODS` with `querySelector` (Modern). A single call-site guard can only gate a
**homogeneous** install call — so A1 **extracts** the demotable rows into their own
sub-tables (`DOCUMENT_COOKIE_RW_ACCESSOR`, `DOCUMENT_LIVE_COLLECTION_METHODS`) whose install
call is then guarded. This is the table-seam analogue of why storage is already a separate
const, and is the structural enabler of the "pure level-flip" downstream story. The
event-handler seam (seam-3) is the one exception that *cannot* extract — `onstorage` is one
row of a shared **family loop** over heterogeneous attrs — so there the level is a **per-attr
classification lookup** inside the loop (§3.3 seam-3), the principled carrier for a
heterogeneous loop. (Asymmetry is intentional: homogeneous sub-table ⇒ call-site literal;
heterogeneous family loop ⇒ per-row lookup.)

### 2.2 `EngineMode` placement + threading (unchanged from the first A1 — review-confirmed)

`EngineMode {BrowserCompat, BrowserCore, App}` + `SpecLevelPolicy` + the derive fn live in
`elidex-plugin` next to the enums (zero `elidex-*` deps → no inversion). Threaded as a
**construction param** mirroring `global_scope_kind`: `new_with_scope` takes the mode, the VM
stores the derived `spec_level_policy` field, `register_globals` (called last in the ctor,
*after* the field is set — the F4 write-before-install invariant) reads it via the seam
guards. One authority feeds Web-API + style + DOM policies (whole-engine consistency, A0
R3-6). Not `bind_session` (installers run at construction; a bind-time mode could not
*prevent* an install — it would need a removal path = strangler).

### 2.3 `compat-webapi` feature (unchanged — F5 kept)

Independent cargo feature (NOT `engine`-implied — additive semantics, A0 R5-1). The compile
ceiling is applied once at construction (`init.rs`): `compat-webapi` off ⇒ the derived policy
is lowered via `with_legacy_excluded()`, so every seam inherits Legacy-exclusion with no
per-seam `cfg`. A1 does **not** cfg-gate `WebStorageManager`/`SessionStorageState` or the
`elidex-storage-core` dep (that is A2 — `storage-core` is shared by Cache/SW). One mechanism,
two faces: cfg = binary presence (hard ceiling), `EngineMode` = per-session reachability
(soft selector within what is linked); ordered, never contradictory.

---

## §3. Mechanism — the four seams (general gate)

### 3.1 Why four (A0 §3.2a)

A table-only gate is incomplete: legacy top-level globals (`StorageEvent`, future
`XMLHttpRequest`) install via direct `register_*_global()`, `onstorage` installs via the
event-handler-attr loop, and bridge-dispatched DOM methods resolve through the
`DomApiHandler` registry. One predicate must reach all four, else A2/A3/B each grow a one-off
gate (the "new seam + N legacy" anti-pattern). The first A1 reached only the storage subset
of seams 1/2 — F9's "incomplete." The redesign reaches all four with a real `Modern` caller.

### 3.2 The single authority

`EngineMode` (construction param) → `SpecLevelPolicy` (derived once, stored on the VM). Every
seam asks the *same* family-neutral predicate `policy.installs(level)` / `installs_dom(level)`.
`BrowserCompat` ⇒ installs `Modern + Legacy` (current behavior). `BrowserCore`/`App` ⇒
`Modern`/`Living` only (⚠ not selectable for a real session until
`#11-async-core-storage-cookiestore` — §3.5; A1 exercises them by unit test only).

### 3.3 Seam-by-seam (all routed at `Modern`; A1 classifies nothing)

| # | Seam | A1 wiring (level = `Modern`/`Living`, no behavior change) | Downstream flip |
|---|---|---|---|
| 1a | Storage accessors | `window.rs`: `if self.installs(Modern) { install_ro_accessors(proto, WINDOW_STORAGE_ACCESSORS) }` (de-storage-specific the existing guard) | A2 → `Legacy` |
| 1b | **`document.cookie`** | `document.rs`: **extract** `DOCUMENT_COOKIE_RW_ACCESSOR` (the cookie row) from `DOCUMENT_RW_ACCESSORS`; install the remaining (`title`) unconditionally, the cookie sub-table under `if self.installs(Modern) { … }` | **A3** → `Legacy` (one literal) |
| 1c | **Document live-collections** | `document.rs`: **extract** `DOCUMENT_LIVE_COLLECTION_METHODS` (`getElementsByTagName`/`getElementsByClassName`/`getElementsByName`) from `DOCUMENT_METHODS`; install the rest unconditionally, the sub-table under `if self.installs_dom(Living) { … }`. **Classifies nothing** — stays `Living`; B0 owns the `Legacy` decision + the full-family sweep (§0.4). **Spec-home note (F3):** the three share a "live collection" *shape* but **two spec homes** — `getElementsByTagName`/`getElementsByClassName` = DOM §4.5, **`getElementsByName` = HTML §3.1.7** (DOM tree accessors) — so B0 must cite each to its own home (the code-symbol grouping here implies no single §; do not let it inherit a DOM §4.5 cite for `getElementsByName`) | **B0/B1** → `Legacy` + sweep |
| 2 | Direct `register_*_global` | `globals.rs`: `if self.installs(Modern) { register_storage_global() }` + same for `register_storage_event_global()` (de-storage-specific). Permanently-`Modern` globals (crypto/fetch/ws) are **not** gated (never excluded — gating them is churn, not One-issue-one-way) | A2 → `Legacy` |
| 3 | **Event-handler IDL attrs (`onstorage`, F11)** | `event_handler_attrs.rs`: in `install_handler_attr_family`'s `EVENT_HANDLER_ATTRS` loop, look up each attr's level via a new SoT sibling `event_handler_attr_spec_level(attr) -> WebApiSpecLevel` and `if self.installs(level) { install_bound_accessor_pair(…) }`. The sibling must be **total over `EVENT_HANDLER_ATTRS`** exactly like the existing `event_handler_attr_event_type` (which `.expect()`s a known row) — A1 returns `Modern` for every attr, but a future attr added to the family table without a level arm must **not** silently fall through to an unintended default (match-exhaustive, or an explicit `Modern` total-default mirroring the sibling's totality). Wires `onstorage` (+ every handler attr) at `Modern` now (F2) | **A2** flips `onstorage` → `Legacy` in the SoT lookup (one row) |
| 4 | `DomApiHandler` registry (defense-in-depth, F8/D4) | `create_dom_registry_with_policy` (kept) withholds `Legacy` handlers at registration. **Reframed:** this is **dispatch-level** enforcement (a withheld handler ⇒ a leaked property's call fails cleanly), **not** the property-absence lever — that is seam-1 (the table install). The live-collection getters have **no** registry handler (they alloc directly in `document.rs`), so seam-1c is their *only* lever; seam-4 covers *future* bridge-dispatched `Legacy` handlers (none today). Gates the **static built-in** set only; the registry's `register_dynamic` path stays un-gated by-construction (slot **`#11-dom-registry-dynamic-policy-gate`**, F4) | B1 (when a bridge-dispatched method demotes) |

**Uniformity check**: all four seams end at the **one** predicate `installs`/`installs_dom`.
No seam grows a bespoke `if mode == …` branch and no second family-named helper exists. A2's
storage demotion is: flip seam-1a + seam-2 + seam-3-onstorage literals to `Legacy` (the whole
Web Storage family in lockstep). A3: flip seam-1b. B0/B1: flip seam-1c (+ sweep + seam-4 for
any bridge-dispatched member). *One issue, one way.*

### 3.4 The absence-lever hierarchy (F8, made explicit)

For a JS-visible API the **install seam is the absence lever**: not installing the property /
global / accessor makes the API *absent* (`typeof X === 'undefined'`, the spec-correct
"unsupported" observation). The `DomApiHandler` registry is **downstream of dispatch**:
withholding a handler makes a *present* property's call throw — useful as defense-in-depth
(a leaked Legacy property fails cleanly rather than mis-executing) but **not** a substitute
for not installing it. So every demotable DOM method is gated at its **install** seam
(seam-1c for the live-collection getters; a future bridge-dispatched method at its property
table) **and** optionally at seam-4. A1 documents this hierarchy on
`create_dom_registry_with_policy` and at the seam-1c install.

### 3.5 F10 — mode constructor gating (cfg, not doc)

`Vm::new_with_mode` and `ElidexJsEngine::new_with_mode` are `#[cfg(test)]` — production
embedders cannot name `BrowserCore`/`App`. Verified: the only callers today are
`elidex-js` in-crate tests (production uses `new()` = BrowserCompat; `new_with_mode(BrowserCompat)`
would be redundant with `new()`). The `#[cfg(test)]` gate is per-crate and all mode-selecting
construction is in-crate, so it suffices; the `#11-async-core-storage-cookiestore` PR removes
the gate when a real session may select a non-compat mode. The mode-enum doc still carries the
§14.4.3 precondition warning — but the **cfg is the enforcement**, the doc is documentation.

### 3.6 F1/F7 — realm inheritance (kept, verify the general predicate threads)

The mode is *engine-wide*: a `BrowserCore`/`App` document's dedicated/service workers (R1)
and the Wasm DOM registry (R5) inherit it rather than reset to `BrowserCompat`. These fixes
are **kept**. Because the general predicate reads `self.spec_level_policy`, set in the single
shared `new_with_scope`, every child-realm VM (built through the same ctor with the threaded
mode) automatically uses the general predicate — D1 changes the *predicate name*, not the
threading. **AC: a worker/SW/wasm child built under `BrowserCore` withholds a marked-`Legacy`
test API at every seam, same as the Window realm** (extend the R1/R5 tests to the general
predicate).

---

## §4. File-level change plan (for the re-implementation commit)

Re-implementation **on top of** the landed A1 (`87e33a0d`), so most edits are *surgical
generalizations* of the storage-specific wiring + the new pre-wired sites.

1. **`elidex-plugin/src/spec_level.rs`** — no type change needed (predicate already general);
   confirm doc comments describe the general (not storage) usage.
2. **`vm/globals.rs`** — **delete** `installs_web_storage()`; add family-neutral
   `installs(WebApiSpecLevel) -> bool` + `installs_dom(DomSpecLevel) -> bool` forwarders;
   re-express the seam-2 guards (`register_storage_global` / `register_storage_event_global`)
   via `self.installs(Modern)`.
3. **`vm/host/window.rs`** — re-express the seam-1a storage-accessor guard via `self.installs(Modern)`.
4. **`vm/host/document.rs`** — **extract** `DOCUMENT_COOKIE_RW_ACCESSOR` (seam-1b) +
   `DOCUMENT_LIVE_COLLECTION_METHODS` (seam-1c) from their shared tables; gate each install
   at `Modern`/`Living`; install the Modern remainder unconditionally. Doc the seam-1c
   absence-lever note (F8/D4).
5. **`vm/host/event_handler_attrs.rs`** + **`elidex-script-session`** — add
   `event_handler_attr_spec_level(attr) -> WebApiSpecLevel` SoT sibling, **total over
   `EVENT_HANDLER_ATTRS`** (match-exhaustive or explicit `Modern` total-default, mirroring
   `event_handler_attr_event_type`'s totality — F2; no silent fall-through when the family
   table grows); A1 returns `Modern` for all. Gate the `install_handler_attr_family` loop
   per-attr via `self.installs(level)` (F11/seam-3).
6. **`elidex-dom-api/src/registry.rs`** — keep `create_dom_registry_with_policy`; reframe the
   doc comment to the **defense-in-depth** role + the absence-lever hierarchy (F8/D4).
7. **`vm/init.rs`** + **`engine.rs`** — `#[cfg(test)]` on `Vm::new_with_mode` +
   `ElidexJsEngine::new_with_mode` (F10/D5); `new()` stays public.
8. **Tests** — §5. Extend `tests_webapi_gate` + the registry tests to the general predicate
   and to **all four seams** (a marked-`Legacy` test API per seam withheld under
   `BrowserCore`); extend the R1/R5 realm tests to the general predicate.

Files A1 does **not** touch: `storage.rs` / `navigator.rs` bodies (A2/A3), the shell
(supplies `BrowserCompat`), `elidex-script-session/src/engine.rs` (seam-4 pinned to
`elidex-dom-api`).

---

## §5. Testing / Acceptance criteria

1. **No behavior change under `BrowserCompat`** — existing VM/engine suites pass unchanged
   (default mode installs Modern + Legacy as today). Primary regression guard.
2. **General predicate + end-to-end VM-seam exclusion (F9).** Proven at three levels:
   (i) the **predicate matrix** (`installs`/`installs_dom` × modes) — the one family-neutral
   predicate (no `installs_web_storage`) every seam shares; (ii) **two concrete VM-seam
   end-to-end exclusions** — a mock `Legacy` `DomApiHandler` withheld at the **seam-4 registry**
   (`elidex-dom-api`), and a test-only `Legacy`-classified **direct-global probe**
   (`register_globals`, `legacy_probe_withheld_in_core_modes`) withheld under
   `BrowserCore`/`App` (the seam-2 shape — closes F9's "direct table/global installs" doubt the
   storage-specific first A1 left open); (iii) **behavior-preservation** (next item). End-to-end
   exclusion at the table/accessor/handler-attr seams (1a/1b/1c/3) lands when A2/A3/B mark a
   *real* API `Legacy` (one literal flip) — A1 proves the mechanism, not a premature demotion.
3. **Behavior-preservation of the rewired/extracted seams (no premature demotion).** A1
   classifies nothing `Legacy`, so the storage accessors + `onstorage`
   (`rewired_window_seams_present_in_all_modes`), `document.cookie`, and the live-collection
   getters are **present in all modes**; the cookie/live-collection sub-table extraction is
   behavior-preserving (same properties, `[SameObject]`, order-observable shape — guarded by the
   broad elidex-js DOM suite, which regresses if the extraction drops a property).
4. **F10 — non-compat modes are not production-selectable.** `new_with_mode` is `#[cfg(test)]`;
   a non-test build exposes only `new()` (BrowserCompat). Asserted by the shell default
   unchanged + the cfg gate compiling.
5. **F11 — `onstorage` routed.** `onstorage` installs at `Modern` in every mode
   (`rewired_window_seams_present_in_all_modes`); the per-attr `event_handler_attr_spec_level`
   is total over `EVENT_HANDLER_ATTRS`, so A2 hides `onstorage` by one SoT-lookup flip
   (its end-to-end exclusion is A2's test).
6. **F1/F7 — realm inheritance via the general predicate.** A worker/SW/wasm child under
   `BrowserCore`/`App` derives a Legacy-excluding policy (`worker_realms_inherit_engine_mode`),
   so the general predicate threads identically in child realms.
7. **F8 — absence-lever.** The **install seam** (table/global) is the property-absence lever:
   the direct-global probe is *absent* under exclusion (`get_global` → `None`), not a present
   property whose call throws. The seam-4 registry is dispatch-level defense-in-depth. (When B0
   marks the live-collection getters `Legacy`, their seam-1c *property* goes absent the same
   way — A1 makes the seam capable; B0 flips.)
8. **`compat-webapi`** declared independent of `engine`; both profiles compile (browser =
   `engine`+`compat-webapi`; a profile with `engine` alone). A1 does **not** require the
   storage backend to drop under `engine`-alone (that is A2).
9. **Write-before-install ordering (F4)** — the field-set precedes `register_globals`; the
   seam-exclusion test is the live guard (a Legacy API observed absent under `BrowserCore` ⇒
   the policy was set before the installer ran).

`mise run ci` green; `cargo test -p elidex-plugin -p elidex-js -p elidex-dom-api -p
elidex-script-session --all-features`.

---

## §6. Collision / sequencing

- **#372 (media Slice 2b-ii) MERGED** (`d8858d67`); the landed A1 is rebased onto it, the
  redesign builds on the landed A1 — no live conflict. The redesign edits `window.rs`
  (seam-1a), `document.rs` (seam-1b/1c — **new** vs. the first A1), `globals.rs`,
  `event_handler_attrs.rs`, `init.rs`, `engine.rs`, `elidex-dom-api`, `elidex-plugin`.
  Confirm at open-time no other in-flight PR edits `document.rs`'s `DOCUMENT_METHODS` /
  `DOCUMENT_RW_ACCESSORS` (B0/B1 not yet open; A2/A3 gated behind this A1).
- **`#11-async-core-storage-cookiestore` precondition** — enforced now by the `#[cfg(test)]`
  gate (F10/D5), not only the doc.
- **Worktree isolation** — re-implementation in the existing dedicated worktree
  `/Users/kazuaki/repos/send.sh/elidex-webapi-a1` (branch `webapi-compat-a1`).

---

## §7. Open questions for `/elidex-plan-review` (re-review)

1. **Carrier: inline guard vs. leveled helper (§2.1, §0.2).** The redesign uses an inline
   level-guard at each demotable call site (idiom-faithful to A0 §2.1) and **rejects** the
   leveled-helper variant. Confirm the inline guard is not under-protecting against a future
   un-guarded demotable installer (forget-safety), given the demotable set is closed + small
   and BrowserCore/App are not production-selectable.
2. **A1 pre-wiring cookie + live-collections — boundary confirm (§0.3/§0.4/§3.3).** A1
   extracts the cookie + Document-live-collection sub-tables and routes them at `Modern` —
   structural edits in `document.rs` whose *classification* is A3's / B0's. **Resolved
   (plan-review F1):** routing **both** now is *required* by §0.4 (every identifiable
   demotable site routed) — deferring cookie to A3 would re-introduce the F9 "sibling site
   un-routed → downstream structural edit" anti-pattern the redesign exists to kill. The
   remaining question for the gate is only the **boundary correctness**: that
   route-at-`Modern` + classify-**nothing**-`Legacy` (A1) cleanly preserves A2/A3/B0's
   ownership of the `Legacy` flip + (for B0) the full-family sweep — *not* whether to defer.
3. **Seam-3 carrier asymmetry (§2.1, §3.3 seam-3).** Homogeneous sub-tables carry the level
   as a call-site literal; the heterogeneous `EVENT_HANDLER_ATTRS` family loop carries it as
   a per-attr SoT lookup (`event_handler_attr_spec_level`). Confirm the asymmetry is
   principled (not two ways to gate) and that the SoT lookup is the right home (beside
   `event_handler_attr_event_type`).
4. **F8 lever hierarchy (§3.4).** Confirm seam-4 (registry-withholding) as documented
   defense-in-depth + seam-1 as the property-absence lever is correct, and that keeping the
   already-built `create_dom_registry_with_policy` (vs. removing it) is right (not dead).
5. **F10 cfg gate (§3.5).** Confirm `#[cfg(test)]` on `new_with_mode` is the right enforcement
   (vs. a runtime assert / a `compat` feature gate), given all mode-selecting construction is
   in-crate today.
6. **B0-ownership (§0.4).** Confirm A1 routing the live-collection getters at `Living` while
   asserting **no** classification (and **no** full-family sweep) correctly preserves B0's
   ownership of the `Legacy` decision + the cross-table sweep.
7. **Re-grep discipline (Axis 5) — LIVE.** Re-grep §1 at impl-open (line numbers drift on the
   landed A1); navigate by symbol.

---

## §8. Citation appendix (webref-verified)

| Concept | Source | Anchor |
|---|---|---|
| Web API core/compat boundary | design `14-script-engines-webapi.md` §14.4.2 | (in-repo) |
| Engine-mode storage contract (core/app ⇒ async, no sync) | design §14.4.3 | (in-repo) |
| DOM core/compat | design `12-dom-cssom.md` §12.1.2 | (in-repo) |
| Web Storage / `StorageEvent` (A2's client) | WHATWG HTML §12.2 / §12.2.4 | `#storage` / `#the-storageevent-interface` |
| `document.cookie` (A3's client) | WHATWG HTML §3.1.4 | `#dom-document-cookie` |
| `getElementsByClassName` (live, B's client) | DOM LS §4.5 | `#dom-document-getelementsbyclassname` |

> A1 cites no new WHATWG algorithm prose (it implements no algorithm). The HTML/DOM rows are
> the downstream clients of the seams A1 routes, named for traceability. Anchors carried
> verbatim from A0 §7 (webref-verified there).
