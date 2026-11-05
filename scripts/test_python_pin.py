"""Set member: the pin's reader, admission and refusal (scripts/python_pin.py), N2-N23 and NM mutants.

Plan of record: docs/plans/2026-10-python-pin-pr1-pin-and-gate.md (sections 4.2, 4.8);
docs/plans/2026-10-python-pin-pr1-tests.md (sections 6.3, 6.5).

Not a pin file: it imports `python_pin` (pure functions) and loads `scripts/python_pin_testing.py`
by path; every test that reaches the refusal does so in a child process started with the
helper's isolated environment (a fake `mise`, or none), never with the user's. Controls are plain
methods `control_<id>` called by `test_<id>`, so the mutant tests can run them again under a
mutant (`self.source()` / `self.mod()` return the live module or the mutant). The helper is
reached only as attributes of its loaded module object (what NM-I7 patches).
"""

import ast
import hashlib
import importlib.util
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import python_pin as live  # noqa: E402

REPO = os.path.dirname(HERE)
MODULE_PATH = os.path.join(HERE, "python_pin.py")
HELPER = "scripts/python_pin_testing.py"
REGIME_MEMBER = "scripts/test_python_suites_regime.py"
PREFIX = live.PREFIX


def _load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


HELPER_PATH = os.path.join(REPO, *HELPER.split("/"))
H = _load(HELPER_PATH, "python_pin_testing_under_member")

# N17: the last run of the refusal on a real non-pin interpreter (plan section 10 step 4).
# sha256 of scripts/python_pin.py, the date, "<version> <sys.executable>" and the mise the run could
# reach ("none reachable" or its version) of that run.
REAL_OLD_RUN = {
    "sha256": "619c86f5ade25ecf097a0d7752da1192c833953ea38b7dbe77dc02d48c846846",
    "date": "2026-11-05",
    "interpreter": "3.9.6 /Applications/Xcode.app/Contents/Developer/usr/bin/python3",
    "mise": "none reachable",
}


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


LIVE_SOURCE = read(MODULE_PATH)


def run_child(argv, env, cwd, timeout=60):
    """The one launcher of the member's children; a hang is a red control, not a stuck run."""
    try:
        p = subprocess.run(argv, env=env, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
    except subprocess.TimeoutExpired:
        raise AssertionError("child %r did not exit within %s s" % (argv[3:5], timeout))
    return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace")


def pid_gone(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return True
    except OSError:
        return False
    try:
        with open("/proc/%d/stat" % pid) as f:
            return f.read().rsplit(")", 1)[1].split()[0] == "Z"  # a zombie nobody reaped yet is killed
    except (OSError, IndexError):
        return False


def kill_pids(pids, gone=()):
    """SIGKILL the pids; not the ones `poll_gone` saw gone (a pid reused since is someone else's)."""
    for pid in pids:
        if pid in gone:
            continue
        try:
            os.kill(pid, 9)
        except OSError:
            pass


# Answers shaped like the measured mise 2026.10.25 ones (records R4.2); fault injections are marked.
Q_A = "ls --current --json python"
Q_B = "which python3"
Q_C = "config ls --json"


def Q_D(path):
    return "config get -f %s env._.path" % path


def elem(version="3.15.2", src_type=".tool-versions", src_path="/p/.tool-versions", installed=False, source=None):
    return {
        "version": version,
        "requested_version": version,
        "install_path": "/data/installs/python/" + version,
        "source": source if source is not None else {"type": src_type, "path": src_path},
        "installed": installed,
        "active": True,
    }


def ok(stdout):
    return {"rc": 0, "stdout": stdout}


def fail(rc=1, stderr=""):
    return {"rc": rc, "stdout": "", "stderr": stderr}


NOT_A_MISE_BIN = "mise ERROR python3 is not a mise bin. Perhaps you need to install it first.\n"
UNTRUSTED = (
    "mise ERROR error parsing config file: /p/mise.toml\n"
    "mise ERROR Config files in /p/mise.toml are not trusted.\n"
    "Trust them with `mise trust`. See https://mise.jdx.dev/cli/trust.html for more information.\n"
    "mise ERROR Version: 2026.10.25 macos-arm64\n"
    "Run with --verbose or MISE_VERBOSE=1 for more information\n"
)


def state_entries(base=None):
    """The tracked-configs / trusted-configs targets of the state dir `base` (default: the real one;
    read only; absent = empty, never a skip)."""
    base = base or os.environ.get("MISE_STATE_DIR")
    if not base:
        xdg = os.environ.get("XDG_STATE_HOME") or os.path.join(os.path.expanduser("~"), ".local", "state")
        base = os.path.join(xdg, "mise")
    out = []
    for name in ("tracked-configs", "trusted-configs"):
        d = os.path.join(base, name)
        if not os.path.isdir(d):
            continue
        for entry in os.listdir(d):
            p = os.path.join(d, entry)
            if os.path.islink(p):
                out.append(os.path.realpath(os.path.join(d, os.readlink(p))))
            elif os.path.isfile(p):
                with open(p, errors="replace") as f:
                    out.extend(os.path.realpath(line.strip()) for line in f if line.strip())
    return out


class Result:
    def __init__(self, rc, err, root, bindir, tmp):
        self.rc, self.err, self.root, self.bindir, self.tmp = rc, err, root, bindir, tmp
        self.lines = err.split("\n")

    def calls(self):
        return H.read_calls(self.bindir) if self.bindir else []


class PinCase(unittest.TestCase):
    def setUp(self):
        self._src = None
        self._mods = 0
        self._gone = set()
        # N19 (c): a sentinel `mise` / `python3` is first on the test's PATH for the whole case.
        self.sentinel = self.tmp()
        H.write_sentinel(self.sentinel, ("mise", "python3"))
        patcher = mock.patch.dict(os.environ, {"PATH": self.sentinel + os.pathsep + os.environ.get("PATH", "")})
        patcher.start()
        self.addCleanup(patcher.stop)

    def reap_at_cleanup(self, bindir):
        self.addCleanup(lambda: kill_pids(H.read_hang_pids(bindir) if bindir else [], self._gone))

    def tmp(self):
        d = os.path.realpath(tempfile.mkdtemp(prefix="python-pin-test-"))
        self.addCleanup(shutil.rmtree, d, True)
        return d

    # -- the module under test: live, or a mutant -------------------------------------------

    def source(self):
        return LIVE_SOURCE if self._src is None else self._src

    def mod(self):
        if self._src is None:
            return live
        d = self.tmp()
        self._mods += 1
        path = os.path.join(d, "python_pin.py")
        with open(path, "w", encoding="utf-8") as f:
            f.write(self._src)
        return _load(path, "python_pin_mutant_%d" % self._mods)

    # -- the child ---------------------------------------------------------------------------

    def run_pin(self, version="3.9.6", answers=None, fake=True, pin_line="python 3.15.2\n", cwd=None,
                extra_env=None, executable="", timeout="", child_timeout=60):
        """pin_line None = no pin file."""
        tmp = self.tmp()
        root = os.path.join(tmp, "repo")
        module = H.pin_tree(root, self.source(), pin_line)
        bindir = os.path.join(tmp, "bin")
        if fake:
            H.write_fake_mise(bindir, answers if answers is not None else {"*": fail()})
        else:
            bindir = None
        env = H.isolated_mise_env(os.path.join(tmp, "iso"), fake_mise=bindir, extra_env=extra_env)
        self.reap_at_cleanup(bindir)
        argv = [sys.executable, "-c", H.LAUNCHER, module, H.five(version), executable, str(timeout)]
        rc, _, err = run_child(argv, env, cwd or root, child_timeout)
        # N19 (c): a child that bypassed the helper reached the sentinel, not the user's mise.
        self.assertEqual(
            H.read_sentinel(self.sentinel), [], "a child reached the sentinel: it bypassed the helper's environment"
        )
        # N19 (a): the child's MISE_* are the helper's, under <tmp>/iso, in every run.
        # (the MISE_* a test sets on purpose through extra_env are the test's, not the helper's.)
        built = dict((k, v) for k, v in env.items() if k not in (extra_env or {}))
        self.assertEqual(H.isolation_problems(built, os.path.join(tmp, "iso")), [])
        # N19 (a): the child's PATH reaches the fake mise, or none.
        found = shutil.which("mise", path=env["PATH"])
        self.assertEqual(found, os.path.join(bindir, "mise") if bindir else None)
        return Result(rc, err, os.path.realpath(root), bindir, tmp)

    def refused(self, r, running="3.9.6 final", pin="3.15.2"):
        self.assertEqual(r.rc, 1, r.err)
        self.assertEqual(r.lines[0], PREFIX + "REFUSED: the running interpreter is not the pin.")
        self.assertIn(PREFIX + "pin: %s (%s)" % (pin, os.path.join(r.root, ".tool-versions")), r.lines)
        self.assertIn(PREFIX + "running: " + running, r.lines)
        self.assertTrue(any(l.startswith(PREFIX + "sys.executable: ") for l in r.lines))
        self.assertTrue(any(l.startswith(PREFIX + "cwd: ") for l in r.lines))
        self.assertIn(PREFIX + "repository root: " + r.root, r.lines)
        block = live.GENERIC_BLOCK % {"pin": pin, "root": r.root}
        self.assertEqual(r.err.count(block), 1, "the generic block is missing or repeated")
        self.assertTrue(r.err.endswith(block + "\n" + PREFIX + "exit 1\n"), "the generic block, then the exit line, are not last")

    def a_fixture(self, a, b=None, c=None, d=None):
        ans = {Q_A: a, Q_B: b or fail(1, NOT_A_MISE_BIN), Q_C: c or fail(1, UNTRUSTED)}
        for path, answer in (d or {}).items():
            ans[Q_D(path)] = answer
        return ans

    # ===== N2 parser ======================================================================

    def control_N2(self):
        m = self.mod()
        for text in ("python 3.15.2", "python 3.15.2\n", "python 0.0.0\n", "python 10.20.30"):
            nums = tuple(int(x) for x in text.split()[1].split("."))
            self.assertEqual(m.parse_pin(text), nums, text)
        for text, why in H.RED_PIN_TEXTS:
            with self.assertRaises(m.PinFileError, msg=repr(text)) as cm:
                m.parse_pin(text)
            self.assertIn(why, str(cm.exception), repr(text))
        d = self.tmp()
        with self.assertRaises(m.PinFileError) as cm:
            m.read_pin(d)  # no file
        self.assertIn("no readable file", str(cm.exception))
        for data, why in ((b"\xff\xfe python", "not UTF-8"), (b"", "empty")):
            with open(os.path.join(d, ".tool-versions"), "wb") as f:
                f.write(data)
            with self.assertRaises(m.PinFileError) as cm:
                m.read_pin(d)
            self.assertIn(why, str(cm.exception))
        with open(os.path.join(d, ".tool-versions"), "wb") as f:
            f.write(b"python 3.15.2\n")
        self.assertEqual(m.read_pin(d), (3, 15, 2))
        # The live pin file parses, and to the triple the live gate admits.
        pin = m.read_pin(REPO)
        text = read(os.path.join(REPO, ".tool-versions"))
        self.assertEqual(pin, tuple(int(x) for x in text.split()[1].split(".")))
        self.assertTrue(m.admitted(tuple(sys.version_info), pin))  # this member runs under the pin (the driver's Step 1)
        # A malformed pin file is exit 2 and not the refusal: no oracle, no generic block.
        for line in ("python 3.15\n", "python 3.15.2\r\n", "python 3.15.2\ncore:python 3.14.10\n"):
            r = self.run_pin(version="3.15.2", pin_line=line)
            self.assertEqual(r.rc, 2, r.err)
            self.assertIn(PREFIX + "pin file %s: " % os.path.join(r.root, ".tool-versions"), r.err)
            self.assertIn(PREFIX + "the pin file is exactly one line, python X.Y.Z", r.lines)
            self.assertEqual(r.calls(), [])
            self.assertNotIn("REFUSED", r.err)
        r = self.run_pin(version="3.15.2", pin_line=None)
        self.assertEqual(r.rc, 2, r.err)
        self.assertIn("no readable file", r.err)

    # ===== N3 admission ===================================================================

    def control_N3(self):
        m = self.mod()
        pin = (3, 15, 2)
        self.assertTrue(m.admitted((3, 15, 2, "final", 0), pin))
        for v in ((3, 15, 1, "final", 0), (3, 15, 3, "final", 0), (3, 14, 10, "final", 0), (3, 16, 0, "final", 0),
                  (3, 9, 6, "final", 0), (3, 15, 2, "candidate", 1), (3, 15, 2, "alpha", 1), (2, 15, 2, "final", 0)):
            self.assertFalse(m.admitted(v, pin), v)
        # The version is read by index: a plain tuple (no attribute access) is refused with its facts.
        text = m.refusal_text(pin, "/r", (3, 9, 6, "final", 0), "/x/python3", "/c", [], [])
        self.assertIn(PREFIX + "running: 3.9.6 final", text.split("\n"))
        # Admission never consults mise: on an admitted run the fake logs no call, even though
        # it would answer "3.15.2 installed".
        r = self.run_pin(version="3.15.2", answers={"*": ok("3.15.2 installed\n")})
        self.assertEqual(r.rc, 0, r.err)
        self.assertEqual(r.calls(), [])
        self.assertEqual(r.err, PREFIX + "admitted 3.15.2\n")
        # And a version the pin does not admit is refused although mise would say it is installed.
        r = self.run_pin(version="3.15.2.candidate.1", answers={"*": ok("3.15.2 installed\n")})
        self.refused(r, running="3.15.2 candidate")

    # ===== N9 root ========================================================================

    def control_N9(self):
        decoy = self.tmp()
        with open(os.path.join(decoy, ".tool-versions"), "w") as f:
            f.write("python 3.9.6\n")
        r = self.run_pin(version="3.9.6", cwd=decoy)
        self.refused(r)
        self.assertIn(PREFIX + "cwd: " + decoy, r.lines)

    # ===== N10 each case's fix, N11 argv, N12 compound ====================================

    def control_N10(self):
        # E6: no mise on PATH.
        r = self.run_pin(fake=False)
        self.refused(r)
        self.assertIn(PREFIX + "  mise not on PATH (FileNotFoundError)", r.lines)
        self.assertIn(PREFIX + "(b) mise which python3", r.lines)
        self.assertIn(PREFIX + "  not asked: mise not on PATH (FileNotFoundError)", r.lines)
        # E7: untrusted.
        r = self.run_pin(answers=self.a_fixture(fail(1, UNTRUSTED)))
        self.refused(r)
        for line in UNTRUSTED.rstrip("\n").split("\n"):
            self.assertIn(PREFIX + line, r.lines)
        self.assertIn(PREFIX + "  exit status 1", r.lines)
        self.assertEqual([c["argv"] for c in r.calls()], [Q_A.split(), Q_B.split(), Q_C.split()])  # (d) not asked
        # E8: another version wins, from a file and from a variable.
        local = "/p/mise.local.toml"
        a = [elem("3.14.10", "mise.toml", local, True)]
        r = self.run_pin(answers=self.a_fixture(
            ok(json.dumps(a)), ok("/data/installs/python/3.14.10/bin/python3\n"),
            ok(json.dumps([{"path": local}, {"path": "/p/.tool-versions"}])),
            {local: fail(1, "Key not found: env._.path in /p/mise.local.toml\n"), "/p/.tool-versions": fail(1, "parse error\n")}))
        self.refused(r)
        self.assertIn(PREFIX + '  version="3.14.10" source={"type": "mise.toml", "path": "%s"} installed=true '
                      'install_path="/data/installs/python/3.14.10"' % local, r.lines)
        env_source = {"type": "environment", "key": "MISE_PYTHON_VERSION", "value": "3.14.10"}
        r = self.run_pin(answers=self.a_fixture(ok(json.dumps([elem("3.14.10", source=env_source)]))))
        self.refused(r)
        self.assertIn(PREFIX + '  version="3.14.10" source=%s installed=false install_path="/data/installs/python/3.14.10"'
                      % json.dumps(env_source), r.lines)
        # The two-element answer (records R4.2 draft 6): both printed, in mise's order.
        two = [elem("3.14.10", "mise.toml", local), elem("3.15.2", "mise.toml", local)]
        r = self.run_pin(answers=self.a_fixture(ok(json.dumps(two))))
        self.refused(r)
        first = [i for i, l in enumerate(r.lines) if 'version="3.14.10"' in l]
        second = [i for i, l in enumerate(r.lines) if 'version="3.15.2"' in l]
        self.assertEqual((len(first), len(second)), (1, 1))
        self.assertLess(first[0], second[0])
        # E9: nothing resolved.
        r = self.run_pin(answers=self.a_fixture(ok("[]")))
        self.refused(r)
        self.assertIn(PREFIX + "  mise resolves no Python at " + r.root, r.lines)
        # E10: the pin, not installed.
        r = self.run_pin(answers=self.a_fixture(ok(json.dumps([elem()]))))
        self.refused(r)
        self.assertIn(PREFIX + '  version="3.15.2" source={"type": ".tool-versions", "path": "/p/.tool-versions"} '
                      'installed=false install_path="/data/installs/python/3.15.2"', r.lines)
        self.assertIn(PREFIX + "(b) mise which python3", r.lines)
        self.assertEqual(r.lines[r.lines.index(PREFIX + "(b) mise which python3") + 1], PREFIX + "  exit status 1")
        # E11: another python3 ahead of the shim; E13: the cwd outside the root.
        other = self.tmp()
        exe = "/opt/homebrew/opt/python@3.14/bin/python3.14"
        r = self.run_pin(version="3.14.10", executable=exe, cwd=other, answers=self.a_fixture(
            ok(json.dumps([elem(installed=True)])), ok("/data/installs/python/3.15.2/bin/python3\n"),
            ok(json.dumps([{"path": "/p/mise.toml"}])), {"/p/mise.toml": fail(1, "Key not found\n")}))
        self.refused(r, running="3.14.10 final")
        self.assertIn(PREFIX + "sys.executable: " + exe, r.lines)
        self.assertIn(PREFIX + "  /data/installs/python/3.15.2/bin/python3", r.lines)
        self.assertIn(PREFIX + "cwd: " + other, r.lines)
        self.assertIn(PREFIX + "repository root: " + r.root, r.lines)
        self.assertNotEqual(other, r.root)
        # E12: `[env] _.path` points elsewhere: the file and its list are printed.
        r = self.run_pin(answers=self.a_fixture(
            ok(json.dumps([elem()])), None, ok(json.dumps([{"path": "/p/mise.toml"}, {"path": "/p/.tool-versions"}])),
            {"/p/mise.toml": ok('["altbin"]\n'), "/p/.tool-versions": fail(1, "toml parse error\n")}))
        self.refused(r)
        d_head = PREFIX + "(d) " + Q_D("/p/mise.toml").join(["mise ", ""])
        self.assertEqual(r.lines[r.lines.index(d_head) + 1], PREFIX + '  ["altbin"]')
        self.assertIn(PREFIX + "  /p/mise.toml", r.lines)
        self.assertIn(PREFIX + "  /p/.tool-versions", r.lines)

    def control_N11(self):
        paths = ["/p/mise.toml", "/p/.tool-versions", "/home/u/.config/mise/config.toml"]
        r = self.run_pin(answers=self.a_fixture(
            ok(json.dumps([elem()])), None, ok(json.dumps([{"path": p} for p in paths])),
            dict((p, fail(1, "no\n")) for p in paths)))
        self.refused(r)
        self.assertEqual([c["argv"] for c in r.calls()],
                         [Q_A.split(), Q_B.split(), Q_C.split()] + [Q_D(p).split() for p in paths])

    def control_N12(self):
        local = "/p/mise.local.toml"
        exe = "/opt/homebrew/opt/python@3.14/bin/python3.14"
        r = self.run_pin(version="3.14.10", executable=exe, extra_env={"MISE_PYTHON_COMPILE": "1"},
                         answers=self.a_fixture(ok(json.dumps([elem("3.14.10", "mise.toml", local, True)])),
                                                ok("/data/installs/python/3.15.2/bin/python3\n")))
        self.refused(r, running="3.14.10 final")
        self.assertIn(PREFIX + '  version="3.14.10" source={"type": "mise.toml", "path": "%s"} installed=true '
                      'install_path="/data/installs/python/3.14.10"' % local, r.lines)
        self.assertIn(PREFIX + "  /data/installs/python/3.15.2/bin/python3", r.lines)
        self.assertIn(PREFIX + "sys.executable: " + exe, r.lines)
        self.assertEqual([l for l in r.lines if "warning:" in l], [warning("MISE_PYTHON_COMPILE", "1")])

    # ===== N13 warning ====================================================================

    def control_N13(self):
        env = {"MISE_PYTHON_COMPILE": "1", "MISE_PYTHON_FOO": "x", "MISE_GITHUB_ATTESTATIONS": "false"}
        expected = [warning(k, env[k]) for k in sorted(env)]
        r = self.run_pin(version="3.9.6", extra_env=env, answers=self.a_fixture(ok("[]")))
        self.refused(r)
        self.assertEqual([l for l in r.lines if "warning:" in l], expected)
        r = self.run_pin(version="3.15.2", extra_env=env)
        self.assertEqual(r.rc, 0, r.err)
        self.assertEqual(r.err, "\n".join(expected) + "\n" + PREFIX + "admitted 3.15.2\n")
        # E18: a version input is not a warning (the oracle shows it).
        for version, rc in (("3.9.6", 1), ("3.15.2", 0)):
            r = self.run_pin(version=version, extra_env={"MISE_PYTHON_VERSION": "3.14.10"}, answers=self.a_fixture(ok("[]")))
            self.assertEqual(r.rc, rc, r.err)
            self.assertNotIn("warning:", r.err)

    # ===== N14 sys.executable =============================================================

    def control_N14(self):
        r = self.run_pin(answers=self.a_fixture(ok("[]")))
        self.refused(r)
        tmp = self.tmp()
        env = H.isolated_mise_env(tmp)
        rc, out, err = run_child([sys.executable, "-c", "import sys; print(sys.executable)"], env, tmp)
        self.assertEqual(rc, 0, err)
        self.assertIn(PREFIX + "sys.executable: " + out.strip(), r.lines)

    # ===== N15 limits =====================================================================

    def poll_gone(self, pids, seconds=2):
        deadline = time.time() + seconds
        while time.time() < deadline:
            self._gone.update(p for p in pids if pid_gone(p))
            if self._gone.issuperset(pids):
                return
            time.sleep(0.1)
        self._gone.update(p for p in pids if pid_gone(p))
        alive = [p for p in pids if p not in self._gone]
        raise AssertionError("still alive after the refusal returned: %s" % alive)

    def wait_for_pids(self, bindir, seconds=30):
        """Block until hang.pids holds the fake and its sleep (bounded: a start-up that never gets there is red)."""
        deadline = time.time() + seconds
        while time.time() < deadline:
            if len(H.read_hang_pids(bindir)) >= 2:
                return
            time.sleep(0.02)
        raise AssertionError("hang.pids did not hold two pids within %s s: %s" % (seconds, H.read_hang_pids(bindir)))

    def control_N15(self):
        # E16: stderr longer than the cap.
        r = self.run_pin(answers=self.a_fixture(fail(1, "x" * 10000)))
        self.refused(r)
        self.assertIn(PREFIX + "x" * 4000, r.lines)
        self.assertNotIn("x" * 4001, r.err)
        self.assertIn(PREFIX + "[truncated: 6000 more characters]", r.lines)
        # E15: (a) hangs with a grandchild holding the pipes. In process, so that the 1 s timeout is
        # started (by `communicate`) only once hang.pids holds the fake and its sleep.
        m = self.mod()
        tmp = self.tmp()
        bindir = os.path.join(tmp, "bin")
        H.write_fake_mise(bindir, {"*": {"hang": True}})
        self.reap_at_cleanup(bindir)
        env = H.isolated_mise_env(os.path.join(tmp, "iso"), fake_mise=bindir)
        original = subprocess.Popen.communicate

        def communicate_when_hung(proc, *args, **kwargs):
            if kwargs.get("timeout") is None and len(args) < 2:
                raise AssertionError("communicate() was called without a timeout")
            self.wait_for_pids(bindir)
            return original(proc, *args, **kwargs)

        with mock.patch.dict(os.environ, env, clear=True), mock.patch.object(subprocess.Popen, "communicate",
                                                                             communicate_when_hung):
            records = m.ask_mise(tmp, 1.0)
        text = m.refusal_text((3, 15, 2), tmp, (3, 9, 6, "final", 0), sys.executable, tmp, [], records)
        lines = text.split("\n")
        self.assertIn(PREFIX + "  no answer within 1.0 seconds (the question timed out)", lines)
        self.assertEqual(text.count("not asked: an earlier question timed out"), 2)  # (b) and (c)
        self.assertEqual([c["argv"] for c in H.read_calls(bindir)], [Q_A.split()])
        pids = H.read_hang_pids(bindir)
        self.assertEqual(len(pids), 2, "hang.pids must hold the fake and its sleep: %s" % pids)
        self.poll_gone(pids)
        # E14: garbage and exit 2; (b)-(c) still asked, (d) not when (c) failed.
        r = self.run_pin(answers=self.a_fixture(ok("not json"), fail(1, NOT_A_MISE_BIN), fail(2, "boom\n")))
        self.refused(r)
        self.assertIn(PREFIX + "  the output does not parse as a JSON array of objects", r.lines)
        self.assertIn(PREFIX + "  exit status 2", r.lines)
        self.assertIn(PREFIX + "boom", r.lines)
        self.assertEqual([c["argv"] for c in r.calls()], [Q_A.split(), Q_B.split(), Q_C.split()])
        r = self.run_pin(answers=self.a_fixture(fail(2, "bad\n")))
        self.refused(r)
        self.assertIn(PREFIX + "bad", r.lines)
        # Deeply nested JSON (RecursionError in the parser) is "does not parse", not a traceback.
        r = self.run_pin(answers=self.a_fixture(ok("[" * 100000 + "]" * 100000)))
        self.refused(r)
        self.assertIn(PREFIX + "  the output does not parse as a JSON array of objects", r.lines)
        self.assertNotIn("Traceback", r.err)

    def control_N15_sigint(self):
        m = self.mod()
        tmp = self.tmp()
        bindir = os.path.join(tmp, "bin")
        H.write_fake_mise(bindir, {"*": {"hang": True}})
        self.reap_at_cleanup(bindir)
        env = H.isolated_mise_env(os.path.join(tmp, "iso"), fake_mise=bindir)
        old = signal.getsignal(signal.SIGINT)
        signal.signal(signal.SIGINT, signal.default_int_handler)
        failure = []

        def interrupt_when_hung():
            try:
                self.wait_for_pids(bindir)
            except AssertionError as e:
                failure.append(e)
            os.kill(os.getpid(), signal.SIGINT)  # also on failure: the main thread must not hang in communicate

        timer = threading.Thread(target=interrupt_when_hung)
        try:
            with mock.patch.dict(os.environ, env, clear=True):
                timer.start()
                with self.assertRaises(KeyboardInterrupt):
                    m.ask_mise(tmp, 60)
        finally:
            timer.join()
            signal.signal(signal.SIGINT, old)
        if failure:
            raise failure[0]
        pids = H.read_hang_pids(bindir)
        self.assertEqual(len(pids), 2, pids)
        self.poll_gone(pids)

    # ===== N16 grammar, N17 record, N18 imports ===========================================

    def control_N16(self):
        try:
            ast.parse(self.source(), feature_version=(3, 9))
        except SyntaxError as e:
            raise AssertionError("python_pin.py does not parse with 3.9 grammar: %s" % e)

    def control_N17(self):
        self.assertTrue(REAL_OLD_RUN.get("mise"), "REAL_OLD_RUN records no mise (a version, or 'none reachable')")
        digest = hashlib.sha256(self.source().encode("utf-8")).hexdigest()
        self.assertEqual(digest, REAL_OLD_RUN["sha256"],
                         "python_pin.py changed since its last run on %s (%s, %s): record a new run"
                         % (REAL_OLD_RUN["interpreter"], REAL_OLD_RUN["date"], digest))

    def control_N18(self):
        tree = ast.parse(self.source())
        top = []
        for node in tree.body:
            if isinstance(node, ast.Import):
                top.extend(a.name for a in node.names)
            elif isinstance(node, ast.ImportFrom):
                top.append(node.module)
        self.assertEqual(sorted(top), ["os", "sys"])
        inside = {}
        for fn in ast.walk(tree):
            if isinstance(fn, ast.FunctionDef):
                for n in ast.walk(fn):
                    if isinstance(n, ast.Import):
                        inside.setdefault(fn.name, set()).update(a.name for a in n.names)
                    elif isinstance(n, ast.ImportFrom):
                        inside.setdefault(fn.name, set()).add(n.module)
        self.assertEqual(inside, {"ask_mise": {"json", "signal", "subprocess"}, "_guarded_main": {"traceback"}})
        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef):
                for base in node.bases:
                    self.assertNotIn("TestCase", ast.dump(base))
            if isinstance(node, ast.Constant) and isinstance(node.value, str):
                self.assertNotEqual(node.value, "--self" + "-test")

    # ===== N19 / N20 / N21 the I7 pins ====================================================

    def control_N19(self):
        r = self.run_pin(answers=self.a_fixture(ok(json.dumps([elem()]))))  # (c) and the PATH half of (a) run inside
        self.refused(r)
        calls = r.calls()
        self.assertTrue(calls)
        # (a) every call's MISE_* are the helper's, under the helper's root <tmp>/iso (also asserted in run_pin).
        for call in calls:
            self.assertEqual(H.isolation_problems(call["environ"], os.path.join(r.tmp, "iso")), [])
        # (b) no entry of the real mise state resolves into this run's <tmp>.
        self.assertEqual(self.entries_into(r.tmp), [])
        # Control: entries planted in a scratch state dir (a link under tracked-configs, a line in
        # trusted-configs) that resolve into <tmp> are reported, so (b) cannot be green by reading nothing.
        scratch = self.tmp()
        os.makedirs(os.path.join(scratch, "tracked-configs"))
        os.makedirs(os.path.join(scratch, "trusted-configs"))
        os.symlink(os.path.join(r.tmp, "mise.toml"), os.path.join(scratch, "tracked-configs", "a"))
        with open(os.path.join(scratch, "trusted-configs", "b"), "w") as f:
            f.write(os.path.join(r.tmp, "x", ".tool-versions") + "\n")
        self.assertEqual(len(self.entries_into(r.tmp, scratch)), 2)

    def entries_into(self, tmp, base=None):
        prefix = tmp + os.sep
        return [p for p in state_entries(base) if (p + os.sep).startswith(prefix)]

    def test_NM_I7(self):
        # The helper dropped: the outer environment passes through (and the call is counted).
        self.assertTrue(hasattr(H, "isolated_mise_env"))
        self.control_N19()
        count = []

        def passthrough(tmp_root, fake_mise=None, extra_env=None):
            count.append(1)
            return dict(os.environ, **(extra_env or {}))

        with mock.patch.object(H, "isolated_mise_env", passthrough):
            with self.assertRaises(AssertionError):
                self.control_N19()
        self.assertTrue(count, "the patched helper was never called: the mutant proves nothing")

    def control_N20(self):
        r = self.run_pin(answers=self.a_fixture(ok("[]")))
        self.refused(r)
        skip = ("PATH", "PWD", "OLDPWD", "SHLVL", "_")
        want = dict((k, v) for k, v in os.environ.items() if not k.startswith("MISE_") and k not in skip)
        for call in r.calls():
            got = dict((k, v) for k, v in call["environ"].items() if not k.startswith("MISE_") and k not in skip)
            self.assertEqual(got, want)

    def test_N21(self):
        with open(HELPER_PATH, encoding="utf-8") as f:
            helper = ast.parse(f.read())
        regime = ast.parse(read(os.path.join(REPO, *REGIME_MEMBER.split("/"))))
        fn = [n for n in ast.walk(regime) if isinstance(n, ast.FunctionDef)
              and n.name == "test_G9_every_launch_goes_through_the_accounting"]
        self.assertEqual(len(fn), 1, "G9's spawner test was not found")
        # Fail closed: G9's `spawners` is read in exactly the shape this control understands (one
        # Assign of a set display, one `|=`, no method call on it, no other store) and its prefixes
        # come from exactly one `startswith(<tuple>)`; any other shape is red, never a smaller set.
        named, prefixes = set(), ()
        assigns, augs, stores, uses, tuple_calls = [], [], 0, 0, []
        for n in ast.walk(fn[0]):
            if isinstance(n, ast.Name) and n.id == "spawners" and isinstance(n.ctx, ast.Store):
                stores += 1
            if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "spawners" for t in n.targets):
                assigns.append(n)
            if isinstance(n, ast.AugAssign) and isinstance(n.target, ast.Name) and n.target.id == "spawners":
                augs.append(n)
            if isinstance(n, ast.Attribute) and isinstance(n.value, ast.Name) and n.value.id == "spawners":
                uses += 1
            if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute) and n.func.attr == "startswith" \
                    and n.args and isinstance(n.args[0], ast.Tuple):
                tuple_calls.append(n)
        shape = "G9's spawner set has another shape than this control reads"
        self.assertEqual((len(assigns), len(augs), stores, uses, len(tuple_calls)), (1, 1, 2, 0, 1), shape)
        self.assertTrue(isinstance(assigns[0].value, ast.Set) and len(assigns[0].targets) == 1
                        and isinstance(augs[0].op, ast.BitOr), shape)
        named = set(e.value for e in assigns[0].value.elts if isinstance(e, ast.Constant))
        prefixes = tuple(e.value for e in tuple_calls[0].args[0].elts if isinstance(e, ast.Constant))
        self.assertTrue(named and prefixes, shape)
        stray = []
        for n in ast.walk(helper):
            if isinstance(n, ast.ClassDef):
                stray.append("class %s at %d" % (n.name, n.lineno))
            if isinstance(n, ast.If) and "__name__" in ast.dump(n.test):
                stray.append("__main__ guard at %d" % n.lineno)
            if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute) and isinstance(n.func.value, ast.Name) \
                    and n.func.value.id in ("subprocess", "os") \
                    and (n.func.attr in named or n.func.attr.startswith(prefixes)):
                stray.append("%s.%s at %d" % (n.func.value.id, n.func.attr, n.lineno))
        self.assertEqual(stray, [])
        # The member loads the helper through the one constant, never an assembled name.
        member = ast.parse(read(os.path.abspath(__file__)))
        constants = [n.value for n in ast.walk(member) if isinstance(n, ast.Constant) and isinstance(n.value, str)]
        self.assertEqual(constants.count(HELPER), 1)
        self.assertFalse([c for c in constants if c == "python_pin_testing" + ".py"])

    # ===== N23 the exit line ==============================================================

    def control_N23(self):
        # Every non-zero exit ends its stderr with exactly `python-pin: exit N`; exit 0 ends with `python-pin: admitted X.Y.Z` and no exit line.
        r = self.run_pin(answers=self.a_fixture(ok("[]")))
        self.assertEqual((r.rc, r.lines[-2:]), (1, [PREFIX + "exit 1", ""]), r.err)
        self.assertEqual(r.err.count(PREFIX + "exit "), 1)
        r = self.run_pin(version="3.15.2", pin_line="python 3.15\n")
        self.assertEqual((r.rc, r.lines[-2:]), (2, [PREFIX + "exit 2", ""]), r.err)
        self.assertEqual(r.err.count(PREFIX + "exit "), 1)
        r = self.run_pin(version="3.15.2", pin_line=None)
        self.assertEqual((r.rc, r.lines[-2:]), (2, [PREFIX + "exit 2", ""]), r.err)
        r = self.run_pin(version="3.15.2")
        self.assertEqual((r.rc, r.err), (0, PREFIX + "admitted 3.15.2\n"))
        self.assertEqual(r.err.count(PREFIX + "exit "), 0)
        # An uncaught exception in main(): the traceback, then `python-pin: exit 3` last. The module runs
        # as a script (the __main__ path), with read_pin made to raise.
        tmp = self.tmp()
        root = os.path.join(tmp, "repo")
        raising = self.source().replace(
            "    path = os.path.join(root, PIN_FILE)\n",
            "    path = os.path.join(root, PIN_FILE)\n    raise RuntimeError('boom')\n", 1)
        self.assertNotEqual(raising, self.source())
        module = H.pin_tree(root, raising, "python 3.15.2\n")
        env = H.isolated_mise_env(os.path.join(tmp, "iso"), fake_mise=None)
        rc, _, err = run_child([sys.executable, "-I", "-S", module], env, root)
        lines = err.split("\n")
        self.assertEqual((rc, lines[-2:]), (3, [PREFIX + "exit 3", ""]), err)
        self.assertIn("Traceback (most recent call last)", err)
        self.assertIn("RuntimeError: boom", err)
        self.assertEqual(err.count(PREFIX + "exit "), 1)

    # ===== mutants (tests memo section 6.5) ==============================================

    def test_mutants_are_killed(self):  # sorts after every test_N*: the unmutated passes are recorded by then
        self.assertEqual(sorted(m[0] for m in MUTANTS), sorted(MUTANT_IDS))
        for control in sorted(set(m[3] for m in MUTANTS)):
            if control not in PinCase.passed:  # not recorded by its own test_<id> in this process
                getattr(self, "control_" + control)()  # each killing control passes unmutated
        for mid, find, replace, control in MUTANTS:
            self.assertEqual(LIVE_SOURCE.count(find), 1, "stale row %s: its find text must occur exactly once" % mid)
            self._src = LIVE_SOURCE.replace(find, replace)
            killed = False
            try:
                getattr(self, "control_" + control)()
            except AssertionError:
                killed = True
            finally:
                self._src = None
            self.assertTrue(killed, "mutant %s survived %s" % (mid, control))


def _control_test(cid):
    def test(self):
        getattr(self, "control_" + cid)()
        PinCase.passed.add(cid)  # the unmutated pass the mutants test relies on

    return test


PinCase.passed = set()
for _cid in ("N2", "N3", "N9", "N10", "N11", "N12", "N13", "N14", "N15", "N15_sigint", "N16", "N17", "N18",
             "N19", "N20", "N23"):
    setattr(PinCase, "test_" + _cid, _control_test(_cid))


def warning(name, value):
    return (PREFIX + "warning: %s=%s is set: it may override the tracked install setting for any install "
            "made in this environment" % (name, value))


_ADMIT = 'return (v[0], v[1], v[2]) == pin and v[3] == "final"'
_ROW_MAIN = 'if __name__ == "__main__":'
_BODY = 'body = text[:-1] if text.endswith("\\n") else text\n    head, sep, version = body.partition(" ")'


def _row(src):
    return src + "\n" + _ROW_MAIN


# (ID, find, replace, killing control): the find text occurs exactly once in the live module.
MUTANTS = (
    ("NM1", _ADMIT, 'return (v[0], v[1]) == pin[:2] and v[3] == "final"', "N3"),
    ("NM2", _ADMIT, "return (v[0], v[1], v[2]) == pin", "N3"),
    ("NM3", 'head, sep, version = body.partition(" ")',
     'head, version = (body.split(None, 1) + ["", ""])[:2]; sep = " "', "N2"),
    ("NM4", _BODY, 'body = text.split("\\n")[0]\n    head, sep, version = body.partition(" ")', "N2"),
    ("NM5", "pin = read_pin(root)", "pin = read_pin(os.getcwd())", "N9"),
    ("NM6", '    b = question("b", ["mise", "which", "python3"])',
     '    b = question("b", ["mise", "which", "python3"])\n    question("e", ["mise", "env", "--json"])', "N11"),
    ("NM7", "out, err = proc.communicate(timeout=timeout)", "out, err = proc.communicate()", "N15"),
    ("NM8", "STDERR_CAP = 4000", "STDERR_CAP = 10 ** 9", "N15"),
    ("NM9",
     'return (name.startswith("MISE_PYTHON_") and name != "MISE_PYTHON_VERSION") or name == "MISE_GITHUB_ATTESTATIONS"',
     'return name.startswith("MISE_PYTHON_")', "N13"),
    ("NM10", "import os\nimport sys\n\nPIN_FILE", "import json\nimport os\nimport sys\n\nPIN_FILE", "N18"),
    ("NM11a", _ROW_MAIN, _row("match PIN_FILE:\n    case 1:\n        pass"), "N16"),
    ("NM11b", _ROW_MAIN, _row("try:\n    pass\nexcept* ValueError:\n    pass"), "N16"),
    ("NM11c", _ROW_MAIN, _row("type X = int"), "N16"),
    ("NM11d", _ROW_MAIN, _row("def _f[T](x: T) -> T:\n    return x"), "N16"),
    ("NM11e", _ROW_MAIN, _row("try:\n    pass\nexcept ValueError, TypeError:\n    pass"), "N16"),
    ("NM11f", _ROW_MAIN, _row("x = t'{PIN_FILE}'"), "N16"),
    ("NM12", 'GENERIC_BLOCK % {"pin": pin_s, "root": root}',
     'GENERIC_BLOCK % {"pin": pin_s + " (running %d.%d.%d)" % (v[0], v[1], v[2]), "root": root}', "N10"),
    ("NM13", "if admitted(sys.version_info, pin):",
     "if (ask_mise(os.getcwd(), ORACLE_TIMEOUT_S), admitted(sys.version_info, pin))[1]:", "N3"),
    ("NM14", "os.killpg(proc.pid, 9)", "proc.kill()", "N15"),
    ("NM15", "return _exit(1)", "return 1", "N23"),
    ("NM16", "return _exit(2)", "return 2", "N23"),
    ("NM17", "sys.exit(_guarded_main())", "sys.exit(main())", "N23"),
)
MUTANT_IDS = ["NM%s" % x for x in ("1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11a", "11b", "11c", "11d",
                                    "11e", "11f", "12", "13", "14", "15", "16", "17")]


if __name__ == "__main__":
    unittest.main()
