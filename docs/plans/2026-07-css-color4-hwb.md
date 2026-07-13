# Plan: `hwb()` color function — CSS Color 4 sRGB-cheap subset

Slot: `#11-css-color4-extended-syntax` (sRGB-cheap subset only). Closes the
`hwb()` facet; the float-pipeline / used-value facets stay open.

## Goal

Add `hwb()` to the engine's single `<color>` grammar chokepoint
(`elidex-plugin/src/color/mod.rs::parse_color`, re-exported as
`elidex_css::parse_color`). Because that function is the *one* parse home,
every consumer — the CSS cascade (`background-color`, `color`, borders, …) and
`<input type=color>` value sanitization (`CssColor::parse_str`) — gains `hwb()`
support automatically.

## Spec

- CSS Color 4 **§8 HWB Colors: `hwb()` function** — grammar + semantics.
- CSS Color 4 **§8.1 Converting HWB Colors to sRGB** — the `hwbToRgb`
  reference algorithm (verified via webref `body css-color-4 the-hwb-notation`).

Grammar (spec §8):

```
hwb() = hwb( [<hue> | none]
             [<percentage> | <number> | none]
             [<percentage> | <number> | none]
             [ / [<alpha-value> | none] ]? )
```

"HWB colors resolve to sRGB" ⇒ **sRGB-cheap** (no Color-4 float pipeline), the
gating property for this slot. hwb() is *new* in Color 4 and has **no legacy
comma syntax** — commas inside `hwb()` are an error (space-separated only).

## Scope (in / out) — bounded per slot

**IN**

- `hwb(<hue> <whiteness%> <blackness%>)` + optional `/ <alpha>`.
- Hue: `<number>` | `<angle>` (deg/grad/rad/turn) — reuses existing
  `parse_hue` (identical to `hsl()` per §8).
- Whiteness/blackness: `<percentage>`, clamped `[0%, 100%]` — reuses existing
  `parse_percentage_unit_value` (the same helper `hsl()` s/l use).
- Alpha: `<number>` | `<percentage>`, clamped `[0, 1]` — reuses
  `parse_alpha_component`.
- `hwbToRgb` (§8.1) with achromatic short-circuit (`W + B ≥ 100%` → grey
  `W / (W + B)`, hue powerless).

**OUT (explicitly deferred, not piecemeal here)**

- `lab()` / `lch()` / `oklab()` / `oklch()` / `color()` — need the Color-4
  float pipeline (`#11-color-well-alpha-colorspace` companion). Not sRGB-cheap.
- `currentColor` / `<system-color>` — context-dependent (used-value)
  resolution, not a pure value transform.
- Bare `<number>` whiteness/blackness **and** the `none` keyword — the engine's
  polar-notation surface (`hsl()`) is deliberately percentage-only and rejects
  bare numbers + `none` (see the existing `hsl_bare_numbers_for_sl_rejected`
  test, comment "s and l must be percentages"). Per **One-issue-one-way**, the
  `<number>`/`none` component grammar is added *uniformly* across `hsl()`+`hwb()`
  as one future unit, **not** bolted onto `hwb()` alone (which would make `hwb`
  the lone function accepting `30` while `hsl` rejects it — a decision-tax
  divergence). `hwb()` therefore ships the same percentage-only component
  fidelity as today's `hsl()`.

## Design

**One-issue-one-way / reuse over new abstraction.** `hwb()` introduces exactly
one genuinely new step — the whiteness/blackness *mix* — everything else reuses
the existing `hsl()` helpers (`parse_hue`, `parse_percentage_unit_value`,
`parse_alpha_component`, `clamp_u8`).

The pure-hue → sRGB sextant is shared by HSL and HWB, so extract it once:

- `hue_to_rgb01(h) -> (f32, f32, f32)` — pure hue at full chroma (C=1) in
  `[0, 1]` (finite-guard + `[0, 360)` normalization live here now).
- `hsl_to_rgb` becomes: `hue_to_rgb01` scaled by chroma `C` + lightness offset
  `m` — **behavior-preserving** (verified against every existing `hsl` test:
  `r1 = R·C`, `x = x_hue·C`, so the refactor is algebraically identical).
- `hwb_to_rgb(h, w, b)` = `hue_to_rgb01` mixed: `channel·(1−W−B) + W`, with the
  `W + B ≥ 1` achromatic branch.

This keeps a single sextant implementation (no duplicated hue math).

**Worked-example check (spec §8):** `hwb(150 20% 10%)` = `rgb(20% 90% 55%)` and
`hwb(45 40% 80%)` = achromatic `rgb(33.33%…)` both reproduce (the 90% channel
lands at 229/230 depending on f32 rounding at the `.5` boundary — asserted with
±1 tolerance, exact elsewhere).

## Files (all within lane: `elidex-plugin` color grammar — mission-authorized)

- `crates/core/elidex-plugin/src/color/mod.rs` — module doc, `parse_color`
  dispatch (`"hwb"` arm), `hue_to_rgb01` extraction, `hsl_to_rgb` refactor,
  `hwb_to_rgb`, `parse_hwb_function`.
- `crates/core/elidex-plugin/src/color/tests.rs` — `hwb()` coverage.

Not edge-dense (single canonical algorithm, no intersecting invariants, pure
value transform) ⇒ no `/elidex-plan-review` gate; standard `/pre-push` +
`/external-review`.

## Cross-lane coordination note (for PM)

`crates/dom/elidex-form/src/sanitize.rs` (lines ~180–192, **dom lane, not
touched here**) documents the supported color surface and lists `hwb()` as
"fails to parse … falls to opaque black" / "hwb() is cheap sRGB [deferred]".
Once this lands, that enumeration is stale (its forward-looking "when
`parse_str` gains them, color sanitization benefits automatically" clause stays
correct). Flag for a dom-lane doc touch; do **not** fold a `crates/dom` edit
into this CSS PR.

## Slot bookkeeping (PM reconciles SoT)

`#11-css-color4-extended-syntax` is **not fully closed** — it carves down to the
remaining functions. Suggested post-land disposition: rename/retarget the slot
to the float-pipeline residue (`lab`/`lch`/`oklab`/`oklch`/`color`) +
`currentColor`/`<system-color>`, or fold those into
`#11-color-well-alpha-colorspace`. New sub-note worth tracking: uniform
`<number>`/`none` polar components across `hsl()`+`hwb()`.
