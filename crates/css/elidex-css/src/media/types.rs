//! Media Queries Level 4/5 AST types + the environment the evaluator reads.
//!
//! Grammar productions (`<media-query>` etc.) are defined in mediaqueries-4
//! §3 Syntax (`#typedef-*`); §2.x carries the semantics; per-feature
//! descriptors live in MQ4 §4/§5/§6 and MQ5 §12. The types here are pure
//! values — no JS, no DOM, no engine state.

use elidex_plugin::{CalcExpr, LengthUnit};

/// A `<media-query-list>` — mediaqueries-4 §3 (`#typedef-media-query-list`).
///
/// `evaluate` ORs over the queries (§2.1 Combining Media Queries). An empty
/// list (`MediaQueryList(vec![])`, produced from an empty/whitespace query
/// string) evaluates to `true` per §2.1.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaQueryList(pub Vec<MediaQuery>);

/// A single `<media-query>` — mediaqueries-4 §3 (`#typedef-media-query`):
/// `<media-condition> | [ not | only ]? <media-type> [ and <media-condition-without-or> ]?`.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaQuery {
    /// `not` / `only` modifier — mediaqueries-4 §2.2 Media Query Modifiers.
    pub qualifier: Option<Qualifier>,
    /// `<media-type>` — §2.3. `None` for a condition-only query
    /// (e.g. `(width > 0px)` with no leading type).
    pub media_type: Option<MediaType>,
    /// `<media-condition>` — §2.5. `None` for a type-only query (e.g. `screen`).
    pub condition: Option<MediaCondition>,
}

impl MediaQuery {
    /// The `not all` sentinel a *top-level* grammar-malformed query is replaced
    /// by — mediaqueries-4 §3.2 Error Handling: a reserved keyword used as a
    /// `<media-type>` (`or`/`and`/`only`/`not`/`layer`), `and`/`or` mixed at one
    /// level, or other bare-token garbage. (An unknown feature *inside* a
    /// `( … )` is NOT this — it is `GeneralEnclosed` → Kleene unknown.) Always
    /// evaluates to `false` (its `not` is baked in, so it is false even though
    /// it carries a qualifier).
    pub(crate) fn not_all() -> Self {
        MediaQuery {
            qualifier: Some(Qualifier::Not),
            media_type: Some(MediaType::All),
            condition: None,
        }
    }
}

/// `not` | `only` — mediaqueries-4 §2.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qualifier {
    Not,
    Only,
}

/// `<media-type>` — mediaqueries-4 §2.3.
///
/// Not `Copy`: [`Other`](Self::Other) carries the ident text so it can be
/// serialized back (CSSOM-1 §4.2 step 2 — "the media type … converted to ASCII
/// lowercase"); the heap `Box<str>` makes the value non-`Copy`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaType {
    All,
    Screen,
    Print,
    /// A recognized-but-non-matching `<media-type>` ident: an unknown type, or
    /// a deprecated one (`tty`/`tv`/`projection`/`handheld`/`braille`/
    /// `embossed`/`aural`/`speech`). §2.3 + §3.2: definite-FALSE but NEGATABLE
    /// (`not <Other>` = true) — distinct from the `not all` sentinel (which is
    /// false even under `not`). Carries the **lowercased** ident (CSSOM-1 §4.2
    /// step 2 lowercases the media type) so `matchMedia('TV').media` → `tv`;
    /// without it the serializer would have no token to emit.
    Other(Box<str>),
}

/// `<media-condition>` — mediaqueries-4 §2.5 — a recursive boolean tree
/// evaluated with Kleene 3-valued logic (§3.1).
#[derive(Clone, Debug, PartialEq)]
pub enum MediaCondition {
    Feature(MediaFeature),
    Not(Box<MediaCondition>),
    And(Vec<MediaCondition>),
    Or(Vec<MediaCondition>),
    /// MQ4 §3 defines the function and parenthesized forms of `<general-enclosed>`.
    /// The parser uses this variant for unrecognized content. It evaluates to
    /// unknown under §3.1, including for unknown feature names or values (§3.2).
    ///
    /// Function source through the closing delimiter or EOF, or the trimmed
    /// contents of a parenthesized block wrapped in a new pair of parentheses.
    GeneralEnclosed(Box<str>),
}

/// `<media-feature>` — mediaqueries-4 §2.4.
#[derive(Clone, Debug, PartialEq)]
pub enum MediaFeature {
    /// MQ4 §2.4.3 defines single and chained range comparisons. Section 2.4.4
    /// maps the min- and max- prefixes to inclusive comparisons.
    ///
    /// `syntax` records which of the two equivalent notations was written, so
    /// `.media` serializes back the same one — the constraints alone can't tell
    /// `(min-width: 5px)` from `(width >= 5px)` (both are `[{Ge, 5px}]`).
    Range {
        name: RangeFeature,
        constraints: Vec<RangeConstraint>,
        syntax: RangeSyntax,
    },
    /// A discrete feature (`orientation`/`prefers-*`) with an explicit keyword
    /// value — §2.4 discrete type.
    Discrete {
        name: DiscreteFeature,
        value: DiscreteValue,
    },
    /// A feature used in boolean context `(name)` — §2.4.2: true iff the
    /// feature would be true for some value (non-zero / non-none).
    Boolean(BooleanFeature),
}

/// The source notation for a range feature. MQ4 §2.4.4 makes `min-width: 5px`
/// equivalent to `width >= 5px`. Retaining the notation alongside the constraints lets
/// `.media` serialize the original form (CSSOM-1 §4.2 only models the colon
/// notation as feature *names* — the comparison notation has no spec
/// serialization, so it follows the browser de-facto form).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeSyntax {
    /// Colon notation: `(width: 5px)` / `(min-width: 5px)` / `(max-width: 5px)`
    /// — §2.4.4. Always a single constraint with op `=` / `>=` / `<=`.
    Plain,
    /// Comparison notation: `(width >= 5px)`, `(5px <= width <= 10px)` — §2.4.3
    /// range context. One or two constraints with any comparison operator.
    Comparison,
}

/// One comparison in a range feature: `<op> <value>` — mediaqueries-4 §2.4.3.
#[derive(Clone, Debug, PartialEq)]
pub struct RangeConstraint {
    pub op: RangeOp,
    pub value: RangeValue,
}

/// A range comparison operator — mediaqueries-4 §2.4.3 (`<mf-comparison>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

impl RangeOp {
    /// The reflected operator: `a <op> b` ≡ `b <op.flipped()> a` (§2.4.3). The
    /// parser uses it to rewrite a value-first comparison (`5px < width`) to the
    /// canonical name-first orientation; the serializer flips back to recover
    /// the left operand of a two-sided range (`a <= width <= b`).
    #[must_use]
    pub(crate) fn flipped(self) -> RangeOp {
        match self {
            RangeOp::Lt => RangeOp::Gt,
            RangeOp::Le => RangeOp::Ge,
            RangeOp::Gt => RangeOp::Lt,
            RangeOp::Ge => RangeOp::Le,
            RangeOp::Eq => RangeOp::Eq,
        }
    }
}

/// A range feature's target value, resolved to a comparable `f64` at eval
/// time. Lengths keep their unit so viewport-relative units resolve against
/// the [`MediaEnvironment`] (the f32→f64 lift from `parse_length`'s
/// `CssValue::Length(f32, _)` happens here).
#[derive(Clone, Debug, PartialEq)]
pub enum RangeValue {
    /// A `<length>` for `width`/`height` in px or a viewport/font-relative unit
    /// — resolved to px at eval and compared EXACTLY. A direct/relative
    /// `<length>` is faithful to the cssparser `f32` source (px is the
    /// comparison unit; relative units multiply by an exact factor), so
    /// fractional px breakpoints (`min-width: 1024.0005px`) stay distinct to the
    /// f32 ULP — the tolerance must NOT widen them.
    Length { value: f64, unit: LengthUnit },
    /// A length-typed `calc()` for `width`/`height` — MQ4 §1.2/§1.3 delegates
    /// `<mf-value>` types/units to CSS Values, so the math tree (parsed by the
    /// canonical `elidex_plugin::css_resolve::strict::parse_length`) is carried symbolically and
    /// resolved against the queried environment at eval, not at parse —
    /// relative/viewport units (`em`, `vw`, …) need the environment. The
    /// remaining unit/math tail (abs-unit lengths, resolution `calc()`, font-
    /// relative/logical/viewport-variant units) is the carved follow-up slot
    /// `#11-media-css-values-fidelity`.
    Calc(Box<CalcExpr>),
    /// A `<ratio>` (css-values-4 §5.7) for `aspect-ratio`. The numerator and
    /// denominator are kept separate (not pre-divided) so `.media` serializes
    /// back `16 / 9` rather than `1.7777…`; eval compares `num / den`. A bare
    /// `<number>` ratio (`aspect-ratio: 2`) is `den == 1.0` (css-values-4 §5.7:
    /// `<number>` ≡ `<number> / 1`, serialized `2 / 1`).
    Ratio { num: f64, den: f64 },
    /// A `<resolution>` (css-values-4 §7.4) in dppx for `resolution`, from a
    /// `dppx`/`x` token (the canonical unit — no conversion) or the `infinite`
    /// keyword (`f64::INFINITY`, MQ4 §5.1). Compared EXACTLY.
    Dppx(f64),
    /// A unitless `<integer>` for `color` (bits per component, MQ4 §6.1).
    /// Compared EXACTLY (an `<integer>` has no conversion error).
    Number(f64),
    /// A scalar in the feature's comparison unit (px for `width`/`height`, dppx
    /// for `resolution`) resolved from a **lossy unit conversion** — a CSS
    /// absolute length (`in`/`cm`/`mm`/`q`/`pt`/`pc` → px) or a `dpi`/`dpcm`
    /// resolution. The cssparser `f32` source + conversion factor make the
    /// result inexact (`2.54cm` → 95.9999986, not 96px), so this — and ONLY
    /// this — compares with the magnitude-relative tolerance (`approx_eq`). Kept
    /// distinct from [`Length`](Self::Length)/[`Dppx`](Self::Dppx) so a *direct*
    /// fractional px/dppx breakpoint is never widened by that tolerance.
    ///
    /// `px` is the resolved comparison value (what eval reads); `value` + `unit`
    /// (the **lowercased** specified dimension, e.g. `2.54` + `cm`) are retained
    /// so `.media` serializes back `2.54cm` rather than the lossy `95.9999986px`
    /// — the conversion is one-way, so the resolved scalar can't recover it.
    Converted { px: f64, value: f64, unit: Box<str> },
}

/// The range-typed media features supported in this slice (the extended MQ5
/// feature set — `hover`/`pointer`/`update`/`overflow-*`/etc. — is the carved
/// follow-up slot `#11-media-extended-features`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeFeature {
    /// MQ4 §4.1 `width`.
    Width,
    /// MQ4 §4.2 `height`.
    Height,
    /// MQ4 §4.3 `aspect-ratio`.
    AspectRatio,
    /// MQ4 §5.1 `resolution`.
    Resolution,
    /// MQ4 §6.1 `color` (bits per color component).
    Color,
}

/// The discrete-typed media features supported in this slice (further MQ5
/// `prefers-*` features + their change-event delivery are the carved follow-up
/// slot `#11-media-prefers-features`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscreteFeature {
    /// MQ4 §4.4 `orientation`.
    Orientation,
    /// MQ5 §12.5 `prefers-color-scheme`.
    PrefersColorScheme,
    /// MQ5 §12.1 `prefers-reduced-motion`.
    PrefersReducedMotion,
}

/// A recognized discrete feature value. An unknown keyword gives the feature
/// the value unknown (MQ4 §3.2), which propagates according to §3.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscreteValue {
    Portrait,
    Landscape,
    Light,
    Dark,
    NoPreferenceMotion,
    Reduce,
}

/// A feature usable in boolean context `(name)` — mediaqueries-4 §2.4.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanFeature {
    Width,
    Height,
    AspectRatio,
    Resolution,
    Orientation,
    /// MQ4 §6.1 `color`.
    Color,
    PrefersColorScheme,
    PrefersReducedMotion,
}

/// The environment the evaluator reads — the consumer (Slice 2 VM / Slice 3
/// CSS cascade) constructs this and calls in; the evaluator never reads
/// global/engine state. All lengths are CSS px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaEnvironment {
    /// The output medium (`screen`/`print`) the `<media-type>` matches against —
    /// §2.3. Defaults to `Screen` (the `matchMedia` case).
    pub medium: Medium,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub resolution_dppx: f64,
    /// The initial font-size in CSS px that media-query relative lengths
    /// (`em`/`rem`) resolve against — MQ4 §1.3: relative units use the initial
    /// value defined by the UA or user preferences, never a declared/element
    /// font-size. Typically the UA `medium` default (16px), but a user with a
    /// larger default font reports it here, shifting `em`-based breakpoints.
    pub root_font_size_px: f64,
    /// Bits per color component (0 = monochrome / not a color device) — MQ4 §6.1.
    pub color_bits: u16,
    pub color_scheme: ColorScheme,
    pub reduced_motion: ReducedMotion,
}

/// `prefers-color-scheme` user preference — MQ5 §12.5. The feature value is
/// `light | dark` only; a UA with no active preference reports `light` (UA
/// convention), so there is no separate "no-preference" state — the default is
/// `Light`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

/// `prefers-reduced-motion` user preference — MQ5 §12.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ReducedMotion {
    #[default]
    NoPreference,
    Reduce,
}

pub use elidex_plugin::Medium;

impl Default for MediaEnvironment {
    /// A screen UA at 1024×768, 1dppx, no user preference — lets Slice-1 unit
    /// tests (and a rewired boa) construct an environment with no engine
    /// coupling. The real values are wired by Slice 2 (`HostDriver` viewport
    /// transport).
    fn default() -> Self {
        MediaEnvironment {
            medium: Medium::Screen,
            viewport_width: 1024.0,
            viewport_height: 768.0,
            resolution_dppx: 1.0,
            // CSS initial font-size (`medium` = 16px) — the UA default basis for
            // `em`/`rem` in media queries (§1.3).
            root_font_size_px: 16.0,
            color_bits: 8,
            color_scheme: ColorScheme::Light,
            reduced_motion: ReducedMotion::NoPreference,
        }
    }
}
