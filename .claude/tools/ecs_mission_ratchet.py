#!/usr/bin/env python3
"""Whole-tree ratchet for the ECS-mission browser-mention rule (CI `tools` job).

The classifier below (BROWSER, GENERIC, TAG, TOOL_NAMES, UA_MARKERS, is_browser_hit,
line_denied) mirrors ~/.claude/hooks/ecs-mission-guard.py. The two change together;
test_ecs_mission_ratchet.py compares them when the hook file is present.

The hook only sees Write/Edit tool calls. This ratchet reads every tracked file and
compares the lines the classifier denies against `ecs-mission-allowlist.tsv`
(one row per occurrence). The allowlist only shrinks: there is no command that adds a row.
The allowlist file itself is not scanned (its rows would each be a hit for themselves).

Usage: python3 .claude/tools/ecs_mission_ratchet.py [--prune]
"""
import collections
import os
import re
import subprocess
import sys

BROWSER = re.compile(
    r"(?<![A-Za-z0-9_])(Chromium|Chrome|Blink|V8|Firefox|Gecko|SpiderMonkey|Safari|WebKit|JSC|JavaScriptCore|Servo|Ladybird)(?![A-Za-z0-9_])",
    re.I)
GENERIC = re.compile(
    r"(?<![A-Za-z0-9_])(?:all browsers|every browser|browsers do|major browsers|other engines|production[- ]engine|like browsers)(?![A-Za-z0-9_])",
    re.I)
TAG = re.compile(r"\[(?:spec|evidence|compat|contrast|history)\]", re.I)
TOOL_NAMES = ("mcp__claude-in-chrome", "claude-in-chrome", "Claude-in-Chrome", "ChromeDriver", "chromedriver")
UA_MARKERS = ("Mozilla/5.0", "User-Agent")

CLASSES = "SECTDN"
SLOT = re.compile(r"^#11-[a-z0-9]+(?:-[a-z0-9]+)*$")
DEFAULT_ALLOWLIST = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ecs-mission-allowlist.tsv")

REMEDY = (
    "Tag the line per CLAUDE.md 至上命題 bullet 3 ([spec] / [evidence] / [compat] / [contrast] / [history]), "
    "or rewrite it without the engine name. Adding an allowlist row is not a remedy: the allowlist only shrinks."
)


def is_browser_hit(m, line):
    """Apply the N-class exceptions to one BROWSER match."""
    w = m.group(0)
    s, e = m.span()
    low = w.lower()
    nxt = line[e:e + 1]
    prev = line[s - 1:s] if s else ""
    if low == "chrome":
        if w in ("Chrome", "CHROME"):
            return True
        return bool(re.match(r"[/\d]|chromium", line[e:], re.I))
    if low == "blink":
        return w != "blink"
    if low == "webkit":
        if prev == "-" and nxt == "-":
            return False
        if w == "webkit" or w == "Webkit":
            return False if (prev == "-" or nxt == "_") else True
        return True
    return True


def line_denied(line):
    """Return 'deny' | 'tagged' | None."""
    if any(u in line for u in UA_MARKERS):
        return None
    clean = line
    for t in TOOL_NAMES:
        clean = clean.replace(t, " ")
    if not (any(is_browser_hit(m, clean) for m in BROWSER.finditer(clean)) or GENERIC.search(clean)):
        return None
    if TAG.search(clean):
        return "tagged"
    return "deny"


def normalise(line):
    return " ".join(line.split())


def tracked_files(repo_root):
    out = subprocess.run(["git", "ls-files", "-z"], cwd=repo_root, check=True,
                         capture_output=True).stdout
    return sorted(p.decode("utf-8", "surrogateescape") for p in out.split(b"\0") if p)


def iter_texts(repo_root, exclude=()):
    """Yield (path, text) for every tracked file that reads as UTF-8 without NUL."""
    for path in tracked_files(repo_root):
        if path in exclude:
            continue
        full = os.path.join(repo_root, path)
        try:
            if not os.path.isfile(full):
                continue
            with open(full, "rb") as f:
                raw = f.read()
            if b"\0" in raw:
                continue
            text = raw.decode("utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        yield path, text


def scan(repo_root, exclude=()):
    hits = []
    for path, text in iter_texts(repo_root, exclude):
        for i, line in enumerate(text.splitlines(), 1):
            if line_denied(line) == "deny":
                hits.append((path, i, normalise(line)))
    return hits


def _rel(repo_root, p):
    return os.path.relpath(os.path.abspath(p), os.path.abspath(repo_root)).replace(os.sep, "/")


def read_allowlist(path):
    """-> list of (tsv_lineno, raw_line, fields) for every non-comment non-blank line."""
    rows = []
    try:
        with open(path, encoding="utf-8") as f:
            lines = f.read().split("\n")
    except FileNotFoundError:
        return rows
    for i, raw in enumerate(lines, 1):
        if raw.startswith("#") or not raw.strip():
            continue
        rows.append((i, raw, raw.split("\t")))
    return rows


def malformed(rows):
    errs = []
    prev = None
    for n, _raw, f in rows:
        if len(f) != 4:
            errs.append("allowlist line %d: malformed row (need 4 TAB-separated columns, got %d)" % (n, len(f)))
            continue
        cls, slot, path, text = f
        if cls not in list(CLASSES):
            errs.append("allowlist line %d: unknown class %r (expected one of %s)" % (n, cls, ",".join(CLASSES)))
        if cls in ("T", "D") and slot == "-":
            errs.append("allowlist line %d: class %s row must carry a slot" % (n, cls))
        elif slot != "-" and not SLOT.match(slot):
            errs.append("allowlist line %d: bad slot shape %r (expected '-' or '#11-<kebab>')" % (n, slot))
        if not path:
            errs.append("allowlist line %d: empty path" % n)
        if not text:
            errs.append("allowlist line %d: empty text" % n)
        elif text != normalise(text) or line_denied(text) != "deny":
            errs.append("allowlist line %d: text is not a normalised deny line under the classifier" % n)
        key = (path, text)
        if prev is not None and key < prev:
            errs.append("allowlist line %d: rows must be sorted by (path, text)" % n)
        prev = key
    return errs


def _diff(repo_root, allowlist_path):
    ap = _rel(repo_root, allowlist_path)
    hits = scan(repo_root, exclude=(ap,))
    rows = read_allowlist(allowlist_path)
    tree = collections.defaultdict(list)
    for p, ln, t in hits:
        tree[(p, t)].append(ln)
    al = collections.defaultdict(list)
    for n, _raw, f in rows:
        if len(f) == 4:
            al[(f[2], f[3])].append(n)
    new = []
    stale = []
    for key in sorted(set(tree) | set(al)):
        t, a = tree.get(key, []), al.get(key, [])
        if len(t) > len(a):
            for ln in t[len(a):]:
                new.append((key[0], ln, key[1]))
        elif len(a) > len(t):
            stale.extend((n, key) for n in a[len(t):])
    return hits, rows, new, stale


def check(repo_root, allowlist_path):
    _, rows, new, stale = _diff(repo_root, allowlist_path)
    errs = malformed(rows)
    for p, ln, t in new:
        errs.append("new untagged browser mention: %s:%d: %s" % (p, ln, t))
    if new:
        errs.append(REMEDY)
    for n, (p, t) in sorted(stale):
        errs.append("stale allowlist row (line %d, the mention was fixed, moved or tagged; delete the row, "
                    "or run with --prune): %s: %s" % (n, p, t))
    return errs


def prune(repo_root, allowlist_path):
    """Rewrite the allowlist dropping only stale rows. Never adds."""
    _, _, _, stale = _diff(repo_root, allowlist_path)
    drop = {n for n, _ in stale}
    if not drop:
        return 0
    with open(allowlist_path, encoding="utf-8") as f:
        lines = f.read().split("\n")
    kept = [l for i, l in enumerate(lines, 1) if i not in drop]
    with open(allowlist_path, "w", encoding="utf-8") as f:
        f.write("\n".join(kept))
    return len(drop)


def repo_root_of(path):
    return subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=path, check=True,
                          capture_output=True, text=True).stdout.strip()


def main(argv):
    if argv not in ([], ["--prune"]):
        print("usage: ecs_mission_ratchet.py [--prune]", file=sys.stderr)
        return 2
    do_prune = argv == ["--prune"]
    root = repo_root_of(os.path.dirname(os.path.abspath(__file__)))
    if do_prune:
        n = prune(root, DEFAULT_ALLOWLIST)
        print("ecs-mission-ratchet: pruned %d stale row(s)" % n)
    errs = check(root, DEFAULT_ALLOWLIST)
    if errs:
        print("\n".join(errs))
        return 1
    n = len(read_allowlist(DEFAULT_ALLOWLIST))
    print("ecs-mission-ratchet: ok (%d allowlisted, 0 new, 0 stale)" % n)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
