"""Reader and CI-wiring pins (R1-R9, M1-M3, ...) for scripts/python-suites.sh.

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
    job_key_problems,
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
    "    runs-on: ubuntu-24.04\n"
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


def workflow(tools=None, on=ON_OK, after="", top=""):
    tools = TOOLS_HEAD + TOOLS_STEPS if tools is None else tools
    return "name: CI\n\n" + on + top + "\njobs:\n" + CHECK_JOB + "\n" + tools + after


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

    def q7(self, text):
        job = self.tools_job(text)
        return q7_problems(job, driver_step_index(job))

    # Live files.

    def test_Q4_live_mise_toml(self):
        self.assertEqual(mise_problems((REPO / "mise.toml").read_text(encoding="utf-8")), [])
        # Red leg: a wrong `run` string is refused, with the reader's own text.
        self.assertEqual(
            mise_problems(mise_fixture(tt='run = "true"')),
            ["[tasks.tools-test].run is not the string 'bash scripts/python-suites.sh'"],
        )

    def test_Q6_Q7_Q8_live_ci_yml(self):
        text = (REPO / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        self.assertEqual(tools_problems(text), [])

    def test_Q6_driver_step_required(self):
        empty = TOOLS_HEAD + "    steps:\n      - uses: actions/checkout@v6\n"
        self.assertIn("no run: step executes the driver", tools_problems(workflow(empty)))
        self.assertEqual(tools_problems(workflow()), [])

    def test_Q7_matrix_consumed_by_setup_python_before_the_driver(self):
        good = workflow()
        self.assertEqual(self.q7(good), [])
        unconsumed = good.replace("          python-version: ${{ matrix.python-version }}\n", "          python-version: '3.12'\n")
        consume = ["the last setup-python before the driver does not consume ${{ matrix.python-version }}"]
        self.assertEqual(self.q7(unconsumed), consume)
        after = TOOLS_HEAD + (
            "    steps:\n"
            "      - run: bash scripts/python-suites.sh\n"
            "      - uses: actions/setup-python@v6\n"
            "        with:\n"
            "          python-version: ${{ matrix.python-version }}\n"
        )
        self.assertEqual(self.q7(workflow(after)), ["no actions/setup-python step before the driver step"])
        one_leg = good.replace('["3.9", "3.x"]', '["3.x"]')
        self.assertEqual(self.q7(one_leg), ['python-version legs are not exactly quoted "3.9" and "3.x"'])
        floating = good.replace("ubuntu-24.04", "ubuntu-latest")
        self.assertEqual(self.q7(floating), ["runs-on is not exactly ubuntu-24.04"])
        include = good.replace('        python-version: ["3.9", "3.x"]\n', '        python-version: ["3.9", "3.x"]\n        include:\n          - os: macos-latest\n')
        self.assertEqual(self.q7(include), ["matrix keys ['include', 'python-version'] are not exactly python-version"])

    def test_Q8_present_ungated_and_trigger(self):
        self.assertEqual(job_key_problems(self.tools_job(workflow())), [])
        # Negative control on the fixture copy of `check`'s gating: the allowlist refuses it.
        refused = " ".join(job_key_problems(Workflow(workflow()).job("check")))
        self.assertIn("'needs'", refused)
        self.assertIn("'if'", refused)
        self.assertIn("tools job absent or empty", tools_problems(workflow(tools="")))
        gated_new_output = workflow(tools_with("    needs: changes\n    if: needs.changes.outputs.tools == 'true'\n"))
        problems = tools_problems(gated_new_output)
        self.assertIn("job key 'if' is not in the tools-job allowlist", problems)
        self.assertIn("job key 'needs' is not in the tools-job allowlist", problems)
        # Both triggers are read: green when each is exactly `branches: [main]`.
        self.assertEqual(trigger_problems(Workflow(workflow()).on()), [])
        ok = "    branches: [main]\n"
        bad_keys = ("    branches: [main]\n    paths: ['crates/**']\n", "    branches: [main]\n    paths-ignore: ['docs/**']\n")
        bad_branches = ("    branches: [main, dev]\n", "    branches: [dev]\n")
        for trig, other in (("pull_request", "push"), ("push", "pull_request")):
            cases = [(bad, "%s keys are not exactly branches" % trig) for bad in bad_keys]
            cases += [(bad, "%s branches is not exactly [main]" % trig) for bad in bad_branches]
            for bad, want in cases:
                on = "on:\n  %s:\n%s  %s:\n%s" % (trig, bad, other, ok)
                self.assertEqual(trigger_problems(Workflow(workflow(on=on)).on()), [want], on)
            lone = "on:\n  %s:\n%s" % (trig, ok)
            self.assertEqual(trigger_problems(Workflow(workflow(on=lone)).on()), ["on: has no %s mapping" % other], lone)

    # R-rows.

    def test_R1_keys_recognised_in_any_quoting(self):
        for line, key in (('    "needs": changes\n', "needs"), ("    'if': x\n", "if")):
            self.assertEqual(
                job_key_problems(self.tools_job(workflow(tools_with(line)))),
                ["job key %r is not in the tools-job allowlist" % key],
                line,
            )

    def test_R1_keys_differing_only_in_quoting_are_duplicates(self):
        dup_job = workflow(after='\n  "tools":\n    runs-on: ubuntu-24.04\n')
        self.assertReaderRed(Workflow, dup_job, self.line_of(dup_job, '"tools":'))
        dup_on = workflow(on=ON_OK + '  "pull_request":\n    branches: [main]\n')
        self.assertReaderRed(lambda t: Workflow(t).on(), dup_on, self.line_of(dup_on, '"pull_request":'))
        dup_key = workflow(tools_with("    timeout-minutes: 5\n    \"timeout-minutes\": 6\n"))
        self.assertReaderRed(self.tools_job, dup_key, self.line_of(dup_key, '"timeout-minutes":'))

    def test_R2_modelled_values_read_correctly(self):
        self.assertEqual(list(Workflow(workflow()).on()["pull_request"]["branches"]), ["main"])
        job = self.tools_job(workflow())
        self.assertEqual(list(job["strategy"]["matrix"]["python-version"]), ["3.9", "3.x"])
        self.assertEqual(str(job["steps"][1]["with"]["python-version"]), MATRIX_EXPR)

    def test_R3_value_position_constructs_are_red(self):
        for line in ("    env: {a: 1}\n", "    timeout-minutes: &x 10\n", "    runs-on: *x\n"):
            text = workflow(tools_with(line).replace("    runs-on: ubuntu-24.04\n", "", 1 if "runs-on" in line else 0))
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
        trailing_comment = self.tools_job(workflow(tools_with("    needs: changes # x\n")))
        self.assertEqual(job_key_problems(trailing_comment), ["job key 'needs' is not in the tools-job allowlist"])
        self.assertEqual(str(trailing_comment["needs"]), "changes")
        self.assertEqual(job_key_problems(self.tools_job(workflow(tools_with("    # needs: changes\n")))), [])
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
            self.assertEqual(job_key_problems(job), [], header)
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
        self.assertEqual(job_key_problems(job), [])
        self.assertIsNone(driver_step_index(job))
        self.assertTrue(q7_problems(job, driver_step_index(job)))
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
        # The driver step's keys are an allowlist too: any key outside it refuses the credit.
        drv = "      - run: bash scripts/python-suites.sh\n"
        for key in (
            "env:\n          PYTHONPATH: x",
            "env:\n          BASH_ENV: x",
            "shell: sh",
            "if: always()",
            "working-directory: x",
            "some-future-key: x",
        ):
            self.assertFalse(credited(drv + "        %s\n" % key), key)
        for key in ("name: x", "id: x", "timeout-minutes: 5"):
            self.assertTrue(credited(drv + "        %s\n" % key), key)
        # The job's keys are an allowlist: any key outside it refuses the credit, named or not.
        for lines in (
            "    defaults:\n      run:\n        shell: sh\n",
            "    env:\n      PYTHONPATH: x\n",
            "    needs: changes\n",
            "    if: always()\n",
            "    some-future-key: x\n",
        ):
            self.assertFalse(credited(job_lines=lines), lines)
            self.assertTrue(job_key_problems(self.tools_job(workflow(tools_with(lines)))), lines)

    def test_D1_workflow_keys_and_env_are_allowlists(self):
        # The real ci.yml is green (its workflow-level env is CARGO_*).
        live = (REPO / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        self.assertEqual(tools_problems(live), [])
        self.assertTrue(Workflow(live).env())
        for key in ("BASH_ENV", "ENV", "SHELLOPTS", "BASHOPTS", "PATH", "PYTHONPATH", "PYTHONSTARTUP", "HTTP_PROXY", "http_proxy"):
            text = workflow(top="\nenv:\n  CARGO_TERM_COLOR: always\n  %s: x\n" % key)
            self.assertIn("workflow env key %r is not in the allowlist" % key, tools_problems(text), key)
        # Allowed keys alone are green; a workflow key outside the allowlist is red.
        self.assertEqual(tools_problems(workflow(top="\nenv:\n  CARGO_TERM_COLOR: always\n")), [])
        for top in ("\ndefaults:\n  run:\n    shell: sh\n", "\nsome-future-key: x\n"):
            self.assertReaderRed(tools_problems, workflow(top=top))

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
        self.assertReaderRed(mise_problems, mise_fixture(extra='\n[tasks."tools-test".env]\nX = "1"\n'))
        esc = mise_fixture(extra='\n[tasks."tools\\u002dtest".env]\nX = "1"\n')
        self.assertReaderRed(mise_problems, esc, self.line_of(esc, "u002d"))
        junk = mise_fixture(extra='\n[tasks."tools-test"x]\nX = "1"\n')
        self.assertReaderRed(mise_problems, junk, self.line_of(junk, '"x]'))
        # A quoted part is one key, dots included: this is not a subtable of tools-test.
        self.assertEqual(mise_problems(mise_fixture(extra='\n[tasks."tools-test.env"]\nX = "1"\n')), [])
        text = mise_fixture(tt="run =")
        self.assertReaderRed(mise_problems, text, text.splitlines().index("run =") + 1)

    def test_M3_multi_line_string_bodies_are_text(self):
        live = (REPO / "mise.toml").read_text(encoding="utf-8")
        body = '\n[tasks.other]\nrun = """\n[ -d target ] || mkdir -p target\n[tasks.tools-test]\nrun = "x"\n"""\n'
        self.assertEqual(mise_problems(live + body), [])
        lit = "\n[tasks.other]\nrun = '''\n[ -d target ] || mkdir -p target\n'''\n"
        self.assertEqual(mise_problems(live + lit), [])
        for bad in (
            '\n[tasks.other]\nrun = """\nnever closed\n',
            '\n[tasks.other]\nrun = """\nescaped close \\"""\n',
            '\n[tasks.other]\nrun = """\nfour quotes""""\n',
        ):
            self.assertReaderRed(mise_problems, live + bad)
        for cont in ('  [ "x" ],', '  ["a"],', '  "[ -d t ]",'):
            arr = '\n[tasks.other]\nrun = [\n%s\n  "b",\n]\n[tasks.another]\nrun = "x"\n' % cont
            self.assertEqual(mise_problems(live + arr), [], cont)
        self.assertReaderRed(mise_problems, live + '\n[tasks.other]\nrun = [\n  [ "x" ],\n')
        stray = live + '\n[tasks.other]\nrun = "x" ]\n'
        self.assertReaderRed(mise_problems, stray, self.line_of(stray, '"x" ]'))

    def test_M2_tools_test_has_no_sources_or_outputs(self):
        for line in ('sources = ["scripts/**"]', 'outputs = ["x"]'):
            tt = 'run = "bash scripts/python-suites.sh"\n' + line
            self.assertTrue(any(line.split()[0] in p for p in mise_problems(mise_fixture(tt=tt))), line)


if __name__ == "__main__":
    unittest.main()
