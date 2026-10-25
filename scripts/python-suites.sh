#!/usr/bin/env bash
#
# `mise run tools-test` and the `tools` job in `.github/workflows/ci.yml`: run
# every tracked Python suite, offline, and fail loudly on any suite that could go
# silently uncollected. Both callers invoke it as `bash scripts/python-suites.sh`,
# so the set, the checks and the offline regime belong to this one file and
# neither caller can skip a part of them. Plan of record:
# docs/plans/2026-07-citation-hygiene-Aiii-suite-scheduler.md (slice A-iii).
#
# The Python half (driver, per-root loader, complement checks) and the offline
# guard are the two heredocs below, so the procedure has one home. They need the
# Python >= 3.9 standard library and nothing else.
#
# ----------------------------------------------------------------------------
# THE PROCEDURE (one way for a suite to enter)
#
# 1. The floor. `python3` (first on PATH) must report >= 3.9, checked before
#    anything else; `python3 -VV` is echoed. The floor is only *measured* by the
#    CI job's "3.9" leg; this assertion is its statement.
# 2. The set is `git ls-files ':(glob)**/test_*.py'` (MEMBER_PATHSPEC) -- tracked files, repo-wide
#    (the `:(glob)` magic is what makes a top-level `test_x.py` count). An empty
#    set fails.
# 3. The complement, by property, over every tracked `.py`
#    (`git ls-files ':(glob)**/*.py'`; the set must be a subset of it) that is
#    NOT a set member (members are run by steps 4-5):
#    (a) a non-member defining a class whose base resolves, within that file, to
#        a `TestCase` / `IsolatedAsyncioTestCase` of any `unittest.*` module
#        fails, naming it -- whether or not any member names it (rename it
#        `test_*.py`). Resolution covers `import unittest [as u]`, any
#        `import unittest.<x>` (which binds `unittest`), `from unittest[.<x>]
#        import TestCase [as T]` / `*` / a submodule, simple and annotated
#        aliases (`T = unittest.TestCase`) and in-file subclass chains; a base
#        reached only through another file is outside (a), by design. A
#        dynamic import of a unittest module (`__import__("unittest...")`,
#        `importlib.import_module("unittest...")`) is a form the resolver
#        cannot follow: it fails, naming the file.
#    (b) a non-member entry-point self-test -- a top-level
#        `if __name__ == "__main__":` (either operand order, a single `==`) AND
#        a string constant `"--self-test"` anywhere in the file, both read from
#        the AST, never by a text grep -- must be NAMED BY PATH in some set
#        member, else the run fails naming it. Named by path = one string
#        constant equal to the entry point's path relative to the repository
#        root, or relative to the member's own directory (posixpath.relpath,
#        exact string equality). A bare filename elsewhere, or a path assembled
#        from several constants, is not a name. (b) checks the naming clause
#        only.
#    A tracked `.py` -- member or not -- that does not parse under the running
#    interpreter fails, naming it.
# 4. The roots are the set's unique dirnames, derived on every run. Each root
#    runs in its own interpreter, NON-recursively: it imports exactly the set
#    members whose dirname is that root, each loaded from its PATH under a
#    unique module name (sys.path[0] = the root, cwd = the repository root), in
#    sorted order; a member that does not come from its own path fails, naming
#    both. The loader never walks a directory. Every root runs under the offline
#    regime below.
# 5. Every member contributes at least one test that executed and passed, else
#    the run fails naming the file. A test counts only if it executed and
#    passed: a failure, an error, a skip, an expected failure or an unexpected
#    success makes the run red, naming the test; so does a `test_*` method whose
#    body we cannot show runs: a generator / async generator function on any
#    class, or a coroutine function on a class that is not
#    IsolatedAsyncioTestCase (calling it runs no body). The only exemption is a
#    coroutine function on an IsolatedAsyncioTestCase.
#
# ----------------------------------------------------------------------------
# THE OFFLINE REGIME
#
# Environment half -- the REGIME KEYS, set for every root's interpreter:
#   PYTHONPATH            the guard's directory, first (inherited entries kept after it)
#   http_proxy/https_proxy  http://127.0.0.1:<a closed local port>
#   NO_PROXY/no_proxy     absent
#   XDG_CACHE_HOME        a fresh empty directory per root
#   GIT_*                 every variable dropped (the driver's own git calls too)
# Before each member is imported and run, the loader restores the regime in one
# step: these keys (os.environ and the process environment) AND the socket guard
# (re-installed from the table the guard publishes), so one member's mutation
# cannot unguard the next member or its children.
#
# Behaviour half -- the socket guard, a `sitecustomize.py` in the guard
# directory, so it is installed at interpreter startup in the loader and in every
# Python child that inherits the regime. In a guarded process these fail with an
# OSError whose message starts "python-suites offline regime:":
#   socket.socket.connect, socket.socket.connect_ex (every family except AF_UNIX:
#   AF_INET, AF_INET6 and any family not known here are denied; AF_UNIX is local
#   IPC, e.g. multiprocessing's forkserver and Manager, and goes through),
#   socket.getaddrinfo, socket.gethostbyname, socket.gethostbyname_ex,
#   socket.gethostbyaddr, socket.getnameinfo.
# The guard installs exactly once per process (a marker on the socket module;
# a nested driver's guard sees it and skips), then chain-loads the next
# `sitecustomize` on sys.path after its own directory, so one that would run
# without the guard still runs. If the guard itself fails to install, the
# process exits 97 (fail closed) -- CPython's `site` would otherwise print the
# error and carry on unguarded. The loader also refuses to run (exit 98) unless
# the marker says the guard is installed exactly once.
#
# THE STATED LIMIT -- the unguarded-child class: a child whose environment
# differs from the regime (of the nearest enclosing driver run) on any regime key
# -- the guard's PYTHONPATH entry missing or not first, a proxy key changed,
# NO_PROXY/no_proxy present, XDG_CACHE_HOME changed -- however it came to differ,
# or that runs Python with -I / -E / -S, or is not Python. Also outside the claim,
# in a guarded process: socket-layer paths not listed above (`_socket` used
# directly, a raw `sendto`). A green run is the offline measurement for the
# loader and every child outside the class, over the functions listed above.
#
# ----------------------------------------------------------------------------
# THE WRAPPER CONTRACT -- how a command-shaped self-test enters the set
#
# A self-test that is a command rather than a unittest module enters as a thin
# tracked `test_*.py` beside it (the wrapper). The contract:
#   - the command's self-test is reached by the flag `--self-test` from a
#     top-level `if __name__ == "__main__":` guard. `--self-test` is the one
#     normative spelling; `--selftest`, or a guard-less `__main__.py`, does not
#     conform (3(b) does not chase it);
#   - the command's own file is not named `test_*.py` (a member is excluded
#     from 3(b), so its self-test would never be required to run) and defines
#     no unittest.TestCase (3(a) would keep it red);
#   - the wrapper names the command's file by path (step 3(b));
#   - it runs the command through `sys.executable`, without -I / -E / -S, in an
#     environment equal to the regime on every regime key: it may add or change
#     only non-regime keys, and PYTHONPATH only by appending after the guard's
#     entry -- whether through an `env=` it builds or by mutating its own
#     os.environ / os.putenv before spawning;
#   - it asserts exit 0 AND nonzero control and mutant counts as the self-test
#     reports them (a self-test that ran no control is not a pass);
#   - it stays offline.
# Step 3(b) checks the naming clause; the other clauses are the wrapper's own
# test body, reviewed in the wrapper's PR.
#
# The pins for every clause above live in scripts/test_python_suites_{driver,regime,ci}.py
# (set members, so they run here too, with the helper modules python_suites_fixtures.py /
# python_suites_readers.py, which are not members; none ever runs this driver on the live tree --
# only on scratch repositories).

set -euo pipefail

# Resolve the script's real location (a symlink on PATH must not move the root),
# then the repository root, the same way scripts/trip-wires.sh does.
src="$0"
case "$src" in
  /*) ;;
  *) src="$(cd -P "$(dirname "$src")" && pwd)/$(basename "$src")" ;;
esac
while [ -L "$src" ]; do
  link_dir="$(cd -P "$(dirname "$src")" && pwd)"
  src="$(readlink "$src")"
  case "$src" in
    /*) ;;
    *) src="$link_dir/$src" ;;
  esac
done
root="$(cd -P "$(dirname "$src")/.." && pwd)"
cd "$root"

# Step 1: the floor, before anything else. Written so any python3 can run it, and
# run with -I -S: the floor is a property of the interpreter, so no inherited
# PYTHONPATH or sitecustomize (an enclosing driver's guard included) takes part.
if ! command -v python3 >/dev/null 2>&1; then
  echo "python-suites: FAIL floor: no python3 on PATH (the interpreter floor is Python 3.9)" >&2
  exit 1
fi
if ! ver="$(python3 -I -S -c 'import sys; v = sys.version_info; sys.stdout.write("%d.%d.%d" % (v[0], v[1], v[2])); sys.exit(0 if tuple(v[:2]) >= (3, 9) else 1)')"; then
  echo "python-suites: FAIL floor: the interpreter floor is Python 3.9; python3 reports ${ver:-no version}" >&2
  exit 1
fi
echo "python-suites: python3 -VV: $(python3 -VV 2>&1 | tr '\n' ' ')"

tmp="$(mktemp -d "${TMPDIR:-/tmp}/python-suites.XXXXXX")"
case $tmp in /*) ;; *) tmp="$PWD/$tmp" ;; esac
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/driver" "$tmp/guard"

cat > "$tmp/driver/python_suites.py" <<'PY_DRIVER'
"""Python half of scripts/python-suites.sh -- see that file's header."""

import ast
import importlib.util
import inspect
import json
import os
import posixpath
import subprocess
import sys
import traceback
import unittest
import warnings

PREFIX = "python-suites:"
# The set definition, in one place (the pins read it from here): the git
# pathspec the driver lists, and the regex for the same names on a path string.
MEMBER_PATHSPEC = ":(glob)**/test_*.py"
MEMBER_RE = r"(^|/)test_[^/]*\.py$"
ALL_PY_PATHSPEC = ":(glob)**/*.py"
SELF_TEST_FLAG = "--self-test"
TESTCASE_BASES = ("TestCase", "IsolatedAsyncioTestCase")
GUARD_MARK = "_python_suites_guard_installs"
GUARD_TABLE = "_python_suites_guard_table"  # [(owner, name, guarded value)], published by the guard
REGIME_KEYS = (
    "PYTHONPATH",
    "http_proxy",
    "https_proxy",
    "NO_PROXY",
    "no_proxy",
    "XDG_CACHE_HOME",
)
MEMBER_MODULE_PREFIX = "python_suites_member_"


def say(msg):
    print("%s %s" % (PREFIX, msg), flush=True)


# --------------------------------------------------------------------------
# Step 3 predicates (also imported by scripts/test_python_suites_driver.py, P6).


def parse(source, path):
    """ast.parse under the running interpreter; SyntaxError propagates."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return ast.parse(source, filename=path)


def string_constants(tree):
    return {
        n.value
        for n in ast.walk(tree)
        if isinstance(n, ast.Constant) and isinstance(n.value, str)
    }


def _is_main_guard(test):
    if not (
        isinstance(test, ast.Compare)
        and len(test.ops) == 1
        and isinstance(test.ops[0], ast.Eq)
    ):
        return False
    a, b = test.left, test.comparators[0]

    def is_name(n):
        return isinstance(n, ast.Name) and n.id == "__name__"

    def is_main(n):
        return isinstance(n, ast.Constant) and n.value == "__main__"

    return (is_name(a) and is_main(b)) or (is_main(a) and is_name(b))


def is_self_test_entry_point(tree):
    """3(b)'s population: a top-level __main__ guard AND the '--self-test' constant."""
    guard = any(
        isinstance(n, ast.If) and _is_main_guard(n.test) for n in tree.body
    )
    return guard and SELF_TEST_FLAG in string_constants(tree)


def names_by_path(constants, entry, member):
    """Does a member whose string constants are `constants` name `entry` by path?"""
    if entry in constants:
        return True
    rel = posixpath.relpath(entry, posixpath.dirname(member) or ".")
    return rel in constants


def _is_unittest_name(name):
    return name == "unittest" or name.startswith("unittest.")


def testcase_analysis(tree):
    """3(a): ([(name, lineno)], [unresolved]) for one file.

    The first list is the classes whose base resolves in-file to a TestCase /
    IsolatedAsyncioTestCase of any `unittest.*` module. The second lists the
    unittest import forms this resolver cannot follow (a dynamic import of a
    unittest module): the caller fails closed on any."""
    modules = set()  # names bound to unittest or a unittest.* module
    direct = set()  # names bound to TestCase / IsolatedAsyncioTestCase
    unresolved = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for a in node.names:
                if _is_unittest_name(a.name):
                    # `import unittest.mock` binds `unittest`; `... as m` binds the submodule.
                    modules.add(a.asname or "unittest")
        elif isinstance(node, ast.ImportFrom):
            if node.level or not node.module or not _is_unittest_name(node.module):
                continue
            for a in node.names:
                if a.name == "*":
                    direct.update(TESTCASE_BASES)
                elif a.name in TESTCASE_BASES:
                    direct.add(a.asname or a.name)
                else:
                    # any other imported name may be a submodule (`from unittest import mock`)
                    modules.add(a.asname or a.name)
        elif isinstance(node, ast.Call):
            fn = node.func
            callee = fn.id if isinstance(fn, ast.Name) else fn.attr if isinstance(fn, ast.Attribute) else None
            if (
                callee in ("__import__", "import_module")
                and node.args
                and isinstance(node.args[0], ast.Constant)
                and isinstance(node.args[0].value, str)
                and _is_unittest_name(node.args[0].value)
            ):
                unresolved.append("%s(%r) (line %d)" % (callee, node.args[0].value, node.lineno))

    def is_module(expr):
        if isinstance(expr, ast.Name):
            return expr.id in modules
        return isinstance(expr, ast.Attribute) and is_module(expr.value)

    def resolves(expr):
        if isinstance(expr, ast.Name):
            return expr.id in direct
        if isinstance(expr, ast.Attribute):
            return expr.attr in TESTCASE_BASES and is_module(expr.value)
        return False

    def alias_targets(node):
        if isinstance(node, ast.Assign) and resolves(node.value):
            return [t.id for t in node.targets if isinstance(t, ast.Name)]
        if (
            isinstance(node, ast.AnnAssign)
            and node.value is not None
            and isinstance(node.target, ast.Name)
            and resolves(node.value)
        ):
            return [node.target.id]
        return []

    found = {}
    changed = True
    while changed:
        changed = False
        for node in ast.walk(tree):
            if isinstance(node, ast.ClassDef) and node.name not in found:
                if any(resolves(b) for b in node.bases):
                    found[node.name] = node.lineno
                    direct.add(node.name)
                    changed = True
            else:
                for name in alias_targets(node):
                    if name not in direct:
                        direct.add(name)
                        changed = True
    return sorted(found.items(), key=lambda kv: kv[1]), unresolved


def complement_failures(members, trees, all_py):
    """Step 3 over non-members. `trees` maps every parsed tracked .py to its AST."""
    member_set = set(members)
    member_constants = {
        m: string_constants(trees[m]) for m in members if m in trees
    }
    failures = []
    for path in all_py:  # git_lines returns it sorted
        if path in member_set or path not in trees:
            continue
        tree = trees[path]
        classes, unresolved = testcase_analysis(tree)
        if unresolved:
            failures.append(
                "%s imports unittest in a form step 3(a) cannot resolve: %s"
                " (use a plain `import unittest` / `from unittest import ...`)"
                % (path, ", ".join(unresolved))
            )
        if classes:
            failures.append(
                "%s defines unittest.TestCase subclass(es) %s but is not a set member"
                " (step 3(a): rename it test_*.py)"
                % (path, ", ".join("%s (line %d)" % c for c in classes))
            )
        if is_self_test_entry_point(tree) and not any(
            names_by_path(consts, path, m) for m, consts in member_constants.items()
        ):
            failures.append(
                "%s is a %s entry point that no set member names by path"
                " (step 3(b): add a tracked test_*.py wrapper per the contract in"
                " scripts/python-suites.sh)" % (path, SELF_TEST_FLAG)
            )
    return failures


# --------------------------------------------------------------------------
# The driver.


def without_git_env(env):
    """`env` minus every GIT_* variable: the caller's hook / worktree context
    must neither relocate the driver's own git calls (GIT_DIR, GIT_INDEX_FILE,
    ...), reinterpret its pathspecs (GIT_NOGLOB_PATHSPECS, ...) nor reach a
    member's git children."""
    return {k: v for k, v in env.items() if not k.startswith("GIT_")}


def git_lines(root, *args):
    env = without_git_env(os.environ)
    out = subprocess.run(
        ["git", "-C", root] + list(args),
        env=env,
        stdout=subprocess.PIPE,
        check=True,
    ).stdout
    return sorted(p for p in out.decode("utf-8", "surrogateescape").split("\0") if p)


def closed_local_port():
    import socket

    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]
    finally:
        s.close()


def regime_env(base, guard_dir, port, cache_dir):
    env = without_git_env(base)
    inherited = env.get("PYTHONPATH")
    env["PYTHONPATH"] = guard_dir + (os.pathsep + inherited if inherited else "")
    env["http_proxy"] = "http://127.0.0.1:%d" % port
    env["https_proxy"] = "http://127.0.0.1:%d" % port
    env.pop("NO_PROXY", None)
    env.pop("no_proxy", None)
    env["XDG_CACHE_HOME"] = cache_dir
    return env


def run(root, tmp):
    failures = []

    members = git_lines(root, "ls-files", "-z", "--", MEMBER_PATHSPEC)
    if not members:
        say("FAIL set: git ls-files '%s' is empty -- nothing to run" % MEMBER_PATHSPEC)
        return 1
    all_py = git_lines(root, "ls-files", "-z", "--", ALL_PY_PATHSPEC)
    outside = sorted(set(members) - set(all_py))
    if outside:
        say("FAIL set: member(s) %s missing from the tracked .py population" % ", ".join(outside))
        return 1

    trees = {}
    for path in all_py:
        try:
            with open(os.path.join(root, path), "rb") as f:
                trees[path] = parse(f.read(), path)
        except (OSError, SyntaxError, ValueError) as e:
            failures.append(
                "%s does not parse under Python %d.%d (%s: %s)"
                % (path, sys.version_info[0], sys.version_info[1], type(e).__name__, e)
            )
    failures.extend(complement_failures(members, trees, all_py))

    roots = {}
    for m in members:
        roots.setdefault(posixpath.dirname(m), []).append(m)
    say(
        "%d member(s) in %d root(s); %d tracked .py parsed"
        % (len(members), len(roots), len(trees))
    )

    guard_dir = os.path.join(tmp, "guard")
    script = os.path.abspath(__file__)
    total = 0
    for i, (rdir, rmembers) in enumerate(sorted(roots.items())):
        cache_dir = os.path.join(tmp, "xdg-cache-%d" % i)
        os.mkdir(cache_dir)
        out = os.path.join(tmp, "result-%d.json" % i)
        env = regime_env(os.environ, guard_dir, closed_local_port(), cache_dir)
        say("root %s (%d member(s))" % (rdir or ".", len(rmembers)))
        proc = subprocess.run(
            [sys.executable, script, "load", root, rdir, out] + rmembers,
            env=env,
            cwd=root,
        )
        if proc.returncode != 0 or not os.path.exists(out):
            failures.append(
                "root %s: loader exited %d without a result" % (rdir or ".", proc.returncode)
            )
            continue
        with open(out) as f:
            result = json.load(f)
        for m in rmembers:
            r = result.get(m)
            if r is None:
                failures.append("%s: no result from the loader" % m)
                continue
            for p in r["problems"]:
                failures.append(p)
            if r["passed"] == 0:
                failures.append("%s: no test executed and passed (step 5)" % m)
            total += r["passed"]
            say(
                "%s %s: %d test(s) passed"
                % ("ok  " if r["passed"] and not r["problems"] else "FAIL", m, r["passed"])
            )

    if failures:
        for f in failures:
            say("FAIL " + f)
        say("RED -- %d failure(s)" % len(failures))
        return 1
    say("GREEN -- %d member(s), %d test(s) passed, offline" % (len(members), total))
    return 0


# --------------------------------------------------------------------------
# The per-root loader (runs under the regime).


def _restore_regime(snapshot, guard):
    """The one way the regime is put back: the REGIME_KEYS environment and the
    socket guard (re-installed from the guard's own table), so a member that
    undid either in-process cannot leave a later member unguarded. The install
    marker is left as the member left it: stamping it here would hide a member
    (or a chained sitecustomize) that changed the count."""
    for k, v in snapshot.items():
        if v is None:
            os.environ.pop(k, None)
            if hasattr(os, "unsetenv"):
                os.unsetenv(k)
        else:
            os.environ[k] = v
    for owner, name, value in guard:
        setattr(owner, name, value)


class _Result(unittest.TestResult):
    def __init__(self, member, modname):
        super().__init__()
        self.member = member
        self.modname = modname
        self.passed = 0
        self.problems = []

    def _name(self, test):
        tid = test.id()
        if tid.startswith(self.modname + "."):
            tid = tid[len(self.modname) + 1 :]
        return "%s::%s" % (self.member, tid)

    def _bad(self, kind, test, detail):
        name = self._name(test)
        say("test %s %s" % (kind, name))
        if detail:
            sys.stdout.write(detail if detail.endswith("\n") else detail + "\n")
        self.problems.append("%s %s" % (kind, name))

    def addSuccess(self, test):
        super().addSuccess(test)
        self.passed += 1
        say("test ok %s" % self._name(test))

    def addFailure(self, test, err):
        super().addFailure(test, err)
        self._bad("FAILED", test, self._exc_info_to_string(err, test))

    def addError(self, test, err):
        super().addError(test, err)
        self._bad("ERROR", test, self._exc_info_to_string(err, test))

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        self._bad("SKIPPED", test, "skip reason: %s" % reason)

    def addExpectedFailure(self, test, err):
        super().addExpectedFailure(test, err)
        self._bad("EXPECTED-FAILURE", test, "")

    def addUnexpectedSuccess(self, test):
        super().addUnexpectedSuccess(test)
        self._bad("UNEXPECTED-SUCCESS", test, "")

    def addSubTest(self, test, subtest, err):
        super().addSubTest(test, subtest, err)
        if err is not None:
            self._bad("FAILED", subtest, self._exc_info_to_string(err, subtest))


def _never_runs_body(test):
    """The function kind of a test method whose body we cannot show runs, else
    None: a generator / async generator function on any class, or a coroutine
    function on a class that is not IsolatedAsyncioTestCase (calling it only
    creates the coroutine / generator). The one exemption is a coroutine
    function on an IsolatedAsyncioTestCase, which awaits it."""
    method = getattr(test, getattr(test, "_testMethodName", ""), None)
    if method is None:
        return None
    try:
        method = inspect.unwrap(method)
    except ValueError:
        pass
    for kind, check in (
        ("coroutine function", inspect.iscoroutinefunction),
        ("generator function", inspect.isgeneratorfunction),
        ("async generator function", inspect.isasyncgenfunction),
    ):
        if check(method):
            if kind == "coroutine function" and isinstance(test, unittest.IsolatedAsyncioTestCase):
                return None
            return kind
    return None


def _flatten(suite):
    for t in suite:
        if isinstance(t, unittest.TestSuite):
            yield from _flatten(t)
        else:
            yield t


def load(root, rdir, out, members):
    import socket

    if getattr(socket, GUARD_MARK, 0) != 1:
        say(
            "FAIL root %s: the offline guard is not installed exactly once"
            " (marker %r) -- refusing to run" % (rdir or ".", getattr(socket, GUARD_MARK, 0))
        )
        sys.exit(98)
    guard = list(getattr(socket, GUARD_TABLE, ()))
    if not guard:
        say("FAIL root %s: the offline guard published no table -- refusing to run" % (rdir or "."))
        sys.exit(98)
    snapshot = {k: os.environ.get(k) for k in REGIME_KEYS}
    rootdir = os.path.join(root, rdir)
    here = os.path.dirname(os.path.abspath(__file__))
    if sys.path and os.path.abspath(sys.path[0] or ".") == here:
        sys.path[0] = rootdir
    else:
        sys.path.insert(0, rootdir)
    os.chdir(root)
    results = {}
    for n, member in enumerate(sorted(members)):
        _restore_regime(snapshot, guard)
        # Loaded from its PATH under a unique name: a same-named module elsewhere
        # on sys.path or in sys.modules can never stand in for the member.
        modname = "%s%d_%s" % (MEMBER_MODULE_PREFIX, n, posixpath.splitext(posixpath.basename(member))[0])
        r = _Result(member, modname)
        try:
            path = os.path.join(root, member)
            spec = importlib.util.spec_from_file_location(modname, path)
            mod = importlib.util.module_from_spec(spec)
            sys.modules[modname] = mod
            spec.loader.exec_module(mod)
            if os.path.realpath(getattr(mod, "__file__", "") or "") != os.path.realpath(path):
                raise ImportError(
                    "member %s loaded from %s, not from %s"
                    % (member, getattr(mod, "__file__", None), path)
                )
            loader = unittest.TestLoader()
            suite = loader.loadTestsFromModule(mod)
            for t in _flatten(suite):
                kind = _never_runs_body(t)
                if kind:
                    where = (
                        " on a class that is not IsolatedAsyncioTestCase"
                        if kind == "coroutine function"
                        else ""
                    )
                    r.problems.append(
                        "%s::%s is a %s%s (calling it runs no body)"
                        % (member, t.id()[len(modname) + 1 :], kind, where)
                    )
            for e in loader.errors:
                r.problems.append("%s: loader error: %s" % (member, e.strip().splitlines()[-1]))
                sys.stdout.write(e)
            suite.run(r)
        except KeyboardInterrupt:
            raise
        except BaseException:
            tb = traceback.format_exc()
            sys.stdout.write(tb)
            r.problems.append(
                "%s: import failed (%s)" % (member, tb.strip().splitlines()[-1])
            )
        results[member] = {"passed": r.passed, "problems": r.problems}
        sys.stdout.flush()
    _restore_regime(snapshot, guard)
    with open(out, "w") as f:
        json.dump(results, f)
    return 0


def main(argv):
    if argv[:1] == ["run"] and len(argv) == 3:
        return run(argv[1], argv[2])
    if argv[:1] == ["load"] and len(argv) >= 4:
        return load(argv[1], argv[2], argv[3], argv[4:])
    say("FAIL usage: python_suites.py run ROOT TMP | load ROOT RDIR OUT MEMBER...")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
PY_DRIVER

cat > "$tmp/guard/sitecustomize.py" <<'PY_GUARD'
# The python-suites offline guard (scripts/python-suites.sh). Installed through
# PYTHONPATH; runs at interpreter startup, before any member loads.
import os
import sys

_MARK = "_python_suites_guard_installs"
_TABLE = "_python_suites_guard_table"


class _OfflineRegimeError(OSError):
    pass


def _deny(what):
    def deny(*args, **kwargs):
        raise _OfflineRegimeError("python-suites offline regime: %s denied" % what)

    deny.__name__ = what.rsplit(".", 1)[-1]
    return deny


def _deny_unless_unix(what, original):
    """connect / connect_ex: AF_UNIX (local IPC: multiprocessing's forkserver
    and Manager, ...) goes through; every other family -- AF_INET, AF_INET6 and
    any family not known here -- is denied (fail closed)."""
    import socket

    unix = getattr(socket, "AF_UNIX", None)
    deny = _deny(what)

    def guarded(self, *args, **kwargs):
        if unix is not None and self.family == unix:
            return original(self, *args, **kwargs)
        return deny()

    guarded.__name__ = what.rsplit(".", 1)[-1]
    return guarded


def _install():
    import socket

    if getattr(socket, _MARK, 0):
        return
    table = [
        (socket.socket, "connect", _deny_unless_unix("socket.connect", socket.socket.connect)),
        (socket.socket, "connect_ex", _deny_unless_unix("socket.connect_ex", socket.socket.connect_ex)),
    ] + [
        (socket, name, _deny("socket." + name))
        for name in (
            "getaddrinfo",
            "gethostbyname",
            "gethostbyname_ex",
            "gethostbyaddr",
            "getnameinfo",
        )
    ]
    for owner, name, fn in table:
        setattr(owner, name, fn)
    setattr(socket, _TABLE, table)  # the loader re-installs it before each member
    setattr(socket, _MARK, getattr(socket, _MARK, 0) + 1)


def _chain_next_sitecustomize():
    here = os.path.dirname(os.path.abspath(__file__))
    rest = None
    for i, p in enumerate(sys.path):
        if os.path.abspath(p or ".") == here:
            rest = sys.path[i + 1 :]
            break
    if rest is None:
        return
    import importlib.util
    from importlib.machinery import PathFinder

    spec = PathFinder.find_spec("sitecustomize", rest)
    if spec is None or spec.loader is None:
        return
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)


try:
    _install()
except BaseException as e:  # fail closed: `site` would print this and carry on
    sys.stderr.write(
        "python-suites: FAIL the offline guard did not install (%s: %s);"
        " refusing to run unguarded\n" % (type(e).__name__, e)
    )
    sys.stderr.flush()
    os._exit(97)
_chain_next_sitecustomize()
PY_GUARD

python3 -I "$tmp/driver/python_suites.py" run "$root" "$tmp"
