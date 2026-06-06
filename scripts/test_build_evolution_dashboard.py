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
                    "event_count": 4,
                    "event_counts": {"FileEdited": 1, "CommandStarted": 1},
                    "latest_gnomes": {"coding_log_score": 0.8, "coding_log_available": True},
                    "gnome_keys": ["coding_log_score", "coding_log_available"],
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
                            },
                        }
                    ],
                    "patches": [{"patch_id": "patch-1"}],
                    "decisions": [{"decision": "promote", "eligible": True}],
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
            (session / "transcripts/task_01_attempt1.log").write_text("task\n", encoding="utf-8")

            data = build(root / "sessions", root / "out")

            self.assertEqual(data["schema_version"], 2)
            self.assertEqual(data["gnome_numeric_keys"], ["coding_log_score"])
            self.assertEqual(data["gnome_history"][0]["values"], {"coding_log_score": 0.8})

            work = data["sessions"][0]["work_summary"]
            self.assertIn("2/2 tasks completed", work["headline"])
            self.assertEqual(work["edited_files"], ["scripts/build_evolution_dashboard.py"])
            self.assertEqual(work["commands"], ["cargo test"])
            self.assertEqual(work["transcripts"]["phase_counts"], {"plan": 1, "task": 1})
            self.assertEqual(work["patch_count"], 1)
            self.assertEqual(work["decision_count"], 1)

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
            (repo / "src/lib.rs").write_text("pub fn ok() {}\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "src/lib.rs"], check=True)
            subprocess.run(
                ["git", "-C", str(repo), "commit", "-m", "Day 1 (00:00): implement source"],
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
            self.assertEqual(work["commits"][0]["subject"], "Day 1 (00:00): implement source")

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
