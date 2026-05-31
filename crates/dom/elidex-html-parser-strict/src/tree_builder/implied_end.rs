//! WHATWG HTML §13.2.6.3 "Closing elements that have implied end tags".

use super::TreeBuilder;

/// Elements popped by "generate implied end tags" (§13.2.6.3).
const IMPLIED_END_TAGS: &[&str] = &[
    "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc",
];

/// Additional elements popped by "generate all implied end tags thoroughly".
const THOROUGH_END_TAGS: &[&str] = &[
    "caption", "colgroup", "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc",
    "tbody", "td", "tfoot", "th", "thead", "tr",
];

impl TreeBuilder {
    /// §13.2.6.3 "generate implied end tags": while the current node is one of
    /// the implied-end-tag elements, pop it off the stack of open elements.
    pub(super) fn generate_implied_end_tags(&mut self) {
        self.generate_implied_end_tags_except("");
    }

    /// §13.2.6.3 "generate implied end tags" excluding `exclude` from the
    /// element list (the spec's "except for _tag_ elements"). Pass `""` to
    /// exclude nothing.
    pub(super) fn generate_implied_end_tags_except(&mut self, exclude: &str) {
        while let Some(node) = self.state.current_node() {
            let should_pop = self.dom.with_tag_name(node, |tag| {
                matches!(tag, Some(name) if name != exclude && IMPLIED_END_TAGS.contains(&name))
            });
            if should_pop {
                self.pop();
            } else {
                break;
            }
        }
    }

    /// §13.2.6.3 "generate all implied end tags thoroughly": as above but over
    /// the wider element list (adds caption / colgroup / table-section / cell /
    /// row elements). Used when closing a `<template>`.
    pub(super) fn generate_all_implied_end_tags_thoroughly(&mut self) {
        while let Some(node) = self.state.current_node() {
            let should_pop = self.dom.with_tag_name(
                node,
                |tag| matches!(tag, Some(name) if THOROUGH_END_TAGS.contains(&name)),
            );
            if should_pop {
                self.pop();
            } else {
                break;
            }
        }
    }
}
