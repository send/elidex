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

# Require a §-number marker in the heading so a section titled e.g.
# "Quick Reference: spec coverage map" doesn't accidentally pick up.
# Number flexibility: §3 / §2.5 / §3.1 / annex letters are all accepted
# per `feedback_plan-scope-re-evaluation.md`'s "§3 (or §2.5)" wording.
HEADING_RE = re.compile(
    r"^(#{1,6})\s+§[\d.A-Z]+\.?\s+.*spec\s+coverage\s+map", re.IGNORECASE
)
FENCE_RE = re.compile(r"^\s*(```|~~~)")
SEPARATOR_CELL_RE = re.compile(r"^:?-+:?$")
# Section numbers contain only digits, dots, and uppercase annex letters
# (A through Z — tc39 annexes A-G, W3C/WHATWG occasionally further). AO
# names always have lowercase (CamelCase) so won't match. Don't enumerate
# specific annex letters — that drifts when a spec adds annex H+.
# `\s*` after `§` tolerates copy/paste with a space after the section
# mark (`§ 15.7.14`); webref `_lookup_section` does the same normalization.
SECTION_REF_RE = re.compile(r"§\s*([\d.A-Z]+)")


def _fence_state_array(lines: list[str]) -> list[bool]:
    """Per-line bool: True iff `lines[i]` is inside (or is a marker of) a
    fenced code block.

    Single SoT for fence tracking shared by `find_coverage_map_section` and
    `find_table` — fence markers (``` / ~~~) start/end blocks, and only
    matching markers close. Both the opener and closer lines themselves
    are flagged True so heading/table detection skips them.
    """
    state = [False] * len(lines)
    in_fence = False
    marker: str | None = None
    for i, line in enumerate(lines):
        m = FENCE_RE.match(line)
        if m:
            current = m.group(1)
            if not in_fence:
                in_fence = True
                marker = current
                state[i] = True
            elif marker == current:
                in_fence = False
                marker = None
                state[i] = True
            else:
                state[i] = True
        else:
            state[i] = in_fence
    return state


def _parse_table_row(line: str) -> list[str] | None:
    """Return GFM table row cells, or None if `line` isn't a table row.

    Accepts both forms:
      - With outer pipes: `| a | b | c |`
      - Without outer pipes: `a | b | c`
    A row must contain at least one `|` and split into ≥ 2 cells. Outer
    `|`s and surrounding whitespace are stripped before splitting.
    """
    stripped = line.strip()
    if not stripped or "|" not in stripped:
        return None
    if stripped.startswith("|"):
        stripped = stripped[1:]
    if stripped.endswith("|"):
        stripped = stripped[:-1]
    cells = [c.strip() for c in stripped.split("|")]
    return cells if len(cells) >= 2 else None

EXPECTED_COLUMNS = ["spec section", "step", "branch", "touch", "full enum", "user-input flow"]


def find_coverage_map_section(
    lines: list[str], fence_state: list[bool]
) -> tuple[int, int, int] | None:
    """Locate Spec coverage map section, skipping fenced code blocks.

    Returns (heading_line, body_start, body_end) where body covers lines
    after the heading until the next heading at same/shallower level.
    `fence_state[i]` (from `_fence_state_array`) gates BOTH the outer
    heading scan (a §3 template inside an earlier fenced block won't be
    picked up as a real heading) and the body terminator scan (a `#`
    comment inside a python/bash snippet won't end the §3 body early).
    Returns None if no Spec coverage map heading found outside fences.
    """
    for i, line in enumerate(lines):
        if fence_state[i]:
            continue
        m = HEADING_RE.match(line)
        if not m:
            continue
        heading_level = len(m.group(1))
        body_start = i + 1
        body_end = len(lines)
        for j in range(body_start, len(lines)):
            if fence_state[j]:
                continue
            line_j = lines[j]
            if not line_j.startswith("#"):
                continue
            level = len(line_j) - len(line_j.lstrip("#"))
            if 0 < level <= heading_level:
                body_end = j
                break
        return (i, body_start, body_end)
    return None


def find_table(
    lines: list[str], start: int, end: int, fence_state: list[bool]
) -> list[list[str]] | None:
    """Find the first GFM table in lines[start:end], skipping fenced blocks.

    A GFM table is identified by a header row followed by a separator row
    (cells matching `^:?-+:?$`). This is the unambiguous GFM marker and
    avoids false positives on prose containing `|`. Both outer-pipe and
    no-outer-pipe forms are accepted via `_parse_table_row()`.
    `fence_state[i]` skips fenced code blocks so embedded markdown samples
    don't get treated as tables.
    """
    for i in range(start, end - 1):
        if fence_state[i]:
            continue
        header_cells = _parse_table_row(lines[i])
        if header_cells is None:
            continue
        sep_cells = _parse_table_row(lines[i + 1])
        if sep_cells is None or not is_separator_row(sep_cells):
            continue
        rows = [header_cells, sep_cells]
        for j in range(i + 2, end):
            if fence_state[j]:
                break
            row = _parse_table_row(lines[j])
            if row is None:
                break
            rows.append(row)
        return rows
    return None


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
    """Invoke webref to verify citation. Returns (ok, message).

    Uses `webref heading --exact` so partial citations don't silently pass
    via prefix-tree matching (e.g. `§4.13` passing because `§4.13.1` exists).
    The drift-catch invariant is "section number = exact clause".

    Invokes webref via `sys.executable` (mirrors SKILL.md's `python3 path`
    pattern) so verify works on environments that don't preserve exec bits
    (Windows git, some CI runners) — same defensive choice as F12.
    """
    if not WEBREF.is_file():
        return (False, f"webref tool missing at {WEBREF}")
    try:
        result = subprocess.run(
            [sys.executable, str(WEBREF), "heading", "--exact", shortname, section],
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
    fence_state = _fence_state_array(lines)

    section = find_coverage_map_section(lines, fence_state)
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
    table = find_table(lines, body_start, body_end, fence_state)
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

    # Distinguish two unparseable modes:
    #   - malformed:  row has no `§<number>` reference at all → hard-fail
    #     candidate (truly malformed §3 cell)
    #   - unmapped:   row has `§<number>` but the label isn't in
    #     SPEC_LABEL_REVERSE → soft-warn (verify skipped, but row still
    #     counts toward breadth so author intent isn't masked)
    specs_seen: dict[str, int] = {}
    malformed_rows = 0
    malformed_row_details: list[tuple[int, str]] = []  # (1-based row idx, cell preview)
    unmapped_rows = 0
    citations: list[tuple[str, str]] = []
    unrecognized_labels: list[str] = []
    unique_specs: set[str] = set()  # K basis (mapped shortname OR unmapped label)
    for idx, row in enumerate(data_rows, start=1):
        if not row:
            malformed_rows += 1
            malformed_row_details.append((idx, "<empty row>"))
            continue
        spec_cell = row[0] if row else ""
        label, section_num = parse_spec_cell(spec_cell)
        if section_num is None:
            malformed_rows += 1
            preview = spec_cell.strip()[:80] if spec_cell.strip() else "<empty>"
            malformed_row_details.append((idx, preview))
            continue
        shortname = shortname_from_label(label)
        if shortname is None:
            unrecognized_labels.append(label or "<empty>")
            unmapped_rows += 1
            unique_specs.add(f"unmapped:{label}" if label else "unmapped:<empty>")
            continue
        specs_seen[shortname] = specs_seen.get(shortname, 0) + 1
        citations.append((shortname, section_num))
        unique_specs.add(shortname)

    # Breadth = author's intent surface (total data rows). Both verification
    # success (parsed_count) and unmapped rows are reported separately.
    # `K` counts unique spec identifiers including unmapped labels — a plan
    # citing "WHATWG XR §1" alongside "ECMA-262 §15" represents two distinct
    # specs regardless of whether the labels map to webref shortnames.
    K = len(unique_specs)
    M = len(data_rows)
    parsed_count = sum(specs_seen.values())

    # Hard-fail on ANY malformed row (no `§<number>` reference): the §3
    # table's structural-integrity invariant is "every row contains
    # §<number>". A single malformed row would let that row's intended
    # citation slip past the verify gate. Unmapped labels are still
    # soft-warn (verify skipped per row, table still counts toward
    # breadth) — the label-map is a verification-depth choice, not a
    # structural invariant.
    malformed_hard_fail = malformed_rows > 0

    verify_failed: list[tuple[str, str, str]] = []
    seen_pairs: set[tuple[str, str]] = set()
    if not args.no_verify and citations:
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
    print(f"  total entries  (M):   {M}  (data rows, breadth basis)")
    print(f"  parsed citations:     {parsed_count}")
    print(f"  malformed rows:       {malformed_rows}  (no §<number> reference)")
    print(f"  unmapped-label rows:  {unmapped_rows}  (has §<number>, label not in SPEC_LABEL_REVERSE)")
    displayed_specs = sorted(specs_seen)
    if unrecognized_labels:
        displayed_specs.extend(f"<{lbl}>" for lbl in sorted(set(unrecognized_labels)))
    print(f"  unique specs (K):     {K} "
          f"({', '.join(displayed_specs) if displayed_specs else '-'})")

    if unrecognized_labels:
        # Soft-warn → stderr (matches the docstring's "unmapped labels =
        # soft-warn" semantics and keeps stdout copy-paste-clean for
        # consumers that just want the summary).
        print(f"  ⚠ unrecognized labels: {sorted(set(unrecognized_labels))}",
              file=sys.stderr)
        print("    (extend SPEC_LABEL_REVERSE in preflight.py to add coverage)",
              file=sys.stderr)

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
            # Stderr + explicit HARD FAIL header — matches malformed_hard_fail
            # diagnostic shape so CI grepping for "HARD FAIL" catches both.
            print(f"\npreflight: ❌ HARD FAIL — citation verification: "
                  f"{len(verify_failed)} failure(s)", file=sys.stderr)
            for shortname, section_num, msg in verify_failed:
                print(f"  - {shortname} §{section_num}: {msg}", file=sys.stderr)
        elif seen_pairs:
            print(f"  citation verify:      ok ({len(seen_pairs)} unique citation(s) checked)")

    if malformed_hard_fail:
        print()
        print(f"preflight: ❌ HARD FAIL — {malformed_rows} of {len(data_rows)} "
              f"row(s) missing `§<number>` reference.", file=sys.stderr)
        print("  Spec section cells must contain `§<number>` (e.g. "
              "`ECMA-262 §15.7.14 ...`).", file=sys.stderr)
        for row_idx, preview in malformed_row_details:
            print(f"  - row {row_idx}: {preview!r}", file=sys.stderr)
        print("  Run `.claude/tools/webref coverage-map <spec> <ref> ...` "
              "to regenerate the table.", file=sys.stderr)
        return 1

    if breadth_hard_fail:
        print("\npreflight: ❌ HARD FAIL — breadth split-default "
              "(K>=6 or M>=30) with --strict-breadth",
              file=sys.stderr)
        return 1
    if verify_failed:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
