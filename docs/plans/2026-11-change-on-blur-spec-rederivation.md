# Change-on-blur — model H re-derived from HTML (spec → ECS principles → evidence)

Slot `#11-change-on-blur-model-spec-rederivation` (owner Layout; PM 432, PM 842), with
`#11-implicit-submission-commits-change` folded in (PM 842). Rev 3, 2026-11-21. Branch
`layout-change-on-blur-rederivation`. **Docs only**: this memo changes no code and plans no code change (PM 842).

**Defect this memo removes.** The landed change-on-blur model **H** (#597, `e0e18713`) names a measurement of one
engine as its ground (umbrella `docs/plans/2026-11-focus-fixup-program.md` DC2, row (1), §3; plan (1)
`docs/plans/2026-11-change-on-blur-user-provenance.md` D-1, §4.3). Under CLAUDE.md's ground order a core requirement
comes from the spec, then from elidex's own principles where the spec is silent; a measurement is at most `[evidence]`.

**Result.**

- Of the 19 rows of §4, 16 hold: 14 on spec text, 1 on spec text plus an elidex principle (E-15), 1 on an elidex
  principle alone (E-13, `<textarea>`, where the spec is silent). No measurement is needed as a ground for any of them.
  1 row (E-16, user validity) is not an element of H. (E-18 is counted as holding: it holds for a flip to a kind
  that step 2.1 excludes; its other part is DF-2 (ii).)
- **The baseline** (E-5) differs from the spec. HTML compares with the value "when the control was first focused"; H
  compares with the value just before the first user value change. They differ only when a non-user write lands
  between focus and the first user value change. Defect **DF-1** (§6) is class (a); class (b) is examined with it:
  - **(a)** the value at focus loss equals the first-focus value → H fires `change`; the spec fires none under every
    reading of the sentence. **[spec]**.
  - **(b)** the value differs from the first-focus value and the user's edits net to nothing → H fires none. Step
    2.1's text alone does **not** determine this class (§4.1); §4.10.5.5 does: **D-3 is decided, K-first** (PM 866),
    so no `change` fires. That is H's outcome: class (b) is **not a defect** **[spec]**. The same holds for M1 and
    M6, where the user did change the value and H fires `change`, as K-first does.
- **The snapshot's population** (E-19) differs from the spec: a control whose `type` changes from a non-text kind to
  a text kind while focused has no snapshot, so a user edit there fires no `change`; and a user-edited text control
  whose `type` changes to Number, Range or a date / time kind loses its mark. Defect **DF-2** (§6). **[spec]**.
- Both fixes are a separate code PR (PM 842).
- Implicit submission: the current order (`submit`, then `change` at a later focus loss) **conforms** (§5).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is read at `origin/main` **`29edf6fc`** (`git log --oneline -1` → `29edf6fc`), this branch's base.
Spec text is quoted from `.claude/tools/webref body …`, fetched 2026-11-20 (commands in §3).

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
| P14 | The snapshot is written at four instants, not only on the focus transition: the seed (the shell sink after `focus` / `focusin`, and VM `focus()`), the mark (the edit path), the clear (a `type` flip to a non-text kind while focused), the take (focus loss). A `type` flip from a non-text kind to a text kind writes nothing | seed `update_steps.rs:310-311`, `crates/shell/elidex-shell/src/content/focus.rs:142`, `html_element_proto.rs:311`; mark `user_edit.rs:62-64`; clear `reconciler.rs:165-167`; take `focus_snapshot.rs:87-95` |
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
  and no `change` fires. For the mixed class where the value differs, the text is undetermined (§4.1); rev 1 of this
  memo overstated PE-4 as "the spec decides the mixed case". See E-5.
- **PE-5 (plan (1) §4.4)** — "Replacing a selection with identical text was not measured either; I2 follows elidex's
  own value comparison". §4.10.5.5 names that case as a non-change (Q4). The spec decides it; no probe is needed.

### §0.4 Decisions

Each line names the ground-order step that decided it.

- **D-1 The mark, the funnel, the provenance rule, the commit point, the event order and the kind boundary hold on spec
  text** — [spec]: §4.10.5.5, §6.6.4 step 2.1, §2.1.4, UI Events §3.3.2 (table §4, E-1 to E-4, E-6 to E-12, E-14, E-17,
  E-18). E-14 and E-18 hold under reading R-2 (§0.5); across a `type` change the kind boundary differs (DF-2).
- **D-2 No `change` fires at step 2.1 when the value equals the value at first focus** — [spec]: §6.6.4 step 2.1's
  parenthetical (Q1), under every reading of it (§4.1). H's baseline (the value before the first user value change)
  fires one in class (a). It has no ground in spec text or in an elidex principle; its only source is the #597
  measurement [history]. Recorded as defect DF-1 (a); this memo does not change the behaviour.
- **D-3 The mixed class — decided: K-first (PM 866, 2026-11-20, the user's choice)** — [spec]. When a non-user
  write and a user value change both happened while the control was focused, the value differs from the first-focus
  value, and the readings of §4.1 split (class (b), M1 and M6), `change` fires iff the user changed the value,
  `V ≠ F0`, and `V` differs from the value just before the first user value change (K-first, §4.1). Step 2.1's text
  alone does not determine the class (§4.1). §4.10.5.5 decides class (b) (Q5: "These events are not fired in
  response to changes made to the values of form controls by scripts."): the user's edits net to nothing, the whole
  difference from `F0` is the script's, and no `change` fires. In M1 and M6 the user did change the value, and
  `change` fires. K-last and L are rejected (§4.1). "K-first is closest to what landed" is not a ground
  (後方互換性は維持しない). The readings not chosen (L, K-last) go to the compat layer only if content needs them.
- **D-4 `<textarea>` fires change-on-blur under the same predicate as `<input>`** — [spec] silent (PE-2);
  [principle] decides (§4.2): One issue, one way — the text-control class has one edit funnel and one commit
  mechanism. ECS-native first does not decide it by itself (§4.2).
- **D-5 Implicit submission is not a commit point in core** — [spec] §4.10.5.5 and §4.10.22.2 for the event set;
  [principle] (One issue, one way) for the user-interface classification the spec leaves to the user agent (§5). A
  different order is a `[compat]` concern.
- **D-6 The #597 measurements stay in plan (1) as `[history]`** and are a ground for nothing. Three PM decisions cite
  them or follow from them: PM 305 (the choice of H), PM 307 (W6 as an accepted divergence) and PM 393 (the
  `Option<String>` representation). The grounds those records cite do not hold (§6.3). They are the PM's decisions:
  the PM withdraws them at landing (LR-6). This memo withdraws nothing.

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

## §1. Scope

In scope: the grounds of every element of H (§4); the implicit-submission decision (§5); the differences as defects
with the cells a later code PR adds (§6), where the spec or a principle decides the expected value; the ledger
records (§7); ground corrections in the two landed memos (§8).

Out of scope: any code change; the plan of DF-1's and DF-2's fix (PM 842); the choice between the readings of §4.1
(D-3, the PM's: decided by PM 866, recorded here); VM `focus()` / `blur()` events (A2c); user validity at step 2.1.1
(`#11-focus-change-user-validity`); the four defect slots plan (1) registered (umbrella §4).

**Shape.** One docs PR. Not edge-dense in itself: it adds no mechanism and couples no new invariant. It is plan-reviewed
because it changes a ratified ground (D-2) and records behaviour differences (DF-1, DF-2).

## §2. Coupled invariants

None new. The memo touches two existing ones and leaves both intact.

- **Plan (1) I3 (provenance lifetime = snapshot lifetime) × D-5.** A commit at Enter would need a second consumer of
  the snapshot and a re-seed outside a focus transition. D-5 keeps I3 as landed.
- **Umbrella X1 (one judgement, every sink) × DF-1.** DF-1 is a defect in what the one judgement compares with, not in
  where it is computed. X1, plan (1) I1, I2, I4, I5, I6 and I7 are untouched by DF-1.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §6.6.4 Processing model | *focus update steps* step 2.1 | all four conjuncts; the parenthetical's reference point (first focused); the parenthetical's two readings (§4.1; D-3 decided, K-first, PM 866) | re-derived, E-4 to E-8, E-18, E-19; DF-1, DF-2 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focus update steps* steps 2.1 → 2.4, 4.1 | `change` before `blur`; the old entry stays the focused area during `change` | re-derived, E-10 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focusing steps* step 5; *unfocusing steps* step 8 | already focused → return; every focus loss reaches the focus update steps | re-derived, E-8, E-17 | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | the three user-interface classes; "without an explicit commit action" | text control = no commit action; `change` at focus loss | re-derived, E-1, E-3, E-4, E-11, E-15 | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | "not fired in response to changes made … by scripts"; state restoration; "on behalf of the user" | non-user writers fire nothing and do not mark; class (b) fires no `change` (D-3) | re-derived, E-2, E-6, D-3 | ✓ | yes |
| WHATWG HTML §4.10.22.2 Implicit submission | default button → `click`; no submit button → the blocking-field count, then submit | names no `change`, no focus change | re-derived, E-15, §5 | ✓ | yes |
| WHATWG HTML §4.10.11 The textarea element | user changes the raw value → `input`; no `change` rule | spec silent on `change`; decided by principle | re-derived, E-13 | ✓ | yes |
| WHATWG HTML §4.10.7 The select element | *send select update notifications* | `input` then `change`, own commit | boundary only, E-14; `#11-select-keyboard-change-event` | ✗ | yes |
| WHATWG HTML §4.10.5.1.15 Checkbox state (type=checkbox) | input activation behavior | `input` then `change`, own commit | boundary only, E-14 | ✗ | yes |
| WHATWG HTML §2.1.4 DOM trees | removing steps, step 2 and its note | removal of the focused area fires no `blur`, no `change` | re-derived, E-9 | ✓ | no |
| WHATWG HTML §4.10.18.1 A form control's value | user validity; dirty value flag | not H (E-16); the dirty flag has its own predicate (plan (1) D-5) | pointer only | ✗ | no |
| UIEVENTS §3.3.2 Focus Event Order | rows 3–4 | `focusout` follows `blur` | re-derived, E-10 | ✓ | yes |
| WHATWG HTML §4.10.5 The input element | activation behavior of `input` elements; input type change steps step 2 | every `input` has activation behavior (R-2); the value written by a `type` change (R-3) | readings only (§0.5); DF-2 | ✗ | yes |
| WHATWG HTML §6.6.6 Focus management APIs | `activeElement` getter steps | reads the focused area's DOM anchor | observation only, O-2 | ✗ | no |
| WHATWG HTML §8.1.7.2 Queuing tasks | queue an element task | → queue a global task | observation only, O-3 | ✗ | yes |
| WHATWG HTML §8.1.7.3 Processing model | event loop step 2.1 | the task queue is chosen in an implementation-defined manner | observation only, O-3 | ✗ | yes |

Numbers and titles: `.claude/tools/webref coverage-map html 6.6.4 html 4.10.5.5 html 4.10.22.2 html 4.10.11 html 4.10.7
html 4.10.5.1.15 html 2.1.4 html 4.10.18.1 uievents 3.3.2 html 4.10.5 html 6.6.6 html 8.1.7.2 html 8.1.7.3` (run 2026-11-20; anchors `#focus-processing-model`,
`#common-input-element-events`, `#implicit-submission`, `#the-textarea-element`, `#the-select-element`,
`#checkbox-state-(type=checkbox)`, `#dom-trees`, `#a-form-control's-value`, `#events-focusevent-event-order`,
`#the-input-element`, `#focus-management-apis`, `#queuing-tasks`, `#event-loop-processing-model`).
Breadth: K = 2 (html, uievents), M = 16 → single PR scope. Also looked up: `webref dfn html 'unload a document'` →
§7.5.9 Unloading documents; `webref body html unloading-documents | /usr/bin/grep -c -i 'focus\|blur\|change event'`
→ 0 (control: `-c -i 'unload'` on the same output → 25). The preflight does not know the label `UIEVENTS` that
`coverage-map` prints, so it skips that row; its number and title were checked by hand
(`.claude/tools/webref heading uievents 3.3` → §3.3.2 Focus Event Order `#events-focusevent-event-order`).

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

### §3.2 User-input touch audit

Nothing is added on a user-input path. The audit of the existing paths is plan (1) §3.1 and stays as it is.

## §4. The re-derivation

Verdict column: **holds** = H's behaviour is what the named ground requires; **differs** = it is not.

| # | H element (as landed) | Spec ground | Principle ground (spec silent) | Evidence | Verdict |
|---|---|---|---|---|---|
| E-1 | The mark is set when a user edit changes the value: the value after the edit differs from the value before it (P2). A caret move, a refused paste and an identical replacement do not mark | Q1 "the user has changed the element's value"; Q3 "any time the user causes the element's value to change"; Q4 lists the non-changes (PE-5) | — | plan (1) §6 N1–N4 [history], not a ground | holds [spec] |
| E-2 | No non-user writer marks: the `value` setters, `defaultValue`, the `value` attribute, type-change and `multiple` sanitization, `setRangeText`, form reset (plan (1) W1–W8) | Q5. A reset and a sanitization are the user agent's algorithms, not "the user" of Q1 | — | — | holds [spec] |
| E-3 | `input` fires iff the user changed the value; bubbles and composed | Q3 (input), Q8 (textarea) | — | — | holds [spec] |
| E-4 | A text control's only commit point is the loss of focus | Q3: typing is a change "without an explicit commit action" (Q4 names typing and pasting), and its `change` "will be fired when the control loses focus"; Q1 fires it | — | — | holds [spec] |
| E-5 | **The baseline**: the value just before the first user value change since focus (P1, P3). A non-user write made before that change moves the baseline | Q1: "different to what it was when the control was first focused". Under every reading, no `change` fires when the value equals the first-focus value (§4.1). For the mixed class where the value differs, Q1 alone does not determine the outcome (§4.1); Q5 decides class (b): no `change`; in M1 and M6 the user did change the value: `change` (D-3, K-first, PM 866) | not reached for the mixed class: rung 1 decides it (D-3). The lens alone did not select a reading (§4.1) | the #597 runs M4, M9 and W3–W8, measured on one engine [history]; a ground for nothing | **differs** in class (a) [spec] → DF-1 (a). Class (b), M1, M6: holds [spec] (D-3) |
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
| E-18 | A `type` flip to a non-text kind while focused clears the snapshot, so no `change` fires at blur (P4) | Q1's conjuncts are evaluated at blur: "the change event applies to the element, and the element does not have a defined activation behavior" (R-2) | — | — | holds [spec] under R-2 for a flip to Checkbox, Radio Button, File Upload, Color, Hidden or a button state. A flip to Number, Range or a date / time kind is cleared too, though step 2.1 still applies to those kinds: **differs** → DF-2 (ii) |
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
  non-user write (the user's edits since that write net to a change).

| Sequence while focused (plan (1) §4.3 names) | `F0` | `V` | L | K-first | K-last | H as landed |
|---|---|---|---|---|---|---|
| class (a), M4 (cell C6): script `x`; user Backspace | `""` | `""` | none | none | none | **`change`** |
| M2: user types `a`; script `""` | `""` | `""` | none | none | none | none |
| M3: script `x`; user types `a` | `""` | `"xa"` | `change` | `change` | `change` | `change` |
| M8, second session: script `x`; user types `b` | `"a"` | `"xb"` | `change` | `change` | `change` | `change` |
| M1 (cell C4): user types `a`; script `third` | `""` | `"third"` | `change` | `change` | none | `change` |
| M6: script `x`; user types `a`; script `y` | `""` | `"y"` | `change` | `change` | none | `change` |
| class (b), M9 (cells C20–C26, C7 (l)): script `x`; user types `y`, Backspace | `"a"` | `"x"` | `change` | none | none | none |

So:

- **Class (a) is decided by the spec under every reading**: no `change`. H fires one. DF-1 (a).
- **M2, M3 and M8 are not in dispute**: every reading gives H's outcome.
- **Class (b), M1 and M6 are not determined by step 2.1's text alone.** On M1 the correction to rev 1: rev 1 said L
  and K agree on M1 and also that K is undefined on it. The second statement is the true one. K-first and K-last
  differ on M1 and M6, so both belong with class (b). D-3 decides the three (below): K-first's column.

**The weight of Q5.** "These events are not fired in response to changes made to the values of form controls by
scripts." In class (b), and in M1 and M6, the value's difference from `F0` at focus loss comes only from a script
write; the user's own edits left nothing. A `change` there reports a difference the script made. That leans toward K,
and rev 1's rebuttal ("no `change` fires without a user change of the value") did not answer it: the user's change is
necessary for the event under L, but so is the script's write. Against the lean: Q5's stated purpose ("to make it
easier to update the values of form controls in response to the user manipulating the controls, without having to
then filter out the script's own changes to avoid an infinite loop") is met by every reading, since under none of
them does a script write alone fire an event; and K-first does not apply the lean evenly (it fires on M1 and M6,
where the difference is also only the script's). Q5 leans; it does not decide. Q13 (non-normative: "when the user
commits a value change") leans the same way for class (b). *[Rev 3: superseded by D-3 below. The PM reads Q5 as
deciding class (b) at rung 1, and answers the remark on M1 and M6: there the user did change the value.]*

**The principle rung — lens.** One line per principle of CLAUDE.md's Design philosophy and Design discipline:
which reading it selects or rejects.

| Principle | Selects / rejects | Why |
|---|---|---|
| Ground order (spec → principles → evidence) | selects none; rejects the #597 measurement as a ground for K-first | the text is undetermined here; Q5 leans K without deciding (rev 3: the PM reads Q5 as deciding class (b), D-3); a measurement is `[evidence]` at most and is reached only if this table selects nothing |
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
selects K, and none rejects it. That is not a decision by principle. Rev 1's three grounds for L do not survive
as stated: ground 1 called L "the sentence as written" (it is one of two readings); ground 2 counted stored facts and
named the seed as `F0`'s one writer (today the seed stores no value, and it has two call sites, P14); ground 3
(no non-user writer calls anything) holds for K-first and K-last as well.

**D-3 is therefore not decided by the lens.** The designer did not pick (brief, PM 776 scope rule) and referred
the question to the PM (LR-7): for class (b), M1 and M6, which of L, K-first, K-last (or another completion of K) is
elidex's core behaviour, and on which ground.

**D-3 — decided: K-first (PM 866, 2026-11-20, the user's choice). Ground: rung 1, the spec.**

- Step 2.1's text alone is undetermined (above). §4.10.5.5 decides class (b): Q5, "These events are not fired in
  response to changes made to the values of form controls by scripts." In class (b) the user's edits net to nothing
  and the whole difference from `F0` is the script's: no `change`. In M1 and M6 the user did change the value:
  `change`.
- **K-last is rejected** for the reason that defeats `change`'s purpose: a formatter script that rewrites the value
  on every `input` leaves the value equal to the latest non-user write, so `change` would never fire after the user
  commits.
- **L is rejected** for class (b), the case where the whole difference is the script's.
- **Not a ground:** "K-first is closest to what landed" (後方互換性は維持しない; lens row above).
- The readings not chosen (L, K-last) go to the compat layer only if content needs them. None is registered.
- Rung 3 (the #597 runs) is not reached.

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
     second instant. Rev 1 said every writer sits on the focus transition; that was wrong (P14), and the argument
     drawn from it (a re-seed "outside a focus transition" would be a new kind of write) is withdrawn.
   D-5 does not depend on the withdrawn argument: steps 1–2 are spec text, and One issue, one way decides the
   residual of step 3.
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
- **(b) [spec: Q5; D-3, PM 866]** the user's edits net to nothing and the value differs from the first-focus
  value → elidex fires no `change`. Step 2.1's text alone does not determine the outcome (§4.1); §4.10.5.5 (Q5)
  does: no `change` (D-3, K-first). elidex conforms. **Not a defect.**

M1 and M6 are in the same mixed class with the opposite sign: elidex fires `change` there, which is K-first's
outcome (D-3: the user did change the value). elidex conforms. Not a defect.

*Reference instant* (reading R-1, the memo's inference, not text): "first focused" is the designation of the control
as the focused area (focus update steps step 4.1), which precedes the `focus` event (step 4.4), once per focus
session. Today the seed runs after the `focus` / `focusin` listeners
(`crates/dom/elidex-dom-api/src/focus/update_steps.rs:304-311`) and stores no value (P1).

*Not affected.* The funnel, the mark, `input`, the one-judgement rule, the event order
(E-1 to E-4, E-6 to E-15, E-17).

*Cells the code PR adds or re-states.* The Ground column is the source of the expected value. "UA focus" and
"UA blur" are the shell's user focus moves, as in plan (1) §6. The cells of class (b), M1 and M6 (DC-2 to DC-7,
DC-9, DC-11, DC-13) take their expected value from D-3 (K-first, PM 866); in rev 2 they were pending. Their "Today"
already equals it.

| Cell | Sequence | Expected | Ground | Today |
|---|---|---|---|---|
| DC-1 (re-states C6), both kinds | `<input id=b value="">`; UA focus `b`; script `b.value = "x"`; user Backspace (value `""`); UA blur | no `change` | [spec] Q1 (F), every reading: the value equals the first-focus value | `change` |
| DC-2 (re-states C20), both kinds | UA focus `b`; user types `a`; UA blur (`change`); UA focus `b`; script `b.value = "x"`; user types `y`, Backspace (value `"x"`); UA blur | no `change` at the second blur | [spec] Q5 (D-3, K-first, PM 866): class (b), `"x"` ≠ `"a"`, the user's edits net to nothing, so the difference is the script's | none |
| DC-3 (re-states C21) | `<textarea id=b>p</textarea>`; UA focus; script `b.defaultValue = "q"`; user `z`, Backspace; UA blur | no `change` | [spec] Q5 (D-3), class (b) (`"q"` ≠ `"p"`); and E-13 [principle] for the kind | none |
| DC-4 (re-states C22) | `<input id=b value="p">`; UA focus; script `b.setAttribute("value", "q")`; user `z`, Backspace; UA blur | no `change` | [spec] Q5 (D-3), class (b) | none |
| DC-5 (re-states C23) | `<input id=b value=" a">`; UA focus; script `b.type = "url"` (value `"a"`); user `z`, Backspace; UA blur | no `change` | [spec] Q5 (D-3), class (b) (`"a"` ≠ `" a"`) | none |
| DC-6 (re-states C25) | `<input id=b value="p">`; UA focus; script `b.setRangeText("q", 0, 0)` (value `"qp"`); user `z`, Backspace; UA blur | no `change` | [spec] Q5 (D-3), class (b) | none |
| DC-7 (re-states C26) | `<form id=f><input id=b value="d"></form>`; script `b.value = "x"` before focus; UA focus; script `f.reset()` (value `"d"`); user `z`, Backspace; UA blur | no `change` | [spec] Q5 (D-3), class (b) (`"d"` ≠ `"x"`) | none |
| DC-8 (new), both kinds | UA focus `b` (value `""`); script `b.value = "x"`; user types `a` (value `"xa"`); user Backspace ×2 (value `""`); UA blur | no `change` | [spec] Q1 (F), every reading | `change` |
| DC-9 (re-states C24) | `<input id=b type=email value="a@b, c@d">`; UA focus; script `b.setAttribute("multiple", "")` (value `"a@b,c@d"`); user `z`, Backspace; UA blur | no `change` | [spec] Q5 (D-3), class (b). The landed expected value rested on PM 307, whose cited ground does not hold (§6.3); the cell's ground is now D-3, not PM 307 | none |
| DC-10 (new) | `<input id=b value="">` with a `focus` listener that sets `b.value = "x"`; UA focus `b`; user Backspace (value `""`); UA blur | no `change` | [spec] Q1 (F) for the comparison; **reading R-1** for the instant (the first-focus value is taken at step 4.1, before the listener runs). Under a reading that takes it after `focus`, this is class (b)'s mirror and the cell changes | `change` |
| DC-11 (unit; re-states C7 (l)) | seed at `""`; a non-user write to `"x"`; `apply_user_edit` appends `y`; a second `apply_user_edit` removes it (value `"x"`) | the judgement returns `false` | [spec] Q5 (D-3, K-first), class (b) | `false` |
| DC-12 (control, unchanged) | C1, C3, C5, C9, C14, C16, C18, C19 of plan (1) | as today | [spec] E-6, E-7, and M8: every reading gives H's outcome (§4.1) | green |
| DC-13 (control, unchanged; D-3 chose K-first) | C4 (M1), C17 (M6) of plan (1) | `change` (as today) | [spec] Q1 (U) and (F) under D-3 (K-first, PM 866): the user did change the value (§4.1) | `change` |

The code PR also rewrites the two code comments that name the measurement as the ground
(`focus_snapshot.rs:4-5`, `:69-71`; the second is the ECS-mission allowlist's class-E row for that file, LR-2).

This memo does not plan the fix (PM 842).

### §6.2 DF-2 The snapshot's population follows the kind at each write, not step 2.1's conjuncts at focus loss

*Defect.* Step 2.1 evaluates its conjuncts when focus is lost (Q1: "If entry is an input element, and the change
event applies to the element, and the element does not have a defined activation behavior, and the user has changed
the element's value … while the control was focused"). elidex decides who carries the snapshot by `is_text_control`
at the seed and at each `type` change (P4, P14, P16). Two sequences where the two disagree:

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
  survives the type-change sanitization still requires `change` at blur. elidex fires none. (Found while writing
  (i); the brief named only (i).)

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

*Cells* (for the code PR; DC-14 and DC-15 do not depend on D-3: no non-user write changes the value).

| Cell | Sequence | Expected | Ground | Today |
|---|---|---|---|---|
| DC-14 (new) | `<input id=b type=checkbox>` (no `value` attribute), `<input id=c>`; UA focus `b` by Tab; script `b.type = "text"` (value `""`); user types `a`; UA blur | `input`, then `change` at blur | [spec] Q1, all conjuncts at focus loss, under R-2; `"a"` differs from the first-focus value under R-3 (`""`) and also under the IDL-result reading (`"on"`) | `input`, no `change` |
| DC-15 (new) | `<input id=b>`; UA focus `b`; user types `1`; script `b.type = "number"` (value `"1"`); UA blur | `change` | [spec] Q1 at focus loss, under R-2: `change` applies to the Number state, the user changed the value while focused, `"1"` ≠ `""` | none |
| DC-16 (control) | `<input id=b>`; UA focus; user types `a`; script `b.type = "checkbox"`; UA blur | no `change` at blur | [spec] under R-2 (E-18): the Checkbox state has a defined input activation behavior | none (green) |

This memo does not plan the fix (PM 842).

### §6.3 The corrections, stated once

The two landed memos point here; they do not restate these.

- **What the landed cells pin.** Until the code PR lands:
  - C6 pins behaviour that contradicts the spec text (class (a), DF-1 (a)).
  - C7 (l) and C20–C26 pin class (b)'s outcome, and C4 and C17 M1's and M6's. Step 2.1's text alone does not
    determine them; D-3 does (K-first, PM 866: §4.10.5.5 Q5), and each pins K-first's outcome. They hold [spec].
  - Every other cell of plan (1) holds on the ground §4 names for its element.
- **The first-focus argument** is §4.1: the sentence fixes `V ≠ F0` as necessary and leaves the mixed class
  undetermined; §4.10.5.5 decides it (D-3, PM 866). "The spec decides the mixed case" (this memo's rev 1 and the
  errata it added to the landed memos) was wrong as a claim about step 2.1's sentence: rev 1 drew it from that
  sentence alone, for class (b), M1 and M6.
- **The `<textarea>` ground** is §4.2 (One issue, one way).
- **The grounds three PM records cite.** The records are the PM's; the PM withdraws them (LR-6). What this memo
  establishes is only that the cited ground does not hold:
  - *PM 305* (the choice of H) cites the #597 measurement. A measurement is not a ground for a core requirement
    (ground order), and H's outcome in class (a) contradicts Q1 under every reading (§4.1).
  - *PM 307* (W6 as an accepted divergence) cites "the spec defines no baseline", and makes agreement of another
    engine the condition to reopen. Q1 names the reference value (the value at first focus), and an engine's
    behaviour is not an acceptance criterion. C24's outcome itself is class (b): it holds on D-3 (K-first, Q5),
    not on PM 307's ground.
  - *PM 393* (`FocusValueSnapshot(Option<String>)`) cites "the focus-time `initial` was dead state". Class (a)
    needs the first-focus value at focus loss under every reading, so it is not dead state.

### §6.4 Observations outside H (for the PM to home)

Found while reading the code for §0.2. None is an element of H; none is fixed or planned here, and no home is
proposed. Each states whether it is a defect against the spec. Ledger checks are `/usr/bin/grep -n -i` runs over the
slot ledger on 2026-11-20; "0 rows" means that grep only.

- **O-1 Implicit submission skips the default button and the blocking-field count. Defect against the spec: yes.**
  On Enter in a single-line text control the shell dispatches `submit` at the form ancestor of the focused control
  (`crates/shell/elidex-shell/src/content/event_handlers.rs:574-577` →
  `crates/shell/elidex-shell/src/content/form_input.rs:129-143`), with no default-button lookup and no field count.
  HTML §4.10.22.2 (Q7): implicit submission of a form "whose default button has activation behavior and is not
  disabled, must cause the user agent to fire a click event at that default button", and "If the form has no submit
  button, then the implicit submission mechanism must perform the following steps: 1. If the form has more than one
  field that blocks implicit submission, then return." So elidex (1) fires no `click` at a default button, and
  (2) submits a form that has no submit button and two or more blocking fields. *Owner today: none.*
  `#11-form-submission-dispatcher` is closed (ledger `:961`, D-29, #209); ledger greps for `default button` and
  `blocks implicit` → 0 rows (control: `implicit submission` → 1 row, the slot LR-3 closes).
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
    observable. *Owner today: none.* The ECS-mission allowlist has 0 rows under
    `crates/dom/elidex-dom-api/src/focus/` (control: `elidex-dom-api` → 21 rows), and the ledger row of
    `#11-core-browser-parity-deviations` (`:1365`) has no focus or blur entry (control on the same row: `crypto`
    matches).
  - **(ii) Why the ratchet did not list the comment.** A gap in the ratchet's vocabulary. The PM has routed it to
    CI/tools; it is not described further here and no tool is changed by this PR.
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
    after its own `keydown` listener made the document not fully active was not traced.
- **O-4 The snapshot's population is `is_text_control`; step 2.1's (under R-2) is every `input` to which `change`
  applies and that has no defined input activation behavior**, which also covers Number, Range and the date / time
  kinds. **Not a defect while the kind does not change during focus**: those kinds have no user edit path (P12), so
  "the user has changed the element's value" cannot hold for them and step 2.1 cannot fire. Number's missing edit
  path is `#11-number-input-user-edit-path`. **A defect when the kind changes during focus**: DF-2. A future user
  edit path for one of those kinds must write the same component; that is a requirement on that path's PR, not a
  defect today.

## §7. Ledger records (applied by the PM at landing)

- **LR-1 → `#11-change-on-blur-model-spec-rederivation`: close** (D-3 is decided, PM 866; LR-7). Record: "Re-derived in
  `docs/plans/2026-11-change-on-blur-spec-rederivation.md`. 16 elements of H hold on spec text or an elidex
  principle; the measurement is `[history]` and a ground for nothing. Two elements differ (the baseline, the
  snapshot's population) → LR-2. The PR pruned 15 allowlist rows: the slot's 14 class-T rows (13 in plan (1), 1 in
  the umbrella), and 1 class-E row (slot `-`) for plan (1) §8's line recording W6 as a divergence from one engine,
  which the PR tagged `[history]` when it added the dated pointer to that line; a tagged line's row is stale and
  the allowlist only shrinks."
- **LR-2 → new slot, ID proposed `#11-change-on-blur-baseline-first-focus-value`** (owner Layout; the PM draws the ID;
  the PM may give DF-2 its own slot). **Re-eval 2026-12-15** (the date of the parent slot it replaces).
  - *Defect DF-1 (a)* — **[spec]**: "`take_change_on_blur` compares the value at focus loss with the value just
    before the first user value change; HTML §6.6.4 focus update steps step 2.1 compares it with the value when the
    control was first focused. After a non-user write between focus and the first user value change, elidex fires
    `change` when the user's edits return the value to its first-focus value; the spec fires none under every reading
    of the step. Cells DC-1, DC-8, DC-10."
  - *DF-1 (b)* — **[spec; D-3, PM 866]: not a defect**: "In the same situation, when the user's edits net to
    nothing and the value differs from its first-focus value, elidex fires no `change`. The step's text alone does
    not determine the outcome (memo §4.1); HTML §4.10.5.5 Common event behaviors ("These events are not fired in
    response to changes made to the values of form controls by scripts.") decides it: no `change` (D-3, K-first,
    PM 866). elidex conforms. Cells DC-2 to DC-7, DC-9, DC-11 and DC-13 (M1, M6: `change`) pin K-first's outcome
    as today; the fix of DF-1 (a) keeps them green."
  - *Defect DF-2* — **[spec]**: "The change-on-blur snapshot's population follows `is_text_control` at the seed and
    at each `type` change; step 2.1 evaluates its conjuncts at focus loss. (i) A control whose `type` changes from a
    non-text kind to a text kind while focused has no snapshot, so a user edit there fires `input` and no `change`.
    (ii) A user-edited text control whose `type` changes to Number, Range or a date / time kind while focused loses
    its mark, so no `change` fires. Cells DC-14 to DC-16."
  - *Where the fix is planned.* A code PR, planned as a **per-PR slice of the focus-fixup umbrella**
    (`docs/plans/2026-11-focus-fixup-program.md`), with `/elidex-plan-review` before implementation: mandatory, not
    a judgement. The area is edge-dense and the umbrella's §1.1 says so; the PR also changes a ratified behaviour.
  - *Known requirements of the fix* (recorded so the slice plan starts from them; not a design):
    1. the component must carry both the first-focus value and the user-change mark, so the
       `FocusValueSnapshot(Option<String>)` shape of PM 393 is re-decided there. Under D-3 (K-first) the mark is
       the value just before the first user value change, as today, so the facts are two and D-3 adds no third (§4.1);
    2. the seed instant must move before the `focus` / `focusin` listeners (today
       `crates/dom/elidex-dom-api/src/focus/update_steps.rs:304-311` runs after them), under reading R-1;
    3. the VM `focus()` seed site (`crates/script/elidex-js/src/vm/host/html_element_proto.rs:311`, P9) is a second
       seed and must follow the same rule;
    4. a redundant focus of the focused control must not re-seed (E-17);
    5. the population must be decided at focus loss, or kept correct across `type` changes (DF-2).
  - *Allowlist.* The class-E row (slot `-`) for `crates/dom/elidex-form/src/focus_snapshot.rs:71` is the code-side
    statement of H's ground and of DF-1. The allowlist header allows deletions only, so its reclassification is that
    code PR's: rewriting the comment makes the row stale and the PR prunes it.
  - *Why not here:* PM 842, docs and code are separate PRs.
- **LR-3 → `#11-implicit-submission-commits-change`: close as not a core defect.** Record: "The current order conforms:
  HTML §4.10.22.2 names no `change` and no focus change for implicit submission, and §4.10.5.5 places a typed text
  control's `change` at the loss of focus; elidex's text control has no explicit commit action (One issue, one way:
  one commit point). The slot's `[evidence]` (two engines fire `change` before `submit`) is a record of engine
  behaviour, not a content need, and opens nothing. A different order would be a `[compat]` shim; none is
  registered. A new compat slot is opened when a content need is recorded. What counts as one: a named real page
  (from the compatibility survey `docs/design/ja/29-survey-analysis.md` or an exit-gate site set) whose function
  fails under the core order, with the failing pair identified (state written in a `change` listener, read in a
  `submit` listener, on a submission by Enter). An engine's order alone, or a test that asserts only the order,
  does not count (`docs/plans/2026-11-change-on-blur-spec-rederivation.md` §5)."
- **LR-4 → the PM**: observations O-1 to O-4 (§6.4), to home or dismiss. No ID and no home is proposed. Each states
  whether it is a defect against the spec; O-1 and O-2 (i) have no owner today.
- **LR-5 → `#11-focus-change-user-validity`, `#11-vm-host-synthetic-dom-event-dispatch`,
  `#11-number-input-user-edit-path` (ledger `:1062`), `#11-select-keyboard-change-event` (ledger `:1063`)**: no
  change; this memo cites them as the owners of E-16, E-8, O-4 and E-14.
- **LR-6 → the PM: withdraw PM 305, PM 307 and PM 393** (the PM issues the withdrawal records; PM 847). The ground
  each loses (§6.3):
  - PM 305 (the choice of H): its ground is the #597 measurement, which the ground order does not admit for a core
    requirement; and H's class-(a) outcome contradicts step 2.1 under every reading.
  - PM 307 (W6 as an accepted divergence): its ground "the spec defines no baseline" is false (step 2.1 names the
    value at first focus), and its reopen condition is an engine's behaviour.
  - PM 393 (the `Option<String>` representation): its ground "the focus-time `initial` was dead state" is false once
    class (a) is decided by the spec; the first-focus value is read at focus loss.
- **LR-7 → the PM: decide D-3** (§4.1) — **answered by PM 866** (2026-11-20, the user's choice): K-first, on rung 1
  (step 2.1 alone is undetermined; §4.10.5.5 decides class (b)). Recorded in §0.4 D-3, §4.1, §6.1 and LR-2: the
  expected value of DC-2 to DC-7, DC-9 and DC-11 is no `change` (`false` for DC-11), that of DC-13 is `change`, and
  DF-1 (b) is not a defect. Nothing remains open under this record.

## §8. Edits to the landed memos (this PR)

Both memos are landed records (PM 511). Each edit keeps the landed words, marks the measurement `[history]`, and adds
a dated correction. No decision of either memo is rewritten in place. From rev 2 each erratum keeps one line of
factual correction and points to this memo's section for the substance (§6.3); the substance is stated here only.

| Memo | Site | Edit |
|---|---|---|
| umbrella | DC2 (`:44`) | the textarea ground: principle (E-13), measurement `[history]` |
| umbrella | §1.1 (b) | "the mixed case is measured" → the sentence fixes the reference point; the mixed class is open (§4.1) |
| umbrella | §1.2 row (1) (`:81`) | "design H" ground: re-derived here; the baseline differs (DF-1) |
| umbrella | §3 rows for step 2.1 and textarea (`:159`, `:163`) | pointers to E-5 / DF-1 and E-13 |
| umbrella | §7; changelog line | erratum pointer; the cell list replaced by a pointer to §6.3 |
| plan (1) | opening table rows M1, M4, M9 | `[history]` tags; M4 → DF-1 (a) [spec]; M9 → class (b), undetermined (§4.1) |
| plan (1) | E1 | textarea ground (§4.2) |
| plan (1) | D-1 | erratum: one line per correction, pointers to §4.1, §6.1, §6.3; PM 305 / PM 393 "the PM withdraws at landing (LR-6)" |
| plan (1) | §3 textarea row; §4.3 opening and table header; §4.4 | grounds corrected (PE-4, PE-5, §4.2); §4.3's erratum no longer says the spec decides the mixed case |
| plan (1) | §5.2 closing paragraph; R3; §6 pre-step heading, N6 and the W6 record; §8 first bullet; §10 Q-1; changelog Rev 3 | `[history]` tags; dated pointers; W6: the cited ground does not hold, the PM withdraws PM 307 at landing (LR-6) |
| plan (1) | changelog | erratum line |

The umbrella's line numbers `:44`, `:81` and `:163` do not move: every edit above them is in place.

## §9. Definition of done

1. `python3 .claude/skills/elidex-plan-review/preflight.py <memo>` exits 0 for this memo, the umbrella and plan (1).
2. `python3 .claude/tools/ecs_mission_ratchet.py` → ok, with no class-T row left for
   `#11-change-on-blur-model-spec-rederivation`
   (`/usr/bin/grep -c 'change-on-blur-model-spec-rederivation' .claude/tools/ecs-mission-allowlist.tsv` → 0).
3. `git diff --stat origin/main` lists only `docs/plans/` files and the allowlist (rows deleted only: 15 = 14 class T
   for the slot + 1 class E, LR-1).
4. D-3 is decided by the PM (PM 866: K-first) and recorded in §0.4, §4.1, §6.1 and LR-2 (LR-7). Satisfied when
   PM 866 is recorded in PORTFOLIO.
5. The PM applies LR-1 to LR-6 at landing.

## Changelog

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
