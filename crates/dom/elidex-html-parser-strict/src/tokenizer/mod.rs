//! The strict HTML tokenizer (WHATWG HTML §13.2.5 "Tokenization").
//!
//! Internal to the crate: [`Tokenizer`] produces a stream of [`Token`]
//! values consumed by the tree builder, driven by the public
//! [`crate::parse_strict`] entry point.
//!
//! The tokenizer is `EcsDom`-unreachable by construction — it has no
//! dependency on `elidex-ecs` and emits only value tokens.

// Every state handler shares one signature — `fn(&mut self) ->
// Result<(), StrictParseError>` — so the §13.2.5 dispatch match is
// uniform and any state may begin rejecting without a signature change.
// Handlers that cannot currently fail therefore always return `Ok(())`.
#![allow(clippy::unnecessary_wraps)]

pub(crate) mod char_ref;
pub(crate) mod states;
pub(crate) mod token;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_html5lib;
