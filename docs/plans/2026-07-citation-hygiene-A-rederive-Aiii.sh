# Slice A-iii's part of the re-derivation harness (`…-Aiii-suite-scheduler.md`)
# — sourced by `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# `suites` is also cited by the umbrella, which is not a slice. A-iii's shared
# blocks -- `couplings`, `budget`, `lanes` -- are in `-common.sh`.

suites() {  # §1 / §4.3.1 / §4.3.3 — 47 tests, 4 files, and the fetch count
  # THREE ways this block used to certify a run it did not have. (1) The file
  # count was `ls … | wc -l`, which is `0` when the globs match nothing -- see
  # `_measure`. (2) The suite runner discarded `subprocess.run`'s `returncode`,
  # AND the output filter kept only `Ran `/`URLOPEN`/`OK`, so a failing suite
  # printed `Ran 35 tests in 0.03s | URLOPEN=0` -- the failure invisible as well
  # as non-fatal, and the URLOPEN figure the umbrella `:82` cites taken from a run
  # that did not complete. (3) The function then returned the status of the
  # SUCCESSFUL `git worktree remove`, so even a detected failure could not reach
  # `all`'s roster.
  local T rc=0 n
  T=$(mktemp -d)
  git worktree add -q "$T" "$MAIN" || { echo "!! cannot create the $MAIN worktree"; return 1; }
  _measure n ls "$T"/.claude/tools/_webref/test_*.py \
                "$T"/.claude/skills/elidex-plan-review/test_*.py || rc=1
  echo "$n"
  # §4.3.1 / §1 claim FOUR suite files at the base; the count was echoed and
  # never compared (the block-audit of 2026-08-22). The figure has a lifetime --
  # A-ii's `test_preflight.py` makes it five -- and when it moves, this line is
  # what says so, not a reader noticing the memo drifted.
  [ "$n" -eq 4 ] || { echo "!! $n suite files at $MAIN; A-iii §4.3.1 says 4 — the memo's figure has moved"; rc=1; }
  python3 - "$T" <<'SUITESPY' || rc=1
import re, subprocess, sys
spy = ("import sys, urllib.request\n_c=[]\n_o=urllib.request.urlopen\n"
       "urllib.request.urlopen=lambda r,*a,**k:(_c.append(getattr(r,'full_url',r)),_o(r,*a,**k))[1]\n"
       "import atexit; atexit.register(lambda: sys.stderr.write('URLOPEN=%d\\n'%len(_c)))\n")
t = sys.argv[1]
rc = 0
# `FAILED`/`ERROR:`/`FAIL:` are unittest's DIAGNOSTICS, and dropping them is half
# of why a red suite read green here. They are kept, and on a nonzero exit the
# child's whole stderr is replayed so the traceback survives too.
KEEP = ("Ran ", "URLOPEN", "OK", "FAILED", "ERROR:", "FAIL:")
for args in (["discover","-s",f"{t}/.claude/tools/_webref","-p","test_*.py","-t",f"{t}/.claude/tools"],
             ["discover","-s",f"{t}/.claude/skills/elidex-plan-review","-p","test_*.py"]):
    code = spy + "import unittest,sys;sys.argv=['x']+%r;unittest.main(module=None)" % args
    r = subprocess.run([sys.executable,"-c",code], capture_output=True, text=True)
    print(" | ".join(l for l in r.stderr.splitlines() if l.startswith(KEEP)))
    if r.returncode != 0:
        rc = 1
        print(f"!! SUITE FAILED (rc={r.returncode}) under {args[2]} -- the counts on the")
        print("!! line above are from a run that did not pass; they measure nothing.")
        sys.stdout.flush()          # or the replayed traceback lands above its own header
        sys.stderr.write(r.stderr)
    # The umbrella (`:82`) and §4.3.3 cite this block for ZERO `urlopen` calls at
    # the base; the `URLOPEN=` line was printed and never read back, so a suite
    # that started fetching read green (the block-audit of 2026-08-22).
    m = re.search(r"URLOPEN=(\d+)", r.stderr)
    if m is None or m.group(1) != "0":
        rc = 1
        print(f"!! URLOPEN={m.group(1) if m else 'unmeasured'} under {args[2]} -- the suites fetch;"
              " the 0-urlopen claim does not hold")
sys.exit(rc)
SUITESPY
  git worktree remove --force "$T"
  return "$rc"
}

suiteset() {  # §4.3.2 J4 — the set the uncollected-suite check must range over
  # `git ls-files` exits 0 for a pathspec that matches NOTHING, and this block IS
  # that census, so an empty set was indistinguishable from the real one -- and
  # the block's status was the trailing `echo`'s, i.e. always 0. J4 is a claim
  # about which suites EXIST; an empty answer is a broken glob, not a repo with
  # no tests.
  local n
  _measure n git ls-files '.claude/**/test_*.py' || return 1
  _measured
  [ "$n" -gt 0 ] || { echo "!! EMPTY SUITE SET — the pathspec matched no file; J4 has"
                      echo "!! nothing to range over, which is not the same as 'no uncollected suite'."
                      return 1; }
  echo "-- discover roots --"; echo ".claude/tools/_webref"; echo ".claude/skills/elidex-plan-review"
  # J4's claim is not only that the set is non-empty but that EVERY member is
  # under one of the two discover roots -- a suite outside both is the
  # uncollected suite J4 exists to catch, and the roots were echoed rather than
  # applied (the block-audit of 2026-08-22).
  local outside
  outside=$(printf '%s\n' "$_MEASURE_OUT" | grep -vE '^\.claude/(tools/_webref|skills/elidex-plan-review)/' || true)
  [ -z "$outside" ] || { echo "!! suite(s) outside both discover roots — J4's uncollected-suite case:"; printf '%s\n' "$outside" | sed 's/^/     /'; return 1; }
  # Under a root is not yet collected: `unittest discover` recurses only into
  # PACKAGE directories (every directory between the root and the file needs an
  # `__init__.py`; the Python 3.9+ discovery contract the floor admits), so a
  # suite in `<root>/fixtures/test_x.py` passed the prefix test above and was
  # still never run (Codex R9). The package chain is checked for every file.
  # The checker is a `_measure`d command: a python that cannot start is a
  # validation that never ran, and an empty capture from it read as "every
  # suite is collectable" (Codex R12, reproduced with python3 at exit 127).
  local files n_unc uncollected
  files=$_MEASURE_OUT
  _measure n_unc python3 -c '
import os, sys
ROOTS = (".claude/tools/_webref", ".claude/skills/elidex-plan-review")
for f in sys.argv[1].split():
    root = next(r for r in ROOTS if f.startswith(r + "/"))
    d = os.path.dirname(f)
    while d != root:
        if not os.path.isfile(os.path.join(d, "__init__.py")):
            print(f"{f}  (no __init__.py in {d})"); break
        d = os.path.dirname(d)' "$files" || return 1
  uncollected=$(_measured)
  [ -z "$uncollected" ] || { echo "!! suite(s) under a root but below a non-package directory — discover never reaches them:"; printf '%s\n' "$uncollected" | sed 's/^/     /'; return 1; }
  return 0
}

filters() {  # §4.3.2 — ci.yml's path filters at the base A-iii argues from
  # `git show | sed -n` under `pipefail` catches an unresolvable ref, but not the
  # other half: a sed range that matches nothing prints nothing and exits 0, so a
  # renamed `filters:` key reads as "this workflow has no filters" -- the claim
  # A-iii's §4.3.2 argues AGAINST, handed to it for free.
  local n body
  _measure n git show "$MAIN:.github/workflows/ci.yml" || return 1
  body=$(printf '%s' "$_MEASURE_OUT" | sed -n '/filters:/,/^  check:/p')
  [ -n "$body" ] || { echo "!! no \`filters:\` .. \`check:\` range in ci.yml at $MAIN ($n lines read);"
                      echo "!! an empty range is a renamed key, not an absent filter."
                      return 1; }
  printf '%s\n' "$body"
  # §4.1 makes four claims this block printed a range for and never checked (the
  # block-audit of 2026-08-22): `.claude/**` is in neither filter set; the three
  # validation jobs are gated on the filter; `trip-wires` is ungated; and the
  # workflow never invokes `mise`. The first is read off the range above, the
  # rest off the whole file, each named when it fails.
  local rc=0 ci
  ci=$_MEASURE_OUT
  printf '%s\n' "$body" | grep -q '\.claude' && { echo "!! .claude/** now appears in a filter set — §4.1's 'in neither' no longer holds"; rc=1; }
  local j
  for j in check doc deny; do
    printf '%s\n' "$ci" | awk -v j="$j" '$0 ~ "^  "j":$" {f=1; next} f && /^  [a-z-]+:$/ {exit} f && /^    needs: changes/ {ok=1} END {exit !ok}' \
      || { echo "!! job \`$j\` is not gated on \`changes\` — §4.1's 'all three jobs gated' no longer holds"; rc=1; }
  done
  # A missing job is not an ungated one: the claim is "present AND ungated",
  # and an awk that never saw the header exited 0 on the pre-#496 workflow
  # (Codex R12). Both halves are required.
  printf '%s\n' "$ci" | awk '$0 ~ "^  trip-wires:$" {f=1; next} f && /^  [a-z-]+:$/ {exit} f && /^    (needs|if):/ {g=1} END {exit (!f || g)}' \
    || { echo "!! \`trip-wires\` is absent or gated — §9's 'present and ungated since #496' no longer holds"; rc=1; }
  # "invokes" = a `run:` step naming the binary; the path filter's `mise.toml`
  # entry is a FILE, and a bare-word grep flagged it (caught on the first run).
  # A step's command is the WHOLE scalar, block-scalar bodies (`run: |` …)
  # included -- keeping only the `run:` key line missed every multi-line step
  # (Codex R10). The python below collects each step's full scalar.
  printf '%s\n' "$ci" | python3 -c '
import re, sys
lines = sys.stdin.read().splitlines()
cmds = []
i = 0
while i < len(lines):
    m = re.match(r"^(\s*)(-\s*)?run:\s*(.*)$", lines[i])
    if m:
        indent = len(m.group(1)) + (len(m.group(2)) if m.group(2) else 0)
        val = m.group(3).strip()
        # A block scalar is ANY value whose first character is the `|` or `>`
        # indicator -- a plain scalar cannot begin with either -- so the test is
        # the complement, not a list. The six-value list this replaced left
        # `|2`, `|2-`, `>2+` (indentation indicators, YAML §8.1.1) classified as
        # inline commands and their bodies skipped (Codex R14): the exemption
        # list left the next header form authoritative by default.
        if val[:1] in "|>":
            j = i + 1
            while j < len(lines) and (not lines[j].strip() or len(lines[j]) - len(lines[j].lstrip()) > indent):
                cmds.append(lines[j]); j += 1
            i = j; continue
        cmds.append(val)
    i += 1
hit = [c for c in cmds if re.search(r"(^|[^A-Za-z0-9_./-])mise(\s|$)", c)]
sys.exit(1 if hit else 0)' || { echo "!! ci.yml invokes mise in a run step — §4.1's 'never invokes mise' no longer holds"; rc=1; }
  return "$rc"
}

ruleset() {  # §13.2 — main's ruleset, READ rather than recalled
  # THREE `gh api` calls, and this block used to return the LAST one's status. So
  # when the first call -- the ONLY measurement of `enforcement` and `target`,
  # which is exactly what A-iii's memo cites this block for -- failed transiently,
  # the two detail calls could still succeed and the block certified a reading it
  # never took. `5abe729e` introduced `_measure` to make that unrepresentable and
  # swept the `git`-shaped and `subprocess.run`-shaped sites; `gh api` is the same
  # SHAPE and was missed because the sweep was scoped by TOOL.
  #
  # An empty `$id` is the same defect one step on: the detail URL is built by
  # CONCATENATION, so a name lookup that matched nothing sends `…/rulesets/`.
  # MEASURED rather than assumed -- an earlier revision of this very comment
  # asserted that degrades to the LISTING endpoint and certifies a ruleset list as
  # the detail read. It does not: `gh api repos/send/elidex/rulesets/` returns HTTP
  # 404, rc 1. The guard stays, and not because the block would otherwise pass --
  # whether an empty path segment 404s is the REMOTE's shape rather than this
  # harness's invariant, and `Not Found` reads as "no such ruleset" when the fact
  # is "the name lookup matched nothing". Name the cause here, before an empty
  # variable is interpolated into a URL.
  local failed=0 n_list n_id n_detail id
  echo "-- every ruleset on the repo (enforcement / target — what A-iii cites) --"
  _measure n_list gh api repos/send/elidex/rulesets \
    --jq '.[] | {id, name, enforcement, target}' || failed=1
  _measured
  _measure n_id gh api repos/send/elidex/rulesets \
    --jq '.[] | select(.name=="main-protection") | .id' || failed=1
  id=$(_measured | tr -d '[:space:]')
  if [ "$n_id" != 1 ] || [ -z "$id" ]; then
    echo "!! no single \`main-protection\` ruleset id (matching lines: $n_id) — the name"
    echo "!! lookup matched nothing. Not reading \`…/rulesets/\` with an empty segment:"
    echo "!! whatever the remote answers there is not a reading of main's ruleset."
    return 1
  fi
  echo "-- main-protection ($id), in detail --"
  _measure n_detail gh api "repos/send/elidex/rulesets/$id" --jq \
    '{enforcement, target, rules: [.rules[].type], pr: (.rules[]|select(.type=="pull_request").parameters.required_approving_review_count), bypass: [.bypass_actors[] | {actor_type, bypass_mode}]}' || failed=1
  _measured
  # Printing the detail is not checking it. The memo makes claims of this
  # ruleset in THREE places -- §4.5 (active; branch target; rule set exactly
  # `deletion` / `non_fast_forward` / `pull_request`, "exactly" because §4.5's
  # next sentence is "there is NO `required_status_checks` rule"; governs main)
  # and §10 Q1 / §11 (`required_approving_review_count: 0`, a `RepositoryRole`
  # bypass with `bypass_mode: always` -- the facts the deferral rests on). Three
  # rounds enumerated clauses one memo section at a time and each round found the
  # next section's claim unasserted (Codex R4/R5/R6), so the check is an EQUALITY
  # against one expected object, `_rulesetcheck`'s `$want`: every field the
  # projection carries is a claim, and a field the memo does not claim is not
  # projected. "Governs main" is NOT simulated from `conditions.ref_name` -- a
  # fifth round (Codex R10) found the simulation short of GitHub's own matching
  # (`fnmatch` patterns in `exclude`), and a bash re-implementation of GitHub's
  # ref-matching has no floor. GitHub answers the question itself:
  # `GET /rules/branches/main` evaluates every ruleset's include/exclude and
  # returns the rules that APPLY to `main`, each tagged with its `ruleset_id`;
  # the rules it attributes to this ruleset must be exactly the claimed set.
  # Every clause is READ, not recalled; a differing field is named; and both
  # checks are `_measure`d commands -- a `jq` that cannot run is a validation
  # that never happened (Codex R5). The `_measure` sweep was scoped by TOOL
  # twice (`git`, then `gh api`); the rule is SHAPE -- every command whose
  # status is a fact.
  local detail applied n_applied
  detail=$(_measured)
  _measure n_applied gh api repos/send/elidex/rules/branches/main \
    --jq "[.[] | select(.ruleset_id == $id) | .type]" || failed=1
  applied=$(_measured)
  echo "-- rules GitHub applies to \`main\` from this ruleset (/rules/branches/main) --"
  echo "$applied"
  _rulesetcheck "$detail" "$applied" || failed=1
  return "$failed"
}

_rulesetcheck() {  # $1 = projected detail JSON, $2 = rule types GitHub applies to main from it. 0 iff every memo claim holds.
  local n_bad bad
  _measure n_bad jq -rn --argjson d "$1" --argjson applied "$2" '
    { enforcement: "active", target: "branch",
      rules: ["deletion","non_fast_forward","pull_request"], pr: 0,
      bypass: [{actor_type: "RepositoryRole", bypass_mode: "always"}],
      applied_to_main: ["deletion","non_fast_forward","pull_request"] } as $want
    | ($d | { enforcement, target, rules: (.rules | sort), pr,
              bypass: (.bypass | sort_by(.actor_type, .bypass_mode)),
              applied_to_main: ($applied | sort) }) as $got
    | [ ($want | keys[]) as $k | select($got[$k] != $want[$k]) | "\($k)=\($got[$k]) (claimed \($want[$k]))" ]
    | join("; ")' || return 1
  bad=$(_measured)
  if [ -n "$bad" ]; then
    echo "!! main-protection does not certify the memo's claims (§4.5 / §10 Q1 / §11): $bad"
    return 1
  fi
  return 0
}
