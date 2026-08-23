# Slice A-iii's part of the re-derivation harness (`…-Aiii-suite-scheduler.md`)
# — sourced by `2026-07-citation-hygiene-A-rederive.sh`, the only entry point.
#
# A-iii's shared blocks -- `suites` (cited by the umbrella too), `couplings`,
# `budget`, `lanes` -- are in `-common.sh`, per the seam rule the dispatcher
# states: cited by more than one memo -> common. `suites` sat here as a
# recorded exception until Codex R17; the rule has no exceptions.

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
  # A quoted heredoc, not `python3 -c '…'`: a `-c` payload is wrapped in the
  # shell's single quotes, so one apostrophe in a comment ended the program
  # mid-line twice in one session (R15/R16). The heredoc idiom has no such
  # hazard, so the harness uses it everywhere and carries no payload guard.
  _measure n_unc python3 - "$files" <<'PY' || return 1
import os, sys
ROOTS = (".claude/tools/_webref", ".claude/skills/elidex-plan-review")
for f in sys.argv[1].split():
    root = next(r for r in ROOTS if f.startswith(r + "/"))
    d = os.path.dirname(f)
    while d != root:
        if not os.path.isfile(os.path.join(d, "__init__.py")):
            print(f"{f}  (no __init__.py in {d})"); break
        d = os.path.dirname(d)
PY
  uncollected=$(_measured)
  [ -z "$uncollected" ] || { echo "!! suite(s) under a root but below a non-package directory — discover never reaches them:"; printf '%s\n' "$uncollected" | sed 's/^/     /'; return 1; }
  return 0
}

# `_gated JOB TEXT`: exit 0 iff JOB's region in TEXT carries both `needs:
# changes` and an `if:` on a `changes` output. Region = from `  JOB:` to the next
# line at two-space indent that is not blank/comment/list (the full job-key
# grammar, quoted and digit-bearing keys included).
# `_job_region JOB TEXT`: the lines of JOB's region (whole lines, block
# scalars included -- a region read, not a step parser). Shared by `_gated`
# and by A-iii §6 Q6, which reads the `tools` job's region for the driver path.
# The header matches the key plain or quoted (`  check:` / `  "check":` /
# `  'check':`), the same key grammar the boundary rule admits (Codex R37).
_job_region() { printf '%s\n' "$2" | awk -v j="$1" -v q="'" '$0 ~ "^  [\"" q "]?" j "[\"" q "]?:$" {f=1; next} f && /^  [^ #-]/ {exit} f {print}'; }
# "Gated" = the COMPLETE approved condition, anchored: `needs.changes.outputs.<set>
# == 'true'` with at most the intentional push exception (`|| github.event_name ==
# 'push'`, which is how main gets every job). A prefix match certified
# `… == 'true' || always()` -- a job that runs on every PR -- as gated (Codex R38).
_gated() { _job_region "$1" "$2" | awk -v q="'" '/^    needs: changes$/ {n=1} $0 ~ ("^    if: needs\\.changes\\.outputs\\.(rust|config) == " q "true" q "( \\|\\| github\\.event_name == " q "push" q ")?$") {g=1} END {exit !(n && g)}'; }

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
  { r=$(_job_region trip-wires "$ci"); [ -n "$r" ] && ! printf '%s\n' "$r" | grep -qE '^    (needs|if):'; } \
    || { echo "!! \`trip-wires\` is absent or gated — §9's 'present and ungated since #496' no longer holds"; rc=1; }
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

floor() {  # §4.4 — a STATIC PRE-CHECK of the declared 3.9 floor, not a proof of it
  # What this block can decide: grammar (`ast` feature_version), def-time PEP 604
  # unions, and a sample of runtime-only 3.10+ names. What it cannot: every
  # newer-runtime API (`tomllib`, `Path.walk`, …) or an evaluated type alias
  # under a future import (Codex R21). The only proof that the suites run on
  # 3.9 is running them on 3.9, which CI does not do; A-iii §4.4 says so and
  # claims "declared and pre-checked", not "measured".
  # "No .claude Python source uses syntax newer than 3.9" was a sentence with
  # no instrument behind it (Codex R17 -- whose own evidence, `str | None` in
  # `cache.py` and `preflight.py`, is a string annotation under `from __future__
  # import annotations` and parses on 3.9; the point stands that nothing
  # CHECKED). Three readings, each a way a 3.9 interpreter fails: grammar
  # (`ast.parse` with `feature_version`), PEP 604 unions evaluated at def time
  # (no future-import), and runtime-only 3.10+ APIs. The memo figure is 3.9.
  local files
  _measure files git ls-files ".claude/**/*.py" || return 1
  [ "$files" -gt 0 ] || { echo "!! no .claude Python files enumerated"; return 1; }
  # (The heredoc IS python3's stdin, so the file list cannot also arrive on
  # it -- the first cut piped `git ls-files` into a heredoc and measured 0 files.)
  python3 - <<'FLOORPY'
import ast, re, subprocess, sys
_ls = subprocess.run(["git", "ls-files", ".claude/**/*.py"], capture_output=True, text=True)
if _ls.returncode != 0 or not _ls.stdout.split():
    raise SystemExit("!! `git ls-files .claude/**/*.py` enumerated nothing; 'floor 3.0' would be the reading")
files = _ls.stdout.split()
RUNTIME = re.compile(r"isinstance\([^)]*\|[^)]*\)|zip\([^)]*strict=|slots=True|kw_only=|pairwise\(|TypeAlias|ParamSpec")
def pep604(tree, future):
    # Every annotation that is EVALUATED on 3.9 -- any `X | Y` inside a
    # parameter/return/variable annotation without the future import, and a
    # module/class-level alias `Name = X | Y` regardless of it (an alias is a
    # runtime expression, not an annotation) -- not only `| None` (Codex R37).
    def has_bitor(node):
        return any(isinstance(n, ast.BinOp) and isinstance(n.op, ast.BitOr) for n in ast.walk(node))
    for node in ast.walk(tree):
        if not future:
            if isinstance(node, ast.AnnAssign) and has_bitor(node.annotation):
                return True
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                a = node.args
                for arg in a.args + a.posonlyargs + a.kwonlyargs + [a.vararg, a.kwarg]:
                    if arg is not None and arg.annotation is not None and has_bitor(arg.annotation):
                        return True
                if node.returns is not None and has_bitor(node.returns):
                    return True
        # An assignment's VALUE is evaluated whatever the future import says, so
        # any type-shaped `X | Y` anywhere inside it -- `str | bytes | None`
        # (a BitOr whose left operand is itself a BitOr), `list[str | bytes]`
        # (a BitOr under a Subscript) -- raises on 3.9 (Codex R38). "Type-shaped"
        # = both operands are names / attributes / subscripts / None / further
        # unions. A runtime `flags | MASK` of two bare names is indistinguishable
        # statically and reads RED: the safe direction. Dotted operands are NOT
        # type-like: `re.IGNORECASE | re.MULTILINE` is the tree's only dotted
        # `|` (measured: `git ls-files '.claude/**/*.py' | xargs grep -nE
        # '\b\w+\.\w+\s*\|\s*\w'` → re-flag ORs only), so a dotted
        # `mod.Type | None` alias would be missed -- a stated limit, not a claim.
        if isinstance(node, (ast.Assign, ast.AnnAssign)) and node.value is not None:
            def typelike(o):
                return (isinstance(o, ast.Name) or
                        (isinstance(o, ast.Constant) and o.value is None) or
                        (isinstance(o, ast.Subscript) and typelike(o.value)) or
                        (isinstance(o, ast.BinOp) and isinstance(o.op, ast.BitOr) and typelike(o.left) and typelike(o.right)))
            for sub in ast.walk(node.value):
                if isinstance(sub, ast.BinOp) and isinstance(sub.op, ast.BitOr) and typelike(sub.left) and typelike(sub.right):
                    return True
    return False
need, bad = {}, []
for f in files:
    src = open(f, encoding="utf-8").read()
    floor = None; tree = None
    for minor in (9, 10, 11, 12, 13):
        try:
            tree = ast.parse(src, feature_version=(3, minor)); floor = minor; break
        except SyntaxError:
            continue
    if floor is None:
        bad.append((f, "does not parse under any feature_version 3.9..3.13")); continue
    future = any(isinstance(n, ast.ImportFrom) and n.module == "__future__" and any(a.name == "annotations" for a in n.names) for n in tree.body)
    if floor == 9 and pep604(tree, future):
        floor = 10
    if floor < 10 and RUNTIME.search(src):
        floor = 10
    need[f] = floor
mx = max(need.values()) if need else 0
print(f"  {len(need)} files; highest floor measured: 3.{mx}")
for f, why in bad: print(f"  !! {f}: {why}")
if mx != 9 or bad:
    for f, v in sorted(need.items()):
        if v > 9: print(f"  !! needs 3.{v}: {f}")
    print("  !! A-iii §4.4 states the floor is 3.9; the tree says otherwise")
    sys.exit(1)
FLOORPY
  return $?    # the heredoc'd command IS the measurement; say so
}
