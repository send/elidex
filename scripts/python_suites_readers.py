"""Text readers for `.github/workflows/ci.yml` and `mise.toml` (slice A-iii).

Used only by `test_python_suites_ci.py`. They model a stated subset and raise
`ReaderError` naming the line on anything else inside the regions they read;
their grammar is stated on each reader. Not a set member, no tests, no entry
point.
"""

from __future__ import annotations

import re

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
    """The key text of a `_YKEY` / `_TKEY` match, whichever alternative matched."""
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
TOOLS_JOB_KEYS = {"name", "runs-on", "timeout-minutes", "strategy", "steps"}
WORKFLOW_ENV_KEYS = {"CARGO_TERM_COLOR", "CARGO_INCREMENTAL"}  # neither touches the shell or Python
DRIVER_STEP_KEYS = {"name", "id", "run", "timeout-minutes"}
SETUP_STEP_KEYS = {"name", "id", "uses", "with"}
MATRIX_EXPR = "${{ matrix.python-version }}"


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


def q7_problems(job, di):
    """Q7: the two-leg matrix, consumed by the last setup-python before the driver,
    on the runner image that serves the "3.9" leg (`ubuntu-latest` will not
    keep doing so)."""
    problems = []
    if job.get("runs-on") != TOOLS_RUNS_ON:
        problems.append("runs-on is not exactly %s" % TOOLS_RUNS_ON)
    matrix = job.get("strategy", {}).get("matrix") if isinstance(job.get("strategy"), YMap) else None
    if not isinstance(matrix, YMap):
        return ["no strategy.matrix mapping"]
    if set(matrix) != {"python-version"}:
        problems.append("matrix keys %s are not exactly python-version" % sorted(matrix))
    legs = matrix.get("python-version")
    if not (
        isinstance(legs, YSeq)
        and sorted(legs) == ["3.9", "3.x"]
        and all(isinstance(l, YScalar) and l.style == "quoted" for l in legs)
    ):
        problems.append("python-version legs are not exactly quoted \"3.9\" and \"3.x\"")
    if di is None:
        return problems + ["no credited driver step to precede"]
    setup = None
    for st in job["steps"][:di]:
        if isinstance(st, YMap) and str(st.get("uses", "")).startswith("actions/setup-python@"):
            setup = st
    if setup is None:
        return problems + ["no actions/setup-python step before the driver step"]
    with_ = setup.get("with")
    if (
        set(setup) - SETUP_STEP_KEYS
        or not isinstance(with_, YMap)
        or set(with_) != {"python-version"}
        or re.sub(r"\s+", " ", str(with_["python-version"])) != MATRIX_EXPR
    ):
        problems.append("the last setup-python before the driver does not consume %s" % MATRIX_EXPR)
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
    problems += q7_problems(job, di)
    return problems


_TKEY = re.compile(r"""^(?:"([^"\\]*)"|'([^']*)'|([A-Za-z0-9_-]+))[ ]*=[ ]*""")
_THEADER = re.compile(r"^\[\[?([^\[\]]+)\]\]?[ ]*(#.*)?$")
_TPART = re.compile(r"""[ ]*(?:"([^"\\]*)"|'([^']*)'|([A-Za-z0-9_-]+))[ ]*""")


def _toml_header_path(h, lineno):
    """The header's key parts, each bare or quoted (the `_TKEY` key grammar), joined
    by `.`. Anything else -- a backslash escape, a part that is not one key, text
    between parts -- raises."""
    parts, pos = [], 0
    while True:
        m = _TPART.match(h, pos)
        if m is None:
            raise ReaderError(lineno, "unmodelled header key", h)
        parts.append(_key(m))
        pos = m.end()
        if pos == len(h):
            return parts
        if h[pos] != ".":
            raise ReaderError(lineno, "unmodelled header key", h)
        pos += 1


def _toml_value(v, lineno):
    v = v.strip()
    if not v:
        raise ReaderError(lineno, "missing value", v)
    if v.startswith(('"""', "'''")):
        raise ReaderError(lineno, "multi-line string not modelled", v)
    if v[:1] in "\"'":
        q = v[0]
        end = v.find(q, 1)
        if end < 0 or (q == '"' and "\\" in v[1:end]):
            raise ReaderError(lineno, "unmodelled string", v)
        tail = v[end + 1 :].strip()
        if tail and not tail.startswith("#"):
            raise ReaderError(lineno, "text after a string", v)
        return v[1:end]
    if v.startswith("["):
        m = re.match(r"""^\[((?:\s*(?:"[^"\\]*"|'[^']*')\s*,)*\s*(?:"[^"\\]*"|'[^']*')?\s*)\]\s*(#.*)?$""", v)
        if m is None:
            raise ReaderError(lineno, "unmodelled array (one line of strings only)", v)
        return [s[1:-1] for s in re.findall(r"""("[^"\\]*"|'[^']*')""", m.group(1))]
    if re.match(r"^(true|false)\s*(#.*)?$", v):
        return v.startswith("true")
    raise ReaderError(lineno, "unmodelled value", v)


_CLOSED = (None, 0)


def _scan_strings(raw, carry, lineno):
    """Advance the carry (open multi-line string delimiter or None, open-array
    depth) over one line; return the new carry. The body of a multi-line string
    and the continuation lines of an open array are text, never structure. A
    close that cannot be read unambiguously (an escaped `\"\"\"`, a fourth quote
    right after it, a `]` with no array open) raises."""
    delim, depth = carry
    i, n = 0, len(raw)
    while i < n:
        if delim is not None:
            j = raw.find(delim, i)
            if j < 0:
                return delim, depth
            slashes = len(raw[:j]) - len(raw[:j].rstrip("\\"))
            if (delim == '"""' and slashes % 2) or raw[j + 3 : j + 4] == delim[0]:
                raise ReaderError(lineno, "ambiguous multi-line string close", raw)
            delim, i = None, j + 3
            continue
        c = raw[i]
        if c == "#":
            return None, depth
        if raw.startswith(('"""', "'''"), i):
            delim, i = raw[i : i + 3], i + 3
        elif c in "\"'":
            j = i + 1
            while j < n and raw[j] != c:
                j += 2 if (c == '"' and raw[j] == "\\") else 1
            i = j + 1
        else:
            depth += (c == "[") - (c == "]")
            if depth < 0:
                raise ReaderError(lineno, "a ] with no array open", raw)
            i += 1
    return delim, depth


def mise_sections(text, wanted):
    """Reads the `[tasks.<name>]` sections in `wanted` of `mise.toml`.

    Modelled inside those sections: blank and `#` lines; `key = value` with a
    bare or quoted key and a value that is a one-line basic/literal string, a
    one-line array of such strings, or true/false, with an optional trailing
    comment. Anything else inside them raises ReaderError naming the line, and
    so does anything elsewhere that could also define those tasks: a `[tasks]`
    table, a subtable `[tasks.<name>.x]`, or a dotted `tasks.` key. A missing
    or duplicated section is red. The body of a multi-line string (`\"\"\"` /
    `'''`) opened outside the read sections is text, and so are the continuation
    lines of an array left open there: the file is tracked across lines, so a
    line in them that looks like a header or a key is neither; an unterminated
    string or array, or a close that is ambiguous, is a ReaderError."""
    out = {}
    current = None
    carry = _CLOSED
    for lineno, raw in enumerate(text.splitlines(), 1):
        if carry != _CLOSED:
            carry = _scan_strings(raw, carry, lineno)
            continue
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        hm = _THEADER.match(line)
        if line.startswith("["):
            if hm is None:
                raise ReaderError(lineno, "unmodelled table header", raw)
            path = _toml_header_path(hm.group(1), lineno)
            if path == ["tasks"] or (len(path) > 2 and path[:2] in (["tasks", w] for w in wanted)):
                raise ReaderError(lineno, "a second way to define a read task", raw)
            current = None
            for w in wanted:
                if path == ["tasks", w]:
                    if w in out:
                        raise ReaderError(lineno, "duplicate section", raw)
                    out[w] = {}
                    current = w
            continue
        if current is None:
            if re.match(r"""^["']?tasks["']?\s*\.""", line):
                raise ReaderError(lineno, "a dotted tasks. key", raw)
            carry = _scan_strings(raw, carry, lineno)
            continue
        km = _TKEY.match(line)
        if km is None:
            raise ReaderError(lineno, "unmodelled key", raw)
        key = _key(km)
        if key in out[current]:
            raise ReaderError(lineno, "duplicate key %r" % key, raw)
        out[current][key] = (_toml_value(line[km.end():], lineno), lineno)
    if carry != _CLOSED:
        raise ReaderError(0, "unterminated multi-line string or array", carry)
    for w in wanted:
        if w not in out:
            raise ReaderError(0, "section [tasks.%s] missing" % w, "")
    return out


def mise_problems(text):
    """Q4 over mise.toml's [tasks.ci] and [tasks.tools-test]."""
    s = mise_sections(text, ("ci", "tools-test"))
    problems = []
    ci, tt = s["ci"], s["tools-test"]
    for k in set(ci) - {"description", "depends"}:
        problems.append("[tasks.ci] has unmodelled key %s" % k)
    deps = ci.get("depends", (None, 0))[0]
    if not (isinstance(deps, list) and "tools-test" in deps):
        problems.append('[tasks.ci].depends does not contain "tools-test"')
    for k in ("sources", "outputs"):
        if k in tt:
            problems.append("[tasks.tools-test] has %s (mise would skip it when unchanged)" % k)
    for k in set(tt) - {"description", "run", "sources", "outputs"}:
        problems.append("[tasks.tools-test] has unmodelled key %s" % k)
    if tt.get("run", (None, 0))[0] != "bash " + DRIVER_REL:
        problems.append("[tasks.tools-test].run is not the string %r" % ("bash " + DRIVER_REL))
    return problems


