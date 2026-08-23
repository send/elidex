#!/usr/bin/env python3
"""Machine-check a plan memo's umbrella rows against the way its own prose names them.

Why this exists rather than another prose rule, and rather than a hand sweep.
The VM-P4 umbrella memo's §5 states that an umbrella row carries neither the
ordering nor the owner and no acceptance condition, so "naming an umbrella as
an owner, as a 'lands second' party, or in a `Deps` cell names *nobody*".  Four
converge rounds of PR #506 then found that rule violated by hand, one site per
round, and three separate sweeps each scoped themselves to whatever the
previous round had named -- §5's `Deps` column once, the acceptance class once,
that round's own corrections once -- so each swept the population that
motivated it rather than the population the rule reaches.  This program is the
enumerator those sweeps did not have.

It does NOT discharge either of the two slots that memo carves.  Both are
declared `UMBRELLA, not a terminal unit`, so what discharges them is their own
derivation minting terminal children.  `#11-plan-memo-spec-field-single-home-check`
says so in its own cell: of its four assertions, two are "mechanical programs
over this document's table parse" and two are natural-language claim extraction
"for which no canonical algorithm exists".  This file is the first pair, plus a
declared-recall seed for the second pair.  That boundary is printed in the
report rather than left to the reader, because a checker that prints `0` for a
class it cannot see is the failure the same document records under I-8.

PROGRAM AND MODULES
Carved out of #506 into its own program
(`docs/plans/2026-08-plan-memo-umbrella-checker.md`, branch
`vm-p4-plan-memo-checker`): the checker grew inside a converge loop without a
plan-review and then became the loop's only subject for three rounds.  Modules:
  plan_memo_lexer.py      CommonMark 0.31.2 / GFM 0.29 subset: fences, rows,
                          code spans, links, reference definitions
  plan_memo_tables.py     schemas, `Memo`, the transitive `Population`
  plan_memo_roles.py      licensing rule, role ranking, assertions (a)-(d)
  (this file)             mention scanners, `check()`, the report
  plan_memo_umbrella_selftest.py / _selftest_cases.py / _selftest_mutants.py
The §8 defer of the plan names the GFM row splitter duplicated across the
in-flight plan-memo programs on their branch families; `plan_memo_lexer.py::
split_row` is the candidate canonical copy, trigger = two of them on `main`.

⚠ This file does NOT cover the restatement sweep -- "You are changing one
decision.  This lists EVERY site in the memo that restates it" across
STATEMENT / OBLIGATION / CONSEQUENCE surfaces.  That class has its own tool on
another branch (`plan-sweep.py`); until it lands, a decision change over a memo
is swept by hand.  This file's subject is different: which rows carry no owner,
and which prose names one of them in a role §5 says it cannot hold.

WHERE THIS RUNS
Two places.  (1) By hand, on a memo: `SCHEMAS` matches one document family's
exact header rows, so against any other plan memo this program prints
`FATAL: no table matched schema ...` and exits 2 -- which is why the MEMO run
is not a trip-wire.  (2) Its self-test IS one:
`.claude/tools/plan-memo-umbrella-selftest-trip-wire.sh` runs
`--self-test --mutants` (memo-independent; fixtures live in `tempfile` dirs)
under `scripts/trip-wires.sh` on every PR, so the checker cannot rot on `main`
while the memo it gates is still in flight.

EXIT STATUS
  0  no mechanical finding
  1  at least one mechanical finding (the assertions, not the seeds)
  2  a schema miss -- an absent linked memo, an unmatched schema, a body row
     whose width differs from its header, the same id declared twice.  The run
     is a SKIP, not a clean result, and it is never exit 0.
Seeds and reported naming sites do NOT affect it.  They cannot: the naming scan
reports by default, so a green state would not exist and the code would be a
gate nobody could ever satisfy.

LEXING (plan §2 / §3 -- the bound is the listed constructs, nothing more)
  fenced blocks (CommonMark §4.5) masked -> GFM rows split on RAW unescaped `|`
  (GFM §4.10; an escaped pipe becomes `|`) -> per block (paragraph / cell): code spans
  (CommonMark §6.1, equal-length backtick strings; a span may cross a line, a
  line is a reporting coordinate only) -> links (CommonMark §6.3 / §4.7) over
  the masked stream -> row ids from the raw id cell -> disposition (an id-only
  code span is the document spelling an id: a mention) -> kind markers from the
  MASKED declaring field -> scanners.  Not lexed, read as written: §4.4 indented
  code, §4.6 HTML blocks, §6.5 autolinks, §2.5 entities.

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
  (c) ORDER-PROSE     SEED.  Prose asserting an ordering is natural language.
  (d) TWO-OWNERS      SEED.  Two sentences of one row naming two owners is
                      natural language.
  NAMING              mechanical over its population, SEED as to that population.
                      Every mention of a no-owner id that is not inside one of
                      the constructions §5 licenses.  Two passes, over every
                      table cell except the row's own id cell AND over every
                      paragraph outside the tables: a row-noun-anchored pass,
                      and a bare pass that needs no row noun.  Two id shapes are
                      DECLARED MISSES, held as red controls in the self-test:
                      a purely numeric id and an undecorated single letter,
                      each written without a row noun.
  ACCEPT-VOCAB        SEED, and declared as one by the memo itself: the slot
                      `#11-plan-memo-acceptance-falsifiability-check` measures
                      this approximation's own miss class and prints it.

The licensing rule is NOT a list of forbidden phrasings.  It is the complement:
a mention is licensed iff the umbrella is named as the possessor of a DERIVATION
or of its CHILDREN, or as the thing a child is "of".  Anything else is reported.

Usage:  plan-memo-umbrella-check.py <memo> [--worklist]   (linked memos = the population)
        plan-memo-umbrella-check.py --self-test [--mutants]
"""

import sys
import pathlib
from collections import Counter, defaultdict, namedtuple

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from plan_memo_tables import SCHEMAS, Population, bare_id, code_mask, mask_spans  # noqa: E402
from plan_memo_roles import (  # noqa: E402
    CELL_SPLIT, CELL_TOKEN, MENTION_PROSE, MENTION_SLOT, acceptance_vocab_seed,
    assertion_a, assertion_b, assertion_cd_seed, classify, roles,
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
    match span would count it twice.  All three are RAW columns of `line`.
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


class Block:
    """One scanned unit of text -- a cell or one line of a paragraph -- with
    its mask (spans in `text` coordinates the scanners must not read an id out
    of) and `raw`, the map from a `text` offset to a column of `line`."""

    __slots__ = ("file", "lineno", "line", "text", "raw", "mask", "link_mask", "source", "self_id")

    def __init__(self, file, lineno, line, text, raw, mask, link_mask, source, self_id=None):
        self.file, self.lineno, self.line, self.text = file, lineno, line, text
        self.raw, self.mask, self.link_mask = raw, mask, link_mask
        self.source, self.self_id = source, self_id

    def masked(self, i):
        return any(s <= i < e for s, e in self.mask)


def _anchored(b, umb, out):
    """Row-noun-anchored ids, plus `#11-` slot ids."""
    for mt in MENTION_PROSE.finditer(b.text):
        if mt.group(1) not in umb or mt.group(1) == b.self_id:
            continue
        if b.masked(mt.start(1)):
            continue
        out.append(classify(Mention(b.file, b.lineno, mt.group(1), b.raw(mt.start()),
                                    b.raw(mt.end()), b.line, b.source, b.raw(mt.start(1)))))
    # The slot pass reads through code spans: a slot id is always written
    # inside backticks, and a declared slot id is an id-only span and therefore
    # unmasked -- but a backticked command line naming a slot is not, and the
    # slot pass must still see it.  Links and fences mask it like everything
    # else.
    for mt in MENTION_SLOT.finditer(b.text):
        if mt.group(1) not in umb or mt.group(1) == b.self_id:
            continue
        if any(s <= mt.start(1) < e for s, e in b.link_mask):
            continue
        out.append(classify(Mention(b.file, b.lineno, mt.group(1), b.raw(mt.start()),
                                    b.raw(mt.end()), b.line, b.source)))


def _bare(b, umb, out):
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
        if tid not in umb or tid == b.self_id:
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
        out.append(classify(Mention(b.file, b.lineno, tid, b.raw(s), b.raw(e), b.line,
                                    b.source, b.raw(tok.start("id")))))


def blocks(memo, keep):
    """Every scanned unit of `memo`: each cell of each table row (header rows
    too; the delimiter row has no cells to scan; a schema data row's own id
    cell is excluded) and each line of each paragraph, with the block-level
    mask projected onto it."""
    name = memo.path.name
    out = []
    for t in memo.tables:
        rows = [(t.header_lineno, t.header, None)]
        for lineno, cells in t.rows:
            rows.append((lineno, cells, t.schema))
        for lineno, cells, schema in rows:
            line = memo.lines[lineno - 1]
            idc = next((i for n, _, _, i in SCHEMAS if n == schema), None)
            self_id = bare_id(cells[idc].text) if idc is not None else None
            for col, cell in enumerate(cells):
                if col == idc:
                    continue
                src = "%s:col%d" % (schema or "table", col)
                full = mask_spans(cell.text, keep, memo.defs)
                code = code_mask(cell.text, keep)
                out.append(Block(name, lineno, line, cell.text, cell.raw, full,
                                 [sp for sp in full if sp not in code], src, self_id))
    for para in memo.paragraphs:
        full = para.per_line(mask_spans(para.content, keep, memo.defs))
        code = para.per_line(code_mask(para.content, keep))
        for lineno, text in para.lines:
            f, c = full.get(lineno, []), code.get(lineno, [])
            out.append(Block(name, lineno, text, text, lambda i: i, f,
                             [sp for sp in f if sp not in c], "prose"))
    return out


def collect_mentions(pop, umb):
    """The whole naming pipeline, in ONE place, over the whole population."""
    mentions = []
    keep = pop.keep() | set(umb)
    for memo in pop.memos:
        for b in blocks(memo, keep):
            _anchored(b, umb, mentions)
            _bare(b, umb, mentions)
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


# --------------------------------------------------------------------------
# The pipeline
# --------------------------------------------------------------------------

Result = namedtuple("Result", "findings notes rc mentions population")


def check(path):
    """The ONLY pipeline: `main()` and `--self-test` both run this.

    Returns `Result(findings, notes, rc, mentions, population)`; findings are
    `(code, file, lineno, message)`.  rc 2 = the population could not be
    scanned (every schema miss is listed as a `SCHEMA` finding); rc 1 = a
    mechanical finding; rc 0 = none.  Seeds (`?` codes) and naming sites never
    gate.
    """
    pop = Population(path)
    findings, notes = [], []
    for file, lineno, msg in pop.misses:
        findings.append(("SCHEMA", file, lineno, msg + ". This is a skip, not a clean run."))
    if pop.misses:
        return Result(findings, notes, 2, [], pop)
    umb = pop.no_owner_ids()
    if not umb:
        findings.append(("SCHEMA", pop.main.path.name, 0,
                         "no umbrella rows found -- the table schema did not match. "
                         "This is a skip, not a clean run."))
        return Result(findings, notes, 2, [], pop)
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
    assertion_a(pop, findings, notes)
    assertion_b(pop, findings, notes)
    assertion_cd_seed(pop, findings, notes)
    acceptance_vocab_seed(pop, findings, notes)
    mentions = collect_mentions(pop, umb)
    # Mechanical findings only.  A code ending in `?` is a SEED -- a class this
    # program cannot decide -- and seeds do not gate, nor do naming sites, which
    # are non-zero by construction because the scan reports by default.
    mechanical = [f for f in findings if not f[0].endswith("?")]
    return Result(findings, notes, 1 if mechanical else 0, mentions, pop)


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
    byrole = Counter()
    for m in unlicensed:
        r = roles(m)
        byrole["+".join(r) if r else "(no role vocabulary)"] += 1
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
    mechanical = [f for f in res.findings if not f[0].endswith("?")]
    print("%d mechanical finding(s) gate the exit status; %d seed(s) and %d reported "
          "naming site(s) do not." % (len(mechanical), len(res.findings) - len(mechanical),
                                      len(unlicensed)))
    return res.rc


if __name__ == "__main__":
    sys.exit(main(sys.argv))
