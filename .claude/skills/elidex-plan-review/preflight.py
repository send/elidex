#!/usr/bin/env python3
"""elidex-plan-review Step 0 preflight — Spec coverage map check.

Verifies that a plan-memo includes a §3-style "Spec coverage map" section
with a markdown table conforming to the schema in
`feedback_plan-scope-re-evaluation.md`. Counts breadth (K=unique specs,
M=total entries), runs webref verification on each parsed citation, and
prints a split-decision verdict.

Hard-fail conditions (exit 1):
  - No "Spec coverage map" heading found in plan-memo
  - Heading found but no markdown table follows it
  - Table has 0 data rows
  - Any citation fails webref verification (use --no-verify to skip)

Soft-warn conditions (exit 0 with warning):
  - K >= 6 OR M >= 30 → SPLIT-DEFAULT (single-PR needs explicit justification)
  - K >= 4 OR M >= 20 → SPLIT-RECOMMENDED
  - Header columns differ from expected schema
  - Spec label not recognized (warns + skips verify for that row)

Sibling rule: `feedback_plan-memo-pre-verify-grep.md` covers impl-side
verification (Op/fn/handler grep); this script covers the spec side.
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
WEBREF = REPO_ROOT / ".claude" / "tools" / "webref"

# Maps plan-memo spec-label text → webref shortname. Mirror of
# `.claude/tools/webref` `_SPEC_LABEL_MAP` but reversed; keep in sync when
# adding new specs to that map. Aliases (e.g. "HTML" without "WHATWG") are
# tolerated for plan-memos that abbreviate.
SPEC_LABEL_REVERSE = {
    "ECMA-262": "ecma262",
    "ECMA-402": "ecma402",
    "WHATWG HTML": "html",
    "WHATWG DOM": "dom",
    "WHATWG URL": "url",
    "WHATWG Fetch": "fetch",
    "WHATWG Streams": "streams",
    "WHATWG XHR": "xhr",
    "Web Cryptography API": "webcrypto",
    "Web IDL": "webidl",
    "CSS Selectors L4": "selectors-4",
    "Geometry Interfaces L1": "geometry-1",
    "HTML": "html",
    "DOM": "dom",
    "URL": "url",
}

HEADING_RE = re.compile(r"^(#{1,6})\s+.*spec\s+coverage\s+map", re.IGNORECASE)
TABLE_ROW_RE = re.compile(r"^\|(.+)\|\s*$")
SEPARATOR_CELL_RE = re.compile(r"^:?-+:?$")
SECTION_REF_RE = re.compile(r"§([\d.A-B]+)")

EXPECTED_COLUMNS = ["spec section", "step", "branch", "touch", "full enum", "user-input flow"]


def find_coverage_map_section(lines: list[str]) -> tuple[int, int, int] | None:
    """Locate Spec coverage map section.

    Returns (heading_line, body_start, body_end) where body covers lines
    after the heading until the next heading at same/shallower level.
    Returns None if no Spec coverage map heading found.
    """
    for i, line in enumerate(lines):
        m = HEADING_RE.match(line)
        if not m:
            continue
        heading_level = len(m.group(1))
        body_start = i + 1
        body_end = len(lines)
        for j in range(body_start, len(lines)):
            line_j = lines[j]
            if not line_j.startswith("#"):
                continue
            # Count leading '#'s
            level = len(line_j) - len(line_j.lstrip("#"))
            if 0 < level <= heading_level:
                body_end = j
                break
        return (i, body_start, body_end)
    return None


def find_table(lines: list[str], start: int, end: int) -> list[list[str]] | None:
    """Find the first markdown table in lines[start:end], return parsed rows."""
    table_start = None
    for i in range(start, end):
        if TABLE_ROW_RE.match(lines[i]):
            table_start = i
            break
    if table_start is None:
        return None
    rows = []
    for i in range(table_start, end):
        m = TABLE_ROW_RE.match(lines[i])
        if not m:
            break
        cells = [c.strip() for c in m.group(1).split("|")]
        rows.append(cells)
    return rows if len(rows) >= 2 else None


def is_separator_row(row: list[str]) -> bool:
    return all(SEPARATOR_CELL_RE.match(c.strip()) for c in row if c.strip())


def verify_header_columns(header: list[str]) -> list[str]:
    """Return list of expected columns missing from header (empty = ok)."""
    header_norm = [h.lower() for h in header]
    missing = []
    for expected in EXPECTED_COLUMNS:
        if not any(expected in h for h in header_norm):
            missing.append(expected)
    return missing


def parse_spec_cell(cell: str) -> tuple[str | None, str | None]:
    """Extract (spec-label-text, section-number) from a Spec section cell.

    Examples:
      "ECMA-262 §15.7.14 Runtime Semantics: ..." → ("ECMA-262", "15.7.14")
      "WHATWG HTML §4.13.4 step 6.f.ii"          → ("WHATWG HTML", "4.13.4")
    Returns (None, None) if no §-reference is found.
    """
    m = SECTION_REF_RE.search(cell)
    if not m:
        return (None, None)
    section_num = m.group(1).rstrip(".")
    spec_label_text = cell[: m.start()].strip()
    return (spec_label_text, section_num)


def shortname_from_label(label: str | None) -> str | None:
    if not label:
        return None
    if label in SPEC_LABEL_REVERSE:
        return SPEC_LABEL_REVERSE[label]
    for k, v in SPEC_LABEL_REVERSE.items():
        if k.lower() == label.lower():
            return v
    return None


def verify_citation(shortname: str, section: str) -> tuple[bool, str]:
    """Invoke webref to verify citation. Returns (ok, message)."""
    if not WEBREF.is_file():
        return (False, f"webref tool missing at {WEBREF}")
    try:
        result = subprocess.run(
            [str(WEBREF), "heading", shortname, section],
            capture_output=True, text=True, timeout=30,
        )
    except subprocess.TimeoutExpired:
        return (False, "webref timeout (30s)")
    if result.returncode != 0:
        first = (result.stderr or result.stdout or "").strip().splitlines()
        return (False, first[0] if first else "unknown failure")
    return (True, "ok")


def main() -> int:
    p = argparse.ArgumentParser(
        description=__doc__.splitlines()[0] if __doc__ else "",
    )
    p.add_argument("plan_memo", help="path to plan-memo markdown file")
    p.add_argument("--no-verify", action="store_true",
                   help="skip webref citation verify (structure + breadth only)")
    p.add_argument("--strict-breadth", action="store_true",
                   help="treat SPLIT-DEFAULT (K>=6 or M>=30) as hard fail")
    args = p.parse_args()

    plan_path = Path(args.plan_memo)
    if not plan_path.is_file():
        print(f"preflight: plan-memo not found: {plan_path}", file=sys.stderr)
        return 1

    lines = plan_path.read_text(encoding="utf-8").splitlines()

    section = find_coverage_map_section(lines)
    if section is None:
        print("preflight: ❌ HARD FAIL — no 'Spec coverage map' heading in plan-memo.",
              file=sys.stderr)
        print("  Add a `## §3. Spec coverage map` section per "
              "feedback_plan-scope-re-evaluation.md.", file=sys.stderr)
        print("  Generate a starter table with:", file=sys.stderr)
        print("    .claude/tools/webref coverage-map <spec> <ref> [<spec> <ref> ...]",
              file=sys.stderr)
        return 1

    heading_line, body_start, body_end = section
    table = find_table(lines, body_start, body_end)
    if table is None:
        print(f"preflight: ❌ HARD FAIL — Spec coverage map heading at line "
              f"{heading_line + 1} but no markdown table follows it "
              f"(searched through line {body_end}).", file=sys.stderr)
        return 1

    header = table[0]
    missing = verify_header_columns(header)
    if missing:
        print(f"preflight: ⚠ table header missing expected columns: "
              f"{', '.join(missing)}", file=sys.stderr)
        print(f"  found columns: {header}", file=sys.stderr)

    # Skip header + separator (if present)
    data_rows = table[2:] if len(table) >= 2 and is_separator_row(table[1]) else table[1:]
    if not data_rows:
        print("preflight: ❌ HARD FAIL — Spec coverage map table has 0 data rows.",
              file=sys.stderr)
        return 1

    specs_seen: dict[str, int] = {}
    unparseable = 0
    citations: list[tuple[str, str]] = []  # (shortname, section) per row
    unrecognized_labels: list[str] = []
    for row in data_rows:
        if not row:
            continue
        spec_cell = row[0] if row else ""
        label, section_num = parse_spec_cell(spec_cell)
        if section_num is None:
            unparseable += 1
            continue
        shortname = shortname_from_label(label)
        if shortname is None:
            unrecognized_labels.append(label or "<empty>")
            unparseable += 1
            continue
        specs_seen[shortname] = specs_seen.get(shortname, 0) + 1
        citations.append((shortname, section_num))

    K = len(specs_seen)
    M = sum(specs_seen.values())

    verify_failed: list[tuple[str, str, str]] = []
    if not args.no_verify and citations:
        seen_pairs: set[tuple[str, str]] = set()
        for shortname, section_num in citations:
            key = (shortname, section_num)
            if key in seen_pairs:
                continue
            seen_pairs.add(key)
            ok, msg = verify_citation(shortname, section_num)
            if not ok:
                verify_failed.append((shortname, section_num, msg))

    # Summary
    print(f"§3 Spec coverage map preflight — {plan_path.name}")
    print(f"  heading line:         {heading_line + 1}")
    print(f"  data rows:            {len(data_rows)}")
    print(f"  parsed citations:     {M}")
    print(f"  unparseable rows:     {unparseable}")
    print(f"  unique specs (K):     {K} "
          f"({', '.join(sorted(specs_seen)) if specs_seen else '-'})")
    print(f"  total entries  (M):   {M}")

    if unrecognized_labels:
        print(f"  unrecognized labels:  {sorted(set(unrecognized_labels))}")
        print(f"    (extend SPEC_LABEL_REVERSE in preflight.py to add coverage)")

    breadth_hard_fail = False
    if K >= 6 or M >= 30:
        verdict = "⚠ SPLIT-DEFAULT (K>=6 or M>=30)"
        advice = "分割 default — single-PR を維持するなら plan-memo §3 narrative に正当化を明記。"
        breadth_hard_fail = args.strict_breadth
    elif K >= 4 or M >= 20:
        verdict = "SPLIT-RECOMMENDED (K>=4 or M>=20)"
        advice = "分割推奨 — per-PR scope を再評価。"
    else:
        verdict = "ok (single PR scope)"
        advice = ""
    print(f"  split decision:       {verdict}")
    if advice:
        print(f"                        {advice}")

    if not args.no_verify:
        if verify_failed:
            print()
            print(f"⚠ citation verification — {len(verify_failed)} failure(s):")
            for shortname, section_num, msg in verify_failed:
                print(f"  - {shortname} §{section_num}: {msg}")
        else:
            print(f"  citation verify:      ok ({len(citations)} citation(s) checked)")

    if verify_failed or breadth_hard_fail:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
