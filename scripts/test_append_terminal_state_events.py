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
                None,
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
                None,
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
                None,
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

    def test_closes_run_when_model_completed_but_run_terminal_missing(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            write_event(events, "RunStarted", "agent-run")
            write_event(events, "ModelCallStarted", "agent-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "agent-run", {"model": "deepseek-v4-pro"})
            write_event(events, "CacheMetricsRecorded", "agent-run", {"hit_tokens": 100})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                None,
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

            self.assertEqual(result["completed_model_calls"], [])
            self.assertEqual(result["completed_runs"], ["agent-run"])
            self.assertEqual(
                sum(1 for row in rows if row["event_type"] == "ModelCallCompleted"),
                1,
            )
            run_done = [
                row
                for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "agent-run"
            ][0]
            self.assertEqual(run_done["payload"]["status"], "completed")
            self.assertEqual(run_done["payload"]["stage"], "assess")

    def test_scans_current_file_when_after_line_exceeds_reset_events_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "agent-run")
            write_event(events, "ModelCallStarted", "agent-run", {"model": "deepseek-v4-pro"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                99,
                None,
                "session-1",
                "trace-1",
                "task_01_attempt1",
                "completed",
                "completed",
                "agent_process_exited",
                "",
                "agent process exited with status 0",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]

            self.assertEqual(result["completed_model_calls"], ["agent-run"])
            self.assertEqual(result["completed_runs"], ["agent-run"])
            self.assertTrue(
                any(
                    row["event_type"] == "ModelCallCompleted" and row["run_id"] == "agent-run"
                    for row in rows
                )
            )
            self.assertTrue(
                any(
                    row["event_type"] == "RunCompleted" and row["run_id"] == "agent-run"
                    for row in rows
                )
            )

    def test_ambiguous_reset_scan_does_not_close_historical_open_runs(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "old-run-1")
            write_event(events, "ModelCallStarted", "old-run-1", {"model": "deepseek-v4-pro"})
            write_event(events, "RunStarted", "old-run-2")
            write_event(events, "ModelCallStarted", "old-run-2", {"model": "deepseek-v4-pro"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                99,
                None,
                "session-1",
                "trace-1",
                "task_03_attempt2",
                "completed",
                "completed",
                "agent_process_exited",
                "",
                "agent process exited with status 0",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]

            self.assertEqual(result["diagnostics"]["scope"], "ambiguous_reset_full_scan")
            self.assertEqual(result["diagnostics"]["ambiguous_open_run_count"], 2)
            self.assertEqual(result["completed_model_calls"], [])
            self.assertEqual(result["completed_runs"], [])
            self.assertFalse(any(row["event_type"] == "RunCompleted" for row in rows))
            self.assertFalse(any(row["event_type"] == "ModelCallCompleted" for row in rows))

    def test_fallback_after_line_closes_open_agent_run_and_session_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "session-run", {"phase": "session"})
            write_event(events, "RunStarted", "agent-run")
            write_event(events, "ModelCallStarted", "agent-run", {"model": "deepseek-v4-pro"})
            write_event(events, "FileEdited", "agent-run", {"path": "journals/JOURNAL.md"})
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                0,
                "session-1",
                "trace-1",
                "task_01_attempt1",
                "completed",
                "completed",
                "agent_process_exited",
                "",
                "agent process exited with status 0",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]

            self.assertEqual(result["diagnostics"]["scope"], "fallback_after_line")
            self.assertEqual(result["diagnostics"]["session_run_ignored_count"], 1)
            self.assertEqual(result["completed_model_calls"], ["agent-run"])
            self.assertEqual(result["completed_runs"], ["agent-run"])
            self.assertEqual(result["completed_session_runs"], ["session-run"])
            session_done = [
                row
                for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "session-run"
            ][0]
            self.assertEqual(session_done["payload"]["outcome"], "post_hoc_closed")


    def test_closes_session_scope_orphan_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            write_event(events, "RunStarted", "session-run", {"phase": "session"})
            write_event(events, "SessionStarted", "session-run", {"phase": "session"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                None,
                "session-1",
                "trace-1",
                "post_session",
                "completed",
                "completed",
                "post_hoc_closure",
                "",
                "session scope run was orphaned",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]

            self.assertEqual(result["completed_session_runs"], ["session-run"])
            self.assertEqual(result["completed_runs"], [])
            self.assertEqual(result["diagnostics"]["open_session_run_count"], 1)
            run_done = [
                row
                for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "session-run"
            ][0]
            self.assertEqual(run_done["payload"]["outcome"], "post_hoc_closed")
            self.assertEqual(run_done["payload"]["terminal_reason"], "post_hoc_closure")
            self.assertEqual(run_done["payload"]["status"], "completed")

    def test_does_not_double_close_completed_session_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            write_event(events, "RunStarted", "session-run", {"phase": "session"})
            write_event(events, "RunCompleted", "session-run", {"status": "completed", "phase": "session"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                None,
                "session-1",
                "trace-1",
                "post_session",
                "completed",
                "completed",
                "post_hoc_closure",
                "",
                "no orphan to close",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]

            self.assertEqual(result["completed_session_runs"], [])
            self.assertEqual(result["diagnostics"]["open_session_run_count"], 0)
            self.assertEqual(
                sum(1 for row in rows if row["event_type"] == "RunCompleted" and row["run_id"] == "session-run"),
                1,
            )

    def test_detects_and_closes_orphaned_run_from_previous_session(self):
        """Full-scan orphan detector closes an interrupted run that predates after_line.

        Simulates a GitHub Actions cancellation: a run has RunStarted +
        ModelCallStarted but the harness was killed before writing
        RunCompleted.  The orphaned run sits *before* after_line so the
        incremental scan ignores it, but the full-scan orphan detector
        identifies and closes it.
        """
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            # Write an orphaned run that was interrupted (has model activity
            # but no RunCompleted).
            write_event(events, "RunStarted", "orphaned-run")
            write_event(events, "ModelCallStarted", "orphaned-run", {"model": "deepseek-v4-pro"})
            write_event(events, "FileEdited", "orphaned-run", {"path": "src/main.rs"})
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            # Write a current run that completed normally.
            write_event(events, "RunStarted", "current-run")
            write_event(events, "ModelCallStarted", "current-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "current-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "current-run", {"status": "completed"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                None,
                "session-1",
                "trace-1",
                "post_hoc",
                "error",
                "error",
                "post_hoc_closure",
                "",
                "closing orphans",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]

            # The orphaned run should be detected and closed.
            orphan_diag = result["diagnostics"]["full_scan_orphan_diagnostics"]
            self.assertEqual(orphan_diag["full_scan_orphaned_runs"], 1)
            self.assertIn("orphaned-run", result["completed_runs"])
            # The already-completed current-run should not be double-closed.
            self.assertNotIn("current-run", result["completed_runs"])
            # Verify the terminal event payload.
            orphan_done = [
                row
                for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "orphaned-run"
            ][0]
            self.assertEqual(orphan_done["payload"]["status"], "error")
            self.assertEqual(orphan_done["payload"]["terminal_reason"], "orphaned_previous_session")
            self.assertEqual(orphan_done["payload"]["outcome"], "post_hoc_closed")
            # The original events + 1 orphan closure = 8 lines.
            self.assertEqual(len(rows), 8)

    def test_orphan_detector_skips_bare_run_started_without_model_calls(self):
        """A bare RunStarted without ModelCallStarted is not a real orphaned run.

        Some test fixtures and edge cases produce isolated RunStarted events
        that should not be treated as orphaned agent invocations.
        """
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "bare-run")
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            write_event(events, "RunStarted", "current-run")
            write_event(events, "ModelCallStarted", "current-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "current-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "current-run", {"status": "completed"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                None,
                "session-1",
                "trace-1",
                "post_hoc",
                "error",
                "error",
                "post_hoc_closure",
                "",
                "closing orphans",
            )
            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]

            # The bare run should NOT be closed — it has no model activity.
            self.assertNotIn("bare-run", result["completed_runs"])
            self.assertFalse(
                any(
                    row["event_type"] == "RunCompleted" and row["run_id"] == "bare-run"
                    for row in rows
                )
            )
            # The current-run should also not be touched.
            self.assertNotIn("current-run", result["completed_runs"])
            orphan_diag = result["diagnostics"]["full_scan_orphan_diagnostics"]
            self.assertEqual(orphan_diag["full_scan_orphaned_runs"], 0)


if __name__ == "__main__":
    unittest.main()
