"""Set member: T5 clauses 1-5 (scripts/tracked_tool_config.py), its listing, controls and mutants.

Plan of record: docs/plans/2026-10-python-pin-pr0-tracked-mise-config.md (sections 4.3 and 6);
clause 1, W1-W3 and the valid-world baseline: docs/plans/2026-10-python-pin-pr1-pin-and-gate.md (4.5)
and docs/plans/2026-10-python-pin-pr1-tests.md (6.4).

Outside the pin layer (plan D2): imports neither python_suites_fixtures nor python_suites_readers.
The file list is this member's own `git ls-files -z`, stdout only (plan D3): stderr is piped
separately and any stderr byte is a failure, never data. Controls are plain methods
`control_<id>` called by `test_<id>`, so the mutant test can run them again under a mutant.
The module is called as `t.f(...)` so unittest.mock.patch.object reaches the member's calls.
"""

import ast
import atexit
import importlib.util
import errno
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import tracked_tool_config as t  # noqa: E402

REPO = os.path.dirname(HERE)
MODULE_PATH = os.path.join(HERE, "tracked_tool_config.py")
MEMBER_PATH = os.path.abspath(__file__)
HELPER = "scripts/python_pin_testing.py"
_spec = importlib.util.spec_from_file_location("python_pin_testing_under_tool_config", os.path.join(REPO, *HELPER.split("/")))
H = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(H)
with open(os.path.join(REPO, ".tool-versions"), "rb") as _f:
    PIN_BYTES = _f.read()  # a malformed live pin reds every fixture world, which is the right answer


# ----------------------------------------------------------------------------
# The listing (plan section 4.3).


def checked_z_output(argv, returncode, stdout, stderr):
    if returncode != 0 or stderr:
        raise AssertionError(
            "%r exited %d with stderr: %s" % (list(argv), returncode, stderr.decode("utf-8", "replace"))
        )
    return [r for r in stdout.decode("utf-8", "surrogateescape").split("\0") if r]


_ISO_ROOT = []


def _without_git_env():
    """The environment of the member's git children: the helper's isolated MISE_* state (no MISE_* of
    the caller, the five isolated ones set) with the outer PATH kept (`keep_path=True`: a directory
    holding both `mise` and `git` must not take `git` away), minus GIT_*, so a mise shim named `git`
    reached through the outer PATH cannot write the user's mise state. One root for the process
    lifetime."""
    if not _ISO_ROOT:
        _ISO_ROOT.append(os.path.realpath(tempfile.mkdtemp(prefix="tracked-tool-config-mise-")))
        atexit.register(shutil.rmtree, _ISO_ROOT[0], True)
    env = H.isolated_mise_env(_ISO_ROOT[0], keep_path=True)
    env = {k: v for k, v in env.items() if not k.startswith("GIT_")}
    env.update(MISE_OFFLINE="1", MISE_AUTO_INSTALL="false")
    return env


def git_z(root, *args, run=subprocess.run):
    argv = ["git", "-C", root] + list(args)
    r = run(
        argv,
        env=_without_git_env(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=600,
    )
    return checked_z_output(argv, r.returncode, r.stdout, r.stderr)


def tracked_paths(root):
    return sorted(set(git_z(root, "ls-files", "-z")))


def _setup(cwd, *argv):
    subprocess.run(list(argv), cwd=cwd, env=_without_git_env(), check=True,
                   stdout=subprocess.PIPE, stderr=subprocess.PIPE)


# ----------------------------------------------------------------------------
# Fixtures.

OLD_DESCRIPTION = (
    'description = "Run every tracked Python suite (test_*.py), offline, on the `python3` '
    'first on PATH (floor: Python 3.9)."'
)
DRIVER = "scripts/python-suites.sh"
E3 = [
    ".miserc.toml", ".miserc", "miserc.toml", "sub/.miserc.toml",
    ".config/mise/miserc.toml", ".MISERC.TOML", "x/.miserc.d/y",
]
E4 = ["sub/.tool-versions", ".Tool-Versions"]
E6 = [".python-version", ".python-versions", "sub/.python-version", ".python-version.bak", ".PYTHON-VERSION"]
E7 = ["python-version", "tool-versions"]
E8_PATHS = [".config/mise/conf.d/py.toml", "mise.ci.toml", ".rtx.toml", "rtx.toml", "mise/tasks/x"]


def memory_read(files):
    def read(path):
        if path not in files:
            raise t.Unclassifiable("missing")
        return files[path]

    return read


def pinned(tracked, read):
    """The valid world: `tracked` plus the root pin file, and a `read` that answers it with the live bytes."""
    tracked = list(tracked)
    if t.ROOT_PIN not in tracked:
        tracked.append(t.ROOT_PIN)

    def read_pinned(path):
        if path == t.ROOT_PIN:
            return PIN_BYTES
        return read(path)

    return tracked, read_pinned


def probs(files, extra=()):
    """problems() over in-memory files {path: text-or-bytes}; `extra` = more tracked paths.

    Built on `pinned` unless `files` name the root pin file themselves (W1's clause-1 cases).
    """
    data = {p: (v.encode("utf-8") if isinstance(v, str) else v) for p, v in files.items()}
    tracked, read = list(data) + list(extra), memory_read(data)
    if t.ROOT_PIN not in data:
        tracked, read = pinned(tracked, read)
    return t.problems(tracked, read)


def mise_probs(text, tracked=()):
    return probs({"mise.toml": text}, extra=tracked)


# The only modules tracked_tool_config.py may import (no alias): none launches when imported, and
# from `python_pin` (which also holds `ask_mise`, a launcher) only the parser names are taken.
MODULE_IMPORTS = ("errno", "os", "re", "stat", "string", "python_pin")
PIN_IMPORTS = ("PinFileError", "parse_pin")
CONTROLS_2_TO_5 = ("C2", "C3", "C4", "C5", "L1", "L2", "mutants_are_killed")
CONTROLS_1 = ("W1", "W2", "W3", "clause1_mutants_are_killed")
CLAUSE_1_NAMES = ("root_pin_problems", "_REMEDY_PIN")


def _prefix(clause, path, line=None):
    return "T5 clause %s: %s" % (clause, path) + ("" if line is None else ":%d" % line)


class TrackedToolConfig(unittest.TestCase):
    def only(self, found, clause, path, line=None):
        self.assertEqual(len(found), 1, found)
        prefix = _prefix(clause, path, line)
        self.assertTrue(found[0].startswith(prefix + ":"), "%r does not start with %r" % (found[0], prefix))
        return found[0]

    # -- C0 ------------------------------------------------------------------

    def control_C0(self):
        trees = {}
        for p in (MODULE_PATH, MEMBER_PATH):
            with open(p, "rb") as f:
                trees[p] = ast.parse(f.read(), p)
        for p, tree in trees.items():
            for n in ast.walk(tree):
                mods = []
                if isinstance(n, ast.Import):
                    mods = [a.name for a in n.names]
                elif isinstance(n, ast.ImportFrom):
                    mods = [n.module or ""]
                for m in mods:
                    self.assertFalse(m.split(".")[0].startswith("python_suites"), "%s imports %s" % (p, m))
                    self.assertFalse(m.split(".")[0].startswith("test_python_suites"), "%s imports %s" % (p, m))
        mod = trees[MODULE_PATH]
        launchers = ("system", "popen", "fork", "posix_spawn", "execv", "execl", "spawn", "exec")
        for n in ast.walk(mod):
            if isinstance(n, ast.Import):
                for a in n.names:
                    self.assertIsNone(a.asname, "import alias %s" % a.name)
                    self.assertIn(a.name, MODULE_IMPORTS, "import outside the allow-list: %s" % a.name)
                    self.assertNotEqual(a.name, "python_pin", "import python_pin binds the module, ask_mise included")
            if isinstance(n, ast.ImportFrom):
                self.assertIn(n.module, MODULE_IMPORTS, "import outside the allow-list: %s" % n.module)
                for a in n.names:
                    self.assertIsNone(a.asname, "import alias %s" % a.name)
                    if n.module == "python_pin":
                        self.assertIn(a.name, PIN_IMPORTS, "python_pin name outside the parser: %s" % a.name)
                    if n.module == "os":
                        self.assertFalse(a.name.startswith(launchers), "os launch import %s" % a.name)
            if isinstance(n, ast.Attribute):
                self.assertFalse(
                    isinstance(n.value, ast.Name) and n.value.id in ("os", "subprocess")
                    and n.attr.startswith(launchers),
                    "os / subprocess launch in the predicate module",
                )
            self.assertFalse(isinstance(n, ast.Name) and n.id in ("TestCase", "__name__"), "TestCase / __main__ guard")
            self.assertFalse(isinstance(n, ast.Attribute) and n.attr == "TestCase")
            self.assertFalse(isinstance(n, ast.Constant) and n.value == "--self-test")
        # the seam: CLAUSES_2_TO_5 is the module's complete set of top-level names minus `problems` and clause 1's names
        defined = set()
        for n in mod.body:
            if isinstance(n, (ast.FunctionDef, ast.ClassDef)):
                defined.add(n.name)
            elif isinstance(n, (ast.Assign, ast.AnnAssign, ast.AugAssign)):
                targets = n.targets if isinstance(n, ast.Assign) else [n.target]
                for tg in targets:
                    defined.update(x.id for x in ast.walk(tg) if isinstance(x, ast.Name))
        outside = {"problems"} | set(CLAUSE_1_NAMES)
        self.assertTrue(outside <= defined, outside - defined)
        self.assertFalse(outside & set(t.CLAUSES_2_TO_5))
        self.assertEqual(set(t.CLAUSES_2_TO_5), defined - outside)
        self.assertEqual(len(t.CLAUSES_2_TO_5), len(set(t.CLAUSES_2_TO_5)))
        for cid in CONTROLS_2_TO_5 + CONTROLS_1:
            self.assertTrue(callable(getattr(type(self), "test_" + cid, None)), "no test_" + cid)

    def test_C0(self):
        self.control_C0()

    # -- C1 ------------------------------------------------------------------

    def control_C1(self, check=checked_z_output):
        argv = ["git", "ls-files", "-z"]
        same = b".miserc.toml\0.mise.toml\0mise.toml\0"
        self.assertEqual(check(argv, 0, same, b""), [".miserc.toml", ".mise.toml", "mise.toml"])
        with self.assertRaises(AssertionError) as cm:
            check(argv, 0, same, b"warning: x\n")
        self.assertIn("warning: x", str(cm.exception))
        with self.assertRaises(AssertionError) as cm:
            check(argv, 128, b"", b"fatal: y\n")
        self.assertIn("128", str(cm.exception))
        self.assertIn("fatal: y", str(cm.exception))
        out = check(argv, 0, b"mise\xff.toml\0", b"")
        self.assertEqual(len(out), 1)
        self.assertEqual(os.fsencode(out[0]), b"mise\xff.toml")

    def test_C1(self):
        self.control_C1()

    # -- C2 ------------------------------------------------------------------

    def control_C2(self):
        for p in E3 + E4 + E6 + E8_PATHS + ["Mise/notes.md", "mise.toml", ".tool-versions"]:
            self.assertTrue(t.is_member(p), p)
        green = E7 + [
            "..mise.toml",
            "crates/css/elidex-css/src/declaration/misc.rs",
            "scripts/tracked_tool_config.py",
            "scripts/test_tracked_tool_config.py",
        ]
        for p in green:
            self.assertFalse(t.is_member(p), p)
        self.assertFalse(t.is_member("crates/x/src/promise.rs"))  # `mise` inside a word is not a component prefix

    def test_C2(self):
        self.control_C2()

    # -- C3 ------------------------------------------------------------------

    def control_C3(self):
        for p in E3:  # r1
            self.only(probs({p: ""}), "3a", p)
        for p in E4:  # r2, with node-only and empty content
            self.only(probs({p: "node 20\n"}), "3b", p)
            self.only(probs({p: ""}), "3b", p)
        for p in E6:  # r3
            self.only(probs({p: "3.12\n"}), "3c", p)

        def boom(path):
            raise AssertionError("clause 3 read %s" % path)

        self.only(t.problems(*pinned([".miserc.toml"], boom)), "3a", ".miserc.toml")  # r4
        # g1: W1's green case -- the root .tool-versions holding the live line is green and read
        self.pin_green_and_read(PIN_BYTES)

    def pin_green_and_read(self, data):
        seen = []

        def read(path):
            seen.append(path)
            return data

        self.assertEqual(t.problems([t.ROOT_PIN], read), [])
        self.assertEqual(seen, [t.ROOT_PIN], "the root pin file was not read")

    def test_C3(self):
        self.control_C3()

    # -- C4 ------------------------------------------------------------------

    def control_C4(self):
        # r1: main's line 122 text is red at its line
        self.only(mise_probs("[tasks.tools-test]\n" + OLD_DESCRIPTION + "\n"), "4", "mise.toml", 2)
        # r2: python-spelling members
        e8 = {
            ".config/mise/conf.d/py.toml": '[tools]\npython = "3.12"\n',
            "mise.ci.toml": '[tools]\npython = "3.12"\n',
            ".rtx.toml": '[tools]\npython = "3.12"\n',
            "rtx.toml": '[tools]\npython = "3.12"\n',
            "mise/tasks/x": "#!/usr/bin/env bash\n#MISE tools=python@3.12\n",
        }
        for p, text in e8.items():
            self.only(probs({p: text}), "4", p, 2)
        for text in (
            '[env]\nMISE_PYTHON_VERSION = "3.12"\n',
            '[tools]\n"core:python" = "3.12"\n',
            "python.compile = true\n",
            '[env]\nPYTHONOPTIMIZE = "1"\n',
            "# Python\n",
            "PYTHON\n",
        ):
            f = mise_probs(text)
            self.assertEqual(len(f), 1, (text, f))
            self.assertTrue(f[0].startswith("T5 clause 4: mise.toml:"), f)
        # r3: substrings that are not the tracked path
        tracked = [DRIVER, "python.txt"]
        for line in (
            "run = ./scripts/python-suites.sh",
            "run = ubi:scripts/python-suites.sh",
            "run = scripts/python-suites.sh.bak",
            "run = scripts/python-suites.sh@1",
            "run = scripts/python_nope.sh",
            "python.txt = 1",
        ):
            self.only(mise_probs(line + "\n", tracked), "4", "mise.toml", 1)
        # r4
        self.only(probs({"scripts/mise_helper.py": "x = 'python'\n"}), "4", "scripts/mise_helper.py", 1)
        # r5: unclassifiable, read through worktree_reader
        with tempfile.TemporaryDirectory() as d:
            with open(os.path.join(d, "target.txt"), "w") as f:
                f.write("plain\n")
            os.symlink("target.txt", os.path.join(d, "mise.link.toml"))
            os.mkdir(os.path.join(d, "mise"))
            with open(os.path.join(d, "mise.ok.toml"), "w") as f:
                f.write("x = 1\n")
            with open(os.path.join(d, "mise.bad.toml"), "wb") as f:
                f.write(b"\xff\xfe")
            read = t.worktree_reader(d)
            cases = (
                ("mise.missing.toml", "missing", "git checkout --"),
                ("mise.link.toml", "symlink", "track the file itself as UTF-8 text"),
                ("mise", "not-a-file", "track the file itself as UTF-8 text"),
                ("mise.bad.toml", "non-UTF-8", "track the file itself as UTF-8 text"),
            )
            for p, kind, remedy in cases:
                f = self.only(t.problems(*pinned([p], read)), "4", p)
                self.assertIn("unclassifiable (%s)" % kind, f)
                self.assertIn(remedy, f)
            # unreadable: any OSError from the read maps to its own kind (open patched; a chmod is
            # no control when the suite runs as root)
            for exc, name in (
                (PermissionError(errno.EACCES, "denied"), "EACCES"),
                (IsADirectoryError(errno.EISDIR, "is a directory"), "EISDIR"),
            ):
                with mock.patch("builtins.open", side_effect=exc):
                    f = self.only(t.problems(*pinned(["mise.ok.toml"], read)), "4", "mise.ok.toml")
                self.assertIn("unclassifiable (unreadable: %s)" % name, f)
                self.assertIn("track the file itself as UTF-8 text", f)
        # a symlinked parent component: the file itself is a regular file, the path is not
        with tempfile.TemporaryDirectory() as d:
            os.mkdir(os.path.join(d, "real"))
            with open(os.path.join(d, "real", "mise.toml"), "w") as f:
                f.write("x = 1\n")
            os.symlink("real", os.path.join(d, "link"))
            read = t.worktree_reader(d)
            f = self.only(t.problems(*pinned(["link/mise.toml"], read)), "4", "link/mise.toml")
            self.assertIn("unclassifiable (symlink)", f)
            self.assertEqual(t.problems(*pinned(["real/mise.toml"], read)), [])
        # content holding NUL (BOM-less UTF-16 decodes as UTF-8) is non-text
        f = self.only(probs({"mise.toml": "x = 1\n".encode("utf-16-le")}), "4", "mise.toml")
        self.assertIn("unclassifiable (contains NUL)", f)
        self.assertIn("track the file itself as UTF-8 text", f)
        bad = "mise\udcff.toml"
        f = self.only(t.problems(*pinned([bad], memory_read({}))), "4", "mise\\udcff.toml")
        self.assertIn("unclassifiable (non-UTF-8)", f)
        # r6, r7
        self.only(mise_probs("python.compile = true\n"), "4", "mise.toml", 1)
        for line in (
            "  python.compile = false",
            "python.compile = false ",
            "python.compile = false # c",
            "python.compile=false",
            "python.github_attestations = false",
        ):
            self.only(mise_probs(line + "\n"), "4", "mise.toml", 1)
        for p in ("sub/mise.toml", ".mise.toml"):
            self.only(probs({p: "python.compile = false\n"}), "4", p, 1)
        # g2: path substrings are green while the path is tracked, red once it is not
        greens = (
            'run = "bash scripts/python-suites.sh"',
            "see `scripts/python-suites.sh`",
            "(scripts/python-suites.sh)",
            "ends at scripts/python-suites.sh.",
            "scripts/python-suites.sh:435",
            "scripts/python-suites.sh, then",
        )
        for line in greens:
            self.assertEqual(mise_probs(line + "\n", [DRIVER]), [], line)
            self.only(mise_probs(line + "\n"), "4", "mise.toml", 1)
        # g3: the admitted lines, LF and CRLF
        for eol in ("\n", "\r\n"):
            self.assertEqual(
                mise_probs("python.compile = false" + eol + "python.github_attestations = true" + eol), []
            )
        # g4: a non-Python tool is green
        self.assertEqual(probs({"sub/mise.toml": '[tools]\nnode = "20"\n'}), [])
        self.assertEqual(probs({"scripts/mise_helper.py": "x = 1\n"}), [])

    def test_C4(self):
        self.control_C4()

    # -- C5 ------------------------------------------------------------------

    def control_C5(self):
        long_line = 'x = "python' + "a" * 200 + '"'
        files = {
            ".miserc.toml": "",
            ".python-version": "3.12\n",
            "mise.toml": "\n" + 'a = "python"\n' + "\n" * 7 + long_line + "\n",
            "sub/mise.toml": 'b = "Python"\n',
        }
        tracked = list(files) + ["mise.missing.toml"]
        read = memory_read({p: v.encode("utf-8") for p, v in files.items()})
        self.assertEqual(t.problems(*pinned(list(reversed(tracked)), read)), t.problems(*pinned(tracked, read)))
        expect = (
            ("3a", ".miserc.toml", None, "rc file"),
            ("3c", ".python-version", None, "another tool goes into mise.toml [tools]"),
            ("4", "mise.missing.toml", None, "git checkout --"),
            ("4", "mise.toml", 2, "reword"),
            ("4", "mise.toml", 10, "reword"),
            ("4", "sub/mise.toml", 1, "reword"),
        )
        out = t.problems(*pinned(tracked, read))
        self.assertEqual(len(out), len(expect), out)
        for got, (clause, path, line, remedy) in zip(out, expect):
            prefix = _prefix(clause, path, line)
            self.assertTrue(got.startswith(prefix + ":"), (got, prefix))
            self.assertIn(" -- ", got)
            self.assertIn(remedy, got.split(" -- ", 1)[1])
        self.assertIn(long_line.strip()[:120], out[4])
        self.assertNotIn(long_line.strip()[:121], out[4])

    def test_C5(self):
        self.control_C5()

    # -- C6 ------------------------------------------------------------------

    def control_C6(self, launch=git_z):
        seen = {}

        def fake(argv, **kw):
            seen["argv"], seen["kw"] = argv, kw
            return subprocess.CompletedProcess(argv, 0, b"mise.toml\0", b"warning: x\n")

        err = None
        with mock.patch.dict(os.environ, {"GIT_DIR": "x"}):
            try:
                launch(REPO, "ls-files", "-z", run=fake)
            except AssertionError as e:
                err = e
        self.assertEqual(seen["kw"].get("stderr"), subprocess.PIPE, "stderr is merged into stdout")
        self.assertFalse([k for k in seen["kw"]["env"] if k.startswith("GIT_")])
        self.assertIsNotNone(err, "a stderr byte was accepted")
        self.assertIn("warning: x", str(err))

    def test_C6(self):
        self.control_C6()

    # -- L1 / L2 -------------------------------------------------------------

    def test_L1(self):
        listed = tracked_paths(REPO)
        self.assertTrue(listed)
        self.assertIn("mise.toml", listed)
        self.assertTrue(t.is_member("mise.toml"))
        self.assertEqual(t.problems(listed, t.worktree_reader(REPO)), [])
        rx = re.compile(r"[0-7]{6} [0-9a-f]{40,64} [0-3]\t([^\0]+)")
        staged = set()
        for rec in git_z(REPO, "ls-files", "-z", "--stage"):
            m = rx.fullmatch(rec)
            self.assertIsNotNone(m, rec)
            staged.add(m.group(1))
        self.assertEqual(staged, set(listed))

    def control_L2(self, lister=tracked_paths):
        with tempfile.TemporaryDirectory() as d:
            _setup(d, "git", "-c", "init.defaultBranch=main", "init", "-q")
            for name in (".miserc.toml", "mise.local.toml", ".python-version"):
                with open(os.path.join(d, name), "w") as f:
                    f.write("python 3.12\n")
            with open(os.path.join(d, t.ROOT_PIN), "wb") as f:
                f.write(PIN_BYTES)
            _setup(d, "git", "-c", "core.excludesFile=", "add", "-f", ".miserc.toml", t.ROOT_PIN)
            self.only(t.problems(lister(d), t.worktree_reader(d)), "3a", ".miserc.toml")

    def test_L2(self):
        self.control_L2()

    # -- W1: clause 1 --------------------------------------------------------

    def control_W1(self):
        # green: the live line with and without the final newline, and the root file is read
        self.pin_green_and_read(PIN_BYTES)
        self.pin_green_and_read(PIN_BYTES.rstrip(b"\n"))
        # red: not tracked (the file is readable, so only the tracking is wrong)
        found = t.problems(*pinned(["mise.toml"], memory_read({"mise.toml": b"x = 1\n"})))
        self.assertEqual(found, [])
        untracked = t.problems(["mise.toml"], lambda path: PIN_BYTES if path == t.ROOT_PIN else b"x = 1\n")
        self.assertIn("the root pin file is not tracked", self.only(untracked, "1", t.ROOT_PIN))
        # red: tracked but absent
        self.assertIn("unclassifiable (missing)", self.only(t.problems([t.ROOT_PIN], memory_read({})), "1", t.ROOT_PIN))
        # red: a symlink (its target holds a valid pin, so a reader that follows it reads green)
        with tempfile.TemporaryDirectory() as d:
            with open(os.path.join(d, "pin.txt"), "wb") as f:
                f.write(PIN_BYTES)
            os.symlink("pin.txt", os.path.join(d, t.ROOT_PIN))
            f = self.only(t.problems([t.ROOT_PIN], t.worktree_reader(d)), "1", t.ROOT_PIN)
            self.assertIn("unclassifiable (symlink)", f)
        # red: not UTF-8
        self.assertIn("non-UTF-8", self.only(probs({t.ROOT_PIN: b"python 3.15.2\n\xff"}), "1", t.ROOT_PIN))
        # red: N2's texts, through problems
        for text, _why in H.RED_PIN_TEXTS:
            f = self.only(probs({t.ROOT_PIN: text}), "1", t.ROOT_PIN)
            self.assertIn("not exactly one line", f, text)
            self.assertIn("another tool goes into mise.toml [tools]", f)

    def test_W1(self):
        self.control_W1()

    def control_git_env_keeps_git(self):
        # a directory holding both a `mise` and a `git` (distro /usr/bin): git must still resolve
        tmp = tempfile.mkdtemp(prefix="git-env-control-")
        self.addCleanup(shutil.rmtree, tmp, True)
        both = os.path.join(tmp, "both")
        os.makedirs(both)
        for name in ("mise", "git"):
            with open(os.path.join(both, name), "w") as f:
                f.write("#!/bin/sh\nexit 0\n")
            os.chmod(os.path.join(both, name), 0o755)
        with mock.patch.dict(os.environ, {"PATH": both + os.pathsep + os.environ.get("PATH", ""),
                                          "MISE_DATA_DIR": "/outer", "GIT_DIR": "/outer"}):
            env = _without_git_env()
        self.assertEqual(shutil.which("git", path=env["PATH"]), os.path.join(both, "git"))
        mise = {k: v for k, v in env.items() if k.startswith("MISE_") and k not in ("MISE_OFFLINE", "MISE_AUTO_INSTALL")}
        self.assertEqual(H.isolation_problems(mise, _ISO_ROOT[0]), [])
        self.assertEqual((env["MISE_OFFLINE"], env["MISE_AUTO_INSTALL"]), ("1", "false"))
        self.assertNotIn("GIT_DIR", env)
        self.assertNotEqual(env.get("MISE_DATA_DIR"), "/outer")

    def test_git_env_keeps_git(self):
        self.control_git_env_keeps_git()

    def test_git_env_mutant_is_killed(self):
        real = H.isolated_mise_env
        # mutant: the path-dropping env (keep_path ignored)
        with mock.patch.object(H, "isolated_mise_env", lambda root, **kw: real(root)):
            with self.assertRaises(AssertionError):
                self.control_git_env_keeps_git()

    def test_problems_names_a_newline_bearing_member_path(self):
        # F4: the findings are concatenated, never re-parsed; a newline or ": " in a path is a named finding.
        for path in ("mise\n.toml", "a: b/mise.toml"):
            out = probs({path: "python\n"})
            self.assertEqual(len(out), 1, out)
            self.assertTrue(out[0].startswith("T5 clause 4: " + path), out)
        # an iterator input is walked by both finders (F4 addendum): a valid tree is clean
        tracked, read = pinned(["mise.toml"], memory_read({"mise.toml": b"x = 1\n"}))
        self.assertEqual(t.problems(iter(tracked), read), [])
        # clause 1 precedes the members' findings
        out = probs({t.ROOT_PIN: "python 3.15.2 3.14.10\n", "mise\n.toml": "python\n"})
        self.assertEqual([f[:12] for f in out], ["T5 clause 1:", "T5 clause 4:"], out)

    # -- W2: the install settings over parsed TOML ---------------------------

    def control_W2(self, check=None):
        check = check or settings_problems
        with open(os.path.join(REPO, "mise.toml"), "rb") as f:
            live = f.read().decode("utf-8")
        self.assertEqual(check(live), [], "the live mise.toml's [settings] is not the admitted table")
        lines = "\n".join(t.ADMITTED_LINES) + "\n"
        self.assertEqual(check("[settings]\n" + lines), [])
        self.assertEqual(check("[settings.python]\ncompile = false\ngithub_attestations = true\n"), [])  # E22: parse-equal
        reds = {
            "absent table": '[tasks.check]\nrun = "x"\n',
            "E21 lines under a task": "[tasks.x]\n" + lines,
            "E21 lines before the table, then another": lines.replace(" = ", "=") + "[tasks.x]\n",
            "E23 compile flipped": "[settings]\npython.compile = true\npython.github_attestations = true\n",
            "E23 attestations flipped": "[settings]\npython.compile = false\npython.github_attestations = false\n",
            "extra settings.python key": "[settings]\n" + lines + "python.extra = 1\n",
            "missing settings.python key": "[settings]\npython.compile = false\n",
            "E24 extra settings key": "[settings]\n" + lines + "experimental = true\n",
            "settings not a table": "settings = 1\n",
            "TOML error": "[settings\n",
        }
        for name, text in reds.items():
            self.assertTrue(check(text), name)
        self.assertTrue(any("experimental" in m for m in check(reds["E24 extra settings key"])))

    def test_W2(self):
        self.control_W2()

    # -- W3: PR-1's writes x clause 4 ----------------------------------------

    def control_W3(self):
        text = W3_MISE_TOML
        self.assertEqual(mise_probs(text, [DRIVER]), [])  # the comment spells no path, so nothing to track
        self.assertEqual(settings_problems(text), [])
        # a pointer spelling the path is admitted only while that path is tracked
        with_path = text.replace("(docs/plans)", "(docs/plans/2026-10-python-pin-umbrella.md)")
        self.assertEqual(mise_probs(with_path, [DRIVER, "docs/plans/2026-10-python-pin-umbrella.md"]), [])
        self.only(mise_probs(with_path, [DRIVER]), "4", "mise.toml", 5)

    def test_W3(self):
        self.control_W3()

    # -- clause-1 mutants (kept apart from MUTANTS) --------------------------

    def test_clause1_mutants_are_killed(self):
        self.assertEqual(sorted(m[0] for m in WMUTANTS), ["WM%d" % i for i in (1, 2, 3, 4, 5, 6)])
        self._assert_all_killed(WMUTANTS)

    # -- mutants -------------------------------------------------------------

    def _assert_all_killed(self, table):
        for control in sorted({m[3] for m in table}):
            getattr(self, "control_" + control)()  # each killing control passes unpatched
        for mid, target, wrong, control, param in table:
            body = getattr(self, "control_" + control)
            killed = False
            try:
                if param is not None:
                    body(**{param: wrong})
                else:
                    with mock.patch.object(t, target, wrong):
                        body()
            except AssertionError:
                killed = True
            self.assertTrue(killed, "mutant %s survived %s" % (mid, control))

    def test_mutants_are_killed(self):
        self.assertEqual(sorted(m[0] for m in MUTANTS), sorted("M%d" % i for i in (1, 2, 3, 4, 5, 6, 8, 9, 10)))
        self._assert_all_killed(MUTANTS)


def settings_problems(text):
    """W2: the parsed `settings` table equals the one `ADMITTED_LINES` spells (any other key is red, named)."""
    try:
        doc = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        return ["mise.toml does not parse: %s" % e]
    expected = tomllib.loads("[settings]\n" + "\n".join(t.ADMITTED_LINES) + "\n")["settings"]
    got = doc.get("settings")
    if got == expected:
        return []
    if not isinstance(got, dict):
        return ["`settings` is not a table (expected %r)" % (expected,)]
    out = []
    for key in sorted(set(got) | set(expected)):
        if got.get(key) != expected.get(key):
            out.append("settings.%s is %r, expected %r" % (key, got.get(key), expected.get(key)))
    return out


# The post-PR-1 shape of the mise.toml pieces clause 4 reads: the [settings] block, its comment,
# the pointer to the umbrella, and the reworded task description / comment (live file: L1).
W3_MISE_TOML = (
    "# Install settings for the interpreter pinned in `.tool-versions`: an attested prebuilt\n"
    "# install or nothing (no source build, no unverified archive). This table sits before every\n"
    "# task so no later header can capture the lines. mise reads neither this table nor\n"
    "# `[bootstrap.packages]` in an untrusted worktree (`mise trust`; `main` already refuses so\n"
    "# since #535). Rationale and the invariant (I6): the interpreter-pin umbrella plan memo (docs/plans).\n"
    "[settings]\n"
    "python.compile = false\n"
    "python.github_attestations = true\n"
    "\n"
    "[tasks.tools-test]\n"
    'description = "Run every tracked suite (test_*.py), offline, on the pinned interpreter '
    '(`.tool-versions`; `mise run` installs it if missing)."\n'
    "# the interpreter pin (Step 1) and the wrapper contract live in scripts/python-suites.sh\n"
    'run = "bash scripts/python-suites.sh"\n'
)


# Wrong implementations. Each is a plausible shortcut, not a strawman.


def _walk_lister(root):
    out = []
    for base, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d != ".git"]
        out += [os.path.relpath(os.path.join(base, f), root).replace(os.sep, "/") for f in files]
    return sorted(out)


def _fold_case_sensitive(c):
    return c[1:] if c.startswith(".") else c


def _admits(line, s, tracked, need_slash, bounded):
    for p in tracked:
        if need_slash and "/" not in p:
            continue
        a = line.find(p)
        while a != -1:
            if a <= s and s + 6 <= a + len(p) and (not bounded or t._bounded(line, a, a + len(p))):
                return True
            a = line.find(p, a + 1)
    return False


def _no_slash_rule(line, s, tracked):
    return _admits(line, s, tracked, False, True)


def _no_boundaries(line, s, tracked):
    return _admits(line, s, tracked, True, False)


def _admitted_by_in(path, line):
    return path == t.ROOT_CONFIG and any(a in line for a in t.ADMITTED_LINES)


def _following_reader(root):
    def read(path):
        full = os.path.join(root, path)
        try:
            os.stat(full)
        except (FileNotFoundError, NotADirectoryError):
            raise t.Unclassifiable("missing")
        if os.path.isdir(full):
            raise t.Unclassifiable("not-a-file")
        with open(full, "rb") as f:
            return f.read()

    return read


def _merged_stderr(argv, returncode, stdout, stderr):
    if returncode != 0:
        raise AssertionError("%r exited %d" % (list(argv), returncode))
    return [r for r in (stdout + stderr).decode("utf-8", "surrogateescape").split("\0") if r]


def _replace_decode(argv, returncode, stdout, stderr):
    if returncode != 0 or stderr:
        raise AssertionError("%r exited %d: %s" % (list(argv), returncode, stderr))
    return [r for r in stdout.decode("utf-8", "replace").split("\0") if r]


def _git_z_merging_stderr(root, *args, run=subprocess.run):
    argv = ["git", "-C", root] + list(args)
    r = run(argv, env=_without_git_env(), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=600)
    return checked_z_output(argv, r.returncode, r.stdout, r.stderr or b"")


# (ID, target on the module or None, wrong implementation, killing control, parameter or None)
MUTANTS = (
    ("M1", None, _walk_lister, "L2", "lister"),
    ("M2", "fold", _fold_case_sensitive, "C3", None),
    ("M3", "admits_occurrence", _no_slash_rule, "C4", None),
    ("M4", "admits_occurrence", _no_boundaries, "C4", None),
    ("M5", "admitted_line", _admitted_by_in, "C4", None),
    ("M6", "worktree_reader", _following_reader, "C4", None),
    ("M8", None, _merged_stderr, "C1", "check"),
    ("M9", None, _replace_decode, "C1", "check"),
    ("M10", None, _git_z_merging_stderr, "C6", "launch"),
)


# Clause 1's mutants: (ID, target on the module or None, wrong implementation, killing control, parameter or None)


def _untracked_pin_passes(tracked, read):
    try:
        data = read(t.ROOT_PIN)
    except t.Unclassifiable as e:
        return [t.finding("1", t.ROOT_PIN, None, "unclassifiable (%s)" % e.kind, None, t._REMEDY_PIN)]
    try:
        t.parse_pin(data.decode("utf-8"))
    except (t.PinFileError, UnicodeDecodeError) as e:
        return [t.finding("1", t.ROOT_PIN, None, "bad: %s" % e, None, t._REMEDY_PIN)]
    return []


def _first_line_parse(text):
    first = text.split("\n")[0]
    if not re.fullmatch(r"python [0-9]+\.[0-9]+\.[0-9]+", first):
        raise t.PinFileError("first line")
    return tuple(int(x) for x in first.split(" ")[1].split("."))


def _text_search_settings(text):
    return [] if all(line in text for line in t.ADMITTED_LINES) else ["lines absent"]


def _any_table_settings(text):
    """WM6: admits the text when ANY table of the document equals the admitted one (a block under a task passes)."""
    expected = tomllib.loads("[settings]\n" + "\n".join(t.ADMITTED_LINES) + "\n")["settings"]

    def tables(node):
        if isinstance(node, dict):
            yield node
            for v in node.values():
                yield from tables(v)

    try:
        doc = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        return ["does not parse: %s" % e]
    return [] if any(tbl == expected for tbl in tables(doc)) else ["no table equals the admitted one"]


WMUTANTS = (
    ("WM1", "root_pin_problems", _untracked_pin_passes, "W1", None),
    ("WM2", "parse_pin", _first_line_parse, "W1", None),
    ("WM3", "worktree_reader", _following_reader, "W1", None),
    ("WM4", None, _text_search_settings, "W2", "check"),
    ("WM5", "ADMITTED_LINES", t.ADMITTED_LINES + ("python.extra = 1",), "W2", None),
    ("WM6", None, _any_table_settings, "W2", "check"),
)


if __name__ == "__main__":
    unittest.main()
