# Plan: shorthand serialization → handler-owned + registry-dispatched

Umbrella for `#11-style-shorthand-expand`. **Design direction PM-approved** (handler-
owned serialization + elidex-style coordinator). This memo = the **foundational
PR** (behavior-preserving relocation + trait slot); per-family coverage lands in
follow-up PRs.

## Problem (design-level, not a patch)

`serialize_shorthand_value` currently lives in **elidex-css** (the parser crate,
`shorthand.rs:36`) and hardcodes shorthand grammar (rectangular / axis-pair
collapse) **disconnected from the handlers that own the property grammar**. Two
consequences block extending it to the omit-initial families:

1. **Initials are unreachable.** omit-initial serialization needs each longhand's
   initial value. The single source of truth is `CssPropertyHandler::initial_value`
   (elidex-plugin `traits.rs:122`), dispatched via the populated
   `CssPropertyRegistry` assembled in **elidex-style** (`default_css_property_registry`,
   lib.rs:63). elidex-css sits *below* elidex-style — precisely: `elidex-plugin`
   (base) → { `elidex-css`, the handler crates } as a **tier, not a chain**
   (`elidex-css-flex` depends only on elidex-plugin, *parallel* to elidex-css;
   Box/Table/Multicol/Text depend on elidex-css) → `elidex-style` (assembles the
   registry) → `elidex-dom-api`. elidex-css **cannot reach the populated registry**
   (assembled a tier up) — so elidex-css would have to duplicate initials
   (anti-pattern, violates the single-SoT the `get_initial_value` docstring states).
2. **Misplaced ownership.** Each family's grammar + initials + parser already live
   in its handler (Box/Multicol/Flex/Text/Table…). Serialization is the read-side
   twin of parse; it belongs with the handler, not in a central parser-crate table.

Verified: the parser expands a shorthand to **all** longhands, defaulting omitted
ones to initials (`parse_column_rule_shorthand`, misc.rs:614 — `width.unwrap_or(3px
medium)` etc.), so the serializer genuinely must omit-initial to round-trip
`column-rule: solid` → `"solid"`.

## §2. Coupled invariants (foundational PR)

The foundational PR is **breadth-heavy but single-binding-invariant** —
deliberately NOT the coupled-invariant-at-corners density that mandates per-corner
enumeration. Its invariants and their (weak) intersections:

- **I1 Behavior-preservation** — byte-identical `getPropertyValue` /
  `getComputedStyle` output for the 6 currently-covered families. The dominant,
  binding constraint, mechanically enforced by a **golden-reference oracle**
  (every existing rectangular/axis-pair test passes unchanged).
- **I2 Layer-correctness** — helpers in elidex-plugin so `elidex-css-flex`
  (plugin-only dep) can reach them; coordinator in elidex-style for registry
  access.
- **I3 No-strangler** — fully converge (delete the old elidex-css seam; no
  coexistence).

Pairwise intersections (all weak — this is why it is a valid single PR):
- **I1 × I2** — the relocation must preserve output *regardless of* where code
  lives; but the moved logic is **verbatim** (same `serialize_rectangular` /
  `serialize_axis_pair` bodies), so placement is orthogonal to output.
- **I2 × I3** — converging all families at once forces the helper/coordinator
  placement to serve *every* handler simultaneously (hence base-crate helpers).
  Corner probe: a handler crate depending on neither elidex-plugin helpers nor
  elidex-css → **none exists** (all handlers depend on elidex-plugin).
- **I3 × I1** — converging all families in one PR keeps I1 checkable by one
  static golden oracle rather than a moving target.

**The genuinely-coupled invariants are deferred**: omit-initial × canonical-order
× per-family-initials-SoT intersect at real corners (`column-rule: solid` must
omit medium-width AND currentcolor AND round-trip with the parser). Those live in
the **per-family PRs (1–5+), each with its own `/elidex-plan-review`** where the §2
corner matrix is owed. This foundational PR introduces none of them — it moves only
the already-covered structural (rectangular / axis-pair) families.

**ECS-native check**: N/A — no ECS component is added, read, or written; this is a
pure plugin-dispatch relocation over a static property→handler registry (no
per-entity state). See the Step 1.5 dry-run for the sub-check 2b trace.

## Ideal architecture (plugin-first)

Serialization becomes **handler-owned**, dispatched through the registry:

- **Trait slot** (elidex-plugin `CssPropertyHandler`):
  ```rust
  /// Serialize the shorthand `property` from its ordered, validated longhand
  /// (name, serialized-value) pairs. `None` ⇒ CSSOM-valid "" (not serializable
  /// or not covered). Omit-initial families compare each value against
  /// `self.initial_value(name).to_css_string()` — the handler's own SoT.
  fn serialize_shorthand(&self, property: &str, longhands: &[(&str, &str)]) -> Option<String> { None }
  ```
  Default `None` (most handlers own no shorthands / not-yet-covered).
- **Coordinator** (elidex-style, has the registry, in-lane) — the single entry the
  CSSOM surface calls:
  ```rust
  pub fn serialize_shorthand_value(registry, property, get: impl Fn(&str)->Option<(String,bool)>) -> Option<String> {
      let longhands = elidex_css::shorthand_longhands(property);   // static table stays in css (parser uses it too)
      if longhands.is_empty() { return None; }                     // not a shorthand
      let decls = longhands.iter().map(|lh| get(lh)).collect::<Option<Vec<_>>>()?;  // §6.6.1: all present
      let imp = decls[0].1;
      if !decls.iter().all(|(_, i)| *i == imp) { return None; }    // §6.6.1: uniform !important
      let pairs: Vec<(&str,&str)> = longhands.iter().map(|s| s.as_str()).zip(decls.iter().map(|(v,_)| v.as_str())).collect();
      // ⚠ The registry is keyed by LONGHAND name — `CssPropertyHandler::property_names()`
      // is longhands-only (shorthand expansion is internal to `parse`), so a shorthand
      // name NEVER resolves. Find the owner via the shorthand's first longhand; every
      // longhand of a shorthand belongs to the same handler (margin-* → Box,
      // border-spacing-* → Table, flex-* → Flex, …).  [discovered at impl; the
      // behavior-preserving tests caught it — follow-up PRs must use the same lookup]
      registry.resolve(&longhands[0])?.serialize_shorthand(property, &pairs)
  }
  ```
  The **common CSSOM §6.6.1 checks (all-present + uniform-important) stay in the
  coordinator** (they are property-agnostic); only the **per-family collapse** is
  dispatched to the handler.
- **Shared structural helpers** (`serialize_rectangular`, `serialize_axis_pair`)
  move to **elidex-plugin** (base crate) so *every* handler can call them —
  crucially `elidex-css-flex` does **not** depend on elidex-css, so the helpers
  cannot live in elidex-css.

### Why elidex-style (not elidex-css) for the coordinator
elidex-style is the lowest layer that (a) can reach the populated registry and (b)
is in this lane (`crates/css/elidex-style` ∈ OWN). elidex-dom-api already depends on
elidex-style and both callers already hold the default registry (`style.rs:32`
`inline_style_registry()`), so routing through it is natural.

## Foundational PR scope (behavior-preserving — NO new families)

One-issue-one-way: **fully converge**, no strangler (don't leave the old elidex-css
seam beside a new coordinator). Concretely:

1. **elidex-plugin**: add the `serialize_shorthand` trait method (default `None`);
   move `serialize_rectangular` / `serialize_axis_pair` here as shared helpers.
2. **Handlers** — implement `serialize_shorthand` for the **currently-covered**
   families only (behavior-preserving), each in its owning handler (confirm owner
   via `property_names()` at impl):
   - `BoxHandler`: `margin` / `padding` / `border-radius` (rectangular), `overflow`
     (axis-pair) [+ `gap` if Box owns row-gap/column-gap — else its real owner].
   - `TableHandler`: `border-spacing` (axis-pair).
   - (`gap` owner TBD-verify: candidates Box / Flex / Grid.)
3. **elidex-style**: add the registry-aware `serialize_shorthand_value` coordinator
   (above). Re-export or new module.
4. **elidex-dom-api callers (cross-lane, PM-approved)** — 2 sites (verified
   2026-07-13 via `grep -rn serialize_shorthand_value crates/dom`:
   `elidex-dom-api/src/style.rs:72`, `elidex-dom-api/src/cssom_sheet.rs:614`) switch from
   `elidex_css::serialize_shorthand_value(property, get)` to
   `elidex_style::serialize_shorthand_value(registry, property, get)`, passing the
   registry they already hold (`style.rs:72` inline path already has
   `inline_style_registry()`; `cssom_sheet.rs:614` rule path adds a
   `default_css_property_registry()` handle). **Minimal** — import + one arg.
5. **elidex-css**: delete the old `serialize_shorthand_value` +
   `serialize_rectangular`/`serialize_axis_pair` (moved). Keep `shorthand_longhands`
   (parser + coordinator both use it).

**Behavior preservation is the acceptance bar**: every existing rectangular/axis-
pair test (shorthand.rs tests, dom-api style tests, cssom_sheet tests) passes
unchanged; getComputedStyle / getPropertyValue output byte-identical.

## Staging (umbrella → per-family)

- **PR 0 (this)**: foundational relocation + trait slot (behavior-preserving).
- **PR 1**: MulticolHandler — `column-rule` / `columns` (ordered omit-initial, own
  initials).
- **PR 2**: FlexHandler — `flex-flow` (+ `flex` keyword cases none/auto/initial).
- **PR 3**: TextHandler — `text-decoration`.
- **PR 4**: BoxHandler — `border` (nested 4-side + omit-initial).
- **PR 5+**: layered/grid (BackgroundHandler `background`; `font`; GridHandler
  `grid`/`grid-template`) — individually edge-dense, own plan-reviews.
- **`list-style` — BLOCKED on longhand-mapping completeness, not scheduled here.**
  It is an ordered-omit-initial family, but elidex's `shorthand_longhands("list-style")`
  currently maps to a **single** longhand `list-style-type` (missing
  `list-style-position` / `list-style-image` — `declaration.rs:760`). Serializing a
  degenerate single-longhand shorthand is trivial, but doing it before the mapping
  is completed would bake in the wrong grammar. Its serialization is deferred under
  the same umbrella until the `list-style` longhand mapping is completed (a separate
  sub-task); flagged here so it is not orphaned.

Each per-family PR is small (one handler's `serialize_shorthand` arm + tests),
uses that handler's `initial_value` (no duplication), spec-cited per module.

## §3. Spec coverage map

Behavior-preserving relocation: the foundational PR **moves** the existing CSSOM
serialization logic without changing its observable output. No spec *behavior*
changes; the map records where the (relocated) algorithm now compiles.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM 1 §6.6.1 The CSSStyleDeclaration Interface | `getPropertyValue` shorthand branch | (i) any longhand absent → `""` | elidex-style coordinator `serialize_shorthand_value` (NEW home; relocated from elidex-css, byte-identical) | ✓ | no (read path; `property` is the JS arg, ASCII-lowercased upstream) |
| CSSOM 1 §6.6.1 The CSSStyleDeclaration Interface | shorthand important-uniformity | (ii) mixed `!important` → `""` / (iii) uniform → serialize | same coordinator (common check kept central, property-agnostic) | ✓ | no |
| CSSOM 1 §6.7.2 Serializing CSS Values | serialize-a-CSS-value (shorthand) | (i) rectangular / axis-pair → collapse (this PR) / (ii) omit-initial + layered/grid → `None`=`""` (follow-up PRs) | `CssPropertyHandler::serialize_shorthand` (NEW trait slot) dispatch | ✓ (covered families; uncovered honestly return `None`) | no |

**Breadth**: K=1 spec (cssom-1), M=3 entries → **single-PR scope** (well under
K≥4/M≥20). Per-family follow-up PRs add their own module citations (css-multicol
§, css-flexbox §, css-text-decor-3 §) at that time.

### §3.1 User-input touch audit

No user-controllable input reaches new logic paths: `getPropertyValue(property)` is
a **read** API; `property` is caller-supplied but only indexes the static
`shorthand_longhands` table + registry `resolve` (both total functions, unknown →
`None`→`""`). The moved collapse operates on already-serialized, already-validated
longhand strings. No injection / cycle / prototype surface (contrast the D-17b
`Op::SetPrototype` exposure pattern — N/A here).

## Spec (per-family, follow-up)

- CSSOM §6.6.1 `getPropertyValue` (all-present + uniform-important) + §6.7.2
  serialize-a-CSS-value — the coordinator's common checks (this PR).
- Per-family serialization order + initials → each CSS module (css-multicol,
  css-flexbox, css-text-decor-3, css-backgrounds) at the per-family PR.

## Lane / coordination

- **In-lane** (OWN = crates/css/** + elidex-style): elidex-css edits, elidex-style
  coordinator, the handler crates. **elidex-plugin** (trait + helpers) is
  `crates/core/elidex-plugin` — NOT elidex-ecs, and mission-analogous to the color
  grammar (declared CSS surface); treat as in-scope but flag to PM.
- **Cross-lane** (crates/dom, PM-approved for this direction): the **2
  elidex-dom-api caller** updates (import + registry arg). Minimal; coordinate with
  the active dom lane (`elidex-wt-clone`) to avoid a `style.rs`/`cssom_sheet.rs`
  collision — verify no concurrent edit at impl.

## Edge-density → plan-review

Intersecting axes: trait design × handler distribution × registry dispatch × cross-
lane callers × behavior-preservation. **Run `/elidex-plan-review` on this memo
before implementing** (mission edge-dense rule + PM "reconsider design" steer).

## Test plan

- Behavior-preserving: all existing shorthand/style/cssom tests pass unchanged.
- New: a handler-level `serialize_shorthand` unit test per moved family (assert
  identical output to the old central path).
- `mise run ci` (App-profile note: elidex-plugin trait change compiles under
  `--no-default-features` too).
