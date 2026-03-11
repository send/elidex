//! Token and keyword types for the JavaScript lexer.

use crate::atom::Atom;
use crate::span::Span;

/// A single token produced by the lexer.
#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// All token kinds for ES2020+ JavaScript.
///
/// All variants are `Copy` thanks to `Atom` (interned string handle).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    // === Literals ===
    NumericLiteral(f64),
    BigIntLiteral(Atom),
    StringLiteral(Atom),
    /// Template: no substitutions `` `hello` ``
    TemplateNoSub {
        cooked: Option<Atom>,
        raw: Atom,
    },
    /// Template head `` `hello ${ ``
    TemplateHead {
        cooked: Option<Atom>,
        raw: Atom,
    },
    /// Template middle `` } middle ${ ``
    TemplateMiddle {
        cooked: Option<Atom>,
        raw: Atom,
    },
    /// Template tail `` } tail` ``
    TemplateTail {
        cooked: Option<Atom>,
        raw: Atom,
    },
    /// `RegExp` literal `/pattern/flags`
    RegExpLiteral {
        pattern: Atom,
        flags: Atom,
    },

    // === Identifiers & Keywords ===
    Identifier(Atom),
    /// Private identifier `#name` (ES2022)
    PrivateIdentifier(Atom),
    Keyword(Keyword),

    // === Punctuators ===
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Dot,       // .
    Ellipsis,  // ...
    Semicolon, // ;
    Comma,     // ,
    Colon,     // :
    Question,  // ?
    OptChain,  // ?.
    NullCoal,  // ??
    Arrow,     // =>

    // Comparison / equality
    Lt,       // <
    Gt,       // >
    LtEq,     // <=
    GtEq,     // >=
    EqEq,     // ==
    NotEq,    // !=
    StrictEq, // ===
    StrictNe, // !==

    // Arithmetic
    Plus,    // +
    Minus,   // -
    Star,    // *
    Exp,     // **
    Slash,   // /
    Percent, // %

    // Increment / Decrement
    PlusPlus,   // ++
    MinusMinus, // --

    // Bitwise
    Amp,   // &
    Pipe,  // |
    Caret, // ^
    Tilde, // ~
    Shl,   // <<
    Shr,   // >>
    UShr,  // >>>

    // Logical
    And, // &&
    Or,  // ||
    Not, // !

    // Assignment
    Eq,         // =
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    ExpEq,      // **=
    SlashEq,    // /=
    PercentEq,  // %=
    AmpEq,      // &=
    PipeEq,     // |=
    CaretEq,    // ^=
    ShlEq,      // <<=
    ShrEq,      // >>=
    UShrEq,     // >>>=
    AndEq,      // &&=
    OrEq,       // ||=
    NullCoalEq, // ??=

    // Hash for private identifiers is handled by PrivateIdentifier
    // At sign reserved for future decorators (Phase 4)
    Eof,
}

impl TokenKind {
    /// Whether this token could end an expression (used for regexp vs division).
    #[must_use]
    pub fn is_expression_end(&self) -> bool {
        matches!(
            self,
            Self::Identifier(_)
                | Self::NumericLiteral(_)
                | Self::BigIntLiteral(_)
                | Self::StringLiteral(_)
                | Self::TemplateNoSub { .. }
                | Self::TemplateTail { .. }
                | Self::RegExpLiteral { .. }
                | Self::PrivateIdentifier(_)
                | Self::RParen
                | Self::RBracket
                | Self::RBrace
                | Self::PlusPlus
                | Self::MinusMinus
                | Self::Keyword(
                    Keyword::This | Keyword::Super | Keyword::True | Keyword::False | Keyword::Null
                )
        )
    }
}

/// JavaScript keywords (reserved + contextual).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    // Reserved words
    Break,
    Case,
    Catch,
    Class,
    Const,
    Continue,
    Debugger,
    Default,
    Delete,
    Do,
    Else,
    Export,
    Extends,
    Finally,
    For,
    Function,
    If,
    Import,
    In,
    Instanceof,
    Let,
    New,
    Return,
    Super,
    Switch,
    This,
    Throw,
    Try,
    Typeof,
    Var,
    Void,
    While,
    With,

    // Contextual keywords (treated as identifiers by the lexer,
    // promoted to keywords by the parser in specific contexts)
    Async,
    Await,
    Yield,
    From,
    As,
    Get,
    Set,
    Of,
    Target,
    Meta,
    Static,

    // Strict-mode reserved (always reserved in strict; elidex always strict)
    Enum,
    Implements,
    Interface,
    Package,
    Private,
    Protected,
    Public,

    // Literal keywords
    True,
    False,
    Null,
}

impl Keyword {
    /// Return the string representation of this keyword.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Break => "break",
            Self::Case => "case",
            Self::Catch => "catch",
            Self::Class => "class",
            Self::Const => "const",
            Self::Continue => "continue",
            Self::Debugger => "debugger",
            Self::Default => "default",
            Self::Delete => "delete",
            Self::Do => "do",
            Self::Else => "else",
            Self::Export => "export",
            Self::Extends => "extends",
            Self::Finally => "finally",
            Self::For => "for",
            Self::Function => "function",
            Self::If => "if",
            Self::Import => "import",
            Self::In => "in",
            Self::Instanceof => "instanceof",
            Self::Let => "let",
            Self::New => "new",
            Self::Return => "return",
            Self::Super => "super",
            Self::Switch => "switch",
            Self::This => "this",
            Self::Throw => "throw",
            Self::Try => "try",
            Self::Typeof => "typeof",
            Self::Var => "var",
            Self::Void => "void",
            Self::While => "while",
            Self::With => "with",
            Self::Async => "async",
            Self::Await => "await",
            Self::Yield => "yield",
            Self::From => "from",
            Self::As => "as",
            Self::Get => "get",
            Self::Set => "set",
            Self::Of => "of",
            Self::Target => "target",
            Self::Meta => "meta",
            Self::Static => "static",
            Self::Enum => "enum",
            Self::Implements => "implements",
            Self::Interface => "interface",
            Self::Package => "package",
            Self::Private => "private",
            Self::Protected => "protected",
            Self::Public => "public",
            Self::True => "true",
            Self::False => "false",
            Self::Null => "null",
        }
    }

    /// Try to match a string to a reserved keyword.
    /// Contextual keywords (`async`, `await`, etc.) are returned as `Identifier`
    /// by the lexer and promoted by the parser.
    #[must_use]
    pub fn from_reserved(s: &str) -> Option<Self> {
        match s {
            "break" => Some(Self::Break),
            "case" => Some(Self::Case),
            "catch" => Some(Self::Catch),
            "class" => Some(Self::Class),
            "const" => Some(Self::Const),
            "continue" => Some(Self::Continue),
            "debugger" => Some(Self::Debugger),
            "default" => Some(Self::Default),
            "delete" => Some(Self::Delete),
            "do" => Some(Self::Do),
            "else" => Some(Self::Else),
            "export" => Some(Self::Export),
            "extends" => Some(Self::Extends),
            "finally" => Some(Self::Finally),
            "for" => Some(Self::For),
            "function" => Some(Self::Function),
            "if" => Some(Self::If),
            "import" => Some(Self::Import),
            "in" => Some(Self::In),
            "instanceof" => Some(Self::Instanceof),
            "let" => Some(Self::Let),
            "new" => Some(Self::New),
            "return" => Some(Self::Return),
            "super" => Some(Self::Super),
            "switch" => Some(Self::Switch),
            "this" => Some(Self::This),
            "throw" => Some(Self::Throw),
            "try" => Some(Self::Try),
            "typeof" => Some(Self::Typeof),
            "var" => Some(Self::Var),
            "void" => Some(Self::Void),
            "while" => Some(Self::While),
            "with" => Some(Self::With),
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "null" => Some(Self::Null),
            // Strict-mode reserved words (elidex always strict)
            "enum" => Some(Self::Enum),
            "implements" => Some(Self::Implements),
            "interface" => Some(Self::Interface),
            "package" => Some(Self::Package),
            "private" => Some(Self::Private),
            "protected" => Some(Self::Protected),
            "public" => Some(Self::Public),
            "static" => Some(Self::Static),
            // S3: `yield` is a reserved word in strict mode (§12.1.1)
            "yield" => Some(Self::Yield),
            // `undefined` is not a keyword — it's a global identifier that can be rebound
            _ => None,
        }
    }

    /// Match any keyword including contextual ones (used by the parser).
    #[must_use]
    pub fn from_str_any(s: &str) -> Option<Self> {
        Self::from_reserved(s).or(match s {
            "async" => Some(Self::Async),
            "await" => Some(Self::Await),
            "from" => Some(Self::From),
            "as" => Some(Self::As),
            "get" => Some(Self::Get),
            "set" => Some(Self::Set),
            "of" => Some(Self::Of),
            "target" => Some(Self::Target),
            "meta" => Some(Self::Meta),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_keywords() {
        assert_eq!(Keyword::from_reserved("let"), Some(Keyword::Let));
        assert_eq!(Keyword::from_reserved("class"), Some(Keyword::Class));
        assert_eq!(Keyword::from_reserved("true"), Some(Keyword::True));
        // `undefined` is not a keyword
        assert_eq!(Keyword::from_reserved("undefined"), None);
        // Contextual keywords are NOT returned by from_reserved
        assert_eq!(Keyword::from_reserved("async"), None);
        // S3: yield is reserved in strict mode
        assert_eq!(Keyword::from_reserved("yield"), Some(Keyword::Yield));
    }

    #[test]
    fn contextual_keywords() {
        assert_eq!(Keyword::from_str_any("async"), Some(Keyword::Async));
        assert_eq!(Keyword::from_str_any("yield"), Some(Keyword::Yield));
        assert_eq!(Keyword::from_str_any("from"), Some(Keyword::From));
        assert_eq!(Keyword::from_str_any("notAKeyword"), None);
    }

    #[test]
    fn strict_future_reserved_words() {
        // B1: These 6 words are reserved in strict mode (elidex always strict)
        for word in [
            "implements",
            "interface",
            "package",
            "private",
            "protected",
            "public",
        ] {
            assert!(
                Keyword::from_reserved(word).is_some(),
                "{word} should be a reserved keyword"
            );
        }
    }

    #[test]
    fn expression_end_tokens() {
        assert!(TokenKind::Identifier(Atom::EMPTY).is_expression_end());
        assert!(TokenKind::NumericLiteral(42.0).is_expression_end());
        assert!(TokenKind::RParen.is_expression_end());
        assert!(!TokenKind::Plus.is_expression_end());
        assert!(!TokenKind::LParen.is_expression_end());
    }
}
