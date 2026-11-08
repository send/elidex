# Umbrella U0 — per-feature-profile build matrix (draft 3, 2026-11-08)

## §0 Status

**Lane**: CI/tools (gate policy). **Base**: `origin/main` at **`69fbe61f`** (#609, 2026-11-08). Every path, line and
count below was read at that commit; the command that produced each count is printed next to it.

**Slots**: `#11-elidex-js-default-features-build-broken` (owner CI/tools; this umbrella is its program) and
`#11-elidex-js-no-default-features-support` (the "is the no-engine build a supported surface" question; answered by
PM 520, §5; re-homed to VM P4 by PM 531, §8). `#11-test-only-feature-boundary-guard` is coupled
(§4.2, PR-M2a). `#11-vm-typed-array-family-layering-and-gate` (VM P4) is affected: a premise of it ends when PR-A1
lands (§5, §8).

**What this memo is**: the umbrella of an edge-dense program — PM 498 ("umbrella U0（designer）+ PR ごとの plan、
実装前に /elidex-plan-review"): the decisions on the four axes PM 501 fixed ("4 軸 = (i)…(iv)"), the coupled
invariants (§2), the PR list with owners, scope, acceptance and order (§6). Per-PR implementation detail belongs to
each PR's own plan, and each PR plan goes through `/elidex-plan-review` before implementation.

**F1 decided (PM 520, user decision).** Whether `elidex-js` built without `engine` is a supported surface was a
feature-shape question, which PM 498 reserved ("収束しないか feature の形を変える設計変更なら PM へ"); §5 keeps the
derivation as the record. The user chose **F1-c** (split the crate). Per §5's contingent path, the split is its own
umbrella, owned by the VM P4 lane and **outside this umbrella's scope**; **F1-b** (keep `default = []`, fix the
gating) is the interim. So elidex-js declares three rows (§4.1) and PR-A1 has the F1-b shape only (§6): it relocates
engine-bound code into the host layer, gates in place only where the code is core, and returns to PM any site whose
right fix is a non-optional dependency (§5).

**What a row asserts** (stated once here, used everywhere): a row's lib and bin targets compile under
`clippy -D warnings` in that configuration — **cfg hygiene**. A row does not assert that the configuration is
ES-complete, nor that the code is in the right layer (§2 I2, §5).

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
every feature a member exposes is classified (product or test-only) and every product feature is compiled by some
profile; test-only features, unclassified features and per-profile absences are asserted on each profile's own
resolve; and there is exactly one mechanism for "which configurations does CI compile" — the `--all-features` test
configuration stays separate because it is a different property (dev-only features on, test targets), not a second
answer to this one.

## §2 Coupled invariants

Eight invariants; the pair table holds all 28 pairs.

- **I1 — one declaration.** Feature profiles are declared once per member in that member's `Cargo.toml` under
  `[package.metadata.elidex.feature-profiles]` (NEW) — the table Cargo ignores and leaves to tools (Cargo Book §3.1
  #the-metadata-table: "completely ignored by Cargo … can be used for tools"). Tools read it through `cargo metadata`,
  which returns it as `packages[].metadata` (Cargo Book §4.3.5 #json-format), always called with
  `--format-version 1` (§4.3.5: "It is recommended to include the --format-version flag"). A member's `default`
  feature set is its implicit `default` profile and needs no entry. Test-only features are declared in the same
  table (I5). The prose comments that describe profiles today become pointers to the declaration, not restatements.
- **I2 — derived population; what a row asserts.** The rows are `workspace_members` (from `cargo metadata`) ×
  ({`default`} ∪ declared profiles) × OS legs. Never a list in YAML or a script. A row asserts cfg hygiene (§0): its
  lib and bins compile clean in that configuration. It does not assert ES completeness of the configuration (the
  no-engine `default` row has no typed-array constructors, §5) or layer correctness (§5); behaviour per profile is
  PR-T's. A member cannot opt its `default` out: a default that should not be a declared configuration is a
  feature-shape question (PM 498, ruled for elidex-js by PM 520), not a skipped row. An unparseable declaration, an
  unknown key, or a profile naming a feature the member does not have is red, never skipped. Non-vacuity: the run
  asserts it produced at least one row per workspace member.
- **I3 — isolation.** One Cargo invocation per row (`-p <member>`; named profiles via `--no-default-features
  --features <list>`), lib + bin targets only. Under resolver 2 (`Cargo.toml:2`) a lib/bin build activates no
  dev-dependency feature (Cargo Book §3.4 #feature-resolver-version-2), so a row is the member's production
  configuration; one invocation per row is what the Cargo Book names as the way to avoid cross-package unification.
  `clippy -- -D warnings`, so a warning that exists only in that configuration (e.g. an import left unused) is red.
  The row's compile is the authoritative check of what builds; I4/I5 read the resolve, which is an approximation
  of it (I5).
- **I4 — completeness.** Every feature in a member's `[features]` — Cargo's implicit optional-dependency features
  (value exactly `["dep:<name>"]` in `cargo metadata`) included; excluding only the `default` key and declared
  test-only features — is active in the resolve of at least one row. Measured on each row's own resolve, never on the
  workspace resolve. This also makes every unclassified feature that no row activates red.
- **I5 — forbidden and unknown features (fail-safe).** Each feature of a workspace member is classified from
  `cargo metadata` (manifest data, exact): **product** = reachable from some member's declared profiles (its
  `default` included) through `[features]` definitions, across members (`"dep/feat"`, `"dep?/feat"`), but not
  through a dependency declaration's `features = [...]`; **test-only** = declared test-only by its member; a feature
  in both is a declaration error (red). In each row's resolve: an active test-only feature is red; an active feature
  in the profile's declared absences is red; an active feature that is neither product nor test-only is **unknown →
  red** — implicit optional-dependency features included (the slot's `foo-test-support` enabled from a normal edge
  is this case). App's absences are both A2 edges: `elidex-storage-core/web-storage` and
  `elidex-script-session/web-storage` (`crates/script/elidex-js/Cargo.toml:35-38`). Third-party crates' features are
  outside the population (no member declares them). **What I5 can assert**: the resolve is read with `cargo tree`,
  which "doesn't guarantee the exact equivalence to what Cargo is going to build" (Cargo Book §4.3.8
  #feature-unification). So I5 is an approximation of the built configuration, fail-closed on what it reads: a cargo
  failure, an output with no feature nodes (the non-vacuity check `scripts/check-fixture-boundary.sh:23-28` does
  today), or a line the reader cannot parse is red. It asserts what Cargo's resolver reports as activated for that
  row's invocation; the row's compile (I3) remains the authority for what builds.
- **I6 — one mechanism.** After the program, the configurations CI compiles without dev-only features are exactly
  the matrix rows. #585's step and `mise check-default` are retired in the same PR that adds the rows (PR-M1);
  `scripts/check-fixture-boundary.sh` is absorbed into I5 (PR-M2a). The `check` job's `--all-features` steps stay:
  they are the test configuration (dev-only features on, `--all-targets`), a different property.
- **I7 — CI shape, parity, cost.** A dedicated job `feature-profiles` (NEW) on the same three OS as `check`, rows
  run serially inside one entry point; `mise run check-profiles` (NEW) runs the same entry point (host OS only);
  `timeout-minutes` derived from the job's own run (§4.3).
- **I8 — land green.** No PR of the program lands red, and the matrix PR lands only on a base where every row is
  green on all three OS. Adding a profile to the declaration later is adding a row: the PR that adds it makes it
  green (or carries the fix) in the same PR.

| Pair | Intersection |
|---|---|
| I1×I2 | The deriver reads only `cargo metadata --format-version 1`; a malformed declaration is a red row, not a dropped one. |
| I1×I3 | A profile is a feature list for one `--no-default-features --features` invocation; the declaration cannot express cross-member unification, by construction. |
| I1×I4 | Completeness is computed against every `[features]` key (implicit ones included) minus `default` and declared test-only ones; a new feature with no covering row is red in the PR that adds it, which forces its declaration there. |
| I1×I5 | Classification is computed from the declaration alone (profiles' `[features]` closure; `test-only` lists), never from which edges happen to enable a feature; the fixture script's hard-coded `elidex-ecs` / `fixtures` literal becomes a `test-only` entry. |
| I1×I6 | The Cargo.toml prose (§1 list) becomes a pointer to the declaration; profile semantics are not restated in comments. |
| I1×I7 | The declaration lives in `Cargo.toml` files, already inside ci.yml's `rust` path filter (`ci.yml:34-48`, `crates/**`). |
| I1×I8 | Declaring a profile adds rows on three OS; the declaring PR owns their green. |
| I2×I3 | Population × isolation = invocation count = cost; the only permitted reduction is de-duplicating rows whose (member, resolved feature set, OS) is identical, computed per run, never listed. |
| I2×I4 | Completeness is only as strong as the population; the non-vacuity assertion (≥ 1 row per member) stops a silently shrinking derivation from making I4 vacuous. |
| I2×I5 | Forbidden checks run per row on each OS leg; `cargo tree`'s `--target` defaults to the host platform (Cargo Book §4.3.8 #option-cargo-tree---target), so the three legs see three hosts' target-specific edges (the fixture script today sees one host per leg too, but through a workspace union). |
| I2×I6 | The workspace-union configuration is not a row (§4.2 says what that gives up). |
| I2×I7 | Row count × three legs drives the timeout; a PR adding members or profiles re-derives it (§4.3). |
| I2×I8 | The census (C0, §7) is the population evaluated on the base before PR-M1; PR-M1's own three-OS run is the land-green evidence. |
| I3×I4 | Each row's resolve is read from that row's own invocation (`cargo tree -p <member> -e normal,build,features` with the row's feature flags); reading the workspace resolve (`cargo metadata`'s `resolve` is "for the entire workspace", §4.3.5 #json-format) would re-hide what unification hides. |
| I3×I5 | Same reader. Edge kinds are explicit: `cargo tree`'s default is `normal,build,dev` (§4.3.8 #option-cargo-tree---edges), and `-e features` alone adds the other kinds; measured on the pinned 1.99.1: `cargo tree -p elidex-js --features engine,compat-webapi -e features` is byte-identical to `-e normal,build,dev,features` and contains an `elidex-ecs feature "fixtures"` node (dev edge), `-e normal,build,features` contains none — so a row reading `-e features` would be falsely red on every member with a `fixtures` dev-dependency. The fixture script already uses `-e normal,build,features` (`scripts/check-fixture-boundary.sh:17`). |
| I3×I6 | The `--all-features` steps are not rows: they turn dev-only features on for test targets. |
| I3×I7 | Serial invocations share one target dir; that Cargo keys artifacts on the feature set (so rows do not rebuild or contaminate each other) is not stated in the Cargo Book (§3.9 Build Cache does not mention features); PR-M1 measures it on its run (a second row with a different feature set leaves the first row's artifacts fresh). A separate job gets its own rust-cache key, so its feature sets do not evict `check`'s. |
| I3×I8 | A row's verdict does not depend on row order (each row is its own invocation; artifact reuse is PR-M1's measurement, I3×I7), so a green census predicts a green PR-M1 run up to base drift. |
| I4×I5 | A feature can be required by I4 and forbidden by I5 at once only per profile: App forbids both `web-storage` features, the browser row covers them. Both hold; PR-M2b's controls include exactly this case. |
| I4×I6 | I4 is strictly stronger than #585's step: #585 compiled features some member enables; I4 also reaches features no member enables (`elidex-style/parallel`). |
| I4×I7 | One `cargo tree` per row, inside the same entry point, shared with I5. |
| I4×I8 | `elidex-style/parallel` (and its implicit `rayon`) is uncovered today, so I4 is red at the base: it needs its fix (declare + green, or remove) before I4 lands. So I4 is its own PR, PR-M2b, after PR-A2; I5 (PR-M2a) does not wait for it (§4.4). |
| I5×I6 | PR-M2a retires `scripts/check-fixture-boundary.sh` and its ci.yml step and path-filter line, moving the assertion into I5; `#11-test-only-feature-boundary-guard` attaches here instead of to the retired #585 step. |
| I5×I7 | The forbidden and unknown checks run on every OS leg. |
| I5×I8 | At PR-M2a's base the App row must satisfy both A2 absences (census C0 includes it), and no active feature may be unknown. At `69fbe61f` no member's dependency declaration or `[features]` value names another member's implicit feature, and `engine` lists all 25 of `elidex-js`'s implicit features (`cargo metadata --no-deps --format-version 1`, `packages[].dependencies[].features` and `packages[].features`; 26 implicit features in total: `elidex-js` 25, `elidex-style` `rayon`), so the unknown rule is green there by measurement of the manifests; C0 confirms it on the resolves. |
| I6×I7 | `[tasks.ci]` (`mise.toml:142`) replaces `check-default` with `check-profiles` in the same PR. |
| I6×I8 | Retiring #585's step and adding the rows are one PR: no commit on `main` has neither, or both. |
| I7×I8 | PR-M1's own three-OS run is both its land-green evidence and its timeout measurement. |

## §3. Spec coverage map

A CI plan touches no web-platform algorithm. The rows below are the **governing sources** of the design: Cargo's
documented behaviour (the Cargo Book shipped with the pinned toolchain 1.99.1, `rust-toolchain.toml`, read from
`~/.rustup/toolchains/1.99.1-aarch64-apple-darwin/share/doc/rust/html/cargo/`; the § numbers are that book's chapter
numbering from its `toc-0e7001e7.js`, anchors are its heading and option ids) and the two specification sections the
§5 derivation rests on. Honesty notes: (a) `preflight.py` cannot verify the Cargo Book rows — "Cargo Book" is not in
its `SPEC_LABEL_REVERSE`, so those rows are unmapped-label soft-warns; they were checked by hand against the local
book; (b) the repo's own rules (CLAUDE.md, ci.yml comments, PM decisions) govern as much as Cargo does, but they carry
no § number and a row without one is a preflight hard-fail, so they are listed under the table, not in it; (c) "Step"
holds an anchor id, "Branch" the behaviour the design relies on.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Cargo Book §3.4 Features | #feature-unification | a dependency is built with the union of features its dependents enable; features "should be additive" | I3 (one invocation per row), §4.2 | ✓ | no |
| Cargo Book §3.4 Features | #mutually-exclusive-features | "rare cases where features may be mutually incompatible" exist | §4.2 (what absorbing gives up) | ✓ | no |
| Cargo Book §3.3.3 Dependency Resolution | #features | `--workspace` / several `-p` unify; separate invocations do not | I3, I6 (#585's step is a union) | ✓ | no |
| Cargo Book §3.4 Features | #feature-resolver-version-2 | dev-dependencies activate no feature for lib/bin targets | I3 (lib + bins only) | ✓ | no |
| Cargo Book §3.4 Features | #resolver-version-2-command-line-flags | `--features` / `--no-default-features` apply to the `-p` selection | row invocation shape | ✓ | no |
| Cargo Book §3.4 Features | #optional-dependencies | an optional dep without `dep:` defines an implicit feature | I4 population, I5 unknown rule | ✓ | no |
| Cargo Book §3.4 Features | #feature-combinations | full coverage is exponential; the book encourages each project to choose its configurations | I1/I2 (declared profiles, not a powerset) | ✓ | no |
| Cargo Book §3.1 The Manifest Format | #the-metadata-table | `package.metadata` "completely ignored by Cargo", for tools | I1 (declaration home) | ✓ | no |
| Cargo Book §3.5 Profiles | #profiles | "profile" already means build profile | §0 terminology, §4.5 | ✓ | no |
| Cargo Book §4.3.5 cargo metadata | #option-cargo-metadata---format-version | the output format may change; `--format-version` is recommended | I1, I2 (deriver input) | ✓ | no |
| Cargo Book §4.3.5 cargo metadata | #json-format | `packages[].metadata`, `packages[].features`, `workspace_members`; `resolve` is the whole workspace's | I1, I5 classification, I3×I4 | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #feature-unification | "doesn't guarantee the exact equivalence to what Cargo is going to build"; `-e normal,build` is "pretty close" to `cargo build` | I5 (what it can assert), I3 (compile is authoritative) | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #option-cargo-tree---edges | default kinds `normal,build,dev`; `features` alone adds the others | I3×I5 flags | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #option-cargo-tree---target | default is the host platform | I2×I5 (per-leg hosts) | ✓ | no |
| ECMA-262 §4.2 Hosts and Implementations | #sec-hosts-and-implementations | host-defined facilities are deferred to an external source; "Implementations that are not hosts may also provide definitions for host-defined facilities" | §5 derivation step (1) | ✗ | no |
| WHATWG HTML §8.1.6 JavaScript specification host hooks | #javascript-specification-host-hooks | HTML is the host that defines those hooks | §5 derivation step (1) | ✗ | no |

**Breadth**: K=3 sources (preflight count), M=16 rows. The spec rows are ✗ for full enumeration on purpose: §5 uses
them only to establish that the language/host seam is a specification structure, not to implement any hook.

**Repo rules that govern (no § number)**: CLAUDE.md 至上命題 (authority order), Design philosophy (core/compat
layering, plugin-first, ScriptSession as the sole Script↔ECS boundary, Ideal over pragmatic), Layering mandate,
Design discipline (One issue one way, dead code, Supported-surface testing, Edge-dense, 1000-line touch-time split),
`## CI` (CLAUDE.md:93-95: ci.yml's job comments are canonical); ci.yml's job-boundary rule (`ci.yml:236-245`), its
tamper-path reasoning (`ci.yml:182-189`, `:229-231`) and its `tools`-job timeout rule (`ci.yml:250-269`); PM 336
(relative acceptance), PM 410 (matrix direction), PM 498 / PM 501 (this umbrella's frame), PM 520 / PM 531 (F1),
PM 536 (ordering against Python-pin PR-2, citation form).

### §3.1 User-input touch audit

None. No row is reached by web content; the program changes CI, manifests' metadata tables, one tool crate, and the
code fixes PR-A* make (whose own plans run this audit).

## §4 Decisions

### §4.1 Axis (i) — where profiles are declared, how the matrix is derived

- **Declaration**: `[package.metadata.elidex.feature-profiles]` (NEW) in the member's own `Cargo.toml`, next to its
  `[features]`. One entry per named profile: its feature list and, optionally, its declared absences (I5); plus one
  `test-only` list of the member's test-only features. Exact TOML schema is PR-M1's plan (PR-M2a adds the absence and
  test-only keys). Co-location is the reason: the features and their supported combinations change in the same diff,
  and a workspace-level table would be a second place a feature author must remember.
- **What gets declared at the base** (by PR-M1):
  - `elidex-js`: `browser = ["engine", "compat-webapi"]` and `app = ["engine"]` (names from
    `crates/script/elidex-js/Cargo.toml:15-17`), plus its implicit `default` row (`default = []`,
    `crates/script/elidex-js/Cargo.toml:14`) — three rows: `default` / `app` / `browser`. The `default` row is a
    declared configuration under F1-b, the interim of the F1 decision (§5); what it asserts is §0's cfg hygiene.
  - `elidex-style`: depends on PR-A2's outcome for `parallel`.
  - The four other feature-declaring members (`cargo metadata --no-deps --format-version 1` → 6 of 54 packages
    have a non-empty `features` map, verified 2026-11-08: elidex-ecs {fixtures}, elidex-script-session
    {web-storage}, elidex-storage-core {web-storage}, elidex-net {test-hooks}, elidex-style {parallel, rayon},
    elidex-js) need no named profile: both `web-storage` features are product through `elidex-js`'s browser profile
    (`compat-webapi`'s `"…?/web-storage"` values, I5), and `fixtures` / `test-hooks` are test-only (PR-M2a declares
    them so).
- **Derivation**: at run time, by one tracked entry point that reads `cargo metadata --format-version 1`
  (`workspace_members`, `packages[].metadata.elidex`, `packages[].features`), emits the rows, and runs them serially.
  No row list is ever checked in (ci.yml carries no per-row YAML). **Implementation home**: a small Rust tool crate
  under `crates/tools/` (NEW) — the job already declares the Rust toolchain (the job-boundary rule, `ci.yml:236-245`:
  a check needs only the runtimes its job declares), and it is itself a workspace member, so it is in its own
  population. **Entry-point constraint (PR-M1)**: the entry point lives inside the `rust` path filter — the tool
  crate under `crates/**`, invoked as `cargo run -p <tool>` from ci.yml and mise.toml (both already in the filter,
  `ci.yml:46-47`). An entry point anywhere else (a `scripts/*.sh`, a runner config) is registered in the `rust`
  filter in the same PR, as `scripts/check-fixture-boundary.sh` is (`ci.yml:39`); otherwise a PR editing only the
  entry point skips the job that runs it — the tamper path ci.yml's own comments describe (`ci.yml:182-189`). PR-M1's
  plan may instead justify an off-the-shelf runner under the same constraint plus I1–I3 and I7 (the runner reads
  *this* declaration, one invocation per row, same entry point locally and in CI). Whether an existing runner reads a
  `package.metadata` declaration is unverified (Q6).

### §4.2 Axis (ii) — unification, and #585's step: **absorbed (retired), not kept**

**Decision**: PR-M1 deletes ci.yml's `Clippy (default features, no dev-only feature)` step and mise's
`check-default`, in the same PR that adds the rows (I6×I8). There is no interval with both, and none with neither.

**Why absorbing loses no feature coverage**: #585's step compiles each crate with the features some member's normal
edges enable. Every such (crate, feature) pair is compiled by that member's own `default` row, which builds the same
normal edges in isolation. So per (crate, feature) pair the rows ⊇ #585's step; and they add each member's own
configuration, which #585 cannot see.

**What absorbing gives up** (stated, not hidden): a failure that appears only when features requested by *different*
members meet in one build — the workspace union. No consumer ships that configuration (an embedder builds one
member), and the Cargo Book says features "should be additive" (§3.4 #feature-unification) while acknowledging "rare
cases where features may be mutually incompatible" (§3.4 #mutually-exclusive-features). So such a failure is a
non-additive feature pair, which the book asks projects to avoid; the `check` job's `--all-features` build remains
the maximal union and still compiles together every feature any member exposes. The intermediate unions are not
covered by anything; the book leaves that choice to each project ("We encourage you to consider your strategy …
Every project will have different requirements", §3.4 #feature-combinations).

**The fixture-boundary step** (`ci.yml:114-118`, `scripts/check-fixture-boundary.sh`): its own scope is "the
workspace's default-features build on the host target" (script header lines 6-8), computed by its own `cargo tree`,
so it stays valid when PR-M1 retires #585's step; PR-M1 rewrites its comment (it points at "the comment above",
`ci.yml:114`). PR-M2a then moves its assertion into I5 — per row, per OS, over every declared test-only feature and
every unknown feature — and deletes the script, its step and its filter line (`ci.yml:39`). That is the generic guard
`#11-test-only-feature-boundary-guard` books. Item by item against the slot's text (ledger,
`project_open-defer-slots.md:734`): (i) "take the population from cargo" — I5 classifies from `cargo metadata` and
reads activity from `cargo tree`; (ii) "fail closed on unknown ones", including implicit features — I5's unknown rule;
(iii) "cover non-host targets (`--target all` or 3 OS)" — the three OS legs; (iv) "also watch product features that
no member enables (`parallel`)" — I4, which lands in PR-M2b. The slot's defect sentence (no mechanism guaranteeing a
test-only feature is enabled from dev-dependencies only) is closed by (i)–(iii), so PR-M2a discharges the slot and
the ledger points remedy (iv) at PR-M2b. The slot's "attaches to the default-features clippy step" is superseded by
this decision. The slot's remedy is recorded as unreviewed; PR-M2a's plan-review is its first review.

### §4.3 Axis (iii) — CI shape and cost

- **Job**: a new job `feature-profiles` (NEW), `needs: changes`, gated like `check`
  (`if: needs.changes.outputs.rust == 'true' || github.event_name == 'push'`; `ci.yml:59`) — it compiles Rust, and
  under §4.1's entry-point constraint all its inputs (manifests, the tool crate, ci.yml, mise.toml) are inside the
  `rust` filter, so the tamper path that keeps `trip-wires` / `tools` ungated (`ci.yml:182-189`, `:229-231`) does not
  arise. Not a step of `check`: a separate job runs in parallel with `check` instead of lengthening it, and gets its
  own rust-cache key (I3×I7).
- **OS legs**: `ubuntu-latest`, `macos-latest`, `windows-latest`, `fail-fast: false` — the same set as `check`
  (`ci.yml:61-65`), for #585's own reason (`ci.yml:105-107`: default-feature builds differ per target because
  `cfg(target_os/windows/unix)` sites exist). Not a GitHub matrix over rows: per-row legs would each cold-build the
  shared dependency graph.
- **mise parity**: `mise run check-profiles` (NEW) runs the same entry point; `[tasks.ci]` depends on it instead of
  `check-default`. Local runs cover the host OS only; the other two legs are CI-only (stated in the task's comment).
- **Timeout**: derived the way ci.yml's `tools` job does (`ci.yml:250-269`): PR-M1 sets `timeout-minutes` to
  `ceil(4 × slowest leg's end-to-end minutes)` of its own run (the `tools` job's "about 4x", `ci.yml:257`) and
  records run id and leg time in the job's comment; any later PR that adds rows (members or profiles) or row work
  (PR-M2a / PR-M2b's resolve checks) re-derives it from its own run (`ci.yml:250`). Whether a re-derivation may lower
  the value is PR-M1's plan's call: the `tools` job's raise-only rule is program-scoped ("raise-only for the
  program, PM 353", `ci.yml:267`; #586's plan §7), not a general rule. The `check` job's `timeout-minutes: 20`
  (`ci.yml:60`) is not touched.
- **Cost**: unmeasured (Q8). Each row is an incremental clippy whose dependency artifacts are shared whenever the
  feature set matches (PR-M1 measures that reuse, I3×I7); only members whose resolve differs from rows already built
  pay. PR-M1's plan measures it on the census; it is judgment input, not a reason to shrink the population (I2).

### §4.4 Axis (iv) — cross-lane ordering

1. **F1 decided** (PM 520, §5): F1-b is the interim, so PR-A1 has the F1-b shape and elidex-js declares three rows.
2. **Census C0** (§7) on the then-current base — names every red row and every I4/I5 violation, with owners.
3. **Row-fixing PRs**: **PR-A1** (VM P4 lane) and any **PR-A3…** the census finds, each in its owning lane, in any
   order among themselves. **PR-A1′** (exists only if PM rules a returned site's fix to be a non-optional
   dependency; owner and PR named by PM) lands before PR-A1, or PM names PR-A1 itself as the PR that applies it
   (PM 531: PR-A1 does not land before the ruling is applied).
4. **PR-M1** (CI/tools) — reviewed and ready *before* PR-A1 lands; lands in a PM-held landing window directly after
   the later of (a) the last row-fixing PR of step 3 and (b) **Python-pin PR-2**: PR-M1 edits ci.yml, so by
   decision 150 (3) it lands after Python-pin PR-2 (PM 536: "PM 494 は decision 150 (3)…を上書きしていない";
   `docs/plans/2026-10-python-pin-umbrella.md:599-602`). It lands after a fresh three-OS run on the new head.
   Reason for the window: the VM P4 lane lands into `elidex-js` weekly, and the default configuration has been red
   for six months with no gate (§1); every day between PR-A1 and PR-M1 is a day it can go red again unseen.
5. **PR-M2a** (CI/tools; I5), after PR-M1. It needs nothing else: I5 is green at the base by measurement (I5×I8).
6. **PR-A2** (CSS owner of `elidex-style`), after C0. Its connect branch declares a `parallel` profile in PR-M1's
   table, so that branch lands after PR-M1 with its row green (I8); its delete branch is a shape change ruled by PM,
   applied by **PR-A2′** (owner and PR named by PM; exists only if PM rules removal).
7. **PR-M2b** (CI/tools; I4), after PR-M2a (it needs the `test-only` declarations I4 excludes) and after PR-A2 or
   PR-A2′.
8. **PR-T** (§6), after PR-M2b.

**Why I5 and I4 are two PRs** (PR-M2a, PR-M2b): from PR-M1 to the fixture script's absorption, two mechanisms
answer the test-only question (the workspace-union, host-only script and the matrix) — the strangler interval One
issue, one way forbids keeping. Only I4 waits for PR-A2, and PR-A2 can return to PM as a shape change with no date.
One PR with an internal dependency split still lands at its last dependency, so the interval would last until
PR-A2; two PRs bound it by PR-M2a, which depends on PR-M1 alone.

Coordination owed (population: `git grep -n -e 'check-fixture-boundary' -e 'default-features clippy' -e 'default
features, no dev-only' -e 'check-default' -- docs/plans` at `69fbe61f`): the VM P4 lane (its PRs are gated by new rows
from PR-M1 on; PR-A1 is in its files; the typed-array slot's premise, §8); the L3 lane, whose active
`docs/plans/2026-11-range-contents-program.md:326-327`, `:885` names the default-features clippy step and the fixture
script as the enforcing mechanism for RC-2b / A6 — the property survives in the per-member `default` rows and in I5,
but those cites will name retired mechanisms, and the owning lane updates its own memo; the Layout lane (C0b's
fixture script is retired by PR-M2a; its landed memos `2026-10-eh-attrs-c0-insertion-adoption.md` and
`2026-11-eh-attrs-c0b-node-document-fact.md` cite the step as records and are not edited; its landing list does not
cite it — Q4); the owner of the A2 Web-Storage demotion (PR-M2a encodes their absence guarantee — Q9).

### §4.5 Naming

"Feature profile" everywhere this program writes (declaration key, job, task, comments), because Cargo's "profile" is
the build profile (Cargo Book §3.5) and the `check` job already passes nextest a `--profile` (`ci.yml:90`).

## §5 F1 — is `elidex-js` without `engine` a supported surface? (DECIDED: F1-c, F1-b interim — PM 520)

The fork, its derivation and its options are kept below as the record of why this was a fork and what was chosen
between; the decision is at the end of this section. Corrections made at R1 are marked as such; the record is not
otherwise rewritten.

**Why it was a fork here.** I2 makes every member's `default` a declared row, so the matrix cannot land while
`elidex-js`'s `default` is red: either the no-engine build is made green, or its shape changes. PM 498 reserves the
second ("収束しないか feature の形を変える設計変更なら PM へ"). The question already has a slot with a recorded fork,
`#11-elidex-js-no-default-features-support` ("(1) engine-always … recommended — no parser-only consumer exists vs (2)
truly support parser-only"), whose trigger ("S5 boa→VM cutover … OR a parser-only consumer appears") has fired:
`elidex-shell` now depends on `elidex-js` (`crates/shell/elidex-shell/Cargo.toml:31`).

**Derivation by the authority order.**

1. **Spec.** ECMA-262 §4.2 Hosts and Implementations defers host-defined facilities to an external source through
   host hooks; WHATWG HTML §8.1.6 is that host for the web. So the language/host seam is a specification structure.
   §4.2 also says "Implementations that are not hosts may also provide definitions for host-defined facilities" and
   "A host environment is a particular choice of definition for all host-defined facilities" (R1: these two
   sentences were missing). ECMA-262's *host* is the definer of host-defined facilities; elidex's `engine` feature is
   the ECS/DOM wiring, which is not the same boundary — a no-engine build still defines host hooks (e.g. job
   enqueueing) inside the core. The specification says nothing about whether an implementation must be *buildable*
   without its embedder's wiring. **Silent on the question.**
2. **Elidex principles.** They pull two ways. (R1 tier correction: tier (2) of the authority order is the Design
   philosophy section, CLAUDE.md:10. The engine-always reasons below are Design discipline rules — dead code; One
   issue, one way — plus a fact, not Design philosophy principles. Read by tier (2) alone, the derivation leans toward
   enforcing the host boundary, which is what PM 520 chose; "does not converge" below is kept as what draft 1
   recorded, not re-asserted.)
   - *Toward a host-less build*: ScriptSession is the sole Script↔ECS boundary, and the VM's host layer is the
     engine-bound layer (Layering mandate). (R1 correction: a no-engine build that compiles proves only cfg
     hygiene — every ECS/host name outside the `engine`-gated code is gone. It does not prove that the core reaches
     ECS only through the host layer: 207 of the 419 `src` files carrying `feature = "engine"` are outside
     `vm/host/` (`git grep -l 'feature = "engine"' -- crates/script/elidex-js/src | grep -v /vm/host/ | wc -l`;
     without the `grep -v`, 419).) The code carries 187 `cfg(not(feature = "engine"))` sites
     (`git grep -n 'not(feature = "engine")' -- crates | wc -l`), 179 of them in `vm/gc/collect.rs`. (R1 correction:
     these are not host-less behaviour. In `collect.rs`, 176 are `None` fills that keep `GcRoots.proto_roots`'s
     fixed-length array aligned (all inside the array literal at `:59-1089`;
     `git grep -n -A1 'not(feature = "engine")' -- crates/script/elidex-js/src/vm/gc/collect.rs | grep -c 'None,'`),
     2 are the empty `subclass_array_proto_roots` / `subclass_array_ctor_roots` slices (`:1093`, `:1097`), and 1 is the
     empty `active_cssom_rule_ids` map at `:45-49`.)
   - *Toward engine-always*: no consumer exists (every product configuration in the design has `engine`: App =
     `engine`, browser = `engine` + `compat-webapi`); dead configuration is deleted, not kept (Design discipline);
     One issue, one way counts the decision tax — 1996 `feature = "engine"` occurrences in 420 files
     (`git grep -o 'feature = "engine"' -- crates/script/elidex-js | wc -l`; the line count `git grep -c` gives the
     same 1996; `git grep -l 'feature = "engine"' -- crates/script/elidex-js | wc -l`), every new VM site asks "does
     this need the gate?", and after PR-M1 every VM PR is gated by a configuration nobody uses.
   - And the *ideal form* of the first reading is not the current shape: if the language/host boundary is worth
     enforcing, the clean mechanism is a crate boundary (language crate + host crate, with a host-hook seam that
     mirrors ECMA-262 §4.2), enforced by Cargo's dependency graph, not ~2000 `cfg` attributes. That is also a shape
     change. **Does not converge**: the two principled endpoints are both feature/crate-shape changes, and the
     status-quo shape (fix the gating, keep `default = []`) is the endpoint of neither.
3. **Repo measurement (recorded as evidence).** (R1 tier correction: tier (3) is measurement of existing browsers,
   which does not apply to a build-configuration question; what follows is the repo's own measurement.) The
   no-engine build has not compiled since at least #177 (2026-05-10, §1); the line that blocks it is in a
   `cfg(not(feature = "engine"))` branch (`crates/script/elidex-js/src/vm/gc/collect.rs:45-49` names
   `elidex_ecs::Entity`, while `elidex-ecs` is an optional dependency). The core VM's construction imports
   `elidex_plugin::EngineMode` (`crates/script/elidex-js/src/vm/init.rs:11`, ungated, `elidex-plugin` optional). No
   other dependency declaration of `elidex-js` exists than the workspace entry and `elidex-shell`
   (`git grep -n -E '^elidex-js *=' -- '*Cargo.toml'` → root `Cargo.toml:214`, `elidex-shell` `:31`; the plain
   `git grep -n 'elidex-js'` also lists the members entry `Cargo.toml:49` and comment lines).

**Observation for the split umbrella** (recorded, not planned here): `collect.rs`'s 176 `None` fills exist because
DOM interface prototypes (`event_target_prototype`, `node_prototype`, `abort_signal_prototype`, …) are fields of the
core VM struct, rooted positionally by the core GC; a language/host crate split decides where those roots live.

**Options** (as put to PM in draft 1).

- **F1-a — engine-always** (not taken). `engine`'s dependencies become non-optional, the `engine` feature and every
  `cfg(feature = "engine")` / `cfg(not(feature = "engine"))` go, `compat-webapi` stays (App = default, browser =
  `compat-webapi`). Shape change. PR-A1 = that refactor (VM P4 lane; mechanical but wide: 14 of the 420 files are
  over 1000 lines, so the touch-time split rule applies to each it touches —
  `for f in $(git grep -l 'feature = "engine"' -- crates/script/elidex-js); do n=$(wc -l <$f); [ $n -gt 1000 ] && echo $f; done`).
  Gives up: the (never-enforced) compile-time cfg hygiene of the core; the `cfg` annotations that map the seam a
  future crate split would need.
- **F1-b — support the host-less build as is** (taken as the interim). Keep `default = []`, fix the gating. Not a
  shape change *if* no fix makes a dependency non-optional; a site whose right fix is that returns to PM. PR-A1 = the
  targeted fix (the 15 named diagnostics of §9, plus whatever later compiler phases surface once name resolution
  passes — the count is a lower bound).
- **F1-c — split the crate** (language crate + host crate) (taken). Shape change and a program of its own; listed
  so the choice is made knowingly, not because this umbrella can carry it.

**Decision (PM 520; decided by the user, relayed by PM).** **F1-c.** Draft 1 recommended F1-a; that recommendation
was not taken and is withdrawn — it is recorded here only as what was put to PM.

- **The split is not this umbrella's.** By the contingent path draft 1 stated, F1-c is its own umbrella. It is owned
  by the VM P4 lane and is outside U0's scope: U0 does not plan, scope or schedule it. It is the external program
  that will eventually replace F1-b's `cfg` gating.
- **F1-b is the interim (bridge).** Until the split lands, `elidex-js` keeps `default = []` and the `default` row is
  a declared row like any other (I2). PR-A1 (VM P4 lane, §6) makes that row compile (§9's diagnostics and whatever
  later compiler phases surface). The interim is re-evaluated on the slot's Re-eval-date, 2026-11-30 (ledger, §8);
  U0 sets no other date.
- **PR-A1's fix kinds** (Layering mandate). (a) **Relocate** engine-bound code into the host layer — an
  `engine`-gated module under `vm/host/` (or an engine-independent crate where the Layering mandate puts the
  algorithm). The known case: `Vm::unbind`'s body (`crates/script/elidex-js/src/vm/lifecycle.rs:15-69`: mutation
  dispatcher, transient observers, live-range / NodeIterator / TreeWalker / Selection state of `HostData`) is
  engine-bound cleanup in a core module that is not gated (`mod lifecycle;`, `vm/mod.rs:36`), and 13 of §9's 15
  errors are there. (b) **Gate in place** only where the code is correct to keep in the core and merely names a gated
  type. (c) A site whose right fix is making a dependency **non-optional** — a shape change — is not decided in PR-A1
  and not in this umbrella: PR-A1's plan names it and returns it to PM (PM 520). PR-A1's plan carries a Layering
  check: for each fix, which kind it is and, for (a), the host-layer or engine-independent home it maps to.
- **The known (c) candidate, measured**: `elidex_plugin::EngineMode` (`vm/init.rs:11`, `elidex-plugin` optional,
  `crates/script/elidex-js/Cargo.toml:80`). The public `Vm::new()` (`init.rs:30`) is ungated and passes
  `EngineMode::BrowserCompat` to `new_with_scope` (`:31`), which takes `engine_mode: EngineMode` in every build
  (`:163`) but reads it only under `cfg(feature = "engine")` (`:176-177`, `:184`; `:162` allows the unused variable
  otherwise, and `:158-160` says "In non-`engine` builds the scope kind / mode are unused"); `new_with_mode` is
  `#[cfg(test)]` (`:47`), `new_worker` / `new_service_worker` are `#[cfg(feature = "engine")]` (`:80`, `:131`). So a
  gate (the import, the parameter and the `:31` argument) appears to suffice, and the (c) path would then be unused
  for this site. U0 does not decide it: PR-A1's plan classifies it, and PM rules any site the plan returns.
- **What the interim carries** (stated, not argued): after PR-M1, every `elidex-js` PR is also gated by the
  `default` row, and the `cfg(feature = "engine")` decision tax counted in step (2) stays until the split replaces it.
  In return, the `default` row checks cfg hygiene structurally from PR-M1 on: no name from an optional dependency is
  used outside code gated on it. It does not check layer correctness (engine-gated code may sit anywhere in the core;
  that is PR-A1's Layering check now and the split's structure later), and it does not make the no-engine
  configuration ES-complete: its typed-array family is absent (`vm/host/mod.rs:74-75`, `:101-102`, `:343-356` are
  `engine`-gated) while `ObjectKind::TypedArray` / `DataView` are not gated against a gated `ArrayBuffer`
  (`vm/object_kind.rs:420-421` vs `:677`, `:694`). That gap is VM P4's
  `#11-vm-typed-array-family-layering-and-gate` (`docs/plans/2026-07-vm-p4-slot-ledger.md:310`), whose observable
  (i) is conditional "once the non-`engine` configuration builds" and whose "Until then observable (ii) is this
  row's only runnable one" ends when PR-A1 lands. PM routes that to VM P4; U0 only records it (§8).
- **Retired with F1-a**: draft 1's condition on F1-a (a structural guard for host-freedom once `engine` is gone,
  possibly a new `trip-wires` entry) and its question Q5 no longer apply — under F1-b the `default` row is the cfg
  hygiene guard. §7 records Q5 as retired.

## §6 PR list

| PR | Owner | Scope | Acceptance | After |
|---|---|---|---|---|
| **PR-A1** | VM P4 lane (`vm/lifecycle.rs`, `vm/init.rs`, `vm/gc/collect.rs`, `vm/natives_json/mod.rs`) | F1-b shape only (§5): fix the leaks so `elidex-js`'s `default` (`default = []`) row compiles, keeping `app` and `browser` green, on three OS. Fix kinds (a) relocate engine-bound code into the host layer, (b) gate in place only for core code that merely names a gated type, (c) non-optional dependency → named in the plan and returned to PM (§5); the plan carries the Layering check (§5). `vm/gc/collect.rs` is 2074 lines (`wc -l`): its split is already assigned to VM P4's umbrella 7b (`docs/plans/2026-07-vm-p4-touch-time-split-record.md:84`); PR-A1's plan takes that record's decision, not a second one. | Relative to the slot (PM 336): each diagnostic named in §9 is gone, each one newly surfaced is named in the PR body and gone; every elidex-js row (`default` / `app` / `browser`) exits 0 under `clippy -D warnings` on the PR's three-OS run (run by hand in that PR; the job does not exist yet); each fix is listed with its kind (a)/(b). A returned-to-PM site does not land open: PR-A1 lands only after PM has ruled on each such site and the ruling is applied — in PR-A1 if PM names it, otherwise in PR-A1′ — so no row is skipped (PM 531; I2, I8). | F1 (decided, PM 520); PR-A1′ if it exists |
| **PR-A1′** | Named by PM | Exists only if PM rules a site PR-A1 returned to be a non-optional-dependency fix (a shape change). Applies that ruling. | PM's ruling applied; every elidex-js row exits 0 on three OS as for PR-A1. | PM's ruling |
| **PR-A2** | CSS lane (owner of `elidex-style`) | `parallel` is enabled by no member: either declare a profile and make it green (connect), or remove the feature (delete — a shape change, to PM; applied by PR-A2′). | I4 holds for `elidex-style` (`parallel` and `rayon`) at PR-M2b's base. | C0; connect branch also PR-M1 (it writes PR-M1's table) |
| **PR-A2′** | Named by PM | Exists only if PM rules removal of `parallel`. Applies that ruling. | `parallel` and `rayon` gone; the member's rows green. | PM's ruling |
| **PR-A3…** | Lane owning each module the census finds red | One PR per owning lane, same rule as PR-A1. | Same form as PR-A1. | C0 |
| **PR-M1** | CI/tools | Declaration (I1) for the base; the deriver tool crate (entry point inside the `rust` filter, §4.1); job `feature-profiles` on three OS; `mise run check-profiles`; retire #585's step and `check-default` (I6) and every instruction-file site that names them — at `69fbe61f` (`git grep -n 'check-default'`; `git grep -n -e 'default-features clippy' -e 'clippy --workspace -- -D warnings' -- ':!docs/plans'`): `.claude/skills/pre-push/SKILL.md:28`, `:30`, `:34` (`:34`'s serial fallback would otherwise call a deleted task and skip the rows), `.claude/skills/external-review/project.md:17`, `.claude/skills/external-converge/project.md:13`, `README.md:85`, `CONTRIBUTING.md:46`, `:55`, re-grepped on PR-M1's own base — the #585 precedent (PM 262 "[1] 3 more instruction-file places"; PM 265 fixed them in the same PR); rewrite the fixture step's comment and the profile prose in the three Cargo.toml comments into pointers (incl. the stale `crates/script/elidex-js/Cargo.toml:20-22`, §9); timeout derived from its run. CLAUDE.md needs no edit: its `## CI` lists no jobs (CLAUDE.md:93-95) and it names no `check-default`. | Every row green on the PR's own three-OS run; the run produced ≥ 1 row per workspace member; a control: a scratch commit that breaks one profile only (e.g. an ungated engine-only import) turns exactly that row red while `check` stays green — recorded in the PR body, not merged; the grep above returns no live instruction site; the merged deriver has no mode that skips a row or reports a red row as green (C0's report-only setting is never merged, §7). | every row-fixing PR (PR-A1, PR-A1′, PR-A3…); Python-pin PR-2 (PM 536) |
| **PR-M2a** | CI/tools (consult: Layout lane for the fixture script, A2 owner for the absence) | I5: `test-only` and per-profile absence declarations (App: both `web-storage` features); the classification and the forbidden / unknown checks per row with `-e normal,build,features`; absorb and delete `scripts/check-fixture-boundary.sh` (+ step, filter line); discharge `#11-test-only-feature-boundary-guard` (§4.2). | Controls, each recorded: a normal edge enabling `fixtures` → red; `elidex-storage-core/web-storage` reached in the App row → red, and `elidex-script-session/web-storage` likewise; an undeclared implicit optional-dependency feature enabled from a normal edge (the slot's `foo-test-support`) → red; a `cargo tree` output with no feature node → red; the `fixtures` dev edges of the base → green. Timeout re-derived. | PR-M1 |
| **PR-M2b** | CI/tools (consult: CSS lane) | I4 completeness per row. | Controls, each recorded: a feature no row activates → red (incl. an implicit one); the browser row covering both `web-storage` features while App forbids them → green. Timeout re-derived. | PR-M2a, PR-A2 or PR-A2′ |
| **PR-T** | CI/tools (job) + VM P4 (test gating) | Defect: a non-full profile's own behaviour (code compiled out, e.g. the compat ceiling with `compat-webapi` off, `vm/init.rs:184-185`) is executed by no test in that profile. Run each declared row's tests under that row's flags; its plan first runs the test-target census (Q7) and splits fix-first PRs by the same (a)-before-(b) rule. | Every declared row's test targets compile and pass under the row's flags on three OS, run by the same entry point (one `cargo nextest run -p <member>` per row); per row, the executed-test count is > 0 for every member whose `--all-features` run has tests (non-vacuity); a test that cannot run in a profile is cfg-gated on that profile in the same PR, never skipped by name; the Q7 census is in the PR body. | PR-M2b |

Every row's PR plan goes through `/elidex-plan-review` before implementation (PM 498). PR-A1's plan is the VM P4
lane's; this umbrella fixes only its acceptance, its fix kinds and its return-to-PM boundary (§5). **PR-M1 is a
terminal unit**: I6×I8 puts retiring #585's step and adding the rows in one PR, and the deriver, declaration and
`check-profiles` without the job would be configuration nothing runs; its own plan-review confirms the slice.

## §7 Census C0 and open questions

**C0** (measurement, not a PR; permitted by PM 501, "空き 50G を切ったら止める" — it stops when free disk falls under
50 GB): on the base PR-M1 will be cut from, every row of §4.1's declaration, on macOS locally and on Linux/Windows
through a draft CI run of PR-M1's branch; plus I4/I5 over each row's resolve. Report-only is a setting of that
unmerged draft run (e.g. `continue-on-error` on a scratch commit), never a flag of the tracked deriver. Output: the
red rows with their diagnostics and owners → the PR-A3… list. Until C0 runs, the only measured row set is
elidex-js's three (§9) — **1 of 54 members measured**.

Questions (for scout / PM; Q3 and Q5 are retired by PM 520 and Q4 is answered, kept so the numbering holds):

- **Q1** C0 itself (above).
- **Q2** (PM) Where PR-A1 (F1-b shape, §6) sits in the VM P4 lane's queue (guard memo, S1, call-depth guard, …), and
  whether any in-flight VM P4 branch touches the four files of PR-A1.
- **Q3** — retired: F1 is decided (PM 520), so the merge it proposed is superseded by PM 531 (§8).
- **Q4** — answered: the Layout lane's landing list
  (`elidex-pm-trial/handoff/pending-at-landing/c0-program.md`) has 0 hits for `fixture`, `ci.yml`, `clippy`,
  `check-default` and `585` (`/usr/bin/grep -c`, 2026-11-08). The widened population (plans naming the step or the
  script) is §4.4's coordination list.
- **Q5** — retired: it served F1-a's condition only, and F1-a was not taken (§5).
- **Q6** (scout) Can an off-the-shelf runner read a `package.metadata` declaration, or only CLI flags? (PR-M1's
  implementation choice.)
- **Q7** (scout) Does `cargo test -p elidex-js --no-run` compile with each declared feature list? (PR-T. The
  `#[cfg(test)]` `Vm::new_with_mode` takes `EngineMode`, `vm/init.rs:47-49`, so the `default` row's tests need PR-A1's
  `EngineMode` treatment too.)
- **Q8** (PR-M1) Row cost per OS leg on the census base.
- **Q9** (PM) Who owns the A2 absence guarantee today (`docs/plans/2026-06-web-api-compat-a2-storage-demotion-plan.md`
  §4 (C), `:261`) and is encoding it as a declared absence acceptable to them?

## §8 Slots

- `#11-elidex-js-default-features-build-broken` — discharged when PR-M1 lands (the gate) with every row-fixing PR
  landed (the code). Its ledger "Update (PM 498)" line already carries the corrected breakdown (PM 501); what is
  stale is the slot's head sentence, which lists `dispatch.rs:998` among the failing files — §9's measurement has no
  error there. The correction is PMO's.
- `#11-elidex-js-no-default-features-support` — stays open, re-homed to the VM P4 lane, its defect restated as the
  language-core/host boundary being held only by `cfg` annotations that are themselves broken, and closed when the
  crate-split umbrella lands (PM 531; already so in the ledger). Its Re-eval-date 2026-11-30 is the interim's
  re-evaluation (§5).
- `#11-test-only-feature-boundary-guard` — discharged by PR-M2a (§4.2); its attach point moves from the retired #585
  step to the matrix, and its remedy (iv) (product features no member enables) points at PR-M2b. Ledger Re-eval
  2026-12-15.
- `#11-vm-typed-array-family-layering-and-gate` (VM P4, `docs/plans/2026-07-vm-p4-slot-ledger.md:310`) — not this
  program's; its "Until then" premise ends when PR-A1 lands (§5). PM routes it to VM P4.
- No new slot: PR-T is in this program, not deferred. §10's per-profile rustdoc gap has no slot (§10).

## §9 Brief claims found false or stale (measured at `69fbe61f`)

- **"15 errors, all in `vm/lifecycle.rs`"** — false. 15 errors: 13 in `crates/script/elidex-js/src/vm/lifecycle.rs`
  (E0433 at `:45`, E0599 at `:34` and `:45`, E0609 at `:46-47`, `:51-54`, `:59-61`, `:63`), **1 E0432 at
  `crates/script/elidex-js/src/vm/init.rs:11`** (`elidex_plugin`), **1 E0433 at
  `crates/script/elidex-js/src/vm/gc/collect.rs:47`** (`elidex_ecs`); plus the 2 unused-import warnings at
  `crates/script/elidex-js/src/vm/natives_json/mod.rs:13` and `:15`, red under `-D warnings`. Source: the driver's
  own log `prof-default.log` (`grep -nE 'error|warning|-->'`). The false form is PM 498's text ("すべて
  vm/lifecycle.rs"); the ledger's PM 498 line was corrected by PM 501.
- **`crates/script/elidex-js/Cargo.toml:20-22`** ("No such embedder wires `elidex-js` yet — the shell still runs the
  boa engine") — stale: `crates/shell/elidex-shell/Cargo.toml:31` enables `engine` + `compat-webapi`. PR-M1 replaces
  the passage with a pointer to the declaration.
- Everything else the brief states held when re-checked: 6 of 54 packages declare features; the profile prose lines;
  #585's step at `ci.yml:111-112` and `mise.toml:14-19`, in `[tasks.ci]` at `mise.toml:142`.

## §10 What this does not buy

- Union-only, non-additive feature failures (§4.2: the book asks projects to avoid them; `--all-features` remains
  the maximal union).
- Targets other than the three OS legs: the same set `check` compiles. `rust-toolchain.toml` installs no extra
  target and no `wasm32` site exists (`git grep -n 'wasm32' -- crates '*Cargo.toml'` → 0); the one
  arch-split site, `crates/core/elidex-sandbox/src/linux.rs:171-179`, has a Linux-aarch64 arm that no CI job
  compiles — a gap shared with `check`, not introduced or closed here.
- Per-profile rustdoc: the `doc` job stays `--all-features`, and clippy does not resolve intra-doc links, so a doc
  link to an item a profile compiles out (e.g. `vm/init.rs`'s links to `EngineMode` variants in the `default` row) is
  checked by nothing. Unmeasured, and no slot holds it — driver/PM must confirm whether a slot is registered or the
  rows also run rustdoc.
- Third-party crates' features (outside I4/I5's population, §2 I5).
- Merge blocking: `main-protection` has no required status check, so a red `feature-profiles` job is red, not
  blocking — the eyeball-before-squash convention remains the final gate.
