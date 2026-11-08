"""Reader and CI-wiring pins (R1-R9, N4-N8, ...) for scripts/python-suites.sh.

Reads `.github/workflows/ci.yml` through the text reader and `mise.toml` through the `tomllib`
reader in `python_suites_readers.py`; fixtures for them are below. N4-N8 are the pins of the
python-pin PR-1 plan (docs/plans/2026-10-python-pin-pr1-tests.md, 6.2): N4 replaces Q7,
N5 replaces R2, N7 / N8 restate Q4 / M1-M3 over parsed data.
"""

from __future__ import annotations

import tomllib
import unittest

from python_suites_fixtures import REPO
from python_suites_readers import (
    ReaderError,
    Workflow,
    driver_step_index,
    job_key_problems,
    mise_problems,
    setup_problems,
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
)
TOOLS_SETUP = (
    "      - uses: actions/setup-python@v6\n"
    "        with:\n"
    "          python-version-file: .tool-versions\n"
)
TOOLS_STEPS = (
    "    steps:\n"
    "      - uses: actions/checkout@v6\n"
    + TOOLS_SETUP
    + "      - name: Run the Python suites\n"
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

    def setup(self, text):
        job = self.tools_job(text)
        return setup_problems(job, driver_step_index(job))

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

    def test_N4_one_setup_step_reads_only_the_pin_file(self):
        good = workflow()
        self.assertEqual(self.setup(good), [])
        setup = TOOLS_SETUP
        driver = "      - name: Run the Python suites\n        run: bash scripts/python-suites.sh\n"

        def with_steps(steps):
            return workflow(TOOLS_HEAD + "    steps:\n" + steps)

        checkout = "      - uses: actions/checkout@v6\n"
        file_key = "          python-version-file: .tool-versions\n"
        # Each variant is red, with its own message (E20).
        one_key = ["the actions/setup-python `with` is not exactly the one key python-version-file"]
        self.assertEqual(self.setup(good.replace(file_key, file_key + "          python-version: '3.12'\n")), one_key)
        self.assertEqual(self.setup(good.replace(file_key, "          python-version: '3.15.2'\n")), one_key)
        self.assertEqual(self.setup(good.replace(file_key, file_key + "          check-latest: true\n")), one_key)
        not_file = ["python-version-file is not exactly the scalar .tool-versions"]
        self.assertEqual(self.setup(good.replace(".tool-versions\n", ".python-version\n", 1)), not_file)
        self.assertEqual(self.setup(good.replace(".tool-versions\n", "./.tool-versions\n", 1)), not_file)
        self.assertEqual(self.setup(good.replace(".tool-versions\n", "${{ matrix.python-version }}\n", 1)), not_file)
        # Quoting a scalar changes nothing about its value.
        self.assertEqual(self.setup(good.replace(".tool-versions\n", "'.tool-versions'\n", 1)), [])
        # A block scalar is another spelling.
        self.assertEqual(self.setup(good.replace(file_key, "          python-version-file: |\n            .tool-versions\n")), not_file)
        keys = self.setup(good.replace("      - uses: actions/setup-python@v6\n", "      - uses: actions/setup-python@v6\n        if: always()\n"))
        self.assertEqual(len(keys), 1, keys)
        self.assertIn("keys outside", keys[0])
        second = "2 actions/setup-python steps: exactly one is allowed (a second is a second spelling of the interpreter)"
        before = with_steps(checkout + setup + setup + driver)
        after = with_steps(checkout + setup + driver + setup)
        self.assertEqual(self.setup(before), [second])
        self.assertEqual(self.setup(after), [second])
        self.assertEqual(self.setup(with_steps(checkout + driver + setup)), ["the actions/setup-python step does not precede the driver step"])
        self.assertEqual(self.setup(with_steps(checkout + driver)), ["no actions/setup-python step in the job"])
        self.assertEqual(self.setup(good.replace("ubuntu-24.04", "ubuntu-latest")), ["runs-on is not exactly ubuntu-24.04"])
        # F13: the count is case-insensitive on owner/repo.
        shouting = "      - uses: Actions/Setup-Python@v7\n        with:\n          python-version-file: .tool-versions\n"
        self.assertEqual(self.setup(with_steps(checkout + setup + shouting + driver)), [second])
        # F13: every step before the driver is scanned for what could change the pin or the interpreter.
        def pre(step):
            return self.setup(with_steps(checkout + step + setup + driver))

        def run(cmd):
            return "      - run: %s\n" % cmd

        self.assertEqual(pre(run("echo hi")), [])
        for cmd, marker in (
            ("echo 'python 3.12.0' > .tool-versions", ".tool-versions"),
            ("echo /opt/bin >> $GITHUB_PATH", "GITHUB_PATH"),
            ("echo X=1 >> \"$GITHUB_ENV\"", "GITHUB_ENV"),
        ):
            got = pre(run(cmd))
            self.assertEqual(len(got), 1, got)
            self.assertIn("naming " + marker, got[0])
        late = self.setup(with_steps(checkout + setup + run("echo > .tool-versions") + driver))
        self.assertEqual(len(late), 1, late)
        got = pre("      - uses: some/other-action@v1\n")
        self.assertEqual(len(got), 1, got)
        self.assertIn("only checkout and setup-python may precede", got[0])
        self.assertEqual(pre("      - uses: Actions/Checkout@v6\n"), [])
        # Through the whole job reader too (the reader is wired into tools_problems).
        for text in (before, after, with_steps(checkout + run("echo > .tool-versions") + setup + driver), good.replace("ubuntu-24.04", "ubuntu-latest")):
            self.assertTrue(tools_problems(text), text)
        self.assertEqual(tools_problems(good), [])

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

    def test_N5_modelled_values_read_correctly(self):
        self.assertEqual(list(Workflow(workflow()).on()["pull_request"]["branches"]), ["main"])
        job = self.tools_job(workflow())
        self.assertEqual(str(job["steps"][1]["with"]["python-version-file"]), ".tool-versions")
        self.assertEqual(list(job["steps"][1]["with"]), ["python-version-file"])

    def test_N6_strategy_is_a_job_key_red(self):
        planted = "    strategy:\n      matrix:\n        python-version: ['3.15.2']\n"
        job = self.tools_job(workflow(tools_with(planted)))
        self.assertEqual(job_key_problems(job), ["job key 'strategy' is not in the tools-job allowlist"])
        self.assertIn("job key 'strategy' is not in the tools-job allowlist", tools_problems(workflow(tools_with(planted))))

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
            "          python-version-file: .tool-versions\n"
            "      - run: bash scripts/python-suites.sh\n"
        )
        job = self.tools_job(workflow(bare, after=nxt))
        self.assertEqual(job_key_problems(job), [])
        self.assertIsNone(driver_step_index(job))
        self.assertTrue(setup_problems(job, driver_step_index(job)))
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

    # N7 / N8: `mise.toml` over parsed data.

    def test_N7_mise_toml_asserted_over_parsed_data(self):
        self.assertEqual(mise_problems(mise_fixture()), [])
        outside = "[tasks.tools-test] has key %r outside ['description', 'run']"
        # A denylist of two keys, or a text grep, would pass these.
        for line, key in (
            ('sources = ["scripts/**"]', "sources"),
            ('outputs = ["x"]', "outputs"),
            ('env = { X = "1" }', "env"),
            ('dir = "scripts"', "dir"),
            ('some_future_key = "x"', "some_future_key"),
        ):
            tt = 'run = "bash scripts/python-suites.sh"\n' + line
            self.assertEqual(mise_problems(mise_fixture(tt=tt)), [outside % key], line)
        for ci in ('depends = ["check"]', "depends = { a = 1 }", 'depends = "tools-test"', 'depends = [1, "tools-test"]'):
            self.assertEqual(
                mise_problems(mise_fixture(ci=ci)),
                ['[tasks.ci].depends is not a list of strings containing "tools-test"'],
                ci,
            )
        self.assertEqual(
            mise_problems(mise_fixture(ci='depends = ["tools-test"]\nsome_future_key = 1')),
            ["[tasks.ci] has key 'some_future_key' outside ['depends', 'description']"],
        )
        want_run = ["[tasks.tools-test].run is not the string 'bash scripts/python-suites.sh'"]
        for tt in ('run = "true"', 'run = ["bash scripts/python-suites.sh"]', 'run = "bash scripts/python-suites.sh --x"', ""):
            self.assertEqual(mise_problems(mise_fixture(tt=tt)), want_run, tt)
        array = '[tasks.ci]\ndepends = ["tools-test"]\n\n[[tasks.tools-test]]\nrun = "bash scripts/python-suites.sh"\n'
        self.assertEqual(mise_problems(array), ["[tasks.tools-test] is not a table"])
        self.assertEqual(mise_problems("[tasks.ci]\ndepends = [\"tools-test\"]\n"), ["[tasks.tools-test] is not a table"])
        self.assertEqual(mise_problems("[settings]\nx = 1\n"), ["[tasks] is not a table"])
        # A syntax error is one problem, naming tomllib's own message.
        broken = mise_fixture(tt="run =")
        with self.assertRaises(tomllib.TOMLDecodeError) as cm:
            tomllib.loads(broken)
        self.assertEqual(mise_problems(broken), ["mise.toml does not parse: %s" % cm.exception])

    def test_N8_the_reader_is_the_parser(self):
        live = (REPO / "mise.toml").read_text(encoding="utf-8")
        # Text that looks like a section inside a multi-line string creates no section.
        body = '\n[tasks.other]\nrun = """\n[ -d target ] || mkdir -p target\n[tasks.tools-test]\nrun = "x"\n"""\n'
        self.assertEqual(mise_problems(live + body), [])
        lit = "\n[tasks.other]\nrun = \'\'\'\n[tasks.tools-test.env]\n\'\'\'\n"
        self.assertEqual(mise_problems(live + lit), [])
        env = "[tasks.tools-test] has key 'env' outside ['description', 'run']"
        # A subtable, in any quoting, is the unknown key `env`.
        for header in ("[tasks.tools-test.env]", '[tasks."tools-test".env]', "[tasks.'tools-test'.env]"):
            self.assertEqual(mise_problems(mise_fixture(extra="\n%s\nX = \"1\"\n" % header)), [env], header)
        # So is the dotted-key spelling, an inline table, and an escaped key spelling `tasks`.
        base = 'ci.depends = ["tools-test"]\n%s.tools-test.run = "bash scripts/python-suites.sh"\n'
        for tasks in ("tasks", '"tasks"', "'tasks'", '"task\\u0073"'):
            text = (base % tasks).replace("ci.depends", tasks + ".ci.depends", 1) + '%s."tools-test".env.X = "1"\n' % tasks
            self.assertEqual(mise_problems(text), [env], text)
        inline = 'tasks = { ci = { depends = ["tools-test"] }, tools-test = { run = "bash scripts/python-suites.sh", env = { X = "1" } } }\n'
        self.assertEqual(mise_problems(inline), [env])
        # A quoted part is one key, dots included: this is not a subtable of tools-test.
        self.assertEqual(mise_problems(mise_fixture(extra='\n[tasks."tools-test.env"]\nX = "1"\n')), [])
        # The same table defined twice does not parse.
        dup = mise_fixture(extra='\n[tasks.tools-test]\nrun = "true"\n')
        self.assertTrue(mise_problems(dup)[0].startswith("mise.toml does not parse:"), dup)


if __name__ == "__main__":
    unittest.main()
