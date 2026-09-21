#!/usr/bin/env python3
"""Restatement sweep for a plan-memo (plan-memo §13.4).

You are changing one decision. This lists EVERY site in the memo that restates
it, grouped by the three surfaces a changed decision has, so you re-derive all
of them in one edit instead of sweeping the statements and leaving the
obligations and the consequences standing:

  STATEMENT    the prose and tables that state the decision (every other §)
  OBLIGATION   §8 definition of done, §10 slot ledger actions
  CONSEQUENCE  §6 edge-matrix cells, §7 downstream

The primary interface is an ad-hoc pattern for the decision you are changing
*now* — that is the whole job:

    plan-sweep.py memo.md --pattern 'PR-1b|behaviour.neutral'

`--concept D5` replays a frozen preset from an earlier round, and no selector
at all runs every preset. The presets are a convenience; they can only sweep
last month's decisions.

Every match is reported individually (all matches in a line, not just the
first) with its character offset and a window of context centred on it, with
the matched text delimited »like this«. That is load-bearing: this memo's
tables are single lines thousands of characters long, so a restatement at
char 3000 of a table row is invisible to anything that prints a line prefix.

WHAT THIS DOES NOT CATCH — it is a *lexical* sweep, not a semantic one. A
restatement that shares no token with the pattern does not match and is not
listed: the same decision worded differently ("the line survives" for "stays
non-phantom"), reached through a synonym or an abbreviation, split across a
line break, or merely implied by a number, a diagram, or a row count. A clean
run is evidence about the pattern you chose — never proof that the decision is
stated in exactly one place. Empty surfaces are therefore printed loudly:
`(0 sites)` under OBLIGATION much more often means the pattern is too narrow
than that §8 is genuinely free of the decision.

Exit code is always 0. This reports; it does not gate.
"""
import argparse, re, sys, pathlib

# concept -> regex matching any site that restates the decision (not just the quoted one)
CONCEPTS = {
 "D1 existence-predicate":   r"contributes_content|clause-3 (test|predicate)|inline-axis components|stays phantom|line stays suppressed",
 "D2 edge-set-arity":        r"LogicalEdges|EdgeSizes|padding/border/margin|padding`/`border`/`margin",
 "D3 logical-physical-leg":  r"from_physical|to_physical|WritingModeContext|is_vertical|physical",
 "D4 intrinsic-sizing":      r"min_content|max_content|intrinsic|shrink-to-fit|measure\.rs",
 "D5 pr1b-neutrality":       r"neutral|line_count|discard arm|items\.is_empty|held gate|on_line",
 "D6 softwrap-guard":        r":690|soft-wrap|soft wrap|wrap guard|wrap opportunity",
 "D7 nonpersist-fold":       r"non-persist|commit_aligned_entity_rects|line_rects|InlineClientRects",
 "D8 prereq-split-ground":   r"prereq split|seam 3|seam-3|fragmented-fn|1000",
 "D9 cell-16b":              r"16b|margin edge|margin-edge",
 "D10 rtl-vertical":         r"direction: rtl|vertical-rl|writing.mode|RTL",
 "D11 row-pair-counts":      r"M1[-–]M[78]|eight pairs|nine pairs|K=\d|M=\d",
}

SURFACES = ("STATEMENT", "OBLIGATION", "CONSEQUENCE")
# §N -> surface. Anything unlisted, and the front matter above §1, is STATEMENT.
SURFACE_OF = {"8": "OBLIGATION", "10": "OBLIGATION",
              "6": "CONSEQUENCE", "7": "CONSEQUENCE"}
HEADING = re.compile(r"^#{1,6}\s*§\s*(\d+)")


def section_map(lines):
    """1 entry per line: (section label, surface). A heading belongs to itself."""
    out, cur = [], ("(front matter)", "STATEMENT")
    for line in lines:
        m = HEADING.match(line)
        if m:
            cur = (line.lstrip("#").strip(), SURFACE_OF.get(m.group(1), "STATEMENT"))
        out.append(cur)
    return out


def window(line, start, end, width):
    """Context centred on [start,end), the match delimited, whitespace collapsed."""
    pre = re.sub(r"\s+", " ", line[max(0, start - width):start])
    post = re.sub(r"\s+", " ", line[end:end + width])
    lead = "…" if start > width else ""
    tail = "…" if end + width < len(line) else ""
    return f"{lead}{pre}»{line[start:end]}«{post}{tail}"


def sweep(name, pattern, lines, sections, width):
    rx = re.compile(pattern, re.I)
    found = {s: [] for s in SURFACES}
    for i, line in enumerate(lines):
        for m in rx.finditer(line):
            if m.start() == m.end():
                continue  # zero-width match: nothing to show
            label, surface = sections[i]
            found[surface].append((i + 1, m.start(), label,
                                   window(line, m.start(), m.end(), width)))
    total = sum(len(v) for v in found.values())
    span = len({h[0] for v in found.values() for h in v})
    print(f"\n=== {name} ===  pattern: {pattern}")
    print(f"    {total} match(es) across {span} line(s)")
    for surface in SURFACES:
        hits = found[surface]
        if not hits:
            print(f"\n  --- {surface} (0 sites) ---   <<< EMPTY — a surface with "
                  f"no hits is the failure mode this tool exists to catch: either "
                  f"the pattern is too narrow, or this surface really was left "
                  f"standing. Read it before believing the zero.")
            continue
        print(f"\n  --- {surface} ({len(hits)} sites) ---")
        for n, off, label, text in hits:
            print(f"    L{n} c{off}  [{label}]")
            print(f"      {text}")
    print("\n  summary: " + " | ".join(
        f"{s} {len(found[s])}" + (" (0 sites)" if not found[s] else "") for s in SURFACES))
    return total


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0],
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("memo", help="path to the plan-memo")
    ap.add_argument("--pattern", action="append", metavar="RE",
                    help="ad-hoc regex to sweep (repeatable); the primary interface")
    ap.add_argument("--concept", action="append", metavar="ID",
                    help="frozen preset by name prefix, e.g. D5 (repeatable)")
    ap.add_argument("--window", type=int, default=90, metavar="N",
                    help="context chars either side of a match (default 90)")
    ap.add_argument("--list-concepts", action="store_true", help="print the presets and exit")
    args = ap.parse_args(argv)

    if args.list_concepts:
        for name, pat in CONCEPTS.items():
            print(f"{name}\n    {pat}")
        return 0

    jobs = [(f"--pattern {p}", p) for p in (args.pattern or [])]
    for want in (args.concept or []):
        matched = [(n, p) for n, p in CONCEPTS.items() if n.startswith(want)]
        if not matched:
            ap.error(f"no preset starts with {want!r} (see --list-concepts)")
        jobs += matched
    if not jobs:  # no selector: replay every preset
        jobs = list(CONCEPTS.items())

    lines = pathlib.Path(args.memo).read_text().split("\n")
    sections = section_map(lines)
    for name, pattern in jobs:
        sweep(name, pattern, lines, sections, args.window)
    print("\nLexical sweep only — a restatement sharing no token with the pattern "
          "is not listed. See the module docstring.")
    return 0


if __name__ == "__main__":
    main()
    sys.exit(0)
