use super::*;
use elidex_css::{parse_stylesheet, Origin};
use elidex_plugin::GridTrackList;

struct AreaProbe(Size);

fn record_area_probe(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> elidex_layout_block::LayoutOutcome {
    let result = dispatch_layout_child(dom, entity, input);
    if input.is_probe
        && matches!(
            input.item_context,
            elidex_layout_block::ItemContext::GridArea(_)
        )
    {
        dom.world_mut()
            .insert_one(entity, AreaProbe(result.layout_box.content.size))
            .unwrap();
    }
    result
}

#[test]
fn subgrid_area_probe_and_publication_agree() {
    for paged in [false, true] {
        for (col, row) in [(true, false), (false, true), (true, true), (false, false)] {
            let width = if col { 100. } else { 40. };
            let height = if row { 100. } else { 40. };
            let lb = geometry_with_root(
                ComputedStyle::default(),
                ComputedStyle {
                    width: Dimension::Length(40.),
                    height: Dimension::Length(40.),
                    max_width: Dimension::Length(50.),
                    max_height: Dimension::Length(50.),
                    justify_self: JustifySelf::End,
                    align_self: AlignSelf::End,
                    ..Default::default()
                },
                0.,
                paged,
                |dom, _, item| {
                    let mut style = dom.world_mut().get::<&mut ComputedStyle>(item).unwrap();
                    style.display = Display::Grid;
                    if col {
                        style.grid_template_columns = GridTrackList::Subgrid { line_names: vec![] };
                    }
                    if row {
                        style.grid_template_rows = GridTrackList::Subgrid { line_names: vec![] };
                    }
                },
                |dom, grid, item, input| {
                    elidex_layout_grid::layout_grid(dom, grid, input, record_area_probe);
                    let probe = dom
                        .world()
                        .get::<&AreaProbe>(item)
                        .expect("item received an area probe");
                    assert_eq!(probe.0, Size::new(width, height));
                },
            );
            assert_eq!(
                (lb.content.origin, lb.content.size),
                (
                    Point::new(140. - width, 140. - height),
                    Size::new(width, height)
                )
            );
        }
    }
}

fn layout_authored(dom: &mut EcsDom, root: Entity, viewport: Size, paged: bool) {
    if paged {
        let env = elidex_layout_block::LayoutEnv {
            font_db: &FONTS,
            layout_child: dispatch_layout_child,
            depth: 0,
            viewport: Some(viewport),
            scroll_limits: elidex_layout_block::ScrollLimits::default(),
            layout_generation: 0,
            is_probe: false,
        };
        let input = LayoutInput {
            depth: 0,
            containing: CssSize::definite(500., 500.),
            is_probe: false,
            ..LayoutInput::probe(&env, 500.)
        };
        let page = elidex_layout_block::FragmentainerContext {
            available_block_size: 500.,
            fragmentation_type: elidex_layout_block::FragmentationType::Page,
        };
        assert_eq!(layout_fragmented(dom, root, &input, page).len(), 1);
    } else {
        layout_tree(dom, viewport, &FONTS);
    }
}

#[test]
fn authored_grid_uses_nearest_scroll_origin_after_relayout() {
    for paged in [false, true] {
        for (parent_value, child_value, positions) in [
            ("end", "auto", [20., 40., 65.]),
            ("safe end", "auto", [40., 40., 65.]),
            ("safe end", "unsafe end", [20., 20., 45.]),
        ] {
            let mut dom = EcsDom::new();
            let root = add(&mut dom, None, ComputedStyle::default());
            let scroller = add(&mut dom, Some(root), ComputedStyle::default());
            let grid = add(&mut dom, Some(scroller), ComputedStyle::default());
            let item = add(&mut dom, Some(grid), ComputedStyle::default());
            let leaf = add(&mut dom, Some(item), ComputedStyle::default());
            for (node, name) in [(scroller, "scroller"), (grid, "grid"), (item, "item")] {
                dom.set_attribute(node, "id", name);
            }
            for (overflow, left, expected) in [
                ("visible", 40, positions[0]),
                ("hidden", 40, positions[1]),
                ("hidden", 65, positions[2]),
            ] {
                let css=format!("div {{ display:block }}
                    #scroller {{ overflow:{overflow}; margin-left:{left}px; width:100px }}
                    #grid {{ display:grid; grid-template-columns:100px; grid-template-rows:100px; justify-items:{parent_value} }}
                    #item {{ width:120px; margin-left:auto; justify-self:{child_value} }}");
                let sheet = parse_stylesheet(&css, Origin::Author);
                let viewport = Size::new(500., 500.);
                elidex_style::resolve_styles(&mut dom, &[&sheet], viewport);
                layout_authored(&mut dom, root, viewport, paged);
                assert_eq!(get_layout(&dom, item).margin.left, 0.);
                assert_eq!(get_layout(&dom, item).content.origin.x, expected);
                assert_eq!(get_layout(&dom, leaf).content.origin.x, expected);
            }
        }
    }
}

#[test]
fn authored_self_values_preserve_serialization_through_placement() {
    for paged in [false, true] {
        for (child_value, size, coordinate) in [
            ("auto", 40, 60.),
            ("auto", 120, 0.),
            ("unsafe end", 120, -20.),
        ] {
            let mut dom = EcsDom::new();
            let grid = add(&mut dom, None, ComputedStyle::default());
            let item = add(&mut dom, Some(grid), ComputedStyle::default());
            dom.set_attribute(grid, "id", "grid");
            dom.set_attribute(item, "id", "item");
            let css=format!("#grid {{display:grid;grid-template-columns:100px;grid-template-rows:100px;justify-items:safe end;align-items:safe end}}
                #item {{width:{size}px;height:{size}px;justify-self:{child_value};align-self:{child_value}}}");
            let sheet = parse_stylesheet(&css, Origin::Author);
            let viewport = Size::new(500., 500.);
            elidex_style::resolve_styles(&mut dom, &[&sheet], viewport);
            layout_authored(&mut dom, grid, viewport, paged);
            for (entity, property, expected) in [
                (grid, "justify-items", "safe end"),
                (grid, "align-items", "safe end"),
                (item, "justify-self", child_value),
                (item, "align-self", child_value),
            ] {
                assert_eq!(
                    elidex_style::serialize_resolved_value(
                        property,
                        &elidex_layout_block::get_style(&dom, entity)
                    ),
                    expected
                );
            }
            assert_eq!(
                get_layout(&dom, item).content.origin,
                Point::new(coordinate, coordinate)
            );
        }
    }
}
