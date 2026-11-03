"""Set member: T5 clauses 2-5 (scripts/tracked_tool_config.py), its listing, controls and mutants.

Plan of record: docs/plans/2026-10-python-pin-pr0-tracked-mise-config.md (sections 4.3 and 6).

Outside the pin layer (plan D2): imports neither python_suites_fixtures nor python_suites_readers.
The file list is this member's own `git ls-files -z`, stdout only (plan D3): stderr is piped
separately and any stderr byte is a failure, never data. Controls are plain methods
`control_<id>` called by `test_<id>`, so the mutant test can run them again under a mutant.
The module is called as `t.f(...)` so unittest.mock.patch.object reaches the member's calls.
"""

import ast
import os
import re
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import tracked_tool_config as t  # noqa: E402

REPO = os.path.dirname(HERE)
MODULE_PATH = os.path.join(HERE, "tracked_tool_config.py")
MEMBER_PATH = os.path.abspath(__file__)


# ----------------------------------------------------------------------------
# The listing (plan section 4.3).


def checked_z_output(argv, returncode, stdout, stderr):
    if returncode != 0 or stderr:
        raise AssertionError(
            "%r exited %d with stderr: %s" % (list(argv), returncode, stderr.decode("utf-8", "replace"))
        )
    return [r for r in stdout.decode("utf-8", "surrogateescape").split("\0") if r]


def _without_git_env():
    return {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}


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


def probs(files, extra=()):
    """problems() over in-memory files {path: text-or-bytes}; `extra` = more tracked paths."""
    data = {p: (v.encode("utf-8") if isinstance(v, str) else v) for p, v in files.items()}
    return t.problems(list(data) + list(extra), memory_read(data))


def mise_probs(text, tracked=(), path="mise.toml"):
    return probs({path: text}, extra=tracked)


CONTROLS_2_TO_5 = ("C2", "C3", "C4", "C5", "L1", "L2", "mutants_are_killed")


class TrackedToolConfig(unittest.TestCase):
    def only(self, found, clause, path, line=None):
        self.assertEqual(len(found), 1, found)
        prefix = "T5 clause %s: %s" % (clause, path) + ("" if line is None else ":%d" % line)
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
                self.assertNotIn("subprocess", [a.name for a in n.names])
            if isinstance(n, ast.ImportFrom):
                self.assertNotEqual(n.module, "subprocess")
                if n.module == "os":
                    for a in n.names:
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
        names = {n.name for n in mod.body if isinstance(n, ast.FunctionDef)}
        for name in t.CLAUSES_2_TO_5:
            self.assertIn(name, names)
        self.assertNotIn("problems", t.CLAUSES_2_TO_5)
        for cid in CONTROLS_2_TO_5:
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
        promise = [p for p in tracked_paths(REPO) if "promise" in p.rsplit("/", 1)[-1].casefold()]
        self.assertGreaterEqual(len(promise), 1)
        for p in promise:
            self.assertFalse(t.is_member(p), p)

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

        self.only(t.problems([".miserc.toml"], boom), "3a", ".miserc.toml")  # r4
        # g1: the root .tool-versions is skipped (clause 1 is PR-1's) and never read
        self.assertEqual(t.problems([".tool-versions"], boom), [])

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
                f = self.only(t.problems([p], read), "4", p)
                self.assertIn("unclassifiable (%s)" % kind, f)
                self.assertIn(remedy, f)
        bad = "mise\udcff.toml"
        f = self.only(t.problems([bad], memory_read({})), "4", "mise\\udcff.toml")
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
        # g1: the live file (also L1)
        with open(os.path.join(REPO, "mise.toml"), "rb") as f:
            live = f.read()
        self.assertEqual(probs({"mise.toml": live}, extra=[DRIVER]), [])
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
        self.assertEqual(t.problems(list(reversed(tracked)), read), t.problems(tracked, read))
        expect = (
            ("3a", ".miserc.toml", None, "rc file"),
            ("3c", ".python-version", None, "another tool goes into mise.toml [tools]"),
            ("4", "mise.missing.toml", None, "git checkout --"),
            ("4", "mise.toml", 2, "reword"),
            ("4", "mise.toml", 10, "reword"),
            ("4", "sub/mise.toml", 1, "reword"),
        )
        out = t.problems(tracked, read)
        self.assertEqual(len(out), len(expect), out)
        for got, (clause, path, line, remedy) in zip(out, expect):
            prefix = "T5 clause %s: %s" % (clause, path) + ("" if line is None else ":%d" % line)
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
            _setup(d, "git", "add", ".miserc.toml")
            self.only(t.problems(lister(d), t.worktree_reader(d)), "3a", ".miserc.toml")

    def test_L2(self):
        self.control_L2()

    # -- mutants -------------------------------------------------------------

    def test_mutants_are_killed(self):
        self.assertEqual(sorted(m[0] for m in MUTANTS), sorted("M%d" % i for i in (1, 2, 3, 4, 5, 6, 8, 9, 10)))
        for mid, target, wrong, control, param in MUTANTS:
            body = getattr(self, "control_" + control)
            body()  # the killing control passes unpatched
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
        if (need_slash and "/" not in p) or p not in line:
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


if __name__ == "__main__":
    unittest.main()
