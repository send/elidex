//! CSS Syntax §5.5.6 and CSS Variables §3 declaration-value capture.

use cssparser::{Parser, Token};
use elidex_plugin::component_values::ComponentValues;
use elidex_plugin::CssValue;

/// Preserve complete component spellings and trim only top-level whitespace
/// tokens. A whitespace code point belonging to an escape stays in its token.
pub(crate) fn capture(input: &mut Parser) -> Option<ComponentValues> {
    let mut first = None;
    let mut last = input.position();
    loop {
        let state = input.state();
        let Ok(token) = input.next_including_whitespace_and_comments() else {
            break;
        };
        match token {
            Token::Semicolon | Token::Delim('!') => {
                input.reset(&state);
                break;
            }
            Token::WhiteSpace(_) => continue,
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                let _ = input.parse_nested_block(|nested| {
                    while nested.next_including_whitespace_and_comments().is_ok() {}
                    Ok::<_, cssparser::ParseError<'_, ()>>(())
                });
            }
            _ => {}
        }
        first.get_or_insert(state.position());
        last = input.position();
    }
    let source = first.map_or("", |start| input.slice(start..last));
    let components = ComponentValues::capture(source)?;
    components.valid_variables().then_some(components)
}

/// Parse a whole var function with its original first argument and fallback.
#[allow(clippy::result_unit_err)]
pub fn parse_var_function(input: &mut Parser) -> Result<CssValue, ()> {
    input.skip_whitespace();
    let start = input.position();
    input.expect_function_matching("var").map_err(|_| ())?;
    input
        .parse_nested_block(|args| {
            while args.next_including_whitespace_and_comments().is_ok() {}
            Ok::<_, cssparser::ParseError<'_, ()>>(())
        })
        .map_err(|_| ())?;
    let value = ComponentValues::capture(input.slice_from(start)).ok_or(())?;
    value
        .valid_variables()
        .then_some(CssValue::Pending(value))
        .ok_or(())
}
