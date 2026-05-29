#!/usr/bin/env python3
"""Unit tests for grep_pass — Check 1/2/3 + exemption patterns + overrides.

Run via:
    python3 .claude/skills/elidex-plan-review/test_grep_pass.py

Each test sets up a temp repo (with a mock `crates/` tree) + a temp
plan-memo, invokes `run_grep_pass`, and asserts the (severity, message)
findings. No external dependencies — stdlib `unittest` + `tempfile`.
"""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from grep_pass import run_grep_pass  # noqa: E402


def _mk_repo(tmp: Path, files: dict[str, str]) -> Path:
    """Create a fake repo at `tmp` with the given `path → content` files.

    Paths are relative to `tmp/`. Parent dirs are auto-created.
    """
    for rel, content in files.items():
        p = tmp / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content, encoding="utf-8")
    return tmp


def _mk_plan(tmp: Path, content: str) -> Path:
    """Write a plan-memo to `tmp/plan.md`."""
    p = tmp / "plan.md"
    p.write_text(content, encoding="utf-8")
    return p


class Check1FilePathTests(unittest.TestCase):
    """Check 1 — file path existence (HARD by default)."""

    def test_existing_path_no_finding(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/script/foo.rs": "fn x() {}\n"})
            plan = _mk_plan(tmp, "See `crates/script/foo.rs` for details.\n")
            findings = run_grep_pass(plan, tmp)
            file_findings = [f for f in findings if "crates/script/foo.rs" in f[1]]
            self.assertEqual(file_findings, [])

    def test_missing_path_hard_fail(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})  # crates/ does not exist
            plan = _mk_plan(tmp, "See `crates/script/missing.rs` for details.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertTrue(any(
                sev == "HARD" and msg.startswith(
                    "path crates/script/missing.rs does not exist")
                for sev, msg in findings
            ), f"expected HARD missing-path, got {findings}")

    def test_line_in_range_no_finding(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "a\nb\nc\nd\ne\n"})  # 5 lines
            plan = _mk_plan(tmp, "See `crates/foo.rs:3`.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(findings, [])

    def test_line_out_of_range_soft_warn(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "a\nb\nc\n"})  # 3 lines
            plan = _mk_plan(tmp, "See `crates/foo.rs:99`.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertTrue(any(
                sev == "SOFT" and "crates/foo.rs:99 out of range" in msg
                for sev, msg in findings
            ), f"expected SOFT out-of-range, got {findings}")

    def test_path_inside_code_block_skipped(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp, "```rust\n// crates/script/foo.rs example\n```\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "crates/script/foo.rs" in f[1]],
                [],
                "paths in fenced code blocks should be skipped",
            )

    def test_brace_expansion_soft_warn_not_hard(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp,
                "Touch `crates/script/elidex-js/src/vm/host/wasm/"
                "{module,instance,memory}.rs`.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertFalse(
                any(sev == "HARD" for sev, _ in findings),
                f"brace-expansion should not hard-fail, got {findings}",
            )
            self.assertTrue(any(
                sev == "SOFT" and "shell glob/brace" in msg
                for sev, msg in findings
            ), f"expected SOFT shell-syntax warn, got {findings}")

    def test_glob_soft_warn_not_hard(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp,
                "Touch all `crates/script/elidex-js/src/vm/host/wasm/*.rs` modules.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertFalse(
                any(sev == "HARD" for sev, _ in findings),
                f"glob path should not hard-fail, got {findings}",
            )

    def test_missing_path_NEW_annotation_exempts(self):
        # Planned new files should not hard-fail Check 1 when annotated.
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp,
                "Stage 4: create `crates/script/elidex-js/src/vm/wasm_payload.rs` "
                "(NEW) for the payload definitions.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "wasm_payload.rs" in f[1]],
                [],
                "(NEW) annotation should exempt Check 1 hard-fail",
            )

    def test_missing_path_NEW_annotation_anywhere_exempts(self):
        # Author intent for (NEW) is global — annotation on later mention
        # must exempt the path even if earlier mentions are bare. PR #243
        # Copilot R2 IMP (same bug shape as Check 2, audit hit).
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp,
                "Stage 1: prose mentions `crates/script/elidex-js/src/vm/"
                "wasm_payload.rs` casually before the formal declaration.\n\n"
                "## §4. Architecture\n\n"
                "Create `crates/script/elidex-js/src/vm/wasm_payload.rs` "
                "(NEW) for payloads.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "wasm_payload.rs" in f[1]], [],
                "(NEW) on later mention should exempt path globally",
            )


class Check2RustSymbolTests(unittest.TestCase):
    """Check 2 — Rust symbol grep + NEW-annotation exemption."""

    def test_symbol_found_no_finding(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {
                "crates/foo.rs": "impl X { fn do_thing() {} }\nuse X::do_thing;\n",
            })
            plan = _mk_plan(tmp, "Stage 1: invoke `X::do_thing` from caller.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "X::do_thing" in f[1]],
                [],
            )

    def test_symbol_missing_soft_warn_default(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp, "Stage 1: invoke `X::do_thing` from caller.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertTrue(any(
                sev == "SOFT" and "X::do_thing" in msg and "not found" in msg
                for sev, msg in findings
            ), f"expected SOFT not-found, got {findings}")

    def test_symbol_missing_hard_fail_with_strict(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp, "Stage 1: invoke `X::do_thing` from caller.\n")
            findings = run_grep_pass(plan, tmp, strict_symbols=True)
            self.assertTrue(any(
                sev == "HARD" and "X::do_thing" in msg
                for sev, msg in findings
            ), f"expected HARD with --strict-symbols, got {findings}")

    def test_NEW_annotation_exempts(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "Stage 1: add `X::do_thing` (NEW) to wire up the dispatcher.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "X::do_thing" in f[1]],
                [],
                "(NEW) annotation in vicinity should exempt the symbol",
            )

    def test_planned_annotation_exempts(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "Stage 1: add `X::do_thing` (planned) to the API.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "X::do_thing" in f[1]], [])

    def test_pr_surface_annotation_exempts(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "Stage 1: `X::do_thing` (F3 surface) is the new entry point.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "X::do_thing" in f[1]], [])

    def test_NEW_annotation_anywhere_exempts(self):
        # Author intent for (NEW) is global — annotation on later mention
        # must exempt the symbol even if earlier mentions are bare.
        # Original logic was first-occurrence-only and got the order
        # backwards in this case (PR #243 Copilot R2 IMP).
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "Stage 1: prose mentions `X::do_thing` casually.\n\n"
                "## §4. Architecture\n\n"
                "Add `X::do_thing` (NEW) as the dispatcher entry.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "X::do_thing" in f[1]], [],
                "(NEW) on later mention should exempt symbol globally",
            )

    def test_symbol_in_code_block_skipped(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "```rust\nlet _ = `X::do_thing`(arg);\n```\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "X::do_thing" in f[1]],
                [],
                "symbols inside fenced code blocks should be skipped",
            )

    def test_symbol_in_tilde_fence_skipped(self):
        # ~~~ fences are GFM-valid. Mirrored from preflight.py's FENCE_RE
        # (PR #243 Copilot R1 MIN).
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "~~~rust\nlet _ = `X::do_thing`(arg);\n~~~\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "X::do_thing" in f[1]],
                [],
                "symbols inside ~~~-fenced blocks should be skipped",
            )

    def test_unclosed_fence_extends_to_eof(self):
        # Unclosed fence at EOF must extend span to len(content) so
        # subsequent content stays treated as fenced (PR #243 Copilot R1 MIN).
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "Some text.\n```\n// unclosed fence — `X::do_thing` should be skipped\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "X::do_thing" in f[1]], [],
                "unclosed fence should extend span to EOF",
            )

    def test_symbol_in_coverage_map_skipped(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "## §3. Spec coverage map\n\n"
                "| Spec section | Step | Branch | Touch | Full enum? | User-input flow |\n"
                "|---|---|---|---|---|---|\n"
                "| ECMA-262 §15.7.14 ClassDefinitionEvaluation | step 6.f | (ii) "
                "non-constructor | `Op::AssertConstructor` | yes | yes |\n\n"
                "## §4. Architecture\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "Op::AssertConstructor" in f[1]],
                [],
                "symbols inside §3 coverage map should be skipped",
            )

    def test_coverage_map_subsection_preserved(self):
        # SKILL.md template mandates `### §3.1 User-input touch audit` inside
        # §3. Coverage-map span must include subheadings, not terminate at
        # the first `### §3.x` (PR #243 Copilot R1 IMP).
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "## §3. Spec coverage map\n\n"
                "| Spec | Step |\n|---|---|\n| ECMA-262 §15 | step 1 |\n\n"
                "### §3.1 User-input touch audit\n\n"
                "- `Op::SuperCall`: user-controlled heritage (citation, not a real symbol)\n\n"
                "## §4. Architecture\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "Op::SuperCall" in f[1]], [],
                "symbol inside §3.1 (within §3 span) should be skipped",
            )

    def test_coverage_map_terminates_at_same_level_heading(self):
        # Span ends at next heading at same-or-shallower level regardless of
        # § prefix — original NEXT_HEADING_RE required §, so a non-§ `##`
        # heading didn't terminate, silently exempting later symbols (PR
        # #243 Copilot R1 IMP, consequence #2).
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "## §3. Spec coverage map\n\n"
                "| Spec | Step |\n|---|---|\n| ECMA-262 §15 | step 1 |\n\n"
                "## Implementation notes\n\n"
                "- `Y::missing_method` is referenced here but not in repo\n")
            findings = run_grep_pass(plan, tmp)
            self.assertTrue(
                any("Y::missing_method" in msg for _sev, msg in findings),
                f"symbol after `## Implementation notes` should be Check 2'd, "
                f"got {findings}",
            )

    def test_single_token_skipped(self):
        # `foo` alone (no `::`) is too noisy to match; skip via pattern.
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp, "Refer to `do_thing` from the API.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "do_thing" in f[1]], [],
                "single tokens (no ::) should not be checked",
            )

    def test_duplicate_symbol_grepped_once(self):
        # Repeated symbol mention shouldn't generate duplicate findings.
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn other() {}\n"})
            plan = _mk_plan(tmp,
                "First: `X::do_thing` and later: `X::do_thing` again.\n")
            findings = run_grep_pass(plan, tmp)
            xdo = [f for f in findings if "X::do_thing" in f[1]]
            self.assertEqual(len(xdo), 1,
                f"expected 1 finding for duplicate symbol, got {xdo}")


class Check3EnumerationClaimTests(unittest.TestCase):
    """Check 3 — enumeration claim verification artifact."""

    def test_claim_with_grep_artifact_no_finding(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp,
                "22 callers (verified via `grep -rn pattern crates/`).\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "22 callers" in f[1]], [])

    def test_claim_with_verified_date_no_finding(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp,
                "22 callers, verified 2026-05-29.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "22 callers" in f[1]], [])

    def test_claim_with_line_list_no_finding(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp,
                "22 callers at lines 111/180/227/324/360.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "22 callers" in f[1]], [])

    def test_claim_without_artifact_soft_warn(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp, "Stage 3.5: handle 30+ TypedArray methods.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertTrue(any(
                sev == "SOFT" and "30+ TypedArray methods" in msg
                for sev, msg in findings
            ), f"expected SOFT enum-no-artifact, got {findings}")

    def test_claim_without_artifact_hard_fail_with_strict(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp, "Stage 6: 8 BufferSource consumer modules.\n")
            findings = run_grep_pass(plan, tmp, strict_enum=True)
            self.assertTrue(any(
                sev == "HARD" and "8 BufferSource consumer modules" in msg
                for sev, msg in findings
            ), f"expected HARD with --strict-enum, got {findings}")

    def test_claim_in_code_block_skipped(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp,
                "```\n# touches 30 sites\n```\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(
                [f for f in findings if "30 sites" in f[1]], [],
                "enum claims inside fenced code blocks should be skipped",
            )

    def test_units_case_insensitive(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {})
            plan = _mk_plan(tmp, "Stage X: 12 Callers across the dispatcher.\n")
            findings = run_grep_pass(plan, tmp)
            self.assertTrue(any(
                "12 Callers" in msg or "12 callers" in msg.lower()
                for _sev, msg in findings
            ), f"expected case-insensitive match, got {findings}")


class IntegrationTests(unittest.TestCase):
    """Cross-check combinations + override flag interactions."""

    def test_no_findings_clean_plan(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {
                "crates/foo.rs": "impl X { fn do_thing() {} }\n"
                                  "use X::do_thing;\n",
            })
            plan = _mk_plan(tmp,
                "Stage 1: Touch `crates/foo.rs`. Invoke `X::do_thing` from caller.\n"
                "Caller count: 1 (verified via `grep -rn X::do_thing crates/`).\n")
            findings = run_grep_pass(plan, tmp)
            self.assertEqual(findings, [])

    def test_combined_drift_classes(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            _mk_repo(tmp, {"crates/foo.rs": "fn unrelated() {}\n"})
            plan = _mk_plan(tmp,
                "Stage 1: edit `crates/script/missing.rs:42` and call "
                "`Y::nonexistent`. Total 5 callers affected.\n")
            findings = run_grep_pass(plan, tmp)
            severities = {sev for sev, _ in findings}
            self.assertIn("HARD", severities, "missing file should yield HARD")
            self.assertIn("SOFT", severities,
                "missing symbol + bare enum should yield SOFT")
            msgs = " | ".join(m for _, m in findings)
            self.assertIn("crates/script/missing.rs does not exist", msgs)
            self.assertIn("Y::nonexistent", msgs)
            self.assertIn("5 callers", msgs)


if __name__ == "__main__":
    unittest.main()
