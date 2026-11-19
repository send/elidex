#!/usr/bin/env python3
"""Whole-tree ratchet for the ECS-mission browser-mention rule (CI `tools` job).

The classifier below (BROWSER, GENERIC, TAG, TOOL_NAMES, UA_STRIP, is_browser_hit,
line_denied) mirrors ~/.claude/hooks/ecs-mission-guard.py in behaviour (same patterns and branches;
the text need not be identical). The two change together;
test_ecs_mission_ratchet.py compares them when the hook file is present.

The hook only sees Write/Edit tool calls. This ratchet reads every tracked file and
compares the lines the classifier denies against `ecs-mission-allowlist.tsv`
(one row per occurrence). The allowlist only shrinks (PM 433), with one exception (PM 855, 857): rows are appended only through `--add-new`, which itself refuses lines the base classifier already denies; existing rows may move to a heavier class (T/D) only through `--relabel`, which keeps the row count. Both take a class T or D and a slot; the PR body lists the rows and the command. Every other row change is a deletion.
The lexical classifier is a floor, not the population (a paraphrase, or a phrase split across lines, is not caught); the population is covered by the recurring ledger slot `#11-ecs-mission-concept-sweep`.
Not scanned: the allowlist (its rows would each be a hit for themselves) and SELF_FILES (this
module, its test, its fixtures), which hold the classifier's vocabulary and deny examples by
construction (class N); scanning them would make every fixture edit an allowlist addition.
Every other tracked file is scanned unless it is binary by git's own rule (a NUL byte in the first
8000 bytes); bytes that are not UTF-8 are decoded with U+FFFD replacement, so an encoding slip does
not exempt a file. A binary without such a NUL is read as text and can produce a hit. A tracked
path that is not a readable regular file in the working tree (deleted, a dangling or directory
symlink, a gitlink) is skipped: there is no content to classify.
Line unit: rows are keyed and numbered by '\n'-separated lines (as git and editors count), and a
line is a hit when any of its str.splitlines() segments is denied, which is the unit the hook
classifies (so a tag behind a CR, form feed or U+2028 does not cover a mention in another segment).
Edits that narrow what the ratchet catches (adding an allowlist row, moving a row's class to a
lighter one or dropping / misdirecting a T/D row's slot, adding a path to SELF_FILES, narrowing the
classifier) are invisible to it; elidex-review
axes.md Axis 2 sub-check 2d (authority source) grades them.
Slot ids are checked for shape only (`#11-<kebab>`); that each one exists in the defer ledger is
checked by the PM's periodic ledger check, not here (an in-repo registry would duplicate the ledger).

Usage: python3 .claude/tools/ecs_mission_ratchet.py [--prune | --add-new <spec.tsv> [--base <rev>] | --relabel <spec.tsv>]
--base defaults to origin/main.
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
    r"(?<![A-Za-z0-9_])(?:all browsers|every browser|browsers do|major browsers|other engines|production[- ]engine|like browsers|browser behaviou?r|real browsers|browsers? (?:accept|reject)s?)(?![A-Za-z0-9_])",
    re.I)
TAG = re.compile(r"\[(?:spec|evidence|compat|contrast|history)\]", re.I)
TOOL_NAMES = ("mcp__claude-in-chrome", "claude-in-chrome", "Claude-in-Chrome", "ChromeDriver", "chromedriver")
# UA strings are stripped from a line (like TOOL_NAMES) before it is checked; the rest is still checked:
# a quoted value starting with Mozilla/5.0, an unquoted Mozilla/5.0 plus its run of UA tokens, and the
# header name User-Agent plus a following ":" and UA-token run. Anything not recognised stays and is checked.
_UA_TOK = r"(?:[A-Za-z][A-Za-z0-9_.+-]*/[0-9][A-Za-z0-9_.+-]*|\([^()]*\))"
_UA_RUN = r"(?:\s+" + _UA_TOK + r")*"
UA_STRIP = (
    re.compile(r"([\"'`])\s*Mozilla/5\.0.*?\1"),
    re.compile(r"Mozilla/5\.0" + _UA_RUN),
    re.compile(r"User-Agent(?:\s*:\s*" + _UA_TOK + _UA_RUN + r")?", re.I),
)

CLASSES = "SECTDN"
ADD_CLASSES = "TD"  # the only classes --add-new may append
SLOT = re.compile(r"^#11-[a-z0-9]+(?:-[a-z0-9]+)*$")
DEFAULT_ALLOWLIST = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ecs-mission-allowlist.tsv")
SELF_FILES = tuple(".claude/tools/" + n for n in (
    "ecs_mission_ratchet.py", "test_ecs_mission_ratchet.py", "ecs_mission_fixtures.json"))

REMEDY = (
    "Tag the line per CLAUDE.md 至上命題 bullet 3 ([spec] / [evidence] / [compat] / [contrast] / [history]), "
    "or rewrite it without the engine name. Adding an allowlist row is not a remedy: the allowlist only shrinks. "
    "Inside /// and //! doc comments write the tag as a code span (`[evidence]`): rustdoc reads a bare [evidence] "
    "as an intra-doc link and -D warnings fails."
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
        return nxt == "/"  # lowercase: only chrome/<ver>; the boundary rejects a following letter/digit/_
    if low == "blink":
        return w != "blink"
    if low == "webkit":
        if prev == "-" and nxt == "-":
            return False
        if w == "webkit" or w == "Webkit":
            return prev != "-"  # -webkit-* vendor prefix (a following "_" is rejected by the boundary)
        return True
    return True


def line_denied(line):
    """Return 'deny' | 'tagged' | None."""
    clean = line
    for rx in UA_STRIP:
        clean = rx.sub(" ", clean)
    for t in TOOL_NAMES:
        clean = clean.replace(t, " ")
    if not (any(is_browser_hit(m, clean) for m in BROWSER.finditer(clean)) or GENERIC.search(clean)):
        return None
    if TAG.search(clean):
        return "tagged"
    return "deny"


def normalise(line):
    return " ".join(line.split())


def denied(line):
    """A '\n'-line is a hit when any splitlines() segment (the hook's unit) is denied."""
    return any(line_denied(seg) == "deny" for seg in line.splitlines())


_LOCAL_ENV = None


def git_env():
    """os.environ minus git's repository-locating variables (`git rev-parse --local-env-vars`),
    so a hook's GIT_DIR / GIT_INDEX_FILE cannot redirect the ratchet to another repo or index."""
    global _LOCAL_ENV
    if _LOCAL_ENV is None:
        clean = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        _LOCAL_ENV = set(subprocess.run(["git", "rev-parse", "--local-env-vars"], check=True,
                                        capture_output=True, text=True, env=clean).stdout.split())
    return {k: v for k, v in os.environ.items() if k not in _LOCAL_ENV}


def _git(args, cwd):
    return subprocess.run(["git"] + args, cwd=cwd, check=True, capture_output=True, env=git_env()).stdout


def tracked_files(repo_root):
    out = _git(["ls-files", "-z"], repo_root)
    return sorted(p.decode("utf-8", "surrogateescape") for p in out.split(b"\0") if p)


def iter_texts(repo_root, exclude=()):
    """Yield (path, text) for every tracked regular file without a NUL in its first 8000 bytes."""
    for path in tracked_files(repo_root):
        if path in exclude:
            continue
        full = os.path.join(repo_root, path)
        try:
            if not os.path.isfile(full):
                continue
            with open(full, "rb") as f:
                raw = f.read()
        except OSError:
            continue
        if b"\0" in raw[:8000]:
            continue
        text = raw.decode("utf-8", "replace")
        yield path, text


def scan(repo_root, exclude=()):
    hits = []
    for path, text in iter_texts(repo_root, exclude):
        for i, line in enumerate(text.split("\n"), 1):
            if denied(line):
                hits.append((path, i, normalise(line)))
    return hits


def _rel(repo_root, p):
    """Repo-relative path of p; the directory is resolved (symlinked checkout), the file name is not
    (a tracked symlink is listed by git under its own name)."""
    full = os.path.join(os.path.realpath(os.path.dirname(os.path.abspath(p))), os.path.basename(p))
    return os.path.relpath(full, os.path.realpath(repo_root)).replace(os.sep, "/")


def parse_tsv(text):
    """-> list of (lineno, fields) for every non-comment non-blank line (1-based, '\n'-numbered)."""
    return [(i, raw.split("\t")) for i, raw in enumerate(text.split("\n"), 1)
            if not raw.startswith("#") and raw.strip()]


def read_allowlist(path):
    try:
        with open(path, encoding="utf-8") as f:
            return parse_tsv(f.read())
    except FileNotFoundError:
        return []


def write_allowlist(path, comment_lines, rows):
    """The one allowlist writer: comment lines, then rows (field lists) stably sorted by (path, text)."""
    body = ["\t".join(r) for r in sorted(rows, key=lambda r: (r[2], r[3]))]
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(list(comment_lines) + body) + "\n")


def _comments(path):
    with open(path, encoding="utf-8") as f:
        return [l for l in f.read().split("\n") if l.startswith("#")]


def malformed(rows):
    errs = []
    prev = None
    for n, f in rows:
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
        elif text != normalise(text) or not denied(text):
            errs.append("allowlist line %d: text is not a normalised deny line under the classifier" % n)
        key = (path, text)
        if prev is not None and key < prev:
            errs.append("allowlist line %d: rows must be sorted by (path, text)" % n)
        prev = key
    return errs


def _diff(repo_root, allowlist_path):
    hits = scan(repo_root, exclude=SELF_FILES + (_rel(repo_root, allowlist_path),))
    rows = read_allowlist(allowlist_path)
    tree = collections.defaultdict(list)
    for p, ln, t in hits:
        tree[(p, t)].append(ln)
    al = collections.defaultdict(list)
    for n, f in rows:
        if len(f) == 4:
            al[(f[2], f[3])].append(n)
    new = []
    stale = []
    for key in sorted(set(tree) | set(al)):
        t, a = tree.get(key, []), al.get(key, [])
        if len(t) > len(a):
            new.extend((key[0], ln, key[1]) for ln in t[len(a):])
        elif len(a) > len(t):
            stale.extend((n, key) for n in a[len(t):])
    return rows, new, stale


def check(repo_root, allowlist_path):
    return errors(*_diff(repo_root, allowlist_path))


def errors(rows, new, stale):
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
    """Rewrite the allowlist dropping only stale rows. Never adds. -> (dropped, post-prune diff)."""
    rows, new, stale = _diff(repo_root, allowlist_path)
    drop = {n for n, _ in stale}
    if not drop:
        return 0, (rows, new, stale)
    write_allowlist(allowlist_path, _comments(allowlist_path), [f for n, f in rows if n not in drop])
    return len(drop), (read_allowlist(allowlist_path), new, [])


def base_denied(repo_root, rev):
    """`denied` of the classifier committed at `rev`; raises if unreadable (an unknown base accepts nothing)."""
    ns = {"__name__": "ecs_ratchet_base", "__file__": rev}
    exec(compile(_git(["show", rev + ":.claude/tools/ecs_mission_ratchet.py"], repo_root).decode("utf-8"), rev, "exec"), ns)
    return ns["denied"]


def add_new(repo_root, allowlist_path, spec_rows, base="origin/main"):
    """Append rows for lines that are new hits now and that the classifier at `base` did not deny.
    All-or-nothing. -> (errs, added rows).
    Each spec row is class, slot, path, line_no (1-based, '\\n'-numbered). "New hit" is _diff's `new`."""
    try:
        was_denied = base_denied(repo_root, base)
    except Exception as e:
        return ["base %s unreadable (%s); nothing accepted" % (base, type(e).__name__)], []
    rows, new, _ = _diff(repo_root, allowlist_path)
    avail = collections.Counter((p, t) for p, _, t in new)
    text_at = {(p, ln): t for p, ln, t in new}
    used = collections.Counter()
    errs = []
    added = []
    for n, f in spec_rows:
        where = "spec line %d" % n
        if len(f) != 4:
            errs.append("%s: need 4 TAB-separated columns (class, slot, path, line_no), got %d" % (where, len(f)))
            continue
        cls, slot, path, ln = f
        if cls not in ADD_CLASSES:
            errs.append("%s: class %r rejected: only T or D rows can be added; S/E/C/N are fixed by tag or rewrite, not rows" % (where, cls))
            continue
        if not SLOT.match(slot):
            errs.append("%s: bad slot %r: T/D rows need a '#11-<kebab>' slot" % (where, slot))
            continue
        if not ln.isdigit() or (path, int(ln)) not in text_at:
            errs.append("%s: %s:%s is not a new hit (not denied, not scanned, out of range, or already covered)" % (where, path, ln))
            continue
        key = (path, text_at[(path, int(ln))])
        if was_denied(key[1]):
            errs.append("%s: not newly denied by this classifier change (base %s already denies it, or it is not denied now)" % (where, base))
            continue
        if not avail[key] > used[key]:
            errs.append("%s: %s:%s is named twice" % (where, path, ln))
            continue
        used[key] += 1
        added.append([cls, slot, path, key[1]])
    if errs:
        return errs, []
    write_allowlist(allowlist_path, _comments(allowlist_path), [f for _, f in rows if len(f) == 4] + added)
    return [], ["\t".join(r) for r in sorted(added, key=lambda r: (r[2], r[3]))]


def relabel(repo_root, allowlist_path, spec_rows):
    """Move existing rows to class T or D with a slot; never changes the row count. All-or-nothing.
    Spec rows: class, slot, path, line_no. -> (errs, ["old -> new", ...])."""
    rows, _, _ = _diff(repo_root, allowlist_path)
    rows = [(n, list(f)) for n, f in rows if len(f) == 4]
    done, errs, moved = set(), [], []
    for n, f in spec_rows:
        where = "spec line %d" % n
        if len(f) != 4 or f[0] not in ADD_CLASSES or not SLOT.match(f[1]) or not f[3].isdigit():
            errs.append("%s: need class T or D, a '#11-<kebab>' slot, path, line_no" % where)
            continue
        cls, slot, path, ln = f
        text = _line_of(repo_root, path, int(ln))
        if text is None or not denied(text):
            errs.append("%s: %s:%s is not a denied line now" % (where, path, ln))
            continue
        hit = next((i for i, (_, r) in enumerate(rows) if i not in done and r[2] == path and r[3] == normalise(text) and r[0] != cls), None)
        if hit is None:
            errs.append("%s: no allowlist row for %s:%s that is not already class %s" % (where, path, ln, cls))
            continue
        done.add(hit)
        old = "\t".join(rows[hit][1])
        rows[hit][1][:2] = [cls, slot]
        moved.append("%s -> %s" % (old, "\t".join(rows[hit][1])))
    if errs:
        return errs, []
    write_allowlist(allowlist_path, _comments(allowlist_path), [r for _, r in rows])
    return [], moved


def _line_of(repo_root, path, ln):
    try:
        with open(os.path.join(repo_root, path), encoding="utf-8", errors="replace") as f:
            return f.read().split("\n")[ln - 1]
    except (OSError, IndexError):
        return None


def repo_root_of(path):
    return _git(["rev-parse", "--show-toplevel"], path).decode("utf-8", "surrogateescape").strip()


def main(argv):
    mode = argv[0] if argv else None
    base = "origin/main"
    if len(argv) == 4 and mode == "--add-new" and argv[2] == "--base":
        base, argv = argv[3], argv[:2]
    if not (argv == [] or argv == ["--prune"] or (len(argv) == 2 and mode in ("--add-new", "--relabel"))):
        print("usage: ecs_mission_ratchet.py [--prune | --add-new <spec.tsv> [--base <rev>] | --relabel <spec.tsv>]", file=sys.stderr)
        return 2
    root = repo_root_of(os.path.dirname(os.path.abspath(__file__)))
    if mode in ("--add-new", "--relabel"):
        with open(argv[1], encoding="utf-8") as f:
            spec = parse_tsv(f.read())
        errs, done = add_new(root, DEFAULT_ALLOWLIST, spec, base) if mode == "--add-new" else relabel(root, DEFAULT_ALLOWLIST, spec)
        if errs:
            print("\n".join(errs))
            print("ecs-mission-ratchet: %s rejected; nothing written" % mode)
            return 1
        for r in done:
            print(("added: " if mode == "--add-new" else "relabelled: ") + r)
        diff = _diff(root, DEFAULT_ALLOWLIST)
    elif mode == "--prune":
        n, diff = prune(root, DEFAULT_ALLOWLIST)
        print("ecs-mission-ratchet: pruned %d stale row(s)" % n)
    else:
        diff = _diff(root, DEFAULT_ALLOWLIST)
    errs = errors(*diff)
    if errs:
        print("\n".join(errs))
        return 1
    n = len(diff[0])
    print("ecs-mission-ratchet: ok (%d allowlisted, 0 new, 0 stale)" % n)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
