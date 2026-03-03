//! Selector parsing: tokenization to `Selector` components.

use cssparser::{Parser, Token};

use super::types::{AttributeMatcher, PseudoElement, SelectorComponent, Specificity};
use super::Selector;

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

    // If a pseudo-element was found, no more compounds are allowed.
    if pseudo_element.is_none() {
        loop {
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
fn parse_compound_selector(
    input: &mut Parser,
    components: &mut Vec<SelectorComponent>,
    specificity: &mut Specificity,
    in_negation: bool,
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
                        parse_pseudo(i, components, specificity, in_negation, pseudo_element)?;
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
    }

    if components.len() > start_len || pseudo_element.is_some() {
        Ok(())
    } else {
        Err(())
    }
}

/// Parse a pseudo-class, functional pseudo-class (`:not()`), or pseudo-element.
///
/// Called after consuming the first `Token::Colon`. When `in_negation` is true,
/// `:not()` is rejected (CSS Selectors Level 3 forbids nested `:not()`).
///
/// Pseudo-elements use `::` syntax (CSS3) or legacy single-colon (CSS2) for
/// `before` and `after`. They are stored in `pseudo_element`, not in `components`.
fn parse_pseudo(
    input: &mut Parser,
    components: &mut Vec<SelectorComponent>,
    specificity: &mut Specificity,
    in_negation: bool,
    pseudo_element: &mut Option<PseudoElement>,
) -> Result<(), ()> {
    // Try `::pseudo-element` (double-colon syntax).
    if !in_negation {
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

    // Try `:not(...)` functional pseudo-class.
    let parsed_not = !in_negation
        && input
            .try_parse(|inner| -> Result<(), ()> {
                match inner.next().map_err(|_| ())? {
                    Token::Function(ref name) if name.eq_ignore_ascii_case("not") => inner
                        .parse_nested_block(|block| {
                            let mut not_components = Vec::new();
                            let mut not_specificity = Specificity::default();
                            let mut not_pseudo = None;
                            if parse_compound_selector(
                                block,
                                &mut not_components,
                                &mut not_specificity,
                                true,
                                &mut not_pseudo,
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

        // Legacy single-colon pseudo-elements (CSS2 `:before` / `:after`).
        if !in_negation {
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
