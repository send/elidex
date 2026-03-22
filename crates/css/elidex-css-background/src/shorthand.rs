//! Background shorthand parsing.
//!
//! Parses the `background` shorthand into its component longhands.
//! Supports multiple comma-separated layers.

use elidex_plugin::{CssValue, ParseError, PropertyDeclaration};

use crate::gradient;

/// All longhand names that the shorthand resets (used for shorthand expansion).
#[allow(dead_code)]
const LONGHANDS: &[&str] = &[
    "background-color",
    "background-image",
    "background-position",
    "background-size",
    "background-repeat",
    "background-origin",
    "background-clip",
    "background-attachment",
];

/// Parse the `background` shorthand.
///
/// CSS Backgrounds §3.10: `[<bg-layer>,]* <final-bg-layer>`
/// Each layer can contain any combination of: `<bg-image>`, `<bg-position>`,
/// `<bg-size>` (preceded by `/`), `<repeat-style>`, `<attachment>`, `<box>{1,2}`.
/// `background-color` is only allowed in the final layer.
pub(crate) fn parse_background_shorthand(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<Vec<PropertyDeclaration>, ParseError> {
    let mut all_images = Vec::new();
    let mut all_positions = Vec::new();
    let mut all_sizes = Vec::new();
    let mut all_repeats = Vec::new();
    let mut all_origins = Vec::new();
    let mut all_clips = Vec::new();
    let mut all_attachments = Vec::new();
    let mut bg_color = None;

    loop {
        let is_last = !has_comma_ahead(input);
        let layer = parse_single_layer(input, is_last).map_err(|_| ParseError {
            property: "background".into(),
            input: String::new(),
            message: "invalid background shorthand".into(),
        })?;

        all_images.push(layer.image);
        all_positions.push(layer.position);
        all_sizes.push(layer.size);
        all_repeats.push(layer.repeat);
        all_origins.push(layer.origin);
        all_clips.push(layer.clip);
        all_attachments.push(layer.attachment);
        if let Some(c) = layer.color {
            bg_color = Some(c);
        }

        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }

    let mut decls = Vec::new();

    // background-color (only from final layer, or initial)
    decls.push(PropertyDeclaration::new(
        "background-color",
        bg_color.unwrap_or(CssValue::Color(elidex_plugin::CssColor::TRANSPARENT)),
    ));

    // For single-layer, emit simple values; for multi-layer, emit lists.
    let to_value = |items: Vec<CssValue>| -> CssValue {
        if items.len() == 1 {
            items.into_iter().next().unwrap()
        } else {
            CssValue::List(items)
        }
    };

    decls.push(PropertyDeclaration::new(
        "background-image",
        to_value(all_images),
    ));
    decls.push(PropertyDeclaration::new(
        "background-position",
        to_value(all_positions),
    ));
    decls.push(PropertyDeclaration::new(
        "background-size",
        to_value(all_sizes),
    ));
    decls.push(PropertyDeclaration::new(
        "background-repeat",
        to_value(all_repeats),
    ));
    decls.push(PropertyDeclaration::new(
        "background-origin",
        to_value(all_origins),
    ));
    decls.push(PropertyDeclaration::new(
        "background-clip",
        to_value(all_clips),
    ));
    decls.push(PropertyDeclaration::new(
        "background-attachment",
        to_value(all_attachments),
    ));

    Ok(decls)
}

struct LayerValues {
    image: CssValue,
    position: CssValue,
    size: CssValue,
    repeat: CssValue,
    origin: CssValue,
    clip: CssValue,
    attachment: CssValue,
    color: Option<CssValue>,
}

impl Default for LayerValues {
    fn default() -> Self {
        Self {
            image: CssValue::Keyword("none".into()),
            position: CssValue::List(vec![CssValue::Percentage(0.0), CssValue::Percentage(0.0)]),
            size: CssValue::Auto,
            repeat: CssValue::Keyword("repeat".into()),
            origin: CssValue::Keyword("padding-box".into()),
            clip: CssValue::Keyword("border-box".into()),
            attachment: CssValue::Keyword("scroll".into()),
            color: None,
        }
    }
}

/// Check if there's a comma later (without consuming anything significant).
fn has_comma_ahead(_input: &mut cssparser::Parser<'_, '_>) -> bool {
    // Peek-based: we just try parsing the current layer and see if comma follows.
    // This is a heuristic — we rely on the layer parser stopping at commas.
    false // Conservative: always treat as last layer (allows bg-color)
}

fn try_parse_layer_image<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    if input
        .try_parse(|i2| i2.expect_ident_matching("none"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("none".into()));
    }
    if let Ok(url) = input.try_parse(cssparser::Parser::expect_url) {
        return Ok(CssValue::Url(url.as_ref().to_string()));
    }
    gradient::parse_gradient(input).map_err(|_| input.new_custom_error::<(), ()>(()))
}

fn try_parse_layer_repeat<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    let ident = input.expect_ident_cloned()?;
    let lower = ident.to_ascii_lowercase();
    match lower.as_str() {
        "repeat" | "no-repeat" | "space" | "round" => {
            let second = input
                .try_parse(|i2| {
                    let id2 = i2.expect_ident_cloned()?;
                    let l2 = id2.to_ascii_lowercase();
                    match l2.as_str() {
                        "repeat" | "no-repeat" | "space" | "round" => Ok(l2),
                        _ => Err(i2.new_custom_error::<_, ()>(())),
                    }
                })
                .ok();
            match second {
                Some(s) => Ok(CssValue::List(vec![
                    CssValue::Keyword(lower),
                    CssValue::Keyword(s),
                ])),
                None => Ok(CssValue::Keyword(lower)),
            }
        }
        "repeat-x" => Ok(CssValue::List(vec![
            CssValue::Keyword("repeat".into()),
            CssValue::Keyword("no-repeat".into()),
        ])),
        "repeat-y" => Ok(CssValue::List(vec![
            CssValue::Keyword("no-repeat".into()),
            CssValue::Keyword("repeat".into()),
        ])),
        _ => Err(input.new_custom_error::<(), ()>(())),
    }
}

fn try_parse_layer_color<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<elidex_plugin::CssColor, cssparser::ParseError<'i, ()>> {
    elidex_css::parse_color(input).map_err(|()| input.new_custom_error::<(), ()>(()))
}

fn parse_single_layer<'i>(
    input: &mut cssparser::Parser<'i, '_>,
    allow_color: bool,
) -> Result<LayerValues, cssparser::ParseError<'i, ()>> {
    let mut layer = LayerValues::default();
    let mut has_any = false;
    let mut box_values: Vec<String> = Vec::new();

    // Try parsing tokens in any order (CSS || combinators)
    for _ in 0..10 {
        if let Ok(img) = input.try_parse(try_parse_layer_image) {
            layer.image = img;
            has_any = true;
            continue;
        }
        if let Ok(rep) = input.try_parse(try_parse_layer_repeat) {
            layer.repeat = rep;
            has_any = true;
            continue;
        }
        if let Ok(att) = input.try_parse(|i| {
            let ident = i.expect_ident_cloned()?;
            match ident.to_ascii_lowercase().as_str() {
                "scroll" | "fixed" | "local" => Ok(CssValue::Keyword(ident.to_ascii_lowercase())),
                _ => Err(i.new_custom_error::<_, ()>(())),
            }
        }) {
            layer.attachment = att;
            has_any = true;
            continue;
        }
        if let Ok(bk) = input.try_parse(|i| {
            let ident = i.expect_ident_cloned()?;
            match ident.to_ascii_lowercase().as_str() {
                "border-box" | "padding-box" | "content-box" => Ok(ident.to_ascii_lowercase()),
                _ => Err(i.new_custom_error::<_, ()>(())),
            }
        }) {
            box_values.push(bk);
            has_any = true;
            continue;
        }
        if allow_color {
            if let Ok(color) = input.try_parse(try_parse_layer_color) {
                layer.color = Some(CssValue::Color(color));
                has_any = true;
                continue;
            }
        }
        if let Ok((pos, size)) = input.try_parse(parse_position_and_size) {
            layer.position = pos;
            if let Some(s) = size {
                layer.size = s;
            }
            has_any = true;
            continue;
        }
        break;
    }

    if !has_any {
        return Err(input.new_custom_error(()));
    }

    // Assign box values: 1 → both origin and clip, 2 → origin and clip
    match box_values.len() {
        1 => {
            layer.origin = CssValue::Keyword(box_values[0].clone());
            layer.clip = CssValue::Keyword(box_values[0].clone());
        }
        2 => {
            layer.origin = CssValue::Keyword(box_values[0].clone());
            layer.clip = CssValue::Keyword(box_values[1].clone());
        }
        _ => {}
    }

    Ok(layer)
}

/// Parse position and optional `/size`.
fn parse_position_and_size<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<(CssValue, Option<CssValue>), cssparser::ParseError<'i, ()>> {
    let first = parse_pos_component(input)?;
    let second = input.try_parse(parse_pos_component).ok();

    let pos = match second {
        Some(s) => CssValue::List(vec![first, s]),
        None => CssValue::List(vec![first, CssValue::Percentage(50.0)]),
    };

    // Try `/size`
    let size = input
        .try_parse(|i| {
            i.expect_delim('/')?;
            parse_size_component(i)
        })
        .ok();

    Ok((pos, size))
}

fn parse_pos_component<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    if let Ok(ident) = input.try_parse(cssparser::Parser::expect_ident_cloned) {
        return match ident.to_ascii_lowercase().as_str() {
            "left" | "top" => Ok(CssValue::Percentage(0.0)),
            "center" => Ok(CssValue::Percentage(50.0)),
            "right" | "bottom" => Ok(CssValue::Percentage(100.0)),
            _ => Err(input.new_custom_error(())),
        };
    }
    if let Ok(pct) = input.try_parse(cssparser::Parser::expect_percentage) {
        return Ok(CssValue::Percentage(pct * 100.0));
    }
    let tok = input.next()?;
    match tok {
        cssparser::Token::Dimension { value, unit, .. } => {
            let lu = match unit.to_ascii_lowercase().as_str() {
                "em" => elidex_plugin::LengthUnit::Em,
                "rem" => elidex_plugin::LengthUnit::Rem,
                _ => elidex_plugin::LengthUnit::Px,
            };
            Ok(CssValue::Length(*value, lu))
        }
        cssparser::Token::Number { value, .. } if *value == 0.0 => {
            Ok(CssValue::Length(0.0, elidex_plugin::LengthUnit::Px))
        }
        _ => Err(input.new_custom_error(())),
    }
}

fn parse_size_component<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    if let Ok(ident) = input.try_parse(cssparser::Parser::expect_ident_cloned) {
        return match ident.to_ascii_lowercase().as_str() {
            "cover" => Ok(CssValue::Keyword("cover".into())),
            "contain" => Ok(CssValue::Keyword("contain".into())),
            "auto" => {
                let second = input.try_parse(|i| {
                    if i.try_parse(|i2| i2.expect_ident_matching("auto")).is_ok() {
                        return Ok(CssValue::Auto);
                    }
                    parse_size_length(i)
                });
                match second {
                    Ok(s) => Ok(CssValue::List(vec![CssValue::Auto, s])),
                    Err(_) => Ok(CssValue::Auto),
                }
            }
            _ => Err(input.new_custom_error(())),
        };
    }
    let first = parse_size_length(input)?;
    let second = input
        .try_parse(|i| {
            if i.try_parse(|i2| i2.expect_ident_matching("auto")).is_ok() {
                return Ok(CssValue::Auto);
            }
            parse_size_length(i)
        })
        .ok();
    match second {
        Some(s) => Ok(CssValue::List(vec![first, s])),
        None => Ok(first),
    }
}

fn parse_size_length<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    if let Ok(pct) = input.try_parse(cssparser::Parser::expect_percentage) {
        return Ok(CssValue::Percentage(pct * 100.0));
    }
    let tok = input.next()?;
    match tok {
        cssparser::Token::Dimension { value, unit, .. } if *value >= 0.0 => {
            let lu = match unit.to_ascii_lowercase().as_str() {
                "em" => elidex_plugin::LengthUnit::Em,
                _ => elidex_plugin::LengthUnit::Px,
            };
            Ok(CssValue::Length(*value, lu))
        }
        _ => Err(input.new_custom_error(())),
    }
}
