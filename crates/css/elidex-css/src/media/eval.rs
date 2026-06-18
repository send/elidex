//! Kleene 3-valued evaluation — mediaqueries-4 §3.1 Evaluating Media Queries.
//!
//! The internal evaluation is three-valued ([`Tri`]); the public [`evaluate`]
//! coerces the top-level `Unknown → false` exactly once, at the 2-valued
//! boundary (§3.1: "if the result is used in a two-valued boolean context,
//! unknown must be converted to false"). No internal site collapses `Unknown`
//! early — that asymmetry is the whole point of the Kleene logic (so that
//! `not <general-enclosed>` does not become `true`).

use elidex_plugin::{CalcExpr, LengthUnit};

#[allow(clippy::wildcard_imports)]
use super::types::*;

/// Internal Kleene 3-valued truth — mediaqueries-4 §3.1. NEVER surfaces past
/// the public [`evaluate`] boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    fn from_bool(b: bool) -> Tri {
        if b {
            Tri::True
        } else {
            Tri::False
        }
    }

    /// Kleene negation — §3.1 "the negation of unknown is unknown".
    fn negate(self) -> Tri {
        match self {
            Tri::True => Tri::False,
            Tri::False => Tri::True,
            Tri::Unknown => Tri::Unknown,
        }
    }
}

/// Kleene AND — §3.1: `False` if any term is false, else `Unknown` if any is
/// unknown, else `True`.
fn and_tri(a: Tri, b: Tri) -> Tri {
    match (a, b) {
        (Tri::False, _) | (_, Tri::False) => Tri::False,
        (Tri::Unknown, _) | (_, Tri::Unknown) => Tri::Unknown,
        _ => Tri::True,
    }
}

/// Kleene OR — §3.1: `True` if any term is true, else `Unknown` if any is
/// unknown, else `False`.
fn or_tri(a: Tri, b: Tri) -> Tri {
    match (a, b) {
        (Tri::True, _) | (_, Tri::True) => Tri::True,
        (Tri::Unknown, _) | (_, Tri::Unknown) => Tri::Unknown,
        _ => Tri::False,
    }
}

/// Evaluate a `<media-query-list>` against an environment — mediaqueries-4
/// §3.1. ORs over the queries (§2.1). An empty list evaluates to `true` (§2.1).
/// This is the sole 2-valued boundary: each query's top-level `Tri::Unknown`
/// is coerced to `false` here.
#[must_use]
pub fn evaluate(list: &MediaQueryList, env: &MediaEnvironment) -> bool {
    // §2.1: an empty media query list evaluates to true.
    if list.0.is_empty() {
        return true;
    }
    list.0.iter().any(|q| eval_query(q, env) == Tri::True)
}

/// Evaluate one `<media-query>`: `(type-match AND condition)` then apply the
/// `not`/`only` qualifier — §3.1.
fn eval_query(query: &MediaQuery, env: &MediaEnvironment) -> Tri {
    let type_tri = match query.media_type {
        None => Tri::True, // condition-only query
        Some(t) => match_media_type(t, env.medium),
    };
    let cond_tri = match &query.condition {
        None => Tri::True, // type-only query
        Some(c) => eval_condition(c, env),
    };
    let body = and_tri(type_tri, cond_tri);
    match query.qualifier {
        // `not` negates (Kleene). `only` is a legacy no-op at eval (§2.2).
        Some(Qualifier::Not) => body.negate(),
        Some(Qualifier::Only) | None => body,
    }
}

/// Match a `<media-type>` against the environment's output medium — §2.3 / §3.2.
/// `screen`/`print` match iff they equal the device medium (so `@media print`
/// applies in paged output and not on screen, and vice versa); `all` matches
/// any medium. A recognized-but-non-matching ident (`Other`) is definite-false
/// — its negatability comes from the query qualifier (`not <Other>` = true),
/// applied in `eval_query`, not from this arm.
fn match_media_type(query: MediaType, medium: Medium) -> Tri {
    match query {
        MediaType::All => Tri::True,
        MediaType::Screen => Tri::from_bool(medium == Medium::Screen),
        MediaType::Print => Tri::from_bool(medium == Medium::Print),
        MediaType::Other => Tri::False,
    }
}

/// Evaluate a `<media-condition>` with Kleene logic — §3.1.
fn eval_condition(cond: &MediaCondition, env: &MediaEnvironment) -> Tri {
    match cond {
        MediaCondition::Feature(f) => eval_feature(f, env),
        MediaCondition::Not(inner) => eval_condition(inner, env).negate(),
        MediaCondition::And(terms) => terms
            .iter()
            .fold(Tri::True, |acc, t| and_tri(acc, eval_condition(t, env))),
        MediaCondition::Or(terms) => terms
            .iter()
            .fold(Tri::False, |acc, t| or_tri(acc, eval_condition(t, env))),
        // §3.1: `<general-enclosed>` evaluates to unknown.
        MediaCondition::GeneralEnclosed => Tri::Unknown,
    }
}

/// Evaluate a single `<media-feature>` against the environment — a feature
/// always resolves to a definite true/false here (an unknown feature
/// name/value was already turned into `not all` at parse, §3.2).
fn eval_feature(feature: &MediaFeature, env: &MediaEnvironment) -> Tri {
    match feature {
        MediaFeature::Range { name, constraints } => {
            let actual = range_feature_value(*name, env);
            Tri::from_bool(constraints.iter().all(|c| {
                let (target, tolerant) = resolve_range_value(&c.value, env);
                compare(actual, c.op, target, tolerant)
            }))
        }
        MediaFeature::Discrete { name, value } => {
            Tri::from_bool(match_discrete(*name, *value, env))
        }
        MediaFeature::Boolean(bf) => Tri::from_bool(eval_boolean(*bf, env)),
    }
}

/// The actual value of a range feature in the environment.
fn range_feature_value(name: RangeFeature, env: &MediaEnvironment) -> f64 {
    match name {
        RangeFeature::Width => env.viewport_width,
        RangeFeature::Height => env.viewport_height,
        // §4.3: width / height. A degenerate zero height yields ±inf or NaN
        // (f64 division), which `compare` handles correctly — not a collapse to 0.
        RangeFeature::AspectRatio => env.viewport_width / env.viewport_height,
        RangeFeature::Resolution => env.resolution_dppx,
        RangeFeature::Color => f64::from(env.color_bits),
    }
}

/// Resolve a parsed [`RangeValue`] to a comparable `f64` against the
/// environment, plus whether it compares with tolerance (`true`) or exactly
/// (`false`). Lengths resolve here so viewport-relative units use the queried
/// viewport. Only a value from a lossy unit conversion
/// ([`RangeValue::Converted`]) is tolerant — a direct px/relative `<length>`,
/// `<ratio>`, exact `<resolution>`, and `<integer>` compare exactly, so
/// fractional breakpoints are never widened (MQ4 §2.4.3).
fn resolve_range_value(value: &RangeValue, env: &MediaEnvironment) -> (f64, bool) {
    match value {
        RangeValue::Length { value, unit } => (resolve_px(*value, *unit, env), false),
        RangeValue::Calc(expr) => {
            let v = resolve_calc(expr, env);
            // css-values-4 §10.9.2 + §10.12: a top-level calc() result has its
            // special values censored, then is clamped to the target context's
            // allowed range. NaN (e.g. `calc(0px / 0)`, or a `<percentage>` the
            // parser excludes) is censored to 0 (§10.9.2: "NaN … is censored
            // into a zero value"); negatives clamp to the non-negative range of
            // width/height (the only calc-bearing features — "width is false in
            // the negative range", MQ4 §4.1), so `(width: calc(-100px))` and
            // `(width: calc(0px / 0))` both resolve to `0px`. (A literal negative
            // length is NOT clamped — it stays "false in the negative range",
            // the css-values "-5px ≠ calc(-5px)" rule.) +∞ is left as-is so an
            // `(min-width: calc(infinity * 1px))` comparison stays false.
            let clamped = if v.is_nan() { 0.0 } else { v.max(0.0) };
            (clamped, false)
        }
        RangeValue::Ratio(r) => (*r, false),
        RangeValue::Dppx(d) => (*d, false),
        RangeValue::Number(n) => (*n, false),
        RangeValue::Converted(v) => (*v, true),
    }
}

/// Resolve a length-typed `calc()` tree to CSS px against the environment —
/// MQ4 §1.2/§1.3 delegates `<mf-value>` to CSS Values. `<length>` leaves
/// resolve via [`resolve_px`] (so relative/viewport units use the queried
/// environment); `<number>` leaves are unitless multipliers/divisors. A
/// `<percentage>` cannot appear — the parser only admits a length-typed,
/// percentage-free `calc()` for `width`/`height` — but it maps to `NaN` (any
/// comparison against which is false) rather than silently contributing 0.
fn resolve_calc(expr: &CalcExpr, env: &MediaEnvironment) -> f64 {
    match expr {
        CalcExpr::Length(v, unit) => resolve_px(f64::from(*v), *unit, env),
        CalcExpr::Number(n) => f64::from(*n),
        CalcExpr::Percentage(_) => f64::NAN,
        CalcExpr::Add(a, b) => resolve_calc(a, env) + resolve_calc(b, env),
        CalcExpr::Sub(a, b) => resolve_calc(a, env) - resolve_calc(b, env),
        CalcExpr::Mul(a, b) => resolve_calc(a, env) * resolve_calc(b, env),
        CalcExpr::Div(a, b) => resolve_calc(a, env) / resolve_calc(b, env),
    }
}

/// Resolve a `<length>` to CSS px. Media-query relative units use the initial
/// values (MQ4 §1.3): `em`/`rem` against the environment's initial font-size
/// (`root_font_size_px`, the UA/user default — never a declared font-size),
/// viewport units against the queried viewport.
#[allow(clippy::match_same_arms)] // `px` (identity) is kept explicit alongside the non_exhaustive fallback.
fn resolve_px(value: f64, unit: LengthUnit, env: &MediaEnvironment) -> f64 {
    match unit {
        LengthUnit::Px => value,
        LengthUnit::Em | LengthUnit::Rem => value * env.root_font_size_px,
        LengthUnit::Vw => value / 100.0 * env.viewport_width,
        LengthUnit::Vh => value / 100.0 * env.viewport_height,
        LengthUnit::Vmin => value / 100.0 * env.viewport_width.min(env.viewport_height),
        LengthUnit::Vmax => value / 100.0 * env.viewport_width.max(env.viewport_height),
        // `LengthUnit` is `#[non_exhaustive]`; any future unit falls back to a
        // px interpretation (parse only admits the units handled above).
        _ => value,
    }
}

/// Relative tolerance for [`RangeValue::Converted`] comparisons only.
///
/// A value resolved from a lossy unit conversion (`2.54cm` → 95.9999986px,
/// `dpi`/`dpcm` → dppx) cannot round-trip exactly: the cssparser `f32` source
/// (`2.54f32` ≈ 2.5399999…) carries ≤ ~3×10⁻⁸ relative error through the
/// conversion factor. A magnitude-relative tolerance (cancellation-aware, per
/// the f64-tolerance lesson — relative, not absolute, so it tracks the value's
/// scale) absorbs that, with `1e-6` leaving a ~30× margin over the worst-case
/// f32 conversion error. Crucially this tolerance is **scoped to converted
/// values**: direct px/relative `<length>`, `<ratio>`, exact `<resolution>`,
/// and `<integer>` (`color`) compare exactly (see `compare`/`resolve_range_value`),
/// so the tolerance can never widen a direct fractional breakpoint. (A *global*
/// tolerance can't: the conversion error ~3×10⁻⁸ and one f32 ULP ~6×10⁻⁸ nearly
/// coincide, so no single epsilon both absorbs conversion error and preserves
/// f32-distinct breakpoints — scoping decouples the two.)
const VALUE_REL_EPS: f64 = 1e-6;

/// Magnitude-relative approximate equality for lossy-converted media values:
/// equal if the difference is within [`VALUE_REL_EPS`] of the larger magnitude
/// (floored at 1.0, so sub-unit values keep an absolute ~1e-6 tolerance).
/// Non-finite inputs compare exactly — `∞ == ∞`, but `∞ ≠` any finite value and
/// `NaN ≠ NaN`.
fn approx_eq(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    (a - b).abs() <= VALUE_REL_EPS * a.abs().max(b.abs()).max(1.0)
}

/// `<mf-comparison>` numeric comparison — §2.4.3. `tolerant` selects the
/// magnitude-relative equality ([`approx_eq`]) for lossy-converted values
/// ([`RangeValue::Converted`]); every other value compares exactly, so the
/// equality boundary is f32-faithful and fractional breakpoints stay distinct.
///
/// The tolerance is gated to **non-negative** targets: all range features
/// (width/height/resolution/…) are "false in the negative range" (§2.4.3), so a
/// negative converted target (`(width: -0.00000001cm)`) must compare exactly —
/// otherwise `approx_eq(0, -tiny)` would rescue a negative breakpoint onto a
/// zero viewport.
fn compare(actual: f64, op: RangeOp, target: f64, tolerant: bool) -> bool {
    let eq = if tolerant && target >= 0.0 {
        approx_eq(actual, target)
    } else {
        actual == target
    };
    match op {
        RangeOp::Lt => actual < target && !eq,
        RangeOp::Le => actual < target || eq,
        RangeOp::Gt => actual > target && !eq,
        RangeOp::Ge => actual > target || eq,
        RangeOp::Eq => eq,
    }
}

/// Match a discrete feature value against the environment.
fn match_discrete(name: DiscreteFeature, value: DiscreteValue, env: &MediaEnvironment) -> bool {
    match (name, value) {
        // §4.4: portrait iff height ≥ width; landscape iff width > height.
        (DiscreteFeature::Orientation, DiscreteValue::Portrait) => {
            env.viewport_height >= env.viewport_width
        }
        (DiscreteFeature::Orientation, DiscreteValue::Landscape) => {
            env.viewport_width > env.viewport_height
        }
        (DiscreteFeature::PrefersColorScheme, DiscreteValue::Light) => {
            env.color_scheme == ColorScheme::Light
        }
        (DiscreteFeature::PrefersColorScheme, DiscreteValue::Dark) => {
            env.color_scheme == ColorScheme::Dark
        }
        (DiscreteFeature::PrefersReducedMotion, DiscreteValue::NoPreferenceMotion) => {
            env.reduced_motion == ReducedMotion::NoPreference
        }
        (DiscreteFeature::PrefersReducedMotion, DiscreteValue::Reduce) => {
            env.reduced_motion == ReducedMotion::Reduce
        }
        // A value not valid for this feature is unreachable: parse rejected it
        // (unknown `<mf-value>` → `not all`).
        _ => false,
    }
}

/// Evaluate a feature in boolean context `(name)` — §2.4.2: true iff the
/// feature would be true for some value (non-zero / non-none).
// `Width` and `AspectRatio` share an expression but are distinct features kept
// explicit: the equality is incidental (see the `AspectRatio` comment), not a
// shared semantic, so they must not be merged into one pattern.
#[allow(clippy::match_same_arms)]
fn eval_boolean(bf: BooleanFeature, env: &MediaEnvironment) -> bool {
    match bf {
        BooleanFeature::Width => env.viewport_width != 0.0,
        BooleanFeature::Height => env.viewport_height != 0.0,
        // §2.4.2: boolean context is true iff the feature's value is non-zero,
        // and `aspect-ratio` = width / height (§4.3). That ratio is zero only
        // when the width is zero (a zero *height* yields ±∞, which is non-zero),
        // so this must mirror `range_feature_value` — keyed on width, not height
        // — else `(aspect-ratio)` disagrees with `(aspect-ratio > 0/1)` in the
        // zero-height viewport.
        BooleanFeature::AspectRatio => env.viewport_width != 0.0,
        // §2.4.2/§5.1: true for any non-zero resolution, including `infinite`.
        BooleanFeature::Resolution => env.resolution_dppx > 0.0,
        // §6.1: `(color)` is true iff the device has a non-zero color depth.
        BooleanFeature::Color => env.color_bits > 0,
        // A viewport always has an orientation; prefers-color-scheme always
        // resolves to light or dark — both are true in boolean context.
        BooleanFeature::Orientation | BooleanFeature::PrefersColorScheme => true,
        // True iff the user asked for reduced motion (no-preference is "off").
        BooleanFeature::PrefersReducedMotion => env.reduced_motion == ReducedMotion::Reduce,
    }
}
