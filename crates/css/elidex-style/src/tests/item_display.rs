//! Computed display observed before layout, including after a second cascade.
use super::*;

fn child(dom: &mut EcsDom, parent: Entity, tag: &str) -> Entity {
    let entity = dom.create_element_ownerless_fixture(tag, Attributes::default());
    assert!(dom.append_child(parent, entity));
    entity
}

fn resolve(dom: &mut EcsDom, css: &str) {
    let sheet = parse_stylesheet(css, Origin::Author);
    resolve_styles(dom, &[&sheet], Size::new(300., 200.));
}

#[test]
fn inherited_display_reads_finalized_parent_on_each_restyle() {
    let (mut dom, _, _, body) = build_simple_dom();
    let item = child(&mut dom, body, "span");
    let leaf = child(&mut dom, item, "b");
    let boundary = child(&mut dom, body, "div");
    let sibling = child(&mut dom, boundary, "span");
    for (container, expected) in [
        ("flex", Display::Block),
        ("block", Display::Inline),
        ("flex", Display::Block),
    ] {
        resolve(&mut dom, &format!("body {{display:{container}}} span {{display:inline}} b {{display:inherit}} div {{display:block}}"));
        assert_eq!(get_style(&dom, item).display, expected);
        assert_eq!(get_style(&dom, leaf).display, expected);
        assert_eq!(get_style(&dom, sibling).display, Display::Inline);
    }
}

#[test]
fn contents_fixups_and_hidden_grid_or_flex_subtrees() {
    let (mut dom, _, html, body) = build_simple_dom();
    let root_child = child(&mut dom, html, "span");
    let wrapper = child(&mut dom, body, "label");
    let item = child(&mut dom, wrapper, "span");
    let canvas = child(&mut dom, body, "canvas");
    let fallback = child(&mut dom, canvas, "span");
    let hidden = child(&mut dom, body, "aside");
    let flex = child(&mut dom, hidden, "section");
    let inline_flex = child(&mut dom, flex, "em");
    let grid = child(&mut dom, hidden, "nav");
    let cell = child(&mut dom, grid, "td");
    let table = child(&mut dom, body, "table");
    resolve(&mut dom, "html,label,canvas {display:contents} body,section {display:flex} span {display:inline} aside {display:none} em {display:inline-flex} nav {display:inline-grid} td {display:table-cell} table {display:inline-table}");
    for (node, display) in [
        (html, Display::Block),
        (root_child, Display::Inline),
        (wrapper, Display::Contents),
        (item, Display::Block),
        (canvas, Display::None),
        (fallback, Display::Inline),
        (hidden, Display::None),
        (inline_flex, Display::Flex),
        (grid, Display::InlineGrid),
        (cell, Display::Block),
        (table, Display::Table),
    ] {
        assert_eq!(get_style(&dom, node).display, display);
    }
}

#[test]
fn generated_display_default_participates_in_adjustment() {
    let (mut dom, _, _, body) = build_simple_dom();
    let wrapper = child(&mut dom, body, "label");
    let block = child(&mut dom, body, "div");
    resolve(&mut dom, "body {display:flex} label {display:contents} label::before, div::before, div::after {content:'x'} div::after {position:absolute}");
    for (node, expected) in [
        (dom.get_first_child(wrapper), Display::Block),
        (dom.get_first_child(block), Display::Inline),
        (dom.get_last_child(block), Display::Block),
    ] {
        assert_eq!(get_style(&dom, node.unwrap()).display, expected);
    }
    resolve(&mut dom, "body {display:block} div::before {content:'x';display:inherit} div::after {content:'y';display:inline-grid}");
    assert_eq!(
        get_style(&dom, dom.get_first_child(block).unwrap()).display,
        Display::Block
    );
    assert_eq!(
        get_style(&dom, dom.get_last_child(block).unwrap()).display,
        Display::InlineGrid
    );
}

#[test]
fn assigned_host_descends_through_its_own_shadow_and_restores_sibling_context() {
    let (mut dom, _, _, body) = build_simple_dom();
    let outer = child(&mut dom, body, "div");
    let assigned = child(&mut dom, outer, "section");
    let outer_root = setup_shadow_with_style(&mut dom, outer, "slot {display:contents}");
    child(&mut dom, outer_root, "slot");
    let own_root = setup_shadow_with_style(
        &mut dom,
        assigned,
        ":host {display:flex} b {display:inline-flex}",
    );
    let inside = child(&mut dom, own_root, "b");
    let sibling = child(&mut dom, body, "i");
    for (host_css, expected) in [
        ("", Display::Flex),
        ("section {display:block}", Display::InlineFlex),
    ] {
        resolve(&mut dom, &format!("i {{display:inline-flex}} {host_css}"));
        assert_eq!(get_style(&dom, inside).display, expected);
        assert_eq!(get_style(&dom, sibling).display, Display::InlineFlex);
    }
}

#[test]
fn contents_slots_pass_effective_display_and_keep_slot_inheritance() {
    let (mut dom, _, _, body) = build_simple_dom();
    let host = child(&mut dom, body, "div");
    let assigned = child(&mut dom, host, "span");
    let root = setup_shadow_with_style(
        &mut dom,
        host,
        "section {display:flex} slot {display:contents;color:red} b {display:inline}",
    );
    let flex = child(&mut dom, root, "section");
    let slot = child(&mut dom, flex, "slot");
    let suppressed = child(&mut dom, slot, "b");
    let empty = child(&mut dom, root, "slot");
    dom.set_attribute(empty, "name", "empty");
    let fallback = child(&mut dom, empty, "b");
    resolve(&mut dom, "span {display:inline}");
    for (node, expected) in [(assigned, Display::Block), (fallback, Display::Inline)] {
        let style = get_style(&dom, node);
        assert_eq!((style.color, style.display), (CssColor::RED, expected));
    }
    assert!(dom.world().get::<&ComputedStyle>(suppressed).is_err());
}
