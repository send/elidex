//! Observe the actual registry argument at the pending page declaration receiver.
use elidex_css::{parse_stylesheet_with_registry, Origin};
use elidex_plugin::{CssPropertyRegistry, EdgeSizes, PageGeometry, Size};
use std::cell::RefCell;
type Calls = Vec<(String, usize)>;
thread_local! { static CALLS:RefCell<Option<Calls>>=const {RefCell::new(None)}; }

pub(super) fn observe(name: &str, registry: Option<&CssPropertyRegistry>) {
    CALLS.with_borrow_mut(|calls| {
        if let Some(calls) = calls {
            calls.push((
                name.into(),
                registry.map_or(0, |r| std::ptr::from_ref(r).addr()),
            ));
        }
    });
}

#[test]
fn page_shorthand_and_independent_longhand_reparse_in_their_own_sheets() {
    let device = PageGeometry {
        sheet: Size::new(400., 600.),
        margins: EdgeSizes::uniform(10.),
    };
    for (value, expected) in [("2px", 2.), ("invalid", 0.)] {
        let parse = |css: &str| {
            parse_stylesheet_with_registry(
                css,
                Origin::Author,
                Some(crate::default_css_property_registry()),
            )
        };
        let a = parse(&format!("@page {{--a:{value};margin:var(--a)}}"));
        let b = parse("@page {--b:7px;margin-left:var(--b)}");
        let registry_id = |sheet: &elidex_css::Stylesheet| {
            std::ptr::from_ref(sheet.grammar_registry.as_deref().unwrap()).addr()
        };
        let expected_calls = [
            ("margin".to_owned(), registry_id(&a)),
            ("margin-left".to_owned(), registry_id(&b)),
        ];
        assert_ne!(expected_calls[0].1, expected_calls[1].1);
        CALLS.replace(Some(Vec::new()));
        let prepare = |sheets: &[&elidex_css::Stylesheet]| {
            crate::page::prepare_print_style(
                &mut elidex_ecs::EcsDom::new(),
                sheets,
                &[],
                &|_, _| vec![],
                device,
            )
        };
        let pages = prepare(&[&a, &b]);
        let calls = CALLS.take().unwrap();
        assert_eq!(calls.len(), 12);
        assert!(calls.chunks_exact(2).all(|pair| pair == expected_calls));
        let control = parse(&format!("@page {{ margin:{expected}px;margin-left:7px }}"));
        let margins = pages.select(1, false).geometry.margins;
        assert_eq!(
            margins,
            EdgeSizes {
                top: expected,
                right: expected,
                bottom: expected,
                left: 7.
            }
        );
        assert_eq!(
            margins,
            prepare(&[&control]).select(1, false).geometry.margins
        );
    }
}
