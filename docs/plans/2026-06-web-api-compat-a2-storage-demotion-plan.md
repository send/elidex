# A2 — Web Storage Surface Demotion (plan-memo)

Plan date: 2026-06-21 JST
Program: `docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md` → Program A,
PR A2. Parent design (SSoT for boundary + mechanism): A0 =
`docs/plans/2026-06-web-api-compat-split-design.md` (Program-A table, §5 row **A2**).
A1 (gate mechanism) ✅ MERGED [#376](https://github.com/send/elidex/pull/376)
(`f4d76c4f`); A2 is the first **classification** PR — it flips the Web Storage
family from `Modern` to `Legacy` at A1's single source and finishes the
`compat-webapi` code-gating A1 deliberately left open.

> **Edge-dense → `/elidex-plan-review` REQUIRED before implementation** (CLAUDE.md;
> A0 §5 marks A2 **PR-R**). This memo is the design gate. It is a **base-case slice
> under the approved A0 umbrella** (per CLAUDE.md "Edge-dense work" base case), so
> it is an allowed single PR — but it touches identity-of-surface × realm-scope ×
> feature-gating × cross-process delivery, so it gets its own plan-review.
>
> **Premise-correction discipline (CLAUDE.md Axis-5, A0 §8 Q7):** every file:line
> below was **re-grepped against `main` at HEAD `f4d76c4f` (2026-06-21)** — A0's
> anchors were a snapshot at `2f4a9d5a` (2026-06-20), *before* A1 landed and
> reshaped the install seams. §1 records the deltas; the most consequential are
> (i) A1 has **already pre-wired every storage install seam** behind
> `installs(web_storage_spec_level())`, so A2's install-exclusion work collapses to
> a **one-line source flip**; and (ii) the shell does **not** run the VM yet (boa
> still drives script), which materially changes the A0 "shell tab/IPC plumbing"
> scope (§7 Q1).

---

## §0. Premise-correction — A0 (2f4a9d5a) vs. HEAD (f4d76c4f)

What A1 changed, that A0's A2 row did not yet know:

1. **All four install seams are already gated, at `Modern`.** A1 pre-wired the
   demotable storage sites with an inline `if self.installs(level)` guard reading
   the family's **single source** `web_storage_spec_level()`
   (`elidex-script-session/src/event_handler_consumer.rs:235`, currently returns
   `WebApiSpecLevel::Modern`):
   - seam-1 (accessors): `localStorage`/`sessionStorage` install in
     `register_window_prototype` — `vm/host/window.rs:467`
     `if self.installs(web_storage_spec_level())`.
   - seam-2 (`Storage` global): `vm/globals.rs:506-507`
     `if self.installs(web_storage_spec_level()) { self.register_storage_global(); }`.
   - seam-2 (`StorageEvent` global): `vm/globals.rs:687-688`, same guard.
   - seam-3 (`onstorage` handler attr): `event_handler_consumer.rs:285`
     `"onstorage" => web_storage_spec_level()`.
   - seam-4 (`DomApiHandler` registry): N/A for storage (storage is not
     registry-dispatched; it is host-native glue).

   ⟹ **A2's install-time exclusion is a single source flip** `Modern → Legacy`
   in `web_storage_spec_level()`. All four seams follow from the one source by
   construction (this is exactly the "pure one-source level-flip" A1 was built to
   enable). No per-seam edits for the *exclusion* itself.

2. **Media Slice 2b landed → the A0 §6 `window.rs` collision is cleared.**
   `matchMedia` installs at `vm/host/window.rs:523`
   (`super::media_query::native_window_match_media`; `:521-522` are its
   CSSOM-View doc-comment). A0 §6 said "do not open A2
   while Slice 2b is open"; Slice 2b is merged (#370/#372), so A2 may proceed on
   `window.rs`.

3. **The shell does not run the VM yet.** `elidex-js/Cargo.toml:23-24`:
   *"No such embedder wires `elidex-js` yet — the shell still runs the boa engine
   (S5 cutover pending)."* `shell/.../pipeline.rs:90` drives script via
   `ScriptEngine::eval` with **no `EngineMode`**, and the shell has **zero
   `EngineMode` concept** (repo-wide grep-negative outside `elidex-plugin` +
   `elidex-js`). The StorageEvent delivery path
   (`shell/.../content/mod.rs:563` `dispatch_storage_event` →
   `pipeline.dispatch_event`) is **engine-agnostic** (a ScriptSession boundary),
   not VM-specific. ⟹ the A0 A2 row's "shell tab/IPC mode plumbing" has **no live
   VM consumer and no excluded-mode session to suppress** today (§7 Q1).

4. **`compat-webapi` feature exists but the storage *code* is not yet cfg-gated.**
   `elidex-js/Cargo.toml:25` `compat-webapi = []` (independent of `engine`, A1).
   But `vm/host/storage.rs:56` and `vm/host_data.rs:24` import
   `elidex_storage_core::{WebStorageManager, SessionStorageState}`
   **unconditionally**, and `host_data.rs:295/311/328` hold those types as fields.
   A1's Cargo comment is explicit: *"A1 does NOT cfg-gate the storage backend
   (A2)."* ⟹ making an `engine`-without-`compat-webapi` build compile is **A2's**
   work.

5. **Realm-scope gap is real at HEAD.** `register_storage_global`
   (`globals.rs:507`) and `register_storage_event_global` (`globals.rs:688`) run in
   the **scope-common** region of `register_globals`, *outside* the
   `GlobalScopeKind::Window` branch (which begins at `globals.rs:528` / the
   `match` at `:562`). So worker/SW VMs over-expose `Storage`/`StorageEvent`. The
   *accessors* are already Window-only (they install inside
   `register_window_prototype`, called only in the Window arm `globals.rs:562-563`).

---

## §A. Spec coverage map (preflight hard-gate)

> "Touch" = the registration/classification site A2 edits at HEAD `f4d76c4f`.
> All §-numbers webref-verified (§8). "Full enum?" = is the demoted family closed.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| HTML §12.2 *The API* | classify Web Storage family `Legacy` | Modern→Legacy | `event_handler_consumer.rs:235` `web_storage_spec_level()` (single source) | ✓ (whole family) | yes (`setItem` value → `WebStorageManager`) |
| HTML §12.2.1 *The Storage interface* | gate + Window-scope `Storage` | Legacy, `[Exposed=Window]` | `globals.rs:507` `register_storage_global` (move into Window arm) | ✓ | yes |
| HTML §12.2.2 *sessionStorage getter* | gate `sessionStorage` accessor | Legacy | `window.rs:467` accessor install (already Window-only + gated) | yes | yes |
| HTML §12.2.3 *localStorage getter* | gate `localStorage` accessor | Legacy | `window.rs:467` (already Window-only + gated) | yes | yes |
| HTML §12.2.4 *The StorageEvent interface* | gate + Window-scope `StorageEvent` | Legacy, `[Exposed=Window]` | `globals.rs:688` `register_storage_event_global` (move into Window arm) | ✓ | no (ctor) |
| HTML §8.1.8.2 (`onstorage`, WindowEventHandlers) | gate `window.onstorage` | Legacy | `event_handler_consumer.rs:285` (already reads the source) | n/a | no |

**Breadth**: K=1 spec (html), M=6 entries (verified 2026-06-21 — table rows above) → single-PR scope (base-case slice under
the approved A0 umbrella). **User-input audit**: A2 introduces **no new untrusted
input path** — it only *gates* the existing `setItem`/`getItem` glue; the value
flow into `WebStorageManager` is unchanged (trust-boundary unchanged ⟹ no new
sanitize obligation).

### §A.1 Surface-completeness

The Web Storage family is **closed**: `Storage` + `localStorage` + `sessionStorage`
+ `StorageEvent` + `onstorage`. A0 §1.1/§1.2 verified no other Web-Storage member
is installed (no `document.write`-shaped legacy globals; `StorageManager`/
`navigator.storage` not present). cookie (A3) and live collections (B0) are
**separate** families on their own sources — out of A2.

---

## §1. Verified anchors (re-grep at impl-open — A0 §8 Q7 LIVE)

All confirmed via Read+grep at HEAD `f4d76c4f`:

- **Single source**: `event_handler_consumer.rs:235`
  `pub fn web_storage_spec_level() -> WebApiSpecLevel` → returns `Modern`
  (asserted `:454`). Re-exported `lib.rs:36`.
- **Seam guards**: `window.rs:467`, `globals.rs:506-507`, `globals.rs:687-688`,
  `event_handler_consumer.rs:285`.
- **EngineMode/policy**: `elidex-plugin/src/spec_level.rs` —
  `EngineMode {BrowserCompat,BrowserCore,App}` (`:94`),
  `spec_level_policy()` `:112` (`exclude_legacy = !BrowserCompat` `:114`).
  `installs` predicate `vm/globals.rs:1102`. VM construction supplies
  `BrowserCompat` (`vm/init.rs:31`); `new_with_mode` is `#[cfg(test)]` (`:49`, F10).
- **Realm branch**: `register_globals` `globals.rs:88`; Window-only guard
  `:528`; `match global_scope_kind` arms `:562` Window / `:592` DedicatedWorker /
  `:612` ServiceWorker.
- **Backend glue (to cfg-gate)**: `vm/host/storage.rs:56`
  `use elidex_storage_core::{StorageError, StorageErrorKind, WebStorageManager}`;
  `host_data.rs:24` import, fields `web_storage` `:295` / `session_storage` `:311`
  / `fallback_local_storage` `:328`; `install_web_storage` `:936`; ctor inits
  `:788/:790/:792`.
- **Cargo**: `compat-webapi = []` `Cargo.toml:25`; `engine = [… "elidex-storage-core" …]`
  `:26-…`; `elidex-storage-core = { workspace = true, optional = true }` `:133`.
- **Shell (NOT edited by A2 — see §7 Q1)**: broadcast `app/mod.rs:635`
  `BrowserToContent::StorageEvent`; IPC variant `ipc.rs:128`; engine-agnostic
  dispatch `content/mod.rs:563` `dispatch_storage_event`; consumer
  `content/event_loop.rs:356-362`.

---

## §2. Decisions this memo commits to

1. **Exclusion = one-source flip.** Flip `web_storage_spec_level()`
   `Modern → Legacy`. Under `BrowserCompat` (the only production mode, supplied at
   VM construction) `exclude_legacy = false` ⟹ **byte-identical Window surface**;
   under `BrowserCore`/`App` (test-only, §4.2 async-core precondition) all four
   seams drop together. No per-seam install edits (A1 already routed them).

2. **Realm-scope `[Exposed=Window]` correction (intended spec change, A0 R8-3).**
   Move `register_storage_global` and `register_storage_event_global` from the
   scope-common region into the `GlobalScopeKind::Window` arm so worker/SW VMs no
   longer expose `Storage`/`StorageEvent` (HTML §12.2.1/§12.2.4 `[Exposed=Window]`).
   This corrects an over-exposure that exists *even under `BrowserCompat`* — it is
   a spec-compliance fix, **not** a parity break.

3. **`compat-webapi` code-gating (finish A1's open item).** cfg-gate the
   elidex-js-resident Web-Storage **glue** under `feature = "compat-webapi"` so an
   `engine`-without-`compat-webapi` build (the future `App` profile) compiles
   without referencing `WebStorageManager`/`SessionStorageState`: the
   `vm/host/storage.rs` module, the `host_data.rs` storage fields +
   `install_web_storage` + their ctor inits + accessors, and the
   `register_storage_global`/`register_storage_event_global` fns + their call
   sites. **Per A0 R9-1: gate the Web-Storage *code*, NOT the `elidex-storage-core`
   crate dependency** — Cache API/SW import `SqliteConnection` from the same crate
   under `engine`; dropping the dep breaks them. (Whether `elidex-storage-core`
   *additionally* feature-gates `WebStorageManager`/`SessionStorageState` behind
   its own feature is §7 Q2 — the necessary-and-sufficient move for the
   elidex-js-absence requirement is gating the **glue**.)

4. **Shell event-delivery suppression → defer slot, NOT A2 code (diverges from A0
   A2 row R5-7/R8-6; see §7 Q1 for the full argument and recommendation).**

5. **No ECS-native concern** (detail in §2.6).

### 2.6 ECS-native check (Axis 2)

A2 introduces **no new ECS component, no new per-entity side-store, and no OO
pattern**. The two storage backends are CLAUDE.md side-store exception **(b)**
(shared cross-cutting state, correctly *not* components), for two distinct reasons:
`WebStorageManager` is an `Arc`-shared, origin-keyed **process resource**
(host_data.rs:287-289); `SessionStorageState`/`fallback_local_storage` are
**browsing-context-scoped per-VM** state held on `HostData` (per-VM, not
entity-keyed; cleared on `Vm::unbind`). Neither is an entity-keyed
`HashMap<entity,_>`/`*_cache`/`*Registry`, so the side-store→component rule does
not fire. The realm-scope move (§2.2) reuses existing VM-construction `match
global_scope_kind` gating — no new observer/registry/inheritance shape. (A0 §4.3
pre-clears this; Axis-2 plan-review confirmed against CLAUDE.md + code.) The only
integrity risks here are non-ECS: the install-ordering invariant (§4.2) and
`cfg(not(compat-webapi))` reader completeness (§7 Q3).

---

## §3. The shell-side scope question (the load-bearing design decision)

A0's A2 row states A2 "spans the VM **and** the shell tab/IPC mode plumbing"
because hiding the `StorageEvent` constructor/`onstorage` handler is not enough:
the shell broadcasts `BrowserToContent::StorageEvent` and the content loop
dispatches it "regardless of Web-API mode." **The premise-correction (§0.3)
changes the calculus**:

- The dispatch path (`content/mod.rs:563` → `pipeline.dispatch_event`) is the
  **engine-agnostic ScriptSession boundary**, and the shell carries **no
  `EngineMode`** anywhere today. Adding suppression therefore means **inventing
  per-tab/per-session `EngineMode` propagation through shell IPC** — infrastructure
  that does not exist.
- The **suppression target does not exist either**: `BrowserCore`/`App` are not
  production-selectable (A0 §4.2 — async-core precondition), and the shell runs
  **boa, not the VM**. So a mode-aware broadcast filter would, today, **always
  evaluate to "deliver"** — pure latent coupling with zero live consumer.
- The VM gate is **already internally coherent without it**: in an excluded mode
  the page cannot construct `StorageEvent`, cannot read `window.onstorage`, and has
  no `localStorage`/`sessionStorage` to mutate — so there is no excluded-mode
  session for an incoming broadcast to reach. The residual is a cross-process
  delivery filter for a session class that can't be selected.
- **Active collision**: a `shell-viewport-pr-b` worktree is in-flight editing
  `shell/app/` (the `App.placement` SoT). A2 touching `shell/app/mod.rs` now would
  collide with active work.

**Recommendation (philosophy-first):** A2 is **VM-side only**; register defer slot
`#11-storage-event-mode-aware-delivery` for the broadcast-suppression, to land with
the **mode-plumbing program** (when `EngineMode` first flows shell→content — i.e.
S5 VM cutover and/or `BrowserCore`/`App` becoming selectable via the async-core
work). Lenses: **narrow-slot-no-deferred-coupling** (don't couple storage demotion
to unmodelled per-session mode propagation), **one-issue-one-way** (the
shell-side mode authority should be designed once, with the dual-mode program, not
bolted onto a storage PR), and **existing-infra premise** (the shell delivery infra
is *not* mode-complete; building on it now = the canary the feedback warns about).
This is a **scope reduction vs. a Codex-reviewed A0 row**, so it is **§7 Q1 — the
primary plan-review question**, surfaced honestly rather than silently cut.

---

## §4. File-level change plan (as built — verified compiles both profiles + 5959 tests green)

**(A) One-source flip.** `event_handler_consumer.rs:236` —
`web_storage_spec_level()` body `Modern → Legacy`; docstring updated (shell-delivery
deferral noted, slot referenced); canary test renamed
`a1_classifies_every_family_modern` → `family_classification_sources`, asserting
`web_storage = Legacy` (cookie/live-collection still `Modern`/`Living`).

**(B) Realm-scope `[Exposed=Window]`.** `vm/globals.rs` — instead of *moving* the
installs (the `Storage` global already runs before `register_window_prototype` in
the scope-common region, satisfying the `storage_prototype` ordering), the two
install blocks (`register_storage_global` / `register_storage_event_global`) gain a
`matches!(self.global_scope_kind, GlobalScopeKind::Window) &&` guard. The accessors
were already Window-only (`register_window_prototype` runs only in the Window arm).
Ordering preserved by *not* moving.

**(C) `compat-webapi` cfg-gating (absence guarantee — resolves §7 Q2/Q3).** Two
cooperating gates:
- **C1 — `elidex-storage-core` `web-storage` feature (Q2 answer = yes, a
  storage-core feature).** Gates `pub mod web_storage` + the `WebStorageManager` /
  `SessionStorageState` / `StorageArea` re-exports. Off by default; Cache/SW/IndexedDB
  never used those types, so they compile without it. This is what makes the *types*
  truly absent (not merely unused) in the `App` binary.
- **C2 — `elidex-js` glue gated under `feature = "compat-webapi"`**, which now
  enables `elidex-storage-core?/web-storage` (weak-dep — additive, pulls the backend
  feature only when `engine` already pulled the crate). Gated: `host/mod.rs`
  `storage`/`storage_event` module decls (`all(engine, compat-webapi)`); `storage.rs`
  inner header; `host_data.rs` the `elidex_storage_core` import + 4 fields
  (`web_storage`/`session_storage`/`opaque_origin_sentinel`/`fallback_local_storage`)
  + their ctor inits + `install_web_storage`/`web_storage()`/`opaque_origin_sentinel()`
  + the opaque-origin counter/prefix/helper + the `atomic` import; `window.rs` the
  `web_storage_spec_level` import + accessor install + `WINDOW_STORAGE_ACCESSORS` +
  the 2 getter natives; `globals.rs` the `web_storage_spec_level` import + 2 install
  blocks; `vm_api.rs` the 2 unbind clears.
- **C2 — `ObjectKind::Storage`/`StorageEvent` variant + reference gating (Q3
  answer).** The variants were already `#[cfg(feature = "engine")]`; tightened to
  `#[cfg(all(feature = "engine", feature = "compat-webapi"))]`. Because exhaustive
  `match`es over `ObjectKind` *already* compile with these variants absent (the
  pre-existing `engine`-off case), no catch-all arm is needed; only the ~12
  variant-*reference* sites are gated: the 10 named-property-exotic dispatch blocks
  (`ops_property` ×3, `ops_element` ×2, `dispatch_objects`, `dispatch_iter`,
  `coerce_format`, `natives_object::descriptor`/`prototype`) + the 2 explicit match
  arms in `gc/trace.rs` and `host/structured_clone.rs`. The `storage_local_instance`
  / `storage_session_instance` `ObjectId` fields (`vm/mod.rs`) stay always-defined
  (type-free, always-`None` in `App` builds) — avoiding GC/init churn.

**Shell:** **none** (§3 / §7 Q1 — deferred to slot
`#11-storage-event-mode-aware-delivery`).

---

## §5. Testing / Acceptance criteria

1. **One-source flip, all seams**: under `EngineMode::BrowserCore`/`App`
   (`#[cfg(test)]` `new_with_mode`), assert **absent**: `Storage` global,
   `StorageEvent` global (`typeof StorageEvent !== 'function'`),
   `window.localStorage`/`sessionStorage`, `window.onstorage`. Under
   `BrowserCompat`: all **present** (production parity). Reuse A1's seam probes
   where they exist; storage is now *real* `Legacy`, so the synthetic marked-Legacy
   API is no longer the only witness.
2. **Realm scope**: a DedicatedWorker / ServiceWorker VM (even under
   `BrowserCompat`) exposes **no** `Storage`/`StorageEvent` (the corrected
   `[Exposed=Window]`); a Window VM still does.
3. **Both Cargo profiles compile**: (a) `--features engine,compat-webapi`
   (browser) — sync storage present; (b) `--features engine` alone (app) — Web
   Storage glue absent **and** Cache API / SW still compile (`elidex-storage-core`
   still linked). Add to CI matrix awareness (A0 R5-1 additive semantics).
4. **`BrowserCompat` byte-identical for Window realms** (the only behavior change
   for Window is the intended worker `[Exposed=Window]` correction, R8-3).
5. `cargo fmt --all` + `mise run ci` green; scoped `cargo test -p elidex-js
   --all-features` + `-p elidex-script-session`.

---

## §6. Collision / sequencing (re-confirmed at HEAD `f4d76c4f`)

- **`window.rs` vs media Slice 2b — CLEARED** (§0.2; `matchMedia` already landed).
  A2's `window.rs` edits are the storage accessor cfg-gate, disjoint from
  `matchMedia` (`:523`).
- **`shell/app/` vs `shell-viewport-pr-b` (App.placement) — AVOIDED by §3
  recommendation** (A2 touches no shell files). If plan-review *rejects* the §3
  deferral and pulls shell plumbing into A2, sequence A2's shell work **after**
  PR-B lands (rebase) to avoid the active collision.
- **`document.rs` vs A3 (cookie)** — A3 is `∥` and edits the cookie source
  (`document_cookie_spec_level`), disjoint from A2's storage source. No collision.
- **Worktree**: `webapi-compat-a2-storage` off `origin/main` (this memo's tree).

---

## §7. Open questions for `/elidex-plan-review`

1. **(PRIMARY) Shell event-delivery suppression — RESOLVED: defer (ratified).**
   `/elidex-plan-review` Axis 3 independently assessed the deferral as a legitimate
   philosophy-driven scope boundary (not a pragmatic scope-cut); the user ratified
   the lens-driven decision. Deferred to slot `#11-storage-event-mode-aware-delivery`
   (registered in the ledger at landing). Rationale unchanged (§3): shell has no
   `EngineMode`; VM not wired to shell [boa runs]; `BrowserCore`/`App` non-selectable
   ⟹ zero live consumer; active `shell/app/` collision; narrow-slot lens.
2. **`elidex-storage-core` feature split — RESOLVED (yes, a storage-core feature).**
   Implementation showed that gating only the elidex-js glue would leave
   `WebStorageManager`/`SessionStorageState` *compiled* in the still-linked
   `elidex-storage-core`, so the AC's "drops the Web Storage code" (R9-1) is met only
   by adding a non-default `web-storage` feature to `elidex-storage-core` gating those
   types, enabled from `elidex-js`'s `compat-webapi` via the weak-dep
   `elidex-storage-core?/web-storage`. Cache/SW/IndexedDB never used those types →
   compile without the feature (verified: `cargo check -p elidex-storage-core` default
   + `--features web-storage` + the `engine`-only elidex-js profile both green). §4 C1.
3. **`compat-webapi`-off field shape — RESOLVED (no `cfg(not)` counterpart needed).**
   The gated `HostData` fields have **no** always-compiled readers: every reader
   (`storage.rs`, the `vm_api.rs` unbind clears, the dispatch blocks) is itself
   `compat-webapi`-gated, so the fields are simply absent — no dual field shape, no
   strangler. `engine`-only build compiles clean under `clippy -D warnings`. §4 C2.
   **The 4 `VmInner` `ObjectId` cache slots** (`storage_prototype` /
   `storage_event_prototype` / `storage_local_instance` / `storage_session_instance`)
   intentionally stay at the looser gate (always-compiled field; `engine`-gated read
   in the GC roots array with a `None` fallback) — this is the **established
   VmInner-slot convention** (`gc/collect.rs`'s counted intrinsic-roots array gates
   every slot `#[cfg(engine)] self.X / #[cfg(not engine)] None`; the slots are
   correctly always-`None` in `App` builds because all writers are `compat-webapi`-
   gated). They are type-free (carry no `elidex-storage-core` type), so the A2
   absence guarantee ("drop the `WebStorageManager`/`SessionStorageState` *code*")
   is fully met; tightening these 4 to `all(engine, compat-webapi)` would make them
   the only feature-gated `VmInner` slots and fragment the uniform roots array for
   zero runtime effect. (`/elidex-review` Axis 2+3 flagged the asymmetry as MIN;
   accepted-as-is per this convention — Trigger-B root check.)
4. **Opaque-origin slot.** A0 says re-evaluate
   `#11-storage-opaque-origin-securityerror` at A2. Recommendation: keep as-is
   (orthogonal to gating; depends on sandbox/opaque-origin plumbing, same trigger
   as the cookie one) — confirm no A2 action.
5. **Re-grep discipline (Axis 5).** All §1 anchors re-grepped at `f4d76c4f`;
   confirm none drift before the implementation commit.

---

## §8. Citation appendix (webref-verified, `.claude/tools/webref`)

| Concept | §number → title | Anchor |
|---|---|---|
| Web storage chapter | HTML §12.2 — *The API* | `#storage` |
| `Storage` interface (`[Exposed=Window]`) | HTML §12.2.1 — *The Storage interface* | `#the-storage-interface` |
| `sessionStorage` getter | HTML §12.2.2 — *The sessionStorage getter* | `#the-sessionstorage-attribute` |
| `localStorage` getter | HTML §12.2.3 — *The localStorage getter* | `#the-localstorage-attribute` |
| `StorageEvent` (`[Exposed=Window]`) | HTML §12.2.4 — *The StorageEvent interface* | `#the-storageevent-interface` |
| `onstorage` (WindowEventHandlers) | HTML §8.1.8.2 — *Event handlers on elements, Document objects, and Window objects* | `#handler-window-onstorage` |

> Stale in-code citations (`window.rs:434` "§11.2"; `StorageEvent` "§11.4.2";
> `content/mod.rs:558` "§11.2.1") are owned by the **independent F2 clerical
> micro-PR** (A0 §1.5), **not** A2 — A2 must not fold the citation sweep in
> (avoids the per-owner ownership error A0 R3-3 flagged). The §-numbers above must
> be re-confirmed by `body html <anchor>` before the impl commit (number↔title
> pairs lookup-verified, not transcribed).

---

## §9. Defer slots registered by this PR

- **`#11-storage-event-mode-aware-delivery`** (§3 / §7 Q1). *Why:* the shell
  broadcasts `StorageEvent` to tabs with no `EngineMode` metadata; a
  Web-Storage-excluded (`BrowserCore`/`App`) session should not receive delivery.
  Deferred because the shell carries no `EngineMode` today, the VM is not the
  shell's engine yet (boa), and the excluded mode is not production-selectable, so
  there is no live consumer to suppress. *How to apply:* when `EngineMode` first
  flows shell→content (S5 VM cutover and/or async-core makes `BrowserCore`/`App`
  selectable), filter `StorageEvent` delivery (cleanest placement TBD at that
  point: shell broadcast filter vs. content-side `dispatch_storage_event`
  self-suppression via the session's `EngineMode`). *Trigger:* S5 boa removal OR
  `#11-async-core-storage-cookiestore` landing. *Re-evaluation date:* revisit with
  the S5 / world_id dual-mode program (MEMORY.md Active state), or sooner if a
  `BrowserCore`/`App` storage mode is scheduled. *(Q1 resolved = defer; slot
  registered in the ledger at landing.)*
- Pre-existing slot **`#11-storage-opaque-origin-securityerror`** re-evaluated
  (§7 Q4): unchanged, kept.
