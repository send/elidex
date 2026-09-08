#!/usr/bin/env python3
"""The GENERATED growth control for `plan-memo-umbrella-check.py --self-test`:
the one linearity witness whose population is derived from the grammar rather
than written against a shape.

WHY IT IS ITS OWN MODULE.  `plan_memo_selftest_work.py` is a list of
per-clause witnesses -- each one names a construct, a bound and a
discriminating partner, and each one was written after a review round
reported that construct.  This module holds the opposite thing: a corpus
generated from the grammar (`_growth_atoms` / `_growth_corpus`) and ONE rule
applied to every member of it.  The two are different mechanisms with
different failure modes, so they are different modules -- carved while
writing, at the 800-line mark the work module would otherwise have crossed,
rather than left for a later split.

It is still a WORK module: it measures cost with a harness witness
(`_count_line_sites`) and never a wall clock, its `registry()` fragment is
merged by `plan_memo_selftest_work.registry()` into the one table, and it is
listed in `plan_memo_selftest_mutants.SELFTEST` so a mutant row naming it
patches the self-test rather than the checker set.

Import direction, one way: the work module imports this one; this one imports
the harness and nothing of the controls.
"""

import ast
import itertools
import pathlib

from plan_memo_selftest_harness import MODULES, SOURCES, _count_line_sites


# --------------------------------------------------------------------------
# The GENERATED growth corpus (PR #510 R27).
#
# Every atom below is READ OFF the grammar rather than listed: the characters
# the checker's own sources branch on, the emphasis delimiters the emphasis
# module declares, the raw-HTML openers `plan_memo_html._CLOSERS` names with
# their own closers, and one document spelling per id KIND `plan_memo_ids`
# declares.  The corpus is every atom repeated, and every unordered PAIR of
# atoms interleaved -- which is where the interactions live: eight of the
# nine non-linear sites this checker has had were a construct that only costs
# when a SECOND construct stands beside it.
#
# What stays hand-written is the CLOSED companion of a bracket construct (a
# resolved link, a reference, a code span), because no table of the grammar
# spells one; that residue is bounded by the completeness half of the control,
# which requires every branch character to be an atom in its own right.
# --------------------------------------------------------------------------

# The ids the corpus's probes declare, so the token atoms are the document
# SPELLING an id rather than prose that happens to look like one.
GROWTH_KEEP = ("9z", "#11-a", "[C1]")

# kind name (`plan_memo_ids.KINDS`) -> (the bare id, how a document spells it).
# The map is checked against `KINDS` at run time and against `kind_of`, so a
# kind added to the grammar without a spelling here turns the control red
# instead of silently leaving that kind's pairs out of the sweep.
ID_SPELLINGS = {"short": ("9z", "9z "), "slug": ("#11-a", "`#11-a` "), "cite": ("[C1]", "[C1] ")}

# The bracket constructs, each in the two states the scan distinguishes: an
# opener that never closes, and the closed form.  `a.md` is the fixture
# sibling every other control links to.
BRACKET_ATOMS = ("[", "![", "](", "[x](a.md)", "![x](a.md)", "[x][m]", "[x]",
                 "`", "`x`", "\\[", "\\\n", "&", "&amp;", "x.md ")

# Linear work doubles when the input doubles and quadratic work quadruples, so
# the bound sits between them; the slack above 2 absorbs the log factors of the
# bisects the scan runs (a line inside one runs N log N times, 2.09x at these
# sizes) without reaching a re-scan's 4.  The floor is for lines whose count is
# small enough that the shape of the text's boundary, not its length, decides
# it.
GROWTH_SLACK, GROWTH_FLOOR = 2.5, 16
# Two sizes for the sweep and two for the confirmation.  The sweep is
# deliberately over-inclusive: it flags every line that outgrew the bound at 6
# and 12 repetitions, which no quadratic can escape but a scan bounded by a
# CONSTANT (`DESTINATION_NESTING_LIMIT` = 32, whose plateau lies past those
# sizes) also trips.  The confirmation re-measures the flagged probes at 96 and
# 192, where a constant-bounded scan has flattened and only real growth is
# left.  Confirming costs ~250 ms a probe, so it runs on the candidates and not
# on the corpus.
GROWTH_SWEEP, GROWTH_CONFIRM = 6, 96
# How many confirmed probes a red verdict prints before it stops confirming:
# one is the verdict, and a handful is what said R27's thirteen were TWO
# defects rather than thirteen.
GROWTH_REPORT = 6


def _branch_literals(src):
    """Every ONE-CHARACTER string constant `src` COMPARES against -- the
    characters this source dispatches on (`c == "<"`, `s[j] in "<>\\n"`), and
    not the ones it merely emits.  `ast.Compare` is the whole population: `==`,
    `!=` and `in` are all comparisons, and a longer constant is a phrase rather
    than a character the scan branches at."""
    out = set()
    for node in ast.walk(ast.parse(src)):
        if isinstance(node, ast.Compare):
            for k in [node.left] + list(node.comparators):
                if isinstance(k, ast.Constant) and isinstance(k.value, str) and len(k.value) == 1:
                    out.add(k.value)
    return out


def _scan_alphabet():
    """The union of `_branch_literals` over the module set AS LOADED -- the
    sources `load()` recorded, so a mutant's patched text is what is read."""
    out = set()
    for _, file in MODULES:
        out |= _branch_literals(SOURCES[file])
    return out


def _growth_atoms():
    """The corpus atoms, sorted: the branch characters, the emphasis
    delimiters open and paired, the raw-HTML openers unclosed and closed, the
    bracket constructs, and one spelling per id kind."""
    import plan_memo_emphasis
    import plan_memo_html
    import plan_memo_ids

    atoms = set(_scan_alphabet()) | set(BRACKET_ATOMS)
    for d in plan_memo_emphasis.DELIMS:
        atoms |= {d, d + "x" + d}
    for opener, closer, _off in plan_memo_html._CLOSERS:
        atoms |= {opener + "x", opener + "x" + closer}
    # the sixth §6.6 alternative, whose closer is the bare `>` every other one
    # would also accept: a declaration, and an open tag
    atoms |= {"<!a", "<!a>", "<b>"}
    kinds = {name for name, _rx in plan_memo_ids.KINDS}
    if set(ID_SPELLINGS) != kinds:
        return None, "ID_SPELLINGS covers %s; plan_memo_ids.KINDS declares %s" % (
            sorted(ID_SPELLINGS), sorted(kinds))
    for kind, (bare, spelling) in sorted(ID_SPELLINGS.items()):
        if plan_memo_ids.kind_of(bare) != kind:
            return None, "the %r sample %r is a %r id" % (kind, bare, plan_memo_ids.kind_of(bare))
        atoms |= {spelling}
    return sorted(atoms), ""


def _growth_corpus(atoms):
    """(name, unit) for every atom repeated and every unordered PAIR of atoms
    interleaved.  Interleaved rather than blocked, because the shapes that
    have cost this checker its linear contract are a construct standing beside
    a second one over and over (`![` beside a resolved link; an id token
    beside a code span), not one run followed by another."""
    corpus = [(repr(a), a) for a in atoms]
    corpus += [("%s + %s" % (repr(x), repr(y)), x + y)
               for x, y in itertools.combinations(atoms, 2)]
    return corpus


def _read_block(text, keep):
    """The block-level inline reading of one text, which is what the corpus
    measures: Phase 2 over the raw text, the disposition, and the residue
    scan -- `plan_memo_lexer.inline_pass` and everything
    `plan_memo_tables.dispose` / `split_units` reach from it.  Not `check()`:
    a probe is one BLOCK, and the file, the memo and the population around it
    are constants of the run rather than of its length."""
    import plan_memo_lexer
    import plan_memo_tables

    lexed = plan_memo_lexer.Lexed(text)
    lexed.resolve({})
    plan_memo_tables.dispose(lexed, keep)
    plan_memo_tables.split_units(lexed, keep)


def _outgrew(small, large):
    """The source line that outgrew the bound worst, as (site, small, large,
    how far over), or None -- the ONE comparison every probe is judged by."""
    worst = None
    for site, big in large.items():
        allow = GROWTH_SLACK * small.get(site, 0) + GROWTH_FLOOR
        over = big / allow
        if over > 1.0 and (worst is None or over > worst[3]):
            worst = (site, small.get(site, 0), big, over)
    return worst


def _growth_at(unit, n, modules, keep):
    """`_outgrew` between `unit` repeated n and 2n times."""
    tallies = []
    for reps in (n, 2 * n):
        with _count_line_sites(modules) as c:
            _read_block(unit * reps, keep)
        tallies.append(c.counts)
    return _outgrew(*tallies)


def generated_growth_control(M):
    """THE LINEARITY CLAIM AS A PROPERTY, over a population GENERATED from the
    grammar: no source line of this checker's scans runs more than
    `GROWTH_SLACK` times as often when its input doubles.

    WHY THIS EXISTS.  "Is `inline_pass` linear?" had been answered wrong nine
    times in five review rounds.  R23 found two non-linear sites, R26 four
    (one reported, three found by taking the question rather than the report
    as the subject), R27 two more -- and one of R27's was not in `inline_pass`
    at all (`plan_memo_tables._straddles`, which summed the whole blank list
    per candidate token), which is what says the family is THIS PROGRAM'S
    SCANS and not that function.  Every round added a witness for the shape
    just reported, and every next round arrived with a shape nobody had
    written one for: a population defined by the symptoms already seen, which
    is the failure `feedback_checks-must-not-be-defined-by-the-symptom-vocabulary`
    names.  So the population is generated instead
    (`_growth_atoms` / `_growth_corpus`): every character the sources branch
    on, every emphasis delimiter, every raw-HTML opener with and without its
    closer, every bracket construct open and closed, one spelling per id kind
    -- each repeated, and each PAIR of them interleaved.

    WHAT IS MEASURED, and why it is per LINE.  A total line count is dominated
    by the linear pass over the text (~48 source lines per character here), so
    a re-scan whose inner loop is two lines hides inside it until the input is
    thousands of characters long -- both of R27's defects were invisible to a
    total at 200 characters.  Per SITE the constants cancel: a line the scan
    runs once per character doubles when the input doubles, a line inside a
    re-scan quadruples, and neither figure depends on what the rest of the
    pass costs.  Both of R27's defects are red under this control with no fix
    applied, at 6 and 12 repetitions -- `plan_memo_lexer`'s stack walk 78 ->
    300 where 2.5x + 16 allows 211, and `plan_memo_tables._straddles`' sum
    336 -> 1248 where it allows 856.

    TWO STAGES, because one bound cannot separate growth from a large
    CONSTANT at one size.  §6.3's destination scan is bounded by
    `DESTINATION_NESTING_LIMIT` (32), so at 6 and 12 repetitions it has not
    reached its plateau yet and looks quadratic; at 96 and 192 it has (its
    worst line measured 0.88 of the bound, against 1.59 for each real defect).
    The sweep is therefore deliberately over-inclusive and the confirmation
    decides -- and the sweep can be over-inclusive safely because a quadratic
    line quadruples at EVERY size, so nothing real escapes it.

    WHAT IT CANNOT SEE.  Work inside the C `re` engine (no Python line runs, so
    no line grows -- that is what `linear_html_attempts_control` counts
    ATTEMPTS for); work outside the block-level reading (Phase 1, the memo
    walk, the population -- their own controls are below); a cost that is
    superlinear in something other than the repeated unit (a nesting DEPTH,
    which `linear_image_demotion_control` probes); and a constant factor,
    which this control says nothing about by construction."""
    modules = [__import__(name) for name, _ in MODULES]
    keep = set(GROWTH_KEEP)
    atoms, why = _growth_atoms()
    if atoms is None:
        return False, "the corpus could not be generated: %s" % why
    # THE COMPLETENESS HALF: every character the sources branch on is an atom
    # of its own, so it is paired with every other atom.  Without it a
    # narrowed derivation would quietly shrink the population and every probe
    # left would still pass.
    missing = sorted(_scan_alphabet() - set(atoms))
    if missing:
        return False, "branch characters absent from the corpus: %r" % missing
    corpus = _growth_corpus(atoms)
    flagged = []
    for name, unit in corpus:
        if _growth_at(unit, GROWTH_SWEEP, modules, keep) is not None:
            flagged.append((name, unit))
    bad, tested = [], 0
    for name, unit in flagged:
        # the verdict needs ONE confirmation and the report needs a few, so a
        # red run stops confirming after `GROWTH_REPORT` of them.  The GREEN
        # path is unaffected -- it confirms every candidate, because none of
        # them confirms -- and it is the green path the trip-wire pays for.
        if len(bad) >= GROWTH_REPORT:
            break
        tested += 1
        w = _growth_at(unit, GROWTH_CONFIRM, modules, keep)
        if w is not None:
            (file, lineno), small, large, over = w
            bad.append("%s :: %s:%d ran %d -> %d over %d -> %d repetitions (%.2fx the bound)"
                       % (name, pathlib.Path(file).name, lineno, small, large,
                          GROWTH_CONFIRM, 2 * GROWTH_CONFIRM, over))
    return not bad, ("%d atoms, %d probes, %d flagged at %d/%d and %d confirmed at %d/%d%s%s"
                     % (len(atoms), len(corpus), len(flagged), GROWTH_SWEEP, 2 * GROWTH_SWEEP,
                        len(bad), GROWTH_CONFIRM, 2 * GROWTH_CONFIRM,
                        (": " + "; ".join(bad)) if bad else "",
                        (" (%d of the flagged probes were left unmeasured: the verdict is "
                         "already red)" % (len(flagged) - tested)) if tested < len(flagged) else ""))


def registry():
    """name -> (kind, control), this module's fragment of the one table."""
    return {
        "the scans are linear over a corpus GENERATED from the grammar: every branch character, delimiter, HTML opener, bracket construct and id kind, each repeated and each PAIR of them interleaved, and no source line grows worse than its input":
            ("CONTROL", generated_growth_control),
    }
