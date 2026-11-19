# Change-on-blur — model H re-derived from HTML (spec → ECS principles → evidence)

Slot `#11-change-on-blur-model-spec-rederivation` (owner Layout; PM 432, PM 842), with
`#11-implicit-submission-commits-change` folded in (PM 842). Rev 1, 2026-11-20. Branch
`layout-change-on-blur-rederivation`. **Docs only**: this memo changes no code and plans no code change (PM 776).

**Defect this memo removes.** The landed change-on-blur model **H** (#597, `e0e18713`) names a measurement of one
engine as its ground (umbrella `docs/plans/2026-11-focus-fixup-program.md` DC2, row (1), §3; plan (1)
`docs/plans/2026-11-change-on-blur-user-provenance.md` D-1, §4.3). Under CLAUDE.md's ground order a core requirement
comes from the spec, then from elidex's own principles where the spec is silent; a measurement is at most `[evidence]`.

**Result.**

- Of the 18 rows of §4, 16 hold: 14 on spec text, 1 on spec text plus an elidex principle (E-15), 1 on an elidex
  principle alone (E-13, `<textarea>`, where the spec is silent). No measurement is needed as a ground for any of them.
  1 row (E-16, user validity) is not an element of H.
- 1 element differs from the spec: **the baseline** (E-5). HTML compares with the value "when the control was first
  focused"; H compares with the value just before the first user value change. They differ only when a non-user write
  lands between focus and the first user value change. Defect **DF-1** (§6). The fix is a separate code PR (PM 842).
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
| P10 | Checkbox / radio fire `input` then `change` from their own state-change path; a select arrow key fires `input` only (CP-4, open) | `form_input.rs:80-86`; `event_handlers.rs:619-623` |
| P11 | No sink call sits on a navigation or unload path | `git grep -n 'blur_current(\|focus_fixup(\|set_focus(' -- crates/shell/elidex-shell/src ':!*tests*' ':!*test_support*'` → 14 hits = 3 definitions + 11 call sites: click, Tab, label click, iframe click, the rendering fixup (run 2026-11-20) |
| P12 | No shell user edit path exists for Number, Range or the date / time kinds | `git grep -n 'FormControlKind::\(Range\|Number\|Date\|Color\|Time\|Month\|Week\|DatetimeLocal\)' -- crates/shell/elidex-shell/src ':!*tests*'` → 0 hits; control: the same command with `FormControlKind::Checkbox` → 2 hits (run 2026-11-20) |
| P13 | The cells that pin H's baseline: C6 (`change` expected), C20–C26 (no `change` expected), unit C7 (l) | `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs:508-527`, `:562-679`; `focus_snapshot.rs:211` |

### §0.3 Premise errors

- **PE-1 (brief)** — "implicit submission (§4.10.21.2)". The section is **§4.10.22.2 Implicit submission**
  (`.claude/tools/webref dfn html 'default button'` → §4.10.22.2).
- **PE-2 (brief)** — "the textarea / select change rules". §4.10.11 The textarea element has **no** `change` rule
  (`.claude/tools/webref body html the-textarea-element | /usr/bin/grep -c 'named change'` → 0; control: the same
  command with `'named input'` → 2). The select rule is *send select update notifications* (§4.10.7).
- **PE-3 (slot row of `#11-implicit-submission-commits-change`)** — "HTML §4.10.5.5 fires a text control's `change`
  when it loses focus". §4.10.5.5 only announces it ("will be fired when the control loses focus"). The step that fires
  it is §6.6.4 *focus update steps* step 2.1.
- **PE-4 (plan (1) §4.3)** — "It does not say what a script write does to either [condition]". The sentence fixes the
  reference point: "what it was when the control was first focused". Nothing in it lets a later write move that point.
  The spec is therefore not silent on the baseline, so ground-order step 3 (a measurement) was never reached. See E-5.
- **PE-5 (plan (1) §4.4)** — "Replacing a selection with identical text was not measured either; I2 follows elidex's
  own value comparison". §4.10.5.5 names that case as a non-change (Q4). The spec decides it; no probe is needed.

### §0.4 Decisions

Each line names the ground-order step that decided it.

- **D-1 The mark, the funnel, the provenance rule, the commit point, the event order and the kind boundary hold on spec
  text** — [spec]: §4.10.5.5, §6.6.4 step 2.1, §2.1.4, UI Events §3.3.2 (table §4, E-1 to E-4, E-6 to E-12, E-14, E-17,
  E-18).
- **D-2 The baseline is the value when the control was first focused** — [spec]: §6.6.4 step 2.1's parenthetical (Q1).
  H's baseline (the value before the first user value change) has no ground in spec text or in an elidex principle.
  Its only source is the #597 measurement [history]. Recorded as defect DF-1; this memo does not change the behaviour.
- **D-3 Where step 2.1's parenthetical admits two readings (§4.1), elidex reads it as a conjunction** — [spec] for the
  letter; [principle] for the choice between the readings: the predicate reads exactly the two facts the sentence
  names, each with one writer (ECS-native first: state is component facts, not a translation of another engine's
  internal field; Ideal over pragmatic: spec-faithful by default).
- **D-4 `<textarea>` fires change-on-blur under the same predicate as `<input>`** — [spec] silent (PE-2);
  [principle] decides (§4.2): behaviour follows the component a control carries, not its tag, and the text-control
  class has one commit mechanism (ECS-native first; One issue, one way).
- **D-5 Implicit submission is not a commit point in core** — [spec] §4.10.5.5 and §4.10.22.2 for the event set;
  [principle] for the user-interface classification the spec leaves to the user agent (§5). A different order is a
  `[compat]` concern.
- **D-6 The #597 measurements stay in plan (1) as `[history]`** and are a ground for nothing. The "accepted
  divergence" record for W6 (PM 307) is withdrawn as a record of a ground: see DF-1 (b), cell DC-9.

## §1. Scope

In scope: the grounds of every element of H (§4); the implicit-submission decision (§5); the differences as defects
with the cells a later code PR adds (§6); the ledger records (§7); ground corrections in the two landed memos (§8).

Out of scope: any code change; the plan of DF-1's fix (PM 776, PM 842); VM `focus()` / `blur()` events (A2c); user
validity at step 2.1.1 (`#11-focus-change-user-validity`); CP-1 to CP-4.

**Shape.** One docs PR. Not edge-dense in itself: it adds no mechanism and couples no new invariant. It is plan-reviewed
because it changes a ratified ground (D-2) and records a behaviour difference.

## §2. Coupled invariants

None new. The memo touches two existing ones and leaves both intact.

- **Plan (1) I3 (provenance lifetime = snapshot lifetime) × D-5.** A commit at Enter would need a second consumer of
  the snapshot and a re-seed outside a focus transition. D-5 keeps I3 as landed.
- **Umbrella X1 (one judgement, every sink) × DF-1.** DF-1 is a defect in what the one judgement compares with, not in
  where it is computed. X1, plan (1) I1, I2, I4, I5, I6 and I7 are untouched by DF-1.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §6.6.4 Processing model | *focus update steps* step 2.1 | all four conjuncts; the parenthetical's reference point (first focused) | re-derived, E-4 to E-8, E-18; DF-1 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focus update steps* steps 2.1 → 2.4, 4.1 | `change` before `blur`; the old entry stays the focused area during `change` | re-derived, E-10 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focusing steps* step 5; *unfocusing steps* step 8 | already focused → return; every focus loss reaches the focus update steps | re-derived, E-8, E-17 | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | the three user-interface classes; "without an explicit commit action" | text control = no commit action; `change` at focus loss | re-derived, E-1, E-3, E-4, E-11, E-15 | ✓ | yes |
| WHATWG HTML §4.10.5.5 Common event behaviors | "not fired in response to changes made … by scripts"; state restoration; "on behalf of the user" | non-user writers fire nothing and do not mark | re-derived, E-2, E-6 | ✓ | yes |
| WHATWG HTML §4.10.22.2 Implicit submission | default button → `click`; no submit button → the blocking-field count, then submit | names no `change`, no focus change | re-derived, E-15, §5 | ✓ | yes |
| WHATWG HTML §4.10.11 The textarea element | user changes the raw value → `input`; no `change` rule | spec silent on `change`; decided by principle | re-derived, E-13 | ✓ | yes |
| WHATWG HTML §4.10.7 The select element | *send select update notifications* | `input` then `change`, own commit | boundary only, E-14; CP-4 | ✗ | yes |
| WHATWG HTML §4.10.5.1.15 Checkbox state (type=checkbox) | input activation behavior | `input` then `change`, own commit | boundary only, E-14 | ✗ | yes |
| WHATWG HTML §2.1.4 DOM trees | removing steps, step 2 and its note | removal of the focused area fires no `blur`, no `change` | re-derived, E-9 | ✓ | no |
| WHATWG HTML §4.10.18.1 A form control's value | user validity; dirty value flag | not H (E-16); the dirty flag has its own predicate (plan (1) D-5) | pointer only | ✗ | no |
| UIEVENTS §3.3.2 Focus Event Order | rows 3–4 | `focusout` follows `blur` | re-derived, E-10 | ✓ | yes |

Numbers and titles: `.claude/tools/webref coverage-map html 6.6.4 html 4.10.5.5 html 4.10.22.2 html 4.10.11 html 4.10.7
html 4.10.5.1.15 html 2.1.4 html 4.10.18.1 uievents 3.3.2` (run 2026-11-20; anchors `#focus-processing-model`,
`#common-input-element-events`, `#implicit-submission`, `#the-textarea-element`, `#the-select-element`,
`#checkbox-state-(type=checkbox)`, `#dom-trees`, `#a-form-control's-value`, `#events-focusevent-event-order`).
Breadth: K = 2 (html, uievents), M = 12 → single PR scope. Also looked up: `webref dfn html 'unload a document'` →
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
| E-5 | **The baseline**: the value just before the first user value change since focus (P1, P3). A non-user write made before that change moves the baseline | Q1: "different to what it was when the control was first focused". The reference is the value at first focus. No sentence moves it (PE-4) | none: no elidex principle asks for a third fact (§4.1) | the #597 runs M4, M9 and W3–W8, measured on one engine [history]; not reached, since step 1 decides | **differs** → DF-1 |
| E-6 | No mark → no `change`: a script-only change fires nothing (M0, M5) | Q1 "the user has changed"; Q5 | — | — | holds [spec] |
| E-7 | A user edit that is undone before blur fires no `change` (M7) | Q1's parenthetical: the value is not different from what it was at first focus | — | — | holds [spec] |
| E-8 | Every focus-loss path runs the same substep: a user focus move, the unfocusing steps, the rendering-time fixup (P6). VM `focus()` / `blur()` do not yet (P9) | Q10: the focusing steps and the unfocusing steps both end in the focus update steps; `focus()` and `blur()` run them too (`webref body html dom-focus`: "Run the focusing steps for this"; "run the unfocusing steps given this") | — | — | holds [spec] for the sink paths. The VM gap is known and owned: `#11-vm-host-synthetic-dom-event-dispatch` (A2c), record R-1 of plan (1). No new defect |
| E-9 | Removing the focused control fires no `change`; a page unload fires none (P11) | Q9; §7.5.9 Unloading documents has no focus or `change` step (§3) | — | — | holds [spec] |
| E-10 | Order at focus loss: `change`, `blur`, `focusout`. `change` is fired synchronously inside the step, while the old control is still the focused area (P5, P6) | Q1 is step 2.1, `blur` is step 2.4, and the focused area is designated at step 4.1; Q12 | — | — | holds [spec] (the state seen *during `blur`* is not H: observation O-2) |
| E-11 | `input` precedes the `change` of the same edit; the mark is set before `input` is built (plan (1) I6) | Q2 "In all cases, the input event comes before the corresponding change event" | the mark-before-`input` order is elidex's device, with no observable of its own | — | holds [spec] |
| E-12 | `change` attributes: bubbles, not cancelable, not composed | Q1 "with the bubbles attribute initialized to true"; nothing else is initialized | — | — | holds [spec] |
| E-13 | `<textarea>` fires change-on-blur under the same predicate as `<input>` | silent: Q1 says "input element"; §4.10.11 has no `change` rule (PE-2); no text forbids it; Q13 (non-normative) describes `change` as fired at form controls | §4.2 | plan (1) §6: `<textarea>` measured identical to `<input>` on one engine [history]; not reached, since step 2 decides | holds [principle] |
| E-14 | Kind boundary: only text controls carry the snapshot (P4). Checkbox, radio and select commit through their own paths (P10) | Q1 excludes an element with "a defined activation behavior"; §4.10.5.1.15 / §4.10.5.1.16 fire `input` then `change` from the input activation behavior; Q11 | — | — | holds [spec]. Open and already owned: CP-4 (select arrow key). See O-4 for the kinds with no user edit path |
| E-15 | Implicit submission does not commit: Enter dispatches `submit`; `change` fires at a later focus loss (P8) | Q7 names the events of implicit submission: a `click` at the default button, or the form's submission. No `change`, no focus change. Q3: no explicit commit action for typed text | §5: the classification Q6 leaves to the user agent | slot row: two engines fire `change` before `submit` [evidence]; not re-measured here; not reached | holds [spec] + [principle] |
| E-16 | User validity is not set at step 2.1.1 | Q1 substep 1 | — | — | not an element of H; open and owned: `#11-focus-change-user-validity` |
| E-17 | A redundant focus of the focused control does not re-seed | Q10 step 5; Q1 "first focused" | — | — | holds [spec] |
| E-18 | A `type` flip to a non-text kind while focused clears the snapshot, so no `change` fires at blur (P4) | Q1's conjuncts are evaluated at blur: "the change event applies to the element, and the element does not have a defined activation behavior" | — | — | holds [spec] |

### §4.1 E-5 in detail — what the sentence fixes, and what it leaves

Q1 has two parts: (U) "the user has changed the element's value … while the control was focused without committing
that change", and (F) "(such that it is different to what it was when the control was first focused)". Write `F0` for
the value when the control was first focused and `V` for the value at focus loss.

- **What every reading fixes.** `V ≠ F0` is necessary. A `change` at a control whose value equals its value at first
  focus contradicts (F) however "such that" is read.
- **What two readings split.** Reading **L** (conjunction): fire iff a user edit changed the value since focus and
  `V ≠ F0`. Reading **K** (causal): fire only if the user's edits are what made `V` differ from `F0`.
- **L and K agree on every sequence without a non-user write**, and on M0, M1, M2, M3, M5, M6, M7 and M8 of plan (1)
  §4.3. They split on one class: a non-user write moves the value away from `F0`, and the user's edits net to nothing
  (M9; cells C20–C26).

| Class (control focused, `F0` as stated) | L | K | H as landed |
|---|---|---|---|
| (a) `F0 = ""`; non-user write `"x"`; user Backspace → `V = ""` (M4, cell C6) | none | none | **`change`** |
| (b) `F0 = "a"`; non-user write `"x"`; user types `y`, Backspace → `V = "x"` (M9, cells C20–C26) | `change` | none | none |
| every other case of plan (1) §4.3 | same as H | same as H | — |

**Decision (D-3): L.** Grounds, in order:

1. [spec] L is the sentence as written: two conditions, both stated. K adds a condition ("caused by the user") that the
   sentence does not state and that has no definition for mixed sequences (M1: the user typed, then a script replaced
   the value; whose difference is it?). Q5 does not select K: under L no `change` fires without a user change of the
   value (E-6), so no event is fired "in response to" a script's change.
2. [principle] ECS-native first. Under L the predicate reads two facts the spec names, each one field with one writer:
   `F0` (written by the seed at focus) and the user-change mark (written by `apply_user_edit`). K needs a third stored
   fact, the value before the first user change, which the spec does not name. In H that fact came from one engine's
   internal baseline field [history] — a translation of another engine's state, which the mission rule excludes as a
   design ground.
3. [principle] L keeps what plan (1) D-4 wanted: no non-user writer calls anything, so an unlisted or future writer
   cannot be missed. That property does not depend on H's baseline.

Class (a) is a difference under every reading. Class (b) is a difference under L, and L is elidex's reading by 1–3.

### §4.2 E-13 in detail — `<textarea>`

The spec gives no rule that fires `change` at a `<textarea>` and none that forbids it (PE-2). Step 2 decides:

- **ECS-native first.** A control's change-on-blur behaviour follows from the components it carries
  (`FormControlState` of a text kind, `FocusValueSnapshot`), read by one system-like function
  (`take_change_on_blur`). Excluding `<textarea>` would be a branch on the element's tag inside that function: behaviour
  owned by an element class, which is the OO shape CLAUDE.md asks not to translate.
- **One issue, one way.** `<textarea>` already shares the funnel and the `input` rule with `<input>` (Q3 and Q8 use the
  same words). A text-control class with one edit funnel and two commit behaviours would be a second way.
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
   - ECS-native first: `FocusValueSnapshot` lives for one focus session and has three writers, all on the focus
     transition (seed, mark, take; plan (1) I3). A commit at Enter consumes it while the control stays focused, so it
     needs a re-seed outside a focus transition: a second lifetime rule for the same component.
5. **[evidence]** The slot row records that two engines fire `change` before `submit` [evidence]. It was not
   re-measured for this memo and is not a ground: steps 1–4 decide.

**Compat boundary (description only; nothing is planned).** Content that keeps state in a `change` listener and reads
it in a `submit` listener sees the state from before the last edit when the user submits with Enter. If such content
has to work, the remedy is a `[compat]` shim outside core (CLAUDE.md core / compat / deprecated): on implicit
submission it would run the same judgement (`take_change_on_blur`) and dispatch `change` before the submission starts.
Core keeps one commit point. No slot is proposed for the shim: no content need is recorded.

Out of this decision: whether elidex's implicit submission itself follows Q7 (observation O-1).

## §6. Defects — where H differs from the spec

### DF-1 The change-on-blur baseline is not the value at first focus

*Defect.* `take_change_on_blur` (`crates/dom/elidex-form/src/focus_snapshot.rs:87-95`) compares the value at focus
loss with the value just before the first user value change (`:75-79`). HTML compares it with the value "when the
control was first focused" (Q1). The two differ when a non-user write (plan (1) W1–W8, or a `focus` listener's write)
lands between focus and the first user value change:

- **(a)** the user's edits bring the value back to the first-focus value → elidex fires `change`; the spec fires none.
  Wrong under every reading of Q1 (§4.1).
- **(b)** the user's edits net to nothing and the value differs from the first-focus value → elidex fires no `change`;
  the spec fires one (reading L, D-3).

Reference instant: "first focused" is the designation of the control as the focused area (focus update steps step
4.1), which precedes the `focus` event (step 4.4). Today the seed runs after the `focus` / `focusin` listeners
(`crates/dom/elidex-dom-api/src/focus/update_steps.rs:304-311`) and stores no value (P1).

*Not affected.* The funnel, the mark, `input`, the one-judgement rule, the kind boundary, the event order
(E-1 to E-4, E-6 to E-15, E-17, E-18).

*Cells the code PR adds or re-states* (spec-derived; the cited ground is the expected value's source). "UA focus" and
"UA blur" are the shell's user focus moves, as in plan (1) §6.

| Cell | Sequence | Expected | Ground | Today |
|---|---|---|---|---|
| DC-1 (re-states C6), both kinds | `<input id=b value="">`; UA focus `b`; script `b.value = "x"`; user Backspace (value `""`); UA blur | no `change` | Q1 (F): value equals the first-focus value | `change` |
| DC-2 (re-states C20), both kinds | UA focus `b`; user types `a`; UA blur (`change`); UA focus `b`; script `b.value = "x"`; user types `y`, Backspace (value `"x"`); UA blur | `change` on the second blur too | Q1, reading L: user changed the value; `"x"` ≠ `"a"` | none |
| DC-3 (re-states C21) | `<textarea id=b>p</textarea>`; UA focus; script `b.defaultValue = "q"`; user `z`, Backspace; UA blur | `change` | as DC-2 (`"q"` ≠ `"p"`) | none |
| DC-4 (re-states C22) | `<input id=b value="p">`; UA focus; script `b.setAttribute("value", "q")`; user `z`, Backspace; UA blur | `change` | as DC-2 | none |
| DC-5 (re-states C23) | `<input id=b value=" a">`; UA focus; script `b.type = "url"` (value `"a"`); user `z`, Backspace; UA blur | `change` | as DC-2 (`"a"` ≠ `" a"`) | none |
| DC-6 (re-states C25) | `<input id=b value="p">`; UA focus; script `b.setRangeText("q", 0, 0)` (value `"qp"`); user `z`, Backspace; UA blur | `change` | as DC-2 | none |
| DC-7 (re-states C26) | `<form id=f><input id=b value="d"></form>`; script `b.value = "x"` before focus; UA focus; script `f.reset()` (value `"d"`); user `z`, Backspace; UA blur | `change` | as DC-2 (`"d"` ≠ `"x"`) | none |
| DC-8 (new), both kinds | UA focus `b` (value `""`); script `b.value = "x"`; user types `a` (value `"xa"`); user Backspace ×2 (value `""`); UA blur | no `change` | Q1 (F) | `change` |
| DC-9 (re-states C24) | `<input id=b type=email value="a@b, c@d">`; UA focus; script `b.setAttribute("multiple", "")` (value `"a@b,c@d"`); user `z`, Backspace; UA blur | `change` | as DC-2. This replaces the "accepted divergence" of PM 307: the expected value now has a spec ground (D-6) | none |
| DC-10 (new) | `<input id=b value="">` with a `focus` listener that sets `b.value = "x"`; UA focus `b`; user Backspace (value `""`); UA blur | no `change` | Q1 (F) with the reference instant at step 4.1 | `change` |
| DC-11 (unit; re-states C7 (l)) | seed at `""`; a non-user write to `"x"`; `apply_user_edit` appends `y`; a second `apply_user_edit` removes it (value `"x"`) | the judgement returns `true` (`"x"` ≠ `""`) | Q1, reading L | `false` |
| DC-12 (control, unchanged) | C1, C3, C4, C5, C9, C14, C16, C17, C18, C19 of plan (1) | as today | E-6, E-7; L and H agree (§4.1) | green |

The code PR also rewrites the two code comments that name the measurement as the ground
(`focus_snapshot.rs:4-5`, `:69-71`; the second is the ECS-mission allowlist's class-E row for that file).

This memo does not plan the fix (PM 776, PM 842).

### Observations outside H (no verdict; for the PM to home)

Found while reading the code for §0.2. None is an element of H; none is fixed or planned here. Ledger check: a
`/usr/bin/grep -n -i` of the slot ledger for each key phrase, run 2026-11-20; "no row found" means that grep only.

- **O-1 Implicit submission skips the default button and the blocking-field count.** `handle_form_submit`
  (`crates/shell/elidex-shell/src/content/form_input.rs:129-143`) dispatches `submit` at the form ancestor of the
  focused control. Q7 requires a `click` at the default button, or, with no submit button, a return when more than one
  field blocks implicit submission. Ledger: no row found for `default button`; `#11-form-submission-dispatcher` exists
  and was not read for coverage.
- **O-2 The focused area during `blur`.** `focus_update_steps` clears the `FOCUS` bit before `blur`
  (`crates/dom/elidex-dom-api/src/focus/update_steps.rs:281-287`); its comment gives matching existing engines as the
  ground and says the spec keeps the old entry designated until step 4.1. A spec departure with a parity ground;
  probably within `#11-core-browser-parity-deviations` (not verified).
- **O-3 `input` is dispatched synchronously in the key handler.** Q3, Q8 and Q11 say "queue an element task on the
  user interaction task source". Whether the shell's key handling is itself that task was not examined. No row found.
- **O-4 The snapshot's population is `is_text_control`; step 2.1's is "input element, change applies, no defined
  activation behavior"**, which also covers Number, Range and the date / time kinds. Equal in effect today because
  those kinds have no user edit path (P12; CP-3 for Number). A future edit path for one of them must seed and mark the
  same component.

## §7. Ledger records (applied by the PM at landing)

- **LR-1 → `#11-change-on-blur-model-spec-rederivation`: close.** Record: "Re-derived in
  `docs/plans/2026-11-change-on-blur-spec-rederivation.md`. 16 elements of H hold on spec text or an elidex
  principle; the measurement is `[history]` and a ground for nothing. One element differs (the baseline) → LR-2. The
  slot's 14 class-T allowlist rows (13 in plan (1), 1 in the umbrella) are discharged by this PR (tagged; pruned)."
- **LR-2 → new slot, ID proposed `#11-change-on-blur-baseline-first-focus-value`** (owner Layout; the PM draws the ID).
  *Defect:* "`take_change_on_blur` compares the value at focus loss with the value just before the first user value
  change; HTML §6.6.4 focus update steps step 2.1 compares it with the value when the control was first focused. After
  a non-user write between focus and the first user value change, elidex (a) fires `change` when the user's edits
  return the value to its first-focus value, and (b) fires none when the user's edits net to nothing and the value
  differs from its first-focus value. Cells DC-1 to DC-12 of
  `docs/plans/2026-11-change-on-blur-spec-rederivation.md` §6." *Trigger:* a code PR, plan-reviewed (it changes a
  ratified behaviour). *Why not here:* PM 842, docs and code are separate PRs.
- **LR-3 → `#11-implicit-submission-commits-change`: close as not a core defect.** Record: "The current order conforms:
  HTML §4.10.22.2 names no `change` and no focus change for implicit submission, and §4.10.5.5 places a typed text
  control's `change` at the loss of focus; elidex's text control has no explicit commit action (One issue, one way; the
  snapshot's lifetime is the focus session). A different order would be a `[compat]` shim; none is registered, because
  no content need is recorded (`docs/plans/2026-11-change-on-blur-spec-rederivation.md` §5)."
- **LR-4 → the PM**: observations O-1 to O-4 (§6), to home or dismiss. No ID is proposed.
- **LR-5 → `#11-focus-change-user-validity`, `#11-vm-host-synthetic-dom-event-dispatch`, CP-3, CP-4**: no change; this
  memo cites them as the owners of E-16, E-8, O-4 and E-14.

## §8. Edits to the landed memos (this PR)

Both memos are landed records (PM 511). Each edit keeps the landed words, marks the measurement `[history]`, and adds
a dated correction that states the re-derived ground. No decision of either memo is rewritten in place.

| Memo | Site | Edit |
|---|---|---|
| umbrella | DC2 (`:44`) | the textarea ground: principle (E-13), measurement `[history]` |
| umbrella | §1.1 (b) | "the mixed case is measured" → the spec fixes the reference point (PE-4) |
| umbrella | §1.2 row (1) (`:81`) | "design H" ground: re-derived here; the baseline differs (DF-1) |
| umbrella | §3 rows for step 2.1 and textarea (`:159`, `:163`) | pointers to E-5 / DF-1 and E-13 |
| umbrella | new section before the changelog; changelog line | erratum pointer |
| plan (1) | opening table rows M1, M4, M9 | `[history]` tags; M4 / M9 outcomes point to DF-1 |
| plan (1) | E1 | textarea ground (E-13) |
| plan (1) | D-1 | ground statement corrected; erratum paragraph |
| plan (1) | §3 textarea row; §4.3 opening and table header; §4.4 | grounds corrected (PE-4, PE-5, E-13) |
| plan (1) | §5.2 closing paragraph; R3; §6 pre-step heading and the W6 record; §8 first bullet; §10 Q-1; changelog Rev 3 | `[history]` tags; W6 record withdrawn as a ground (D-6) |
| plan (1) | changelog | erratum line |

The umbrella's line numbers `:44`, `:81` and `:163` do not move: every edit above them is in place.

## §9. Definition of done

1. `python3 .claude/skills/elidex-plan-review/preflight.py <memo>` exits 0 for this memo, the umbrella and plan (1).
2. `python3 .claude/tools/ecs_mission_ratchet.py` → ok, with no class-T row left for
   `#11-change-on-blur-model-spec-rederivation`
   (`/usr/bin/grep -c 'change-on-blur-model-spec-rederivation' .claude/tools/ecs-mission-allowlist.tsv` → 0).
3. `git diff --stat origin/main` lists only `docs/plans/` files and the allowlist (rows deleted only).
4. The PM applies LR-1 to LR-5 at landing.

## Changelog

- **Rev 1 (2026-11-20)** — initial re-derivation (PM 432, PM 842).
