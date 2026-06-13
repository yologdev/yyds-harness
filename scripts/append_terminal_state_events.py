#!/usr/bin/env python3
"""Append terminal lifecycle events for interrupted yoyo agent invocations."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from append_state_event import append_event


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not path.is_file():
        return rows
    with path.open(encoding="utf-8", errors="replace") as handle:
        for line in handle:
            text = line.strip()
            if not text:
                continue
            try:
                value = json.loads(text)
            except json.JSONDecodeError:
                continue
            if isinstance(value, dict):
                rows.append(value)
    return rows


def event_type(event: dict[str, Any]) -> str:
    value = event.get("event_type") or event.get("kind")
    if isinstance(value, str):
        return value
    payload = event.get("payload")
    if isinstance(payload, dict):
        meta = payload.get("_yoyo")
        if isinstance(meta, dict) and isinstance(meta.get("event_type"), str):
            return meta["event_type"]
    return ""


def payload(event: dict[str, Any]) -> dict[str, Any]:
    value = event.get("payload")
    if not isinstance(value, dict):
        return {}
    wrapped = value.get("value")
    if set(value.keys()).issubset({"_yoyo", "value"}) and isinstance(wrapped, dict):
        return wrapped
    return value


def run_id(event: dict[str, Any], data: dict[str, Any]) -> str | None:
    for value in (
        event.get("run_id"),
        (event.get("payload") or {}).get("_yoyo", {}).get("run_id")
        if isinstance(event.get("payload"), dict) and isinstance((event.get("payload") or {}).get("_yoyo"), dict)
        else None,
        data.get("run_id"),
        data.get("_yoyo", {}).get("run_id") if isinstance(data.get("_yoyo"), dict) else None,
    ):
        if isinstance(value, str) and value:
            return value
    return None


def is_session_run(data: dict[str, Any]) -> bool:
    return data.get("phase") == "session"


def lifecycle_for_scope(
    events: list[dict[str, Any]],
    scan_start: int,
) -> tuple[dict[str, dict[str, Any]], set[str], dict[str, Any]]:
    scoped = events[max(scan_start, 0) :]
    lifecycle_start_count = 0
    lifecycle_completion_count = 0
    session_run_ignored_count = 0
    run_started: set[str] = set()
    run_completed: set[str] = set()
    model_started: dict[str, dict[str, Any]] = {}
    model_completed: set[str] = set()
    for event in scoped:
        kind = event_type(event)
        data = payload(event)
        rid = run_id(event, data)
        if not rid:
            continue
        if kind in {"RunStarted", "RunCompleted"} and is_session_run(data):
            session_run_ignored_count += 1
            continue
        if kind == "RunStarted":
            lifecycle_start_count += 1
            run_started.add(rid)
        elif kind == "RunCompleted":
            lifecycle_completion_count += 1
            run_completed.add(rid)
        elif kind == "ModelCallStarted":
            lifecycle_start_count += 1
            model_started[rid] = data
        elif kind == "ModelCallCompleted":
            lifecycle_completion_count += 1
            model_completed.add(rid)
    open_models = {rid: data for rid, data in model_started.items() if rid not in model_completed}
    open_runs = run_started - run_completed
    diagnostics = {
        "scan_start": max(scan_start, 0),
        "scanned_events": len(scoped),
        "lifecycle_start_count": lifecycle_start_count,
        "lifecycle_completion_count": lifecycle_completion_count,
        "session_run_ignored_count": session_run_ignored_count,
        "open_model_count": len(open_models),
        "open_run_count": len(open_runs),
    }
    return open_models, open_runs, diagnostics


def open_lifecycle(
    events: list[dict[str, Any]],
    after_line: int,
    fallback_after_line: int | None = None,
) -> tuple[dict[str, dict[str, Any]], set[str], dict[str, Any]]:
    if after_line > len(events):
        # The live state file can be reset/rebuilt during an agent invocation.
        # In that case the saved pre-agent line number belongs to the old file,
        # so scan the current file rather than silently missing open runs.
        scan_start = 0
        scope = "full_after_line_exceeded_file"
    else:
        scan_start = max(after_line, 0)
        scope = "requested_after_line"
    open_models, open_runs, diagnostics = lifecycle_for_scope(events, scan_start)
    diagnostics["event_count"] = len(events)
    diagnostics["requested_after_line"] = after_line
    diagnostics["scope"] = scope
    if (
        diagnostics["lifecycle_start_count"] == 0
        and fallback_after_line is not None
        and fallback_after_line < scan_start
    ):
        fallback_models, fallback_runs, fallback_diagnostics = lifecycle_for_scope(events, fallback_after_line)
        if fallback_diagnostics["lifecycle_start_count"] > 0:
            fallback_diagnostics["event_count"] = len(events)
            fallback_diagnostics["requested_after_line"] = after_line
            fallback_diagnostics["fallback_after_line"] = fallback_after_line
            fallback_diagnostics["primary_scan_start"] = scan_start
            fallback_diagnostics["primary_scanned_events"] = diagnostics["scanned_events"]
            fallback_diagnostics["scope"] = "fallback_after_line"
            return fallback_models, fallback_runs, fallback_diagnostics
    return open_models, open_runs, diagnostics


def append_terminal_events(
    events_path: Path,
    after_line: int,
    fallback_after_line: int | None,
    session_id: str,
    trace_id: str,
    stage: str,
    run_status: str,
    model_status: str,
    reason: str,
    error: str,
    error_detail: str,
) -> dict[str, Any]:
    events = load_jsonl(events_path)
    open_models, open_runs, diagnostics = open_lifecycle(events, after_line, fallback_after_line)
    completed_models: list[str] = []
    completed_runs: list[str] = []

    for rid, started_payload in sorted(open_models.items()):
        payload = {
            "model": started_payload.get("model"),
            "status": model_status,
            "terminal_reason": reason,
            "stage": stage or None,
        }
        if error:
            payload["error"] = error
        if error_detail:
            payload["error_detail"] = error_detail
        append_event(events_path, "ModelCallCompleted", "yoyo", rid, session_id, trace_id, payload)
        completed_models.append(rid)
        open_runs.add(rid)

    for rid in sorted(open_runs):
        payload = {
            "status": run_status,
            "terminal_reason": reason,
            "stage": stage or None,
        }
        if error:
            payload["error"] = error
        if error_detail:
            payload["error_detail"] = error_detail
        append_event(events_path, "RunCompleted", "harness", rid, session_id, trace_id, payload)
        completed_runs.append(rid)

    return {
        "completed_model_calls": completed_models,
        "completed_runs": completed_runs,
        "diagnostics": diagnostics,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", required=True, type=Path)
    parser.add_argument("--after-line", required=True, type=int)
    parser.add_argument("--fallback-after-line", type=int)
    parser.add_argument("--session-id", default="")
    parser.add_argument("--trace-id", default="")
    parser.add_argument("--stage", default="")
    parser.add_argument("--run-status", default="error")
    parser.add_argument("--model-status", default="interrupted")
    parser.add_argument("--reason", required=True)
    parser.add_argument("--error", default="")
    parser.add_argument("--error-detail", default="")
    args = parser.parse_args()
    result = append_terminal_events(
        args.events,
        args.after_line,
        args.fallback_after_line,
        args.session_id,
        args.trace_id,
        args.stage,
        args.run_status,
        args.model_status,
        args.reason,
        args.error,
        args.error_detail,
    )
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
