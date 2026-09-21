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
asked at its one sanctioned site.  They are derived from the census module's
AST, each carries a STATED COMPLEMENT (`_SCOPE_EXEMPT`, `_KIND_QUESTION_SITES`)
rather than a list of what exists, and they are the family that grows as more
classes get a ratchet -- which is exactly why they should not grow inside a
module whose subject is something else.

Both exist because the same defect arrived three or four rounds running, each
time fixed at the site a reviewer had just named.  A ratchet is what turns
"fix the instance" into "the class is covered or the suite is red".

`ratchet_registry()` is merged by `plan_memo_selftest_controls.registry()`, the
same way `property_registry()` is, so nothing here is referenced by the module
it came from and the import goes one way.
"""

from plan_memo_selftest_properties import HERE, _swept_sources

_SCOPE_EXEMPT = {
    "for s in SCHEMAS:":
        "SCHEMAS is a module constant of the GRAMMAR, not a population this walk "
        "collects; its own completeness is `schema_*` controls' subject",
    "for lx in memo.lexed():":
        "truncating it RAISES -- `stream() before dispose()` -- so the scope is held by "
        "the program's own assertion and not by a control. MEASURED, not assumed: the "
        "truncation was run and the suite reports an AssertionError, and the mutant "
        "runner counts a crash as a FAIL rather than as a kill, so a row here would be "
        "a row that cannot pass",
    "for row in self.declaring_rows():":
        "the same: truncating it raises a TypeError downstream (a row whose `field` was "
        "never set), so the scope is held by construction. MEASURED the same way",
}


def _census_loops(text):
    """Every `ast.For` in the census module, as (lineno, stripped source line)
    -- the DERIVED population of the scope ratchet."""
    import ast
    lines = text.split("\n")
    return [(n.lineno, lines[n.lineno - 1].strip())
            for n in ast.walk(ast.parse(text)) if isinstance(n, ast.For)]


def _truncating_mutants():
    """Every mutant row against the census module whose REPLACEMENT truncates a
    loop, as the set of `for <target> in ` prefixes it truncates.

    Read off the registry rather than off a list, and keyed on the REPLACEMENT
    (what the mutant does) rather than on the find (where it is), because a
    mutant whose find happens to CONTAIN a loop line proves nothing about that
    loop -- which is exactly how the first version of this ratchet reported a
    loop as pinned while its truncation survived."""
    import plan_memo_selftest_mutants as mm
    # ⚠ The four review-round modules APPEND to `mm.MUTANTS`, and a plain
    # `--self-test` never imports them -- so reading the base module alone gave
    # an EMPTY registry and every loop read as unpinned.  A control whose
    # population depends on whether something else happened to be imported is
    # not a control (`memory/feedback_derived-populations-shrink-in-silence.md`),
    # so the appenders are imported here, by the same glob that finds them.
    import importlib
    for name in sorted(q.stem for q in HERE.glob("plan_memo_selftest_mutants*.py")):
        importlib.import_module(name)
    out = set()
    for _name, file, _find, replace, _controls in mm.MUTANTS:
        if file != "plan_memo_population.py":
            continue
        for line in replace.split("\n"):
            line = line.strip()
            if line.startswith("for ") and " in " in line and "[:1]" in line:
                out.add(line.split(" in ")[0] + " in ")
    return out


def population_scope_control(M):
    """PROPERTY: every loop in the census module that walks a POPULATION is
    pinned by a mutant that TRUNCATES THAT LOOP.

    ⚠ THE RATCHET WAS A HAND-WRITTEN LIST AND IT MISSED A LOOP IN EVERY ONE OF
    THREE CONSECUTIVE ROUNDS (PR #510 R45 / R47).  R45 found `_unkeyed` and the
    table-miss loop unpinned; R47 pinned the OUTER loop of `_unbound_claims`
    and left its two inner ones; an independent AST inventory then found more.
    Each time the response was to add the row the reviewer named, which is the
    shape this document's own root analysis already committed against in
    writing: **a population is DERIVED, not listed**.  So the population here
    is every `ast.For` in the module, read off the source the current module
    set was exec'd from, and a loop that is neither pinned nor in the stated
    complement `_SCOPE_EXEMPT` is red.

    ⚠ STATIC, AND THAT IS WHAT MAKES IT AFFORDABLE.  It asserts that a
    truncating mutant EXISTS for each loop; the mutation proof separately
    asserts that every mutant's named control goes red.  The two together are
    the behavioural property -- "truncating this loop is noticed" -- at no
    extra runtime, where measuring it directly costs ~6.4 s per loop.

    ⚠ KEYED ON THE REPLACEMENT, not on the find.  A first version matched a
    mutant whose FIND text contained the loop line, and reported
    `for t in memo.tables:` as pinned by a row that truncates the enclosing
    `self.memos` loop and never touches it
    (`memory/feedback_gate-criterion-by-behaviour-not-by-name.md`).

    Both directions, like the module map: a truncating mutant naming a loop the
    module no longer has is also red, since a re-point that orphans a row
    leaves the ratchet looking full."""
    text = dict(_swept_sources()).get("plan_memo_population.py", "")
    if not text:
        return False, "the census module is not in the swept population"
    loops = _census_loops(text)
    pinned = _truncating_mutants()
    unpinned = [(ln, src) for ln, src in loops
                if src not in _SCOPE_EXEMPT
                and not any(src.startswith(pre) for pre in pinned)]
    orphan = [pre for pre in pinned
              if not any(src.startswith(pre) for _ln, src in loops)]
    bad = (["%d `%s` is neither pinned nor in the stated complement" % (ln, src)
            for ln, src in unpinned]
           + ["a truncating mutant names `%s`, which the module no longer has" % pre
              for pre in orphan])
    # ⚠ DISJOINT, because the first version's three numbers summed to 21 over 20
    # loops -- a loop that is exempt AND pinned was counted twice, and a report
    # whose own arithmetic does not add up invites the reader to trust the
    # wrong one.  Exempt is decided first, so each loop lands in exactly one.
    n_exempt = sum(1 for _ln, src in loops if src in _SCOPE_EXEMPT)
    n_pinned = sum(1 for _ln, src in loops
                   if src not in _SCOPE_EXEMPT and any(src.startswith(p) for p in pinned))
    assert n_exempt + n_pinned + len(unpinned) == len(loops), "the three buckets must partition"
    return not bad, ("%d loop(s) = %d pinned by a truncating mutant + %d exempt with a reason "
                     "+ %d unpinned%s"
                     % (len(loops), n_pinned, n_exempt, len(unpinned),
                        ("; " + "; ".join(bad[:5])) if bad else ""))


# The sanctioned callers of each kind-phrase question, with the reason each is
# allowed to ask it directly.  ⚠ The stated COMPLEMENT of a DERIVED population
# (every call in the census module), not a list of the callers that exist:
# a new one is red until it is either routed through `_claims` or written here.
_KIND_QUESTION_SITES = {
    "_phrases": {
        "_kind": "decides WHICH kind a field DECLARES, which is a question about the "
                 "rendering alone -- the doubt is `_kind_residue`'s to report",
        "_claims": "the contradiction question's own implementation: it is `_phrases` PLUS "
                   "the disagreement arm",
    },
    "kind_disagreements": {
        "_claims": "the contradiction question, for every caller that asks whether a cell "
                   "CLAIMS a kind",
        "_kind_residue": "the REPORTING site: it names which phrase the two readings "
                         "disagree about, which is more than `_claims`' boolean",
    },
}


def kind_question_site_control(M):
    """PROPERTY: each kind-phrase question is asked at its sanctioned sites only.

    ⚠ WRITTEN AFTER THE SAME DEFECT AT THREE SITES IN THREE ROUNDS (PR #510
    R47-2, R49-1).  "Does this cell claim a kind?" must read BOTH renderings,
    because a claim spelled across a masked span is what a reader sees and what
    the disposed stream does not.  `_kind_residue` did; `_unbound_claims` did
    not, and was taught to after a round reported it; the blank-id contradiction
    still did not, and was reported the round after THAT -- each fix a patch at
    the site that had just been named.
    ⚠ So the question has ONE implementation (`Population._claims`) and this
    control is what stops a fourth caller from asking it a fourth way: the
    population is every call to either name in the census module, read off the
    AST, and a caller outside the stated complement is red.  It is the
    `population_scope_control` idea applied to a QUESTION rather than to a
    loop -- and, like that one, it is derived rather than listed."""
    import ast
    text = dict(_swept_sources()).get("plan_memo_population.py", "")
    if not text:
        return False, "the census module is not in the swept population"
    tree = ast.parse(text)
    fn_of, bad, seen = {}, [], 0
    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef):
            for sub in ast.walk(node):
                fn_of[id(sub)] = node.name
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        f = node.func
        name = f.attr if isinstance(f, ast.Attribute) else getattr(f, "id", None)
        if name not in _KIND_QUESTION_SITES:
            continue
        seen += 1
        caller = fn_of.get(id(node))
        if caller not in _KIND_QUESTION_SITES[name]:
            bad.append("`%s` is called from `%s`, which is not a sanctioned site "
                       "(route it through `_claims`, or state why it asks directly)"
                       % (name, caller))
    if not seen:
        return False, ("no call to a kind-phrase question was found: the names this control "
                       "reads are no longer the ones the module writes")
    return not bad, ("%d call site(s) over %d question(s), %d unsanctioned%s"
                     % (seen, len(_KIND_QUESTION_SITES), len(bad),
                        ("; " + "; ".join(bad[:4])) if bad else ""))


def ratchet_registry():
    """name -> (kind, control) for the ratchets -- this module's fragment of
    the one registry, merged by `plan_memo_selftest_controls.registry()`."""
    return {
        "PROPERTY: every loop in the census module that walks a POPULATION is pinned by a mutant that truncates THAT loop (the ratchet is derived from the code, not a list somebody extends when a reviewer names one)":
            ("CONTROL", population_scope_control),
        "PROPERTY: the kind-phrase questions are asked at their sanctioned sites only -- \"which kind does this field DECLARE\" and \"does this cell CLAIM one, under either reading\" are two questions with one site each, and a third caller asking either one directly is the shape three rounds of findings had":
            ("CONTROL", kind_question_site_control),
    }
