#!/usr/bin/env python3
"""Concept sweep for a plan-memo: for each changed decision, list EVERY site
that restates it, so all can be re-derived in one edit (plan-memo §13.4)."""
import re, sys, pathlib

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

def main(path, only=None):
    lines = pathlib.Path(path).read_text().split("\n")
    for name, pat in CONCEPTS.items():
        if only and not name.startswith(only):
            continue
        rx = re.compile(pat, re.I)
        hits = [(i+1, l) for i, l in enumerate(lines) if rx.search(l)]
        print(f"\n=== {name}  ({len(hits)} sites) ===")
        for n, l in hits:
            sec = l.strip()[:110]
            print(f"  {n:>4}: {sec}")

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else None)
