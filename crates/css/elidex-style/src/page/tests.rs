use super::*;
use elidex_css::{parse_stylesheet_with_registry, Origin};
use elidex_plugin::{LengthUnit, PageSelector};

fn sheet(css: &str, origin: Origin) -> Stylesheet {
    parse_stylesheet_with_registry(css, origin, Some(crate::default_css_property_registry()))
}

fn prepare(sheets: &[&Stylesheet]) -> PreparedPages {
    prepare_print_style(
        &mut elidex_ecs::EcsDom::new(),
        sheets,
        &[],
        &|_, _| Vec::new(),
        PageGeometry {
            sheet: Size::new(700., 900.),
            margins: EdgeSizes::uniform(10.),
        },
    )
}
#[test]
fn page_selectors_match_complete_alternatives_and_keep_source_order() {
    use PageSelector::{Blank, First, Left, Right};
    for (prelude, first, left, selectors) in [
        ("", 300., 300., vec![]),
        (":unknown", 200., 200., vec![]),
        (":first, :unknown", 200., 200., vec![]),
        ("invoice:first", 200., 200., vec![]),
        ("invoice:first, :left", 200., 300., vec![]),
        (":first:left", 200., 200., vec![]),
        (":first:left, :right", 300., 200., vec![Right]),
        (":right, :first:right", 300., 200., vec![Right, First]),
    ] {
        let s = sheet(
            &format!("@page {{size:200px}} @page {prelude} {{size:300px}}"),
            Origin::Author,
        );
        let p = prepare(&[&s]);
        assert_eq!(p.select(1, false).matched_selectors, selectors, "{prelude}");
        assert_eq!(p.select(1, false).geometry.sheet.width, first);
        assert_eq!(p.select(2, false).geometry.sheet.width, left);
    }
    let s = sheet(
        "@page :first {size:300px} @page :blank {size:400px} @page :right,:left {size:200px}",
        Origin::Author,
    );
    let p = prepare(&[&s]);
    assert_eq!(p.select(1, false).geometry.sheet.width, 300.);
    assert_eq!(p.select(2, true).geometry.sheet.width, 400.);
    assert_eq!(p.select(2, true).matched_selectors, vec![Blank, Left]);
}

#[test]
fn most_specific_matching_alternative_beats_a_later_rule() {
    for earlier in [":right, :first:right", ":first:first"] {
        let s = sheet(
            &format!("@page {earlier} {{size:300px}} @page :first {{size:200px}}"),
            Origin::Author,
        );
        assert_eq!(
            prepare(&[&s]).select(1, false).geometry.sheet.width,
            300.,
            "{earlier}"
        );
    }
}

#[test]
fn page_margins_resolve_css_wide_keywords_against_the_preliminary_root() {
    for (value, left) in [
        ("inherit", 32.),
        ("var(--missing, inherit)", 32.),
        ("initial", 0.),
        ("unset", 0.),
        ("var(--missing)", 0.),
    ] {
        let s = sheet(
            &format!(
                "html {{margin-left:32px}} @page {{size:200px; margin:0; margin-left:{value}}}"
            ),
            Origin::Author,
        );
        let mut dom = elidex_ecs::EcsDom::new();
        let document = dom.create_document_root();
        let html = dom.create_element_ownerless_fixture("html", elidex_ecs::Attributes::default());
        assert!(dom.append_child(document, html));
        let device = PageGeometry {
            sheet: Size::new(800., 600.),
            margins: EdgeSizes::uniform(10.),
        };
        let pages = prepare_print_style(&mut dom, &[&s], &[], &|_, _| vec![], device);
        let geometry = pages.select(1, false).geometry;
        assert_eq!(geometry.margins.left, left, "{value}");
        assert_eq!(geometry.area().size.width, 200. - left, "{value}");
    }
}

#[test]
fn finite_percentage_margins_leave_a_finite_area() {
    for value in ["50%", "calc(50%)", "calc(25% + 25%)"] {
        let s = sheet(
            &format!("@page {{size:1e37px 600px; margin:0; margin-left:{value}}}"),
            Origin::Author,
        );
        let pages = prepare(&[&s]);
        let geometry = pages.select(1, false).geometry;
        assert_eq!(geometry.margins.left, 5e36, "{value}");
        assert_eq!(geometry.area().size.width, 5e36, "{value}");
    }
}

#[test]
fn later_spreads_resolve_viewport_margins_against_the_selected_first_area() {
    let s=sheet("@page {margin:0} @page :first {size:200px} @page :left {margin-left:10vw} @page :right {margin-right:5vw} @page :first {margin-right:0} @page :blank {margin-top:3px}",Origin::Author);
    let pages = prepare(&[&s]);
    assert_eq!(pages.select(1, false).geometry.area().size.width, 200.);
    for (number, left, right) in [
        (1, 0., 0.),
        (2, 20., 0.),
        (3, 0., 10.),
        (4, 20., 0.),
        (5, 0., 10.),
    ] {
        for blank in [false, true] {
            let margins = pages.select(number, blank).geometry.margins;
            assert_eq!((margins.left, margins.right), (left, right));
            assert_eq!(margins.top, if blank { 3. } else { 0. });
        }
    }
}

#[test]
fn size_and_margins_use_selected_sheet_and_device() {
    for (descriptor, w, h) in [
        ("auto", 700., 900.),
        ("landscape", 900., 700.),
        ("calc(1in + 12pt)", 112., 112.),
        ("calc(-1px)", 0., 0.),
        ("var(--absent)", 700., 900.),
        ("80px 120px", 80., 120.),
        ("90px potato", 200., 200.),
    ] {
        let s = sheet(
            &format!("@page {{size:200px;size:{descriptor}}}"),
            Origin::Author,
        );
        assert_eq!(
            prepare(&[&s]).select(1, false).geometry.sheet,
            Size::new(w, h),
            "{descriptor}"
        );
    }
    let s=sheet("@page {size:400px 600px; margin:10% 12pt calc(5% + 2px) 10%;} @page :left {margin-left:10mm}",Origin::Author);
    let p = prepare(&[&s]);
    assert_eq!(
        p.select(1, false).geometry.margins,
        EdgeSizes {
            top: 60.,
            right: 16.,
            bottom: 32.,
            left: 40.
        }
    );
    assert!((p.select(2, false).geometry.margins.left - 96. / 2.54).abs() < 0.001);
}

#[test]
fn page_font_is_inherited_by_one_box_per_position() {
    let s=sheet("@page {font-size:20px; @top-left {content:'1'; padding-left:1em}} @page :first {font-size:40px; @top-left {color:red}}",Origin::Author);
    let p = prepare(&[&s]);
    for (number, em) in [(1, 40.), (2, 20.)] {
        let boxes = &p.select(number, false).boxes;
        assert_eq!(boxes.len(), 1);
        let props = &boxes[0].1.properties;
        for key in ["font-size", "padding-left"] {
            assert_eq!(
                props.iter().find(|d| d.property == key).unwrap().value,
                CssValue::Length(em, LengthUnit::Px)
            );
        }
    }
    let declared = &s.page_rules[0].boxes[0].1;
    assert_eq!(
        declared
            .iter()
            .find(|d| d.property == "padding-left")
            .unwrap()
            .value,
        CssValue::Length(1., LengthUnit::Em)
    );
}
#[test]
fn page_origin_and_importance_survive_substitution() {
    let ua = sheet(
        "@page {--s:101px!important; margin-left:3px!important}",
        Origin::UserAgent,
    );
    let author=sheet("@page {--s:20px!important;size:var(--s);margin:40px!important} @page :first {size:31px;size:var(--bad)}",Origin::Author);
    let p = prepare(&[&ua, &author]);
    assert_eq!(p.select(1, false).geometry.sheet.width, 700.);
    assert_eq!(p.select(2, false).geometry.sheet.width, 101.);
    assert_eq!(p.select(1, false).geometry.margins.left, 3.);
    assert_eq!(p.select(1, false).geometry.margins.top, 40.);
}

#[test]
fn margin_box_content_preserves_strings_and_counter_styles() {
    use elidex_plugin::{ContentItem, ContentValue, ListStyleType};
    let s = sheet(
        r#"@page { @top-center { content:"Page }@" counter(page) counter(section,upper-roman) counters(chapter,"."); } }"#,
        Origin::Author,
    );
    let prepared = prepare(&[&s]);
    assert_eq!(
        prepared.select(1, false).boxes[0].1.content,
        ContentValue::Items(vec![
            ContentItem::String("Page }@".into()),
            ContentItem::Counter {
                name: "page".into(),
                style: ListStyleType::Decimal
            },
            ContentItem::Counter {
                name: "section".into(),
                style: ListStyleType::UpperRoman
            },
            ContentItem::Counters {
                name: "chapter".into(),
                separator: ".".into(),
                style: ListStyleType::Decimal
            }
        ])
    );
}

#[test]
fn page_local_invalid_variable_does_not_revive_a_losing_declaration() {
    let ua = PageGeometry {
        sheet: Size::new(700., 900.),
        margins: EdgeSizes::uniform(10.),
    };
    for (value, first) in [("140px", 140.), ("invalid", 700.)] {
        let s=sheet(&format!("html{{--dim:240px}} @page{{size:100px;size:var(--dim)}} @page :first{{--dim:{value}}}"),Origin::Author);
        let mut dom = elidex_ecs::EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element_ownerless_fixture("html", elidex_ecs::Attributes::default());
        assert!(dom.append_child(doc, html));
        let p = prepare_print_style(&mut dom, &[&s], &[], &|_, _| vec![], ua);
        assert_eq!(p.select(1, false).geometry.sheet.width, first);
        assert_eq!(p.select(2, false).geometry.sheet.width, 240.);
    }
}

#[test]
fn box_font_keeps_zero_in_both_absolute_units() {
    for (size, px) in [("0in", 0.), ("0px", 0.), ("12px", 12.)] {
        let s = sheet(
            &format!("@page {{ @top-left {{content:'1'; font-size:{size}}} }}"),
            Origin::Author,
        );
        let pages = prepare(&[&s]);
        let value = &pages.select(1, false).boxes[0]
            .1
            .properties
            .iter()
            .find(|p| p.property == "font-size")
            .unwrap()
            .value;
        assert_eq!(*value, CssValue::Length(px, LengthUnit::Px));
    }
}

#[test]
fn compound_percentage_preserves_finite_quotient() {
    let s = sheet(
        "@page {size:200px; margin:0; margin-left:calc(50% * 1e37 / 1e37)}",
        Origin::Author,
    );
    let pages = prepare(&[&s]);
    let page = pages.select(1, false).geometry;
    assert_eq!(page.margins.left, 100.);
    assert_eq!(page.area().size.width, 100.);
}

#[test]
fn opposing_margins_preserve_both_sheet_extents() {
    let s = sheet(
        "@page {size:3e38px; margin:-50% 50% 50% -50%}",
        Origin::Author,
    );
    assert_eq!(
        prepare(&[&s]).select(1, false).geometry.area().size,
        Size::new(3e38, 3e38)
    );
}

#[test]
fn large_viewport_units_use_the_finite_selected_area() {
    for unit in ["vw", "vh", "vmin", "vmax", "lvw", "lvh", "lvmin", "lvmax"] {
        let s = sheet(
            &format!("@page {{size:1e37px;margin:0}} @page :left {{margin-left:50{unit}}}"),
            Origin::Author,
        );
        let pages = prepare(&[&s]);
        let page = pages.select(2, false).geometry;
        assert_eq!(page.margins.left, 5e36, "{unit}");
        assert_eq!(page.area().size.width, 5e36);
    }
}

#[test]
fn margin_box_percentage_font_retains_its_finite_em_base() {
    let s = sheet(
        "@page {font-size:1e37px; @top-left {content:'x';font-size:50%;padding-left:1em}}",
        Origin::Author,
    );
    let pages = prepare(&[&s]);
    let props = &pages.select(1, false).boxes[0].1.properties;
    for name in ["font-size", "padding-left"] {
        assert_eq!(
            props.iter().find(|d| d.property == name).unwrap().value,
            CssValue::Length(5e36, LengthUnit::Px)
        );
    }
}

#[test]
fn inherited_margin_box_padding_uses_computed_page_length() {
    let s=sheet("@page {font-size:12px;padding-left:2em; @top-left {font-size:8px;content:'x';padding-left:inherit}}",Origin::Author);
    let pages = prepare(&[&s]);
    let props = &pages.select(1, false).boxes[0].1.properties;
    assert_eq!(
        props
            .iter()
            .find(|d| d.property == "padding-left")
            .unwrap()
            .value,
        CssValue::Length(24., LengthUnit::Px)
    );
}
