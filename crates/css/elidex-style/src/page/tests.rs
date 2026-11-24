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
    let s=sheet("@page :first {size:300px} @page :blank {size:400px} @page :right,:left {size:200px} @page :first:first {size:500px}",Origin::Author);
    let p = prepare(&[&s]);
    assert_eq!(p.select(1, false).geometry.sheet.width, 500.);
    assert_eq!(p.select(2, true).geometry.sheet.width, 400.);
    assert_eq!(p.select(2, true).matched_selectors, vec![Blank, Left]);
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
