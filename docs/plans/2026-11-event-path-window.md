# `#11-event-path-window-missing` — the event path reaches the Window: one DOM *get the parent* (per-PR plan memo)

Per-PR plan for the slot `#11-event-path-window-missing` (ledger row registered 2026-11-05, PM 377; direction PM 378:
one DOM *get the parent* function whose Document arm returns the Document's relevant global object except for `load`,
not a Window special case beside the walk). Owner L3, the unit after RC-2a (#596 `17b2a84d`). Written against
`origin/main` = `47b6c934` (#606); every coordinate below is from that commit (H-0 is the provenance: the cited files
each re-pin changed — 11 of #603's 21 at Rev 3.2, two of #604–#606's ten at Rev 3.3 — with the re-runnable command).
**PM decision 132 is kept** (Layout T3 D-T3: the active document and its Window are browsing-context state, marshalled
per call, not a component; their component form is B1's, `#11-browsing-context-state-ecs-components`). This memo adds
no component and no store.

**Defect.** `build_propagation_path` (`crates/script/elidex-script-session/src/event_dispatch/mod.rs:203`) walks
`EcsDom::get_parent` to the tree root, and the Window entity has no `TreeRelation`
(`crates/core/elidex-ecs/src/dom/mod.rs:491`), so the Window is never on a node-targeted event's path. Runtime
probe (Layout, `f9cf8c40`, VM `dispatchEvent`): capture and bubble `click` listeners on window, document and body fire
`d-cap b-cap b-bub d-bub` (DOM §2.9 order: `w-cap d-cap b-cap b-bub d-bub w-bub`); `composedPath()` during dispatch
is `[body, html, document]`. Every `window.addEventListener` for a node-targeted event never fires — `click`,
`keydown`, `submit`, `input`, `DOMContentLoaded`, and so on.

**Scope.** One PR, the path only: `get_the_parent` (D-1) reading the one existing browsing-context fact through a
value argument built by one constructor (D-2), the path built once per dispatch (D-3), the two test-only production
functions and the unread `dispatch_flag` deleted (D-6: `composed_path_for_js`, the callback-form loop
`dispatch_event`, `DispatchEvent::dispatch_flag`). Not in scope:
handler-target determination (T3's, D-8), any Document↔Window component or store (PM 132), the *legacy target
override flag*, closed-tree hiding in `composedPath()` (P2's, D11), `relatedTarget` / touch retargeting, slot
assignment on the path (P2), the at-target two-pass order — §7.

**Changelog.**
- *Rev 3.3* (2026-11-07): the Rev 3.2 attestation MINs (M-1..M-6) and the re-pin onto `47b6c934` (#606). **M-1**:
  the Rev 3.2 provenance ("eight cited files"; "every other cited file is byte-identical to `4728972c`") was false —
  #603's 21 files (verified 2026-11-07, H-0) contain **11** this memo cites; the three missed are
  `content/event_handlers.rs` (H-22's `:878/:941`, which were `:893/:956` at `4728972c`), `content_focus_fixup_tests.rs` and
  `content_change_on_blur_tests.rs` (§5.1); the coordinates were right. The provenance is now §0.2 **H-0** (NEW),
  with the re-runnable command and both pin pairs, re-run at DoD item 1. **M-2**: DoD item 4's comment category —
  the filter's stated example was false (the two comment mentions of `build_propagation_path`, `focus/sot.rs:77` /
  `app/events.rs:195`, have no `(` and are outside the `build_propagation_path(` grep; the filter drops **0** lines
  at the base, measured); "first three greps count them" → **four** (the `dispatch_event(` grep counts the doc
  `mod.rs:57`); the `ActiveDocument {` grep and every grep after the first four take the filter (a doc quoting
  `ActiveDocument { document, window }` is dropped, not a construction). **M-3**: D-2 / §8 / the Rev 3.2 changelog
  said a T3 deviation is "caught by DoD item 1's re-measure", but DoD item 1 re-runs §0.2 and §0.2 had neither grep
  — **H-23** (NEW) holds both greps with the expected result per landing order (at `47b6c934` both → 0: this PR lands
  first) and the escalation ("T3 landed without routing `Vm::bind` through `HostData::active_document()` → stop,
  raise to the PM before implementation"); D-2, §8, DoD items 1 and 4 point at H-23. **M-4**:
  `#11-visibilitychange-bubbles-flag` **exists** (ledger, registered PM 455; Owner L3; Re-eval 2026-12-15; coordinate
  `message_dispatch.rs:317`, ruled correct PM 466) — §7's row says so (owner / re-eval the row's; trigger and
  acceptance are additions), DoD item 10 counts three proposals and five existing rows. **M-5**: H-21 / D-6 (iii):
  `respondWith(r)` is Service Workers **§4.6.7** and its step 2 reads the dispatch flag **directly** ("If event's
  dispatch flag is unset, throw an "InvalidStateError" DOMException"), not through §4.4 *active*; *active* is §4.4
  `ExtendableEvent`'s dfn, read by *add lifetime promise* step 2 = §4.4.1 `event.waitUntil(f)` — every Service
  Workers citation in H-21 / D-6 (iii) re-looked up (webref `heading` / `dfn` / `body service-workers`, 2026-11-07) and
  the §4.3 row for the deletion's doc gains them. **M-6**: re-pin — `origin/main` moved by #604 (`228764b6`, the CV
  umbrella erratum), #605 (`b86671a5`), #606 (`47b6c934`); `git diff --name-only 1580af13 47b6c934 -- crates docs`
  → ten files, two cited (H-0): `vm/tests/mod.rs` (+1 `mod` line at `:151`, #605; `:34` / `:52` unchanged; 205 →
  206) and `content-visibility.md` (878 → 1003 lines: D11 `:204-222` → `:229-247`, `:473` → `:544`, `:627-628` →
  `:709-712`, `:660` → `:750-752`, `:762` → `:868`; D-1, §3, §7, §8, DoD item 4). #604 re-grounded CV-details on
  main (`:50`; `:751-752` mark the `composed_path_for_js` premise "(re-ground)" on this PR's replacement path
  builder; P2-R3 `:710-712` names this slot as P2's hard order edge) — Rev 3's F5 hand-off is **done**; §8 and DoD
  item 10 say so, and DoD item 4's docs grep expects the three CV lines. H-14 re-run at `47b6c934`: 82 branches →
  27 with commits, the one addition `origin/vm-regexp-lastindex-overrun` (#605, landed); every other classification
  unchanged. H-4 / H-6 / H-16 / H-18 / H-21 / H-1 re-run (unchanged); the §5.1 complement over #604–#606 → 0 lines.
- *Rev 3.2* (2026-11-06): the Rev 3.1 attestation (seven findings N-1..N-7; the PM 455 rulings in the PM handoff
  `evpath-r31-findings.md`). **Re-pin**: `origin/main` moved to `1580af13` (#603, focus-fixup PR 2); the branch is
  rebased. `git diff --stat 4728972c 1580af13 -- crates docs/architecture` → 21 files (verified 2026-11-06), of
  which this memo cites eight [**11** — Rev 3.3 M-1, H-0]:
  shell `lib.rs` (`PipelineResult::dispatch_event` `:293-302` → `:291-300`, the bracket `:299-301` → `:297-299`,
  the `script_dispatch_event` call `:300` → `:298`, `ScriptContext::new` `:294` → `:292`, `teardown_document` `:468` →
  `:466`; 730 → 719 lines), `pipeline.rs` (every cited line is at or before `:533` and unchanged; 975 → 971),
  `content/mod.rs` (`:738/749/753-772` unchanged; the `#[path]` convention `:790-829` → `:787-832`, S-1..S-3's pair
  goes after `:832`; 865 → 869), `content/focus.rs` (`change` `:182` → `:195`), `content_test_support.rs`
  (`:377/405` unchanged), dom-api `focus/sot.rs` (`:78` → `:77`), and `content/iframe/mod.rs` / `content/event_loop.rs`
  (newly cited, H-22). Every other cited file is outside the 21 [false for three of them — Rev 3.3 M-1, H-0], so
  its `4728972c` measurement is its `1580af13` one.
  Re-run at `1580af13` (verified 2026-11-06): H-4 (50 lines / 17 files), H-6 (35), H-18 (17), H-20's six shell
  lines — unchanged. H-14:
  `layout-focus-fixup-canonical` is landed (#603). §5.1 gains the complement over #603's added test lines (no new
  member). **N-1** (IMP; PM 455): the cross-lane agreement — T3's memo routes its `Vm::bind` construction of
  `ActiveDocument` (T3 `:447-448`, `:538`, `:919`) through `HostData::active_document()` — is stated in D-2, §2 I-2
  and §8; Rev 3.1's "rewrite T3's helper body" fallback is withdrawn (this PR does not rewrite T3's landed code; a T3
  landing without the method is raised to the PM at DoD item 1 [the greps: H-23, Rev 3.3 M-3]); DoD item 4 gains
  the struct-literal grep
  (`ActiveDocument {` outside tests → exactly one, inside the constructor, in both orders) and DoD item 5's T3-first
  case is restated. **N-2** (IMP; PM 455): H-22 (b) had the OOP `message` direction reversed — the Document-targeted
  site is the parent → OOP-child *receive* (`iframe/thread.rs:185-196`, the `BrowserToIframe::PostMessage` arm
  `:82-84`, no production sender), while the iframe → parent `message` already fires at the parent's Window
  (`thread.rs:116` → `iframe/mod.rs:123` → `event_loop.rs:303-305` → `content/mod.rs:764`); restated in H-22 and §7,
  homed on the existing `#11-windowproxy-browsing-context` as an acceptance hand-off; the proposed slot is now
  `#11-visibilitychange-bubbles-flag` for (a) alone [registered PM 455 — Rev 3.3 M-4]; §0.3's
  "`message_dispatch.rs:320` is the iframe→parent `message`"
  (it is `visibilitychange` at the Document) is corrected with it. **N-3**: DoD item 10 lists §7's four proposals and
  four acceptance additions. **N-4**: DoD item 4's line numbers are the base's, re-measured at item 1, comment lines
  have a stated category (excluded by a filter, listed), and H-14 lists `lib.rs` as a touched file (the live
  `layout-focus-fixup-ungated` hunks are disjoint). **N-5**: DoD item 5's T3-first case allows the `engine.rs` doc
  line, as the this-first case does. **N-6**: H-21 / D-6 (iii): `event_target.rs:706` → `:707` (the `contains`
  check); the flag's spec readers are enumerated (DOM §2.7 `dispatchEvent` step 1; §2.2 `initEvent` step 1, not
  implemented, `events.rs:47`; Service Workers §4.4 *active*, read by *add lifetime promise* step 2 = `waitUntil`, and
  `respondWith` step 2 — `service_worker/event.rs:250/291`), every implemented one on `dispatched_events` (webref,
  2026-11-06). **N-7**: §4.1's header drops the engine attribution (the table has five rows); `[contrast]` kept.
- *Rev 3.1* (2026-11-06): `/elidex-plan-review` R3 (a focused re-check of the Rev 2→3 delta, two reviewers; the
  21 findings R3-1..R3-21 and the PM 441 rulings in the PM handoff `l3-evpath-plan-r3-dispositions.md`). Coordinates
  stay at `4728972c` (`origin/main` had not moved). **R3-1** (IMP; PM 441 ruling 1 — a concept sweep, not a
  one-line fix): §5.1 (NEW) enumerates, by meaning, every existing test asserting the path / `composedPath()` /
  the listeners reached along the path, with the commands and both counts — **changed 1** (`tests_dispatch_event.rs:327`
  `composed_path_returns_path_during_dispatch`, 4 → 5; now a touched row in §5, updated in commit 3, §4) /
  **checked-unchanged 193**; DoD item 6 accounts for the changed test (red in its old form at the head, red in its
  new form on the pre-fix source) and makes any other existing red a sweep miss to record, not a silent update. The
  sweep's surplus → H-22 (two Document-targeted shell UA dispatches with pre-existing flag / target deviations,
  `visibilitychange` and the OOP iframe `message`) and one proposed slot in §7. **R3-2 / R3-18 / R3-20** (IMP; PM
  441 ruling 3): DoD item 4's greps are each exact on a correct implementation — `composed_path_for_js` over
  `crates` → 0 and over `docs` with this memo excluded by pathspec → CV `:660` alone (Layout's, DoD 10); the
  `dispatch_event(` grep carries `| grep -v MutationEvent` (the ECS dispatcher `dom/mod.rs:235`, now named in H-16)
  and expects exactly `shell/lib.rs:293`. **R3-3 / R3-11 / R3-19** (IMP; PM 441 ruling 2): `HostData::active_document()`
  is mandatory in both landing orders and the only constructor; a T3 helper of any other shape delegates to it (the
  PM informs Layout; this memo does not edit T3's) — D-2, §2 I-2, §8 order-free paragraph, DoD items 4 (`fn
  active_document` → three in either order) and 5 (T3-first: no `window_entity` reader beyond the method) rewritten.
  **R3-4**: the VM walk reads `active` before its scoped `dom` borrow (`NativeContext::host` and `HostData::dom` are
  both `&mut self`) — snippet in §4 commit 3, §5 row. **R3-5**: DoD item 5 "six lines" (`:504` is a doc comment).
  **R3-6**: D-6 (ii) and §4 commit 1 list the module doc `mod.rs:3-5` and the shell comment
  `content_eh_attrs_bind_tests.rs:348-349` (now a touched row, comment only). **R3-7**: the migrated calls gain `None`
  as the fourth argument in commit 3 (D-9, §4, §5). **R3-8**: the three `tests.rs` fixture helpers move to
  `test_support.rs` as `pub(super)` (D-9, §5, §6.2). **R3-9**: U-8 is the non-composed shadow-internal case
  (`get_parent(sr) == host`, `shadow.rs:203`; the composed case does not separate the mutant). **R3-10** (PM 441
  ruling 5, decided on merit): `DispatchEvent::dispatch_flag` is **deleted** — H-21 (NEW): its only production reader
  was `composed_path_for_js`, and the spec's reader (DOM §2.7 `dispatchEvent` step 1) is the `dispatched_events` field of `Vm` on
  the JS `Event` object, where a per-dispatch Rust struct cannot be; D-6 (iii), §4 commit 1, §5 (`vm/test_helpers.rs`
  becomes a touched row; H-14 re-run for it: one disjoint C0b branch), DoD items 2 and 4 (`-w dispatch_flag` → 0);
  the test `dispatch_flag_set_during_dispatch` goes with the field, so **15** of H-16's 16 tests migrate (D-6, D-9,
  §4, §5, §10 Q-2). **R3-12**: step 6.9.7 is the "Otherwise" of 6.9.6 — §3 row and the §7 relatedTarget acceptance
  scoped to a parent outside the target's root. **R3-13**: D-3 cites PM 421, not the base case. **R3-14**: DoD item
  4's `get_the_parent` clause includes the `lib.rs` re-export. **R3-15**: the `unhandledrejection` disposition says
  node- / Document-targeted events, not global ones. **R3-16**: `vm/tests/mod.rs:31` → `:34` (Rev 3 changelog, H-14).
  **R3-17** (PM 441 ruling 4): §4.1's header keeps `[contrast]` and names the one engine the four symbols belong
  to. **R3-21** (FP): no action beyond R3-17. §0.1's scope sentence names the third deletion.
- *Rev 3* (2026-11-06): `/elidex-plan-review` R2 (five reviewers, 2026-11-06; dispositions in the PM handoff
  `l3-evpath-plan-r2-dispositions.md`). **F1** (Ax2 IMP; Ax2 MIN gating; Ax5 MIN marshalling surface): `window_entity`
  is a `HostDriver` item, not a `ScriptEngine` one, so Rev 2's session-side composition had no carrier → D-2 rewritten:
  one constructor `HostData::active_document()` (NEW, bound-gated), one `HostDriver::active_document()` trait item
  (NEW) delegating to it, and the two shell bracket sites read it inside the `with_bound` closure and pass it as a
  value argument through `script_dispatch_event` → `script_dispatch_event_core` → `build_dispatch_plan`; the
  `engine.window_entity()` + `ctx.document` composition is gone; both entries use one bound gate; every
  "`ScriptEngine` trait item" wording corrected to `HostDriver` (H-4, D-8, §0.3). **F2** (Ax3 IMP; Ax2 MIN mock):
  the callback-form `dispatch_event` is deleted with its `lib.rs` re-export; its 16 tests (not 17, §0.3) move onto
  `script_dispatch_event_core` through one test-only mock `ScriptEngine` (`event_dispatch/test_support.rs`, NEW, D-9),
  as commit 1, behaviour-preserving; U-6 and U-9 use the same mock; the loop count is two, and the collapse slot binds
  the runner to the session crate (§7). **F3** (Ax4 IMP): `retarget`'s docstring (`mod.rs:575`) cites DOM §2.5 =
  *Constructing events*; corrected to §4.8 *Interface ShadowRoot* (*retarget*) in this PR; a citation table for every
  new or rewritten doc (§4.3). **F4** (Ax5 IMP; Ax3 MIN): DoD item 5 is a command over a defined population with the
  new readers named and the T3-first case covered; DoD item 4's `fn active_document` count is conditional on the
  landing order. **F5** (Ax5 IMP): the CV-details edge (`content-visibility.md:660` rests on `composed_path_for_js`
  for a crate choice) recorded in §8 and handed to Layout in DoD item 10. MINs: §1 cites PM 421, not base case (c);
  every proposed slot has a re-eval date and the at-target row says why it is deferred; the pre-bind claim corrected
  (inside the brackets that population is empty, H-20; the shell fallbacks are unobservable); the single-world caveat
  (T3 H-T15) stated (H-5, D-2); §4.1 ECS-native mapping and §4.2 layering table added; Q-1 decided (T3's placement
  stays) and Q-2 resolved by F2, the changelog contradiction fixed; "has a browsing context" ≈ "is the bound document"
  stated as an approximation and measured (H-19, §3 row HTML §7.3.2); T-6 and U-5 use `composed: true`; the
  relatedTarget acceptance reworded per DOM §2.9 step 6.9.7 (the parent set to null is not appended); H-14 re-run over
  every file the PR touches or reads (the "none" claim was false: trivial overlaps listed); the B1 paragraph corrected
  (§5 req 5 is at `agent-scoped-ecsdom-world.md:447`; one `Vm` hosts several documents, so the argument is the
  interim single-browsing-context form and B1 replaces it); C1-pre cited at rev 2.4 `81d63d84`; S-1..S-3 in their own
  shell test file (`content/mod.rs` gains the `#[path]` pair); the `unhandledrejection` trigger has a recorded
  disposition (§7). Coordinates re-pinned at `4728972c` (`git diff --stat e8b6a24d 4728972c -- crates
  docs/architecture` → five files: #600's two `crates/dom/elidex-dom-api/src/element/table_mutation/*` files, #602's
  `compiler/resolve.rs`, `vm/tests/tests_closure_resolution.rs` and `vm/tests/mod.rs` (+1 `mod` line at `:34`); #601
  touches `.claude/tools`, `scripts`, CI and plan memos only. One cited line moved: `vm/tests/mod.rs`
  `tests_dispatch_event` `:51` → `:52`).
- *Rev 2* (2026-11-06): structural rewrite after `/elidex-plan-review` R1 (five reviewers, 2026-11-05; dispositions in
  the PM handoff `l3-evpath-plan-r1-dispositions.md`). Ax5 CRIT (D-2 `AssociatedWindow` reversed PM 132) → D-2 dropped,
  no component, no store. Ax5 CRIT (D-3 entered T3's scope) and Ax4 CRIT (D-3 kept the handler on the element where
  HTML §8.1.8.1 step 3 returns null) → D-3 dropped; `resolve_handler_target`, the body WEH accessors,
  `EcsDom`'s `window_entity`, the `HostDriver` trait item and the shell lookups are untouched; the boundary is recorded
  at D-8. Ax2 IMP ×3 (two homes, rebind asserts, detached-body regression) → gone with D-2 / D-3. Ax1 IMP (two copies of
  target resolution) → gone. Ax3 IMP (PR-A not a base-case slice) → one PR (§1). Ax5 IMP (D-11 split bundled) → D-11
  dropped (984 lines, `dom/mod.rs` is not touched). Ax4 T-4 → at-target items run in the bubble pass regardless of
  `bubbles` (DOM §2.9 step 6.14.1), expectation corrected; step 6.9.7 (was "6.7"); `composedPath()` hiding = steps
  7–16 (was "10–15"); *retarget* steps 1 and 2 cited by step; HTML §13.2.7 `load` step is 11.5 (was "9.5", a drift).
  Ax3 MINs → every slot carries owner / trigger / acceptance (§7); the override-flag deferral re-measured (the firing
  site has `document`; the reason is now "target semantics, not path", not "needs the inverse lookup"); dead
  `composed_path_for_js` deleted (D-6). Ax5 edges with C0b / C0c / T3 / C1-pre / CV P2 D11 → §8. H-14 widened. A-*
  tests, rebind asserts, the PR-A / PR-B split removed. §2, §3, §6, §8, §9 re-derived. Coordinates re-pinned at
  `e8b6a24d`.
- *Rev 1* (2026-11-05): first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Coordinates are `47b6c934`. At the implementation base (§9 item 1) every coordinate is re-measured with the commands in
§0.2; the counts are re-derived, not carried (the cited files each re-pin moved are listed in H-0; H-23 decides the
landing order). The ledger row's own coordinates (`pipeline.rs:470/516`,
`content/mod.rs:738/762`) are re-stated in §0.3 where they drifted. Sibling plans not on main are cited at the commits
named in §8 and were read from their branches (`git show <branch>:docs/plans/<memo>`).

### §0.2 Premises (re-measured at `47b6c934`)

| id | claim | evidence (command → result) |
|---|---|---|
| H-0 | **Provenance of the re-pins**: the files this memo cites that each re-pin changed; every other cited file is byte-identical across that re-pin, so its measurement carries | Rev 3.2 (`4728972c` → `1580af13`, #603): `git diff --name-only 4728972c 1580af13 -- crates docs/architecture` → 21 files (verified 2026-11-07); each is grepped in this memo by the spelling the memo uses, `/usr/bin/grep -c -F '<spelling>' docs/plans/2026-11-event-path-window.md` (`/usr/bin/grep`, not ugrep's `grep`, which can print 0 silently) — **11 cited** (each ≥ 1): `focus/sot.rs`, `content/event_handlers.rs`, `content/event_loop.rs`, `content/focus.rs`, `iframe/mod.rs`, `content/mod.rs`, `content_change_on_blur_tests.rs`, `content_focus_fixup_tests.rs`, `content_test_support.rs`, `elidex-shell/src/lib.rs`, `elidex-shell/src/pipeline.rs`; **10 not cited** (each 0): `focus/delegate.rs`, `focus/mod.rs`, `focus/tests.rs`, `update_steps.rs`, `ancestor_cache.rs`, `elidex-form/src/lib.rs`, `radio.rs`, `submit.rs`, `iframe/load.rs`, `iframe/render.rs`. (Rev 3.2 said "eight" and missed `content/event_handlers.rs` — H-22's `:878/:941` were `:893/:956` at `4728972c` — `content_focus_fixup_tests.rs` and `content_change_on_blur_tests.rs`, §5.1; the coordinates were right.) Rev 3.3 (`1580af13` → `47b6c934`, #604–#606): `git diff --name-only 1580af13 47b6c934 -- crates docs` → 10 files (verified 2026-11-07); **2 cited**: `vm/tests/mod.rs` (+1 `mod` line at `:151`, #605; `:34` / `:52` unchanged; 205 → 206 lines) and `content-visibility.md` (#604; 878 → 1003 lines; the cited lines re-mapped in D-1, §3, §7, §8, DoD item 4); **8 not cited** (each 0): `handle.rs`, `natives_regexp.rs`, `natives_string.rs`, `sw_thread.rs`, `tests_regexp_last_index_overrun.rs`, `tests_service_worker.rs`, `tests_worker.rs`, `sw_fetch_relay.rs`; `docs/architecture/script.md` unchanged. Re-run at DoD item 1 with the next pin pair (verified 2026-11-07) |
| H-1 | `build_propagation_path` is the only builder of a **node**-targeted path | `git grep -n 'build_propagation_path' -- crates docs` → 13 lines outside this memo: the definition `event_dispatch/mod.rs:203`; production calls `:262` (`build_dispatch_plan`), `:373` (`dispatch_event`, the callback form), `:456` (`script_dispatch_event_core`), `crates/script/elidex-js/src/vm/host/event_target_dispatch.rs:120` (the VM `dispatchEvent` walk; its import `:20`); tests `event_dispatch/tests.rs:38/51/60`; the re-export `crates/script/elidex-script-session/src/lib.rs:34`; comments only at `crates/dom/elidex-dom-api/src/focus/sot.rs:77` and `crates/shell/elidex-shell/src/app/events.rs:195`; `docs/architecture/script.md:11` |
| H-2 | A second *get the parent* provider exists for non-entity targets | `git grep -n 'fn vm_event_parent\|fn build_vm_dispatch_plan' -- crates` → `crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs:51` / `:155` (`IDBRequest → IDBTransaction → IDBDatabase`, else null; `ObjectId`-keyed, no entity). Not a path for any node; see D-1 (c) |
| H-3 | The Window entity is a non-Node with no `TreeRelation`, created once per `Vm` at the first bind | `crates/core/elidex-ecs/src/dom/mod.rs:491` (`create_window_root`: `spawn((NodeKind::Window,))`); `crates/script/elidex-js/src/vm/vm_api.rs:221-253` (first bind allocates, later binds reuse `HostData::window_entity`, and the bind caches `window_entity → global_object` in the wrapper cache) |
| H-4 | The Window has three readers today; **none is touched by this PR** | `git grep -n 'window_entity' -- crates ':!crates/script/elidex-js/src/vm/tests'` → 50 lines in 17 files (`\| cut -d: -f1 \| sort \| uniq -c`, verified 2026-11-07 at `47b6c934`). (i) `EcsDom`'s `window_entity` (`dom/mod.rs:504`, a linear scan for *the* `NodeKind::Window`), read by `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:251` (`resolve_handler_target`) and its tests `tests_insertion_and_write.rs:60/101/235` — T3 deletes it (D-8); (ii) `HostData::window_entity` (`crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs:492`; the field `host_data/mod.rs:105`), read by `vm_api.rs:232`, `crates/script/elidex-js/src/vm/globals.rs:60` (comment), `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:448/471`, `crates/script/elidex-js/src/vm/host/history_events.rs:76/224`, `events_ui.rs:58`, `lifecycle.rs:88/399`, `document_accessors.rs:215`; (iii) the **`HostDriver`** trait's `window_entity` (`crates/script/elidex-script-session/src/engine.rs:565`; impl `crates/script/elidex-js/src/engine.rs:694` = `HostData::window_entity`), read by the shell at `crates/shell/elidex-shell/src/pipeline.rs:470/516`, `crates/shell/elidex-shell/src/content/mod.rs:738/762`, `crates/shell/elidex-shell/src/content/message_dispatch.rs:267`. (ii) is the one home of the fact; (iii) is its session-side marshalling, kept by T3 (its DoD item 4, which calls it "the `ScriptEngine` trait method" — an erratum, §8). It is **not** a `ScriptEngine` item: `ScriptEngine` (`engine.rs:57-120`) has `eval` / `call_listener` / `remove_listener` / `run_microtasks` / `drain_reactions` / `drain_timers` and the defaulted `bound_dom_mut`; `HostDriver` (`:151-722`) is not a supertrait and is not object-safe (`with_bound` is generic, `:216`) |
| H-5 | The bound document is one per `Vm`, set at bind and cleared at unbind; one `EcsDom` can hold several Document entities and only the bound one has a browsing context; one Window per `Vm`, never cleared | `bind.rs:115` (`document_entity = Some(document)`), `:121` (`None` at unbind), `:465` (`document()`), `:474-480` (`document_entity_opt()`: `None` when unbound). `git grep -n 'create_document_node()' -- crates \| grep -v tests` → `dom/mod.rs:453` (the root), `dom/tree/teardown.rs:302` (the fragment throwaway), `crates/dom/elidex-dom-api/src/element/document_base.rs:508`, `crates/dom/elidex-form/src/inert_document.rs:119` (DOMParser), `crates/script/elidex-script-session/src/scripting.rs:140`. **Single-world caveat (T3 H-T15)**: `HostData::window_entity` is set once (`bind.rs:504-508` asserts on a second set) and not cleared on unbind (`bind.rs:118-125` clears `document_entity` only); a `Vm` rebound to a different `EcsDom` would keep the old World's Window — a caller contract (`host_data/mod.rs:80-97`, "Callers must not rebind a `HostData` to a different `EcsDom` world"), kept by every production bind (H-19). C1-pre D-4 states the fact as the VM's one predicate (`document_has_browsing_context(ctx, doc)` = "`doc` is the bound document", `HostData::document`); T3 D-T3 states it as `ActiveDocument { document, window }` (§8) |
| H-6 | Every shell UA dispatch is bracketed and reaches the session builder with the bound document as `ctx.document`; the bracket closure holds `&mut ElidexJsEngine` | `git grep -n 'pipeline\.dispatch_event(\|dispatch_event_bracketed(\|result\.dispatch_event(' -- crates/shell/elidex-shell/src \| grep -v '_tests.rs\|fn dispatch_event_bracketed'` → 35 sites; all go through `PipelineResult::dispatch_event` (`crates/shell/elidex-shell/src/lib.rs:291-300`) or `dispatch_event_bracketed` (`pipeline.rs:121-136`); both build `ScriptContext::new(session, dom, document)` once and run `script_dispatch_event(engine, event, ctx)` under `with_bound(&mut ctx, |engine, ctx| …)` (`lib.rs:297-299`, `pipeline.rs:133-135`), where `engine: &mut ElidexJsEngine` (`HostDriver::with_bound`'s `f: FnOnce(&mut Self, …)`, `engine.rs:216-220`; impl `crates/script/elidex-js/src/engine.rs:419-437`); the bracket's `bind` passes `ctx.document` to `Vm::bind` (`crates/script/elidex-js/src/engine.rs:377-398`), so while bound `ctx.document == HostData::document` by construction |
| H-7 | The VM-side UA fires share the VM walk | `git grep -n 'dispatch_script_event(' -- crates/script/elidex-js/src \| grep -v 'fn dispatch_script_event\|//'` → 11 call sites: `event_target.rs:727` (`dispatchEvent`), `event_target_dispatch.rs:623/736/821/898`, `history_events.rs:187` (at the Window), `pending_tasks.rs:461` (`postMessage`), `service_worker/event.rs:211`, `worker.rs:278`, `worker_scope.rs:333`, `dom_selection_proto.rs:740` (`selectionchange` at the Document) — all under `crates/script/elidex-js/src/vm/host/`. The walk builds its plan through `build_dispatch_plan` (`event_target_dispatch.rs:119`) and reads `HostData` through `ctx.host()` (`:118`) |
| H-8 | Readers of the built path | `git grep -n '\.composed_path\b' -- crates \| grep -v 'tests\|//'` → 9 lines: `event_target_dispatch.rs:120/142` (seeds the `composedPath()` Array), `crates/script/elidex-js/src/vm/host/events.rs:187/189` (same, UA path), `crates/script/elidex-js/src/vm/natives_promise.rs:524` (hand-set `[document]`, no builder), `mod.rs:161/166` (`composed_path_for_js`), `mod.rs:373/459` (writes) |
| H-9 | `composed_path_for_js` (`mod.rs:156`) has no caller outside its own tests | `git grep -n 'composed_path_for_js' -- crates` → the definition `:156`, two doc-comment mentions `:67/:80`, the `lib.rs:34` re-export and `event_dispatch/tests.rs:433/461`. The VM's `composedPath()` returns the seeded Array as is (`crates/script/elidex-js/src/vm/natives_event.rs:143`), so DOM §2.2 `composedPath()` steps 7–16 (closed-tree hiding) are not applied — P2's (§7) |
| H-10 | Window listeners live in the ECS `EventListeners` component on the Window entity and are collected by the same `collect_listeners` as every node's | `crates/script/elidex-js/src/vm/host/dispatch_target.rs` module doc ("`Node` — the per-entity ECS `EventListeners` component (every DOM node, `Window`, `Document`, …)"); `mod.rs:306` reads `EventListeners` on any entity |
| H-11 | `currentTarget` / `composedPath()` entries are wrappers from the identity cache; the Window's cache entry is the global object | `crates/script/elidex-js/src/vm/host/elements.rs:74` (`create_element_wrapper`: cache hit first); `vm_api.rs:239-253` (bind caches `window_entity → global_object`); `crates/script/elidex-js/src/engine.rs:203` (`call_listener` wraps `event.target` / `current_target` through it); `event_target_dispatch.rs:139-149` (the `composedPath()` Array), `:320-326` (`walk_phase` writes `currentTarget` per item) |
| H-12 | The tree predicates are total on a non-tree entity | `crates/core/elidex-ecs/src/dom/tree/mod.rs:63` (`read_rel` → `Default` when no `TreeRelation`), `:39` (`is_ancestor` walks `get_parent`, so a Window descendant query is `false`); `crates/core/elidex-ecs/src/dom/tree/walkers.rs:395` (`find_tree_root` of the Window is the Window) |
| H-13 | The three dispatch loops gate at-target and bubble on the stop-propagation flags and run the target's listeners in one pass | `mod.rs:364` (`dispatch_event`, callback form), `:434` (`script_dispatch_event_core`), `event_target_dispatch.rs:42` (`dispatch_script_event`): capture / at-target / bubble each test `propagation_stopped \|\| immediate_propagation_stopped`; the at-target entries are `collect_listeners(…, None)` (`mod.rs:285`) = every listener on the target in registration order (`crates/script/elidex-script-session/src/event_listener.rs:318` `iter_matching`), capture and non-capture mixed; the host items of a shadow-internal target are capture / bubble entries (phase CAPTURING / BUBBLING). DOM §2.9 steps 6.13–6.14 run the target's capture listeners in the capture pass and its non-capture listeners in the bubble pass, both at AT_TARGET, and every item with a non-null shadow-adjusted target the same way — pre-existing, §7. The callback form and `_core` differ only in what `_core` adds per listener (`:493-519`: the `once` removal before the call, `run_microtasks` after it, `remove_listener` after a `once` call) and in `apply_retarget` at the at-target item (`:480`, `retarget(target, target)` = `target`); the plan, the phase order and the gates are the same |
| H-14 | Unlanded branches touching a file this PR touches or reads: **four classes of trivial overlap, none on a line this PR edits** | `FILES` = every path in §5 (touched) plus the read list (§5, "Read, not touched"); `for b in $(git branch -a --format='%(refname:short)' \| grep -v HEAD); do git log --format='%h %s' $b --not origin/main --numstat -- $FILES; done` over 82 branches → 27 with commits (Rev 3.3 re-run at `47b6c934`, 2026-11-07; Rev 3.2: 77 → 26). **Landed, pre-squash history** (`gh pr list --state all --head <b>`): `origin/rc2a-clone-created` (#596), `origin/rc0a-clone-mutation-hook-test-split` (#587), `origin/vm-construct-isconstructor-check` (#591), `origin/vm-engine-drop-bound` (#584: `engine.rs`, `bind.rs`, `tests_dispatch_integration.rs`, `tests_engine_common.rs`, `vm_api.rs`, `dispatch_target.rs`), `vm-globals-split-c1-pre0` (#592), `vm-host-document-split` (#599), `layout-change-on-blur-provenance` (#597), `vm-closure-resolution-lexical` (#602, landed at `4728972c`: its one `mod` line at `vm/tests/mod.rs:34` is on main), `layout-focus-fixup-canonical` (#603, landed at `1580af13`, the Rev 3.2 base: shell `lib.rs`, `pipeline.rs`, `content/mod.rs` (its test `mod` is the `#[path]` pair `:831-832`), `content/focus.rs`, `content_test_support.rs`, dom-api `focus/*` — the cited lines re-pinned, Rev 3.2 changelog), `origin/vm-regexp-lastindex-overrun` (#605, landed at `b86671a5`: its `mod` line `vm/tests/mod.rs:151` is on main), `ci-default-features-check` (#585, merge only). **Merge-only** (no own change): `k2-wire-fgit-scrub-complement`, `vm-p4-slice-1a-plan`. **Live, on files this PR touches** (`git diff origin/main...<b> -- <file> \| grep '^@@'`): (a) `vm/tests/mod.rs`, one `mod` line each (hunk positions from the three-dot diff, i.e. against each branch's merge base) — `layout-dom-c0b-node-document-fact` (`:114`), `vm-p4-rooting-carved` (`:120`, `:137`), `vm-upvalue-identity-dedup` (`:178`), `vm-p4-backup` / `vm-p4-plan` (no PR); this PR's `mod` line goes beside `:52` — disjoint hunks; (b) `content/mod.rs`, the two live focus-fixup branches (`layout-focus-fixup-ungated` and `layout-probe-hidden-focus-fixup`, `@@ -823,6 +823,10 @@` each, no PR: each appends a test `mod`) — this PR appends one too (§5), an adjacent-line textual conflict, resolved by keeping both; (c) `event_target_dispatch.rs`, `domform-submittable-category` (no PR; doc-comment lines `:741`, `:826-830`) — this PR edits `:117-122`, disjoint; (d) shell `lib.rs` (a touched file, §5 — Rev 3.1 filed it under the read list, corrected): `layout-focus-fixup-ungated` (`@@ -550,7 @@`, `@@ -606,14 @@`, `@@ -622,16 @@`) — this PR edits `:291-300`, disjoint. **Live, on files this PR only reads**: `layout-split-ecs-dom-factory-seam-abandoned` on `dom/mod.rs` (abandoned by name); `vm-p4-rooting-carved` / `domform-submittable-category` on `globals.rs`. Plans on unlanded branches that name this slot: T3 (H-T27, §8), C1-pre (`:123`, a passing mention). **Rev 3.1 re-run** (the same loop) for the three files that joined the touched set — `vm/test_helpers.rs`, `content_eh_attrs_bind_tests.rs`, `vm/tests/tests_dispatch_event.rs` → one branch, `layout-dom-c0b-node-document-fact` on `test_helpers.rs` (`@@ -84,7 @@`, `@@ -115,9 @@`; this PR edits `:211/:227`, disjoint); the other two files: none. **Rev 3.3 re-run** (the same loop at `47b6c934`, the base): the one addition is `origin/vm-regexp-lastindex-overrun` (landed, above); every other branch's classification and hunk positions are as stated; #604 (`docs/plans` only) and #606 (no file in `FILES`) add nothing |
| H-15 | `relatedTarget` is marshalled raw, never retargeted | `crates/core/elidex-plugin/src/event_types.rs:119` (`related_target: Option<u64>` entity bits); `crates/script/elidex-js/src/vm/host/event_shapes.rs:450` (bits → wrapper, no `retarget`). Touch events do not exist. §7 |
| H-16 | The session's callback-form loop `dispatch_event` has test callers only: 16 | `git grep -n -E '(^\|[^_a-z.])dispatch_event\(' -- crates \| grep -v 'MutationEvent\|self\.dispatch_event(&event)\|pipeline\.dispatch_event\|result\.dispatch_event'` → 19 lines: the definition `mod.rs:364`, its doc mention `:57`, the shell's unrelated `PipelineResult::dispatch_event` (`lib.rs:291`), and **16** calls in `event_dispatch/tests.rs` (`:75`, `:94`, `:115`, `:137`, `:156`, `:173`, `:190`, `:211`, `:233`, `:276`, `:296`, `:331`, `:360`, `:378`, `:400`, `:418`; one per test, 16 of the file's 22 tests — the other six are the three `propagation_path_*` tests `:35-62`, the two `composed_path_for_js` tests `:427-461` and `custom_event_defaults_composed_false` `:471`). The `grep -v MutationEvent` drops `crates/core/elidex-ecs/src/dom/mod.rs:235` (`fn dispatch_event(&mut self, event: &MutationEvent<'_>)`, the ECS mutation dispatcher, unrelated). Deleted by D-6; **15** of the 16 tests migrate (D-9) and one, `dispatch_flag_set_during_dispatch` (`:409-424`, the call `:418`), is deleted with its subject, the `dispatch_flag` field (D-6 (iii)). Prose mentions of the deleted form outside `mod.rs:57`: the module doc `mod.rs:3-5` and the shell comment `content_eh_attrs_bind_tests.rs:348-349` (both rewritten, D-6 (ii)) |
| H-21 | **`DispatchEvent::dispatch_flag` has no production reader once `composed_path_for_js` is gone; every implemented reader of the spec's flag is in the VM, on one field** | `git grep -n -w 'dispatch_flag' -- crates` → 20 lines at `47b6c934`: the field `mod.rs:82` and its doc `:80`, the constructor inits `:109/:144`, the only production read `:157` (inside `composed_path_for_js`, D-6 (i)), the writes `:374/:421` (the callback form, D-6 (ii)), `:460/:553` (`_core`), `event_target_dispatch.rs:112`, `natives_promise.rs:523` (+ its comment `:509`), the test helper `vm/test_helpers.rs:227` (+ doc `:211`), and test reads in `event_dispatch/tests.rs:415/419/423` (`dispatch_flag_set_during_dispatch`) and `:431/:436/:457` (the two `composed_path_for_js` tests). The spec's readers of the flag (webref `body`, 2026-11-06; the Service Workers items re-looked up with `heading` / `dfn` / `body service-workers`, 2026-11-07): DOM §2.7 `dispatchEvent(event)` step 1 ("If event's dispatch flag is set, or if its initialized flag is not set, then throw an `InvalidStateError`"); DOM §2.2 `initEvent()` step 1 ("If this's dispatch flag is set, then return"); Service Workers §4.4 `ExtendableEvent`'s *active* (`#extendableevent-active`: "active when its timed out flag is unset and either its pending promises count is greater than zero or its dispatch flag is set"), read by *add lifetime promise* step 2 (`#extendableevent-add-lifetime-promise`: "If event is not active, throw an "InvalidStateError" DOMException") = §4.4.1 `event.waitUntil(f)` ("The waitUntil(f) method steps are to add lifetime promise f to this"); and Service Workers §4.6.7 `event.respondWith(r)` step 2, a **direct** reader of the flag, not one through *active* (`#fetch-event-respondwith`: "If event's dispatch flag is unset, throw an "InvalidStateError" DOMException"). In the VM every implemented one reads the `dispatched_events` field of `Vm` (`vm/mod.rs:1392`, a `HashSet<ObjectId>` bracketed around the walk) on the JS `Event` object: `dispatchEvent` at `vm/host/event_target.rs:707` (the `contains` check; tested by `tests_indexeddb_events.rs:239`), `waitUntil` at `vm/host/service_worker/event.rs:250`, `respondWith` at `:291` (both `ctx.vm.dispatched_events.contains(&id)`); `initEvent` is not implemented (`vm/host/events.rs:47`, the module doc's deferred list), so its step 1 has no reader at HEAD. The `DispatchEvent` struct is built fresh per dispatch (`event_target_dispatch.rs:107`, `new_untrusted`) and borrowed `&mut` for the whole `_core` loop, so its copy of the flag can observe no re-entry; it is a second home with no reader (D-6 (iii)) |
| H-22 | Two shell UA dispatches target the **Document** with `bubbles = false`, so after D-1 their path is `[Window, document]` and only a Window **capture** listener newly observes them; both carry a pre-existing spec deviation that is not the path's (the second is unreachable today) | `git grep -n 'DispatchEvent::new' -- crates/shell/elidex-shell/src \| grep -v '_tests.rs\|/tests'` → 32 sites; the Document-targeted ones are `pipeline.rs:448` (`DOMContentLoaded`, bubbles — S-3), `:489` (`readystatechange`, pre-script, H-20), (a) `content/message_dispatch.rs:313-317` (`visibilitychange`, `bubbles = false` at `:317`; HTML §6.2 *update the visibility state* step 7 fires it "with its bubbles attribute initialized to true" — a flag deviation, §7) and (b) `content/iframe/thread.rs:185-196` (`dispatch_message_in_pipeline`: the **parent → OOP-child** `message`, received by the child thread from `BrowserToIframe::PostMessage` (`:82-84`) and fired at the child's Document `pipeline.document` (`:187`), `bubbles = false`; HTML §9.3.3 *window post message steps* step 8.7 fires it "at targetWindow" — a target deviation, §7, **unreachable today**: `git grep -n 'BrowserToIframe::' -- crates/shell \| grep -v _tests.rs` → the production senders are `MouseClick` (`content/event_handlers.rs:878`), `KeyEvent` (`:941`) and `Shutdown` (`iframe/lifecycle.rs:180`); `PostMessage` and `Navigate` have none). The **iframe → parent** `message` is already at the Window: `IframeToBrowser::PostMessage` (`thread.rs:116`) → `drain_parent_messages` (`iframe/mod.rs:123`, the arm `:133`) → `dispatch_message_event` (`event_loop.rs:303-305`; `content/mod.rs:753-772`, the target `window_entity()` `:762-764`), the same site the in-process iframe uses. Rev 3.1 stated (b) in the reverse direction. Neither (a) nor (b) has a test with a Window listener (§5.1) |
| H-17 | `ScriptEngine` has one implementor and no mock; `HostDriver` has one implementor | `git grep -n 'impl.*ScriptEngine for\|impl.*HostDriver for' -- crates` → `crates/script/elidex-js/src/engine.rs:175` and `:373` only. The VM-side real-engine dispatch fixture is `crates/script/elidex-js/src/tests_dispatch_integration.rs` (`fresh_unbound` `:28`, `with_bound_ctx` from `tests_engine_common.rs:15-26`) |
| H-18 | Callers of the two session entry points | `git grep -n 'script_dispatch_event(' -- crates \| grep -v '//'` → 17 lines (verified 2026-11-07 at `47b6c934`): the definition `mod.rs:565`; calls at shell `lib.rs:298`, `pipeline.rs:134`, and 14 in `tests_dispatch_integration.rs` (`:62`, `:84`, `:118`, `:155`, `:196`, `:232`, `:308`, `:390`, `:425`, `:427`, `:475`, `:522`, `:559`, `:590`; 12 inside `with_bound_ctx` closures, `:308` and `:390` after an explicit `HostDriver::bind`). `git grep -n 'script_dispatch_event_core(' -- crates \| grep -v '//'` → the definition `mod.rs:434` and one caller, `:570` (its doc `:429` names `drain_queued_events`, which does not exist in `crates` — a stale sentence, rewritten in this PR). Every other mention is a comment or doc (`engine.rs:266/315/387`, `bind.rs:169`, `natives_promise.rs:383/468/513/555`, `tests_promise_rejection_event.rs:395`, session `engine.rs:63`, shell docs) or the `lib.rs:35` re-export |
| H-19 | **"Has a browsing context" ≈ "is the bound document" — measured**: at every path build the two coincide; the only gap is the inter-bracket window, where nothing dispatches | (a) One Document per `Vm` lifetime: the shell never reassigns a pipeline's document (`git grep -n '\.document = ' -- crates/shell/elidex-shell/src \| grep -v _tests.rs` → none) and has no `document.open()` native (`git grep -n -i 'document_open' -- crates/script/elidex-js/src/vm/host` → none); the `document_methods_installed` comment's "unbind doc1 → bind doc2" (`host_data/mod.rs:110-116`) names no production caller (T3 H-T15 measured the same). (b) Navigation and iframes build a new `EcsDom` + `ElidexJsEngine` per document (`pipeline.rs:220-226`; `app/navigation.rs:295`, `content/navigation.rs:216` → `load_document` → a new pipeline) and run `teardown_document()` on the outgoing one (`lib.rs:466`; callers `app/navigation.rs:322`, `content/navigation.rs:223`, `content/message_dispatch.rs:102`, `content/iframe/lifecycle.rs:177/665/667`) — elidex's *destroy a document* moment (HTML §7.5.10 step 6: "Set document's browsing context to null"); the Document entity dies with its World. (c) No bfcache: `git grep -n -i 'bfcache' -- crates` → one hit, the `PageTransitionEvent` field doc `event_types.rs:202`; no restore path. (d) Between `unbind` and the next `bind` the page document is unbound while, in the spec, it still has its browsing context — but every dispatch runs inside a bracket (H-6, H-7), so no path is built in that window. So at HEAD the spec's "Document with a non-null browsing context" and elidex's "the bound document" pick the same entity at every `get_the_parent` call; D-8's "agree by construction" is softened to this measured equivalence |
| H-20 | **Inside the brackets the pre-bind population is empty** | `Vm::bind` allocates the Window at the first bind (`vm_api.rs:228-235`), and `document_entity_opt()` is `Some` while bound (`bind.rs:474-480`), so inside any bracket `HostData::active_document()` is `Some`. The shell's `window_entity().unwrap_or(document)` fallbacks (`pipeline.rs:470/516`, `content/mod.rs:738/762`, `message_dispatch.rs:267`) read *outside* the bracket and take the fallback only if the engine has never bound; `window_entity` is never cleared (H-5), so "never bound" also means "no script has run" = no Window listener exists. If the fallback is taken, the dispatch then runs inside a bracket with `active = Some`, and the `document`-targeted event's path is `[Window, document]` for `storage` / `message` / `beforeunload` / `unload` / `resize` (arm (a)) and `[document]` for `load` (arm (a), the `load` test) — with no Window listener to observe it. `DOMContentLoaded` (`pipeline.rs:448-450`) fires inside a bracket after the scripts ran, so S-3 is a real observation |
| H-23 | **T3's landing state at the base** (the cross-lane agreement, D-2 / §8): the two greps that decide the landing order, re-run at DoD item 1 before implementation | `git grep -n 'fn active_document' -- crates` → **0** at `47b6c934`; `git grep -n -E 'ActiveDocument \{' -- crates ':!*/tests/*' ':!*tests*.rs' ':!*test_*.rs'` → **0** (T3 is not on main: this PR lands first and adds the type, the constructor, the trait item and its impl — DoD item 4's three `fn` and one construction). **If T3 landed first**, the expected result is exactly: `fn active_document` → **one**, `HostData::active_document` in `host_data/engine_feature/bind.rs` (this PR then adds the `HostDriver` item and its impl, making DoD item 4's three); `ActiveDocument \{` → the definition line in `event_handler_processing/mod.rs` and **one** construction, inside that method; and `git grep -n 'active_document()' -- crates/script/elidex-js/src/vm/vm_api.rs` → ≥ 1 (T3's `Vm::bind` site calls the method). **Any other result** — `ActiveDocument` defined with no `fn active_document`, a second construction (a struct literal in `vm_api.rs` or in a `ctx`-taking helper), a definition elsewhere — is a T3 landing that did not route `Vm::bind` through `HostData::active_document()`: **stop and raise it to the PM before implementation** (PM 455's agreement is Layout's to keep; this PR does not rewrite T3's landed code, D-2); nothing is patched here |

### §0.3 Premise errors in the ledger row / briefs

- "`build_propagation_path` is the only path builder": true for node targets (H-1), but a second *get the parent*
  provider exists for `VmObject` targets (H-2), and `natives_promise.rs:524` hand-sets a one-item path (H-8). D-1 (c)
  says what this slice does with each.
- "the shell's Window-targeted dispatches (`pipeline.rs:470/516`, `content/mod.rs:738/762`)": the lines are the
  `window_entity()` lookups; the dispatches are `pipeline.rs:474/521/533` and `content/mod.rs:749/772` (`storage`,
  and the iframe→parent `message`; `message_dispatch.rs:273` is one more, `resize`; `:320` is not — it is
  `visibilitychange` at the Document, H-22 (a); Rev 3.1 called it the iframe→parent `message`). None puts the Window
  on a node's path, as the row says; after D-1 they are unchanged (the Window's parent is null).
- "Shell input events use the same builder (code read)": confirmed transitively (H-6), not directly — no shell site
  calls `build_propagation_path`; they call `PipelineResult::dispatch_event` / `dispatch_event_bracketed`.
- The Rev 2 brief's, Rev 2's own and PM 421's "the `ScriptEngine` trait item `window_entity`" (also T3 DoD item 4):
  `window_entity` is a **`HostDriver`** item (H-4 (iii)). `script_dispatch_event_core` takes `&mut impl
  ScriptEngine` (`mod.rs:434-438`), which cannot reach it, so Rev 2's session-side composition
  `engine.window_entity().map(…)` could not compile — the R1 recommendation carried the error. D-2 (Rev 3) gives the
  session loop the fact as a value argument read in the shell's bracket closure.
- The Rev 3 brief's "its 17 tests" for the callback-form loop: 16 (H-16). The two `composed_path_for_js` tests are a
  separate pair, also deleted (D-6).
- The Rev 2 brief's "`HostData::window_entity` plus C1-pre D-4 `document_has_browsing_context`" names the fact
  correctly but not its session-side carrier: D-4 is a VM-host predicate (`vm/host/document_accessors.rs`, C1-pre, not
  on main) that the session crate cannot call. The carrier is D-2's value.
- Rev 2's DoD item 4 "`fn active_document` → exactly one": under D-2 there are three definitions (the `HostData`
  constructor, the `HostDriver` item, its impl) — §9 item 4.
- The Rev 2 brief's "decide whether the 'load → None' Document-arm exception is implementable now": that exception is
  in scope and was in Rev 1 (now D-5). What R1 asked to re-measure is the deferral of the *legacy target override
  flag*; its Rev 1 reason ("needs the Window→Document direction") was false, since the firing site has `document` —
  D-5 and §7 give the corrected reason.
- The brief's "(uses at ~:262 / :373 / :456)" and "`create_window_root` sets no `TreeRelation`" are exact at
  `47b6c934` (H-1, H-3).

### §0.4 Decisions

- **D-1 — one *get the parent*.** `event_dispatch::get_the_parent(dom, node, event, active) -> Option<Entity>` (NEW)
  in `crates/script/elidex-script-session/src/event_dispatch/mod.rs` implements DOM §2.7's default and the three
  overrides as four arms keyed on the entity's components: (a) **Document** (DOM §4.5): `None` if
  `event.event_type == "load"`; else `Some(active.window)` when `active.document == node` (the Document has a
  browsing context, and its relevant global object is that Window); else `None` (no browsing context: the DOMParser,
  inert, throwaway and any other unbound Document, H-5); (b) **ShadowRoot** (DOM §4.8): `None` if `!event.composed`
  and `node` is the root of the original target (`dom.find_tree_root(event.target)`), else `host`; (c) **Node** (DOM
  §4.4): `dom.get_parent(node)` — the *assigned slot* arm is CV P2's (`docs/plans/2026-10-content-visibility.md:544`,
  `:709-712`), and the function is shaped so that P2 adds it as one `if let` in this arm; (d) **default** (Window,
  Worker, MessagePort, Selection, any other non-Node entity): `None`. `build_propagation_path(dom, event, active)` is
  its only production caller and takes the `&DispatchEvent` (the ShadowRoot arm needs `target` and `composed`, the
  Document arm `event_type`); it keeps the `MAX_ANCESTOR_DEPTH` bound. The `VmObject` provider `vm_event_parent` (H-2)
  stays where it is: its targets are per-VM `ObjectId` handles (CLAUDE.md side-store exception (a)), it never meets a
  node, and folding an `ObjectId` chain into an `Entity` function would need the engine type in the session crate — §7
  records the two-provider state once, with its reason. The rejection loop (`natives_promise.rs:524`) is not a
  consumer and keeps its hand-set path; its comment ("matching what a regular dispatch … would produce") is rewritten,
  since a regular Document dispatch now carries the Window item (its target moves under
  `#11-unhandledrejection-dispatch-target`, §7).
- **D-2 — the browsing-context fact is read, not stored: one constructor, one trait item, a value argument.** The
  Document arm's input is `active: Option<ActiveDocument>` where `pub struct ActiveDocument { pub document: Entity,
  pub window: Entity }` is T3 D-T3's `Copy` value (the active document of the engine's single browsing context and
  its relevant global object; `None` = not bound). The type is defined once, in
  `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` (T3's placement; Q-1 decided, §10) and
  re-exported from `lib.rs`; whichever of T3 and this PR lands first defines it with the same shape and doc, the
  second reuses it (§8). **Home**: the fact has one home, `HostData` (`document_entity`, `window_entity`; H-4 (ii),
  H-5), the same home C1-pre D-4 reads; no second store, no component (PM 132). **One constructor**:
  `HostData::active_document(&self) -> Option<ActiveDocument>` (NEW, `bind.rs`, beside `document_entity_opt` `:474`) =
  `self.document_entity_opt().zip(self.window_entity()).map(|(document, window)| ActiveDocument { document, window })`
  — bound-gated by `document_entity_opt` (`None` when unbound), and the Window read is the field (`:492`). It is
  **mandatory in both landing orders and the only constructor of `ActiveDocument`** (PM 441). **Cross-lane
  agreement (PM 455)**: T3's memo (rev 4.4) builds the value at two marshalling points — the consumer's construction
  in `Vm::bind` (T3 `:447-448`; `:538` "marshalled at `vm_api.rs:300`"; `:919`) and its "one host helper" (`:648-649`:
  "`ActiveDocument` from `host.document_entity_opt()` and `host.window_entity()`; `None` = unbound") — and routes
  both through `HostData::active_document()`: the helper *is* this method, and the `Vm::bind` construction calls it.
  The PM informs Layout; this memo makes no edit to T3's memo, and this PR does not rewrite T3's landed code (Rev
  3.1's "rewrite the helper's body" fallback is withdrawn). So in either order there is one struct literal
  `ActiveDocument { .. }` outside tests, inside this method (DoD item 4); a T3 landing without the method is a
  deviation from the agreement, caught by H-23's two greps (their expected result per landing order; re-run at DoD
  item 1) and raised to the PM before implementation, not patched here. A `ctx`-taking helper cannot be the
  constructor: `ElidexJsEngine::active_document(&self)` (the
  `HostDriver` impl below) has no `NativeContext`, so it would need a second field-reading site, which breaks I-2.
  **One trait item**: `HostDriver::active_document(&self) -> Option<ActiveDocument>` (NEW,
  `crates/script/elidex-script-session/src/engine.rs`, beside `window_entity` `:565`), implemented once
  (`crates/script/elidex-js/src/engine.rs`, beside `:694`) as `self.vm.inner.host_data.as_deref().and_then(
  HostData::active_document)` — the same delegation shape as `window_entity`'s impl. **Two entries, one gate**: the
  two shell bracket sites read it inside the `with_bound` closure, where `engine: &mut ElidexJsEngine` (H-6) — `let
  active = engine.active_document(); script_dispatch_event(engine, event, ctx, active)` (`lib.rs:297-299`,
  `pipeline.rs:133-135`; the `&self` borrow ends at the `Copy` return before the `&mut` reborrow, and
  `elidex_script_session::HostDriver` is already in scope at both files for `with_bound`); `script_dispatch_event(engine,
  event, ctx, active)` → `script_dispatch_event_core(engine, event, ctx, active)` → `build_dispatch_plan(dom, event,
  active)` (`mod.rs:565-573`, `:434-457`, `:261`). The VM walk `dispatch_script_event` passes
  `ctx.host().active_document()` (`event_target_dispatch.rs:117-122`). The `engine.window_entity()` + `ctx.document`
  composition of Rev 2 is gone; both entries read the one bound-gated constructor (R2 Ax2 MIN: the gating mismatch).
  *Why a value argument and not a trait read inside the builder*: the builder's callers have no `ScriptEngine` at the
  VM walk, and `ScriptEngine` has no host-fact items (H-4 (iii), H-17) — a trait read inside `build_propagation_path`
  would make the session's path builder engine-bound; the value keeps `build_dispatch_plan` and the mock engine (D-9)
  engine-independent. *Why not a closure*: the fact is a named value T3 already defines, and a `Copy` pair is what the
  tests build. *Why not `EcsDom`'s `window_entity`*: a world query answers "is there a Window" for every Document in
  the world (wrong for the DOMParser document, H-5, T-9) — the registry idiom T3 deletes. *Why not a field on
  `ScriptContext`*: `ScriptContext` is built by the shell before the bind (`lib.rs:292`), when the Window may not
  exist (H-20), and a field read by two of its sixteen construction sites is a second carrier of a fact the engine
  already owns. **Pre-bind**: inside the brackets there is no pre-bind case (H-20); outside them nothing builds a path.
  **Single world (T3 H-T15)**: `active.window` comes from `HostData::window_entity`, which unbind does not clear; the
  arm is correct under the caller contract "one `EcsDom` per `Vm`" (`host_data/mod.rs:80-97`), which every production
  bind keeps (H-19 (b)); B1 makes it structural. **B1**: `Option<ActiveDocument>` is the interim single-browsing-context
  form of the fact. When `#11-browsing-context-state-ecs-components` moves `HostData`'s per-context fields to
  components on the entity whose fact each is (B1 §5 req 5, `docs/plans/2026-06-agent-scoped-ecsdom-world.md:447-452`:
  one `Vm` then hosts several same-agent documents, so a VM-global value is presumptively wrong), the Document arm
  reads the Document entity's component (`dom.get::<…>(node)`) and the `active` argument, the constructor and the
  trait item are deleted; the arm's spec logic (`load` first, then the browsing-context test, then the Window) and the
  tests' expectations stay, the tests' fixture changes from building `ActiveDocument` to inserting the component. One
  place changes, the same as T3's.
- **D-3 — the path is built once per dispatch.** `DispatchPlan` (`mod.rs:248`) gains `path: Vec<Entity>` (NEW
  field); `build_dispatch_plan` builds it once and buckets from it; the re-builds (`mod.rs:456`,
  `event_target_dispatch.rs:120`; `mod.rs:373` is gone with the callback form, D-6) read `plan.path` instead.
  `build_propagation_path` stays `pub` (tests) with one production caller. This is a behaviour-preserving change (the
  same walk, run once instead of twice per dispatch) and lands as commit 2 of this PR rather than as a prerequisite PR:
  D-1 changes the builder's signature at the same call sites, so a separate PR would touch the same lines twice and buy
  a second full gate for ~20 lines; one PR is PM 421's ruling (§1; the memo claims no base case of the edge-dense
  rule), and commit 2 is reviewable on its own (`cargo test` green with no test change).
- **D-4 — the event path stays `Vec<Entity>`.** DOM §2.2's *event path item* fields (seven: invocation target,
  invocation-target-in-shadow-tree, shadow-adjusted target, relatedTarget, touch target list, root-of-closed-tree,
  slot-in-closed-tree) are, at `47b6c934`, functions of (invocation target, original target, event):
  *shadow-adjusted target* = `apply_retarget` per item (`mod.rs:625`; the Window item's is null by DOM §2.9 step
  6.9.6.2, and `retarget(target, Window)` yields the outermost light-tree host because a Window is not a node — DOM
  §4.8 *retarget* step 1 third bullet is false for a non-node B, so step 2 repeats to the host; H-12);
  *invocation-target-in-shadow-tree* and *root-of-closed-tree* are readable off the entity; *slot-in-closed-tree* is
  false until P2; *relatedTarget* and *touch target list* have no producer (H-15). A per-item struct buys nothing for
  the Window and is the first feature's to introduce that needs a stored field (§7).
- **D-5 — `load`.** The Document arm's `"load"` comparison is the whole exception and is in scope. The shell keeps
  firing the Document's `load` at the Window (`pipeline.rs:470-475`; HTML §13.2.7 step 11.5) — path `[Window]`,
  unchanged. A `load` dispatched at a node (`img`, `script`, `link`, `iframe`) stops at the Document: Document capture
  listeners run, Window listeners do not. The *legacy target override flag* (DOM §2.9 step 2: `event.target` reads the
  Window's associated Document) is not implemented at HEAD and stays out. Re-measured: it is implementable now
  without any Window→Document lookup — the firing site holds `document` (`pipeline.rs:436`, a parameter of
  `dispatch_lifecycle_events`), so the shape is a `DispatchEvent` field carrying the override target that the first
  path item's shadow-adjusted target reads, plus both walkers' `target` slot seeding. It stays out because it is
  target semantics (what `e.target` reads at the Window item), not the path, and this slice is the path only — §7.
- **D-6 — retargeting and `composedPath()` need no change; the two test-only production functions are deleted.**
  `retarget` (`mod.rs:583`) and `apply_retarget` (`:625`) already compute the spec result for a Window invocation
  target (D-4, H-12). The Window reaches `composedPath()` through the wrapper cache (H-11), so `e.composedPath().at(-1)
  === window` and `e.currentTarget === window` hold in both walkers without a new marshalling site. Deleted
  (dead-code rule; the class "production code whose only callers are tests"): (i) `composed_path_for_js`
  (`mod.rs:150-192`), its two tests (`tests.rs:427-461`), its re-export (`lib.rs:34`) and its two doc-comment mentions
  (`mod.rs:67/:80`) — it computes a per-`currentTarget` filtered copy of a Rust `Vec<Entity>`, a shape the VM never
  reads (the VM seeds one Array per dispatch and returns it as is, `natives_event.rs:143`), so "connecting" it is not a
  wiring but the design of the closed-tree surface, which is P2's (CV D11, §8); (ii) the callback-form loop
  `dispatch_event` (`mod.rs:364-424`), its re-export (`lib.rs:35`) and its prose mentions — the field doc `:57`
  ("Built by `dispatch_event()`"), the module doc `mod.rs:3-5` ("The `dispatch_event` function traverses the
  propagation path and invokes listeners via a callback") and the shell comment `content_eh_attrs_bind_tests.rs:348-349`
  ("`PipelineResult::dispatch_event` → session `dispatch_event` → `call_listener`"), each rewritten to name
  `script_dispatch_event_core` — its 16 callers are tests (H-16, verified 2026-11-06), and Rev 2's reason for keeping
  it ("its deletion would move the session's own loop tests onto a mock engine") is the same mock U-9 needs anyway
  (R2 Ax3 IMP); 15 of the tests migrate onto `script_dispatch_event_core` (D-9) and the sixteenth goes with (iii).
  The loop count is two (`_core`, the VM walk); their collapse is `#11-dispatch-at-target-two-pass`'s (§7).
  (iii) **`DispatchEvent::dispatch_flag`** (`mod.rs:75-82`), the same dead-state class once (i) is gone: its only
  production reader was `composed_path_for_js` (`:157`), and the spec's readers — DOM §2.7 `dispatchEvent` step 1
  (the `InvalidStateError` on a re-entrant dispatch), DOM §2.2 `initEvent` step 1 (not implemented, `events.rs:47`),
  Service Workers §4.4.1 `waitUntil(f)` (through §4.4 `ExtendableEvent`'s *active*, *add lifetime promise* step 2)
  and §4.6.7 `respondWith(r)` step 2 (a direct read of the flag; H-21) (`service_worker/event.rs:250/291`) — read it
  on the JS `Event`
  object, where every implemented one is the `dispatched_events` field of `Vm` (H-21), where it must: the Rust struct
  is rebuilt per dispatch and borrowed `&mut` for the whole loop, so a copy of the flag on it can never observe a
  re-entry. Two homes of one flag, every reader on one of them → the unread one goes (R3-10,
  decided on merit: the "connect" branch of the dead-code rule has nothing to connect to, the VM reader is already
  wired). Deleted with it: the constructor inits (`:109/:144`), the writes (`:460/:553`, `event_target_dispatch.rs:112`,
  `natives_promise.rs:523` and its comment line `:509`), the test helper's set and doc (`vm/test_helpers.rs:211/:227`),
  and the test `dispatch_flag_set_during_dispatch` (`tests.rs:409-424`), whose only subject is the field. The
  `composed_path` field doc (`:57-74`) is rewritten to DOM §2.2 (§4.3) and carries what `:75-81` said that still holds:
  the retained `composed_path` is never read after dispatch (the VM clears its Array slot,
  `event_target_dispatch.rs:243`; DOM §2.2 `composedPath()` step 3 returns the empty list for an empty path).
  `retarget`'s docstring (`:575`) cites DOM §4.8 (was §2.5, R2 Ax4 IMP; §4.3).
- **D-7 — phases, stop flags, Window-targeted events, the depth cap.** `eventPhase` is unchanged (the Window is a
  capture / bubble item, never at-target for a node event; a Window-targeted event stays `AT_TARGET` with path
  `[Window]`). `stopPropagation()` at the Window's capture item skips at-target and bubble by H-13.
  Window-targeted dispatches (`resize`, `load`, `beforeunload`, `unload`, `message`, `storage`, `popstate`,
  `hashchange`) get path `[Window]` from arm (d) — no doubled Window. The `MAX_ANCESTOR_DEPTH` bound stays in
  `build_propagation_path` (one item deeper now); it is a sibling instance of
  `#11-capped-walker-deep-element-processing`, §7. The at-target one-pass order (H-13) is pre-existing and stays
  (§7); T-4 is written so that it does not depend on it.
- **D-8 — handler-target determination is T3's; the boundary.** HTML §8.1.8.1 *determine the target of an event
  handler* step 3 ("If eventTarget's node document is not an active document, then return null") and step 4 (the node
  document's relevant global object) are implemented by T3's `determine_event_handler_target` (T3 D-T1, D-T3; slot
  `#11-event-handler-target-determination`), replacing `resolve_handler_target` and deleting `EcsDom`'s `window_entity`.
  This PR does not touch `resolve_handler_target` (`event_handler_processing/mod.rs:251`), `native_body_weh_get` /
  `_set` (`event_handler_attrs.rs:448/471`), `EcsDom`'s `window_entity` (`dom/mod.rs:504`), the `HostDriver` item
  `window_entity` (`engine.rs:565`) or the shell lookups (H-4 (iii)). The two algorithms read the same fact (D-2's
  `ActiveDocument`, both through `HostData::active_document()`) and agree once both have landed — by the measured
  equivalence of H-19, not by construction alone: a `body` whose node document is not the active document gets no
  Window handler (T3 step 3), and an event dispatched at a node of that document never reaches the Window (D-1 arm
  (a)). *Rejected (R1 Ax4 CRIT)*: Rev 1's D-3 kept the handler on the element where step 3 returns null.
- **D-9 — tests and the mock engine.** A new VM end-to-end file, a new session unit file, a new shell test file, and
  one test-only mock `ScriptEngine` in the session crate: `event_dispatch/test_support.rs` (NEW, `#[cfg(test)]`),
  `RecordingEngine` — `call_listener` pushes `(listener_id, current_target, event.phase, event.target,
  event.composed_path.clone())` to a `Vec` and, if the test installed one, runs a closure `FnMut(ListenerId, Entity,
  &mut DispatchEvent)` (the callback form's shape, so the 15 migrated tests keep their bodies); `remove_listener`
  records the id; `eval` returns success; `run_microtasks` / `drain_reactions` / `drain_timers` are no-ops;
  `bound_dom_mut` keeps the default `None`, so `_core` uses `ctx.dom`. The same file is the home of the three fixture
  helpers now private to `mod tests` (`tests.rs:4` `elem`, `:10` `setup_shadow_dom`, `:22` `add_listener`; plain
  `fn`, unreachable from a sibling module): they move to `test_support.rs` as `pub(super)` and `tests.rs` /
  `tests_get_the_parent.rs` import them (`use super::test_support::*`). Each migrated test builds `SessionCore::new()`
  and `ScriptContext::new(&mut session, &mut dom, root)` (`root` = the test's tree root; `ctx.document` is not read by
  `_core`, H-18, and `active` is the argument — `None` for all 15, so their paths are today's; the fourth argument
  arrives with commit 3, §4). Behaviour-preserving: the plan, the phase order, the gates and the
  retarget are the same in both loops (H-13); what `_core` adds per listener is a no-op on the mock (`run_microtasks`)
  or invisible to these tests (`once` removal — no migrated test registers `once`: `grep -n 'once' tests.rs` → one
  comment line, `:404`, at `47b6c934`, and the `add_listener` helper `:22-32` takes no `once`). The red-before-fix
  controls are named in §6.

## §1. Shape and verdict

**Edge-dense: yes.** The slice binds six intersecting invariant axes (§2): (A1) the path algorithm, (A2) the
browsing-context fact's marshalling at two entry points, (A3) wrapper identity of the Window as `currentTarget` /
`composedPath()` entry, (A4) retargeting with a non-node invocation target, (A5) two loops consuming one plan,
(A6) worlds with several Documents. Three or more intersect at the Document arm alone (A1 × A2 × A6). Under CLAUDE.md
(*Edge-dense work*), `/elidex-plan-review` is mandatory before implementation (R1 and R2 done; R3 on this revision).
**One PR**: PM 421 (the ruling on the R1 dispositions: PM 132 kept, no component or store, the handler target is T3's,
the slice is the path only in one PR). The slice has one design decision (D-2, which reads an existing fact through one
constructor and a value argument) and one behaviour change (the Window on the path); it carries no data-home move, no
split and no handler-target change (R1 Ax3 IMP, Ax5 IMP). It is not claimed as base case (c) of the edge-dense rule:
there is no umbrella above it (R2 Ax3 MIN).

**Breadth** (§3): K=2, M=21 → preflight says SPLIT-RECOMMENDED (M ≥ 20), not SPLIT-DEFAULT. Single PR, stated:
of the 21 rows, 6 are hand-offs or records with no code here (the two relatedTarget rows, step 2, `composedPath()`
7–16, §8.1.8.1, §7.2.2) and the remaining 15 all land in one function and one plan field (§4), two of them with a
pre-existing ✗ half recorded in §7 (6.13, 6.14); a split would cut one function in two. Rev 1's M=18 covered two PRs;
the three rows added in Rev 2 (6.13 split from 6.14, the target-item steps split from the parent-append steps, the
relatedTarget steps) are the step-level records R1 asked for, not new code.

## §2. Coupled invariants

- **I-1 path**: the path is DOM §2.9 step 6's item sequence — target, then each *get the parent* until null.
- **I-2 fact**: "this Document has a browsing context, and its Window is W" has one home (`HostData`), one constructor
  (`HostData::active_document()`, in either landing order — the cross-lane agreement PM 441 / 455: T3's `Vm::bind`
  construction and its host helper both go through it, so the struct literal appears once outside tests, D-2, DoD
  item 4), and reaches the builder as a value; no arm queries the world for a Window.
- **I-3 identity**: the Window as an invocation target marshals to the global object (SameObject with `window`).
- **I-4 retarget**: at a non-node invocation target, `event.target` is the outermost light-tree host.
- **I-5 walkers**: the two loops consume one plan; neither re-derives the path.
- **I-6 documents**: a Document that is not the bound document has no event parent.
- **I-7 load**: the Document arm returns null for `load`; Window-targeted `load` is unaffected.

Pairs (every pair of the seven; "—" = independent, stated once):
- I-1 × I-2 = the Document arm is `active.document == node`, never a world query (a query is what breaks I-6; T-9).
- I-1 × I-3 — (identity is the cache's, H-11; the path holds entities).
- I-1 × I-4 = the Window item's *shadow-adjusted target* is null (step 6.9.6.2) and the lazy `apply_retarget` at that
  item walks `retarget(target, Window)` to the outermost host (DOM §4.8 step 1, third bullet false for a non-node B;
  U-5, T-6, both with `composed: true` so the ShadowRoot arm yields the host — DOM §4.8 *get the parent*).
- I-1 × I-5 = `DispatchPlan::path` is built once; `plan.capture` / `plan.bubble` are bucketed from it; both loops read
  `plan.path` for `composed_path` (D-3; U-9).
- I-1 × I-6 = the DOMParser / inert / throwaway Document is a path root (arm (a) `None`); its subtree's events never
  reach the Window (T-9, U-1).
- I-1 × I-7 = the `"load"` comparison is in the Document arm, so a `load` at a node stops at the Document and a `load`
  at the Window (path `[Window]`) never reaches the arm (T-5).
- I-2 × I-3 = `HostData::window_entity` is the entity the bind cached as `globalThis` (`vm_api.rs:239-253`), so the
  Window the arm returns is the one whose wrapper is `window` (T-3).
- I-2 × I-4 — (retargeting reads the tree, not the fact).
- I-2 × I-5 = both entries read the one constructor (D-2): the session loop receives `engine.active_document()` read in
  the bracket closure, the VM walk reads `ctx.host().active_document()`; both are bound-gated by `document_entity_opt`,
  so a mismatch between the two entries is impossible (T-1 and S-1 cover one entry each; U-9 the argument's effect).
- I-2 × I-6 = `active.document` is one entity per `Vm`; every other Document compares unequal (H-5); the bound one is
  the one with a browsing context at every dispatch (H-19).
- I-2 × I-7 — (the `load` test precedes the fact read).
- I-3 × I-4 = `e.target` at the Window listener is the host's wrapper, `e.currentTarget` the global object (T-6).
- I-3 × I-5 = both loops resolve `current_target` through `create_element_wrapper` (H-11); no walker-specific Window
  marshalling.
- I-3 × I-6, I-3 × I-7 — (identity does not depend on which Document or which type).
- I-4 × I-5 — (`apply_retarget` is shared, `mod.rs:625`).
- I-4 × I-6 = the Window's own tree queries are total (H-12), so no arm needs an "is this a node" pre-check beyond the
  component match.
- I-4 × I-7, I-5 × I-7 — (`load` changes the path's end, not retargeting or the loops).
- I-5 × I-6 = the rejection loop's hand-set `[document]` path is not a consumer (H-8); its Window move is another slot.
- I-6 × I-7 = both are `None` arms of the same function; order inside the arm is `load` first (U-1 covers each alone).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §2.7 Interface EventTarget | *get the parent*, default | non-Node entity (Window, Worker, MessagePort, Selection) → null | `get_the_parent` arm (d) (NEW) | ✓ | yes |
| WHATWG DOM §4.4 Interface Node | *get the parent* | (i) assigned → assigned slot (CV P2, `content-visibility.md:544`); (ii) otherwise parent | arm (c): (ii) now, (i) P2's | ✗ (P2) | yes |
| WHATWG DOM §4.5 Interface Document | *get the parent* | (i) type is `load` → null; (ii) no browsing context → null; (iii) relevant global object | arm (a): (i) string compare, (ii) `active.document != node` or `active` is `None`, (iii) `active.window` | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *get the parent* | (i) composed unset and root of path[0]'s invocation target → null; (ii) host | arm (b) (HEAD stops at any shadow root when non-composed; equal until P2) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *retarget* steps 1–2 | step 1: (i) A not a node; (ii) A's root not a shadow root; (iii) B a node and A's root a shadow-including inclusive ancestor of B → return A; step 2: A ← A's root's host, repeat | `retarget` `mod.rs:583` — B = Window makes (iii) false (H-12); its docstring `:575` re-cited to this section (D-6) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | steps 6.3, 6.8, 6.9.9 (the target item; *get the parent* calls) | target appended first; each parent from the previous item's *get the parent* | `build_propagation_path` (D-1, D-3) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | steps 6.9.6 / 6.9.8 (append a parent) | Window parent, or a node in the target's root → 6.9.6.2 append with null shadow-adjusted target; otherwise 6.9.8 with shadow-adjusted target = parent | `build_propagation_path` + lazy `apply_retarget` (D-4) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | steps 4, 6.9.3, 6.9.7 (relatedTarget retargeting; 6.9.7 is the "Otherwise" of 6.9.6 — a parent that is neither a Window nor a node in the target's root, and is the retargeted relatedTarget → set to null, so 6.9.8 does not append it) | no producer (H-15) | absent → `#11-related-target-retargeting` (§7) | ✗ | no |
| WHATWG DOM §2.9 Dispatching events | step 6.13 (capture pass; 6.13.1 AT_TARGET for items with a non-null shadow-adjusted target) | Window item: CAPTURING, capture listeners | the two loops' capture phase (H-13); ✗ host items of a shadow-internal target read CAPTURING, spec AT_TARGET (pre-existing, §7) | ✗ | yes |
| WHATWG DOM §2.9 Dispatching events | step 6.14 (bubble pass; 6.14.1 AT_TARGET items run regardless of `bubbles`; 6.14.2.1 others continue when `bubbles` is false) | Window item: BUBBLING iff `bubbles`; the target's non-capture listeners always run (T-4) | the two loops' at-target + bubble phases (H-13); ✗ the target's capture and non-capture listeners run in one registration-ordered pass (pre-existing, §7) | ✗ | yes |
| WHATWG DOM §2.9 Dispatching events | *invoke* steps 2–3 (shadow-adjusted target look-back) | null at the Window item → preceding non-null | lazy `apply_retarget` `mod.rs:625` (D-4) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | *invoke* step 6 (stop propagation flag) | set at the Window's capture item → at-target and bubble skipped | the two loops' gates (H-13) | ✓ | yes |
| WHATWG DOM §2.9 Dispatching events | step 2 (*targetOverride*, legacy target override flag) | Window `load`: target reads the associated Document | absent at HEAD → `#11-window-load-legacy-target-override` (§7) | ✗ | no |
| WHATWG DOM §2.2 Interface Event | `composedPath()` steps 1–6 (path non-empty; currentTarget appended) | Window entry marshals through the wrapper cache (H-11) | `event_target_dispatch.rs:139-149`, `events.rs:187` | ✓ | yes |
| WHATWG DOM §2.2 Interface Event | `composedPath()` steps 7–16 (closed-tree hiding), 17 (return) | not applied by the VM (H-9) | `composed_path_for_js` deleted (D-6) → P2, `#11-shadow-slot-assignment-flat-tree` (CV D11, §8) | ✗ | yes |
| WHATWG DOM §2.2 Interface Event | *path*, *event path item* (seven fields) | shadow-adjusted target / invocation-target-in-shadow-tree / root-of-closed-tree derived; slot-in-closed-tree false until P2; relatedTarget / touch target list have no producer (H-15) | `Vec<Entity>` (D-4); `DispatchPlan::path` (NEW, D-3); the `composed_path` field doc `mod.rs:57-81` rewritten to this section (§4.3) | ✗ (relatedTarget, touch) | no |
| WHATWG HTML §7.3.2 Browsing contexts | a Document's *browsing context* ("initially null"; the note: a Document created by an API such as `createDocument()` never has a non-null one; nulled by *destroy a document*, §7.5.10 Destroying documents, step 6) | **approximation, measured (H-19)**: non-null iff the Document is the bound document (C1-pre D-4's reading, H-5) — exact at every path build at HEAD, since a Document is destroyed with its World at `teardown_document` and nothing dispatches between brackets | `active.document == node` (D-2) | ✓ | no |
| WHATWG HTML §8.1.3.3.4 Relevant | relevant global object of a Document | the bound document's Window | `active.window` (D-2) | ✓ | no |
| WHATWG HTML §7.2.2 The Window object | *associated Document* | the inverse of the pair; not read here | `HostData` (`document_entity`, `window_entity`), unchanged | ✓ | no |
| WHATWG HTML §13.2.7 The end | step 8.2 (`DOMContentLoaded` at the Document, bubbles); step 11.2 (browsing context null → abort), 11.3 (window = relevant global), 11.5 (fire `load` at window, legacy target override flag) | 8.2: path `[Window, Document]` now (T-10, S-3); 11.3 / 11.5 target: `pipeline.rs:470` unchanged; 11.2: the shell's `document` fallback predates this memo (unobservable, H-20); the flag: ✗ (§7) | `dispatch_lifecycle_events` `pipeline.rs:432-475` | ✗ (the flag) | no |
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* step 3 (node document not active → null), step 4 (relevant global object) | T3's `determine_event_handler_target`; not touched here (D-8) | boundary record only | ✗ (T3) | yes |

**Breadth**: K=2 (dom, html), M=21 → SPLIT-RECOMMENDED by preflight; single PR justified in §1.

### §3.1 User-input touch audit

- Event type strings from script (`new Event(type)`; `dispatchEvent`): arm (a)'s `"load"` compare is exact and
  case-sensitive, as DOM §4.5 reads it; `"Load"` reaches the Window (spec).
- The `composed` flag from script: arm (b) reads it; a non-composed event at a node inside a shadow tree still stops
  at that root (unchanged).
- Documents created by script (`DOMParser`) and by the engine (fragment throwaway, inert form document): arm (a)
  returns null for all of them because none is `active.document` — fail-safe direction = no Window; an unknown
  Document kind falls to the same side.
- Pre-bind: inside the brackets the population is empty (H-20). The shell's `unwrap_or(document)` fallbacks (§0.3)
  can be taken only before any script has run, and then no Window listener exists to observe the `[Window, document]`
  path they would get; the pre-script `readystatechange` dispatches are in the same case. `DOMContentLoaded` fires
  after the scripts (S-3).
- `<body on*>` content attributes: not read by this PR (D-8).
- Adjacent pre-existing, exposure unchanged: the `MAX_ANCESTOR_DEPTH` truncation of a deep path (silent;
  `#11-capped-walker-deep-element-processing` is the family); the VM's `composedPath()` exposing closed-tree entries
  (P2, §8); the at-target one-pass order (§7).
- No new network, storage or permission surface.

## §4. Mechanism (one PR, three commits)

**Commit 1 — delete the two test-only production functions and the unread flag; the mock engine; the test
migration (D-6, D-9, behaviour-preserving).** `composed_path_for_js` (`mod.rs:150-192`) and the callback-form
`dispatch_event` (`mod.rs:364-424`) go, with their `lib.rs:34-35` re-exports and prose mentions (`mod.rs:3-5`, `:57`,
`:67`, `:80`; `content_eh_attrs_bind_tests.rs:348-349`, comment only); `DispatchEvent::dispatch_flag` goes
(`mod.rs:75-82`, `:109`, `:144`, `:460`, `:553`; `event_target_dispatch.rs:112`; `natives_promise.rs:509/:523`;
`vm/test_helpers.rs:211/:227`; H-21); `tests.rs:409-424` (`dispatch_flag_set_during_dispatch`) and `:427-461` (the
two `composed_path_for_js` tests) go; `event_dispatch/test_support.rs` (NEW) adds `RecordingEngine` and receives the
three fixture helpers from `tests.rs:4-32` (D-9); the 15 remaining `dispatch_event` tests (H-16) call
`script_dispatch_event_core(&mut engine, &mut event, &mut ctx)` with the same closure bodies. `cargo test -p
elidex-script-session -p elidex-js --all-features` green (the flag's writes are in both crates); the 15 tests'
assertions unchanged.

**Commit 2 — `DispatchPlan::path` (D-3, behaviour-preserving).** `build_dispatch_plan` keeps the path it builds in
`plan.path`; `script_dispatch_event_core` (`:456`) and `dispatch_script_event` (`event_target_dispatch.rs:120`) set
`composed_path` from it (`clone()` in the session loop where the plan is borrowed; a move in the VM walk, which owns
`local`). `cargo test -p elidex-script-session -p elidex-js --all-features` green with no test change.

**Commit 3 — `get_the_parent`, the fact's carrier, the tests (D-1, D-2, D-5, D-7, D-9).**

```rust
/// WHATWG DOM §2.7 *get the parent*, with the §4.4 / §4.5 / §4.8 overrides.
/// Called at path-build time only (`debug_assert!(event.original_target.is_none())`).
/// `active`: the bound document and its Window (HTML §7.3.2 / §8.1.3.3.4), `None` when unbound.
pub fn get_the_parent(
    dom: &EcsDom, node: Entity, event: &DispatchEvent, active: Option<ActiveDocument>,
) -> Option<Entity> {
    let world = dom.world();
    if let Ok(sr) = world.get::<&ShadowRoot>(node) {                       // §4.8
        let stops = !event.composed && dom.find_tree_root(event.target) == node;
        return (!stops).then_some(sr.host);
    }
    match dom.node_kind(node) {
        Some(NodeKind::Document) => {                                       // §4.5
            if event.event_type == "load" { return None; }
            active.filter(|a| a.document == node).map(|a| a.window)
        }
        Some(kind) if kind.is_node() => dom.get_parent(node),               // §4.4 (P2: assigned slot first)
        _ => None,                                                          // §2.7 default
    }
}
```

```rust
// crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs, beside `document_entity_opt` (:474)
/// The bound document and its Window (HTML §7.3.2 *browsing context*, §8.1.3.3.4
/// *relevant global object*), `None` when not bound. The one constructor of
/// `ActiveDocument` (T3's host helper); read by `HostDriver::active_document` and
/// the VM dispatch walk.
pub fn active_document(&self) -> Option<ActiveDocument> {
    self.document_entity_opt()
        .zip(self.window_entity())
        .map(|(document, window)| ActiveDocument { document, window })
}

// crates/script/elidex-script-session/src/engine.rs, `HostDriver`, beside `window_entity` (:565)
/// The bound document and its Window, `None` when not bound — the browsing-context
/// fact the event path's Document arm reads (DOM §4.5 *get the parent*). Read inside
/// a `with_bound` bracket and passed to `script_dispatch_event` as a value.
#[must_use]
fn active_document(&self) -> Option<ActiveDocument>;

// crates/shell/elidex-shell/src/lib.rs:297-299 and pipeline.rs:133-135 (the same three lines)
self.runtime.with_bound(&mut ctx, |engine, ctx| {
    let active = engine.active_document();
    elidex_script_session::script_dispatch_event(engine, event, ctx, active)
})
```

`build_propagation_path(dom, event, active)` pushes `event.target`, then `get_the_parent` until `None` or the depth
bound, and reverses; `build_dispatch_plan(dom, event, active)` stores it in `plan.path`;
`script_dispatch_event(engine, event, ctx, active)` and `script_dispatch_event_core(engine, event, ctx, active)`
thread it through; `dispatch_script_event` reads the fact **before** its scoped DOM borrow — `NativeContext::host`
is `&mut self` (`native_context.rs:171`) and `HostData::dom` is `&mut self` (`bind.rs:158`), so
`build_dispatch_plan(dom, &local, ctx.host().active_document())` inside the `let dom = ctx.host().dom();` scope is
E0499:

```rust
// crates/script/elidex-js/src/vm/host/event_target_dispatch.rs:117-122 (step C)
let active = ctx.host().active_document();
let plan = {
    let dom = ctx.host().dom();
    build_dispatch_plan(dom, &local, active)
};
local.composed_path = plan.path;      // D-3: the second walk is gone
```

The 14 `script_dispatch_event` calls in `tests_dispatch_integration.rs` (H-18) pass `engine.active_document()` (they
hold `&mut ElidexJsEngine`); the 15 migrated `script_dispatch_event_core` calls in `event_dispatch/tests.rs` (commit
1) gain `None` as the fourth argument (D-9), and the three `build_propagation_path` tests `(&dom, &event, None)`.
Existing tests whose assertions the Window item changes are updated in this commit (§5, the R3-1 sweep; DoD item 6).
The `NodeKind::is_node` predicate (`crates/core/elidex-ecs/src/components.rs:538`) is false for Window / Worker /
OffscreenCanvas, so arm (c) never reads a `TreeRelation` off a non-Node; `dom.node_kind` is
`crates/core/elidex-ecs/src/dom/classify.rs:97`. Nothing else in the two loops changes (H-13).
`docs/architecture/script.md:11-12` is rewritten to the new signatures, the Window item and the `HostDriver` item;
the `natives_promise.rs:510-516` comment is rewritten (D-1); the `_core` doc's `drain_queued_events` sentence
(`mod.rs:429`) is corrected (H-18).

### §4.1 ECS-native check (OO → ECS mapping)

The OO column is a contrast only; the authority for each row is the spec section named in §3, then ECS first
principles — not the OO engine's structure.

| OO engine idiom [contrast] | elidex form in this PR | why |
|---|---|---|
| `Document::domWindow()` — a class-owned pointer from the Document to its Window; the `EventPath` builder reads it | no field on the Document: the fact is browsing-context state in `HostData` (`document_entity`, `window_entity`), read per dispatch through one constructor and passed as a `Copy` value (D-2) | PM 132 / T3 D-T3: not a per-entity fact of the Document; its ECS home is a per-context component under B1 (the side-store→component rule applies then, on the entity whose fact it is — B1 §5 req 5), and this PR adds no interim store |
| `EventPath` / `NodeEventContext` list — a heap list of per-node contexts owned by the event | `DispatchPlan::path: Vec<Entity>` built once per dispatch (D-3); listeners are read per path entity from the `EventListeners` component (`collect_listeners`, H-10) | the path is a query result over components, not an object graph; the Window is one more entity with `NodeKind::Window` and an `EventListeners` component (H-3, H-10) — no Window-specific listener registry |
| `LocalDOMWindow` as an `EventTarget` subclass with its own listener map | the Window entity's `EventListeners` component, the same component every node has | marker component + query, not inheritance |
| a virtual `getParent()` on `EventTarget` overridden per class | one function with arms keyed on components (`ShadowRoot` component, `NodeKind::Document`, `NodeKind::is_node`) — D-1 | dispatch on component presence, not on class; P2's slot arm is one more component read in arm (c) |
| per-listener retargeting caches on the event | `apply_retarget` recomputes from the tree (D-4, D-6) | derived, not stored |

Side-store audit: this PR adds no entity-keyed side store; `ActiveDocument` is a per-call value (not a `HashMap`, not a
registry); the test mock's recording `Vec` is test-only.

### §4.2 Layering

| site | layer | what it does | engine-independent API reused |
|---|---|---|---|
| `get_the_parent`, `build_propagation_path`, `build_dispatch_plan`, `script_dispatch_event(_core)` (`event_dispatch/mod.rs`) | `elidex-script-session` (engine-independent) | the DOM §2.9 step 6 algorithm | `EcsDom::get_parent` / `find_tree_root` / `node_kind`, the `ShadowRoot` component (`elidex-ecs`) |
| `ActiveDocument` (`event_handler_processing/mod.rs`) | `elidex-script-session` | the fact's value type (T3's) | — |
| `HostDriver::active_document` (`engine.rs`) | `elidex-script-session` (trait declaration) | marshalling contract | — |
| `HostData::active_document` (`host_data/engine_feature/bind.rs`) | VM host data (`elidex-js`) | marshalling: two field reads → one value | — |
| `ElidexJsEngine::active_document` (NEW) (`elidex-js/src/engine.rs`, the `impl HostDriver`) | engine adapter | delegation to `HostData` | — |
| `dispatch_script_event` (`vm/host/event_target_dispatch.rs:117-122`) | `vm/host/` (invoker) | reads `ctx.host().active_document()`, calls `build_dispatch_plan`, seeds the `composedPath()` Array | `build_dispatch_plan` (session) — no algorithm added in `vm/host/` |
| `PipelineResult::dispatch_event`, `dispatch_event_bracketed` (shell) | shell | reads the trait item inside the bracket, passes the value | `script_dispatch_event` (session) |
| `RecordingEngine` (`event_dispatch/test_support.rs`) | `elidex-script-session`, test-only | `ScriptEngine` mock | — |

The Layering mandate's check: no DOM algorithm is written in `vm/host/` (the walk calls the session's builder as
today); `EcsDom::*` calls added in `vm/host/` — none.

### §4.3 Citations in new or rewritten docs

Every § below was looked up with `.claude/tools/webref heading` / `body` on 2026-11-06.

| doc | cites | verified title |
|---|---|---|
| `get_the_parent` (NEW) | DOM §2.7 (default), §4.4 (node), §4.5 (document), §4.8 (shadow root) | §2.7 *Interface EventTarget*; §4.4 *Interface Node*; §4.5 *Interface Document*; §4.8 *Interface ShadowRoot* |
| `build_propagation_path` (rewritten) | DOM §2.9 steps 6.3 (append the target), 6.8 (first *get the parent*), 6.9.9 (next parent) | §2.9 *Dispatching events* |
| `DispatchPlan::path` (NEW field) | DOM §2.2 *path* (a list of event path items), built per §2.9 step 6 | §2.2 *Interface Event* (`dfn dom path` → `#event-path`) |
| `composed_path` field doc (`mod.rs:57-74`, rewritten; `:75-82` is deleted with `dispatch_flag`, D-6 (iii)) | DOM §2.2 *event path item*: the seven fields; §2.2 `composedPath()` step 3 (empty path → empty list) | §2.2 *Interface Event* (the current doc says "DOM §2.9 specifies per-entry metadata" and lists five fields; the struct is §2.2's and has seven) |
| `dispatch_flag` deletion (D-6 (iii), the commit message / H-21) | DOM §2.7 `dispatchEvent(event)` step 1 (dispatch flag set → `InvalidStateError`); §2.9 *dispatch* steps 1 and 10 (set / unset); Service Workers §4.4 (*active*), §4.4.1 (`waitUntil(f)`), §4.6.7 (`respondWith(r)` step 2) | §2.7 *Interface EventTarget*; §2.9 *Dispatching events* (`body dom dom-eventtarget-dispatchevent` / `concept-event-dispatch`, 2026-11-06); Service Workers §4.4 *ExtendableEvent*, §4.4.1 *event.waitUntil(f)*, §4.6.7 *event.respondWith(r)* (`heading service-workers 4.4` / `4.6.7`, `dfn` / `body` for `extendableevent-active`, `extendableevent-add-lifetime-promise`, `wait-until-method`, `fetch-event-respondwith`, 2026-11-07) |
| `retarget` docstring (`mod.rs:575`, corrected) | DOM §4.8 *retarget* | §4.8 *Interface ShadowRoot* (was "§2.5" = *Constructing events*) |
| `HostData::active_document` (NEW) | HTML §7.3.2 (a Document's *browsing context*), §8.1.3.3.4 (*relevant global object*) | §7.3.2 *Browsing contexts*; §8.1.3.3.4 *Relevant* |
| `HostDriver::active_document` (NEW) | DOM §4.5 *get the parent*; HTML §7.3.2 / §8.1.3.3.4 as above | as above |
| `ActiveDocument` (NEW here or T3's) | HTML §7.3.2, §8.1.3.3.4 (T3's doc text) | as above |
| `natives_promise.rs:510-516` comment (rewritten) | no § (names the shared walker and `#11-unhandledrejection-dispatch-target`) | — |
| `docs/architecture/script.md:11-12` | no § | — |

## §5. Populations

`wc -l` at `47b6c934` (re-run at the base). The 1000-line audit: no touched file is above 1000 or crosses it;
`event_target_dispatch.rs` 902 shrinks by two lines (D-3, D-6 (iii)) and grows by the `active` read;
`natives_promise.rs` 961 loses one line and a comment line (D-6 (iii)) and has its comment rewritten; `pipeline.rs`
971 gains three lines inside one function and no new unit (no seam, no split); `dom/mod.rs` (984) is not touched.

| file | lines | change |
|---|---|---|
| `crates/script/elidex-script-session/src/event_dispatch/mod.rs` | 654 | `get_the_parent` (NEW), `build_propagation_path` / `build_dispatch_plan` / `script_dispatch_event` / `script_dispatch_event_core` signatures, `DispatchPlan::path` (NEW), −1 re-build, −`composed_path_for_js` (`:150-192`), −`dispatch_event` (`:364-421`), −`dispatch_flag` (`:75-82`, `:109`, `:144`, `:460`, `:553`), docs at `:1-5` (module doc), `:57-74`, `:429`, `:575` rewritten; `#[cfg(test)] mod test_support;` and `#[cfg(test)] mod tests_get_the_parent;` beside `mod tests;` (`:653-654`) |
| `crates/script/elidex-script-session/src/event_dispatch/test_support.rs` (NEW) | — | `RecordingEngine` (D-9); the three fixture helpers moved from `tests.rs:4-32` as `pub(super)` |
| `crates/script/elidex-script-session/src/event_dispatch/tests.rs` | 481 | −`:4-32` (helpers → `test_support.rs`, imported); `:38`, `:51`, `:60` take `(&dom, &event, None)`; the 15 remaining `dispatch_event` calls (H-16) become `script_dispatch_event_core` on `RecordingEngine` in commit 1 and gain `None` as the fourth argument in commit 3; −`:409-424` (`dispatch_flag_set_during_dispatch`, D-6 (iii)); −`:427-461` (two tests) |
| `crates/script/elidex-script-session/src/event_dispatch/tests_get_the_parent.rs` (NEW) | — | U-1..U-9; declared beside `mod tests;`; helpers from `test_support.rs` |
| `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` | 410 | `ActiveDocument` (NEW, ~10 lines) unless T3 landed first (§8) |
| `crates/script/elidex-script-session/src/engine.rs` | 722 | `HostDriver::active_document` (NEW item, beside `:565`) |
| `crates/script/elidex-script-session/src/lib.rs` | 68 | `:34-35`: −`composed_path_for_js`, −`dispatch_event`, +`get_the_parent`; +`ActiveDocument` re-export |
| `crates/script/elidex-js/src/engine.rs` | 829 | `impl HostDriver`: `active_document` (NEW, beside `:694`) |
| `crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs` | 573 | `HostData::active_document()` (NEW, beside `:474`) unless T3 landed first (§8) |
| `crates/script/elidex-js/src/vm/host/event_target_dispatch.rs` | 902 | −`:112` (`local.dispatch_flag = true`, D-6 (iii)); `:117-122`: `let active = ctx.host().active_document();` before the scoped borrow, `build_dispatch_plan(dom, &local, active)`, `local.composed_path = plan.path` (§4 commit 3) |
| `crates/script/elidex-js/src/vm/natives_promise.rs` | 961 | `:509-516` comment rewritten (D-1; the `dispatch_flag` mention dropped), −`:523` (D-6 (iii)) |
| `crates/script/elidex-js/src/vm/test_helpers.rs` | 324 | `make_event`: doc `:211` and −`:227` (`ev.dispatch_flag = true`, D-6 (iii)); H-14 for this file: one live branch, `layout-dom-c0b-node-document-fact` (hunks `@@ -84,7 @@` / `@@ -115,9 @@`, disjoint) |
| `crates/script/elidex-js/src/tests_dispatch_integration.rs` | 597 | the 14 `script_dispatch_event` calls (H-18) pass `engine.active_document()` |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 206 | `mod tests_event_path_window;` beside `:52` (`tests_dispatch_event`; #605's `mod` line is `:151`) |
| `crates/script/elidex-js/src/vm/tests/tests_event_path_window.rs` (NEW) | — | T-1..T-10 |
| `crates/script/elidex-js/src/vm/tests/tests_dispatch_event.rs` | 742 | `composed_path_returns_path_during_dispatch` (`:327-338`): `assert_eq!(out, 4.0)` at `:337` → `5.0`, the comment `:336` → `[p, body, html, doc, window]`, and the test also asserts the last `composedPath()` entry `=== window` (§5.1; the one existing test the Window item changes). No branch touches the file (H-14 re-run for it) |
| `crates/shell/elidex-shell/src/lib.rs` | 719 | `:297-299` (`PipelineResult::dispatch_event`, `:291-300`): the `active` read + argument |
| `crates/shell/elidex-shell/src/pipeline.rs` | 971 | `:133-135` (`dispatch_event_bracketed`): the same |
| `crates/shell/elidex-shell/src/content/mod.rs` | 869 | after `:832`: `#[cfg(test)] #[path = "../content_event_path_window_tests.rs"] mod event_path_window_tests;` (the file's convention, `:787-832`; the last pair `:831-832` is #603's `content_focus_fixup_tests.rs`) |
| `crates/shell/elidex-shell/src/content_event_path_window_tests.rs` (NEW) | — | S-1..S-3 (one file per feature, as `content_change_on_blur_tests.rs` / `content_fragment_nav_tests.rs`; harness `super::test_support::{build_main_document, probe_body_attr}` from `content_test_support.rs:377/405`, a local `click` helper in the shape of `content_eh_attrs_bind_tests.rs:96-101`) |
| `crates/shell/elidex-shell/src/content_eh_attrs_bind_tests.rs` | 390 | `:348-349` comment only: "session `dispatch_event`" → `script_dispatch_event_core` (D-6 (ii)); no branch touches it (H-14 re-run for this file) |
| `docs/architecture/script.md` | 50 | `:11-12` |

Read, not touched (the H-14 read list): `crates/script/elidex-script-session/src/event_listener.rs`,
`crates/script/elidex-script-session/src/scripting.rs`, `crates/core/elidex-ecs/src/dom/mod.rs`,
`crates/core/elidex-ecs/src/dom/classify.rs`, `crates/core/elidex-ecs/src/dom/tree/mod.rs`,
`crates/core/elidex-ecs/src/dom/tree/walkers.rs`, `crates/core/elidex-ecs/src/dom/tree/teardown.rs`,
`crates/core/elidex-ecs/src/components.rs`, `crates/core/elidex-plugin/src/event_types.rs`,
`crates/script/elidex-js/src/tests_engine_common.rs`, `crates/script/elidex-js/src/vm/host_data/mod.rs`,
`crates/script/elidex-js/src/vm/vm_api.rs`, `crates/script/elidex-js/src/vm/globals.rs`,
`crates/script/elidex-js/src/vm/natives_event.rs`, `crates/script/elidex-js/src/vm/tests/tests_dispatch_event.rs`,
`crates/script/elidex-js/src/vm/tests/tests_dom_parser.rs`,
`crates/script/elidex-js/src/vm/host/event_target_dispatch_vm.rs` (673; the VmObject provider, D-1 (c)),
`crates/script/elidex-js/src/vm/host/events.rs`, `crates/script/elidex-js/src/vm/host/event_shapes.rs`,
`crates/script/elidex-js/src/vm/host/dispatch_target.rs`, `crates/script/elidex-js/src/vm/host/elements.rs`,
`crates/script/elidex-js/src/vm/host/event_handler_attrs.rs`, `crates/script/elidex-js/src/vm/host/history_events.rs`
(242; dispatches at `HostData::window_entity`), `crates/script/elidex-js/src/vm/host/document_accessors.rs`,
`crates/dom/elidex-form/src/inert_document.rs`, `crates/dom/elidex-dom-api/src/element/document_base.rs`,
`crates/shell/elidex-shell/src/content/message_dispatch.rs`, `crates/shell/elidex-shell/src/content_test_support.rs`
(`content_eh_attrs_bind_tests.rs` and `vm/tests/tests_dispatch_event.rs` moved to the touched table in Rev 3.1).
Not touched by design:
`dom/mod.rs`, `components.rs`, `vm_api.rs`, `event_handler_attrs.rs`, `event_handler_processing/mod.rs:251`,
`content/mod.rs` outside the `#[path]` pair, `message_dispatch.rs` (D-8, H-4).

### §5.1 Existing tests on the path — the concept sweep (R3-1, PM 441)

Population: every existing test, in every crate, that asserts the event path or `composedPath()` length / contents,
or the set, order or count of listeners reached along the path — enumerated by meaning (a `currentTarget` or
`eventPhase` sequence, a listener-call log, a Window-level listener registered for a node- or Document-targeted
event, a depth-cap shape), not by one string. Run at `4728972c` on 2026-11-06 (a `reviewer` sweep, the candidates
read in full; the counts and the one changed row re-verified by hand). **Re-pin at `1580af13` (Rev 3.2)**: the
complement over #603's additions — `git diff 4728972c 1580af13 -- crates | grep '^+' | grep -E
'addEventListener|dispatchEvent|dispatch_event\(|composedPath|currentTarget|eventPhase|EventListeners|MAX_ANCESTOR_DEPTH|window\.on|ctl_listen'`
→ 7 lines: `blur` / `focusout` recorders on the control itself in `content_focus_fixup_tests.rs:193/223/243/311-312/522`
(NEW, 21 tests) and the `el.addEventListener(t, …)` probe moved from `content_change_on_blur_tests.rs` into the
harness (`content_test_support.rs:482`); no Window or Document listener, no path or `composedPath()` assertion — the
reading under which `content_change_on_blur_tests.rs` was outside the population at `4728972c`. The population and
both counts are unchanged. **Re-pin at `47b6c934` (Rev 3.3)**: the same complement over #604–#606's additions (`git
diff 1580af13 47b6c934 -- crates | grep '^+' | grep -E '<the same pattern>'`) → 0 lines (verified 2026-11-07); the
population and both counts are unchanged. Commands (each over `-- crates`; the raw hit count before reading follows each):
`git grep -n -E 'composedPath|composed_path|propagation_path|build_dispatch_plan|\.capture\b|\.bubble\b'` (148);
`git grep -n -E 'eventPhase|event_phase|currentTarget|current_target'` (152);
`git grep -n -E "window\.addEventListener|globalThis\.addEventListener|window\.on[a-z]+ *=|add_listener\(window|add_listener\(w\b|EventListeners"` (236)
and its widening to every `addEventListener(` receiver, `on<type> =` assignment and the macro-built receivers
(`ctl_listen!("window", …)` in `content_eh_attrs_bind_tests.rs`) (89 after dropping comments; a `-P` grep over
`w|win|top|parent|frames[n]|contentWindow|defaultView` receivers → 0); `git grep -n -E 'MAX_ANCESTOR_DEPTH'` (162);
`git grep -n -E "dispatchEvent\(|dispatch_event\(|script_dispatch_event\(|dispatch_script_event\("` (241, split
into test files and `#[cfg(test)]` modules); `git grep -n -iE 'composedPath|event-path|eventpath|dispatch' --
crates/tools '*.json' '*.txt' '*.toml' '*.md'` + `git ls-files crates/tools/elidex-wpt` (no `dom/events` expectation
file exists; the WPT crate is the cascade suite only); `git grep -n -E "addEventListener\('(load|DOMContentLoaded|
click|keydown|input|submit)" -- crates/shell` (16); a grep for a test pinning the defect (a Window listener asserted
*not* called for a node event) → 0.

**Changed: 1.** `vm/tests/tests_dispatch_event.rs:327` `composed_path_returns_path_during_dispatch` — `length == 4`
for `[doc, html, body, p]`; `build_simple_tree` (`:33-49`) binds the tree's own root (`bind_vm(vm, session, dom,
doc)`), so `doc` is the bound document, the type `'c'` is not `load`, and the path gains the Window → `5`. Red at
head unless updated; updated in commit 3 (§5 row; DoD item 6 counts it).

**Checked, unchanged: 193** (reason per group; the members are the `#[test]` fns in the ranges):
- `event_dispatch/tests.rs:35-471`, 22 (3 `propagation_path_*`, 16 callback-form, 2 `composed_path_for_js`, 1
  `custom_event_defaults_composed_false`): `active = None`; no Window entity in the file (3 of the 22 are deleted by
  D-6 for their own reasons).
- `tests_dispatch_event.rs:101-741` other than `:327`, 34: listeners on `el` / `body` / `html` only; `:341` asserts
  the post-dispatch `[]`.
- `tests_dispatch_integration.rs:41-597`, 13: no Window listener for `click` / `touchstart` / `keydown`; the `:144`
  Window `message` listener's target is the Window.
- `tests_call_listener.rs:57-441`, 9: call `call_listener` directly, no path. `tests_promise_rejection_event.rs:389`,
  1: the hand-set `[document]` loop (H-8). `tests_event_object.rs:126/168`, `tests_gc_audit.rs:138/171`,
  `tests_create_event_object.rs:48/165`, 6: slot / GC / construction, no dispatch. `tests_window_global.rs:342`, 1:
  registration count only.
- `tests_event_handler_attrs.rs:234/248/262/447`, 4: no dispatch, or `window.dispatchEvent` (path `[Window]`).
  `tests_event_handler_attrs_window_delegating.rs`, 9 (`:16`–`:172`): `hashchange` is Window-targeted; the node
  dispatches (`:105-136`, `:172`) are `new Event` with the default `bubbles: false`, and an event handler is never a
  capture listener, so the Window's bubble pass does not run. `tests_event_handler_scripting_gates.rs`, 14: no Window
  `click` listener; `:129` is `document.dispatchEvent` with `bubbles: false`; `:381` is a DOMParser document.
- `tests_post_message.rs:107-651`, 24; `tests_history_events.rs:52/103/214/246/300/344`, 6;
  `tests_integration_pr5b.rs:81/111/157`, `tests_engine_s1a.rs:145/179/209`, `tests_engine_s1b.rs:223/246`,
  `tests_engine_s6a.rs:455/629`, `tests_events_misc.rs:841`, 11: Window listeners whose events (`message`,
  `popstate`, `hashchange`) are Window-targeted (`pending_tasks.rs:461`, `history_events.rs:187`).
- `tests_selection.rs:642/762`, `tests_document_global.rs:63`, 3: Document listeners only.
- Shell: `tests.rs:340/360/380/409`, 4 (lifecycle: `DOMContentLoaded` at the Document with no Window `DOMContentLoaded`
  listener; `load` at the Window); the Window-listener tests `app_history_phase_sep_tests.rs:174`,
  `app_turn_completion_tests.rs:395/613`, `content_fragment_nav_tests.rs:384/419`,
  `content_history_phase_sep_tests.rs:490/545`, `content_history_pump_turn_tests.rs:131/180/238`,
  `viewport_tests.rs:162/257/419`, 13 (`popstate` / `hashchange` / `beforeunload` / `resize`, all at `window_target`:
  `pipeline.rs:518/530`, `message_dispatch.rs:270`, `content/mod.rs:740/764`); the `click` / `keydown` tests
  `tests.rs:93/322`, `content_tests.rs:271/310`, `viewport_tests.rs:546`, `app_history_phase_sep_tests.rs:303`,
  `app_turn_completion_tests.rs:285`, `content_history_phase_sep_tests.rs:389/428`, 9 (no Window listener of the
  type); `content_eh_attrs_bind_tests.rs`, 10 (`:165` `ctl_listen!("window","hashchange")` is Window-targeted; the
  rest are node `click`s with no Window listener).
- `MAX_ANCESTOR_DEPTH`: no test of the event path's cap (every hit is an ECS / dom-api / form / style walker test);
  WPT / manifests: none; `benches/event_dispatch.rs` has no assertion.

Two facts the sweep surfaced beyond its population, recorded as premises: the shell's bubbling UA events now reach
Window non-capture listeners (`change` / `submit` / `reset`, `DispatchEvent::new`'s default `bubbles: true`,
`content/form_input.rs:83/140/298`, `content/focus.rs:195` — spec-correct, the fix's purpose; `close` at
`form_input.rs:175` sets `bubbles = false`, and the iframe element's `load` at `iframe/lifecycle.rs:430` is `load`,
arm (a) `None`), and two **Document-targeted** UA dispatches outside §0.3's enumeration, H-22.

## §6. Tests

Each test names its kind: **red** = fails at `47b6c934` and passes after; **guard** = passes before and after, and
names the mutant it kills.

### §6.1 VM end-to-end (`tests_event_path_window.rs`; the `tests_dispatch_event.rs` harness: `bind_vm` from `vm/test_helpers.rs:49`, `build_simple_tree` `:33`)

- **T-1** (red; the probe): capture + bubble `click` listeners on `window`, `document`, `body`;
  `body.dispatchEvent(new Event('click', {bubbles: true}))` → `"w-cap d-cap b-cap b-bub d-bub w-bub"`.
- **T-2** (red): inside the body listener, `e.composedPath()` is `[body, html, document, window]` by `===`.
- **T-3** (red): at the window listeners, `e.currentTarget === window`, `e.target === body`, `e.eventPhase` is `1`
  then `3`.
- **T-4** (red): a non-bubbling event at `body`, listeners registered `b-cap` before `b-bub` → `"w-cap d-cap b-cap
  b-bub"`: the Window and Document items run in the capture pass only (DOM §2.9 step 6.14.2.1), the target's
  non-capture listener runs in the bubble pass regardless of `bubbles` (step 6.14.1). The registration order makes
  HEAD's one-pass at-target order equal the spec's two-pass order (H-13), so the test is about the path, not the order.
- **T-5** (guard; kills a Window arm without the `load` compare): `window.addEventListener('load', f, true)` +
  `el.dispatchEvent(new Event('load'))` → `f` not called; `document.addEventListener('load', g, true)` → `g` called;
  `window.dispatchEvent(new Event('load'))` → the window listener called once with `e.composedPath()` = `[window]`.
- **T-6** (red): open shadow root on `el`, target inside; `inner.dispatchEvent(new Event('click', {bubbles: true,
  composed: true}))` (`composed: true`, else DOM §4.8 *get the parent* returns null at the root of the first item's
  invocation target and the event never leaves the shadow tree); the window listener sees `e.target === el` and
  `e.composedPath()` ending `…, el, body, html, document, window` with the inner node and shadow root present (open).
- **T-7** (red): `stopPropagation()` in the window capture listener → no document / body listener runs;
  `stopImmediatePropagation()` there → the second window capture listener does not run.
- **T-8** (guard; kills a doubled Window): `window.dispatchEvent(new Event('resize'))` → one call, `e.eventPhase === 2`,
  `e.composedPath()` = `[window]`.
- **T-9** (guard; kills a world-query arm): a `DOMParser` document's body `dispatchEvent(new Event('click',
  {bubbles: true}))` (the `tests_dom_parser.rs` shape, `bound_vm` `:23`) → the window listener is not called; that
  document's own listener is; `e.composedPath()` ends at that document.
- **T-10** (red): `document.dispatchEvent(new Event('DOMContentLoaded', {bubbles: true}))` → the window bubble
  listener runs (the shell's `pipeline.rs:450` dispatch has this shape; S-3 is the shell-side run).

### §6.2 Engine-independent (`tests_get_the_parent.rs`; the helpers `elem`, `setup_shadow_dom`, `add_listener` and `RecordingEngine` from `test_support.rs`, D-9)

`active` is built as `Some(ActiveDocument { document: doc, window: w })` with `w = dom.create_window_root()`.

- **U-1** (red): `get_the_parent(window, click, active)` = `None`; `(document, click, active)` = `Some(window)`;
  `(document, load, active)` = `None`; `(document, click, None)` = `None`; a second Document (`create_document_node`)
  with the same `active` → `None`.
- **U-2** (red): `build_propagation_path` for `doc > html > body > p` with `active` = `[window, doc, html, body, p]`;
  `build_dispatch_plan` has the Window first in `capture` and last in `bubble`; `plan.path` equals the built path.
- **U-3** (guard): the same tree with `active = None` → `[doc, html, body, p]` (today's shape).
- **U-4** (guard): ShadowRoot arm: non-composed event targeted inside the shadow tree → `None` at that root; composed
  → `host`; a non-composed event targeted in the light tree whose walk meets a shadow root cannot occur before P2 —
  stated, not tested.
- **U-5** (red): a `composed: true` event at a shadow-internal target; `apply_retarget` at the Window item →
  `event.target == host`, `original_target == Some(inner)`.
- **U-6** (red): `script_dispatch_event_core` on `RecordingEngine` with `active` given invokes the Window's capture
  listener first and bubble listener last, through the `EventListeners` component on the Window (`add_listener(window,
  …)`); the recorded `(current_target, phase)` sequence is `(window, Capturing), …, (window, Bubbling)`.
- **U-7** (guard): the depth bound still terminates: a `doc > …` chain of `MAX_ANCESTOR_DEPTH + 2` elements with
  `active` → the path has `MAX_ANCESTOR_DEPTH + 1` items and the Window is absent (the truncation is the pre-existing
  silent cap, §7; the test pins its shape so a later uncapping is a visible change).
- **U-8** (guard; kills an arm order swap): a ShadowRoot is `NodeKind::DocumentFragment`, a node whose tree parent
  is its host (`attach_shadow_with_init` appends the shadow root to the host, `shadow.rs:203`, so
  `dom.get_parent(sr) == host`). A **non-composed** event targeted inside the shadow tree: the §4.8 arm (b) returns
  `None` at `sr` (the root of the target); a mutant that tests `node_kind` first sends `sr` to arm (c) and returns
  `Some(host)` — killed. The composed case does not separate the two (both yield `host`) and is U-4's; the Document
  side of the order (a Document never carries `ShadowRoot`) is U-1's.
- **U-9** (red): `script_dispatch_event_core(&mut RecordingEngine, &mut event, &mut ctx, Some(ActiveDocument {
  document: doc, window: w }))` sets `event.composed_path` to the plan's path and the Window is on it: `[w, doc, …]`
  recorded inside `call_listener`; the same call with `None` → `[doc, …]`. (The value is the argument; the mock has no
  host fact to fake — D-2.)

### §6.3 Shell (`content_event_path_window_tests.rs`, NEW)

- **S-1** (red): a page whose inline script runs `window.addEventListener('click', e => document.body.setAttribute(
  'data-w', e.currentTarget === window))`; `click(pipeline, "t")` (through `PipelineResult::dispatch_event`, a real
  bracket) → `probe_body_attr(&pipeline, "data-w")` = `Some("true")`.
- **S-2** (guard): `dispatch_lifecycle_events`' `load` reaches a `window.addEventListener('load')` listener with
  `e.composedPath().length === 1`.
- **S-3** (red): a page whose inline script runs `window.addEventListener('DOMContentLoaded', …)` records the call
  (HTML §13.2.7 step 8.2 through `pipeline.rs:450`; the `build_main_document` harness runs the lifecycle,
  `content_test_support.rs:377`).

## §7. Out of scope and slots

Each item states the defect at HEAD, then the slot with its **owner** / **trigger** / **acceptance** / **re-eval**.
Registration is the PM's at landing; the names are proposals unless the slot exists.

- **Two *get the parent* providers.** `vm_event_parent` (`event_target_dispatch_vm.rs:51`) chains `ObjectId` targets;
  `get_the_parent` chains entities. Reason: the `VmObject` targets are per-VM identity handles (side-store exception
  (a)); one function would need the engine's `ObjectId` in the session crate. Not a defect; recorded here once so no
  later reader re-opens it. Re-eval if a `VmObject` ever needs a node parent.
- **Window `load` target** → `#11-window-load-legacy-target-override` (proposed). *Defect*: `pipeline.rs:474` fires the
  Document's `load` at the Window without the *legacy target override flag*: `e.target === window`, spec `document`
  (DOM §2.9 step 2; HTML §13.2.7 step 11.5). *Shape* (D-5): a `DispatchEvent` field set by the firing site, which has
  `document`; the first item's shadow-adjusted target reads it; both walkers seed the `target` slot from it. No
  Window→Document lookup is needed. *Why deferred*: target semantics, not the path (D-5). **Owner**: L3 (the dispatch
  surface). **Trigger**: the first consumer that reads `e.target` in a Window `load` listener (a site's
  `e.target.readyState` / `e.target.body`), or T3's `<body onload>` cells if they observe `e.target`. **Acceptance**:
  `window.addEventListener('load', e => e.target === document)` is `true` from the shell's lifecycle dispatch and
  `e.currentTarget === window`. **Re-eval**: 2026-12-31.
- **`composedPath()` closed-tree hiding** → P2, `#11-shadow-slot-assignment-flat-tree` (exists; CV D11 names the event
  path's *get the parent* / `composedPath` as P2's own surfaces). *Defect*: the VM returns the whole path from every
  listener (H-9); DOM §2.2 `composedPath()` steps 7–16 hide a closed shadow tree's entries from listeners outside it.
  `composed_path_for_js` is deleted here (D-6); P2 designs the surface against the VM's one-Array-per-dispatch
  representation (a per-item Array, or a per-call compute in `native_event_composed_path`). **Owner**: Layout (P2).
  **Trigger**: P2's plan. **Acceptance**: a listener on the host of a closed tree sees no entry below the host in
  `e.composedPath()`; a listener inside sees them. **Re-eval**: P2's.
- **`relatedTarget` retargeting** → `#11-related-target-retargeting` (proposed). *Defect*: `MouseEvent` / `FocusEvent`
  `relatedTarget` is marshalled raw (H-15); DOM §2.9 steps 4 and 6.9.3 retarget it per item, and step 6.9.7 —
  reached only when 6.9.6 is false, i.e. the parent is neither a Window nor a node in the target's root — sets the
  parent to null when it is the retargeted relatedTarget, so that parent is not appended (6.9.8 is skipped) and the
  path does not extend past the item whose *get the parent* returned it. The first consumer introduces the per-item
  struct D-4 defers. *Why deferred*: no producer at HEAD. **Owner**: L3. **Trigger**: the first `relatedTarget`
  producer that crosses a shadow boundary (focus / mouse enter-leave in a shadow tree). **Acceptance**: a `focusout`
  whose `relatedTarget` is inside a closed tree reads the host from outside, and a parent outside the target's root
  that equals the retargeted relatedTarget is not on the path (the path's last item is the one before it); a parent
  in the target's root, or the Window, is appended regardless (6.9.6). **Re-eval**: 2026-12-31.
- **Slot assignment on the path** → P2 (`#11-shadow-slot-assignment-flat-tree`): CV `content-visibility.md:544`,
  `:709-712` — the Node arm's assigned-slot branch, `slotInClosedTree`, and the ShadowRoot arm's path-root test
  becoming load-bearing. **Owner / trigger / acceptance / re-eval**: P2's.
- **At-target order, the shadow-host items, and the two loops** → `#11-dispatch-at-target-two-pass` (proposed).
  *Defect* (H-13): the target's capture and non-capture listeners run in one registration-ordered pass; DOM §2.9 steps
  6.13 / 6.14 run the capture ones first (capture pass, AT_TARGET) then the non-capture ones (bubble pass, AT_TARGET) —
  observable as `body.addEventListener('click', bub); body.addEventListener('click', cap, true)` firing `bub, cap`
  (spec `cap, bub`); and the host items of a shadow-internal target (non-null shadow-adjusted target, step 6.9.8) read
  `eventPhase` CAPTURING / BUBBLING where the spec reads AT_TARGET, and their non-capture listeners are skipped for a
  non-bubbling event where the spec runs them (6.14.1). Second defect, folded in: two copies of the loop remain after
  D-6 — `script_dispatch_event_core` (`mod.rs:434`) and `dispatch_script_event` (`event_target_dispatch.rs:42`) each
  run steps 6.13–6.14 over one plan (one-issue-one-way). *Why deferred*: the two-pass change rewrites the phase
  iteration of both loops, which is the natural moment to collapse them, and it is a separate observable change from
  the path (this slice is the path only); T-4 is written so that it does not depend on the order. **Owner**: L3.
  **Trigger**: a WPT or site that orders at-target listeners by capture, or P2's closed-tree work (which touches the
  same items). **Acceptance**: the two-pass order above; `e.eventPhase === 2` at a shadow host listener for an event
  targeted inside its tree; and one phase runner in `elidex-script-session` with two invokers — `vm/host/` keeps only
  the invoker (the wrapper seeding and `currentTarget` write), no loop (`git grep -n 'propagation_stopped' --
  crates/script/elidex-js/src/vm/host` → 0 loop gates). **Re-eval**: 2026-12-31.
- **`unhandledrejection` at the Window** → `#11-unhandledrejection-dispatch-target` (exists; owner the core VM area,
  trigger "a PR touching rejection tracking or the dispatch target of global events"). The rejection loop targets the
  Document with a hand-built path (H-8); when the target moves it should go through the shared walker, and the
  Window's path is then `[Window]`. **Trigger fired (recorded here, the N1 form)**: this PR touches
  `natives_promise.rs` (rejection tracking) by a comment rewrite and the `dispatch_flag` line deletion (D-1, D-6
  (iii)) and changes the path of **node- and Document-targeted** events (the Window item), not the target of any
  global event; Window-targeted events keep `[Window]` (D-7). **Disposition**: no change in this PR (its scope is the path; the slot's remedy is unreviewed); the comment
  rewrite stops the file from claiming its hand-set path matches a regular Document dispatch (it no longer does: the
  regular one carries the Window item). **Acceptance** (added for the existing row): the listener runs with
  `e.currentTarget === window` and `e.composedPath()` = `[window]`. Owner and re-eval (2026-11-30) unchanged.
- **`visibilitychange` bubbles (H-22 (a))** → `#11-visibilitychange-bubbles-flag` (exists: registered PM 455 from
  this memo's Rev 3.2; Owner L3; Re-eval 2026-12-15; its coordinate `message_dispatch.rs:317` ruled correct, PM 466).
  *Defect*: `message_dispatch.rs:313-317` fires `visibilitychange` at the Document with `bubbles = false` (`:317`);
  HTML §6.2 *update the visibility state* step 7 fires it "with its bubbles attribute initialized to true", so after
  this PR a Window capture listener sees it and a Window bubble listener does not (spec: both). *Why not here*: the
  firing site's flag, not the path (D-5's reasoning). **Owner**: L3 (the row's). **Trigger** (added for the existing
  row): the first Window `visibilitychange` listener in a site or test. **Acceptance** (added for the existing row):
  `window.addEventListener('visibilitychange', f)` (non-capture) runs on a visibility change, with
  `e.bubbles === true`. **Re-eval**: 2026-12-15 (the row's).
- **The OOP child's `message` target (H-22 (b))** → `#11-windowproxy-browsing-context` (exists; a hand-off, no new
  slot — PM 455). *Defect*: the parent → OOP-child receive site `dispatch_message_in_pipeline`
  (`iframe/thread.rs:185-196`, the `BrowserToIframe::PostMessage` arm `:82-84`) fires the child's `message` at the
  child's Document (`pipeline.document`, `:187`, `bubbles = false`); HTML §9.3.3 *window post message steps* step 8.7
  fires it at `targetWindow`, so after this PR the site's path would be `[Window, document]` with `e.currentTarget
  === document` at the target item (spec: `[Window]`, `AT_TARGET` at the Window). *Unreachable today*: nothing in
  production sends `BrowserToIframe::PostMessage` (H-22); a parent reaches an OOP child's Window only through the
  WindowProxy surface that slot owns (`contentWindow.postMessage`, `docs/plans/2026-06-iframe-browsing-context-plan.md`).
  The iframe → parent `message` is not a defect (H-22: it fires at the parent's Window, `content/mod.rs:764`).
  **Hand-off** (DoD item 10): one acceptance line for the existing row — when `contentWindow.postMessage` reaches an
  OOP child, the child's `message` fires at the child's Window (`e.currentTarget === window`, `e.composedPath()` =
  `[window]`), not at its Document. Owner / trigger / re-eval: the row's.
- **The depth bound** → `#11-capped-walker-deep-element-processing` (exists; owner L3, registered PM 368). The path
  builder's bound is one more instance (D-7; U-7 pins its shape). **Acceptance** (added for the existing row's list):
  the path of a node at depth `MAX_ANCESTOR_DEPTH + 2` reaches the Window. Re-eval (2026-12-31) unchanged.
- **The browsing-context fact as components** → B1, `#11-browsing-context-state-ecs-components` (exists): the
  constructor, the trait item and the `active` argument are the interim single-browsing-context marshalling of
  `HostData`'s pair (D-2); B1 §5 req 5 moves the pair to components and the Document arm reads the component. **Owner /
  trigger**: B1's. **Acceptance** (for this memo's part): `git grep -n 'fn active_document\|ActiveDocument' -- crates`
  → none (the arm reads the component; T3's consumer reads it too).

## §8. Edges and hand-offs

- **T3 (`docs/plans/2026-10-eh-attrs-t3-target-determination.md`, rev 4.4 `d15a141c`, branch
  `layout-eh-target-determination-plan`, not on main; plan-review closed)** — D-T3 defines `ActiveDocument {document,
  window}` in `event_handler_processing/mod.rs` and builds it in the VM "through one host helper" (`:648-649`: from
  `host.document_entity_opt()` and `host.window_entity()`, `None` = unbound — D-2's constructor exactly; T3 places it
  "beside D-4, in the same layer", `:200`); D-T1 deletes `resolve_handler_target`; D-T5 deletes `EcsDom`'s
  `window_entity`; its DoD item 4 keeps `HostData::window_entity` and "the `ScriptEngine` trait method" — an erratum,
  it is `HostDriver`'s (H-4); its H-T27 and DoD item 1 already provide for this slot landing first ("T3's element
  dispatches are non-bubbling with non-capture handlers, so a Window on the path changes none of them" — still true
  under the T-4 reading: the Window item runs capture listeners only for a non-bubbling event). Its relied-on caveat
  H-T15 (`:207-209`) is this memo's too (H-5, D-2). **Order-free, one constructor (PM 441; the cross-lane agreement
  PM 455)**: `HostData::active_document()` is added by whichever PR lands first and is the only constructor of
  `ActiveDocument` in both orders. T3's memo has two marshalling points for the value (`:447-448`: the consumer's
  construction in `Vm::bind` — `:538` "marshalled at `vm_api.rs:300`", `:919` — and the VM accessor helper); the
  agreement is that T3's memo routes both through `HostData::active_document()` (the helper is the method; the
  `Vm::bind` construction calls it). The PM tells Layout; this memo makes no edit to T3's memo, and this PR does not
  rewrite T3's landed code. If T3 lands first, this PR imports `ActiveDocument` and calls the method, adding nothing
  to `event_handler_processing/mod.rs` or `bind.rs`; a T3 landing without the method (or with a second construction
  site) is a deviation from the agreement, caught by DoD item 1's re-measure (`fn active_document`, `ActiveDocument
  {`) and raised to the PM before implementation — not patched here. The `HostDriver` impl calls the method, never a
  `ctx`-taking helper (D-2: `ElidexJsEngine::active_document(&self)` has no `NativeContext`). If this PR lands first,
  T3 finds `ActiveDocument` and `HostData::active_document()` with its shape and doc and re-pins (DoD item 1),
  routing its bind-time construction (`Vm::bind`, T3 D-T3) and its accessor helper through the method. After both:
  one constructor and one struct literal outside tests (DoD item 4); `active.document == X` is the same comparison
  in T3's step 3 and arm (a), of one fact; the two algorithms read one fact (D-8). The `ScriptEngine` → `HostDriver`
  erratum is an FYI for T3's next touch.
- **C1-pre (`docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md`, rev 2.4 `81d63d84`, branch
  `layout-c1-pre-document-wrapper-plan`, not on main; plan-review closed)** — D-4 `document_has_browsing_context(ctx,
  doc)` (`:140-144`) is the VM-host predicate over the same `HostData::document` (H-5); its `:123` names event paths
  among the places a Document entity is reachable. No shared symbol, no ordering constraint; the fact's reading
  ("non-null iff the bound document") is cited from it in §3.
- **C0b / C0c (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md`, `…-c0c-adopt-notification-adoptnode.md`,
  branch `layout-dom-c0b-node-document-fact`, not on main)** — C0b renames the node-document reader (`owner_document` →
  `node_document`); C0c adds `adoptNode`. This PR reads neither (the Document arm compares the Document entity itself,
  not a node's document; Rev 1's D-3 readers of `owner_document` are gone). A node adopted by C0c into a non-bound
  Document dispatches to a path ending at that Document (arm (a) `None`) — consistent with T3's step 3 for the same
  node. The branch's one-line `vm/tests/mod.rs` add is a disjoint hunk (H-14). No ordering constraint.
- **CV P2 (Layout; `docs/plans/2026-10-content-visibility.md:229-247` D11, `:544`, `:709-712`, `:868`; slot
  `#11-shadow-slot-assignment-flat-tree`)** — P2 adds the assigned-slot `if let` to arm (c) and nothing else to the
  function; the ShadowRoot arm's path-root test (b) is already the spec's form, so P2 does not revisit it.
  `composedPath()`'s closed-tree hiding is P2's own surface (D11): this memo deletes the dead filter (D-6) and hands the
  surface over (§7). Hand-off: this memo's `get_the_parent` is the one function P2 extends; P2's memo cites it by
  symbol — on main since #604 (`228764b6`): P2-R3 (`:710-712`) names this slot as "a hard order edge (§8): P2 extends
  the one *get the parent* that slot's PR leaves (re-ground)".
- **CV-details (`content-visibility.md:750-752`)** — the CV-details open item chose the crate of its "script-visible
  shadow root" query by the rule "a home in `elidex-dom-api` cycles with `composed_path_for_js` in
  `elidex-script-session` (`event_dispatch/mod.rs:156`), so at or below `elidex-script-session`". D-6 deletes
  `composed_path_for_js`, so that premise goes. **Done on main (#604, `228764b6`)**: the sentence now carries "(L3's
  `#11-event-path-window-missing` PR deletes `composed_path_for_js`: this premise is re-grounded on its replacement
  path builder when it lands; re-ground)" (`:751-752`) and the Re-ground record (`:50`) names it; Layout re-derives
  the crate choice against the replacement builder at that landing (the query's other readers are P2's and
  CV-details' own). Nothing is left to hand over: Rev 3's F5 hand-off is closed, and DoD item 10 cites the three
  lines as the record (DoD item 4's docs grep expects them).
- **EH-attrs slice-a (`docs/plans/2026-10-eh-attrs-slice-a.md:597`, landed #563)** — its session fixtures call
  `create_window_root()`; the signature is unchanged by this PR (Rev 1's change is gone).
- **IBP handler gates (`docs/plans/2026-10-ibp-sandbox-handler-gates.md:905`, landed #580)** — its C8e row reads a
  handler through the VM dispatch walk; unchanged here (the walk's plan gains the Window item; the handler lookup is
  per listener).
- **Unlanded branches** — H-14: four trivial overlap classes on touched files (`vm/tests/mod.rs` one-line `mod` adds
  on disjoint lines; the two live focus-fixup branches' test-`mod` appends at the end of `content/mod.rs`, adjacent to
  this PR's; `domform-submittable-category`'s doc lines in `event_target_dispatch.rs`, disjoint;
  `layout-focus-fixup-ungated`'s `lib.rs` hunks at `:550` and below, disjoint from `:291-300`); none of them is an
  open PR (`layout-focus-fixup-canonical` landed as #603 at `1580af13`, `vm-regexp-lastindex-overrun` as #605 at
  `b86671a5`; the base is `47b6c934`). Rebase order is free.
- **Memory / ledger** — the slot row's "`build_propagation_path` is the only path builder" is corrected per §0.3 at
  landing; `docs/architecture/script.md:11-12` is rewritten in this PR.

## §9. Definition of done

1. Base re-measure: every command in §0.2 re-run at the base; counts and lines updated in this memo (rev bump). Two
   rows decide something: H-0's provenance is re-run with the new pin pair (the cited files that moved are the ones
   to re-read), and **H-23's two greps decide the landing order** — the result must be one of H-23's two expected
   ones; any other result means T3 landed without routing `Vm::bind` through `HostData::active_document()`: stop and
   raise it to the PM before implementation (D-2, §8); no patching here.
2. Commit 1 (D-6 deletions including `dispatch_flag`, D-9 mock and helper move, the 15-test migration) green on its
   own: `cargo test -p elidex-script-session -p elidex-js --all-features`, the 15 migrated tests' assertions
   unchanged (`git diff` of `tests.rs` shows only the call shape, the fixture lines, the helper import and the two
   deleted test ranges `:409-424` / `:427-461`). Commit 2 (D-3) green on its own: `cargo test -p
   elidex-script-session -p elidex-js --all-features` with no test change.
3. `cargo test -p elidex-script-session -p elidex-js -p elidex-shell --all-features` green at the head;
   `mise run ci` before push; `/elidex-review`; `/external-review` after push.
4. Greps, each exact on a correct implementation (the expected result is the whole result, not a subset). Line
   numbers are the base's as re-measured at item 1 (the expectation is the symbol and the file; a number that moved
   under a landed branch is re-stated there, not a failure). Comment lines have one category: the **first four**
   greps count them (the deleted names must not survive even in a doc; at `47b6c934` the `dispatch_event(` grep's 19
   lines include the doc `mod.rs:57`, which goes with the function); every later grep in this item excludes them
   with `<filter>` = `| grep -v -E ':[[:space:]]*//'` (drops `//`, `///`, `//!` lines). At `47b6c934` the filter
   drops nothing from any of them (measured): the two comment mentions of `build_propagation_path`
   (`focus/sot.rs:77`, `app/events.rs:195`) have no `(` and are outside the `build_propagation_path(` grep already,
   and no line names `get_the_parent` or `ActiveDocument` (H-23); its work is at the head, where the rewritten docs
   (`mod.rs:1-5`, `:57-74`, `script.md:11-12`, the new items' docs) may name a symbol with its parentheses or braces.
   So a doc mention of `get_the_parent`, `build_propagation_path` or `ActiveDocument { document, window }` in
   `event_dispatch/mod.rs`, `focus/sot.rs`, `app/events.rs`, `bind.rs`, `engine.rs` or a new doc is allowed and not
   counted. `git grep -n 'composed_path_for_js' -- crates` → 0; `git grep -n 'composed_path_for_js' -- docs
   ':!docs/plans/2026-11-event-path-window.md'` → three lines, `docs/plans/2026-10-content-visibility.md:50`, `:750`,
   `:752` (#604's record that the premise is retired and re-grounded on this PR's builder, §8; they stay; this
   memo's own mentions are excluded by the pathspec); `git grep -n -w 'dispatch_flag' -- crates`
   → 0 (`-w`: the VM test names `…_dispatch_flag_against_…` and `…_dispatch_flags` are not whole-word matches; H-21);
   `git grep -n -E '(^|[^_a-z.])dispatch_event\(' -- crates | grep -v MutationEvent` → exactly one line,
   `crates/shell/elidex-shell/src/lib.rs:291` at `47b6c934` (`PipelineResult::dispatch_event`'s definition; the
   `grep -v` drops the ECS `MutationEvent` dispatcher `dom/mod.rs:235`, H-16; no session definition, no doc mention,
   no `tests.rs` call — comment lines count here); `git grep -n 'build_propagation_path(' -- crates | <filter>` → the
   definition, one production call (in `build_dispatch_plan`; none in `event_target_dispatch.rs`, D-3), the three
   `tests.rs` calls and U-2 / U-3 / U-7; `git grep -n 'get_the_parent' -- crates | <filter>` → the definition, its one
   caller (`build_propagation_path`), the `lib.rs` re-export, `tests_get_the_parent.rs` (its `use` and calls) and
   its `mod` line; `git grep -n 'struct ActiveDocument' -- crates | <filter>` → exactly one; `git grep -n -E
   'ActiveDocument \{' -- crates ':!*/tests/*' ':!*tests*.rs' ':!*test_*.rs' | <filter>` (the pathspecs exclude
   `vm/tests/`, `tests*.rs` and `test_*.rs`, where U-1 / U-9 and T3's tests build the value; the filter drops a doc
   line quoting `ActiveDocument { document, window }`, which is not a construction — its category) → the definition
   line (`pub struct ActiveDocument {`) and **exactly one construction**, the `.map(|(document, window)|
   ActiveDocument { document, window })` inside `HostData::active_document` (`bind.rs`) — in both landing orders
   (the cross-lane agreement, D-2 / §8; H-23 at item 1 says which order holds and stops on any other state); any
   other hit is listed with its kind and must be a reader (a destructuring pattern), never a construction; `git grep
   -n 'fn active_document' -- crates | <filter>` → **three** in either landing order (the `HostData` constructor, the
   `HostDriver` item, its impl; T3's helper is the first of them, D-2); `git grep -n 'impl.*ScriptEngine for' --
   crates | <filter>` → `ElidexJsEngine` and the test-only `RecordingEngine`.
5. **The `window_entity` readers are unchanged** — the population is the command, compared base to head with line
   numbers dropped: `git grep -n 'window_entity' <rev> -- crates ':!crates/script/elidex-js/src/vm/tests' | sed -E
   's/^[^:]*:([^:]*):[0-9]+:/\1:/'` for `<rev>` = `origin/main` and `HEAD` (the first field is the rev); `diff` of
   the two outputs shows **only additions**, all inside `HostData::active_document`'s body and doc (`bind.rs`) and the
   `HostDriver::active_document` doc (`engine.rs`, if it names the field); no addition in `event_dispatch/`, the shell,
   or `event_target_dispatch.rs`; no deletion. Baseline at `47b6c934`: 50 lines / 17 files (H-4, verified 2026-11-07).
   If T3 landed first, its deletions (`dom/mod.rs:504`, `event_handler_processing/mod.rs:251`,
   `tests_insertion_and_write.rs:60/101/235`) are already absent from the base and `HostData::active_document` is
   on it (the cross-lane agreement, D-2 / §8); the expectation is then: **no `window_entity` reader is added** — the
   diff is empty, or shows only the `HostDriver::active_document` doc line in `engine.rs` (the same allowance as
   the this-first case); no addition in `bind.rs`, nothing else. `git grep -n 'window_entity()' -- crates/shell` →
   the same six lines as at `47b6c934` (`pipeline.rs:470/504/516`, `content/mod.rs:738/762`,
   `message_dispatch.rs:267`; `:504` is a doc comment; the shell bracket sites add no `window_entity` read).
   `git diff --stat origin/main -- crates/core/elidex-ecs
   crates/script/elidex-js/src/vm/host/event_handler_attrs.rs crates/script/elidex-script-session/src/event_handler_processing/mod.rs`
   → empty except the `ActiveDocument` definition (or fully empty if T3 landed first).
6. T-1..T-10, U-1..U-9, S-1..S-3 present and green; the red set (T-1, T-2, T-3, T-4, T-6, T-7, T-10, U-1, U-2, U-5,
   U-6, U-9, S-1, S-3) shown red on the base (run from the test commit against the pre-fix source commit; no stash).
   **Existing tests** (§5.1, the sweep's 1 / 193): the one changed test, `composed_path_returns_path_during_dispatch`
   (`tests_dispatch_event.rs:327`), is updated in commit 3 and shown red in its *updated* form on the pre-fix source
   (the same run as the red set) and, in its *old* form, red at the head (`git stash` is not used: check out the old
   file from `origin/main` into the head tree, run, restore); the 193 checked-unchanged tests are green at the head
   with no edit (`cargo test -p elidex-script-session -p elidex-js -p elidex-shell --all-features`, item 3, is the
   run; the sweep's list is the population, and any other existing test that goes red at the head is a sweep miss,
   to be added to §5.1 before merge, not silently updated).
7. The probe of the ledger row re-run on the head prints `w-cap d-cap b-cap b-bub d-bub w-bub` and
   `BODY > HTML > D > W` (`composedPath()` during dispatch).
8. `wc -l` of every touched file ≤ 1000 (§5).
9. `docs/architecture/script.md:11-12` describes `build_propagation_path(dom, event, active)`, the Window item, and
   `HostDriver::active_document`.
10. The landing memo corrects the ledger row (§8) and hands §7's texts to the PM: three proposals
    (`#11-window-load-legacy-target-override`, `#11-related-target-retargeting`, `#11-dispatch-at-target-two-pass`,
    each with its re-eval date), the acceptance additions to five existing rows
    (`#11-unhandledrejection-dispatch-target`, `#11-capped-walker-deep-element-processing`,
    `#11-browsing-context-state-ecs-components`, `#11-windowproxy-browsing-context`,
    `#11-visibilitychange-bubbles-flag` — its trigger and acceptance), and the fired-trigger disposition for
    `#11-unhandledrejection-dispatch-target`. The CV-details hand-off to Layout (Rev 3 F5) is **done on main** (#604
    `228764b6`: `content-visibility.md:50`, `:751-752` mark the `composed_path_for_js` premise "(re-ground)" on this
    PR's replacement builder, §8) — nothing to hand; the landing memo cites those lines as the record.
11. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
    docs/plans/2026-11-event-path-window.md` exits 0.
12. The citation table (§4.3) is applied: each listed doc carries the listed §, and `git grep -n 'DOM §2.5' --
    crates/script/elidex-script-session/src/event_dispatch` → 0.

## §10. Open questions

None open. Two records:

- **Q-1 (decided)** `ActiveDocument`'s module stays T3's (`event_handler_processing/mod.rs`), so neither memo edits the
  other and the second lander reuses the first's definition (§8). A crate-level home (`engine.rs`, beside
  `ScriptContext`) is a pure move either lane can make at the second landing; not this PR's.
- **Q-2 (resolved by F2)** The callback-form `dispatch_event` is deleted (D-6); 15 of its tests run
  `script_dispatch_event_core` on the test-only `RecordingEngine` (D-9) and the sixteenth goes with the
  `dispatch_flag` field it tested (D-6 (iii)). The loop count is two; the collapse is
  `#11-dispatch-at-target-two-pass`'s (§7).
