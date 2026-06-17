//! WHATWG HTML §6.6.2/§6.6.3 focusable-area predicates — the attribute-based,
//! engine- and form-independent evaluation of "is this entity a focusable area"
//! and the per-element default `tabIndex`. The write-side gate of the
//! `FOCUS`-set ⟹ connected/focusable invariant maintained by the SoT writers.

// Cohesive `focus` module split: the submodules share the parent namespace.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Per-element default `tabIndex` value (WHATWG HTML §6.6.3 "tabindex value" —
/// the value when no `tabindex` content attribute is present): `0` for
/// intrinsically focusable areas (button / select / textarea / iframe / object
/// / embed, `<a>`/`<area>` with `href`, `<input>` other than `type=hidden`, the
/// first `<summary>` of a `<details>`, and editing hosts — an own
/// `contenteditable` in the true/plaintext-only state), `-1` otherwise.
///
/// These §6.6.3 UA-determined defaults are **HTML-namespace only**: a foreign
/// (SVG / MathML) element whose local name happens to match an HTML control —
/// e.g. parser-created `<svg><button>` or an SVG-namespaced `<input>` — is not
/// an HTML control and gets `-1`. The engine treats namespace as load-bearing
/// (form-control state creation and `datalist` resolution gate on
/// `EcsDom::is_html_namespace` too). An explicit `tabindex` still grants
/// focusability cross-namespace via [`is_focusable`]'s separate branch (the
/// attribute is global); only this per-element *default* is HTML-only.
///
/// Backs the `tabIndex` IDL getter — it reflects the default tab *order* and is
/// independent of disabled state (a disabled `<button>` still has `tabIndex`
/// `0`); see [`is_focusable`] for the focusability decision, which does honour
/// `disabled`.
#[must_use]
pub fn tab_index_default_for(dom: &EcsDom, entity: Entity) -> i32 {
    // Tag-driven branch decisions read the borrowed tag directly so the
    // lowercase comparison is zero-allocation; an explicit `Option<TagDefault>`
    // enum lets the inner `dom.with_attribute` / `dom.has_attribute` calls run
    // AFTER the tag borrow drops.
    enum TagDefault {
        // Definitely focus-zero (button / select / textarea / iframe / object
        // / embed) — no further attribute lookup needed.
        Zero,
        // Link — focus-zero only when the element also carries `href`.
        Link,
        // `<input>` — focus-zero unless `type="hidden"`.
        Input,
        // `<summary>` — focus-zero only as the first summary child of a details.
        Summary,
        // Generic element — depends on `contenteditable`.
        Generic,
    }
    // §6.6.3 UA-determined focus defaults apply only to HTML-namespace elements:
    // a foreign (SVG / MathML) look-alike whose local name matches an HTML
    // control is not an HTML control, so it gets no per-element default (`-1`).
    // An explicit `tabindex` still makes it focusable via `is_focusable`'s
    // separate branch (the attribute is global). `is_html_namespace` gates on
    // `is_element`, so non-element entities also return `-1` here.
    if !dom.is_html_namespace(entity) {
        return -1;
    }
    let kind = dom.with_tag_name(entity, |t| match t {
        None => None,
        Some(s) => {
            if s.eq_ignore_ascii_case("button")
                || s.eq_ignore_ascii_case("select")
                || s.eq_ignore_ascii_case("textarea")
                || s.eq_ignore_ascii_case("iframe")
                || s.eq_ignore_ascii_case("object")
                || s.eq_ignore_ascii_case("embed")
            {
                Some(TagDefault::Zero)
            } else if s.eq_ignore_ascii_case("a") || s.eq_ignore_ascii_case("area") {
                Some(TagDefault::Link)
            } else if s.eq_ignore_ascii_case("input") {
                Some(TagDefault::Input)
            } else if s.eq_ignore_ascii_case("summary") {
                Some(TagDefault::Summary)
            } else {
                Some(TagDefault::Generic)
            }
        }
    });
    let focusable = match kind {
        None => false,
        Some(TagDefault::Zero) => true,
        Some(TagDefault::Link) => dom.has_attribute(entity, "href"),
        Some(TagDefault::Input) => {
            // `<input type="hidden">` is unfocusable; everything else
            // participates in sequential focus navigation.
            !dom.with_attribute(entity, "type", |t| {
                t.is_some_and(|s| s.eq_ignore_ascii_case("hidden"))
            })
        }
        Some(TagDefault::Summary) => is_first_summary_of_details(dom, entity),
        Some(TagDefault::Generic) => dom.with_attribute(entity, "contenteditable", |v| {
            v.is_some_and(|s| {
                s.is_empty()
                    || s.eq_ignore_ascii_case("true")
                    || s.eq_ignore_ascii_case("plaintext-only")
            })
        }),
    };
    if focusable {
        0
    } else {
        -1
    }
}

/// Whether `summary` is the first `<summary>` child of a `<details>` — the
/// disclosure widget's built-in control, which WHATWG HTML §6.6.2 designates a
/// **UA-determined focusable area** (so it participates in sequential focus
/// navigation with no author `tabindex`). A `<summary>` outside a `<details>`,
/// or any but the first, is not UA-focusable. (This is the focusability half
/// only; the activation behaviour — Enter/Space toggling `open` — is a separate
/// disclosure-widget default-action concern (distinct from the `details.open`
/// ToggleEvent, which is already handled), not modelled here.)
///
/// HTML-namespace only on **all three** tag matches (summary self, `details`
/// parent, and the first-`summary`-child scan) — mirroring `tab_index_default_for`
/// and the form-control exclusions: a foreign (SVG / MathML) `<details>` or
/// `<summary>` is not the HTML disclosure widget, so neither an HTML `<summary>`
/// under a foreign `<details>` nor a foreign `<summary>` preceding the HTML one
/// perturbs the built-in tab default.
fn is_first_summary_of_details(dom: &EcsDom, summary: Entity) -> bool {
    if !dom.is_html_namespace(summary) {
        return false;
    }
    let Some(parent) = dom.get_parent(summary) else {
        return false;
    };
    if !dom.is_html_namespace(parent)
        || !dom.with_tag_name(parent, |t| {
            t.is_some_and(|s| s.eq_ignore_ascii_case("details"))
        })
    {
        return false;
    }
    // The control is the first HTML `<summary>` child — a foreign `<summary>`
    // sibling does not count, so `first_child_with_tag` (namespace-blind) is not
    // sufficient; scan children for the first HTML-namespaced summary.
    dom.children(parent).into_iter().find(|&c| {
        dom.is_html_namespace(c)
            && dom.with_tag_name(c, |t| t.is_some_and(|s| s.eq_ignore_ascii_case("summary")))
    }) == Some(summary)
}

/// Whether `entity` is a focusable area (WHATWG HTML §6.6.2 "Data model").
///
/// §6.6.2 gives five criteria an Element must *all* meet to be a focusable area;
/// this is the engine-independent, **attribute-based** evaluation of them (this
/// crate has no `elidex-form` / computed-style dependency). Per-criterion
/// coverage:
///
/// **C1 — tabindex value non-null, OR UA-determined focusable** — *enforced*: a
/// `tabindex` that parses as a valid integer (the §2.3.4.1 "rules for parsing
/// integers", which §6.6.3 applies to the tabindex value, via
/// [`parse_tab_index_value`] — shared with the `tabIndex` IDL getter), or a
/// non-negative per-element default ([`tab_index_default_for`]:
/// `<a href>` / button / input(non-hidden) / select / textarea / iframe /
/// object / embed, the **first `<summary>` child of a `<details>`** via
/// `is_first_summary_of_details`, and an **editing host** — an element whose
/// *own* `contenteditable` is in the true or plaintext-only state). §6.6.3 lists
/// "Editing hosts" among the UA-determined focusable areas, and §6.8.4 defines an
/// editing host as the element carrying `contenteditable` in the true/plaintext-
/// only state — **not** the merely-editable descendants that inherit editability:
/// the editing-host arm of [`tab_index_default_for`] therefore reads the element's
/// *own* `contenteditable` (case-insensitive `""`/`true`/`plaintext-only`), not
/// the inherited `isContentEditable` algorithm ([`crate::element::is_content_editable`]).
/// A `<span>` inside `<div contenteditable>` is editable but not an editing host,
/// so it is not a focusable area; a click/`focus()` on it retargets to the host
/// via "get the focusable area" (§6.6.4), which is the
/// `#11-focusing-steps-fallback-target` slot — the same path every non-focusable
/// click target takes, not a contenteditable special case. The §6.6.2 table's
/// open-ended "any other element… determined by the user agent… to better match
/// platform conventions" clause is UA discretion, not a spec mandate, so it is
/// intentionally not enumerated here (it is not a fixed criterion to model).
///
/// **C2 — not a shadow host, or shadow root delegates-focus = false** — *not
/// enforced here*: a delegates-focus host is not itself the focusable area (its
/// first focusable shadow descendant is), which needs the shadow focus
/// *delegation* algorithm, not a bare exclusion (excluding it would wrongly make
/// `host.focus()` a no-op). Slot `#11-shadow-focus-delegation`.
///
/// **C3 — not actually disabled** — *enforced* (`is_actually_disabled`: a direct
/// `disabled` on a disablable element). Fieldset-inherited `disabled` lives in
/// the form subsystem (the shell overlays `FormControlState`); slot
/// `#11-focusable-area-fieldset-inherited-disabled` brings it to this path.
///
/// **C4 — not inert** — `inert` is not modelled by the engine, so there is
/// nothing to exclude (no gap today; revisit if `inert` lands).
///
/// **C5 — being rendered** — *partially enforced* from attributes. Three
/// attribute-reachable non-rendered cases are excluded: **disconnected**
/// ([`EcsDom::is_connected`] — a disconnected element is not rendered, so
/// `createElement('input').focus()` is a no-op); **`<input type=hidden>`**
/// (`is_hidden_input` — never rendered, even with a `tabindex`; §6.6.3 notes a
/// tabindex cannot grant focusability §6.6.2 withholds, and the shell rejects it
/// via `FormControlKind::Hidden` — the two focus writers must agree); and the
/// global **`hidden` attribute** (`is_in_hidden_subtree`, self or ancestor —
/// §6.1: both the Hidden and Hidden-Until-Found states are "will not be
/// rendered", and a `hidden` element hides its subtree). The CSS residue
/// (`display:none` / `visibility:hidden`, and a `[hidden] { display: block }`
/// author override), which needs computed style, is slot
/// `#11-focusable-area-being-rendered`. (Elements never rendered by category —
/// `<head>` metadata content — are a niche residual, not yet excluded.)
#[must_use]
pub fn is_focusable(dom: &EcsDom, entity: Entity) -> bool {
    // §6.6.2 criterion 5 (being rendered) — the attribute-reachable slice. Gate
    // these BEFORE the criterion-1 tabindex short-circuit, else a `tabindex`
    // would wrongly grant focusability to a non-rendered element.
    if !dom.is_connected(entity) {
        return false;
    }
    if is_hidden_input(dom, entity) {
        return false;
    }
    if is_in_hidden_subtree(dom, entity) {
        return false;
    }
    // §6.6.2 criterion 3 (not actually disabled).
    if is_actually_disabled(dom, entity) {
        return false;
    }
    // §6.6.2 criterion 1 (tabindex value non-null, or UA-determined focusable):
    // the `tabindex` attribute participates only when it parses as a valid
    // integer (the §2.3.4.1 "rules for parsing integers", which §6.6.3 applies);
    // an invalid value (`tabindex="foo"`) yields a null tabindex and falls
    // through to the per-element default — matching the `tabIndex` IDL getter.
    dom.with_attribute(entity, "tabindex", |v| {
        v.and_then(parse_tab_index_value).is_some()
    }) || tab_index_default_for(dom, entity) >= 0
}

/// Parse a `tabindex` content attribute value (WHATWG HTML §6.6.3 "tabindex
/// value" — the attribute is parsed using the §2.3.4.1 rules for parsing
/// integers; a failure yields a null tabindex). Delegates to the shared
/// `element::numeric_reflect::parse_integer` core so the focusable-area
/// predicate ([`is_focusable`]) and the VM `tabIndex` IDL getter parse
/// identically — and identically to every other `long` IDL reflect
/// (one-issue-one-way). The §2.3.4.1 rules collect the leading ASCII-digit run
/// and ignore trailing characters (`tabindex="1foo"` → `1`); the returned
/// `Option` preserves the null-tabindex distinction `is_focusable` reads via
/// `.is_some()`.
#[must_use]
pub fn parse_tab_index_value(raw: &str) -> Option<i32> {
    crate::element::numeric_reflect::parse_integer(raw)
}

/// A disablable form element carrying a direct `disabled` content attribute.
/// (Direct-attribute only; fieldset inheritance is the slot above.)
///
/// HTML-namespace only — `disabled` is an HTML form-control concept, so a
/// foreign (SVG / MathML) element merely *named* like a control and carrying a
/// `disabled` attribute is not "actually disabled". Mirrors
/// [`tab_index_default_for`]'s namespace gate: a foreign element is focusable
/// only via an explicit `tabindex`, which this exclusion must not suppress.
fn is_actually_disabled(dom: &EcsDom, entity: Entity) -> bool {
    dom.is_html_namespace(entity)
        && dom.has_attribute(entity, "disabled")
        && dom.with_tag_name(entity, |t| {
            t.is_some_and(|s| {
                s.eq_ignore_ascii_case("button")
                    || s.eq_ignore_ascii_case("input")
                    || s.eq_ignore_ascii_case("select")
                    || s.eq_ignore_ascii_case("textarea")
                    || s.eq_ignore_ascii_case("optgroup")
                    || s.eq_ignore_ascii_case("option")
                    || s.eq_ignore_ascii_case("fieldset")
            })
        })
}

/// `<input type=hidden>` — a hidden input is never "being rendered" (WHATWG HTML
/// §6.6.2 criterion 5), so it is not a focusable area even with a `tabindex`
/// (§6.6.3: a tabindex cannot grant focusability §6.6.2 withholds). The
/// attribute-based mirror of the shell's `FormControlKind::Hidden` rejection, so
/// the VM `focus()` and shell UA-input writers agree.
///
/// HTML-namespace only — an SVG / MathML element with local name `input` is not
/// an HTML hidden input, so this exclusion must not suppress its explicit-
/// `tabindex` focusability (mirrors [`tab_index_default_for`]'s namespace gate).
fn is_hidden_input(dom: &EcsDom, entity: Entity) -> bool {
    dom.is_html_namespace(entity)
        && dom.with_tag_name(entity, |t| {
            t.is_some_and(|s| s.eq_ignore_ascii_case("input"))
        })
        && dom.with_attribute(entity, "type", |v| {
            v.is_some_and(|s| s.eq_ignore_ascii_case("hidden"))
        })
}

/// Whether `entity` is in a `hidden`-attribute subtree — itself or a
/// shadow-including ancestor carries the global `hidden` content attribute
/// (WHATWG HTML §6.1). Both the *Hidden* and *Hidden Until Found* states are
/// "will not be rendered", and a hidden element hides its whole subtree, so an
/// element anywhere under a `hidden` node is not "being rendered" (§6.6.2
/// criterion 5) and hence not a focusable area — `<button hidden>` and a
/// `<button>` inside `<div hidden>` alike. Walks via [`EcsDom::get_parent`]
/// (shadow-inclusive, so a `hidden` host hides its shadow tree), bounded by
/// [`MAX_ANCESTOR_DEPTH`]. Attribute-based: a `[hidden] { display: block }`
/// author override (computed-style residue, slot
/// `#11-focusable-area-being-rendered`) is not reflected here.
///
/// [`MAX_ANCESTOR_DEPTH`]: elidex_ecs::MAX_ANCESTOR_DEPTH
fn is_in_hidden_subtree(dom: &EcsDom, entity: Entity) -> bool {
    let mut cur = Some(entity);
    let mut depth = 0;
    while let Some(c) = cur {
        if dom.has_attribute(c, "hidden") {
            return true;
        }
        if depth >= elidex_ecs::MAX_ANCESTOR_DEPTH {
            return false;
        }
        cur = dom.get_parent(c);
        depth += 1;
    }
    false
}

/// Whether `entity` is **click focusable** (WHATWG HTML §6.6.2 `#click-focusable`
/// — a focusable area the user agent permits to be focused by clicking). elidex
/// has no "do not
/// focus on click" UA setting, so every focusable area is click focusable; this
/// is kept spec-shaped for the [`autofocus_delegate`] click filter (presently
/// always `true`).
///
/// [`autofocus_delegate`]: super::autofocus_delegate
pub(super) fn is_click_focusable(dom: &EcsDom, entity: Entity) -> bool {
    is_focusable(dom, entity)
}
