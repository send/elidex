"""`coverage-map` subcommand — plan-memo §3 skeleton + breadth verdict."""
from __future__ import annotations

import argparse
import sys

from ..resolver import lookup_section

# Human-readable spec label for the first column of §3 table rows. Falls back
# to UPPER(shortname-with-dashes-as-spaces) for unmapped shortnames — extend
# the map when a new spec becomes frequently cited (cosmetic only, not load-
# bearing for verification).
_SPEC_LABEL_MAP = {
    "ecma262": "ECMA-262",
    "ecma402": "ECMA-402",
    "html": "WHATWG HTML",
    "dom": "WHATWG DOM",
    "url": "WHATWG URL",
    "fetch": "WHATWG Fetch",
    "streams": "WHATWG Streams",
    "xhr": "WHATWG XHR",
    "webcrypto": "Web Cryptography API",
    "webidl": "Web IDL",
    "selectors-4": "CSS Selectors L4",
    "geometry-1": "Geometry Interfaces L1",
}


def _spec_label(shortname: str) -> str:
    if shortname in _SPEC_LABEL_MAP:
        return _SPEC_LABEL_MAP[shortname]
    return shortname.upper().replace("-", " ")


def cmd_coverage_map(args: argparse.Namespace) -> None:
    """Generate plan-memo §3 Spec coverage map skeleton from (spec, ref) pairs.

    Each (shortname, ref) pair is verified via webref/tc39-biblio. Verified
    cell goes into the Spec section column; remaining columns are blanked
    out for the plan author to fill (Step / Branch / Touch / Full enum? /
    User-input flow). Prints breadth (unique specs, total entries) and the
    split-decision verdict per `feedback_plan-scope-re-evaluation.md`:
      K≥6 OR M≥30 → 分割 default
      K≥4 OR M≥20 → 分割推奨
      else        → single PR OK
    """
    pairs = args.pairs
    if len(pairs) % 2 != 0:
        sys.exit("webref: coverage-map needs (spec, ref) pairs — got odd-count args")
    if not pairs:
        sys.exit("webref: coverage-map needs at least one (spec, ref) pair")

    rows: list[tuple[str, str]] = []  # (spec_cell, anchor)
    specs_seen: set[str] = set()  # verified shortnames
    failed: list[tuple[str, str, str]] = []
    # Requested scope = input pairs as-given. Breadth/verdict are computed
    # from this (author intent) rather than only the verified subset so a
    # plan with 1 verified + 1 unresolved doesn't appear narrower than it
    # is. Verified subset is reported separately for transparency.
    requested_shortnames: set[str] = set()
    requested_pairs = len(pairs) // 2

    for i in range(0, len(pairs), 2):
        shortname = pairs[i]
        ref = pairs[i + 1]
        requested_shortnames.add(shortname)
        resolved = lookup_section(shortname, ref)
        if resolved is None:
            failed.append((shortname, ref, "no heading / AO match"))
            continue
        number, title, anchor = resolved
        spec_cell = f"{_spec_label(shortname)} §{number} {title}"
        rows.append((spec_cell, anchor))
        specs_seen.add(shortname)

    # stdout = the copy/paste-friendly markdown artifact (table + anchors +
    # breadth verdict). stderr = diagnostics (verification failures). Matches
    # webref's other subcommands' stderr-for-errors convention.
    print("| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |")
    print("|---|---|---|---|---|---|")
    for spec_cell, _anchor in rows:
        print(f"| {spec_cell} | <step> | <branch> | <site> | ✗ | <yes/no> |")

    if rows:
        print()
        print("Anchors (for cross-reference, drop into plan-memo §0.5 Spec citation table):")
        for spec_cell, anchor in rows:
            print(f"  {spec_cell}")
            print(f"    {anchor}")

    # Breadth + verdict reflect *requested* scope (author intent), not just
    # successfully verified rows — a failure-verify pair is still part of
    # the plan's surface, it just needs the ref corrected before landing.
    K = len(requested_shortnames)
    M = requested_pairs
    print()
    print(f"**Breadth (requested)**: spec={K} "
          f"({', '.join(sorted(requested_shortnames)) if requested_shortnames else '-'}), "
          f"step={M} entries")
    if len(rows) != M or len(specs_seen) != K:
        print(f"  verified subset:        spec={len(specs_seen)}, step={len(rows)} "
              f"({len(failed)} failed)")
    if K >= 6 or M >= 30:
        print("**Split decision**: ⚠ K≥6 or M≥30 → 分割 default (split-into-multiple-PRs を真剣に検討)")
    elif K >= 4 or M >= 20:
        print("**Split decision**: K≥4 or M≥20 → 分割推奨 (per-PR scope を再評価)")
    else:
        print("**Split decision**: ok → single PR scope")

    if failed:
        print(f"\n⚠ {len(failed)} entry/entries failed verification:", file=sys.stderr)
        for shortname, ref, reason in failed:
            print(f"  - {shortname} {ref}: {reason}", file=sys.stderr)
        sys.exit(1)
