"""Support module (not a set member) for the tests that reach the pin's refusal (invariant I7).

Plan of record: docs/plans/2026-10-python-pin-pr1-pin-and-gate.md (section 4.8).

It holds no `TestCase`, no `__main__` guard and launches no process: callers start the children
(`[sys.executable, "-c", LAUNCHER, ...]`) with the environment `isolated_mise_env` builds, so a
test that reaches the oracle talks to its own fake `mise` (or to none) and never to the user's.
Every caller loads this file by path through the one constant "scripts/python_pin_testing.py"
joined to the repository root, and calls the functions as attributes of the loaded module.

The fake `mise` and `python3` are one Python script each per process (`_wrapper`), written once
into a process-lifetime temporary directory; `write_fake_mise` / `write_fake_python3` put a symlink
to it named `mise` / `python3` into the caller's bin directory, next to the data the script reads
(found through the directory of `argv[0]`, the symlink's, never its target's). The script's shebang
is `sys.executable` itself: no `/bin/sh` stands between the caller's environment and the fake, so
the fake sees the environment its child was given (a `/bin/sh` drops `DYLD_*` under SIP and some
names under dash). A shebang holds one path: `sys.executable` must have no white space and be at
most 127 bytes (the Linux limit), else `_wrapper` raises.
"""

import atexit
import json
import os
import shutil
import stat
import sys
import tempfile

MISE_DIRS = (("MISE_DATA_DIR", "data"), ("MISE_CACHE_DIR", "cache"), ("MISE_STATE_DIR", "state"), ("MISE_CONFIG_DIR", "config"))
MISE_VARS = tuple(n for n, _ in MISE_DIRS) + ("MISE_GLOBAL_CONFIG_FILE",)
SENTINEL_LOG = "sentinel.log"

# Pin-file texts no reader admits, with the phrase the parser names (N2 parses them, W1 runs them through `problems`).
RED_PIN_TEXTS = (
    ("", "empty"),
    ("\n", "blank line"),
    ("\ufeffpython 3.15.2\n", "byte order mark"),
    ("python 3.15.2\r\n", "carriage return"),
    ("python 3.15.2\ncore:python 3.14.10\n", "2 lines"),
    ("python 3.15.2\n\n", "2 lines"),
    ("# comment\n", "comment"),
    ("# comment\npython 3.15.2\n", "2 lines"),
    ("node 20.0.0\n", "not the tool"),
    ("python 3.15\n", "exact X.Y.Z"),
    ("python latest\n", "exact X.Y.Z"),
    ("python 3.16.0rc1\n", "exact X.Y.Z"),
    ("python 3.15.2 3.14.10\n", "more than one version"),
    ("python 3.15.2 # c\n", "trailing comment"),
    ("python 3.15.2#c\n", "trailing comment"),
    ("python\t3.15.2\n", "tab"),
    ("python  3.15.2\n", "two spaces"),
    ("Python 3.15.2\n", "not the tool"),
    ("python 03.15.2\n", "exact X.Y.Z"),
    ("python 3.15.2 \n", "white space"),
    ("python 3.15.2.1\n", "exact X.Y.Z"),
    ("python \u0663.15.2\n", "exact X.Y.Z"),
    ("# python 3.15.2\n", "comment"),
    ("python 3.15.2\n# c\n", "2 lines"),
    ("node 20\n", "not the tool"),
    ("python 3.15.2rc1\n", "exact X.Y.Z"),
)

# Run as `<sys.executable> -c LAUNCHER <module path> <version> <executable> <timeout>`: loads the
# module at argv[1] by path under a fresh name, replaces sys.version_info by a tuple subclass built
# from argv[2] ("major.minor.micro.releaselevel.serial"), sys.executable when argv[3] is non-empty,
# sets ORACLE_TIMEOUT_S when argv[4] is non-empty, and exits with main().
LAUNCHER = """
import importlib.util
import sys


class _V(tuple):
    major = property(lambda s: s[0])
    minor = property(lambda s: s[1])
    micro = property(lambda s: s[2])
    releaselevel = property(lambda s: s[3])
    serial = property(lambda s: s[4])


_p = sys.argv[2].split(".")
sys.version_info = _V((int(_p[0]), int(_p[1]), int(_p[2]), _p[3], int(_p[4])))
if sys.argv[3]:
    sys.executable = sys.argv[3]
_spec = importlib.util.spec_from_file_location("python_pin_under_test", sys.argv[1])
_mod = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_mod)
if sys.argv[4]:
    _mod.ORACLE_TIMEOUT_S = float(sys.argv[4])
sys.exit(_mod.main())
"""

_FAKE_MISE = r'''import json
import os
import subprocess
import sys
import time

here = os.path.dirname(os.path.abspath(sys.argv[0]))
argv = sys.argv[1:]
with open(os.path.join(here, "calls.jsonl"), "a") as f:
    f.write(json.dumps({"argv": argv, "environ": dict(os.environ)}) + "\n")
with open(os.path.join(here, "answers.json")) as f:
    answers = json.load(f)
answer = answers.get(" ".join(argv), answers.get("*"))
if answer is None:
    sys.stderr.write("fake mise: no answer for %r\n" % (argv,))
    sys.exit(1)
if answer.get("hang"):
    sleeper = subprocess.Popen(["/bin/sleep", "3600"])
    with open(os.path.join(here, "hang.pids"), "a") as f:
        f.write("%d\n%d\n" % (os.getpid(), sleeper.pid))
    while True:
        time.sleep(60)
sys.stdout.write(answer.get("stdout", ""))
sys.stderr.write(answer.get("stderr", ""))
sys.exit(answer.get("rc", 0))
'''

_FAKE_PYTHON3 = r'''import os
import sys

here = os.path.dirname(os.path.abspath(sys.argv[0]))
with open(os.path.join(here, "fake_python3.version")) as f:
    VERSION = f.read().strip()
LAUNCHER = @LAUNCHER@
args = sys.argv[1:]
if "-VV" in args:
    print("Python %s (fake)" % ".".join(VERSION.split(".")[:3]))
    sys.exit(0)
for a in args:
    if a.endswith("python_pin.py"):
        os.execv(sys.executable, [sys.executable, "-c", LAUNCHER, a, VERSION, "", ""])
os.execv(sys.executable, [sys.executable] + args)
'''


def _executable(path, text):
    with open(path, "w") as f:
        f.write(text)
    os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


_WRAPPER_DIR = []


def _wrapper(name, body):
    """The per-process script `name` (shebang = sys.executable, then `body`), written on first use."""
    if not _WRAPPER_DIR:
        d = tempfile.mkdtemp(prefix="python-pin-wrappers-")
        atexit.register(shutil.rmtree, d, True)
        _WRAPPER_DIR.append(d)
    path = os.path.join(_WRAPPER_DIR[0], name)
    if not os.path.exists(path):
        exe = sys.executable
        if len(exe.encode("utf-8")) > 127 or any(c.isspace() for c in exe):
            raise RuntimeError("sys.executable %r cannot be a shebang (white space, or over 127 bytes)" % exe)
        _executable(path, "#!%s\n%s" % (exe, body))
    return path


def _shim(bindir, name, body):
    """bindir/name: a symlink to the per-process script `name`."""
    bindir = str(bindir)
    os.makedirs(bindir, exist_ok=True)
    link = os.path.join(bindir, name)
    if os.path.lexists(link):
        os.remove(link)
    os.symlink(_wrapper(name, body), link)


def _lines(path):
    if not os.path.exists(path):
        return []
    with open(path) as f:
        return [line.rstrip("\n") for line in f if line.strip()]


def isolated_mise_env(tmp_root, fake_mise=None, extra_env=None):
    env = dict((k, v) for k, v in os.environ.items() if not k.startswith("MISE_"))
    tmp_root = str(tmp_root)
    for name, sub in MISE_DIRS:
        os.makedirs(os.path.join(tmp_root, "mise-" + sub), exist_ok=True)
        env[name] = os.path.join(tmp_root, "mise-" + sub)
    env["MISE_GLOBAL_CONFIG_FILE"] = os.path.join(tmp_root, "mise-global.toml")
    open(env["MISE_GLOBAL_CONFIG_FILE"], "w").close()
    outer = os.environ.get("PATH", "")
    if fake_mise is not None:
        env["PATH"] = str(fake_mise) + os.pathsep + outer
    else:
        env["PATH"] = os.pathsep.join(d for d in outer.split(os.pathsep) if shutil.which("mise", path=d) is None)
    env.update(extra_env or {})
    return env


def isolation_problems(environ, root):
    """What is wrong with a child's MISE_* environment as an isolated one under `root` (pure: launches nothing)."""
    out = []
    keys = sorted(k for k in environ if k.startswith("MISE_"))
    if keys != sorted(MISE_VARS):
        out.append("MISE_* keys are %r, expected %r" % (keys, sorted(MISE_VARS)))
    prefix = os.path.realpath(str(root)) + os.sep
    for k in MISE_VARS:
        if k in environ and not os.path.realpath(environ[k]).startswith(prefix):
            out.append("%s=%s is outside %s" % (k, environ[k], root))
    g = environ.get("MISE_GLOBAL_CONFIG_FILE")
    if g is not None and (not os.path.exists(g) or os.path.getsize(g) != 0):
        out.append("MISE_GLOBAL_CONFIG_FILE %s is not an empty file" % g)
    return out


def write_fake_mise(bindir, answers):
    """answers: {" ".join(argv after mise): {"rc", "stdout", "stderr"} or {"hang": True}}; "*" = any other."""
    _shim(bindir, "mise", _FAKE_MISE)
    with open(os.path.join(str(bindir), "answers.json"), "w") as f:
        json.dump(answers, f)


def write_sentinel(bindir, names):
    bindir = str(bindir)
    os.makedirs(bindir, exist_ok=True)
    for name in names:
        _executable(
            os.path.join(bindir, name),
            '#!/bin/sh\necho "$0 $*" >> "%s"\nexit 1\n' % os.path.join(bindir, SENTINEL_LOG),
        )


def read_sentinel(bindir):
    """The lines the sentinels in `bindir` logged: what was run of them (none = nothing reached them)."""
    return _lines(os.path.join(str(bindir), SENTINEL_LOG))


def five(version):
    parts = str(version).split(".")
    return ".".join(parts if len(parts) == 5 else parts + ["final", "0"])


def write_fake_python3(bindir, version):
    """version: "3.15.1" (final, serial 0) or the launcher's "3.15.2.candidate.1"."""
    _shim(bindir, "python3", _FAKE_PYTHON3.replace("@LAUNCHER@", repr(LAUNCHER)))
    with open(os.path.join(str(bindir), "fake_python3.version"), "w") as f:
        f.write(five(version))


def refused_versions(pin):
    """E2's versions for a pin (X, Y, Z): the refused neighbours as (spec, shown, level), derived, never spelled."""
    x, y, z = pin
    out = []
    if z > 0:
        out.append("%d.%d.%d" % (x, y, z - 1))
    out.append("%d.%d.%d" % (x, y, z + 1))
    if y > 0:
        out.append("%d.%d.10" % (x, y - 1))
    out.append("%d.%d.0" % (x, y + 1))
    if (x, y, z) != (3, 9, 6):
        out.append("3.9.6")
    rows = [(v, v, "final") for v in out]
    rows.append(("%d.%d.%d.candidate.1" % pin, "%d.%d.%d" % pin, "candidate"))
    return tuple(rows)


def pin_tree(dst, module_source, pin_line):
    """dst/scripts/python_pin.py (module_source) and dst/.tool-versions (pin_line, as given; None = no pin file)."""
    dst = str(dst)
    os.makedirs(os.path.join(dst, "scripts"), exist_ok=True)
    module = os.path.join(dst, "scripts", "python_pin.py")
    with open(module, "w", encoding="utf-8") as f:
        f.write(module_source)
    if pin_line is not None:
        with open(os.path.join(dst, ".tool-versions"), "wb") as f:
            f.write(pin_line.encode("utf-8"))
    return module


def read_calls(bindir):
    return [json.loads(line) for line in _lines(os.path.join(str(bindir), "calls.jsonl"))]


def read_hang_pids(bindir):
    return [int(line) for line in _lines(os.path.join(str(bindir), "hang.pids"))]
