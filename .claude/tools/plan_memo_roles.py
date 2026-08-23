#!/usr/bin/env python3
"""Licensing rule, role ranking and assertions (a)-(d) for `plan-memo-umbrella-check.py`.

Everything here answers "is this mention licensed, which role does its context
spell, and do the four assertions of
`#11-plan-memo-spec-field-single-home-check` hold over the row inventory?".
The mention scanners and the report stay in the checker; the row inventory
(tables, ids, kinds) is the transitive `Population` in `plan_memo_tables.py`,
which is the ONLY input of every assertion below: a row declared in a linked
memo is asserted exactly like a row of the main memo.  Findings are
`(code, file, lineno, message)`; a code ending in `?` is a seed and never gates.
"""

import re
from collections import Counter

from plan_memo_tables import MARKER, ROW_NOUN, SCHEMAS, bare_id, mask_spans, masked_text


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
# A slot id is a naming site however the document spells it: backticked,
# bold, both, or bare.  Accepting only the backticked form let an ownership
# claim written as `**#11-vm-foo**` or plain `#11-vm-foo` pass unreported
# under a rule whose stated polarity is reported-by-default.
MENTION_SLOT = re.compile(r"(?<![\w-])(?:\*\*|`)*(#11-[a-z0-9-]+)(?:\*\*|`)*")
# Bare ids inside a mention-bearing table cell, tokenised on the separators
# those cells actually use.  No row noun is required, because the column's
# grammar is what makes the token an id.
# Decoration must BALANCE.  `**A call at the finalizer sites is not the fix.**`
# opens with `**A` and a space; an unbalanced-decoration rule reads that as a
# decorated row id `A` and reports the sentence opener.  Measured: three such
# sites in this memo before the balance requirement.
CELL_TOKEN = re.compile(r"(?P<l>\*\*|`)?(?P<id>[0-9A-Za-z]{1,4})(?P<r>\*\*|`)?")
CELL_SPLIT = re.compile(r"[\s,;/()\[\]·→>+&]+")



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

# ⚠ Both ids must be DECORATED (`**id**` / `` `id` ``) or a `#11-` slug.  The
# first attempt allowed a bare 1-4 char token, and `[0-9A-Za-z]{1,4}` matches a
# fragment INSIDE a word: "own manager" parsed as owners `m` and `ator`, and the
# seed reported 31 clauses of pure noise.  A seed that reports garbage is worse
# than one that reports nothing, because a reader cannot tell them apart.
_OWNER_REF = r"(?:\*\*(?P<%s>#11-[a-z0-9-]+|[0-9A-Za-z]{1,4})\*\*|`(?P<%s>#11-[a-z0-9-]+|[0-9A-Za-z]{1,4})`)"
OWNS_TWO = re.compile(
    r"\b(?:owns?|owned by|owner is|carries|carried by)\s+"
    + (_OWNER_REF % ("a1", "a2"))
    + r"\s*(?:,\s*|\s+and\s+|\s+or\s+|\s*/\s*)"
    + (_OWNER_REF % ("b1", "b2")),
    re.IGNORECASE)

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

# A row that has already landed states no acceptance condition it still owes.
RETIRED = re.compile(r"\bMERGED\b|\bRETIRED\b|\bLANDED\b")

# The `slice` schema's body columns the assertions read.
SLICE_ID, SLICE_BODY, SLICE_DEPS = 0, 1, 5


def assertion_a(pop, findings, notes):
    """Every row that DECLARES itself an umbrella carries the marker.

    Mechanical half: the marker count read from the masked declaring field,
    reported with the two halves so the figure is a program's output rather
    than recall.  Seed half: a row whose declaring field says the kind in
    words -- "is an umbrella", "edge-dense", "no canonical algorithm" --
    without the literal.
    """
    umb = pop.umbrella_ids()
    by_table = Counter(t for _, _, t, _ in umb.values())
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
    declares = re.compile(
        r"(?:is an umbrella|not a terminal unit|≥3 intersecting|three intersecting|"
        r"no canonical algorithm|edge-dense)",
        re.IGNORECASE,
    )
    keep = pop.keep()
    for name, _, decl, idc in SCHEMAS:
        if decl is None or idc is None:
            continue
        for memo, lineno, cells in pop.data_rows(name):
            rid = bare_id(cells[idc].text)
            field = masked_text(cells[decl].text, keep)
            if MARKER not in field and declares.search(field):
                # SEED, with a measured false-positive mechanism: this
                # vocabulary also appears when a cell QUOTES the criterion to
                # conclude the row is terminal, and when a cell discusses
                # ANOTHER row's kind.  Deciding which is natural language.
                findings.append(
                    ("UMBRELLA-MARK?", memo.path.name, lineno,
                     "row %r uses the kind vocabulary in its declaring field without the "
                     "marker -- read it: a declaration, a quotation of the criterion, or "
                     "another row's kind?" % rid))
            # the marker outside the declaring field certifies nothing
            if MARKER not in field and any(
                    MARKER in masked_text(c.text, keep) for i, c in enumerate(cells) if i != decl):
                findings.append(
                    ("UMBRELLA-MARK", memo.path.name, lineno,
                     "row %r carries the marker outside its declaring field" % rid))


def assertion_b(pop, findings, notes):
    """The `Deps` half of assertion (b).  The acceptance half has no cell to read
    and is left to (c)/(d)'s natural-language class; see the header."""
    # Both no-owner kinds: §5 gives a kind-undetermined row the same "carries the
    # split and nothing else" obligation, so scoping this to the UMBRELLA marker
    # left four §5 rows unchecked for the very cell it is about.
    no_owner = pop.no_owner_ids()
    checked = 0
    for memo, lineno, cells in pop.data_rows("slice"):
        rid = bare_id(cells[SLICE_ID].text)
        if rid not in no_owner:
            continue
        kind = "umbrella" if no_owner[rid][0] == "umbrella" else "kind-undetermined"
        checked += 1
        deps = cells[SLICE_DEPS].text
        if deps and deps not in {"—", "-", "n/a"}:
            findings.append(("UMBRELLA-CELL", memo.path.name, lineno,
                             "%s row %r carries a Deps edge: %s" % (kind, rid, deps[:120])))
    notes.append(
        "[UMBRELLA-CELL] %d §5 no-owner rows (umbrella + kind-undetermined) checked for a Deps edge. "
        "⚠ HALF of assertion (b): the acceptance half is NOT checked and is not "
        "mechanisable -- §5 gives acceptance no cell, only prose in the Slice cell. "
        "A `0` here says nothing about it." % checked)


def assertion_cd_seed(pop, findings, notes):
    """(c) prose ordering vs the cell it names, and (d) two owners in one row.

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
    known = pop.keep()
    for memo, lineno, cells in pop.data_rows("slice"):
        rid = bare_id(cells[SLICE_ID].text)
        deps = cells[SLICE_DEPS].text
        body = cells[SLICE_BODY].text
        empty = deps in {"", "—", "-"}
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
            findings.append(("ORDER-PROSE?", memo.path.name, lineno,
                             "row %r states ordering vocabulary in prose while its Deps cell is %r"
                             % (rid, deps)))
            continue
        # Non-empty: report only when the prose names a party the cell does not.
        # ⚠ Filter to ids that EXIST.  `MENTION_PROSE` matches ROW_NOUN + token,
        # and "rows sat" / "row says" / "Slice has" put `sat` / `says` / `has`
        # in the set -- the same garbage the two-owner seed produced one commit
        # earlier, from the same cause: a token shaped like an id is not an id.
        cell_ids = {m.group("id") for m in CELL_TOKEN.finditer(deps)} | set(MENTION_SLOT.findall(deps))
        # ⚠ Mask first: the `[\s-]+` separator (added for `Slice-M`) read
        # `...-slice-1a-1b-...md` as "Slice 1a".  Measured: 18 filename-derived
        # hits across §5, 2 of which reached this seed's output.
        _mask = mask_spans(body, known, memo.defs)
        prose_ids = {m.group(1) for m in MENTION_PROSE.finditer(body)
                     if not any(s <= m.start(1) < e for s, e in _mask)} & known
        extra = sorted(prose_ids - cell_ids - {rid})
        if extra:
            n += 1
            findings.append(("ORDER-PROSE?", memo.path.name, lineno,
                             "row %r states ordering vocabulary in prose naming %s, which its Deps "
                             "cell does not carry" % (rid, ", ".join(repr(e) for e in extra))))
    notes.append("[ORDER-PROSE?] SEED -- %d rows; the class is natural language and is not bounded by this figure" % n)

    # (d) TWO-OWNERS.  A SEED keyed on ownership vocabulary, which is the miss
    # class: a row assigning one deliverable to two owners in words this
    # pattern does not carry is invisible to it, and the count bounds nothing.
    d = 0
    for memo, lineno, cells in pop.data_rows("slice"):
        rid = bare_id(cells[SLICE_ID].text)
        for m in OWNS_TWO.finditer(cells[SLICE_BODY].text):
            a = m.group("a1") or m.group("a2")
            b = m.group("b1") or m.group("b2")
            if a == b:
                continue
            d += 1
            findings.append(("TWO-OWNERS?", memo.path.name, lineno,
                             "row %r assigns one deliverable to %r and %r in one clause: %r"
                             % (rid, a, b, m.group(0)[:110])))
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
    no_owner = pop.no_owner_ids()
    n, named = 0, []
    for memo, lineno, cells in pop.data_rows("slice"):
        rid = bare_id(cells[SLICE_ID].text)
        body = cells[SLICE_BODY].text
        if rid in no_owner:
            continue
        # ⚠ Keyed on one spelling, and the safe polarity: a differently-spelled
        # pointer row is REPORTED, never missed.
        if "is a pointer rather than a slice" in body:
            continue
        if RETIRED.search(body) or RETIRED.search(cells[SLICE_ID].text):
            continue
        if ACCEPT_WORDS.search(body):
            continue
        n += 1
        named.append(rid)
        findings.append(("ACCEPT-VOCAB?", memo.path.name, lineno,
                         "active-terminal row %r carries no acceptance vocabulary" % rid))
    notes.append(
        "[ACCEPT-VOCAB] SEED -- %d ACTIVE-TERMINAL §5 rows (not umbrella, not "
        "kind-undetermined, not a pointer, not retired) carry no acceptance vocabulary: %s. "
        "The slot that owns this states the approximation's miss class IS the deliverable; "
        "this figure bounds nothing." % (n, ", ".join(named) if named else "(none)"))
