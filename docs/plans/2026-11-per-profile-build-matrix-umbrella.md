# Umbrella U0 — per-feature-profile build matrix (draft 2, 2026-11-08)

## §0 Status

**Lane**: CI/tools (gate policy). **Base**: `origin/main` at **`69fbe61f`** (#609, 2026-11-08). Every path, line and
count below was read at that commit; the command that produced each count is printed next to it.

**Slots**: `#11-elidex-js-default-features-build-broken` (owner CI/tools; this umbrella is its program) and
`#11-elidex-js-no-default-features-support` (the "is the no-engine build a supported surface" question; answered by
PM 520, §5; re-homed to VM P4 by PM 531, §8). `#11-test-only-feature-boundary-guard` is coupled
(§4.2, PR-M2).

**What this memo is**: the umbrella of an edge-dense program (PM 494/498 decision 1): the decisions on the four axes
PM fixed (decision 2), the coupled invariants (§2), the PR list with owners, scope, acceptance and order (§6). Per-PR
implementation detail belongs to each PR's own plan, and each PR plan goes through `/elidex-plan-review` before
implementation.

**F1 decided (PM 520, user decision).** Whether `elidex-js` built without `engine` is a supported surface did not
converge by the authority order (PM decision 4 reserved it; §5 keeps the derivation as the record). The user chose
**F1-c** (split the crate). Per §5's contingent path, the split is its own umbrella, owned by the VM P4 lane and
**outside this umbrella's scope**; **F1-b** (keep `default = []`, fix the gating) is the interim. So elidex-js
declares three rows (§4.1) and PR-A1 has the F1-b shape only (§6); a site whose right fix is a non-optional
dependency returns to PM (§5).

**Terminology.** In this memo a **feature profile** is a named set of Cargo features of one workspace member that the
member declares as a supported configuration. It is not a Cargo *build* profile (`[profile.*]`, Cargo Book §3.5
Profiles; nextest's `--profile ci` is a third meaning). The declaration's key and the CI job's name say
`feature-profiles` for that reason (§4.5).

## §1 The defect, and the ideal

**Defect.** CI and every mise task compile the workspace as one Cargo invocation. Cargo builds a dependency with the
union of the features every selected package enables on it (Cargo Book §3.4 #feature-unification, §3.3.3 #features:
"When building multiple packages in a workspace … the features of the dependencies of all of those packages are
unified … build them via separate cargo invocations"). So a member's own configuration — the one an embedder gets
from `cargo build -p <member>` — is compiled by no gate whenever another member enables more features on the same
crates. Today:

- `elidex-shell` enables `elidex-js` `engine` + `compat-webapi` (`crates/shell/elidex-shell/Cargo.toml:31`), so
  every workspace build of `elidex-js` has both on. `cargo check -p elidex-js --lib --bins` with no features fails
  (15 errors at `69fbe61f`, §9) and nothing in CI sees it; the earliest red line still present dates from #177
  (`git blame -L 45,48 crates/script/elidex-js/src/vm/gc/collect.rs` → `5ba1ac38`, 2026-05-10; `elidex-ecs` was
  already optional then: `git show 5ba1ac38:crates/script/elidex-js/Cargo.toml`).
- The App configuration (`engine` without `compat-webapi`, the comment at `crates/script/elidex-js/Cargo.toml:15-17`)
  shipped an E0432 through a green `mise run ci` (PM 410; fixed in D-14 (b); green at `69fbe61f`).
- The one compiling step without `--all-features` — #585's `Clippy (default features, no dev-only feature)`
  (`.github/workflows/ci.yml:111-112`, `cargo clippy --workspace -- -D warnings`, mirrored by mise `check-default`,
  `mise.toml:14-19`) — is itself a workspace build, so it sees the union of members' normal-edge features, not any
  member's own configuration (its comment says so, `ci.yml:99-101`).
- The profiles themselves exist only as prose: `crates/script/elidex-js/Cargo.toml:15-35`,
  `crates/script/elidex-script-session/Cargo.toml:15-24`, `crates/storage/elidex-storage-core/Cargo.toml:17-24`.
  Nothing machine-readable says which configurations are supported, so no gate can be derived from them, and the
  "A2 absence guarantee" those comments promise (App build carries no `WebStorageManager`) is checked by nothing.
- `elidex-style`'s `parallel` feature is enabled by no member (`git grep -n 'parallel' -- '*Cargo.toml'` → only
  `crates/css/elidex-style/Cargo.toml:31` and two unrelated root lines), so its code compiles only under
  `--all-features`, i.e. only with dev-only features on.

**Ideal.** Each member states, once and machine-readably, the feature profiles it supports; CI derives from that
statement one isolated Cargo invocation per (member, profile) and compiles each on every OS leg the codebase targets;
every feature a member exposes is compiled by some profile; test-only features and per-profile absences are asserted
on each profile's own resolve; and there is exactly one mechanism for "which configurations does CI compile" — the
`--all-features` test configuration stays separate because it is a different property (dev-only features on, test
targets), not a second answer to this one.

## §2 Coupled invariants

Eight invariants; the pair table holds all 28 pairs.

- **I1 — one declaration.** Feature profiles are declared once per member in that member's `Cargo.toml` under
  `[package.metadata.elidex.feature-profiles]` (NEW) — the table Cargo ignores and leaves to tools (Cargo Book §3.1
  #the-metadata-table) and that `cargo metadata` returns (`packages[].metadata`). A member's `default` feature set
  is its implicit `default` profile and needs no entry. Test-only features are declared in the same table (I5).
  The prose comments that describe profiles today become pointers to the declaration, not restatements.
- **I2 — derived population, every row supported.** The rows are `workspace_members` (from `cargo metadata`) ×
  ({`default`} ∪ declared profiles) × OS legs. Never a list in YAML or a script. A member cannot opt its `default`
  out: a default that is not a supported surface is a feature-shape question (PM decision 4), not a skipped row. An
  unparseable declaration, an unknown key, or a profile naming a feature the member does not have is red, never
  skipped. Non-vacuity: the run asserts it produced at least one row per workspace member.
- **I3 — isolation.** One Cargo invocation per row (`-p <member>`; named profiles via `--no-default-features
  --features <list>`), lib + bin targets only. Under resolver 2 (`Cargo.toml:2`) a lib/bin build activates no
  dev-dependency feature (Cargo Book §3.4 #feature-resolver-version-2), so a row is the member's production
  configuration; one invocation per row is what the Cargo Book names as the way to avoid cross-package unification.
  `clippy -- -D warnings`, so a warning that exists only in that configuration (e.g. an import left unused) is red.
- **I4 — completeness.** Every feature a member declares in `[features]` — excluding `default`, Cargo's implicit
  optional-dependency features (value exactly `["dep:<name>"]`), and declared test-only features — is active in the
  resolve of at least one row. Measured on each row's own resolve, never on the workspace resolve.
- **I5 — forbidden features.** No declared test-only feature is active in any row's resolve, and each profile's
  declared absences (e.g. App: `elidex-storage-core/web-storage`) are inactive in that row's resolve. Fail-closed:
  a resolve the check cannot read is red.
- **I6 — one mechanism.** After the program, the configurations CI compiles without dev-only features are exactly
  the matrix rows. #585's step and `mise check-default` are retired in the same PR that adds the rows (PR-M1);
  `scripts/check-fixture-boundary.sh` is absorbed into I5 (PR-M2). The `check` job's `--all-features` steps stay:
  they are the test configuration (dev-only features on, `--all-targets`), a different property.
- **I7 — CI shape, parity, cost.** A dedicated job `feature-profiles` (NEW) on the same three OS as `check`, rows
  run serially inside one entry point; `mise run check-profiles` (NEW) runs the same entry point (host OS only);
  `timeout-minutes` derived from the job's own run (§4.3).
- **I8 — land green.** No PR of the program lands red, and the matrix PR lands only on a base where every row is
  green on all three OS. Adding a profile to the declaration later is adding a row: the PR that adds it makes it
  green (or carries the fix) in the same PR.

| Pair | Intersection |
|---|---|
| I1×I2 | The deriver reads only `cargo metadata`; a malformed declaration is a red row, not a dropped one. |
| I1×I3 | A profile is a feature list for one `--no-default-features --features` invocation; the declaration cannot express cross-member unification, by construction. |
| I1×I4 | Completeness is computed against `[features]` keys minus implicit and declared test-only ones; a new feature with no covering row or edge is red in the PR that adds it, which forces its declaration there. |
| I1×I5 | Test-only features and per-profile absences live in the same table as profiles; the fixture script's hard-coded `elidex-ecs` / `fixtures` literal becomes a declaration entry. |
| I1×I6 | The Cargo.toml prose (§1 list) becomes a pointer to the declaration; profile semantics are not restated in comments. |
| I1×I7 | The declaration lives in `Cargo.toml` files, already inside ci.yml's `rust` path filter (`ci.yml:34-48`, `crates/**`). |
| I1×I8 | Declaring a profile adds rows on three OS; the declaring PR owns their green. |
| I2×I3 | Population × isolation = invocation count = cost; the only permitted reduction is de-duplicating rows whose (member, resolved feature set, OS) is identical, computed per run, never listed. |
| I2×I4 | Completeness is only as strong as the population; the non-vacuity assertion (≥ 1 row per member) stops a silently shrinking derivation from making I4 vacuous. |
| I2×I5 | Forbidden checks run per row, so per-OS target-specific edges are seen on the three OS legs (the fixture script today sees the host target only: it runs `cargo tree` with no `--target`). |
| I2×I6 | The workspace-union configuration is not a row (§4.2 says what that gives up). |
| I2×I7 | Row count × three legs drives the timeout; a PR adding members or profiles re-derives it (§4.3). |
| I2×I8 | The census (C0, §7) is the population evaluated on the base before PR-M1; PR-M1's own three-OS run is the land-green evidence. |
| I3×I4 | Each row's resolve is read from that row's own invocation (`cargo tree -p <member> -e features` with the row's flags); reading the workspace resolve would re-hide what unification hides. |
| I3×I5 | Same: the fixture script's `cargo tree --workspace` is a union; after absorption the assertion is per row. |
| I3×I6 | The `--all-features` steps are not rows: they turn dev-only features on for test targets. |
| I3×I7 | Serial invocations share one target dir; Cargo keys artifacts on the feature set, so rows do not contaminate each other, and a separate job gets its own rust-cache key, so its feature sets do not evict `check`'s. |
| I3×I8 | A row's verdict does not depend on row order (artifacts are feature-keyed), so a green census predicts a green PR-M1 run up to base drift. |
| I4×I5 | A feature can be required by I4 and forbidden by I5 at once only per profile: App forbids `web-storage`, the browser row covers it. Both hold; PR-M2's controls include exactly this case. |
| I4×I6 | I4 is strictly stronger than #585's step: #585 compiled features some member enables; I4 also reaches features no member enables (`elidex-style/parallel`). |
| I4×I7 | One `cargo tree` per row, inside the same entry point. |
| I4×I8 | `elidex-style/parallel` is uncovered today, so I4 is red at the base: it needs its fix (declare + green, or remove) before PR-M2 lands (PR-A2). |
| I5×I6 | PR-M2 retires `scripts/check-fixture-boundary.sh` and its ci.yml step and path-filter line, moving the assertion into I5; `#11-test-only-feature-boundary-guard` attaches here instead of to the retired #585 step. |
| I5×I7 | The forbidden checks run on every OS leg. |
| I5×I8 | The App row must already satisfy the A2 absence at PR-M2's base (census C0 includes it). |
| I6×I7 | `[tasks.ci]` (`mise.toml:142`) replaces `check-default` with `check-profiles` in the same PR. |
| I6×I8 | Retiring #585's step and adding the rows are one PR: no commit on `main` has neither, or both. |
| I7×I8 | PR-M1's own three-OS run is both its land-green evidence and its timeout measurement. |

## §3. Spec coverage map

A CI plan touches no web-platform algorithm. The rows below are the **governing sources** of the design: Cargo's
documented feature-resolution behaviour (the Cargo Book shipped with the pinned toolchain 1.99.1,
`rust-toolchain.toml`, read from `~/.rustup/toolchains/1.99.1-aarch64-apple-darwin/share/doc/rust/html/cargo/`; the
§ numbers are that book's chapter numbering, anchors are its heading ids) and the two specification sections the §5
derivation rests on. Honesty notes: (a) `preflight.py` cannot verify the Cargo Book rows — "Cargo Book" is not in its
`SPEC_LABEL_REVERSE`, so those rows are unmapped-label soft-warns; they were checked by hand against the local book;
(b) the repo's own rules (CLAUDE.md, ci.yml comments, PM decisions) govern as much as Cargo does, but they carry no
§ number and a row without one is a preflight hard-fail, so they are listed under the table, not in it; (c) "Step"
holds an anchor, "Branch" the behaviour the design relies on.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Cargo Book §3.4 Features | #feature-unification | a dependency is built with the union of features its dependents enable | I3 (one invocation per row) | ✓ | no |
| Cargo Book §3.3.3 Dependency Resolution | #features | `--workspace` / several `-p` unify; separate invocations do not | I3, I6 (#585's step is a union) | ✓ | no |
| Cargo Book §3.4 Features | #feature-resolver-version-2 | dev-dependencies activate no feature for lib/bin targets | I3 (lib + bins only) | ✓ | no |
| Cargo Book §3.4 Features | #resolver-version-2-command-line-flags | `--features` / `--no-default-features` apply to the `-p` selection | row invocation shape | ✓ | no |
| Cargo Book §3.4 Features | #optional-dependencies | an optional dep without `dep:` defines an implicit feature | I4 exclusion rule | ✓ | no |
| Cargo Book §3.4 Features | #feature-combinations | full coverage is exponential; the project chooses its configurations | I1/I2 (declared profiles, not a powerset) | ✓ | no |
| Cargo Book §3.1 The Manifest Format | #the-metadata-table | `package.metadata` ignored by Cargo, returned to tools | I1 (declaration home) | ✓ | no |
| Cargo Book §3.5 Profiles | (chapter) | "profile" already means build profile | §0 terminology, §4.5 | ✓ | no |
| ECMA-262 §4.2 Hosts and Implementations | host / host hook / host environment | the language defers host-defined facilities to an external source | §5 derivation step (1) | ✗ | no |
| WHATWG HTML §8.1.6 JavaScript specification host hooks | (section) | HTML is the host that defines those hooks | §5 derivation step (1) | ✗ | no |

**Breadth**: K=3 sources (preflight count), M=10 rows. The spec rows are ✗ for full enumeration on purpose: §5 uses them only to
establish that the language/host seam is a specification structure, not to implement any hook.

**Repo rules that govern (no § number)**: CLAUDE.md 至上命題 (authority order), Design philosophy (core/compat
layering, plugin-first, ScriptSession as the sole Script↔ECS boundary, Ideal over pragmatic), Design discipline (One
issue one way, dead code, Supported-surface testing, Edge-dense, 1000-line touch-time split), `## CI` (job boundary,
filter gating); ci.yml's `tools`-job timeout rule (`ci.yml:250-269`); PM 336 (relative acceptance), PM 410 (matrix
direction), PM 494/498 (this umbrella's frame).

### §3.1 User-input touch audit

None. No row is reached by web content; the program changes CI, manifests' metadata tables, one tool crate, and the
code fixes PR-A* make (whose own plans run this audit).

## §4 Decisions

### §4.1 Axis (i) — where profiles are declared, how the matrix is derived

- **Declaration**: `[package.metadata.elidex.feature-profiles]` (NEW) in the member's own `Cargo.toml`, next to its
  `[features]`. One entry per named profile: its feature list and, optionally, its declared absences (I5); plus one
  `test-only` list of the member's test-only features. Exact TOML schema is PR-M1's plan (PR-M2 adds the absence and
  test-only keys). Co-location is the reason: the features and their supported combinations change in the same diff,
  and a workspace-level table would be a second place a feature author must remember.
- **What gets declared at the base** (by PR-M1):
  - `elidex-js`: `browser = ["engine", "compat-webapi"]` and `app = ["engine"]` (names from
    `crates/script/elidex-js/Cargo.toml:15-17`), plus its implicit `default` row (`default = []`,
    `crates/script/elidex-js/Cargo.toml:14`) — three rows: `default` / `app` / `browser`. The `default` row is a
    supported surface under F1-b, the interim of the F1 decision (§5).
  - `elidex-style`: depends on PR-A2's outcome for `parallel`.
  - The four other feature-declaring members (`cargo metadata --no-deps --format-version 1` → 6 of 54 packages
    have a non-empty `features` map, verified 2026-11-08: elidex-ecs {fixtures}, elidex-script-session
    {web-storage}, elidex-storage-core {web-storage}, elidex-net {test-hooks}, elidex-style {parallel, rayon},
    elidex-js) need no named profile: `web-storage` is reached through `elidex-js`'s browser row, and `fixtures` /
    `test-hooks` are test-only (PR-M2 declares them so).
- **Derivation**: at run time, by one tracked entry point that reads `cargo metadata --format-version 1`
  (`workspace_members`, `packages[].metadata.elidex`), emits the rows, and runs them serially. No row list is ever
  checked in (ci.yml carries no per-row YAML). **Implementation home**: a small Rust tool crate under
  `crates/tools/` (NEW) — the job already declares the Rust toolchain (the CLAUDE.md `## CI` job-boundary rule: a
  check needs only the runtimes its job declares), the crate is inside the `rust` path filter, and it is itself a
  workspace member, so it is in its own population. PR-M1's plan may instead justify an off-the-shelf runner; the
  constraint it must meet is I1–I3 and I7 (the runner reads *this* declaration, one invocation per row, same entry
  point locally and in CI). Whether an existing runner reads a `package.metadata` declaration is unverified (Q6).

### §4.2 Axis (ii) — unification, and #585's step: **absorbed (retired), not kept**

**Decision**: PR-M1 deletes ci.yml's `Clippy (default features, no dev-only feature)` step and mise's
`check-default`, in the same PR that adds the rows (I6×I8). There is no interval with both, and none with neither.

**Why absorbing loses no feature coverage**: #585's step compiles each crate with the features some member's normal
edges enable. Every such (crate, feature) pair is compiled by that member's own `default` row, which builds the same
normal edges in isolation. So per (crate, feature) pair the rows ⊇ #585's step; and they add each member's own
configuration, which #585 cannot see.

**What absorbing gives up** (stated, not hidden): a failure that appears only when features requested by *different*
members meet in one build — the workspace union. No consumer ships that configuration (an embedder builds one
member), and Cargo requires features to be additive (Cargo Book §3.4 #feature-unification), so such a failure is a
non-additive-feature bug; the `check` job's `--all-features` build remains the maximal union and still compiles
together every feature any member exposes. The intermediate unions are not covered by anything, which is what Cargo
Book §3.4 #feature-combinations says every project must choose.

**The fixture-boundary step** (`ci.yml:114-118`, `scripts/check-fixture-boundary.sh`): its own scope is "the
workspace's default-features build on the host target" (script header lines 6-8), computed by its own `cargo tree`,
so it stays valid when PR-M1 retires #585's step; PR-M1 rewrites its comment (it points at "the comment above",
`ci.yml:114`). PR-M2 then moves its assertion into I5 — per row, per OS, over every declared test-only feature —
and deletes the script, its step and its filter line (`ci.yml:39`). That is the generic guard
`#11-test-only-feature-boundary-guard` books (slot text: take the population from cargo, fail closed on unknown
features, cover non-host targets, watch product features no member enables), so PR-M2 discharges that slot, and the
slot's "attaches to the default-features clippy step" is superseded by this decision. The slot's remedy is recorded
as unreviewed; PR-M2's plan-review is its first review.

### §4.3 Axis (iii) — CI shape and cost

- **Job**: a new job `feature-profiles` (NEW), `needs: changes`, gated like `check`
  (`if: needs.changes.outputs.rust == 'true' || github.event_name == 'push'`; `ci.yml:59`) — it compiles Rust, its
  inputs (manifests, the tool crate) are all inside the `rust` filter, so the tamper-path argument that keeps
  `trip-wires` / `tools` ungated does not arise. Not a step of `check`: a separate job runs in parallel with `check`
  instead of lengthening it, and gets its own rust-cache key (I3×I7).
- **OS legs**: `ubuntu-latest`, `macos-latest`, `windows-latest`, `fail-fast: false` — the same set as `check`
  (`ci.yml:61-65`), for #585's own reason (`ci.yml:105-107`: default-feature builds differ per target because
  `cfg(target_os/windows/unix)` sites exist). Not a GitHub matrix over rows: per-row legs would each cold-build the
  shared dependency graph.
- **mise parity**: `mise run check-profiles` (NEW) runs the same entry point; `[tasks.ci]` depends on it instead of
  `check-default`. Local runs cover the host OS only; the other two legs are CI-only (stated in the task's comment).
- **Timeout**: derived the way ci.yml's `tools` job does (`ci.yml:250-269`): PR-M1 sets `timeout-minutes` to
  `ceil(4 × slowest leg's end-to-end minutes)` of its own run and records run id and leg time in the job's comment;
  any later PR that adds rows (members or profiles) or row work (PR-M2's resolve checks) re-derives it from its own
  run, raise-only. The `check` job's `timeout-minutes: 20` (`ci.yml:60`) is not touched.
- **Cost**: unmeasured (Q8). Each row is an incremental clippy whose dependency artifacts are shared whenever the
  feature set matches; only members whose resolve differs from rows already built pay. PR-M1's plan measures it on
  the census; it is judgment input, not a reason to shrink the population (I2).

### §4.4 Axis (iv) — cross-lane ordering

1. **F1 decided** (PM 520, §5): F1-b is the interim, so PR-A1 has the F1-b shape and elidex-js declares three rows.
2. **Census C0** (§7) on the then-current base — names every red row and every I4/I5 violation, with owners.
3. **PR-A1** (VM P4 lane) and **PR-A2** (CSS owner of `elidex-style`) and any **PR-A3…** the census finds, each in
   its owning lane, in any order among themselves.
4. **PR-M1** (CI/tools) — reviewed and ready *before* PR-A1 lands; lands in a PM-held landing window directly after
   the last PR-A, after a fresh three-OS run on the new head. Reason: the VM P4 lane lands into `elidex-js` weekly,
   and the default configuration has been red for six months with no gate (§1); every day between PR-A1 and PR-M1 is
   a day it can go red again unseen.
5. **PR-M2** (CI/tools), after PR-M1 and after PR-A2.
6. **PR-T** (§6), after PR-M2.

Coordination owed: the VM P4 lane (its PRs are gated by new rows from PR-M1 on; PR-A1 is in its files), the Layout
lane (C0b's fixture script is retired by PR-M2; its `pending-at-landing/c0-program.md` may cite it — Q4), the owner of
the A2 Web-Storage demotion (PR-M2 encodes their absence guarantee — Q9).

### §4.5 Naming

"Feature profile" everywhere this program writes (declaration key, job, task, comments), because Cargo's "profile" is
the build profile (Cargo Book §3.5) and the `check` job already passes nextest a `--profile` (`ci.yml:90`).

## §5 F1 — is `elidex-js` without `engine` a supported surface? (DECIDED: F1-c, F1-b interim — PM 520)

The fork, its derivation and its options are kept below as the record of why this was a fork and what was chosen
between; the decision is at the end of this section.

**Why it was a fork here.** I2 makes every member's `default` a supported row, so the matrix cannot land while
`elidex-js`'s `default` is red: either the no-engine build is made green, or its shape changes. PM decision 4 reserves
the second. The question already has a slot with a recorded fork, `#11-elidex-js-no-default-features-support`
("(1) engine-always … recommended — no parser-only consumer exists vs (2) truly support parser-only"), whose trigger
("S5 boa→VM cutover … OR a parser-only consumer appears") has fired: `elidex-shell` now depends on `elidex-js`
(`crates/shell/elidex-shell/Cargo.toml:31`).

**Derivation by the authority order.**

1. **Spec.** ECMA-262 §4.2 Hosts and Implementations defines the language with host-defined facilities deferred to an
   external source through host hooks; WHATWG HTML §8.1.6 is that host for the web. So the language/host seam is a
   specification structure. The specification says nothing about whether an implementation must be *buildable*
   without its host. **Silent on the question.**
2. **ECS-native principles.** They pull two ways.
   - *Toward a host-less build*: ScriptSession is the sole Script↔ECS boundary, and the VM's host layer is the
     engine-bound layer (Layering mandate). A build of the language core with no ECS crate linked is a structural
     proof that the core reaches ECS only through that layer — structure, not review convention. The code was
     written toward it: 187 `cfg(not(feature = "engine"))` sites
     (`git grep -n 'not(feature = "engine")' -- crates | wc -l`; 179 of them in `vm/gc/collect.rs`).
   - *Toward engine-always*: no consumer exists (every product configuration in the design has `engine`: App =
     `engine`, browser = `engine` + `compat-webapi`); dead configuration is deleted, not kept (Design discipline);
     One issue, one way counts the decision tax — 1996 `feature = "engine"` occurrences in 420 files
     (`git grep -c 'feature = "engine"' -- crates/script/elidex-js | awk -F: '{s+=$2} END{print s}'`;
     `git grep -l … | wc -l`), every new VM site asks "does this need the gate?", and after PR-M1 every VM PR is
     gated by a configuration nobody uses.
   - And the *ideal form* of the first reading is not the current shape: if the language/host boundary is worth
     enforcing, the clean mechanism is a crate boundary (language crate + host crate, with a host-hook seam that
     mirrors ECMA-262 §4.2), enforced by Cargo's dependency graph, not ~2000 `cfg` attributes. That is also a shape
     change. **Does not converge**: the two principled endpoints are both feature/crate-shape changes, and the
     status-quo shape (fix the gating, keep `default = []`) is the endpoint of neither.
3. **Measurement (recorded as evidence).** The no-engine build has not compiled since at least #177 (2026-05-10, §1);
   the line that blocks it is in a `cfg(not(feature = "engine"))` branch — code written *for* that build
   (`crates/script/elidex-js/src/vm/gc/collect.rs:45-49` names `elidex_ecs::Entity`, while `elidex-ecs` is an
   optional dependency). The core VM's construction takes `elidex_plugin::EngineMode`
   (`crates/script/elidex-js/src/vm/init.rs:11`, ungated, `elidex-plugin` optional), so even a pure gating fix must
   choose between gating `EngineMode` out of the core and making `elidex-plugin` non-optional — the latter is a
   shape change (it removes the implicit `elidex-plugin` feature). No parser-only or host-less consumer exists in the
   workspace (`git grep -n 'elidex-js' -- '*Cargo.toml'` → root `:214` and `elidex-shell :31` only).

**Options** (as put to PM in draft 1).

- **F1-a — engine-always** (not taken). `engine`'s dependencies become non-optional, the `engine` feature and every
  `cfg(feature = "engine")` / `cfg(not(feature = "engine"))` go, `compat-webapi` stays (App = default, browser =
  `compat-webapi`). Shape change. PR-A1 = that refactor (VM P4 lane; mechanical but wide: 14 of the 420 files are
  over 1000 lines, so the touch-time split rule applies to each it touches —
  `for f in $(git grep -l 'feature = "engine"' -- crates/script/elidex-js); do n=$(wc -l <$f); [ $n -gt 1000 ] && echo $f; done`).
  Gives up: the (never-enforced) compile-time proof that the core is host-free; the `cfg` annotations that map the
  seam a future crate split would need.
- **F1-b — support the host-less build as is** (taken as the interim). Keep `default = []`, fix the gating. Not a
  shape change *if* every fix is a gate; if PR-A1's plan finds a site whose right fix is a non-optional dependency
  (candidate: `EngineMode`, above), that sub-question returns to PM. PR-A1 = the targeted fix (the 15 named
  diagnostics of §9, plus whatever later compiler phases surface once name resolution passes — the count is a lower
  bound).
- **F1-c — split the crate** (language crate + host crate) (taken). Shape change and a program of its own; listed
  so the choice is made knowingly, not because this umbrella can carry it.

**Decision (PM 520; decided by the user, relayed by PM).** **F1-c.** Draft 1 recommended F1-a; that recommendation
was not taken and is withdrawn — it is recorded here only as what was put to PM.

- **The split is not this umbrella's.** By the contingent path draft 1 stated, F1-c is its own umbrella. It is owned
  by the VM P4 lane and is outside U0's scope: U0 does not plan, scope or schedule it. It is the external program
  that will eventually replace F1-b's `cfg` gating.
- **F1-b is the interim (bridge).** Until the split lands, `elidex-js` keeps `default = []` and the `default` row is
  a supported row like any other (I2). PR-A1 (VM P4 lane, §6) makes that row compile by fixing the gating leaks
  (§9's diagnostics and whatever later compiler phases surface).
- **Return-to-PM boundary.** A fix that is a gate is PR-A1's to make. A site whose right fix is making a dependency
  non-optional — a shape change — is not decided in PR-A1 and not in this umbrella: PR-A1's plan names it and
  returns it to PM. The one candidate known now: `elidex_plugin::EngineMode`, used ungated by the core VM's
  construction (`crates/script/elidex-js/src/vm/init.rs:11`), with `elidex-plugin` optional
  (`crates/script/elidex-js/Cargo.toml:80`). How PR-A1's acceptance treats such a site while PM's ruling is pending
  is §6.
- **What the interim carries** (stated, not argued): after PR-M1, every `elidex-js` PR is also gated by the
  `default` row, and the `cfg(feature = "engine")` decision tax counted in step (2) stays until the split replaces it.
  In return, the property the host-less build proves — the language core names no ECS / host type outside the
  `engine`-gated code — is checked structurally by that row from PR-M1 on.
- **Retired with F1-a**: draft 1's condition on F1-a (a structural guard for host-freedom once `engine` is gone,
  possibly a new `trip-wires` entry) and its question Q5 no longer apply — under F1-b the `default` row is that
  guard. §7 records Q5 as retired.

## §6 PR list

| PR | Owner | Scope | Acceptance | After |
|---|---|---|---|---|
| **PR-A1** | VM P4 lane (`vm/lifecycle.rs`, `vm/init.rs`, `vm/gc/collect.rs`, `vm/natives_json/mod.rs`) | F1-b shape only (§5): fix the gating leaks so `elidex-js`'s `default` (`default = []`) row compiles, keeping `app` and `browser` green, on three OS. Every fix is a gate; a site whose right fix is a non-optional dependency (candidate: `EngineMode` / `elidex-plugin`, `vm/init.rs:11`) is named in the plan and returned to PM, not decided here. `vm/gc/collect.rs` is 2074 lines (`wc -l`): the touch-time split decision precedes the fix. | Relative to the slot (PM 336): each diagnostic named in §9 is gone, each one newly surfaced is named in the PR body and gone; every elidex-js row (`default` / `app` / `browser`) exits 0 under `clippy -D warnings` on the PR's three-OS run (run by hand in that PR; the job does not exist yet). A returned-to-PM site does not land open: PR-A1 lands only after PM has ruled on each such site and the ruling is applied — in PR-A1 if it is a gate, otherwise in the PR PM names — so no row is skipped (I2, I8). | F1 (decided, PM 520); PM's ruling on each returned site |
| **PR-A2** | CSS lane (owner of `elidex-style`) | `parallel` is enabled by no member: either declare a profile and make it green (connect), or remove the feature (delete — a shape change, to PM). | I4 holds for `elidex-style` at PR-M2's base. | C0 |
| **PR-A3…** | Lane owning each module the census finds red | One PR per owning lane, same rule as PR-A1. | Same form as PR-A1. | C0 |
| **PR-M1** | CI/tools | Declaration (I1) for the base; the deriver tool crate; job `feature-profiles` on three OS; `mise run check-profiles`; retire #585's step and `check-default` (I6); rewrite the fixture step's comment and the profile prose in the three Cargo.toml comments into pointers (incl. the stale `crates/script/elidex-js/Cargo.toml:20-22`, §9); timeout derived from its run. | Every row green on the PR's own three-OS run; the run produced ≥ 1 row per workspace member; a control: a scratch commit that breaks one profile only (e.g. an ungated engine-only import) turns exactly that row red while `check` stays green — recorded in the PR body, not merged. | all PR-A |
| **PR-M2** | CI/tools (consult: Layout lane for the fixture script, A2 owner for the absence) | `test-only` and per-profile absence declarations; I4 completeness and I5 forbidden checks per row; absorb and delete `scripts/check-fixture-boundary.sh` (+ step, filter line); discharge `#11-test-only-feature-boundary-guard`. | Controls, each recorded: a normal edge enabling `fixtures` → red; `web-storage` reached in the App row → red; an undeclared, unenabled feature → red; the browser row covering `web-storage` while App forbids it → green. Timeout re-derived. | PR-M1, PR-A2 |
| **PR-T** | CI/tools (job) + VM P4 (test gating) | Defect: a non-full profile's own behaviour (code compiled out, e.g. the compat ceiling with `compat-webapi` off, `vm/init.rs`; and the 179 not-engine branches of `vm/gc/collect.rs`, `git grep -c 'not(feature = "engine")' -- crates/script/elidex-js/src/vm/gc/collect.rs`) is executed by no test in that profile. Run each declared profile's tests under that profile; its plan first runs the test-target census (Q7) and splits fix-first PRs by the same (a)-before-(b) rule. | Its own plan. | PR-M2 |

Every row's PR plan goes through `/elidex-plan-review` before implementation (PM decision 1). PR-A1's plan is the
VM P4 lane's; this umbrella fixes only its acceptance and its return-to-PM boundary (§5).

## §7 Census C0 and open questions

**C0** (measurement, not a PR): on the base PR-M1 will be cut from, every row of §4.1's declaration, on macOS locally
and on Linux/Windows through PR-M1's first draft CI run in report-only mode; plus I4/I5 over each row's resolve. Output:
the red rows with their diagnostics and owners → the PR-A3… list. Until C0 runs, the only measured row set is
elidex-js's three (§9) — **1 of 54 members measured**.

Questions (for scout / PM; none is answered in this memo — Q3 and Q5 are retired by PM 520, kept so the numbering
holds):

- **Q1** C0 itself (above).
- **Q2** (PM) Where PR-A1 (F1-b shape, §6) sits in the VM P4 lane's queue (guard memo, S1, call-depth guard, …), and
  whether any in-flight VM P4 branch touches the four files of PR-A1.
- **Q3** — retired: F1 is decided (PM 520), so the merge it proposed is superseded by PM 531 (§8).
- **Q4** (scout) Does the Layout lane's C0 program (`pending-at-landing/c0-program.md`, C0c in flight) cite
  `scripts/check-fixture-boundary.sh` or ci.yml `:111-118` in a way PR-M1/M2 must update?
- **Q5** — retired: it served F1-a's condition only, and F1-a was not taken (§5).
- **Q6** (scout) Can an off-the-shelf runner read a `package.metadata` declaration, or only CLI flags? (PR-M1's
  implementation choice.)
- **Q7** (scout) Does `cargo test -p elidex-js --no-run` compile with each declared feature list? (PR-T.)
- **Q8** (PR-M1) Row cost per OS leg on the census base.
- **Q9** (PM) Who owns the A2 absence guarantee today (`docs/plans/2026-06-web-api-compat-a2-storage-demotion-plan.md`
  §(C)) and is encoding it as a declared absence acceptable to them?

## §8 Slots

- `#11-elidex-js-default-features-build-broken` — discharged when PR-M1 lands (the gate) with every PR-A landed (the
  code). Its "Update (PM 498)" line is wrong on the error locations (§9); the ledger needs the correction.
- `#11-elidex-js-no-default-features-support` — stays open, re-homed to the VM P4 lane, its defect restated as the
  language-core/host boundary being held only by `cfg` annotations that are themselves broken, and closed when the
  crate-split umbrella lands (PM 531; the ledger rewrite is PMO's).
- `#11-test-only-feature-boundary-guard` — discharged by PR-M2 (§4.2); its attach point moves from the retired #585
  step to the matrix.
- No new slot: PR-T is in this program, not deferred.

## §9 Brief claims found false or stale (measured at `69fbe61f`)

- **"15 errors, all in `vm/lifecycle.rs`"** — false. 15 errors: 13 in `crates/script/elidex-js/src/vm/lifecycle.rs`
  (E0433 at `:45`, E0599 at `:34` and `:45`, E0609 at `:46-47`, `:51-54`, `:59-61`, `:63`), **1 E0432 at
  `crates/script/elidex-js/src/vm/init.rs:11`** (`elidex_plugin`), **1 E0433 at
  `crates/script/elidex-js/src/vm/gc/collect.rs:47`** (`elidex_ecs`); plus the 2 unused-import warnings at
  `crates/script/elidex-js/src/vm/natives_json/mod.rs:13` and `:15`, red under `-D warnings`. Source: the driver's
  own log `prof-default.log` (`grep -nE 'error|warning|-->'`). The slot ledger's PM 498 line repeats the false form.
- **`crates/script/elidex-js/Cargo.toml:20-22`** ("No such embedder wires `elidex-js` yet — the shell still runs the
  boa engine") — stale: `crates/shell/elidex-shell/Cargo.toml:31` enables `engine` + `compat-webapi`. PR-M1 replaces
  the passage with a pointer to the declaration.
- Everything else the brief states held when re-checked: 6 of 54 packages declare features; the profile prose lines;
  #585's step at `ci.yml:111-112` and `mise.toml:14-19`, in `[tasks.ci]` at `mise.toml:142`.

## §10 What this does not buy

- Union-only, non-additive feature failures (§4.2).
- Targets other than the three OS legs.
- `cargo doc` per profile: the `doc` job stays `--all-features`.
- Merge blocking: `main-protection` has no required status check, so a red `feature-profiles` job is red, not
  blocking — the eyeball-before-squash convention remains the final gate.
