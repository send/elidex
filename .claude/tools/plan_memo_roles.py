#!/usr/bin/env python3
"""Licensing rule, role ranking and assertions (a)-(d) for `plan-memo-umbrella-check.py`.

Everything here answers "is this mention licensed, which role does its context
spell, and do the four assertions of the single-home slot (minted in #506's
memo §8, not yet in the slot SoT ledger) hold over the row inventory?".
The mention scanners and the report stay in the checker; the row inventory
(tables, ids, kinds) is the transitive `Population` in `plan_memo_population.py`,
which is the ONLY input of every assertion below: a row declared in a linked
memo is asserted exactly like a row of the main memo.  Findings are
`(code, file, lineno, message)`; a code ending in `?` is a seed and never gates.
"""

import bisect
import re
from collections import Counter

from plan_memo_ids import AFTER, BEFORE, ROW_ID, balanced, bounded, decorated_id, kind_of
from plan_memo_stream import MARKER_RE, stream
from plan_memo_tables import ROW_NOUN_SEP, is_empty


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
# child is "of", or the runner of a derivation.  Each phrase is written ONCE,
# here, and the pattern and the index of where one may START are both composed
# from this tuple (`LICENSE_BEFORE` / `_LICENCE_KEYWORD`), so a phrase added
# below is indexed by arriving in it and cannot be reachable by one reader and
# not the other.
_LICENCE_PHRASES = (
    r"child(?:ren)?\s+(?:of\s+)?",           # the child of X / any child of X / children of X
    r"derivation\s+(?:that\s+)?",            # the derivation Slice B runs at its own start
    r"naming\s+",                            # naming X itself would name nobody
    r"mint(?:s|ed|ing)?\s+(?:onto\s+)?",     # ... mints / minted / minting X
)

LICENSE_BEFORE = re.compile(
    BEFORE +                                  # PR #510 R22, see LICENSE_AFTER
    r"(?:" + "|".join(_LICENCE_PHRASES) +
    # ⚠ NO TRAILING-ROW-NOUN CLAUSE, and that is a deletion rather than an
    # omission (PR #510 R24).  A `_TRAILING_NOUN` substitution stood here --
    # "a row noun between the licensing phrase and the id (`the child of
    # umbrella **3**`) must not hide the phrase" -- and it was UNREACHABLE:
    # wherever a row noun stands immediately before a declared id, the
    # ANCHORED pass reads that site too, its span STARTS at the noun, and it
    # wins the dedup (`collect_mentions`: "the anchored reading wins because
    # its span is what the licensing rule was written against"), so no mention
    # this rule is ever asked about has a row noun between the phrase and its
    # start.  The anchored pass is a superset of the bare pass at exactly
    # those positions -- it admits a digit and a single letter the bare pass
    # declines, and takes the same token otherwise -- so the case is not rare,
    # it is empty.  Measured both ways before deleting, and RE-RUNNABLE rather
    # than a count that goes stale: drop this clause and run `--self-test`
    # plus the #506 `--worklist` -- NO control turns red and the census is
    # byte-identical -- and do the same to the `_TRAILING_NOUN` substitution
    # at `3a9f61a0`, which was already dead there.  It is deleted rather than
    # ported because a clause nothing can reach reports coverage this rule
    # does not have.
    r")(?:the\s+)?$",
    re.IGNORECASE | re.ASCII,
)

# What may stand immediately AFTER the mention: the mention possesses one of
# the things §5 says an umbrella carries, or the sentence is about its kind.
# ⚠ No `(?:\*\*)?` prefix: the mention's own token consumes the decoration it
# carries (`plan_memo_ids.DECOR`), and since design re-gate 4 the only `**`
# left in the stream is a KEPT id decoration -- whose content is an id, never a
# licensed noun -- so a decoration spelled here would be a second spelling of
# the id grammar's that nothing can reach.  Measured: removing it moves no
# control and no site of the #506 memo (1,205 mentions, 491 licensed, both
# before and after).
# ⚠ NO `^`, and the difference is a COPY rather than a semantic (PR #510
# R31-4).  `.match(s, pos)` is anchored AT `pos` by definition, so `^` -- which
# matches only at the string's real beginning -- was there to serve a call that
# handed the pattern a fresh slice of the tail (`m.text[m.end:]`).  That slice
# is one copy of the rest of the block per mention, which is quadratic in the
# block for the same reason the backward look was, and it buys nothing an
# argument does not: `.match(m.text, m.end)` matches exactly where the slice's
# position 0 was.
LICENSE_AFTER = re.compile(
    r"(?:"
    r"(?:'s|’s)\s+(?:own\s+)?(?:derivation|children|charter|memo|split|plan-memo|sub-slices)"
    r"|,?\s+whose\s+(?:derivation|charter|children)"
    r"|\s+is\s+an?\s+umbrella"
    r"|\s+runs\s+at\s+its\s+own\s+start"
    r"|\s+became\s+an\s+umbrella"
    r")" + AFTER,
    re.IGNORECASE | re.ASCII,
)

# Where a licensing phrase may BEGIN: the literal each phrase of
# `_LICENCE_PHRASES` opens with.  DERIVED from the phrases rather than listed
# beside them -- `licence_index_control` requires each phrase's source to open
# with that literal and to continue with a regex metacharacter, so a phrase
# whose first move is not a plain literal turns the control red instead of
# quietly leaving its sites out of the index.
# ⚠ AND IT CARRIES NO BOUNDARY OF ITS OWN.  `LICENSE_BEFORE` opens with
# `BEFORE`, so `grandchild of 9z` is not the licensing phrase -- and that edge
# is stated THERE, once.  Spelling it here as well would make this index a
# second, silent statement of it: a mutant that dropped `BEFORE` from the
# pattern then survived, because the index went on rejecting the site the
# pattern had stopped rejecting (measured, PR #510 R31-4).  A superset of the
# starts is all this needs to be -- what it must never do is miss one -- and
# the pattern applied at each of them is the one reader of the edge.
# ⚠ A phrase that opens with no plain literal CONTRIBUTES NOTHING here rather
# than raising, and that is deliberate: an exception at import would make
# `licence_index_control` a CRASH instead of a red control, and a crash proves
# nothing about the clause (the mutation proof counts one as a failure for
# exactly that reason).  The control states the property in both directions --
# each phrase opens with a literal that this set holds, and the index agrees
# with the whole preceding text at every position of a generated corpus -- so a
# phrase whose sites would silently leave the index is named there.
_LICENCE_KEYWORD = re.compile(
    "|".join(sorted({m.group() for m in
                     (re.match(r"[a-z]+", p) for p in _LICENCE_PHRASES) if m})),
    re.IGNORECASE | re.ASCII,
)


def licence_starts(text):
    """Every offset of `text` at which a `LICENSE_BEFORE` phrase MAY begin, in
    order -- one linear scan, read once per block and shared by every mention
    in it (`Block.licence`).

    WHY A BLOCK NEEDS ONE AT ALL (PR #510 R31-4, and it is a regression of my
    own).  `LICENSE_BEFORE` is anchored at `$` against the mention, and R24
    established that "immediately before" is a GRAMMAR fact rather than a
    character count: it deleted a 40-character slice whose index 0 let the
    lookbehind succeed against nothing, and handed the pattern the block's
    whole preceding text (`search(m.text, 0, m.start)`) instead.  That was
    right about the boundary and wrong about the cost -- it replaced a bounded
    scan with an unbounded one, and since every mention of the block asks, a
    paragraph of N mentions scanned O(N^2) characters (measured 3.0x then 3.5x
    per doubling over `"9z unrelated words. " * n`).

    THE NEW BOUND IS THE SAME KIND OF FACT AS THE OLD ONE, which is what keeps
    both properties: no width appears anywhere here, and the position comes
    from the grammar.  Every branch of `LICENSE_BEFORE` opens with a plain
    literal, so a match can only START where one of those literals stands; and
    of the candidates before a mention, only the LAST one can reach it.  A
    match beginning earlier would have to CONTAIN that candidate, and it
    cannot: what a phrase spells is its own keyword, whitespace, and the words
    `of` / `that` / `onto` / `the`, and no keyword literal occurs inside any of
    those or inside another keyword.  So the pattern is applied at ONE
    position, with `match` rather than `search`, and the walk that chooses the
    position is the caller's and countable in Python (which is exactly what
    `leading_run_scan_control` says a scanning method's is not).

    THE INDEX IS A SUPERSET AND THE PATTERN DECIDES.  Every edge of the rule
    -- the `BEFORE` boundary, the whitespace, the trailing `the` -- is read
    from `LICENSE_BEFORE` alone, at the position this hands it; an offset here
    that no phrase actually begins at costs one failed match.  The failure
    that matters is the other one, so `licence_index_control` states the
    property this must have: at every position of a generated corpus, the
    licensed verdict is the one the whole preceding text gives."""
    return [m.start() for m in _LICENCE_KEYWORD.finditer(text)]
# ⚠ THE OUTER EDGES, PR #510 R22.  These two are the licensing rule -- a match
# SUPPRESSES a report -- so an unbounded edge is the dangerous polarity: it
# licenses a mention on a phrase the document does not contain.  `LICENSE_
# BEFORE` was unbounded on the left (`grandchild of 9z`, `renaming 9z`,
# `remints 9z` all read as the licensed phrase) and `LICENSE_AFTER` on the
# right (`9z's memorandum`, `9z is an umbrellaless slot`).  The inner edges
# need nothing: each pattern is anchored (`$` / `^`) against the MENTION,
# whose own extent the id grammar already bounded (`plan_memo_ids.tokens`).
# The plural (`9z's derivations`) is now REPORTED rather than licensed --
# tightening a licence is the safe direction under a reported-by-default
# rule, and widening the nouns to `s?` would license `9z's memos` and
# `9z's splits` on a list this seed does not name.  Measured on the #506
# memo: 1,205 mentions, 491 licensed, unchanged by both edges.
# ⚠ THE LEFT EDGE WAS ALSO GIVEN A SLICE, and that was the same defect one
# level down (PR #510 R24).  `classify` handed the backward look the 40
# characters before the mention, so a licensing phrase 40 characters long began
# at index 0 of that slice with nothing behind it and `BEFORE`'s lookbehind
# succeeded VACUOUSLY -- measured reachable with `"derivation" + " " * 21 +
# "that the "`, licensing a mention the document says `xderivation … that the`
# about.  Widening the slice moves the edge; only removing it removes the edge,
# and it is removable because "immediately before" is a GRAMMAR fact: the
# pattern is anchored at `$`, so the search is bounded by `endpos` at the
# mention's start and reads the block's whole preceding text.  There is no
# width constant in the backward look at all now, and no slice for a
# lookbehind to fall off the start of.

# The mention shapes.  Every id token -- a short id or a `#11-` slug, in a
# cell or in prose, decorated or bare -- is read by the ONE grammar
# (`plan_memo_ids.tokens`: the checker's `_bare` pass; a slug is its own
# anchor, and a slot id is a naming site however the document spells it --
# backticked, bold, both, or bare; accepting only the backticked form let an
# ownership claim written as `**#11-vm-foo**` or plain `#11-vm-foo` pass
# unreported under a rule whose stated polarity is reported-by-default).  A
# short id's decoration must BALANCE (`Token.balanced`): `**A call at the
# finalizer sites is not the fix.**` opens with `**A` and a space, and an
# unbalanced-decoration rule read that as a decorated row id `A`.  Measured:
# three such sites in this memo before the balance requirement.
#
# The ANCHORED reading (the one the licensing rule was written against) is
# a row noun, a space or a hyphen, then a short-id token starting exactly
# where `NOUN_ANCHOR` ends -- the token's boundary is the grammar's, not a
# second spelling here.
# ⚠ ASCII boundaries by PROPERTY: the anchor before the row noun composes the
# grammar's `ALNUM`, never `\b` / `\w` in Unicode mode -- there `次のSlice C`
# has no word boundary before `Slice` and `次は#11-zz-alpha` none before `#`,
# and both naming sites went unreported.
NOUN_ANCHOR = re.compile(BEFORE + ROW_NOUN_SEP)


def classify(m):
    """Set `m.licensed` from what stands around the match in its block's
    DISPOSED stream (`m.text`): a licensing phrase inside a code span is
    code, not a licence.

    NEITHER SIDE IS GIVEN A SLICE, AND NEITHER IS GIVEN THE BLOCK.  Both
    patterns are anchored against the mention -- `LICENSE_BEFORE` at `$`,
    `LICENSE_AFTER` at the position it is applied at -- and the mention's own
    extent is the id grammar's, so "immediately before" and "immediately
    after" are stated by the grammar and need no width.  A slice is not a
    cheaper spelling of an anchor: it truncates the MATCH, and a lookbehind at
    index 0 of a slice succeeds against nothing (PR #510 R24, the note above
    `NOUN_ANCHOR`).

    Each side is therefore applied at ONE position, and each position is a
    fact of the grammar rather than a count of characters (PR #510 R31-4):
    forward, the mention's own end; backward, the last offset before the
    mention at which a licensing phrase may begin -- `licence_starts`, the
    block's index, which carries the argument that no earlier one can reach.
    R24 removed the backward width for a CORRECTNESS reason and left an
    unbounded scan in its place, so every mention re-read the block from its
    start; the index keeps R24's boundary (there is still no width here) and
    costs one bisect per mention instead."""
    starts = m.licence
    j = bisect.bisect_left(starts, m.start) - 1
    m.licensed = bool((j >= 0 and LICENSE_BEFORE.match(m.text, starts[j], m.start))
                      or LICENSE_AFTER.match(m.text, m.end))
    return m


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
        r"waits? on|until|once)\b", re.IGNORECASE | re.ASCII)),
    ("owner", re.compile(
        r"\b(?:owns?|owned|owner|belongs?|carries|carry|holds?|responsible|"
        r"assigned|charter(?:ed)?s? to|placed on|home|hand(?:s|ed)?-?off)\b",
        re.IGNORECASE | re.ASCII)),
    ("landing", re.compile(
        r"\b(?:lands?|landed|landing|ships?|shipped|retires?|retired|merged|"
        r"PR|delivers?|deliverable)\b", re.ASCII)),
    ("acceptance", re.compile(
        r"\b(?:acceptance|witness|regression|assert(?:s|ion)?|must|probe|"
        r"observable|green|red)\b", re.IGNORECASE | re.ASCII)),
]


def roles(m, w=110):
    """Role vocabulary around the mention, read from the disposed stream."""
    ctx = m.window(w)
    return [name for name, pat in ROLE_PATTERNS if pat.search(ctx)]


# --------------------------------------------------------------------------
# Assertions (a)-(d) of the single-home slot #506's memo §8 mints
# --------------------------------------------------------------------------


def _stream(row, header_cell):
    """The disposed stream of the cell under `header_cell` -- the ONE text a
    seed's vocabulary reads (a `gates` or a `MERGED` inside a code span is
    code, not prose)."""
    return stream(row.col(header_cell).lexed)


def _row_key(x):
    """(memo identity, line) of a `Row` or a `Mention`: the seeds' row map key."""
    return (x.memo.key, x.lineno)

# ⚠ Both ids must be DECORATED (`**id**` / `` `id` ``, balanced) or a `#11-`
# slug.  The first attempt allowed a bare 1-4 char token, and `[0-9A-Za-z]{1,4}`
# matches a fragment INSIDE a word: "own manager" parsed as owners `m` and
# `ator`, and the seed reported 31 clauses of pure noise.  A seed that reports
# garbage is worse than one that reports nothing, because a reader cannot tell
# them apart.  One decorated-id group per owner (`a` / `b`); `_two_owners`
# checks the balance, so `**id**` and `` `id` `` match and `**id`` ` does not.
# An owner is a row of any row kind: the grammar's `ROW_ID` (slug | short),
# the same alternation the appositive and the anchored reading compose --
# a local `(?:slug|short)` here was a second spelling of it until PR #510 R20.
OWNS_TWO = re.compile(
    r"\b(?:owns?|owned by|owner is|carries|carried by)\s+"
    + decorated_id(ROW_ID, "a")
    + r"\s*(?:,\s*|\s+and\s+|\s+or\s+|\s*/\s*)"
    + decorated_id(ROW_ID, "b"),
    re.IGNORECASE | re.ASCII)


def _owner_ok(m, tag):
    return balanced(m, tag) or kind_of(m.group(tag + "id")) == "slug"


# ORDER-PROSE?'s vocabulary is DELIBERATELY narrower than the ranking's
# "ordering" row below: the ranking reads `until` / `once` / `follows` /
# `depends` / `gates` as single words because it only orders a reported set,
# while this seed PRODUCES findings, and those words alone are the acceptance
# prose of nearly every terminal row ("until the probe is green", "once the
# drain runs").  Measured on the #506 memo's §5 tables (64 rows, `rank.search`
# vs `ORDER_WORDS.search` over each Slice cell): the ranking vocabulary
# matches 61 rows, this one 39 -- of which 36 reach a finding.
ORDER_WORDS = re.compile(
    r"\b(?:before|after|lands? (?:first|second)|prerequisite of|gates?|blocked by|"
    r"depends? on|ordered (?:before|after)|sequenced (?:before|after))\b",
    re.IGNORECASE | re.ASCII,
)
# EXACTLY the two tokens `#11-plan-memo-acceptance-falsifiability-check` names.
# It read `witness|regression|assert` as well for one revision, which is a
# DIFFERENT predicate from the one this reproduces, and reproducing a figure
# with a wider predicate than the figure's own is how a cross-check agrees with
# something it never measured.
ACCEPT_WORDS = re.compile(r"\b(?:acceptance|must)\b", re.IGNORECASE | re.ASCII)

# A row that has already landed states no acceptance condition it still owes.
RETIRED = re.compile(r"\bMERGED\b|\bRETIRED\b|\bLANDED\b", re.ASCII)

# The kind said in WORDS, for assertion (a)'s seed half.  BOUNDED, from the
# grammar's one spelling (PR #510 R22): unbounded, `not a terminal unitary
# claim` and `edge-densely` seeded a finding this vocabulary does not name.
DECLARES = re.compile(
    bounded(r"is an umbrella|not a terminal unit|≥3 intersecting|three intersecting|"
            r"no canonical algorithm|edge-dense"),
    re.IGNORECASE | re.ASCII,
)


def assertion_a(pop, findings, notes):
    """Every row that DECLARES itself an umbrella carries the marker.

    Mechanical half: the marker count read from the masked declaring field,
    reported with the two halves so the figure is a program's output rather
    than recall.  Seed half: a row whose declaring field says the kind in
    words -- "is an umbrella", "edge-dense", "no canonical algorithm" --
    without the literal.
    """
    umb = pop.ids_of_kind("umbrella")
    by_table = Counter(r.schema.name for r in umb.values())
    for file, _name, lineno, name, other in pop.attributed:
        findings.append(
            ("UMBRELLA-MARK", file, lineno,
             "row %s carries the marker in its declaring field but attributes it to row %r; "
             "§5 says a pointer slot carries no marker of its own, so it is NOT in the count"
             % (name, other)))
    notes.append(
        "[UMBRELLA-MARK] %d rows carry the marker in their declaring field "
        "(%s) -- read from the declaring field, not from a grep over the marker"
        % (len(umb), ", ".join("%s=%d" % kv for kv in sorted(by_table.items())))
    )
    for row in pop.declaring_rows():
        # The short-circuit is the SEED's alone: a row that carries the marker
        # in its declaring field needs no seed for the kind said in words.  It
        # is NOT the out-of-field scan's -- that scan says every marker
        # outside the declaring field is mechanically invalid, and a row
        # marked in its own field and marked AGAIN in another cell (`Primary
        # module(s)`, `Deps`) is exactly the double-marker this assertion
        # forbids.  Until PR #510 R21 one `continue` gated both, so the
        # repeated marker was reported only on rows that had none where it
        # belongs.
        if not MARKER_RE.search(row.field) and DECLARES.search(row.field):
            # SEED, with a measured false-positive mechanism: this
            # vocabulary also appears when a cell QUOTES the criterion to
            # conclude the row is terminal, and when a cell discusses
            # ANOTHER row's kind.  Deciding which is natural language.
            findings.append(
                ("UMBRELLA-MARK?", pop.display(row.memo.path), row.lineno,
                 "row %s uses the kind vocabulary in its declaring field without the "
                 "marker -- read it: a declaration, a quotation of the criterion, or "
                 "another row's kind?" % row.name()))
        # the marker outside the declaring field certifies nothing
        if any(MARKER_RE.search(stream(c.lexed))
               for i, c in enumerate(row.cells) if i != row.schema.decl):
            findings.append(
                ("UMBRELLA-MARK", pop.display(row.memo.path), row.lineno,
                 "row %s carries the marker outside its declaring field" % row.name()))


def assertion_b(pop, findings, notes):
    """The `Deps` half of assertion (b).  The acceptance half has no cell to read
    and is left to (c)/(d)'s natural-language class; see the header."""
    # Both no-owner kinds: §5 gives a kind-undetermined row the same "carries the
    # split and nothing else" obligation, so scoping this to the UMBRELLA marker
    # left four §5 rows unchecked for the very cell it is about.
    no_owner = pop.no_owner_ids()
    checked = 0
    for row in pop.data_rows("slice"):
        if row.self_id not in no_owner:
            continue
        kind = "umbrella" if no_owner[row.self_id].kind == "umbrella" else "kind-undetermined"
        checked += 1
        deps = row.col("Deps").text
        if not is_empty(_stream(row, "Deps")):
            findings.append(("UMBRELLA-CELL", pop.display(row.memo.path), row.lineno,
                             "%s row %s carries a Deps edge: %s" % (kind, row.name(), deps[:120])))
    notes.append(
        "[UMBRELLA-CELL] %d §5 no-owner rows (umbrella + kind-undetermined) checked for a Deps edge. "
        "⚠ HALF of assertion (b): the acceptance half is NOT checked and is not "
        "mechanisable -- §5 gives acceptance no cell, only prose in the Slice cell. "
        "A `0` here says nothing about it." % checked)


def assertion_cd_seed(pop, mentions, findings, notes):
    """(c) prose ordering vs the cell it names, and (d) two owners in one row.
    `mentions` = the population's row-noun-anchored mentions (every declared
    id, not only the no-owner ones), which is the seed's reading of the prose.

    Both are natural-language claim extraction, for which the memo's own cell
    says no canonical algorithm exists.  What is reported here is a SEED: rows
    whose prose carries ordering vocabulary while their `Deps` cell is empty.
    A row that states an ordering in words the seed does not carry is invisible
    to it, and no count printed here bounds that class.
    """
    # ⚠ DECLARED MISS, measured: this compares **ids**, not **artifacts**.  Row
    # 9db's `Deps` named "the child umbrella 9da's derivation mints for the
    # thenable-job tick" while its prose measured an edge onto the child 9da
    # mints for *thenable assimilation* -- 9da's own cell lists the two as
    # SEPARATE axes, so the cell pointed at a different child than the
    # measurement reached.  `9da` is in the cell, so the id never enters
    # `extra` and this seed cannot see it.  An artifact-level comparison is a
    # different program; this one does not attempt it.
    n = 0
    # (file, lineno) -> ids the anchored pass read in that row's Slice cell,
    # and -> every id the population read in its Deps cell (the same mask,
    # the same grammar, the same keep-set: one scan, not two -- a second
    # tokeniser over the raw cell once read `9z` out of `slice-9z-sib.md`)
    # keyed on the memo's resolved path + line (`Memo.key`): a basename key
    # aliases two memos of the same name in different directories
    named, in_deps = {}, {}
    for m in mentions:
        if m.anchored and m.source == "slice:Slice":
            named.setdefault(_row_key(m), set()).add(m.id)
        if m.source == "slice:Deps":
            in_deps.setdefault(_row_key(m), set()).add(m.id)
    for row in pop.data_rows("slice"):
        rid = row.self_id
        deps = row.col("Deps").text
        body = _stream(row, "Slice")
        empty = is_empty(_stream(row, "Deps"))
        if not ORDER_WORDS.search(body):
            continue
        # ⚠ A NON-EMPTY `Deps` cell does not discharge this.  The seed used to
        # `continue` on one, so a row whose cell names ONE party while its prose
        # hands off to a SECOND was invisible -- which is exactly how 2ac's
        # super-property dependency lived in a "lands second" sentence while the
        # cell, §5's single home for ordering, named only umbrella A's child.
        # A partially-filled cell is the harder case, not the settled one.
        if empty:
            n += 1
            findings.append(("ORDER-PROSE?", pop.display(row.memo.path), row.lineno,
                             "row %s states ordering vocabulary in prose while its Deps cell is %r"
                             % (row.name(), deps)))
            continue
        # Non-empty: report only when the prose names a party the cell does not.
        # ⚠ Only ids that EXIST: the anchored pass matches ROW_NOUN + token, and
        # "rows sat" / "row says" / "Slice has" would put `sat` / `says` / `has`
        # in the set -- a token shaped like an id is not an id.  The mention
        # scan already filters to declared ids and reads through the mask (the
        # `[\s-]+` separator once read `...-slice-1a-1b-...md` as "Slice 1a";
        # measured: 18 filename-derived hits across §5, 2 reaching this seed).
        cell_ids = in_deps.get(_row_key(row), set())
        prose_ids = named.get(_row_key(row), set())
        extra = sorted(prose_ids - cell_ids - {rid})
        if extra:
            n += 1
            findings.append(("ORDER-PROSE?", pop.display(row.memo.path), row.lineno,
                             "row %s states ordering vocabulary in prose naming %s, which its Deps "
                             "cell does not carry" % (row.name(), ", ".join(repr(e) for e in extra))))
    notes.append("[ORDER-PROSE?] SEED -- %d rows; the class is natural language and is not bounded by this figure" % n)

    # (d) TWO-OWNERS.  A SEED keyed on ownership vocabulary, which is the miss
    # class: a row assigning one deliverable to two owners in words this
    # pattern does not carry is invisible to it, and the count bounds nothing.
    d = 0
    for row in pop.data_rows("slice"):
        for m in OWNS_TWO.finditer(_stream(row, "Slice")):
            a, b = m.group("aid"), m.group("bid")
            if a == b or not (_owner_ok(m, "a") and _owner_ok(m, "b")):
                continue
            d += 1
            findings.append(("TWO-OWNERS?", pop.display(row.memo.path), row.lineno,
                             "row %s assigns one deliverable to %r and %r in one clause: %r"
                             % (row.name(), a, b, m.group(0)[:110])))
    notes.append("[TWO-OWNERS?] SEED -- %d clause(s); ownership-vocabulary keyed, so a row that "
                 "spells it otherwise is not in this figure" % d)


def acceptance_vocab_seed(pop, findings, notes):
    """Terminal §5 rows carrying neither `must` nor `acceptance`.

    ⚠ The population is ACTIVE-TERMINAL, derived explicitly.  It used to be
    "every row that is not an umbrella", which misclassifies three other kinds:
    a **retired** row (`0a — MERGED`) has already landed, and a **kind-undetermined**
    row is *forbidden* to carry an acceptance condition until its kind is
    measured -- so listing them told a reader to add exactly what §5 forbids.
    Measured on the committed memo, that complement reported `0a`, `0c`, `E` and
    `10`, and every one of the four was wrong.
    """
    n, named = 0, []
    for row in pop.data_rows("slice"):
        rid = row.self_id
        body = _stream(row, "Slice")
        # the population decided the kind once (`Population._kind`): a no-owner
        # row and a pointer row owe no acceptance condition
        if row.kind != "terminal":
            continue
        if RETIRED.search(body) or RETIRED.search(_stream(row, "#")):
            continue
        if ACCEPT_WORDS.search(body):
            continue
        n += 1
        named.append(rid)
        findings.append(("ACCEPT-VOCAB?", pop.display(row.memo.path), row.lineno,
                         "active-terminal row %s carries no acceptance vocabulary" % row.name()))
    notes.append(
        "[ACCEPT-VOCAB] SEED -- %d ACTIVE-TERMINAL §5 rows (not umbrella, not "
        "kind-undetermined, not a pointer, not retired) carry no acceptance vocabulary: %s. "
        "The slot that owns this states the approximation's miss class IS the deliverable; "
        "this figure bounds nothing." % (n, ", ".join(named) if named else "(none)"))
