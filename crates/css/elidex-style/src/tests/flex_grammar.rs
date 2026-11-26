use super::*;

#[test]
fn flex_factor_math_and_basis_substitution_use_the_literal_grammar() {
    for value in ["calc(2 - 3 + 2) calc(-1) 1in", "1 0 1in"] {
        let direct = resolve_single(&format!("div {{ flex:{value} }}")).2;
        let substituted = resolve_single(&format!("div {{ --x:{value}; flex:var(--x) }}")).2;
        for style in [direct, substituted] {
            assert_eq!(style.flex_grow, 1.);
            assert_eq!(style.flex_shrink, 0.);
            assert_eq!(style.flex_basis, elidex_plugin::FlexBasis::Length(96.));
        }
    }
}

#[test]
fn deferred_lengths_fail_on_reparse_without_reviving_earlier_width() {
    for unit in "ex rex cap rcap ch rch ic ric lh rlh vi vb lvi lvb svw svh svi svb svmin svmax dvw dvh dvi dvb dvmin dvmax".split_whitespace() {
        for value in [format!("1{unit}"), format!("calc(1{unit})")] {
            let style = resolve_single(&format!("div {{ width:12px; width:{value} }}")).2;
            assert_eq!(style.width, Dimension::Length(12.), "{value}");
            let style = resolve_single(&format!("div {{ --x:{value}; width:12px; width:var(--x) }}")).2;
            assert_eq!(style.width, Dimension::Auto, "{value}");
        }
    }
}

#[test]
fn strict_physical_unit_readers_agree_with_px_in_computed_style() {
    for unit in ["1in", "96px"] {
        let style = resolve_single(&format!("div {{font-size:{unit};line-height:{unit};letter-spacing:{unit};vertical-align:{unit};grid-template-columns:{unit}; flex-basis:{unit};padding-left:{unit}}}")).2;
        let control = resolve_single("div {font-size:96px;line-height:96px;letter-spacing:96px;vertical-align:96px;grid-template-columns:96px;flex-basis:96px;padding-left:96px}").2;
        assert_eq!(style.font_size, 96.);
        assert_eq!(style.line_height, control.line_height);
        assert_eq!(style.letter_spacing, control.letter_spacing);
        assert_eq!(style.vertical_align, control.vertical_align);
        assert_eq!(style.grid_template_columns, control.grid_template_columns);
        assert_eq!(style.flex_basis, control.flex_basis);
        assert_eq!(style.padding.left, control.padding.left);
    }
}

#[test]
fn inline_and_rule_substitution_validate_winning_flex_declarations() {
    for (block, grow, shrink, basis) in [
        (
            "--x:2 0 10px;flex:var(--x)",
            2.,
            0.,
            elidex_plugin::FlexBasis::Length(10.),
        ),
        (
            "--x:2 -3 10px;flex:var(--x)",
            0.,
            1.,
            elidex_plugin::FlexBasis::Auto,
        ),
        (
            "--b:-1px;flex-basis:var(--b)",
            0.,
            1.,
            elidex_plugin::FlexBasis::Auto,
        ),
        (
            "--g:2;flex-grow:var(--g)",
            2.,
            1.,
            elidex_plugin::FlexBasis::Auto,
        ),
    ] {
        let rule_style = resolve_single(&format!("div {{{block}}}")).2;
        let (mut dom, div, _) = resolve_single("");
        dom.set_attribute(div, "style", block);
        resolve_styles(&mut dom, &[], Size::new(200., 100.));
        for style in [rule_style, get_style(&dom, div)] {
            assert_eq!(
                (style.flex_grow, style.flex_shrink, style.flex_basis),
                (grow, shrink, basis)
            );
        }
    }
}
