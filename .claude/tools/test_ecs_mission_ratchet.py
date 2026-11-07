"""Tests for ecs_mission_ratchet.py (runs in the `tools` job). No skips: the hook comparison
degrades to fixtures-only when the hook file is absent (CI)."""
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest

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
        env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True, env=env)
        self.env = env

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
        self.put("latin1.md", b"caf\xe9\nmatches Chrome here\n")
        self.allow()
        errs = self.check()
        self.assertTrue(any("latin1.md:2: matches Chrome here" in e for e in errs), errs)
        self.assertFalse(any("bin.dat" in e for e in errs), errs)

    def test_line_numbers_split_on_newline_only(self):
        self.put("a.md", "x\x0cy\u2028z\r\nmatches Chrome here\r\n")
        self.allow()
        self.assertTrue(any("a.md:2: matches Chrome here" in e for e in self.check()))

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
        other = tempfile.mkdtemp()
        self.addCleanup(lambda: subprocess.run(["rm", "-rf", other]))
        subprocess.run(["git", "init", "-q", other], check=True, env=self.env)
        saved = os.environ.get("GIT_DIR")
        os.environ["GIT_DIR"] = os.path.join(other, ".git")
        try:
            errs = self.check()
        finally:
            if saved is None:
                del os.environ["GIT_DIR"]
            else:
                os.environ["GIT_DIR"] = saved
        self.assertTrue(any("a.md:1: matches Chrome here" in e for e in errs), errs)

    def test_allowlist_inside_repo_not_scanned(self):
        self.al = os.path.join(self.root, "allow.tsv")
        self.put("a.md", "matches Chrome here\n")
        self.allow("N\t-\ta.md\tmatches Chrome here")
        subprocess.run(["git", "add", "allow.tsv"], cwd=self.root, check=True, env=self.env)
        self.assertEqual(self.check(), [])


if __name__ == "__main__":
    unittest.main()
