"""Canonical spec shortname ↔ human label mapping — the single source.

Three sites used to carry a hand-maintained copy of this mapping:

  - `commands/coverage_map.py` (shortname → label, for §3 table rows)
  - `commands/cite_audit.py`   (label → shortname, for cite attribution)
  - `.claude/skills/elidex-plan-review/preflight.py` (label → shortname,
    for parsing plan-memo §3 Spec-section cells)

Adding a spec to one did not reach the other two, so the mapping drifted
by construction — the same partial, hand-maintained enumeration whose
failure mode the `cite-audit` command exists to detect. Keeping three
copies inside the sweep that diagnoses that pattern would be
self-refuting, so they all import from here.

`SPECS` is the only thing to edit; both directions and every alias are
derived from it, so a new entry reaches all consumers at once.
"""
from __future__ import annotations

# (webref shortname, canonical display label, *additional parse aliases)
#
# The canonical label is what `coverage-map` prints and what a plan-memo
# §3 cell should say. Aliases exist only for *parsing* — real comments
# and memos abbreviate ("HTML §4.10.5" rather than "WHATWG HTML §4.10.5").
SPECS: tuple[tuple[str, ...], ...] = (
    ("ecma262", "ECMA-262"),
    ("ecma402", "ECMA-402"),
    ("html", "WHATWG HTML", "HTML"),
    ("dom", "WHATWG DOM", "DOM"),
    ("url", "WHATWG URL", "URL"),
    ("fetch", "WHATWG Fetch", "Fetch"),
    ("streams", "WHATWG Streams", "Streams"),
    ("xhr", "WHATWG XHR", "XHR"),
    ("webcrypto", "Web Cryptography API", "WebCrypto"),
    ("webidl", "Web IDL", "WebIDL"),
    ("selectors-4", "CSS Selectors L4"),
    ("geometry-1", "Geometry Interfaces L1"),
)

#: shortname → canonical display label (e.g. `"html"` → `"WHATWG HTML"`).
SHORTNAME_TO_LABEL: dict[str, str] = {
    entry[0]: entry[1] for entry in SPECS
}

#: **lower-cased** label (canonical or alias) → shortname. Keys are
#: lower-cased so callers can look up case-insensitively without a
#: second scan; the shortname itself is accepted as its own alias so
#: `"selectors-4"` resolves whether a comment writes the label or the
#: shortname.
#:
#: Built as a comprehension rather than an accumulating loop so no
#: module-level temporaries exist to `del`. An earlier form ended with
#: `del _entry, _short, _labels, _label`, which raises `NameError` **at
#: import** if `SPECS` is ever empty — and this module is imported at
#: load time by `cite-audit`, `coverage-map`, and the plan-review
#: preflight gate, so that failure would take all three out at once.
LABEL_TO_SHORTNAME: dict[str, str] = {
    key.lower(): entry[0]
    for entry in SPECS
    for key in (entry[0], *entry[1:])
}


def label_for(shortname: str) -> str | None:
    """Canonical display label for `shortname`, or None if unknown."""
    return SHORTNAME_TO_LABEL.get(shortname)


def shortname_for(label: str) -> str | None:
    """Shortname for a label or alias, case-insensitively, or None."""
    return LABEL_TO_SHORTNAME.get(label.strip().lower())
