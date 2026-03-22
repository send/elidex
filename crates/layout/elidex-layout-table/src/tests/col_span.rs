//! Tests for `col_span_count` reading from HTML span attribute.

use super::*;

#[test]
fn col_span_default_1() {
    let mut dom = EcsDom::new();
    let td = dom.create_element("td", Attributes::default());
    assert_eq!(crate::col_span_count(&dom, td), 1);
}

#[test]
fn col_span_attribute_2() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("span", "2");
    let td = dom.create_element("td", attrs);
    assert_eq!(crate::col_span_count(&dom, td), 2);
}

#[test]
fn col_span_clamped() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("span", "9999");
    let td = dom.create_element("td", attrs);
    assert_eq!(crate::col_span_count(&dom, td), 1000);
}

#[test]
fn col_span_invalid() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("span", "abc");
    let td = dom.create_element("td", attrs);
    assert_eq!(crate::col_span_count(&dom, td), 1);
}
