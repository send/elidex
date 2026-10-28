"""Wrapper that enters plan-memo-umbrella-check.py's self-test into the `tools` job.

Contract: THE WRAPPER CONTRACT in scripts/python-suites.sh.
  - names the command's file by path, as ONE string constant equal to its path
    relative to this file's directory (CHECKER, step 3(b));
  - runs it through sys.executable, no -I / -E / -S, with the process
    environment inherited unchanged (a copy of os.environ that sets only the
    non-regime key PYTHONDONTWRITEBYTECODE), so every regime key is untouched;
  - asserts exit 0 AND a nonzero control count AND a nonzero mutant count, as
    the self-test's own summary lines report them -- defence in depth: the
    self-test has its own emptiness guard
    (`plan_memo_selftest_controls.empty_registry_fails` makes an empty run
    exit 1), and this wrapper does not rely on it;
  - decodes the captured output as UTF-8 explicitly (`encoding="utf-8"`): the
    checker writes UTF-8 whatever the locale, so a locale-derived decode
    (under `LC_ALL=C PYTHONUTF8=0`, US-ASCII) dies on its first non-ASCII byte;
  - stays offline (the checker reads only local files and builds tempdirs).
`--mutants` runs the mutation proof as well as the controls.
"""

import os
import re
import subprocess
import sys
import unittest

CHECKER = "plan-memo-umbrella-check.py"
HERE = os.path.dirname(os.path.abspath(__file__))


def _tail(text, n=40):
    return "\n".join(text.splitlines()[-n:])


class PlanMemoUmbrellaCheckSelfTest(unittest.TestCase):
    def test_self_test_with_mutants(self):
        env = dict(os.environ)
        env["PYTHONDONTWRITEBYTECODE"] = "1"
        proc = subprocess.run(
            [sys.executable, os.path.join(HERE, CHECKER), "--self-test", "--mutants"],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            encoding="utf-8",
            env=env,
        )
        out = proc.stdout
        detail = "\n--- tail of output ---\n" + _tail(out)
        self.assertEqual(proc.returncode, 0, "self-test exited %d%s" % (proc.returncode, detail))
        controls = re.search(r"^(\d+) control\(s\):", out, re.M)
        mutants = re.search(r"^(\d+) mutant\(s\), (\d+) survived, (\d+) crashed\.", out, re.M)
        self.assertIsNotNone(controls, "no control summary line" + detail)
        self.assertIsNotNone(mutants, "no mutant summary line" + detail)
        self.assertGreater(int(controls.group(1)), 0, "zero controls ran" + detail)
        self.assertGreater(int(mutants.group(1)), 0, "zero mutants ran" + detail)
        self.assertEqual(
            (mutants.group(2), mutants.group(3)), ("0", "0"), "mutant survived/crashed" + detail
        )


if __name__ == "__main__":
    unittest.main()
