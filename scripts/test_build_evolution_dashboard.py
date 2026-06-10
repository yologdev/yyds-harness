#!/usr/bin/env python3
"""Tests for scripts/build_evolution_dashboard.py."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from build_evolution_dashboard import build, summarize_transcript_actions  # noqa: E402


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def write_events(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


class BuildEvolutionDashboard(unittest.TestCase):
    def test_transcript_action_paths_drop_pseudo_workspace_dot(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "\n".join(
                    [
                        "  ▶ read /home/runner/work/yyds-harness/yyds-harness/src/state.rs ✓ (5ms)",
                        "  ▶ edit /home/runner/work/yyds-harness/yyds-harness/session_plan/eval_task_1.md ✓ (5ms)",
                        "  ▶ edit /home/runner/work/yyds-harness/yyds-harness/.github/workflows/ci.yml ✓ (5ms)",
                        "  ▶ edit /home/runner/work/yyds-harness/yyds-harness/.gitignore ✓ (5ms)",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            actions = summarize_transcript_actions(session)

            self.assertIn("src/state.rs", actions["read_files"])
            self.assertIn("session_plan/eval_task_1.md", actions["edited_files"])
            self.assertIn(".github/workflows/ci.yml", actions["edited_files"])
            self.assertIn(".gitignore", actions["edited_files"])
            self.assertNotIn(".src/state.rs", actions["read_files"])
            self.assertNotIn(".session_plan/eval_task_1.md", actions["edited_files"])

    def test_transcript_search_in_src_does_not_emit_pseudo_dot_src(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "  ▶ search 'mark_run_completed_with_error' in /home/runner/work/yyds-harness/yyds-harness/src ✗ (17ms)\n",
                encoding="utf-8",
            )

            actions = summarize_transcript_actions(session)

            self.assertNotIn(".src", actions["read_files"])
            self.assertNotIn("src", actions["read_files"])
            self.assertIn("search 'mark_run_completed_with_error' in src", actions["failed_commands"])

    def test_data_contract_reports_generated_at_and_latest_session(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            older = root / "sessions/day-2-20260602T000000Z"
            newer = root / "sessions/day-10-20260610T000000Z"
            write_json(
                older / "outcome.json",
                {
                    "day": 2,
                    "ts": "2026-06-02T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(older / "state/summary.json", {"latest_gnomes": {"coding_log_score": 0.2}})
            write_json(
                newer / "outcome.json",
                {
                    "day": 10,
                    "ts": "2026-06-10T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(newer / "state/summary.json", {"latest_gnomes": {"coding_log_score": 0.9}})

            data = build(root / "sessions", root / "out")

            self.assertRegex(data["generated_at"], r"^\d{4}-\d{2}-\d{2}T")
            self.assertEqual([session["id"] for session in data["sessions"]], [older.name, newer.name])
            self.assertEqual(data["aggregate"]["latest_session_id"], newer.name)
            self.assertEqual(data["aggregate"]["latest_ts"], "2026-06-10T00:00:00Z")
            self.assertEqual(data["aggregate"]["latest_gnomes"]["coding_log_score"], 0.9)

    def test_derives_operational_state_capture_from_trace_quality(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1, "tasks_attempted": 0, "tasks_succeeded": 0})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"RunStarted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {"state_capture_coverage": 1.0},
                    "gnome_keys": ["state_capture_coverage"],
                    "evals": [{"suite": "log-feedback", "gnomes": {"state_capture_coverage": 1.0}}],
                },
            )

            data = build(root / "sessions", root / "out")
            latest = data["sessions"][0]["latest_eval"]["gnomes"]

            self.assertEqual(latest["state_capture_coverage"], 1.0)
            self.assertEqual(latest["state_operational_capture_coverage"], 0.0)
            self.assertIn("state_operational_capture_coverage", data["aggregate"]["gnome_keys"])

    def test_dashboard_prioritizes_operational_state_capture(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1, "tasks_attempted": 0, "tasks_succeeded": 0})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"RunStarted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {
                        "coding_log_score": 0.8,
                        "task_success_rate": 0.0,
                        "workflow_success_rate": 1.0,
                        "state_capture_coverage": 1.0,
                    },
                    "gnome_keys": [
                        "coding_log_score",
                        "task_success_rate",
                        "workflow_success_rate",
                        "state_capture_coverage",
                    ],
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "gnomes": {
                                "coding_log_score": 0.8,
                                "task_success_rate": 0.0,
                                "workflow_success_rate": 1.0,
                                "state_capture_coverage": 1.0,
                            },
                        }
                    ],
                },
            )

            build(root / "sessions", root / "out")
            html = (root / "out/index.html").read_text(encoding="utf-8")
            priority_block = html.split("const priorityGnomes = [", 1)[1].split("];", 1)[0]

            self.assertLess(
                priority_block.index('"state_operational_capture_coverage"'),
                priority_block.index('"state_capture_coverage"'),
            )
            self.assertIn('return "-"', html)

    def test_derives_work_summary_and_gnome_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 2,
                    "tasks_succeeded": 2,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 5,
                    "event_counts": {"FileEdited": 1, "CommandStarted": 1},
                    "latest_gnomes": {
                        "coding_log_score": 0.8,
                        "coding_log_available": True,
                        "evolution_friction_count": 2,
                    },
                    "gnome_keys": [
                        "coding_log_score",
                        "coding_log_available",
                        "evolution_friction_count",
                    ],
                    "evals": [
                        {
                            "eval_id": "eval-1",
                            "suite": "log-feedback",
                            "status": "passed",
                            "score": 0.8,
                            "gnomes": {
                                "coding_log_score": 0.8,
                                "coding_log_available": True,
                                "closed_loop_fix_rate": None,
                                "evolution_friction_count": 2,
                            },
                        }
                    ],
                    "patches": [{"patch_id": "patch-1"}],
                    "decisions": [{"decision": "promote", "eligible": True}],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Improve dashboard",
                            "status": "completed",
                            "source_files": ["scripts/build_evolution_dashboard.py"],
                            "commit_shas": ["abc123"],
                            "eval": {"verdict": "PASS"},
                            "gnome_deltas": {"coding_log_score": 0.1},
                        }
                    ],
                    "blockers": [],
                },
            )
            write_events(
                session / "state/events.jsonl",
                [
                    {
                        "kind": "FileEdited",
                        "payload": {"path": "scripts/build_evolution_dashboard.py"},
                    },
                    {
                        "kind": "CommandStarted",
                        "payload": {"command": "cargo test"},
                    },
                ],
            )
            (session / "transcripts").mkdir()
            (session / "transcripts/plan.log").write_text("plan\n", encoding="utf-8")
            (session / "transcripts/task_01_attempt1.log").write_text("task\nline2\n", encoding="utf-8")
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {
                        "planning_failed": False,
                        "task_count": 1,
                        "selected_task_count": 1,
                        "assessment_present": True,
                    },
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Improve dashboard",
                            "files": ["scripts/build_evolution_dashboard.py"],
                            "issue": "none",
                            "origin": "planner",
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0, "generic_self_improvement": False},
                        }
                    ],
                    "artifacts": {
                        "manifest": "tasks/manifest.json",
                        "assessment": "tasks/assessment.md",
                    },
                    "warnings": [],
                },
            )
            (session / "tasks/assessment.md").write_text("# Assessment\n", encoding="utf-8")
            (session / "tasks/task_01/task.md").write_text("Title: Improve dashboard\nFiles: scripts/build_evolution_dashboard.py\n", encoding="utf-8")
            (session / "tasks/task_01/attempts.jsonl").write_text(
                json.dumps(
                    {
                        "task_id": "task_01",
                        "phase": "implementation",
                        "attempt": 1,
                        "stage_name": "task_01_attempt1",
                        "transcript_path": "transcripts/task_01_attempt1.log",
                        "exit_code": 0,
                        "status": "completed",
                        "line_count": 2,
                    },
                    separators=(",", ":"),
                )
                + "\n",
                encoding="utf-8",
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "attempt": 1,
                    "status": "pass",
                    "verdict": "Verdict: PASS",
                    "transcript_path": "transcripts/eval_task1_attempt1.log",
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Improve dashboard",
                    "status": "completed",
                },
            )

            data = build(root / "sessions", root / "out")

            self.assertEqual(data["schema_version"], 2)
            self.assertEqual(
                data["gnome_numeric_keys"],
                [
                    "coding_log_score",
                    "evaluator_timeout_with_verdict_count",
                    "evaluator_unverified_count",
                    "evolution_friction_count",
                    "session_success_rate",
                    "task_success_rate",
                    "task_unlanded_source_count",
                ],
            )
            self.assertEqual(
                data["gnome_history"][0]["values"],
                {
                    "coding_log_score": 0.8,
                    "evaluator_timeout_with_verdict_count": 0.0,
                    "evaluator_unverified_count": 0.0,
                    "evolution_friction_count": 2.0,
                    "session_success_rate": 1.0,
                    "task_success_rate": 1.0,
                    "task_unlanded_source_count": 0.0,
                },
            )

            work = data["sessions"][0]["work_summary"]
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("const planned = text((row.planned_files || [])", html)
            self.assertIn("const touched = text((row.touched_files || [])", html)
            self.assertNotIn("planned ${(row.planned_files || [])", html)
            self.assertIn("1/1 verified tasks", work["headline"])
            self.assertIn("outcome reported 2/2 tasks", work["headline"])
            self.assertEqual(work["edited_files"], ["scripts/build_evolution_dashboard.py"])
            self.assertEqual(work["commands"], ["cargo test"])
            self.assertEqual(work["transcripts"]["phase_counts"], {"plan": 1, "task": 1})
            self.assertEqual(work["transcripts"]["files"][1]["line_count"], 2)
            self.assertEqual(work["task_manifest"]["selected_task_count"], 1)
            self.assertFalse(work["task_manifest"]["planning_failed"])
            self.assertEqual(work["task_manifest"]["tasks"][0]["quality_score"], 1.0)
            self.assertEqual(work["task_artifacts"][0]["task_id"], "task_01")
            self.assertEqual(work["task_artifacts"][0]["attempt_count"], 1)
            self.assertEqual(work["task_artifacts"][0]["eval_statuses"], ["pass"])
            self.assertTrue(work["task_artifacts"][0]["has_outcome"])
            self.assertEqual(work["task_verification"]["verified_task_count"], 1)
            self.assertEqual(work["task_verification"]["unverified_task_count"], 0)
            self.assertEqual(work["causal_chains"][0]["task_id"], "task_01")
            self.assertEqual(work["causal_chains"][0]["planned_files"], ["scripts/build_evolution_dashboard.py"])
            self.assertEqual(work["causal_chains"][0]["commit_shas"], ["abc123"])
            self.assertIn("task.md", [row["name"] for row in work["task_artifacts"][0]["artifacts"]])
            self.assertEqual(work["patch_count"], 1)
            self.assertEqual(work["decision_count"], 1)
            self.assertEqual(work["task_lineage"][0]["task_id"], "task_01")
            self.assertEqual(work["task_lineage"][0]["gnome_deltas"], {"coding_log_score": 0.1})
            self.assertTrue(work["task_lineage"][0]["strict_success"])
            self.assertEqual(work["task_lineage"][0]["verification_status"], "strict_pass")
            self.assertEqual(work["task_lineage"][0]["verification_problems"], [])
            self.assertEqual(data["sessions"][0]["trace_quality"]["status"], "full")
            self.assertEqual(data["sessions"][0]["health"], "passed")

    def test_state_pipeline_diagnostics_are_visible(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"RunStarted": 1, "CacheMetricsRecorded": 1},
                },
            )
            write_json(
                session / "state_replay.json",
                {"files_read": 3, "events_written": 20, "duplicates_skipped": 2},
            )
            write_json(
                session / "state/merge_state_delta.json",
                {
                    "live_events": 25,
                    "base_lines": 20,
                    "delta_events": 5,
                    "added": 4,
                    "skipped_duplicate": 1,
                    "session_events_after": 6,
                },
            )
            (session / "state/append_state_event.log").write_text(
                "append_state_event.py failed once\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out")
            current = data["sessions"][0]
            pipeline = current["work_summary"]["state_pipeline"]
            html = (root / "out/index.html").read_text(encoding="utf-8")

            self.assertEqual(current["trace_quality"]["operational_event_count"], 1)
            self.assertEqual(pipeline["replay_scope"], "audit_history")
            self.assertEqual(pipeline["replay_events_written"], 20)
            self.assertEqual(pipeline["merge_scope"], "live_delta")
            self.assertEqual(pipeline["merge_added_events"], 4)
            self.assertEqual(pipeline["append_problem_lines"], 1)
            self.assertIn("State pipeline", html)
            self.assertIn("audit replay", html)

    def test_state_pipeline_explains_missing_live_merge_diagnostics(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1})
            write_json(session / "state/summary.json", {})
            write_json(
                session / "state_replay.json",
                {"files_read": 3, "events_written": 20, "duplicates_skipped": 2},
            )

            data = build(root / "sessions", root / "out")
            pipeline = data["sessions"][0]["work_summary"]["state_pipeline"]
            html = (root / "out/index.html").read_text(encoding="utf-8")

            self.assertEqual(pipeline["replay_scope"], "audit_history")
            self.assertIsNone(pipeline["merge_scope"])
            self.assertIn("live delta merge diagnostics not recorded", html)

    def test_strict_verification_requires_landed_source_commit(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 4,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                    "evals": [{"suite": "log-feedback", "status": "passed", "score": 0.8}],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Unlanded source edit",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                            "commit_shas": [],
                            "eval": {"verdict": "PASS"},
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Unlanded source edit",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": [],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            verification = session_data["work_summary"]["task_verification"]

            self.assertEqual(verification["verified_task_count"], 0)
            self.assertEqual(verification["unverified_task_count"], 1)
            self.assertIn("no_landed_source_commit", verification["rows"][0]["problems"])
            self.assertEqual(session_data["latest_gnomes"]["task_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["session_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["task_unlanded_source_count"], 1)
            self.assertEqual(session_data["health"], "attention")

    def test_timed_out_verdict_does_not_count_as_strict_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 4,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                    "evals": [{"suite": "log-feedback", "status": "passed", "score": 0.8}],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Timeout after verdict",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                            "commit_shas": ["abc123"],
                            "eval": {"verdict": "PASS"},
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Timeout after verdict",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": ["abc123"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "status": "pass",
                    "exit_code": 124,
                    "verdict": "Verdict: PASS",
                    "verdict_file": "eval_attempt_1.md",
                },
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            verification = session_data["work_summary"]["task_verification"]

            self.assertEqual(verification["verified_task_count"], 0)
            self.assertEqual(verification["unverified_task_count"], 1)
            self.assertIn("evaluator_timed_out_after_verdict", verification["rows"][0]["problems"])
            self.assertIn("no_passing_verifier", verification["rows"][0]["problems"])
            self.assertEqual(session_data["latest_gnomes"]["task_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["session_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["evaluator_timeout_with_verdict_count"], 1)
            self.assertEqual(session_data["health"], "attention")

    def test_task_lineage_gnome_snapshots_use_corrected_strict_metrics(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            stale_gnomes = {
                "coding_log_score": 0.8,
                "session_success_rate": 1.0,
                "task_success_rate": 0.5,
            }
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 4,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1},
                    "latest_gnomes": stale_gnomes,
                    "gnome_keys": list(stale_gnomes),
                    "evals": [{"suite": "log-feedback", "status": "passed", "score": 0.8}],
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Stale strict metric",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                            "commit_shas": [],
                            "eval": {"verdict": "PASS"},
                            "gnome_metrics": stale_gnomes,
                            "gnome_deltas": {
                                "session_success_rate": 1.0,
                                "task_success_rate": -0.5,
                            },
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Stale strict metric",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": [],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            lineage = session_data["work_summary"]["task_lineage"][0]

            self.assertEqual(session_data["latest_gnomes"]["task_success_rate"], 0.0)
            self.assertEqual(session_data["latest_gnomes"]["session_success_rate"], 0.0)
            self.assertEqual(lineage["gnome_metrics"]["task_success_rate"], 0.0)
            self.assertEqual(lineage["gnome_metrics"]["session_success_rate"], 0.0)
            self.assertEqual(lineage["gnome_metrics"]["task_unlanded_source_count"], 1)
            self.assertEqual(lineage["gnome_deltas"]["task_success_rate"], -1.0)
            self.assertEqual(lineage["gnome_deltas"]["session_success_rate"], 0.0)
            self.assertEqual(
                lineage["gnome_corrections"]["task_success_rate"],
                {"from": 0.5, "to": 0.0},
            )
            self.assertEqual(
                session_data["latest_eval"]["score"],
                session_data["latest_gnomes"]["coding_log_score"],
            )
            self.assertEqual(
                session_data["work_summary"]["latest_eval_score"],
                session_data["latest_gnomes"]["coding_log_score"],
            )
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("corrected gnome(s)", html)

    def test_cache_ratio_without_token_evidence_is_marked_unverified(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 0,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_cache_hit_ratio": 0.91,
                        "deepseek_cache_hit_tokens": None,
                        "deepseek_cache_miss_tokens": None,
                    },
                    "gnome_keys": [
                        "deepseek_cache_hit_ratio",
                        "deepseek_cache_hit_tokens",
                        "deepseek_cache_miss_tokens",
                    ],
                    "evals": [
                        {
                            "suite": "log-feedback",
                            "status": "passed",
                            "gnomes": {
                                "deepseek_cache_hit_ratio": 0.91,
                                "deepseek_cache_hit_tokens": None,
                                "deepseek_cache_miss_tokens": None,
                            },
                        }
                    ],
                },
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            latest = session_data["latest_gnomes"]

            self.assertIsNone(latest["deepseek_cache_hit_ratio"])
            self.assertEqual(latest["deepseek_cache_ratio_unverified_count"], 1)
            self.assertIsNone(session_data["latest_eval"]["gnomes"]["deepseek_cache_hit_ratio"])
            self.assertEqual(
                session_data["latest_eval"]["gnome_corrections"]["deepseek_cache_hit_ratio"],
                {"from": 0.91, "to": None},
            )
            self.assertNotIn("deepseek_cache_hit_ratio", data["gnome_history"][0]["values"])
            self.assertEqual(data["gnome_history"][0]["values"]["deepseek_cache_ratio_unverified_count"], 1.0)
            self.assertIn("deepseek_cache_ratio_unverified_count", data["aggregate"]["gnome_keys"])
            self.assertIn("deepseek_cache_ratio_unverified_count", data["gnome_numeric_keys"])
            html = (root / "out/index.html").read_text(encoding="utf-8")
            self.assertIn("Cache evidence", html)
            self.assertIn("token evidence was missing", html)
            self.assertIn("CacheMetricsRecorded events", html)

    def test_manifest_files_backfilled_from_task_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 5,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1},
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                    "evals": [{"suite": "log-feedback", "status": "passed", "score": 0.8}],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Wire panic hook",
                            "files": [],
                            "artifact_path": "tasks/task_01/task.md",
                            "quality": {"score": 1.0},
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )
            (session / "tasks/task_01/task.md").write_text(
                "\n".join(
                    [
                        "Title: Wire panic hook",
                        "",
                        "Files: src/state.rs",
                        "",
                        "Issue: none",
                        "",
                        "Origin: planner",
                        "",
                        "Objective: Make panic evidence visible.",
                        "",
                        "Success Criteria:",
                        "- Panic diagnostic is stashed.",
                        "",
                        "Verification:",
                        "- cargo test",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": ["abc123"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
            )

            data = build(root / "sessions", root / "out")
            work = data["sessions"][0]["work_summary"]

            self.assertEqual(work["task_manifest"]["tasks"][0]["files"], ["src/state.rs"])
            self.assertEqual(
                work["task_verification"]["rows"][0]["planned_files"],
                ["src/state.rs"],
            )
            self.assertEqual(work["task_verification"]["verified_task_count"], 1)

    def test_derives_source_changes_from_matching_session_commits(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            repo = root / "repo"
            repo.mkdir()
            subprocess.run(["git", "-C", str(repo), "init"], check=True, stdout=subprocess.DEVNULL)
            subprocess.run(
                ["git", "-C", str(repo), "config", "user.email", "test@example.com"],
                check=True,
            )
            subprocess.run(["git", "-C", str(repo), "config", "user.name", "Test"], check=True)
            (repo / "src").mkdir()
            (repo / "session_plan").mkdir()
            (repo / ".yoyo").mkdir()
            (repo / "src/lib.rs").write_text("pub fn ok() {}\n", encoding="utf-8")
            (repo / "session_plan/task_01.md").write_text("plan\n", encoding="utf-8")
            (repo / ".yoyo/context-semantic-index.json").write_text("{}\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
            subprocess.run(
                ["git", "-C", str(repo), "commit", "-m", "Day 1 (00:00): implement source"],
                check=True,
                stdout=subprocess.DEVNULL,
            )
            subprocess.run(
                ["git", "-C", str(repo), "add", "session_plan/task_01.md", ".yoyo/context-semantic-index.json"],
                check=True,
            )
            subprocess.run(
                ["git", "-C", str(repo), "commit", "-m", "Day 1 (00:00): session wrap-up"],
                check=True,
                stdout=subprocess.DEVNULL,
            )

            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "session_time": "00:00",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                },
            )
            write_json(session / "state/summary.json", {})
            write_events(
                session / "state/events.jsonl",
                [{"kind": "FileEdited", "payload": {"path": "journals/JOURNAL.md"}}],
            )

            data = build(root / "sessions", root / "out", repo)

            work = data["sessions"][0]["work_summary"]
            self.assertEqual(work["source_changed_files"], ["src/lib.rs"])
            self.assertEqual(work["touched_source_files"], ["src/lib.rs"])
            self.assertEqual(work["edited_files"], ["journals/JOURNAL.md"])
            self.assertIn("1 source file(s) changed", work["headline"])
            self.assertEqual(work["source_patch_count"], 1)
            self.assertEqual(work["landed_patch_count"], 1)
            self.assertEqual(work["state_patch_count"], 0)
            self.assertEqual(work["landed_commit_count"], 2)
            self.assertEqual(work["source_commit_count"], 1)
            self.assertEqual(work["bookkeeping_commit_count"], 1)
            self.assertEqual(len(work["source_commits"]), 1)
            self.assertEqual(len(work["bookkeeping_commits"]), 1)
            self.assertEqual(work["commits"][0]["subject"], "Day 1 (00:00): implement source")
            self.assertEqual(work["commits"][1]["source_files"], [])
            self.assertEqual(work["source_commits"][0]["subject"], "Day 1 (00:00): implement source")
            self.assertEqual(work["bookkeeping_commits"][0]["subject"], "Day 1 (00:00): session wrap-up")

    def test_feedback_only_trace_is_explicit(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "github_run_id": "123",
                    "github_run_attempt": "2",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 1,
                    "event_counts": {"PatchEvaluated": 1},
                    "evals": [{"suite": "log-feedback", "eval_id": "log-feedback-123-1"}],
                },
            )
            write_events(
                session / "state/events.jsonl",
                [{"event_type": "PatchEvaluated", "payload": {"suite": "log-feedback"}}],
            )

            data = build(root / "sessions", root / "out")

            current = data["sessions"][0]
            self.assertEqual(current["github_run_id"], "123")
            self.assertEqual(current["github_run_attempt"], "2")
            self.assertEqual(current["trace_quality"]["status"], "feedback_only")
            self.assertEqual(current["trace_quality"]["trace_event_count"], 0)
            self.assertEqual(data["aggregate"]["feedback_only_sessions"], 1)
            self.assertEqual(data["aggregate"]["full_trace_sessions"], 0)

    def test_lifecycle_only_trace_is_not_counted_as_full(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"day": 1})
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 7,
                    "event_counts": {
                        "RunStarted": 2,
                        "RunCompleted": 2,
                        "DecisionRecorded": 1,
                        "TaskLineageLinked": 1,
                        "PatchEvaluated": 1,
                    },
                    "evals": [{"suite": "log-feedback", "eval_id": "log-feedback-123-1"}],
                },
            )

            data = build(root / "sessions", root / "out")
            current = data["sessions"][0]

            self.assertEqual(current["trace_quality"]["status"], "lifecycle")
            self.assertEqual(current["trace_quality"]["label"], "lifecycle-only trace")
            self.assertEqual(current["trace_quality"]["operational_event_count"], 0)
            self.assertEqual(current["trace_quality"]["operational_capture_coverage"], 0.0)
            self.assertEqual(data["aggregate"]["full_trace_sessions"], 0)
            self.assertEqual(data["aggregate"]["lifecycle_trace_sessions"], 1)
            self.assertIn("Operational traces", (root / "out/index.html").read_text(encoding="utf-8"))

    def test_task_lineage_eval_is_enriched_from_task_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "tasks_attempted": 1, "tasks_succeeded": 1, "build_ok": True, "test_ok": True},
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 3,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Timeout eval should be visible",
                            "status": "completed",
                            "planned_files": ["src/state.rs"],
                            "source_files": ["src/state.rs"],
                        }
                    ],
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "planner": {"planning_failed": False, "task_count": 1, "selected_task_count": 1},
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Timeout eval should be visible",
                            "files": ["src/state.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            (session / "tasks/task_01/task.md").write_text("Title: Timeout eval should be visible\n", encoding="utf-8")
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Timeout eval should be visible",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "attempt": 1,
                    "status": "timeout",
                    "exit_code": 124,
                    "transcript_path": "transcripts/eval_task1_attempt1.log",
                },
            )

            data = build(root / "sessions", root / "out")
            lineage = data["sessions"][0]["work_summary"]["task_lineage"][0]
            causal = data["sessions"][0]["work_summary"]["causal_chains"][0]
            verification = data["sessions"][0]["work_summary"]["task_verification"]

            self.assertEqual(lineage["eval"]["verdict"], "TIMEOUT")
            self.assertEqual(lineage["eval"]["status"], "timeout")
            self.assertIn("timed out", lineage["eval"]["reason"])
            self.assertEqual(lineage["eval"]["transcript_path"], "transcripts/eval_task1_attempt1.log")
            self.assertFalse(lineage["strict_success"])
            self.assertEqual(lineage["verification_status"], "strict_failed")
            self.assertIn("no_passing_verifier", lineage["verification_problems"])
            self.assertFalse(causal["strict_success"])
            self.assertEqual(causal["verification_status"], "strict_failed")
            self.assertIn("no_passing_verifier", causal["verification_problems"])
            self.assertEqual(verification["verified_task_count"], 0)
            self.assertIn("no_passing_verifier", verification["rows"][0]["problems"])

    def test_transcript_actions_fill_work_evidence_when_state_events_are_sparse(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 3,
                    "event_counts": {"RunStarted": 1, "RunCompleted": 1, "PatchEvaluated": 1},
                    "evals": [{"suite": "log-feedback", "status": "failed", "score": 0.4}],
                },
            )
            (session / "transcripts").mkdir(parents=True)
            (session / "transcripts/task_01_attempt1.log").write_text(
                "\n".join(
                    [
                        "  ▶ read src/state.rs:30..80 ✓ (5ms)",
                        "  ▶ search 'fn build_why_report' in src/commands_state.rs ✗ (17ms)",
                        "  ▶ edit src/state.rs (3 → 5 lines)",
                        "  ▶ $ cd /home/runner/work/yyds-harness/yyds-harness && cargo test state::tests::panic_hook ✓ (141ms)",
                        "  ✗ Watch failed: `cargo clippy --all-targets -- -D warnings && cargo test`",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            data = build(root / "sessions", root / "out")
            work = data["sessions"][0]["work_summary"]

            self.assertEqual(work["read_files"], ["src/state.rs", "src/commands_state.rs"])
            self.assertEqual(work["edited_files"], ["src/state.rs"])
            self.assertEqual(work["touched_source_files"], ["src/state.rs"])
            self.assertEqual(
                work["commands"],
                [
                    "cargo test state::tests::panic_hook",
                    "cargo clippy --all-targets -- -D warnings && cargo test",
                ],
            )
            self.assertIn(
                "cargo clippy --all-targets -- -D warnings && cargo test",
                work["failed_commands"],
            )
            self.assertIn("1 source file(s) touched", work["headline"])
            self.assertIn("2 command/check signal(s)", work["headline"])
            self.assertEqual(work["transcript_actions"]["edited_files"], ["src/state.rs"])

    def test_session_sort_is_natural_by_day_and_timestamp(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            sessions = root / "sessions"
            for name, day, ts in [
                ("day-99-20260607T224346Z", 99, "2026-06-07T22:43:46Z"),
                ("day-100-20260608T003408Z", 100, "2026-06-08T00:34:08Z"),
                ("day-98-20260606T163045Z", 98, "2026-06-06T16:30:45Z"),
            ]:
                write_json(sessions / name / "outcome.json", {"day": day, "ts": ts})
                write_json(sessions / name / "state/summary.json", {})

            data = build(sessions, root / "out")

            self.assertEqual(
                [session["id"] for session in data["sessions"]],
                [
                    "day-98-20260606T163045Z",
                    "day-99-20260607T224346Z",
                    "day-100-20260608T003408Z",
                ],
            )

    def test_aggregate_gnome_keys_include_corrected_latest_gnomes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Unlanded task",
                    "status": "completed",
                    "planned_files": ["src/state.rs"],
                    "touched_files": ["src/state.rs"],
                    "source_files": ["src/state.rs"],
                    "commit_shas": [],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "status": "pass",
                    "exit_code": 0,
                    "verdict": "Verdict: PASS",
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "selected_tasks": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "title": "Unlanded task",
                            "files": ["src/state.rs"],
                        }
                    ],
                    "artifacts": {"manifest": "tasks/manifest.json"},
                },
            )

            data = build(root / "sessions", root / "out")

            self.assertIn("coding_log_score", data["aggregate"]["gnome_keys"])
            self.assertIn("task_unlanded_source_count", data["aggregate"]["gnome_keys"])
            self.assertEqual(data["aggregate"]["latest_gnomes"]["task_unlanded_source_count"], 1)

    def test_raw_outcome_tasks_without_strict_evidence_are_not_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 3,
                    "tasks_succeeded": 3,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                },
            )

            data = build(root / "sessions", root / "out")
            session_data = data["sessions"][0]
            aggregate = data["aggregate"]

            self.assertEqual(session_data["health"], "attention")
            self.assertIn("3/3 raw outcome task(s)", session_data["work_summary"]["headline"])
            self.assertIn("missing strict task evidence", session_data["work_summary"]["labels"])
            self.assertEqual(aggregate["tasks_attempted"], 0)
            self.assertEqual(aggregate["tasks_succeeded"], 0)
            self.assertIsNone(aggregate["task_success_rate"])
            self.assertEqual(aggregate["raw_task_outcome_attempted"], 3)
            self.assertEqual(aggregate["raw_task_outcome_succeeded"], 3)
            self.assertEqual(aggregate["unverified_raw_task_outcome_attempted"], 3)
            self.assertEqual(aggregate["unverified_raw_task_outcome_succeeded"], 3)

    def test_task_artifacts_without_manifest_still_enter_strict_verification(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-06-06T00:00:00Z",
                    "build_ok": True,
                    "test_ok": True,
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"coding_log_score": 0.8},
                    "gnome_keys": ["coding_log_score"],
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "task_title": "Historical task without manifest",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": [],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {
                    "task_id": "task_01",
                    "status": "pass",
                    "exit_code": 0,
                    "verdict": "Verdict: PASS",
                },
            )

            data = build(root / "sessions", root / "out")
            verification = data["sessions"][0]["work_summary"]["task_verification"]

            self.assertEqual(verification["task_count"], 1)
            self.assertEqual(verification["verified_task_count"], 0)
            self.assertEqual(verification["unverified_task_count"], 1)
            self.assertEqual(verification["rows"][0]["task_id"], "task_01")
            self.assertIn("missing_planned_files", verification["rows"][0]["problems"])
            self.assertIn("no_landed_source_commit", verification["rows"][0]["problems"])
            self.assertEqual(data["aggregate"]["tasks_attempted"], 1)
            self.assertEqual(data["aggregate"]["tasks_succeeded"], 0)
            self.assertEqual(data["aggregate"]["unverified_raw_task_outcome_attempted"], 0)

    def test_missing_optional_artifacts_do_not_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"tasks_attempted": 0, "tasks_succeeded": 0})
            write_json(session / "state/summary.json", {})

            data = build(root / "sessions", root / "out")

            self.assertEqual(len(data["sessions"]), 1)
            self.assertFalse(data["sessions"][0]["work_summary"]["task_verification"]["all_verified"])
            self.assertEqual(data["gnome_history"][0]["values"], {})
            self.assertEqual(
                data["sessions"][0]["work_summary"]["headline"],
                "No detailed work signals captured",
            )


if __name__ == "__main__":
    unittest.main()
