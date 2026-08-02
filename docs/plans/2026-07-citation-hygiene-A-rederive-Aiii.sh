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
  git ls-files '.claude/**/test_*.py'
  echo "-- discover roots --"; echo ".claude/tools/_webref"; echo ".claude/skills/elidex-plan-review"
}

filters() { git show "$MAIN:.github/workflows/ci.yml" | sed -n '/filters:/,/^  check:/p'; }

ruleset() {
  gh api repos/send/elidex/rulesets --jq '.[] | {id, name, enforcement, target}'
  local id; id=$(gh api repos/send/elidex/rulesets --jq '.[] | select(.name=="main-protection") | .id')
  gh api "repos/send/elidex/rulesets/$id" --jq \
    '{rules: [.rules[].type], pr: (.rules[]|select(.type=="pull_request").parameters.required_approving_review_count), bypass: [.bypass_actors[].actor_type], mode: [.bypass_actors[].bypass_mode]}'
}
