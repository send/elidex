//! Axis constraint solvers for absolutely positioned elements.
//!
//! CSS 2.1 §10.3.7 / §10.6.4 constraint equations, generalized for
//! writing modes per CSS Writing Modes L3 §4.3.

/// Physical properties mapped to the inline axis.
pub(crate) struct InlineAxisProps {
    /// Resolved inline-start offset (physical left or top depending on WM).
    pub start: Option<f32>,
    /// Resolved inline-end offset (physical right or bottom depending on WM).
    pub end: Option<f32>,
    /// Specified inline-size (physical width or height depending on WM).
    pub size: Option<f32>,
    /// Raw margin at inline-start side.
    pub margin_start_raw: f32,
    /// Raw margin at inline-end side.
    pub margin_end_raw: f32,
    /// Whether inline-start margin is auto.
    pub margin_start_auto: bool,
    /// Whether inline-end margin is auto.
    pub margin_end_auto: bool,
    /// Inline-axis padding + border.
    pub pb: f32,
    /// Containing block inline size.
    pub containing: f32,
    /// Static position on inline axis (relative to CB origin).
    pub static_offset: f32,
}

/// Physical properties mapped to the block axis.
pub(crate) struct BlockAxisProps {
    /// Resolved block-start offset.
    pub start: Option<f32>,
    /// Resolved block-end offset.
    pub end: Option<f32>,
    /// Specified block-size (`None` if auto).
    pub size: Option<f32>,
    /// Content size from layout (used when size is auto).
    pub content_size: Option<f32>,
    /// Raw margin at block-start side.
    pub margin_start_raw: f32,
    /// Raw margin at block-end side.
    pub margin_end_raw: f32,
    /// Whether block-start margin is auto.
    pub margin_start_auto: bool,
    /// Whether block-end margin is auto.
    pub margin_end_auto: bool,
    /// Block-axis padding + border.
    pub pb: f32,
    /// Containing block block size.
    pub containing: f32,
    /// Static position on block axis (relative to CB origin).
    pub static_offset: f32,
}

/// Result of axis constraint resolution, in logical terms.
pub(crate) struct AxisResult {
    /// Resolved content size along this axis.
    pub size: f32,
    /// Margin at the start side.
    pub margin_start: f32,
    /// Margin at the end side.
    pub margin_end: f32,
    /// Offset from CB edge to margin edge (start side).
    pub offset: f32,
}

/// Resolve the inline-axis constraint equation.
///
/// This is the generalization of CSS 2.1 §10.3.7 for any writing mode.
/// The inline axis supports shrink-to-fit, direction-dependent over-constrained
/// handling, and direction-dependent static position.
///
/// Returns `(size, margin_start, margin_end, offset)` where offset is from
/// the inline-start edge of the containing block.
#[allow(clippy::similar_names)]
pub(crate) fn resolve_inline_axis(
    props: &InlineAxisProps,
    shrink_to_fit: impl FnOnce() -> f32,
) -> AxisResult {
    let mut ms = if props.margin_start_auto {
        0.0
    } else {
        props.margin_start_raw
    };
    let mut me = if props.margin_end_auto {
        0.0
    } else {
        props.margin_end_raw
    };

    match (props.start, props.size, props.end) {
        // All three auto: use static position for start, shrink-to-fit for size.
        (None, None, None) => {
            let w = shrink_to_fit();
            let offset = props.static_offset;
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        // start+size auto, end specified: shrink-to-fit, solve start.
        (None, None, Some(e)) => {
            let w = shrink_to_fit();
            let offset = props.containing - e - w - props.pb - ms - me;
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        // start auto only (size + end specified): solve start.
        (None, Some(w), Some(e)) => {
            let offset = props.containing - e - w - props.pb - ms - me;
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        // start+end auto, size specified: start = static position.
        (None, Some(w), None) => {
            let offset = props.static_offset;
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        // size+end auto, start specified: shrink-to-fit.
        (Some(s), None, None) => {
            let w = shrink_to_fit();
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset: s,
            }
        }
        // size auto, start+end specified: stretch.
        (Some(s), None, Some(e)) => {
            let w = (props.containing - s - e - props.pb - ms - me).max(0.0);
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset: s,
            }
        }
        // end auto, start+size specified.
        (Some(s), Some(w), None) => AxisResult {
            size: w,
            margin_start: ms,
            margin_end: me,
            offset: s,
        },
        // Over-constrained: all three specified.
        (Some(s), Some(w), Some(e)) => {
            let available = props.containing - s - w - props.pb - e;
            if props.margin_start_auto && props.margin_end_auto {
                if available < 0.0 {
                    // Negative centering: absorb overflow into end-side margin.
                    ms = 0.0;
                    me = available;
                } else {
                    let half = available / 2.0;
                    ms = half;
                    me = available - half;
                }
            } else if props.margin_start_auto {
                ms = available - me;
            } else if props.margin_end_auto {
                me = available - ms;
            } else {
                // All non-auto, over-constrained: ignore inline-end.
                // (The start offset stands as given.)
            }
            AxisResult {
                size: w,
                margin_start: ms,
                margin_end: me,
                offset: s,
            }
        }
    }
}

/// Resolve the block-axis constraint equation.
///
/// This is the generalization of CSS 2.1 §10.6.4 for any writing mode.
/// The block axis supports stretch (auto size with both offsets specified),
/// always-equal auto margin splitting, and always ignores block-end when
/// over-constrained.
///
/// Returns `(size, margin_start, margin_end, offset)` where offset is from
/// the block-start edge of the containing block.
#[allow(clippy::similar_names)]
pub(crate) fn resolve_block_axis(props: &BlockAxisProps) -> AxisResult {
    let mut ms = if props.margin_start_auto {
        0.0
    } else {
        props.margin_start_raw
    };
    let mut me = if props.margin_end_auto {
        0.0
    } else {
        props.margin_end_raw
    };

    // Effective size: specified or content-based.
    let h = props.size.or(props.content_size).unwrap_or(0.0);

    match (props.start, props.end) {
        (None, None) => {
            // block-start = static position.
            AxisResult {
                size: h,
                margin_start: ms,
                margin_end: me,
                offset: props.static_offset,
            }
        }
        (Some(s), None) => AxisResult {
            size: h,
            margin_start: ms,
            margin_end: me,
            offset: s,
        },
        (None, Some(e)) => {
            let offset = props.containing - e - h - props.pb - ms - me;
            AxisResult {
                size: h,
                margin_start: ms,
                margin_end: me,
                offset,
            }
        }
        (Some(s), Some(e)) => {
            if props.size.is_none() {
                // Auto size with both offsets → stretch.
                let stretch = (props.containing - s - e - props.pb - ms - me).max(0.0);
                AxisResult {
                    size: stretch,
                    margin_start: ms,
                    margin_end: me,
                    offset: s,
                }
            } else {
                // Over-constrained.
                let available = props.containing - s - h - props.pb - e;
                if props.margin_start_auto && props.margin_end_auto {
                    // Block axis: always equal split (no directional asymmetry).
                    let half = available / 2.0;
                    ms = half;
                    me = available - half;
                } else if props.margin_start_auto {
                    ms = available - me;
                } else if props.margin_end_auto {
                    me = available - ms;
                }
                // Over-constrained with no auto margins: block-end ignored,
                // offset stays at `s`.
                AxisResult {
                    size: h,
                    margin_start: ms,
                    margin_end: me,
                    offset: s,
                }
            }
        }
    }
}

/// Resolve a CSS size dimension (width or height) to a content-box pixel value.
///
/// Handles Length, Percentage (against `containing`), and Auto (falls back to
/// `intrinsic`). Applies border-box adjustment when `adjust_border_box` is true.
#[must_use]
pub(crate) fn resolve_size_value(
    dim: elidex_plugin::Dimension,
    containing: f32,
    pb: f32,
    adjust_border_box: bool,
    intrinsic: Option<f32>,
) -> Option<f32> {
    use crate::sanitize;
    match dim {
        elidex_plugin::Dimension::Length(px) => {
            let v = sanitize(px);
            Some(if adjust_border_box {
                (v - pb).max(0.0)
            } else {
                v
            })
        }
        elidex_plugin::Dimension::Percentage(pct) => {
            if containing >= 0.0 && containing.is_finite() {
                let v = sanitize(containing * pct / 100.0);
                Some(if adjust_border_box {
                    (v - pb).max(0.0)
                } else {
                    v
                })
            } else {
                None
            }
        }
        elidex_plugin::Dimension::Auto => intrinsic,
    }
}

/// Compute shrink-to-fit width for an absolutely positioned element.
pub(crate) fn shrink_to_fit_width(
    dom: &elidex_ecs::EcsDom,
    entity: elidex_ecs::Entity,
    font_db: &elidex_text::FontDatabase,
    depth: u32,
    cb_width: f32,
    h_pb: f32,
) -> f32 {
    let preferred = crate::block::children::max_content_width(dom, entity, font_db, depth);
    let available = (cb_width - h_pb).max(0.0);
    preferred.min(available).max(0.0)
}
