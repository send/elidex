# Change-on-blur — model H re-derived from HTML (spec → ECS principles → evidence)

Slot `#11-change-on-blur-model-spec-rederivation` (owner Layout; PM 432, PM 842), with
`#11-implicit-submission-commits-change` folded in (PM 842). Rev 3.7, 2026-11-21. Branch
`layout-change-on-blur-rederivation`. **Docs only**: this memo changes no code and plans no code change (PM 842).

**Defect this memo removes.** The landed change-on-blur model **H** (#597, `e0e18713`) names a measurement of one
engine as its ground (umbrella `docs/plans/2026-11-focus-fixup-program.md` DC2, row (1), §3; plan (1)
`docs/plans/2026-11-change-on-blur-user-provenance.md` D-1, §4.3). Under CLAUDE.md's ground order a core requirement
comes from the spec, then from elidex's own principles where the spec is silent; a measurement is at most `[evidence]`.

**Result.**

- Every row of §4 holds except E-5 and E-19, and E-18 in the cases DF-2 names ((ii) and (iii)); E-16 (user
  validity) is not an element of H. A holding row rests on spec text, on spec text plus an elidex principle (E-15),
  or on an elidex principle alone where the spec is silent (E-13, `<textarea>`); E-14, and E-18 where the kind at
  focus loss is one step 2.1 excludes, hold under reading R-2 (§0.5). No measurement is a ground for any of them.
- **The baseline** (E-5) differs from the spec. HTML compares with the value "when the control was first focused"; H
  compares with the value just before the first user value change. They differ only when a non-user write lands
  between focus and the first user value change. Defect **DF-1** (§6) is class (a); class (b) is examined with it:
  - **(a)** the value at focus loss equals the first-focus value → H fires `change`; the spec fires none under every
    reading of the sentence. **[spec]**.
  - **(b)** the value differs from the first-focus value and the user's edits net to nothing → H fires none, the
    outcome **D-3** (§0.4) decides: class (b) is **not a defect**. D-3 also decides M1, M6 and S*: `change`, H's
    outcome there too.
- **The snapshot's population** (E-19) differs from the spec: elidex chooses it by the kind at each write (the seed
  and each `type` change), step 2.1 by its conjuncts at focus loss. Defect **DF-2** (§6.2, whose sequences (i)–(iii)
  are its cells). **[spec] under R-2 (§0.5)**; where a non-user write in the round trip (a sanitization, or a write
  of the value by type-change step 2 or 3) changes the value, the comparison also rests on D-3 (DC-19).
- Both fixes are a separate code PR (PM 842).
- Implicit submission: the current order (`submit`, then `change` at a later focus loss) **conforms** (§5).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is read at `origin/main` **`29edf6fc`**, this branch's original base. The premise code is the
same at the branch's merge base with `origin/main`:
``/usr/bin/grep -o 'crates/[a-z][^`:) ]*\.rs' docs/plans/2026-11-change-on-blur-spec-rederivation.md | sort -u | xargs git diff --stat 29edf6fc $(git merge-base origin/main HEAD) --``
(every `.rs` file this memo cites) → empty (run 2026-11-21; control: the same `git diff --stat` with `-- .claude/tools/ecs-mission-allowlist.tsv` →
1 file changed). The allowlist and ratchet figures (LR-1, §9) are measured on this revision. Spec text is quoted
from `.claude/tools/webref body …` (commands in §3).

### §0.2 Premises — H as landed

| ID | Premise | Where (at `29edf6fc`) |
|---|---|---|
| P1 | The state is one ECS component on the control, `FocusValueSnapshot(Option<String>)`: `None` at seed, `Some(before)` from the first user value change | `crates/dom/elidex-form/src/focus_snapshot.rs:32`, seed `:53-63`, mark `:75-79` (`get_or_insert`, `:77`) |
| P2 | The mark is set only by `apply_user_edit`, and only when the value after the edit differs from the value before it | `crates/dom/elidex-form/src/user_edit.rs:50-66` (compare `:59`, mark `:63`) |
| P3 | The verdict is computed in one function: remove the snapshot, fire iff `Some(b)` and `value != b` | `focus_snapshot.rs:87-95` (`take_change_on_blur`) |
| P4 | Only text controls carry a snapshot (`is_text_control`: TextInput, Password, TextArea, Email, Url, Tel, Search); a `type` flip to a non-text kind clears it | `crates/dom/elidex-form-core/src/lib.rs:104-115`; `crates/dom/elidex-form/src/reconciler.rs:166` |
| P5 | The substep runs at step 2.1 of the one focus-update implementation, before the `FOCUS` bit is cleared and before `blur`; the seed runs after the `focus` / `focusin` listeners | `crates/dom/elidex-dom-api/src/focus/update_steps.rs:254`, `:287`, `:289`, `:303-311` |
| P6 | The shell sink dispatches `change` (bubbles, not cancelable, not composed) iff the verdict is true, then `blur`, then `focusout`. Its callers: user focus moves (`set_focus`), the unfocusing steps (`blur_current`), the rendering-time fixup (`focus_fixup`) | `crates/shell/elidex-shell/src/content/focus.rs:132-140`, `:160-161`, `:192-199`; `:73`, `:94`, `:111` |
| P7 | User edits reach the funnel from four shell sites, and `input` fires iff `changed`: text keys, cut, paste, IME commit | `crates/shell/elidex-shell/src/content/event_handlers.rs:549-571`, `:706-716`, `:723-727`; `crates/shell/elidex-shell/src/content/ime.rs:117-152` |
| P8 | Enter in a single-line text control returns `KeyAction::Submit`; the shell dispatches `submit` at the form and does not run the change-on-blur substep. The control keeps focus | `crates/dom/elidex-form-core/src/input.rs:689-697`; `event_handlers.rs:574-577`; `crates/shell/elidex-shell/src/content/form_input.rs:129-143` |
| P9 | VM `focus()` sets the bit and seeds; VM `blur()` clears the bit. Neither fires an event or consumes the snapshot (A2c, `#11-vm-host-synthetic-dom-event-dispatch`) | `crates/script/elidex-js/src/vm/host/html_element_proto.rs:269-314`, `:344-353` |
| P10 | Checkbox / radio fire `input` then `change` from their own state-change path; a select arrow key fires `input` only (`#11-select-keyboard-change-event`, open) | `form_input.rs:80-86`; `event_handlers.rs:619-623` |
| P11 | No sink call sits on a navigation or unload path | `git grep -n 'blur_current(\|focus_fixup(\|set_focus(' -- crates/shell/elidex-shell/src ':!*tests*' ':!*test_support*'` → 14 hits = 3 definitions + 11 call sites: click, Tab, label click, iframe click, the rendering fixup (run 2026-11-20) |
| P12 | No shell user edit path exists for Number, Range or the date / time kinds | `git grep -n 'FormControlKind::\(Range\|Number\|Date\|Color\|Time\|Month\|Week\|DatetimeLocal\)' -- crates/shell/elidex-shell/src ':!*tests*'` → 0 hits; control: the same command with `FormControlKind::Checkbox` → 2 hits (run 2026-11-20) |
| P13 | The cells that pin H's baseline: C6 (`change` expected), C20–C26 (no `change` expected), unit C7 (l) | `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs:508-527`, `:562-679`; `focus_snapshot.rs:211` |
| P14 | The snapshot is written at four instants, not only on the focus transition: the seed (the shell sink after `focus` / `focusin`, and VM `focus()`), the mark (the edit path), the clear (a `type` flip to a non-text kind while focused), the take (focus loss). A `type` flip from a non-text kind to a text kind writes nothing | seed `update_steps.rs:310-311`, `crates/shell/elidex-shell/src/content/focus.rs:147` (in `seed_focus_snapshot`, `:142`), `html_element_proto.rs:311`; mark `user_edit.rs:62-64`; clear `reconciler.rs:165-167`; take `focus_snapshot.rs:87-95` |
| P15 | The shell's key handler reads the focused control's kind at each `keydown`, so a control that became a text kind while focused is edited through the funnel | `crates/shell/elidex-shell/src/content/event_handlers.rs:324-334` |
| P16 | The seed of a non-text control creates no snapshot (it removes one), and the mark never inserts one | `focus_snapshot.rs:53-63`, `:75-79` |

### §0.3 Premise errors

- **PE-1 (brief)** — "implicit submission (§4.10.21.2)". The section is **§4.10.22.2 Implicit submission**
  (`.claude/tools/webref dfn html 'default button'` → §4.10.22.2).
- **PE-2 (brief)** — "the textarea / select change rules". §4.10.11 The textarea element has **no** `change` rule
  (`.claude/tools/webref body html the-textarea-element | /usr/bin/grep -c 'named change'` → 0; control: the same
  command with `'named input'` → 2). The select rule is *send select update notifications* (§4.10.7).
- **PE-3 (slot row of `#11-implicit-submission-commits-change`)** — "HTML §4.10.5.5 fires a text control's `change`
  when it loses focus". §4.10.5.5 only announces it ("will be fired when the control loses focus"). The step that fires
  it is §6.6.4 *focus update steps* step 2.1.
- **PE-4 (plan (1) §4.3)** — "It does not say what a script write does to either [condition]". For the second
  condition the sentence names its reference value: "what it was when the control was first focused". Nothing in it
  lets a later write move that value. So the spec is not silent on class (a): the value equals the first-focus value
  and no `change` fires. The mixed class, where the value differs: §4.1 and D-3 (§0.4). See E-5.
- **PE-5 (plan (1) §4.4)** — "Replacing a selection with identical text was not measured either; I2 follows elidex's
  own value comparison". §4.10.5.5 names that case as a non-change (Q4). The spec decides it; no probe is needed.

### §0.4 Decisions

Each line names the ground-order step that decided it. The behavioural exception is D-3: no step of the ground order
decided it (step 2.1's text is undetermined, §4.10.5.5 leans without deciding, and the principle lens did not settle,
§4.1), so the user decided it, and it carries no step label. Its ground is stated once, in D-3 below; every other
site of this memo and of the two landed memos refers to it as "D-3" (plan (1) writes "the re-derivation's D-3",
because plan (1) has a D-3 of its own). The outcomes D-3 gives are the K-first column of §4.1's table.

- **D-1 The mark, the funnel, the provenance rule, the commit point, the event order and the kind boundary hold on spec
  text** — [spec]: §4.10.5.5, §6.6.4 step 2.1, §2.1.4, UI Events §3.3.2 (table §4, E-1 to E-4, E-6 to E-12, E-14, E-17,
  and E-18 where the kind at focus loss is one step 2.1 excludes). E-14 and that part of E-18 hold under reading R-2
  (§0.5); across a `type` change the kind boundary differs: E-19 and E-18's other cases (DF-2).
- **D-2 No `change` fires at step 2.1 when the value equals the value at first focus** — [spec]: §6.6.4 step 2.1's
  parenthetical (Q1), under every reading of it (§4.1). H's baseline (the value before the first user value change)
  fires one in class (a). It has no ground in spec text or in an elidex principle; its only source is the #597
  measurement [history]. Recorded as defect DF-1 (a); this memo does not change the behaviour.
- **D-3 The mixed class — decided: K-first** — ground: the user's choice (PM 866, 2026-11-20, as corrected by
  PM 870, 2026-11-21); no step label (above). When a non-user write and a user value change both happened while the
  control was focused, the value differs from the first-focus value, and the readings of §4.1 split, `change` fires
  iff the user changed the value, `V ≠ F0`, and `V` differs from the value just before the first user value change.
  So class (b) fires no `change`; M1, M6, S* and DF-2 (iii) where a non-user write in the round trip (a
  sanitization, or a write of the value by type-change step 2 or 3) changes the value (DC-19) fire `change`. §4.10.5.5 (Q5: "These events are not fired in response to changes made
  to the values of form controls by scripts.") stays a lean: K-first applies it only to the value before the first
  user edit, not to later script writes. K-last and L are rejected (§4.1). "K-first is closest to what landed" is
  not a ground (後方互換性は維持しない). The readings not chosen (L, K-last) go to the compat layer only on a content
  need in the form of LR-3; none is recorded.
- **D-4 `<textarea>` fires change-on-blur under the same predicate as `<input>`** — [spec] silent (PE-2);
  [principle] decides (§4.2): One issue, one way — the text-control class has one edit funnel and one commit
  mechanism. ECS-native first does not decide it by itself (§4.2).
- **D-5 Implicit submission is not a commit point in core** — [spec] §4.10.5.5 and §4.10.22.2 for the event set;
  [principle] (One issue, one way) for the user-interface classification the spec leaves to the user agent (§5). A
  different order is a `[compat]` concern.
- **D-6 The #597 measurements stay in plan (1) as `[history]`** and are a ground for nothing. Parts of PM 305, PM 307
  and PM 393 rest on them or contradict Q1; LR-6 proposes what the PM withdraws and what stays. This memo withdraws
  nothing.

### §0.5 Readings this memo takes

Each is the memo's inference, not spec text. A cell or verdict that depends on one names it.

- **R-1 "first focused" (Q1).** The text names no step. The memo reads it as the instant the control is designated
  the focused area of the document (focus update steps step 4.1, sub-step 2: "Designate entry as the focused area of
  the document"), which precedes the `focus` event (step 4.4). "First" is read per focus session: the designation
  that began the period "while the control was focused" that ends at this focus loss. It separates that instant from
  later focus requests in the same session, which return at focusing steps step 5 (Q10); it does not mean the first
  focus in the control's lifetime. Reason for the inference: the same sentence scopes the user's change to "while the
  control was focused" and to a change not yet committed.
- **R-2 "a defined activation behavior" (Q1).** Read as §4.10.5.5's "a defined input activation behavior". Read
  literally, every `input` element has activation behavior (§4.10.5 The input element: "The activation behavior for
  input elements element, given event, are these steps: … Run element's input activation behavior, if any, and do
  nothing otherwise."), and step 2.1 would never fire. Under R-2 the states that define an input activation behavior
  and to which `change` applies are excluded from step 2.1: Checkbox, Radio Button, File Upload and Color
  (`webref body html 'color-state-(type=color)'` and `'file-upload-state-(type=file)'`: "The input activation
  behavior for such an element element is to show the picker, if applicable, for element."). Two consequences of R-2
  that the text does not resolve: step 2.1's "or its list of selected files" has no element left to apply to (only
  File Upload has such a list), and §4.10.5.5's own list of states with a defined input activation behavior names
  Checkbox, Radio Button and File Upload but not Color.
- **R-3 "the element's value" at first focus (Q1), for a control whose `type` changes while focused.** Read as the
  control's value in the sense of §4.10.18.1 A form control's value ("A control's value is its internal state"),
  whatever the `type` state was at first focus; not the `value` IDL attribute's result in the mode of that state.
  Used by DF-2 only.
- **R-4** — not used; the number is not reused. A form reset the user starts through a Reset Button is decided by
  spec text (§4.10.23, Q14; E-2).

## §1. Scope

In scope: the grounds of every element of H (§4); the implicit-submission decision (§5); the differences as defects
with the cells a later code PR adds (§6), where the spec, a principle or D-3 decides the expected value; the ledger
records (§7); ground corrections in the two landed memos (§8).

Out of scope: any code change; the plan of DF-1's and DF-2's fix (PM 842); the choice between the readings of §4.1
(D-3, §0.4); VM `focus()` / `blur()` events (A2c); user validity at step 2.1.1
(`#11-focus-change-user-validity`); the four defect slots plan (1) registered (umbrella §4).

**Shape.** One docs PR. Not edge-dense in itself: it adds no mechanism and couples no new invariant. It is plan-reviewed
because it changes a ratified ground (D-2) and records behaviour differences (DF-1, DF-2).

## §2. Coupled invariants

None new. The memo touches existing ones: it leaves two pairs intact, and DF-2 corrects three statements.

- **Plan (1) I3 (provenance lifetime = snapshot lifetime) × D-5.** A commit at Enter would need a second consumer of
  the snapshot and a re-seed outside a focus transition. D-5 keeps I3's lifetime rule as landed.
- **Umbrella X1 (one judgement, every sink) × DF-1.** DF-1 is a defect in what the one judgement compares with, not in
  where it is computed. X1, plan (1) I1, I2, I4, I5 and I6 are untouched by DF-1.
- **Corrected by DF-2**: plan (1) I7 ("only `is_text_control` kinds carry a snapshot"), the type-flip clear and the
  non-text re-seed in plan (1) I3's list of what removes the snapshot, and umbrella X2 ("a type flip to a non-text
  kind clears snapshot and mark together"). Each chooses the population by the kind at a write; step 2.1 decides by
  its conjuncts at focus loss (§6.2), and LR-2's known requirement 5 removes those writes. The landed memos carry an
  erratum pointer at each (§8).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §6.6.4 Processing model | *focus update steps* step 2.1 | all four conjuncts; the parenthetical's reference point (first focused); the parenthetical's two readings (§4.1; D-3) | re-derived, E-4 to E-8, E-18, E-19; DF-1, DF-2 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focus update steps* steps 2.1 → 2.4, 4.1 | `change` before `blur`; the old entry stays the focused area during `change` | re-derived, E-10 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focusing steps* step 5; *unfocusing steps* step 8 | already focused → return; every focus loss reaches the focus update steps | re-derived, E-8, E-17 | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | the three user-interface classes; "without an explicit commit action" | text control = no commit action; `change` at focus loss | re-derived, E-1, E-3, E-4, E-11, E-15 | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | "not fired in response to changes made … by scripts"; state restoration; "on behalf of the user" | non-user writers fire nothing and do not mark; class (b): D-3 (§0.4) | re-derived, E-2, E-6, D-3 | ✓ | yes |
| WHATWG HTML §4.10.22.2 Implicit submission | default button → `click`; no submit button → the blocking-field count, then submit | names no `change`, no focus change | re-derived, E-15, §5 | ✓ | yes |
| WHATWG HTML §4.10.11 The textarea element | user changes the raw value → `input`; no `change` rule | spec silent on `change`; decided by principle | re-derived, E-13 | ✓ | yes |
| WHATWG HTML §4.10.7 The select element | *send select update notifications* | `input` then `change`, own commit | boundary only, E-14; `#11-select-keyboard-change-event` | ✗ | yes |
| WHATWG HTML §4.10.5.1.12 Number state (type=number) | value sanitization algorithm | a value that is not a valid floating-point number becomes the empty string | boundary only; DF-2 (iii)'s Number example (§6.2, no cell) | ✗ | no |
| WHATWG HTML §4.10.5.1.13 Range state (type=range) | value sanitization algorithm; default value, default minimum / maximum, default step | a value that is not a valid floating-point number becomes the best representation of the default value | boundary only; DF-2 (iii)'s Range example (§6.2, no cell) | ✗ | no |
| WHATWG HTML §2.3.4.3 Floating-point numbers | best representation of a number | ToString(n) | boundary only; DF-2 (iii)'s Range example | ✗ | no |
| WHATWG HTML §4.10.5.1.14 Color state (type=color) | value sanitization algorithm (*update a color well control color*) | a value that does not parse as a CSS color becomes opaque black | boundary only; DF-2 (iii) passes through it (DC-19) | ✗ | no |
| WHATWG HTML §4.10.5.1.15 Checkbox state (type=checkbox) | input activation behavior | `input` then `change`, own commit | boundary only, E-14; DF-2 (iii) passes through it (DC-18) | ✗ | yes |
| WHATWG HTML §4.10.5.1.16 Radio Button state (type=radio) | input activation behavior | `input` then `change`, own commit | boundary only, E-14 | ✗ | yes |
| WHATWG HTML §4.10.5.1.20 Reset Button state (type=reset) | input activation behavior | resets the form owner | re-derived, E-2 | ✗ | yes |
| WHATWG HTML §4.10.23 Resetting a form | reset algorithm; "do not count as changes caused by the user" | a reset, however started, is not a user change (Q14) | re-derived, E-2 | ✗ | yes |
| WHATWG HTML §2.1.4 DOM trees | removing steps, step 2 and its note | removal of the focused area fires no `blur`, no `change` | re-derived, E-9 | ✓ | no |
| WHATWG HTML §4.10.18.1 A form control's value | user validity; dirty value flag | not H (E-16); the dirty flag has its own predicate (plan (1) D-5) | pointer only | ✗ | no |
| UIEVENTS §3.3.2 Focus Event Order | rows 3–4 | `focusout` follows `blur` | re-derived, E-10 | ✓ | yes |
| WHATWG HTML §4.10.5 The input element | activation behavior of `input` elements; *update an input element's type* steps 1–3 and 6 | every `input` has activation behavior (R-2); the value written by a `type` change (R-3; DF-2 (iii): value mode → default/on → value mode keeps the value; value mode → value mode runs the new kind's sanitization) | readings only (§0.5); DF-2, DC-18, DC-19, DF-2 (iii) through File Upload | ✗ | yes |
| WHATWG HTML §6.6.6 Focus management APIs | `activeElement` getter steps | reads the focused area's DOM anchor | observation only, O-2 | ✗ | no |
| WHATWG HTML §8.1.7.2 Queuing tasks | queue an element task | → queue a global task | observation only, O-3 | ✗ | yes |
| WHATWG HTML §8.1.7.3 Processing model | event loop step 2.1 | the task queue is chosen in an implementation-defined manner | observation only, O-3 | ✗ | yes |
| CSS COLOR 4 §16.2.1 HTML-compatible serialization of sRGB values | 6-digit hex serialization | opaque black serializes as `"#000000"` | boundary only; DC-19 | ✗ | no |

Numbers and titles: `.claude/tools/webref coverage-map html 6.6.4 html 4.10.5.5 html 4.10.22.2 html 4.10.11 html 4.10.7
html 4.10.5.1.12 html 4.10.5.1.13 html 2.3.4.3 html 4.10.5.1.14 html 4.10.5.1.15 html 4.10.5.1.16 html 4.10.5.1.20 html 4.10.23 html 2.1.4 html 4.10.18.1 uievents 3.3.2 html 4.10.5 html 6.6.6 html 8.1.7.2 html 8.1.7.3 css-color-4 16.2.1` (run 2026-11-21; anchors `#focus-processing-model`,
`#common-input-element-events`, `#implicit-submission`, `#the-textarea-element`, `#the-select-element`,
`#number-state-(type=number)`, `#range-state-(type=range)`, `#floating-point-numbers`,
`#color-state-(type=color)`, `#checkbox-state-(type=checkbox)`, `#radio-button-state-(type=radio)`, `#reset-button-state-(type=reset)`,
`#resetting-a-form`, `#dom-trees`, `#a-form-control's-value`, `#events-focusevent-event-order`,
`#the-input-element`, `#focus-management-apis`, `#queuing-tasks`, `#event-loop-processing-model`,
`#HTML-compatible-serialization-of-srgb`).
Breadth (verified 2026-11-21): `coverage-map` prints "spec=3 (css-color-4, html, uievents), step=21 entries" and
"K≥4 or M≥20 → 分割推奨"; the preflight counts the 24 rows and prints "SPLIT-RECOMMENDED (K>=4 or M>=20)". Not taken:
the memo is one docs PR that adds no mechanism (§1 Shape), and the rows §4.10.5.1.12, §4.10.5.1.13, §2.3.4.3,
§4.10.5.1.14 and CSS COLOR 4 §16.2.1 are boundary-only citations for DC-19 and for the two examples of DF-2 (iii)
that add no cell (§6.2). Also looked up: `webref dfn html 'unload a document'` →
§7.5.9 Unloading documents; `webref body html unloading-documents | /usr/bin/grep -c -i 'focus\|blur\|change event'`
→ 0 (control: `-c -i 'unload'` on the same output → 25). The preflight does not know the label `UIEVENTS` that
`coverage-map` prints, so it skips that row; its number and title were checked by hand
(`.claude/tools/webref heading uievents 3.3` → §3.3.2 Focus Event Order `#events-focusevent-event-order`). The
CSS COLOR 4 row was checked by `.claude/tools/webref dfn css-color-4 'HTML-compatible serialization is requested'`
→ §16.2.1 HTML-compatible serialization of sRGB values.

### §3.1 Normative text quoted

Each quote is copied from the output of the named command.

| ID | Source (`.claude/tools/webref body html <anchor>`) | Text |
|---|---|---|
| Q1 | `focus-update-steps`, step 2.1 | "If entry is an input element, and the change event applies to the element, and the element does not have a defined activation behavior, and the user has changed the element's value or its list of selected files while the control was focused without committing that change (such that it is different to what it was when the control was first focused): 1. Set entry's user validity to true. 2. Fire an event named change at the element, with the bubbles attribute initialized to true." |
| Q2 | `common-input-element-events` | "The input event fires whenever the user has modified the data of the control. The change event fires when the value is committed, if that makes sense for the control, or else when the control loses focus. In all cases, the input event comes before the corresponding change event (if any)." |
| Q3 | `common-input-element-events` | "For input elements without a defined input activation behavior, but to which these events apply, any time the user causes the element's value to change without an explicit commit action, the user agent must queue an element task on the user interaction task source given the input element to fire an event named input at the input element, with the bubbles and composed attributes initialized to true. The corresponding change event, if any, will be fired when the control loses focus." |
| Q4 | `common-input-element-events` | "Examples of a user changing the element's value would include the user typing into a text control, pasting a new value into the control, or undoing an edit in that control. Some user interactions do not cause changes to the value, e.g., hitting the "delete" key in an empty text control, or replacing some text in the control with text from the clipboard that happens to be exactly the same text." |
| Q5 | `common-input-element-events` | "These events are not fired in response to changes made to the values of form controls by scripts." and "These events are also not fired when the browser changes the values of form controls as part of state restoration during navigation." |
| Q6 | `common-input-element-events` | "For input elements without a defined input activation behavior, but to which these events apply, and for which the user interface involves both interactive manipulation and an explicit commit action, then when the user changes the element's value, the user agent must queue an element task … to fire an event named input …, and any time the user commits the change, the user agent must queue an element task … to set its user validity to true and fire an event named change …" |
| Q7 | `implicit-submission` | "If the user agent supports letting the user submit a form implicitly (for example, on some platforms hitting the "enter" key while a text control is focused implicitly submits the form), then doing so for a form, whose default button has activation behavior and is not disabled, must cause the user agent to fire a click event at that default button." and "If the form has no submit button, then the implicit submission mechanism must perform the following steps: 1. If the form has more than one field that blocks implicit submission, then return. 2. Submit the form element from the form element itself with userInvolvement set to "activation"." |
| Q8 | `the-textarea-element` | "Any time the user causes the element's raw value to change, the user agent must queue an element task on the user interaction task source given the textarea element to fire an event named input at the textarea element, with the bubbles and composed attributes initialized to true." |
| Q9 | `html-element-removing-steps` | "If document's focused area is removedNode, then set document's focused area to document's viewport … This does not perform the unfocusing steps, focusing steps, or focus update steps, and thus no blur or change events are fired." |
| Q10 | `focusing-steps` | step 5 "If new focus target is the currently focused area of a top-level traversable, then return."; step 8 "Run the focus update steps with old chain, new chain, and new focus target respectively."; unfocusing steps step 8 "… run the focusing steps for topDocument's viewport." |
| Q11 | `send-select-update-notifications` | "… queue an element task on the user interaction task source given element to run these steps: 1. Set element's user validity to true. … 4. Fire an event named input at element … 5. Fire an event named change at element, with the bubbles attribute initialized to true." |
| Q12 | `.claude/tools/webref body uievents events-focusevent-event-order` | rows 3–4: "blur — Sent after first target element loses focus"; "focusout — Follows the blur event". |
| Q13 | `event-change` (the events index; non-normative) | "change — Event — Form controls — Fired at controls when the user commits a value change (see also the input event)" |
| Q14 | `resetting-a-form` | "Each resettable element defines its own reset algorithm. Changes made to form controls as part of these algorithms do not count as changes caused by the user (and thus, e.g., do not cause input events to fire)." |
| Q15 | `the-input-element` (*update an input element's type*) | "1. If oldState put the value IDL attribute in the value mode, element's value is not the empty string, and newState puts the value IDL attribute in either the default mode or the default/on mode, then set element's value content attribute to element's value. 2. Otherwise, if oldState put the value IDL attribute in any mode other than the value mode, and newState puts the value IDL attribute in the value mode, then set the value of element to the value of its value content attribute, if there is one, or the empty string otherwise, and then set element's dirty value flag to false. 3. Otherwise, if oldState put the value IDL attribute in any mode other than the filename mode, and newState puts the value IDL attribute in the filename mode, then set the value of element to the empty string. … 6. Invoke the value sanitization algorithm for element, if one is defined for newState." |
| Q16 | `color-state-(type=color)`; `update-a-color-well-control-color` | "The value IDL attribute is in mode value." "The value sanitization algorithm is as follows: Run update a color well control color for the element." Its steps: "2. Let value be the result of running these steps: 1. If element's dirty value flag is false, then return the result of getting an attribute by namespace and local name given null, "value", and element. 2. Return element's value. 3. Let color be the result of parsing value. 4. If color is failure, then set color to opaque black. 5. Set element's value to the result of serializing a color well control color given element and color." Serializing: with no `alpha` attribute and the Limited sRGB `colorspace` (the missing value default), "set htmlCompatible to true" |
| Q17 | `the-input-element`; `.claude/tools/webref body css-color-4 color-serialization-html-compatible-serialization-is-requested`; `body css-color-4 opaque-black` | "An input element's dirty value flag must be set to true whenever the user interacts with the control in a way that changes the value." CSS COLOR 4 §16.2.1: "A seven-character string consisting of the character "#", followed immediately by the two-digit hexadecimal representations of the red component, the green component, and the blue component, in that order, using ASCII lower hex digits."; "opaque black is defined as the color rgb(0 0 0 / 100%)" |
| Q18 | `range-state-(type=range)`; `number-state-(type=number)`; `floating-point-numbers` | Range: "The value sanitization algorithm is as follows: If the value of the element is not a valid floating-point number, then set it to the best representation, as a floating-point number, of the default value."; "The default value is the minimum plus half the difference between the minimum and the maximum, unless the maximum is less than the minimum, in which case the default value is the minimum."; "The default minimum is 0." "The default maximum is 100."; "The default step is 1 (allowing only integers, unless the min attribute has a non-integer value)." Number: "The value sanitization algorithm is as follows: If the value of the element is not a valid floating-point number, then set it to the empty string instead." §2.3.4.3: "best representation of the number n as a floating-point number is the string obtained from running ToString(n)." Both states: "The value IDL attribute is in mode value." |

### §3.2 User-input touch audit

Nothing is added on a user-input path. The audit of the existing paths is plan (1) §3.1 and stays as it is.

## §4. The re-derivation

Verdict column: **holds** = H's behaviour is what the named ground requires; **differs** = it is not.

| # | H element (as landed) | Spec ground | Principle ground (spec silent) | Evidence | Verdict |
|---|---|---|---|---|---|
| E-1 | The mark is set when a user edit changes the value: the value after the edit differs from the value before it (P2). A caret move, a refused paste and an identical replacement do not mark | Q1 "the user has changed the element's value"; Q3 "any time the user causes the element's value to change"; Q4 lists the non-changes (PE-5) | — | plan (1) §6 N1–N4 [history], not a ground | holds [spec] |
| E-2 | No non-user writer marks: the `value` setters, `defaultValue`, the `value` attribute, type-change and `multiple` sanitization, `setRangeText`, form reset (plan (1) W1–W8) | Q5. A sanitization is the user agent's algorithm, not "the user" of Q1. A reset, including one the user starts through a Reset Button (part of W8): Q14, its changes "do not count as changes caused by the user" | — | — | holds [spec] |
| E-3 | `input` fires iff the user changed the value; bubbles and composed | Q3 (input), Q8 (textarea) | — | — | holds [spec] |
| E-4 | A text control's only commit point is the loss of focus | Q3: typing is a change "without an explicit commit action" (Q4 names typing and pasting), and its `change` "will be fired when the control loses focus"; Q1 fires it | — | — | holds [spec] |
| E-5 | **The baseline**: the value just before the first user value change since focus (P1, P3). A non-user write made before that change moves the baseline | Q1: "different to what it was when the control was first focused". Under every reading, no `change` fires when the value equals the first-focus value (§4.1). The mixed class, where the value differs: §4.1 | the lens did not select a reading (§4.1) | the #597 runs M4, M9 and W3–W8, measured on one engine [history]; a ground for nothing | **differs** in class (a) [spec] → DF-1 (a). Class (b), M1, M6, S*: holds under D-3 (§0.4) |
| E-6 | No mark → no `change`: a script-only change fires nothing (M0, M5) | Q1 "the user has changed"; Q5 | — | — | holds [spec] |
| E-7 | A user edit that is undone before blur fires no `change` (M7) | Q1's parenthetical: the value is not different from what it was at first focus | — | — | holds [spec] |
| E-8 | Every focus-loss path runs the same substep: a user focus move, the unfocusing steps, the rendering-time fixup (P6). VM `focus()` / `blur()` do not yet (P9) | Q10: the focusing steps and the unfocusing steps both end in the focus update steps; `focus()` and `blur()` run them too (`webref body html dom-focus`: "Run the focusing steps for this"; "run the unfocusing steps given this") | — | — | holds [spec] for the sink paths. The VM gap is known and owned: `#11-vm-host-synthetic-dom-event-dispatch` (A2c), record R-1 of plan (1). No new defect |
| E-9 | Removing the focused control fires no `change`; a page unload fires none (P11) | Q9; §7.5.9 Unloading documents has no focus or `change` step (§3) | — | — | holds [spec] |
| E-10 | Order at focus loss: `change`, `blur`, `focusout`. `change` is fired synchronously inside the step, while the old control is still the focused area (P5, P6) | Q1 is step 2.1, `blur` is step 2.4, and the focused area is designated at step 4.1; Q12 | — | — | holds [spec] (the state seen *during `blur`* is not H: observation O-2) |
| E-11 | `input` precedes the `change` of the same edit; the mark is set before `input` is built (plan (1) I6) | Q2 "In all cases, the input event comes before the corresponding change event" | the mark-before-`input` order is elidex's device, with no observable of its own | — | holds [spec] |
| E-12 | `change` attributes: bubbles, not cancelable, not composed | Q1 "with the bubbles attribute initialized to true"; nothing else is initialized | — | — | holds [spec] |
| E-13 | `<textarea>` fires change-on-blur under the same predicate as `<input>` | silent: Q1 says "input element"; §4.10.11 has no `change` rule (PE-2); no text forbids it; Q13 (non-normative) describes `change` as fired at form controls | §4.2: One issue, one way | plan (1) §6: `<textarea>` measured identical to `<input>` on one engine [history]; not reached, since step 2 decides | holds [principle] |
| E-14 | Kind boundary: only text controls carry the snapshot (P4). Checkbox, radio and select commit through their own paths (P10) | Q1 excludes an element with "a defined activation behavior", read as a defined input activation behavior (R-2); §4.10.5.1.15 / §4.10.5.1.16 fire `input` then `change` from the input activation behavior; Q11 | — | — | holds [spec] under R-2. Open and already owned: `#11-select-keyboard-change-event` (select arrow key). See O-4 for the kinds with no user edit path, and E-19 for a kind that changes while focused |
| E-15 | Implicit submission does not commit: Enter dispatches `submit`; `change` fires at a later focus loss (P8) | Q7 names the events of implicit submission: a `click` at the default button, or the form's submission. No `change`, no focus change. Q3: no explicit commit action for typed text | §5: the classification Q6 leaves to the user agent | slot row: two engines fire `change` before `submit` [evidence]; not re-measured here; not reached | holds [spec] + [principle] |
| E-16 | User validity is not set at step 2.1.1 | Q1 substep 1 | — | — | not an element of H; open and owned: `#11-focus-change-user-validity` |
| E-17 | A redundant focus of the focused control does not re-seed | Q10 step 5; Q1 "first focused" | — | — | holds [spec] |
| E-18 | A `type` flip to a non-text kind while focused clears the snapshot, so no `change` fires at blur (P4) | Q1's conjuncts are evaluated at blur: "the change event applies to the element, and the element does not have a defined activation behavior" (R-2) | — | — | **decided by the kind at focus loss**, not by the kind at the flip. Holds [spec] under R-2 when the kind at focus loss is Checkbox, Radio Button, File Upload, Color, Hidden or a button state. **Differs** when it is one step 2.1 applies to: a flip to Number, Range or a date / time kind is cleared too → DF-2 (ii); a round trip through a kind the reconciler clears (Checkbox, Color, …) and back to a text kind is cleared at the first flip and not restored at the second → DF-2 (iii) |
| E-19 | A `type` flip from a non-text kind to a text kind while focused creates no snapshot (P14, P16); a user edit made after it is not marked, and blur fires no `change` | Q1's conjuncts are evaluated at focus loss, and at that instant all four hold: the element is an `input`, `change` applies, it has no defined input activation behavior (R-2), and the user changed its value while it was focused | — | — | **differs** → DF-2 |

### §4.1 E-5 in detail — what the sentence fixes, what it leaves, and how D-3 is decided

Q1 has two parts: (U) "the user has changed the element's value … while the control was focused without committing
that change", and (F) "(such that it is different to what it was when the control was first focused)". Write `F0` for
the value when the control was first focused (R-1) and `V` for the value at focus loss.

**What every reading fixes [spec].**

- `V ≠ F0` is necessary. (F) is in the present tense at the step: a `change` at a control whose value equals its
  value at first focus contradicts (F) however "such that" is read.
- A user value change while focused is necessary (U; E-6).
- With no non-user write while focused, `change` fires iff the user changed the value and `V ≠ F0` (E-4, E-7).

**What the text leaves.** (F) modifies "that change", the user's change. "Such that" can be read two ways, and the
text does not choose:

- **L (an added condition).** Two conditions, both stated: the user changed the value while focused, and `V ≠ F0`.
  Fire iff both hold.
- **K (a result clause).** The user's change is such that the value differs from `F0`: the difference from `F0` is the
  result of the user's change. A difference that results only from a non-user write does not satisfy it.

L is a complete predicate. K is complete only where the user's change and the difference coincide; for a sequence
that mixes user changes and non-user writes, the text gives K no rule for whose difference it is. Two completions
can be computed from facts written at defined instants (others may exist):

- **K-first**: fire iff the user changed the value, `V ≠ F0`, and `V` differs from the value just before the first
  user value change. This is H's predicate with (F) added.
- **K-last**: fire iff the user changed the value, `V ≠ F0`, and `V` differs from the value left by the most recent
  non-user write (the user's edits since that write net to a change). A write that does not change the value does
  not move that reference (as for DC-14 and DC-15, §6.2); with no value-changing non-user write the reference is
  `F0`, where K is complete.

| Sequence while focused (plan (1) §4.3 names) | `F0` | `V` | L | K-first | K-last | H as landed |
|---|---|---|---|---|---|---|
| class (a), M4 (cell C6): script `x`; user Backspace | `""` | `""` | none | none | none | **`change`** |
| M2: user types `a`; script `""` | `""` | `""` | none | none | none | none |
| M3: script `x`; user types `a` | `""` | `"xa"` | `change` | `change` | `change` | `change` |
| M8, second session: script `x`; user types `b` | `"a"` | `"xb"` | `change` | `change` | `change` | `change` |
| M1 (cell C4): user types `a`; script `third` | `""` | `"third"` | `change` | `change` | none | `change` |
| M6: script `x`; user types `a`; script `y` | `""` | `"y"` | `change` | `change` | none | `change` |
| class (b), M9 (cells C20–C26, C7 (l)): script `x`; user types `y`, Backspace | `"a"` | `"x"` | `change` | none | none | none |
| S* (cell DC-17): script `x`; user types `a`, Backspace (value `"x"`); script `y` | `""` | `"y"` | `change` | `change` | none | `change` |
| DF-2 (iii) through Checkbox (cell DC-18): user types `a`; script `type = "checkbox"`; script `type = "text"` (the type-change steps write `"a"` over `"a"`) | `""` | `"a"` | `change` | `change` | `change` | none (no snapshot; DF-2) |
| DF-2 (iii) through Color (cell DC-19): user types `a`; script `type = "color"` (the sanitization writes `"#000000"`); script `type = "text"` | `""` | `"#000000"` | `change` | `change` | none | none (no snapshot; DF-2) |

S* differs from M6 only in the user's net edit: in M6 it is non-zero (`x` → `xa`), in S* it is zero (`a`, then
Backspace), so the whole difference from `x` to `y` is the script's; K-first still fires, because it compares `V`
with the value just before the first user edit (`"x"`), not with the value the last script write left (`"y"`, which
is K-last's comparison).

The two DF-2 (iii) rows, derived in §6.2: through Checkbox, no non-user write changes the value, so K is complete;
through Color, the Color sanitization does, so the row is in the mixed class.

So:

- **Class (a) is decided by the spec under every reading**: no `change`. H fires one. DF-1 (a).
- **M2, M3 and M8 are not in dispute**: every reading gives H's outcome. Nor is DC-18: every reading fires
  `change`, and H fires none (DF-2, a defect under every reading).
- **Class (b), M1, M6, S* and DC-19 are the mixed class**: K is not complete on them, and K-first and K-last
  differ on M1, M6, S* and DC-19 as L and K-first differ on class (b). Step 2.1's text does not choose among them;
  D-3 (§0.4) does: K-first's column.

**The weight of Q5.** "These events are not fired in response to changes made to the values of form controls by
scripts." In class (b), and in M1 and M6, the value's difference from `F0` at focus loss comes only from a script
write; the user's own edits left nothing. A `change` there reports a difference the script made. That leans toward K,
and "no `change` fires without a user change of the value" does not answer it: the user's change is necessary for
the event under L, but so is the script's write. Against the lean: Q5's stated purpose ("to make it
easier to update the values of form controls in response to the user manipulating the controls, without having to
then filter out the script's own changes to avoid an infinite loop") is met by every reading, since under none of
them does a script write alone fire an event; and K-first does not apply the lean evenly (it fires on M1 and M6,
where the difference is also only the script's). Q5 leans; it does not decide. Q13 (non-normative: "when the user
commits a value change") leans the same way for class (b).

**The principle rung — lens.** One line per principle of CLAUDE.md's Design philosophy and Design discipline:
which reading it selects or rejects.

| Principle | Selects / rejects | Why |
|---|---|---|
| Ground order (spec → principles → evidence) | selects none; rejects the #597 measurement as a ground for K-first | the text is undetermined here; Q5 leans K without deciding (D-3, §0.4, is not a rung-1 decision); a measurement is `[evidence]` at most and is reached only if this table selects nothing. This table selected nothing, but rung 3 was not consulted: the question went to the PM (LR-7), and D-3 came first |
| ECS-native first | selects none | L, K-first and K-last are each one component on the control read by one judgement function, every fact written at a defined instant by the seed or the funnel. L stores `F0` and the mark; K-first stores `F0` and the pre-edit value; K-last one fact more. A count of fields is not an ECS principle. The rule excludes another engine's internal field as a *ground* for K-first; it does not exclude the outcome |
| Whole-engine core / compat / deprecated | selects none | all three are readings of the standard text; none is legacy or non-standard. It says only where the unchosen outcome would live if content needed it (compat) |
| Plugin-first extensibility | not engaged | no extension point is involved |
| ScriptSession as the sole Script↔ECS boundary | not engaged | under all three no non-user writer calls anything (plan (1) D-4 holds for each) |
| Concurrency by ownership and phases | not engaged | |
| Security by structure | not engaged | |
| Ideal over pragmatic | **split** | "cleanest": L — one reference value, the only one the sentence names, and no completion rule to invent. "spec-faithful": cannot choose on text; the lean of Q5 and Q13 is toward K. The two halves of the principle point in different directions |
| 設計優先 (existing abstraction before a new type) | selects none | all three use the existing component and the one judgement function |
| One issue, one way | selects none between L and K-first; weakly rejects K-last | L and K-first use one definition of "the user changed the value" (the funnel's `changed`, shared with `input`); K-last adds a second notion (a net change since the last non-user write). The one-judgement rule (umbrella X1) holds under all three |
| Phase design is not a shortcut license | not engaged | |
| Supported-surface testing | selects none | each reading has cells (§6) |
| TODO 先送り禁止 | not engaged | |
| 後方互換性は維持しない | rejects "K-first because it is closest to what landed" as a reason | H being landed is not a ground |
| 1000-line debt; Edge-dense work; html5ever 非依存 | not engaged in the choice | (Edge-dense applies to the fix: LR-2) |

**Result of the lens: not settled.** The only principle that selects is Ideal over pragmatic, and it selects L by
its "cleanest" half while its "spec-faithful" half cannot choose and the spec's own lean is toward K. No principle
selects K, and none rejects it. That is not a decision by principle, so the question went to the PM (LR-7), and
D-3 (§0.4) records the answer.

**What D-3 rejects, and where the readings part.**

- **K-last** defeats `change`'s purpose: a formatter script that rewrites the value on every `input` leaves the
  value equal to the latest non-user write, so `change` would never fire after the user commits.
- **L** differs from K-first only wherever `V` returns to the pre-edit value while `V ≠ F0` (class (b), and a later
  non-user write restoring it): K-first fires none and L fires `change`. "The whole difference is the script's" does
  not separate them: it is also true of M1, M6 and S*, where K-first fires.

### §4.2 E-13 in detail — `<textarea>`

The spec gives no rule that fires `change` at a `<textarea>` and none that forbids it (PE-2). Step 2 decides:

- **One issue, one way** (decides). `<textarea>` already shares the funnel and the `input` rule with `<input>` (Q3 and
  Q8 use the same words). A text-control class with one edit funnel and two commit behaviours would be a second way
  to answer "what happens to an uncommitted user change at focus loss".
- **ECS-native first does not decide it by itself.** `TextArea` is a `FormControlKind` value, as the other text kinds
  are, and `is_text_control` already selects by kind; excluding `TextArea` there would be one more kind test, not a
  different shape. The principle says where the state lives (a component on the control, read by one function); it
  does not say which kinds carry it.
- **No contrary text.** Q13 describes `change` as fired at form controls when the user commits a value change.

So elidex fires it, as its own choice. This is not compatibility behaviour and needs no measurement as a ground.

## §5. Implicit submission (`#11-implicit-submission-commits-change`)

**Decision (D-5): the current order conforms. Core does not change.**

1. **[spec] The event set.** Q7 states what implicit submission does: a `click` at the default button, or the form's
   submission after the blocking-field count. It names no `change` and moves no focus. The control stays the focused
   area, so the focus update steps do not run and Q1 does not fire.
2. **[spec] The class of a typed text control.** Q3 covers a value changed "without an explicit commit action", Q4
   gives typing and pasting as its examples, and Q3 places the `change` at the loss of focus.
3. **Spec-silent residual.** Q6 applies to controls "for which the user interface involves … an explicit commit
   action". Whether a user agent's text field has one is the user agent's own user-interface fact; the spec does not
   say that Enter is, or is not, such an action.
4. **[principle] elidex's text control has no explicit commit action.**
   - One issue, one way: a text control's uncommitted user change is committed at one point (focus loss, step 2.1) by
     one judgement. A second commit point at Enter is a second way to answer "when does `change` fire".
   - ECS-native first (a fact about the component, not a ground by itself): `FocusValueSnapshot` is seeded on the
     focus transition, marked on the edit path, cleared by a `type` flip and taken at focus loss (P14). It has one
     consumer, `take_change_on_blur`, at one instant (plan (1) I3). A commit at Enter would be a second consumer at a
     second instant.
   D-5 rests on steps 1–2 (spec text) and on One issue, one way for the residual of step 3.
5. **[evidence]** The slot row records that two engines fire `change` before `submit` [evidence]. It was not
   re-measured for this memo and is not a ground: steps 1–4 decide.

**Compat boundary (description only; nothing is planned).** Content that keeps state in a `change` listener and reads
it in a `submit` listener sees the state from before the last edit when the user submits with Enter. If such content
has to work, the remedy is a `[compat]` shim outside core (CLAUDE.md core / compat / deprecated): on implicit
submission it would run the same judgement (`take_change_on_blur`) and dispatch `change` before the submission starts.
Core keeps one commit point. No slot is proposed for the shim: no content need is recorded (LR-3 says what would
count as one).

Out of this decision: whether elidex's implicit submission itself follows Q7 (observation O-1).

## §6. Defects — where H differs from the spec

### §6.1 DF-1 The change-on-blur baseline is not the value at first focus

*Defect.* `take_change_on_blur` (`crates/dom/elidex-form/src/focus_snapshot.rs:87-95`) compares the value at focus
loss with the value just before the first user value change (`:75-79`). HTML compares it with the value "when the
control was first focused" (Q1). The two differ when a non-user write (plan (1) W1–W8, or a `focus` listener's write)
lands between focus and the first user value change:

- **(a) [spec]** the user's edits bring the value back to the first-focus value → elidex fires `change`; the spec
  fires none, under every reading of Q1 (§4.1). A defect under every reading, K-first (D-3) included.
- **(b) D-3** the user's edits net to nothing and the value differs from the first-focus value → elidex fires no
  `change`, D-3's outcome (§0.4). elidex conforms. **Not a defect.**

M1, M6 and S* are in the same mixed class with the opposite sign: elidex fires `change` there, D-3's outcome (`V`
differs from the value just before the first user edit). elidex conforms. Not a defect.

*Reference instant* (reading R-1, the memo's inference, not text): "first focused" is the designation of the control
as the focused area (focus update steps step 4.1), which precedes the `focus` event (step 4.4), once per focus
session. Today the seed runs after the `focus` / `focusin` listeners
(`crates/dom/elidex-dom-api/src/focus/update_steps.rs:304-311`) and stores no value (P1).

*Not affected.* The funnel, the mark, `input`, the one-judgement rule, the event order
(E-1 to E-4, E-6 to E-15, E-17).

*Cells the code PR adds or re-states.* The Ground column is the source of the expected value. "UA focus" and
"UA blur" are the shell's user focus moves, as in plan (1) §6. The cells whose Ground is D-3 take their expected
value from it (§0.4); their "Today" already equals it.

| Cell | Sequence | Expected | Ground | Today |
|---|---|---|---|---|
| DC-1 (re-states C6), both kinds | `<input id=b value="">`; UA focus `b`; script `b.value = "x"`; user Backspace (value `""`); UA blur | no `change` | [spec] Q1 (F), every reading: the value equals the first-focus value | `change` |
| DC-2 (re-states C20), both kinds | UA focus `b`; user types `a`; UA blur (`change`); UA focus `b`; script `b.value = "x"`; user types `y`, Backspace (value `"x"`); UA blur | no `change` at the second blur | D-3: class (b), `"x"` ≠ `"a"`, and `V` equals the value just before the first user edit | none |
| DC-3 (re-states C21) | `<textarea id=b>p</textarea>`; UA focus; script `b.defaultValue = "q"`; user `z`, Backspace; UA blur | no `change` | D-3, class (b) (`"q"` ≠ `"p"`); and E-13 [principle] for the kind | none |
| DC-4 (re-states C22) | `<input id=b value="p">`; UA focus; script `b.setAttribute("value", "q")`; user `z`, Backspace; UA blur | no `change` | D-3, class (b) | none |
| DC-5 (re-states C23) | `<input id=b value=" a">`; UA focus; script `b.type = "url"` (value `"a"`); user `z`, Backspace; UA blur | no `change` | D-3, class (b) (`"a"` ≠ `" a"`) | none |
| DC-6 (re-states C25) | `<input id=b value="p">`; UA focus; script `b.setRangeText("q", 0, 0)` (value `"qp"`); user `z`, Backspace; UA blur | no `change` | D-3, class (b) | none |
| DC-7 (re-states C26) | `<form id=f><input id=b value="d"></form>`; script `b.value = "x"` before focus; UA focus; script `f.reset()` (value `"d"`); user `z`, Backspace; UA blur | no `change` | D-3, class (b) (`"d"` ≠ `"x"`) | none |
| DC-8 (new), both kinds | UA focus `b` (value `""`); script `b.value = "x"`; user types `a` (value `"xa"`); user Backspace ×2 (value `""`); UA blur | no `change` | [spec] Q1 (F), every reading | `change` |
| DC-9 (re-states C24) | `<input id=b type=email value="a@b, c@d">`; UA focus; script `b.setAttribute("multiple", "")` (value `"a@b,c@d"`); user `z`, Backspace; UA blur | no `change` | D-3, class (b); not PM 307 (LR-6) | none |
| DC-10 (new) | `<input id=b value="">` with a `focus` listener that sets `b.value = "x"`; UA focus `b`; user Backspace (value `""`); UA blur | no `change` | [spec] Q1 (F) for the comparison; **reading R-1** for the instant (the first-focus value is taken at step 4.1, before the listener runs). Under a reading that takes it after `focus`, this is class (b)'s mirror and the cell changes | `change` |
| DC-11 (unit; re-states C7 (l)) | seed at `""`; a non-user write to `"x"`; `apply_user_edit` appends `y`; a second `apply_user_edit` removes it (value `"x"`) | the judgement returns `false` | D-3, class (b) | `false` |
| DC-12 (control, unchanged) | C1, C3, C5, C9, C14, C16, C18, C19 of plan (1) | as today | [spec] E-6, E-7, and M8: every reading gives H's outcome (§4.1) | green |
| DC-13 (control, unchanged) | C4 (M1), C17 (M6) of plan (1) | `change` (as today) | D-3: `V` differs from the value just before the first user edit (§4.1) | `change` |
| DC-17 (new; S*), both kinds | UA focus `b` (value `""`); script `b.value = "x"`; user types `a` (value `"xa"`); user Backspace (value `"x"`); script `b.value = "y"`; UA blur | `change` | D-3: `"y"` ≠ `""` and `"y"` differs from the value just before the first user edit (`"x"`); the user's net edit is zero (§4.1) | `change` |

*Code comments the code PR rewrites* (read at `29edf6fc`; each was checked there by `git show 29edf6fc:<path>`).
The code PR rewrites each with the requirement of LR-2 that replaces it:

- *The measurement named as the ground* (each with an ECS-mission allowlist class-E row, pruned with the rewrite,
  except the first, which has none; LR-2's allowlist rows):
  - `crates/dom/elidex-form/src/focus_snapshot.rs:4-5`, the module doc (the `<textarea>` rule grounded on the
    measurement; no row);
  - `focus_snapshot.rs:69-73`, the doc of `note_user_value_change` (the re-baseline grounded on the measured model);
  - `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs:324`, the doc of `focus_with_caret_at_end` (its
    setup named as that of the measured N1 / M5 cases);
  - `content_change_on_blur_tests.rs:648`, C24's doc comment (below).
- *H's baseline stated as a rule* (requirement 1): `focus_snapshot.rs:13-16` (module doc: the snapshot "holds one
  thing", `None` until the first user value change), `:29-31` (doc of `FocusValueSnapshot`: "No focus-time copy of
  the value is kept", PM 393), `:69-73`; `crates/dom/elidex-form/src/reconciler.rs:161` (`type` arm: "baseline =
  value before the first user change, plan D-1"); `crates/core/elidex-ecs/src/dom/tree_clone.rs:30` (the
  clone-policy table's `FocusValueSnapshot` entry: "the value before the first user value change, plan D-1");
  `crates/dom/elidex-dom-api/src/focus/update_steps.rs:89-92` (trait doc of `seed_focus_snapshot`) and `:305-307`;
  `crates/shell/elidex-shell/src/content/focus.rs:143-145` (the shell sink's `seed_focus_snapshot`).
- *The seed after the listeners* (requirement 2): `update_steps.rs:89-92` ("after the `focus`/`focusin` listeners";
  "Seed timing does not decide whether a non-user write fires `change`", which DC-10 contradicts under R-1),
  `:305-307` ("Seed the change-on-blur baseline AFTER the `focus`/`focusin` listeners"), `focus.rs:143` ("After
  §6.6.4 focus update steps step 4.4"), `focus_snapshot.rs:44-45` (doc of `record_focus_snapshot`: "after §6.6.4
  focus update steps step 4.4").
- *The population chosen by kind* (requirement 5): `focus_snapshot.rs:25-26` (doc of `FocusValueSnapshot`: "Only
  text controls carry it"), `:43` and `:48-52` (doc of `record_focus_snapshot`: the seed "**if** it is a text
  control", and the removal for a non-text control), `:97-104` (doc of `clear_focus_snapshot`, which requirement 5
  deletes with its call), `reconciler.rs:159-164` (the `type` arm's clear).

*What happens to the existing cells.* The behaviour-level shell cells (C20–C26, C4, C17) keep their expected values
and are cited by C-id; C6's expectation flips (DF-1 (a)). These existing tests and guards change for other reasons
(each found at `29edf6fc` by `git grep -n <name> 29edf6fc -- crates`):

- C7 (l) (`c7l_first_mark_rebaselines_once`, `focus_snapshot.rs:211`) asserts the snapshot's internal
  representation, which LR-2 known requirement 1 re-decides, so it is rewritten at the judgement level (DC-11 is
  that assertion). The test helper it reads, `test_support::snap` / `Snap` (`crates/dom/elidex-form/src/test_support.rs`,
  over `FocusValueSnapshot::baseline`, the `Option<String>`), is re-defined with requirement 1's representation.
- C24's doc comment names PM 307's accepted divergence as its ground, which LR-6 withdraws, so the comment is
  rewritten (expected value unchanged; the allowlist's class-E row for it is pruned with it).
- `record_clears_stale_snapshot_when_no_longer_text_control` (`focus_snapshot.rs:223`) asserts the removal at a
  non-text re-seed that requirement 5 deletes, so it is replaced by a cell for the population rule (DF-2).
- `no_snapshot_for_non_text_control` (`focus_snapshot.rs:246`) asserts that the seed of a Button leaves no snapshot
  (`Snap::Absent`). Under requirement 5 the seed does not test the kind, so that assertion inverts; the test is
  rewritten to assert the judgement: `take_change_on_blur` is `false` because the predicate at focus loss excludes
  the kind, not because no component exists.
- `e2c_type_flip_to_non_text_while_focused_clears_focus_snapshot` (`crates/dom/elidex-form/src/reconciler_tests.rs:212`)
  asserts the `type`-arm clear that requirement 5 deletes, so it is rewritten: the component survives the flip, and
  at focus loss the Checkbox state is excluded (no `change`, as DC-16).
- `e2d_type_flip_between_text_kinds_preserves_focus_snapshot` (`reconciler_tests.rs:235`): its assertion holds (a
  text → text flip keeps the mark); its comment ("only a non-text destination clears the snapshot") is rewritten,
  since no destination clears it.
- Plan (1) MG1 and MG6 are defined on the `Option<String>` representation (MG1: "the seed stores `Some(value)`";
  MG6: "Since D-1 there is no seed value"). Requirement 1 puts the first-focus value in the component, so the slice's
  plan re-defines both against that representation.

### §6.2 DF-2 The snapshot's population follows the kind at each write, not step 2.1's conjuncts at focus loss

*Defect.* Step 2.1 evaluates its conjuncts when focus is lost (Q1: "If entry is an input element, and the change
event applies to the element, and the element does not have a defined activation behavior, and the user has changed
the element's value … while the control was focused"). elidex decides who carries the snapshot by `is_text_control`
at the seed and at each `type` change (P4, P14, P16). Three sequences where the two disagree:

- **(i) non-text → text while focused.** Focus `<input type=checkbox>` (or any non-text kind): the seed creates no
  snapshot (`focus_snapshot.rs:53-63`). A script sets `b.type = "text"`: the reconciler updates the kind and seeds
  nothing (`crates/dom/elidex-form/src/reconciler.rs:133-167`; its only snapshot write is the clear at `:165-167`).
  The user types: the shell reads the kind at that `keydown` (P15, `event_handlers.rs:324-334`), so the edit goes
  through the funnel and fires `input`, but the mark does nothing without a snapshot (`focus_snapshot.rs:75-79`).
  At blur `take_change_on_blur` returns `false` (`:88-91`). The spec fires `change`: at focus loss the element is an
  `input`, `change` applies, it has no defined input activation behavior (R-2), the user changed its value while it
  was focused, and the value differs from the first-focus value.
- **(ii) text → Number, Range or a date / time kind while focused, after a user edit.** The reconciler clears the
  snapshot for every non-text kind (`reconciler.rs:165-167`), the mark with it. Step 2.1 still applies to those
  kinds (`change` applies; no defined input activation behavior, R-2), so a user change made before the flip that
  survives the type-change sanitization still requires `change` at blur. elidex fires none.
- **(iii) text → a non-text kind → text while focused, after a user edit.** The population: a round trip through a
  kind whose snapshot the reconciler clears (every non-text kind, `reconciler.rs:165-167`), ending in a kind step 2.1
  applies to. At the first flip the reconciler clears the snapshot, the mark with it; at the second it writes nothing
  (P14). At blur the element is a text-state `input`, `change` applies, it has no defined input activation behavior
  (R-2), and the user changed its value while it was focused: step 2.1's conjuncts hold again, and at focus loss,
  where step 2.1 evaluates them, the interval in the other kind left no trace (nothing activated the control, so
  nothing committed the change). `take_change_on_blur` finds no snapshot and returns `false`. What the round trip does to the value depends on the kind passed through (Q15):
  - *Through a default or default/on mode kind* (Checkbox, Radio Button, Hidden, the button states; shown with
    Checkbox, DC-18). The user types `a` into `<input id=b>`; a script sets `b.type = "checkbox"`, then
    `b.type = "text"`. Step 1 (value mode → default/on mode, value not empty) sets the `value` content attribute to
    `"a"`; step 2 (default/on mode → value mode) sets the value to that attribute, `"a"` over `"a"` (elidex:
    `crates/dom/elidex-form/src/value_mode.rs:64` and `:88-89`, called from `reconciler.rs:150`). No non-user write
    changes the value, so every difference from `F0` (`""`; the control was first focused in the Text state, so
    R-3 is not needed) is the user's: K is complete (§4.1), and every reading fires `change`. The expected value is
    [spec] under R-2 and does not rest on D-3, as for DC-14 and DC-15. When the value is empty at the first flip,
    step 1 does not run, and step 2 on the way back sets the value to the `value` content attribute: a non-user
    write that changes the value when the attribute is not empty. Example: `<input id=b value=p>`; a script sets
    `b.value = "q"` before focus (`F0` = `"q"`); the user deletes it to `""`; `b.type = "checkbox"` (step 1 does not
    run, the value being empty; step 2 does not apply, the old state being in the value mode, nor step 3, the new
    one not being in the filename mode; the Checkbox state defines no value sanitization algorithm); `b.type = "text"` (step 2: the value becomes `"p"`; the Text
    sanitization leaves it). `V` = `"p"` ≠ `F0` = `"q"`, and the step-2 write changed the value, so the sequence is in
    §4.1's mixed class: L and K-first (the value just before the first user edit, `"q"`) fire, K-last (`"p"`) fires
    none. The expected value `change` is K-first's, decided by D-3, as for DC-19; the example adds no cell.
  - *Through Color* (DC-19). Text → color is value mode to value mode: steps 1–3 do not apply, and step 6 runs the
    Color sanitization (Q16). The user's edit set the dirty value flag (Q17), so the value read is `"a"`; it does not
    parse as a CSS color, so the color is opaque black, and with no `alpha` attribute and the default Limited sRGB
    `colorspace` it is serialized as `"#000000"` (Q16, Q17). Color → text is value mode to value mode again; the Text
    sanitization ("Strip newlines from the value",
    `webref body html 'text-(type=text)-state-and-search-state-(type=search)'`) leaves `"#000000"`. `V` = `"#000000"` ≠ `F0` = `""`, and a non-user write (the
    Color sanitization) changed the value, so the sequence is in §4.1's mixed class: L and K-first fire, K-last
    fires none. The expected value `change` is K-first's, decided by D-3: `V` differs from the value just before the
    first user edit (`""`).
  - *Through File Upload*: a defect only when the control was dirty at first focus (its value then differed from
    its `value` content attribute). Step 3 empties the value and step 2 on the way back sets it to the content
    attribute, or to the empty string without one, so otherwise `V` = `F0` and no reading fires. When it was dirty,
    `V` ≠ `F0`: where the step-3 or the step-2 write changed the value, the sequence is in the mixed class, as DC-19;
    where neither did (the user emptied the value and there is no content attribute), the difference is the user's
    and every reading fires `change`, as DC-18.
  - *Through a value mode non-text kind* (Number, Range, the date / time kinds). Text → that kind and back are value
    mode to value mode: steps 1–3 do not apply, and step 6 runs the new kind's sanitization at each flip (Q15; for
    Number and Range, Q18). Where neither sanitization changes the value (e.g. the user's `1` through Number), no
    non-user write changes the value: the expected value is [spec] under R-2, as for DC-18. Where a sanitization
    changes it, the sequence is in §4.1's mixed class and the expected value is K-first's, decided by D-3, as for
    DC-19. DC-19 is the class's cell; the two examples below add none.
    - *Range*: `<input id=b>` (no `min`, `max` or `step` attribute); the user types `a`; `b.type = "range"`: `"a"` is
      not a valid floating-point number, so the value becomes the best representation of the default value, the
      minimum plus half the difference between the minimum and the maximum, 0 + (100 − 0) / 2 = 50 (default minimum
      0, default maximum 100; 50 is an integer, which the default step 1 allows), ToString(50) = `"50"` (Q18); `b.type =
      "text"`: the Text sanitization leaves `"50"`. `V` = `"50"` ≠ `F0` = `""`. L and K-first (the value just before
      the first user edit, `""`) fire; K-last (the value the last value-changing non-user write left, `"50"`) fires
      none.
    - *Number*: `<input id=b value=q>`; the user types `a` (the value is `q` with `a` inserted at the caret);
      `b.type = "number"`: that value is not a valid floating-point number, so the value becomes `""`; `b.type =
      "text"`: `""`. `V` = `""` ≠ `F0` = `"q"`. L and K-first (`"q"`) fire; K-last (`""`) fires none. The `value`
      attribute is what puts the sequence in the class: without it `V` = `F0` = `""`, and no reading fires.

*Reachability of (i).* Reachable: every step above is on a shipped path (UA focus of a checkbox:
`crates/shell/elidex-shell/src/content/form_input.rs:62`; the `type` setter reaches the reconciler's `type` arm;
key routing by the live kind, P15). No guard stops the edit of a control that became a text kind while focused.

*What `F0` is* (reading R-3, marked): the spec says "what it was when the control was first focused", and the control
was first focused in a non-text state. The memo reads `F0` as the control's value (§4.10.18.1, its internal state) at
that instant, whatever the state. Today nothing is stored for a non-text kind, so there is no `F0` to compare with.
The input type change steps may then write the value (§4.10.5 The input element, step 2: "if oldState put the value
IDL attribute in any mode other than the value mode, and newState puts the value IDL attribute in the value mode,
then set the value of element to the value of its value content attribute, if there is one, or the empty string
otherwise"); that write is a non-user write between first focus and the user's edit, so with a `value` attribute the
sequence can fall in §4.1's mixed class.

*Cells* (for the code PR). Only the cells whose Ground is D-3 depend on it: in the others no non-user write changes
the value.

| Cell | Sequence | Expected | Ground | Today |
|---|---|---|---|---|
| DC-14 (new) | `<input id=b type=checkbox>` (no `value` attribute), `<input id=c>`; UA focus `b` by Tab; script `b.type = "text"` (value `""`); user types `a`; UA blur | `input`, then `change` at blur | [spec] Q1, all conjuncts at focus loss, under R-2; `"a"` differs from the first-focus value under R-3 (`""`) and also under the IDL-result reading (`"on"`) | `input`, no `change` |
| DC-15 (new) | `<input id=b>`; UA focus `b`; user types `1`; script `b.type = "number"` (value `"1"`); UA blur | `change` | [spec] Q1 at focus loss, under R-2: `change` applies to the Number state, the user changed the value while focused, `"1"` ≠ `""` | none |
| DC-16 (control) | `<input id=b>`; UA focus; user types `a`; script `b.type = "checkbox"`; UA blur | no `change` at blur | [spec] under R-2 (E-18): the Checkbox state has a defined input activation behavior | none (green) |
| DC-18 (new; DF-2 (iii) through Checkbox) | `<input id=b>`, `<input id=c>`; UA focus `b`; user types `a` (`input`); script `b.type = "checkbox"`; script `b.type = "text"` (value `"a"`); UA blur | `change` at blur | [spec] under R-2: the conjuncts at focus loss, `V` = `"a"` (Q15 steps 1–2, which write `"a"` over `"a"`), and `"a"` ≠ `F0` = `""`; no non-user write changes the value, so every reading gives `change` (§4.1) | no `change` |
| DC-19 (new; DF-2 (iii) through Color) | `<input id=b>`, `<input id=c>`; UA focus `b`; user types `a` (`input`); script `b.type = "color"` (value `"#000000"`); script `b.type = "text"` (value `"#000000"`); UA blur | `change` at blur | [spec] under R-2 for the conjuncts at focus loss and for `V` = `"#000000"` (Q15 step 6, Q16, Q17); D-3 for the comparison: `"#000000"` differs from the value just before the first user edit (`""`); K-last would give none (§4.1) | no `change` |

### §6.3 The corrections, stated once

The two landed memos point here; they do not restate these.

- **What the landed cells pin.** Until the code PR lands:
  - C6 pins behaviour that contradicts the spec text (class (a), DF-1 (a)).
  - C7 (l) and C20–C26 pin class (b)'s outcome, and C4 and C17 M1's and M6's: D-3's outcomes (§0.4). They hold.
  - Every other cell of plan (1) holds on the ground §4 names for its element.
- **The first-focus argument** is §4.1: the sentence fixes `V ≠ F0` as necessary, and D-3 decides the mixed class.
- **The `<textarea>` ground** is §4.2 (One issue, one way).
- **The PM records PM 305, PM 307 and PM 393**: the parts proposed for withdrawal, the ground each loses, and the
  parts that stay in force are LR-6.

### §6.4 Observations outside H (for the PM to home)

Found while reading the code for §0.2. None is an element of H; none is fixed or planned here. Each states whether
it is a defect against the spec; a defect has a home, either an existing slot or a slot draft in LR-4 (the PM draws
the ID). Ledger checks are `/usr/bin/grep -n -i <term>` over the slot ledger
`/Users/kazuaki/.claude/projects/-Users-kazuaki-repos-send-sh-elidex/memory/project_open-defer-slots.md` (run 2026-11-21); "0 rows" means that grep only.

- **O-1 Implicit submission skips the default button and the blocking-field count. Defect against the spec: yes.**
  On Enter in a single-line text control the shell dispatches `submit` at the form ancestor of the focused control
  (`crates/shell/elidex-shell/src/content/event_handlers.rs:574-577` →
  `crates/shell/elidex-shell/src/content/form_input.rs:129-143`), with no default-button lookup and no field count.
  HTML §4.10.22.2 (Q7): implicit submission of a form "whose default button has activation behavior and is not
  disabled, must cause the user agent to fire a click event at that default button", and "If the form has no submit
  button, then the implicit submission mechanism must perform the following steps: 1. If the form has more than one
  field that blocks implicit submission, then return." So elidex (1) fires no `click` at a default button, and
  (2) submits a form that has no submit button and two or more blocking fields. *Owner today: none.*
  `#11-form-submission-dispatcher` is closed (D-29, #209); ledger greps for `default button` and `blocks implicit` →
  0 rows (control: `implicit submission` → 1 row, the slot LR-3 closes). *Home:* the slot draft in LR-4.
- **O-2 The focused area during `blur`.**
  - **(i) The core-side departure. Defect against the spec: yes** (draft for the PM). `focus_update_steps` clears the
    `FOCUS` bit (`crates/dom/elidex-dom-api/src/focus/update_steps.rs:287`) before it fires `blur` / `focusout`
    (`:289`), so in those listeners `document.activeElement` is the body element. The comment at `:280-283` says so
    and gives the behaviour of existing engines as its ground [history]. In HTML §6.6.4 the `blur` is fired at step
    2.4 ("fire a focus event named blur at blur event target"), and the focused area changes only at step 4.1
    ("Designate entry as the focused area of the document"); §6.6.6's `activeElement` getter reads it ("Let
    candidate be this's node document's focused area's DOM anchor. … If candidate is not a Document object, then
    return candidate."). So during `blur` the spec's `activeElement` is still the old control. The comment's second
    use of the early clear (an exact re-entrancy signal for the step-4 gate) is a device and does not ground the
    observable. *Home:* `#11-blur-focus-cleared-before-blur-event` (owner Layout, registered by PM 861 from this
    memo's draft, re-eval 2026-12-31). After the merge of #652 the ECS-mission allowlist has 1 row
    under `crates/dom/elidex-dom-api/src/focus/`, class T for that slot (`update_steps.rs`, the comment at `:282`;
    control: `elidex-dom-api` → 22 rows).
  - **(ii) Why the ratchet did not list the comment.** At `29edf6fc` it was a gap in the ratchet's vocabulary. #652
    (`1b727d18`) widened the classifier, and the comment is now listed (the row above). No tool is changed by this
    PR.
- **O-3 `input` is dispatched synchronously in the turn that handled the key. Not shown to be a defect; one
  question remains.**
  - *Spec.* Q3, Q8 and Q11: "the user agent must queue an element task on the user interaction task source given the
    input element to fire an event named input". §8.1.7.2 Queuing tasks: to queue an element task is to "Queue a
    global task given source, global, and steps". §8.1.7.3 Processing model picks the next task from a task queue
    "chosen in an implementation-defined manner". §4.10.5.5 adds that "user agents may wait for a suitable break in
    the user's interaction before queuing the tasks".
  - *elidex.* No user interaction task source exists (`git grep -n -i 'user interaction task source' -- crates
    ':!*tests*'` → 0 hits; control: `-c -i 'task source'` → 10 files). On the key path
    (`crates/shell/elidex-shell/src/content/message_dispatch.rs:166` → `event_handlers.rs:262`) one shell turn
    dispatches `keydown` (`:297`), runs the microtask and custom-element-reaction checkpoint inside that dispatch
    (`crates/script/elidex-script-session/src/event_dispatch/mod.rs:571`; `crates/shell/elidex-shell/src/lib.rs:283-288`),
    applies the edit (`:547-551`), dispatches `input` synchronously (`:570` → `form_input.rs:102-118`), and only then
    drains queued tasks (`:353`). Cut and paste run in the same turn (`:706-727`).
  - *Verdict.* It is a structural departure: no task is queued. No script-observable order that the spec fixes was
    found to differ: `input` follows the value change; the microtasks of the `keydown` listeners run before `input`;
    a task that a `keydown` listener queues on another task source runs after `input` in elidex, an order the spec
    leaves to the user agent; and nothing else in elidex queues on the user interaction task source, so there is no
    same-source order to break.
  - *Remaining question, exact.* (1) The IME commit path (`message_dispatch.rs:365` →
    `crates/shell/elidex-shell/src/content/ime.rs:117-152`) was not traced through its dispatch order. (2) A queued
    element task is runnable only while its document is fully active; whether a turn can reach the `input` dispatch
    after its own `keydown` listener made the document not fully active was not traced. Both go to the PM (LR-4).
- **O-4 The snapshot's population is `is_text_control`; step 2.1's (under R-2) is every `input` to which `change`
  applies and that has no defined input activation behavior**, which also covers Number, Range and the date / time
  kinds. **Not a defect while the kind does not change during focus**: those kinds have no user edit path (P12), so
  "the user has changed the element's value" cannot hold for them and step 2.1 cannot fire. Number's missing edit
  path is `#11-number-input-user-edit-path`. **A defect when the kind changes during focus**: DF-2. A future user
  edit path for one of those kinds writes the same component with no step of its own: under LR-2's known
  requirement 5 every `input` carries it from the seed, and the funnel marks it.

## §7. Ledger records (applied by the PM at landing)

- **LR-1 → `#11-change-on-blur-model-spec-rederivation`: close** (D-3 is decided; LR-7). Record: "Re-derived in
  `docs/plans/2026-11-change-on-blur-spec-rederivation.md`. Every element of H holds on spec text or an elidex
  principle except two, which differ: the baseline (E-5) and the snapshot's population (E-19, and E-18 in the cases
  DF-2 names) →
  `#11-change-on-blur-baseline-first-focus-value` (LR-2); the measurement is `[history]` and a ground for nothing.
  The PR deleted 18 allowlist rows (`git diff origin/main...HEAD -- .claude/tools/ecs-mission-allowlist.tsv`): 17
  class T — this slot's 14 (13 in plan (1), 1 in the umbrella) and the 3 that #652 added for
  `#11-change-on-blur-baseline-first-focus-value` (PM 855) on lines this PR rewrote (plan (1)'s §3 textarea row,
  the umbrella's DC2 textarea sentence and its §3 textarea row) — and 1 class E (slot `-`) for plan (1) §8's line
  recording W6 as a divergence from one engine, which the PR tagged `[history]`. A rewritten or tagged line's row is
  stale, and the allowlist only shrinks."
- **LR-2 → `#11-change-on-blur-baseline-first-focus-value`: amend the registered row** (registered by PM 861 from
  this memo's draft, with DF-2 made a facet of the DF-1 slot). Owner Layout and Re-eval-date 2026-12-15 are
  unchanged. The amendment, against the row's current text:
  - *DF-1 (a)*: unchanged (cells DC-1, DC-8, DC-10).
  - *DF-1 (b)*: replace "decided by D-3 (user) — when the user's edits net to nothing and the value differs from the
    first-focus value; expected values of DC-2..DC-7, DC-9, DC-11, DC-13 follow D-3" with: "**not a defect**: when
    the user's edits net to nothing and the value differs from the first-focus value, elidex fires no `change`, the
    outcome D-3 of the memo (§0.4) gives. The cells whose expected value follows D-3: DC-2 to DC-7, DC-9, DC-11 (no
    `change`; `false` for DC-11), DC-13 (M1, M6) and DC-17 (S*) (`change`), all green today; and DC-19 (`change`),
    DF-2 (iii)'s cell through Color, which is red today. The fix of DF-1 (a) keeps the green ones green."
  - *Facet DF-2*: replace "the snapshot population follows `is_text_control` at the seed and on `type` change, but
    step 2.1 evaluates at focus loss — (i) a control switched from non-text to text while focused has no snapshot
    (`reconciler.rs` :165-167 clears, never seeds), (ii) an edited text control switched to Number / Range /
    date-time loses its mark (cells DC-14..DC-16)" with: "the population is chosen by the kind at each write (the
    seed and each `type` change), not by step 2.1's conjuncts at focus loss, under reading R-2 (memo §0.5). Its cells
    are the sequences (i)–(iii) of memo §6.2: DC-14 to DC-16, DC-18, DC-19. Where a non-user write in the round trip
    (a sanitization, or a write of the value by type-change step 2 or 3) changes the value, the comparison also rests
    on D-3 (DC-19)."
  - *Fix*: after "/elidex-plan-review mandatory", add: "That slice's plan-review decides whether DF-1 and DF-2 are
    separate per-PR slices." Then replace "known requirements: one component holds the first-focus value and the
    user-change mark (re-decides PM 393), the seed before `focus` / `focusin` listeners, the VM `focus()` seed
    (`html_element_proto.rs`) follows the same rule, a redundant focus does not re-seed, population decided at focus
    loss." with:
    "Population (stated once; requirements 1 and 5 refer to it): every `input` and every `<textarea>` (memo D-4).
    Known requirements:
    1. one component holds the first-focus value and the user-change mark, on every control of the population from
       the seed (re-decides PM 393 D1, and the overwrite half of PM 307's approval, "initial = before"; memo LR-6).
       Under D-3 the mark is the value just before the first user value change, as today, so the facts are two and
       D-3 adds no third (memo §4.1);
    2. the seed before the `focus` / `focusin` listeners (today `focus_update_steps` calls the sink's
       `seed_focus_snapshot` after them), under reading R-1;
    3. the VM `focus()` seed (the `record_focus_snapshot` call in `native_html_element_focus`, P9) follows the same
       rule. The seed rule lives in one place in an engine-independent crate (`elidex-form` or `elidex-dom-api`);
       the VM host and the shell sink only call it (Layering mandate);
    4. a redundant focus does not re-seed (E-17);
    5. no write chooses the population by kind: the seed does not test the kind, and a `type` change does not clear
       the component (the fix deletes the `is_text_control` test in `record_focus_snapshot` and the reconciler's
       `type`-arm call to `clear_focus_snapshot`). Whether `change` fires is decided only inside
       `take_change_on_blur`, by one predicate over the kind at focus loss: step 2.1's conjuncts "the change event
       applies to the element, and the element does not have a defined activation behavior", under reading R-2
       (memo §0.5), with `<textarea>` included (memo D-4)."
  - *Allowlist rows*: replace "`focus_snapshot.rs:71` (E, reclassified by the code PR), `change-on-blur-user-provenance.md:199`,
    `focus-fixup-program.md:44` / `:163`" with "three class-E rows (slot `-`), each a code comment that names the
    measurement as the ground (memo §6.1): the doc comment of `note_user_value_change` in `focus_snapshot.rs`, and in
    `content_change_on_blur_tests.rs` the doc comment of C24 (W6 as PM 307's accepted divergence) and the doc
    comment of `focus_with_caret_at_end` (the measured N1 / M5 cases); the code PR
    rewrites each comment and prunes its row". The three plan-memo rows are deleted by this docs PR (LR-1).
  - *Add Trigger*: "the focus-fixup umbrella's per-PR slice for this slot starts" (before the Re-eval-date).
  - *Add Why-deferred*: "PM 842: docs and code are separate PRs; the re-derivation memo is docs only."
- **LR-3 → `#11-implicit-submission-commits-change`: close as not a core defect.** Record: "The current order conforms:
  HTML §4.10.22.2 names no `change` and no focus change for implicit submission, and §4.10.5.5 places a typed text
  control's `change` at the loss of focus; elidex's text control has no explicit commit action (One issue, one way:
  one commit point). The slot's `[evidence]` (two engines fire `change` before `submit`) is a record of engine
  behaviour, not a content need, and opens nothing. A different order would be a `[compat]` shim; none is
  registered. A new compat slot is opened when a content need is recorded. What counts as one: a named real page
  (from the compatibility survey `docs/design/ja/29-survey-analysis.md` or an exit-gate site set) whose function
  fails under the core behaviour, with the failing pair identified (here: state written in a `change` listener,
  read in a `submit` listener, on a submission by Enter). An engine's outcome alone, or a test that asserts only
  the outcome, does not count (`docs/plans/2026-11-change-on-blur-spec-rederivation.md` §5; the same form applies
  to the compat readings of its D-3)."
- **LR-4 → the PM**: observations O-1 to O-4 (§6.4). For an observation that is a defect against the spec the only
  treatment is a home; the others may be homed or dismissed.
  - *O-1 (defect: yes)* → **slot draft**, proposed ID `#11-implicit-submission-default-button-and-field-count` (the
    PM accepts or renames it at landing): "Defect [spec]: on Enter in a single-line text control the shell's
    `KeyAction::Submit` arm calls `handle_form_submit`, which dispatches `submit` at the form ancestor with no
    default-button lookup and no field count. HTML §4.10.22.2 Implicit submission fires a `click` at the default
    button when it has activation behavior and is not disabled, and, when the form has no submit button, returns if
    the form has more than one field that blocks implicit submission. elidex fires no `click` at a default button
    and submits a form with no submit button and two or more blocking fields. **Trigger**: a PR that touches the
    shell's implicit-submission path (`handle_form_submit`) or the form-submission algorithm. **Why deferred**:
    found by a docs-only memo (PM 842), which changes no code." Proposed owner Layout (the owner of the
    implicit-submission slot LR-3 closes); the owner and the Re-eval-date are the PM's. No existing slot (ledger
    greps in §6.4 O-1).
  - *O-2 (i) (defect: yes)* → already homed: `#11-blur-focus-cleared-before-blur-event` (PM 861). No new record.
    O-2 (ii): resolved by #652; nothing to record.
  - *O-3 (not shown to be a defect)* → home or dismiss. The two paths not traced go to the PM. They are outside this
    memo: its scope is the grounds of H (PM 842, docs only), H begins at the mark, and the `input` task model is not
    an element of H. A slice that already rewrites each path is where it would be traced: (1) the IME commit's
    dispatch order, by the slice of `#11-ime-commit-event-order-and-input-type` (it reorders `ime.rs`'s dispatch);
    (2) the fully-active check before `input`, by the slice of `#11-text-edit-primitives-report-change` (its D4
    makes one `input` dispatch site). The PM decides whether those slots take them.
  - *O-4 (not a defect while the kind does not change during focus)* → no new record: the kind-change part is DF-2
    (LR-2), and a future edit path for Number is owned by `#11-number-input-user-edit-path` (LR-5).
- **LR-5 → `#11-focus-change-user-validity`, `#11-vm-host-synthetic-dom-event-dispatch`,
  `#11-number-input-user-edit-path`, `#11-select-keyboard-change-event`**: no change; this memo cites them as the
  owners of E-16, E-8, O-4 and E-14.
- **LR-6 → the PM: withdraw the parts of PM 305, PM 307 and PM 393 that rest on a ground that does not hold or that
  contradict Q1** (the PM issues the withdrawal records, PM 847, and settles the final range at landing). The
  records are PM 305, PM 307 and PM 393 in `/Users/kazuaki/repos/send.sh/elidex-pm-trial/PORTFOLIO-archive-2026-W45.md`.
  This memo establishes only that each cited ground does not hold, or that the part contradicts Q1; it withdraws
  nothing. Proposed:
  - *PM 305*
    - Withdraw: the choice of H on the measurement ("all predictions hold → (i) single branch"). Ground lost: the
      #597 measurement, which the ground order does not admit for a core requirement; and H's class-(a) outcome
      contradicts step 2.1 under every reading (§4.1).
    - Stays in force: the run decisions (run M6 / M7 and the discriminating case M8 with predictions written first;
      N5 unmeasured), as a record of what was run.
  - *PM 307*
    - Withdraw: (b), W6 recorded as an accepted divergence, no slot, reopen if another engine matches. Ground lost:
      "the spec defines no baseline" (step 2.1 names the value at first focus), and its reopen condition is an
      engine's behaviour, not an acceptance criterion. C24's outcome itself holds on D-3 (DC-9).
    - Withdraw: "initial = before" in the approved deviation, the overwrite of the first-focus value by the value
      before the first user edit (plan (1) D-1: "the first mark always overwrote it"). It contradicts Q1 under every
      reading: class (a) needs the first-focus value at focus loss (DF-1 (a)). Re-decided in the slice (LR-2, known
      requirement 1).
    - Stays in force: the rest of the approved deviation, recording at the first mark the value just before the
      first user edit (fail-safe, no script-side calls), which is K-first's mark under D-3.
  - *PM 393*
    - Withdraw: D1 (`FocusValueSnapshot` → `Option<String>`). Ground lost: "the focus-time `initial` was dead state";
      class (a) needs the first-focus value at focus loss under every reading, so it is not dead state. The shape
      is re-decided in the slice (LR-2, known requirement 1).
    - Withdraw: D2's probe step ("a Chromium probe first" [history]). Ground lost: §4.10.5.5 names an identical-text
      replace as a non-change (PE-5), so the spec, not a probe, grounds it; plan (1) §6 N6 ("not run; PM 393, to run
      later") falls with it (§8).
    - Stays in force: D2's outcome (identical-text replace is not a value change), D3 + D4
      (`#11-text-edit-primitives-report-change`) and the slots it registered.
- **LR-7 → the PM: decide D-3** (§4.1) — **answered by PM 866** (2026-11-20) and PM 870 (2026-11-21): K-first,
  recorded in §0.4 D-3; the cells whose Ground is D-3 (§6.1, §6.2) take its outcome, and DF-1 (b) is not a defect.
  PM 870: the user was told S* as FYI and may revisit D-3; a revisit is a new decision. Nothing remains open under
  this record.

## §8. Edits to the landed memos (this PR)

Both memos are landed records (PM 511). Each edit keeps the landed words, marks the measurement `[history]`, and adds
a dated correction. No decision of either memo is rewritten in place. Each erratum is one line of factual correction
and a pointer to this memo's section for the substance (§6.3); the substance is stated here only. Sites are named by
section and by a quoted fragment.

| Memo | Site | Edit |
|---|---|---|
| umbrella | DC2 ("plan (1) keeps browser behaviour, measured") | the textarea ground: principle (E-13), measurement `[history]` |
| umbrella | §1.1 (b) ("the mixed case is measured") | `[history]`; the sentence fixes the reference point, the value at first focus (§4.1) |
| umbrella | §1.2 row (1) ("design H") | ground: re-derived here; the baseline differs (DF-1) |
| umbrella | §3 rows for step 2.1 and textarea | pointers to E-5 / DF-1 (class (a) contradicts the text; class (b): D-3, §0.4) and E-13 |
| umbrella | §2 X2 ("a type flip to a non-text kind clears snapshot and mark together") | erratum pointer: corrected by DF-2 (§6.2), LR-2 known requirement 5 |
| umbrella | §7; changelog | erratum section: what differs is a pointer to §6 and §6.3; X2 is corrected by DF-2 (LR-2 known requirement 5), the other X rows are not affected; changelog: one dated line pointing to this memo's changelog |
| plan (1) | opening table rows M1, M4, M9 | `[history]` tags; M4 → DF-1 (a) [spec]; M1 and M9 → the re-derivation's D-3 (its §0.4), with K-first's comparison (`V` against the value just before the first user edit) |
| plan (1) | E1 | textarea ground (§4.2); the correction line is tagged `[contrast]` ("the ground is not browser behaviour [contrast]"): it states that the ground is *not* browser behaviour |
| plan (1) | D-1 | erratum: one line per correction, pointers to §4.1, §6.1, §6.3 and LR-6 |
| plan (1) | §3 textarea row; §4.3 opening and table header; §4.4 | grounds corrected (PE-4, PE-5, §4.2) |
| plan (1) | §5.2 closing paragraph; R3; §6 pre-step heading and the W6 record; §8 first bullet; §10 Q-1; changelog Rev 3 | `[history]` tags, read through one banner at the memo's head; W6 erratum: the cited ground does not hold, LR-6 for PM 307, C24's outcome by D-3 (DC-9) |
| plan (1) | §1 (b) ("settled by measurement") | `[history]` tag; erratum pointing to D-3 (§0.4) |
| plan (1) | D-3 ("they carry no snapshot (P2)"); I3 ("reconciler type-flip clear, non-text re-seed"); I7 ("only `is_text_control` kinds carry a snapshot"); I7 × I1 ("a non-text control passed to it has no snapshot to mark") | erratum pointer at each: the population by kind is DF-2 (§6.2); LR-2 known requirement 5 |
| plan (1) | §7 "Seed gap" ("recorded as a known property, not a defect") | erratum pointer: a defect, DF-2 (i) (§6.2), under reading R-3 |
| plan (1) | §6 N6 ("not run; PM 393, to run later") | **no edit**; the line stays as landed. Recorded here: the instruction to run it falls with PM 393 D2's probe step, which LR-6 proposes the PM withdraw (PE-5: §4.10.5.5 decides the case) |
| plan (1) | changelog | one dated erratum line pointing to this memo's changelog |

## §9. Definition of done

1. `python3 .claude/skills/elidex-plan-review/preflight.py <memo>` exits 0 for this memo, the umbrella and plan (1).
2. `python3 .claude/tools/ecs_mission_ratchet.py` → `ok (495 allowlisted, 0 new, 0 stale)` (run 2026-11-21 on this
   revision, rev 3.7), with no class-T row left for `#11-change-on-blur-model-spec-rederivation` or
   `#11-change-on-blur-baseline-first-focus-value`
   (`/usr/bin/grep -c 'change-on-blur-model-spec-rederivation\|change-on-blur-baseline-first-focus-value' .claude/tools/ecs-mission-allowlist.tsv`
   → 0; control: the same command with `blur-focus-cleared-before-blur-event` → 1).
3. `git diff --stat origin/main...HEAD` lists the three `docs/plans/` memos and the allowlist (verified 2026-11-21 on
   this revision: "4 files changed"); the allowlist's change is row deletions only, the rows LR-1 names.
4. D-3 is decided by the PM (PM 866, as corrected by PM 870) and recorded in §0.4 (LR-7). Satisfied when PM 866 and PM 870 are recorded in PORTFOLIO.
5. The PM applies LR-1 to LR-6 at landing.

## Changelog

- *Rev 3.7* (2026-11-21): /pre-push Stage 4 (`/code-review high`) fixes; no decision, expected value or cell
  changes, and slot content changes only in LR-2's allowlist-row list.
  - The landed statements that choose the population by kind get erratum pointers: umbrella X2 (and §7, which said
    X1–X8 were not affected), plan (1) D-3, I3, I7, I7 × I1, and its §7 seed gap (a defect: DF-2 (i)). §2 names
    them; §8 lists the sites.
  - §6.1: the code comments the code PR rewrites, by the requirement that replaces each (the measurement as ground,
    the baseline, the seed instant, the population by kind); the existing tests and guards that change
    (`no_snapshot_for_non_text_control`, e2c, e2d's comment, the `test_support::snap` helper, plan (1) MG1 / MG6).
  - The D-3 part of DF-2 (iii) stated by its property, a non-user write in the round trip (a sanitization, or a
    write of the value by type-change step 2 or 3) that changes the value, in the summary, D-3, LR-2 and §6.2; §6.2's
    Checkbox case gains the empty-value example (step 1 skipped, step 2 writes the content attribute) and the File
    Upload case is stated for both writes. No cell added.
  - E-18 counted as the table states it (holds where the kind at focus loss is one step 2.1 excludes, differs in
    DF-2 (ii) and (iii)) in the summary, D-1 and LR-1. LR-2's allowlist rows add the two class-E rows of
    `content_change_on_blur_tests.rs`. §8: N6 is not in the `[history]` row (it has no edit).
- *Rev 3.6* (2026-11-21): /simplify: restatements collapsed; no decision, expected value or slot content changes
  except LR-2's known requirement 5 (below).
  - LR-2's known requirement 5 narrowed (driver's decision): the component is on every `input` and `<textarea>`; the
    seed does not test the kind and a `type` change does not clear it; the kind conjuncts of step 2.1 (R-2) are one
    predicate inside `take_change_on_blur`. The option "kept correct across every `type` change" is removed. The
    population sentence names every `input` and `<textarea>`; Facet DF-2 states the mechanism, (i)–(iii) are its
    cells; O-4's future edit path writes the component with no step of its own. No DC expected value changes.
  - D-3 stated once in §0.4; §4.1 keeps the derivation, K-last's rejection and where L parts from K-first; the
    "text alone does not determine / §4.10.5.5 leans" qualifiers elsewhere (here, plan (1), the umbrella) are bare
    pointers. PM 305 / 307 / 393: withdrawn part, lost ground and remaining part per record in LR-6 only; D-6 and
    §6.3 point there. Content need defined in LR-3 only. DF-2 (iii)'s Checkbox / Color derivation in §6.2 only.
  - History removed from the body (§3, §4.1, §5, §6.2, §8 cells; PE-4, R-4); plan (1)'s dated parentheticals folded
    into one banner; the errata lines of both landed changelogs reduced to one line each.
  - Pins by content: sister-memo `:NN` and ledger / PORTFOLIO / archive line numbers removed (slot IDs, PM numbers,
    section names and quoted fragments); code pins in LR-2 / LR-4's ledger text replaced by function names.
  - Provenance re-runnable: §0.1's path set is a command over this memo; §6.4 and LR-6 give the ledger's and the
    archive's paths; DoD 3 points to LR-1 for the row count; the summary's E-row count does not depend on row order;
    "the cells whose Ground is D-3" in §6.1, §6.2 and LR-7.

- *Rev 3.5a* (2026-11-21, clerical, PM 11-21): LR-4 O-1's slot draft proposes the ID
  `#11-implicit-submission-default-button-and-field-count`; the PM accepts or renames it at landing.
- *Rev 3.5* (2026-11-21): Step 4.5 second re-check of rev 3.4 (0 CRIT / 0 IMP / 4 MIN).
  - DF-2 (iii) gains a fourth case, the round trip through a value mode non-text kind (Number, Range, the date /
    time kinds): where no sanitization changes the value the expected value is [spec] (as DC-18); where one does,
    the mixed class, D-3 (as DC-19). Two examples, no new cell: Range (`"a"` → `"50"`, `F0` = `""`) and Number
    (`<input value=q>`, `"qa"` / `"aq"` → `""`, `F0` = `"q"`); quote Q18 and the §3 rows §4.10.5.1.12,
    §4.10.5.1.13 and §2.3.4.3 (K = 3, 24 rows over 21 sections; `coverage-map` now prints the split
    recommendation, not taken, §3). The summary, D-3 and LR-2's Facet DF-2 state the D-3 part by its property ("where
    the sanitization of the kind passed through changes the value"), not as "through Color".
  - §3's §4.10.5 row: *update an input element's type* steps 1–3 and 6 (was 1–2); its touch column names DC-18,
    DC-19 and the File Upload case.
  - PM 307's "initial = before": withdrawn because it contradicts Q1 under every reading (DF-1 (a)), not because its
    ground does not hold (D-6, §6.3, LR-6 and its heading). The other parts' wording is unchanged. Plan (1)'s D-1
    erratum names it (`:95`); its W6 erratum (`:500`) and §8 (`:584`) are unchanged.
    The added line moves plan (1)'s later lines by one; §8 and LR-6 cite `:446`, `:500` and `:584` (were `:445`,
    `:499` and `:583`).
  - No expected value of DC-1 to DC-19 changes.
- *Rev 3.4* (2026-11-21): Step 4.5 re-check of the rev 3.3 fixes (`d61a7b4e..df7e759e`; 0 CRIT / 3 IMP / 10 MIN).
  - IMP: DC-18 (the round trip through Checkbox) is [spec] under R-2 and does not rest on D-3: the step-2 write puts
    `"a"` over `"a"`, so no non-user write changes the value and K is complete there (§4.1), as for DC-14 and DC-15.
    K-last's definition made explicit (a write that does not change the value does not move its reference); its
    column for DC-18 is `change`, and no other row's column changes. "(iii) rests on D-3" removed from the summary,
    §0.4 D-3, §4.1 and LR-2. LR-6's PM 307 split: the first-mark record of the value just before the first user edit
    (fail-safe, no script-side calls) stays in force; "initial = before" (the overwrite of the first-focus value) is
    proposed for withdrawal on DF-1 (a), with PM 307 (b); LR-2's known requirement 1 re-decides it with PM 393 D1;
    PM 393 D2's probe step is proposed for withdrawal (PE-5), and plan (1)'s N6 is recorded in §8 without an edit
    (also D-6, §6.3). Plan (1)'s D-1 and W6 errata and §8's W6 line name the parts LR-6 names, not the whole
    records; §8's rows match their text.
  - MIN: R-4 removed (§4.10.23 states it: Q14, the ground of E-2; §3 rows for §4.10.5.1.20 and §4.10.23 now
    "re-derived, E-2"; summary and D-1 no longer cite it). DF-2 (iii)'s population re-stated as a round trip through
    a kind whose snapshot the reconciler clears; new cell DC-19 (through Color: the sanitization gives `"#000000"`,
    the mixed class, `change` by D-3; today none), with Q15–Q17 and the §3 rows §4.10.5.1.14 and CSS COLOR 4 §16.2.1
    (K = 3, 21 rows); File Upload is in the population only when dirty at first focus. §4.1's L-rejection sentence
    re-worded ("wherever `V` returns to the pre-edit value"). D-3's ground no longer re-stated in §1 ("In scope" now
    names D-3 as a source of expected values), §4.1 and the umbrella (`:159`, `:262`); §0.4 notes plan (1)'s "the
    re-derivation's D-3". DoD 3 is `git diff --stat origin/main...HEAD` (the three memos and the allowlist); §0.1's frame extended to the
    merge of `7af70b68` (#650) at `af895c9b`, the premise code unchanged over `29edf6fc..7af70b68`. LR-2's known
    requirements written as a replacement of the row's sentence, the population stated once (with `<textarea>`,
    D-4); "Why-deferred" marked "add". LR-4's O-1 draft gains a Trigger and a Why-deferred.
  - No expected value of DC-1 to DC-17 changes.
- *Rev 3.3* (2026-11-21): plan-review round 2 fixes (on `8a11ff6b`: 0 CRIT / 4 IMP / 17 MIN / 2 FP).
  - IMP: DF-2 (iii), the `type` round trip text → Checkbox → text after a user edit (§6.2, new cell DC-18,
    expected `change`, today none; §4.1 table row; E-18's verdict is decided by the kind at focus loss; LR-2). The
    frame re-measured after the merge of `1b727d18` (§0.1; the premise code is unchanged): 18 allowlist rows
    deleted (17 T + 1 E), ratchet `ok (495 allowlisted, 0 new, 0 stale)` (LR-1, DoD 2, DoD 3). LR-2 rewritten as
    an amendment of the registered row `#11-change-on-blur-baseline-first-focus-value` (ledger `:1736`, PM 861): DF-1
    (b) not a defect with DC-17 and DC-18, DF-2 (iii), the allowlist-row list without the 3 rows this PR deletes, a
    trigger, the slice's plan-review decides the DF-1 / DF-2 split, known requirements 1, 3 and 5 extended. LR-6
    narrowed to the parts of PM 305, PM 307 (b) and PM 393 D1 whose ground does not hold; the rest stays in force;
    the PM settles the range at landing (also D-6, §6.3).
  - MIN: "under R-2" on the DF-2 claims and the count of §4 re-stated; new reading R-4 (the user-started reset, W8)
    for E-2; L's rejection is the user's choice, explained by K-first's comparison, not by "the whole difference is
    the script's" (§4.1; plan (1)'s M1 / M9 rows likewise); §8 "M1 (mixed class)"; §0.4 "the behavioural
    exception"; §3 rows for §4.10.5.1.16, §4.10.5.1.20 and §4.10.23; the lens row "Ground order" and rung 3 (not
    consulted, because the user's choice came first); D-3's ground stated once in §0.4 and referred to as "D-3"
    elsewhere, in this memo and in the two landed memos; LR-7 records PM 870's FYI to the user; LR-4 homes the
    defects (O-1: a slot draft; O-2 (i): `#11-blur-focus-cleared-before-blur-event`, PM 861) and hands O-3's two
    untraced paths to the PM; the compat condition for L and K-last in the form of LR-3 (§4.1); LR-2's known
    requirement 3 places the seed rule in one engine-independent crate; P14's citation `focus.rs:142` → `:147` (the
    call; `:142` is the function). O-2 (i) / (ii) re-stated after #652 (the allowlist now lists the comment).
  - Recorded: the merge `d61a7b4e` tagged plan (1)'s E1 correction line `[contrast]` (`:58`; §8).
  - No expected value of DC-1 to DC-17 changes.
- *Rev 3.2* (2026-11-21): ground of D-3 corrected (PM 870). Rev 3 copied PM 866's ground sentence "§4.10.5.5 decides
  (b) at rung 1"; it over-reached. The outcomes of class (b), M1, M6 and S* follow from the K-first predicate the
  user chose; step 2.1's text alone does not decide them, and §4.10.5.5 stays a lean that K-first applies only to the
  value before the first user edit. No step label (§0.4 exception for D-3); the ground is written "D-3 (K-first, the
  user's choice, PM 866; ground corrected PM 870)". Rev 3's "superseded" mark on the Q5 paragraph of §4.1 withdrawn:
  its reading stands. New sequence S* (script `x`; user `a`, Backspace; script `y`): §4.1 table row and cell DC-17,
  expected `change` (Today `change`). Applied to the summary, §0.4, §3 row, E-5, §4.1, §6.1, §6.3, LR-2 DF-1 (b) (and
  its cell list), LR-7, §8, DoD 4; in the landed memos: plan (1) M1 / M9 rows, §1 (b) erratum (`[history]` on
  "settled by measurement"), W6 erratum, changelog; umbrella §3 step 2.1 row, §7, changelog. No other expected value
  changes.
- *Rev 3.1* (2026-11-21): downstream sweep of D-3 into the two landed memos (plan (1) opening table M1 / M9 and the W6
  erratum; umbrella §3 step 2.1 row and §7 "What differs"); §8 rows re-worded to describe those edits.
- **Rev 3 (2026-11-21)** — D-3 decided by the PM: **K-first** (PM 866, 2026-11-20, the user's choice). Ground: rung
  1, the spec — step 2.1's text alone is undetermined, and §4.10.5.5 (Q5, re-checked with `.claude/tools/webref body
  html common-input-element-events` on 2026-11-21) decides class (b): no `change`; M1 and M6 fire. K-last and L
  rejected (§4.1); "closest to what landed" is not a ground; L and K-last go to compat only if content needs them.
  Applied to the summary, §0.4 D-3, §1, §3 rows, E-5, §4.1, §6.1 (DC-2 to DC-7, DC-9, DC-11, DC-13 no longer
  pending), §6.3, LR-1, LR-2 (DF-1 (b) not a defect; known requirement 1), LR-7 (answered) and DoD 4.
- **Rev 2 (2026-11-20)** — plan-review R1 fixes. §4.1: class (a) is [spec] under every reading; class (b), M1 and M6
  are spec-undetermined; the principle lens does not select a reading, so **D-3 is open and referred to the PM**
  (LR-7), and the (b) cells are "pending D-3". M1 contradiction resolved (K is undefined on M1). Readings labelled
  (§0.5 R-1 to R-3). "PM 776" → PM 842 for the docs-only rule. PM 305 / 307 / 393: not withdrawn here; LR-6 asks the
  PM. O-1 to O-4 in defect form (§6.4). New defect DF-2 (snapshot population across a `type` change; E-19, DC-14 to
  DC-16). LR-2: re-eval date, umbrella slice + plan-review, known requirements, allowlist row. LR-3: what counts as
  a content need. Corrections stated once (§6.3); the landed memos' errata are pointers. Slot IDs in place of the
  CP-3 / CP-4 aliases. Allowlist prune count 15 (14 T + 1 E). §4.2 and §5 step 4: over-stated ECS arguments removed.
- **Rev 1 (2026-11-20)** — initial re-derivation (PM 432, PM 842).
