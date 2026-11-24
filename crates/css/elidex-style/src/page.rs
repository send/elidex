//! Cascade and resolve page-context declarations before paginated layout.
use crate::{
    cascade::{push_page_block, CascadeEntry},
    resolve::{resolve_font_size, resolve_in_context},
};
use elidex_css::{page::content_value_from_css_value, Stylesheet};
use elidex_plugin::{
    ComputedStyle, CssValue, EdgeSizes, LengthUnit, MarginBoxContent, NamedPageSize, PageGeometry,
    PreparedPage, PreparedPages, PropertyDeclaration, ResolveContext, Size,
};
use std::collections::BTreeMap;

fn length(value: &CssValue, base: f32, ctx: &ResolveContext) -> Option<f32> {
    match value {
        CssValue::Length(n, u) => Some(elidex_plugin::css_resolve::resolve_length(*n, *u, ctx)),
        CssValue::Calc(e) => Some(elidex_plugin::css_resolve::resolve_calc_expr(e, base, ctx)),
        CssValue::Percentage(n) => Some(elidex_plugin::css_resolve::percentage_of(base, *n)),
        _ => None,
    }
}

fn sheet_size(value: Option<&&CssValue>, device: Size, ctx: &ResolveContext) -> Size {
    let Some(CssValue::List(items)) = value else {
        return device;
    };
    match items.as_slice() {
        [w, rest @ ..] if !matches!(w, CssValue::Keyword(_)) => {
            let w = length(w, 0., ctx).unwrap_or_default().max(0.);
            let h = rest
                .first()
                .and_then(|v| length(v, 0., ctx))
                .unwrap_or(w)
                .max(0.);
            Size::new(w, h)
        }
        _ => {
            let mut sheet = device;
            for item in items {
                if let CssValue::Keyword(k) = item {
                    if let Some(named) = NamedPageSize::from_keyword(k) {
                        let (w, h) = named.dimensions();
                        sheet = Size::new(w, h);
                    } else if (k == "landscape" && sheet.width < sheet.height)
                        || (k == "portrait" && sheet.width > sheet.height)
                    {
                        std::mem::swap(&mut sheet.width, &mut sheet.height);
                    }
                }
            }
            sheet
        }
    }
}

fn prepare_geometry(
    sheets: &[&Stylesheet],
    device: PageGeometry,
    root: &ComputedStyle,
    number: usize,
    blank: bool,
    viewport: Size,
) -> (PreparedPage, f32) {
    let ctx = ResolveContext {
        viewport,
        em_base: root.font_size,
        root_font_size: root.font_size,
    };
    let mut matched_selectors = Vec::new();
    let mut candidates = Vec::new();
    let mut boxes: BTreeMap<&str, Vec<CascadeEntry>> = BTreeMap::new();
    for (sheet_index, sheet) in sheets.iter().enumerate() {
        for (order, rule) in sheet.page_rules.iter().enumerate() {
            let mut specificity = None;
            for alternative in rule
                .alternatives
                .iter()
                .filter(|a| a.matches(None, number, blank))
            {
                specificity = Some(alternative.specificity.max(specificity.unwrap_or_default()));
                for pseudo in &alternative.pseudos {
                    if !matched_selectors.contains(pseudo) {
                        matched_selectors.push(*pseudo);
                    }
                }
            }
            let Some(specificity) = specificity else {
                continue;
            };
            let push = |entries: &mut Vec<_>, decls| {
                push_page_block(
                    entries,
                    decls,
                    sheet,
                    u32::try_from(sheet_index).unwrap_or(u32::MAX),
                    u32::try_from(order).unwrap_or(u32::MAX),
                    specificity,
                );
            };
            push(&mut candidates, &rule.declarations);
            for (position, decls) in &rule.boxes {
                push(boxes.entry(position.as_str()).or_default(), decls);
            }
        }
    }
    let resolved = resolve_in_context(&candidates, root, true);
    let winners = resolved.ordinary();
    let mut parent = crate::resolve::build_computed_style(&winners, root, &ctx);
    let font = parent.font_size;
    let page_ctx = ctx.with_em_base(font);
    let sheet = sheet_size(winners.get("size"), device.sheet, &page_ctx);
    let margin = |name, base, default| {
        winners.get(name).map_or(default, |v| {
            let computed = crate::resolve::helpers::resolve_keyword(name, v, root);
            length(computed.as_ref().unwrap_or(v), base, &page_ctx).unwrap_or(0.0)
        })
    };
    let margins = EdgeSizes {
        top: margin("margin-top", sheet.height, device.margins.top),
        bottom: margin("margin-bottom", sheet.height, device.margins.bottom),
        left: margin("margin-left", sheet.width, device.margins.left),
        right: margin("margin-right", sheet.width, device.margins.right),
    };
    parent.custom_properties = resolved.custom;
    let boxes = elidex_css::page::MARGIN_BOX_NAMES
        .iter()
        .filter_map(|&name| boxes.remove(name).map(|entries| (name, entries)))
        .map(|(name, entries)| {
            let box_style = resolve_in_context(&entries, &parent, false);
            let mut winners = box_style.ordinary();
            let content = winners
                .remove("content")
                .map(content_value_from_css_value)
                .unwrap_or_default();
            let mut properties: Vec<_> = winners
                .into_iter()
                .map(|(k, v)| {
                    PropertyDeclaration::new(
                        k,
                        crate::resolve::helpers::resolve_keyword(k, v, &parent)
                            .unwrap_or_else(|| v.clone()),
                    )
                })
                .collect();
            properties.sort_by(|a, b| a.property.cmp(&b.property));
            (
                name.to_owned(),
                MarginBoxContent {
                    content,
                    properties,
                },
            )
        })
        .collect();
    (
        PreparedPage {
            geometry: PageGeometry { sheet, margins },
            boxes,
            matched_selectors,
        },
        font,
    )
}

fn prepare_boxes(page: &mut PreparedPage, em_base: f32, root_font_size: f32, viewport: Size) {
    let parent = ComputedStyle {
        font_size: em_base,
        ..ComputedStyle::default()
    };
    let page_ctx = ResolveContext {
        em_base,
        root_font_size,
        viewport,
    };
    for (_, slot) in &mut page.boxes {
        let values = slot
            .properties
            .iter()
            .map(|d| (d.property.as_str(), &d.value))
            .collect();
        let font = resolve_font_size(&values, &parent, &page_ctx);
        let ctx = page_ctx.with_em_base(font);
        slot.properties.retain(|p| p.property != "font-size");
        slot.properties.push(PropertyDeclaration::new(
            "font-size",
            CssValue::Length(font, LengthUnit::Px),
        ));
        for decl in &mut slot.properties {
            if let CssValue::Length(n, u) = decl.value {
                decl.value = CssValue::Length(
                    elidex_plugin::css_resolve::resolve_length(n, u, &ctx),
                    LengthUnit::Px,
                );
            }
        }
    }
}

/// Prepare page geometry, publish document styles against the first page area,
/// then finish margin-box lengths using the published root font.
/// Print media conditions use the device sheet. The UA sheet minus its default
/// margins supplies the preliminary root and first-page geometry viewport to
/// break their dependency cycle (PM 1001, print style preparation §1 step 4).
/// The selected first page area supplies the document viewport, subsequent
/// page viewport units, and margin-box viewport units (CSS Page 3 §§3, 7.1
/// and CSS Values 4 §6.1.2).
#[must_use]
pub fn prepare_print_style(
    dom: &mut elidex_ecs::EcsDom,
    authors: &[&Stylesheet],
    extra: &[&Stylesheet],
    hints: &dyn Fn(elidex_ecs::Entity, &elidex_ecs::EcsDom) -> Vec<elidex_css::Declaration>,
    device: PageGeometry,
) -> PreparedPages {
    let sheets = crate::style_sheets(authors, extra);
    let media = crate::media_environment(device.sheet, elidex_css::media::Medium::Print);
    let ctx = ResolveContext {
        viewport: device.area().size,
        ..ResolveContext::default()
    };
    let root = crate::walk::preliminary_root(dom, &sheets, &ctx, &media, hints);
    let (mut first, font) = prepare_geometry(&sheets, device, &root, 1, false, ctx.viewport);
    let viewport = first.geometry.area().size;
    crate::apply_document_style(dom, &sheets, hints, viewport, media);
    let final_font = dom
        .document_root()
        .and_then(|r| dom.first_element_child(r))
        .and_then(|r| {
            dom.world()
                .get::<&ComputedStyle>(r)
                .ok()
                .map(|style| style.font_size)
        })
        .unwrap_or(ctx.root_font_size);
    prepare_boxes(&mut first, font, final_font, viewport);
    PreparedPages::from_first(first, |number, blank| {
        let geometry_viewport = if number == 1 { ctx.viewport } else { viewport };
        let (mut page, font) =
            prepare_geometry(&sheets, device, &root, number, blank, geometry_viewport);
        prepare_boxes(&mut page, font, final_font, viewport);
        page
    })
}

/// Prepare an authored stylesheet against a fixed device for layout and render fixtures.
#[cfg(feature = "fixtures")]
#[must_use]
pub fn fixture(width: f32, height: f32, margin: f32, css: &str) -> PreparedPages {
    let sheet = elidex_css::parse_stylesheet_with_registry(
        css,
        elidex_css::Origin::Author,
        Some(crate::default_css_property_registry()),
    );
    prepare_print_style(
        &mut elidex_ecs::EcsDom::new(),
        &[&sheet],
        &[],
        &|_, _| Vec::new(),
        PageGeometry {
            sheet: Size::new(width, height),
            margins: EdgeSizes::uniform(margin),
        },
    )
}

#[cfg(test)]
mod tests;
