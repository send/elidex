# PR-A2 — `elidex-style/parallel`: measurement, branch derivation (returned to PM), I4 by hand (per-PR plan memo, U0 §6)

## §0. Frame

**Umbrella**: U0, `docs/plans/2026-11-per-profile-build-matrix-umbrella.md` (#619). This memo is the per-PR plan of
U0 §6's **PR-A2** row and satisfies that row's conditions (1)–(3), its stop rule, its acceptance and its deadline.
It goes through `/elidex-plan-review` before anything is implemented (U0 §6, PM 498).

**Base and frame**: `origin/main` at **`b8b4d6f5`** (#619), macOS, rustc 1.99.1 (`rust-toolchain.toml`; the session
shell's `RUSTUP_TOOLCHAIN` unset, `rustc --version` → `rustc 1.99.1 (58cf8f9c3 2026-10-13)`). Every `file:line`
below is at `b8b4d6f5`. Every count carries the command that produced it. Linux and Windows are unmeasured unless a
line says otherwise.

**Rulings that bind this plan**:
- **PM 605** (C0 ruling): C0 at `b8b4d6f5` found `elidex-style/parallel` and its implicit `rayon` uncovered by I4.
  Kind PR-A2, owner CI/tools, order U0 §4.4 step 6, **due 2026-11-20**.
- **PM 543**: CI/tools owns the PR (`elidex-style` is L1's file and L1 has no driver). A gate-only fix is CI/tools'
  call. A shape branch (remove the feature, or make it always on) is PM's ruling, applied by PR-A2′. The plan writes
  the derivation in the authority order and returns it to PM.
- **PM 569**: the stop rule and the deadline are as in U0 §6.

**Outcome of this plan, stated first.** The derivation (§4) lands on a **shape change**. It recommends **always on**,
names remove as the defensible alternative, and finds connect disfavoured. So under the stop rule the branch is
**returned to PM** and not decided here. The rest of the PR proceeds as U0 §6 requires: the measurement (§1), the
I4 computation by hand (§6), and the `elidex-style` site list for PM (§7). The measurement also found a defect the
brief did not name. The two style walks that `parallel` selects attach different `ComputedStyle` components
(§1.4 F-1). That is a behaviour site, also returned to PM (§7, S2).

### §0.1 Premises re-verified

| Premise (driver's brief) | Verdict | Command (at `b8b4d6f5`) |
|---|---|---|
| `crates/css/elidex-style/Cargo.toml`: `[features] parallel = ["rayon"]`; `rayon = { workspace = true, optional = true }` at :28 | holds (`[features]` at :30, `parallel` at :31) | `cat -n crates/css/elidex-style/Cargo.toml` |
| No member enables `parallel` | holds | `git grep -n 'parallel' -- '*Cargo.toml'` → `crates/css/elidex-style/Cargo.toml:31` plus root `Cargo.toml:266`, `:280` (wasmtime's `parallel-compilation`) |
| Dependents of `elidex-style`: elidex-render, elidex-dom-api, elidex-shell, elidex-wpt, all `elidex-style.workspace = true`, no features | holds for direct dependents | `git grep -n 'elidex-style' -- '*Cargo.toml'` → `crates/core/elidex-render/Cargo.toml:22`, `crates/dom/elidex-dom-api/Cargo.toml:19`, `crates/shell/elidex-shell/Cargo.toml:28`, `crates/tools/elidex-wpt/Cargo.toml:17` |
| `parallel` and `rayon` active in no row's resolve | holds: 0 of 56 rows (§6.1) | §6.1's script |
| `src/lib.rs:19` gates `mod parallel;` | holds (`:19-20`) | `git grep -n 'feature = "parallel"' -- crates` |
| `src/walk.rs:130-145` chooses `walk_children_parallel` (:151) or the sequential loop | holds with a line shift: the cfg pair is `:130-133` / `:135-146`; `:151` is the `#[cfg]` of `walk_children_parallel`, whose `fn` line is `:153` | `sed -n 123,153p crates/css/elidex-style/src/walk.rs` |
| `src/parallel.rs`: `par_resolve_siblings` on the rayon global pool, sequential threshold at :26 | holds with a line shift: the threshold constant is `:27` (`PARALLEL_THRESHOLD = 8`), `:25-26` is its doc comment; `par_iter` runs outside any `install`, so on rayon's global pool | `sed -n 25,51p crates/css/elidex-style/src/parallel.rs` |
| clippy `--no-default-features --features parallel -- -D warnings` → rc 0 | holds | §1.3 |
| rustdoc `--document-private-items -D warnings`, same flags → rc 0 | holds | §1.3 |
| Root `Cargo.toml:289` `rayon = "1"`; wasmtime's `parallel-compilation` uses rayon's global pool (comment `:266`) | holds | `sed -n 266,289p Cargo.toml` |
| No other `crates/**/*.rs` names rayon or a thread pool | holds | `git grep -n -i 'rayon\|ThreadPool\|thread_pool' -- 'crates/**/*.rs' ':!crates/css/elidex-style' \| wc -l` → `0` |

### §0.2 Premise errors and omissions

- **Line drift, not substance**: `walk_children_parallel` is `walk.rs:153` (`:151` is its `#[cfg]`); the threshold is
  `parallel.rs:27` (`:26` is a comment line).
- **Omission: rayon is already in the shipped product's resolve.** The brief's "other rayon use" lists source files
  only. Through wasmtime's `parallel-compilation`, `rayon` / `rayon-core` are in the resolve of 4 of the 56 rows:
  `elidex-shell` `default`, `elidex-wasm-runtime` `default`, `elidex-js` `app` and `browser`. Command: for each row,
  `cargo tree -p <member> <row flags> -e normal,build -f '{p}' --prefix none | grep -q '^rayon-core v'`. The path
  is `cargo tree -p elidex-shell -e normal,build -i rayon --depth 3` → `rayon v1.12.0` ← `wasmtime v50.0.1` ←
  `elidex-wasm-runtime` ← `elidex-js`. So the browser product already links rayon and uses its global pool. Making
  `parallel` always on adds no crate to `elidex-shell`'s resolve and no second pool (§4.2).
- **Omission: the two walks are not equivalent** (§1.4 F-1). The brief describes `parallel` as a pure execution
  strategy. It is not one: the two walks attach different `ComputedStyle` components, and consumer code compensates.

## §1. Condition (1): consumers, gated sites, nature of the red

### §1.1 Who enables `parallel`

No workspace member enables `parallel` (§0.1 row 2). No profile could enable it either, because U0 PR-M1 declares
none for `elidex-style` (U0 §4.1). The resolve of a product build therefore never contains it. This includes every
one of the 16 rows whose resolve contains `elidex-style`: `elidex-style` appears in each with an empty feature set
(§6.1). The only builds that compile `parallel` are those that pass `--all-features`. These are the `check` job's
clippy, nextest and doc-test steps (`.github/workflows/ci.yml:87`, `:90`, `:93`), the `doc` job (`ci.yml:134`) and
their mise mirrors (`mise.toml:12`, `:23`, `:27`, `:38`, `:81`). The `--all-features` flag turns `parallel` on for
`elidex-style` and, by unification, for every dependent in the same invocation (Cargo Book §3.4 #feature-unification).

So **the only configuration that runs the parallel walk is the test configuration, and every shipped configuration
runs the sequential walk** (§1.4 F-2).

### §1.2 What `parallel` gates

`git grep -n 'feature = "parallel"' -- crates` → 5 lines in 2 files (`git grep -l` with the same pattern):

| Site | What it is |
|---|---|
| `crates/css/elidex-style/Cargo.toml:28`, `:30-31` | the optional `rayon` dependency (implicit feature `rayon`, Cargo Book §3.4 #optional-dependencies) and `parallel = ["rayon"]` |
| `crates/css/elidex-style/src/lib.rs:19-20` | `mod parallel;` |
| `crates/css/elidex-style/src/parallel.rs:1-181` | the whole module. `OwnedPropertyMap`, `to_owned_map`, `PARALLEL_THRESHOLD` (`:27`, 8), `par_resolve_siblings` (`:35-51`: below the threshold a sequential `iter`, otherwise `par_iter` on the global pool), and 6 unit tests (`grep -c '#\[test\]' crates/css/elidex-style/src/parallel.rs` → 6) |
| `crates/css/elidex-style/src/walk.rs:65-84` | `build_anim_style_from_owned`, used only by the parallel path |
| `crates/css/elidex-style/src/walk.rs:119-122` | `walk_children`'s doc paragraph describing the parallel path |
| `crates/css/elidex-style/src/walk.rs:130-133` / `:135-146` | the cfg pair: `walk_children_parallel`, or the sequential loop over `walk_tree` |
| `crates/css/elidex-style/src/walk.rs:149-281` | `walk_children_parallel`. Phase 1 cascades every child sequentially (it needs `&EcsDom`). Phase 2 runs `par_resolve_siblings`. Phase 3 attaches the results, generates pseudo-elements and recurses sequentially. A sibling set containing a shadow host falls back to `walk_tree` (`:176-188`). |

Comments that describe `parallel` without being gated by it: `crates/css/elidex-style/src/generated_content.rs:133-137`
(the "load-bearing" note, §1.4 F-1), `crates/css/elidex-style/src/parallel.rs:1-5` (module doc), and
`docs/architecture/css.md:29-35` (`## elidex-style (parallel)`).

### §1.3 Nature of the red

The red is **I4's uncovered feature only**. There is no gate leak: both compile columns are green in both
configurations on macOS. Logs are in this session's scratchpad, and the commands are re-runnable:

| Column | `parallel` on | default |
|---|---|---|
| clippy `-D warnings` | `cargo clippy -p elidex-style --no-default-features --features parallel -- -D warnings` → rc 0 | `cargo clippy -p elidex-style -- -D warnings` → rc 0 |
| rustdoc `--document-private-items`, `RUSTDOCFLAGS="-D warnings"` | `cargo doc --no-deps --document-private-items -p elidex-style --no-default-features --features parallel` → rc 0 | `cargo doc --no-deps --document-private-items -p elidex-style` → rc 0 |
| tests (information only; PR-T's column) | `cargo nextest run -p elidex-style --features parallel` → 347 passed | `cargo nextest run -p elidex-style` → 341 passed (the difference is the 6 tests of `parallel.rs`) |

Linux and Windows are unmeasured. `crates/css/elidex-style/Cargo.toml` has no target-specific table
(`grep -c '^\[target' crates/css/elidex-style/Cargo.toml` → 0), so the resolve does not differ by leg. A
`cfg(target_os)` site in the compiled code could still differ, so §6.2 still requires three-OS evidence.

### §1.4 Findings beyond the brief

- **F-1 — the two walks attach different components (measured).** The sequential resolver bakes the implicit
  list-item counters into the attached `ComputedStyle` (`walk.rs:361-362`, inside `resolve_and_attach_style`). The
  parallel path's Phase 3 attaches `par_resolve_siblings`' output without that step (`walk.rs:230-265`).
  - **Probe** (append, run, restore; no probe stays in the tree). Append to
    `crates/css/elidex-style/src/tests/mod.rs` a test that builds `body > ul > li` with `build_simple_dom`, calls
    `resolve_styles` with an empty author sheet, and prints `get_style(&dom, li).counter_increment`. Run
    `cargo test -q -p elidex-style --lib pr_a2_probe -- --nocapture` → `[("list-item", 1)]`; the same with
    `--features parallel` → `[]`. Then `git checkout -- crates/css/elidex-style/src/tests/mod.rs` and
    `git status --porcelain` → empty.
  - **Inconsistent within one configuration.** `resolve_and_attach_style` is also the resolver of the root
    (`walk_tree`, `:431`, `:442`), of shadow trees (`:570`, `:599`, `:622`) and of the shadow-host fallback. So with
    `parallel` on, a `<li>` gets the baked counter inside a shadow tree and does not get it outside one.
  - **Consumers compensate.** Two consumers re-apply the counters at use time, and the bake's idempotence guard
    hides the difference: `generated_content.rs:141-143` (whose comment, `:133-137`, calls the re-application
    "load-bearing" because of the parallel path) and `crates/core/elidex-render/src/builder/walk.rs:133-134`.
  - **The spec decides which component is right.** CSS Lists 3 §4.6 (`webref body css-lists-3 list-item-counter`)
    says the implicit increment happens "at the same time that counters are normally incremented" and "does not
    affect the specified or computed values of counter-increment". The parallel path's component (`[]`) matches the
    spec. The shipped sequential path's component does not.
  - **Unverified**: whether a script-visible reader serializes this field (Q1). The defect is a behaviour site, not
    a gate site, so it is returned to PM (§7, S2).
- **F-2 — the shipped walk is the untested walk.** CI's only test configuration is `--all-features` (`ci.yml:90`,
  `:93`), and that configuration runs the parallel walk in every crate whose tests style a document. The sequential
  walk, which every product ships (§1.1), runs in no CI test step at `b8b4d6f5`. Only local
  `cargo test -p <crate>` runs without features execute it. U0's PR-T closes this for the `default` rows (their
  test columns). Until then, F-1 shows the cost: the two walks differ and no CI step compares them.
- **F-3 — one pool today.** `par_iter` outside an `install` runs on rayon's global pool. wasmtime's
  `parallel-compilation` uses the same global pool (`Cargo.toml:266-267`), and no source file builds a `ThreadPool`
  (§0.1, last row). So whichever branch is taken, the workspace has one rayon pool.
- **F-4 — the design describes a pool the code does not have.** `docs/design/ja/06-thread-model.md` §6.3.1 (`:281-306`)
  describes one renderer pool built by `create_renderer_thread_pool` with cores−2 workers. §6.3.3 (`:318-331`) has
  the main thread join it through `pool.install` or rayon's `scope`. Neither exists in code. The style phase calls
  `par_iter` from the main thread, which blocks while the global pool's workers run. The global pool's default width
  reserves no core for the compositor. No branch of this PR decides the pool's home (Q2).

## §2. Coupled invariants

**Edge-dense verdict: yes.** The branch decision intersects three U0 invariants (I2, I4, I8) with two invariants of
its own subject (walk equivalence, the shared pool), which meets CLAUDE.md's "≥3 intersecting invariant axes". It
is also the base case of that rule: a narrowly scoped per-PR slice under the approved umbrella, so a terminal unit
(`.claude/skills/elidex-plan-review/SKILL.md` "When to invoke"). The enumeration below is therefore written out
rather than argued.

- **A1 — I4 completeness (U0 §2 I4).** Every entry of `elidex-style`'s `[features]` (`parallel`, implicit `rayon`)
  is active in some declared row's resolve, read by the reader contract (U0 I3×I4), or the entry no longer exists.
- **A2 — a row is a supported configuration (U0 §2 I2).** A declared profile asserts cfg hygiene of a configuration
  someone builds. It is not a CI-only switch.
- **A3 — land green (U0 §2 I8).** The applying PR lands with every row it adds or changes green on three OS. A
  connect row exists only once PR-M1's table exists.
- **A4 — one walk, spec outcome.** Every configuration attaches the same components for the same document (the
  `ComputedStyle` and `AnimStyle` components, and the pseudo-element entities), with values as the CSS specs define
  them (CSS Cascade 5 §4; CSS Lists 3 §4.6 for counters).
- **A5 — one pool.** CPU-parallel style work runs on the one shared rayon pool, and nothing in `elidex-style`
  creates a pool (CLAUDE.md "Concurrency by ownership and phases"; `docs/design/ja/06-thread-model.md` §6.1).

| Pair | Intersection |
|---|---|
| A1×A2 | Connect satisfies A1 only by declaring a profile that A2 asks a consumer for. No member is that consumer (§1.1), so on connect, A1's green rests on a row that no product builds (§4.3). |
| A1×A3 | Connect's A1 evidence (the `parallel` row's resolve) exists only after PR-M1, so it lands after PR-M1. A shape branch empties `[features]`, so A1 holds vacuously on the applying PR's own head, before or after PR-M1. |
| A1×A4 | Covering `parallel` by a row keeps both walks, so A4's divergence (F-1) becomes a difference between two declared rows. Removing the feature leaves one walk, but A4 still needs S2's ruling, because the bake is spec-wrong in the surviving sequential path (F-1). |
| A1×A5 | Always on puts rayon into the resolve of the 16 rows that contain `elidex-style` (§6.1). It adds no pool, because the global pool is already the only one (F-3). |
| A2×A3 | A declared `parallel` row is green on three OS forever after, and PR-T later adds its test columns. That is maintenance cost for a configuration no one ships. |
| A2×A4 | If both walks stay declared, every behaviour fix to the walk is owed twice and compared by no test until PR-T. That is U0's "N ways" in the style subsystem. |
| A2×A5 | A `parallel` row does not exercise the design's pool shape (F-4) either, so declaring it does not verify A5 beyond "it compiles". |
| A3×A4 | A shape branch changes the product walk (always on) or the test walk (remove). Either way the applying PR must show A4 against the base: the attached components are unchanged on a fixture corpus, except for the difference PM rules for S2 (§5.2). |
| A3×A5 | The three-OS run is also the only check that the rayon edge resolves on every leg. The manifest makes this target-independent (§1.3), and the run confirms it. |
| A4×A5 | The parallel path's ownership shape keeps cascade and ECS writes on the main thread (Phase 1 and Phase 3) and sends only owned maps and `&ComputedStyle` to workers. That makes it phase-separated access, which the design asks for (`06-thread-model.md` §6.2.1, `:69`). A4 is about outcome, not ownership, so moving to the parallel path must change no outcome beyond S2. |

## §3. Spec coverage map

No web-platform algorithm is implemented or changed by this PR. The rows below are the governing sources the
derivation (§4) and F-1 rely on. Honesty notes: (a) the CSS module labels and "Cargo Book" are not in `preflight.py`'s
`SPEC_LABEL_REVERSE`, so these rows are unmapped-label soft warnings (the CSS case is the registered slot
`#11-preflight-css-module-labels`). The CSS rows were checked by hand with `.claude/tools/webref`:
`heading css-cascade-5 4` → §4 Value Processing `#value-stages`, `heading css-cascade-5 4.4` → §4.4 Computed Values
`#computed`, `heading css-lists-3 4.6` → §4.6 The Implicit list-item Counter `#list-item-counter`. The Cargo Book
rows reuse U0 §3's hand-checked rows. (b) The repo's own rules govern the branch choice. They carry no § number, so
they are listed under the table.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-cascade-5 §4 Value Processing | #value-stages | the result of the cascade → specified → computed pipeline is defined per element; the execution strategy is not | A4; both walks (`walk.rs:123-281`, `:313-399`) | ✗ | yes (author CSS reaches the walk) |
| css-cascade-5 §4.4 Computed Values | #computed | Note: computed values avoid "hard-to-parallelize operations, such as … retrieving values other than from the element and its parent" — permits sibling parallelism, does not require it | §4.1 | ✗ | yes |
| css-lists-3 §4.6 The Implicit list-item Counter | #list-item-counter | the implicit increment "does not affect the specified or computed values of counter-increment" | F-1, S2 (`walk.rs:361-362` vs `:230-265`) | ✓ | yes |
| Cargo Book §3.4 Features | #feature-unification | `--all-features` and workspace builds turn `parallel` on for every dependent in the invocation | F-2 | ✓ | no |
| Cargo Book §3.4 Features | #optional-dependencies | `rayon` optional → implicit feature `rayon` | A1 population | ✓ | no |
| Cargo Book §3.1 The Manifest Format | #the-metadata-table | connect's declaration home (U0 I1) | §5.1 connect | ✓ | no |

**Breadth**: K=3 (css-cascade-5, css-lists-3, Cargo Book), M=6. Single PR.

**Repo rules that govern (no § number)**: CLAUDE.md 至上命題 (authority order), Design philosophy (Concurrency by
ownership and phases, Plugin-first, Ideal over pragmatic, ECS-native first), Design discipline (設計優先 "dead code
は接続するか削除", One issue one way, 後方互換性は維持しない, Supported-surface testing). Design docs: the shared
pool and the parallel style phase (`docs/design/ja/06-thread-model.md` §6.1 `:14`, §6.2.1 `:64`, `:69`, §6.3
`:279-331`; `docs/design/ja/15-rendering-pipeline.md` §15.2 `:20-34`; `docs/design/ja/01-executive-summary.md:28`;
`docs/design/ja/02-architecture-overview.md:11`). U0 §2 I2/I4/I8, §4.4 steps 6–7, §6 PR-A2 / PR-A2′ / PR-M2b, §10.
PM 543, PM 558 (1), PM 569, PM 605.

### §3.1 User-input touch audit

This PR adds no site that web content reaches. On a shape branch, the applying PR changes which walk the shipped
build runs (always on) or deletes one (remove). Author CSS flows through either walk. The walk's existing depth cap
applies on both paths (`MAX_WALK_DEPTH`, `walk.rs:162`, `:413`, `:544`), so neither branch removes a bound. The
parallel path adds no new input-controlled size: `par_iter`'s fan-out is bounded by the child count, the same count
the sequential loop walks.

## §4. Branch derivation (authority order)

### §4.1 Spec

No web spec decides whether style resolution is parallel. CSS Cascade 5 §4 defines the value each element receives,
not the order or concurrency in which an engine computes it. The §4.4 note says computed values are defined so that
they need "the element and its parent" only, which makes sibling-parallel computation possible but does not require
it. So the spec is silent on the branch, and the decision falls to elidex's principles.

The spec does constrain any branch in one respect: whichever walk survives must attach what the spec defines.
CSS Lists 3 §4.6 makes F-1's sequential bake spec-wrong (S2). That is a condition on the result, not a reason to
prefer a branch.

### §4.2 elidex principles, branch by branch

| Principle | connect | remove | always on |
|---|---|---|---|
| Concurrency by ownership and phases: "CPU 並列 work は共有 rayon pool", no duplicate pools | neutral: the product stays sequential | neutral: no CPU-parallel style work remains | satisfied: style work runs on the one global pool that wasmtime already uses (F-3). The pool's home and width (F-4) stay open under every branch |
| Design docs: style resolution is a parallel phase on the shared pool (`06-thread-model.md` §6.2.1 `:64`, §6.3.2; `15-rendering-pipeline.md` §15.2) | the product contradicts the design, and a CI row keeps the only realization alive | the product contradicts the design and no realization remains | the product follows the design's direction, with a narrower unit (sibling value computation, not subtrees) and without the design's renderer pool (F-4) |
| One issue, one way | **violated**: two walks, both declared supported, already divergent (F-1), with consumer-side compensation (`generated_content.rs:133-137`) | satisfied | satisfied (the shadow-host and below-threshold fallbacks are steps inside one algorithm, not configurations) |
| 設計優先: dead code は接続するか削除 | **not satisfied**: a CI row is not a consumer, so the path stays dead in every shipped build | satisfied (delete) | satisfied (connect to the product) |
| 後方互換性は維持しない / no shims | keeps a cfg fork that exists for no consumer | satisfied | satisfied |
| Plugin-first | a Cargo feature is not elidex's extension mechanism. If execution strategy ever needs to be configurable, the plugin and runtime configuration path is the place, not a compile-time fork | — | — |
| Ideal over pragmatic | the most pragmatic option: it changes nothing and adds a row | the smaller change, but it moves the code away from the design | the design's direction, at the cost of a behaviour change in the shipped walk (§5.2) |

### §4.3 Connect: who would run it

U0 I2 says a row asserts cfg hygiene of a configuration the member supports. Connect needs a consumer that builds
`elidex-style` with `parallel`, and none exists. No member enables it (§1.1). The product (`elidex-shell`) does not.
An embedder could only reach it by depending on `elidex-style` directly with the feature, and no such embedder is
named anywhere. A `parallel` row would therefore be a CI row, not a supported configuration. It would also make
F-1's divergence permanent between two declared rows. Connect is the only branch CI/tools could take alone (PM 543),
and this derivation does not reach it. So nothing here is decided by CI/tools.

### §4.4 Remove versus always on

**For always on.**
- The design names parallel style resolution on the shared pool as the architecture (§3's design-doc list). This is
  the only realization in the code, and it already keeps ECS ownership on the main thread (A4×A5).
- It adds no pool and no crate to the shipped product's resolve (F-3, §0.2).
- In CI it is the most-tested walk: it runs under every `--all-features` test step (F-2), plus its 6 unit tests.
- After the change the product ships the walk CI tests, and F-2's inversion ends without waiting for PR-T.

**For remove.**
- The parallel path is a partial realization. Its unit is the sibling set's value computation, not the design's
  per-subtree resolution. It uses the global pool instead of the design's renderer pool, and the main thread blocks
  rather than joining (F-4). Making it canonical adopts a shape the design does not describe. That could be read as
  a "minimal v1", which Ideal over pragmatic disfavours.
- It also adds a per-element cost the sequential path does not have: Phase 1 clones every element's winners into an
  `OwnedPropertyMap` with owned `String` keys (`parallel.rs:18-23`), even below the threshold. The cost is
  unmeasured. `cargo bench -p elidex-style` (`[[bench]] style_resolve`) is the instrument, and its result is judgment
  input, not authority.
- Remove keeps the shipped walk unchanged, so S2 is the only behaviour question left.

**Recommendation: always on**, with S2 ruled first in the spec's direction (no bake into the computed value), so
that the switch changes no attached value except the spec fix. Reasons:
- Of the two principle-satisfying branches, always on is the only one consistent with the design documents' stated
  architecture.
- Remove would leave the shipped engine with no parallel phase at all. A later parallel-style program would then add
  back what is deleted now.
- The partial-realization objection is real but does not favour remove. Neither branch builds the design's pool or
  per-subtree unit (F-4, Q2). Always on keeps the phase structure (sequential cascade → parallel compute →
  sequential apply) that such work extends.

If PM weighs the unmeasured per-element cost or the partial shape higher, **remove** is the consistent alternative.
Connect is not recommended.

**Returned to PM (stop rule, PM 569).** This memo makes no shape decision. The question for PM is: **always on
(recommended), remove, or connect?** Two further items go with it:
- S2 (§7), whose ruling always on depends on.
- Q2, the pool's home. It is out of PR-A2's scope and is routed by PM.

## §5. What the PR carries, per branch

Scope is WHAT and contracts. Code shape is the implementation's.

### §5.1 Always (whatever PM rules)

- This memo, landing as the PR-A2 plan record.
- In the PR body: the I4 computation by hand (§6.1) and the site list (§7), recorded via PM (condition (3)).
- No code under `crates/` unless PM names PR-A2 as the applying PR. If PM names PR-A2′ instead, PR-A2 reduces to
  this record and PR-A2′ carries U0 §6's acceptance.

### §5.2 Contracts for the applying PR (PR-A2 if PM names it, otherwise PR-A2′)

- **connect** (only if PM rules it): declare a profile `parallel` (features `["parallel"]`) for `elidex-style` in
  PR-M1's `[package.metadata.elidex.feature-profiles]` table (U0 I1; exact schema is PR-M1's). No code change. It
  lands after PR-M1.
- **always on**:
  - Make `rayon` a non-optional dependency. Delete `[features]` (both entries) and every `cfg(feature = "parallel")`
    / `cfg(not(feature = "parallel"))` site (§1.2). Delete the sequential loop (`walk.rs:135-146`).
  - Update the comments and docs that describe a feature: `generated_content.rs:133-137`, `parallel.rs:1-5`,
    `walk.rs:119-122`, `docs/architecture/css.md:29-35`.
  - Whether `parallel.rs` stays a module or folds into `walk.rs` is the implementation's choice.
    `walk.rs` is 827 lines (`wc -l crates/css/elidex-style/src/walk.rs`), so a fold must stay under 1000 lines
    (CLAUDE.md touch-time split).
  - **Outcome contract (A4)**: on a fixture corpus that includes lists (`ol`, `ul`, `li`, `reversed`, `start`,
    `value`), shadow hosts and sibling sets on both sides of the threshold, every attached `ComputedStyle` /
    `AnimStyle` and every pseudo-element entity is equal to the base sequential walk's result. The one exception is
    the difference PM's S2 ruling prescribes. The test is new, `walk_equivalence` (NEW); its form is the
    implementation's.
  - Record `cargo bench -p elidex-style` before and after, as information.
- **remove**:
  - Delete the optional dependency, `[features]`, `mod parallel`, `parallel.rs`, `build_anim_style_from_owned` and
    `walk_children_parallel`, and collapse `walk_children` to its sequential body.
  - Update the same comments and docs as always on.
  - The `--all-features` test steps then run the sequential walk. Their green on the PR's own three-OS `check` run is
    the evidence that the suite does not depend on the parallel walk.

Under every branch, S2's fix is applied by the PR PM names for it, not folded into a shape PR without that ruling.

## §6. Acceptance

### §6.1 I4 for `elidex-style`, by hand, by the reader contract

Procedure (U0 I3×I4: each row's own resolve, normal and build edges, per-package resolved feature sets). Run it on
the PR's own head, for every declared row (54 `default` rows, `elidex-js` `app` and `browser`, plus any row the PR
adds):

```sh
unset RUSTUP_TOOLCHAIN
cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; m=json.load(sys.stdin); ws=set(m["workspace_members"]); [print(p["name"]) for p in m["packages"] if p["id"] in ws]' | sort > members.txt
while read mem; do echo "$mem default $(cargo tree -p "$mem" -e normal,build -f '{p} {f}' --prefix none | grep '^elidex-style v' | sed 's/ (\*)$//' | sort -u | sed 's/.*) *//')"; done < members.txt
for f in engine engine,compat-webapi; do echo "elidex-js $f $(cargo tree -p elidex-js --no-default-features --features $f -e normal,build -f '{p} {f}' --prefix none | grep '^elidex-style v' | sed 's/ (\*)$//' | sort -u | sed 's/.*) *//')"; done
cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print([p["features"] for p in json.load(sys.stdin)["packages"] if p["name"]=="elidex-style"])'
```

**At the base `b8b4d6f5` (macOS)**:
- The member list is 54 lines, and the metadata command prints `[{'parallel': ['rayon'], 'rayon': ['dep:rayon']}]`.
- `elidex-style` appears in 16 of the 56 rows: elidex-a11y, elidex-dom-api, elidex-form, elidex-layout,
  elidex-layout-block, elidex-layout-flex, elidex-layout-grid, elidex-layout-multicol, elidex-layout-table,
  elidex-render, elidex-shell, elidex-style, elidex-wasm-runtime and elidex-wpt `default`, and elidex-js `app` and
  `browser`. In all 16 its resolved feature set is empty. All 56 invocations exit 0.
- So `parallel` and `rayon` are active in 0 rows, and **I4 is red for both**. This confirms PM 605.

**Expected on the applying PR's head**:
- **connect**: the `parallel` row lists `elidex-style` with `parallel,rayon`.
- **always on / remove**: the metadata command prints `[{}]`, so the population is empty and I4 holds for the member.
  For always on, the 16 rows also list `rayon` / `rayon-core` in their resolves (not part of I4; recorded as
  information).

The output goes in the PR body, as U0 §6 requires.

### §6.2 Three-OS evidence

- **connect**: lands after PR-M1. Evidence is the `feature-profiles` job's own run on the PR head: the `parallel` row
  green in both compile columns, on three OS (U0 I8).
- **shape branch landing before PR-M1**: the job does not exist yet, and the rustdoc column with
  `--document-private-items` exists only as PR-M1's column. Evidence follows the temporary-step rule (U0 §6,
  PM 558 (1)):
  - A scratch commit adds, as a temporary step of the `check` job, the two compile-column commands for
    `elidex-style`'s `default` row:
    - `cargo clippy -p elidex-style -- -D warnings`
    - `RUSTDOCFLAGS="-D warnings" cargo doc -p elidex-style --no-deps --document-private-items`
  - For always on, the same commands for the 15 other rows that contain `elidex-style` (§6.1), because their resolve
    gains rayon.
  - A later commit removes the step. PM checks conditions (i)–(iii) at merge (no `.github/` change in the merge diff,
    the two heads differ only under `.github/`, run id and sha in the body).
  - The `check` job's normal `--all-features` steps provide the test evidence. The shape PR edits no ci.yml line that
    stays in the merge, so decision 150 (3) does not order it after Python-pin PR-2.
- **shape branch landing after PR-M1**: the job's run on the PR head (every row, `elidex-style`'s `default` and doc
  row included).

### §6.3 Acceptance summary (U0 §6)

- On the PR head (PR-A2's, or PR-A2′'s if PM names it), §6.1's output shows `parallel` and `rayon` each active in a
  declared row (connect), or `elidex-style`'s `[features]` empty (shape). The output is recorded in the PR body.
- Connect: the `parallel` row is green in both compile columns on three OS.
- Always on: the `walk_equivalence` (NEW) contract of §5.2 holds.
- A returned site does not land open. The branch, and S2, land only after PM's ruling, applied by the PR PM names.

## §7. Condition (3): the `elidex-style` site list (for PM to record)

| Id | Sites (`b8b4d6f5`) | Kind | Disposition |
|---|---|---|---|
| S1 | `crates/css/elidex-style/Cargo.toml:28`, `:30-31`; `src/lib.rs:19-20`; `src/parallel.rs:1-181`; `src/walk.rs:65-84`, `:119-122`, `:130-133`, `:135-146`, `:149-281`; comment `src/generated_content.rs:133-137`; doc `docs/architecture/css.md:29-35` | shape of `parallel` | **returned to PM**. Recommendation: always on (§4.4). Applying PR named by PM (§5.2) |
| S2 | the bake `src/walk.rs:361-362` vs the parallel attach `src/walk.rs:230-265`; compensations `src/generated_content.rs:141-143`, `crates/core/elidex-render/src/builder/walk.rs:133-134` | behaviour (CSS Lists 3 §4.6: the implicit increment is not part of the computed value) | **returned to PM** (not a gate fix; L1's file). Owner and PR named by PM. Always on depends on it (§5.2) |
| S3 | `.github/workflows/ci.yml:90`, `:93` (`--all-features` is the only test configuration) | record, not a fix | the sequential walk is untested in CI (F-2). U0's PR-T closes it for the `default` rows. Always on removes it |
| S4 | `docs/design/ja/06-thread-model.md` §6.3.1 `:281-306`, §6.3.3 `:318-331` vs the global pool | design gap, not this PR's | Q2 for PM to route. No slot is opened here |

## §8. Ordering, deadline, stop rule

- **Order**: U0 §4.4 step 6, after C0 (done; PM 605 is its ruling).
  - Connect lands after PR-M1, because it writes PR-M1's table.
  - A shape branch has no ordering constraint against PR-M1. If it lands before PR-M1, §10's gap never opens (§9).
  - Every branch lands before PR-M2b (step 7), and so before PR-T (step 9).
- **Due 2026-11-20** (PM 605). The applying PR cannot land open, so meeting the date needs PM's ruling on S1 (and on S2, if always on) before it.
- **Stop rule** (PM 569, U0 §6): at PR-M2b's implementation readiness, the driver raises to PM any of these that is
  not done: PR-A2 not landed, S1's shape without an applied ruling, or S2 without an applied ruling. PM's ruling
  names the applying PR and a date, and that date bounds PR-M2b's wait and, through it, PR-T's. PR-M2b never lands
  with I4 red (U0 I8).
- **Backstop**: `#11-test-only-feature-boundary-guard`'s Re-eval 2026-12-15. Its remaining remedy (iv) is this
  feature (U0 §8), and PR-M2b closes it.

## §9. U0 §10's `parallel` rustdoc gap

At `b8b4d6f5` the `doc` job (`--all-features`, `ci.yml:134`) reads `parallel`'s rustdoc. PR-M1 retires that job, and
from then until the applying PR lands, no row reads it. The `check` job's `--all-features` clippy still compiles it.
Which branch closes the gap, and how:

| Branch | Closer | How |
|---|---|---|
| connect | PR-A2 (after PR-M1) | the `parallel` row's rustdoc column, `--document-private-items -D warnings` |
| always on | the applying PR | the cfg is gone, so `elidex-style`'s `default` row (and its doc row) reads the code |
| remove | the applying PR | the code is gone, so nothing is left to read |

On a shape branch that lands before PR-M1, the `doc` job reads the code up to the applying PR and the rows read the
result from PR-M1 on, so the gap has zero length. This is a reason to apply a shape ruling before PR-M1, not a
condition of the ruling.

## §10. Questions and out of scope

- **Q1** (scout): does any script-visible reader (a CSSOM `getComputedStyle` serializer, a `get_computed` property
  handler) expose `counter-increment` / `counter-reset` from the attached `ComputedStyle`? This decides whether S2 is
  observable to script today or only internal. Its spec direction does not depend on the answer.
- **Q2** (PM routing): the pool's home and width (F-4, S4). Options include a renderer pool installed for the style
  and layout phases, or the global pool configured once at startup, wasmtime's compile included so A5 holds. This
  belongs to a design owner, not to PR-A2.
- **Q3** (applying PR, always on): is the parallel walk's cascade order equivalent to the sequential walk's? Phase 1
  cascades all siblings before any sibling's subtree is resolved, while the sequential walk interleaves. A cascade
  that reads only DOM structure, attributes and per-element state should not care. The `walk_equivalence` (NEW)
  corpus is where that is shown, not assumed.
- **Out of scope**: the design's per-subtree parallelism and renderer pool (F-4); parallel layout; any change to
  `elidex-style` beyond §5.2; the matrix mechanisms themselves (PR-M1, PR-M2a, PR-M2b).
