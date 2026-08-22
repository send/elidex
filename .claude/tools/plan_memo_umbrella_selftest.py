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

HERE = pathlib.Path(__file__).resolve().parent


def _load():
    spec = importlib.util.spec_from_file_location(
        "pmuc", HERE / "plan-memo-umbrella-check.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


M = _load()

# A minimal memo carrying both table schemas, so the fixtures exercise the real
# parse rather than a hand-built cell list.
HEADER = """# fixture

## §0.5 Spec citation table

| ID | Citation | Anchor | Used by |
|---|---|---|---|
| [C1] | ECMA-262 §1 X | `#a` | {c1} |

## §5. Slice plan

| # | Slice | Primary module(s) | Slot | Tier | Deps |
|---|---|---|---|---|---|
| **9z** | **UMBRELLA, not a terminal unit.** {s9z} | `a.rs` | — | T1 | {d9z} |
| **7z** | Terminal. {s7z} | `b.rs` | — | T1 | {d7z} |
| **9** | **UMBRELLA, not a terminal unit.** numeric id. | `c.rs` | — | T1 | — |
| **C** | **UMBRELLA, not a terminal unit.** single-letter id. | `d.rs` | — | T1 | — |
| **Qx** | {sqx} | `e.rs` | — | T1 | — |
| **Uz** | {suz} | `f.rs` | — | T1 | {duz} |

## §8. Slot ledger changes at landing

| Slot | Why deferred | Trigger | Re-eval |
|---|---|---|---|
| `#11-zz-alpha` | **UMBRELLA, not a terminal unit.** {wa} | {ta} | 2026-12-31 |
| `#11-zz-beta` | Terminal. {wb} | {tb} | 2026-12-31 |
"""

# The two terminal rows carry acceptance vocabulary in the BASE fixture, so a
# control that varies ONE row's cell measures that row.  Without it the
# accept-vocab negative control counted the other terminal row and could never
# reach 0 -- a control that cannot go green tests nothing.
BLANK = dict(c1="—", s9z="charter.", d9z="—",
             s7z="Terminal.  Acceptance: the probe must return 3.", d7z="—",
             sqx="Terminal.  Acceptance: the probe must return 4.",
             suz="Terminal.  Acceptance: the probe must return 5.", duz="—",
             wa="why.", ta="now", wb="why.", tb="now")


def build(**kw):
    f = dict(BLANK)
    f.update(kw)
    return HEADER.format(**f)


def run_on(text, prose=""):
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "fixture.md"
        p.write_text(text + "\n" + prose + "\n")
        memo = M.Memo(str(p))
        umb = memo.umbrella_ids()
        all_ids = set(memo.all_row_ids()) | set(umb)
        cellm, tl = M.scan_tables(p.name, memo, umb, ids=all_ids)
        mentions = cellm + M.scan_prose(p.name, memo.lines, umb, tl, ids=all_ids)
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


CASES = []


def case(kind, name, text, prose, expect):
    CASES.append((kind, name, text, prose, expect))


# ---------------------------------------------------------------- POSITIVE --
case("POSITIVE", "used-by cell names an umbrella outright",
     build(c1="Slice 9z (the close rule)"), "", 1)
case("POSITIVE", "Deps cell of a terminal row names an umbrella",
     build(d7z="**9z**"), "", 1)
case("POSITIVE", "trigger cell names another row's umbrella slot as a co-occasion",
     build(tb="now, or with `#11-zz-alpha`"), "", 1)
case("POSITIVE", "prose orders an umbrella",
     build(), "Slice 9z lands before Slice 7z.", 1)
case("POSITIVE", "prose names an umbrella bare, with no row noun",
     build(), "9z owns the close rule outright.", 1)

# ---------------------------------------------------------- POSITIVE-NOVEL --
# None of these spellings occurs in the memo this checker was written against.
# They must be reported because the rule is stated as a complement -- an id is
# licensed only when attached to a derivation, to children, to a charter, to a
# memo or to a split -- and not as a list of bad phrasings.
case("POSITIVE-NOVEL", "novel verb: an umbrella is 'chartered to deliver'",
     build(), "9z is chartered to deliver the close rule by Q3.", 1)
case("POSITIVE-NOVEL", "novel role noun: an umbrella as 'the integrator'",
     build(), "The integrator for this artifact is 9z.", 1)
case("POSITIVE-NOVEL", "novel ordering idiom: 'downstream of'",
     build(), "Everything here sits downstream of 9z.", 1)
case("POSITIVE-NOVEL", "novel acceptance idiom: 'signs off'",
     build(), "9z signs off the observable once the probe is green.", 1)
case("POSITIVE-NOVEL", "novel spelling in a cell: an em-dash hand-off",
     build(c1="9z — hand-off"), "", 1)

# ---------------------------------------------------------------- NEGATIVE --
case("NEGATIVE", "the child-of construction",
     build(c1="the child of umbrella 9z that owns the close rule, via that "
             "umbrella's derivation, which mints it"), "", 0)
case("NEGATIVE", "the derivation-mints construction",
     build(c1="the child umbrella **9z**'s derivation mints for the close rule"), "", 0)
case("NEGATIVE", "possessor of children",
     build(), "umbrella 9z's children carry the obligation.", 0)
case("NEGATIVE", "possessor of a charter",
     build(), "The surface sits inside umbrella 9z's charter.", 0)
case("NEGATIVE", "statement about the row's kind",
     build(), "Slice 9z is an umbrella, so it ships no PR.", 0)
case("NEGATIVE", "a self-declaring row that MENTIONS a sibling stays in the population",
     build(suz="Unlike Slice 7z, **UMBRELLA, not a terminal unit.** charter."),
     "Uz owns the close rule.", 1)
case("NEGATIVE", "a row naming itself in its own cell",
     build(s9z="charter; 9z mints its children here."), "", 0)
case("POSITIVE", "a multi-character id inside a bold PHRASE, not bold itself",
     build(c1="**block-scope entry 9z**"), "", 1)
case("POSITIVE", "a backticked run of ids and separators is a Deps-shaped edge, not code",
     build(), "The same thing happened to `9z / 7z`, one layer down.", 1)
case("NEGATIVE", "a bold sentence opener is not row A",
     build(), "⚠ **A call at the finalizer sites is not the fix.**", 0)
case("POSITIVE", "a backticked BARE id is the document spelling an id, not code",
     build(), "The obligation is `9z`'s, and naming `9z` there names nobody.", 1)
case("NEGATIVE", "an id-looking token inside inline code",
     build(), "The probe reads `Reflect.construct(9z, [], D)` and stops.", 0)
case("NEGATIVE", "an id-looking token inside a file name",
     build(), "See [detail](2026-07-vm-p4-slice-9z-detail.md) for the walk.", 0)

# -------------------------------------------------------------- KNOWN-MISS --
# These are wrong sites.  The checker does not report them, and that is the
# declared boundary of the seed, not a pass.
case("KNOWN-MISS", "purely numeric id with no row noun",
     build(), "Everything here lands after 9 and before 10.", 0)
case("KNOWN-MISS", "undecorated single letter with no row noun",
     build(), "The obligation is C's, and C integrates it.", 0)


# ---------------------------------------------------- assertion controls ----
ASSERT_CASES = []


def acase(kind, name, text, code, expect):
    ASSERT_CASES.append((kind, name, text, code, expect))


acase("POSITIVE", "(a-seed) a row declaring the kind in words carries no marker",
      build(sqx="This row is an umbrella: three intersecting axes."),
      "UMBRELLA-MARK?", 1)
acase("POSITIVE", "(a-seed) fires on a cell QUOTING the criterion too -- a "
                  "measured false positive, kept visible rather than filtered",
      build(sqx="The criterion asks for a subsystem with no canonical algorithm; "
                "this row touches none, so it is terminal."),
      "UMBRELLA-MARK?", 1)
acase("POSITIVE", "(a) the marker outside the declaring field certifies nothing",
      build(sqx="body.", d7z="**UMBRELLA, not a terminal unit** stray"),
      "UMBRELLA-MARK", 1)
acase("POSITIVE", "(a) a marker that names ANOTHER row is not a self-declaration",
      build(wb="**(carved at PR-B)** Slice **9z** — **UMBRELLA, not a terminal unit** — "
               "with sub-slices; this slot points into §5."),
      "UMBRELLA-MARK", 1)
acase("POSITIVE", "(b) a KIND-UNDETERMINED row carrying a Deps edge is checked too",
      build(suz="**KIND UNDETERMINED**: neither an umbrella nor a terminal unit.", duz="**7z**"),
      "UMBRELLA-CELL", 1)
acase("POSITIVE", "(b) an umbrella row carrying a Deps edge",
      build(d9z="**7z**"), "UMBRELLA-CELL", 1)
acase("NEGATIVE", "(accept-vocab seed) a POINTER row is excluded from the population",
      build(sqx="This row is a pointer rather than a slice; the work is scheduled from its slot."),
      "ACCEPT-VOCAB?", 0)
acase("POSITIVE", "(c-seed) ordering vocabulary in prose against an empty Deps cell",
      build(s7z="Terminal.  This row lands before 9z and is a prerequisite of it.",
            d7z="—"),
      "ORDER-PROSE?", 1)
acase("NEGATIVE", "(c-seed) ordering vocabulary WITH a Deps cell is not reported",
      build(s7z="Terminal.  This row lands before 9z and is a prerequisite of it.",
            d7z="**Qx**"),
      "ORDER-PROSE?", 0)
acase("POSITIVE", "(accept-vocab seed) a terminal row with neither `must` nor `acceptance`",
      build(sqx="Terminal.  Lowers the thing."), "ACCEPT-VOCAB?", 1)
acase("NEGATIVE", "(accept-vocab seed) a terminal row stating an acceptance condition",
      build(sqx="Terminal.  Acceptance: the probe must return 3."), "ACCEPT-VOCAB?", 0)
acase("NEGATIVE", "(b) an umbrella row with an empty Deps cell",
      build(), "UMBRELLA-CELL", 0)


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
                      if l.startswith("|") and M.MARKER in l)
    return by_field, by_grep


def run():
    fails = []
    counts = {"POSITIVE": 0, "POSITIVE-NOVEL": 0, "NEGATIVE": 0, "KNOWN-MISS": 0}
    print("=" * 74)
    print("plan-memo-umbrella-check  --  self-test")
    print("=" * 74)

    for kind, name, text, prose, expect in CASES:
        umb, reported, _ = run_on(text, prose)
        got = len(reported)
        ok = (got >= expect) if expect else (got == 0)
        counts[kind] += 1
        if kind == "KNOWN-MISS":
            print("  RED  [KNOWN-MISS] %s -- reported %d (expected 0; this site IS wrong)"
                  % (name, got))
            if got:
                fails.append("KNOWN-MISS %s now reports; update the declared miss class" % name)
            continue
        if not ok:
            fails.append("%s %s: expected %s, got %d :: %s"
                         % (kind, name, ">=1" if expect else "0", got,
                            [m.context()[:80] for m in reported]))
        print("  %-4s [%s] %s (%d reported)" % ("ok" if ok else "FAIL", kind, name, got))

    for kind, name, text, code, expect in ASSERT_CASES:
        counts[kind] += 1
        _, _, findings = run_on(text, "")
        got = sum(1 for c, _, _ in findings if c == code)
        ok = (got >= expect) if expect else (got == 0)
        if not ok:
            fails.append("%s %s [%s]: expected %s, got %d"
                         % (kind, name, code, ">=1" if expect else "0", got))
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
