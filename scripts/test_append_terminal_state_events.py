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

    def test_appends_failure_observed_for_error_completed_run(self):
        """A RunCompleted with error status and no FailureObserved gets one appended."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "error-run")
            write_event(events, "ModelCallStarted", "error-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "error-run", {"status": "error"})
            write_event(events, "RunStarted", "success-run")
            write_event(events, "ModelCallStarted", "success-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "success-run", {"status": "completed"})
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            # Write current session events so the full scan has something to skip.
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

            # error-run should get a FailureObserved.
            failure_diag = result["diagnostics"]["failure_observed_diagnostics"]
            self.assertEqual(failure_diag["error_completed_runs"], 1)
            self.assertEqual(failure_diag["missing_failure_observed"], 1)
            self.assertIn("error-run", result["failure_observed_appended"])

            # success-run should not.
            self.assertNotIn("success-run", result["failure_observed_appended"])

            # Verify the appended FailureObserved event payload.
            fo_rows = [
                row for row in rows
                if row["event_type"] == "FailureObserved" and row["run_id"] == "error-run"
            ]
            self.assertEqual(len(fo_rows), 1)
            fo = fo_rows[0]
            self.assertEqual(fo["actor"], "harness")
            self.assertIn("retroactive", fo["payload"])
            self.assertTrue(fo["payload"]["retroactive"])
            self.assertIn("error status 'error'", fo["payload"]["reason"])

    def test_skips_failure_observed_when_already_present(self):
        """A RunCompleted with error status that already has FailureObserved is not double-counted."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "error-run")
            write_event(events, "ModelCallStarted", "error-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "error-run", {"status": "error"})
            write_event(events, "FailureObserved", "error-run", {"reason": "original failure"})
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

            failure_diag = result["diagnostics"]["failure_observed_diagnostics"]
            self.assertEqual(failure_diag["error_completed_runs"], 1)
            self.assertEqual(failure_diag["failure_observed_runs"], 1)
            self.assertEqual(failure_diag["missing_failure_observed"], 0)
            self.assertEqual(result["failure_observed_appended"], [])

            # Only the original FailureObserved should exist.
            fo_rows = [
                row for row in rows
                if row["event_type"] == "FailureObserved" and row["run_id"] == "error-run"
            ]
            self.assertEqual(len(fo_rows), 1)

    def test_skips_retroactive_failure_observed_on_second_invocation(self):
        """A second invocation does not emit a duplicate retroactive FailureObserved."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "error-run")
            write_event(events, "ModelCallStarted", "error-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "error-run", {"status": "error"})
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            write_event(events, "RunStarted", "current-run")
            write_event(events, "ModelCallStarted", "current-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "current-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "current-run", {"status": "completed"})

            # First invocation: should emit retroactive FailureObserved for error-run.
            result1 = append_terminal_state_events.append_terminal_events(
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
            self.assertEqual(result1["failure_observed_appended"], ["error-run"])
            self.assertTrue(result1["diagnostics"]["failure_observed_diagnostics"]["missing_failure_observed"] >= 1)

            # Second invocation: should NOT emit another retroactive FailureObserved
            # for the same run, because the first invocation's retroactive event
            # is now in the events file.
            second_after_line = len(events.read_text(encoding="utf-8").splitlines())
            result2 = append_terminal_state_events.append_terminal_events(
                events,
                second_after_line,
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
            failure_diag2 = result2["diagnostics"]["failure_observed_diagnostics"]
            self.assertEqual(failure_diag2["missing_failure_observed"], 0)
            self.assertEqual(result2["failure_observed_appended"], [])

            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]
            fo_rows = [
                row for row in rows
                if row["event_type"] == "FailureObserved" and row["run_id"] == "error-run"
            ]
            self.assertEqual(len(fo_rows), 1, f"Expected 1 FailureObserved for error-run, got {len(fo_rows)}")
            self.assertTrue(fo_rows[0]["payload"].get("retroactive"),
                            "The FailureObserved should be marked retroactive")

    def test_skips_failure_observed_for_success_run(self):
        """A RunCompleted with status 'completed' does not trigger FailureObserved."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "success-run")
            write_event(events, "ModelCallStarted", "success-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "success-run", {"status": "completed"})
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

            failure_diag = result["diagnostics"]["failure_observed_diagnostics"]
            self.assertEqual(failure_diag["error_completed_runs"], 0)
            self.assertEqual(failure_diag["missing_failure_observed"], 0)
            self.assertEqual(result["failure_observed_appended"], [])

            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]
            fo_rows = [
                row for row in rows
                if row["event_type"] == "FailureObserved"
            ]
            self.assertEqual(len(fo_rows), 0)

    def test_closes_run_with_failure_observed_but_no_run_completed(self):
        """A run with FailureObserved but no RunCompleted gets RunCompleted appended."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "crashed-run")
            write_event(events, "ModelCallStarted", "crashed-run", {"model": "deepseek-v4-pro"})
            write_event(events, "FailureObserved", "crashed-run", {"reason": "panic", "error": "panicked at src/main.rs:42"})
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
            # The crashed-run should get a RunCompleted appended
            rc_rows = [
                row for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "crashed-run"
            ]
            self.assertEqual(len(rc_rows), 1)
            self.assertEqual(rc_rows[0]["payload"]["status"], "error")
            self.assertEqual(rc_rows[0]["payload"]["terminal_reason"], "orphaned_previous_session")
            self.assertEqual(rc_rows[0]["payload"]["outcome"], "post_hoc_closed")
            self.assertIn("crashed-run", result["completed_runs"])

            # Verify the diagnostics track the open_after_FailureObserved gap.
            fo_diag = result["diagnostics"]["failure_observed_no_completion_diagnostics"]
            self.assertEqual(fo_diag["failure_observed_runs"], 1)
            self.assertEqual(fo_diag["runs_with_failure_observed_no_completion"], 1)

    def test_failure_observed_alone_without_run_started_gets_closed_with_started(self):
        """A run with FailureObserved but no RunStarted gets retroactive RunStarted + RunCompleted."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "ModelCallStarted", "orphan-run", {"model": "deepseek-v4-pro"})
            write_event(events, "FailureObserved", "orphan-run", {"reason": "signal", "error": "process killed by signal"})
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            write_event(events, "RunStarted", "current-run")
            write_event(events, "ModelCallStarted", "current-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "current-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "current-run", {"status": "completed"})

            result = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                0,
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
            # Verify retroactive RunStarted was emitted before RunCompleted
            rs_rows = [
                row for row in rows
                if row["event_type"] == "RunStarted" and row["run_id"] == "orphan-run"
            ]
            self.assertEqual(len(rs_rows), 1)
            self.assertTrue(rs_rows[0].get("payload", {}).get("retroactive"))
            self.assertEqual(rs_rows[0]["payload"]["reason"], "retroactive: no RunStarted found for orphaned run")
            rc_rows = [
                row for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "orphan-run"
            ]
            self.assertEqual(len(rc_rows), 1)
            self.assertEqual(rc_rows[0]["payload"]["terminal_reason"], "orphaned_previous_session")

    def test_run_with_both_failure_observed_and_run_completed_does_not_double_close(self):
        """A run with both FailureObserved and RunCompleted gets no duplicate RunCompleted."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "closed-run")
            write_event(events, "ModelCallStarted", "closed-run", {"model": "deepseek-v4-pro"})
            write_event(events, "FailureObserved", "closed-run", {"reason": "panic", "error": "panicked"})
            write_event(events, "RunCompleted", "closed-run", {"status": "error"})
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

            # closed-run should NOT get a duplicate RunCompleted.
            rc_rows = [
                row for row in rows
                if row["event_type"] == "RunCompleted" and row["run_id"] == "closed-run"
            ]
            self.assertEqual(len(rc_rows), 1, "should not double-close a run that already has RunCompleted")

            # Diagnostics should report zero orphans.
            fo_diag = result["diagnostics"]["failure_observed_no_completion_diagnostics"]
            self.assertEqual(fo_diag["failure_observed_runs"], 1)
            self.assertEqual(fo_diag["runs_with_failure_observed_no_completion"], 0)

            # closed-run should NOT appear in completed_runs (it was already closed).
            self.assertNotIn("closed-run", result["completed_runs"])


    def test_retroactive_model_call_started_for_unmatched_completed(self):
        """Emit retroactive ModelCallStarted when ModelCallCompleted has no matching ModelCallStarted."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            # Run with ModelCallCompleted but NO ModelCallStarted
            write_event(events, "RunStarted", "orphan-model-run")
            write_event(events, "ModelCallCompleted", "orphan-model-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "orphan-model-run", {"status": "error"})

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

            # Should have emitted a retroactive ModelCallStarted
            retro_started = [
                row for row in rows
                if row["event_type"] == "ModelCallStarted" and row["run_id"] == "orphan-model-run"
            ]
            self.assertEqual(len(retro_started), 1)
            self.assertTrue(retro_started[0]["payload"].get("retroactive"))
            self.assertEqual(
                retro_started[0]["payload"].get("model_call_id"),
                "retroactive-orphan-model-run",
            )
            self.assertEqual(retro_started[0]["payload"].get("model"), "deepseek-v4-pro")
            self.assertEqual(retro_started[0]["actor"], "harness")

            # Diagnostics should reflect the retroactive action
            diag = result["diagnostics"]["model_call_started_diagnostics"]
            self.assertEqual(diag["model_call_completed_count"], 1)
            self.assertEqual(diag["model_call_started_count"], 0)
            self.assertEqual(diag["unmatched_model_call_completed_count"], 1)

            self.assertEqual(result["model_call_started_appended"], 1)

    def test_no_retroactive_model_call_started_when_all_matched(self):
        """Do not emit retroactive ModelCallStarted when all pairs are matched."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            write_event(events, "RunStarted", "matched-run")
            write_event(events, "ModelCallStarted", "matched-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "matched-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "matched-run", {"status": "completed"})

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

            # No new ModelCallStarted should have been added
            retro_started = [
                row for row in rows
                if row["event_type"] == "ModelCallStarted" and row["run_id"] == "matched-run"
            ]
            self.assertEqual(len(retro_started), 1)  # the original one, not retroactive

            diag = result["diagnostics"]["model_call_started_diagnostics"]
            self.assertEqual(diag["unmatched_model_call_completed_count"], 0)
            self.assertEqual(result["model_call_started_appended"], 0)

    def test_retroactive_model_call_started_skips_ambiguous_reset(self):
        """Skip retroactive ModelCallStarted when scope is ambiguous_reset_full_scan."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            # after_line exceeds file length -> triggers full scan
            after_line = 999
            # Two runs that are open (RunStarted but no RunCompleted) to
            # trigger the ambiguous_reset_full_scan guard (>1 visible open run).
            write_event(events, "RunStarted", "run-a")
            write_event(events, "ModelCallStarted", "run-a", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "run-a", {"model": "deepseek-v4-pro"})
            write_event(events, "RunStarted", "run-b")
            write_event(events, "ModelCallCompleted", "run-b", {"model": "deepseek-v4-pro"})

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

            diag = result["diagnostics"]["model_call_started_diagnostics"]
            self.assertTrue(diag.get("skipped"))
            self.assertEqual(diag["reason"], "ambiguous_reset_full_scan")

    def test_closes_orphaned_model_call_started_in_closed_run(self):
        """Retroactive ModelCallCompleted for ModelCallStarted without match in a closed run."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            # Historical run: fully closed at run level but has orphaned model call
            write_event(events, "RunStarted", "old-run")
            write_event(events, "ModelCallStarted", "old-run",
                        {"model_call_id": "mc-abc123", "model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "old-run", {"status": "completed"})
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            # Current session events (fully closed)
            write_event(events, "RunStarted", "current-run")
            write_event(events, "ModelCallStarted", "current-run",
                        {"model_call_id": "mc-current", "model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "current-run",
                        {"model_call_id": "mc-current", "model": "deepseek-v4-pro"})
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

            # Should have emitted a retroactive ModelCallCompleted for old-run
            retro_completed = [
                row for row in rows
                if row["event_type"] == "ModelCallCompleted" and row["run_id"] == "old-run"
            ]
            self.assertEqual(len(retro_completed), 1,
                             f"Expected 1 retroactive ModelCallCompleted, got {len(retro_completed)}")
            self.assertTrue(retro_completed[0]["payload"].get("retroactive"))
            self.assertEqual(
                retro_completed[0]["payload"].get("model_call_id"),
                "mc-abc123",
            )
            self.assertEqual(retro_completed[0]["payload"].get("model"), "deepseek-v4-pro")
            self.assertEqual(retro_completed[0]["payload"].get("status"), "interrupted")
            self.assertEqual(
                retro_completed[0]["payload"].get("terminal_reason"),
                "retroactive: ModelCallStarted orphaned — no ModelCallCompleted found",
            )
            self.assertEqual(retro_completed[0]["actor"], "harness")

            # Diagnostics should reflect the orphaned model call
            omc_diag = result["diagnostics"]["orphaned_model_call_diagnostics"]
            self.assertEqual(omc_diag["model_call_started_count"], 2)
            self.assertEqual(omc_diag["model_call_completed_count"], 1)
            self.assertEqual(omc_diag["orphaned_model_calls"], 1)

            # Should not have changed the current run
            current_completed = [
                row for row in rows
                if row["event_type"] == "ModelCallCompleted" and row["run_id"] == "current-run"
            ]
            self.assertEqual(len(current_completed), 1)  # only the original

    def test_no_retroactive_model_call_completed_when_all_matched(self):
        """Do not emit retroactive ModelCallCompleted when all model calls are matched."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            # Historical run: fully closed at both run and model levels
            write_event(events, "RunStarted", "old-run")
            write_event(events, "ModelCallStarted", "old-run",
                        {"model_call_id": "mc-matched", "model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "old-run",
                        {"model_call_id": "mc-matched", "model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "old-run", {"status": "completed"})
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            # Current session
            write_event(events, "RunStarted", "current-run")
            write_event(events, "ModelCallStarted", "current-run",
                        {"model_call_id": "mc-cur", "model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "current-run",
                        {"model_call_id": "mc-cur", "model": "deepseek-v4-pro"})
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

            # No new ModelCallCompleted for old-run (already has one)
            old_completed = [
                row for row in rows
                if row["event_type"] == "ModelCallCompleted" and row["run_id"] == "old-run"
            ]
            self.assertEqual(len(old_completed), 1)  # only the original

            omc_diag = result["diagnostics"]["orphaned_model_call_diagnostics"]
            self.assertEqual(omc_diag["orphaned_model_calls"], 0)

    def test_closes_orphaned_model_call_without_model_call_id(self):
        """Retroactive ModelCallCompleted for orphaned ModelCallStarted without model_call_id field."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            # Historical run: ModelCallStarted without model_call_id (test fixture style)
            write_event(events, "RunStarted", "old-run")
            write_event(events, "ModelCallStarted", "old-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "old-run", {"status": "completed"})
            after_line = len(events.read_text(encoding="utf-8").splitlines())

            # Current session
            write_event(events, "RunStarted", "current-run")
            write_event(events, "ModelCallStarted", "current-run",
                        {"model_call_id": "mc-cur", "model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "current-run",
                        {"model_call_id": "mc-cur", "model": "deepseek-v4-pro"})
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

            retro_completed = [
                row for row in rows
                if row["event_type"] == "ModelCallCompleted" and row["run_id"] == "old-run"
            ]
            self.assertEqual(len(retro_completed), 1)
            self.assertTrue(retro_completed[0]["payload"].get("retroactive"))
            # When the original ModelCallStarted had no model_call_id, the
            # retroactive ModelCallCompleted also omits model_call_id so
            # that find_orphaned_model_calls matches it by run_id on the
            # next scan (same key as the original started event).
            self.assertIsNone(
                retro_completed[0]["payload"].get("model_call_id"),
            )

    def test_orphaned_model_call_without_mcid_dedup_on_second_run(self):
        """Second janitor invocation writes zero retroactive ModelCallCompleted for same orphan.

        When the original ModelCallStarted has no model_call_id, the orphan
        detection keys by run_id.  The retroactive ModelCallCompleted must
        also omit model_call_id so it is matched by run_id on the next scan,
        preventing duplicate retroactive events.  (Same bug class as Day 139's
        FailureObserved dedup.)
        """
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            write_event(events, "RunStarted", "orphan-run")
            write_event(events, "ModelCallStarted", "orphan-run",
                        {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "orphan-run",
                        {"status": "error"})
            after_line = 0

            # First invocation: should append one retroactive ModelCallCompleted.
            result1 = append_terminal_state_events.append_terminal_events(
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
            self.assertEqual(result1["orphaned_model_calls_appended"], 1,
                             "First janitor run should append 1 retroactive ModelCallCompleted")

            # Second invocation: should append zero — the retroactive event
            # matches the orphaned ModelCallStarted by run_id.
            result2 = append_terminal_state_events.append_terminal_events(
                events,
                after_line,
                None,
                "session-2",
                "trace-2",
                "post_hoc",
                "error",
                "error",
                "post_hoc_closure",
                "",
                "closing orphans",
            )
            self.assertEqual(result2["orphaned_model_calls_appended"], 0,
                             "Second janitor run should append 0 retroactive ModelCallCompleted (dedup)")

            rows = [json.loads(line) for line in events.read_text(encoding="utf-8").splitlines()]
            retro_completed = [
                row for row in rows
                if row["event_type"] == "ModelCallCompleted"
                and row["run_id"] == "orphan-run"
                and row["payload"].get("retroactive")
            ]
            self.assertEqual(len(retro_completed), 1,
                             "Only one retroactive ModelCallCompleted should exist")

    def test_cancelled_run_gets_specific_failure_observed_reason(self):
        """Cancelled RunCompleted yields a specific cancellation reason in FailureObserved."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            write_event(events, "RunStarted", "cancelled-run")
            write_event(events, "ModelCallStarted", "cancelled-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "cancelled-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "cancelled-run", {"status": "cancelled"})

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

            fo_rows = [
                row for row in rows
                if row["event_type"] == "FailureObserved" and row["run_id"] == "cancelled-run"
            ]
            self.assertEqual(len(fo_rows), 1)
            reason = fo_rows[0]["payload"].get("reason", "")
            self.assertIn("cancelled by next hourly session", reason,
                          f"Expected cancellation-specific reason, got: {reason}")
            self.assertTrue(fo_rows[0]["payload"].get("retroactive"))

    def test_error_run_keeps_generic_failure_observed_reason(self):
        """Error RunCompleted keeps the generic reason in FailureObserved."""
        with tempfile.TemporaryDirectory() as tmp:
            events = Path(tmp) / "events.jsonl"
            after_line = 0
            write_event(events, "RunStarted", "error-run")
            write_event(events, "ModelCallStarted", "error-run", {"model": "deepseek-v4-pro"})
            write_event(events, "ModelCallCompleted", "error-run", {"model": "deepseek-v4-pro"})
            write_event(events, "RunCompleted", "error-run", {"status": "error"})

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

            fo_rows = [
                row for row in rows
                if row["event_type"] == "FailureObserved" and row["run_id"] == "error-run"
            ]
            self.assertEqual(len(fo_rows), 1)
            reason = fo_rows[0]["payload"].get("reason", "")
            self.assertIn("run completed with error status", reason)
            self.assertNotIn("cancelled by next hourly session", reason)


if __name__ == "__main__":
    unittest.main()
