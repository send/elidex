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
two slots #506's memo §8 carves (both are umbrellas; their own derivations
do; neither is in the slot SoT ledger yet -- registration is owed at #506's
landing, see the plan's §8): the single-home slot says in its own §8 cell
that of its four assertions two are "mechanical programs over this
document's table parse" and two are natural-language claim extraction "for
which no canonical algorithm exists" -- this file is the first pair plus a
declared-recall seed for the second, and prints that boundary rather than
leaving it to the reader.  NOT covered: the restatement sweep ("you are
changing one decision; list EVERY site that restates it" over STATEMENT /
OBLIGATION / CONSEQUENCE surfaces); a decision change over a memo is swept by
hand.

MODULES
  plan_memo_ids.py        the id-token grammar: the three kinds, decoration, and
                          the ONE boundary every reader consumes (`tokens`)
  plan_memo_lexer.py      Phase 2 (inline): code spans + links / images in one
                          pass, link grammar, `Lexed`
  plan_memo_blocks.py     Phase 1 (blocks): raw extents (indented code, fences,
                          HTML blocks), the block-quote marker, block starts, the
                          one `block_end` predicate, GFM rows, reference definitions
  plan_memo_tables.py     id grammar, schemas, `Row`, `admit_table`, the mask
                          disposition
  plan_memo_memo.py       `Memo` (the Phase-1 driver, file I/O, the sibling
                          resolver) and the transitive `Population`
  plan_memo_roles.py      licensing rule, role ranking, assertions (a)-(d)
  (this file)             mention scanners, `check()`, the report
  plan_memo_umbrella_selftest.py (the runner) / _selftest_controls.py (the
                          function-shaped controls + `registry()`) /
                          _selftest_harness.py (loader, fixture runner,
                          work witnesses) / _selftest_cases.py /
                          _selftest_cases_pr510.py /
                          _selftest_mutants.py / _selftest_mutants_pr510.py /
                          _selftest_conformance.py (the
                          CommonMark 0.31.2 spec examples, vendored in
                          commonmark-0.31.2-block-examples.json, through Phase 1)

WHERE THIS RUNS.  By hand on a memo: `SCHEMAS` matches one document family's
exact header rows, so against any other plan memo it prints `FATAL: no table
matched schema ...` and exits 2 -- which is why the MEMO run is not a
trip-wire.  Its self-test IS one: `plan-memo-umbrella-selftest-trip-wire.sh`
runs `--self-test --mutants` under `scripts/trip-wires.sh` on every PR.

FINDING CODES.  Mechanical (gate the exit status): UMBRELLA-MARK (a),
UMBRELLA-CELL (b, the `Deps` half only -- the acceptance half has no cell and
is not implementable here), KIND-SPELLING, SCHEMA.  Seeds (`?` suffix, never
gate): UMBRELLA-MARK?, ORDER-PROSE? (c), TWO-OWNERS? (d), ACCEPT-VOCAB?,
LEX-UNSUPPORTED? (a RAW line never inline-parsed -- an HTML-block line, an
indented-code line, a fence excepted, or an inline raw-HTML span (§6.6) --
holding a `|` or a declared id: one seed rule, the READING printed with it).
NAMING sites are mechanical over their population and a seed as to it; two id
shapes are DECLARED MISSES held as red controls.  Each code's miss class is
stated beside its check in `plan_memo_roles.py` and in the report's notes.
NOTES (printed, never gating): `[CENSUS]` the no-owner row count (0 is a
clean result, not a schema miss), `[KIND-UNDETERMINED]`.

EXIT STATUS
  0  no mechanical finding
  1  at least one mechanical finding (the assertions, not the seeds)
  2  a schema miss -- an absent linked memo, a reference no definition answers
     (the memo it meant to link is NOT in the population), an unmatched
     schema, a body row whose width differs from its header, the same id
     declared twice, a schema row whose id cell is not an id (unkeyed, so its
     cells would go unasserted).  The run is a SKIP, not a clean result, and
     it is never exit 0.
Seeds and reported naming sites do NOT affect it.  They cannot: the naming scan
reports by default, so a green state would not exist and the code would be a
gate nobody could ever satisfy.

Usage:  plan-memo-umbrella-check.py <memo> [--worklist]   (linked memos = the population)
        plan-memo-umbrella-check.py --self-test [--mutants]
"""

import sys
import pathlib
from collections import Counter, defaultdict, namedtuple

HERE = str(pathlib.Path(__file__).resolve().parent)
if HERE not in sys.path:      # the self-test execs this file once per mutant
    sys.path.insert(0, HERE)
from plan_memo_ids import ROW_KINDS, tokens  # noqa: E402
from plan_memo_tables import stream  # noqa: E402
from plan_memo_memo import Population  # noqa: E402
from plan_memo_roles import (  # noqa: E402
    NOUN_ANCHOR, acceptance_vocab_seed, assertion_a, assertion_b, assertion_cd_seed, classify,
    roles,
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
    def memo(self):
        return self.block.memo

    @property
    def source(self):
        return self.block.source

    @property
    def text(self):
        """The block's DISPOSED stream (what the licensing rule reads around
        the match), not its raw text."""
        return self.block.stream

    @property
    def key(self):
        return (self.block.memo.key, self.lineno, self.col)

    def window(self, w=110):
        """The disposed stream around the match (what the role ranking
        reads); `context()` is the raw text, for display."""
        return self.block.window(self.start, self.end, w)

    def context(self, w=95):
        return self.block.context(self.start, self.end, w)


class Block:
    """One scanned unit of text -- a cell or a paragraph -- with its tagged
    mask (spans in `text` coordinates the scanners must not read an id out of,
    each with its kind), its disposed `stream` (the ONE text every predicate
    reads -- the scanners included: `stream()` blanks every masked span in
    place, offsets preserved, so a match can never straddle a mask boundary,
    where a raw-text scan read `` `Slice `C `` as `Slice C` with the row
    noun inside the code span; what a scanner may read out of a code span --
    an id-only run, a kept `#11-` slug -- was excepted by the disposition
    step and stands in the stream), the id tokens of that stream (`tokens`:
    read ONCE by the grammar, `plan_memo_ids.tokens`; both passes consume
    the same list), and the map back to reporting coordinates.  Minted only
    after the `Population` has disposed the block (`stream()` asserts it)."""

    __slots__ = ("memo", "file", "text", "mask", "stream", "tokens", "source", "self_id")

    def __init__(self, memo, file, lexed, source, self_id=None):
        # `file` = the population's ONE display name of the memo
        # (`Population.display`: relative to the root memo's directory), for
        # the report and the worklist; identity is `memo.key`
        self.memo, self.file, self.text, self.mask = memo, file, lexed.text, lexed.mask
        self.stream = stream(lexed)
        self.tokens = list(tokens(self.stream))
        self.source, self.self_id = source, self_id

    def window(self, start, end, w):
        return self.stream[max(0, start - w): end + w].replace("\n", " ")


class CellBlock(Block):
    __slots__ = ("lineno", "line", "cell")

    def __init__(self, memo, file, lineno, line, cell, source, self_id):
        super().__init__(memo, file, cell.lexed, source, self_id)
        self.lineno, self.line, self.cell = lineno, line, cell

    def locate(self, i):
        return self.lineno, self.line, self.cell.raw(i)

    def context(self, start, end, w):
        return self.line[max(0, self.cell.raw(start) - w): self.cell.raw(end) + w].strip()


class ProseBlock(Block):
    __slots__ = ("para",)

    def __init__(self, memo, file, para):
        super().__init__(memo, file, para.lexed, "prose")
        self.para = para

    def locate(self, i):
        return self.para.locate(i)

    def context(self, start, end, w):
        return self.text[max(0, start - w): end + w].replace("\n", " ").strip()


def _anchored(b, keep, out):
    """Row-noun-anchored row ids (the anchored reading the licensing rule
    was written against): a `NOUN_ANCHOR` match followed, exactly at its
    end, by one of the block's grammar tokens of a ROW kind (`ROW_KINDS`:
    slug or short; a citation is no row) -- the same token the bare pass
    reads, so the two readings can never disagree on where an id starts or
    ends.  A row noun anchors a digit or a single letter too (`Slice 9`,
    `Slice C`): the bare pass's declared misses do not apply.  A slug after
    a row noun (`Slice `#11-zz-alpha``) is the anchored reading of that
    site too (until PR #510 R20 only the short kind was: the bare pass
    still reported the slug, but the site was never `anchored`, so the (c)
    seed's prose-vs-`Deps` comparison never saw a slug named in a Slice
    cell)."""
    at = {t.start: t for t in b.tokens}
    for nm in NOUN_ANCHOR.finditer(b.stream):
        t = at.get(nm.end())
        if t is None or t.kind not in ROW_KINDS or t.id not in keep or t.id == b.self_id:
            continue
        out.append(classify(Mention(b, t.id, nm.start(), t.end, t.idstart, anchored=True)))


def _bare(b, keep, out):
    """Bare (row-noun-free) ids: every short-id and `#11-` slug token of the
    block's stream (`plan_memo_ids.tokens` -- the boundary is the grammar's:
    a hyphen bounds a short id, a slug is bounded on both sides, a dotted
    number is one token, a decorated side is bounded by its decoration).  A
    slug is its own anchor and is reported however it is decorated.

    A short id is recognised only where the token cannot be confused with
    the other things these documents write: pure digits are §-numbers, step
    numbers, file counts and line numbers; an undecorated single letter is
    an English article or a family name.  Both are DECLARED MISSES, carried
    as red controls in the self-test rather than argued away.  Everything
    else the grammar bounds is reported, licensed or not.
    """
    for t in b.tokens:
        tid = t.id
        if t.kind == "cite" or tid not in keep or tid == b.self_id:
            continue
        if t.kind == "short":
            if tid.isdigit():       # `tid` is SHORT_ID, ASCII by grammar: this is `[0-9]+`
                continue
            # Unbalanced decoration means a bold RUN opened or closed nearby,
            # not that this token is decorated: `**A call at the finalizer …**`
            # and `**block-scope entry 10b**` are the two directions.  Treating
            # it as undecorated handles both -- the single-letter guard still
            # drops `A`, and a multi-character id inside a bold phrase is no
            # longer invisible.
            if len(tid) == 1 and tid.isalpha() and not t.balanced:
                continue
        out.append(classify(Mention(b, tid, t.start, t.end, t.idstart)))


def blocks(pop):
    """Every scanned unit of the population, memo by memo: each cell of each
    table row (header rows too; the delimiter row has no cells to scan; a
    schema data row's own id cell is scanned with its own id suppressed) and
    each paragraph.  Takes the
    `Population`, not a memo, because a block's stream exists only after the
    population's disposition step ran over every memo.  A cell's `source` is
    `<schema>:<header cell>` (`slice:Deps`) -- the name the seeds key on --
    or `table:col<n>` for a non-schema table."""
    out = []
    for memo in pop.memos:
        file = pop.display(memo.path)
        for t in memo.tables:
            for row in [t.header] + t.rows:
                # the id cell is scanned too: its trailing prose (`**7z** —
                # Slice 9z lands first`) can name a row; the row's OWN id
                # token is suppressed by `self_id` in both passes.  `row.line`
                # is the CONTENT line the cells were split from (inside a
                # block quote the raw line carries the marker), so a cell's
                # raw column is a column of it
                for col, cell in enumerate(row.cells):
                    src = ("%s:%s" % (t.schema.name, t.schema.header[col]) if t.schema is not None
                           else "table:col%d" % col)
                    out.append(CellBlock(memo, file, row.lineno, row.line, cell, src, row.self_id))
        for para in memo.paragraphs:
            out.append(ProseBlock(memo, file, para))
    return out


def collect_mentions(pop):
    """The whole naming pipeline, in ONE place, over the whole population and
    over EVERY declared id (the naming report keeps the no-owner ones; the
    ordering seed reads the rest)."""
    mentions = []
    keep = pop.keep()
    for b in blocks(pop):
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


_READING = {
    "html": "raw HTML-block line (CommonMark §4.6) never inline-parsed",
    "indented": "indented-code line (CommonMark §4.4: raw, like a fence -- cmark-gfm agrees, an indented "
                "row after a table is `<pre><code>`) never inline-parsed",
    "inline": "inline raw HTML span (CommonMark §6.6: a tag, comment, processing instruction, declaration "
              "or CDATA section inside a paragraph or a cell) never inline-parsed",
}


def lex_unsupported_seed(pop, findings, notes):
    """`[LEX-UNSUPPORTED?]` SEED: a RAW line this lexer never inline-parses
    (`Memo.raw`: every line of an HTML block, §4.6, or of an indented code
    block, §4.4 -- a fence excepted, the author's explicit code marker --
    and, since PR #510 R17, every INLINE raw HTML span, §6.6, keyed on its
    first line: the same disposition for the same kind of text, "raw,
    seeded") that holds a `|` or a declared id: the content a table or a
    naming scan would have read had the text been prose, printed with the
    READING that makes it raw (`_READING`) rather than assumed.  ONE seed
    rule for every raw line (design re-gate 3, IMP-2: an indented schema
    row after a table's rows -- `    | id | ... |`, or a tab -- is raw
    under cmark-gfm too, and left the census silently, the I-C class; it
    is seeded exactly as a raw HTML line holding a `|` always was).  A
    seed in the ORDER-PROSE? idiom: never gating, and no count here bounds
    the class (an HTML table row whose ids are undeclared is invisible to
    it).  Containers are not seeded -- a block quote's or a list item's
    content IS parsed (§5.1 / §5.2; until PR #510 R15 an item's indented
    second paragraph was raw here, seeded with an `item` reading, and the
    memo it linked was never walked).  The ids are read by the ONE grammar
    (`plan_memo_ids.tokens`, the kinds the naming scan reads: a citation
    id is masked everywhere else and is no seed here either), so a raw
    line's `9z-owner` seeds `9z` exactly as prose would report it."""
    keep, n = pop.keep(), 0
    for memo in pop.memos:
        for lineno, line, reading in memo.raw:
            ids = sorted({t.id for t in tokens(line) if t.kind != "cite" and t.id in keep})
            if "|" in line or ids:
                n += 1
                findings.append(("LEX-UNSUPPORTED?", pop.display(memo.path), lineno, "%s; it holds %s" % (
                    _READING[reading], ", ".join(["a `|`"] * ("|" in line) + [repr(i) for i in ids]))))
    notes.append("[LEX-UNSUPPORTED?] SEED -- %d raw line(s) never inline-parsed (an HTML-block line, an "
                 "indented-code line, or an inline raw-HTML span) hold a `|` or a declared id; the bound is "
                 "the plan's §3 table, not this figure" % n)


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
    # Schema matching is its own miss (above).  A memo whose every row is
    # terminal has an EMPTY naming population, which is a clean result.
    notes.append("[CENSUS] %d no-owner rows (umbrella + kind-undetermined)" % len(umb))
    undet = pop.ids_of_kind("undetermined")
    if undet:
        notes.append(
            "[KIND-UNDETERMINED] %d row(s) declare an unsettled kind (%s) and are IN the naming "
            "population, because §5 gives them the same no-owner/no-ordering obligation as an "
            "umbrella." % (len(undet), ", ".join(sorted(undet))))
        if len(pop.spellings) > 1:
            findings.append(
                ("KIND-SPELLING", pop.display(pop.main.path), 0,
                 "the undetermined kind is written %d ways (%s); a kind with more than one spelling "
                 "is a kind no program can enumerate"
                 % (len(pop.spellings), " / ".join(sorted(pop.spellings)))))
    lex_unsupported_seed(pop, findings, notes)
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
          % ", ".join(pop.display(m.path) for m in pop.memos[1:]) if len(pop.memos) > 1
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
    print("         Everything else bounded by a non-id character is reported, licensed or")
    print("         not, so a spelling nobody has written yet is reported by default.")
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
