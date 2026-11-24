//! Prepare the single print reference context shared by the two paged builders.
use elidex_ecs::EcsDom;
use elidex_plugin::{
    ComputedStyle, CssValue, LengthUnit, PageMargins, PagedMediaContext, PropertyDeclaration,
    ResolveContext, Size,
};
use elidex_text::FontDatabase;

use crate::{builder, PagedDisplayList};

/// Paint previously laid-out fragments using the current document's font and page area.
#[must_use]
pub fn build_paged_display_lists(
    dom: &EcsDom,
    font_db: &FontDatabase,
    fragments: &[elidex_layout::PageFragment],
    context: &PagedMediaContext,
) -> PagedDisplayList {
    builder::build_paged_display_lists(dom, font_db, fragments, &prepare(dom, context))
}

/// Lay out and paint each page with the same margin-box preparation as precomputed fragments.
#[must_use]
pub fn build_paged_display_lists_interleaved(
    dom: &mut EcsDom,
    font_db: &FontDatabase,
    context: &PagedMediaContext,
) -> PagedDisplayList {
    let prepared = prepare(dom, context);
    builder::build_paged_display_lists_interleaved(dom, font_db, &prepared)
}

fn font_size(properties: &[PropertyDeclaration], ctx: &ResolveContext) -> Option<f32> {
    properties.iter().rev().find_map(|d| {
        if d.property != "font-size" {
            return None;
        }
        match &d.value {
            CssValue::Length(n, u) => Some(elidex_plugin::css_resolve::resolve_length(*n, *u, ctx)),
            CssValue::Percentage(p) => Some(ctx.em_base * p / 100.),
            CssValue::Number(n) => Some(*n),
            _ => None,
        }
        .filter(|v| v.is_finite() && *v >= 0.)
    })
}

fn prepare(dom: &EcsDom, context: &PagedMediaContext) -> PagedMediaContext {
    let root_font_size = dom
        .document_root()
        .and_then(|root| dom.first_element_child(root))
        .and_then(|root| {
            dom.world()
                .get::<&ComputedStyle>(root)
                .ok()
                .map(|style| style.font_size)
        })
        .unwrap_or_else(|| ResolveContext::default().root_font_size);
    let root_ctx = ResolveContext {
        root_font_size,
        em_base: root_font_size,
        viewport: Size::new(context.content_width(), context.content_height()),
    };
    // CSS Page 3 §6: margin boxes inherit from the page context. The
    // document's root font and the print ICB remain separate references.
    let page_font = context
        .page_rules
        .iter()
        .rev()
        .filter(|rule| rule.selectors.is_empty())
        .find_map(|rule| font_size(&rule.properties, &root_ctx))
        .unwrap_or(root_font_size);
    let page_ctx = root_ctx.with_em_base(page_font);
    let mut prepared = context.clone();
    for rule in &mut prepared.page_rules {
        for decl in &mut rule.properties {
            if decl.property.starts_with("margin-") {
                normalize(&mut decl.value, &page_ctx);
            }
        }
        prepare_boxes(&mut rule.margins, &page_ctx);
    }
    prepared
}

fn normalize(value: &mut CssValue, ctx: &ResolveContext) {
    if let CssValue::Length(n, unit) = value {
        *value = CssValue::Length(
            elidex_plugin::css_resolve::resolve_length(*n, *unit, ctx),
            LengthUnit::Px,
        );
    }
}

fn prepare_boxes(margins: &mut PageMargins, page: &ResolveContext) {
    for slot in [
        &mut margins.top_left_corner,
        &mut margins.top_left,
        &mut margins.top_center,
        &mut margins.top_right,
        &mut margins.top_right_corner,
        &mut margins.right_top,
        &mut margins.right_middle,
        &mut margins.right_bottom,
        &mut margins.bottom_right_corner,
        &mut margins.bottom_right,
        &mut margins.bottom_center,
        &mut margins.bottom_left,
        &mut margins.bottom_left_corner,
        &mut margins.left_bottom,
        &mut margins.left_middle,
        &mut margins.left_top,
    ]
    .into_iter()
    .flatten()
    {
        let size = font_size(&slot.properties, page).unwrap_or(page.em_base);
        let ctx = page.with_em_base(size);
        let mut has_font = false;
        for decl in &mut slot.properties {
            if decl.property == "font-size" {
                decl.value = CssValue::Length(size, LengthUnit::Px);
                has_font = true;
            } else {
                normalize(&mut decl.value, &ctx);
            }
        }
        if !has_font {
            slot.properties.push(PropertyDeclaration::new(
                "font-size",
                CssValue::Length(size, LengthUnit::Px),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::MarginBoxContent;

    #[test]
    fn zero_page_box_font_resolves_to_zero_for_px_and_inches() {
        for unit in [LengthUnit::Px, LengthUnit::In] {
            let mut margins = PageMargins {
                top_left: Some(MarginBoxContent {
                    content: elidex_plugin::ContentValue::default(),
                    properties: vec![PropertyDeclaration::new(
                        "font-size",
                        CssValue::Length(0., unit),
                    )],
                }),
                ..PageMargins::default()
            };
            prepare_boxes(&mut margins, &ResolveContext::default());
            let props = margins.top_left.unwrap().properties;
            assert_eq!(props.len(), 1);
            assert_eq!(props[0].value, CssValue::Length(0., LengthUnit::Px));
        }
    }
}
