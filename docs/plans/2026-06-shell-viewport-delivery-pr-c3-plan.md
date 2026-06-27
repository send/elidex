# Shell viewport delivery — PR-C **C3** (device facts: dppx + prefers-color-scheme) plan-memo

Slot: `#11-shell-viewport-delivery` (PR-C, sub-slice **C3**, "prefers/dppx device facts"). Parent
sub-umbrella: `docs/plans/2026-06-shell-viewport-delivery-pr-c-plan.md` (§0 D-C1/D-C3/D-C4, §5, §9
Q-C1/Q-C2). Prior slices: PR-A ✅ #386 / PR-B ✅ #388 (placement SoT) / **C1 ✅ #396** (construction-input +
`ViewportCell` + seq) / **C2 ✅ #411** (`publish_if_changed` seq-iff-`size_logical`; f64 `logical_px`).

Anchors re-grepped at HEAD `0e24b137` (2026-06-27, post-C2 merge). Spec § numbers cited from the C-parent
(webref-verified 2026-06-21); `/elidex-plan-review` Axis 4 re-verifies.

---

## §0. Decisions this memo commits to

C3's job (C-parent D-C3/§2): deliver the two DPI/theme **content facts** — `window.devicePixelRatio` /
`@media (resolution)` (dppx) and `@media (prefers-color-scheme)` — from the shell to every content thread,
through the **one device-fact chokepoint** (D-C1), so a single canonical evaluator reflects the live
OS/display state. C1 made the *size* fact correct; C3 makes the *device* facts correct.

- **D1 — C3 subsumes and CLOSES `#11-shell-viewport-scalefactorchanged-x11-coverage` (the F3 slot); they
  are one mechanism.** The C-parent (§4/§5) sequenced C3's dppx fact onto "the `ScaleFactorChanged` arm C2
  adds." **C2 pivoted and added no such arm** — it carved the X11 `ScaleFactorChanged`-only gap to that slot
  instead (c2-plan §0/§8). The dppx fact's value **is** `scale_factor` (`media_query.rs:302`
  `resolution_dppx ← device_pixel_ratio`); it must update on **every** DPI change, *including the X11
  constrained/tiling-WM `ScaleFactorChanged`-only case where no `Resized` follows* (winit 0.30.13
  `x11/event_processor.rs:742-747`, source-re-verified). "Detect every DPI change and propagate" is exactly
  the F3 slot's definition. So C3's dppx producer is not merely *coupled to* F3 — it **is** F3, with a
  content-fact payload. C3 is the slot's natural and correct discharge point. **A geometric-only F3 fix
  shipped separately would carry nothing new to content** (C2 already handles common-path geometry), so F3
  should not be a standalone PR — it lands here.

- **D2 — the producer collapses to ONE steady-state chokepoint at redraw-top (F3 option (c)), and the
  `placement` recompute + `seq` bump are updated *atomically there* (the placement↔seq coherence
  invariant).** All ongoing device-state work — `placement` **recompute**, `publish` (size seq bump + device
  facts), **and** `reclip` (gated on a real placement change) — flows through the single per-frame recompute
  at `threaded.rs:139` (`handle_redraw_threaded`, every `RedrawRequested`). The event arms (`Resized`, NEW
  `ScaleFactorChanged`, NEW `ThemeChanged`) do **only** `gpu.resize` (to the new physical size) + cache theme
  + `request_redraw`; they **neither recompute `placement` nor publish nor reclip**. This is the correction
  of the plan-review F1 hazard: C2's `Resized` arm recomputed `placement` **and** bumped `seq` synchronously
  (atomic) and returned; a *partial* move (publish→redraw-top, recompute stays in the arm) would leave a
  window where `placement` is new but `seq` old — input mapped against the new placement, stamped the old
  seq, is then dropped as stale ⇒ the exact phantom input-drop C2 prevents. Moving **both** to redraw-top
  keeps them atomic: across the event→redraw gap `placement` and `seq` are **both** still old (coherent;
  input there maps old-placement/old-seq and is correctly superseded when the new viewport applies). This
  (a) closes the X11-only gap by construction — every DPI change requests a redraw, and redraw-top observes
  the **settled** `(inner_size, scale_factor)`, so the `old_phys/new_scale` bogus-intermediate the F3 slot's
  "naive route" warns about *never materializes*; (b) rejects F3 options (a) [replicate winit's private
  `adjust_for_dpi` rounding — brittle/version-coupled] and (b) [read winit's proposed phys — `InnerSizeWriter`
  is **write-only**, infeasible]; (c) is *One-issue-one-way* — **two** publish sites total (`resumed`
  establish + redraw-top steady-state), folding C2's `Resized`-arm site into the recompute rather than adding
  a third. The `resumed` establish-recompute-publish-before-spawn (C1 construction-input) **stays** synchronous
  (the spawn reads the cell) — the seed phase; redraw-top's first post-resume pass is an idempotent no-op.
  **Two behavior deltas vs C2 (deliberate, plan-review-accepted; both from the focused F1 re-check):**
  (1) *Gap-input* — input in the ≤1-frame event→redraw gap now maps old placement (it *is* old until the
  redraw) and is dropped if the viewport changed — vs C2 applying it against the synchronously-recomputed new
  placement. Geometrically the gap-input maps the pre-resize rect, so superseding it is defensible; ≤1 frame
  of an active resize. (2) *Zero-size (minimize)* — C2's `Resized` arm recomputed/published/reclipped
  **unconditionally** (only `request_redraw` was `>0`-gated, `mod.rs:959-961`). Routing all three through
  redraw-top makes them redraw-driven, and the size-tied redraw is `>0`-gated, so a `0×0` minimize now
  recomputes/publishes **nothing** until the next redraw / restore-`Resized` — benign-or-better, dropping
  C2's spurious `size_logical = 0` generation (a phantom seq bump content does not need while hidden);
  restore republishes the real size. **`reclip` gate (F1 re-check IMP-b)**: `reclip` reads the *whole*
  placement geometry (`origin_logical` + `scale_factor` + `size_logical`), so it is gated on a **full
  `placement != cached_placement` comparison** at redraw-top (NOT the size-only `DeviceDelta`), covering the
  origin axis `#11-window-level-tab-bar-position` will make live.

- **D3 — `ViewportCell` generalizes to a single browser-published device-state cell; the `seq` stays
  `size_logical`-only.** The dppx/color-scheme facts are **orthogonal to the viewport `seq`**: a pure-scale
  change that the OS *does* absorb (phys resizes, `size_logical` preserved) changes dppx but **not**
  `size_logical`, so it must ship `SetDeviceFacts` **without** bumping `seq` (bumping would manufacture the
  phantom input-drop C2 exists to prevent). So the cell holds `{ size, seq, dppx, color_scheme }`, and the
  single `publish_if_changed`-successor returns *which* facts changed: `size` → `SetViewport` (+seq bump, the
  C2 discipline, unchanged); `dppx`/`color_scheme` → `SetDeviceFacts` (no seq). One writer (browser
  redraw-top), per-fact change-detection by construction, one cell. dppx delivery **must not** gate on the
  `size` seq (the C-parent never anticipated this — it assumed dppx rode the size publish).

- **D4 — the consumer routes boa's `@media` evaluation through the canonical `elidex-css` evaluator
  (resolves Q-C2 = route-canonical-now; NOT defer-to-S5).** The boa-local `evaluate_media_query_raw`
  (`bridge/mod.rs:774`) is a `(query:&str, w:f32, h:f32) -> bool` **pure free function** that stubs
  `prefers-color-scheme => false` (`:796`) and has no `resolution` branch — *all* `JsValue`/`MediaQueryList`
  concerns are already caller-side (`globals/window/media_query.rs:38`, `bridge/media.rs:46`). Routing is a
  **localized body-swap**: change its inputs to a `MediaEnvironment` (mirroring the VM's 6-line
  `media_query.rs:298-307` builder) and call `elidex_css::media::evaluate(&parse_media_query_list(query),
  env)`. `elidex-css` is already a **zero-cost transitive dep** of boa (via `elidex-dom-api` /
  `elidex-script-session`); boa **already** routes inline-style parsing through canonical
  `elidex_css::parse_inline_style` (`globals/window/mod.rs:655`) — identical precedent. **Justification re
  net-new boa work (F3, `feedback_boa-findings-light-touch`)**: this is *not* purely "deletion-shaped" — it
  deletes the `:796` stub but **adds** a multi-site routing layer (body+signature swap, two caller env-builds,
  a `color_scheme` field + accessor pair, a `Cargo.toml` dep promotion), all boa, deleted-with-boa at S5. It
  is justified **not** as lesson-exempt but on the **D-C4 hard gate**: a live media-fact consumer is
  *mandatory* pre-S5 (no producer-without-consumer dead-store), so *some* boa consumer work is unavoidable;
  among the ways to provide it, routing through canonical `elidex-css` is the **One-issue-one-way** form (one
  evaluator for both engines — the durable win the VM already embodies) **and** is *smaller in net boa code*
  than the alternative the C-parent Q-C2 named (widen the `:796` stub to hand-roll `prefers-color-scheme` +
  `resolution` = a parallel second evaluator, pure throwaway). So the bounded boa work is the cheapest
  *correct* discharge of the mandatory consumer, not stub-growth (which the lesson forbids). The D-C4 fallback
  (defer C3 to S5) is **not** taken — the seam is CLEAN, so C3 ships producer + IPC + store + a **live**
  canonical consumer in one slice (D-C4 hard gate satisfied).

- **D5 — IPC = one unified `BrowserToContent::SetDeviceFacts { color_scheme, dppx }` (resolves Q-C1).**
  Mirrors the VM's already-unified `HostDriver::set_media_environment` (one push of all device state,
  `engine.rs:364`); one variant → one content arm → one re-eval → one repaint (the HTML §8.1.7.3
  update-the-rendering cadence; CSSOM-View §4.2 *evaluate media queries and report changes*), vs two variants that would double the re-eval/repaint on a
  `ScaleFactorChanged` that changes scale while a theme toggle co-occurs. Carries a `reduced_motion` field
  too **only if** `prefers-reduced-motion` is in scope (it is **not** — see §7); otherwise the shape is
  `{ color_scheme, dppx }` and the future `prefers-reduced-motion` producer extends this same variant (NOT a
  third one). `BrowserToContent` is `#[derive(Debug)]`-only (not `Clone`) → per-recipient reconstruction, as
  existing variants.

- **D6 — no new geometric SoT; reuse PR-B `placement` (C-parent D-C5).** dppx's source is
  `placement.scale_factor` (the sole `window.scale_factor()` reader, `viewport.rs:88`); color-scheme's
  source is `window.theme()`. No sub-slice re-reads the PR-B primitives behind the strangler guard.

---

## §1. The reconciliation — what C2's pivot changed for C3, and why F3 lands here

The C-parent was written at PR-B HEAD (`26687d18`) and assumed C2 would add a `ScaleFactorChanged` arm
emitting the dppx trigger (C-parent §4 "C3's dppx fact is emitted from the same arm C2 adds → C3 depends on
C2"; §5 C2 = "`ScaleFactorChanged` → recompute…"). **C2 (#411) did not.** C2's plan-review-ratified pivot
(c2-plan D1) was "no `ScaleFactorChanged` handler — `Resized` always follows," which Codex R3/F3 then
falsified on X11 (winit forces the follow-up `Resized` only when the DPI-adjusted physical size differs),
carving the gap to `#11-shell-viewport-scalefactorchanged-x11-coverage`.

Consequence for C3, source-verified (post-C2 HEAD `0e24b137`):

| C-parent assumption | Post-C2 reality | C3 obligation |
|---|---|---|
| C2 adds a `ScaleFactorChanged` arm; C3 extends it for dppx | **No arm exists** (`app/mod.rs` grep: only the `:962-966` *comment* naming the carved gap) | C3 **adds** the arm |
| dppx rides C2's `Resized`/`ScaleFactorChanged` size publish | C2 publishes `size_logical` only, gated on `seq`; `scale_factor` is **browser-side only** (`viewport.rs:111-112` "only `size_logical` crosses the IPC, never `scale_factor`") | C3 ships dppx as a **new fact**, on its own change-detection (D3) |
| (F3 unknown at C-parent time) | X11 `ScaleFactorChanged`-only ⇒ a real DPI change with no `Resized` | C3 must fire on it ⇒ **C3 = the F3 fix** (D1) |

So the X11-only DPI change is *precisely* where dppx delivery and F3 coincide: phys is constrained (constant),
scale changes ⇒ `size_logical = phys/scale` **also** changes (e.g. 800→640) **and** dppx changes, with **no**
`Resized`. The redraw-top chokepoint (D2) handles both: the `ScaleFactorChanged` arm requests a redraw;
redraw-top recomputes from the settled `(phys, scale)` and publishes the changed facts. **C2's pivot did not
create new work — it relocated the `ScaleFactorChanged` handling from C2 to C3, where it belongs (it has a
content-fact payload only here).**

---

## §2. The ideal mechanism (first-principles)

**Producer (shell, one steady-state chokepoint).** The browser thread is the single writer of the published
device state. The `ViewportCell` (D3) becomes `{ size, seq, dppx, color_scheme }`. Its writer
`publish_if_changed`-successor (call it `publish_device_state(size, dppx, color_scheme) -> DeviceDelta`)
returns a `DeviceDelta { size_changed: bool, facts_changed: bool }`:
- `size_changed` ⇒ bump `seq`, store size; the caller broadcasts `SetViewport { …, seq }` (C2 discipline,
  verbatim — input-drop protection intact).
- `facts_changed` (dppx or color_scheme differ) ⇒ store them, **no** seq bump; the caller broadcasts
  `SetDeviceFacts { color_scheme, dppx }`.

The **redraw-top recompute** (`threaded.rs:139`, already rebuilds `placement` every frame from the settled
window state) is extended to `placement = recompute` → `publish_device_state` → gate `broadcast_viewport`
(size) + `broadcast_device_facts` (dppx/scheme) + `reclip` on the returned `DeviceDelta`. `placement` and
`seq` thus update **atomically** here (D2). Event arms feed it by requesting a redraw only:
- `resumed` — **keep** the synchronous establish: recompute + publish-before-spawn (C1 construction-input
  seed; extend to also seed dppx/color_scheme so the spawned tab reads all facts by construction). The seed
  phase; redraw-top's first post-resume pass is an idempotent no-op.
- `Resized` — `gpu.resize` + `request_redraw` only. **No recompute, no publish, no reclip** (all → redraw-top,
  to keep placement↔seq atomic — F1).
- `ScaleFactorChanged` (NEW arm) — `gpu.resize` to the new physical size + `request_redraw`. No publish.
  (Closes F3: on the X11-only case this is the only event; it drives the redraw whose settled-state recompute
  ships the changed size **and** dppx.)
- `ThemeChanged(Theme)` (NEW arm; macOS/Windows — winit emits no theme event on X11/Wayland) — cache the
  color-scheme + `request_redraw`.

**Coupled invariants this design simultaneously satisfies + each pairwise intersection** (edge-dense §2
enumeration, F2):

| invariant pair | intersection (the one mechanism that holds both) |
|---|---|
| size-`seq`-iff-changed (C2) × redraw-cadence publish (D2) | `publish_device_state`'s `size_changed` arm bumps `seq` only on a real `size_logical` change, evaluated at redraw-top against the *settled* state — idempotent per frame, so cadence-driven publish never spuriously bumps |
| placement-recompute-site × input-stamp-site × `seq` (the F1 corner) | both `placement` and `seq` update **only** at redraw-top, atomically; input stamped in the `CursorMoved`/`MouseInput` arms (`current_placement_seq`) between an event and the redraw reads old-placement **and** old-seq (coherent), and is superseded together |
| device-fact-delta (dppx/scheme) × `seq`-suppression | facts travel the `facts_changed` arm with **no** seq bump (D3), so a pure-scale change the OS absorbs ships `SetDeviceFacts` without manufacturing a phantom input-drop generation |
| dppx producer × F3 X11-only coverage | the dppx fact's value *is* `scale_factor`, so "detect every DPI change and propagate" (F3) and "deliver dppx" are the same redraw-top recompute (D1) |
| `reclip` (stuck-`:hover` clear) × placement-geometry × redraw-cadence | `reclip` reads `origin_logical` + `scale_factor` + `size_logical` (`cursor_to_content`/`point_in_content`), so it is gated on a **full `placement != cached_placement`** comparison at redraw-top — NOT the size-only `DeviceDelta` (which omits `origin_logical`); it fires once per real placement change, all three axes covered (F1 re-check) |
| consumer canonicalization (D4) × both engines | one `elidex-css::media::evaluate` over a `MediaEnvironment` serves boa (routed) and the VM (already canonical) — facts in, one evaluator |

*One invariant: every device fact a content thread reads reflects the settled OS/display state, published
once per frame iff it changed, through one cell — with `placement` and `seq` always coherent.*

**Consumer (one canonical evaluator).** The content arm for `SetDeviceFacts` stores the facts into the engine
via bridge setters — `bridge.set_device_pixel_ratio(dppx)` (activating the **dead** setter `viewport.rs:61`,
never called today, which is why `window.devicePixelRatio` is stuck at 1.0) + a new `bridge.set_color_scheme`
— then calls the **existing** `re_evaluate_media_queries` + `dispatch_media_query_changes` path
(`event_loop.rs:338/354`, the #370/#372/#378-inherited MQL `change` machinery, reused verbatim). The boa
evaluator is routed through canonical `elidex-css` (D4), so `@media (prefers-color-scheme | resolution)` now
read the live facts. *One invariant: one evaluator (`elidex-css::media::evaluate`) serves both engines.*

---

## §3. Spec coverage map (preflight hard-gate)

| Spec | Surface | C3 delivery | observable-on-boa-today? |
|---|---|---|---|
| CSSOM View §4 `devicePixelRatio` (`#dom-window-devicepixelratio`) = `scale_factor` | dppx fact → bridge setter → getter (`screen.rs:133`) | **yes** (getter activates) |
| Media Queries L5 §5.1 `resolution` (dppx) (`#descdef-media-resolution`) | dppx fact → canonical evaluator | **yes** (via D4 routing) |
| Media Queries L5 §12.5 `prefers-color-scheme` (`#descdef-media-prefers-color-scheme`) | `ThemeChanged`/`window.theme()` fact → canonical evaluator | **yes on macOS/Windows**; Light default on X11/Wayland (winit emits no theme there — §7) |
| CSSOM View §4.2.1 `MediaQueryList` `change` event (`#eventdef-mediaquerylist-change`) | re-eval + fire `change` on fact change | reuse C1/#370/#372/#378 path (inherited) |
| HTML §8.1.7.3 update the rendering (`#update-the-rendering`) + CSSOM View §4.2 *evaluate media queries and report changes* (`#evaluate-media-queries-and-report-changes`) | one re-eval + one repaint per unified `SetDeviceFacts` | D5 (unified variant) |

**Breadth audit**: per-window single facts (dppx, color-scheme); the fan-out multiplicity is C1's (already
shipped). **User-input touch audit**: no untrusted web input — all inputs are OS/winit device facts
(scale, theme) + the internal redraw cadence. No trust-boundary enumeration needed.

---

## §4. The change (per file, post-C2 anchors)

**Producer — `crates/shell/elidex-shell/src/ipc.rs`**
- `ViewportCell` value `{ size, seq }` → `{ size, seq, dppx, color_scheme }`. `publish_if_changed(size)` →
  `publish_device_state(size, dppx, color_scheme) -> DeviceDelta { size_changed, facts_changed }` (size path
  bumps seq exactly as today; facts path no seq). `read()` exposes the facts for the broadcast.
- NEW `BrowserToContent::SetDeviceFacts { color_scheme, dppx }` adjacent to `SetViewport` (`:199`).
  `color_scheme` = `elidex_css::media::ColorScheme` (engine-independent type; not a winit `Theme`).

**Producer — `crates/shell/elidex-shell/src/app/{mod.rs,viewport.rs,threaded.rs}`**
- `app/mod.rs`: `Resized` arm (`:951-985`) — reduce to `gpu.resize` + `request_redraw` (move
  `content_area_placement` recompute + `publish` + `reclip` to redraw-top — F1/D2 atomicity). NEW
  `WindowEvent::ScaleFactorChanged` arm `(NEW arm)` (sibling) = `gpu.resize` (new phys) + `request_redraw`.
  NEW `WindowEvent::ThemeChanged` arm `(NEW arm)` = cache color-scheme + `request_redraw`. `resumed`
  (`:871-911`) — extend the synchronous establish to seed dppx/color_scheme alongside size, before
  `spawn_pending_initial_tab` (recompute + publish stay synchronous here — the seed phase).
  ⚠ **`app/mod.rs` is at 999 lines (F10)**: removing the Resized-arm publish/reclip body (→ redraw-top) is
  net-negative there, but the two NEW arms add back; **measure the post-change count and, if it crosses 1000
  with a real cohesion seam beyond the already-extracted `ViewportProducer` (#407), do a standalone prereq
  split** (not bundled into C3) per the touch-time discipline; else document the exemption (cohesive
  `App` event-dispatch unit).
- `app/viewport.rs`: a `broadcast_device_facts()` sibling of `broadcast_viewport()` (`:140`).
  `content_area_placement` already yields `scale_factor` (dppx source); add a `window.theme()` read for
  color-scheme (default Light on `None`).
- `app/threaded.rs`: at the redraw-top recompute (`:130-148`), capture `prev = self.viewport.placement`,
  recompute `placement = content_area_placement` (relocated from `Resized`) → `publish_device_state` →
  gate `broadcast_viewport` on `DeviceDelta.size_changed`, `broadcast_device_facts` on
  `DeviceDelta.facts_changed`, and **`reclip` on `Some(placement) != prev`** (full geometry comparison, NOT
  the size-only delta — F1 re-check: `reclip` reads `origin_logical`+`scale_factor`+`size_logical`). The
  single steady-state chokepoint (D2); `placement` + `seq` update atomically here.

**Consumer — `crates/shell/elidex-shell/src/content/{event_loop.rs,mod.rs}`**
- `event_loop.rs`: NEW `SetDeviceFacts` arm modeled on `VisibilityChanged` (`:362-373`, the pure-fact-push
  analog) → `bridge.set_device_pixel_ratio(dppx)` + `bridge.set_color_scheme(scheme)` → existing
  `re_evaluate_media_queries` + `dispatch_media_query_changes` + `re_render`.

**Consumer — `crates/script/elidex-js-boa/` (D4 routing — the cross-crate half)**
- `Cargo.toml`: `elidex-css.workspace = true` (transitive → direct, zero-cost).
- `bridge/mod.rs`: inner struct gains `color_scheme` next to `device_pixel_ratio` (`:418`); **replace the body**
  of `evaluate_media_query_raw` (`:774-801`, **deleting** the `:781-799` hand-rolled table + the `:796`
  `prefers-color-scheme => false` stub), changing its signature `(query:&str, w, h)` → `(query:&str, env:
  &MediaEnvironment)` and calling `elidex_css::media::evaluate(&parse_media_query_list(query), env)`.
- **The env is built at the two callers, from the SAME source, so they cannot diverge (F7, sub-check 2b
  reconciler-uniformity)**: `re_evaluate_media_queries(width, height)` (`bridge/media.rs:46`, the function the
  shell `SetViewport`/`SetDeviceFacts` arms both call) builds `MediaEnvironment { viewport_width: width,
  viewport_height: height, resolution_dppx: self.device_pixel_ratio(), color_scheme: self.color_scheme(),
  ..Default }` — i.e. viewport from its args, **device facts from the bridge's cached fields** (written by the
  `SetDeviceFacts` setters before re-eval). `matchMedia`'s `evaluate_media_query` (`globals/window/media_query.rs:38`)
  builds the same shape. Both mirror the VM's `media_environment()` (`vm/host/media_query.rs:298-307`). The
  public `re_evaluate_media_queries(width,height)` signature is **unchanged** (viewport still its args); only
  its internal env-build is new — so the shell call sites (`event_loop.rs:338`) need no change.
- `bridge/viewport.rs`: add `set_color_scheme`/`color_scheme` accessors mirroring the existing (now-activated)
  `set_device_pixel_ratio` (`:61`).

**No change**: `elidex-css` (consumer ready — `MediaEnvironment{resolution_dppx,color_scheme}` `types.rs:295/304`,
grammar parses both `media/parse.rs:716/726`); the MQL `change` machinery; the VM path (S5-dormant, already
canonical).

---

## §5. Edge matrix (the plan-review surface)

| Edge | Hazard | Resolution |
|---|---|---|
| **E1 — pure-scale change the OS absorbs** (phys resizes, `size_logical` preserved) | dppx changed, size unchanged ⇒ must ship `SetDeviceFacts` but **not** bump seq / drop input | D3: facts path is seq-independent; `size_changed=false` so no `SetViewport` |
| **E2 — X11 `ScaleFactorChanged`-only** (phys constrained, no `Resized`) | size **and** dppx changed, only event is `ScaleFactorChanged` | D2: arm requests redraw; redraw-top ships both. **Closes F3.** |
| **E3 — common-path `ScaleFactorChanged`+`Resized`** | event-arm publish would see stale `inner_size` → bogus `old_phys/new_scale` | D2: no event-arm publish; redraw-top sees settled state. No bogus intermediate. |
| **E4 — redraw-top publishes every frame** (animations) | publish spam | `publish_device_state` idempotent (no-op when nothing changed); no broadcast |
| **E5 — placement↔seq coherence across the event→redraw gap** (the F3 slot's caveat; plan-review F1) | input stamping happens in the *separate* `CursorMoved`/`MouseInput` arms (`current_placement_seq` = `cell.read().1`, `threaded.rs:317/339/384`), NOT in the redraw handler — so "publish is the first redraw block" does NOT order publish before stamping (different winit events). The real hazard: a *partial* move (recompute in `Resized`, seq bump at redraw-top) makes gap-input read **new** placement + **old** seq → dropped-as-stale = the phantom drop C2 prevents. | D2: move **both** `placement` recompute and `seq` bump to redraw-top (atomic). The `Resized`/`ScaleFactorChanged` arms touch neither, so across the event→redraw gap `placement` and `seq` are **both old** (coherent): gap-input maps old-placement, stamps old-seq, and is superseded together when the new viewport publishes. The deliberate delta (gap-input now dropped vs C2-applied) is in D2; geometrically it maps the pre-resize rect, so superseding is correct. |
| **E6 — `resumed` establish vs redraw-top steady-state** (two size-publish sites) | strangler / double-publish | distinct phases: resumed seeds before spawn (construction-input, synchronous); redraw-top's first post-resume publish is an idempotent no-op |
| **E7 — color-scheme producer on Linux** (winit: no `ThemeChanged`, `theme()→None` on X11/Wayland) | no live theme | default Light; `ThemeChanged` arm serves macOS (dev/test) + Windows. Documented platform limit, not a gap |
| **E8 — boa MQL `change` for the new facts** | does a fact change actually fire the MQL `change`? | D4 routing makes the canonical evaluator read the facts, so `re_evaluate_media_queries` flips them → existing dispatch fires `change` |
| **E9 — `f32`(boa viewport)↔`f64`(`MediaEnvironment`)** | precision at the env builder | widening cast, lossless |
| **E10 — zero-size `Resized` (minimize)** (F1 re-check) | C2 recomputed/published/reclipped unconditionally on `0×0`; redraw-top is `>0`-gated, so it now skips until restore | benign-or-better: drops C2's spurious `size_logical=0` generation (phantom seq bump content does not need while hidden); restore-`Resized` republishes. Stated as D2 delta (2). |
| **E11 — `reclip` origin axis** (F1 re-check) | `reclip` reads `origin_logical` too; a size-only gate would miss a future origin change (`#11-window-level-tab-bar-position`) | gate `reclip` on full `placement != cached` (D2 / §2 matrix / §4), not `DeviceDelta`; covers origin+scale+size by construction |

---

## §6. Testing / acceptance

- **dppx getter**: a delivered `SetDeviceFacts{dppx:2.0}` → `window.devicePixelRatio === 2` (was always 1.0).
- **`@media (resolution)`**: an MQL on `(resolution: 2dppx)` flips `matches` + fires `change` on the dppx
  delivery (canonical evaluator, D4).
- **`@media (prefers-color-scheme: dark)`**: flips + fires `change` on a `ThemeChanged(Dark)` (macOS/Windows
  path; unit-drives the content arm with a Dark fact).
- **F3 (the subsumed slot)**: a `ScaleFactorChanged`-only sequence (no `Resized`) → redraw-top recompute →
  `SetViewport`(new size) **and** `SetDeviceFacts`(new dppx) both delivered; queued input mapped against the
  prior seq is dropped iff `size_logical` actually changed (the X11-only case: it did). Reuse the C2 seq-test
  shape.
- **E1 regression**: an OS-absorbed pure-scale change → `SetDeviceFacts` delivered, **no** `seq` bump, queued
  input **not** dropped.
- **D4 routing**: `evaluate_media_query_raw` now returns canonical results for `prefers-color-scheme`/
  `resolution` (was `false`/unhandled) — a boa unit test on the routed evaluator.
- Supported-surface: shell content-thread + boa-bridge unit/integration tests; each IMPORTANT a regression
  test. The DPI/theme display-drag is a manual smoke step (not headlessly automatable).

---

## §7. What this PR explicitly does NOT do

- **No `prefers-reduced-motion`** — a third device fact (Media Queries L5 §12.1, `#descdef-media-prefers-reduced-motion`); its producer is a separate
  OS signal. The `SetDeviceFacts` variant (D5) is shaped so the future producer extends it (NOT a new
  variant). Stays in `#11-media-prefers-features` until its own producer slice.
- **No sub-pixel / fractional render fidelity** — `#11-hidpi-render-fidelity` (the geometric *paint*
  precision); C3 delivers dppx as a *content fact*, it does not re-bake rendering at the new scale.
- **No VM/S5 consumer work** — the VM is already a canonical `MediaEnvironment` consumer (S5-dormant); C3's
  routing is the **boa** half. At S5 the boa routing is deleted with boa (the durable artifacts — IPC
  variant, cell facts, shell producer — are engine-agnostic and survive).
- **No new MQL/`change` machinery** — reuses the inherited path.
- **No Linux theme** beyond the Light default (winit limitation, §5 E7).

---

## §8. Defer / slot disposition

- **`#11-shell-viewport-scalefactorchanged-x11-coverage` — CLOSED by C3** (D1). C3 adds the
  `ScaleFactorChanged` arm + the redraw-top chokepoint (option (c)) that the slot's design surface named,
  with the dppx-fact payload that makes it C3-scope. The slot's Re-eval trigger ("the umbrella's next PR-C
  axis-3 slice") is *this* slice. Records the close in `project_open-defer-slots` at landing.
- **`#11-media-prefers-features` — narrows (prefers-color-scheme + resolution discharged by C3; residual =
  `prefers-reduced-motion`)** (C-parent §10). **Why deferred**: `prefers-reduced-motion` needs a separate OS
  signal + producer (winit has no reduced-motion event); orthogonal to the dppx/color-scheme delivery.
  **Re-evaluation trigger**: a `prefers-reduced-motion` producer slice (OS reduced-motion source wired into
  the `SetDeviceFacts` variant, which D5 shaped to extend). **Re-evaluation date**: 2026-09-27.
- **`#11-device-fact-media-consumer-canonicalization` — NOT carved.** It was conditional on Q-C2 electing
  defer-to-S5 (a COUPLED boa↔`elidex-css` seam). The seam is **CLEAN** (D4), so the condition is not met.
  *(Self-contained fallback, F9 — if the focused Q1 re-check finds the seam COUPLED after all: **Why
  deferred** = boa media-evaluator too entangled to route pre-S5; **Re-evaluation trigger** = the VM (canonical
  consumer) replaces boa at S5; **Re-evaluation date** = S5 slice start. Then C3 ships the durable producer +
  IPC + cell store and defers the consumer to S5, AND `#11-media-prefers-features` defers *with* it.)*
- **`#11-shell-viewport-delivery` axis-3 — fully DISCHARGED** (C2 geometric + C3 device-fact); PR-C's three
  sub-slices (C1/C2/C3) complete the slot's producer-delivery breadth.

---

## §9. Open questions for `/elidex-plan-review`

- **Q1 (central) — is the D4 boa→canonical routing genuinely CLEAN, or does a boa-internal coupling lurk?**
  Memo's lean: CLEAN (pure `&str→bool` seam, all JsValue caller-side, `elidex-css` zero-cost transitive dep,
  identical `parse_inline_style` precedent). The plan-review should adversarially read the two callers
  (`bridge/media.rs:46`, `globals/window/media_query.rs:38`) for any `JsValue`/context entanglement that would
  turn the body-swap into a refactor. **If COUPLED after all → fall back to defer C3 to S5** (carve
  `#11-device-fact-media-consumer-canonicalization`).
- **Q2 — is the redraw-top publish relocation (D2, F3 option (c)) the right ideal, or does it over-restructure
  C2's just-landed publish?** Validate: (a) the E5 input-vs-redraw seq ordering proof (publish precedes input
  stamping every frame); (b) that no DPI change fails to `request_redraw` (so redraw-top always runs);
  (c) E6 (resumed establish vs redraw-top steady-state — two size-publish sites — is phase-distinct, not a
  strangler). Reject-to-event-arm only if redraw-top breaks an ordering the event arm preserved.
- **Q3 — `ViewportCell` → device-state cell (D3): one cell with per-fact delta, or a separate device-fact
  dedup beside the size cell?** Memo leans one cell (One-issue-one-way; the seq stays size-only inside it).
  Confirm the `DeviceDelta` return keeps the size-seq discipline byte-identical to C2.
- **Q4 — unified `SetDeviceFacts{color_scheme,dppx}` (D5/Q-C1) vs separate variants?** Memo leans unified
  (matches the VM `set_media_environment` arity; atomic re-eval). Confirm the content-side value-guards
  suppress no-op re-renders when only one fact changed.
- **Q5 — does C3 need a `window.theme()` read at `resumed`/build time** (seed the initial color-scheme), or is
  the Light default + `ThemeChanged` arm sufficient? (macOS dev/test: `theme()` works, so seeding gives the
  real initial scheme; Linux: `None`→Light either way.)
- **Q6 — is shipping color-scheme worth it given the Linux winit limitation (E7)?** dppx works everywhere;
  color-scheme is macOS/Windows-only observable. Memo's lean: yes — macOS is the dev/test platform, the D4
  routing retires the stub regardless (it's the *consumer* that's the One-issue-one-way win), and Linux Light
  is a faithful default. (If the panel disagrees, C3 could ship dppx-only and defer color-scheme — but that
  re-splits the one device-fact path D-C1 unifies.)

---

## §10. Citation appendix (webref re-verify at plan-review, Axis 4)

- CSSOM View Module Level 1 — §4: `devicePixelRatio` `#dom-window-devicepixelratio`; §4.2.1 `MediaQueryList`
  `change` event `#eventdef-mediaquerylist-change` (the interface dfn is `#mediaquerylist`); §4.2 *evaluate
  media queries and report changes* `#evaluate-media-queries-and-report-changes`.
- HTML — §8.1.7.3 *update the rendering* `#update-the-rendering` (the cadence that invokes the CSSOM-View
  re-eval; "update the rendering" is an HTML concept, not CSSOM-View).
- Media Queries Level 5 — `prefers-color-scheme` §12.5 (`#descdef-media-prefers-color-scheme`), `resolution`
  §5.1 (`#descdef-media-resolution`), `prefers-reduced-motion` §12.1 (`#descdef-media-prefers-reduced-motion`)
  — plan-review Axis 4 re-verified 2026-06-27.
- winit 0.30.13 (source-verified this slice): `event.rs:379` `ScaleFactorChanged{scale_factor,inner_size_writer}`,
  `:980-1002` `InnerSizeWriter` (write-only), `:396` `ThemeChanged` (X11/Wayland unsupported); `window.rs:1383`
  `theme()→Option<Theme>`; `x11/event_processor.rs:742-747` (the forcing condition); `macos/window_delegate.rs:844`
  (unconditional `Resized`).
