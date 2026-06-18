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
        Some(t) => match_media_type(t),
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

/// Match a `<media-type>` on a screen UA — §2.3 / §3.2.
fn match_media_type(t: MediaType) -> Tri {
    match t {
        MediaType::All | MediaType::Screen => Tri::True,
        // `print` never matches on a screen-only UA; a recognized-but-
        // non-matching ident (`Other`) is also definite-false here — its
        // negatability comes from the query qualifier (`not <Other>` = true),
        // applied in `eval_query`, not from this arm.
        MediaType::Print | MediaType::Other => Tri::False,
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
            Tri::from_bool(
                constraints
                    .iter()
                    .all(|c| compare(actual, c.op, resolve_range_value(&c.value, env))),
            )
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
/// environment — lengths resolve here so viewport-relative units use the
/// queried viewport.
fn resolve_range_value(value: &RangeValue, env: &MediaEnvironment) -> f64 {
    match value {
        RangeValue::Length { value, unit } => resolve_px(*value, *unit, env),
        RangeValue::Calc(expr) => resolve_calc(expr, env),
        RangeValue::Ratio(r) => *r,
        RangeValue::Dppx(d) => *d,
        RangeValue::Number(n) => *n,
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

/// `<mf-comparison>` numeric comparison — §2.4.3.
fn compare(actual: f64, op: RangeOp, target: f64) -> bool {
    match op {
        RangeOp::Lt => actual < target,
        RangeOp::Le => actual <= target,
        RangeOp::Gt => actual > target,
        RangeOp::Ge => actual >= target,
        RangeOp::Eq => actual == target,
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
