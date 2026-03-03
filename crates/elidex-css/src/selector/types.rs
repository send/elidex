//! Selector type definitions: components, pseudo-elements, attribute matchers,
//! and specificity.

/// A CSS pseudo-element (`::before`, `::after`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PseudoElement {
    /// The `::before` pseudo-element (generated content before element).
    Before,
    /// The `::after` pseudo-element (generated content after element).
    After,
}

/// A single component of a CSS selector.
///
/// Components are stored right-to-left for efficient matching.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SelectorComponent {
    /// The universal selector (`*`).
    Universal,
    /// A tag/type selector (e.g. `div`). Always lowercase.
    Tag(String),
    /// A class selector (e.g. `.foo`).
    Class(String),
    /// An ID selector (e.g. `#bar`).
    Id(String),
    /// Descendant combinator (whitespace).
    Descendant,
    /// Child combinator (`>`).
    Child,
    /// Adjacent sibling combinator (`+`).
    AdjacentSibling,
    /// General sibling combinator (`~`).
    GeneralSibling,
    /// A pseudo-class selector (e.g. `:root`, `:first-child`).
    PseudoClass(String),
    /// Attribute selector (e.g. `[href]`, `[type="text"]`).
    Attribute {
        name: String,
        matcher: Option<AttributeMatcher>,
    },
    /// Negation pseudo-class `:not(selector)`.
    ///
    /// Contains a single compound selector (CSS Selectors Level 3).
    /// Components are stored in parse order (left-to-right), not reversed.
    Not(Vec<SelectorComponent>),
}

/// Attribute value matching operator.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AttributeMatcher {
    /// `[attr=value]` -- exact match.
    Exact(String),
    /// `[attr~=value]` -- whitespace-separated word match.
    Includes(String),
    /// `[attr|=value]` -- exact or prefix with `-`.
    DashMatch(String),
    /// `[attr^=value]` -- prefix match.
    Prefix(String),
    /// `[attr$=value]` -- suffix match.
    Suffix(String),
    /// `[attr*=value]` -- substring match.
    Substring(String),
}

/// CSS selector specificity `(id, class, tag)`.
///
/// Implements `Ord` for cascade ordering: higher specificity wins.
///
/// **Important:** Field declaration order matters -- derived `Ord` compares
/// fields top-to-bottom, so `id` takes highest priority, then `class`, then `tag`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Specificity {
    pub id: u16,
    pub class: u16,
    pub tag: u16,
}

impl Specificity {
    /// Component-wise saturating addition of two specificities.
    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        Self {
            id: self.id.saturating_add(other.id),
            class: self.class.saturating_add(other.class),
            tag: self.tag.saturating_add(other.tag),
        }
    }
}
