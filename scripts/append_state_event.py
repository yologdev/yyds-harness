#!/usr/bin/env python3
"""Append a compact yoagent-state-compatible event from shell harness code."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
import time
from pathlib import Path
from typing import Any


def load_payload(raw: str) -> dict[str, Any]:
    if not raw:
        return {}
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid --payload-json: {exc}") from exc
    if not isinstance(value, dict):
        raise SystemExit("--payload-json must decode to an object")
    return value


def append_event(
    events_path: Path,
    event_type: str,
    actor: str,
    run_id: str,
    session_id: str,
    trace_id: str,
    payload: dict[str, Any],
) -> dict[str, Any]:
    now_ms = int(time.time() * 1000)
    seed = json.dumps(
        {
            "actor": actor,
            "event_type": event_type,
            "payload": payload,
            "pid": os.getpid(),
            "run_id": run_id,
            "session_id": session_id,
            "time": now_ms,
            "trace_id": trace_id,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
    event = {
        "event_id": f"evt-harness-{hashlib.sha1(seed.encode()).hexdigest()[:16]}",
        "event_type": event_type,
        "schema_version": 1,
        "timestamp_ms": now_ms,
        "actor": actor,
        "run_id": run_id or None,
        "session_id": session_id or None,
        "trace_id": trace_id,
        "parent_event_ids": [],
        "payload": payload,
    }
    events_path.parent.mkdir(parents=True, exist_ok=True)
    with events_path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(event, sort_keys=True, separators=(",", ":")) + "\n")
    return event


def run_self_tests() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "events.jsonl"
        event = append_event(
            path,
            "RunStarted",
            "harness",
            "run-1",
            "session-1",
            "trace-1",
            {"phase": "session"},
        )
        rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]
        assert len(rows) == 1
        assert rows[0]["event_id"] == event["event_id"]
        assert rows[0]["event_type"] == "RunStarted"
        assert rows[0]["payload"]["phase"] == "session"
    print("append_state_event self-tests passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", required=True, type=Path)
    parser.add_argument("--event-type", required=True)
    parser.add_argument("--actor", default="harness")
    parser.add_argument("--run-id", default="")
    parser.add_argument("--session-id", default="")
    parser.add_argument("--trace-id", default="")
    parser.add_argument("--payload-json", default="{}")
    parser.add_argument("--test", action="store_true")
    args = parser.parse_args()
    if args.test:
        return run_self_tests()
    append_event(
        args.events,
        args.event_type,
        args.actor,
        args.run_id,
        args.session_id,
        args.trace_id,
        load_payload(args.payload_json),
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
