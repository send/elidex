//! CSS Syntax §5.5 component values. Flat storage keeps deeply nested blocks
//! independent of the native call stack, including when the sequence is dropped.

use std::ops::Range;
use std::sync::Arc;

use cssparser::{Parser, ParserInput, Token, TokenSerializationType};

/// Immutable authored bytes plus a token index. Block openers point past their
/// children (and the closing token, when present). CSS Syntax closes blocks at EOF.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentValues {
    source: Arc<str>,
    items: Arc<[Component]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub kind: ComponentKind,
    pub span: Range<usize>,
    pub end: usize,
    pub serialization: TokenSerializationType,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComponentKind {
    Ident(String),
    Function(String),
    Block(char),
    Close(char),
    Comma,
    Delim(char),
    Whitespace,
    Comment,
    AtKeyword,
    Hash,
    IdHash,
    String,
    Url,
    Number,
    Percentage,
    Dimension,
    Colon,
    Semicolon,
    IncludeMatch,
    DashMatch,
    PrefixMatch,
    SuffixMatch,
    SubstringMatch,
    Cdo,
    Cdc,
}

impl ComponentValues {
    /// Tokenize an entire declaration-value, retaining exact source slices.
    /// Opening blocks and their closing delimiters occupy separate positions.
    #[must_use]
    pub fn capture(source: &str) -> Option<Self> {
        Self::scan(source, true)
    }

    /// Tokenize a component sequence inside a function, where ! and ; are ordinary tokens.
    #[must_use]
    pub fn fragment(source: &str) -> Option<Self> {
        Self::scan(source, false)
    }

    fn scan(source: &str, declaration: bool) -> Option<Self> {
        let mut items: Vec<Component> = Vec::new();
        let mut blocks: Vec<(usize, char)> = Vec::new();
        let mut offset = 0;
        while offset < source.len() {
            // Parser normally skips an unopened block on the next call. Advance
            // by one token so the index includes every child and closing token.
            let mut pi = ParserInput::new(&source[offset..]);
            let mut parser = Parser::new(&mut pi);
            let token = parser.next_including_whitespace_and_comments().ok()?;
            let index = items.len();
            let kind = match token {
                Token::Function(name) => {
                    blocks.push((index, ')'));
                    ComponentKind::Function(name.to_string())
                }
                Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                    let close = match token {
                        Token::ParenthesisBlock => ')',
                        Token::SquareBracketBlock => ']',
                        _ => '}',
                    };
                    blocks.push((index, close));
                    ComponentKind::Block(close)
                }
                Token::CloseParenthesis | Token::CloseSquareBracket | Token::CloseCurlyBracket => {
                    let close = match token {
                        Token::CloseParenthesis => ')',
                        Token::CloseSquareBracket => ']',
                        _ => '}',
                    };
                    let (open, expected) = blocks.pop()?;
                    if close != expected {
                        return None;
                    }
                    items[open].end = index + 1;
                    items[open].closed = true;
                    ComponentKind::Close(close)
                }
                Token::BadUrl(_) | Token::BadString(_) => return None,
                Token::Semicolon | Token::Delim('!') if declaration && blocks.is_empty() => {
                    return None
                }
                Token::Ident(name) => ComponentKind::Ident(name.to_string()),
                Token::WhiteSpace(_) => ComponentKind::Whitespace,
                Token::Comment(_) => ComponentKind::Comment,
                Token::Comma => ComponentKind::Comma,
                Token::Delim(ch) => ComponentKind::Delim(*ch),
                Token::AtKeyword(_) => ComponentKind::AtKeyword,
                Token::Hash(_) => ComponentKind::Hash,
                Token::IDHash(_) => ComponentKind::IdHash,
                Token::QuotedString(_) => ComponentKind::String,
                Token::UnquotedUrl(_) => ComponentKind::Url,
                Token::Number { .. } => ComponentKind::Number,
                Token::Percentage { .. } => ComponentKind::Percentage,
                Token::Dimension { .. } => ComponentKind::Dimension,
                Token::Colon => ComponentKind::Colon,
                Token::Semicolon => ComponentKind::Semicolon,
                Token::IncludeMatch => ComponentKind::IncludeMatch,
                Token::DashMatch => ComponentKind::DashMatch,
                Token::PrefixMatch => ComponentKind::PrefixMatch,
                Token::SuffixMatch => ComponentKind::SuffixMatch,
                Token::SubstringMatch => ComponentKind::SubstringMatch,
                Token::CDO => ComponentKind::Cdo,
                Token::CDC => ComponentKind::Cdc,
            };
            let serialization = token.serialization_type();
            let next = offset + parser.position().byte_index();
            items.push(Component {
                kind,
                span: offset..next,
                end: index + 1,
                serialization,
                closed: false,
            });
            offset = next;
        }
        let end = items.len();
        for (open, _) in blocks {
            items[open].end = end;
        }
        Some(Self {
            source: source.into(),
            items: items.into(),
        })
    }

    /// CSS Variables §3 requires a nonempty first argument. Its value is
    /// parsed as a custom-property name after substitution. A comma alone
    /// introduces the (possibly empty) fallback.
    #[must_use]
    pub fn valid_variables(&self) -> bool {
        self.items.iter().enumerate().all(|(i, item)| {
            if !matches!(&item.kind, ComponentKind::Function(f) if f.eq_ignore_ascii_case("var")) {
                return true;
            }
            let args = self.arguments(i);
            let present = self.items[args.0.clone()]
                .iter()
                .any(|arg| !matches!(arg.kind, ComponentKind::Whitespace | ComponentKind::Comment));
            let mut cursor = args.0.start;
            let end = item.end - usize::from(item.closed);
            while cursor < end {
                if matches!(
                    self.items[cursor].kind,
                    ComponentKind::Semicolon | ComponentKind::Delim('!')
                ) {
                    return false;
                }
                cursor = self.items[cursor].end;
            }
            present
        })
    }

    /// The first argument and optional fallback, split on a top-level comma.
    #[must_use]
    pub fn arguments(&self, index: usize) -> (Range<usize>, Option<Range<usize>>) {
        let item = &self.items[index];
        let end = item.end - usize::from(item.closed);
        let start = index + 1;
        let mut cursor = start;
        while cursor < end {
            if self.items[cursor].kind == ComponentKind::Comma {
                return (start..cursor, Some(cursor + 1..end));
            }
            cursor = self.items[cursor].end;
        }
        (start..end, None)
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn items(&self) -> &[Component] {
        &self.items
    }

    #[must_use]
    pub fn contains_var(&self) -> bool {
        self.items.iter().any(|item| matches!(&item.kind, ComponentKind::Function(name) if name.eq_ignore_ascii_case("var")))
    }
}

/// The computed value of an unregistered custom property (CSS Variables §2).
#[derive(Clone, Debug, PartialEq)]
pub enum ComputedCustom {
    Invalid,
    Value(ComponentValues),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_share_source_and_preserve_nested_ranges() {
        let source = r"f([001.0px], /*x*/ V\41R(--n, ))";
        let seq = ComponentValues::capture(source).unwrap();
        assert!(seq.contains_var() && seq.valid_variables());
        assert_eq!(seq.items[0].end, seq.items.len());
        assert!(matches!(seq.items[2].kind, ComponentKind::Dimension));
        assert_eq!(&seq.source[seq.items[2].span.clone()], "001.0px");
        let clone = seq.clone();
        assert!(Arc::ptr_eq(&clone.items, &seq.items) && Arc::ptr_eq(&clone.source, &seq.source));
    }
}
