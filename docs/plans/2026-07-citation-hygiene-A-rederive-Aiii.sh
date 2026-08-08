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
  python3 - "$T" <<'SUITESPY' || rc=1
import subprocess, sys
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
  return 0
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
    '{rules: [.rules[].type], pr: (.rules[]|select(.type=="pull_request").parameters.required_approving_review_count), bypass: [.bypass_actors[].actor_type], mode: [.bypass_actors[].bypass_mode]}' || failed=1
  _measured
  return "$failed"
}
