#!/usr/bin/env python3
"""Licensing rule, role ranking and assertions (a)-(d) for `plan-memo-umbrella-check.py`.

Everything here answers "is this mention licensed, which role does its context
spell, and do the four assertions of the single-home slot (minted in #506's
memo §8, not yet in the slot SoT ledger) hold over the row inventory?".
The mention scanners and the report stay in the checker; the row inventory
(tables, ids, kinds) is the transitive `Population` in `plan_memo_memo.py`,
which is the ONLY input of every assertion below: a row declared in a linked
memo is asserted exactly like a row of the main memo.  Findings are
`(code, file, lineno, message)`; a code ending in `?` is a seed and never gates.
"""

import re
from collections import Counter

from plan_memo_tables import (
    DECOR, MARKER, ROW_NOUN, ROW_NOUN_ID, SHORT_ID, SLUG_ID, balanced, decorated_id, is_empty,
    stream,
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
    re.IGNORECASE | re.ASCII,
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
    re.IGNORECASE | re.ASCII,
)

# The three mention shapes, each the ONE decorated-id grammar with a different
# core / anchor (group `id` is the id token in all three):
#   * a row noun anchors a short id in PROSE (`ROW_NOUN_ID`: noun, a space or
#     a hyphen, a decorated short id);
#   * a slot id is a naming site however the document spells it: backticked,
#     bold, both, or bare.  Accepting only the backticked form let an
#     ownership claim written as `**#11-vm-foo**` or plain `#11-vm-foo` pass
#     unreported under a rule whose stated polarity is reported-by-default;
#   * a bare short id, in a cell or in prose, tokenised by `_bare` on the
#     separators those blocks actually use.  No row noun is required; its
#     decoration must BALANCE (`balanced`): `**A call at the finalizer sites
#     is not the fix.**` opens with `**A` and a space, and an unbalanced-
#     decoration rule read that as a decorated row id `A`.  Measured: three
#     such sites in this memo before the balance requirement.
# ⚠ ASCII boundaries by PROPERTY: every anchor around these ASCII grammars
# is an explicit ASCII class (or the pattern is compiled `re.ASCII`), never
# `\b` / `\w` in Unicode mode -- there `次のSlice C` has no word boundary
# before `Slice` and `次は#11-zz-alpha` none before `#`, and both naming sites
# went unreported.
MENTION_PROSE = re.compile(r"(?<![0-9A-Za-z])" + ROW_NOUN_ID + r"(?![0-9A-Za-z])")
MENTION_SLOT = re.compile(r"(?<![0-9A-Za-z_-])" + decorated_id(SLUG_ID))
CELL_TOKEN = re.compile(decorated_id(SHORT_ID))


# A row noun standing between the licensing phrase and the id ("the child of
# umbrella **3**") must not hide the phrase from the backward look.
_TRAILING_NOUN = re.compile(r"(?<![0-9A-Za-z])" + ROW_NOUN + r"[ \t\n-]+" + DECOR + "$")

# The backward look reads the 40 characters before the mention (after a
# trailing row noun is dropped); a row noun + its decoration is shorter than
# 40, so an 80-character window is the same read without a block-length copy.
_BEFORE = 80


def classify(m):
    """Set `m.licensed` from what stands around the match in its block's
    DISPOSED stream (`m.text`): a licensing phrase inside a code span is
    code, not a licence."""
    before = _TRAILING_NOUN.sub("", m.text[max(0, m.start - _BEFORE): m.start])
    m.licensed = bool(LICENSE_BEFORE.search(before[-40:])
                      or LICENSE_AFTER.match(m.text[m.end:]))
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
_OWNER_CORE = "(?:" + SLUG_ID + "|" + SHORT_ID + ")"
OWNS_TWO = re.compile(
    r"\b(?:owns?|owned by|owner is|carries|carried by)\s+"
    + decorated_id(_OWNER_CORE, "a")
    + r"\s*(?:,\s*|\s+and\s+|\s+or\s+|\s*/\s*)"
    + decorated_id(_OWNER_CORE, "b"),
    re.IGNORECASE | re.ASCII)


def _owner_ok(m, tag):
    return balanced(m, tag) or m.group(tag + "id").startswith("#11-")


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

# The kind said in WORDS, for assertion (a)'s seed half.
DECLARES = re.compile(
    r"(?:is an umbrella|not a terminal unit|≥3 intersecting|three intersecting|"
    r"no canonical algorithm|edge-dense)",
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
    for file, name, lineno, rid, other in pop.attributed:
        findings.append(
            ("UMBRELLA-MARK", file, lineno,
             "row %r carries the marker in its declaring field but attributes it to row %r; "
             "§5 says a pointer slot carries no marker of its own, so it is NOT in the count"
             % (rid, other)))
    notes.append(
        "[UMBRELLA-MARK] %d rows carry the marker in their declaring field "
        "(%s) -- read from the declaring field, not from a grep over the marker"
        % (len(umb), ", ".join("%s=%d" % kv for kv in sorted(by_table.items())))
    )
    for row in pop.declaring_rows():
        if MARKER in row.field:
            continue
        if DECLARES.search(row.field):
            # SEED, with a measured false-positive mechanism: this
            # vocabulary also appears when a cell QUOTES the criterion to
            # conclude the row is terminal, and when a cell discusses
            # ANOTHER row's kind.  Deciding which is natural language.
            findings.append(
                ("UMBRELLA-MARK?", row.memo.path.name, row.lineno,
                 "row %r uses the kind vocabulary in its declaring field without the "
                 "marker -- read it: a declaration, a quotation of the criterion, or "
                 "another row's kind?" % row.self_id))
        # the marker outside the declaring field certifies nothing
        if any(MARKER in stream(c.lexed)
               for i, c in enumerate(row.cells) if i != row.schema.decl):
            findings.append(
                ("UMBRELLA-MARK", row.memo.path.name, row.lineno,
                 "row %r carries the marker outside its declaring field" % row.self_id))


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
        if not is_empty(deps):
            findings.append(("UMBRELLA-CELL", row.memo.path.name, row.lineno,
                             "%s row %r carries a Deps edge: %s" % (kind, row.self_id, deps[:120])))
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
        empty = is_empty(deps)
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
            findings.append(("ORDER-PROSE?", row.memo.path.name, row.lineno,
                             "row %r states ordering vocabulary in prose while its Deps cell is %r"
                             % (rid, deps)))
            continue
        # Non-empty: report only when the prose names a party the cell does not.
        # ⚠ Only ids that EXIST: `MENTION_PROSE` matches ROW_NOUN + token, and
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
            findings.append(("ORDER-PROSE?", row.memo.path.name, row.lineno,
                             "row %r states ordering vocabulary in prose naming %s, which its Deps "
                             "cell does not carry" % (rid, ", ".join(repr(e) for e in extra))))
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
            findings.append(("TWO-OWNERS?", row.memo.path.name, row.lineno,
                             "row %r assigns one deliverable to %r and %r in one clause: %r"
                             % (row.self_id, a, b, m.group(0)[:110])))
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
        findings.append(("ACCEPT-VOCAB?", row.memo.path.name, row.lineno,
                         "active-terminal row %r carries no acceptance vocabulary" % rid))
    notes.append(
        "[ACCEPT-VOCAB] SEED -- %d ACTIVE-TERMINAL §5 rows (not umbrella, not "
        "kind-undetermined, not a pointer, not retired) carry no acceptance vocabulary: %s. "
        "The slot that owns this states the approximation's miss class IS the deliverable; "
        "this figure bounds nothing." % (n, ", ".join(named) if named else "(none)"))
