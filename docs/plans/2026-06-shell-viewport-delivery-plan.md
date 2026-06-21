# Shell viewport delivery — coordinate-system unification (plan-memo)

Slot: `#11-shell-viewport-delivery` (re-plan of CLOSED PR #383; the shell-producer
half of the media-query program — S0 Slice C).
HEAD at re-grep: `c801ef6c` (#380), 2026-06-21.
Status: **plan-memo (pre-implementation)** → next `/elidex-plan-review` (edge-dense, mandatory).

> Why this is a re-plan, not a resume: #383 framed this as a *"trivial producer on
> complete existing `SetViewport` infra"*. False — the `SetViewport` transport was
> only ever exercised by **one single-tab content-thread test with an explicit
> `re_render`**. Production delivery couples ≥5 invariant axes; a reactive
> chrome-size fix on #383 **broke the size↔paint-origin invariant** (Codex
> external-converge R1 #716: a content-area-sized page still painted at surface
> `(0,0)` → top hidden under chrome + blank bottom strip). That invariant break is
> the canary → STOP + re-plan as ONE plan-reviewed edge-dense slice.
> See `memory/feedback_existing-infra-production-completeness-premise.md`.

---

## §0. Decisions this memo commits to

These are the design forks resolved by first-principles lens **before** plan-review
(per `feedback_decide-via-philosophy-before-asking`); plan-review may overturn them.

- **D0 — There is one coordinate-system source of truth, not three ad-hoc
  subtractions.** A single browser-side **`ContentAreaPlacement`** descriptor —
  `{ chrome reservation (logical: offset + size), scale_factor }` — is the SoT.
  The viewport **producer** (SetViewport size), the **compositor** (paint
  origin + extent + scale + clip), and the **input mapper** (cursor→CSS-px)
  all *derive* from it. Today they are three independent computations that each
  silently bake `scale_factor == 1` and disagree about the chrome reservation
  (producer subtracts it, compositor + input do not) — that disagreement *is*
  the #716 bug. **Bound structurally** via one `App::content_area_placement()`
  accessor (the sole caller of the three primitives), not a naming convention —
  the descriptor *type* shared by value is not enough (§2.2). (§2)

- **D1 — The compositor places the content display list via the *existing*
  base-transform render path, unifying it with the iframe-offset mechanism.**
  `elidex-render` already has `build_scene_with_transform` (used to place an
  iframe sub-display-list at an offset, `vello_backend.rs:330`). The chrome
  content offset is the *same* mechanism (a display list placed at an
  offset+scale) at the browser-composite layer. We generalize
  `VelloRenderer::render` (currently hardcodes `Affine::IDENTITY`,
  `vello_backend.rs:325`) to accept a base transform + content-area clip rather
  than adding a parallel "blit offset" code path. One-issue-one-way. (§2.2)

- **D2 — `scale_factor` is a first-class parameter of the SoT, not an implicit
  `1`.** ScaleFactorChanged is one of the named axes; the entire shell currently
  assumes scale 1 (content DL rendered at IDENTITY into a physical-px surface;
  input cursor used physical px against logical chrome constants with no `÷scale`).
  We refuse to re-bake `scale == 1` into the new compositor (ideal-over-pragmatic).
  *Geometric* scale correctness is in scope; broader HiDPI rendering fidelity
  (sub-pixel text, hairline snapping) is flagged as a plan-review question (§10 Q3).

- **D3 — The content thread stays device-agnostic; device facts live in the
  browser shell.** Per *concurrency-by-ownership* + *security-by-structure*: the
  content/renderer thread is the owner of **CSS space only**. The browser process
  owns the device facts (scale, chrome geometry, OS theme) and the composite.
  The content thread receives only the *abstracted* CSS viewport **size** via
  `SetViewport` — never the offset, never the scale. This is why the placement
  SoT is browser-side and why prefers-color-scheme/dppx (§7 PR-C) reuse the same
  producer chokepoint rather than letting content read the device directly.

- **D4 — Structure = umbrella + 3 plan-reviewed sub-slices (A→B→C), not one PR
  and not piecemeal producers.** The slot text says "ONE slice"; the edge-dense
  RULE (≥5 axes) says "single PR 禁止 → umbrella + per-PR plan-review". These
  reconcile via the base-case clause: the slot's "one slice" was a corrective
  against *trivial-producer piecemeal*; an umbrella whose sub-slices are each
  narrowly-scoped + plan-reviewed honors both. Split points are placed **only**
  where main is left CORRECT (no strangler intermediate). (§7)

- **D5 — Consumer event order is fixed to spec; restyle-on-read is scoped out.**
  The SetViewport consumer must fire **resize before MQL-change** (HTML §8.1.7.3
  step 8 < step 10) — currently reversed (`event_loop.rs`). That narrow fix is
  in-core (PR-B). The *general* "force style/layout flush on a script layout-read
  inside the handler" (lazy reflow-on-read) is **not** viewport-specific and is
  scoped out to a named slot (§4 axis 5, §10 Q2).

- **D6 — prefers-color-scheme + `devicePixelRatio` producers belong to the PR-C
  sub-umbrella.** They share PR-B's producer chokepoint + consumer restyle path +
  PR-A's wake, so they live with the other device-fact producers — **but PR-C is
  itself edge-dense (F2)**: it is NOT asserted "trivial on production-complete
  infra" (that is the exact #383 premise this program retires). `dppx` is
  `scale_factor` (pairs with ScaleFactorChanged); prefers-color-scheme adds a winit
  `ThemeChanged` source. PR-C's own plan-review decides one-slice-vs-split
  (C1/C2/C3) from its own edge-matrix and retires `#11-media-prefers-features`.
  (§10 Q4.)

---

## §1. Verified anchors (re-grepped at HEAD `c801ef6c`, 2026-06-21)

All five axes confirmed against current `origin/main` (roadmap memo treated as
stale; every fact below is a fresh grep/Read, not a memory carry-over).

### 1.1 Paint origin — content blits full-surface at `(0,0)`, chrome opaque-on-top
- `with_frame` (`app/render.rs:218-295`) renders the content display list into an
  intermediate texture sized `surface_config.{width,height}` = **full physical
  surface** (`render.rs:223-224`, `vello_backend.rs:71-118`), with base transform
  **`Affine::IDENTITY`** (`vello_backend.rs:325`).
- The texture is **blitted 1:1** to the surface frame (`render.rs:275-277`,
  `TextureBlitter::copy`) — no offset, no sub-rect.
- The egui chrome (tab bar + address bar) is drawn **on top** afterwards via
  `render_egui_output` with `LoadOp::Load` (`render.rs:289`,
  `handle_redraw_with_tabs:300-321`).
- ⇒ A content display list sized to the *content area* (window − chrome) painted
  at `(0,0)` full-surface ⇒ **top hidden under the opaque chrome + blank strip at
  the bottom**. This is the #716 invariant break, confirmed structurally.

### 1.2 Input offset — `chrome_content_offset` is input-only and unscaled
- `chrome::chrome_content_offset(position)` is consumed at **one** site:
  `app/threaded.rs:50`, feeding cursor/press/wheel handlers
  (`handle_cursor_move_threaded:194-199`, `handle_mouse_press_threaded:213-219`,
  `handle_mouse_wheel_threaded:243-256`).
- Mapping is `content_pos = cursor_physical − chrome_offset_logical`
  (`threaded.rs:199/219/256`). The winit cursor is **physical px**; the offset is
  built from **logical** chrome constants (`chrome.rs` `CHROME_HEIGHT` etc.); there
  is **no `÷ scale_factor`**. ⇒ Input is correct only at `scale == 1`.

### 1.3 The whole shell currently assumes `scale_factor == 1`
- Content DL → surface: `Affine::IDENTITY` (no `× scale`) → CSS px painted 1:1 into
  physical px (`vello_backend.rs:325`).
- Input: physical cursor used directly, no `÷ scale` (§1.2).
- The **only** place scale exists is the #383 carry-forward producer, which sends
  *logical* CSS px to content (`new_size.to_logical(scale_factor)`) — correct per
  CSSOM but **inconsistent** with the IDENTITY paint + unscaled input at scale ≠ 1.
  ⇒ scale is the hidden third dimension of the same knot (D2).

### 1.4 SetViewport consumer — event order reversed vs spec; restyle after events
- `content/event_loop.rs:263-284`, `BrowserToContent::SetViewport`:
  1. `state.pipeline.viewport = …` (265) + `bridge.set_viewport` (267)
  2. `re_evaluate_media_queries` → `dispatch_media_query_changes` — **MQL change
     events** (269-272)
  3. **resize event** dispatch (274-280)
  4. `state.re_render()` — restyle + layout + paint (282) + `send_display_list` (283)
- Two facts: (a) **MQL-change fires before resize** — reversed vs HTML §8.1.7.3
  (resize step 8 < media-queries step 10); (b) `re_render` (the cascade restyle
  that `@media` gates) runs **after** the events — spec-aligned at the macro level
  (events at steps 8/10 precede recalc at step 16) but means a layout/computed-style
  *read inside a handler* sees stale state absent a forced reflow (§4 axis 5).

### 1.5 Multi-tab — `send_to_content` is active-tab-only; new tabs unseeded
- `send_to_content` (`app/mod.rs:689-697`) → `mgr.active_tab()` only.
- `window.open` new tabs are spawned at the pipeline default and **never seeded**
  with a viewport (`app/mod.rs:664-675`); `open_new_tab` (`threaded.rs:454`) +
  `SwitchTab` (`threaded.rs:433`) likewise push no viewport.
- Pipeline default = **`1024 × 768`** (`lib.rs:297-299`, `pipeline.rs:64`).
  ⇒ A second tab renders at `1024×768` regardless of window size until a resize
  *while it is active* happens to push it.

### 1.6 Async repaint-wake — absent; content-initiated frames stall under `Wait`
- `DisplayListReady` consumer (`app/mod.rs:398-400`) only stores
  `tab.display_list = dl`; it does **not** `request_redraw`.
- The drain (`drain_content_messages`, `mod.rs:379-400`) runs **inside** the
  redraw handler (`threaded.rs:118`) — i.e. it only consumes content messages
  when a redraw is *already* happening.
- **No `ControlFlow`, no `about_to_wait`, no `Poll`, no `EventLoopProxy`** anywhere
  in `crates/shell/elidex-shell/src/` (confirmed: zero grep hits). The event loop
  uses winit default **`ControlFlow::Wait`**.
  ⇒ A content-initiated `DisplayListReady` (the SetViewport round-trip's corrected
  frame, `setTimeout`/rAF/animation, async DOM update) does **not** wake the
  browser loop — it paints only on the *next* OS-driven event. This is the
  cross-cutting wake gap (not viewport-specific).

### 1.7 Carry-forward asset inventory (branch `media-prefers-producers`, on origin)
Two commits: `8cd501a1` (producer) + `a14f3123` (#383 Codex chrome-size fix). Both
are diff-local-correct; reuse the **good** parts, drop the producer wiring that
assumed trivial-infra:
- ✅ **`chrome::content_size(w, h, position) -> Size`** (`chrome.rs`, clamps ≥0,
  Top/Left/Right reservation, unit-tested) — correct, reuse verbatim as part of the
  `ContentAreaPlacement` SoT.
- ✅ **`content_thread_setviewport_flips_width_media_query`** content test
  (`content_tests.rs`) — drives `SetViewport` 1024→800 and asserts a
  `@media (max-width: 900px)` rule flips a div red — the live end-to-end cascade
  contract. Reuse verbatim (PR-B).
- ⚠️ **`App::send_viewport` + Resized/resumed wiring** (`app/mod.rs`) — the
  *producer* logic is reusable, but it lands **with** the compositor offset in
  PR-B (never alone — that is exactly the #716 strangler). Its doc-comment's
  "active tab only / repaint on next redraw" carve notes describe the gaps PR-A/PR-C
  close.

---

## §2. The ideal mechanism — coordinate-system unification

### 2.1 Three coordinate spaces, one SoT

| Space | Unit / origin | Owners |
|---|---|---|
| **Content / CSS** | logical CSS px, origin = content-area top-left | content thread, cascade, `@media`, layout, display list, `clientRect`s, DOM-delivered input |
| **Window-logical** | logical px, origin = window top-left | egui chrome, winit logical coords |
| **Physical surface** | device px, origin = surface top-left | wgpu surface, Vello render target, blit |

Relations: `content = window_logical − chrome_offset_logical`;
`physical = window_logical × scale_factor`.

The SoT that ties them is the browser-side **`ContentAreaPlacement`**:

```
ContentAreaPlacement {
    origin_logical: Point,   // chrome_content_offset(position)   — top-left of content area
    size_logical:   Size,    // chrome::content_size(win, position) — CSS viewport size
    scale_factor:   f32,     // window.scale_factor()
}
// derived:
//   origin_phys = origin_logical * scale_factor
//   size_phys   = size_logical   * scale_factor
//   content_transform = translate(origin_phys) ∘ scale(scale_factor)   // CSS-px DL → physical surface
```

Computed once per frame (or on each device-fact change) in the `App`, from
`window.inner_size()`, `window.scale_factor()`, and `tab_bar_position()`.

### 2.2 The three consumers all derive from the SoT

**Structural commitment — single per-frame placement, not three re-computations
(F1).** The SoT is a per-frame-**cached** `App.placement: ContentAreaPlacement`
field, recomputed **once** — at redraw top and on each device-fact event
(`Resized` / `ScaleFactorChanged` / `ThemeChanged` / tab switch) — via the sole
`App::content_area_placement()` builder. Producer, compositor, and input mapper all
read `self.placement`; the builder is the **only** caller of `chrome_content_offset`
+ `content_size` + `window.scale_factor()` (egui's own DPI read at `render.rs:66`
is a separate legitimate consumer and is excepted from the guard). Caching — *not*
a re-reading accessor — is required so the three primitives are snapshotted
**atomically** (one `scale_factor` read per frame); a re-reading accessor would let
two consumers in one frame read across a device event and desync click-vs-paint
(F9). A grep-guard test pins each primitive to exactly one caller (the builder) and
`self.placement` as the only content-area size/origin source. Absent this the
"single SoT" is value-copies at three sites — the #716 bug restated, not fixed.

**Surface fact vs content-area fact (F8).** `gpu.surface_config.{width,height}`
(`render.rs:223-224`, written by `gpu.resize`) stays the **full-window physical**
surface = the Vello render-target + blit extent (chrome-inclusive). It is a
*different fact* from the content-area placement, **not** a duplicate to eliminate:
the content render extent + origin come from `self.placement` (transform + clip,
§2.2 #2), **never** from `surface_config`. **Per-frame coherence invariant** (pinned,
not merely asserted): `surface_config` (written by `gpu.resize` on the single
`Resized` site, `gpu.rs:25-26`) and `placement` (built from `window.inner_size()` at
redraw top) both re-derive from the *same* winit physical size, and the synchronous
redraw cannot interleave a `Resized` between the two reads — so the §8 PR-B geometry
test (content painted at `origin_phys` / extent `size_phys`, never `(0,0)`
full-surface) guards the target↔content coherence. The compositor reads
`surface_config` only for the full render-target/blit and `placement` for
where/how-big the content lands within it. Reading `surface_config` as the *content*
size is the #716 trap, one axis over.

1. **Producer** (`SetViewport`): send `placement.size_logical` (CSS px) to the
   active/all content thread(s). Content stays device-agnostic (D3). *(carry-forward
   `content_size` already computes this; this slice makes the compositor + input
   agree with it.)*

2. **Compositor** (paint): generalize `VelloRenderer::render` to take a base
   `Affine` + content-area clip (D1) — render the content DL with
   `placement.content_transform`, clipped to the content-area physical rect; the
   chrome reserved region stays `base_color` until egui draws on top (unchanged
   egui path). This **is** the #716 fix (origin now matches the size the content
   was told) **and** the scale fix (the IDENTITY is gone). Reuses the existing
   `build_scene_with_transform` precedent (iframe offsets) rather than a parallel
   blit-offset path.

3. **Input mapper**: `content_css = (cursor_physical ÷ scale_factor) −
   origin_logical`. Replaces the current `cursor_physical − offset_logical`
   (§1.2). Now symmetric with the compositor transform (inverse of it).

> **Why this kills #716 structurally**: a chrome-size *standalone* fix (what #383
> tried) changes only consumer #1 and leaves #2/#3 disagreeing → the next corner
> breaks. Routing all three through one descriptor means "the page is told a size,
> painted at the matching origin+scale, and clicked at the inverse" is a single
> invariant, not three that must be kept in sync by hand.

### 2.3 ECS-native / layering check
- The placement SoT is **browser-process (shell) owned** device state (D3) — it is
  not per-DOM-entity content state, so it is *not* an ECS component (correctly a
  shell-local value; `feedback_existing-infra-production-completeness-premise` /
  the side-store→component rule's "shared cross-cutting state" exception).
- No new algorithm enters `vm/host/` (Layering mandate untouched — this is all
  shell/render).
- The compositor change lives in `elidex-render` (engine-independent render crate)
  + the shell composite call site; the transform generalization is a pure
  extension of an existing seam.

---

## §3. Spec coverage map (preflight hard-gate)

Per `feedback_plan-scope-re-evaluation` (§3 must carry a spec-coverage map +
breadth + user-input-touch audit). All anchors webref-verified 2026-06-21.

### 3.1 Algorithms / interfaces this slice must honor

Canonical 6-column schema (`feedback_plan-scope-re-evaluation`). `Full enum?`
column = "is every branch of this step covered?". `User-input flow` = is a
web-content-controllable value involved? (here uniformly **no** — device facts
come from winit/OS, not page content; see §3.1.1). Anchors webref-verified
2026-06-21 (§11). Labels `CSSOM View` / `Media Queries` are not yet in the
preflight `SPEC_LABEL_REVERSE` map → preflight soft-warns + skips auto-verify for
those rows; they are manually webref-verified in §11.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM View §4 Extensions to the Window Interface | `innerWidth`/`innerHeight` read (`#dom-window-innerwidth`) | content CSS viewport size = painted content area | producer `SetViewport` size + compositor agreement (PR-B) | ✓ | no |
| CSSOM View §4 Extensions to the Window Interface | `devicePixelRatio` read (`#dom-window-devicepixelratio`) | = `scale_factor`; `dppx` device fact | PR-C dppx producer (D6) | ✓ | no |
| CSSOM View §4 Extensions to the Window Interface | `matchMedia` (`#dom-window-matchmedia`) / MQL `change` | re-evaluate on viewport/dppx/theme change | engine side #370/#372/#378 (consumed, not re-touched) | ✓ | no |
| CSSOM View §13.3 Event summary | `resize` event fire (`#eventdef-window-resize`) | non-bubbling, on size change | consumer (PR-B); already fired — order fixed | ✓ | no |
| WHATWG HTML §8.1.7.3 Processing model | step 8 run the resize steps (`#update-the-rendering`) | resize **before** media queries | consumer order fix (PR-B, D5) | ✓ | no |
| WHATWG HTML §8.1.7.3 Processing model | step 10 evaluate media queries and report changes | MQL `change` **after** resize | consumer order fix (PR-B, D5) | ✓ | no |
| WHATWG HTML §8.1.7.3 Processing model | step 16.1 recalc styles + layout / step 22 update rendering | restyle/paint **after** resize+MQL events (macro-order); reads inside handlers need a forced flush | macro-order already correct; forced-reflow-on-read **scoped out** (§4 axis 5) | ✓ (boundary) | no |

#### 3.1.1 User-input touch audit
Every row is `User-input flow = no`: the values driving this slice (window inner
size, `scale_factor`, OS theme, tab-bar position) are **device/UA facts from
winit/OS**, never web-content-controllable. ⇒ no untrusted-input parsing
introduced → trust-boundary enumeration N/A
(`feedback_trust-boundary-enumerate-upfront`). The web-content-facing surface
(`@media`/`matchMedia`/`innerWidth` *values*) is engine-side and pre-existing
(#370/#372/#378) — this slice only supplies the device fact, it does not parse
page input.

### 3.2 Breadth + scope boundary
- **Breadth**: shell render/composite + winit event handling + content-thread IPC
  consumer + `elidex-render` transform seam. K=2 specs (CSSOM View, HTML), M=7
  entries → preflight `split decision: ok (single PR scope)` for the *spec* breadth
  — but the **invariant-axis** breadth (§4, ≥5) is what drives the umbrella split
  (§7), not the spec-citation count. (Spec breadth is light precisely because the
  web-facing surface is pre-existing; the work is shell coordinate-system, which
  the spec-citation metric does not measure.)
- **Engine-side already done (do NOT re-touch)**: the cascade `@media` gate +
  `MediaEnvironment` (#378), `matchMedia` registry + `change` delivery
  (#370/#372). This slice only *drives* them from the real device — it is the
  shell-producer half. (`feedback_existing-infra-...`: this is the half whose
  production-completeness #383 mis-asserted.)
- **Out of spec scope (named, §10)**: forced reflow-on-read; full HiDPI render
  fidelity beyond geometry.

---

## §4. The five coupled invariant axes — disposition

| # | Axis | Verified state (§1) | Disposition |
|---|---|---|---|
| 1 | **Coordinate system** (size↔origin↔offset↔scale) | three ad-hoc computations, all bake scale=1, producer disagrees with compositor+input | **PR-B core** — the `ContentAreaPlacement` SoT (§2). Atomic with the producer. |
| 2 | **Multi-tab lifecycle** | active-tab-only send; new/`window.open` tabs unseeded; default 1024×768 | **PR-C** — seed on tab create/switch + fan-out resize to all tabs. Additive on PR-B SoT; main correct without it (new tab at default = no regression). |
| 3 | **ScaleFactorChanged** | no handler; DPI changes logical px without a `Resized` | **PR-C** — winit `ScaleFactorChanged` → re-derive placement → re-send `SetViewport` + repaint. Depends on PR-B scale parameter (D2). |
| 4 | **Async repaint-wake** | no `EventLoopProxy`/`Poll`; content-initiated frames stall under `Wait` | **PR-A (infra, first)** — generic `EventLoopProxy` waker → `request_redraw` on content message. Cross-cutting (timers/rAF/animation/async DOM), **not** viewport-specific → own PR, independent value. |
| 5 | **Consumer restyle/event order** | (a) MQL fires before resize (reversed); (b) restyle (`re_render`) after events | (a) **PR-B core** — swap to resize→MQL (HTML §8.1.7.3 step 8<10). (b) macro-order is spec-correct; the real gap is **forced-reflow-on-read** = general engine concern → **scoped out** to a named slot (§10 Q2), not grown reactively here. |

---

## §5. Carry-forward cherry-pick

From branch `media-prefers-producers` (`8cd501a1` + `a14f3123`) on origin:

- **Take into PR-B**: `chrome::content_size` (+ its 3 unit tests) verbatim, as the
  `size_logical` component of `ContentAreaPlacement`;
  `content_thread_setviewport_flips_width_media_query` content test verbatim.
- **Re-author in PR-B (do not cherry-pick the wiring as-is)**: `App::send_viewport`
  + the Resized/resumed producer — they land *with* the compositor offset + input
  `÷scale`, never alone (the #383 strangler). Fold `send_viewport` into the
  placement-driven producer.
- **Drop**: the `a14f3123` chrome-size-only mitigation (it is the corner-fix that
  broke #716 — superseded by the §2 SoT).
- **Mechanics**: `git worktree add` from `origin/main` (clean base, per the
  worktree-isolation rule — do **not** branch off `media-prefers-producers`);
  `git show <sha>:<path>` to lift the good hunks, or `git cherry-pick -n` then
  prune. Verify cwd before commit (`feedback_worktree-cwd-drift`).

---

## §6. Change surface (file-level, grouped by axis)

PR-A (wake):
- `app/mod.rs` / `lib.rs` — `EventLoop` user-event type + `event_loop.create_proxy()`;
  thread the proxy to the content-drain so a `DisplayListReady` → `request_redraw`.
  (Confirm at PR-A planning whether to use a `UserEvent` round-trip or
  `Window::request_redraw` from the proxy; confirm `ControlFlow` stays `Wait`.)

PR-B (core):
- `chrome.rs` — `content_size` (carry-forward). `app/mod.rs` — `App.placement:
  ContentAreaPlacement{ origin_logical, size_logical, scale_factor }` **cached
  field** + the single `App::content_area_placement()` builder (F1: the *only*
  caller of `chrome_content_offset` + `content_size` + `window.scale_factor()`,
  egui-init `render.rs:66` excepted), recomputed at redraw top + on each
  device-fact event; a grep-guard test pins each primitive to one caller **and**
  `self.placement` as the sole content size/origin source.
- `crates/core/elidex-render/src/vello_backend.rs` — generalize `render` to accept a
  base `Affine` + content-area clip (extend the `build_scene_with_transform`
  path, drop the hardcoded `IDENTITY` at :325 to a parameter).
- `app/render.rs` — `with_frame`/`handle_redraw*` render the content DL with
  `placement.content_transform` + content-area clip; `surface_config` stays the
  full-window render-target/blit extent (F8: **not** the content size — content
  extent/origin come from `placement`).
- `app/threaded.rs` — input mapper `÷ scale_factor` (replace the §1.2 sites);
  derive offset from the placement.
- `app/mod.rs` — placement-driven `send_viewport` producer (Resized/resumed).
- `content/event_loop.rs` — swap resize↔MQL order (resize first, §1.4/D5).
- tests: carry-forward content test + `content_size` unit tests + a compositor
  origin/scale geometry test.

PR-C (breadth):
- `app/mod.rs` / `app/tab.rs` / `app/threaded.rs` — seed `SetViewport` on tab
  create/`window.open`/switch + fan-out resize to all tabs.
- `app/mod.rs` — `WindowEvent::ScaleFactorChanged` handler (re-derive placement,
  re-send, repaint).
- `app/mod.rs` — prefers-color-scheme (winit `ThemeChanged`) + `dppx`(=scale)
  device-fact producers on the same chokepoint (D6).

---

## §7. Umbrella decision — single slice vs umbrella + sub-slices

**Decision (D4): umbrella + 3 plan-reviewed sub-slices, dependency A → B → C.**

Rationale:
- ≥5 intersecting invariant axes (§4) + **no canonical algorithm** for the
  coordinate composite ⇒ the edge-dense RULE forbids a single PR and mandates
  per-PR plan-review.
- The slot's "ONE slice" intent (anti-piecemeal-trivial-producer) is satisfied by
  an umbrella whose sub-slices are each narrowly-scoped + plan-reviewed (base-case
  clause). It is *not* satisfied by cramming 5 axes into one un-splittable PR.
- Split points are placed **only** where main is left CORRECT (no strangler,
  One-issue-one-way):

| PR | Scope | Depends | Why a clean boundary (main stays correct) |
|---|---|---|---|
| **A** | async repaint-wake (generic `EventLoopProxy` waker) | — | Independent pre-existing fix (content-initiated repaints stall under `Wait`); not viewport-specific; one canonical "content woke us → repaint" path. Main strictly better, no strangler. |
| **B** | coordinate-system SoT + producer + compositor offset/scale + input `÷scale` + consumer resize↔MQL order | A | **Atomic** — producer + paint-offset must ship together (else #716). After B: single active-tab viewport delivery fully correct (size↔origin↔offset↔scale), settles immediately (A's wake). New tabs at default = incomplete-but-correct (no regression). |
| **C** | multi-tab seed/fan-out (incl. seed-at-create when the window may not yet exist, §1.5) + ScaleFactorChanged + prefers-color-scheme/dppx producers | B | Producers on B's SoT + A's wake, but **itself edge-dense** (≥3 axes: multi-tab lifecycle + ScaleFactorChanged re-derive + ≥2 device-fact sources) → gets its **own** edge-matrix + `/elidex-plan-review`, and may further split (C1 multi-tab / C2 ScaleFactorChanged+dppx / C3 prefers-color-scheme). **Not** asserted "trivial/additive" — that is the #383 premise this program exists to retire (F2). |

- **Why A is not folded into B**: the wake benefits every content message (rAF,
  animation, timers, async DOM), so folding it understates its blast radius and
  re-couples a cross-cutting concern into a viewport PR (the anti-pattern #383
  was punished for). It is the canonical "ControlFlow::Wait + content-initiated
  repaint" fix.
- **Why B must stay atomic**: any split of "tell content a content-area size" from
  "paint at the content-area origin" reproduces the #716 strangler. The base-case
  clause does **not** license breaking an atomic invariant.
- **Why prefers/dppx live in the PR-C sub-umbrella (D6)**: they share B's producer
  chokepoint + consumer restyle path + A's wake → they belong with the other
  device-fact producers (One-issue-one-way; one chokepoint for every device fact
  the shell pushes to content) and retire `#11-media-prefers-features`. Whether
  PR-C is one slice or splits into C1/C2/C3 is decided at **PR-C's own
  plan-review** from its own edge-matrix (its axis count, not an assumed-trivial
  label). *(Q4: split prefers-color-scheme out if its OS-theme source warrants.)*

Each sub-slice gets its own plan-memo + `/elidex-plan-review` before implementation.
This memo is the **umbrella**; PR-B (the core) likely also needs its own focused
plan-memo since it carries the SoT design.

---

## §8. Testing / acceptance criteria

- **PR-A**: a content thread that emits an unsolicited `DisplayListReady` (no
  preceding browser event) causes a browser repaint (deterministic via the proxy;
  assert `request_redraw` invoked / frame consumed). Confirm no busy-loop (stays
  `Wait`, wakes only on message).
- **PR-B**:
  - carry-forward `@media`-width content test (1024→800 flips red) — green.
  - compositor geometry test: a full-viewport content rect is painted at
    `origin_phys`, extent `size_phys`, not at `(0,0)` full-surface (the #716
    regression guard) — at scale 1 **and** scale 2.
  - input round-trip: a click at window pixel `p` maps to CSS `(p ÷ scale) −
    offset` (inverse of the compositor transform) — at scale 1 and 2.
  - consumer order: assert a `resize` listener runs before a `matchMedia` `change`
    listener on one SetViewport (HTML §8.1.7.3 step 8<10).
- **PR-C**: new/`window.open` tab receives the current viewport on creation; a
  resize fans out to a hidden tab; a `ScaleFactorChanged` re-sends + repaints;
  prefers-color-scheme/`dppx` `@media` flip on the respective device-fact change.
- Supported-surface: these are shell/integration contracts → shell content-thread
  + unit tests (no WPT subset claimed for the shell composite).

---

## §9. Collision / sequencing

- **Engine-side media (#378/#370/#372) is upstream and untouched** — this is the
  producer half; no overlap with in-flight S0 engine work.
- **S5 boa→VM cutover**: the shell producers are engine-agnostic (drive
  `BrowserToContent` IPC, not boa/VM directly) → no S5 coupling; lands before or
  after S5 freely.
- **Terminal-Z render↔layout convergence**: PR-B touches `elidex-render`'s
  `render` signature (base transform) — coordinate with the C-3/C-4 fragment-walk
  work if concurrent (different files: `vello_backend.rs` transform vs the
  fragment consumers). Low collision risk; flag at PR-B plan-review.
- **Worktree isolation**: each sub-slice in its own worktree off `origin/main`
  (§5 mechanics).

---

## §10. Open questions for `/elidex-plan-review`

- **Q1** — Compositor seam: generalize `VelloRenderer::render` to take a base
  `Affine`+clip (D1, reuse `build_scene_with_transform`), **or** apply the offset at
  the blit step (`TextureBlitter` sub-rect)? Memo prefers the render-transform
  (unifies with iframe offset; clip handles overflow); is there a Vello cost
  argument for the blit path?
- **Q2** — Is the resize↔MQL **order** fix (D5a) sufficient for PR-B, with
  **forced-reflow-on-read** (D5b) explicitly scoped to a new slot
  (`#11-forced-style-layout-flush-on-script-read`)? Or does the viewport case
  warrant a narrow forced flush now? (Memo: scope out — it is general, not
  viewport-specific; growing it here is the reactive-mechanism anti-pattern.)
- **Q3** — Is *geometric* scale correctness (D2) the right PR-B boundary, with full
  HiDPI render fidelity (sub-pixel text, hairline snapping, image resampling) a
  named follow-on? Or must PR-B not introduce `scale` at all and keep the explicit
  `scale==1` until a dedicated HiDPI program? (Memo: parameterize scale now — refuse
  to re-bake `1`; defer only the *fidelity*, not the *geometry*.)
- **Q4** — Fold prefers-color-scheme + dppx into PR-C (D6), or split
  prefers-color-scheme (OS-theme `ThemeChanged` source) into its own sub-slice?
  (dppx is free in PR-C = scale.)
- **Q5** — PR-A wake transport: a winit `UserEvent` (custom `EventLoop<T>`) vs a
  bare `EventLoopProxy` + `Window::request_redraw`? Does a custom user-event type
  ripple through the existing `run_app` signatures (`lib.rs:339/887`)? **Contract
  (F6)**: whichever transport, PR-A's plan-memo must state the
  **wake→redraw→drain→paint** ordering — a content message must cause a redraw
  whose `drain_content_messages` consumes the new DL *before* `present`. Today the
  drain runs at the top of the redraw handler (`threaded.rs:118`), so a
  `request_redraw` from the wake suffices; PR-A must confirm and pin it.
- **Q6** — Does PR-B need its own dedicated plan-memo (it carries the SoT), with
  this doc as the umbrella, or is this memo's §2/§6 detail enough for PR-B's
  plan-review directly?

### §10.1 Proposed new defer slots (register in the ledger at slice landing)

Per `feedback_defer-slot-eligibility-audit-at-create` — each carries Why / Trigger
/ Date. Both are *proposed* (not yet in `project_open-defer-slots.md`); register at
PR-B landing (F3, F4).

| Slot | Why deferred | Re-evaluation trigger | Date |
|---|---|---|---|
| `#11-forced-style-layout-flush-on-script-read` | Forced synchronous style/layout flush on a script layout / computed-style read inside an event handler (lazy reflow-on-read) is a **general engine concern** affecting every script layout read, not viewport-specific; growing it inside a viewport slice is the reactive-mechanism anti-pattern. PR-B fixes only the resize↔MQL *order* (D5a). | A WPT/test demanding `getBoundingClientRect`/`getComputedStyle` freshness *inside* a resize/MQL handler, OR any script-layout-read fidelity program. | 2026-06-21 (re-eval at PR-B landing) |
| `#11-hidpi-render-fidelity` | HiDPI render fidelity *beyond* geometric scale (sub-pixel text positioning, hairline snapping, fractional-dppx image resampling). PR-B delivers geometric scale correctness only (D2 — refuse to re-bake `scale==1`); the fidelity surface is its own program. | HiDPI visual-correctness demand / a fractional-scale (e.g. 1.5×) display regression. | 2026-06-21 (re-eval at PR-B landing) |

---

## §11. Citation appendix (webref-verified 2026-06-21)

- CSSOM View Module Level 1 — §4 Extensions to the Window Interface:
  `innerWidth` `#dom-window-innerwidth`, `devicePixelRatio`
  `#dom-window-devicepixelratio`; §13.3 Event summary: `resize`
  `#eventdef-window-resize`.
- HTML — §8.1.7.3 Processing model, "update the rendering" `#update-the-rendering`:
  step 8 *run the resize steps* [CSSOMVIEW]; step 10 *evaluate media queries and
  report changes* [CSSOMVIEW]; step 16.1 *recalculate styles and update layout*;
  step 22 *update the rendering or user interface*.
- CSSOM View Module Level 1 — §4 Extensions to the Window Interface: `matchMedia`
  `#dom-window-matchmedia`; §4.2.1 Event summary: MediaQueryList `change`
  `#eventdef-mediaquerylist-change`. (F5: Media Queries Level 5 governs the MQ
  *evaluation grammar*, **not** the `matchMedia`/`change` API surface.) Engine
  side: #370/#372/#378.

---

## §12. As-built notes (implementation)

_(filled in per sub-slice during implementation)_
