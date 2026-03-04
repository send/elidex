use super::*;

#[test]
fn specificity_tag() {
    let sel = parse_sel("div").unwrap();
    assert_eq!(sel.specificity, spec(0, 0, 1));
}

#[test]
fn specificity_class() {
    let sel = parse_sel(".foo").unwrap();
    assert_eq!(sel.specificity, spec(0, 1, 0));
}

#[test]
fn specificity_id() {
    let sel = parse_sel("#bar").unwrap();
    assert_eq!(sel.specificity, spec(1, 0, 0));
}

#[test]
fn specificity_compound() {
    let sel = parse_sel("div.foo#bar").unwrap();
    assert_eq!(sel.specificity, spec(1, 1, 1));
}

#[test]
fn specificity_ordering() {
    let id = spec(1, 0, 0);
    let class = spec(0, 1, 0);
    let tag = spec(0, 0, 1);
    assert!(id > class);
    assert!(class > tag);
}

// --- M3-3: Specificity tests ---

#[test]
fn specificity_attr_presence() {
    let sel = parse_sel("[attr]").unwrap();
    assert_eq!(sel.specificity, spec(0, 1, 0));
}

#[test]
fn specificity_attr_value() {
    let sel = parse_sel(r"[attr=val]").unwrap();
    assert_eq!(sel.specificity, spec(0, 1, 0));
}

#[test]
fn specificity_not_id() {
    // CSS Selectors Level 3: :not() specificity = argument specificity.
    // :not(#id) -> (1, 0, 0), not (1, 1, 0).
    let sel = parse_sel(":not(#id)").unwrap();
    assert_eq!(sel.specificity, spec(1, 0, 0));
}

#[test]
fn specificity_tag_first_child() {
    let sel = parse_sel("div:first-child").unwrap();
    assert_eq!(sel.specificity, spec(0, 1, 1));
}
