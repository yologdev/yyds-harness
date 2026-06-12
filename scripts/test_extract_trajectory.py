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


if __name__ == "__main__":
    unittest.main()
