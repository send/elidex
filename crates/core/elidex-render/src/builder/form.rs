//! Form control rendering: text inputs, passwords, checkboxes, radios, buttons,
//! textareas, selects.

use elidex_form::{FormControlKind, FormControlState};
use elidex_plugin::{ComputedStyle, CssColor, LayoutBox, Rect};
use elidex_text::{shape_text, to_fontdb_style, FontDatabase};

use crate::display_list::{DisplayItem, DisplayList};
use crate::font_cache::FontCache;

use super::paint::apply_opacity;
use super::{families_as_refs, place_glyphs};

/// Shared font resources for form control rendering.
pub(crate) struct FontEnv<'a> {
    pub(crate) db: &'a FontDatabase,
    pub(crate) cache: &'a mut FontCache,
}

/// Return scroll offset clamped to a finite, non-negative value.
fn safe_scroll_offset(fcs: &FormControlState) -> f32 {
    let v = fcs.scroll_offset_x;
    if v.is_finite() {
        v.max(0.0)
    } else {
        0.0
    }
}

/// Compute the text rendering origin for a form control.
///
/// Returns `(scroll-adjusted x, baseline y)` as a `Point`.
fn text_origin(content: &Rect, fcs: &FormControlState, ascent: f32) -> elidex_plugin::Point {
    elidex_plugin::Point::new(
        content.origin.x - safe_scroll_offset(fcs),
        content.origin.y + ascent,
    )
}

/// Build a vertical-spanning rect within `content` (for caret / selection).
///
/// The rect spans the full height of `content` at position `x` with the given `width`.
fn content_vspan_rect(content: &Rect, x: f32, width: f32) -> Rect {
    Rect::new(x, content.origin.y, width, content.size.height)
}

/// Placeholder text color (grey).
const PLACEHOLDER_COLOR: CssColor = CssColor {
    r: 169,
    g: 169,
    b: 169,
    a: 255,
};

/// Caret width in pixels.
const CARET_WIDTH: f32 = 1.0;

/// Checkbox/radio checkmark fill color.
const CHECKMARK_COLOR: CssColor = CssColor {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

/// Selection highlight color (semi-transparent blue).
const SELECTION_COLOR: CssColor = CssColor {
    r: 51,
    g: 144,
    b: 255,
    a: 100,
};

/// Password mask character.
const PASSWORD_MASK_STR: &str = "\u{2022}";
/// Maximum mask characters to render (visual display is clipped to content box).
const MAX_MASK_RENDER_CHARS: usize = 2000;

/// Checkbox check indicator inset factor.
const CHECKBOX_INSET_FACTOR: f32 = 0.25;

/// Radio check indicator inset factor.
const RADIO_INSET_FACTOR: f32 = 0.3;

/// Dropdown arrow indicator string.
const DROPDOWN_ARROW: &str = "\u{25BE}";

/// Right-edge offset for dropdown arrow (CSS pixels).
const DROPDOWN_ARROW_OFFSET: f32 = 14.0;

/// Emit display items for a form control element.
pub(crate) fn emit_form_control(
    lb: &LayoutBox,
    fcs: &FormControlState,
    style: &ComputedStyle,
    fonts: &mut FontEnv<'_>,
    dl: &mut DisplayList,
    is_focused: bool,
    caret_visible: bool,
) {
    match fcs.kind {
        FormControlKind::TextInput
        | FormControlKind::TextArea
        | FormControlKind::Email
        | FormControlKind::Url
        | FormControlKind::Tel
        | FormControlKind::Search
        | FormControlKind::Number => {
            emit_text_input(lb, fcs, style, fonts, dl, is_focused, caret_visible);
        }
        FormControlKind::Password => {
            emit_password(lb, fcs, style, fonts, dl, is_focused, caret_visible);
        }
        FormControlKind::Checkbox => {
            emit_check_indicator(lb, fcs, style, dl, CHECKBOX_INSET_FACTOR);
        }
        FormControlKind::Radio => {
            emit_check_indicator(lb, fcs, style, dl, RADIO_INSET_FACTOR);
        }
        FormControlKind::SubmitButton
        | FormControlKind::ResetButton
        | FormControlKind::Button
        | FormControlKind::Range
        | FormControlKind::Color
        | FormControlKind::Date
        | FormControlKind::DatetimeLocal
        | FormControlKind::File => {
            emit_button(lb, fcs, style, fonts, dl);
        }
        FormControlKind::Select => {
            emit_select(lb, fcs, style, fonts, dl);
        }
        // Hidden/Output/Meter/Progress: no visual representation yet.
        FormControlKind::Hidden
        | FormControlKind::Output
        | FormControlKind::Meter
        | FormControlKind::Progress => {}
    }
}

/// Query font and return (`font_id`, ascent).
fn query_font(style: &ComputedStyle, font_db: &FontDatabase) -> Option<(fontdb::ID, f32)> {
    let families = families_as_refs(&style.font_family);
    let font_style = to_fontdb_style(style.font_style);
    let font_id = font_db.query(&families, style.font_weight, font_style)?;
    let ascent = font_db
        .font_metrics(font_id, style.font_size)
        .map_or(style.font_size * 0.8, |m| m.ascent);
    Some((font_id, ascent))
}

/// Line height multiplier for textarea line spacing.
const TEXTAREA_LINE_HEIGHT_FACTOR: f32 = 1.2;

/// Compute the total advance width of shaped glyphs.
fn text_width(glyphs: &[elidex_text::ShapedGlyph]) -> f32 {
    glyphs.iter().map(|g| g.x_advance).sum()
}

/// Shape `text` and return `base_x + advance_width`, or `base_x` if shaping fails or text is empty.
fn shaped_text_x_offset(
    text: &str,
    base_x: f32,
    font_db: &FontDatabase,
    font_id: fontdb::ID,
    font_size: f32,
) -> f32 {
    if text.is_empty() {
        return base_x;
    }
    if let Some(shaped) = shape_text(font_db, font_id, font_size, text) {
        base_x + text_width(&shaped.glyphs)
    } else {
        base_x
    }
}

/// Emit placeholder text if the value is empty and a placeholder exists.
///
/// Returns `true` if placeholder was rendered (caller should skip value rendering).
fn emit_placeholder(
    content: &Rect,
    fcs: &FormControlState,
    style: &ComputedStyle,
    fonts: &mut FontEnv<'_>,
    font_id: fontdb::ID,
    dl: &mut DisplayList,
    ascent: f32,
) -> bool {
    if !fcs.value().is_empty() || fcs.placeholder.is_empty() {
        return false;
    }
    // For text inputs, also skip if composition is active.
    if fcs.composition_text.is_some() {
        return false;
    }
    let color = apply_opacity(PLACEHOLDER_COLOR, style.opacity);
    let origin = text_origin(content, fcs, ascent);
    emit_shaped_text(&fcs.placeholder, origin, style, fonts, font_id, dl, color);
    true
}

/// Emit text input or textarea.
fn emit_text_input(
    lb: &LayoutBox,
    fcs: &FormControlState,
    style: &ComputedStyle,
    fonts: &mut FontEnv<'_>,
    dl: &mut DisplayList,
    is_focused: bool,
    caret_visible: bool,
) {
    let content = &lb.content;
    let Some((font_id, ascent)) = query_font(style, fonts.db) else {
        return;
    };

    // Placeholder takes priority when value is empty and no composition active.
    if emit_placeholder(content, fcs, style, fonts, font_id, dl, ascent) {
        return;
    }

    {
        let color = apply_opacity(style.color, style.opacity);

        // Build display text: splice composition text at cursor position if present (L-11).
        let display_text: String;
        let comp_range: Option<(usize, usize)>;
        if let Some(ref comp) = fcs.composition_text {
            let pos = fcs.safe_cursor_pos();
            let mut buf = fcs.value().to_string();
            buf.insert_str(pos, comp);
            comp_range = Some((pos, pos + comp.len()));
            display_text = buf;
        } else {
            display_text = fcs.value().to_string();
            comp_range = None;
        }

        // Emit selection highlight.
        if fcs.selection_start() != fcs.selection_end() {
            emit_selection_highlight(lb, fcs, style, fonts.db, font_id, dl);
        }

        // Textarea: split on newlines and render each line (M-11).
        if fcs.kind == FormControlKind::TextArea {
            let line_height = style.font_size * TEXTAREA_LINE_HEIGHT_FACTOR;
            let origin = text_origin(content, fcs, ascent);
            for (i, line) in display_text.split('\n').enumerate() {
                #[allow(clippy::cast_precision_loss)]
                let baseline_y = origin.y + (i as f32) * line_height;
                if baseline_y > content.bottom() {
                    break; // Clip lines outside content box.
                }
                if !line.is_empty() {
                    emit_shaped_text(
                        line,
                        elidex_plugin::Point::new(origin.x, baseline_y),
                        style,
                        fonts,
                        font_id,
                        dl,
                        color,
                    );
                }
            }
        } else {
            // Single-line text input.
            let origin = text_origin(content, fcs, ascent);
            if !display_text.is_empty() {
                emit_shaped_text(&display_text, origin, style, fonts, font_id, dl, color);
            }
        }

        // Composition underline (rendered over the spliced composition range).
        if let Some((start, end)) = comp_range {
            let comp_text = &display_text[start..end];
            if !comp_text.is_empty() {
                let comp_origin = text_origin(content, fcs, ascent);
                // Compute x offset for composition start.
                let comp_x = if start == 0 {
                    comp_origin.x
                } else {
                    shaped_text_x_offset(
                        &display_text[..start],
                        comp_origin.x,
                        fonts.db,
                        font_id,
                        style.font_size,
                    )
                };
                if let Some(shaped) = shape_text(fonts.db, font_id, style.font_size, comp_text) {
                    let w: f32 = text_width(&shaped.glyphs);
                    let underline_y = comp_origin.y + 2.0;
                    dl.push(DisplayItem::SolidRect {
                        rect: Rect::new(comp_x, underline_y, w, 1.0),
                        color,
                    });
                }
            }
        }
    }

    if is_focused && caret_visible {
        emit_caret(lb, fcs, style, fonts.db, font_id, dl);
    }
}

/// Emit password field (masked characters).
fn emit_password(
    lb: &LayoutBox,
    fcs: &FormControlState,
    style: &ComputedStyle,
    fonts: &mut FontEnv<'_>,
    dl: &mut DisplayList,
    is_focused: bool,
    caret_visible: bool,
) {
    let content = &lb.content;
    let Some((font_id, ascent)) = query_font(style, fonts.db) else {
        return;
    };

    // Placeholder rendering (shared with text input).
    if emit_placeholder(content, fcs, style, fonts, font_id, dl, ascent) {
        return;
    }

    // Shape a single mask character to get its advance width.
    let single_advance = shape_text(fonts.db, font_id, style.font_size, PASSWORD_MASK_STR)
        .map_or(style.font_size * 0.6, |s| text_width(&s.glyphs));

    if !fcs.value().is_empty() {
        // Cap rendered mask characters — visual display is clipped to content box anyway.
        let total_chars = fcs.char_count().min(MAX_MASK_RENDER_CHARS);
        #[allow(clippy::cast_precision_loss)]
        let mask_width = single_advance * total_chars as f32;
        let color = apply_opacity(style.color, style.opacity);
        // Draw a sequence of mask glyphs using repeated single-char shaping.
        let mask: String = PASSWORD_MASK_STR.repeat(total_chars);
        let origin = text_origin(content, fcs, ascent);
        emit_shaped_text(&mask, origin, style, fonts, font_id, dl, color);
        let _ = mask_width; // width available for future clipping
    }

    if is_focused && caret_visible {
        let caret_pos = fcs.safe_cursor_pos();
        let char_count = fcs.value()[..caret_pos].chars().count();
        #[allow(clippy::cast_precision_loss)]
        let caret_x = content.origin.x + single_advance * char_count as f32;
        let caret_color = apply_opacity(style.color, style.opacity);
        dl.push(DisplayItem::SolidRect {
            rect: content_vspan_rect(content, caret_x, CARET_WIDTH),
            color: caret_color,
        });
    }
}

/// Emit a check indicator (checkbox or radio): filled inner rect when checked.
///
/// `inset_factor` controls the inset size relative to the content width
/// (0.25 for checkbox, 0.3 for radio).
fn emit_check_indicator(
    lb: &LayoutBox,
    fcs: &FormControlState,
    style: &ComputedStyle,
    dl: &mut DisplayList,
    inset_factor: f32,
) {
    if fcs.checked {
        let inset = (lb.content.size.width * inset_factor).max(2.0);
        let inner = lb.content.inset(inset);
        dl.push(DisplayItem::SolidRect {
            rect: inner,
            color: apply_opacity(CHECKMARK_COLOR, style.opacity),
        });
    }
}

/// Emit a button: centered label text.
fn emit_button(
    lb: &LayoutBox,
    fcs: &FormControlState,
    style: &ComputedStyle,
    fonts: &mut FontEnv<'_>,
    dl: &mut DisplayList,
) {
    let label = fcs.value();
    if label.is_empty() {
        return;
    }

    let Some((font_id, ascent)) = query_font(style, fonts.db) else {
        return;
    };

    let Some(shaped) = shape_text(fonts.db, font_id, style.font_size, label) else {
        return;
    };
    let Some((font_blob, font_index)) = fonts.cache.get(fonts.db, font_id) else {
        return;
    };

    let text_width: f32 = text_width(&shaped.glyphs);

    let mut text_x = lb.content.origin.x + (lb.content.size.width - text_width) / 2.0;
    let baseline_y = lb.content.origin.y + f32::midpoint(lb.content.size.height, ascent);
    let color = apply_opacity(style.color, style.opacity);

    let glyphs = place_glyphs(&shaped.glyphs, &mut text_x, baseline_y, 0.0, 0.0, label);
    dl.push(DisplayItem::Text {
        glyphs,
        font_blob,
        font_index,
        font_size: style.font_size,
        color,
    });
}

/// Emit a select element: selected option text + dropdown arrow.
fn emit_select(
    lb: &LayoutBox,
    fcs: &FormControlState,
    style: &ComputedStyle,
    fonts: &mut FontEnv<'_>,
    dl: &mut DisplayList,
) {
    let Some((font_id, ascent)) = query_font(style, fonts.db) else {
        return;
    };
    let content = &lb.content;
    let baseline_y = content.origin.y + ascent;
    let color = apply_opacity(style.color, style.opacity);

    // Display selected option text.
    #[allow(clippy::cast_sign_loss)]
    let selected_text = if fcs.selected_index >= 0 {
        fcs.options
            .get(fcs.selected_index as usize)
            .map_or("", |o| o.text.as_str())
    } else {
        ""
    };

    if !selected_text.is_empty() {
        emit_shaped_text(
            selected_text,
            elidex_plugin::Point::new(content.origin.x, baseline_y),
            style,
            fonts,
            font_id,
            dl,
            color,
        );
    }

    // Dropdown arrow at the right edge.
    let arrow_x = content.right() - DROPDOWN_ARROW_OFFSET;
    emit_shaped_text(
        DROPDOWN_ARROW,
        elidex_plugin::Point::new(arrow_x, baseline_y),
        style,
        fonts,
        font_id,
        dl,
        color,
    );
}

/// Emit shaped text at a position.
fn emit_shaped_text(
    text: &str,
    pos: elidex_plugin::Point,
    style: &ComputedStyle,
    fonts: &mut FontEnv<'_>,
    font_id: fontdb::ID,
    dl: &mut DisplayList,
    color: CssColor,
) {
    let Some(shaped) = shape_text(fonts.db, font_id, style.font_size, text) else {
        return;
    };
    let Some((font_blob, font_index)) = fonts.cache.get(fonts.db, font_id) else {
        return;
    };
    let mut text_x = pos.x;
    let glyphs = place_glyphs(&shaped.glyphs, &mut text_x, pos.y, 0.0, 0.0, text);
    dl.push(DisplayItem::Text {
        glyphs,
        font_blob,
        font_index,
        font_size: style.font_size,
        color,
    });
}

/// Emit caret at the cursor position.
fn emit_caret(
    lb: &LayoutBox,
    fcs: &FormControlState,
    style: &ComputedStyle,
    font_db: &FontDatabase,
    font_id: fontdb::ID,
    dl: &mut DisplayList,
) {
    let content = &lb.content;
    let caret_pos = fcs.safe_cursor_pos();
    let origin = text_origin(content, fcs, 0.0); // ascent not needed for x
    let caret_x = shaped_text_x_offset(
        &fcs.value()[..caret_pos],
        origin.x,
        font_db,
        font_id,
        style.font_size,
    );
    let caret_color = apply_opacity(style.color, style.opacity);
    dl.push(DisplayItem::SolidRect {
        rect: content_vspan_rect(content, caret_x, CARET_WIDTH),
        color: caret_color,
    });
}

/// Emit selection highlight rectangles.
fn emit_selection_highlight(
    lb: &LayoutBox,
    fcs: &FormControlState,
    style: &ComputedStyle,
    font_db: &FontDatabase,
    font_id: fontdb::ID,
    dl: &mut DisplayList,
) {
    let content = &lb.content;
    let (sel_start, sel_end) = fcs.safe_selection_range();
    let origin = text_origin(content, fcs, 0.0);

    let start_x = shaped_text_x_offset(
        &fcs.value()[..sel_start],
        origin.x,
        font_db,
        font_id,
        style.font_size,
    );
    let end_x = shaped_text_x_offset(
        &fcs.value()[..sel_end],
        origin.x,
        font_db,
        font_id,
        style.font_size,
    );

    let width = (end_x - start_x).max(0.0);
    if width > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: content_vspan_rect(content, start_x, width),
            color: apply_opacity(SELECTION_COLOR, style.opacity),
        });
    }
}
