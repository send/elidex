# Focus-fixup program — the rendering-time focus fixup through the canonical steps, with a correct change-on-blur (umbrella memo)

Program label **`focus-fixup`**. Owner: Layout lane. Rev 2, 2026-11-03 (plan-review of rev 1 `1e5976a7` applied; PM
decisions 278, 279). Required by **PM decision 265**: the provenance PR and the focus-fixup PR each become a terminal
per-PR slice under CLAUDE.md *Edge-dense work* base case (c), which names an approved umbrella. This memo is that
umbrella: the nested-PR set, the order and its edges, the invariants that span PRs, the slot inventory and the program
DoD. Slice detail (mechanism, files, new functions, cells) is each slice plan's and is pointed to, not restated.

Nested per-PR plans (each plan-reviewed on its own):
- **(1) provenance** — `docs/plans/2026-11-change-on-blur-user-provenance.md` (rev 1 at `fb42a1a0`; this branch).
- **(2) focus-fixup** — `docs/plans/2026-11-focus-fixup-canonical-steps.md` (rev 3 at `36bcc114`, branch
  `layout-focus-fixup-ungated`; not on this branch until (2) rebases, so cited as a path).
- **(3) being-rendered** — the first CV-focus leg that makes the focus predicate read computed style (§1.2); its plan is
  the CV umbrella's. This program fixes only the obligation it carries (O1, §4) and the edges into it.

**Approval scope.** Approving this umbrella decides the nested-PR set, the order and its edges (§1), the cross-PR
invariants (§2), the program-level spec map (§3), the slot inventory and owners (§4) and the program DoD (§6). It does
not decide anything inside a slice.

## §0. Frame, program statement, decisions

### §0.1 Coordinate frame

Base `origin/main` **`d7d34035`** (#582) for both per-PR plans (`git log --oneline -1 origin/main` → `d7d34035`).
This memo cites no new code line; code facts are the per-PR plans' and keep their frame.

### §0.2 Program statement — the defect classes

Names used below. **The fixup step** = WHATWG HTML *update the rendering* (`#update-the-rendering`), the step "if the
focused area of doc is not a focusable area, then run the focusing steps for doc's viewport" (currently step 17).
**The change-on-blur substep** = *focus update steps* (`#focus-update-steps`) step 2's first substep (currently 2.1).

- **DC1 The fixup step is gated on session records** (slot `#11-focus-fixup-gated-on-session-records`, PM decision 245;
  the folded `#11-render-side-effect-version-delta-signal`, PM decision 278). VM-native writes produce no records, so a
  non-focusable element stays focused; ungating HEAD's silent reset alone fires no `change` / `blur` / `focusout`.
  PM decision 248 sets the ideal: ungated **and** through the canonical focusing steps for the viewport. Plan (2).
- **DC2 Change-on-blur has no user-change provenance.** The change-on-blur substep fires `change` at an entry only if it
  is an input element, the change event applies to it, it has no defined activation behavior, and the user has changed
  its value (or list of selected files) while focused without committing that change, so that it differs from the
  focus-time value; its substep 1 sets user validity first (`#11-focus-change-user-validity`, not this program). elidex
  compares only with the focus-time value, so a script write followed by a blur fires `change`. The same predicate gates
  `input` (fired today on user actions that change nothing). Found by plan (2) rev 2 (PM decision 252 (M5)); own PR by PM
  decision 260; `input` in scope by PM decision 264 (M3). Plan (1). `textarea` has no change rule in the spec text;
  plan (1) keeps browser behaviour, measured.
- **DC3 The being-rendered obligation** (slot `#11-focusable-area-being-rendered`). Once the focusable-area predicate
  reads computed style (HTML §6.6.2: the element is "being rendered, delegating its rendering to its children, or being
  used as relevant canvas fallback content"), the fixup step must run after that frame's style and layout and after
  ResizeObserver delivery (spec order: step 16 before the fixup step). PM decisions 246 and 248 put the move with that
  PR, not with (2). Which part of it (3) reads is decided by CV-focus's plan. Obligation O1 (§4).

### §0.3 Decisions already made (cited as "PM decision N")

| PM decision | Content | Applied in |
|---|---|---|
| 242 (1) | The fixup step runs after this frame's style/layout, ungated; spec order: the un-focus restyle shows next frame (one-frame `:focus` lag), no in-frame re-style; recorded as decided | (3); O1; X7 |
| 242 (7) | CV-focus is split: the author-hidden S2 leg lands right after CV-render; the until-found attribute→S2 switch stays after A2c | (3); x1; e5 |
| 245 | Slot `#11-focus-fixup-gated-on-session-records` registered (owner Layout, re-eval 2026-12-15); Layout order C0 → C0 plan-reviews → this PR → C1 → CV erratum → T3; the CV erratum cites the slot | (2); §1.2 order; e4 |
| 246 | The fixup stays before style in (2); the move goes to `#11-focusable-area-being-rendered` | (2); (3) |
| 248 | Reverses 246's "gate only": ungated **and** canonical; form-owner cache sibling in (2); RO order a second reason for the move | (2); (3) |
| 252 | (M1) Window `focus` → canonical-focus A3; (M2) cross-document order → slot; (M3) radio same-tree filter → slot; (M5) a wrong event the fixup newly reaches is fixed before or with it | (2) §8; (1) exists because of M5 |
| 260 | Provenance is its own PR and lands before (2) | (1); e1 |
| 264 | Owners of the proposed slots (§4); (M2) no split bundled in (1), a standalone split PR first if `content/event_handlers.rs` is over 1000 lines after (1) — supersedes plan (1) D-6 and plan (2)'s §0.1 re-base note; (M3) `input` fix in (1) | (1), (2) next revs; §4; e3 |
| 265 | (1) and (2) need an approved umbrella | this memo |
| 269 | Row (3) shared with the CV umbrella, the PR is CV-focus; both plans apply 264 (M2) in their next revisions, before their reviews; vehicle (§1.4) accepted | §1.2; §5 |
| 278 | `#11-render-side-effect-version-delta-signal` folded into `#11-focus-fixup-gated-on-session-records`; O1 on the first style-reading CV-focus leg; CV erratum edge; discharge decided by CV-focus's plan; the CV erratum renames the CV umbrella's `reconcile_focus` references | §1.2–§1.3; §4; §6 |
| 279 | The step-2.1 judgement in one elidex-form fn | X1, X8; §4 |

## §1. Program shape

### §1.1 Edge density

Edge-dense by both arms of the CLAUDE.md rule: (a) event dispatch order (`input` / `change` / `blur`), the focus-transition
paths (UA sink, `blur_current`, VM `focus()` / `blur()`, the fixup step), script writers and re-entry, the snapshot
lifecycle and the render-phase position intersect (§2); (b) "the user has changed the element's value" is prose with no
algorithm, and the mixed case is measured. So: a multi-PR program, each slice plan-reviewed, each a terminal unit.

### §1.2 Nested PRs

| nested PR | scope (detail: the slice plan) | owner | slots | prerequisites |
|---|---|---|---|---|
| **(1) provenance** | DC2: one user-edit funnel; `input` iff the value changed; the change-on-blur substep judged by one elidex-form fn (X1); the Chromium pre-step picks the branch | Layout | closes none (DC2 was found inside DC1's scope); registers CP-1 to CP-4 | this umbrella approved; plan (1) plan-reviewed; split PR if 264 (M2) triggers |
| **(2) focus-fixup** | DC1: the fixup step through the canonical focusing steps for the viewport, ungated, at HEAD's position; the silent reset deleted; the record-gated form-owner cache removed | Layout | closes `#11-focus-fixup-gated-on-session-records` and the folded `#11-render-side-effect-version-delta-signal` (PM 278); appends O1; registers FF-1 to FF-3 (FF-4 if Axis 2 picks (Y)) | (1) on `origin/main`; plan (2) plan-reviewed |
| **(3) being-rendered** | DC3: the first CV-focus leg that makes the focus predicate read computed style (per PM 242 (7) likely the author-hidden S2 leg right after CV-render); carries O1 | Layout (CV umbrella `docs/plans/2026-10-content-visibility.md`, CV-focus) | discharge of `#11-focusable-area-being-rendered` decided by CV-focus's plan (ledger APPEND on that slot); O1 is cited there | (2) landed; the CV erratum (e5); the CV umbrella's own edges |

Row (3) is shared (PM decision 269): this program fixes O1 and the edges e2 / e5; the PR, its plan and its other
prerequisites are the CV umbrella's.

**Order (Layout lane; PM decisions 245, 260, 264, 265):** C0 restructure → C0 plan-reviews → this umbrella approved →
[standalone `event_handlers.rs` split, only if 264 (M2) triggers] → **(1) provenance** → **(2) focus-fixup** → C1 →
CV erratum → T3. **(3)** follows (2) and the CV erratum, at the CV umbrella's position for its leg.

### §1.3 Edges

| # | edge | kind | reason |
|---|---|---|---|
| e1 | (1) → (2) | semantic precondition | (2) runs the change-on-blur substep on a script-made non-focusability; without (1), VM `b.value = "x"; b.disabled = true` fires `change` from script alone (PM 252 (M5)). Plan (2)'s pin C19 is red until (1) is on its base |
| e2 | (2) → (3) | code predecessor + semantic | (3) moves the call (2) creates and keeps its canonical events; moving the silent reset would skip the substep. The move is needed only once the predicate reads style (PM 246, 248) |
| e3 | split PR → (1) | scheduling, conditional | PM 264 (M2) |
| e4 | (2) → CV erratum | doc coherence | the CV erratum writes the CV-focus split into the CV umbrella and cites `#11-focus-fixup-gated-on-session-records` (PM 245); it also renames the CV umbrella's `reconcile_focus` references (record CR-1, §4), which name a symbol (2) deletes |
| e5 | CV erratum → (3) | definitional | the erratum creates the leg that is (3) (PM 242 (7)); O1 attaches to it |
| x1 | canonical-focus A2c (`#11-vm-host-synthetic-dom-event-dispatch`) | external, after (1) and (2) | VM `blur()` / `focus()` start running the canonical transition: A2c's VM sink calls the elidex-form judgement and only dispatches (X1, record R-1); (1)'s "mark before `input`" becomes observable; a fixup listener's reentrant `focus()` fires its own events (plan (2) I2 × I8). A2c is **not** necessarily before (3): after PM 242 (7) only the until-found leg waits on A2c (CF-1), so X8 must hold without A2c |
| x2 | canonical-focus A3 | external, after (2) | the Window `focus` event at the Document entry of the new chain, not fired by (2) (PM 252 (M1); record R-A3) |
| x3 | canonical-focus A2b | external, file overlap, no order | A2b (click ancestor climb, editing-host fallback) edits shell `content/focus.rs` `set_focus` / the click entry, which (1) and (2) also edit; the later lander rebases. The A2b / A2c / A3 order is Layout's to propose (PM 98), not fixed |
| x4 | L3 form domain | external, no order | CP-1 to CP-4, FF-2, FF-3, FF-4 are L3-owned defects found by (1) and (2); none gates a slice |
| x5 | `#11-windowproxy-browsing-context` | external, after (2) | record R-S9 (plan (2) §4.4 S9) |
| x7 | `#11-shell-effective-disabled-predicate-adoption` | external, file overlap, no order | pointer only: (1) edits shell files that slot cites (§5) |

Script-time freshness of VM `focus()` against the fixup, and the window between the leg and A2c, are decided by the CV-focus leg's plan (CV K15 :393-396).

Acyclic. No edge points from (2) or (3) back into (1).

### §1.4 Vehicle: this memo travels with (1)

The umbrella is committed on (1)'s branch (PM decision 269): (i) e1 makes (2) and (3) wait for (1) anyway; (ii) (1)'s
open item "does a PM decision count as the umbrella" is answered by this memo, on the same branch; (iii) precedent: the
EH-attrs umbrella's revisions travelled with IBP-sandbox N1's PR. Approval is this memo's plan-review, before (1)'s
first code commit.

**Stall contingency.** Owner: Layout lane. Trigger: plan (1)'s Chromium pre-step hits its STOP row. Layout reports the
STOP to the PM the same day. DC1 stays a live defect meanwhile (its slot's re-eval 2026-12-15); the PM decides between
waiting for (1) and shipping (2) ahead of it with a guard against e1's wrong `change`.

## §2. Cross-PR coupled invariants

Only pairs that span two slices. "(1) I5" = plan (1)'s invariant I5.

- **X1 One change-on-blur judgement, in elidex-form, every sink** — (1) I5 × (2) I2 × x1. The substep's condition (the
  qualifiers of DC2 and the branch's user-change predicate, `user_changed` or `last_user_value`) is computed by one
  elidex-form function (NEW, engine-independent; record PR-1 names it); it consumes the snapshot. Every sink — the shell
  sink (UA, `blur_current`, (2)'s fixup) and A2c's VM sink — calls it and only dispatches `change`; no sink, shell or
  `vm/host`, computes the predicate. Pinned by (1) C1 / C3 and (2) C19; A2c's cells are A2c's.
- **X2 Snapshot consumed once, mark dies with it** — (1) I3 × (2) I3: (2)'s snapshot-reading cells are re-derived at its
  rebase onto (1); a type flip to a non-text kind clears snapshot and mark together.
- **X3 (2)'s user-edit step goes through (1)'s funnel** — (1) I1 / I2 × (2) C6: after (1), `input` fires iff the value
  changed, so C6 uses a value-changing key; its cites are re-derived at the rebased head (264 (M2) drops the move).
- **X4 Fixup listeners never mark** — (1) I4 × (2) I4: a listener run by the fixup that writes `value` is a script
  writer; the snapshot is already taken. Position-independent, so it also holds after (3).
- **X5 IME commit at a fixup blur** — (1)'s IME routing × (2) I10: a future blur-time composition commit goes through
  `apply_user_edit` before the change-on-blur substep. Home: FF-3's slot text (record FP-2).
- **X6 Placement after the style the predicate reads** — (2) I9 × (3): the fixup step runs after the style and layout
  whose computed style the predicate reads, and after RO delivery. Before (3) the predicate is attribute-only, so the
  before-style position gives the same answer (PM 246).
- **X7 Same-frame effects re-stated** — (2) I4 / I10 × (3): (2)'s C7 and C17 assert same-frame effects that hold only
  while the fixup precedes style. Decided (PM 242 (1)): spec order, no in-frame re-style; (3) re-states both as
  next-frame effects.
- **X8 One predicate for writer and fixup** — (2) I8 × (3): the leg that makes the fixup's predicate read computed style
  changes the VM `focus()` gate to the same predicate in the same leg, with or without A2c (x1), or every frame
  focuses-then-resets with events. See e4/e5.

**(1) × (3): empty.** (1) changes who marks and where the substep is judged (snapshot and value only); (3) changes when
the fixup step runs and what the focusability predicate reads (computed style only). They share no state; the only path
between them is (2)'s call into the sink, covered by X1, and listener writes, covered by X4.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering*, the fixup step (currently 17) | focused area not a focusable area → focusing steps for the viewport, ungated, canonical events | (2) | ✓ | yes |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering*, ResizeObserver step (currently 16) before the fixup step | style, layout and RO before the fixup | (3), O1; (2) keeps the position (X6) | ✗ | no |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering*, docs in tree order (currently step 2) | each step over all documents | not met; FF-1 | ✗ | no |
| WHATWG HTML §6.6.4 Processing model | focusing steps, target = viewport; unfocusing steps (currently step 8) | one viewport transition, two callers | (2) | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focus update steps*, the change-on-blur substep (currently 2.1) | DC2's qualifiers; user change since focus and value ≠ focus-time value; script-only → none; mixed case by the pre-step | (1); X1 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focus update steps*, user validity (currently 2.1.1) | set user validity | not this program; `#11-focus-change-user-validity` | ✗ | no |
| WHATWG HTML §6.6.4 Processing model | *focus update steps*, Document entry of the new chain (currently step 4) | `focus` at the Window | not fired; canonical-focus A3 (R-A3) | ✗ | no |
| WHATWG HTML §4.10.5.5 Common event behaviors | `input` when the user changes the value; none for script | `input` iff the funnel reports a change | (1) | ✓ | yes |
| WHATWG HTML §4.10.11 The textarea element | user change → `input`; no `change` rule in the text | same funnel; change-on-blur kept as browser behaviour, measured | (1) | ✗ | yes |
| WHATWG HTML §6.6.2 Data model | focusable area condition | being rendered, delegating its rendering to its children, or relevant canvas fallback content | (3) | ✗ | no |
| WHATWG HTML §15.1 Introduction | *being rendered* | the predicate's style read | (3) | ✗ | no |

Numbers and titles: `.claude/tools/webref coverage-map html 8.1.7.3 html 6.6.4 html 6.6.2 html 4.10.5.5 html 4.10.11`
(§8.1.7.3 `#event-loop-processing-model`, §6.6.4 `#focus-processing-model`, §6.6.2 `#data-model`, §4.10.5.5
`#common-input-element-events`, §4.10.11 `#the-textarea-element`); `webref dfn html 'being rendered'` → §15.1
`#being-rendered`. Step texts: `.claude/tools/webref body html update-the-rendering`, `focus-update-steps`, `data-model`
(2026-11-03). Breadth: K = 1, M = 11. Each slice's §3 is the row-level SSoT; this table only assigns rows to slices.

### §3.1 User-input touch audit

User input reaches the program through (1)'s funnel (keys, clipboard, IME) and is judged at the change-on-blur substep
on every blur path, the fixup's included (X1). Script reaches it through value writers (never marking, X4) and through
the listeners (2) newly runs inside the rendering update, on the renderer main thread. No slice adds a parse of user input.

## §4. Slots and records (registered or applied by the PM at each landing)

IDs are proposals, prefixed **CP-** (plan (1) §8.1) and **FF-** (plan (2) §8.1, §4.3). Owners: PM decision 264. Slot
texts are the per-PR plans', with the amendments below. Re-eval dates are proposals: 2026-12-15 matches DC1's slot.

| ID | Slot | Owner | Why not in a slice | Trigger | Re-eval | Registered at |
|---|---|---|---|---|---|---|
| CP-1 | `#11-readonly-selection-edit-bypass` | L3 | form-core key action, not DC2 | next PR touching form-core key handling | 2026-12-15 | (1) landing |
| CP-2 | `#11-dirty-flag-on-cursor-move` | L3 | dirty value flag, not DC2 | with CP-1 | 2026-12-15 | (1) landing |
| CP-3 | `#11-number-input-user-edit-path` | L3 | number has no user edit path at all | L3 form program's next slice | 2026-12-15 | (1) landing |
| CP-4 | `#11-select-keyboard-change-event` | L3 | select's own event, not text controls | next PR touching select interaction | 2026-12-15 | (1) landing |
| FF-1 | `#11-cross-document-rendering-order` | Layout | multi-document order, not the fixup | the probe in plan (2) FF-1 (§8.1); re-probe at `#11-windowproxy-browsing-context` | 2026-12-15 | (2) landing |
| FF-2 | `#11-radio-group-same-tree-filter` | L3 | radio semantics, not the fixup | next PR touching radio-group semantics | 2026-12-15 | (2) landing |
| FF-3 | `#11-ime-composition-end-on-blur` | L3 (proposed) | platform IME wiring; carries X5 (FP-2) | Chromium measurement, or the first PR wiring IME commit / cancel | 2026-12-15 | (2) landing |
| FF-4 | `#11-form-owner-ecs-component` | L3 | only if plan-review Axis 2 picks (Y) | Axis 2's verdict | PM sets at landing (conditional) | (2) landing |
| O1 | obligation on `#11-focusable-area-being-rendered` | Layout (CV-focus) | needs computed style in the predicate | the CV erratum names the leg (e5) | 2026-12-15 | (2) landing |

**O1 (program text; plan (2) §8's O1 plus the amendments in FP-3):** attaches to the first CV-focus leg that makes the
focus predicate read computed style. That leg moves the fixup step after the frame's style and layout and after RO
delivery (X6), re-states C7 / C17 under PM 242 (1)'s one-frame lag (X7), and changes the VM `focus()` gate in the same leg
(X8). CV-focus's plan decides the slot's discharge.

Records for existing slots: **R-1** → `#11-vm-host-synthetic-dom-event-dispatch` (A2c; plan (1) §8.2, amended by PR-2);
**R-2** → `#11-focus-fixup-gated-on-session-records` (plan (1) §8.2); **R-S9** → `#11-windowproxy-browsing-context`;
**R-A3** → `#11-canonical-focus-update-steps` (plan (2) §8.2); and:
- **CR-1** → the CV erratum PR (e4): it renames the CV umbrella's `reconcile_focus` references (12 at `d7d34035`:
  `git grep -n reconcile_focus d7d34035 -- docs/plans/2026-10-content-visibility.md` → :89, :241, :291, :360, :394,
  :398, :463, :634, :664, :690, :841, :875) to the canonical fixup (`focus_fixup_steps` / `content::focus::focus_fixup` (NEW), both plan (2)'s);
  the PM updates the ledger's CF-R1 text at that landing.

Records for the slice plans (the driver applies them in their next revisions):
- **PR-1 → plan (1) rev 2**: "The change-on-blur substep judgement moves out of shell `dispatch_change_on_blur` into
  `elidex_form::take_change_on_blur` (NEW) (single owner `elidex-form`, module `focus_snapshot.rs`). It consumes the
  `FocusValueSnapshot` and returns whether the substep fires (the §4.3 branch's predicate: A `user_changed && value !=
  initial`; B2 `value == last_user_value && value != initial`; B1 as A after re-baseline). Every sink calls it and only
  dispatches `change`: the shell sink's `dispatch_change_on_blur` becomes a dispatcher, and A2c's VM sink calls the same
  function. I5 reads 'one judgement, called by every sink' (umbrella X1)."
  Existing text PR-1 rewrites: `crates/dom/elidex-dom-api/src/focus/update_steps.rs:72-76` (the `commit_change_on_blur` doc comment 'the shell takes the snapshot, compares, and dispatches').
- **PR-2 → plan (1) R-1**, appended: "A2c's VM sink calls `elidex_form::take_change_on_blur` (NEW) and only dispatches
  `change`; `vm/host` does not compute the predicate (Layering mandate; umbrella X1)."
- **FP-1 → plan (2) §8.2 and §9**: "PR (2) closes `#11-focus-fixup-gated-on-session-records` and the folded
  `#11-render-side-effect-version-delta-signal` (PM 278); the folded slot's remedy (a version-delta signal threaded
  through the render entry) is superseded by this PR's design."
- **FP-2 → plan (2) §8.1 P-3**, appended to *Defect*: "*Constraint (umbrella X5):* when blur commits an active
  composition, the commit goes through `elidex_form::apply_user_edit` (NEW) before the change-on-blur substep runs, so a
  commit that changes the value marks the snapshot and fires `input`, and one that changes nothing fires neither."
- **FP-3 → plan (2) §8 O1 and the CV-umbrella bullet**: O1 is re-pointed to "the first CV-focus leg that makes the focus
  predicate read computed style (PM 242 (7))", gains X8, and states the lag as decided (PM 242 (1));
  the CV-umbrella bullet's "not edited here; CV-focus re-derives" becomes "the CV erratum renames them (CR-1, PM 278)".

## §5. Open for the PM

None. Rev 1's M1 is decided (PM decision 269: shared row, the PR is CV-focus; refined by 278: the first style-reading
leg, discharge by CV-focus's plan) and M2 is decided (PM decision 269: both plans apply 264 (M2) in their next revisions,
before their reviews). x7's slot is not edited by this program; the driver reports to the PM whether (1)'s edits change
its trigger or scope.

## §6. Definition of done (program)

1. This umbrella is plan-reviewed and approved before (1)'s first code commit; each slice's plan is plan-reviewed before
   its implementation.
2. The order of §1.2 holds: (1) on `origin/main` before (2) merges; (2) before (3). (2)'s C19 recorded red before its
   rebase and green after (plan (2) §9 item 8).
3. At each landing the PM registers that slice's slots and applies its records (§4); at (2)'s landing
   `#11-focus-fixup-gated-on-session-records` and the folded `#11-render-side-effect-version-delta-signal` are closed.
4. O1 is on `#11-focusable-area-being-rendered` from (2)'s landing, and the CV erratum names the leg it attaches to and
   applies CR-1. From then on O1's discharge is CV-focus's plan's (PM 278); this program does not wait for that leg.
5. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-11-focus-fixup-program.md` exits 0.

## Changelog

- *Rev 2.1* (2026-11-03): removals per PM 282 (X6 sentence; FS-1, x6, FU-4 assignment, display repro); e4/e5 pointer; PR-1 list + update_steps.rs:72-76.
- **Rev 2 (2026-11-03)** — plan-review of rev 1 (CRIT 0 / IMP 8 after merge / MIN 14 raw) and PM decisions 278, 279:
  X1 becomes one elidex-form judgement for every sink (records PR-1, PR-2); X5 homed on FF-3 (FP-2); X8 into O1, style
  freshness as defect record FS-1 on FU (edge x6); (3) = the first style-reading CV-focus leg, x1 corrected, CV erratum
  edges e4 / e5, discharge by CV-focus's plan, CR-1 rename record; folded slot closed by (2) (FP-1); §4 why / trigger /
  re-eval; §5 decided (PM 269); (1) × (3) stated empty; slice detail trimmed to pointers; stall contingency; spec steps
  by anchor + name, the full §6.6.2 condition and the change-on-blur qualifiers; Layout-lane order; A2b and x7 edges;
  X7's lag decided.
- **Rev 1 (2026-11-03)** — initial draft (`1e5976a7`).
