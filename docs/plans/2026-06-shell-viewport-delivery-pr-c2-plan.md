# Shell viewport delivery — PR-C **C2** (DPI/scale placement refresh) plan-memo

Slot: `#11-shell-viewport-delivery` (PR-C, sub-slice **C2**, axis-3 "ScaleFactorChanged / DPI placement
refresh"). Registered at PR-B landing (#388, citing Codex #388 R1b/R6).
Parent (PR-C sub-umbrella): `docs/plans/2026-06-shell-viewport-delivery-pr-c-plan.md` (realizes its **C2**
row). Grand-parent (program umbrella): `docs/plans/2026-06-shell-viewport-delivery-plan.md`.
Prior slices: PR-A ✅ #386 (repaint-wake), PR-B ✅ #388 (placement SoT + scale plumbing), C1 ✅ #396
(construction-input + `ViewportCell` latest-value cell + seq reconciliation + input-seq drop).

Anchors re-grepped at HEAD `75c327c1` (2026-06-26, includes #407 `ViewportProducer` extraction + #408).

C2's one invariant: **a DPI/scale change refreshes the browser-side placement SoT (compositor scale +
input ÷scale) and notifies content of a `size_logical` change iff one actually occurred — the
`ViewportCell` seq advances iff `size_logical` changed, so it never spuriously supersedes legitimately-
queued input.**

---

## §0. Decisions this memo commits to

> **R3 post-implementation correction (Codex #411, 2026-06-27, both source-verified).** Two findings on the
> landed mechanism, neither known when this memo was first written:
> - **F2 (landed in C2).** `content_area_placement` divided `phys / scale` in **f32**, so a logical-preserving
>   DPI change round-trips to a *different* f32 (960px @ 1.2 → 799.99994 ≠ 800.0), which `Size`'s exact
>   equality treats as a new generation — reintroducing the very §2 input-drop. Fixed by dividing in **f64**
>   (`logical_px`, `app/viewport.rs`) so the value recovers bit-exactly; exact-eq then holds *by construction*
>   (retires §9 Q4). Regression test added.
> - **F3 (carved, NOT in C2).** **D1's X11 premise is false.** winit 0.30.13 `x11/event_processor.rs:715-748`
>   forces the follow-up `Resized` **only when** the DPI-adjusted physical size differs; a constrained /
>   tiling-WM X11 window whose physical size rounds back gets `ScaleFactorChanged` **alone**, so its logical
>   viewport (`phys / new_scale`) changes with no `Resized` and content goes stale. The fix is edge-dense (no
>   canonical algorithm — a naive "route `ScaleFactorChanged` through the chokepoint" is *wrong*: it
>   broadcasts a bogus `old_phys / new_scale` intermediate on the common Resized-follows path) and reverses a
>   plan-reviewed decision → carved to **`#11-shell-viewport-scalefactorchanged-x11-coverage`** for
>   `/elidex-plan-review` (§8), not bundled into this converge loop (CLAUDE.md edge-dense: 単一 PR に束ねない).

- **D1 — C2 (this slice) adds no `ScaleFactorChanged` handler; the X11 `ScaleFactorChanged`-only gap is
  carved, not closed here.** On macOS / Wayland / Windows a `Resized` follows every `ScaleFactorChanged`
  (§1, source-verified), and on X11 it does too **whenever the DPI-adjusted physical size changes** — at that
  `Resized` `window.scale_factor()`/`inner_size()` already return the new values and the existing arm
  (`app/mod.rs`) performs the full DPI refresh (gpu resize + placement recompute + content notify + reclip +
  redraw), so for the **common** DPI path a parallel handler is redundant (*Ideal over pragmatic* + *dead code
  は接続するか削除* + *設計優先 (場当たり reactive fix 禁止)*). **But** winit 0.30.13's X11 backend forces that
  follow-up `Resized` *only* when the physical size differs (R3/F3 above), so the `ScaleFactorChanged`-only
  corner is a real coverage gap — closed in its own plan-reviewed slice
  (`#11-shell-viewport-scalefactorchanged-x11-coverage`), not C2.

- **D2 — the lens-correct content of axis-3 is enforcing the seq invariant at the publish chokepoint:
  `seq` advances iff `size_logical` changed.** Replace the unconditional `ViewportCell::publish` with a
  single canonical `publish_if_changed(size) -> bool` (returns whether the size differed → seq bumped).
  Both producer call sites (`resumed` `mod.rs:891`, `Resized` `mod.rs:965`) route through it; **`broadcast`
  is gated on the return**. This (a) fixes the pure-scale input-drop (§2), (b) collapses the duplicated
  `publish → broadcast` pair into one form — *One-issue-one-way*. The old unconditional `publish` is
  **deleted** (後方互換性は維持しない).

- **D3 — the content side is unchanged.** `content/event_loop.rs:292-323` reconciliation (staleness drop +
  unconditional `applied_viewport_seq` advance + CSSOM View §13.1 value guard) is spec-mandated and
  orthogonal to producer publish discipline; `publish_if_changed` only changes *which* deliveries the
  producer emits, never how a delivery is reconciled. No `content/`, no IPC-protocol, no `vello_backend`,
  no scale-plumbing change.

- **D4 — `publish_if_changed` is a pure, window-free, unit-testable primitive.** The original framing's
  worry ("`ScaleFactorChanged` は real window なしに unit test しづらい") is **dissolved by D1**: there is no
  window-coupled handler to test. The seq discipline is tested directly on `ViewportCell` (§6).

## §1. The pivot — the axis-3 premise is false on winit 0.30; reframe, do not add a handler

The slot/umbrella framed C2 as "純 DPI 変化は `Resized` を発火しない → `ScaleFactorChanged` を別 handler で
処理 → placement 再導出 → cell publish (3rd site) → broadcast → repaint." **That premise is false for winit
0.30.13.** Source-verified (winit 0.30.13, `~/.cargo/.../winit-0.30.13/src`):

| Platform | Contract (winit 0.30.13 source) |
|---|---|
| **macOS** (dev/test platform) | `platform_impl/macos/window_delegate.rs:832-844` — emits `ScaleFactorChanged`, then **unconditionally** `Resized(physical_size)` immediately after; `scale_factor` updated at `:198` *before* the pair; default `suggested_size` is logical-preserving (`content_size.to_physical(scale)` `:830`). |
| **Wayland** | `platform_impl/linux/wayland/event_loop/mod.rs:411` — `if resized \|\| scale_changed` → `Resized(physical_size)` (`:434`) **and** `redraw_requested = true` (`:425`). |
| **Windows** | `platform_impl/windows/event_loop.rs:2216` `WM_DPICHANGED` → `ScaleFactorChanged` (`:2285`) → `SetWindowPos` → `WM_SIZE` → `Resized` (`:1398-1405`). |
| **X11** | `platform_impl/linux/x11/event_processor.rs:715-748` — emits `ScaleFactorChanged` on every scale change, but forces the follow-up `Resized` **only when** the DPI-adjusted physical size differs (`resized = true` gated on `new_inner_size != old_inner_size`, `:738-743`). **⚠ The premise FAILS here** (R3/F3, 2026-06-27): a constrained / tiling-WM window whose scaled physical size rounds back to the old size gets `ScaleFactorChanged` **alone** — the v0.18 changelog "`Resized` always follows" is *not* unconditional in 0.30.13. Carved to `#11-shell-viewport-scalefactorchanged-x11-coverage`. |

On macOS / Wayland / Windows (and on X11 **when the physical size changes**) `Resized` follows
`ScaleFactorChanged`, and at that `Resized` `window.scale_factor()` returns the **new** scale and
`inner_size()` the **new** physical size. So for that path elidex's existing `Resized` arm already performs
the full DPI refresh the slot asked for (the X11 `ScaleFactorChanged`-only exception is R3/F3 above):

- `state.gpu.resize(..)` → surface to new physical (`mod.rs:951`).
- `content_area_placement(&window)` (`app/viewport.rs:69-80`) reads the **new** `window.scale_factor()` (`:70`)
  + **new** `inner_size()` (`:71`) → `placement.scale_factor` updated → compositor `base_transform` +
  `clip_rect` (`vello_backend.rs:54-70`, fed from `placement` at `threaded.rs:141-148`) and input `÷scale`
  (`threaded.rs:236-245`) both pick up the new scale on the requested redraw.
- `size_logical` recomputed (`app/viewport.rs:77`): **unchanged** for a pure scale change (logical-preserving),
  correctly **changed** if the OS also changed the logical size.
- `viewport_cell.publish` + `broadcast_viewport` → content notified; `reclip_cursor_after_placement_change`
  → stuck `:hover` cleared.

**dppx / `window.devicePixelRatio` are C3** (device facts), not C2 (geometric scale only — umbrella §3; PR-B
refused to re-bake `scale==1`; `#11-hidpi-render-fidelity` owns sub-pixel fidelity). So a "geometric scale
only" C2 has **no content-facing payload beyond `size_logical`**, which the `Resized` arm already carries.
A dedicated `ScaleFactorChanged` handler is therefore redundant (D1). The Codex #388 R1b/R6 citation
reflects winit's pre-0.18 model or an abstract spec concern; the winit-0.30 contract was never checked when
the slot was carved. (Same failure class, inverted, as `feedback_existing-infra-production-completeness-
premise` (#383): there elidex *assumed* infra complete; here the slot *assumed* infra missing — both demand
verifying the premise. Verified here; the residual is §2, the manual confirmation is §6.)

## §2. The real residual + coupled invariants — the seq spuriously supersedes input on a pure-scale resize

`ViewportCell::publish` (`ipc.rs:65-69`) bumps `seq` **unconditionally**. The `Resized` arm
(`mod.rs:965-966`) calls it on every resize, then `broadcast_viewport`. On a **pure scale change**
(`size_logical` unchanged — the common macOS monitor-drag case):

1. `publish(same size)` → `seq: N → N+1` (size identical, seq still bumps).
2. `broadcast_viewport` → `SetViewport { same size, seq N+1 }` to every tab.
3. Content (`event_loop.rs:300-321`): `N+1 > applied N` → `applied_viewport_seq = N+1` (advance is
   **unconditional**, `:308`), then the CSSOM View §13.1 value guard (`:319-321`) no-ops the `resize` event
   (size unchanged). **So `applied_viewport_seq` advanced with no layout change.**
4. Any `MouseClick`/`MouseMove`/`MouseWheel` queued carrying the **old** stamp `placement_seq = N`
   (browser-stamped via `current_placement_seq` `app/viewport.rs:136-138`) is now judged stale
   (`input_placement_stale` `event_loop.rs:204-205`: `N < N+1`) and **dropped** — though it was mapped
   against a layout (keyed to `size_logical`) that **did not change**. → a click/scroll lost during a DPI
   change.

This violates the documented seq invariant. **Coupled invariants this design simultaneously satisfies**
(per `feedback_coupled-invariant-design-corner`):

- **seq-monotonicity × size-generation**: seq must mean "`size_logical` generation," so it must bump iff
  `size_logical` changes. Intersection = the `publish_if_changed` guard (`guard.size == size` short-circuit).
- **seq × input-drop**: `input_placement_stale` drops input with `placement_seq < applied_viewport_seq`.
  Intersection = a no-op size change must NOT advance `applied`, so the producer must not emit it ⇒ broadcast
  gated on `published`.
- **seq × CSSOM View §13.1 value-idempotency**: the content value guard is the *consumer*-side dedup; the
  `publish_if_changed` guard is the *producer*-side dedup. They are independent layers — §13.1 stays for
  spec conformance + the build-vs-broadcast race (E5); `publish_if_changed` removes the redundant emission
  that triggered the input-drop. Neither subsumes the other.
- **scale × layout-seq (the non-coupling that must be preserved)**: scale changes the *browser-side*
  physical→CSS input map (`cursor_to_content` `threaded.rs:236`, re-applied per fresh winit event) but
  **not** the CSS-px content layout. So a scale change must **not** advance the layout seq — exactly what
  `publish_if_changed` guarantees (no `size_logical` delta ⇒ no bump).

This is the dual of the C1 R2 regression (Codex #396 R2: a seq *guard* dropping a queued resize made input
hit-test the wrong layout); here a seq *bump* drops input against an *unchanged* layout.

**ECS-native homing (no ECS DOM component involved).** `ViewportCell.{size,seq}` and the per-thread
`applied_viewport_seq`/`placement_seq` are **not** per-entity DOM state and do **not** become ECS components.
`ViewportCell` is per-window **shared cross-cutting shell state** (one `Arc<ViewportCell>` read by every tab
of a window — the CLAUDE.md side-store exception (b), the cookie-jar/`NetworkHandle` category), and the seq
fields are producer/consumer-thread struct state. The side-store→component rule does not fire; this is the
correct home, and `publish_if_changed` is a primitive on that already-correctly-homed cell, not a new store.

## §3. Spec coverage map (preflight hard-gate)

C2 touches **no spec algorithm** — it is a browser-side IPC/seq-discipline change. The single spec invariant
in scope is one the PR *preserves* (on the unchanged consumer side). **K=1 unique spec (CSSOM View), M=1 row
→ single PR** (far under the K≥4/M≥20 split heuristic; the only genuine "is this worth a PR" question is §9
Q5, a priority question, not a spec-breadth one).

| Spec section | Step / context | Branch | Touch (code site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM View §13.1 Resizing viewports — "run the resize steps" (`#document-run-the-resize-steps`) | `resize` fires only when width/height changed since last run (step 1) | (a) size changed → fire / (b) unchanged → no-op | **consumer UNCHANGED** (`content/event_loop.rs:319-321`); producer change only *reduces* same-size deliveries, so §13.1 is preserved a fortiori | ✓ | no (OS device fact) |

### §3.1 Breadth + user-input touch audit

- **Breadth**: one preserved invariant; no spec-step branch matrix (discipline PR, not algorithm impl). The
  producer change cannot regress §13.1 — it strictly removes same-size emissions (branch (b) inputs),
  never adds one, so the consumer's branch coverage is unchanged.
- **User-input touch**: C2 reads **no** untrusted web input — `size_logical`/scale are OS/winit device facts
  + internal tab lifecycle. No trust-boundary enumeration needed.
- **Out of C2 scope (bounding the surface)**: CSSOM View §4 `devicePixelRatio` (`#dom-window-devicepixelratio`)
  / Media Queries L5 §12.5 `prefers-color-scheme` / §5.1 `resolution` → **C3** (device facts). (Anchors
  webref-verified 2026-06-26.)

*(CSSOM View label is not in the preflight reverse map — expected soft-warn, not citation drift. §13.1
"Resizing viewports" / `#document-run-the-resize-steps` webref-verified 2026-06-26 via `dfn cssom-view-1
'run the resize steps'`.)*

## §4. The change (browser-side only)

**`ViewportCell` (`ipc.rs:48-78`)** — replace `publish` with `publish_if_changed` (NEW):

```rust
/// Browser writer: store `size` and bump the monotonic seq **iff** `size` differs from
/// the current value; returns whether it changed (and thus published a new seq).
///
/// `seq` identifies `size_logical` generations (the unit `placement_seq`/`applied_viewport_seq`
/// reconcile against, `event_loop.rs:204` / `ipc.rs:108`). Bumping on an unchanged size would
/// manufacture a phantom generation that spuriously supersedes queued input mapped against the
/// still-current layout (a pure DPI/scale `Resized` carries the same `size_logical`).
#[must_use]
pub fn publish_if_changed(&self, size: Size) -> bool {
    let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
    if guard.size == size {
        return false;
    }
    guard.size = size;
    guard.seq += 1;
    true
}
```

(`Size` is `Copy + PartialEq` — `elidex_plugin::Size` `layout_types/rect.rs:18` `derive(... PartialEq)`,
verified 2026-06-26. Exact-`f32` equality is correct **given `size_logical` is computed via f64 division**
(`logical_px`, `app/viewport.rs`, R3/F2): a logical-preserving pure-scale change then recovers a bit-identical
value, so this stays a "did the producer compute a different value" test, not a tolerance question. An **f32**
division would *not* round-trip — `960px @ 1.2 → 799.99994 ≠ 800.0` would spuriously bump (R3/F2); the bug is
fixed at the source rather than with an epsilon. See §9 Q4.)

**`resumed` (`mod.rs:886-906`)** — `viewport_cell.publish(placement.size_logical)` (`:891`) becomes
`let published = self.viewport.viewport_cell.publish_if_changed(placement.size_logical);` before the spawn;
the post-spawn `broadcast_viewport()` (`:906`) becomes `if published { self.broadcast_viewport(); }`.
(First resume: `DEFAULT → real` is a change ⇒ `published`, seq 0→1; the just-spawned tab reads the bumped
cell at build, so the broadcast it receives is dropped by the staleness guard — as today. `real == DEFAULT`
corner → §5-E1.)

**`Resized` (`mod.rs:958-966`)** — `publish` (`:965`) → `publish_if_changed`; `broadcast_viewport()` (`:966`)
gated `if published`. The placement-cache update (`:964`), `gpu.resize` (`:951`), `request_redraw` (`:955`),
and `reclip` (`:969-971`) are **unchanged** — they run on every `Resized` regardless of whether
`size_logical` changed (the compositor/input still need the new scale even when `size_logical` is constant —
the DPI path, already working — §1). Only the *content notification* (publish/broadcast) becomes conditional.

**`content_area_placement` (`app/viewport.rs`, R3/F2)** — extract a `logical_px(physical: u32, scale: f64) -> f32`
helper and compute `win_logical_{w,h}` by **f64** division, narrowing to f32 once, instead of casting `scale`
to f32 first and dividing in f32. This makes a logical-preserving pure-scale change recover a bit-identical
`size_logical` so the §4 exact-eq guard does not manufacture a phantom generation from an f32 round-trip
(`960px @ 1.2 → 799.99994`). The `placement.scale_factor` field stays f32 (compositor/input tolerate it).

No new event arm; no `content/`, `vello_backend`, or IPC-message change. **`publish` has 4 callers**
(verified 2026-06-26 via `grep -rn '\.publish(' crates/shell/elidex-shell/src` excluding the def): 2
production (`mod.rs:891` resumed + `mod.rs:965` Resized) + **2 existing test** (`viewport_tests.rs:331`
`content_thread_builds_at_latest_published_cell_size` + `:384` `content_thread_drops_stale_seq_viewport`).
All 4 migrate to `publish_if_changed`, but they are **not** uniform mechanical swaps:
- `:331` seeds 1024×768 then publishes **640×480** (a genuine size change) → `publish_if_changed` returns
  `true`, bumps to seq 1 — behavior identical, mechanical swap (bind/assert the `#[must_use]` return).
- `:384` seeds 800×600 then publishes **800×600** (the *same* size). It currently reaches "seq 1" *only
  because the old `publish` bumps seq on a same-size publish* — the exact behavior `publish_if_changed`
  removes. A literal swap would leave seq at 0 and break the test's "cell (800px, seq 1) … build reads
  seq 1 as its mark" premise (`:382`). **Forced fixture change** (not a swap): reseed the cell at a
  *different* size (e.g. 640×480) then `publish_if_changed(800×600)` → seq 0→1 with size 800×600,
  preserving the test's seq-1-mark intent via a *real* size change. (This test's old setup relying on
  same-size-publish-bumps-seq is itself evidence the design is removing a real, exercised behavior —
  the fixture redesign *validates* `publish_if_changed`, it is not incidental churn.)

## §5. Edge matrix (invariant interactions — the plan-review surface)

- **E1 — resume with `real == DEFAULT`.** `publish_if_changed` returns false, seq stays 0, no broadcast.
  Spawn reads `(DEFAULT, 0)` = correct size, `applied = 0`. A later real `Resized` to a different size
  publishes seq 1 > 0 → applies. ✓ (Delta vs today: seq is 0 not 1 in this corner; harmless — seq is only
  ever compared, never assumed to start at 1.)
- **E2 — pure scale `Resized` (the §2 bug, the fix target).** `size_logical` unchanged → no publish, no
  broadcast, `applied_viewport_seq` unmoved → queued input (old seq) **not** dropped. Browser-side placement
  still updated (`:964`) + redraw requested (`:955`) → compositor renders at new scale, input maps under new
  scale. ✓
- **E3 — scale change that *also* changes `size_logical`** (OS overrode logical size / combined resize+DPI
  on monitor move). `size_logical` differs → exactly one publish + broadcast → content re-lays-out once. ✓
  (No double-bump even though `ScaleFactorChanged`+`Resized` both fire: only `Resized` publishes, once.)
- **E4 — same-size non-scale `Resized`** (drag netting zero content-size change after chrome rounding).
  Today: spurious bump+broadcast (= §2). After: no-op. ✓ (Strictly-more-correct; §2 is a special case.)
- **E5 — suspend → resume, size changed while suspended.** `suspended` drops `placement` but not the cell
  (`mod.rs:921-924`); cell retains `(size_A, seq_N)`. Resume recomputes `size_B ≠ size_A` →
  `publish_if_changed` bumps → gated broadcast fans `size_B` to persisted tabs → re-layout. ✓ Size unchanged
  across suspend → no publish/broadcast, persisted tabs already correct. ✓ (The `event_loop.rs:315-318`
  comment — resume broadcast "fans unconditionally, value-idempotency absorbs it" — is now superseded for
  the resume case: we fan only on a real change. The value guard stays for §13.1 + the build-vs-broadcast
  staleness race. **Update that comment** as part of this PR.)
- **E6 — multi-tab fan-out.** Unchanged: `broadcast_viewport` (`app/viewport.rs:115-125`) still fans the cell's
  current seq to all tabs; it just runs only when `publish_if_changed` reported a change. New/`window.open`
  tabs still born at the real size via the construction-input cell read (C1) — unaffected. ✓
- **E7 — `current_placement_seq` stamping across a pure-scale resize.** Input sent before *or* after the
  (no-op) event is stamped with the un-bumped seq N, matching content's `applied = N` → not dropped.
  Consistent on both sides. ✓

## §6. Testing — supported-surface coverage

**Home = `crates/shell/elidex-shell/src/viewport_tests.rs`** (633 lines, the existing cell/seq/input-drop
suite — `content_thread_drops_stale_seq_viewport` `:378`, the C1 R2 input-drop test `:468`). New tests land
here as siblings (well under 1000 lines after ~30 added). The 2 existing `publish` test callers (§4) migrate
in place — `:331` mechanically, `:384` with the **forced reseed** (§4) so it still reaches seq 1 via a real
size change (this fixture migration is itself part of the test surface: it confirms same-size publishes no
longer advance seq).

- **Unit (`ViewportCell`, window-free) — the headline guard (`publish_if_changed_bumps_seq_only_on_size_change`):**
  fresh `s`-seeded cell: `publish_if_changed(s)` → `false` + seq unchanged; `publish_if_changed(s')` (s'≠s) →
  `true` + seq+1; repeated same-size → never bumps; alternating → +1 each. **This `bool` return IS the
  producer-emission assertion** — the caller's `if published { broadcast }` skips `broadcast_viewport`
  exactly when this returns `false`, so "a same-size resize emits zero `SetViewport`" reduces to "no-op →
  `false`," which this test pins. (Directly guards D2's seq invariant + the gate signal.)
- **§2/E2 input-survival is covered by composition, not a new test:** with the unit test proving a same-size
  `Resized` no longer *emits* a seq-bumped `SetViewport`, the content `applied_viewport_seq` is not advanced,
  so input stamped at the build seq survives — the *consumer*-side input-drop logic is already pinned by the
  existing `content_thread_input_dropped_against_superseded_placement`-style test (`viewport_tests.rs:468`,
  Codex #396 R2) + the reseeded `content_thread_drops_stale_seq_viewport`. The producer fix + these unchanged
  consumer tests close the loop; a bespoke "App Resized → no broadcast" integration test would need a real
  window (window-coupled, not headless — see the manual smoke) and would re-derive the consumer behavior.
- **Manual macOS DPI smoke (out-of-band, the #383-lesson de-risk):** run the shell, drag the window between
  a Retina and non-Retina display (or toggle display scale). Expect: content re-renders sharp at the new
  scale, cursor hit-testing stays aligned, no lost click mid-drag, `innerWidth` (= `size_logical`) unchanged.
  (Not headlessly automatable — DPI changes need a real multi-monitor session; recorded as the manual
  acceptance step. Source contract §1 is what we rely on; this confirms it empirically.)

## §7. What this PR explicitly does NOT do

- No `WindowEvent::ScaleFactorChanged` arm **in C2** (D1 — the `Resized` follow-up covers the *common* DPI
  path; the X11 `ScaleFactorChanged`-only gap where no `Resized` follows is real, R3/F3, and carved to
  `#11-shell-viewport-scalefactorchanged-x11-coverage` — not closed here).
- No `inner_size_writer` servicing — the OS default (logical-preserving, `window_delegate.rs:830`) is exactly
  elidex's desired DPI policy (CSS px scale-invariant); overriding it is a non-goal.
- No `content/` *behavior*, IPC-protocol, `vello_backend`, or scale-plumbing change — PR-B's compositor
  base-transform + clip + input ÷scale + first-class `scale` param (D2) reused as-is. (One `content/event_loop.rs`
  **comment** is updated — the §13.1 value-guard rationale, since the resume broadcast is no longer
  unconditional; comment-only, no code/behavior change.)
- No dppx / `devicePixelRatio` / prefers-* work — that is C3 (device facts), kept separate per the umbrella.

## §8. Defer / slot disposition

- **`#11-shell-viewport-delivery` axis-3** — **common DPI path discharged** by this PR (seq-invariant fix +
  the f64 precision fix, R3/F2). **The X11 `ScaleFactorChanged`-only corner is NOT discharged** — D1's premise
  was false there (R3/F3) → carved to the slot below. axis-2 (multi-tab) already discharged by C1; **C3
  (prefers/dppx device facts) remains** the open PR-C axis.
- **`#11-shell-viewport-scalefactorchanged-x11-coverage` (CARVED, `/elidex-plan-review` required).** Real
  coverage gap: winit 0.30.13 X11 emits `ScaleFactorChanged` without a following `Resized` when the
  DPI-adjusted physical size rounds back, so a constrained / tiling-WM window's logical viewport
  (`phys / new_scale`) changes unobserved (R3/F3, source-verified). **Edge-dense** (intersecting invariants:
  seq-iff-size, input-vs-redraw ordering, no spurious intermediate broadcast, platform event coverage) with
  **no canonical algorithm** → design surface for the panel: (a) a `ScaleFactorChanged` handler that
  replicates winit's Resized-forcing condition (publish only when no `Resized` will follow); (b) read winit's
  proposed phys via the event's `inner_size_writer`; (c) move the publish to the single redraw-top recompute
  point (subsumes every event by construction, **but must prove it preserves the input-vs-redraw seq
  ordering** C2 protects). The naive "route `ScaleFactorChanged` straight through `publish_if_changed`" is
  **wrong** — it broadcasts a bogus `old_phys / new_scale` intermediate on the common Resized-follows path.
  Supersedes the speculative `#11-winit-scale-contract-regression-guard` (the contract gap is now *actual*,
  not hypothetical, so the eligibility-audit that rejected the guard now passes for this slot).
  **Why deferred**: edge-dense (no canonical algorithm) + reverses plan-reviewed D1 → `/elidex-plan-review`
  required, not improvised in the converge loop (the naive route-through-chokepoint is wrong, above).
  **Re-evaluation trigger**: a DPI / X11-coverage pass, OR the umbrella's next PR-C axis-3 slice, OR a
  WPT/site exercising a Linux tiling-WM scale change, OR S5 / agent-scoped World (B1, supersedes `world_id`) viewport work touching the publish
  path. **Re-evaluation date**: 2026-08-26.
- Untouched siblings: `#11-iframe-build-viewport` (per-child viewport), `#11-hidpi-render-fidelity`
  (sub-pixel/fractional re-bake), `#11-content-message-coordination-wake` — all out of C2 scope.
- **Parallel-branch collision clearance (parent §8 per-sub-slice mandate).** C2's edit sites — `ipc.rs`
  (`ViewportCell`), `app/mod.rs` (resumed `:886-906` + Resized `:947-972`), `viewport_tests.rs` — are
  **disjoint** from the only active shell-touching branch, E0 `feat/e0-shell-style-compat-mode` (#406,
  style-compat in `re_render`/builders/parse-path): `git diff --stat origin/main feat/e0-shell-style-
  compat-mode -- crates/shell/elidex-shell/src/app/ …/ipc.rs …/viewport_tests.rs` = **empty** (verified
  2026-06-26). `b1.2b-3` is engine-side (DOM validity), no shell `app/` touch. No rebase collision expected.

## §9. Open questions for `/elidex-plan-review`

- **Q1 — Is reframing axis-3 from "add a `ScaleFactorChanged` handler" to "fix the publish seq-invariant"
  faithful to the slot's *intent* (DPI placement refresh correctness), or does it drop a real obligation?**
  (My read: the only obligation the original framing carried that the `Resized` arm doesn't already meet is
  the §2 seq bug; everything else is redundant. Challenge this — is there a DPI scenario the `Resized` arm
  genuinely misses on a supported platform?) **[ANSWERED R3/F3: yes — X11 `ScaleFactorChanged`-only (no
  following `Resized`) misses it; carved to `#11-shell-viewport-scalefactorchanged-x11-coverage`.]**
- **Q2 — `publish_if_changed` at the `resumed` site:** is folding the initial `DEFAULT→real` establishment
  through the same conditional primitive (vs keeping `resumed` an unconditional establish-publish) the right
  *One-issue-one-way* call given the `real == DEFAULT` corner (E1) changes the seq-counter start? Or does the
  establishment semantically differ enough to warrant a distinct entry point?
- **Q3 — Gating the `resumed` broadcast (`mod.rs:906`) on `published`** retires the `event_loop.rs:315-318`
  "fan unconditionally" contract for the resume case. Confirm nothing else relies on an *unconditional*
  resume broadcast (the §13.1 value guard stays).
- **Q4 — Exact-`f32` equality in `publish_if_changed`.** `size_logical` is recomputed from identical inputs
  on an unchanged window, so equality holds bit-exactly — but `content_area_placement`'s arithmetic
  (`win_logical = phys / scale`, `chrome::content_size` subtraction `app/viewport.rs:72-77`): is there a path
  where the *same* physical window yields a different `f32` across two computes (NaN, sub-normal, operation
  reordering)? If so, a no-op resize could still bump. **[ANSWERED R3/F2: the *unchanged-window* case is
  safe, but the adjacent *pure-scale* case (different `phys`+`scale`, same logical) was NOT — f32 division
  round-trips `960px @ 1.2` to `799.99994 ≠ 800.0` and bumped. Fixed by computing the division in f64
  (`logical_px`), making exact-eq hold by construction across pure-scale changes; not a tolerance.]**
- **Q5 — Is this worth a PR at all, vs registering the §2 edge as a low-priority slot and moving to
  media/S5?** The fix is small + canonical + closes a documented-invariant violation in the exact spot Codex
  caught C1 R2; the alternative leaves a known ad-hoc/incorrect state (decision-surface tax). Rule on priority.
