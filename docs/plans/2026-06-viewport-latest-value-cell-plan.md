# Viewport latest-value cell — plan-memo (PR-C C1 structural follow-up)

Slot: `#11-viewport-latest-value-cell` (to register). Parent: `docs/plans/2026-06-shell-viewport-delivery-pr-c1-plan.md`
(this realizes its §2 ideal — "viewport is a fact the content thread needs as **input**, not shared mutable
state to be reconciled after the fact" — that the shipped C1 only *approximated*). Grand-parent (PR-C
sub-umbrella): `docs/plans/2026-06-shell-viewport-delivery-pr-c-plan.md`.

Anchors re-grepped at HEAD `480c02f7` (2026-06-22, the drain/replay revert).

This memo's one invariant: **a content thread never builds at, nor diverges to, a stale viewport — the
window content-area size it builds against is the latest the browser has published, and runtime resize
events apply in a single total order that the build is the zeroth element of (no replayed intermediate,
no lost update, no backward flash).**

---

## §0. Why this PR exists (what C1 left, and why a cell)

The shipped C1 (`66e81bcc`) delivers the viewport as a **spawn-time snapshot** (`viewport: Size` threaded
into `spawn_content_thread*` → `content_thread_main*` → `build_pipeline_*`). For a content thread that
**blocks on `load_document`** before building (the URL spawn `content_thread_main_url` and the navigation
rebuild `handle_navigate`), a window resize landing *during* that blocking load makes the snapshot stale:
the document builds (resolves cascade, runs initial scripts, lays out) at the pre-resize size.

C1's first attempt to close this (the Codex-R2–R5 **drain/replay** mechanism) was **reverted** (`480c02f7`)
as ad-hoc: it folded-latest + replayed a buffered message stream to reconcile the viewport *after* the
build, generating 8 findings across 5 review rounds — the anti-pattern CLAUDE.md *Concurrency by ownership*
and C1 plan-memo §2 both name. Post-revert, the resize-during-load edge is handled only by the natural
`run_event_loop` FIFO: the build is at the snapshot, one transient frame ships, then the queued
`SetViewport` corrects it. **Non-regression vs pre-C1, but still a reconcile-after-the-fact transient.**

The structural fix is to make the viewport a **latest-value cell** the content thread *reads* (a pull
source), so the build is at the latest **by construction** — never reconciled. The subtlety this memo
must get right (§2) is reconciling the cell-read build with the in-flight `SetViewport` message stream so
the cell does **not** introduce a *new* defect (a backward "flash" to a replayed intermediate, or a lost
update) in exchange for closing the old one.

The benefit is **not cosmetic** (plan-review F5): under the post-revert baseline (and pre-C1) the initial
scripts (`innerWidth`/`matchMedia`) **run** inside `build_pipeline` at the stale spawn-time size during a
resize-over-load and are **never** corrected — the script already ran, and the later resize event cannot
re-run it (C1 §2/D2). The cell makes those scripts run at the real size. The thing being fixed is
**script-observable value-correctness**, the exact invariant C1 §2 committed to, not merely a transient
frame.

---

## §1. Verified anchors (HEAD `480c02f7`)

Producer (browser main thread, the writer):
- `App.placement: Option<ContentAreaPlacement>` `app/mod.rs:259` — main-thread SoT, holds
  `size_logical`+`origin_logical`+`scale_factor`; rebuilt by `content_area_placement(&window)`
  `app/viewport.rs:34` on `resumed` `app/mod.rs:898-899` and `Resized` `:971`.
- `broadcast_viewport(&self)` `app/viewport.rs:74` → `seed_tab_viewport(placement, tab)` `:55` sends
  `BrowserToContent::SetViewport{width,height}` to every tab; called on `resumed` `:916` + `Resized` `:972`.
- Initial spawn deferred to `resumed`: `spawn_pending_initial_tab(placement.size_logical)`
  `app/mod.rs:907` → `app/viewport.rs:87`; `window.open` `app/mod.rs:776-784`, Ctrl+T `open_new_tab`
  `app/threaded.rs` pass `placement.size_logical` (fallback `DEFAULT`).
- **Only `size_logical` crosses IPC** (`app/viewport.rs:50` "never `scale_factor`").

Transport: `BrowserToContent::SetViewport{width: f32, height: f32}` (the variant the cell tags with a seq).

Consumer (content thread, the reader):
- `spawn_content_thread`/`_url`/`_blank` `content/mod.rs:380/405/421` and `content_thread_main`/`_url`
  `:441/471` take `viewport: elidex_plugin::Size` (the snapshot to replace).
- Build sites consuming the viewport: `content_thread_main` (HTML, **no** `load_document`) `:466`;
  `content_thread_main_url` (URL, **blocks** on `load_document` `:494` then builds `:519`);
  `handle_navigate` (nav rebuild, **blocks** on `load_document` `content/navigation.rs:123` then builds
  `:141`, viewport captured off the old pipeline at `:138` — the D7 capture this PR replaces with a cell read).
- `SetViewport` consumer (runtime resize event) `content/event_loop.rs:280-329`: value-idempotency guard
  (`unchanged` drop) `:290-292`, sets `pipeline.viewport`+`bridge.set_viewport`, re-evaluates MQL, fires
  `resize` then MQL `change` (HTML §8.1.7.3 order), `re_render`+`send_display_list`.
- `run_event_loop` `content/event_loop.rs:19` blocks on `channel.recv_timeout` (so a browser→content wake
  needs a *message*; there is no out-of-band content-side doorbell — PR-A `WakeHandle` is content→browser).

Spec consumer of the size: `run_scripts_and_finalize(viewport)` `pipeline.rs` — cascade (`@media (width)`)
+ `bridge.set_viewport` (`innerWidth`/`matchMedia`) + `layout_tree` + `PipelineResult.viewport`.

---

## §2. The ideal mechanism (first-principles)

Two facts must travel browser→content: the viewport **value** (a *pull* fact the build needs at an
unpredictable time — whenever the blocking load happens to finish) and the **resize event** (a *push*
fact already-running tabs need promptly). C1 conflated them into one snapshot+message. The clean shape
gives each its own transport, **correlated by a monotonic sequence** so the two never disagree:

1. **Latest-value cell (the pull source).** The browser owns a single
   `ViewportCell { size: Size, seq: u64 }` behind an `Arc` (one per window, **shared** into every content
   thread — all tabs share the window content area). On every placement change (`resumed`, `Resized`,
   C2's `ScaleFactorChanged`) the browser, as the **single writer**, bumps `seq` and stores the new
   `size_logical`. The cell is the IPC-published projection of `placement.size_logical` (the main-thread
   `placement` keeps `origin`/`scale_factor`, which never cross IPC). Each build site **reads** the cell
   *after* its blocking load returns (or immediately, for the HTML path) → the build is at the latest
   `(size, seq)` **by construction**, no drain, no snapshot staleness.

2. **Seq-tagged resize message (the push source).** `SetViewport` carries `{width, height, seq}`. The
   browser sends it to every tab on a placement change (the existing `broadcast_viewport` fan-out), and
   it stays FIFO-ordered with input on each tab's channel (so input-vs-resize hit-test ordering is
   preserved). It drives the runtime resize event (`resize`/MQL/re-layout) — unchanged from PR-B except
   for the seq field and the guard below.

3. **The seq reconciles build-consumption with message-consumption (the crux).** The build "jumps" to the
   latest cell value (seq `S0`); the queued `SetViewport` messages arrive one-at-a-time and include
   intermediates with seq `< S0`. Without reconciliation the cell would make the document **backward-flash**
   to a replayed intermediate (build at `B` seq2, then a queued `SetViewport(A, seq1)` fires because
   `A != B`). The fix: the content thread records `applied_viewport_seq = S0` at build, and the
   `SetViewport` arm runs **two independent guards** (plan-review F4): (i) the staleness guard — **drop any
   message with `seq <= applied_viewport_seq`** (already consumed — by the build or a prior apply); (ii) on
   a fresh `seq > applied`, **advance `applied_viewport_seq` unconditionally** (even if the size is
   unchanged), then let the existing value-`unchanged` guard decide whether the resize **event** fires.
   Seq bookkeeping (advance on `seq > applied`) and resize-event firing (only on size change, CSSOM View
   §13.1) are orthogonal — collapsing them would leave the high-water mark behind on a seq-newer/value-same
   message and mis-judge a later equal-seq as fresh. This is *consumption bookkeeping*, not state
   reconstruction: O(1) per message, no buffer, no replay.

Why seq and not value-idempotency (the current guard) or "read the cell in the message arm": value-only
fires the backward intermediate (`A != B`); "read the cell on every doorbell" makes a doorbell jump to a
*newer* size `C` before processing an input event that was hit-tested against the intermediate `B`
(mis-hit). The seq is the unique marker that makes build-consumption and ordered message-consumption agree
— monotone forward, no backward flash, no lost update (a genuinely newer resize always has `seq > applied`).

Net: build reads the cell (latest by construction); runtime applies the seq-ordered message stream with the
build as element zero. The reverted drain/replay edifice is replaced by **one `Arc<ViewportCell>` + one
`u64` field on the message + one `u64` field on `ContentState`**.

---

## §3. Spec coverage map

Same value-correctness surface as C1 (no new spec algorithm; this is a transport/ordering change).

| Spec section | Read context | Touch | Full enum? |
|---|---|---|---|
| CSSOM View §13.1 "run the resize steps" (`#document-run-the-resize-steps`) | resize fires only on size *change since last run* | seq guard + value-idempotency in the `SetViewport` arm | ✓ |
| CSSOM View §4 `innerWidth`/`innerHeight`/`matchMedia` | initial-script read at build | build reads the cell → `bridge.set_viewport` | ✓ |
| Media Queries L5 §4 width/height/aspect-ratio | initial cascade gate | `resolve_with_compat` at cell size | ✓ |
| HTML §8.1.7.3 update-the-rendering (step 8 resize before step 10 MQL `change`) | resize→MQL `change` order | consumer order unchanged from PR-B | ✓ |

- §-numbers to webref-verify at plan-review (per parent §11): CSSOM View `#document-run-the-resize-steps`,
  `#dom-window-innerwidth`, `#dom-window-matchmedia`; Media Queries L5 §4.
- **No untrusted web input**: the viewport+seq are OS/winit device facts + a browser-owned monotonic
  counter. No trust-boundary enumeration.

---

## §4. Edge matrix + dissolved findings

| Scenario | Post-revert (snapshot + FIFO) | This PR (cell + seq) |
|---|---|---|
| no resize during load (common) | build at snapshot = correct | build at cell = correct (identical) |
| resize lands during blocking load | build at **stale** snapshot → 1 transient frame → corrected by queued msg | build reads cell **after** load → latest by construction (**F5/F8/F15 dissolved**) |
| `[resize→A, resize→B]` during load | build at snapshot; A then B both fire (monotone) | build at B (seq2); queued A(seq1)/B(seq2) **dropped** (`seq ≤ 2`) — no backward flash |
| `[resize→A, click, resize→B]` during load | build at snapshot; A fires, click@A, B fires | build at B; A/B-intermediate ≤ build-seq dropped; click hit-tests @ build layout (see slot, F14) |
| resize after first frame (normal) | `SetViewport` applies | `SetViewport(seq>applied)` applies (unchanged) |
| resize to N tabs | `broadcast_viewport` → all | cell write once + seq-tagged broadcast → all (each tab tracks own `applied_viewport_seq`) |
| lost update (resize during the cell-read→loop-entry gap) | n/a (snapshot) | newer resize has `seq>build-seq` → its message applies (**no lost update**) |

**Dissolved**: F5/F8/F15 (build-at-stale → cell-read-at-build), F13 (frame-tick replay → no replay),
F9/F16 (replay-loop shutdown handling → no replay loop, already gone with the revert). **Out of scope
(slot)**: F10/F14 pre-first-frame **input** hit-testing — see §10; non-regression, needs input-seq.

---

## §5. Change surface (file-level)

**Cell type (new, shell crate):**
- `ViewportCell { size: Size, seq: u64 }` behind `Arc<Mutex<…>>` (writes are per-resize, reads per-build —
  no contention; a `Mutex` is the simplest correct shared cell, atomics are premature). Newtype with
  `publish(&self, size)` (browser: bump seq + store) and `read(&self) -> (Size, u64)` (content) so the
  single-writer/multi-reader contract is named, not open-coded. **Home: `ipc.rs`** (plan-review F3) — the
  existing shared leaf both `app` and `content` already import (`BrowserToContent` / `LocalChannel` live
  there), so the cell adds **no new dependency edge**. *Not* `app/viewport.rs`: it is winit/`App`-coupled,
  so a content-side import would create a content→`app` edge. (An earlier draft justified this by "content
  must not depend on `app`" — that rule is empirically **false** (`content/` already imports `crate::app::`
  in 4 files), so the home rests on the simpler true fact: `ipc.rs` is the leaf both sides already share.)

**Producer (`app/`):**
- `App` gains `viewport_cell: Arc<ViewportCell>` (constructed in `new_threaded*`/`from_tab_manager`).
- `content_area_placement` callers (`resumed` `:898`, `Resized` `:971`) call `self.viewport_cell.publish(p.size_logical)` right after caching `placement` (single new line each; the cell is the published
  projection of `placement.size_logical`).
- `spawn_content_thread*` / `spawn_pending_initial_tab` / `window.open` / `open_new_tab`: pass
  `Arc::clone(&self.viewport_cell)` **instead of** the `viewport: Size` snapshot. (`DEFAULT` fallback for
  window-less builds becomes a cell seeded with `DEFAULT` — D6 stays: window-less ⇒ explicit size.)
- `seed_tab_viewport`/`broadcast_viewport`: `SetViewport` gains the current `seq` (read from the cell, or
  carried alongside `placement`). Fan-out unchanged.
- **File-size note (plan-review F7)**: `app/mod.rs` is at 992 lines; these additions (the field + the two
  `publish` call-sites) cross 1000. The cell *type* lives in `ipc.rs` (F3), so the `app/` delta is only the
  field + two call-sites — keep it minimal and record the 1000-line crossing in the landing memo's existing
  "extract the viewport-producer cluster" follow-up (the C1 handoff already tracks it).

**Transport:** `BrowserToContent::SetViewport { width, height, seq: u64 }` (one field; update the 3-ish
constructors + the consumer destructure).

**Consumer (`content/`):**
- `spawn_content_thread*`/`content_thread_main*` take `viewport_cell: Arc<ViewportCell>` instead of
  `viewport: Size`; the build sites read `(size, seq) = cell.read()` immediately before
  `build_pipeline_*` (after `load_document` for the URL/nav paths; immediately, with no `load_document`,
  for the HTML path) and pass `size`.
- `ContentState` gains `viewport_cell: Arc<ViewportCell>` (for the nav-rebuild read) + `applied_viewport_seq: u64`.
- **`applied_viewport_seq` init — load-bearing, no zero-default window (plan-review F1).** For the **two
  thread-entry** build sites (`content_thread_main`, `content_thread_main_url`) it is a `ContentState::new`
  **parameter**, set to the `seq` of the build's `cell.read()`, **never** defaulted to `0`. (The nav
  rebuild reuses the live `ContentState` — it does not call `ContentState::new` — and instead *reassigns*
  the field; see the next bullet.) A `0` default would let a queued `SetViewport(seq < build-seq)` satisfy
  `seq > 0` and **apply a pre-build intermediate** — the exact backward-flash the seq guard exists to
  prevent (the *new* regression of §6 Alt-A). Order: the build reads the cell, then
  `ContentState::new(…, applied_viewport_seq = build_seq)`, so there is no window where the field is unset.
- `handle_navigate`: replace the D7 `let viewport = state.pipeline.viewport;` capture with
  `let (viewport, seq) = state.viewport_cell.read();` **after** `load_document` returns (race-free vs the
  reverted drain). **Reassign `state.applied_viewport_seq = seq` unconditionally in the `Ok(loaded)` arm,
  inside the existing per-pipeline reset cluster** (`navigation.rs:146-150`, beside `hover_chain.clear()` /
  `focusable_cache = None` / `viewport_scroll = default()`) — plan-review **F2**: *not* after
  `run_event_loop` re-entry, so every rebuild re-bases the high-water mark (else a post-nav `SetViewport`
  is judged against the pre-nav baseline → lost update or replay).
- `SetViewport` arm `event_loop.rs:280` — **two independent guards (plan-review F4)**: (i) staleness —
  `if seq <= state.applied_viewport_seq { return }`; (ii) advance the high-water mark
  `state.applied_viewport_seq = seq` **unconditionally** on a fresh `seq` (even if the *size* is unchanged,
  so a later equal-seq is correctly judged stale); then the existing value-`unchanged` early-return governs
  whether the resize **event** fires (`re_render` / MQL). Seq bookkeeping and event firing are orthogonal
  (CSSOM View §13.1 governs only the event half).

**Tests:** cell-read-at-build (resize between spawn and build ⇒ document at the new size); seq-drop (a
queued `SetViewport(seq ≤ build-seq)` fires no `resize`); seq-apply (`seq > applied` fires); no-lost-update
(resize after the build read still reaches the document); multi-tab (background tab tracks its own
`applied_viewport_seq`). The window-less spawn (`content_thread_wake_fires_on_display_list`) keeps passing
with a `DEFAULT`-seeded cell (no hang — the cell is a non-blocking read).

---

## §6. Alternatives considered

- **A — value-idempotency only (no seq), cell read at build.** Rejected: build jumps to `B`, a queued
  `SetViewport(A)` intermediate (`A != B`) fires a **backward flash** — a *new* regression the cell would
  introduce. The seq is what prevents it.
- **B — payload-less "doorbell" message; the arm reads the cell.** Rejected: a doorbell read jumps to a
  *newer* size `C` before an input event hit-tested against the intermediate `B` is processed → **mis-hit**.
  Reading the cell loses the per-message value needed for input-vs-resize ordering.
- **C — discard-drain at the build→loop boundary** (drop queued input + viewports, re-dispatch
  Navigate/Shutdown). Rejected: discarding a `SetViewport` that reflects a size *newer* than the build
  read is a **lost update**; making it safe requires comparing values/seqs — i.e. the seq anyway, plus a
  selective drain (the reverted-mechanism smell). The seq guard achieves the same with no drain.
- **D — keep the post-revert snapshot + FIFO (do nothing).** The honest baseline: non-regression and
  self-correcting for the *visible frame* (one transient frame). **Rejected** (plan-review F5 ruled this
  the ideal, not over-engineering) — not merely on the C1 §2 ideal, but because D leaves a real
  **script-observable** gap: initial `innerWidth`/`matchMedia` run inside `build_pipeline` at the stale
  spawn size during a resize-over-load and are **never** corrected (the script already ran — C1 §2/D2). The
  cell+seq is the minimum that fixes that: cell-*without*-seq (Alt-A) regresses with a backward flash, so
  if the cell ships, the seq ships. The protocol is three small `u64`-sized fields; the gap it closes is
  value-correctness, not a cosmetic transient.

---

## §7. Open forks for the 5-agent plan-review

- **Q1 — does the cell PR also tag *input* with a placement-seq to drop pre-first-frame input (F10/F14)?**
  Lean **no** (separate slot): F10/F14 are non-regression narrow edges (a click during the sub-second load
  window); folding input-seq in widens the protocol change to every input variant. But it is the *same*
  seq mechanism — the review should rule whether it is cheap enough to fold (one `seq` field on the input
  variants + a `< build-seq` drop) or belongs in `#11-…-loading-input-drop`.
- **Q2 — cell representation: `Arc<Mutex<ViewportCell>>` vs `Arc<AtomicU64×2 + AtomicU64 seq>`.** Lean
  Mutex (zero contention, clearest contract). Review: is there a lock-ordering or `!Send` concern with the
  content thread holding the Arc across `load_document`? (No lock is held across the load — `read()` is a
  lock-acquire-copy-release.)
- **Q3 — cell home — RESOLVED at this plan-review (F3): `ipc.rs`.** The existing shared leaf both `app` and
  `content` already import (`BrowserToContent` / `LocalChannel` live there) → zero new dependency edge. Not
  `app/viewport.rs` (winit/`App`-coupled → a content import would create a content→`app` edge). The
  "content must not depend on `app`" framing of earlier drafts is empirically false (`content/` already
  imports `crate::app::` in 4 files); the home rests on the simpler true fact that `ipc.rs` is the shared
  leaf, not on a layering prohibition.
- **Q4 — does `seq` belong on the cell *and* the message, or only the message with the cell carrying just
  `size`?** The build needs `seq` to set `applied_viewport_seq` (so the first queued message is correctly
  judged stale-or-fresh) → the cell must carry `seq`. Confirm no simpler factoring.

---

## §8. Collision / sequencing

- **Lands after the C1 revert (`480c02f7`) on `shell-viewport-pr-c`** (this branch) — either as the next
  commits on the same PR (if the review is quick) or a fresh follow-up PR once C1-core is judged shippable.
  Decide at plan-review exit.
- **C2/C3 orthogonal**: C2 adds `ScaleFactorChanged` as a third cell-publish + broadcast site; C3 adds
  device facts. The seq is shared (any placement change bumps it). C1's cell is the substrate C2/C3 extend.
- **S5 boa→VM**: engine-agnostic (cell holds `Size`, the consumer calls the existing
  `bridge.set_viewport`); no S5 coupling.
- **Parallel sessions**: `crates/shell/` — re-grep `app/mod.rs` `resumed`/`Resized`, `app/viewport.rs`,
  `content/mod.rs` spawn sigs, `event_loop.rs` `SetViewport` arm immediately before implementing. Worktree
  `elidex-wt-pr-c` is isolated.
- **Process-boundary (Phase) note**: content "threads" are same-process `std::thread`s over `LocalChannel`
  today, so the `Arc<ViewportCell>` shared pull-source is sound. A future renderer **process** split
  (design doc security-by-structure) would move the cell out of shared memory — but the **seq-tagged
  message already crosses a process boundary unchanged**, so the split degrades to "the seq-message is the
  sole viewport source + a block-read of the first message at build" (C1 §6 Alternative A, now seq-correct)
  or a shared-memory cell. The seq mechanism is the part that survives the boundary; the `Arc` cell is the
  same-process optimization. This is a local-swap boundary (the `ViewportCell::read()` seam), not a
  hard-coded assumption — flag, not block.

---

## §9. Testing / acceptance

- **cell-read-at-build**: drive a content thread whose cell is updated between spawn and the build read;
  assert the first `DisplayListReady` reflects the *updated* size (not the spawn value).
- **seq-drop (no backward flash)**: build at seq `S`; enqueue `SetViewport(other_size, seq ≤ S)`; assert
  **no** `resize` event / no relayout.
- **seq-apply**: enqueue `SetViewport(new_size, seq > S)`; assert `resize` + relayout.
- **no lost update**: write the cell to `C` (seq `S+1`) *after* the build read `B` (seq `S`) and send the
  `SetViewport(C, S+1)`; assert the document reaches `C`.
- **multi-tab**: two tabs at seq `S`; a resize to seq `S+1` broadcasts; assert each tab applies once and
  ends at the new size (independent `applied_viewport_seq`).
- **window-less no-hang**: `content_thread_wake_fires_on_display_list` passes with a `DEFAULT`-seeded cell.
- Each IMPORTANT from the review gets a regression test (Supported-surface: shell content-thread units).

---

## §10. Defer slots

- **`#11-viewport-latest-value-cell`** — **this PR** (register at impl; CLOSE on land).
- **`#11-viewport-loading-input-drop`** (NEW, conditional on Q1=no) — drop input hit-tested against a
  pre-build placement (F10/F14). Same seq mechanism extended to input variants. **Why deferred**:
  non-regression narrow edge; widening the protocol to every input variant is a distinct, larger change.
  **Re-evaluation trigger**: a reported mis-click during navigation, or Q1=yes at plan-review.
  **Re-evaluation date**: this plan-review (Q1 resolved → defer; F5 leaning).
- **`#11-window-level-tab-bar-position`** (existing, untouched) — the one-cell-for-all-tabs assumption
  (all tabs share the window content area) holds only while `tab_bar_position` is uniformly `Top`; same
  caveat C1 already carries.
- **C2/C3 cell extensions** (`scale_factor`, color-scheme, dppx) tracked under the parent umbrella, not here.
