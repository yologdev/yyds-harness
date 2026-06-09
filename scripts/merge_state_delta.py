#!/usr/bin/env python3
"""Merge the current live yoagent-state delta into a session evidence file."""

from __future__ import annotations

import argparse
import hashlib
import json
import tempfile
from pathlib import Path
from typing import Any


def load_jsonl(path: Path) -> list[tuple[dict[str, Any], str]]:
    if not path.is_file():
        return []
    rows: list[tuple[dict[str, Any], str]] = []
    with path.open(encoding="utf-8", errors="replace") as handle:
        for raw in handle:
            line = raw.strip()
            if not line:
                continue
            try:
                value = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(value, dict):
                rows.append((value, line))
    return rows


def event_key(event: dict[str, Any], raw: str) -> str:
    event_id = event.get("event_id")
    if isinstance(event_id, str) and event_id:
        return event_id
    return "raw:" + hashlib.sha1(raw.encode("utf-8", errors="replace")).hexdigest()


def merge_delta(live_path: Path, session_path: Path, base_lines: int) -> dict[str, int]:
    base_lines = max(base_lines, 0)
    session_rows = load_jsonl(session_path)
    seen = {event_key(event, raw) for event, raw in session_rows}

    live_rows = load_jsonl(live_path)
    delta_rows = live_rows[base_lines:]
    merged_rows = list(session_rows)
    added = 0
    skipped_duplicate = 0
    for event, raw in delta_rows:
        key = event_key(event, raw)
        if key in seen:
            skipped_duplicate += 1
            continue
        seen.add(key)
        merged_rows.append((event, raw))
        added += 1

    session_path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        "w",
        encoding="utf-8",
        dir=str(session_path.parent),
        delete=False,
    ) as tmp:
        tmp_path = Path(tmp.name)
        for event, _raw in merged_rows:
            tmp.write(json.dumps(event, sort_keys=True, separators=(",", ":")) + "\n")
    tmp_path.replace(session_path)

    return {
        "live_events": len(live_rows),
        "base_lines": base_lines,
        "delta_events": len(delta_rows),
        "session_events_before": len(session_rows),
        "session_events_after": len(merged_rows),
        "added": added,
        "skipped_duplicate": skipped_duplicate,
    }


def run_self_tests() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        live = root / "live.jsonl"
        session = root / "session.jsonl"
        live.write_text(
            "\n".join(
                [
                    json.dumps({"event_id": "old", "event_type": "RunStarted"}),
                    json.dumps({"event_id": "dup", "event_type": "RunStarted"}),
                    json.dumps({"event_id": "tool", "event_type": "CommandStarted"}),
                    json.dumps({"event_id": "tool-done", "event_type": "CommandCompleted"}),
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        session.write_text(
            json.dumps({"event_id": "dup", "event_type": "RunStarted"}) + "\n",
            encoding="utf-8",
        )
        stats = merge_delta(live, session, 1)
        rows = [json.loads(line) for line in session.read_text(encoding="utf-8").splitlines()]
        assert stats["added"] == 2, stats
        assert stats["skipped_duplicate"] == 1, stats
        assert [row["event_id"] for row in rows] == ["dup", "tool", "tool-done"]
    print("merge_state_delta self-tests passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live", type=Path)
    parser.add_argument("--session", type=Path)
    parser.add_argument("--base-lines", type=int, default=0)
    parser.add_argument("--test", action="store_true")
    args = parser.parse_args()
    if args.test:
        return run_self_tests()
    if args.live is None:
        parser.error("--live is required unless --test is set")
    if args.session is None:
        parser.error("--session is required unless --test is set")
    stats = merge_delta(args.live, args.session, args.base_lines)
    print(json.dumps(stats, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
