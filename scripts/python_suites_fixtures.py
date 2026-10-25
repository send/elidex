"""Shared fixtures for the python-suites pins (slice A-iii, memo §6).

Imported by the three pin files (`test_python_suites_{driver,regime,ci}.py`).
This module is NOT a set member and defines no `unittest.TestCase` subclass:
the driver's complement check (a) fails any tracked non-`test_*.py` file that
does, so `DriverCase` is a plain mixin that each pin class combines with
`unittest.TestCase`. It carries no `--self-test` constant either (check (b)).

Every process the pins start goes through `_launch` (pinned by G9), which
refuses a launch that changes a regime key unless its purpose is one of the
named members of the unguarded-child class (`UNGUARDED_LAUNCHES`).
"""

# pyright: reportAttributeAccessIssue=false

from __future__ import annotations

import os
import posixpath
import re
import shutil
import subprocess
import sys
import tempfile
import textwrap
import types
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DRIVER = REPO / "scripts" / "python-suites.sh"
LAUNCH_TIMEOUT = 600

# Every file that is part of the pins (members and helper modules). P6 and G9
# read all of them, so splitting the pins cannot hide a stray launch or a
# credited entry point in one of the pieces.
PIN_FILES = (
    "scripts/test_python_suites_driver.py",
    "scripts/test_python_suites_regime.py",
    "scripts/test_python_suites_ci.py",
    "scripts/python_suites_fixtures.py",
    "scripts/python_suites_readers.py",
)

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
    def __init__(self, case, files):
        self.dir = case.tmpdir("python-suites-pin-")
        (self.dir / "scripts").mkdir()
        shutil.copy(str(DRIVER), str(self.dir / "scripts" / "python-suites.sh"))
        for rel, body in files.items():
            p = self.dir / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(textwrap.dedent(body).lstrip("\n"), encoding="utf-8")
        self.git("-c", "init.defaultBranch=main", "init", "-q")
        self.git("add", "-A")

    def git(self, *args):
        code, out = _launch(["git"] + list(args), env=base_env(), cwd=self.dir, purpose="git")
        if code != 0:
            raise AssertionError("git %s failed: %s" % (args, out))
        return out

    def members(self):
        out = self.git("ls-files", "-z", "--", DRV.MEMBER_PATHSPEC)
        return sorted(p for p in out.split("\0") if p)

    def run(self, extra_env=None, purpose="driver"):
        e = base_env()
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


class DriverCase:
    """Mixin: combine as `class X(DriverCase, unittest.TestCase)`. It has no
    TestCase base (complement check (a)), so the `assert*` / `addCleanup` calls
    below resolve only on the combined class: the one attribute-access rule is
    switched off for this file (see the pyright comment at the top)."""

    maxDiff = None

    def scratch(self, files):
        return Scratch(self, files)

    def tmpdir(self, prefix):
        """A fresh directory, removed when the test ends."""
        d = Path(tempfile.mkdtemp(prefix=prefix))
        self.addCleanup(shutil.rmtree, str(d), True)
        return d

    @staticmethod
    def pythonpath_after(d):
        """PYTHONPATH with `d` appended after any inherited entries (the contract's one allowed change)."""
        inherited = os.environ.get("PYTHONPATH")
        return (inherited + os.pathsep if inherited else "") + str(d)

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

