//! Selector parsing: tokenization to `Selector` components.

use cssparser::{Parser, Token};

use super::types::{AttributeMatcher, PseudoElement, SelectorComponent, Specificity};
use super::Selector;

/// Maximum number of components in a single selector to prevent abuse via
/// deeply nested selectors (e.g. `div > div > div > ... × 10000`).
const MAX_SELECTOR_COMPONENTS: usize = 512;

/// Parse a single selector from the token stream.
///
/// A selector is a sequence of compound selectors separated by combinators
/// (whitespace for descendant, `>` for child, `+` for adjacent sibling,
/// `~` for general sibling).
pub(super) fn parse_one_selector(input: &mut Parser) -> Result<Selector, ()> {
    let mut components = Vec::new();
    let mut specificity = Specificity::default();
    let mut pseudo_element: Option<PseudoElement> = None;

    // Parse the first compound selector.
    parse_compound_selector(
        input,
        &mut components,
        &mut specificity,
        false,
        &mut pseudo_element,
    )?;

    // M3: ::slotted() acts as terminal — only ::before/::after may follow.
    let has_slotted = components
        .iter()
        .any(|c| matches!(c, SelectorComponent::Slotted(_)));

    // If a pseudo-element or ::slotted() was found, no more compounds are allowed.
    if pseudo_element.is_none() && !has_slotted {
        loop {
            if components.len() >= MAX_SELECTOR_COMPONENTS {
                return Err(());
            }
            // Try explicit combinators: > (child), + (adjacent sibling), ~ (general sibling).
            let explicit_combinators = [
                ('>', SelectorComponent::Child),
                ('+', SelectorComponent::AdjacentSibling),
                ('~', SelectorComponent::GeneralSibling),
            ];
            if let Some(combinator) = try_parse_combinator(input, &explicit_combinators) {
                components.push(combinator);
                parse_compound_selector(
                    input,
                    &mut components,
                    &mut specificity,
                    false,
                    &mut pseudo_element,
                )?;
                if pseudo_element.is_some() {
                    break;
                }
                // M3: ::slotted() terminates the selector.
                if components
                    .iter()
                    .any(|c| matches!(c, SelectorComponent::Slotted(_)))
                {
                    break;
                }
                continue;
            }

            // Try descendant combinator: if we can parse another compound selector
            // without an explicit combinator, whitespace was the separator.
            // We use a temporary vec to avoid corrupting `components` on failure.
            let mut tmp_components = Vec::new();
            let mut tmp_specificity = Specificity::default();
            let mut tmp_pseudo = None;
            let ok = input
                .try_parse(|i| {
                    parse_compound_selector(
                        i,
                        &mut tmp_components,
                        &mut tmp_specificity,
                        false,
                        &mut tmp_pseudo,
                    )
                })
                .is_ok();
            if ok {
                components.push(SelectorComponent::Descendant);
                components.extend(tmp_components);
                specificity = specificity.saturating_add(tmp_specificity);
                if tmp_pseudo.is_some() {
                    pseudo_element = tmp_pseudo;
                    break;
                }
                // M3: ::slotted() terminates the selector.
                if components
                    .iter()
                    .any(|c| matches!(c, SelectorComponent::Slotted(_)))
                {
                    break;
                }
                continue;
            }

            break;
        }
    }

    if components.is_empty() && pseudo_element.is_none() {
        return Err(());
    }

    components.reverse();
    Ok(Selector {
        components,
        specificity,
        pseudo_element,
    })
}

/// Parse a compound selector (e.g. `div.foo#bar`, `[href]`, `:not(.x)`).
///
/// A compound selector starts with an optional tag/universal, followed by
/// zero or more class/id/pseudo-class/attribute selectors.
///
/// In cssparser, whitespace is automatically consumed. To distinguish
/// compound boundaries, we only continue the compound when the next token
/// is a `.` (class), `#` (ID hash), `:` (pseudo), or `[` (attribute) --
/// these can directly follow a tag without whitespace in CSS. An `Ident`
/// token after whitespace starts a new compound (descendant combinator).
#[allow(clippy::too_many_lines)]
fn parse_compound_selector(
    input: &mut Parser,
    components: &mut Vec<SelectorComponent>,
    specificity: &mut Specificity,
    in_functional_pseudo: bool,
    pseudo_element: &mut Option<PseudoElement>,
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

    // Parse class, ID, pseudo-class, attribute, :not(), and pseudo-element selectors.
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
                        parse_pseudo(
                            i,
                            components,
                            specificity,
                            in_functional_pseudo,
                            pseudo_element,
                        )?;
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
        // If a pseudo-element was parsed, stop -- nothing may follow it.
        if pseudo_element.is_some() {
            break;
        }
        // M3: After ::slotted(), only ::before/::after may follow (CSS Scoping §6.1).
        if components
            .iter()
            .any(|c| matches!(c, SelectorComponent::Slotted(_)))
        {
            let _ = input.try_parse(|i| -> Result<(), ()> {
                match i.next().map_err(|_| ())? {
                    Token::Colon => {}
                    _ => return Err(()),
                }
                match i.next().map_err(|_| ())? {
                    Token::Colon => {}
                    _ => return Err(()),
                }
                let name = i
                    .expect_ident()
                    .map_err(|_| ())?
                    .as_ref()
                    .to_ascii_lowercase();
                match name.as_str() {
                    "before" => {
                        *pseudo_element = Some(PseudoElement::Before);
                        specificity.tag = specificity.tag.saturating_add(1);
                        Ok(())
                    }
                    "after" => {
                        *pseudo_element = Some(PseudoElement::After);
                        specificity.tag = specificity.tag.saturating_add(1);
                        Ok(())
                    }
                    _ => Err(()),
                }
            });
            break;
        }
    }

    if components.len() > start_len || pseudo_element.is_some() {
        Ok(())
    } else {
        Err(())
    }
}

/// Parse a pseudo-class, functional pseudo-class (`:not()`), or pseudo-element.
///
/// Called after consuming the first `Token::Colon`. When `in_functional_pseudo`
/// is true, `:not()` is rejected and pseudo-elements are suppressed. This flag
/// is set inside `:not()`, `:host()`, and `::slotted()`.
///
/// Pseudo-elements use `::` syntax (CSS3) or legacy single-colon (CSS2) for
/// `before` and `after`. They are stored in `pseudo_element`, not in `components`.
#[allow(clippy::too_many_lines)]
fn parse_pseudo(
    input: &mut Parser,
    components: &mut Vec<SelectorComponent>,
    specificity: &mut Specificity,
    in_functional_pseudo: bool,
    pseudo_element: &mut Option<PseudoElement>,
) -> Result<(), ()> {
    // Try `::pseudo-element` (double-colon syntax).
    if !in_functional_pseudo {
        // Try ::slotted(selector) functional pseudo-element.
        let parsed_slotted = input
            .try_parse(|i| -> Result<(), ()> {
                match i.next().map_err(|_| ())? {
                    Token::Colon => {}
                    _ => return Err(()),
                }
                match i.next().map_err(|_| ())? {
                    Token::Function(ref name) if name.eq_ignore_ascii_case("slotted") => i
                        .parse_nested_block(|block| {
                            // ::slotted() specificity = (0, 0, 1) + inner.
                            parse_functional_inner(
                                block,
                                components,
                                specificity,
                                SelectorComponent::Slotted,
                                (0, 1),
                            )
                        })
                        .map_err(|_: cssparser::ParseError<'_, ()>| ()),
                    _ => Err(()),
                }
            })
            .is_ok();
        if parsed_slotted {
            return Ok(());
        }

        let parsed_pe = input
            .try_parse(|i| -> Result<(), ()> {
                match i.next().map_err(|_| ())? {
                    Token::Colon => {}
                    _ => return Err(()),
                }
                let name = i
                    .expect_ident()
                    .map_err(|_| ())?
                    .as_ref()
                    .to_ascii_lowercase();
                match name.as_str() {
                    "before" => {
                        *pseudo_element = Some(PseudoElement::Before);
                        // Pseudo-elements contribute to tag-level specificity.
                        specificity.tag = specificity.tag.saturating_add(1);
                        Ok(())
                    }
                    "after" => {
                        *pseudo_element = Some(PseudoElement::After);
                        specificity.tag = specificity.tag.saturating_add(1);
                        Ok(())
                    }
                    _ => Err(()),
                }
            })
            .is_ok();
        if parsed_pe {
            return Ok(());
        }
    }

    // Try `:host` (plain identifier, no parentheses).
    // Allowed inside :not() per CSS Selectors L4 §4.3.
    let parsed_host = input
        .try_parse(|i| -> Result<(), ()> {
            let name = i.expect_ident().map_err(|_| ())?;
            if name.eq_ignore_ascii_case("host") {
                components.push(SelectorComponent::Host);
                // :host specificity = (0, 1, 0).
                specificity.class = specificity.class.saturating_add(1);
                Ok(())
            } else {
                Err(())
            }
        })
        .is_ok();

    if parsed_host {
        return Ok(());
    }

    // Try functional pseudo-classes: `:not()`, `:host()`.
    // :not() cannot be nested (CSS Selectors L3 §4.3.6), but :host() is
    // allowed inside :not() (CSS Selectors L4 §4.3).
    let parsed_functional = input
        .try_parse(|inner| -> Result<(), ()> {
            match inner.next().map_err(|_| ())? {
                Token::Function(ref name)
                    if name.eq_ignore_ascii_case("not") && !in_functional_pseudo =>
                {
                    inner
                        .parse_nested_block(|block| {
                            // :not() specificity = argument specificity only.
                            parse_functional_inner(
                                block,
                                components,
                                specificity,
                                SelectorComponent::Not,
                                (0, 0),
                            )
                        })
                        .map_err(|_: cssparser::ParseError<'_, ()>| ())
                }
                Token::Function(ref name) if name.eq_ignore_ascii_case("host") => inner
                    .parse_nested_block(|block| {
                        // :host() specificity = (0, 1, 0) + inner.
                        parse_functional_inner(
                            block,
                            components,
                            specificity,
                            SelectorComponent::HostFunction,
                            (1, 0),
                        )
                    })
                    .map_err(|_: cssparser::ParseError<'_, ()>| ()),
                _ => Err(()),
            }
        })
        .is_ok();

    if !parsed_functional {
        let pseudo_name = input
            .expect_ident()
            .map_err(|_| ())?
            .as_ref()
            .to_ascii_lowercase();

        // Legacy single-colon pseudo-elements (CSS2 `:before` / `:after`).
        if !in_functional_pseudo {
            match pseudo_name.as_str() {
                "before" => {
                    *pseudo_element = Some(PseudoElement::Before);
                    specificity.tag = specificity.tag.saturating_add(1);
                    return Ok(());
                }
                "after" => {
                    *pseudo_element = Some(PseudoElement::After);
                    specificity.tag = specificity.tag.saturating_add(1);
                    return Ok(());
                }
                _ => {}
            }
        }

        components.push(SelectorComponent::PseudoClass(pseudo_name));
        specificity.class = specificity.class.saturating_add(1);
    }
    Ok(())
}

/// Parse a compound selector inside a functional pseudo-class/element block.
///
/// Shared by `:host()`, `::slotted()`, and `:not()`. Parses the inner compound
/// selector and pushes the constructed component, adding specificity.
fn parse_functional_inner<'i>(
    block: &mut Parser<'i, '_>,
    components: &mut Vec<SelectorComponent>,
    specificity: &mut Specificity,
    make_component: fn(Vec<SelectorComponent>) -> SelectorComponent,
    base_specificity: (u16, u16),
) -> Result<(), cssparser::ParseError<'i, ()>> {
    let mut inner_components = Vec::new();
    let mut inner_specificity = Specificity::default();
    let mut inner_pseudo = None;
    if parse_compound_selector(
        block,
        &mut inner_components,
        &mut inner_specificity,
        true,
        &mut inner_pseudo,
    )
    .is_err()
    {
        return Err(block.new_custom_error(()));
    }
    components.push(make_component(inner_components));
    specificity.class = specificity.class.saturating_add(base_specificity.0);
    specificity.tag = specificity.tag.saturating_add(base_specificity.1);
    *specificity = specificity.saturating_add(inner_specificity);
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
        _ => return Err(()),
    }))
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
