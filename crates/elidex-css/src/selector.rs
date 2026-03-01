//! CSS selector parsing, matching, and specificity.
//!
//! Supports: universal (`*`), tag, class, id, descendant (space), child (`>`),
//! adjacent sibling (`+`), general sibling (`~`), attribute selectors,
//! pseudo-classes (`:root`, `:first-child`, `:last-child`, `:only-child`,
//! `:empty`), and negation (`:not()`).

use cssparser::{Parser, ParserInput, Token};
use elidex_ecs::{Attributes, EcsDom, Entity, TagType};

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
    /// `[attr=value]` — exact match.
    Exact(String),
    /// `[attr~=value]` — whitespace-separated word match.
    Includes(String),
    /// `[attr|=value]` — exact or prefix with `-`.
    DashMatch(String),
    /// `[attr^=value]` — prefix match.
    Prefix(String),
    /// `[attr$=value]` — suffix match.
    Suffix(String),
    /// `[attr*=value]` — substring match.
    Substring(String),
}

/// A parsed CSS selector with its computed specificity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selector {
    /// Components stored right-to-left for efficient matching.
    pub components: Vec<SelectorComponent>,
    /// Computed specificity.
    pub specificity: Specificity,
}

/// CSS selector specificity `(id, class, tag)`.
///
/// Implements `Ord` for cascade ordering: higher specificity wins.
///
/// **Important:** Field declaration order matters — derived `Ord` compares
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

impl Selector {
    /// Check if this selector matches the given entity in the DOM.
    pub fn matches(&self, entity: Entity, dom: &EcsDom) -> bool {
        if self.components.is_empty() {
            return false;
        }
        match_components(&self.components, 0, entity, dom)
    }
}

/// Parse a comma-separated list of selectors.
#[must_use = "parsing result should be used"]
#[allow(clippy::result_unit_err)]
pub fn parse_selector_list(input: &mut Parser) -> Result<Vec<Selector>, ()> {
    let mut selectors = vec![parse_one_selector(input)?];
    while input
        .try_parse(|i| i.expect_comma().map_err(|_| ()))
        .is_ok()
    {
        selectors.push(parse_one_selector(input)?);
    }
    Ok(selectors)
}

/// Try to parse an explicit combinator delimiter (`>`, `+`, or `~`).
///
/// Returns `Some(combinator)` if one of the given `(char, SelectorComponent)` pairs
/// matches the next delimiter token, or `None` if no combinator is found.
fn try_parse_combinator(
    input: &mut Parser,
    combinators: &[(char, SelectorComponent)],
) -> Option<SelectorComponent> {
    for &(delim, ref component) in combinators {
        if input
            .try_parse(|i| -> Result<(), ()> {
                match i.next() {
                    Ok(&Token::Delim(c)) if c == delim => Ok(()),
                    _ => Err(()),
                }
            })
            .is_ok()
        {
            return Some(component.clone());
        }
    }
    None
}

/// Parse a single selector from the token stream.
///
/// A selector is a sequence of compound selectors separated by combinators
/// (whitespace for descendant, `>` for child, `+` for adjacent sibling,
/// `~` for general sibling).
fn parse_one_selector(input: &mut Parser) -> Result<Selector, ()> {
    let mut components = Vec::new();
    let mut specificity = Specificity::default();

    // Parse the first compound selector.
    parse_compound_selector(input, &mut components, &mut specificity, false)?;

    loop {
        // Try explicit combinators: > (child), + (adjacent sibling), ~ (general sibling).
        let explicit_combinators = [
            ('>', SelectorComponent::Child),
            ('+', SelectorComponent::AdjacentSibling),
            ('~', SelectorComponent::GeneralSibling),
        ];
        if let Some(combinator) = try_parse_combinator(input, &explicit_combinators) {
            components.push(combinator);
            parse_compound_selector(input, &mut components, &mut specificity, false)?;
            continue;
        }

        // Try descendant combinator: if we can parse another compound selector
        // without an explicit combinator, whitespace was the separator.
        // We use a temporary vec to avoid corrupting `components` on failure.
        let mut tmp_components = Vec::new();
        let mut tmp_specificity = Specificity::default();
        let ok = input
            .try_parse(|i| {
                parse_compound_selector(i, &mut tmp_components, &mut tmp_specificity, false)
            })
            .is_ok();
        if ok {
            components.push(SelectorComponent::Descendant);
            components.extend(tmp_components);
            specificity = specificity.saturating_add(tmp_specificity);
            continue;
        }

        break;
    }

    if components.is_empty() {
        return Err(());
    }

    components.reverse();
    Ok(Selector {
        components,
        specificity,
    })
}

/// Parse a compound selector (e.g. `div.foo#bar`, `[href]`, `:not(.x)`).
///
/// A compound selector starts with an optional tag/universal, followed by
/// zero or more class/id/pseudo-class/attribute selectors.
///
/// In cssparser, whitespace is automatically consumed. To distinguish
/// compound boundaries, we only continue the compound when the next token
/// is a `.` (class), `#` (ID hash), `:` (pseudo), or `[` (attribute) —
/// these can directly follow a tag without whitespace in CSS. An `Ident`
/// token after whitespace starts a new compound (descendant combinator).
fn parse_compound_selector(
    input: &mut Parser,
    components: &mut Vec<SelectorComponent>,
    specificity: &mut Specificity,
    in_negation: bool,
) -> Result<(), ()> {
    let start_len = components.len();

    // Try tag or universal first.
    // Safety: mutation of `components`/`specificity` only occurs on Ok paths,
    // so try_parse rollback on Err does not leave inconsistent state.
    let _ = input.try_parse(|i| -> Result<(), ()> {
        match i.next().map_err(|_| ())? {
            Token::Ident(ref name) => {
                components.push(SelectorComponent::Tag(name.to_ascii_lowercase()));
                specificity.tag = specificity.tag.saturating_add(1);
                Ok(())
            }
            Token::Delim('*') => {
                components.push(SelectorComponent::Universal);
                Ok(())
            }
            _ => Err(()),
        }
    });

    // Parse class, ID, pseudo-class, attribute, and :not() selectors.
    loop {
        let ok = input
            .try_parse(|i| -> Result<(), ()> {
                match i.next().map_err(|_| ())? {
                    Token::IDHash(ref name) => {
                        components.push(SelectorComponent::Id(name.as_ref().to_string()));
                        specificity.id = specificity.id.saturating_add(1);
                        Ok(())
                    }
                    Token::Delim('.') => {
                        let class_name = i.expect_ident().map_err(|_| ())?.as_ref().to_string();
                        components.push(SelectorComponent::Class(class_name));
                        specificity.class = specificity.class.saturating_add(1);
                        Ok(())
                    }
                    Token::Colon => {
                        parse_pseudo(i, components, specificity, in_negation)?;
                        Ok(())
                    }
                    Token::SquareBracketBlock => {
                        i.parse_nested_block(|block| {
                            let name = match block.expect_ident() {
                                Ok(n) => n.as_ref().to_ascii_lowercase(),
                                Err(e) => return Err(e.into()),
                            };
                            let Ok(matcher) = parse_attribute_matcher(block) else {
                                return Err(block.new_custom_error(()));
                            };
                            components.push(SelectorComponent::Attribute { name, matcher });
                            specificity.class = specificity.class.saturating_add(1);
                            Ok(())
                        })
                        .map_err(|_: cssparser::ParseError<'_, ()>| ())?;
                        Ok(())
                    }
                    _ => Err(()),
                }
            })
            .is_ok();
        if !ok {
            break;
        }
    }

    if components.len() > start_len {
        Ok(())
    } else {
        Err(())
    }
}

/// Parse a pseudo-class or functional pseudo-class (`:not()`).
///
/// Called after consuming the `Token::Colon`. When `in_negation` is true,
/// `:not()` is rejected (CSS Selectors Level 3 forbids nested `:not()`).
fn parse_pseudo(
    input: &mut Parser,
    components: &mut Vec<SelectorComponent>,
    specificity: &mut Specificity,
    in_negation: bool,
) -> Result<(), ()> {
    let parsed_not = !in_negation
        && input
            .try_parse(|inner| -> Result<(), ()> {
                match inner.next().map_err(|_| ())? {
                    Token::Function(ref name) if name.eq_ignore_ascii_case("not") => inner
                        .parse_nested_block(|block| {
                            let mut not_components = Vec::new();
                            let mut not_specificity = Specificity::default();
                            if parse_compound_selector(
                                block,
                                &mut not_components,
                                &mut not_specificity,
                                true,
                            )
                            .is_err()
                            {
                                return Err(block.new_custom_error(()));
                            }
                            components.push(SelectorComponent::Not(not_components));
                            // :not() specificity = argument specificity only.
                            *specificity = specificity.saturating_add(not_specificity);
                            Ok(())
                        })
                        .map_err(|_: cssparser::ParseError<'_, ()>| ()),
                    _ => Err(()),
                }
            })
            .is_ok();

    if !parsed_not {
        let pseudo_name = input
            .expect_ident()
            .map_err(|_| ())?
            .as_ref()
            .to_ascii_lowercase();
        components.push(SelectorComponent::PseudoClass(pseudo_name));
        specificity.class = specificity.class.saturating_add(1);
    }
    Ok(())
}

/// Parse the operator and value inside an attribute selector bracket.
///
/// Returns `None` for presence-only (`[attr]`), or `Some(matcher)` for
/// value-matching operators.
fn parse_attribute_matcher(input: &mut Parser) -> Result<Option<AttributeMatcher>, ()> {
    if input.is_exhausted() {
        return Ok(None);
    }

    let tok = input.next().map_err(|_| ())?;
    let op = match tok {
        Token::Delim('=') => "=",
        Token::IncludeMatch => "~=",
        Token::DashMatch => "|=",
        Token::PrefixMatch => "^=",
        Token::SuffixMatch => "$=",
        Token::SubstringMatch => "*=",
        _ => return Err(()),
    };

    let value = match input.next().map_err(|_| ())? {
        Token::Ident(ref s) | Token::QuotedString(ref s) => s.as_ref().to_string(),
        _ => return Err(()),
    };

    Ok(Some(match op {
        "=" => AttributeMatcher::Exact(value),
        "~=" => AttributeMatcher::Includes(value),
        "|=" => AttributeMatcher::DashMatch(value),
        "^=" => AttributeMatcher::Prefix(value),
        "$=" => AttributeMatcher::Suffix(value),
        "*=" => AttributeMatcher::Substring(value),
        _ => unreachable!(),
    }))
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/// Recursive right-to-left selector matching.
fn match_components(
    components: &[SelectorComponent],
    idx: usize,
    entity: Entity,
    dom: &EcsDom,
) -> bool {
    if idx >= components.len() {
        return true;
    }

    match &components[idx] {
        // Combinators — navigate the tree, then continue matching.
        SelectorComponent::Descendant => {
            use elidex_ecs::MAX_ANCESTOR_DEPTH;
            let mut current = dom.get_parent(entity);
            let mut depth = 0;
            while let Some(ancestor) = current {
                depth += 1;
                if depth > MAX_ANCESTOR_DEPTH {
                    return false;
                }
                if match_components(components, idx + 1, ancestor, dom) {
                    return true;
                }
                current = dom.get_parent(ancestor);
            }
            false
        }
        SelectorComponent::Child => dom
            .get_parent(entity)
            .is_some_and(|parent| match_components(components, idx + 1, parent, dom)),
        SelectorComponent::AdjacentSibling => prev_element_sibling(dom, entity)
            .is_some_and(|prev| match_components(components, idx + 1, prev, dom)),
        SelectorComponent::GeneralSibling => {
            let mut current = prev_element_sibling(dom, entity);
            while let Some(sib) = current {
                if match_components(components, idx + 1, sib, dom) {
                    return true;
                }
                current = prev_element_sibling(dom, sib);
            }
            false
        }
        SelectorComponent::Not(ref inner) => {
            let inner_matched = match_compound_forward(inner, entity, dom);
            !inner_matched && match_components(components, idx + 1, entity, dom)
        }
        // Simple selectors — delegate to shared helper.
        other => {
            match_simple(other, entity, dom) && match_components(components, idx + 1, entity, dom)
        }
    }
}

/// Match a single simple selector (non-combinator) against an entity.
fn match_simple(component: &SelectorComponent, entity: Entity, dom: &EcsDom) -> bool {
    match component {
        SelectorComponent::Universal => true,
        SelectorComponent::Tag(tag) => dom
            .world()
            .get::<&TagType>(entity)
            .ok()
            .is_some_and(|t| t.0 == *tag),
        SelectorComponent::Class(class) => {
            dom.world()
                .get::<&Attributes>(entity)
                .ok()
                .is_some_and(|attrs| {
                    attrs
                        .get("class")
                        .is_some_and(|c| c.split_whitespace().any(|w| w == class.as_str()))
                })
        }
        SelectorComponent::Id(id) => dom
            .world()
            .get::<&Attributes>(entity)
            .ok()
            .is_some_and(|attrs| attrs.get("id") == Some(id.as_str())),
        SelectorComponent::PseudoClass(ref name) => match_pseudo_class(name, entity, dom),
        SelectorComponent::Attribute { name, matcher } => {
            match_attr(name, matcher.as_ref(), entity, dom)
        }
        // Combinators and :not() handled in match_components.
        _ => false,
    }
}

/// Match a pseudo-class by name against an entity.
fn match_pseudo_class(name: &str, entity: Entity, dom: &EcsDom) -> bool {
    match name {
        "root" => is_root_element(entity, dom),
        "first-child" => dom
            .get_parent(entity)
            .is_some_and(|parent| first_element_child(dom, parent) == Some(entity)),
        "last-child" => dom
            .get_parent(entity)
            .is_some_and(|parent| last_element_child(dom, parent) == Some(entity)),
        "only-child" => dom.get_parent(entity).is_some_and(|parent| {
            first_element_child(dom, parent) == Some(entity)
                && last_element_child(dom, parent) == Some(entity)
        }),
        "empty" => dom.get_first_child(entity).is_none(),
        _ => false,
    }
}

/// Match an attribute selector against an entity.
///
/// Attribute names are compared case-sensitively. Both the selector name
/// (lowercased during parse) and the DOM attribute name (lowercased by
/// html5ever) are stored in lowercase, so this is effectively
/// case-insensitive for HTML documents.
fn match_attr(
    name: &str,
    matcher: Option<&AttributeMatcher>,
    entity: Entity,
    dom: &EcsDom,
) -> bool {
    dom.world()
        .get::<&Attributes>(entity)
        .ok()
        .is_some_and(|attrs| match matcher {
            None => attrs.get(name).is_some(),
            Some(m) => attrs.get(name).is_some_and(|v| match_attribute(m, v)),
        })
}

/// Match a compound selector in forward (parse) order.
///
/// Used for `:not()` inner selectors, which contain only simple selectors
/// (no combinators) stored in parse order.
fn match_compound_forward(components: &[SelectorComponent], entity: Entity, dom: &EcsDom) -> bool {
    components.iter().all(|c| match_simple(c, entity, dom))
}

/// Check if an attribute value matches the given matcher.
fn match_attribute(matcher: &AttributeMatcher, value: &str) -> bool {
    match matcher {
        AttributeMatcher::Exact(expected) => value == expected.as_str(),
        AttributeMatcher::Includes(word) => value.split_whitespace().any(|w| w == word.as_str()),
        AttributeMatcher::DashMatch(prefix) => {
            value == prefix.as_str()
                || (value.starts_with(prefix.as_str())
                    && value.as_bytes().get(prefix.len()) == Some(&b'-'))
        }
        AttributeMatcher::Prefix(p) => value.starts_with(p.as_str()),
        AttributeMatcher::Suffix(s) => value.ends_with(s.as_str()),
        AttributeMatcher::Substring(sub) => value.contains(sub.as_str()),
    }
}

// ---------------------------------------------------------------------------
// Element-only traversal helpers
// ---------------------------------------------------------------------------

/// Return the first child of `parent` that is an element (has `TagType`).
fn first_element_child(dom: &EcsDom, parent: Entity) -> Option<Entity> {
    let mut child = dom.get_first_child(parent);
    while let Some(c) = child {
        if dom.world().get::<&TagType>(c).is_ok() {
            return Some(c);
        }
        child = dom.get_next_sibling(c);
    }
    None
}

/// Return the last child of `parent` that is an element (has `TagType`).
fn last_element_child(dom: &EcsDom, parent: Entity) -> Option<Entity> {
    let mut child = dom.get_last_child(parent);
    while let Some(c) = child {
        if dom.world().get::<&TagType>(c).is_ok() {
            return Some(c);
        }
        child = dom.get_prev_sibling(c);
    }
    None
}

/// Return the previous sibling that is an element (has `TagType`).
fn prev_element_sibling(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    let mut current = dom.get_prev_sibling(entity);
    while let Some(sib) = current {
        if dom.world().get::<&TagType>(sib).is_ok() {
            return Some(sib);
        }
        current = dom.get_prev_sibling(sib);
    }
    None
}

/// Check if the entity is the root element (`<html>`).
///
/// The root element is the `<html>` tag whose parent is the document root
/// (an entity without a `TagType` component).
fn is_root_element(entity: Entity, dom: &EcsDom) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .ok()
        .is_some_and(|t| t.0 == "html")
        && dom
            .get_parent(entity)
            .is_some_and(|p| dom.world().get::<&TagType>(p).is_err())
}

/// Parse a comma-separated list of selectors from a string.
///
/// Convenience wrapper around [`parse_selector_list`] that handles
/// `ParserInput` / `Parser` creation internally, so callers don't need
/// a `cssparser` dependency.
#[must_use = "parsing result should be used"]
#[allow(clippy::result_unit_err)]
pub fn parse_selector_from_str(selector: &str) -> Result<Vec<Selector>, ()> {
    let mut input = ParserInput::new(selector);
    let mut parser = Parser::new(&mut input);
    parse_selector_list(&mut parser)
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use cssparser::ParserInput;

    fn parse_sel(css: &str) -> Result<Selector, ()> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_one_selector(&mut parser)
    }

    fn parse_list(css: &str) -> Result<Vec<Selector>, ()> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_selector_list(&mut parser)
    }

    #[test]
    fn parse_tag() {
        let sel = parse_sel("div").unwrap();
        assert_eq!(sel.components, vec![SelectorComponent::Tag("div".into())]);
    }

    #[test]
    fn parse_class() {
        let sel = parse_sel(".foo").unwrap();
        assert_eq!(sel.components, vec![SelectorComponent::Class("foo".into())]);
    }

    #[test]
    fn parse_id() {
        let sel = parse_sel("#bar").unwrap();
        assert_eq!(sel.components, vec![SelectorComponent::Id("bar".into())]);
    }

    #[test]
    fn parse_universal() {
        let sel = parse_sel("*").unwrap();
        assert_eq!(sel.components, vec![SelectorComponent::Universal]);
    }

    #[test]
    fn parse_compound() {
        let sel = parse_sel("div.foo#bar").unwrap();
        // Stored right-to-left: Id, Class, Tag (reversed from parse order)
        assert_eq!(
            sel.components,
            vec![
                SelectorComponent::Id("bar".into()),
                SelectorComponent::Class("foo".into()),
                SelectorComponent::Tag("div".into()),
            ]
        );
    }

    #[test]
    fn parse_descendant() {
        let sel = parse_sel("div p").unwrap();
        assert!(sel.components.contains(&SelectorComponent::Descendant));
        assert!(sel
            .components
            .contains(&SelectorComponent::Tag("div".into())));
        assert!(sel.components.contains(&SelectorComponent::Tag("p".into())));
    }

    #[test]
    fn parse_child() {
        let sel = parse_sel("div > p").unwrap();
        assert!(sel.components.contains(&SelectorComponent::Child));
        assert!(sel
            .components
            .contains(&SelectorComponent::Tag("div".into())));
        assert!(sel.components.contains(&SelectorComponent::Tag("p".into())));
    }

    #[test]
    fn parse_selector_list_test() {
        let sels = parse_list("div, p").unwrap();
        assert_eq!(sels.len(), 2);
    }

    #[test]
    fn specificity_tag() {
        let sel = parse_sel("div").unwrap();
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 0,
                class: 0,
                tag: 1
            }
        );
    }

    #[test]
    fn specificity_class() {
        let sel = parse_sel(".foo").unwrap();
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 0,
                class: 1,
                tag: 0
            }
        );
    }

    #[test]
    fn specificity_id() {
        let sel = parse_sel("#bar").unwrap();
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 1,
                class: 0,
                tag: 0
            }
        );
    }

    #[test]
    fn specificity_compound() {
        let sel = parse_sel("div.foo#bar").unwrap();
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 1,
                class: 1,
                tag: 1
            }
        );
    }

    #[test]
    fn specificity_ordering() {
        let id = Specificity {
            id: 1,
            class: 0,
            tag: 0,
        };
        let class = Specificity {
            id: 0,
            class: 1,
            tag: 0,
        };
        let tag = Specificity {
            id: 0,
            class: 0,
            tag: 1,
        };
        assert!(id > class);
        assert!(class > tag);
    }

    // --- DOM matching tests ---

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    fn elem_with_class(dom: &mut EcsDom, tag: &str, class: &str) -> Entity {
        let mut attrs = Attributes::default();
        attrs.set("class", class);
        dom.create_element(tag, attrs)
    }

    fn elem_with_attr(dom: &mut EcsDom, tag: &str, attr: &str, value: &str) -> Entity {
        let mut attrs = Attributes::default();
        attrs.set(attr, value);
        dom.create_element(tag, attrs)
    }

    #[test]
    fn match_tag_against_dom() {
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let sel = parse_sel("div").unwrap();
        assert!(sel.matches(div, &dom));

        let span = elem(&mut dom, "span");
        assert!(!sel.matches(span, &dom));
    }

    #[test]
    fn match_class_against_dom() {
        let mut dom = EcsDom::new();
        let e = elem_with_class(&mut dom, "div", "foo bar");
        let sel_foo = parse_sel(".foo").unwrap();
        let sel_bar = parse_sel(".bar").unwrap();
        let sel_absent = parse_sel(".baz").unwrap();
        assert!(sel_foo.matches(e, &dom));
        assert!(sel_bar.matches(e, &dom));
        assert!(!sel_absent.matches(e, &dom));
    }

    #[test]
    fn class_matching_is_case_sensitive() {
        let mut dom = EcsDom::new();
        let e = elem_with_class(&mut dom, "div", "foo");
        let sel = parse_sel(".Foo").unwrap();
        assert!(!sel.matches(e, &dom));
    }

    #[test]
    fn id_matching_is_case_sensitive() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("id", "main");
        let e = dom.create_element("div", attrs);
        let sel = parse_sel("#Main").unwrap();
        assert!(!sel.matches(e, &dom));
    }

    #[test]
    fn match_descendant() {
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let span = elem(&mut dom, "span");
        let p = elem(&mut dom, "p");
        dom.append_child(div, span);
        dom.append_child(span, p);

        let sel = parse_sel("div p").unwrap();
        assert!(sel.matches(p, &dom));
    }

    #[test]
    fn match_child_direct_only() {
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let span = elem(&mut dom, "span");
        let p = elem(&mut dom, "p");
        dom.append_child(div, span);
        dom.append_child(span, p);

        let sel_child = parse_sel("div > p").unwrap();
        // p's direct parent is span, not div.
        assert!(!sel_child.matches(p, &dom));

        let sel_direct = parse_sel("span > p").unwrap();
        assert!(sel_direct.matches(p, &dom));
    }

    // --- Pseudo-class tests (M3-0) ---

    #[test]
    fn parse_pseudo_class_root() {
        let sel = parse_sel(":root").unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::PseudoClass("root".into())]
        );
        // Pseudo-class has class-level specificity (0, 1, 0).
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 0,
                class: 1,
                tag: 0
            }
        );
    }

    #[test]
    fn parse_pseudo_class_with_tag() {
        let sel = parse_sel("html:root").unwrap();
        assert!(sel
            .components
            .contains(&SelectorComponent::PseudoClass("root".into())));
        assert!(sel
            .components
            .contains(&SelectorComponent::Tag("html".into())));
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 0,
                class: 1,
                tag: 1
            }
        );
    }

    #[test]
    fn match_root_pseudo_class() {
        let mut dom = EcsDom::new();
        let doc_root = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc_root, html);
        dom.append_child(html, body);

        let sel = parse_sel(":root").unwrap();
        assert!(sel.matches(html, &dom));
        assert!(!sel.matches(body, &dom));
    }

    #[test]
    fn root_requires_document_parent() {
        // An html element without a proper document root parent should not match :root.
        let mut dom = EcsDom::new();
        let html = dom.create_element("html", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(html, body);
        // html has no parent at all — :root requires parent to be non-element.
        let sel = parse_sel(":root").unwrap();
        assert!(!sel.matches(html, &dom));
    }

    // --- M3-3: Sibling combinator parse tests ---

    #[test]
    fn parse_adjacent_sibling() {
        let sel = parse_sel("h1 + p").unwrap();
        assert!(sel.components.contains(&SelectorComponent::AdjacentSibling));
        assert!(sel
            .components
            .contains(&SelectorComponent::Tag("h1".into())));
        assert!(sel.components.contains(&SelectorComponent::Tag("p".into())));
    }

    #[test]
    fn parse_general_sibling() {
        let sel = parse_sel("h1 ~ p").unwrap();
        assert!(sel.components.contains(&SelectorComponent::GeneralSibling));
        assert!(sel
            .components
            .contains(&SelectorComponent::Tag("h1".into())));
        assert!(sel.components.contains(&SelectorComponent::Tag("p".into())));
    }

    // --- M3-3: Attribute selector parse tests ---

    #[test]
    fn parse_attr_presence() {
        let sel = parse_sel("[href]").unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Attribute {
                name: "href".into(),
                matcher: None,
            }]
        );
    }

    #[test]
    fn parse_attr_exact() {
        let sel = parse_sel(r#"[type="text"]"#).unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Attribute {
                name: "type".into(),
                matcher: Some(AttributeMatcher::Exact("text".into())),
            }]
        );
    }

    #[test]
    fn parse_attr_includes() {
        let sel = parse_sel(r#"[class~="foo"]"#).unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Attribute {
                name: "class".into(),
                matcher: Some(AttributeMatcher::Includes("foo".into())),
            }]
        );
    }

    #[test]
    fn parse_attr_dash_match() {
        let sel = parse_sel(r#"[lang|="en"]"#).unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Attribute {
                name: "lang".into(),
                matcher: Some(AttributeMatcher::DashMatch("en".into())),
            }]
        );
    }

    #[test]
    fn parse_attr_prefix() {
        let sel = parse_sel(r#"[href^="https"]"#).unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Attribute {
                name: "href".into(),
                matcher: Some(AttributeMatcher::Prefix("https".into())),
            }]
        );
    }

    #[test]
    fn parse_attr_suffix() {
        let sel = parse_sel(r#"[href$=".pdf"]"#).unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Attribute {
                name: "href".into(),
                matcher: Some(AttributeMatcher::Suffix(".pdf".into())),
            }]
        );
    }

    #[test]
    fn parse_attr_substring() {
        let sel = parse_sel(r#"[title*="hello"]"#).unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Attribute {
                name: "title".into(),
                matcher: Some(AttributeMatcher::Substring("hello".into())),
            }]
        );
    }

    // --- M3-3: Structural pseudo-class parse tests ---

    #[test]
    fn parse_first_child() {
        let sel = parse_sel(":first-child").unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::PseudoClass("first-child".into())]
        );
    }

    #[test]
    fn parse_last_child() {
        let sel = parse_sel(":last-child").unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::PseudoClass("last-child".into())]
        );
    }

    #[test]
    fn parse_only_child() {
        let sel = parse_sel(":only-child").unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::PseudoClass("only-child".into())]
        );
    }

    #[test]
    fn parse_empty() {
        let sel = parse_sel(":empty").unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::PseudoClass("empty".into())]
        );
    }

    // --- M3-3: :not() parse tests ---

    #[test]
    fn parse_not_class() {
        let sel = parse_sel(":not(.foo)").unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Not(vec![SelectorComponent::Class(
                "foo".into()
            )])]
        );
    }

    #[test]
    fn parse_not_tag() {
        let sel = parse_sel(":not(div)").unwrap();
        assert_eq!(
            sel.components,
            vec![SelectorComponent::Not(vec![SelectorComponent::Tag(
                "div".into()
            )])]
        );
    }

    #[test]
    fn parse_nested_not_rejected() {
        // CSS Selectors Level 3: :not() cannot contain :not().
        assert!(parse_sel(":not(:not(.foo))").is_err());
    }

    // --- M3-3: Specificity tests ---

    #[test]
    fn specificity_attr_presence() {
        let sel = parse_sel("[attr]").unwrap();
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 0,
                class: 1,
                tag: 0
            }
        );
    }

    #[test]
    fn specificity_attr_value() {
        let sel = parse_sel(r"[attr=val]").unwrap();
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 0,
                class: 1,
                tag: 0
            }
        );
    }

    #[test]
    fn specificity_not_id() {
        // CSS Selectors Level 3: :not() specificity = argument specificity.
        // :not(#id) → (1, 0, 0), not (1, 1, 0).
        let sel = parse_sel(":not(#id)").unwrap();
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 1,
                class: 0,
                tag: 0
            }
        );
    }

    #[test]
    fn specificity_tag_first_child() {
        let sel = parse_sel("div:first-child").unwrap();
        assert_eq!(
            sel.specificity,
            Specificity {
                id: 0,
                class: 1,
                tag: 1
            }
        );
    }

    // --- M3-3: Sibling combinator matching tests ---

    #[test]
    fn match_adjacent_sibling() {
        // <div><h1/><p/></div> — `h1 + p` matches p.
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let h1 = elem(&mut dom, "h1");
        let p = elem(&mut dom, "p");
        dom.append_child(div, h1);
        dom.append_child(div, p);

        let sel = parse_sel("h1 + p").unwrap();
        assert!(sel.matches(p, &dom));
        // h1 has no previous sibling that is p.
        assert!(!sel.matches(h1, &dom));
    }

    #[test]
    fn match_adjacent_sibling_not_immediate() {
        // <div><h1/><span/><p/></div> — `h1 + p` should NOT match p
        // because span is between h1 and p.
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let h1 = elem(&mut dom, "h1");
        let span = elem(&mut dom, "span");
        let p = elem(&mut dom, "p");
        dom.append_child(div, h1);
        dom.append_child(div, span);
        dom.append_child(div, p);

        let sel = parse_sel("h1 + p").unwrap();
        assert!(!sel.matches(p, &dom));
    }

    #[test]
    fn match_general_sibling() {
        // <div><h1/><span/><p/></div> — `h1 ~ p` matches p.
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let h1 = elem(&mut dom, "h1");
        let span = elem(&mut dom, "span");
        let p = elem(&mut dom, "p");
        dom.append_child(div, h1);
        dom.append_child(div, span);
        dom.append_child(div, p);

        let sel = parse_sel("h1 ~ p").unwrap();
        assert!(sel.matches(p, &dom));
        // p before h1 should NOT match.
        assert!(!sel.matches(h1, &dom));
    }

    #[test]
    fn match_general_sibling_before() {
        // <div><p/><h1/></div> — `h1 ~ p` should NOT match p (p is before h1).
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let p = elem(&mut dom, "p");
        let h1 = elem(&mut dom, "h1");
        dom.append_child(div, p);
        dom.append_child(div, h1);

        let sel = parse_sel("h1 ~ p").unwrap();
        assert!(!sel.matches(p, &dom));
    }

    // --- M3-3: Attribute matching tests ---

    #[test]
    fn match_attr_presence() {
        let mut dom = EcsDom::new();
        let a = elem_with_attr(&mut dom, "a", "href", "https://example.com");
        let div = elem(&mut dom, "div");

        let sel = parse_sel("[href]").unwrap();
        assert!(sel.matches(a, &dom));
        assert!(!sel.matches(div, &dom));
    }

    #[test]
    fn match_attr_exact() {
        let mut dom = EcsDom::new();
        let input = elem_with_attr(&mut dom, "input", "type", "text");
        let checkbox = elem_with_attr(&mut dom, "input", "type", "checkbox");

        let sel = parse_sel(r#"[type="text"]"#).unwrap();
        assert!(sel.matches(input, &dom));
        assert!(!sel.matches(checkbox, &dom));
    }

    #[test]
    fn match_attr_includes() {
        let mut dom = EcsDom::new();
        let e1 = elem_with_class(&mut dom, "div", "foo bar");
        let e2 = elem_with_class(&mut dom, "div", "foobar");

        let sel = parse_sel(r#"[class~="foo"]"#).unwrap();
        assert!(sel.matches(e1, &dom)); // "foo bar" contains word "foo"
        assert!(!sel.matches(e2, &dom)); // "foobar" does not contain word "foo"
    }

    #[test]
    fn match_attr_dash_match() {
        let mut dom = EcsDom::new();
        let en = elem_with_attr(&mut dom, "div", "lang", "en");
        let en_us = elem_with_attr(&mut dom, "div", "lang", "en-US");
        let eng = elem_with_attr(&mut dom, "div", "lang", "eng");

        let sel = parse_sel(r#"[lang|="en"]"#).unwrap();
        assert!(sel.matches(en, &dom));
        assert!(sel.matches(en_us, &dom));
        assert!(!sel.matches(eng, &dom));
    }

    #[test]
    fn match_attr_prefix() {
        let mut dom = EcsDom::new();
        let https = elem_with_attr(&mut dom, "a", "href", "https://example.com");
        let http = elem_with_attr(&mut dom, "a", "href", "http://example.com");

        let sel = parse_sel(r#"[href^="https"]"#).unwrap();
        assert!(sel.matches(https, &dom));
        assert!(!sel.matches(http, &dom));
    }

    #[test]
    fn match_attr_suffix() {
        let mut dom = EcsDom::new();
        let pdf = elem_with_attr(&mut dom, "a", "href", "/doc/report.pdf");
        let html = elem_with_attr(&mut dom, "a", "href", "/page/index.html");

        let sel = parse_sel(r#"[href$=".pdf"]"#).unwrap();
        assert!(sel.matches(pdf, &dom));
        assert!(!sel.matches(html, &dom));
    }

    #[test]
    fn match_attr_substring() {
        let mut dom = EcsDom::new();
        let has_hello = elem_with_attr(&mut dom, "div", "title", "say hello world");
        let no_hello = elem_with_attr(&mut dom, "div", "title", "goodbye");

        let sel = parse_sel(r#"[title*="hello"]"#).unwrap();
        assert!(sel.matches(has_hello, &dom));
        assert!(!sel.matches(no_hello, &dom));
    }

    // --- M3-3: Structural pseudo-class matching tests ---

    #[test]
    fn match_first_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "ul");
        let li1 = elem(&mut dom, "li");
        let li2 = elem(&mut dom, "li");
        dom.append_child(parent, li1);
        dom.append_child(parent, li2);

        let sel = parse_sel(":first-child").unwrap();
        assert!(sel.matches(li1, &dom));
        assert!(!sel.matches(li2, &dom));
    }

    #[test]
    fn match_first_child_with_text_node_before() {
        // Text node before element — :first-child should still match the
        // first element child (text nodes are not elements).
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let text = dom.create_text("some text");
        let span = elem(&mut dom, "span");
        dom.append_child(parent, text);
        dom.append_child(parent, span);

        let sel = parse_sel(":first-child").unwrap();
        assert!(sel.matches(span, &dom));
    }

    #[test]
    fn match_last_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "ul");
        let li1 = elem(&mut dom, "li");
        let li2 = elem(&mut dom, "li");
        dom.append_child(parent, li1);
        dom.append_child(parent, li2);

        let sel = parse_sel(":last-child").unwrap();
        assert!(!sel.matches(li1, &dom));
        assert!(sel.matches(li2, &dom));
    }

    #[test]
    fn match_only_child() {
        let mut dom = EcsDom::new();
        let parent1 = elem(&mut dom, "div");
        let only = elem(&mut dom, "span");
        dom.append_child(parent1, only);

        let parent2 = elem(&mut dom, "div");
        let child1 = elem(&mut dom, "span");
        let child2 = elem(&mut dom, "span");
        dom.append_child(parent2, child1);
        dom.append_child(parent2, child2);

        let sel = parse_sel(":only-child").unwrap();
        assert!(sel.matches(only, &dom));
        assert!(!sel.matches(child1, &dom));
        assert!(!sel.matches(child2, &dom));
    }

    #[test]
    fn match_empty() {
        let mut dom = EcsDom::new();
        let empty_div = elem(&mut dom, "div");
        let non_empty_div = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");
        dom.append_child(non_empty_div, child);

        let sel = parse_sel(":empty").unwrap();
        assert!(sel.matches(empty_div, &dom));
        assert!(!sel.matches(non_empty_div, &dom));
    }

    #[test]
    fn match_empty_with_text_child() {
        // :empty should NOT match if there's a text node child.
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let text = dom.create_text("hello");
        dom.append_child(div, text);

        let sel = parse_sel(":empty").unwrap();
        assert!(!sel.matches(div, &dom));
    }

    // --- M3-3: :not() matching tests ---

    #[test]
    fn match_not_class() {
        let mut dom = EcsDom::new();
        let foo = elem_with_class(&mut dom, "div", "foo");
        let bar = elem_with_class(&mut dom, "div", "bar");

        let sel = parse_sel(":not(.foo)").unwrap();
        assert!(!sel.matches(foo, &dom));
        assert!(sel.matches(bar, &dom));
    }

    #[test]
    fn match_not_tag() {
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let span = elem(&mut dom, "span");

        let sel = parse_sel(":not(div)").unwrap();
        assert!(!sel.matches(div, &dom));
        assert!(sel.matches(span, &dom));
    }

    // --- M3-3: Sibling with text node skipping ---

    #[test]
    fn adjacent_sibling_skips_text_nodes() {
        // <div><h1/>text<p/></div> — `h1 + p` should match p
        // because text nodes are not elements and should be skipped.
        let mut dom = EcsDom::new();
        let div = elem(&mut dom, "div");
        let h1 = elem(&mut dom, "h1");
        let text = dom.create_text("between");
        let p = elem(&mut dom, "p");
        dom.append_child(div, h1);
        dom.append_child(div, text);
        dom.append_child(div, p);

        let sel = parse_sel("h1 + p").unwrap();
        assert!(sel.matches(p, &dom));
    }

    // --- M3-3: Complex combined selectors ---

    #[test]
    fn parse_compound_with_attr_and_class() {
        let sel = parse_sel(r#"input.required[type="text"]"#).unwrap();
        assert!(sel
            .components
            .contains(&SelectorComponent::Tag("input".into())));
        assert!(sel
            .components
            .contains(&SelectorComponent::Class("required".into())));
        assert!(sel.components.contains(&SelectorComponent::Attribute {
            name: "type".into(),
            matcher: Some(AttributeMatcher::Exact("text".into())),
        }));
    }

    #[test]
    fn match_child_first_child_combined() {
        // ul > li:first-child
        let mut dom = EcsDom::new();
        let ul = elem(&mut dom, "ul");
        let li1 = elem(&mut dom, "li");
        let li2 = elem(&mut dom, "li");
        dom.append_child(ul, li1);
        dom.append_child(ul, li2);

        let sel = parse_sel("ul > li:first-child").unwrap();
        assert!(sel.matches(li1, &dom));
        assert!(!sel.matches(li2, &dom));
    }
}
