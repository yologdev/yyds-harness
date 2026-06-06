#!/usr/bin/env python3
"""Replay prior audit-log state JSONL into the live yoagent-state event log.

The audit-log branch stores each session's state delta under:

    sessions/<session-id>/state/events.jsonl

This script rebuilds the live `.yoyo/state/events.jsonl` from those durable
JSONL deltas. SQLite remains a generated projection; callers should run
`yoyo state project --rebuild` after replaying.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tempfile
from pathlib import Path
from typing import Any


def event_key(value: dict[str, Any], raw_line: str) -> str:
    event_id = value.get("event_id") or value.get("id")
    if isinstance(event_id, str) and event_id:
        return f"id:{event_id}"
    digest = hashlib.sha256(raw_line.encode("utf-8")).hexdigest()
    return f"sha256:{digest}"


def iter_event_files(sessions_dir: Path) -> list[Path]:
    if not sessions_dir.is_dir():
        return []
    return sorted(
        path
        for path in sessions_dir.glob("*/state/events.jsonl")
        if path.is_file()
    )


def replay(sessions_dir: Path, output: Path) -> dict[str, Any]:
    seen: set[str] = set()
    files = iter_event_files(sessions_dir)
    stats: dict[str, Any] = {
        "sessions_dir": str(sessions_dir),
        "output": str(output),
        "files_read": len(files),
        "lines_read": 0,
        "events_written": 0,
        "duplicates_skipped": 0,
        "malformed_skipped": 0,
    }

    output.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(
        prefix=f".{output.name}.",
        suffix=".tmp",
        dir=str(output.parent),
        text=True,
    )
    tmp_path = Path(tmp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            for path in files:
                with path.open(encoding="utf-8", errors="replace") as source:
                    for line in source:
                        text = line.strip()
                        if not text:
                            continue
                        stats["lines_read"] += 1
                        try:
                            value = json.loads(text)
                        except json.JSONDecodeError:
                            stats["malformed_skipped"] += 1
                            continue
                        if not isinstance(value, dict):
                            stats["malformed_skipped"] += 1
                            continue
                        key = event_key(value, text)
                        if key in seen:
                            stats["duplicates_skipped"] += 1
                            continue
                        seen.add(key)
                        handle.write(text)
                        handle.write("\n")
                        stats["events_written"] += 1
        tmp_path.replace(output)
    except Exception:
        try:
            tmp_path.unlink()
        except OSError:
            pass
        raise
    return stats


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sessions-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--manifest", type=Path)
    args = parser.parse_args()

    stats = replay(args.sessions_dir, args.output)
    if args.manifest:
        args.manifest.parent.mkdir(parents=True, exist_ok=True)
        with args.manifest.open("w", encoding="utf-8") as handle:
            json.dump(stats, handle, indent=2, sort_keys=True)
            handle.write("\n")
    print(
        "state replay: "
        f"{stats['events_written']} events from {stats['files_read']} files "
        f"({stats['duplicates_skipped']} duplicate, "
        f"{stats['malformed_skipped']} malformed skipped)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
