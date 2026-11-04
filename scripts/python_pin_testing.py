"""Support module (not a set member) for the tests that reach the pin's refusal (invariant I7).

Plan of record: docs/plans/2026-10-python-pin-pr1-pin-and-gate.md (section 4.8).

It holds no `TestCase`, no `__main__` guard and launches no process: callers start the children
(`[sys.executable, "-c", LAUNCHER, ...]`) with the environment `isolated_mise_env` builds, so a
test that reaches the oracle talks to its own fake `mise` (or to none) and never to the user's.
Every caller loads this file by path through the one constant "scripts/python_pin_testing.py"
joined to the repository root, and calls the functions as attributes of the loaded module.
"""

import json
import os
import shutil
import stat
import sys

MISE_VARS = ("MISE_DATA_DIR", "MISE_CACHE_DIR", "MISE_STATE_DIR", "MISE_CONFIG_DIR", "MISE_GLOBAL_CONFIG_FILE")

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

_FAKE_MISE = r'''
import json
import os
import subprocess
import sys
import time

here = os.path.dirname(os.path.abspath(__file__))
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

_FAKE_PYTHON3 = r'''
import os
import sys

VERSION = %(version)r
LAUNCHER = %(launcher)r
args = sys.argv[1:]
if "-VV" in args:
    print("Python %%s (fake)" %% ".".join(VERSION.split(".")[:3]))
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


def isolated_mise_env(tmp_root, fake_mise=None, extra_env=None):
    env = dict((k, v) for k, v in os.environ.items() if not k.startswith("MISE_"))
    tmp_root = str(tmp_root)
    for name, sub in (("MISE_DATA_DIR", "data"), ("MISE_CACHE_DIR", "cache"), ("MISE_STATE_DIR", "state"), ("MISE_CONFIG_DIR", "config")):
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


def write_fake_mise(bindir, answers):
    """answers: {" ".join(argv after mise): {"rc", "stdout", "stderr"} or {"hang": True}}; "*" = any other."""
    bindir = str(bindir)
    os.makedirs(bindir, exist_ok=True)
    with open(os.path.join(bindir, "answers.json"), "w") as f:
        json.dump(answers, f)
    with open(os.path.join(bindir, "fake_mise.py"), "w") as f:
        f.write(_FAKE_MISE)
    _executable(
        os.path.join(bindir, "mise"),
        '#!/bin/sh\nexec "%s" "%s" "$@"\n' % (sys.executable, os.path.join(bindir, "fake_mise.py")),
    )


def write_sentinel(bindir, names):
    bindir = str(bindir)
    os.makedirs(bindir, exist_ok=True)
    for name in names:
        _executable(
            os.path.join(bindir, name),
            '#!/bin/sh\necho "$0 $*" >> "%s"\nexit 1\n' % os.path.join(bindir, "sentinel.log"),
        )


def _five(version):
    parts = str(version).split(".")
    return ".".join(parts if len(parts) == 5 else parts + ["final", "0"])


def write_fake_python3(bindir, version):
    """version: "3.15.1" (final, serial 0) or the launcher's "3.15.2.candidate.1"."""
    bindir = str(bindir)
    os.makedirs(bindir, exist_ok=True)
    with open(os.path.join(bindir, "fake_python3.py"), "w") as f:
        f.write(_FAKE_PYTHON3 % {"version": _five(version), "launcher": LAUNCHER})
    _executable(
        os.path.join(bindir, "python3"),
        '#!/bin/sh\nexec "%s" "%s" "$@"\n' % (sys.executable, os.path.join(bindir, "fake_python3.py")),
    )


def pin_tree(dst, module_source, pin_line):
    """dst/scripts/python_pin.py (module_source) and dst/.tool-versions (pin_line, written as given)."""
    dst = str(dst)
    os.makedirs(os.path.join(dst, "scripts"), exist_ok=True)
    module = os.path.join(dst, "scripts", "python_pin.py")
    with open(module, "w", encoding="utf-8") as f:
        f.write(module_source)
    with open(os.path.join(dst, ".tool-versions"), "wb") as f:
        f.write(pin_line if isinstance(pin_line, bytes) else pin_line.encode("utf-8"))
    return module


def read_calls(bindir):
    path = os.path.join(str(bindir), "calls.jsonl")
    if not os.path.exists(path):
        return []
    with open(path) as f:
        return [json.loads(line) for line in f if line.strip()]


def read_hang_pids(bindir):
    path = os.path.join(str(bindir), "hang.pids")
    if not os.path.exists(path):
        return []
    with open(path) as f:
        return [int(line) for line in f if line.strip()]
