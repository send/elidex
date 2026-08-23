#!/usr/bin/env python3
"""Machine-check a plan memo's umbrella rows against the way its own prose names them.

Design, invariants, lexing order, spec coverage and the seed/mechanical
boundary: `docs/plans/2026-08-plan-memo-umbrella-checker.md` (§1 charter, §2
invariants I-A..I-F + lexing order, §3 coverage map, §8 defers).  What follows
is only what that plan does not say.

WHY A PROGRAM.  #506's §5 rule ("naming an umbrella as an owner, as a 'lands
second' party, or in a `Deps` cell names *nobody*") was found violated by hand
one site per converge round, and three separate sweeps each scoped themselves
to whatever the previous round had named -- §5's `Deps` column once, the
acceptance class once, that round's own corrections once -- so each swept the
population that motivated it rather than the population the rule reaches.
This is the enumerator those sweeps did not have.  It does NOT discharge the
two slots #506's memo carves (both are umbrellas; their own derivations do):
`#11-plan-memo-spec-field-single-home-check` says in its own cell that of its
four assertions two are "mechanical programs over this document's table parse"
and two are natural-language claim extraction "for which no canonical
algorithm exists" -- this file is the first pair plus a declared-recall seed
for the second, and prints that boundary rather than leaving it to the reader.
NOT covered: the restatement sweep ("you are changing one decision; list EVERY
site that restates it" over STATEMENT / OBLIGATION / CONSEQUENCE surfaces) --
that class has its own tool on another branch (`plan-sweep.py`); until it
lands, a decision change over a memo is swept by hand.

MODULES
  plan_memo_lexer.py      CommonMark 0.31.2 / GFM 0.29 subset: fences, rows,
                          code spans, links, reference definitions, `Lexed`
  plan_memo_tables.py     schemas, `Row`, `Memo`, the transitive `Population`,
                          the mask disposition
  plan_memo_roles.py      licensing rule, role ranking, assertions (a)-(d)
  (this file)             mention scanners, `check()`, the report
  plan_memo_umbrella_selftest.py / _selftest_cases.py / _selftest_mutants.py

WHERE THIS RUNS.  By hand on a memo: `SCHEMAS` matches one document family's
exact header rows, so against any other plan memo it prints `FATAL: no table
matched schema ...` and exits 2 -- which is why the MEMO run is not a
trip-wire.  Its self-test IS one: `plan-memo-umbrella-selftest-trip-wire.sh`
runs `--self-test --mutants` under `scripts/trip-wires.sh` on every PR.

FINDING CODES.  Mechanical (gate the exit status): UMBRELLA-MARK (a),
UMBRELLA-CELL (b, the `Deps` half only -- the acceptance half has no cell and
is not implementable here), KIND-SPELLING, SCHEMA.  Seeds (`?` suffix, never
gate): UMBRELLA-MARK?, ORDER-PROSE? (c), TWO-OWNERS? (d), ACCEPT-VOCAB?.
NAMING sites are mechanical over their population and a seed as to it; two id
shapes are DECLARED MISSES held as red controls.  Each code's miss class is
stated beside its check in `plan_memo_roles.py` and in the report's notes.

EXIT STATUS
  0  no mechanical finding
  1  at least one mechanical finding (the assertions, not the seeds)
  2  a schema miss -- an absent linked memo, an unmatched schema, a body row
     whose width differs from its header, the same id declared twice.  The run
     is a SKIP, not a clean result, and it is never exit 0.
Seeds and reported naming sites do NOT affect it.  They cannot: the naming scan
reports by default, so a green state would not exist and the code would be a
gate nobody could ever satisfy.

Usage:  plan-memo-umbrella-check.py <memo> [--worklist]   (linked memos = the population)
        plan-memo-umbrella-check.py --self-test [--mutants]
"""

import sys
import pathlib
from collections import Counter, defaultdict, namedtuple

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from plan_memo_lexer import in_spans  # noqa: E402
from plan_memo_tables import CELL_SPLIT, Population  # noqa: E402
from plan_memo_roles import (  # noqa: E402
    CELL_TOKEN, MENTION_PROSE, MENTION_SLOT, acceptance_vocab_seed,
    assertion_a, assertion_b, assertion_cd_seed, classify, roles,
)


# --------------------------------------------------------------------------
# Mentions
# --------------------------------------------------------------------------


class Mention:
    """One naming site, in the coordinates of the block it was read from.

    `start`/`end` bound the whole match (a row noun plus the id, where there is
    one) because that is what the licensing rule reads around.  `idpos` is the
    position of the ID TOKEN, and it is the identity: the anchored pass and the
    bare pass see the same site through different spans, and deduping on the
    match span would count it twice.  `lineno` / `line` / `col` are the
    reporting coordinates (the raw line holding the id token, and the id
    token's raw column in it).
    """

    __slots__ = ("block", "id", "start", "end", "idpos", "anchored", "licensed",
                 "lineno", "line", "col")

    def __init__(self, block, id_, start, end, idpos=None, anchored=False):
        self.block, self.id, self.start, self.end = block, id_, start, end
        self.idpos = start if idpos is None else idpos
        self.anchored, self.licensed = anchored, False
        self.lineno, self.line, self.col = block.locate(self.idpos)

    @property
    def file(self):
        return self.block.file

    @property
    def source(self):
        return self.block.source

    @property
    def text(self):
        return self.block.text

    @property
    def key(self):
        return (self.file, self.lineno, self.col)

    def context(self, w=95):
        return self.block.context(self.start, self.end, w)


class Block:
    """One scanned unit of text -- a cell or a paragraph -- with its tagged
    mask (spans in `text` coordinates the scanners must not read an id out of,
    each with its kind) and the map back to reporting coordinates."""

    __slots__ = ("file", "text", "mask", "source", "self_id")

    def __init__(self, file, lexed, source, self_id=None):
        self.file, self.text, self.mask = file, lexed.text, lexed.mask
        self.source, self.self_id = source, self_id

    def masked(self, i, through_code=False):
        """Whether `i` is under the mask.  The slot pass reads THROUGH code
        spans (a backticked slug is the document spelling an id -- the same
        disposition exception as an id-only run -- and a backticked command
        line naming a slot must still be seen); every other kind masks it."""
        return in_spans(i, (m for m in self.mask if not (through_code and m[2] == "code")))


class CellBlock(Block):
    __slots__ = ("lineno", "line", "cell")

    def __init__(self, file, lineno, line, cell, source, self_id):
        super().__init__(file, cell.lexed, source, self_id)
        self.lineno, self.line, self.cell = lineno, line, cell

    def locate(self, i):
        return self.lineno, self.line, self.cell.raw(i)

    def context(self, start, end, w):
        return self.line[max(0, self.cell.raw(start) - w): self.cell.raw(end) + w].strip()


class ProseBlock(Block):
    __slots__ = ("para",)

    def __init__(self, file, para):
        super().__init__(file, para.lexed, "prose")
        self.para = para

    def locate(self, i):
        return self.para.locate(i)

    def context(self, start, end, w):
        return self.text[max(0, start - w): end + w].replace("\n", " ").strip()


def _anchored(b, keep, out):
    """Row-noun-anchored ids, plus `#11-` slot ids."""
    for mt in MENTION_PROSE.finditer(b.text):
        if mt.group(1) not in keep or mt.group(1) == b.self_id:
            continue
        if b.masked(mt.start(1)):
            continue
        out.append(classify(Mention(b, mt.group(1), mt.start(), mt.end(), mt.start(1), anchored=True)))
    for mt in MENTION_SLOT.finditer(b.text):
        if mt.group(1) not in keep or mt.group(1) == b.self_id:
            continue
        if b.masked(mt.start(1), through_code=True):
            continue
        out.append(classify(Mention(b, mt.group(1), mt.start(), mt.end())))


def _bare(b, keep, out):
    """Bare (row-noun-free) ids.

    Recognised only where the token cannot be confused with the other things
    these documents write: pure digits are §-numbers, step numbers, file counts
    and line numbers; an undecorated single letter is an English article or a
    family name.  Both are DECLARED MISSES, carried as red controls in the
    self-test rather than argued away.
    """
    text = b.text
    for tok in CELL_TOKEN.finditer(text):
        tid = tok.group("id")
        if tid not in keep or tid == b.self_id:
            continue
        if b.masked(tok.start("id")):
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
        lhs, rhs = text[:s], text[e:]
        if lhs and not CELL_SPLIT.search(lhs[-1]) and lhs[-1] not in "*`":
            continue
        if rhs and not CELL_SPLIT.search(rhs[0]) and rhs[0] not in "*`'\u2019.:-\u2014":
            continue
        out.append(classify(Mention(b, tid, s, e, tok.start("id"))))


def blocks(memo):
    """Every scanned unit of `memo`: each cell of each table row (header rows
    too; the delimiter row has no cells to scan; a schema data row's own id
    cell is excluded) and each paragraph."""
    name = memo.path.name
    out = []
    for t in memo.tables:
        for row in [t.header] + t.rows:
            line = memo.lines[row.lineno - 1]
            idc = row.schema.idc if row.schema is not None else None
            for col, cell in enumerate(row.cells):
                if col == idc:
                    continue
                src = "%s:col%d" % (t.schema.name if t.schema is not None else "table", col)
                out.append(CellBlock(name, row.lineno, line, cell, src, row.self_id))
    for para in memo.paragraphs:
        out.append(ProseBlock(name, para))
    return out


def collect_mentions(pop):
    """The whole naming pipeline, in ONE place, over the whole population and
    over EVERY declared id (the naming report keeps the no-owner ones; the
    ordering seed reads the rest)."""
    mentions = []
    keep = pop.keep()
    for memo in pop.memos:
        for b in blocks(memo):
            _anchored(b, keep, mentions)
            _bare(b, keep, mentions)
    # The anchored pass and the bare pass see the same site through different
    # spans.  Identity is the id token's position, and the anchored reading wins
    # because its span is what the licensing rule was written against.
    seen = {}
    for m in mentions:
        prev = seen.get(m.key)
        if prev is None or m.start < prev.start:
            seen[m.key] = m
    return list(seen.values())


# --------------------------------------------------------------------------
# The pipeline
# --------------------------------------------------------------------------

Result = namedtuple("Result", "findings notes rc mentions population mechanical")


def _result(findings, notes, rc, mentions, pop):
    # Mechanical findings only.  A code ending in `?` is a SEED -- a class this
    # program cannot decide -- and seeds do not gate, nor do naming sites, which
    # are non-zero by construction because the scan reports by default.
    mechanical = [f for f in findings if not f[0].endswith("?")]
    if rc is None:
        rc = 1 if mechanical else 0
    return Result(findings, notes, rc, mentions, pop, mechanical)


def check(path):
    """The ONLY pipeline: `main()` and `--self-test` both run this.

    Returns `Result(findings, notes, rc, mentions, population, mechanical)`;
    findings are `(code, file, lineno, message)`.  rc 2 = the population could
    not be scanned (every schema miss is listed as a `SCHEMA` finding); rc 1 =
    a mechanical finding; rc 0 = none.  Seeds (`?` codes) and naming sites
    never gate.
    """
    pop = Population(path)
    findings, notes = [], []
    for file, lineno, msg in pop.misses:
        findings.append(("SCHEMA", file, lineno, msg + ". This is a skip, not a clean run."))
    if pop.misses:
        return _result(findings, notes, 2, [], pop)
    umb = pop.no_owner_ids()
    if not umb:
        findings.append(("SCHEMA", pop.main.path.name, 0,
                         "no umbrella rows found -- the table schema did not match. "
                         "This is a skip, not a clean run."))
        return _result(findings, notes, 2, [], pop)
    undet = pop.undetermined_ids()
    if undet:
        notes.append(
            "[KIND-UNDETERMINED] %d row(s) declare an unsettled kind (%s) and are IN the naming "
            "population, because §5 gives them the same no-owner/no-ordering obligation as an "
            "umbrella." % (len(undet), ", ".join(sorted(undet))))
        if len(pop.spellings) > 1:
            findings.append(
                ("KIND-SPELLING", pop.main.path.name, 0,
                 "the undetermined kind is written %d ways (%s); a kind with more than one spelling "
                 "is a kind no program can enumerate"
                 % (len(pop.spellings), " / ".join(sorted(pop.spellings)))))
    all_mentions = collect_mentions(pop)
    assertion_a(pop, findings, notes)
    assertion_b(pop, findings, notes)
    assertion_cd_seed(pop, all_mentions, findings, notes)
    acceptance_vocab_seed(pop, findings, notes)
    return _result(findings, notes, None, [m for m in all_mentions if m.id in umb], pop)


# --------------------------------------------------------------------------
# Report
# --------------------------------------------------------------------------


def main(argv):
    if "--self-test" in argv:
        import plan_memo_umbrella_selftest as st  # noqa
        return st.run(mutants="--mutants" in argv)
    paths = [a for a in argv[1:] if not a.startswith("--")]
    if len(paths) != 1:
        print(__doc__)
        return 2
    res = check(paths[0])
    if res.rc == 2:
        for code, file, lineno, msg in res.findings:
            print("FATAL [%s] %s:%d  %s" % (code, file, lineno, msg))
        return 2
    pop, mentions = res.population, res.mentions
    unlicensed = [m for m in mentions if not m.licensed]

    print("=" * 78)
    print("plan-memo-umbrella-check  --  %s" % pop.main.path)
    print("  population (transitive over the memo's links): %s"
          % ", ".join(m.path.name for m in pop.memos[1:]) if len(pop.memos) > 1
          else "  population: the memo alone (it links no other memo)")
    print("=" * 78)
    for n in res.notes:
        print(n)
    print()
    print("[NAMING] %d mentions of a no-owner id; %d licensed, %d REPORTED."
          % (len(mentions), len(mentions) - len(unlicensed), len(unlicensed)))
    print("         SEED, not an inventory.  Two classes are DECLARED MISSES and are")
    print("         carried as red controls in --self-test rather than argued away:")
    print("           * a purely numeric id written without a row noun --")
    print("             indistinguishable from a §-number or a step;")
    print("           * an undecorated single letter written without a row noun --")
    print("             indistinguishable from an article or a family.")
    print("         Everything else is reported, licensed or not, so a spelling nobody")
    print("         has written yet is reported by default rather than admitted.")
    print()
    bysrc = Counter(m.source for m in unlicensed)
    for k, v in sorted(bysrc.items()):
        print("         %-22s %d" % (k, v))
    print()
    role = {m.key: ",".join(roles(m)) or "-" for m in unlicensed}
    byrole = Counter(r.replace(",", "+") if r != "-" else "(no role vocabulary)"
                     for r in role.values())
    print("         rank (a RANKING over the reported set, never a filter on it):")
    for k, v in byrole.most_common(8):
        print("         %-42s %d" % (k[:42], v))
    print()
    for code, file, lineno, msg in res.findings:
        print("[%s] %s:%d  %s" % (code, file, lineno, msg))
    print()
    byid = defaultdict(list)
    for m in unlicensed:
        byid[m.id].append(m)
    if "--worklist" in argv:
        for m in sorted(unlicensed, key=lambda m: m.key):
            print("%s\t%d\t%s\t%s\t%s\t%s"
                  % (m.file, m.lineno, m.id, m.source, role[m.key],
                     m.context().replace("\t", " ")))
    else:
        for rid in sorted(byid, key=lambda r: (-len(byid[r]), r)):
            print("--- %s  (%d reported)" % (rid, len(byid[rid])))
            for m in byid[rid]:
                print("    %s:%d [%s] {%s}  %s"
                      % (m.file, m.lineno, m.source, role[m.key], m.context()[:190]))
    print()
    print("%d mechanical finding(s) gate the exit status; %d seed(s) and %d reported "
          "naming site(s) do not." % (len(res.mechanical), len(res.findings) - len(res.mechanical),
                                      len(unlicensed)))
    return res.rc


if __name__ == "__main__":
    sys.exit(main(sys.argv))
