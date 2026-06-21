# PR-A — async repaint-wake (plan-memo)

Sub-slice of the **`#11-shell-viewport-delivery` umbrella**
(`docs/plans/2026-06-shell-viewport-delivery-plan.md`, plan-reviewed clean
2026-06-21). PR-A is the umbrella's dependency root (§7): a generic
content→browser repaint wake, **viewport-independent**, that the core slice (PR-B)
and every content-initiated frame (timers / rAF / animation / async DOM) depend on.
HEAD at re-grep: `c801ef6c` (#380), 2026-06-21.
Status: **plan-memo (pre-implementation)** → `/elidex-plan-review` → worktree → impl.

> Base-case note: PR-A is a narrow sub-slice under the approved, plan-reviewed
> umbrella. Its mechanism (winit `EventLoopProxy` → `user_event` → `request_redraw`)
> is the canonical winit pattern (no novel algorithm) and couples ~1 invariant axis
> (wake→redraw→drain→paint ordering) — below the mandatory-split threshold. It still
> gets its own plan-review per the umbrella commitment, because the `EventLoop<T>`
> type change ripples across both `run_app` entry points + the shared
> `ApplicationHandler` impl + all 4 content-thread spawn sites, and the wake is
> cross-cutting (every content message), so the blast radius warrants the gate.

---

## §0. Decisions this memo commits to

- **D0 — One canonical "content woke the browser → repaint" path.** Today the
  browser event loop is winit-default `ControlFlow::Wait`; a content-thread
  `ContentToBrowser::DisplayListReady` only updates `tab.display_list` and is
  *drained inside the redraw handler*, so a content-initiated frame paints only on
  the next OS-driven event. PR-A makes the content thread *wake* the loop on a
  display-affecting send. This is One-issue-one-way: a single wake mechanism for
  every frame the content thread **sends to the browser**, not a viewport-specific
  hack. (Scope precision: in-process iframes flow through the parent's
  `re_render_all_iframes` → `send_display_list` → wake (covered); an *out-of-process*
  iframe's own async frame is cached by the parent **without** re-emitting
  (`iframe/mod.rs:111-112`) — a separate pre-existing parent-frame-production gap,
  upstream of the wake, carved in §6.1 (F1).)

- **D1 — The wake is injected as a windowing-agnostic callback, not a winit type
  on the content thread.** The content thread (the CSS/renderer owner per
  *concurrency-by-ownership*) must not depend on the windowing system. The browser
  constructs a `WakeHandle = Arc<dyn Fn() + Send + Sync>` from its
  `EventLoopProxy<WakeEvent>` and injects it at content-thread spawn; the content
  thread calls `wake()` after a display-affecting send, knowing only "notify the
  host", not "winit proxy". (Resolves the Axis-1 layering concern of coupling
  content to winit; mirrors the design doc's `FrameSource`-boundary intent.)

- **D2 — Wake at the single content→browser send chokepoint, on display-affecting
  messages; rely on winit redraw coalescing for the rest.** The content side
  funnels every `ContentToBrowser` through `self.channel.send(...)`
  (`content/mod.rs`). PR-A routes the display-affecting sends
  (`DisplayListReady`, plus the chrome-affecting `TitleChanged` / `UrlChanged` /
  `NavigationState`) through one `ContentState::notify_browser(msg)` that does
  `channel.send` + `wake()`. winit coalesces multiple `request_redraw` within one
  frame, so an animation/timer burst is bounded to one repaint per frame — no
  busy-loop, `ControlFlow` stays `Wait`. (F5: the residual per-send cost is one
  `proxy.send_event` + one idempotent `request_redraw` flag-set — *not* an extra
  paint; bounded and acceptable, so no DL-diff gating is warranted.)

- **D3 — Threaded-mode only; inline mode is synchronous and needs no wake.** Inline
  (`InteractiveState`, test-only `build_pipeline` API) renders on the main thread
  with no IPC, so it has no async stall. The `EventLoop<WakeEvent>` *type* change is
  global (the `ApplicationHandler` impl is shared), but `user_event` is a no-op when
  `render_state`/`tab_manager` is absent.

- **D4 — `user_event` requests a redraw; it does not itself drain.** The wake's only
  job is to schedule a rendering opportunity. The existing drain
  (`drain_content_messages`, run at the top of `handle_redraw_threaded`,
  `threaded.rs:118`) then consumes the new DL before `with_frame` presents — so the
  **wake→redraw→drain→paint** ordering (umbrella §10 Q5 contract) holds with no
  drain-site change. (F6: the wake is **best-effort** — if it arrives before
  `resumed` creates the window (`render_state = None`), `user_event` no-ops, but
  `resumed` itself `request_redraw`s (`mod.rs:754`) and the channel is the SoT for
  the pending DL, so the startup frame still drains+paints on the first post-resume
  redraw; no frame lost.)

---

## §1. Verified anchors (re-grepped at HEAD `c801ef6c`, 2026-06-21)

- **Event loop**: `EventLoop::new()` (winit 0.30) at `lib.rs:339` (`run`) +
  `lib.rs:887` (`run_url`); both `event_loop.run_app(&mut app)`. No `ControlFlow` /
  `about_to_wait` / `Poll` / `EventLoopProxy` / `with_user_event` anywhere in
  `crates/shell/elidex-shell/src/` (0 grep hits) ⇒ default `ControlFlow::Wait`,
  no proxy.
- **Handler**: `impl ApplicationHandler for App` (`app/mod.rs:734`) = the unit
  (`()`) user-event handler; **no `user_event` method**. `resumed` (:735),
  `window_event` (`fn` opens :762), `suspended` (:758). (F7: line cites refreshed.)
- **Stall site**: `DisplayListReady` consumer `app/mod.rs:398-400` sets
  `tab.display_list = dl` only — no `request_redraw`. `drain_content_messages`
  (`mod.rs:379-400`) runs **inside** `handle_redraw_threaded` (`threaded.rs:118`),
  i.e. only when a redraw is already happening.
- **Content→browser sends** (the wake chokepoint): `ContentState` funnels through
  `self.channel.send(...)` — `send_display_list` (`content/mod.rs:67-71`,
  `DisplayListReady`), `send_navigation_state` (`content/mod.rs:74-78`), plus
  `TitleChanged` / `UrlChanged` sends. (Verified `self.channel.send` is the single
  sender field on `ContentState`.)
- **Content-thread spawn sites** (must receive the `WakeHandle`): `spawn_content_thread`
  (`content/mod.rs:341`), `spawn_content_thread_url` (:356), `spawn_content_thread_blank`
  (:370); **production** callers `App::new_threaded` (`mod.rs:282`), `new_threaded_url`
  (:304), `window.open` (:672), `open_new_tab` (`fn` :454, spawn call ~:466). **Plus
  11 `content_tests.rs` test callers** (:23/44/75/109/156/176/214/266/298/330/360)
  that the signature change breaks — mechanical update set (F2/F7).
- **Spawn timing**: `EventLoop::new()` (`lib.rs:339`) precedes `App::new_threaded`
  (`lib.rs:340`), so the proxy can be created from the event loop and passed into
  the App ctor → the **initial** content thread gets the `WakeHandle` at spawn (no
  late injection needed). `window.open`/`open_new_tab` tabs are spawned after
  `resumed`, when the App already holds the proxy.
- **winit 0.30**: `EventLoop::<T>::with_user_event().build()?` + `create_proxy()`;
  `EventLoopProxy<T>: Send + Clone`; `ApplicationHandler<T>::user_event(&mut self,
  &ActiveEventLoop, T)`.

---

## §2. Mechanism — the wake seam

```
content thread                         browser (winit) thread
──────────────                         ──────────────────────
notify_browser(DisplayListReady(dl)):
   channel.send(dl)            ──────▶ (channel: drained later)
   wake()                      ──────▶ EventLoopProxy::send_event(WakeEvent::Repaint)
                                          └▶ ApplicationHandler::user_event
                                               └▶ render_state.window.request_redraw()
                                                    └▶ WindowEvent::RedrawRequested
                                                         └▶ handle_redraw_threaded:
                                                              drain_content_messages()  // consumes dl  (threaded.rs:118)
                                                              with_frame(... present)   // paints it
```

- `WakeHandle = Arc<dyn Fn() + Send + Sync>`. The browser builds it once:
  `let proxy = event_loop.create_proxy(); let wake = Arc::new(move || { let _ =
  proxy.send_event(WakeEvent::Repaint); });` and clones it into each spawned content
  thread (D1).
- `enum WakeEvent { Repaint }` — a typed user event (extensible later to
  per-tab/targeted wakes; a bare `()` would also work for PR-A but the enum keeps
  the door open without churn).
- `ContentState` gains a `wake: WakeHandle` field + `notify_browser(&self, msg)`
  that does `self.channel.send(msg)` then `(self.wake)()`. The wake-set is defined by
  **`ContentToBrowser` variant** (the read contract), not by which helper happened
  to be called (F3, One-issue-one-way): `DisplayListReady` (via `send_display_list`),
  `NavigationState` (`send_navigation_state`), `UrlChanged`, and `TitleChanged` —
  the last currently emitted by *raw* `channel.send` at two non-method sites
  (`content/mod.rs:90` in `notify_navigation`, `content/navigation.rs:282`), so PR-A
  introduces a `send_title` method (or routes both) to make the chokepoint real.
  Pure control acks that never change displayed state may keep the bare
  `channel.send` (coalescing makes any over-wake harmless).
- `user_event` (new on `impl ApplicationHandler<WakeEvent> for App`): `if let
  Some(s) = &self.render_state { s.window.request_redraw(); }` (D3/D4).

Why a callback, not the proxy directly on the content thread (D1): keeps
`crates/shell/elidex-shell/src/content/` free of `winit` types — the content thread
sees a `Fn()`. The winit coupling stays in the browser half (`app/`, `lib.rs`).

---

## §3. Spec coverage map

Per `feedback_plan-scope-re-evaluation`. PR-A is shell-infra (a rendering-opportunity
scheduler); its only spec obligation is that a produced frame actually reaches a
rendering opportunity. webref-verified 2026-06-21 (§8).

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.7.3 Processing model | rendering opportunity → step 22 update the rendering or user interface (`#update-the-rendering`) | a content-produced frame must reach a rendering opportunity, not stall until the next OS input event | wake → `request_redraw` → `drain_content_messages` → `with_frame` present | ✓ | no |

### §3.1 User-input touch audit
`User-input flow = no`: the wake is a content-thread→browser internal signal. A
page *can* influence wake *frequency* (e.g. `setTimeout` producing frames), but
(a) it is not parsed/trusted input — no trust-boundary enumeration
(`feedback_trust-boundary-enumerate-upfront` N/A), and (b) winit `request_redraw`
coalescing bounds frequency to one repaint per frame (D2), so a hostile-frequency
page cannot busy-loop the browser. `ControlFlow` stays `Wait` (wakes only on a real
message).

### §3.2 Breadth
K=1 spec, M=1 entry → single-PR scope (the breadth driver for the *umbrella* was
the ≥5-axis coordinate work in PR-B/PR-C; PR-A is the narrow infra root).

---

## §4. File-level change plan

- `lib.rs` — `run` / `run_url`: `EventLoop::new()` → `EventLoop::<WakeEvent>::
  with_user_event().build()?`; `let proxy = event_loop.create_proxy()`; pass a
  `WakeHandle` built from `proxy` into `App::new_threaded` / `new_threaded_url`.
  Define `enum WakeEvent { Repaint }` (+ `WakeHandle` type alias). F4: `WakeEvent`
  stays browser-internal (`lib.rs`/`app/`); `content/` references only the
  windowing-agnostic `WakeHandle` alias, never `WakeEvent`.
- `app/mod.rs` — `App` holds the `WakeHandle` (to clone into `window.open` tabs);
  `new_threaded`/`new_threaded_url` take it + thread it to the initial
  `spawn_content_thread*`; `window.open` spawn (:672) clones it.
  `impl ApplicationHandler for App` → `impl ApplicationHandler<WakeEvent> for App`
  + `fn user_event(&mut self, _el, _e: WakeEvent) { request_redraw if render_state }`.
- `app/threaded.rs` — `open_new_tab` (`fn` :454, spawn ~:466) clones the
  `WakeHandle` into its spawn.
- `content/mod.rs` — `ContentState` gains `wake: WakeHandle`; add
  `notify_browser(&self, msg)` (send + wake); route the display/chrome-affecting
  variants through it via `send_display_list` / `send_navigation_state` /
  `send_url_changed` / new `send_title` (F3). `spawn_content_thread*`
  (:341/:356/:370) take `WakeHandle` and store it on `ContentState`.
- `content/iframe/*` — iframe sub-threads send `IframeToBrowser` to the *parent
  content thread* (`thread.rs:108/214`), not to the browser, so they need no wake
  injection. **In-process** iframes: the parent's `re_render_all_iframes` runs in
  the parent tick → `needs_render` → `send_display_list` → wake (covered).
  **Out-of-process** iframes (VERIFIED `iframe/mod.rs:111-112`):
  `drain_oop_messages` caches `oop.display_list = dl` **without** setting
  `needs_render` (`event_loop.rs:80`), so the parent does NOT re-emit on an OOP
  child's own async frame → no wake. This is a **pre-existing parent-frame-
  production gap** (upstream of and orthogonal to PR-A's wake), entangled with
  partial OOP-iframe support (M4-13) → carved to slot `#11-oop-iframe-parent-
  rerender-on-child-frame` (§6.1, F1). PR-A's wake covers every frame the content
  thread *sends*; it does not change what the parent decides to send.
- tests: `content_tests.rs` — (a) a new test: a content thread that emits an
  unsolicited `DisplayListReady` invokes the injected wake (assert via a test
  `WakeHandle` counter closure); (b) **mechanical signature update** — the
  `WakeHandle` param on `spawn_content_thread*` breaks all **11 existing
  `spawn_content_thread` callers** (:23/44/75/109/156/176/214/266/298/330/360),
  each passes a no-op/counter `WakeHandle` (F2).

---

## §5. Testing / acceptance criteria

- **Wake fires on unsolicited frame**: spawn a content thread with a test
  `WakeHandle` (an `Arc<AtomicUsize>` counter closure); drive it to emit a
  `DisplayListReady` with no preceding `BrowserToContent` → assert the wake counter
  incremented. (Unit-level; the winit half is event-loop-driven like the other
  shell window producers — no winit harness, the contract tested is "a
  display-affecting send calls `wake()`".)
- **No busy-loop**: assert the wake is called once per display-affecting send (not
  in a spin); `ControlFlow` unchanged (`Wait`).
- **Coalescing**: N rapid `DisplayListReady` sends → N wakes, but winit collapses to
  ≤1 repaint/frame (documented; not unit-asserted since it is winit behavior).
- **Layering**: `crates/shell/elidex-shell/src/content/` has no new `winit::`
  import **and no `WakeEvent` reference** (grep-guard, F4: the content thread depends
  only on the windowing-agnostic `WakeHandle = Fn()`, never the winit user-event
  payload type).
- `mise run ci` green (the `EventLoop<WakeEvent>` change is type-checked across both
  `run_app` sites).

---

## §6.1 Proposed defer slots (register at PR-A landing) — F1 + diff-review

| Slot | Why deferred | Re-evaluation trigger | Date |
|---|---|---|---|
| `#11-oop-iframe-parent-rerender-on-child-frame` | An out-of-process iframe's own async frame (`IframeToBrowser::DisplayListReady`) is cached by the parent (`iframe/mod.rs:111-112`, `drain_oop_messages` at `event_loop.rs:80`) **without** setting `needs_render`, so the parent does not re-render/re-emit → the OOP child frame neither composites nor wakes. This is a **parent-frame-production** gap *upstream of* PR-A's wake (PR-A correctly wakes on every frame the content thread *sends*; here the parent never sends one), and it is entangled with partial OOP-iframe support (M4-13 — cf. `#11-oop-iframe-focus-lifecycle` / `#11-cross-frame-visibility-propagation`). | OOP-iframe rendering productionization (M4-13), OR a test demanding an OOP-iframe-initiated animation frame to appear/wake. | 2026-06-21 (re-eval at PR-A landing / M4-13 OOP-iframe work) |
| `#11-content-message-coordination-wake` | **(diff-review, Agent 2 IMP boundary)** Non-rendering `ContentToBrowser` *coordination* messages — `StorageChanged` / `IdbVersionChangeRequest` / `SwRegister` / `IdbConnectionsClosed` / `ManifestDiscovered` — keep the bare `channel.send` (no wake): they change no on-screen state. Under `ControlFlow::Wait` they are processed on the next *drain* (next OS event / wake), not immediately. This is a delivery-*latency* concern on a different axis from repaint (cross-tab storage relay timeliness, IDB upgrade latency), **pre-existing** (nothing wakes today). PR-A wakes the rendering/chrome/window-action variants (incl. `OpenNewTab` / `FocusWindow`); coordination-message timeliness is out of its repaint scope. | A test/report showing a coordination message (storage event to another tab, IDB versionchange) is observably late because the browser was idle, OR the event loop moves off `ControlFlow::Wait`. | 2026-06-21 (re-eval at PR-A landing) |

## §6. Collision / sequencing

- **Unblocks PR-B**: PR-B's SetViewport round-trip settles immediately because the
  corrected frame wakes the loop (umbrella §7/§8). PR-A lands first.
- **No engine / VM / S5 coupling**: pure shell event-loop plumbing.
- **Terminal-Z / other shell work**: touches `lib.rs` `run*` + `app/mod.rs`
  `ApplicationHandler` + `content/mod.rs` sends — low overlap; the `EventLoop<T>`
  generic change is mechanical. Flag if a concurrent branch also edits the
  `run_app` signature.
- **Worktree**: `git worktree add -b shell-repaint-wake <dir> origin/main` (clean
  base). Verify cwd before commit (`feedback_worktree-cwd-drift`).

---

## §7. Open questions for `/elidex-plan-review`

- **Q1** — `WakeEvent` enum vs bare `()` user event: is the typed enum (future
  per-tab/targeted wake) worth it now, or is `EventLoop::<()>` simpler for PR-A?
  (Memo: typed enum, no churn cost, keeps targeted-wake door open.)
- **Q2** — Wake set: route *only* `DisplayListReady` through `notify_browser`, or
  also `TitleChanged`/`UrlChanged`/`NavigationState` (chrome freshness)? (Memo:
  all display/chrome-affecting; coalescing makes the extra wakes free. Pure control
  acks excluded.)
- **Q3** — `WakeHandle = Arc<dyn Fn()+Send+Sync>` (D1) vs giving the content thread
  the `EventLoopProxy<WakeEvent>` directly. (Memo: callback keeps `content/`
  winit-free = Layering mandate; the proxy-direct path couples the renderer owner to
  the windowing system.)
- **Q4 — RESOLVED (plan-review F1)**: VERIFIED — the parent does NOT re-emit on an
  *out-of-process* iframe frame (`drain_oop_messages` caches without `needs_render`,
  `iframe/mod.rs:111-112`); in-process iframes are covered via `re_render_all_iframes`.
  The OOP case is a pre-existing parent-frame-production gap upstream of the wake →
  carved to `#11-oop-iframe-parent-rerender-on-child-frame` (§6.1). PR-A's wake
  covers every frame the content thread *sends*; not in PR-A scope to change what
  the parent sends.
- **Q5** — Does the `EventLoop<()> → EventLoop<WakeEvent>` change affect the
  `accesskit_winit` / `egui-winit` integration (they wrap winit events)? Confirm no
  user-event-type clash at impl.

---

## §8. Citation appendix (webref-verified 2026-06-21)

- WHATWG HTML — §8.1.7.3 Processing model: the *rendering opportunity* defined term
  (`#rendering-opportunity`) + the "update the rendering" algorithm
  (`#update-the-rendering`) step 22 *update the rendering or user interface of doc
  and its node navigable to reflect the current state*. PR-A ensures a
  content-produced frame reaches a rendering opportunity rather than stalling under
  `ControlFlow::Wait`. (The spec does not mandate a model for *selecting* rendering
  opportunities — winit `ControlFlow`/`request_redraw`/`EventLoopProxy` is the
  mechanism, §8.1.7.3 the contract.)

---

## §9. As-built notes (implementation — branch `shell-repaint-wake`)

- **`WakeHandle = Box<dyn Fn() + Send>`** (lib.rs), **not** `Arc<…+Sync>` as the
  plan body sketched. Each content thread *owns* its boxed wake closure (built
  from a cloned `EventLoopProxy<WakeEvent>` per spawn), so `Send` suffices and no
  `Sync` bound on `EventLoopProxy` is required — strictly simpler than a shared
  `Arc`, same windowing-agnostic seam (D1/F4 preserved: `content/` sees only the
  boxed `Fn`).
- **Wake minting helpers** (`app/mod.rs`, `impl App`): `wake_from_proxy(&proxy)`
  (the single mint) + `wake_or_noop(Option<&proxy>)` (mint-or-no-op, taken by
  `Option<&_>` not `&self` so the `window.open` / `open_new_tab` spawn sites can
  call it while holding the disjoint `&mut self.tab_manager` borrow). `new_threaded*`
  call `wake_from_proxy(&wake_proxy)` directly (they own the proxy). Inline
  (`new_interactive*`) sets `wake_proxy: None`.
- **Content side** (`content/mod.rs`): `ContentState.wake: crate::WakeHandle` +
  `notify_browser(msg)` chokepoint (`channel.send` + `wake()`); `send_display_list`
  / `send_navigation_state` / `send_url_changed` / new `send_title` route through
  it (wake-set by `ContentToBrowser` variant, F3). `content/navigation.rs`
  `apply_state_change`'s raw `TitleChanged` send → `send_title`.
- **Browser side** (`app/mod.rs`): `App.wake_proxy: Option<EventLoopProxy<WakeEvent>>`
  + `impl ApplicationHandler<crate::WakeEvent>` with `user_event` →
  `render_state.window.request_redraw()` (best-effort; `resumed` covers a
  pre-window wake). `lib.rs` `run`/`run_url`:
  `EventLoop::<WakeEvent>::with_user_event().build()?` + `create_proxy()` →
  `App::new_threaded*(.., proxy)`.
- **F1 carve confirmed in code**: OOP-iframe frames cache `oop.display_list`
  (`iframe/mod.rs:111`) which is never read by the compositor (reads only the
  in-process-written `IframeDisplayList` component), so OOP compositing is unwired
  → nothing to wake into; the wake is complete for what the content thread sends.
  Slot `#11-oop-iframe-parent-rerender-on-child-frame` (§6.1) stands.
- **Tests** (`content_tests.rs`): new `content_thread_wake_fires_on_display_list`
  (counting `WakeHandle` asserts the initial `DisplayListReady` send invoked the
  wake) + `content_module_is_winit_free` (D1/F4 grep-guard over `src/content/`) +
  the 11 existing `spawn_content_thread` callers redirected through a no-op
  `spawn_test_content` helper. `ContentState::new` direct caller
  (`build_iframe_test_state`) also threads a no-op wake.
- **Verification**: `cargo check -p elidex-shell --all-features --all-targets`
  clean (0 warnings); `cargo test -p elidex-shell --all-features` = 120 passed,
  0 failed; `mise run ci` green.

### Diff-review fixes (5-agent `/elidex-review`, 0 CRIT / 1 IMP / 3 MIN → all applied)
- **IMP (Agent 2 — wake-set incompleteness)**: `OpenNewTab` + `FocusWindow` are
  user-visible chrome/window actions reachable from a *pure-async* callback (a
  `setTimeout` doing only `window.open`/`window.focus`, no DOM change) — they
  bypassed `notify_browser` and would stall under `Wait`. Routed all 5 send sites
  (`event_loop.rs` ×2, `navigation.rs` ×2, `event_handlers.rs` ×1) through
  `notify_browser` for a uniform "rendering/chrome/window-action variants always
  wake" invariant (the per-variant enumeration that *missed* them — and that I
  myself missed `IdbVersionChangeRequest` in a grep — is the One-issue-one-way
  smell). Non-rendering coordination messages stay bare, carved to
  `#11-content-message-coordination-wake` (§6.1).
- **MIN (Agent 2)**: stale `[App::make_wake]` intra-doc ref (method renamed to
  `wake_or_noop`) → fixed. (Not a doc-CI break — it was on a *private* field,
  which rustdoc does not document; `mise run ci` doc passed.)
- **MIN (Agent 4)**: `wake()` doc "schedules a rendering opportunity" → "schedules
  a redraw so the frame reaches a rendering opportunity" (the spec leaves
  *selecting* opportunities to the UA).
- **MIN (Agent 5)**: register `#11-oop-iframe-parent-rerender-on-child-frame` +
  `#11-content-message-coordination-wake` in `project_open-defer-slots.md` at
  landing (done).
- **Clean (FP)**: Layering (content/ winit-free, guard adequate), `Box<dyn Fn()+Send>`
  shape (not a component — per-thread egress, correct), OOP carve (verified: OOP
  `display_list` written but never read → compositing unwired → nothing to wake),
  inline no-op wake, the wake test (real, would-fail-if-broken assertion), spec
  citations (§8.1.7.3 / rendering-opportunity verbatim-correct).
