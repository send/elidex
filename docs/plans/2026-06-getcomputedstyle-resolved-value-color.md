# Plan: getComputedStyle resolved-value color serialization (`#11-getcomputedstyle-resolved-value-color`)

**Status**: plan-memo (pre-`/elidex-plan-review`)
**Branch**: `getcomputedstyle-resolved-color`
**Scope**: single PR (narrowly-scoped slice; edge-dense → mandatory plan-review per CLAUDE.md "Edge-dense work" rule)

---

## §1. Problem

`getComputedStyle(el).<colorProp>` currently returns the **declared-value** serialization
(`#rrggbb` opaque / `rgba(r, g, b, 0.50)` translucent), not the CSSOM **resolved-value**
form. Per CSSOM-1 §9 *Resolved Values*, every color longhand
(`color`, `background-color`, `border-*-color`, `outline-color`,
`text-decoration-color`, …) is a **"resolved value special case property"** whose
**resolved value is the used value**. Per CSS Color 4 §16.2.2, the used value of an
sRGB color serializes in the **legacy `rgb()` / `rgba()` form** — comma separators,
exactly one ASCII space after each comma, `rgb()` when alpha == 1 else `rgba()`,
components as base-10 `<number>` in [0, 255].

Concretely, today:

| input | elidex returns now | spec resolved value |
|-------|--------------------|---------------------|
| `color: red` | `#ff0000` | `rgb(255, 0, 0)` |
| `background: rgba(0,0,0,.5)` | `rgba(0, 0, 0, 0.50)` | `rgba(0, 0, 0, 0.5)` |
| `text-decoration-color` (initial) | `currentcolor` | `rgb(r, g, b)` of element's `color` |

### Verified current path (Read + grep)

- **Single boundary chokepoint**: `crates/dom/elidex-dom-api/src/computed_style.rs:54-55`
  — `GetComputedStyle::invoke` does `get_computed(&property, &style).to_css_string()`.
  This is the **only** getComputedStyle serialization site (boa bridge + VM both
  funnel `getComputedStyle(el).prop` into this one handler — verified via grep of
  `getComputedStyle`/`get_computed_style` callers).
- **`get_computed` dispatch**: `crates/css/elidex-style/src/resolve/mod.rs:46-74`
  (`get_computed_with_registry`) → `registry.resolve(prop)?.get_computed(prop, style)`,
  returning a `CssValue`. `get_computed` re-export = `crates/css/elidex-style/src/lib.rs`.
- **Color arm of the declared-value serializer**:
  `crates/core/elidex-plugin/src/values.rs:594` — `CssValue::Color(c) => c.to_string()`
  → `CssColor`'s `fmt::Display` (`values.rs:728-743`): `#{:02x}{:02x}{:02x}` opaque /
  `rgba({}, {}, {}, {:.2})` translucent.
- **`CssColor`** = `values.rs:682-691` — `{ r, g, b, a: u8 }`, 8-bit per component.

### Why `CssColor::Display` must NOT change (constraint)

`CssColor::Display` (= the `CssValue::Color` arm of `to_css_string`) is the **declared
value** serializer: it backs `InlineStyle` storage, `cssText` round-trips, the
`style`-attribute write-back (re-parseable form), and the `<input type=color>`
sanitizer's `#rrggbb` canonical form (#371, `crates/dom/elidex-form/src/sanitize_tests.rs:393`).
Per CSS Color 4 §16.2, the **declared** value of a named/hex color *retains* its
author form, while the **computed/used** value is the sRGB `rgb()`/`rgba()` form.
These are two genuinely distinct serialization contexts mandated by spec — so the
resolved-value form is a **new serializer**, not a replacement (not a strangler:
declared-value `to_css_string` and resolved-value serialization are different
spec-defined operations, both permanent).

---

## §2. Coupled-invariant corner (edge-matrix)

This slice sits at the intersection of **≥3 invariant axes** (why it's edge-dense and
plan-review-mandatory):

1. **Serialization-context axis** — declared value (`#rrggbb`, retains author form,
   re-parseable) vs resolved/used value (`rgb()`/`rgba()`, CSSOM §9). The fix must add
   the second WITHOUT perturbing the first. Test both stay separate.

2. **currentcolor used-value-resolution axis** — CSSOM §9 resolved value = **used
   value**, so any `currentcolor` surfacing at the boundary must resolve to the
   element's concrete `color`. Audited state (Explore-verified): the cascade *already*
   resolves currentcolor → concrete `CssColor` for **7 of 8** implemented color props
   (`color`, `background-color`, `border-{top,right,bottom,left}-color`,
   `column-rule-color` — see `resolve/box_model/mod.rs:278-289`, `resolve/font.rs:148-167`).
   The **one** residual is `text-decoration-color`, stored `Option<CssColor>` where
   `None` = currentcolor (`computed_style/mod.rs:255`), and `get_computed` returns
   `CssValue::Keyword("currentcolor")` for it (`css-text/src/lib.rs:279-282`). So the
   boundary sees `currentcolor` from exactly one prop today, but the fix must be
   **value-shape-general** (any residual `currentcolor` keyword), not prop-name-special.

3. **alpha-precision axis** — `{:.2}` (declared form) is wrong for resolved value;
   resolved value uses CSS Color 4 §16.1 alpha rules (integer-percentage path → `n/100`,
   else `round(α/0.255)/1000`, trailing zeros trimmed, leading zero kept). Exact integer
   arithmetic on the u8 — **no f64 cancellation risk** (cf. memory f64-tolerance lesson;
   not applicable here because α is an exact 8-bit integer, not a computed float).

4. **value-type non-uniformity axis** — `CssValue::Color(c)` vs
   `CssValue::Keyword("currentcolor")` both reach the boundary as "a color". The
   serializer must handle both shapes.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CSSOM-1 §9 Resolved Values (`#resolved-values`) | color longhand resolved value = **used value** | (i) field already concrete `CssColor` (7/8 props) | `serialize_resolved_value` Color arm (NEW) | ✓ | no (read-only output) |
| CSSOM-1 §9 Resolved Values (`#resolved-values`) | color longhand resolved value = **used value** | (ii) residual `currentcolor` → element `color` | `serialize_resolved_value` currentcolor arm (NEW) | ✓ | no |
| CSSOM-1 §6.7.2 Serializing CSS Values (`#serializing-css-values`) | serialize-a-CSS-value (resolved context) | non-color values | `other => to_css_string()` (existing, unchanged) | ✓ | no |
| CSS Color 4 §16.2.2 CSS serialization of sRGB values (`#css-serialization-of-srgb`) | sRGB used value → `rgb()`/`rgba()`, comma + 1 space, base-10 [0,255] | (i) α==255 → `rgb()` | `CssColor::to_resolved_value_string` (NEW) | ✓ | no |
| CSS Color 4 §16.2.2 CSS serialization of sRGB values (`#css-serialization-of-srgb`) | sRGB used value form | (ii) α<255 → `rgba()` | `CssColor::to_resolved_value_string` (NEW) | ✓ | no |
| CSS Color 4 §16.1 Serializing alpha values (`#serializing-alpha-values`) | 8-bit α serialization | (i) integer-% preimage → n/100 | `serialize_alpha_u8` (NEW) | ✓ | no |
| CSS Color 4 §16.1 Serializing alpha values (`#serializing-alpha-values`) | 8-bit α serialization | (ii) no preimage → round(α·1000/255)/1000 | `serialize_alpha_u8` (NEW) | ✓ | no |
| CSS Selectors L4 §8.2 :link/:visited privacy (`#link`) | color props return unvisited value | no `:visited` computed divergence exists | unchanged (`computed_style.rs:47-53` note) | ✓ | no |

**Breadth**: K=3 specs (CSSOM-1, CSS Color 4, CSS Selectors L4), M=8 entries.
**Split decision**: single PR — narrowly-scoped resolved-value serialization slice; the
8 entries are one serialization concern at one boundary, not 8 independent surfaces.

### §3.1 User-input touch audit / breadth

**Breadth detail**: 8 implemented color longhands route through the single boundary
(`color`, `background-color`, `border-{top,right,bottom,left}-color`,
`text-decoration-color`, `column-rule-color`). `outline-color` / `fill` / `stroke`
are **not yet implemented** in `get_computed` (Explore-confirmed) → out of scope, will
be covered for free when added (they'll return `CssValue::Color`/`currentcolor` through
the same boundary). No new property surface is introduced.

**User-input touch audit**: **none**. This is a read-only serialization path
(getComputedStyle output). No untrusted input is parsed; no write-site/attribute
round-trip is touched (the declared-value path is explicitly left intact). No new
trust boundary.

---

## §4. DESIGN FORK (the decision plan-review must ratify)

CSSOM §9 says color resolved value = **used value**, so `currentcolor` must appear as a
concrete color at the getComputedStyle boundary. There are two structural places to make
that true. Both are presented; §4.3 states the recommendation + rationale.

### §4.1 Fork (a) — resolve at the getComputedStyle / OM boundary  *(RECOMMENDED)*

Add a resolved-value serializer used *only* by the getComputedStyle boundary. It:
- serializes `CssValue::Color(c)` via a new `CssColor::to_resolved_value_string()`
  (`rgb()`/`rgba()`, §16.2.2 + §16.1);
- maps any residual `CssValue::Keyword("currentcolor")` → `style.color` (used value),
  then serializes that;
- delegates every non-color value to the existing `to_css_string()` (unchanged).

Cascade storage (`ComputedStyle` fields, incl. `text_decoration_color: Option<CssColor>`
`None`=currentcolor) is **untouched**.

```rust
// elidex-style: the CSSOM "resolved value" serialization (§9 + §6.7.2).
pub fn serialize_resolved_value(property: &str, style: &ComputedStyle) -> String {
    match get_computed(property, style) {
        CssValue::Color(c) => c.to_resolved_value_string(),
        CssValue::Keyword(ref k) if k.eq_ignore_ascii_case("currentcolor") => {
            style.color.to_resolved_value_string()   // §9 used value
        }
        other => other.to_css_string(),
    }
}
```
Boundary becomes: `Ok(JsValue::String(serialize_resolved_value(&property, &style)))`.

**Pros**: spec frames resolved value as a *serialization-time* (OM-boundary) concept,
so this lands the transform exactly where the spec scopes it. Minimal, localized,
**zero coupling** into render-time text-decoration semantics. Value-shape-general
(handles future currentcolor-surfacing props automatically). One serializer per
serialization context = One-issue-one-way.
**Cons**: leaves `text-decoration-color` field as `Option<CssColor>` `None`=currentcolor
(a deliberate "resolve at render time" convention, `font.rs:362-365`), so the
field-level representation stays non-uniform (but that non-uniformity is *correct* —
see §4.2 con).

### §4.2 Fork (b) — resolve currentcolor upstream in the cascade

Change `text_decoration_color` to a concrete `CssColor` (drop the `Option`/`None`
convention), resolving currentcolor → `style.color` during cascade like the other 7
props. Then every color field is concrete and the boundary serializer needs no
currentcolor branch.

**Pros**: field-level uniformity — all 8 color fields concrete `CssColor`; boundary
serializer is pure (Color arm only).
**Cons**: **couples a getComputedStyle serialization slice into text-decoration render
propagation** (CSS Text Decoration §2.1 `text-decoration-line` — a decoration introduced by an ancestor is
painted by that ancestor's box and propagated to descendants; the `None`="resolve at
render time" convention exists so the decoration picks up the *originating* element's
color, `paint/mod.rs:736` + `font.rs:362-365`). Collapsing `None`→`style.color` at
cascade time risks changing *paint* behavior for propagated decorations — a different
subsystem. This is exactly the **narrow-slot-no-deferred-coupling** anti-pattern
(memory `feedback_narrow-slot-no-deferred-coupling`): a serialization slot must not
branch-flatten another subsystem's lifecycle state. Larger blast radius, touches render.

### §4.3 Recommendation → **Fork (a)**

Decided via Design-philosophy lens (memory `feedback_decide-via-philosophy-before-asking`),
not deferred to the user:
- **Ideal/spec-faithful**: CSSOM §9 *defines* resolved value as a getComputedStyle-time
  notion ("the resolved value … can be determined as follows" — a serialization-time
  query, not a stored value). Fork (a) puts the transform at the spec's own seam.
- **Narrow-slot-no-deferred-coupling**: Fork (b) couples into text-decoration paint
  propagation — out of this slot's scope, risk of behavior change in render.
- **One-issue-one-way**: Fork (a) gives exactly one resolved-value serializer for the
  one resolved-value context; it is value-shape-general, so future currentcolor props
  need no new code.

Fork (a)'s only "con" (field stays `Option`) is not a defect: the `None`=currentcolor
convention is *load-bearing for render* and correct to keep. Plan-review is asked to
confirm fork (a) (or surface a coupling I've under-weighted).

---

## §5. Implementation (fork a)

### §5.1 `CssColor::to_resolved_value_string()` — `crates/core/elidex-plugin/src/values.rs`

New method (sibling to `Display`, does **not** replace it). CSS Color 4 §16.2.2 + §16.1:

```rust
impl CssColor {
    /// CSSOM resolved/used-value serialization (CSS Color 4 §16.2.2):
    /// legacy `rgb()`/`rgba()` sRGB form. Distinct from `Display`
    /// (`#rrggbb`, the *declared* value / inline-style round-trip form).
    pub fn to_resolved_value_string(&self) -> String {
        if self.a == 255 {
            format!("rgb({}, {}, {})", self.r, self.g, self.b)
        } else {
            format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, serialize_alpha_u8(self.a))
        }
    }
}
```

`serialize_alpha_u8(a: u8) -> String` (CSS Color 4 §16.1, exact integer arithmetic):
1. **Integer-% preimage (step 2)**: for `n` in `0..=100`, if `round_half_up(n*255, 100) == a`
   → return `n/100` as a `<number>` (trailing zeros trimmed, leading zero kept). *Common
   case* — `rgba(_, .5)` stores u8 128, `n=50` → `round(127.5)=128` → `"0.5"`; `n=10`→26→
   `"0.1"`; `n=93`→237→`"0.93"`.
2. **No preimage (step 3, §16.1 closed form)**: `round(a/0.255)/1000` = the integer
   `round(a*1000/255)` over `1000`, formatted as a `<number>` (trailing zeros trimmed,
   leading zero kept). E.g. `a=236` → `round(925.49)=925` → `"0.925"`; `a=127` →
   `round(498.04)=498` → `"0.498"`. Always round-trips: the result is within 0.0005 of
   `a/255`, so `round(v*255)==a`. Implemented as `(a*1000 + 127)/255` (round-to-nearest;
   `a*1000 mod 255` is never exactly 127.5, so no tie).

   **NOTE (cross-round review correction)**: §16.1 step 3 gives this closed form
   directly. The precision is "not defined … must at least round-trip", and the worked
   example `236 → "0.92549"` is illustrative (a *longer* also-conformant form). An earlier
   draft used a "fewest-decimals search" emitting `0.926`; both plan-review (Axis 4) and
   /code-review re-flagged the step-3 area, so the implementation follows the **literal
   normative closed form** `round(a/0.255)/1000` — simplest, deterministic, normative, and
   it removes the search loop + the (previously dead) fallback branch entirely.

`round_half_up(num, den)` = `(num + den/2) / den` on integers (den even ⇒ exact half-up);
this is the alpha re-parse model `round(v*255)`, ties up. Number formatting: leading zero
kept, trailing zeros trimmed.

### §5.2 `serialize_resolved_value` — `crates/css/elidex-style/src/resolve/mod.rs` (+ re-export `lib.rs`)

As in §4.1. Lives in elidex-style (engine-independent CSS algorithm layer) next to
`get_computed`. **Not** in `vm/host/` and **not** new algorithm in dom-api — the
boundary handler just calls it (Layering mandate: algorithm in engine-independent crate,
dom-api/VM are thin callers).

**Used-value contract (F4 plan-review)**: the currentcolor arm hard-resolves *any*
residual `CssValue::Keyword("currentcolor")` reaching the boundary to `style.color`.
Invariant: **a `currentcolor` keyword at the getComputedStyle boundary always means the
element's own used-value color** (= `style.color`). This holds today because the cascade
pre-resolves currentcolor → concrete for every color prop *except* `text-decoration-color`
(`None`), whose used value per CSS Text Decoration *is* the element's color. Any §8 future
prop added as `get_computed → Keyword("currentcolor")` (e.g. `outline-color`, whose used
value is likewise the element color) inherits the correct resolution for free; a future
prop whose currentcolor must resolve to something *other* than `style.color` would need
cascade pre-resolution instead (none exists today).

### §5.3.1 Layering + ECS-native check (F5/F6 plan-review)

| New symbol | Host crate / layer | Existing sibling it sits beside |
|---|---|---|
| `CssColor::to_resolved_value_string` / `serialize_alpha_u8` | `elidex-plugin` `values::` (engine-independent value type) | `CssColor::Display` / `CssValue::to_css_string` |
| `serialize_resolved_value` | `elidex-style` `resolve::` (engine-independent CSS algorithm) | `get_computed` / `get_computed_with_registry` |
| boundary call | `elidex-dom-api` `computed_style::` (thin caller) | existing `get::<&ComputedStyle>` marshalling |

**ECS-native check**: no new ECS component, no new system/query, no side-store, no
registry, no `ObjectKind` variant. Pure read-side serializer over the already-populated
`ComputedStyle` component (cascade is the sole writer; getComputedStyle is a reader). No
OO→ECS translation surface — `CssColor` is a plain engine-independent value, not host
side-store. Verified data-flow clean via mental dry-run (no unwired read; §6 tests insert
`ComputedStyle` directly).

### §5.3 Boundary — `crates/dom/elidex-dom-api/src/computed_style.rs:54-55`

```rust
let css_value_string = serialize_resolved_value(&property, &style);  // was: get_computed(..).to_css_string()
Ok(JsValue::String(css_value_string))
```
Custom properties (`--*`) are handled *inside* `get_computed_with_registry` (returns
`RawTokens`) → flows through the `other => to_css_string()` arm unchanged. Confirm the
`--bg` test (`computed_style.rs:112-131`) still passes.

### §5.4 One-issue-one-way convergence (added after /code-review)

`CssColor::to_resolved_value_string` is the **single canonical** resolved-value color
serializer. Two pre-existing lossy-f64 copies of the `rgb()`/`rgba()` form are converged
onto it so they don't form a strangler middle-state (CLAUDE.md "One issue, one way"):
- `elidex-css-background::serialize_color` (gradient color stops, **in the live
  getComputedStyle path** — previously serialized alpha as lossy 3-dp f64, e.g. `a=128`→
  `"0.502"`, diverging from `color`'s `"0.5"`) → now delegates.
- `elidex-wpt` harness `css_value_to_string` `Color` arm (test tooling, lossy 6-dp f64) →
  now delegates.
- **Kept distinct**: `elidex-web-canvas::serialize_canvas_color` — HTML Canvas serializes
  opaque colors as `#rrggbb` (a *different* spec context), so it is correctly NOT
  converged.

---

## §6. Test plan (supported-surface)

Engine-independent unit tests (no VM needed) at the `serialize_resolved_value` /
`to_resolved_value_string` layer + the dom-api boundary:

- `color: red` → `"rgb(255, 0, 0)"` (was `#ff0000`). **Update** `get_computed_color`
  (`computed_style.rs:89`) from `matches!(_, String(_))` to assert `"rgb(255, 0, 0)"`.
- opaque border/background/column-rule → `rgb(...)`.
- translucent `CssColor::new(0,0,0,128)` → `"rgba(0, 0, 0, 0.5)"`.
- `CssColor::new(0,0,0,0)` (transparent) → `"rgba(0, 0, 0, 0)"`.
- alpha §16.1 table: 255→omitted (rgb form); 128→`0.5`; 26→`0.1` (n=10: round(25.5)=26);
  237→`0.93`; step-3 no-preimage `236 → "0.925"` (closed form `round(236/0.255)=925`),
  `127 → "0.498"`. Round-trip property test over all `a` in `0..=255`: re-parsing the
  serialized alpha yields back `a`.
- `text-decoration-color` initial (None) on element with `color: blue` →
  `"rgb(0, 0, 255)"` (currentcolor → used value). With explicit color → that color.
- **Separation guard**: `CssColor::RED.to_string()` (Display) still `"#ff0000"` and
  the `<input type=color>` sanitizer hex tests (`sanitize_tests.rs:393`) untouched —
  assert both serializers coexist.
- `--bg` custom property still `"#0d1117"` (non-color path unchanged).

WPT: `cssom/getComputedStyle-*` color subset is the supported surface this guards
(engine-independent unit coverage is primary; note any WPT we map).

---

## §7. Risk / blast radius

- **Blast radius**: getComputedStyle color output changes `#rrggbb`→`rgb()` — a
  **web-observable** behavior change (scripts reading computed color). This is the
  *correct* spec form and what every browser returns, so it's a fidelity fix, but it's
  high-visibility → **`/external-converge`** (not single-pass) per the high-blast policy,
  with Step-4 attestation + fix-delta `/elidex-review`.
- **Non-regression**: declared-value path (`to_css_string`/`Display`), inline-style
  round-trip, #371 color-well sanitizer all untouched and test-guarded (§6 separation
  guard).
- **No coupling** into render/cascade (fork a).

---

## §8. Out of scope

**Automatic future coverage (not a defer slot)** — these need no slot because the
value-shape-general boundary serializer covers them the moment their producer exists:
- `outline-color` / `fill` / `stroke` `get_computed` arms — not yet implemented in any
  plugin (Explore-verified absent from `get_computed`). When their arms are added (each
  returning `CssValue::Color` or `currentcolor`), they serialize correctly through the
  same boundary with zero new code. This is *upstream-gap coverage*, not a deferred scope
  of *this* slice.

**Named defer (separate slot, re-eval trigger stated)**:
- CSS Color 4 modern color spaces (`lab()`/`oklch()`/`color()`), `none` components, wide
  gamut — elidex `CssColor` is 8-bit sRGB only, so no producer exists to serialize.
  Tracked by slot **`#11-css-color4-extended-syntax`** (which already flags
  currentColor/system-color used-value resolution). *Re-eval trigger*: when `CssColor` is
  widened beyond 8-bit sRGB / a modern color-syntax parser lands.

**Explicitly rejected (not a follow-up)**:
- Field-level uniformity of `text_decoration_color` (fork b) — rejected on
  narrow-slot-no-deferred-coupling grounds (§4.2/§4.3); the `None`=currentcolor convention
  is render-load-bearing and correct to keep.
