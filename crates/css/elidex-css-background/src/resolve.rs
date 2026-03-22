//! `CssValue` → typed conversion helpers for background properties.
//!
//! Used by `resolve_background_layers()` in elidex-style.

use elidex_plugin::{
    background::{
        BackgroundImage, BgAttachment, BgPosition, BgRepeat, BgRepeatAxis, BgSize, BoxArea,
    },
    CssColor, CssValue, Point, Size,
};

/// Convert a `CssValue` to a `BackgroundImage`.
#[must_use]
pub fn resolve_bg_image(value: &CssValue) -> BackgroundImage {
    use elidex_plugin::background::{ConicGradient, LinearGradient, RadialGradient};
    use elidex_plugin::{AngleOrDirection, GradientValue};

    match value {
        CssValue::Keyword(k) if k == "none" => BackgroundImage::None,
        CssValue::Url(u) => BackgroundImage::Url(u.clone()),
        CssValue::Gradient(g) => match g.as_ref() {
            GradientValue::Linear {
                direction,
                stops,
                repeating,
            } => {
                let angle = match direction {
                    AngleOrDirection::Angle(a) => *a,
                    AngleOrDirection::To(sides) => direction_to_angle(sides),
                };
                let resolved_stops = resolve_color_stops(stops);
                BackgroundImage::LinearGradient(LinearGradient {
                    angle,
                    stops: resolved_stops,
                    repeating: *repeating,
                })
            }
            GradientValue::Radial {
                shape: _,
                size: _,
                position,
                stops,
                repeating,
            } => {
                // Default center: 50% 50% (resolved later against painting area)
                let center = resolve_position_pair(position.as_ref());
                let resolved_stops = resolve_color_stops(stops);
                BackgroundImage::RadialGradient(RadialGradient {
                    center,
                    radii: Size::ZERO, // Resolved against painting area at render time
                    stops: resolved_stops,
                    repeating: *repeating,
                })
            }
            GradientValue::Conic {
                from_angle,
                position,
                stops,
                repeating,
            } => {
                let from_angle = from_angle.unwrap_or(0.0);
                let center = resolve_position_pair(position.as_ref());
                let resolved_stops = resolve_angular_stops(stops);
                BackgroundImage::ConicGradient(ConicGradient {
                    center,
                    start_angle: from_angle,
                    end_angle: from_angle + 360.0,
                    stops: resolved_stops,
                    repeating: *repeating,
                })
            }
        },
        _ => BackgroundImage::None,
    }
}

/// Convert `to <side-or-corner>` keywords to an angle in degrees.
fn direction_to_angle(sides: &[String]) -> f32 {
    match sides.len() {
        1 => match sides[0].as_str() {
            "top" => 0.0,
            "right" => 90.0,
            "left" => 270.0,
            _ => 180.0,
        },
        2 => {
            let (s1, s2) = (sides[0].as_str(), sides[1].as_str());
            match (s1, s2) {
                ("top", "right") | ("right", "top") => 45.0,
                ("top", "left") | ("left", "top") => 315.0,
                ("bottom", "right") | ("right", "bottom") => 135.0,
                ("bottom", "left") | ("left", "bottom") => 225.0,
                _ => 180.0,
            }
        }
        _ => 180.0,
    }
}

/// Resolve parse-stage color stops to computed stops with positions 0.0–1.0.
fn resolve_color_stops(
    stops: &[elidex_plugin::CssColorStop],
) -> Vec<elidex_plugin::background::ColorStop> {
    use elidex_plugin::background::ColorStop;

    let mut resolved: Vec<(Option<f32>, CssColor)> = stops
        .iter()
        .map(|s| {
            let color = match &s.color {
                CssValue::Color(c) => *c,
                _ => CssColor::TRANSPARENT,
            };
            let pos = s.position.as_ref().and_then(|p| match p {
                CssValue::Percentage(pct) => Some(*pct / 100.0),
                CssValue::Length(v, _) => Some(*v), // px, will be normalized later
                _ => None,
            });
            (pos, color)
        })
        .collect();

    // Auto-distribute: first stop defaults to 0.0, last to 1.0
    if let Some(first) = resolved.first_mut() {
        if first.0.is_none() {
            first.0 = Some(0.0);
        }
    }
    if let Some(last) = resolved.last_mut() {
        if last.0.is_none() {
            last.0 = Some(1.0);
        }
    }

    // Fill in missing positions by linear interpolation
    let len = resolved.len();
    let mut i = 0;
    while i < len {
        if resolved[i].0.is_none() {
            // Find next defined position
            let start = i - 1;
            let mut end = i + 1;
            while end < len && resolved[end].0.is_none() {
                end += 1;
            }
            let start_pos = resolved[start].0.unwrap_or(0.0);
            let end_pos = resolved[end.min(len - 1)].0.unwrap_or(1.0);
            let count = end - start;
            #[allow(clippy::cast_precision_loss)]
            for (idx, item) in resolved[(start + 1)..end].iter_mut().enumerate() {
                let t = (idx + 1) as f32 / count as f32;
                item.0 = Some(start_pos + t * (end_pos - start_pos));
            }
            i = end;
        } else {
            i += 1;
        }
    }

    resolved
        .into_iter()
        .map(|(pos, color)| ColorStop {
            color,
            position: pos.unwrap_or(0.0),
        })
        .collect()
}

/// Resolve angular color stops for conic gradients (positions in degrees 0–360).
fn resolve_angular_stops(
    stops: &[elidex_plugin::CssColorStop],
) -> Vec<elidex_plugin::background::ColorStop> {
    // Same logic as linear but positions are in degrees (0–360)
    let mut resolved: Vec<(Option<f32>, CssColor)> = stops
        .iter()
        .map(|s| {
            let color = match &s.color {
                CssValue::Color(c) => *c,
                _ => CssColor::TRANSPARENT,
            };
            let pos = s.position.as_ref().and_then(|p| match p {
                CssValue::Percentage(pct) => Some(*pct / 100.0 * 360.0),
                CssValue::Angle(a) => Some(*a),
                _ => None,
            });
            (pos, color)
        })
        .collect();

    if let Some(first) = resolved.first_mut() {
        if first.0.is_none() {
            first.0 = Some(0.0);
        }
    }
    if let Some(last) = resolved.last_mut() {
        if last.0.is_none() {
            last.0 = Some(360.0);
        }
    }

    // Fill gaps
    let len = resolved.len();
    let mut i = 0;
    while i < len {
        if resolved[i].0.is_none() {
            let start = i - 1;
            let mut end = i + 1;
            while end < len && resolved[end].0.is_none() {
                end += 1;
            }
            let start_pos = resolved[start].0.unwrap_or(0.0);
            let end_pos = resolved[end.min(len - 1)].0.unwrap_or(360.0);
            let count = end - start;
            #[allow(clippy::cast_precision_loss)]
            for (idx, item) in resolved[(start + 1)..end].iter_mut().enumerate() {
                let t = (idx + 1) as f32 / count as f32;
                item.0 = Some(start_pos + t * (end_pos - start_pos));
            }
            i = end;
        } else {
            i += 1;
        }
    }

    resolved
        .into_iter()
        .map(|(pos, color)| elidex_plugin::background::ColorStop {
            color,
            position: pos.unwrap_or(0.0),
        })
        .collect()
}

/// Resolve position pair from parse-stage `CssValue` list to a `Point` (percentages).
fn resolve_position_pair(position: Option<&Vec<CssValue>>) -> Point {
    let Some(pos) = position else {
        return Point::new(50.0, 50.0); // center center
    };
    let x = match pos.first() {
        Some(CssValue::Percentage(p)) => *p,
        Some(CssValue::Length(v, _)) => *v, // px (will be resolved against area)
        _ => 50.0,
    };
    let y = match pos.get(1) {
        Some(CssValue::Percentage(p)) => *p,
        Some(CssValue::Length(v, _)) => *v,
        _ => 50.0,
    };
    Point::new(x, y)
}

/// Convert a `CssValue` to a `BgRepeat`.
#[must_use]
pub fn resolve_bg_repeat(value: &CssValue) -> BgRepeat {
    fn axis_from_str(s: &str) -> BgRepeatAxis {
        match s {
            "no-repeat" => BgRepeatAxis::NoRepeat,
            "space" => BgRepeatAxis::Space,
            "round" => BgRepeatAxis::Round,
            _ => BgRepeatAxis::Repeat,
        }
    }
    match value {
        CssValue::Keyword(k) => {
            let a = axis_from_str(k);
            BgRepeat { x: a, y: a }
        }
        CssValue::List(items) if items.len() == 2 => {
            let x = items[0]
                .as_keyword()
                .map_or(BgRepeatAxis::Repeat, axis_from_str);
            let y = items[1]
                .as_keyword()
                .map_or(BgRepeatAxis::Repeat, axis_from_str);
            BgRepeat { x, y }
        }
        _ => BgRepeat::default(),
    }
}

/// Convert a `CssValue` to a `BgPosition`.
#[must_use]
pub fn resolve_bg_position(value: &CssValue) -> BgPosition {
    use elidex_plugin::background::BgPositionAxis;
    match value {
        CssValue::Percentage(p) => BgPosition {
            x: BgPositionAxis::Percentage(*p),
            y: BgPositionAxis::Percentage(50.0),
        },
        CssValue::Length(v, _) => BgPosition {
            x: BgPositionAxis::Length(*v),
            y: BgPositionAxis::Percentage(50.0),
        },
        CssValue::List(items) if items.len() == 2 => {
            let x = css_to_position_axis(&items[0]);
            let y = css_to_position_axis(&items[1]);
            BgPosition { x, y }
        }
        _ => BgPosition::default(),
    }
}

fn css_to_position_axis(value: &CssValue) -> elidex_plugin::background::BgPositionAxis {
    use elidex_plugin::background::BgPositionAxis;
    match value {
        CssValue::Percentage(p) => BgPositionAxis::Percentage(*p),
        CssValue::Length(v, _) => BgPositionAxis::Length(*v),
        CssValue::Keyword(k) => match k.as_str() {
            "right" | "bottom" => BgPositionAxis::Percentage(100.0),
            "center" => BgPositionAxis::Percentage(50.0),
            _ => BgPositionAxis::Percentage(0.0),
        },
        _ => BgPositionAxis::Percentage(0.0),
    }
}

/// Convert a `CssValue` to a `BgSize`.
#[must_use]
pub fn resolve_bg_size(value: &CssValue) -> BgSize {
    use elidex_plugin::background::BgSizeDimension;
    match value {
        CssValue::Keyword(k) => match k.as_str() {
            "cover" => BgSize::Cover,
            "contain" => BgSize::Contain,
            _ => BgSize::default(),
        },
        CssValue::Length(v, _) => BgSize::Explicit(Some(BgSizeDimension::Length(*v)), None),
        CssValue::Percentage(p) => BgSize::Explicit(Some(BgSizeDimension::Percentage(*p)), None),
        CssValue::List(items) if items.len() == 2 => {
            let w = css_to_size_dim(&items[0]);
            let h = css_to_size_dim(&items[1]);
            BgSize::Explicit(w, h)
        }
        _ => BgSize::default(),
    }
}

fn css_to_size_dim(value: &CssValue) -> Option<elidex_plugin::background::BgSizeDimension> {
    use elidex_plugin::background::BgSizeDimension;
    match value {
        CssValue::Length(v, _) => Some(BgSizeDimension::Length(*v)),
        CssValue::Percentage(p) => Some(BgSizeDimension::Percentage(*p)),
        _ => None,
    }
}

/// Convert a `CssValue` to a `BoxArea`.
#[must_use]
pub fn resolve_box_area_keyword(value: &CssValue) -> BoxArea {
    match value {
        CssValue::Keyword(k) => match k.as_str() {
            "border-box" => BoxArea::BorderBox,
            "content-box" => BoxArea::ContentBox,
            _ => BoxArea::PaddingBox,
        },
        _ => BoxArea::PaddingBox,
    }
}

/// Convert a `CssValue` to a `BgAttachment`.
#[must_use]
pub fn resolve_bg_attachment(value: &CssValue) -> BgAttachment {
    match value {
        CssValue::Keyword(k) => match k.as_str() {
            "fixed" => BgAttachment::Fixed,
            "local" => BgAttachment::Local,
            _ => BgAttachment::Scroll,
        },
        _ => BgAttachment::Scroll,
    }
}
