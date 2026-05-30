//! The strict HTML tokenizer (WHATWG HTML §13.2.5 "Tokenization").
//!
//! Internal to the crate: [`Tokenizer`] produces a stream of [`Token`]
//! values consumed by the tree builder (A3, not yet landed). The public
//! [`crate::parse_strict`] entry point remains an A1 skeleton stub until
//! A4 wires the tokenizer and tree builder together.
//!
//! The tokenizer is `EcsDom`-unreachable by construction — it has no
//! dependency on `elidex-ecs` and emits only value tokens.

// A2 staging: the tokenizer is complete but not yet wired to its
// consumer. `parse_strict` still returns the A1 skeleton stub and only
// drives the tokenizer in A4, so under non-test compilation every item
// here is currently unreachable from the crate's public API (the unit
// tests exercise all of it). Remove this allow in A4.
#![allow(dead_code)]
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
