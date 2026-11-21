# `focus-fixup-change-on-blur-baseline` — the change-on-blur judgement compares with the value at first focus, over a population chosen by element (per-PR plan-memo, two slices)

This memo plans two per-PR slices of the umbrella `docs/plans/2026-11-focus-fixup-program.md` (its §7 erratum names
this fix as "a separate code PR, planned as a slice of this umbrella"): **PR-α** (the population, DF-2) and then
**PR-β** (the first-focus value, DF-1). Each is a terminal slice by CLAUDE.md *Edge-dense work* base case once this
memo is plan-reviewed (PM 933, §1). Slot **`#11-change-on-blur-baseline-first-focus-value`** (owner Layout;
registered PM 861, amended at #656 per the re-derivation's LR-2, PM 909). The slot row is the requirement; this memo
plans it and changes none of its decisions. The inputs are the re-derivation
`docs/plans/2026-11-change-on-blur-spec-rederivation.md` ("the re-derivation"): its §0.4 D-3 / D-4, §0.5 R-1 / R-2 /
R-3, §4.1, §6 (DF-1, DF-2, cells DC-1 to DC-19), §6.4 and §7 LR-2. Rev 3, 2026-11-22. Branch
`layout-change-on-blur-baseline-plan`, worktree `wt-cob2`. **Docs only**: this PR adds this memo and, in the
umbrella, an in-line pointer in §7 and a changelog entry; the two code PRs it plans are separate.

File name: plan (2) of the same umbrella is `2026-11-focus-fixup-canonical-steps.md`; this memo takes the same
`focus-fixup-` prefix. Plan (1) (`2026-11-change-on-blur-user-provenance.md`) predates the prefix; no other convention
exists in the umbrella.

**Defects the code PRs remove** (the slot row, stated once there; summarised here only to name the cells).

- **DF-2 [spec] under R-2 — PR-α**: the population is chosen by the kind at each write (the seed, the reconciler's
  `type` arm), not by step 2.1's conjuncts at focus loss. Red today: DC-14, DC-15, DC-18, DC-19; DC-16 is the control.
- **DF-1 (a) [spec] — PR-β**: `take_change_on_blur` compares the value at focus loss with the value just before the
  first user edit; HTML §6.6.4 *focus update steps* step 2.1 compares with the value "when the control was first
  focused". Red today and at PR-β's base: DC-1, DC-8, DC-10. DF-1 (b) is not a defect (D-3); its cells stay green.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` below is at `origin/main` **`1f81fdc2`** (#659), this branch's base
(`git -C <wt> merge-base HEAD origin/main` → `1f81fdc2`). Symbols are the SSoT; a line is a reading aid at that sha.
PR-β's base is PR-α's merge, so PR-β's lines move; its plan cites symbols. The re-derivation read the code at
`29edf6fc`. Drift over the files this memo cites: `git diff --stat 29edf6fc 1f81fdc2 -- <the .rs files of §7>` →
`tree_clone.rs` (73 lines, #659's NodeKind work; the `FocusValueSnapshot` row of its clone-policy table is unchanged in
text) and `content_change_on_blur_tests.rs` (2 lines, #655's inline `` `[contrast]` `` on C24's doc comment, which the
re-derivation already records). No premise symbol moved (run 2026-11-22).

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
| B-P10 | Exactly two production writers designate a focused element, and both seed: the transition's step 4.1 and VM `focus()`. Enumerated by the property "writes `ElementState`'s FOCUS bit" (any production insert or mutation of an `ElementState` value that can carry FOCUS), not by one spelling | Command and result: §0.2.1 |
| B-P11 | `<button>`, `<select>`, `<output>`, `<meter>`, `<progress>` also carry `FormControlState`; `<button>` shares the kinds `SubmitButton` / `ResetButton` / `Button` with `<input>`, and `<input type=textarea>` carries the kind `TextArea`. So no kind test can express "is an `input` element" or "is a `textarea` element" | `crates/dom/elidex-form-core/src/lib.rs` `FormControlKind` (:54-98), `from_tag_and_type_attr` (:232-247), `from_type_str` (:253-279) |
| B-P12 | `is_text_control` has callers besides the two PR-α deletes (key routing, IME, the shell's text-input gate), so it stays | `git grep -n 'is_text_control' -- crates` → `event_handlers.rs:333`, `ime.rs:19/:45`, `content/mod.rs:479`, form-core `:120/:164` |
| B-P13 | The dom-api unit test pins the seed **after** the `focus` event, on a ground R-1 contradicts ("a focus-handler value write must be in the baseline") | `update_steps.rs` `mod tests`, `focusing_steps_designates_last_and_fires_in_spec_order` (`seeds_len_at_focus == Some(0)`, :439-444) |
| B-P14 | The cells of the re-derivation's §6.1 / §6.2 exist as named (C6 = `c6_*`, C20–C26 = `c20_*` / `writer_rebaselines` users, C4 = `c4_*`, C17 = `c17_*`, C7 (l) = `c7l_first_mark_rebaselines_once`) | `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs` (:508-527, :562-679); `focus_snapshot.rs` `mod tests` (:211) |
| B-P15 | `sanitize_color`'s doc comment states *update a color well control color* step 2.1 backwards (it says the content attribute is read when the dirty flag is **set**) and grounds the live-value read on "the interoperable behavior every browser ships" [contrast]; that line is the allowlist row at line 43 of the tsv (class T, `#11-core-browser-parity-deviations`) | `crates/dom/elidex-form-core/src/sanitize.rs` `sanitize_color` doc (:163-171); the Color arm of `sanitize_value` (:285) reads `state.value()` |
| B-P16 | `sanitize_tests.rs` grounds a Color expectation on interop: `color_idl_value_set_round_trips_valid_color`'s comment "B2 (interop-faithful)" | `crates/dom/elidex-form-core/src/sanitize_tests.rs` (:441-445) |
| B-P17 | `FormControlState` is only ever attached to HTML-namespace elements; the canonical HTML-namespace gate is `EcsDom::is_html_namespace` | `crates/dom/elidex-form/src/init.rs` `is_html_namespaced` (:20, used at parse, insert, the `createElement` post-handler and clone); `crates/core/elidex-ecs/src/dom/classify.rs` `is_html_namespace` (:135) |
| B-P18 | The seed-after-focus order is #357's Codex R2 fix ("snapshot-seed-after-focus", `memory/m4-12-landings-ledger.md` #357 entry; `memory/project_canonical-focus-update-steps.md`). Its comment: seeding before the focus event "would make a focus-handler write spuriously fire `change` on blur" | `git show a7c80bd6` (the `seed_focus_snapshot` doc and the inline comment at the seed) |

#### §0.2.1 B-P10: writers of the FOCUS bit, by property

Command (run 2026-11-22):
`git grep -n -P '&mut ElementState|insert_one\([^;]*ElementState|ElementState::(FOCUS|from_bits|all\b)|\(ElementState' -- 'crates/*.rs' ':!*tests*' ':!*test_support*'`,
then each hit outside a `#[cfg(test)]` module is classified by whether the value it writes can carry FOCUS.
Control: the same command lists `crates/dom/elidex-dom-api/src/focus/sot.rs:125` (the known setter). Result (R1's,
re-run): the only site that sets FOCUS is `sot.rs` `set_focus_bit` (:125, `s.insert(ElementState::FOCUS)`);
`crates/core/elidex-ecs/src/dom/tree/mutation.rs` (:635) only removes it; clone does not copy `ElementState`
(`tree_clone.rs`, "deliberate non-copy" row; `elidex-form` `clone.rs` re-sets only CHECKED / INDETERMINATE); every
other writer inserts a fresh `ElementState::default()` or sets a non-FOCUS bit. Production callers of
`set_focus_bit(.., Some(..))`: `update_steps.rs:303` and `html_element_proto.rs:305` (the other hits are the definition,
two `None` clears and `update_steps.rs`'s `mod tests`).

### §0.3 Premise errors and findings

The brief's premises held at rev 1 (each re-verified above). Findings it did not state:

- **F-1 (`sanitize_color`, beyond the comment)** [history of rev 1; decided by PM 932]. Spec text (`.claude/tools/webref
  body html update-a-color-well-control-color`, re-derivation Q16), step 2: "1. If element's dirty value flag is false,
  then return the result of getting an attribute by namespace and local name given null, "value", and element. 2.
  Return element's value." The comment states it backwards, and its supporting argument falls with it: the value IDL
  setter sets the dirty flag before it sanitizes (QB-6), so under the correct reading `input.value = "#ff0000"` reads
  the live value and round-trips. The code (`sanitize_value`'s Color arm, live value always) still differs from step 2
  wherever the dirty flag is false and the live value differs from the content attribute: `<input type=number
  value=red>` (Number sanitization: value `""`, dirty false); script `type = "color"`: the spec reads the attribute
  `"red"` → `"#ff0000"`; elidex reads `""` → `"#000000"`. Rev 1 offered (A) comment only or (B) also a behaviour fix
  reading `FormControlState::default_value`; PM 931 approved (B) conditional on Q-S3, Q-S3 failed (§8.3), and PM 932
  replaced it with (A) plus a new slot (LR-B2). DC-19 is unaffected (the user's edit set the dirty flag, Q17, so both
  readings use the live value).
- **F-2 (the predicate's input).** `from_type_str` maps `"image"` to `TextInput` (the Image Button state is missing),
  `"select-one"` to `Select` and `"textarea"` to `TextArea` for an `<input>`. HTML: "The attribute's missing value default
  and invalid value default are both the Text state" (QB-3), and Image Button is a state to which "The input and change
  events do not apply" (QB-1). So for `<input type=image>` the step-2.1 predicate of B-3 receives the wrong kind and
  would answer "applies"; today the same element is already seeded and edited as a text control (it is a `TextInput`
  to every reader). Pre-existing, not caused or widened by either slice; no DC cell depends on it. Observation O-B1
  (§9, LR-B3). Ledger check: `/usr/bin/grep -c -i 'type=image' <slot ledger>` → 0 (control:
  `core-browser-parity-deviations` → 4 hits), run 2026-11-22.
- **F-3 (a test the re-derivation's §6.1 did not list).** B-P13's assertion inverts under R-1; PR-β flips it (DC-10u).
- **F-4 (stale names).** The test names `writer_rebaselines`, `c21_…_rebaselines` to `c26_…_rebaselines` and
  `c7l_first_mark_rebaselines_once` name H's model ("the first mark re-baselines"); their outcomes hold under D-3 but the
  model they name is gone. PR-β renames them (OQ-3, PM 931).
- **F-5 (B-5 reverses a converged fix).** Seeding at designation reverses #357's R2 fix (B-P18). R2's failure mode —
  a `focus`-listener write with no user edit fires `change` — is prevented after PR-β by B-4's user-mark conjunct
  (I-B5): an unmarked component never fires. PR-β pins it with DC-10n (red under MG-B1).
- **F-6 (a stale reason in `value_mode.rs`).** The type-change step-2 comment in `elidex-form` `value_mode.rs` reads the
  content attribute from `Attributes` "NOT `FormControlState::default_value`" because "the buffered-mutation flush path
  (`SessionCore::flush` → `apply_set_attribute`) writes `Attributes` directly without running that reconciler". Since
  #428 `apply_set_attribute` calls `dom.set_attribute`, which dispatches, so that reason is stale. The conclusion stands on
  Q-S3's reasons (1)–(3) (§8.3). Recorded on the new slot (LR-B2); neither PR here edits `value_mode.rs`.

### §0.4 Decisions

Each names the rung that decides it, and the PR that carries it. Decisions of the slot row (requirements 1–5, the
Population, D-3, D-4, R-1, R-2) are not re-decided; B-1 to B-11 are how the code PRs meet them. Numbered B-n so they do
not collide with the re-derivation's D-n.

- **B-0 Two slices, PR-α then PR-β** — PM 933 (OQ-1). §1.
- **B-1 The component** (PR-β) — [slot req 1; spec Q1 (F); D-3]. `FocusValueSnapshot` becomes two private fields:
  `first_focus: String` (the value at first focus, `F0`, R-1 / R-3) and `user_change_from: Option<String>` (the mark:
  `Some(value just before the first user value change)`, D-3's K-first reference). No third fact (D-3 adds none, the slot
  row). The accessor `baseline()` is replaced by `first_focus()` and `user_change_from()` (NEW), read by
  `test_support::snap` only. PR-α keeps the shape `FocusValueSnapshot(Option<String>)`.
- **B-2 The population test is the element** (PR-α) — [slot Population; spec Q1 "If entry is an input element"; D-4].
  `record_focus_snapshot` writes iff the entity passes `EcsDom::is_html_namespace` (the canonical gate, B-P17) and its
  `TagType` is `input` or `textarea`, and it has `FormControlState` (the value's source). Never from
  `FormControlState.kind` (B-P11 shows the kind cannot express it). `focus_snapshot.rs` adds no HTML-namespace gate of
  its own (no second `namespace_of(..) == Namespace::Html`). Any other element gets no write — no insert and no remove.
  The same test is the only population gate: the take reads only a component this seed wrote, and the seed is the only
  inserter (§4.7).
- **B-3 One predicate over the kind at focus loss, private to the judgement** (PR-α) — [spec Q1's conjuncts "the change
  event applies to the element, and the element does not have a defined activation behavior", under R-2; D-4 for
  `TextArea`; One issue, one way; umbrella X1]. `change_on_blur_kind_applies(kind)` (NEW, private fn in
  `focus_snapshot.rs`), an exhaustive `match` with no wildcard arm, so a new `FormControlKind` must be decided there. Its
  one caller is `take_change_on_blur`. Truth table, derivation and doc requirement: §4.4. Not a `FormControlKind` method
  in `elidex-form-core`: a public classifier would invite a second caller deciding `change` elsewhere.
- **B-4 The judgement** (PR-α adds the kind conjunct; PR-β the comparison) — [spec Q1 (F); D-3]. After PR-α:
  `take_change_on_blur` removes the component, then returns `change_on_blur_kind_applies(kind at focus loss) &&
  Some(b) && V != b` (H's comparison). After PR-β: `change_on_blur_kind_applies(kind at focus loss) &&
  user_change_from == Some(p) && V != first_focus && V != p`. `false` without a component or without
  `FormControlState`.
- **B-5 The seed instant** (PR-β) — [slot req 2; R-1; reverses #357 R2, F-5]. In `focus_update_steps` the sink's
  `seed_focus_snapshot(new)` moves to immediately after step 4.1's `set_focus_bit(sink.dom(), Some(new))` and before
  `fire_focus_event(new, Focus, old)`. The "still focused" guard is deleted: nothing runs between the designation and the
  seed. R2's failure mode is closed by B-4's mark conjunct (I-B5), pinned by DC-10n.
- **B-6 VM `focus()`** (PR-β) — [slot req 3, 4; Layering mandate]. It keeps calling the one seed rule,
  `elidex_form::record_focus_snapshot`, immediately after its `set_focus_bit`; it fires no listener, so the instant is
  already R-1's. Its own redundant-focus return (B-P9) stays: it is the VM's copy of focusing steps step 5 until A2c
  routes VM `focus()` through `focusing_steps` (LR-B4). Only its comment changes (§4.6).
- **B-7 No write chooses by kind** (PR-α) — [slot req 5; 後方互換性は維持しない]. Deleted: the `is_text_control` test and the
  remove branch in `record_focus_snapshot`; `clear_focus_snapshot`, its re-export in `elidex-form`'s `lib.rs` and its
  call and import in the reconciler's `type` arm.
- **B-8 The seed rule's home is `elidex-form`** (both) — [slot req 3; Layering mandate; crate graph]. The slot allows
  `elidex-form` or `elidex-dom-api`; `elidex-dom-api` has no `elidex-form` dependency (the `FocusEventSink` trait doc:
  "this crate has no `elidex-form` dependency") and cannot read `FormControlState`. So the rule (population, the seeded
  value, unmarked) is `elidex_form::record_focus_snapshot`; the instant is the caller's (the transition, VM `focus()`
  after its designation). The shell sink and the VM host only call it.
- **B-9 Lifetime** (both) — [ECS-native first; B-P10]. §4.7. The component is written only by the seed (insert,
  overwriting), the mark (in place) and the take (remove); despawn removes it. A component left by VM `blur()`, VM
  `focus()` away, or removal (§2.1.4, no `change`) is never read: the only read is the take at the holder's focus loss.
  After PR-β, every designation of a Population element re-seeds first (B-P10, B-5). In PR-α's state the seed still
  runs after the `focus` / `focusin` listeners behind the "still focused" guard, so that sentence does not hold there;
  the argument for PR-α's state is §1.1.1.
- **B-10 `sanitize_color`: comment only** (PR-α) — PM 932 (OQ-2). The "Dirty-value-flag source" paragraph states step 2.1
  correctly (dirty value flag false → the `value` content attribute; else the live value), then states the remaining
  difference as a defect with slot `#11-color-well-update-reads-live-value` (LR-B2), by its property (flag false, live
  value ≠ content attribute) and F-1's example, naming no engine. The interop ground quoted in B-P15 and the
  re-evaluation sentence are removed; allowlist row 43 is pruned. The `sanitize_tests.rs` comment at B-P16 is regrounded
  on QB-6 (the setter sets the dirty flag before sanitizing, so step 2.2 reads the live value). History: rev 1's
  option (B) (read `FormControlState::default_value` in the Color arm) was dropped because `default_value` is not an
  invariant mirror of the attribute (Q-S3, §8.3). The ledger already records this comment as facet (g) of
  `#11-core-browser-parity-deviations` (added at #656, PM 909; owner L3; it names the `sanitize.rs` interop line and
  allowlist row 43, "Fix with the slot's code PR"). At PR-α's landing the PM closes facet (g) and re-points it to
  `#11-color-well-update-reads-live-value`, so the defect has one home (owner Layout) (LR-B7).
- **B-11 Test renames** (PR-β) — OQ-3, PM 931 ("OQ-1、OQ-3 は報告のとおりでかまいません"): rename (F-4; a name is a
  claim).

### §0.5 OO idioms translated (ECS-native first)

| Pattern a class-based engine uses [contrast] | elidex (ECS) |
|---|---|
| a text-control object remembers its value at focus in a member field | `FocusValueSnapshot` component on the control's entity; despawn cleans it |
| a per-class virtual "has activation behavior / does change apply" | one exhaustive `match` over `FormControlKind` (static dispatch, the Plugin-first built-in form), read once at focus loss |
| a focus observer / listener that captures the value | no observer: the canonical transition calls the sink's seed at a defined step (4.1); the one rule lives in `elidex-form` |
| an "is text control" subclass test at each write | the population is the element (`input` / `textarea`), fixed for the entity's lifetime; the kind is read only at focus loss |

## §1. Shape and decision: two per-PR slices (PM 933)

Edge-dense (≥3 intersecting axes: seed instant × population × `type` change × non-user writes × user edits × the VM
`focus()` path; §5), so plan-review is mandatory.

**Decision (PM 933, OQ-1): two per-PR slices, one memo.** PR-α = DF-2 (B-2, B-3, B-7, B-4's kind conjunct) + the
`sanitize.rs` / `sanitize_tests.rs` comment work (B-10) + allowlist row 43 pruned. PR-β = DF-1 (B-1, B-4's comparison,
B-5, B-6, B-11), landing after PR-α. Ground: CLAUDE.md *Edge-dense work* is a rule, not a judgement: a 6-axis unit with
two defects is not a narrowly-scoped per-PR slice (precedent #339, a single PR whose review tail outgrew its
implementation).

| Principle | One slice | Two slices (PR-α, then PR-β) |
|---|---|---|
| Ground order | neutral: both defects are [spec] (DF-2 under R-2) | neutral |
| ECS-native first | neutral: which entities carry the component (the population) and which fields it has (the shape) are separate questions; neither PR adds a side-store, registry or observer | neutral |
| One issue, one way | neutral: rewriting the same functions in two PRs in sequence leaves no two coexisting mechanisms at any main (§1.1) | neutral |
| Edge-dense work | rejects | **selects** (the rule) |
| Ideal over pragmatic, Supported-surface testing, 設計優先, Plugin-first, others | not engaged | not engaged |

### §1.1 The main between PR-α and PR-β is coherent

After PR-α lands, every HTML `input` and `textarea` element is seeded at each designation with the old component shape
`FocusValueSnapshot(None)` (B-2), still after the `focus` / `focusin` listeners (the seed instant is PR-β's); no write
chooses by kind and a `type` change writes nothing to the component (B-7); `take_change_on_blur` evaluates the new kind
predicate at focus loss and then H's unchanged comparison (`Some(b) && V != b`, B-4). The removal branch's old reason
(a stale text baseline surviving a `type` change and firing a spurious `change`) is covered: every designation of an
`input` overwrites the component, and a kind without step 2.1 is judged `false` at focus loss. One mechanism holds the
population, one the predicate, one the comparison; nothing is half-migrated. Cells in that state: green — DC-14,
DC-14u, DC-15, DC-16, DC-16u, DC-18, DC-19, PB-1α to PB-8, SC-1, and every control (DC-2 to DC-7, DC-9, DC-11 as
`c7l_*`, DC-12, DC-13, DC-17, E17); red — DC-1, DC-1v, DC-8, DC-10, DC-10u, DC-10r, E17v, PB-1β (DF-1's, not added
until PR-β). PR-α adds none of the red ones.

#### §1.1.1 Why a stale component is unreachable in PR-α's state

In PR-α's state the seed still runs after the `focus` / `focusin` listeners, behind the "`new` is still focused" guard
(B-P6), so "every designation re-seeds first" (B-9, I-B2, §4.7) does not hold yet. During the listener window, `new`
carries whatever component an earlier focus session left (or none), and if a listener moves focus away, the guard skips
the seed. The argument that such a component is never read:

- The only read is the take, and only a canonical transition (`focusing_steps`, `unfocusing_steps`,
  `focus_fixup_steps`) runs it. Every production caller of the shell's `set_focus` / `blur_current` / `focus_fixup` and of
  `focusing_steps` / `focus_fixup_steps` is a UA path: the shell's input handlers (`content/event_handlers.rs`,
  `app/events.rs`, `content/form_input.rs`, `content/iframe/thread.rs`), the shell's wrappers themselves
  (`content/focus.rs`), and the rendering step-17 fixup (`lib.rs`, the `focus_fixup` call in the `re_render` path).
  Command (run 2026-11-22 at `47cb4834`, the same code as `1f81fdc2`):
  `git grep -n -P '\b(set_focus|blur_current|focus_fixup|focusing_steps|unfocusing_steps|focus_fixup_steps|take_change_on_blur)\(' -- 'crates/*.rs' ':!*tests*' ':!*test_support*'`.
  The known-hit control is `app/events.rs:44` (`crate::content::focus::set_focus`). The remaining hits are definitions,
  `update_steps.rs`'s and `focus_snapshot.rs`'s in-file `mod tests`, and the one production take, the shell sink's
  `dispatch_change_on_blur` (`content/focus.rs`); `crates/script/elidex-js` has no call to any of them. A script can
  reach focus only through VM `focus()` (bit plus seed, no take) and VM `blur()` (no take). So no take runs inside a
  listener window.
- A take reads `new`'s component only while `new` is the holder. When `new` is the holder, its latest designation
  either completed its seed or came from VM `focus()`, which seeds right after its `set_focus_bit`. A canonical
  designation whose seed the guard skipped left `new` not focused. `new` becomes the holder again only through a later
  designation, which seeds or, if its own seed is skipped, again leaves `new` not focused.

PR-α widens what such a stale component can be. Without the `type`-change clear (B-7), a component can survive on an
`input` of any kind. Reachability is unchanged, and a kind without step 2.1 is judged `false` at focus loss (B-3). PR-β
replaces this argument with B-5's: nothing runs between the designation and the seed.

## §2. Coupled invariants

Each names the PR that establishes it.

- **I-B1 One judgement, one predicate** (PR-α; umbrella X1 × B-3 × B-4): whether `change` fires at step 2.1 is computed
  only in `take_change_on_blur`; the kind conjuncts only in `change_on_blur_kind_applies`; every sink only dispatches.
- **I-B2 Every designation seeds** (both; B-P10 × B-5 × B-6 × B-9): each production writer of `set_focus_bit(Some(e))`
  calls `record_focus_snapshot(e)`; after PR-β, before anything else can run. This is what makes a stale component
  unreachable (§4.7); in PR-α's state the seed runs after the listeners, and the argument is §1.1.1. A new designation
  writer must seed; each PR's DoD re-runs §0.2.1.
- **I-B3 One population, two sites** (PR-α; B-2 × B-4): the seed's element test is the only population test; the take
  has no population test of its own (it reads what the seed wrote). No write and no read tests an `input`'s kind for
  population.
- **I-B4 Seed before listeners** (PR-β; B-5 × R-1 × DC-10): a `focus` / `focusin` listener's write is a non-user write
  after `F0`. Reentrancy: a `focusin` listener that moves focus on finds `new` already seeded; a canonical move takes it
  (no mark, so no `change`); a VM `focus()` leaves it (B-9).
- **I-B5 The mark is the funnel's** (both; plan (1) I1 / I2 × B-1): the mark is set only by `note_user_value_change`
  from `apply_user_edit`; non-user writers call nothing (plan (1) D-4; Q5). After PR-β it is also what keeps #357 R2's
  case closed (F-5, DC-10n).
- **I-B6 Redundant focus does not re-seed** (both; E-17 × B-P7 × B-6): both designation paths return before the seed.
- **I-B7 `type` changes write nothing to the component** (PR-α; B-7 × DF-2 × the type-change steps 1–3 and 6): the value
  they write is a non-user write, judged at focus loss under D-3 (DC-19) or [spec] (DC-18).
- **I-B8 Unchanged pairs** (both): umbrella X3 / X4 / X5 (fixup listeners never mark; IME commit through the funnel) and
  plan (2)'s fixup path to the same sink (its C6 / C19, the consumed-component assertions in
  `content_focus_fixup_tests.rs`) are not affected: the fixup never designates an element, so B-5 does not reach it.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §6.6.4 Processing model | *focus update steps* step 2.1 | all four conjuncts at focus loss (PR-α: the kind conjuncts); the reference value "when the control was first focused" (R-1, PR-β); D-3's completion | `take_change_on_blur`, `change_on_blur_kind_applies` (NEW); DC-1 to DC-19 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focus update steps* step 4.1, before step 4.4 | the seed at designation, before `focus` (PR-β) | `focus_update_steps`; DC-10 | ✓ | yes |
| WHATWG HTML §6.6.4 Processing model | *focusing steps* step 5 | redundant focus returns | `focusing_steps`; VM `focus()` early return; E-17 cells | ✓ | yes |
| WHATWG HTML §4.10.5.1 States of the type attribute | each state's "The input and change events (do not) apply" | B-3's table | `change_on_blur_kind_applies` (NEW) | ✓ | no |
| WHATWG HTML §4.10.5 The input element | activation behavior (R-2); *update an input element's type* steps 1–3, 6; the `type` attribute's missing / invalid value default is the Text state (QB-3) | the `type` arm writes no component (PR-α); F-2 | reconciler `type` arm; DC-14 to DC-16, DC-18, DC-19, PB-5 to PB-7; `from_type_str` (O-B1, LR-B3) | ✗ | yes |
| WHATWG HTML §4.10.5.1.14 Color state (type=color) | *update a color well control color* step 2 | dirty → live value; not dirty → content attribute | `sanitize_color` doc (PR-α, B-10); LR-B2 | ✗ | no |
| WHATWG HTML §4.10.5.4 Common input element APIs | `value` setter, mode value, steps 3–4 | dirty flag set before sanitization | B-10's test comment; F-1 | ✗ | no |
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
| QB-3 | `the-input-element`, §4.10.5 The input element (the `type` attribute; before §4.10.5.1) | "The attribute's missing value default and invalid value default are both the Text state." |
| QB-4 | `focus-update-steps` step 4.1 | "If entry is a focusable area, and the focused area of the document is not entry: 1. Set document's relevant global object's navigation API's focus changed during ongoing navigation to true. 2. Designate entry as the focused area of the document." (step 4.4 fires `focus`) |
| QB-5 | `focusing-steps` step 5 | "If new focus target is the currently focused area of a top-level traversable, then return." |
| QB-6 | `dom-input-value` (mode value, on setting) | "1. Let oldValue be the element's value. 2. Set the element's value to the new value. 3. Set the element's dirty value flag to true. 4. Invoke the value sanitization algorithm, if the element's type attribute's current state defines one." |

### §3.2 User-input touch audit

No user-input path is added or changed by either PR. The funnel (`apply_user_edit`) and its four shell callers are as
landed (plan (1) §3.1); only what the mark writes into is re-shaped (B-1, PR-β). The key router still reads the live kind
at each `keydown` (re-derivation P15), which DF-2 (i) depends on.

## §4. Mechanism — `1f81fdc2` → PR-α → PR-β

### §4.1 The component (B-1, PR-β)

`FocusValueSnapshot { first_focus: String, user_change_from: Option<String> }`, private fields, written only in
`focus_snapshot.rs`. Accessors `first_focus()` / `user_change_from()` (NEW) for `test_support::snap`. The clone policy
stays "deliberate non-copy" (`tree_clone.rs`'s table; only its parenthetical text changes, §6.2). PR-α leaves the shape
as is.

### §4.2 The seed (B-2 in PR-α; B-5, B-6 in PR-β; B-8)

PR-α: `record_focus_snapshot(dom, entity)`: if `dom.is_html_namespace(entity)`, its `TagType` is `input` or `textarea`,
and it has `FormControlState`, insert `FocusValueSnapshot(None)` (overwriting any earlier one). Else nothing (no
remove). The call sites and instant are unchanged.

PR-β: the insert becomes `FocusValueSnapshot { first_focus: value.clone(), user_change_from: None }`. Callers: the
shell sink's `seed_focus_snapshot` (from `focus_update_steps` at step 4.1, B-5) and VM `focus()` (after its designation,
B-6). The doc of the `seed_focus_snapshot` method of the `FocusEventSink` trait states the instant (step 4.1, before step
4.4) and drops "Seed timing does not decide whether a non-user write fires `change`" (DC-10 contradicts it).

### §4.3 The mark

`note_user_value_change(dom, entity, before)`: `get_or_insert(before)` on an existing component's mark (PR-β: the
`user_change_from` field). Behaviour unchanged in both PRs; its doc is rewritten in PR-β (§6.2).

### §4.4 The judgement and the predicate (B-3, B-4)

`change_on_blur_kind_applies(kind)` (NEW, PR-α), derived from QB-1, QB-2 and R-2. Of the states to which `change`
applies, Checkbox, Radio Button, File Upload and Color define an input activation behavior (Submit Button, Image Button
and Reset Button also define one, but `change` does not apply to them; Button has none):

| Kind (state) | `change` applies (QB-1 / QB-2) | defined input activation behavior (R-2) | Result |
|---|---|---|---|
| `TextInput`, `Search`, `Tel`, `Url`, `Email`, `Password`, `Date`, `Month`, `Week`, `Time`, `DatetimeLocal`, `Number`, `Range` | yes | no | **true** |
| `TextArea` (the `<textarea>` element; also `<input type=textarea>`, F-2, Text state by QB-3, same answer) | spec silent (re-derivation PE-2) | — | **true** (D-4) |
| `Checkbox`, `Radio`, `File`, `Color` | yes | yes | false |
| `Hidden`, `SubmitButton`, `ResetButton`, `Button` | no | — | false |
| `Select`, `Output`, `Meter`, `Progress` | not an `input` state; on an `input` only through F-2's mapping (`"select-one"`) | — | false (the mapping is O-B1's) |

**Doc requirement for `change_on_blur_kind_applies`** (PR-α writes it; review checks it): the fn doc cites HTML §6.6.4
*focus update steps* step 2.1's conjuncts "the change event applies to the element, and the element does not have a
defined activation behavior"; each arm's doc (or a per-group line) cites the §4.10.5.1.x state's "The input and change
events apply" / "do not apply" (QB-2 / QB-1) and, for the activation half, R-2 (re-derivation §0.5); the `TextArea` arm
cites D-4 (the spec is silent for `<textarea>`); the `Select` / `Output` / `Meter` / `Progress` arm cites O-B1 (reached
on an `input` only through `from_type_str`'s mapping, the slot of LR-B3). No arm names an engine or a measurement.

`take_change_on_blur`: B-4. The mark test is "the user has changed the element's value … while the control was focused"
(plan (1) D-1's funnel); after PR-β, `V != first_focus` is Q1 (F) and `V != user_change_from` is D-3's K-first. Step
2.1's "or its list of selected files" has no element under R-2 (the re-derivation §0.5); B-3's `File` arm is false.

### §4.5 `type` change (B-7, PR-α)

The reconciler's `type` arm keeps steps 1–3 and 6 and loses the clear. The comment block above the clear is deleted
with it.

### §4.6 VM `focus()` (B-6, PR-β; one comment phrase in PR-α)

PR-α: the inline comment at the seed says "for text controls", which names the population wrongly once B-2 lands; PR-α
replaces that phrase with a population-neutral one ("Seed the change-on-blur snapshot (the one seed rule,
`elidex_form::record_focus_snapshot`, which decides the population)"), and the doc's "for a user-edited text control —
the §4.10.5.5 change-on-blur event" becomes "the change-on-blur event (whose verdict is `take_change_on_blur`'s)". PR-α
changes no code in this file.

PR-β: code unchanged but for the call it already makes; the comment "Seed the change-on-blur snapshot for text controls,
just like the shell `set_focus` reconciler" becomes "seed the value at first focus (the one seed rule,
`elidex_form::record_focus_snapshot`) at the designation, as the canonical transition does at focus update steps step
4.1". The doc paragraph on the old holder's component ("its user-edit baseline") is re-worded to "its first-focus value
and mark". The early-return inline comment is rewritten too (§6.2).

### §4.7 Lifetime (after PR-β; PR-α differs only in what the seed writes and in the seed's instant, and its reachability argument is §1.1.1)

| Event | Write | Reader consequence |
|---|---|---|
| designation (step 4.1; VM `focus()`) of a Population element | insert `{F0, None}`, overwriting | a stale component from an earlier session is replaced (I-B2) |
| designation of any other element | none | never carries one |
| redundant focus (QB-5; VM early return) | none | `F0` and the mark survive (E-17) |
| a `focus` / `focusin` listener writes the value | none | non-user write after `F0` (DC-10, DC-10n) |
| user value change | `user_change_from.get_or_insert(before)` | the mark |
| non-user write (setters, attributes, `type` change steps 1–3 and 6, reset, `setRangeText`) | none | judged at focus loss |
| `type` change | none (B-7) | kind read at focus loss (B-3) |
| focus loss through the canonical transition (UA move, `blur_current`, the step-17 fixup) | remove (take) | the verdict |
| VM `blur()`, VM `focus()` to another element (A2c), removal of the focused element (§2.1.4) | none | left in place, unreachable until the next designation re-seeds (B-9) |
| despawn | removed by the World | — |
| clone | not copied | — |

### §4.8 ECS-native and Layering checks

- One component on the entity; no side-store, no registry, no observer; one component get per transition.
- The population is read from the entity's own components (`TagType`, the namespace through the canonical
  `EcsDom::is_html_namespace`), not from a list; `focus_snapshot.rs` adds no second namespace gate.
- `vm/host` only calls `record_focus_snapshot`; it computes no population and no predicate (Layering mandate).
- `elidex-dom-api` owns the instant, `elidex-form` the rule and the judgement; no new cargo edge.

## §5. Edge matrix and test plan

Axes: **S** seed instant (before listeners / redundant / VM), **P** population (input by kind class / textarea / other
`FormControlState` elements), **T** `type` change while focused (none / text→text / to an excluded kind / to a value-mode
non-text kind / round trip / non-text→text), **W** non-user writes (none / before the first edit / after it / in a
`focus` listener / a sanitization in the round trip), **U** user edits (none / net zero / back to `F0` / net change),
**F** path (UA sink / `blur_current` / fixup / VM `focus()` seed). PR-α covers P, T and the kind half of the
judgement; PR-β covers S, W × U and F.

"Base" = expected result at that PR's base with that PR's fix absent (red = fails as the defect predicts): PR-α's base
is `1f81fdc2`; PR-β's base is PR-α's merge (§1.1). Shell cells are in `content_change_on_blur_tests.rs` and use its
`setup` / `pair` / `ua_focus` / `user_key` / `script_set_value` helpers ("UA focus" = `set_focus`, the canonical
transition; the cells' "by Tab" is the same transition). Unit cells are engine-independent: `elidex-form`
(`focus_snapshot.rs`, `user_edit.rs` and `reconciler_tests.rs` test modules), `elidex-form-core` (`sanitize_tests.rs`),
`elidex-dom-api` (`update_steps.rs` test module), `elidex-js` VM (`tests_html_element_proto.rs`). Every cell is
green-able within its PR.

### §5.1 PR-α cells (base `1f81fdc2`)

| Cell | Axes | Test (NEW unless stated) | Sequence (re-derivation §6.2 unless stated) | Expected | Ground | Base |
|---|---|---|---|---|---|---|
| DC-14 | P checkbox at seed; T non-text→text | `dc14_checkbox_focused_then_text_then_user_edit` | §6.2 DC-14 | `input`, then `change` | [spec] Q1 under R-2; R-3 | red |
| DC-14u | same, unit | `reconciler_tests` `dc14u_seed_on_checkbox_survives_flip_to_text` | seed on `<input type=checkbox>`; `set_attribute("type","text")`; `apply_user_edit` inserts `a`; take | `true` | as DC-14 | red |
| DC-15 | T to Number after U | `dc15_text_to_number_after_user_edit` | §6.2 DC-15 | `change` | [spec] Q1 under R-2 | red |
| DC-16 | T to Checkbox after U | `dc16_text_to_checkbox_after_user_edit_no_change` | §6.2 DC-16 | no `change` | [spec] R-2 | green |
| DC-16u (e2c) | same, unit | `e2c_*` rewritten: the component survives the flip; take → `false` | — | component present after the flip; `false` | B-7; R-2 | red (component absent) |
| DC-18 | T round trip through Checkbox | `dc18_round_trip_through_checkbox` | §6.2 DC-18 | `change` | [spec] under R-2 | red |
| DC-19 | T round trip through Color | `dc19_round_trip_through_color` | §6.2 DC-19 | `change` | [spec] under R-2; D-3 | red |
| PB-1α | P every `input` kind | `focus_snapshot` `seed_writes_every_input_kind` (a loop over every `FormControlKind` an `input` can carry, on an `input` element) | seed | component present and unmarked (`Snap::Unmarked`, the old shape) | slot Population; B-2 | red (non-text kinds absent) |
| PB-2 | P textarea | `seed_writes_textarea` (a `textarea` element, `FormControlState` attached by production attach, §5.3) | seed | present | D-4 | green |
| PB-3 | P other `FormControlState` elements | `seed_writes_nothing_for_non_population` (`select`, `button`, `output`, `meter`, `progress` elements, each given `FormControlState` by production attach, `create_form_control_state`, asserted to return `true` (§5.3); one set with a component pre-inserted by the test, one without) | seed | no insert; a pre-existing component is untouched (no write at all) | slot Population ("get no write") | red for the untouched half (today the seed removes it) |
| PB-4 | predicate | `kind_predicate_truth_table` (every `FormControlKind`, §4.4) | — | §4.4's column | QB-1, QB-2, R-2, D-4 | n/a (new fn; does not compile at base) |
| PB-5 | T to Hidden after U | `reconciler_tests` `take_false_after_flip_to_hidden` | `<input value="x">` (value `"x"`); seed; `apply_user_edit` appends `a` (`"xa"`, mark `"x"`); `set_attribute("type","hidden")`: step 1 writes the content attribute `"xa"`, Hidden defines no sanitization, so `V` stays `"xa"`; take | `false` | QB-1 | green |
| PB-6 | T to File after U | `reconciler_tests` `take_false_after_flip_to_file` | as PB-5 to `"xa"`, mark `"x"`; `set_attribute("type","file")`: step 3 (filename mode) sets `V` to `""`, File defines no sanitization; take | `false` | R-2 | green |
| PB-7 | T to Color, still Color at focus loss | `reconciler_tests` `take_false_at_color` | as PB-5 to `"xa"`, mark `"x"`; `set_attribute("type","color")`: value mode to value mode, so steps 1–3 do nothing; step 6 sanitizes `"xa"` to `"#000000"`; take | `false` | R-2 | green |
| PB-8 | `no_snapshot_for_non_text_control` rewritten | `button_kind_input_is_seeded_and_judged_false` (`control` makes an `input`) | seed on `<input type=button>`; script value `x`; take | component present after the seed; take `false` | B-2, B-3 (QB-1) | red (absent) |
| SC-1 | `sanitize_color` round trip | existing `color_idl_value_set_round_trips_valid_color`, comment regrounded (B-10) | as today | as today | [spec] QB-6, step 2.2 | green |

Controls that stay green in PR-α (unchanged tests): DC-2 to DC-7, DC-9 (C20–C26, C24), DC-11 (`c7l_*` as is), DC-12
(C1, C3, C5, C9, C14, C16, C18, C19), DC-13 (`c4_*`, `c17_*`), DC-17, E17, the `sanitize_color_*` cells, plan (2)'s
fixup cells (I-B8).

In PB-5 to PB-7 and DC-16 the value at focus loss differs from the mark after the flip (PB-5 `"xa"`, PB-6 `""`, PB-7
`"#000000"`, each against `"x"`; DC-16: the user types `a` from `""`, step 1 writes the content attribute `"a"`, and
Checkbox defines no sanitization, so `V` = `"a"` against the mark `""`), so H's comparison alone gives `true`. The kind
predicate is the only thing that makes the result `false`, and MG-A3 turns each of them red. The type-change steps are
from `.claude/tools/webref body html the-input-element` (*update an input element's type*, steps 1–3 and 6), and the
code is `elidex-form` `value_mode.rs` `apply_type_change_value_migration` and the reconciler's `type` arm
(`sanitize_for_type_change`).

PR-α mutation guards (run on PR-α's final head only; each must turn the named cells red, then reverted):

| Guard | Mutation | Red cells |
|---|---|---|
| MG-A1 | the seed tests `is_text_control` again | DC-14, DC-14u, PB-1α, PB-8 |
| MG-A2 | the reconciler's `type` arm removes the component for non-text kinds | DC-15, DC-16u, DC-18, DC-19 |
| MG-A3 | `change_on_blur_kind_applies` returns `true` for every kind | PB-4, PB-5, PB-6, PB-7, DC-16 |
| MG-A4 | the seed writes every element with `FormControlState` | PB-3 |

### §5.2 PR-β cells (base = PR-α's merge)

| Cell | Axes | Test (NEW unless stated) | Sequence (re-derivation §6.1 unless stated) | Expected | Ground | Base |
|---|---|---|---|---|---|---|
| DC-1 (C6) | W before, U back to F0; both kinds | `c6_*` rewritten, renamed `dc1_script_write_then_user_revert_to_first_focus_value_{input,textarea}` | §6.1 DC-1 | no `change` | [spec] Q1 (F) | red |
| DC-1v | F VM, W before, U back to F0 | `dc1v_vm_focus_then_script_write_then_user_revert` | `b.focus()` (VM); script `b.value = "x"`; user Backspace (`""`); UA focus `c` | no `change` | [spec] Q1 (F); slot req 3 | red |
| DC-2 to DC-7, DC-9 (C20–C23, C25, C26, C24) | W before, U net zero | existing `c20_*`, `c21`–`c26` (renamed, F-4) | as tabled there | no `change` | D-3 | green |
| DC-8 | W before, U net change then back to F0; both kinds | `dc8_script_write_user_type_then_delete_to_first_focus_value_{input,textarea}` | §6.1 DC-8 | no `change` | [spec] Q1 (F) | red |
| DC-10 | S before listeners, W in `focus` listener | `dc10_focus_listener_write_then_user_revert` (a `focus` listener on `b` sets `b.value = "x"`) | §6.1 DC-10 | no `change` | [spec] Q1 (F); R-1 | red |
| DC-10n | S before listeners, W in `focus` listener, U none | `dc10n_focus_listener_write_without_user_edit_no_change` (a `focus` listener on `b` sets `b.value = "x"`; the user does nothing; UA focus `c`) | this memo (F-5) | no `change` | [spec] Q1 ("the user has changed"); §4.10.5.5; #357 R2's case | green |
| DC-10u | S | dom-api `focusing_steps_designates_last_and_fires_in_spec_order`, assertion flipped | `seeds_len_at_focus` | `Some(1)`; message cites R-1 | R-1 (QB-4) | red |
| DC-10r | S × reentrancy | dom-api `focusin_reentrant_focus_finds_new_seeded`; `TestSink` gains `reentrant_on_focus` (NEW) | focus `b`; the `Focus` event moves the bit to `c` | `seeds == [b]` | R-1: the seed belongs to the designation | red (the guard skips the seed) |
| DC-11 (C7 (l)) | W before, U net zero | `c7l_*` rewritten at the judgement level, renamed `dc11_net_zero_user_edit_after_script_write_is_false` | §6.1 DC-11 | `take_change_on_blur == false` | D-3 | green |
| DC-12 | controls | existing C1, C3, C5, C9, C14, C16, C18, C19 | — | as today | [spec] E-6, E-7, M8 | green |
| DC-13 | W after; U net change | existing `c4_*` (M1), `c17_*` (M6) | — | `change` | D-3 | green |
| DC-17 | W, U net zero, W; both kinds | `dc17_script_user_net_zero_script_{input,textarea}` | §6.1 DC-17 | `change` | D-3 | green |
| PB-1β | P every `input` kind × F0 | PB-1α's test, expectation extended | seed with a per-kind value | present, `first_focus()` = the value, `user_change_from()` = `None` | slot req 1; B-1 | red (no `first_focus` at base) |
| E17 | S redundant | existing VM redundant-`focus()` test (`tests_html_element_proto.rs`, its `take_change_on_blur` assertion) and the dom-api step-5 cells | — | as today | QB-5 | green |
| E17v | S redundant × F0 | `vm_redundant_focus_keeps_first_focus_value` | VM `focus()` (value `""`); script `b.value = "x"`; Rust `apply_user_edit` deletes the `x` (value `""`); VM `focus()` again; read the component, then take | `first_focus() == ""` and `user_change_from() == Some("x")` (a re-seed would leave it unmarked); take `false` (`V` = `F0`) | QB-5; Q1 (F) | red (no `first_focus` at base) |

Every PR-α cell stays green in PR-β (PB-1α is superseded by PB-1β in the same test).

PR-β mutation guards (run on PR-β's final head only):

| Guard | Mutation | Red cells |
|---|---|---|
| MG-B1 (plan (1) MG1 re-defined) | `take_change_on_blur` drops the `user_change_from` test (fires on `V != F0` alone) | C1, C3, C9, C14, C16 (DC-12), DC-10n |
| MG-B2 (plan (1) MG6 re-defined) | the comparison drops `V != user_change_from` (reading L) | DC-2 to DC-7, DC-9, DC-11 |
| MG-B3 | the comparison drops `V != first_focus` (H's predicate) | DC-1, DC-1v, DC-8, DC-10 |
| MG-B4 | the seed moves back after `fire_focus_event` | DC-10, DC-10u, DC-10r |
| MG-B9 | VM `focus()`'s early return removed | E17, E17v |

Coverage: engine-independent units in `elidex-form`, `elidex-form-core`, `elidex-dom-api` and the VM; shell integration
cells for the user-visible sequences. No WPT subset is claimed.

### §5.3 Existing tests that change (beyond the cells above)

PR-α:
- `record_clears_stale_snapshot_when_no_longer_text_control` (`focus_snapshot.rs`): deleted; it asserts the remove branch
  B-7 deletes. PB-1α / PB-8 cover the population.
- `e2d_type_flip_between_text_kinds_preserves_focus_snapshot`: assertion holds; its comment ("only a non-text
  destination clears the snapshot") is rewritten.
- `test_support`: PR-α does not touch `Snap` (the old shape serves PB-1α). It adds a fixture that takes a tag name
  (`textarea`, `select`, `button`, `output`, `meter`, `progress`) for PB-2 / PB-3 and attaches `FormControlState`
  through the production single-entity attach `elidex_form::create_form_control_state` (asserting `true`), as parse,
  insert and `createElement` do. An element from `create_element_ownerless_fixture` alone carries no
  `FormControlState` (it runs no attach). The seed's `FormControlState` conjunct, not its tag conjunct, would then
  exclude PB-3's elements, and MG-A4 could not turn PB-3 red. `control` (a hand-built state, inserted by the test) keeps
  making an `input`.

PR-β:
- `test_support::Snap` / `snap`: re-defined on B-1 (`Absent`, `Unmarked { first_focus }`, `Marked { first_focus,
  user_change_from }`).
- `focus_snapshot.rs` `c7a_seed_is_unmarked`, `c7b`, `c7d`, `c7j`, `c7k`: assertions move to the new `Snap`.
- `user_edit.rs` test module (`use crate::test_support::{control, snap, Snap}`; the `marked` helper's
  `Snap::Baseline(_)`, used by `c7e`–`c7h`): the helper matches `Snap::Marked { .. }`; the cells' outcomes are unchanged.

## §6. Comments, names and allowlist rows the code PRs rewrite

Each listed site is rewritten with the requirement that replaces it; no rewritten comment names a measurement or an
engine as a ground. A comment that states the verdict refers to `take_change_on_blur` rather than restating its
comparison, so PR-β does not rewrite PR-α's comments.

### §6.1 PR-α

- **Allowlist row pruned** (`.claude/tools/ecs-mission-allowlist.tsv`, at `1f81fdc2`): the row at line 43 (T,
  `#11-core-browser-parity-deviations`) — `sanitize_color`'s doc. `python3 .claude/tools/ecs_mission_ratchet.py
  --prune`, then the ratchet → `ok (294 allowlisted, 0 new, 0 stale)`.
- **`sanitize.rs`**: the "Dirty-value-flag source" paragraph (B-10): step 2.1 stated as the spec does (Q16), the
  setter's order (QB-6), the remaining difference as a defect with `#11-color-well-update-reads-live-value` (property
  and F-1's example), no engine named; the interop ground (B-P15) and the re-evaluation sentence removed.
- **`sanitize_tests.rs`**: `color_idl_value_set_round_trips_valid_color`'s "B2 (interop-faithful)" comment → the
  setter sets the dirty value flag before sanitizing (QB-6), so step 2.2 reads the live value.
- **`focus_snapshot.rs`**: the module doc's `<textarea>` rule as measured behaviour → D-4 and the Population;
  `FocusValueSnapshot`'s "Only text controls carry it" → B-2; `record_focus_snapshot`'s "**if** it is a text control"
  and the removal paragraph → B-2, B-7; `take_change_on_blur`'s "not a tracked text control" → B-3, B-4;
  `clear_focus_snapshot` deleted with its doc; `change_on_blur_kind_applies`'s doc per §4.4.
- **`reconciler.rs`**: the comment block of the deleted clear; the `type` arm's opening comment keeps its spec text.
- **`update_steps.rs`**: `commit_change_on_blur`'s doc ("a focused text control's value was user-edited since focus" →
  "the Population element's verdict from `take_change_on_blur`").
- **`content/focus.rs`**: the module doc ("and change-on-blur for text controls" → "and the change-on-blur dispatch of
  `take_change_on_blur`'s verdict"); `dispatch_change_on_blur`'s doc ("on text-control blur" → the Population and the
  verdict).
- **`html_element_proto.rs`** (comment only, §4.6): the seed's inline comment's "for text controls"; the doc's "for a
  user-edited text control — the §4.10.5.5 change-on-blur event".
- **`content_change_on_blur_tests.rs`**: the module doc line "`change` fires at a blurred text control" → "at a blurred
  `input` / `textarea` element" (the rest of the module doc is PR-β's, §6.2).
- **`reconciler_tests.rs`**: `e2c_*`'s comment (rewritten with the cell, DC-16u) and `e2d_*`'s ("keeps the control a
  text control … only a non-text destination clears the snapshot", §5.3).
- **`focus_snapshot.rs` test module**: `record_clears_stale_snapshot_when_no_longer_text_control` (deleted) and
  `no_snapshot_for_non_text_control` (rewritten as PB-8) take their "non-text control" comments with them.
- **Umbrella** (`docs/plans/2026-11-focus-fixup-program.md`): LR-B6's PR-α part.

**"text control" sweep.** `/usr/bin/grep -n -i 'text control\|text-control'` over every file either PR touches (§7)
(run 2026-11-22 at `47cb4834`) gives the hits in `focus_snapshot.rs` (:5, :25, :43, :48, :99; tests :225–:253),
`reconciler_tests.rs` (:213, :237), `update_steps.rs` (:77), `content/focus.rs` (:2, :188, :232),
`content_change_on_blur_tests.rs` (:3, :326), `user_edit.rs` (:33) and `html_element_proto.rs` (:262, :306). The control
for the command is `focus_snapshot.rs:25` ("Only text controls"). Each population claim is listed above under PR-α,
where B-2 makes it stale. Three hits stay, because they are about `is_text_control`'s own meaning or are not claims:

- `content/focus.rs:232`, the `text_input` test helper: a default `<input>` is a `TextInput`.
- `user_edit.rs:33`: the funnel's callers are the paths by which the user edits a text-entry value, and the key router
  gates them on `is_text_control`.
- `content_change_on_blur_tests.rs:326`: a slot ID.

### §6.2 PR-β

- **Allowlist rows pruned**: the row at line 44 (T, this slot) — the doc of `note_user_value_change`; the row at line
  141 (E, slot `-`) — the doc of `focus_with_caret_at_end`. Then the ratchet → `ok (292 allowlisted, 0 new, 0 stale)`.
- **`focus_snapshot.rs`**: the module doc's "holds one thing … `None` until the first user value change" → B-1;
  `FocusValueSnapshot`'s "No focus-time copy of the value is kept … PM 393" → B-1; `record_focus_snapshot`'s "after
  §6.6.4 focus update steps step 4.4" → B-5; `note_user_value_change`'s doc (the measured model as ground → "the mark
  records the value just before the first user value change: D-3's K-first reference").
- **`content_change_on_blur_tests.rs`**: module doc ("model H of §4.3" → this memo and the re-derivation); the
  "Commit (3): the mark decides, and the first mark re-baselines" banner; `focus_with_caret_at_end`'s doc (its setup
  stated by what it does: a script write before focus leaves the caret at the end); C6's doc (now DC-1); C24's doc (W6
  as PM 307's accepted divergence → "D-3, class (b); DC-9"); `writer_rebaselines` → `writer_then_net_zero_user_edit`,
  and `c21`–`c26`'s `_rebaselines` suffix → `_net_zero_user_edit` (F-4, B-11).
- **`update_steps.rs`**: the `seed_focus_snapshot` trait doc and the inline comment at the seed (B-5, §4.2; the #357 R2
  reasoning is replaced by R-1 and I-B5); the step-5 comment in `focusing_steps` ("must not refresh the snapshot
  baseline" → "must not re-seed the first-focus value").
- **`content/focus.rs`**: `seed_focus_snapshot`'s comment ("After §6.6.4 focus update steps step 4.4"; "the baseline is
  the value before the first user value change, plan D-1").
- **`html_element_proto.rs`**: §4.6. Three more pieces:
  - The whole seed comment.
  - The doc's "its user-edit baseline".
  - The early-return inline comment. "would otherwise refresh the snapshot baseline to the edited value" becomes "a
    re-seed would replace the first-focus value and drop the mark". "Mirrors the shell `set_focus` reconciler's
    `old == Some(entity)` early return" was already stale at `1f81fdc2`: shell `set_focus` routes through
    `focusing_steps`, whose step-5 return is the `old == Some(target)` (B-P7). It becomes "the VM's copy of focusing
    steps step 5 (`focusing_steps`) until A2c (LR-B4)".
- **`tree_clone.rs`**: the `FocusValueSnapshot` entry's parenthetical ("the value before the first user value change,
  plan D-1" → "the first-focus value and the user-change mark of the current focus session").
- **Umbrella**: LR-B6's PR-β part.

## §7. Touch lists with line counts (`wc -l` at `1f81fdc2`; PR-β re-runs at its base)

**PR-α**

| File | Lines | Change | Split rule |
|---|---|---|---|
| `crates/dom/elidex-form/src/focus_snapshot.rs` | 258 | B-2, B-3, B-4 (kind conjunct), B-7; docs; PB-* | < 1000 |
| `crates/dom/elidex-form/src/reconciler.rs` | 340 | clear deleted (B-7) | < 1000 |
| `crates/dom/elidex-form/src/reconciler_tests.rs` | 747 | e2c rewritten, e2d comment, DC-14u | < 1000 |
| `crates/dom/elidex-form/src/test_support.rs` | 42 | optional tag-taking fixture | < 1000 |
| `crates/dom/elidex-form/src/lib.rs` | 69 | `clear_focus_snapshot` re-export removed | < 1000 |
| `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | 773 | `commit_change_on_blur` doc | < 1000 |
| `crates/shell/elidex-shell/src/content/focus.rs` | 404 | module doc, `dispatch_change_on_blur` doc | < 1000 |
| `crates/script/elidex-js/src/vm/host/html_element_proto.rs` | 909 | comments only (§4.6's PR-α phrases) | < 1000 (±0) |
| `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs` | 679 | DC-14, DC-15, DC-16, DC-18, DC-19; module doc line | < 1000 (about +90) |
| `crates/dom/elidex-form-core/src/sanitize.rs` | 308 | doc (B-10) | < 1000 |
| `crates/dom/elidex-form-core/src/sanitize_tests.rs` | 451 | one comment (B-10) | < 1000 |
| `.claude/tools/ecs-mission-allowlist.tsv` | 303 | row 43 pruned | data |
| `docs/plans/2026-11-focus-fixup-program.md` | 285 (287 after this docs PR) | LR-B6 PR-α pointer | doc |

**PR-β**

| File | Lines | Change | Split rule |
|---|---|---|---|
| `crates/dom/elidex-form/src/focus_snapshot.rs` | 258 + PR-α | B-1, B-4 (comparison); docs; tests (DC-11, c7 rewrites, PB-1β) | < 1000 |
| `crates/dom/elidex-form/src/test_support.rs` | 42 + PR-α | `Snap` re-defined | < 1000 |
| `crates/dom/elidex-form/src/user_edit.rs` | 145 | test module's `marked` helper (§5.3) | < 1000 |
| `crates/dom/elidex-dom-api/src/focus/update_steps.rs` | 773 + PR-α | B-5; docs; DC-10u, DC-10r, `TestSink.reentrant_on_focus` | < 1000 (about +25) |
| `crates/shell/elidex-shell/src/content/focus.rs` | 404 + PR-α | comments | < 1000 |
| `crates/script/elidex-js/src/vm/host/html_element_proto.rs` | 909 + PR-α | comments only (B-6) | < 1000 (±0) |
| `crates/script/elidex-js/src/vm/tests/tests_html_element_proto.rs` | 658 | E17v | < 1000 |
| `crates/shell/elidex-shell/src/content_change_on_blur_tests.rs` | 679 + PR-α | DC-1, DC-1v, DC-8, DC-10, DC-10n, DC-17, renames, comments | < 1000 expected (about +100 over PR-α's); PR-β re-runs `wc -l` at its base and applies the touch-time split rule if the file is past 1000 |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 418 | one table cell | < 1000 |
| `.claude/tools/ecs-mission-allowlist.tsv` | 303 − 1 | rows 44 and 141 pruned | data |
| `docs/plans/2026-11-focus-fixup-program.md` | 287 (after this docs PR) + PR-α | LR-B6 PR-β pointer | doc |

No touched file is over 1000 lines at `1f81fdc2`, so neither PR needs a prereq split as planned. Not touched by either:
`elidex-form-core/src/lib.rs` (B-3 keeps the predicate private to `focus_snapshot.rs` on the design ground),
`content_focus_fixup_tests.rs` (its cells are controls, I-B8), `value_mode.rs` (F-6 is the new slot's). Re-run: `wc -l`
on the paths above.

## §8. Commit plans, decided questions, scout answers

### §8.1 Commit plans

**PR-α.** (α1) DF-2: B-2, B-3, B-4's kind conjunct, B-7 with PB-*, DC-14 to DC-16(u), DC-18, DC-19 and §6.1's comments
on those functions; (α2) B-10: `sanitize.rs` doc, `sanitize_tests.rs` comment, row 43 pruned; plus the LR-B6 PR-α
pointer. Each commit builds and is green; red-first results per cell in the PR body.

**PR-β.** (β1) DF-1: B-1, B-4's comparison, B-5, B-6 with DC-1(v), DC-8, DC-10(n/u/r), DC-11, DC-17, PB-1β, E17v, the
`Snap` re-definition and its users (`focus_snapshot.rs`, `user_edit.rs`), §6.2's comments, rows 44 and 141 pruned;
(β2) B-11 renames (no behaviour); plus the LR-B6 PR-β pointer. Each commit builds and is green.

### §8.2 Decided questions

- **OQ-1 One slice or two** — PM 933: two (§1).
- **OQ-2 `sanitize_color` behaviour** — PM 932: comment only (B-10); new slot LR-B2. PM 931 had approved the behaviour
  fix conditional on Q-S3; Q-S3 failed (§8.3).
- **OQ-3 Test renames** — PM 931 ("OQ-1、OQ-3 は報告のとおりでかまいません"): rename (B-11).

### §8.3 Scout answers (R1)

- **Q-S1** (the HTML `input` / `textarea` test): `FormControlState` is only ever attached to HTML-namespace elements
  (`elidex-form` `init.rs` `is_html_namespaced`, at parse, insert, the `createElement` post-handler and clone). The
  canonical gate is `EcsDom::is_html_namespace` (`elidex-ecs` `classify.rs`); B-2 uses it with the `TagType` read and
  adds no gate of its own.
- **Q-S2** (`FormControlState::value()` for Checkbox / Radio without a `value` attribute): `""`. `"on"` appears only in
  the IDL getter (`value_mode.rs`, the DefaultOn mode) and in `submit.rs`. So `F0` = `""` under R-3 for DC-14; DC-14's
  expected value holds under either reading (re-derivation §6.2).
- **Q-S3** (does `FormControlState::default_value` equal the `value` content attribute after every write): **no** — this
  answer changed a decision (OQ-2, PM 932). (1) `EcsDom::dispatch_event` does nothing while the mutation dispatcher is
  Absent / Taken (teardown, `inert_document.rs`), so a `value` write then skips the reconciler. (2) The reconciler's
  `value` arm (`reconciler.rs` `handle_attribute_change`) leaves `default_value` wrong in two distinct cases (both
  verified at `47cb4834`):
  - (2a) A **dirty** control of kind TextArea / Select / Output / Meter / Progress: the arm's gate
    (`!fcs.dirty_value || !matches!(fcs.kind, TextArea | Select | Output | Meter | Progress)`) skips the mirror update,
    so `default_value` keeps the old attribute value. This is reachable on `<input>` only through `from_type_str`'s
    mapping (O-B1).
  - (2b) A **non-dirty** `<textarea>`: the gate passes, and the arm overwrites `default_value` (and, in its
    `!dirty_value` branch, the live value) with the `value` attribute. A textarea's default value is its child text
    content, not a `value` attribute.

  (3) An absent and an empty attribute are indistinguishable in a `String`.

## §9. Ledger records (applied by the PM)

Slot text cites files and symbols, no line numbers.

- **LR-B1 → `#11-change-on-blur-baseline-first-focus-value`.**
  - *At this memo's landing* — append: "Plan: `docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md` (two per-PR
    slices of the focus-fixup umbrella, PM 933): PR-α (population, DF-2; with the `sanitize_color` comment, PM 932),
    then PR-β (first-focus value, DF-1)." The row's requirements are unchanged.
  - *At PR-α's landing* — append: "PR-α #NNN: the seed writes every `input` and `textarea` element
    (`record_focus_snapshot`, by `EcsDom::is_html_namespace` and `TagType`), no write chooses by kind
    (`clear_focus_snapshot` and the reconciler's `type`-arm clear deleted), and `take_change_on_blur` evaluates step
    2.1's kind conjuncts at focus loss (one private predicate, `change_on_blur_kind_applies`, under R-2; `<textarea>` by
    D-4). DC-14 to DC-16, DC-18, DC-19 green. DF-1 open: PR-β." Requirement 5 is met; requirements 1–4 remain.
  - *At PR-β's landing* — close: "Fixed by #NNN (PR-α) and #NNN (PR-β). `FocusValueSnapshot` (`elidex-form`
    `focus_snapshot.rs`) holds the value at first focus and the user-change mark on every `input` and `textarea` element
    from the seed, seeded at focus update steps step 4.1 before the `focus` / `focusin` listeners
    (`focus_update_steps`) and by VM `focus()` through the one rule `record_focus_snapshot`. `take_change_on_blur` fires
    iff the kind at focus loss passes step 2.1's conjuncts, the user changed the value, and the value differs from both
    the first-focus value and the value just before the first user edit (D-3). Cells DC-1 to DC-19 green. Allowlist
    rows pruned: the `note_user_value_change` doc (T, this slot) and the `focus_with_caret_at_end` doc (E)."
**Timing.** LR-B2 and LR-B3 are registered **at this memo's (the docs PR's) landing**, from its landing list (PM 931 /
PM 932), not through PR-α's PR body. PR-α's code cites both slots, so they exist before PR-α opens, under the IDs the
PM registers (PR-α DoD 6). The other records are applied at the landing each one names.

- **LR-B2 → the PM: new slot draft (PM 932), registered at this memo's landing**:
  **`#11-color-well-update-reads-live-value`**, Owner Layout. "Defect [spec]: HTML *update a color well control color*
  step 2.1 reads the `value` content attribute when the dirty value flag is false; `sanitize_value`'s Color arm
  (`elidex-form-core` `sanitize.rs`) reads the live value. Example: `<input type=number value=red>` then `type =
  "color"`: spec `#ff0000`, elidex `#000000`. Fix direction: read the content attribute from `Attributes`, as the repo
  precedents do (`elidex-form` `value_mode.rs` type-change step 2, `submit.rs`), not the `FormControlState::default_value`
  mirror. Why not the mirror: `default_value` is not an invariant mirror — (1) `EcsDom::dispatch_event` does nothing
  while the mutation dispatcher is Absent / Taken (teardown, `inert_document.rs`), so a `value` write then skips the
  reconciler; (2a) for a dirty control of kind TextArea / Select / Output / Meter / Progress, the gate of the
  reconciler's `value` arm skips the mirror update and leaves `default_value` stale. That is reachable on `<input>` only
  through `from_type_str`'s mapping, the slot of LR-B3 (O-B1) of the change-on-blur plan. (2b) For a non-dirty
  `<textarea>`, the same arm overwrites `default_value` (and the live value) with the `value` attribute, although a
  textarea's default value is its child text content. (3) An absent and an empty attribute are indistinguishable in a
  `String`.
  Also: `value_mode.rs`'s comment at type-change step 2 gives as its reason that the buffered flush path
  (`SessionCore::flush` → `apply_set_attribute`) writes `Attributes` without the reconciler; that reason is stale (since
  #428 `apply_set_attribute` calls `dom.set_attribute`, which dispatches); the conclusion (read `Attributes`) stands on
  reasons (1)–(3), and rewording that comment is part of this slot's fix. Open design question (this slot's, not the
  change-on-blur plan's): what `sanitize_value` as a whole reads. Found by
  `docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md` (F-1, Q-S3); `sanitize_color`'s doc names this slot after
  PR-α. **Why deferred**: PM 932: what `sanitize_value` reads is a design question for `sanitize_value` as a whole, kept
  out of the change-on-blur slice. **Trigger** (proposed): the next PR that touches `sanitize_value` /
  `sanitize_for_type_change`, or this slot's own fix PR. **Re-eval-date** (proposed): 2026-12-31." The PM may change the
  trigger and date. The open design question of what `sanitize_value` as a whole reads belongs to this slot; no other
  record carries it.
- **LR-B3 → the PM: slot draft for O-B1** (F-2), registered at this memo's landing, proposed ID `#11-input-type-keyword-state-mapping` (the PM accepts or
  renames it): "Defect [spec]: `FormControlKind::from_type_str` (`elidex-form-core`) maps the `type` keyword `image` to
  `TextInput` (HTML has an Image Button state, to which the input and change events do not apply), and `select-one` /
  `textarea` to `Select` / `TextArea`, where HTML's invalid value default is the Text state. An `<input type=image>` is
  seeded, edited and judged as a text control. Through the mapping, a stale `default_value` is also reachable on an
  `<input>` (uncertain, found by plan-review R1): `<input type=textarea value="#111111">` → script `value = "abc"`
  (dirty) → `setAttribute("value", "#222222")` leaves `default_value` stale (the reconciler's `value` arm gate) →
  `form.reset()` → `type = "color"`. Also, `from_type_str`'s doc cites "§4.10.5.1" for the Text fallback; the missing
  value default and invalid value default are stated in §4.10.5 The input element, so the fix cites §4.10.5.
  **Trigger**: the next PR that touches `from_type_str` or adds the Image Button
  state. **Why deferred**: found by the change-on-blur baseline plan, whose cells do not depend on it." Proposed owner
  L3 (form domain); proposed **Re-eval-date 2026-12-31** (the PM may change it). Ledger check: F-2.
- **LR-B4 → `#11-vm-host-synthetic-dom-event-dispatch` (A2c)** — append: "When VM `focus()` routes through
  `focusing_steps`, its own redundant-focus return and its direct `record_focus_snapshot` call are deleted, and the VM sink's
  `seed_focus_snapshot` (its `FocusEventSink` impl) must call `elidex_form::record_focus_snapshot` (I-B2 of
  `docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md` depends on it: every designation seeds): the seed is then
  the transition's step-4.1 call (that memo's B-5, B-6), and the VM sink's `commit_change_on_blur` calls
  `take_change_on_blur` (umbrella X1)."
- **LR-B5 → the PM: PM 393 D1 and PM 307's "initial = before"** — the re-derivation's LR-6 proposed their withdrawal;
  B-1 is the re-decision the slot names. Nothing new to withdraw here.
- **LR-B6 → the umbrella (`docs/plans/2026-11-focus-fixup-program.md`), carried by the code PRs as in-line errata**
  (the re-derivation's §8 table covers DC2, §1.1 (b), §1.2 row (1), the §3 rows, X2 and §7, not these two sites; checked
  2026-11-22). At PR-α's landing, X1's parenthetical "(the qualifiers of DC2 and H's predicate, plan (1) D-1 / §4.3)"
  and record PR-1's "(H's predicate, plan (1) D-1 / §4.3)" get "[erratum: the kind qualifiers are step 2.1's conjuncts at
  focus loss, `docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md` B-3]". At PR-β's landing the same two sites
  get "[erratum: the comparison is with the first-focus value and the value just before the first user edit, same memo
  B-4]". Each adds a dated changelog line in the umbrella's order (newest first). X1's "one judgement, every sink" is
  unchanged.
- **LR-B7 → `#11-core-browser-parity-deviations`, facet (g), at PR-α's landing.** Facet (g) was added at #656
  (PM 909; owner L3). It names the `sanitize.rs` interop line of `sanitize_color`'s doc and allowlist row 43 (class T),
  with "Fix with the slot's code PR". At PR-α's landing the PM closes facet (g) with: "Closed by #NNN (PR-α of
  `docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md`): the comment states step 2.1 as the spec does, names no
  engine, and allowlist row 43 is pruned. The remaining code defect (the Color arm reads the live value when the dirty
  value flag is false) is `#11-color-well-update-reads-live-value` (owner Layout)." The PM also updates this slot's
  statements of its allowlist row count (one class-T row fewer). The defect then has one home, the new slot.

## §10. Definition of done

**PR-α**

1. §5.1 cells green on the final head; cells marked red observed red at `1f81fdc2` with the fix absent (results in the
   PR body); MG-A1 to MG-A4 each red on their cells, then reverted.
2. `git grep -n -w 'clear_focus_snapshot' -- crates` → 0; `git grep -n -w 'is_text_control' --
   crates/dom/elidex-form/src/focus_snapshot.rs crates/dom/elidex-form/src/reconciler.rs` → 0 (control: the same
   command over `crates/shell/elidex-shell/src/content/ime.rs` → ≥ 1). `git grep -n 'Namespace::Html' --
   crates/dom/elidex-form/src/focus_snapshot.rs` → 0 (control: the same over `crates/dom/elidex-form/src/init.rs` → ≥ 1).
3. `python3 .claude/tools/ecs_mission_ratchet.py` → `ok (294 allowlisted, 0 new, 0 stale)` (295 at `1f81fdc2`, minus row
   43); `/usr/bin/grep -c 'interoperable behavior' crates/dom/elidex-form-core/src/sanitize.rs` → 0 (control: the same
   at `1f81fdc2` → 1); `/usr/bin/grep -c '<LR-B2's registered ID>' crates/dom/elidex-form-core/src/sanitize.rs` → ≥ 1
   (proposed `color-well-update-reads-live-value`).
4. §0.2.1's enumeration re-run on the final head: still two production designation writers, both seeding (I-B2).
5. `cargo fmt --all`; `cargo test -p elidex-form -p elidex-form-core -p elidex-dom-api -p elidex-shell --all-features`;
   `mise run ci` green (`grep 'ERROR task failed'` on its log → none).
6. Before PR-α opens, LR-B2's and LR-B3's slots are registered in the ledger (at this memo's landing, §9 Timing). Every
   slot ID PR-α's code cites (LR-B2's in `sanitize.rs`; LR-B3's in `change_on_blur_kind_applies`'s doc, §4.4) is the ID
   as registered: if the PM renamed a slot at registration, PR-α uses the registered ID. Check: `/usr/bin/grep -c -F
   '<ID>' <ledger>` → ≥ 1 for each. PR body carries LR-B1's PR-α text, LR-B6's PR-α part and LR-B7.

**PR-β**

1. §5.2 cells green on the final head; cells marked red observed red at PR-α's merge with the fix absent; MG-B1 to
   MG-B4 and MG-B9 each red on their cells, then reverted. Every §5.1 cell still green.
2. `git grep -n -P '\.baseline\(\)|Snap::Baseline' -- crates` → 0 (the accessor and the old `Snap` arm are gone;
   control: the same command at PR-α's merge → ≥ 1, and `git grep -n 'Snap::Absent' -- crates` → ≥ 1 on the final head).
3. `python3 .claude/tools/ecs_mission_ratchet.py` → `ok (292 allowlisted, 0 new, 0 stale)`; `/usr/bin/grep -c
   'change-on-blur-baseline-first-focus-value' .claude/tools/ecs-mission-allowlist.tsv` → 0 (control:
   `blur-focus-cleared-before-blur-event` → 1).
4. §0.2.1's enumeration re-run on the final head: still two production designation writers, both seeding before any
   listener (I-B2, I-B4).
5. `cargo fmt --all`; `cargo test -p elidex-form -p elidex-dom-api -p elidex-js -p elidex-shell --all-features`;
   `mise run ci` green (`grep 'ERROR task failed'` on its log → none).
6. PR body carries LR-B1's closing text and LR-B4.

**This docs PR**: its landing list carries LR-B1's memo-landing text, LR-B2 and LR-B3 (both registered at this
landing, §9 Timing); `python3 .claude/tools/ecs_mission_ratchet.py` → `ok (295 allowlisted, 0 new, 0 stale)`;
`python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md`
exits 0. `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-11-focus-fixup-program.md` exits 2 at
`1f81fdc2` and on this branch ("no table matched schema …"): the umbrella has none of the checker's table schemas, so
it is not scanned — a skip, not a clean run, and not changed by this PR.

## Changelog

- **Rev 3 (2026-11-22)** — plan-review R2. Findings → where fixed:
  R2-IMP-1 (Ax5; ledger facet (g) already exists) → B-10, §9 LR-B7 (new; facet (g) closed at PR-α's landing and
  re-pointed to `#11-color-well-update-reads-live-value`), §10 PR-α DoD 6, rev 2's changelog wording. R2-IMP-2 (Ax5;
  LR-B2 / LR-B3 timing) → §9 Timing, LR-B2, LR-B3, §10 "This docs PR" and PR-α DoD 6 (replaces its routing); also
  resolves R2-Ax3-MIN-5 (PR-α uses the registered IDs; PR-α DoD 3). R2-Ax3-MIN-1 → §4.6, §6.1, §7 PR-α. R2-Ax1 note +
  R2-Ax3-MIN-3 → §6.1 (`content/focus.rs` module doc, `html_element_proto.rs` doc and inline phrase,
  `content_change_on_blur_tests.rs` module doc line, test-module comments, the "text control" sweep), §6.2
  (`html_element_proto.rs` early-return comment). R2-Ax3-MIN-2 → LR-B2 (Why deferred, Trigger, Re-eval; the
  `sanitize_value` design question is this slot's; R2-Ax5-MIN-6 resolved this way). R2-Ax3-MIN-4 → PB-5, PB-6, PB-7
  sequences, the note under §5.1, MG-A3 (red set listed). R2-Ax4-MIN → §3 (QB-3 in the §4.10.5 row), §3.1 QB-3, LR-B3
  (`from_type_str`'s doc cite). R2-Ax2-MIN-1 → PB-2, PB-3, §5.3 `test_support`. R2-Ax2-MIN-2 → §1.1.1 (new), B-9, I-B2,
  §4.7. R2-Ax5-MIN-1 → umbrella §7 pointer and changelog entry (two per-PR slices, PM 933), this memo's header.
  R2-Ax5-MIN-2 → §7 umbrella rows (285 at `1f81fdc2`). R2-Ax5-MIN-3 → F-4, B-11, §8.2 OQ-3 (PM 931). R2-Ax5-MIN-4 →
  §8.3 Q-S3 (2a) / (2b), LR-B2. R2-Ax5-MIN-5 → LR-B2 ("the slot of LR-B3 (O-B1)"). R1 findings that R2 Ax5 could not
  locate in rev 2's map: F1 → B-0, §1 (split, PM 933); F2 / F3 → B-10, F-1, §8.2 OQ-2 (option (B) dropped, PM 932);
  F15 → LR-B3 (the O-B1 `<input type=textarea>` reset path to a stale `default_value`); F16 → B-P16, B-10, SC-1, §6.1
  (`sanitize_tests.rs` interop comment).
- **Rev 2 (2026-11-22)** — plan-review R1 and PM 932 / PM 933. Findings → where fixed:
  OQ-1 / PM 933 → B-0, §1 (decision; lens table corrected), §1.1 (post-PR-α state), per-PR §5.1 / §5.2, §6.1 / §6.2,
  §7, §8.1, §9 LR-B1, §10. OQ-2 / PM 932 → B-10, F-1 (history), §6.1, §8.2, §9 LR-B2 (new slot draft
  `#11-color-well-update-reads-live-value`; rev 2 called rev 1's facet "withdrawn", but the ledger already held it as
  facet (g), which is closed at PR-α's landing, LR-B7 of rev 3); R1 Ax2 MIN
  (`sanitize_tests.rs` interop ground) → B-P16, B-10, SC-1, §6.1. OQ-3 → B-11. F4 → B-2, B-P17, §4.2, §4.8, §8.3
  Q-S1, PR-α DoD 2. F5 → PB-1α / PB-1β. F6 → LR-B3 (re-eval date, the R1 Ax2 path). F7 → §8.3 (scout answers replace
  the questions). F8 → B-3 (line-count ground dropped), §7. F9 → §4.4 lead. F10 → §4.4 doc requirement. F11 →
  §5.3, §7 PR-β (`user_edit.rs`; PR-α does not touch `Snap`). F12 → B-P18, F-5, B-5, I-B5, DC-10n, MG-B1. F13 → LR-B6.
  F14 → umbrella changelog order (the pointer line moved to the top, newest first). F17 → LR-B4. F18 → B-P10,
  §0.2.1, DoD 4 of each PR. Rev-1 F-1 and LR-B2 re-checked against PM 932 (F-1 is history; LR-B2 is the new slot).
  F-6 (stale `value_mode.rs` reason) added for LR-B2.
- **Rev 1 (2026-11-22)** — initial plan from the slot row (`#11-change-on-blur-baseline-first-focus-value`, PM 861 /
  909) and the re-derivation; premises re-verified at `1f81fdc2`; findings F-1 to F-4; open questions OQ-1 to OQ-3.
