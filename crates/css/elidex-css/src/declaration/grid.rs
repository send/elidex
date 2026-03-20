//! CSS Grid track list and line value parsers.
//!
//! Handles `grid-template-columns/rows` track list parsing, `repeat()`, line names,
//! and `<grid-line>` value parsing.
//!
//! Other grid properties (longhands and shorthands) are in the sibling
//! `grid_shorthand` module.

use cssparser::{Parser, Token};
use elidex_plugin::{CssValue, LengthUnit};

use crate::values::{parse_length_or_percentage, parse_non_negative_length_or_percentage};

use super::{single_decl, Declaration};

// ---------------------------------------------------------------------------
// Track list parsing (grid-template-columns / grid-template-rows)
// ---------------------------------------------------------------------------

/// Parse a single `<track-size>` value.
///
/// Returns `CssValue` encoding:
/// - `Length(v, Px)` for px values
/// - `Percentage(v)` for percentages
/// - `Length(v, Fr)` for fr values
/// - `Auto` for `auto`
/// - `Keyword("min-content")` / `Keyword("max-content")`
/// - `List([Keyword("minmax"), min, max])` for `minmax()`
pub(crate) fn parse_track_size(input: &mut Parser) -> Result<CssValue, ()> {
    // Try minmax() function.
    if let Ok(val) = input.try_parse(parse_minmax) {
        return Ok(val);
    }

    // Try keywords: auto, min-content, max-content.
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        match ident.to_ascii_lowercase().as_str() {
            "auto" => Ok(CssValue::Auto),
            "min-content" => Ok(CssValue::Keyword("min-content".into())),
            "max-content" => Ok(CssValue::Keyword("max-content".into())),
            _ => Err(()),
        }
    }) {
        return Ok(val);
    }

    // Try fr unit.
    if let Ok(val) = input.try_parse(parse_fr) {
        return Ok(val);
    }

    // Fall back to length/percentage.
    parse_non_negative_length_or_percentage(input)
}

/// Parse an `<fr>` dimension value.
fn parse_fr(input: &mut Parser) -> Result<CssValue, ()> {
    let tok = input.next().map_err(|_| ())?;
    match tok {
        Token::Dimension {
            value, ref unit, ..
        } if unit.eq_ignore_ascii_case("fr") && *value >= 0.0 => {
            Ok(CssValue::Length(*value, LengthUnit::Fr))
        }
        _ => Err(()),
    }
}

/// Parse `minmax(min, max)`.
fn parse_minmax(input: &mut Parser) -> Result<CssValue, ()> {
    input.expect_function_matching("minmax").map_err(|_| ())?;
    input
        .parse_nested_block(|args| -> Result<CssValue, cssparser::ParseError<'_, ()>> {
            let min = parse_track_breadth(args).map_err(|()| args.new_custom_error(()))?;
            args.expect_comma().map_err(cssparser::ParseError::from)?;
            let max = parse_track_breadth(args).map_err(|()| args.new_custom_error(()))?;
            Ok(CssValue::List(vec![
                CssValue::Keyword("minmax".into()),
                min,
                max,
            ]))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}

/// Parse a `<track-breadth>` value (used inside minmax).
fn parse_track_breadth(input: &mut Parser) -> Result<CssValue, ()> {
    // Try keywords first.
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        match ident.to_ascii_lowercase().as_str() {
            "auto" => Ok(CssValue::Auto),
            "min-content" => Ok(CssValue::Keyword("min-content".into())),
            "max-content" => Ok(CssValue::Keyword("max-content".into())),
            _ => Err(()),
        }
    }) {
        return Ok(val);
    }

    // Try fr.
    if let Ok(val) = input.try_parse(parse_fr) {
        return Ok(val);
    }

    // Fall back to length/percentage.
    parse_length_or_percentage(input)
}

/// Result from parsing a `repeat()` function.
enum RepeatResult {
    /// Integer repeat: already expanded tracks.
    Expanded(Vec<CssValue>),
    /// Auto-fill or auto-fit: the mode keyword and the pattern tracks.
    AutoRepeat(String, Vec<CssValue>),
}

/// Parse `[line-names]` bracket block: `[ident1 ident2 ...]`.
pub(crate) fn parse_line_names(input: &mut Parser) -> Result<Vec<String>, ()> {
    input.expect_square_bracket_block().map_err(|_| ())?;
    input
        .parse_nested_block(
            |block| -> Result<Vec<String>, cssparser::ParseError<'_, ()>> {
                let mut names = Vec::new();
                while let Ok(ident) =
                    block.try_parse(|b| b.expect_ident().map(std::string::ToString::to_string))
                {
                    names.push(ident);
                }
                Ok(names)
            },
        )
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}

/// Interleaved track + line-names representation during parsing.
pub(crate) struct TrackListParts {
    pub(crate) tracks: Vec<CssValue>,
    pub(crate) line_names: Vec<Vec<String>>, // len == tracks.len() + 1 (once finalized)
}

impl TrackListParts {
    pub(crate) fn new() -> Self {
        Self {
            tracks: Vec::new(),
            line_names: Vec::new(),
        }
    }

    pub(crate) fn push_names(&mut self, mut names: Vec<String>) {
        // If we already have a trailing names entry, merge into it.
        if self.line_names.len() > self.tracks.len() {
            self.line_names.last_mut().unwrap().append(&mut names);
        } else {
            self.line_names.push(names);
        }
    }

    pub(crate) fn push_track(&mut self, track: CssValue) {
        // Ensure there's a line-names entry before this track.
        while self.line_names.len() <= self.tracks.len() {
            self.line_names.push(vec![]);
        }
        self.tracks.push(track);
    }

    pub(crate) fn finalize(&mut self) {
        // Ensure trailing line-names entry.
        while self.line_names.len() <= self.tracks.len() {
            self.line_names.push(vec![]);
        }
    }

    pub(crate) fn has_names(&self) -> bool {
        self.line_names.iter().any(|n| !n.is_empty())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Encode as `CssValue`. If there are named lines, use the named-tracks marker.
    #[allow(clippy::wrong_self_convention)] // Consumes fields via mem::take
    pub(crate) fn to_css_value(&mut self) -> CssValue {
        self.finalize();
        if !self.has_names() {
            return CssValue::List(std::mem::take(&mut self.tracks));
        }
        // Encode: [Keyword("named-tracks"), List(names_0), track0, List(names_1), track1, ..., List(names_n)]
        let mut items = vec![CssValue::Keyword("named-tracks".into())];
        for (i, track) in self.tracks.iter().enumerate() {
            let names = self.line_names.get(i).cloned().unwrap_or_default();
            items.push(CssValue::List(
                names.into_iter().map(CssValue::Keyword).collect(),
            ));
            items.push(track.clone());
        }
        let trailing = self
            .line_names
            .get(self.tracks.len())
            .cloned()
            .unwrap_or_default();
        items.push(CssValue::List(
            trailing.into_iter().map(CssValue::Keyword).collect(),
        ));
        CssValue::List(items)
    }
}

/// Parse `grid-template-columns` or `grid-template-rows`.
///
/// Accepts: `none` | `[name] <track-size>+ [name]` | `repeat(...)`.
/// `[name]` brackets are optional at each line boundary.
pub(crate) fn parse_grid_template(input: &mut Parser, name: &str) -> Vec<Declaration> {
    // Try `none` keyword.
    if let Ok(()) = input.try_parse(|i| -> Result<(), ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case("none") {
            Ok(())
        } else {
            Err(())
        }
    }) {
        return single_decl(name, CssValue::Keyword("none".into()));
    }

    let mut before = TrackListParts::new();
    let mut auto_repeat: Option<(String, Vec<CssValue>)> = None;
    let mut after = TrackListParts::new();

    while !input.is_exhausted() {
        // Try [line-names]
        if let Ok(names) = input.try_parse(parse_line_names) {
            if auto_repeat.is_some() {
                after.push_names(names);
            } else {
                before.push_names(names);
            }
            continue;
        }

        // Try repeat() function.
        if let Ok(result) = input.try_parse(parse_repeat) {
            match result {
                RepeatResult::Expanded(expanded) => {
                    let target = if auto_repeat.is_some() {
                        &mut after
                    } else {
                        &mut before
                    };
                    for ts in expanded {
                        target.push_track(ts);
                    }
                }
                RepeatResult::AutoRepeat(mode, pattern) => {
                    if auto_repeat.is_some() {
                        break;
                    }
                    auto_repeat = Some((mode, pattern));
                }
            }
            continue;
        }

        // Try a single track-size.
        if let Ok(ts) = input.try_parse(parse_track_size) {
            if auto_repeat.is_some() {
                after.push_track(ts);
            } else {
                before.push_track(ts);
            }
            continue;
        }

        break;
    }

    if let Some((mode, pattern)) = auto_repeat {
        let value = CssValue::List(vec![
            CssValue::Keyword("auto-repeat".into()),
            CssValue::Keyword(mode),
            before.to_css_value(),
            CssValue::List(pattern),
            after.to_css_value(),
        ]);
        return single_decl(name, value);
    }

    if before.is_empty() {
        return Vec::new();
    }

    single_decl(name, before.to_css_value())
}

/// Maximum repeat count to prevent OOM from malicious CSS (e.g. `repeat(999999999, 1fr)`).
const MAX_REPEAT_COUNT: u32 = 10_000;

/// Parse `repeat(N, <track-size>+)` or `repeat(auto-fill/auto-fit, <track-size>+)`.
#[allow(clippy::cast_sign_loss)] // CSS repeat count is always >= 1
fn parse_repeat(input: &mut Parser) -> Result<RepeatResult, ()> {
    input.expect_function_matching("repeat").map_err(|_| ())?;
    input
        .parse_nested_block(
            |args| -> Result<RepeatResult, cssparser::ParseError<'_, ()>> {
                // Try integer count first.
                let count_or_mode = if let Ok(n) = args.try_parse(|i| -> Result<u32, ()> {
                    let tok = i.next().map_err(|_| ())?;
                    match *tok {
                        Token::Number {
                            int_value: Some(n), ..
                        } if n >= 1 => Ok(n as u32),
                        _ => Err(()),
                    }
                }) {
                    Ok(n)
                } else {
                    // auto-fill / auto-fit
                    let ident = args.expect_ident().map_err(cssparser::ParseError::from)?;
                    let lower = ident.to_ascii_lowercase();
                    if lower == "auto-fill" || lower == "auto-fit" {
                        Err(lower)
                    } else {
                        return Err(args.new_custom_error(()));
                    }
                };

                args.expect_comma().map_err(cssparser::ParseError::from)?;

                // Parse track list inside repeat.
                let mut pattern = Vec::new();
                while !args.is_exhausted() {
                    let ts = parse_track_size(args).map_err(|()| args.new_custom_error(()))?;
                    pattern.push(ts);
                }
                if pattern.is_empty() {
                    return Err(args.new_custom_error(()));
                }

                match count_or_mode {
                    Ok(count) => {
                        let count = count.min(MAX_REPEAT_COUNT);
                        let mut result = Vec::new();
                        for _ in 0..count {
                            result.extend(pattern.clone());
                        }
                        Ok(RepeatResult::Expanded(result))
                    }
                    Err(mode) => {
                        // CSS Grid §7.2.3.2: auto-repeat tracks must all be
                        // fixed-size (Length, Percentage, or minmax with fixed
                        // bounds). Reject patterns containing fr, auto,
                        // min-content, or max-content.
                        if !pattern.iter().all(is_fixed_track_value) {
                            return Err(args.new_custom_error(()));
                        }
                        Ok(RepeatResult::AutoRepeat(mode, pattern))
                    }
                }
            },
        )
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}

/// Check whether a parsed track-size `CssValue` is a `<fixed-size>` per
/// CSS Grid §7.2.3.2. Valid forms:
///   - `<fixed-breadth>` (length or percentage, not `fr`)
///   - `minmax(<fixed-breadth>, <track-breadth>)` — any max OK if min is fixed
///   - `minmax(<inflexible-breadth>, <fixed-breadth>)` — any non-fr min OK if max is fixed
fn is_fixed_track_value(v: &CssValue) -> bool {
    match v {
        CssValue::Length(_, unit) => *unit != LengthUnit::Fr,
        CssValue::Percentage(_) => true,
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("minmax".into())) && items.len() == 3 =>
        {
            let min_fixed = is_fixed_breadth_value(&items[1]);
            let max_fixed = is_fixed_breadth_value(&items[2]);
            if min_fixed {
                // minmax(<fixed-breadth>, <track-breadth>) — any max is OK
                true
            } else {
                // minmax(<inflexible-breadth>, <fixed-breadth>) — min must not be fr
                is_inflexible_breadth_value(&items[1]) && max_fixed
            }
        }
        _ => false,
    }
}

/// Check whether a track-breadth `CssValue` is a fixed (definite) size.
/// `fr` units are not fixed.
fn is_fixed_breadth_value(v: &CssValue) -> bool {
    match v {
        CssValue::Length(_, unit) => *unit != LengthUnit::Fr,
        CssValue::Percentage(_) => true,
        _ => false,
    }
}

/// Check whether a track-breadth is inflexible (anything except `fr`).
fn is_inflexible_breadth_value(v: &CssValue) -> bool {
    !matches!(v, CssValue::Length(_, unit) if *unit == LengthUnit::Fr)
}

/// Forbidden identifiers for grid-line and area names (CSS Grid §8.1).
pub(crate) fn is_forbidden_grid_ident(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "auto" | "span" | "inherit" | "initial" | "unset" | "default"
    )
}

// ---------------------------------------------------------------------------
// Grid line placement
// ---------------------------------------------------------------------------

/// Parse a single `<grid-line>` value (CSS Grid §8.1).
///
/// ```text
/// auto | <custom-ident> |
/// [ <integer [-∞,-1]> | <integer [1,∞]> ] && <custom-ident>? |
/// [ span && [ <integer [1,∞]> || <custom-ident> ] ]
/// ```
#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
// CSS grid line values: full §8.1 grammar with named idents.
pub(crate) fn parse_grid_line_value(input: &mut Parser) -> Result<CssValue, ()> {
    // Try `auto`.
    if input
        .try_parse(|i| -> Result<(), ()> {
            let ident = i.expect_ident().map_err(|_| ())?;
            if ident.eq_ignore_ascii_case("auto") {
                Ok(())
            } else {
                Err(())
            }
        })
        .is_ok()
    {
        return Ok(CssValue::Auto);
    }

    // Try span variants.
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if !ident.eq_ignore_ascii_case("span") {
            return Err(());
        }

        // Try integer first
        let maybe_int = i.try_parse(|i2| -> Result<i32, ()> {
            let tok = i2.next().map_err(|_| ())?;
            match *tok {
                Token::Number {
                    int_value: Some(n), ..
                } if n >= 1 => Ok(n),
                _ => Err(()),
            }
        });

        // Try ident
        let maybe_ident = i.try_parse(|i2| -> Result<String, ()> {
            let id = i2.expect_ident().map_err(|_| ())?;
            let s = id.to_string();
            if is_forbidden_grid_ident(&s) {
                return Err(());
            }
            Ok(s)
        });

        let (n, named_ident) = match (maybe_int, maybe_ident) {
            (Ok(n), Ok(ident)) => (n, Some(ident)),
            (Ok(n), Err(())) => (n, None),
            (Err(()), Ok(ident)) => {
                let trailing = i.try_parse(|i2| -> Result<i32, ()> {
                    let tok = i2.next().map_err(|_| ())?;
                    match *tok {
                        Token::Number {
                            int_value: Some(n), ..
                        } if n >= 1 => Ok(n),
                        _ => Err(()),
                    }
                });
                (trailing.unwrap_or(1), Some(ident))
            }
            (Err(()), Err(())) => return Err(()),
        };

        if let Some(ident) = named_ident {
            Ok(CssValue::List(vec![
                CssValue::Keyword("span-named".into()),
                CssValue::Number(n as f32),
                CssValue::Keyword(ident),
            ]))
        } else {
            Ok(CssValue::List(vec![
                CssValue::Keyword("span".into()),
                CssValue::Number(n as f32),
            ]))
        }
    }) {
        return Ok(val);
    }

    // Try "<integer> <custom-ident>" or just "<integer>".
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let tok = i.next().map_err(|_| ())?;
        let n = match *tok {
            Token::Number {
                int_value: Some(n), ..
            } if n != 0 => n,
            _ => return Err(()),
        };
        let maybe_ident = i.try_parse(|i2| -> Result<String, ()> {
            let id = i2.expect_ident().map_err(|_| ())?;
            let s = id.to_string();
            if is_forbidden_grid_ident(&s) {
                return Err(());
            }
            Ok(s)
        });
        if let Ok(ident) = maybe_ident {
            Ok(CssValue::List(vec![
                CssValue::Number(n as f32),
                CssValue::Keyword(ident),
            ]))
        } else {
            Ok(CssValue::Number(n as f32))
        }
    }) {
        return Ok(val);
    }

    // Try "<custom-ident> <integer>" or just "<custom-ident>".
    input.try_parse(|i| -> Result<CssValue, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        let s = ident.to_string();
        if is_forbidden_grid_ident(&s) {
            return Err(());
        }
        let maybe_int = i.try_parse(|i2| -> Result<i32, ()> {
            let tok = i2.next().map_err(|_| ())?;
            match *tok {
                Token::Number {
                    int_value: Some(n), ..
                } if n != 0 => Ok(n),
                _ => Err(()),
            }
        });
        if let Ok(n) = maybe_int {
            Ok(CssValue::List(vec![
                CssValue::Keyword(s),
                CssValue::Number(n as f32),
            ]))
        } else {
            Ok(CssValue::Keyword(s))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ParserInput;

    fn parse_template(css: &str) -> Vec<Declaration> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_grid_template(&mut parser, "grid-template-columns")
    }

    #[test]
    fn repeat_count_capped_at_max() {
        // A huge repeat count should be capped at MAX_REPEAT_COUNT.
        let decls = parse_template("repeat(99999, 1fr)");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(tracks) = &decls[0].value {
            assert_eq!(tracks.len(), MAX_REPEAT_COUNT as usize);
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn repeat_small_count_works() {
        let decls = parse_template("repeat(3, 100px)");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(tracks) = &decls[0].value {
            assert_eq!(tracks.len(), 3);
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn auto_fill_emits_auto_repeat_marker() {
        let decls = parse_template("repeat(auto-fill, 200px)");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(items) = &decls[0].value {
            assert_eq!(items[0], CssValue::Keyword("auto-repeat".into()));
            assert_eq!(items[1], CssValue::Keyword("auto-fill".into()));
            // before: empty
            assert_eq!(items[2], CssValue::List(vec![]));
            // pattern: [Length(200, Px)]
            if let CssValue::List(pattern) = &items[3] {
                assert_eq!(pattern.len(), 1);
            } else {
                panic!("expected pattern List");
            }
            // after: empty
            assert_eq!(items[4], CssValue::List(vec![]));
        } else {
            panic!("expected List with auto-repeat marker");
        }
    }

    #[test]
    fn auto_fit_emits_auto_repeat_marker() {
        let decls = parse_template("repeat(auto-fit, 100px 200px)");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(items) = &decls[0].value {
            assert_eq!(items[0], CssValue::Keyword("auto-repeat".into()));
            assert_eq!(items[1], CssValue::Keyword("auto-fit".into()));
            if let CssValue::List(pattern) = &items[3] {
                assert_eq!(pattern.len(), 2);
            } else {
                panic!("expected pattern List");
            }
        } else {
            panic!("expected List with auto-repeat marker");
        }
    }

    #[test]
    fn auto_repeat_rejects_non_fixed_tracks() {
        // repeat(auto-fill, 1fr) should be rejected — fr is not a fixed size.
        let decls = parse_template("repeat(auto-fill, 1fr)");
        assert!(
            decls.is_empty(),
            "repeat(auto-fill, 1fr) should be rejected, got {decls:?}"
        );

        // repeat(auto-fit, auto) should also be rejected.
        let decls = parse_template("repeat(auto-fit, auto)");
        assert!(
            decls.is_empty(),
            "repeat(auto-fit, auto) should be rejected, got {decls:?}"
        );

        // repeat(auto-fill, minmax(100px, 1fr)) is valid: min is <fixed-breadth>.
        let decls = parse_template("repeat(auto-fill, minmax(100px, 1fr))");
        assert_eq!(
            decls.len(),
            1,
            "repeat(auto-fill, minmax(100px, 1fr)) should be accepted (fixed min)"
        );

        // repeat(auto-fill, minmax(min-content, 200px)) is valid: max is <fixed-breadth>.
        let decls = parse_template("repeat(auto-fill, minmax(min-content, 200px))");
        assert_eq!(
            decls.len(),
            1,
            "repeat(auto-fill, minmax(min-content, 200px)) should be accepted"
        );

        // repeat(auto-fill, minmax(1fr, 200px)) is invalid: min is fr (flexible).
        let decls = parse_template("repeat(auto-fill, minmax(1fr, 200px))");
        assert!(
            decls.is_empty(),
            "repeat(auto-fill, minmax(1fr, 200px)) should be rejected (fr min), got {decls:?}"
        );

        // repeat(auto-fill, 100px) should still work.
        let decls = parse_template("repeat(auto-fill, 100px)");
        assert_eq!(
            decls.len(),
            1,
            "repeat(auto-fill, 100px) should be accepted"
        );

        // repeat(auto-fit, minmax(100px, 200px)) should work (both bounds fixed).
        let decls = parse_template("repeat(auto-fit, minmax(100px, 200px))");
        assert_eq!(
            decls.len(),
            1,
            "repeat(auto-fit, minmax(100px, 200px)) should be accepted"
        );
    }

    #[test]
    fn auto_fill_with_fixed_tracks() {
        // 100px repeat(auto-fill, 200px) 50px
        let decls = parse_template("100px repeat(auto-fill, 200px) 50px");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(items) = &decls[0].value {
            assert_eq!(items[0], CssValue::Keyword("auto-repeat".into()));
            assert_eq!(items[1], CssValue::Keyword("auto-fill".into()));
            // before: [Length(100, Px)]
            if let CssValue::List(before) = &items[2] {
                assert_eq!(before.len(), 1);
            } else {
                panic!("expected before List");
            }
            // after: [Length(50, Px)]
            if let CssValue::List(after) = &items[4] {
                assert_eq!(after.len(), 1);
            } else {
                panic!("expected after List");
            }
        } else {
            panic!("expected List with auto-repeat marker");
        }
    }

    // ---------------------------------------------------------------------------
    // Named line tests
    // ---------------------------------------------------------------------------

    #[test]
    fn named_lines_basic() {
        // "[a] 1fr [b]" — named-tracks marker present, 1 track
        let decls = parse_template("[a] 1fr [b]");
        assert_eq!(decls.len(), 1, "expected 1 declaration");
        if let CssValue::List(items) = &decls[0].value {
            assert_eq!(
                items[0],
                CssValue::Keyword("named-tracks".into()),
                "expected named-tracks marker"
            );
            // Structure: [named-tracks, [a], 1fr, [b]]
            // items[1] = names_0 = ["a"], items[2] = track, items[3] = names_1 = ["b"]
            assert_eq!(
                items.len(),
                4,
                "expected 4 items (marker + names + track + names)"
            );
        } else {
            panic!("expected List, got {:?}", decls[0].value);
        }
    }

    #[test]
    fn named_lines_multiple_names() {
        // "[a b] 1fr" — two names in first name slot
        let decls = parse_template("[a b] 1fr");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(items) = &decls[0].value {
            assert_eq!(items[0], CssValue::Keyword("named-tracks".into()));
            // items[1] should be List([Keyword("a"), Keyword("b")])
            if let CssValue::List(names) = &items[1] {
                assert_eq!(names.len(), 2);
                assert_eq!(names[0], CssValue::Keyword("a".into()));
                assert_eq!(names[1], CssValue::Keyword("b".into()));
            } else {
                panic!("expected List for names, got {:?}", items[1]);
            }
        } else {
            panic!("expected List, got {:?}", decls[0].value);
        }
    }

    #[test]
    fn named_lines_empty_brackets() {
        // "[] 1fr" — valid parse with empty name list
        let decls = parse_template("[] 1fr");
        assert_eq!(decls.len(), 1, "expected 1 declaration for [] 1fr");
        // Empty brackets with no names → no named-tracks marker (no names to record)
        // The track list should just be a simple List with the track value
        match &decls[0].value {
            CssValue::List(_) => {} // Either with or without marker is acceptable
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn named_lines_compat_no_names() {
        // "1fr 2fr" — NO named-tracks marker (backward compat)
        let decls = parse_template("1fr 2fr");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(items) = &decls[0].value {
            // Must NOT start with "named-tracks" marker
            assert_ne!(
                items.first(),
                Some(&CssValue::Keyword("named-tracks".into())),
                "plain tracks should not have named-tracks marker"
            );
            assert_eq!(items.len(), 2, "expected 2 track items");
        } else {
            panic!("expected List, got {:?}", decls[0].value);
        }
    }
}
