use super::*;

fn computed_custom(declarations: &str, property: &str) -> String {
    let (_, _, style) = resolve_single(&format!("div {{ {declarations} }}"));
    serialize_resolved_value(property, &style)
}

#[test]
fn substitutions_preserve_token_boundaries_and_source() {
    for (declarations, expected) in [
        ("--x:a/*b*/; --m:var(--x)*", "a/*b*/*"),
        ("--x:foo; --m:var(--x)+2", "foo/**/+2"),
        ("--x:2; --m:var(--x)px", "2/**/px"),
        ("--d:--; --m:var(--d)>", "--/**/>"),
        ("--lt:<; --m:f(var(--lt)!--)", "f(</**/!--)"),
        ("--m:f(<!--)", "f(<!--)"),
        (r"--x:\61; --m:var(--x) b", r"\61/**/ b"),
        (r"--x:\000061; --m:var(--x) b", r"\000061/**/ b"),
        (r"--x:\61 ; --m:var(--x) b", r"\61  b"),
        (
            "--n:001.0px; --m:/*a*/ var(--n)  /*b*/",
            "/*a*/ 001.0px  /*b*/",
        ),
        ("--m: ", " "),
    ] {
        assert_eq!(
            computed_custom(declarations, "--m"),
            expected,
            "{declarations}"
        );
        let copied = elidex_css::parse_value_for_property("--copy", expected, None).unwrap();
        assert_eq!(copied[0].value.to_css_string(), expected);
    }
}

#[test]
fn dynamic_names_spread_and_selected_fallbacks() {
    for (source, expected) in [
        (
            "--a:var(--missing) var(--b); --b:var(--a,2px); --m:var(--b,7px)",
            "7px",
        ),
        ("--ok:2px; --m:var(--ok,f(...var(--missing)))", ""),
        ("--ok:2px; --m:var(--ok,[...var(--missing)])", ""),
        ("--ok:2px; --m:var(--ok,f(var(--missing)))", "2px"),
        ("--ok:2px; --m:var(--absent,f([...var(--ok)]))", "f([2px])"),
        ("--args:,7px; --m:var(...var(--args))", ""),
        ("--name:--x; --x:10px; --m:var(var(--name))", "10px"),
        ("--args:--missing,7px; --m:var(...var(--args))", "7px"),
        ("--args:--missing,7px; --m:var(var(--args))", ""),
        ("--m:var(123,5px)", "5px"),
        ("--ok:2px; --x:var(--ok,var(--x)); --m:var(--x,7px)", "2px"),
        ("--x:var(--ok,var(--x)); --m:var(--x,7px)", "7px"),
        ("--a:var(--b); --b:var(--a,5px); --m:var(--a,7px)", "7px"),
    ] {
        assert_eq!(computed_custom(source, "--m"), expected, "{source}");
    }
}

#[test]
fn bounded_chains_publish_independent_suffixes() {
    for count in [32, 33, 40] {
        let mut chain: Vec<_> = (1..count)
            .map(|i| format!("--n{i}:var(--n{});", i + 1))
            .collect();
        chain.push(format!("--n{count}:2px;"));
        let mut previous = None;
        for _ in 0..2 {
            let (_, _, _, style, child) =
                resolve_with_child(&format!("div {{ {} }}", chain.join("")));
            assert_eq!(style.custom_properties, child.custom_properties);
            if let Some(previous) = previous {
                assert_eq!(style.custom_properties, previous);
            }
            previous = Some(style.custom_properties.clone());
            assert_eq!(
                serialize_resolved_value("--n1", &style),
                if count <= 32 { "2px" } else { "" }
            );
            assert_eq!(
                serialize_resolved_value(&format!("--n{}", count - 9), &style),
                "2px"
            );
            chain.reverse();
        }
    }
}

#[test]
fn replay_respects_order_and_importance() {
    for reverse in [false, true] {
        for short_important in [false, true] {
            for long_important in [false, true] {
                let shorthand = format!(
                    "margin:var(--n) 1px{};",
                    if short_important { " !important" } else { "" }
                );
                let longhand = format!(
                    "margin-left:7px{};",
                    if long_important { " !important" } else { "" }
                );
                let declarations = if reverse {
                    longhand + &shorthand
                } else {
                    shorthand + &longhand
                };
                let (_, _, style) = resolve_single(&format!("div {{--n:2px; {declarations}}}"));
                assert_eq!(
                    serialize_resolved_value("margin-left", &style),
                    if (short_important, reverse) > (long_important, false) {
                        "1px"
                    } else {
                        "7px"
                    }
                );
            }
        }
    }
}

#[test]
fn inline_eof_repairs_only_the_computed_join() {
    for (authored, expected) in [
        ("\"a", "\"a\", serif"),
        ("a/*x*", "a/*x**/, serif"),
        ("a/*x**/", "a/*x**/, serif"),
        ("f([a", "f([a]), serif"),
    ] {
        let (mut dom, _, _, parent) = build_simple_dom();
        dom.world_mut()
            .get::<&mut Attributes>(parent)
            .unwrap()
            .set("style", format!("--s:{authored}"));
        let mut attrs = Attributes::default();
        attrs.set("style", "--m:var(--s), serif");
        let child = dom.create_element_ownerless_fixture("div", attrs);
        dom.append_child(parent, child);
        resolve_styles(&mut dom, &[], Size::new(800., 600.));
        assert_eq!(
            serialize_resolved_value("--m", &get_style(&dom, child)),
            expected
        );
        let declared = elidex_css::parse_inline_style(&format!("--s:{authored}"), None);
        assert_eq!(declared.get("--s"), Some(authored));
        let copy = elidex_css::parse_value_for_property("--copy", expected, None).unwrap();
        assert_eq!(copy[0].value.to_css_string(), expected);
    }
}

#[test]
fn computed_custom_values_are_terminal_on_inheritance() {
    for (local, expected) in [
        ("", "2px"),
        ("--m:var(--n);", "7px"),
        ("--m:inherit;", "2px"),
        ("--m:unset;", "2px"),
        ("--m:initial;", "9px"),
    ] {
        let (_, _, _, _, child) = resolve_with_child(&format!(
            "div {{--m:var(--n);--n:2px}} span {{--n:7px;{local} width:var(--m,9px)}}"
        ));
        assert_eq!(serialize_resolved_value("width", &child), expected);
    }
}

#[test]
fn literal_and_substituted_background_reset_the_prior_image() {
    let registry = default_css_property_registry();
    for declaration in [
        "background:red",
        "--red:red; background:var(--red)",
        "--bad:20px; background:var(--bad)",
    ] {
        let sheet = elidex_css::parse_stylesheet_with_registry(
            &format!("body {{background-image:url(picture.png); {declaration} }}"),
            Origin::Author,
            Some(registry),
        );
        let (mut dom, _, _, body) = build_simple_dom();
        resolve_styles(&mut dom, &[&sheet], Size::new(800., 600.));
        assert!(get_style(&dom, body).background_layers.is_none());
        let sheets = [&sheet];
        let entries = super::super::cascade::collect_and_cascade(
            body,
            &dom,
            &sheets,
            &[],
            &[],
            &super::super::cascade::ShadowCascade::default(),
            &MediaEnvironment::default(),
        );
        if declaration.contains("--bad") {
            let result =
                super::super::resolve::resolve_candidates(&entries, &ComputedStyle::default());
            for member in elidex_css::shorthand_longhands("background") {
                assert_eq!(result.ordinary()[member.as_str()], &CssValue::Unset);
            }
        }
    }
}

#[test]
fn producing_rules_keep_their_admitting_registry() {
    use elidex_css::parse_stylesheet_with_registry;
    use elidex_css_transform::TransformHandler;
    use elidex_plugin::CssPropertyRegistry;

    let mut registered = CssPropertyRegistry::new();
    registered.register_dynamic("transform".into(), Box::new(TransformHandler));
    let empty = CssPropertyRegistry::new();
    let first = parse_stylesheet_with_registry(
        "body {transform:rotate(5deg)}",
        Origin::Author,
        Some(&registered),
    );
    let frozen = first.grammar_registry.as_ref().unwrap();
    assert!(std::ptr::eq(
        frozen.resolve("transform").unwrap(),
        registered.resolve("transform").unwrap()
    ));
    registered.register_static("transform", Box::new(TransformHandler));
    registered.register_static("later", Box::new(TransformHandler));
    assert!(frozen.resolve("later").is_none());
    for source in [
        "--x:rotate(20deg);transform:var(--x)",
        "--x:20deg;transform:rotate(var(--x))",
    ] {
        for supported in [false, true] {
            let second = parse_stylesheet_with_registry(
                &format!("body {{{source}}}"),
                Origin::Author,
                Some(if supported { &registered } else { &empty }),
            );
            let count = second.rules[0]
                .declarations
                .iter()
                .filter(|d| d.property == "transform")
                .count();
            assert_eq!(count, usize::from(supported));
            let (mut dom, _, _, body) = build_simple_dom();
            resolve_styles(&mut dom, &[&first, &second], Size::new(800., 600.));
            let expected = parse_stylesheet_with_registry(
                if supported {
                    "body {transform:rotate(20deg)}"
                } else {
                    "body {transform:rotate(5deg)}"
                },
                Origin::Author,
                Some(&registered),
            );
            let result = get_style(&dom, body).transform;
            resolve_styles(&mut dom, &[&expected], Size::new(800., 600.));
            assert_eq!(result, get_style(&dom, body).transform);
        }
    }
}

#[test]
fn animation_style_reads_the_resolved_winners() {
    let registry = default_css_property_registry();
    for (text, direct) in [
        ("var(--a)", "fade"),
        ("spin,var(--a)", "spin,fade"),
        ("var(--absent)", "unset"),
    ] {
        let names = |value: &str| {
            let sheet = elidex_css::parse_stylesheet_with_registry(
                &format!("body {{ --a:fade; animation-name:{value} }}"),
                Origin::Author,
                Some(registry),
            );
            let (mut dom, _, _, body) = build_simple_dom();
            resolve_styles(&mut dom, &[&sheet], Size::new(800., 600.));
            dom.world()
                .get::<&elidex_css_anim::style::AnimStyle>(body)
                .ok()
                .map(|s| s.animation_name.clone())
        };
        assert_eq!(names(text), names(direct), "{text}");
    }
}

#[test]
fn fallback_is_reparsed_by_the_receiving_grammar() {
    for (source, prop, expected) in [
        (
            "flex-shrink:3; flex-shrink:var(--missing, 0px)",
            "flex-shrink",
            "1",
        ),
        ("flex-shrink:var(--missing, 0)", "flex-shrink", "0"),
        ("margin:3px var(--missing,)", "margin-left", "3px"),
        ("margin:3px var(--missing)", "margin-left", "0px"),
        ("margin:7px; margin:\"var(\" 2px", "margin-left", "7px"),
        (
            "--n:red; margin:7px; margin:VAR(--n) 2px",
            "margin-left",
            "0px",
        ),
        ("--n:3px; margin:VAR(--n) 2px", "margin-top", "3px"),
        ("width:7px; width:var()", "width", "7px"),
        ("width:7px; width:1ch", "width", "7px"),
        ("--x:1ch; width:7px; width:var(--x)", "width", "auto"),
        (
            "--x:2; --m:var(--x)px; width:7px; width:var(--m)",
            "width",
            "auto",
        ),
        ("--x:7px; --x:var(); width:var(--x)", "width", "7px"),
        (
            "width:7px; width:calc(var(--valid) + var())",
            "width",
            "7px",
        ),
    ] {
        let (_, _, style) = resolve_single(&format!("div {{{source}}}"));
        assert_eq!(serialize_resolved_value(prop, &style), expected, "{source}");
    }
}

#[test]
fn pseudo_display_and_existence_use_resolved_candidates() {
    for (declarations, expected) in [
        ("", None),
        ("content:'x'", Some(Display::Inline)),
        ("content:'x'; display:block", Some(Display::Block)),
        (
            "content:'x'; --d:block; display:var(--d)",
            Some(Display::Block),
        ),
        (
            "content:'x'; display:block; display:var(--missing)",
            Some(Display::Inline),
        ),
    ] {
        let (dom, div, _) = resolve_single(&format!("div::before {{{declarations}}}"));
        let actual = dom
            .children_iter(div)
            .find(|child| dom.world().get::<&PseudoElementMarker>(*child).is_ok())
            .map(|child| get_style(&dom, child).display);
        assert_eq!(actual, expected, "{declarations}");
    }
}

#[test]
fn updating_a_parent_recomputes_descendants() {
    let (mut dom, _, _, body) = build_simple_dom();
    let child = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.append_child(body, child);
    let sheet = parse_stylesheet(
        "body {--m:var(--n)} div {--n:7px;width:var(--m)}",
        Origin::Author,
    );
    for value in ["2px", "4px"] {
        dom.world_mut()
            .get::<&mut Attributes>(body)
            .unwrap()
            .set("style", format!("--n:{value}"));
        resolve_styles(&mut dom, &[&sheet], Size::new(800., 600.));
        assert_eq!(
            serialize_resolved_value("width", &get_style(&dom, child)),
            value
        );
    }
}

#[test]
fn pending_flex_members_retain_the_original_rank() {
    for (text, expected) in [
        ("flex:var(--f); flex-grow:3", "3"),
        ("flex-grow:3; flex:var(--f)", "2"),
        ("flex:initial; flex:var(--f)", "2"),
        ("flex:var(--f); flex:initial", "0"),
    ] {
        let (_, _, style) = resolve_single(&format!("div {{--f:2 1 0px; {text}}}"));
        assert_eq!(serialize_resolved_value("flex-grow", &style), expected);
    }
    for (text, expected) in [
        ("flex-flow:initial;flex-flow:var(--f)", "column"),
        ("flex-flow:var(--f);flex-flow:initial", "row"),
    ] {
        let (_, _, style) = resolve_single(&format!("div {{--f:column wrap; {text}}}"));
        assert_eq!(serialize_resolved_value("flex-direction", &style), expected);
    }
}

#[test]
fn receiving_grammars_match_direct_declarations() {
    for (property, value) in [
        ("font-family", "'Courier New', monospace"),
        ("font-family", "monospace"),
        ("border-top", "2px solid red"),
        ("border-spacing", "2px 3px"),
        ("background", "#0d1117"),
        ("margin", "2px 3px"),
        ("border-radius", "2px"),
        ("grid-template", "10px / 1fr"),
        ("grid-column", "1 / 3"),
        ("flex", "2 1 0px"),
        ("color", "red"),
        ("display", "block"),
    ] {
        let direct = resolve_single(&format!("div {{{property}:{value}}}")).2;
        let mut pending =
            resolve_single(&format!("div {{--arg:{value}; {property}:var(--arg)}}")).2;
        pending.custom_properties.clear();
        assert_eq!(direct, pending, "{property}: {value}");
    }
}

/// External grammar fixture with a visible computed member and two valid
/// source forms, so both whole and embedded pending values can be tested.
struct TestMargin;
impl elidex_plugin::CssPropertyHandler for TestMargin {
    fn property_names(&self) -> &[&str] {
        &["test-margin"]
    }
    fn declaration_members<'a>(&'a self, _: &'a str) -> Vec<&'a str> {
        vec!["margin-left"]
    }
    fn parse(
        &self,
        _: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<elidex_plugin::PropertyDeclaration>, elidex_plugin::ParseError> {
        let _ = input.try_parse(|p| p.expect_ident_matching("left"));
        let n = input
            .expect_number()
            .map_err(|_| elidex_plugin::ParseError::simple("expected a number"))?;
        Ok(vec![elidex_plugin::PropertyDeclaration::new(
            "margin-left",
            CssValue::Length(n, elidex_plugin::LengthUnit::Px),
        )])
    }
    fn resolve(
        &self,
        _: &str,
        _: &CssValue,
        _: &elidex_plugin::ResolveContext,
        _: &mut ComputedStyle,
    ) {
    }
    fn initial_value(&self, _: &str) -> CssValue {
        CssValue::Initial
    }
    fn is_inherited(&self, _: &str) -> bool {
        false
    }
    fn get_computed(&self, _: &str, _: &ComputedStyle) -> CssValue {
        CssValue::Initial
    }
}

#[test]
fn external_shorthand_is_admitted_only_by_its_producer() {
    let empty = CssPropertyRegistry::new();
    let mut registered = CssPropertyRegistry::new();
    registered.register_dynamic("test-margin".into(), Box::new(TestMargin));
    let first = elidex_css::parse_stylesheet_with_registry(
        "body {test-margin:7}",
        Origin::Author,
        Some(&registered),
    );
    for (value, number, expected) in [
        ("var(--x)", "2", "2px"),
        ("left var(--x)", "2", "2px"),
        ("var(--x)", "red", "0px"),
    ] {
        for supported in [false, true] {
            let sheet = elidex_css::parse_stylesheet_with_registry(
                &format!("body {{--x:{number};test-margin:{value}}}"),
                Origin::Author,
                Some(if supported { &registered } else { &empty }),
            );
            assert_eq!(
                sheet.rules[0].declarations.len(),
                1 + usize::from(supported)
            );
            let (mut dom, _, _, body) = build_simple_dom();
            resolve_styles(&mut dom, &[&first, &sheet], Size::new(800., 600.));
            assert_eq!(
                serialize_resolved_value("margin-left", &get_style(&dom, body)),
                if supported { expected } else { "7px" }
            );
        }
    }
}

#[test]
fn inline_registry_and_builtin_shadow_grammar_are_distinct() {
    let (mut dom, _, _, body) = build_simple_dom();
    let child = dom.create_element_ownerless_fixture("div", Attributes::default());
    dom.append_child(body, child);
    let _ = setup_shadow_with_style(
        &mut dom,
        child,
        ":host {--shade: red; color:var(--shade);transform:var(--motion)}",
    );
    let ua = parse_stylesheet(
        "div {--motion:rotate(15deg);transform:var(--motion)}",
        Origin::UserAgent,
    );
    resolve_styles(&mut dom, &[&ua], Size::new(800., 600.));
    let style = get_style(&dom, child);
    assert!(style.transform.is_empty());
    assert_eq!(style.color, CssColor::RED);
    let mut transforms = Vec::new();
    for input in ["transform:rotate(15deg)", "transform:var(--motion)"] {
        dom.world_mut()
            .get::<&mut Attributes>(child)
            .unwrap()
            .set("style", input);
        resolve_styles(&mut dom, &[&ua], Size::new(800., 600.));
        transforms.push(get_style(&dom, child).transform);
    }
    assert!(!transforms[0].is_empty());
    assert_eq!(transforms[0], transforms[1]);
}

#[test]
fn inline_and_stylesheet_members_compare_at_their_original_priority() {
    let (mut dom, _, _, body) = build_simple_dom();
    let sheet = parse_stylesheet("body {--n:2px; margin:var(--n) !important}", Origin::Author);
    for (inline, expected) in [
        ("margin-left:7px", "2px"),
        ("margin-left:7px !important", "7px"),
    ] {
        dom.world_mut()
            .get::<&mut Attributes>(body)
            .unwrap()
            .set("style", inline);
        resolve_styles(&mut dom, &[&sheet], Size::new(800., 600.));
        assert_eq!(
            serialize_resolved_value("margin-left", &get_style(&dom, body)),
            expected
        );
    }
}
