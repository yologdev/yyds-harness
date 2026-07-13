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
    def test_extract_file_mentions_accepts_sentence_punctuation(self):
        text = (
            "Add focused lifecycle assertions in src/commands_state.rs. "
            "Do not count src/prompt.rs.bak as a source path. "
            "Also inspect scripts/build_evolution_dashboard.py, then README.md."
        )

        self.assertEqual(
            task_manifest.extract_file_mentions(text),
            ["src/commands_state.rs", "scripts/build_evolution_dashboard.py", "README.md"],
        )

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

    def test_manifest_requires_nonempty_expected_evidence_section(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "task_01.md").write_text(
                """Title: Tighten task evidence scoring
Files: scripts/task_manifest.py
Issue: none
Origin: planner

Objective:
Prevent empty evidence sections from looking complete.

Success Criteria:
- empty evidence sections are warned

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
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

            self.assertIsNone(task["expected_evidence"])
            self.assertFalse(task["quality"]["has_expected_evidence"])
            self.assertLess(task["quality"]["score"], 1.0)
            self.assertIn("task_01:missing_expected_evidence", manifest["warnings"])
            self.assertNotIn("task_01:thin_task_spec", manifest["warnings"])

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

    def test_manifest_planning_failure_selects_no_tasks_even_if_task_files_exist(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "planning_failure.md").write_text("planner gave up after analysis\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: Planner task that must not run
Files: scripts/task_manifest.py
Issue: none
Origin: planner

Objective:
This task would be valid only if planning succeeded.

Success Criteria:
- Planning failure does not leak selected tasks.

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- selected_tasks is empty when planning_failed is true.
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

            self.assertTrue(manifest["planner"]["planning_failed"])
            self.assertEqual(manifest["planner"]["task_count"], 1)
            self.assertEqual(manifest["planner"]["selected_task_count"], 0)
            self.assertEqual(manifest["selected_tasks"], [])
            self.assertEqual(payload["decision"], "planning_failed")
            self.assertEqual(payload["tasks"], [])

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
            quality = manifest["tasks"][0]["quality"]

            self.assertIn("task_01:assessment_contradiction", manifest["warnings"])
            self.assertLess(quality["score"], 0.75)
            self.assertTrue(quality["assessment_alignment"]["contradicted_by_assessment"])
            self.assertEqual(manifest["selected_tasks"], [])

    def test_manifest_flags_cold_start_seed_when_no_completed_failure_sessions_is_healthy(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text(
                """# Assessment

## Recent Changes
Day 113 fixed `state why last-failure` messaging: it no longer says `no state event found`.

## Self-Test Results
- `yyds state why last-failure`: correctly reports "No completed failure sessions" + 1 incomplete run
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
            quality = manifest["tasks"][0]["quality"]

            self.assertIn("task_01:assessment_contradiction", manifest["warnings"])
            self.assertTrue(quality["assessment_alignment"]["contradicted_by_assessment"])
            self.assertEqual(manifest["selected_tasks"], [])

    def test_manifest_excludes_file_less_tasks_from_selectable(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nNot much to do.\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: File-less task that should not be selectable
Files:
Issue: #79
Origin: planner

Objective:
This task has an empty Files header and no file mentions in the body.

Success Criteria:
- The task is excluded from selectable.
- missing_files warning is emitted.
- no_selectable_tasks fires when it is the only task.

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- The task gets a missing_files warning and is NOT in selected_tasks.
""",
                encoding="utf-8",
            )
            (plan / "task_02.md").write_text(
                """Title: A real task with files should still be selectable
Files: scripts/task_manifest.py
Issue: none
Origin: planner

Objective:
Make sure the file-less filter does not exclude valid tasks.

Success Criteria:
- This task is selected.

Verification:
- python3 -m unittest scripts.test_task_manifest
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

            # file-less task gets a warning
            self.assertIn("task_01:missing_files", manifest["warnings"])
            # file-less task is excluded from selectable
            selected_ids = [t["task_id"] for t in manifest["selected_tasks"]]
            self.assertNotIn("task_01", selected_ids)
            # task with files is still selected
            self.assertIn("task_02", selected_ids)
            self.assertEqual(len(manifest["selected_tasks"]), 1)

    def test_manifest_no_selectable_when_all_file_less(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nNo real tasks.\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: Another file-less task
Files:
Issue: #79
Origin: planner

Objective:
This task has an empty Files header too.

Success Criteria:
- no_selectable_tasks fires when all tasks are file-less.
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

            self.assertIn("task_01:missing_files", manifest["warnings"])
            self.assertIn("no_selectable_tasks", manifest["warnings"])
            self.assertEqual(manifest["selected_tasks"], [])

    def test_manifest_rejects_recently_failed_analysis_only_seed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text(
                """# Assessment

## Recent session outcomes
- Task 1: Make analysis-only task pressure landable was reverted.

## Bugs / Friction Found
- Make analysis-only task pressure landable timed out without a verifier verdict after no file progress.
""",
                encoding="utf-8",
            )
            (plan / "task_01.md").write_text(
                """Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed (refined by planner)

Objective:
Make analysis-only pressure produce a landable task.

Success Criteria:
- No analysis-only pressure task is selected after it just failed.

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- selected_tasks excludes this stale task.
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
            alignment = manifest["tasks"][0]["quality"]["assessment_alignment"]

            self.assertTrue(alignment["contradicted_by_assessment"], alignment)
            self.assertIn("task_01:assessment_contradiction", manifest["warnings"])
            self.assertIn("no_selectable_tasks", manifest["warnings"])
            self.assertEqual(manifest["selected_tasks"], [])
            self.assertEqual(payload["decision"], "no_selectable_tasks")

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

    def test_manifest_warns_when_all_tasks_harness_seeded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text(
                "# Assessment\nNo known current bug matched this assessment.\n", encoding="utf-8"
            )
            (plan / "task_01.md").write_text(
                """Title: Repair evidence-backed planning after no-task sessions
Files: scripts/preseed_session_plan.py, scripts/task_manifest.py
Issue: none
Origin: harness-seed
validated_against_assessment: true

Objective:
Improve fallback task selection.

Why this matters:
The harness reached planning with no task artifacts.

Success Criteria:
- Fallback tasks avoid protected files.

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- planning_failed remains visible when it occurs.
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            self.assertTrue(manifest["planner"]["planning_failed"])
            self.assertEqual(manifest["planner"]["selected_task_count"], 0)
            self.assertEqual(len(manifest["tasks"]), 1)
            self.assertEqual(manifest["selected_tasks"], [])
            self.assertEqual(manifest["tasks"][0]["origin"], "harness-seed")
            self.assertIn("all_tasks_harness_seeded", manifest["warnings"])
            self.assertIn("planner_produced_no_task_files", manifest["warnings"])
            payload = task_manifest.decision_payload(manifest)
            self.assertTrue(payload["planning_failed"])
            self.assertEqual(payload["decision"], "planning_failed")
            self.assertEqual(payload["selected_task_count"], 0)
            self.assertEqual(payload["tasks"], [])

    def test_manifest_no_warning_when_planner_tasks_exist(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nSome assessment.\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: Planner task
Files: src/main.rs
Issue: none
Origin: planner

Objective:
Do something useful.

Why this matters:
It matters.

Success Criteria:
- Something works.

Verification:
- cargo test

Expected Evidence:
- Tests pass.
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            self.assertNotIn("all_tasks_harness_seeded", manifest["warnings"])

    def test_manifest_rejects_analysis_only_verified_without_code_escape(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text(
                "# Assessment\nTranscript-only tool failure gap needs current evidence.\n",
                encoding="utf-8",
            )
            (plan / "task_01.md").write_text(
                """Title: Close transcript-only tool-failure state-capture gap
Files: src/tool_wrappers.rs, src/state.rs
Issue: none
Origin: planner

Evidence:
- transcript_only_failed_tool_count=1

Objective:
Ensure every transcript-visible tool failure is represented in state.

Success Criteria:
- transcript_only_failed_tool_count drops to 0

Verification:
- cargo test -- tool_failure

Expected Evidence:
- Future dashboard runs show transcript_only_failed_tool_count=0.

Fallback:
- If investigation confirms the 1 transcript-only failure is a dashboard false positive,
  document the finding and mark the task verified without code changes.
""",
                encoding="utf-8",
            )
            (plan / "task_02.md").write_text(
                """Title: Tighten manifest planning quality
Files: scripts/task_manifest.py, scripts/test_task_manifest.py
Issue: none
Origin: planner

Objective:
Prevent analysis-only implementation tasks from being selected.

Success Criteria:
- analysis-only escape tasks are skipped before implementation

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- manifest warnings include analysis_only_escape.
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            payload = task_manifest.decision_payload(manifest)

            self.assertIn("task_01:analysis_only_escape", manifest["warnings"])
            self.assertIn("task_01:thin_task_spec", manifest["warnings"])
            self.assertEqual(manifest["planner"]["selected_task_count"], 1)
            self.assertEqual(manifest["selected_tasks"][0]["task_id"], "task_02")
            self.assertTrue(manifest["tasks"][0]["quality"]["analysis_only_escape"])
            self.assertLess(manifest["tasks"][0]["quality"]["score"], 0.75)
            self.assertEqual(payload["decision"], "tasks_selected")
            self.assertEqual(payload["tasks"][0]["task_id"], "task_02")

    def test_manifest_rejects_protected_file_tasks_before_selection(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nProtected edit was proposed.\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: Patch live evolve script
Files: scripts/evolve.sh
Issue: none
Origin: planner

Objective:
Change the running evolution script.

Success Criteria:
- Protected edit is blocked.

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- The task is not selected.
""",
                encoding="utf-8",
            )
            (plan / "task_02.md").write_text(
                """Title: Improve manifest filtering
Files: scripts/task_manifest.py
Issue: none
Origin: planner

Objective:
Select the safe follow-up task instead.

Success Criteria:
- Safe tasks still run.

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- task_02 is selected after task_01 is rejected.
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            payload = task_manifest.decision_payload(manifest)

            self.assertEqual(manifest["planner"]["task_count"], 2)
            self.assertEqual(manifest["planner"]["selected_task_count"], 1)
            self.assertEqual(manifest["planner"]["protected_task_count"], 1)
            self.assertEqual(manifest["selected_tasks"][0]["task_id"], "task_02")
            self.assertEqual(manifest["tasks"][0]["protected_files"], ["scripts/evolve.sh"])
            self.assertIn("task_01:protected_files", manifest["warnings"])
            self.assertEqual(payload["decision"], "tasks_selected")
            self.assertEqual(payload["tasks"][0]["task_id"], "task_02")

    def test_manifest_records_no_selectable_tasks_when_all_tasks_are_protected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "task_01.md").write_text(
                """Title: Patch protected skill
Files: skills/evolve/SKILL.md
Issue: none
Origin: planner

Objective:
Change protected evolution prompt files.

Success Criteria:
- The task is rejected before implementation.

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- No implementation task is selected.
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)
            payload = task_manifest.decision_payload(manifest)

            self.assertFalse(manifest["planner"]["planning_failed"])
            self.assertEqual(manifest["planner"]["task_count"], 1)
            self.assertEqual(manifest["planner"]["selected_task_count"], 0)
            self.assertEqual(manifest["selected_tasks"], [])
            self.assertIn("task_01:protected_files", manifest["warnings"])
            self.assertIn("no_selectable_tasks", manifest["warnings"])
            self.assertEqual(payload["decision"], "no_selectable_tasks")

    def test_manifest_does_not_reject_safe_task_that_mentions_protected_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "task_01.md").write_text(
                """Title: Document protected-file guard
Files: scripts/task_manifest.py
Issue: none
Origin: planner

Objective:
Improve validation while explaining why scripts/evolve.sh must not be edited by this task.

Success Criteria:
- The safe manifest file remains selectable.

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- The task is selected despite the protected-file mention in the body.
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            self.assertEqual(manifest["planner"]["selected_task_count"], 1)
            self.assertEqual(manifest["selected_tasks"][0]["task_id"], "task_01")
            self.assertNotIn("task_01:protected_files", manifest["warnings"])

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


    def test_evidence_references_nonexistent_artifact_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nState feedback.\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: Fix something
Files: src/main.rs
Issue: none
Origin: planner

Objective:
Make something work.

Evidence:
- The assessment at session_plan/assessment.md shows the issue.
- Referenced artifact: nonexistent/path/to/dead/artifact.log
- Another reference: also/does/not/exist.json
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            # The task should still be selected (warning only, not an error)
            self.assertEqual(manifest["planner"]["selected_task_count"], 1)
            # Warning should mention the stale artifact reference
            warnings = manifest.get("warnings") or []
            stale_warnings = [w for w in warnings if "stale_evidence_artifact" in w]
            self.assertTrue(len(stale_warnings) > 0,
                            f"Expected stale_evidence_artifact warning, got warnings={warnings}")

    def test_evidence_references_existing_artifact_path_no_warning(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nState feedback.\n", encoding="utf-8")
            # Create an actual artifact file
            (plan / "real_artifact.log").write_text("real content\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                f"""Title: Fix something
Files: src/main.rs
Issue: none
Origin: planner

Objective:
Make something work.

Evidence:
- The assessment at session_plan/assessment.md shows the issue.
- Referenced artifact: {plan}/real_artifact.log
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            warnings = manifest.get("warnings") or []
            stale_warnings = [w for w in warnings if "stale_evidence_artifact" in w]
            self.assertEqual(len(stale_warnings), 0,
                             f"Should NOT have stale_evidence_artifact warning for existing paths, got warnings={warnings}")


    def test_cross_reference_mismatch_detected_and_scores_lowered(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nState feedback.\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: Fix the thing
Files: src/main.rs
Issue: none
Origin: planner

Objective:
Update the library in src/lib.rs and the config in scripts/config.py.

Why this matters:
It breaks things.

Success Criteria:
- Library updated

Verification:
- python3 -m unittest

Expected Evidence:
- Test passes
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            task = manifest["selected_tasks"][0]
            quality = task["quality"]
            self.assertEqual(
                quality["cross_reference_mismatch"],
                ["src/lib.rs", "scripts/config.py"],
                "Should detect body file mentions not in Files: line",
            )
            # Score should be capped at 0.8 and reduced by 0.2 (2 mismatches) -> 0.6
            self.assertLess(quality["score"], 0.9, "Score should be lowered for mismatch")
            self.assertGreaterEqual(quality["score"], 0.3, "Score floor should be 0.3")

            warnings = manifest.get("warnings") or []
            mismatch_warnings = [w for w in warnings if "cross_reference_mismatch" in w]
            self.assertEqual(len(mismatch_warnings), 1, f"Expected cross_reference_mismatch warning, got warnings={warnings}")

    def test_no_cross_reference_mismatch_when_files_match(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "session_plan"
            plan.mkdir()
            (plan / "assessment.md").write_text("# Assessment\nState feedback.\n", encoding="utf-8")
            (plan / "task_01.md").write_text(
                """Title: Fix the thing
Files: src/main.rs, src/lib.rs
Issue: none
Origin: planner

Objective:
Update src/main.rs and src/lib.rs.

Why this matters:
It breaks things.

Success Criteria:
- Both files updated

Verification:
- cargo test

Expected Evidence:
- Test passes
""",
                encoding="utf-8",
            )
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
                    "planning_failed": False,
                },
            )()

            manifest = task_manifest.build_manifest(args)

            task = manifest["selected_tasks"][0]
            quality = task["quality"]
            self.assertEqual(
                quality["cross_reference_mismatch"],
                [],
                "No mismatch when all body mentions are in Files: line",
            )
            # Score should be full 1.0 (no mismatches, all fields present)
            self.assertEqual(quality["score"], 1.0)


if __name__ == "__main__":
    unittest.main()
