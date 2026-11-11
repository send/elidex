# PR-A2 — `elidex-style/parallel`: measurement, branch derivation (ruled: remove, PM 624), I4 by hand (per-PR plan memo, U0 §6)

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
- **PM 611** (after plan-review R1): (1) the list-counter bake is split out of PR-A2 into
  `#11-list-counter-baking-ua-rule-and-hints` (§7.1); (2) U0 is read narrowly: PR-A2 and PR-A2′ apply only the
  **shape** of `parallel`, and any other returned **site** goes back to PM; (3) the shape is ruled after R2, on two
  axes: one walk (One issue, one way) and the design's phase parallelism (`06-thread-model.md` §6.3). Counter
  differences between the walks are excluded from the shape ruling, because the slot's fix removes them.
- **PM 614**: under always on, PR-A2′ has its own plan and plan-review; remove stays under this memo.
- **PM 620** (after plan-review R2): the slot text states the defect only (§7.1), and PM 611's "hint source is core"
  is withdrawn; the shape is re-derived on axis (i) and cost (§4.5).
- **PM 624** (after R2): the shape is **remove** (§4.6). PR-A2 applies it (§5.2).

**Outcome of this plan, stated first.** The derivation (§4) lands on a **shape change**, so under the stop rule the
branch was **returned to PM**, and PM ruled **remove** (PM 624, §4.6). **Connect** fails axis (i). **Remove** and
**always on** both satisfy axis (i), remove by deletion alone and always on only with a resolver unification.
Neither branch reaches the design's phase parallelism; whether always on's sibling phase is a step toward it is not
derived (§4.3). On cost, always on adds the owned-map copy and puts rayon into 12 of the 16 rows that contain
`elidex-style`; remove adds neither. The one-machine measurement of the copy cost resolves no difference (§4.5). The
rest of the PR proceeds as U0 §6 requires: the measurement (§1), the I4 computation by hand (§6), and the
`elidex-style` site list for PM (§7).

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
  `parallel` always on adds no crate to `elidex-shell`'s resolve and no second pool (§4.3).
- **Omission: `parallel` is not a pure execution strategy.** Within one configuration the two code paths are two
  per-element writers of `ComputedStyle` (§1.4 F-5), and they attach different list-counter values (§1.4 F-1).

## §1. Condition (1): consumers, gated sites, nature of the red

### §1.1 Who enables `parallel`

No workspace member enables `parallel` (§0.1 row 2). No profile could enable it either, because U0 PR-M1 declares
none for `elidex-style` (U0 §4.1). The resolve of a product build therefore never contains it. This includes every
one of the 16 rows whose resolve contains `elidex-style`: `elidex-style` appears in each with an empty feature set
(§6.1). The only builds that compile `parallel` are those that pass `--all-features`. The population is bounded by
`git grep -n -e '--all-features' -- .github scripts mise.toml`:
- CI: the `check` job's clippy, nextest and doc-test steps (`.github/workflows/ci.yml:87`, `:90`, `:93`) and the
  `doc` job (`ci.yml:134`);
- mise: `mise.toml:12`, `:23`, `:27`, `:32-33` (`[tasks.test-all]`, which `mise run ci` runs, so the developers'
  pre-push gate), `:38`, `:81`;
- `scripts/doc-changed.sh:29`, `:42`, `:46`.

`--all-features` activates every feature of every package selected on the command line (Cargo Book §3.4
#command-line-feature-options), so `elidex-style` gets `parallel` whenever `--workspace` selects it. Dependents have
no `parallel` feature of their own: features are unified across the packages of one workspace build (Cargo Book
§3.3.3 #features), so the one `elidex-style` build carrying `parallel` is shared by every dependent in the
invocation.

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

- **F-1 — the two walks attach different list-counter values. Routed to `#11-list-counter-baking-ua-rule-and-hints`
  (PM 611), not PR-A2's.** The facts, in summary; the slot text (§7.1) carries the detail and the citations.
  - The sequential resolver runs the post-cascade bake `apply_implicit_list_counters_from_dom` (`walk.rs:362` →
    `counter.rs:376`, body `:296-365`). It writes three fields: `counter_reset` on `ol` / `ul`, `counter_set` on
    `li[value]`, and `counter_increment` on `li`. The parallel path's Phase 3 attaches without it (`walk.rs:230-265`),
    so those fields are `[]` there. R1 Axis 2's probe confirmed all three fields; every other field is equal.
  - Neither walk is spec-correct, and the spec direction is not the same for the three fields. CSS Lists 3 §4.6
    keeps the implicit increment out of the computed `counter-increment`. HTML §15.3.7 puts
    `ol, ul, menu { counter-reset: list-item }` in the UA sheet and makes `ol[start]` / `[reversed]` and `li[value]`
    presentational hints, so `counter-reset` and `counter-set` belong in the computed value through the cascade.
    elidex has neither the UA rule (`ua.rs:53-61`) nor the hints, so the bake is their only source. The defect is in
    the missing cascade inputs, not in either walk.
  - Consumers compensate, so neither the visual output nor script observes the difference. The pre-layout pass
    re-applies the bake (`generated_content.rs:139-145`, whose comment `:133-137` calls it "load-bearing"). Render
    re-applies it on the **paged path only** (`crates/core/elidex-render/src/builder/walk.rs:121-141`, under
    `if ctx.paged`). `getComputedStyle` returns `"initial"` for every `counter-*` property whatever the field holds
    (R1 Axis 4's Q1 trace), which is the slot's facet (§10 Q1).
  - **Consequence for PR-A2**: counter differences are excluded from the shape ruling (PM 611).
- **F-2 — the shipped walk is the untested walk.** CI's only test configuration is `--all-features` (`ci.yml:90`,
  `:93`), and that configuration runs the parallel walk in every crate whose tests style a document. The sequential
  walk, which every product ships (§1.1), runs in no CI test step at `b8b4d6f5`. Only local
  `cargo test -p <crate>` runs without features execute it. Remove (PM 624) closes this: the `--all-features` steps
  then run the sequential walk, which is the shipped walk (§5.2). Until PR-A2 lands, F-1 shows the cost: the two
  walks differ and no CI step compares them.
- **F-3 — one pool today.** `par_iter` outside an `install` runs on rayon's global pool. wasmtime's
  `parallel-compilation` uses the same global pool (`Cargo.toml:266-267`), and no source file builds a `ThreadPool`
  (§0.1, last row). So whichever branch is taken, the workspace has one rayon pool.
- **F-4 — the code does not realize the settled pool design.** OPEN-015 is settled (`docs/design/ja/OPEN-ISSUES.md`
  "OPEN-015: プロセス内スレッドモデル [P1] — 解決済", resolved by chapter 6). `docs/design/ja/06-thread-model.md`
  §6.3.1 (`:281-306`) fixes one renderer pool of cores−2 workers built by `create_renderer_thread_pool`; §6.3.2
  (`:312`) makes style resolution parallel per DOM subtree; §6.3.3 (`:318-331`) has the main thread join through
  `pool.install` or rayon's `scope`. None of this exists in code. The style phase calls `par_iter` from the main
  thread; the main thread blocks and does no work; the pool is rayon's global pool, sized to all cores, shared with
  wasmtime's compile, and reserving no core for the compositor. What is missing is an implementation home, not a
  design decision (§10 Q2).
- **F-5 — two per-element writers inside one configuration.** With `parallel` on, `ComputedStyle`, `AnimStyle` and
  the pseudo-element entities are written by two implementations of one per-element sequence:
  - `resolve_and_attach_style` (`walk.rs:313-400`: cascade → build → bake → attach → AnimStyle → pseudo). It serves
    shadow-tree elements (`:599`), nested hosts (`:570`), slotted nodes (`:622`), every sibling set that contains a
    shadow host (fallback `:176-188` → `walk_tree` `:406`), and element roots when `find_roots` falls back to a
    parentless scan (`:291-297`). With a document root, `walk_tree(root)` reaches the Document node, which is not an
    element, so `<html>` goes through the parallel path.
  - `walk_children_parallel`'s Phase 1 / Phase 3 (`walk.rs:196-265`), an inline copy of the same sequence with its
    own AnimStyle builder (`build_anim_style_from_owned`, `:68-84`, a copy of `build_anim_style_from_winners`,
    `:45-63`).
  F-1 is a drift between these two copies. A second, non-counter drift is the depth bound (R1 Axis 2, R6):
  sequential styles elements at depth ≤ 10,000 (`walk_tree`'s check `:413`), the parallel path at depth ≤ 10,001
  (its check `:162` runs on the parent's depth before Phases 1–3 style the children). R1 Axis 2's full dump
  (12 siblings above the threshold, links, pseudo-elements, an animation, em/rem, sibling combinators) found no
  other field or entity that differs.

## §2. Coupled invariants

**Edge-dense verdict: yes.** The branch decision intersects three U0 invariants (I2, I4, I8) with two invariants of
its own subject (one per-element resolver, the shared pool), which meets CLAUDE.md's "≥3 intersecting invariant
axes". PR-A2 is the base case of that rule: a narrowly scoped per-PR slice under the approved umbrella, so a terminal
unit (`.claude/skills/elidex-plan-review/SKILL.md` "When to invoke"). The work it routes elsewhere is not covered by
that exemption: the slot is edge-dense and gets its own plan-review (§7.1).

- **A1 — I4 completeness (U0 §2 I4).** Every entry of `elidex-style`'s `[features]` (`parallel`, implicit `rayon`)
  is active in some declared row's resolve, read by the reader contract (U0 I3×I4), or the entry no longer exists.
- **A2 — a row is a supported configuration (U0 §2 I2).** A declared profile asserts cfg hygiene of a configuration
  someone builds. It is not a CI-only switch.
- **A3 — land green (U0 §2 I8).** The applying PR lands with every row it adds or changes green on three OS. A
  connect row exists only once PR-M1's table exists.
- **A4 — one per-element resolver.** Inside every configuration, the style walk attaches each element's
  `ComputedStyle`, `AnimStyle` and pseudo-element entities through one per-element resolve step, whichever walker
  reaches the element (light tree, shadow tree, slotted node, shadow-host fallback, root fallback).
- **A5 — one pool.** CPU-parallel style work runs on the one shared rayon pool, and nothing in `elidex-style`
  creates a pool (CLAUDE.md "Concurrency by ownership and phases"; `docs/design/ja/06-thread-model.md` §6.1).

| Pair | Intersection |
|---|---|
| A1×A2 | Connect satisfies A1 only by declaring a profile that A2 asks a consumer for. No member is that consumer (§1.1), so on connect, A1's green rests on a row that no product builds (§4.4). |
| A1×A3 | Connect's A1 evidence (the `parallel` row's resolve) exists only after PR-M1, so it lands after PR-M1. A shape branch empties `[features]`, so A1 holds vacuously on the applying PR's own head, before or after PR-M1. |
| A1×A4 | Covering `parallel` by a row keeps both writers in the `parallel` row (F-5) and makes the two rows attach different components (F-1). Remove leaves one writer (`resolve_and_attach_style`). Always on satisfies A4 only with a single-resolver unification; deleting the cfg alone leaves F-5's two writers. |
| A1×A5 | Always on puts `elidex-style`'s rayon edge into the resolve of the 16 rows that contain `elidex-style` (§6.1). rayon is new in 12 of them; the 4 of §0.2 already have it. It adds no second pool (F-3). In the 12 rows, rayon's global pool is created lazily at `elidex-style`'s first `par_iter`. |
| A2×A3 | A declared `parallel` row is green on three OS forever after, and PR-T later adds its test columns. That is maintenance cost for a configuration no one ships. |
| A2×A4 | If both paths stay declared, every behaviour fix to the per-element sequence is owed twice and compared by no test until PR-T. That is U0's "N ways" in the style subsystem. |
| A2×A5 | A `parallel` row does not exercise the design's pool shape (F-4) either, so declaring it does not verify A5 beyond "it compiles". |
| A3×A4 | A shape branch changes the product walk (always on) or the test walk (remove). Remove's evidence is the `--all-features` steps' green (§5.2). |
| A3×A5 | The three-OS run is also the only check that the rayon edge resolves on every leg. The manifest makes this target-independent (§1.3), and the run confirms it. |
| A4×A5 | The parallel path keeps cascade and ECS writes on the main thread (Phase 1 and Phase 3) and sends only owned maps and `&ComputedStyle` to workers. That split comes from today's code, not from the design: Phase 1 needs `&EcsDom` (`walk.rs:197`), and the walk writes the DOM before the cascade (`set_link_state`, `remove_pseudo_entities`: `walk.rs:331`, `:336`, and in Phase 1 `:205`, `:207`). The design runs the whole `resolve_styles` of a subtree on workers (`06-thread-model.md` §6.3.2, §6.3.3; §4.3). |

## §3. Spec coverage map

No web-platform algorithm is implemented or changed by this PR. The rows below are the governing sources the
derivation (§4) relies on. Honesty notes: (a) the CSS module labels and "Cargo Book" are not in `preflight.py`'s
`SPEC_LABEL_REVERSE`, so these rows are unmapped-label soft warnings (the CSS case is the registered slot
`#11-preflight-css-module-labels`). The CSS rows were checked by hand with `.claude/tools/webref`:
`heading css-cascade-5 4` → §4 Value Processing `#value-stages`, `heading css-cascade-5 4.4` → §4.4 Computed Values
`#computed`. The Cargo Book rows were checked against the local book (`$(rustc --print sysroot)/share/doc/rust/html/cargo`,
1.99.1): §3.4 Features (`reference/features.html`, `id="command-line-feature-options"`, `#optional-dependencies`),
§3.3.3 Dependency Resolution (`reference/resolver.html`, `id="features"`), §3.1 The Manifest Format
(`#the-metadata-table`). (b) The list-counter citations (CSS Lists 3 §4.6, §4.4.2, HTML §15.3.7) are the slot's and
live in its text (§7.1), with their webref commands; PR-A2 cites none of them as coverage. (c) The repo's own rules
govern the branch choice. They carry no § number, so they are listed under the table.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-cascade-5 §4 Value Processing | #value-stages | the result of the cascade → specified → computed pipeline is defined per element; the execution strategy is not | A4; both paths (`walk.rs:123-281`, `:313-399`) | ✗ | yes (author CSS reaches the walk) |
| css-cascade-5 §4.4 Computed Values | #computed | non-normative Note: "In general, the computed value resolves the specified value as far as possible without" "retrieving values other than from the element and its parent" — a tendency, which permits sibling parallelism and does not require it | §4.1 | ✗ | yes |
| Cargo Book §3.4 Features | #command-line-feature-options | `--all-features` activates every feature of every selected package, `elidex-style` included under `--workspace` | §1.1, F-2 | ✓ | no |
| Cargo Book §3.3.3 Dependency Resolution | #features | features are unified across the packages of one workspace build, so dependents share the one `elidex-style` build with `parallel` | §1.1 | ✓ | no |
| Cargo Book §3.4 Features | #optional-dependencies | `rayon` optional → implicit feature `rayon` | A1 population | ✓ | no |
| Cargo Book §3.1 The Manifest Format | #the-metadata-table | connect's declaration home (U0 I1) | §4.4 | ✓ | no |

**Breadth**: K=2 (css-cascade-5, Cargo Book), M=6. Single PR.

**Repo rules that govern (no § number)**: CLAUDE.md 至上命題 (authority order), Design philosophy (Concurrency by
ownership and phases, Plugin-first, Ideal over pragmatic, ECS-native first), Design discipline (設計優先 "dead code
は接続するか削除", One issue one way, 後方互換性は維持しない, Supported-surface testing). Design docs: the shared
pool and the parallel style phase (`docs/design/ja/06-thread-model.md` §6.1 `:14`, §6.2.1 `:64`, §6.3
`:279-331`; `docs/design/ja/15-rendering-pipeline.md` §15.2 `:20-34`; `docs/design/ja/01-executive-summary.md:28`;
`docs/design/ja/02-architecture-overview.md:11`; `docs/design/ja/OPEN-ISSUES.md` OPEN-015). U0 §2 I2/I4/I8, §4.4
steps 6–7, §6 PR-A2 / PR-A2′ / PR-M2b and the pre-matrix evidence paragraph, §10. PM 543, PM 558 (1), PM 569,
PM 605, PM 611, PM 614, PM 620, PM 624.

### §3.1 User-input touch audit

This PR adds no site that web content reaches. It deletes the parallel walk (remove, PM 624); the shipped walk is
unchanged. Both paths bound recursion by `MAX_WALK_DEPTH` (`walk.rs:162`, `:413`, `:544`), but not at the same depth:
the parallel path styles one level deeper (F-5). Remove keeps the sequential walk's bounds.

## §4. Branch derivation (authority order)

### §4.1 Spec

No web spec decides whether style resolution is parallel. CSS Cascade 5 §4 defines the value each element receives,
not the order or concurrency in which an engine computes it. The §4.4 Note says that, in general, the computed value
is resolved as far as possible without retrieving values other than from the element and its parent. The Note is
non-normative and states a tendency; it makes sibling-parallel computation possible and does not require it. So the
spec is silent on the branch, and the decision falls to elidex's principles. (The list-counter defect, F-1, is a
condition on what any surviving resolver attaches, not a reason to prefer a branch, and PM 611 moved it to the slot.)

### §4.2 Why was `parallel` optional?

One issue, one way asks, before N forms collapse to one, what property the N-form had
(`memory/feedback_one-issue-one-way.md`). Search, with its result:
- `git log --format='%h %ad %s' --date=short -S'parallel = ["rayon"]' -- crates/css/elidex-style/Cargo.toml` →
  `785a197b` (#19, 2026-03-13), which only moved the crate from `crates/web/` to `crates/css/`. The feature was
  introduced earlier: `git log --all --oneline -S'feature = "parallel"'` → oldest `e312c36b` (2026-03-11, "M3.5-11:
  Integration — parallel style resolution + WPT harness + benchmarks", merged by PR #10, `6504035c`).
- `e312c36b`'s message says "Parallel style resolution (elidex-style, feature-gated "parallel")" and lists the
  three-phase shape, the threshold, the shadow-host fallback and the depth guard. It gives no reason for the gate.
- PR #10's body (`gh pr view 10 --json body`) lists "M3.5-11: Integration (rayon parallel style resolution, …)" and
  the check "`cargo test -p elidex-style --features parallel` — parallel path tests pass". No reason.
- `docs/architecture/css.md:31` ("**Feature flag**: `parallel` enables rayon-based sibling parallel style
  resolution") and `parallel.rs:1-5` describe the gate and do not justify it. `git grep -n -i 'parallel' b8b4d6f5 --
  docs/design` → 55 lines (the parallel phase, §3's design-doc list); piped to
  `grep -i -E 'feature|optional|オプション|フラグ|cfg|任意'` → 0, so no optional or feature-gated form of it.

**Not found.** No record states why `parallel` was optional. The candidate properties the N-form could have had are
unexamined: single-threaded determinism for tests (the parallel path's writes are sequential in Phase 3, which argues
against it, but this is not measured), a target or embedder without threads (the elidex-app embedding, design chapter
26; no such target is in the workspace's rows), and keeping rayon out of a lean resolve (§0.2 answers it for
`elidex-shell` only; rayon is new in 12 of the 16 rows, A1×A5). Under the one-issue-one-way lesson "not found" is a
stop, not a pass: PM rules the collapse knowing that the N-form's property was searched for and not found.

### §4.3 The two axes, branch by branch

PM 611 sets the axes: (i) one walk (CLAUDE.md One issue, one way) and (ii) the design's phase parallelism
(`06-thread-model.md` §6.3). Counter differences are excluded (F-1).

| Axis | connect | remove | always on |
|---|---|---|---|
| (i) one walk: one per-element writer of the style components in every configuration | **violated**: two paths, both declared supported, two writers inside the `parallel` row (F-5), already divergent (F-1, depth bound) | **satisfied**: `resolve_and_attach_style` is the only per-element resolve step | **satisfied only with a resolver unification**: one per-element resolver, with shadow trees, slotted nodes, the shadow-host fallback and the root fallback going through the same resolver. Deleting the cfg alone leaves F-5's two writers |
| (ii) phase parallelism: style resolution is a parallel phase on the shared pool (`06-thread-model.md` §6.2.1 `:64`, §6.3; `15-rendering-pipeline.md` §15.2) | the product stays sequential | the product stays sequential | the product computes values per sibling set on the global pool, with cascade, attach and recursion sequential on the main thread (A4×A5); the design's whole-subtree `resolve_styles` on workers (§6.3.2, §6.3.3) is not reached |

**Axis (ii)**: neither branch reaches §6.3.2 / §6.3.3; whether always on's sibling phase is a step toward them is not
derived. Today's Phase 1 is serial for code reasons (A4×A5), not because the design asks for it.

The other principles separate connect from the shape branches but not remove from always on:
- 設計優先 "dead code は接続するか削除": connect does not satisfy it (a CI row is not a consumer); remove deletes,
  always on connects.
- 後方互換性は維持しない / no shims: connect keeps a cfg fork that exists for no consumer; both shape branches drop it.
- Plugin-first: a Cargo feature is not elidex's extension mechanism. If execution strategy ever needs to be
  configurable, the plugin and runtime configuration path is the place, not a compile-time fork.
- Concurrency by ownership and phases ("CPU 並列 work は共有 rayon pool", no duplicate pools): neither shape branch
  adds a second pool (F-3).

### §4.4 Connect: who would run it

U0 I2 says a row asserts cfg hygiene of a configuration the member supports. Connect needs a consumer that builds
`elidex-style` with `parallel`, and none exists. No member enables it (§1.1). The product (`elidex-shell`) does not.
An embedder could only reach it by depending on `elidex-style` directly with the feature, and no such embedder is
named anywhere. A `parallel` row would therefore be a CI row, not a supported configuration, and it would make the
two writers permanent between two declared rows. Connect is the only branch CI/tools could take alone (PM 543), and
this derivation does not reach it. So nothing here is decided by CI/tools.

### §4.5 Remove versus always on: axis (i) and cost

Axis (ii) is not derived either way (§4.3); this section derives axis (i) and cost.

**Axis (i).** Both shape branches reach one per-element resolve step. Remove reaches it by deletion: the step that
remains, `resolve_and_attach_style`, already serves every walker, and the applying PR is deletion only, under this
memo (PM 614). Always on reaches it only through a resolver unification (PR-A2′), which has its own plan and
plan-review (PM 614); deleting the cfg alone leaves F-5's two writers.

**Cost.** Both costs fall on always on only; remove adds neither, and its shipped walk is unchanged.
- **rayon in 12 more rows.** rayon is new in 12 of the 16 rows that contain `elidex-style` (A1×A5).
- **The owned-map copy.** Phase 1 clones every element's winners into an `OwnedPropertyMap` with owned `String`
  keys (`parallel.rs:15-23`), below the threshold too. Instrument: `crates/css/elidex-style/benches/style_resolve.rs`
  (`Cargo.toml:37-39`, `[[bench]] style_resolve`). Cases: `resolve_deep_100` (`:84`, a chain of single children,
  every sibling set below the threshold, so copy cost with no parallel gain), and `resolve_100_flat` /
  `resolve_1000_flat` (`:56`, `:70`, one sibling set of 100 / 1000 under `body`, so copy cost against parallel gain).
  - What was run: in this worktree at `fa524624` (A1, B1) and `f12dc726` (B2, A2; docs-only, committed between B1
    and B2), neither with a `crates/` diff against `b8b4d6f5`, macOS, 10 cores
    (`sysctl -n hw.physicalcpu` → 10), rustc 1.99.1, `RUSTUP_TOOLCHAIN` unset, one cargo process at a time, in this
    order: `cargo bench -p elidex-style --bench style_resolve` (A1), the same with `--features parallel` (B1), with
    `--features parallel` again (B2), without it again (A2). The machine was shared with other sessions; `uptime`'s
    1-minute load average was recorded before and after each run.
  - Criterion point estimates, `deep_100` / `100_flat` / `1000_flat`:
    - A1 426.30 µs / 771.87 µs / 33.957 ms (load 6.64 → 15.00); B1 405.17 µs / 672.63 µs / 28.298 ms (15.00 → 7.67).
    - B2 405.49 µs / 649.46 µs / 29.334 ms (5.41 → 4.93); A2 376.04 µs / 643.51 µs / 28.657 ms (4.93 → 4.22).
  - Reading: the figures are point estimates. The spread between runs of the same configuration (A1 → A2: −11.8 %,
    −16.6 %, −15.6 %) is larger than the B2/A2 differences (+7.8 %, +0.9 %, +2.4 %). So no
    difference is resolvable, the measurement supports no direction, and it does not bound the cost.
    Logs: this session's scratchpad (`a2-bench/{default,parallel,parallel2,default2}.log`).

**What the derivation supports.** On axis (i), both shape branches satisfy it; remove does so by deletion, always on
through an edge-dense unification with its own plan and plan-review. On cost, everything falls on always on, and the
one machine's measurement resolves no difference in either direction. So on axis (i) and cost the derivation gives
always on no advantage over remove. It does not show the gain always on's parallel phase would need on other machines
or documents. The N-form's property is not found (§4.2) under both.

### §4.6 Ruled by PM: remove (PM 624)

The branch was returned to PM (stop rule, PM 569; ruled after R2, PM 611) with the question **always on, remove, or
connect?**, together with §4.5's derivation and measurement, §4.2's "not found" and Q2 (§10).

**PM 624: remove.** PR-A2 deletes the `elidex-style` `parallel` feature, `parallel.rs`, `walk_children_parallel` and
the optional `rayon` dependency (§5.2). Re-introduce `parallel` together with the implementation of the renderer pool
in design doc 06 §6.3, at subtree granularity.

## §5. What the PR carries

Scope is WHAT and contracts. Code shape is the implementation's.

### §5.1 Record

- This memo, landing as the PR-A2 plan record.
- In the PR body: the I4 computation by hand (§6.1) and the site list (§7), recorded via PM (condition (3)).
- PR-A2 applies only the shape of `parallel` (§5.2; PM 614, PM 624). Every other site this memo returns (§7) goes
  back to PM and is applied by whatever PM names for it (PM 611, narrow reading of U0).

### §5.2 Contract: remove (PM 624), applied by PR-A2

PR-A2 is deletion only and stays under this memo (PM 614).
- Delete the optional dependency (and the workspace entry `rayon = "1"` in the root `Cargo.toml`, which then has no
  user), `[features]`, `mod parallel`, `parallel.rs`, `build_anim_style_from_owned` and
  `walk_children_parallel`, and collapse `walk_children` to its sequential body. `resolve_and_attach_style` is then
  the only per-element resolve step of the style walk (A4), and the depth bound is the base product's.
- Update the comments and docs that describe the feature: `generated_content.rs:133-137`, `walk.rs:119-122`,
  `docs/architecture/css.md:29-35`.
- The `--all-features` test steps then run the sequential walk. Their green on the PR's own three-OS `check` run is
  the evidence that the suite does not depend on the parallel walk. The shipped walk is unchanged, so no outcome
  dump is needed.

PR-A2 does not change the list-counter bake or its compensations; that is the slot's (§7.1).

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

**Expected on PR-A2's head**: the metadata command prints `[{}]`, so the population is empty and I4 holds for the
member.

The output goes in the PR body, as U0 §6 requires.

### §6.2 Three-OS evidence

- **PR-A2 landing before PR-M1**: the job does not exist yet, and the rustdoc column with
  `--document-private-items` exists only as PR-M1's column.
  - **Question for PM (answered after R2)**: does U0's pre-matrix evidence rule (§6, PM 558 (1)) apply to PR-A2?
    Its population is "PR-A1, PR-A1′, PR-T0…, PR-T0′", which does not name it, so this memo does not assume it.
    The steps below are what PR-A2 does if PM extends the rule; if PM does not, PM names the evidence.
  - A scratch commit adds, as a temporary step of the `check` job, the two compile-column commands for
    `elidex-style`'s `default` row:
    - `cargo clippy -p elidex-style -- -D warnings`
    - `RUSTDOCFLAGS="-D warnings" cargo doc -p elidex-style --no-deps --document-private-items`
  - A later commit removes the step. PM checks conditions (i)–(iii) at merge (no `.github/` change in the merge diff,
    the two heads differ only under `.github/`, run id and sha in the body).
  - The `check` job's normal `--all-features` steps provide the test evidence. If PM extends the rule, PR-A2
    edits no ci.yml line that stays in the merge, and decision 150 (3) does not order it after Python-pin PR-2; that
    consequence is part of the same question.
- **PR-A2 landing after PR-M1**: the job's run on the PR head (every row, `elidex-style`'s `default` and doc
  row included).

### §6.3 Acceptance summary (U0 §6)

- On PR-A2's head, §6.1's output shows `elidex-style`'s `[features]` empty. The output is recorded in the PR body.
- The shape is ruled (PM 624) and applied by PR-A2. Other returned sites (§7) are not applied by PR-A2; they go back
  to PM (PM 611).

## §7. Condition (3): the `elidex-style` site list (for PM to record)

| Id | Sites (`b8b4d6f5`) | Kind | Disposition |
|---|---|---|---|
| S1 | `crates/css/elidex-style/Cargo.toml:28`, `:30-31`; root `Cargo.toml:289` (workspace `rayon`); `src/lib.rs:19-20`; `src/parallel.rs:1-181`; `src/walk.rs:65-84`, `:119-122`, `:130-133`, `:135-146`, `:149-281`; comment `src/generated_content.rs:133-137`; doc `docs/architecture/css.md:29-35` | shape of `parallel` | **ruled remove** (PM 624, §4.6). Applied by PR-A2 (§5.2) |
| S2 | the list-counter bake and its compensations (F-1) | behaviour, not this PR's | **routed to `#11-list-counter-baking-ua-rule-and-hints`** (PM 611). Slot text: §7.1. Not in PR-A2's acceptance or stop rule |
| S3 | `.github/workflows/ci.yml:90`, `:93` (`--all-features` is the only test configuration) | record, not a fix | the sequential walk is untested in CI (F-2). Remove (PM 624) closes it: the `--all-features` steps then run the sequential walk, which is the shipped walk (§5.2) |
| S4 | `docs/design/ja/06-thread-model.md` §6.3.1 `:281-306`, §6.3.2 `:312`, §6.3.3 `:318-331` vs the global pool | implementation home missing for a settled design (OPEN-015) | §10 Q2, for PM |

### §7.1 Slot text: `#11-list-counter-baking-ua-rule-and-hints` (PM 611, PM 620; pmo registers the text between the BEGIN and END markers, including the back-pointer item)

<!-- BEGIN slot-text #11-list-counter-baking-ua-rule-and-hints -->

**`#11-list-counter-baking-ua-rule-and-hints`** — owner **L1** (parked, no driver); at Re-eval, PM assigns an owner:
a free lane, or a driver for L1. **Re-eval 2026-12-15.** **Trigger**: L1 gets a driver, or a lane frees up.
**Edge-dense: plan-review is mandatory before work starts** (cascade origin × presentational hints × counter
processing × both style paths × render × engine modes). **Exit criterion**: a target of the paydown exit criterion
(c), "本番で黙って誤る slot を 0 に" (no silent-wrong-in-production slot) (PM 620).

**Defect.** List counters come from the post-cascade bake `apply_implicit_list_counters_from_dom`
(`crates/css/elidex-style/src/counter.rs:376`; body `apply_implicit_list_counters`, `:296-365`; called at
`crates/css/elidex-style/src/walk.rs:362`). HTML §15.3.7 defines them through the UA rule
`ol, ul, menu { counter-reset: list-item; }` and the list presentational hints (`ol[start]` / `[reversed]` →
'counter-reset', `li[value]` → 'counter-set'). The bake also writes CSS Lists 3 §4.6's implicit increment into the
computed `counter-increment`, which §4.6 says it "does not affect". Cases:
- `li` `counter_increment` gets `list-item 1` (§4.6: belongs in counter processing, not the computed value);
- `li[value]` `counter_set` is appended after the cascade, so an author `counter-set: foo 1` does not override it
  (code-read);
- `ol` / `ul` `counter_reset` is appended after the cascade, so an author `ol { counter-reset: foo }` keeps
  `list-item` (code-read);
- `reversed` without `start` bakes an integer `li_count + 1` into the computed `counter_reset`. The hint yields
  `reversed(list-item)` with no integer, and CSS Lists 3 §4.4.2 defines the initial value of a counter instantiated
  without one: the user agent calculates it at layout time;
- `menu` is missing (the bake matches `"ol"`, `"ul"`, `"li"` only, `counter.rs:303`, `:333`, `:341`);
- author-override precedence is wrong for the reset and the set (the bake's `already_has` guards are not cascade
  semantics);
- on a reversed counter, `CounterState::process_element` negates every increment, explicit ones included
  (`counter.rs:115`); §4.6 makes only the implicit increment -1 (code-read);
- `CounterState::process_element` runs reset → set → increment (`counter.rs:79` comment, `:84-125`); CSS Lists 3 §4
  orders reset → increment → set (steps 2–4), so author `x { counter-set: c 5; counter-increment: c }` gives 6 where
  the spec gives 5 (code-read);
- `counter-reset` without an integer cannot be represented: `CounterResetEntry.value` is an `i32` with no absent
  state (`crates/core/elidex-plugin/src/computed_style/mod.rs:121-128`), and `resolve_counter_reset` fills 0
  (`unwrap_or(0)`, `crates/css/elidex-style/src/resolve/box_model/mod.rs:512`), so author
  `counter-reset: reversed(foo)` also gets 0 (code-read);
- the bake runs inside shadow trees and for slotted nodes (`walk.rs:570`, `:599`, `:622`) and not in the parallel
  path's light tree (`walk.rs:230-265`), so at `b8b4d6f5`, with `parallel` on, one document gets two answers;
- `li` is counted by tag (the `"li"` arm, `counter.rs:341`), not by `display: list-item` (§4.6 "every list item"):
  `li{display:block}` still increments, and a non-`li` `display: list-item` element does not. The pre-layout
  counter pass is gated by tag too (`is_list_tag` = `ol` / `ul` / `li` by `TagType`,
  `crates/css/elidex-style/src/generated_content.rs:124-127`; gate `:139`), so it skips `process_element` for a
  non-`li` `display: list-item` element without counter properties (code-read). This is what master
  plan §10 (c) records (`memory/render-layout-inline-convergence-plan.md:246`; Copilot #273 R1), deferred there with
  no slot. This slot bundles it with the bake as **one decision surface**.

**Observable symptoms** (the numbering cases are code-read, not rendered): master §10 (c)'s mis-numbering; author
overrides of the reset and the set; `menu`; `display: list-item`; reversed numbering; and `getComputedStyle`
returning `"initial"` (facet below).

**Facts (R1 Axis 2, PR-A2 plan review).** UA sheet: `crates/css/elidex-style/src/ua.rs:53-61` has margins,
padding, `li { display: list-item; }` and `ol { list-style-type: decimal; }`, and no `counter-*`. No hint source
produces `counter-*` (`crates/dom/elidex-dom-compat/src/presentational.rs` has none). So the bake is the only source
of the reset and set values. Its outcome reaches the screen only through compensations:
`crates/css/elidex-style/src/generated_content.rs:139-145` (the pre-layout counter pass re-applies the bake; comment
`:133-137` calls it "load-bearing", recorded at #273's landing, `memory/m4-12-landings-ledger.md:201`) and
`crates/core/elidex-render/src/builder/walk.rs:121-141` (re-applies it on the paged path only, `if ctx.paged`).

**Facet: `getComputedStyle` for `counter-*`.** `getComputedStyle(x).getPropertyValue('counter-increment')` (and
`counter-reset`, `counter-set`) returns `"initial"` for every element, author values included: no
`CssPropertyHandler` registers a `counter-*` name, so `get_computed_with_registry` falls back to `get_initial_value`
(`crates/css/elidex-style/src/resolve/mod.rs:70-73`) → `CssValue::Initial`. That fallback is general: any property
without a handler gets `CssValue::Initial` there. The spec's resolved value is the computed value (CSSOM-1 §9,
`webref heading cssom-1 9`). Of the three, only `counter-increment` resolves to `none` on a list item with no author
value (§4.6 keeps the implicit increment out of it; initial value `none`, `webref css css-lists-3
counter-increment`); `counter-reset` on `ol` / `ul` / `menu` and `counter-set` on `li[value]` carry the §15.3.7 UA
rule and hints. Neighbouring slots: `#11-cssvalue-list-separator-fidelity`, `#11-computed-style-enumeration`.

**Hint placement.** Where hints sit (core/compat) is an undecided decision that spans E0
(`docs/plans/2026-06-e0-shell-style-compat-mode.md:95`; `StyleCompatPolicy`,
`crates/core/elidex-plugin/src/spec_level.rs:203-230`) and L2's IBP plan
(`docs/plans/2026-09-inline-box-predicate.md:280-282`); the slot's plan-review takes them up together; a conclusion
that overturns E0 returns to E0's ratifier. PM 611's "hint source is core" is withdrawn (PM 620).

**Why.** The fix changes cascade inputs in every engine mode, so it is not a gate fix and not CI/tools' (PM 543,
PM 611). It is edge-dense, and its owner (L1) is parked.

**First task of the slot's plan**: confirm the numbering cases above by rendering and by `getComputedStyle` (PM 620).
The remedy is the slot's own plan-review's.

**Citations** (webref): `heading html 15.3.7` → §15.3.7 Lists `#lists`; `heading css-lists-3 4.6` → §4.6 The
Implicit list-item Counter `#list-item-counter`; `heading css-lists-3 4.4.2` → §4.4.2 Instantiating Counters
`#instantiating-counters`; `body css-lists-3 auto-numbering` → §4 Automatic Numbering With Counters (steps 2–4:
reset, increment, set); `body css-lists-3 list-item-counter` ("This does not affect the specified or computed
values of counter-increment"); `body css-lists-3 instantiating-counters` ("When a counter is instantiated without an
initial value, the user agent must dynamically calculate the initial value at layout-time"); `body html lists` (the
UA rule and the two hint paragraphs).

**Owed by PM/pmo**: a back-pointer from master plan §10 (c) (`memory/render-layout-inline-convergence-plan.md:246`)
to `#11-list-counter-baking-ua-rule-and-hints`. That file is in memory, not the repo, so PR-A2 does not edit it.

<!-- END slot-text #11-list-counter-baking-ua-rule-and-hints -->

## §8. Ordering, deadline, stop rule

- **Order**: U0 §4.4 step 6, after C0 (done; PM 605 is its ruling).
  - PR-A2 has no ordering constraint against PR-M1. If it lands before PR-M1, §10's gap never opens (§9).
  - PR-A2 lands before PR-M2b (step 7), and so before PR-T (step 9).
  - The slot (§7.1) is not ordered against any of these.
- **Due 2026-11-20** (PM 605). S1 is ruled (PM 624).
- **Stop rule** (PM 569, U0 §6, read narrowly per PM 611): at PR-M2b's implementation readiness, the driver raises to
  PM either of these that is not done: PR-A2 not landed, or S1's shape without an applied ruling. PM's ruling names
  the applying PR and a date, and that date bounds PR-M2b's wait and, through it, PR-T's. PR-M2b never lands with I4
  red (U0 I8).
- **Backstop**: `#11-test-only-feature-boundary-guard`'s Re-eval 2026-12-15. Its remaining remedy (iv) is this
  feature (U0 §8), and PR-M2b closes it.

## §9. U0 §10's `parallel` rustdoc gap

At `b8b4d6f5` the `doc` job (`--all-features`, `ci.yml:134`) reads `parallel`'s rustdoc. PR-M1 retires that job, and
from then until PR-A2 lands, no row reads it. The `check` job's `--all-features` clippy still compiles it. PR-A2
closes the gap: the code is gone, so nothing is left to read. If PR-A2 lands before PR-M1, the `doc` job reads the
code up to PR-A2, so the gap has zero length.

## §10. Questions and out of scope

- **Q1** (answered in R1, Axis 4): no script-visible reader exposes the walk's `counter-*` fields. `getComputedStyle`
  returns `"initial"` for every `counter-*` property, whatever the field holds, on both paths. PM 611 made this a
  facet of the slot (§7.1). It does not bear on the shape.
- **Q2** (PM): the settled pool design (OPEN-015; `06-thread-model.md` §6.3.1 cores−2 renderer pool, §6.3.2 subtree
  unit, §6.3.3 main-thread join) has **no implementation home**. Defect: the main thread does not take part in the
  style phase's parallel work (it blocks while `par_iter` runs; §6.3.3), and the pool is rayon's global
  pool, sized to all cores and shared with wasmtime's compile, not the cores−2 renderer pool (§6.3.1) (F-4). No
  `#11-*` slot carries it (R1 Axis 5's ledger check) and no lane owns it. This memo returns it as a PM item to home,
  with a slot or a lane. The design is not reopened. Under remove (PM 624) it stays latent.
- **Out of scope**: the design's per-subtree parallelism and renderer pool (F-4, Q2); parallel layout; the list
  counters (§7.1); any change to `elidex-style` beyond §5.2; the matrix mechanisms themselves (PR-M1, PR-M2a,
  PR-M2b).
