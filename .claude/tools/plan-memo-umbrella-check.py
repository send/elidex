#!/usr/bin/env python3
"""Machine-check a plan memo's umbrella rows against the way its own prose names them.

Why this exists rather than another prose rule, and rather than a hand sweep.
`docs/plans/2026-07-vm-p4-es-language-completeness.md` §5 states that an umbrella
row carries neither the ordering nor the owner and no acceptance condition, so
"naming an umbrella as an owner, as a 'lands second' party, or in a `Deps` cell
names *nobody*". Four converge rounds of PR #506 then found that rule violated by
hand, one site per round, and three separate sweeps each scoped themselves to
whatever the previous round had named -- §5's `Deps` column once, the acceptance
class once, that round's own corrections once -- so each swept the population
that motivated it rather than the population the rule reaches. This program is
the enumerator those sweeps did not have.

It does NOT discharge either of the two slots this document carves. Both are
declared `UMBRELLA, not a terminal unit`, so what discharges them is their own
derivation minting terminal children. `#11-plan-memo-spec-field-single-home-check`
says so in its own cell: of its four assertions, two are "mechanical programs over
this document's table parse" and two are natural-language claim extraction "for
which no canonical algorithm exists". This file is the first pair, plus a
declared-recall seed for the second pair. That boundary is printed in the report
rather than left to the reader, because a checker that prints `0` for a class it
cannot see is the failure the same document records under I-8.

WHY A SEPARATE FILE, AND WHAT N ACTUALLY IS
CLAUDE.md *One issue, one way* asks for N=1 and asks that anyone keeping N>1 be
able to write down why N existed.  This block used to answer that against the two
paths it already knew about -- itself and `claim-gate-plan-check.py` -- and
reported N=2.  That is the same "measure the population you know" failure this
program exists to catch, committed in the paragraph justifying the program.
Re-derived over every in-flight worktree instead:

    cd "$(git rev-parse --show-toplevel)/.." && for wt in elidex-wt-* elidex; do
      ls "$wt"/.claude/tools/*.py 2>/dev/null; done

Four distinct plan-memo programs are in flight, on three branch families, and
NONE is on `main` (`git ls-tree origin/main -- .claude/tools/` lists only
`webref` and the trip-wire shells):

  claim-gate-plan-check.py (+3 modules)  branches claim-gate-plan-check,
                                         stale-claim-detector
  plan-sweep.py                          branch layout-decorated-inline
  plan-xcheck.py                         branch layout-decorated-inline
  plan-memo-umbrella-check.py (this)     branch vm-p4-plan-doc

⚠ This file does NOT cover the restatement sweep -- "You are changing one
decision.  This lists EVERY site in the memo that restates it" across
STATEMENT / OBLIGATION / CONSEQUENCE surfaces, the un-propagated-decision
failure.  That class has a tool (`plan-sweep.py`), but it is NOT in this tree,
so nothing here names it as canonical or mandates running it; until it lands,
a decision change over this memo is swept by hand.  This file's subject is
different: which rows carry no owner, and which prose names one of them in a
role §5 says it cannot hold.

So N>1 is real per class, and the residue worth collapsing is the shared GFM
row splitter that honours an escaped pipe -- some thirty lines, duplicated four ways.  The
collapse is a landing-order question: joining them today couples each branch's
landing to the others' unconverged reviews.  Trigger to collapse: two or more of
them on `main`, at which point the splitter moves to one module and each program
keeps its own schema.

WHERE THIS RUNS
It is NOT wired into `scripts/trip-wires.sh`, and deliberately: that script globs
`.claude/tools/*-trip-wire.sh`, the CI job runs it ungated on every PR, and
`SCHEMAS` matches one document's exact header rows -- run against any other plan
memo this program prints `FATAL: no table matched schema(s) ...` and exits 2, so
wiring it there would red every unrelated PR.  Its home is CLAUDE.md's
*Development Rules*, invoked by hand when the VM-P4 umbrella memo is edited.

EXIT STATUS
  0  no mechanical finding
  1  at least one mechanical finding (the assertions, not the seeds)
  2  a schema did not match -- the run is a SKIP, not a clean result
Seeds and reported naming sites do NOT affect it.  They cannot: the naming scan
reports by default, so a green state would not exist and the code would be a
gate nobody could ever satisfy.

WHAT IS MECHANICAL AND WHAT IS A SEED (read this before believing a count)
  (a) UMBRELLA-MARK   mechanical, complete, for the half that counts: the
                      marker population read from the DECLARING FIELD (§5 rows:
                      the `Slice` cell; §8 slot rows: `Why deferred`), never
                      from a grep over the marker's own vocabulary.
      UMBRELLA-MARK?  SEED for the other half -- a row declaring the kind in
                      WORDS and carrying no marker.  Measured false positives:
                      a cell quoting the criterion to conclude it is terminal,
                      and a cell discussing another row's kind.
  (b) UMBRELLA-CELL   mechanical and complete FOR THE `Deps` HALF ONLY, and
                      only over §5's rows, which are the only rows with a `Deps`
                      column.  ⚠ The assertion as §8 words it is "no umbrella
                      row carries an acceptance condition OR a `Deps` edge", and
                      **the acceptance half is not implemented and is not
                      implementable here**: §5 gives acceptance no cell of its
                      own -- it is prose inside the `Slice` cell -- so deciding
                      whether a sentence states one is the same natural-language
                      problem as (c) and (d).  A reader who takes this check for
                      the whole of (b) reads `0` for a class it never looked at.
                      That is the exact shape of the defect the naming rule
                      exists for, so it is printed with the count.
  (c) ORDER-PROSE     SEED.  Prose asserting an ordering is natural language.
  (d) TWO-OWNERS      SEED.  Two sentences of one row naming two owners is
                      natural language.
  NAMING              mechanical over its population, SEED as to that population.
                      Every mention of an umbrella id that is not inside one of
                      the constructions §5 licenses.  Two passes, over every
                      table cell except the row's own id cell AND over every
                      line outside the tables: a row-noun-anchored pass, and a
                      bare pass that needs no row noun.  Two id shapes are
                      DECLARED MISSES, held as red controls in the self-test:
                      a purely numeric id and an undecorated single letter,
                      each written without a row noun.  Everything else is
                      reported whether or not anyone has written that spelling
                      before, which is what the POSITIVE-NOVEL controls test.
  ACCEPT-VOCAB        SEED, and declared as one by the memo itself: the slot
                      `#11-plan-memo-acceptance-falsifiability-check` measures
                      this approximation's own miss class and prints it.

The licensing rule is NOT a list of forbidden phrasings.  It is the complement:
a mention is licensed iff the umbrella is named as the possessor of a DERIVATION
or of its CHILDREN, or as the thing a child is "of".  Anything else is reported.
A phrasing nobody has written yet is therefore reported by default rather than
admitted by default, which is the safe polarity for a rule whose failures have
all been new spellings of an old mistake.

Usage:  plan-memo-umbrella-check.py <memo>      (carved siblings = the memo's own links)
        plan-memo-umbrella-check.py --self-test
"""

import re
import sys
import pathlib
from collections import Counter, defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from plan_memo_tables import ROW_NOUN, SCHEMAS, Memo, bare_id, code_spans  # noqa: E402
from plan_memo_roles import (  # noqa: E402
    CELL_SPLIT, CELL_TOKEN, LICENSE_AFTER, LICENSE_BEFORE, MENTION_PROSE, MENTION_SLOT,
    acceptance_vocab_seed, assertion_a, assertion_b, assertion_cd_seed, roles,
)


# --------------------------------------------------------------------------
# Mentions
# --------------------------------------------------------------------------


class Mention:
    """One naming site.

    `start`/`end` bound the whole match (a row noun plus the id, where there is
    one) because that is what the licensing rule reads around.  `idpos` is the
    position of the ID TOKEN, and it is the identity: the anchored pass and the
    bare pass see the same site through different spans, and deduping on the
    match span would count it twice.
    """

    __slots__ = ("file", "lineno", "id", "start", "end", "idpos", "line",
                 "source", "licensed", "why")

    def __init__(self, file, lineno, id_, start, end, line, source, idpos=None):
        self.file, self.lineno, self.id = file, lineno, id_
        self.start, self.end, self.line, self.source = start, end, line, source
        self.idpos = start if idpos is None else idpos
        self.licensed, self.why = False, ""

    @property
    def key(self):
        return (self.file, self.lineno, self.idpos)

    def context(self, w=95):
        return self.line[max(0, self.start - w) : self.end + w].strip()


# A row noun standing between the licensing phrase and the id ("the child of
# umbrella **3**") must not hide the phrase from the backward look.
_TRAILING_NOUN = re.compile(r"\b" + ROW_NOUN + r"[\s-]+(?:\*\*|`)*$")


def classify(m):
    before = m.line[: m.start]
    before = _TRAILING_NOUN.sub("", before)
    after = m.line[m.end :]
    if LICENSE_BEFORE.search(before[-40:]):
        m.licensed, m.why = True, "child-of / derivation-runner"
        return m
    if LICENSE_AFTER.match(after):
        m.licensed, m.why = True, "possessor of a thing §5 says an umbrella carries"
        return m
    m.licensed, m.why = False, ""
    return m


def _anchored(path, lineno, line, umb, off, cell, source, self_id, out, skip_spans=(), ids=None):
    """Row-noun-anchored ids, plus `#11-` slot ids.  Works on a cell or a whole line."""
    # The mask applies to the ROW-NOUN pass only.  A slot id is always written
    # inside backticks, so masking code runs would hide every one of them --
    # which it did, and the self-test's trigger-cell control is what said so.
    prose_skip = tuple(skip_spans) + tuple(code_spans(cell, keep=ids or umb))
    for mt in MENTION_PROSE.finditer(cell):
        if mt.group(1) not in umb or mt.group(1) == self_id:
            continue
        if any(s <= mt.start(1) < e for s, e in prose_skip):
            continue
        out.append(classify(Mention(path, lineno, mt.group(1), off + mt.start(),
                                    off + mt.end(), line, source, off + mt.start(1))))
    for mt in MENTION_SLOT.finditer(cell):
        if mt.group(1) not in umb or mt.group(1) == self_id:
            continue
        if any(s <= mt.start(1) < e for s, e in skip_spans):
            continue
        out.append(classify(Mention(path, lineno, mt.group(1), off + mt.start(), off + mt.end(), line, source)))


def _bare(path, lineno, line, umb, off, cell, source, self_id, out, ids=None):
    """Bare (row-noun-free) ids.

    Recognised only where the token cannot be confused with the other things
    these documents write: pure digits are §-numbers, step numbers, file counts
    and line numbers; an undecorated single letter is an English article or a
    family name.  Both are DECLARED MISSES, carried as red controls in the
    self-test rather than argued away.
    """
    code = code_spans(cell, keep=ids or umb)
    for tok in CELL_TOKEN.finditer(cell):
        tid = tok.group("id")
        if tid not in umb or tid == self_id:
            continue
        if any(s <= tok.start("id") < e for s, e in code):
            continue
        if tid.isdigit():
            continue
        # Unbalanced decoration means a bold RUN opened or closed nearby, not
        # that this token is decorated: `**A call at the finalizer …**` and
        # `**block-scope entry 10b**` are the two directions.  Treating it as
        # undecorated handles both -- the single-letter guard still drops `A`,
        # and a multi-character id inside a bold phrase is no longer invisible.
        balanced = tok.group("l") is not None and tok.group("l") == tok.group("r")
        if len(tid) == 1 and tid.isalpha() and not balanced:
            continue
        s, e = tok.start(), tok.end()
        lhs, rhs = cell[:s], cell[e:]
        if lhs and not CELL_SPLIT.search(lhs[-1]) and lhs[-1] not in "*`":
            continue
        if rhs and not CELL_SPLIT.search(rhs[0]) and rhs[0] not in "*`'\u2019.:-\u2014":
            continue
        out.append(classify(Mention(path, lineno, tid, off + s, off + e, line,
                                    source, off + tok.start("id"))))


def scan_tables(path, memo, umb, ids=None):
    """Every cell of every parsed table except the row's own id cell.

    Bare ids are read only in the mention-bearing columns, where the column's
    grammar makes a bare token an id.  Everywhere else in the row an id must be
    anchored by a row noun, exactly as in prose.  A row naming ITSELF is not a
    naming site, so the row's own id is excluded from its own row.
    """
    out = []
    ids = ids or set(memo.all_row_ids()) | set(umb)
    table_lines = set()
    for name, hdr, decl, idc, mention_cols in SCHEMAS:
        for lineno, cells in memo.data_rows(name):
            line = memo.lines[lineno - 1]
            table_lines.add(lineno)
            self_id = bare_id(cells[idc]) if idc is not None and len(cells) > idc else None
            off = 0
            for col, cell in enumerate(cells):
                if col == idc:
                    off += len(cell) + 1
                    continue
                src = "%s:col%d" % (name, col)
                _anchored(path, lineno, line, umb, off, cell, src, self_id, out, ids=ids)
                # Every cell but the row's own id cell is prose that can name a
                # row, so the bare pass runs over all of them, not only the
                # mention-bearing columns.  Scoping it to those columns was the
                # same "sweep the population that motivated the rule" error the
                # three earlier hand sweeps made.
                _bare(path, lineno, line, umb, off, cell, src, self_id, out, ids=ids)
                off += len(cell) + 1
    return out, table_lines


def scan_prose(path, lines, umb, table_lines, ids=None):
    """Both passes, over every line outside the parsed tables."""
    out = []
    for lineno, line in enumerate(lines, 1):
        if lineno in table_lines:
            continue
        _anchored(path, lineno, line, umb, 0, line, "prose", None, out, ids=ids)
        _bare(path, lineno, line, umb, 0, line, "prose", None, out, ids=ids)
    return out



# --------------------------------------------------------------------------
# Report
# --------------------------------------------------------------------------


def collect_mentions(memo, umb):
    """The whole naming pipeline, in ONE place.

    The self-test used to reimplement this -- with a first-wins dedup where this
    one keeps the smaller start -- so every control was green against a program
    that was not the one shipped.  The two happened to agree on today's inputs,
    which is exactly how that kind of divergence survives.  `--self-test` calls
    this function now, so a stage that exists in production cannot be missing
    from the harness (which is how two assertions went unexercised).
    """
    mentions = []
    all_ids = set(memo.all_row_ids()) | set(umb)
    cellm, table_lines = scan_tables(memo.path.name, memo, umb, ids=all_ids)
    mentions += cellm
    mentions += scan_prose(memo.path.name, memo.lines, umb, table_lines, ids=all_ids)
    # Siblings come from the memo's own links, never from the caller.
    for sp in memo.linked_memos():
        sl = sp.read_text().split("\n")
        sm = Memo(str(sp))
        # The sibling's own table cells count too.  This used to discard them and
        # keep only `stl`, so every mention inside a carved file's tables was
        # invisible -- a whole population silently at zero.
        sibm, stl = scan_tables(sp.name, sm, umb, ids=all_ids)
        mentions += sibm
        mentions += scan_prose(sp.name, sl, umb, stl, ids=all_ids)

    # The anchored pass and the bare pass see the same site through different
    # spans.  Identity is the id token's position, and the anchored reading wins
    # because its span is what the licensing rule was written against.
    seen, deduped = {}, []
    for m in mentions:
        prev = seen.get(m.key)
        if prev is None:
            seen[m.key] = m
            deduped.append(m)
        elif m.start < prev.start:
            deduped[deduped.index(prev)] = m
            seen[m.key] = m
    return deduped


def main(argv):
    if "--self-test" in argv:
        import plan_memo_umbrella_selftest as st  # noqa
        return st.run()
    paths = [a for a in argv[1:] if not a.startswith("--")]
    if len(paths) != 1:
        print(__doc__)
        return 2
    memo = Memo(paths[0])
    # A linked sibling that is not on disk is an unscanned population, not a
    # warning: the same exit code for "scanned" and "could not scan" is the
    # defect this discovery replaced.
    absent = [str(p) for p in memo.linked_memos() if not p.is_file()]
    if absent:
        print("FATAL: linked memo(s) not found -- their population is unscanned: %s. "
              "This is a skip, not a clean run." % ", ".join(absent))
        return 2
    spellings = set()
    undet = memo.undetermined_ids(spellings)
    # The naming rule is stated over rows that carry no owner and no ordering.
    # Umbrella rows are one kind of those; kind-undetermined rows are another.
    umb = memo.no_owner_ids()
    if not umb:
        print("FATAL: no umbrella rows found -- the table schema did not match. "
              "This is a skip, not a clean run.")
        return 2

    # ⚠ A guard that only fires when EVERY table is missing lets one table drop
    # out silently.  Measured: renaming a single §8 header cell drops the census
    # from 48 to 33 with zero slot umbrellas, no FATAL, and a report line whose
    # `slot=` term is absent rather than zero -- which a reader must notice by
    # absence.  Each schema must match at least one table.
    matched = {name for name, _, _ in memo.tables}
    missing = [name for name, hdr, decl, idc, _ in SCHEMAS if name not in matched]
    if missing:
        print("FATAL: no table matched schema(s) %s -- their whole population is "
              "unscanned. This is a skip, not a clean run." % ", ".join(missing))
        return 2

    findings, notes = [], []
    if undet:
        notes.append(
            "[KIND-UNDETERMINED] %d row(s) declare an unsettled kind (%s) and are IN the naming "
            "population, because §5 gives them the same no-owner/no-ordering obligation as an "
            "umbrella." % (len(undet), ", ".join(sorted(undet))))
        if len(spellings) > 1:
            findings.append(
                ("KIND-SPELLING", 0,
                 "the undetermined kind is written %d ways (%s); a kind with more than one spelling "
                 "is a kind no program can enumerate"
                 % (len(spellings), " / ".join(sorted(spellings)))))
    assertion_a(memo, findings, notes)
    assertion_b(memo, findings, notes)
    assertion_cd_seed(memo, findings, notes)
    acceptance_vocab_seed(memo, findings, notes)

    # -- naming scan over the memo and its siblings ------------------------
    mentions = collect_mentions(memo, umb)
    unlicensed = [m for m in mentions if not m.licensed]

    print("=" * 78)
    print("plan-memo-umbrella-check  --  %s" % memo.path)
    print("  siblings (from the memo's links): %s"
          % (", ".join(p.name for p in memo.linked_memos()) or "none"))
    print("=" * 78)
    for n in notes:
        print(n)
    print()
    print("[NAMING] %d mentions of an umbrella id; %d licensed, %d REPORTED."
          % (len(mentions), len(mentions) - len(unlicensed), len(unlicensed)))
    print("         SEED, not an inventory.  Two classes are DECLARED MISSES and are")
    print("         carried as red controls in --self-test rather than argued away:")
    print("           * a purely numeric id (Slice 2/3/4/5/6/7/8/9/10) written without")
    print("             a row noun -- indistinguishable from a §-number or a step;")
    print("           * an undecorated single letter (A/B/C/E/L/M/P/R) written without")
    print("             a row noun -- indistinguishable from an article or a family.")
    print("         Everything else is reported, licensed or not, so a spelling nobody")
    print("         has written yet is reported by default rather than admitted.")
    print()
    bysrc = Counter(m.source for m in unlicensed)
    for k, v in sorted(bysrc.items()):
        print("         %-22s %d" % (k, v))
    print()
    byrole = Counter()
    for m in unlicensed:
        r = roles(m)
        byrole["+".join(r) if r else "(no role vocabulary)"] += 1
    print("         rank (a RANKING over the reported set, never a filter on it):")
    for k, v in byrole.most_common(8):
        print("         %-42s %d" % (k[:42], v))
    print()
    for code, lineno, msg in findings:
        print("[%s] %s:%d  %s" % (code, memo.path.name, lineno, msg))
    print()
    byid = defaultdict(list)
    for m in unlicensed:
        byid[m.id].append(m)
    if "--worklist" in argv:
        for m in sorted(unlicensed, key=lambda m: (m.file, m.lineno, m.start)):
            print("%s\t%d\t%s\t%s\t%s\t%s"
                  % (m.file, m.lineno, m.id, m.source, ",".join(roles(m)) or "-",
                     m.context().replace("\t", " ")))
    else:
        for rid in sorted(byid, key=lambda r: (-len(byid[r]), r)):
            print("--- %s  (%d reported)" % (rid, len(byid[rid])))
            for m in byid[rid]:
                print("    %s:%d [%s] {%s}  %s"
                      % (m.file, m.lineno, m.source, ",".join(roles(m)) or "-", m.context()[:190]))
    print()

    # Mechanical findings only.  A code ending in `?` is a SEED -- a class this
    # program cannot decide -- and seeds do not gate, nor does `unlicensed`,
    # which is non-zero by construction because the naming scan reports by
    # default.  Gating on either would mean no green state exists and the code
    # would be a gate nobody could satisfy.  See EXIT STATUS in the header.
    mechanical = [f for f in findings if not f[0].endswith("?")]
    print("%d mechanical finding(s) gate the exit status; %d seed(s) and %d reported "
          "naming site(s) do not." % (len(mechanical), len(findings) - len(mechanical),
                                      len(unlicensed)))
    return 1 if mechanical else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
