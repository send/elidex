# Landing file 4 — `docs/plans/2026-10-python-pin-umbrella-rounds.md`

The python-pin umbrella's review record (body: `docs/plans/2026-10-python-pin-umbrella.md`): plan-review
dispositions (§12), the per-PR plan consequences (§13) and provenance (§14). Append-only. **Cross-references**: a
§n here is the body's section as it stood in the draft named; since draft 7, §4.1's rows, §4.2's measured tables,
§7.2's method and §11's recipes are in the records file as R§4.1, R§4.2, R§7.2 and R§11, and §5.2–§5.3 in the VM
P4 file, numbers kept.

## §12 Plan-review dispositions

### §12.1 Plan-review R1 (draft 1) — disposition

| Finding | Disposition |
|---|---|
| A3 IMP §2 is a fact table, 3 of 10 pairs | resolved: §2 = six invariants, all 15 pairs |
| A3 IMP single PR, no umbrella / base case | resolved: §6 |
| A3 IMP guard threshold source left to implementation | resolved: §4.3 + PR-2 §4.3 — exact, read at run time from `.tool-versions` through `scripts/python_pin.py`; no literal anywhere |
| A3 IMP cross-PR coupling only #514; #510 adds python3-shebang files | resolved: §8; shebangs are not launch evidence, #510's one entry point and its reachability are measured |
| A3 MIN tomllib deferral without slot; A-iii's "no `tomllib`" becomes false | resolved: not deferred — in draft 3, PR-1 (§6), which rewrites that sentence |
| A3 MIN G13 without owner / route | resolved: §10 |
| A3 MIN `mise run` auto-install unmeasured | resolved: measured (§7.1 row 1) |
| A3 note D5 inconsistency | resolved: §4.3 (exact everywhere) |
| A4 IMP F3 misread listing | resolved, refined: §4.1 — the listing is python-build's while `python.compile` is unset (draft 4's correction); installability is python-build-standalone's |
| A4 MIN F10 "every other job ubuntu-latest" | resolved: PR-1 §4.4 (`check` is an OS matrix; `grep -n runs-on .github/workflows/ci.yml`) |
| A4 MIN F7/F8/G13 not re-runnable | resolved: §4.1 row 3 and §10 commands; the by-path warnings runner is dropped (compile `-W error` is the re-runnable part kept) |
| A4 MIN #558 unmerged | resolved: landed as `de10bb04`; draft 4's base is `74e15df1` (§0) |
| A4 unverified "python-build-standalone builds are image-independent" | resolved: no longer relied on (S3 rejected) |
| A5 IMP cross-lane effects | resolved: §8 |
| A5 IMP pattern-derived positive list, no ratchet | resolved: PR-2 §4.1–§4.2 (launch evidence, unknown → red, independent lower bound, controls) |
| A5 IMP D3 / §8 / §12(1) not named | resolved: §5.1 |
| A5 IMP #514 wording list incomplete | resolved: §8 — derived by construct grep; F1's home named |
| A5 IMP (uncertain) global hooks resolve to the pin | resolved: measured, §7.2 |
| A5 MIN symptom-word site search | resolved: PR-1 §4.6 searches constructs (`legs?`, `matrix`, `setup-python`, `python-version`) — draft 3 notes that its `\b` spelling matched nothing (§13 C2) |
| A5 MIN contradicts the lane ledger | resolved: §5.1 amends it explicitly |
| A5 MIN G13 unassigned | resolved: §10 |
| A5 MIN pins' home files; regime file near 1000 lines | resolved: each per-PR plan names its pin files; §6 1000-line rule (now a command) |
| A5 MIN measured digits in the memo | resolved: §0 — commands only; digits only inside dated records |

### §12.2 Plan-review R2 (draft 2) — disposition

| Finding | What changed, where |
|---|---|
| **F1** §4.1's criterion picks 3.15.2; "neither has a 3.15.2 build" false | §4.1 retitled and rewritten as the 2026-10-31 decision record with a *Correction* paragraph (decision 117 (1)); 3.15.1 → 3.15.2 throughout (§6, §7, §11); "Which patch both serve" re-derived |
| **F2** row 3 measured on `de10bb04`'s suite set | §4.1 row 3 re-measured at `b051329b` with #510's and #501's members (driver's run, 2026-10-31); row 3x added (`sre_parse`, symlink loop) |
| **F3** #510 landed; floor-motivated code branches and two-leg statements on `main` | §5.2 (the VM P4 file/line table V1–V14 with the derivation commands and manifest impact); §2 I2 ("no branch for an interpreter the pin excludes"); §1; §4.4; §8 row #510 rewritten as landed; per-PR consequence C1 |
| **F4** #510 memo §8's timeout rule | §5.1 rows "#510 memo §8" and "§1"; §5.1 *Which timeout rule PR-1 follows* |
| **F5** PR-1 → PR-3 interval without I6 | §6: PR-3 folded into PR-1, with the four candidates and why; §2 I6 ("holds whole from PR-1 on") and ownership line; §9 step 4 now runs with compile-off in place; §0 landing paths |
| **F6** `mise.toml [tools] python` / `mise.local.toml` beat `.tool-versions` | §2 I1, I1 × I6, I4 × I6, I5 × I6; §4.2 fact rows (re-measured, mise 2026.10.21, incl. `MISE_PYTHON_VERSION`); §7.1 row 2; per-PR consequence C3 — generalised by draft 4 to T5's property (§12.3 G4) |
| **F7** X1 unbounded | §2 I4 (the lower bound bounds every exemption); §11 bullet; per-PR consequence C6 (`.claude/skills/elidex-plan-review/test_grep_pass.py:5`) |
| **F8** "Rust is on the latest stable (`de10bb04`)" was false | §1 and §4.1: ADR #36 as #567 applies it ("the pin tracks each new stable", 1.99.1); §0 base → `b051329b`, facts re-derived |
| **F9** tamper-slot trigger (b) fires on PR-1 | §9 step 3 (re-evaluation, and N4's relation to item (1)) |
| MIN §8 rows #501 / #514 / #506 | §8 rewritten with a state column; #501 landed + K2 rule for PR-2's shim; #514 base = main, list derived from its diff (ordering since decided, decision 124); #506 row (ordering since changed, decision 127) |
| MIN §4.4 #510's CLAUDE.md bullet | §4.4 ("discovery is authoritative; the list is examples", bullet named); §5.2 V14 |
| MIN §6 1000-line note | §6: a measuring command, plus the family's own bound |
| MIN §6 "no regression" PR-1 → PR-2 | §6 *The interval between PR-1 and PR-2* — rewritten by draft 4 (§12.3 G2) |
| MIN §11 inline Python | §11 bullet with its discovery command |
| MIN §6 cohesion argument for PR-2 | §6 *Why PR-2 is separate from PR-1, and one PR itself* |
| MIN #564 visibility vs admission | §2, the paragraph after the table |
| MIN "No ECS surface" | §0 |
| MIN `mise ls-remote` depends on `python.compile`; mise version on every claim | §4.1 *What `mise ls-remote` lists* (measured both ways; source corrected by draft 4); §0 rule; §4.2, §7.2 and §11 name the version or its absence |
| MIN §0 decision 41 "(a)(2)" not in the record | §4.2 S3 now cites decision 62 ("S3 kept", `PORTFOLIO:149`); §0 quotes decision 41 as recorded at `:59` |
| (found while revising) `git grep -E` with `\b` matches nothing on the measuring host | §5.2 and §8 commands spell word edges out; per-PR consequence C2 for PR-1 §4.6's command |

### §12.3 Plan-review R3 (draft 3) — disposition

R3 (2026-10-31, against `74e15df1`): CRIT 0, 6 IMP (G1–G6), ~16 MIN (`r3-disposition.md`). G1 and G2 decided by
PM decision 127; G3–G6 by the lens, accepted by the same decision. Section names in this table are draft 4's; where draft 5 retired
the mechanism (T5a / T5b, the walk-up lookup), §12.4 says what replaced it.

| Finding | What changed, where |
|---|---|
| **G1** "#506's split landed" is not an event; PR-1's real gate would be #506's merge | decision 127: the gate is **withdrawn**, PR-1 goes first and #506 absorbs it. §0 (decision 117 (3) marked withdrawn; 127 recorded); §6 table ("Not #506"), *Ordering* (incl. the M4-12 program consequence); **§5.3 NEW** — the PR-1 × split overlap table O1–O7 and the PR-2 rows P1–P2 (decision 127 (b)), read at the split's local `c032379d`; §5.2 heading (decision 127 (a)) and derivation ("at its own base"); §8 #506 row; §9 steps 3 and 7; §11 (unpushed split); §13 C5, C14 |
| **G2** PR-1 → PR-2 interval bound; ledger items citing PR-2 artifacts while #514 runs between | decision 127 option (ii). §6 *The interval between PR-1 and PR-2* rewritten: what holds, what regresses (bounded to CI measurement of direct launches outside the supported path), what is not in it, the time bound with #514 between; §5.1 ledger items 3 and 4 cite PR-1's artifacts only and say what PR-2 changes later; §8 #514 row (F1 withdrawn in the interval, E1 after PR-2); §9 step 5; §13 C13 |
| **G3** PR-1 after the fold bundles six pairs; edge-dense or not? | §0 and §6 *PR-1 is itself edge-dense*: its plan carries the intra-PR §2 pair matrix and its own plan-review; terminal under CLAUDE.md (c), not under axes.md:139's single-intersection wording, which its diff review does not invoke. §6 *Seams searched for interval-freedom*: T5a taken as **PR-0**; every other candidate fails on `main` or is false before the pin. §2 ownership; §0 landing paths; §9 step 2; §10; §13 C9, C11 |
| **G4** T5 enumerated bad keys by name; three tracked spellings passed it | §2 I1 (population by property, unknown → red); **§4.2 *What can spell Python's version to mise***: T5a's path property + the measured lower bound (17 + 5 paths, idiomatic files, mise 2026.10.21, `mise config ls`), T5b's admitted key shapes classified by `mise config explain` with every other key red (incl. `"core:python"`, `[tool_alias…]`, `tasks.<n>.tools`, `[env]`); §2 I1 × I6; §8 last row (a `[tools]` entry is a reader change); §11 (future mise releases); §13 C3, C11 |
| **G5** I6's "attested" half had no tracked writer or reader | §2 I6 (`python.github_attestations = true` beside `python.compile = false`, both asserted); §4.2 *The install settings* (the key's existence and both env overrides measured 2026-10-31); §2 I4 × I6 narrowed to the version spelling; §11 stated limit; §6 fold paragraph; §9 steps 3–4; §13 C4, C10 |
| **G6** decision 117 without a record location; its (4)–(6) not in the record | §0: 117 cited at `PORTFOLIO:217` for (1)–(3); F5–F9 approval as "PM message, decision 117"; MIN folding as draft 3's own processing; §7.2 (3) / §11 Linux as decisions 41 / 62; decisions 124 (`:228`) and 127 (`:232`) added with their content and the lane SSoT as second location |
| MIN §2 ownership: the tomllib consequence is I2 × I3 / I1 × I6, and the ordering coupling | §2 *Ownership* (I2 × I3 with I1 × I6; the reader runs only after Step 1's admission); I1 × I5 row ("binds the pin reader only") |
| MIN V6–V10: both outcomes of VM P4's answer | §5.2 *V6's two outcomes*; V6 row; §11 bullet |
| MIN §8 #514 row: decided by decision 124 | §8 #514 row, ordering column |
| MIN §5.1 "§6's UNCHECKED floor line (PR-1 / PR-2 say which)" without an owner | §5.1: A-iii `:441-443`, PR-1 rewrites it, PR-2 flips it; §13 C13 |
| MIN §5.2 stage-1 list misses #510 memo :434, :1472, :1647, :1687; CLAUDE.md :30, :114 | §5.2: **V15** (`:434`, coupled to V6/V9); `:1472` (word), `:1647` / `:1687` (dated records), `CLAUDE.md:30` (word), `:114` (CI/tools-owned) in the not-edited list; stage 2 also searches the memo |
| MIN §5.2 / §13 C1 globs miss the split's new test file | §5.2 stage 1 adds `'docs/plans/test_vm_p4*'` and the split plan's path; §13 C1 |
| MIN §8: no row for VM P4's small checker-family PRs; #506's other edits | §8 row "small checker-family PRs" (#562, #569); #506's edits in §5.3 (`:663` only in #510's memo — `:1098` / `:1103` stay per the split plan; `CLAUDE.md:55` as P1) |
| MIN §8 #506: split plan unpushed; HEAD; checker copy checkable locally | §8 #506 row and §5.3 *Source* (local `c032379d`, origin `190d2adb`, copy: none locally); §11 bullet replaced |
| MIN §13 C3 misses PR-1 :6-7, :105, :134 and PR-3's §10 (2)(3), §11, §12 item 2 | §13 C3 |
| MIN §0 decision 62's date | §0: 2026-10-28 |
| MIN §5.2 manifest rule: CONTROL / CASE / MUTANT digest different things | §5.2 *Manifest rule* rewritten with the three digests and their lines; V7 says *find* text |
| MIN §4.1 `ls-remote` with compile off gains 14 pre-releases, loses `3.15-dev`; plain listing is python-build's | re-measured 2026-10-31 (confirmed); §4.1 *What `mise ls-remote` lists*, *Which patch both serve*, the compile-off paragraph; §12.1 A4 row |
| MIN guard vs driver: where `.tool-versions` is looked up | §4.3 *Where the pin file is found — one rule* (from the launched file's path; T5a makes walk-up = root); §8 #501 row points to it |

### §12.4 Plan-review R4 (draft 4) — disposition

R4 (2026-10-31, against `1ed126b4`): CRIT 0, 6 IMP (H1–H6), ~18 MIN (`r4-disposition.md`). **Root check**: the
mise-input population drew findings in R3 (G4) and again in R4 (H1, H2) — T5 statically modelled a surface mise
defines and can widen in any release. Decided by PM decision 150 (1): change the tool — ask mise locally, one
fail-safe CI rule. H5 decided by decision 150 (3); the decision-141 note by 150 (4); H2, H3, H4, H6 approved by
150 (5).

**Terminator for R5 (decision 150 (2)).** If R5 finds a T5-class IMP — a tracked or untracked input through which
mise resolves or installs Python that the T5 rule, the oracle or the warning of **this draft** handles wrongly —
in the parts draft 5 adds (§2 I1 / I5 / I6 and their rows, §4.2 from *Why T5 changed shape* on, §4.3's fixed
location, §6's PR-0 shape), the driver sends it to the PM **unfixed**; the PM decides whether it goes to the user.

| Finding | What changed, where |
|---|---|
| **H1** `.miserc.toml` (`override_tool_versions_filenames` / `override_config_filenames`) outside T5a's regex; swaps the file mise reads | decision 150 (1): T5a/T5b retired. §4.2 *Why T5 changed shape*; **T5** clause 2 (component *starts with* `mise`/`rtx`, so `miserc` is a member) and clause 3 (`.miserc*` red outright); the oracle names the renamed file as the winning `source` (measured 2026-10-31, §4.2 table row 3); §2 I1, I1 × I6, I4 × I6 |
| **H2** a second `.tool-versions` line (`core:python 3.14.10`) wins in mise, not in setup-python / a first-line reader | §2 I1 (exactly one line); §4.2 T5 clause 1 (parsed by `python_pin.py`'s parser; PR-1); the oracle names `.tool-versions` as the source (row 4); §4.2 S2 row; C15 (PR-1 §4.1's comment line goes) |
| **H3** walk-up lookup reads an untracked nested `.tool-versions` | §4.3 *one fixed location*: root from `python_pin.py`'s own path; PR-2's guard by its entry file's fixed depth; §8 #501 row; C16, C19 |
| **H4** I5 half in PR-1, its pairs listed for PR-2 only | §2 I5 split into gate half (PR-1) and entry-point half (PR-2); §2 *Ownership* (PR-1 = I5 gate half; the I5 pairs split); §6 PR-1's ten pairs; C9 (pair list), C17 |
| **H5** "between PR-1 and PR-2 the lane's only work is #514" unchecked against the queue | decision 150 (3). §6 *Ordering* (cargo-deny PR after PR-2, because PR-2 §8's timeout re-derive edits `ci.yml`; M4-12 after PR-2 and the cargo-deny PR, pre-research after PR-1; sweep 4 anywhere — no `ci.yml` item); §6 interval bullet *What else runs in the interval*; §9 steps 5, 7; §8 rows |
| **H6** the supported path assumed by PATH order; the remedy a dead end when another `python3` precedes the shim | §2 I2 (supported path = the running interpreter is the pin), I5 (refusal names `sys.executable`), I5 × I6 (the branch `mise which python3` ≠ `sys.executable`); §7.1 table row 3 and *The supported path is what runs*; §6 interval (such a session is in the regression class); `which -a python3` measured for this session, other lanes via the PM (§9 step 4, §11) |
| **Decision-141 note** (fail red on the override env vars?) | decision 150 (4): warning only. §2 I6; §4.2 *the warning* (three variables, refusal path + driver Step 1 = every `mise run tools-test` / `mise run ci`, no exit change, no CI red); §11 stated limit; C16 |
| MIN §0 decisions 141 / 142; 127 says V1–V14; "freshness re-gate" source | §0: 141 (`PORTFOLIO:244`), 142 (`:245`), 150 (`:251`); 127 now quotes "the V1–V14 table from draft 4" and names V15 as draft 4's addition; "freshness re-gate" attributed to the VM P4 lane's SSoT (step 3), not to decision 127 |
| MIN §4.2 `MISE_ENV` probe listed 5 of mise's 8 paths | **moot**: no path list; every `MISE_ENV` variant is a member by clause 2 (§4.2 clause 4) |
| MIN §11/§6 the refusal on a non-pin interpreter unmeasured permanently | §11 *The refusal on a real non-pin interpreter*: UNCHECKED, owner CI/tools; measured once by PR-1 under `/usr/bin/python3`, re-measured on any `python_pin.py` edit (C16, C20) |
| MIN §6 the interval's closing time has no trigger / date | answered, not changed in substance: §6 *Bounded by order, not by date* (decision 127 bounded it by order; PR-2's landing report closes it, §9 step 6) |
| MIN §6 seams after PR-1 not tabled | §6 *Seams after PR-1* (table) |
| MIN PR-2's slice status (≥ 3 axes) not stated | §0, §6 *PR-2 is edge-dense too*; C18 |
| MIN §11 "re-checked at each mise bump" — mise is not pinned | **moot**: T5 does not depend on mise's list; §11 *T5's residual* says no event exists and none is needed; the oracle's shape dependence is its own §11 bullet |
| MIN §9 step 7 post-#506 ratchet is manual | §9 step 8 states it as manual, with its trigger and why it is not a pin; §11 bullet |
| MIN §8 / §9 PR-0's T5a a cross-lane red with no notice | §9 step 2 (PM notice with PR-0) and step 3 (with PR-1); §8 row *every lane* rewritten for T5 |
| MIN §5.3 P1 lists 1 of the split's launch lines; C12's "open question" already answered | §5.3 (c) P1 lists all six split-side lines targeting the checker; P2 states PR-2's scope (every tracked text file, PR-2 §4.2) and the hand-off; C12 rewritten |
| MIN §13 missing: PR-1 `python_pin.py` "given repository root" vs §4.3; §7.2 (4) re-run in PR-1 §12; PR-1 §12 step 4's renumbered cite | C16 (own path, no root parameter), C20 (§7.2 (1)–(2) and (4) re-runs, cite → umbrella §9 step 4) |
| MIN §6 1000-line rule vs #510's memo (1707 lines, edited in place) | §6 *One stated exception* |
| MIN `symbol_attribution_control` reads every `docs/plans/*.md`; the landing memos become its input | §5.2 *The landing memos are the checker's input*; §5.3 O8; C14 |
| MIN PR-0's probe must isolate `MISE_STATE_DIR` | **moot for PR-0** (no probe in its new shape); §0's isolation rule for every mise probe of the program; §4.1's recipes gain `MISE_STATE_DIR`; draft 5's probes followed it, and the user's trust and tracked-config listings were unchanged (§0) |

R4's ~18 raw MINs deduplicate to the fourteen MIN rows above (`r4-disposition.md`, "MIN (fold into draft 5)").

### §12.5 Plan-review R5 (draft 5) — disposition

R5 (2026-10-31, against `1ed126b4`): CRIT 0, 7 IMP — T1–T3 (T5-class, sent to the PM unfixed under decision 150
(2)) and J1–J4 — and the MINs of `r5-disposition.md` "MIN (fold into draft 6)" per axis. T3 decided by PM
decision 167 (a defect: fix); T1 / T2 by **user decision 170** (narrow the guarantee; the option text is quoted in
§0); J1–J4 accepted by decision 167, J3's command change ratified there. Draft 6 is written against `6aea28da`.

| Finding | What changed, where |
|---|---|
| **T1** tracked `[env] _.path` → `mise run` / `mise exec` run another `python3`; silent if it reports the pin, a dead-end remedy otherwise | decision 170: **narrowed, CI rule unchanged**. §0 (170 with the option text); §1 *What the program guarantees*; §2 I1 (version-qualified), I2 (supported path by version), I6 (holds for what mise installs, not for which interpreter a launcher runs), I1 × I6, I4 × I6, I5 × I6; §4.2 *Residual* ("none that changes the version is silent"), the oracle table's T1 row, the `mise which` branch naming both paths and the mise-added PATH entry as a cause; §6 interval *What holds*; §7.1 rows 1 and 4, *supported path*; §9 step 3 (`_.path` stays the tamper slot's); §11 *Decision 170's limit* and **T1** with its recipe; §13 C22 |
| **T2** `.miserc.toml` `override_tool_versions_filenames` → the oracle names the substitute file, not the rc file | decision 170: §11 **T2** with its recipe (same version admitted; other version refused naming the substitute; deleted substitute → `[]`); §4.2 oracle table rows (`[]`, T2 same version); the refusal's `[]` branch; §13 C22 |
| **T3** tests reaching the refusal call the real mise: write the user's `tracked-configs`, and CI / local take different branches | decision 167: **fixed** as invariant **I7** (§2) with owner PR-1 (helper + controls) and PR-2 (reuse), pins (a)–(c), and its six pairs; §0 (the probe rule is now an invariant); §2 I3 × I5; §11 (the real-old-interpreter run uses I7's environment); §13 C23 |
| **H6** (R4, decision 150 (5); re-stated by the driver for draft 6) the refusal is never a dead end | §2 I5 and §4.2 *branches*: every refusal names **both** `sys.executable` and mise's winning config as the oracle reports it (`source`, plus `mise which python3` when installed), or mise's error lines when it gives no answer; tests: C4 / C16 (e) assert both in every branch's text |
| **J1** pairs no per-PR matrix covered after the I5 split | §2 *Every pair × half, by PR* (21 pairs × PR-0 / PR-1 / PR-2, with the count command); §0, §6 (fifteen / eighteen); C9, C17, C18 |
| **J2** "compiles on old grammar" guarded by a header comment | §2 I1 × I5: two controls — `ast.parse(…, feature_version=(3, 9))` (measured best-effort) and the module's recorded SHA-256 tied to the last real-old-interpreter run; §11 bullet; C24 (replaces C16 (f)'s header rule) |
| **J3** §0 decision 150 (1)'s record names `mise current` / `mise config ls` | §0: the departure stated, with its measured reason, as ratified by decision 167 |
| **J4** #506's split is pushed | §0 decision 154; §5.3 *Source* (pushed `c032379d`, re-checked; merge base unchanged); §8 #506 row (checker-copy text dropped); §11 bullet removed; C7 |
| MIN A2 oracle JSON is an array | §4.2 *The output is a JSON array* (every element named; first-element rule; `[]`); table rows for `[]` and two elements; C16 (e) fixtures |
| MIN A2 untrusted worktree | §4.2 table (untrusted row, measured), the no-answer branch prints mise's first error lines and names `mise trust`; §7.1 new row; §9 step 4 |
| MIN A2 warning list | §2 I6, §4.2 *Warning*: every `MISE_PYTHON_*` except `MISE_PYTHON_VERSION`, plus `MISE_GITHUB_ATTESTATIONS`; C16 (c) |
| MIN A2 compound causes | §4.2 branches evaluated independently, all printed; §2 I5 × I6 |
| MIN A2 measured-benign residual examples | §4.2 *Residual* (`_.file`, rc-only setting in `mise.toml`), recipe in §11 T2 |
| MIN A3 PR-0 "single intersection" asserted | §0, §6, §2 table: PR-0's plan decides (C11) |
| MIN A3 PR-0 → PR-1 interval not stated | §6 *The PR-0 → PR-1 interval* |
| MIN A3 interval-free criterion applied unevenly | §6 *PR-0 rebuilt*: the criterion restated ("without weakening any invariant that holds on `main`"), applied to what PR-0 checks |
| MIN A3 "none widens the class" measured over the CI/tools queue only | §6 interval bullet: says so; other lanes' new entry points join the class until PR-2 |
| MIN A3 C9 missing PR-1's `mise.toml` writes × PR-0's clause 4 | §6 PR-1 slice-local pairs; C9; C21 |
| MIN A3 cite slots by ID | §4.2 S2, §10, §11 (`#11-python-pin-single-builder`, `#11-toolchain-pin-bump-automation`) |
| MIN A3 PR-0's verdict assumes `git ls-files` under the offline regime | §6 *PR-0 rebuilt*: conditional, with the driver-supplied list as the other branch; §11 bullet unchanged |
| MIN A4 "PR-2 edits `ci.yml`" inferred | §6 cargo-deny bullet says it is inferred; C19 makes PR-2's plan state it |
| MIN A4 decision 142 / 149 lines | §0: 142 at `PORTFOLIO:245-246`; cause and remedy are decision 149 at `:250` (R5 said `:248`, which is decision 147) |
| MIN A4 sweep 4's no-`ci.yml` claim inferred from IDs (K27) | §6 sweep 4 bullet |
| MIN A5 #571 shifts the `CLAUDE.md` `## CI` line | #571 **landed** (`e389fdc7`) — no open-PR row; §0 base `6aea28da`, `:114` → `:99` in §4.4 and §5.2 (the umbrella cites the line for PR-1; PR-1's plan names the paragraph without a number) |
| MIN A5 PR-1 §4.5's `mise.toml` text predates PR-0; its "after" must avoid `python` | C21 |
| MIN A5 C7 must drop pr2 `:199` "#506 (its checker copy)" | C7 |
| MIN A5 the umbrella is longer than 1000 lines | §6 *This umbrella's own length*: split at landing (§12–§13 → `…-rounds.md`, #510 precedent; §5.2–§5.3 next if still over); §0 landing paths |
| MIN A5 §9 step 3 quotes slot text not in the ledger; clause 4 incidentally reds `[env] PYTHONOPTIMIZE` | §9 step 3 quotes items (1) and (3) from the ledger row and states the incidental red |
| MIN A5 place the `RUSTUP_TOOLCHAIN` mise-task unset and the `ci.yml:167` item | §6 *Ordering*, two new bullets; §6 interval bullet |

### §12.6 Plan-review R6 (draft 6) — disposition

R6 (2026-11-01, against `ee6c7a1a`; frozen snapshot of draft 6, SHA-256 `8e779d8b…5144cf`; 5 × reviewer, at most two
at once, PM decision 204): after dedup **CRIT 0, IMP 3 (K1–K3), MIN 14 (M1–M14), FP 3** (`r6-disposition.md`). Two
IMPs come from base drift — #506 landed (`3ec0ae5c`) before PR-1 — not from a regression of draft 6; K3 is draft 6's
own addition. Draft 6's T5-class additions produced MINs only. Root check: K1 and K2 were order-dependent text
whose order was outside the lane's control; now that the order is fixed, the fix states the single landed state
and adds no contingency. Q1–Q3 and K3's direction decided by **PM decision 205**. Draft 7 is written against
`ee6c7a1a`.

| Finding | What changed, where |
|---|---|
| **K1** §5.3 written for "#506 after PR-1"; #506 landed first (O1 no merge-main; O3 the guard's cost already in the job; O4 PR-1's CI is the guard's first run on the pin; O5 / O6 on `main`; §5.2's "extra pathspecs match nothing"; "#506 touches no `.claude/**`"; the post-#506 ratchet already triggered; decision 154's head stale; C5 / C12 / C14) | §5.3 rewritten as the landed state at `ee6c7a1a`, one owner and one time per row (O1–O8, P1–P2), the `axes.md` edit stated; O5 / O6 → **V16 / V17** (decision 205 Q1); §5.2 derivation at `ee6c7a1a` with #506's paths (+ `docs/plans/2026-07-vm-p4-*`), V14 a plain edit; §0 (base, decisions 127 / 154 / 205); §6 *Ordering* close; §8 #506 row; §9 steps 3 and 8 (no post-#506 step); §11 ratchet bullet removed; §13 C5, C12, C14 restated, C26 |
| **K2** P2 (`seam-split.md:333`, `:799`) red at PR-2's own base | decision 205 Q2: **in PR-2's edit set** (§5.3 (b) P2 with the rewrite constraint), PR-2's *Depends on* (§6 table) and §9 steps 3 and 6; consent in the one VM P4 request; §13 C12, C25 |
| **K3** "split at landing" cannot reach ≤ 1000 lines | decision 205: the seam set decided now — four landing files (body §0–§11; VM P4 §5.2–§5.3; records R§0 / R§4.1 / R§4.2 / R§6 / R§7.2 / R§11; rounds §12–§14), counts measured on draft 7 (§6 *This umbrella's own length*, §14); §0 *Layout* |
| M1 J2's controls: the SHA-256 record forces a record, not a run; `feature_version` gaps on the pin | measured on 3.15.2 (R§11 *J2 probe*: four accepted gaps, incl. `lazy import` and comprehension unpacking); both limits in §11's refusal bullet and §2 I1 × I5; C24 |
| M2 "print the two commands" undefined; the no-mise remedy `mise install` is a dead end | §4.2 branches: no-mise split out with "install mise, then `mise install`, or run with an interpreter of the pin's version"; "the two commands" = the oracle's commands rooted at the repository; §7.1 no-mise row; §11 oracle bullet; C27 |
| M3 I7 (b) / (c) must compare real paths | §2 I7: `os.path.realpath` on both sides, with the `/var` vs `/private/var` reason; C28 |
| M4 the real old-interpreter run has no recipe; empty state makes `mise.toml` untrusted | R§11 *The real old interpreter* (`MISE_TRUSTED_CONFIG_PATHS=<worktree>`, `/usr/bin` first); §11 bullet; C20 |
| M5 child vs in-process not fixed | §2 I7: every fixture reaching the oracle or the warning runs `python_pin.py` as a child under the helper; in-process only for pure functions; C28 |
| M6 the `mise which` branch lists two causes without a read; T2 (b)'s remedy lands in T2 (a) | §4.2 branches: `mise env --json` read (measured, R§11 *M6 read*) and the remedy per cause; "remove the winning input, never write the pin into it"; §11 T1 (b), T2 (b); C27 |
| M7 home of `isolated_mise_env` | §2 I7: `scripts/python_pin_testing.py`, a non-member support module; §6 PR-1 scope; C28 |
| M8 T5's home in a two-file reader | §4.2 T5 and §6 PR-0 row: PR-0's plan records why, or names its own module, and states the launcher-passed-data case; C29 |
| M9 CLAUDE.md `## CI` paragraph is `:100` | §0, §4.4, §5.2 not-edited list |
| M10 the suite population is ten; 89 `.py` | §0; R§0 (census, compile under `-W error` on 3.15.2 and 3.9.6, the guard green alone in the driver's mode) |
| M11 the stale "0 hits" note near V12 / V13 / V15 | decision 205 Q3: **V18** in PR-1 (§5.2, stage 1b); three sibling lines in #506's slot-ledger memo reported to the PM |
| M12 placement of the three CI/tools slots | §6 *Ordering*: sandbox lints anywhere; private-doc-links and default-features after PR-2 if they edit `ci.yml` |
| M13 cite decision 170's option text at its text location | §0: the lane SSoT bullet named |
| M14 `\|` in table-cell commands | §0 *Layout*: the escape stated |
| FP §3 has no spec surface; mise 2026.10.25 re-ran every oracle row identically; the two PR-1 slots register at landing | noted: §0 (mise version), R§4.2, §9 step 3 (registration) |
| Brief premise: the guard imports the hyphenated checker | false — it imports `plan_memo_population` / `plan_memo_tables` (§0, §5.3 O4); CLAUDE.md:54's "library" wording is right in substance |

R6's raw axis reports: `axis1.md`, `axis2.md`, `axis3.md`, `axis5.md` in the run directory; Axis 4's report is
not in that directory (its findings are in `r6-disposition.md` only).

**Draft 7 fold (PM decision 211, 2026-11-01)** — the three items draft 7 reported to the PM: M11's sibling lines
→ **V19–V21** (D7-1 (a); both slots' homing re-checked, §5.2 V19); the guard's `-W error` warning → **V22** (D7-2;
with the two mutation rows and two MUTANT lines it couples to); the `ci.yml:167` item → after PR-2 (D7-3, §6).
Found while folding, not decided: the slot ledger's `:327` (§5.2 *Stage 1b's hits*), reported to the PM.

### §12.7 Plan-review R7 (draft 7 + PM 211 fold) — disposition

R7 (2026-11-01, against `ee6c7a1a`; frozen snapshot of the draft-7 fold, SHA-256 `f7d75b2b…a5b31`; 5 × reviewer,
at most two at once): after dedup **CRIT 0, IMP 6 (J1–J6), MIN 19, FP 5** (`r7-disposition.md`), plus V23 (PM
decision 213). No IMP from the draft-6 parts R6 cleared; every IMP came from what draft 7 added (the K3 split,
M4, the PM 211 fold); J2 regresses R6 K1. **Root check (PM decision 217, option (a))**: the refusal's
cause-inference layer had produced findings three rounds running (R5 T1 / T2, R6 M6, R7 J4 / J6), each designed
by inference and not measured — a generator, so it is removed, not patched: the refusal prints observed facts and
one generic block (body §2 I5, §4.2). **R8's terminator, fixed in advance (PM decision 217)**: R8 IMP 0 →
converged; IMPs clustered in what draft 8 added → no R9, the memo becomes a PR and moves to `/external-converge`
(the user's N1 precedent: change the instrument); any other IMP → the PM. Draft 8 is written against `ee6c7a1a`.

| Finding | What changed, where |
|---|---|
| **J1** (A5-1) PR-1 edits a VM-P4 family memo but takes no CLAUDE.md:55 checker-run duty | body §9 step 3 (the run, exit 0 expected; measured 0 at `ee6c7a1a` on 3.15.2 and 3.9.6, R§0) and the consent request; §8 #506 row; §5.2 preamble; §5.3 O9; C32 |
| **J2** (A5-2, regression of R6 K1) O4 understates the guard's import closure | §5.3 O4: the twelve-module closure (measured by `ast`, R§0), V6 and V22 inside it with V11; the coupling joins the consent (body §9 step 3); C14 |
| **J3** (A5-3) V18 records the wrong event | §5.2 V18: false since **#510's** landing (`282f04d6`, 2026-10-26), in the ledger rows' own words ("added to the index at #510's landing, 2026-10-26"; R§0), not #506's; the replacement avoids "registered" and stage 1b's tokens. V19's replacement drops "registered" too (the slot ledger's `:278`: an index line is not a registration); C26 |
| **J4** (A2-1, A4-2, + A3-2) the real-old recipe's mise path is not wired; the post-check cannot fire | R§11 *The real old interpreter*: `type -P` / `whence -p` (measured where `mise` is a shell function), the post-check as positive evidence in `$ISO/state/tracked-configs` (measured), the blind diff stated; C20 |
| **J5** (A2-2) landing files quote X1-shaped launch lines | §5.3 P2 and C6 describe instead of quoting; body §11 and §5.3: landing files add none (0 hits, R§0); C33 |
| **J6** (A4 IMP + A2-3) the M6 cause rule fails on a real interpreter | **vanished with the layer** (PM decision 217): `mise env --json` is no longer read; R§11 *M6 read* withdrawn (its measurement kept as record). No redesign — a redesign is the generator decision 217 removed |
| A1-1 the helper module's home vs driver step 3 and PR-2's reachability; the child launcher unnamed | body §2 I7: no `TestCase`, no `__main__`; loaded through the constant `"scripts/python_pin_testing.py"`; the launcher a `-c` source string; C34 |
| A1-2 §5.3 lacks V19–V21 / V22; V22's conditional owner has no time | §5.3 O7 (V22, owner decided by PR-1's stage 1c at its own base), O9 (V19–V21, V23) |
| A2-3 the M6 compare is realpath on one side | **vanished with the layer** (no compare remains) |
| A2-4 the `mise env --json` failure path | **vanished with the layer** (no such read); the two remaining questions' failure path is §4.2 item 4 |
| A2-5 C16 (e)'s compound fixture expects an unreachable cause line | **vanished with the layer** (no cause lines; C16 (e) restated: every fixture prints the same fact kinds and the one block) |
| A2-6 I7 child env vs the offline regime | body §2 I7: from `os.environ`, regime keys untouched, no `-I` / `-E` / `-S`, a control; C34 |
| A3-1 the real non-pin refusal is guarded by review only | considered, not taken, with the reason (R§11 *The real old interpreter*); body §11 points there |
| A3-3 stage 1c is a symptom grep | measured the property: the set on 3.15.2 with `PYTHONWARNINGS=always::DeprecationWarning` — V22's line only (R§0); stage 1c labelled a lower bound; C35 |
| A3-4 no reason or trigger for not pinning mise | body §11 (no guarantee rests on mise's version; trigger: a mise answer deciding an exit status) |
| A4-3 (A4-1 in the disposition) Expected `sys.executable` for `/usr/bin/python3` | R§11: the Xcode path, measured; body §4.2 item 2 |
| A4-4 (A4-2 in the disposition) the slip-through list | R§11 J2 table re-measured on 3.15.2 and 3.9.6: parenthesised context managers removed (3.9.6 parses them), PEP 646 ×2, `a[x := 1]`, PEP 798 dict added — seven gaps; body §2 I1 × I5, §11; C24 |
| A4-5 (A4-3) the `match` snippet is unindented | R§11 J2 table: indented, `·` marking spaces |
| A4-6 (A4-4) two enumerations of the J2 red set | R§11: the six rejected rows are the one enumeration; body and C24 cite it |
| A4-7 (A4-5) the untrusted refusal blamed on PR-1's `[settings]` | re-measured: `main`'s `[bootstrap.packages]` (#535) already makes mise exit 1 untrusted (R§0, R§4.2 row); body §7.1 row 3, §9 step 4, §11 refusal bullet, R§11 |
| A5-4 memory-ledger lines off by one, and drift | V19, R§0: cited by slot ID and heading, not line |
| A5-5 "merged 2026-10-31" vs JST | body §0, §8, §5.3 *State*: 2026-11-01 02:35 JST (`git show -s --format=%ci 3ec0ae5c`) |
| A5-6 "V1–V15 unchanged since `74e15df1`" false for V14 | body §0, §5.2: V1–V13 and V15; V14 re-read at `ee6c7a1a` |
| A5-7 pointers at §4.1 rows that moved to R§4.1 | body §0, §5.2 V5, R§0 |
| A5-8 present-tense counts | body §0: "at `ee6c7a1a`", with what PR-1 adds |
| A5-9 the LF2 intro and §5.2 heading omit decision 211 | both cite 205, 211, 213 |
| V23 (PM decision 213) | §5.2 V23 (verified at `ee6c7a1a`, R§0; slot homed), §5.3 O9, the one consent request; reviewed by R8 |
| FP: §0 "No ECS surface"; §11 Linux bullet; §10 G13; the two PR-1 slots; MEMORY.md's CI/tools line | noted, unchanged |

**Draft 8's own departures, for R8 to read first**: (1) the generic block has **four** actions, not decision 217's
three — without "remove what puts `sys.executable`'s directory ahead of mise's install on PATH", T1 (b) is a dead
end (R§4.2 *Each case's fix*); (2) the refusal also prints the cwd and the root, the version found and the pin
file (facts beyond decision 217's list); (3) R6 M6's "never write the pin into the winning input" is withdrawn
with the cause layer (body §11 T2); (4) found, not decided: the slot ledger's `:278` and the *Re-eval* cells of
`:327` / `:328` (§5.2 *Stage 1b's hits*); (5) the V16–V23 request is sent once the umbrella converges (decision
211 said "after R7 converges", which R7 did not).

---

## §13 Per-PR plan consequences (drafts 3–8)

For the driver to schedule; the per-PR plans are not edited by this draft. C1–C8 are draft 3's, C9–C14 draft
4's, C15–C20 draft 5's, C21–C24 draft 6's, each amended where a later draft changes it; C25–C29 are new in draft 7, C30 in its
decision-211 fold, C31–C36 in draft 8 (which amends C4, C6, C14, C16, C20, C24, C26, C27).
This list is complete for draft 8: every per-PR plan statement that draft 8 makes false or incomplete has an item
here.

- **C1 — PR-1 §4.6** gains the class **floor-motivated code branch** (edit: remove under the pin, with its coupled
  rows — mutation rows, controls, comments, coverage rows — in the same commit) and **live rule or present-tense
  description in a landed memo** (edit: amend in place with a dated marker; `docs/plans/**` is otherwise excluded
  as landed records). §4.6's discovery covers §5.2's derivation, including the pathspecs for the split's files
  (`docs/plans/test_vm_p4*`, `docs/plans/2026-10-vm-p4-memo-seam-split.md`) and the memo in stage 2; PR-1 §8's
  edit set and §11 / §12 list §5.2's files (V1–V22, draft 7 with decision 211), the manifest regeneration, V6's two outcomes and the
  PM route to the VM P4 lane; draft 7 adds `docs/plans/2026-07-vm-p4-*` to stage 1 and stage 1b (§5.2).
- **C2 — PR-1 §4.6's discovery command** uses `\blegs?\b`, which `git grep -E` matches nowhere on the measuring
  host (`git grep -cE '\blegs?\b' origin/main -- .github/workflows/ci.yml` prints nothing, while
  `git grep -cwE 'legs?'` counts the lines): spell word edges out as §5.2 does, and re-run.
- **C3 — PR-1 absorbs PR-3** (`pr3-mise-toml-tomllib.md`): its §4.1 properties, §4.2 `[settings]` line, §4.3
  A-iii rewrite and T1–T4 move into PR-1 (§4.5, §4.7, §6), and so do PR-3's §10 exit criteria (2) (no `mise.toml`
  grammar code remains, by its `git grep` of the helper names) and (3) (A-iii §6 reads as its §4.3), its §11 sweep
  (`git diff --stat origin/main...<head> -- scripts` per open PR) and its §12 item 2 (report the tamper-slot trigger
  and the A-iii §6 rewrite). PR-1's own text that presumes PR-3 is rewritten: `:6-7` ("I6 (except the
  install-time compile refusal, PR-3's)", "Lands **first** of the three; PR-2 and PR-3 build on it"), `:105`
  ("`[settings] python.compile = false` is PR-3's"), `:134` ("§6's text-reader rationale is PR-3's"), and PR-1
  §2's "In the interval nothing can trigger the fallback" bullet is deleted; PR-1 §5 gains PR-3's deltas.
  **T5 (draft 5)**: draft 4's T5a / T5b are retired (decision 150 (1)). PR-1 carries **T5's clause 1** only (§4.2:
  the root `.tool-versions` is exactly one `python X.Y.Z` line, parsed by `python_pin.py`'s parser) and the I6
  assertion of the two `[settings]` lines over parsed TOML (C10); **no** key classification of `mise.toml` — the
  admitted-key-shape list, its `mise config explain` remedy and its controls (`"core:python"`,
  `[tool_alias.python.versions]`, `tasks.<n>.tools`, a `MISE_*` task-env key, an extra `[settings]` key, a
  not-yet-existing key) leave PR-1; the same inputs are red by PR-0's clause 4 because they spell `python`.
  Clause 1's controls: a second line (`core:python 3.14.10`), a comment line, a blank line, a second tool, `3.15`,
  `latest`, a pre-release, CRLF, no file — each red; the one line with and without a final newline — green. `pr3`
  is retired, and the umbrella's landing paths drop it (§0).
- **C4 — PR-1 §4.2 (b)**, the admission diagnostic, is rebuilt around the oracle (§4.2, §2 I5, I5 × I6): it names
  the pin, the version found, **`sys.executable`**, the pin file, and the remedy chosen by `mise ls --current
  --json python` (+ `mise which python3` when installed) asked at the cwd, with the branches of §4.2 (no mise /
  no answer / an error, with mise's first error lines and `mise trust` for the untrusted case; another source
  wins — every array element named by type and path or variable; `[]`; not installed; another interpreter ahead
  of the shim, both paths named; cwd outside the repository), **evaluated independently, every one that holds
  printed** (draft 6). Every branch's text names **both** `sys.executable` and mise's answer (H6). It promises
  nothing about the install settings beyond the warning, and nothing about which build of the pin's version runs
  (decision 170, §11). **Draft 8 replaces the branches** by facts and one generic block (PM decision 217): C31.
- **C5 — PR-1 base and ordering** (restated in draft 7): base = `main` at PR-1's own time, which already holds
  #506 (`3ec0ae5c`); PR-1 depends on PR-0; "red at main" in PR-1 §6 is re-read at that base; PR-1 §4.1's
  `.tool-versions` line is `python 3.15.2`; N2's green example follows; the suite set has ten members, including
  #506's guard `docs/plans/test_vm_p4_family_layout.py` (§0); PR-1 §8's timeout re-derive follows §5.1's *Which
  timeout rule* and is the first measured with the guard on the pin (§5.3 O3); PR-1 §12 step 3 adds the
  tamper-slot re-evaluation (§9 step 3). No hand-over of §5.3 at PR-1's merge commit: nothing absorbs PR-1.
- **C6 — PR-2 §4.1 / §4.2 / §6**: X1's exemption gets X2's bound — R-b's lower bound red on a tracked launch
  instruction targeting an X1 file (an E3/E4 control for it); the instance at `b051329b`,
  `.claude/skills/elidex-plan-review/test_grep_pass.py:5` (its docstring's "Run via:" line, which launches that
  file directly with `python3`; described, not quoted, since draft 8, R7 J5), is rewritten to the driver (`mise
  run tools-test`). PR-2 §2's "X1 × I2" bullet and §4.4 follow. At `ee6c7a1a`
  #506's two (§5.3 P2) are on `main` too — C25.
- **C7 — PR-2 §4.1's re-derivable evidence** is re-run at PR-2's base (it now includes
  `plan-memo-umbrella-check.py`; at `b051329b` the E-b grep also matches `scripts/python-suites.sh`, a shell file
  with inline Python, which the predicate's "Python source" definition excludes — say so); PR-2 §0 / §8 / §11
  drop "preferred after #510" (landed) and gain the K2 constraint on the `webref` shim's prelude (§8 row #501),
  with the root found by §4.3's fixed location (C19). PR-2 §11's hand-off list drops "#506 (its checker copy)"
  (`pr2-entry-point-guard.md:199`): #506 landed without one (§5.3); #506's P2 moves from the hand-off list into
  PR-2's edit set (C25).
- **C8 — PR-2 §9 step / landing**: report the close of the PR-1 → PR-2 interval (§6, §9 step 6).
- **C9 — PR-1 is edge-dense (G3, H4)**: PR-1 §0 says so; PR-1 §2 becomes its intra-PR pair matrix — the
  fifteen gate-half pairs of §2 *Every pair × half* (I1 × I2, I1 × I3, I1 × I5, I1 × I6, I1 × I7, I2 × I3,
  I2 × I5, I2 × I6, I2 × I7, I3 × I5, I3 × I6, I3 × I7, I5 × I6, I5 × I7, I6 × I7; draft 5 listed ten, before I7
  and J1), each with its PR-1 mechanism and the pin that holds it, plus the slice-local pairs (scratch repos × I1,
  the oracle × I3 (no mise in CI: the fake-`mise` fixtures), the VM P4 removals × I2, the `mise.toml` reader ×
  Step 1's ordering, **PR-1's own `mise.toml` writes × PR-0's clause 4**, C21) — replacing its one-line-per-pair
  list; PR-1 passes its own `/elidex-plan-review` (already required);
  PR-1's diff review cites the umbrella and its plan as the split's trace, not axes.md:139's exclusion.
- **C10 — PR-1 `[settings]` (G5)**: `mise.toml` gains `python.github_attestations = true` beside
  `python.compile = false`, spelled exactly as T5's clause 4 (b) admits them; T1 (or a T6) asserts both keys,
  their `[settings]` table and their values; a control with each value flipped is red (and, through clause 4,
  a flipped line is no longer admitted); PR-1 §11 / §12 state §11's limit (untracked overrides) and the warning,
  and PR-1's landing step re-measures §4.2's two attestation rows under the then-current mise.
- **C11 — PR-0's plan is new** (`…-pr0-tracked-mise-config.md`), for T5's clauses 2–5 as §4.2 defines them — the
  member predicate, the red-outright clause, the no-`python` clause with its admitted occurrences (tracked-path
  substrings; the two `[settings]` lines, admitted ahead of PR-1), the diagnostic — and the reword of
  `mise.toml:115`. Controls: red — `.miserc.toml`, `.miserc` (a dot-less `miserc.toml` too), a nested
  `sub/.tool-versions`, `.python-version`, `.python-versions`, `.config/mise/conf.d/py.toml` and `mise.ci.toml`
  each holding a python line, a file task `mise/tasks/x` with a `#MISE tools=` python header, `[env]
  MISE_PYTHON_VERSION` in `mise.toml`, `python.compile = true`, a `.rtx.toml` with a python line; green — the
  live tree, a nested `mise.toml` with `[tools] node = …` only, the `run` line naming `scripts/python-suites.sh`,
  the two admitted lines, the root `.tool-versions` (left to clause 1); not members — the `*promise*` Rust
  files. It reads A-iii's `mise.toml` reader and pins first, in case one pins the description it rewords (§11).
  No mise probe (T5 does not model mise; §0's isolation rule applies if one is added). The offline-regime
  question (§11), and with it the branch of §6 *PR-0 rebuilt* (its own `git ls-files`, or the driver's list). The
  edge-dense check of its own scope **is the plan's to decide** (draft 6; R5 MIN): the §2 cell is I1's tracked
  half × I6's two admitted lines, and the plan weighs whether `main`'s two legs, the offline regime and A-iii's
  `mise.toml` reader make it more than one intersection. It states the PR-0 → PR-1 interval (§6). It runs on both
  of `main`'s legs and must be green on `"3.9"`.
- **C12 — PR-2 and #506 (§5.3 (b))** (restated in draft 7): R-b's scope is decided by PR-2's plan — every tracked
  text file (PR-2 §4.2), landed plan records included. The six lines launching `plan-memo-umbrella-check.py`
  (P1) are green once that entry point carries the guard; the two launching `docs/plans/test_vm_p4_family_layout.py`
  (P2) are on `main` and red under the X1 bound at PR-2's own base, so PR-2 edits them (C25). `CLAUDE.md:55`
  needs no change.
- **C13 — UNCHECKED → CHECKED after PR-2**: PR-2 flips, wherever they are on `main` at its base, A-iii `:441-443`
  (as rewritten by PR-1), the ledger's item 3 and A-ii's line (if #514 has landed), and B's F1 pointer (to its E1,
  `scripts/test_python_entry_points.py`); if #514 has not landed, #514's next rework (same lane) does the A-ii / B
  part before #514 lands. PR-2 §9's landing report lists each flip.
- **C14 — PR-1 and #506 (§5.3)** (restated in draft 7): PR-1's plan carries §5.3 (a)'s O1–O8 as its cross-lane
  table — lines and couplings on `main`, each with PR-1 as owner (O8 split with VM P4) — not as an absorption
  table; the right-to-left contingency is gone. PR-0, PR-1 and PR-2 each run the suites with their landing files in
  `docs/plans/` (O8: `symbol_attribution_control` reads them; PR-1 lands four umbrella files, §6). **Draft 8
  (R7 J2)**: O4 names the guard's twelve-module import closure and V6 / V22 inside it; PR-1's table carries O7's
  V22 owner rule and O9 (the slot ledger, V19–V21, V23, and the checker run).
- **C15 — PR-1 §4.1, the pin file (H2)**: `.tool-versions` is the one line `python 3.15.2` — the planned comment
  "pointing at `scripts/python_pin.py`'s header for the bump rule" is dropped (a comment line is red by clause 1);
  the pointer to the bump rule lives in `python_pin.py`'s header and in README / CONTRIBUTING.
- **C16 — PR-1 §4.2, `scripts/python_pin.py`**: (a) it reads the root `.tool-versions` located from **its own
  path** (H3) — the "from a given repository root" wording goes; scratch-repo pins copy the module into each
  scratch tree (as PR-2's E3 does); the parser accepts exactly the one line (C3's controls); (b) the refusal as
  C4; (c) the warning of §4.2 (every `MISE_PYTHON_*` except `MISE_PYTHON_VERSION`, plus `MISE_GITHUB_ATTESTATIONS`
  — draft 6; stderr; no exit change; printed on refusal and by Step 1; a control per class: a measured variable, an
  unmeasured `MISE_PYTHON_*`, `MISE_PYTHON_VERSION` not warned); (d)
  "no imports beyond `sys` / `os`" becomes "beyond `sys` / `os`, plus `subprocess` and `json` imported on the
  refusal path only" (I1 × I5); (e) controls: a fake `mise` on PATH printing each JSON shape of §4.2's table
  (including `[]`, two elements, the T2 substitute source), one that hangs past the timeout, one that prints
  garbage, one that exits 1 with the untrusted-config stderr, none on PATH — each yields the refusal and its
  branch, never an admission, each text names both `sys.executable` and mise's answer (H6), and a compound
  fixture (another version wins *and* `mise which` ≠ `sys.executable` *and* a warned variable) prints all three
  (**draft 8**: no cause lines exist; each fixture asserts the fact lines of §4.2 *The refusal* and the one
  generic block, byte-identical across fixtures — C31);
  every one of these runs through I7's helper (C23); (f) the re-measure rule of §11 is **C24's pin**, not a header
  rule; the header keeps the bump rule and the oracle probe scripts with §0's isolation lines.
- **C17 — I5 split (H4)**: PR-1 §0's ownership line names I5's gate half; PR-2 §0's "This slice owns I2
  (entry-point half), I4, I5" becomes "… I5 (entry-point half), and I7 for its tests"; PR-2 §2 lists, from §2
  *Every pair × half* (J1, draft 6), the **entry-point halves** of I1 × I2, I1 × I5, I1 × I7, I2 × I3, I2 × I5,
  I2 × I6, I2 × I7, I3 × I5, I3 × I7, I5 × I6, I5 × I7 and I6 × I7, consuming PR-1's refusal and I7's helper
  unchanged. (Draft 5 listed four of these; R5 J1 found I1 × I2, I2 × I3 and I2 × I6's entry halves missing.)
- **C18 — PR-2 is edge-dense** (R4 MIN): PR-2 §0 says so; PR-2 §2 is its intra-PR pair matrix — the eighteen
  pairs × halves of §2: the **whole** pairs I1 × I4, I2 × I4, I3 × I4, I4 × I5, I4 × I6, I4 × I7 (draft 5 listed
  I2 × I4 and I4 × I5; R5 J1 found I1 × I4, I3 × I4 and I4 × I6 missing; I4 × I7 is draft 6's), plus C17's entry
  halves; its own plan-review (already required by §9 step 1).
- **C19 — PR-2's guard and the fixed location (H3)**: the prelude finds the root by its entry file's fixed
  tracked depth and loads `<root>/scripts/python_pin.py` — not by walking up to `.tool-versions`; E5's "launched
  from another cwd and through a symlink still finds the repository's `.tool-versions`" holds by
  `Path(__file__).resolve()`; an entry file moved to another depth finds no `scripts/python_pin.py` at the root
  it computes and refuses naming that path (fail-closed; an E5 control). PR-2 §8's `tools` timeout re-derive in `ci.yml` is load-bearing for
  §6's ordering of the cargo-deny PR, and today it is only **inferred** from `ci.yml`'s own rule (R5 MIN): PR-2's
  plan states it in words as an edit of `ci.yml`, or, if it drops it, reports that to the PM so the cargo-deny PR
  moves (§6).
- **C20 — PR-1 §12 (landing checklist)**: step 4's cite "umbrella §9 step 3" → **§9 step 4**, and it adds the
  `which -a python3` request; a step re-runs §7.2 (1)–(2) under the then-current mise and names it, and re-runs
  `usage-pace.test.py` on 3.15.2 (§7.2 (4)); a step runs the refusal under macOS `/usr/bin/python3` — in I7's
  isolated mise environment, by R§11's *real old interpreter* recipe (`MISE_TRUSTED_CONFIG_PATHS=<worktree>`,
  draft 7; **draft 8**: mise's path by `type -P` / `whence -p`, never `command -v`, and the post-check is the
  listing of `$ISO/state/tracked-configs` holding `<W>`'s entries, recorded in the PR body; the expected
  `sys.executable` is the Xcode path) — records it (§11) and writes C24's digest record; step 3's report includes the
  notice that the root `.tool-versions` is one line (§9 step 3); step 4 names `mise trust` for an untrusted
  worktree (§9 step 4).
- **C21 — PR-1 §4.5, `mise.toml`'s task text (R5 MIN)**: the "before" is **PR-0's reworded** description (§4.2
  clause 4: the interpreter first on PATH, floor 3.9, without the word `python`), not `main`'s "(floor: Python
  3.9)", and the "after" must spell no `python` outside clause 4's admitted occurrences — PR-1's planned "on the
  pinned interpreter (`.tool-versions`; `mise run` installs it if missing)" passes; PR-1's other `mise.toml`
  writes (the two `[settings]` lines, spelled exactly as admitted) are the slice-local pair "PR-1's writes ×
  PR-0's clause 4" (C9), with a control that the post-PR-1 `mise.toml` is green under clause 4. Its "(… is
  PR-3's, §2)" line goes (C3).
- **C22 — the narrowed guarantee (decision 170)** in the per-PR plans: every statement that a non-pin interpreter
  is never admitted, that a refusal is "never silent", or that I6 holds whole, is restated by version — "an
  interpreter of another version is refused, naming it; one of the pin's version from another source is
  admitted (umbrella §11)". At least: PR-1 §0's ownership line (I6), PR-1 §2's I2 / I6 bullets (and a
  stated-limits paragraph pointing at umbrella §11 T1 / T2 / same-version — PR-1 has no limits section), PR-1 §5's
  behaviour deltas and **§10 exit criterion**; PR-2's title ("admits only the pin"), §1, §2's X1 × I2 bullet,
  §4.4 *Limits* (gains the same-version bullet), §5 row 1 and **§10 exit criterion**. A DoD or exit criterion
  that says "refuses every other interpreter" becomes "refuses every interpreter whose version is not the pin". No new CI check follows from it (§4.2 clauses
  1–5 unchanged).
- **C23 — I7 in the per-PR plans (decision 167)**: PR-1 §2 / §6 add I7's helper (`isolated_mise_env`) to PR-1's
  pins and route N1, every fake-`mise` fixture (C16 (e)) and the warning controls through it, with pins (a)–(c)
  of §2 I7 and its mutant (PR-1 §6, *Pins*); PR-1 §4's edit set names the helper's file. PR-2 §6's E1 and every test that executes
  an entry point (E3–E5) reuse it; PR-2 §2 lists I7's entry halves (C17). Both plans' acceptance includes the
  control that the real `tracked-configs` / `trusted-configs` hold no entry into the test's root.
- **C24 — J2, the grammar pin for `python_pin.py`**: PR-1's pins gain (i) a control `ast.parse`-ing the module
  with `feature_version=(3, 9)` (a mutant inserting a `match` statement is red) and (ii) a control holding
  `(sha256, date, interpreter)` of the last real-old-interpreter refusal run (C20), red when the module's digest
  differs; editing the module therefore requires the run and a new record in the same PR. This replaces the
  header-comment rule of C16 (f) and draft 5's §11 wording. **Draft 7 (R6 M1)**: (i)'s mutants are R§11 *J2
  probe*'s rejected rows, and PR-1 states (i)'s measured gaps on 3.15.2 (parenthesised context managers, PEP 701
  f-strings, `lazy import`, comprehension unpacking) and (ii)'s limit (it forces a record, not the run; the run is
  enforced by review and C20's checklist) where it states the controls. **Draft 8 (R7 A4-4–A4-6)**: the mutants
  are the six rejected rows of the re-measured table (the `match` mutant indented), and the gaps are its seven
  rows accepted on 3.15.2 and rejected on 3.9.6 — parenthesised context managers leave the list, PEP 646 (two
  forms), `a[x := 1]` and the PEP 798 dict form join it.
- **C25 — PR-2's edit set gains P2 (decision 205 Q2)**: PR-2 §0 / §4 / §8 list
  `docs/plans/2026-10-vm-p4-memo-seam-split.md`
  `:333` and `:799` with the rewrite constraint of §5.3 (b) P2 (each line stays true as a record: `:799` →
  `mise run tools-test`; `:333` → names the untracked `r3/guard-controls.sh`); PR-2's *Depends on* gains the VM P4
  lane's consent from the one request of §9 step 3; E4 runs R-b over the rewritten lines (green); PR-2 §11's
  hand-off list loses #506; its landing report lists the two edits (§9 step 6).
- **C26 — PR-1's edit set gains V16–V18 (decision 205 Q1, Q3)**: PR-1 §4.6 / §8 list
  `docs/plans/2026-10-vm-p4-memo-seam-split.md:356-357` (V16) and `:804-806` (V17), and #510's memo
  `:1097-1108` (V18), each amended in place with a dated marker as §5.2 tables them; PR-1 §4.6's classes gain
  "a ledger claim tied to an event that has happened" (stage 1b) with its discovery command; the consent covers
  them (§9 step 3). **Decision 211 D7-1 (a)**: the three sibling lines of #506's slot-ledger memo are V19–V21
  (`docs/plans/2026-07-vm-p4-slot-ledger.md:58-60`, `:322`, `:323`), amended in V18's commit the same way.
  **Draft 8**: V18's and V19's replacement texts as §5.2 now tables them (#510's landing as V18's origin, R7 J3;
  "listed", not "registered"); **V23** (`:327`, PM decision 213) joins the same commit.
- **C27 — PR-1's refusal text (R6 M2, M6)**: C4 / C16 (b) gain the no-mise branch's remedy (install mise, then
  `mise install`, or an interpreter of the pin's version — never `mise install` alone); "the two commands" =
  `mise -C <root> ls --current --json python` and `mise -C <root> which python3`, verbatim; the `mise which`
  branch asks `mise env --json` and picks one cause and its remedy (§4.2); the "resolved ≠ pin" branch's remedy
  is "remove the winning input", never "write the pin into it". Fixtures (C16 (e)): a fake `mise` whose `env
  --json` lists the running interpreter's directory before the install's (cause: `_.path`) and one that does not
  (cause: PATH order); a no-mise fixture asserting the remedy text names installing mise. **Draft 8 withdraws
  the `mise env --json` read, both cause fixtures and "never write the pin into it"** (PM decision 217; C31); "the
  two commands" and the no-mise fixture stay, the latter asserting the generic block's first action.
- **C28 — I7's helper home and the child-process rule (R6 M3, M5, M7)**: PR-1 §4's edit set names
  `scripts/python_pin_testing.py` (not a set member, not a T5 member) as the helper's file; PR-1 §6 states that
  every fixture reaching the oracle or the warning runs `python_pin.py` as a child under the helper (a test-only
  launcher replaces `sys.version_info` in the child), in-process tests only for pure functions; pin (b) compares
  `os.path.realpath` on both sides. PR-2 §6 loads the same module by path for E1 / E3–E5.
- **C29 — PR-0's plan, T5's home (R6 M8)**: PR-0's plan records why T5 lives in `scripts/python_suites_readers.py`
  (a two-file text reader today) or names its own module, and, on the offline-regime branch that takes the file
  list from the driver, states that the check then reads launcher-passed data and how a control covers that.
- **C30 — PR-1's edit set gains V22 (decision 211 D7-2)**: PR-1 §4.6's classes gain "a call the pin deprecates"
  with §5.2 stage 1c / 2c as its discovery; §8 lists `plan_memo_sibling.py:158` → `maxsplit=1` and the two
  mutation rows' find / replace (`plan_memo_selftest_mutants_codex0927.py:51-52, 56-57`) in one commit with the
  regenerated manifest (two MUTANT lines); the consent request offers the VM P4 lane the fix itself, and if it
  takes it, V22 leaves PR-1's edit set and PR-1's re-derivation records that.
- **C31 — PR-1's refusal is facts plus one generic block (PM decision 217)**: PR-1 §4.2 (b) / C4 / C16 (b) /
  C27 are restated to body §4.2 *The refusal*: items 1–5 (the pin and its file, the version found,
  `sys.executable` as reported, the cwd and the root, mise's answer per element with `install_path` and `mise
  which python3`'s result — or "no answer" with mise's first three stderr lines —, the warning lines), then the
  fixed generic block of four actions and the oracle's two commands; no cause line, no remedy chosen. Pins: every
  oracle fixture asserts its fact lines and that the generic block is byte-identical to the module's one
  constant; the fake `mise` records its argv, and a control asserts it was asked exactly `ls --current --json
  python` and `which python3` (a mutant adding `env --json` is red); the dead-end check of R§4.2 *Each case's
  fix* is a table in PR-1's plan with one fixture per row. PR-1 §2's I5 × I6 row follows body §2.
- **C32 — PR-1's VM-P4 checker run (R7 J1)**: PR-1 §8 / §12 add, after the V19–V21 / V23 edits, `python3
  .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md` on the pin,
  exit 0 expected (CLAUDE.md:55), its output in the PR body; a 1 or 2 is fixed in PR-1's edit. The guard runs as a
  suite member regardless.
- **C33 — no X1-shaped line in a landing file (R7 J5)**: each PR runs body §11's X1 pattern over its own landing
  files (this umbrella's four with PR-1, each per-PR plan with its PR) and expects 0 hits; PR-2 §4.2's R-b count at
  its base is three (C6's and P2's two). The per-PR plans' own quotes of those lines (if any) are described
  instead.
- **C34 — I7's helper shape (R7 A1-1, A2-6)**: PR-1 §4 / §6 state that `scripts/python_pin_testing.py` defines no
  `TestCase` and no `__main__` guard, that tests load it through the one constant `"scripts/python_pin_testing.py"`
  (PR-2's named form), that the child launcher is a `-c` source string in it, and that the child's environment is
  `os.environ` minus `MISE_*` plus the five variables and `PATH`, regime keys untouched, no `-I` / `-E` / `-S`,
  with a control comparing the regime keys. PR-2 §4.2's E3 gains a green case for that by-constant load.
- **C35 — the deprecation property, measured (R7 A3-3)**: PR-1 runs the set on the pin at its base with
  `PYTHONWARNINGS=always::DeprecationWarning` and records the count of `DeprecationWarning` lines by source line
  in its PR body (at `ee6c7a1a`: all at `plan_memo_sibling.py:158`); after V22 the expected count is 0. Stage 1c
  stays as the lower bound that names candidates.
- **C36 — the untrusted case on `main` (R7 A4-7)**: PR-1 §12's landing step and its plan's `[settings]` text say
  that `main` is already untrusted-refusing (`[bootstrap.packages]`, #535) and that PR-1's `[settings]` adds a
  second such table — not that PR-1 introduces the case.

---

## §14 Provenance

- Draft 1 (2026-10-26): single-PR plan memo `python-pin-plan.md` (scratch), reviewed by plan-review R1.
- Draft 2 (2026-10-26): PM decision 41; umbrella + three per-PR plans; measurements as in its §4.1 / §7.2.
  Reviewed by plan-review R2 (2026-10-31, against `b051329b`): CRIT 0, 9 IMP, ~12 MIN (`r2-disposition.md`).
- Draft 3 (2026-10-31): PM decision 117; base `b051329b`; pin 3.15.2; PR-3 folded into PR-1; §5.2 VM P4 table;
  §12.2 R2 disposition; §13 per-PR consequences. Measurements: §4.1 rows 1a–3 by the driver (2026-10-31, mise
  2026.10.21, isolated data dir); row 3x, §4.1's listing and compile-off rows and §4.2's precedence rows by the
  designer (same day, same mise, scratch dirs). Kept as `umbrella.draft3.md`. Reviewed by plan-review R3
  (2026-10-31, against `74e15df1`): CRIT 0, 6 IMP, ~16 MIN (`r3-disposition.md`).
- Draft 4 (2026-10-31): PM decisions 124 and 127; base `74e15df1`; PR-0 split off (G3); T5 by property (G4);
  `python.github_attestations` (G5); §5.3 PR-1 × #506 overlap table; §12.3 R3 disposition; §13 C9–C14.
  Measurements by the designer, 2026-10-31, mise 2026.10.21, scratch dirs with empty global config and fresh data
  dirs: §4.2's config-path probe (27 candidates, `mise config ls`, with and without `MISE_ENV=ci`), the idiomatic
  probe (`MISE_IDIOMATIC_VERSION_FILE_ENABLE_TOOLS=python`), `mise config explain` on `main`'s `mise.toml`, the
  `python.github_attestations` key and its two env overrides (two `mise install python@3.15.2`), and §4.1's
  `ls-remote` comparison. The split's lines read from the VM P4 lane's local commits `707dae15` / `c032379d`
  through the shared object store (read-only).
  Reviewed by plan-review R4 (2026-10-31, against `1ed126b4`): CRIT 0, 6 IMP, ~18 MIN (`r4-disposition.md`).
  Kept as `umbrella.draft4.md`.
- Draft 5 (2026-10-31): PM decisions 141, 142 and 150; base `1ed126b4`. T5 rebuilt (decision 150 (1)): T5a /
  T5b retired; the CI rule (§4.2 clauses 1–5) and the local oracle (`mise ls --current --json python`, `mise
  which python3`); PR-0 rebuilt as T5's clauses 2–5; I5 split (H4); the pin file one line at one fixed location
  (H2, H3); the supported path by what runs (H6); the override warning (decision 150 (4)); §6 ordering of the
  cargo-deny PR, the M4-12 program and sweep 4 (decision 150 (3)); §12.4 R4 disposition; §13 C15–C20.
  Measurements by the designer, 2026-10-31, mise 2026.10.21, every mise dir isolated under the scratchpad, an
  empty global config, trust by `MISE_TRUSTED_CONFIG_PATHS` only, offline: the oracle's five cases and the
  installed-shape case (§4.2 table); `which -a python3` in the CI/tools session (§7.1); the name-predicate and
  `python`-occurrence census of `main`'s tree and `mise.toml` (§4.2 clauses 2 and 4); the split's launch lines
  (§5.3 P1–P2). The user's `~/.local/state/mise/trusted-configs` and `tracked-configs` listings were
  identical before and after. Reviewed by plan-review R5 (2026-10-31, against `1ed126b4`): CRIT 0, 7 IMP
  (T1–T3 sent to the PM unfixed, J1–J4), MINs per axis (`r5-disposition.md`). Kept as `umbrella.draft5.md`.
- Draft 6 (2026-10-31): PM decisions 154, 167, 178 and user decision 170; base `6aea28da` (§0). The guarantee
  narrowed to "never silent when the version differs" (decision 170; §1, §2, §4.2, §6, §7.1, §11 T1 / T2, C22),
  CI rule unchanged; **I7** hermetic refusal tests (decision 167; §2, C23); H6's two names in every refusal;
  §2 *Every pair × half* (J1); the grammar pin (J2, C24); J3's departure stated (§0); #506 pushed (J4); R5's MINs
  (§12.5); the split at landing (§6). Measurements by the designer, 2026-10-31, mise 2026.10.21, isolated as §0
  (plus offline, auto-install off, fake installs): §4.2's added oracle rows (`[]`, two elements, T1, T2, untrusted),
  the benign pair, §11's T1 / T2 recipes run verbatim from this memo's text, and the `ast` `feature_version`
  behaviour (on the local 3.14.7). The listing of the user's `~/.local/state/mise` was identical before and after.
  Reviewed by plan-review R6 (2026-11-01, against `ee6c7a1a`): CRIT 0, IMP 3, MIN 14, FP 3 (`r6-disposition.md`,
  §12.6). Kept as `umbrella.draft6.md` (SHA-256 `8e779d8b…`).
- Draft 7 (2026-11-01): PM decision 205; base `ee6c7a1a` (#506 landed as `3ec0ae5c`). §5.3 restated as the landed
  state (K1); V16–V18 (§5.2); P2 in PR-2's edit set (K2); the seam set decided — four landing files, the body
  978 lines, VM P4 153, records 412, rounds 462, total 2005 (`wc -l` / `awk` per §6) (K3); M1–M14 (§12.6);
  §13 C25–C29. Restructured from draft 6 by moving blocks between files unchanged (a script over draft 6's line
  ranges), then edited. Measurements by the designer, 2026-11-01 (R§0): the suite census and `.py` count at
  `ee6c7a1a`; the 89 files compiled under `-W error` on Homebrew 3.15.2 and macOS 3.9.6; #506's guard alone on both
  (and its 3.15.2 `DeprecationWarning` under `-W error`); the J2 `feature_version` probe on 3.15.2 (R§11); one mise
  probe (2026.10.25, isolated, offline, fake install) for `mise env --json` under T1 (R§11 *M6 read*); the ledger
  row counts (V18, stage 1b); the `mkdtemp` / `realpath` forms (§2 I7). No `~/.local/state/mise` entry resolves
  into the scratch dir afterwards; no before/after listing was taken. No cargo build; no repository write.
- Draft 7 fold (2026-11-01): PM decision 211, in place (backup `umbrella.draft7.md`, SHA-256 `4b8b794c…`): V19–V22
  (§5.2, stages 1c / 2c), D7-3 (§6), §12.6 fold, §13 C26 / C30; body 984 lines, VM P4 160, records 414, rounds 478,
  total 2036 (§6's commands). Measured by the designer at `ee6c7a1a` (read-only): the slot ledger's four stage-1b
  lines, both slots' pointer rows, V22's five copies and the 17 `re` calls, `re.split`'s warning and keyword form on
  3.15.2 / 3.9.6. No mise, no cargo, no repository write.
  Reviewed by plan-review R7 (2026-11-01, against `ee6c7a1a`): CRIT 0, IMP 6, MIN 19, FP 5 (`r7-disposition.md`,
  §12.7). Kept as `umbrella.draft7-fold.md` (SHA-256 `f7d75b2b…`).
- Draft 8 (2026-11-01): PM decisions 213 and 217; base `ee6c7a1a` (`main` at `593aa314` since, #579, crates only;
  §0). In place over the draft-7 fold. The refusal's cause layer removed (decision 217; §2 I5, §4.2 *The
  refusal*, §7.1, §11 T1 / T2, R§4.2 *Each case's fix*, R§11 *M6 read* withdrawn); J1–J5 and the MINs (§12.7),
  J6 / A2-3 / A2-4 / A2-5 vanished with the layer; V23 (§5.2); §5.3 O4 / O7 / O9; §13 C31–C36 and amendments to
  C4, C6, C14, C16, C20, C24, C26, C27. Counts (§6's commands): body 999 lines, VM P4 167, records 493, rounds 585,
  total 2244. Measurements by the designer, 2026-11-01 (R§0 *Draft 8's measurements*): base drift; the ledger rows
  and landing dates for J3; V23 at `ee6c7a1a`; the guard's import closure; the VM-P4 checker (rc 0 on 3.15.2 and
  3.9.6); the suite set on 3.15.2 with deprecations shown; the X1 pattern over this memo; the mise lookups and
  `sys.executable` for J4; three isolated mise 2026.10.25 probes (A4-7 and the post-check); the J2 table on both
  interpreters. The user's `~/.local/state/mise/tracked-configs` listing had 144 lines before and after; no entry
  points into `scratchpad/d8`. No cargo build; no repository write.
