# S5-2 — window-parity live producers (Screen + VisualViewport)

Per-PR plan-memo under the S5 umbrella (`docs/plans/2026-06-s5-flip-boa-deletion-umbrella.md`,
§5 row "S5-2 minor window parity"). **Anchor = the ideal end-state**, not the incremental patch
on `c925ff6f` (`feedback_plan-memo-anchor-on-ideal-not-incremental`). This memo replaces the
umbrella's "presence-first / no-plan-review" framing for S5-2 after Codex R1/R2 on #423 falsified
it across all three surfaces (`project_s5-2-replan-infra-backed`).

> **Gate**: `/elidex-plan-review` (5-agent design review) BEFORE impl, per CLAUDE.md
> "Edge-dense work = multi-PR program + 実装前 plan-review 必須" — this slice crosses
> transport × event-producer × platform-object-rigor × VM≥boa, so the base-case
> plan-review exemption no longer applies.

All file:line cites re-verified against the `s5-2-minor-window-parity` worktree tip `c925ff6f`
(2026-06-28) + main HEAD `4f777467`. Spec sections webref-verified 2026-06-28 (`cssom-view-1`).

---

## §0 Re-scope decision (read first)

### §0.1 cookieStore is SPLIT out of S5-2 (already carved at `c925ff6f`)

The umbrella's S5-2 row bundled **VisualViewport + cookieStore + Screen**. Codex R1 on #423 produced
8 P2 findings, **all on cookieStore**, all rooting to one mechanism (routing structured cookieStore
args through the `document.cookie` Set-Cookie *string* chokepoint, breaking Cookie Store API §3
structured semantics). cookieStore was carved at `c925ff6f` to slot
`#11-cookiestore-structured-spec-faithful` with its own thorough scope memo
(`project_cookiestore-structured-spec-faithful`).

**This plan keeps the split.** cookieStore is a **different subsystem** (an `elidex_net::CookieJar`
structured-setter + Cookie Store §3 algorithm concern) from Screen/VisualViewport (a viewport /
device-fact transport + platform-object concern). Bundling them would produce a wide PR crossing the
cookie-jar crate AND the viewport-transport AND the platform-object-kind machinery — exactly the
edge-dense bundle the discipline forbids. Each is independently edge-dense with its own slot, so the
ideal decomposition is **two plan-reviewed slices**:

- **S5-2 (this memo)** = Screen + VisualViewport window-parity live producers.
- **cookieStore (separate)** = its own plan-memo at pickup, seeded by `project_cookiestore-structured-spec-faithful`.

(`/elidex-plan-review` should challenge the split if it sees a cheaper bundling; the recommendation
is split.)

### §0.2 S5-2 is re-scoped from "presence-first additive" to "infra-backed VM capability"

The worktree's presence-first Screen/VisualViewport were **not** 0-findings benign approximations —
Codex R2 found 4 P2 across both (T1–T4 below). The re-scope: build each surface **with its backing
infra** (a dedicated transport endpoint, a real event producer, platform-object rigor) so it is
**non-regressing at the flip (S5-6)**, rather than a presence-first stub.

---

## §1 Goal + ideal end-state

**Goal**: give the bytecode VM a `Screen` and `VisualViewport` surface that is **strictly ≥ boa** and
**spec-faithful**, so the S5-6 flip exposes them to real pages without regression.

**Ideal end-state**:
- **`window.screen`** (CSSOM-View §4.3) returns **monitor** dimensions (CSS px), not viewport
  dimensions, read from a dedicated device-fact field — never aliased to `innerWidth`/`innerHeight`.
  The monitor dims arrive over a **dedicated `set_screen_dimensions` transport endpoint**, a sibling
  of `set_media_environment` (monitor dims are a device fact but **not** a `MediaEnvironment` input —
  no media feature reads them, and there is no `change` event for `screen`, so they need a state-push
  with **no delivery turn**).
- **`window.visualViewport`** (CSSOM-View §12.1; spec IDL `[SameObject, Replaceable] readonly attribute
  VisualViewport? visualViewport;` — nullable) is a singleton whose geometry getters read live
  `ViewportState`, and whose `resize`/`scroll`/`scrollend` events fire through a **real VM producer**
  (`deliver_visual_viewport_events`, modeled on `deliver_media_query_changes`) — never an inert event
  surface (the "presence-without-events is worse-than-absence" failure, §2 T2).
- **Both singletons install via the engine's standard `[Replaceable]`-Window-attribute treatment** —
  the spec IDL is `[SameObject, Replaceable]` (webref-verified §4), and elidex installs **every**
  `[Replaceable]` Window attribute (`innerWidth`/`innerHeight`/`scrollX`/`scrollY`/`devicePixelRatio`,
  the `WINDOW_RO_ACCESSORS` table at `window.rs:591`) as a **no-setter RO accessor** — it does **not**
  implement `[Replaceable]` value-shadowing anywhere. So screen/VV move from their **anomalous
  writable-global** install (today the only writable Window attrs besides `name`, per the `window.rs:501`
  comment) onto that same no-setter RO accessor + cached-singleton getter (the
  `localStorage`/`sessionStorage` `[SameObject]` form). This **normalizes** screen/VV to their sibling
  device-fact attrs and preserves `[SameObject]` identity. Implementing proper `[Replaceable]`
  (assignment shadows the accessor with an own data prop, WebIDL §3.3.11) is a **pre-existing
  engine-wide gap** that affects innerWidth/scrollX/dppx identically → deferred engine-wide (§9),
  deliberately **not** implemented for screen/VV alone (that would be a lone-`[Replaceable]` outlier,
  One-issue-one-way violation).
- **Both are dedicated platform-object kinds** — `structuredClone(screen)` /
  `structuredClone(visualViewport)` throw `DataCloneError` (T4), not silently clone to `{}`.
- **Layering preserved**: Screen/VisualViewport getters read VM-global `ViewportState`; the host only
  marshals. No new algorithm in `host/`.
- **ECS-native preserved**: monitor dims + VV geometry are **shell-driven device facts** (per-VM
  transport state, the CLAUDE.md side-store (b) shared-cross-cutting exception), **not** per-entity
  DOM components — same classification as the already-landed `inner_width`/`dppx`/`color_scheme`.

---

## §2 Current-state reconciliation (`c925ff6f`) + the four defects

`c925ff6f` ships `screen` + `visualViewport` on the VM (cookieStore already carved). The reusable,
**correct** parts (Codex 0-findings) and the four defects:

### Reusable (keep, Codex 0-findings)
  > **Worktree note**: `crates/script/elidex-js/src/vm/host/visual_viewport.rs` (S5-2 surface) and
  > `ObjectKind::VisualViewport` (S5-2 surface) are **net-new in the `c925ff6f` worktree, not yet in
  > main** — so a main-checkout grep (the preflight's) does not find them; they are the reuse base
  > this PR builds on.
- **VisualViewport geometry getters** (`crates/script/elidex-js/src/vm/host/visual_viewport.rs:160-247`):
  `offsetLeft/Top → 0`, `pageLeft/Top → ViewportState.scroll_x/y`, `width/height →
  inner_width/inner_height`, `scale → 1.0`. All **correct live reads** (CSS px; no pinch-zoom modeled
  yet → offset 0 / scale 1, spec-faithful for a non-pinch-zoom UA).
- **`VisualViewport.prototype → EventTarget.prototype` chain + `onresize/onscroll/onscrollend` IDL
  handler attrs** (`visual_viewport.rs:84-111`) over the shared VmObject event-handler backend;
  `ObjectKind::VisualViewport` (`object_kind.rs:739`) + its `is_non_node_event_target` membership
  (`object_kind.rs:1609`) + structured_clone Unclonable arm (`host/structured_clone.rs:317`) + gc no-op
  trace arm — **VV's T4 is already fixed** in the worktree.
- **Screen `colorDepth`/`pixelDepth`** getters (worktree `screen.rs`) — return 24 (CSSOM-View §4.3
  sanctions returning 24 when the UA does not expose color depth; the two return the same value "for
  compatibility reasons" per the spec note). **Constant, not a transported fact.**
- **`wire_interface_ctor_prototype` convergence** (`shape_ops.rs:343`, 4 callers — verified 2026-06-28
  via `grep -rn wire_interface_ctor_prototype crates/script/elidex-js/src/` in worktree `c925ff6f`:
  `dom_parser.rs:99/131` (DOMParser + XMLSerializer), `visual_viewport.rs:123`, `media_query.rs:187`)
  — One-issue-one-way interface-ctor↔prototype wiring;
  reuse for Screen's interface object. (Residual ~30 inline call-sites = separate sweep
  `#11-interface-ctor-prototype-wiring-convergence`.)

### Defect T1 — `screen.width` returns `innerWidth` (VM < boa regression)
Worktree `screen.rs` accessors read viewport (`inner_width`) for `width`/`availWidth`. boa returns
**monitor** dims (`elidex-js-boa/src/globals/window/screen.rs:25/35` → `bridge.monitor_width/height()`).
On a non-maximized window a responsive probe reading `screen.width` sees the **window** size, not the
display — a visible regression vs boa. **Fix**: `screen.*` reads a dedicated `ViewportState.screen_*`
field, populated by a new `set_screen_dimensions` endpoint.

### Defect T2 — VisualViewport event surface is inert (worse-than-absence)
Worktree has **no producer at all** for `resize`/`scroll`/`scrollend` (module doc, `visual_viewport.rs:53-57`:
"have no shell producer yet, so they never fire" — the deferred slot `#11-s5-2-window-parity-live-producers`).
A page doing `if (window.visualViewport) vv.addEventListener('resize', h); else window.addEventListener('resize', h);`
registers on VV and **never** gets `resize`, losing the `window.resize` fallback it would have taken
had VisualViewport been absent. This only bites **once the VM is live (S5-6)**, but it bites unless the
producer exists by then. **Fix**: build the VM producer now (so S5-6 only wires the shell call-site).

### Defect T3 — `screen`/`visualViewport` installed as **anomalous writable globals** (inconsistent with sibling `[Replaceable]` Window attrs)
Both are `globals.insert(name, obj)` writable data slots (`screen.rs:63`, `visual_viewport.rs:139`) —
making them, with `name`, the **only** writable Window attributes the VM exposes (`window.rs:501`
comment: "`name` is the only writable Window attribute"). Their spec IDL is `[SameObject, Replaceable]`
(webref-verified §4), and **every other** `[Replaceable]` Window attr (`innerWidth`/`scrollX`/`devicePixelRatio`,
`WINDOW_RO_ACCESSORS` at `window.rs:591`) is installed as a **no-setter RO accessor**, not a writable
global. So screen/VV are the outlier. **Fix**: normalize them onto the sibling treatment — install as
no-setter RO accessors on `Window.prototype` whose getter returns the cached singleton `ObjectId` (the
`localStorage`/`sessionStorage` `[SameObject]` form, `window.rs:521`/`:622-642`); `screen = null` then
hits the inherited-no-setter branch (`ops_property.rs:445`, `NoDataWrite`) and never reaches
`globals.insert`. **Note**: this is the *same* `[Replaceable]` divergence the whole `WINDOW_RO_ACCESSORS`
family already has (none implement `[Replaceable]` value-shadowing); S5-2 makes screen/VV **consistent**
with that family, and the engine-wide proper-`[Replaceable]` gap is slotted (§9), **not** fixed for
screen/VV alone (Codex R2's T3 "singleton 喪失" was directionally right about the inconsistency, but
the spec-faithful target is the family's existing no-setter form, not a lone `[Replaceable]` impl).

### Defect T4 — `Screen` is `ObjectKind::Ordinary` (structuredClone silently succeeds)
`structuredClone(screen)` clones the Ordinary object to `{}` (accessors skipped → empty), no
`DataCloneError` (`structured_clone.rs:228` → `clone_ordinary` succeeds). **Fix**: add
`ObjectKind::Screen` (NEW) (payload-free) + Unclonable classify arm + no-op trace arm (3 arms; Screen is
not an EventTarget so it needs neither `is_non_node_event_target` nor `is_callable`). (VV's kind +
arms already exist in the worktree.)

---

## §3 Spec coverage map (webref-verified `cssom-view-1`, 2026-06-28)

Citations webref-verified 2026-06-28 via `.claude/tools/webref heading cssom-view-1` (`cssom-view-1`
is not yet in the preflight's `SPEC_LABEL_REVERSE` → rows show as "unmapped" soft-warn; the §-titles
below are the manually-verified pairs: §4.3 = The Screen Interface, §12.1 = The VisualViewport
Interface, §4 = Extensions to the Window Interface).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM-View §4.3 The Screen Interface | `width`/`height` attr | monitor dims in CSS px | `screen.rs` getters → `ViewportState.screen_*` (NEW field) | ✓ | no (shell-driven) |
| CSSOM-View §4.3 The Screen Interface | `availWidth`/`availHeight` attr | = full monitor dims (no work-area source) | `screen.rs` getters → `ViewportState.avail_*` (NEW field) | ✗ (work-area deferred §9) | no |
| CSSOM-View §4.3 The Screen Interface | `colorDepth`/`pixelDepth` attr | constant 24 (UA-undisclosed fallback, spec-sanctioned) | `screen.rs` getters | ✓ | no |
| CSSOM-View §12.1 The VisualViewport Interface | geometry getters (`offsetLeft/Top`, `pageLeft/Top`, `width`, `height`, `scale`) | not-fully-active → 0, else live `ViewportState` read | `visual_viewport.rs` getters (reuse `c925ff6f`) | ✓ | no |
| CSSOM-View §12.1 The VisualViewport Interface | `onresize`/`onscroll`/`onscrollend` events | size→`resize` / scroll→`scroll`+`scrollend` | `deliver_visual_viewport_events` (NEW) → `fire_vm_event` | ✗ (scrollend momentum deferred §9) | no |
| CSSOM-View §4 Extensions to the Window Interface | `window.screen` `[SameObject, Replaceable]` | RO singleton install (T3); `[Replaceable]` not impl (family-consistent, §9) | `Window.prototype` RO accessor → cached `ObjectId` | ✗ (`[Replaceable]` deferred engine-wide §9) | no |
| CSSOM-View §4 Extensions to the Window Interface | `window.visualViewport` `[SameObject, Replaceable]`, type `VisualViewport?` | (a) fully-active → singleton (RO install T3); (b) **not fully active → null**; `[Replaceable]` not impl | `Window.prototype` RO accessor → cached `ObjectId`; not-fully-active → null | ✗ (`[Replaceable]` §9; null branch = single-doc-unreachable, impl for faithfulness) | no |

**Breadth** (the 7 table rows above): K=1 spec (`cssom-view-1`), M=7 → single PR scope (well inside the band; the
edge-density here is the §4 design corner, not spec breadth — hence plan-review on coupled-invariants,
not a split).

### §3.1 User-input touch audit

No surface takes user-controllable input — Screen + VisualViewport are **shell-driven** (monitor dims,
viewport geometry, scroll offset all originate in the shell, not page script). The page can only
**read** the getters and **register listeners**; it cannot inject values. No new sanitization
obligation (the umbrella §3.1 trust-boundary-neutral audit covers this cohort). Adjacent pre-existing
code exposure: the T3 normalization moves `screen`/`visualViewport` onto the no-setter RO-accessor form
its sibling `[Replaceable]` Window attrs already use (a consistency fix, not a security boundary — a
page already cannot shadow `innerWidth`/`scrollX`); navigator/location/history keep their pre-existing
looseness (slotted §9), exposure unchanged by this PR.

**Spec-faithfulness notes** (per CSSOM-View prose, webref):
- §4.3 `width`/`height`/`availWidth`/`availHeight` return the **Web-exposed screen area in CSS px** —
  so the shell observes physical `current_monitor().size()` and divides by the monitor scale factor
  **at the producer** (matching how `inner_width` is already CSS px), keeping the VM getter a pure read.
- §4.3 `colorDepth`/`pixelDepth` "should return 24" when the UA does not expose the value → constant 24.
- §12.1 each VisualViewport geometry getter step 1: "If the associated document is not fully active,
  return 0." → guard in the getters (worktree returns live reads unconditionally; add the
  not-fully-active → 0 guard, or document why it is unconditionally fully-active in elidex's
  single-document model). **Distinct from** the §4 *attribute*-level branch (next note) — two layers.
- §4 `window.visualViewport` (`VisualViewport?`): "If the associated document is fully active, … return
  the VisualViewport object …; **Otherwise, it must return null**" (webref-verified). This is the
  **attribute-level** null branch, separate from the §12.1 geometry-getter → 0 branch. In elidex's
  single-document bound model the bound document is effectively always fully active → the null branch is
  **currently unreachable**, but the getter implements it for spec-faithfulness (returns null when no
  bound/active document) rather than asserting always-present. (Q5 covers both layers.)
- §12.1 `scale` getter has **three** steps (webref-verified): (1) not fully active → 0; (2) **no output
  device → return 1**; (3) otherwise → the visual viewport's scale factor. elidex's `scale → 1.0`
  constant coincides with step 2/3 for a non-pinch-zoom UA with an output device; the §3 row's
  "not-fully-active → 0, else live read" wording is shorthand — `scale`'s non-step-1 result is the
  constant 1 (steps 2+3 collapse for elidex), not a "live read".
- §4.3 `availWidth`/`availHeight` = the **available** (OS-chrome-excluded) screen area. winit exposes
  no cross-platform work-area API → `avail* = full monitor dims` (boa parity, a common UA fallback) →
  **defer** the real work-area source (§9).

---

## §4 The three coupled mechanisms (the design corner)

The edge-density is the **intersection** of three mechanisms. Each pair's coupling named
(`feedback_coupled-invariant-design-corner`):

### (M1) monitor-dims transport endpoint (fixes T1)
- **VM SoT**: add `screen_width`/`screen_height`/`avail_width`/`avail_height: f64` (CSS px) to
  `ViewportState` (`host/window.rs:66-103`), siblings of `device_pixel_ratio`/`color_scheme`. Sane
  realistic default (e.g. 1920×1080), **distinct from the viewport default**, overridden by the
  producer at the flip.
- **Endpoint**: new `HostDriver::set_screen_dimensions(w, h, avail_w, avail_h)` (NEW)
  (`elidex-script-session/src/engine.rs`, sibling of `set_media_environment` at `:364`) →
  `ElidexJsEngine::set_screen_dimensions` (NEW) (`elidex-js/src/engine.rs`, alongside `:523`) →
  `Vm::set_screen_dimensions` (NEW) (`vm_api.rs`, alongside `:1094`). **Pure state push, NO delivery turn**
  (no `change` event for screen; not a `MediaEnvironment` field → does **not** flow through
  `media_environment()` or `deliver_media_query_changes`).
- **Coupling — M1 × `set_media_environment`**: both are device-fact pushes, but monitor dims are
  **not** a media input. Keeping them a **separate endpoint** (not extra params on
  `set_media_environment`) prevents polluting `MediaEnvironment` and prevents implying a media re-eval
  turn. (This is the One-issue-one-way decision the plan locks: device-fact ≠ media-fact.)

### (M2) VisualViewport event producer (fixes T2)
- **Producer**: new `Vm::deliver_visual_viewport_events()` (NEW) (modeled on
  `host/media_query.rs:352 deliver_media_query_changes`): bound-guard → diff current `ViewportState`
  against a stored prior → fire each changed event via `fire_vm_event`
  (`host/event_target_dispatch_vm.rs:392`) at the singleton's cached `ObjectId` (VmObject home).
  - **Per-axis diff** (not a single "geometry changed" flag): `resize` ⇐ the `(inner_width,
    inner_height)` pair changed vs prior; `scroll` ⇐ the `(scroll_x, scroll_y)` pair changed vs prior;
    `scrollend` ⇐ **only when `scroll` fired** (a resize-only deliver must not fire `scroll`/`scrollend`).
    The shell echoes discrete settled offsets, so each settled scroll fires `scroll`+`scrollend`;
    momentum/debounce gesture-end timing = §9 defer.
- **State**: a `vv_delivered: Option<(w,h,scroll_x,scroll_y)>` prior advanced after each deliver, in VM
  host state, mirroring the MQL flip-prior. **Seed (F3)** — pinned to the lazy-alloc path, **not** a
  separate bind hook: `vv_delivered` is seeded to the current `ViewportState` geometry **inside the
  cached-singleton alloc-or-cached getter** (the exact `create_media_query_list` parallel — it seeds
  `last_matches` at construction, `media_query.rs:241`). Because the producer resolves the singleton
  **through that same getter** (below), the getter **allocates-and-seeds `vv_delivered` before the
  producer's first diff-read** — so the first deliver after creation (or after an unbind that reset the
  prior to `None`, F4) diffs against the real starting geometry and fires **nothing** spuriously. The
  seed-write thus happens-before every diff-read by construction; there is no `None`-window where the
  producer reads an unseeded prior (which would mis-fire `resize`+`scroll`). The singleton's `ObjectId`
  is **not** held in a separate cross-DOM side-cache — that same getter is the sole resolution path for
  both the T3 RO accessor and the producer (`alloc_or_cached`-style, §4-M3), and it participates in the
  unbind scrub (F4 / §4-M3), which is also where `vv_delivered` resets to `None`.
- **Endpoint**: `HostDriver::deliver_visual_viewport_events` (NEW) → `ElidexJsEngine` → `Vm`, alongside
  the `deliver_media_query_changes` plumbing (`engine.rs:540` / `vm_api.rs:1117`).
- **Coupling — M2 × the existing window `resize`**: the shell already fires window `resize` at the
  **document entity** (Node home, `content/event_loop.rs:409-417`). VV `resize` is a **separate**
  VmObject-home dispatch at the singleton — it **cannot** piggyback the document dispatch. Both fire
  at the flip (a page legitimately receives both `window.resize` and `visualViewport.resize`). This
  is the load-bearing distinction the producer must honor.
- **Coupling — M2 × GC keepalive (S5-3)**: the VV singleton is `[SameObject]` + GC-rooted by the
  singleton cache + the `Window.prototype` RO accessor (M3), so it is **never** listener-only-rooted —
  so S5-2's VV producer does **not** depend on the S5-3 keepalive-rooting slot (which covers
  `matchMedia(q)`-style transients). Stated here so plan-review confirms VV is outside S5-3's hazard.

### (M3) platform-object rigor (fixes T3, T4)
- **T3 (normalize to the sibling install form)**: install `screen` + `visualViewport` as **no-setter
  RO accessors on `Window.prototype`** whose getter returns the **cached singleton `ObjectId`**
  (`install_ro_accessors` + an `alloc_or_cached`-style cached-singleton getter, the
  `localStorage`/`sessionStorage` `[SameObject]` form at `window.rs:521`/`:622-642`, AND the
  `WINDOW_RO_ACCESSORS` no-setter treatment that `innerWidth`/`scrollX`/`dppx` already use for their
  `[Replaceable]` IDL). Replaces the **anomalous** writable `globals.insert`. The getter (T3) and the
  M2 producer resolve the singleton through the **same** cached getter — no separate cross-DOM side-cache.
  **`[Replaceable]` is deliberately not implemented** (family-consistent; the engine-wide gap → §9).
  **Scope = screen + VV only**.
- **Singleton lifecycle / cross-DOM (F4)**: the cached-singleton getter must follow the `localStorage`
  precedent **fully** — its cache is **cleared on `unbind`** (the `Vm::unbind` cross-DOM scrub,
  `vm_api.rs:773 clear_storage_instance_cache`, which the cited `localStorage` form itself participates
  in), and the `vv_delivered` prior (M2) is reset there too. This avoids firing `fire_vm_event` at a
  **stale `ObjectId` from a prior `EcsDom`** after rebind (two `EcsDom` worlds share ObjectId/entity
  space). I.e. screen/VV are treated as **per-bind** cached singletons (like `localStorage`), not
  realm-structural survivors — the cleanest cross-DOM-safe choice and consistent with the precedent
  the install form is borrowed from.
- **T4**: add `ObjectKind::Screen` (payload-free, engine-gated) + `classify` Unclonable arm
  (`host/structured_clone.rs`) + no-op `gc/trace.rs` arm. Re-home Screen's accessors onto a
  `Screen.prototype` (mirroring VV's structure minus EventTarget), allocate the singleton as
  `ObjectKind::Screen`.
- **Pre-existing looseness left out (One-issue-one-way framing)**: `navigator`/`location`/`history`
  share **all three** of T3 (writable `globals.insert`, `navigator.rs:179`/`location.rs:324`), T4
  (`Ordinary` kind), and the `[Replaceable]` non-impl — **pre-existing, not introduced by S5-2**. S5-2
  uses the **existing canonical** no-setter-RO-accessor + dedicated-kind forms for its **new** surfaces
  (it does **not** introduce a new form), and registers the pre-existing debt as an **engine-wide
  normalization slot** (§9) that coordinates with the existing `#11-navigator-interface-object-branding`
  (#398), not a fresh strangler. Converging 3 established globals + their tests + a proper `[Replaceable]`
  install is a separate cohesive unit. (Plan-review: confirm "fix what you touch + slot the
  pre-existing", not a strangler S5-2 creates.)

---

## §5 Layering + ECS-native lens

- **Layering**: Screen/VisualViewport getters read VM-global `ViewportState` (marshalling), the §4.3
  CSS-px conversion lives at the **shell producer** (device-fact observation), and the §12.1
  geometry semantics are pure reads. **No algorithm in `host/`** — consistent with the mandate.
- **New-fn → engine-indep-crate mapping** (the `[plan]` Layering artifact): every S5-2 `host/` fn is
  marshalling/transport with **no domain algorithm** → no engine-indep crate applies. Explicit table:

  | New `host/` fn | Engine-indep crate / API | Why no crate |
  |---|---|---|
  | `screen.*` getters | — (read `ViewportState`) | pure field read |
  | `set_screen_dimensions` | — (state push) | transport endpoint, no logic |
  | `deliver_visual_viewport_events` | — (f64 field diff + `fire_vm_event`) | **marshalling-only**, precedent = `deliver_media_query_changes` (also lives in `host/media_query.rs` as marshalling-only; its *domain* algorithm — `evaluate` — is the part that lives engine-indep in `elidex_css::media`, and the VV producer has **no** such algorithm) |
  | T3 RO-accessor getters / T4 `ObjectKind::Screen` arms | — (VM-internal brand/property machinery) | engine-bound by definition (marshalling layer) |

  The CSS-px conversion (the one piece of arithmetic) lives at the **shell producer**, not `host/`.
- **ECS-native (side-store→component audit)**: monitor dims + VV geometry are **shell-driven per-VM
  device facts** carried in `ViewportState`. They are **not** per-entity DOM facts → **not** ECS
  components. This is the CLAUDE.md side-store **(b) shared-cross-cutting** exception (browsing-context
  / device-level state), the **same** classification already accepted for `inner_width`/`dppx`/
  `color_scheme`. The screen/VV singletons are payload-free `ObjectId`s whose state derives entirely
  from `ViewportState` → no per-instance side table, no GC payload. (Plan-review ECS axis: confirm
  this is (b), not a per-entity fact mis-stored in a side-store.)

---

## §6 Implementation decomposition (file-level, deltas from `c925ff6f`)

**VM-side (the deliverable — all VM-test-exercised):**
1. `host/window.rs:66-103` — add `screen_{width,height}`/`avail_{width,height}` to `ViewportState`
   + defaults.
2. `host/screen.rs` — re-home accessors to a `Screen.prototype`; read `ViewportState.screen_*` (not
   `inner_*`, fixes T1); allocate `ObjectKind::Screen`; RO-singleton install on `Window.prototype`
   (T3).
3. `host/visual_viewport.rs` — RO-singleton install on `Window.prototype` (T3) via the cached-singleton
   getter (no separate cross-DOM side-cache; resolved by the same getter the producer uses); add the
   producer `deliver_visual_viewport_events` + `vv_delivered` prior **seeded at creation/bind** (M2,
   F3); add §12.1 not-fully-active→0 guard (or document single-doc always-active) + the §4 attribute-level
   →null branch (F2).
4. `object_kind.rs` + `host/structured_clone.rs` + `gc/trace.rs` — `ObjectKind::Screen` 3 arms (T4).
5. `vm_api.rs` (`:1094`/`:1117` neighborhood) — `set_screen_dimensions` + `deliver_visual_viewport_events`.
6. `engine.rs` (script-session `:364`/`:379`; elidex-js `:523`/`:540`) — the two HostDriver trait
   methods + impls.
7. `Vm::unbind` (`vm_api.rs:773` scrub cohort) — clear the screen/VV singleton caches + `vv_delivered`
   prior on unbind (F4), alongside `clear_storage_instance_cache`.
8. Reuse `wire_interface_ctor_prototype` (`shape_ops.rs:343`) for Screen's interface object.

**1000-line touched-file audit** (F10, CLAUDE.md touch-time-split): S5-2 adds to four >1000-line files
(line counts verified 2026-06-28 via `wc -l` in worktree `c925ff6f`) — `object_kind.rs` (1667),
`vm_api.rs` (1208), `host/structured_clone.rs` (1058), `gc/trace.rs` (1209). **No split owed**: the `ObjectKind::Screen` additions are single arms on **flat
case-tables** (CLAUDE.md exempts "flat case table / 巨大 generated table"); the `vm_api.rs` additions
are two small public methods sitting **cohesively** in the existing `set_media_environment` /
`deliver_media_query_changes` device-fact neighborhood (no real cohesion seam to carve). Recorded so the
adds are not silent.

**VM tests** (the oracle, since boa is the live engine — §8): `set_screen_dimensions(W,H,...)` then
`screen.width === W` (CSS px); `screen !== window` width on a non-square viewport; `screen = null`
leaves the singleton (T3); `structuredClone(screen)` throws DataCloneError (T4); a `ViewportState`
size change + `deliver_visual_viewport_events()` fires `visualViewport`'s `resize` listener; a scroll
change fires `scroll` + `scrollend`; `[SameObject]` identity stable across reads.

**Out of S5-2 (rides S5-6, §8)**: the live shell producer — winit `current_monitor()` observe → CSS-px
conversion → `set_screen_dimensions`; the `deliver_visual_viewport_events` call-sites in
`content/event_loop.rs` (resize) + the scroll-echo path (`content/mod.rs:211`).

---

## §7 Edge matrix (review-tail pre-empt)

| Invariant axis | M1 transport | M2 VV producer | M3 platform-rigor |
|---|---|---|---|
| device-fact transport (`ViewportState`) | ✔ (screen_* fields + endpoint) | reads (size/scroll diff) | — |
| event delivery (VmObject home, `fire_vm_event`) | — | ✔ (resize/scroll/scrollend) | — |
| `[SameObject]` singleton identity | — | resolves via cached getter (shared w/ M3) | ✔ (RO install both, cached getter) |
| structured-clone brand | — | (VV done) | ✔ (Screen kind) |
| GC rooting | — | singleton GC-rooted (NOT listener-only → outside S5-3) | ✔ (cached singleton rooted) |
| cross-DOM cache lifecycle (unbind scrub) | — | `vv_delivered` reset on unbind | ✔ (singleton cache cleared on unbind, `localStorage` precedent) |
| `[Replaceable]` (engine-wide gap) | — | — | ✔ (not impl, family-consistent → §9 slot) |
| CSS-px ⟷ physical-px | ✔ (conversion at shell producer, §8) | reads CSS-px | — |
| VM ≥ boa | ✔ (monitor dims, fixes boa's latent 800×600 too at flip) | ✔ (boa has no VV) | ✔ |

**Densest intersection**: M3 × the singleton identity + cross-DOM lifecycle (the cached-singleton getter
must preserve `[SameObject]`, normalize the writable-global anomaly to the no-setter-RO-accessor family
form, AND clear on unbind to avoid stale-`ObjectId` cross-DOM aliasing) and M2 × the window-`resize`
distinction (two separate dispatch homes). Plan-review hardest there.

---

## §8 The boa-live constraint + S5-6 hand-off (the central scope decision)

**The shell drives the concrete `elidex_js_boa::JsRuntime` (not a `dyn HostDriver`) until the S5-6
flip** (umbrella §2: `pipeline.rs:10`, `lib.rs:39/433`). The VM's `HostDriver` surface
(`set_media_environment`, and now `set_screen_dimensions`/`deliver_visual_viewport_events`) is
exercised **only by VM tests** until S5-6 swaps the runtime to `ElidexJsEngine`. Consequences:

- **S5-2 = VM capability + endpoints + producers, VM-tested.** Every endpoint is reached by VM tests,
  so nothing is dead infra; the surfaces are real (not stubs) and non-regressing **at the flip**.
- **The live shell wiring rides S5-6** (the flip already re-points every device-fact producer from
  the boa bridge to the live VM `HostDriver`; the monitor-observe + the VV deliver-call-sites are part
  of that same re-point). This is **not** deferring backing infra — the backing (endpoint + producer)
  is in S5-2; only the live observation/dispatch call-site, which **must** move at the flip anyway,
  rides S5-6.
- **Why this is non-regressing**: VisualViewport never becomes visible to a real page without its
  producer being callable — both the producer (S5-2) and the shell call-site (S5-6) land by the flip,
  so the "inert VV worse-than-absence" window never opens for a real page. Screen reads a dedicated
  monitor field (a realistic default until the flip wires the observe) — boa-parity (boa's
  `set_monitor_dimensions` is itself uncalled today, so boa screen is also default-only), so no
  regression vs boa even mid-flight.

**Recommendation**: keep S5-2 pure VM-capability (no shell/boa touch), consistent with umbrella §8
("S5-1..S5-5 are VM-capability-only, boa stays live"). **Alternative considered & rejected**: wire the
live monitor-observe to the boa bridge now (C3 / #415 style, fixing boa's 800×600). Rejected because
(a) it touches boa + shell for a surface the flip re-points anyway (umbrella §0.2 "no mirroring new
work to boa"), and (b) it creates an asymmetry with VisualViewport (which has **no** boa equivalent to
wire). **Plan-review Q (§10-Q1)**: validate this boundary.

**S5-2 retires `#11-s5-2-window-parity-live-producers`**: the VM-side producer + transport endpoint
(the hard part of that slot) land here; the residual live shell call-sites fold into S5-6's
device-fact re-point. The slot is split into "done now (VM)" + "absorbed by S5-6 (shell)".

---

## §9 Defer ledger + slots (per-PR cap ≤3)

Per-PR cap ≤3 → **2 NEW slots** (the scrollend-timing item folds into the rigor/work-area tail rather
than a third slot, see below). Each carries the 3 required elements (Why / Re-evaluation-trigger /
Re-evaluation-date).

- **`#11-window-platform-object-rigor-engine-wide`** (NEW) —
  - **Why deferred**: the **pre-existing** `navigator`/`location`/`history` looseness has **three**
    facets — (a) loose-writable-global (T3, `navigator.rs:179`/`location.rs:324`), (b) `Ordinary` kind
    (T4), and (c) `[Replaceable]` non-impl (shared with the whole `WINDOW_RO_ACCESSORS` family). S5-2
    normalizes only its **new** surfaces (screen/VV); converging 3 established globals + the family-wide
    proper-`[Replaceable]` install touches established code + tests → separate cohesive sweep.
  - **Coordinates with** the existing `#11-navigator-interface-object-branding` (#398), which owns the
    adjacent *interface-object branding / `instanceof`* facet of the same navigator family — this slot
    should **subsume or co-land** with it (One-issue-one-way: one navigator-platform-object-rigor
    program, not branding-vs-writability-vs-clonability-vs-Replaceable split across slots).
  - **Re-evaluation trigger**: next PR that touches the navigator/location/history install or any
    `WINDOW_RO_ACCESSORS` member's identity/clonability, OR a compat-plugin pass that needs real
    `[Replaceable]` shadowing.
  - **Re-evaluation date**: demand-gated (no fixed date; surfaced at the next navigator-family touch).
- **`#11-screen-available-area-workarea-source`** (NEW) —
  - **Why deferred**: `availWidth`/`availHeight` need the OS-chrome-excluded work-area; winit exposes no
    cross-platform work-area API → S5-2 uses `avail* = full monitor dims` (boa parity, common UA fallback).
  - **Re-evaluation trigger**: winit gains a work-area API, OR a platform-specific shell backend exposes
    the taskbar-excluded rect.
  - **Re-evaluation date**: demand-gated.
  - **Folds in** the `scrollend` momentum/gesture-end timing tail (S5-2 fires `scroll`+`scrollend` per
    settled discrete echo; real debounce/momentum timing is the same class of "shell-input-fidelity
    tail" as work-area) — tracked here rather than a third slot to respect the per-PR cap; same
    trigger/date (demand-gated, surfaced when a real scroll-gesture/momentum source lands).
- **`#11-secure-context-window-interface-gating`** (existing, engine-wide) — NOT an S5-2 concern
  (Screen/VisualViewport are `[Exposed=Window]`, not `[SecureContext]`); stays with the
  caches/crypto.subtle/cookieStore cohort.
- **`#11-interface-ctor-prototype-wiring-convergence`** (existing) — residual ~30 inline call-sites,
  separate sweep; S5-2 reuses the converged `wire_interface_ctor_prototype` for Screen.

**Ledger admin note**: the four cited slots above (+ `#11-s5-2-window-parity-live-producers`,
`#11-cookiestore-structured-spec-faithful`) are registered in `project_s5-2-replan-infra-backed` +
MEMORY active-state (the 2026-06-28 post-#423-carve registration) but not yet folded into the canonical
`project_open-defer-slots.md` (last reconciled 2026-06-19, pre-carve) — the canonical fold is owed at
S5-2 **landing-memo** time (admin debt, not a plan blocker).

---

## §10 Open questions for /elidex-plan-review

- **Q1 (the §8 boundary — the central decision)**: Is "S5-2 = VM capability + endpoints + producer
  (VM-tested); live shell observe/deliver-call rides S5-6" the right cut, given the shell drives
  concrete boa until the flip? Or should S5-2 wire the live monitor-observe to the boa bridge now
  (C3/#415 style)? Lean: **VM-capability-only** (umbrella §8; no boa touch; the flip re-points anyway).
- **Q2 (cookieStore split)**: Confirm cookieStore stays a **separate** plan-reviewed slice
  (`#11-cookiestore-structured-spec-faithful`), not re-bundled into S5-2. Lean: **split** (distinct
  subsystem, independently edge-dense).
- **Q3 (M3 scope — narrow vs engine-wide)**: Is fixing T3/T4 for **screen + VV only** (slotting the
  pre-existing navigator/location/history looseness) the right One-issue-one-way call, or must S5-2
  converge all five now? Lean: **narrow + slot** (S5-2 uses the existing canonical form for its new
  surfaces; pre-existing debt is a separate sweep).
- **Q4 (monitor dims = separate endpoint vs `set_media_environment` param)**: Confirm
  `set_screen_dimensions` as a **separate** endpoint (device-fact, non-media, no delivery turn) vs
  overloading `set_media_environment`. Lean: **separate** (device-fact ≠ media-fact; avoids
  `MediaEnvironment` pollution + a spurious re-eval turn).
- **Q5 (not-fully-active — TWO layers)**: there are two distinct branches (webref-verified): (a) the
  §12.1 *geometry getter* step 1 → **return 0**, and (b) the §4 *attribute* `VisualViewport?` → **return
  null**. In elidex's single-document bound model the bound document is effectively always fully active,
  so both are currently unreachable — confirm S5-2 still implements both for faithfulness (getter → 0
  when no active doc; `window.visualViewport` → null when no bound/active doc), rather than asserting
  always-present, and that "Full enum?" for those rows reflects the deferred-only-on-`[Replaceable]`
  state.
- **Q6 (ViewportState home vs a ScreenState struct)**: Put `screen_*` in `ViewportState` (sibling of
  `dppx`/`color_scheme`, the agent-recommended home) or carve a small `ScreenState`? Lean:
  **ViewportState** (already the device-fact home; monitor dims are device facts).
- **Q7 (`[Replaceable]` — confirm the family-consistency call)**: spec IDL is `[SameObject, Replaceable]`
  for screen/VV (and innerWidth/scrollX/dppx). elidex implements `[Replaceable]` value-shadowing for
  **none** of them (all `WINDOW_RO_ACCESSORS` = no-setter). S5-2 installs screen/VV the **same** way
  (no-setter RO accessor + cached singleton), deferring proper `[Replaceable]` engine-wide (§9). Confirm
  this is the right One-issue-one-way call vs implementing `[Replaceable]` for screen/VV alone (a
  lone-outlier impl). Lean: **family-consistent no-setter + slot the engine-wide `[Replaceable]` gap**.

---

## §11 Workflow

plan-verify grep against `c925ff6f`/HEAD → **`/elidex-plan-review` (this memo) BEFORE impl** → impl in
the `s5-2-minor-window-parity` worktree (build on `c925ff6f`, fix T1–T4 + add producers) → un-draft
#423 → `/pre-push` (6-stage) → `/external-converge` (Codex) → squash merge. boa untouched (§8). world_id
migration stays out (umbrella §0).
