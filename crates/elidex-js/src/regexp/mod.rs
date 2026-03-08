//! `RegExp` pattern internal syntax parser.
//!
//! Parses the pattern string of a `/pattern/flags` literal into a `RegExpNode` AST.
//! Validates flags. Used by the lexer after tokenizing the regexp literal.

mod class_set;
mod escape;
mod parser;
mod unicode_property;

use std::fmt;

/// A node in the `RegExp` pattern AST.
#[derive(Debug, Clone, PartialEq)]
pub enum RegExpNode {
    /// Literal character.
    Literal(char),
    /// `.` (any character).
    Dot,
    /// Character class `[...]` or `[^...]`.
    CharClass {
        negated: bool,
        ranges: Vec<CharRange>,
    },
    /// Group `(...)`.
    Group {
        kind: GroupKind,
        body: Box<RegExpNode>,
    },
    /// Quantifier `*`, `+`, `?`, `{n,m}`.
    Quantifier {
        body: Box<RegExpNode>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    },
    /// Assertion `^`, `$`, `\b`, `\B`, lookahead, lookbehind.
    Assertion(AssertionKind),
    /// Sequence of nodes (implicit concatenation).
    Alternative(Vec<RegExpNode>),
    /// Alternation `|`.
    Disjunction(Vec<RegExpNode>),
    /// Backreference `\1`.
    Backreference(u32),
    /// Named backreference `\k<name>`.
    NamedBackreference(String),
    /// Unicode property escape `\p{...}` / `\P{...}`.
    UnicodeProperty {
        name: String,
        value: Option<String>,
        negated: bool,
    },
    /// Character escape `\d`, `\w`, `\s`, etc.
    Escape(EscapeKind),
    /// Set operation (v flag): intersection `&&` or subtraction `--`.
    ClassSetExpression {
        op: ClassSetOp,
        operands: Vec<RegExpNode>,
    },
}

/// Set operation kind (v flag).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClassSetOp {
    Intersection,
    Subtraction,
}

/// Group kind.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupKind {
    Capturing,
    Named(String),
    NonCapturing,
}

/// Assertion kind.
#[derive(Debug, Clone, PartialEq)]
pub enum AssertionKind {
    /// `^`
    Start,
    /// `$`
    End,
    /// `\b`
    WordBoundary,
    /// `\B`
    NonWordBoundary,
    /// `(?=...)`
    Lookahead(Box<RegExpNode>),
    /// `(?!...)`
    NegativeLookahead(Box<RegExpNode>),
    /// `(?<=...)` (ES2018)
    Lookbehind(Box<RegExpNode>),
    /// `(?<!...)` (ES2018)
    NegativeLookbehind(Box<RegExpNode>),
}

/// Character escape kind.
#[derive(Debug, Clone, PartialEq)]
pub enum EscapeKind {
    Digit,          // \d
    NonDigit,       // \D
    Word,           // \w
    NonWord,        // \W
    Whitespace,     // \s
    NonWhitespace,  // \S
    Tab,            // \t
    Newline,        // \n
    CarriageReturn, // \r
    FormFeed,       // \f
    VerticalTab,    // \v
    Null,           // \0
    Hex(char),      // \xHH
    Unicode(char),  // \uHHHH or \u{HHHHH}
    Identity(char), // escaped literal char
}

impl EscapeKind {
    /// Convert to a concrete character, if this escape represents one.
    /// Returns `None` for class escapes (`\d`, `\w`, `\s` and their negations).
    pub fn to_char(&self) -> Option<char> {
        match self {
            Self::Hex(c) | Self::Unicode(c) | Self::Identity(c) => Some(*c),
            Self::Tab => Some('\t'),
            Self::Newline => Some('\n'),
            Self::CarriageReturn => Some('\r'),
            Self::FormFeed => Some('\x0C'),
            Self::VerticalTab => Some('\x0B'),
            Self::Null => Some('\0'),
            Self::Digit | Self::NonDigit | Self::Word | Self::NonWord | Self::Whitespace | Self::NonWhitespace => None,
        }
    }
}

/// A range in a character class.
#[derive(Debug, Clone, PartialEq)]
pub enum CharRange {
    Single(CharClassAtom),
    Range(CharClassAtom, CharClassAtom),
}

/// An atom in a character class.
#[derive(Debug, Clone, PartialEq)]
pub enum CharClassAtom {
    Literal(char),
    Escape(EscapeKind),
    /// Nested character class (v flag only): `[[a-z]]`
    NestedClass(Box<RegExpNode>),
    /// String alternative `\q{abc|def}` (v flag only)
    StringAlternative(Vec<Vec<char>>),
}

/// Validated regexp flags.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default)]
pub struct RegExpFlags {
    pub global: bool,
    pub ignore_case: bool,
    pub multiline: bool,
    pub dot_all: bool,
    pub unicode: bool,
    pub unicode_sets: bool,
    pub sticky: bool,
    pub has_indices: bool,
}

/// `RegExp` parse error.
#[derive(Debug, Clone)]
pub struct RegExpError {
    pub message: String,
    pub offset: usize,
}

impl fmt::Display for RegExpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RegExp error at {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for RegExpError {}

/// Parse and validate regexp flags.
pub fn parse_flags(flags: &str) -> Result<RegExpFlags, RegExpError> {
    let mut result = RegExpFlags::default();

    macro_rules! set_flag {
        ($field:ident, $ch:expr, $i:expr) => {{
            if result.$field {
                return Err(RegExpError {
                    message: format!("Duplicate flag '{}'", $ch),
                    offset: $i,
                });
            }
            result.$field = true;
        }};
    }

    for (i, c) in flags.chars().enumerate() {
        match c {
            'g' => set_flag!(global, c, i),
            'i' => set_flag!(ignore_case, c, i),
            'm' => set_flag!(multiline, c, i),
            's' => set_flag!(dot_all, c, i),
            'u' => set_flag!(unicode, c, i),
            'v' => set_flag!(unicode_sets, c, i),
            'y' => set_flag!(sticky, c, i),
            'd' => set_flag!(has_indices, c, i),
            _ => {
                return Err(RegExpError {
                    message: format!("Invalid flag '{c}'"),
                    offset: i,
                });
            }
        }
    }
    if result.unicode && result.unicode_sets {
        return Err(RegExpError {
            message: "Flags 'u' and 'v' are mutually exclusive".into(),
            offset: 0,
        });
    }
    Ok(result)
}

/// Parse a regexp pattern string into an AST.
pub fn parse_pattern(pattern: &str) -> Result<RegExpNode, RegExpError> {
    parse_pattern_with_flags(pattern, &RegExpFlags::default())
}

/// Parse a regexp pattern string with flags context.
pub fn parse_pattern_with_flags(
    pattern: &str,
    flags: &RegExpFlags,
) -> Result<RegExpNode, RegExpError> {
    let mut p = parser::RegExpParser::new(pattern, flags);
    let node = p.parse_disjunction()?;
    if p.pos < p.source_len() {
        return Err(RegExpError {
            message: "Unexpected character in pattern".into(),
            offset: p.pos,
        });
    }
    // B3/B4: validate backreferences after full parse (allows forward references)
    p.validate_backreferences()?;
    Ok(node)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
