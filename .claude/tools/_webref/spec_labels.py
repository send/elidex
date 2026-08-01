"""Canonical spec shortname ↔ human display label.

Two sites in the generic tree carried a hand-maintained copy of this
enumeration:

  - `commands/coverage_map.py` — shortname → label, for §3 table rows
  - `cli.py`'s `COMMON_SHORTNAMES` help blurb

Adding a spec to one did not reach the other, so the two drifted apart by
construction. Both now derive from `SPECS` below, so a spec is added in
exactly one place.

The plan-review gate keeps a reversed copy of its own. That one lives
outside this tree, behind its own failure semantics, and migrates
separately — this module is where it lands.
"""
from __future__ import annotations

# (shortname, canonical display label, help blurb)
#
# The canonical label is what `coverage-map` prints and what a plan-memo
# §3 cell should say; the blurb is `cli.py`'s `Common shortnames:` help
# text, which was the second of the two copies this module replaces. The
# tuple's ORDER is the order `cli.py` renders, so it is part of the help
# output, not an implementation detail.
#
# No separate parse-alias column: `LABEL_TO_SHORTNAME` keys the shortname
# itself, and every abbreviation this repo actually used (`HTML`, `DOM`,
# `URL`) lower-cases to its own shortname, so an alias column would add
# no key.
SPECS: tuple[tuple[str, str, str], ...] = (
    ("html", "WHATWG HTML",
     "HTML LS (Custom Elements / Canvas / Workers / Form / Events — monolithic)"),
    ("dom", "WHATWG DOM", "DOM LS"),
    ("selectors-4", "CSS Selectors L4", "CSS Selectors L4"),
    ("geometry-1", "Geometry Interfaces L1",
     "Geometry Interfaces (DOMRect / DOMMatrix)"),
    ("url", "WHATWG URL", "URL LS"),
    ("fetch", "WHATWG Fetch", "Fetch LS"),
    ("streams", "WHATWG Streams", "Streams LS"),
    ("webcrypto", "Web Cryptography API",
     "Web Cryptography API (series → current spec webcrypto-2)"),
    ("xhr", "WHATWG XHR", "XMLHttpRequest LS"),
    ("webidl", "Web IDL", "Web IDL"),
    ("ecma262", "ECMA-262",
     "ECMAScript Language Specification (tc39, biblio.json)"),
    ("ecma402", "ECMA-402",
     "ECMAScript Internationalization API (tc39, biblio.json)"),
)

#: shortname → canonical display label, for the specs `SPECS` pins.
SHORTNAME_TO_LABEL: dict[str, str] = {e[0]: e[1] for e in SPECS}

#: shortname → one-line help blurb (consumed by `cli.py`).
SHORTNAME_TO_BLURB: dict[str, str] = {e[0]: e[2] for e in SPECS}

#: **lower-cased** display label or shortname → shortname, for the specs
#: `SPECS` pins. Keys are lower-cased so callers look up
#: case-insensitively without a second scan; the shortname is its own
#: parse key, so `"selectors-4"` resolves whether a comment writes the
#: label or the shortname. Built as a comprehension rather than an
#: accumulating loop so no module-level temporaries exist to `del`: a
#: trailing `del _entry, …` raises `NameError` **at import** if `SPECS`
#: is ever empty, and this module is imported at load time by
#: `coverage-map` and `cli` — and, once the gate's copy migrates, by the
#: plan-review gate too.
LABEL_TO_SHORTNAME: dict[str, str] = {
    key.lower(): entry[0]
    for entry in SPECS
    # shortname + CANONICAL LABEL. Omitting `entry[1]` here would leave
    # `shortname_for("WHATWG HTML")` returning None — the canonical label
    # is the primary parse key, not just a display string.
    for key in (entry[0], entry[1])
}


def label_for(shortname: str) -> str | None:
    """Canonical display label for `shortname`, or None if unknown.

    `SPECS` pins this repo's display conventions (the `"WHATWG "` prefix,
    the tc39 pair), so the answer is a pinned one or no answer at all.
    """
    return SHORTNAME_TO_LABEL.get(shortname)


def shortname_for(label: str) -> str | None:
    """Shortname for a display label or a shortname — case-insensitively.

    Whitespace-tolerant because the callers are plan-memo table cells and
    source comments, where a stray leading space is not a different spec.
    """
    if not label:
        return None
    return LABEL_TO_SHORTNAME.get(label.strip().lower())
