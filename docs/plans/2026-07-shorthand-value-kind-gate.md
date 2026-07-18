# Plan: shorthand serialization → property-agnostic value-KIND gate

Prerequisite under `#11-style-shorthand-expand` (the handler-owned shorthand
serialization umbrella; foundational coordinator landed #468 `769aa49c`). This
memo adds the **CSSOM §6.7.2 step 1.2 value-KIND gate** to the coordinator
`elidex_style::serialize_shorthand_value` (`crates/css/elidex-style/src/lib.rs`).
It **fixes a real, confirmed latent bug in the 6 families #468 already ships** and
is the **prerequisite that makes PR1 (Multicol `column-rule`/`columns`) correct**.

Design direction PM-anchored: *property-agnostic value-KIND gate in the
coordinator, running after the §6.6.1 all-present + uniform-`!important` checks and
before the per-family collapse dispatch.* This memo grounds every corner against
the live parser/serializer (throwaway probe, since removed) and live Blink
(Chrome 148.0.7778.271), and resolves the one genuinely family-dependent corner.

---

## Problem (design-level, not a patch)

`serialize_shorthand_value` performs the two **property-agnostic** CSSOM §6.6.1
`getPropertyValue` shorthand checks — (1) every mapped longhand present, (2)
`!important` flags uniform — then dispatches the per-family **collapse** (CSSOM
§6.7.2 "serialize a CSS value") to the owning
`CssPropertyHandler::serialize_shorthand`. The collapse helpers
(`elidex_plugin::serialize_rectangular` / `serialize_axis_pair`,
`crates/core/elidex-plugin/src/shorthand.rs`) are **pure string-equality**
reducers: `[t,r,b,l]` → shortest form by comparing the serialized strings.

**The coordinator is missing CSSOM §6.7.2 step 1.2** (verified
`.claude/tools/webref body cssom-1 serialize-a-css-value`):

> 1.2 If there is no such shorthand or **shorthand cannot exactly represent the
> values of all the properties in list, return the empty string.**

When a component longhand holds a value whose **kind** the shorthand grammar
cannot represent alongside its siblings — a **CSS-wide keyword** (css-cascade-4
§7.3 *Explicit Defaulting*: `initial`/`inherit`/`unset`; `revert` is cascade-
dependent) or an **unresolved `var()`** (css-variables-1 §3 / §2.2 — a var-
carrying value is not substituted until computed-value time, so its specified-
value serialization is not a concrete component) — the string-equality helpers
treat those strings (`"initial"`, `"var(--x)"`) as ordinary component values and
emit **invalid, non-round-tripping shorthand serializations** instead of `""`.

This is reachable from ordinary author CSS: the parser expands a shorthand
CSS-wide keyword into **per-longhand** CSS-wide keyword declarations *before*
shorthand grammar parsing (`declaration.rs:187-193` "Check for global keywords
first" → `expand_global_keyword` `declaration.rs:826`), and a longhand `var()`
is stored as `CssValue::Var` / a `RawTokens("var(…)")` string
(`declaration.rs:197-223`). Both serialize (via
`CssValue::to_css_string`, `elidex-plugin/src/values.rs:585`) to the literal
strings the helpers then mis-collapse.

**The gap affects ALL families the coordinator serves, not just PR1's Multicol.**
The 6 families #468 landed — `margin`/`padding`/`border-radius` (rectangular),
`gap`/`overflow` (Box axis-pair), `border-spacing` (Table axis-pair) — carry the
identical latent bug today (measured below).

### Confirmed via throwaway probe (elidex, then removed)

- `margin-top:initial; margin-right:5px; margin-bottom:initial; margin-left:5px`
  → elidex **`"initial 5px"`** (Blink `""`).
- `margin-top:inherit; …:5px …` → elidex **`"inherit 5px 5px"`** (Blink `""`).
- `margin-top:var(--x); …:0px …` → elidex **`"var(--x) 0px 0px"`** (Blink `""`).
- `row-gap:var(--g); column-gap:4px` → elidex **`"var(--g) 4px"`** (Blink `""`).
- `margin-top:initial; margin-right:inherit; …` (mixed different) → elidex
  **`"initial inherit"`** (Blink `""`).

---

## §2. Coupled invariants + corner matrix

Unlike the #468 foundational relocation (single binding invariant), this prereq is
**value-KIND × family-KIND coupled at the corners**, which is why it earns a
plan-review. Invariants:

- **I1 §6.7.2 step 1.2 fidelity** — a component value whose kind the shorthand
  grammar cannot represent → `""` (not a mis-collapse).
- **I2 Property-agnostic single site (One-issue-one-way)** — value-KIND
  classification lives in *exactly one* place (the coordinator), never
  re-implemented per handler. Handlers receive **only all-physical** longhands.
- **I3 Behavior-preservation for physical values** — every existing all-physical
  test collapses byte-identically (the gate falls through untouched).
- **I4 Blink-fidelity vs spec-fidelity** — where Blink is internally inconsistent
  (measured below), the ideal is the **spec-uniform** result; divergences are
  explicit and cited, never accidental.

The genuine coupling is **I1 × I4 at corner 4** (CSS-wide keyword mixed with a
physical value): the *spec* answer is uniform (`""`), but *Blink* answers three
different ways depending on the specific shorthand's hand-written serializer. That
is the crux this memo resolves.

### Value-KIND classes (per component, classified on the serialized string)

`InlineStyle` is a **string-backed** CSSOM store (`InlineDeclaration { value:
String }`, `elidex-ecs/src/components/inline_style.rs:16`) — the inline `el.style`
path can only surface serialized strings; only the rule path holds `CssValue`.
A uniform property-agnostic gate must therefore operate on the **common
denominator = strings** (see *Ideal architecture*). Classes:

- **P** physical — a concrete component value (`5px`, `hidden`, `red`).
- **K** CSS-wide keyword — string ∈ {`initial`,`inherit`,`unset`,`revert`,
  `revert-layer`} (globally reserved; never a valid physical value).
- **V** unresolved var — string contains `var(` (`CssValue::Var` →`"var(--x)"`,
  or a `RawTokens` carrying `var(`).

### Corner matrix (Blink = Chrome 148, measured; elidex-today measured/derived)

| # | Kind pattern | Representative input | **Blink** | **elidex today** | **Target** | Decided by |
|---|---|---|---|---|---|---|
| 1 | all-same **K** | `margin: initial` | `initial` | `initial` (str-eq accident) | `initial` | coord: all-same-K |
| 1 | all-same **K** | `column-rule: initial` | `initial` | `""` (Multicol unserved; PR1-naive → `"initial initial initial"`) | `initial` | coord: all-same-K |
| 1 | all-same **K** | `overflow: initial` / `gap: initial` / `columns: initial` | `initial` | `initial` / `initial` / `""` | `initial` | coord: all-same-K |
| 1 | all-same **K** (longhands) | `margin-top:initial; …:initial ×4` | `initial` | `initial` | `initial` | coord: all-same-K |
| 2 | mixed different **K** | `margin-top:initial; right:inherit; bottom:initial; left:inherit` | **`""`** | **`"initial inherit"`** ✗ | `""` | coord: mixed-K |
| 2 | mixed different **K** | `row-gap:initial; column-gap:inherit` | **`""`** | `"initial inherit"` ✗ | `""` | coord: mixed-K |
| 3 | any **V** | `margin-top:var(--x); right:0; bottom:0; left:0` | **`""`** | **`"var(--x) 0px 0px"`** ✗ | `""` | coord: any-V |
| 3 | any **V** | `row-gap:var(--g); column-gap:4px` | **`""`** | **`"var(--g) 4px"`** ✗ | `""` | coord: any-V |
| 3 | any **V** | `overflow-x:var(--o); overflow-y:hidden` | `""` | `"var(--o) hidden"` ✗ | `""` | coord: any-V |
| 3 | any **V** | `column-width:var(--w); column-count:auto` | `""` | `""` (unserved; PR1-naive → `"var(--w)"`) | `""` | coord: any-V |
| **4** | **K + P** — `inherit`/`unset` + physical | `margin-top:inherit; …:5px` · `overflow-x:inherit; y:hidden` · `column-rule-width:inherit; solid; red` · `gap row:inherit; col:4px` | **`""`** (ALL families) | mis-collapse ✗ / unserved | `""` | coord: K+P |
| **4** | **K + P** — `initial` + physical, **structural** | `margin-top:initial; …:5px` · `padding` · `overflow-x:initial; y:hidden` · `inset` · `columns width:initial; count:3` | **`""`** | `"initial 5px"` ✗ | `""` | coord: K+P |
| **4** | **K + P** — `initial` + physical, **`gap` (Blink quirk)** | `row-gap:initial; column-gap:4px` | **`"initial 4px"`** (⚠ non-round-trip; see below) | `"initial 4px"` (matches quirk) | **`""`** (spec; **divergence from Blink**) | coord: K+P |
| **4** | **K + P** — `initial` + physical, **`column-rule` omit-initial** | `column-rule-width:initial; style:solid; color:red` → Blink `"solid red"`; `column-rule-style:initial; width:2px; color:red` → Blink `"2px red"` | **omit** (`"solid red"`) | `""` (unserved) | **`""`** (under-approx; Blink-faithful omit **deferred**) | coord: K+P |
| 5 | whole-shorthand **V** | `margin: var(--x)` · `columns: var(--w)` | `var(--x)` | **`var(--x)`** ✓ (already correct) | `var(--x)` (unchanged) | caller `.or_else` fallback (gate not reached) |
| 6 | all **P** | `margin-top:1px; right:2px; …` | collapse | collapse ✓ | collapse (unchanged) | handler dispatch |
| — | `revert` (whole) | `margin: revert` | `revert` | **`""`** (elidex DROPS `revert` at parse) | `""` (accepted; `revert` unrepresented) | parser gap — separate slot |

Notes proving the corner-4 divergences are **Blink outliers, not the ideal**:

- **`gap: initial 4px` does not round-trip in Blink.** `el.style.setProperty('gap',
  'initial 4px')` → `cssText === ""` (Blink rejects its own getter output as
  invalid input). So Blink's `"initial 4px"` is a **non-round-tripping bug**;
  `overflow` (the other Box axis-pair) correctly returns `""` for the identical
  shape. Uniform `""` is spec-faithful (§6.7.2 step 1.2) and internally consistent.
- **Blink is inconsistent even among omit-initial families.** `column-rule` omits a
  component declared `initial` (`"solid red"`), but `columns` returns `""` for
  `column-width:initial; column-count:3`. Blink's omit behavior comes from its
  shorthand-expansion representation: `column-rule: solid red` expands the omitted
  width to the **`initial` keyword** longhand (measured: `getPropertyValue(
  'column-rule-width') === "initial"`), so its serializer must treat `initial` as
  omittable to round-trip. **elidex expands omitted components to their *concrete*
  initial values** (per #468 note: `parse_column_rule_shorthand` `width.unwrap_or(
  3px)`), so elidex's common omit-initial case (`column-rule: solid` → `"solid"`)
  works via concrete-value comparison **without** interpreting the `initial`
  keyword. Only the narrow *author-writes-`initial`-on-a-longhand* corner needs the
  keyword interpretation — a separable, edge-dense, per-family facet.

---

## Ideal architecture (property-agnostic gate in the coordinator)

Insert the gate in `serialize_shorthand_value` **after** the §6.6.1 all-present +
uniform-`!important` checks and **before** the per-family dispatch:

```rust
// … existing: longhands present? uniform important? build `pairs` …

// CSSOM §6.7.2 step 1.2 — value-KIND gate (property-agnostic; the single
// canonical site). Returns Some(..) for every case the component KINDS decide;
// None only when all components are physical → the family collapse runs.
if let Some(result) = value_kind_gate(&pairs) {
    return Some(result);
}
registry.resolve(&longhands[0])?.serialize_shorthand(property, &pairs)  // all-P
```

```rust
enum Kind { Physical, CssWide, Var }

fn value_kind(v: &str) -> Kind {
    if v.contains("var(") {
        Kind::Var                                   // CssValue::Var or RawTokens("…var(…")
    } else if matches!(
        v.to_ascii_lowercase().as_str(),
        "initial" | "inherit" | "unset" | "revert" | "revert-layer"
    ) {
        Kind::CssWide            // css-cascade-4 §7.3 (revert-layer: css-cascade-5 §7.3.5)
    } else {
        Kind::Physical
    }
}

/// CSSOM §6.7.2 step 1.2 applied by component value-kind:
/// - any unresolved var()        → Some("")   (not a concrete component pre-substitution)
/// - all the same CSS-wide kw     → Some(kw)   (the shorthand IS that keyword)
/// - mixed different CSS-wide kw   → Some("")   (cannot exactly represent)
/// - a CSS-wide kw mixed with a physical value → Some("")   (cannot exactly represent)
/// - all physical                 → None       (defer to the family collapse)
fn value_kind_gate(pairs: &[(&str, &str)]) -> Option<String> {
    let kinds: Vec<Kind> = pairs.iter().map(|(_, v)| value_kind(v)).collect();
    if kinds.iter().any(|k| matches!(k, Kind::Var)) {
        return Some(String::new());
    }
    let csswide = kinds.iter().filter(|k| matches!(k, Kind::CssWide)).count();
    if csswide == 0 {
        return None;                                // all physical → handler
    }
    if csswide == pairs.len() {
        let first = pairs[0].1;
        if pairs.iter().all(|(_, v)| v.eq_ignore_ascii_case(first)) {
            return Some(first.to_ascii_lowercase()); // §6.7.2 keyword serialization
        }
        return Some(String::new());                 // mixed different CSS-wide
    }
    Some(String::new())                             // CSS-wide mixed with physical
}
```

### Why string classification (not `CssValue` kind)

`InlineStyle` stores **strings** (it is the CSSOM `CSSStyleDeclaration` backing
store; CSSOM stores serialized values). The inline path's `get` closure
(`elidex-dom-api/src/style.rs:73`) can only return `style.get(lh): &str`; the rule
path (`cssom_sheet.rs:607`) has `CssValue` but must feed the *same* coordinator.
A property-agnostic gate therefore classifies **strings**, which is robust here:
CSS-wide keywords are globally reserved exact tokens, and `var(` is the sole CSS
function spelled that way (a standard serialized value never contains the literal
substring `var(` except an actual `var()`). `revert`/`revert-layer` are included
for forward-compat though currently **unreachable** (the parser drops them — see
Scope).

### The coordinator-vs-handler split for corner 4 (decided)

**The coordinator returns `""` for *all* CSS-wide-mixed-with-physical cases** (and
all mixed-different-CSS-wide, and any-var). Rationale:

1. **Spec-faithful** — §6.7.2 step 1.2 is uniform: a CSS-wide keyword on one
   longhand but not others cannot be exactly represented by the shorthand → `""`.
2. **Matches Blink for the majority** — `margin`/`padding`/`border-radius`/
   `overflow`/`border-spacing`/`inset`/`place-items`/`columns` all return `""`.
3. **Diverges only from Blink's two outliers, both toward `""`** — `gap`
   (`"initial 4px"`, proven non-round-tripping) and `column-rule` (omit →
   `"solid red"`). Reproducing them would require the gate to be *not*
   property-agnostic (`gap` joins while sibling `overflow` rejects — both Box
   axis-pair), destroying I2.
4. **Fully unblocks PR1 without pushing css-wide logic into handlers** — the gate
   hands `MulticolHandler::serialize_shorthand` **only all-physical** longhands, so
   PR1 is a pure omit-initial collapse with **no** css-wide/var branch.

**Deferred (named slot) — the Blink-faithful omit-initial `initial`-omit:** making
`column-rule`/`columns` omit an author-written `column-rule-width: initial`
(≡ its initial value) to reach Blink's `"solid red"` is a family-dependent facet
requiring the handler to interpret the literal `initial` keyword as the initial
value — and Blink itself is inconsistent about it (`columns` doesn't omit). Slot
**`#11-shorthand-omit-initial-csswide-omission`**, owned by the omit-initial family
PRs (PR1+). Until then the coordinator's `""` is a safe CSSOM-valid
under-approximation. The **common** omit-initial case (`column-rule: solid` →
`"solid"`) is unaffected — it never carries a css-wide keyword (elidex expands to
concrete initials), so the gate falls through to the handler.

---

## Scope

**Fixes (regression fixes on already-shipped surface):**

- All **6 landed families** — `margin`/`padding`/`border-radius`/`overflow`/`gap`/
  `border-spacing` — for corners 2, 3, 4 (measured mis-collapses → `""`), and
  corner 1 stays correct (now *intentionally*, via the gate, not str-eq accident).
- One **intentional Blink divergence**: `gap` `initial`+physical `"initial 4px"` →
  `""` (spec-faithful; Blink's is non-round-tripping).

**Unblocks:**

- **PR1 (Multicol `column-rule`/`columns`)** — with the gate, PR1's handler sees
  only all-physical longhands; corners 1–4 are handled property-agnostically. PR1
  needs **no functional change beyond rebasing** onto this prereq (it drops any
  css-wide/var handling it would otherwise need) and documents the deferred
  `initial`-omit under-approximation.

**Defers (named slots):**

- **`#11-shorthand-omit-initial-csswide-omission`** — corner-4 Blink-faithful
  omission of an explicit `initial`-keyword longhand in omit-initial shorthands
  (`column-rule` `"solid red"`). Edge-dense (`initial` vs `inherit`/`unset` ×
  component position × per-family initial values × Blink's own inconsistency).
  Owned by PR1+. **Trigger**: a WPT/site depending on the omit; **Re-eval**:
  2026-10-31.
- **`#11-css-wide-revert-keyword`** — `revert`/`revert-layer` have **no**
  representation in elidex: `parse_global_keyword` (`elidex-css/src/values.rs:96`)
  returns `None` for them, so `margin: revert` produces **0 declarations**
  (measured) and `getPropertyValue` yields `""` where Blink yields `"revert"`.
  This is a **parser + `CssValue` model** gap orthogonal to serialization; the gate
  already classifies the strings for forward-compat. **Trigger**: revert support
  (needs a `CssValue` variant + cascade origin rollback) or a WPT/site. **Re-eval**:
  2026-10-31.
- **Corner 5 (whole-shorthand `var()`) is NOT deferred — it already works.**
  `margin: var(--x)` / `columns: var(--w)` store the value under the **shorthand**
  name (whole-value var is not longhand-expanded; only global keywords expand), so
  the coordinator's all-present check fails and each caller's `.or_else(...)` fallback
  reads the shorthand's own stored value — inline path `style.get(property)`
  (`style.rs:78`), rule path `last(&normalized)` last-declaration lookup
  (`cssom_sheet.rs:619`) — returning `"var(--x)"` (measured, matches Blink). The gate
  runs *after* all-present and is never reached, so it cannot regress this. (This
  corrects the task premise that elidex returns `""` here.)

---

## Parse-discrepancy investigation (resolved — no parse bug)

An earlier probe of `parse_declaration_block("column-width: var(--w);
column-count: auto")` returned only **one** declaration, suggesting `var()` aborts
declaration-block parsing. **Determined via throwaway `#[test]` (dumped
`parse_declaration_block_with_registry` output, `--nocapture`, then removed +
`git checkout`): it was a no-registry probe artifact, NOT a parse bug.**

- `parse_declaration_block(css)` passes **`None`** registry. `column-count`/
  `column-width` are **registry-backed** (`MulticolHandler`, not in the built-in
  `parse_property_value` match), so with no registry `column-count: auto` fails to
  parse → dropped; `column-width: var(--w)` survives only because the var branch is
  registry-independent (`RawTokens`).
- **With a registry** (`parse_declaration_block_with_registry(css, Some(&reg))` —
  what the real CSSOM path always uses via `inline_style_registry()` /
  `default_css_property_registry()`) → **both** declarations parse (`column-width =
  RawTokens("var(--w)")`, `column-count = Auto`). Confirmed end-to-end via
  `parse_inline_style` too. `collect_declaration_value_tokens` stops at the
  top-level `;`, so `var()` never swallows the following declaration.

**No parse fix in scope.** (Guard for reviewers: any elidex-internal probe of
multicol/flex/grid/transform properties MUST pass a populated registry.)

---

## §3. Spec coverage map

| Spec section | Step / dfn | Branch | Touch (dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM 1 §6.6.1 *The CSSStyleDeclaration Interface* | `getPropertyValue` shorthand | all-present / uniform-important (unchanged) | `serialize_shorthand_value` (existing checks) | ✓ | no (read path; `property` ASCII-lowercased upstream) |
| CSSOM 1 §6.7.2 *Serializing CSS Values* | step **1.2** ("cannot exactly represent → empty string") | any-V → `""` / all-same-K → kw / mixed-K → `""` / K+P → `""` / all-P → collapse | **`value_kind_gate` (NEW)** in the coordinator | ✓ | yes — the K/V component values are author-controllable (see §3.1) |
| CSSOM 1 §6.7.2 | "serialize a CSS component value: keyword → ASCII lowercase" | all-same-K keyword output | `value_kind_gate` returns `first.to_ascii_lowercase()` | ✓ | no |
| css-cascade-4 §7.3 *Explicit Defaulting* | `initial`/`inherit`/`unset` (+`revert`; `revert-layer` is css-cascade-5 §7.3.5 *Rolling Back Cascade Layers*) | **K** classification | `value_kind` keyword set | ✓ (`revert`/`revert-layer` dormant — parser drops them) | yes |
| css-variables-1 §3 *Using Cascading Variables: the var() notation* / §2.2 *Guaranteed-Invalid Values* | unsubstituted `var()` is not a concrete component at specified-value time | **V** classification → `""` | `value_kind` `contains("var(")` | ✓ | yes |

**Breadth**: K=3 specs (cssom-1, css-cascade-4, css-variables-1), M≈5 entries —
under the K≥4 / M≥20 multi-PR threshold ⇒ **single-PR scope**. But it is
**edge-dense at corner 4** (value-KIND × family-KIND, with Blink internally
inconsistent) ⇒ **plan-review required** (this memo).

### §3.1 User-input touch audit

`getPropertyValue(property)` is a **read** API; `property` is the JS arg
(ASCII-lowercased upstream) and only indexes the static `shorthand_longhands` table
+ `registry.resolve` (total functions; unknown → `None` → `""`). The **new**
user-controllable surface is the **component value strings** classified by the
gate: an author writes `margin: var(--x)`, `column-rule-width: initial`, etc. in a
stylesheet or `el.style.cssText`; these flow through the parser
(`expand_global_keyword` / the var branches) into `InlineStyle`/rule declarations
as strings, then into `value_kind`. The gate only **reads** those strings and
returns a computed shorthand string or `""` — no eval, no reflection into a sink,
no indexing-by-value, no recursion. Classification is exact-keyword-match +
substring `var(` — total, panic-free, allocation-bounded. No injection/cycle/
prototype surface.

---

## Test plan

- **Gate unit tests** (`crates/css/elidex-style/src/tests/shorthand.rs`, extend the
  existing `serialize_shorthand_value(registry, prop, lookup)` harness):
  - all-same-K → keyword (`margin`/`overflow`/`column-rule`/`columns` ×
    `initial`/`inherit`/`unset`).
  - mixed-different-K → `""`.
  - any-V (Var and `RawTokens("var(…")`) → `""`, incl. V+K and V+P mixes.
  - K+P (`initial`/`inherit`/`unset` + physical) → `""` for every family incl.
    **`gap`** (the intentional Blink divergence — assert `""`, comment cites the
    non-round-trip proof).
  - all-P → unchanged collapse (regression: the existing rectangular/axis-pair
    assertions stay byte-identical).
- **Regression on the 6 landed families**: assert the measured mis-collapses are
  now `""` (corners 2/3/4) and corner 1 unchanged.
- **Corner-5 guard**: an inline/rule test that `margin: var(--x)` /
  `columns: var(--w)` still return `"var(--x)"` (fallback unbroken).
- **No new test asserts the deferred `column-rule` `"solid red"` omit** — it is a
  documented under-approximation (`""`), owned by `#11-shorthand-omit-initial-
  csswide-omission`.
- `cargo test -p elidex-style -p elidex-plugin --all-features`; then `mise run ci`.

---

## Lane / coordination

- **In-lane** (OWN = `crates/css/**` + `elidex-style`): the gate is **internal** to
  `elidex_style::serialize_shorthand_value` (`elidex-style/src/lib.rs`). No new
  `elidex-plugin` trait surface, no handler change.
- **No cross-lane caller changes** — the 2 `elidex-dom-api` callers
  (`style.rs:73`, `cssom_sheet.rs:614`) already call `serialize_shorthand_value`;
  the gate is behind that boundary, so they are untouched (contrast #468, which
  changed the callers). Verify no concurrent `style.rs`/`cssom_sheet.rs` edit at
  impl.
- **This is the PR1 prerequisite.** PR1 (`MulticolHandler::serialize_shorthand`)
  **rebases onto this** and relies on the gate to strip all css-wide/var before
  dispatch. Land this first.
- Under the `#11-style-shorthand-expand` umbrella (slot STAYS OPEN for per-family
  omit-initial coverage). Register the two new slots at landing per the defer
  lifecycle (per-PR ≤3: 2 registered here — omit-initial-omission, revert-keyword).

## Edge-density → plan-review

Intersecting axes: value-KIND classification × family-KIND collapse × Blink
internal inconsistency (corner 4) × string-vs-`CssValue` classification constraint
× the coordinator-vs-handler split × 6-family regression + PR1 unblock. **Run
`/elidex-plan-review` on this memo before implementing** (mission edge-dense rule;
corner 4 is a real coupled invariant, and the `gap` Blink-divergence is a judgment
call reviewers should ratify).

**Post-push review = `/external-converge` (pre-committed), not single-pass.** This
is plan-review-mandatory edge-dense work, and the sibling PR1 (#471) single-pass
`/external-review` already surfaced a *real* gate-miss — the value-KIND bug this
prereq fixes. Per `feedback_gate-miss-on-edge-dense-escalate-to-converge` both the
proactive (plan-review-mandatory) and reactive (a prior single-pass produced a real
gate-miss) arms fire → drive Codex to real-gap exhaustion from round 1.

## Open questions (flagged, not invented)

1. **`gap` Blink-divergence ratification.** This memo chooses spec-uniform `""`
   over Blink's non-round-tripping `"initial 4px"`. If a surveyed site depends on
   Blink's `gap` quirk, reconsider — but that would re-introduce per-property
   special-casing into the property-agnostic gate (I2 cost). Recommend: keep `""`.
2. **`column-rule` omit under-approximation acceptance.** PR1 lands with `""` for
   `column-rule-width:initial; solid; red` (Blink `"solid red"`). Confirm this is
   acceptable for PR1 vs pulling `#11-shorthand-omit-initial-csswide-omission`
   forward into PR1 (raises PR1's edge-density; Blink is itself inconsistent here).
   Recommend: defer.
3. **`revert` scope.** Confirmed unrepresented (dropped at parse). Left to
   `#11-css-wide-revert-keyword` (parser + `CssValue` model). Not this prereq.
4. **`contains("var(")` robustness.** Chosen over `CssValue`-kind classification
   because `InlineStyle` is string-backed. Reviewers: confirm no serialized
   non-var value in a shorthand longhand can contain the literal `var(` (none
   found — `var` is the only such-spelled CSS function).
