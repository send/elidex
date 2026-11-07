"""The interpreter pin: its one reader, exact admission and the refusal (python-pin PR-1).

`scripts/python-suites.sh` Step 1 runs this file as `python3 -I -S scripts/python_pin.py` before
anything else. Exit 0: the running interpreter is the pin in the root `.tool-versions`; the last stderr line is
`python-pin: admitted X.Y.Z`. Exit 1: it is
not; what was observed is printed to stderr (the facts, then one generic block). Exit 2: the pin
file itself is unreadable or malformed, so no version can be compared. Exit 3: an uncaught
exception in this module (its traceback is printed). Non-zero runs end with `python-pin: exit N`
(N = 1, 2 or 3), so a caller can tell this file's exit from a failure of the interpreter before it ran.

Plan of record: docs/plans/2026-10-python-pin-pr1-pin-and-gate.md (section 4.2);
umbrella docs/plans/2026-10-python-pin-umbrella.md (sections 4.1-4.3).

Bump rule: stated once, in the umbrella body section 4.1 (the criterion and its rows); this header
points to it and does not restate it.

Shape: stdlib only; the top-level imports are `os` and `sys` and nothing else (`subprocess`,
`json` and `signal` are imported inside `ask_mise`, on the refusal path only); Python 3.9 grammar (the oldest
interpreter this module is refused on is measured, section 10 step 4 of the plan); `%`-formatting;
`sys.version_info` is read by index, never by attribute.

The oracle probe recipe (umbrella body 0 isolation rule; records R11 T1's environment block), the
program's isolation lines first, then questions (a)-(d) as `ask_mise` asks them:

    ISO=$(mktemp -d); P=$ISO/proj; mkdir -p "$ISO"/{data,cache,state,config} "$P"; : > "$ISO/global.toml"
    export MISE_DATA_DIR=$ISO/data MISE_CACHE_DIR=$ISO/cache MISE_STATE_DIR=$ISO/state \\
           MISE_CONFIG_DIR=$ISO/config MISE_GLOBAL_CONFIG_FILE=$ISO/global.toml MISE_OFFLINE=1 \\
           MISE_AUTO_INSTALL=false MISE_NOT_FOUND_AUTO_INSTALL=false MISE_TRUSTED_CONFIG_PATHS=$P
    cd "$P"; cp <repo>/.tool-versions .
    mise ls --current --json python        # (a)
    mise which python3                     # (b)
    mise config ls --json                  # (c)
    mise config get -f <path of (c)> env._.path    # (d), one per path
"""

import os
import sys

PIN_FILE = ".tool-versions"
ORACLE_TIMEOUT_S = 10
STDERR_CAP = 4000
GROUP = hasattr(os, "killpg")
PREFIX = "python-pin: "
_DIGITS = "0123456789"

GENERIC_BLOCK = (
    "python-pin: To run on `%(pin)s`: install it -- `mise install` in `%(root)s` (mise itself: README\n"
    "python-pin: *Prerequisites* / CONTRIBUTING *Getting Started*); or, if mise reports this worktree\n"
    "python-pin: untrusted, `mise trust` in `%(root)s`; or remove or change a mise input -- the `source`\n"
    "python-pin: above, an `[env] _.path` printed above, a file `mise config ls` lists, or an rc file\n"
    "python-pin: `.miserc*`, which it does not list; or launch through mise -- `mise -C %(root)s exec --\n"
    "python-pin: <tool>`. mise's answers again: `mise -C %(root)s ls --current --json python`,\n"
    "python-pin: `mise -C %(root)s which python3`, `mise -C %(root)s config ls`."
)


class PinFileError(Exception):
    """The pin file cannot be read or is not exactly one line `python X.Y.Z`."""


def repo_root():
    return os.path.dirname(os.path.dirname(os.path.realpath(__file__)))


def _why(text):
    """What a text that is not `python X.Y.Z` holds, in words (clause 1's parser, one form)."""
    if text == "":
        return "the file is empty"
    if text.startswith("\ufeff"):
        return "the file starts with a byte order mark"
    if "\r" in text:
        return "the file holds a carriage return (CRLF line endings)"
    body = text[:-1] if text.endswith("\n") else text
    lines = body.split("\n")
    if len(lines) > 1:
        return "the file holds %d lines (a second line: %r)" % (len(lines), lines[1])
    line = lines[0]
    if line == "":
        return "the file holds a blank line"
    if line.startswith("#"):
        return "the line is a comment: %r" % line
    if "\t" in line:
        return "the line holds a tab: %r" % line
    if line != line.strip():
        return "the line has leading or trailing white space: %r" % line
    if "  " in line:
        return "the line holds two spaces in a row: %r" % line
    head, sep, version = line.partition(" ")
    if head != "python":
        return "the line names %r, not the tool `python`: %r" % (head, line)
    if sep == "":
        return "the line has no version: %r" % line
    if "#" in version:
        return "the line holds a trailing comment: %r" % line
    if " " in version:
        return "the line holds more than one version: %r" % line
    if version == "latest":
        return "the version is not an exact X.Y.Z: %r" % version
    return "the version is not an exact X.Y.Z (digits without leading zeros): %r" % version


def parse_pin(text):
    """Exactly `python X.Y.Z` plus an optional final newline; returns (X, Y, Z) as ints."""
    body = text[:-1] if text.endswith("\n") else text
    head, sep, version = body.partition(" ")
    if head != "python" or sep != " ":
        raise PinFileError(_why(text))
    parts = version.split(".")
    if len(parts) != 3:
        raise PinFileError(_why(text))
    numbers = []
    for part in parts:
        if part == "" or any(c not in _DIGITS for c in part):
            raise PinFileError(_why(text))
        if len(part) > 1 and part[0] == "0":
            raise PinFileError(_why(text))
        numbers.append(int(part))
    return (numbers[0], numbers[1], numbers[2])


def read_pin(root):
    path = os.path.join(root, PIN_FILE)
    try:
        with open(path, "rb") as f:
            data = f.read()
    except OSError as e:
        raise PinFileError("no readable file (%s)" % (e.strerror or e))
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        raise PinFileError("the file is not UTF-8")
    return parse_pin(text)


def admitted(v, pin):
    return (v[0], v[1], v[2]) == pin and v[3] == "final"


def _warned(name):
    return (name.startswith("MISE_PYTHON_") and name != "MISE_PYTHON_VERSION") or name == "MISE_GITHUB_ATTESTATIONS"


def warning_lines(environ):
    return [
        "%swarning: %s=%s is set: it may override the tracked install setting for any install made in this environment"
        % (PREFIX, name, environ[name])
        for name in sorted(environ)
        if _warned(name)
    ]


def _reap(proc):
    """Kill `proc` (its whole group where there is one), wait for it and close its pipes."""
    if GROUP:
        try:
            os.killpg(proc.pid, 9)
        except OSError:
            pass
    else:
        proc.kill()
    proc.wait()
    proc.stdout.close()
    proc.stderr.close()


def ask_mise(cwd, timeout):
    """Ask mise questions (a)-(d), in this order and no other; diagnostic only, never admits.

    Returns one record per question: {"key", "label", "kind", ...}. Kinds: "answer" (lines),
    "exit" (rc, out, err), "unparsed" (out, err), "missing", "timeout", "notasked" (why).
    """
    import json
    import signal
    import subprocess

    stop = []

    def question(key, argv):
        label = " ".join(argv)
        if stop:
            return {"key": key, "label": label, "kind": "notasked", "why": stop[0]}
        # Once the process exists, any exit path (a timeout, a KeyboardInterrupt, any BaseException)
        # kills its group. SIGINT is blocked (where `pthread_sigmask` exists) from before Popen until
        # `proc` is bound, so an interrupt is delivered only when the group can be reaped; the
        # residual is a platform without `pthread_sigmask`, where an interrupt in Popen's blocking
        # error-pipe read, until the child execs, leaves mise running because no pid reaches this
        # frame.
        proc = None
        masked = hasattr(signal, "pthread_sigmask")
        try:
            if masked:
                signal.pthread_sigmask(signal.SIG_BLOCK, [signal.SIGINT])
            try:
                proc = subprocess.Popen(
                    argv,
                    cwd=cwd,
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    start_new_session=GROUP,
                )
            except OSError as e:
                if isinstance(e, FileNotFoundError):
                    reason = "mise not on PATH (FileNotFoundError)"
                else:
                    reason = "mise could not be started (%s)" % e
                stop.append(reason)
                return {"key": key, "label": label, "kind": "missing", "reason": reason}
            finally:
                if masked:
                    signal.pthread_sigmask(signal.SIG_UNBLOCK, [signal.SIGINT])
            out, err = proc.communicate(timeout=timeout)
        except BaseException as e:
            if proc is not None:
                _reap(proc)
            if isinstance(e, subprocess.TimeoutExpired):
                stop.append("an earlier question timed out")
                return {"key": key, "label": label, "kind": "timeout", "timeout": timeout}
            raise
        out = out.decode("utf-8", "replace")
        err = err.decode("utf-8", "replace")
        if proc.returncode != 0:
            return {"key": key, "label": label, "kind": "exit", "rc": proc.returncode, "out": out, "err": err}
        return {"key": key, "label": label, "kind": "answer", "out": out, "err": err}

    def shown(value):
        return "(absent)" if value is None else json.dumps(value)

    def parsed(rec, build):
        # (a) and (c): a JSON array of objects, else "does not parse"; `build` makes the one field
        # the question's printer reads.
        if rec["kind"] != "answer":
            return rec
        try:
            data = json.loads(rec["out"])
        except (ValueError, RecursionError):
            data = None
        if not isinstance(data, list) or not all(isinstance(x, dict) for x in data):
            return {"key": rec["key"], "label": rec["label"], "kind": "unparsed", "out": rec["out"], "err": rec["err"]}
        rec.update(build(data))
        return rec

    def items(data):
        fields = ("version", "source", "installed", "install_path")
        return {"items": [dict((k, shown(x.get(k))) for k in fields) for x in data]}

    def paths(data):
        return {"paths": [x["path"] for x in data if isinstance(x.get("path"), str)]}

    a = parsed(question("a", ["mise", "ls", "--current", "--json", "python"]), items)
    b = question("b", ["mise", "which", "python3"])
    c = parsed(question("c", ["mise", "config", "ls", "--json"]), paths)
    records = [a, b, c]
    if c["kind"] == "answer":
        for path in c["paths"]:
            records.append(question("d", ["mise", "config", "get", "-f", path, "env._.path"]))
    return records


def capped(text):
    if len(text) <= STDERR_CAP:
        return text
    return "%s\n[truncated: %d more characters]" % (text[:STDERR_CAP], len(text) - STDERR_CAP)


def _prefixed(text):
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    return [PREFIX + line for line in lines]


def _answer_lines(rec, cwd):
    key = rec["key"]
    label = "(%s) %s" % (key, rec["label"])
    if rec["kind"] == "notasked":
        return [PREFIX + label, PREFIX + "  not asked: " + rec["why"]]
    if rec["kind"] == "missing":
        return [PREFIX + label, PREFIX + "  " + rec["reason"]]
    if rec["kind"] == "timeout":
        return [PREFIX + label, PREFIX + "  no answer within %s seconds (the question timed out)" % rec["timeout"]]
    lines = [PREFIX + label]
    if rec["kind"] == "answer" and key == "a":
        if not rec["items"]:
            lines.append(PREFIX + "  mise resolves no Python at " + cwd)
        for item in rec["items"]:
            lines.append(
                PREFIX
                + "  version=%(version)s source=%(source)s installed=%(installed)s install_path=%(install_path)s" % item
            )
    elif rec["kind"] == "answer" and key == "c":
        for path in rec["paths"]:
            lines.append(PREFIX + "  " + path)
    elif rec["kind"] == "answer":
        out = rec["out"].rstrip("\n")
        lines.append(PREFIX + "  " + (out if out else "(empty output)"))
    elif key in ("b", "d"):
        lines.append(PREFIX + "  exit status %d" % rec["rc"])
    else:
        # (a) or (c) failed or did not parse: item 6, with mise's stderr.
        if rec["kind"] == "exit":
            lines.append(PREFIX + "  exit status %d" % rec["rc"])
        else:
            lines.append(PREFIX + "  the output does not parse as a JSON array of objects")
        lines.append(PREFIX + "  mise stderr (verbatim, up to %d characters):" % STDERR_CAP)
        lines.extend(_prefixed(capped(rec["err"])))
    return lines


def refusal_text(pin, root, v, executable, cwd, warnings, records):
    """The refusal: items 1-6 of the umbrella body 4.2 in order, then GENERIC_BLOCK. Pure."""
    pin_s = "%d.%d.%d" % pin
    lines = [
        PREFIX + "REFUSED: the running interpreter is not the pin.",
        PREFIX + "pin: %s (%s)" % (pin_s, os.path.join(root, PIN_FILE)),
        PREFIX + "running: %d.%d.%d %s" % (v[0], v[1], v[2], v[3]),
        PREFIX + "sys.executable: %s" % executable,
        PREFIX + "cwd: %s" % cwd,
        PREFIX + "repository root: %s" % root,
    ]
    lines.extend(warnings)
    for rec in records:
        lines.extend(_answer_lines(rec, cwd))
    lines.append(GENERIC_BLOCK % {"pin": pin_s, "root": root})
    return "\n".join(lines) + "\n"


def _say(text):
    sys.stderr.write(text)
    sys.stderr.flush()


def _exit(code):
    """The last stderr line of every non-zero exit; returns `code`."""
    _say("%sexit %d\n" % (PREFIX, code))
    return code


def main():
    warnings = warning_lines(os.environ)
    root = repo_root()
    try:
        pin = read_pin(root)
    except PinFileError as e:
        _say("".join(w + "\n" for w in warnings))
        _say(
            "%spin file %s: %s\n%sthe pin file is exactly one line, python X.Y.Z\n"
            % (PREFIX, os.path.join(root, PIN_FILE), e, PREFIX)
        )
        return _exit(2)
    if admitted(sys.version_info, pin):
        _say("".join(w + "\n" for w in warnings))
        _say("%sadmitted %d.%d.%d\n" % (PREFIX, pin[0], pin[1], pin[2]))
        return 0
    cwd = os.getcwd()
    records = ask_mise(cwd, ORACLE_TIMEOUT_S)
    _say(refusal_text(pin, root, sys.version_info, sys.executable, cwd, warnings, records))
    return _exit(1)


def _guarded_main():
    """main(), except that an uncaught exception still ends stderr with the exit line (exit 3)."""
    try:
        return main()
    except SystemExit:
        raise
    except BaseException:
        import traceback

        traceback.print_exc()
        return _exit(3)


if __name__ == "__main__":
    sys.exit(_guarded_main())
