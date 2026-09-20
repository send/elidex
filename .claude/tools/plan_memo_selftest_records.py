#!/usr/bin/env python3
"""The WRITTEN-RECORD property controls for `plan-memo-umbrella-check.py
--self-test`: every control whose subject is something a person WROTE ABOUT
this module set -- a map in a docstring, an attribution `mod.sym`, an "only
importer of X" -- checked against what the module set actually IS.

THE SEAM, and why it is not a line count.  `plan_memo_selftest_properties.py`
asks questions the code answers about ITSELF: is this character class spelled
once, does this pattern begin with an unbounded repeat, does this `open()` name
an encoding.  Nobody wrote those down; they are read off the AST and compared
with nothing.  The controls HERE each have a second subject -- a sentence -- and
what they compare is the sentence against the tree.  That is why they share a
failure mode the other half does not have: every one of them was written
BECAUSE a touch-time split moved a file and left a true sentence false, and
every one of them will go stale again the next time one does.  The population
of sentences is the thing this module owns.

⚠ THE SEAM THE HANDOFF NAMED WAS "cross-file consistency vs the checker as
written", and it does not partition: `id_spelling_sweep_control` sweeps the
whole tree for a consistency claim and stays, while
`dash_spelling_sweep_control` -- whose own docstring calls itself "the
`id_spelling_sweep_control` shape applied to the other character class" --
would have moved away from its stated sibling.  A seam that separates a pair
the source itself calls a pair is a line count wearing a cohesion label.  The
seam above keeps both spelling sweeps together and takes the map pair with the
attribution sweep, which `symbol_attribution_control`'s own docstring already
names as its other half ("the map pair next to it carries both directions for
exactly this reason").

`registry()` returns this module's fragment MERGED over the property module's
(which is itself merged over the invariants module's), so the runner and the
mutation proof still read ONE table; `plan_memo_selftest_controls.registry()`
reads this one.  A mutant row whose file is THIS module patches the self-test,
not the checker set (`plan_memo_selftest_mutants.SELFTEST`).

Import direction, one way: the controls module imports this one; this one
imports the property module (for the shared population `_swept_sources` and the
entry point's file name) and the harness, and nothing of the controls.
"""

import ast
import io
import re
import tokenize

import pathlib
import tempfile

from plan_memo_selftest_cases import build
from plan_memo_selftest_harness import HERE
from plan_memo_selftest_properties import ENTRY, _swept_sources, registry as property_registry

# The module map lives in the entry point's docstring, between these two
# headings.  A name is spelled either in full or relative to the `plan_memo`
# prefix (`_selftest_controls.py`), which is the one normalisation below.
_MAP_START, _MAP_END = "\nMODULES\n", "\nWHERE THIS RUNS"
_MAP_NAME = re.compile(r"[A-Za-z0-9_.-]+\.py(?![A-Za-z0-9_])")


def _mapped_modules():
    """The module names the entry point's MODULES map spells, normalised to
    file names -> (set, "") or (None, why)."""
    src = dict(_swept_sources())[ENTRY]
    doc = ast.get_docstring(ast.parse(src, filename=ENTRY)) or ""
    doc = "\n" + doc
    a, b = doc.find(_MAP_START), doc.find(_MAP_END)
    if a < 0 or b < a:
        return None, "the docstring has no %r ... %r section" % (_MAP_START.strip(), _MAP_END.strip())
    return {n if not n.startswith("_") else "plan_memo" + n
            for n in _MAP_NAME.findall(doc[a:b])}, ""


def _map_population():
    """(mapped names, the files on disk the map is required to name).  The
    entry point is excluded from the requirement: the map names it `(this
    file)`, deliberately, and a map that had to name itself would be the one
    line nobody can get wrong."""
    return sorted(f for f, _ in _swept_sources() if f != ENTRY)


def module_map_completeness_control(M):
    """PROPERTY: every module of this checker is named in the entry point's
    MODULES map.  The population is `_swept_sources()` minus the entry point --
    globbed, not listed -- and a file the map does not name is red.

    WHY A CONTROL AND NOT A WARNING (PR #510 R28-2).  The map says what each
    module OWNS, which `ls` cannot, so it earns its place; but it drifted three
    times, and the response the second time was to add the three missing names
    AND A PROSE WARNING saying the map drifts.  One round later it drifted
    again -- `plan_memo_selftest_growth.py`, carved by R27's split -- exactly as
    the warning predicted and did not prevent.  A sentence asking the author to
    be careful is not a mechanism (`memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md`);
    a checker over the artefact is.  The warning is now a pointer to this
    control rather than a request.

    HONESTLY, what it cannot catch.  It checks that the map NAMES every module,
    not that what it says about one is TRUE: a description that has gone stale,
    or is attached to the wrong module, reads exactly like a fresh one here.
    It is also blind to a module outside the `plan_memo*.py` glob (a helper
    named otherwise would be swept by nothing in this file), and it takes the
    map's own `(this file)` for the entry point on trust."""
    mapped, why = _mapped_modules()
    if mapped is None:
        return False, why
    want = _map_population()
    missing = [f for f in want if f not in mapped]
    return not missing, ("%d module(s) on disk against %d name(s) in the map, %d unnamed%s"
                         % (len(want), len(mapped), len(missing),
                            (": " + ", ".join(missing)) if missing else ""))


def module_map_existence_control(M):
    """PROPERTY, the OTHER direction of the map: every name the MODULES map
    spells is a file that exists.  The completeness half cannot see a name left
    behind by a rename or a deletion -- the map would still cover every file on
    disk and simply describe one that is gone -- so the two directions are two
    controls, and each has its own mutant.

    HONESTLY: a name is anything ending in `.py` inside the section, so a name
    that appears there for another reason (a shell command quoting a glob, say)
    would be read as a claim about a file.  That is the reason the section
    holds no such text; if it ever needs to, this control is where the
    exception has to be written down rather than assumed."""
    mapped, why = _mapped_modules()
    if mapped is None:
        return False, why
    have = set(_map_population()) | {ENTRY}
    dangling = sorted(n for n in mapped if n not in have)
    return not dangling, ("%d name(s) in the map against %d file(s), %d naming nothing on disk%s"
                          % (len(mapped), len(have), len(dangling),
                             (": " + ", ".join(dangling)) if dangling else ""))


# The three spellings this corpus uses to attribute a symbol to a module.
# READ OFF THE CORPUS, not guessed: `mod.sym` is the sources' form, `mod.py::sym`
# the plan's §3 coverage-map form (which carried 12 of the 28 sites the first
# run found), and ``sym` in `mod.py`` a prose form the other two miss.  A fourth
# spelling would be invisible here, which is why the control REPORTS its
# denominator -- a sweep that silently matched nothing would read as clean.
_ATTRIB_SPELLINGS = (
    # ⚠ `(?!py\b)` -- `` `plan_memo_blocks.py` `` is a MODULE MENTION, not an
    # attribution, and this pattern was reading it as the symbol `py`.  It cost
    # nothing (no module defines `py`, so every one landed in the skip arm) but
    # it inflated the skip arm to 30 of its 38 entries and hid the eight that
    # matter there (PR #510 R38 re-gate).
    (re.compile(r"`(plan_memo_[a-z_0-9]+)\.(?!py\b)([A-Za-z_]\w*)"), 1, 2),
    (re.compile(r"`(plan_memo_[a-z_0-9]+|plan-memo-umbrella-check)\.py::([A-Za-z_]\w*)"), 1, 2),
    (re.compile(r"`([A-Za-z_]\w*)` in `(plan_memo_[a-z_0-9]+)\.py`"), 2, 1),
)


def _defining_module():
    """{top-level name: {module names that define it}} over the checker set,
    from the AST -- the authority a written attribution is checked against."""
    home = {}

    def names(t):
        if isinstance(t, ast.Name):
            yield t.id
        elif isinstance(t, (ast.Tuple, ast.List)):
            for e in t.elts:
                yield from names(e)

    for file, src in _swept_sources():
        mod = "plan_memo_umbrella_check" if file == ENTRY else file[:-3]
        for node in ast.parse(src).body:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                home.setdefault(node.name, set()).add(mod)
            elif isinstance(node, ast.Assign):
                for target in node.targets:
                    for nm in names(target):
                        home.setdefault(nm, set()).add(mod)
            elif isinstance(node, ast.AnnAssign):
                for nm in names(node.target):
                    home.setdefault(nm, set()).add(mod)
    return home


def symbol_attribution_control(M):
    """PROPERTY: every written `module.symbol` attribution in this checker
    names the module that actually DEFINES that symbol.

    THE CLASS HAD NO DETECTOR AND FIVE SPLITS HAD RUN (PR #510 R32).  Each
    touch-time split moves symbols between modules, and every docstring,
    comment and plan row that attributed one by name went stale silently: the
    stream reader attributed to the tables module after it was carved out, the
    file-and-citation scan still attributed to the lexer after the token
    grammar left it, and a quote-cost control still attributed to the work
    module after THIS round's own carve.  Nothing read them, so nothing was
    red.  First run: 31 sites over 100 files.

    ⚠ THE EXAMPLES ABOVE NAME NO MODULE, AND THAT IS THIS CONTROL'S ONE REAL
    LIMIT.  It cannot tell a present-tense attribution from a historical one --
    "`x.y` was wrong" reads exactly like "`x.y`" -- so prose ABOUT a stale
    attribution would be a finding against itself.  The first run proved it:
    three of the thirty-one were in this very docstring.  Narrative about a
    move therefore carries no locator; a POINTER a reader follows must carry a
    correct one, which is why the historical note at
    `plan_memo_tokens.covers` was corrected rather than reworded.

    ⚠ THE POPULATION IS THE PROPERTY, NOT THE SYMPTOM.  A sweep for the
    symbols a reader happens to know moved found 8; a sweep for the shape
    `module.symbol` found 15; asking the question of every spelling the corpus
    actually uses found 28 -- and the spelling that contributed most (the
    plan's `mod.py::sym` coverage-map form) is the one no symbol-name grep
    reaches.  `memory/feedback_checks-must-not-be-defined-by-the-symptom-
    vocabulary.md` is the rule; this is the measurement behind it.

    HONESTLY, what it cannot see: an attribution spelled a fourth way; a
    symbol named with no module beside it (the overwhelming majority, and the
    reason this is not a general staleness check); a module named with no
    symbol; and a name this checker does not define at top level (a method, an
    attribute), which is skipped rather than guessed at.  The DENOMINATOR is
    reported for exactly that reason -- a regex that stopped matching would
    otherwise read as a clean sweep."""
    home = _defining_module()
    corpus = _attribution_corpus()
    bad, checked = [], 0
    for name, src in corpus:
        for lineno, line in _prose_of(name, src):
            for pattern, mod_group, sym_group in _ATTRIB_SPELLINGS:
                for m in pattern.finditer(line):
                    # ⚠ A DATED LOCATOR IS A STATEMENT ABOUT THE PAST (PR #510
                    # R38).  This corpus marks one with the §3 Touch column's
                    # own convention -- "⚠ pre-split name `X`", "⚠ pre-R14 name
                    # `X`" -- and such a name is CORRECT precisely by naming the
                    # module the symbol has LEFT.  Read as a present-tense
                    # claim it is a violation, and the mechanical sweep this
                    # control's first run drove REWROTE TWO OF THEM into
                    # falsehoods ("pre-split name" then naming the post-split
                    # module, which is vacuous as well as false).  ⚠ The R32
                    # record said one replacement was "refused rather than
                    # applied, the guard doing its job"; two others were applied
                    # and the guard cannot see the difference, because a count
                    # is not a reading.  Recognising the marker is reading the
                    # corpus's stated convention, not exempting a case.
                    if _DATED_LOCATOR.search(line[:m.start()]):
                        continue
                    mod = m.group(mod_group).replace("plan-memo-umbrella-check", "plan_memo_umbrella_check")
                    sym = m.group(sym_group)
                    if sym not in home:
                        continue
                    checked += 1
                    if mod not in home[sym]:
                        bad.append("%s:%d `%s.%s` -> %s"
                                   % (name, lineno, mod, sym, "/".join(sorted(home[sym]))))
    if not checked:
        return False, ("no `module.symbol` attribution matched in %d file(s): the spellings this "
                       "control reads are no longer the ones the corpus writes" % len(corpus))
    return not bad, ("%d attribution(s) checked over %d file(s), %d naming the wrong module%s"
                     % (checked, len(corpus), len(bad), ("; " + "; ".join(bad[:6])) if bad else ""))


# The corpus's own marker for a DATED locator -- a name introduced as the one a
# symbol had at some earlier point ("pre-split name", "pre-R14 name").  Such an
# attribution is correct by naming the module the symbol has left.
_DATED_LOCATOR = re.compile(r"pre-[\w.]+ name\s*$")


def _prose_of(name, src):
    """The PROSE of one file: comments and docstrings for a `.py`, the whole
    text for a `.md`.  Returned as [(lineno, text)].

    WHY PROSE AND NOT THE WHOLE SOURCE (PR #510 R32).  An attribution a reader
    FOLLOWS lives in a comment or a docstring.  A module name inside a string
    literal is DATA -- a mutant's replacement payload, a fixture, a regex -- and
    a mutant row that injects a stale attribution must spell that stale
    attribution to inject it.  Swept as source text, the mutation proof's own
    payload was reported as a defect the moment the row was written.  Excluding
    the mutants file by name would be the enumerated exemption this suite keeps
    being bitten by (`memory/feedback_enumerated-exemptions-leave-the-next-class-
    authoritative.md`); "a literal is data, prose is a claim" is a rule about
    what the check is FOR, and it holds for the next file too."""
    if name.endswith(".md"):
        return list(enumerate(src.split("\n"), 1))
    out = []
    try:
        for tok in tokenize.generate_tokens(io.StringIO(src).readline):
            if tok.type == tokenize.COMMENT:
                out.append((tok.start[0], tok.string))
    except (tokenize.TokenError, IndentationError, SyntaxError):
        return out
    for node in ast.walk(ast.parse(src)):
        if isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            doc = ast.get_docstring(node, clean=False)
            if doc:
                base = node.body[0].lineno if node.body else 1
                for k, line in enumerate(doc.split("\n")):
                    out.append((base + k, line))
    return out


def _attribution_corpus():
    """The files whose attributions are checked: every source of the checker
    set, plus the plan memos beside it when this tool is sitting in its own
    repository (`<root>/docs/plans/*.md`).  The plan's §3 coverage map is the
    single densest population of these attributions, so leaving it out would
    put the majority of the class outside the control; taking it only when the
    directory exists keeps the tool runnable from anywhere, which its header
    promises."""
    files = list(_swept_sources())
    plans = HERE.parents[1] / "docs" / "plans"
    if plans.is_dir():
        files += [(p.name, p.read_text(encoding="utf-8")) for p in sorted(plans.glob("*.md"))]
    return files


# The import-graph seams this suite states in prose, as DATA: name -> (the
# thing imported, the modules allowed to import it).  Every entry is a sentence
# some module's docstring asserts; the control below is what makes the sentence
# fail red instead of reading true.
#
# ⚠ THREE OF THESE WERE FALSE WHEN THIS TABLE WAS FIRST WRITTEN (PR #510 R32),
# and they had been false for rounds: `ast` and the harness's module-set
# handles were claimed exclusive to this module while the growth module had
# imported them since R27, and `build` / `run_on` were claimed exclusive to the
# invariants module while SEVEN modules import `build`.  Each is a universal
# claim ("the ONLY importer of X") offered to a reader IN PLACE of their own
# judgement about where the next control goes, so a false one does not merely
# mislead -- it routes work to the wrong file.
# Each entry: label -> (names, allowed importers, POPULATION the claim ranges
# over -- None for "every module of this checker").
#
# ⚠ THE POPULATION IS PART OF THE CLAIM, and leaving it out makes the seam
# meaningless rather than merely loose.  Written first without one, the
# `tempfile` row had to list every module that touches `tempfile` for any
# reason, which is not the sentence the docstring makes: that sentence ranges
# over the WORK modules and says which of those two writes a file.  A seam
# whose allow-list is "everyone who does it" asserts nothing.
_WORK = {"plan_memo_selftest_work.py", "plan_memo_selftest_pipeline.py"}
_IMPORT_SEAMS = {
    "ast": ("ast", {"plan_memo_selftest_properties.py", "plan_memo_selftest_growth.py",
                    "plan_memo_selftest_records.py"}, None),
    "the harness's module-set handles": (
        ("MODULES", "SOURCES", "GRAMMAR", "HERE"),
        {"plan_memo_selftest_properties.py", "plan_memo_selftest_growth.py",
         "plan_memo_selftest_records.py"}, None),
    "the fixture runner": (("run_on",),
                           {"plan_memo_selftest_invariants.py", "plan_memo_selftest_controls.py"}, None),
    "the work witnesses": (
        ("_count_calls", "_count_lines", "_count_line_sites", "_CountedList", "_count_pattern_spans"),
        {"plan_memo_selftest_work.py", "plan_memo_selftest_pipeline.py",
         "plan_memo_selftest_growth.py"}, None),
    "tempfile, among the two written-shape work modules": (
        "tempfile", {"plan_memo_selftest_pipeline.py"}, _WORK),
}


def import_seam_control(M):
    """PROPERTY: every import-graph seam this suite states in prose is true of
    the imports.

    WHY A CONTROL AND NOT A CAREFUL READER (PR #510 R32).  Several module
    docstrings offer the import graph as a MECHANICAL answer to "is this a work
    control?" / "where does the next control go?" -- "the ONLY importer of
    `ast`", "the only importer of `build` / `run_on`", "the only importers of
    the work witnesses".  A reader is told to trust the graph rather than their
    judgement, so the sentences are load-bearing; and three of them were false
    when this control was written, one of them by six modules.  They went false
    the way every claim here goes false: a touch-time split carved a module out,
    the new module imported what it needed, and no one re-read the sentence that
    said nobody else could.  `memory/feedback_universal-claims-need-the-
    complement-measured.md` is the rule -- an "only" is a claim about the
    COMPLEMENT, and the complement is what nobody measures.

    The table is the permitted set, so a NEW importer is red and must either be
    added (the seam moved, deliberately) or removed (the seam held and the
    import was a mistake).  Both directions are reported: a name no module
    imports at all is red too, because a seam nobody can violate is a sentence
    about nothing.

    ⚠ EACH SEAM CARRIES THE POPULATION IT RANGES OVER, because without one the
    `tempfile` row degenerated into "allowed: everyone who imports it" -- a row
    that can never be red.  The claim its docstring makes is about the two WORK
    modules and which of them writes a file to disk, so that is the population.

    HONESTLY, what it cannot see: a module reaching a name WITHOUT importing it
    (`__import__`, an attribute off an already-imported module) -- the mutation
    proof does exactly that on purpose, which is why the checker set is not in
    the table; and prose that states a seam this table does not list, which is
    the same open edge every enumerated table in this file has."""
    seen = {}
    for file, src in _swept_sources():
        for node in ast.walk(ast.parse(src)):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    seen.setdefault(alias.name.split(".")[0], set()).add(file)
            elif isinstance(node, ast.ImportFrom):
                for alias in node.names:
                    seen.setdefault(alias.name, set()).add(file)
    bad = []
    for label, (names, allowed, population) in sorted(_IMPORT_SEAMS.items()):
        names = (names,) if isinstance(names, str) else names
        importers = set()
        for name in names:
            importers |= seen.get(name, set())
        if population is not None:
            importers &= population
        if not importers:
            bad.append("%s: nothing imports %s -- the seam is a sentence about nothing"
                       % (label, "/".join(names)))
            continue
        extra = sorted(importers - allowed)
        if extra:
            bad.append("%s: also imported by %s" % (label, ", ".join(extra)))
    return not bad, ("%d import seam(s) hold%s" % (len(_IMPORT_SEAMS),
                                                   ("; " + "; ".join(bad)) if bad else ""))


_REPORT_MODULES = ("plan_memo_umbrella_selftest.py", "plan_memo_selftest_mutants.py")


def report_channel_control(M):
    """PROPERTY: every line the run REPORTS goes through the escape, measured
    over the emit sites rather than over the escape function.

    ⚠ THE CONTROL BESIDE THIS ONE HAD THE WRONG SUBJECT (PR #510 Axis 3 + Axis
    5, and each reached it by a different route).  `printable_output_control`
    iterates all 33 control characters through `printable()` directly, and the
    two mutants beside it edit `printable()`'s own expression -- so the SUBJECT
    of all three is the function.  Axis 5 proved the gap by executing it: delete
    every `printable(` CALL SITE from the runner, leave the function intact, and
    the whole suite stays green while a raw NUL returns to the log.  A probe
    whose subject is the dependency proves the dependency, never the caller
    (`memory/feedback_surviving-mutation-means-the-probe-has-another-subject.md`).
    And the docstring's claim -- "it lives at the one place every line goes
    through" -- was false when written: there are two print channels and eleven
    emit sites, of which three were wrapped.

    THE POPULATION IS THE EMIT SITE, NOT THE NAME.  Every `print(...)` and every
    `<list>.append(...)` in the two report modules whose argument is a `%`
    formatting expression over a literal format string must be wrapped in
    `printable(...)`.  That predicate is structural: it does not ask whether the
    arguments happen to carry a control name today, which is the symptom
    vocabulary and would leave the next site authoritative
    (`memory/feedback_checks-must-not-be-defined-by-the-symptom-vocabulary.md`).
    ⚠ No site is exempted for "it only formats numbers" -- an exemption list is
    the next class's hiding place, and escaping a number costs nothing.

    HONESTLY, what it cannot see: a line built by concatenation or an f-string
    rather than `%`; a third report module (the population is the two named
    here, and a new one would be invisible until it is added -- the same open
    edge every enumerated table in this suite has, and the reason the count is
    reported); and output written by something other than `print` / `append`.

    ⚠ AND ONE STRUCTURAL LIMIT ON THE PROOF, not on the control.  Both mutants
    that kill this control patch the RUNNER; none patches
    `plan_memo_selftest_mutants.py`, whose eight emit sites this control DOES
    cover.  That is not an omission: a mutant against that file would have to be
    exec'd by the loop that lives in it, which is already running, so the mutant
    runner is the one file in the set that cannot be its own subject.  The
    control's population is both modules; the mutation proof reaches one.  The
    gap is named here rather than left for the round that finds it
    (`memory/feedback_declared-blind-spots-are-where-the-next-finding-lands.md`),
    and the whole-channel attack IS verified by hand at every re-gate: strip
    every `printable(` call site from both files, keep the function, and this
    control goes red where the function's own control stays green -- reproduced
    on this commit (rc 1, one raw NUL back in the log)."""
    def formats(node):
        """The node is a `%` over a literal format string -- the shape a report
        line is built with, wrapped or not."""
        return (isinstance(node, ast.BinOp) and isinstance(node.op, ast.Mod)
                and isinstance(node.left, ast.Constant) and isinstance(node.left.value, str))

    bad, sites = [], 0
    for file, src in _swept_sources():
        if file not in _REPORT_MODULES:
            continue
        for node in ast.walk(ast.parse(src)):
            if not isinstance(node, ast.Call) or not node.args:
                continue
            f = node.func
            if not ((isinstance(f, ast.Name) and f.id == "print")
                    or (isinstance(f, ast.Attribute) and f.attr == "append")):
                continue
            arg = node.args[0]
            # ⚠ THE POPULATION IS BOTH STATES.  Counting only the UNWRAPPED
            # shape makes the denominator go to zero exactly when the property
            # holds, and the emptiness guard then reports a clean suite as a
            # broken control -- measured, on this control's first run.
            wrapped = (isinstance(arg, ast.Call) and isinstance(arg.func, ast.Name)
                       and arg.func.id == "printable" and arg.args and formats(arg.args[0]))
            if not (wrapped or formats(arg)):
                continue
            sites += 1
            if not wrapped:
                bad.append("%s:%d" % (file, node.lineno))
    if not sites:
        return False, ("no %%-formatted emit site found in %s: the shape this control reads is no "
                       "longer the one the report modules write" % ", ".join(_REPORT_MODULES))
    return not bad, ("%d emit site(s) over %d report module(s), %d not escaped%s"
                     % (sites, len(_REPORT_MODULES), len(bad),
                        ("; " + "; ".join(sorted(bad)[:4])) if bad else ""))


def option_set_control(M):
    """PROPERTY: the entry point accepts a CLOSED set of options and rejects
    its complement, and the set the code checks is the set the docstring's
    usage lines spell.

    WHY (PR #510 R42).  `main` took the path list as "every argv entry that
    does not start with `--`", so an unknown option was DISCARDED: `--worklis`,
    one character off `--worklist`, ran the ordinary report and exited 0, and a
    caller that asked for the worklist got the other format with nothing saying
    so. This PR's own census attestation is a byte comparison of `--worklist`
    output, so that caller was here.

    THE SUBJECT IS THE COMPLEMENT, which is the half that goes stale: a
    deny-list of known-bad spellings would pass every new one
    (`memory/feedback_enumerated-exemptions-leave-the-next-class-authoritative.md`).
    So the control asks TWO things -- that `OPTIONS` is what the usage lines
    spell (a drift check in both directions), and that a spelling outside it is
    refused by `main` itself, run over a real fixture rather than reasoned about.

    HONESTLY, what it cannot see: whether an accepted option does what it says,
    and a single-dash or bare-word argument (the path rule takes those, which is
    what makes a memo path a memo path)."""
    src = dict(_swept_sources())[ENTRY]
    doc = ast.get_docstring(ast.parse(src, filename=ENTRY)) or ""
    spelled = set(re.findall(r"--[a-z][a-z-]*", doc))
    declared = set(M.OPTIONS)
    if spelled != declared:
        return False, ("the usage lines spell %s and OPTIONS declares %s -- the two homes disagree"
                       % (sorted(spelled), sorted(declared)))
    import contextlib
    import io as _io
    bad = []
    with tempfile.TemporaryDirectory() as d:
        memo = pathlib.Path(d) / "fixture.md"
        memo.write_text(build(), encoding="utf-8")
        for arg, want in [("--worklis", 2), ("--definitely-invalid", 2), ("--WORKLIST", 2),
                          ("--worklist", 0), ("--mutants", 0)]:
            buf = _io.StringIO()
            with contextlib.redirect_stdout(buf):
                rc = M.main(["prog", str(memo), arg])
            if rc != want:
                bad.append("%s -> rc %d (want %d)" % (arg, rc, want))
    return not bad, ("%d option spelling(s) declared, %d probe(s) run, %d wrong%s"
                     % (len(declared), 5, len(bad), ("; " + "; ".join(bad)) if bad else ""))


def registry():
    """name -> (kind, control), this module's fragment of the one table, merged
    over the property module's (invariants included)."""
    reg = dict(property_registry())
    reg.update({
        "PROPERTY: every module of this checker is NAMED in the entry point's MODULES map (the map is checked, not asked to be kept)":
            ("CONTROL", module_map_completeness_control),
        "PROPERTY: every name the entry point's MODULES map spells is a file that exists (the rename half the completeness direction cannot see)":
            ("CONTROL", module_map_existence_control),
        "PROPERTY: every written `module.symbol` attribution names the module that DEFINES that symbol (the class five touch-time splits left with no detector)":
            ("CONTROL", symbol_attribution_control),
        "PROPERTY: every import-graph seam this suite states in prose is true of the imports (an \"only importer of X\" is a claim about the COMPLEMENT)":
            ("CONTROL", import_seam_control),
        "PROPERTY: every line the run REPORTS goes through the escape -- measured over the EMIT SITES, the subject the escape function's own control cannot reach":
            ("CONTROL", report_channel_control),
        "PROPERTY: the entry point accepts a CLOSED option set and REFUSES its complement (an unknown option was discarded, so a misspelt --worklist returned the other format at rc 0)":
            ("CONTROL", option_set_control),
    })
    return reg
