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
    fn invalid_two_sided_range_is_general_enclosed_not_definite() {
        // §3 `<mf-range>`: two-sided form requires both ops same-direction, no
        // `=`. Mixed / `=` two-sided → `<general-enclosed>` → Kleene unknown,
        // NOT a definite Range. (C1 regression.)
        let env = landscape(); // width 1024
                               // mixed `<`…`>` — was wrongly true (built `>400 AND >700`).
        assert!(!matches("(400px < width > 700px)", &env));
        // `=` in two-sided — was wrongly negatable.
        assert!(!matches("not (400px = width = 700px)", &env));
        // unknown→false at the boundary, and `or true` still true (unknown OR true).
        assert!(matches("(400px < width > 700px) or (min-width: 1px)", &env));
        // single-sided `=` value-first stays valid (§ allows `<mf-eq>` there).
        assert!(matches("(1024px = width)", &env));
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
