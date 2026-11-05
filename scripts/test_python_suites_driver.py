"""Driver and complement pins for scripts/python-suites.sh (slice A-iii; python-pin PR-1).

Plan of record: docs/plans/2026-07-citation-hygiene-Aiii-suite-scheduler.md, §6
(pins Q1-Q13) and §6.1 (Property -> pin); N1 / N1b / N1c / N22 and the DM / FM mutants are the
pins of docs/plans/2026-10-python-pin-pr1-tests.md (6.1). Every test method's name starts
with the §6 / §6.1 row it pins (`test_P4_...` pins row P4). Fixtures: see
`python_suites_fixtures.py`. Every driver-executing pin builds a scratch git
repository (never this file) and runs the copied driver there.
"""

from __future__ import annotations

import ast
import importlib.util
import os
import re
import shutil
import sys
import unittest
from unittest import mock

import python_suites_fixtures as fixtures
from python_suites_fixtures import (
    DRV,
    PIN_FILES,
    REPO,
    DriverCase,
    PASSING,
    _launch,
    base_env,
    fail_lines,
)

# The I7 helper (plan 4.8) is loaded by path through this one constant, joined to the repository root.
HELPER = "scripts/python_pin_testing.py"
H = importlib.util.module_from_spec(importlib.util.spec_from_file_location("python_pin_testing_under_driver_pin", str(REPO / HELPER)))
H.__spec__.loader.exec_module(H)

# ---------------------------------------------------------------------------
# §6 driver pins.


class TestDriverPins(DriverCase, unittest.TestCase):
    def test_Q1_every_member_named_with_nonzero_count(self):
        s = self.scratch(
            {
                "test_top.py": PASSING,
                "a/test_one.py": PASSING + "\n    def test_two(self):\n        pass\n",
                "b/c/test_two.py": PASSING,
                "b/c/helper.py": "X = 1\n",
            }
        )
        code, out = s.run()
        self.assertGreen((code, out))
        members = s.members()
        self.assertEqual(len(members), 3, members)
        for m in members:
            hit = re.search(r"^python-suites: ok   %s: (\d+) test\(s\) passed$" % re.escape(m), out, re.M)
            self.assertIsNotNone(hit, "%s not named with a count:\n%s" % (m, out))
            assert hit is not None
            self.assertGreater(int(hit.group(1)), 0)

    def test_Q2_failing_plant_in_a_lone_directory_is_red(self):
        s = self.scratch(
            {
                "a/test_ok.py": PASSING,
                "z/y/test_planted.py": """
                    import unittest


                    class Planted(unittest.TestCase):
                        def test_fails(self):
                            self.fail("planted")
                """,
            }
        )
        self.assertRed(s.run(), "z/y/test_planted.py::Planted.test_fails")

    def test_Q3_member_without_tests_beside_a_passing_sibling_is_red(self):
        s = self.scratch({"a/test_ok.py": PASSING, "a/test_empty.py": "def helper():\n    return 1\n"})
        out = self.assertRed(s.run(), "a/test_empty.py: no test executed and passed")
        self.assertNotNamed(out, "a/test_ok.py")

    def test_Q3_empty_set_is_red(self):
        s = self.scratch({"a/x.py": "X = 1\n"})
        self.assertRed(s.run(), "set:")

    def test_Q9_non_recursive_every_test_id_exactly_once(self):
        s = self.scratch(
            {
                "a/test_parent.py": PASSING.replace("test_passes", "test_parent_case"),
                "a/pkg/__init__.py": "",
                "a/pkg/test_child.py": PASSING.replace("test_passes", "test_child_case"),
            }
        )
        code, out = s.run()
        self.assertGreen((code, out))
        for method in ("test_parent_case", "test_child_case"):
            hits = re.findall(r"^python-suites: test ok .*\.%s$" % method, out, re.M)
            self.assertEqual(len(hits), 1, "%s ran %d times:\n%s" % (method, len(hits), out))

    def test_Q12_skip_is_red_naming_the_test(self):
        s = self.scratch(
            {
                "a/test_skips.py": """
                    import unittest


                    class S(unittest.TestCase):
                        def test_passes(self):
                            pass

                        @unittest.skip("planted")
                        def test_skipped(self):
                            pass
                """
            }
        )
        self.assertRed(s.run(), "SKIPPED a/test_skips.py::S.test_skipped")

    def test_Q12_expected_failure_and_unexpected_success_are_red(self):
        s = self.scratch(
            {
                "a/test_xfail.py": """
                    import unittest


                    class X(unittest.TestCase):
                        def test_passes(self):
                            pass

                        @unittest.expectedFailure
                        def test_xfail(self):
                            self.fail("x")

                        @unittest.expectedFailure
                        def test_xpass(self):
                            pass
                """
            }
        )
        self.assertRed(
            s.run(),
            "EXPECTED-FAILURE a/test_xfail.py::X.test_xfail",
            "UNEXPECTED-SUCCESS a/test_xfail.py::X.test_xpass",
        )


# ---------------------------------------------------------------------------
# §6.1 P-rows: the complement (step 3).


# ---------------------------------------------------------------------------
# Step 1: exact admission (N1, N1b, N1c), the isolation of the pins' children (N22), and the
# driver / fixture mutants (DM1-DM5, FM1).

# The pin and the versions refused beside it, both derived from the `.tool-versions` text (the
# live file at import; a scratch tree's own copy through `pin_of`): this file spells the pin no
# second time, and does not import python_pin (P6).
_PIN_LINE = re.compile(r"python ([0-9]+)\.([0-9]+)\.([0-9]+)\n?")


def pin_of(text):
    m = _PIN_LINE.fullmatch(text)
    if m is None:
        raise AssertionError("not a one-line `python X.Y.Z` pin: %r" % text)
    return tuple(int(g) for g in m.groups())


def refused_versions(pin):
    """E2's versions (the helper's derivation from the pin triple): every neighbour of `pin` is refused."""
    return H.refused_versions(pin)


PIN_TEXT = (REPO / ".tool-versions").read_text(encoding="utf-8")
PIN = pin_of(PIN_TEXT)
PIN_DOTTED = "%d.%d.%d" % PIN
REFUSED_VERSIONS = refused_versions(PIN)

# Source mutants over the scratch copies: (id, file, find, replace, killing control). The find text
# occurs exactly once in the live file it mutates (a stale row is red).
STEP1_CALL = 'python3 -I -S "$root/scripts/python_pin.py"'
FAIL_BRANCH = '  fi\n  exit 1\nfi\n'
DRIVER_MUTANTS = (
    (
        "DM1",
        "scripts/python-suites.sh",
        STEP1_CALL,
        "python3 -I -S -c 'import sys; sys.exit(0 if tuple(sys.version_info[:2]) >= (3, 9) else 1)'",
        "refusals",
    ),
    (
        "DM2",
        "scripts/python-suites.sh",
        STEP1_CALL,
        "python3 -I -S -c 'import sys; sys.exit(0 if tuple(sys.version_info[:3]) == %r else 1)'" % (PIN,),
        "refusals",
    ),
    ("DM3", "scripts/python-suites.sh", STEP1_CALL, "true", "refusals"),
    (
        "DM4",
        "scripts/python-suites.sh",
        FAIL_BRANCH,
        "  fi\nfi\n",
        "refusals",
    ),
    # DM6: DM4's mutation, over a tree whose suite then fails by itself (see MUTANT_EXTRA_FILES): the
    # run is non-zero and names the pin, so only the `member(s) in` assertion tells it from a refusal.
    (
        "DM6",
        "scripts/python-suites.sh",
        FAIL_BRANCH,
        "  fi\nfi\n",
        "refusals",
    ),
    # DM7: the first-branch text replaced by the second branch's, always.
    (
        "DM7",
        "scripts/python-suites.sh",
        'if [ "$pin_last" = "python-pin: exit $pin_rc" ]; then',
        "if false; then",
        "refusals",
    ),
    (
        "DM5",
        "scripts/python_pin.py",
        "if admitted(sys.version_info, pin):",
        "if admitted(sys.version_info, %r):" % (PIN,),
        "spelling",
    ),
)


MUTANT_EXTRA_FILES = {
    "DM6": {"z/test_fails.py": "import unittest\n\n\nclass T(unittest.TestCase):\n    def test_fails(self):\n        self.fail('x')\n"},
}


def _old_base_env():
    """`base_env` as it was at 660e594c: the outer PATH and MISE_* kept (FM1's wrong implementation)."""
    return {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}


class TestStep1Pins(DriverCase, unittest.TestCase):
    def pin_scratch(self, extra=None):
        return self.scratch(dict({"a/test_ok.py": PASSING}, **(extra or {})))

    def fakes(self, answers=None):
        """A directory B holding the fake python3 and the fake mise (answering `[]`)."""
        b = self.tmpdir("python-suites-fakes-")
        H.write_fake_mise(b, answers or {"*": {"rc": 0, "stdout": "[]"}})
        return b

    def step1_env(self, b):
        """N1's environment, exactly: base_env() minus all its MISE_* (MISE_OFFLINE and MISE_AUTO_INSTALL
        included), plus the helper's five MISE_* variables, B first on PATH."""
        tmp_root = os.path.realpath(str(self.tmpdir("python-suites-mise-")))
        e = {k: v for k, v in base_env().items() if not k.startswith("MISE_")}
        e.update(kv for kv in H.isolated_mise_env(tmp_root, fake_mise=b).items() if kv[0].startswith("MISE_"))
        e["PATH"] = str(b) + os.pathsep + e["PATH"]
        return e, tmp_root

    def launch(self, s, env):
        return _launch(["bash", s.dir / "scripts" / "python-suites.sh"], env=env, cwd=s.dir, purpose="driver")

    def refusals(self, s, versions=None, only_first=False):
        """N1's refusal cases on scratch tree `s` (live or mutated); versions derived from its pin text."""
        if versions is None:
            versions = refused_versions(pin_of((s.dir / ".tool-versions").read_text(encoding="utf-8")))
            versions = versions[:1] if only_first else versions
        b = self.fakes()
        env, tmp_root = self.step1_env(b)
        root = os.path.realpath(str(s.dir))
        for spec, shown, level in versions:
            H.write_fake_python3(b, spec)
            self.assertEqual(shutil.which("python3", path=env["PATH"]), os.path.join(str(b), "python3"))
            self.assertEqual(shutil.which("mise", path=env["PATH"]), os.path.join(str(b), "mise"))
            code, out = self.launch(s, env)
            self.assertNotEqual(code, 0, out)
            self.assertRed((code, out), "pin")
            self.assertIn("FAIL pin: scripts/python_pin.py refused or failed (exit 1; its output is above)", out)
            self.assertNotIn("did not end with", out)
            self.assertIn("pin: %s (%s)" % (PIN_DOTTED, os.path.join(root, ".tool-versions")), out)
            self.assertIn("running: %s %s" % (shown, level), out)
            self.assertNotIn("member(s) in", out)
        calls = H.read_calls(b)
        self.assertTrue(calls, "the refusal never asked the fake mise")
        for call in calls:
            e = call["environ"]
            self.assertEqual(H.isolation_problems(e, tmp_root), [])
            self.assertEqual([k for k in e if k.startswith("GIT_")], [])
            self.assertEqual(e["PATH"], env["PATH"])

    def spelling(self, s):
        """N1's spelling control: the scratch pin rewritten to the fake's version admits it."""
        b = self.fakes()
        env, _ = self.step1_env(b)
        other = REFUSED_VERSIONS[0][0]
        H.write_fake_python3(b, other)
        (s.dir / ".tool-versions").write_text("python %s\n" % other, encoding="utf-8")
        self.assertGreen(self.launch(s, env))

    def test_N1_step1_admits_exactly_the_pin(self):
        s = self.pin_scratch()
        self.refusals(s)
        self.spelling(s)

    def test_N1b_no_python3_on_path(self):
        s = self.pin_scratch()
        bindir = self.tmpdir("python-suites-utils-")
        for name in ("dirname", "basename", "readlink"):
            os.symlink(shutil.which(name, path=fixtures.CHILD_PATH), str(bindir / name))
        env = base_env()
        env["PATH"] = str(bindir)
        code, out = _launch(["/bin/bash", s.dir / "scripts" / "python-suites.sh"], env=env, cwd=s.dir, purpose="driver")
        self.assertRed((code, out), "pin: no python3 on PATH")
        self.assertNotIn("member(s) in", out)

    def test_N1c_malformed_pin_stops_the_run(self):
        s = self.pin_scratch()
        (s.dir / ".tool-versions").write_text("python  %s\n" % PIN_DOTTED, encoding="utf-8")
        code, out = s.run()
        self.assertRed((code, out), "pin")
        self.assertIn(".tool-versions", out)
        self.assertNotIn("member(s) in", out)

    N22_PARTS = ("scratch_run", "direct_launch", "isolation", "bash_env")

    def control_N22(self, parts=N22_PARTS):
        sentinel = self.tmpdir("python-suites-sentinel-")
        H.write_sentinel(sentinel, ("mise", "python3"))
        marker = self.tmpdir("python-suites-bashenv-") / "hit"
        startup = marker.parent / "startup.sh"
        startup.write_text("echo hit > '%s'\n" % marker, encoding="utf-8")
        patched = {
            "PATH": str(sentinel) + os.pathsep + os.environ.get("PATH", ""),
            "MISE_PIN_SENTINEL": "1",
            "BASH_ENV": str(startup),
            "ENV": str(startup),
            "BASH_FUNC_dirname%%": "() { echo hit > '%s'; command dirname \"$@\"; }" % marker,
        }
        with mock.patch.dict(os.environ, patched):
            if "scratch_run" in parts:
                self.assertGreen(self.pin_scratch().run())
            if "direct_launch" in parts:
                s = self.pin_scratch()
                self.assertGreen(
                    _launch(["bash", s.dir / "scripts" / "python-suites.sh"], env=fixtures.base_env(), cwd=s.dir, purpose="driver")
                )
            if "bash_env" in parts:
                self.assertGreen(self.pin_scratch().run())
                self.assertFalse(marker.exists(), "a bash start-up file or exported function reached a scratch driver")
            if "isolation" in parts:
                e = fixtures.base_env()
                root = os.path.realpath(fixtures.mise_isolation_root()) + os.sep
                self.assertEqual(
                    sorted(k for k in e if k.startswith("MISE_")),
                    sorted(H.MISE_VARS + ("MISE_OFFLINE", "MISE_AUTO_INSTALL")),
                )
                self.assertEqual(
                    sorted([n for n, _ in fixtures.MISE_DIR_VARS] + [fixtures.MISE_GLOBAL_VAR]), sorted(H.MISE_VARS)
                )
                for k in H.MISE_VARS:
                    self.assertTrue(os.path.realpath(e[k]).startswith(root), (k, e[k]))
                self.assertEqual(os.path.getsize(e[fixtures.MISE_GLOBAL_VAR]), 0)
                self.assertEqual((e["MISE_OFFLINE"], e["MISE_AUTO_INSTALL"]), ("1", "false"))
                self.assertEqual([k for k in e if k in ("BASH_ENV", "ENV") or k.startswith(("GIT_", "BASH_FUNC_"))], [])
            self.assertEqual(H.read_sentinel(sentinel), [], "a child of the pins reached the sentinel")
            # The constant itself (FM1 leaves it): python3 on CHILD_PATH is the interpreter running the pins.
            self.assertEqual(
                os.path.realpath(shutil.which("python3", path=fixtures.CHILD_PATH)), os.path.realpath(sys.executable)
            )

    def test_N22_pin_children_cannot_reach_mise(self):
        self.control_N22()

    def test_N22_a_reachable_mise_is_isolated_not_raised(self):
        # A `mise` in any directory of CHILD_PATH (a distro package in /usr/bin): base_env does not
        # raise; whatever that mise reads and writes is confined to the isolation root.
        d = self.tmpdir("python-suites-withmise-")
        H.write_sentinel(d, ("mise",))
        for dirs in ((str(d), "/usr/bin", "/bin"), ("/usr/bin", str(d), "/bin"), ("/usr/bin", "/bin", str(d))):
            with mock.patch.object(fixtures, "CHILD_PATH", os.pathsep.join(dirs)):
                e = fixtures.base_env()
            self.assertEqual(e["PATH"], os.pathsep.join(dirs))
            root = os.path.realpath(fixtures.mise_isolation_root()) + os.sep
            for k in H.MISE_VARS:
                self.assertTrue(os.path.realpath(e[k]).startswith(root), (k, e[k]))
            self.assertEqual((e["MISE_OFFLINE"], e["MISE_AUTO_INSTALL"]), ("1", "false"))

    def test_F6_a_python3_failing_before_python_pin_ran_is_not_blamed_on_it(self):
        # A python3 that exits 1 without python_pin's final stderr marker (a mise shim refusing an untrusted config).
        s = self.pin_scratch()
        b = self.tmpdir("python-suites-failpy-")
        py = b / "python3"
        py.write_text("#!/bin/sh\necho 'mise ERROR untrusted config' >&2\nexit 1\n", encoding="utf-8")
        py.chmod(0o755)
        e = base_env()
        e["PATH"] = str(b) + os.pathsep + e["PATH"]
        code, out = self.launch(s, e)
        self.assertNotEqual(code, 0, out)
        self.assertIn("mise ERROR untrusted config", out)
        self.assertIn("FAIL pin: python3 did not end with scripts/python_pin.py's exit line (exit 1)", out)
        self.assertNotIn("refused or failed", out)
        self.assertNotIn("member(s) in", out)

    def test_DM_FM_mutants_are_killed(self):
        self.assertEqual([m[0] for m in DRIVER_MUTANTS], ["DM1", "DM2", "DM3", "DM4", "DM6", "DM7", "DM5"])
        # Each killing control passes unmutated.
        live = self.pin_scratch()
        self.refusals(live, only_first=True)
        self.spelling(live)
        self.control_N22()
        for mid, rel, find, replace, control in DRIVER_MUTANTS:
            text = (REPO / rel).read_text(encoding="utf-8")
            self.assertEqual(text.count(find), 1, "stale row %s: its find text must occur exactly once in %s" % (mid, rel))
            s = self.pin_scratch(MUTANT_EXTRA_FILES.get(mid))
            (s.dir / rel).write_text(text.replace(find, replace), encoding="utf-8")
            with self.assertRaises(AssertionError, msg="%s survived" % mid):
                if control == "refusals":
                    self.refusals(s, only_first=True)
                else:
                    self.spelling(s)
        # FM1: base_env restored to 660e594c's body; N22's four parts (Scratch run, direct
        # launch, isolation, bash start-up) each kill it alone (the patch is on the module object).
        for part in self.N22_PARTS:
            with mock.patch.object(fixtures, "base_env", _old_base_env):
                with self.assertRaises(AssertionError, msg="FM1 survived N22's %s" % part):
                    self.control_N22((part,))


class TestComplementPins(DriverCase, unittest.TestCase):
    def test_P1_testcase_outside_the_set_is_red(self):
        s = self.scratch(
            {
                "ok/test_ok.py": PASSING,
                "p1a/helper.py": "import unittest\n\n\nclass A(unittest.TestCase):\n    pass\n",
                "p1b/helper.py": "from unittest import TestCase as T\n\n\nclass A(T):\n    pass\n",
                "p1c/helper.py": "import unittest\n\n\nclass A(unittest.IsolatedAsyncioTestCase):\n    pass\n",
                "p1d/helper.py": "import unittest\n\n\nclass B(unittest.TestCase):\n    pass\n\n\nclass C(B):\n    pass\n",
                "p1e/helper.py": "class Foo:\n    pass\n\n\nclass A(Foo):\n    pass\n",
            }
        )
        out = self.assertRed(
            s.run(),
            "p1a/helper.py defines",
            "p1b/helper.py defines",
            "p1c/helper.py defines",
            "p1d/helper.py defines unittest.TestCase subclass(es) B (line 4), C (line 8)",
        )
        self.assertNotNamed(out, "p1e/helper.py")

    def test_P2_base_through_another_file_is_the_stated_limit(self):
        s = self.scratch(
            {
                "ok/test_ok.py": PASSING,
                "p2/base.py": "import unittest\n\n\nclass MyCase(unittest.TestCase):\n    pass\n",
                "p2/helper.py": "from base import MyCase\n\n\nclass A(MyCase):\n    pass\n",
            }
        )
        out = self.assertRed(s.run(), "p2/base.py defines")
        # The stated limit of 3(a): a base reaching TestCase only through
        # another file is outside the in-file resolution.
        self.assertNotNamed(out, "p2/helper.py")

    def test_P3_entry_point_predicate_is_syntactic(self):
        guard = 'if __name__ == "__main__":\n    main()\n'
        s = self.scratch(
            {
                "ok/test_ok.py": PASSING,
                "a/tool.py": 'FLAG = "--self-test"\n\n\ndef main():\n    pass\n\n\n' + guard,
                "b/tool.py": 'FLAG = "--self-test"\n',
                "c/tool.py": "def main():\n    pass\n\n\n" + guard,
                "d/tool.py": "def main():\n    pass  # --self-test\n\n\n" + guard,
                "e/tool.py": 'def main():\n    if __name__ == "__main__":\n        return "--self-test"\n',
            }
        )
        out = self.assertRed(s.run(), "a/tool.py is a --self-test entry point")
        self.assertNotNamed(out, "b/tool.py", "c/tool.py", "d/tool.py", "e/tool.py")

    def test_P4_named_by_path_one_constant(self):
        tool = 'if __name__ == "__main__":\n    print("--self-test")\n'
        s = self.scratch(
            {
                "a/tool.py": tool,
                "a/test_tool.py": PASSING + '\nTOOL = "tool.py"\n',
                "b/tool.py": tool,
                "c/tool.py": tool,
                "m/test_m.py": PASSING + '\nTOOL = "c/tool.py"\n',
                "d/tool.py": tool,
                "n/test_n.py": PASSING + '\nTOOL = "d/" + "tool.py"\n',
            }
        )
        out = self.assertRed(
            s.run(),
            "b/tool.py is a --self-test entry point",
            "d/tool.py is a --self-test entry point",
        )
        self.assertNotNamed(out, "a/tool.py", "c/tool.py")

    def test_P5_members_are_excluded_from_3a_and_3b(self):
        s = self.scratch(
            {
                "a/test_wrap.py": PASSING
                + '\nFLAG = "--self-test"\n\nif __name__ == "__main__":\n    unittest.main()\n'
            }
        )
        self.assertGreen(s.run())

    def test_P6_pins_file_never_credits_a_live_entry_point(self):
        code, out = _launch(
            ["git", "-C", REPO, "ls-files", "-z", "--", "*.py"], env=base_env(), purpose="git"
        )
        self.assertEqual(code, 0, out)
        tracked = sorted(p for p in out.split("\0") if p)
        for rel in PIN_FILES:
            self.assertIn(rel, tracked)
        trees = {p: DRV.parse((REPO / p).read_bytes(), p) for p in tracked}
        constants = set()
        for rel in PIN_FILES:
            constants |= DRV.string_constants(trees[rel])
        # Completeness, by property: a tracked file a pin file imports, and a file that imports a
        # helper module of the pins, are pin files too. An import statement's dotted names (and
        # `from M import n` as M.n) resolve against the importer's directory, scripts/ and the
        # root, as a module or a package. Outside the check: a relative import (scripts/ is not a
        # package) and a module loaded by a computed name (importlib, __import__).
        def imports(rel, tree=None, known=frozenset(tracked)):
            names = set()
            for n in ast.walk(trees[rel] if tree is None else tree):
                if isinstance(n, ast.Import):
                    names |= {a.name for a in n.names}
                elif isinstance(n, ast.ImportFrom) and not n.level and n.module:
                    names |= {n.module} | {n.module + "." + a.name for a in n.names}
            bases = {os.path.dirname(rel) + "/", "scripts/", ""} - {"/"}
            dotted = {".".join(d.split(".")[:i]) for d in names for i in range(1, d.count(".") + 2)}
            found = set()
            for b in bases:
                for d in dotted:
                    stem = b + d.replace(".", "/")
                    found |= {stem + ".py", stem + "/__init__.py"} & known
            return found - {rel}

        # Control: a helper in a subdirectory and a scripts.-qualified import both resolve.
        planted = ast.parse("from pins.launch import go\nfrom scripts import python_suites_fixtures\n")
        self.assertEqual(
            imports("scripts/p.py", planted, set(tracked) | {"scripts/pins/launch.py"}),
            {"scripts/pins/launch.py", "scripts/python_suites_fixtures.py"},
        )

        helpers = set()
        for rel in PIN_FILES:
            for dep in imports(rel):
                self.assertIn(dep, PIN_FILES, "%s imports %s, which is not a pin file" % (rel, dep))
                helpers.add(dep)
        self.assertLessEqual({"scripts/python_suites_fixtures.py", "scripts/python_suites_readers.py"}, helpers)
        for path in tracked:
            self.assertTrue(path in PIN_FILES or not imports(path) & helpers, "%s imports a pin helper but is not a pin file" % path)
        entry_points = []
        for path in tracked:
            if re.search(DRV.MEMBER_RE, path):
                continue  # members are not entry points for 3(b)
            if DRV.is_self_test_entry_point(trees[path]):
                entry_points.append(path)
        for e in entry_points:
            for rel in PIN_FILES:
                self.assertFalse(
                    DRV.names_by_path(constants, e, rel),
                    "the pins credit live entry point %s (checked from %s)" % (e, rel),
                )
        # The planted file is the positive control: without it, a predicate that is False
        # everywhere would pass the live-tree loop above vacuously.
        planted = 'FLAG = "--self-test"\n\nif __name__ == "__main__":\n    pass\n'
        self.assertTrue(DRV.is_self_test_entry_point(DRV.parse(planted.encode(), "planted.py")))
        for no in ('if __name__ == "__main__":\n    pass\n', 'FLAG = "--self-test"\n'):
            self.assertFalse(DRV.is_self_test_entry_point(DRV.parse(no.encode(), "planted.py")), no)
        # Controls: the same predicate does credit, root- and member-dir-relative.
        self.assertTrue(DRV.names_by_path(constants | {"x/tool.py"}, "x/tool.py", PIN_FILES[0]))
        self.assertTrue(DRV.names_by_path({"tool.py"}, "scripts/tool.py", PIN_FILES[0]))
        self.assertFalse(DRV.names_by_path({"tool.py"}, "x/tool.py", PIN_FILES[0]))

    def test_P7_unparsable_tracked_py_is_red_member_or_not(self):
        s = self.scratch(
            {"ok/test_ok.py": PASSING, "bad.py": "def (:\n", "q/test_bad.py": "class (:\n"}
        )
        self.assertRed(s.run(), "bad.py does not parse", "q/test_bad.py does not parse")

    def test_P8_pathspec_environment_cannot_empty_the_population(self):
        # GIT_NOGLOB_PATHSPECS / GIT_GLOB_PATHSPECS reinterpret `*.py`; the driver's git
        # runs without any GIT_* and with `:(glob)**/*.py`, so the non-member is found.
        s = self.scratch(
            {
                "ok/test_ok.py": PASSING,
                "p8/helper.py": "import unittest\n\n\nclass A(unittest.TestCase):\n    pass\n",
            }
        )
        for var in ("GIT_NOGLOB_PATHSPECS", "GIT_GLOB_PATHSPECS", "GIT_LITERAL_PATHSPECS"):
            out = self.assertRed(s.run({var: "1"}), "p8/helper.py defines")
            self.assertNotIn(" 0 tracked .py parsed", out)

    def test_P9_every_unittest_import_form_is_resolved_or_red(self):
        tc = "\n\nclass A(%s):\n    pass\n"
        forms = {
            "p9a": "import unittest.mock" + tc % "unittest.TestCase",
            "p9b": "import unittest.util" + tc % "unittest.IsolatedAsyncioTestCase",
            "p9c": "import unittest.async_case" + tc % "unittest.async_case.IsolatedAsyncioTestCase",
            "p9d": "import unittest.case as c" + tc % "c.TestCase",
            "p9e": "from unittest import mock, async_case as ac" + tc % "ac.IsolatedAsyncioTestCase",
            "p9f": "from unittest.async_case import IsolatedAsyncioTestCase" + tc % "IsolatedAsyncioTestCase",
            "p9g": "import unittest\nT: type = unittest.TestCase" + tc % "T",
            "p9h": "import unittest\nT = U = unittest.TestCase" + tc % "U",
        }
        files = {"ok/test_ok.py": PASSING, "p9z/helper.py": "import unittest.mock\n\n\nclass Plain:\n    pass\n"}
        files.update({d + "/helper.py": src for d, src in forms.items()})
        files["p9x/helper.py"] = 'import importlib\n\nu = importlib.import_module("unittest.case")\n'
        files["p9y/helper.py"] = '__import__("unittest")\n'
        out = self.assertRed(
            self.scratch(files).run(),
            *[d + "/helper.py defines" for d in forms],
            "p9x/helper.py imports unittest in a form step 3(a) cannot resolve",
            "p9y/helper.py imports unittest in a form step 3(a) cannot resolve",
        )
        self.assertNotNamed(out, "p9z/helper.py")

    def test_P10_member_is_loaded_from_its_own_path(self):
        # The live _webref idiom: a member puts its parent directory on sys.path; a
        # later member of the same root, imported by name, would resolve to the
        # same-named file of another root and the real file would never run.
        s = self.scratch(
            {
                "t/w/test_a.py": PASSING
                + "\nimport os, sys\nsys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))\n",
                "t/w/test_refresh.py": """
                    import unittest


                    class Real(unittest.TestCase):
                        def test_fails(self):
                            self.fail("the real member ran")
                """,
                "t/test_refresh.py": PASSING,
            }
        )
        self.assertRed(s.run(), "t/w/test_refresh.py::Real.test_fails")

    def test_P11_function_kind_by_class_eight_cells(self):
        # function kind (plain / coroutine / generator / async generator) x class
        # (TestCase / IsolatedAsyncioTestCase). Only an Iso coroutine is exempt.
        # The "runs" cells print a marker from the body: "test ok" alone would not
        # show the body executed.
        runs = self.scratch(
            {
                "r/test_runs.py": """
                    import unittest


                    class PlainTC(unittest.TestCase):
                        def test_plain(self):
                            print("BODY-RAN plain-on-TestCase")


                    class PlainIso(unittest.IsolatedAsyncioTestCase):
                        def test_plain(self):
                            print("BODY-RAN plain-on-Iso")


                    class CoIso(unittest.IsolatedAsyncioTestCase):
                        async def test_co(self):
                            print("BODY-RAN coroutine-on-Iso")
                """,
            }
        )
        code, out = runs.run()
        for cell, marker, line in (
            ("plain on TestCase", "plain-on-TestCase", "r/test_runs.py::PlainTC.test_plain"),
            ("plain on Iso", "plain-on-Iso", "r/test_runs.py::PlainIso.test_plain"),
            ("coroutine on Iso", "coroutine-on-Iso", "r/test_runs.py::CoIso.test_co"),
        ):
            with self.subTest(cell=cell):
                self.assertIn("BODY-RAN " + marker, out)
                self.assertIn("python-suites: test ok " + line, out)
                self.assertNotNamed(out, line)
        self.assertEqual(code, 0, out)
        red = self.scratch(
            {
                "ok/test_ok.py": PASSING,
                "a/test_red.py": """
                    import unittest


                    class TC(unittest.TestCase):
                        async def test_co(self):
                            pass

                        def test_gen(self):
                            yield

                        async def test_agen(self):
                            yield


                    class Iso(unittest.IsolatedAsyncioTestCase):
                        def test_gen(self):
                            yield

                        async def test_agen(self):
                            yield
                """,
            }
        )
        out = self.assertRed(red.run())
        fl = fail_lines(out)
        suffix = " (calling it runs no body)"
        nc = " on a class that is not IsolatedAsyncioTestCase"
        for cell, expect in (
            ("coroutine on TestCase", "a/test_red.py::TC.test_co is a coroutine function" + nc + suffix),
            ("generator on TestCase", "a/test_red.py::TC.test_gen is a generator function" + suffix),
            ("async generator on TestCase", "a/test_red.py::TC.test_agen is a async generator function" + suffix),
            ("generator on Iso", "a/test_red.py::Iso.test_gen is a generator function" + suffix),
            ("async generator on Iso", "a/test_red.py::Iso.test_agen is a async generator function" + suffix),
        ):
            with self.subTest(cell=cell):
                self.assertTrue(any(expect in l for l in fl), "%r not in FAIL lines:\n%s" % (expect, out))

    def test_P12_subtest_failure_is_red_naming_the_subtest(self):
        s = self.scratch(
            {
                "a/test_sub.py": """
                    import unittest


                    class S(unittest.TestCase):
                        def test_x(self):
                            for i in range(3):
                                with self.subTest(i=i):
                                    self.assertNotEqual(i, 1)
                """
            }
        )
        out = self.assertRed(s.run(), "FAILED a/test_sub.py::S.test_x (i=1)")
        self.assertNotNamed(out, "(i=0)", "(i=2)")
