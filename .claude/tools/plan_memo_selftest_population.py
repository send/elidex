#!/usr/bin/env python3
"""Controls over the MODULE POPULATION and the two ROW REGISTRIES -- the
partner controls of every mechanism `plan_memo_selftest_harness` (the one
population) and `plan_memo_selftest_registry` (the one collection step) hold.

WHY A MODULE OF ITS OWN (the sixth attestation).  Three passes in a row added a
mechanism here -- a glob, a file-name partition, an import order, a collection
step, a seal -- and each pass left some of them without a control that could
go red, so the next attestation's probes found them silent.  The rule is now
that a mechanism arrives WITH its partner here and its killing row in
`plan_memo_selftest_mutants_population.py`.  Every partner works on a
TEMPORARY directory or a PLANTED module, never on the real tree, because the
real tree is exactly the input on which a wrong rule and a right one agree.

Every harness / registry function is reached through a CALL-TIME import, so a
row that patches either module (both are leaves: they own no controls) is
seen by the control it names.
"""

import ast
import importlib
import pathlib
import sys
import tempfile


def _h():
    return importlib.import_module("plan_memo_selftest_harness")


def _tree(d, files):
    for name, text in files.items():
        (d / name).write_text(text, encoding="utf-8")


# -- the population ----------------------------------------------------------

_ENTRY_SRC = "import plan_memo_a\n"
_DIR = {
    "plan_memo.py": "",                    # the glob is `plan_memo*.py`, not `plan_memo_*.py`
    "plan_memoize.py": "",
    "plan_memo_a.py": "import plan_memo_b\n",   # an `ast.Import` edge: b loads first
    "plan_memo_b.py": "",
    "plan_memo_selftest_mutants.py": "MUTANTS = []\n",
    "plan_memo_selftest_mut_r26.py": "MUTANTS = []\n",     # renamed out of any name prefix
    "plan_memo_selftest_mutants_zz.py": "X = 1\n",         # named like one, holds no list
    "plan_memo_selftest_mut_ann.py": "MUTANTS: list = []\n",        # an AnnAssign binds it
    "plan_memo_selftest_mut_nested.py": "def f():\n    MUTANTS = []\n",  # not at top level
    "plan_memo_c.py": "from plan_memo_d import x\n",   # a `from X import` edge: d loads first
    "plan_memo_d.py": "x = 1\nMUTANTS = []\n",        # holds a list, but is no SELF-TEST module
    "plan_memo_umbrella_selftest.py": "",
    "memo_extra.py": "",                   # outside the glob
    "other.py": "",
}


def population_partner_control(M):
    """(See `_population_partner`; an exception from the harness is this
    control going red, not a crash.)"""
    try:
        return _population_partner()
    except Exception as e:
        return False, "the harness RAISED on the planted directory: %s: %s" % (type(e).__name__, e)


def _population_partner():
    """PROPERTY: the harness DERIVES the population from a directory -- the
    glob (`plan_memo*.py` plus the entry point), the self-test partition by
    file name, the registry partition by CONTENT (a module holding its own
    `MUTANTS`), and the checker load order from the imports, with a cycle an
    error -- each asked of a temporary directory the real tree does not look
    like."""
    H = _h()
    bad = []
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        _tree(d, dict(_DIR, **{H.ENTRY: _ENTRY_SRC}))
        want = sorted(f for f in _DIR if f.startswith("plan_memo")) + [H.ENTRY]
        if H.files(d) != want:
            bad.append("files() = %s, want %s" % (H.files(d), want))
        part = {f: H.is_selftest(f) for f in ("plan_memo_umbrella_selftest.py",
                                              "plan_memo_selftest_mut_r26.py", "plan_memo_a.py")}
        if part != {"plan_memo_umbrella_selftest.py": True, "plan_memo_selftest_mut_r26.py": True,
                    "plan_memo_a.py": False}:
            bad.append("is_selftest partition %s" % part)
        reg = H.registry_modules("MUTANTS", d)
        if reg != ["plan_memo_selftest_mut_ann", "plan_memo_selftest_mut_r26",
                   "plan_memo_selftest_mutants"]:
            bad.append("registry_modules('MUTANTS') = %s (want the two that HOLD a list)" % reg)
        order = [n for n, _f in H._import_order(d)]
        if order.index("plan_memo_b") > order.index("plan_memo_a"):
            bad.append("import order %s puts an importer before its `import`ed module" % order)
        if order.index("plan_memo_d") > order.index("plan_memo_c"):
            bad.append("import order %s puts an importer before its `from`-imported module" % order)
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        _tree(d, {"plan_memo_a.py": "import plan_memo_b\n", "plan_memo_b.py": "import plan_memo_a\n",
                  H.ENTRY: ""})
        try:
            H._import_order(d)
            bad.append("a top-level import cycle was NOT an error")
        except RuntimeError:
            pass
    return not bad, "; ".join(bad) or "glob, partition, registry-by-content, order and cycle all hold"


# -- the golden manifest ------------------------------------------------------

def _mf():
    return importlib.import_module("plan_memo_selftest_manifest")


def manifest_control(M):
    """PROPERTY: the live collection is exactly the committed golden manifest,
    and every part of the manifest mechanism holds -- asked of the real
    collection and of planted inputs:

      (a) the COMPARISON: a manifest with one line removed, one added and one
          changed is reported as exactly that, and each kind of difference
          ALONE fails the check;
      (b) ONE DOOR: `take()` raises on a difference, so a run that does not
          compare has no table; `finish()` reports a verified control the run
          did not execute, and the wrapper records the ones it did;
      (c) ONE SOURCE: `write()` into a scratch file produces exactly
          `lines(_snapshot())`, `read` reads the file it is given, each
          registry is collected ONCE per snapshot, and a control's body (its
          code, nested bodies included -- not its constants) and a row's
          find/replace are in their lines;
      (d) DEEP IMMUTABILITY: `.pop()` on a collected row, a row's control list,
          a registry module's list and a case's files RAISES;
      (e) the generation-time VALIDATIONS: a row naming no control, a label
          used twice, a row naming a missing control, a case name used twice
          and two rows repeating one edit are each reported; a duplicated
          control name is refused while the table is built;
      (f) the manifest FILE's fault shapes -- missing, a directory,
          unreadable, empty, undecodable -- are one `ManifestError` naming the
          regenerate command, never a traceback;
      (g) `lines()` keeps DUPLICATES: a row collected twice is two lines."""
    mf, H = _mf(), _h()
    snap = mf._snapshot()
    real = mf.verify(snap)
    got = mf.lines(snap)
    if real:
        # the live collection already differs: that IS this control's claim,
        # and the arms below take a verified table, which `take` refuses
        return False, "%d difference(s) from the committed manifest: %s" % (len(real), real[0][:160])
    arms = {}
    extra = "CONTROL\tCONTROL\tplanted\tx"
    planted = got[1:] + [extra]
    planted[0] = planted[0] + "-changed"
    arms["(a) the diff is exactly one removed, one added and one changed line"] = (
        mf.diff(planted, got) == ([extra], [got[0]], [(got[1] + "-changed", got[1])]))
    arms["(a) each kind of difference ALONE fails the check"] = all(
        mf.verify(snap, want) for want in (got + [extra], got[1:], [got[0] + "-changed"] + got[1:]))
    taken = mf.take()
    arms["(b) `take` returns the verified table and the verified ROWS"] = (
        len(taken.entries) == len(snap.table) and taken.rows is snap.rows)
    arms["(b) the handle cannot be re-pointed at a smaller table"] = _raises(
        lambda: setattr(taken, "entries", taken.entries[1:]))
    saved = sys.modules.pop("plan_memo_umbrella_check", None)
    try:
        arms["(b) the escape does not depend on the checker being loaded"] = (
            mf.printable("\x07") != "\x07")
    finally:
        if saved is not None:
            sys.modules["plan_memo_umbrella_check"] = saved
    arms["(b) `finish` refuses a table nothing has run"] = _raises(lambda: mf.finish(taken))
    forged = mf.Taken(taken.entries, taken.rows)
    arms["(b) `finish` refuses a table this module did not make"] = _manifest_raises(
        mf, lambda: mf.finish(forged))
    marks = {}
    mf._wrap("x", lambda M: (True, ""), marks)(None)
    mf._wrap("x", lambda M: (True, ""), marks)(None)
    arms["(b) the wrapper COUNTS the runs of each control"] = marks == {"x": 2}
    one = [("a", "CONTROL")]
    arms["(b) the audit is: exactly once, and nothing else"] = (
        mf.audit(one, {"a": 1}) == []
        and any("ran 2 time" in q for q in mf.audit(one, {"a": 2}))
        and any("ran 0 time" in q for q in mf.audit(one, {}))
        and any("not in the verified table" in q for q in mf.audit(one, {"a": 1, "b": 1})))
    arms["(b) the counts are of what RAN"] = (mf.counts(one, {"a": 1}) == {"CONTROL": 1}
                                              and mf.counts(one, {}) == {"CONTROL": 0})
    # ONE enumeration per registry: the collection step records its calls
    reg_mod = importlib.import_module("plan_memo_selftest_registry")
    del reg_mod.CALLS[:]
    mf._build()          # the BUILDER: `_snapshot` serves the one already built
    built = list(reg_mod.CALLS)
    del reg_mod.CALLS[:]
    importlib.import_module("plan_memo_selftest_mutants").mutants()
    arms["(c) the snapshot collects each registry ONCE, by its own name"] = (
        built == ["CASES", "MUTANTS"] and reg_mod.CALLS == ["MUTANTS"])
    del reg_mod.CALLS[:]
    same = mf._snapshot() is mf._snapshot()
    arms["(c) the snapshot is built ONCE per process"] = same and reg_mod.CALLS == []
    # one definition per LINE: the source block of a `lambda` is its line, so
    # two lambdas sharing a line share a digest (stated in `_source_digest`)
    def _body_a(M):
        return True, "stub"

    def _body_b(M):
        return M.check(M)

    def _nest_a(M):
        return (lambda: "a")()

    def _nest_b(M):
        return (lambda: "b")()

    stub = mf._fn_id(_body_a) != mf._fn_id(_body_b)
    nested = mf._fn_id(_nest_a) != mf._fn_id(_nest_b)
    arms["(c) a control's BODY is in its line, nested bodies included"] = stub and nested
    arms["(c) a row's find/replace is in its line"] = (
        mf.lines(mf.Snapshot({}, (("a", "f.py", "x", "y", ()),), ()))
        != mf.lines(mf.Snapshot({}, (("a", "f.py", "y", "x", ()),), ())))
    with tempfile.TemporaryDirectory() as d:
        # the DOOR, driven with a planted committed manifest: `take()` reads
        # `PATH`, so pointing it at a corrupt file must make it raise
        corrupt = pathlib.Path(d) / "corrupt.txt"
        corrupt.write_text("CONTROL\tCONTROL\tplanted\tx\n", encoding="utf-8")
        real_path = mf.PATH
        try:
            mf.PATH = corrupt
            arms["(b) `take` RAISES when the committed manifest differs"] = _manifest_raises(
                mf, mf.take)
        finally:
            mf.PATH = real_path
        # the DIGEST, driven with a planted module: it must read the text the
        # module was EXEC'D from (`SOURCES`), and refuse an unreachable source
        H_mod = _h()
        probe_file = pathlib.Path(d) / "plan_memo_selftest_plant_src.py"
        probe_file.write_text("def g(M):\n    return 1\n", encoding="utf-8")
        sys.path.insert(0, d)
        try:
            src_mod = importlib.import_module("plan_memo_selftest_plant_src")
            before = mf._fn_id(src_mod.g)
            H_mod.SOURCES["plan_memo_selftest_plant_src.py"] = "def g(M):\n    return 2\n"
            arms["(c) the digest reads the text the module was exec'd from"] = (
                mf._fn_id(src_mod.g) != before)
        finally:
            H_mod.SOURCES.pop("plan_memo_selftest_plant_src.py", None)
            sys.path.remove(d)
            sys.modules.pop("plan_memo_selftest_plant_src", None)
        gone = {}
        exec(compile("def g(M):\n    return 1\n", "plan_memo_no_such_file.py", "exec"), gone)
        arms["(c) an unreachable source is an ERROR, never a constant"] = _manifest_raises(
            mf, lambda: mf._fn_id(gone["g"]))
        target = pathlib.Path(d) / "m.txt"
        rc, _msgs = mf.write(target)
        written = (sorted(ln for ln in target.read_text(encoding="utf-8").split("\n")
                          if ln and not ln.startswith("#")) if target.exists() else None)
        arms["(c) the generator writes exactly the runner's collection"] = rc == 0 and written == got
        target.write_text("# a comment\nB\nA\n", encoding="utf-8")
        arms["(c) `read` reads the file it is given, comments dropped"] = mf.read(target) == ["A", "B"]
        faults = {"missing": pathlib.Path(d) / "nope.txt", "a directory": pathlib.Path(d),
                  "empty": pathlib.Path(d) / "empty.txt",
                  "undecodable": pathlib.Path(d) / "bytes.txt"}
        faults["empty"].write_text("", encoding="utf-8")
        faults["undecodable"].write_bytes(b"\xff\xfe\x00rows")
        unreadable = pathlib.Path(d) / "locked.txt"
        unreadable.write_text("x\n", encoding="utf-8")
        unreadable.chmod(0)
        faults["unreadable"] = unreadable
        # ⚠ `chmod(0)` does not stop root, so the unreadable shape is asked
        # only when the file really is unreadable -- and that is measured HERE,
        # while the mode is still 0, not after restoring it (measured after,
        # it was always readable and the shape was silently never asked)
        really_unreadable = not _readable(unreadable)
        shapes = {k: _manifest_error(mf, v) for k, v in faults.items()}
        unreadable.chmod(0o600)
        if not really_unreadable:
            shapes.pop("unreadable")
        arms["(f) every fault shape (%d of them) names the regenerate command"
             % len(shapes)] = all(shapes.values())
    # (d) on a FRESHLY planted base module: the real modules were frozen by the
    # first collection of this run, so only a module collected now can show
    # whether the collection step freezes
    name = "plan_memo_selftest_plant_manifest_base"
    with tempfile.TemporaryDirectory() as d:
        (pathlib.Path(d) / (name + ".py")).write_text(
            "MUTANTS = [('planted', 'f.py', 'x', 'y', ['c']), {'k': [1]}]\n", encoding="utf-8")
        sys.path.insert(0, d)
        try:
            rows = reg_mod.collect("MUTANTS", name)
            mod = importlib.import_module(name)
            pops = [lambda: rows[0][4].pop(), lambda: rows.pop(), lambda: mod.MUTANTS.pop(),
                    lambda: mod.MUTANTS[1].pop("k"), lambda: mod.MUTANTS[1][0][1].pop(),
                    lambda: snap.rows[0][4].pop(), lambda: snap.cases[0].files.pop()]
            arms["(d) every collected value refuses `.pop()`"] = all(_raises(f) for f in pops)
        finally:
            sys.path.remove(d)
            sys.modules.pop(name, None)
    row = ("a", "f.py", "x", "y", ("c",))
    fake = mf.Snapshot({"c": ("CONTROL", None)},
                       (("e", "f.py", "p", "q", ()), row, ("a", "f.py", "x2", "y2", ("gone",)),
                        ("b", "f.py", "x", "y", ("c",))),
                       (snap.cases[0], snap.cases[0]))
    probs = mf.validate(fake)
    arms["(e) each validation reports its planted defect"] = (
        any("'e' names no control" in q for q in probs) and any("'a' is used 2 times" in q for q in probs)
        and any("names 'gone'" in q for q in probs) and any("case name" in q for q in probs)
        and any("repeat one edit" in q for q in probs)
        and _raises(lambda: H.merge({"k": 1}, {"k": 2}))
        and _raises(lambda: H.merge({"k": 1}).update({"k": 2})))
    twice = mf.Snapshot({}, (row, row), ())
    arms["(g) a row collected twice is two lines"] = len(mf.lines(twice)) == 2
    # the separator is escaped, so the control list is injective
    one_pipe = mf.Snapshot({}, (("a", "f.py", "x", "y", ("c|d",)),), ())
    two_names = mf.Snapshot({}, (("a", "f.py", "x", "y", ("c", "d")),), ())
    arms["(g) a `|` inside a control name is escaped"] = mf.lines(one_pipe) != mf.lines(two_names)
    with tempfile.TemporaryDirectory() as d:
        empty = "plan_memo_selftest_plant_empty_base"
        (pathlib.Path(d) / (empty + ".py")).write_text("MUTANTS = []\n", encoding="utf-8")
        sys.path.insert(0, d)
        try:
            arms["(h) a registry module holding an EMPTY list is refused"] = _raises(
                lambda: reg_mod.collect("MUTANTS", empty))
            sys.modules.pop(empty, None)     # or the next collection would find it
            # the NEGATIVE half: a module holding rows is collected, so the
            # refusal above is not "every plant is refused"
            full = "plan_memo_selftest_plant_full_base"
            (pathlib.Path(d) / (full + ".py")).write_text(
                "MUTANTS = [('planted', 'f.py', 'x', 'y', ['c'])]\n", encoding="utf-8")
            arms["(h) a registry module holding ROWS is collected"] = (
                len(reg_mod.collect("MUTANTS", full)) > len(snap.rows))
            sys.modules.pop(full, None)
            # the CASES constructors write where the collector reads: a planted
            # module's own list, not a captured or a base one (CRIT-2)
            probe = "plan_memo_selftest_plant_cases_probe"
            (pathlib.Path(d) / (probe + ".py")).write_text(
                "from plan_memo_selftest_cases import build, spellings\n"
                "CASES = []\n"
                "case, acase, rcase = spellings()\n"
                "case('POSITIVE', 'planted', build(), '', 1)\n"
                "rcase('POSITIVE', 'planted rc', build(), '', 0)\n", encoding="utf-8")
            base_cases = len(importlib.import_module("plan_memo_selftest_cases").CASES)
            mod = importlib.import_module(probe)
            arms["(h) the case constructors write into the CALLING module's list"] = (
                len(mod.CASES) == 2
                and len(importlib.import_module("plan_memo_selftest_cases").CASES) == base_cases)
            sys.modules.pop(probe, None)
            # a module that WRITES a case but holds no list is refused at its
            # import: its cases would otherwise go nowhere, and a case that was
            # never collected is the one thing the manifest cannot compare
            listless = "plan_memo_selftest_plant_cases_listless"
            (pathlib.Path(d) / (listless + ".py")).write_text(
                "from plan_memo_selftest_cases import build, spellings\n"
                "case, acase, rcase = spellings()\n"
                "case('POSITIVE', 'planted', build(), '', 1)\n", encoding="utf-8")
            arms["(h) a module that writes a case but holds no list is refused"] = _refuses(
                "holds no `CASES` list", lambda: importlib.import_module(listless))
            sys.modules.pop(listless, None)
        finally:
            sys.path.remove(d)
            sys.modules.pop(empty, None)
    failed = [k for k, v in arms.items() if not v]
    return not failed, ("%d manifest line(s), 0 difference(s); arms: %s"
                        % (len(got), "all hold" if not failed else "FAILED " + "; ".join(failed)))


def _manifest_raises(mf, fn):
    """True when `fn` raises the manifest's OWN error -- any other exception is
    a crash, not a refusal."""
    try:
        fn()
    except mf.ManifestError:
        return True
    except Exception:
        return False
    return False


def _refuses(message, fn):
    """True when `fn` raises with `message` in it -- a REFUSAL, not whatever
    exception a missing check happens to produce downstream."""
    try:
        fn()
    except Exception as e:
        return message in str(e)
    return False


def _readable(path):
    try:
        path.read_text(encoding="utf-8")
    except OSError:
        return False
    return True


def _manifest_error(mf, path):
    """True when reading `path` raises the ONE manifest error, with the
    regenerate command in its message."""
    try:
        mf.read(path)
    except mf.ManifestError as e:
        return mf.REGENERATE in str(e)
    except Exception:
        return False
    return False


def _raises(fn):
    try:
        fn()
    except Exception:
        return True
    return False



# -- membership: nothing related to the population is outside it, and nothing
# in it is unreached --------------------------------------------------------

def _imports(src, file):
    """Every module name `src` imports, at any depth.  (A relative `from .
    import x` yields `None`, which no file stem equals, so the caller's
    intersection drops it -- a `node.module` guard stood here and was deleted
    as unkillable by the clause-drop sweep.)"""
    got = set()
    for node in ast.walk(ast.parse(src, filename=file)):
        if isinstance(node, ast.ImportFrom):
            got.add(node.module)
        elif isinstance(node, ast.Import):
            got |= {a.name for a in node.names}
    return got


def _membership_verdict(disk, population, roots):
    """The rule's violations over {file: text} of a directory, its population
    and the roots a run starts from: (1) a file OUTSIDE the population that
    imports it or that it imports; (2) a population module reached from no
    root along the import graph -- on disk, and never run."""
    H = _h()
    stem = {H.import_name(f): f for f in disk}
    inside = {H.import_name(f) for f in population}
    graph = {H.import_name(f): _imports(src, f) & set(stem) for f, src in disk.items()}
    bad = []
    for name, deps in sorted(graph.items()):
        if name in inside:
            bad += ["%s is imported by the population but is not in it" % stem[x]
                    for x in sorted(deps - inside)]
        elif deps & inside:
            bad.append("%s imports the population but is not in it" % stem[name])
    # REACHED, as a bounded fixed point (the sixth-pass attestation's N-3: a
    # worklist's `if u in seen` guard was unkillable -- dropping it changed
    # nothing but termination on a cycle).  A root outside the population
    # reaches nothing, which the partner's ghost-root arm pins.
    reached = {r for r in roots if r in inside}
    for _round in range(len(inside)):
        reached |= {v for u in reached for v in graph.get(u, set()) & inside}
    bad += ["%s is in the population and reached from no root -- on disk, never run" % stem[n]
            for n in sorted(inside - reached)]
    return sorted(set(bad))


def _roots(H, here=None):
    return ([H.ENTRY_NAME] + H.registry_modules("MUTANTS", here)
            + H.registry_modules("CASES", here))


def _membership_of(here):
    """THE membership verdict of a directory -- read off its disk exactly as
    the real control reads it, so the partner (a planted directory) and the
    real control call the SAME function (the sixth-pass attestation's IMP-2:
    the partner had driven a hand dict, and narrowing the disk read to the
    population glob left both green)."""
    H = _h()
    disk = {q.name: q.read_text(encoding="utf-8") for q in sorted(here.glob("*.py"))}
    return _membership_verdict(disk, H.files(here), _roots(H, here))


_M_DIR = {
    "plan_memo_roles.py": "import memo_extra\n",
    "memo_extra.py": "",                                  # outside the glob, imported by it
    "stranger.py": "import plan_memo_roles\nimport plan_memo_zz\n",   # outside, importing it
    "plan_memo_zz.py": "",                                # on disk, reached from nothing
    "plan_memo_selftest_cases_zz.py": "CASES = []\n",   # a registry module: a root
}


def registry_membership_control(M):
    """PROPERTY: no module can leave a run unnoticed -- every `.py` file beside
    the checker that the population imports, or that imports it, IS in the
    population; and every module of the population is REACHED from a root
    (the entry point, and each registry module the collection step gathers).

    ⚠ The sixth attestation's probes, each of which was rc 0 before this:
    a helper outside the glob imported by `plan_memo_roles.py` (its calls
    were never read); a cases module on disk that nothing imported (its case
    never ran); a new empty checker file, and a self-test-named helper that
    nothing imports.  The partner arms plant each IN A DIRECTORY and ask the
    same `_membership_of` the real control asks.  The ghost-root arm: a root
    outside the population (`stranger`) reaches nothing."""
    H = _h()
    bad = _membership_of(H.HERE)
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        _tree(d, dict(_M_DIR, **{H.ENTRY: "import plan_memo_roles\n"}))
        got = _membership_of(d)
    # the ghost-root arm asks the CORE directly: a root outside the population
    # (`stranger`, which imports `plan_memo_zz`) must reach nothing
    ghost = _membership_verdict(dict(_M_DIR, **{H.ENTRY: ""}),
                                [f for f in _M_DIR if f.startswith("plan_memo")] + [H.ENTRY],
                                [H.ENTRY_NAME, "stranger"])
    arms = [any("memo_extra.py is imported by the population" in b for b in got),
            any("stranger.py imports the population" in b for b in got),
            any(b.startswith("plan_memo_zz.py is in the population and reached from no root")
                for b in got),
            not any("plan_memo_selftest_cases_zz.py" in b for b in got),
            any(b.startswith("plan_memo_zz.py is in the population and reached from no root")
                for b in ghost)]
    return not bad and all(arms), ("%d violation(s)%s; planted-directory arms %s"
                                   % (len(bad), ("; " + "; ".join(bad[:3])) if bad else "", arms))


def registry():
    """name -> (kind, control), this module's fragment of the one table."""
    return {
        "PROPERTY: the live collection is exactly the committed golden manifest, and the manifest mechanism holds (the one door, the comparison, one source, deep immutability, the generation-time validations and the file's fault shapes)":
            ("CONTROL", manifest_control),
        "PROPERTY: the harness DERIVES the population from a directory (glob, self-test partition, registry-by-content, import order, cycle) -- asked of a planted directory":
            ("CONTROL", population_partner_control),
        "PROPERTY: no module can leave a run unnoticed -- every file related to the population is in it, and every module in it is reached from a root":
            ("CONTROL", registry_membership_control),
    }
