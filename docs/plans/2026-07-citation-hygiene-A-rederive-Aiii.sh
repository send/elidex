# Slice A-iii's part of the re-derivation harness (`…-Aiii-suite-scheduler.md`)
# — sourced by `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# A-iii's shared blocks -- `suites` (cited by the umbrella too), `budget`,
# `lanes` -- are in `-common.sh`, per the seam rule: cited by more than one memo
# -> common.

suiteset() {  # §2 L2 / §4.2 — the set the scheduler ranges over, and why its roots do not recurse
  # `git ls-files` exits 0 for a pathspec that matches NOTHING, so the census is
  # checked for emptiness: an empty answer is a broken glob, not a repo with no
  # tests. `:(glob)` magic: without it a `/**/` still demands a directory
  # component (Codex R45).
  local n rc=0
  _measure n git ls-files ':(glob).claude/**/test_*.py' || return 1
  _measured
  [ "$n" -gt 0 ] || { echo "!! EMPTY SUITE SET — the pathspec matched no file"; return 1; }
  echo "-- derived roots (unique dirnames of the set; D2) --"
  _measured | sed 's#/[^/]*$##' | sort -u | sed 's/^/  /'
  # WHY EACH ROOT RUNS NON-RECURSIVELY, measured rather than asserted: `_webref`
  # is a package, so a recursive `discover` rooted at its parent collects
  # `_webref`'s suites under a second import name. The day a `test_*.py` lands
  # directly under `.claude/tools/` (PR #510's wrapper is the known candidate),
  # `.claude/tools` becomes a root -- and a recursive root there would run
  # `_webref`'s suites twice. This block fails if that hazard is NOT
  # reproducible, because then §4.2's argument for non-recursion has no subject.
  [ -f .claude/tools/_webref/__init__.py ] || { echo "!! _webref is not a package; §4.2's non-recursion argument has moved"; rc=1; }
  python3 - <<'PY' || rc=1
import unittest
s = unittest.defaultTestLoader.discover(".claude/tools", pattern="test_*.py", top_level_dir=".claude/tools")
def ids(x):
    for t in x:
        yield from (ids(t) if isinstance(t, unittest.TestSuite) else [t.id()])
dup = [i for i in ids(s) if i.startswith("_webref.")]
print(f"  recursive discover from .claude/tools collects {len(dup)} test(s) as `_webref.*`")
raise SystemExit(0 if dup else "!! the double-collection hazard did not reproduce")
PY
  return "$rc"
}

# `_gated JOB TEXT`: exit 0 iff JOB's region in TEXT carries both `needs:
# changes` and an `if:` on a `changes` output. Region = from `  JOB:` to the next
# line at two-space indent that is not blank/comment/list (the full job-key
# grammar, quoted and digit-bearing keys included).
# `_job_region JOB TEXT`: the lines of JOB's region (whole lines, block
# scalars included -- a region read, not a step parser). Shared by `_gated`
# and by A-iii §6 Q6, which reads the `tools` job's region for a `run:` step
# that executes the driver (a property of the step's argv, not a token grep).
# The header matches the key plain or quoted (`  check:` / `  "check":` /
# `  'check':`), the same key grammar the boundary rule admits (Codex R37).
_job_region() { printf '%s\n' "$2" | awk -v j="$1" -v q="'" '$0 ~ "^  [\"" q "]?" j "[\"" q "]?:$" {f=1; next} f && /^  [^ #-]/ {exit} f {print}'; }
# "Gated" = the COMPLETE approved condition, anchored: `needs.changes.outputs.<set>
# == 'true'` with at most the intentional push exception (`|| github.event_name ==
# 'push'`, which is how main gets every job). A prefix match certified
# `… == 'true' || always()` -- a job that runs on every PR -- as gated (Codex R38).
# Mapping keys plain or quoted (`needs:` / `"needs":` / `'if':`), the grammar
# the header already admits (Codex R39).
_gated() { _job_region "$1" "$2" | awk -v q="'" '$0 ~ ("^    [\"" q "]?needs[\"" q "]?: changes$") {n=1} $0 ~ ("^    [\"" q "]?if[\"" q "]?: needs\\.changes\\.outputs\\.(rust|config) == " q "true" q "( \\|\\| github\\.event_name == " q "push" q ")?$") {g=1} END {exit !(n && g)}'; }

filters() {  # §4.1 — ci.yml's path filters at the base A-iii argues from
  # `git show | sed -n` under `pipefail` catches an unresolvable ref, but not the
  # other half: a sed range that matches nothing prints nothing and exits 0, so a
  # renamed `filters:` key reads as "this workflow has no filters" -- the claim
  # A-iii's §4.1 argues AGAINST, handed to it for free.
  local n body
  _measure n git show "$MAIN:.github/workflows/ci.yml" || return 1
  # The filter census is the `filters:` mapping INSIDE the `changes` job: the
  # job region comes from `_job_region` (quoted headers admitted, next job ends
  # it -- no `sed` range keyed on unquoted `filters:`/`check:`, Codex R46), and
  # the `filters` key inside it is matched plain or quoted.
  body=$(_job_region changes "$_MEASURE_OUT" | awk -v q="'" '$0 ~ ("^ *[\"" q "]?filters[\"" q "]?:") {f=1} f')
  [ -n "$body" ] || { echo "!! no \`filters:\` mapping in the \`changes\` job of ci.yml at $MAIN ($n lines read);"
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
    # "Gated" is the `if:` on a `changes` OUTPUT, not the `needs:` edge: `needs`
    # only orders the always-green filter job, so a job that kept `needs:
    # changes` and lost its `if:` runs on every PR while this read said gated
    # (Codex R18). Both lines are required.
    # A job REGION ends at the next key of the `jobs:` mapping = any line at
    # exactly two-space indent that is neither blank, a comment, nor a list
    # item -- not only an unquoted `[a-z-]+` key. The narrow pattern let
    # `check` borrow the gates of a following `helper_2:` / `"quoted":` job
    # (Codex R32); the negative control below pins that shape.
    _gated "$j" "$ci" \
      || { echo "!! job \`$j\` lacks \`needs: changes\` or an \`if:\` on a changes output — §4.1's 'all three jobs gated' no longer holds"; rc=1; }
  done
  # Negative control for the boundary rule: `check` with no gates, followed by
  # a gated job whose key the old regex did not recognise. Must NOT be gated.
  if _gated check "$(printf 'jobs:\n  check:\n    runs-on: x\n  helper_2:\n    needs: changes\n    if: needs.changes.outputs.rust == '"'"'true'"'"'\n')"; then
    echo "!! boundary control: an ungated \`check\` borrowed \`helper_2\`'s gates"; rc=1; fi
  if _gated check "$(printf 'jobs:\n  check:\n    needs: changes\n    if: needs.changes.outputs.rust == '"'"'true'"'"'\n  \"doc\":\n    runs-on: x\n')"; then :; else
    echo "!! boundary control: a gated \`check\` read as ungated"; rc=1; fi
  # Condition controls: the push exception is gated; an `|| always()` tail is not.
  if _gated check "$(printf 'jobs:\n  check:\n    needs: changes\n    if: needs.changes.outputs.rust == '"'"'true'"'"' || github.event_name == '"'"'push'"'"'\n')"; then :; else
    echo "!! condition control: the approved push exception read as ungated"; rc=1; fi
  if _gated check "$(printf 'jobs:\n  check:\n    needs: changes\n    if: needs.changes.outputs.rust == '"'"'true'"'"' || always()\n')"; then
    echo "!! condition control: an \`|| always()\` job read as gated"; rc=1; fi
  # A missing job is not an ungated one: the claim is "present AND ungated",
  # and an awk that never saw the header exited 0 on the pre-#496 workflow
  # (Codex R12). Both halves are required.
  # Same region reader as the gated jobs (quoted header admitted); "present
  # AND ungated" = a non-empty region with no `needs:`/`if:` line.
  { r=$(_job_region trip-wires "$ci"); [ -n "$r" ] && ! printf '%s\n' "$r" | grep -qE "^    [\"']?(needs|if)[\"']?:"; } \
    || { echo "!! \`trip-wires\` is absent or gated — §4.1's 'present and ungated' (#496) no longer holds"; rc=1; }
  # "Invokes mise" was read by a hand-rolled YAML step reader for four rounds
  # (R10 block scalars, R14 indentation indicators, R26 flow mappings, R27 the
  # grammar statement) and a shlex tokenizer for three (R14/R15/R16); each round
  # widened it by one shape and the memo's claim narrowed to "as far as this
  # reader parses YAML". The claim is now the COMPLEMENT, measured over the
  # WHOLE file with no parser: every `mise` token in ci.yml is the path-filter
  # file entry `mise.toml`. Any other position -- a run step, `uses:`/`with:`
  # arguments, env -- is RED (a false RED makes a reader look; a false GREEN
  # certifies a claim). A YAML comment line (`^\s*#`) is skipped: a comment
  # can invoke nothing, and ci.yml:151 at origin/main names `mise run` in one
  # (measured on the first run). What a static read cannot see is an
  # invocation through a variable (`"$MISE_BIN"`) or built at runtime; that is
  # the block's stated limit, A-iii §4.1. Quoted heredoc: no apostrophe hazard.
  CI_YML="$ci" python3 - <<'PY' || { echo "!! ci.yml carries a \`mise\` token that is not the \`mise.toml\` filter entry — §4.1's reading no longer holds"; rc=1; }
import os, re, sys
text = os.environ["CI_YML"]
hits = [(n, l.strip()) for n, l in enumerate(text.splitlines(), 1)
        if not l.lstrip().startswith("#") and re.search(r"\bmise\b(?!\.toml\b)", l)]
for n, l in hits:
    print(f"  ci.yml:{n}: {l[:100]}")
sys.exit(1 if hits else 0)
PY
  return "$rc"
}

ruleset() {  # §4.5 — main's ruleset, READ rather than recalled
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


touchset() {  # §7 — A-iii's diff stays inside its edit set
  # At plan stage the diff is this memo and its harness; after implementation it
  # is also the three files §4.2 names. Anything else -- a `_webref` file, a
  # `test_*.py` (the #510 wrapper is #510's to write, §4.2) -- is RED. Committed
  # range and working tree both.
  local n m rc=0 out
  _measure n git diff --name-only "$MAIN"...HEAD || return 1
  out=$_MEASURE_OUT
  _measure m git status --porcelain --untracked-files=all || return 1
  out=$(printf '%s\n%s\n' "$out" "$(_measured | sed 's/^...//')" | sed '/^$/d' | sort -u)
  printf '%s\n' "$out" | sed 's/^/  /'
  local bad
  bad=$(printf '%s\n' "$out" | grep -vE '^(docs/plans/2026-07-citation-hygiene-(Aiii-suite-scheduler\.md|A-rederive(-[A-Za-z]+)?\.sh)|scripts/python-suites\.sh|mise\.toml|\.github/workflows/ci\.yml)$' || true)
  [ -z "$bad" ] || { echo "!! outside A-iii's edit set (§4.2 / §7):"; printf '%s\n' "$bad" | sed 's/^/     /'; rc=1; }
  return "$rc"
}
