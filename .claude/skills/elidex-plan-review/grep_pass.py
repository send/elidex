"""elidex-plan-review preflight grep-pass — Check 1/2/3.

Catches plan-memo §3-§7 structural reference drift at plan-stage rather
than via reactive R-loop discovery. Three checks:

  Check 1 — File path existence (HARD fail by default)
    Extract `crates/...\\.rs(:N)?` references; hard-fail if file missing,
    soft-warn if `:N` overshoots file line count.

  Check 2 — Rust symbol grep verification (SOFT warn + NEW exemption)
    Extract backticked `path::fn` / `Type::method` symbols; soft-warn if
    grep finds 0 hits in crates/ AND no `(NEW)` / `(planned)` /
    `(<PR-ID> surface)` annotation in ±100-char vicinity. Skip symbols
    inside fenced code blocks and inside the §3 Spec coverage map (spec
    citation domain).

  Check 3 — Enumeration claim verification artifact audit (SOFT warn)
    Extract `\\d+\\+? (callers|sites|modules|...)`; soft-warn if no
    verification artifact (grep command literal, `verified YYYY-MM-DD`
    phrase, or explicit line list) in ±200-char vicinity.

Override flags:
  --no-grep-pass       skip all 3 checks (offline / WIP plan-memo)
  --strict-symbols     escalate Check 2 to HARD fail
  --strict-enum        escalate Check 3 to HARD fail

Spec: memory/m4-12-pr-elidex-plan-review-grep-pass-spec.md
Sibling lesson: feedback_plan-memo-pre-verify-grep.md (reactive form;
this module is the preflight enforcement).
"""
from __future__ import annotations

import re
import subprocess
from pathlib import Path

FILE_PATH_RE = re.compile(r"`?(crates/[^\s`:)]+\.rs)(?::(\d+))?`?")

# Match `path::fn` / `mod::Type` / `Type::method`. Single `foo` skipped
# to avoid false-positive noise on common prose words. `::` chain required.
RUST_SYMBOL_RE = re.compile(
    r"`("
    r"[a-z_][a-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+"  # path::fn / mod::Type
    r"|"
    r"[A-Z][A-Za-z0-9_]*::[a-z_][A-Za-z0-9_]*"        # Type::method
    r")`"
)

# Allow 0-3 intervening words between number and unit to catch natural
# phrasings ("30+ TypedArray methods", "8 BufferSource consumer modules")
# — the spec pseudocode's `\s+(units)` was too tight and missed both R3
# and R2 drifts it was designed to catch. Intervening token is `[\w.]+`
# (word chars + dots, for "TypedArray.prototype"). 3-word cap bounds
# false-positive risk for prose like "30 of the 17 callers".
ENUM_CLAIM_RE = re.compile(
    r"\b(\d+)\+?\s+(?:[\w.]+\s+){0,3}"
    r"(callers|sites|modules|methods|entries|consumers|paths|files)\b",
    re.IGNORECASE,
)

# Vicinity exemption: `(NEW)`, `(planned)`, `(F3 surface)`, `(D-16 surface)`,
# `(NEW: …)`, case-insensitive.
NEW_ANNOTATION_RE = re.compile(
    r"\((?:NEW(?::[^)]*)?|planned|[A-Z]\d*-?\d*\s+surface)\)",
    re.IGNORECASE,
)

# Verification artifact = grep command literal, "verified YYYY-MM-DD" phrase,
# or explicit line list (`lines 111/180/227`).
VERIFICATION_ARTIFACT_RE = re.compile(
    r"grep\s+-[rlnEFhio]+"
    r"|verified\s+\d{4}-\d{2}-\d{2}"
    r"|lines?\s+\d+(?:/\d+)+"
)

# §3 Spec coverage map heading detection (mirrors preflight.py HEADING_RE
# wording so both detect the same section).
COVERAGE_MAP_HEADING_RE = re.compile(
    r"^(#{1,6})\s+§[\d.A-Z]+\.?\s+.*spec\s+coverage\s+map",
    re.IGNORECASE | re.MULTILINE,
)
NEXT_HEADING_RE = re.compile(r"^#{1,6}\s+§[\d.A-Z]", re.MULTILINE)


def run_grep_pass(
    plan_memo: Path,
    repo_root: Path,
    strict_symbols: bool = False,
    strict_enum: bool = False,
) -> list[tuple[str, str]]:
    """Run all 3 grep-pass checks; return list of (severity, message).

    `severity` ∈ {"HARD", "SOFT"}. Caller is responsible for merging into
    the preflight HARD FAIL / SOFT WARN output pipeline and for the
    eventual exit-code decision (any HARD finding → exit 1).
    """
    content = plan_memo.read_text(encoding="utf-8")
    findings: list[tuple[str, str]] = []

    code_block_spans = _code_block_spans(content)
    coverage_map_span = _coverage_map_span(content)

    # --- Check 1: file path existence -------------------------------------
    for m in FILE_PATH_RE.finditer(content):
        if _in_spans(m.start(), code_block_spans):
            # File path inside a fenced code block — likely an example /
            # template, not a structural reference. Skip.
            continue
        path_str, line_str = m.group(1), m.group(2)
        # Shell-syntax shorthand (brace expansion / glob) doesn't resolve
        # to a literal filesystem path — common in plan-memos to enumerate
        # a tree of related files (e.g. `host/{module,instance,memory}.rs`).
        # Soft-warn so author sees them but doesn't block landing on a
        # phantom path; per-variant expansion is followup work.
        if any(ch in path_str for ch in "{}*?[]"):
            findings.append((
                "SOFT",
                f"path {path_str} contains shell glob/brace syntax; "
                f"cannot verify (expand to individual paths if structural)",
            ))
            continue
        full = repo_root / path_str
        if not full.exists():
            # NEW exemption (mirrors Check 2): planned new files are a
            # valid reason to reference a non-existent path. Spec missed
            # this — only required for `(NEW)` / `(planned)` / `(<PR-ID>
            # surface)` annotations, matching Check 2's exemption pattern.
            vicinity = content[max(0, m.start() - 100): m.end() + 100]
            if NEW_ANNOTATION_RE.search(vicinity):
                continue
            findings.append((
                "HARD",
                f"path {path_str} does not exist "
                f"(annotate (NEW) if planned)",
            ))
        elif line_str:
            try:
                with full.open(encoding="utf-8", errors="ignore") as fh:
                    line_count = sum(1 for _ in fh)
            except OSError as e:
                findings.append(("SOFT", f"cannot count lines for {path_str}: {e}"))
                continue
            if int(line_str) > line_count:
                findings.append((
                    "SOFT",
                    f"{path_str}:{line_str} out of range (file has {line_count} lines)",
                ))

    # --- Check 2: Rust symbol grep verification ---------------------------
    crates_dir = repo_root / "crates"
    if crates_dir.exists():
        seen_symbols: set[str] = set()
        for m in RUST_SYMBOL_RE.finditer(content):
            if _in_spans(m.start(), code_block_spans):
                continue
            if coverage_map_span and coverage_map_span[0] <= m.start() < coverage_map_span[1]:
                # Inside §3 coverage map — spec citation domain, skip.
                continue
            symbol = m.group(1)
            if symbol in seen_symbols:
                continue
            seen_symbols.add(symbol)
            vicinity = content[max(0, m.start() - 100): m.end() + 100]
            if NEW_ANNOTATION_RE.search(vicinity):
                continue
            if not _grep_repo(symbol, crates_dir):
                severity = "HARD" if strict_symbols else "SOFT"
                findings.append((
                    severity,
                    f"symbol `{symbol}` not found in codebase; "
                    f"annotate (NEW) if planned",
                ))

    # --- Check 3: enumeration claim audit ---------------------------------
    for m in ENUM_CLAIM_RE.finditer(content):
        if _in_spans(m.start(), code_block_spans):
            # Code-block illustrations (template snippets) are not author
            # claims about reality — skip.
            continue
        vicinity = content[max(0, m.start() - 200): m.end() + 200]
        if VERIFICATION_ARTIFACT_RE.search(vicinity):
            continue
        severity = "HARD" if strict_enum else "SOFT"
        findings.append((
            severity,
            f"'{m.group(0)}' claim without verification artifact; "
            f"cache grep result inline (e.g. `grep -rn ... | wc -l` or "
            f"`verified YYYY-MM-DD`)",
        ))

    return findings


# --- Helpers --------------------------------------------------------------

def _code_block_spans(content: str) -> list[tuple[int, int]]:
    """Return (start, end) spans of fenced code blocks (triple backtick).

    Same fence-tracking shape as preflight.py `_fence_state_array`, but
    span-based rather than per-line. Tracks only ``` (not ~~~) since
    plan-memos predominantly use backtick fences; extending to ~~~ is
    trivial if needed.
    """
    spans: list[tuple[int, int]] = []
    in_block = False
    block_start = 0
    for m in re.finditer(r"```", content):
        if not in_block:
            block_start = m.start()
            in_block = True
        else:
            spans.append((block_start, m.end()))
            in_block = False
    return spans


def _coverage_map_span(content: str) -> tuple[int, int] | None:
    """Return (start, end) span of §3 Spec coverage map (heading → next §).

    Returns None if no Spec coverage map heading found. The end boundary is
    the start of the next `## §<N>` heading at same/shallower level (or
    end-of-file if none follows).
    """
    m = COVERAGE_MAP_HEADING_RE.search(content)
    if not m:
        return None
    start = m.start()
    nxt = NEXT_HEADING_RE.search(content, pos=m.end())
    end = nxt.start() if nxt else len(content)
    return (start, end)


def _in_spans(pos: int, spans: list[tuple[int, int]]) -> bool:
    return any(s <= pos < e for s, e in spans)


def _grep_repo(symbol: str, crates_dir: Path) -> bool:
    """Return True iff `symbol` is found in any file under `crates_dir`.

    Uses `grep -rln -F` (fixed string, no regex) so `::` etc don't need
    escaping. 30s timeout — way more than enough for a single grep over
    crates/, but bounds the worst case if filesystem hangs.
    """
    try:
        result = subprocess.run(
            ["grep", "-rln", "-F", symbol, str(crates_dir)],
            capture_output=True,
            text=True,
            timeout=30,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        # On timeout or missing grep binary, fall open (don't generate
        # spurious findings). Preflight is best-effort; the goal is to
        # catch obvious drift, not to be a hard correctness gate.
        return True
    return result.returncode == 0
