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
mod parse;
mod position;
pub mod resolve;
mod shorthand;

#[cfg(test)]
mod tests;

// Re-export resolve functions at crate root for backward compatibility.
pub use resolve::{
    resolve_bg_attachment, resolve_bg_image, resolve_bg_position, resolve_bg_repeat,
    resolve_bg_size, resolve_box_area_keyword,
};

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

            "background-image" => parse::parse_bg_image(input)?,

            "background-repeat" => parse::parse_bg_repeat(input)?,

            "background-origin" | "background-clip" => parse::parse_box_keyword(input)?,

            "background-attachment" => parse::parse_attachment(input)?,

            "background-position" => return position::parse_bg_position_declaration(input),

            "background-size" => parse::parse_bg_size(input)?,

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
    let has_radii = rg.radii.width > 0.0 || rg.radii.height > 0.0;
    if has_radii {
        let is_circle = (rg.radii.width - rg.radii.height).abs() < 0.01;
        if is_circle {
            s.push_str("circle ");
            s.push_str(&fmt_f32(rg.radii.width));
            s.push_str("px");
        } else {
            s.push_str(&fmt_f32(rg.radii.width));
            s.push_str("px ");
            s.push_str(&fmt_f32(rg.radii.height));
            s.push_str("px");
        }
    }

    // Emit center if not default 50% 50%
    let non_default_center = (rg.center.x - 50.0).abs() > 0.01 || (rg.center.y - 50.0).abs() > 0.01;
    if non_default_center {
        if has_radii {
            s.push(' ');
        }
        s.push_str("at ");
        s.push_str(&fmt_f32(rg.center.x));
        s.push_str("% ");
        s.push_str(&fmt_f32(rg.center.y));
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
    let non_default_center = (cg.center.x - 50.0).abs() > 0.01 || (cg.center.y - 50.0).abs() > 0.01;
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
        s.push_str(&fmt_f32(cg.center.x));
        s.push_str("% ");
        s.push_str(&fmt_f32(cg.center.y));
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
