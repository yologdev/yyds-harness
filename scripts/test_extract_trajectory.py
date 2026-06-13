#!/usr/bin/env python3
"""Tests for trajectory extraction evidence summaries."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import extract_trajectory  # noqa: E402


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


class ExtractTrajectoryTests(unittest.TestCase):
    def test_recent_outcomes_sort_by_outcome_timestamp_not_file_mtime(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            older = audit_dir / "day-1"
            newer = audit_dir / "day-2"
            write_json(
                older / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z", "tasks_attempted": 0},
            )
            write_json(
                newer / "outcome.json",
                {"day": 2, "ts": "2026-01-02T00:00:00Z", "tasks_attempted": 0},
            )
            os.utime(newer / "outcome.json", (1, 1))
            os.utime(older / "outcome.json", (2, 2))

            sessions = extract_trajectory.load_recent_session_outcomes(audit_dir)

            self.assertEqual(sessions[0][0].name, "day-2")
            self.assertEqual(sessions[1][0].name, "day-1")

    def test_log_feedback_uses_session_timestamp_for_latest_feedback(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            older = audit_dir / "day-1"
            newer = audit_dir / "day-2"
            write_json(older / "outcome.json", {"ts": "2026-01-01T00:00:00Z"})
            write_json(newer / "outcome.json", {"ts": "2026-01-02T00:00:00Z"})
            write_json(older / "log_feedback.json", {"metrics": {"coding_log_score": 0.1}})
            write_json(newer / "log_feedback.json", {"metrics": {"coding_log_score": 0.9}})
            os.utime(newer / "log_feedback.json", (1, 1))
            os.utime(older / "log_feedback.json", (2, 2))

            feedbacks = extract_trajectory.load_log_feedback(audit_dir)

            self.assertEqual(feedbacks[0]["metrics"]["coding_log_score"], 0.9)

    def test_log_feedback_prefers_corrected_lessons(self) -> None:
        rendered = extract_trajectory.render_log_feedback(
            [
                {
                    "metrics": {
                        "coding_log_score": 0.9,
                        "coding_log_confidence": 1.0,
                        "recurring_failure_count": 1,
                        "state_capture_coverage": 1.0,
                    },
                    "top_lessons": [
                        {
                            "fingerprint": "seeded task was contradicted by fresh assessment evidence",
                            "action": "replace stale seed",
                        }
                    ],
                }
            ],
            [
                {
                    "fingerprint": "DeepSeek model call lifecycle was incomplete",
                    "action": "close model-call lifecycle events",
                }
            ],
        )

        self.assertIn("Corrected top lessons for next run:", rendered)
        self.assertIn("DeepSeek model call lifecycle was incomplete", rendered)
        self.assertNotIn("seeded task was contradicted", rendered)

    def test_raw_success_without_task_artifacts_is_not_rendered_as_verified(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 3,
                    "tasks_succeeded": 3,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )

            rendered = extract_trajectory.render_outcomes(
                extract_trajectory.load_recent_session_outcomes(audit_dir)
            )

            self.assertIn("tasks 3/3", rendered)
            self.assertIn("raw outcome 3/3 lacks strict task evidence", rendered)
            self.assertNotIn("tasks 3/3 ✅", rendered)

    def test_strict_verified_task_artifacts_are_rendered_as_verified(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {
                    "tasks": [
                        {
                            "task_id": "task_01",
                            "title": "Tighten trajectory evidence",
                            "files": ["src/lib.rs"],
                        }
                    ]
                },
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "planned_files": ["src/lib.rs"],
                    "source_files": ["src/lib.rs"],
                    "commit_shas": ["abc123"],
                },
            )
            write_json(
                session / "tasks/task_01/eval_attempt_1.json",
                {"task_id": "task_01", "status": "pass", "verdict": "Verdict: PASS"},
            )

            rendered = extract_trajectory.render_outcomes(
                extract_trajectory.load_recent_session_outcomes(audit_dir)
            )

            self.assertIn("tasks 1/1 ✅", rendered)
            self.assertIn("1/1 strict verified; build OK, tests OK", rendered)

    def test_unverified_task_artifacts_show_strict_failure_reasons(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 1,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {"tasks": [{"task_id": "task_01", "files": ["src/lib.rs"]}]},
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "planned_files": ["src/lib.rs"],
                    "source_files": ["docs/readme.md"],
                    "commit_shas": [],
                },
            )

            rendered = extract_trajectory.render_outcomes(
                extract_trajectory.load_recent_session_outcomes(audit_dir)
            )

            self.assertIn("tasks 1/1 ⚠️", rendered)
            self.assertIn("0/1 strict verified", rendered)
            self.assertIn("raw outcome 1/1", rendered)
            self.assertIn("no planned-file overlap", rendered)
            self.assertIn("no passing verifier", rendered)

    def test_structured_state_snapshot_surfaces_claims_task_states_and_tool_failures(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {
                    "day": 1,
                    "ts": "2026-01-01T00:00:00Z",
                    "tasks_attempted": 1,
                    "tasks_succeeded": 0,
                    "build_ok": True,
                    "test_ok": True,
                    "reverted": False,
                },
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {"tool_error_count": 1},
                    "gnome_keys": ["tool_error_count"],
                },
            )
            write_json(
                session / "tasks/manifest.json",
                {"tasks": [{"task_id": "task_01", "title": "Fix search", "files": ["src/lib.rs"]}]},
            )
            write_json(
                session / "tasks/task_01/outcome.json",
                {
                    "task_id": "task_01",
                    "status": "reverted",
                    "planned_files": ["src/lib.rs"],
                    "touched_files": ["src/lib.rs"],
                    "source_files": ["src/lib.rs"],
                    "commit_shas": [],
                },
            )
            transcript_dir = session / "transcripts"
            transcript_dir.mkdir(parents=True)
            transcript_dir.joinpath("task_01_attempt1.log").write_text(
                "  ▶ search 'fn handle_run\\(' in src/commands.rs ✗ (17ms)\n",
                encoding="utf-8",
            )
            events = [
                {"kind": "RunStarted", "run_id": "run-open", "payload": {"status": "started"}},
                {
                    "kind": "ModelCallStarted",
                    "run_id": "run-model-open",
                    "payload": {"model": "deepseek-v4-pro"},
                },
            ]
            state_dir = session / "state"
            state_dir.mkdir(parents=True, exist_ok=True)
            state_dir.joinpath("events.jsonl").write_text(
                "\n".join(json.dumps(event) for event in events) + "\n",
                encoding="utf-8",
            )

            rendered = extract_trajectory.render_structured_state_snapshot(audit_dir)

            self.assertIn("## Structured state snapshot", rendered)
            self.assertIn("claims:", rendered)
            self.assertIn("deepseek_model_call_lifecycle_balanced", rendered)
            self.assertIn("latest=day-1", rendered)
            self.assertIn("task states:", rendered)
            self.assertIn("unlanded_source_edits=1", rendered)
            self.assertIn("tool failures:", rendered)
            self.assertIn("search_tool_error=1", rendered)
            self.assertIn("lifecycle gnomes:", rendered)
            self.assertIn("state_run_incomplete_count=1", rendered)
            self.assertIn("deepseek_model_call_incomplete_count=1", rendered)

    def test_graph_suggestions_surface_lifecycle_pressure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            audit_dir = Path(tmp)
            session = audit_dir / "day-1"
            write_json(
                session / "outcome.json",
                {"day": 1, "ts": "2026-01-01T00:00:00Z"},
            )
            write_json(
                session / "state/summary.json",
                {
                    "latest_gnomes": {
                        "deepseek_model_call_incomplete_count": 1,
                        "search_error_count": 3,
                    }
                },
            )

            rendered = extract_trajectory.render_graph_suggestions(audit_dir)

            self.assertIn("## Graph-derived next-task pressure", rendered)
            self.assertIn("Close yyds state and model lifecycle gaps", rendered)
            self.assertIn("deepseek_model_call_incomplete_count=1", rendered)


if __name__ == "__main__":
    unittest.main()
