#!/usr/bin/env python3
"""The RATCHETS: controls that assert a MECHANISM covers a derived class.

Carved out of `plan_memo_selftest_properties.py` at PR #510 R49, when that
module crossed the 1000-line bound -- and it crossed it on the commit that
ADDED the second ratchet, so `line_bound_control` reported its own author.

⚠ The seam is a real one and not the line count.  Every control left in
`plan_memo_selftest_properties.py` sweeps the source for a property the code
must HAVE -- an anchored pattern is not also width-bounded, text I/O names its
encoding, no list is popped from the front, the dash set is spelled once, every
source compiles without a SyntaxWarning.  The two here ask a different
question: **is this CLASS ratcheted at all** -- is every loop that walks a
population pinned by a mutant that truncates it, is every kind-phrase question
asked at its one sanctioned site.  They are derived from the checker's AST,
each carries a STATED COMPLEMENT (`_SCOPE_EXEMPT`, `_KIND_QUESTION_SITES`)
rather than a list of what exists, and they are the family that grows as more
classes get a ratchet -- which is exactly why they should not grow inside a
module whose subject is something else.

Both exist because the same defect arrived three or four rounds running, each
time fixed at the site a reviewer had just named.  A ratchet is what turns
"fix the instance" into "the class is covered or the suite is red".

⚠ EACH RATCHET HAS A DISCRIMINATING PARTNER HERE (the fourth attestation over
`9b2e1fa9..00dfd095`).  A ratchet run only over the real source cannot tell a
precise criterion from a generous one: both print "0 unpinned" while nothing
is wrong, so reverting either ratchet to the keying it replaced left every
control green and no mutant row named this module.  The partners feed each
ratchet's PURE core a fixture the generous criterion gets wrong, and
`plan_memo_selftest_mutants_ratchets.py` re-injects each reverted criterion
against them.

`registry()` is merged by `plan_memo_selftest_controls.registry()`, the same
way the records and work fragments are, so nothing here is referenced by the
module it came from and the import goes one way.  It is called `registry` --
and this module is in `plan_memo_selftest_mutants.SELFTEST` -- so a mutant row
against it takes its controls from the PATCHED text.
"""

import ast

from plan_memo_selftest_harness import files
from plan_memo_selftest_properties import _swept_sources

CENSUS = "plan_memo_population.py"


# -- the loop ratchet --------------------------------------------------------

def _loops(text):
    """{identity: ast.For} for every `for` statement of a module, where the
    identity is POSITIONAL: (qualified name of the enclosing def or class
    body, ordinal of the loop among that body's loops in source order).

    ⚠ NOT THE LINE TEXT (the fourth attestation over `9b2e1fa9..00dfd095`).
    799349db keyed a loop on its exact header line, which is one step
    finer than the target prefix it replaced and the same alias one level
    down: `for memo in self.memos:` is the header of FIVE loops, `for s in
    SCHEMAS:` of three, and `for t in memo.tables:` / `for row in
    memo.schema_rows(s.name):` of two each -- so one truncating mutant still
    credited every loop spelled like its target, and "17 pinned" was 15.  A
    position is the loop itself; a spelling is a name that resembles it
    (`memory/feedback_gate-criterion-by-behaviour-not-by-name.md`)."""
    out = {}

    def body(node, qual):
        n = 0
        stack = list(ast.iter_child_nodes(node))[::-1]
        while stack:
            ch = stack.pop()
            if isinstance(ch, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                body(ch, qual + (ch.name,))
                continue
            if isinstance(ch, (ast.For, ast.AsyncFor)):
                n += 1
                out[(".".join(qual) or "<module>", n)] = ch
            stack.extend(list(ast.iter_child_nodes(ch))[::-1])

    body(ast.parse(text), ())
    return out


def _header(text, node):
    return text.split("\n")[node.lineno - 1].strip()


def _is_truncation(patched, original):
    """`patched` is `original[:1]` or `list(original)[:1]` -- the edit a
    scope mutant makes, recognised on the TREE, so a replacement that merely
    CONTAINS the original's spelling is not one."""
    if not (isinstance(patched, ast.Subscript) and isinstance(patched.slice, ast.Slice)):
        return False
    sl = patched.slice
    if sl.lower is not None or sl.step is not None or not (
            isinstance(sl.upper, ast.Constant) and sl.upper.value == 1):
        return False
    v = patched.value
    if (isinstance(v, ast.Call) and isinstance(v.func, ast.Name) and v.func.id == "list"
            and len(v.args) == 1 and not v.keywords):
        v = v.args[0]
    return ast.dump(v) == ast.dump(original)


def _intends_truncation(replace):
    """A row whose REPLACEMENT spells a truncated loop header: it is meant to
    pin a loop, so crediting none is an orphan, not a quiet no-op."""
    return any(ln.strip().startswith("for ") and " in " in ln and "[:1]" in ln
               for ln in replace.split("\n"))


def _credit(text, rows):
    """({identity: [row names]}, [orphan row names]) -- which loop of `text`
    each (name, find, replace) row TRUNCATES, computed per row against the
    positional identity: the row is applied, both trees are walked, and a
    loop is credited when the loop AT THE SAME POSITION had its iterable
    replaced by the truncation of the original one.  Nothing else credits --
    not the find text, not a header that reads the same elsewhere."""
    base = _loops(text)
    credit, orphans = {}, []
    for name, find, replace in rows:
        hits = set()
        if text.count(find) == 1:
            try:
                patched = _loops(text.replace(find, replace))
            except SyntaxError:
                patched = {}
            hits = {k for k, node in base.items()
                    if k in patched and _is_truncation(patched[k].iter, node.iter)}
        for k in hits:
            credit.setdefault(k, []).append(name)
        if not hits and _intends_truncation(replace):
            orphans.append(name)
    return credit, orphans


def _scope_verdict(text, rows, exempt):
    """(bad, n_loops, n_pinned, n_exempt, n_unpinned) -- the loop ratchet's
    PURE core, over any module text, any rows and any stated complement, so
    its discriminating partner can hand it a fixture."""
    loops = _loops(text)
    credit, orphans = _credit(text, rows)
    bad = []
    for k, (want, _reason) in sorted(exempt.items()):
        if k not in loops:
            bad.append("the exempt entry %s names no loop -- the stated complement is stale" % (k,))
        elif _header(text, loops[k]) != want:
            bad.append("the exempt entry %s is `%s`, not `%s` -- a position that moved exempts "
                       "a different loop" % (k, _header(text, loops[k]), want))
    unpinned = [k for k in loops if k not in exempt and k not in credit]
    bad += ["%d `%s` (%s #%d) is neither pinned nor in the stated complement"
            % (loops[k].lineno, _header(text, loops[k]), k[0], k[1]) for k in unpinned]
    bad += ["the truncating mutant %r credits no loop -- its edit truncates nothing the module "
            "has" % n for n in orphans]
    # ⚠ DISJOINT, because the first version's three numbers summed to 21 over 20
    # loops -- a loop that is exempt AND pinned was counted twice.  Exempt is
    # decided first, so each loop lands in exactly one bucket.
    n_exempt = sum(1 for k in loops if k in exempt)
    n_pinned = sum(1 for k in loops if k not in exempt and k in credit)
    assert n_exempt + n_pinned + len(unpinned) == len(loops), "the three buckets must partition"
    return bad, len(loops), n_pinned, n_exempt, len(unpinned)


_SCHEMAS_REASON = ("SCHEMAS is a module constant of the GRAMMAR, not a population this walk "
                   "collects; its own completeness is `schema_*` controls' subject")
_RAISES = ("truncating it RAISES -- `stream() before dispose()` -- so the scope is held by "
           "the program's own assertion and not by a control. MEASURED, not assumed: the "
           "truncation was run and the suite reports an AssertionError, and the mutant "
           "runner counts a crash as a FAIL rather than as a kill, so a row here would be "
           "a row that cannot pass")

# The stated complement: POSITION -> (the header that position must still
# have, why no mutant pins it).  ⚠ Keyed on the position the ratchet itself
# uses, with the header as a GUARD -- an edit that shifts the ordinals makes
# the entry red rather than silently exempting whichever loop moved into it.
_SCOPE_EXEMPT = {
    ("Population.__init__", 3): ("for s in SCHEMAS:", _SCHEMAS_REASON),
    ("Population._declare", 1): ("for s in SCHEMAS:", _SCHEMAS_REASON),
    ("Population._unkeyed", 1): ("for s in SCHEMAS:", _SCHEMAS_REASON),
    # ⚠ The OUTER loop of the disposition, and until the fourth attestation it
    # was reported "pinned" only because its header reads like four other
    # loops'.  Truncating it is the same crash as truncating the inner one:
    # MEASURED -- 91 controls raise the `stream() before dispose()`
    # AssertionError and none goes red for a reason of its own.  Exempt by
    # construction, stated as such, rather than pinned by an alias.
    ("Population.__init__", 8): ("for memo in self.memos:", _RAISES),
    ("Population.__init__", 9): ("for lx in memo.lexed():", _RAISES),
    ("Population.__init__", 11): (
        "for row in self.declaring_rows():",
        "the same: truncating it raises a TypeError downstream (a row whose `field` was "
        "never set), so the scope is held by construction. MEASURED the same way"),
}


def _census_rows():
    """(name, find, replace) of every mutant row against the census module,
    read off the ONE registry (`plan_memo_selftest_mutants.mutants()`).

    ⚠ It globbed the registry's modules and imported them itself, a THIRD
    spelling of that population (the harness's hand list and the runner's
    import list were the others), and its imports filled the runner's
    registry as a side effect -- which is why deleting two of the runner's
    imports changed nothing (the fifth attestation).  A control whose
    population depends on what else was imported is not a control
    (`memory/feedback_derived-populations-shrink-in-silence.md`); it now asks
    the one step that builds the registry and holds no spelling of its own."""
    import plan_memo_selftest_mutants as mm
    return [(name, find, replace)
            for name, file, find, replace, _controls in mm.mutants() if file == CENSUS]


def population_scope_control(M):
    """PROPERTY: every loop in the census module that walks a POPULATION is
    pinned by a mutant that TRUNCATES THAT LOOP.

    ⚠ THE RATCHET WAS A HAND-WRITTEN LIST AND IT MISSED A LOOP IN EVERY ONE OF
    THREE CONSECUTIVE ROUNDS (PR #510 R45 / R47).  So the population here is
    every `for` statement of the module, read off the source the current
    module set was exec'd from, and a loop that is neither pinned nor in the
    stated complement `_SCOPE_EXEMPT` is red.  Both directions: an exempt
    entry naming no loop, and a truncating row crediting none, are red too.

    ⚠ A LOOP IS A POSITION (`_loops`), and a row's credit is computed by
    APPLYING it (`_credit`).  The first version matched a row's FIND text and
    credited a loop the row never touched; the second matched a target
    PREFIX, the third the exact header LINE -- each one step finer and each
    still an alias, because headers repeat.  `population_scope_partner_control`
    is the fixture that tells the three apart.

    ⚠⚠ WHAT IT DOES NOT MEASURE, stated because a ratchet's own reach is the
    first thing its reader will over-trust (PR #510 R51 audit):
      * the population is `for` statements in ONE file.  It misses the
        comprehensions and generator expressions and the `while queue:` walk
        of this module, and every loop in every other module.  Truncating
        `_phrases`' own comprehension CRASHES (held by construction), and
        truncating `data_rows`' is caught by controls (measured: the outer half turns 8 red, the inner 80), so there is no live
        hole by that route today;
      * `map` / `filter` / `next` / recursion are not loops to `ast`;
      * a row that removes a loop instead of truncating it credits nothing,
        which is why `population: every memo's rows are declared` is spelled
        as a truncation;
      * it asserts a truncating mutant EXISTS, not that it is the strongest
        one possible.
    A declared blind spot is a map of where the next finding lands
    (`memory/feedback_declared-blind-spots-are-where-the-next-finding-lands.md`).

    ⚠ STATIC, AND THAT IS WHAT MAKES IT AFFORDABLE.  It asserts that a
    truncating mutant EXISTS for each loop; the mutation proof separately
    asserts that every mutant's named control goes red.  The two together are
    the behavioural property at no extra runtime."""
    # ⚠ An `if not text: return False, "not in the swept population"` guard
    # stood here and is DELETED as dead (the sixth attestation, probed first):
    # the census module is in `files()` by the glob, and a run whose census
    # module is absent or empty never reaches a control -- `load()` execs it.
    text = dict(_swept_sources())[CENSUS]
    bad, n, n_pinned, n_exempt, n_unpinned = _scope_verdict(text, _census_rows(), _SCOPE_EXEMPT)
    return not bad, ("%d loop(s) = %d pinned by a truncating mutant + %d exempt with a reason "
                     "+ %d unpinned%s"
                     % (n, n_pinned, n_exempt, n_unpinned,
                        ("; " + "; ".join(bad[:5])) if bad else ""))


# Six loops, one per criterion `_loops` / `_credit` state: a module-level
# loop; in `f`, two with ONE header, a third sharing only its target prefix,
# and a loop NESTED in the third; and a loop in a METHOD, whose identity must
# carry its class.  Three rows: one truncates the first of the pair (its find
# text contains the second), one RE-POINTS the third to the truncation of a
# DIFFERENT iterable (the shape of a truncation, the population of none: an
# orphan), and one truncates the method's loop in the `list(...)[:1]` form.
_PARTNER_TEXT = ("for top in zs:\n    pass\n"
                 "def f(xs, ys):\n"
                 "    for x in xs:\n        pass\n"
                 "    for x in xs:\n        pass\n"
                 "    for x in ys:\n        for y in x:\n            pass\n"
                 "class C:\n    def g(self, xs):\n        for x in xs:\n            pass\n"
                 "async def k(xs):\n    async for x in xs:\n        pass\n"
                 "def h(zs):\n"
                 "    for a in zs:\n        pass\n"
                 "    for b in zs:\n        pass\n"
                 "    for c in zs:\n        pass\n"
                 "    for d in zs:\n        pass\n")
_PARTNER_LOOPS = {("<module>", 1), ("f", 1), ("f", 2), ("f", 3), ("f", 4), ("C.g", 1), ("k", 1),
                  ("h", 1), ("h", 2), ("h", 3), ("h", 4)}
_PARTNER_ROWS = [("truncate the first",
                  "    for x in xs:\n        pass\n    for x in xs:",
                  "    for x in xs[:1]:\n        pass\n    for x in xs:"),
                 ("re-point the third",
                  "    for x in ys:\n        for y",
                  "    for x in xs[:1]:\n        for y"),
                 ("truncate the method's",
                  "        for x in xs:\n            pass\n",
                  "        for x in list(xs)[:1]:\n            pass\n"),
                 # the criteria `_is_truncation` states, one row each: none of
                 # these is a truncation to the first element, so none credits
                 ("slice to TWO", "    for a in zs:", "    for a in zs[:2]:"),
                 ("slice with a LOWER bound", "    for b in zs:", "    for b in zs[1:1]:"),
                 ("slice with a STEP", "    for c in zs:", "    for c in zs[:1:2]:"),
                 ("truncate a CALL that is not `list`", "    for d in zs:",
                  "    for d in sorted(zs)[:1]:"),
                 # a find that is NOT unique applies to nothing
                 ("an ambiguous find", "    for x in xs:\n        pass\n",
                  "    for x in xs[:1]:\n        pass\n")]


def population_scope_partner_control(M):
    """PROPERTY (the loop ratchet's partner): one row truncating one of two
    loops that share a header credits THAT loop and no other -- and every
    other criterion the ratchet states, each with a fixture arm (the
    population is every `for` statement, nested and in methods; a re-point to
    ANOTHER iterable's `[:1]` credits nothing and is an orphan; the
    `list(...)[:1]` form is a truncation).

    A criterion keyed on the find text, on the target prefix or on the header
    line credits two or all three loops of `_PARTNER_TEXT` from one row, so
    the loops the row never touches read as pinned -- the defect three
    versions of `population_scope_control` had, each invisible over the real
    module because every loop there happened to be pinned or exempt anyway.
    The negative half: with the row removed, nothing is credited and every
    loop is unpinned, so the positive half is not passing vacuously.

    The complement is positional too, so its guard is asked here as well: an
    exempt entry whose position now holds a DIFFERENT header, and one whose
    position holds no loop, are both red -- the two ways an edit that shifts
    the ordinals would otherwise exempt the wrong loop in silence."""
    loops = set(_loops(_PARTNER_TEXT))
    credit, orphans = _credit(_PARTNER_TEXT, _PARTNER_ROWS)
    bad, n, n_pinned, _n_exempt, n_unpinned = _scope_verdict(_PARTNER_TEXT, _PARTNER_ROWS, {})
    empty, no_orphans = _credit(_PARTNER_TEXT, [])
    moved, _n, _p, _e, _u = _scope_verdict(_PARTNER_TEXT, _PARTNER_ROWS,
                                           {("f", 3): ("for x in xs:", "a moved position")})
    gone, _n, _p, _e, _u = _scope_verdict(_PARTNER_TEXT, _PARTNER_ROWS,
                                          {("f", 9): ("for x in xs:", "no such position")})
    guard = (any("is `for x in ys:`" in b for b in moved)
             and any("names no loop" in b for b in gone))
    want = {("f", 1): ["truncate the first"], ("C.g", 1): ["truncate the method's"]}
    want_orphans = ["re-point the third", "truncate a CALL that is not `list`", "an ambiguous find"]
    ok = (loops == _PARTNER_LOOPS and credit == want and orphans == want_orphans
          and n == 11 and n_pinned == 2 and n_unpinned == 9 and len(bad) == 12
          and not empty and not no_orphans and guard)
    return ok, ("population %s (want %s); credit %s (want %s); orphans %s; %d of %d pinned, %d "
                "unpinned, %d bad; no row credits %s; the exempt guard %s"
                % (sorted(loops), sorted(_PARTNER_LOOPS), sorted(credit), sorted(want), orphans,
                   n_pinned, n, n_unpinned, len(bad), sorted(empty),
                   "reports a moved and a vanished position" if guard else "is SILENT"))


# -- the kind-question ratchet -----------------------------------------------

# The sanctioned callers of each kind-phrase question: QUESTION -> {(module,
# qualified function): why it may ask directly}.  ⚠ The stated COMPLEMENT of a
# DERIVED population (every call in the checker's module set), not a list of
# the callers that exist: a new one is red until it is either routed through
# `_claims` or written here.  ⚠ QUALIFIED, not a bare function name (the fourth
# attestation): keyed on the name alone, a `def _kind` in `plan_memo_roles.py`
# or a `def assertion_a` in `plan_memo_memo.py` asked the question with the
# sanction written for a DIFFERENT function.
_KIND_QUESTION_SITES = {
    "_phrases": {
        (CENSUS, "Population._kind"):
            "decides WHICH kind a field DECLARES, which is a question about the rendering "
            "alone -- the doubt is `_kind_residue`'s to report",
        (CENSUS, "Population._claims"):
            "the contradiction question's own implementation: it is `_phrases` PLUS the "
            "disagreement arm",
    },
    "kind_disagreements": {
        (CENSUS, "Population._claims"):
            "the contradiction question, for every caller that asks whether a cell CLAIMS a kind",
        (CENSUS, "Population._kind_residue"):
            "the REPORTING site: it names which phrase the two readings disagree about, which "
            "is more than `_claims`' boolean",
    },
    "_claims": {
        (CENSUS, "Population._unkeyed"): "the blank-id contradiction",
        (CENSUS, "Population._unbound_claims"):
            "the unbound-table gate. ⚠ Written here first as `_lost_declarations`, a name from "
            "a second arm that was reverted with its code -- and this ratchet caught the stale "
            "spelling on its first run, which is the difference between a complement that is "
            "checked and one that is asserted",
        ("plan_memo_roles.py", "assertion_a"):
            "the marker-outside-the-declaring-field check, which asks the same question of a "
            "NON-declaring cell",
    },
}


def _kind_question_modules(here=None):
    """THE POPULATION: EVERY module of the harness's ONE population
    (`plan_memo_selftest_harness.files`), self-test modules included, and
    never written here.  `here` lets
    the partner ask it of a directory holding a module the real one does not.

    ⚠ IT WAS A HAND-WRITTEN 5-TUPLE (799349db, for PR #510 R52),
    under a comment that said "THE POPULATION IS THE WHOLE MODULE SET" while
    the set had 14 modules: a caller in `plan_memo_sibling.py` or in the entry
    point was outside it, the exact "caller in another module" class the tuple
    had been widened to close.  Widening a list is the enumerated-population
    shape again (`memory/feedback_enumerated-exemptions-leave-the-next-class-
    authoritative.md`); the population is now the set itself.
    ⚠ And "the set" was itself a hand list -- the harness's `MODULES` -- until
    the fifth attestation, so a NEW `plan_memo_extra.py` calling `pop._claims`
    was outside it; it is derived from the disk now.
    ⚠ And the self-test half was EXCLUDED "by design: they call the questions
    to test them" -- which, measured (the sixth attestation), no self-test
    module does: 0 calls.  An exclusion with no measured reason is a hole (a
    self-test-named helper calling `_claims` went unread), so it is gone."""
    return files(here)


def _qualified_callers(text):
    """(call name, qualified enclosing def) for every call in a module."""
    out = []

    def walk(node, qual):
        for ch in ast.iter_child_nodes(node):
            if isinstance(ch, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                walk(ch, qual + (ch.name,))
                continue
            if isinstance(ch, ast.Call):
                f = ch.func
                name = f.attr if isinstance(f, ast.Attribute) else getattr(f, "id", None)
                out.append((name, ".".join(qual) or "<module>"))
            walk(ch, qual)

    walk(ast.parse(text), ())
    return out


def _kind_question_verdict(sources, modules, sites):
    """(bad, n_calls): the kind-question ratchet's PURE core over any
    {file: text}, population and complement.  A call is sanctioned when its
    (module, qualified caller) is written under its question; a written
    site that makes no such call is red too, since a complement that names
    what no longer exists is how a stale sanction outlives its caller.

    ⚠ A "module absent from the swept population" guard stood here and is
    DELETED as dead code (the sixth pass): since the population is one
    derivation, `sources` and `modules` come from the same `files()` in the
    real control, and from the same temporary directory plus the one planted
    file in the partner -- probed both ways, and with a new file on disk, the
    difference is empty.  Only a caller that hand-builds an inconsistent pair
    could reach it, and that caller now gets a KeyError, which is louder."""
    bad, seen, n = [], set(), 0
    for mod in modules:
        for name, caller in _qualified_callers(sources[mod]):
            if name not in sites:
                continue
            n += 1
            if (mod, caller) in sites[name]:
                seen.add((name, mod, caller))
            else:
                bad.append("`%s` is called from `%s` in %s, which is not a sanctioned site "
                           "(route it through `_claims`, or state why it asks directly)"
                           % (name, caller, mod))
    for name, allowed in sorted(sites.items()):
        for mod, caller in sorted(allowed):
            if (name, mod, caller) not in seen:
                bad.append("the sanctioned site `%s` in %s makes no call to `%s` -- the stated "
                           "complement names a caller that is gone" % (caller, mod, name))
    return bad, n


def kind_question_site_control(M):
    """PROPERTY: each kind-phrase question is asked at its sanctioned sites only.

    ⚠ WRITTEN AFTER THE SAME DEFECT AT THREE SITES IN THREE ROUNDS (PR #510
    R47-2, R49-1).  "Does this cell claim a kind?" must read BOTH renderings,
    because a claim spelled across a masked span is what a reader sees and what
    the disposed stream does not.  `_kind_residue` did; `_unbound_claims` did
    not, and was taught to after a round reported it; the blank-id contradiction
    still did not, and was reported the round after THAT -- and R52 found a
    fourth, in `plan_memo_roles.py`.
    ⚠ So the question has ONE implementation (`Population._claims`), and this
    control makes any other caller visible.  THE POPULATION is every call, in
    every module of the harness's ONE population, self-test included
    (`_kind_question_modules`, derived from the disk), to any of the three names `_KIND_QUESTION_SITES`
    keys -- `_phrases`, `kind_disagreements`, `_claims`.  THE COMPLEMENT is
    that table, keyed on (module, qualified function): a caller not written
    there is red, and so is a written site that no longer calls.
    `kind_question_partner_control` feeds the same core the callers a
    narrower population or a name-only sanction would miss.

    ⚠⚠ IT MEASURES CALLS TO THREE NAMES, NOT THE QUESTION:
      * a caller that RE-DERIVES the question from `KIND_PHRASES` or from one
        of its member matchers directly, stream-only -- the exact R49-1
        defect -- calls none of the three names and leaves this green.  Such
        direct reads exist in the checker set today (AST loads of those names
        outside `Population`), so the gap is not hypothetical.  ⚠ NOT LIVE:
        when the R49-1 shape was re-injected the behavioural backstop fired
        (the R49-1 control went red), so the class is covered by a control
        even though it is not covered here;
      * a call made under ANOTHER name (an alias, `getattr`, a bound method
        passed as a value) is not a call to the name."""
    bad, n = _kind_question_verdict(dict(_swept_sources()), _kind_question_modules(),
                                    _KIND_QUESTION_SITES)
    return not bad, ("%d call site(s) over %d question(s) in %d module(s), %d unsanctioned%s"
                     % (n, len(_KIND_QUESTION_SITES), len(_kind_question_modules()), len(bad),
                        ("; " + "; ".join(bad[:4])) if bad else ""))


# Each probe adds ONE caller the ratchet must report, in the shape a
# narrower criterion lets through: a module outside a hand-written list, the
# entry point, and a function whose bare NAME is sanctioned for another module.
_PROBE = "\n\ndef %s(pop, lx):\n    return pop.%s(lx)\n"
_KIND_PROBES = (
    ("a caller in the sibling resolver", "plan_memo_sibling.py", _PROBE % ("_probe", "_claims")),
    ("a caller in the entry point", "plan-memo-umbrella-check.py", _PROBE % ("_probe", "_claims")),
    ("a `_kind` outside the census module", "plan_memo_roles.py", _PROBE % ("_kind", "_phrases")),
    ("an `assertion_a` outside the roles module", "plan_memo_memo.py",
     _PROBE % ("assertion_a", "_claims")),
    ("an unsanctioned name in the roles module", "plan_memo_roles.py",
     _PROBE % ("_probe", "_phrases")),
)


_EXTRA = "plan_memo_extra.py"
_EXTRA_SELFTEST = "plan_memo_selftestx_helper.py"


def kind_question_partner_control(M):
    """PROPERTY (the kind-question ratchet's partner): each probe caller is
    reported, and the unprobed module set is clean -- including a caller in a
    module that EXISTS ON DISK ONLY: a directory holding every current file
    plus `plan_memo_extra.py` must yield a population that contains it, and
    its `pop._claims(lx)` must be reported (a hand-written population, however
    complete today, misses it).

    A population narrower than the module set misses the first two probes; a
    sanction keyed on the bare function name misses the next two -- the two
    criteria this ratchet had, each green over the real source because no
    such caller existed yet.  The fifth is the shape both criteria DID catch,
    so a probe that stops reporting it is a broken probe and not a fix.  And
    the complement's other direction: a sanctioned site that makes no call is
    red, or a sanction outlives the caller it was written for."""
    sources = dict(_swept_sources())
    modules = _kind_question_modules()
    base, _n = _kind_question_verdict(sources, modules, _KIND_QUESTION_SITES)
    stale = dict(_KIND_QUESTION_SITES)
    stale["_claims"] = {**stale["_claims"], (CENSUS, "Population._gone"): "no such caller"}
    gone, _n = _kind_question_verdict(sources, modules, stale)
    gone = any("`Population._gone`" in b and "makes no call" in b for b in gone)
    quiet = []
    for label, file, tail in _KIND_PROBES:
        probed = dict(sources)
        probed[file] = sources.get(file, "") + tail
        bad, _n = _kind_question_verdict(probed, modules, _KIND_QUESTION_SITES)
        if not any(("in %s," % file) in b for b in bad):
            quiet.append(label)
    import pathlib
    import tempfile
    extra = _PROBE % ("helper", "_claims")
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        for f in files():
            (d / f).write_text("", encoding="utf-8")
        (d / _EXTRA).write_text(extra, encoding="utf-8")
        (d / _EXTRA_SELFTEST).write_text(extra, encoding="utf-8")
        grown = _kind_question_modules(d)
    bad, _n = _kind_question_verdict(dict(sources, **{_EXTRA: extra, _EXTRA_SELFTEST: extra}),
                                     grown, _KIND_QUESTION_SITES)
    if not any(("in %s," % _EXTRA) in b for b in bad):
        quiet.append("a caller in a module that exists on disk only")
    if not any(("in %s," % _EXTRA_SELFTEST) in b for b in bad):
        quiet.append("a caller in a SELF-TEST-named module")
    return not base and not quiet and gone, (
        "unprobed: %d unsanctioned; %d of %d probe(s) reported%s; a stale sanction is %s"
        % (len(base), len(_KIND_PROBES) + 2 - len(quiet), len(_KIND_PROBES) + 2,
           ("; NOT reported: " + ", ".join(quiet)) if quiet else "",
           "reported" if gone else "SILENT"))


def criteria_rows_control(M):
    """PROPERTY: every criterion the two ratchets state names an existing
    killing row -- the table is DATA (`plan_memo_selftest_mutants_ratchets.
    CRITERIA`), because a comment that said "the criteria are enumerated in the
    commit" pointed at an enumeration that did not exist (the sixth
    attestation)."""
    import importlib
    table = importlib.import_module("plan_memo_selftest_mutants_ratchets").CRITERIA
    names = [r[0] for r in importlib.import_module("plan_memo_selftest_mutants").mutants()]
    missing = _criteria_missing(table, names)
    planted = _criteria_missing([("planted", "no row is named like this")], names)
    return bool(table) and not missing and planted == ["planted"], (
        "%d criteri(a), %d without a row%s; a planted criterion is %s"
        % (len(table), len(missing), ("; " + "; ".join(missing[:3])) if missing else "",
           "reported" if planted == ["planted"] else "SILENT"))


def _criteria_missing(table, names):
    return [c for c, row in table if not any(n.startswith(row) for n in names)]


def registry():
    """name -> (kind, control) for the ratchets -- this module's fragment of
    the one registry, merged by `plan_memo_selftest_controls.registry()`."""
    return {
        "PROPERTY: every loop in the census module that walks a POPULATION is pinned by a mutant that truncates THAT loop (the ratchet is derived from the code, not a list somebody extends when a reviewer names one)":
            ("CONTROL", population_scope_control),
        "PROPERTY: the loop ratchet credits a loop by POSITION: one row truncating one of two loops that share a header pins that loop and no other":
            ("CONTROL", population_scope_partner_control),
        "PROPERTY: every criterion the two ratchets state names an existing killing row (the table is data in plan_memo_selftest_mutants_ratchets.CRITERIA)":
            ("CONTROL", criteria_rows_control),
        "PROPERTY: the kind-phrase questions are asked at their sanctioned sites only -- \"which kind does this field DECLARE\" and \"does this cell CLAIM one, under either reading\" are two questions with one site each, and a third caller asking either one directly is the shape three rounds of findings had":
            ("CONTROL", kind_question_site_control),
        "PROPERTY: the kind-question ratchet reports a caller in ANY module of the checker set, judged by (module, qualified function) -- not a caller outside a listed subset, not one whose bare name is sanctioned elsewhere":
            ("CONTROL", kind_question_partner_control),
    }
