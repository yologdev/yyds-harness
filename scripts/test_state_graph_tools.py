#!/usr/bin/env python3
"""Tests for scripts/state_graph_tools.py."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import state_graph_tools  # noqa: E402


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


class StateGraphTools(unittest.TestCase):
    def test_replay_check_and_causal_chain(self):
        with tempfile.TemporaryDirectory() as tmp:
            session = Path(tmp) / "sessions/day-1"
            write_jsonl(
                session / "state/events.jsonl",
                [
                    {"event_type": "DecisionRecorded", "payload": {"decision": "tasks_selected"}},
                    {"event_type": "PatchEvaluated", "payload": {"suite": "log-feedback"}},
                ],
            )
            write_json(
                session / "state/summary.json",
                {
                    "event_count": 2,
                    "event_counts": {"DecisionRecorded": 1, "PatchEvaluated": 1},
                    "latest_gnomes": {
                        "task_artifact_coverage": 1.0,
                        "evaluator_unverified_count": 1,
                        "max_task_turn_count": 30,
                    },
                    "task_lineage": [
                        {
                            "task_id": "task_01",
                            "task_number": 1,
                            "task_title": "Improve eval",
                            "status": "completed",
                            "source_files": ["src/eval.rs"],
                            "touched_files": ["src/eval.rs"],
                            "commit_shas": ["abcdef123"],
                            "eval": {"verdict": "PASS", "reason": "ok"},
                            "gnome_deltas": {"coding_log_score": 0.1},
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
                            "title": "Improve eval",
                            "files": ["src/eval.rs"],
                            "artifact_path": "tasks/task_01/task.md",
                        }
                    ],
                },
            )
            (session / "tasks/task_01").mkdir(parents=True)
            (session / "tasks/task_01/task.md").write_text("Title: Improve eval\n", encoding="utf-8")
            write_json(session / "tasks/task_01/decision.json", {"task_id": "task_01"})
            write_json(session / "tasks/task_01/eval_attempt_1.json", {"status": "pass"})

            replay = state_graph_tools.replay_check(session.parent)
            self.assertEqual(replay["state_replay_integrity_rate"], 1.0)
            self.assertTrue(replay["sessions"][0]["ok"])

            chains = state_graph_tools.build_causal_chains(session)
            self.assertEqual(chains[0]["task_id"], "task_01")
            self.assertEqual(chains[0]["planned_files"], ["src/eval.rs"])
            self.assertEqual(chains[0]["source_files"], ["src/eval.rs"])
            self.assertEqual(chains[0]["commit_shas"], ["abcdef123"])
            self.assertEqual(chains[0]["eval_verdict"], "PASS")

            suggestions = state_graph_tools.evolution_suggestions(session)
            titles = [item["title"] for item in suggestions]
            self.assertIn("Bound evaluator checks so verdicts are not skipped", titles)
            self.assertIn("Split high-turn tasks into narrower plans", titles)

    def test_compare_previous_session(self):
        with tempfile.TemporaryDirectory() as tmp:
            sessions = Path(tmp) / "sessions"
            base = sessions / "day-1"
            cand = sessions / "day-2"
            write_json(base / "outcome.json", {"day": 1, "ts": "2026-01-01T00:00:00Z", "tasks_succeeded": 1})
            write_json(cand / "outcome.json", {"day": 2, "ts": "2026-01-02T00:00:00Z", "tasks_succeeded": 2})
            write_json(base / "state/summary.json", {"latest_gnomes": {"coding_log_score": 0.7}})
            write_json(cand / "state/summary.json", {"latest_gnomes": {"coding_log_score": 0.9}})

            comparison = state_graph_tools.compare_sessions(sessions, "previous", "latest")
            self.assertEqual(comparison["baseline_session"], "day-1")
            self.assertEqual(comparison["candidate_session"], "day-2")
            self.assertAlmostEqual(comparison["gnome_deltas"]["coding_log_score"]["delta"], 0.2)


if __name__ == "__main__":
    unittest.main()
