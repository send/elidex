#!/usr/bin/env python3
"""Firing proofs and controls for `plan-memo-umbrella-check.py`.

A checker that has never been shown to fire is a checker that reports `0`
for a class it cannot see.  Every check in the companion has at least one
control here, and the controls come in four kinds:

  POSITIVE          a wrong site the checker MUST report.
  POSITIVE-NOVEL    a wrong site written in a spelling that appears NOWHERE in
                    the memo and nowhere in the licensing tables.  This is the
                    control that decides whether the predicate is a rule or a
                    transcription of the sites it was written against.  If a
                    novel wrong spelling passes, the checker is a grep for
                    yesterday's mistakes.
  NEGATIVE          a licensed site the checker must NOT report.
  KNOWN-MISS        a wrong site the checker is KNOWN not to report.  These are
                    RED and stay red.  They are printed with the run so a
                    reader never reads the finding count as a coverage
                    statement.  Turning one green by widening the claim -- for
                    instance by redefining the population so the missed site is
                    out of scope -- is the failure
                    `feedback_control-rewritten-to-bless-the-defect` names.

Run:  python3 .claude/tools/plan-memo-umbrella-check.py --self-test
"""

import importlib.util
import pathlib
import sys
import tempfile

from plan_memo_selftest_cases import ASSERT_CASES, CASES, build
from plan_memo_tables import MARKER

HERE = pathlib.Path(__file__).resolve().parent


def _load():
    spec = importlib.util.spec_from_file_location(
        "pmuc", HERE / "plan-memo-umbrella-check.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


M = _load()


def run_on(text, prose="", sibling=None):
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "fixture.md"
        p.write_text(text + "\n" + prose + "\n")
        # Every fixture link resolves to this file (discovery reads the memo's
        # links and an absent target is a FATAL in production); its name
        # carries an id so the destination-masking control keeps its subject.
        (pathlib.Path(d) / "slice-9z-sib.md").write_text((sibling or "") + "\n")
        memo = M.Memo(str(p))
        # ⚠ The PRODUCTION population.  This passed `umbrella_ids()` while
        # `main()` passes `no_owner_ids()`, so a regression dropping
        # kind-undetermined naming detection left `--self-test` green while the
        # comment two lines below claimed the harness runs the same pipeline.
        umb = memo.no_owner_ids()
        # The SAME pipeline main() runs -- not a copy of it.  The copy that
        # stood here had a different dedup rule and omitted two assertions.
        mentions = M.collect_mentions(memo, umb)
        seen, dd = set(), []
        for m in mentions:
            if m.key in seen:
                continue
            seen.add(m.key)
            dd.append(m)
        mentions = dd
        findings, notes = [], []
        # EVERY assertion the production entry point runs.  Calling only a and b
        # here let the other two regress to reporting nothing while `--self-test`
        # printed that all controls behaved -- a checker with no firing proof,
        # which is the exact failure this file exists to prevent.
        M.assertion_a(memo, findings, notes)
        M.assertion_b(memo, findings, notes)
        M.assertion_cd_seed(memo, findings, notes)
        M.acceptance_vocab_seed(memo, findings, notes)
        return umb, [m for m in mentions if not m.licensed], findings


def attribution_control():
    """A pointer slot whose cell opens `Slice **9z** -- **UMBRELLA, ...**` is
    declaring 9z's kind, not its own.  §5: a pointer slot "carries no marker of
    its own".  The count must not move when such a row is added."""
    base = build()
    ptr = build(wb="**(carved at PR-B)** Slice **9z** — **UMBRELLA, not a terminal unit** — points into §5.")
    out = []
    for text in (base, ptr):
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d) / "fixture.md"
            p.write_text(text)
            out.append(len(M.Memo(str(p)).umbrella_ids()))
    return out


def degenerate_control():
    """A whole-line grep for the marker CANNOT disagree with the marker count.

    The declaring-field parse can.  This proves the two are different programs
    rather than one program written twice, which is what the memo's own control
    failed at when it searched the same literal it was counting.
    """
    text = build(d7z="**UMBRELLA, not a terminal unit** stray")
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "fixture.md"
        p.write_text(text)
        memo = M.Memo(str(p))
        by_field = len(memo.umbrella_ids())
        by_grep = sum(1 for l in memo.lines
                      if l.startswith("|") and MARKER in l)
    return by_field, by_grep


def run():
    fails = []
    counts = {"POSITIVE": 0, "POSITIVE-NOVEL": 0, "NEGATIVE": 0, "KNOWN-MISS": 0}
    print("=" * 74)
    print("plan-memo-umbrella-check  --  self-test")
    print("=" * 74)

    for kind, name, text, prose, expect, sibling in CASES:
        umb, reported, _ = run_on(text, prose, sibling)
        got = len(reported)
        # Exact, not `>=`: every fixture carries exactly one intended site, so a
        # scanner that reports one site twice must turn a control red rather
        # than inflate the production census behind a green self-test.
        ok = (got == expect)
        counts[kind] += 1
        if kind == "KNOWN-MISS":
            print("  RED  [KNOWN-MISS] %s -- reported %d (expected 0; this site IS wrong)"
                  % (name, got))
            if got:
                fails.append("KNOWN-MISS %s now reports; update the declared miss class" % name)
            continue
        if not ok:
            fails.append("%s %s: expected %d, got %d :: %s"
                         % (kind, name, expect, got,
                            [m.context()[:80] for m in reported]))
        print("  %-4s [%s] %s (%d reported)" % ("ok" if ok else "FAIL", kind, name, got))

    for kind, name, text, code, expect in ASSERT_CASES:
        counts[kind] += 1
        _, _, findings = run_on(text, "")
        got = sum(1 for c, _, _ in findings if c == code)
        ok = (got == expect)
        if not ok:
            fails.append("%s %s [%s]: expected %d, got %d"
                         % (kind, name, code, expect, got))
        print("  %-4s [%s] %s (%s x%d)" % ("ok" if ok else "FAIL", kind, name, code, got))

    n_base, n_ptr = attribution_control()
    ok = n_base == n_ptr
    if not ok:
        fails.append("attribution control: adding a pointer slot whose marker names ANOTHER "
                     "row moved the count %d -> %d" % (n_base, n_ptr))
    print("  %-4s [CONTROL] a marker naming another row does not enter the count (%d -> %d)"
          % ("ok" if ok else "FAIL", n_base, n_ptr))

    by_field, by_grep = degenerate_control()
    ok = by_field != by_grep
    if not ok:
        fails.append("degenerate control: declaring-field parse (%d) agreed with a "
                     "whole-line marker grep (%d) on a fixture built to separate them"
                     % (by_field, by_grep))
    print("  %-4s [CONTROL] declaring-field parse=%d vs whole-line marker grep=%d "
          "(must differ)" % ("ok" if ok else "FAIL", by_field, by_grep))

    print()
    # ⚠ Count BOTH registries.  This read `len(CASES)` and silently omitted every
    # assertion control, so the summary said 25 while 37 controls had run -- a
    # report that does not match what the program did, which is the class this
    # whole file exists to catch.
    print("%d case(s) -- %d naming + %d assertion: %d POSITIVE, %d POSITIVE-NOVEL, %d NEGATIVE, "
          "%d KNOWN-MISS (red, and they stay red)."
          % (len(CASES) + len(ASSERT_CASES), len(CASES), len(ASSERT_CASES),
             counts["POSITIVE"], counts["POSITIVE-NOVEL"],
             counts["NEGATIVE"], counts["KNOWN-MISS"]))
    if fails:
        print()
        for f in fails:
            print("FAIL: %s" % f)
        return 1
    print("all controls behaved as declared.")
    return 0
