"""Canonical spec shortname ↔ human label mapping.

Four sites used to carry a hand-maintained copy of this enumeration:

  - `commands/coverage_map.py` (shortname → label, for §3 table rows)
  - `commands/cite_audit.py`   (label → shortname, for cite attribution)
  - `.claude/skills/elidex-plan-review/preflight.py` (label → shortname,
    for parsing plan-memo §3 Spec-section cells)
  - `cli.py`'s `COMMON_SHORTNAMES` help blurb

Adding a spec to one did not reach the others, so the mapping drifted by
construction — the same partial hand-maintained enumeration whose failure
mode `cite-audit` exists to detect.

**`SPECS` is a fallback, not the source.** `sources/webref_data.py`
already memoizes upstream's full catalog with a `shortTitle` per
shortname, and states the principle verbatim: *"No hand-maintained alias
map — the catalog answers both."* So a lookup miss consults the catalog
before giving up, in the same direct-then-catalog shape `try_fetch_data`
uses. `SPECS` is reduced to what upstream cannot supply: the tc39 pair
(absent from webref's catalog) and the `"WHATWG "` display prefix this
repo's memos use. Adding a W3C/WHATWG spec is now a no-op.
"""
from __future__ import annotations

# (shortname, canonical display label, help blurb, *parse aliases)
#
# The canonical label is what `coverage-map` prints and what a plan-memo
# §3 cell should say. Aliases exist only for *parsing* — real comments and
# memos abbreviate ("HTML §4.10.5" rather than "WHATWG HTML §4.10.5").
# The blurb is `cli.py`'s `Common shortnames:` help text, which was a
# fourth copy of this same list.
SPECS: tuple[tuple[str, ...], ...] = (
    ("html", "WHATWG HTML",
     "HTML LS (Custom Elements / Canvas / Workers / Form / Events — monolithic)",
     "HTML"),
    ("dom", "WHATWG DOM", "DOM LS", "DOM"),
    ("selectors-4", "CSS Selectors L4", "CSS Selectors L4"),
    ("geometry-1", "Geometry Interfaces L1",
     "Geometry Interfaces (DOMRect / DOMMatrix)"),
    ("url", "WHATWG URL", "URL LS", "URL"),
    ("fetch", "WHATWG Fetch", "Fetch LS", "Fetch"),
    ("streams", "WHATWG Streams", "Streams LS", "Streams"),
    ("webcrypto", "Web Cryptography API",
     "Web Cryptography API (series → current spec webcrypto-2)", "WebCrypto"),
    ("xhr", "WHATWG XHR", "XMLHttpRequest LS", "XHR"),
    ("webidl", "Web IDL", "Web IDL", "WebIDL"),
    ("ecma262", "ECMA-262",
     "ECMAScript Language Specification (tc39, biblio.json)"),
    ("ecma402", "ECMA-402",
     "ECMAScript Internationalization API (tc39, biblio.json)"),
)

#: shortname → canonical display label, for the specs `SPECS` pins.
SHORTNAME_TO_LABEL: dict[str, str] = {e[0]: e[1] for e in SPECS}

#: shortname → one-line help blurb (consumed by `cli.py`).
SHORTNAME_TO_BLURB: dict[str, str] = {e[0]: e[2] for e in SPECS}

#: **lower-cased** label (canonical or alias) → shortname, for the specs
#: `SPECS` pins. Keys are lower-cased so callers look up
#: case-insensitively without a second scan; the shortname is its own
#: alias so `"selectors-4"` resolves whether a comment writes the label or
#: the shortname. Built as a comprehension rather than an accumulating
#: loop so no module-level temporaries exist to `del` — an earlier form
#: ended with `del _entry, …`, which raises `NameError` **at import** if
#: `SPECS` is ever empty, and this module is imported at load time by
#: `cite-audit`, `coverage-map`, `cli`, and the plan-review preflight.
LABEL_TO_SHORTNAME: dict[str, str] = {
    key.lower(): entry[0]
    for entry in SPECS
    # shortname + CANONICAL LABEL + aliases. Omitting `entry[1]` here
    # silently broke `shortname_for("WHATWG HTML")` when the blurb was
    # inserted at index 2 and shifted the aliases — the canonical label is
    # the primary parse key, not just a display string.
    for key in (entry[0], entry[1], *entry[3:])
}


def _catalog() -> dict[str, dict]:
    """Upstream's spec catalog, or `{}` when it cannot be reached.

    Imported lazily and failure-tolerantly: the catalog *widens* `SPECS`,
    it is never a precondition, so an offline run degrades to the pinned
    set rather than dying.
    """
    try:
        from .sources.webref_data import _data_index

        return _data_index()
    except Exception:  # noqa: BLE001 — offline / fetch failure is expected
        return {}


def label_for(shortname: str) -> str | None:
    """Canonical display label for `shortname`, or None if unknown.

    `SPECS` first (it pins this repo's display conventions, e.g. the
    `"WHATWG "` prefix), then upstream's `shortTitle`.
    """
    pinned = SHORTNAME_TO_LABEL.get(shortname)
    if pinned:
        return pinned
    entry = _catalog().get(shortname)
    if entry:
        title = entry.get("shortTitle") or entry.get("title")
        if title:
            return str(title)
    return None


def shortname_for(label: str) -> str | None:
    """Shortname for a label, alias, or shortname — case-insensitively.

    `SPECS` first, then upstream's catalog by shortname and by
    `shortTitle` / `title`. That is what makes a CSS-module row such as
    `CSS Text 3 §4.1.3` resolve without anyone hand-adding `css-text-3`:
    the plan-review gate previously failed **open** on those, soft-warning
    and skipping citation verification while still counting the row toward
    breadth.
    """
    if not label:
        return None
    key = label.strip().lower()
    pinned = LABEL_TO_SHORTNAME.get(key)
    if pinned:
        return pinned
    catalog = _catalog()
    if key in catalog:
        return key
    for short, entry in catalog.items():
        for field in ("shortTitle", "title"):
            value = entry.get(field)
            if value and str(value).strip().lower() == key:
                return short
    return None
