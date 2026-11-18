//! EH-attrs-at-creation T3-b: cell T-19 (plan
//! `docs/plans/2026-11-eh-attrs-t3b-body-prototype.md` §6.1) — `HTMLBodyElement`
//! is the interface of a `body` in the HTML namespace only (WHATWG HTML §3.2.2
//! Elements in the DOM, *element interface* step 4; §4.3.1 The body element;
//! DOM §4.5 Interface Document; Web IDL §3.7.6 Attributes).
//!
//! Foreign `body` elements come from the harness
//! (`with_page_and_foreign_bodies`): no script can create one.

#![cfg(feature = "engine")]

use super::super::test_helpers::{run, with_page_and_foreign_bodies};

// ---- T-19 (red; R-D1, FN-3, D-T11): a non-HTML `body` is not an HTMLBodyElement ----
#[test]
fn t19_non_html_body_is_not_an_html_body_element() {
    with_page_and_foreign_bodies(|vm| {
        let got = run(
            vm,
            "var BP = Object.getPrototypeOf(document.createElement('body')); \
             var D = Object.getOwnPropertyDescriptor(BP, 'onhashchange'); \
             r.push(Object.getPrototypeOf(svgBody) === BP); \
             svgBody.onhashchange = fn; \
             svgBody.dispatchEvent(new Event('hashchange')); \
             r.push(fnRan === 0, window.onhashchange === null); \
             mathBody.onhashchange = fn; \
             r.push(window.onhashchange === null); \
             try { D.get.call(svgBody); r.push(false) } catch (e) { r.push(e instanceof TypeError) }",
        );
        assert_eq!(
            got, "false,true,true,true,true",
            "T-19: a foreign `body` takes HTMLElement.prototype, its on<T> is an ordinary property, \
             and the body pair's accessor rejects it with a TypeError"
        );
    });
}
