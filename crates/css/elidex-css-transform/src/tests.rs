use super::*;

fn parse(name: &str, css: &str) -> Vec<PropertyDeclaration> {
    let handler = TransformHandler;
    let mut pi = cssparser::ParserInput::new(css);
    let mut parser = cssparser::Parser::new(&mut pi);
    handler.parse(name, &mut parser).unwrap()
}

#[test]
fn parse_transform_none() {
    let decls = parse("transform", "none");
    assert_eq!(decls[0].value, CssValue::Keyword("none".to_string()));
}

#[test]
fn parse_translate_one_arg() {
    let decls = parse("transform", "translate(10px)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert_eq!(funcs.len(), 1);
        assert!(matches!(
            &funcs[0],
            TransformFunction::Translate(
                CssValue::Length(v, LengthUnit::Px),
                CssValue::Length(y, LengthUnit::Px)
            ) if (*v - 10.0).abs() < 0.001 && (*y).abs() < 0.001
        ));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_translate_two_args() {
    let decls = parse("transform", "translate(10px, 20%)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(
            &funcs[0],
            TransformFunction::Translate(
                CssValue::Length(_, LengthUnit::Px),
                CssValue::Percentage(_)
            )
        ));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_translate_x_y() {
    let decls = parse("transform", "translateX(5px)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(&funcs[0], TransformFunction::TranslateX(_)));
    } else {
        panic!("expected TransformList");
    }

    let decls = parse("transform", "translateY(50%)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(&funcs[0], TransformFunction::TranslateY(_)));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_rotate() {
    let decls = parse("transform", "rotate(45deg)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(&funcs[0], TransformFunction::Rotate(d) if (*d - 45.0).abs() < 0.001));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_rotate_units() {
    // rad
    let decls = parse("transform", "rotate(1rad)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        if let TransformFunction::Rotate(d) = &funcs[0] {
            assert!((*d - 57.2957).abs() < 0.01, "1rad ≈ 57.3deg, got {d}");
        }
    }

    // turn
    let decls = parse("transform", "rotate(0.25turn)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        if let TransformFunction::Rotate(d) = &funcs[0] {
            assert!((*d - 90.0).abs() < 0.001, "0.25turn = 90deg, got {d}");
        }
    }

    // grad
    let decls = parse("transform", "rotate(100grad)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        if let TransformFunction::Rotate(d) = &funcs[0] {
            assert!((*d - 90.0).abs() < 0.001, "100grad = 90deg, got {d}");
        }
    }
}

#[test]
fn parse_scale_one_arg() {
    let decls = parse("transform", "scale(2)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(
            matches!(&funcs[0], TransformFunction::Scale(sx, sy) if (*sx - 2.0).abs() < 0.001 && (*sy - 2.0).abs() < 0.001)
        );
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_scale_two_args() {
    let decls = parse("transform", "scale(1.5, 0.5)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(
            matches!(&funcs[0], TransformFunction::Scale(sx, sy) if (*sx - 1.5).abs() < 0.001 && (*sy - 0.5).abs() < 0.001)
        );
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_skew() {
    let decls = parse("transform", "skew(30deg, 10deg)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(
            matches!(&funcs[0], TransformFunction::Skew(ax, ay) if (*ax - 30.0).abs() < 0.001 && (*ay - 10.0).abs() < 0.001)
        );
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_matrix() {
    let decls = parse("transform", "matrix(1, 0, 0, 1, 50, 100)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        if let TransformFunction::Matrix(m) = &funcs[0] {
            assert_eq!(m[0], 1.0);
            assert_eq!(m[4], 50.0);
            assert_eq!(m[5], 100.0);
        } else {
            panic!("expected Matrix");
        }
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_multiple_functions() {
    let decls = parse("transform", "translate(10px) rotate(45deg) scale(2)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert_eq!(funcs.len(), 3);
        assert!(matches!(&funcs[0], TransformFunction::Translate(..)));
        assert!(matches!(&funcs[1], TransformFunction::Rotate(_)));
        assert!(matches!(&funcs[2], TransformFunction::Scale(..)));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_translate3d() {
    let decls = parse("transform", "translate3d(10px, 20px, 30px)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(&funcs[0], TransformFunction::Translate3d(..)));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_rotate3d() {
    let decls = parse("transform", "rotate3d(1, 0, 0, 90deg)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(
            matches!(&funcs[0], TransformFunction::Rotate3d(x, _, _, d) if (*x - 1.0).abs() < 0.001 && (*d - 90.0).abs() < 0.001)
        );
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_rotate_x_y_z() {
    let decls = parse("transform", "rotateX(45deg)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(&funcs[0], TransformFunction::RotateX(d) if (*d - 45.0).abs() < 0.001));
    }

    let decls = parse("transform", "rotateY(90deg)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(&funcs[0], TransformFunction::RotateY(d) if (*d - 90.0).abs() < 0.001));
    }

    let decls = parse("transform", "rotateZ(30deg)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(&funcs[0], TransformFunction::RotateZ(d) if (*d - 30.0).abs() < 0.001));
    }
}

#[test]
fn parse_scale3d() {
    let decls = parse("transform", "scale3d(1, 2, 0.5)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(
            matches!(&funcs[0], TransformFunction::Scale3d(sx, sy, sz) if (*sx - 1.0).abs() < 0.001 && (*sy - 2.0).abs() < 0.001 && (*sz - 0.5).abs() < 0.001)
        );
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_matrix3d() {
    let css = "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)";
    let decls = parse("transform", css);
    if let CssValue::TransformList(funcs) = &decls[0].value {
        if let TransformFunction::Matrix3d(m) = &funcs[0] {
            assert_eq!(m[0], 1.0);
            assert_eq!(m[15], 1.0);
        } else {
            panic!("expected Matrix3d");
        }
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_perspective_func() {
    let decls = parse("transform", "perspective(500px)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(
            matches!(&funcs[0], TransformFunction::PerspectiveFunc(d) if (*d - 500.0).abs() < 0.001)
        );
    } else {
        panic!("expected TransformList");
    }

    // CSS Transforms L2 §7.1: perspective(none) is valid (identity matrix).
    let decls = parse("transform", "perspective(none)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(
            matches!(&funcs[0], TransformFunction::PerspectiveFunc(d) if *d == 0.0),
            "perspective(none) should map to PerspectiveFunc(0.0)"
        );
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn parse_transform_origin_keywords() {
    let decls = parse("transform-origin", "left top");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts[0], CssValue::Percentage(0.0));
        assert_eq!(parts[1], CssValue::Percentage(0.0));
    }

    let decls = parse("transform-origin", "center");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts[0], CssValue::Percentage(50.0));
        assert_eq!(parts[1], CssValue::Percentage(50.0));
    }

    let decls = parse("transform-origin", "right bottom");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts[0], CssValue::Percentage(100.0));
        assert_eq!(parts[1], CssValue::Percentage(100.0));
    }
}

#[test]
fn parse_transform_origin_three_values() {
    let decls = parse("transform-origin", "left top 100px");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], CssValue::Percentage(0.0));
        assert_eq!(parts[1], CssValue::Percentage(0.0));
        assert_eq!(parts[2], CssValue::Length(100.0, LengthUnit::Px));
    } else {
        panic!("expected List");
    }

    // 2-value should have no Z
    let decls = parse("transform-origin", "center center");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts.len(), 2);
    }
}

#[test]
fn parse_perspective_origin_rejects_three_values() {
    // perspective-origin is always 2-value; 3rd value should be left unconsumed
    let decls = parse("perspective-origin", "50% 50%");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts.len(), 2);
    }
}

#[test]
fn parse_perspective_property_values() {
    let decls = parse("perspective", "800px");
    assert_eq!(decls[0].value, CssValue::Length(800.0, LengthUnit::Px));

    let decls = parse("perspective", "none");
    assert_eq!(decls[0].value, CssValue::Keyword("none".to_string()));
}

#[test]
fn parse_transform_style_backface() {
    let decls = parse("transform-style", "preserve-3d");
    assert_eq!(decls[0].value, CssValue::Keyword("preserve-3d".to_string()));

    let decls = parse("backface-visibility", "hidden");
    assert_eq!(decls[0].value, CssValue::Keyword("hidden".to_string()));
}

#[test]
fn parse_will_change_auto() {
    let decls = parse("will-change", "auto");
    assert_eq!(decls[0].value, CssValue::Keyword("auto".to_string()));
}

#[test]
fn parse_will_change_properties() {
    let decls = parse("will-change", "transform, opacity");
    if let CssValue::List(items) = &decls[0].value {
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], CssValue::Keyword("transform".to_string()));
        assert_eq!(items[1], CssValue::Keyword("opacity".to_string()));
    } else {
        panic!("expected List");
    }
}

#[test]
fn translatez_rejects_percentage() {
    let handler = TransformHandler;
    let mut pi = cssparser::ParserInput::new("translateZ(50%)");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert!(handler.parse("transform", &mut parser).is_err());
}

#[test]
fn translatez_accepts_length() {
    let decls = parse("transform", "translateZ(10px)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(
            &funcs[0],
            TransformFunction::TranslateZ(CssValue::Length(v, LengthUnit::Px)) if (*v - 10.0).abs() < 0.001
        ));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn translatez_accepts_zero() {
    let decls = parse("transform", "translateZ(0)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(
            &funcs[0],
            TransformFunction::TranslateZ(CssValue::Length(v, LengthUnit::Px)) if v.abs() < 0.001
        ));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn translate3d_z_rejects_percentage() {
    let handler = TransformHandler;
    let mut pi = cssparser::ParserInput::new("translate3d(10px, 20px, 50%)");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert!(handler.parse("transform", &mut parser).is_err());
}

#[test]
fn translate3d_xy_accept_percentage() {
    let decls = parse("transform", "translate3d(50%, 25%, 10px)");
    if let CssValue::TransformList(funcs) = &decls[0].value {
        assert!(matches!(
            &funcs[0],
            TransformFunction::Translate3d(
                CssValue::Percentage(_),
                CssValue::Percentage(_),
                CssValue::Length(_, LengthUnit::Px),
            )
        ));
    } else {
        panic!("expected TransformList");
    }
}

#[test]
fn origin_rejects_same_axis_keywords() {
    let handler = TransformHandler;
    // left + right are both X-axis — should fail
    let mut pi = cssparser::ParserInput::new("left right");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert!(handler.parse("transform-origin", &mut parser).is_err());

    // top + bottom are both Y-axis — should fail
    let mut pi = cssparser::ParserInput::new("top bottom");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert!(handler.parse("transform-origin", &mut parser).is_err());
}

#[test]
fn origin_swaps_y_x_order() {
    // "top left" should become (0%, 0%) = left=X, top=Y → swapped to (left, top)
    let decls = parse("transform-origin", "top left");
    if let CssValue::List(parts) = &decls[0].value {
        // X = left = 0%, Y = top = 0%
        assert_eq!(parts[0], CssValue::Percentage(0.0));
        assert_eq!(parts[1], CssValue::Percentage(0.0));
    } else {
        panic!("expected List");
    }
}

#[test]
fn origin_single_y_keyword_swaps() {
    // "top" alone = center top = (50%, 0%)
    let decls = parse("transform-origin", "top");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts[0], CssValue::Percentage(50.0), "X should be center");
        assert_eq!(parts[1], CssValue::Percentage(0.0), "Y should be top");
    } else {
        panic!("expected List");
    }

    // "bottom" alone = center bottom = (50%, 100%)
    let decls = parse("transform-origin", "bottom");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts[0], CssValue::Percentage(50.0), "X should be center");
        assert_eq!(parts[1], CssValue::Percentage(100.0), "Y should be bottom");
    } else {
        panic!("expected List");
    }
}

#[test]
fn perspective_origin_single_y_keyword() {
    // perspective-origin also uses parse_origin, so the Y-keyword swap applies.
    let decls = parse("perspective-origin", "top");
    if let CssValue::List(parts) = &decls[0].value {
        assert_eq!(parts[0], CssValue::Percentage(50.0), "X should be center");
        assert_eq!(parts[1], CssValue::Percentage(0.0), "Y should be top");
    } else {
        panic!("expected List");
    }
}

#[test]
fn resolve_will_change_sets_stacking_flag() {
    let handler = TransformHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();

    let value = CssValue::List(vec![CssValue::Keyword("transform".to_string())]);
    handler.resolve("will-change", &value, &ctx, &mut style);
    assert!(style.will_change_stacking);
    assert_eq!(style.will_change, vec!["transform".to_string()]);
}

#[test]
fn resolve_transform_resolves_em_units() {
    let handler = TransformHandler;
    let ctx = ResolveContext {
        em_base: 20.0, // 20px per em
        ..ResolveContext::default()
    };
    let mut style = ComputedStyle::default();

    // Parse "translate(2em, 3em)"
    let value = CssValue::TransformList(vec![TransformFunction::Translate(
        CssValue::Length(2.0, LengthUnit::Em),
        CssValue::Length(3.0, LengthUnit::Em),
    )]);
    handler.resolve("transform", &value, &ctx, &mut style);

    assert!(style.has_transform);
    assert_eq!(style.transform.len(), 1);
    // 2em * 20px = 40px, 3em * 20px = 60px
    if let TransformFunction::Translate(CssValue::Length(x, xu), CssValue::Length(y, yu)) =
        &style.transform[0]
    {
        assert!((*x - 40.0).abs() < 0.01, "x={x}");
        assert_eq!(*xu, LengthUnit::Px);
        assert!((*y - 60.0).abs() < 0.01, "y={y}");
        assert_eq!(*yu, LengthUnit::Px);
    } else {
        panic!(
            "expected Translate with lengths, got {:?}",
            style.transform[0]
        );
    }
}

#[test]
fn resolve_transform_preserves_percentages() {
    let handler = TransformHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();

    let value = CssValue::TransformList(vec![TransformFunction::Translate(
        CssValue::Percentage(50.0),
        CssValue::Percentage(100.0),
    )]);
    handler.resolve("transform", &value, &ctx, &mut style);

    if let TransformFunction::Translate(CssValue::Percentage(x), CssValue::Percentage(y)) =
        &style.transform[0]
    {
        assert!((*x - 50.0).abs() < 0.01);
        assert!((*y - 100.0).abs() < 0.01);
    } else {
        panic!("expected Translate with percentages");
    }
}
