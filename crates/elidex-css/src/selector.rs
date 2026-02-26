//! CSS selector parsing, matching, and specificity.
//!
//! Supports Phase 1 selector types: universal (`*`), tag, class, id,
//! descendant (space), and child (`>`) combinators.

use cssparser::{Parser, Token};
use elidex_ecs::{Attributes, EcsDom, Entity, TagType};

/// A single component of a CSS selector.
///
/// Components are stored right-to-left for efficient matching.
#[derive(Clone, Debug, PartialEq)]
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
}

/// A parsed CSS selector with its computed specificity.
#[derive(Clone, Debug, PartialEq)]
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

/// Parse a single selector from the token stream.
///
/// A selector is a sequence of compound selectors separated by combinators
/// (whitespace for descendant, `>` for child).
fn parse_one_selector(input: &mut Parser) -> Result<Selector, ()> {
    let mut components = Vec::new();
    let mut specificity = Specificity::default();

    // Parse the first compound selector.
    parse_compound_selector(input, &mut components, &mut specificity)?;

    loop {
        // Try child combinator.
        if input
            .try_parse(|i| -> Result<(), ()> {
                match i.next() {
                    Ok(&Token::Delim('>')) => Ok(()),
                    _ => Err(()),
                }
            })
            .is_ok()
        {
            components.push(SelectorComponent::Child);
            parse_compound_selector(input, &mut components, &mut specificity)?;
            continue;
        }

        // Try descendant combinator: if we can parse another compound selector
        // without an explicit combinator, whitespace was the separator.
        // We use a temporary vec to avoid corrupting `components` on failure.
        let mut tmp_components = Vec::new();
        let mut tmp_specificity = Specificity::default();
        let ok = input
            .try_parse(|i| parse_compound_selector(i, &mut tmp_components, &mut tmp_specificity))
            .is_ok();
        if ok {
            components.push(SelectorComponent::Descendant);
            components.extend(tmp_components);
            specificity.id = specificity.id.saturating_add(tmp_specificity.id);
            specificity.class = specificity.class.saturating_add(tmp_specificity.class);
            specificity.tag = specificity.tag.saturating_add(tmp_specificity.tag);
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

/// Parse a compound selector (e.g. `div.foo#bar`).
///
/// A compound selector starts with an optional tag/universal, followed by
/// zero or more class/id selectors (no whitespace between them).
///
/// In cssparser, whitespace is automatically consumed. To distinguish
/// compound boundaries, we only continue the compound when the next token
/// is a `.` (class) or `#` (ID hash) — these can directly follow a tag
/// without whitespace in CSS. An `Ident` token after whitespace starts a
/// new compound (descendant combinator).
fn parse_compound_selector(
    input: &mut Parser,
    components: &mut Vec<SelectorComponent>,
    specificity: &mut Specificity,
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

    // Parse class and ID selectors (these chain without whitespace).
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
        SelectorComponent::Universal => match_components(components, idx + 1, entity, dom),
        SelectorComponent::Tag(tag) => {
            let matches = dom
                .world()
                .get::<&TagType>(entity)
                .ok()
                .is_some_and(|t| t.0 == *tag);
            if matches {
                match_components(components, idx + 1, entity, dom)
            } else {
                false
            }
        }
        SelectorComponent::Class(class) => {
            let matches = dom
                .world()
                .get::<&Attributes>(entity)
                .ok()
                .is_some_and(|attrs| {
                    attrs
                        .get("class")
                        .is_some_and(|c| c.split_whitespace().any(|w| w == class.as_str()))
                });
            if matches {
                match_components(components, idx + 1, entity, dom)
            } else {
                false
            }
        }
        SelectorComponent::Id(id) => {
            let matches = dom
                .world()
                .get::<&Attributes>(entity)
                .ok()
                .is_some_and(|attrs| attrs.get("id") == Some(id.as_str()));
            if matches {
                match_components(components, idx + 1, entity, dom)
            } else {
                false
            }
        }
        SelectorComponent::Descendant => {
            // Walk up ancestors looking for a match (depth-limited to match EcsDom's MAX_ANCESTOR_DEPTH).
            const MAX_ANCESTOR_DEPTH: usize = 10_000;
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
        SelectorComponent::Child => {
            if let Some(parent) = dom.get_parent(entity) {
                match_components(components, idx + 1, parent, dom)
            } else {
                false
            }
        }
    }
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
        let sel_baz = parse_sel(".baz").unwrap();
        assert!(sel_foo.matches(e, &dom));
        assert!(sel_bar.matches(e, &dom));
        assert!(!sel_baz.matches(e, &dom));
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
}
