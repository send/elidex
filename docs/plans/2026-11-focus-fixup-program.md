# Focus-fixup program — the step-17 focus fixup through the canonical steps, with a correct change-on-blur (umbrella memo)

Program label **`focus-fixup`**. Owner: Layout lane. Rev 1, 2026-11-03, draft for `/elidex-plan-review`. Required by
**PM decision 265**: the provenance PR and the focus-fixup PR each become a terminal per-PR slice under CLAUDE.md
*Edge-dense work* base case (c), which names an approved umbrella; a PM decision does not stand in for one. This memo
is that umbrella. It restates what the two per-PR plans decide only where the cross-PR layer needs it, and adds that layer:
the nested-PR set, the order and its edges, the invariants that span PRs, the slot inventory and the program DoD.

Nested per-PR plans (each plan-reviewed on its own):
- **(1) provenance** — `docs/plans/2026-11-change-on-blur-user-provenance.md`, rev 1 at `fb42a1a0`, branch
  `layout-change-on-blur-provenance` (this memo travels on that branch, §1.4).
- **(2) focus-fixup** — `docs/plans/2026-11-focus-fixup-canonical-steps.md`, rev 3 at `36bcc114`, branch
  `layout-focus-fixup-ungated` (worktree `wt-focus-fixup`; the file is not on this branch until (2) rebases, so it is
  cited as a path, not linked).
- **(3) being-rendered** — no plan yet; the obligation it carries is fixed here (§1.2, §2 X6–X8).

**Approval scope.** Approving this umbrella decides the nested-PR set, the order and its edges (§1), the cross-PR
invariants (§2), the program-level spec map (§3), the slot inventory and owners (§4) and the program DoD (§6). It does
not decide anything inside a slice: each slice's mechanism, cells and guards stay its own plan's.

## §0. Frame, program statement, decisions

### §0.1 Coordinate frame

Base `origin/main` **`d7d34035`** (#582) for both per-PR plans (`git log --oneline -1 origin/main` → `d7d34035`).
This memo cites no new code line; every code fact below is quoted from a per-PR plan and keeps that plan's frame.

### §0.2 Program statement — the defect classes

- **DC1 Step-17 fixup gated on session records** (slot `#11-focus-fixup-gated-on-session-records`, PM decision 245).
  The render-time focus fixup runs only when `session.flush` returns records, so VM-native writes (`d.hidden = true`)
  leave a non-focusable element focused; and once ungated, HEAD's `reconcile_focus` is a silent reset (no `change` /
  `blur` / `focusout`, snapshot never consumed). PM decision 248 sets the ideal: ungated **and** through the canonical
  §6.6.4 focusing steps for the viewport. Plan (2).
- **DC2 Change-on-blur has no "the user has changed the value" provenance.** Step 2.1 of the focus update steps fires
  `change` only if the user changed the value while focused; elidex compares the value with the focus-time snapshot and
  nothing else, so a script `value` write followed by a blur fires `change`. Found by plan (2)'s rev 2 under PM decision
  252 (M5); moved to its own PR by PM decision 260. Plan (1). The same predicate gates `input`: elidex fires `input` on
  user actions that do not change the value (cursor moves, readonly navigation, empty paste / IME commit) — plan (1) D-2,
  in scope by PM decision 264 (M3).
- **DC3 The being-rendered obligation** (slot `#11-focusable-area-being-rendered`). Once the focusability predicate
  reads computed style (*being rendered*), the fixup must run after that frame's style and layout, and after
  ResizeObserver delivery (spec step 16 precedes 17); the one-frame `:focus` lag question (PM decision 242 (1)) is
  decided there. PM decisions 246 and 248 put the move with that PR, not with (2). Plan (2) records it as obligation O1.

### §0.3 Decisions already made (cited as "PM decision N")

| PM decision | Content | Applied in |
|---|---|---|
| 242 (1) | The one-frame `:focus` lag after the fixup moves after style is decided with the being-rendered move | (3); (2) O1 |
| 245 | Slot `#11-focus-fixup-gated-on-session-records` registered, owner Layout, a standalone PR, plan-review if edge-dense | (2) |
| 246 | Scope narrowed: the fixup stays before style; the move after style/layout goes to `#11-focusable-area-being-rendered` | (2) D-6; (3) |
| 248 | Scope re-widened (reverses 246's "gate only"): ungated **and** canonical; form-owner cache sibling in (2); RO order added as a second reason for the being-rendered move | (2) D-1 to D-8; (3) |
| 252 | (M1) Window `focus` event → canonical-focus A3; (M2) cross-document order → proposed slot; (M3) form-owner side-store judged by plan-review Axis 2, radio same-tree filter → proposed slot; (M4) in-process child `needs_render` → record on `#11-windowproxy-browsing-context`; (M5) a pre-existing wrong event the fixup newly reaches is fixed before or with it | (2) §8; (1) exists because of M5 |
| 260 | The change-on-blur provenance is its own PR and lands before (2) | (1); (2) rev 3 |
| 264 | Owners of the proposed slots (§4); (M2) no split commit bundled in (1) — if `content/event_handlers.rs` is over 1000 lines after (1), a standalone split PR goes before (1); (M3) the `input`-firing fix is in (1), and the Chromium pre-step also measures `input` | (1) rev 2 (pending); §4 |
| 265 | (1) and (2) need an approved umbrella; this memo | this memo |

PM decision 264 (M2) supersedes plan (1)'s D-6 (commit (0), the `text_edit.rs` move) and with it plan (2)'s §0.1
re-base note, which assumes the move (§2 X3). Both per-PR plans apply it in their next revision; this memo does not edit them.

## §1. Program shape

### §1.1 Edge density

Edge-dense by both arms of the CLAUDE.md rule: (a) the axes event dispatch (`input` / `change` / `blur` order),
focus-transition paths (UA sink, `blur_current`, VM `focus()` / `blur()`, the step-17 fixup), script writers and
re-entry, the snapshot lifecycle, and render-phase position all intersect (§2); (b) "the user has changed the
element's value" is prose with no algorithm, and the mixed case is measured, not read. So: a multi-PR program, each
slice plan-reviewed before implementation, each slice a terminal unit under base case (c).

### §1.2 Nested PRs

| nested PR | scope | owner | slots owned / closed | prerequisites |
|---|---|---|---|---|
| **(1) provenance** (`layout-change-on-blur-provenance`) | DC2: one user-edit funnel at the value change (`elidex_form::apply_user_edit` (NEW)); `input` iff the value changed; step 2.1 reads a user-change mark on `FocusValueSnapshot`; the §6 Chromium pre-step picks branch A / B1 / B2 | Layout | owns no registered slot (its defect is DC2, found inside DC1's scope, PM decision 252 (M5) / 260); closes none; registers CP-1 to CP-4 (§4) | this umbrella approved; plan (1) plan-reviewed; if PM decision 264 (M2) triggers, the standalone split PR |
| **(2) focus-fixup** (`layout-focus-fixup-ungated`) | DC1: `focus_fixup_steps` = focusing steps for the viewport, ungated in every `crate::re_render`; `reconcile_focus` deleted; form-owner cache validated at read time; position unchanged relative to style | Layout | owns and closes `#11-focus-fixup-gated-on-session-records`; appends O1 to `#11-focusable-area-being-rendered`; records R-S9, R-A3; registers FF-1 to FF-3 (and FF-4 if Axis 2 picks (Y)) | (1) on `origin/main`; branch rebased onto it; plan (2) plan-reviewed |
| **(3) being-rendered** | DC3: the predicate reads computed style (*being rendered*); the fixup moves after style and layout and after RO delivery; the one-frame `:focus` lag decided (PM decision 242 (1)); C7 / C17 of plan (2) re-stated | Layout (the CV umbrella's CV-focus, `docs/plans/2026-10-content-visibility.md`, which says CV-focus's plan decides the slot's discharge) | owns and closes `#11-focusable-area-being-rendered`, O1 included | (2) landed; the CV umbrella's own edges into CV-focus (A2c, FU) |

Row (3) is **shared** with the CV umbrella: this program fixes only the obligation (O1 and §2 X6–X8) and the edge
(2) → (3). The PR itself, its plan and its other prerequisites are the CV umbrella's. One PR, two umbrellas that each
name one part of it: §5 M1 asks the PM to confirm this.

**Order (PM decisions 260, 246/248, 265):** this umbrella approved → [standalone `event_handlers.rs` split, only if PM
decision 264 (M2) triggers] → **(1) provenance** → **(2) focus-fixup** → **(3) being-rendered**.

### §1.3 Edges

| # | edge | kind | reason |
|---|---|---|---|
| e1 | (1) → (2) | semantic precondition | (2) fires step 2.1 from a script-made non-focusability. Without (1)'s predicate, "VM `b.value = "x"; b.disabled = true`" fires `change` at the next rendering update from script alone, a newly reachable wrong event (PM decision 252 (M5)). Plan (2)'s pin C19 is red until (1) is on its base |
| e2 | (2) → (3) | code predecessor + semantic | (3) moves the call (2) creates (`content::focus::focus_fixup` (NEW) in `crate::re_render`) and keeps (2)'s canonical events; moving a silent `reconcile_focus` instead would skip step 2.1. The move is needed only once the predicate reads style (DC3; PM decisions 246, 248) |
| e3 | split PR → (1) | scheduling, conditional | PM decision 264 (M2): only if `content/event_handlers.rs` is over 1000 lines after (1) |
| x1 | canonical-focus `#11-canonical-focus-update-steps` A2c (`#11-vm-host-synthetic-dom-event-dispatch`) | external, after (1) and (2) | VM `blur()` / `focus()` start running the canonical transition: (1)'s "mark before `input`" becomes observable (record R-1), and a fixup listener's reentrant `focus()` fires its own events (plan (2) I2 × I8). The CV umbrella's CF-1 puts A2c before CV-focus, so before (3) |
| x2 | canonical-focus A3 | external, after (2) | the Window `focus` event at the Document entry of the new chain, which (2) does not fire (plan (2) D-9; PM decision 252 (M1); record R-A3) |
| x3 | CV-focus (CV umbrella) | external, shared row (3) | §1.2 |
| x4 | L3 form domain | external, no order | CP-1 to CP-4, FF-2, FF-3, FF-4 (§4) are L3-owned defects found by (1) and (2); none gates a slice of this program |
| x5 | `#11-windowproxy-browsing-context` | external, after (2) | record R-S9: when cross-document script access lands, re-check whether the in-process child's `needs_render`-gated fixup becomes reachable (plan (2) §4.4 S9) |

Acyclic. No edge points from (2) or (3) back into (1).

### §1.4 Vehicle: this memo travels with (1)

The umbrella is committed on (1)'s branch, not in a separate docs PR. Reasons: (i) e1 makes (2) and (3) wait for (1)
anyway, so a separate docs PR would land earlier without unblocking anything; (ii) (1)'s plan has an open item that
this memo answers (plan (1) §1 / §10 M1, "confirm that a PM decision counts as the umbrella"), and its rev 2 (PM decision
264) is written on the same branch, so the answer and the question land together; (iii) precedent: the EH-attrs umbrella's
revisions travelled with IBP-sandbox N1's PR. Approval is the plan-review of this memo, not its merge: it is reviewed
before (1)'s first code commit. If (1) stalls (the pre-step's STOP row), (2) is blocked by e1 regardless.

## §2. Cross-PR coupled invariants

Only pairs that span two slices (intra-slice pairs are each plan's §2). "(1) I5" = plan (1)'s invariant I5.

- **X1 One step-2.1 reader, every blur path** — (1) I5 × (2) I2: `commit_change_on_blur` → `dispatch_change_on_blur` is the
  only reader of the provenance; (2) adds the fixup path through the same sink method, so the fixup inherits (1)'s
  predicate with no fixup-specific code. Pinned by (1) C1 / C3 (UA, `blur_current`) and (2) C19 (fixup).
- **X2 Snapshot consumed once, mark dies with it** — (1) I3 × (2) I3: (1) changes `take_focus_snapshot`'s return type and
  the `FocusValueSnapshot` fields; (2)'s cells that read it (C6, C20: `take_focus_snapshot(b)` → None) are re-derived at
  (2)'s rebase. A type flip to a non-text kind clears snapshot and mark together ((2) E5 / C20).
- **X3 (2)'s user-edit step goes through (1)'s funnel** — (1) I1 / I2 × (2) C6: after (1), `input` fires iff the value
  changed, so (2)'s C6 must use a value-changing key; its `event_handlers.rs` cite and the `text_edit.rs` re-base note
  are re-derived at the rebased head (PM decision 264 (M2) drops the move).
- **X4 Fixup listeners never mark** — (1) I4 × (2) I4: a `change` / `blur` / `focusout` listener run by the fixup that
  writes `value` is a script writer (no mark), and the snapshot is already taken ((2) I3 × I4). No cell beyond C19.
- **X5 IME composition at a fixup blur** — (1)'s IME-commit routing × (2) I10 / FF-3: a future blur-time commit must
  go through `apply_user_edit` before step 2.1 so it marks and fires `input`; FF-3's text carries this.
- **X6 Placement after the style the predicate reads** — (2) I9 × (3): the fixup runs after the style pass whose
  computed style the predicate reads, and after RO delivery ((2) I5 × I9). Before (3) the predicate is attribute-only,
  so before-style placement gives the same answer (PM decision 246).
- **X7 Same-frame effects re-stated** — (2) I4 / I10 × (3): (2)'s C7 (listener mutation rendered this frame) and C17
  (`:focus` cleared this frame) hold only while the fixup precedes style; (3) re-states both by its PM decision 242 (1)
  choice (re-style the frame, or accept a one-frame lag).
- **X8 One predicate for writer and fixup** — (2) I8 × (3): when (3) makes the fixup's predicate read style, the VM
  `focus()` gate changes in the same PR, or every frame focuses-then-resets with events ((2) I1 × I8).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 17 | focused area not a focusable area → focusing steps for the viewport, ungated, canonical events | (2): `crate::re_render` in `crates/shell/elidex-shell/src/lib.rs` → `focus_fixup_steps` (NEW) in `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | ✓ | yes |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* steps 16 → 17 | style, layout and ResizeObserver before the fixup | (3) (obligation O1); (2) leaves the position, X6 | ✗ | no |
| WHATWG HTML §8.1.7.3 Processing model | *update the rendering* step 2 | documents in tree order, each step over all | not met; proposed FF-1 (owner Layout) | ✗ | no |
| WHATWG HTML §6.6.4 Processing model | focusing steps 1–8, target = viewport; unfocusing steps step 8 | one viewport transition, two callers | (2): `focusing_steps_for_viewport` (NEW), `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1 | `change` iff the user changed the value since focus and value ≠ focus-time value; script-only → none; mixed case by the pre-step | (1): `dispatch_change_on_blur`, `crates/shell/elidex-shell/src/content/focus.rs`; mark on `FocusValueSnapshot`, `crates/dom/elidex-form/src/focus_snapshot.rs` | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 4, Document entry | `focus` at the Window | not fired; external canonical-focus A3 (R-A3) | ✗ | no |
| WHATWG HTML §4.10.5.5 Common event behaviors | `input` when the user changes the value; none for script changes | `input` iff `apply_user_edit` reports a change | (1): `apply_user_edit` (NEW, `elidex-form`); shell key / clipboard / IME handlers | ✓ | yes |
| WHATWG HTML §4.10.11 The textarea element | user change → `input`; no `change` rule in the text | same funnel; change-on-blur kept as browser behaviour, measured | (1) | ✗ | yes |
| WHATWG HTML §6.6.2 Data model | focusable area criteria (being rendered) | predicate reads computed style | (3): `crates/dom/elidex-dom-api/src/focus/predicate.rs` | ✗ | no |
| WHATWG HTML §15.1 Introduction | *being rendered* | the predicate's style read | (3) | ✗ | no |

Numbers and titles: `.claude/tools/webref coverage-map html 8.1.7.3 html 6.6.4 html 6.6.2 html 4.10.5.5 html 4.10.11`
(§8.1.7.3 `#event-loop-processing-model`, §6.6.4 `#focus-processing-model`, §6.6.2 `#data-model`, §4.10.5.5
`#common-input-element-events`, §4.10.11 `#the-textarea-element`); `webref dfn html 'update the rendering'` → §8.1.7.3;
`webref dfn html 'focus update steps'` → §6.6.4; `webref dfn html 'being rendered'` → §15.1 Introduction
`#being-rendered`. Step 17 and step 2.1 texts: `.claude/tools/webref body html update-the-rendering`,
`focus-update-steps` (2026-11-03). Breadth: K = 1, M = 10. Each slice's §3 is the row-level SSoT; this table only
assigns each row to its slice.

### §3.1 User-input touch audit

User input reaches the program through (1)'s funnel (keys, clipboard, IME) and is compared at step 2.1 on every blur
path, the fixup's included (X1). Script reaches it through value writers (never marking, X4) and through the
listeners (2) newly runs inside `crate::re_render`, on the renderer main thread inside the existing dispatch bracket.
No slice adds a parse of user input.

## §4. Slots and records (registered or applied by the PM at each landing)

IDs are proposals. The per-PR plans both number theirs P-1…; here they are prefixed **CP-** (plan (1) §8.1) and
**FF-** (plan (2) §8.1, §4.3) so the program has one namespace. Owners: PM decision 264.

| ID | Slot | Owner | From | Registered at |
|---|---|---|---|---|
| CP-1 | `#11-readonly-selection-edit-bypass` | L3 | plan (1) P-1 | (1) landing |
| CP-2 | `#11-dirty-flag-on-cursor-move` | L3 | plan (1) P-2 | (1) landing |
| CP-3 | `#11-number-input-user-edit-path` | L3 | plan (1) P-3 | (1) landing |
| CP-4 | `#11-select-keyboard-change-event` | L3 | plan (1) P-4 | (1) landing |
| FF-1 | `#11-cross-document-rendering-order` | Layout | plan (2) P-1 (PM decision 252 (M2)) | (2) landing |
| FF-2 | `#11-radio-group-same-tree-filter` | L3 | plan (2) P-2 (PM decision 252 (M3) (b)) | (2) landing |
| FF-3 | `#11-ime-composition-end-on-blur` | L3 (proposed) | plan (2) P-3; carries X5 | (2) landing |
| FF-4 | `#11-form-owner-ecs-component` | L3 | plan (2) §4.3 (Y), only if plan-review Axis 2 picks (Y) | (2) landing |

Records for existing slots: **R-1** → `#11-vm-host-synthetic-dom-event-dispatch` (A2c), **R-2** →
`#11-focus-fixup-gated-on-session-records` (plan (1) §8.2); **R-S9** → `#11-windowproxy-browsing-context`, **R-A3** →
`#11-canonical-focus-update-steps`, **O1** → `#11-focusable-area-being-rendered` (plan (2) §8.2). Texts are the per-PR plans'.

## §5. Open for the PM

- **M1** — Row (3) is shared with the CV umbrella (§1.2): this program owns the obligation and the edge (2) → (3), and
  the CV umbrella owns the PR (CV-focus). Confirm, or name one owner (alternative: (3) is a standalone Layout PR before
  CV-focus, nested here).
- **M2** — PM decision 264 (M2) supersedes plan (1) D-6 and plan (2)'s §0.1 re-base note (§0.3). Confirm that both
  plans' next revisions apply it before their plan-reviews, not at implementation.

## §6. Definition of done (program)

1. This umbrella is plan-reviewed and approved before (1)'s first code commit; each slice's own plan is plan-reviewed
   before its implementation.
2. The landing order of §1.2 holds: (1) on `origin/main` before (2) merges; (2) before (3). (2)'s C19 recorded red
   before its rebase and green after (plan (2) §9 item 8).
3. At each landing the PM registers that slice's slots (§4) and applies its records; `#11-focus-fixup-gated-on-session-records`
   is reported closed at (2)'s landing.
4. The being-rendered obligation O1 (placement after style/layout, RO order, the PM decision 242 (1) lag, X6–X8) is on
   `#11-focusable-area-being-rendered` from (2)'s landing until (3) discharges it.
5. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-11-focus-fixup-program.md` exits 0.
