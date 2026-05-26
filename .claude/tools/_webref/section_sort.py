"""Sort key for spec section numbers."""
from __future__ import annotations


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
