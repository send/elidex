"""T5 clauses 2-5: the tracked-tree rule over mise's tool-configuration files.

Plan of record: docs/plans/2026-10-python-pin-pr0-tracked-mise-config.md (section 4.2 / 4.3).

A fail-safe predicate over tracked NAMES and TEXT. It models no part of mise's configuration
surface: a tracked file whose path has a component starting `mise` / `rtx` (case-folded, one
leading dot ignored), or a `.tool-versions` / `.python-version*` file, is a member, and a member
may not spell `python` outside the admitted occurrences. The version is spelled once (the
root `.tool-versions`, clause 1, added later); a second tracked spelling is red.

This module is pure: it launches nothing and holds no test. The set member
`scripts/test_tracked_tool_config.py` supplies the file list (its own `git ls-files`) and the
controls. Stdlib only, Python 3.9 syntax.
"""

import errno
import os
import re
import stat
import string

ROOT_PIN = ".tool-versions"
ROOT_CONFIG = "mise.toml"
ADMITTED_LINES = (
    "python.compile = false",
    "python.github_attestations = true",
)
# Every top-level name of this module except `problems` (the composition point PR-1 edits, with C0's two lists); C0 holds it.
CLAUSES_2_TO_5 = (
    "ROOT_PIN",
    "ROOT_CONFIG",
    "ADMITTED_LINES",
    "CLAUSES_2_TO_5",
    "_WORD",
    "_BEFORE_FORBIDDEN",
    "_AFTER_FORBIDDEN",
    "_PYTHON",
    "_CUT",
    "_REMEDY_RC",
    "_REMEDY_VERSION_FILE",
    "_REMEDY_OCCURRENCE",
    "_REMEDY_MISSING",
    "_REMEDY_UNREADABLE",
    "Unclassifiable",
    "fold",
    "_basename",
    "is_member",
    "_show",
    "finding",
    "red_outright",
    "admitted_line",
    "_bounded",
    "admits_occurrence",
    "content_findings",
    "_unreadable",
    "member_problems",
    "_errno_kind",
    "worktree_reader",
)

_WORD = string.ascii_letters + string.digits + "_"
_BEFORE_FORBIDDEN = frozenset(_WORD + ".-/@:+~")
_AFTER_FORBIDDEN = frozenset(_WORD + "-/@+~")
_PYTHON = re.compile("python", re.IGNORECASE)
_CUT = 120

_REMEDY_RC = (
    "a tracked mise rc file is never admitted (it chooses which files mise reads); "
    "remove it from the index"
)
_REMEDY_VERSION_FILE = (
    "the Python version is spelled only in the root .tool-versions; remove this file "
    "(another tool goes into mise.toml [tools])"
)
_REMEDY_OCCURRENCE = (
    "the Python version is spelled only in the root .tool-versions; reword (a description "
    "need not say Python; a tracked path is admitted), or rename the file so no path "
    "component starts with mise / rtx; for a new install setting, extend clause 4 (b) "
    "together with I6's assertion in the same PR"
)
_REMEDY_MISSING = (
    "the file is tracked but absent from the worktree (or, under a sparse checkout, outside the cone); restore it "
    "(git checkout -- <path>) or remove it from the index (git rm <path>)"
)
_REMEDY_UNREADABLE = (
    "track the file itself as UTF-8 text under a UTF-8 name (not a symlink or a "
    "submodule), or rename it out of the member names"
)


class Unclassifiable(Exception):
    """The text mise would read cannot be seen: kind is missing / symlink / not-a-file / unreadable: <errno>."""

    def __init__(self, kind):
        Exception.__init__(self, kind)
        self.kind = kind


def fold(c):
    """Case-fold one path component and remove ONE leading dot."""
    c = c.casefold()
    return c[1:] if c.startswith(".") else c


def _basename(path):
    return path.rsplit("/", 1)[-1]


def is_member(path):
    """Clause 2."""
    if any(fold(c).startswith(("mise", "rtx")) for c in path.split("/")):
        return True
    base = _basename(path).casefold()
    return base == ".tool-versions" or base.startswith(".python-version")


def _show(s):
    return s.encode("utf-8", "backslashreplace").decode("utf-8")


def finding(clause, path, line, what, text, remedy):
    """Clause 5: one diagnostic string."""
    out = "T5 clause %s: %s" % (clause, _show(path))
    if line is not None:
        out += ":%d" % line
    out += ": " + what
    if text is not None:
        out += ": " + _show(text.strip()[:_CUT])
    return out + " -- " + remedy


def red_outright(path):
    """Clause 3: a finding string, or None. Reads no text."""
    if any(fold(c).startswith("miserc") for c in path.split("/")):
        return finding("3a", path, None, "tracked mise rc file", None, _REMEDY_RC)
    base = _basename(path).casefold()
    if base == ".tool-versions" and path != ROOT_PIN:
        return finding(
            "3b", path, None, "tracked .tool-versions that is not the root pin file", None,
            _REMEDY_VERSION_FILE,
        )
    if base.startswith(".python-version"):
        return finding("3c", path, None, "tracked .python-version file", None, _REMEDY_VERSION_FILE)
    return None


def admitted_line(path, line):
    """Clause 4 (b): exact text, root mise.toml only."""
    return path == ROOT_CONFIG and line in ADMITTED_LINES


def _bounded(line, start, end):
    if start > 0 and line[start - 1] in _BEFORE_FORBIDDEN:
        return False
    if end < len(line):
        after = line[end]
        if after in _AFTER_FORBIDDEN:
            return False
        if after == "." and end + 1 < len(line) and line[end + 1] in _WORD:
            return False
    return True


def admits_occurrence(line, s, tracked):
    """Clause 4 (a): the `python` at line[s:s+6] lies inside a tracked path containing `/`."""
    for p in tracked:
        if "/" not in p:
            continue
        a = line.find(p)
        while a != -1:
            if a <= s and s + 6 <= a + len(p) and _bounded(line, a, a + len(p)):
                return True
            a = line.find(p, a + 1)
    return False


def content_findings(path, text, tracked):
    """Clause 4 over one member's text: one finding per offending line, in line order."""
    out = []
    for n, line in enumerate(text.split("\n"), 1):
        if line.endswith("\r"):
            line = line[:-1]
        occurrences = [m.start() for m in _PYTHON.finditer(line)]
        if not occurrences or admitted_line(path, line):
            continue
        if all(admits_occurrence(line, s, tracked) for s in occurrences):
            continue
        out.append(finding("4", path, n, "python outside the admitted occurrences", line, _REMEDY_OCCURRENCE))
    return out


def _unreadable(path, kind):
    remedy = _REMEDY_MISSING if kind == "missing" else _REMEDY_UNREADABLE
    return finding("4", path, None, "unclassifiable (%s)" % kind, None, remedy)


def member_problems(tracked, read):
    """Clauses 2-5 over `tracked`; `read(path)` returns bytes or raises Unclassifiable."""
    paths = sorted(set(tracked))
    out = []
    for path in paths:
        if not is_member(path):
            continue
        red = red_outright(path)
        if red is not None:
            out.append(red)
            continue
        if path == ROOT_PIN:
            continue
        try:
            path.encode("utf-8")
        except UnicodeEncodeError:
            out.append(_unreadable(path, "non-UTF-8"))
            continue
        try:
            data = read(path)
        except Unclassifiable as e:
            out.append(_unreadable(path, e.kind))
            continue
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError:
            out.append(_unreadable(path, "non-UTF-8"))
            continue
        if "\x00" in text:
            out.append(_unreadable(path, "contains NUL"))
            continue
        out.extend(content_findings(path, text, paths))
    return out


def problems(tracked, read):
    """The composition (not a clause): in PR-0, clauses 2-5."""
    return member_problems(tracked, read)


def _errno_kind(e):
    return "unreadable: " + errno.errorcode.get(e.errno, "OSError")


def worktree_reader(root):
    """`read` over a checkout: only a regular file (by lstat) yields bytes."""

    real_root = os.path.realpath(root)

    def read(path):
        full = os.path.join(root, path)
        try:
            st = os.lstat(full)
        except (FileNotFoundError, NotADirectoryError):
            raise Unclassifiable("missing")
        except OSError as e:
            raise Unclassifiable(_errno_kind(e))
        if stat.S_ISLNK(st.st_mode):
            raise Unclassifiable("symlink")
        if os.path.realpath(full) != os.path.join(real_root, path):
            raise Unclassifiable("symlink")  # a symlinked parent component
        if not stat.S_ISREG(st.st_mode):
            raise Unclassifiable("not-a-file")
        try:
            with open(full, "rb") as f:
                return f.read()
        except OSError as e:
            raise Unclassifiable(_errno_kind(e))

    return read
