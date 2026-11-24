//! CSS Syntax §9 token serialization at substitution boundaries.

use cssparser::{Parser, ParserInput, ToCss, Token};
use elidex_plugin::component_values::{ComponentKind, ComponentValues, Spelling};
use std::ops::Range;

const MAX_BYTES: usize = 65_536;

struct Piece {
    spelling: Spelling,
    intended: Spelling,
    kind: ComponentKind,
    separator: bool,
    join: bool,
}

impl Piece {
    fn source(&self) -> &str {
        self.spelling.as_str()
    }
}

/// Collect and verify token spellings before publishing either projection.
#[derive(Default)]
pub struct SubstitutionWriter {
    pieces: Vec<Piece>,
    bytes: usize,

    implicit_closers: Vec<char>,
    join: bool,
}

impl SubstitutionWriter {
    pub const MAX_BYTES: usize = MAX_BYTES;
    fn boundary(&mut self) {
        self.join = true;
    }

    pub fn append(&mut self, seq: &ComponentValues, range: Range<usize>) -> Option<()> {
        if !range.is_empty() && !self.implicit_closers.is_empty() {
            let closers = std::mem::take(&mut self.implicit_closers);
            for close in closers {
                let pair = match close {
                    ')' => "()",
                    ']' => "[]",
                    _ => "{}",
                };
                let block = ComponentValues::capture(pair)?;
                self.boundary();
                self.append(&block, 1..2)?;
            }
            self.boundary();
        }
        if range.is_empty() {
            return Some(());
        }
        let items = seq.items();
        let len = items[range.end - 1].span.end - items[range.start].span.start;
        if len > MAX_BYTES - self.bytes {
            return None;
        }
        self.bytes += len;
        for index in range {
            let spelling = seq.spelling(index);
            self.pieces.push(Piece {
                intended: spelling.clone(),
                spelling,
                kind: items[index].kind.clone(),
                separator: false,
                join: std::mem::take(&mut self.join),
            });
        }
        Some(())
    }

    /// A complete replacement can contain blocks closed implicitly by EOF.
    /// If another component follows, emit those closers before that component.
    pub fn replacement(&mut self, seq: &ComponentValues) -> Option<()> {
        self.boundary();
        self.append(seq, 0..seq.items().len())?;
        for item in seq.items().iter().rev().filter(|item| !item.closed) {
            match item.kind {
                ComponentKind::Function(_) => self.implicit_closers.push(')'),
                ComponentKind::Block(close) => self.implicit_closers.push(close),
                _ => {}
            }
        }
        self.boundary();
        Some(())
    }

    pub fn finish(mut self) -> Option<ComponentValues> {
        for i in 0..self.pieces.len() {
            if i > 0 && self.pieces[i].join {
                // CSS Syntax §9 also requires separators for certain pairs even
                // when the particular spellings would happen to tokenize separately.
                // Retained comment trivia is absent from the spec's token-pair table.
                let mandatory = match (
                    first(self.pieces[i - 1].source())?.0,
                    first(self.pieces[i].source())?.0,
                ) {
                    (Token::Comment(_), _) | (_, Token::Comment(_)) => false,
                    (left, right) => left
                        .serialization_type()
                        .needs_separator_when_before(right.serialization_type()),
                };
                let start = i.saturating_sub(3);
                let end = (i + 4).min(self.pieces.len());
                let window = &self.pieces[start..end];
                let join = i - start;
                if mandatory || !equivalent(window, join, "") {
                    if equivalent(window, join, "/**/") {
                        self.pieces[i].separator = true;
                    } else {
                        let prev = &mut self.pieces[i - 1];
                        let repaired = repair(prev.source())?;
                        self.bytes = self.bytes - prev.source().len() + repaired.len();
                        if self.bytes > MAX_BYTES {
                            return None;
                        }
                        prev.spelling = Spelling::owned(repaired);
                        if mandatory || !equivalent(&self.pieces[start..end], join, "") {
                            if !equivalent(&self.pieces[start..end], join, "/**/") {
                                return None;
                            }
                            self.pieces[i].separator = true;
                        }
                    }
                }
                if self.pieces[i].separator {
                    self.bytes += 4;
                    if self.bytes > MAX_BYTES {
                        return None;
                    }
                }
            }
        }
        ComponentValues::from_splices(self.pieces.into_iter().flat_map(|piece| {
            let separator = piece
                .separator
                .then(|| (ComponentKind::Comment, Spelling::owned("/**/".to_owned())));
            separator
                .into_iter()
                .chain(std::iter::once((piece.kind, piece.spelling)))
        }))
    }
}

fn first(source: &str) -> Option<(Token<'_>, usize)> {
    let mut pi = ParserInput::new(source);
    let mut p = Parser::new(&mut pi);
    let token = p.next_including_whitespace_and_comments().ok()?.clone();
    Some((token, p.position().byte_index()))
}

fn equivalent(pieces: &[Piece], join: usize, separator: &str) -> bool {
    let mut trial = String::new();
    for (index, piece) in pieces.iter().enumerate() {
        if index == join {
            trial.push_str(separator);
        } else if index > 0 && piece.separator {
            trial.push_str("/**/");
        }
        trial.push_str(piece.source());
    }
    let mut remaining = trial.as_str();
    for (index, piece) in pieces.iter().enumerate() {
        if (index == join && !separator.is_empty()) || (index > 0 && piece.separator) {
            let Some((Token::Comment(""), len)) = first(remaining) else {
                return false;
            };
            remaining = &remaining[len..];
        }
        if index == join {
            return true;
        }
        let Some((actual, len)) = first(remaining) else {
            return false;
        };
        let Some((expected, _)) = first(piece.intended.as_str()) else {
            return false;
        };
        if actual != expected || len != piece.source().len() {
            return false;
        }
        remaining = &remaining[len..];
    }
    remaining.is_empty()
}

fn repair(source: &str) -> Option<String> {
    let (token, _) = first(source)?;
    Some(match token {
        Token::Comment(body) if source.len() == body.len() + 2 => format!("{source}*/"),
        _ => token.to_css_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_occurrence_before_queuing_a_join() {
        let cached = ComponentValues::capture(&"x".repeat(MAX_BYTES / 2 + 1)).unwrap();
        let mut writer = SubstitutionWriter::default();
        writer.replacement(&cached).unwrap();
        assert!(writer.replacement(&cached).is_none());
        assert_eq!(writer.pieces.len(), 1);
        assert_eq!(writer.bytes, MAX_BYTES / 2 + 1);
    }
}
