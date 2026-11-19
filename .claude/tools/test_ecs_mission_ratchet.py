"""Tests for ecs_mission_ratchet.py (runs in the `tools` job). No skips: the hook comparison
degrades to fixtures-only when the hook file is absent (CI)."""
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import types
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


mod = load(os.path.join(HERE, "ecs_mission_ratchet.py"), "ecs_mission_ratchet_under_test")
with open(os.path.join(HERE, "ecs_mission_fixtures.json"), encoding="utf-8") as _f:
    FX = json.load(_f)
HOOK_PATH = os.environ.get("ECS_GUARD_HOOK") or os.path.expanduser("~/.claude/hooks/ecs-mission-guard.py")
REPO = mod.repo_root_of(HERE)
ALLOWLIST = mod.DEFAULT_ALLOWLIST


def tagged_pairs():
    return [(l, t) for l in FX["deny"] for t in FX["tags"]]


def fixture_lines():
    return list(FX["deny"]) + list(FX["pass"]) + [l + " " + t for l, t in tagged_pairs()]


class Fixtures(unittest.TestCase):
    def test_deny(self):
        for l in FX["deny"]:
            with self.subTest(line=l):
                self.assertEqual(mod.line_denied(l), "deny")

    def test_tagged(self):
        for l, t in tagged_pairs():
            with self.subTest(line=l, tag=t):
                self.assertEqual(mod.line_denied(l + " " + t), "tagged")

    def test_pass(self):
        for l in FX["pass"]:
            with self.subTest(line=l):
                self.assertIsNone(mod.line_denied(l))


class HookParity(unittest.TestCase):
    def test_parity(self):
        if not os.path.isfile(HOOK_PATH):
            print("ecs-mission hook parity: hook not present (%s); fixtures only, hook comparison not available" % HOOK_PATH)
            return
        hook = load(HOOK_PATH, "ecs_guard_hook_under_test")
        lines = fixture_lines() + [l for _, text in mod.iter_texts(REPO) for l in text.splitlines()]
        mism = [l for l in lines if hook.line_denied(l) != mod.line_denied(l)]
        print("ecs-mission hook parity: hook present: %d lines compared, %d mismatches" % (len(lines), len(mism)))
        self.assertEqual(mism[:5], [])


class WholeTree(unittest.TestCase):
    def test_tree_matches_allowlist(self):
        self.assertEqual(mod.check(REPO, ALLOWLIST), [])


SRC = os.path.join(HERE, "ecs_mission_ratchet.py")
LINE = "matches Chrome here"


class AddNewHarness:
    """The one temp-repo fixture: builds a repo and runs the checker from module `m` (the real one, or a mutant copy)."""

    def __init__(self, m, files=None, rows=()):
        self.m = m
        self.td = tempfile.TemporaryDirectory()
        self.root = os.path.join(self.td.name, "repo")
        os.makedirs(self.root)
        self.env = m.git_env()
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True, env=self.env)
        for name, data in (files or {}).items():
            self.put(name, data)
        self.base_src("def denied(line):\n    return False\n")  # the default base classifier denies nothing
        self.al = os.path.join(self.td.name, "allow.tsv")
        self.allow(*rows)

    def put(self, name, data):
        p = os.path.join(self.root, name)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.write(data if isinstance(data, bytes) else data.encode("utf-8"))
        subprocess.run(["git", "add", name], cwd=self.root, check=True, env=self.env)

    def base_src(self, src):
        """Commit `src` as the classifier module together with every file put so far; HEAD is the base
        the scenarios pass as --base (a file put afterwards is a line added after the base)."""
        self.put(".claude/tools/ecs_mission_ratchet.py", src)
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "base"],
                       cwd=self.root, check=True, env=self.env)

    def allow(self, *rows, header="# header\n"):
        with open(self.al, "w", encoding="utf-8") as f:
            f.write(header + "".join(r + "\n" for r in rows))

    def slurp(self):
        with open(self.al, "rb") as f:
            return f.read()

    def add(self, *spec, base="HEAD"):
        return self.m.add_new(self.root, self.al, self.m.parse_tsv("\n".join(spec) + "\n"), base)

    def relabel(self, *spec):
        return self.m.relabel(self.root, self.al, self.m.parse_tsv("\n".join(spec) + "\n"))

    def close(self):
        self.td.cleanup()


class Controls(unittest.TestCase):
    def setUp(self):
        self.h = AddNewHarness(mod)
        self.addCleanup(self.h.close)
        self.root, self.al, self.env = self.h.root, self.h.al, self.h.env

    def put(self, name, data):
        self.h.put(name, data)

    def allow(self, *rows, header="# header\n"):
        self.h.al = self.al
        self.h.allow(*rows, header=header)

    def slurp(self):
        with open(self.al, encoding="utf-8") as f:
            return f.read()

    def check(self):
        return mod.check(self.root, self.al)

    def test_new_hit(self):
        self.put("a.md", "ok\nmatches Chrome here\n")
        self.allow()
        errs = self.check()
        self.assertTrue(any(e.startswith("new untagged browser mention: a.md:2: matches Chrome here") for e in errs), errs)
        self.assertTrue(any("not a remedy" in e for e in errs))

    def test_tagged_ok(self):
        self.put("a.md", "matches Chrome here [spec]\nplain\n")
        self.allow()
        self.assertEqual(self.check(), [])

    def test_allowlisted_ok(self):
        self.put("a.md", "matches   Chrome here\n")
        self.allow("N\t-\ta.md\tmatches Chrome here")
        self.assertEqual(self.check(), [])

    def test_stale(self):
        self.put("a.md", "plain\n")
        self.allow("N\t-\ta.md\tmatches Chrome here")
        errs = self.check()
        self.assertEqual(len(errs), 1)
        self.assertIn("stale allowlist row", errs[0])

    def test_multiplicity(self):
        self.put("a.md", "matches Chrome here\nmatches Chrome here\n")
        self.allow("N\t-\ta.md\tmatches Chrome here")
        errs = [e for e in self.check() if e.startswith("new untagged")]
        self.assertEqual(len(errs), 1)
        self.assertIn("a.md:2:", errs[0])

    def test_t_row_needs_slot(self):
        self.put("a.md", "matches Chrome here\n")
        self.allow("T\t-\ta.md\tmatches Chrome here")
        errs = self.check()
        self.assertEqual(len(errs), 1)
        self.assertIn("must carry a slot", errs[0])

    def test_t_row_with_slot_ok(self):
        self.put("a.md", "matches Chrome here\n")
        self.allow("T\t#11-some-slot\ta.md\tmatches Chrome here")
        self.assertEqual(self.check(), [])

    def test_bad_slot_class_columns(self):
        self.put("a.md", "matches Chrome here\n")
        self.allow("N\tslot\ta.md\tmatches Chrome here", "?\t-\ta.md\tmatches Chrome here", "N\ta.md")
        errs = self.check()
        self.assertTrue(any("bad slot shape" in e for e in errs))
        self.assertTrue(any("unknown class" in e for e in errs))
        self.assertTrue(any("need 4 TAB-separated columns" in e for e in errs))

    def test_unsorted(self):
        self.put("a.md", "matches Chrome here\nmatches Blink here\n")
        self.allow("N\t-\ta.md\tmatches draft", "N\t-\ta.md\tmatches Chrome here")
        errs = self.check()
        self.assertTrue(any("sorted" in e for e in errs), errs)
        self.put("b.md", "matches Chrome here\nmatches Blink here\n")
        self.allow("N\t-\ta.md\tmatches Chrome here", "N\t-\ta.md\tmatches Blink here",
                   "N\t-\tb.md\tmatches Blink here", "N\t-\tb.md\tmatches Chrome here")
        self.assertTrue(any("sorted" in e for e in self.check()))
        self.allow("N\t-\ta.md\tmatches Blink here", "N\t-\ta.md\tmatches Chrome here",
                   "N\t-\tb.md\tmatches Blink here", "N\t-\tb.md\tmatches Chrome here")
        self.assertEqual(self.check(), [])

    def test_row_text_not_deny_line(self):
        self.put("a.md", "plain\n")
        self.allow("N\t-\ta.md\tplain text")
        errs = self.check()
        self.assertTrue(any("not a normalised deny line" in e for e in errs), errs)

    def test_prune_drops_only_stale(self):
        self.put("a.md", "matches Chrome here\nmatches Blink here\nmatches Servo here\n")
        self.allow("N\t-\ta.md\tmatches Blink here",
                   "N\t-\ta.md\tmatches Chrome here",
                   "N\t-\ta.md\tmatches Gecko here",
                   "N\t-\tz.md\tgone Chrome line",
                   header="# keep me\n")
        before = self.slurp()
        dropped, _ = mod.prune(self.root, self.al)
        after = self.slurp()
        self.assertEqual(dropped, 2)
        self.assertTrue(after.startswith("# keep me\n"))
        self.assertNotIn("Gecko", after)
        self.assertNotIn("gone", after)
        self.assertNotIn("Servo", after)  # the untracked-by-allowlist hit is not added
        self.assertEqual(after.count("\n"), before.count("\n") - 2)
        errs = self.check()
        self.assertTrue(any("new untagged browser mention: a.md:3: matches Servo here" in e for e in errs))
        self.assertFalse(any("stale" in e for e in errs))

    def test_nul_skipped_non_utf8_scanned(self):
        self.put("bin.dat", b"matches Chrome here\0\n")
        self.put("late_nul.md", b"x" * 8000 + b"\nmatches Chrome here\0\n")
        self.put("nul_7999.md", b"matches Chrome here\n" + b"x" * (7999 - 20) + b"\0")
        self.put("nul_8000.md", b"matches Chrome here\n" + b"x" * (8000 - 20) + b"\0")
        self.put("latin1.md", b"caf\xe9\nmatches Chrome here\n")
        self.allow()
        errs = self.check()
        self.assertTrue(any("latin1.md:2: matches Chrome here" in e for e in errs), errs)
        self.assertFalse(any("bin.dat" in e for e in errs), errs)
        self.assertTrue(any("late_nul.md:2: matches Chrome here" in e for e in errs), errs)
        self.assertFalse(any("nul_7999.md" in e for e in errs), errs)
        self.assertTrue(any("nul_8000.md:1: matches Chrome here" in e for e in errs), errs)

    def test_line_numbers_split_on_newline_only(self):
        self.put("a.md", "x\x0cy\u2028z\r\nmatches Chrome here\r\n")
        self.allow()
        self.assertTrue(any("a.md:2: matches Chrome here" in e for e in self.check()))

    def test_tag_behind_cr_does_not_cover_other_segment(self):
        self.put("a.md", b"matches Chrome here\rnote [spec]\n")
        self.allow()
        self.assertTrue(any("a.md:1: matches Chrome here note [spec]" in e for e in self.check()))

    def test_tracked_symlink_allowlist_excluded_by_own_name(self):
        target = os.path.join(os.path.dirname(self.root), "outside.tsv")  # untracked target
        open(target, "w").close()
        os.symlink(target, os.path.join(self.root, "allow.tsv"))
        subprocess.run(["git", "add", "allow.tsv"], cwd=self.root, check=True, env=self.env)
        self.al = os.path.join(self.root, "allow.tsv")
        self.put("a.md", "matches Chrome here\n")
        self.allow("N\t-\ta.md\tmatches Chrome here")
        self.assertEqual(self.check(), [])

    def test_self_files_not_scanned(self):
        for rel in mod.SELF_FILES:
            self.put(rel, "matches Chrome here\n")
        self.allow()
        self.assertEqual(self.check(), [])

    def test_symlinked_root_still_excludes_allowlist(self):
        link = self.root + "-link"
        os.symlink(self.root, link)
        self.al = os.path.join(link, "allow.tsv")
        self.put("a.md", "matches Chrome here\n")
        self.allow("N\t-\ta.md\tmatches Chrome here")
        subprocess.run(["git", "add", "allow.tsv"], cwd=self.root, check=True, env=self.env)
        self.assertEqual(mod.check(os.path.realpath(self.root), self.al), [])

    def test_inherited_git_dir_ignored(self):
        self.put("a.md", "matches Chrome here\n")
        self.allow()
        other = tempfile.TemporaryDirectory()
        self.addCleanup(other.cleanup)
        subprocess.run(["git", "init", "-q", other.name], check=True, env=self.env)
        with mock.patch.dict(os.environ, {"GIT_DIR": os.path.join(other.name, ".git")}):
            errs = self.check()
        self.assertTrue(any("a.md:1: matches Chrome here" in e for e in errs), errs)

    def test_allowlist_inside_repo_not_scanned(self):
        self.al = os.path.join(self.root, "allow.tsv")
        self.put("a.md", "matches Chrome here\n")
        self.allow("N\t-\ta.md\tmatches Chrome here")
        subprocess.run(["git", "add", "allow.tsv"], cwd=self.root, check=True, env=self.env)
        self.assertEqual(self.check(), [])


def sc_control(m):
    h = AddNewHarness(m, {"a.md": "ok\n" + LINE + "\n"})
    try:
        errs, added = h.add("T\t#11-x\ta.md\t2")
        return (errs == [] and added == ["T\t#11-x\ta.md\t" + LINE]
                and h.slurp() == ("# header\nT\t#11-x\ta.md\t" + LINE + "\n").encode()
                and m.check(h.root, h.al) == [])
    finally:
        h.close()


def _rejected(m, files, spec, rows=()):
    h = AddNewHarness(m, files, rows)
    try:
        before = h.slurp()
        errs, added = h.add(*spec)
        return bool(errs) and added == [] and h.slurp() == before
    finally:
        h.close()


def sc_class_s(m):  # (a)
    return _rejected(m, {"a.md": LINE + "\n"}, ["S\t#11-x\ta.md\t1"])


def sc_slot_dash(m):  # (b)
    return _rejected(m, {"a.md": LINE + "\n"}, ["T\t-\ta.md\t1"])


def sc_not_denied(m):  # (d) denied()
    return _rejected(m, {"a.md": "plain\n" + LINE + "\n"}, ["T\t#11-x\ta.md\t1"])


def sc_covered(m):  # (d) count vs existing row
    return _rejected(m, {"a.md": LINE + "\n"}, ["T\t#11-x\ta.md\t1"], ["N\t-\ta.md\t" + LINE])


def sc_range(m):  # (c) line_no
    return _rejected(m, {"a.md": LINE + "\n"}, ["T\t#11-x\ta.md\t9"])


def sc_self_file(m):  # (c) SELF_FILES
    rel = m.SELF_FILES[0]
    return _rejected(m, {rel: LINE + "\n"}, ["T\t#11-x\t%s\t1" % rel])


def sc_base_denies(m):  # base scope: the base classifier already denies the line
    h = AddNewHarness(m, {"a.md": LINE + "\n"})
    try:
        with open(SRC, encoding="utf-8") as f:
            h.base_src(f.read())
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t1")
        return bool(errs) and "not newly denied" in errs[0] and added == [] and h.slurp() == before
    finally:
        h.close()


def sc_base_raw_line(m):  # the base judges the raw line: \x0c is collapsed by normalise() but splits segments
    h = AddNewHarness(m, {"a.md": "matches Chrome\x0chere\n"})
    try:
        h.base_src("def denied(line):\n    return '\\x0c' in line\n")
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t1")
        return bool(errs) and "not newly denied" in errs[0] and added == [] and h.slurp() == before
    finally:
        h.close()


def sc_base_real_ff(m):  # real classifier at base: `Chrome is fast\x0c[spec]` is denied at base, tagged once normalised
    h = AddNewHarness(m, {"a.md": "Chrome is fast\x0c[spec]\n"})
    try:
        with open(SRC, encoding="utf-8") as f:
            h.base_src(f.read())
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t1")
        return bool(errs) and added == [] and h.slurp() == before
    finally:
        h.close()


def sc_norm_not_denied(m):  # normalised text not denied now: the row would be malformed (base denies nothing)
    h = AddNewHarness(m, {"a.md": "Chrome is fast\x0c[spec]\n"})
    try:
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t1")
        return bool(errs) and "not itself denied" in errs[0] and added == [] and h.slurp() == before
    finally:
        h.close()


def sc_not_at_base(m):  # a line added after the base commit is never allowlisted, even if base does not deny it
    h = AddNewHarness(m, {"a.md": "ok\n"})
    try:
        h.put("a.md", "ok\n" + LINE + "\n")
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t2")
        return bool(errs) and "not present at base" in errs[0] and added == [] and h.slurp() == before
    finally:
        h.close()


def sc_new_file(m):  # a file that is not at base at all
    h = AddNewHarness(m, {"b.md": "ok\n"})
    try:
        h.put("a.md", LINE + "\n")
        errs, added = h.add("T\t#11-x\ta.md\t1")
        return bool(errs) and "not present at base" in errs[0] and added == []
    finally:
        h.close()


def sc_base_count(m):  # count-based: base has one occurrence, the PR added a second; one row exists already
    h = AddNewHarness(m, {"a.md": LINE + "\n"}, ["N\t-\ta.md\t" + LINE])
    try:
        h.put("a.md", LINE + "\n" + LINE + "\n")
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t2")
        return bool(errs) and "not present at base" in errs[0] and added == [] and h.slurp() == before
    finally:
        h.close()


def sc_ln_garbage(m):  # '\u00b2' passes isdigit() but not int(): an error item, no traceback
    return _rejected(m, {"a.md": LINE + "\n"}, ["T\t#11-x\ta.md\t\u00b2"])


def sc_ln_zero(m):
    return _rejected(m, {"a.md": LINE + "\n"}, ["T\t#11-x\ta.md\t0"])


def sc_dup_named(m):  # line 1 named twice must not cover the un-named identical line 2
    return _rejected(m, {"a.md": LINE + "\n" + LINE + "\n"}, ["T\t#11-x\ta.md\t1", "T\t#11-x\ta.md\t1"])


def sc_shape_add(m):  # a 5-column row blocks the write; the file is untouched, rows are not dropped
    h = AddNewHarness(m, {"a.md": LINE + "\n"}, ["N\t-\tb.md\tx\textra"])
    try:
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t1")
        return bool(errs) and added == [] and h.slurp() == before
    finally:
        h.close()


def sc_shape_prune(m):  # a 3-column row beside a stale row: no traceback, file unchanged
    h = AddNewHarness(m, {"a.md": "plain\n"}, ["N\t-\tb.md", "S\t-\ta.md\t" + LINE])
    try:
        before = h.slurp()
        n, diff = h.m.prune(h.root, h.al)
        return n == 0 and h.slurp() == before and bool(h.m.errors(*diff))
    finally:
        h.close()


def sc_shape_relabel(m):
    return _relabel_rejected(m, ["T\t#11-x\ta.md\t1"], [ROW_S, "N\t-\tb.md"])


def sc_base_unreadable(m):  # a base that cannot be read accepts nothing
    h = AddNewHarness(m, {"a.md": LINE + "\n"})
    try:
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t1", base="no-such-rev")
        return bool(errs) and added == [] and h.slurp() == before
    finally:
        h.close()


ROW_S = "S\t-\ta.md\t" + LINE


def _relabel_rejected(m, spec, rows=(ROW_S,), files=None):
    h = AddNewHarness(m, files or {"a.md": LINE + "\n"}, rows)
    try:
        before = h.slurp()
        errs, moved = h.relabel(*spec)
        return bool(errs) and moved == [] and h.slurp() == before
    finally:
        h.close()


def sc_rl_control(m):  # S -> T with a slot: row count unchanged, check passes
    h = AddNewHarness(m, {"a.md": LINE + "\n"}, [ROW_S])
    try:
        errs, moved = h.relabel("T\t#11-x\ta.md\t1")
        return (errs == [] and moved == [ROW_S + " -> T\t#11-x\ta.md\t" + LINE]
                and h.slurp() == ("# header\nT\t#11-x\ta.md\t" + LINE + "\n").encode()
                and m.check(h.root, h.al) == [])
    finally:
        h.close()


def sc_rl_class_s(m):  # relabel moves only toward T / D
    return _relabel_rejected(m, ["S\t#11-x\ta.md\t1"], ["N\t-\ta.md\t" + LINE])


def sc_rl_t_to_d(m):  # a T row is never moved (T <-> D would swap the slot)
    return _relabel_rejected(m, ["D\t#11-y\ta.md\t1"], ["T\t#11-x\ta.md\t" + LINE])


def sc_rl_line0(m):
    return _relabel_rejected(m, ["T\t#11-x\ta.md\t0"], files={"a.md": LINE})  # line 0 must not wrap to the last line


def sc_rl_ln_garbage(m):
    return _relabel_rejected(m, ["T\t#11-x\ta.md\t\u00b2"])


def sc_rl_slot_dash(m):
    return _relabel_rejected(m, ["T\t-\ta.md\t1"])


def sc_rl_slot_empty(m):
    return _relabel_rejected(m, ["T\t\ta.md\t1"])


def sc_rl_no_row(m):  # a row for another text of the same path is not a match
    return _relabel_rejected(m, ["T\t#11-x\ta.md\t1"], ["N\t-\ta.md\tmatches Firefox here"])


def sc_rl_already(m):
    return _relabel_rejected(m, ["T\t#11-x\ta.md\t1"], ["T\t#11-y\ta.md\t" + LINE])


def sc_rl_not_denied(m):
    return _relabel_rejected(m, ["T\t#11-x\ta.md\t1"], ["S\t-\ta.md\tplain"], {"a.md": "plain\n"})


def sc_rl_atomic(m):  # one bad row in the batch: nothing is written
    return _relabel_rejected(m, ["T\t#11-x\ta.md\t1", "T\t#11-x\tb.md\t1"], [ROW_S, "S\t-\tb.md\tplain"],
                             {"a.md": LINE + "\n", "b.md": "plain\n"})


def sc_rl_consumed(m):  # two spec rows for one allowlist row: the second finds none left
    return _relabel_rejected(m, ["T\t#11-x\ta.md\t1", "D\t#11-x\ta.md\t1"])


def sc_atomic(m):
    return _rejected(m, {"a.md": LINE + "\n", "b.md": "plain\n"},
                     ["T\t#11-x\ta.md\t1", "T\t#11-y\tb.md\t1"])


def sc_count(m):  # a batch consumes the new-hit occurrences in turn
    return _rejected(m, {"a.md": LINE + "\n" + LINE + "\n"}, ["T\t#11-x\ta.md\t2", "T\t#11-x\ta.md\t2"],
                     ["N\t-\ta.md\t" + LINE])


SCENARIOS = {
    "control": sc_control, "class_s": sc_class_s, "slot_dash": sc_slot_dash, "not_denied": sc_not_denied,
    "covered": sc_covered, "range": sc_range, "self_file": sc_self_file, "atomic": sc_atomic, "count": sc_count,
    "base_denies": sc_base_denies, "base_unreadable": sc_base_unreadable,
    "base_raw_line": sc_base_raw_line, "base_real_ff": sc_base_real_ff, "norm_not_denied": sc_norm_not_denied,
    "not_at_base": sc_not_at_base, "new_file": sc_new_file, "base_count": sc_base_count,
    "ln_garbage": sc_ln_garbage, "ln_zero": sc_ln_zero, "dup_named": sc_dup_named,
    "shape_add": sc_shape_add, "shape_prune": sc_shape_prune, "shape_relabel": sc_shape_relabel,
    "rl_t_to_d": sc_rl_t_to_d, "rl_line0": sc_rl_line0, "rl_ln_garbage": sc_rl_ln_garbage,
    "rl_control": sc_rl_control, "rl_class_s": sc_rl_class_s, "rl_slot_dash": sc_rl_slot_dash,
    "rl_slot_empty": sc_rl_slot_empty, "rl_no_row": sc_rl_no_row, "rl_already": sc_rl_already,
    "rl_not_denied": sc_rl_not_denied, "rl_atomic": sc_rl_atomic, "rl_consumed": sc_rl_consumed,
}
# (mutant name, ((source text, replacement), ...), scenarios that must go red)
NEWHIT = "(path, ln) not in text_at"
MUTANTS = [
    ("class: any class", (('ADD_CLASSES = "TD"', 'ADD_CLASSES = "TDSECN"'),), ["class_s"]),
    ("slot: any slot", (("if not SLOT.match(slot):", "if False:"),), ["slot_dash"]),
    # the new-hit position check is the only gate for denied / in-range; the lookup that follows needs a
    # default, else the mutant dies by KeyError instead of by the scenario
    ("new-hit position not required", ((NEWHIT, "False"), ("text_at[(path, ln)]", "text_at.get((path, ln), LINE_)")),
     ["not_denied", "range"]),
    ("duplicate named line ignored", (("if (path, ln) in named:", "if False:"),), ["dup_named"]),
    ("atomic: writes despite errors", (("    if errs:\n        return errs, []\n    write_allowlist(allowlist_path, _comments(allowlist_path), [f for _, f in rows] + added)",
                                         "    if False:\n        return errs, []\n    write_allowlist(allowlist_path, _comments(allowlist_path), [f for _, f in rows] + added)"),), ["atomic"]),
    ("relabel class: any class", (("or f[0] not in ADD_CLASSES ", ""),), ["rl_class_s"]),
    ("relabel slot: any slot", (("or not SLOT.match(f[1]) ", ""),), ["rl_slot_dash", "rl_slot_empty"]),
    ("relabel row match: text ignored", (("r[3] == normalise(text) and ", ""),), ["rl_no_row"]),
    ("relabel row match: class ignored", ((" and r[0] in LIGHT", ""),), ["rl_already", "rl_t_to_d"]),
    ("relabel denied-now not required", (("if text is None or not denied(text):", "if text is None:"),), ["rl_not_denied"]),
    ("relabel atomic: writes despite errors", (("    if errs:\n        return errs, []\n    write_allowlist(allowlist_path, _comments(allowlist_path), [r for _, r in rows])",
                                                "    if False:\n        return errs, []\n    write_allowlist(allowlist_path, _comments(allowlist_path), [r for _, r in rows])"),), ["rl_atomic"]),
    ("base scope: base-denied lines accepted", (("if was_denied(_line_of(repo_root, path, ln)):", "if False:"),), ["base_denies", "base_raw_line"]),
    ("base scope: normalised text judged, not the raw line", (("was_denied(_line_of(repo_root, path, ln))", "was_denied(key[1])"),), ["base_raw_line"]),
    ("normalised text need not be denied now", (("if not denied(key[1]):", "if False:"),), ["norm_not_denied"]),
    ("base presence not required", (("if at_base[path] is None or not at_base[path][key[1]] - existing[key] - used[key] > 0:", "if False:"),), ["not_at_base", "new_file", "base_count"]),
    ("base presence: not count-based", (("- existing[key] - used[key] > 0", "> 0"),), ["base_count"]),
    ("line_no parsed by isdigit", (("    try:\n        n = int(s)\n    except ValueError:\n        return None\n    return n if n >= 1 else None",
                                    "    n = int(s) if s.isdigit() else None\n    return n if n is None or n >= 1 else None"),), ["ln_garbage", "rl_ln_garbage"]),
    ("line_no >= 1 not required", (("return n if n >= 1 else None", "return n"), ("    if ln < 1:\n        return None\n", "")), ["rl_line0"]),
    ("base unreadable: accepted as 'denies nothing'", (("was_denied = base_denied(repo_root, base)", "was_denied = lambda t: False"),
                                                      ("at_base[path] = _base_texts(repo_root, base, path)", "at_base[path] = collections.Counter({key[1]: 9})")), ["base_unreadable"]),
]


def mutant_module(pairs):
    with open(SRC, encoding="utf-8") as f:
        src = f.read()
    for o, n in pairs:
        assert src.count(o) == 1, o
        src = src.replace(o, n)
    m = types.ModuleType("ecs_ratchet_mutant")
    m.__file__ = SRC
    m.LINE_ = LINE
    exec(compile(src, SRC, "exec"), m.__dict__)
    return m


def passes(fn, m):
    try:
        return bool(fn(m))
    except Exception:
        return False


class AddNew(unittest.TestCase):
    def test_scenarios_hold(self):
        for name, fn in SCENARIOS.items():
            with self.subTest(scenario=name):
                self.assertTrue(fn(mod))

    def test_mutants_are_caught(self):
        for name, old, must_fail in MUTANTS:
            m = mutant_module(old)
            for sc in must_fail:
                with self.subTest(mutant=name, scenario=sc):
                    self.assertFalse(passes(SCENARIOS[sc], m))

    def test_inserted_in_sorted_position(self):
        h = AddNewHarness(mod, {"a.md": LINE + "\n", "b.md": LINE + "\n"}, ["N\t-\tb.md\t" + LINE])
        self.addCleanup(h.close)
        errs, _ = h.add("D\t#11-x\ta.md\t1")
        self.assertEqual(errs, [])
        self.assertEqual(mod.check(h.root, h.al), [])

    def test_cli_mutually_exclusive_and_usage(self):
        self.assertEqual(mod.main(["--add-new"]), 2)
        self.assertEqual(mod.main(["--add-new", "x", "--prune"]), 2)
        self.assertEqual(mod.main(["--prune", "--add-new", "x"]), 2)
        self.assertEqual(mod.main(["--add-new", "x", "--base"]), 2)
        self.assertEqual(mod.main(["--prune", "--base", "HEAD"]), 2)
        self.assertEqual(mod.main(["--relabel"]), 2)
        self.assertEqual(mod.main(["--relabel", "x", "--prune"]), 2)
        self.assertEqual(mod.main(["--add-new", "x", "--relabel", "y"]), 2)
        self.assertEqual(mod.main(["--relabel", "x", "--base", "HEAD"]), 2)


if __name__ == "__main__":
    unittest.main()
