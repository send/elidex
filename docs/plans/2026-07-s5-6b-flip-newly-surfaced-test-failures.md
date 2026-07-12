# S5-6b flip — 17 test failures surfaced by the shell-test-bridge migration

The `#11-shell-test-bridge-migration` slice (commits `d740a5ea` T1 + `71968b70` T2) made
`cargo build -p elidex-shell --all-features --tests` **GREEN** (was 57 compile errors) and the
suite now RUNS for the first time on the `s5-6b-flip` branch: **224 passed / 17 failed**.

**The 17 failures are pre-existing flip-wiring gaps, NOT caused by the migration** — verified
below (backtraces + per-test diff audit). They were invisible until now because the shell test
target did not compile on this branch. They are the flip's remaining **CI-green blockers**: `mise
run ci` (which compiles+runs all tests) will not pass until they are resolved, so they gate the
Stage 3 merge and the 2f-4 security gate's `/external-converge` (both CI-green-gated). They span
**5 distinct subsystems** — none is a test-migration concern; each belongs to a flip stage.

Verified against HEAD `71968b70` (2026-07-12). Migration-causation ruled out per-category by
git-diff audit of the touched hunks (`git diff 71968b70~1 71968b70`).

---

## ✅ Category A — RESOLVED (commit `9ff26165`, 2026-07-12): lifecycle/unload dispatch ran the VM UNBOUND (5 tests)

**Fix landed**: added `dispatch_event_bracketed` (pipeline.rs — the free-function analog of
`PipelineResult::dispatch_event`'s §4.1 batch-bind bracket) and routed all five dispatches
(`readystatechange`/`DOMContentLoaded`/`load` in `dispatch_lifecycle_events`,
`beforeunload`/`unload` in `dispatch_unload_events`) through it. Also rewrote
`domcontentloaded_fires_before_load` to capture firing order via DOM mutation (the VM-robust sibling
pattern) instead of a cross-eval `var`-global read (which surfaced as a secondary failure once the
panic was gone — the follow-up `eval_script` reading a top-level `var order` returned empty; whether
top-level `var` should persist as a global across `Vm::eval` calls is a separate VM-semantics
question, NOT chased here). Suite: **17 → 12 failures** (229 pass). The 12 below remain (B/C/D/E).

The original analysis is retained for the record:

## Category A — lifecycle/unload event dispatch runs the VM UNBOUND (5 tests) — SEVEREST

**Tests**: `tests::domcontentloaded_fires`, `domcontentloaded_fires_before_load`,
`lifecycle_events_not_cancelable`, `load_event_fires`,
`content::fragment_nav_tests::addressbar_cross_document_nav_fires_unload`.

**Symptom**: panic `HostData accessed while unbound` (`host_data/mod.rs:1299`).

**Root cause (backtrace-confirmed)**:
`build_pipeline_interactive → run_scripts_and_finalize → dispatch_lifecycle_events →
script_dispatch_event → call_listener → ensure_event_handler_current → HostData::dom → panic`.
`dispatch_lifecycle_events` (`pipeline.rs:385`) fires `readystatechange` / `DOMContentLoaded` /
`load` via bare `script_dispatch_event(runtime, &mut ev, &mut ScriptContext::new(...))` calls
**without** the `with_bound` batch-bind bracket that `PipelineResult::dispatch_event` (`lib.rs:259`)
and `flush_with_ce_reactions` (`pipeline.rs:110`) use. Under the VM, `ensure_event_handler_current`
(the event-handler-IDL-attribute reflection, hit by `call_listener` for any listener) reads
`HostData::dom`, which requires the VM bound. So a lifecycle listener panics.

**Migration-causation**: NONE. `domcontentloaded_fires` / `load_event_fires` were not touched by
the migration; the panic fires during BUILD, before any migrated assertion line.

**Severity**: HIGH — this is a **real production path** (`run_scripts_and_finalize` is shared by
`build_pipeline_from_loaded`), so any page with a `DOMContentLoaded`/`load` listener panics.
Undetected only because these tests never compiled on the branch.

**Fix locus (edge-dense — do NOT rush)**: wrap `dispatch_lifecycle_events`' dispatch sequence in
the batch-bind bracket, matching `PipelineResult::dispatch_event`. Subtlety: the interleaved
`flush_with_ce_reactions` opens its OWN `with_bound` (it takes `&mut dom`, which the bound `*mut dom`
aliasing contract forbids overlapping), and `transition_ready_state` also dispatches — so the
bracket structure must respect the same `&mut dom` non-overlap contract stage 2d / 2f-k handled
(the 2f-k SIGBUS was exactly this aliasing class). Belongs to the flip's event-dispatch-bind stage,
plausibly warranting the same care as 2d.

---

## Category B — MQL `change` / matchMedia not delivered through the content-thread pump (7 tests)

**Tests** (all `content::viewport_tests`): `content_thread_setviewport_flips_width_media_query`
(:83), `content_thread_first_frame_at_spawn_viewport` (:130),
`content_thread_resize_listener_sees_fresh_matchmedia` (:227),
`content_thread_same_size_setviewport_is_idempotent` (:340),
`content_thread_builds_at_latest_published_cell_size` (:399),
`content_thread_drops_stale_seq_viewport` (:515),
`atomic_size_and_facts_delivery_fires_no_intermediate_mql_change` (:1099).

**Symptom**: TWO sub-symptoms on the content-thread viewport/facts pump —
(a) **style re-cascade** on `SetViewport`: e.g. `content_thread_setviewport_flips_width_media_query`
(:83) sends `SetViewport{width:800}`, receives `DisplayListReady`, and asserts `has_red(&resized)`
(the 800px viewport should match `@media (max-width:900px)` → red div). It does NOT use JS MQL
listeners — it asserts the layout **re-cascaded** at the new viewport. Failing = the resized display
list did not reflect the new-viewport media match.
(b) **JS MQL `change`-firing count** / `matchMedia().matches` freshness: e.g. `:1099`
`left Some("0") right Some("1")` — "facts-only delivery flips the live query exactly once" but it
flipped 0 times.

**Root cause (to investigate)**: the content-thread `SetViewport`/`SetDeviceFacts` handler
(`event_loop.rs:398+`, the arms that call `set_media_environment`) does not, under the VM, (a)
re-evaluate media queries + re-cascade + rebuild the display list for the new viewport, and/or (b)
call `deliver_media_query_changes` to fire the live `MediaQueryList` `change` events (CSSOM-View
§4.2). This is a **deeper pump/re-render wiring gap, NOT another unbound-dispatch** (the failures are
assertion mismatches / stale display lists, not `HostData`-unbound panics). Needs a focused look at
the content-thread viewport arm + `re_render` + `set_media_environment` ordering — its own fix, not a
tail-end sweep.

**Migration-causation**: NONE. 6 of the 7 (`:83`–`:515`) were NOT touched by the migration (all
migration hunks in `viewport_tests.rs` are ≥ line 845). The 1 touched (`atomic`, migration added a
`matchMedia(...)` **read** at ~:1080) fails at `:1099` with "flipped 0 times" — a change that did
NOT fire, which a listener-less `matchMedia().matches` read cannot cause (it could only add flips,
never remove them); its 6 untouched siblings fail identically, confirming the pump-gap root cause.

**Fix locus**: the content-thread `SetViewport`/`SetDeviceFacts` handler + the runtime's
`deliver_media_query_changes` wiring. Flip viewport/facts-pump stage.

---

## Category C — CSS animations do not survive `re_render` (2 tests)

**Tests**: `tests::re_render_preserves_running_animations` (:670, `left 0 right 1`),
`re_render_does_not_duplicate_animations` (:698, `left 0 right 2`).

**Symptom**: running animation count drops to 0 across a no-op `re_render`.

**Migration-causation**: NONE — `:670`/`:698` untouched (no migration hunks there).

**Root cause (to investigate)**: `re_render` under the VM does not preserve the
`AnimationEngine` running set (a re-sync drops/resets animations). Flip re-render/animation stage.

---

## Category D — test-JS boa-ism `WebSocket()` without `new` (2 tests)

**Tests**: `content::iframe_security_tests::sandboxed_iframe_initial_script_observes_opaque_origin`
(:189), `unsandboxed_iframe_initial_script_observes_tuple_origin` (:219).

**Symptom**: assertion `observed.contains("network")` / `("mixed content")` fails — `observed` is
`"Failed to construct 'WebSocket': Please use the 'new' operator"`.

**Root cause**: the inline test JS calls `WebSocket("ws://…")` **without `new`** (the test comment
literally says *"boa registers WebSocket as a plain callable — invoke without new"*). The VM is
spec-correct (WebSocket requires `new`), so the call throws before reaching the mixed-content /
origin oracle these tests use as a side-channel to observe the iframe's initial-script origin.

**Migration-causation**: NONE — the failure is at the `.contains("network")` assert (`:189`),
BEFORE the migrated `.origin()` swap (`:193`/`:223`, never reached).

**Fix locus (uncertain — flag)**: minimally `WebSocket(...)` → `new WebSocket(...)` in the two
`srcdoc` snippets, BUT this only passes if the VM `WebSocket` constructor replicates the
mixed-content gate + reads the shell-installed (sandbox-opaque vs inherited-tuple) origin the way
the test's oracle expects (uncertain — the whole design is a boa-era origin side-channel). May need
re-authoring onto a direct origin oracle (e.g. `location.origin` / a dedicated read-back) rather
than the WebSocket-mixed-content proxy. Flip WebSocket / origin-wiring question.

---

## Category E — mouse-wheel scroll on `overflow:hidden` (1 test)

**Test**: `content::content_tests::content_thread_mouse_wheel_no_scroll_overflow_hidden` (:409,
`assertion failed: result.is_err()`).

**Migration-causation**: NONE — `:409` untouched.

**Root cause (to investigate)**: the wheel/scroll path no longer rejects a scroll on an
`overflow:hidden` container as expected. Flip scroll/input stage.

---

## Disposition

- The migration (T1 `d740a5ea` + T2 `71968b70`) is **mechanically complete** (build-green) and did
  its job: it surfaced these 17. They are **out of the test-migration's scope** (5 other subsystems)
  but **in scope for "get the flip CI-green before Stage 3"**.
- **✅ Category A fixed** (`9ff26165`, 17→12, 229 pass). Remaining **12** = B(7) + C(2) + D(2) + E(1).
- **Remaining priority order**: B (pump/re-cascade, 7 tests — biggest cluster, deeper subsystem) →
  C (animations) → E (wheel) → D (test-JS re-author + uncertain WebSocket wiring).
- B is a content-thread viewport/facts pump gap (re-cascade + MQL deliver), C/E are re-render /
  scroll subsystem behaviours, D is a test-JS boa-ism + an uncertain VM-WebSocket-origin wiring
  question. Each is a focused per-subsystem fix, NOT a bundled tail-end sweep. Recommend B next
  (read `event_loop.rs:398+` SetViewport/SetDeviceFacts arms + `re_render` + `set_media_environment`
  ordering).
