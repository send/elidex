# `focus-fixup-canonical-steps` — the render-time focus fixup runs ungated, through the canonical focus update steps (per-PR plan-memo)

This PR is slice **(2) focus-fixup** under the umbrella `docs/plans/2026-11-focus-fixup-program.md` (order, edges,
cross-PR invariants X1–X8, slot inventory and owners: its §1–§4; it lands with the provenance PR, umbrella §1.4), a terminal per-PR slice by CLAUDE.md *Edge-dense work*
base case (c). Slot **`#11-focus-fixup-gated-on-session-records`** (owner Layout, registered PM 245). Scope set by **PM
decision 248** (items 1–6, quoted by number below as "248 (n)"). Branch `layout-focus-fixup-ungated`, worktree
`wt-focus-fixup`. Rev 5.1, 2026-11-03 (PM decision 298 / 295; rev 5: PM decision 292 applied, quoted as "292 (n)"). Rev 2 applied **PM decision 252** (M1–M5, quoted
as "252 (Mn)") and the scout answers to rev 1's Q1–Q6 (§0.5); rev 3 applied **PM decision 260** (rev 2's D-11 became
slice (1), `docs/plans/2026-11-change-on-blur-user-provenance.md`, "the provenance PR", which lands first: umbrella
e1). Rev 1 = `cbee12e2`, rev 2 = `1477feef`, rev 3 = `36bcc114`, rev 4 = `90d2c4d2`. Changelog at the end.

**Measured defect.** `crate::re_render` (`crates/shell/elidex-shell/src/lib.rs:559`) ran the "update the rendering" step-17
focus fixup `reconcile_focus` (`crates/dom/elidex-dom-api/src/focus/sot.rs:178`) only when `session.flush` returned
records. VM-native mutations write the `EcsDom` directly and produce none, so `d.hidden = true` on the focused element's
ancestor left it focused (probe `f740a890`, now `crates/shell/elidex-shell/src/content_focus_fixup_tests.rs`). The branch
removed the gate (`0e0d7c93`); `/code-review` then showed that the ungated call is a **silent** reset (no `change` /
`blur` / `focusout`, the holder's change-on-blur snapshot never consumed), which for VM-native mutations is a regression
against `origin/main` — there the holder stayed focused and a later user blur still fired `change`.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

All `file:line` below are at branch HEAD `68c17ccf` (= `origin/main` `d7d34035` + `2163f113` probe, `0e0d7c93` gate
removal, `68c17ccf` /simplify). Two bases are used for red-first runs (§6): **base A** = `d7d34035` (gated fixup) and
**base B** = `68c17ccf` (ungated silent fixup). Re-run: `git -C <wt> log --oneline d7d34035..68c17ccf` → 3 commits.
Revs 2–5 cite the same coordinates (`git diff --stat 68c17ccf 90d2c4d2` → only this memo). After the rebase both bases
are re-anchored: base A = the `origin/main` this branch rebases onto, base B = the rebased `68c17ccf`. **Re-base note:** this branch rebases onto `origin/main`
after the provenance PR lands (umbrella e1). That PR edits `content/event_handlers.rs` and `content/focus.rs` in place
(no split, PM 264 (M2)), so their cites below are re-measured at the rebased head before implementation.

### §0.2 Premises re-measured

| ID | Premise | Measurement (re-runnable) |
|---|---|---|
| P1 | `reconcile_focus` has exactly one non-test caller, `lib.rs:635` | `git grep -n 'reconcile_focus(' -- crates` → `lib.rs:635` + `focus/tests.rs` lines 72/731/740/751/758/767/780 (7 test sites, 3 test fns), verified 2026-11-03 |
| P2 | The iframe paths do not call `reconcile_focus`; they reach the fixup only through `crate::re_render` | `content/iframe/render.rs:38` (`re_render_all_iframes`, gated on `needs_render`), `content/iframe/thread.rs:107` (`iframe_thread_main`, per frame) |
| P3 | `reconcile_focused_iframe` is a different function (the `focused_iframe` side-field reconcile), defined in `content/event_handlers.rs:774`, called from `content/mod.rs:351` (pass 1) and `:423` (pass 2) | `git grep -n 'reconcile_focused_iframe' -- crates/shell` |
| P4 | Every other `crate::re_render` caller has no `ContentState`: `app/events.rs:83`, `:183`; `app/inline.rs:102`, `:132`, `:215`; `app/navigation.rs:94`, `:233`, `:466`; `content/iframe/thread.rs:107`; `app_test_support.rs:62` | `git grep -n 'crate::re_render(' -- crates/shell` (10 call sites besides `content/mod.rs:357`, verified 2026-11-03) |
| P5 | The canonical §6.6.4 transition exists: `focusing_steps` (`focus/update_steps.rs:106`), `unfocusing_steps` (`:165`), private `focus_update_steps` (`:175`), sink trait `FocusEventSink`; the shell sink `ShellFocusSink` (`content/focus.rs:108`, impl `:112-155`) dispatches `blur`+`focusout` / `focus`+`focusin` and consumes the snapshot in `dispatch_change_on_blur` (`:177`) | read at HEAD |
| P6 | `elidex_form::AncestorCache` (`ancestor_cache.rs`, `HashMap<Entity, Option<Entity>>`) is cleared only when `session.flush` returned records (`lib.rs:599-601`); its readers are `toggle_radio` (`radio.rs:77`), `radio_arrow_navigate` (`:204`) and the group scan (`:132`), whose `None` arm already calls `resolve_form_owner` (`:134`); the shell holds it as `PipelineResult.ancestor_cache` (`lib.rs:255`; initialised `pipeline.rs:632`, `:715`, `:811`, `:911`) and passes it at `content/event_handlers.rs:139`, `:609`, `:654`, `:660` | `git grep -n 'AncestorCache\|ancestor_cache\|get_form_owner' -- crates` |
| P8 | VM `focus()` gates on `is_focusable && is_in_document` (`crates/script/elidex-js/src/vm/host/html_element_proto.rs:302-305`) and writes the bit directly (`set_focus_bit`), seeding the snapshot; VM `blur()` is bit-level (`:351`, `focus::blur`); `is_focusable` (`focus/predicate.rs:205`) does not enforce C2 (`:176-180`) nor fieldset-inherited disabled (`:182-185`) | read at HEAD |
| P9 | The change-on-blur snapshot exists only on text controls: `record_focus_snapshot` stores it iff `kind.is_text_control()` and removes it otherwise (`crates/dom/elidex-form/src/focus_snapshot.rs:28-45`); `is_text_control` = text / password / textarea / email / url / tel / search (`crates/dom/elidex-form-core/src/lib.rs:104-115`, no `Hidden`, no button kinds); a `type` flip to a non-text kind clears it at the `set_attribute` chokepoint (`crates/dom/elidex-form/src/reconciler.rs:155-162`, cell `reconciler_tests.rs` `e2c_type_flip_to_non_text_while_focused_clears_focus_snapshot`); VM `type =` reaches it through `attr_set` (`html_input_proto.rs:848-853`) | read at HEAD |
| P10 | At HEAD, `dispatch_change_on_blur` fires `change` iff a snapshot exists and `fcs.value() != snapshot` (`content/focus.rs:177-193`): it does not know *who* changed the value, so a script `value` write to a focused text control makes the next blur fire `change`. The provenance PR replaces this predicate with "a user value change since focus **and** value ≠ focus-time value", computed by `elidex_form::take_change_on_blur` (NEW) (the provenance PR's D-1, umbrella X1), which the shell sink's `commit_change_on_blur` calls before only dispatching; this PR's fixup reaches it through that sink method | read at HEAD; the provenance memo §0.2 P4 |

### §0.3 Premise errors (in the brief)

- **E1** — "`reconcile_focused_iframe` in content/mod.rs": it is defined in `content/event_handlers.rs:774`; `content/mod.rs`
  only calls it (`:351`, `:423`). No consequence for the design.
- **E2** — "focusing steps for the viewport vs unfocusing steps — choose": the choice is forced by the spec. The
  unfocusing steps' step 6 ("If old focus target is not a focusable area, then return") returns for exactly the target
  step 17 handles, so they cannot implement the fixup. elidex's `unfocusing_steps` omits step 6 (its guard is step 5
  only, `update_steps.rs:165-171`), so calling it would "work" only through that omission — and would break silently when
  A2c adds step 6 for VM `blur()`. Pre-existing omission; recorded for A2c (§8).
- **E3** — The literal step-17 chain fires one event the brief does not list: `focus` at the **Window**. Derivation in
  §4.1 (S7); decided as a hand-off to A3 in D-9 (PM 252 (M1)). Reading taken (unmeasured): the old chain of the
  non-focusable holder is [holder] (`#focus-chain` is defined for a focusable-area subject; its loop breaks at a
  non-focusable element), so step 1 pops nothing and the new chain keeps its Document entry; the reading [holder,
  Document] would pop it and fire no Window `focus`.
- **E4** — Item 2 asks to "move it to a version-delta gate like the consumers [that] moved to
  `ContentState::last_render_dom_version`". The ideal is not a gate at `re_render`: the cache's readers run between
  frames (`toggle_radio_with_events`, `event_handlers.rs:650-663`, reads the cache after `set_focus`'s listeners ran), so
  any render-time invalidation leaves that window stale. D-8 removes the cache instead (292 (1)).
- **E5 (rev 1's own, R3 / §3 last row)** — rev 1 said "a text→hidden `type` flip after an edit fires `change`". It does
  not: the flip clears the snapshot at the `set_attribute` chokepoint (P9), and the `Hidden` state and button kinds never
  carry one, so the "change event applies" conjunct of focus update step 2.1 holds structurally. The step-2.1 conjunct
  that is **not** met is a different one, "the user has changed the element's value" (P10): a script write of `value`
  while focused makes the next blur fire `change`. 252 (M5)'s question is answered for that conjunct (§4.5); PM 260 moved
the fix to the provenance PR, which lands first.
- **E6 (scout Q5)** — "a new harness is needed for C16": the existing OOP harness `run_oop_and_collect_posts`
  (`content_test_support.rs:340`) collects the child's `postMessage`s, and a child `blur` listener can post what it
  observed, so C16 is built on it (D-12). What does not exist is an OOP test that inspects the child `EcsDom`; C16 does
  not need one.
- **E7 (rev 1's own, D-10)** — commit (1) deleted `reconcile_focus` while its caller `lib.rs:635` stays until commit (2),
  so commit (1) would not build. Rev 2 moves the deletion into the shell commit.

### §0.4 Decisions (this PR)

- **D-1 The fixup is the focusing steps for the viewport** (248 (1)). Step mapping in §4.1. Not the unfocusing steps (E2).
- **D-2 One function moves focus to the viewport.** `focusing_steps_for_viewport` (NEW, private in `update_steps.rs`)
  implements focusing steps 1–8 for the viewport target in the single-element model (steps 1–4 vacuous, step 5 = "no
  holder → return", steps 6–8 = `focus_update_steps(sink, Some(old), None)`). Two callers: `focus_fixup_steps` (NEW, pub,
  the step-17 decision) and `unfocusing_steps` step 8, whose system-focus branch is "run the focusing steps for
  topDocument's viewport" (its other branch, "run the focus update steps given old chain, an empty list, and null", is
  not distinguished: elidex does not model system focus; today both are the inlined `focus_update_steps(sink,
  Some(old_target), None)` at `update_steps.rs:170`). One-issue-one-way: "focus the viewport" has one implementation.
- **D-3 The predicate is `is_focusable`, unchanged** (248 (6)). It is the VM `focus()` gate's predicate (P8); a fixup that
  rejected more than the writer accepts would focus-then-reset every frame, now with events. C2 (`is_focusable_area`) and
  the shell's fieldset overlay (the shell `is_focusable`, `content/focus.rs:204`) are excluded for that reason; each
  has its slot (§7).
- **D-4 `reconcile_focus` is deleted.** Its one non-test caller (P1) is replaced; its test uses move to
  `update_steps.rs`'s test module as `focus_fixup_steps` cells using the existing `TestSink` (C14): two whole fns and the
  fixup half of a third (§5.2). `raw_focus_holder`
  (`sot.rs:16`) loses its second reader and stays for `blur` (`sot.rs:146`) and `current_focus`. The iframe paths need no
  change (P2): each pipeline's `crate::re_render` runs its own fixup. `reconcile_focused_iframe` (P3) is unchanged in code;
  its two call-site comments are corrected (F5, §5.1).
- **D-5 Shell entry.** `content::focus::focus_fixup` (NEW, `pub(crate)`, `content/focus.rs`) constructs
  `ShellFocusSink { pipeline }` and calls `elidex_dom_api::focus::focus_fixup_steps` (NEW). `crate::re_render` calls it. The sink
  needs only `&mut PipelineResult`, so every caller in P4 gets the same behaviour; no `ContentState` dependency.
- **D-6 Position** (248 (4)). **This PR does not move the position** (the call stays at `lib.rs:635`); the move goes
  with O1's leg (§8).
- **D-7 No second settle after the fixup.** The fixup's listeners are VM callbacks; VM-native writes reach the `EcsDom`
  directly and settle MutationObserver / CE at the VM's own checkpoints (`lib.rs:566-571` comment). **Evidence (Q1, NO):**
  no path reachable from a VM listener writes `SessionCore`'s pending buffer. The production `record_mutation` callers are
  `markup.rs:56` (`SetInnerHtml`) and `:117` (`InsertAdjacentHtml`), both `DomApiHandler`s the VM does not invoke — VM
  `innerHTML` calls `apply_set_inner_html` directly (`vm/host/dom_inner_html.rs:144-146`), and its record goes to the
  §4.3 notify queue, not the pending buffer; the other callers in `elidex-script-session/src/session.rs` (`:209`–`:348`)
  are in its `mod tests` (`:192`). Re-run: `git grep -n 'record_mutation(' -- crates`. A re-flush after the fixup would be
  code no cell can turn red.
- **D-8 The record-gated form-owner cache is deleted** (248 (2), E4; 292 (1)). `AncestorCache` (P6), its `lib.rs:598-601`
  gate and the `PipelineResult.ancestor_cache` field are removed; every radio form-owner lookup calls `resolve_form_owner`
  (§4.2). Re-keying the cache was rejected: its validity key was the defect (rev 4's review). The form-owner
  representation is FF-4's (§8.1). No new cell: the defect is the cache's existence (§9 item 3); `radio.rs`'s existing
  tests cover the grouping, uncached.
- **D-9 The Window `focus` event goes to A3** (E3; 252 (M1)). **Defect: elidex does not fire the Window `focus` event
  the spec chain gives** — for the fixup, focus update step 4 reaches the new chain's Document entry, whose focus event
  target is its Window; elidex fires nothing there. Cause: the single-element model (A2a, `update_steps.rs:14-23`) fires
  events only at elements. **Home:** structural slot `#11-canonical-focus-update-steps`, part A3 (the multi-entry chain is A3's per
  `memory/project_canonical-focus-update-steps.md:67`, "chain + step-1 pop = A3 (cross-frame)"; A3's scope list `:35-37`).
  A3's recorded scope names the chain but not the Window events, so §8 carries a record for the PM to append. This PR
  fires `change` (when step 2.1 holds, by the provenance PR's predicate) and `blur` at the holder, related target null — the literal spec's events at
  the element — plus the sink's `focusout`, as on every elidex blur.
- **D-10 Commits** (E7; 292 (3)), on the provenance-rebased base: (1) dom-api: `focusing_steps_for_viewport`,
  `focus_fixup_steps`, `unfocusing_steps` step 8 re-route, new and copied cells (the `reconcile_focus` cells stay until
  (3)); (2) shell: cell C19 alone; (3) shell + dom-api: `focus_fixup`, `lib.rs` call, `reconcile_focus` deletion with its
  relocated cells, comments F4/F5, the other shell cells; (4) form + shell: `AncestorCache` deletion (§4.2). C19 is
  observed red at (2) and green from (3) (recorded in the PR body); every other cell is green at every commit, and each
  commit builds.
- **D-11 (withdrawn in rev 3, PM 260)** — the change-on-blur user-change provenance moved to its own PR,
  `docs/plans/2026-11-change-on-blur-user-provenance.md`, which lands before this PR. This PR has no code for it. It keeps one ordering pin cell (C19: the blur/focusout assertions) and the reachability argument that made the order necessary
  (§4.5). The absence of a script-only `change` is guarded on the provenance side (its `take_change_on_blur` cells and MG1; docs/plans/2026-11-change-on-blur-user-provenance.md). The ID is kept so D-12 / D-13 keep their numbers.
- **D-12 C16 is in scope, on the existing OOP harness** (Q5, E6): an OOP child focuses an input, hides its ancestor in
  script, and its `blur` listener posts what it saw; the parent-side harness collects the posts (§6).
- **D-13 C17 observes the style consequence, not the caret** (Q3). No caret display-item probe exists in the shell
  tests, so C17 asserts what the same frame's style pass shows: an author `:focus` rule (`:focus` matches the `FOCUS`
  bit, `elidex-css` `selector/matching.rs:349`) no longer applies to the blurred control's `ComputedStyle` after the
  `re_render` that ran the fixup. The caret follows the same bit at display-list build (`elidex-render`
  `builder/form.rs:98-112` takes `is_focused`); no cell observes it.

### §0.5 Scout answers to rev 1's Q1–Q6 (2026-11-03; each re-read at `68c17ccf` where rev 2 relies on it)

| Q | Answer | Where it lands |
|---|---|---|
| Q1 | NO — no VM-reachable path writes the session's pending buffer | D-7 evidence |
| Q2 | YES — a microtask checkpoint after each listener (`elidex-script-session/src/event_dispatch/mod.rs:512-513`) and `drain_reactions` after the walk (`:571`); `fire_focus_event` → `dispatch_focus_event_with_related` (`content/focus.rs:136-170`) → `PipelineResult::dispatch_event` (`lib.rs:293`) | C8's premise holds |
| Q3 | NO / NO — no shell caret display-item probe; no path ends or cancels an IME composition on blur (`content/ime.rs:157-159`, `ImeKind::Enabled \| Disabled` arms are no-ops) | D-13 (C17), I10 restated as a pre-existing gap (§8) |
| Q4 | Unreachable today — `contentDocument` / `contentWindow` return `null` (`vm/host/html_iframe_proto.rs:300-311`, slot `#11-windowproxy-browsing-context`); `needs_render` setters are timers (`content/iframe/render.rs:60-68`), blur (`content/event_handlers.rs:770-776`) and event forwarding (`:760`, `:884`, `:945`) | §4.4 S9 |
| Q5 | No focus-inspecting OOP test; harness `run_oop_and_collect_posts` exists | D-12, E6 |
| Q6 | YES, structurally — all five constructors take the parser's `create_document_root` | unused since rev 5 (D-8 removes the cache) |

## §1. Shape and verdict

Edge-dense by CLAUDE.md's rule (§2 has ten coupled invariants spanning event dispatch, script re-entry, iframe passes,
form state and render position), so plan-review is mandatory. **Single PR**, a terminal slice under the umbrella (base
case (c)): the algorithm is the approved A2a seam (`#357`), reused; the parts that would widen the PR are homed elsewhere
(§7). Breadth K = 1 spec, M = 12 rows (§3) → preflight "single PR scope". The form-owner cache (D-8) is a different
concept found by 248 (2)'s sweep and kept in this PR by that decision; it is a separate commit (D-10). The
provenance PR and the order against it are the umbrella's (§1.2, e1); this PR keeps only the pin C19.

## §2. Coupled invariants

- **I1 Ungated trigger** — every `crate::re_render` (all callers, P4) runs `focus_fixup_steps` once, regardless of
  `session.flush`'s output.
- **I2 Canonical transition** — the fixup moves focus only through `focus_update_steps(old, None)` via a sink: step 2.1
  `change` while `old` is still designated, then the bit cleared, then `blur` + `focusout` at `old`, related null. Step
  2.1 is judged by `elidex_form::take_change_on_blur` (P10), which the sink calls on every blur path, the fixup's
  included; the sink only dispatches (umbrella X1), so no fixup-specific invariant exists for it. The absence of a script-only `change` is guarded on the provenance side (its `take_change_on_blur` cells and MG1; docs/plans/2026-11-change-on-blur-user-provenance.md).
- **I3 Snapshot consumed once** — the holder's `FocusValueSnapshot` is removed by `elidex_form::take_change_on_blur`,
  called from the sink's `commit_change_on_blur`, exactly once per transition (umbrella X2).
- **I4 Script re-entry inside `re_render`** — `change` / `blur` / `focusout` listeners run after the settle and before
  style; their VM-native writes are visible to this frame's style, layout and display list; MutationObserver / CE for
  them settle at the VM checkpoint after each callback (Q2); a reentrant `focus()` wins (`update_steps.rs:204`, `:223`).
- **I5 Observers see the pre-fixup holder** — MutationObserver callbacks for mutations made before the frame (VM-native:
  delivered at the script's own microtask checkpoint; external: `deliver_records_and_drain`, `lib.rs:580`) run before
  the fixup and read `activeElement` = the holder. Kept (the after-settle position, i.e. after the first flush, is pinned by C3 / MG6). Animation state of entities deleted by a fixup listener is pruned on the next frame, because `prune_dead_animation_entities` (lib.rs:606) runs before the fixup (lib.rs:635); this PR keeps HEAD's order.
- **I6 Per-document fixup** — each pipeline's fixup reads and writes only its own `EcsDom` and dispatches into its own
  runtime: the parent in `ContentState::re_render` (`content/mod.rs:357`), in-process children in
  `re_render_all_iframes` (`:355`, only when `needs_render`), OOP children per frame (`iframe/thread.rs:107`). Parent pass 2
  (`:423`) runs after the parent fixup and blurs the child of a no-longer-focused `<iframe>`.
- **I7 Form-owner answers are computed at read time** — no radio form-owner lookup reads a cached answer (D-8).
- **I8 One predicate for writer and fixup** — the fixup resets exactly what the VM `focus()` gate would refuse
  (`is_focusable`, D-3); the shell UA writer (`focus_target`, `content/focus.rs:52`) accepts a subset of that.
- **I9 Position** — before style; ResizeObserver delivery (`content/mod.rs:433`) after it (not spec order — obligation,
  §8).
- **I10 Blurred control's UI state** — the cleared `FOCUS` bit is read by this frame's style pass (`:focus`, C17) and
  display-list build (caret, `is_focused`; no cell, D-13). IME composition is **not** ended on blur — a stated
  pre-existing gap, not an invariant this PR keeps. Defect: when focus leaves a text control during an active IME
  composition (any blur: UA, VM, or the step-17 fixup), elidex neither commits nor cancels it — no `compositionend`
  fires, `FormControlState.composition_text` keeps the preedit, and the display-list build still splices it into the
  control's text (`elidex-render` `builder/form.rs:226-236`), because the shell has no blur-time composition path
  (`content/ime.rs:157-159`, Q3). The fixup makes this blur reachable from script, but every UA blur has the same gap at
  `origin/main`. Home: FF-3 (§8.1, which records the search for an existing slot).

Pairs:
- **I1 × I2** — ungated + event-firing: an idle frame must fire nothing. `focus_fixup_steps` returns before the
  transition when there is no holder or the holder is focusable (C4, C14 (b)/(c)).
- **I1 × I8** — running every frame turns any writer/fixup predicate mismatch into a per-frame focus-then-reset loop with
  events; I8 removes it for the VM writer (C18 pins the C2 case).
- **I2 × I3** — step 2.1 *is* the consume; base B's silent reset skipped it, leaving the snapshot for a later unrelated
  blur (C6).
- **I2 × I4** — the transition's listener calls are the re-entry points; reentrant-wins is checked after step 2.1 and
  after step 2.4, so a listener's `other.focus()` stands and the outer transition designates nothing (new = viewport) (C9).
- **I2 × I5** — the frame's earlier MutationObserver deliveries precede the fixup (I5); records from the fixup's own
  listeners are delivered at their own checkpoint, after the fixup, and see the post-fixup `activeElement` (C8).
- **I2 × I6** — a child pipeline's sink dispatches into the child runtime; the parent's fixup blurs the `<iframe>` element
  in the parent; the child is blurred by pass 2 through the child's own canonical `unfocusing_steps` (`blur_iframe_focus`
  → `blur_current`, `event_handlers.rs:755-762`) (C11).
- **I2 × I8** — a listener's reentrant `focus()` goes through VM `focus()` (bit-only, no events): the reentrant target wins
  without `focus` / `focusin`, and if it fires from `change`, `old` gets no `blur` (`update_steps.rs:184-203`). Pre-existing
  (A2c), not widened: base A/B fire nothing at all.
- **I2 × I10** — the bit is cleared at step 2.2 (before `blur`), which is before style resolution and
  `build_display_list_with_scroll` (`lib.rs:705`), so `:focus` stops matching and the caret disappears this frame, as at
  base B (C17). A live IME composition survives the transition (the gap above).
- **I3 × I4** — a `change` listener writing `value` cannot cause a second `change`: the snapshot is already taken.
- **I4 × I6** — listener mutations in the parent land before `ContentState`'s version snapshot (`content/mod.rs:367-372`),
  so `focusable_cache`, the iframe rescan (`:441`) and `event_loop.rs:424-432`'s render signal see them this frame.
- **I4 × I9** — listener writes are styled this frame only because the fixup is before style; after O1's move they show
  next frame (one-frame lag, decided by PM 242 (1); O1, §8).
- **I4 × FU** — a listener's script-time style read (`getComputedStyle`, once FU exists) reaches FU's forced update, which
  must not include step 17 (CV umbrella FU-5, `docs/plans/2026-10-content-visibility.md:239-241`); no recursion.
- **I5 × I9** — spec step 16 (ResizeObserver) precedes step 17, so RO callbacks should see the pre-fixup holder; at HEAD
  and after this PR they see the post-fixup one (same as base A/B). Obligation O1.
- **I6 × I1** — an in-process child runs its fixup only on frames where `needs_render` is set (`iframe/render.rs:37`); a
  child mutation that does not set it would wait for the next flagged frame — not reachable today (§4.4 row S9, Q4).
- **I6 × I8** — an `<iframe>` is focusable by its tab-index default; `hidden` on it (or an ancestor) makes it
  non-focusable by the same predicate (C11).
- **I3 × I6**, **I5 × I6**, **I4 × I7**, **I7 × I1**, **I7 × I2**, **I7 × I3**, **I8 × I9**, **I10 × I6** — none beyond the shared
  pipeline (each pipeline's sink reads its own snapshots).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 17 | focused area of doc not a focusable area → focusing steps for doc's viewport; focusable or no holder → nothing | `lib.rs` `re_render` → `content::focus::focus_fixup` (NEW) → `focus_fixup_steps` (NEW) | ✓ | yes |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 17 | "set the navigation API's focus changed during ongoing navigation to false" | unmodelled (no Navigation API; `update_steps.rs:227-230` records the same for step 4.1.1) | ✓ | no |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 17 notes | "will usually fire blur events, and possibly change events"; removal = the synchronous fixup, no events | events: I2; removal: `EcsDom::fire_after_remove` unchanged | ✓ | yes |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* steps 16 → 17 | ResizeObserver broadcast before the fixup | not met: RO delivered after (`content/mod.rs:433`); obligation O1 | ✗ | no |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 2 | docs order: a container document before its children, each step over all docs | not met: elidex runs a whole `re_render` per pipeline, children first (`content/mod.rs:355-357`); unchanged, FF-1 (§8.1, 252 (M2)) | ✗ | no |
| WHATWG HTML §6.6.4 Processing model | focusing steps 1–5, target = viewport | 1 viewport is a focusable area; 2 n/a; 3 not a navigable container; 4 DOM anchor = Document, `inert` unmodelled; 5 viewport already focused (no holder) → return | `focusing_steps_for_viewport` (NEW) | ✓ | no |
| WHATWG HTML §6.6.4 Processing model | focusing steps 6–8; focus update steps step 1 | old chain [holder] (holder not focusable → chain stops), new chain [viewport, Document]; no common tail; single-element model → `focus_update_steps(old, None)` | `update_steps.rs:175` (unchanged) | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1–2.4 | user-edited text `input` → `change` (step 2.1's subject is an `input` element; a `textarea` also carries a snapshot, P9, unchanged here; snapshot consume; "the user has changed the element's value" = the provenance PR's predicate, a script-only change fires none, pinned on the fixup path by C19); blur target = holder; related blur target null (last new entry is a Document) | `ShellFocusSink::commit_change_on_blur` (calls `elidex_form::take_change_on_blur`, P10) / `fire_focus_event` (`content/focus.rs:121`, `:136`), unchanged here | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 4 | Document entry: not a focusable area, focus event target = its Window → `focus` at Window (not fired, D-9); viewport entry: designated (bit cleared), no event; reentrant-wins gate | `update_steps.rs:220-245`; the Window event → A3 | ✗ | yes |
| WHATWG HTML §6.6.4 Processing model | unfocusing steps steps 5, 6, 8 | 6 "not a focusable area → return" (why D-1 rejects them; omitted in elidex, E2); 8 both branches (system focus or not) as one (D-2) | `unfocusing_steps` step 8 → `focusing_steps_for_viewport` (NEW) | ✗ | yes |
| WHATWG HTML §6.6.2 Data model | focusable area criteria; focus chain | predicate = `is_focusable`: C1, C3 (direct `disabled` only), C5 (attribute-reachable); C2, fieldset-inherited C3, CSS C5 not enforced | `focus/predicate.rs:205` (unchanged) | ✗ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | `change` when the control loses focus | change applies except buttons and Hidden-state inputs: structural — only text controls carry a snapshot, a `type` flip to a non-text kind clears it (P9, E5); pinned by the existing reconciler cell (P9) | `record_focus_snapshot` / reconciler `type` arm (unchanged) | ✓ | yes |

Numbers and titles: `.claude/tools/webref coverage-map html 8.1.7.3 html 6.6.4 html 6.6.2 html 4.10.5.5` (§8.1.7.3
Processing model `#event-loop-processing-model`; §6.6.4 Processing model `#focus-processing-model`; §6.6.2 Data model
`#data-model`; §4.10.5.5 Common event behaviors `#common-input-element-events`); `webref dfn html 'update the rendering'`
→ §8.1.7.3; `'focusing steps'`, `'unfocusing steps'`, `'focus update steps'` → §6.6.4; `'focusable area'`, `'focus chain'`
→ §6.6.2. Step texts: `.claude/tools/webref body html update-the-rendering`, `focusing-steps`, `focus-update-steps`,
`unfocusing-steps`, `focus-chain`, `common-input-element-events` (2026-11-03). UI Events' focus-event order is the sink's,
unchanged, and not cited here. Breadth: K = 1, M = 12 (rev 2's thirteenth row, §4.10.5.5 "not fired … by scripts", is the
provenance PR's; its fixup-path consequence is folded into the step 2.1–2.4 row).

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: any script write that changes the holder's focusability (`hidden` on
it or an ancestor, `disabled`, `type`, `tabindex`, `contenteditable`, `href` removal) — now fires author listeners
(`change`, `blur`, `focusout`) from inside `re_render`; those listeners can mutate the DOM, move focus, and register
observers (I4). User input: the edited value, judged by `elidex_form::take_change_on_blur` (P10). Radio `name` / `form` attributes and
tree moves reach `resolve_form_owner` on every lookup (D-8). Every listener runs on the renderer main thread inside the existing `with_bound`
bracket discipline (`PipelineResult::dispatch_event`, `lib.rs:293`); `crate::re_render` is never entered inside a bracket
(its `session.flush` would alias, `lib.rs:560-563`), so the dispatch nests nothing. Adjacent pre-existing surface:
the radio group scan includes radios outside the document tree (`radio.rs:118-123` has no same-tree filter, against
§4.10.5.1.16 `#radio-button-group`'s "Both a and b are in the same tree") — not widened; FF-2 (§8.1, 252 (M3) (b)). User edits mark the
change-on-blur snapshot in the provenance PR's funnel, not here.

## §4. Mechanism

### §4.1 The fixup — HEAD → this PR

`focus_fixup_steps` (NEW, `update_steps.rs`):

```rust
pub fn focus_fixup_steps(sink: &mut dyn FocusEventSink) {
    let doc = sink.document();
    let Some(holder) = current_focus(sink.dom(), doc) else { return };
    if is_focusable(sink.dom(), holder) { return; }
    focusing_steps_for_viewport(sink);
}
```

`current_focus` = `raw_focus_holder` + `is_in_document`, i.e. HEAD's `reconcile_focus` test (`sot.rs:184-188`) exactly.

Step mapping (S1–S9; spec text from §3's commands):

| # | Spec | elidex |
|---|---|---|
| S1 | step 17: focused area not a focusable area | `focus_fixup_steps`'s two early returns |
| S2 | focusing step 1 (viewport is a focusable area) | vacuous |
| S3 | focusing steps 2–4 (null target, navigable container, inert) | vacuous / `inert` unmodelled (as `update_steps.rs:126-128`) |
| S4 | focusing step 5 (already the focused area) | `focusing_steps_for_viewport`: `current_focus` None → return |
| S5 | focusing steps 6–8; focus update step 1 | `focus_update_steps(sink, Some(old), None)` |
| S6 | focus update step 2 (change, blur at holder, related null) | `update_steps.rs:177-217`; `new = None` → related null |
| S7 | focus update step 4 for the Document entry (`focus` at Window) | not fired (D-9) |
| S8 | focus update step 4 for the viewport entry (designate) | the bit cleared at step 2.2 (A2a browser-parity ordering, `update_steps.rs:208-215`) |
| S9 | step 17's navigation-API flag | unmodelled |

`unfocusing_steps` (`update_steps.rs:165-171`): its body after the step-5 guard becomes `focusing_steps_for_viewport(sink)`;
behaviour unchanged (the guard already established `current_focus == Some(old_target)`), pinned by the existing unfocusing
cells in `update_steps.rs`'s test module.

`crate::re_render`: the call at `lib.rs:635` becomes `crate::content::focus::focus_fixup(result);` (D-6). The comment at `lib.rs:608-634` is rewritten: canonical steps, events listed, the step-17 note, the position obligation
(O1) and I5.

### §4.2 The form-owner cache — HEAD → this PR

D-8. `elidex-form`: `ancestor_cache.rs` is deleted with its `lib.rs` module and re-export (`:17`, `:45`); in `radio.rs`
the `cache` parameter of `toggle_radio` (`:59`), `find_radio_group_impl` / `find_radio_group` / `find_radio_group_scoped`
(`:114`, `:155`, `:171`) and `radio_arrow_navigate` (`:195`) is removed, their lookups call `resolve_form_owner`, the doc
of `resolve_form_owner_public` (`:14`, "used by `AncestorCache`") names its remaining caller (`submit.rs:301`), and the
test sites drop `AncestorCache::new()` (`git grep -n 'AncestorCache::new' -- crates/dom/elidex-form/src/radio.rs` → 10), the `&mut cache` arguments, and the `find_radio_group(&dom, "g1", None)` call at `radio.rs:296`; the stale docs at `radio.rs:108-109` and `:118-119` are updated (enumerate: `git grep -n 'cache' -- crates/dom/elidex-form/src/radio.rs`).
Shell: the `lib.rs:255` field, the `lib.rs:598-601` gate, the four `pipeline.rs` initialisers and the four
`content/event_handlers.rs` arguments (P6) are deleted. After D-10 commit (4) the `mutation_records.extend(follow_up)` (`lib.rs:588`) is deleted too, with the `let mut` at `:576`, which only that gate read.

### §4.3 ECS-native and Layering checks

- The step-17 decision and the viewport transition are engine-independent (`elidex-dom-api`); the shell contributes only
  the sink (event dispatch + `elidex-form` snapshot), as A2a set up. `vm/host/` is untouched.
- No new side-store; one is removed (D-8). The FOCUS bit stays the SoT; no new version counter.
- The form owner as ECS state (a component written by *reset the form owner*) is FF-4 (§8.1): plan-review Axis 2 judged
  it out of this PR (252 (M3) (a), plan-review Axis 2 (PM 292)), since it is a mutation-time system across three reader families.

### §4.4 Concept sweep — "a consumer that depends on DOM changes but is gated on session records"

Scope: `crate::re_render` (`lib.rs:559-711`), the functions it calls in the same pass, and `ContentState::re_render`
(`content/mod.rs:313-460`). Method: every conditional in those bodies read for its gate's subject; plus
`git grep -n 'mutation_records\|records.is_empty()' -- crates/shell/elidex-shell/src`.

| # | Site | Gate | Disposition |
|---|---|---|---|
| S1 | `lib.rs:579-596` `deliver_records_and_drain` + CE stabilization loop | flush records | **correct** — the records are the subject (external-record MO/CE delivery); VM-native records deliver in the VM |
| S2 | `lib.rs:598-601` `ancestor_cache.invalidate_all()` | flush records | **hit (F1)** — the record-gated cache is removed (D-8) |
| S3 | `lib.rs:635` focus fixup | (gate removed by `0e0d7c93`) | **hit (the slot)** — D-1/D-5 |
| S4 | `lib.rs:606` `prune_dead_animation_entities` | none | correct (unconditional, comment `:603-605`) |
| S5 | `lib.rs:655-665` `collect_document_stylesheets` | per-owner version | correct (version-compared, DOM-as-truth) |
| S6 | `lib.rs:677-710` keyframes, `resolve_with_mode`, transitions, animations, layout, display list | none | correct (unconditional) |
| S7 | `content/mod.rs:367-372` version snapshot → `focusable_cache` (`:406-408`), `rescan_iframes_by_diff` (`:440-444`) | version delta | correct (already moved, the `re_render` doc's claim holds) |
| S8 | `content/mod.rs:351`, `:423` `reconcile_focused_iframe` | none (reads the FOCUS bit) | correct; comments stale (F5) |
| S9 | `content/mod.rs:355` → `iframe/render.rs:37` in-process child `re_render` | `needs_render` flag | **adjacent class, not this concept** (a writer-set dirty flag, not session records). **Not reachable today** (Q4): every path that runs child script sets the flag (timers `iframe/render.rs:60-68`, blur `event_handlers.rs:770-776`, event forwarding `:760`, `:884`, `:945`), and a parent script cannot reach the child's `EcsDom` (`contentDocument` / `contentWindow` are `null` stubs, `html_iframe_proto.rs:300-311`). Re-check when `#11-windowproxy-browsing-context` lands; no slot (252 (M4)); the re-check pointer goes into that slot (record R-S9, §8.2) |
| S10 | `content/mod.rs:447` `check_lazy_iframes`, `:433` `deliver_layout_observations` | none | correct |
| S11 | `lib.rs:545-558` `re_render` doc ("ancestor-cache invalidation" listed as an internal record consumer) | — | doc updated with S2 |
| S12 | `pipeline.rs:147-170` build-time record delivery | flush records | correct (delivery; outside `re_render`, listed for completeness) |

Hits: S2, S3 (fixed here); adjacent S9 (not reachable, §8). No other record-gated DOM consumer.

### §4.5 Change-on-blur provenance — the fixup-path facts (252 (M5), PM 260)

The design is the provenance PR's; the order and its reason are umbrella e1. What stays here is what the fixup path adds:

1. The brief's example (text→hidden after an edit) is not the case: P9 clears the snapshot at the flip, so no `change`
   fires on any path (E5); the existing reconciler cell pins it (P9).
2. The case that is: a focused text control, a script `value` write, then a script-made non-focusability (`disabled`,
   `hidden` on it or an ancestor, `tabindex` removal on a non-control, …). Base A leaves it focused; base B resets it
   silently; this PR on a base without the provenance PR fires `change` at the next rendering update from script alone.
   C19 pins only the ordering facts (the blur/focusout assertions go red at D-10 commit (2) and green from commit (3)). The absence of a script-only `change` is guarded on the provenance side (its `take_change_on_blur` cells and MG1; docs/plans/2026-11-change-on-blur-user-provenance.md).

## §5. Populations and named risks

### §5.1 Docstring and comment edits

- **F4** `sot.rs:152-177` (moved with the deleted function's doc into `focus_fixup_steps`'s doc): "`disabled` lands"
  overclaims — fieldset-inherited disabled is not in the dom-api predicate (`predicate.rs:182-185`); the new doc says
  "a direct `disabled`", names slot `#11-focusable-area-fieldset-inherited-disabled`, and drops "Silent".
- `sot.rs:1-5` module doc and `mod.rs:35-38` module doc: "the asynchronous focusability fixup (`reconcile_focus`)" →
  `focus_fixup_steps` in `update_steps`; `mod.rs:26-29` ("Event dispatch … stays with the caller; these helpers only
  reconcile the `FOCUS` bit") narrowed to `sot`.
- `sot.rs:24-51` (`current_focus` doc), `sot.rs:130-145` (`blur` doc) and `delegate.rs:66`, `:128` name
  `reconcile_focus` → re-pointed.
- `update_steps.rs:14-23` ("For a single document this is exact") gains the fixup's Document entry (S7, D-9);
  `:184-203` ("unreachable on this path in A2a: the live shell runs boa") → the VM is the shell's engine and a fixup
  listener's VM `focus()` reaches this path (I2 × I8).
- **F5** `content/mod.rs:310-312` (`ContentState::re_render` doc: "Invalidates `focusable_cache` when mutations affect …
  (childList, tabindex, disabled, etc.)" → "on any document-tree version change"); `:338-350` (pass 1: "the iframe element
  made non-focusable by a *buffered* mutation … can only be reconciled after `crate::re_render` flushes + GCs the bit" →
  any mutation, VM-native included, reconciled by the ungated fixup inside `crate::re_render`); `:410-422` (pass 2: "that
  buffered attribute is only flushed + reconciled … INSIDE `crate::re_render`" → the fixup's canonical transition, which
  also fired `blur` at the `<iframe>` element).
- `lib.rs:545-558` (`re_render` doc, S11) and `lib.rs:608-634` (§4.1).
- `content_focus_fixup_tests.rs:1-3` module doc: `reconcile_focus` → the canonical fixup.

### §5.2 Touch list with line counts (`wc -l` at `68c17ccf`)

| File | Lines | Change | Split rule |
|---|---|---|---|
| `crates/shell/elidex-shell/src/lib.rs` | 731 | call replaced in place, the cache gate and field deleted, comments | < 1000 |
| `crates/shell/elidex-shell/src/content/mod.rs` | 865 | comments only (F5) | < 1000 |
| `crates/shell/elidex-shell/src/content/focus.rs` | 397 | `focus_fixup` (NEW, ~15 lines) | < 1000 |
| `crates/shell/elidex-shell/src/content_focus_fixup_tests.rs` | 97 | F6 + cells C5–C9, C11, C16–C19 (~+300) | ~400, < 1000 |
| `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | 550 | two fns (~40), step-8 re-route, relocated + new cells (~+120) | ~710, < 1000 |
| `crates/dom/elidex-dom-api/src/focus/sot.rs` | 202 | `reconcile_focus` deleted, docs | shrinks |
| `crates/dom/elidex-dom-api/src/focus/mod.rs` | 57 | module doc | — |
| `crates/dom/elidex-dom-api/src/focus/tests.rs` | 999 | 3 fns relocated out (~−75); `current_focus_keeps_holder_until_async_fixup` (`:41`) keeps its first half, its fixup half moves | shrinks; **no growth allowed** (any net add crosses 1000 → standalone split first) |
| `crates/dom/elidex-form/src/ancestor_cache.rs` | 114 | deleted | — |
| `crates/dom/elidex-form/src/lib.rs` | 69 | module + re-export removed | shrinks |
| `crates/dom/elidex-form/src/radio.rs` | 500 | `cache` parameters and test constructions removed (§4.2) | shrinks |
| `crates/shell/elidex-shell/src/pipeline.rs` | 975 | four field initialisers removed | shrinks |
| `crates/shell/elidex-shell/src/content/event_handlers.rs` | 965 | four cache arguments removed (`reconcile_focused_iframe` unchanged) | shrinks |

Re-run: `wc -l` on the paths above. `focus/predicate.rs` (329) is read, not edited; `focus/delegate.rs` gets doc lines only (§5.1).
`elidex-form`'s `focus_snapshot.rs` and the shell's `form_input.rs` are no longer touched (rev 2's D-11 rows went to the
provenance PR). `content/focus.rs`'s line count is re-measured at the rebased head (the provenance PR edits
`dispatch_change_on_blur` there).

### §5.3 Named risks

- **R1 Script now runs inside `re_render`.** Listener cost and listener-driven DOM growth land in the frame. Bounded by
  one transition per pipeline per frame (no loop: step 17 runs once; a listener-created non-focusable holder waits for
  the next frame, as in the spec).
- **R2 Uncached form-owner lookup.** The radio scan (`radio.rs:118-140`) resolves each same-name candidate: an ancestor
  walk, or a world query over forms for a `form` attribute. It runs only on a user radio event (click, Space, arrows),
  never per frame; no figure is given.
- **R3 Change-on-blur provenance** (§4.5; rev 1's "applies" framing withdrawn, E5). Without the provenance PR the fixup
  would fire a script-caused `change` from script alone. The landing order (§8) is the guard; the absence of a script-only `change` is guarded on the provenance side (its `take_change_on_blur` cells and MG1; docs/plans/2026-11-change-on-blur-user-provenance.md). C19 pins only the ordering (D-10). The provenance's own risks (the user-edit funnel, the mixed case) are that
  PR's (its §5.2).
- **R4 F8 rejected — per-frame cost of the ungated fixup.** Per frame: `raw_focus_holder` iterates only archetypes
  carrying `ElementState` and stops at the first FOCUS holder (`sot.rs:16-22`, `find`); then one `is_in_document` walk and
  one `is_focusable` evaluation (its hidden-subtree walk) for that one holder, both capped at `MAX_ANCESTOR_DEPTH`
  (`sot.rs:81`); the full-holder sweep in `set_focus_bit` runs only on a reset. And `re_render` runs only when a render is
  needed (`event_loop.rs:252-447`'s `needs_render`; in-process children on `needs_render`). Base A ran the same work on
  every frame with records. No figure is given (machine-dependent); the argument is the bound above.

## §6. Test and coverage plan

Red-first: each NEW cell is first run on base A and base B (re-anchored after the rebase, §0.1) with the fix absent; the expected
result is in the "A / B" column ("red" = fails as the defect predicts; "green" = pin or control). The PR body records the
observed result per cell and base. Shell cells live in `content_focus_fixup_tests.rs` (fixture helpers `focused_on_b`,
`by_id`, `focused` exist); iframe cells reuse the `<iframe srcdoc>` in-process fixture of
`content/iframe/lifecycle.rs:587-589`; the OOP cell (C16) uses `run_oop_and_collect_posts` (`content_test_support.rs:340`)
and observes through the child's `postMessage`s. Events are observed by listeners writing to a global array read back
with `eval_script`.

**Pin C19 (PM 260; 292 (3)):** observed red at D-10's commit (2) and green from commit (3).

| Cell | Setup → assertion | A / B |
|---|---|---|
| C1 (F6) `vm_native_hidden_ancestor_runs_focus_fixup` | existing; assertion → `current_focus == None` | red / green |
| C2 (F6) `vm_native_disabled_focused_button_runs_focus_fixup` | existing; → `== None` | red / green |
| C3 (F6) `control_session_buffered_hidden_runs_focus_fixup` | existing control; → `== None` | green / green |
| C4 `focusable_focus_survives_idle_rendering_updates` | existing negative; add: no `blur` recorded | green / green |
| C5 (NEW) `focus_fixup_fires_blur_then_focusout_at_holder` | `b` has `blur`/`focusout` listeners recording type, `relatedTarget`, `document.activeElement`; VM `d.hidden = true`; frame → `["blur","focusout"]`, related null, `activeElement === body` in both | red / red |
| C6 (NEW) `focus_fixup_fires_change_for_edited_text_control` | `<input id=b>` focused by UA `set_focus` (seeds snapshot); value changed through the shell user-edit path (a value-changing text key, which after the rebase goes through the provenance PR's `apply_user_edit` funnel; umbrella X3); VM `b.disabled = true`; frame → `["change","blur","focusout"]`, `change` listener sees `activeElement === b`, `b` has no `FocusValueSnapshot` | red / red |
| C7 (NEW) `focus_fixup_listener_mutation_is_rendered_this_frame` | `blur` listener appends `<p id=late>x</p>`; frame → `#late` has a `LayoutBox` after the same `re_render` | red / red |
| C8 (NEW) `focus_fixup_listener_mutation_is_observed` | MutationObserver on `body` (childList); `blur` listener appends; after the frame the callback has run once and saw `activeElement === body` | red / red (Q2) |
| C9 (NEW) `focus_fixup_reentrant_focus_wins` | `blur` listener calls `c.focus()` (`c` focusable); frame → `current_focus == c`, `b` saw exactly one `blur`, the outer transition designated nothing after | red / red |
| C11 (NEW, F7) `vm_native_hidden_focused_iframe_blurs_parent_and_child` | in-process `<iframe srcdoc="<input id=i>">`; focus into it (parent bit on the iframe, `focused_iframe`, child bit on `i`); parent VM `iframe.hidden = true`; `ContentState::re_render` → parent `current_focus == None`, `focused_iframe == None`, child `current_focus == None`, parent `blur` fired at the iframe element | red / green-except-`blur` (B resets silently: the `blur` assertion is red) |
| C14 (NEW + relocated, `update_steps.rs`) `focus_fixup_steps_*` with `TestSink` | (a) non-focusable holder (HEAD's five arms: `hidden` on self, `hidden` on an ancestor, `disabled`, `type=hidden`, `href` removed — `tests.rs:719-770`) → events `[(h, Blur, None)]`, `changes == [h]`, focus None; (b) focusable holder → no events (`tests.rs:772-785`); (c) no holder → none; (d) holder in a `cloneNode` document → none, bit kept; (e) `reentrant_on_blur` → reentrant target holds focus | relocated: green / green on `reconcile_focus`; new events: n/a (new API) |
| C15 existing `unfocusing_steps` cells (`update_steps.rs` test module) | unchanged; must pass after the step-8 re-route | green |
| C16 (NEW, D-12) `oop_iframe_vm_native_hidden_runs_focus_fixup` | OOP child `<div id=d><input id=b></div>` with a script: `b` gets a `blur` listener that posts `"blur:" + (document.activeElement === document.body)`, then `b.focus()`, `d.hidden = true`, post `"armed"`; `run_oop_and_collect_posts` → posts contain `"armed"` and exactly one `"blur:true"` (the iframe thread's per-frame `crate::re_render`, `iframe/thread.rs:107`, ran the fixup) | red / red |
| C17 (NEW, D-13) `focus_fixup_unfocus_is_styled_this_frame` | `<style>#b:focus{color:rgb(255,0,0)}</style><button id=b>`; UA focus `b`; one frame (pin: `b`'s `ComputedStyle` color is red); VM `b.disabled = true`; one `re_render` → `b`'s `ComputedStyle` color is not red and `ElementState` lacks `FOCUS`, after that same `re_render` | red / green |
| C18 (NEW pin, 248 (6)) `delegates_focus_host_focused_by_vm_survives_fixup` | `<div id=h tabindex=0>` with an open `delegatesFocus` shadow root and a focusable delegate; VM `h.focus()`; two frames → `current_focus == h`, no `blur` | green / green |
| C19 (NEW pin, PM 260) `focus_fixup_no_change_from_script_alone` | `<input id=b>` UA-focused (snapshot seeded), no user edit; VM `b.value = "x"; b.disabled = true`; one rendering update → `["blur","focusout"]`, no `change` | red / red (A: no events; B: silent); red at D-10 commit (2), green from (3) |

Mutation guards (run on the final head; each must turn the named cells red):

| Guard | Mutation | Red cells |
|---|---|---|
| MG1 | wrap the `focus_fixup` call in `if !mutation_records.is_empty()` | C1, C2, C11, C16 |
| MG2 | in `focus_fixup_steps`, replace `focusing_steps_for_viewport(sink)` with `set_focus_bit(sink.dom(), None)` | C5, C6, C7, C8, C9, C14 (a)/(e) |
| MG3 | predicate `is_focusable` → `is_focusable_area` | C18 |
| MG5 | move the `focus_fixup` call after `layout_tree` (`lib.rs:703`) | C7, C17 |
| MG6 | move the `focus_fixup` call before the first `session.flush` (`lib.rs:576`) | C3 |
| MG7 | in `unfocusing_steps`, replace the step-8 call with a bare `return` | C15 |

Coverage: engine-independent cells in `elidex-dom-api` (C14, C15); integration in `elidex-shell` (rest). No WPT subset is claimed (the focus WPTs need the chain, A3).

## §7. Out of scope and slots

- Position after style + computed-style predicate + RO order → O1's CV-focus leg (§8).
- Window `focus` (S7) and the focus chain → `#11-canonical-focus-update-steps` part A3 (D-9; record R-A3, §8.2).
- C2 in the predicate → `#11-shadow-focus-delegation` / A2 (c).
- Fieldset-inherited disabled → `#11-focusable-area-fieldset-inherited-disabled` (A3; coordinated with
  `docs/plans/2026-07-form-derived-state-reconciliation.md` `:4`).
- VM `focus()` / `blur()` event dispatch and unfocusing step 6 → A2c, `#11-vm-host-synthetic-dom-event-dispatch`.
- OOP iframe focus messages → `#11-oop-iframe-focus-lifecycle` (C16 tests only the child's own fixup, not messages).
- `user validity` at step 2.1.1 → `#11-focus-change-user-validity`.
- Cross-document rendering order (§3 row 5) → FF-1; radio same-tree filter (§3.1) → FF-2; IME composition end on blur
  (I10) → FF-3; form owner as ECS state (§4.3) → FF-4; in-process child
  `needs_render` (S9) → no slot, re-check record R-S9 (§8.2). Owners and re-eval: umbrella §4.
- **Moved, not deferred:** change-on-blur user-change provenance → slice (1) (umbrella §1.2; the absence of a script-only `change` is guarded on the provenance side (its `take_change_on_blur` cells and MG1; docs/plans/2026-11-change-on-blur-user-provenance.md); C19 pins ordering only).

## §8. Edges, hand-offs and records

- **Order and edges:** umbrella §1.2–§1.3 (e1: this PR merges only after the provenance PR is on `origin/main`; this
  branch rebases onto it, §0.1 re-base note). Stall contingency: umbrella §1.4.
- **O1 — obligation on `#11-focusable-area-being-rendered`** (umbrella §4 O1; 248 (4)). O1 attaches to the first
  CV-focus leg that makes the focus predicate read computed style (PM 242 (7)). This PR does not move the position; that
  leg moves the fixup after style/layout because (a) its predicate reads computed style, which exists only after
  Phase 2, and (b) ResizeObserver delivery (`content/mod.rs:433`, `deliver_layout_observations`) must see the pre-fixup
  holder — spec step 16 precedes 17 (I5 × I9). The one-frame `:focus` lag is decided (PM 242 (1)): spec order, no
  in-frame re-style; the leg re-states C7 and C17 (and the listener writes of I4 × I9) as next-frame effects (umbrella
  X7). The same leg changes the VM `focus()` gate to the fixup's predicate (umbrella X8; I8 here); script-time freshness
  of that gate is the leg's plan's (umbrella e4 / e5). Its plan cites this paragraph.
- **CV umbrella** (`docs/plans/2026-10-content-visibility.md`, approved #541) names `reconcile_focus` at 12 sites (`git
  grep -c reconcile_focus d7d34035 -- docs/plans/2026-10-content-visibility.md` → 12, verified 2026-11-03; listed in
  umbrella CR-1); after this PR the symbol is `focus_fixup_steps` and it fires events. The CV erratum renames them
  (umbrella CR-1, PM 278). Its K8 ("render-time readers … read the last style pass, which they run after", `:360`) is
  O1's target state, not HEAD's.
- **FU** (`#11-forced-style-layout-flush-on-script-read`): FU-5's "not `reconcile_focus`" carries to `focus_fixup_steps`;
  the new fact is that the fixup runs script, whose style reads reach FU's point (I4 × FU).
- **Canonical-focus A2b / A2c / A3** (umbrella x1–x3): A2c adds unfocusing step 6 and VM event dispatch — `focus_fixup_steps` does not use
  `unfocusing_steps`, so step 6 cannot break it (E2); after A2c a reentrant VM `focus()` from a fixup listener fires its own
  events (I2 × I8). A3's chain adds S7 (Window `focus`) and may re-home `focusing_steps_for_viewport` under a target enum.
  I8 binding recorded on `#11-canonical-focus-update-steps` at landing (PM).
- **PR body records** (248 (6), R4): (i) VM `focus()` gates on `is_focusable && is_in_document`
  (`html_element_proto.rs:302`); `is_focusable` is C2-blind (`predicate.rs:176-180`) like the fixup, so a `delegatesFocus`
  host focused via VM `focus()` is not reset — consistent (C18; C2 = `#11-shadow-focus-delegation` / A2 (c)); (ii) F8's
  rejection with R4's reasoning; (iii) D-9 / E3; (iv) O1 verbatim; (v) red-first results per cell and base; (vi) the
  concept-sweep table §4.4; (vii) §8.1–§8.2's texts for the PM; (viii) C19's red at D-10 commit (2) and green from (3).

### §8.1 Slots FF-1 to FF-4 (defect and trigger texts; ID, owner, why and re-eval: umbrella §4)

- **FF-1 `#11-cross-document-rendering-order`** (252 (M2); crate `elidex-shell`). *Defect:* HTML *update
  the rendering* collects the documents in tree order (a container before its children) and runs each step over all of
  them before the next step; elidex runs one whole `crate::re_render` per pipeline, the in-process children's
  (`content/mod.rs:355`, `re_render_all_iframes`) before the parent's (`:357`), and OOP children on their own thread
  clock (`iframe/thread.rs:107`). *Measured:* the order is structural (read from the call sites above). *Not measured:*
  whether any web-observable outcome differs — ResizeObserver callbacks across a parent and child, `requestAnimationFrame`
  callback order across documents, and the focus fixup across iframes (the child's fixup runs before the parent's, so a
  child listener sees the parent's pre-fixup state). *Trigger:* (1) a probe first — a parent + in-process child fixture
  whose RO / rAF / fixup listeners record order into a shell-visible log, compared with the spec order; a difference
  makes the slot a defect with a cell, no difference closes it as unobservable today; (2) re-run the probe when
  `#11-windowproxy-browsing-context` lands (cross-document script access makes more orders observable).
- **FF-2 `#11-radio-group-same-tree-filter`** (252 (M3) (b); crate `elidex-form`). *Defect:* the radio button
  group scan (`elidex-form/src/radio.rs:110-123`, `find_radio_group_impl`) selects candidates by kind + `name` and form
  owner only; §4.10.5.1.16 Radio Button state's *radio button group* (`#radio-button-group`) also requires "Both a and b
  are in the same tree", so a detached or other-tree radio with the same name and form owner joins the group (checking
  one unchecks the other).
  *Trigger:* the next PR touching `radio.rs` or radio-group semantics.
- **FF-3 `#11-ime-composition-end-on-blur`** (Q3, I10; crates `elidex-shell` `content/ime.rs` + `elidex-form`). *Search first:* the ledger has no IME composition-end slot — the only IME-related one,
  `#11-textarea-edit-path-newline-normalization` (`memory/project_open-defer-slots.md:123`), is newline folding on the
  paste / IME insert paths, a different defect. *Defect:* when focus leaves a text control during an active composition
  (UA blur, VM `blur()`, or the step-17 fixup), elidex neither commits nor cancels it: no `compositionend`, the preedit
  stays in `FormControlState.composition_text` and is still painted into the control (`elidex-render`
  `builder/form.rs:226-236`); the shell's `ImeKind::Enabled | Disabled` arms are no-ops (`content/ime.rs:157-159`).
  *Constraint (umbrella X5):* when blur commits an active composition, the commit goes through
  `elidex_form::apply_user_edit` (NEW) before the change-on-blur substep runs, so a commit that changes the value marks
  the snapshot and fires `input`, and one that changes nothing fires neither.
  *Trigger:* (1) measure in the built-in Chromium whether blur commits or cancels the composition and which events fire;
  (2) the first shell PR that wires the platform IME cancel / commit request.
- **FF-4 `#11-form-owner-ecs-component`** (252 (M3) (a), plan-review Axis 2 (PM 292); crates `elidex-form`, `elidex-js` VM
  host, `elidex-shell`). *Defect:* no ECS state holds the form owner; three divergent resolvers recompute it per read —
  `find_form_ancestor` (ignores the `form` attribute) in nine VM prototypes (`html_input_proto.rs:876`,
  `html_button_proto.rs:398`, `html_select_proto.rs:399`, `html_textarea_proto.rs:641`, `html_fieldset_proto.rs:193`,
  `html_label_proto.rs:177`, `html_legend_proto.rs:96`, `html_option_proto.rs:370`, `html_output_proto.rs:186`) and the
  shell's `content/form_input.rs:131`, `:290`; `resolve_form_owner` (`radio.rs:19`, and `submit.rs:301` through
  `resolve_form_owner_public`); and the string reverse query in `submit.rs:332-338`. `find_form_by_id` (radio.rs:44-53) searches the whole World, so the `form`-attribute half of *reset the form owner* has no same-tree filter. The spec's form owner is set by
  *reset the form owner* (§4.10.18.3 `#reset-the-form-owner`); the ECS-native form is a form-owner component written
  there and read by all three. *Trigger:* the next PR that touches form-owner association (form-associated custom
  elements, `form` attribute reflection) or the L3 form program's next slice; re-eval date: the PM's, at landing.

### §8.2 Records for existing slots (PM applies at landing)

- **R-S9 → `#11-windowproxy-browsing-context`** (252 (M4), no new slot), text verbatim: "on landing, re-check whether S9
  (the in-process child iframe gated on needs_render) can be reached; see
  docs/plans/2026-11-focus-fixup-canonical-steps.md".
- **R-A3 → `#11-canonical-focus-update-steps` (A3)** (252 (M1)): "A3 also fires the focus update steps' events at
  Document entries — `focus` / `blur` at the Window — which the single-element model omits (elidex does not fire the
  Window focus event the spec chain gives). First observable site: the step-17 focus fixup (docs/plans/2026-11-focus-fixup-canonical-steps.md
  D-9 and E3, whose chain reading is unmeasured)."
- **Closure (umbrella FP-1):** "PR (2) closes `#11-focus-fixup-gated-on-session-records` and the folded
  `#11-render-side-effect-version-delta-signal` (PM 278); the folded slot's remedy (a version-delta signal threaded
  through the render entry) is superseded by this PR's design: the focus half by the ungated fixup, the ancestor-cache
  half by the cache's removal."
- **At landing (PM):** the ledger's CF-R1, the A3 named risk ("`reconcile_focus` blurs it") and FU-5 name
  `reconcile_focus`; the fixup is now `focus_fixup_steps` and fires events.
- **O1 → `#11-focusable-area-being-rendered`** (§8 O1; as §9 item 7).

## §9. Definition of done

1. `git grep -n 'reconcile_focus\b' -- crates` → only `reconcile_focused_iframe` hits (use
   `git grep -n -w 'reconcile_focus' -- crates` → 0).
2. `git grep -n 'mutation_records' -- crates/shell/elidex-shell/src/lib.rs` → only the settle block (no consumer gate).
3. `git grep -n 'AncestorCache\|ancestor_cache\|get_form_owner' -- crates` → 0.
4. Cells C1–C9, C11, C14–C19 green on the final head; the NEW cells red / green on the bases as tabled (results in the PR body);
   MG1–MG3, MG5–MG7 each red on their cells, then reverted.
5. Comments F4, F5 and §5.1's sites updated; `focus/tests.rs` ≤ 999 lines.
6. `cargo fmt --all`; `cargo test -p elidex-dom-api -p elidex-form -p elidex-shell --all-features`; `mise run ci` green
   (`grep 'ERROR task failed'` on its log → none).
7. PR body carries §8's records; `#11-focus-fixup-gated-on-session-records` and the folded
   `#11-render-side-effect-version-delta-signal` reported closed to the PM, the folded slot's version-delta remedy
   superseded (§8.2 closure, umbrella FP-1), with O1 appended to `#11-focusable-area-being-rendered`, R-S9 and R-A3
   (§8.2) and FF-1 to FF-4 (§8.1) handed over (PM transcribes).
8. The provenance PR (`docs/plans/2026-11-change-on-blur-user-provenance.md`) is merged on `origin/main`, this branch is
   rebased onto it before D-10's commits, and C19 is red at commit (2) and green from (3) (recorded in the PR body).

## §10. Open items

Rev 1's Q1–Q6 are answered (§0.5) and its M1–M5 decided by PM 252 (applied: M1 → D-9 / R-A3; M2 → FF-1; M3 (b) → FF-2;
M4 → §4.4 S9 / R-S9; M5 → §4.5, whose fix PM 260 moved to the provenance PR).

**Questions for scout:** none. Rev 2's Q7 (the user-edit path enumeration) and Q8 (the script-writer population)
moved with the provenance design; the provenance memo answers Q7 (§4.2) and lists the population (§0.2 P7).

**Needs a PM decision / judgement:** none (M3 (a): Axis 2 judged it out, FF-4).

## Changelog

- *Rev 5.1* (2026-11-03): C19 narrowed to ordering (PM 298); re-check MINs m1–m5; FF-4 clause (PM 295).
- **Rev 5 (2026-11-03; PM 292 on the rev 4 plan-review: IMP 5 / MIN 15).** 292 (1): D-8 deletes `AncestorCache`
  instead of validating it on read (removed: the validity-stamp design, P7, §4.2's stamp text, the "only path" claim
  and the `document_root()` stand-in, the §4.3 (X)/(Y) block, cells C12 / C13, MG4, pair I4 × I7); FF-4 moved to §8.1
  with its three reader families; closure record names the removal. 292 (2): umbrella row (2) wording. 292 (3): D-10
  reordered (C19 alone at commit (2), red there, green from (3)); §4.5, R3, §6, §8 (viii), §9 item 8. 292 (4): A2c/A3
  bullet's I8 pointer. 292 (5): D-6's one-statement move deleted; C10 (no guard turns it red) and C20 (duplicate of the
  reconciler cell, P9) removed (IDs not reused); bases re-anchored (§0.1); stall contingency → umbrella §1.4; ledger
  cite by slot ID; umbrella location stated once (header); E3's chain reading stated as unmeasured (also R-A3);
  D-2 / §3 unfocusing step 8 both branches; §3 step 2.1 row narrowed to `input`; anchors on §4.10.5.1.16 /
  §4.10.18.3; §5.1 adds `delegate.rs:66`, `:128` and `update_steps.rs:14-23`, `:184-203`; §8.2 at-landing line for
  CF-R1 / the A3 named risk / FU-5.

- **Rev 4 (2026-11-03; umbrella `docs/plans/2026-11-focus-fixup-program.md` rev 2.1 records FP-1 to FP-3, CR-1; PM 264
  (M2), 278, 282).** Header: slice (2) under the umbrella, base case (c). FP-1: §8.2 closure record and §9 item 7 (the
  folded `#11-render-side-effect-version-delta-signal` closed, its version-delta remedy superseded). FP-2: FF-3's X5
  constraint. FP-3: O1 attaches to the first style-reading CV-focus leg (PM 242 (7)), the one-frame lag decided (PM 242
  (1); also D-6, I4 × I9), X8 added with freshness pointed to the leg's plan; the CV-umbrella bullet points to CR-1.
  PM 264 (M2): §0.1 re-base note and C6 no longer assume a `text_edit.rs` move. Change-on-blur judgement =
  `elidex_form::take_change_on_blur` (umbrella X1): P10, I2, I3, §3 row, §3.1; C6 / C20 assert the absent
  `FocusValueSnapshot` (`take_focus_snapshot` is replaced by the provenance PR). Umbrella pointers: §1's provenance
  rationale, §4.5 (narrowed to the fixup-path facts), §7, §8 order line; P-1 to P-3 → FF-1 to FF-3, §4.3 (Y) → FF-4,
  owners and dates removed from slot texts. §10's M6 removed (decided, rev 3).

- **Rev 3 (2026-11-03, PM 260).** Rev 2's D-11 (change-on-blur user-change provenance) moved to its own PR,
  `docs/plans/2026-11-change-on-blur-user-provenance.md`, which lands first. Removed: D-11's design (now a tombstone), I11 and its pairs (I2 × I11, I3 × I11,
  I4 × I11, I11 × I6), the §3 row for §4.10.5.5's "not fired … by scripts" (M 13 → 12), §4.5's mechanism and mixed-case
  text, the §6 Chromium pre-step, cells C19–C22, guards MG8 / MG9, §5.2's `form_input.rs` / `focus_snapshot.rs` rows,
  Q7 / Q8, DoD item 8's grep. Kept: §4.5's reachability argument (it justifies the order). Moved and renumbered: rev 2's
  C23 → C20, because it pins a fixup-path fact (the type flip clears the snapshot before the fixup runs, E5), not the
  provenance. Added: C19, the landing-order pin (script-only change → no `change` on the fixup path), red on this branch
  until the provenance PR lands; the §8 order line and dependency; the §0.1 re-base note; DoD item 8 (dependency merged,
  C19 green after the rebase); M6 decided. Commits renumbered (D-10): (2) → removed, (3) → (2), (4) → (3).
- **Rev 2 (`1477feef`).** PM 252 (M1–M5) and scout Q1–Q6 applied.
- **Rev 1 (`cbee12e2`).** First draft.
