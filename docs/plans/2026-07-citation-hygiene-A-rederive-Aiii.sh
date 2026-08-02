# Slice A-iii's part of the re-derivation harness (`…-Aiii-suite-scheduler.md`)
# — sourced by `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# `suites` is also cited by the umbrella, which is not a slice. A-iii's shared
# blocks -- `couplings`, `budget`, `lanes` -- are in `-common.sh`.

suites() {  # §1 / §4.3.1 / §4.3.3 — 47 tests, 4 files, and the fetch count
  local T; T=$(mktemp -d); git worktree add -q "$T" "$MAIN"
  ls "$T"/.claude/tools/_webref/test_*.py "$T"/.claude/skills/elidex-plan-review/test_*.py | wc -l
  python3 - "$T" <<'PY'
import subprocess, sys
spy = ("import sys, urllib.request\n_c=[]\n_o=urllib.request.urlopen\n"
       "urllib.request.urlopen=lambda r,*a,**k:(_c.append(getattr(r,'full_url',r)),_o(r,*a,**k))[1]\n"
       "import atexit; atexit.register(lambda: sys.stderr.write('URLOPEN=%d\\n'%len(_c)))\n")
t = sys.argv[1]
for args in (["discover","-s",f"{t}/.claude/tools/_webref","-p","test_*.py","-t",f"{t}/.claude/tools"],
             ["discover","-s",f"{t}/.claude/skills/elidex-plan-review","-p","test_*.py"]):
    code = spy + "import unittest,sys;sys.argv=['x']+%r;unittest.main(module=None)" % args
    r = subprocess.run([sys.executable,"-c",code], capture_output=True, text=True)
    print(" | ".join(l for l in r.stderr.splitlines() if l.startswith(("Ran ","URLOPEN","OK"))))
PY
  git worktree remove --force "$T"
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
