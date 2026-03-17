//! CSS Backgrounds Level 3 property handler plugin.
//!
//! Handles `background-color`, `background-image`, `background-position`,
//! `background-size`, `background-repeat`, `background-origin`,
//! `background-clip`, `background-attachment`, and the `background` shorthand.

use elidex_plugin::{
    background::{
        BackgroundImage, BgAttachment, BgPosition, BgRepeat, BgRepeatAxis, BgSize, BoxArea,
    },
    css_resolve::resolve_color,
    ComputedStyle, CssColor, CssPropertyHandler, CssValue, LengthUnit, ParseError,
    PropertyDeclaration, ResolveContext,
};

mod gradient;
mod position;
mod shorthand;

#[cfg(test)]
mod tests;

/// CSS Backgrounds Level 3 property handler.
#[derive(Clone)]
pub struct BackgroundHandler;

impl BackgroundHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        elidex_plugin::register_css_handler(registry, Self);
    }
}

/// All property names handled by [`BackgroundHandler`].
const BG_PROPERTIES: &[&str] = &[
    "background-color",
    "background-image",
    "background-position",
    "background-size",
    "background-repeat",
    "background-origin",
    "background-clip",
    "background-attachment",
    "background",
];

impl CssPropertyHandler for BackgroundHandler {
    fn property_names(&self) -> &[&str] {
        BG_PROPERTIES
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        let value = match name {
            "background-color" => elidex_css::parse_color_with_currentcolor(input)?,

            "background-image" => parse_bg_image(input)?,

            "background-repeat" => parse_bg_repeat(input)?,

            "background-origin" | "background-clip" => parse_box_keyword(input)?,

            "background-attachment" => parse_attachment(input)?,

            "background-position" => return position::parse_bg_position_declaration(input),

            "background-size" => parse_bg_size(input)?,

            "background" => return shorthand::parse_background_shorthand(input),

            _ => return Ok(vec![]),
        };
        Ok(vec![PropertyDeclaration::new(name, value)])
    }

    fn resolve(
        &self,
        name: &str,
        value: &CssValue,
        _ctx: &ResolveContext,
        style: &mut ComputedStyle,
    ) {
        // Other background-* longhands are resolved in bulk by
        // resolve_background_layers() in elidex-style (not per-property).
        if name == "background-color" {
            style.background_color = resolve_color(value, style.color);
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "background-color" => CssValue::Color(CssColor::TRANSPARENT),
            "background-image" => CssValue::Keyword("none".to_string()),
            "background-position" => CssValue::Percentage(0.0),
            "background-size" => CssValue::Auto,
            "background-repeat" => CssValue::Keyword("repeat".to_string()),
            "background-origin" => CssValue::Keyword("padding-box".to_string()),
            "background-clip" => CssValue::Keyword("border-box".to_string()),
            "background-attachment" => CssValue::Keyword("scroll".to_string()),
            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, _name: &str) -> bool {
        false
    }

    fn affects_layout(&self, _name: &str) -> bool {
        false
    }

    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue {
        match name {
            "background-color" => CssValue::Color(style.background_color),
            "background-image" => get_computed_bg_image(style),
            "background-position" => get_computed_bg_position(style),
            "background-size" => get_computed_bg_size(style),
            "background-repeat" => get_computed_bg_repeat(style),
            "background-origin" => get_computed_bg_origin(style),
            "background-clip" => get_computed_bg_clip(style),
            "background-attachment" => get_computed_bg_attachment(style),
            _ => CssValue::Initial,
        }
    }
}

// ---------------------------------------------------------------------------
// Parse helpers
// ---------------------------------------------------------------------------

fn parse_bg_image(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try "none"
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
    }
    // Try url()
    if let Ok(url) = input.try_parse(cssparser::Parser::expect_url) {
        return Ok(CssValue::Url(url.as_ref().to_string()));
    }
    if let Ok(gradient) = gradient::parse_gradient(input) {
        return Ok(gradient);
    }
    Err(ParseError {
        property: "background-image".into(),
        input: String::new(),
        message: "expected none, url(), or gradient".into(),
    })
}

fn parse_bg_repeat(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    let first = input.expect_ident().map_err(|_| ParseError {
        property: "background-repeat".into(),
        input: String::new(),
        message: "expected repeat keyword".into(),
    })?;
    let first_lower = first.to_ascii_lowercase();

    // Shorthand keywords
    match first_lower.as_str() {
        "repeat-x" => {
            return Ok(CssValue::List(vec![
                CssValue::Keyword("repeat".into()),
                CssValue::Keyword("no-repeat".into()),
            ]));
        }
        "repeat-y" => {
            return Ok(CssValue::List(vec![
                CssValue::Keyword("no-repeat".into()),
                CssValue::Keyword("repeat".into()),
            ]));
        }
        _ => {}
    }

    let valid = ["repeat", "no-repeat", "space", "round"];
    if !valid.contains(&first_lower.as_str()) {
        return Err(ParseError {
            property: "background-repeat".into(),
            input: first_lower,
            message: "invalid repeat keyword".into(),
        });
    }

    // Try second keyword
    let second = input.try_parse(|i| {
        let ident = i.expect_ident().map_err(|_| ())?;
        let lower = ident.to_ascii_lowercase();
        if valid.contains(&lower.as_str()) {
            Ok(lower)
        } else {
            Err(())
        }
    });

    match second {
        Ok(s) => Ok(CssValue::List(vec![
            CssValue::Keyword(first_lower),
            CssValue::Keyword(s),
        ])),
        Err(()) => {
            // 1-value: same for both axes
            Ok(CssValue::Keyword(first_lower))
        }
    }
}

fn parse_box_keyword(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    elidex_plugin::parse_css_keyword(input, &["border-box", "padding-box", "content-box"])
}

fn parse_attachment(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    elidex_plugin::parse_css_keyword(input, &["scroll", "fixed", "local"])
}

fn parse_bg_size(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // cover | contain
    if input
        .try_parse(|i| i.expect_ident_matching("cover"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("cover".into()));
    }
    if input
        .try_parse(|i| i.expect_ident_matching("contain"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("contain".into()));
    }
    // auto | <length-percentage>
    let first = parse_size_value(input)?;
    let second = input.try_parse(parse_size_value).ok();
    match second {
        Some(s) => Ok(CssValue::List(vec![first, s])),
        None => Ok(first),
    }
}

fn parse_size_value(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }
    elidex_plugin::css_resolve::parse_non_negative_length_or_percentage(input)
}

// ---------------------------------------------------------------------------
// get_computed helpers
// ---------------------------------------------------------------------------

fn get_computed_bg_image(style: &ComputedStyle) -> CssValue {
    let Some(ref layers) = style.background_layers else {
        return CssValue::Keyword("none".to_string());
    };
    if layers.len() == 1 {
        return bg_image_to_css_value(&layers[0].image);
    }
    CssValue::List(
        layers
            .iter()
            .map(|l| bg_image_to_css_value(&l.image))
            .collect(),
    )
}

fn bg_image_to_css_value(img: &BackgroundImage) -> CssValue {
    match img {
        BackgroundImage::Url(u) => CssValue::Url(u.clone()),
        BackgroundImage::LinearGradient(lg) => CssValue::Keyword(serialize_linear_gradient(lg)),
        BackgroundImage::RadialGradient(rg) => CssValue::Keyword(serialize_radial_gradient(rg)),
        BackgroundImage::ConicGradient(cg) => CssValue::Keyword(serialize_conic_gradient(cg)),
        _ => CssValue::Keyword("none".to_string()),
    }
}

/// Format a color as `rgba(r, g, b, a)` or `rgb(r, g, b)`.
fn serialize_color(c: elidex_plugin::CssColor) -> String {
    if c.a == 255 {
        format!("rgb({}, {}, {})", c.r, c.g, c.b)
    } else {
        let alpha = f64::from(c.a) / 255.0;
        // Round to 3 decimal places to avoid excessive precision
        let alpha = (alpha * 1000.0).round() / 1000.0;
        format!("rgba({}, {}, {}, {alpha})", c.r, c.g, c.b)
    }
}

/// Format a float without unnecessary trailing zeros (e.g. `45` not `45.000`).
fn fmt_f32(v: f32) -> String {
    if !v.is_finite() {
        return "0".to_string();
    }
    if v.fract() == 0.0 {
        // Use {:.0} formatting instead of casting to i64, which would
        // overflow for very large floats.
        format!("{v:.0}")
    } else {
        // Up to 3 decimal places
        let s = format!("{v:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn serialize_linear_gradient(lg: &elidex_plugin::background::LinearGradient) -> String {
    let prefix = if lg.repeating {
        "repeating-linear-gradient("
    } else {
        "linear-gradient("
    };
    let mut s = String::from(prefix);
    // Omit default angle (180deg = to bottom)
    if (lg.angle - 180.0).abs() > 0.01 {
        s.push_str(&fmt_f32(lg.angle));
        s.push_str("deg, ");
    }
    serialize_color_stops_normalized(&lg.stops, &mut s);
    s.push(')');
    s
}

fn serialize_radial_gradient(rg: &elidex_plugin::background::RadialGradient) -> String {
    let prefix = if rg.repeating {
        "repeating-radial-gradient("
    } else {
        "radial-gradient("
    };
    let mut s = String::from(prefix);

    // Emit shape/size if explicit radii are set
    let has_radii = rg.radii.0 > 0.0 || rg.radii.1 > 0.0;
    if has_radii {
        let is_circle = (rg.radii.0 - rg.radii.1).abs() < 0.01;
        if is_circle {
            s.push_str("circle ");
            s.push_str(&fmt_f32(rg.radii.0));
            s.push_str("px");
        } else {
            s.push_str(&fmt_f32(rg.radii.0));
            s.push_str("px ");
            s.push_str(&fmt_f32(rg.radii.1));
            s.push_str("px");
        }
    }

    // Emit center if not default 50% 50%
    let non_default_center = (rg.center.0 - 50.0).abs() > 0.01 || (rg.center.1 - 50.0).abs() > 0.01;
    if non_default_center {
        if has_radii {
            s.push(' ');
        }
        s.push_str("at ");
        s.push_str(&fmt_f32(rg.center.0));
        s.push_str("% ");
        s.push_str(&fmt_f32(rg.center.1));
        s.push('%');
    }

    if has_radii || non_default_center {
        s.push_str(", ");
    }

    serialize_color_stops_normalized(&rg.stops, &mut s);
    s.push(')');
    s
}

fn serialize_conic_gradient(cg: &elidex_plugin::background::ConicGradient) -> String {
    let prefix = if cg.repeating {
        "repeating-conic-gradient("
    } else {
        "conic-gradient("
    };
    let mut s = String::from(prefix);
    let non_default_angle = cg.start_angle.abs() > 0.01;
    let non_default_center = (cg.center.0 - 50.0).abs() > 0.01 || (cg.center.1 - 50.0).abs() > 0.01;
    if non_default_angle {
        s.push_str("from ");
        s.push_str(&fmt_f32(cg.start_angle));
        s.push_str("deg");
        if non_default_center {
            s.push(' ');
        } else {
            s.push_str(", ");
        }
    }
    if non_default_center {
        s.push_str("at ");
        s.push_str(&fmt_f32(cg.center.0));
        s.push_str("% ");
        s.push_str(&fmt_f32(cg.center.1));
        s.push_str("%, ");
    }
    serialize_angular_stops(&cg.stops, &mut s);
    s.push(')');
    s
}

/// Serialize linear/radial color stops (positions are 0.0–1.0, emitted as percentages).
fn serialize_color_stops_normalized(
    stops: &[elidex_plugin::background::ColorStop],
    s: &mut String,
) {
    for (i, stop) in stops.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&serialize_color(stop.color));
        s.push(' ');
        s.push_str(&fmt_f32(stop.position * 100.0));
        s.push('%');
    }
}

/// Serialize conic color stops (positions are in degrees).
fn serialize_angular_stops(stops: &[elidex_plugin::background::ColorStop], s: &mut String) {
    for (i, stop) in stops.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&serialize_color(stop.color));
        s.push(' ');
        s.push_str(&fmt_f32(stop.position));
        s.push_str("deg");
    }
}

fn get_computed_bg_position(style: &ComputedStyle) -> CssValue {
    let Some(ref layers) = style.background_layers else {
        return CssValue::Percentage(0.0);
    };
    if layers.len() == 1 {
        return position_to_css(&layers[0].position);
    }
    CssValue::List(
        layers
            .iter()
            .map(|l| position_to_css(&l.position))
            .collect(),
    )
}

fn position_to_css(pos: &BgPosition) -> CssValue {
    use elidex_plugin::background::BgPositionAxis;
    let x = match &pos.x {
        BgPositionAxis::Length(v) => CssValue::Length(*v, LengthUnit::Px),
        BgPositionAxis::Percentage(p) => CssValue::Percentage(*p),
        BgPositionAxis::Edge(_, offset) => CssValue::Length(*offset, LengthUnit::Px),
    };
    let y = match &pos.y {
        BgPositionAxis::Length(v) => CssValue::Length(*v, LengthUnit::Px),
        BgPositionAxis::Percentage(p) => CssValue::Percentage(*p),
        BgPositionAxis::Edge(_, offset) => CssValue::Length(*offset, LengthUnit::Px),
    };
    CssValue::List(vec![x, y])
}

fn get_computed_bg_size(style: &ComputedStyle) -> CssValue {
    let Some(ref layers) = style.background_layers else {
        return CssValue::Auto;
    };
    if layers.len() == 1 {
        return size_to_css(&layers[0].size);
    }
    CssValue::List(layers.iter().map(|l| size_to_css(&l.size)).collect())
}

fn size_to_css(size: &BgSize) -> CssValue {
    match size {
        BgSize::Cover => CssValue::Keyword("cover".to_string()),
        BgSize::Contain => CssValue::Keyword("contain".to_string()),
        BgSize::Explicit(w, h) => {
            use elidex_plugin::background::BgSizeDimension;
            let w_val = match w {
                Some(BgSizeDimension::Length(v)) => CssValue::Length(*v, LengthUnit::Px),
                Some(BgSizeDimension::Percentage(p)) => CssValue::Percentage(*p),
                None => CssValue::Auto,
            };
            let h_val = match h {
                Some(BgSizeDimension::Length(v)) => CssValue::Length(*v, LengthUnit::Px),
                Some(BgSizeDimension::Percentage(p)) => CssValue::Percentage(*p),
                None => CssValue::Auto,
            };
            CssValue::List(vec![w_val, h_val])
        }
    }
}

fn get_computed_bg_repeat(style: &ComputedStyle) -> CssValue {
    let Some(ref layers) = style.background_layers else {
        return CssValue::Keyword("repeat".to_string());
    };
    if layers.len() == 1 {
        return repeat_to_css(&layers[0].repeat);
    }
    CssValue::List(layers.iter().map(|l| repeat_to_css(&l.repeat)).collect())
}

fn repeat_to_css(r: &BgRepeat) -> CssValue {
    let x = repeat_axis_str(r.x);
    let y = repeat_axis_str(r.y);
    if x == y {
        CssValue::Keyword(x.to_string())
    } else {
        CssValue::List(vec![
            CssValue::Keyword(x.to_string()),
            CssValue::Keyword(y.to_string()),
        ])
    }
}

fn repeat_axis_str(axis: BgRepeatAxis) -> &'static str {
    match axis {
        BgRepeatAxis::Repeat => "repeat",
        BgRepeatAxis::NoRepeat => "no-repeat",
        BgRepeatAxis::Space => "space",
        BgRepeatAxis::Round => "round",
    }
}

fn box_area_str(area: BoxArea) -> &'static str {
    match area {
        BoxArea::BorderBox => "border-box",
        BoxArea::PaddingBox => "padding-box",
        BoxArea::ContentBox => "content-box",
    }
}

fn get_computed_bg_origin(style: &ComputedStyle) -> CssValue {
    let Some(ref layers) = style.background_layers else {
        return CssValue::Keyword("padding-box".to_string());
    };
    if layers.len() == 1 {
        return CssValue::Keyword(box_area_str(layers[0].origin).to_string());
    }
    CssValue::List(
        layers
            .iter()
            .map(|l| CssValue::Keyword(box_area_str(l.origin).to_string()))
            .collect(),
    )
}

fn get_computed_bg_clip(style: &ComputedStyle) -> CssValue {
    let Some(ref layers) = style.background_layers else {
        return CssValue::Keyword("border-box".to_string());
    };
    if layers.len() == 1 {
        return CssValue::Keyword(box_area_str(layers[0].clip).to_string());
    }
    CssValue::List(
        layers
            .iter()
            .map(|l| CssValue::Keyword(box_area_str(l.clip).to_string()))
            .collect(),
    )
}

fn attachment_str(a: BgAttachment) -> &'static str {
    match a {
        BgAttachment::Scroll => "scroll",
        BgAttachment::Fixed => "fixed",
        BgAttachment::Local => "local",
    }
}

fn get_computed_bg_attachment(style: &ComputedStyle) -> CssValue {
    let Some(ref layers) = style.background_layers else {
        return CssValue::Keyword("scroll".to_string());
    };
    if layers.len() == 1 {
        return CssValue::Keyword(attachment_str(layers[0].attachment).to_string());
    }
    CssValue::List(
        layers
            .iter()
            .map(|l| CssValue::Keyword(attachment_str(l.attachment).to_string()))
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// CssValue → typed conversion helpers (used by resolve_background_layers)
// ---------------------------------------------------------------------------

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
                    radii: (0.0, 0.0), // Resolved against painting area at render time
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

    let mut resolved: Vec<(Option<f32>, elidex_plugin::CssColor)> = stops
        .iter()
        .map(|s| {
            let color = match &s.color {
                CssValue::Color(c) => *c,
                _ => elidex_plugin::CssColor::TRANSPARENT,
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
    let mut resolved: Vec<(Option<f32>, elidex_plugin::CssColor)> = stops
        .iter()
        .map(|s| {
            let color = match &s.color {
                CssValue::Color(c) => *c,
                _ => elidex_plugin::CssColor::TRANSPARENT,
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

/// Resolve position pair from parse-stage `CssValue` list to (f32, f32) percentages.
fn resolve_position_pair(position: Option<&Vec<CssValue>>) -> (f32, f32) {
    let Some(pos) = position else {
        return (50.0, 50.0); // center center
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
    (x, y)
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
            let x = css_to_position_axis(&items[0], true);
            let y = css_to_position_axis(&items[1], false);
            BgPosition { x, y }
        }
        _ => BgPosition::default(),
    }
}

fn css_to_position_axis(
    value: &CssValue,
    _is_x: bool,
) -> elidex_plugin::background::BgPositionAxis {
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
