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

⚠ **`plan-sweep.py` is the canonical tool for a class this file does NOT cover,
and it should be reached for first.**  Its subject is the restatement sweep --
"You are changing one decision.  This lists EVERY site in the memo that restates
it" across STATEMENT / OBLIGATION / CONSEQUENCE surfaces.  That is the
un-propagated-decision failure, and running it over this memo returns 41 sites in
seconds.  This file's subject is different: which rows carry no owner, and which
prose names one of them in a role §5 says it cannot hold.  Use `plan-sweep.py`
when you change a decision; use this when you want the naming population.

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

Usage:  plan-memo-umbrella-check.py <memo> [<sibling> ...]
        plan-memo-umbrella-check.py --self-test
"""

import re
import sys
import pathlib
from collections import Counter, defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from plan_memo_tables import (  # noqa: E402
    MARKER, ROW_NOUN, SCHEMAS, UNDETERMINED, Memo, bare_id, code_spans,
    find_tables, is_row, is_separator, split_row,
)

# --------------------------------------------------------------------------
# The licensing rule.
#
# §5 enumerates what an umbrella row DOES carry: (1) the split, a charter, a
# derivation from which it mints terminal children at its own start, its own
# plan-memo, and those children.  It says the row carries neither (2) the
# ordering nor (3) the owner, and no acceptance condition.
#
# So the predicate is stated over what the id is ATTACHED TO, not over a list
# of bad phrasings: a mention is licensed iff the umbrella is named as the
# possessor of one of the things §5 says it carries, or as the thing a child
# is "of", or as the subject of a statement about its kind.  Every other
# attachment -- an ordering, an owner, a landing, an acceptance -- is reported.
#
# This is an exemption list, and an exemption list leaves the next class
# authoritative unless the DEFAULT is the safe side.  Here it is: a spelling
# absent from both tables below is REPORTED, never admitted.  The cost is
# false positives on legitimate prose, which a reader disposes of in one read;
# the cost of the other polarity is the class this file was written for.
# --------------------------------------------------------------------------

# What may stand immediately BEFORE the mention: the mention is the thing a
# child is "of", or the runner of a derivation.
LICENSE_BEFORE = re.compile(
    r"(?:"
    r"child(?:ren)?\s+(?:of\s+)?"          # the child of X / any child of X / children of X
    r"|derivation\s+(?:that\s+)?"           # the derivation Slice B runs at its own start
    r"|naming\s+"                           # naming X itself would name nobody
    r"|mint(?:s|ed|ing)?\s+(?:onto\s+)?"    # ... mints / minted / minting X
    r")(?:the\s+)?$",
    re.IGNORECASE,
)

# What may stand immediately AFTER the mention: the mention possesses one of
# the things §5 says an umbrella carries, or the sentence is about its kind.
LICENSE_AFTER = re.compile(
    r"^(?:\*\*)?(?:"
    r"(?:'s|’s)\s+(?:own\s+)?(?:derivation|children|charter|memo|split|plan-memo|sub-slices)"
    r"|,?\s+whose\s+(?:derivation|charter|children)"
    r"|\s+is\s+an?\s+umbrella"
    r"|\s+runs\s+at\s+its\s+own\s+start"
    r"|\s+became\s+an\s+umbrella"
    r")",
    re.IGNORECASE,
)

# Row nouns that anchor an id in PROSE.  Over table cells no anchor is needed
# where a bare token is read as an id by `_bare` without any anchor at all.
# The id may be decorated with bold, backticks, or both, in either order.
DECOR_ID = r"(?:\*\*|`)*([0-9A-Za-z]{1,4})(?:\*\*|`)*"

# `Slice-M` / `Slice-4a` are the same anchor with a hyphen.  Requiring `\s+`
# left them invisible to both passes; measured, four of five such sites in this
# memo are real violations.
MENTION_PROSE = re.compile(r"\b" + ROW_NOUN + r"[\s-]+" + DECOR_ID + r"(?![0-9A-Za-z])")
MENTION_SLOT = re.compile(r"`(#11-[a-z0-9-]+)`")
# Bare ids inside a mention-bearing table cell, tokenised on the separators
# those cells actually use.  No row noun is required, because the column's
# grammar is what makes the token an id.
# Decoration must BALANCE.  `**A call at the finalizer sites is not the fix.**`
# opens with `**A` and a space; an unbalanced-decoration rule reads that as a
# decorated row id `A` and reports the sentence opener.  Measured: three such
# sites in this memo before the balance requirement.
CELL_TOKEN = re.compile(r"(?P<l>\*\*|`)?(?P<id>[0-9A-Za-z]{1,4})(?P<r>\*\*|`)?")
CELL_SPLIT = re.compile(r"[\s,;/()\[\]·→>+&]+")


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
# Role ranking.
#
# This is a RANKING over the reported set, never a filter on it.  The whole
# report is the population; the rank only says which sites to read first.
# Defining the POPULATION by this vocabulary would be the exact mistake
# `feedback_checks-must-not-be-defined-by-the-symptom-vocabulary` records --
# a site that spells the same role in words absent from these lists would then
# be authoritative.  Here it is merely ranked LOW and still printed.
# --------------------------------------------------------------------------

ROLE_PATTERNS = [
    ("ordering", re.compile(
        r"\b(?:before|after|first|second|prerequisite|gates?|gated|blocked|blocks|"
        r"depends?|dependent|deps|sequenced|order(?:ed|ing)?|precede|follows?|"
        r"waits? on|until|once)\b", re.IGNORECASE)),
    ("owner", re.compile(
        r"\b(?:owns?|owned|owner|belongs?|carries|carry|holds?|responsible|"
        r"assigned|charter(?:ed)?s? to|placed on|home|hand(?:s|ed)?-?off)\b",
        re.IGNORECASE)),
    ("landing", re.compile(
        r"\b(?:lands?|landed|landing|ships?|shipped|retires?|retired|merged|"
        r"PR|delivers?|deliverable)\b")),
    ("acceptance", re.compile(
        r"\b(?:acceptance|witness|regression|assert(?:s|ion)?|must|probe|"
        r"observable|green|red)\b", re.IGNORECASE)),
]


def roles(m, w=110):
    ctx = m.line[max(0, m.start - w) : m.end + w]
    return [name for name, pat in ROLE_PATTERNS if pat.search(ctx)]


# --------------------------------------------------------------------------
# Assertions (a)-(d) of `#11-plan-memo-spec-field-single-home-check`
# --------------------------------------------------------------------------

ORDER_WORDS = re.compile(
    r"\b(?:before|after|lands? (?:first|second)|prerequisite of|gates?|blocked by|"
    r"depends? on|ordered (?:before|after)|sequenced (?:before|after))\b",
    re.IGNORECASE,
)
# EXACTLY the two tokens `#11-plan-memo-acceptance-falsifiability-check` names.
# It read `witness|regression|assert` as well for one revision, which is a
# DIFFERENT predicate from the one this reproduces, and reproducing a figure
# with a wider predicate than the figure's own is how a cross-check agrees with
# something it never measured.
ACCEPT_WORDS = re.compile(r"\b(?:acceptance|must)\b", re.IGNORECASE)


def assertion_a(memo, findings, notes, attributed=None):
    """Every row that DECLARES itself an umbrella carries the marker.

    Mechanical half: the marker count read from the declaring field, reported
    with the two halves so the figure is a program's output rather than recall.
    Seed half: a row whose declaring field says the kind in words -- "is an
    umbrella", "edge-dense", "no canonical algorithm" -- without the literal.
    """
    attributed = [] if attributed is None else attributed
    umb = memo.umbrella_ids(attributed=attributed)
    by_table = Counter(t for t, _ in umb.values())
    for name, lineno, rid, other in attributed:
        findings.append(
            ("UMBRELLA-MARK", lineno,
             "row %r carries the marker in its declaring field but attributes it to row %r; "
             "§5 says a pointer slot carries no marker of its own, so it is NOT in the count"
             % (rid, other)))
    notes.append(
        "[UMBRELLA-MARK] %d rows carry the marker in their declaring field "
        "(%s) -- read from the declaring field, not from a grep over the marker"
        % (len(umb), ", ".join("%s=%d" % kv for kv in sorted(by_table.items())))
    )
    declares = re.compile(
        r"(?:is an umbrella|not a terminal unit|≥3 intersecting|three intersecting|"
        r"no canonical algorithm|edge-dense)",
        re.IGNORECASE,
    )
    for name, hdr, decl, idc, _ in SCHEMAS:
        if decl is None or idc is None:
            continue
        for lineno, cells in memo.data_rows(name):
            if len(cells) <= max(decl, idc):
                continue
            field = cells[decl]
            if MARKER in field:
                continue
            if declares.search(field):
                # SEED, and it has a measured false-positive mechanism: this
                # vocabulary also appears when a cell QUOTES the criterion in
                # order to conclude the row is terminal, and when a cell
                # discusses ANOTHER row's kind.  Deciding which of the three a
                # sentence is doing is natural language, so the words-half of
                # assertion (a) is reported as a seed and the marker-population
                # read above is the mechanical half.
                findings.append(
                    ("UMBRELLA-MARK?", lineno,
                     "row %r uses the kind vocabulary in its declaring field without the "
                     "marker -- read it: a declaration, a quotation of the criterion, or "
                     "another row's kind?" % bare_id(cells[idc])))
            # the marker outside the declaring field certifies nothing
            row = "|".join(cells)
            if MARKER in row and MARKER not in field:
                findings.append(
                    ("UMBRELLA-MARK", lineno,
                     "row %r carries the marker outside its declaring field" % bare_id(cells[idc])))


def assertion_b(memo, findings, notes):
    """The `Deps` half of assertion (b).  The acceptance half has no cell to read
    and is left to (c)/(d)'s natural-language class; see the header."""
    # Both no-owner kinds: §5 gives a kind-undetermined row the same "carries the
    # split and nothing else" obligation, so scoping this to the UMBRELLA marker
    # left four §5 rows unchecked for the very cell it is about.
    umb = memo.umbrella_ids()
    undet = memo.undetermined_ids()
    checked = 0
    for lineno, cells in memo.data_rows("slice"):
        if len(cells) <= 6:
            continue
        rid = bare_id(cells[1])
        kind = "umbrella" if rid in umb else ("kind-undetermined" if rid in undet else None)
        if kind is None:
            continue
        checked += 1
        deps = cells[6].strip()
        if deps and deps not in {"—", "-", "n/a"}:
            findings.append(("UMBRELLA-CELL", lineno,
                             "%s row %r carries a Deps edge: %s" % (kind, rid, deps[:120])))
    notes.append(
        "[UMBRELLA-CELL] %d §5 no-owner rows (umbrella + kind-undetermined) checked for a Deps edge. "
        "⚠ HALF of assertion (b): the acceptance half is NOT checked and is not "
        "mechanisable -- §5 gives acceptance no cell, only prose in the Slice cell. "
        "A `0` here says nothing about it." % checked)


def assertion_cd_seed(memo, findings, notes):
    """(c) prose ordering vs the cell it names, and (d) two owners in one row.

    Both are natural-language claim extraction, for which the memo's own cell
    says no canonical algorithm exists.  What is reported here is a SEED: rows
    whose prose carries ordering vocabulary while their `Deps` cell is empty.
    A row that states an ordering in words the seed does not carry is invisible
    to it, and no count printed here bounds that class.
    """
    n = 0
    for lineno, cells in memo.data_rows("slice"):
        if len(cells) <= 6:
            continue
        rid = bare_id(cells[1])
        deps = cells[6].strip()
        if deps not in {"", "—", "-"}:
            continue
        body = cells[2]
        if ORDER_WORDS.search(body):
            n += 1
            findings.append(("ORDER-PROSE?", lineno,
                             "row %r states ordering vocabulary in prose while its Deps cell is %r"
                             % (rid, deps)))
    notes.append("[ORDER-PROSE?] SEED -- %d rows; the class is natural language and is not bounded by this figure" % n)


def acceptance_vocab_seed(memo, findings, notes):
    """The two-token approximation `#11-plan-memo-acceptance-falsifiability-check`
    prints and rejects: a §5 row that is terminal, is not a pointer, and carries
    neither `must` nor `acceptance`.

    Reproduced here because the slot's cell states a figure for it that a reader
    would otherwise have to take on trust.  Its measured miss class is the whole
    deliverable, not a tail: a row that states no acceptance condition while
    spelling `must` once in ordinary design prose is authoritative under it.
    """
    umb = memo.umbrella_ids()
    hits = []
    for lineno, cells in memo.data_rows("slice"):
        if len(cells) <= 6:
            continue
        rid = bare_id(cells[1])
        if rid in umb:
            continue
        body = cells[2]
        # ⚠ Keyed on ONE spelling.  Measured: three §5 rows carry it and exactly
        # three are pointer rows, so it is exact today -- but rows R and 1b also
        # use the word "pointer" and are correctly not excluded only by accident
        # of phrasing.  The polarity is the safe one: a differently-spelled
        # pointer row is REPORTED, never missed.
        if "is a pointer rather than a slice" in body:
            continue
        if not ACCEPT_WORDS.search(body):
            hits.append((lineno, rid))
    notes.append(
        "[ACCEPT-VOCAB] SEED -- %d §5 terminal non-pointer rows carry no acceptance vocabulary%s. "
        "The slot that owns this states the approximation's miss class IS the deliverable; "
        "this figure bounds nothing."
        % (len(hits), (": " + ", ".join(r for _, r in hits)) if hits else "")
    )
    for lineno, rid in hits:
        findings.append(("ACCEPT-VOCAB?", lineno, "row %r carries no acceptance vocabulary" % rid))


# --------------------------------------------------------------------------
# Report
# --------------------------------------------------------------------------


def collect_mentions(memo, umb, siblings=()):
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
    for sib in siblings:
        sp = pathlib.Path(sib)
        sl = sp.read_text().split("\n")
        sm = Memo(sib)
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
    if not paths:
        print(__doc__)
        return 2
    memo = Memo(paths[0])
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
    mentions = []
    mentions = collect_mentions(memo, umb, paths[1:])
    unlicensed = [m for m in mentions if not m.licensed]

    print("=" * 78)
    print("plan-memo-umbrella-check  --  %s" % memo.path)
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
