"""Reader and CI-wiring pins (R1-R9, M1-M2, ...) for scripts/python-suites.sh.

Reads `.github/workflows/ci.yml` and `mise.toml` through the text readers in
`python_suites_readers.py`; fixtures for them are below.
"""

from __future__ import annotations

import unittest

from python_suites_fixtures import REPO
from python_suites_readers import (
    MATRIX_EXPR,
    ReaderError,
    Workflow,
    driver_step_index,
    gate_keys,
    mise_problems,
    q7_problems,
    tools_problems,
    trigger_problems,
)

# Reader fixtures.

ON_OK = "on:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\n"
CHECK_JOB = (
    "  check:\n"
    "    needs: changes\n"
    "    if: needs.changes.outputs.rust == 'true' || github.event_name == 'push'\n"
    "    runs-on: ubuntu-latest\n"
    "    steps:\n"
    "      - run: cargo check\n"
)
TOOLS_HEAD = (
    "  tools:\n"
    "    runs-on: ubuntu-latest\n"
    "    strategy:\n"
    "      fail-fast: false\n"
    "      matrix:\n"
    '        python-version: ["3.9", "3.x"]\n'
)
TOOLS_STEPS = (
    "    steps:\n"
    "      - uses: actions/checkout@v6\n"
    "      - uses: actions/setup-python@v6\n"
    "        with:\n"
    "          python-version: ${{ matrix.python-version }}\n"
    "      - name: Run the Python suites\n"
    "        run: bash scripts/python-suites.sh\n"
)


def workflow(tools=None, on=ON_OK, after=""):
    tools = TOOLS_HEAD + TOOLS_STEPS if tools is None else tools
    return "name: CI\n\n" + on + "\njobs:\n" + CHECK_JOB + "\n" + tools + after


def tools_with(extra_job_lines="", step=None):
    """The good tools job, with extra job-level lines and/or a replaced driver step."""
    steps = TOOLS_STEPS
    if step is not None:
        steps = steps.replace("      - name: Run the Python suites\n        run: bash scripts/python-suites.sh\n", step)
    return TOOLS_HEAD + extra_job_lines + steps


def mise_fixture(ci='depends = ["check", "tools-test"]', tt='run = "bash scripts/python-suites.sh"', extra=""):
    return (
        '[tasks.check]\nrun = "cargo check"\n\n[tasks.ci]\ndescription = "all"\n'
        + ci
        + "\n\n[tasks.tools-test]\ndescription = \"suites\"\n"
        + tt
        + "\n"
        + extra
    )


class TestReaderPins(unittest.TestCase):
    def assertReaderRed(self, fn, text, lineno=None):
        with self.assertRaises(ReaderError) as cm:
            fn(text)
        if lineno is not None:
            self.assertEqual(cm.exception.lineno, lineno, str(cm.exception))
        return cm.exception

    @staticmethod
    def line_of(text, needle):
        for n, l in enumerate(text.splitlines(), 1):
            if needle in l:
                return n
        raise AssertionError(needle)

    def tools_job(self, text):
        job = Workflow(text).job("tools")
        assert job is not None
        return job

    # Live files.

    def test_Q4_live_mise_toml(self):
        self.assertEqual(mise_problems((REPO / "mise.toml").read_text(encoding="utf-8")), [])

    def test_Q6_Q7_Q8_live_ci_yml(self):
        text = (REPO / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        self.assertEqual(tools_problems(text), [])
        wf = Workflow(text)
        # Q8 negative control: the same predicate reads `check` as gated.
        self.assertEqual(gate_keys(wf.job("check")), ["if", "needs"])

    def test_Q6_driver_step_required(self):
        empty = TOOLS_HEAD + "    steps:\n      - uses: actions/checkout@v6\n"
        self.assertIn("no run: step executes the driver", tools_problems(workflow(empty)))
        self.assertEqual(tools_problems(workflow()), [])

    def test_Q7_matrix_consumed_by_setup_python_before_the_driver(self):
        good = workflow()
        self.assertEqual(q7_problems(self.tools_job(good)), [])
        unconsumed = good.replace("          python-version: ${{ matrix.python-version }}\n", "          python-version: '3.12'\n")
        self.assertTrue(q7_problems(self.tools_job(unconsumed)))
        after = TOOLS_HEAD + (
            "    steps:\n"
            "      - run: bash scripts/python-suites.sh\n"
            "      - uses: actions/setup-python@v6\n"
            "        with:\n"
            "          python-version: ${{ matrix.python-version }}\n"
        )
        self.assertTrue(q7_problems(self.tools_job(workflow(after))))
        one_leg = good.replace('["3.9", "3.x"]', '["3.x"]')
        self.assertTrue(q7_problems(self.tools_job(one_leg)))
        include = good.replace('        python-version: ["3.9", "3.x"]\n', '        python-version: ["3.9", "3.x"]\n        include:\n          - os: macos-latest\n')
        self.assertTrue(q7_problems(self.tools_job(include)))

    def test_Q8_present_ungated_and_trigger(self):
        self.assertEqual(gate_keys(self.tools_job(workflow())), [])
        self.assertIn("tools job absent or empty", tools_problems(workflow(tools="")))
        gated_new_output = workflow(tools_with("    needs: changes\n    if: needs.changes.outputs.tools == 'true'\n"))
        self.assertIn("tools job gated by if", tools_problems(gated_new_output))
        self.assertIn("tools job gated by needs", tools_problems(gated_new_output))
        self.assertEqual(gate_keys(Workflow(workflow()).job("check")), ["if", "needs"])
        for on in (
            "on:\n  pull_request:\n    branches: [main]\n    paths: ['crates/**']\n",
            "on:\n  pull_request:\n    branches: [main]\n    paths-ignore: ['docs/**']\n",
            "on:\n  pull_request:\n    branches: [main, dev]\n",
            "on:\n  push:\n    branches: [main]\n",
        ):
            self.assertTrue(trigger_problems(Workflow(workflow(on=on)).on()), on)

    # R-rows.

    def test_R1_keys_recognised_in_any_quoting(self):
        for line in ('    "needs": changes\n', "    'if': x\n"):
            self.assertTrue(gate_keys(self.tools_job(workflow(tools_with(line)))), line)

    def test_R2_modelled_values_read_correctly(self):
        wf = Workflow(workflow())
        self.assertEqual(list(wf.on()["pull_request"]["branches"]), ["main"])
        job = wf.job("tools")
        assert job is not None
        self.assertEqual(list(job["strategy"]["matrix"]["python-version"]), ["3.9", "3.x"])
        self.assertEqual(str(job["steps"][1]["with"]["python-version"]), MATRIX_EXPR)

    def test_R3_value_position_constructs_are_red(self):
        for line in ("    env: {a: 1}\n", "    timeout-minutes: &x 10\n", "    runs-on: *x\n"):
            text = workflow(tools_with(line).replace("    runs-on: ubuntu-latest\n", "", 1 if "runs-on" in line else 0))
            self.assertReaderRed(self.tools_job, text, self.line_of(text, line.strip()))

    def test_R4_key_position_constructs_are_red(self):
        for line in ("    &a needs: x\n", "    !!str needs: x\n", "    ? needs\n"):
            text = workflow(tools_with(line))
            self.assertReaderRed(self.tools_job, text, self.line_of(text, line.strip()))

    def test_R5_merge_keys_are_red(self):
        for line in ("    <<: *x\n", "    <<: {needs: changes}\n"):
            text = workflow(tools_with(line))
            self.assertReaderRed(self.tools_job, text, self.line_of(text, line.strip()))

    def test_R6_multi_line_flow_sequence_is_red(self):
        on = "on:\n  pull_request:\n    branches: [main,\n      dev]\n"
        text = workflow(on=on)
        self.assertReaderRed(lambda t: Workflow(t).on(), text, self.line_of(text, "[main,"))
        legs = workflow(tools_with().replace('        python-version: ["3.9", "3.x"]\n', '        python-version: [\n          "3.9",\n          "3.x"]\n'))
        # Fail closed (first-fix 10): never read as anything, not even both legs.
        self.assertReaderRed(self.tools_job, legs, self.line_of(legs, "python-version: ["))

    def test_R7_comments_are_neither_values_nor_keys(self):
        self.assertEqual(gate_keys(self.tools_job(workflow(tools_with("    needs: changes # x\n")))), ["needs"])
        self.assertEqual(str(self.tools_job(workflow(tools_with("    needs: changes # x\n")))["needs"]), "changes")
        self.assertEqual(gate_keys(self.tools_job(workflow(tools_with("    # needs: changes\n")))), [])
        hdr = workflow(tools_with().replace("  tools:\n", "  tools: # c\n"))
        self.assertEqual(tools_problems(hdr), [])
        trailing = workflow(tools_with().replace("run: bash scripts/python-suites.sh\n", "run: bash scripts/python-suites.sh # c\n"))
        self.assertIsNotNone(driver_step_index(self.tools_job(trailing)))

    def test_R8_block_scalars(self):
        for header in ("|", ">", "|-", ">-", "|+"):
            step = (
                "      - name: Run the Python suites\n"
                "        run: bash scripts/python-suites.sh\n"
                "      - name: listing\n"
                "        run: %s\n"
                "          ls *.py\n"
                "          needs: x\n" % header
            )
            job = self.tools_job(workflow(tools_with(step=step)))
            self.assertEqual(gate_keys(job), [], header)
            self.assertIn("needs: x", str(job["steps"][3]["run"]))
            self.assertEqual(tools_problems(workflow(tools_with(step=step))), [], header)
        bad = workflow(tools_with(step="      - run: |9x\n          ls\n"))
        self.assertReaderRed(self.tools_job, bad, self.line_of(bad, "|9x"))

    def test_R9_region_bounded_by_the_next_job(self):
        bare = TOOLS_HEAD + "    steps:\n      - uses: actions/checkout@v6\n"
        nxt = (
            "  other:\n"
            "    needs: tools\n"
            "    steps:\n"
            "      - uses: actions/setup-python@v6\n"
            "        with:\n"
            "          python-version: ${{ matrix.python-version }}\n"
            "      - run: bash scripts/python-suites.sh\n"
        )
        job = self.tools_job(workflow(bare, after=nxt))
        self.assertEqual(gate_keys(job), [])
        self.assertIsNone(driver_step_index(job))
        self.assertTrue(q7_problems(job))
        commented = workflow(TOOLS_HEAD + "  # a two-space comment inside tools\n" + TOOLS_STEPS)
        self.assertEqual(tools_problems(commented), [])

    # D1.

    def test_D1_driver_is_the_command_and_unmasked(self):
        def credited(step=None, job_lines=""):
            return driver_step_index(self.tools_job(workflow(tools_with(job_lines, step=step)))) is not None

        for run in ("bash scripts/python-suites.sh", "scripts/python-suites.sh", "mise run tools-test"):
            self.assertTrue(credited("      - run: %s\n" % run), run)
        for run in (
            "echo scripts/python-suites.sh",
            "bash scripts/python-suites.sh || true",
            "bash scripts/python-suites.sh; true",
        ):
            self.assertFalse(credited("      - run: %s\n" % run), run)
        self.assertFalse(credited("      - env:\n          DRIVER: scripts/python-suites.sh\n"))
        self.assertFalse(credited("      - run: bash scripts/python-suites.sh\n        continue-on-error: true\n"))
        self.assertFalse(credited(job_lines="    continue-on-error: true\n"))

    # M-rows.

    def test_M1_mise_unmodelled_constructs_are_red(self):
        self.assertEqual(mise_problems(mise_fixture()), [])
        for ci in ("depends = { a = 1 }", 'depends = [\n  "check",\n  "tools-test",\n]'):
            text = mise_fixture(ci=ci)
            self.assertReaderRed(mise_problems, text, self.line_of(text, "depends ="))
        text = mise_fixture(tt="run = bash scripts/python-suites.sh")
        self.assertReaderRed(mise_problems, text, self.line_of(text, "run = bash"))
        self.assertTrue(mise_problems(mise_fixture(tt='run = ["bash scripts/python-suites.sh"]')))
        elsewhere = mise_fixture(ci='depends = ["check"]', extra='\n[tasks.other]\ndepends = ["tools-test"]\n')
        self.assertIn('[tasks.ci].depends does not contain "tools-test"', mise_problems(elsewhere))
        self.assertReaderRed(mise_problems, mise_fixture(extra='\n[tasks.tools-test.env]\nX = "1"\n'))

    def test_M2_tools_test_has_no_sources_or_outputs(self):
        for line in ('sources = ["scripts/**"]', 'outputs = ["x"]'):
            tt = 'run = "bash scripts/python-suites.sh"\n' + line
            self.assertTrue(any(line.split()[0] in p for p in mise_problems(mise_fixture(tt=tt))), line)


if __name__ == "__main__":
    unittest.main()
