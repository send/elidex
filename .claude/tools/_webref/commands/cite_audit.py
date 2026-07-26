"""`cite-audit` subcommand — repo citation inventory + resolution.

An **adapter command** (DESIGN.md "Externalization Criteria": generic
behavior stays free of elidex-specific paths; elidex policy lives in
adapter commands). The generic half — resolving a §number to its title —
is `resolver.lookup_section`, untouched. This command adds the elidex
policy half: which tree to scan, and what a citation looks like in it.

Purpose: make citation **discovery** a checked-in detector rather than a
hand-authored pattern list. Successive citation sweeps each shipped a
detector that could only find what its author already suspected:

  1. resolution-only-by-known-outcome  — "does `heading --exact` fail?"
  2. enumeration-only-by-known-pattern — a hand-written grep alternation
  3. triage-only-by-known-suspicion    — total enumeration, partial triage

The fix common to all three is that the *candidate set* must be derived,
and must carry enough context to classify each cite by concept. This
command emits exactly that: every distinct `§<section>` cited in the
scanned tree, its resolved title (or UNRESOLVED), and every citing line
with `file:line` plus the comment text around it.

Discovery is the tool's job; classification stays the author's.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

from ..resolver import lookup_section
from ..section_sort import sec_number_key
from ..spec_labels import LABEL_TO_SHORTNAME

# Default scan root/glob are elidex POLICY, not generic CLI behavior, so
# they live with the adapter command and `cli.py` imports them. Restating
# them as literals in the CLI wiring would give the defaults two homes —
# the duplication class this PR exists to remove — and would park elidex
# policy in the generic core, against DESIGN.md's generic/adapter split.
DEFAULT_ROOT = "crates"
DEFAULT_GLOB = "*.rs"

# Longest labels first so "WHATWG HTML" wins over the bare "HTML" alias.
_LABEL_ALT = "|".join(
    re.escape(k) for k in sorted(LABEL_TO_SHORTNAME, key=len, reverse=True)
)

# ONE regex for both halves of a citation: the optional spec label and the
# section number. An earlier form used two (a label regex + a number
# regex) rejoined by (line, column) with an `abs(pos - start) <= 2`
# tolerance, because the `§`'s position had to be *inferred* from the
# label match. The optional group removes the second scan, the coordinate
# join, and the tolerance window at once.
#
# The number grammar accepts **annex** components (`A.1.1`, `F.2`), not
# only digits. Real annex citations exist — `elidex-api-crypto/src/rsa.rs`
# cites RFC 3447 `§A.1.1` (a cross-spec mis-attribution, exactly the class
# this tool advertises) and `events_modern/mod.rs` cites a plan-memo
# `§F.2` (the UNATTRIBUTED class). A digits-only grammar made both
# invisible. `preflight.SECTION_REF_RE` already accepts annexes — with a
# comment explaining that enumerating annex letters drifts — and
# `resolver.lookup_section` already resolves them; this aligns with both.
# ⚠ `re.IGNORECASE` applies to the WHOLE pattern, so a naive
# `[\dA-Z]+` number half also matched lowercase and turned anchor-style
# refs into phantom sections: `§attr-fs-method` → section `attr`,
# `§dom-document-title` → `dom`, and internal markers like `§Deferred` /
# `§C1` → `D` / `C`. Those were reported UNRESOLVED, so `--strict` failed
# partly on citations that do not exist. The number half is therefore
# case-SCOPED with `(?-i:…)` while the label alternation stays
# case-insensitive, and it admits exactly two shapes:
#   - numeric sections  `4.10.5.1`
#   - single-letter annexes with numeric subsections  `A.1.1`, `F.2`
# The trailing `(?![\w-])` rejects `§C1` and `§attr-fs-method`, which
# would otherwise match a one-character prefix. Annex support (the
# finding-1 widening) is preserved — `sec_number_key` orders them.
_CITE_RE = re.compile(
    rf"(?:({_LABEL_ALT})(?:\s+spec)?\s*)?"
    r"§\s*(?-i:(\d+(?:\.\d+)*|[A-Z](?:\.\d+)*))(?![\w-])",
    re.IGNORECASE,
)

# A spec label at end-of-line with no `§` after it: the citation wrapped to
# the next comment line ("… + WHATWG HTML\n/// §4.10.21.3 step 7").
# Without this the next line inherits whatever spec was named EARLIER on
# the wrapped line — a false negative *inside* an audited set, which is
# worse than an unattributed cite because nothing reports it.
#
# NB a cheaper `line.endswith(alias)` guard is NOT equivalent: multi-word
# labels ("Web Cryptography API") do not end in any single alias, and it
# disagreed with this regex on 179 real lines.
_DANGLING_LABEL_RE = re.compile(rf"({_LABEL_ALT})\s*$", re.IGNORECASE)

_COMMENT_RE = re.compile(r"^\s*(///|//!|//|\*|/\*)")


def _attribute(lines: list[str]) -> list[tuple[int, str, str | None]]:
    """Return `(lineno, section, shortname_or_None)` for every citation.

    Three attribution buckets — a bare `§N.N` means nothing on its own:
    `§2.2.2` is WHATWG Fetch in `elidex-net`, and `§0.5` is a plan-memo
    pointer that is not a spec citation at all.

      (a) EXPLICIT     — a spec label immediately precedes the `§`
      (b) INHERITED    — bare, but a labelled cite appears earlier in the
                         SAME comment block (any non-comment line ends it),
                         or a label dangled at the end of the prior line
      (c) UNATTRIBUTED — neither; reported as its own class, because a `§`
                         a reader cannot attribute is a defect regardless
                         of whether it happens to resolve somewhere
    """
    out: list[tuple[int, str, str | None]] = []
    block_spec: str | None = None
    carried: str | None = None  # label dangling at the end of the prior line
    for lineno, line in enumerate(lines, start=1):
        in_comment = _COMMENT_RE.match(line) is not None
        if not in_comment:
            block_spec = None  # a non-comment line ends the block
            carried = None
        # 97.7% of lines carry no `§`, and the alternation is ~15 branches
        # under IGNORECASE, so this guard is the difference between 4.85s
        # and 0.37s over the tree. Exactly equivalent — the pattern
        # requires a literal `§`.
        if "§" in line:
            for m in _CITE_RE.finditer(line):
                label, section = m.group(1), m.group(2)
                if label:
                    block_spec = LABEL_TO_SHORTNAME[label.lower()]
                elif carried:
                    block_spec = carried
                out.append((lineno, section, block_spec))
                carried = None  # a carry attributes at most one cite
        # Recompute the carry AFTER this line's cites, so it applies to
        # the next line only. This MUST run on every comment line, not
        # only when `carried` is already None: guarding on that made the
        # carry *sticky*, so "WHATWG HTML / prose only / §4.10.5" wrongly
        # attributed the third line. (An `endswith` guard is likewise
        # unsafe — multi-word labels such as "Web Cryptography API" do not
        # end in any single alias, and it disagreed on 179 real lines.)
        if in_comment:
            dangling = _DANGLING_LABEL_RE.search(line)
            carried = (
                LABEL_TO_SHORTNAME[dangling.group(1).lower()] if dangling else None
            )
    return out


def _iter_cites(root: Path, glob: str):
    """Yield (section, spec_or_None, path, lineno, line_text) per citation."""
    for path in sorted(root.rglob(glob)):
        if "target" in path.parts:
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        if "§" not in text:
            continue
        lines = text.splitlines()
        for lineno, section, spec in _attribute(lines):
            yield section, spec, path, lineno, lines[lineno - 1].strip()


def _render(path: Path, lineno: int, text: str) -> str:
    return f"    {path}:{lineno}: {text[:150]}"


def _record(rec: tuple[Path, int, str]) -> dict:
    path, lineno, text = rec
    return {"file": str(path), "line": lineno, "text": text}


def _emit_text(args, sections, resolved, by_section, other_spec, unattributed,
               total_cites, unresolved, unresolved_cites) -> None:
    print(f"# cite-audit — spec={args.spec} root={args.root} glob={args.glob}"
          + (f" prefix=§{args.prefix}" if args.prefix else ""))
    print(f"# attributed to {args.spec}: {len(sections)} distinct sections, "
          f"{total_cites} cites")
    print(f"# UNRESOLVED sections: {len(unresolved)}   "
          f"cites carrying them: {unresolved_cites}")
    print(f"# attributed to another spec (not audited here): {other_spec} cites")
    print(f"# UNATTRIBUTED (bare § with no spec in its comment block): "
          f"{len(unattributed)} cites")
    print()
    if unattributed and args.show_unattributed:
        print("## UNATTRIBUTED cites — a reader cannot tell which spec these "
              "name. A defect independent of resolution.")
        for rec in unattributed:
            print(_render(*rec))
        print()
    for section in sections:
        title = resolved[section] or "*** UNRESOLVED ***"
        print(f"§{section}  [{len(by_section[section])} cite(s)]  {title}")
        if not args.summary:
            for rec in by_section[section]:
                print(_render(*rec))
            print()


def _emit_json(args, sections, resolved, by_section, other_spec, unattributed,
               total_cites, unresolved, unresolved_cites) -> None:
    json.dump(
        {
            "spec": args.spec,
            "root": str(args.root),
            "distinct_sections": len(sections),
            "total_cites": total_cites,
            "unresolved_sections": len(unresolved),
            "unresolved_cites": unresolved_cites,
            "other_spec_cites": other_spec,
            "unattributed_cites": len(unattributed),
            "unattributed": [_record(r) for r in unattributed],
            "sections": [
                {
                    "section": s,
                    "title": resolved[s],
                    "resolved": resolved[s] is not None,
                    "cite_count": len(by_section[s]),
                    "cites": [_record(r) for r in by_section[s]],
                }
                for s in sections
            ],
        },
        sys.stdout,
        indent=2,
        # The tool's whole subject is `§`; the default would ship it as
        # `§`. Matches `diff.py` / `agent_brief.py`.
        ensure_ascii=False,
    )
    print()


def cmd_cite_audit(args: argparse.Namespace) -> None:
    root = Path(args.root)
    if not root.is_dir():
        # `cli.main` calls `args.func(args)` and DISCARDS the return value —
        # every command signals failure via `sys.exit`, so a bare `return 1`
        # would be silently dropped and this error would exit 0.
        sys.exit(f"cite-audit: not a directory: {root}")

    by_section: dict[str, list[tuple[Path, int, str]]] = defaultdict(list)
    unattributed: list[tuple[Path, int, str]] = []
    other_spec = 0
    # `--prefix` is a dotted-COMPONENT filter, not a string prefix:
    # `4.10.2` must match `4.10.2` and `4.10.2.7` but NOT `4.10.20.3`. A
    # bare `str.startswith` silently widens by an order of magnitude — the
    # quiet over-match `heading --exact` guards against. It scopes EVERY
    # reported class, not just `by_section`, so a scoped run does not dump
    # the whole tree's unattributed list.
    want = args.prefix.split(".") if args.prefix else None
    for section, spec, path, lineno, line in _iter_cites(root, args.glob):
        if want and section.split(".")[: len(want)] != want:
            continue
        if spec is None:
            unattributed.append((path, lineno, line))
        elif spec != args.spec:
            other_spec += 1
        else:
            by_section[section].append((path, lineno, line))

    # `lookup_section` returns (number, title, anchor) or None and is
    # prefix-tolerant, so an EXACT match is required — otherwise `§4.13`
    # would silently pass because `§4.13.1` exists (the same drift-catch
    # invariant `heading --exact` enforces).
    resolved: dict[str, str | None] = {}
    for section in by_section:
        try:
            hit = lookup_section(args.spec, section)
        except Exception:  # noqa: BLE001 — resolver raises varied lookup errors
            hit = None
        resolved[section] = hit[1] if hit and hit[0] == section else None

    # `sec_number_key` (shared with resolver / aoid / heading / inventory)
    # orders annex components after numeric chapters; a naive
    # `tuple(int(p) …)` key would `ValueError` on `A.1.1`, which is why the
    # digits-only grammar above used to be load-bearing.
    sections = sorted(by_section, key=sec_number_key)
    unresolved = [s for s in sections if resolved[s] is None]
    total_cites = sum(len(v) for v in by_section.values())
    unresolved_cites = sum(len(by_section[s]) for s in unresolved)

    emit = _emit_json if args.format == "json" else _emit_text
    emit(args, sections, resolved, by_section, other_spec, unattributed,
         total_cites, unresolved, unresolved_cites)

    if args.strict and unresolved:
        sys.exit(1)
