//! Element-namespace tracking: `Namespace` component, `create_element_ns`,
//! `namespace_of`, the reworked `is_html_namespace`, and the
//! HTML-namespace-restricted `is_base_element` predicate.

use super::*;

#[test]
fn plain_element_is_html_namespace() {
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    assert_eq!(dom.namespace_of(div), Namespace::Html);
    assert!(dom.is_html_namespace(div));
}

#[test]
fn html_elements_carry_no_namespace_component() {
    // The component is sparse: HTML elements (the default) must NOT carry
    // a `Namespace` component, so `create_element` is unchanged.
    let mut dom = EcsDom::new();
    let div = elem(&mut dom, "div");
    assert!(
        dom.world().get::<&Namespace>(div).is_err(),
        "HTML element should have no Namespace component (absent ⇒ Html)"
    );
}

#[test]
fn create_element_ns_svg() {
    let mut dom = EcsDom::new();
    let svg = dom.create_element_ns("svg", Namespace::Svg, Attributes::default(), None);
    assert_eq!(dom.namespace_of(svg), Namespace::Svg);
    assert!(!dom.is_html_namespace(svg));
    assert!(dom.world().get::<&Namespace>(svg).is_ok());
}

#[test]
fn create_element_ns_mathml() {
    let mut dom = EcsDom::new();
    let math = dom.create_element_ns("math", Namespace::MathMl, Attributes::default(), None);
    assert_eq!(dom.namespace_of(math), Namespace::MathMl);
    assert!(!dom.is_html_namespace(math));
}

#[test]
fn create_element_ns_html_stays_sparse() {
    // Passing `Namespace::Html` to the namespaced constructor must NOT
    // attach a component (the absent-⇒-Html invariant is preserved).
    let mut dom = EcsDom::new();
    let div = dom.create_element_ns("div", Namespace::Html, Attributes::default(), None);
    assert!(
        dom.world().get::<&Namespace>(div).is_err(),
        "Html namespace must stay component-free (sparse invariant)"
    );
    assert_eq!(dom.namespace_of(div), Namespace::Html);
    assert!(dom.is_html_namespace(div));
}

#[test]
fn namespace_of_defaults_html_for_non_element() {
    // A non-element entity has no namespace; `namespace_of` returns the
    // `Html` default, but `is_html_namespace` is false (not an element).
    let mut dom = EcsDom::new();
    let text = dom.create_text("hi");
    assert_eq!(dom.namespace_of(text), Namespace::Html);
    assert!(!dom.is_html_namespace(text));
}

#[test]
fn namespace_uri_constants() {
    assert_eq!(Namespace::Html.uri(), "http://www.w3.org/1999/xhtml");
    assert_eq!(Namespace::Svg.uri(), "http://www.w3.org/2000/svg");
    assert_eq!(
        Namespace::MathMl.uri(),
        "http://www.w3.org/1998/Math/MathML"
    );
}

#[test]
fn is_base_element_html_namespace_only() {
    // HTML `<base>` is the document base; a foreign-namespace `<base>`
    // look-alike (HTML §4.2.3 restricts `<base>` to the HTML namespace) is
    // not.
    let mut dom = EcsDom::new();
    let html_base = elem(&mut dom, "base");
    assert!(dom.is_base_element(html_base));

    let svg_base = dom.create_element_ns("base", Namespace::Svg, Attributes::default(), None);
    assert!(
        !dom.is_base_element(svg_base),
        "foreign-namespace <base> is not the document base element"
    );
}
