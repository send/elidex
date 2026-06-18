//! CSS Media Queries Level 4/5 — engine-independent parse + evaluate.
//!
//! A pure `parse + evaluate` library for media queries — no JS, no DOM, no
//! engine state. Consumers build a [`MediaEnvironment`] and call in:
//!
//! - [`parse_media_query_list`] — total parser (mediaqueries-4 §3 Syntax,
//!   §3.2 Error Handling): never errors or panics; a malformed or
//!   unknown-feature query becomes `not all`.
//! - [`evaluate`] — Kleene 3-valued evaluation (mediaqueries-4 §3.1), with the
//!   `Unknown → false` coercion applied once at the public boundary.
//!
//! This is the single SSoT media-query evaluator: the VM `matchMedia` (Slice
//! 2) and the CSS `@media` cascade (Slice 3) both consume it, replacing boa's
//! `evaluate_media_query_raw` string-splitter.

mod eval;
mod parse;
mod types;

pub use eval::evaluate;
pub use parse::parse_media_query_list;
pub use types::{
    BooleanFeature, ColorScheme, DiscreteFeature, DiscreteValue, MediaCondition, MediaEnvironment,
    MediaFeature, MediaQuery, MediaQueryList, MediaType, Qualifier, RangeConstraint, RangeFeature,
    RangeOp, RangeValue, ReducedMotion,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// 1024×768 screen, 1dppx, no preference.
    fn landscape() -> MediaEnvironment {
        MediaEnvironment::default()
    }

    fn portrait() -> MediaEnvironment {
        MediaEnvironment {
            viewport_width: 768.0,
            viewport_height: 1024.0,
            ..MediaEnvironment::default()
        }
    }

    /// Parse + evaluate against an environment.
    fn matches(query: &str, env: &MediaEnvironment) -> bool {
        evaluate(&parse_media_query_list(query), env)
    }

    // --- §3 empty list -----------------------------------------------------

    #[test]
    fn empty_list_is_true() {
        assert_eq!(parse_media_query_list(""), MediaQueryList(Vec::new()));
        assert!(matches("", &landscape()));
        assert!(matches("   ", &landscape()));
    }

    // --- §2.3 / §3.2 media types + negation asymmetry ----------------------

    #[test]
    fn media_types_screen_ua() {
        assert!(matches("all", &landscape()));
        assert!(matches("screen", &landscape()));
        assert!(!matches("print", &landscape()));
        assert!(!matches("not screen", &landscape()));
        assert!(matches("not print", &landscape()));
        assert!(matches("only screen", &landscape()));
    }

    #[test]
    fn unknown_media_type_is_negatable_false() {
        // §3.2: unknown/deprecated type → definite false, but `not` negates it.
        assert!(!matches("aural", &landscape()));
        assert!(matches("not aural", &landscape()));
        assert!(!matches("tty", &landscape()));
        assert!(matches("not tty", &landscape()));
    }

    #[test]
    fn reserved_keyword_as_type_is_not_all() {
        // `or`/`and` cannot be a media type → grammar fail → not all → false,
        // and `not all` is false even under a leading `not`.
        assert!(!matches("or", &landscape()));
        assert!(!matches("and", &landscape()));
    }

    // --- §3.2 unknown feature → parse-time not all -------------------------

    #[test]
    fn unknown_feature_name_is_not_all() {
        // Feature-shaped but unknown → whole query `not all` (false), and
        // `not (...)` is ALSO false (not Kleene-negated).
        assert!(!matches("(max-weight: 3kg)", &landscape()));
        assert!(!matches("not (max-weight: 3kg)", &landscape()));
        assert!(!matches("(unknownboolfeature)", &landscape()));
    }

    #[test]
    fn unknown_feature_nukes_only_its_own_query_in_a_list() {
        // §3.2: recovery at the top-level comma — the sibling survives.
        assert!(matches(
            "(max-weight: 3kg), (min-width: 500px)",
            &landscape()
        ));
        assert!(matches("(max-weight: 3kg), screen", &landscape()));
        assert!(!matches(
            "(max-weight: 3kg), (min-width: 5000px)",
            &landscape()
        ));
    }

    #[test]
    fn unknown_value_for_known_feature_is_not_all() {
        // `(width: foo)` matches mf-plain grammar but the value is invalid →
        // §3.2 → not all, and so is `... or (width: foo)`.
        assert!(!matches("(width: foo)", &landscape()));
        assert!(!matches("(color) or (width: foo)", &landscape()));
    }

    // --- §2.4.4 min-/max- gating ------------------------------------------

    #[test]
    fn min_max_on_discrete_is_not_all() {
        assert!(!matches("(min-orientation: portrait)", &landscape()));
        assert!(!matches("(max-orientation: landscape)", &landscape()));
    }

    #[test]
    fn min_max_in_boolean_context_is_not_all() {
        assert!(!matches("(min-width)", &landscape()));
        assert!(!matches("(max-height)", &landscape()));
    }

    // --- §3.1 Kleene 3-valued logic ---------------------------------------

    #[test]
    fn general_enclosed_is_unknown_not_false() {
        // `not <general-enclosed>` must NOT become true (the whole point of
        // Kleene logic); at the boundary it coerces to false.
        assert!(!matches("not (weird-fn(x))", &landscape()));
        assert!(!matches("(weird-fn(x))", &landscape()));
    }

    #[test]
    fn kleene_or_and_with_unknown() {
        // true OR unknown = true; false AND unknown = false.
        assert!(matches("(color) or (weird-fn(x))", &landscape()));
        assert!(!matches(
            "(min-width: 999999px) and (weird-fn(x))",
            &landscape()
        ));
        // unknown OR false-feature stays unknown → boundary false.
        assert!(!matches(
            "(weird-fn(x)) or (min-width: 999999px)",
            &landscape()
        ));
    }

    // --- §2.4.3 range features + boundaries --------------------------------

    #[test]
    fn width_height_boundaries_inclusive() {
        let env = landscape(); // 1024×768
        assert!(matches("(min-width: 1024px)", &env));
        assert!(matches("(max-width: 1024px)", &env));
        assert!(!matches("(min-width: 1025px)", &env));
        assert!(matches("(width: 1024px)", &env));
        assert!(matches("(min-height: 768px)", &env));
        assert!(!matches("(min-height: 769px)", &env));
    }

    #[test]
    fn l4_range_syntax_both_sides() {
        let env = landscape(); // width 1024
        assert!(matches("(width >= 500px)", &env));
        assert!(!matches("(width < 500px)", &env));
        assert!(!matches("(400px <= width <= 700px)", &env));
        assert!(matches("(400px <= width <= 2000px)", &env));
        // value-first single side: `500px < width` ≡ `width > 500px`.
        assert!(matches("(500px < width)", &env));
        assert!(!matches("(2000px < width)", &env));
    }

    #[test]
    fn malformed_two_sided_range_is_not_all() {
        // §3 `<mf-range>`: a two-sided form requires both ops same-direction and
        // forbids `=`. A recognized feature with a comparison has *committed* to
        // the `<mf-range>` shape, so a mixed/`=` form is a malformed feature →
        // §3.2 → `not all`, NOT `<general-enclosed>` (which is reserved for
        // content that never commits to a feature — see
        // `range_without_operator_is_general_enclosed`). (C1, corrected per R3-5.)
        let env = landscape(); // width 1024
        assert!(!matches("(400px < width > 700px)", &env)); // mixed `<`…`>`
        assert!(!matches("not (400px = width = 700px)", &env)); // `=` two-sided
                                                                // committed-but-malformed makes the whole `<media-query>` `not all`, so a
                                                                // sibling `or` cannot rescue it (contrast `unknown OR true` = true).
        assert!(!matches(
            "(400px < width > 700px) or (min-width: 1px)",
            &env
        ));
        // single-sided `=` value-first stays valid (§ allows `<mf-eq>` there).
        assert!(matches("(1024px = width)", &env));
    }

    #[test]
    fn range_without_operator_is_general_enclosed() {
        // `(width 500px)` has no `:` or comparison, so it never commits to a
        // feature shape — it matches `<general-enclosed>` (Kleene unknown), NOT
        // `not all`, and so does not poison a sibling `or` term. (WPT
        // mq-range-001: `range syntax without operator isn't valid syntax`.)
        let env = landscape();
        assert!(matches("(width 500px) or (min-width: 1px)", &env)); // unknown OR true
        assert!(!matches("(width 500px)", &env)); // unknown → false at boundary
    }

    #[test]
    fn aspect_ratio_and_resolution() {
        let env = landscape(); // 1024/768 ≈ 1.333
        assert!(matches("(min-aspect-ratio: 1/1)", &env));
        assert!(!matches("(min-aspect-ratio: 2/1)", &env));
        assert!(matches("(min-resolution: 1dppx)", &env));
        assert!(!matches("(min-resolution: 2dppx)", &env));
        assert!(matches("(resolution: 96dpi)", &env)); // 96dpi == 1dppx
    }

    #[test]
    fn negative_ratio_components_are_not_all() {
        // `<ratio>` is `[0,∞]` (css-values-4 §5.7) — a negative component is
        // outside the value syntax → §3.2 `not all`. (I1 regression.)
        let env = landscape();
        assert!(!matches("(min-aspect-ratio: -1/1)", &env));
        assert!(!matches("(aspect-ratio: -2)", &env));
        assert!(!matches("(min-aspect-ratio: 1/-1)", &env));
    }

    // --- §4.4 orientation + MQ5 §12 prefers-* -----------------------------

    #[test]
    fn orientation_flips_with_viewport() {
        assert!(matches("(orientation: landscape)", &landscape()));
        assert!(!matches("(orientation: portrait)", &landscape()));
        assert!(matches("(orientation: portrait)", &portrait()));
        assert!(!matches("(orientation: landscape)", &portrait()));
    }

    #[test]
    fn prefers_color_scheme_and_reduced_motion() {
        let dark = MediaEnvironment {
            color_scheme: ColorScheme::Dark,
            ..landscape()
        };
        assert!(matches("(prefers-color-scheme: dark)", &dark));
        assert!(!matches("(prefers-color-scheme: light)", &dark));
        // §12.5: no separate no-preference — the default UA value is `light`,
        // so default users match `light` and not `dark`.
        assert!(matches("(prefers-color-scheme: light)", &landscape()));
        assert!(!matches("(prefers-color-scheme: dark)", &landscape()));

        let reduce = MediaEnvironment {
            reduced_motion: ReducedMotion::Reduce,
            ..landscape()
        };
        assert!(matches("(prefers-reduced-motion: reduce)", &reduce));
        assert!(matches(
            "(prefers-reduced-motion: no-preference)",
            &landscape()
        ));
        assert!(!matches("(prefers-reduced-motion: reduce)", &landscape()));
    }

    // --- Codex R1 regressions ---------------------------------------------

    #[test]
    fn color_is_a_range_feature() {
        // §6.1: color is a range feature; `(color)` and `(min-color: 1)` are
        // equivalent on a color device. (F2 regression.)
        let env = landscape(); // color_bits = 8
        assert!(matches("(color)", &env));
        assert!(matches("(min-color: 1)", &env));
        assert!(matches("(color: 8)", &env));
        assert!(matches("(color >= 8)", &env));
        assert!(!matches("(min-color: 9)", &env));
        let mono = MediaEnvironment {
            color_bits: 0,
            ..landscape()
        };
        assert!(!matches("(color)", &mono));
        assert!(!matches("(min-color: 1)", &mono));
    }

    #[test]
    fn whitespace_inside_comparison_operator_fails() {
        // §3: no whitespace between `<`/`>` and `=`; `(width < = 2000px)` is
        // malformed → not all, while `(width <= 2000px)` is valid. (F3.)
        let env = landscape(); // width 1024
        assert!(matches("(width <= 2000px)", &env));
        assert!(!matches("(width < = 2000px)", &env));
        assert!(!matches("(width > = 1px)", &env));
    }

    #[test]
    fn grouped_unknown_feature_is_not_all() {
        // §3.2: an unknown feature inside a group poisons the whole query to
        // `not all`; it must NOT be rescued into Kleene unknown. (F4.)
        assert!(!matches(
            "((max-weight: 3kg) or (min-width: 1px)) or (color)",
            &landscape()
        ));
        assert!(!matches("((max-weight: 3kg)) or (color)", &landscape()));
        // a genuinely general-enclosed group still works (true OR unknown).
        assert!(matches("(color) or (weird-fn(x))", &landscape()));
    }

    #[test]
    fn absolute_length_units_in_features() {
        // §4.1/§1.3: width/height accept CSS absolute lengths (96dpi). (F5.)
        let env = landscape(); // width 1024px
        assert!(matches("(min-width: 10in)", &env)); // 10in = 960px ≤ 1024
        assert!(!matches("(min-width: 11in)", &env)); // 11in = 1056px > 1024
        assert!(matches("(min-width: 20cm)", &env)); // 20cm ≈ 756px ≤ 1024
        assert!(matches("(min-width: 100pt)", &env)); // 100pt ≈ 133px ≤ 1024
    }

    #[test]
    fn infinite_resolution_keyword() {
        // §5.1: `resolution = <resolution> | infinite`. (F6.)
        let env = landscape(); // 1dppx
        assert!(matches("(max-resolution: infinite)", &env));
        assert!(!matches("(min-resolution: infinite)", &env));
    }

    // --- Codex R2 regressions ---------------------------------------------

    #[test]
    fn color_accepts_negative_integers() {
        // §2.4.3: `color` is false in the negative range, but negative values
        // must parse and reach `compare`. (R2-1.)
        let env = landscape(); // color_bits = 8
        assert!(matches("(color > -1)", &env)); // 8 > -1
        assert!(matches("(min-color: -1)", &env)); // 8 >= -1
        assert!(!matches("(max-color: -1)", &env)); // 8 <= -1 is false
        assert!(matches("not (color <= -1)", &env));
    }

    #[test]
    fn infinite_resolution_boolean_is_true() {
        // §2.4.2/§5.1: an infinite (non-zero) resolution satisfies `(resolution)`.
        // (R2-2.)
        let inf = MediaEnvironment {
            resolution_dppx: f64::INFINITY,
            ..landscape()
        };
        assert!(matches("(resolution)", &inf));
        assert!(matches("(max-resolution: infinite)", &inf));
    }

    #[test]
    fn malformed_known_feature_value_is_not_all() {
        // §3.2: a known feature with a value that doesn't match its syntax (extra
        // tokens) → not all, not general-enclosed (must not match via OR). (R2-3.)
        let env = landscape();
        assert!(!matches("(width: 1px 2px)", &env));
        assert!(!matches("(color) or (width: 1px 2px)", &env));
    }

    #[test]
    fn degenerate_ratios_parse() {
        // css-values-4 §5.7: `<ratio>` is `[0,∞]` on both sides; a zero
        // denominator is a valid degenerate ratio (→ ±inf). (R2-4.)
        let env = landscape(); // 1024/768 ≈ 1.333
        assert!(matches("(aspect-ratio < 1/0)", &env)); // 1.333 < inf
        assert!(matches("not (aspect-ratio: 1/0)", &env)); // 1.333 == inf is false
    }

    #[test]
    fn whitespace_around_ratio_slash_parses() {
        // css-values-4 §2.5/§5.7: whitespace around the `/` is allowed. (R2-6.)
        let env = landscape(); // 1.333
        assert!(matches("(min-aspect-ratio: 1 / 1)", &env)); // 1.333 >= 1
        assert!(!matches("(min-aspect-ratio: 2 / 1)", &env)); // 1.333 >= 2 is false
    }

    #[test]
    fn zero_height_aspect_ratio_is_infinite() {
        // §4.3: width/height with zero height → ∞, not 0. (R2-7.)
        let env = MediaEnvironment {
            viewport_height: 0.0,
            ..landscape()
        };
        assert!(matches("(min-aspect-ratio: 1/1)", &env)); // inf >= 1
        assert!(matches("(aspect-ratio > 100/1)", &env)); // inf > 100
    }

    // --- Codex R3 regressions ---------------------------------------------

    #[test]
    fn calc_in_width_resolves_via_css_values() {
        // MQ4 §1.2/§1.3 delegates numeric `<mf-value>` to CSS Values, so `calc()`
        // works for width/height — parsed by the canonical `crate::values::
        // parse_length`, resolved against the environment at eval. (R3-1.)
        let env = landscape(); // 1024×768, root font 16px
        assert!(matches("(min-width: calc(40em + 1px))", &env)); // 40*16+1=641 ≤ 1024
        assert!(!matches("(min-width: calc(700px + 700px))", &env)); // 1400 > 1024
                                                                     // a viewport unit inside calc() resolves against the queried viewport.
        assert!(matches("(width: calc(50vw + 512px))", &env)); // 512+512 == 1024
                                                               // a number-typed calc() is not a `<length>` → not all.
        assert!(!matches("(width: calc(40))", &env));
        // calc() is not accepted for non-length features this slice → not all.
        assert!(!matches("(min-color: calc(2 + 6))", &env));
    }

    #[test]
    fn flex_unit_is_not_a_media_length() {
        // `fr` is a grid flex fraction, not a `<length>` — `crate::values` rejects
        // it, so width/height `fr` values never resolve as px; they are invalid →
        // `not all`. (R3-3: Codex flagged this as accepted; confirmed FP, locked.)
        let env = landscape();
        assert!(!matches("(width: 1fr)", &env));
        assert!(!matches("(min-width: 1fr)", &env));
        // committed-but-invalid value → not all, so `or true` cannot rescue it.
        assert!(!matches("(min-width: 1024fr) or (color)", &env));
    }

    #[test]
    fn color_requires_an_integer_token() {
        // §6.1 + §2.4.3: `color` is an `<integer>` — an integer *token*, so a
        // decimal `<number>` token (`8.0`) is invalid even with a zero fraction →
        // not all, while the integer `8` matches. (R3-4.)
        let env = landscape(); // color_bits = 8
        assert!(matches("(color: 8)", &env));
        assert!(!matches("(color: 8.0)", &env));
        assert!(!matches("(color >= 1.0)", &env));
        assert!(!matches("(color: 8.0) or (min-width: 1px)", &env)); // not all, not rescued
    }

    #[test]
    fn malformed_committed_range_is_not_all() {
        // A `:` or comparison commits to a feature; trailing tokens then make it a
        // malformed feature → §3.2 → `not all`, NOT `<general-enclosed>`, so a
        // sibling `or` cannot rescue it. (R3-5: sibling-sweep of R2-3 across the
        // name-first and value-first range arms.)
        let env = landscape();
        // name-first range with trailing junk.
        assert!(!matches("(width > 1px 2px)", &env));
        assert!(!matches("((width > 1px 2px) or (color))", &env)); // Codex's example
                                                                   // two-sided range with trailing junk.
        assert!(!matches("(400px < width < 700px 800px)", &env));
    }

    #[test]
    fn aspect_ratio_boolean_matches_range_value() {
        // §2.4.2: `(aspect-ratio)` boolean must agree with the range value
        // (width/height, §4.3). A zero-height viewport gives ratio +∞ (non-zero)
        // → true, mirroring `(aspect-ratio > 0/1)`; a zero-width viewport gives
        // ratio 0 → false. (R3-2.)
        let zero_h = MediaEnvironment {
            viewport_height: 0.0,
            ..landscape()
        };
        assert!(matches("(aspect-ratio)", &zero_h)); // ∞ is non-zero
        assert!(matches("(aspect-ratio) and (min-width: 1px)", &zero_h));
        let zero_w = MediaEnvironment {
            viewport_width: 0.0,
            ..landscape()
        };
        assert!(!matches("(aspect-ratio)", &zero_w)); // ratio 0
    }

    #[test]
    fn em_resolves_against_environment_root_font_size() {
        // §1.3: MQ relative lengths use the environment's initial font-size, not a
        // baked-in 16px. With a 20px root, `50em` = 1000px. (R3-6.)
        let big_font = MediaEnvironment {
            root_font_size_px: 20.0,
            ..landscape()
        };
        assert!(matches("(min-width: 50em)", &big_font)); // 1000 ≤ 1024
        assert!(!matches("(min-width: 52em)", &big_font)); // 1040 > 1024
                                                           // the default 16px root still holds (regression).
        assert!(matches("(min-width: 50em)", &landscape())); // 800 ≤ 1024
        assert!(matches("(min-width: 64em)", &landscape())); // 1024 == 1024
    }

    // --- Codex R4 regressions ---------------------------------------------

    #[test]
    fn deeply_nested_parens_is_not_all_not_a_panic() {
        // §3.2 total contract: pathologically nested parentheses (a DoS vector
        // for untrusted CSS / matchMedia) must fail to `not all`, never abort the
        // process via stack overflow. The depth cap bounds recursion regardless
        // of input length. (R4-3.)
        let env = landscape();
        let deep = format!(
            "{}(min-width: 1px){}",
            "(".repeat(20_000),
            ")".repeat(20_000)
        );
        assert!(!matches(&deep, &env)); // does not panic; over the cap → not all
                                        // sane nesting depths still parse + evaluate.
        assert!(matches("(((min-width: 1px)))", &env));
        assert!(matches("((min-width: 1px) and (max-width: 5000px))", &env));
    }

    // --- §2.5 combining ----------------------------------------------------

    #[test]
    fn and_or_not_combining() {
        let env = landscape();
        assert!(matches("screen and (min-width: 500px)", &env));
        assert!(!matches("screen and (min-width: 5000px)", &env));
        assert!(matches(
            "(min-width: 500px) and (orientation: landscape)",
            &env
        ));
        assert!(matches(
            "(min-width: 5000px) or (orientation: landscape)",
            &env
        ));
        assert!(matches("not (min-width: 5000px)", &env));
    }

    #[test]
    fn mixed_and_or_is_not_all() {
        // §3.2: mixing `and` + `or` at one level is a grammar error → not all.
        assert!(!matches(
            "(min-width: 1px) and (min-height: 1px) or (color)",
            &landscape()
        ));
    }

    // --- VM ≥ boa parity-superset (boa's 4 features: max/min width/height) --

    #[test]
    fn boa_parity_superset() {
        let env = landscape(); // 1024×768
                               // Each of boa's 4 features yields the same boolean a min/max string
                               // splitter would: actual vs threshold.
        assert!(matches("(min-width: 1024px)", &env));
        assert!(!matches("(min-width: 1025px)", &env));
        assert!(matches("(max-width: 1024px)", &env));
        assert!(!matches("(max-width: 1023px)", &env));
        assert!(matches("(min-height: 768px)", &env));
        assert!(!matches("(min-height: 769px)", &env));
        assert!(matches("(max-height: 768px)", &env));
        assert!(!matches("(max-height: 767px)", &env));
    }
}
