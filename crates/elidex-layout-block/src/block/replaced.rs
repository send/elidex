//! Width and height resolution for replaced elements (e.g. `<img>`).

use elidex_plugin::{BoxSizing, ComputedStyle, Dimension, EdgeSizes};

use crate::{horizontal_pb, resolve_dimension_value, sanitize, vertical_pb};

/// Resolve width for a replaced element (e.g. `<img>`).
///
/// CSS 2.1 §10.3.2: replaced elements with `width: auto` use intrinsic width.
/// When only height is specified, width is computed from the aspect ratio.
#[allow(clippy::cast_precision_loss)]
pub(super) fn resolve_replaced_width(
    style: &ComputedStyle,
    containing_width: f32,
    intrinsic_w: u32,
    intrinsic_h: u32,
    padding: &EdgeSizes,
    border: &EdgeSizes,
) -> f32 {
    let iw = intrinsic_w as f32;
    let ih = intrinsic_h as f32;

    if style.width == Dimension::Auto {
        match style.height {
            Dimension::Length(h) if h.is_finite() && ih > 0.0 => {
                // height specified, width auto: compute from aspect ratio.
                let css_h = if style.box_sizing == BoxSizing::BorderBox {
                    (h - vertical_pb(padding, border)).max(0.0)
                } else {
                    h
                };
                (css_h * iw / ih).max(0.0)
            }
            _ => iw, // Both auto or height auto: use intrinsic width.
        }
    } else {
        let raw = sanitize(resolve_dimension_value(style.width, containing_width, iw));
        if style.box_sizing == BoxSizing::BorderBox {
            (raw - horizontal_pb(padding, border)).max(0.0)
        } else {
            raw
        }
    }
}

/// Resolve height for a replaced element (e.g. `<img>`).
///
/// CSS 2.1 §10.6.2: replaced elements with `height: auto` use intrinsic height.
/// When only width is specified, height is computed from the aspect ratio.
#[allow(clippy::cast_precision_loss)]
pub(super) fn resolve_replaced_height(
    style: &ComputedStyle,
    used_width: f32,
    intrinsic_w: u32,
    intrinsic_h: u32,
    padding: &EdgeSizes,
    border: &EdgeSizes,
) -> f32 {
    let iw = intrinsic_w as f32;
    let ih = intrinsic_h as f32;

    match style.height {
        Dimension::Auto => {
            if !matches!(style.width, Dimension::Auto) && iw > 0.0 {
                // width specified, height auto: compute from aspect ratio.
                (used_width * ih / iw).max(0.0)
            } else {
                ih // Both auto: use intrinsic height.
            }
        }
        Dimension::Length(h) if h.is_finite() => {
            if style.box_sizing == BoxSizing::BorderBox {
                (h - vertical_pb(padding, border)).max(0.0)
            } else {
                h
            }
        }
        _ => ih,
    }
}
