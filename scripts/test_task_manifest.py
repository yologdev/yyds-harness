#!/usr/bin/env python3
"""Tests for scripts/task_manifest.py."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import task_manifest  # noqa: E402


class TaskManifest(unittest.TestCase):
    def test_manifest_captures_rich_task_decisions(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nState feedback found evaluator timeout.\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: Improve evaluator timeout evidence
Files: scripts/log_feedback.py, scripts/build_evolution_dashboard.py
Issue: none
Origin: planner

Objective:
Make evaluator timeout visible in gnome metrics and dashboard task evidence.

Why this matters:
DeepSeek coding tasks need independent evaluator evidence, not just build/test success.

Success Criteria:
- evaluator timeout is counted separately
- dashboard shows unverified evaluator status

Verification:
- python3 scripts/log_feedback.py --test

Expected Evidence:
- task_verification_rate drops when evaluator times out
""",
                encoding="utf-8",
            )
            args = type(
                "Args",
                (),
                {
                    "session_plan_dir": plan,
                    "assessment_file": plan / "assessment.md",
                    "issue_responses_file": plan / "issue_responses.md",
                    "planning_failure_file": plan / "planning_failure.md",
                    "selected_limit": 3,
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            payload = task_manifest.decision_payload(manifest)

            self.assertFalse(manifest["planner"]["planning_failed"])
            self.assertEqual(manifest["planner"]["selected_task_count"], 1)
            self.assertEqual(manifest["selected_tasks"][0]["task_id"], "task_01")
            self.assertTrue(manifest["selected_tasks"][0]["quality"]["has_expected_evidence"])
            self.assertEqual(
                manifest["selected_tasks"][0]["expected_evidence"],
                "task_verification_rate drops when evaluator times out",
            )
            self.assertEqual(manifest["selected_tasks"][0]["quality"]["score"], 1.0)
            self.assertEqual(payload["tasks"][0]["task_title"], "Improve evaluator timeout evidence")
            self.assertEqual(payload["tasks"][0]["planned_files"], ["scripts/log_feedback.py", "scripts/build_evolution_dashboard.py"])
            self.assertEqual(
                payload["tasks"][0]["expected_evidence"],
                "task_verification_rate drops when evaluator times out",
            )

    def test_manifest_parses_blank_separated_task_header_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "task_01.md").write_text(
                """Title: Capture panic diagnostics

Files: src/state.rs

Issue: none

Origin: planner

Objective:
Make panic details visible in RunCompleted error_detail.

Success Criteria:
- panic diagnostics are stashed

Verification:
- cargo test panic_hook
""",
                encoding="utf-8",
            )
            args = type(
                "Args",
                (),
                {
                    "session_plan_dir": plan,
                    "assessment_file": plan / "assessment.md",
                    "issue_responses_file": plan / "issue_responses.md",
                    "planning_failure_file": plan / "planning_failure.md",
                    "selected_limit": 3,
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            payload = task_manifest.decision_payload(manifest)

            self.assertEqual(manifest["selected_tasks"][0]["files"], ["src/state.rs"])
            self.assertEqual(payload["tasks"][0]["planned_files"], ["src/state.rs"])

    def test_manifest_normalizes_planned_file_annotations(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "task_01.md").write_text(
                """Title: Extract diagnostics module
Files: src/commands_state.rs, src/commands_state_diagnostics.rs (new), `src/lib.rs` - module registration
Issue: none
Origin: planner

Objective:
Move diagnostics into a focused module.

Success Criteria:
- module compiles

Verification:
- cargo test commands_state
""",
                encoding="utf-8",
            )
            args = type(
                "Args",
                (),
                {
                    "session_plan_dir": plan,
                    "assessment_file": plan / "assessment.md",
                    "issue_responses_file": plan / "issue_responses.md",
                    "planning_failure_file": plan / "planning_failure.md",
                    "selected_limit": 3,
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            payload = task_manifest.decision_payload(manifest)

            self.assertEqual(
                manifest["selected_tasks"][0]["files"],
                ["src/commands_state.rs", "src/commands_state_diagnostics.rs", "src/lib.rs"],
            )
            self.assertEqual(
                payload["tasks"][0]["planned_files"],
                ["src/commands_state.rs", "src/commands_state_diagnostics.rs", "src/lib.rs"],
            )

    def test_manifest_parses_markdown_heading_before_fields(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "task_01.md").write_text(
                """# Task 01: Wire crash reporter into pre-agent bootstrap path

Title: Wire crash reporter into pre-agent bootstrap path
Files: src/lib.rs
Issue: none
Origin: planner

## Objective
Install the crash reporter before context loading can fail.

Success Criteria:
- startup failures are recorded

Verification:
- cargo test crash_reporter
""",
                encoding="utf-8",
            )
            args = type(
                "Args",
                (),
                {
                    "session_plan_dir": plan,
                    "assessment_file": plan / "assessment.md",
                    "issue_responses_file": plan / "issue_responses.md",
                    "planning_failure_file": plan / "planning_failure.md",
                    "selected_limit": 3,
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            task = manifest["selected_tasks"][0]

            self.assertEqual(task["title"], "Wire crash reporter into pre-agent bootstrap path")
            self.assertEqual(task["files"], ["src/lib.rs"])
            self.assertTrue(task["quality"]["has_goal"])
            self.assertFalse(task["quality"]["has_expected_evidence"])
            self.assertIn("task_01:missing_expected_evidence", manifest["warnings"])
            self.assertNotIn("task_01:missing_files", manifest["warnings"])

    def test_manifest_records_planning_failure_without_fake_task(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "planning_failure.md").write_text("planner produced no tasks\n", encoding="utf-8")
            args = type(
                "Args",
                (),
                {
                    "session_plan_dir": plan,
                    "assessment_file": plan / "assessment.md",
                    "issue_responses_file": plan / "issue_responses.md",
                    "planning_failure_file": plan / "planning_failure.md",
                    "selected_limit": 3,
                    "planning_failed": True,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            payload = task_manifest.decision_payload(manifest)

            self.assertTrue(manifest["planner"]["planning_failed"])
            self.assertEqual(manifest["tasks"], [])
            self.assertIn("planner_produced_no_task_files", manifest["warnings"])
            self.assertEqual(payload["selected_task_count"], 0)
            self.assertTrue(payload["planning_failed"])
            self.assertEqual(payload["decision"], "planning_failed")

    def test_manifest_flags_assessment_contradicted_seed_task(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text(
                """# Assessment

## Self-Test Results
- `yyds state why last-failure` -> no failure found (healthy state), shows diagnostic guidance
""",
                encoding="utf-8",
            )
            (plan / "task_01.md").write_text(
                """Title: Improve cold-start state failure diagnostics
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: harness-seed

Objective:
Make `yyds state why last-failure` useful when there are no completed failed sessions yet.

Why this matters:
The assessment found `state why last-failure` returning only `no state event found` during fresh-state sessions.

Success Criteria:
- output gives actionable diagnostics

Verification:
- cargo test commands_state state
""",
                encoding="utf-8",
            )
            args = type(
                "Args",
                (),
                {
                    "session_plan_dir": plan,
                    "assessment_file": plan / "assessment.md",
                    "issue_responses_file": plan / "issue_responses.md",
                    "planning_failure_file": plan / "planning_failure.md",
                    "selected_limit": 3,
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            quality = manifest["selected_tasks"][0]["quality"]

            self.assertIn("task_01:assessment_contradiction", manifest["warnings"])
            self.assertLess(quality["score"], 0.75)
            self.assertTrue(quality["assessment_alignment"]["contradicted_by_assessment"])

    def test_manifest_ignores_obsolete_note_files_as_tasks(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nSeed was stale.\n", encoding="utf-8")
            (plan / "task_01_obsolete.md").write_text(
                "This is an obsolete-note artifact, not an implementation task.\n",
                encoding="utf-8",
            )
            args = type(
                "Args",
                (),
                {
                    "session_plan_dir": plan,
                    "assessment_file": plan / "assessment.md",
                    "issue_responses_file": plan / "issue_responses.md",
                    "planning_failure_file": plan / "planning_failure.md",
                    "selected_limit": 3,
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            self.assertEqual(manifest["tasks"], [])
            self.assertTrue(manifest["planner"]["planning_failed"])

    def test_manifest_records_missing_assessment_diagnostic(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment_missing.md").write_text("assessment agent wrote no artifact\n", encoding="utf-8")
            (plan / "planning_failure.md").write_text("planner produced no tasks\n", encoding="utf-8")
            args = type(
                "Args",
                (),
                {
                    "session_plan_dir": plan,
                    "assessment_file": plan / "assessment.md",
                    "assessment_missing_file": plan / "assessment_missing.md",
                    "issue_responses_file": plan / "issue_responses.md",
                    "planning_failure_file": plan / "planning_failure.md",
                    "selected_limit": 3,
                    "planning_failed": True,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            self.assertFalse(manifest["planner"]["assessment_present"])
            self.assertTrue(manifest["planner"]["assessment_missing_present"])
            self.assertIsNone(manifest["artifacts"]["assessment"])
            self.assertEqual(manifest["artifacts"]["assessment_missing"], "tasks/assessment_missing.md")

    def test_write_task_decisions_creates_per_task_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = {
                "selected_tasks": [
                    {
                        "task_id": "task_01",
                        "task_number": 1,
                        "title": "Keep evidence",
                        "files": ["scripts/evolve.sh"],
                    }
                ]
            }
            out = root / "tasks" / "manifest.json"
            out.parent.mkdir()
            task_manifest.write_task_decisions(manifest, out)
            decision = json.loads((root / "tasks/task_01/decision.json").read_text(encoding="utf-8"))
            self.assertEqual(decision["title"], "Keep evidence")


if __name__ == "__main__":
    unittest.main()
