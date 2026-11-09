# Umbrella U0 — per-feature-profile build matrix (draft 5.3, 2026-11-09)

## §0 Status

**Lane**: CI/tools (gate policy). **Base**: `origin/main` at **`69fbe61f`** (#609, 2026-11-08). Every path, line and
count below was read at that commit; the command that produced each count is printed next to it.

**Slots**: `#11-elidex-js-default-features-build-broken` (owner CI/tools; this umbrella is its program) and
`#11-elidex-js-no-default-features-support` (the "is the no-engine build a supported surface" question; answered by
PM 520, §5; re-homed to VM P4 by PM 531, §8). `#11-test-only-feature-boundary-guard` is coupled (§4.2: PR-M2a
narrows it to remedy (iv), PR-M2b closes it, PM 545). `#11-private-intra-doc-links-unchecked` (CI/tools) is resolved
by PR-M1 (PM 559, §8). `#11-host-data-full-decomposition` is coupled to PR-A1 (§6 PR-A1 hand-off, PM 564).
`#11-vm-typed-array-family-layering-and-gate` (VM P4) is affected: a premise of it ends when PR-A1 lands (§8).

**What this memo is**: the umbrella of an edge-dense program — PM 498 ("umbrella U0（designer）+ PR ごとの plan、
実装前に /elidex-plan-review"): the decisions on the four axes PM 501 fixed ("4 軸 = (i)…(iv)"), the coupled
invariants (§2), the PR list with owners, scope, acceptance and order (§6). Per-PR implementation detail belongs to
each PR's own plan, and each PR plan goes through `/elidex-plan-review` before implementation.

**F1 decided (PM 520, user decision).** Whether `elidex-js` built without `engine` is a supported surface was a
feature-shape question, which PM 498 reserved ("収束しないか feature の形を変える設計変更なら PM へ"). §5 holds the
decision record only — the decision, PM 520's reasons and the options as put to PM; it does not re-derive the
decision (PM 547), and the plan content that used to follow it is in §6 (PR-A1 hand-off) and §8 (PM 559). The user
chose **F1-c** (split the crate). Per the contingent path draft 1 stated, the split is its own umbrella, owned by the
VM P4 lane and **outside this umbrella's scope**; **F1-b** (keep `default = []`, fix the gating) is the interim. So
elidex-js declares three rows (§4.1) and PR-A1 (VM P4 lane) has the F1-b shape only. U0 fixes PR-A1's acceptance,
its fix kinds and the three contracts it is held to (§6, PR-A1 hand-off); the destination and the unit of each fix
are PR-A1's own plan's (PM 564).

**What a row asserts** (stated once here, used everywhere): in that configuration, a row's lib and bin targets
compile under `clippy -D warnings` and document under `rustdoc -D warnings` with private items documented (so a doc
link from any item, public or private, to an item the configuration compiles out is red) — **cfg hygiene**, of the
code and of its docs (the two **compile columns**, §4.3). A row does not assert that the configuration is
ES-complete, nor that the code is in the right layer (§2 I2, §8.1). Behaviour in the configuration — its tests and
doc-tests, the two **test columns** — is PR-T's. A member's **doc row** (§2 I2) is not a configuration: it runs the
rustdoc column only, with every feature of the member that some row turns on, plus the member's declared test-only
features, so that the code those features gate has a rustdoc reader too (PM 569, PM 573).

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
  table — by PR-M1, which reads them for the doc rows (I2), and read by I5 from PR-M2a on (PM 573). The prose
  comments that describe profiles today become pointers to the declaration, not restatements.
- **I2 — derived population; what a row asserts.** The rows are `workspace_members` (from `cargo metadata`) ×
  ({`default`} ∪ declared profiles) × OS legs. Never a list in YAML or a script. A row asserts cfg hygiene (§0) in
  its compile columns; not ES completeness of the configuration (the no-engine `default` row has no typed-array
  constructors, §8.1), not layer correctness (§8.1), not behaviour (PR-T's). A member cannot opt its `default` out: a
  default that should not be a declared configuration is a feature-shape question (PM 498, ruled for elidex-js by PM 520), not a skipped row. An unparseable declaration, an
  unknown key, or a profile or `test-only` entry naming a feature the member does not have is red, never skipped.
  **Doc rows** (PM 569, PM 573): each member also gets one **doc row**, which runs the rustdoc column only. Its
  feature set is the union of (a) the member's `[features]` entries (as `cargo metadata` lists them) that are active
  for that member in the resolve of some row of the population above — a row of any member, its `default` or a
  declared profile — and (b) the member's declared `test-only` list; it is passed in the flag form a profile is
  (I1×I3). The set is derived from those rows' own resolves, read by the reader contract (I3×I4), so nothing new
  is declared and the declaration is still read in one place (I1). Why it exists: the rustdoc column documents only the row's member
  (`--no-deps`, §4.3) under that row's flags, so without it two kinds of code would have no rustdoc reader once the
  `doc` job is retired (I6) — code gated on a feature of the member that only another member's row turns on (a; at
  the base, both `web-storage` features, §4.1), and code gated on a test-only feature, which no row turns on because
  test-only features are never product (b; I5). The clippy column needs no such row: a row's clippy lints the
  workspace path dependencies it compiles, at that row's resolved features, under `-D warnings` (§7 Q10, measured),
  so (a)'s code is linted in the row that turns it on, and (b)'s by the `check` job's `--all-features` clippy (the
  test configuration). It runs no test column either. **Emission** is I2×I3's de-duplication applied to it: when its
  resolved feature set equals that of one of the member's rows, that row's rustdoc column already reads it, and the
  run reports the doc row with that row. A feature that no row turns on and that is not test-only is in no doc row's
  set — at the base, `elidex-style/parallel` (§10). It is not a supported configuration: I4 does not count its resolve
  (I4) and I5 does not apply to it (I5). Non-vacuity: the run asserts it produced at least one row per workspace
  member, and one doc row per member, run or reported as de-duplicated.
- **I3 — isolation.** Every Cargo invocation of a row selects that row's member alone (`-p <member>`; named
  profiles via `--no-default-features --features <list>`), and no invocation selects two members; separate
  invocations are what the Cargo Book names as the way to avoid cross-package unification (a doc row too:
  `-p <member>` alone, I2). A row has one invocation per column, and its columns come in two pairs:
  - the **compile columns** (PR-M1) build lib + bin targets only — `clippy -- -D warnings` and
    `doc --no-deps --document-private-items` with `RUSTDOCFLAGS=-D warnings` (§4.3, PM 542 (1), PM 559). Under
    resolver 2 (`Cargo.toml:2`) a lib/bin build activates no dev-dependency feature (Cargo Book §3.4
    #feature-resolver-version-2), so these two columns see the member's production configuration. A warning that
    exists only in that configuration (e.g. an import left unused, or a doc link from a public or private item to an
    item compiled out) is red;
  - the **test columns** (PR-T) build test targets — `nextest run` and `test --doc`. The same anchor says
    "Dev-dependencies do not activate features unless building a Cargo target that needs them (like tests or
    examples)", so these two columns see the production configuration plus the member's own dev edges (e.g.
    `elidex-ecs/fixtures` in `elidex-js`'s test build, I3×I5): not the production configuration, and not the
    workspace union either.

  The compile columns are the authoritative check of what builds; I4/I5 read the resolve, which is an approximation
  of it (I5).
- **I4 — completeness.** Every feature in a member's `[features]` as `cargo metadata` lists it — Cargo's implicit
  optional-dependency features included; excluding only the `default` key and declared test-only features — is
  active in the resolve of at least one row. Which entries are implicit does not change the gate (implicit and
  explicit features are both in the population). For counting only, this memo identifies an implicit feature by an
  elidex-side rule: a `packages[].features` entry `k` whose value is exactly `["dep:k"]`. That is how 1.99.1's
  `cargo metadata` shows one (measured); the Cargo Book does not define the representation, and an explicit feature
  written `k = ["dep:k"]` would look the same — at `69fbe61f` no manifest writes one
  (`git grep -n '"dep:' 69fbe61f -- '*Cargo.toml'` → 1 line, `elidex-script-session`'s two-element `web-storage`).
  Measured on each row's own resolve (read by the reader contract, I3×I4), never on the workspace resolve, and
  never on a doc row's (I2): that row turns test-only features on, and whatever they enable must not count as covered; every other feature it turns on is
  active in some row's resolve already (its set is derived from them), so counting it could add only what must not
  count — what test-only features enable, or what only the doc row's combination enables (a `?/` value whose
  dependency another feature of its set pulls in), which no supported configuration has. This also makes every
  unclassified feature that no row activates red.
- **I5 — forbidden and unknown features (fail-safe).** Each feature of a workspace member is classified from
  `cargo metadata` (manifest data, exact): **product** = reachable from some member's declared profiles (its
  `default` included) through `[features]` definitions, across members (`"dep/feat"`, `"dep?/feat"`), but not
  through a dependency declaration's `features = [...]`; **test-only** = declared test-only by its member; a feature
  in both is a declaration error (red). In each row's resolve: an active test-only feature is red; an active feature
  in the profile's declared absences is red; an active feature that is neither product nor test-only is **unknown →
  red** — implicit optional-dependency features included (the slot's `foo-test-support` enabled from a normal edge
  is this case). App's absences are both A2 edges: `elidex-storage-core/web-storage` and
  `elidex-script-session/web-storage` (`crates/script/elidex-js/Cargo.toml:35-38`). Third-party crates' features are
  outside the population (no member declares them). **What I5 can assert**: the resolve is read by the reader
  contract (I3×I4) through `cargo tree`, which "doesn't guarantee the exact equivalence to what Cargo is going to
  build" (Cargo Book §4.3.8 #feature-unification). So I5 is an approximation of the built configuration, fail-closed
  on what it reads (the contract's (3), I3×I4). It asserts what Cargo's resolver reports as activated for that
  row's compile columns' invocations; the row's compile (I3) remains the authority for what builds. **I5 does not
  apply to a doc row** (I2, PM 573), not even with its test-only features excluded, because each of I5's three rules
  there is either a false red or a re-check: (1) an active test-only feature is the doc row's purpose (I2 (b)), so
  that rule is red on every doc row with one; (2) declared absences belong to a profile, and a doc row is no profile:
  its set is the union of what rows of different profiles turn on, so a profile's absence can be active in it by
  construction — `elidex-storage-core`'s doc row turns `web-storage` on because the browser row does, while the App
  row forbids it, and both hold (I4×I5); (3) under the unknown rule, a feature active in a doc row is either active in
  some row's resolve, where I5 already reads it, or reached only through `[features]` definitions from the doc row's
  set (a test-only feature's, or a `?/` value whose dependency another feature of the set pulls in), which makes it
  active in no row's resolve, so I4 is red on it unless a row covers it or it is declared test-only. Nothing is left
  unchecked. **I5 does not
  apply to the test columns** (PR-T): a test build turns on the member's own dev edges by design (I3), so a test-only
  feature active there (`elidex-ecs/fixtures`) is the intended case, and reading the test build's resolve would be
  red on every member with a `fixtures` dev-dependency (the false red of I3×I5). Applying it with dev edges excluded
  would re-read the row's normal-and-build resolve (I3×I4), which I5 already checks for that row, so nothing is
  left unchecked: a product or unknown feature reaches a test build through the normal edges I5 reads, and a
  feature enabled only by a dev edge is active in no row's resolve, so I4 is red on it unless it is declared
  test-only.
- **I6 — one mechanism.** After the program, the configurations CI compiles without dev-only features are exactly
  the matrix rows, and rustdoc runs in exactly one place, the matrix's rustdoc column (PM 559). #585's step and
  `mise check-default` are retired in the same PR that adds the rows (PR-M1; PM 501 (ii), "#585 の step を matrix に
  吸収して同じ PR で廃止"); the `doc` job and its local mirrors are retired in the same PR on the same line (PM 559);
  `scripts/check-fixture-boundary.sh` is absorbed into I5 (PR-M2a). The `check` job's `--all-features` steps stay:
  they are the test configuration (dev-only features on, `--all-targets`), a different property. Of what the `doc`
  job read under `--all-features`, the code gated on test-only features, and a member's code gated on a feature that
  only another member's row turns on, are read by the doc rows (I2, PM 569, PM 573), in the same rustdoc column;
  `elidex-style/parallel`'s is not read until PR-A2 (§10).
- **I7 — CI shape, parity, cost.** A dedicated job `feature-profiles` (NEW) on the same three OS as `check`, rows
  (every column the job runs at that point) run serially inside one entry point; `mise run check-profiles` (NEW)
  runs the same entry point (host OS only); `timeout-minutes` derived from the job's own run, every column included
  (§4.3).
- **I8 — land green.** No PR of the program lands red, and the matrix PR lands only on a base where every row is
  green on all three OS. Adding a profile to the declaration later is adding a row: the PR that adds it makes it
  green (or carries the fix) in the same PR.

| Pair | Intersection |
|---|---|
| I1×I2 | The deriver reads only `cargo metadata --format-version 1`; a malformed declaration is a red row, not a dropped one. |
| I1×I3 | A profile is one feature list, passed as `--no-default-features --features` to each of the row's invocations; the declaration cannot express cross-member unification, by construction. |
| I1×I4 | Completeness is computed against every `[features]` key (implicit ones included) minus `default` and declared test-only ones; a new feature with no covering row is red in the PR that adds it, which forces its declaration there. |
| I1×I5 | Classification is computed from the declaration alone (profiles' `[features]` closure; `test-only` lists), never from which edges happen to enable a feature; the fixture script's hard-coded `elidex-ecs` / `fixtures` literal becomes the `test-only` entry PR-M1 declares (I1), which I5 reads from PR-M2a. |
| I1×I6 | The Cargo.toml prose (§1 list) becomes a pointer to the declaration; profile semantics are not restated in comments. |
| I1×I7 | The declaration lives in `Cargo.toml` files, already inside ci.yml's `rust` path filter (`ci.yml:34-48`, `crates/**`). |
| I1×I8 | Declaring a profile adds rows on three OS; the declaring PR owns their green. |
| I2×I3 | Population × isolation = invocation count = cost; the only permitted reduction is de-duplicating rows whose (member, resolved feature set, OS) is identical, computed per run, never listed. De-duplication is not a skip: the configuration is still compiled and documented, once, and the run reports the de-duplicated row with the row whose result it shares, so a redundant declaration stays visible. |
| I2×I4 | Completeness is only as strong as the population; the non-vacuity assertion (≥ 1 row per member) stops a silently shrinking derivation from making I4 vacuous. Doc rows are outside I4's count (I4), so they cannot satisfy it either. |
| I2×I5 | Forbidden checks run per row on each OS leg; `cargo tree`'s `--target` defaults to the host platform (Cargo Book §4.3.8 #option-cargo-tree---target), so the three legs see three hosts' target-specific edges (the fixture script today sees one host per leg too, but through a workspace union). |
| I2×I6 | The workspace-union configuration is not a row (§4.2 says what that gives up). |
| I2×I7 | Row count after de-duplication (doc rows included) × three legs drives the timeout; a PR adding members or profiles re-derives it (§4.3). |
| I2×I8 | The census (C0, §7) is the population evaluated on the base before PR-M1; PR-M1's own three-OS run is the land-green evidence. |
| I3×I4 | **The reader contract** (PM 575; every check that reads a row's resolve — I2's doc-row derivation, I4, I5 — reads it by this cell and does not restate it). (1) *Whose resolve*: each row's resolve is read from that row's own invocation (`-p <member>` with the row's feature flags, normal and build edges, no dev edge, I3×I5); reading the workspace resolve (`cargo metadata`'s `resolve` is "for the entire workspace", §4.3.5 #json-format) would re-hide what unification hides. (2) *What is read*: the **resolved feature set of each package** in that resolve — for every package, the features Cargo reports as activated for it — and a check's population is that set, never the feature nodes a traversal of the forward `cargo tree -e features` graph renders. In the forward graph, a feature that the root member's own `[features]` turns on through a `?/` value is not rendered as a feature node (measured, §4.1: on the browser row `elidex-storage-core/web-storage` and `elidex-script-session/web-storage` are active and have no node, and so does a `?/` value added to the App row's `engine`), so a check defined on the display shape is green on exactly that case — the false green PM 575 rules out. A per-package view (for example `cargo tree -f '{p} {f}'`, §4.1) is one way to read it; which invocation and format the deriver uses is PR-M1's and PR-M2a's plans' (PM 564). (3) *Fail-closed*: a cargo failure, a reading in which the row's own member does not appear (non-vacuity; what `scripts/check-fixture-boundary.sh:23-28` does today with a feature node), or a line the reader cannot parse is red (I5). |
| I3×I5 | Same reader (I3×I4's contract). Edge kinds are explicit: `cargo tree`'s default is `normal,build,dev` (§4.3.8 #option-cargo-tree---edges), and `-e features` alone adds the other kinds; measured on the pinned 1.99.1: `cargo tree -p elidex-js --features engine,compat-webapi -e features` is byte-identical to `-e normal,build,dev,features` and contains an `elidex-ecs feature "fixtures"` node (dev edge), `-e normal,build,features` contains none; in the per-package view the same holds (`cargo tree -p elidex-js --features engine,compat-webapi -e normal,build,dev -f '{p} {f}' --prefix none` lists `elidex-ecs` with `fixtures`, `-e normal,build` with no feature; 2026-11-09, `69fbe61f` code) — so a row that read dev edges would be falsely red on every member with a `fixtures` dev-dependency. The fixture script already uses `-e normal,build,features` (`scripts/check-fixture-boundary.sh:17`). The same fact is why I5 reads the compile columns' resolve only and does not apply to the test columns (I5). |
| I3×I6 | The `--all-features` steps are not rows: they turn every dev-only feature of the workspace union on for every target. PR-T's test columns are per row: one member, the row's flags, and only the dev edges that member's own test targets declare. |
| I3×I7 | Serial invocations share one target dir; that Cargo keys artifacts on the feature set (so rows do not rebuild or contaminate each other) is not stated in the Cargo Book (§3.9 Build Cache does not mention features); PR-M1 measures it on its run for the compile columns (a second row with a different feature set leaves the first row's artifacts fresh). A test column changes the dependencies' feature sets (dev edges, I3), so it can rebuild what the compile columns built; PR-T measures the reuse for the test columns on its own run, as PR-M1 does for its columns. A separate job gets its own rust-cache key, so its feature sets do not evict `check`'s. |
| I3×I8 | A row's verdict does not depend on row order (each row's invocations are its own; artifact reuse is PR-M1's measurement, I3×I7), so a green census predicts a green PR-M1 run up to base drift. |
| I4×I5 | A feature can be required by I4 and forbidden by I5 at once only per profile: App forbids both `web-storage` features, the browser row covers them. Both hold; PR-M2b's controls include exactly this case. |
| I4×I6 | I4 is strictly stronger than #585's step: #585 compiled features some member enables; I4 also reaches features no member enables (`elidex-style/parallel`). |
| I4×I7 | One resolve reading per row (I3×I4's contract), inside the same entry point, shared with I5. |
| I4×I8 | `elidex-style/parallel` (and its implicit `rayon`) is uncovered today, so I4 is red at the base: it needs its fix (declare + green, or a shape change PM rules) before I4 lands. So I4 is its own PR, PR-M2b, after PR-A2; I5 (PR-M2a) does not wait for it (§4.4). |
| I5×I6 | PR-M2a retires `scripts/check-fixture-boundary.sh` and its ci.yml step and path-filter line, moving the assertion into I5; `#11-test-only-feature-boundary-guard` attaches here instead of to the retired #585 step. |
| I5×I7 | The forbidden and unknown checks run on every OS leg. |
| I5×I8 | At PR-M2a's base the App row must satisfy both A2 absences (census C0 includes it), and no active feature may be unknown. At `69fbe61f` no member's dependency declaration or `[features]` value names another member's implicit feature, and `engine` lists all 25 of `elidex-js`'s implicit features (`cargo metadata --no-deps --format-version 1`, `packages[].dependencies[].features` and `packages[].features`; 26 implicit features in total by I4's counting rule: `elidex-js` 25, `elidex-style` `rayon`)
, so the unknown rule is green there by measurement of the manifests; C0 confirms it on the resolves. |
| I6×I7 | `[tasks.ci]` (`mise.toml:142`) replaces `check-default` and `doc` with `check-profiles` in the same PR. |
| I6×I8 | Retiring #585's step and the `doc` job and adding the rows, doc rows included, are one PR: no commit on `main` has neither, or both. That is why the `test-only` declaration is PR-M1's (I1, PM 573): with it in PR-M2a, the code gated on test-only features would have no rustdoc reader from PR-M1 to PR-M2a. |
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
| Cargo Book §3.4 Features | #feature-resolver-version-2 | dev-dependencies activate no feature for lib/bin targets, and do for a target that needs them (tests) | I3 (compile vs test columns), I5 (not on test columns) | ✓ | no |
| Cargo Book §3.1.1 Cargo Targets | #the-required-features-field | "If any of the required features are not enabled, the target will be skipped" | PR-T acceptance (exclusion paths) | ✓ | no |
| Cargo Book §3.4 Features | #resolver-version-2-command-line-flags | `--features` / `--no-default-features` apply to the `-p` selection | row invocation shape, the doc row's included (I2) | ✓ | no |
| Cargo Book §3.4 Features | #optional-dependencies | an optional dep without `dep:` defines an implicit feature | I4 population, I5 unknown rule | ✓ | no |
| Cargo Book §3.4 Features | #feature-combinations | full coverage is exponential; the book encourages each project to choose its configurations | I1/I2 (declared profiles, not a powerset) | ✓ | no |
| Cargo Book §3.1 The Manifest Format | #the-metadata-table | `package.metadata` "completely ignored by Cargo", for tools | I1 (declaration home) | ✓ | no |
| Cargo Book §3.5 Profiles | #profiles | "profile" already means build profile | §0 terminology, §4.5 | ✓ | no |
| Cargo Book §4.3.5 cargo metadata | #description | "The output format is subject to change"; "It is recommended to include the --format-version flag" | I1, I2 (deriver input) | ✓ | no |
| Cargo Book §4.3.5 cargo metadata | #json-format | `packages[].metadata`, `packages[].features`, `workspace_members`; `resolve` is the whole workspace's | I1, I5 classification, I3×I4 | ✓ | no |
| Cargo Book §4.2.6 cargo doc | #target-selection | with no target selection, documents "all binary and library targets of the selected package" | §4.3 rustdoc column (same targets as the clippy column) | ✓ | no |
| Cargo Book §4.2.6 cargo doc | #option-cargo-doc---document-private-items | "Include non-public items in the documentation" | §0, I3, §4.3 rustdoc column (PM 559) | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #feature-unification | "doesn't guarantee the exact equivalence to what Cargo is going to build"; `-e normal,build` is "pretty close" to `cargo build` | I5 (what it can assert), I3 (compile is authoritative) | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #option-cargo-tree---edges | default kinds `normal,build,dev`; `features` alone adds the others | I3×I5 flags | ✓ | no |
| Cargo Book §4.3.8 cargo tree | #option-cargo-tree---target | default is the host platform | I2×I5 (per-leg hosts) | ✓ | no |
| ECMA-262 §4.2 Hosts and Implementations | #sec-hosts-and-implementations | host-defined facilities are deferred to an external source; "Implementations that are not hosts may also provide definitions for host-defined facilities" | §5 reasons (the spec clause of PM 520's quote) | ✗ | no |
| WHATWG HTML §8.1.6 JavaScript specification host hooks | #javascript-specification-host-hooks | HTML is the host that defines those hooks | §5 reasons (the spec clause of PM 520's quote) | ✗ | no |
| WHATWG DOM §2.7 Interface EventTarget | #add-an-event-listener | step 5 appends the listener to the event target's event listener list; `dispatchEvent()` returns "the result of dispatching" | PR-T expected values (the no-engine `EventTarget` stubs) | ✗ | no |
| WHATWG DOM §2.9 Dispatching events | #concept-event-dispatch | dispatch returns false only if the event's canceled flag is set | PR-T expected values (`dispatchEvent`) | ✗ | no |
| ECMA-262 §27.5.1.9 HostPromiseRejectionTracker | #sec-host-promise-rejection-tracker | host-defined; "The default implementation of HostPromiseRejectionTracker is to return unused" | PR-T expected value (the no-engine unhandled-rejection fallback) | ✗ | no |

**Breadth**: K=4 sources (preflight count), M=22 rows. The spec rows are ✗ for full enumeration on purpose. The
ECMA-262 §4.2 and HTML §8.1.6 rows are the sections behind the spec clause of PM 520's reasons, quoted in §5 (the
host-hook structure exists; the spec is silent on a host-less build), and nothing here implements a hook. The DOM
rows and the HostPromiseRejectionTracker row are the authorities PR-T's expected values are written against
(PM 564); PR-T writes no algorithm, it asserts these steps' observable results. Preflight's split recommendation
(K ≥ 4 or M ≥ 20) is what this umbrella already is: each PR below has its own plan and plan-review (§6, PM 498).

**Repo rules that govern (no § number)**: CLAUDE.md 至上命題 (authority order), Design philosophy (core/compat
layering, plugin-first, ScriptSession as the sole Script↔ECS boundary, Concurrency by ownership and phases —
PR-A2's shape branch, Security by structure and Ideal over pragmatic — with the Layering mandate, PM 520's reasons,
§5), Layering mandate (also contract (1) of the PR-A1 hand-off, §6),
Design discipline (One issue one way, dead code, Supported-surface testing, Edge-dense, 1000-line touch-time split),
`## CI` (CLAUDE.md:93-95: ci.yml's job comments are canonical); ci.yml's job-boundary rule (`ci.yml:236-245`), its
tamper-path reasoning (`ci.yml:182-189`, `:229-231`) and its `tools`-job timeout rule (`ci.yml:250-269`); PM 336
(relative acceptance), PM 410 (matrix direction), PM 498 / PM 501 (this umbrella's frame), PM 520 / PM 531 (F1),
PM 536 (ordering against Python-pin PR-2, citation form), PM 255 / PM 261 / PM 265 (the mechanisms §4.2 folds),
PM 542 (rustdoc column, per-profile doc-tests), PM 543 / PM 545 (PR-A1′ acceptance, the PR-A2 and PR-A3… owners,
the guard slot's narrowing), PM 547 / PM 559 (§5's form), PM 558 (pre-matrix three-OS evidence), PM 559 / PM 560 /
PM 561 (the rustdoc column's private items, the `doc` job, the PR-D… prereq), PM 562 (PR-T0…, PR-A3′, the test
columns), PM 564 (no per-PR design in U0; PR-T's expected values), PM 569 (rustdoc coverage of test-only code in
the matrix, PR-A2's stop rule and deadline, `#11-no-engine-eventtarget-stubs-nonconforming`), PM 573 (the
`test-only` declaration is PR-M1's; one doc row per member), PM 575 (the reader contract: per-package resolved
feature sets, I3×I4).


### §3.1 User-input touch audit

None. No row is reached by web content; the program changes CI, manifests' metadata tables, the deriver, and the
code fixes PR-A* make (whose own plans run this audit).

## §4 Decisions

### §4.1 Axis (i) — where profiles are declared, how the matrix is derived

- **Declaration**: `[package.metadata.elidex.feature-profiles]` (NEW) in the member's own `Cargo.toml`, next to its
  `[features]`. One entry per named profile: its feature list and, optionally, its declared absences (I5); plus one
  `test-only` list of the member's test-only features. Exact TOML schema is PR-M1's plan (PR-M1 adds the `test-only`
  key, which its doc rows read, I2 and I6×I8, PM 573; PR-M2a adds the absence key). Co-location is the reason: the
  features and their supported combinations change in the same diff,
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
    (`compat-webapi`'s `"…?/web-storage"` values, I5), and `fixtures` / `test-hooks` are test-only: PR-M1 declares
    them so (PM 573). The doc rows (I2) that this gives, at `69fbe61f`: `elidex-ecs` {`fixtures`} and `elidex-net`
    {`test-hooks`} by (b); `elidex-storage-core` {`web-storage`} and `elidex-script-session` {`web-storage`} by (a),
    each on in two rows' resolves, `elidex-js`'s browser row and `elidex-shell`'s `default` row (which enables
    `elidex-js`'s `engine` + `compat-webapi`, `crates/shell/elidex-shell/Cargo.toml:31`). None of the four members has
    another feature, so each doc row is the member's whole `[features]` on — the same set of the member's own
    features the `doc` job's `--all-features` turns on. Every other member's doc row is de-duplicated (I2×I3):
    `elidex-js`'s set equals its browser row's, `elidex-style`'s is empty (`parallel` and `rayon` are on in no row),
    and a member with no feature has only its `default`. Measured by reading, for each of the 56 rows (54 `default`
    rows, `app`, `browser`), `cargo tree -p <member> <row flags> -e normal,build -f '{p} {f}' --prefix none` (each
    package's activated features — the reader contract's per-package resolved feature sets, I3×I4) and keeping, per
    member, the entries of its `cargo metadata --no-deps --format-version 1` `features` keys.
  - **Why the contract reads per-package sets, not feature nodes** (the reproduction behind PM 575; re-run
    2026-11-09 on the `69fbe61f` code, rustc 1.99.1, macOS; `/usr/bin/grep`), on the browser row's flags
    `-p elidex-js --no-default-features --features engine,compat-webapi`:
    1. `cargo tree <flags> -e normal,build,features | grep -c 'elidex-storage-core feature "web-storage"'` → `0`
       (the forward graph renders no such node);
    2. `cargo tree <flags> -e normal,build,features -i elidex-storage-core | grep -c 'feature "web-storage"'` → `4`
       (the inverted graph renders it: 1 `elidex-storage-core feature "web-storage"` line, plus 3
       `elidex-script-session feature "web-storage"` lines on the inverted paths, since that feature enables it too);
    3. `cargo tree <flags> -e normal,build -f '{p} {f}' | grep elidex-storage-core` → every `elidex-storage-core
       v0.1.0 (…)` line ends `web-storage` (the per-package view; the command's first hit is the `elidex-js` line,
       whose `{f}` lists the implicit feature `elidex-storage-core`);
    4. the control PR-M2a records (§6), on the App row's flags `-p elidex-js --no-default-features --features
       engine`: add the line `"elidex-storage-core?/web-storage",` as the first value of `engine` in
       `crates/script/elidex-js/Cargo.toml`; (1) on those flags → `0`, (3) → every `elidex-storage-core` line ends
       `web-storage` (unmutated: with no feature); then `git checkout -- crates/script/elidex-js/Cargo.toml` and
       `git status --porcelain` → empty.

    Cause: both `web-storage` features are turned on by the root member's own `?/` values,
    `"elidex-storage-core?/web-storage"` and `"elidex-script-session?/web-storage"`
    (`crates/script/elidex-js/Cargo.toml:36-37`, in `compat-webapi`). The second is hidden the same way: (1) with
    `'elidex-script-session feature "web-storage"'` → `0`, (3) with `elidex-script-session` → its `elidex-script-session
    v0.1.0 (…)` lines end `web-storage`. Both are App's declared absences (I5), so a check defined on the forward graph's nodes would read
    both as absent on any row whose own `[features]` reached them — step 4, PR-M2a's `?/` control, is this case.
- **Derivation**: at run time, by one tracked entry point that reads `cargo metadata --format-version 1`
  (`workspace_members`, `packages[].metadata.elidex`, `packages[].features`), emits the rows, and runs them serially.
  No row list is ever checked in (ci.yml carries no per-row YAML). **Implementation home**: PR-M1's plan's choice,
  under these requirements: it reads *this* declaration (I1), one member per invocation (I3), the same entry point
  locally and in CI (I7), and it needs only the runtimes the job declares (the job-boundary rule, `ci.yml:236-245`).
  **Entry-point constraint**: the entry point lives inside the `rust` path filter (ci.yml and mise.toml already are,
  `ci.yml:46-47`; so is anything under `crates/**`). An entry point anywhere else (a `scripts/*.sh`, a runner
  config) is registered in the `rust` filter in the same PR, as `scripts/check-fixture-boundary.sh` is
  (`ci.yml:39`); otherwise a PR editing only the entry point skips the job that runs it — the tamper path ci.yml's
  own comments describe (`ci.yml:182-189`). Whether an existing runner reads a `package.metadata` declaration is
  unverified (Q6).

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
  row's own resolve by the reader contract (I3×I4), because the slot's recipe is a workspace union, its `-e
  features` includes dev edges, which makes every member with a `fixtures` dev-dependency falsely red (measured,
  I3×I5), and its feature nodes miss what a root member's own `?/` values turn on (§4.1, PM 575).
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
  under §4.1's entry-point constraint all its inputs (manifests, the deriver, ci.yml, mise.toml) are inside the
  `rust` filter, so the tamper path that keeps `trip-wires` / `tools` ungated (`ci.yml:182-189`, `:229-231`) does not
  arise. Not a step of `check`: a separate job runs in parallel with `check` instead of lengthening it, and gets its
  own rust-cache key (I3×I7).
- **Compile columns** (PM 542 (1), PM 559): each row runs `cargo clippy -p <member> <row flags> -- -D warnings` and
  `cargo doc -p <member> --no-deps --document-private-items <row flags>` with `RUSTDOCFLAGS=-D warnings` — the
  flag the `doc` job sets today (`ci.yml:134-136`). Both compile or document the member's lib and bin targets
  (`cargo doc` with no target selection "will document all binary and library targets of the selected package",
  Cargo Book §4.2.6 #target-selection); `--document-private-items` ("Include non-public items in the
  documentation", §4.2.6 #option-cargo-doc---document-private-items) makes a private item's broken doc link red
  too, which is what §0 asserts. A doc row (I2) runs this column alone, with the same flags. This column is
  the only place rustdoc is checked (PM 559): it is in this matrix,
  not a second job, because a second job would be a second reader of the declaration and a second place deciding
  rustdoc's flags (One issue, one way). Per-profile doc-tests run code, so they are PR-T's test columns.
- **The `doc` job** (`ci.yml:120-136`, `--all-features`, Ubuntu) and its local mirrors (mise `doc`, `doc-full`,
  `scripts/doc-changed.sh`; `mise.toml:52-81`) are absorbed into the rustdoc column and retired in PR-M1, on the
  line PM 501 (ii) drew for #585's step ("#585 の step を matrix に吸収して同じ PR で廃止"; PM 559). Code gated
  on a test-only feature, or on a feature of a member that only another member's row turns on, is not a reason to
  keep it (PM 569, PM 573): the doc rows read it in the rustdoc column (I2). `elidex-style/parallel`'s code stays
  unread until PR-A2, a §10 gap with PR-A2's deadline.
  The private-item warnings the flag turns red at the base are fixed before PR-M1 by the PR-D… prereq (§6,
  PM 560 (a)).
- **OS legs**: `ubuntu-latest`, `macos-latest`, `windows-latest`, `fail-fast: false` — the same set as `check`
  (`ci.yml:61-65`), for #585's own reason (`ci.yml:105-107`: default-feature builds differ per target because
  `cfg(target_os/windows/unix)` sites exist). Not a GitHub matrix over rows: per-row legs would each cold-build the
  shared dependency graph.
- **mise parity**: `mise run check-profiles` (NEW) runs the same entry point; `[tasks.ci]` depends on it instead of
  `check-default` and `doc`. Local runs cover the host OS only; the other two legs are CI-only (stated in the task's comment).
- **Timeout**: derived the way ci.yml's `tools` job does (`ci.yml:250-269`): PR-M1 sets `timeout-minutes` to
  `ceil(4 × slowest leg's end-to-end minutes)` of its own run, every column the job runs included (the `tools` job's "about 4x",
  `ci.yml:257`), and records run id and leg time in the job's comment; any later PR that adds rows (members or
  profiles) or row work (PR-M2a / PR-M2b's resolve checks, PR-T's tests and doc-tests) re-derives it from its own
  run (`ci.yml:250`). Whether a re-derivation may lower the value is PR-M1's plan's call: the `tools` job's
  raise-only rule is program-scoped ("raise-only for the program, PM 353", `ci.yml:267`; #586's plan §7), not a general rule. The `check` job's `timeout-minutes: 20`
  (`ci.yml:60`) is not touched.
- **Cost**: unmeasured (Q8). Each row is an incremental clippy plus a `--no-deps --document-private-items` rustdoc of
one member (a doc row, the rustdoc only), whose
  dependency artifacts are shared whenever the feature set matches (PR-M1 measures that reuse, I3×I7); only members
  whose resolve differs from rows already built pay. PR-M1's plan measures it on the census; it is judgment input,
  not a reason to shrink the population (I2).


### §4.4 Axis (iv) — cross-lane ordering

1. **F1 decided** (PM 520, §5): F1-b is the interim, so PR-A1 has the F1-b shape and elidex-js declares three rows.
2. **Census C0** (§7) on the then-current base — names every red row and every I4/I5 violation, with owners.
3. **Row-fixing PRs**: **PR-A1** (VM P4 lane) and any **PR-A3…** the census finds, each with the owner §6's PR-A3…
   rule gives it (PM 545 P1), in any order among themselves. A site one of them returns to PM is applied by the PR
   PM names: the returning PR itself, or **PR-A1′** / **PR-A3′** (exists only if PM names a PR other than the
   returning one; owner named by PM), which lands before the returning PR: its acceptance is relative and does not
   need the returning PR's rows green (§6, PM 543). Either way the returning PR does not land before the ruling is
   applied (PM 531), and the deadline of §6's PR-A3′ row bounds the wait.
4. **PR-M1** (CI/tools) — reviewed and ready *before* PR-A1 lands; lands in a PM-held landing window directly after
   the latest of (a) the last row-fixing PR of step 3, (b) the last **PR-D…** (the private-doc prereq, §6, PM 560
   (a); deadline = when PR-M1's implementation is ready, PM 561) and (c) **Python-pin PR-2**: PR-M1 edits ci.yml,
   so by decision 150 (3) it lands after Python-pin PR-2 (PM 536: "PM 494 は decision 150 (3)…を上書きしていない";
   `docs/plans/2026-10-python-pin-umbrella.md:599-602`). It lands after a fresh three-OS run on the new head.
   Reason for the window: the VM P4 lane lands into `elidex-js` weekly, and the default configuration has been red
   for six months with no gate (§1); every day between PR-A1 and PR-M1 is a day it can go red again unseen. PR-A1
   does not wait for PR-2 (PM 558 (2)): if PR-2 has not landed when PR-A1 is ready, PR-A1 lands and that gap is
   accepted until PR-M1. The rows' three-OS evidence before PR-M1 comes from a temporary step that is not in the
   merge (§6, PM 558 (1)). When PR-M1 is ready and PR-2 has not landed, or a PR-D… or a returned site has not
   landed, the driver raises it to PM with the reason (PM 536).
5. **PR-M2a** (CI/tools; I5), after PR-M1. It needs nothing else: I5 is green at the base by measurement (I5×I8).
6. **PR-A2** (CI/tools, PM 543), after C0. Its connect branch declares a `parallel` profile in PR-M1's table, so
   that branch lands after PR-M1 with its row green (I8); a shape branch (remove, or always on) is ruled by PM and
   applied by **PR-A2′** (owner and PR named by PM; exists only if PM rules a shape change). Stop rule and deadline
   (§6, PM 569): when PR-M2b's implementation is ready, the driver raises to PM a PR-A2 not yet landed or a returned
   shape still without an applied ruling; PM's ruling names the applying PR and a date, and that date bounds PR-M2b's
   wait (step 7) and so PR-T's (step 9).
7. **PR-M2b** (CI/tools; I4), after PR-M2a (I4 reads each row's resolve through the reader PR-M2a adds for I5,
   under I3×I4's contract, I4×I7) and after PR-A2 or PR-A2′, by the date step 6 bounds. Until it lands, `#11-test-only-feature-boundary-guard`
   stays open, narrowed to (iv), with its Re-eval date (§8).
8. **PR-T0…** (fix-first, §6), after PR-T's Q7 census, which PR-T's plan runs first; it needs no PR of steps 4–7
   (its three-OS evidence is the temporary step, §6). A behaviour site one of them returns to PM is applied by
   **PR-T0′** (named by PM), as in step 3.
9. **PR-T** (§6), after PR-M2b — PM 501 (iv)'s order, "F1 → C0 → PR-A* → PR-M1 → PR-M2 → PR-T" (PM 559) — and
   after every PR-T0… and PR-T0′. Its wait on PR-A2 is bounded through PR-M2b (step 6).

**Why I5 and I4 are two PRs** (PR-M2a, PR-M2b): from PR-M1 to the fixture script's absorption, two mechanisms
answer the test-only question (the workspace-union, host-only script and the matrix) — the strangler interval One
issue, one way forbids keeping. Only I4 waits for PR-A2, and PR-A2 can return to PM as a shape change, whose date PM
sets only at PR-A2's deadline
(step 6).
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
- Every lane with a PR-D… share (L3, VM P4, Layout, and CI/tools for the driverless files): its doc-only PR before
  PR-M1 (§6; PM relays the list, PM 561).

### §4.5 Naming

"Feature profile" everywhere this program writes (declaration key, job, task, comments), because Cargo's "profile" is
the build profile (Cargo Book §3.5) and the `check` job already passes nextest a `--profile` (`ci.yml:90`).

## §5 F1 — is `elidex-js` without `engine` a supported surface? (DECIDED: F1-c, F1-b interim — PM 520)

This section holds three things only: the decision, PM 520's reasons for it, and the options as they were put to PM
(PM 559). The decision of record is PM 520. This memo does not re-derive it (PM 547). PR-A1's fix kinds and
contracts are §6's PR-A1 hand-off; what the interim carries and the observation for the split umbrella are §8.

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
  future crate split would need. Its condition in draft 1 (a structural guard for host-freedom once `engine` is gone,
  possibly a new `trip-wires` entry) and its question Q5 were retired with it (§7).
- **F1-b — support the host-less build as is** (taken as the interim). Keep `default = []`, fix the gating. Not a
  shape change *if* no fix makes a dependency non-optional; a site whose right fix is that returns to PM. PR-A1 = the
  targeted fix (the 15 named diagnostics of §9, plus whatever later compiler phases surface once name resolution
  passes — the count is a lower bound).
- **F1-c — split the crate** (language crate + host crate) (taken). Shape change and a program of its own; listed
  so the choice is made knowingly, not because this umbrella can carry it.

## §6 PR list

| PR | Owner | Scope | Acceptance | After |
|---|---|---|---|---|
| **PR-A1** | VM P4 lane (PM 520 (1); the PR-A1 hand-off below) | F1-b shape only (§5): fix the leaks so `elidex-js`'s `default` (`default = []`) row is green in both compile columns, without adding a diagnostic to `app` or `browser`, on three OS. Fix kinds, contracts and the Layering check: the PR-A1 hand-off below; the destination and the unit of each fix, and so PR-A1's file list beyond the files carrying §9's diagnostics, are PR-A1's plan's (PM 564). `vm/gc/collect.rs` is 2074 lines (`wc -l`): its split is already assigned to VM P4's umbrella 7b (`docs/plans/2026-07-vm-p4-touch-time-split-record.md:84`); PR-A1's plan takes that record's decision, not a second one. | Relative to the slot (PM 336): each diagnostic named in §9 is gone, each one newly surfaced is named in the PR body and gone; every elidex-js row (`default` / `app` / `browser`) exits 0 in the clippy column, and in the rustdoc column has no warning outside the PR-D… population (below) — the `default` row's rustdoc warnings, which become measurable once PR-A1 makes it compile, are listed in the PR body and handed to the owning lane's PR-D… (PM 561); all on three OS by the pre-matrix evidence (below); each fix is listed with its kind (a)/(b)/(c). A returned-to-PM site does not land open: PR-A1 lands only after PM has ruled on each such site and the ruling is applied — in PR-A1 if PM names it (listed as (c)), otherwise in PR-A1′, which lands first — so no row is skipped (PM 531; I2, I8). | F1 (decided, PM 520); PR-A1′ if it exists (PM 543) |
| **PR-A1′** | Named by PM | Exists only if PM rules a site PR-A1 returned to be a non-optional-dependency fix (a shape change) and names a PR other than PR-A1 to apply it. Applies that ruling. | Relative, so it can land before PR-A1 (PM 336, PM 543): the returned site's diagnostic is gone, and the `app` and `browser` rows gain no diagnostic in either compile column, on three OS by the pre-matrix evidence (below). "Gain" is measured against the PR's own base: those rows' rustdoc columns are not clean at `69fbe61f` (the PR-D… population; its one row-only entry, `vm/host/document.rs:477`, is in the `app` row), and the warnings PR-D… owns do not count here. The `default` row may stay red, only on diagnostics PR-A1 names. | PM's ruling |
| **PR-A2** | CI/tools (PM 543: `elidex-style` is L1's file and L1 has no driver; the lane that owns the matrix owns the rows it finds red) | `parallel` is enabled by no member: either declare a profile and make it green (connect), or change the feature's shape — remove it, or make it always on — which is PM's ruling, applied by PR-A2′. Conditions (PM 543): (1) the plan first measures `parallel`'s consumers (which crate or profile would enable it), what it gates (the `rayon` use sites) and what its red is (a gate leak, or the feature's shape); (2) a gate-only fix is CI/tools' call; entering a shape branch, the plan writes the derivation by the authority order (spec, then elidex principles — parallelism touches Concurrency by ownership and phases, "shared rayon pool") and returns to PM; (3) the plan's `elidex-style` site list is recorded via PM. **Stop rule** (PM 569; PR-A3…'s shape): a site that needs anything but a gate fix — a shape change of `parallel` (remove, or always on) included — is not fixed in PR-A2: the plan names it, writes condition (2)'s derivation and returns it to PM, and the rest of the PR proceeds. | Decidable at PR-A2's own head (PM 336), since I4's mechanism lands later (PR-M2b): on that head, `parallel` and `rayon` are each active in the resolve of some declared row, read by the reader contract (I3×I4) — I4's computation for this one member, done by hand, output recorded in the PR body; on the connect branch, the `parallel` row is green in both compile columns on three OS (I8). A returned site does not land open: PR-A2 lands only after PM has ruled on it and the ruling is applied — in PR-A2 if PM names it, otherwise in PR-A2′, which then carries this acceptance. **Deadline** (PM 569): when PR-M2b's implementation is ready (§4.4 step 6), the driver raises to PM a PR-A2 not yet landed, or a returned site still without an applied ruling; PM's ruling names the applying PR and a date, and that date bounds PR-M2b's wait and, through it, PR-T's — PR-M2b never lands with I4 red (I8). | C0; connect branch also PR-M1 (it writes PR-M1's table) |
| **PR-A2′** | Named by PM | Exists only if PM rules a shape change of `parallel`. Applies that ruling. | The ruling applied; on the PR's own head, every remaining `elidex-style` feature passes PR-A2's hand computation of I4; the member's rows green. | PM's ruling; landed by the date that ruling sets (PR-A2's deadline) |
| **PR-A3…** | Per row of C0's list (PM 545 P1): the lane owning the module, if that lane has a driver (L3 / Layout / VM P4 files stay with their lane); CI/tools if it has none — gate fixes only, a site needing another fix kind returns to PM, and the site list is recorded via PM | One PR per owner. Fix kinds (a)/(b)/(c) and contracts (1)–(2) of the PR-A1 hand-off, read in the member's own terms; (a)'s destination is the place the Layering mandate, or the crate's own existing feature/non-feature seam, assigns in that crate, chosen by the PR's own plan. **Stop rule**: a site that needs a fix kind its owner may not apply — for CI/tools anything but a gate fix, for every owner (c) — is not fixed: it is named in the PR body and returned to PM, and the rest of the PR proceeds. | Same form as PR-A1: relative to C0's named diagnostics, both compile columns, three OS by the pre-matrix evidence (below), each fix listed with its kind. A returned-to-PM site does not land open: the PR-A3… lands only after PM has ruled on it and the ruling is applied — in that PR-A3… if PM names it, otherwise in PR-A3′, which lands first. **Deadline**: when PR-M1's implementation is ready (§4.4 step 4), the driver raises to PM every returned site still without an applied ruling; PM's ruling names the applying PR and a date, and that date bounds PR-M1's wait for the row — PR-M1 never lands over a red row (I8). | C0 |
| **PR-A3′, PR-T0′** | Named by PM | Exist only if PM rules a site that a PR-A3… (→ PR-A3′) or a PR-T0… (→ PR-T0′) returned and names a PR other than the returning one to apply it. Apply that ruling — for PR-T0′, a behaviour fix of a subject or of a test (PM 562). PR-A1′'s shape. | Relative, so it can land before the returning PR (PM 336, PM 543): the returned site's diagnostic, or its failing test, is gone in its row, and the member's other rows gain no diagnostic in any column the PR runs, on three OS by the pre-matrix evidence (below). | PM's ruling; landed by the date that ruling sets (PR-A3…'s deadline) |
| **PR-D…** (private-doc prereq) | One per lane, by file (PM 560 (a), PM 561; PM 545 P1's rule): L3 136, VM P4 81, Layout 2, CI/tools 58 — CI/tools takes the files of lanes with no driver, doc-comment fixes only, and sends any behaviour judgment to PM | Fix-only: one standalone, doc-comment-only PR per lane that makes the private-item rustdoc warnings in the lane's files go away (population below), so that PR-M1's rustdoc column lands with `--document-private-items` on (PM 560 (a)). L3's `vm/host/document.rs` share lands after Layout's C1-pre (PM 561). | The lane's entries of the population, re-measured on the PR's own base, are gone; the diff touches doc comments only (`///`, `//!`); a site whose fix is not a doc-comment edit is named in the PR body and returned to PM, not fixed here. | none; deadline = when PR-M1's implementation is ready (PM 561); a PR-D… not landed then is raised to PM (§4.4 step 4) |
| **PR-M1** | CI/tools | Declaration (I1) for the base, the `test-only` lists included (`elidex-ecs` `fixtures`, `elidex-net` `test-hooks`; §4.1, I6×I8, PM 573); the deriver (entry point inside the `rust` filter, §4.1); job `feature-profiles` on three OS, each row run in the two compile columns, `clippy -D warnings` and `rustdoc --document-private-items -D warnings` (I3, PM 542 (1), PM 559), and each member's doc row in the rustdoc column (I2, PM 569, PM 573), which resolves `#11-private-intra-doc-links-unchecked` (§8); `mise run check-profiles`; retire #585's step and `check-default`, and the `doc` job and its local mirrors (mise `doc` / `doc-full`, `scripts/doc-changed.sh`) (I6, §4.3; PM 559), and replace them at every instruction-file site that names them — at `69fbe61f` (`git grep -n 'check-default'`; `git grep -n -e 'default-features clippy' -e 'clippy --workspace -- -D warnings' -- ':!docs/plans'`; `git grep -n -e 'doc-full' -e 'mise run doc' -e 'doc-changed' -e 'cargo doc --workspace' -- ':!docs/plans' ':!crates'`): `.claude/skills/pre-push/SKILL.md:28`, `:30`, `:34` (`:34`'s serial fallback would otherwise call deleted tasks and skip the rows), `.claude/skills/external-review/project.md:17`, `.claude/skills/external-converge/project.md:13`, `README.md:85`, `CONTRIBUTING.md:46`, `:54`, `:55`, and the comment at `scripts/trip-wires.sh:12-13` that cites `[tasks.doc]`'s delegation as its precedent, re-grepped on PR-M1's own base. Each of these lists what CI or `mise run ci` runs (CONTRIBUTING.md's `## CI` list is `:50-58`), so deleting the entry would leave the list incomplete: each names `check-profiles` / `feature-profiles` in its place — the #585 precedent (PM 262 "[1] 3 more instruction-file places"; PM 265 fixed them in the same PR); rewrite the fixture step's comment and the profile prose in the three Cargo.toml comments into pointers (incl. the stale `crates/script/elidex-js/Cargo.toml:20-22`, §9); timeout derived from its run, both columns included. CLAUDE.md needs no edit: its `## CI` lists no jobs (CLAUDE.md:93-95) and it names no `check-default`. | Every row green in both compile columns, and every doc row green in the rustdoc column, on the PR's own three-OS run; the run produced ≥ 1 row per workspace member and one doc row per member, run or reported as de-duplicated (I2); three controls, each a scratch commit recorded in the PR body and not merged: one that breaks one profile only (e.g. an ungated engine-only import) turns exactly that row's clippy column red while `check` stays green, and one that adds, to a private item's doc comment, an intra-doc link to an item one profile compiles out turns exactly that row's rustdoc column red while every other row's rustdoc column stays green; and one that adds, to the doc comment of an item gated on a test-only feature and to that of an item gated on a feature only another member's row turns on (e.g. `elidex-storage-core/web-storage`), an intra-doc link to an item that exists in no configuration turns exactly those two members' doc rows red while every other row stays green (each member's own rows compile those items out, so its doc row is their only rustdoc reader); the greps above return no live instruction site naming a retired step, job or task; the merged deriver has no mode that skips a row or reports a red row as green (C0's report-only setting is never merged, §7), and a de-duplicated row is reported with the row whose result it shares (I2×I3). | every row-fixing PR (PR-A1, PR-A1′, PR-A3…, PR-A3′); every PR-D… (PM 560 (a)); Python-pin PR-2 (PM 536) |
| **PR-M2a** | CI/tools (consult: Layout lane for the fixture script, A2 owner for the absence) | I5: per-profile absence declarations (App: both `web-storage` features), reading the `test-only` declarations PR-M1 made (PM 573); the classification and the forbidden / unknown checks per row, reading each row's resolve by the reader contract (I3×I4); absorb and delete `scripts/check-fixture-boundary.sh` (+ step, filter line); discharge remedies (i)–(iii) of `#11-test-only-feature-boundary-guard`, which stays open narrowed to (iv) (§4.2, §8; PM 545 P2). | Controls, each recorded: a normal edge enabling `fixtures` → red; `elidex-storage-core/web-storage` reached in the App row → red, and `elidex-script-session/web-storage` likewise; the same feature reached in the App row through the root member's own `[features]` `?/` value (e.g. `"elidex-storage-core?/web-storage"` added to `engine`) → red — a reader that traverses the forward graph's feature nodes is green on this case (§4.1's reproduction; PM 575), so it is the control of the reader contract; an undeclared implicit optional-dependency feature enabled from a normal edge (the slot's `foo-test-support`) → red; a reading in which the row's member does not appear → red (I3×I4 (3)); the `fixtures` dev edges of the base → green. Timeout re-derived. | PR-M1 |
| **PR-M2b** | CI/tools | I4 completeness per row; closes `#11-test-only-feature-boundary-guard` (its remaining remedy (iv), PM 545 P2). | Controls, each recorded: a feature no row activates → red (incl. an implicit one); the browser row covering both `web-storage` features while App forbids them → green. Timeout re-derived. | PR-M2a, PR-A2 or PR-A2′ |
| **PR-T0…** (fix-first) | By file, PR-A3…'s owner rule (PM 545 P1): the owning lane if it has a driver; CI/tools if not — test-gate fixes only, any behaviour fix returns to PM (PM 562) | Every test PR-T's Q7 census finds red in a row that compiles the test's subject, in one of two cases: (1) the test compiles there and fails — a behaviour defect, of the subject or of the test, judged against PR-T's expected-value rule; (2) the subject compiles there but the test does not (it names an item the profile compiles out — e.g. a `#[cfg(test)]` helper that takes an engine-only type, Q7): the test is gated in place or rewritten to name only what its configuration compiles (kind (b), applied to test code), never excluded, because its subject is in the profile. A test whose subject the row compiles out is not here: PR-T excludes it under its exclusion rule. One PR per owner; stop rule as PR-A3…'s. | Each test the census names compiles and passes in its row on three OS by the pre-matrix evidence (below); each fix is listed with its case (1)/(2) and, for (1), whether the subject or the test was wrong and by which authority. A returned site does not land open: it is applied in PR-T0′, under PR-A3…'s deadline rule with PR-T in place of PR-M1. | PR-T's Q7 census |
| **PR-T** | CI/tools (the test columns in the job and the entry point); test gating and exclusions per file by PR-A3…'s owner rule (PM 545 P1) | Defect: a non-full profile's own behaviour is executed by no test in that profile — in the `default` row the no-engine `EventTarget` method stubs (`vm/host/event_target.rs:82`, `:529`, `:633`) and the unhandled-rejection fallback (`vm/natives_promise.rs:632`); in the `app` row the compat ceiling with `compat-webapi` off (`vm/init.rs:184-185`). Run each declared row's tests and doc-tests under that row's flags — the two test columns (I3; per-profile doc-tests are PR-T's, PM 542 (1)); its plan first runs the test-target census (Q7). **Expected values come from the governing spec, never from the configuration's current behaviour** (PM 564): `addEventListener` / `removeEventListener` from DOM §2.7 Interface EventTarget ("add an event listener" step 5 appends the listener to the event listener list; `.claude/tools/webref dfn dom 'add an event listener'`), `dispatchEvent` from its method steps, which return "the result of dispatching" — false only if the event's canceled flag is set (DOM §2.9 Dispatching events, step 13) — and the unhandled-rejection path from ECMA-262 HostPromiseRejectionTracker (host-defined; "The default implementation of HostPromiseRejectionTracker is to return unused", so a host-less configuration owes script no observable report, and the fallback's stderr line is not asserted). The no-engine `EventTarget` stubs do not meet DOM §2.7 (`addEventListener` records nothing; `dispatchEvent` returns `false` for an event nobody canceled): that defect is `#11-no-engine-eventtarget-stubs-nonconforming` (VM P4, §8). PR-T names it; its tests stay red against the stubs until a PR-T0′ applies PM's ruling — which may also be that the no-engine configuration does not expose `EventTarget` at all (the split's question, §8) — and are never made green by asserting the stubs. A test that fails in a profile goes to PR-T0…. | Every declared row's test targets compile and pass, and its doc-tests pass, under the row's flags on three OS, run by the same entry point (per row one `cargo nextest run -p <member>` and one `cargo test -p <member> --doc`); per row, the executed-test count is > 0 for every member whose `--all-features` run has tests, and every test target of the member is either built in the row or named as excluded, so a silently skipped target is red (non-vacuity). **Exclusion paths**: a test may be excluded from a profile only when its subject is compiled out of that profile, by a `cfg` on the test or by a `[[test]]` target's `required-features` (Cargo skips such a target without a diagnostic, Cargo Book §3.1.1 #the-required-features-field; 0 `[[test]]` targets use it at `69fbe61f`, `git grep -n -B3 'required-features' 69fbe61f -- '*Cargo.toml'` shows only two `[[bench]]`s), each named in the PR body with its mechanism and the cfg that removes its subject. A doc-test fence's `ignore` / `no_run` is not a per-profile exclusion — it removes the example from every configuration — so PR-T and PR-T0… add none (base: 10 `ignore`, 0 `no_run`; `` git grep -n -P '```[a-z_0-9,]*ignore' 69fbe61f -- crates | wc -l ``, and the same with `no_run`). No test is skipped by name; the Q7 census is in the PR body. Timeout re-derived; the test columns' artifact reuse measured (I3×I7). | PR-M2b (PM 501 (iv), PM 559); every PR-T0… and PR-T0′ |

**Pre-matrix three-OS evidence (PM 558 (1)).** Every PR above that must show rows on three OS before the
`feature-profiles` job exists (PR-A1, PR-A1′, PR-A3…, PR-A3′, PR-T0…, PR-T0′) gets that evidence the same way: a
scratch commit on its branch adds the rows' commands (I3) as a temporary step of the `check` job, and a later commit
on the branch removes it. PM checks three conditions at merge: (i) the merge head's PR diff has 0 changes under
`.github/`; (ii) the head that ran the temporary step and the final head differ only under `.github/`; (iii) the PR
body records the run id and that head's sha. Decision 150 (3) puts after Python-pin PR-2 the ci.yml edits that
stay in the merge; a step that is not in the merge is outside it (PM 558 (1)), so PR-A1 does not wait for PR-2
(PM 558 (2)), and neither do the others for this reason.

**PR-D… population** (PM 560, PM 561). `cargo doc --no-deps --document-private-items --message-format=json`
without `-D warnings`, at `69fbe61f`'s code on macOS (rustc 1.99.1), in 57 runs — the workspace with
`--all-features`, every one of the 54 members' `default` rows, and `elidex-js`'s `app` and `browser` rows — reports
277 distinct (file, line, message) warnings over 20 crates; by owner L3 136 / VM P4 81 / Layout 2 / CI/tools 58.
Re-run: `run.sh` then `agg.py` in `elidex-pm-trial/handoff/ci-tools-u0/r2-run/r3/privdoc/` (`agg.py` reads the
`out/` directory beside it, so `run.sh`'s `OUT=` must point there); the counts from the list it writes, kept there as
`list-lanes.tsv` with owners: `tail -n +3 list-lanes.tsv | wc -l` → 277;
`tail -n +3 list-lanes.tsv | cut -f3 | sed 's/ (.*//' | sort | uniq -c` → the four owner counts;
`tail -n +3 list-lanes.tsv | cut -f1 | cut -d/ -f3 | sort -u | wc -l` → 20. Not measured: `elidex-js`'s `default`
row, where rustdoc stops at E0432 `elidex_plugin::EngineMode` (`vm/init.rs:11`, the §9 diagnostic;
`cargo doc -p elidex-js --no-deps --document-private-items`) — re-measured after PR-A1, and each increment is split
by the same owner rule (PM 561); and Linux / Windows, whose `cfg(target_os)` items C0's rustdoc column reaches —
sites found there are split the same way. The four doc rows that are not de-duplicated (I2, §4.1) add nothing to
it: measured separately on the same code and toolchain, `cargo doc -p elidex-ecs --no-deps --document-private-items
--features fixtures --message-format=json` reports 12 distinct (file, line, message) warnings, the same with
`-p elidex-net --features test-hooks` reports 3, with `-p elidex-storage-core --features web-storage` 1 and with
`-p elidex-script-session --features web-storage` 2, each set identical to that member's entries in
`list-lanes.tsv` (`grep -c 'crates/core/elidex-ecs/' list-lanes.tsv` → 12; the same with `crates/net/elidex-net/`
→ 3, `crates/storage/elidex-storage-core/` → 1, `crates/script/elidex-script-session/` → 2), which are already in
its owners' shares (CI/tools for the first three members, VM P4 for `elidex-script-session`; the `lane` column). So
for the rustdoc column, every row (doc rows included) but
`elidex-js`'s `default` has been measured on one OS; the clippy column has been measured only for `elidex-js`'s three
rows (§9, §7).

**PR-A1 hand-off (VM P4 lane).** PR-A1's owner is the VM P4 lane (PM 520 (1)). U0 fixes only what this block and
PR-A1's table row say. The destination and the unit of each fix, and so the files PR-A1 touches beyond those
carrying §9's diagnostics, are decided by PR-A1's own plan, which goes through `/elidex-plan-review` before
implementation (PM 498, PM 564).

- **Fix kinds.** Each fix is one of:
  - **(a) Relocate** engine-bound code to the existing engine/non-engine seam of the layer that owns its state, or,
    for an algorithm, to the engine-independent crate where the Layering mandate puts it;
  - **(b) Gate in place**, where the code is right to keep in the core and only what it names is wrong (code outside
    a gate names an optional-dependency item, or a `cfg(not(feature = "engine"))` branch does): each branch then
    names only items its own configuration compiles with;
  - **(c) Non-optional dependency**: a shape change, decided neither in PR-A1 nor in this umbrella. PR-A1's plan
    names the site and returns it to PM (PM 520); PM's ruling is applied in PR-A1 if PM names it, otherwise in
    PR-A1′.
- **Contracts.** Every fix, whatever its kind and unit:
  1. obeys the Layering mandate in its destination: `vm/host/` holds only prototype install, brand check and
     JsValue↔Entity marshalling, so it is no destination for lifecycle code or for an algorithm;
  2. leaves one concern's code — in particular `HostData`'s unbind cleanup — with exactly one home: no fix splits it
     between two places (One issue, one way; R2 A1-IMP-1 and R3 A2-IMP-1 were this
     defect). A unit drawn around the diagnosed lines alone is how it usually breaks;
  3. is coupled to `#11-host-data-full-decomposition` (ledger). That slot books decomposing `HostData` into
     per-subsystem sub-structs as an edge-dense design change of its own PR(s), with the bind/unbind lifecycle among
     its intersecting axes and `host_data/engine_feature/` as where the sub-structs' `impl`s land. PR-A1's plan
     names the slot, states for each fix that touches `HostData` how it stands to the slot's sub-struct boundary,
     and records in the slot (via PM) what PR-A1 moved; PR-A1 does not carry out the decomposition, which the slot keeps
     as its own PR(s), not bundled.
- **Layering check.** PR-A1's plan carries one line per fix: its kind and, for (a), its destination.
- **The known (c) candidate**: `elidex_plugin::EngineMode` (`vm/init.rs:11`, the §9 E0432; `elidex-plugin` is
  optional, `crates/script/elidex-js/Cargo.toml:80`). PR-A1's plan classifies it, and PM rules if the plan returns it.
- **What PM routes to VM P4 with it**: the typed-array slot's premise (§8), Q2 (§7), and the rustdoc warnings in
  VM P4's files, which are VM P4's PR-D… (above), not PR-A1's.

Every row's PR plan goes through `/elidex-plan-review` before implementation (PM 498). **PR-M1 is a terminal
unit**: I6×I8 puts retiring #585's step and the `doc` job and adding the rows in one PR, and the deriver,
declaration and `check-profiles` without the job would be configuration nothing runs; its own plan-review confirms
the slice.

## §7 Census C0 and open questions

**C0** (measurement, not a PR; permitted by PM 501, "空き 50G を切ったら止める" — it stops when free disk falls under
50 GB): on the base PR-M1 will be cut from, every row of §4.1's declaration, on macOS locally and on Linux/Windows
through a draft CI run of PR-M1's branch, both compile columns (§4.3, the rustdoc column with
`--document-private-items`) and each doc row's rustdoc column (I2); plus I4/I5 over each row's resolve.
Report-only is a setting of that unmerged draft run (e.g. `continue-on-error` on a scratch commit), never a flag of
the tracked deriver. Output: the
red rows with their diagnostics → the PR-A3… list, each row with its owning lane and whether that lane has a driver
at the time; the list goes to PM (PM 545 P1), which settles each PR-A3…'s owner by §6's rule. Rustdoc warnings
beyond the PR-D… population go to the owning lane's PR-D… by the same rule. Until C0 runs, the clippy column has
been measured only on elidex-js's three rows (§9) — **1 of 54 members** — and the rustdoc column on every row but
elidex-js's `default`, on macOS only (§6, PR-D… population).

Questions (for scout / PM; Q3 and Q5 are retired by PM 520, and Q4 and Q10 are answered, kept so the numbering
holds):

- **Q1** C0 itself (above).
- **Q2** (PM) Where PR-A1 (F1-b shape, §6) sits in the VM P4 lane's queue (guard memo, S1, call-depth guard, …), and
  whether any in-flight VM P4 branch touches PR-A1's files: the four that carry §9's diagnostics and PR-A1's plan's
  destination files, which that plan names (§6, PR-A1 hand-off).
- **Q3** — retired: F1 is decided (PM 520), so the merge it proposed is superseded by PM 531 (§8).
- **Q4** — answered: the Layout lane's landing list
  (`elidex-pm-trial/handoff/pending-at-landing/c0-program.md`) has 0 hits for `fixture`, `ci.yml`, `clippy`,
  `check-default` and `585` (`/usr/bin/grep -c`, 2026-11-08). The widened population (plans naming the step or the
  script) is §4.4's coordination list.
- **Q5** — retired: it served F1-a's condition only, and F1-a was not taken (§5).
- **Q6** (scout) Can an off-the-shelf runner read a `package.metadata` declaration, or only CLI flags? (PR-M1's
  implementation choice.)
- **Q7** (scout) Does `cargo test -p elidex-js --no-run` compile, and does `cargo test -p elidex-js --doc` pass, with
  each declared feature list? (PR-T; its red tests are PR-T0…'s. The `#[cfg(test)]` `Vm::new_with_mode` takes
  `EngineMode`, `vm/init.rs:47-49`, so whether the `default` row's tests compile depends on how PR-A1 classifies
  `EngineMode`, and a test left naming it is PR-T0…'s case (2).)
- **Q8** (PR-M1) Row cost per OS leg on the census base.
- **Q9** (PM) Who owns the A2 absence guarantee today (`docs/plans/2026-06-web-api-compat-a2-storage-demotion-plan.md`
  §4 (C), `:261`) and is encoding it as a declared absence acceptable to them?
- **Q10** — answered (measured 2026-11-09 on the `69fbe61f` code, rustc 1.99.1, macOS): yes. A row's
  `cargo clippy -p <A> <row flags> -- -D warnings` lints the workspace path dependencies it compiles, at the row's
  resolved features, with `-D warnings` applied; so the clippy column already reaches a feature of B that only A's
  row turns on, and only the rustdoc column (`--no-deps`) misses it, which is why the doc row (I2) runs the rustdoc
  column only. Re-runnable probe (append, run, restore; no probe stays in the tree):
  1. append to `crates/storage/elidex-storage-core/src/lib.rs` the line
     `#[cfg(feature = "web-storage")] pub fn q10_probe() -> bool { let v: Vec<u8> = Vec::new(); v.len() == 0 }`;
  2. `cargo clippy -p elidex-js --no-default-features --features engine,compat-webapi --lib --bins -- -D warnings`
     → rc 101, `error: length comparison to zero` and `` error: could not compile `elidex-storage-core` (lib) ``;
  3. controls: `cargo clippy -p elidex-storage-core --features web-storage --lib` reports the same `len_zero`, and
     the App row (`--features engine` in step 2's command) exits 0 — the probe is compiled out there, so step 2's red
     comes from the row's resolved features;
  4. `git checkout -- crates/storage/elidex-storage-core/src/lib.rs`, then `git status --porcelain` → empty.

## §8 Slots

- `#11-elidex-js-default-features-build-broken` — discharged when PR-M1 lands (the gate) with every row-fixing PR
  landed (the code). Its ledger "Update (PM 498)" line carries the corrected breakdown (PM 501), and its head
  sentence no longer lists `dispatch.rs:998` (applied, PM 542 (4); re-check:
  `grep 'default-features-build-broken' memory/project_open-defer-slots.md | grep -c 'dispatch.rs:998'` → 0 on
  2026-11-09; the slot id occurs on one line of the ledger).
- `#11-elidex-js-no-default-features-support` — stays open, re-homed to the VM P4 lane, its defect restated as the
  language-core/host boundary being held only by `cfg` annotations that are themselves broken, and closed when the
  crate-split umbrella lands (PM 531; already so in the ledger). Its Re-eval-date 2026-11-30 is the interim's
  re-evaluation (§5), whose input is §8.1. PMO items on it, via PM:
  - a pointer to §8.2 "Observation for the split umbrella", so the observation is reachable from the slot the split
    starts from (PM 545 P2). It is taken at this draft's commit; at landing, PMO re-measures it on the merge commit
    and repoints the slot to the heading's lines there (PM 559; squash merge leaves this branch's commits off `main`);
  - at PR-M1's landing, an update of its "Cross-ref (2026-11-03, PM 265, at #585)", whose "Per-package /
    per-feature checks (`cargo hack`) stay this slot's job" PR-M1 makes false: per-member, per-profile checks become
    the matrix's rows (PM 559).
- `#11-private-intra-doc-links-unchecked` (CI/tools) — resolved by PR-M1 (PM 559): the matrix's rustdoc column
  carries `--document-private-items`, and the PR-D… prereq (§6) fixes the population first (PM 560 (a), PM 561).
  Its Re-eval-date becomes PR-M1's landing; that ledger change is PMO's, applied from PR-M1's landing list
  (PM 559). The slot closes when PR-M1 lands.
- `#11-host-data-full-decomposition` — not this program's; coupled to PR-A1 by contract (3) of §6's PR-A1
  hand-off (PM 564), so PR-A1's plan names it and it records what PR-A1 moved.
- `#11-test-only-feature-boundary-guard` — narrowed, not discharged, by PR-M2a (PM 545 P2): PR-M2a discharges
  remedies (i)–(iii) (§4.2) and moves the attach point from the retired #585 step to the matrix; the slot stays open
  narrowed to (iv) (product features no member enables) and is closed by PR-M2b. Ledger Re-eval 2026-12-15, so (iv)
  keeps a dated open slot while PR-M2b waits for PR-A2.
- `#11-ownerless-node-creation` (Layout) — its landing note says C0b's facet is "checked by
  `scripts/check-fixture-boundary.sh` in CI", which PR-M2a deletes; the property moves into I5. PMO updates the note,
  via PM, when PR-M2a lands (§4.4 coordination).
- `#11-vm-typed-array-family-layering-and-gate` (VM P4, `docs/plans/2026-07-vm-p4-slot-ledger.md:310`) — not this
  program's; its "Until then" premise ends when PR-A1 lands (§8.1). PM routes it to VM P4.
- `#11-no-engine-eventtarget-stubs-nonconforming` (VM P4; registered by PM 569) — the no-engine `EventTarget`
  stubs fail DOM §2.7 (§6 PR-T). Not this program's: PR-T names it, and a PR-T0′ applies PM's ruling (§6). A PR of
  the slot that touches `vm/host/event_target.rs` is ordered with L3's brand-check PR-1 via PM.
- Otherwise no new slot: PR-T and PR-D… are in this program, not deferred, and per-profile rustdoc is a column of
  PR-M1's matrix (PM 542 (1), PM 559, §4.3).

### §8.1 What the interim carries

Stated, not argued; input to the interim's re-evaluation on `#11-elidex-js-no-default-features-support`'s
Re-eval-date.

- After PR-M1, every `elidex-js` PR is also gated by the `default` row, and the `cfg(feature = "engine")` decision
  tax stays until the split replaces it: 1996 occurrences of `feature = "engine"` in 420 files at `69fbe61f`
  (`git grep -o 'feature = "engine"' 69fbe61f -- crates/script/elidex-js | wc -l`; files:
  `git grep -l 'feature = "engine"' 69fbe61f -- crates/script/elidex-js | wc -l`).
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
  row's only runnable one" ends when PR-A1 lands. PM routes that to VM P4; U0 only records it (the slot's bullet
  above).

### §8.2 Observation for the split umbrella

Recorded, not planned here; the re-homed slot points here (PM 545 P2). All counts at `69fbe61f`.

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
- `event_target` is the only `vm/host/` module with no-engine content. 31 of the 166 `mod` declarations in
  `vm/host/mod.rs` carry no `cfg` attribute
  (`git show 69fbe61f:crates/script/elidex-js/src/vm/host/mod.rs | awk '/^ *#\[/{a=1;next} /^ *\/\//{next} /(^| )mod [a-z_0-9]+;$/{if(!a)n++; t++; a=0; next} {a=0} END{print n, t}'`
  → `31 166`; the pattern counts the line-start `mod byte_io;`, `:82`), but 29 of those 31 files open with `#![cfg(feature = "engine")]`, and `elements.rs` gates each of its
  items; only `event_target.rs` compiles code into the no-engine build (checked file by file, verified 2026-11-09:
  `git show 69fbe61f:<file> | grep -c '^#!\[cfg(feature = "engine")\]$'` → 1 for 29 of the 31, 0 for `elements.rs`
  and `event_target.rs`).
- A language/host crate split decides where those prototype roots, that module and those stubs live. The `EventTarget`
  stubs also fail DOM §2.7 (PR-T's expected values, §6), so the split's decision closes
  `#11-no-engine-eventtarget-stubs-nonconforming` too, unless PM rules it fixed before the split.

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
- **Rustdoc of `elidex-style/parallel`'s code, from PR-M1 until PR-A2** — read by the `doc` job and by no row once
  that job is retired (PM 559, PM 569). `parallel` is not test-only and is enabled by no member, so no row turns it
  on before PR-A2, and so it is in no doc row's set either (I2, PM 573: the doc row unions what rows turn on, plus
  test-only features): `git grep -n 'feature = "parallel"' 69fbe61f -- crates | wc -l` → 5 lines in 2 files (by
  `git grep -l`): 4 `cfg(feature = "parallel")` and 1 `cfg(not(feature = "parallel"))` (`walk.rs:135`), whose branch
  `elidex-style`'s `default` row does read. The `check` job's `--all-targets --all-features` clippy still compiles it;
  only its
  doc links go unchecked. **Closer**: PR-A2's connect branch (its `parallel` row runs the rustdoc column) or PR-A2′
  (removed, nothing is left to read; always on, the `default` row reads it). **Deadline**: PR-A2's (§6; PM's ruling
  at PR-M2b's readiness sets the date), with the dated backstop `#11-test-only-feature-boundary-guard`'s Re-eval
  2026-12-15, the slot whose remaining remedy (iv) is this same feature (§8). Not a gap: the code gated on
  `elidex-ecs/fixtures` (`git grep -n 'feature = "fixtures"' 69fbe61f -- crates | wc -l` → 11 lines in 3 files: 10
  `cfg(any(test, feature = "fixtures"))` and 1 doc-comment line, `node_document.rs:21`) and on
  `elidex-net/test-hooks` (`git grep -n 'feature = "test-hooks"' 69fbe61f -- crates | wc -l` → 21 lines in 2 files,
  all `cfg(feature = "test-hooks")`) — their members' doc rows read it from PR-M1 on (I2 (b)); and the code of a
  member gated on a feature that only another member's row turns on — at `69fbe61f` the two `web-storage`
  features, of the six feature-declaring members (§4.1) the only features that are neither test-only, nor on in one
  of the member's own rows, nor enabled by no member (`git grep -n -E 'feature *= *"web-storage"' 69fbe61f -- crates
  ':!*Cargo.toml'` → 4 lines in 2 files: the whole `web_storage` module of `elidex-storage-core`,
  `src/lib.rs:14-15`, its re-export `:26-27` and the doc line `:12`, and one item of `elidex-script-session`,
  `src/engine.rs:715`) — read by those members' doc rows from PR-M1 on (I2 (a), PM 573; §4.1). Its clippy is not a
  gap either: the rows that turn those features on lint it (§7 Q10, measured).
- Merge blocking: `main-protection` has no required status check, so a red `feature-profiles` job is red, not
  blocking — the eyeball-before-squash convention remains the final gate.
