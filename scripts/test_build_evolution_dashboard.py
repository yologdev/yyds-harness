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

from build_evolution_dashboard import build  # noqa: E402


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def write_events(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


class BuildEvolutionDashboard(unittest.TestCase):
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
                    "evaluator_unverified_count",
                    "evolution_friction_count",
                    "session_success_rate",
                    "task_success_rate",
                ],
            )
            self.assertEqual(
                data["gnome_history"][0]["values"],
                {
                    "coding_log_score": 0.8,
                    "evaluator_unverified_count": 0.0,
                    "evolution_friction_count": 2.0,
                    "session_success_rate": 1.0,
                    "task_success_rate": 1.0,
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
            self.assertEqual(data["sessions"][0]["trace_quality"]["status"], "full")

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

    def test_missing_optional_artifacts_do_not_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            session = root / "sessions/day-1"
            write_json(session / "outcome.json", {"tasks_attempted": 0, "tasks_succeeded": 0})
            write_json(session / "state/summary.json", {})

            data = build(root / "sessions", root / "out")

            self.assertEqual(len(data["sessions"]), 1)
            self.assertEqual(data["gnome_history"][0]["values"], {})
            self.assertEqual(
                data["sessions"][0]["work_summary"]["headline"],
                "No detailed work signals captured",
            )


if __name__ == "__main__":
    unittest.main()
