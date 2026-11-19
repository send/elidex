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


class Controls(unittest.TestCase):
    def setUp(self):
        td = tempfile.TemporaryDirectory()
        self.addCleanup(td.cleanup)
        self.root = os.path.join(td.name, "repo")
        os.makedirs(self.root)
        self.al = os.path.join(td.name, "allow.tsv")
        self.env = mod.git_env()
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True, env=self.env)

    def put(self, name, data):
        p = os.path.join(self.root, name)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.write(data if isinstance(data, bytes) else data.encode("utf-8"))
        subprocess.run(["git", "add", name], cwd=self.root, check=True, env=self.env)

    def allow(self, *rows, header="# header\n"):
        with open(self.al, "w", encoding="utf-8") as f:
            f.write(header + "".join(r + "\n" for r in rows))

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


SRC = os.path.join(HERE, "ecs_mission_ratchet.py")
LINE = "matches Chrome here"


class AddNewHarness:
    """Builds a temp repo and runs add_new from module `m` (the real one, or a mutant copy)."""

    def __init__(self, m, files, rows=()):
        self.m = m
        self.td = tempfile.TemporaryDirectory()
        self.root = os.path.join(self.td.name, "repo")
        os.makedirs(self.root)
        self.env = m.git_env()
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True, env=self.env)
        for name, data in files.items():
            p = os.path.join(self.root, name)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "w", encoding="utf-8") as f:
                f.write(data)
            subprocess.run(["git", "add", name], cwd=self.root, check=True, env=self.env)
        self.al = os.path.join(self.td.name, "allow.tsv")
        with open(self.al, "w", encoding="utf-8") as f:
            f.write("# header\n" + "".join(r + "\n" for r in rows))

    def slurp(self):
        with open(self.al, "rb") as f:
            return f.read()

    def add(self, *spec):
        return self.m.add_new(self.root, self.al, self.m.parse_spec("\n".join(spec) + "\n"))

    def close(self):
        self.td.cleanup()


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


def sc_atomic(m):
    return _rejected(m, {"a.md": LINE + "\n", "b.md": "plain\n"},
                     ["T\t#11-x\ta.md\t1", "T\t#11-y\tb.md\t1"])


def sc_count(m):
    two = LINE + "\n" + LINE + "\n"
    h = AddNewHarness(m, {"a.md": two}, ["N\t-\ta.md\t" + LINE])
    try:
        errs, added = h.add("T\t#11-x\ta.md\t1")
        if errs or len(added) != 1:
            return False
        before = h.slurp()
        errs, added = h.add("T\t#11-x\ta.md\t2")
        if not errs or added or h.slurp() != before:
            return False
    finally:
        h.close()
    # within one batch the rows consume occurrences in turn
    return _rejected(m, {"a.md": two}, ["T\t#11-x\ta.md\t1", "T\t#11-x\ta.md\t2"], ["N\t-\ta.md\t" + LINE])


SCENARIOS = {
    "control": sc_control, "class_s": sc_class_s, "slot_dash": sc_slot_dash, "not_denied": sc_not_denied,
    "covered": sc_covered, "range": sc_range, "self_file": sc_self_file, "atomic": sc_atomic, "count": sc_count,
}
# (mutant name, ((source text, replacement), ...), scenarios that must go red)
MUTANTS = [
    ("a: any class", (('ADD_CLASSES = "TD"', 'ADD_CLASSES = "TDSECN"'),), ["class_s"]),
    ("b: any slot", (("if not SLOT.match(slot):", "if False:"),), ["slot_dash"]),
    ("c: SELF_FILES scanned", (("excluded = SELF_FILES + (", "excluded = ("),), ["self_file"]),
    ("c: no range check", (("if not 1 <= i <= len(lines):", "if i < 1:"),), ["range"]),
    # the explicit denied() test is subsumed by the occurrence count (occ counts denied lines only), so
    # the mutant removes both; removing either alone leaves the other check rejecting the row.
    ("d: denied not required", (("elif not denied(lines[i - 1]):", "elif False:"),
                                ("if denied(l) and normalise(l) == text", "if normalise(l) == text")), ["not_denied"]),
    ("d: coverage ignored", (("if not occ > al[key] + used[key]:", "if not occ > 0:"),), ["covered", "count"]),
    ("d: batch consumption ignored", (("if not occ > al[key] + used[key]:", "if not occ > al[key]:"),), ["count"]),
    ("atomic: writes despite errors", (("    if errs:\n        return errs, []", "    if False:\n        return errs, []"),), ["atomic"]),
]


def mutant_module(old, new):
    pairs = old if isinstance(old, tuple) else ((old, new),)
    with open(SRC, encoding="utf-8") as f:
        src = f.read()
    for o, n in pairs:
        assert src.count(o) == 1, o
        src = src.replace(o, n)
    m = types.ModuleType("ecs_ratchet_mutant")
    m.__file__ = SRC
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
            m = mutant_module(old, None)
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


if __name__ == "__main__":
    unittest.main()
