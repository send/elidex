# Plan — L1 reconciliation completeness (form derived-state Slice 0b)

**Parent**: `docs/plans/2026-07-form-derived-state-reconciliation.md` §4 (FCS-layer holes) + §6 slicing-table row "0b" (umbrella, `/elidex-plan-review`-converged R1-R3).
**Lane**: DOM/form (L3). **Branch**: `domform-slice0b`. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-slice0b`.
**Base**: `9777cdbd` (current `main`; Slice 0a `elidex-form-core` carve **#472 `3e797897`** is in the base).
**Status**: Slice-0b plan-memo, **`/elidex-plan-review` CONVERGED** (5-agent: 0 CRIT / 1 IMP / 4 MIN, all applied). Ready to implement.
**Nature**: a **behavior-completing** slice — it adds three missing reconciler arms and deletes two now-redundant IDL hand-mirrors. Not a refactor and not behavior-invariant (it *fixes* the reset-restores-stale-checkedness bug and the `textarea.rows` layout bug). It intersects ≥3 invariant axes (single-maintainer parity × dirty-checkedness gating × cross-crate visibility × chokepoint equivalence), so per CLAUDE.md's edge-dense rule it takes its own plan-review.
**Coordination** (mirrors the umbrella §7 lane-coordination precedent): `origin/vm-input-value-as-date` (active, unmerged) also touches `reconciler.rs` / `sizing.rs` (0b's hot files), but per MEMORY.md its real deliverable conflict is scoped to `input.rs` / `datetime.rs` — files 0b does **not** touch. The `reconciler.rs`/`sizing.rs` overlap is likely unrebased divergence, not a deliverable collision: 0b's actual edit set (reconciler `checked`/`rows`/`cols` arms + the two `html_input_value.rs` mirror deletions + the form-core helper + the two umbrella doc-edits) does not intersect its deliverable. No logical merge-order constraint on the deliverables — rebase whichever lands second.

---

## §0.5 Spec citation table

Per-concept dfn/anchor citations (webref-verified 2026-07-18; the §3 coverage map references these). The scout-supplied section guesses (`§4.10.5.4`/`§4.10.7`/`§4.11.4` for textarea; `§4.10.22.4` for reset) were **all wrong**; corrected here.

| Spec dfn / concept | § | Title | Anchor | webref command → result |
|---|---|---|---|---|
| `checked` **content** attribute (gives *default* checkedness; sets *live* checkedness only when not dirty) | §4.10.5 | The input element | `#attr-input-checked` / `#concept-input-checked-dirty-flag` | `dfn html 'checked'` → `element-attr for=input → §4.10.5 #attr-input-checked`; `body html concept-input-checked-dirty-flag` (prose in §3) |
| **dirty checkedness flag** | §4.10.5 | The input element | `#concept-input-checked-dirty-flag` | `dfn html 'dirty checkedness flag'` → `§4.10.5 #concept-input-checked-dirty-flag` |
| **checkedness** | §4.10.18.1 | A form control's value | `#concept-fe-checked` | `dfn html 'checkedness'` → `§4.10.18.1 A form control's value #concept-fe-checked` |
| `checked` **IDL** (live checkedness getter/setter) | §4.10.5.4 | Common input element APIs | `#dom-input-checked` | `dfn html 'checked'` → `attribute for=HTMLInputElement → §4.10.5.4 #dom-input-checked` |
| `defaultChecked` `[Reflect="checked"]` / `defaultValue` `[Reflect="value"]` **IDL** (HTMLInputElement IDL block, *before* §4.10.5.1 — **not** §4.10.5.4) | §4.10.5 | The input element | `#dom-input-defaultchecked` / `#dom-input-defaultvalue` | `dfn html 'defaultChecked'` → `attribute for=HTMLInputElement → §4.10.5 #dom-input-defaultchecked`; `dfn html 'defaultValue'` → `attribute for=HTMLInputElement → §4.10.5 #dom-input-defaultvalue`; `idl html HTMLInputElement` → `[Reflect="checked"] boolean defaultChecked; [Reflect="value"] DOMString defaultValue;` |
| **input reset algorithm** (checkedness ← *has* `checked` content attribute; value ← `value` content attribute) | §4.10.5 | The input element | (prose at `#concept-input-checked-dirty-flag`) | `body html concept-input-checked-dirty-flag` (prose in §3) |
| **Resetting a form** / per-control reset algorithm | §4.10.23 | Resetting a form | `#resetting-a-form` / `#concept-form-reset` / `#concept-form-reset-control` | `heading html 4.10.23` → `§4.10.23 Resetting a form`; `dfn html 'reset algorithm'` → `§4.10.23 #concept-form-reset-control` |
| `<textarea>` `rows` / `cols` **content** attributes | §4.10.11 | The textarea element | `#attr-textarea-rows` / `#attr-textarea-cols` | `dfn html 'rows'` → `element-attr for=textarea → §4.10.11 #attr-textarea-rows`; `dfn html 'cols'` → `§4.10.11 #attr-textarea-cols` |
| `<textarea>` `rows` / `cols` **IDL** (`[ReflectPositiveWithFallback, ReflectDefault=2]` / `=20`) | §4.10.11 | The textarea element | `#dom-textarea-rows` / `#dom-textarea-cols` | `idl html HTMLTextAreaElement` → `[ReflectPositiveWithFallback, ReflectDefault=2] rows; [ReflectPositiveWithFallback, ReflectDefault=20] cols;` |
| **reflection rule** for `rows`/`cols` ("valid positive integer → value; else → default") | §2.6.1 | Reflecting content attributes in IDL attributes | `#limited-to-only-non-negative-numbers-greater-than-zero-with-fallback` | `dfn html 'limited to only positive numbers with fallback'` → `§2.6.1 #limited-to-only-non-negative-numbers-greater-than-zero-with-fallback` |
| `<textarea>` `defaultValue` (= child text) / **raw value** | §4.10.11 | The textarea element | `#dom-textarea-defaultvalue` / `#concept-textarea-raw-value` | `idl html HTMLTextAreaElement` → `[CEReactions] DOMString defaultValue;` (**no** `Reflect` → not attribute-backed); `dfn html 'raw value'` → `§4.10.11 #concept-textarea-raw-value` |
| **Constructing the entry list** (submission candidacy — **OUT of 0b**, Slice 1/2) | §4.10.22.4 | Constructing the entry list | `#constructing-form-data-set` | `heading html 4.10.22` → `§4.10.22.4 Constructing the entry list` (this is *submission*, not reset — the scout conflated it with reset) |

---

## §1 Goal — complete the single-maintainer reconciler (and name the one place the ideal has a structural gap)

The umbrella's ideal (§1.1, quoting `reconciler.rs:10-17` verbatim): the attribute-change reconciler is the **single system that maintains derived `FormControlState` (FCS) fields from content attributes** — "derived-state reconciliation belongs to a system subscribed to mutations of the source state, NOT a side effect of every IDL setter." One arm per `content-attribute → FCS-field` mapping; **no "second maintainer"** (an IDL setter hand-mirroring the same field) duplicating it.

Slice 0a carved `elidex-form-core` so this reconciler and the pull predicates can share the FCS component. **Slice 0b completes the reconciler's content-attribute coverage** so that Slice 1's pull derivations (which *read* FCS) read a faithfully-reconciled component instead of a stale one:

1. Add the three missing attribute arms — **`checked`**, **`rows`**, **`cols`** — that today fall through the catch-all `_ => {}` (`reconciler.rs:311`).
2. Delete the two **input** IDL hand-mirrors that the chokepoint now covers (`html_input_value.rs` `default_value` @202-211 and `default_checked` @277-280), shrinking `vm/host/`.

**The honest boundary (this is the point of the "single-maintainer" framing, not a caveat bolted on).** The reconciler subscribes to **`Insert` + `AttributeChange` only** (`reconciler.rs:40-47`). It is structurally blind to `characterData`/`childList` mutations. But **`<textarea>.defaultValue` reflects the element's child text ("raw value", HTML §4.10.11 `#concept-textarea-raw-value`), not a content attribute** — so the AttributeChange reconciler *cannot* be the single maintainer of the textarea `default_value`. The umbrella §4's blanket "remove the IDL hand-mirrors — the chokepoint already covers them once the arms exist" is **correct for the two input mirrors and wrong for the textarea one**. Therefore Slice 0b:

- **keeps** the textarea `default_value` mirror (`html_textarea_proto.rs:651-661`), and
- registers a **new defer slot** naming the future `characterData`/`childList` reconciliation seam that would let the reconciler reach it (§6 F, §10).

So 0b's scope is exactly **"three attribute arms + two input-mirror deletions"**, honestly bounded, and the single-maintainer ideal is *stated with its one structural exception made explicit* rather than silently over-claimed.

**Success** = the two live bugs the umbrella §1.2 named are fixed with new assertions, every mirror deletion is proven chokepoint-equivalent, and `cargo test -p elidex-form-core -p elidex-form -p elidex-js --all-features` + `mise run ci` are green.

---

## §2 Coupled invariants (edge-dense: why 0b gets its own review)

0b looks like "add three match arms," but each arm sits at the intersection of invariants where a prose-only "just add the arm" plan leaks. The value-visibility axis (I5) is exactly such a leak — a code scout's "only clone.rs writes `value`" claim is falsified below (§5), and it changes decision E.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Init/reconciler parity** — for the same attribute, the arm must leave FCS in the **same state** the init path (`from_*_element`, `elidex-form-core/src/lib.rs`) produces at createElement time. Two producers of one derived field that disagree = two maintainers = the bug the umbrella abolishes. | The `rows`/`cols` arm's fallback must equal `from_textarea_element`'s (2/20). This surfaces a **latent init bug**: `from_textarea_element:941-942` uses a plain `u32` parse (`rows="0"` → `0`), which disagrees with the spec reflection (`0` → default 2). Parity forces a decision (§10 J-C). |
| **I2** | **Dirty-checkedness gating** — the `checked` **content** attribute always gives the **default** checkedness, but sets **live** checkedness only "if the control does not have dirty checkedness" (HTML §4.10.5 `#concept-input-checked-dirty-flag`). | The **dirty checkedness flag is unmodeled** (umbrella §5/§6 Slice 4). Without it, the arm cannot decide the live-checkedness half safely (setting it unconditionally would clobber a user-toggled checkbox). So the arm does **only** the unambiguous half: `default_checked = <attr present>` (§ decision B). |
| **I3** | **Attribute-derived vs child-text-derived `default_value`** — `input.defaultValue` reflects the **`value` content attribute** (`[Reflect="value"]`, §4.10.5 IDL block); `textarea.defaultValue` reflects **child text** (§4.10.11 raw value), which has **no content attribute**. | The chokepoint can reach the input mirror (delete it) but **cannot** reach the textarea mirror by construction (keep it). The two mirrors are *not* the same species despite identical shape. |
| **I4** | **Chokepoint equivalence** — deleting a hand-mirror is sound **iff** `attr_set → set_attribute → reconciler` produces the same FCS the mirror wrote (the #466 F3 "provably-identical" contract). | Each deletion needs a per-branch proof + an equivalence test. The input `default_value` mirror's `set_value_initial` **omits** the Submit/Reset empty-label substitution the value-arm applies, so "equivalent" fails on one edge — where deletion is *strictly more correct* (§4 D-a). |
| **I5** | **Cross-crate component visibility** — `value`/`dirty_value` live in `elidex-form-core` but are field-accessed (read **and** write) by multiple **STAY-UP systems** in `elidex-form` (`reconciler.rs`, `select/mod.rs`, `focus_snapshot.rs`, `submit.rs`, `clone.rs`). | They are `pub` because *the whole elidex-form system surface* touches them — not because "only clone.rs does." This **falsifies** the scout's premise for decision E's `pub → pub(crate)` revert (§5) and, per 0a-J1, means `pub` is the **correct ECS-native** visibility (component data touched by many systems), not a widening to fix. |

**Pairwise intersections (load-bearing):**
- **I1 × I2** — parity would ordinarily demand the arm reproduce init exactly; but for `checked`, init (`from_input_element`) sets *live* checkedness from the attribute at create-time (when dirty-checkedness is definitionally false), while a *runtime* `setAttribute("checked")` may hit a user-toggled control. The arm cannot mirror init's live-set without the dirty flag → parity is satisfied on the **default** field (which both maintain) and the live field is deferred to Slice 4. This is why decision B is the headline review question.
- **I3 × I4** — the reason the input mirror is delete-safe and the textarea mirror is not is *the same fact* (what the IDL reflects), seen from the chokepoint's reach. Enumerating I3 prevents the umbrella §4 blanket-deletion error.
- **I1 × I5** — the `rows`/`cols` arm writes the `pub` fields directly (0a-J1: "ECS components take pub data, not OO accessors"); I5 says that is also why `value`/`dirty_value` should *stay* `pub`. The arm-writes-pub-fields pattern and the don't-add-setters conclusion are the same ECS-native principle.

---

## §3 Spec coverage map (schema; per-concept dfn/anchor evidence in §0.5)

Coverage-map schema (`| Spec section | Step | Branch | Touch | Full enum? | User-input flow |`, per `.claude/tools/webref coverage-map`). Each row's citation is webref-verified 2026-07-18 (full dfn/anchor evidence in §0.5). The scout's textarea/reset section guesses were all wrong; corrected here + §0.5.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.5 The input element | `checked` content attr → checkedness (`#attr-input-checked`; dirty-checkedness gate `#concept-input-checked-dirty-flag`) | default checkedness (**always**) vs live checkedness (only if dirty checkedness = false → **Slice 4 OUT**) | `checked` reconciler arm — `fcs.default_checked = new_value.is_some()` (reconciler.rs `match name`, NEW arm; does **not** write live `fcs.checked`) | ✓ | yes — `setAttribute("checked")` / `removeAttribute("checked")` |
| WHATWG HTML §4.10.5 The input element | `defaultChecked` `[Reflect="checked"]` / `defaultValue` `[Reflect="value"]` IDL reflection (HTMLInputElement IDL block, before §4.10.5.1 — **not** §4.10.5.4, which holds only the live `checked` IDL) | reflected IDL → the `set_attribute` chokepoint covers it → hand-mirror is a redundant 2nd maintainer → delete | **2 input hand-mirror DELETIONS** — `html_input_value.rs:202-211` (`default_value`), `:277-280` (`default_checked`) | ✓ | yes — `el.defaultChecked=` / `el.defaultValue=` |
| WHATWG HTML §4.10.23 Resetting a form | reset step: checkedness ← *has* `checked` content attr (elidex caches this as `default_checked`) | reset restores default checkedness (the arm keeps `default_checked` faithful to the attr) | `reset_value` (`lib.rs:677-687`, **existing & correct**; consumes the arm's `default_checked` — no code change) | ✓ | yes — `form.reset()` |
| WHATWG HTML §4.10.11 The textarea element | `rows`/`cols` reflect (IDL `ReflectPositiveWithFallback ReflectDefault=2/20`) + `defaultValue` = raw value (`#concept-textarea-raw-value`) | rows/cols valid-positive→value / 0·invalid·absent→default(2/20); **`defaultValue`=child text → no content attr → AttributeChange reconciler can't reach → mirror KEEP** | `rows`/`cols` arms + shared `parse_positive_with_fallback` (NEW) + textarea `default_value` mirror KEEP (`html_textarea_proto.rs:651-661`) | ✓ | yes — `setAttribute("rows"/"cols")`, `textarea.rows/cols=`, `textarea.defaultValue=` |
| WHATWG HTML §2.6.1 Reflecting content attributes in IDL attributes | "limited to only non-negative numbers greater than zero, with fallback" parse rule (`#limited-to-only-non-negative-numbers-greater-than-zero-with-fallback`) | parse non-negative & > 0 → value; else (0 / invalid / absent) → default | `parse_positive_with_fallback` (NEW; shared by `from_textarea_element` init + the rows/cols arms — J-C single-source) | ✓ | yes — via rows/cols |
| WHATWG HTML §4.10.22.4 Constructing the entry list | submission entry-list candidacy (`submit.rs:353`) — *not* reset (scout conflation corrected) | **OUT of 0b** — n/a (Slice 1/2 territory) | none in 0b | n/a (OUT) | OUT |

**Load-bearing prose** (`body html concept-input-checked-dirty-flag`, verbatim extract):

> "The `checked` content attribute is a boolean attribute that gives the **default checkedness** of the input element. When the `checked` content attribute is added, **if the control does not have dirty checkedness**, the user agent must set the checkedness of the element to true; when the `checked` content attribute is removed, **if the control does not have dirty checkedness**, the user agent must set the checkedness of the element to false."
>
> "The **reset algorithm for input elements** is to set its user validity, dirty value flag, and dirty checkedness flag back to false, set the value of the element to the value of the value content attribute, if there is one, … set the **checkedness of the element to true if the element has a `checked` content attribute and false if it does not**, …"

This prose is the spine of decision B: the content attribute (1) *always* sets **default checkedness** [→ 0b's arm sets `FCS.default_checked` unconditionally], and (2) sets **live checkedness** *only when not dirty* [→ deferred to Slice 4, which models the dirty checkedness flag]. It is also why the reset bug is real and why the arm fixes it: `reset_value` (`lib.rs:677-687`) restores `checked = default_checked` (`:684`), which is spec-faithful **iff** `default_checked` tracks the `checked` content attribute — which today it does not (no arm → `_ => {}` → `default_checked` never updates).

**Breadth** (all citations webref-verified 2026-07-18, evidence in §0.5): K=1 spec (WHATWG HTML), M=6 coverage rows (5 in-scope + §4.10.22.4 OUT) → single-PR scope (a completeness fill, not a spec-surface expansion).

### §3.1 User-input touch audit

All three arms sit on the **existing** `EcsDom::set_attribute`/`remove_attribute` chokepoint (`reconciler.rs:112` `handle_attribute_change`); no new opcode/native path. User-controllable input (`checked`/`rows`/`cols` content attributes, whether from parser, `setAttribute`, or a reflecting IDL setter) already flows here and today dead-ends at `_ => {}` (`:311`). The arms *add* handling to an input path that is otherwise dropped — exposure delta is a **fix** (stale FCS → reconciled FCS), no new attack surface. The two mirror deletions **remove** VM-host FCS writes, shrinking the marshalling surface.

---

## §4 Change-list (per-arm + per-mirror; file:line; provably-identical requirement)

All arm edits land in `crates/dom/elidex-form/src/reconciler.rs` (the STAY-UP reconciler system). Precedents cited are in the same file. The catch-all is `_ => {}` at **`:311`**; the `match name` starts at **`:240`**; the two early-return arms (`value` @172-231, `pattern` @233-238) precede the match.

### A. `checked` arm — updates `default_checked` **only** (decision B)

```rust
// HTML §4.10.5 #attr-input-checked / #concept-input-checked-dirty-flag.
// The `checked` content attribute gives the DEFAULT checkedness. Setting
// LIVE checkedness is gated on the dirty checkedness flag (unmodeled →
// Slice 4), so 0b maintains only the default half — which is what
// `reset_value` (`lib.rs:684`) consumes.
"checked" => fcs.default_checked = new_value.is_some(),
```

- **Placement**: a `match` arm (boolean-attribute cluster alongside `disabled`/`required`/`readonly` @257-260). It creates no guard-conditional no-op body, so — unlike `value`/`pattern` — it does **not** need to be an early-return (no clippy match-arm-collapse concern).
- **Does NOT touch `fcs.checked`** (live checkedness). Live `checked` has its own legitimate direct writer, the IDL setter `native_input_set_checked` (`html_input_value.rs:244`) — a `state.checked = flag` for *attribute-less live state*, correctly **not** a hand-mirror and **not** touched.
- **Fixes**: `setAttribute("checked")` / `removeAttribute("checked")` now update `default_checked` → `form.reset()` restores the correct checkedness (umbrella §1.2 "reset restores a stale value").
- **Non-regressing on live `:checked`**: today `setAttribute("checked")` leaves `FCS.checked` untouched (catch-all); after 0b it *still* leaves `FCS.checked` untouched. `input.checked` (IDL getter reads `FCS.checked`) is unchanged; the live-checkedness spec gap (a not-dirty control should reflect the freshly-set attribute) **pre-exists** and closes in Slice 4 with the dirty flag — 0b does not widen it. Live `:checked` is cache-based until Slice 1 regardless.

### B. `rows` / `cols` arms — parse → `pub` field with the §2.6.1 fallback (decision C)

```rust
// HTML §4.10.11 #attr-textarea-rows/cols; reflection rule §2.6.1
// (limited to only non-negative numbers greater than zero, with
// fallback). `ReflectDefault=2` (rows) / `=20` (cols). Precedent = the
// `min|max|step` arm @300 (parse → write the pub FCS field).
"rows" => fcs.rows = parse_positive_with_fallback(new_value, 2),
"cols" => fcs.cols = parse_positive_with_fallback(new_value, 20),
```

where `parse_positive_with_fallback(v, default)` returns the parsed value **iff** it parses as a non-negative integer **> 0**, else `default` (so absent / invalid / `0` → default, per §2.6.1). Write the `pub` fields directly — **no** `update_rows`/`update_cols` setter (0a-J1: ECS components take pub data). `rows`/`cols` are `pub` (`lib.rs:377-379`); confirmed.

- **Fixes** the umbrella §1.2 layout bug: `textarea.rows = 10` (IDL setter `long_set` @497-511 → `attr_set("rows","10")` → chokepoint) today dead-ends at `_ => {}`, so `FCS.rows` never changes and `sizing.rs:50` (`state.rows.max(1)`, in `form_intrinsic_size`) uses the stale count. With the arm, the IDL setter's existing `attr_set` reaches `FCS.rows` end-to-end.
- **`rows`/`cols` IDL setters are NOT hand-mirrors** — `native_textarea_set_rows`/`set_cols` (`html_textarea_proto.rs:513-543`) already route through `attr_set`; they merely *dead-end today* at the missing arm. Adding the arm makes them work; **do not touch them**.
- **No `kind == TextArea` gate** — matches the `size` arm (@291) and `min|max|step` (@300), which set the field unconditionally; the field is read only for `TextArea` by `sizing.rs`, so a stray `rows` on a non-textarea is inert. (Minor judgment item, §10; recommend no gate.)
- **Divergence from the `size` arm is intentional**: `size` (@291) uses `unwrap_or(0)` because the *concrete* size default is element-type-dependent (input=20, select=1/4) and a runtime mutation can't know which without gating on kind, so it defers the default to `from_*_element`. `rows`/`cols` are **textarea-only**, so their default (2/20) is unambiguous and the arm applies it directly — a faithful single-maintainer value. (The `size` arm's `0`-on-removal is a **pre-existing observation in a different, untouched arm** [`size` @291] — orthogonal to 0b's touch-set [`rows`/`cols`/`checked`]; **no 0b action and no slot** — it is not 0b's deferred work.)
- **`parse_positive_with_fallback` home** (DECIDED, per J-C): authored in `elidex-form-core` as a pure helper and used by **both** `from_textarea_element` (init) **and** the rows/cols arm — one reflection, one maintainer (One-issue-one-way). This also fixes the latent init bug (`lib.rs:941-942` plain-parse `rows="0"`→0). **No arm-only fallback** — leaving init on the plain parse would re-open the init-vs-arm two-maintainer divergence the umbrella §1 single-maintainer ideal forbids.

### C. Delete input hand-mirror (a) — `default_value` (decision D-a)

- **Site**: `native_input_set_default_value` (`html_input_value.rs:190-213`). Delete the FCS-write block **@205-211** (the `let dom = …; if let Ok(mut state) = … { state.default_value.clone_from(&s); if !state.is_dirty() { state.set_value_initial(s); } }`). Keep the `attr_set(ctx, entity, "value", &s)` @201 (the reflection that reaches the chokepoint).
- **Provably-identical proof required at impl** (F3-style, per-branch):
  - `attr_set` @201 → `set_attribute("value", s)` → reconciler **value-arm** (`:172-231`), which for `<input>` (never in the `TextArea|Select|Output|Meter|Progress` set) **always** updates `default_value` (`:196-198`, condition `!dirty || !matches!(…)` is true for input) — identical to the mirror's unconditional `default_value.clone_from` @207.
  - **Non-dirty, non-empty-submit/reset**: value-arm sets `value = raw` (`:227-229`, `displayed == raw`); mirror's `set_value_initial(raw)` sets `value = raw`. **Identical.**
  - **Dirty**: value-arm skips the value update (dirty branch @199-205, `recorrect_range` only); mirror skips `set_value_initial` (`!is_dirty` false). Both update only `default_value`. **Identical.**
  - **The one non-identical edge — and it is strictly more correct**: for an empty-value Submit/Reset button, the value-arm applies the default-label substitution (`:210-214` → `"Submit"`/`"Reset"`) while the mirror's `set_value_initial("")` writes `value = ""` (no substitution, `lib.rs:646-650`) — so **today the mirror clobbers the correct label back to empty**. Deleting the mirror lets the value-arm's substitution stand, matching reconciler test **`e5c`** (`reconciler.rs:624-640`, asserts `FCS.value == "Submit"` for `<input type=submit value="">`). The JS-observable `input.value` returns the `value` *content attribute* (`""`) either way, so the fix is visible in the **rendered button label / internal display value**, not the IDL getter. Document this as an intentional correctness improvement, not a regression.
- **Equivalence test**: a VM/reconciler test that a non-submit input's `defaultValue = "x"` leaves `FCS.default_value == "x"` and `value == "x"` post-deletion (the common-case identity), plus reliance on `e5c` for the Submit/Reset edge.

### D. Delete input hand-mirror (b) — `default_checked` (decision D-b) — **gated on arm A**

- **Site**: `native_input_set_default_checked` (`html_input_value.rs:262-282`). Delete the FCS-write block **@277-280** (`let dom = …; if let Ok(mut state) = … { state.default_checked = flag; }`). Keep the `attr_set("checked","")` / `attr_remove("checked")` @272-276.
- **Delete-safe ONLY after arm A exists** — it is the sole current maintainer of `default_checked` from this path (the reconciler has no `checked` arm today).
- **Provably-identical proof**: `attr_set`/`attr_remove` @272-276 → chokepoint → **arm A** sets `default_checked = new_value.is_some()`; for `flag=true` (`attr_set`) → `is_some()` = true = `flag`; for `flag=false` (`attr_remove`) → `is_some()` = false = `flag`. Byte-identical. (Arm A sets *only* `default_checked`, exactly as the mirror did — neither touches live `checked`, so no divergence.)
- **Equivalence test**: `input.defaultChecked = true; then read default-checkedness / reset behavior` matches pre-deletion; plus a mirror-deletion regression that `form.reset()` after `setAttribute("checked")` restores checked (the arm-A fix).

### E. Keep textarea hand-mirror (c) — `default_value` (decision F)

- **Site**: `native_textarea_set_default_value` (`html_textarea_proto.rs:639-663`). **Not deleted in 0b.**
- **Why it cannot be deleted**: it runs after `invoke_dom_api("textContent.set", …)` @650 — a **child-text** mutation, **not** an attribute write. `textarea.defaultValue` reflects child text (§4.10.11 raw value; IDL `defaultValue` has no `Reflect`). The reconciler subscribes to `Insert`+`AttributeChange` only, so `textContent.set` never reaches it. Deleting the mirror would leave `textarea.defaultValue = "x"` updating the DOM text but **not** `FCS.default_value`/`value` → stale `.value` and stale reset. Guarded conceptually by the umbrella §1.2 asymmetry and by keeping the mirror + a documentation test (§8).
- The future seam that *would* let the reconciler cover it is a new mechanism → **new defer slot** (§6 F, §10).

### Not touched (explicitly)

- input `checked` IDL setter (`html_input_value.rs:244`) — live checkedness, no content attr, legitimate direct writer.
- input `indeterminate` IDL setter (`html_input_value.rs:313`) — same (attribute-less live state).
- textarea `rows`/`cols` IDL setters (`html_textarea_proto.rs:513-543`) — already route through `attr_set`; start working end-to-end once arm B exists.

---

## §5 The `adopt_value_state` cleanup — REFINED: the scout premise is falsified, recommend deferral (decision E)

**What decision E proposed** (from 0a-J5's hand-off): add `pub(crate) fn adopt_value_state(&mut self, value: String, dirty: bool)` to `elidex-form-core` covering `clone.rs`'s raw-restore write pair (`clone.rs:127-128` + the coupled `update_char_count()` @133), then **revert `value`/`dirty_value` from `pub` → `pub(crate)`**. Stated rationale: clone.rs is "the ONLY production external writer of `value`/`dirty_value`," so eliminating it frees the revert.

**The premise is false (grep-verified 2026-07-18).** Applying the scout's *own* exclusion logic — it correctly kept `default_value` `pub` because `value_mode.rs:67` writes it — consistently to `value` and `dirty_value`:

| Field | External (elidex-form, non-test) field access that requires `pub` |
|---|---|
| `value` | **write** `select/mod.rs:382,440,449` (`state.value.clone_from`/`.clear`), `reconciler.rs:227-228` (`fcs.value.clear`/`.push_str`), `focus_snapshot.rs:81` (`state.value = …`), `clone.rs:127`; **read** `submit.rs:429` (`fcs.value.clone()`) |
| `dirty_value` | **read** `reconciler.rs:186,199`; **write** `clone.rs:128` |

Plus dozens of `#[cfg(test)]` sites across `elidex-form` (`reconciler.rs`, `clone.rs`, `submit_tests.rs`, `value_mode.rs`, `select/tests.rs`, `init.rs`). **Neither field can revert to `pub(crate)` by touching clone.rs alone** — `value` is raw-mutated by four STAY-UP systems (`select`, `reconciler`, `focus_snapshot`, plus clone) each of which would need its own encapsulating method, and `dirty_value` is read in the production value-arm.

**The ECS-native reading (0a-J1) says this is not a defect to fix.** 0a-J1's lesson: "a widening is a boundary signal — fix the boundary; do not add setters (ECS components take pub data, not OO accessors)." Here the boundary is the form-core/form crate split; the systems that touch `value`/`dirty_value` legitimately **stay UP** and access the component **DOWN**. `pub` is therefore the **correct minimum** visibility for component data that multiple systems mutate — the ECS-native shape. Adding accessors/`adopt_value_state` to "tighten" it would be the OO anti-pattern J1 warns against.

**The residual justification** decision E offered — that `adopt_value_state` is an *invariant method* (value + dirty + `char_count` coupled per #466), not a gratuitous setter — is real but **does not survive One-issue-one-way**: it would be **one** encapsulated write among **four** raw `value`-write sites that remain field-writes. That is precisely the "new seam + N legacy impls" strangler intermediate the discipline forbids. Adding it alone neither achieves the visibility goal (blocked) nor unifies the write surface (it wraps clone's three lines while `select`/`reconciler`/`focus_snapshot` keep raw-writing).

**Recommendation (headline plan-review item, §10 J-E): DROP decision E from 0b.**
- Leave `value`/`dirty_value` `pub` with a corrected doc note (they are the elidex-form system surface's component data, not a clone-only widening) — i.e. **close 0a-J5 as "no action: `pub` is the correct ECS-native visibility,"** not "deferred-with-a-fix-path via clone."
- If the value↔`char_count` coupling is judged to warrant method-encapsulation, do it **uniformly** across *all* raw `value`-write sites in a dedicated "form value-write API" pass, not piecemeal here → register **`#11-form-value-write-api-unification`** (§10). 0b's scope stays "three arms + two input-mirror deletions."
- **Zero correctness cost to deferring**: `clone.rs:127-133` is correct as-is (writes the pair + resyncs `char_count`); nothing in 0b regresses by leaving it.

(If the review overrides and wants a scoped `adopt_value_state` purely as a #466 invariant-guard *without* the impossible revert, that is the fallback — but it ships a lone-caller method that bends J1 for no visibility gain, and I recommend against it.)

---

## §6 What is explicitly OUT of Slice 0b (with the owning slice/slot)

- **The live-checkedness half of the `checked` attribute** (set live checkedness when not dirty) → **Slice 4** (`#11-input-dirty-checkedness-flag`): needs the dirty checkedness flag modeled. 0b does the default-checkedness half only (§ B).
- **The derive-at-reader pull predicates / `:checked` pseudo-class / ElementState-cache deletion** → **Slice 1** (the keystone). 0b only makes FCS faithful so Slice 1 reads a correct component.
- **`FCS.disabled` effective-disabledness re-layer + push-propagation deletion** → **Slice 2 / Slice 5** (umbrella §4/§4.1/§6). No `disabled` change here.
- **`char_count` derive-at-reader** → `#11-char-count-derive-at-reader` (umbrella §3.2, L3). Same species (a hand-synced cache) but its own measurement-gated follow-on.
- **`<select>.value`/`.selectedIndex` split-SoT** → `#11-select-value-split-sot` (umbrella §10). Different surface.
- **Textarea `defaultValue` (child-text) → FCS reconciliation** → **NEW slot `#11-textarea-defaultvalue-textcontent-reconciliation`** (decision F):
  - **Why deferred**: the reconciler subscribes to `Insert`+`AttributeChange` only; covering `textarea.defaultValue` (child text, §4.10.11 raw value) requires a **new `characterData`/`childList` reconciliation seam** — a genuinely new mechanism, not an arm. Slice 0b keeps the `html_textarea_proto.rs:651-661` mirror as the interim maintainer.
  - **Trigger**: the reconciler gaining a child-text subscription (e.g. when the FCS raw-value/value model is unified with the text-node model), or a WPT/site exercising dynamic `textarea.defaultValue` + reset.
  - **Re-eval**: when Slice 1's pull work or a later text-model slice touches the textarea value pipeline. **Date**: 2026-11.
- **`#11-form-value-write-api-unification`** (NEW, from §5): encapsulate all raw `value`/`dirty_value` writes (`select`/`reconciler`/`focus_snapshot`/`submit`/`clone`) behind invariant-preserving methods, *then* consider `pub(crate)`. **Why deferred**: a cross-module value-write-API design, not L1 reconciliation. **Trigger**: a value-model pass, or if a `char_count` desync bug ever appears. **Re-eval**: Slice 1+. **Date**: 2026-11. *(Only register this if the review keeps the coupling concern alive; otherwise close 0a-J5 as no-action per §5.)*

**Per-PR defer budget**: 0b creates **1** new slot (`#11-textarea-defaultvalue-textcontent-reconciliation`), or **2** if §5's unification slot is registered — within the ≤3 per-PR cap. **Durable registration target = `project_open-defer-slots.md`** (L3 cluster) at ship, per `reference_spawn-task-chips-not-durable` (a committed plan note alone "ages out" — the slot must land in the SoT file).

---

## §7 Layering check

- **Arms land in `elidex-form`** (`reconciler.rs`), the STAY-UP **system** crate — correct: the reconciler is a `MutationEvent` consumer (a system with `&mut EcsDom`), not engine-bound VM/host code. ✓
- **`parse_positive_with_fallback`** (a pure reflection helper) → **`elidex-form-core`** (engine-independent leaf), shareable with `from_textarea_element`. ✓
- **No new `vm/host/` algorithm** — the opposite: the two input-mirror **deletions shrink** `vm/host/html_input_value.rs` (removing FCS-write blocks), reinforcing the Layering mandate ("VM host/ is prototype install / brand check / marshalling only; DOM-mutation algorithm goes through the engine-independent path"). The mirrors were exactly the "second maintainer writing FCS from host/" that the mandate + umbrella forbid. ✓
- **No B1-core (`elidex-ecs`) touch** — 0b does not delete ElementState constants (that's Slice 1). Confined to `crates/dom/elidex-form*` + `crates/script/elidex-js/src/vm/host/html_input_value.rs`. ✓
- **`adopt_value_state`**: deferred (§5) → no form-core API addition. (If the review keeps it, it lands in form-core as an invariant method — still engine-independent.) ✓

### §7.1 ECS-native check

- **Arms write `pub` component fields directly** (`fcs.default_checked = …`, `fcs.rows = …`) — the ECS-native data-flow (system reconciles component data from the source-of-truth attribute). No OO setter introduced (0a-J1). ✓
- **The `checked` arm keeps a single writer per field**: `default_checked` ← the arm (content-attr-derived); live `checked` ← the IDL setter (attribute-less live state). Two *distinct* fields, one maintainer each — not a split SoT. ✓
- **Mirror deletions remove duplicate maintainers** — collapsing `{IDL setter writes FCS} + {reconciler writes FCS}` to `{reconciler writes FCS}` for `default_value`/`default_checked` on the input path. This is the One-issue-one-way collapse the umbrella wants. ✓
- **`value`/`dirty_value` stay `pub`** = component data mutated by multiple systems = correct ECS-native visibility (§5). No side-store, no registry, no accessor-wall. ✓
- **No new component read/write topology** — the pull predicates that *read* FCS are Slice 1; 0b only completes the *write* (reconcile) side. ✓

---

## §8 Test plan (the four differential gaps the scout found + the deferred-gap documentation test)

Engine-independent (`elidex-form` reconciler tests, using the `FormControlOnlyTestDispatcher` at `reconciler.rs:326`) + VM JS-level (`elidex-js`) as noted.

1. **`checked` attr → `default_checked` → reset** (arm A; the reset-bug proof):
   - reconciler: `setup("input", &[("type","checkbox")])`; `set_attribute("checked","")` → `FCS.default_checked == true`; `remove_attribute("checked")` → `default_checked == false`. (New arm coverage.)
   - reset: after `setAttribute("checked")`, `reset_value()` → `FCS.checked == true` (currently restores stale). Asserts the umbrella §1.2 "reset restores a stale value" fix.
   - **Non-regression guard**: `setAttribute("checked")` leaves `FCS.checked` (live) unchanged (deferred to Slice 4) — assert live `checked` is *not* set by the arm, so the deferral is explicit and a future Slice-4 change is detectable.
2. **`rows`/`cols` attr → FCS → `form_intrinsic_size`** (arm B; the layout-bug proof):
   - reconciler: `set_attribute("rows","10")` → `FCS.rows == 10`; `set_attribute("rows","0")` → `FCS.rows == 2` (§2.6.1 fallback); `set_attribute("cols","abc")` → `FCS.cols == 20`; `remove_attribute("rows")` → `FCS.rows == 2`.
   - integration: `form_intrinsic_size` (`sizing.rs:41`) reflects the updated `rows`/`cols` after a runtime attribute change (currently uses the stale init value).
   - VM end-to-end: **omitted (not JS-observable)** — the `textarea.rows`/`cols` IDL getters read the *content attribute*, not `FCS.rows`, so a JS `textarea.rows = 10; textarea.rows` round-trip is tautological (it passes even without the arm), and `FCS.rows` has no JS-observable getter in the VM `run()` harness (unlike `checked`, which is observable via `form.reset()`). The IDL-setter→`attr_set` half is covered by the existing `textarea_cols_round_trip`; the `attr_set`→arm→`FCS`→sizing half by the reconciler + `form_intrinsic_size` tests above.
   - **Parity guard** (ties to J-C): assert the arm's fallback for `"0"`/absent equals `from_textarea_element`'s result for the same attribute (they must agree post-J-C).
3. **Reconciler-arm unit tests** — each arm against its SoT, including the boolean-cluster placement for `checked` and the boundary values for `rows`/`cols` (0, negative, non-numeric, large).
4. **Mirror-deletion equivalence**:
   - input `default_value` (D-a): non-submit input `defaultValue = "x"` → `FCS.{default_value,value} == "x"` unchanged post-deletion; Submit/Reset empty edge covered by `e5c` (`reconciler.rs:624`) which now stands un-clobbered.
   - input `default_checked` (D-b): `defaultChecked = true/false` → `default_checked` matches pre-deletion; `form.reset()` after `setAttribute("checked")` restores checked.
   - **textarea `defaultValue` documentation test** (the §4-E/§6-F deferred gap): a test asserting `textarea.defaultValue = "x"` updates `FCS.default_value` (via the *retained* mirror) — annotated that this works **because** the mirror stays, and that removing it awaits `#11-textarea-defaultvalue-textcontent-reconciliation`. This keeps the deferred gap visible and prevents a future "delete all mirrors" sweep from silently breaking it.

`cargo test -p elidex-form-core -p elidex-form -p elidex-js --all-features` + `mise run ci` (3-OS check / clippy / nextest / doc `-D warnings` / deny) green.

---

## §9 1000-line touch-time note (tests extracted to a sibling module)

Post-implementation counts (`wc -l`): `reconciler.rs` (production + `#[path]` stub) = **333**, `reconciler_tests.rs` (extracted) = **735**, `elidex-form-core/src/lib.rs` = **987**, `clone.rs` = 304, `sizing.rs` = 139, `html_input_value.rs` = 543, `html_textarea_proto.rs` = 851 — **all < 1000**.

- **Production stays a ~333-line flat case-table (exempt from a split).** The three arms add only a few lines to `handle_attribute_change`'s per-attribute match — a cohesive dispatch (the touch-time discipline's explicitly non-splittable "flat case table"), so the *production* half is not a split candidate. The two mirror deletions **remove** lines from `html_input_value.rs`; `parse_positive_with_fallback` (~14 lines incl. doc) lands in `elidex-form-core/src/lib.rs` (987, still < 1000); with decision E deferred (§5) there is no `adopt_value_state` growth.
- **The added tests forced a tests-only split (this note's earlier "no split needed" premise was falsified — it under-counted the tests).** The new arm / mirror-deletion / parity / reset tests grew the inline `#[cfg(test)] mod tests` module enough that the *file total* crossed 1000 (~1063). Per the touch-time discipline (the touched file must stay bounded; test files are in-scope on the same basis as source), the test module is extracted **verbatim** to `reconciler_tests.rs` and included via `#[cfg(test)] #[path = "reconciler_tests.rs"] mod tests;` — the crate's existing `submit.rs` / `submit_tests.rs` (and `lib_tests.rs`) idiom. **No production-code split** — the arms stay in the flat case-table; the split is tests-only, a pure relocation (identical test count/names).
- If `elidex-form-core/src/lib.rs` (987, closest to the cap) is *ever* split, the cleanest cohesion seam is the `FormControlKind` enum + its inherent `impl` (`lib.rs:67-326`, per the 0a move-list line ranges) → its own module. **0b does not need it and must not bundle it** — stated here so plan-review does not demand a speculative split for a small touch.

---

## §10 Open / judgment items (for `/elidex-plan-review`)

- **J-B (headline) — is `default_checked`-only correct for the `checked` arm, or must it also set live `checked` for a "provably non-dirty" init-adjacent control?** Decision B says default-only (§B), grounded in §4.10.5 prose (the content attribute *always* gives default checkedness; live checkedness is gated on the dirty checkedness flag, unmodeled until Slice 4). The counter-case: `el = createElement("input"); el.type="checkbox"; el.setAttribute("checked","")` is *definitionally* not-dirty, so the spec wants live checkedness = true, and 0b leaves `el.checked` false. **Recommendation: default-only.** Without modeling the dirty checkedness flag, "provably non-dirty" cannot be distinguished at runtime from "user-toggled then setAttribute" (which must *not* have live checkedness clobbered); setting it unconditionally introduces a new clobber bug. The create-time not-dirty case is already handled by `from_input_element` at init. Slice 4 adds the flag and the live-set gate. This matches umbrella §8 ("Slice 1 asserts defaultChecked restoration only; full dirty-checkedness-flag interaction deferred to Slice 4").
- **J-C — init/reconciler parity for `rows`/`cols` (the `rows="0"` divergence) — DECIDED (shared helper).** The arm applies the §2.6.1 reflection (`0`/invalid/absent → default 2/20), but `from_textarea_element:941-942` uses a plain `u32` parse (`"0".parse()` = `Ok(0)` → `rows=0`), so init and the arm would disagree for `rows="0"` — two maintainers producing different FCS for the same attribute, the exact thing the umbrella §1 abolishes. **Decision: `parse_positive_with_fallback` is authored in `elidex-form-core` and used by *both* `from_textarea_element` and the rows/cols arm** (single-sourced reflection). This is a ~+2-line init touch that also fixes the latent init bug, squarely serving the single-maintainer ideal ("Ideal over pragmatic"). No arm-only alternative — leaving init on the plain parse would re-open the divergence.
- **J-E — decision E is falsified; recommend DROP (defer).** The `value`/`dirty_value` `pub → pub(crate)` revert is infeasible (§5: four STAY-UP systems raw-write `value`), and a lone `adopt_value_state` is a One-issue-one-way strangler + J1-bending-for-no-gain. **Recommendation: close 0a-J5 as "`pub` is the correct ECS-native visibility"; if the char_count-coupling invariant warrants encapsulation, do it uniformly via `#11-form-value-write-api-unification`, not here.** Review may override to keep a scoped invariant-guard-only method (not recommended).
- **J-F — the new textarea slot name.** `#11-textarea-defaultvalue-textcontent-reconciliation` (decision F). Alternatives: `#11-reconciler-childtext-subscription` (mechanism-named) or `#11-textarea-rawvalue-fcs-sync`. Recommend the first (names the observable gap); confirm.
- **J-rows-gate — should the `rows`/`cols` arm gate on `kind == TextArea`?** Recommend **no** (matches `size`/`min|max|step`; inert on non-textarea). Confirm.

---

## §11 Relationship to the umbrella (and the §4/§8 corrections it implies)

- **Umbrella §4 "IDL hand-mirrors … Remove — the chokepoint already covers them once the arms exist"** lists all three mirrors (`html_input_value.rs:206-211` `default_value`, `:278-280` `default_checked`, `html_textarea_proto.rs:654-661`). **This is correct for the two input mirrors and wrong for the textarea one** (§4-E/§6-F: `textarea.defaultValue` is child-text-derived, not attribute-derived, so the AttributeChange-only reconciler cannot cover it). The umbrella is **already committed on `main`** (#472 `3e797897`), so "fix at umbrella landing" is foreclosed — **0b's PR carries the umbrella doc-edit as a concrete deliverable**: edit `docs/plans/2026-07-form-derived-state-reconciliation.md` §4 (≈:148) to "delete the **2 INPUT** mirrors; the **textarea** `default_value` mirror STAYS (textContent-derived, unreachable by the AttributeChange reconciler — see slot `#11-textarea-defaultvalue-textcontent-reconciliation`)."
- **Umbrella §6 Slice-0b row** ("missing `checked`/`rows`/`cols` arms; delete the IDL hand-mirrors") — **0b's PR also edits this row** (≈:191) to "delete the **two input** IDL hand-mirrors" (the textarea mirror stays), consistent with the §4 edit. Both umbrella doc-edits ship **in 0b's PR** (not deferred).
- **Umbrella §8** ("`form.reset()` → `:checked` follows `defaultChecked`; Slice 1 asserts defaultChecked restoration only; full dirty-checkedness-flag interaction deferred to Slice 4") is **already correct** and is exactly decision B's contract — 0b supplies the `default_checked` reconciliation that makes that Slice-1 assertion pass. No §8 change needed for 0b; 0b adds the §8-2 (`rows`/`cols`) and reset (`default_checked`) tests to the engine-independent layer.
- **0a hand-offs discharged**: 0a-J5's "residual `value`/`dirty_value` widening, left to Slice 0b" is **scoped here** (§5) — the plan-review's disposition (drop/defer per J-E) *is* the discharge. 0a-J6's `replace_selection` merge and 0a-J1's setter caveat are **not** 0b concerns (behavior decisions / already-resolved).
