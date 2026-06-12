#!/usr/bin/env python3
"""Tests for scripts/append_terminal_state_events.py."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import append_terminal_state_events  # noqa: E402


def write_event(path: Path, event_type: str, run_id: str, payload: dict[str, object] | None = None) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    line_count = len(path.read_text(encoding="utf-8").splitlines()) if path.exists() else 0
    row = {
        "event_id": f"evt-{event_type}-{run_id}-{line_count}",
        "event_type": event_type,
        "run_id": run_id,
        "payload": payload or {},
    }
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row, separators=(",", ":")) + "\n")


class AppendTerminalStateEvents(unittest.TestCase):
    def test_closes_open_model_and_run_after_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "prior-run")
            after_line = len(events.read_text(encoding="utf-8").splitlines())
            write_event(events, "RunStarted", "agent-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallStarted", "agent-run", {"model": "deepseek-v4-pro"})
            write_event(events, "FileEdited", "agent-run", {"path": "journals/JOURNAL.md"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                "session-1",
                "trace-1",
                "task_01_attempt1",
                "error",
                "timeout",
                "timeout_after_seconds",
                "timeout",
                "agent timed out",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]

            self.assertEqual(result["completed_model_calls"], ["agent-run"])
            self.assertEqual(result["completed_runs"], ["agent-run"])
            self.assertFalse(
                any(
                    row["event_type"] == "RunCompleted" and row["run_id"] == "prior-run"
                    for row in rows
                )
            )
            model_done = [
                row
                for row in rows
                if row["event_type"] == "ModelCallCompleted" and row["run_id"] == "agent-run"
            ][0]
            run_done = [
                row
                for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "agent-run"
            ][0]
            self.assertEqual(model_done["actor"], "yoyo")
            self.assertEqual(model_done["payload"]["status"], "timeout")
            self.assertEqual(model_done["payload"]["model"], "deepseek-v4-pro")
            self.assertEqual(model_done["payload"]["terminal_reason"], "timeout_after_seconds")
            self.assertEqual(run_done["payload"]["status"], "error")
            self.assertEqual(run_done["payload"]["error_detail"], "agent timed out")

    def test_leaves_already_completed_invocation_unchanged(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            write_event(events, "RunStarted", "agent-run")
            write_event(events, "ModelCallStarted", "agent-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "agent-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "agent-run", {"status": "completed"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                "session-1",
                "trace-1",
                "task_01_attempt1",
                "error",
                "timeout",
                "timeout_after_seconds",
                "timeout",
                "agent timed out",
            )

            self.assertEqual(result["completed_model_calls"], [])
            self.assertEqual(result["completed_runs"], [])
            self.assertEqual(len(events.read_text(encoding="utf-8").splitlines()), 4)

    def test_closes_open_lifecycle_after_normal_agent_exit(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            write_event(events, "RunStarted", "agent-run")
            write_event(events, "ModelCallStarted", "agent-run", {"model": "deepseek-v4-pro"})
            write_event(events, "FileEdited", "agent-run", {"path": "journals/JOURNAL.md"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                "session-1",
                "trace-1",
                "assess",
                "completed",
                "completed",
                "agent_process_exited",
                "",
                "agent process exited with status 0",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]
            model_done = [
                row
                for row in rows
                if row["event_type"] == "ModelCallCompleted" and row["run_id"] == "agent-run"
            ][0]
            run_done = [
                row
                for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "agent-run"
            ][0]

            self.assertEqual(result["completed_model_calls"], ["agent-run"])
            self.assertEqual(result["completed_runs"], ["agent-run"])
            self.assertEqual(model_done["payload"]["status"], "completed")
            self.assertEqual(model_done["payload"]["terminal_reason"], "agent_process_exited")
            self.assertEqual(run_done["payload"]["status"], "completed")
            self.assertEqual(run_done["payload"]["stage"], "assess")


if __name__ == "__main__":
    unittest.main()
