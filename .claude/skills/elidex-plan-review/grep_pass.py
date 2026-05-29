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
    inside fenced code blocks (``` or ~~~) and inside the §3 Spec
    coverage map span (heading → next heading at same-or-shallower
    level, includes ### §3.x subheadings per SKILL.md template).

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

# Match `path::fn` / `mod::Type` / `Type::method` / `Type::Variant` /
# `Type::CONST`. Single `foo` skipped to avoid false-positive noise on
# common prose words. `::` chain required. The `Type::*` alternative's
# post-`::` part accepts both upper and lower case (PR #243 Copilot R3
# IMP): original required lowercase, silently excluding the very common
# enum-variant shape (`Op::AssertConstructor`, `NodeKind::Element`, etc.)
# which is heavy in elidex's opcode/AST code.
RUST_SYMBOL_RE = re.compile(
    r"`("
    r"[a-z_][a-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+"  # path::fn / mod::Type
    r"|"
    r"[A-Z][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*"     # Type::method / Type::Variant / Type::CONST
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

    # Document-wide NEW-annotation exemption sets. Author intent for
    # "(NEW)" / "(planned)" / "(<PR-ID> surface)" is global to the document
    # (one canonical declaration site, possibly later than casual prose
    # references), so we pre-scan ALL occurrences once and exempt by
    # token value at check time. Original per-occurrence vicinity check
    # was order-dependent — if the first textual mention was bare and
    # later mentions added (NEW), the symbol/path was still flagged
    # (PR #243 Copilot R2 IMP). Applied uniformly to Check 1 and Check 2
    # since both shared the same per-occurrence bug shape.
    new_annotated_paths = _collect_new_annotated_tokens(
        content, FILE_PATH_RE, code_block_spans,
    )
    new_annotated_symbols = _collect_new_annotated_tokens(
        content, RUST_SYMBOL_RE, code_block_spans,
    )

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
            if path_str in new_annotated_paths:
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
        # Two-pass: collect candidate symbols first (after exclusions), then
        # bulk-grep in a single subprocess call so crates/ is walked once
        # rather than once per symbol. Order-preserving via seen-set.
        candidates: list[str] = []
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
            if symbol in new_annotated_symbols:
                continue
            candidates.append(symbol)
        if candidates:
            found = _grep_repo_bulk(candidates, crates_dir)
            for symbol in candidates:
                if symbol not in found:
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

# Fence marker at start of line (optionally indented), matching preflight.py
# `FENCE_RE`. Both ``` and ~~~ are supported (GFM-compliant), and only a
# matching marker closes the block (so ~~~ inside a ```-fenced block is
# treated as content, not a closer).
_FENCE_MARKER_RE = re.compile(r"^[ \t]*(```|~~~)", re.MULTILINE)


def _code_block_spans(content: str) -> list[tuple[int, int]]:
    """Return (start, end) spans of fenced code blocks (``` and ~~~).

    Same fence-tracking shape as preflight.py `_fence_state_array`, but
    span-based rather than per-line. Mirrors preflight.py in 3 respects:
      1. Both ``` and ~~~ fences are recognized
      2. Only a matching marker closes the block (mismatched marker is content)
      3. Fence must appear at line start (optionally indented)

    If a fence is opened but never closed (unclosed fence at EOF), the span
    extends to `len(content)` so subsequent content stays treated as fenced
    — preflight.py's per-line tracker has the same effect via its
    `in_fence` state being True at EOF.
    """
    spans: list[tuple[int, int]] = []
    in_block = False
    marker: str | None = None
    block_start = 0
    for m in _FENCE_MARKER_RE.finditer(content):
        current = m.group(1)
        if not in_block:
            in_block = True
            marker = current
            block_start = m.start()
        elif marker == current:
            in_block = False
            marker = None
            spans.append((block_start, m.end()))
        # else: mismatched marker (~~~ inside ```-block, or vice versa) — content
    if in_block:
        # Unclosed fence at EOF — extend the open span to end of content.
        spans.append((block_start, len(content)))
    return spans


def _coverage_map_span(content: str) -> tuple[int, int] | None:
    """Return (start, end) span of §3 Spec coverage map (heading → next heading
    at same-or-shallower level).

    Returns None if no Spec coverage map heading found. The end boundary is
    the start of the next heading whose `#` count is ≤ the coverage-map
    heading's level (regardless of § prefix) — so `### §3.1 ...`
    subheadings inside §3 (mandated by SKILL.md template) remain inside the
    span, while the next top-level `## §4 ...` (or any `# ...`) terminates
    it. Mirrors preflight.py `find_coverage_map_section` heading-level
    tracking — both files implement the same rule (line-based in preflight,
    char-based here).
    """
    m = COVERAGE_MAP_HEADING_RE.search(content)
    if not m:
        return None
    heading_level = len(m.group(1))  # count of `#` chars in the heading marker
    start = m.start()
    # Match any heading at level ≤ heading_level (no § requirement — a
    # non-§ heading like `## Conclusion` also terminates the section).
    next_re = re.compile(
        rf"^#{{1,{heading_level}}}\s+",
        re.MULTILINE,
    )
    nxt = next_re.search(content, pos=m.end())
    end = nxt.start() if nxt else len(content)
    return (start, end)


def _in_spans(pos: int, spans: list[tuple[int, int]]) -> bool:
    return any(s <= pos < e for s, e in spans)


def _collect_new_annotated_tokens(
    content: str,
    token_re: re.Pattern[str],
    code_block_spans: list[tuple[int, int]],
) -> set[str]:
    """Return tokens (from `token_re.group(1)`) that have a NEW annotation
    within ±100-char vicinity of ANY of their occurrences in `content`.

    Used by both Check 1 (paths) and Check 2 (symbols) so the "NEW
    exemption applies if author marks it anywhere" semantic holds
    document-wide rather than only at the first textual mention.

    Skips occurrences inside fenced code blocks (template examples don't
    declare anything). Does NOT skip §3 coverage map occurrences — a
    `(NEW)` annotation inside the coverage map still expresses author
    intent that the token is planned.
    """
    annotated: set[str] = set()
    for m in token_re.finditer(content):
        if _in_spans(m.start(), code_block_spans):
            continue
        vicinity = content[max(0, m.start() - 100): m.end() + 100]
        if NEW_ANNOTATION_RE.search(vicinity):
            annotated.add(m.group(1))
    return annotated


def _grep_repo_bulk(symbols: list[str], crates_dir: Path) -> set[str]:
    """Return the subset of `symbols` found in any `.rs` file under `crates_dir`.

    Single subprocess call (vs one grep per symbol): all symbols passed as
    `-e <pat>` arguments + `--include='*.rs' --exclude-dir=target` so
    `crates/` is walked once and build artifacts are skipped. Matched files
    are then read into memory and each symbol checked with Python `in` so
    per-symbol hit/miss is recovered (grep alone can't tell which `-e`
    matched). Fixed-string match (`-F`) so `::` doesn't need escaping.

    Falls open (returns the full input set as "found") on subprocess
    failure — preflight is best-effort, not a hard correctness gate.
    """
    if not symbols:
        return set()
    cmd = [
        "grep", "-rln", "-F",
        "--include=*.rs", "--exclude-dir=target",
    ]
    for s in symbols:
        cmd.extend(["-e", s])
    cmd.append(str(crates_dir))
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=60,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return set(symbols)
    matched_files = [f for f in result.stdout.splitlines() if f]
    if not matched_files:
        return set()
    contents: list[str] = []
    for fp in matched_files:
        try:
            contents.append(
                Path(fp).read_text(encoding="utf-8", errors="ignore")
            )
        except OSError:
            continue
    return {sym for sym in symbols if any(sym in c for c in contents)}
