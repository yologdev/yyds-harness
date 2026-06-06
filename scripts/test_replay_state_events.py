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


if __name__ == "__main__":
    unittest.main()
