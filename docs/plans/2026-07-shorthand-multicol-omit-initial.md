# Plan: `column-rule` / `columns` shorthand serialization — omit-initial (PR1)

Per-family **PR1** under the umbrella `#11-style-shorthand-expand`. The
**foundational PR #468** (landed) added the trait slot
`CssPropertyHandler::serialize_shorthand`, the `elidex-style`
`serialize_shorthand_value` coordinator (property-agnostic CSSOM §6.6.1 checks +
first-longhand handler dispatch), and the shared *structural* helpers
`serialize_rectangular` / `serialize_axis_pair` in `elidex-plugin`. This PR covers
the **first omit-initial family**: `MulticolHandler::serialize_shorthand` for
`column-rule` and `columns`.

The foundational plan
(`docs/plans/2026-07-shorthand-serialization-handler-dispatch.md`, §2) explicitly
deferred the omit-initial families and stated *each per-family PR owes its own §2
corner matrix*. That matrix — verified corner-by-corner against the spec and a live
engine — **is** the substance of this memo.

## Problem (design-level, not a patch)

The parser expands both shorthands to **all** longhands, defaulting omitted
components to their initial values
(`parse_column_rule_shorthand` / `parse_columns_shorthand`,
`crates/css/elidex-css/src/declaration/misc.rs:614,661`):

- `column-rule: solid` ⇒ `column-rule-width: 3px` (medium) `; column-rule-style: solid ; column-rule-color: currentcolor`
- `columns: 200px` ⇒ `column-width: 200px ; column-count: auto`

So the declaration block **always stores all longhands** as already-serialized
strings. To round-trip `getPropertyValue("column-rule")` back to `"solid"` (not
`"3px solid currentcolor"`), the serializer must **omit** each component equal to
its initial value (CSSOM §6.7.2 "serialize a CSS value", step 2:
> *"If component values can be omitted … without changing the meaning of the value,
> omit … them."* — https://drafts.csswg.org/cssom-1/#serialize-a-css-value)

This is fundamentally different from the structural families the foundational PR
moved: `serialize_rectangular` / `serialize_axis_pair` collapse by **positional
equality** (top==bottom, x==y) and need **no** initials. Omit-initial needs each
longhand's **initial value**, whose single source of truth is
`CssPropertyHandler::initial_value` — the handler's own SoT. Duplicating initials
anywhere else violates the contract the `serialize_shorthand` docstring states
(`crates/core/elidex-plugin/src/traits.rs:137`). Hence: **handler-owned, initials
from `self.initial_value`, never duplicated.**

## §2. Coupled invariants + the corner matrix

The genuinely-coupled invariants (the reason this PR needs plan-review, per the
mission edge-dense rule):

- **I-A Omit-initial** — a component is emitted iff its stored serialization ≠ its
  initial serialization (CSSOM §6.7.2 step 2).
- **I-B Canonical order** — kept components are emitted in the property's canonical
  grammar order (`column-rule` = width → style → color; `columns` = width → count;
  both "Canonical order: per grammar",
  css-multicol-1 §4.5 / §3.3). Both grammars are `||` (double-bar), so §6.7.2 step 2
  also says to *reorder to canonical order* — but since the block already stores
  longhands in canonical order, the operative sub-rule is **do not re-order the
  survivors when a middle component is dropped** (the "gap" corner).
- **I-C Per-family initials SoT** — initials come only from `self.initial_value(name)`
  (§3.1 `column-width`=auto, §3.2 `column-count`=auto, §4.2 `column-rule-color`=
  currentcolor, §4.3 `column-rule-style`=none, §4.4 `column-rule-width`=medium),
  never duplicated in the collapse logic.
- **I-D Non-empty guarantee (emergent)** — a shorthand value must be a *valid* value;
  the empty string is not one. When **every** component is initial, I-A alone would
  omit all of them and yield `""` — which §6.7.2 step 2 forbids
  (> *"If either of the above syntactic translations would be less
  backwards-compatible, do not perform them."*). This is where I-A, I-B and I-C
  **intersect at a real corner** and force a rule the spec does not spell out
  (§6.7.2 note: *"For legacy reasons, some properties serialize in a different
  manner, which is intentionally undefined here …"*). Resolved by observation of a
  live engine — see Corner 3 / Corner 4.

The intersections are **not** weak (contrast the foundational PR's relocation): the
gap corner couples I-A×I-B, and the all-initial corner couples I-A×I-B×I-C×I-D.

### Verification method

All "elidex out" cells are computed from the stored `CssValue` → `to_css_string()`
(the exact strings the coordinator feeds the handler; see §3.1). All "Blink out"
cells are **measured** in Chrome 148 (Blink) via
`el.style.cssText = <author> ; el.style.getPropertyValue(<shorthand>)` on
`https://example.com` (page-independent; CSSOM is engine-level). Spec is
underspecified for the all-initial corners (I-D), so Blink is the cited source
there.

> **Independently re-verified** (Chrome 148.0.7778.271 / Blink): every "Blink out"
> cell above was re-measured before plan-review. The load-bearing I-D corners hold
> exactly — `column-rule: medium none currentcolor` → `"medium"` (keep FIRST = width,
> *not* the style `none` or color `currentcolor`), and `columns: auto auto` → `"auto"`
> (single, not `auto auto`). "keep-first" is thus confirmed as keep-**first-canonical**,
> disambiguated by the `column-rule` all-initial case returning the width keyword
> specifically.

### `column-rule` — canonical order: width, style, color

| # | Author decl | stored width / style / color | non-initial set | **elidex out** | **Blink out** | resolution |
|---|---|---|---|---|---|---|
| 1 | `column-rule: solid` | `3px` / `solid` / `currentcolor` | {style} | `solid` | `solid` | I-A: drop width+color (both initial), keep style |
| 2 | `column-rule: thick blue` | `5px` / `none` / `#0000ff` | {width, color} — **style-gap** | `5px #0000ff` | `thick blue` | I-A×I-B: drop the **middle** (style=none=initial), keep width then color **in order** (no re-sort) |
| 2b | `column-rule: dashed green` | `3px` / `dashed` / `#008000` | {style, color} | `dashed #008000` | `dashed green` | I-A: leading width omitted, survivors in order |
| 3 | `column-rule: medium none currentcolor` | `3px` / `none` / `currentcolor` | **{} — ALL INITIAL** | `3px` | `medium` | **I-D**: omit-all → `""` is invalid → keep the **first** canonical component (width). Structure matches Blink; the `3px` vs `medium` **string** divergence is a pre-existing width-keyword issue (see Risks) |
| 3b | `column-rule: thick solid red` | `5px` / `solid` / `#ff0000` | {all} | `5px solid #ff0000` | `thick solid red` | all kept, canonical order (component-value forms diverge; see Risks) |

### `columns` — canonical order: width, count

The task flagged `columns` as *possibly* not a clean omit-initial family (grammar
`<'column-width'> || <'column-count'>`, both initial `auto`). **Verified answer: it
IS a clean omit-initial family, using the *identical* rule as `column-rule`,
including the same I-D keep-first fallback.** No separate rule is needed.

| # | Author decl | stored width / count | non-initial set | **elidex out** | **Blink out** | resolution |
|---|---|---|---|---|---|---|
| 4 | `columns: auto` **and** `columns: auto auto` | `auto` / `auto` | **{} — ALL INITIAL** | `auto` | `auto` | **I-D**: same keep-first fallback → single `auto`, **not** `auto auto`. (`columns: auto` ≡ `columns: auto auto`, css-multicol-1 §3.3 example table) |
| 5 | `columns: 200px` | `200px` / `auto` | {width} | `200px` | `200px` | I-A: drop count (auto=initial) |
| 5b | `columns: 3` **and** `columns: 3 auto` | `auto` / `3` | {count} | `3` | `3` | I-A: drop width (auto=initial); `Number(3.0)`→`"3"` (no `3.0`) |
| 5c | `columns: 200px 3` | `200px` / `3` | {both} | `200px 3` | `200px 3` | both kept, canonical order width→count |

### The single rule both families obey

Omit each component whose stored serialization equals its initial serialization;
emit the survivors in canonical order joined by `" "`; **if the survivor set is
empty (all-initial), keep the first canonical component** (so the result is never
`""`). This one rule, driven only by the passed longhand order + `self.initial_value`,
reproduces **every** Blink structural result above — for both `column-rule` and
`columns`. The two `match` arms therefore share one body.

## Ideal architecture (plugin-first, one-issue-one-way)

Since `column-rule` **and** `columns` need byte-identical logic *in this PR*, and
`flex-flow` / `text-decoration` / `border` (PR2–4) will need the same, the
omit-initial collapse is a **single canonical form**, not per-handler copies
(one-issue-one-way). Split responsibility exactly like the structural helpers:

- **Shared helper** (`elidex-plugin`, alongside `serialize_rectangular` /
  `serialize_axis_pair` in `crates/core/elidex-plugin/src/shorthand.rs`) — pure,
  bakes in **no** initials:

  ```rust
  /// Collapse an omit-initial `||` shorthand from ordered
  /// `(serialized-value, serialized-initial)` component pairs (CSSOM §6.7.2).
  /// Third shared shorthand-collapse helper (with `serialize_rectangular` /
  /// `serialize_axis_pair`) for the omit-initial families under slot
  /// `#11-style-shorthand-expand`.
  /// Omit each component equal to its initial; join survivors with " " in the
  /// given (canonical) order. When ALL are initial, keep the FIRST component —
  /// omitting all would yield "" (invalid / "less backwards-compatible",
  /// §6.7.2 step 2). Verified vs Blink: `column-rule: medium none currentcolor`
  /// ⇒ first = width; `columns: auto auto` ⇒ first = width.
  pub fn serialize_omit_initial(components: &[(&str, &str)]) -> Option<String> {
      if components.is_empty() {
          return None; // defensive; the coordinator always supplies the full set
      }
      let kept: Vec<&str> = components
          .iter()
          .filter(|(value, initial)| value != initial)
          .map(|(value, _)| *value)
          .collect();
      Some(if kept.is_empty() {
          components[0].0.to_string()
      } else {
          kept.join(" ")
      })
  }
  ```

  The helper receives the initials from the caller — it never knows any property's
  initial (I-C preserved: zero duplication). This mirrors `serialize_rectangular`
  receiving the four side values.

- **Handler** (`MulticolHandler::serialize_shorthand`,
  `crates/css/elidex-css-multicol/src/lib.rs`) — owns *which* shorthands it serves,
  their canonical order (implicit in the passed `longhands`), and their initials
  (its own `initial_value`):

  ```rust
  /// Omit-initial shorthand serialization (CSSOM §6.7.2) for the Multicol
  /// family — first per-family increment under slot `#11-style-shorthand-expand`.
  fn serialize_shorthand(&self, property: &str, longhands: &[(&str, &str)]) -> Option<String> {
      match property {
          // Both are omit-initial `||` shorthands over their (already
          // canonical-ordered) longhands — one body, no per-family branch.
          "column-rule" | "columns" => {
              let initials: Vec<String> = longhands
                  .iter()
                  .map(|(name, _)| self.initial_value(name).to_css_string())
                  .collect();
              let components: Vec<(&str, &str)> = longhands
                  .iter()
                  .zip(&initials)
                  .map(|((_, value), initial)| (*value, initial.as_str()))
                  .collect();
              elidex_plugin::serialize_omit_initial(&components)
          }
          _ => None,
      }
  }
  ```

  The `match` guard keeps the handler opting **in** only to shorthands it owns
  (anything else → `None` → CSSOM-valid `""`). The dispatch is already sound: the
  coordinator resolves the owner via the shorthand's **first longhand**
  (`column-rule` → `column-rule-width`, `columns` → `column-width`), both of which
  `MulticolHandler` owns (`MULTICOL_PROPERTIES`), so both shorthands land here.

**Why not reuse `serialize_axis_pair` for `columns`?** It collapses on
*positional equality* (`first == second` ⇒ one value) with no notion of initials.
For `columns: 200px 3` (unequal) it would emit `200px 3` (correct by luck), but for
`columns: 3 auto` (width=auto, count=3, unequal) it would emit `auto 3` — **wrong**
(must be `3`, dropping the initial width). `columns` is omit-initial, not axis-pair;
they only coincide when neither component is initial.

### Why the shared helper now (not deferred to PR2)

Two consumers exist **in this PR** (`column-rule`, `columns`); writing the loop
twice inside one handler violates one-issue-one-way at the file level. The I-D
keep-first fallback is subtle and load-bearing — centralizing it once means every
future omit-initial family (`flex-flow` / `text-decoration` / `border`) inherits the
correct all-initial behavior for free, instead of each re-deriving it. This is
"ideal over pragmatic": the canonical form is justified by ≥2 present uses, not
speculation. *(Flagged for plan-review — see Open questions Q2.)*

## Scope

**PR1 covers**

- `elidex-plugin`: add `serialize_omit_initial` (NEW) helper (+ unit tests).
- `elidex-css-multicol`: implement `MulticolHandler::serialize_shorthand` (NEW) for
  `column-rule` | `columns` (+ handler-level corner tests).
- `elidex-style` (or `elidex-css-multicol` test module): coordinator-level tests
  asserting the inline path and rule path both reconstruct the shorthand.

**PR1 does NOT touch**

- **No `crates/dom` changes.** The coordinator (`serialize_shorthand_value`) and its
  two callers (`elidex-dom-api/src/style.rs:73`, `cssom_sheet.rs:614`) were fully
  wired by the foundational PR; this PR only flips `MulticolHandler::serialize_shorthand`
  from the default `None` to `Some(...)`. So PR1 is **in-lane only** (contrast the
  foundational PR, which touched dom-api).
- No parser changes (`parse_column_rule_shorthand` / `parse_columns_shorthand`
  already expand correctly).
- No change to component-value serialization (`to_css_string`) — the width-keyword
  and named-color divergences (Risks §R2) are pre-existing and explicitly out of
  scope; PR1 asserts elidex's honest current output.

## §3. Spec coverage map

| Spec section | Step / branch | Touch site | Full enum? | User-input flow |
|---|---|---|---|---|
| CSSOM 1 §6.7.2 Serializing CSS Values (`#serialize-a-css-value`) | omit-initial (step 2) + canonical order for `\|\|` + non-empty ("less backwards-compatible" caveat) | `serialize_omit_initial` (elidex-plugin) | ✓ (all 5 corners + gap + all-initial) | no (read path; operates on already-serialized, already-validated longhand strings) |
| css-multicol-1 §4.5 `column-rule` (`#propdef-column-rule`) | shorthand grammar `<'column-rule-width'> \|\| <'column-rule-style'> \|\| <'column-rule-color'>`, canonical order per grammar | `MulticolHandler::serialize_shorthand` arm | ✓ | no |
| css-multicol-1 §3.3 `columns` (`#propdef-columns`) | shorthand grammar `<'column-width'> \|\| <'column-count'>`, canonical order per grammar | same handler arm | ✓ | no |
| css-multicol-1 §3.1/§3.2/§4.2/§4.3/§4.4 (longhand `Initial:` rows) | initials (auto / auto / currentcolor / none / medium) | `MulticolHandler::initial_value` (unchanged SoT) | ✓ | no |
| CSSOM 1 §6.6.1 (`getPropertyValue`: all-present + uniform-`!important`) | upstream gate | `serialize_shorthand_value` coordinator (unchanged) | ✓ (already covered by #468) | no |

**Breadth**: K=2 specs (cssom-1, css-multicol-1), M=5 entries → **single-PR scope**
(well under K≥4 / M≥20). The coupling density (I-D corner), not breadth, is what
mandates plan-review.

### §3.1 User-input touch audit

No user-controllable input reaches new logic. The `property` argument
(`getPropertyValue(property)`) only indexes the static `shorthand_longhands` table +
the longhand-keyed registry `resolve` (both total; unknown → `None` → `""`). The
collapse consumes **already-serialized, already-validated** longhand strings:

- **Inline path** (`style.rs:73`): strings come from `InlineStyle::get(lh)`, which
  stores `serialize_declaration_value_for_storage(...)` — and that returns
  `CssValue::to_css_string()` **verbatim** for every non-`List` value
  (`crates/css/elidex-css/src/declaration/cssom.rs:168-170`). All multicol longhands
  are `Length` / `Keyword` / `Color` / `Auto` / `Number` (never `List`), so the
  stored string is exactly `to_css_string()`.
- **Rule path** (`cssom_sheet.rs:614`): strings come from `Declaration.value.to_css_string()`.

Both feed the same helper; the helper does pure string equality + join. No
injection / cycle / re-parse surface (the collapse never re-enters the parser).
`!important` never reaches the handler — the coordinator strips priority and rejects
mixed-priority blocks (§6.6.1) before dispatch.

## §3.2 Canonicalization soundness (Corner 6)

**The initial-detection comparison is sound** — `value != initial` compares two
strings produced by the **same** `to_css_string`:

| longhand | initial `CssValue` | `initial_value(...).to_css_string()` | stored string for the initial case | equal? |
|---|---|---|---|---|
| column-rule-width | `Length(3.0, Px)` | `3px` | `3px` (medium→`Length(3,Px)`→`3px`) | ✓ |
| column-rule-style | `Keyword("none")` | `none` | `none` | ✓ |
| column-rule-color | `Keyword("currentcolor")` | `currentcolor` | `currentcolor` (parser lowercases the keyword, `elidex-css/src/lib.rs:65`) | ✓ |
| column-width | `Auto` | `auto` | `auto` | ✓ |
| column-count | `Auto` | `auto` | `auto` | ✓ |

No case/unit mismatch: the width keyword `medium` and the color keyword
`currentColor` are both normalized to their canonical `to_css_string` form at parse,
so both sides of the comparison agree. See Risks §R2 for the *output-string*
divergences (which do **not** affect detection).

## Risks

- **R1 (RESOLVED-SAFE) — initial detection.** Sound as shown in §3.2: stored value,
  initial value, and the comparison all route through the identical `to_css_string`.
  No canonicalization gap in the omit-initial decision.

- **R2 (DIVERGENCE, pre-existing, orthogonal) — component-value output forms.**
  elidex serializes some component *values* differently from Blink because the
  longhand value model resolves keywords/colors eagerly at parse:
  - Border-width keyword: `medium`/`thin`/`thick` → `Length(3/1/5, Px)` →
    `3px`/`1px`/`5px` (`declaration/misc.rs:648,703`), vs Blink's declared-value
    `medium`/`thin`/`thick`. Surfaces in Corners 2, 2b, 3, 3b.
  - Named color: `red` → `CssColor{255,0,0}` → `#ff0000` (`values.rs:816`), vs Blink's
    declared `red`. Surfaces in Corners 2, 2b, 3b.

  These are **inherited from the longhand `to_css_string`, not introduced by the
  collapse** — `el.style.columnRuleWidth = 'medium'` already reads back `3px` today.
  Per CSSOM §6.7.2 a **keyword** serializes as itself (Blink is spec-correct; elidex
  has a latent declared-value gap in its width/color value model). The omit-initial
  **structure** still matches Blink exactly. **Decision for PR1**: assert elidex's
  honest current output (`5px`, `#ff0000`, …), do **not** special-case the width or
  color in the shorthand path (that would re-invent longhand serialization inside the
  collapse, violating I-C + the layering mandate). Carved to the deferred slot
  **`#11-css-declared-value-serialization-fidelity`** (declared-value keyword/color
  preservation in the border-width & color value model; affects `border-*-width` and
  every color longhand, not just multicol) — see Open questions Q1 for the ratified
  slot definition + registration.

- **R3 (checked, no divergence) — number formatting.** `column-count: 3` stores
  `Number(3.0)`; `to_css_string` = `format!("{n}")` on `f32` → `"3"` (Rust prints
  `3.0f32` as `3`), matching Blink. Assert `"3"` (not `"3.0"`) in tests to lock it.

## Test plan (ADDS coverage — NOT behavior-preserving)

New tests assert the exact elidex output for every corner (elidex's honest strings,
per R2). Behavior is *new* (the handler previously returned `None` → `""`), so these
are net-new assertions, not a golden-oracle diff.

- **`serialize_omit_initial` unit tests** (`elidex-plugin`):
  - all-initial → keep first: `[("3px","3px"),("none","none"),("currentcolor","currentcolor")]` → `"3px"`.
  - gap: `[("5px","3px"),("none","none"),("#0000ff","currentcolor")]` → `"5px #0000ff"`.
  - one non-initial: `[("3px","3px"),("solid","none"),("currentcolor","currentcolor")]` → `"solid"`.
  - all non-initial → full join in order.
  - two-component all-initial (`columns`): `[("auto","auto"),("auto","auto")]` → `"auto"`.
  - empty slice → `None` (defensive).
- **`MulticolHandler::serialize_shorthand` corner tests** (`elidex-css-multicol`),
  one per matrix row (Corners 1–5c), asserting the "elidex out" column, plus:
  - unknown property (e.g. `"margin"`) → `None`.
- **Coordinator round-trip tests** (both surfaces reconstruct):
  - inline path: `el.style.cssText = "column-rule: thick blue"` then
    `getPropertyValue("column-rule")` → `"5px #0000ff"`; `columns: auto auto` → `"auto"`.
  - rule path (`cssom_sheet` `RuleStyleGetPropertyValue`): same author decls via a
    parsed rule → same output (locks that both `get` closures feed identical strings).
  - uniform-`!important` gate (upstream): a block with mixed priority on the
    `column-rule-*` longhands → coordinator returns `None` → `""` (asserts the
    handler is never reached for a mixed block).
- `cargo test -p elidex-plugin -p elidex-css-multicol -p elidex-style --all-features`;
  full `mise run ci` before push.

## Lane / coordination

- **OWN = crates/css/\*\* + elidex-style.** `MulticolHandler` lives in
  `crates/css/elidex-css-multicol`; handler + coordinator tests are in-lane.
- **`elidex-plugin` (helper)** is `crates/core/elidex-plugin` — as the foundational
  plan noted, treat as in-scope (mission-analogous to the declared CSS surface /
  structural helpers already homed there) but **flag to PM**.
- **No cross-lane (`crates/dom`) edits** — the coordinator + its two callers are
  already wired (#468). Nothing to coordinate with the dom lane for this PR.
- No collision risk with the active CSS-style lane work beyond the shared
  `shorthand.rs` (append-only helper add).

## Edge-density → plan-review

Intersecting axes: omit-initial × canonical-order × per-family-initials-SoT ×
non-empty-fallback (I-A×I-B×I-C×I-D at the all-initial corner) × the shared-helper
placement decision. Per the mission rule and the foundational plan's explicit
hand-off, **run `/elidex-plan-review` on this memo before implementing.**

## Open questions (could not be closed from the spec alone)

- **Q1 — Corner 3 output `3px` vs Blink `medium` (and `#ff0000` vs `red`).** The
  omit-initial *structure* is verified against Blink, but the component *value*
  strings diverge because elidex's longhand value model resolves the width keyword →
  length and the named color → hex at parse (R2). CSSOM §6.7.2 makes Blink correct
  (a keyword serializes as itself). **Options**: (a) accept + document + assert
  elidex's px/hex output in PR1 (recommended — the divergence is pre-existing and
  orthogonal to omit-initial), carving a separate deferred slot for declared-value
  keyword/color preservation in the width & color value model; or (b) block PR1 on
  that value-model fix (rejected — far larger blast radius: `border-*-width`, every
  color longhand). *Not resolvable from the multicol/CSSOM spec — it is an elidex
  value-model decision.*
  **→ Plan-review RATIFIED option (a)** (2026-07-16). New deferred slot
  **`#11-css-declared-value-serialization-fidelity`**:
  - *Why deferred*: elidex resolves specified-value keyword widths
    (`medium`/`thin`/`thick`→`Length(_,Px)`, `misc.rs:648,703`) and named colors
    (`red`→`#ff0000`, `values.rs:816`) at **parse** time, losing the declared form
    that CSSOM §6.7.2 serializes verbatim (a keyword serializes as itself). Fixing it
    = a specified-value-retention value-model change spanning `border-*-width` + every
    named-color longhand — orthogonal to shorthand collapse, and bundling it here would
    re-invent longhand serialization inside the collapse (violates I-C + layering).
  - *Re-evaluation trigger*: a declared-value / `cssText` fidelity WPT or compat case,
    or the specified-value serialization model being prioritized on its own.
  - *Re-evaluation date*: 2026-10-31 backstop.
  - *Registration*: **register at PR1 landing** (ship-time, per the defer-lifecycle
    convention); PR1 asserts elidex's honest current output (`5px`/`#ff0000`) so this
    slot is the single tracked home for the divergence.

- **Q2 — Shared helper now vs deferred.** Land `serialize_omit_initial` in
  `elidex-plugin` in PR1 (two property consumers already, subtle I-D fallback
  centralized once). The foundational contract docstring phrases omit-initial families
  as "compare each value against `initial_value`" (readable as handler-local). The
  split keeps the *comparison inputs* (initials) sourced from the handler's SoT while
  centralizing only the *pure structural loop* — so it satisfies both readings.
  **→ Plan-review RATIFIED helper-in-PR1** (2026-07-16): it completes the
  `elidex-plugin` structural-helper trilogy (`serialize_rectangular` /
  `serialize_axis_pair` / `serialize_omit_initial`) at the correct tier from the start,
  and centralizes the subtle I-D keep-first fallback ONCE so PR2–4 (`flex-flow` /
  `text-decoration` / `border`) inherit correct all-initial behavior rather than each
  re-deriving it (a strangler + divergence risk). Note (Axis 2): PR1 has a single
  *call site* (the merged `"column-rule" | "columns"` arm); the cross-handler second
  consumer arrives at PR2 — but placing the helper in the base tier now is the
  one-issue-one-way choice, not speculative (the ≥5 omit-initial families are already
  committed by the foundational staging).

- **Q3 (minor) — future family generality.** The keep-first I-D fallback is verified
  only for `column-rule` / `columns` (first = width in both). `flex-flow` (initial
  `row nowrap`) and `text-decoration` (initial `none solid currentcolor`) should be
  re-verified against Blink at their PRs — the *rule* is expected to hold (keep first
  canonical component) but each family's all-initial output must be measured, not
  assumed. Noted so PR2–4 carry the same corner-matrix discipline.

- **Q4 (PR2 refactor consideration) — helper interface: pre-paired slice vs
  initial-getter closure.** PR1 ships `serialize_omit_initial(&[(value, initial)])`,
  so each handler pre-builds the `(value, initial)` pairs — a 3-line gather+zip
  (`longhands.iter().map(initial_value).collect()` + a `zip`). `/simplify` flagged
  (LOW/judgment) that this plumbing will repeat in every PR2–4 omit-initial handler,
  and proposed a closure form `serialize_omit_initial(longhands, |n| self.initial_value(n).to_css_string())`
  that drops each handler to a sibling-matching one-liner (and restores the
  `(longhand-name, value)` tuple order the structural siblings use). **Deferred to PR2
  (not applied in PR1)**: the pre-paired form has the cleanest literal-slice unit tests
  and PR1 has only one call site, so extracting the closure form is best done when the
  **second concrete consumer (`flex-flow`) lands** and can inform the exact interface —
  rather than speculatively generalizing on one use. PR2 should evaluate the extraction
  (it would touch this PR1 handler); flagged here so it is not lost.
