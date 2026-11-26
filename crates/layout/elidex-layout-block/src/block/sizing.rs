//! Box sizing shared by normal flow and grid item layout.

use elidex_plugin::{BoxSizing, ComputedStyle, Dimension, EdgeSizes, Size};

use crate::{
    adjust_min_max_for_border_box, clamp_min_max, resolve_min_max, LayoutInput, UsedAssignment,
};

pub(super) struct AxisSizing {
    preferred: Dimension,
    min: Dimension,
    max: Dimension,
    border_box: bool,
}

impl AxisSizing {
    pub(super) fn new(style: &ComputedStyle, width: bool) -> Self {
        let (preferred, min, max) = if width {
            (style.width, style.min_width, style.max_width)
        } else {
            (style.height, style.min_height, style.max_height)
        };
        Self {
            preferred,
            min,
            max,
            border_box: style.box_sizing == BoxSizing::BorderBox,
        }
    }

    pub(super) fn preferred(&self, reference: Option<f32>, auto: f32, pb: f32) -> f32 {
        let specified = match self.preferred {
            Dimension::Length(px) if px.is_finite() => Some(px),
            Dimension::Percentage(pct) => reference.map(|cb| cb * pct / 100.0),
            _ => None,
        };
        specified.map_or(auto, |size| {
            if self.border_box {
                (size - pb).max(0.0)
            } else {
                crate::sanitize(size)
            }
        })
    }

    pub(super) fn constrained(&self, value: f32, reference: f32, pb: f32, replaced: bool) -> f32 {
        let mut min = resolve_min_max(self.min, reference, 0.0);
        let mut max = resolve_min_max(self.max, reference, f32::INFINITY);
        if self.border_box && !replaced {
            adjust_min_max_for_border_box(&mut min, &mut max, pb);
        }
        clamp_min_max(value, min, max)
    }
}

/// Fit an auto inline content box within the available margin-box space
/// (CSS Sizing 3 §2). Percentage edges keep their containing-block reference.
pub fn fit_inline_content(
    style: &ComputedStyle,
    input: &LayoutInput<'_>,
    available: f32,
    inner: &crate::IntrinsicSizes,
) -> f32 {
    let horizontal = style.writing_mode.is_horizontal();
    let (padding, border, margin) = crate::resolve_box_model(style, input.containing_inline_size);
    let margin = input.assigned.margins(margin);
    let (pb, margins) = if horizontal {
        (
            padding.horizontal() + border.horizontal(),
            margin.horizontal(),
        )
    } else {
        (padding.vertical() + border.vertical(), margin.vertical())
    };
    let room = (available - pb - margins).max(0.);
    let fitted = crate::intrinsic::shrink_to_fit_width(inner, room);
    let reference = if horizontal {
        input
            .containing_width_definite
            .then_some(input.containing.width)
    } else {
        input.containing.height
    };
    AxisSizing::new(style, horizontal).constrained(fitted, reference.unwrap_or(0.), pb, false)
}

/// CSS Grid 2 §6.2. The grid supplies stretch intent and intrinsic measurements.
/// The item's formatter resolves these against the definite grid area.
#[derive(Debug, Clone, Copy)]
pub struct GridItemSizing {
    pub stretch: Size<bool>,
    pub subgridded: Size<bool>,
    /// Intrinsic content-box range on the child's own inline axis.
    pub inner_inline: crate::IntrinsicSizes,
    /// Replaced auto axes defer to the formatter's natural size and aspect-ratio rules.
    pub replaced: bool,
    /// Measured content box for distributing auto margins after formatter reconciliation.
    pub measured: Size<Option<f32>>,
}

impl GridItemSizing {
    pub(crate) fn resolve(self, style: &ComputedStyle, input: &LayoutInput<'_>) -> UsedAssignment {
        let area = input.containing;
        let inline = input.containing_inline_size;
        let padding = crate::resolve_padding(style, inline);
        let border = crate::sanitize_border(style);
        let margin = crate::resolve_margins(style, inline);
        let axis = |width,
                    available: f32,
                    pb: f32,
                    stretch: bool,
                    measured: Option<f32>,
                    start_dim,
                    end_dim,
                    start: f32,
                    end: f32| {
            let sizing = AxisSizing::new(style, width);
            let start_auto = start_dim == Dimension::Auto;
            let end_auto = end_dim == Dimension::Auto;
            let room = (available - pb - start - end).max(0.0_f32);
            let child_inline = width == style.writing_mode.is_horizontal();
            let auto = if stretch && !start_auto && !end_auto {
                Some(room)
            } else if child_inline && !self.replaced {
                Some(crate::intrinsic::shrink_to_fit_width(
                    &self.inner_inline,
                    room,
                ))
            } else {
                None
            };
            let subgridded = if width {
                self.subgridded.width
            } else {
                self.subgridded.height
            };
            let content = if subgridded {
                Some(room)
            } else if sizing.preferred == Dimension::Auto && auto.is_none() {
                None
            } else {
                let size = sizing.preferred(Some(available), auto.unwrap_or(0.0), pb);
                Some(sizing.constrained(size, available, pb, false))
            };
            // CSS Grid 2 §11.2: auto margins absorb positive space before self-alignment.
            let free = measured
                .or(content)
                .map_or(0.0, |size| (available - pb - start - end - size).max(0.0));
            let share = if start_auto && end_auto {
                free / 2.0
            } else {
                free
            };
            (
                content,
                Some(if start_auto { share } else { start }),
                Some(if end_auto { share } else { end }),
            )
        };
        let (width, left, right) = axis(
            true,
            area.width,
            padding.horizontal() + border.horizontal(),
            self.stretch.width,
            self.measured.width,
            style.margin_left,
            style.margin_right,
            margin.left,
            margin.right,
        );
        let (height, top, bottom) = axis(
            false,
            area.height.expect("definite grid area"),
            padding.vertical() + border.vertical(),
            self.stretch.height,
            self.measured.height,
            style.margin_top,
            style.margin_bottom,
            margin.top,
            margin.bottom,
        );
        UsedAssignment {
            fit: crate::FitTarget::GridArea(Size { width, height }),
            margin: EdgeSizes {
                top,
                right,
                bottom,
                left,
            },
            ..Default::default()
        }
    }
}
