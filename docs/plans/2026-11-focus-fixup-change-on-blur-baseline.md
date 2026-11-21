# `focus-fixup-change-on-blur-baseline` — the change-on-blur judgement compares with the value at first focus, over a population chosen by element (per-PR plan-memo)

This PR is a per-PR slice of the umbrella `docs/plans/2026-11-focus-fixup-program.md` (its §7 erratum names this fix as
"a separate code PR, planned as a slice of this umbrella"), a terminal slice by CLAUDE.md *Edge-dense work* base case.
Slot **`#11-change-on-blur-baseline-first-focus-value`** (owner Layout; registered PM 861, amended at #656 per the
re-derivation's LR-2, PM 909). The slot row is the requirement; this memo plans it and changes none of its decisions.
The inputs are the re-derivation `docs/plans/2026-11-change-on-blur-spec-rederivation.md` ("the re-derivation"): its
§0.4 D-3 / D-4, §0.5 R-1 / R-2 / R-3, §4.1, §6 (DF-1, DF-2, cells DC-1 to DC-19), §6.4 and §7 LR-2. Rev 1, 2026-11-22.
Branch `layout-change-on-blur-baseline-plan`, worktree `wt-cob2`. **Docs only**: this PR adds this memo and a one-line
pointer in the umbrella's §7; the code PR it plans is separate.

File name: plan (2) of the same umbrella is `2026-11-focus-fixup-canonical-steps.md`; this memo takes the same
`focus-fixup-` prefix. Plan (1) (`2026-11-change-on-blur-user-provenance.md`) predates the prefix; no other convention
exists in the umbrella.

**Defect the code PR removes** (the slot row, stated once there; summarised here only to name the cells).

- **DF-1 (a) [spec]**: `take_change_on_blur` compares the value at focus loss with the value just before the first user
  edit; HTML §6.6.4 *focus update steps* step 2.1 compares with the value "when the control was first focused". Red
  today: DC-1, DC-8, DC-10. DF-1 (b) is not a defect (D-3); its cells stay green.
- **DF-2 [spec] under R-2**: the population is chosen by the kind at each write (the seed, the reconciler's `type` arm),
  not by step 2.1's conjuncts at focus loss. Red today: DC-14, DC-15, DC-18, DC-19; DC-16 is the control.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` below is at `origin/main` **`1f81fdc2`** (#659), this branch's base
(`git -C <wt> merge-base HEAD origin/main` → `1f81fdc2`). Symbols are the SSoT; a line is a reading aid at that sha.
The re-derivation read the code at `29edf6fc`. Drift over the files this memo cites:
`git diff --stat 29edf6fc 1f81fdc2 -- <the .rs files of §7>` → `tree_clone.rs` (73 lines, #659's NodeKind work; the
`FocusValueSnapshot` row of its clone-policy table is unchanged in text) and `content_change_on_blur_tests.rs` (2 lines,
#655's inline `` `[contrast]` `` on C24's doc comment, which the re-derivation already records). No premise symbol moved
(run 2026-11-22).

### §0.2 Premises re-verified at `1f81fdc2`

| ID | Premise | Where (file + symbol; line at `1f81fdc2`) |
|---|---|---|
| B-P1 | The state is one component on the control, `FocusValueSnapshot(Option<String>)`, private field; `None` at seed, `Some(before)` from the first user value change | `crates/dom/elidex-form/src/focus_snapshot.rs` `FocusValueSnapshot` (:32), `baseline()` (:38) |
| B-P2 | The seed writes `FocusValueSnapshot(None)` iff the control's kind `is_text_control()`, and **removes** the component otherwise | `focus_snapshot.rs` `record_focus_snapshot` (:53-63) |
| B-P3 | The mark is `get_or_insert(before)` on an existing component; it never inserts one. Its only caller is `apply_user_edit`, when the edit changed the value | `focus_snapshot.rs` `note_user_value_change` (:75-79); `crates/dom/elidex-form/src/user_edit.rs` `apply_user_edit` (:50-66) |
| B-P4 | The verdict removes the component and returns `Some(b) && value != b`; no kind test | `focus_snapshot.rs` `take_change_on_blur` (:87-95) |
| B-P5 | The reconciler's `type` arm runs type-change steps 1–3 (`apply_type_change_value_migration`), then the kind update and step-6 sanitization (`sanitize_for_type_change`), then calls `clear_focus_snapshot` iff the new kind is not a text control | `crates/dom/elidex-form/src/reconciler.rs` `handle_attribute_change`, `type` arm (:133-168; the clear :165-167) |
| B-P6 | In the canonical transition the seed runs after step 4.1's designation **and after** the `focus` / `focusin` dispatch, guarded by "`new` is still focused" | `crates/dom/elidex-dom-api/src/focus/update_steps.rs` `focus_update_steps` (designation :303, `fire_focus_event` :304, guard + `seed_focus_snapshot` :310-311) |
| B-P7 | A redundant focus returns before the transition (focusing steps step 5) | `update_steps.rs` `focusing_steps`, the `old == Some(target)` return (:149-151) |
| B-P8 | The shell sink's `seed_focus_snapshot` only calls `elidex_form::record_focus_snapshot`; its `commit_change_on_blur` only dispatches on `take_change_on_blur`'s verdict | `crates/shell/elidex-shell/src/content/focus.rs` `ShellFocusSink` (`seed_focus_snapshot` :142-148, `dispatch_change_on_blur` :192-199) |
| B-P9 | VM `focus()` returns early when the receiver is already the focused element, else (focusable, in the bound document) sets the bit and calls `elidex_form::record_focus_snapshot`; it fires no event and leaves the old holder's component in place (A2c) | `crates/script/elidex-js/src/vm/host/html_element_proto.rs` `native_html_element_focus` (early return :290-292, `set_focus_bit` :305, seed :311) |
| B-P10 | Exactly two production writers designate a focused element, and both seed: the transition's step 4.1 and VM `focus()` | `git grep -n 'set_focus_bit(' -- crates ':!*tests*' ':!*test_support*'` → production `Some(..)` writers: `update_steps.rs:303`, `html_element_proto.rs:305` (the other hits are the definition, two `None` clears and `update_steps.rs`'s `mod tests`); `git grep -n 'ElementState::FOCUS' -- crates ':!*tests*' ':!*test_support*'` with an insert → only `sot.rs:125`, inside `set_focus_bit` (run 2026-11-22) |
| B-P11 | `<button>`, `<select>`, `<output>`, `<meter>`, `<progress>` also carry `FormControlState`; `<button>` shares the kinds `SubmitButton` / `ResetButton` / `Button` with `<input>`, and `<input type=textarea>` carries the kind `TextArea`. So no kind test can express "is an `input` element" or "is a `textarea` element" | `crates/dom/elidex-form-core/src/lib.rs` `FormControlKind` (:54-98), `from_tag_and_type_attr` (:232-247), `from_type_str` (:253-279) |
| B-P12 | `is_text_control` has callers besides the two this PR deletes (key routing, IME, the shell's text-input gate), so it stays | `git grep -n 'is_text_control' -- crates` → `event_handlers.rs:333`, `ime.rs:19/:45`, `content/mod.rs:479`, form-core `:120/:164` |
| B-P13 | The dom-api unit test pins the seed **after** the `focus` event, on a ground R-1 contradicts ("a focus-handler value write must be in the baseline") | `update_steps.rs` `mod tests`, `focusing_steps_designates_last_and_fires_in_spec_order` (`seeds_len_at_focus == Some(0)`, :439-444) |
| B-P14 | The cells of the re-derivation's §6.1 / §6.2 exist as named (C6 = `c6_*`, C20–C26 = `c20_*` / `writer_rebaselines` users, C4 = `c4_*`, C17 = `c17_*`, C7 (l) = `c7l_first_mark_rebaselines_once`) | `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs` (:508-527, :562-679); `focus_snapshot.rs` `mod tests` (:211) |
| B-P15 | `sanitize_color`'s doc comment states *update a color well control color* step 2.1 backwards (it says the content attribute is read when the dirty flag is **set**) and grounds the live-value read on "the interoperable behavior every browser ships" [contrast]; that line is the allowlist row at line 43 of the tsv (class T, `#11-core-browser-parity-deviations`) | `crates/dom/elidex-form-core/src/sanitize.rs` `sanitize_color` doc (:163-171); the Color arm of `sanitize_value` (:285) reads `state.value()` |

### §0.3 Premise errors and findings

The brief's premises hold (each re-verified above). Findings it did not state:

- **F-1 (sanitize_color, beyond the comment).** Spec text (`.claude/tools/webref body html update-a-color-well-control-color`,
  re-derivation Q16): "If element's dirty value flag is false, then return the result of getting an attribute by
  namespace and local name given null, "value", and element. 2. Return element's value." So the comment is backwards,
  as the brief says, and its supporting argument falls with it: the value IDL setter sets the dirty flag before it
  sanitizes (QB-6), so under the correct reading `input.value = "#ff0000"` reads the live value and round-trips. But
  the code (`sanitize_value`'s Color arm, live value always) **still differs** from step 2 wherever the dirty flag is
  false and the live value differs from the content attribute. Reachable: `<input type=number value=red>` (Number
  sanitization: value `""`, dirty false); script `type = "color"` (value mode to value mode, step 6 runs the Color
  sanitization): the spec reads the attribute `"red"` → `"#ff0000"`; elidex reads `""` → opaque black `"#000000"`. So
  rewriting the comment cannot ground the live-value read on the spec; nor does an elidex principle ground it. OQ-2 asks
  the PM whether the behaviour is fixed in this PR. DC-19 is unaffected (the user's edit set the dirty flag, Q17, so
  both readings use the live value).
- **F-2 (the predicate's input).** `from_type_str` maps `"image"` to `TextInput` (the Image Button state is missing),
  `"select-one"` to `Select` and `"textarea"` to `TextArea` for an `<input>`. HTML: "The attribute's missing value default
  and invalid value default are both the Text state" (QB-3), and Image Button is a state to which "The input and change
  events do not apply" (QB-1). So for `<input type=image>` the step-2.1 predicate of B-3 receives the wrong kind and
  would answer "applies"; today the same element is already seeded and edited as a text control (it is a `TextInput`
  to every reader). Pre-existing, not caused or widened by this slice; no DC cell depends on it. Observation O-B1 (§9,
  LR-B3). Ledger check: `/usr/bin/grep -c -i 'type=image' <slot ledger>` → 0 (control:
  `core-browser-parity-deviations` → 4 hits), run 2026-11-22.
- **F-3 (a test the re-derivation's §6.1 did not list).** B-P13's assertion inverts under R-1; the code PR flips it
  (DC-10u).
- **F-4 (stale names).** The test names `writer_rebaselines`, `c21_…_rebaselines` to `c26_…_rebaselines` and
  `c7l_first_mark_rebaselines_once` name H's model ("the first mark re-baselines"); their outcomes hold under D-3 but the
  model they name is gone. §6 renames them (item (b) of the PM's carry-over; OQ-3).

### §0.4 Decisions (this plan)

Each names the rung that decides it. Decisions of the slot row (requirements 1–5, the Population, D-3, D-4, R-1, R-2)
are not re-decided; B-1 to B-9 are how the code PR meets them. Numbered B-n so they do not collide with the
re-derivation's D-n.

- **B-1 The component** — [slot req 1; spec Q1 (F); D-3]. `FocusValueSnapshot` becomes two private fields:
  `first_focus: String` (the value at first focus, `F0`, R-1 / R-3) and `user_change_from: Option<String>` (the mark:
  `Some(value just before the first user value change)`, D-3's K-first reference). No third fact (D-3 adds none, the slot
  row). The accessor `baseline()` is replaced by `first_focus()` and `user_change_from()` (NEW), read by
  `test_support::snap` only.
- **B-2 The population test is the element** — [slot Population; spec Q1 "If entry is an input element"; D-4].
  `record_focus_snapshot` writes iff the entity is an HTML `input` element or an HTML `textarea` element, read from the
  element (its tag and namespace, Q-S1), never from `FormControlState.kind` (B-P11 shows the kind cannot express it).
  Any other element gets no write — no insert and no remove. The same test is the only population gate: the take reads
  only a component this seed wrote, and the seed is the only inserter (§4.7).
- **B-3 One predicate over the kind at focus loss, private to the judgement** — [spec Q1's conjuncts "the change event
  applies to the element, and the element does not have a defined activation behavior", under R-2; D-4 for `TextArea`;
  One issue, one way; umbrella X1]. `change_on_blur_kind_applies(kind)` (NEW, private fn in `focus_snapshot.rs`), an
  exhaustive `match` with no wildcard arm, so a new `FormControlKind` must be decided there. Its one caller is
  `take_change_on_blur`. Truth table and derivation: §4.4. Not a `FormControlKind` method in `elidex-form-core`: a
  public classifier would invite a second caller deciding `change` elsewhere, and form-core's `lib.rs` is at 960 lines
  (§7).
- **B-4 The judgement** — [spec Q1 (F); D-3]. `take_change_on_blur` removes the component, then returns
  `change_on_blur_kind_applies(kind at focus loss) && user_change_from == Some(p) && V != first_focus && V != p`.
  `false` without a component or without `FormControlState`.
- **B-5 The seed instant** — [slot req 2; R-1]. In `focus_update_steps` the sink's `seed_focus_snapshot(new)` moves to
  immediately after step 4.1's `set_focus_bit(sink.dom(), Some(new))` and before `fire_focus_event(new, Focus, old)`.
  The "still focused" guard is deleted: nothing runs between the designation and the seed.
- **B-6 VM `focus()`** — [slot req 3, 4; Layering mandate]. It keeps calling the one seed rule,
  `elidex_form::record_focus_snapshot`, immediately after its `set_focus_bit`; it fires no listener, so the instant is
  already R-1's. Its own redundant-focus return (B-P9) stays: it is the VM's copy of focusing steps step 5 until A2c
  routes VM `focus()` through `focusing_steps` (record LR-B4). Only its comment changes (§4.6).
- **B-7 No write chooses by kind** — [slot req 5; 後方互換性は維持しない]. Deleted: the `is_text_control` test and the
  remove branch in `record_focus_snapshot`; `clear_focus_snapshot`, its re-export in `elidex-form`'s `lib.rs` and its
  call and import in the reconciler's `type` arm.
- **B-8 The seed rule's home is `elidex-form`** — [slot req 3; Layering mandate; crate graph]. The slot allows
  `elidex-form` or `elidex-dom-api`; `elidex-dom-api` has no `elidex-form` dependency (the `FocusEventSink` trait doc:
  "this crate has no `elidex-form` dependency") and cannot read `FormControlState`. So the rule (population, `F0`,
  unmarked) is `elidex_form::record_focus_snapshot`; the instant is the caller's (the transition at step 4.1, VM
  `focus()` after its designation). The shell sink and the VM host only call it.
- **B-9 Lifetime** — [ECS-native first; B-P10]. §4.7. The component is written only by the seed (insert, overwriting),
  the mark (in place) and the take (remove); despawn removes it. A component left by VM `blur()`, VM `focus()` away, or
  removal (§2.1.4, no `change`) is never read: the only read is the take at the holder's focus loss, and every
  designation of a Population element re-seeds first (B-P10).

### §0.5 OO idioms translated (ECS-native first)

| Pattern a class-based engine uses [contrast] | elidex (ECS) |
|---|---|
| a text-control object remembers its value at focus in a member field | `FocusValueSnapshot` component on the control's entity; despawn cleans it |
| a per-class virtual "has activation behavior / does change apply" | one exhaustive `match` over `FormControlKind` (static dispatch, the Plugin-first built-in form), read once at focus loss |
| a focus observer / listener that captures the value | no observer: the canonical transition calls the sink's seed at a defined step (4.1); the one rule lives in `elidex-form` |
| an "is text control" subclass test at each write | the population is the element (`input` / `textarea`), fixed for the entity's lifetime; the kind is read only at focus loss |

## §1. Shape and verdict; DF-1 and DF-2: one slice or two

Edge-dense (≥3 intersecting axes: seed instant × population × `type` change × non-user writes × user edits × the VM
`focus()` path; §5), so plan-review is mandatory. A terminal slice under the approved umbrella.

**The open question the slot leaves to plan-review: one per-PR slice or two.** Lens over CLAUDE.md's principles:

| Principle | One slice (DF-1 + DF-2) | Two slices (DF-2 PR, then DF-1 PR) |
|---|---|---|
| Ground order | neutral: both defects are [spec] (DF-2 under R-2) | neutral |
| ECS-native first | **selects**: the component's shape (B-1) and its population (B-2) are one decision about one component; split, the first PR defines the component on a population or a shape the second PR rewrites | — |
| One issue, one way | **selects**: `record_focus_snapshot` and `take_change_on_blur` are rewritten once; split, an intermediate main has the new population with H's comparison (or the reverse), a state no plan describes as an end state | — |
| Ideal over pragmatic | neutral | neutral |
| Edge-dense work | neutral: a plan-reviewed slice under an approved umbrella is a terminal unit; the cells separate cleanly (DF-1's need only B-1 / B-4 / B-5, DF-2's only B-2 / B-3 / B-7; DC-19 needs only DF-2) | permits it; it lowers each PR's edge count |
| Supported-surface testing, 設計優先, Plugin-first, others | not engaged | not engaged |

**Recommendation: one per-PR slice, two behaviour commits** (DF-2 first, then DF-1; §8 commit plan), each green with its
red-first cells. Ground: the two principles that select (ECS-native first, One issue, one way) both select it; none
selects the split. **Alternative**: two PRs in the same order, both planned by this memo (no second plan memo): PR-α =
B-2, B-3, B-7 and DC-14 to DC-16, DC-18, DC-19, PB-*; PR-β = B-1, B-4, B-5, B-6 and DC-1, DC-1v, DC-8, DC-10, DC-11,
E17v. The comment and allowlist work (§6) then splits by the lines each PR rewrites. The plan-review decides (slot row);
this memo does not.

## §2. Coupled invariants

- **I-B1 One judgement, one predicate** (umbrella X1 × B-3 × B-4): whether `change` fires at step 2.1 is computed only
  in `take_change_on_blur`; the kind conjuncts only in `change_on_blur_kind_applies`; every sink only dispatches.
- **I-B2 Every designation seeds** (B-P10 × B-5 × B-6 × B-9): each production writer of `set_focus_bit(Some(e))` calls
  `record_focus_snapshot(e)` before anything else can run. This is what makes a stale component unreachable (§4.7). A
  new designation writer must seed; DoD 4 re-runs the B-P10 enumeration.
- **I-B3 One population, two sites** (B-2 × B-4): the seed's element test is the only population test; the take has no
  population test of its own (it reads what the seed wrote). No write and no read tests an `input`'s kind for
  population.
- **I-B4 Seed before listeners** (B-5 × R-1 × DC-10): a `focus` / `focusin` listener's write is a non-user write after
  `F0`. Reentrancy: a `focusin` listener that moves focus on finds `new` already seeded; a canonical move takes it
  (no mark, so no `change`); a VM `focus()` leaves it (B-9).
- **I-B5 The mark is the funnel's** (plan (1) I1 / I2 × B-1): `user_change_from` is set only by
  `note_user_value_change` from `apply_user_edit`; non-user writers call nothing (plan (1) D-4; Q5).
- **I-B6 Redundant focus does not re-seed** (E-17 × B-P7 × B-6): both designation paths return before the seed.
- **I-B7 `type` changes write nothing to the component** (B-7 × DF-2 × the type-change steps 1–3 and 6): the value they
  write is a non-user write, judged at focus loss under D-3 (DC-19) or [spec] (DC-18).
- **I-B8 Unchanged pairs**: umbrella X3 / X4 / X5 (fixup listeners never mark; IME commit through the funnel) and plan
  (2)'s fixup path to the same sink (its C6 / C19, the consumed-component assertions in
  `content_focus_fixup_tests.rs`) are not affected: the fixup never designates an element, so B-5 does not reach it.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §6.6.4 Processing model | *focus update steps* step 2.1 | all four conjuncts at focus loss; the reference value "when the control was first focused" (R-1); D-3's completion | `take_change_on_blur`, `change_on_blur_kind_applies` (NEW); DC-1 to DC-19 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focus update steps* step 4.1, before step 4.4 | the seed at designation, before `focus` | `focus_update_steps`; DC-10 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focusing steps* step 5 | redundant focus returns | `focusing_steps`; VM `focus()` early return; E-17 cells | ✓ | yes |
| WHATWG HTML §4.10.5.1 States of the type attribute | each state's "The input and change events (do not) apply"; the Text state as missing / invalid value default | B-3's table; F-2 | `change_on_blur_kind_applies` (NEW) | ✓ | no |
| WHATWG HTML §4.10.5 The input element | activation behavior (R-2); *update an input element's type* steps 1–3, 6 | the `type` arm writes no component | reconciler `type` arm; DC-14 to DC-16, DC-18, DC-19 | ✗ | yes |
| WHATWG HTML §4.10.5.1.14 Color state (type=color) | *update a color well control color* step 2 | dirty → live value; not dirty → content attribute | `sanitize_color` doc; OQ-2 | ✗ | no |
| WHATWG HTML §4.10.5.4 Common input element APIs | `value` setter, mode value, steps 3–4 | dirty flag set before sanitization | F-1 (argument only) | ✗ | no |
| WHATWG HTML §4.10.5.5 Common event behaviors | "not fired in response to changes made … by scripts" | non-user writers do not mark | unchanged (I-B5) | ✗ | yes |
| WHATWG HTML §4.10.11 The textarea element | no `change` rule | D-4: same predicate | `TextArea` arm of B-3 | ✗ | yes |
| WHATWG HTML §4.10.18.1 A form control's value | a control's value is its internal state | R-3: `F0` is `FormControlState`'s value whatever the state | B-1 | ✗ | no |

Numbers and titles: `.claude/tools/webref heading html 6.6.4` → §6.6.4 Processing model `#focus-processing-model`;
`heading html 4.10.5` → §4.10.5 The input element, §4.10.5.1 States of the type attribute
`#states-of-the-type-attribute`; `heading html 4.10.5.1.14` → Color state (type=color); `heading html 4.10.5.1.19` →
Image Button state (type=image); `heading html 4.10.5.4` → Common input element APIs `#common-input-element-apis`;
`heading html 4.10.5.5` → Common event behaviors `#common-input-element-events`; `heading html 4.10.11` → The textarea
element; `heading html 4.10.18.1` → A form control's value (run 2026-11-22). Step texts: `webref body html
focus-update-steps`, `focusing-steps`, `the-input-element`, `update-a-color-well-control-color`, `dom-input-value`.
Breadth K = 1, M = 10.

### §3.1 Normative text (beyond the re-derivation's Q1–Q19, which this memo cites by ID)

| ID | Source (`.claude/tools/webref body html <anchor>`) | Text |
|---|---|---|
| QB-1 | `the-input-element`, §4.10.5.1.1 Hidden, §4.10.5.1.18 Submit Button, §4.10.5.1.19 Image Button, §4.10.5.1.20 Reset Button, §4.10.5.1.21 Button | "The input and change events do not apply." |
| QB-2 | `the-input-element`, §4.10.5.1.2 to §4.10.5.1.17 (Text/Search, Telephone, URL, Email, Password, Date, Month, Week, Time, Local Date and Time, Number, Range, Color, Checkbox, Radio Button, File Upload) | "The input and change events apply." |
| QB-3 | `the-input-element` (the `type` attribute) | "The attribute's missing value default and invalid value default are both the Text state." |
| QB-4 | `focus-update-steps` step 4.1 | "If entry is a focusable area, and the focused area of the document is not entry: 1. Set document's relevant global object's navigation API's focus changed during ongoing navigation to true. 2. Designate entry as the focused area of the document." (step 4.4 fires `focus`) |
| QB-5 | `focusing-steps` step 5 | "If new focus target is the currently focused area of a top-level traversable, then return." |
| QB-6 | `dom-input-value` (mode value, on setting) | "1. Let oldValue be the element's value. 2. Set the element's value to the new value. 3. Set the element's dirty value flag to true. 4. Invoke the value sanitization algorithm, if the element's type attribute's current state defines one." |

### §3.2 User-input touch audit

No user-input path is added or changed. The funnel (`apply_user_edit`) and its four shell callers are as landed (plan
(1) §3.1); only what the mark writes into is re-shaped (B-1). The key router still reads the live kind at each
`keydown` (re-derivation P15), which DF-2 (i) depends on.

## §4. Mechanism — `1f81fdc2` → this PR

### §4.1 The component (B-1)

`FocusValueSnapshot { first_focus: String, user_change_from: Option<String> }`, private fields, written only in
`focus_snapshot.rs`. Accessors `first_focus()` / `user_change_from()` (NEW) for `test_support::snap`. The clone policy
stays "deliberate non-copy" (`tree_clone.rs`'s table; only its parenthetical text changes, §6).

### §4.2 The seed (B-2, B-5, B-6, B-8)

`record_focus_snapshot(dom, entity)`: if `entity` is an HTML `input` or `textarea` element and has `FormControlState`,
insert `FocusValueSnapshot { first_focus: value.clone(), user_change_from: None }` (overwriting any earlier one). Else
nothing. Callers: the shell sink's `seed_focus_snapshot` (from `focus_update_steps` at step 4.1, B-5) and VM `focus()`
(after its designation, B-6). The doc of the `seed_focus_snapshot` method of the `FocusEventSink` trait states the instant (step 4.1,
before step 4.4) and drops "Seed timing does not decide whether a non-user write fires `change`" (DC-10 contradicts it).

### §4.3 The mark

`note_user_value_change(dom, entity, before)`: `user_change_from.get_or_insert(before)` on an existing component.
Behaviour unchanged; its doc is rewritten (§6).

### §4.4 The judgement and the predicate (B-3, B-4)

`change_on_blur_kind_applies(kind)` (NEW), derived from QB-1, QB-2 and R-2 (Checkbox, Radio Button, File Upload and Color
define an input activation behavior):

| Kind (state) | `change` applies (QB-1 / QB-2) | defined input activation behavior (R-2) | Result |
|---|---|---|---|
| `TextInput`, `Search`, `Tel`, `Url`, `Email`, `Password`, `Date`, `Month`, `Week`, `Time`, `DatetimeLocal`, `Number`, `Range` | yes | no | **true** |
| `TextArea` (the `<textarea>` element; also `<input type=textarea>`, F-2, Text state by QB-3, same answer) | spec silent (re-derivation PE-2) | — | **true** (D-4) |
| `Checkbox`, `Radio`, `File`, `Color` | yes | yes | false |
| `Hidden`, `SubmitButton`, `ResetButton`, `Button` | no | — | false |
| `Select`, `Output`, `Meter`, `Progress` | not an `input` state; on an `input` only through F-2's mapping (`"select-one"`) | — | false (the mapping is O-B1's; the arm's doc says so) |

`take_change_on_blur`: B-4. The `user_change_from` test is "the user has changed the element's value … while the
control was focused" (plan (1) D-1's funnel); `V != first_focus` is Q1 (F); `V != user_change_from` is D-3's K-first.
Step 2.1's "or its list of selected files" has no element under R-2 (the re-derivation §0.5); B-3's `File` arm is
false.

### §4.5 `type` change (B-7)

The reconciler's `type` arm keeps steps 1–3 and 6 and loses the clear. The comment block above the clear is deleted
with it.

### §4.6 VM `focus()` (B-6)

Code unchanged but for the call it already makes; the comment "Seed the change-on-blur snapshot for text controls,
just like the shell `set_focus` reconciler" becomes "seed the value at first focus (the one seed rule,
`elidex_form::record_focus_snapshot`) at the designation, as the canonical transition does at focus update steps step
4.1". The doc paragraph on the old holder's component ("its user-edit baseline") is re-worded to "its first-focus value
and mark".

### §4.7 Lifetime

| Event | Write | Reader consequence |
|---|---|---|
| designation (step 4.1; VM `focus()`) of a Population element | insert `{F0, None}`, overwriting | a stale component from an earlier session is replaced (I-B2) |
| designation of any other element | none | never carries one |
| redundant focus (QB-5; VM early return) | none | `F0` and the mark survive (E-17) |
| a `focus` / `focusin` listener writes the value | none | non-user write after `F0` (DC-10) |
| user value change | `user_change_from.get_or_insert(before)` | the mark |
| non-user write (setters, attributes, `type` change steps 1–3 and 6, reset, `setRangeText`) | none | judged at focus loss |
| `type` change | none (B-7) | kind read at focus loss (B-3) |
| focus loss through the canonical transition (UA move, `blur_current`, the step-17 fixup) | remove (take) | the verdict |
| VM `blur()`, VM `focus()` to another element (A2c), removal of the focused element (§2.1.4) | none | left in place, unreachable until the next designation re-seeds (B-9) |
| despawn | removed by the World | — |
| clone | not copied | — |

### §4.8 ECS-native and Layering checks

- One component on the entity; no side-store, no registry, no observer; one component get per transition.
- `vm/host` only calls `record_focus_snapshot`; it computes no population and no predicate (Layering mandate).
- `elidex-dom-api` owns the instant, `elidex-form` the rule and the judgement; no new cargo edge.

## §5. Edge matrix and test plan

Axes: **S** seed instant (before listeners / redundant / VM), **P** population (input by kind class / textarea / other
`FormControlState` elements), **T** `type` change while focused (none / text→text / to an excluded kind / to a value-mode
non-text kind / round trip / non-text→text), **W** non-user writes (none / before the first edit / after it / in a
`focus` listener / a sanitization in the round trip), **U** user edits (none / net zero / back to `F0` / net change),
**F** path (UA sink / `blur_current` / fixup / VM `focus()` seed).

"Base" = expected result at `1f81fdc2` with the fix absent (red = fails as the defect predicts). Shell cells are in
`content_change_on_blur_tests.rs` and use its `setup` / `pair` / `ua_focus` / `user_key` / `script_set_value`
helpers ("UA focus" = `set_focus`, the canonical transition; the cells' "by Tab" is the same transition). Unit cells
are engine-independent: `elidex-form` (`focus_snapshot.rs` and `reconciler_tests.rs` test modules),
`elidex-dom-api` (`update_steps.rs` test module), `elidex-js` VM (`tests_html_element_proto.rs`).

| Cell | Axes | Test (NEW unless stated) | Sequence (re-derivation §6.1 / §6.2 unless stated) | Expected | Ground | Base |
|---|---|---|---|---|---|---|
| DC-1 (C6) | W before, U back to F0; both kinds | `c6_*` rewritten, renamed `dc1_script_write_then_user_revert_to_first_focus_value_{input,textarea}` | §6.1 DC-1 | no `change` | [spec] Q1 (F) | red |
| DC-1v | F VM, W before, U back to F0 | `dc1v_vm_focus_then_script_write_then_user_revert` | `b.focus()` (VM); script `b.value = "x"`; user Backspace (`""`); UA focus `c` | no `change` | [spec] Q1 (F); slot req 3 | red |
| DC-2 to DC-7, DC-9 (C20–C23, C25, C26, C24) | W before, U net zero | existing `c20_*`, `c21`–`c26` (renamed, F-4) | as tabled there | no `change` | D-3 | green |
| DC-8 | W before, U net change then back to F0; both kinds | `dc8_script_write_user_type_then_delete_to_first_focus_value_{input,textarea}` | §6.1 DC-8 | no `change` | [spec] Q1 (F) | red |
| DC-10 | S before listeners, W in `focus` listener | `dc10_focus_listener_write_then_user_revert` (a `focus` listener on `b` sets `b.value = "x"`) | §6.1 DC-10 | no `change` | [spec] Q1 (F); R-1 | red |
| DC-10u | S | dom-api `focusing_steps_designates_last_and_fires_in_spec_order`, assertion flipped | `seeds_len_at_focus` | `Some(1)`; message cites R-1 | R-1 (QB-4) | red |
| DC-10r | S × reentrancy | dom-api `focusin_reentrant_focus_finds_new_seeded`; `TestSink` gains `reentrant_on_focus` (NEW) | focus `b`; the `Focus` event moves the bit to `c` | `seeds == [b]` | R-1: the seed belongs to the designation | red (the guard skips the seed) |
| DC-11 (C7 (l)) | W before, U net zero | `c7l_*` rewritten at the judgement level, renamed `dc11_net_zero_user_edit_after_script_write_is_false` | §6.1 DC-11 | `take_change_on_blur == false` | D-3 | green |
| DC-12 | controls | existing C1, C3, C5, C9, C14, C16, C18, C19 | — | as today | [spec] E-6, E-7, M8 | green |
| DC-13 | W after; U net change | existing `c4_*` (M1), `c17_*` (M6) | — | `change` | D-3 | green |
| DC-14 | P checkbox at seed; T non-text→text | `dc14_checkbox_focused_then_text_then_user_edit` | §6.2 DC-14 | `input`, then `change` | [spec] Q1 under R-2; R-3 | red |
| DC-14u | same, unit | `reconciler_tests` `dc14u_seed_on_checkbox_survives_flip_to_text` | seed on `<input type=checkbox>`; `set_attribute("type","text")`; `apply_user_edit` inserts `a`; take | `true` | as DC-14 | red |
| DC-15 | T to Number after U | `dc15_text_to_number_after_user_edit` | §6.2 DC-15 | `change` | [spec] Q1 under R-2 | red |
| DC-16 | T to Checkbox after U | `dc16_text_to_checkbox_after_user_edit_no_change` | §6.2 DC-16 | no `change` | [spec] R-2 | green |
| DC-16u (e2c) | same, unit | `e2c_*` rewritten: the component survives the flip; take → `false` | — | component present after the flip; `false` | B-7; R-2 | red (component absent) |
| DC-17 | W, U net zero, W; both kinds | `dc17_script_user_net_zero_script_{input,textarea}` | §6.1 DC-17 | `change` | D-3 | green |
| DC-18 | T round trip through Checkbox | `dc18_round_trip_through_checkbox` | §6.2 DC-18 | `change` | [spec] under R-2 | red |
| DC-19 | T round trip through Color | `dc19_round_trip_through_color` | §6.2 DC-19 | `change` | [spec] under R-2; D-3 | red |
| PB-1 | P every `input` kind | `focus_snapshot` `seed_writes_every_input_kind` (a loop over every `FormControlKind` an `input` can carry, on an `input` element) | seed | component present, `first_focus` = the value, unmarked | slot Population; B-2 | red (non-text kinds absent) |
| PB-2 | P textarea | `seed_writes_textarea` (a `textarea` element) | seed | present | D-4 | green |
| PB-3 | P other `FormControlState` elements | `seed_writes_nothing_for_non_population` (`select`, `button`, `output`, `meter`, `progress` elements, one set with a component pre-inserted by the test, one without) | seed | no insert; a pre-existing component is untouched (no write at all) | slot Population ("get no write") | red for the untouched half (today the seed removes it) |
| PB-4 | predicate | `kind_predicate_truth_table` (every `FormControlKind`, §4.4) | — | §4.4's column | QB-1, QB-2, R-2, D-4 | n/a (new fn) |
| PB-5 | T to Hidden after U | `take_false_after_flip_to_hidden` (unit) | seed text, edit, `type=hidden`, take | `false` | QB-1 | green |
| PB-6 | T to File after U | `take_false_after_flip_to_file` (unit) | seed text, edit, `type=file`, take | `false` | R-2 | green |
| PB-7 | T to Color, still Color at focus loss | `take_false_at_color` (unit) | seed text, edit, `type=color`, take | `false` | R-2 | green |
| PB-8 | `no_snapshot_for_non_text_control` rewritten | `button_kind_input_is_seeded_and_judged_false` (`control` makes an `input`) | seed on `<input type=button>`; script value `x`; take | component present after the seed; take `false` | B-2, B-3 (QB-1) | red (absent) |
| E17 | S redundant | existing VM redundant-`focus()` test (`tests_html_element_proto.rs`, its `take_change_on_blur` assertion) and the dom-api step-5 cells | — | as today | QB-5 | green |
| E17v | S redundant × F0 | `vm_redundant_focus_keeps_first_focus_value` | VM `focus()` (value `""`); script `b.value = "x"`; Rust `apply_user_edit` deletes the `x` (value `""`); VM `focus()` again; read the component, then take | `first_focus() == ""` and `user_change_from() == Some("x")` (a re-seed would leave it unmarked); take `false` (`V` = `F0`) | QB-5; Q1 (F) | red (no `first_focus` at base) |

Mutation guards (run on the final head only; each must turn the named cells red, then reverted):

| Guard | Mutation | Red cells |
|---|---|---|
| MG-B1 (plan (1) MG1 re-defined) | `take_change_on_blur` drops the `user_change_from` test (fires on `V != F0` alone) | C1, C3, C9, C14, C16 (DC-12) |
| MG-B2 (plan (1) MG6 re-defined) | the comparison drops `V != user_change_from` (reading L) | DC-2 to DC-7, DC-9, DC-11 |
| MG-B3 | the comparison drops `V != first_focus` (H's predicate) | DC-1, DC-1v, DC-8, DC-10 |
| MG-B4 | the seed moves back after `fire_focus_event` | DC-10, DC-10u, DC-10r |
| MG-B5 | the seed tests `is_text_control` again | DC-14, DC-14u, PB-1, PB-8 |
| MG-B6 | the reconciler's `type` arm removes the component for non-text kinds | DC-15, DC-16u, DC-18, DC-19 |
| MG-B7 | `change_on_blur_kind_applies` returns `true` for every kind | DC-16, PB-4 to PB-8 |
| MG-B8 | the seed writes every element with `FormControlState` | PB-3 |
| MG-B9 | VM `focus()`'s early return removed | E17, E17v |

Coverage: engine-independent units in `elidex-form`, `elidex-dom-api` and the VM; shell integration cells for the
user-visible sequences. No WPT subset is claimed.

### §5.1 Existing tests that change (beyond the cells above)

- `record_clears_stale_snapshot_when_no_longer_text_control` (`focus_snapshot.rs`): deleted; it asserts the remove branch
  B-7 deletes. PB-1 / PB-8 cover the population.
- `c7a_seed_is_unmarked`, `c7b`, `c7d`, `c7j`, `c7k`: assertions move to `test_support::Snap`'s new shape
  (`Unmarked { first_focus }` / `Marked { first_focus, user_change_from }`, `Absent`).
- `e2d_type_flip_between_text_kinds_preserves_focus_snapshot`: assertion holds; its comment ("only a non-text
  destination clears the snapshot") is rewritten.
- `test_support::Snap` / `snap`: re-defined on B-1.

## §6. Comments, names and allowlist rows the code PR rewrites

Each listed site is rewritten with the requirement that replaces it; no rewritten comment names a measurement or an
engine as a ground.

- **Allowlist rows pruned** (`.claude/tools/ecs-mission-allowlist.tsv`, at `1f81fdc2`): the row at line 44 (T, this slot) — the doc
  of `note_user_value_change`; the row at line 141 (E, slot `-`) — the doc of `focus_with_caret_at_end`; the row at line 43 (T,
  `#11-core-browser-parity-deviations`) — `sanitize_color`'s doc. Each is pruned in the commit that rewrites its line
  (`python3 .claude/tools/ecs_mission_ratchet.py --prune`, then the ratchet → `ok (292 allowlisted, 0 new, 0 stale)`).
- **`focus_snapshot.rs`**: the module doc (the `<textarea>` rule as measured behaviour → D-4, One issue, one way; "holds
  one thing … `None` until the first user value change" → B-1); `FocusValueSnapshot`'s doc ("Only text controls carry
  it"; "No focus-time copy of the value is kept … PM 393" → B-1, B-2); `record_focus_snapshot`'s doc ("**if** it is a
  text control"; "after §6.6.4 focus update steps step 4.4"; the removal paragraph → B-2, B-5); `note_user_value_change`'s
  doc (the measured model as ground → "the mark records the value just before the first user value change: D-3's K-first
  reference"); `take_change_on_blur`'s doc ("not a tracked text control" → B-3, B-4); `clear_focus_snapshot` deleted with
  its doc.
- **`content_change_on_blur_tests.rs`**: module doc ("model H of §4.3" → this memo and the re-derivation); the
  "Commit (3): the mark decides, and the first mark re-baselines" banner; `focus_with_caret_at_end`'s doc (its setup
  stated by what it does: a script write before focus leaves the caret at the end); C6's doc (now DC-1); C24's doc (W6 as
  PM 307's accepted divergence → "D-3, class (b); DC-9"); `writer_rebaselines` → `writer_then_net_zero_user_edit`, and
  `c21`–`c26`'s `_rebaselines` suffix → `_net_zero_user_edit` (F-4, OQ-3).
- **`update_steps.rs`**: the `seed_focus_snapshot` trait doc and the inline comment at the seed (B-5, §4.2); the
  step-5 comment in `focusing_steps` ("must not refresh the snapshot baseline" → "must not re-seed the first-focus
  value"); `commit_change_on_blur`'s doc ("a focused text control's value was user-edited since focus" → the
  Population and B-4).
- **`content/focus.rs`**: `seed_focus_snapshot`'s comment ("After §6.6.4 focus update steps step 4.4"; "the baseline is
  the value before the first user value change, plan D-1"); `dispatch_change_on_blur`'s doc ("on text-control blur").
- **`reconciler.rs`**: the comment block of the deleted clear; the `type` arm's opening comment keeps its spec text.
- **`html_element_proto.rs`**: §4.6.
- **`tree_clone.rs`**: the `FocusValueSnapshot` entry's parenthetical ("the value before the first user value change,
  plan D-1" → "the first-focus value and the user-change mark of the current focus session").
- **`user_edit.rs`**: none required (its qualifier pointer to `focus_snapshot` stays valid).
- **`sanitize.rs`** (PM carry-over (a); a facet of `#11-core-browser-parity-deviations`): the "Dirty-value-flag source"
  paragraph is rewritten to state step 2 as the spec does (Q16, QB-6, F-1). What the rest says depends on OQ-2: under
  (B) the code follows step 2 and the paragraph says so; under (A) the paragraph states the remaining difference as a
  defect with the slot ID, by its property (not dirty, live value ≠ content attribute) and F-1's example, naming no
  engine. Either way the ground row 43 records and the comment's re-evaluation condition (row 43's neighbour line) are
  removed.

## §7. Touch list with line counts (`wc -l` at `1f81fdc2`)

| File | Lines | Change | Split rule |
|---|---|---|---|
| `crates/dom/elidex-form/src/focus_snapshot.rs` | 258 | B-1 to B-4, B-7; docs; tests (PB-*, DC-11, rewrites) | < 1000 |
| `crates/dom/elidex-form/src/reconciler.rs` | 340 | clear deleted (B-7) | < 1000 |
| `crates/dom/elidex-form/src/reconciler_tests.rs` | 747 | e2c rewritten, e2d comment, DC-14u | < 1000 |
| `crates/dom/elidex-form/src/test_support.rs` | 42 | `Snap` re-defined | < 1000 |
| `crates/dom/elidex-form/src/lib.rs` | 69 | `clear_focus_snapshot` re-export removed | < 1000 |
| `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | 773 | B-5; docs; DC-10u, DC-10r, `TestSink.reentrant_on_focus` | < 1000 (about +25) |
| `crates/shell/elidex-shell/src/content/focus.rs` | 404 | comments | < 1000 |
| `crates/script/elidex-js/src/vm/host/html_element_proto.rs` | 909 | comments only (B-6) | < 1000 (±0) |
| `crates/script/elidex-js/src/vm/tests/tests_html_element_proto.rs` | 658 | E17v | < 1000 |
| `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs` | 679 | DC cells, renames, comments | < 1000 (about +170) |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 418 | one table cell | < 1000 |
| `crates/dom/elidex-form-core/src/sanitize.rs` | 308 | doc (and, under OQ-2 (B), the Color arm) | < 1000 |
| `crates/dom/elidex-form-core/src/sanitize_tests.rs` | 451 | under OQ-2 (B) only: F-1's cell | < 1000 |
| `.claude/tools/ecs-mission-allowlist.tsv` | 303 | 3 rows pruned | data |

No touched file is over 1000 lines, so no prereq split. Not touched: `elidex-form-core/src/lib.rs` (960 lines; B-3 keeps
the predicate out of it, so it does not grow toward the threshold), `user_edit.rs`, `content_focus_fixup_tests.rs`
(its cells are controls, I-B8). Re-run: `wc -l` on the paths above.

## §8. Commits, open questions, scout questions

**Commit plan (one-slice recommendation, §1).** (1) DF-2: B-2, B-3, B-7 with PB-*, DC-14 to DC-16(u), DC-18, DC-19 and
the comment lines those functions carry; (2) DF-1: B-1, B-4, B-5, B-6 with DC-1(v), DC-8, DC-10(u/r), DC-11, E17v, the
`Snap` re-definition, the remaining §6 comments and renames, rows 44 and 141 pruned; (3) `sanitize.rs` (OQ-2) and row 43
pruned. Every commit builds and is green; the red-first results per cell are recorded in the PR body.

**Open questions (for plan-review / the PM).**

- **OQ-1 One slice or two** — §1. Recommendation: one slice, commits (1) and (2). Alternative: PR-α / PR-β.
- **OQ-2 `sanitize_color` behaviour** (F-1). (A) Comment only, as the PM's carry-over names it: state step 2 correctly
  and record the remaining difference as a defect on `#11-core-browser-parity-deviations` (LR-B2 (A)). (B) Also fix it
  in commit (3): the Color arm sanitizes `default_value` when `dirty_value` is false (the content attribute's mirror,
  precondition Q-S3), with F-1's cell (`<input type=number value=red>`, `type = "color"` → `"#ff0000"`) and the existing
  `sanitize_color_*` cells. **Recommendation: (B)**. Ground: rung 1 decides the behaviour (step 2's text); and a doc
  comment that records its own function's spec divergence, written by the PR that edits that doc, is a deferral the
  TODO 先送り禁止 rule asks to justify. (A) if Q-S3 shows `default_value` does not mirror the attribute (the fix then
  needs a read of `Attributes` and is not a one-arm change). The PM decides: (B) widens this PR by a behaviour change in
  another slot's facet.
- **OQ-3 Test renames** (F-4). Recommendation: rename in commit (2) (the model they name is gone; a name is a claim).
  Alternative: keep the names and rewrite only the docs.

**Questions for scout** (answered before commit (1); none changes a decision here):

- **Q-S1** The canonical test for "HTML `input` element" / "HTML `textarea` element" in `elidex-form` (`TagType` plus
  namespace, or an existing helper); and whether a foreign-namespace `input` ever carries `FormControlState`.
- **Q-S2** What `FormControlState::value()` holds for `Checkbox` / `Radio` without a `value` attribute (default/on mode).
  It is `F0` under R-3 for DC-14; DC-14's expected value does not depend on it (re-derivation §6.2).
- **Q-S3** Whether `FormControlState.default_value` equals the `value` content attribute after every attribute write
  (`setAttribute` / `removeAttribute("value")` / the `defaultValue` setter / clone). OQ-2 (B)'s precondition.

## §9. Ledger records (applied by the PM)

Slot text cites files and symbols, no line numbers.

- **LR-B1 → `#11-change-on-blur-baseline-first-focus-value`.**
  - *At this memo's landing* — append: "Plan: `docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md` (per-PR slice
    of the focus-fixup umbrella). Plan-review decided: [one per-PR slice | two per-PR slices, PR-α (population) then
    PR-β (first-focus value)]." The row's requirements are unchanged.
  - *At the code PR's landing* — close: "Fixed by #NNN. `FocusValueSnapshot` (`elidex-form` `focus_snapshot.rs`) holds
    the value at first focus and the user-change mark on every `input` and `textarea` element from the seed, seeded at
    focus update steps step 4.1 before the `focus` / `focusin` listeners (`focus_update_steps`) and by VM `focus()`
    through the one rule `record_focus_snapshot`. `take_change_on_blur` fires iff the kind at focus loss passes step
    2.1's conjuncts (one private predicate, under R-2; `<textarea>` by D-4), the user changed the value, and the value
    differs from both the first-focus value and the value just before the first user edit (D-3). `clear_focus_snapshot`
    and the reconciler's `type`-arm clear are deleted. Cells DC-1 to DC-19 green. Allowlist rows pruned: the
    `note_user_value_change` doc (T, this slot) and the `focus_with_caret_at_end` doc (E)."
- **LR-B2 → `#11-core-browser-parity-deviations`** (facet: `sanitize_color`). Under OQ-2 (B): "Fixed in #NNN:
  `sanitize_value`'s Color arm follows HTML *update a color well control color* step 2 (the content attribute when the
  dirty value flag is false); its doc no longer names existing engines as the ground; allowlist row pruned." Under (A):
  "Defect [spec]: `sanitize_value`'s Color arm (`elidex-form-core` `sanitize.rs`) sanitizes the live value; HTML *update
  a color well control color* step 2 reads the `value` content attribute when the dirty value flag is false. They
  differ when the flag is false and the live value differs from the attribute, e.g. `<input type=number value=red>`
  then `type = "color"`: spec `#ff0000`, elidex `#000000`. The doc comment now states this; its allowlist row was pruned
  by #NNN."
- **LR-B3 → the PM: slot draft for O-B1** (F-2), proposed ID `#11-input-type-keyword-state-mapping` (the PM accepts or
  renames it): "Defect [spec]: `FormControlKind::from_type_str` (`elidex-form-core`) maps the `type` keyword `image` to
  `TextInput` (HTML has an Image Button state, to which the input and change events do not apply), and `select-one` /
  `textarea` to `Select` / `TextArea`, where HTML's invalid value default is the Text state. An `<input type=image>` is
  seeded, edited and judged as a text control. **Trigger**: the next PR that touches `from_type_str` or adds the Image
  Button state. **Why deferred**: found by the change-on-blur baseline plan, whose cells do not depend on it." Proposed
  owner L3 (form domain); owner and re-eval date are the PM's. Ledger check: F-2.
- **LR-B4 → `#11-vm-host-synthetic-dom-event-dispatch` (A2c)** — append: "When VM `focus()` routes through
  `focusing_steps`, its own redundant-focus return and its `record_focus_snapshot` call are deleted: the seed is then
  the transition's step-4.1 call (`docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md` B-5, B-6), and the VM
  sink's `commit_change_on_blur` calls `take_change_on_blur` (umbrella X1)."
- **LR-B5 → the PM: PM 393 D1 and PM 307's "initial = before"** — the re-derivation's LR-6 proposed their withdrawal;
  B-1 is the re-decision the slot names. Nothing new to withdraw here.

## §10. Definition of done (the code PR)

1. Cells of §5 green on the final head; the cells marked red at base observed red at `1f81fdc2` with the fix absent
   (results in the PR body); MG-B1 to MG-B9 each red on their cells, then reverted.
2. `git grep -n -w 'clear_focus_snapshot' -- crates` → 0; `git grep -n -w 'is_text_control' --
   crates/dom/elidex-form/src/focus_snapshot.rs crates/dom/elidex-form/src/reconciler.rs` → 0 (control: the same
   command over `crates/shell/elidex-shell/src/content/ime.rs` → ≥ 1).
3. `python3 .claude/tools/ecs_mission_ratchet.py` → `ok (292 allowlisted, 0 new, 0 stale)` (295 at `1f81fdc2`, minus
   rows 43, 44, 141); `/usr/bin/grep -c 'change-on-blur-baseline-first-focus-value' .claude/tools/ecs-mission-allowlist.tsv`
   → 0 (control: `blur-focus-cleared-before-blur-event` → 1).
4. B-P10's enumeration re-run on the final head: still two production designation writers, both seeding (I-B2).
5. `cargo fmt --all`; `cargo test -p elidex-form -p elidex-form-core -p elidex-dom-api -p elidex-js -p elidex-shell
   --all-features`; `mise run ci` green (`grep 'ERROR task failed'` on its log → none).
6. PR body carries §9's texts for the PM and OQ-1 / OQ-2's outcomes.

This docs PR: `python3 .claude/tools/ecs_mission_ratchet.py` → `ok (295 allowlisted, 0 new, 0 stale)`;
`python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md`
exits 0. `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-11-focus-fixup-program.md` exits 2 at
`1f81fdc2` and on this branch ("no table matched schema …"): the umbrella has none of the checker's table schemas, so
it is not scanned — a skip, not a clean run, and not changed by this PR.

## Changelog

- **Rev 1 (2026-11-22)** — initial plan from the slot row (`#11-change-on-blur-baseline-first-focus-value`, PM 861 /
  909) and the re-derivation; premises re-verified at `1f81fdc2`; findings F-1 to F-4; open questions OQ-1 to OQ-3.
