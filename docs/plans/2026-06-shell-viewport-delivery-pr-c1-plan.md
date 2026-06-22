# Shell viewport delivery — PR-C **C1** (multi-tab delivery + startup-at-real-size) plan-memo

Slot: `#11-shell-viewport-delivery` (PR-C, sub-slice **C1**) + `#11-content-startup-viewport-seed`.
Parent (PR-C sub-umbrella): `docs/plans/2026-06-shell-viewport-delivery-pr-c-plan.md` (this memo realizes
its §5 row **C1** and resolves its **Q-C3** / **Q-C5**). Grand-parent (program umbrella):
`docs/plans/2026-06-shell-viewport-delivery-plan.md`. PR-A ✅ #386, PR-B ✅ #388.

Anchors re-grepped at HEAD `26687d18` (2026-06-22).

C1's one invariant: **no top-level content thread ever resolves style, runs script, or lays out at a
guessed viewport — every content thread builds at its real content-area size from its first frame, and
a window resize fans the new size out to every tab.**

---

## §0. Decisions this memo commits to

- **D1 — viewport is a *construction input* and `run_scripts_and_finalize` is the *single
  viewport-injection point* feeding every consumer.** The single place a content thread first resolves
  cascade + runs scripts + lays out is `pipeline::run_scripts_and_finalize` (`pipeline.rs:50`), which
  today hardcodes `DEFAULT_VIEWPORT` (`:64` cascade, `:107` re-cascade, `:111` `layout_tree`). C1
  threads a `viewport: Size` **into** that function and makes it inject that one value into **all**
  viewport consumers before any script runs: (a) the **CSS cascade** (`resolve_with_compat`,
  `@media (width)`), (b) the **JS bridge** that `window.innerWidth`/`innerHeight`/`matchMedia` read
  (`runtime.bridge().set_viewport(viewport.w, viewport.h)` — the same call the per-message path makes
  at `event_loop.rs:269`; **today `run_scripts_and_finalize` never calls it**, so the bridge keeps its
  `800×600` default `bridge/mod.rs:416` while the cascade uses `1024×768` — a pre-existing split this
  unification retires), (c) `layout_tree`, (d) `PipelineResult.viewport`. The viewport flows
  `App.placement.size_logical → spawn_content_thread*(viewport) → content_thread_main*(viewport) →
  build_pipeline_*(viewport) → run_scripts_and_finalize(viewport){cascade + bridge.set_viewport +
  layout + PipelineResult}`. This **supersedes** the parent memo's "block after `ContentState::new`,
  before first `re_render`" gate — see D2 for why that gate is unsound for its own stated goal.

- **D2 — the parent memo's gate point does not achieve its stated goal; replace it.** Parent §2/§1
  proposed blocking *after* `ContentState::new` and *before* the first `re_render` "so initial scripts
  (`innerWidth`/`matchMedia`) … run at the real size." But **scripts already ran by then.**
  `content_thread_main` (`content/mod.rs:443`) calls `build_pipeline_interactive_with_network` →
  `run_scripts_and_finalize`, which evals every inline `<script>` (`pipeline.rs:88-91`) and dispatches
  `DOMContentLoaded`/`load` (`:99`) at `default_viewport`, **before** `ContentState::new`
  (`content/mod.rs:444`). A gate after `ContentState::new` would fix only the first `re_render`'s
  layout, not the initial scripts it was justified by. The only place that fixes initial-script
  `innerWidth` is *before* `run_scripts_and_finalize` runs scripts → a construction input (D1).

- **D3 — the initial top-level tab is spawned at `resumed`, not at `new_threaded*`.** The viewport for
  the initial tab is unknown until the window exists (winit `resumed`; `placement` is `None` before it
  — `app/mod.rs:245`/`929`). So C1 **defers** the initial content-thread spawn out of `new_threaded`
  /`new_threaded_url` (`app/mod.rs:378-389`/`404-411`) into `resumed` (`:904-932`), which already
  builds `placement` first (`:929`). With the window's `placement` in hand at spawn, the initial tab is
  born at its real size **by construction** — no thread block, no timeout, no handshake. (`resumed` is
  re-entry-guarded `:905`; the pending spawn is `take()`-once so a suspend→resume cycle does not
  re-spawn.)

- **D4 — seed-at-create is *subsumed* by D1; only resize fan-out remains a runtime send.** Because every
  tab (initial via D3, `window.open` `app/mod.rs:770-785`, Ctrl+T `open_new_tab` `threaded.rs:574-595`)
  is spawned with the current `placement.size_logical`, there is **no separate "seed `SetViewport` to
  the new tab" message** (the parent memo's `seed(tab)`): the tab is already correct. The *only*
  remaining producer send is the **resize fan-out** — on `Resized` (and C2's `ScaleFactorChanged`) the
  cached viewport must reach **already-running** tabs. One primitive `seed_tab_viewport(placement, tab)`
  (an associated fn, composes with a `&mut tab_manager` borrow like `wake_or_noop` `app/mod.rs:265`),
  fanned over `tabs()` by `broadcast_viewport(&self)`. This is strictly fewer moving parts than
  seed+broadcast (One-issue-one-way).

- **D5 — `send_viewport` (active-tab-only) is removed; `send_to_content` (active-tab-only) stays.** The
  active-only `send_viewport` (`app/mod.rs:830-837`) existed only to deliver the initial + resize
  viewport to the active tab. C1 retires it: initial delivery → D1 construction input; resize delivery
  → D4 `broadcast_viewport` (all tabs, because all share the window content area, so background tabs'
  `innerWidth`/`matchMedia` stay spec-correct). `send_to_content` (`:840-848`) is **unchanged and
  correct** — input events (click/key/move/wheel/IME) target *only* the active tab; they are not device
  facts and must not fan out.

- **D6 — `DEFAULT_VIEWPORT` survives only as an *explicit* value passed by window-less spawns.** It stops
  being a silent in-pipeline guess (`pipeline.rs:64/107/111`) and becomes the value test/headless
  spawns pass on purpose (`build_pipeline_interactive(html,css)` keeps it internally; the content-thread
  test helper passes it). "No silent caps" — a window-less build still has a defined size, but it is
  chosen at the call site, not buried in the builder.

- **D7 — C1 owns the *full* param threading in one slice (no C1a/C1b split); navigation rebuild is an
  in-scope fix, iframe is a distinct fact verified at impl.** `run_scripts_and_finalize`'s `viewport`
  param forces every `build_pipeline_*` and thus every caller to pass a viewport. Splitting "top-level
  with the param" from "`from_loaded`/nav/iframe still guessing `DEFAULT`" would leave a strangler (the
  param exists but some callers guess) — One-issue-one-way forbids it, so C1 threads it everywhere in
  one slice (resolves Q1). Per-caller disposition:
    - **In-content navigation** (`content/navigation.rs:130`) + main-thread navigation
      (`app/navigation.rs:95`): the plan-review **confirmed these already silently build + run post-nav
      scripts at `DEFAULT`** (the same "guessed size" defect as initial load, recurring on every
      navigation). C1 **fixes this in-scope**: read the tab's current real viewport from the *old*
      `state.pipeline.viewport` **before** reassigning `state.pipeline`, and pass it (the old pipeline
      holds the last `SetViewport` size, so this is the real content-area size, not `DEFAULT`). This is
      a free correctness win from the same one mechanism (resolves Q4).
    - **iframe** (`iframe/load.rs:159/247/309`, `iframe/thread.rs:199`): an iframe's viewport is a
      **different fact** — the iframe element's used content-box, not the window content area
      (sub-browsing-context sizing, out of C1's top-level scope). C1 passes the iframe's current
      best-known box (preserving behavior, no regression). **Impl MUST verify** whether iframes
      currently build at `DEFAULT` (the review noted `iframe/load.rs` never assigns `pipeline.viewport`
      post-build, and `iframe/thread.rs:79` sets `pipeline.viewport` but not `bridge.set_viewport`); if
      that is a latent bug, carve `#11-iframe-build-viewport` (§10) — C1 does **not** silently fix it
      (different scope) nor silently leave it wrong.
    - **inline / standalone** (`build_pipeline_interactive`, `app/navigation.rs` inline mode): pass
      `DEFAULT` explicitly (D6) — window-less, no better value.

---

## §1. Verified anchors (HEAD `26687d18`)

Producer / placement (PR-B SoT, read-only baseline):
- `ContentAreaPlacement{origin_logical,size_logical,scale_factor}` `app/mod.rs:163-189`; cached
  `App.placement: Option<_>` `:245` (`None` before first `resumed`/after `suspended` `:936`); sole
  builder `content_area_placement()` `:809-820`; `send_viewport()` (active-only, **to be removed**)
  `:830-837`; `send_to_content()` (active-only, **kept**) `:840-848`; `resumed` builds placement then
  sends `:929-931`; `Resized` rebuilds + sends `:973-974`; redraw-top rebuild `threaded.rs:139`.
  Strangler guard `viewport_tests.rs:167-230` (sole prod caller of the three primitives).

Tabs:
- `TabManager{tabs:Vec<Tab>, active_id, id_gen}` `tab.rs:82-86`; `create_tab` sets the new tab active
  `:99-110`; accessors `tabs()` `:154` / `tabs_mut()` `:159` / `active_tab()` `:135`. `Tab.channel:
  LocalChannel<BrowserToContent,ContentToBrowser>` `:33`. `BrowserToContent` is **not `Clone`** → the
  existing broadcast pattern reconstructs per recipient (`app/mod.rs:743-767` SW broadcast).
- Tab create sites: initial `new_threaded`/`new_threaded_url` → `mgr.create_tab` `app/mod.rs:382`/`409`
  (**before** `resumed`, placement absent → the reason for D3); `window.open` loop `:770-785`
  (`create_tab :783`, **after** `resumed`); `open_new_tab` (Ctrl+T) `threaded.rs:574-595`
  (`create_tab :589`, **after** `resumed`). `SwitchTab` `threaded.rs:553-569` sends only
  `VisibilityChanged` (no viewport — correct under D4: resize keeps all tabs current, so a switched-to
  tab is already at the right size).

Build path (the headline finding — D2):
- `content_thread_main` `content/mod.rs:429-455`: `build_pipeline_interactive_with_network(html,css,…)`
  `:443` → `ContentState::new` `:444` → `re_render` `:452` → `send_display_list` `:453` →
  `run_event_loop` `:454`. `content_thread_main_url` `:457-504`: `build_pipeline_from_loaded(…)` `:484`
  → `ContentState::new` `:489` → `re_render` `:501` → `run_event_loop` `:503`.
- `run_scripts_and_finalize` `pipeline.rs:50-114`: initial cascade at `default_viewport`
  `:64-71`; **inline `<script>` eval** `:88-91`; **`DOMContentLoaded`/`load`** `:99`; re-cascade
  `:103-109`; `layout_tree(default_viewport)` `:111`. ⇒ scripts + lifecycle run at the guessed size
  **inside** the builder, before `ContentState` exists.
- `build_pipeline_*` entry points, each consuming `run_scripts_and_finalize` and setting
  `PipelineResult.viewport = DEFAULT`: `build_pipeline_interactive` `lib.rs:465` (tests, no network);
  `build_pipeline_interactive_with_network` `:533` (← `content_thread_main`, `content_tests.rs:561`);
  `build_pipeline_interactive_shared` `:603` (← iframe `iframe/load.rs:309`);
  `build_pipeline_from_loaded` `:807` (← `content_thread_main_url`, in-content nav
  `content/navigation.rs:130`, main-thread nav `app/navigation.rs:95`, iframe `iframe/load.rs:159/247`,
  `build_pipeline_from_url`); `build_pipeline_from_url` `:878` (← iframe `iframe/thread.rs:199`).

Consumer of `SetViewport` (for the resize fan-out, unchanged): `event_loop.rs:265-301` — sets
`pipeline.viewport`, `bridge.set_viewport`, re-evaluates MQL, fires `resize` then MQL `change`,
`re_render` + `send_display_list` (HTML §8.1.7.3 order, inherited from PR-B).

Test reality (constrains D6): `content_thread_wake_fires_on_display_list` `content_tests.rs:11-54`
spawns a real content thread and **blocks on the initial `DisplayListReady` without ever sending
`SetViewport`** (`:33`). The window-less test helper `spawn_test_content` `content_test_support.rs:25`
injects a no-op wake. ⇒ a startup design that *withholds the first frame until a `SetViewport` arrives*
(a "defer-first-render" gate) would hang/timeout these tests; the construction-input design does not —
the window-less spawn passes `DEFAULT` and renders immediately (D6).

PR-A repaint-wake (Q-C5 input): `WakeHandle` minted at every spawn (`wake_from_proxy`/`wake_or_noop`
`app/mod.rs:252-272`); `user_event` → `request_redraw` `:894-902`; `resumed` does **not** block on the
content thread before sending — it builds placement and sends synchronously `:929-931`.

---

## §2. The ideal mechanism (first-principles)

The viewport is a **fact the content thread needs as input**, not shared mutable state to be reconciled
after the fact. The clean shape is *ownership transfer at spawn* (CLAUDE.md "Concurrency by ownership
and phases" — "ownership transfer・single writer で競合を構造的に消す", not a lock/handshake to make timing
work):

1. **Construction input (D1/D2).** `run_scripts_and_finalize` takes `viewport: Size` and, **before the
   inline-script eval loop (`pipeline.rs:88`)**, injects it into *both* viewport SoTs: the CSS cascade
   (`resolve_with_compat` → `@media (width)`) **and** the JS bridge
   (`runtime.bridge().set_viewport(viewport.w, viewport.h)` → `window.innerWidth`/`matchMedia`). Then
   the lifecycle handlers + first `layout_tree` + `PipelineResult.viewport` all use the same value. So
   an inline script reading `innerWidth` and a `@media` rule both see the real size from the first
   evaluation — not the cascade's `1024×768` / bridge's `800×600` defaults. The value rides the spawn
   call:
   `placement.size_logical → spawn_content_thread*(…, viewport) → content_thread_main*(…, viewport) →
   build_pipeline_*(…, viewport) → run_scripts_and_finalize(…, viewport)`.

2. **Deferred initial spawn (D3).** The initial tab can't carry a viewport from `new_threaded*` (no
   window yet), so `new_threaded*` stores a *pending spawn intent* and `resumed` performs the spawn
   after building `placement`. The page's network load now begins when the window is ready — the same
   ordering a real browser uses (create window → navigate), at the cost of the small parse/GPU-init
   overlap the current early spawn buys (see §6 tradeoff).

3. **Resize fan-out (D4/D5).** Already-running tabs still need the new size on a window resize. One
   primitive `seed_tab_viewport(placement, &tab)` reconstructs + sends `SetViewport{size_logical}` to
   one tab; `broadcast_viewport(&self)` fans it over `tabs()`. Wired on `Resized` (replacing the
   active-only `send_viewport`); C2 adds the `ScaleFactorChanged` call. On `resumed` no broadcast is
   needed on first run (the just-spawned tab is already correct); a *re-*resume after `suspended`
   broadcasts to the persisted tabs (Q3).

Net: the parent memo's `seed(tab)` message disappears (subsumed by #1+#2), the blocking gate
disappears (#2 makes the size known at spawn), and the only producer send left is the resize fan-out.
Fewer mechanisms, the invariant true by construction.

---

## §3. Spec coverage map (preflight hard-gate)

C1's spec surface is *value-correctness at read time* (the viewport a content thread reports/cascades
against), not a branchy algorithm — so "Step/Branch" are the read context, not algorithm steps.
**K=3 unique specs (CSSOM View, Media Queries L5, HTML), M=4 rows → single PR** (well under the
K≥4/M≥20 split heuristic; the only genuine split question is the *code-surface* Q1 in §9, deferred to
the 5-agent review, not a spec-breadth one).

| Spec section | Step / read context | Branch | Touch (code site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM View §4 Extensions to the Window Interface — `innerWidth`/`innerHeight` (`#dom-window-innerwidth`) | initial-script read at page load | n/a (single getter) | `run_scripts_and_finalize(viewport)` via `bridge.set_viewport` at build | ✓ | no (OS device fact) |
| CSSOM View §4 — `matchMedia` (`#dom-window-matchmedia`) + §4.2 `MediaQueryList.matches` | initial `mql.matches` at load | n/a | first cascade at real viewport (`resolve_with_compat`) | ✓ | no |
| Media Queries L5 §4 Viewport/Page Characteristics Media Features (§4.1 width / §4.2 height / §4.3 aspect-ratio) | initial cascade gate | width/height/aspect-ratio (all gate on viewport) | `resolve_with_compat(viewport, Medium::Screen)` `pipeline.rs:64/107` | ✓ | no |
| HTML §8.1.7.3 — update-the-rendering | resize→MQL `change` order on fan-out | n/a | consumer unchanged from PR-B (`event_loop.rs:265-301`) | ✓ | no |

### §3.1 Breadth + user-input touch audit

- **Breadth**: the only multiplicity is per-tab resize fan-out (N tabs); every other row is a
  per-spawn single value. No spec-step branch matrix (coordination PR, not algorithm impl).
- **User-input touch**: C1 reads **no** untrusted web input — the viewport is an OS/winit device fact
  + internal tab lifecycle. No trust-boundary enumeration needed.
- **Out of C1 scope (no row above, listed to bound the surface)**: CSSOM View §4 `devicePixelRatio`
  (`#dom-window-devicepixelratio`); Media Queries L5 §12.5 "the prefers-color-scheme feature"
  (`#prefers-color-scheme`); §5.1 "Display Resolution: the resolution feature" (`#resolution`) →
  **C2/C3** (C1 sends only `size_logical`, never `scale_factor`/color-scheme/dppx). (All three
  §-numbers webref-verified 2026-06-22.)

*(CSSOM View / Media Queries L5 anchors to webref-verify at plan-review per parent §11; the CSS-module
labels are not in the preflight's reverse map — expected soft-warn, not citation drift.)*

---

## §4. C1 edge-matrix

| Axis | A1 multi-tab delivery | A4 startup-at-real-size |
|---|---|---|
| **initial tab** | born active + correct (D3 spawn-at-resumed) | built at `placement.size_logical` (D1) |
| **`window.open` / Ctrl+T tab** | born at `placement.size_logical` (post-`resumed`, placement `Some`) | same path, no seed message (D4) |
| **resize while N tabs open** | `broadcast_viewport` → all `tabs()` (D4) | (already built; just re-sized) |
| **tab switch** | `VisibilityChanged` only (no viewport — already current) | n/a |
| **suspend→resume** | re-resume broadcasts to persisted tabs (Q3); first resume spawns once (D3 `take`) | initial tab already built; resume only re-sizes |
| **headless / window-less spawn (tests)** | n/a (single thread) | explicit `DEFAULT` at call site (D6) — renders immediately, no hang |
| **`placement` somehow `None` at a post-resumed create** | fall back to `DEFAULT` at the spawn site | same |

**Intersections forcing coupling:**
- A1×A4 collapse into one invariant under D1+D3: "born at the real size." The parent memo's *gate↔seed
  coupling* (gate is unsound unless the producer guarantees a seed) **dissolves** — there is no gate
  and no seed; the size is a spawn argument. (This is the structural simplification that retires
  parent Q-C3/Q-C5.)
- A4×navigation: `build_pipeline_from_loaded` is shared with in-content navigation; the `viewport`
  param threads there too (D7) → navigation rebuild also stops guessing (free win, **fixed in-scope** —
  plan-review confirmed navigation currently builds at `DEFAULT`; Q4 resolved).
- C1 ⟂ C2/C3: C1 sends only `size_logical`; `scale_factor`/color-scheme/dppx are C2/C3. Any landing
  order; main correct after C1 alone.

---

## §5. Change surface (file-level)

**Pipeline (engine-adjacent, shell crate):**
- `pipeline.rs` — `run_scripts_and_finalize(… , viewport: Size)`; replace `default_viewport` at `:64`,
  `:107`, `:111` with the param (drop the local `default_viewport` `:64`); **AND add
  `runtime.bridge().set_viewport(viewport.width, viewport.height)` immediately after the runtime is
  built (`:75`) and *before* the inline-script eval loop (`:88`)** (mirrors `event_loop.rs:269`) — this
  is the load-bearing F1 fix: without it `window.innerWidth`/`matchMedia` read the bridge's `800×600`
  default during initial scripts even though the cascade/layout are correct. `run_scripts_and_finalize`
  is the single injection point feeding cascade + bridge + layout from the one `viewport`.
- `lib.rs` — add `viewport: Size` to `build_pipeline_interactive_with_network` (`:533`),
  `build_pipeline_from_loaded` (`:807`), `build_pipeline_interactive_shared` (`:603`),
  `build_pipeline_from_url` (`:878`); each sets `PipelineResult.viewport = viewport` (was `DEFAULT`,
  `:585/657/859/…`) and forwards to `run_scripts_and_finalize`. Keep `build_pipeline_interactive`
  (`:465`, tests) signature stable by passing `DEFAULT` internally (D6) → `tests.rs` untouched.

**Content thread (shell crate):**
- `content/mod.rs` — `spawn_content_thread` / `spawn_content_thread_url` / `spawn_content_thread_blank`
  (`:380/396/411`) and `content_thread_main` / `content_thread_main_url` (`:429/457`) gain
  `viewport: Size`, forwarded to the `build_pipeline_*` call (`:443/484`). No startup gate; no
  `re_render`/`run_event_loop` reorder — first `re_render` (`:452/501`) now lays out at the real size
  because the pipeline was already built at it.

**App lifecycle (shell crate):**
- `app/mod.rs` — (a) `new_threaded`/`new_threaded_url` stop spawning; store a `pending_initial_spawn`
  (html+css | url, + the title/chrome) on `App`. (b) `resumed` (`:904-932`): after
  `self.placement = Some(content_area_placement(...))`, `take()` the pending spawn and
  `spawn_content_thread*(…, placement.size_logical)` + `create_tab`; then set the title from the now-
  present active tab (reorder the `:915-921` title block after the spawn). (c) remove `send_viewport`
  (`:830-837`) + its `resumed` call (`:931`). (d) add `seed_tab_viewport(placement, &Tab)` (assoc fn)
  + `broadcast_viewport(&self)`. (e) `Resized` (`:973-974`): `send_viewport()` → `broadcast_viewport()`.
  (f) `window.open` loop (`:780-783`): pass `self.placement…size_logical` (fallback `DEFAULT` if
  `None`) to `spawn_content_thread_url`.
- `app/threaded.rs` — `open_new_tab` (`:587-588`): pass `self.placement…size_logical` (fallback
  `DEFAULT`) to `spawn_content_thread_blank`. (`ScaleFactorChanged` is C2, not here.)

**Tests:**
- `content_test_support.rs:25` `spawn_test_content` + `content_tests.rs:23` direct spawn: pass an
  explicit `DEFAULT` (or a chosen size) viewport — proves D6 (window-less ⇒ explicit size, no hang).
- New regressions (§7).

**Navigation (D7, in-scope fix — BOTH rebuild sites):** the *same* capture-before-reassign applies at
**two** script-running rebuild paths (plan-review IMP-1 — easy to apply one and drop the other):
- content-thread nav `content/navigation.rs:130` (`handle_navigate`): `let vp =
  state.pipeline.viewport;` before `state.pipeline = build_pipeline_from_loaded(…, vp)`.
- main-thread nav `app/navigation.rs:95` (`load_url_into_pipeline`, shared by `navigate` +
  `navigate_to_history_url`): the same capture off `InteractiveState.pipeline.viewport` (inline mode's
  `PipelineResult`) before its `build_pipeline_from_loaded(…, vp)`.
`vp` = the old pipeline's last `SetViewport` size = the tab's real content area. The *new* runtime's
bridge is set by `run_scripts_and_finalize` from `vp` (F1), so post-nav `innerWidth`/`matchMedia` are
correct too. Soundness is contingent on D1/D3 landing in this same slice (they do — they make
`pipeline.viewport` reliably real at nav time, never `DEFAULT`). Confirmed by plan-review: both paths
currently rebuild at `DEFAULT` (Q4 resolved).

**iframe (D7, distinct fact — verify, do not fix in C1):** `iframe/load.rs:159/247/309`,
`iframe/thread.rs:199`, `build_pipeline_from_url` callers — pass the iframe's current best-known
content-box size (preserve behavior). **Impl MUST verify** whether iframes build at `DEFAULT` today
(`iframe/load.rs` appears not to assign `pipeline.viewport` post-build); if a latent bug, carve
`#11-iframe-build-viewport` (§10), do **not** fold the sub-browsing-context sizing fix into C1.

---

## §6. Alternatives considered (and why rejected)

- **A — block the content thread on a bounded `recv` for the first `SetViewport`, with a timeout→`DEFAULT`
  fallback** (parent memo's leaning, placed *correctly* before `run_scripts_and_finalize`). Rejected:
  (i) it is a synchronization handshake to make timing work, which the concurrency-by-ownership
  philosophy says to replace with ownership transfer (D1/D3 do exactly that); (ii) it can't parse/run
  scripts until the viewport arrives anyway, so it preserves no useful parallelism over D3; (iii) the
  timeout fallback adds latency + nondeterminism to the *many* window-less tests (§1 test reality) and
  risks flaky `DEFAULT` fallback under load; (iv) it leaves `DEFAULT` as a timing-dependent guess
  rather than an explicit call-site choice (D6).
- **B — defer the first `re_render`/`send_display_list` until the first `SetViewport` in
  `run_event_loop`** (parent Q-C3 alt). Rejected twice over: (i) it does **not** fix initial-script
  `innerWidth` (scripts ran in `build_pipeline` regardless — D2); (ii) it withholds the first frame,
  which hangs/timeouts the window-less tests that block on the initial `DisplayListReady` (§1).
- **C′ — keep the early spawn but pass viewport via a one-shot channel the thread reads before
  `build_pipeline`.** Same as A minus the timeout shape; same ownership-handshake objection; strictly
  more moving parts than D3 (a side channel + a read point) for no parallelism gain.

**Honest tradeoff of the chosen design (D3):** deferring the initial spawn to `resumed` loses the
current overlap of "HTML parse + network fetch on the content thread" against "window + GPU init on the
main thread." Mitigations / why acceptable: scripts + layout can't proceed without the viewport anyway
(only parse+fetch overlap is lost); "don't load before you can display" matches real-browser ordering;
the delta is a few ms of GPU-init on first paint only. **If the plan-review judges this regression
unacceptable, the fallback is A placed before `run_scripts_and_finalize`** — but the memo anchors on
D3 as the ideal.

---

## §7. Testing / acceptance

- **initial tab at real size**: spawn the initial tab through the `resumed` path at a non-default
  placement; assert the first `DisplayListReady`'s layout reflects the real width (not 1024), and an
  inline script that records `innerWidth` at load observes the real width. *(Drive via the content-
  thread harness with an explicit non-default viewport — the construction input is directly testable
  without a real window.)*
- **`window.open` / new tab at real size**: a post-`resumed` `create_tab` builds at the current
  placement (assert via the spawned tab's first display list / `innerWidth`).
- **resize fans out to a background tab**: two tabs, switch away from tab B, resize, switch back —
  tab B's layout reflects the new size (it received the broadcast while hidden).
- **window-less spawn renders immediately at `DEFAULT` (no hang)**: the existing
  `content_thread_wake_fires_on_display_list` keeps passing with the explicit-`DEFAULT` spawn (D6) —
  this is the no-producer regression guard.
- **navigation rebuilds at the tab's real size** (Q4 fix): after a same-tab navigation at a non-default
  viewport, the rebuilt pipeline + post-nav `innerWidth` reflect the tab's real size, not `DEFAULT`
  (guards the in-scope navigation fix in §5).
- Supported-surface: shell content-thread + unit tests (no WPT subset for the shell composite). Each
  IMPORTANT gets a regression test.

---

## §8. Collision / sequencing

- **Engine-side media (#370/#372/#378)** upstream + untouched: C1 only feeds the *cascade/script*
  viewport at build + the resize `SetViewport`; the MQL consumer is unchanged.
- **S5 boa→VM cutover**: C1 producers + the construction-input plumbing are engine-agnostic (they set
  `pipeline.viewport` / call the existing `bridge.set_viewport`); no S5 coupling. (C3 is the S5 pivot,
  not C1.)
- **Parallel sessions**: `crates/shell/elidex-shell/src/app/*` has a history of cross-session collision
  (the A2 StorageEvent slot noted a prior `shell/app/` collision). As of 2026-06-22 no *active* branch
  touches `crates/shell/` (the concurrent `b1.2b-direct-tree-ops` worktree's apparent `app/mod.rs` diff
  is a stale-base artifact vs its pre-PR-B merge-base, not a real shell touch). Regardless of which
  branches are live, the operative guard stands: **re-grep `app/mod.rs` `resumed`/`Resized`/
  `new_threaded` + `lib.rs` `build_pipeline_*` + `pipeline.rs run_scripts_and_finalize` for drift
  immediately before implementing.** Worktree `elidex-wt-pr-c` is already isolated off the branch.
- **Terminal-Z render↔layout**: C1 does not touch `elidex-render`/`elidex-layout` signatures
  (`layout_tree` is called with the param value, signature unchanged) — low collision.
- **Landing order**: C1 first (⟂ C2/C3); then C2 (`ScaleFactorChanged` geometric); then C3 (device
  facts, depends on C2's arm + the D-C4 fork).

---

## §9. Decisions (resolved at this plan-review) + remaining impl-time items

**Resolved by the 5-agent plan-review (2026-06-22):**
- **Q1 — param-threading scope → RESOLVED: one slice, no C1a/C1b split.** The `viewport` param is one
  mechanism; splitting leaves a strangler (`with_network` takes the param but `from_loaded` still
  guesses) — One-issue-one-way forbids it. C1 threads it through every `build_pipeline_*` + caller in
  one slice (D7). (Spec breadth K=3/M=4 confirms single-PR scope.)
- **Q2 — defer-spawn latency → RESOLVED: accept D3.** Decided via philosophy (ownership-transfer over
  handshake), not punted: the lost overlap is parse+fetch only (scripts/layout can't proceed pre-
  viewport), it matches real-browser ordering, and the §6 fallback (A before `run_scripts_and_finalize`)
  is documented if a real startup-latency regression surfaces in benchmarking.
- **Q4 — navigation current-viewport → RESOLVED: navigation builds at `DEFAULT` today; C1 fixes it
  in-scope.** The plan-review confirmed `handle_navigate` (`content/navigation.rs:130`) rebuilds via
  `build_pipeline_from_loaded` (→ `DEFAULT`) and never re-applies the tab's real size. C1 folds the fix
  in (capture the old `state.pipeline.viewport` before reassign; pass it — see §5 Navigation bullet).
  iframe is a distinct sub-browsing-context fact → verify-at-impl + conditional slot (§10), not folded.

**Remaining impl-time items (mechanical, not design forks):**
- **Q3 — `resumed` re-resume broadcast.** On first `resumed` the just-spawned initial tab is already
  correct; on a *re-*resume after `suspended` the persisted tabs may need the (possibly changed) size.
  Broadcast unconditionally in `resumed` (one redundant idempotent `SetViewport` on first run) vs only
  when tabs pre-existed? Lean unconditional-broadcast for path uniformity (the redundant first-run
  `re_render` is cheap); confirm at impl it doesn't double-paint the first frame.
- **Q5 — `pending_initial_spawn` shape.** Store the raw inputs (html/css | url + chrome/title +
  net handle/jar) and call `spawn_content_thread*` in `resumed`, vs store a boxed spawn closure? Memo
  leans raw inputs (no boxed-FnOnce capturing the net handle across the App field); confirm the
  borrow/ownership shape against `create_network_process`/`wake_proxy` availability in `resumed`.

---

## §10. Defer slots

- **`#11-content-startup-viewport-seed`** — **CLOSED by C1**: the construction input (D1) + deferred
  initial spawn (D3) make "content built at the real viewport from frame 1" true by construction; the
  "seed message" this slot anticipated is subsumed (D4), not implemented.
- **`#11-shell-viewport-delivery`** — **discharged by C1** for the multi-tab axis (the ledger's
  axis-2: spawn-at-real-size for every tab + resize fan-out). (Ledger axis numbering: axis-2 = multi-tab
  seed/fan-out [→ C1]; axis-3 = `ScaleFactorChanged` [→ C2]; device facts [→ C3, parent §10]. C1's own
  §4 edge-matrix labels these A1/A2/A3 — same dispositions, local labels only.)
- **No C1a/C1b split → no conditional C1b slot** (Q1 resolved: one slice). The deferred-spawn + full
  param threading complete in-slice, not deferred.
- **`#11-iframe-build-viewport`** (conditional, NOT yet registered) — carved ONLY if impl confirms
  iframes currently build at `DEFAULT` (D7 iframe bullet / §5). **Why deferred**: an iframe's viewport
  is the iframe element's used content-box (sub-browsing-context sizing), a different fact from the
  top-level window content area C1 owns; folding it in would balloon C1 past its terminal scope.
  **Scope note (plan-review IMP-2)**: C1's param threading through `build_pipeline_from_loaded`/`from_url`
  *does* give the iframe **build** paths (`iframe/load.rs:159/247`, `iframe/thread.rs:199`) a correct
  bridge-set for free (via `run_scripts_and_finalize`) once they pass their box size — so the residual
  slot work is specifically the iframe **runtime** path: `iframe/thread.rs:79` sets `pipeline.viewport`
  on `SetViewport` but **not** `bridge.set_viewport` (the same bridge/cascade asymmetry F1 fixes for
  top-level). **Re-evaluation trigger**: impl reads `iframe/load.rs`/`iframe/thread.rs:79` and confirms
  the latent default. **Re-evaluation date**: C1 impl (decided then; no pre-commitment here).
- Untouched umbrella slots `#11-forced-style-layout-flush-on-script-read` /
  `#11-hidpi-render-fidelity` remain deferred (not C1 scope).
