//! CSS Syntax §5.5 component values. Flat storage keeps deeply nested blocks
//! independent of the native call stack, including when the sequence is dropped.

use std::ops::Range;
use std::sync::{Arc, OnceLock};

use cssparser::{Parser, ParserInput, Token};

/// Immutable authored bytes plus a token index. Block openers point past their
/// children (and the closing token, when present). CSS Syntax closes blocks at EOF.
#[derive(Clone, Debug)]
pub struct ComponentValues {
    source: Arc<Source>,
    items: Arc<[Component]>,
}

/// Spliced values retain individual spellings until a consumer needs CSS text.
#[derive(Debug)]
enum Source {
    Authored(Arc<str>),
    Spliced {
        spellings: Vec<Spelling>,
        text: OnceLock<String>,
    },
}

#[derive(Clone, Debug)]
pub struct Spelling {
    text: Arc<str>,
    span: Range<usize>,
}

impl Spelling {
    pub fn as_str(&self) -> &str {
        &self.text[self.span.clone()]
    }

    #[must_use]
    pub fn owned(text: String) -> Self {
        let span = 0..text.len();
        Self {
            text: text.into(),
            span,
        }
    }
}

impl PartialEq for ComponentValues {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items && self.source() == other.source()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub kind: ComponentKind,
    pub span: Range<usize>,
    pub end: usize,
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
        let mut items: Vec<Component> = Vec::new();
        Self::walk_declaration(source, |token, span, open| {
            let index = items.len();
            if let Some(open) = open {
                items[open].end = index + 1;
                items[open].closed = true;
            }
            let kind = match token {
                Token::Function(name) => ComponentKind::Function(name.to_string()),
                Token::ParenthesisBlock => ComponentKind::Block(')'),
                Token::SquareBracketBlock => ComponentKind::Block(']'),
                Token::CurlyBracketBlock => ComponentKind::Block('}'),
                Token::CloseParenthesis => ComponentKind::Close(')'),
                Token::CloseSquareBracket => ComponentKind::Close(']'),
                Token::CloseCurlyBracket => ComponentKind::Close('}'),
                Token::BadString(_) | Token::BadUrl(_) => unreachable!("validated declaration"),
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
            items.push(Component {
                kind,
                span,
                end: index + 1,
                closed: false,
            });
        })?;
        let end = items.len();
        for item in &mut items {
            if !item.closed
                && matches!(
                    item.kind,
                    ComponentKind::Function(_) | ComponentKind::Block(_)
                )
            {
                item.end = end;
            }
        }
        Some(Self {
            source: Arc::new(Source::Authored(source.into())),
            items: items.into(),
        })
    }

    /// Inspect decoded var functions without owning token spellings or building an index.
    #[must_use]
    pub fn inspect_variables(source: &str) -> Option<bool> {
        Self::walk_declaration(source, |_, _, _| {})
    }

    /// CSS Syntax 3 §§5.5.8–10: consume component values and functions. Advance
    /// one token at a time to visit children and explicit closing delimiters.
    fn walk_declaration(
        source: &str,
        mut visit: impl FnMut(&Token, Range<usize>, Option<usize>),
    ) -> Option<bool> {
        let mut blocks = Vec::new();
        let mut pending = false;
        let mut offset = 0;
        let mut index = 0;
        while offset < source.len() {
            let mut input = ParserInput::new(&source[offset..]);
            let mut parser = Parser::new(&mut input);
            let token = parser
                .next_including_whitespace_and_comments()
                .ok()?
                .clone();
            let close = match &token {
                Token::CloseParenthesis => Some(')'),
                Token::CloseSquareBracket => Some(']'),
                Token::CloseCurlyBracket => Some('}'),
                _ => None,
            };
            let mut open = None;
            if let Some(close) = close {
                let (start, expected) = blocks.pop()?;
                if close != expected {
                    return None;
                }
                open = Some(start);
            } else {
                match &token {
                    Token::Function(name) => {
                        pending |= name.eq_ignore_ascii_case("var");
                        blocks.push((index, ')'));
                    }
                    Token::ParenthesisBlock => blocks.push((index, ')')),
                    Token::SquareBracketBlock => blocks.push((index, ']')),
                    Token::CurlyBracketBlock => blocks.push((index, '}')),
                    Token::BadString(_) | Token::BadUrl(_) => return None,
                    Token::Semicolon | Token::Delim('!') if blocks.is_empty() => return None,
                    _ => {}
                }
            }
            let next = offset + parser.position().byte_index();
            visit(&token, offset..next, open);
            offset = next;
            index += 1;
        }
        Some(pending)
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
            self.variable_arguments(i + 1..item.end - usize::from(item.closed))
                .is_some()
        })
    }

    /// Return the validated arguments of the var function at this index.
    #[must_use]
    pub fn arguments(&self, index: usize) -> (Range<usize>, Option<Range<usize>>) {
        let item = &self.items[index];
        self.variable_arguments(index + 1..item.end - usize::from(item.closed))
            .expect("validated var()")
    }

    /// Validate the var() argument grammar and split at its first top-level comma.
    /// Used both on admission and after early invocation (CSS Values 5 Appendix A).
    #[must_use]
    pub fn variable_arguments(
        &self,
        range: Range<usize>,
    ) -> Option<(Range<usize>, Option<Range<usize>>)> {
        let mut comma = None;
        let mut present = false;
        let mut cursor = range.start;
        while cursor < range.end {
            match self.items[cursor].kind {
                ComponentKind::Semicolon | ComponentKind::Delim('!') => return None,
                ComponentKind::Comma => {
                    comma.get_or_insert(cursor);
                }
                ComponentKind::Whitespace | ComponentKind::Comment => {}
                _ if comma.is_none() => present = true,
                _ => {}
            }
            cursor = self.items[cursor].end;
        }
        present.then(|| {
            (
                range.start..comma.unwrap_or(range.end),
                comma.map(|i| i + 1..range.end),
            )
        })
    }

    #[must_use]
    pub fn source(&self) -> &str {
        match self.source.as_ref() {
            Source::Authored(text) => text,
            Source::Spliced { spellings, text } => {
                text.get_or_init(|| spellings.iter().map(Spelling::as_str).collect())
            }
        }
    }

    #[must_use]
    pub fn spelling(&self, index: usize) -> Spelling {
        match self.source.as_ref() {
            Source::Authored(text) => Spelling {
                text: text.clone(),
                span: self.items[index].span.clone(),
            },
            Source::Spliced { spellings, .. } => spellings[index].clone(),
        }
    }

    /// Build the block index from existing components without tokenizing CSS again.
    pub fn from_splices(
        tokens: impl IntoIterator<Item = (ComponentKind, Spelling)>,
    ) -> Option<Self> {
        let mut items: Vec<Component> = Vec::new();
        let mut spellings = Vec::new();
        let mut blocks = Vec::new();
        let mut offset = 0;
        for (kind, spelling) in tokens {
            let i = items.len();
            match kind {
                ComponentKind::Function(_) => blocks.push((i, ')')),
                ComponentKind::Block(close) => blocks.push((i, close)),
                ComponentKind::Close(close) => {
                    let (open, expected) = blocks.pop()?;
                    if close != expected {
                        return None;
                    }
                    items[open].end = i + 1;
                    items[open].closed = true;
                }
                _ => {}
            }
            let next = offset + spelling.as_str().len();
            items.push(Component {
                kind,
                span: offset..next,
                end: i + 1,
                closed: false,
            });
            spellings.push(spelling);
            offset = next;
        }
        for (open, _) in blocks {
            items[open].end = items.len();
        }
        Some(Self {
            source: Arc::new(Source::Spliced {
                spellings,
                text: OnceLock::new(),
            }),
            items: items.into(),
        })
    }

    #[must_use]
    pub fn items(&self) -> &[Component] {
        &self.items
    }

    /// CSS Syntax 3 §§5.5.8, 5.5.10 consume a component value/function.
    /// Recognize the decoded var() function name (CSS Variables 1 §3) at any depth.
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
        assert_eq!(&seq.source()[seq.items[2].span.clone()], "001.0px");
        let clone = seq.clone();
        assert!(Arc::ptr_eq(&clone.items, &seq.items) && Arc::ptr_eq(&clone.source, &seq.source));
    }
}
