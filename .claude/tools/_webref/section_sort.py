"""Sort key for spec section numbers."""
from __future__ import annotations

import re

# A leading `N`, `N.M`, `N.M.P`, … followed by `. ` at the start of a heading
# title (e.g. `"31. HMAC"`). Used to recover a section number that the spec
# author baked into the title text instead of ReSpec's auto-numbering.
_TITLE_NUMBER_RE = re.compile(r"^(\d+(?:\.\d+)*)\.\s+(.*)$")


def heading_number_title(h: dict) -> tuple[str, str]:
    """Effective `(number, title)` for a webref heading entry.

    Most specs carry an explicit `number`. Some W3C specs — notably the Web
    Cryptography API algorithm chapters (`"29. AES-GCM"`, `"31. HMAC"`, …) —
    author their top-level section numbers manually in the title text and
    leave webref's `number` null. When `number` is absent, recover it from a
    leading `N[.M…]. ` prefix in the title so §-number lookups resolve, and
    return the title with that redundant prefix stripped for display.
    Falls back to `("", title)` for genuinely unnumbered headings (title
    page, table of contents).
    """
    number = h.get("number")
    title = h.get("title", "")
    if number:
        return number, title
    m = _TITLE_NUMBER_RE.match(title)
    if m:
        return m.group(1), m.group(2)
    return "", title


def sec_number_key(num: str) -> tuple:
    """Sort `25.5.2` and `25.5.10` numerically and ECMA-262 annex `A.1`,
    `B.2.1` after the numeric chapters (and stably among themselves).

    Each component is keyed as `(0, int)` for digits and `(1, str)` for
    non-digits, so digit chapters precede annex chapters, and `A` precedes
    `B` lexically.
    """
    out: list[tuple[int, int | str]] = []
    for p in num.split("."):
        if p.isdigit():
            out.append((0, int(p)))
        else:
            out.append((1, p))
    return tuple(out)
