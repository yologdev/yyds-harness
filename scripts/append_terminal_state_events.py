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
) -> tuple[dict[str, dict[str, Any]], set[str], set[str], dict[str, Any]]:
    scoped = events[max(scan_start, 0) :]
    lifecycle_start_count = 0
    lifecycle_completion_count = 0
    session_run_ignored_count = 0
    run_started: set[str] = set()
    run_completed: set[str] = set()
    model_started: dict[str, dict[str, Any]] = {}
    model_completed: set[str] = set()
    session_run_started: set[str] = set()
    session_run_completed: set[str] = set()
    for event in scoped:
        kind = event_type(event)
        data = payload(event)
        rid = run_id(event, data)
        if not rid:
            continue
        if kind in {"RunStarted", "SessionStarted", "RunCompleted"} and is_session_run(data):
            session_run_ignored_count += 1
            if kind == "RunStarted" or kind == "SessionStarted":
                session_run_started.add(rid)
            elif kind == "RunCompleted":
                session_run_completed.add(rid)
            continue
        if kind == "RunStarted" or kind == "SessionStarted":
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
    active_runs = run_started | set(model_started) | model_completed
    open_runs = active_runs - run_completed
    open_session_runs = session_run_started - session_run_completed
    diagnostics = {
        "scan_start": max(scan_start, 0),
        "scanned_events": len(scoped),
        "lifecycle_start_count": lifecycle_start_count,
        "lifecycle_completion_count": lifecycle_completion_count,
        "session_run_ignored_count": session_run_ignored_count,
        "open_model_count": len(open_models),
        "open_run_count": len(open_runs),
        "open_session_run_count": len(open_session_runs),
    }
    return open_models, open_runs, open_session_runs, diagnostics


def open_lifecycle(
    events: list[dict[str, Any]],
    after_line: int,
    fallback_after_line: int | None = None,
) -> tuple[dict[str, dict[str, Any]], set[str], set[str], dict[str, Any]]:
    if after_line > len(events):
        # The live state file can be reset/rebuilt during an agent invocation.
        # In that case the saved pre-agent line number belongs to the old file,
        # so scan the current file rather than silently missing a single fresh
        # open run. If the rebuilt file exposes multiple open runs, it is
        # usually a replayed historical state snapshot; closing all of them
        # creates ghost terminal events in the current session.
        scan_start = 0
        scope = "full_after_line_exceeded_file"
    else:
        scan_start = max(after_line, 0)
        scope = "requested_after_line"
    open_models, open_runs, open_session_runs, diagnostics = lifecycle_for_scope(events, scan_start)
    diagnostics["event_count"] = len(events)
    diagnostics["requested_after_line"] = after_line
    diagnostics["scope"] = scope
    if scope == "full_after_line_exceeded_file":
        visible_open_runs = set(open_runs) | set(open_models)
        if len(visible_open_runs) > 1:
            diagnostics["scope"] = "ambiguous_reset_full_scan"
            diagnostics["ambiguous_open_run_count"] = len(visible_open_runs)
            return {}, set(), set(), diagnostics
    if (
        diagnostics["lifecycle_start_count"] == 0
        and fallback_after_line is not None
        and fallback_after_line < scan_start
    ):
        fallback_models, fallback_runs, fallback_session_runs, fallback_diagnostics = lifecycle_for_scope(events, fallback_after_line)
        if fallback_diagnostics["lifecycle_start_count"] > 0:
            fallback_diagnostics["event_count"] = len(events)
            fallback_diagnostics["requested_after_line"] = after_line
            fallback_diagnostics["fallback_after_line"] = fallback_after_line
            fallback_diagnostics["primary_scan_start"] = scan_start
            fallback_diagnostics["primary_scanned_events"] = diagnostics["scanned_events"]
            fallback_diagnostics["scope"] = "fallback_after_line"
            return fallback_models, fallback_runs, fallback_session_runs, fallback_diagnostics
    return open_models, open_runs, open_session_runs, diagnostics


def find_stale_orphaned_runs(
    events: list[dict[str, Any]],
) -> tuple[set[str], set[str], dict[str, Any]]:
    """Scan ALL events for runs that have RunStarted but no RunCompleted.

    Unlike lifecycle_for_scope which only scans from an ``after_line`` offset,
    this scans the entire event file.  It catches orphaned runs from previous
    sessions — e.g. a GitHub Actions cancellation where RunStarted was written
    but the harness never reached RunCompleted.

    Only runs that have at least one ModelCallStarted event are considered —
    bare RunStarted entries without model activity are not real orphaned runs.

    Returns (orphaned_runs, orphaned_session_runs, diagnostics).
    """
    run_started: set[str] = set()
    run_completed: set[str] = set()
    runs_with_model_calls: set[str] = set()
    session_run_started: set[str] = set()
    session_run_completed: set[str] = set()
    for event in events:
        kind = event_type(event)
        data = payload(event)
        rid = run_id(event, data)
        if not rid:
            continue
        if kind in {"RunStarted", "SessionStarted", "RunCompleted"} and is_session_run(data):
            if kind == "RunStarted" or kind == "SessionStarted":
                session_run_started.add(rid)
            elif kind == "RunCompleted":
                session_run_completed.add(rid)
        elif kind == "RunStarted" or kind == "SessionStarted":
            run_started.add(rid)
        elif kind == "RunCompleted":
            run_completed.add(rid)
        elif kind == "ModelCallStarted":
            runs_with_model_calls.add(rid)
    # Only consider runs that have model activity as real orphans.
    # Bare RunStarted entries without ModelCallStarted are not
    # indicative of an interrupted agent invocation.
    orphaned_runs = (run_started - run_completed) & runs_with_model_calls
    orphaned_session_runs = session_run_started - session_run_completed
    diagnostics = {
        "full_scan_events": len(events),
        "full_scan_run_started": len(run_started),
        "full_scan_run_completed": len(run_completed),
        "full_scan_orphaned_runs": len(orphaned_runs),
        "full_scan_session_run_started": len(session_run_started),
        "full_scan_session_run_completed": len(session_run_completed),
        "full_scan_orphaned_session_runs": len(orphaned_session_runs),
        "full_scan_runs_with_model_calls": len(runs_with_model_calls),
    }
    return orphaned_runs, orphaned_session_runs, diagnostics


def find_missing_failure_observed(
    events: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Find runs completed with error status that lack FailureObserved events.

    Returns (missing_entries, diagnostics) where each missing entry is a dict
    with keys run_id, status, timestamp_ms for runs whose RunCompleted
    payload status is not "success"/"completed" and that have no matching
    FailureObserved event.
    """
    error_completed: dict[str, dict[str, Any]] = {}  # run_id -> {status, timestamp_ms}
    failure_observed_runs: set[str] = set()

    for event in events:
        kind = event_type(event)
        data = payload(event)
        rid = run_id(event, data)
        if not rid:
            continue
        if kind == "RunCompleted":
            status = data.get("status", "")
            if status and status not in ("success", "completed"):
                error_completed[rid] = {
                    "status": status,
                    "timestamp_ms": event.get("timestamp_ms", 0),
                }
        elif kind == "FailureObserved":
            failure_observed_runs.add(rid)

    missing = sorted(
        [
            {"run_id": rid, **info}
            for rid, info in error_completed.items()
            if rid not in failure_observed_runs
        ],
        key=lambda m: m["run_id"],
    )

    diagnostics = {
        "error_completed_runs": len(error_completed),
        "failure_observed_runs": len(failure_observed_runs),
        "missing_failure_observed": len(missing),
    }

    return missing, diagnostics


def find_runs_with_failure_observed_no_completion(
    events: list[dict[str, Any]],
) -> tuple[set[str], dict[str, Any]]:
    """Find runs that have FailureObserved but no RunCompleted.

    These runs recorded a failure but their lifecycle was never formally
    closed.  They contribute to ``open_after_FailureObserved`` and inflate
    ``state_run_incomplete_count`` in gnome summaries.

    Returns (runs_missing_run_completed, diagnostics).
    """
    failure_observed_runs: set[str] = set()
    run_completed_runs: set[str] = set()

    for event in events:
        kind = event_type(event)
        data = payload(event)
        rid = run_id(event, data)
        if not rid:
            continue
        if kind == "FailureObserved":
            failure_observed_runs.add(rid)
        elif kind == "RunCompleted":
            run_completed_runs.add(rid)

    missing = failure_observed_runs - run_completed_runs

    diagnostics = {
        "failure_observed_runs": len(failure_observed_runs),
        "run_completed_runs_in_fo_scan": len(run_completed_runs),
        "runs_with_failure_observed_no_completion": len(missing),
    }

    return missing, diagnostics


def find_missing_model_call_started(
    events: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Find ModelCallCompleted entries whose run_id has no prior ModelCallStarted.

    Returns (missing_entries, diagnostics) where each missing entry is a dict
    with keys run_id, model, timestamp_ms for orphaned ModelCallCompleted
    events.
    """
    model_started_runs: set[str] = set()
    model_completed_entries: dict[str, dict[str, Any]] = {}

    for event in events:
        kind = event_type(event)
        data = payload(event)
        rid = run_id(event, data)
        if not rid:
            continue
        if kind == "ModelCallStarted":
            model_started_runs.add(rid)
        elif kind == "ModelCallCompleted":
            model_completed_entries[rid] = {
                "model": data.get("model"),
                "timestamp_ms": event.get("timestamp_ms", 0),
            }

    missing = sorted(
        [
            {"run_id": rid, **info}
            for rid, info in model_completed_entries.items()
            if rid not in model_started_runs
        ],
        key=lambda m: m["run_id"],
    )

    diagnostics = {
        "model_call_started_count": len(model_started_runs),
        "model_call_completed_count": len(model_completed_entries),
        "unmatched_model_call_completed_count": len(missing),
    }

    return missing, diagnostics


def find_orphaned_model_calls(
    events: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Find ModelCallStarted events whose model_call_id has no matching ModelCallCompleted.

    Unlike lifecycle_for_scope which pairs by run_id within a limited scan
    window, this scans ALL events and pairs by model_call_id (the natural
    key in production events).  When model_call_id is absent (legacy test
    fixtures), it falls back to run_id.

    Returns (orphaned_entries, diagnostics) where each orphaned entry is a
    dict with keys run_id, model, model_call_id, timestamp_ms, key.
    """
    model_started_entries: dict[str, dict[str, Any]] = {}
    model_completed_keys: set[str] = set()
    keyed_by_model_call_id: set[str] = set()

    for event in events:
        kind = event_type(event)
        data = payload(event)
        rid = run_id(event, data)
        if not rid:
            continue
        if kind == "ModelCallStarted":
            mcid = data.get("model_call_id")
            if isinstance(mcid, str) and mcid:
                key = mcid
                keyed_by_model_call_id.add(key)
            else:
                key = rid
            # When multiple ModelCallStarted events share the same key
            # (e.g., same run_id without model_call_id), keep the last one.
            model_started_entries[key] = {
                "run_id": rid,
                "model": data.get("model"),
                "model_call_id": mcid if isinstance(mcid, str) and mcid else None,
                "timestamp_ms": event.get("timestamp_ms", 0),
                "key": key,
            }
        elif kind == "ModelCallCompleted":
            mcid = data.get("model_call_id")
            if isinstance(mcid, str) and mcid:
                model_completed_keys.add(mcid)
            else:
                model_completed_keys.add(rid)

    orphaned = sorted(
        [
            info
            for key, info in model_started_entries.items()
            if key not in model_completed_keys
        ],
        key=lambda m: m["run_id"],
    )

    diagnostics = {
        "model_call_started_count": len(model_started_entries),
        "model_call_completed_count": len(model_completed_keys),
        "orphaned_model_calls": len(orphaned),
        "keyed_by_model_call_id": len(keyed_by_model_call_id),
    }

    return orphaned, diagnostics


def _maybe_append_event(
    events_path: Path,
    event_type: str,
    actor: str,
    rid: str,
    session_id: str,
    trace_id: str,
    payload: dict[str, Any],
    dry_run: bool,
) -> dict[str, Any] | None:
    """Append an event or print what would be appended in dry-run mode."""
    if dry_run:
        import sys
        print(
            f"[dry-run] Would append {event_type} actor={actor} run_id={rid} "
            f"payload={json.dumps(payload, sort_keys=True, separators=(',', ':'))}",
            file=sys.stderr,
        )
        return None
    return append_event(events_path, event_type, actor, rid, session_id, trace_id, payload)


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
    dry_run: bool = False,
) -> dict[str, Any]:
    events = load_jsonl(events_path)
    open_models, open_runs, open_session_runs, diagnostics = open_lifecycle(events, after_line, fallback_after_line)
    completed_models: list[str] = []
    completed_runs: list[str] = []
    completed_session_runs: list[str] = []

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
        _maybe_append_event(events_path, "ModelCallCompleted", "yoyo", rid, session_id, trace_id, payload, dry_run)
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
        _maybe_append_event(events_path, "RunCompleted", "harness", rid, session_id, trace_id, payload, dry_run)
        completed_runs.append(rid)

    for rid in sorted(open_session_runs):
        payload = {
            "status": run_status,
            "terminal_reason": reason,
            "stage": stage or None,
            "outcome": "post_hoc_closed",
        }
        if error:
            payload["error"] = error
        if error_detail:
            payload["error_detail"] = error_detail
        _maybe_append_event(events_path, "RunCompleted", "harness", rid, session_id, trace_id, payload, dry_run)
        completed_session_runs.append(rid)

    # Full-scan orphan detection: find any runs in the entire event file
    # that have RunStarted but no RunCompleted, regardless of position.
    # This catches orphans from previous sessions (e.g., GitHub Actions
    # cancellations) that the incremental after_line scan cannot see.
    # Only runs that have at least one ModelCallStarted are considered
    # (bare RunStarted entries are not real orphaned runs).
    # The scan is skipped when the scope is ambiguous (events file was
    # reset/rebuilt) to avoid closing historical replayed runs.
    if diagnostics.get("scope") != "ambiguous_reset_full_scan":
        stale_orphaned_runs, stale_orphaned_session_runs, orphan_diag = find_stale_orphaned_runs(events)
        diagnostics["full_scan_orphan_diagnostics"] = orphan_diag

        # Only close orphans that weren't already closed by the incremental scan above.
        already_closed = set(completed_runs + completed_session_runs)
        new_orphan_runs = stale_orphaned_runs - already_closed
        new_orphan_session_runs = stale_orphaned_session_runs - already_closed

        for rid in sorted(new_orphan_runs):
            payload = {
                "status": "error",
                "terminal_reason": "orphaned_previous_session",
                "stage": stage or None,
                "outcome": "post_hoc_closed",
            }
            _maybe_append_event(events_path, "RunCompleted", "harness", rid, session_id, trace_id, payload, dry_run)
            completed_runs.append(rid)

        for rid in sorted(new_orphan_session_runs):
            payload = {
                "status": "error",
                "terminal_reason": "orphaned_previous_session",
                "stage": stage or None,
                "outcome": "post_hoc_closed",
            }
            _maybe_append_event(events_path, "RunCompleted", "harness", rid, session_id, trace_id, payload, dry_run)
            completed_session_runs.append(rid)
    else:
        diagnostics["full_scan_orphan_diagnostics"] = {
            "skipped": True,
            "reason": "ambiguous_reset_full_scan",
        }

    # Full-scan for missing FailureObserved events: find any runs that
    # completed with error status (not "success"/"completed") but have no
    # matching FailureObserved event.  This closes the gap where error-
    # completed sessions were never formally recorded as failures.
    # Unlike the orphan scan, this scan is NOT gated by the ambiguous-reset
    # guard: the find_missing_failure_observed function already checks for
    # existing FailureObserved events per run_id, so there is no risk of
    # duplicates even when scanning a rebuilt events file that contains
    # historical replayed runs.
    missing_failures: list[dict[str, Any]] = []
    missing_failures, failure_diag = find_missing_failure_observed(events)
    diagnostics["failure_observed_diagnostics"] = failure_diag

    for entry in missing_failures:
        rid = entry["run_id"]
        status = entry.get("status", "")
        if status == "cancelled":
            reason_text = "retroactive: run cancelled by next hourly session"
        else:
            reason_text = (
                f"retroactive: run completed with error status "
                f"'{status}' but no FailureObserved was recorded"
            )
        payload_fo: dict[str, Any] = {
            "reason": reason_text,
            "retroactive": True,
        }
        ts = entry.get("timestamp_ms")
        if ts:
            payload_fo["original_run_completed_timestamp_ms"] = ts
        _maybe_append_event(events_path, "FailureObserved", "harness", rid, session_id, trace_id, payload_fo, dry_run)

    # Full-scan for runs with FailureObserved but no RunCompleted.
    # These runs recorded a failure but their lifecycle was never formally
    # closed — they contribute to open_after_FailureObserved and inflate
    # state_run_incomplete_count. Emit the missing RunCompleted to close
    # the lifecycle book.
    fo_no_rc_runs, fo_no_rc_diag = find_runs_with_failure_observed_no_completion(events)
    diagnostics["failure_observed_no_completion_diagnostics"] = fo_no_rc_diag

    # Recompute already_closed to include runs closed by the stale-orphan
    # scan above (which appended to completed_runs after the original
    # already_closed was computed).
    already_closed_fo = set(completed_runs + completed_session_runs)

    # Collect runs that already have RunStarted events so we can detect
    # and prevent unmatched non-validation completions (RunCompleted without
    # a matching RunStarted).  Historical runs recorded from before the
    # ensure_run_started guard was added may have FailureObserved but no
    # RunStarted — closing them with RunCompleted alone would create
    # state_run_unmatched_non_validation_completed_count inflation.
    run_started_runs: set[str] = set()
    for event in events:
        if event_type(event) == "RunStarted":
            _data = event.get("payload")
            if isinstance(_data, dict):
                _wrapped = _data.get("value")
                if set(_data.keys()).issubset({"_yoyo", "value"}) and isinstance(_wrapped, dict):
                    _data = _wrapped
            rid_rs = run_id(event, _data if isinstance(_data, dict) else {})
            if rid_rs:
                run_started_runs.add(rid_rs)

    for rid in sorted(fo_no_rc_runs):
        if rid in already_closed_fo:
            continue
        # If the run has no RunStarted, emit a retroactive one first.
        if rid not in run_started_runs:
            _maybe_append_event(
                events_path, "RunStarted", "harness", rid, session_id, trace_id,
                {
                    "reason": "retroactive: no RunStarted found for orphaned run",
                    "retroactive": True,
                },
                dry_run,
            )
        payload_fo_rc = {
            "status": "error",
            "terminal_reason": "orphaned_previous_session",
            "stage": stage or None,
            "outcome": "post_hoc_closed",
        }
        _maybe_append_event(events_path, "RunCompleted", "harness", rid, session_id, trace_id, payload_fo_rc, dry_run)
        completed_runs.append(rid)

    # Full-scan for unmatched ModelCallCompleted: find any ModelCallCompleted
    # entries whose run_id has no prior ModelCallStarted.  Emit retroactive
    # ModelCallStarted events to close the model-call lifecycle gap.
    # Gated by the ambiguous-reset guard to avoid emitting retroactive
    # events when the events file was reset/rebuilt (same as stale orphan scan).
    model_call_started_appended = 0
    if diagnostics.get("scope") != "ambiguous_reset_full_scan":
        missing_starts, mcs_diag = find_missing_model_call_started(events)
        diagnostics["model_call_started_diagnostics"] = mcs_diag

        for entry in missing_starts:
            rid = entry["run_id"]
            payload_mcs: dict[str, Any] = {
                "model": entry.get("model"),
                "model_call_id": f"retroactive-{rid}",
                "retroactive": True,
            }
            ts = entry.get("timestamp_ms")
            if ts:
                payload_mcs["original_model_call_completed_timestamp_ms"] = ts
            _maybe_append_event(events_path, "ModelCallStarted", "harness", rid, session_id, trace_id, payload_mcs, dry_run)
            model_call_started_appended += 1
    else:
        diagnostics["model_call_started_diagnostics"] = {
            "skipped": True,
            "reason": "ambiguous_reset_full_scan",
        }

    # Full-scan for orphaned ModelCallStarted: find any ModelCallStarted
    # events whose model_call_id has no matching ModelCallCompleted.  Emit
    # retroactive ModelCallCompleted events to close the model-call lifecycle
    # gap.  This is the forward-direction counterpart to the
    # find_missing_model_call_started scan above (which handles the inverse).
    # Gated by the ambiguous-reset guard.
    orphaned_model_calls_appended = 0
    if diagnostics.get("scope") != "ambiguous_reset_full_scan":
        orphaned_model_calls, omc_diag = find_orphaned_model_calls(events)
        diagnostics["orphaned_model_call_diagnostics"] = omc_diag

        for entry in orphaned_model_calls:
            rid = entry["run_id"]
            mcid = entry.get("model_call_id")
            payload_omc: dict[str, Any] = {
                "status": "interrupted",
                "model": entry.get("model"),
                "terminal_reason": "retroactive: ModelCallStarted orphaned — no ModelCallCompleted found",
                "retroactive": True,
            }
            if mcid:
                payload_omc["model_call_id"] = mcid
            # When the original ModelCallStarted had no model_call_id,
            # omit it from the retroactive event as well.  On the next
            # janitor invocation, find_orphaned_model_calls will detect
            # this completed event by run_id (the same key used for the
            # original started event), preventing duplicate retroactive
            # events (same bug class as Day 139's FailureObserved dedup).
            ts = entry.get("timestamp_ms")
            if ts:
                payload_omc["original_model_call_started_timestamp_ms"] = ts
            _maybe_append_event(events_path, "ModelCallCompleted", "harness", rid, session_id, trace_id, payload_omc, dry_run)
            orphaned_model_calls_appended += 1
    else:
        diagnostics["orphaned_model_call_diagnostics"] = {
            "skipped": True,
            "reason": "ambiguous_reset_full_scan",
        }

    return {
        "completed_model_calls": completed_models,
        "completed_runs": completed_runs,
        "completed_session_runs": completed_session_runs,
        "failure_observed_appended": [m["run_id"] for m in missing_failures],
        "model_call_started_appended": model_call_started_appended,
        "orphaned_model_calls_appended": orphaned_model_calls_appended,
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
    parser.add_argument("--dry-run", action="store_true", default=False)
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
        dry_run=args.dry_run,
    )
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
