# Umbrella U0 — per-feature-profile build matrix (draft 4, 2026-11-09)

## §0 Status

**Lane**: CI/tools (gate policy). **Base**: `origin/main` at **`69fbe61f`** (#609, 2026-11-08). Every path, line and
count below was read at that commit; the command that produced each count is printed next to it.

**Slots**: `#11-elidex-js-default-features-build-broken` (owner CI/tools; this umbrella is its program) and
`#11-elidex-js-no-default-features-support` (the "is the no-engine build a supported surface" question; answered by
PM 520, §5; re-homed to VM P4 by PM 531, §8). `#11-test-only-feature-boundary-guard` is coupled (§4.2: PR-M2a
narrows it to remedy (iv), PR-M2b closes it, PM 545). `#11-vm-typed-array-family-layering-and-gate` (VM P4) is
affected: a premise of it ends when PR-A1 lands (§5, §8).

**What this memo is**: the umbrella of an edge-dense program — PM 498 ("umbrella U0（designer）+ PR ごとの plan、
実装前に /elidex-plan-review"): the decisions on the four axes PM 501 fixed ("4 軸 = (i)…(iv)"), the coupled
invariants (§2), the PR list with owners, scope, acceptance and order (§6). Per-PR implementation detail belongs to
each PR's own plan, and each PR plan goes through `/elidex-plan-review` before implementation.

**F1 decided (PM 520, user decision).** Whether `elidex-js` built without `engine` is a supported surface was a
feature-shape question, which PM 498 reserved ("収束しないか feature の形を変える設計変更なら PM へ"). §5 records
the decision, PM 520's reasons and the options as put to PM; it does not re-derive the decision (PM 547). The user
chose **F1-c** (split the crate). Per the contingent path draft 1 stated, the split is its own umbrella, owned by the
VM P4 lane and **outside this umbrella's scope**; **F1-b** (keep `default = []`, fix the gating) is the interim. So
elidex-js declares three rows (§4.1) and PR-A1 has the F1-b shape only (§6): it relocates engine-bound code to the
existing engine/non-engine seam of the layer that owns it, gates in place where the code is right to keep in the
core, and returns to PM any site whose right fix is a non-optional dependency (§5).

**What a row asserts** (stated once here, used everywhere): in that configuration, a row's lib and bin targets
compile under `clippy -D warnings` and document under `rustdoc -D warnings` (so a doc link to an item the
configuration compiles out is red) — **cfg hygiene**, of the code and of its docs (the two columns, §4.3). A row
does not assert that the configuration is ES-complete, nor that the code is in the right layer (§2 I2, §5).
Behaviour in the configuration — its tests and doc-tests — is PR-T's.

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
  ({`default`} ∪ declared profiles) × OS legs. Never a list in YAML or a script. A row asserts cfg hygiene (§0) in
  both columns; not ES completeness of the configuration (the no-engine `default` row has no typed-array
  constructors, §5), not layer correctness (§5), not behaviour (PR-T's). A member cannot opt its `default` out: a
  default that should not be a declared configuration is a feature-shape question (PM 498, ruled for elidex-js by PM 520), not a skipped row. An unparseable declaration, an
  unknown key, or a profile naming a feature the member does not have is red, never skipped. Non-vacuity: the run
  asserts it produced at least one row per workspace member.
- **I3 — isolation.** Every Cargo invocation of a row selects that row's member alone (`-p <member>`; named
  profiles via `--no-default-features --features <list>`), lib + bin targets only; a row has one invocation per
  column — `clippy -- -D warnings` and `doc --no-deps` with `RUSTDOCFLAGS=-D warnings` (§4.3, PM 542 (1)) — and no
  invocation selects two members. Under resolver 2 (`Cargo.toml:2`) a lib/bin build activates no dev-dependency
  feature (Cargo Book §3.4 #feature-resolver-version-2), so a row is the member's production configuration;
  separate invocations are what the Cargo Book names as the way to avoid cross-package unification. A warning that
  exists only in that configuration (e.g. an import left unused, or a doc link to an item compiled out) is red.
  The row's compile is the authoritative check of what builds; I4/I5 read the resolve, which is an approximation
  of it (I5).
- **I4 — completeness.** Every feature in a member's `[features]` as `cargo metadata` lists it — Cargo's implicit
  optional-dependency features included; excluding only the `default` key and declared test-only features — is
  active in the resolve of at least one row. Which entries are implicit does not change the gate (implicit and
  explicit features are both in the population). For counting only, this memo identifies an implicit feature by an
  elidex-side rule: a `packages[].features` entry `k` whose value is exactly `["dep:k"]`. That is how 1.99.1's
  `cargo metadata` shows one (measured); the Cargo Book does not define the representation, and an explicit feature
  written `k = ["dep:k"]` would look the same — at `69fbe61f` no manifest writes one
  (`git grep -n '"dep:' 69fbe61f -- '*Cargo.toml'` → 1 line, `elidex-script-session`'s two-element `web-storage`).
  Measured on each row's own resolve, never on the workspace resolve. This also makes every unclassified feature that
  no row activates red.
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
- **I6 — one mechanism.** After the program, the configurations CI compiles or documents without dev-only features
  are exactly the matrix rows. #585's step and `mise check-default` are retired in the same PR that adds the rows
  (PR-M1); `scripts/check-fixture-boundary.sh` is absorbed into I5 (PR-M2a). The `check` job's `--all-features`
  steps and the `doc` job stay: they are the test configuration (dev-only features on, `--all-targets`) and its
  documentation, a different property.
- **I7 — CI shape, parity, cost.** A dedicated job `feature-profiles` (NEW) on the same three OS as `check`, rows
  (both columns each) run serially inside one entry point; `mise run check-profiles` (NEW) runs the same entry point
  (host OS only); `timeout-minutes` derived from the job's own run, both columns included (§4.3).
- **I8 — land green.** No PR of the program lands red, and the matrix PR lands only on a base where every row is
  green on all three OS. Adding a profile to the declaration later is adding a row: the PR that adds it makes it
  green (or carries the fix) in the same PR.

| Pair | Intersection |
|---|---|
| I1×I2 | The deriver reads only `cargo metadata --format-version 1`; a malformed declaration is a red row, not a dropped one. |
| I1×I3 | A profile is one feature list, passed as `--no-default-features --features` to each of the row's invocations; the declaration cannot express cross-member unification, by construction. |
| I1×I4 | Completeness is computed against every `[features]` key (implicit ones included) minus `default` and declared test-only ones; a new feature with no covering row is red in the PR that adds it, which forces its declaration there. |
| I1×I5 | Classification is computed from the declaration alone (profiles' `[features]` closure; `test-only` lists), never from which edges happen to enable a feature; the fixture script's hard-coded `elidex-ecs` / `fixtures` literal becomes a `test-only` entry. |
| I1×I6 | The Cargo.toml prose (§1 list) becomes a pointer to the declaration; profile semantics are not restated in comments. |
| I1×I7 | The declaration lives in `Cargo.toml` files, already inside ci.yml's `rust` path filter (`ci.yml:34-48`, `crates/**`). |
| I1×I8 | Declaring a profile adds rows on three OS; the declaring PR owns their green. |
| I2×I3 | Population × isolation = invocation count = cost; the only permitted reduction is de-duplicating rows whose (member, resolved feature set, OS) is identical, computed per run, never listed. De-duplication is not a skip: the configuration is still compiled and documented, once, and the run reports the de-duplicated row with the row whose result it shares, so a redundant declaration stays visible. |
| I2×I4 | Completeness is only as strong as the population; the non-vacuity assertion (≥ 1 row per member) stops a silently shrinking derivation from making I4 vacuous. |
| I2×I5 | Forbidden checks run per row on each OS leg; `cargo tree`'s `--target` defaults to the host platform (Cargo Book §4.3.8 #option-cargo-tree---target), so the three legs see three hosts' target-specific edges (the fixture script today sees one host per leg too, but through a workspace union). |
| I2×I6 | The workspace-union configuration is not a row (§4.2 says what that gives up). |
| I2×I7 | Row count × three legs drives the timeout; a PR adding members or profiles re-derives it (§4.3). |
| I2×I8 | The census (C0, §7) is the population evaluated on the base before PR-M1; PR-M1's own three-OS run is the land-green evidence. |
| I3×I4 | Each row's resolve is read from that row's own invocation (`cargo tree -p <member> -e normal,build,features` with the row's feature flags); reading the workspace resolve (`cargo metadata`'s `resolve` is "for the entire workspace", §4.3.5 #json-format) would re-hide what unification hides. |
| I3×I5 | Same reader. Edge kinds are explicit: `cargo tree`'s default is `normal,build,dev` (§4.3.8 #option-cargo-tree---edges), and `-e features` alone adds the other kinds; measured on the pinned 1.99.1: `cargo tree -p elidex-js --features engine,compat-webapi -e features` is byte-identical to `-e normal,build,dev,features` and contains an `elidex-ecs feature "fixtures"` node (dev edge), `-e normal,build,features` contains none — so a row reading `-e features` would be falsely red on every member with a `fixtures` dev-dependency. The fixture script already uses `-e normal,build,features` (`scripts/check-fixture-boundary.sh:17`). |
| I3×I6 | The `--all-features` steps are not rows: they turn dev-only features on for test targets. |
| I3×I7 | Serial invocations share one target dir; that Cargo keys artifacts on the feature set (so rows do not rebuild or contaminate each other) is not stated in the Cargo Book (§3.9 Build Cache does not mention features); PR-M1 measures it on its run (a second row with a different feature set leaves the first row's artifacts fresh). A separate job gets its own rust-cache key, so its feature sets do not evict `check`'s. |
| I3×I8 | A row's verdict does not depend on row order (each row's invocations are its own; artifact reuse is PR-M1's measurement, I3×I7), so a green census predicts a green PR-M1 run up to base drift. |
| I4×I5 | A feature can be required by I4 and forbidden by I5 at once only per profile: App forbids both `web-storage` features, the browser row covers them. Both hold; PR-M2b's controls include exactly this case. |
| I4×I6 | I4 is strictly stronger than #585's step: #585 compiled features some member enables; I4 also reaches features no member enables (`elidex-style/parallel`). |
| I4×I7 | One `cargo tree` per row, inside the same entry point, shared with I5. |
| I4×I8 | `elidex-style/parallel` (and its implicit `rayon`) is uncovered today, so I4 is red at the base: it needs its fix (declare + green, or a shape change PM rules) before I4 lands. So I4 is its own PR, PR-M2b, after PR-A2; I5 (PR-M2a) does not wait for it (§4.4). |
| I5×I6 | PR-M2a retires `scripts/check-fixture-boundary.sh` and its ci.yml step and path-filter line, moving the assertion into I5; `#11-test-only-feature-boundary-guard` attaches here instead of to the retired #585 step. |
| I5×I7 | The forbidden and unknown checks run on every OS leg. |
| I5×I8 | At PR-M2a's base the App row must satisfy both A2 absences (census C0 includes it), and no active feature may be unknown. At `69fbe61f` no member's dependency declaration or `[features]` value names another member's implicit feature, and `engine` lists all 25 of `elidex-js`'s implicit features (`cargo metadata --no-deps --format-version 1`, `packages[].dependencies[].features` and `packages[].features`; 26 implicit features in total by I4's counting rule: `elidex-js` 25, `elidex-style` `rayon`)
, so the unknown rule is green there by measurement of the manifests; C0 confirms it on the resolves. |
| I6×I7 | `[tasks.ci]` (`mise.toml:142`) replaces `check-default` with `check-profiles` in the same PR. |
| I6×I8 | Retiring #585's step and adding the rows are one PR: no commit on `main` has neither, or both. |
| I7×I8 | PR-M1's own three-OS run is both its land-green evidence and its timeout measurement. |

## §3. Spec coverage map

A CI plan touches no web-platform algorithm. The rows below are the **governing sources** of the design: Cargo's
documented behaviour (the Cargo Book shipped with the pinned toolchain 1.99.1, `rust-toolchain.toml`, read from
`~/.rustup/toolchains/1.99.1-aarch64-apple-darwin/share/doc/rust/html/cargo/`; the § numbers are that book's chapter
numbering from its `toc-0e7001e7.js`, anchors are its heading and option ids) and the two specification sections
behind the spec clause of PM 520's reasons (§5). Honesty notes: (a) `preflight.py` cannot verify the Cargo Book
rows — "Cargo Book" is not in its `SPEC_LABEL_REVERSE`, so those rows are unmapped-label soft-warns; they were checked by hand against the local
book; (b) the repo's own rules (CLAUDE.md, ci.yml comments, PM decisions) govern as much as Cargo does, but they carry
no § number and a row without one is a preflight hard-fail, so they are listed under the table, not in it; (c) "Step"
holds an anchor id, "Branch" the behaviour the design relies on.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Cargo Book §3.4 Features | #feature-unification | a dependency is built with the union of features its dependents enable; features "should be additive" | I3 (one member per invocation), §4.2 | ✓ | no |
| Cargo Book §3.4 Features | #mutually-exclusive-features | "rare cases where features may be mutually incompatible" exist | §4.2 (what absorbing gives up) | ✓ | no |
| Cargo Book §3.3.3 Dependency Resolution | #features | `--workspace` / several `-p` unify; separate invocations do not | I3, I6 (#585's step is a union) | ✓ | no |
| Cargo Book §3.4 Features | #feature-resolver-version-2 | dev-dependencies activate no feature for lib/bin targets | I3 (lib + bins only) | ✓ | no |
| Cargo Book §3.4 Features | #resolver-version-2-command-line-flags | `--features` / `--no-default-features` apply to the `-p` selection | row invocation shape | ✓ | no |
| Cargo Book §3.4 Features | #optional-dependencies | an optional dep without `dep:` defines an implicit feature | I4 population, I5 unknown rule | ✓ | no |
| Cargo Book §3.4 Features | #feature-combinations | full coverage is exponential; the book encourages each project to choose its configurations | I1/I2 (declared profiles, not a powerset) | ✓ | no |
| Cargo Book §3.1 The Manifest Format | #the-metadata-table | `package.metadata` "completely ignored by Cargo", for tools | I1 (declaration home) | ✓ | no |
| Cargo Book §3.5 Profiles | #profiles | "profile" already means build profile | §0 terminology, §4.5 | ✓ | no |
| Cargo Book §4.3.5 cargo metadata | #description | "The output format is subject to change"; "It is recommended to include the --format-version flag" | I1, I2 (deriver input) | ✓ | no |
| Cargo Book §4.3.5 cargo metadata | #json-format | `packages[].metadata`, `packages[].features`, `workspace_members`; `resolve` is the whole workspace's | I1, I5 classification, I3×I4 | ✓ | no |
| Cargo Book §4.2.6 cargo doc | #target-selection | with no target selection, documents "all binary and library targets of the selected package" | §4.3 rustdoc column (same targets as the clippy column) | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #feature-unification | "doesn't guarantee the exact equivalence to what Cargo is going to build"; `-e normal,build` is "pretty close" to `cargo build` | I5 (what it can assert), I3 (compile is authoritative) | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #option-cargo-tree---edges | default kinds `normal,build,dev`; `features` alone adds the others | I3×I5 flags | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #option-cargo-tree---target | default is the host platform | I2×I5 (per-leg hosts) | ✓ | no |
| ECMA-262 §4.2 Hosts and Implementations | #sec-hosts-and-implementations | host-defined facilities are deferred to an external source; "Implementations that are not hosts may also provide definitions for host-defined facilities" | §5 reasons (the spec clause of PM 520's quote) | ✗ | no |
| WHATWG HTML §8.1.6 JavaScript specification host hooks | #javascript-specification-host-hooks | HTML is the host that defines those hooks | §5 reasons (the spec clause of PM 520's quote) | ✗ | no |

**Breadth**: K=3 sources (preflight count), M=17 rows. The spec rows are ✗ for full enumeration on purpose: they are
the sections behind the spec clause of PM 520's reasons, quoted in §5 (the host-hook structure exists; the spec is
silent on a host-less build), and nothing here implements a hook.

**Repo rules that govern (no § number)**: CLAUDE.md 至上命題 (authority order), Design philosophy (core/compat
layering, plugin-first, ScriptSession as the sole Script↔ECS boundary, Concurrency by ownership and phases —
PR-A2's shape branch, Security by structure and Ideal over pragmatic — with the Layering mandate, PM 520's reasons,
§5), Layering mandate (also the destination rule of PR-A1's fix kind (a)),
Design discipline (One issue one way, dead code, Supported-surface testing, Edge-dense, 1000-line touch-time split),
`## CI` (CLAUDE.md:93-95: ci.yml's job comments are canonical); ci.yml's job-boundary rule (`ci.yml:236-245`), its
tamper-path reasoning (`ci.yml:182-189`, `:229-231`) and its `tools`-job timeout rule (`ci.yml:250-269`); PM 336
(relative acceptance), PM 410 (matrix direction), PM 498 / PM 501 (this umbrella's frame), PM 520 / PM 531 (F1),
PM 536 (ordering against Python-pin PR-2, citation form), PM 255 / PM 261 / PM 265 (the mechanisms §4.2 folds),
PM 542 (rustdoc column, per-profile doc-tests), PM 543 / PM 545 (PR-A1′ acceptance, the PR-A2 and PR-A3… owners,
the guard slot's narrowing), PM 547 (§5's form).


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
  - `elidex-style`: nothing. PR-M1 declares no named profile for it, so at PR-M1 it has its `default` row only;
    PR-A2's connect branch adds a `parallel` row later (§4.4 step 6).
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
  *this* declaration, one member per invocation, same entry point locally and in CI). Whether an existing runner reads a
  `package.metadata` declaration is unverified (Q6).

### §4.2 Axis (ii) — unification, and #585's step: **absorbed (retired), not kept**

**Decision**: PR-M1 deletes ci.yml's `Clippy (default features, no dev-only feature)` step and mise's
`check-default`, in the same PR that adds the rows (I6×I8). There is no interval with both, and none with neither.

**The decisions this folds** (PM 501 (ii), "#585 の step を matrix に吸収して同じ PR で廃止"): both mechanisms were
created by user decisions — PM 255 (user: γ), "O-3 = default-features check. Split: a CI/tools prereq PR …
→ C0b (scripts/check-fixture-boundary.sh …)"; PM 261, "The PM 256 two-mechanism text stays"; PM 265 (user), "the
task name check-default is kept". Absorbing keeps what those decisions bought — a default-features check, run in CI
and in `mise run ci`, and a test-only-feature assertion — and folds the mechanisms that carried them: the
default-features check becomes each member's own `default` row (stronger, below), and the fixture assertion becomes
I5 (PR-M2a). Whether the user is told is PM's call.

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
`#11-test-only-feature-boundary-guard` books. Item by item against the slot's remedy (ledger
`memory/project_open-defer-slots.md`, the slot's "Remedy (unreviewed, from the reviewer)" sentence):

- (i) "take the population from cargo (`cargo tree --workspace --all-features -e features --prefix none`)" — the
  population is taken from cargo, but the recipe is **replaced**: I5 classifies from `cargo metadata` and reads each
  row's own resolve with `-e normal,build,features` (I3×I4, I3×I5), because the slot's recipe is a workspace union
  and its `-e features` includes dev edges, which makes every member with a `fixtures` dev-dependency falsely red
  (measured, I3×I5).
- (ii) "classify every feature that the default normal edges enable as product/default, and fail closed on unknown
  ones" — the fail-closed half is I5's unknown rule (implicit features included). The classification half is
  **replaced**: I5's product is the declared profiles' `[features]` closure and explicitly *not* what dependency
  declarations enable, because classifying by enabling edges would make any normal edge that turns a test-only
  feature on classify it as product — the defect the slot books.
- (iii) "cover non-host targets (`--target all` or 3 OS)" — the three OS legs.
- (iv) "also watch product features that no member enables (`parallel`)" — I4, which lands in PR-M2b.

The slot's defect sentence (no mechanism guaranteeing a test-only feature is enabled from dev-dependencies only) is
closed by (i)–(iii), so PR-M2a discharges (i)–(iii); the slot stays open narrowed to (iv) and PR-M2b closes it
(PM 545 P2, §8). The slot's "attaches to the default-features clippy step" is superseded by this decision. The
slot's remedy is recorded as unreviewed; PR-M2a's plan-review is its first review, and the two replacements above
are what that review is asked to confirm.

### §4.3 Axis (iii) — CI shape and cost

- **Job**: a new job `feature-profiles` (NEW), `needs: changes`, gated like `check`
  (`if: needs.changes.outputs.rust == 'true' || github.event_name == 'push'`; `ci.yml:59`) — it compiles Rust, and
  under §4.1's entry-point constraint all its inputs (manifests, the tool crate, ci.yml, mise.toml) are inside the
  `rust` filter, so the tamper path that keeps `trip-wires` / `tools` ungated (`ci.yml:182-189`, `:229-231`) does not
  arise. Not a step of `check`: a separate job runs in parallel with `check` instead of lengthening it, and gets its
  own rust-cache key (I3×I7).
- **Columns** (PM 542 (1)): each row runs `cargo clippy -p <member> <row flags> -- -D warnings` and
  `cargo doc -p <member> --no-deps <row flags>` with `RUSTDOCFLAGS=-D warnings` — the same flag the `doc` job sets
  (`ci.yml:134-136`). Both document or compile the member's lib and bin targets (`cargo doc` with no target
  selection "will document all binary and library targets of the selected package", Cargo Book §4.2.6
  #target-selection). The rustdoc column is in this matrix, not a second job, because a second job would be a
  second reader of the declaration (One issue, one way). Per-profile doc-tests run code, so they are PR-T's. The
  `doc` job (`ci.yml:120-136`, `--all-features`, Ubuntu) stays, as the `check` job's `--all-features` steps do (I6).
- **OS legs**: `ubuntu-latest`, `macos-latest`, `windows-latest`, `fail-fast: false` — the same set as `check`
  (`ci.yml:61-65`), for #585's own reason (`ci.yml:105-107`: default-feature builds differ per target because
  `cfg(target_os/windows/unix)` sites exist). Not a GitHub matrix over rows: per-row legs would each cold-build the
  shared dependency graph.
- **mise parity**: `mise run check-profiles` (NEW) runs the same entry point; `[tasks.ci]` depends on it instead of
  `check-default`. Local runs cover the host OS only; the other two legs are CI-only (stated in the task's comment).
- **Timeout**: derived the way ci.yml's `tools` job does (`ci.yml:250-269`): PR-M1 sets `timeout-minutes` to
  `ceil(4 × slowest leg's end-to-end minutes)` of its own run, both columns included (the `tools` job's "about 4x",
  `ci.yml:257`), and records run id and leg time in the job's comment; any later PR that adds rows (members or
  profiles) or row work (PR-M2a / PR-M2b's resolve checks, PR-T's tests and doc-tests) re-derives it from its own
  run (`ci.yml:250`). Whether a re-derivation may lower the value is PR-M1's plan's call: the `tools` job's
  raise-only rule is program-scoped ("raise-only for the program, PM 353", `ci.yml:267`; #586's plan §7), not a general rule. The `check` job's `timeout-minutes: 20`
  (`ci.yml:60`) is not touched.
- **Cost**: unmeasured (Q8). Each row is an incremental clippy plus a `--no-deps` rustdoc of one member, whose
  dependency artifacts are shared whenever the feature set matches (PR-M1 measures that reuse, I3×I7); only members
  whose resolve differs from rows already built pay. PR-M1's plan measures it on the census; it is judgment input,
  not a reason to shrink the population (I2).


### §4.4 Axis (iv) — cross-lane ordering

1. **F1 decided** (PM 520, §5): F1-b is the interim, so PR-A1 has the F1-b shape and elidex-js declares three rows.
2. **Census C0** (§7) on the then-current base — names every red row and every I4/I5 violation, with owners.
3. **Row-fixing PRs**: **PR-A1** (VM P4 lane) and any **PR-A3…** the census finds, each with the owner §6's PR-A3…
   rule gives it (PM 545 P1), in any order among themselves. **PR-A1′** (exists only if PM rules a returned site's
   fix to be a non-optional dependency and names a PR other than PR-A1; owner named by PM) lands before PR-A1: its
   acceptance is relative and does not need the `default` row green (§6, PM 543). Otherwise PM names PR-A1 itself
   as the PR that applies the ruling. Either way PR-A1 does not land before the ruling is applied (PM 531).
4. **PR-M1** (CI/tools) — reviewed and ready *before* PR-A1 lands; lands in a PM-held landing window directly after
   the later of (a) the last row-fixing PR of step 3 and (b) **Python-pin PR-2**: PR-M1 edits ci.yml, so by
   decision 150 (3) it lands after Python-pin PR-2 (PM 536: "PM 494 は decision 150 (3)…を上書きしていない";
   `docs/plans/2026-10-python-pin-umbrella.md:599-602`). It lands after a fresh three-OS run on the new head.
   Reason for the window: the VM P4 lane lands into `elidex-js` weekly, and the default configuration has been red
   for six months with no gate (§1); every day between PR-A1 and PR-M1 is a day it can go red again unseen. PR-A1
   does not wait for PR-2: if PR-2 has not landed when PR-A1 is ready, PR-A1 lands and that gap is accepted until
   PR-M1. When PR-M1 is ready and PR-2 has not landed, the driver raises it to PM with the reason (PM 536).
5. **PR-M2a** (CI/tools; I5), after PR-M1. It needs nothing else: I5 is green at the base by measurement (I5×I8).
6. **PR-A2** (CI/tools, PM 543), after C0. Its connect branch declares a `parallel` profile in PR-M1's table, so
   that branch lands after PR-M1 with its row green (I8); a shape branch (remove, or always on) is ruled by PM and
   applied by **PR-A2′** (owner and PR named by PM; exists only if PM rules a shape change).
7. **PR-M2b** (CI/tools; I4), after PR-M2a (it needs the `test-only` declarations I4 excludes) and after PR-A2 or
   PR-A2′. Until it lands, `#11-test-only-feature-boundary-guard` stays open, narrowed to (iv), with its Re-eval
   date (§8).
8. **PR-T** (§6), after PR-M1: it needs the rows and the entry point, and nothing from I4 or I5, so it does not wait
   for PR-M2a, PR-M2b or PR-A2.

**Why I5 and I4 are two PRs** (PR-M2a, PR-M2b): from PR-M1 to the fixture script's absorption, two mechanisms
answer the test-only question (the workspace-union, host-only script and the matrix) — the strangler interval One
issue, one way forbids keeping. Only I4 waits for PR-A2, and PR-A2 can return to PM as a shape change with no date.
One PR with an internal dependency split still lands at its last dependency, so the interval would last until
PR-A2; two PRs bound it by PR-M2a, which depends on PR-M1 alone.

Coordination owed. Population: `git grep -n -e 'check-fixture-boundary' -e 'default-features clippy' -e 'default
features, no dev-only' -e 'check-default' 69fbe61f -- docs/plans` → 20 lines in 5 memos (`git grep -c` with the same
patterns), plus the ledger, which that command does not reach. Each, with who acts:

- The VM P4 lane: its PRs are gated by new rows from PR-M1 on; PR-A1 is in its files; the typed-array slot's
  premise (§8).
- The L3 lane, `docs/plans/2026-11-range-contents-program.md` (5 lines, active): `:326-327` and `:885` name the
  default-features clippy step and the fixture script as the enforcing mechanism for RC-2b / A6; `:893` is the
  RC-2b-first branch's basis, which rests on C0b's two properties and reopens "If C0b lands without either"; `:2317`
  is the closed H11 record of those properties. The properties survive — the per-member `default` rows and I5 keep
  both — but the cites will name retired mechanisms, and `:893`'s reopen condition must be read against I5, not
  against the script. The owning lane updates its own memo.
- The Layout lane: `2026-10-eh-attrs-c0-insertion-adoption.md` (6 lines) and
  `2026-11-eh-attrs-c0b-node-document-fact.md` (7 lines) are landed memos that cite the step and the script as
  records and are not edited; C0b's fixture script is retired by PR-M2a; its landing list does not cite it (Q4).
- CI/tools' own landed records, `2026-10-python-pin-pr0-tracked-mise-config.md` and
  `2026-10-python-pin-pr1-pin-and-gate.md` (1 line each, naming `check-default` as history): not edited.
- The ledger (`memory/project_open-defer-slots.md`): `#11-ownerless-node-creation`'s landing note names
  `scripts/check-fixture-boundary.sh` as the check of C0b's facet. PMO updates it via PM when PR-M2a lands (§8).
- The owner of the A2 Web-Storage demotion (PR-M2a encodes their absence guarantee — Q9).

### §4.5 Naming

"Feature profile" everywhere this program writes (declaration key, job, task, comments), because Cargo's "profile" is
the build profile (Cargo Book §3.5) and the `check` job already passes nextest a `--profile` (`ci.yml:90`).

## §5 F1 — is `elidex-js` without `engine` a supported surface? (DECIDED: F1-c, F1-b interim — PM 520)

This section holds three things: the decision, PM 520's reasons for it, and the options as they were put to PM. The
decision of record is PM 520. This memo does not re-derive it (PM 547).

**Decision (PM 520; decided by the user, relayed by PM).** **F1-c**: split `elidex-js` into a language crate and a
host crate, so that Cargo's dependency graph enforces the language/host boundary (PM 520: "`cfg` の注釈や trip-wire
という取り決めではなく、構造で守る"). Draft 1 recommended F1-a. That recommendation was not taken and is withdrawn; it
is recorded below only as what was put to PM.

- **The split is not this umbrella's.** By the contingent path draft 1 stated, F1-c is its own umbrella. It is owned
  by the VM P4 lane and is outside U0's scope: U0 does not plan, scope or schedule it. It is the external program
  that will eventually replace F1-b's `cfg` gating.
- **F1-b is the interim (bridge).** Until the split lands, `elidex-js` keeps `default = []` and the `default` row is
  a declared row like any other (I2). PR-A1 (VM P4 lane, §6) makes that row green (§9's diagnostics and whatever
  later compiler phases surface). The interim is re-evaluated on the slot's Re-eval-date, 2026-11-30 (ledger, §8);
  U0 sets no other date.

**The reasons** (PM 520, `elidex-pm-trial/PORTFOLIO-2026-10-26.md:801`, quoted as written): "根拠: elidex 原則 —
Ideal over pragmatic、Security by structure, not review convention、Layering mandate（仕様は ECMA-262 の host hook
という構造は持つが、host なしで build できるかについては沈黙。節番号は未検証）". These three principles are the
reasons of record. *Annotation, not part of the quote:* the section numbers were later looked up with
`.claude/tools/webref` for §3 — ECMA-262 §4.2 Hosts and Implementations (`webref heading ecma262 4.2`) and WHATWG
HTML §8.1.6 JavaScript specification host hooks (`webref heading html 8.1.6`). PM 520's text is left as written.

**The fork and the options, as put to PM in draft 1.** I2 makes every member's `default` a declared row, so the
matrix cannot land while `elidex-js`'s `default` is red: either the no-engine build is made green, or its shape
changes. PM 498 reserves the second ("収束しないか feature の形を変える設計変更なら PM へ"). The slot
`#11-elidex-js-no-default-features-support` already recorded a fork ("(1) engine-always … recommended — no
parser-only consumer exists vs (2) truly support parser-only"), and its trigger ("S5 boa→VM cutover … OR a
parser-only consumer appears") had fired: `elidex-shell` depends on `elidex-js` (`crates/shell/elidex-shell/Cargo.toml:31`).

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

**PR-A1's fix kinds** (Layering mandate). The layer that owns a piece of code decides the fix's kind and its unit.
Which lines carry a diagnostic does not: the unit is the whole code that one layer owns — a function, or the part of
a function that operates on one owner's state — never only the diagnosed lines.

- **(a) Relocate** engine-bound code to the existing engine/non-engine seam of the layer that owns its state, or, for
  an algorithm, to the engine-independent crate where the Layering mandate puts it. `vm/host/` is not a destination
  for lifecycle code: the Layering mandate limits it to prototype install, brand check and JsValue↔Entity
  marshalling, and a second home for the same cleanup would break One issue, one way. The known case is `HostData`'s
  unbind cleanup. `Vm::unbind` (`crates/script/elidex-js/src/vm/lifecycle.rs:15-518`, in a core module that is not
  gated: `mod lifecycle;`, `vm/mod.rs:36`) has two parts, one per owner. `:16-69` is ungated and operates only on
  `HostData` (its `if hd.is_bound()` block `:30-67` clears the mutation dispatcher, transient observers and the
  live-range / NodeIterator / TreeWalker / Selection state, then `:68` calls `hd.unbind()`); 13 of §9's 15 errors
  are in that block. `:90-517` clears `VmInner`-owned state and is already under `#[cfg(feature = "engine")]`
  (`:90`). `HostData` already has the seam: `#[cfg(feature = "engine")] mod engine_feature`
  (`vm/host_data/mod.rs:22-23`), whose `bind.rs` holds the bind lifecycle (`HostData::unbind` at
  `vm/host_data/engine_feature/bind.rs:118`), and the non-engine stub `HostData` (`vm/host_data/mod.rs:697-721`,
  with a no-op `unbind()` at `:711` and `is_bound()` at `:713`). So the unit of (a) here is the whole `:30-67` block,
  moved into the engine side of that seam (PR-A1's plan names the method); `Vm::unbind`'s ungated part then calls
  only what both `HostData` definitions provide. The `VmInner` part stays with `Vm`, where its state lives; whether
  `VmInner`'s engine-gated fields belong in the core is a question for the split, not for PR-A1.
- **(b) Gate in place** where the code is right to keep in the core and only what it names is wrong: code outside a
  gate names an optional-dependency item, or a `cfg(not(feature = "engine"))` branch does (`vm/gc/collect.rs:45-49`
  declares its empty map with the key type `elidex_ecs::Entity`, and `elidex-ecs` is optional). The fix makes each
  branch name only items its own configuration compiles with.
- **(c) Non-optional dependency.** A site whose right fix makes a dependency non-optional is a shape change. It is
  decided neither in PR-A1 nor in this umbrella: PR-A1's plan names it and returns it to PM (PM 520), and PM's ruling
  is applied in PR-A1 if PM names it, otherwise in PR-A1′.

PR-A1's plan carries a Layering check: one line per fix, giving its kind and, for (a), the owning layer's seam or the
engine-independent crate it moves to.

**The known (c) candidate, measured**: `elidex_plugin::EngineMode` (`vm/init.rs:11`, `elidex-plugin` optional,
`crates/script/elidex-js/Cargo.toml:80`). The public `Vm::new()` (`init.rs:30`) is ungated and passes
`EngineMode::BrowserCompat` to `new_with_scope` (`:31`). `new_with_scope` takes `engine_mode: EngineMode` in every
build (`:163`) but reads it only at `:177` (`engine_mode.spec_level_policy()`, under `:176`'s `cfg(feature =
"engine")`); `:162` allows the unused variable otherwise, and `:158-160` says "In non-`engine` builds the scope kind /
mode are unused". `:184-185` is the compat ceiling, which reads the derived `spec_level_policy`, not `engine_mode`.
`new_with_mode` is `#[cfg(test)]` (`:47`), and `new_worker` / `new_service_worker` are `#[cfg(feature = "engine")]`
(`:80`, `:131`). So a gate (the import, the parameter and the `:31` argument) appears to suffice, and the (c) path
would then be unused for this site. U0 does not decide it: PR-A1's plan classifies it, and PM rules any site the plan
returns.

**What the interim carries** (stated, not argued):

- After PR-M1, every `elidex-js` PR is also gated by the `default` row, and the `cfg(feature = "engine")` decision
  tax stays until the split replaces it: 1996 occurrences of `feature = "engine"` in 420 files at `69fbe61f`
  (`git grep -o 'feature = "engine"' 69fbe61f -- crates/script/elidex-js | wc -l`; files:
  `git grep -l 'feature = "engine"' 69fbe61f -- crates/script/elidex-js | wc -l`).
- In return, the `default` row checks cfg hygiene structurally from PR-M1 on (§0): no name from an optional
  dependency is used outside code gated on it, in code or in docs.
- It does not check layer correctness. PR-A1's Layering check covers PR-A1's own fixes only. Engine-gated code
  already outside `vm/host/` — 207 of the 419 `src` files carrying `feature = "engine"`
  (`git grep -l 'feature = "engine"' 69fbe61f -- crates/script/elidex-js/src | grep -v /vm/host/ | wc -l`; without
  the `grep -v`, 419) — and engine-gated code added during the interim are checked for layer only by each PR's own
  review (`/elidex-review` Axis 1). The structural guarantee comes with the split.
- It does not make the no-engine configuration ES-complete: its typed-array family is absent (`vm/host/mod.rs:74-75`,
  `:101-102`, `:343-356` are `engine`-gated) while `ObjectKind::TypedArray` / `DataView` are not gated against a
  gated `ArrayBuffer` (`vm/object_kind.rs:420-421` vs `:677`, `:694`). That gap is VM P4's
  `#11-vm-typed-array-family-layering-and-gate` (`docs/plans/2026-07-vm-p4-slot-ledger.md:310`), whose observable
  (i) is conditional "once the non-`engine` configuration builds" and whose "Until then observable (ii) is this
  row's only runnable one" ends when PR-A1 lands. PM routes that to VM P4; U0 only records it (§8).

**Observation for the split umbrella** (recorded, not planned here; the re-homed slot points here, §8, PM 545 P2).
All counts at `69fbe61f`.

- `vm/gc/collect.rs` holds 179 of the 187 `cfg(not(feature = "engine"))` sites under `crates/`
  (`git grep -n 'not(feature = "engine")' 69fbe61f -- crates | wc -l` → 187; the same with
  `-- crates/script/elidex-js/src/vm/gc/collect.rs` → 179). 176 of them are `None` fills that keep
  `GcRoots.proto_roots`'s fixed-length array aligned, all inside the array literal at `:59-1089`
  (`git grep -n -A1 'not(feature = "engine")' 69fbe61f -- crates/script/elidex-js/src/vm/gc/collect.rs | grep -c 'None,'`
  → 176); 2 are the empty `subclass_array_proto_roots` / `subclass_array_ctor_roots` slices (`:1093`, `:1097`); 1 is
  the empty `active_cssom_rule_ids` map (`:45-49`).
- A `None` fill exists for each **engine-gated** prototype field of the core VM struct (e.g.
  `abort_signal_prototype`, gated at `:81-82`), because the core GC roots those fields by position. Ungated
  prototype fields — `event_target_prototype` and `node_prototype` (`:76-77`) — are rooted in every configuration
  and produce no fill.
- The no-engine configuration already holds DOM interface surface. `register_event_target_prototype()` runs ungated
  (`vm/globals.rs:166`; `register_node_prototype()` is gated, `:174-175`). In that configuration
  `EventTarget.prototype`'s `addEventListener` / `removeEventListener` / `dispatchEvent` are no-op stubs
  (`vm/host/event_target.rs:82`, `:529`, `:633`), and the unhandled-rejection dispatch is a stub that returns `false`
  (`vm/natives_promise.rs:632`). These 4 are the host-less behaviours among the 8 `cfg(not(feature = "engine"))`
  sites outside `collect.rs`
  (`git grep -n 'not(feature = "engine")' 69fbe61f -- crates ':!crates/script/elidex-js/src/vm/gc/collect.rs'`); the
  other 4 are the `HostData` stub (`vm/host_data/mod.rs:697`), an unused-variable allow (`vm/init.rs:162`) and two
  match arms (`vm/gc/trace.rs:591`, `vm/object_kind.rs:1598`).
- `event_target` is the only `vm/host/` module with no-engine content. 31 of the 165 `mod` declarations in
  `vm/host/mod.rs` carry no `cfg` attribute
  (`git show 69fbe61f:crates/script/elidex-js/src/vm/host/mod.rs | awk '/^ *#\[/{a=1;next} /^ *\/\//{next} / mod [a-z_0-9]+;$/{if(!a)n++; t++; a=0; next} {a=0} END{print n, t}'`
  → `31 165`), but 29 of those 31 files open with `#![cfg(feature = "engine")]`, and `elements.rs` gates each of its
  items; only `event_target.rs` compiles code into the no-engine build (checked file by file, verified 2026-11-09:
  `git show 69fbe61f:<file> | grep -c '^#!\[cfg(feature = "engine")\]$'` → 1 for 29 of the 31, 0 for `elements.rs`
  and `event_target.rs`).
- A language/host crate split decides where those prototype roots, that module and those stubs live.

**Retired with F1-a**: draft 1's condition on F1-a (a structural guard for host-freedom once `engine` is gone,
possibly a new `trip-wires` entry) and its question Q5 no longer apply — under F1-b the `default` row is the cfg
hygiene guard. §7 records Q5 as retired.

## §6 PR list

| PR | Owner | Scope | Acceptance | After |
|---|---|---|---|---|
| **PR-A1** | VM P4 lane (`vm/lifecycle.rs`, `vm/init.rs`, `vm/gc/collect.rs`, `vm/natives_json/mod.rs`, plus the destination files of fix kind (a) — for the known case `vm/host_data/mod.rs` and `vm/host_data/engine_feature/*`; PR-A1's plan fixes the final list) | F1-b shape only (§5): fix the leaks so `elidex-js`'s `default` (`default = []`) row is green in both columns, keeping `app` and `browser` green, on three OS. Fix kinds as §5 defines them, each unit decided by the layer that owns the code: (a) relocate engine-bound code to the existing engine/non-engine seam of its owning layer, (b) gate in place, (c) non-optional dependency → named in the plan and returned to PM; the plan carries the Layering check (§5). `vm/gc/collect.rs` is 2074 lines (`wc -l`): its split is already assigned to VM P4's umbrella 7b (`docs/plans/2026-07-vm-p4-touch-time-split-record.md:84`); PR-A1's plan takes that record's decision, not a second one. | Relative to the slot (PM 336): each diagnostic named in §9 is gone, each one newly surfaced is named in the PR body and gone; every elidex-js row (`default` / `app` / `browser`) exits 0 in both columns (`clippy -D warnings`, `rustdoc -D warnings`) on a three-OS run of the PR's branch — the `feature-profiles` job does not exist yet, so a scratch commit on the branch adds the three rows' commands as a temporary step of the `check` job and a later commit on the branch removes it before merge (never on `main`; run id in the PR body); each fix is listed with its kind (a)/(b)/(c). A returned-to-PM site does not land open: PR-A1 lands only after PM has ruled on each such site and the ruling is applied — in PR-A1 if PM names it (listed as (c)), otherwise in PR-A1′, which lands first — so no row is skipped (PM 531; I2, I8). | F1 (decided, PM 520); PR-A1′ if it exists (PM 543) |
| **PR-A1′** | Named by PM | Exists only if PM rules a site PR-A1 returned to be a non-optional-dependency fix (a shape change) and names a PR other than PR-A1 to apply it. Applies that ruling. | Relative, so it can land before PR-A1 (PM 336, PM 543): the returned site's diagnostic is gone, and the `app` and `browser` rows stay green in both columns on three OS. The `default` row may stay red, only on diagnostics PR-A1 names. | PM's ruling |
| **PR-A2** | CI/tools (PM 543: `elidex-style` is L1's file and L1 has no driver; the lane that owns the matrix owns the rows it finds red) | `parallel` is enabled by no member: either declare a profile and make it green (connect), or change the feature's shape — remove it, or make it always on — which is PM's ruling, applied by PR-A2′. Conditions (PM 543): (1) the plan first measures `parallel`'s consumers (which crate or profile would enable it), what it gates (the `rayon` use sites) and what its red is (a gate leak, or the feature's shape); (2) a gate-only fix is CI/tools' call; entering a shape branch, the plan writes the derivation by the authority order (spec, then elidex principles — parallelism touches Concurrency by ownership and phases, "shared rayon pool") and returns to PM; (3) the plan's `elidex-style` site list is recorded via PM. | I4 holds for `elidex-style` (`parallel` and `rayon`) at PR-M2b's base; on the connect branch, the `parallel` row is green in both columns on three OS (I8). | C0; connect branch also PR-M1 (it writes PR-M1's table) |
| **PR-A2′** | Named by PM | Exists only if PM rules a shape change of `parallel`. Applies that ruling. | The ruling applied; I4 holds for `elidex-style`; the member's rows green. | PM's ruling |
| **PR-A3…** | Per row of C0's list (PM 545 P1): the lane owning the module, if that lane has a driver (L3 / Layout / VM P4 files stay with their lane); CI/tools if it has none — gate fixes only, a site needing another fix kind returns to PM, and the site list is recorded via PM | One PR per owner. Fix kinds as PR-A1's, read in the member's own terms: (a) relocate to the place the Layering mandate, or the crate's own existing feature/non-feature seam, assigns in that crate; (b) gate in place; (c) non-optional dependency → PM. | Same form as PR-A1: relative to C0's named diagnostics, both columns, three OS, each fix listed with its kind. | C0 |
| **PR-M1** | CI/tools | Declaration (I1) for the base; the deriver tool crate (entry point inside the `rust` filter, §4.1); job `feature-profiles` on three OS, each row run in two columns, `clippy -D warnings` and `rustdoc -D warnings` (I3, PM 542 (1)); `mise run check-profiles`; retire #585's step and `check-default` (I6) and replace them at every instruction-file site that names them — at `69fbe61f` (`git grep -n 'check-default'`; `git grep -n -e 'default-features clippy' -e 'clippy --workspace -- -D warnings' -- ':!docs/plans'`): `.claude/skills/pre-push/SKILL.md:28`, `:30`, `:34` (`:34`'s serial fallback would otherwise call a deleted task and skip the rows), `.claude/skills/external-review/project.md:17`, `.claude/skills/external-converge/project.md:13`, `README.md:85`, `CONTRIBUTING.md:46`, `:55`, re-grepped on PR-M1's own base. Each of these lists what CI or `mise run ci` runs (CONTRIBUTING.md's `## CI` list is `:50-58`), so deleting the entry would leave the list incomplete: each names `check-profiles` / `feature-profiles` in its place — the #585 precedent (PM 262 "[1] 3 more instruction-file places"; PM 265 fixed them in the same PR); rewrite the fixture step's comment and the profile prose in the three Cargo.toml comments into pointers (incl. the stale `crates/script/elidex-js/Cargo.toml:20-22`, §9); timeout derived from its run, both columns included. CLAUDE.md needs no edit: its `## CI` lists no jobs (CLAUDE.md:93-95) and it names no `check-default`. | Every row green in both columns on the PR's own three-OS run; the run produced ≥ 1 row per workspace member; two controls, each a scratch commit recorded in the PR body and not merged: one that breaks one profile only (e.g. an ungated engine-only import) turns exactly that row's clippy column red while `check` stays green, and one that adds an intra-doc link to an item one profile compiles out turns exactly that row's rustdoc column red while `doc` stays green; the grep above returns no live instruction site naming the retired step or task; the merged deriver has no mode that skips a row or reports a red row as green (C0's report-only setting is never merged, §7), and a de-duplicated row is reported with the row whose result it shares (I2×I3). | every row-fixing PR (PR-A1, PR-A1′, PR-A3…); Python-pin PR-2 (PM 536) |
| **PR-M2a** | CI/tools (consult: Layout lane for the fixture script, A2 owner for the absence) | I5: `test-only` and per-profile absence declarations (App: both `web-storage` features); the classification and the forbidden / unknown checks per row with `-e normal,build,features`; absorb and delete `scripts/check-fixture-boundary.sh` (+ step, filter line); discharge remedies (i)–(iii) of `#11-test-only-feature-boundary-guard`, which stays open narrowed to (iv) (§4.2, §8; PM 545 P2). | Controls, each recorded: a normal edge enabling `fixtures` → red; `elidex-storage-core/web-storage` reached in the App row → red, and `elidex-script-session/web-storage` likewise; an undeclared implicit optional-dependency feature enabled from a normal edge (the slot's `foo-test-support`) → red; a `cargo tree` output with no feature node → red; the `fixtures` dev edges of the base → green. Timeout re-derived. | PR-M1 |
| **PR-M2b** | CI/tools | I4 completeness per row; closes `#11-test-only-feature-boundary-guard` (its remaining remedy (iv), PM 545 P2). | Controls, each recorded: a feature no row activates → red (incl. an implicit one); the browser row covering both `web-storage` features while App forbids them → green. Timeout re-derived. | PR-M2a, PR-A2 or PR-A2′ |
| **PR-T** | CI/tools (job) + VM P4 (test gating) | Defect: a non-full profile's own behaviour is executed by no test in that profile — in the `default` row the no-op `EventTarget` method stubs (`vm/host/event_target.rs:82`, `:529`, `:633`) and the unhandled-rejection fallback (`vm/natives_promise.rs:632`); in the `app` row the compat ceiling with `compat-webapi` off (`vm/init.rs:184-185`). Run each declared row's tests and doc-tests under that row's flags (per-profile doc-tests are PR-T's, PM 542 (1)); its plan first runs the test-target census (Q7). A test that compiles in a profile and fails there is a defect: it is fixed in a fix-first PR (owner by PR-A3…'s rule) that lands before PR-T. | Every declared row's test targets compile and pass, and its doc-tests pass, under the row's flags on three OS, run by the same entry point (per row one `cargo nextest run -p <member>` and one `cargo test -p <member> --doc`); per row, the executed-test count is > 0 for every member whose `--all-features` run has tests (non-vacuity); a test may be cfg-excluded from a profile only when its subject is compiled out of that profile, each such test named in the PR body with the cfg that removes its subject, and no test is skipped by name; the Q7 census is in the PR body. Timeout re-derived. | PR-M1 (it needs the rows and the entry point, nothing from I4 or I5) |

Every row's PR plan goes through `/elidex-plan-review` before implementation (PM 498). PR-A1's plan is the VM P4
lane's; this umbrella fixes only its acceptance, its fix kinds and its return-to-PM boundary (§5). **PR-M1 is a
terminal unit**: I6×I8 puts retiring #585's step and adding the rows in one PR, and the deriver, declaration and
`check-profiles` without the job would be configuration nothing runs; its own plan-review confirms the slice.

## §7 Census C0 and open questions

**C0** (measurement, not a PR; permitted by PM 501, "空き 50G を切ったら止める" — it stops when free disk falls under
50 GB): on the base PR-M1 will be cut from, every row of §4.1's declaration, on macOS locally and on Linux/Windows
through a draft CI run of PR-M1's branch, both columns (§4.3); plus I4/I5 over each row's resolve.
 Report-only is a setting of that
unmerged draft run (e.g. `continue-on-error` on a scratch commit), never a flag of the tracked deriver. Output: the
red rows with their diagnostics → the PR-A3… list, each row with its owning lane and whether that lane has a driver
at the time; the list goes to PM (PM 545 P1), which settles each PR-A3…'s owner by §6's rule. Until C0 runs, the only measured row set is
elidex-js's three (§9) — **1 of 54 members measured**.

Questions (for scout / PM; Q3 and Q5 are retired by PM 520 and Q4 is answered, kept so the numbering holds):

- **Q1** C0 itself (above).
- **Q2** (PM) Where PR-A1 (F1-b shape, §6) sits in the VM P4 lane's queue (guard memo, S1, call-depth guard, …), and
  whether any in-flight VM P4 branch touches PR-A1's files: the four with diagnostics and the destination files of
  its fix kind (a) (for the known case, `vm/host_data/mod.rs` and `vm/host_data/engine_feature/*`; PR-A1's plan
  fixes the final list, §5).
- **Q3** — retired: F1 is decided (PM 520), so the merge it proposed is superseded by PM 531 (§8).
- **Q4** — answered: the Layout lane's landing list
  (`elidex-pm-trial/handoff/pending-at-landing/c0-program.md`) has 0 hits for `fixture`, `ci.yml`, `clippy`,
  `check-default` and `585` (`/usr/bin/grep -c`, 2026-11-08). The widened population (plans naming the step or the
  script) is §4.4's coordination list.
- **Q5** — retired: it served F1-a's condition only, and F1-a was not taken (§5).
- **Q6** (scout) Can an off-the-shelf runner read a `package.metadata` declaration, or only CLI flags? (PR-M1's
  implementation choice.)
- **Q7** (scout) Does `cargo test -p elidex-js --no-run` compile, and does `cargo test -p elidex-js --doc` pass, with
  each declared feature list? (PR-T. The
  `#[cfg(test)]` `Vm::new_with_mode` takes `EngineMode`, `vm/init.rs:47-49`, so the `default` row's tests need PR-A1's
  `EngineMode` treatment too.)
- **Q8** (PR-M1) Row cost per OS leg on the census base.
- **Q9** (PM) Who owns the A2 absence guarantee today (`docs/plans/2026-06-web-api-compat-a2-storage-demotion-plan.md`
  §4 (C), `:261`) and is encoding it as a declared absence acceptable to them?

## §8 Slots

- `#11-elidex-js-default-features-build-broken` — discharged when PR-M1 lands (the gate) with every row-fixing PR
  landed (the code). Its ledger "Update (PM 498)" line carries the corrected breakdown (PM 501), and its head
  sentence no longer lists `dispatch.rs:998` (applied, PM 542 (4); re-check:
  `grep 'default-features-build-broken' memory/project_open-defer-slots.md | grep -c 'dispatch.rs:998'` → 0 on
  2026-11-09; the slot id occurs on one line of the ledger).
- `#11-elidex-js-no-default-features-support` — stays open, re-homed to the VM P4 lane, its defect restated as the
  language-core/host boundary being held only by `cfg` annotations that are themselves broken, and closed when the
  crate-split umbrella lands (PM 531; already so in the ledger). Its Re-eval-date 2026-11-30 is the interim's
  re-evaluation (§5). PMO adds to it, via PM, a pointer to §5's "Observation for the split umbrella", taken at this
  draft's commit (PM 545 P2), so the observation is reachable from the slot the split starts from.
- `#11-test-only-feature-boundary-guard` — narrowed, not discharged, by PR-M2a (PM 545 P2): PR-M2a discharges
  remedies (i)–(iii) (§4.2) and moves the attach point from the retired #585 step to the matrix; the slot stays open
  narrowed to (iv) (product features no member enables) and is closed by PR-M2b. Ledger Re-eval 2026-12-15, so (iv)
  keeps a dated open slot while PR-M2b waits for PR-A2.
- `#11-ownerless-node-creation` (Layout) — its landing note says C0b's facet is "checked by
  `scripts/check-fixture-boundary.sh` in CI", which PR-M2a deletes; the property moves into I5. PMO updates the note,
  via PM, when PR-M2a lands (§4.4 coordination).
- `#11-vm-typed-array-family-layering-and-gate` (VM P4, `docs/plans/2026-07-vm-p4-slot-ledger.md:310`) — not this
  program's; its "Until then" premise ends when PR-A1 lands (§5). PM routes it to VM P4.
- No new slot: PR-T is in this program, not deferred, and per-profile rustdoc is a column of PR-M1's matrix (PM 542
  (1), §4.3).

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
  compiles — a gap shared with `check`, not introduced or closed here. The ledger already holds it, and states it more
  strongly (aarch64 Linux does not build; CI Linux is x86_64 only): `#11-os-sandbox-enforcer-correctness` S7, homed
  at the M4-12 exit-gate security gate (backstop 2026-12-15).
- Third-party crates' features (outside I4/I5's population, §2 I5).
- Merge blocking: `main-protection` has no required status check, so a red `feature-profiles` job is red, not
  blocking — the eyeball-before-squash convention remains the final gate.
