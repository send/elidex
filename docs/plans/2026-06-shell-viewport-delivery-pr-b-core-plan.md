# Shell viewport delivery — PR-B core: `ContentAreaPlacement` SoT (plan-memo)

**Umbrella**: `docs/plans/2026-06-shell-viewport-delivery-plan.md` (plan-reviewed clean
2026-06-21, 0C/0I). This is the **PR-B** focused plan-memo (umbrella Q6 → PR-B carries the
SoT design, so it gets its own plan-memo + `/elidex-plan-review`). **PR-A** (async
repaint-wake) ✅ MERGED #386 (`d2663e6b`).

**Scope (atomic)**: umbrella **invariant axis 1** (coordinate system: size↔origin↔offset↔scale)
+ **axis 5a** (consumer resize↔MQL order). These two ship **together** — the producer must
not land without the matching paint-offset (else the #716 strangler the program exists to
retire). **Out of PR-B**: axis 2 (multi-tab), axis 3 (`ScaleFactorChanged`), axis 5b
(forced-reflow-on-read) → PR-C / named slots (§9).

**Base / worktree**: branch `shell-viewport-pr-b` off `origin/main` `d2663e6b` (post-PR-A),
worktree `/Users/kazuaki/repos/send.sh/elidex-wt-pr-b`. All anchors below re-verified at
`d2663e6b` (the umbrella verified at `c801ef6c` = pre-PR-A; PR-A touched these exact files, so
§1 supersedes the umbrella's line numbers where they drifted — flagged inline).

---

## §0. Decisions this memo commits to

Inherits umbrella D1–D6; PR-B commits to the umbrella's *memo-preferred* answers for the open
questions it owns, and re-states them as binding PR-B decisions:

- **B-D1 (= umbrella D1 / Q1)** — **Compositor = render-transform seam, not blit sub-rect.**
  Generalize the Vello scene build to accept a base `Affine` + content-area clip; paint the
  content DL with `content_transform = translate(origin_phys) ∘ scale(scale_factor)`, clipped
  to the content-area physical rect. **Reuse the existing `build_scene_with_transform`
  seam** (the iframe-offset `SubDisplayList` precedent, `vello_backend.rs:702-731`) — not a
  parallel `TextureBlitter` offset path. Rationale: unifies with the iframe-offset transform
  (One-issue-one-way), and the clip handles content overflow under the chrome for free. (Q1
  blit-cost counter-argument deferred to plan-review; memo sees no Vello cost case — the
  transform is one extra `Affine` multiply at the scene root.)

- **B-D2 (= umbrella D2 / Q3)** — **Parameterize scale *now*; defer only HiDPI *fidelity*.**
  The `IDENTITY` baked at `vello_backend.rs:325` becomes `scale_factor`. PR-B delivers
  *geometric* scale correctness (content painted at `size_phys = size_logical × scale`, input
  `÷ scale`) at scale 1 **and** 2. Sub-pixel text / hairline snapping / fractional-dppx
  resampling = **named follow-on slot** (`#11-hidpi-render-fidelity`, §9) — PR-B refuses to
  re-bake `scale == 1` (that is the bug this program retires) but does not chase render
  fidelity beyond geometry.

- **B-D3 (= umbrella D3)** — **Content stays device-agnostic.** The producer sends
  `size_logical` (CSS px); `scale_factor` never crosses the `BrowserToContent` IPC. dppx is a
  PR-C device-fact producer (D6), not a PR-B concern. Keeps the content thread / cascade /
  layout in one coordinate space (CSS px).

- **B-D4 (= umbrella D5a, the axis-5 half PR-B owns)** — **Swap consumer order to
  resize→MQL.** HTML "update the rendering" (`#update-the-rendering`) **step 8 = run the
  resize steps** < **step 10 = evaluate media queries and report changes** (webref-verified
  2026-06-21, §3). Current `event_loop.rs` fires MQL `change` *before* the `resize` event
  (§1.5) — reversed. Fix = resize first.

- **B-D5 (= umbrella D5b / Q2)** — **Forced-reflow-on-read is OUT of PR-B.** PR-B fixes only
  the resize↔MQL *event order*; the general "a script layout/computed-style read inside a
  handler must force a style/layout flush" is engine-wide, not viewport-specific → named slot
  `#11-forced-style-layout-flush-on-script-read` (§9). Growing it here is the reactive-mechanism
  anti-pattern (`memory/feedback_review-fix-philosophy-first`).

- **B-D6 — Single cached `App.placement` field, not a re-reading accessor (umbrella F1+F9).**
  `ContentAreaPlacement` is a per-frame **cached field** on `App`, recomputed **once** (redraw
  top + each device-fact event) via the sole `App::content_area_placement()` builder. The
  builder is the **only** caller of `chrome_content_offset` + `content_size` +
  `window.scale_factor()` (egui-init scale read at `render.rs:66` excepted). A grep-guard test
  pins this. Caching (not a re-reading accessor) snapshots the three primitives **atomically**
  (one `scale_factor` read/frame) so two consumers in one frame cannot read across a device
  event and desync click-vs-paint (F9).

---

## §1. Verified anchors (re-grepped at `d2663e6b`, post-PR-A, 2026-06-21)

> ⚠ Drift vs umbrella §1 (verified pre-PR-A `c801ef6c`): corrections flagged **[DRIFT]**.
> Each structural reference below is Verified-via-Read + grep
> (`feedback_plan-memo-pre-verify-grep`).

**A. `App` + redraw — `crates/shell/elidex-shell/src/app/mod.rs`**
- `App` struct `mod.rs:167-198`. **No** scale/viewport/placement field today (PR-B adds
  `placement`). `RenderState` (`mod.rs:136-150`) holds `window: Arc<Window>` (137),
  `surface` (142), `gpu: GpuContext` (143), `renderer: VelloRenderer` (144),
  `blitter: TextureBlitter` (145), egui trio (146-148).
- `wake_proxy: Option<EventLoopProxy<WakeEvent>>` `mod.rs:197` (PR-A). `user_event` →
  `request_redraw` `mod.rs:802-810`. (PR-B builds on the same redraw handler; wake untouched.)
- `chrome_content_offset(position) -> Point` **`chrome.rs:247-253`** (crate-root `chrome.rs`,
  `lib.rs:17` — **[DRIFT]** umbrella implied `app/chrome.rs`; real path `src/chrome.rs`).
  Returns content-area top-left in **logical px**. Sole non-test call site `threaded.rs:50`.
- `send_to_content(msg)` `mod.rs:748-756` — **active-tab only** (PR-C generalizes; PR-B uses
  active-tab for the single-tab producer). `tab_bar_position()` `mod.rs:759-766`.
- **`content_size` ABSENT** (carry-forward, §4). **`send_viewport` ABSENT** — the browser
  **never** sends `SetViewport` today; `Resized` (`mod.rs:860-871`) only does
  `gpu.resize` + `request_redraw`. The whole content-size→`SetViewport` plumbing is unbuilt.
- **No `about_to_wait`** **[DRIFT]**. Redraw seam = `window_event` (`mod.rs:839-883`) →
  `RedrawRequested` → threaded `handle_redraw_threaded` (`threaded.rs:117-187`). Drains content
  msgs (118) before paint → wake→redraw→drain→paint holds (PR-A contract).
- `window.inner_size()` read `render.rs:51` (init), `window.scale_factor()` `render.rs:66`
  (egui init). Per-frame surface dims come from `gpu.surface_config.{w,h}` (`render.rs:223-224`).

**B. Render/composite — `crates/shell/elidex-shell/src/app/render.rs`**
- `with_frame<T>` `render.rs:218-295`: reads surface size `223-224`, renders DL to an
  **intermediate texture** via `renderer.render(device, queue, dl, width, height)` `231-237`
  (**no transform passed**), then **blits full-surface at (0,0)** via `blitter.copy(...)`
  `render.rs:275-277`. egui chrome draws **on top** (`LoadOp::Load` `render.rs:194`,
  `egui_ctx.run_ui` `283-287`). → today chrome *overlays* the top of a full-window content blit.
- `surface_config.{w,h}` **READ** `render.rs:223-224` + `render.rs:152-153` (egui
  `ScreenDescriptor`). **WRITE = `gpu.rs:25-26`** (`GpuContext::resize`, called `mod.rs:864-866`)
  **[DRIFT]** — umbrella's "render.rs:223-224 = the write" is wrong; 223-224 is the read.
  F8 invariant (surface-fact = full-window physical; content extent/origin from `placement`)
  unchanged in substance; the *write* anchor is `gpu.rs:25-26`.
- `handle_redraw_with_tabs` `render.rs:300-321` (threaded). `handle_redraw` `render.rs:328-338`
  (legacy/inline).

**C. Vello backend — `crates/core/elidex-render/src/vello_backend.rs`**
- `VelloRenderer::render(&mut self, device, queue, display_list, width, height) -> Result<Texture>`
  `vello_backend.rs:71-119` — **takes no transform**; calls `build_scene` (85).
- `build_scene(scene, dl, font_cache)` `vello_backend.rs:320-326` → calls
  `build_scene_with_transform(.., Affine::IDENTITY)` at **`:325`** (umbrella anchor correct).
- `build_scene_with_transform(scene, dl, font_cache, base_transform: Affine)`
  `vello_backend.rs:333-767` (**private**), seeds `transform_stack = vec![base_transform]`
  `:373`. **Reuse seam**: `DisplayItem::SubDisplayList` `702-731` already builds a `translate`
  Affine from an iframe offset (715-718) and recursively calls `build_scene_with_transform`
  under a clip layer (`:729`) — the existing "base Affine for an offset subtree" precedent.

**D. Input mapping — `crates/shell/elidex-shell/src/app/threaded.rs`**
- `offset = chrome::chrome_content_offset(position)` computed once `threaded.rs:50` (logical px).
  Cursor positions are **physical px** straight from winit. Three mapping sites, all
  `cursor(physical) − offset(logical)` with **no ÷scale** (§1.2 unscaled bug):
  `handle_cursor_move_threaded:199`, `handle_mouse_press_threaded:219`,
  `handle_mouse_wheel_threaded:253-257`.

**E. Consumer order — `crates/shell/elidex-shell/src/content/event_loop.rs`**
- `BrowserToContent::SetViewport { width, height }` arm `event_loop.rs:265-287`. Current order:
  (1) set `pipeline.viewport` `267`; (2) `bridge.set_viewport` `268-269`; (3) **MQL change**
  `re_evaluate_media_queries` + `dispatch_media_query_changes` `271-274`; (4) **resize event**
  `276-282`; (5) restyle `state.re_render(); send_display_list()` `284-285`. → **MQL before
  resize = REVERSED** vs §3. (`dispatch_media_query_changes`→`runtime.deliver_media_query_changes`,
  `content/mod.rs:512-519`.)

**F. `chrome.rs`** — exists at crate root. Constants `CHROME_HEIGHT=36.0` (`:11`),
`TAB_BAR_HEIGHT=28.0` (`:14`), `TAB_SIDEBAR_WIDTH=200.0` (`:17`). `content_size` **absent**
(no tab-bar-height accessor beyond the constants).

---

## §2. The SoT mechanism — `ContentAreaPlacement`

### 2.1 Three coordinate spaces, one descriptor (umbrella §2.1)

| Space | Unit / origin | Owners (today) |
|---|---|---|
| Content / CSS | logical CSS px, origin = content-area top-left | content thread, cascade, `@media`, layout, DL, `clientRect`s, DOM-delivered input |
| Window-logical | logical px, origin = window top-left | egui chrome, winit logical coords |
| Physical surface | device px, origin = surface top-left | wgpu surface, Vello target, blit |

Relations: `content = window_logical − chrome_offset_logical`; `physical = window_logical × scale`.

```
ContentAreaPlacement {
    origin_logical: Point,   // chrome_content_offset(position)
    size_logical:   Size,    // content_size(win_logical.width, win_logical.height, position)  [carry-forward §4]
    scale_factor:   f32,     // window.scale_factor()
}
// derived:
//   origin_phys       = origin_logical × scale_factor
//   size_phys         = size_logical   × scale_factor
//   content_transform = translate(origin_phys) ∘ scale(scale_factor)   // CSS-px DL → physical surface
//   content_clip_phys = Rect{ origin_phys, size_phys }
```

`window_logical_size = window.inner_size() ÷ scale_factor` (winit `inner_size` is physical).

### 2.2 The three consumers (all read `self.placement`)

1. **Producer** (`SetViewport`): send `placement.size_logical` (CSS px) to the active content
   thread (B-D3). New code — the browser sends no `SetViewport` today (§1.A).
2. **Compositor** (paint): render the content DL with `placement.content_transform`, clipped
   to `placement.content_clip_phys` (B-D1). The chrome region stays `base_color` until egui
   draws on top (unchanged egui path). **This is the #716 fix** (origin now matches the size
   content was told) **and** the scale fix (`IDENTITY` gone).
3. **Input mapper**: `content_css = (cursor_physical ÷ scale_factor) − origin_logical` (B-D2),
   replacing the §1.D `cursor_physical − offset_logical`. Exact inverse of #2.

> **Scale provenance (Axis 4)**: the `× scale` (compositor) / `÷ scale` (input) the descriptor
> embodies is the CSS-px↔device-px ratio that CSSOM View §4 `devicePixelRatio`
> (`#dom-window-devicepixelratio`) *exposes to content* — but PR-B implements only the
> **shell-internal compositor/input geometry** from a winit/OS device fact (`scale_factor`); it
> does **not** implement or expose the `devicePixelRatio` IDL surface (that is the PR-C dppx
> producer, B-D3/D6). So the §3 `devicePixelRatio` row is correctly marked PR-C — PR-B's
> geometry is the same ratio applied internally, no web-facing spec algorithm uncited.

### 2.3 Single cached field + atomicity (umbrella F1/F8/F9 — B-D6)

- `App.placement: ContentAreaPlacement` **cached field**, **seeded at `resumed`** (window create,
  the same point `render_state` is set — placement cannot exist without a window) and recomputed
  at redraw top + on each device-fact event (`Resized`; PR-C adds
  `ScaleFactorChanged`/`ThemeChanged`/tab switch). Sole builder `App::content_area_placement()` =
  the **only** caller of `chrome_content_offset` + `content_size` + `window.scale_factor()`
  (egui-init `render.rs:66` excepted). Grep-guard test pins each primitive to the one builder
  caller **and** `self.placement` as the only content-area size/origin source.
  - **Init ordering invariant (sub-check 2b, dry-run gap 1)**: input events read `self.placement`,
    and `window_event` only gates on `render_state.is_some()` (`mod.rs:847`) — so a cursor event
    arriving after `resumed` but **before** the first `RedrawRequested` (pointer already over the
    window at resume) would read an unbuilt placement. The `resumed` seed closes this: placement is
    built **with** `render_state` (`mod.rs:812-832`), so any input that passes the
    `render_state.is_some()` gate sees a built placement by construction. (Field is non-`Option`
    once `render_state` exists; the two are set together.)
- **Surface fact ≠ content fact (F8)**: `gpu.surface_config.{w,h}` (written `gpu.rs:25-26`)
  stays the **full-window physical** render-target/blit extent (chrome-inclusive) — a
  *different fact*, never the content size. Per-frame coherence: `surface_config` (from the
  single `Resized` site) and `placement` (from `window.inner_size()` at redraw top) both
  re-derive from the same winit physical size, and the synchronous redraw cannot interleave a
  `Resized` between them. The §6 geometry test guards target↔content coherence.
- **Why caching, not an accessor (F9)**: an accessor would let consumer #2 (paint) and #3
  (next input) read `scale_factor` across a device event mid-frame → click maps to a different
  scale than the paint = desync. One snapshot/frame removes the race by construction.

### 2.4 ECS-native / layering check

- `ContentAreaPlacement` is **browser-process (shell) owned device state** — not per-DOM-entity
  content state → correctly a **shell-local value, NOT an ECS component** (the side-store→component
  rule's "shared cross-cutting state / device fact" exception; `feedback_boa-hostbridge-port-is-not-a-registry`
  is about per-entity Send+Sync state, which this is not).
- **No new algorithm in `vm/host/`** — Layering mandate untouched (all shell/render).
- Compositor change lives in `elidex-render` (engine-independent) + the shell call site; the
  transform generalization is a **pure extension of the existing `build_scene_with_transform`
  seam** (§1.C), not a new path. `elidex-render` gains no shell/winit dependency (it receives
  an `Affine` + clip `Rect`, both already in its vocabulary).

---

## §2b. Edge matrix — the coupled invariants *within* PR-B

PR-B is the edge-dense core (`feedback_coupled-invariant-design-corner`: ≥3 invariants couple
at one corner → enumerate the intersections explicitly). The four intersecting invariants and
the single descriptor that ties them:

| # | Invariant | Held by | Breaks if … |
|---|---|---|---|
| I1 | content told size = content painted size | producer `size_logical` ↔ compositor `size_phys = size_logical×scale` | producer sends a size the compositor doesn't paint to (the #716 class — #383 changed only the producer) |
| I2 | content painted origin = chrome-reserved offset | compositor `translate(origin_phys)` ↔ chrome egui draw region | content blits full-surface at (0,0) while chrome overlays the top → top of page hidden + blank bottom strip |
| I3 | click maps to the inverse of the paint transform | input `(p÷scale)−origin` ↔ compositor `translate(origin_phys)∘scale` | input uses `p−offset` unscaled (§1.D) → clicks land off-target at scale≠1 or wrong origin |
| I4 | resize observable before MQL `change` | consumer order (event_loop) ↔ HTML step 8<10 | MQL fires first (§1.E) → a `resize` handler reading `matchMedia` sees the new size but the MQL listener already ran on the old |

**The corner**: I1∩I2∩I3 all derive from one `ContentAreaPlacement` (size↔origin↔scale), so
they are **one invariant, not three to hand-sync** — that is the structural point of the SoT
(routing all three through one descriptor). I4 is orthogonal (event ordering, not geometry) but
ships in PR-B because it is the axis-5a half and touches the same SetViewport round-trip.
Cross-product to test (§6): {scale 1, scale 2} × {origin: Top/Left/Right chrome} × {paint
geometry, input round-trip} + the I4 order assertion.

---

## §3. Spec coverage map (preflight hard-gate)

Per `feedback_plan-scope-re-evaluation`. Anchors webref-verified 2026-06-21 (the §8.1.7.3 step
order re-verified by this memo, not trusted from the umbrella — `body html update-the-rendering`).

| Spec section | Step / member | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.7.3 Processing model (`#event-loop-processing-model`) | **step 8** run the resize steps [CSSOMVIEW] | resize **before** MQs | consumer order swap, `event_loop.rs:271-282` (B-D4) | ✓ | no |
| WHATWG HTML §8.1.7.3 Processing model (`#event-loop-processing-model`) | **step 10** evaluate media queries and report changes [CSSOMVIEW] | MQL `change` **after** resize | consumer order swap, same site | ✓ | no |
| CSSOM View §4 Extensions to the Window Interface (`#dom-window-innerwidth`) | `innerWidth`/`innerHeight` read | content CSS viewport size = painted content area | producer `SetViewport` size + compositor agreement (I1) | ✓ | no |
| CSSOM View §13.3 Event summary (`#eventdef-window-resize`) | `resize` event fire | non-bubbling, on size change | already fired `event_loop.rs:276-282`; **order** fixed (I4) | ✓ | no |
| CSSOM View §4 Extensions to the Window Interface (`#dom-window-devicepixelratio`) | `devicePixelRatio` read | = `scale_factor` device fact | **PR-C** dppx producer (D6) — not PR-B | n/a | no |

> §-number provenance (`feedback_citation`): every number↔title pair webref-verified
> 2026-06-21 — `body html event-loop-processing-model` → `##### 8.1.7.3 Processing model`
> (the "update the rendering" algorithm is the task defined within it; step 8 = run the resize
> steps, step 10 = evaluate media queries and report changes, confirmed verbatim from the `html`
> multipage step list); `dfn cssom-view-1 innerWidth`/`devicePixelRatio` → `§4 Extensions to
> the Window Interface`; `dfn cssom-view-1 resize` → `§13.3 Event summary`. The step-8<step-10
> ordering is the only spec fact PR-B's behavior turns on.

### 3.1 User-input touch audit
Every row is **User-input flow = no**. The values driving PR-B — window inner size,
`scale_factor`, tab-bar position — are **device/UA facts from winit/OS**, never
web-content-controllable. ⇒ no untrusted-input parsing introduced → trust-boundary enumeration
N/A (`feedback_trust-boundary-enumerate-upfront`). The web-content-facing values
(`@media`/`innerWidth`) are engine-side + pre-existing (#378/#370/#372); PR-B only supplies the
device fact.

### 3.2 Breadth + scope boundary
- **Breadth**: shell render/composite + winit redraw + content-thread IPC consumer +
  `elidex-render` transform seam. K=2 specs (HTML, CSSOM View), light — the web-facing surface
  is pre-existing; the work is shell coordinate-system (not measured by spec-citation count).
- **Engine-side already done, do NOT re-touch**: cascade `@media` gate + `MediaEnvironment`
  (#378), `matchMedia` registry + `change` delivery (#370/#372). PR-B only *drives* them from
  the real device.
- **Out of PR-B scope (named, §9)**: forced reflow-on-read (B-D5); HiDPI fidelity beyond
  geometry (B-D2); multi-tab + `ScaleFactorChanged` (PR-C).

---

## §4. Carry-forward cherry-pick (umbrella §5)

From branch `media-prefers-producers` (`8cd501a1` + `a14f3123`) on origin:
- **Take verbatim**: `chrome::content_size(window_width: f32, window_height: f32, position:
  TabBarPosition) -> Size` (+ its 3 unit tests `content_size_{top,left,right}` at
  `a14f3123:chrome.rs:265-...`/`:327+`) as the `size_logical` component — the helper lives in
  **`a14f3123`** (verified `git show a14f3123:.../chrome.rs:265`), NOT `8cd501a1`; lift the
  helper hunk + tests verbatim (the builder calls it as `content_size(win_logical.width,
  win_logical.height, position)`). Also take the
  `content_thread_setviewport_flips_width_media_query` content test (1024→800 flips a width
  `@media`).
- **Re-author (do NOT lift the wiring as-is)**: the `Resized`/`resumed` producer — it lands
  *with* the compositor offset + input `÷scale`, **never alone** (the #383 strangler). Folded
  into the `placement`-driven producer (§5).
- **Drop**: the rest of `a14f3123` — its chrome-size-only mitigation (the corner-fix that broke
  #716); keep only the `content_size` helper + tests from it.
- **Mechanics**: `git show a14f3123:crates/shell/elidex-shell/src/chrome.rs` to lift the
  `content_size` hunk + tests; do **not** branch off `media-prefers-producers`. Verify cwd
  before commit (`feedback_worktree-cwd-drift`).

---

## §5. Change surface (file-level, exact anchors at `d2663e6b`)

**Producer + SoT — `app/mod.rs` + `chrome.rs`**
- `chrome.rs` — add `content_size(window_width: f32, window_height: f32, position) -> Size`
  (carry-forward §4, verbatim from `a14f3123`).
- `app/mod.rs` — add `App.placement: ContentAreaPlacement` field (struct `:167-198`) + the
  `ContentAreaPlacement` type (new small struct, shell-local) + the sole builder
  `App::content_area_placement(&self) -> ContentAreaPlacement` (only caller of
  `chrome_content_offset` + `content_size` + `window.scale_factor()`). **Build at `resumed`**
  (`mod.rs:812-832`, with `render_state` — §2.3 init invariant) and recompute at redraw top
  (`handle_redraw_threaded`, `threaded.rs:117`) and on `Resized` (`mod.rs:860-871`).
- `app/mod.rs` — `App::send_viewport(&self)` sending `BrowserToContent::SetViewport`
  (`ipc.rs:100-105`) with `placement.size_logical` to the active tab (`send_to_content`
  `:748-756`). **Initial send is load-bearing (sub-check 2b, dry-run gap 2)**: call on **first
  `resumed`/tab-ready** AND on `Resized`. Without the initial send a non-resized tab keeps the
  content thread's `DEFAULT_VIEWPORT` (1024×768, `lib.rs:321-323`) while the painted area uses
  `placement.size_logical` → `innerWidth`/`@media`/layout disagree with the paint until the first
  resize. (Single-tab; PR-C fans out to all tabs + on switch.)

**Compositor — `elidex-render` + `app/render.rs`**
- `vello_backend.rs` — generalize: `build_scene` `:320-326` takes a `base_transform: Affine`
  param (drop the hardcoded `IDENTITY` `:325`); `VelloRenderer::render` `:71-119` gains a
  `base_transform: Affine` + `clip: Option<Rect>` param, threaded to `build_scene`. Reuses the
  `build_scene_with_transform` `:333` seam (already accepts a base Affine). Clip applied as a
  scene-root clip layer (same mechanism as `SubDisplayList` `:702-731`).
- `app/render.rs` — `with_frame` `:218-295`: pass `placement.content_transform` +
  `placement.content_clip_phys` to `renderer.render` `:231-237`; the blit stays full-surface
  (`:275-277`) — **the offset/scale now live in the scene transform, not the blit** (so the
  chrome region is `base_color`, content lands inset). `surface_config` `:223-224` stays the
  full render-target size (F8).

**Input — `app/threaded.rs`**
- Replace the three `cursor − offset` sites (`:199`, `:219`, `:253-257`) with
  `(cursor ÷ placement.scale_factor) − placement.origin_logical`. Source offset+scale from
  `self.placement` (not a fresh `chrome_content_offset` call — keep the single-builder
  invariant; remove the `threaded.rs:50` `chrome_content_offset` call in favor of the cached
  placement).

**Consumer order — `content/event_loop.rs`**
- `SetViewport` arm `:265-287`: move the `resize`-event block (`:276-282`) **above** the
  MQL-change block (`:271-274`) → order becomes viewport-set → resize → MQL change → restyle
  (B-D4 / I4). Restyle (`:284-285`) stays last.

**Tests**
- carry-forward content test + `content_size` unit tests (§4); compositor geometry test +
  input round-trip test + consumer-order test (§6); grep-guard test (§2.3).

---

## §6. Testing / acceptance criteria

- **Carry-forward `@media`-width content test** — 1024→800 flips a width `@media` red (green).
- **Compositor geometry (the #716 regression guard)** — a full-viewport content rect is painted
  at `origin_phys`, extent `size_phys`, **not** at `(0,0)` full-surface — at **scale 1 and
  scale 2** (I1∩I2). Asserts the scene base transform = `translate(origin_phys)∘scale` and the
  clip = content-area phys rect.
- **Input round-trip** — a click at window physical `p` maps to CSS `(p÷scale) − origin_logical`
  = exact inverse of the compositor transform — at scale 1 and 2, for Top/Left/Right chrome
  (I3).
- **Consumer order (I4)** — the `SetViewport` arm now dispatches the `resize` event **before**
  re-evaluating media queries (HTML §8.1.7.3 step 8<10), code-visible with a spec-cited comment.
  The carry-forward `@media`-flip content test exercises the full reordered path (resize +
  cascade `@media` re-eval + restyle) → no regression. **Coverage note**: a JS-observable
  *order* test (a `resize` listener vs a `matchMedia` `change` listener recording sequence) is
  **not feasible in the content thread** — boa's `matchMedia` `change` does not deliver on a
  viewport change in this path (verified: the change listener never fires), a pre-existing
  boa-side gap (`matchMedia` is mid-migration to the VM #370/#372, [[feedback_boa-findings-light-touch]],
  resolved at S5 when the VM drives the content thread). The reorder is enforced by code review
  + the spec citation + the no-regression content test.
- **Initial delivery, no resize (sub-check 2b gap 2)** — `App::send_viewport` is wired into
  `resumed` (initial tab-ready) so a tab that never resizes receives `placement.size_logical`,
  not the `DEFAULT_VIEWPORT` 1024×768. **Coverage note**: the producer call site is the
  winit `resumed` lifecycle (needs a real window) → not unit-testable without a windowing
  harness; the `send_viewport` body (reads `placement.size_logical`, sends `SetViewport` to the
  active tab) is trivial and the wiring is code-reviewed. (Shell window-lifecycle wiring is an
  integration/manual-verify boundary, not a unit-test surface — `feedback_existing-infra-...`.)
- **Grep-guard** ✅ `placement_builder_is_sole_caller_of_geometry_primitives` — asserts
  `chrome::chrome_content_offset` + `chrome::content_size` each have exactly one production
  caller (the builder) and `window.scale_factor()` exactly two (builder + egui `render.rs:66`
  exception), so `self.placement` is the only content-area size/origin/scale source (§2.3 / F1).
- **Supported-surface**: shell/integration contracts → shell content-thread + unit tests (no
  WPT subset claimed for the shell composite).

---

## §7. Collision / sequencing

- **PR-A merged** (`d2663e6b`) — PR-B's recompute-at-redraw-top sits in the same
  `handle_redraw_threaded` PR-A drains in; wake path untouched.
- **Engine-side media (#378/#370/#372)** upstream + untouched — PR-B is the producer half.
- **S5 boa→VM**: producers drive `BrowserToContent` IPC, not boa/VM → no S5 coupling; lands
  before/after S5 freely.
- **Terminal-Z render↔layout (C-3/C-4)**: PR-B changes `VelloRenderer::render`'s signature
  (adds base transform + clip). C-3/C-4 touch the fragment *consumers* of the DL, not the scene
  root transform — low collision; **flag at plan-review**. **`VelloRenderer::render` has a SOLE
  caller** — `render.rs:231` (grep-verified: the only `renderer.render(` in shell+core;
  `vello_backend.rs:105` is the internal `render_to_texture`; the iframe thread sends
  `SetViewport`, not a render call) → adding required params breaks exactly one site.
- **Worktree isolation**: `shell-viewport-pr-b` off `origin/main` `d2663e6b`; verify cwd before
  every commit (`feedback_worktree-cwd-drift`).

---

## §8. Open questions for `/elidex-plan-review`

- **Q-B1** — `VelloRenderer::render` signature: add `(base_transform: Affine, clip: Option<Rect>)`
  vs a small `RenderParams` struct? (Memo: two params now; a struct if PR-C/HiDPI adds more.)
  Sole caller is `render.rs:231` (§7 grep) → no fan-out break; the question is ergonomics, not
  blast radius.
- **Q-B2** — Clip mechanism: a scene-root clip layer (push/pop, like `SubDisplayList`
  `:702-731`) vs relying on the blitter sub-rect for content overflow? (Memo: scene-root clip —
  unifies with the transform; the blit stays full-surface.)
- **Q-B3** — Is recompute *only* at redraw-top + `Resized` sufficient for PR-B's single-tab
  scope, given PR-C adds the other device-fact events? Or must the builder be event-driven from
  the start to avoid a PR-C refactor? (Memo: redraw-top + `Resized` now; the builder is already
  the single chokepoint PR-C extends — no refactor, just more call sites.)
- **Q-B4** — Does dropping the `threaded.rs:50` `chrome_content_offset` call (folding it into
  the cached placement) interact with any non-cursor consumer of `offset` in `threaded.rs`?
  (Memo: §1.D shows offset's only uses are the three cursor sites — confirm at review.)

## §9. Proposed defer slots (register in the ledger at PR-B landing)

- `#11-forced-style-layout-flush-on-script-read` — a script layout/computed-style read inside an
  event handler must force a sync style/layout flush before returning the value (HTML
  "update the rendering" macro-order is already correct; this is the *intra-handler* read path).
  **General engine concern, not viewport-specific** — PR-B fixes only the resize↔MQL event
  *order* (I4). **Trigger**: a test/report showing a handler reads stale geometry, OR a forced-flush
  program. (B-D5 / umbrella §10 Q2.)
- `#11-hidpi-render-fidelity` — HiDPI fidelity beyond geometric scale: sub-pixel text
  positioning, hairline/1px snapping, fractional-dppx image resampling. PR-B delivers geometric
  scale (`size_phys`, input `÷scale`) only and refuses to re-bake `scale==1`. **Trigger**: a
  HiDPI fidelity program, OR a fractional-scale display test. (B-D2 / umbrella §10 Q3.)
