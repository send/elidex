#!/usr/bin/env python3
"""The two checks that decide a claim by RUNNING something, split out of
`claim-gate-plan-check.py` at 1351 lines (CLAUDE.md's touch-time split
discipline, which fires at 1000 and is not deferrable).

The seam is not line count. Everything else in the entry point decides a
question by reading the document: does this §-reference resolve, is this table
exhaustive, does this `path:LINE` exist. These two answer a question the
document cannot: what does the command actually print, and what do those two
programs actually do. FENCE runs the repository's two fence trackers and
compares their behaviour rather than either one's prose; TRANSCRIPT re-runs
every command a memo pastes and compares the answer, which is the check round
2's CRIT lived in. They carry their own machinery for it -- a deny-list of the
shapes a reader-*named* program uses to write, a fenced-block scanner, an arrow
comparator -- and none of it is used by anything else in the entry point.

`host` is the entry point, bound by it at import time, exactly as
`claim_gate_selftest` is. `bad` is resolved through it on every call and never
imported by name: the probe file REBINDS `host.FAILS` for the duration of one
check, so a name bound once here would append to the list that was swapped out,
`capture()` would return empty, and every negative probe would silently invert.
"""
from __future__ import annotations

import importlib.util
import re
import subprocess
from types import ModuleType

host: ModuleType  # bound by the entry point at import time
# Annotations below spell the memo type `host.Memo`, not `Memo`: the class
# lives in a file whose name has hyphens and cannot be imported, so a bare
# `Memo` here resolves to nothing for any tool that evaluates annotations.
# `from __future__ import annotations` makes either spelling harmless at
# runtime; this one is the true one.


# --------------------------------------------------------------------------
# 7. FENCE -- the "A differs from B" claim, decided by running A and B
#
# Round 2's CRIT 1: the memo said preflight.py marks a mismatched fence marker
# fenced while grep_pass.py treats it as content, read off `grep_pass.py`'s
# `— content` comment (which means content *of the still-open block*). That
# premise carried the §6 bound accepting a third implementation and all of §7
# Q3. This check does not read either comment.
# --------------------------------------------------------------------------

CORPUS = "\n".join([
    "plain",
    "```",
    "inside a backtick block",
    "~~~",                       # mismatched marker: the disputed case
    "still inside",
    "```",
    "out",
    "~~~",
    "inside a tilde block",
    "```",                       # mismatched the other way
    "still inside",
    "~~~",
    "tail",
    "```",                       # unclosed at EOF
    "runs to the end",
])


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, host.SKILL / f"{name}.py")
    assert spec and spec.loader, f"cannot locate {name}.py under {host.SKILL}"
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def fence_verdict(spans_fn=None) -> tuple[list[int], int]:
    """(lines where the two in-repo trackers disagree, corpus size).

    `spans_fn` overrides the grep_pass side; the self-test feeds it a
    deliberately wrong tracker, because a comparison that structurally cannot
    report a divergence is worth exactly as much as the comment it replaced."""
    pf, gp = _load("preflight"), _load("grep_pass")
    lines = CORPUS.split("\n")
    a = pf._fence_state_array(lines)

    spans = (spans_fn or gp._code_block_spans)(CORPUS)
    b = [False] * len(lines)
    pos = 0
    for i, line in enumerate(lines):
        start, end = pos, pos + len(line)
        b[i] = any(s <= start and end <= e for s, e in spans)
        pos = end + 1
    return [i + 1 for i in range(len(lines)) if a[i] != b[i]], len(lines)


# Any memo line that puts the two trackers and a sameness/difference word
# together is *restating* this verdict. Polarity is deliberately not parsed:
# "they differ" and "they do not differ" are equally forbidden, because a
# checker that reads English polarity is the same unexecuted-claim medium one
# level down. The rule is D10 form 2 applied to a boolean -- a fact the memo's
# own edits can invalidate is stated by reference, not restated.
RESTATES_FENCE = re.compile(
    r"(differ|disagree|divergen|diverge|agree|identical|the same|in step)", re.I)
_A = r"(?:preflight\.py|_fence_state_array)"
_B = r"(?:grep_pass\.py|_code_block_spans|_fenced_spans)"
NAMES_BOTH = re.compile(rf"{_A}.*{_B}|{_B}.*{_A}")
# The load-bearing false claim was spelled "the existing two already differ" --
# it named neither module, so a checker keyed on the two file names would have
# read clean over the very sentence that carried the §6 bound. Match the
# concept, not the spelling (memory
# `feedback_semantic-sibling-selfseed-and-regate-breadth`).
FENCE_TOPIC = re.compile(r"fence", re.I)
PAIR_REF = re.compile(r"\b(the two|both|each other|the existing two|"
                      r"one another|two implementations|two trackers|"
                      r"the other (?:one|implementation|tracker))\b", re.I)


# The ban is SENTENCE-scoped, not line-scoped. It shipped keyed on one physical
# line, and the memo it guards wraps at ~100 columns: the load-bearing sentence,
# "the tier pass is a third fence-tracking implementation, and the existing two
# already differ", escapes a line-scoped ban whenever the wrap falls between "the
# existing two" and "differ" -- exactly the restatement the ban exists to forbid,
# read clean by the checker written to catch it.
#
# The first fix widened the window to the whole paragraph, and running it over
# the memo produced three findings, all false: markdown puts no blank line
# between adjacent table rows or between adjacent bullets, so a paragraph window
# fused §2's I1×I2 row with its I1×I4 row, and fused one bullet's "inside a
# fenced block" with a LATER bullet's sameness word. Widening past the unit the
# claim is actually made in buys the false-positive rate the FENCE_TOPIC +
# PAIR_REF proximity heuristic was designed to avoid. So the window is the
# sentence: wide enough to span a wrap, which is the whole defect, and no wider.
# A blank line still ends it, and so does a structural line -- a table cell and a
# heading cannot wrap, so joining them to a neighbour can only ever fuse.
STRUCTURAL = re.compile(r"^[ \t]*(\||#{1,6}[ \t]|```|~~~)")
# Closing punctuation may follow the stop: the memo's "…is gone** (§7 Q3, §4.2)."
# ends a sentence at `).`, and a bare `(?<=[.!?])\s` boundary does not see it --
# which left the retracted-premise narration on the next line fused to the
# "fence tracker" heading phrase of the sentence before it.
SENT_END = re.compile(r"[.!?][)\]\"'`*_]*\s+")


def windows(lines: list[str]) -> list[tuple[str, list[tuple[int, int]], int]]:
    """(sentence text, [(offset, 1-based line no)], sentence offset).

    Lines are stripped and joined with one space, so a `.*` in NAMES_BOTH spans
    a wrap; the offset table is what turns a match position back into the line
    the offending span STARTS on, which is the line a reader has to edit."""
    out: list[tuple[str, list[tuple[int, int]], int]] = []
    para: list[tuple[int, str]] = []

    def flush() -> None:
        if not para:
            return
        text = " ".join(t for _, t in para)
        idx, pos = [], 0
        for n, t in para:
            idx.append((pos, n))
            pos += len(t) + 1
        start = 0
        for end in [m.end() for m in SENT_END.finditer(text)] + [len(text)]:
            if end > start:
                out.append((text[start:end], idx, start))
            start = end
        para.clear()

    for i, line in enumerate(lines, 1):
        s = line.strip()
        if not s:
            flush()
        elif STRUCTURAL.match(line):
            flush()
            para.append((i, s))
            flush()
        else:
            para.append((i, s))
    flush()
    return out


def line_at(idx: list[tuple[int, int]], off: int) -> int:
    ln = idx[0][1]
    for start, n in idx:
        if start > off:
            break
        ln = n
    return ln


def check_fence(memo: host.Memo) -> None:
    try:
        diverge, n = fence_verdict()
    except Exception as e:  # pragma: no cover
        host.bad("FENCE", f"cannot run the two in-repo fence trackers: {e}")
        return
    verdict = (f"disagree on line(s) {diverge} of {n}" if diverge
               else f"agree on all {n}")
    for text, idx, base in windows(memo.lines):
        r = RESTATES_FENCE.search(text)
        if not r:
            continue
        nb = NAMES_BOTH.search(text)
        ft, pr = FENCE_TOPIC.search(text), PAIR_REF.search(text)
        if nb:
            spans = [r.span(), nb.span()]
        elif ft and pr:
            spans = [r.span(), ft.span(), pr.span()]
        else:
            continue
        lo, hi = min(s for s, _ in spans), max(e for _, e in spans)
        first, last = line_at(idx, base + lo), line_at(idx, base + hi - 1)
        # ...and the matches must sit within a WRAP of each other. The defect is
        # exactly one line's worth of widening: a restatement is a local
        # co-occurrence, and stretching the co-occurrence over a whole long
        # sentence is what produced the last false positive on the memo -- §7's
        # compliant citation ("the `FENCE` check now measures that relation by
        # running both …") picked up `the same` four lines later, out of "the
        # same unexecuted-claim medium one level down", a clause about something
        # else. Two physical lines is the span a ~100-column wrap can hide a
        # claim in, and it is the span this ban reads.
        if last - first > 1:
            continue
        host.bad("FENCE", f"{memo.path.name}:{first} restates how "
                     f"the two fence trackers relate; that fact is measured by "
                     f"this check (they {verdict} lines) and must be cited, "
                     f"not written out: {text[lo:lo + 64]}")


# --------------------------------------------------------------------------
# 8. TRANSCRIPT -- every pasted command is re-run and its pasted answer checked
#
# The memo's provenance contract says every figure "came from a command run in
# this worktree and pasted next to it". That makes the paste a claim, and D10
# says a claim the memo's own landing can invalidate has to be re-executable
# rather than asserted. Round 2 shipped two prose sites missing from a pasted
# `git grep` edit-set listing; re-running the listing is what finds that, and
# no amount of re-reading it was going to.
#
# Only these program names run, and only when the line carries no shell state
# the memo does not also paste. Lines that cannot be re-run are REPORTED as
# unchecked rather than skipped -- a silent cap reads as coverage.
# --------------------------------------------------------------------------

RUNNABLE = re.compile(r"^(git (?:grep|ls-files|show|log)|grep|sed|awk|wc|tail|head)\b")
# `$NF` inside an awk program is not shell state, and a first cut that tested
# the raw line skipped three re-runnable commands as unrunnable -- a cap that
# reads as coverage. Single-quoted spans are removed before the test. `$?` is
# self-contained within one `bash -c`, so it is not state either.
SQUOTED = re.compile(r"'[^']*'")
NEEDS_STATE = re.compile(r"\$[A-Za-z_{#*@]|set --|<prototype|<this memo|<the umbrella")


# This tool EXECUTES what it admits, and RUNNABLE admits by the program's name.
# Round 6 tried replacing that with an analysis of what the script does; the
# analysis leaked (nested `$(...)`, `<(...)`, `&`, `--output`) and delivered no
# coverage, so it was reverted -- and the review that found the leaks also found
# that the SAME hole had always been open on this arm: `sed -i`, an `awk` program
# calling `system()`, `git log --output=`, `git grep -O<cmd>` and anything after
# `&` or `;` were being run, unquoted, from a document.
#
# A deny-list, not an allow-list, and deliberately so: the allow-list attempt is
# what leaked. These are the shapes a READER-named program uses to write. A
# command matching one is reported unrunnable, with its reason, like every other
# entry in the un-run list -- not silently skipped.
WRITES = re.compile(
    r"(^|\s)sed\b[^|;&]*\s-[a-zA-Z]*i|"          # sed -i, in any flag cluster
    r"system\s*\(|"                               # awk's system()
    r"\bprint(f)?\b[^|;&]*>|"                     # awk print > "file"
    r"(^|\s)-O|--output\b|"                       # git grep -O<cmd>, git log --output
    r"(^|\s)-o\s|"                                # sort -o
    r"<\(|>\(|"                                   # process substitution
    r"[^&|]&(\s|$)|"                              # backgrounding (not && )
    r">>?|"                                       # any redirection
    r"(^|\s)(rm|mv|cp|touch|mkdir|curl|wget|chmod|eval|find)\b"
)


# `sed` is the one reader-named program whose SCRIPT is a language with write
# commands (`w`, `W`, `r`, `R`, `e`, and `s///w`), and they sit inside the quoted
# span WRITES cannot safely read. Both memos use exactly one shape -- a line
# printer -- so that shape is whitelisted and every other `sed` is reported.
SED_PRINTER = re.compile(r"^sed -n '[0-9;p, ]+' \S+$")


def needs_state(cmd: str) -> bool:
    if WRITES.search(cmd):
        return True
    if cmd.startswith("sed") and not SED_PRINTER.match(cmd):
        return True
    return bool(NEEDS_STATE.search(SQUOTED.sub("''", cmd)))
FILELINE = re.compile(r"^([\w./-]+):(\d+):")
ARROW = re.compile(r"^(.*?)[ \t]+(?:→|->)[ \t]+(.*)$")


QUOTE = re.compile(r"^[ \t]*>[ \t]?")


# The subject here is the MEMO's own fenced blocks (`check_transcripts`), which
# is a different question from an annotation's tier. §4.2 has one inertness rule
# -- `k` backticks before the `<!--`, identical on all four surfaces -- and the
# fence arm this checker used to carry for it, with its per-surface table and
# its `.rs` rustdoc strip, is what §7 Q3 option (c) deleted. The wire is the
# single site for that rule now. (This paragraph opened by naming a `FENCE_MARK`
# constant that no longer exists, and trailed off mid-sentence: it was spliced
# by the edit that deleted the pattern and not read afterwards.)
def fenced_blocks(memo: host.Memo) -> list[tuple[int, list[str]]]:
    """(1-based line of the opener, body lines) for every fenced block.

    A leading `> ` is stripped first: the memo's own status header is a
    blockquote, and its two artifact commands -- the ones the header offers as
    the proof that the memo is machine-checked -- sat outside every block
    because a fence pattern anchored at the margin does not match `> ``` `.
    That strip is the only thing this needs that the repository's tracker does
    not do, so it is the only thing done here.

    The block boundaries come from `preflight._fence_state_array`, the tracker
    FENCE compares against, rather than from a pattern of this module's own.
    There was one: a `FENCE_MARK` toggle that flipped on ANY marker, so a `~~~`
    closed a ```-block -- which is, line for line, the mutant the self-test
    plants as `wrong_spans` to prove the FENCE comparison can detect a bad
    tracker. A third fence implementation is what §7 Q3 (c) deleted, and this
    one decided which pasted commands get re-run and which are silently not,
    inside the program whose FENCE check exists because of exactly that. It
    also dropped an unclosed trailing block with no finding and no un-run
    entry; the shared tracker fences it to EOF."""
    pf = _load("preflight")
    lines = [QUOTE.sub("", ln) for ln in memo.lines]
    state = pf._fence_state_array(lines)
    out: list[tuple[int, list[str]]] = []
    i = 0
    while i < len(lines):
        if not state[i]:
            i += 1
            continue
        j = i
        while j + 1 < len(lines) and state[j + 1]:
            j += 1
        # The run spans opener..closer inclusive. Drop the opener always, and
        # the closer when there is one -- an unclosed block runs to EOF and its
        # last line is body.
        body = lines[i + 1:j] if pf.FENCE_RE.match(lines[j]) and j > i else lines[i + 1:j + 1]
        out.append((i + 1, body))
        i = j + 1
    return out


def run_line(cmd: str) -> str:
    p = subprocess.run(["bash", "-c", cmd], cwd=host.REPO, capture_output=True, text=True)
    return (p.stdout or p.stderr).strip()


def compare_arrow(memo: host.Memo, ln: int, cmd: str, want: str) -> None:
    """One `<cmd> → <answer>` line.

    An empty re-run is a MISMATCH unless the paste is empty too. The first cut
    tested `first not in head`, which is vacuously true when the command prints
    nothing -- so any pasted answer survived the moment its command stopped
    producing output, which is the drift direction the whole program exists to
    catch. `awk '{s+=$NF} END{print s}'` over no input prints an empty line, not
    `0`, and a pasted `→ 0` sailed through on exactly that."""
    got = run_line(cmd)
    head = re.split(r"\s{2,}|\s+[#(]", want)[0].strip()
    if not head:
        return
    first = got.split("\n")[0].strip()
    if not first:
        host.bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes `{cmd[:56]}` → {head!r}; "
                          f"re-running prints nothing at all")
        return
    # `first not in head` was the acceptance for an abbreviated paste -- a `→ 3
    # files` gloss over an output of `3`. Containment is the wrong relation for
    # it: `13` contains `3`, so a stale count passed silently against a true
    # one, in the comparator round 2's CRIT lived in. The gloss is always a
    # SUFFIX, so the test is a prefix test.
    if head not in (first, got) and not head.startswith(first):
        host.bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes `{cmd[:56]}` "
                          f"→ {head!r}; re-running gives {first!r}")


# A block that builds its own repository is a FIXTURE transcript: its commands
# describe a tree this one is not, so re-running them here compares against the
# wrong world. Recognised by the construction the memo itself specifies for a
# hermetic fixture, and reported as unchecked rather than silently compared --
# a `git grep` inside such a block ran against the real repo and "reproduced"
# nothing, which read as a stale paste.
FIXTURE_BLOCK = re.compile(r"^\s*(?:\S+=\$\()?(?:mktemp|git init)\b", re.M)


# `RUNNABLE` gates the FIRST WORD. Everything after a separator is a command
# position too, and nothing gated it: a review executed `| tee`, `; python3 -c`,
# `&& python3 -c`, `$(python3 -c ...)` and a backtick form out of a memo, writing
# six files into the repo root -- three of them with ZERO findings, because the
# paste still matched and the run went silent. The comment above claims this hole
# is closed; `&` has an arm and `;` never did, and `$(...)` is named there as
# something the reverted allow-list leaked without anything replacing it.
#
# So every segment is gated, not the first. Substitution is REFUSED rather than
# gated: round 6 tried to reason about what a substitution does and leaked
# (`printf "$(rm -rf /)"`), and refusing is a reported un-run entry, not silence.
SEPARATOR = re.compile(r"\|\||&&|[|;]")
SUBSTITUTION = re.compile(r"\$\(|`|<\(|>\(")


def unrunnable(cmd: str) -> str | None:
    """Why this checker will not re-run `cmd`, or None if it will.

    One site. This decision -- which strings out of a document get handed to
    `bash -c` -- was spelled twice, once per pasted form, with the two reason
    strings duplicated verbatim and the second copy re-deriving the answer from
    an inverted second `RUNNABLE.match`. Only one of the two copies had a probe,
    so widening either predicate could pass its own proof while the other branch
    diverged."""
    bare = SQUOTED.sub("''", cmd)
    if SUBSTITUTION.search(bare):
        return ("contains a command substitution, a command position this "
                "checker will not gate")
    for seg in SEPARATOR.split(bare):
        if not seg.strip():
            return "has an empty command segment"
        if not RUNNABLE.match(seg.strip()):
            return "not one of the programs this checker runs"
    if WRITES.search(cmd) or (cmd.startswith("sed") and not SED_PRINTER.match(cmd)):
        return "uses a shape a reader-named program writes with"
    if needs_state(cmd):
        return "needs shell state the memo does not also paste"
    return None


def check_transcripts(memo: host.Memo) -> list[str]:
    """Findings go through `host.bad`; the lines this could not re-run are RETURNED.

    Returned rather than printed from here so a probe can read it: the arrow
    form's hole below shipped precisely because nothing could assert what this
    list does and does not contain. (This sentence used to hand that reason off
    to `verdict_over`, a function deleted with `claim_gate_grammar.py` — a
    pointer whose target is gone reads as a cross-reference and is a dead end.)"""
    unchecked: list[str] = []
    for start, body in fenced_blocks(memo):
        if FIXTURE_BLOCK.search("\n".join(body)):
            unchecked.append(f"{memo.path.name}:{start} (fixture block — builds "
                             f"its own repo, so this tree is not its subject)")
            continue
        i = 0
        while i < len(body):
            ln = start + 1 + i
            # A `$ ` prompt and an ` → ` answer can appear on the SAME line, and
            # the first cut let the `$ `-form branch swallow the arrow into the
            # command string -- silently dropping the wire's five execution-rule
            # transcripts, the evidence for I3/I5 and D4/D6. Strip the prompt
            # first, then decide the form.
            line = body[i].strip()
            prompt = line.startswith("$ ")
            if prompt:
                line = line[2:].strip()

            # An arrow line whose command is not RUNNABLE used to match neither
            # this branch nor the `$`-form's, so it fell through to `i += 1` and
            # vanished: no comparison, no `unchecked` entry, and the run read as
            # coverage -- the exact opposite of what this check's own header
            # states ("a silent cap reads as coverage"), with the two forms
            # holding opposite policies for one shape. §4.1.1's whole
            # transcript is of that shape (`git clone --local`, and three
            # `git -C "$fixture" …` lines that put an option before the
            # subcommand), and all four were invisible. The remedy is the report,
            # NOT a wider RUNNABLE: cloning a repository to audit a memo is a
            # side effect this tool must not have.
            #
            # A `$ ` prompt still goes to the branch below even when it carries
            # an arrow. That branch owns the output lines beneath the prompt and
            # already reports its own non-runnable commands; taking them here
            # would leave those output lines to be re-read as commands.
            m = ARROW.match(line)
            if m and (not prompt or RUNNABLE.match(m.group(1).strip())):
                cmd, want = m.group(1).strip(), m.group(2).strip()
                if (why := unrunnable(cmd)):
                    unchecked.append(f"{memo.path.name}:{ln} {cmd[:60]} ({why})")
                else:
                    compare_arrow(memo, ln, cmd, want)
                i += 1
                continue

            if prompt:
                cmd = line
                j = i + 1
                while cmd.endswith("\\") and j < len(body):
                    cmd = cmd[:-1] + " " + body[j].strip()
                    j += 1
                out_lines = []
                while j < len(body) and not body[j].strip().startswith("$ "):
                    out_lines.append(body[j])
                    j += 1
                pasted = {m2.group(0) for l2 in out_lines if (m2 := FILELINE.match(l2.strip()))}
                if (why := unrunnable(cmd)):
                    unchecked.append(f"{memo.path.name}:{ln} {cmd[:60]} ({why})")
                elif pasted:
                    actual = {m2.group(0) for l2 in run_line(cmd).split("\n")
                              if (m2 := FILELINE.match(l2.strip()))}
                    for miss in sorted(actual - pasted):
                        host.bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes the output of "
                                          f"`{cmd[:56]}` but omits the hit at {miss}")
                    for extra in sorted(pasted - actual):
                        host.bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes a hit at {extra} "
                                          f"that `{cmd[:56]}` no longer produces")
                else:
                    # output that is not a `path:NN:` listing was compared against
                    # an empty set, i.e. passed unconditionally. Say so.
                    body_txt = "\n".join(x for x in out_lines if x.strip())
                    if body_txt.strip():
                        got = run_line(cmd).strip()
                        if got != body_txt.strip():
                            host.bad("TRANSCRIPT", f"{memo.path.name}:{ln} pastes output for "
                                              f"`{cmd[:56]}` that re-running does not "
                                              f"reproduce ({got.splitlines()[:1]} vs "
                                              f"{body_txt.splitlines()[:1]})")
                i = j
                continue
            i += 1
    return unchecked


def report_transcripts(unchecked: list[str]) -> None:
    """The precedent this file's non-coverage accounting follows.

    The header used to name ONE reason for the whole list -- "they need shell
    state the memo does not also paste" -- which was already false of the
    fixture-block entries beneath it, and would have been false of every entry
    the arrow-form fix adds. The reason moved onto each line, where it is true
    of that line."""
    if unchecked:
        print(f"  transcripts: {len(unchecked)} pasted command(s) not re-run:")
        for u in unchecked:
            print(f"    {u}")
