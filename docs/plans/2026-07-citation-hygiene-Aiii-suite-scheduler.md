# Plan — Slice A-iii: something runs the Python suites, ungated

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-iii** (the 2026-08-01 re-slice).
Terminal unit under that boundary (§9). **Branch**: new, stacked on **A-ii's landed head**.
**Nature**: repo infrastructure — one shell script, one `mise` task, one CI job. Zero `crates/**` diff, zero
`_webref` diff, zero gate-semantics change.
**Status**: plan-memo, **draft 1**. `/elidex-plan-review` **required before implementation**.

**This memo carries no measured digits of its own** → `rederive suites`, `filters`, `suiteset`, `ruleset`,
`budget`. `lanes` and `staleclaims` are author-local and excluded from `all`.

### §0.1 What A-iii is

A-i landed the shared map and A-ii made the gate fail closed. **Nothing runs either one's tests.** A-iii
gives the Python suites a scheduler, and nothing else.

---

## §0.5 / §3. Spec coverage map

**No spec surface.** A-iii touches no spec-defined behaviour: a shell script, a `mise` task, and a CI job.

⚠ **A-iii is the first real consumer of A-ii's §4.2.5 declaration**, and it uses it deliberately rather than
authoring fixture citations it does not need. Under the pre-A-ii gate this memo would hard-fail (no table),
and under the merged memo's shape it would have had to invent two citations and then receive
`citation verify: ok` as its headline — §1's failure, in the memo whose own subject is a gate. That is the
whole argument for §4.2.5, demonstrated by the slice that immediately follows it.

If A-iii is authored before A-ii lands, it carries the two-row fixture table instead and this section is
replaced at rebase.

### §3.1 User-input touch audit + discovery method

**No web-content input flow.** The inputs are a CI trigger and a file glob.

**Discovery method.** Measured against `origin/main`; the CI facts against the live ruleset via `gh`; the
cross-lane facts against the branches that contend, not against a recorded list.

---

## §1 Ideal anchor — a check nobody runs is a claim, not a check

`origin/main` carries 4 Python test files under `.claude/`, and **no `mise` task, no CI job and no hook runs
any of them**. The umbrella's own constraint — *a claim is admissible only if something mechanically checks
it* — is unmet for the tooling this program is building. → `rederive suites`

**And a scheduler that can be silently skipped is the same defect with an extra step.** That is why A-iii's
job is ungated (§4.2).

---

## §2 Coupled invariants

- **L1 — one script, two callers.** If `mise` and `ci.yml` each spell the suite invocation, a later suite is
  added to one and not the other.
- **L2 — no suite is silently uncollected.** A `test_*.py` under `.claude/` that neither `discover` root
  reaches must fail loudly, not pass quietly.
- **L3 — the trigger cannot be silently narrowed.** No path filter: see §4.2.
- **L4 — A-iii adds no network dependency.** The suites must run green offline.

**Pairwise intersections:**

| pair | intersection |
|---|---|
| L1 × L2 | the loud failure belongs to the **script**, not to either caller — otherwise it is itself a thing one caller can skip |
| L1 × L3 | with no filter, the CI caller's trigger is unconditional, so the two callers differ only in *where* they run, never in *whether* |
| L2 × L3 | the set L2 ranges over is `git ls-files '.claude/**/test_*.py'` — a repo fact, not a filter list |
| L3 × L4 | an ungated job runs on every PR, so a network-dependent suite would fail every unrelated PR |

---

## §4 The edit set

### §4.1 The hole

`ci.yml`'s `changes` filter has two sets, `rust` and `config`; **`.claude/**` is in neither**, and all three
jobs are gated on one of the two. `ci.yml` never invokes `mise`. `codeql.yml` analyses `[actions, rust]` on
push plus a weekly cron, with no `pull_request` trigger; `audit.yml` is `cargo audit` on a cron. ⇒ a
`.claude/**`-only pull request triggers **zero jobs**. → `rederive filters`, `rederive suites`

⚠ **That is a fact about `origin/main` with a lifetime.** The Layout lane's
[PR #496](https://github.com/send/elidex/pull/496) lands an ungated trip-wire job, which makes it false
whichever of the two lands first. §12's criterion is written so it does not depend on landing order.

### §4.2 The mechanism — one script, two callers, no filter

`.claude/tools/python-suites.sh`, `set -euo pipefail`, then two `discover` lines rooted at
`.claude/tools/_webref` and `.claude/skills/elidex-plan-review`. `mise.toml` gains `[tasks.tools-test]` added
to `[tasks.ci].depends`; `ci.yml` gains a `tools` job that is **deliberately ungated** — no `needs: changes`,
no path-filter entry.

⚠ **Earlier drafts of the merged Slice A specified a `tools` path-filter set (`.claude/tools/**`,
`.claude/skills/**`, `.github/workflows/**`). A-iii drops it, and the reason is §1.** The Layout lane's
branch states it exactly: *to gate them, `.claude/tools/**` would have to be listed in a filter — i.e. the
tamper path of an allowlist gate would itself be an allowlist entry someone must remember to keep current.*
Three grounds, in order of weight:

1. **The filter is the failure mode this slice exists to remove.** A PR editing only the allowlist would
   skip the job that reads the allowlist.
2. **The cost argument does not apply.** Every other `ci.yml` job is filtered because it pays for a Rust
   toolchain. The Python suites need no toolchain and no cache — the filter buys nothing.
3. **One issue, one way.** Two branches were about to ship two answers to one question. The Layout lane's is
   better argued and already open as a PR; adopting it is entirely inside A-iii — **no file of that branch
   is touched.**

This also removes a collateral class earlier drafts had to document: with no filter there is no "every
dependabot GHA bump now runs the Python suites" side effect, because the trigger is not a path list.

**The script fails loudly when a `test_*.py` under `.claude/` is not collected by either `discover` root**
(L2). ⚠ The set the assertion ranges over is `git ls-files '.claude/**/test_*.py'` — a *repo* fact. Wording
it as "outside the filtered paths" keys on a CI filter that no longer exists, and would have let a suite at
`.claude/skills/elidex-review/` pass while uncollected. → `rederive suiteset`

⚠ **Location.** `python-suites.sh` goes where the repo already puts **CI-invoked drivers**. Measured,
`origin/main` uses `scripts/` for that (`scripts/ci-sweep.sh`, `scripts/doc-changed.sh`) while
`.claude/tools/*.sh` holds the trip-*wires* themselves; and the Layout lane's branch moves its own driver to
`scripts/trip-wires.sh`. Earlier drafts sited this in `.claude/tools/` on the ground that "the four
trip-wire scripts already live there", which conflates the wires with their driver. **A-iii uses
`scripts/`**, matching both the pre-existing convention and the adjacent lane.

### §4.3 The network question — answered by construction

Measured: **0 `urlopen` calls** across all `origin/main` tests. A-i's suite exercises the pinned dicts and
`coverage_map._spec_label`; under A-i's split none reaches `sources/webref_data`, because `spec_labels.py`
no longer imports it. A-ii's suite stubs `verify_citation` at suite level, so it spawns no `webref` child.
→ `rederive suites`

⚠ **The baseline instrument has a stated limit.** `verify_citation` runs
`subprocess.run([sys.executable, WEBREF, …])` and `urlopen` is called only inside `cache.py`, in the
**child**; a parent-process patch cannot see it. So the `origin/main` figure measures parent-process calls,
and what those tests do in child processes is invisible to it. That limit is acceptable only because those
suites spawn no `webref` child — which **T-net**'s first assertion is what actually establishes.

⚠ **What A-iii does *not* claim**: that the gate becomes offline-capable. It is not, and was not —
`verify_citation` shells out to `webref heading`, which issues a conditional GET, and `cache.py` `sys.exit`s
on `URLError`, so **`origin/main`'s gate already requires the network in default mode**. A-iii's claim is
exact: *the suites run green offline*, and `--no-verify` stays clean.

⚠ **The umbrella's own record of this was wrong and is corrected there.** It said wiring the suites into CI
takes a live-network dependency, citing a 48-test suite fetching 2 URLs. That figure was measured on a
branch whose catalog fall-through — Slice **B's** — was the thing fetching. B ships the offline contract for
what B introduces; A-iii inherits no such dependency.

### §4.4 The interpreter floor

No `.claude` Python source uses syntax newer than 3.9. `python-suites.sh` asserts
`sys.version_info >= (3, 9)` — the measured need — and the job echoes `python3 -VV`. Slice B raises the floor
when B lands `(?>...)`. `SKILL.md`'s Step 0 invokes `preflight.py` directly, bypassing the script;
unaffected today, marked UNCHECKED in §6.

### §4.5 What "enforced" can honestly mean here

`main` is governed by an **active** ruleset whose rules are `deletion` / `non_fast_forward` /
`pull_request`. There is **no `required_status_checks` rule**, so a red `tools` job does not block a merge;
CLAUDE.md's "CI 全 pass を目視確認してから squash merge" is the blocking step, and it is human. (The 404 from
`…/branches/main/protection` is the **deprecated legacy endpoint** and means "not protected via the legacy
API", not "unprotected".) The claim A-iii may make: the job makes a regression **visible, attributed, and on
the PR page at review time**. → `rederive ruleset`

---

## §5 Behaviour deltas

| # | Input | `origin/main` | After A-iii |
|---|---|---|---|
| 1 | a `.claude/**`-only PR | **zero jobs** | the `tools` job runs |
| 2 | a `crates/**`-only PR | `check`/`doc`/`deny` | + the `tools` job (ungated) |
| 3 | `mise run ci` locally | does not run the Python suites | runs them via `[tasks.tools-test]` |
| 4 | a new `test_*.py` under `.claude/` outside both `discover` roots | passes by not existing to anyone | **script fails loudly** (L2) |
| 5 | the suites, offline | n/a — nothing runs them | green (L4) |

**Newly-red**: 4, and only 4 — A-iii adds a job, it does not change what any existing job asserts.

---

## §6 Pins

| Pin | What it executes | §5 rows | Fails at A-ii's head? |
|---|---|---|---|
| **Q1** | `bash scripts/python-suites.sh` collects and passes both roots' suites | 3 | **yes** — the script does not exist |
| **Q2** | a `test_*.py` planted outside both `discover` roots makes the script exit non-zero, naming the file (L2) | 4 | **yes** |
| **Q3** | the set Q2 ranges over is `git ls-files '.claude/**/test_*.py'`, not a filter list — asserted by planting the file at `.claude/skills/elidex-review/`, which any plausible filter would have covered | 4 | **yes** |
| **Q4** | `mise run ci` reaches `tools-test` (the `depends` edge exists) | 3 | **yes** |
| **Q5** | the interpreter-floor assertion fires below 3.9 | — | **yes** |
| **T-net** | `bash scripts/python-suites.sh` runs green in a child with `http_proxy`/`https_proxy` at a closed port, **and** `subprocess.run` is never called with the resolved `WEBREF` path across the suite set | 5 | **yes** |

⚠ **UNCHECKED, marked not omitted**: that a red `tools` job **blocks** a merge — **false**, see §4.5; the
interpreter floor on `SKILL.md`'s direct `preflight.py` path, which bypasses the script.

⚠ **Q3 exists because the L2/L3 intersection is easy to state wrongly.** A pin that plants the file inside
the old filter's paths would pass under both the filter design and the ungated one, and so discriminates
nothing.

---

## §7 Layering check

**VM host/ / ECS-native** — not applicable.

**Generic core vs elidex adapter.** A-iii touches **no `_webref` file at all** — `scripts/`, `mise.toml`,
`.github/workflows/ci.yml`. It is repo infrastructure, not tool code. → `rederive couplings`

⚠ Earlier drafts planned to record the `mise` task, the CI job and the interpreter floor in
`_webref/DESIGN.md`. That file says the core should "stay generic enough to move to a standalone repository
later"; a section describing `mise.toml` and `ci.yml` travels with the tree at externalization and is wrong
on arrival. Those facts live in `python-suites.sh`'s header and the `mise.toml` task comment.

**One-issue-one-way**: the suite invocation goes from **zero** canonical sites to one.

---

## §8 Line-count budget

→ `rederive budget`. One new ~40-line script; `mise.toml` and `ci.yml` gain a task and a job. Nothing near a
split.

---

## §9 Edge-dense assessment

**(i)** The umbrella names A-iii and states its scope, amended before this memo's plan-review.

**(ii)** L1–L4 are **configuration** invariants with one observable (does the job run, and is it green).
There is no control flow, no exit-code semantics and no data. This is the least dense of the three A slices
and would not trip the edge-dense trigger on its own; it is a separate slice because it was **invisible**
inside the merged memo — the one cross-lane collision that memo missed lived here, and its low finding count
measured that invisibility rather than its simplicity.

---

## §10 Open questions

- **Q1 — does `required_status_checks` belong in this PR?** One rule on an existing active ruleset — but the
  `pull_request` rule already carries `required_approving_review_count: 0` **and** a `RepositoryRole` bypass
  with `bypass_mode: always`, so adding it leaves it author-bypassable: visibility-plus-friction, not
  enforcement. **Recommendation: register as a slot, do not implement** (§11).

---

## §11 Defer slots + per-PR ≤3 audit

**Zero own deferrals.**

**Registered here because A-iii is the slice that adds a CI job**:
**`#11-elidex-ci-required-status-checks`** — the ruleset has no `required_status_checks` rule, so every CI
job is advisory. A-iii neither creates nor worsens it. **Why deferred**: the rule alone is author-bypassable
given the existing bypass actor, so it is not a one-line fix. **Trigger**: the first job stable enough to
require, or the bypass actor being removed. **Re-eval**: 2026-11-30. **Confidence**: Medium.
Measured, it exists in no ledger today — §13 registers it.

⚠ **`#11-layoutbox-trip-wire-not-in-ci` is *not* A-iii's to record.** Its trigger fired and it is being
discharged by [PR #496](https://github.com/send/elidex/pull/496). A-iii's landing checklist notes the PR,
not the trigger — a note saying "the trigger has fired" would land describing a state already superseded.

---

## §12 Exit criterion

**(1) Green:** `mise run tools-test` and `bash scripts/python-suites.sh`.

**(2) Red at A-ii's head:** every pin whose §6 row says "yes". No second list.

**(3) The job fires on this slice's own file class** — a `.claude/**`-only commit shows the `tools` job on
the PR page. ⚠ Stated as a **positive** observation only. Phrasing it as a delta ("today the same
observation yields zero jobs") is falsified by PR #496 independently of anything A-iii does.

**(4) Offline:** T-net.

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** | A-iii branches from its landed head | **A-ii first** |
| **[PR #496](https://github.com/send/elidex/pull/496)** (Layout lane) | **the real contention** — `ci.yml` and `mise.toml`. Its head moves; re-derive rather than record | **design pre-agreed**: §4.2 adopts the ungated shape and `scripts/`, so whichever lands second is a textual merge, not a decision |
| **[PR #381](https://github.com/send/elidex/pull/381)** (`actions/checkout` bump) | `ci.yml` `steps:` only | whichever lands second adapts |
| **Slices B / C** | after A-iii these are enforced from their first commit | after |

⚠ **The lane facts are derived, not listed** → `rederive lanes`, which ranges over the files A-iii contends
on (`.github/`, `mise.toml`, `.claude/tools/`) and not only over `docs/plans/`. A `docs/plans/` filter plus
`gh pr list` could not see PR #496 while it was unpushed, which is how the merged memo missed it.

**Landing checklist**

1. Register `#11-elidex-ci-required-status-checks` in `project_open-defer-slots.md`.
2. Note PR #496's outcome against `#11-layoutbox-trip-wire-not-in-ci` at the five sites that carry it.
3. Update `project_citation-hygiene-program.md`.
4. PR description: §4.2's ungated decision and the Layout-lane reconciliation; §4.5's honest "enforced".

---

## §14 Provenance

A-iii is carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` §4.3 (drafts 1–9). Carried here as
settled: the filter hole (R1), the `trip-wires` precedent correction (R6), the two-caller invariant, the
`git ls-files` scoping of L2 (R8), the honest-enforcement paragraph (R5), and the ungated decision plus the
`scripts/` siting (R9).

---

## §15 Re-derivation

`docs/plans/2026-07-citation-hygiene-A-rederive.sh`. Blocks A-iii cites: `suites filters suiteset ruleset
budget couplings lanes`. ⚠ `lanes` and `staleclaims` are author-local and excluded from `all`.
