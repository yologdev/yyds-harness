#!/usr/bin/env python3
"""Tests for scripts/replay_state_events.py."""

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from replay_state_events import replay  # noqa: E402


def write_events(path: Path, rows: list[object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            if isinstance(row, str):
                handle.write(row)
            else:
                handle.write(json.dumps(row, separators=(",", ":")))
            handle.write("\n")


class ReplayStateEvents(unittest.TestCase):
    def test_replays_session_events_in_stable_order(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_events(
                root / "sessions/day-2/state/events.jsonl",
                [{"event_id": "b", "event_type": "FailureObserved", "payload": {}}],
            )
            write_events(
                root / "sessions/day-1/state/events.jsonl",
                [{"event_id": "a", "event_type": "PatchProposed", "payload": {}}],
            )

            output = root / ".yoyo/state/events.jsonl"
            stats = replay(root / "sessions", output)

            self.assertEqual(stats["files_read"], 2)
            self.assertEqual(stats["events_written"], 2)
            lines = output.read_text(encoding="utf-8").splitlines()
            self.assertEqual([json.loads(line)["event_id"] for line in lines], ["a", "b"])

    def test_deduplicates_by_event_id_and_skips_malformed_lines(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_events(
                root / "sessions/day-1/state/events.jsonl",
                [
                    {"event_id": "same", "event_type": "PatchProposed", "payload": {}},
                    "{not json",
                    ["not", "an", "event"],
                ],
            )
            write_events(
                root / "sessions/day-2/state/events.jsonl",
                [{"event_id": "same", "event_type": "PatchApplied", "payload": {}}],
            )

            output = root / ".yoyo/state/events.jsonl"
            stats = replay(root / "sessions", output)

            self.assertEqual(stats["events_written"], 1)
            self.assertEqual(stats["duplicates_skipped"], 1)
            self.assertEqual(stats["malformed_skipped"], 2)
            self.assertEqual(len(output.read_text(encoding="utf-8").splitlines()), 1)

    def test_missing_sessions_dir_writes_empty_log(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / ".yoyo/state/events.jsonl"
            stats = replay(root / "missing", output)

            self.assertEqual(stats["files_read"], 0)
            self.assertEqual(stats["events_written"], 0)
            self.assertEqual(output.read_text(encoding="utf-8"), "")

    def test_replays_real_format_events_in_order_with_dedup(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            # Write two session files in the real state-event format
            write_events(
                root / "sessions/day-98-ts1/state/events.jsonl",
                [
                    {
                        "id": "evt-001",
                        "schema_version": 1,
                        "ts_ms": 1000,
                        "actor": {"kind": "system", "id": "harness"},
                        "kind": "RunStarted",
                        "payload": {"_yoyo": {"event_type": "RunStarted"}},
                    },
                    {
                        "id": "evt-002",
                        "schema_version": 1,
                        "ts_ms": 2000,
                        "actor": {"kind": "system", "id": "harness"},
                        "kind": "ToolCallStarted",
                        "payload": {"_yoyo": {"event_type": "ToolCallStarted"}, "tool_name": "bash"},
                    },
                    {
                        "id": "evt-003",
                        "schema_version": 1,
                        "ts_ms": 3000,
                        "actor": {"kind": "system", "id": "harness"},
                        "kind": "RunCompleted",
                        "payload": {"_yoyo": {"event_type": "RunCompleted"}, "status": "ok"},
                    },
                ],
            )
            write_events(
                root / "sessions/day-98-ts2/state/events.jsonl",
                [
                    {
                        "id": "evt-004",
                        "schema_version": 1,
                        "ts_ms": 4000,
                        "actor": {"kind": "system", "id": "harness"},
                        "kind": "FileEdited",
                        "payload": {"_yoyo": {"event_type": "FileEdited"}, "path": "src/main.rs"},
                    },
                    # Duplicate event id — should be skipped
                    {
                        "id": "evt-002",
                        "schema_version": 1,
                        "ts_ms": 2000,
                        "actor": {"kind": "system", "id": "harness"},
                        "kind": "ToolCallStarted",
                        "payload": {"_yoyo": {"event_type": "ToolCallStarted"}, "tool_name": "bash"},
                    },
                ],
            )

            output = root / "replayed.jsonl"
            stats = replay(root / "sessions", output)

            # Verify stats
            self.assertEqual(stats["files_read"], 2)
            self.assertEqual(stats["lines_read"], 5)
            self.assertEqual(stats["events_written"], 4)
            self.assertEqual(stats["duplicates_skipped"], 1)
            self.assertEqual(stats["malformed_skipped"], 0)

            # Verify output line count
            lines = output.read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(lines), 4)

            # Verify each line is valid JSON
            events = [json.loads(line) for line in lines]
            for evt in events:
                self.assertIsInstance(evt, dict)

            # Verify events preserve sorted session order:
            # day-98-ts1 (evt-001, evt-002, evt-003) then day-98-ts2 (evt-004)
            event_ids = [evt["id"] for evt in events]
            self.assertEqual(event_ids, ["evt-001", "evt-002", "evt-003", "evt-004"])


if __name__ == "__main__":
    unittest.main()
