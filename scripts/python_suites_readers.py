"""Readers for `.github/workflows/ci.yml` and `mise.toml` (slice A-iii; python-pin PR-1).

Used only by `test_python_suites_ci.py`. `ci.yml` is read as text (no stdlib YAML parser exists
on any version): the YAML reader models a stated subset and raises `ReaderError` naming the line
on anything else inside the regions it reads; its grammar is stated on the reader. `mise.toml` is
read with `tomllib`, which every interpreter the driver admits has. Not a set member, no tests,
no entry point.
"""

from __future__ import annotations

import re
import tomllib

class ReaderError(Exception):
    """An unmodelled construct inside a region the pins read: red, naming the line."""

    def __init__(self, lineno, why, text):
        super().__init__("line %d: %s: %r" % (lineno, why, text))
        self.lineno = lineno


class YScalar(str):
    lineno = 0
    style = "plain"  # plain | quoted | block | null


class YMap(dict):
    lineno = 0


class YSeq(list):
    lineno = 0


def _scalar(value, lineno, style):
    s = YScalar(value)
    s.lineno = lineno
    s.style = style
    return s


_YKEY = re.compile(r"""^(?:"([^"\\]*)"|'([^']*)'|([A-Za-z0-9_][A-Za-z0-9_.-]*))[ ]*:(?=[ ]|$)""")
_BLOCK_HEADER = re.compile(r"^[|>][+-]?$")
_BAD_PLAIN_START = set("&*!{}[]|>'\"%@`,#")


def _key(km):
    """The key text of a `_YKEY` match, whichever alternative matched."""
    return next(g for g in km.groups() if g is not None)


class _YamlSubset:
    """Block-style YAML, the subset GitHub workflows in this repository use.

    Modelled: block mappings and sequences (`- ` items, incl. a sequence at
    its parent key's indent); keys that are plain `[A-Za-z0-9_][A-Za-z0-9_.-]*`
    or single-line quoted; values that are plain scalars on one line, quoted
    scalars closed on the same line, one-line flow sequences of such scalars,
    or block scalars with header `|` `>` `|-` `>-` `|+` `>+` (the body is text).
    Full-line and trailing ` #` comments. Everything else -- anchors, aliases,
    tags, explicit `?` keys, merge keys `<<`, flow mappings, multi-line flow
    or quoted or plain scalars, indentation indicators, tabs, duplicate keys,
    nested `- -` -- raises ReaderError naming the line (fail closed)."""

    def __init__(self, lines):
        self.lines = lines  # [(lineno, raw)]
        self.i = 0

    def _peek(self):
        while self.i < len(self.lines):
            lineno, raw = self.lines[self.i]
            content = raw.lstrip(" ")
            if not content.strip() or content.startswith("#"):
                self.i += 1
                continue
            if raw[: len(raw) - len(content)].count("\t") or content.startswith("\t"):
                raise ReaderError(lineno, "tab indentation", raw)
            return lineno, len(raw) - len(content), content.rstrip("\n").rstrip()
        return None

    @staticmethod
    def _is_item(content):
        return content == "-" or content.startswith("- ")

    def parse(self):
        p = self._peek()
        if p is None:
            return None
        node = self.block()
        rest = self._peek()
        if rest is not None:
            lineno, _, content = rest
            raise ReaderError(lineno, "unexpected indentation", content)
        return node

    def block(self):
        lineno, ind, content = self._peek()
        return self.seq(ind) if self._is_item(content) else self.mapping(ind)

    def mapping(self, indent, first=None):
        m = YMap()
        if first is not None:
            m.lineno = first[2]
            self._entry(m, indent, *first)
        while True:
            p = self._peek()
            if p is None or p[1] < indent:
                break
            lineno, ind, content = p
            if ind > indent:
                raise ReaderError(lineno, "unexpected indentation", content)
            if self._is_item(content):
                break
            km = _YKEY.match(content)
            if km is None:
                raise ReaderError(lineno, "unmodelled key-position construct", content)
            self.i += 1
            if not m:
                m.lineno = lineno
            key = _key(km)
            self._entry(m, indent, key, content[km.end():], lineno)
        return m

    def _entry(self, m, indent, key, rest, lineno):
        if key in m:
            raise ReaderError(lineno, "duplicate key %r" % key, key)
        m[key] = self.value(rest, indent, lineno)

    def value(self, rest, indent, lineno):
        v = rest.strip(" ")
        if v == "" or v.startswith("#"):
            p = self._peek()
            if p is not None and p[1] > indent:
                return self.block()
            if p is not None and p[1] == indent and self._is_item(p[2]):
                return self.seq(indent)
            return _scalar("", lineno, "null")
        if v[0] in "|>":
            header = v.split(" #", 1)[0].strip()
            if not _BLOCK_HEADER.match(header):
                raise ReaderError(lineno, "unmodelled block-scalar header", v)
            body = []
            while self.i < len(self.lines):
                ln, raw = self.lines[self.i]
                c = raw.lstrip(" ")
                if c.strip() and len(raw) - len(c) <= indent:
                    break
                body.append(raw)
                self.i += 1
            return _scalar("\n".join(body), lineno, "block")
        node = self.inline(v, lineno)
        p = self._peek()
        if p is not None and p[1] > indent:
            raise ReaderError(p[0], "unmodelled continuation", p[2])
        return node

    @staticmethod
    def _quoted(s, lineno):
        """s starts with a quote; return (value, rest)."""
        q = s[0]
        i = 1
        out = []
        while i < len(s):
            c = s[i]
            if q == "'" and c == "'":
                if s[i + 1 : i + 2] == "'":
                    out.append("'")
                    i += 2
                    continue
                return "".join(out), s[i + 1 :]
            if q == '"' and c == "\\":
                if s[i + 1 : i + 2] in ('"', "\\"):
                    out.append(s[i + 1])
                    i += 2
                    continue
                raise ReaderError(lineno, "unmodelled escape", s)
            if q == '"' and c == '"':
                return "".join(out), s[i + 1 :]
            out.append(c)
            i += 1
        raise ReaderError(lineno, "quoted scalar not closed on its line", s)

    @staticmethod
    def _tail_ok(rest):
        r = rest.strip(" ")
        return r == "" or r.startswith("#")

    def inline(self, v, lineno):
        if v[0] in "'\"":
            val, rest = self._quoted(v, lineno)
            if not self._tail_ok(rest):
                raise ReaderError(lineno, "text after a quoted scalar", v)
            return _scalar(val, lineno, "quoted")
        if v[0] == "[":
            return self.flow_seq(v, lineno)
        if v[0] in _BAD_PLAIN_START or v[:2] in ("- ", "? ", ": ") or v in ("-", "?", ":"):
            raise ReaderError(lineno, "unmodelled value-position construct", v)
        text = v
        cut = re.search(r"[ ]#", text)
        if cut:
            text = text[: cut.start()]
        text = text.rstrip(" ")
        if ": " in text or text.endswith(":"):
            raise ReaderError(lineno, "mapping indicator inside a plain scalar", v)
        return _scalar(text, lineno, "plain")

    def flow_seq(self, v, lineno):
        s = YSeq()
        s.lineno = lineno
        body = v[1:]
        after_item = False
        while True:
            body = body.lstrip(" ")
            if body.startswith("]"):
                if s and not after_item:
                    raise ReaderError(lineno, "trailing comma in a flow sequence", v)
                if not self._tail_ok(body[1:]):
                    raise ReaderError(lineno, "text after a flow sequence", v)
                return s
            if body == "" or body.startswith("#"):
                raise ReaderError(lineno, "flow sequence not closed on its line", v)
            if body[0] in "'\"":
                val, body = self._quoted(body, lineno)
                s.append(_scalar(val, lineno, "quoted"))
            else:
                m = re.match(r"[^,\]\[{}#'\"&*!|>%@`:]+", body)
                if m is None or not m.group(0).strip():
                    raise ReaderError(lineno, "unmodelled flow-sequence item", v)
                s.append(_scalar(m.group(0).strip(), lineno, "plain"))
                body = body[m.end():]
            after_item = True
            body = body.lstrip(" ")
            if body.startswith(","):
                body = body[1:]
                after_item = False
            elif not body.startswith("]"):
                raise ReaderError(lineno, "unmodelled flow-sequence syntax", v)

    def seq(self, indent):
        s = YSeq()
        while True:
            p = self._peek()
            if p is None or p[1] < indent:
                break
            lineno, ind, content = p
            if ind > indent:
                raise ReaderError(lineno, "unexpected indentation", content)
            if not self._is_item(content):
                break
            self.i += 1
            if not s:
                s.lineno = lineno
            rest = content[1:]
            pad = len(rest) - len(rest.lstrip(" "))
            rest = rest.lstrip(" ")
            col = indent + 1 + pad
            if rest == "" or rest.startswith("#"):
                p2 = self._peek()
                if p2 is not None and p2[1] > indent:
                    s.append(self.block())
                else:
                    s.append(_scalar("", lineno, "null"))
                continue
            if self._is_item(rest):
                raise ReaderError(lineno, "unmodelled nested compact sequence", content)
            km = _YKEY.match(rest)
            if km is not None:
                key = _key(km)
                s.append(self.mapping(col, first=(key, rest[km.end():], lineno)))
            else:
                s.append(self.value(rest, indent, lineno))
        return s


class Workflow:
    """Reads `ci.yml`'s workflow-level `on:` block and named job regions.

    Region bounds come from a raw scan: top-level keys are lines at indent 0,
    jobs are the lines at the indent of the first key under `jobs:`; comment
    and blank lines never bound a region. Only the regions asked for are
    parsed, with `_YamlSubset`. A top-level line that is not a modelled key,
    a top-level key outside `WORKFLOW_KEYS` (an allowlist: `defaults:` would
    change how a `run:` is interpreted), or a line at job indent that is not a
    modelled key, is red."""

    def __init__(self, text):
        self.lines = list(enumerate(text.splitlines(), 1))
        self.top = {}
        order = []
        for idx, (lineno, raw) in enumerate(self.lines):
            if not raw.strip() or raw.lstrip(" ").startswith("#") or raw[0] == " ":
                continue
            km = _YKEY.match(raw)
            if km is None:
                raise ReaderError(lineno, "unmodelled top-level construct", raw)
            key = _key(km)
            if key in self.top:
                raise ReaderError(lineno, "duplicate top-level key %r" % key, raw)
            if key not in WORKFLOW_KEYS:
                raise ReaderError(lineno, "workflow-level key %r is not in the allowlist" % key, raw)
            self.top[key] = idx
            order.append(idx)
        self.top_end = {}
        for a, b in zip(order, order[1:] + [len(self.lines)]):
            self.top_end[a] = b
        self.jobs = {}
        if "jobs" in self.top:
            start, end = self.top["jobs"], self.top_end[self.top["jobs"]]
            first_raw = self.lines[start][1]
            head = _YKEY.match(first_raw)
            if head is None:
                raise ReaderError(self.lines[start][0], "unmodelled jobs: line", first_raw)
            if head.end() < len(first_raw.split(" #")[0].rstrip()):
                raise ReaderError(self.lines[start][0], "jobs: must be a block mapping", first_raw)
            job_indent = None
            heads = []
            for idx in range(start + 1, end):
                lineno, raw = self.lines[idx]
                c = raw.lstrip(" ")
                if not c.strip() or c.startswith("#"):
                    continue
                ind = len(raw) - len(c)
                if job_indent is None:
                    job_indent = ind
                if ind < job_indent:
                    raise ReaderError(lineno, "unexpected dedent under jobs:", raw)
                if ind == job_indent:
                    km = _YKEY.match(c)
                    if km is None:
                        raise ReaderError(lineno, "unmodelled job-position construct", raw)
                    name = _key(km)
                    if name in self.jobs:
                        raise ReaderError(lineno, "duplicate job %r" % name, raw)
                    heads.append((name, idx))
                    self.jobs[name] = None
            bounds = [h[1] for h in heads] + [end]
            for (name, idx), stop in zip(heads, bounds[1:]):
                self.jobs[name] = (idx, stop)

    def _parse(self, a, b):
        node = _YamlSubset(self.lines[a:b]).parse()
        return node

    def on(self):
        if "on" not in self.top:
            raise ReaderError(0, "no workflow-level on: block", "")
        idx = self.top["on"]
        node = self._parse(idx, self.top_end[idx])
        if not isinstance(node, dict):
            raise ReaderError(0, "empty workflow-level on: block", "")
        return node["on"]

    def env(self):
        """The workflow-level `env:` mapping, or None when absent."""
        if "env" not in self.top:
            return None
        idx = self.top["env"]
        return self._parse(idx, self.top_end[idx])["env"]

    def job(self, name):
        """The job's parsed mapping, or None when the job is absent."""
        span = self.jobs.get(name)
        if span is None:
            return None
        node = self._parse(*span)
        if not isinstance(node, dict):
            raise ReaderError(0, "empty job %r" % name, "")
        return node[name]


DRIVER_REL = "scripts/python-suites.sh"
DRIVER_ARGVS = (
    ["bash", DRIVER_REL],
    ["sh", DRIVER_REL],
    [DRIVER_REL],
    ["./" + DRIVER_REL],
    ["mise", "run", "tools-test"],
)
# The allowlists. Each admits what today's ci.yml uses; the two step allowlists
# also admit inert labels whether or not a step uses them (`name` / `id` on both,
# `timeout-minutes` on the driver step). Any other key is red, so a key that could mask the driver's exit or alter its shell / Python
# (job `defaults:` / `env:` / `continue-on-error`, workflow `defaults:`,
# `needs` / `if`, BASH_ENV, PYTHON*, proxy variables, ...) is refused without
# being named.
WORKFLOW_KEYS = {"name", "on", "concurrency", "permissions", "env", "jobs"}
TOOLS_JOB_KEYS = {"name", "runs-on", "timeout-minutes", "steps"}
WORKFLOW_ENV_KEYS = {"CARGO_TERM_COLOR", "CARGO_INCREMENTAL"}  # neither touches the shell or Python
DRIVER_STEP_KEYS = {"name", "id", "run", "timeout-minutes"}
SETUP_STEP_KEYS = {"name", "id", "uses", "with"}
PIN_FILE_INPUT = "python-version-file"
PIN_FILE = ".tool-versions"


def driver_is_command(cmd):
    """D1: the driver is the whole command -- argv[0], or the argument after
    bash / sh -- with no shell syntax at all (so nothing can mask its exit)."""
    return cmd.split() in DRIVER_ARGVS


def job_key_problems(job):
    """D1 / Q8: every job key outside the `tools` job's allowlist."""
    return ["job key %r is not in the tools-job allowlist" % k for k in sorted(set(job) - TOOLS_JOB_KEYS)]


def env_problems(env):
    """D1: every workflow-level `env:` key outside the allowlist (None = no `env:`)."""
    if env is None:
        return []
    if not isinstance(env, YMap):
        return ["workflow env is not a mapping"]
    return ["workflow env key %r is not in the allowlist" % k for k in sorted(set(env) - WORKFLOW_ENV_KEYS)]


def driver_step_index(job):
    """Q6: index of the credited driver step, or None."""
    if not isinstance(job, YMap) or job_key_problems(job):
        return None
    steps = job.get("steps")
    if not isinstance(steps, YSeq):
        return None
    for i, st in enumerate(steps):
        if (
            isinstance(st, YMap)
            and "run" in st
            and not set(st) - DRIVER_STEP_KEYS
            and isinstance(st["run"], YScalar)
            and st["run"].style in ("plain", "quoted")
            and driver_is_command(st["run"])
        ):
            return i
    return None


TOOLS_RUNS_ON = "ubuntu-24.04"


# What a step before the driver may not do: change where `python3` comes from or what the pin says.
PRE_DRIVER_USES = ("actions/checkout", "actions/setup-python")
PRE_DRIVER_RUN_MARKERS = (".tool-versions", "GITHUB_PATH", "GITHUB_ENV")


def before_driver_problems(steps, di):
    """Every step before the driver: `uses:` only checkout / setup-python (owner/repo compared
    case-insensitively), and no `run:` that names the pin file, $GITHUB_PATH or $GITHUB_ENV."""
    out = []
    for i, st in enumerate(steps):
        if i >= di:
            break
        if not isinstance(st, YMap):
            out.append("step %d before the driver is not a mapping" % i)
            continue
        uses = st.get("uses")
        if uses is not None:
            repo = str(uses).split("@", 1)[0].lower()
            if repo not in PRE_DRIVER_USES:
                out.append("step %d before the driver uses %s: only checkout and setup-python may precede it" % (i, uses))
        run = st.get("run")
        if run is not None:
            for marker in PRE_DRIVER_RUN_MARKERS:
                if marker.lower() in str(run).lower():
                    out.append("step %d before the driver has a run: naming %s (it may change the pin or the interpreter on PATH)" % (i, marker))
    return out


def setup_problems(job, di):
    """I1 x I3: the one leg reads the pin only through `python-version-file`, on the pinned image.

    Red unless: exactly one step whose `uses` starts `actions/setup-python@` exists in the whole
    job (a second one is a second spelling of the interpreter), it precedes the credited driver
    step, its keys are within SETUP_STEP_KEYS, its `with` is a mapping whose key set is exactly
    {python-version-file} and whose value is the plain or quoted scalar `.tool-versions`; and
    `runs-on` is exactly TOOLS_RUNS_ON (`ubuntu-latest` will not keep serving the pinned patch)."""
    problems = []
    if job.get("runs-on") != TOOLS_RUNS_ON:
        problems.append("runs-on is not exactly %s" % TOOLS_RUNS_ON)
    steps = job.get("steps")
    found = []
    if isinstance(steps, YSeq):
        found = [
            i for i, st in enumerate(steps) if isinstance(st, YMap) and str(st.get("uses", "")).lower().startswith("actions/setup-python@")
        ]
    if not found:
        return problems + ["no actions/setup-python step in the job"]
    if len(found) != 1:
        return problems + ["%d actions/setup-python steps: exactly one is allowed (a second is a second spelling of the interpreter)" % len(found)]
    if di is None:
        return problems + ["no credited driver step to follow the actions/setup-python step"]
    if found[0] > di:
        return problems + ["the actions/setup-python step does not precede the driver step"]
    problems += before_driver_problems(steps, di)
    setup = steps[found[0]]
    if set(setup) - SETUP_STEP_KEYS:
        problems.append("the actions/setup-python step has keys outside %s: %s" % (sorted(SETUP_STEP_KEYS), sorted(set(setup) - SETUP_STEP_KEYS)))
    with_ = setup.get("with")
    if not isinstance(with_, YMap) or set(with_) != {PIN_FILE_INPUT}:
        problems.append("the actions/setup-python `with` is not exactly the one key %s" % PIN_FILE_INPUT)
    else:
        val = with_[PIN_FILE_INPUT]
        if not (isinstance(val, YScalar) and val.style in ("plain", "quoted") and str(val) == PIN_FILE):
            problems.append("%s is not exactly the scalar %s" % (PIN_FILE_INPUT, PIN_FILE))
    return problems


def _one_trigger_problems(on, name):
    if not isinstance(on, YMap) or name not in on:
        return ["on: has no %s mapping" % name]
    t = on[name]
    if not isinstance(t, YMap) or set(t) != {"branches"}:
        return ["%s keys are not exactly branches" % name]
    if not (isinstance(t["branches"], YSeq) and list(t["branches"]) == ["main"]):
        return ["%s branches is not exactly [main]" % name]
    return []


def trigger_problems(on):
    """Q8: pull_request and push each carry exactly branches: [main] and nothing
    that narrows them."""
    return _one_trigger_problems(on, "pull_request") + _one_trigger_problems(on, "push")


def tools_problems(text):
    wf = Workflow(text)
    problems = trigger_problems(wf.on())
    job = wf.job("tools")
    if not isinstance(job, YMap) or not job:
        return problems + ["tools job absent or empty"]
    problems += env_problems(wf.env())
    problems += job_key_problems(job)
    di = driver_step_index(job)
    if di is None:
        problems.append("no run: step executes the driver")
    problems += setup_problems(job, di)
    return problems


TOOLS_TEST_RUN = "bash " + DRIVER_REL
CI_TASK_KEYS = {"description", "depends"}
TOOLS_TEST_TASK_KEYS = {"description", "run"}


def mise_problems(text):
    """Q4 / N7 over parsed `mise.toml`: `[tasks.ci]` and `[tasks.tools-test]`.

    `tasks` is a table; `tasks.ci` a table with keys within CI_TASK_KEYS whose `depends` is a list
    of strings containing "tools-test"; `tasks.tools-test` a table with keys within
    TOOLS_TEST_TASK_KEYS (an allowlist: `sources`, `outputs`, `env`, `dir` and any future key are
    red, naming it -- mise would skip a task on unchanged `sources`) whose `run` is exactly
    TOOLS_TEST_RUN. A TOML syntax error is one problem naming tomllib's message. The install
    settings are not this reader's (the T5 member asserts them, W2)."""
    try:
        doc = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        return ["mise.toml does not parse: %s" % e]
    tasks = doc.get("tasks")
    if not isinstance(tasks, dict):
        return ["[tasks] is not a table"]
    problems = []
    ci = tasks.get("ci")
    if not isinstance(ci, dict):
        problems.append("[tasks.ci] is not a table")
    else:
        for k in sorted(set(ci) - CI_TASK_KEYS):
            problems.append("[tasks.ci] has key %r outside %s" % (k, sorted(CI_TASK_KEYS)))
        deps = ci.get("depends")
        if not (isinstance(deps, list) and all(isinstance(d, str) for d in deps) and "tools-test" in deps):
            problems.append('[tasks.ci].depends is not a list of strings containing "tools-test"')
    tt = tasks.get("tools-test")
    if not isinstance(tt, dict):
        problems.append("[tasks.tools-test] is not a table")
    else:
        for k in sorted(set(tt) - TOOLS_TEST_TASK_KEYS):
            problems.append("[tasks.tools-test] has key %r outside %s" % (k, sorted(TOOLS_TEST_TASK_KEYS)))
        if tt.get("run") != TOOLS_TEST_RUN:
            problems.append("[tasks.tools-test].run is not the string %r" % TOOLS_TEST_RUN)
    return problems
