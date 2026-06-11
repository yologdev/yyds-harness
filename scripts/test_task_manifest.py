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
            self.assertEqual(manifest["selected_tasks"][0]["quality"]["score"], 1.0)
            self.assertEqual(payload["tasks"][0]["task_title"], "Improve evaluator timeout evidence")
            self.assertEqual(payload["tasks"][0]["planned_files"], ["scripts/log_feedback.py", "scripts/build_evolution_dashboard.py"])

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
