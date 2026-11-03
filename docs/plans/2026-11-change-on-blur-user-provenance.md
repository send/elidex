# `change-on-blur-user-provenance` — `change` on blur fires only for a value the user changed (per-PR plan-memo)

This PR is slice **(1) provenance** under the umbrella `docs/plans/2026-11-focus-fixup-program.md` (order, edges,
cross-PR invariants and slot owners: its §1–§4; scope by PM decisions 260 and 264). Branch
`layout-change-on-blur-provenance`, worktree `wt-change-provenance`. Rev 3, 2026-11-03, draft for `/elidex-plan-review`.
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
| user types `a`; script `b.value = "third"`; user focuses `c` (mixed) | `change` | `change` (Chromium M1) |
| user types `a`; script `b.value = ""` (restores the focus-time value); user focuses `c` | no `change` | no `change` |
| script `b.value = "x"`; user Backspace (value back to `""`); user focuses `c` | no `change` (wrong) | `change` (Chromium M4) |
| script `b.value = "x"`; user types `y`, then Backspace (value `"x"`); user focuses `c` | `change` (wrong) | no `change` (Chromium M9) |
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
| P7 | Non-user writers of a focused text control's live value: eight families W1–W8, defined by property and enumerated in §5.2. None touches the snapshot | §5.2's method |
| P8 | No shell test exercises change-on-blur today: `git grep -n '"change"' -- crates/shell/elidex-shell/src` hits only `content/focus.rs:174/187/188`, `content/form_input.rs:77/82/83` and three `matchMedia` tests in `viewport_tests.rs` | run 2026-11-03 |
| P9 | Shell test entry points: `handle_key` (`event_handlers.rs:266`, `pub(super)`), `handle_ime` (`ime.rs:23`, `pub(super)`), `build_test_content_state` (`content_test_support.rs:118`, mounted `#[cfg(test)]` at `content/mod.rs:786-787`); test modules are mounted in `content/mod.rs` with `#[path]` (`:791-824`) | read |

### §0.3 Premise errors

- **E1 (brief)** — "the input element's change event rules" are the right source for `<input>`, but the focus update
  steps' step 2.1 begins "If entry is an **input** element"; §4.10.11 The textarea element defines the textarea's `input`
  event and dirty flag and no `change` event (`.claude/tools/webref body html the-textarea-element | /usr/bin/grep -c 'named change'` → 0).
  elidex fires change-on-blur for `<textarea>` too (P2), as browsers do. So for textarea the rule is browser behaviour,
  not spec text: the §6 pre-step measured textarea separately (identical to `<input>` in every case), and §3 marks the row.
- **E2 (brief, Q7 list)** — "spin/step for number": `<input type=number>` has **no user edit path** in the shell. It is not a
  text control (P2), so `handle_key` sends it to `handle_key_widget` (`event_handlers.rs:337-340`), which has no Number arm
  (`:587-645`, `_ => {}`), and `is_editable_text` (`ime.rs:13-20`) rejects it. Typing into a number field does nothing.
  `stepUp()`/`stepDown()` (`html_input_value.rs:448`, `:456`) are script. Recorded as a defect, CP-3 (§8.1).
  "Form reset?": reset is the UA's reset algorithm, not a user edit of the control (§4.10.5.5 fires no `input` for it),
  so it is a non-user writer (W8, §5.2), not a funnel member.
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
  value differs from the baseline: the focus-time value, re-baselined by a non-user write made before the user's first
  value change (model H, §4.3; measured in Chromium, §6; PM 305). One function computes it: `elidex_form::take_change_on_blur`
  (NEW, `focus_snapshot.rs`) consumes the snapshot and returns whether the substep fires; every sink only dispatches
  `change` (umbrella X1, record PR-1).
- **D-2 One user-edit funnel, at the value change** (E3). `elidex_form::apply_user_edit` (NEW), in
  `crates/dom/elidex-form/src/user_edit.rs` (NEW), runs a user edit closure on the control's `FormControlState`, compares the value
  before and after, and if it changed (a) marks the snapshot (if one exists) and (b) returns `changed = true`. The shell
  fires `input` **iff** `changed`. One predicate ("the user changed the value"), two consumers (`input` and the change
  provenance), one place. This also fixes the `input` over-firing of P6, which is the same predicate; in scope by PM
  decision 264 (M3).
- **D-3 The routed paths** (§4.2 Q7 table): text key edits, cut, paste, IME commit, the four user writers. Checkbox, radio
  and select keep their own state-change events (`form_input.rs:80-85`, `event_handlers.rs:604-632`); they carry no
  snapshot (P2), so they are outside this predicate.
- **D-4 Non-user writers are not touched** (W1–W8, §5.2). H's re-baseline is taken once, at the first user value change,
  inside `apply_user_edit` (§4.3), so no writer calls anything and an unlisted or future writer cannot skip it.
- **D-5 The dirty value flag is not moved.** It has a different predicate: §4.10.18.1 *A form control's value* sets it for
  user changes **and** for script `value` writes (`set_value` step 3, `crates/dom/elidex-form-core/src/lib.rs:607-610`), so it is
  not "the user changed the value". Its own user-path defect, the flag set by cursor moves (`input.rs:88-90`), is
  recorded as CP-2 (§8.1) and not fixed here.
- **D-6 No split in this PR** (PM decision 264 (M2)). `content/event_handlers.rs` is 965 lines at `d7d34035`; this PR
  wraps its three text-edit writers (rows 1–3 of §4.2: `:550-558` / `:575`, `:712-725`, `:727-740`) in
  `apply_user_edit` and gates their `input` on `changed` (about +15 lines) and adds the test clipboard seam (§6, about
  +6), so ~986 after this PR, under 1000. A standalone split PR first is needed only if the final head measures over
  1000 (umbrella e3; DoD 5).
- **D-7 Commits** — each commit builds, and every cell it adds passes in it:
  1. `elidex-form` + shell sink + doc: `FocusValueSnapshot { initial, user_changed }`; `apply_user_edit` and
     `note_user_value_change`, which marks but does not yet re-baseline; `take_change_on_blur` with HEAD's value-only
     predicate (`value != initial`, mark ignored), so no behaviour changes; the shell's `dispatch_change_on_blur` calls it
     and only dispatches; the `commit_change_on_blur` doc (§4.1). `take_focus_snapshot` and its `lib.rs` re-export are
     removed here, and every caller moves in this commit: `focus_snapshot.rs:95-143` (unit tests),
     `reconciler_tests.rs:229`, `:250`, `crates/script/elidex-js/src/vm/tests/tests_html_element_proto.rs:232` (read the
     component instead). Cells: C7 (a)–(j); shell C2, C4, C5, C10, C13, C15, C17, C18, C19 (green at HEAD).
  2. Shell: rows 1–4 of §4.2 through `apply_user_edit`, `input` iff `changed`; the test clipboard seam (§6). Cells: C8,
     C11, C12, C27, C28, C29.
  3. `elidex-form`: `take_change_on_blur` reads the mark, and the first mark re-baselines (§4.3). Cells: C1, C3, C6, C9,
     C14, C16, C20–C26, C7 (k), (l).

## §1. Shape and verdict

**Edge-dense: yes**, by both arms of the CLAUDE.md rule.
(a) **At least 3 intersecting invariant axes.** §2 has seven coupled invariants on four axes that cross: event dispatch
(`input` / `change` order and firing), focus-transition paths (the UA sink, `blur_current`, VM `focus()` / `blur()`, and
the step-17 fixup after the focus-fixup PR), non-user writers (§5.2), and the snapshot lifecycle (seed / mark / consume /
type-flip clear / despawn).
(b) **No canonical algorithm.** "The user has changed the element's value … without committing that change" is prose,
not steps, and the mixed case (user edit, then script write) is not decided by the text (§4.3; settled by measurement).
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
- **I3 Provenance lifetime = snapshot lifetime** — every seed (UA step 4.1, VM `focus()`) starts unmarked with
  `initial` = the value; the first user value change sets `initial` to the value just before it and marks; later
  changes of either kind touch neither field. The mark goes away with the snapshot (take at step 2.1, reconciler
  type-flip clear, non-text re-seed, despawn).
- **I4 Non-user writers never mark** — no §5.2 writer calls `apply_user_edit`.
- **I5 One judgement, called by every sink** (umbrella X1) — `take_change_on_blur` is the only consumer of the snapshot
  and the only place the predicate is computed; every sink calls it from `commit_change_on_blur` and only dispatches
  `change`. Sink paths here: UA `set_focus`, `blur_current` (shell `dispatch_change_on_blur`); the others are the
  umbrella's (X1, R-1).
- **I6 Mark before `input`** — the mark is set inside `apply_user_edit`, before the shell builds the `input` event, so
  any blur caused by an `input` listener sees it.
- **I7 Kind boundary** — only `is_text_control` kinds carry a snapshot; checkbox / radio / select fire their own
  state-change `input` / `change` and do not use the funnel.

ECS-native check: the provenance lives as fields of the existing `FocusValueSnapshot` component on the control entity;
no side-store is added, despawn removes it, and no per-VM handle is involved.

Pairs:
- **I1 × I2** — one predicate, two consumers. A funnel that fired `input` for a non-change would also have marked
  (E3). C8, C9 and C11 pin it.
- **I1 × I3** — the mark needs a snapshot to land in. User edits reach the element focused when the key arrived (the
  target is fixed before `keydown` dispatch, `event_handlers.rs:284`, `:328-338`; IME goes to `current_focus`,
  `ime.rs:26-30`). A focused text control has a snapshot when its focus came through a seeding writer (P3). Where a
  focus writer did not seed, there is nothing to mark and no `change` fires, as at HEAD; `note_user_value_change` never
  inserts a snapshot (C7 (c)). Known and harmless: a `keydown` listener that moves focus has already run step 2.1 for
  the target, so the edit lands on a control without a snapshot and marks nothing; its next focus re-seeds.
- **I3 × I4** — a non-user write before the mark is absorbed by the first mark's re-baseline (M4, M9); one after the
  mark is not (M1, M2); an unfocused control's write has no snapshot to affect.
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
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1 | user changed the value since focus **and** value ≠ baseline → `change`; script-only change → none; a script write before the user's first change re-baselines (H, §4.3) | `take_change_on_blur` (NEW, `focus_snapshot.rs`), called by the shell sink `dispatch_change_on_blur` (`focus.rs:177`), which only dispatches | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1 (other conjuncts) | "input element", "change event applies", "no defined activation behavior": the snapshot exists only on text controls (P1, P2); unchanged | `record_focus_snapshot` (unchanged) | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | focus update steps step 2.1.1 | set user validity → not here, `#11-focus-change-user-validity` | — | ✗ | no |
| WHATWG HTML §4.10.5.5 Common event behaviors | "any time the user causes the element's value to change … fire … input"; "Some user interactions do not cause changes to the value" | `input` iff the value changed (I2) | `apply_user_edit` (NEW), shell text-edit handlers, IME commit | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | "not fired in response to changes made to the values of form controls by scripts" | no §5.2 writer marks (I4) | §5.2 writers (unchanged) | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | "change an input element's value on behalf of the user" (prefill) → `input` then `change` directly | no such path in elidex (Q7 table row 10); a future autofill fires both itself, not via blur | — | ✗ | no |
| WHATWG HTML §4.10.5.5 Common event behaviors | "not fired when the browser changes the values … as part of state restoration" | no form state restoration in elidex; nothing to route | — | ✗ | no |
| WHATWG HTML §4.10.11 The textarea element | user changes the raw value → `input`; no `change` rule in the text (E1) | same funnel as `<input>`; change-on-blur kept as browser behaviour, measured identical to `<input>` (§6) | `apply_user_edit`, `take_change_on_blur` | ✗ | yes |
| WHATWG HTML §4.10.18.1 A form control's value | dirty value flag set when the user changes the value | not changed (D-5); cursor moves set it (`input.rs:88-90`), CP-2 | — | ✗ | yes |

Numbers and titles: `.claude/tools/webref coverage-map html 6.6.4 html 4.10.5.5 html 4.10.11` (§6.6.4 Processing model
`#focus-processing-model`; §4.10.5.5 Common event behaviors `#common-input-element-events`; §4.10.11 The textarea
element `#the-textarea-element`); `webref dfn html 'focus update steps'` → §6.6.4 `#focus-update-steps`;
`webref dfn html 'dirty value flag'` → §4.10.18.1 A form control's value `#concept-fe-dirty`. Step texts:
`.claude/tools/webref body html focus-update-steps`, `common-input-element-events`, `the-textarea-element` (2026-11-03).
Breadth: K = 1, M = 9.

### §3.1 User-input touch audit

User input reaches the changed code through keys, clipboard and IME (§4.2). Script reaches it through §5.2's writers,
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
borrow; if `changed`, call `focus_snapshot::note_user_value_change(dom, entity, before)` (NEW, `pub(crate)`). The clone
costs O(len), the same order as the edit itself (inserting into or draining from the middle of a `String` shifts the
tail). The doc comment of `apply_user_edit` names the obligation of future user edit paths (§8).

`FocusValueSnapshot` (`focus_snapshot.rs:17`) becomes `{ initial: String, user_changed: bool }`: one shape for both kinds,
since `<input>` and `<textarea>` measured identical (§6), so no per-kind enum. `record_focus_snapshot` seeds
`initial` = the value, `user_changed = false`. `note_user_value_change` does nothing without a snapshot; on an unmarked
one it sets `initial = before` and `user_changed = true`; on a marked one it does nothing (§4.3).
`take_change_on_blur(dom: &mut EcsDom, entity: Entity) -> bool` (NEW, `focus_snapshot.rs`) replaces
`take_focus_snapshot` (whose only non-test caller was the shell sink; the tests read the component instead, D-7 (1)):
it removes the snapshot and returns `snap.user_changed && fcs.value() != snap.initial`, `false` without a snapshot or a
`FormControlState`. The shell's `dispatch_change_on_blur` dispatches `change` iff it returns `true`. The
`commit_change_on_blur` doc (`crates/dom/elidex-dom-api/src/focus/update_steps.rs:72-76`, "the shell takes the snapshot,
compares, and dispatches") is rewritten to: the sink calls `elidex_form::take_change_on_blur` and dispatches `change`
iff it returns `true`; a no-op sink leaves the snapshot intact. The component stays on the entity, as now: the
side-store rule is met, and despawn cleans it up.

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
| 8 | Drag and drop | none (no drop handling in `crates/shell`) | — | — | no path; a future drop-insert path must call `apply_user_edit` (§8) |
| 9 | Undo / redo | none | — | — | no path; same (§8) |
| 10 | Autofill / prefill | none | — | — | no path; per §4.10.5.5 it fires `input` + `change` directly, not via blur |
| 11 | Number spin / typing | none: Number never reaches `handle_key_text` (E2) | — | — | no path; CP-3 |
| 12 | Textarea writing-direction change (§4.10.11 fires `input`) | none | — | — | no path; same (§8) |
| 13 | Reset button | `reset_form` → `reset_value` (`form_input.rs:289-302` → `submit.rs:478`) | none (correct) | — | non-user writer, W8 (§5.2); the button click moves focus off the text control first (step 2.1 runs before the reset) |
| 14 | Checkbox / radio / select | `form_input.rs:28`, `elidex_form::toggle_radio`, `navigate_select` (`event_handlers.rs:604-632`) | `:611`, `:630`, `form_input.rs:81` | — | not text controls, no snapshot (I7); unchanged. Observation: select arrow fires `input` without `change`, though §4.10.7 *send select update notifications* fires both, CP-4 |

**Q7 answer.** Every shell path that writes a text control's value on user input (rows 1–4, 7) fires `input` through
`dispatch_input_event_typed`; **none bypasses it**. The funnel's defect is the other direction: rows 1, 3 and 4 also reach
it **without** a value change. So the mark cannot sit there (E3). It sits at the value change (D-2), and `input` follows it.
Rows 8–12 have no path today. Row 13 is a non-user writer. Row 14 is outside the text-control boundary.

### §4.3 The mixed case — model H (measured, PM 305)

The spec text gives two conditions: the user changed the value while focused, and the value differs from the
focus-time value. It does not say what a script write does to either. Chromium 156 (§6 runs 1–2; `<input>` and
`<textarea>` identical in every case) follows **H**: the baseline is the value at focus; a script value write
re-baselines it only while the user has not changed the value since focus (the mark); after the mark, `change` fires
iff the value ≠ baseline. In rev 2's terms: B1 before the mark, A after it.

| Event | `initial` | `user_changed` |
|---|---|---|
| seed (UA step 4.1, VM `focus()`) | the value | false |
| non-user write (W1–W8, §5.2), unmarked | unchanged (absorbed at the mark, below) | false |
| first user value change (`apply_user_edit`, `changed`) | the value just before that edit | true |
| any later change, user or non-user | unchanged | true |
| `take_change_on_blur` | removed | removed; returns `user_changed && value != initial` |

**Re-baseline at the mark, not at each writer.** While unmarked, the value changes only through non-user writes (a user
change would have marked), and under H each of them re-baselines; so under H the baseline equals the current value at
every unmarked moment, and at the first user value change it is the value just before that edit — the `before` that
`apply_user_edit` already holds (§4.1). Setting `initial = before` at the mark is therefore H exactly, with no call at
any writer. An unlisted or future non-user writer cannot skip the re-baseline, because nothing is asked of it: anything
that does not go through `apply_user_edit` is non-user, the fail-safe direction. While unmarked, `initial` keeps the seed
value and is never read (`take_change_on_blur` returns `false`); it is HEAD's baseline, which commit (1)'s value-only
predicate and MG1 use.

| Case (§6) | Sequence (fresh focus, value `""` unless noted) | Chromium | `initial` at blur | Cell |
|---|---|---|---|---|
| M0 | script `x` | none | unmarked | C1, C3, C14 |
| M1 | type `a`; script `third` | `change` | `""` | C4 |
| M2 | type `a`; script `""` | none | `""` | C5 |
| M3 | script `x`; type `a` | `change` | `"x"` | C17 (same prefix) |
| M4 | script `x`; Backspace | `change` | `"x"` | C6 |
| M5 | script `ab` before focus; ArrowLeft; script `x` | none | unmarked | C9 |
| M6 | script `x`; type `a`; script `y` | `change` | `"x"` | C17 |
| M7 | type `a`; Backspace | none | `""` | C18 |
| M8 | type `a`; blur; focus; script `x`; type `b`; blur | `change` on both blurs | `""`, then `"x"` | C19 |
| M9 | type `a`; blur; focus; script `x`; type `y`; Backspace (value `x`); blur | `change` on the first blur only | `""`, then `"x"` | C20 |

### §4.4 `input` without a value change (§6 N cases)

Measured (run 1, both kinds): N1–N4 fire no `input` and leave the value unchanged (N3 / N4 inserted through the driver's
`Input.insertText`, not a clipboard paste), so I2 stands as designed; C8, C11, C12, C27 and C28 assert it (§4.10.5.5:
"Some user interactions do not cause changes to the value"). N5 (an IME commit with nothing to insert) was not measured:
the driver had only a synthetic composition, so C11 rests on the spec text. Replacing a selection with identical text
was not measured either; I2 follows elidex's own value comparison there (no change, no `input`).

## §5. Populations and named risks

### §5.1 Touch list (`wc -l` at `d7d34035`)

| File | Lines | Change | Split rule |
|---|---|---|---|
| `crates/shell/elidex-shell/src/content/event_handlers.rs` | 965 | rows 1–3 through `apply_user_edit`, `input` iff changed (~+15); `#[cfg(test)]` clipboard seam in `get_clipboard_text` / `set_clipboard_text` (~+6) | ~986 < 1000 (D-6) |
| `crates/shell/elidex-shell/src/content/ime.rs` | 161 | IME commit through `apply_user_edit`, `input` iff changed | < 1000 |
| `crates/shell/elidex-shell/src/content/focus.rs` | 397 | `dispatch_change_on_blur` calls `take_change_on_blur`, dispatches only | < 1000 |
| `crates/shell/elidex-shell/src/content/form_input.rs` | 317 | docs of `dispatch_input_event_typed` (callers now gate on `changed`) | < 1000 |
| `crates/shell/elidex-shell/src/content/mod.rs` | 861 | `#[path]` test module (~+3) | < 1000 |
| `crates/shell/elidex-shell/src/content_test_support.rs` | — | the thread-local test clipboard (§6) | < 1000 |
| `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | 550 | `commit_change_on_blur` doc `:72-76` (§4.1) | < 1000 |
| `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs` (NEW) | — | C1–C6, C8–C29 | ~700 |
| `crates/dom/elidex-form/src/user_edit.rs` (NEW) | — | `apply_user_edit`, `UserEdit`, unit cells (C7 (e)–(h)) | ~150 |
| `crates/dom/elidex-form/src/focus_snapshot.rs` | 148 | struct fields, `note_user_value_change`, `take_change_on_blur`, C7 (a)–(d), (i)–(l); its tests migrated off `take_focus_snapshot` | ~260 |
| `crates/dom/elidex-form/src/lib.rs` | — | re-exports (`take_change_on_blur` in, `take_focus_snapshot` out) | — |
| `crates/dom/elidex-form/src/reconciler_tests.rs`, `crates/script/elidex-js/src/vm/tests/tests_html_element_proto.rs` | — | read the component instead of `take_focus_snapshot` (`:229`, `:250`; `:232`) | — |

No non-user writer's file is edited (D-4). `crates/dom/elidex-form-core` is not edited; the primitives stay as they are.

### §5.2 Non-user writers of a focused text control's live value (by property)

Population: every path by which anything other than a user edit changes the live value of a text control that can hold
a snapshot (P1, P2). Method (re-runnable): `FormControlState::value` is `pub` (`crates/dom/elidex-form-core/src/lib.rs:352`),
so every writer holds a `&mut FormControlState`:
`git grep -n 'get::<&mut FormControlState>' d7d34035 -- crates ':!*_tests.rs' ':!*/tests/*'` → 52 hits (2026-11-03).
15 are in `#[cfg(test)]` modules (`clone.rs:189/210/238/263`, `focus_snapshot.rs:122`, `radio.rs:267/283`,
`value_mode.rs:150/226/244/262/281/301`, shell `focus.rs:257/388`); 12 are the shell's user-input sites (§4.2:
`event_handlers.rs` ×9, `ime.rs:63/120`, `form_input.rs:28`); the other 25 were each read. `create_form_control_state`
(`init.rs:63`) attaches a new state and writes no live value of an existing control.

| W | Non-user writer | Write site | Cell |
|---|---|---|---|
| W1 | input `value` setter (value mode) | `crates/script/elidex-js/src/vm/host/html_input_value.rs:135` (`set_value`) | C20 (input) |
| W2 | textarea `value` setter | `crates/script/elidex-js/src/vm/host/html_textarea_proto.rs:725` | C20 (textarea) |
| W3 | textarea `defaultValue` setter on a non-dirty control | `html_textarea_proto.rs:687` (`set_value_initial`, inside `:684-689`) | C21 |
| W4 | `value` content attribute set or removed on a non-dirty input | `crates/dom/elidex-form/src/reconciler.rs:230-232` | C22 |
| W5 | `type` change between text kinds (value sanitization under the new kind, e.g. text → url trims) | `reconciler.rs:152` → `sanitize_for_type_change` → `crates/dom/elidex-form-core/src/input.rs:639` (`settle_value`); the snapshot survives (`reconciler.rs:160`) | C23 |
| W6 | Email `multiple` set or removed (re-sanitization) | `reconciler.rs:288` | C24 |
| W7 | `setRangeText` | `crates/script/elidex-js/src/vm/host/selection_api.rs:334` (`replace_selection`) | C25 |
| W8 | form reset: script `form.reset()`, and the UA reset button (`content/form_input.rs:289-302` → `:299`) | `crates/dom/elidex-form/src/submit.rs:478` (`reset_value`) | C26 (`form.reset()`) |

Outside the population (18 sites = 25 − W1–W8's 7 sites, verified 2026-11-03): `value_mode.rs:88-89` (type change into value mode: the old kind is not in
value mode, so not a text control and holds no snapshot; §7's seed gap), `value_mode.rs:66` (`default_value` only),
`:97` (filename); `html_input_value.rs:154` (File), `:233`, `:298` (checkedness, indeterminate), `:349`
(`valueAsNumber`, Number / Range), `:425` (`stepUp` / `stepDown`, which do not apply to text kinds);
`selection_api.rs:176/207/245/257/286` (selection only); `validity_state.rs:513` (custom validity); `clone.rs:119` (the
clone, a new unfocused entity); `fieldset.rs:62` (disabled); `radio.rs:80/89` (checkedness). `reconciler.rs:148` and
`:166` host W4–W6; their other arms write no live value (the dirty `value` branch only re-corrects Range).

Under H every family re-baselines before the mark and none marks (I4); no family calls anything new (D-4). Only W1 /
W2 were measured in Chromium; W3–W8 follow elidex's rule that every non-user change is alike (Q-1, §10).

### §5.3 Named risks

- **R1 `input` stops firing for non-changes** (I2). A page that relied on arrow-key `input` events loses them. That is
  spec-correct (§4.10.5.5's own example). No elidex test asserts them (C8 is red-first on HEAD).
- **R2 Fail direction of the funnel.** A future user edit path that skips `apply_user_edit` fires neither `input` nor
  `change`. Both omissions are visible, so the miss is not silent. `apply_user_edit`'s doc comment names the paths (§8).
- **R3 One engine, one writer measured** — H is measured on the `value` setter in Chromium 156 only; W3–W8 follow from
  treating every non-user change alike (§5.2). The PR body says so.
- **R4 Cost** — one value clone per user edit, O(len), the same order as the edit (§4.1).

## §6. Test and coverage plan

**Pre-step: Chromium measurement** (done: runs 1–2 below; the procedure is kept for reproducibility). In the built-in
browser of the Claude desktop app (load the `anthropic-skills:built-in-browser` skill first). Serve a probe page from the
scratchpad through a temporary `.claude/launch.json` (`python3 -m http.server <port> --directory <scratchpad>`) +
`preview_start`, because `file://` pages cannot run page tools. Delete the launch.json and `preview_stop` afterwards.
Page: `<input id=b value="">`, `<textarea id=t></textarea>`, a `readonly` pair `<input id=r readonly value="ab">` /
`<textarea id=tr readonly>ab</textarea>`, a full `maxlength=1` pair `<input id=m maxlength=1 value="z">` /
`<textarea id=tm maxlength=1>z</textarea>`, `<input id=src value="p">`, `<button id=away>`; listeners on every field for
`input`, `change`, `blur` log `{type, isTrusted, inputType, value, activeElement.id}` to a `<pre>` and the console. User
steps use the tool's **trusted** click and keyboard input (`isTrusted === true` in the log is required; a run without it
is invalid). Script steps run via the page tool's JS evaluation, which does not move focus. Blur is a trusted click on
`#away`. Cases, each on `#b` and on `#t`, from a fresh page load:

- **M0**: click field; script `value = "x"`; click `#away`.
- **M1**: click; type `a`; script `value = "third"`; click `#away`.
- **M2**: click; type `a`; script `value = ""`; click `#away`.
- **M3**: click; script `value = "x"`; type `a`; click `#away`.
- **M4**: click; script `value = "x"`; Backspace (value back to `""`); click `#away`.
- **M5**: script `value = "ab"` before focus; click field; ArrowLeft; script `value = "x"`; click `#away`.

`input` cases (§4.4), each on the input and the textarea of its pair; record `input` and the value after:

- **N1** ArrowLeft in `#b` / `#t` holding `ab` (script-set before focus), caret at the end (C8).
- **N2** ArrowLeft in the `readonly` pair, caret at the end (C12).
- **N3** paste into the `readonly` pair; the clipboard is filled first by Cmd+A, Cmd+C in `#src`.
- **N4** paste into the full `maxlength=1` pair, caret at the end.
- **N5** IME commit into the full `maxlength=1` pair, caret at the end (nothing to insert; C11). If the tool cannot drive
  a trusted composition, record "not measured".

Record in the PR body: engine and version (`navigator.userAgent`), date, the page source, and each case's log. One engine
is evidence, not proof; the PR body says so.

**Pre-step run 1 (2026-11-03, observed).** Substitution: Claude in Chrome was not connected; the run used headless Google
Chrome 156.0.8078.80 (macOS) driven over CDP (user steps `Input.dispatchMouseEvent` / `dispatchKeyEvent` / `insertText`, every
logged event `isTrusted === true`; script steps `Runtime.evaluate`). Same on `<input>` and `<textarea>`: M0 none, **M1 change**, M2
none, M3 change, **M4 change**, M5 none; N1–N4 no `input`; N5 not measured (synthetic composition only). M1 and M4 both fire:
rev 2's §4.3 STOP row (PM 303 → PM 305).

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

Red-first: each NEW cell runs on `d7d34035` with the fix absent; "HEAD" is the expected result there (C27–C29 need the
test clipboard, so their HEAD column is `d7d34035` plus that seam). Shell cells drive `handle_key` / `handle_ime` on a
`build_test_content_state` fixture (P9) and observe events through listeners writing to a global array read back with
`eval_script`. UA focus moves use `content::focus::set_focus`. "Both kinds" means two test fns, on `<input>` and on
`<textarea>`. Every writer cell (C20–C26) first asserts that the writer changed the value to the stated string, so a
writer path that does not run fails loudly instead of passing.

| Cell | Setup → assertion | HEAD |
|---|---|---|
| C1 (was rev 2 C20) `ua_blur_no_change_for_script_written_value` | `<input id=b>`, `<input id=c>`; UA focus `b`; VM `b.value = "x"`; UA `set_focus(c)` → `b` saw `blur`, no `change` (M0) | red |
| C2 `ua_blur_change_for_user_edit` | UA focus `b`; `handle_key` `a`; UA `set_focus(c)` → `["input","change","blur","focusout"]` in that order | green (control) |
| C3 `blur_current_no_change_for_script_written_value` | as C1, blur through `blur_current` (the unfocusing steps) | red |
| C4 (was rev 2 C22) `ua_blur_after_user_edit_then_script_write`, both kinds | UA focus `b`; key `a`; VM `b.value = "third"`; UA `set_focus(c)` → `change` (M1) | green |
| C5 `ua_blur_after_user_edit_then_script_restore`, both kinds | UA focus `b`; key `a`; VM `b.value = ""`; UA blur → no `change` (M2) | green (pin) |
| C6 `ua_blur_after_script_write_then_user_revert`, both kinds | UA focus `b`; VM `b.value = "x"`; key Backspace (value back to `""`: the setter moved the cursor to the end, `crates/dom/elidex-form-core/src/lib.rs:621-623`); UA blur → `change` (M4) | red |
| C7 (was rev 2 C21; unit) `focus_snapshot.rs` / `user_edit.rs` | (a) seed → unmarked, `initial` = value; (b) seed + `apply_user_edit` that changes the value → marked; (c) `apply_user_edit` with no snapshot → no component inserted; (d) re-seed after a mark → unmarked; (e) edit that leaves the value equal → `changed == false`, unmarked; (f) `clipboard_paste` on a readonly state and on a full-`maxlength` state → unchanged; (g) `clipboard_cut` with a selection → changed, result = the cut text; (h) no `FormControlState` → `None`; `take_change_on_blur`: (i) no snapshot → `false`; (j) marked, value ≠ `initial` → `true`, snapshot removed, a second call → `false`; (k) unmarked with value ≠ `initial`, and marked with value = `initial` → `false`; (l) seed at `""`, value set to `"x"` directly (a non-user write), `apply_user_edit` appends `y` → `initial == "x"`, marked; a second `apply_user_edit` → `initial` still `"x"` | n/a (new API) |
| C8 `arrow_key_fires_no_input` | `<input id=b value="ab">` UA-focused; VM `b.setSelectionRange(1, 1)` (so ArrowLeft moves); `handle_key` ArrowLeft → cursor at 0, no `input` | red |
| C9 `arrow_key_then_script_write_no_change` | as C8, then VM `b.value = "x"`; UA blur → no `change` (M5) | red |
| C10 `ime_commit_marks_and_fires_input` | UA focus `b`; `handle_ime(Commit("あ"))` → `compositionend`, `input`; UA blur → `change` | green (control) |
| C11 `ime_commit_with_full_maxlength_fires_no_input` | `<input id=b maxlength=1 value="z">`; UA focus; VM `b.setSelectionRange(1, 1)` (no selection, so nothing is deleted); `Commit("あ")` → `compositionend`, value `"z"`, no `input` | red |
| C12 `readonly_navigation_fires_no_input` | `<input id=b readonly value="ab">`; UA focus; VM `b.setSelectionRange(1, 1)`; ArrowLeft → no `input` | red |
| C13 `vm_focus_then_user_edit_fires_change` | VM `b.focus()`; key `a`; UA `set_focus(c)` → `change` | green (control) |
| C14 `textarea_ua_blur_no_change_for_script_written_value` | C1 on `<textarea id=b>` | red |
| C15 `textarea_user_edit_fires_change` | C2 on `<textarea>` with Enter (`insertLineBreak`) | green (control) |
| C16 `stale_mark_does_not_survive_refocus` | UA focus `b`; key `a`; VM `b.blur()` (snapshot left, P3); VM `b.focus()` (re-seed); VM `b.value = "x"`; UA `set_focus(c)` → no `change` | red |
| C17 `script_write_user_edit_script_write`, both kinds | UA focus `b`; VM `b.value = "x"`; key `a`; VM `b.value = "y"`; UA blur → `change` (M6) | green |
| C18 `user_edit_then_revert`, both kinds | UA focus `b`; key `a`; Backspace; UA blur → no `change` (M7) | green |
| C19 `second_focus_rebaselines_before_user_edit`, both kinds | UA focus `b`; key `a`; UA blur → `change`; UA focus `b`; VM `b.value = "x"`; key `b`; UA blur → `change` (M8) | green |
| C20 `user_returns_to_script_written_value`, both kinds (W1 / W2) | UA focus `b`; key `a`; UA blur (`change`); UA focus `b`; VM `b.value = "x"`; key `y`, Backspace (value `"x"`); UA blur → no second `change` (M9) | red |
| C21 `textarea_default_value_write_rebaselines` (W3) | `<textarea id=b>p</textarea>`; UA focus; VM `b.defaultValue = "q"` → value `"q"`; key `z`, Backspace; UA blur → no `change` | red |
| C22 `value_attribute_write_rebaselines` (W4) | `<input id=b value="p">`; UA focus; VM `b.setAttribute("value", "q")` → value `"q"`; key `z`, Backspace; UA blur → no `change` | red |
| C23 `type_change_sanitize_rebaselines` (W5) | `<input id=b value=" a">`; UA focus; VM `b.type = "url"` → value `"a"`; key `z`, Backspace; UA blur → no `change` | red |
| C24 `email_multiple_toggle_rebaselines` (W6) | `<input id=b type=email value="a@b, c@d">`; UA focus; VM `b.setAttribute("multiple", "")` → value `"a@b,c@d"`; key `z`, Backspace; UA blur → no `change` | red |
| C25 `set_range_text_rebaselines` (W7) | `<input id=b value="p">`; UA focus; VM `b.setRangeText("q", 0, 0)` → value `"qp"`; key `z`, Backspace; UA blur → no `change` | red |
| C26 `form_reset_rebaselines` (W8) | `<form id=f><input id=b value="d"></form>`; VM `b.value = "x"` before focus; UA focus; VM `f.reset()` → value `"d"`; key `z`, Backspace; UA blur → no `change` | red |
| C27 `paste_into_readonly_fires_no_input`, both kinds | test clipboard `"q"`; `readonly`, value `"ab"`, UA-focused; Cmd+V → value `"ab"`, no `input` | red |
| C28 `paste_into_full_maxlength_fires_no_input`, both kinds | test clipboard `"q"`; `maxlength=1`, value `"z"`; UA focus; VM `b.setSelectionRange(1, 1)`; Cmd+V → value `"z"`, no `input` | red |
| C29 `cut_marks_and_fires_input` | `<input id=b value="ab">`; UA focus; VM `b.setSelectionRange(0, 1)`; Cmd+X → test clipboard `"a"`, value `"b"`, `input` (`deleteByCut`); UA blur → `change` | green (control) |

Mutation guards (final head; each must turn the named cells red, then reverted):

| Guard | Mutation | Red cells |
|---|---|---|
| MG1 (was rev 2 MG8) | `take_change_on_blur` ignores `user_changed` (HEAD's value-only predicate); the shell cells turning red also show the shell sink's verdict comes from it (I5). They can turn red because `initial` keeps the seed value while unmarked (§4.3) | C1, C3, C7 (k), C9, C14, C16 |
| MG2 | rev 2's design (E3): `dispatch_input_event_typed` marks (the mutation makes `note_user_value_change` `pub` and calls it there with the current value) and every caller fires `input` unconditionally | C8, C9, C11, C12, C27, C28 |
| MG3 (was rev 2 MG9) | delete the `note_user_value_change` call in `apply_user_edit` | C2, C10, C13, C15, C29 |
| MG4 | `apply_user_edit` returns `changed = true` always | C7 (e)/(f), C8, C11, C12, C27, C28 |
| MG5 | `record_focus_snapshot` keeps an existing mark when re-seeding | C7 (d), C16 |
| MG6 | the first mark leaves `initial` at the seed value (rev 2's branch A; the analogue of a writer skipping the re-baseline) | C6, C7 (l), C20–C26 |
| MG7 | every user value change re-baselines, not only the first | C7 (l), C18, C20 |
| MG8 | the input `value` setter (W1) goes through `apply_user_edit` | C1, C3 |

Coverage: engine-independent cells in `elidex-form` (C7); integration in `elidex-shell` (the rest). **Cut and paste
(PM 303): a small seam is enough, so no slot.** The shell reaches the system clipboard only through two private fns,
`set_clipboard_text` / `get_clipboard_text` (`event_handlers.rs:667-694`; `git grep -n arboard -- crates` →
`event_handlers.rs:669`, `:681` and `Cargo.toml`). Each gains a `#[cfg(test)]` early return to a thread-local
in-memory clipboard in `content_test_support.rs` (mounted `#[cfg(test)]`, P9). That is hermetic, because each test runs
on its own thread (`cargo test`) or in its own process (nextest), and it adds nothing to non-test builds. No new
abstraction is introduced. The design's platform interface (`ClipboardManager`,
`docs/design/en/23-platform-abstraction.md` "23.2.4 Clipboard and Drag & Drop") replaces the two fns' bodies when it
lands, and the seam goes with them. No WPT subset is claimed. The change-on-blur WPTs need trusted user input, which
the WPT runner does not drive.

## §7. Out of scope and slots

- `user validity` at step 2.1.1 → `#11-focus-change-user-validity` (adjacent; this PR does not set it).
- VM `focus()` / `blur()` events (I5 × I6 becomes observable there) → A2c, `#11-vm-host-synthetic-dom-event-dispatch`.
- The other sinks' calls of `take_change_on_blur` (the fixup, A2c's VM sink) → umbrella X1.
- Readonly selection-edit bypass → CP-1; dirty flag on cursor moves → CP-2; number input has no user edit path
  → CP-3; select keyboard `change` → CP-4 (§8.1).
- Newline normalization on the paste / IME insert paths → `#11-textarea-edit-path-newline-normalization` (same
  handlers, different defect; this PR keeps the inserted text unchanged).
- Seed gap (existed before this PR): a focused control that script switches from a non-text kind to a text kind holds no
  snapshot (the seed runs at focus only, P1), so a later user edit fires no `change`. The spec compares with the value
  "when the control was first focused", which a control that was not then a text control does not define, so this is
  recorded as a known property, not a defect.
- Observation for the PM, not a slot here: the shell calls the system clipboard directly (`arboard`,
  `event_handlers.rs:669`, `:681`), while `docs/design/en/08-security-model.md` (line 16) routes renderer clipboard
  access through the Browser Process broker.
- `ime.rs:158` ("No-op for now.", IME enable / disable) is unrelated to provenance; left as is.

## §8. Edges, hand-offs and records

- **Order and edges**: umbrella §1.2–§1.3 (this PR is (1); e1 into the focus-fixup PR).
- **A2c** (`#11-vm-host-synthetic-dom-event-dispatch`): when VM `blur()` / `focus()` start running the canonical
  transition, an `input` listener's synchronous blur reaches step 2.1, and I6 (mark before `input`) becomes observable.
  A2c's plan should add the cell. Record R-1 below.
- **Future user edit paths** (drag-drop insert, undo / redo, spin buttons, the textarea writing-direction change): call
  `elidex_form::apply_user_edit` and fire `input` iff `changed` (I2). Home: `apply_user_edit`'s doc comment, the
  function each such path must call; spin buttons are also CP-3's fix. No separate slot: none of these paths exists, and
  a path that skips the funnel shows as a missing `input` (R2).
- **PR body records**: the pre-step's measurement (§6), the measured model H (§4.3), the Q7 table (§4.2), the writer
  table (§5.2), red-first results per cell, MG1–MG8 results, §8.1 / §8.2 texts.

### §8.1 Slots CP-1 to CP-4 (defect texts; ID, owner, why, trigger and re-eval: umbrella §4)

Search first: `memory/project_open-defer-slots.md` has no entry for any of the four (`/usr/bin/grep -n -i 'readonly\|dirty_value\|number input\|select.*keyboard\|change-on-blur'`
→ only unrelated `#11-dom-rect-readonly` and `#11-style-readonly-strict-throw`).

- **CP-1 `#11-readonly-selection-edit-bypass`** (crate `elidex-form-core`). *Defect:* a `readonly` text control
  with a selection accepts typing and Backspace / Delete, because `form_control_key_input_action`'s selection branch
  (`input.rs:47-77`) runs before the readonly check (`:80`); §4.10.5.3.3: "When specified, the element is not mutable".
- **CP-2 `#11-dirty-flag-on-cursor-move`** (crate `elidex-form-core`). *Defect:* `input.rs:88-90` sets the dirty
  value flag for every `Consumed` key, cursor moves included, so an arrow key stops later `value`-attribute / `defaultValue`
  writes from updating the live value (§4.10.18.1: set when the user "changes the value"). *Fix shape:* the user paths
  set the flag from `elidex_form::apply_user_edit`'s `changed` (I1), not from a second edit / move predicate in
  form-core.
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

1. The §6 pre-step is run and recorded (runs 1–2, done) and §4.3 / §4.4 follow it.
2. `git grep -n 'dispatch_input_event_typed(' -- crates/shell/elidex-shell/src/content/event_handlers.rs crates/shell/elidex-shell/src/content/ime.rs`
   → every hit except the select arrow's (§4.2 row 14) is guarded by an `apply_user_edit` result's `changed`.
3. `git grep -n 'apply_user_edit\|note_user_value_change' -- crates ':!*_tests.rs' ':!*/tests/*'` → only the definitions
   and their unit tests (`user_edit.rs`, `focus_snapshot.rs`), the `lib.rs` re-export, and the shell's user-input sites
   of §4.2 rows 1–4 (`event_handlers.rs`, `ime.rs`). A hit in `crates/script`, in another `elidex-form` file (the
   reconciler, `value_mode.rs`, `submit.rs`) or in a shell script-path writer (`form_input.rs`'s reset) is a non-user
   writer marking (I4).
4. Cells C1–C29 green on the final head; NEW cells red / green on `d7d34035` as tabled (results in the PR body);
   MG1–MG8 each red on their cells, then reverted.
5. No touched file over 1000 lines (`event_handlers.rs` estimated ~986, D-6); over 1000 → stop, the split PR lands first (umbrella e3).
6. `cargo fmt --all`; `cargo test -p elidex-form -p elidex-shell -p elidex-js --all-features`; `mise run ci` green
   (`grep 'ERROR task failed'` on its log → none).
7. PR body carries §8's records (applied at landing: umbrella §6 item 3).

## §10. Open items

- **Q-1 (PM)** — measure W3–W8 (§5.2) in Chromium before commit (3)? Only the `value` setter was measured; C21–C26
  assert elidex's rule that every non-user change re-baselines before the mark. The run-2 driver can run C21–C26's
  sequences unchanged. The designer recommends it: a family that diverges would change a cell's expected outcome, not
  the mechanism.

Rev 1's M1 is answered by the umbrella (§1); M2 and M3 are decided by PM decision 264 (D-6, D-2).

## Changelog

- **Rev 3 (2026-11-03)** — single design H (Chromium-measured, PM 305). Deleted: branches A-only / B1 / B2, their
  representations, the per-branch cell outcomes (C4, C6), the STOP rows, the per-kind branch paragraph, §4.4's
  unmeasured rows (incl. the conditional proposed L3 defect), the pre-step's branch-selection sentences and the B1 file
  note. H's re-baseline is taken at the first mark inside `apply_user_edit`, so no writer calls anything (§4.3, D-4,
  I3, I4); the M0–M9 table maps cases to cells. Review fixes: script-writer population by property with a re-runnable
  method (§5.2, P7; Ax2 IMP-1); MG table for H, MG6–MG8 added (Ax3 IMP-1/2); D-7 commits each green, with every cell and
  the `take_focus_snapshot` caller migration placed (Ax3 IMP-4, MIN); cells C17–C29 for M6–M9, W3–W8 and cut / paste,
  M1 / M2 / M4 cells on both kinds; test clipboard seam (PM 303; Ax3 IMP-5); CP-2 fix shape points to `apply_user_edit`
  (Ax3 IMP-6); MG2 states the visibility change; future user-edit paths homed in `apply_user_edit`'s doc comment; DoD 3
  covers all crates; I1 × I3 keydown note, seed gap, ECS-native check, single component shape, `ime.rs:158` (Ax2 MINs).
  §10 Q-1 added.
- **Rev 2 (2026-11-03)** — umbrella `docs/plans/2026-11-focus-fixup-program.md` records PR-1 / PR-2 and PM decision
  264: the change-on-blur judgement moves into `elidex_form::take_change_on_blur` (D-1, I5, §4.1, §5.1, C7 (i)–(k), MG1,
  R-1; `update_steps.rs:72-76` doc); commit (0) dropped, `event_handlers.rs` estimate ~980 (D-6, D-7, §5.1, DoD 2 / 5);
  pre-step gains M5 and the `input` cases N1–N5 with §4.4's branches; header / §1 / §8 / §10 point to the umbrella;
  slots renamed CP-1 to CP-4 with owner / trigger / re-eval left to umbrella §4; D-7 commit (3)'s cells corrected (C14 for
  C15); the plan (2) rev-2 cite carry-over note in §0.1 removed.
- **Rev 1 (2026-11-03)** — initial draft.
