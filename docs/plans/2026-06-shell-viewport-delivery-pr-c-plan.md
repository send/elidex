# Shell viewport delivery — PR-C sub-umbrella (producer-delivery breadth) plan-memo

Slot: `#11-shell-viewport-delivery` (PR-C). Parent: `docs/plans/2026-06-shell-viewport-delivery-plan.md`
(umbrella, §7 row C). PR-A ✅ #386 (`d2663e6b`), **PR-B ✅ #388 (`26687d18`)** = the
`ContentAreaPlacement` coordinate-system SoT. This memo is the **PR-C sub-umbrella**: it owns
PR-C's own edge-matrix and decides the C1/C2/C3 split the umbrella §7 deferred to "PR-C's own
plan-review", and answers umbrella Q4 (prefers/dppx placement).

Anchors re-grepped at HEAD `26687d18` (2026-06-21).

---

## §0. Decisions this memo commits to

- **D-C0 — PR-C is a sub-umbrella of 3 sub-slices, NOT one PR.** PR-C bundles **four** producer
  axes (multi-tab seed/fan-out · startup-seed · ScaleFactorChanged · prefers-color-scheme+dppx) —
  ≥3 intersecting invariant axes ⇒ the edge-dense RULE (CLAUDE.md) forbids a single PR. Split into
  C1/C2/C3, each its own narrow plan-memo + `/elidex-plan-review` before impl (base-case clause:
  each is then a terminal single PR). **Not** asserted "trivial/additive" — that is the #383
  premise this program exists to retire.
- **D-C1 — One device-fact delivery chokepoint (One-issue-one-way).** Every device fact the shell
  pushes to a content thread (viewport size, color-scheme, dppx) flows through **one** producer
  path: `App` cached SoT → a broadcast/seed helper over `send_to_content`'s successor → a
  `BrowserToContent` variant → `ContentState` fact storage → the media/layout consumer. PR-C does
  **not** add ad-hoc per-fact senders or a second tab-iteration path. The active-tab-only
  `send_to_content` (`app/mod.rs:840`) is generalized **once** into seed-at-create + fan-out-to-all,
  reused by every fact.
- **D-C2 — C1 = multi-tab (Axis 1) + startup-seed (Axis 4), coupled by the delivery contract.**
  The startup gate (block a content thread's initial build until its first viewport, §Axis-4) is
  **only sound once the producer guarantees every spawned tab receives a `SetViewport`**
  (seed-at-create for initial/`window.open`/new-tab + fan-out on resize). Shipping the gate without
  the guarantee = a strangler (every spawn hangs → forced to fall back to default → gate is a
  no-op). They are one invariant ("a content thread lays out at its real content-area size from its
  first frame"), so they ship together.
- **D-C3 — dppx travels with C3 (color-scheme), NOT with C2 — answering umbrella Q4.** A DPI change
  has two independent consequences: (a) **geometric** — the compositor transform + surface size +
  input `÷scale` must use the new `scale_factor`; this is shell-internal and observable as *correct
  paint* with **no content plumbing** → **C2**. (b) **content-fact** — `window.devicePixelRatio` /
  `@media (resolution)` must update; this needs the same IPC-variant + `ContentState` fact-storage +
  consumer-evaluator widening as prefers-color-scheme → **C3**. So C2 is geometrically complete
  alone; C3 delivers dppx **and** color-scheme as content facts through the one chokepoint (D-C1).
  (Umbrella's tentative "C2 = ScaleFactorChanged+dppx" is refined: the dppx *trigger* is C2's event,
  but the dppx *fact delivery* is C3 plumbing — co-located with color-scheme, not with the geometric
  recompute.)
- **D-C4 — the device-fact CONSUMER is the canonical `elidex-css` evaluator, not a boa-local stub.**
  The active engine (boa) evaluates `@media` via `elidex-js-boa::bridge::evaluate_media_query_raw`
  (`bridge/mod.rs:774`), a boa-local reimplementation that **stubs `prefers-color-scheme => false`**
  (`:796`) and ignores dppx. The engine-independent `elidex-css::MediaEnvironment`
  (`media/types.rs`) + the VM engine's `media_environment()` already evaluate these facts
  canonically. C3's ideal is to **route boa's media evaluation through the canonical `elidex-css`
  evaluator** (retire the boa-local reimpl, One-issue-one-way) so a single evaluator serves both
  engines — NOT to grow the boa stub (which would be throwaway boa work per
  `feedback_boa-findings-light-touch`, and a second parallel evaluator). If routing-through-canonical
  proves too coupled to boa internals for a pre-S5 slice, the fallback is to **defer C3 to S5**
  (when the VM — already a canonical consumer — replaces boa), shipping the durable producer half
  only if it has a live consumer. **Hard gate (no producer-without-consumer middle state)**: C3
  MUST land producer + IPC variant + `ContentState` store **together with** a live (canonical-routed)
  consumer in one slice, OR defer the **entire** C3 slice (producer included) to S5 — a stored
  device fact with no reader is a forbidden dead-store ("dead code は接続するか削除"). **This is C3's
  central open question (Q-C2) → C3's own plan-review.** C1 + C2 do not depend on it.
- **D-C5 — no new geometric SoT.** All three sub-slices READ the PR-B `App.placement`
  (`content_area_placement()` builder); none re-reads `chrome_content_offset` / `content_size` /
  `window.scale_factor()` (the PR-B strangler guard `viewport_tests.rs:167-230` forbids it). C2
  recomputes `placement` via the existing builder on `ScaleFactorChanged`, exactly as `Resized`
  does.

---

## §1. Verified anchors (HEAD `26687d18`)

PR-B SoT (read-only baseline for PR-C):
- `ContentAreaPlacement` `app/mod.rs:163-189` (`origin_logical`/`size_logical`/`scale_factor` +
  `origin_phys()`/`size_phys()`); cached `App.placement: Option<_>` `app/mod.rs:245`; sole builder
  `App::content_area_placement()` `app/mod.rs:809-820`; producer `App::send_viewport()`
  `app/mod.rs:830-837` (sends `SetViewport{width,height}` = size_logical only); recompute at
  `resumed` `:929`, `Resized` `:973`, redraw-top `threaded.rs:139`. Strangler guard
  `viewport_tests.rs:167-230` (1 prod caller of offset/size, 2 of `scale_factor`).

Axis 1 — multi-tab:
- `App::send_to_content()` `app/mod.rs:840-848` — gates on `mgr.active_tab()` (`:842`) = the
  active-only chokepoint behind `send_viewport` + all input sends.
- `TabManager{tabs:Vec<Tab>, active_id, id_gen}` `app/tab.rs:82-86`; `Tab.channel:
  LocalChannel<BrowserToContent,ContentToBrowser>` `tab.rs:33`; accessors `tabs()` `:154` /
  `tabs_mut()` `:159` / `active_tab()` `:135`. `BrowserToContent` is **not `Clone`** → per-recipient
  reconstruction (existing pattern `app/mod.rs:743-767`).
- Tab create sites (no viewport seed today): initial `new_threaded*`→`create_tab`
  `app/mod.rs:382-409`; `window.open` `app/mod.rs:770-785` (`create_tab` `:783`); `open_new_tab`
  `threaded.rs:574-595` (`create_tab` `:589`). `SwitchTab` `threaded.rs:553-569` sends only
  `VisibilityChanged`, no viewport re-send.

Axis 2 — ScaleFactorChanged: **handler ABSENT** (grep empty, shell crate). `Resized` arm to mirror:
`app/mod.rs:960-981` (gpu.resize → request_redraw → `placement=content_area_placement` →
`send_viewport` → `reclip_cursor_after_placement_change`).

Axis 3 — prefers/dppx:
- winit `ThemeChanged`/`window.theme()` **ABSENT** (grep empty).
- `BrowserToContent` `ipc.rs:55` device-fact variants = `SetViewport` `:100` + `VisibilityChanged`
  `:141` only (no color-scheme/dppx/theme variant).
- boa consumer: `re_evaluate_media_queries(width,height)` `media.rs:46`; `evaluate_media_query_raw`
  `bridge/mod.rs:774`, `prefers-color-scheme => false` `:796`; `set_device_pixel_ratio` exists
  `viewport.rs:61` but **never called** (`window.devicePixelRatio` getter defaults 1.0).
- canonical engine-indep evaluator: `elidex-css::MediaEnvironment{resolution_dppx,color_scheme,…}`
  `media/types.rs:289-358`; cascade defaults the non-viewport facts `elidex-style/lib.rs:196-205`
  ("until shell producers light them up"); VM `media_environment()`
  `vm/host/media_query.rs:298-307` already reads them (S5-dormant path).

Axis 4 — startup-seed:
- `content_thread_main` `content/mod.rs:429-455` builds pipeline (`:443`) + `ContentState::new`
  (`:444`) + `re_render` (`:452`) + `send_display_list` (`:453`) **before** `run_event_loop` (`:454`);
  `content_thread_main_url` same shape `:457-503`. First `SetViewport` processed only inside
  `run_event_loop`→`handle_message`→`event_loop.rs:265`.
- `DEFAULT_VIEWPORT_WIDTH=1024.0` `lib.rs:321` (4 spawn sites `:515/585/656/857`); window opens at
  default `app/render.rs:24-27`. `ContentState` `content/mod.rs:44-72` has no "viewport received"
  flag.

---

## §2. The ideal mechanism (first-principles, per axis)

The PR-B SoT made the *active tab's* viewport correct. PR-C's job: make **every content thread, from
its first frame, lay out + evaluate media at the real device facts** — across the tab lifecycle, DPI
changes, and OS theme — through the single device-fact chokepoint (D-C1).

- **Delivery contract (C1)**: `send_to_content` generalizes to (i) `broadcast(msg_fn)` that sends a
  freshly-reconstructed message to **every** tab channel, and (ii) `seed(tab)` that pushes the
  cached `placement` to a newly-created tab immediately. Resize/placement-change → broadcast
  `SetViewport` to all; create/`window.open`/new-tab → seed; `SwitchTab` → already-seeded (no-op for
  viewport). With every tab guaranteed seeded, the **startup gate** is sound: a content thread, after
  `ContentState::new` and before its first `re_render`, **blocks on its channel for the first
  `SetViewport`** (bounded `recv` with a no-producer/timeout fallback to `DEFAULT_VIEWPORT` for
  headless/test spawns), so initial scripts (`innerWidth`/`matchMedia`) and the first layout run at
  the real size. One invariant: *no content thread ever lays out at a guessed size*.
- **ScaleFactorChanged (C2)**: a `WindowEvent::ScaleFactorChanged` arm mirrors `Resized` — recompute
  `placement` via the builder (new `scale_factor`), `gpu.resize` to the new physical size,
  `request_redraw`, and re-`send_viewport` (size_logical may be unchanged; the content thread
  re-lays-out / the broadcast keeps all tabs coherent). Geometric correctness only (D2/D-C3a);
  fidelity deferred (`#11-hidpi-render-fidelity`).
- **Device facts (C3)**: a `ThemeChanged` arm + the `ScaleFactorChanged` arm feed one IPC
  device-fact path (a `SetColorScheme`/`SetDevicePixelRatio` pair, or a unified `SetDeviceFacts` —
  Q-C1) → `ContentState` stores the facts → the media consumer evaluates `@media
  (prefers-color-scheme | resolution)` against them via the **canonical `elidex-css` evaluator**
  (D-C4). One invariant: *every device-fact `@media` reflects the live OS/display state through one
  evaluator*.

---

## §3. Spec coverage map (preflight hard-gate)

| Spec | Surface | PR-C consumer | sub-slice | breadth |
|---|---|---|---|---|
| CSSOM View §4 Extensions to the Window Interface | `innerWidth`/`innerHeight` (`#dom-window-innerwidth`) read at startup + per tab | startup-seed + fan-out give the real size before first script | C1 | per-tab |
| CSSOM View §4 | `devicePixelRatio` (`#dom-window-devicepixelratio`) = `scale_factor` | dppx fact delivery + consumer | C3 | 1 fact |
| CSSOM View §4 `matchMedia` (`#dom-window-matchmedia`) + §4.2 MediaQueryList / §4.2.1 Event summary (MQL `change`) | re-evaluate + fire `change` on viewport/dppx/theme change | C1(size)/C3(dppx,scheme) | reuse #370/#372/#378 (inherited) |
| Media Queries Level 5 §12.5 prefers-color-scheme (`#descdef-media-prefers-color-scheme`) | OS theme `@media` | `ThemeChanged` producer + canonical evaluator | C3 | 1 fact |
| Media Queries Level 5 §5.1 resolution (`#descdef-media-resolution`, dppx) | display `@media` | `ScaleFactorChanged` dppx fact + evaluator | C3 | 1 fact |
| HTML §8.1.7.3 update-the-rendering | resize→MQL order on each delivery | inherited from PR-B (D5a) — unchanged | — | — |

**Breadth audit**: C1 touches per-tab delivery (N tabs) — the only multiplicity. C2/C3 are
per-window single-fact. **User-input touch audit**: none of PR-C reads untrusted web input — all
inputs are OS/winit device facts (theme, scale, window size) + internal tab lifecycle. No
trust-boundary enumeration needed.

*(mediaqueries-5 §12.5 / §5.1 webref-verified 2026-06-21; not load-bearing for C1/C2.)*

---

## §4. PR-C edge-matrix (the four axes × intersections)

| | A1 multi-tab | A4 startup-seed | A2 ScaleFactorChanged | A3 prefers/dppx |
|---|---|---|---|---|
| **shares `send_to_content` chokepoint** | generalizes it | consumes the guarantee | re-sends via it | sends new facts via it |
| **reads PR-B `placement`** | broadcast source | seed source | **recomputes** it | dppx source = `scale_factor` |
| **new `BrowserToContent` variant** | reuse `SetViewport` | reuse `SetViewport` | reuse `SetViewport` | **new variant(s)** |
| **`ContentState` change** | — | **+viewport-received gate** | — | **+fact storage** |
| **new `window_event` arm** | `SwitchTab` (exists) | — | **`ScaleFactorChanged`** | **`ThemeChanged`** |
| **crosses into engine crate** | no | no | no | **yes (consumer evaluator)** |

**Intersections that force coupling / ordering:**
- A1×A4: the startup gate's soundness *depends on* A1's seed-at-create guarantee (D-C2) → **same
  slice C1**.
- A2×A3(dppx): share the `ScaleFactorChanged` trigger but split by consequence (geometric=C2 vs
  fact=C3, D-C3); C3's dppx fact is emitted from the same arm C2 adds → **C3 depends on C2** (or C2
  adds the arm and C3 extends it; sequence C2→C3).
- A3(scheme)×A3(dppx): share IPC-variant + `ContentState` storage + canonical-evaluator routing →
  **same slice C3** (One-issue-one-way; one device-fact path).
- A1/A4 (C1) are independent of A2 (C2) and A3 (C3) → C1 ⟂ {C2→C3}; any order, main correct after
  each.

---

## §5. Decomposition decision (C1 / C2 / C3)

| Sub-slice | Scope | Depends | Edge-density → plan-review | Why a clean boundary (main stays correct) |
|---|---|---|---|---|
| **C1** | multi-tab seed-at-create + fan-out-on-resize (A1) **+** startup-viewport gate (A4) | PR-B | 2 coupled axes on one delivery-contract invariant → **own `/elidex-plan-review`** (the startup-gate lifecycle change has its own edge matrix: headless/test no-producer fallback, gate↔seed coupling, bounded recv/timeout). | After C1: every tab (initial/`window.open`/new/background) lays out at the real size from frame 1. Before C1: new tab at default = no-regression (PR-B status quo). |
| **C2** | `WindowEvent::ScaleFactorChanged` → recompute `placement` + `gpu.resize` + repaint + re-`send_viewport` (A2 geometric only) | PR-B | 1 axis, mirrors the existing `Resized` arm → **base-case** (narrow; still gets a confirming plan-review per the RULE, expected light). | After C2: DPI change repaints geometrically correct (compositor/input use new scale). Before C2: a no-Resized DPI change keeps stale scale until next resize = no-regression (rare path). Content-JS dppx unaffected (that is C3). |
| **C3** | `ThemeChanged` + dppx producers → new `BrowserToContent` device-fact variant(s) + `ContentState` fact storage + **canonical `elidex-css` media evaluator** routing (A3 = prefers-color-scheme **and** dppx fact) | PR-B, **C2** (shares `ScaleFactorChanged` arm for the dppx trigger) | cross-crate (shell + engine consumer) + new IPC + evaluator-routing decision (D-C4) → **own `/elidex-plan-review`**; may sub-split C3a (color-scheme/`ThemeChanged`) / C3b (dppx, extends C2's arm) if the evaluator-routing scope warrants. | After C3: prefers-color-scheme + `@media (resolution)` flip on the live OS/display fact. Before C3: both evaluate to the default (Light, 1dppx) = no-regression (status quo). |

**Landing order**: C1 and C2 are independent (either first). C3 depends on C2 (dppx trigger arm) +
is the heaviest/riskiest (cross-crate + the D-C4 fork) → last, after its own plan-review resolves
Q-C2. Recommended: **C1 → C2 → C3**.

Each sub-slice: own worktree off `origin/main`, own narrow plan-memo (this memo is the PR-C
sub-umbrella; C1/C2/C3 each cite it), `/elidex-plan-review` before impl, pre-push 6-gate,
`/external-converge`.

---

## §6. Change surface per sub-slice (file-level)

**C1** (shell-only):
- `app/mod.rs` / `app/tab.rs` — generalize `send_to_content` → add `broadcast(BrowserToContent-fn)`
  over `tabs_mut()` (per-recipient reconstruction, msg not `Clone`) + `seed_tab(tab)` pushing cached
  `placement`; call `seed` at the 4 production `create_tab` sites (`app/mod.rs:382/409/783`,
  `threaded.rs:589`; verified 2026-06-21 via `grep -rn '.create_tab('`, `tab.rs:226` test-only); fan-out
  `SetViewport` on `Resized`/`ScaleFactorChanged` recompute.
- `content/mod.rs` — startup gate between `ContentState::new` and first `re_render` (`:445-453` /
  `:490-501`): bounded `recv` for first `SetViewport` + no-producer fallback to `DEFAULT_VIEWPORT`;
  `ContentState` (`:44-72`) gains a "viewport seeded" flag.
- tests: new-tab-seeded; resize-fans-out-to-background-tab; startup-gate-blocks-then-builds-at-real-
  size; headless-no-producer-falls-back (no hang).

**C2** (shell-only):
- `app/mod.rs` — `WindowEvent::ScaleFactorChanged` (NEW; winit variant, unhandled today) arm
  (mirror `Resized` `:960-981`); confirm winit `inner_size_writer` / surface physical-size handling.
- tests: ScaleFactorChanged → placement.scale_factor updated + request_redraw + (fanned-out)
  re-send; compositor geometry correct at the new scale (reuse PR-B scale-2 geometry test shape).

**C3** (shell + engine consumer):
- `app/mod.rs` — `WindowEvent::ThemeChanged` (NEW; winit variant, unhandled today) arm →
  color-scheme fact; dppx fact from C2's `ScaleFactorChanged` arm.
- `ipc.rs` — new `BrowserToContent` device-fact variant(s) (Q-C1: `SetColorScheme`+`SetDevicePixel
  Ratio` vs unified `SetDeviceFacts`).
- `content/mod.rs` / `event_loop.rs` — `ContentState` fact storage + dispatch on the new variant →
  re-evaluate media.
- consumer (Q-C2/D-C4): route boa media evaluation through canonical `elidex-css::MediaEnvironment`
  (retire `evaluate_media_query_raw` stub) **or** defer to S5.
- tests: prefers-color-scheme `@media` flips on `ThemeChanged`; `@media (resolution)` flips on dppx
  change.

---

## §7. Testing / acceptance (per umbrella §8 "PR-C")

- C1: new/`window.open` tab receives the current viewport on creation; a resize fans out to a
  hidden tab; startup script reads the real `innerWidth` (gate); headless spawn does not hang.
- C2: a `ScaleFactorChanged` re-sends + repaints geometrically correct at the new scale.
- C3: prefers-color-scheme / `dppx` `@media` flip on the respective device-fact change.
- Supported-surface: shell/integration contracts → shell content-thread + unit tests (no WPT subset
  claimed for the shell composite). Each IMPORTANT gets a regression test.

---

## §8. Collision / sequencing

- **Engine-side media (#378/#370/#372) upstream + untouched** for C1/C2; C3's consumer-routing
  touches the boa evaluator + reads (not writes) `elidex-css` MediaEnvironment.
- **S5 boa→VM cutover**: C1/C2 producers are engine-agnostic (drive IPC) → no S5 coupling. **C3's
  consumer half is the S5 pivot** — D-C4/Q-C2 explicitly weighs route-canonical-now vs defer-to-S5.
- **Parallel sessions**: A2 storage / B1.2b / A3 cookie / focus / media are active. PR-C touches
  `crates/shell/elidex-shell/src/app/*` + `content/*` + `ipc.rs` (+ C3: `elidex-js-boa/bridge/*`).
  Check for shell `app/mod.rs` collisions before each sub-slice (the A2 StorageEvent slot
  `#11-storage-event-mode-aware-delivery` noted a prior `shell/app/` collision). Each sub-slice in
  its own worktree off `origin/main`.
- **Terminal-Z render↔layout**: PR-C does not touch `elidex-render` signatures (PR-B did); low
  collision.

---

## §9. Open questions for `/elidex-plan-review`

- **Q-C1** — C3 IPC shape: separate `SetColorScheme` + `SetDevicePixelRatio` variants vs a unified
  `SetDeviceFacts{color_scheme, dppx, …}`? (Memo leans unified — one device-fact message matches the
  one-chokepoint principle D-C1 and extends cleanly to future facts; but separate variants are
  simpler to dispatch incrementally. Decide at C3's plan-review.)
- **Q-C2 (central, C3)** — consumer routing (D-C4): route boa media evaluation through the canonical
  `elidex-css` evaluator now (ideal, retires the boa stub, One-issue-one-way) vs widen the boa stub
  (throwaway, `feedback_boa-findings-light-touch`) vs **defer C3 to S5** (VM is the canonical
  consumer; ship the producer with its consumer, not before). Memo leans route-canonical *if* the
  boa↔`elidex-css` seam is clean; else defer C3 to S5. **Resolve at C3's own plan-review** with a
  read of the boa media path.
- **Q-C3** — startup gate (C1): bounded `recv` with a timeout fallback vs a non-blocking
  "first-frame-deferred-until-first-SetViewport" flag (build pipeline but defer `re_render` until the
  first viewport, no thread block)? The non-blocking variant avoids any hang risk but complicates the
  first-frame ordering. (Memo leans bounded-recv-with-fallback for simplicity; the no-producer
  fallback covers headless/test.)
- **Q-C4** — is C2 truly base-case (single arm), or does the winit `inner_size_writer` /
  surface-resize-on-scale-change interaction make it edge-dense enough for a fuller plan-review?
- **Q-C5** — does C1's startup gate interact with PR-A's repaint-wake (a content thread blocked on
  first `SetViewport` must still be wakeable / not deadlock the spawn handshake)? Confirm the gate
  blocks only the *initial build*, not message processing.

---

## §10. Defer slots

PR-C's scope IS the discharge of the 3 PR-C-scope slots registered at PR-B landing (#388) in
`project_open-defer-slots`: `#11-shell-viewport-delivery` axis-2 (→ C1), axis-3 (→ C2 geometric +
C3 dppx-fact), `#11-content-startup-viewport-seed` (→ C1). The 2 umbrella slots
`#11-forced-style-layout-flush-on-script-read` + `#11-hidpi-render-fidelity` remain deferred (not
PR-C scope; their own triggers).

- **`#11-media-prefers-features`** (umbrella §7/D6 commits PR-C to retire it; still open in the
  ledger) — **discharged by C3**: C3 delivers prefers-color-scheme (+dppx) as live content facts
  (D-C4). It **closes when C3 lands with a live consumer**; if C3's consumer routing defers to S5
  (Q-C2), this slot **defers *with* C3 to S5** (not independently). C3's plan-review records the
  final disposition.
- **`#11-device-fact-media-consumer-canonicalization`** (conditional, NOT yet registered) — carved
  ONLY if C3's canonical-evaluator routing (Q-C2) is deferred to S5. **Why deferred**: the
  boa↔`elidex-css` media-evaluator seam may be too coupled to boa internals for a clean pre-S5
  routing. **Re-evaluation trigger**: C3's plan-review elects defer-to-S5 over route-canonical-now.
  **Re-evaluation date**: C3 slice start (decided at C3's plan-review; no pre-commitment here).

---

## §11. Citation appendix (to webref-verify at sub-slice plan-review)

- CSSOM View Module Level 1 — §4 Extensions to the Window Interface: `innerWidth`
  `#dom-window-innerwidth`, `devicePixelRatio` `#dom-window-devicepixelratio`, `matchMedia`
  `#dom-window-matchmedia`.
- Media Queries Level 5 — `prefers-color-scheme` §12.5 (`#descdef-media-prefers-color-scheme`),
  `resolution` (dppx) §5.1 (`#descdef-media-resolution`) — webref-verified 2026-06-21.
- HTML §8.1.7.3 update-the-rendering — resize↔MQL order inherited from PR-B (unchanged).
