#!/usr/bin/env python3
"""The mutation-proof rows against the MODULE POPULATION's rules -- carved on a
SUBJECT, like `plan_memo_selftest_mutants_ratchets.py`: the rules that the one
population (`plan_memo_selftest_harness.files`) must satisfy and that are
checked as rules rather than listed (the fifth attestation): the population's
derivation, membership, the IMPORT DIRECTION rule, the property family and the
golden manifest (`plan_memo_selftest_manifest`).  Each row disables one clause,
against the partner arm that kills it.

`MUTANTS` here is this module's OWN list; `plan_memo_selftest_mutants.mutants()`
gathers every mutants module's list (`plan_memo_selftest_harness.registry_modules`
decides which modules those are, by CONTENT: a module holding its own list) in one explicit step.
"""

from plan_memo_selftest_mutants import CONTROLS, RECORDS

MUTANTS = []

DIRECTION = ("PROPERTY: the self-test's IMPORT DIRECTION rule holds of the imports (no cycle, the "
             "harness imports nothing of the set, no checker module imports the self-test but the "
             "entry point's dispatch) -- a rule over the AST, not an inventory of edges")

FAMILY = "the entries named `PROPERTY: ...` are exactly the property family's fragments (`_FAMILY`), both directions"
HARNESS, REGISTRY = "plan_memo_selftest_harness.py", "plan_memo_selftest_registry.py"
POPULATION_CTL = ("PROPERTY: the harness DERIVES the population from a directory (glob, self-test "
                  "partition, registry-by-content, import order, cycle) -- asked of a planted directory")
MEMBERSHIP = ("PROPERTY: no module can leave a run unnoticed -- every file related to the population "
              "is in it, and every module in it is reached from a root")

POPMOD = "plan_memo_selftest_population.py"

MANIFEST_CTL = ("PROPERTY: the live collection is exactly the committed golden manifest, and the "
                "manifest mechanism holds (the one door, the comparison, one source, deep "
                "immutability, the generation-time validations and the file's fault shapes)")
MANIFEST_MOD = "plan_memo_selftest_manifest.py"

MUTANTS += [
    ("family: the ratchets module IS in the property family (re-inject the three-module family the "
     "docstrings stated -- every ratchet entry is then a PROPERTY name from outside it)", CONTROLS,
     '_FAMILY = ("plan_memo_selftest_records", "plan_memo_selftest_ratchets",',
     '_FAMILY = ("plan_memo_selftest_records",',
     [FAMILY]),
    ("family: the STRAY direction is reported (drop it -- a PROPERTY name from outside the family "
     "passes)", CONTROLS,
     '    return sorted(set(family) - named), sorted(named - set(family))',
     '    return sorted(set(family) - named), []',
     [FAMILY]),
    ("population: the entry point is in it (drop it from `files()`)", HARNESS,
     '    return sorted(p.name for p in here.glob(GLOB)) + [ENTRY]',
     '    return sorted(p.name for p in here.glob(GLOB))',
     [POPULATION_CTL]),
    ("population: the glob is `plan_memo*.py` (re-inject the narrowed `plan_memo_*.py` -- "
     "`plan_memo.py` leaves in silence)", HARNESS,
     'GLOB = "plan_memo*.py"',
     'GLOB = "plan_memo_*.py"',
     [POPULATION_CTL]),
    ("population: the self-test partition reads the whole name (read a prefix -- the runner "
     "`plan_memo_umbrella_selftest.py` becomes a checker module)", HARNESS,
     '    return "selftest" in file',
     '    return file.startswith("plan_memo_selftest")',
     [POPULATION_CTL]),
    ("population: a registry module is one that HOLDS the list (re-inject the name rule -- a "
     "renamed module's rows leave in silence)", HARNESS,
     "    return [import_name(f) for f in files(here) if is_selftest(f) and _assigns(here, f, name)]",
     '    return [import_name(f) for f in files(here)\n'
     '            if f.startswith("plan_memo_selftest_" + name.lower())]',
     [POPULATION_CTL]),
    ("population: an `import X` is an order edge (read `from X import` alone)", HARNESS,
     "            elif isinstance(node, ast.Import):\n                got |= {a.name for a in node.names if a.name in by_name}",
     "            elif False:\n                got |= {a.name for a in node.names if a.name in by_name}",
     [POPULATION_CTL]),
    ("population: an import cycle is an error (break out of it -- a misordered load in silence)",
     HARNESS,
     "        if not ready:\n            raise RuntimeError(",
     "        if not ready:\n            break\n            raise RuntimeError(",
     [POPULATION_CTL]),
    ("membership: a file the population imports must be in it (drop the clause)", "plan_memo_selftest_population.py",
     '            bad += ["%s is imported by the population but is not in it" % stem[x]\n'
     '                    for x in sorted(deps - inside)]',
     "            pass",
     [MEMBERSHIP]),
    ("membership: a file importing the population must be in it (drop the clause)",
     "plan_memo_selftest_population.py",
     '            bad.append("%s imports the population but is not in it" % stem[name])',
     "            pass",
     [MEMBERSHIP]),
    ("membership: every module is reached from a root (drop the clause -- a module on disk never "
     "runs)", "plan_memo_selftest_population.py",
     '    bad += ["%s is in the population and reached from no root -- on disk, never run" % stem[n]\n'
     '            for n in sorted(inside - reached)]',
     "    pass",
     [MEMBERSHIP]),
    ("direction: the harness imports nothing of the population (drop the clause)", RECORDS,
     "    if graph.get(harness):",
     "    if False:",
     [DIRECTION]),
    ("direction: no checker module imports the self-test (drop the clause)", RECORDS,
     "            if (mod, dep) != (import_name(ENTRY), RUNNER_NAME):",
     "            if False:",
     [DIRECTION]),
    ("direction: the entry point's dispatch is the ONE exception (drop the exception -- the rule "
     "then forbids the runner's own entry)", RECORDS,
     "            if (mod, dep) != (import_name(ENTRY), RUNNER_NAME):",
     "            if True:",
     [DIRECTION]),
    ("direction: a cycle is a violation (drop the report)", RECORDS,
     '                bad.append("cycle: %s" % " -> ".join(path[path.index(v):] + [v]))',
     "                pass",
     [DIRECTION]),
    ("direction: a FUNCTION-LOCAL import is an edge (read module-level statements only -- the "
     "form a hidden cycle takes)", RECORDS,
     "        for node in ast.walk(ast.parse(src, filename=file)):\n            if isinstance(node, ast.ImportFrom) and node.module in names:",
     "        for node in ast.parse(src, filename=file).body:\n            if isinstance(node, ast.ImportFrom) and node.module in names:",
     [DIRECTION]),
    ("population: a registry module binds its list at TOP LEVEL (read every assignment -- a list "
     "bound inside a function makes a module a registry)", HARNESS,
     "    for node in tree.body:\n        targets = (node.targets",
     "    for node in ast.walk(tree):\n        targets = (node.targets",
     [POPULATION_CTL]),
    ("population: an ANNOTATED assignment binds the list (read `Assign` alone)", HARNESS,
     "                   else [node.target] if isinstance(node, ast.AnnAssign) else [])",
     "                   else [])",
     [POPULATION_CTL]),
    ("population: a registry module is a SELF-TEST module (drop it -- a checker module holding "
     "a `MUTANTS` becomes one)", HARNESS,
     "if is_selftest(f) and _assigns(here, f, name)]",
     "if _assigns(here, f, name)]",
     [POPULATION_CTL]),
    ("population: a `from X import` is an order edge (drop it)", HARNESS,
     "            if isinstance(node, ast.ImportFrom) and node.module in by_name:\n"
     "                got.add(node.module)",
     "            if False:\n                got.add(node.module)",
     [POPULATION_CTL]),
    ("merge: `Registry` refuses a name it holds (drop the refusal)", HARNESS,
     "        if key in self:\n            raise KeyError(",
     "        if False:\n            raise KeyError(",
     [MANIFEST_CTL]),
    ("merge: `Registry.update` goes through the refusal (bypass it)", HARNESS,
     "        for key, value in dict(*args, **kw).items():\n            self[key] = value",
     "        dict.update(self, *args, **kw)",
     [MANIFEST_CTL]),
    ("membership: the disk is EVERY `.py` beside the checker (read the population glob alone -- "
     "a helper outside it is never seen)", POPMOD,
     '    disk = {q.name: q.read_text(encoding="utf-8") for q in sorted(here.glob("*.py"))}',
     '    disk = {q.name: q.read_text(encoding="utf-8") for q in sorted(here.glob(H.GLOB))}',
     [MEMBERSHIP]),
    ("membership: a FUNCTION-LOCAL import is an edge (read top-level statements only -- the "
     "runner, imported inside the entry point's dispatch, is then reached by nothing)", POPMOD,
     "    for node in ast.walk(ast.parse(src, filename=file)):\n        if isinstance(node, ast.ImportFrom):",
     "    for node in ast.parse(src, filename=file).body:\n        if isinstance(node, ast.ImportFrom):",
     [MEMBERSHIP]),
    ("membership: a root outside the population reaches nothing (take every root)", POPMOD,
     "    reached = {r for r in roots if r in inside}",
     "    reached = set(roots)",
     [MEMBERSHIP]),
    ("membership: reachability follows the import graph (stop after the roots)", POPMOD,
     "    for _round in range(len(inside)):",
     "    for _round in range(0):",
     [MEMBERSHIP]),
]

# -- the golden manifest: each part of the mechanism, against the arm of
# `manifest_control` that kills it.
AXIS5_CHANNEL = ("PROPERTY: every line the run REPORTS goes through the escape -- measured over the "
                 "EMIT SITES, the subject the escape function's own control cannot reach")

MUTANTS += [
    ("report channel: a raised `ManifestError` is a report channel (drop it -- the manifest diff "
     "reaches stderr unescaped, which is where three raw NULs went)", RECORDS,
     '        if name == "ManifestError":\n            return node.exc.args[0]',
     "        if False:\n            return node.exc.args[0]",
     [AXIS5_CHANNEL]),
    ("report channel: a raised CRASH is not a report channel (take every raise -- the population "
     "then holds developer-facing invariant messages)", RECORDS,
     '        name = getattr(node.exc.func, "id", None) or getattr(node.exc.func, "attr", None)\n'
     '        if name == "ManifestError":',
     '        name = "ManifestError"\n        if name == "ManifestError":',
     [AXIS5_CHANNEL]),
    ("cases: a module that writes a case but holds no list is REFUSED (ignore it -- its cases go "
     "nowhere, and a case that was never collected is what the manifest cannot compare)",
     "plan_memo_selftest_cases.py",
     "        if not isinstance(rows, list):",
     "        if False:",
     [MANIFEST_CTL]),
    ("usage: the help text is NOT routed through the report escape (re-inject it -- the 13 KB "
     "usage prints as one physical line)", "plan-memo-umbrella-check.py",
     "        print(__doc__)",
     "        print(printable(__doc__))",
     [AXIS5_CHANNEL]),
    ("report: a line's embedded NEWLINE is escaped (leave line endings alone -- a finding whose "
     "text carries one then breaks a line-oriented consumer)", "plan-memo-umbrella-check.py",
     'if c < " " or c == "\\x7f"',
     'if (c < " " and c != "\\n") or c == "\\x7f"',
     [AXIS5_CHANNEL]),
    ("report: every LONE SURROGATE is escaped (drop the arm -- a non-UTF-8 POSIX filename then "
     "raises UnicodeEncodeError from the strict UTF-8 channel, a traceback at the findings code)",
     "plan-memo-umbrella-check.py",
     ' or "\\ud800" <= c <= "\\udfff"',
     "",
     ["a memo path with a lone surrogate (a non-UTF-8 POSIX filename byte) is reported escaped at rc 2, "
      "never a UnicodeEncodeError from the strict UTF-8 channel"]),
    ("memo: a memo is a REGULAR file, asked before the read (drop the guard -- a `.md` link to a "
     "device is read to EOF: `/dev/zero` never ends)", "plan_memo_memo.py",
     "        if not stat.S_ISREG(os.stat(self.path).st_mode):",
     "        if False:",
     ["a linked `.md` whose target is not a regular file (a device, a FIFO, a socket) is the unavailable-memo miss "
      "at rc 2, refused before it is read"]),
    ("report: the surrogate arm stops at U+D800 (widen it down one -- U+D7FF, the code point just below "
     "the range, is then mangled)", "plan-memo-umbrella-check.py",
     ' or "\\ud800" <= c <= "\\udfff"',
     ' or "\\ud7ff" <= c <= "\\udfff"',
     ["a memo path with a lone surrogate (a non-UTF-8 POSIX filename byte) is reported escaped at rc 2, "
      "never a UnicodeEncodeError from the strict UTF-8 channel"]),
    ("report: the surrogate arm stops at U+DFFF (drop the upper bound -- U+E000 and everything above "
     "it, the fullwidth forms included, is then mangled)", "plan-memo-umbrella-check.py",
     ' or "\\ud800" <= c <= "\\udfff"',
     ' or "\\ud800" <= c',
     ["a memo path with a lone surrogate (a non-UTF-8 POSIX filename byte) is reported escaped at rc 2, "
      "never a UnicodeEncodeError from the strict UTF-8 channel"]),
    ("report: only the surrogates are escaped above DEL (escape all non-ASCII -- every Japanese "
     "line becomes <U+XXXX> soup)", "plan-memo-umbrella-check.py",
     ' or "\\ud800" <= c <= "\\udfff"',
     ' or c > "\\x7f"',
     ["a memo path with a lone surrogate (a non-UTF-8 POSIX filename byte) is reported escaped at rc 2, "
      "never a UnicodeEncodeError from the strict UTF-8 channel"]),
    ("memo: the guard asks REGULAR, not one non-regular kind (narrow it to a char device -- a FIFO "
     "then blocks the run again)", "plan_memo_memo.py",
     "        if not stat.S_ISREG(os.stat(self.path).st_mode):",
     "        if stat.S_ISCHR(os.stat(self.path).st_mode):",
     ["a linked `.md` whose target is not a regular file (a device, a FIFO, a socket) is the unavailable-memo miss "
      "at rc 2, refused before it is read"]),
    ("manifest: the ESCAPE does not depend on load state (make it the identity -- the runner "
     "unloads before it reports, and a control name's BEL then reaches stderr raw)", MANIFEST_MOD,
     "    return _ESCAPE[0](text) if _ESCAPE else text",
     "    return text",
     [MANIFEST_CTL]),
    ("manifest: `take` hands back the VERIFIED rows (hand back fewer -- the mutation proof then "
     "runs a smaller registry and says nothing)", MANIFEST_MOD,
     "    taken = Taken(entries, snap.rows)",
     "    taken = Taken(entries, snap.rows[:-1])",
     [MANIFEST_CTL]),
    ("manifest: a difference is reported (skip the comparison)", MANIFEST_MOD,
     "    if removed or added or changed:",
     "    if False:",
     [MANIFEST_CTL]),
    ("manifest: a REMOVED line is a difference (drop that half)", MANIFEST_MOD,
     "    removed = sorted((w - g).elements())",
     "    removed = []",
     [MANIFEST_CTL]),
    ("manifest: an ADDED line alone is a difference (read removals and changes only)", MANIFEST_MOD,
     "    if removed or added or changed:",
     "    if removed or changed:",
     [MANIFEST_CTL]),
    ("manifest: a CHANGED line is reported once, as changed (leave it in removed and added too)",
     MANIFEST_MOD,
     "    return ([x for x in removed if x not in gone], [a for a in added if a not in came], changed)",
     "    return (removed, added, changed)",
     [MANIFEST_CTL]),
    ("manifest: `read` reads the file it is given (read the committed one always)", MANIFEST_MOD,
     "    target = PATH if path is None else path\n    try:",
     "    target = PATH\n    try:",
     [MANIFEST_CTL]),
    ("manifest: the generator writes the RUNNER's collection (write a different one -- the rows "
     "dropped)", MANIFEST_MOD,
     "    snap = _snapshot()\n    problems = validate(snap)",
     "    snap = _snapshot()._replace(rows=())\n    problems = validate(snap)",
     [MANIFEST_CTL]),
    ("manifest: the collection FREEZES what it reads (return it as it is)", REGISTRY,
     "        frozen = _freeze(getattr(mod, listname))",
     "        frozen = getattr(mod, listname)",
     [MANIFEST_CTL]),
    ("manifest: the collection replaces each module's list by its frozen copy (leave the list)",
     REGISTRY,
     "        setattr(mod, listname, frozen)",
     "        pass",
     [MANIFEST_CTL]),
    ("manifest: a dict is frozen into sorted pairs (leave it a dict -- a case's `files`)", REGISTRY,
     "    if isinstance(x, dict):\n        return tuple(",
     "    if False:\n        return tuple(",
     [MANIFEST_CTL]),
    ("manifest: a row naming NO control is refused at generation (drop the check)", MANIFEST_MOD,
     "        if not controls:",
     "        if False:",
     [MANIFEST_CTL]),
    ("manifest: a row naming a MISSING control is refused at generation (drop the check)",
     MANIFEST_MOD,
     "            if c not in snap.table:",
     "            if False:",
     [MANIFEST_CTL]),
    ("manifest: a mutation label used twice is refused at generation (drop the check)", MANIFEST_MOD,
     '    bad += ["mutation label %r is used %d times"',
     '    bad += [] and ["mutation label %r is used %d times"',
     [MANIFEST_CTL]),
    ("manifest: a case name used twice is refused at generation (drop the check)", MANIFEST_MOD,
     '    bad += ["case name %r is used %d times"',
     '    bad += [] and ["case name %r is used %d times"',
     [MANIFEST_CTL]),
]

# -- the manifest attestation (CRIT 2 / IMP 5 / MIN 4): the door, the sink, the
# restored validations and the digests, each against the arm that kills it.
MUTANTS += [
    ("manifest: `take` RAISES on a difference (return the table anyway -- the runner would "
     "execute a collection it had not compared)", MANIFEST_MOD,
     "    snap = _snapshot()\n    bad = verify(snap)",
     "    snap = _snapshot()\n    bad = []",
     [MANIFEST_CTL]),
    ("manifest: `finish` refuses a run that did not execute the verified table (drop the audit)",
     MANIFEST_MOD,
     "    wrong = audit(verified, seen)",
     "    wrong = []",
     [MANIFEST_CTL]),
    ("manifest: `finish` refuses a table this module did not make (accept any)", MANIFEST_MOD,
     "    if held is None or held[0] is not taken:",
     "    if False:",
     [MANIFEST_CTL]),
    ("manifest: the audit is EXACTLY once (accept any number of runs)", MANIFEST_MOD,
     "             if seen.get(name, 0) != 1]",
     "             if False]",
     [MANIFEST_CTL]),
    ("manifest: the audit reports a control that ran and is not in the table (drop that half)",
     MANIFEST_MOD,
     '            + ["%s is not in the verified table" % name[:60] for name in sorted(seen)\n'
     '               if name not in known])',
     "            + [])",
     [MANIFEST_CTL]),
    ("manifest: the counts are of what RAN (count the verified table instead)", MANIFEST_MOD,
     "        out[kind] = out.get(kind, 0) + seen.get(name, 0)",
     "        out[kind] = out.get(kind, 0) + 1",
     [MANIFEST_CTL]),
    ("manifest: the wrapper COUNTS the control that ran (drop the record -- every control then "
     "reads as never executed)", MANIFEST_MOD,
     "        seen[name] = seen.get(name, 0) + 1",
     "        pass",
     [MANIFEST_CTL]),
    ("manifest: the snapshot enumerates the cases ONCE (take them twice -- the table collects its "
     "own)", MANIFEST_MOD,
     "    table = importlib.import_module(\"plan_memo_selftest_controls\").registry(case_rows)",
     "    table = importlib.import_module(\"plan_memo_selftest_controls\").registry()",
     [MANIFEST_CTL]),
    ("manifest: the collection step RECORDS its calls (drop the record -- the one-enumeration arm "
     "sees nothing)", REGISTRY,
     "    CALLS.append(listname)",
     "    pass",
     [MANIFEST_CTL]),
    ("manifest: a control's line carries its SOURCE digest (name and qualname alone -- a control "
     "re-pointed to a stub, or re-bodied, reads the same)", MANIFEST_MOD,
     '    return "%s.%s#%s" % (getattr(fn, "__module__", "?"), getattr(fn, "__qualname__", "?"),\n'
     '                         _source_digest(fn.__code__))',
     '    return "%s.%s" % (getattr(fn, "__module__", "?"), getattr(fn, "__qualname__", "?"))',
     [MANIFEST_CTL]),
    ("manifest: the digest is of the DEFINITION BLOCK (digest the whole module text instead -- "
     "every control in one file then reads the same)", MANIFEST_MOD,
     '        block = "".join(inspect.getblock(text.splitlines(True)[code.co_firstlineno - 1:]))',
     "        block = text",
     [MANIFEST_CTL]),
    ("manifest: the digest reads the text the module was EXEC'D from (read the file on disk "
     "always -- a patched control digests the unpatched text)", MANIFEST_MOD,
     "    text = harness.SOURCES.get(file)",
     "    text = None",
     [MANIFEST_CTL]),
    ("manifest: an unreachable source is an ERROR (fall back to a constant -- every such control "
     "reads the same)", MANIFEST_MOD,
     "        except OSError as e:\n            raise ManifestError(printable(",
     '        except OSError as e:\n            return "nosource"\n            raise ManifestError(printable(',
     [MANIFEST_CTL]),
    ("manifest: the snapshot is built ONCE per process (build it per call)", MANIFEST_MOD,
     "    if not _SNAP:\n        _SNAP.append(_build())",
     "    if True:\n        _SNAP.append(_build())",
     [MANIFEST_CTL]),
    ("manifest: the `|` between control names is ESCAPED (leave it -- a row naming `a|b` and one "
     "naming `a` and `b` read the same)", MANIFEST_MOD,
     '            .replace("|", "\\\\p"))',
     "            )",
     [MANIFEST_CTL]),
    ("manifest: a row's line carries its EDIT digest (drop it -- two rows' replacements can be "
     "swapped)", MANIFEST_MOD,
     '        edit = hashlib.sha256(("%s\\x00%s" % (find, replace)).encode("utf-8")).hexdigest()[:16]',
     '        edit = ""',
     [MANIFEST_CTL]),
    ("manifest: two rows repeating ONE edit are refused at generation (drop the check)",
     MANIFEST_MOD,
     '    bad += ["mutation rows repeat one edit in %s %d times" % (f, k)',
     '    bad += [] and ["mutation rows repeat one edit in %s %d times" % (f, k)',
     [MANIFEST_CTL]),
    ("manifest: an UNDECODABLE manifest is the one error (let it escape as a traceback)",
     MANIFEST_MOD,
     "    except (OSError, UnicodeDecodeError) as e:",
     "    except OSError as e:",
     [MANIFEST_CTL]),
    ("manifest: an EMPTY manifest is the one error (accept it -- every line then reads as removed)",
     MANIFEST_MOD,
     "    if not body:",
     "    if False:",
     [MANIFEST_CTL]),
    ("manifest: `lines` keeps DUPLICATES (drop them -- a row collected twice reads as one)",
     MANIFEST_MOD,
     "    return sorted(out)",
     "    return sorted(set(out))",
     [MANIFEST_CTL]),
    ("registry: a module holding an EMPTY list is refused (drop the check -- a list emptied "
     "elsewhere, or a name collision, contributes nothing and says nothing)", REGISTRY,
     "        if not frozen:",
     "        if False:",
     [MANIFEST_CTL]),
]
