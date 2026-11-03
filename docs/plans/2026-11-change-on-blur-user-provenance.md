# `change-on-blur-user-provenance` — `change` on blur fires only for a value the user changed (per-PR plan-memo)

This PR is slice **(1) provenance** under the umbrella `docs/plans/2026-11-focus-fixup-program.md` (order, edges,
cross-PR invariants and slot owners: its §1–§4; scope by PM decisions 260 and 264). Branch
`layout-change-on-blur-provenance`, worktree `wt-change-provenance`. Rev 2, 2026-11-03, draft for `/elidex-plan-review`.
Owner Layout; crates `elidex-form`, `elidex-shell`, `elidex-dom-api` (one doc comment).

**Defect.** HTML's focus update steps fire `change` at a blurred control only if "the user has changed the element's
value … while the control was focused without committing that change (such that it is different to what it was when the
control was first focused)", and §4.10.5.5 says `input` and `change` "are not fired in response to changes made to the
values of form controls by scripts". elidex's `dispatch_change_on_blur` (`crates/shell/elidex-shell/src/content/focus.rs:177-193`)
compares the current value with the focus-time snapshot and nothing else, so it cannot tell who changed the value. On
`origin/main`, a script write of `value` to a focused text control, followed by a user blur, fires `change`.

| Sequence (text control `b`, focused by the UA, value `""` at focus) | `origin/main` | after this PR |
|---|---|---|
| script `b.value = "x"`; user focuses `c` | `change` (wrong) | no `change` |
| user types `a`; user focuses `c` | `input`, `change` | `input`, `change` |
| user types `a`, then Backspace; user focuses `c` | `input` ×2, no `change` | same |
| user presses ArrowLeft (value unchanged) | `input` with `inputType ""` (wrong) | nothing |
| user presses ArrowLeft; script `b.value = "x"`; user focuses `c` | `input` (wrong), `change` (wrong) | no `input`, no `change` |
| paste (Cmd+V) into a `readonly` control | `input` (wrong), value unchanged | nothing |
| IME commit into a control whose `maxlength` is full | `compositionend`, `input` (wrong) | `compositionend` only |
| user types `a`; script `b.value = "third"`; user focuses `c` (mixed) | `change` | **measured first** (§6 pre-step) |
| user types `a`; script `b.value = ""` (restores the focus-time value); user focuses `c` | no `change` | no `change` (every branch) |
| script `b.focus()`; user types `a`; user focuses `c` | `input`, `change` | same |

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` below is read at `origin/main` **`d7d34035`** (#582), this branch's base (`git -C <wt> log --oneline -1`
→ `d7d34035`; the branch has only this memo and the umbrella).

### §0.2 Premises re-measured

| ID | Premise | Measurement (re-runnable) |
|---|---|---|
| P1 | The snapshot is an ECS component, `FocusValueSnapshot(pub String)` (`crates/dom/elidex-form/src/focus_snapshot.rs:17`). `record_focus_snapshot` (`:28-45`) inserts it iff `kind.is_text_control()` and removes it otherwise; `take_focus_snapshot` (`:49-54`) reads and removes it; `clear_focus_snapshot` (`:63-65`) is called by the reconciler's `type` arm on a flip to a non-text kind (`crates/dom/elidex-form/src/reconciler.rs:155-162`) | read at `d7d34035` |
| P2 | `is_text_control` = TextInput, Password, TextArea, Email, Url, Tel, Search (`crates/dom/elidex-form-core/src/lib.rs:104-115`). No Number, no button kinds, no Hidden | read |
| P3 | Seeds: the shell sink's `seed_focus_snapshot` (`crates/shell/elidex-shell/src/content/focus.rs:129-134`, step 4.1 of the canonical transition) and VM `focus()` (`crates/script/elidex-js/src/vm/host/html_element_proto.rs:302-311`, bit-level, no events). Consumer: the sink's `commit_change_on_blur` (`focus.rs:121-127`), called at step 2.1 of `focus_update_steps` (`crates/dom/elidex-dom-api/src/focus/update_steps.rs:185`). The sink-driven blur paths are UA `set_focus` (`focus.rs:73`) and `blur_current` (`focus.rs:94`, the unfocusing steps). VM `blur()` is bit-level and leaves the snapshot (`html_element_proto.rs:344-352`, `elidex_dom_api::focus::blur`) | `git grep -n 'record_focus_snapshot\|take_focus_snapshot\|clear_focus_snapshot' -- crates` |
| P4 | `dispatch_change_on_blur` (`focus.rs:177-193`) fires iff a snapshot exists and `fcs.value() != snapshot` | read |
| P5 | Every shell `input` dispatch goes through `dispatch_input_event_typed` (`crates/shell/elidex-shell/src/content/form_input.rs:98-114`). Callers: `event_handlers.rs:575` (text key), `:611` (radio Space, via `dispatch_input_event` `form_input.rs:93`), `:630` (select arrow), `:723` (cut), `:737` (paste), `form_input.rs:81` (checkbox/radio state change), `ime.rs:150` (IME commit) | `git grep -n 'dispatch_input_event' -- crates/shell` → those 7 call sites, the wrapper's own call `form_input.rs:94`, the 2 definitions (`:93`, `:98`) and 2 `use` lines (`event_handlers.rs:17`, `ime.rs:9`), verified 2026-11-03 |
| P6 | The funnel is also reached **without a value change**: `form_control_key_input_action` returns `KeyAction::Consumed` for cursor moves (`crates/dom/elidex-form-core/src/input.rs:688` → `navigate_cursor` `:98-130`) and for readonly navigation (`:80-86`), and the shell fires `input` for every `Consumed` (`event_handlers.rs:561-576`, `inputType` `""` for arrows); paste fires `input` whenever the clipboard has text, though `clipboard_paste` returns early on `readonly` or a full `maxlength` (`crates/dom/elidex-form-core/src/clipboard.rs:38-50`, shell `event_handlers.rs:727-740`); IME commit fires `input` even when the maxlength cap leaves nothing to insert and there was no selection (`ime.rs:108-151`) | read |
| P7 | Script writers of a text control's live value: input `value` setter (`crates/script/elidex-js/src/vm/host/html_input_value.rs:135`), textarea `value` setter (`html_textarea_proto.rs:725`), textarea `defaultValue` on a non-dirty control (`:684-689`), `setRangeText` (`selection_api.rs:334`), the reconciler's `value` arm on a non-dirty control (`reconciler.rs:175-220`), the type change into value mode (`crates/dom/elidex-form/src/value_mode.rs:89`), form reset (`crates/dom/elidex-form/src/submit.rs:478`, reached from script `form.reset()` and from the UA reset button `content/form_input.rs:289-302`). None touches the snapshot | `git grep -n 'set_value(\|set_value_initial(\|set_value_from_content_attr(\|replace_selection(\|reset_value(' -- crates ':!*test*'` |
| P8 | No shell test exercises change-on-blur today: `git grep -n '"change"' -- crates/shell/elidex-shell/src` hits only `content/focus.rs:174/187/188`, `content/form_input.rs:77/82/83` and three `matchMedia` tests in `viewport_tests.rs` | run 2026-11-03 |
| P9 | Shell test entry points: `handle_key` (`event_handlers.rs:266`, `pub(super)`), `handle_ime` (`ime.rs:23`, `pub(super)`), `build_test_content_state` (`content_test_support.rs:118`); test modules are mounted in `content/mod.rs` with `#[path]` (`:791-824`) | read |

### §0.3 Premise errors

- **E1 (brief)** — "the input element's change event rules" are the right source for `<input>`, but the focus update
  steps' step 2.1 begins "If entry is an **input** element"; §4.10.11 The textarea element defines the textarea's `input`
  event and dirty flag and no `change` event (`.claude/tools/webref body html the-textarea-element | /usr/bin/grep -c 'named change'` → 0).
  elidex fires change-on-blur for `<textarea>` too (P2), as browsers do. So for textarea the rule is browser behaviour,
  not spec text: the §6 pre-step measures textarea separately, and §3 marks the row.
- **E2 (brief, Q7 list)** — "spin/step for number": `<input type=number>` has **no user edit path** in the shell. It is not a
  text control (P2), so `handle_key` sends it to `handle_key_widget` (`event_handlers.rs:337-340`), which has no Number arm
  (`:587-645`, `_ => {}`), and `is_editable_text` (`ime.rs:13-20`) rejects it. Typing into a number field does nothing.
  `stepUp()`/`stepDown()` (`html_input_value.rs:448`, `:456`) are script. Recorded as a defect, CP-3 (§8.1).
  "Form reset?": reset is the UA's reset algorithm, not a user edit of the control (§4.10.5.5 fires no `input` for it),
  so it is a non-user writer (P7), not a funnel member.
- **E3 (rev 2's own design)** — rev 2 D-11 put the mark in `dispatch_input_event_typed`, reasoning that the spec couples
  `input` to "the user causes the element's value to change". The coupling holds in the spec, but **not in elidex's
  funnel**: P6 lists three ways it fires without a value change. With the mark there, "ArrowLeft; script writes
  `value`; blur" would still fire the wrong `change`, because the arrow key sets the flag. Rev 2 left this as Q7
  ("does every user writer reach the funnel?"); the measured gap is the other direction: non-writers reach it too.
  D-2 moves the mark to the value change itself.
- **E4 (brief)** — "Today a script `value` write followed by a user blur fires `change` wrongly" is read from the code
  (P4, P7). No test has run it (P8). C1 is the first measurement; red-first on `d7d34035` confirms it.

### §0.4 Decisions

- **D-1 The predicate.** Step 2.1 fires `change` iff the snapshot records a user value change since focus **and** the
  value differs from the focus-time value (the spec's two conditions). The representation follows from the pre-step's
  outcome (§4.3 branches A / B1 / B2); branch A is the default the cells below are written for. One function computes
  it: `elidex_form::take_change_on_blur` (NEW, `focus_snapshot.rs`) consumes the snapshot and returns whether the substep
  fires; every sink only dispatches `change` (umbrella X1, record PR-1).
- **D-2 One user-edit funnel, at the value change** (E3). `elidex_form::apply_user_edit` (NEW), in
  `crates/dom/elidex-form/src/user_edit.rs` (NEW), runs a user edit closure on the control's `FormControlState`, compares the value
  before and after, and if it changed (a) marks the snapshot (if one exists) and (b) returns `changed = true`. The shell
  fires `input` **iff** `changed`. One predicate ("the user changed the value"), two consumers (`input` and the change
  provenance), one place. This also fixes the `input` over-firing of P6, which is the same predicate; in scope by PM
  decision 264 (M3).
- **D-3 The routed paths** (§4.2 Q7 table): text key edits, cut, paste, IME commit, the four user writers. Checkbox, radio
  and select keep their own state-change events (`form_input.rs:80-85`, `event_handlers.rs:604-632`); they carry no
  snapshot (P2), so they are outside this predicate.
- **D-4 Script writers are not touched** in branches A and B2 (P7). Branch B1 adds a re-baseline call at each of P7's
  writers (§4.3).
- **D-5 The dirty value flag is not moved.** It has a different predicate: §4.10.18.1 *A form control's value* sets it for
  user changes **and** for script `value` writes (`set_value` step 3, `crates/dom/elidex-form-core/src/lib.rs:607-610`), so it is
  not "the user changed the value". Its own user-path defect, the flag set by cursor moves (`input.rs:88-90`), is
  recorded as CP-2 (§8.1) and not fixed here.
- **D-6 No split in this PR** (PM decision 264 (M2)). `content/event_handlers.rs` is 965 lines at `d7d34035`; this PR
  wraps its three text-edit writers (rows 1–3 of §4.2: `:550-558` / `:575`, `:712-725`, `:727-740`) in
  `apply_user_edit` and gates their `input` on `changed`, about +15 lines, so ~980 after this PR, under 1000. A standalone
  split PR first is needed only if the final head measures over 1000 (umbrella e3; DoD 5).
- **D-7 Commits.** (1) `elidex-form`: snapshot shape + `apply_user_edit` + `take_change_on_blur` with HEAD's value-only
  predicate + unit cells; the shell's `dispatch_change_on_blur` calls `take_change_on_blur` and only dispatches (no
  behaviour change); (2) shell: the four writers through `apply_user_edit`, `input` iff changed (C8, C10, C11);
  (3) `elidex-form`: `take_change_on_blur` reads the provenance (C1, C3, C9, C14, C16). Each commit builds and passes its
  crates' tests.

## §1. Shape and verdict

**Edge-dense: yes**, by both arms of the CLAUDE.md rule.
(a) **At least 3 intersecting invariant axes.** §2 has seven coupled invariants on four axes that cross: event dispatch
(`input` / `change` order and firing), focus-transition paths (the UA sink, `blur_current`, VM `focus()` / `blur()`, and
the step-17 fixup after the focus-fixup PR), script writers (P7), and the snapshot lifecycle (seed / consume / type-flip
clear / despawn).
(b) **No canonical algorithm.** "The user has changed the element's value … without committing that change" is prose,
not steps, and the mixed case (user edit, then script write) is not decided by the text (§4.3).
So `/elidex-plan-review` is mandatory.

**Single PR.** This PR is a slice under `docs/plans/2026-11-focus-fixup-program.md`, so CLAUDE.md's base case (c)
applies (umbrella §1.2). Inside this PR the parts cannot be split further without a broken intermediate state. The
`input` fix and the provenance share one function (D-2). Landing the funnel alone is a valid change, but it leaves
`apply_user_edit` with one consumer while its doc names two. Landing the provenance alone, with rev 2's funnel, keeps
E3's wrong `change`. Breadth: K = 1, M = 9 (§3) → preflight "single PR scope".

## §2. Coupled invariants

- **I1 One user-edit predicate** — a user action changed the value iff `value` after the action ≠ `value` before it,
  computed only in `apply_user_edit`.
- **I2 `input` iff I1** — the shell fires `input` for a text control exactly when `apply_user_edit` reports a change
  (§4.10.5.5: "Some user interactions do not cause changes to the value … ").
- **I3 Provenance lifetime = snapshot lifetime** — every seed (UA step 4.1, VM `focus()`) starts unmarked; the mark goes
  away with the snapshot (take at step 2.1, reconciler type-flip clear, non-text re-seed, despawn).
- **I4 Script writers never mark** — no P7 writer calls `apply_user_edit` (branch B1: each re-baselines instead).
- **I5 One judgement, called by every sink** (umbrella X1) — `take_change_on_blur` is the only consumer of the snapshot
  and the only place the predicate is computed; every sink calls it from `commit_change_on_blur` and only dispatches
  `change`. Sink paths here: UA `set_focus`, `blur_current` (shell `dispatch_change_on_blur`); the others are the
  umbrella's (X1, R-1).
- **I6 Mark before `input`** — the mark is set inside `apply_user_edit`, before the shell builds the `input` event, so
  any blur caused by an `input` listener sees it.
- **I7 Kind boundary** — only `is_text_control` kinds carry a snapshot; checkbox / radio / select fire their own
  state-change `input` / `change` and do not use the funnel.

Pairs:
- **I1 × I2** — one predicate, two consumers. A funnel that fired `input` for a non-change would also have marked
  (E3). C8, C9 and C11 pin it.
- **I1 × I3** — the mark needs a snapshot to land in. User edits reach only the focused element (keys go to the focused
  target; IME goes to `current_focus`, `ime.rs:26-30`). A focused text control has a snapshot when its focus came
  through a seeding writer (P3). Where a focus writer did not seed, there is nothing to mark and no `change` fires, as at
  HEAD; `note_user_value_change` never inserts a snapshot (C7 (c)).
- **I3 × I4** — branch B1's re-baseline acts only when a snapshot exists; an unfocused control's script write
  has nothing to re-baseline.
- **I3 × I5** — consume-once covers the mark. VM `blur()` leaves a marked snapshot behind (P3), and the next seed
  replaces it (`insert_one`), so a stale mark cannot survive into a new focus (C16).
- **I4 × I5** — script-only changes fire no `change` on any sink path (C1 UA, C3 `blur_current`; the fixup path:
  umbrella X1).
- **I5 × I6** — an `input` listener that blurs synchronously does so through VM `blur()` / `focus()`, which are
  bit-level and fire nothing (A2c, `#11-vm-host-synthetic-dom-event-dispatch`). So the ordering is structural today and
  becomes observable at A2c. No cell; §8 records it for A2c.
- **I2 × I6** — no `input` means no mark, so no listener can observe a half-applied edit.
- **I7 × I1** — the funnel is called only from text-control paths (D-3); a non-text control passed to it has no
  snapshot to mark (C7 (c)).
- **I1 × I4**, **I2 × I3**, **I2 × I4**, **I3 × I6**, **I4 × I6**, **I6 × I7** — none beyond the above.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1 | user changed the value since focus **and** value ≠ focus-time value → `change`; script-only change → none; mixed case per §6 pre-step | `take_change_on_blur` (NEW, `focus_snapshot.rs`), called by the shell sink `dispatch_change_on_blur` (`focus.rs:177`), which only dispatches | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1 (other conjuncts) | "input element", "change event applies", "no defined activation behavior": the snapshot exists only on text controls (P1, P2); unchanged | `record_focus_snapshot` (unchanged) | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1.1 | set user validity → not here, `#11-focus-change-user-validity` | — | ✗ | no |
| WHATWG HTML §4.10.5.5 Common event behaviors | "any time the user causes the element's value to change … fire … input"; "Some user interactions do not cause changes to the value" | `input` iff the value changed (I2) | `apply_user_edit` (NEW), shell text-edit handlers, IME commit | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | "not fired in response to changes made to the values of form controls by scripts" | no P7 writer marks (I4) | P7 writers (unchanged; B1: re-baseline) | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | "change an input element's value on behalf of the user" (prefill) → `input` then `change` directly | no such path in elidex (Q7 table row 10); a future autofill fires both itself, not via blur | — | ✗ | no |
| WHATWG HTML §4.10.5.5 Common event behaviors | "not fired when the browser changes the values … as part of state restoration" | no form state restoration in elidex; nothing to route | — | ✗ | no |
| WHATWG HTML §4.10.11 The textarea element | user changes the raw value → `input`; no `change` rule in the text (E1) | same funnel as `<input>`; change-on-blur kept as browser behaviour, measured in the pre-step | `apply_user_edit`, `take_change_on_blur` | ✗ | yes |
| WHATWG HTML §4.10.18.1 A form control's value | dirty value flag set when the user changes the value | not changed (D-5); cursor moves set it (`input.rs:88-90`), CP-2 | — | ✗ | yes |

Numbers and titles: `.claude/tools/webref coverage-map html 6.6.4 html 4.10.5.5 html 4.10.11` (§6.6.4 Processing model
`#focus-processing-model`; §4.10.5.5 Common event behaviors `#common-input-element-events`; §4.10.11 The textarea
element `#the-textarea-element`); `webref dfn html 'focus update steps'` → §6.6.4 `#focus-update-steps`;
`webref dfn html 'dirty value flag'` → §4.10.18.1 A form control's value `#concept-fe-dirty`. Step texts:
`.claude/tools/webref body html focus-update-steps`, `common-input-element-events`, `the-textarea-element` (2026-11-03).
Breadth: K = 1, M = 9.

### §3.1 User-input touch audit

User input reaches the changed code through keys, clipboard and IME (§4.2). Script reaches it through P7's writers,
which must stay unmarked (I4), and through `input` / `change` / `blur` listeners (I6). Every listener runs on the
renderer main thread, inside `PipelineResult::dispatch_event` (`crates/shell/elidex-shell/src/lib.rs:293`), with no new nesting. The
clipboard text is untrusted user data; its handling (truncation `clipboard.rs:44-55`) is unchanged.

## §4. Mechanism

### §4.1 The funnel and the snapshot — HEAD → this PR

```rust
// crates/dom/elidex-form/src/user_edit.rs (NEW)
/// HTML §4.10.5.5 / §4.10.11: the user causing a text control's value to change.
/// Returns `None` when `entity` has no `FormControlState`.
pub fn apply_user_edit<R>(
    dom: &mut EcsDom,
    entity: Entity,
    edit: impl FnOnce(&mut FormControlState) -> R,
) -> Option<UserEdit<R>>;
pub struct UserEdit<R> { pub result: R, pub changed: bool }
```

Body: borrow the state, `let before = fcs.value().to_owned()`, run `edit`, `changed = fcs.value() != before`, drop the
borrow; if `changed`, call `focus_snapshot::note_user_value_change(dom, entity)` (NEW, `pub(crate)`). The clone costs
O(len), the same order as the edit itself (inserting into or draining from the middle of a `String` shifts the tail).

`FocusValueSnapshot` (`focus_snapshot.rs:17`) becomes `{ initial: String, user_changed: bool }` (branch A; §4.3 for B1 / B2).
`record_focus_snapshot` seeds `user_changed = false`; `note_user_value_change` sets it to true if a snapshot exists and
does nothing otherwise. `take_change_on_blur(dom: &mut EcsDom, entity: Entity) -> bool` (NEW, `focus_snapshot.rs`)
replaces `take_focus_snapshot` (whose only non-test caller was the shell sink; the tests read the component instead):
it removes the snapshot and returns `snap.user_changed && fcs.value() != snap.initial` (branch A; §4.3 for B1 / B2),
`false` without a snapshot or a `FormControlState`. The shell's `dispatch_change_on_blur` dispatches `change` iff it
returns `true`. The `commit_change_on_blur` doc (`crates/dom/elidex-dom-api/src/focus/update_steps.rs:72-76`, "the shell
takes the snapshot, compares, and dispatches") is rewritten to: the sink calls `elidex_form::take_change_on_blur` and
dispatches `change` iff it returns `true`; a no-op sink leaves the snapshot intact. The component stays on the entity,
as now: the side-store rule is met, and despawn cleans it up.

### §4.2 Q7 — every user value writer, and how it reaches the funnel

Method: `git grep -n '&mut FormControlState' -- crates/shell` (14 hits, verified 2026-11-03: `event_handlers.rs:149`, `:388`,
`:527`, `:540`, `:554`, `:625`, `:638`, `:717`, `:733`; `ime.rs:63`, `:120`; `form_input.rs:28` (checkbox); `focus.rs:257`,
`:388` (both in `#[cfg(test)]`)), each read for whether it writes `value`. Then a search for paths that do not exist:
`git grep -n -i 'undo\|redo\|autofill\|prefill' -- crates/shell/elidex-shell/src crates/dom/elidex-form/src crates/dom/elidex-form-core/src`
(no edit path; `reconciler.rs:20/306` are the `autocomplete` attribute), and no `drop` / `dragover` handler in `crates/shell`.

| # | User action | Value writer at HEAD | `input` at HEAD | Over-fires? | This PR |
|---|---|---|---|---|---|
| 1 | Key insert / Backspace / Delete / Enter (textarea) / typed replacement of a selection | `form_control_key_input_action` (`event_handlers.rs:550-558` → `input.rs:30-95`) | `:575` (funnel) | yes: cursor moves and readonly navigation return `Consumed` (P6) | through `apply_user_edit`; `input` iff changed |
| 2 | Cut (Cmd/Ctrl+X) | `clipboard_cut` (`event_handlers.rs:712-725` → `clipboard.rs:20-29`) | `:723`, only if the cut text is non-empty | no | through `apply_user_edit` (clipboard write unchanged) |
| 3 | Paste (Cmd/Ctrl+V) | `clipboard_paste` (`event_handlers.rs:727-740` → `clipboard.rs:38-66`) | `:737`, whenever the clipboard has text | yes: readonly, full `maxlength`, identical replacement | through `apply_user_edit` |
| 4 | IME commit | `delete_selection` + `insert_at_cursor` (`ime.rs:115-139`) | `ime.rs:150` after `compositionend` | yes: empty insert with no selection | through `apply_user_edit`; `compositionend` unchanged, `input` iff changed |
| 5 | IME preedit | writes `composition_text`, not `value` (`ime.rs:59-69`) | none | — | not a value writer; unchanged |
| 6 | Select-all, Shift+Arrow, copy | selection only (`event_handlers.rs:522-548`, `:699-711`) | none | — | unchanged |
| 7 | Typing / Backspace over a selection in a **readonly** control | `input.rs:47-77` edits **before** the readonly check at `:80` | `:575` | — (the value really changes) | through the funnel, so `input` and the mark agree; the readonly bypass itself is a defect, CP-1 |
| 8 | Drag and drop | none (no drop handling in `crates/shell`) | — | — | no path; a future drop-insert path must call `apply_user_edit` (§8 record) |
| 9 | Undo / redo | none | — | — | no path; same record |
| 10 | Autofill / prefill | none | — | — | no path; per §4.10.5.5 it fires `input` + `change` directly, not via blur |
| 11 | Number spin / typing | none: Number never reaches `handle_key_text` (E2) | — | — | no path; CP-3 |
| 12 | Textarea writing-direction change (§4.10.11 fires `input`) | none | — | — | no path |
| 13 | Reset button | `reset_form` → `reset_value` (`form_input.rs:289-302` → `submit.rs:478`) | none (correct) | — | non-user writer, P7; the button click moves focus off the text control first (step 2.1 runs before the reset) |
| 14 | Checkbox / radio / select | `form_input.rs:28`, `elidex_form::toggle_radio`, `navigate_select` (`event_handlers.rs:604-632`) | `:611`, `:630`, `form_input.rs:81` | — | not text controls, no snapshot (I7); unchanged. Observation: select arrow fires `input` without `change`, though §4.10.7 *send select update notifications* fires both, CP-4 |

**Q7 answer.** Every shell path that writes a text control's value on user input (rows 1–4, 7) fires `input` through
`dispatch_input_event_typed`; **none bypasses it**. The funnel's defect is the other direction: rows 1, 3 and 4 also reach
it **without** a value change. So the mark cannot sit there (E3). It sits at the value change (D-2), and `input` follows it.
Rows 8–12 have no path today. Row 13 is a non-user writer. Row 14 is outside the text-control boundary.

### §4.3 The mixed case — branches by the §6 pre-step's outcome

The spec text gives two conditions: the user changed the value while focused, and the value differs from the
focus-time value. It does not say what a later script write does to "the user has changed" (Chromium is said to track
whether the last change was a user edit; that is a claim, which is why it is measured). Measured cases are M0–M5 (§6).

| Outcome | Model | Representation | Script writers (P7) |
|---|---|---|---|
| M1 fires, M4 does not | **A** — a user change since focus counts; later script writes do not undo it | `user_changed: bool` | untouched |
| M1 does not, M4 does not | **B2** — the last change must be the user's; the baseline stays the focus-time value | `last_user_value: Option<String>`; fire iff `value == last_user_value && value != initial` | untouched: a later script write makes `value != last_user_value` (structural, no population) |
| M1 does not, M4 fires | **B1** — a script write re-baselines: `initial = new value`, unmarked | `user_changed: bool` | every P7 writer calls `elidex_form::note_script_value_write` (NEW) when a snapshot exists; a missed writer falls back to A for that writer |
| M0 fires, M2 fires, M1 and M4 both fire, M3 does not fire, or M5 fires | none of the above | — | **STOP**: record the outcome, return to the PM; do not implement |

If `<input>` and `<textarea>` give different outcomes, each kind gets its own branch. The branch is decided per element
at seed time (the kind is known there). The PM is told.

### §4.4 `input` without a value change — by the §6 pre-step's N cases

| Outcome of a case N1–N5 | Design |
|---|---|
| no `input`, value unchanged (expected; §4.10.5.5: "Some user interactions do not cause changes to the value") | I2 as written; C8, C11, C12 assert it |
| `input` fires, value unchanged | I2 kept: unlike the textarea rule (E1) the spec text decides this; the PR body records the divergence, PM told |
| the value changes | Chromium's edit primitive differs from `elidex-form-core`'s (not this PR); recorded as a proposed L3 defect, PM told; I2 unchanged, since it follows elidex's own value |

## §5. Populations and named risks

### §5.1 Touch list (`wc -l` at `d7d34035`)

| File | Lines | Change | Split rule |
|---|---|---|---|
| `crates/shell/elidex-shell/src/content/event_handlers.rs` | 965 | rows 1–3 through `apply_user_edit`, `input` iff changed (~+15) | ~980 < 1000 (D-6) |
| `crates/shell/elidex-shell/src/content/ime.rs` | 161 | IME commit through `apply_user_edit`, `input` iff changed | < 1000 |
| `crates/shell/elidex-shell/src/content/focus.rs` | 397 | `dispatch_change_on_blur` calls `take_change_on_blur`, dispatches only | < 1000 |
| `crates/shell/elidex-shell/src/content/form_input.rs` | 317 | docs of `dispatch_input_event_typed` (callers now gate on `changed`) | < 1000 |
| `crates/shell/elidex-shell/src/content/mod.rs` | 861 | `#[path]` test module (~+3) | < 1000 |
| `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | 550 | `commit_change_on_blur` doc `:72-76` (§4.1) | < 1000 |
| `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs` (NEW) | — | C1–C6, C8–C16 | ~450 |
| `crates/dom/elidex-form/src/user_edit.rs` (NEW) | — | `apply_user_edit`, `UserEdit`, unit cells (C7 (e)–(h)) | ~150 |
| `crates/dom/elidex-form/src/focus_snapshot.rs` | 148 | struct fields, `note_user_value_change`, `take_change_on_blur`, C7 (a)–(d), (i)–(k) | ~240 |
| `crates/dom/elidex-form/src/lib.rs` | — | re-exports (`take_change_on_blur` in, `take_focus_snapshot` out) | — |
| `crates/dom/elidex-form/src/reconciler_tests.rs`, `crates/script/elidex-js/src/vm/tests/tests_html_element_proto.rs` | — | read the component instead of `take_focus_snapshot` (`:229`, `:250`; `:222-232`) | — |

Branch B1 adds the P7 writers' files (seven sites, the P7 row's list, verified 2026-11-03). `crates/dom/elidex-form-core` is not edited; the
primitives stay as they are.

### §5.2 Named risks

- **R1 `input` stops firing for non-changes** (I2). A page that relied on arrow-key `input` events loses them. That is
  spec-correct (§4.10.5.5's own example). No elidex test asserts them (C8 is red-first on HEAD).
- **R2 Fail direction of the funnel.** A future user edit path that skips `apply_user_edit` fires neither `input` nor
  `change`. Both omissions are visible, so the miss is not silent. The §8 record for drag-drop / undo names the funnel.
- **R3 Unmeasured mixed case** — closed by the pre-step before any code; the STOP row of §4.3 makes a surprising outcome
  a PM question, not a guess.
- **R4 Cost** — one value clone per user edit, O(len), the same order as the edit (§4.1).

## §6. Test and coverage plan

**Pre-step (before commit (1)): Chromium measurement.** In the built-in browser of the Claude desktop app (load the
`anthropic-skills:built-in-browser` skill first). Serve a probe page from the scratchpad through a temporary
`.claude/launch.json` (`python3 -m http.server <port> --directory <scratchpad>`) + `preview_start`, because `file://`
pages cannot run page tools. Delete the launch.json and `preview_stop` afterwards. Page: `<input id=b value="">`,
`<textarea id=t></textarea>`, a `readonly` pair `<input id=r readonly value="ab">` / `<textarea id=tr readonly>ab</textarea>`,
a full `maxlength=1` pair `<input id=m maxlength=1 value="z">` / `<textarea id=tm maxlength=1>z</textarea>`, `<input id=src value="p">`,
`<button id=away>`; listeners on every field for `input`, `change`, `blur` log
`{type, isTrusted, inputType, value, activeElement.id}` to a `<pre>` and the console. User steps use the tool's **trusted** click
and keyboard input (`isTrusted === true` in the log is required; a run without it is invalid). Script steps run via the
page tool's JS evaluation, which does not move focus. Blur is a trusted click on `#away`. Cases, each on `#b` and on `#t`,
from a fresh page load:

- **M0** (control, the defect): click field; script `value = "x"`; click `#away` → expected: no `change`.
- **M1** (mixed): click; type `a`; script `value = "third"`; click `#away` → `change`?
- **M2** (restore): click; type `a`; script `value = ""`; click `#away` → expected: no `change`.
- **M3** (positive control): click; script `value = "x"`; type `a`; click `#away` → expected: `change`.
- **M4** (re-baseline probe): click; script `value = "x"`; Backspace (value back to `""`); click `#away` → `change`?
- **M5** (non-change then script, C9): script `value = "ab"` before focus; click field; ArrowLeft; script `value = "x"`;
  click `#away` → expected: no `change`.

`input` cases (§4.4), each on the input and the textarea of its pair; record `input` and the value after:

- **N1** ArrowLeft in `#b` / `#t` holding `ab` (script-set before focus), caret at the end (C8).
- **N2** ArrowLeft in the `readonly` pair, caret at the end (C12).
- **N3** paste into the `readonly` pair; the clipboard is filled first by Cmd+A, Cmd+C in `#src`.
- **N4** paste into the full `maxlength=1` pair, caret at the end.
- **N5** IME commit into the full `maxlength=1` pair, caret at the end (nothing to insert; C11). If the tool cannot drive
  a trusted composition, record "not measured"; the §4.4 first row then rests on the spec text alone.

Record in the PR body: engine and version (`navigator.userAgent`), date, the page source, and each case's log. One engine
is evidence, not proof; the PR body says so. The outcome selects the §4.3 branch (C4 / C6 assert it) and the §4.4 row.

**Pre-step run 1 (2026-11-03, observed).** Substitution: Claude in Chrome was not connected; the run used headless Google
Chrome 156.0.8078.80 (macOS) driven over CDP (user steps `Input.dispatchMouseEvent` / `dispatchKeyEvent` / `insertText`, every
logged event `isTrusted === true`; script steps `Runtime.evaluate`). Same on `<input>` and `<textarea>`: M0 none, **M1 change**, M2
none, M3 change, **M4 change**, M5 none; N1–N4 no `input`; N5 not measured (synthetic composition only). M1 and M4 both fire: the
§4.3 STOP row (PM 303 → PM 305).

**Pre-step run 2 — predictions written before running (PM 305).** Model H: the baseline is the value at focus; a script value
write re-baselines it only while the user has not edited since focus; `change` fires iff the user edited and value ≠ baseline.
Alternative H′: the baseline is the value at the last `change` (or at focus) and a script write re-baselines only before any
user edit ever. Each case on `<input>` and `<textarea>`, fresh page:

- **M6** click; script `x`; type `a`; script `y`; blur → H: change. H′: change.
- **M7** click; type `a`; Backspace; blur → H: none. H′: none.
- **M8** click; type `a`; blur (change); click; script `x`; type `b`; blur → H: change. H′: change.
- **M9** click; type `a`; blur (change); click; script `x`; type `y`; Backspace (value `x`); blur → H: none. H′: change
  (separates H from H′).

**Pre-step run 2 — observed** (same engine and driver as run 1; both kinds identical): M6 change (value `y`); M7 none; M8 change
on both blurs (`a`, then `xb`); M9 `change` on the first blur only, none on the second (value `x`). Every H prediction holds; H′'s
M9 prediction fails. Branch selected: **H = "B1 before the mark, A after it"** (PM 305 (i)).

Red-first: each NEW cell runs on `d7d34035` with the fix absent; "HEAD" is the expected result there. Shell cells drive
`handle_key` / `handle_ime` on a `build_test_content_state` fixture (P9) and observe events through listeners writing to a
global array read back with `eval_script`. UA focus moves use `content::focus::set_focus`.

| Cell | Setup → assertion | HEAD |
|---|---|---|
| C1 (was rev 2 C20) `ua_blur_no_change_for_script_written_value` | `<input id=b>`, `<input id=c>`; UA focus `b`; VM `b.value = "x"`; UA `set_focus(c)` → `b` saw `blur`, no `change` | red |
| C2 `ua_blur_change_for_user_edit` | UA focus `b`; `handle_key` `a`; UA `set_focus(c)` → `["input","change","blur","focusout"]` in that order | green (control) |
| C3 `blur_current_no_change_for_script_written_value` | as C1, blur through `blur_current` (the unfocusing steps) | red |
| C4 (was rev 2 C22) `ua_blur_after_user_edit_then_script_write` | UA focus `b`; key `a`; VM `b.value = "third"`; UA `set_focus(c)` → per §4.3 branch (A: `change`; B1 / B2: none) | A: green; B: red |
| C5 `ua_blur_after_user_edit_then_script_restore` | UA focus `b`; key `a`; VM `b.value = ""`; UA blur → no `change` | green (pin) |
| C6 `ua_blur_after_script_write_then_user_revert` | UA focus `b`; VM `b.value = "x"`; key Backspace (value back to `""`, note below); UA blur → per §4.3 branch (A / B2: no `change`; B1: `change`) | A / B2: green; B1: red |
| C7 (was rev 2 C21; unit) `focus_snapshot.rs` / `user_edit.rs` | (a) seed → unmarked; (b) seed + `apply_user_edit` that changes the value → marked; (c) `apply_user_edit` with no snapshot → no component inserted; (d) re-seed after a mark → unmarked; (e) edit that leaves the value equal → `changed == false`, unmarked; (f) `clipboard_paste` on a readonly state and on a full-`maxlength` state → unchanged; (g) `clipboard_cut` with a selection → changed, result = the cut text; (h) no `FormControlState` → `None`; `take_change_on_blur`: (i) no snapshot → `false`; (j) marked, value ≠ focus-time value → `true`, snapshot removed, a second call → `false`; (k) unmarked with value ≠ focus-time value, and marked with value = focus-time value → `false` | n/a (new API) |
| C8 `arrow_key_fires_no_input` | `<input id=b value="ab">` UA-focused; VM `b.setSelectionRange(1, 1)` (so ArrowLeft moves); `handle_key` ArrowLeft → cursor at 0, no `input` | red |
| C9 `arrow_key_then_script_write_no_change` | as C8, then VM `b.value = "x"`; UA blur → no `change` | red |
| C10 `ime_commit_marks_and_fires_input` | UA focus `b`; `handle_ime(Commit("あ"))` → `compositionend`, `input`; UA blur → `change` | green (control) |
| C11 `ime_commit_with_full_maxlength_fires_no_input` | `<input id=b maxlength=1 value="z">`; UA focus; VM `b.setSelectionRange(1, 1)` (no selection, so nothing is deleted); `Commit("あ")` → `compositionend`, value `"z"`, no `input` | red |
| C12 `readonly_navigation_fires_no_input` | `<input id=b readonly value="ab">`; UA focus; VM `b.setSelectionRange(1, 1)`; ArrowLeft → no `input` | red |
| C13 `vm_focus_then_user_edit_fires_change` | VM `b.focus()`; key `a`; UA `set_focus(c)` → `change` | green (control) |
| C14 `textarea_ua_blur_no_change_for_script_written_value` | C1 on `<textarea id=b>` | red |
| C15 `textarea_user_edit_fires_change` | C2 on `<textarea>` with Enter (`insertLineBreak`) | green (control) |
| C16 `stale_mark_does_not_survive_refocus` | UA focus `b`; key `a`; VM `b.blur()` (snapshot left, P3); VM `b.focus()` (re-seed); VM `b.value = "x"`; UA `set_focus(c)` → no `change` | red |

C6 note: the value goes `""` → `"x"` (script) → `"x"` minus one char = `""` after Backspace with the cursor at the end
(the `value` setter moves the cursor to the end, `crates/dom/elidex-form-core/src/lib.rs:621-623`). Branch A and B2: no `change` (value = focus-time
value); B1: `change` (re-baselined to `"x"`). At HEAD: no `change`.

Mutation guards (final head; each must turn the named cells red, then reverted):

| Guard | Mutation | Red cells |
|---|---|---|
| MG1 (was rev 2 MG8) | `take_change_on_blur` ignores the provenance (HEAD's value-only predicate); the shell cells turning red also show the shell sink's verdict comes from it (I5) | C1, C3, C7 (k), C9, C14, C16 |
| MG2 | mark in `dispatch_input_event_typed` and fire `input` unconditionally (rev 2's design, E3) | C8, C9, C11, C12 |
| MG3 (was rev 2 MG9) | delete the `note_user_value_change` call in `apply_user_edit` | C2, C10, C13, C15 |
| MG4 | `apply_user_edit` returns `changed = true` always | C7 (e)/(f), C8, C11, C12 |
| MG5 | `record_focus_snapshot` keeps an existing mark when re-seeding | C7 (d), C16 |

Coverage: engine-independent cells in `elidex-form` (C7); integration in `elidex-shell` (the rest). Cut and paste have
no shell cell because they read and write the system clipboard (`arboard`, `event_handlers.rs:667-694`), which is not
hermetic in CI. C7 (f)/(g) cover the primitives through the funnel, and the shell call sites are one line each. No
WPT subset is claimed. The change-on-blur WPTs need trusted user input, which the WPT runner does not drive.

## §7. Out of scope and slots

- `user validity` at step 2.1.1 → `#11-focus-change-user-validity` (adjacent; this PR does not set it).
- VM `focus()` / `blur()` events (I5 × I6 becomes observable there) → A2c, `#11-vm-host-synthetic-dom-event-dispatch`.
- The other sinks' calls of `take_change_on_blur` (the fixup, A2c's VM sink) → umbrella X1.
- Readonly selection-edit bypass → CP-1; dirty flag on cursor moves → CP-2; number input has no user edit path
  → CP-3; select keyboard `change` → CP-4 (§8.1).
- Newline normalization on the paste / IME insert paths → `#11-textarea-edit-path-newline-normalization` (same
  handlers, different defect; this PR keeps the inserted text unchanged).

## §8. Edges, hand-offs and records

- **Order and edges**: umbrella §1.2–§1.3 (this PR is (1); e1 into the focus-fixup PR).
- **A2c** (`#11-vm-host-synthetic-dom-event-dispatch`): when VM `blur()` / `focus()` start running the canonical
  transition, an `input` listener's synchronous blur reaches step 2.1, and I6 (mark before `input`) becomes observable.
  A2c's plan should add the cell. Record R-1 below.
- **Future user edit paths** (drag-drop insert, undo / redo, spin buttons): call `elidex_form::apply_user_edit` and fire
  `input` iff `changed` (I2).
- **PR body records**: the pre-step's measurement (§6), the chosen branch, the Q7 table (§4.2), red-first results per cell,
  MG1–MG5 results, §8.1 / §8.2 texts.

### §8.1 Slots CP-1 to CP-4 (defect texts; ID, owner, why, trigger and re-eval: umbrella §4)

Search first: `memory/project_open-defer-slots.md` has no entry for any of the four (`/usr/bin/grep -n -i 'readonly\|dirty_value\|number input\|select.*keyboard\|change-on-blur'`
→ only unrelated `#11-dom-rect-readonly` and `#11-style-readonly-strict-throw`).

- **CP-1 `#11-readonly-selection-edit-bypass`** (crate `elidex-form-core`). *Defect:* a `readonly` text control
  with a selection accepts typing and Backspace / Delete, because `form_control_key_input_action`'s selection branch
  (`input.rs:47-77`) runs before the readonly check (`:80`); §4.10.5.3.3: "When specified, the element is not mutable".
- **CP-2 `#11-dirty-flag-on-cursor-move`** (crate `elidex-form-core`). *Defect:* `input.rs:88-90` sets the dirty
  value flag for every `Consumed` key, cursor moves included, so an arrow key stops later `value`-attribute / `defaultValue`
  writes from updating the live value (§4.10.18.1: set when the user "changes the value"). *Fix shape:* split
  `KeyAction::Consumed` into an edit and a move.
- **CP-3 `#11-number-input-user-edit-path`** (crates `elidex-shell`, `elidex-form-core`). *Defect:* typing into
  `<input type=number>` does nothing. Number is not `is_text_control`, so `handle_key` routes it to `handle_key_widget`
  (no Number arm, `event_handlers.rs:587-645`), and IME rejects it (`ime.rs:13-20`). `form_control_key_input_action`'s
  Number arm is unreachable from the shell. Change-on-blur for number (no snapshot) follows from the same gap. The fix
  routes it through `apply_user_edit`.
- **CP-4 `#11-select-keyboard-change-event`** (crate `elidex-shell`). *Defect:* ArrowUp / ArrowDown on a
  `<select>` fires `input` only (`event_handlers.rs:619-632`); §4.10.7 *send select update notifications* fires `input`
  then `change`.

### §8.2 Records for existing slots

- **R-1 → `#11-vm-host-synthetic-dom-event-dispatch` (A2c)**: "When VM blur()/focus() run the canonical transition, add a
  cell: an `input` listener blurs synchronously and the blur fires `change` (the change-on-blur provenance mark is set
  before `input`, docs/plans/2026-11-change-on-blur-user-provenance.md I6). A2c's VM sink calls
  `elidex_form::take_change_on_blur` (NEW) and only dispatches `change`; `vm/host` does not compute the predicate
  (Layering mandate; umbrella X1)."
- **R-2 → `#11-focus-fixup-gated-on-session-records`**: "Depends on the change-on-blur provenance PR
  (docs/plans/2026-11-change-on-blur-user-provenance.md), which lands first (PM 260)."

## §9. Definition of done

1. The §6 pre-step is run and recorded, and its outcome is in §4.3's table (not a STOP row) and §4.4's before commit (1).
2. `git grep -n 'dispatch_input_event_typed(' -- crates/shell/elidex-shell/src/content/event_handlers.rs crates/shell/elidex-shell/src/content/ime.rs`
   → every hit except the select arrow's (§4.2 row 14) is guarded by an `apply_user_edit` result's `changed`.
3. `git grep -n 'apply_user_edit' -- crates/script` → 0 (no script writer marks, I4); branch B1:
   `git grep -n 'note_script_value_write' -- crates` → one hit per P7 writer.
4. Cells C1–C16 green on the final head (C4 / C6 per branch); NEW cells red / green on `d7d34035` as tabled (results in
   the PR body); MG1–MG5 each red on their cells, then reverted.
5. No touched file over 1000 lines (`event_handlers.rs` estimated ~980, D-6); over 1000 → stop, the split PR lands first (umbrella e3).
6. `cargo fmt --all`; `cargo test -p elidex-form -p elidex-shell -p elidex-js --all-features`; `mise run ci` green
   (`grep 'ERROR task failed'` on its log → none).
7. PR body carries §8's records (applied at landing: umbrella §6 item 3).

## §10. Open items

None for the PM. Rev 1's M1 is answered by the umbrella (§1); M2 and M3 are decided by PM decision 264 (D-6, D-2).

No questions for scout: Q7 is answered in §4.2, and rev 2's Q8 (the script-writer population) is P7, needed only
under branch B1.

## Changelog

- **Rev 2 (2026-11-03)** — umbrella `docs/plans/2026-11-focus-fixup-program.md` records PR-1 / PR-2 and PM decision
  264: the change-on-blur judgement moves into `elidex_form::take_change_on_blur` (D-1, I5, §4.1, §5.1, C7 (i)–(k), MG1,
  R-1; `update_steps.rs:72-76` doc); commit (0) dropped, `event_handlers.rs` estimate ~980 (D-6, D-7, §5.1, DoD 2 / 5);
  pre-step gains M5 and the `input` cases N1–N5 with §4.4's branches; header / §1 / §8 / §10 point to the umbrella;
  slots renamed CP-1 to CP-4 with owner / trigger / re-eval left to umbrella §4; D-7 commit (3)'s cells corrected (C14 for
  C15); the plan (2) rev-2 cite carry-over note in §0.1 removed.
- **Rev 1 (2026-11-03)** — initial draft.
