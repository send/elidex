"""Driver and complement pins for scripts/python-suites.sh (slice A-iii).

Plan of record: docs/plans/2026-07-citation-hygiene-Aiii-suite-scheduler.md, §6
(pins Q1-Q13) and §6.1 (Property -> pin). Every test method's name starts with
the §6 / §6.1 row it pins (`test_P4_...` pins row P4). Fixtures: see
`python_suites_fixtures.py`. Every driver-executing pin builds a scratch git
repository (never this file) and runs the copied driver there.
"""

from __future__ import annotations

import ast
import os
import re
import sys
import textwrap
import unittest

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

    def test_Q5_floor_below_3_9_is_red_naming_the_floor(self):
        bindir = self.tmpdir("python-suites-shim-")
        real = sys.executable
        (bindir / "shim.py").write_text(
            textwrap.dedent(
                """
                import os, sys
                REAL = %r
                args = sys.argv[1:]
                if any(a in ("-V", "-VV", "--version") for a in args):
                    print("Python 3.8.18")
                    sys.exit(0)
                if "-c" in args:
                    i = args.index("-c")
                    code = args[i + 1]
                    sys.argv = ["-c"] + args[i + 2:]
                    sys.version_info = (3, 8, 18, "final", 0)
                    exec(compile(code, "<string>", "exec"), {"__name__": "__main__"})
                    sys.exit(0)
                os.execv(REAL, [REAL] + args)
                """
                % real
            ),
            encoding="utf-8",
        )
        shim = bindir / "python3"
        shim.write_text('#!/bin/sh\nexec "%s" -S "%s" "$@"\n' % (real, bindir / "shim.py"), encoding="utf-8")
        shim.chmod(0o755)
        s = self.scratch({"a/test_ok.py": PASSING})
        out = self.assertRed(
            s.run({"PATH": str(bindir) + os.pathsep + os.environ.get("PATH", "")}),
            "floor",
        )
        self.assertIn("3.9", out)
        self.assertIn("3.8.18", out)
        self.assertNotIn("member(s) in", out)

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
        # root, as a module or a package. Outside the check: a module loaded by a computed name
        # (importlib, __import__).
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
