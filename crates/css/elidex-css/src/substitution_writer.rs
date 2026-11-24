//! CSS Syntax §9 token serialization at substitution boundaries.

use cssparser::{Parser, ParserInput, ToCss, Token, TokenSerializationType};
use elidex_plugin::component_values::{ComponentKind, ComponentValues};
use std::ops::Range;

const MAX_BYTES: usize = 65_536;

struct Piece {
    repaired: Option<String>,
    span: Range<usize>,
    category: TokenSerializationType,
    value: ComponentValues,

    separator: bool,
    join: bool,
}

impl Piece {
    fn source(&self) -> &str {
        self.repaired
            .as_deref()
            .unwrap_or_else(|| &self.value.source()[self.span.clone()])
    }
}

/// Collect and verify token spellings before publishing either projection.
#[derive(Default)]
pub struct SubstitutionWriter {
    pieces: Vec<Piece>,

    implicit_closers: Vec<char>,
    join: bool,
}

impl SubstitutionWriter {
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
        // Every token contributes at least one byte even after EOF repair.
        if range.len() > MAX_BYTES - self.pieces.len() {
            return None;
        }
        for token in &seq.items()[range] {
            self.pieces.push(Piece {
                repaired: None,
                span: token.span.clone(),
                value: seq.clone(),
                separator: false,
                category: token.serialization,
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

    pub fn finish(mut self) -> Option<String> {
        for i in 0..self.pieces.len() {
            if i > 0 && self.pieces[i].join {
                let mandatory = self.pieces[i - 1]
                    .category
                    .needs_separator_when_before(self.pieces[i].category);
                let start = i.saturating_sub(3);
                let end = (i + 4).min(self.pieces.len());
                let window = &self.pieces[start..end];
                let join = i - start;
                if mandatory || !equivalent(window, join, "") {
                    if equivalent(window, join, "/**/") {
                        self.pieces[i].separator = true;
                    } else {
                        let prev = &mut self.pieces[i - 1];
                        prev.repaired = Some(repair(prev.source())?);
                        if mandatory || !equivalent(&self.pieces[start..end], join, "") {
                            if !equivalent(&self.pieces[start..end], join, "/**/") {
                                return None;
                            }
                            self.pieces[i].separator = true;
                        }
                    }
                }
            }
        }
        let mut output = String::new();
        for piece in &self.pieces {
            let separator = if piece.separator { "/**/" } else { "" };
            let len = separator.len().checked_add(piece.source().len())?;
            if len > MAX_BYTES - output.len() {
                return None;
            }
            output.push_str(separator);
            output.push_str(piece.source());
        }
        if self.pieces.iter().any(|piece| piece.join) && !verified(&self.pieces, &output) {
            return None;
        }
        Some(output)
    }
}

fn first(source: &str) -> Option<(Token<'_>, usize)> {
    let mut pi = ParserInput::new(source);
    let mut p = Parser::new(&mut pi);
    let token = p.next_including_whitespace_and_comments().ok()?.clone();
    Some((token, p.position().byte_index()))
}

fn verified(pieces: &[Piece], mut remaining: &str) -> bool {
    for piece in pieces {
        if piece.separator {
            let Some((Token::Comment(""), bytes)) = first(remaining) else {
                return false;
            };
            remaining = &remaining[bytes..];
        }
        let Some((actual, bytes)) = first(remaining) else {
            return false;
        };
        let Some((intended, _)) = first(&piece.value.source()[piece.span.start..]) else {
            return false;
        };
        if actual != intended || bytes != piece.source().len() {
            return false;
        }
        remaining = &remaining[bytes..];
    }
    remaining.is_empty()
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
        let Some((expected, _)) = first(&piece.value.source()[piece.span.start..]) else {
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
