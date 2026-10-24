"""Pins for scripts/python-suites.sh (slice A-iii).

Plan of record: docs/plans/2026-07-citation-hygiene-Aiii-suite-scheduler.md, §6
(pins Q1-Q13, T-net) and §6.1 (Property -> pin). Every test method's name starts
with the §6 / §6.1 row it pins (`test_P4_...` pins row P4); Q10, Q11, Q13 and
T-net are umbrella rows realised by their §6.1 rows (P1-P2, P3-P7, R1-R9 + M1-M2,
G1-G11).

Ground rules:
- No pin runs the driver on the live tree (the driver runs this file; it would
  recurse). Every driver-executing pin builds a scratch git repository in a
  temporary directory, copies the live driver in as `scripts/python-suites.sh`,
  writes fixture files and `git add`s them -- never this file. Every git call
  and every driver run gets the caller's environment with ALL `GIT_*` variables
  removed, so a `GIT_DIR` / `GIT_INDEX_FILE` inherited from a hook cannot point
  git at another repository.
- Every process this file starts goes through `_launch` (pinned by G9), which
  refuses a launch that changes a regime key unless its purpose is one of the
  named members of the unguarded-child class (`UNGUARDED_LAUNCHES`).
- The text readers for `ci.yml` / `mise.toml` (bottom of this file) model a
  stated subset and raise `ReaderError` naming the line on anything else inside
  the regions they read; their grammar is stated on each reader.
"""

from __future__ import annotations

import ast
import os
import posixpath
import re
import shutil
import socket
import stat
import subprocess
import sys
import tempfile
import textwrap
import threading
import time
import types
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DRIVER = REPO / "scripts" / "python-suites.sh"
THIS = Path(__file__).resolve()
THIS_REL = "scripts/test_python_suites.py"
LAUNCH_TIMEOUT = 600


# ---------------------------------------------------------------------------
# The driver's own Python half, loaded from the heredoc (P6 calls its
# predicates; REGIME_KEYS comes from it so the pins and the driver cannot
# disagree on the key set).


def _load_driver_module():
    text = DRIVER.read_text(encoding="utf-8")
    m = re.search(r"<<'PY_DRIVER'\n(.*?)\nPY_DRIVER\n", text, re.S)
    if m is None:
        raise AssertionError("scripts/python-suites.sh: no PY_DRIVER heredoc")
    mod = types.ModuleType("python_suites_driver")
    exec(compile(m.group(1), str(DRIVER) + ":PY_DRIVER", "exec"), mod.__dict__)
    return mod


DRV = _load_driver_module()
REGIME_KEYS = DRV.REGIME_KEYS
GUARD_MSG = "python-suites offline regime"


# ---------------------------------------------------------------------------
# Launch accounting (G9).

# The named members of the unguarded-child class that this file launches. The
# G10 fixture member's own override children (`G10_MEMBER`) are the other named
# members; they are launched by that scratch member, not here.
UNGUARDED_LAUNCHES = {
    "g9-baseline": "a plant run outside the regime (regime keys stripped), "
    "against a loopback listener only, to show the plant can fire",
    "g6-inherited-driver": "the driver launched with inherited NO_PROXY / "
    "no_proxy / proxy / XDG_CACHE_HOME values; its descendants are under the "
    "inner driver's regime (G7)",
}


def base_env():
    """The caller's environment without any GIT_* variable."""
    return {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}


def regime_changes(env, ref=None):
    """Regime keys on which `env` differs from `ref` (default: this process).

    PYTHONPATH is unchanged when it equals the reference or only appends after
    it (the contract's one allowed PYTHONPATH change); with no reference
    PYTHONPATH (this file run outside any driver, so no guard entry to keep
    first) any value is an append."""
    ref = os.environ if ref is None else ref
    changed = []
    for k in REGIME_KEYS:
        old, new = ref.get(k), env.get(k)
        if k == "PYTHONPATH" and new and (not old or new.startswith(old + os.pathsep)):
            continue
        if old != new:
            changed.append(k)
    return changed


def _launch(argv, *, env, purpose, cwd=None, timeout=LAUNCH_TIMEOUT):
    changed = regime_changes(env)
    if changed and purpose not in UNGUARDED_LAUNCHES:
        raise AssertionError(
            "launch %r (purpose %r) changes regime key(s) %s but is not a named "
            "member of the unguarded-child class" % (argv, purpose, changed)
        )
    proc = subprocess.run(
        [str(a) for a in argv],
        env=env,
        cwd=None if cwd is None else str(cwd),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
    )
    return proc.returncode, proc.stdout.decode("utf-8", "replace")


# ---------------------------------------------------------------------------
# Scratch repositories.


class Scratch:
    def __init__(self, case, files, executable=()):
        self.dir = Path(tempfile.mkdtemp(prefix="python-suites-pin-"))
        case.addCleanup(shutil.rmtree, str(self.dir), True)
        (self.dir / "scripts").mkdir()
        shutil.copy(str(DRIVER), str(self.dir / "scripts" / "python-suites.sh"))
        for rel, body in files.items():
            p = self.dir / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(textwrap.dedent(body).lstrip("\n"), encoding="utf-8")
        for rel in executable:
            p = self.dir / rel
            p.chmod(p.stat().st_mode | stat.S_IXUSR)
        self.git("-c", "init.defaultBranch=main", "init", "-q")
        self.git("add", "-A")

    def git(self, *args):
        code, out = _launch(["git"] + list(args), env=base_env(), cwd=self.dir, purpose="git")
        if code != 0:
            raise AssertionError("git %s failed: %s" % (args, out))
        return out

    def members(self):
        out = self.git("ls-files", "-z", "--", ":(glob)**/test_*.py")
        return sorted(p for p in out.split("\0") if p)

    def run(self, extra_env=None, purpose="driver", env=None):
        e = base_env() if env is None else env
        e.update(extra_env or {})
        return _launch(
            ["bash", self.dir / "scripts" / "python-suites.sh"],
            env=e,
            cwd=self.dir,
            purpose=purpose,
        )

    def baseline(self, member, extra_env=None):
        """G9: run one member outside the regime; it must be green."""
        env = {k: v for k, v in base_env().items() if k not in REGIME_KEYS}
        env.update(extra_env or {})
        rel = posixpath.dirname(member)
        mod = posixpath.splitext(posixpath.basename(member))[0]
        return _launch(
            [sys.executable, "-m", "unittest", "-v", mod],
            env=env,
            cwd=self.dir / rel,
            purpose="g9-baseline",
        )


def fail_lines(out):
    return [l for l in out.splitlines() if l.startswith("python-suites: FAIL")]


class DriverCase(unittest.TestCase):
    maxDiff = None

    def scratch(self, files, executable=()):
        return Scratch(self, files, executable)

    def assertGreen(self, result):
        code, out = result
        self.assertEqual(code, 0, out)
        self.assertIn("python-suites: GREEN", out, out)
        self.assertEqual(fail_lines(out), [], out)

    def assertRed(self, result, *names):
        code, out = result
        self.assertNotEqual(code, 0, out)
        fl = fail_lines(out)
        for n in names:
            self.assertTrue(any(n in l for l in fl), "%r not named in FAIL lines:\n%s" % (n, out))
        return out

    def assertNotNamed(self, out, *names):
        fl = fail_lines(out)
        for n in names:
            self.assertFalse(any(n in l for l in fl), "%r wrongly named:\n%s" % (n, out))

    def assertBaselineGreen(self, scratch, member, extra_env=None):
        code, out = scratch.baseline(member, extra_env)
        self.assertEqual(code, 0, "baseline (unguarded) run of %s is not green:\n%s" % (member, out))
        self.assertNotIn("skipped", out, out)


PASSING = """
import unittest


class T(unittest.TestCase):
    def test_passes(self):
        self.assertTrue(True)
"""


# ---------------------------------------------------------------------------
# §6 driver pins.


class TestDriverPins(DriverCase):
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
        bindir = Path(tempfile.mkdtemp(prefix="python-suites-shim-"))
        self.addCleanup(shutil.rmtree, str(bindir), True)
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


class TestComplementPins(DriverCase):
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
        tree = DRV.parse(THIS.read_bytes(), THIS_REL)
        constants = DRV.string_constants(tree)
        code, out = _launch(
            ["git", "-C", REPO, "ls-files", "-z", "--", "*.py"], env=base_env(), purpose="git"
        )
        self.assertEqual(code, 0, out)
        tracked = sorted(p for p in out.split("\0") if p)
        self.assertIn(THIS_REL, tracked)
        entry_points = []
        for path in tracked:
            if re.search(r"(^|/)test_[^/]*\.py$", path):
                continue  # members are not entry points for 3(b)
            ptree = DRV.parse((REPO / path).read_bytes(), path)
            if DRV.is_self_test_entry_point(ptree):
                entry_points.append(path)
        for e in entry_points:
            self.assertFalse(
                DRV.names_by_path(constants, e, THIS_REL),
                "the pins file credits live entry point %s" % e,
            )
        # Controls: the same predicate does credit, root- and member-dir-relative.
        self.assertTrue(DRV.names_by_path(constants | {"x/tool.py"}, "x/tool.py", THIS_REL))
        self.assertTrue(DRV.names_by_path({"tool.py"}, "scripts/tool.py", THIS_REL))
        self.assertFalse(DRV.names_by_path({"tool.py"}, "x/tool.py", THIS_REL))

    def test_P7_unparsable_tracked_py_is_red_member_or_not(self):
        s = self.scratch(
            {"ok/test_ok.py": PASSING, "bad.py": "def (:\n", "q/test_bad.py": "class (:\n"}
        )
        self.assertRed(s.run(), "bad.py does not parse", "q/test_bad.py does not parse")


# ---------------------------------------------------------------------------
# T-net: the offline regime (§6.1 G1-G11).


class Listener:
    """A loopback listener in this process (bind/listen/accept are not guarded):
    reads what the client sends, answers a fixed HTTP/1.0 response, closes."""

    def __init__(self, case, family=socket.AF_INET, host="127.0.0.1"):
        self.sock = socket.socket(family, socket.SOCK_STREAM)
        self.sock.bind((host, 0))
        self.sock.listen(32)
        self.sock.settimeout(0.2)
        self.port = self.sock.getsockname()[1]
        self.stop = False
        self.thread = threading.Thread(target=self._serve, daemon=True)
        self.thread.start()
        case.addCleanup(self.close)

    def _serve(self):
        while not self.stop:
            try:
                conn, _ = self.sock.accept()
            except OSError:
                continue
            try:
                conn.settimeout(0.5)
                try:
                    while conn.recv(65536):
                        pass
                except OSError:
                    pass
                conn.sendall(b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            except OSError:
                pass
            finally:
                conn.close()

    def close(self):
        self.stop = True
        self.thread.join(5)
        self.sock.close()


G1_MEMBER = """
import http.client
import os
import ssl
import unittest
import urllib.error
import urllib.request

PORT = int(os.environ["PYSUITES_PIN_PORT"])
DIRECT = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class G1(unittest.TestCase):
    def test_http_client(self):
        c = http.client.HTTPConnection("127.0.0.1", PORT, timeout=10)
        c.connect()
        c.close()

    def test_urlopen_http(self):
        with DIRECT.open("http://127.0.0.1:%d/" % PORT, timeout=10) as r:
            self.assertEqual(r.read(), b"ok")

    def test_urlopen_https(self):
        # TCP connects, then TLS fails against the plain-HTTP listener: proof
        # the connect happened. Under the regime it fails at the socket layer,
        # before TLS.
        with self.assertRaises(urllib.error.URLError) as cm:
            DIRECT.open("https://127.0.0.1:%d/" % PORT, timeout=10)
        self.assertIsInstance(cm.exception.reason, (ssl.SSLError, ConnectionError))
"""

RAW_CONNECT_CHILD = (
    "import os, socket; s = socket.socket(); "
    "s.connect(('127.0.0.1', int(os.environ['PYSUITES_PIN_PORT']))); s.close()"
)

G2_MEMBER = """
import os
import subprocess
import sys
import tempfile
import unittest

CHILD = %r


class G2(unittest.TestCase):
    def test_python_child(self):
        r = subprocess.run([sys.executable, "-c", CHILD])
        self.assertEqual(r.returncode, 0)

    def test_shebang_child(self):
        d = tempfile.mkdtemp()
        p = os.path.join(d, "tool")
        with open(p, "w") as f:
            f.write("#!/usr/bin/env python3\\n" + CHILD + "\\n")
        os.chmod(p, 0o755)
        r = subprocess.run([p])
        self.assertEqual(r.returncode, 0)
""" % RAW_CONNECT_CHILD

G3_MEMBER = """
import os
import socket
import unittest


class G3(unittest.TestCase):
    def test_connect_ex(self):
        s = socket.socket()
        try:
            self.assertEqual(s.connect_ex(("127.0.0.1", int(os.environ["PYSUITES_PIN_PORT"]))), 0)
        finally:
            s.close()
"""

G4_MEMBER = """
import os
import socket
import unittest


class G4(unittest.TestCase):
    def test_connect_inet6(self):
        s = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
        try:
            s.connect(("::1", int(os.environ["PYSUITES_PIN_PORT6"])))
        finally:
            s.close()
"""

G5_MEMBER = """
import os
import socket
import unittest

PORT = int(os.environ["PYSUITES_PIN_PORT"])


class G5(unittest.TestCase):
    def test_getaddrinfo(self):
        self.assertTrue(socket.getaddrinfo("localhost", PORT))

    def test_gethostbyname(self):
        self.assertTrue(socket.gethostbyname("localhost"))

    def test_gethostbyname_ex(self):
        self.assertTrue(socket.gethostbyname_ex("localhost"))

    def test_gethostbyaddr(self):
        self.assertTrue(socket.gethostbyaddr("127.0.0.1"))

    def test_getnameinfo(self):
        flags = socket.NI_NUMERICHOST | socket.NI_NUMERICSERV
        self.assertTrue(socket.getnameinfo(("127.0.0.1", PORT), flags))
"""

G5_TOPLEVEL_MEMBER = """
import socket
import unittest

ADDRS = socket.getaddrinfo("localhost", 80)


class G5Top(unittest.TestCase):
    def test_imported(self):
        self.assertTrue(ADDRS)
"""

G6_INHERITED = {
    "NO_PROXY": "*",
    "no_proxy": "*",
    "http_proxy": "http://proxy.invalid:3128",
    "https_proxy": "http://proxy.invalid:3128",
    "XDG_CACHE_HOME": "/nonexistent/python-suites-pin-xdg",
}

G6_MEMBER = """
import subprocess
import unittest

INHERITED = %r


def child_env():
    out = subprocess.run(["/bin/sh", "-c", "env"], stdout=subprocess.PIPE).stdout.decode()
    return dict(l.split("=", 1) for l in out.splitlines() if "=" in l)


class G6(unittest.TestCase):
    def check(self, key):
        self.assertEqual(child_env().get(key), INHERITED[key])

    def test_NO_PROXY(self):
        self.check("NO_PROXY")

    def test_no_proxy(self):
        self.check("no_proxy")

    def test_http_proxy(self):
        self.check("http_proxy")

    def test_https_proxy(self):
        self.check("https_proxy")

    def test_XDG_CACHE_HOME(self):
        self.check("XDG_CACHE_HOME")
""" % G6_INHERITED

G7_COUNT_MEMBER = """
import socket
import subprocess
import sys
import unittest

MARK = "_python_suites_guard_installs"


class G7(unittest.TestCase):
    def test_count_in_loader(self):
        self.assertEqual(getattr(socket, MARK, 0), 1)

    def test_count_in_child(self):
        out = subprocess.run(
            [sys.executable, "-c", "import socket; print(getattr(socket, %r, 0))" % MARK],
            stdout=subprocess.PIPE,
        ).stdout
        self.assertEqual(out.strip(), b"1")
"""

RAW_CONNECT_MEMBER = """
import os
import socket
import unittest


class Raw(unittest.TestCase):
    def test_raw_connect(self):
        s = socket.socket()
        try:
            s.connect(("127.0.0.1", int(os.environ["PYSUITES_PIN_PORT"])))
        finally:
            s.close()
"""

G7_NESTED_MEMBER = """
import os
import subprocess
import unittest


class Nested(unittest.TestCase):
    def test_inner_driver_run(self):
        inner = os.environ["PYSUITES_PIN_INNER"]
        p = subprocess.run(
            ["bash", os.path.join(inner, "scripts", "python-suites.sh")],
            cwd=inner,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        out = p.stdout.decode()
        self.assertNotEqual(p.returncode, 0, out)
        fails = [l for l in out.splitlines() if l.startswith("python-suites: FAIL")]
        self.assertTrue(any("r/test_raw.py::Raw.test_raw_connect" in l for l in fails), out)
        self.assertIn("python-suites: test ok c/test_count.py::G7.test_count_in_loader", out)
        self.assertIn("python-suites: test ok c/test_count.py::G7.test_count_in_child", out)
"""

G8_MARK_MEMBER = """
import sys
import unittest


class G8(unittest.TestCase):
    def test_chained_sitecustomize_ran(self):
        self.assertEqual(getattr(sys, "_python_suites_pin_site_marker", None), "set")
"""

G10_MEMBER = """
import os
import socket
import subprocess
import sys
import tempfile
import unittest

# Exit 0: connected. 42: refused by the offline regime. 3: any other failure.
CONNECT = (
    "import socket, sys\\n"
    "try:\\n"
    "    s = socket.socket(); s.connect(('127.0.0.1', int(sys.argv[1]))); s.close()\\n"
    "except OSError as e:\\n"
    "    sys.exit(42 if 'python-suites offline regime' in str(e) else 3)\\n"
)


class G10(unittest.TestCase):
    def setUp(self):
        self.srv = socket.socket()
        self.srv.bind(("127.0.0.1", 0))
        self.srv.listen(8)
        self.addCleanup(self.srv.close)
        self.port = str(self.srv.getsockname()[1])

    def child(self, argv, env=None):
        return subprocess.run(argv, env=env).returncode

    # In the class (the stated limit is real): these CAN reach the network.
    def test_pythonpath_override_is_in_the_class(self):
        d = tempfile.mkdtemp()
        env = {**os.environ, "PYTHONPATH": d}
        self.assertEqual(self.child([sys.executable, "-c", CONNECT, self.port], env), 0)

    def test_parent_environ_mutation_is_in_the_class(self):
        old = os.environ.get("NO_PROXY")
        os.environ["NO_PROXY"] = "*"
        try:
            out = subprocess.run(
                ["/bin/sh", "-c", 'printf %s "$NO_PROXY"'], stdout=subprocess.PIPE
            ).stdout
            self.assertEqual(out, b"*")
        finally:
            if old is None:
                del os.environ["NO_PROXY"]
            else:
                os.environ["NO_PROXY"] = old

    # Outside the class: these CANNOT.
    def test_execve_with_environ_is_outside_the_class(self):
        hop = "import os, sys; os.execve(sys.executable, [sys.executable, '-c', sys.argv[1], sys.argv[2]], os.environ)"
        self.assertEqual(self.child([sys.executable, "-c", hop, CONNECT, self.port]), 42)

    def test_non_regime_key_change_is_outside_the_class(self):
        env = {**os.environ, "X": "1"}
        self.assertEqual(self.child([sys.executable, "-c", CONNECT, self.port], env), 42)

    def test_control_non_network_child_passes(self):
        self.assertEqual(self.child([sys.executable, "-c", "pass"]), 0)
"""

G11_MUTATOR = """
import os
import unittest


class A(unittest.TestCase):
    def test_mutates_regime_keys(self):
        os.environ["NO_PROXY"] = "*"
        os.environ["PYTHONPATH"] = "/nonexistent"
        os.putenv("https_proxy", "http://proxy.invalid:3128")
"""

G11_CHECKER = """
import os
import socket
import subprocess
import sys
import unittest

CONNECT = (
    "import socket, sys\\n"
    "try:\\n"
    "    s = socket.socket(); s.connect(('127.0.0.1', int(sys.argv[1]))); s.close()\\n"
    "except OSError as e:\\n"
    "    sys.exit(42 if 'python-suites offline regime' in str(e) else 3)\\n"
)


class B(unittest.TestCase):
    def test_regime_restored_for_the_next_member(self):
        out = subprocess.run(["/bin/sh", "-c", "env"], stdout=subprocess.PIPE).stdout.decode()
        env = dict(l.split("=", 1) for l in out.splitlines() if "=" in l)
        self.assertNotIn("NO_PROXY", env)
        self.assertTrue(env["https_proxy"].startswith("http://127.0.0.1:"), env["https_proxy"])

    def test_child_still_guarded(self):
        srv = socket.socket()
        srv.bind(("127.0.0.1", 0))
        srv.listen(8)
        try:
            r = subprocess.run([sys.executable, "-c", CONNECT, str(srv.getsockname()[1])])
            self.assertEqual(r.returncode, 42)
        finally:
            srv.close()
"""


class TestOfflineRegimePins(DriverCase):
    def plant(self, files, extra_env, names):
        """Each plant: green without the regime (G9 baseline), red with it."""
        s = self.scratch(files)
        for member in files:
            self.assertBaselineGreen(s, member, extra_env)
        return self.assertRed(s.run(extra_env), *names)

    def test_G1_in_process_connection_is_red(self):
        lst = Listener(self)
        self.plant(
            {"g1/test_g1.py": G1_MEMBER},
            {"PYSUITES_PIN_PORT": str(lst.port)},
            [
                "ERROR g1/test_g1.py::G1.test_http_client",
                "ERROR g1/test_g1.py::G1.test_urlopen_http",
                "g1/test_g1.py::G1.test_urlopen_https",
            ],
        )

    def test_G2_python_child_and_shebang_child_are_red(self):
        lst = Listener(self)
        self.plant(
            {"g2/test_g2.py": G2_MEMBER},
            {"PYSUITES_PIN_PORT": str(lst.port)},
            ["FAILED g2/test_g2.py::G2.test_python_child", "FAILED g2/test_g2.py::G2.test_shebang_child"],
        )

    def test_G3_connect_ex_is_red(self):
        lst = Listener(self)
        self.plant(
            {"g3/test_g3.py": G3_MEMBER},
            {"PYSUITES_PIN_PORT": str(lst.port)},
            ["ERROR g3/test_g3.py::G3.test_connect_ex"],
        )

    def test_G4_inet6_connect_is_red(self):
        lst = Listener(self, socket.AF_INET6, "::1")
        self.plant(
            {"g4/test_g4.py": G4_MEMBER},
            {"PYSUITES_PIN_PORT6": str(lst.port)},
            ["ERROR g4/test_g4.py::G4.test_connect_inet6"],
        )

    def test_G5_name_resolution_is_red_including_at_import_time(self):
        self.plant(
            {"g5/test_g5.py": G5_MEMBER, "g5top/test_g5_top.py": G5_TOPLEVEL_MEMBER},
            {"PYSUITES_PIN_PORT": "80"},
            [
                "ERROR g5/test_g5.py::G5.test_getaddrinfo",
                "ERROR g5/test_g5.py::G5.test_gethostbyname",
                "ERROR g5/test_g5.py::G5.test_gethostbyname_ex",
                "ERROR g5/test_g5.py::G5.test_gethostbyaddr",
                "ERROR g5/test_g5.py::G5.test_getnameinfo",
                "g5top/test_g5_top.py: import failed",
            ],
        )

    def test_G6_environment_half_once_per_regime_key(self):
        s = self.scratch({"g6/test_g6.py": G6_MEMBER})
        self.assertBaselineGreen(s, "g6/test_g6.py", G6_INHERITED)
        out = self.assertRed(
            s.run(G6_INHERITED, purpose="g6-inherited-driver"),
            *["FAILED g6/test_g6.py::G6.test_%s" % k for k in G6_INHERITED]
        )
        self.assertEqual(len([l for l in out.splitlines() if l.startswith("python-suites: test FAILED")]), 5, out)

    def test_G7_guard_installed_exactly_once_and_nested_run_idempotent(self):
        lst = Listener(self)
        port = {"PYSUITES_PIN_PORT": str(lst.port)}
        inner = self.scratch({"c/test_count.py": G7_COUNT_MEMBER, "r/test_raw.py": RAW_CONNECT_MEMBER})
        self.assertBaselineGreen(inner, "r/test_raw.py", port)
        # Direct run: count 1 in the loader and a child; the plant red.
        out = self.assertRed(inner.run(port), "r/test_raw.py::Raw.test_raw_connect")
        self.assertIn("python-suites: test ok c/test_count.py::G7.test_count_in_loader", out)
        self.assertIn("python-suites: test ok c/test_count.py::G7.test_count_in_child", out)
        # The same run nested inside a guarded driver run gives the same result.
        outer = self.scratch({"n/test_nested.py": G7_NESTED_MEMBER})
        self.assertGreen(outer.run(dict(port, PYSUITES_PIN_INNER=str(inner.dir))))

    def test_G7_guard_install_failure_fails_closed(self):
        shadow = Path(tempfile.mkdtemp(prefix="python-suites-shadow-"))
        self.addCleanup(shutil.rmtree, str(shadow), True)
        (shadow / "socket.py").write_text('raise ImportError("planted socket shadow")\n', encoding="utf-8")
        inherited = os.environ.get("PYTHONPATH")
        pp = (inherited + os.pathsep if inherited else "") + str(shadow)
        s = self.scratch({"a/test_ok.py": PASSING})
        code, out = s.run({"PYTHONPATH": pp})
        self.assertNotEqual(code, 0, out)
        self.assertIn("the offline guard did not install (ImportError: planted socket shadow)", out)
        self.assertIn("loader exited 97", out)
        self.assertNotIn("test ok", out)

    def test_G8_existing_sitecustomize_still_runs(self):
        lst = Listener(self)
        site = Path(tempfile.mkdtemp(prefix="python-suites-site-"))
        self.addCleanup(shutil.rmtree, str(site), True)
        (site / "sitecustomize.py").write_text(
            'import sys\nsys._python_suites_pin_site_marker = "set"\n', encoding="utf-8"
        )
        inherited = os.environ.get("PYTHONPATH")
        pp = (inherited + os.pathsep if inherited else "") + str(site)
        s = self.scratch({"m/test_mark.py": G8_MARK_MEMBER, "r/test_raw.py": RAW_CONNECT_MEMBER})
        self.assertBaselineGreen(s, "r/test_raw.py", {"PYSUITES_PIN_PORT": str(lst.port)})
        out = self.assertRed(
            s.run({"PYTHONPATH": pp, "PYSUITES_PIN_PORT": str(lst.port)}),
            "r/test_raw.py::Raw.test_raw_connect",
        )
        self.assertIn("python-suites: test ok m/test_mark.py::G8.test_chained_sitecustomize_ran", out)

    def test_G9_unnamed_regime_changing_launch_is_refused(self):
        env = dict(base_env(), NO_PROXY="*")
        with self.assertRaises(AssertionError) as cm:
            _launch([sys.executable, "-c", "pass"], env=env, purpose="driver")
        self.assertIn("not a named member of the unguarded-child class", str(cm.exception))
        # Appending after the reference PYTHONPATH is not a change.
        ref = {"PYTHONPATH": "/guard"}
        self.assertEqual(regime_changes({"PYTHONPATH": "/guard" + os.pathsep + "/x"}, ref), [])
        self.assertEqual(regime_changes({"PYTHONPATH": "/x" + os.pathsep + "/guard"}, ref), ["PYTHONPATH"])
        # A named member is admitted.
        stripped = {k: v for k, v in base_env().items() if k not in REGIME_KEYS}
        self.assertEqual(_launch([sys.executable, "-c", "pass"], env=stripped, purpose="g9-baseline")[0], 0)

    def test_G9_every_launch_goes_through_the_accounting(self):
        tree = ast.parse(THIS.read_text(encoding="utf-8"))
        spawners = {"run", "Popen", "call", "check_call", "check_output", "system", "popen", "fork", "forkpty"}
        spawners |= {n for n in dir(os) if n.startswith(("exec", "spawn", "posix_spawn"))}
        inside = set()
        for fn in ast.walk(tree):
            if isinstance(fn, ast.FunctionDef) and fn.name == "_launch":
                inside |= {id(n) for n in ast.walk(fn)}
        stray = [
            n.lineno
            for n in ast.walk(tree)
            if isinstance(n, ast.Call)
            and isinstance(n.func, ast.Attribute)
            and isinstance(n.func.value, ast.Name)
            and n.func.value.id in ("subprocess", "os")
            and n.func.attr in spawners
            and id(n) not in inside
        ]
        self.assertEqual(stray, [], "process launches outside _launch at lines %s" % stray)
        # The named set equals the purposes actually used for regime changes.
        used = set()
        for n in ast.walk(tree):
            if isinstance(n, ast.Call):
                for kw in n.keywords:
                    if kw.arg == "purpose" and isinstance(kw.value, ast.Constant):
                        used.add(kw.value.value)
        self.assertEqual(set(UNGUARDED_LAUNCHES) - used, set(), "named but never launched")

    def test_G10_class_is_defined_by_regime_key_difference(self):
        s = self.scratch({"g10/test_g10.py": G10_MEMBER})
        self.assertGreen(s.run())

    def test_G11_regime_keys_restored_before_each_member(self):
        s = self.scratch({"e/test_a_mutator.py": G11_MUTATOR, "e/test_b_checker.py": G11_CHECKER})
        self.assertGreen(s.run())


# ---------------------------------------------------------------------------
# The text readers.


class ReaderError(Exception):
    """An unmodelled construct inside a region the pins read: red, naming the line."""

    def __init__(self, lineno, why, text):
        super().__init__("line %d: %s: %r" % (lineno, why, text))
        self.lineno = lineno


class YScalar(str):
    lineno = 0
    style = "plain"  # plain | quoted | block | null


class YMap(dict):
    lineno = 0


class YSeq(list):
    lineno = 0


def _scalar(value, lineno, style):
    s = YScalar(value)
    s.lineno = lineno
    s.style = style
    return s


_YKEY = re.compile(r"""^(?:"([^"\\]*)"|'([^']*)'|([A-Za-z0-9_][A-Za-z0-9_.-]*))[ ]*:(?=[ ]|$)""")
_BLOCK_HEADER = re.compile(r"^[|>][+-]?$")
_BAD_PLAIN_START = set("&*!{}[]|>'\"%@`,#")


class _YamlSubset:
    """Block-style YAML, the subset GitHub workflows in this repository use.

    Modelled: block mappings and sequences (`- ` items, incl. a sequence at
    its parent key's indent); keys that are plain `[A-Za-z0-9_][A-Za-z0-9_.-]*`
    or single-line quoted; values that are plain scalars on one line, quoted
    scalars closed on the same line, one-line flow sequences of such scalars,
    or block scalars with header `|` `>` `|-` `>-` `|+` `>+` (the body is text).
    Full-line and trailing ` #` comments. Everything else -- anchors, aliases,
    tags, explicit `?` keys, merge keys `<<`, flow mappings, multi-line flow
    or quoted or plain scalars, indentation indicators, tabs, duplicate keys,
    nested `- -` -- raises ReaderError naming the line (fail closed)."""

    def __init__(self, lines):
        self.lines = lines  # [(lineno, raw)]
        self.i = 0

    def _peek(self):
        while self.i < len(self.lines):
            lineno, raw = self.lines[self.i]
            content = raw.lstrip(" ")
            if not content.strip() or content.startswith("#"):
                self.i += 1
                continue
            if raw[: len(raw) - len(content)].count("\t") or content.startswith("\t"):
                raise ReaderError(lineno, "tab indentation", raw)
            return lineno, len(raw) - len(content), content.rstrip("\n").rstrip()
        return None

    @staticmethod
    def _is_item(content):
        return content == "-" or content.startswith("- ")

    def parse(self):
        p = self._peek()
        if p is None:
            return None
        node = self.block(p[1])
        if self._peek() is not None:
            lineno, _, content = self._peek()
            raise ReaderError(lineno, "unexpected indentation", content)
        return node

    def block(self, indent):
        lineno, ind, content = self._peek()
        return self.seq(ind) if self._is_item(content) else self.mapping(ind)

    def mapping(self, indent, first=None):
        m = YMap()
        if first is not None:
            m.lineno = first[2]
            self._entry(m, indent, *first)
        while True:
            p = self._peek()
            if p is None or p[1] < indent:
                break
            lineno, ind, content = p
            if ind > indent:
                raise ReaderError(lineno, "unexpected indentation", content)
            if self._is_item(content):
                break
            km = _YKEY.match(content)
            if km is None:
                raise ReaderError(lineno, "unmodelled key-position construct", content)
            self.i += 1
            if not m:
                m.lineno = lineno
            key = next(g for g in km.groups() if g is not None)
            self._entry(m, indent, key, content[km.end():], lineno)
        return m

    def _entry(self, m, indent, key, rest, lineno):
        if key in m:
            raise ReaderError(lineno, "duplicate key %r" % key, key)
        m[key] = self.value(rest, indent, lineno)

    def value(self, rest, indent, lineno):
        v = rest.strip(" ")
        if v == "" or v.startswith("#"):
            p = self._peek()
            if p is not None and p[1] > indent:
                return self.block(p[1])
            if p is not None and p[1] == indent and self._is_item(p[2]):
                return self.seq(indent)
            return _scalar("", lineno, "null")
        if v[0] in "|>":
            header = v.split(" #", 1)[0].strip()
            if not _BLOCK_HEADER.match(header):
                raise ReaderError(lineno, "unmodelled block-scalar header", v)
            body = []
            while self.i < len(self.lines):
                ln, raw = self.lines[self.i]
                c = raw.lstrip(" ")
                if c.strip() and len(raw) - len(c) <= indent:
                    break
                body.append(raw)
                self.i += 1
            return _scalar("\n".join(body), lineno, "block")
        node = self.inline(v, lineno)
        p = self._peek()
        if p is not None and p[1] > indent:
            raise ReaderError(p[0], "unmodelled continuation", p[2])
        return node

    @staticmethod
    def _quoted(s, lineno):
        """s starts with a quote; return (value, rest)."""
        q = s[0]
        i = 1
        out = []
        while i < len(s):
            c = s[i]
            if q == "'" and c == "'":
                if s[i + 1 : i + 2] == "'":
                    out.append("'")
                    i += 2
                    continue
                return "".join(out), s[i + 1 :]
            if q == '"' and c == "\\":
                if s[i + 1 : i + 2] in ('"', "\\"):
                    out.append(s[i + 1])
                    i += 2
                    continue
                raise ReaderError(lineno, "unmodelled escape", s)
            if q == '"' and c == '"':
                return "".join(out), s[i + 1 :]
            out.append(c)
            i += 1
        raise ReaderError(lineno, "quoted scalar not closed on its line", s)

    @staticmethod
    def _tail_ok(rest):
        r = rest.strip(" ")
        return r == "" or r.startswith("#")

    def inline(self, v, lineno):
        if v[0] in "'\"":
            val, rest = self._quoted(v, lineno)
            if not self._tail_ok(rest):
                raise ReaderError(lineno, "text after a quoted scalar", v)
            return _scalar(val, lineno, "quoted")
        if v[0] == "[":
            return self.flow_seq(v, lineno)
        if v[0] in _BAD_PLAIN_START or v[:2] in ("- ", "? ", ": ") or v in ("-", "?", ":"):
            raise ReaderError(lineno, "unmodelled value-position construct", v)
        text = v
        cut = re.search(r"[ ]#", text)
        if cut:
            text = text[: cut.start()]
        text = text.rstrip(" ")
        if ": " in text or text.endswith(":"):
            raise ReaderError(lineno, "mapping indicator inside a plain scalar", v)
        return _scalar(text, lineno, "plain")

    def flow_seq(self, v, lineno):
        s = YSeq()
        s.lineno = lineno
        body = v[1:]
        while True:
            body = body.lstrip(" ")
            if body.startswith("]"):
                if s and not getattr(s, "_after_item", False):
                    raise ReaderError(lineno, "trailing comma in a flow sequence", v)
                if not self._tail_ok(body[1:]):
                    raise ReaderError(lineno, "text after a flow sequence", v)
                return s
            if body == "" or body.startswith("#"):
                raise ReaderError(lineno, "flow sequence not closed on its line", v)
            if body[0] in "'\"":
                val, body = self._quoted(body, lineno)
                s.append(_scalar(val, lineno, "quoted"))
            else:
                m = re.match(r"[^,\]\[{}#'\"&*!|>%@`:]+", body)
                if m is None or not m.group(0).strip():
                    raise ReaderError(lineno, "unmodelled flow-sequence item", v)
                s.append(_scalar(m.group(0).strip(), lineno, "plain"))
                body = body[m.end():]
            s._after_item = True
            body = body.lstrip(" ")
            if body.startswith(","):
                body = body[1:]
                s._after_item = False
            elif not body.startswith("]"):
                raise ReaderError(lineno, "unmodelled flow-sequence syntax", v)

    def seq(self, indent):
        s = YSeq()
        while True:
            p = self._peek()
            if p is None or p[1] < indent:
                break
            lineno, ind, content = p
            if ind > indent:
                raise ReaderError(lineno, "unexpected indentation", content)
            if not self._is_item(content):
                break
            self.i += 1
            if not s:
                s.lineno = lineno
            rest = content[1:]
            pad = len(rest) - len(rest.lstrip(" "))
            rest = rest.lstrip(" ")
            col = indent + 1 + pad
            if rest == "" or rest.startswith("#"):
                p2 = self._peek()
                if p2 is not None and p2[1] > indent:
                    s.append(self.block(p2[1]))
                else:
                    s.append(_scalar("", lineno, "null"))
                continue
            if self._is_item(rest):
                raise ReaderError(lineno, "unmodelled nested compact sequence", content)
            km = _YKEY.match(rest)
            if km is not None:
                key = next(g for g in km.groups() if g is not None)
                s.append(self.mapping(col, first=(key, rest[km.end():], lineno)))
            else:
                s.append(self.value(rest, indent, lineno) if rest[0] in "|>" else self._item_scalar(rest, indent, lineno))
        return s

    def _item_scalar(self, rest, indent, lineno):
        node = self.inline(rest, lineno)
        p = self._peek()
        if p is not None and p[1] > indent:
            raise ReaderError(p[0], "unmodelled continuation", p[2])
        return node


class Workflow:
    """Reads `ci.yml`'s workflow-level `on:` block and named job regions.

    Region bounds come from a raw scan: top-level keys are lines at indent 0,
    jobs are the lines at the indent of the first key under `jobs:`; comment
    and blank lines never bound a region. Only the regions asked for are
    parsed, with `_YamlSubset`. A top-level line that is not a modelled key,
    a top-level `defaults:` (it would change how the tools job's `run:` is
    interpreted), or a line at job indent that is not a modelled key, is red."""

    def __init__(self, text):
        self.lines = list(enumerate(text.splitlines(), 1))
        self.top = {}
        order = []
        for idx, (lineno, raw) in enumerate(self.lines):
            if not raw.strip() or raw.lstrip(" ").startswith("#") or raw[0] == " ":
                continue
            km = _YKEY.match(raw)
            if km is None:
                raise ReaderError(lineno, "unmodelled top-level construct", raw)
            key = next(g for g in km.groups() if g is not None)
            if key in self.top:
                raise ReaderError(lineno, "duplicate top-level key %r" % key, raw)
            if key == "defaults":
                raise ReaderError(lineno, "workflow-level defaults are not modelled", raw)
            self.top[key] = idx
            order.append(idx)
        self.top_end = {}
        for a, b in zip(order, order[1:] + [len(self.lines)]):
            self.top_end[a] = b
        self.jobs = {}
        if "jobs" in self.top:
            start, end = self.top["jobs"], self.top_end[self.top["jobs"]]
            first_raw = self.lines[start][1]
            if _YKEY.match(first_raw).end() < len(first_raw.split(" #")[0].rstrip()):
                raise ReaderError(self.lines[start][0], "jobs: must be a block mapping", first_raw)
            job_indent = None
            heads = []
            for idx in range(start + 1, end):
                lineno, raw = self.lines[idx]
                c = raw.lstrip(" ")
                if not c.strip() or c.startswith("#"):
                    continue
                ind = len(raw) - len(c)
                if job_indent is None:
                    job_indent = ind
                if ind < job_indent:
                    raise ReaderError(lineno, "unexpected dedent under jobs:", raw)
                if ind == job_indent:
                    km = _YKEY.match(c)
                    if km is None:
                        raise ReaderError(lineno, "unmodelled job-position construct", raw)
                    name = next(g for g in km.groups() if g is not None)
                    if name in self.jobs:
                        raise ReaderError(lineno, "duplicate job %r" % name, raw)
                    heads.append((name, idx))
                    self.jobs[name] = None
            bounds = [h[1] for h in heads] + [end]
            for (name, idx), stop in zip(heads, bounds[1:]):
                self.jobs[name] = (idx, stop)

    def _parse(self, a, b):
        node = _YamlSubset(self.lines[a:b]).parse()
        return node

    def on(self):
        if "on" not in self.top:
            raise ReaderError(0, "no workflow-level on: block", "")
        idx = self.top["on"]
        node = self._parse(idx, self.top_end[idx])
        return node["on"]

    def job(self, name):
        """The job's parsed mapping, or None when the job is absent."""
        span = self.jobs.get(name)
        if span is None:
            return None
        node = self._parse(*span)
        return node[name]


DRIVER_REL = "scripts/python-suites.sh"
DRIVER_ARGVS = (
    ["bash", DRIVER_REL],
    ["sh", DRIVER_REL],
    [DRIVER_REL],
    ["./" + DRIVER_REL],
    ["mise", "run", "tools-test"],
)
DRIVER_STEP_KEYS = {"name", "id", "run", "timeout-minutes"}
SETUP_STEP_KEYS = {"name", "id", "uses", "with"}
MATRIX_EXPR = "${{ matrix.python-version }}"


def driver_is_command(cmd):
    """D1: the driver is the whole command -- argv[0], or the argument after
    bash / sh -- with no shell syntax at all (so nothing can mask its exit)."""
    if any(ch in cmd for ch in ";&|`$<>()\n\\'\"*?~{}[]#!="):
        return False
    return cmd.split() in [list(a) for a in DRIVER_ARGVS]


def driver_step_index(job):
    """Q6: index of the credited driver step, or None."""
    if not isinstance(job, YMap) or "continue-on-error" in job:
        return None
    steps = job.get("steps")
    if not isinstance(steps, YSeq):
        return None
    for i, st in enumerate(steps):
        if (
            isinstance(st, YMap)
            and "run" in st
            and not set(st) - DRIVER_STEP_KEYS
            and isinstance(st["run"], YScalar)
            and st["run"].style in ("plain", "quoted")
            and driver_is_command(st["run"])
        ):
            return i
    return None


def q7_problems(job):
    """Q7: the two-leg matrix, consumed by the last setup-python before the driver."""
    problems = []
    matrix = job.get("strategy", {}).get("matrix") if isinstance(job.get("strategy"), YMap) else None
    if not isinstance(matrix, YMap):
        return ["no strategy.matrix mapping"]
    if set(matrix) != {"python-version"}:
        problems.append("matrix keys %s are not exactly python-version" % sorted(matrix))
    legs = matrix.get("python-version")
    if not (
        isinstance(legs, YSeq)
        and sorted(legs) == ["3.9", "3.x"]
        and all(isinstance(l, YScalar) and l.style == "quoted" for l in legs)
    ):
        problems.append("python-version legs are not exactly quoted \"3.9\" and \"3.x\"")
    di = driver_step_index(job)
    if di is None:
        return problems + ["no credited driver step to precede"]
    setup = None
    for st in job["steps"][:di]:
        if isinstance(st, YMap) and str(st.get("uses", "")).startswith("actions/setup-python@"):
            setup = st
    if setup is None:
        return problems + ["no actions/setup-python step before the driver step"]
    with_ = setup.get("with")
    if (
        set(setup) - SETUP_STEP_KEYS
        or not isinstance(with_, YMap)
        or set(with_) != {"python-version"}
        or re.sub(r"\s+", " ", str(with_["python-version"])) != MATRIX_EXPR
    ):
        problems.append("the last setup-python before the driver does not consume %s" % MATRIX_EXPR)
    return problems


def gate_keys(job):
    """Q8: the job's gating keys (any needs / if, whatever the value)."""
    return sorted(k for k in job if k in ("needs", "if"))


def trigger_problems(on):
    """Q8: pull_request carries exactly branches: [main] and nothing that narrows it."""
    if not isinstance(on, YMap) or "pull_request" not in on:
        return ["on: has no pull_request mapping"]
    pr = on["pull_request"]
    if not isinstance(pr, YMap) or set(pr) != {"branches"}:
        return ["pull_request keys are not exactly branches"]
    if not (isinstance(pr["branches"], YSeq) and list(pr["branches"]) == ["main"]):
        return ["pull_request branches is not exactly [main]"]
    return []


def tools_problems(text):
    wf = Workflow(text)
    problems = trigger_problems(wf.on())
    job = wf.job("tools")
    if not isinstance(job, YMap) or not job:
        return problems + ["tools job absent or empty"]
    problems += ["tools job gated by %s" % k for k in gate_keys(job)]
    if driver_step_index(job) is None:
        problems.append("no run: step executes the driver")
    problems += q7_problems(job)
    return problems


_TKEY = re.compile(r"""^(?:"([^"\\]*)"|'([^']*)'|([A-Za-z0-9_-]+))[ ]*=[ ]*""")
_THEADER = re.compile(r"^\[\[?([^\[\]]+)\]\]?[ ]*(#.*)?$")


def _toml_header_path(h):
    parts = []
    for p in h.split("."):
        p = p.strip()
        if len(p) >= 2 and p[0] == p[-1] and p[0] in "\"'":
            p = p[1:-1]
        parts.append(p)
    return ".".join(parts)


def _toml_value(v, lineno):
    v = v.strip()
    if v.startswith(('"""', "'''")):
        raise ReaderError(lineno, "multi-line string not modelled", v)
    if v[:1] in "\"'":
        q = v[0]
        end = v.find(q, 1)
        if end < 0 or (q == '"' and "\\" in v[1:end]):
            raise ReaderError(lineno, "unmodelled string", v)
        tail = v[end + 1 :].strip()
        if tail and not tail.startswith("#"):
            raise ReaderError(lineno, "text after a string", v)
        return v[1:end]
    if v.startswith("["):
        m = re.match(r"""^\[((?:\s*(?:"[^"\\]*"|'[^']*')\s*,)*\s*(?:"[^"\\]*"|'[^']*')?\s*)\]\s*(#.*)?$""", v)
        if m is None:
            raise ReaderError(lineno, "unmodelled array (one line of strings only)", v)
        return [s[1:-1] for s in re.findall(r"""("[^"\\]*"|'[^']*')""", m.group(1))]
    if re.match(r"^(true|false)\s*(#.*)?$", v):
        return v.startswith("true")
    raise ReaderError(lineno, "unmodelled value", v)


def mise_sections(text, wanted):
    """Reads the `[tasks.<name>]` sections in `wanted` of `mise.toml`.

    Modelled inside those sections: blank and `#` lines; `key = value` with a
    bare or quoted key and a value that is a one-line basic/literal string, a
    one-line array of such strings, or true/false, with an optional trailing
    comment. Anything else inside them raises ReaderError naming the line, and
    so does anything elsewhere that could also define those tasks: a `[tasks]`
    table, a subtable `[tasks.<name>.x]`, or a dotted `tasks.` key. A missing
    or duplicated section is red."""
    out = {}
    current = None
    for lineno, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        hm = _THEADER.match(line)
        if line.startswith("["):
            if hm is None:
                raise ReaderError(lineno, "unmodelled table header", raw)
            path = _toml_header_path(hm.group(1))
            if path == "tasks" or any(path.startswith("tasks.%s." % w) for w in wanted):
                raise ReaderError(lineno, "a second way to define a read task", raw)
            current = None
            for w in wanted:
                if path == "tasks." + w:
                    if w in out:
                        raise ReaderError(lineno, "duplicate section", raw)
                    out[w] = {}
                    current = w
            continue
        if current is None:
            if re.match(r"""^["']?tasks["']?\s*\.""", line):
                raise ReaderError(lineno, "a dotted tasks. key", raw)
            continue
        km = _TKEY.match(line)
        if km is None:
            raise ReaderError(lineno, "unmodelled key", raw)
        key = next(g for g in km.groups() if g is not None)
        if key in out[current]:
            raise ReaderError(lineno, "duplicate key %r" % key, raw)
        out[current][key] = (_toml_value(line[km.end():], lineno), lineno)
    for w in wanted:
        if w not in out:
            raise ReaderError(0, "section [tasks.%s] missing" % w, "")
    return out


def mise_problems(text):
    """Q4 over mise.toml's [tasks.ci] and [tasks.tools-test]."""
    s = mise_sections(text, ("ci", "tools-test"))
    problems = []
    ci, tt = s["ci"], s["tools-test"]
    for k in set(ci) - {"description", "depends"}:
        problems.append("[tasks.ci] has unmodelled key %s" % k)
    deps = ci.get("depends", (None, 0))[0]
    if not (isinstance(deps, list) and "tools-test" in deps):
        problems.append('[tasks.ci].depends does not contain "tools-test"')
    for k in ("sources", "outputs"):
        if k in tt:
            problems.append("[tasks.tools-test] has %s (mise would skip it when unchanged)" % k)
    for k in set(tt) - {"description", "run", "sources", "outputs"}:
        problems.append("[tasks.tools-test] has unmodelled key %s" % k)
    if tt.get("run", (None, 0))[0] != "bash " + DRIVER_REL:
        problems.append("[tasks.tools-test].run is not the string %r" % ("bash " + DRIVER_REL))
    return problems


# ---------------------------------------------------------------------------
# Reader fixtures.

ON_OK = "on:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\n"
CHECK_JOB = (
    "  check:\n"
    "    needs: changes\n"
    "    if: needs.changes.outputs.rust == 'true' || github.event_name == 'push'\n"
    "    runs-on: ubuntu-latest\n"
    "    steps:\n"
    "      - run: cargo check\n"
)
TOOLS_HEAD = (
    "  tools:\n"
    "    runs-on: ubuntu-latest\n"
    "    strategy:\n"
    "      fail-fast: false\n"
    "      matrix:\n"
    '        python-version: ["3.9", "3.x"]\n'
)
TOOLS_STEPS = (
    "    steps:\n"
    "      - uses: actions/checkout@v6\n"
    "      - uses: actions/setup-python@v6\n"
    "        with:\n"
    "          python-version: ${{ matrix.python-version }}\n"
    "      - name: Run the Python suites\n"
    "        run: bash scripts/python-suites.sh\n"
)


def workflow(tools=None, on=ON_OK, after=""):
    tools = TOOLS_HEAD + TOOLS_STEPS if tools is None else tools
    return "name: CI\n\n" + on + "\njobs:\n" + CHECK_JOB + "\n" + tools + after


def tools_with(extra_job_lines="", step=None):
    """The good tools job, with extra job-level lines and/or a replaced driver step."""
    steps = TOOLS_STEPS
    if step is not None:
        steps = steps.replace("      - name: Run the Python suites\n        run: bash scripts/python-suites.sh\n", step)
    return TOOLS_HEAD + extra_job_lines + steps


def mise_fixture(ci='depends = ["check", "tools-test"]', tt='run = "bash scripts/python-suites.sh"', extra=""):
    return (
        '[tasks.check]\nrun = "cargo check"\n\n[tasks.ci]\ndescription = "all"\n'
        + ci
        + "\n\n[tasks.tools-test]\ndescription = \"suites\"\n"
        + tt
        + "\n"
        + extra
    )


class TestReaderPins(unittest.TestCase):
    def assertReaderRed(self, fn, text, lineno=None):
        with self.assertRaises(ReaderError) as cm:
            fn(text)
        if lineno is not None:
            self.assertEqual(cm.exception.lineno, lineno, str(cm.exception))
        return cm.exception

    @staticmethod
    def line_of(text, needle):
        for n, l in enumerate(text.splitlines(), 1):
            if needle in l:
                return n
        raise AssertionError(needle)

    def tools_job(self, text):
        return Workflow(text).job("tools")

    # Live files.

    def test_Q4_live_mise_toml(self):
        self.assertEqual(mise_problems((REPO / "mise.toml").read_text(encoding="utf-8")), [])

    def test_Q6_Q7_Q8_live_ci_yml(self):
        text = (REPO / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        self.assertEqual(tools_problems(text), [])
        wf = Workflow(text)
        # Q8 negative control: the same predicate reads `check` as gated.
        self.assertEqual(gate_keys(wf.job("check")), ["if", "needs"])

    def test_Q6_driver_step_required(self):
        empty = TOOLS_HEAD + "    steps:\n      - uses: actions/checkout@v6\n"
        self.assertIn("no run: step executes the driver", tools_problems(workflow(empty)))
        self.assertEqual(tools_problems(workflow()), [])

    def test_Q7_matrix_consumed_by_setup_python_before_the_driver(self):
        good = workflow()
        self.assertEqual(q7_problems(self.tools_job(good)), [])
        unconsumed = good.replace("          python-version: ${{ matrix.python-version }}\n", "          python-version: '3.12'\n")
        self.assertTrue(q7_problems(self.tools_job(unconsumed)))
        after = TOOLS_HEAD + (
            "    steps:\n"
            "      - run: bash scripts/python-suites.sh\n"
            "      - uses: actions/setup-python@v6\n"
            "        with:\n"
            "          python-version: ${{ matrix.python-version }}\n"
        )
        self.assertTrue(q7_problems(self.tools_job(workflow(after))))
        one_leg = good.replace('["3.9", "3.x"]', '["3.x"]')
        self.assertTrue(q7_problems(self.tools_job(one_leg)))
        include = good.replace('        python-version: ["3.9", "3.x"]\n', '        python-version: ["3.9", "3.x"]\n        include:\n          - os: macos-latest\n')
        self.assertTrue(q7_problems(self.tools_job(include)))

    def test_Q8_present_ungated_and_trigger(self):
        self.assertEqual(gate_keys(self.tools_job(workflow())), [])
        self.assertIn("tools job absent or empty", tools_problems(workflow(tools="")))
        gated_new_output = workflow(tools_with("    needs: changes\n    if: needs.changes.outputs.tools == 'true'\n"))
        self.assertIn("tools job gated by if", tools_problems(gated_new_output))
        self.assertIn("tools job gated by needs", tools_problems(gated_new_output))
        self.assertEqual(gate_keys(Workflow(workflow()).job("check")), ["if", "needs"])
        for on in (
            "on:\n  pull_request:\n    branches: [main]\n    paths: ['crates/**']\n",
            "on:\n  pull_request:\n    branches: [main]\n    paths-ignore: ['docs/**']\n",
            "on:\n  pull_request:\n    branches: [main, dev]\n",
            "on:\n  push:\n    branches: [main]\n",
        ):
            self.assertTrue(trigger_problems(Workflow(workflow(on=on)).on()), on)

    # R-rows.

    def test_R1_keys_recognised_in_any_quoting(self):
        for line in ('    "needs": changes\n', "    'if': x\n"):
            self.assertTrue(gate_keys(self.tools_job(workflow(tools_with(line)))), line)

    def test_R2_modelled_values_read_correctly(self):
        wf = Workflow(workflow())
        self.assertEqual(list(wf.on()["pull_request"]["branches"]), ["main"])
        job = wf.job("tools")
        self.assertEqual(list(job["strategy"]["matrix"]["python-version"]), ["3.9", "3.x"])
        self.assertEqual(str(job["steps"][1]["with"]["python-version"]), MATRIX_EXPR)

    def test_R3_value_position_constructs_are_red(self):
        for line in ("    env: {a: 1}\n", "    timeout-minutes: &x 10\n", "    runs-on: *x\n"):
            text = workflow(tools_with(line).replace("    runs-on: ubuntu-latest\n", "", 1 if "runs-on" in line else 0))
            self.assertReaderRed(self.tools_job, text, self.line_of(text, line.strip()))

    def test_R4_key_position_constructs_are_red(self):
        for line in ("    &a needs: x\n", "    !!str needs: x\n", "    ? needs\n"):
            text = workflow(tools_with(line))
            self.assertReaderRed(self.tools_job, text, self.line_of(text, line.strip()))

    def test_R5_merge_keys_are_red(self):
        for line in ("    <<: *x\n", "    <<: {needs: changes}\n"):
            text = workflow(tools_with(line))
            self.assertReaderRed(self.tools_job, text, self.line_of(text, line.strip()))

    def test_R6_multi_line_flow_sequence_is_red(self):
        on = "on:\n  pull_request:\n    branches: [main,\n      dev]\n"
        text = workflow(on=on)
        self.assertReaderRed(lambda t: Workflow(t).on(), text, self.line_of(text, "[main,"))
        legs = workflow(tools_with().replace('        python-version: ["3.9", "3.x"]\n', '        python-version: [\n          "3.9",\n          "3.x"]\n'))
        # Fail closed (first-fix 10): never read as anything, not even both legs.
        self.assertReaderRed(self.tools_job, legs, self.line_of(legs, "python-version: ["))

    def test_R7_comments_are_neither_values_nor_keys(self):
        self.assertEqual(gate_keys(self.tools_job(workflow(tools_with("    needs: changes # x\n")))), ["needs"])
        self.assertEqual(str(self.tools_job(workflow(tools_with("    needs: changes # x\n")))["needs"]), "changes")
        self.assertEqual(gate_keys(self.tools_job(workflow(tools_with("    # needs: changes\n")))), [])
        hdr = workflow(tools_with().replace("  tools:\n", "  tools: # c\n"))
        self.assertEqual(tools_problems(hdr), [])
        trailing = workflow(tools_with().replace("run: bash scripts/python-suites.sh\n", "run: bash scripts/python-suites.sh # c\n"))
        self.assertIsNotNone(driver_step_index(self.tools_job(trailing)))

    def test_R8_block_scalars(self):
        for header in ("|", ">", "|-", ">-", "|+"):
            step = (
                "      - name: Run the Python suites\n"
                "        run: bash scripts/python-suites.sh\n"
                "      - name: listing\n"
                "        run: %s\n"
                "          ls *.py\n"
                "          needs: x\n" % header
            )
            job = self.tools_job(workflow(tools_with(step=step)))
            self.assertEqual(gate_keys(job), [], header)
            self.assertIn("needs: x", str(job["steps"][3]["run"]))
            self.assertEqual(tools_problems(workflow(tools_with(step=step))), [], header)
        bad = workflow(tools_with(step="      - run: |9x\n          ls\n"))
        self.assertReaderRed(self.tools_job, bad, self.line_of(bad, "|9x"))

    def test_R9_region_bounded_by_the_next_job(self):
        bare = TOOLS_HEAD + "    steps:\n      - uses: actions/checkout@v6\n"
        nxt = (
            "  other:\n"
            "    needs: tools\n"
            "    steps:\n"
            "      - uses: actions/setup-python@v6\n"
            "        with:\n"
            "          python-version: ${{ matrix.python-version }}\n"
            "      - run: bash scripts/python-suites.sh\n"
        )
        job = self.tools_job(workflow(bare, after=nxt))
        self.assertEqual(gate_keys(job), [])
        self.assertIsNone(driver_step_index(job))
        self.assertTrue(q7_problems(job))
        commented = workflow(TOOLS_HEAD + "  # a two-space comment inside tools\n" + TOOLS_STEPS)
        self.assertEqual(tools_problems(commented), [])

    # D1.

    def test_D1_driver_is_the_command_and_unmasked(self):
        def credited(step=None, job_lines=""):
            return driver_step_index(self.tools_job(workflow(tools_with(job_lines, step=step)))) is not None

        for run in ("bash scripts/python-suites.sh", "scripts/python-suites.sh", "mise run tools-test"):
            self.assertTrue(credited("      - run: %s\n" % run), run)
        for run in (
            "echo scripts/python-suites.sh",
            "bash scripts/python-suites.sh || true",
            "bash scripts/python-suites.sh; true",
        ):
            self.assertFalse(credited("      - run: %s\n" % run), run)
        self.assertFalse(credited("      - env:\n          DRIVER: scripts/python-suites.sh\n"))
        self.assertFalse(credited("      - run: bash scripts/python-suites.sh\n        continue-on-error: true\n"))
        self.assertFalse(credited(job_lines="    continue-on-error: true\n"))

    # M-rows.

    def test_M1_mise_unmodelled_constructs_are_red(self):
        self.assertEqual(mise_problems(mise_fixture()), [])
        for ci in ("depends = { a = 1 }", 'depends = [\n  "check",\n  "tools-test",\n]'):
            text = mise_fixture(ci=ci)
            self.assertReaderRed(mise_problems, text, self.line_of(text, "depends ="))
        text = mise_fixture(tt="run = bash scripts/python-suites.sh")
        self.assertReaderRed(mise_problems, text, self.line_of(text, "run = bash"))
        self.assertTrue(mise_problems(mise_fixture(tt='run = ["bash scripts/python-suites.sh"]')))
        elsewhere = mise_fixture(ci='depends = ["check"]', extra='\n[tasks.other]\ndepends = ["tools-test"]\n')
        self.assertIn('[tasks.ci].depends does not contain "tools-test"', mise_problems(elsewhere))
        self.assertReaderRed(mise_problems, mise_fixture(extra='\n[tasks.tools-test.env]\nX = "1"\n'))

    def test_M2_tools_test_has_no_sources_or_outputs(self):
        for line in ('sources = ["scripts/**"]', 'outputs = ["x"]'):
            tt = 'run = "bash scripts/python-suites.sh"\n' + line
            self.assertTrue(any(line.split()[0] in p for p in mise_problems(mise_fixture(tt=tt))), line)


if __name__ == "__main__":
    unittest.main()
