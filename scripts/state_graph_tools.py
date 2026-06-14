#!/usr/bin/env python3
"""ActiveGraph-inspired audit graph helpers for yyds sessions.

The yyds source of truth is still yoagent-state JSONL. These helpers build
replay checks, task causal chains, baseline comparisons, and next-task
suggestions from existing audit-log session artifacts.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Any


GNOME_COMPARE_KEYS = [
    "coding_log_score",
    "tool_call_malformed_rate",
    "json_parse_failure_rate",
    "context_miss_rate",
    "session_success_rate",
    "task_success_rate",
    "task_verification_rate",
    "task_mechanical_verification_rate",
    "task_artifact_coverage",
    "task_lineage_capture_coverage",
    "task_spec_quality_score",
    "state_operational_capture_coverage",
    "state_replay_integrity_rate",
    "state_failure_count",
    "evolution_friction_count",
    "repair_loop_count",
    "recurring_failure_count",
    "max_failure_fingerprint_recurrence",
    "provider_error_count",
    "tool_error_count",
    "prompt_heredoc_expansion_error_count",
    "command_timeout_count",
    "evaluator_timeout_count",
    "evaluator_unverified_count",
    "task_unlanded_source_count",
    "search_error_count",
    "max_task_turn_count",
    "deepseek_cache_hit_ratio",
    "state_run_incomplete_count",
    "state_run_unmatched_non_validation_completed_count",
    "deepseek_model_call_abnormal_completed_count",
    "deepseek_model_call_incomplete_count",
    "deepseek_model_call_unmatched_completed_count",
]


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


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


def event_kind(event: dict[str, Any]) -> str:
    value = event.get("event_type") or event.get("kind")
    if isinstance(value, str):
        return value
    payload = event.get("payload")
    if isinstance(payload, dict):
        meta = payload.get("_yoyo")
        if isinstance(meta, dict) and isinstance(meta.get("event_type"), str):
            return str(meta["event_type"])
    return ""


def event_counts(events: list[dict[str, Any]]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for event in events:
        kind = event_kind(event)
        if kind:
            counts[kind] = counts.get(kind, 0) + 1
    return counts


def session_dirs(sessions_dir: Path) -> list[Path]:
    if not sessions_dir.is_dir():
        return []
    return sorted(path for path in sessions_dir.iterdir() if path.is_dir())


def session_sort_key(session_dir: Path) -> tuple[int, str, str]:
    outcome = load_json(session_dir / "outcome.json")
    day = outcome.get("day")
    return (
        int(day) if isinstance(day, int) else -1,
        str(outcome.get("ts") or ""),
        session_dir.name,
    )


def ordered_sessions(sessions_dir: Path) -> list[Path]:
    return sorted(session_dirs(sessions_dir), key=session_sort_key)


def task_manifest(session_dir: Path) -> dict[str, Any]:
    return load_json(session_dir / "tasks" / "manifest.json")


def assessment_artifact_gap(session_dir: Path, manifest: dict[str, Any]) -> dict[str, Any]:
    planner = manifest.get("planner") if isinstance(manifest.get("planner"), dict) else {}
    artifacts = manifest.get("artifacts") if isinstance(manifest.get("artifacts"), dict) else {}
    assessment_path = session_dir / "tasks" / "assessment.md"
    diagnostic_path = session_dir / "tasks" / "assessment_missing.md"
    transcript_path = session_dir / "transcripts" / "assess.log"
    artifact_present = bool(planner.get("assessment_present") or artifacts.get("assessment") or assessment_path.is_file())
    diagnostic_present = bool(
        planner.get("assessment_missing_present")
        or artifacts.get("assessment_missing")
        or diagnostic_path.is_file()
    )
    transcript_present = transcript_path.is_file() and transcript_path.stat().st_size > 0
    missing_with_evidence = not artifact_present and (diagnostic_present or transcript_present)
    if not missing_with_evidence:
        return {}
    if diagnostic_present and transcript_present:
        classification = "missing_with_diagnostic_and_transcript"
    elif diagnostic_present:
        classification = "missing_with_diagnostic"
    else:
        classification = "missing_transcript_only"
    return {
        "missing": True,
        "classification": classification,
        "diagnostic_present": diagnostic_present,
        "transcript_present": transcript_present,
    }


def selected_tasks(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    tasks = manifest.get("selected_tasks") or manifest.get("tasks") or []
    return [task for task in tasks if isinstance(task, dict)]


TASK_SPEC_WARNING_SUFFIXES = {
    "missing_expected_evidence",
    "assessment_contradiction",
    "generic_self_improvement",
    "missing_files",
    "thin_task_spec",
}


def task_spec_warning_counts(manifest: dict[str, Any]) -> dict[str, int]:
    tasks = selected_tasks(manifest)
    if not tasks:
        return {}
    counts = {suffix: 0 for suffix in TASK_SPEC_WARNING_SUFFIXES}
    counted: set[tuple[str, str]] = set()
    for warning in manifest.get("warnings") or []:
        if not isinstance(warning, str) or ":" not in warning:
            continue
        task_id, suffix = warning.split(":", 1)
        if suffix in TASK_SPEC_WARNING_SUFFIXES and (task_id, suffix) not in counted:
            counts[suffix] += 1
            counted.add((task_id, suffix))
    for task in tasks:
        task_id = str(task.get("task_id") or "")
        quality = task.get("quality") if isinstance(task.get("quality"), dict) else {}
        expected = str(task.get("expected_evidence") or "").strip()
        has_expected_flag = quality.get("has_expected_evidence")
        checks = {
            "missing_expected_evidence": has_expected_flag is False or not expected,
            "generic_self_improvement": quality.get("generic_self_improvement") is True,
            "missing_files": not task.get("files"),
            "thin_task_spec": (
                isinstance(quality.get("score"), (int, float))
                and not isinstance(quality.get("score"), bool)
                and float(quality.get("score") or 0.0) < 0.75
            ),
        }
        alignment = (
            quality.get("assessment_alignment")
            if isinstance(quality.get("assessment_alignment"), dict)
            else {}
        )
        checks["assessment_contradiction"] = alignment.get("contradicted_by_assessment") is True
        for suffix, is_missing in checks.items():
            if is_missing and (task_id, suffix) not in counted:
                counts[suffix] += 1
                counted.add((task_id, suffix))
    return {key: value for key, value in sorted(counts.items()) if value > 0}


def task_spec_warning_detail(counts: dict[str, int]) -> str:
    labels = {
        "assessment_contradiction": "assessment_contradiction",
        "generic_self_improvement": "generic_self_improvement",
        "missing_expected_evidence": "missing_expected_evidence",
        "missing_files": "missing_files",
        "thin_task_spec": "thin_task_spec",
    }
    return ", ".join(
        f"{labels[key]}={value}"
        for key, value in sorted(counts.items(), key=lambda item: (-item[1], item[0]))
        if value > 0
    )


def task_spec_evidence_gap_count(manifest: dict[str, Any]) -> int:
    return task_spec_warning_counts(manifest).get("missing_expected_evidence", 0)


def path_matches(planned: str, touched: str) -> bool:
    planned = str(planned).strip().strip("/")
    touched = str(touched).strip().strip("/")
    if not planned or not touched:
        return False
    return touched == planned or touched.startswith(f"{planned}/")


def file_overlap(planned: list[str], touched: list[str]) -> bool:
    return any(path_matches(planned_file, touched_file) for planned_file in planned for touched_file in touched)


def source_file(path: str) -> bool:
    if not path or path.endswith(".bak"):
        return False
    return not path.startswith((".yoyo/", "journals/", "memory/", "session_plan/", "sessions/", "site/"))


def task_artifact_rows(session_dir: Path) -> dict[str, dict[str, Any]]:
    rows: dict[str, dict[str, Any]] = {}
    tasks_dir = session_dir / "tasks"
    if not tasks_dir.is_dir():
        return rows
    for child in sorted(tasks_dir.iterdir()):
        if not child.is_dir() or not child.name.startswith("task_"):
            continue
        task_id = child.name
        evals = []
        for eval_path in sorted(child.glob("eval_attempt_*.json")):
            value = load_json(eval_path)
            if value:
                evals.append(value)
        attempts = load_jsonl(child / "attempts.jsonl")
        outcome = load_json(child / "outcome.json")
        rows[task_id] = {
            "task_id": task_id,
            "task_file": f"tasks/{task_id}/task.md" if (child / "task.md").is_file() else None,
            "decision_file": f"tasks/{task_id}/decision.json" if (child / "decision.json").is_file() else None,
            "attempts": attempts,
            "evals": evals,
            "outcome": outcome,
        }
    return rows


def eval_passed(evals: list[Any]) -> bool:
    for item in evals:
        if not isinstance(item, dict):
            continue
        status = str(item.get("status") or "").strip().lower()
        verdict = str(item.get("verdict") or "").upper()
        if status == "pass" or ("PASS" in verdict and "FAIL" not in verdict):
            return True
    return False


def task_artifact_verification_metrics(session_dir: Path) -> dict[str, Any]:
    manifest = task_manifest(session_dir)
    tasks = selected_tasks(manifest)
    if not tasks:
        return {}
    artifacts = task_artifact_rows(session_dir)
    rows: list[dict[str, Any]] = []
    for task in tasks:
        task_id = str(task.get("task_id") or "")
        if not task_id:
            continue
        artifact = artifacts.get(task_id) or {}
        outcome = artifact.get("outcome") if isinstance(artifact.get("outcome"), dict) else {}
        planned = [str(path) for path in (outcome.get("planned_files") or task.get("files") or []) if path]
        source_files = [str(path) for path in (outcome.get("source_files") or []) if path]
        touched = [str(path) for path in (outcome.get("touched_files") or source_files or []) if path]
        source_touched = [path for path in source_files + touched if source_file(path)]
        commits = [str(sha) for sha in (outcome.get("commit_shas") or []) if sha]
        outcome_status = str(outcome.get("status") or "").strip().lower()
        revert_reason = str(outcome.get("revert_reason") or "")
        problems: list[str] = []
        overlap = file_overlap(planned, touched) if planned and touched else False
        if not planned:
            problems.append("missing_planned_files")
        if not touched:
            problems.append("no_touched_files")
        if planned and touched and not overlap:
            problems.append("no_planned_file_overlap")
        api_error = "api error" in revert_reason.lower()
        protected_revert = "modified protected files" in revert_reason.lower()
        obsolete = bool(outcome.get("has_obsolete_note"))
        if obsolete:
            problems.append("task_marked_obsolete")
        if api_error:
            problems.append("implementation_api_error")
        if protected_revert:
            problems.append("modified_protected_files")
        if outcome_status == "reverted" and not touched and not obsolete and not api_error and not protected_revert:
            problems.append("no_edit_revert")
        if source_touched and not commits:
            problems.append("source_edits_not_landed")
        passed = eval_passed(artifact.get("evals") if isinstance(artifact.get("evals"), list) else [])
        if not passed and not obsolete:
            problems.append("no_passing_verifier")
        strict_success = outcome_status == "completed" and overlap and passed and not problems
        rows.append(
            {
                "strict_success": strict_success,
                "problems": problems,
                "obsolete": obsolete,
                "api_error": api_error,
                "protected_revert": protected_revert,
            }
        )
    if not rows:
        return {}
    unverified = sum(1 for row in rows if not row["strict_success"])
    return {
        "selected_task_count": len(tasks),
        "task_count": len(rows),
        "task_strict_verified_count": sum(1 for row in rows if row["strict_success"]),
        "task_verified_count": sum(1 for row in rows if row["strict_success"]),
        "task_obsolete_count": sum(1 for row in rows if row["obsolete"] or "task_marked_obsolete" in row["problems"]),
        "task_api_error_count": sum(1 for row in rows if row["api_error"] or "implementation_api_error" in row["problems"]),
        "task_no_edit_revert_count": sum(1 for row in rows if "no_edit_revert" in row["problems"]),
        "protected_file_revert_count": sum(1 for row in rows if row["protected_revert"] or "modified_protected_files" in row["problems"]),
        "task_scope_mismatch_count": sum(1 for row in rows if "no_planned_file_overlap" in row["problems"]),
        "task_unlanded_source_count": sum(
            1
            for row in rows
            if ("source_edits_not_landed" in row["problems"] or "no_landed_source_commit" in row["problems"])
            and not row["protected_revert"]
            and "no_planned_file_overlap" not in row["problems"]
        ),
        "evaluator_unverified_raw_count": unverified,
    }


def lineage_rows(summary: dict[str, Any]) -> dict[str, dict[str, Any]]:
    rows = summary.get("task_lineage") if isinstance(summary.get("task_lineage"), list) else []
    return {
        str(row.get("task_id")): row
        for row in rows
        if isinstance(row, dict) and row.get("task_id")
    }


def build_causal_chains(session_dir: Path) -> list[dict[str, Any]]:
    """Return task -> files -> commits -> eval -> gnome causal-chain rows."""
    summary = load_json(session_dir / "state" / "summary.json")
    manifest = task_manifest(session_dir)
    artifacts = task_artifact_rows(session_dir)
    lineage = lineage_rows(summary)
    chains: list[dict[str, Any]] = []

    for task in selected_tasks(manifest):
        task_id = str(task.get("task_id") or "")
        if not task_id:
            continue
        link = lineage.get(task_id, {})
        artifact = artifacts.get(task_id, {})
        evals = artifact.get("evals") or []
        eval_verdict = None
        eval_reason = None
        if isinstance(link.get("eval"), dict):
            eval_verdict = link["eval"].get("verdict")
            eval_reason = link["eval"].get("reason")
        if eval_verdict is None and evals:
            eval_verdict = evals[-1].get("verdict") or evals[-1].get("status")
            eval_reason = evals[-1].get("reason")
        chains.append(
            {
                "task_id": task_id,
                "task_number": task.get("task_number") or link.get("task_number"),
                "title": task.get("title") or link.get("task_title"),
                "planned_files": task.get("files") or link.get("planned_files") or [],
                "touched_files": link.get("touched_files") or [],
                "source_files": link.get("source_files") or [],
                "commit_shas": link.get("commit_shas") or [],
                "commits": link.get("commits") or [],
                "commit_linkage_method": link.get("commit_linkage_method"),
                "eval_verdict": eval_verdict,
                "eval_reason": eval_reason,
                "eval_statuses": [
                    str(item.get("status"))
                    for item in evals
                    if isinstance(item, dict) and item.get("status") is not None
                ],
                "gnome_deltas": link.get("gnome_deltas") or {},
                "gnome_metrics": link.get("gnome_metrics") or {},
                "task_file": artifact.get("task_file") or task.get("artifact_path"),
                "decision_file": artifact.get("decision_file"),
                "attempt_count": len(artifact.get("attempts") or []),
            }
        )
    return chains


def replay_check_session(session_dir: Path) -> dict[str, Any]:
    summary_path = session_dir / "state" / "summary.json"
    events_path = session_dir / "state" / "events.jsonl"
    summary = load_json(summary_path)
    events = load_jsonl(events_path)
    manifest = task_manifest(session_dir)
    tasks = selected_tasks(manifest)
    task_dirs = sorted(
        path
        for path in (session_dir / "tasks").glob("task_*")
        if path.is_dir()
    )
    counts = event_counts(events)
    summary_counts = summary.get("event_counts") if isinstance(summary.get("event_counts"), dict) else {}
    mismatches: list[str] = []
    if not events_path.is_file():
        mismatches.append("missing_state_events_jsonl")
    if not summary_path.is_file():
        mismatches.append("missing_state_summary_json")
    if summary and int(summary.get("event_count") or 0) != len(events):
        mismatches.append("summary_event_count_mismatch")
    for kind, count in counts.items():
        if int(summary_counts.get(kind) or 0) != count:
            mismatches.append(f"summary_event_count_{kind}_mismatch")
            break
    planning_failed = bool((manifest.get("planner") or {}).get("planning_failed"))
    if manifest:
        selected_count = int((manifest.get("planner") or {}).get("selected_task_count") or len(tasks))
        if selected_count != len(tasks):
            mismatches.append("manifest_selected_task_count_mismatch")
        if planning_failed and task_dirs:
            mismatches.append("planning_failed_but_task_dirs_exist")
        if not planning_failed and tasks and len(task_dirs) < len(tasks):
            mismatches.append("missing_task_artifact_dirs")
    return {
        "session_id": session_dir.name,
        "event_count": len(events),
        "summary_event_count": summary.get("event_count"),
        "task_count": len(tasks),
        "task_artifact_dir_count": len(task_dirs),
        "planning_failed": planning_failed,
        "manifest_available": bool(manifest),
        "events_available": events_path.is_file(),
        "summary_available": summary_path.is_file(),
        "ok": not mismatches,
        "mismatches": mismatches,
    }


def replay_check(sessions_dir: Path) -> dict[str, Any]:
    rows = [replay_check_session(path) for path in ordered_sessions(sessions_dir)]
    checked = len(rows)
    passed = sum(1 for row in rows if row["ok"])
    return {
        "sessions_dir": str(sessions_dir),
        "sessions_checked": checked,
        "sessions_passed": passed,
        "state_replay_integrity_rate": (passed / checked) if checked else None,
        "sessions": rows,
    }


def latest_gnomes(session_dir: Path) -> dict[str, Any]:
    value = load_json(session_dir / "state" / "summary.json").get("latest_gnomes")
    return value if isinstance(value, dict) else {}


def numeric_metric(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def int_metric(metrics: dict[str, Any], key: str) -> int:
    value = metrics.get(key)
    if numeric_metric(value):
        return int(value)
    if isinstance(value, str):
        try:
            return int(float(value))
        except ValueError:
            return 0
    return 0


def latest_log_feedback_metrics(session_dir: Path) -> dict[str, Any]:
    feedback = load_json(session_dir / "log_feedback.json")
    metrics = feedback.get("metrics") if isinstance(feedback.get("metrics"), dict) else {}
    if metrics:
        return metrics

    summary = load_json(session_dir / "state" / "summary.json")
    latest_eval = summary.get("latest_eval") if isinstance(summary.get("latest_eval"), dict) else {}
    if latest_eval.get("suite") != "log-feedback":
        return {}
    eval_metrics = latest_eval.get("gnomes") if isinstance(latest_eval.get("gnomes"), dict) else {}
    return eval_metrics


def resolved_seed_replacement_metrics(metrics: dict[str, Any]) -> bool:
    seed_contradictions = int_metric(metrics, "task_seed_contradiction_count")
    if seed_contradictions <= 0:
        return False
    if int_metric(metrics, "task_manifest_seed_contradiction_count") > 0:
        return False
    selected = int_metric(metrics, "selected_task_count")
    strict_verified = int_metric(metrics, "task_strict_verified_count")
    succeeded = int_metric(metrics, "tasks_succeeded")
    return bool(
        selected > 0
        and strict_verified >= selected
        and succeeded >= selected
        and int_metric(metrics, "task_revert_count") == 0
        and int_metric(metrics, "task_obsolete_count") == 0
    )


def bare_fatal_pattern_only(metrics: dict[str, Any]) -> bool:
    if int_metric(metrics, "search_error_count") <= 0:
        return False
    evidence: list[str] = []
    for item in metrics.get("evidence") or []:
        if isinstance(item, str):
            evidence.append(item)
    for item in metrics.get("failure_fingerprints") or []:
        if not isinstance(item, dict):
            continue
        for key in ("fingerprint", "example"):
            value = item.get(key)
            if isinstance(value, str):
                evidence.append(value)
    search_evidence = [
        line
        for line in evidence
        if "fatal: no pattern given" in line.lower() or "search error" in line.lower()
    ]
    if not search_evidence:
        return False
    if any("search error" in line.lower() for line in search_evidence):
        return False
    return all("fatal: no pattern given" in line.lower() for line in search_evidence)


def fully_verified_success_metrics(metrics: dict[str, Any]) -> bool:
    selected = int_metric(metrics, "selected_task_count")
    if selected <= 0:
        return False
    return bool(
        int_metric(metrics, "task_strict_verified_count") >= selected
        and int_metric(metrics, "tasks_succeeded") >= selected
        and int_metric(metrics, "task_revert_count") == 0
        and int_metric(metrics, "task_scope_mismatch_count") == 0
        and int_metric(metrics, "task_unlanded_source_count") == 0
        and int_metric(metrics, "task_api_error_count") == 0
        and int_metric(metrics, "evaluator_unverified_count") == 0
    )


def float_metric(metrics: dict[str, Any], key: str) -> float | None:
    value = metrics.get(key)
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value)
        except ValueError:
            return None
    return None


def outcome_task_success_rate(session_dir: Path) -> float | None:
    outcome = load_json(session_dir / "outcome.json")
    attempted = int_metric(outcome, "tasks_attempted")
    if attempted <= 0:
        return None
    succeeded = int_metric(outcome, "tasks_succeeded")
    return max(min(succeeded / attempted, 1.0), 0.0)


def lifecycle_cause(last_event: Any) -> str:
    if not isinstance(last_event, dict):
        return "unknown_last_event"
    kind = str(last_event.get("kind") or "")
    if kind == "FileEdited":
        path = str(last_event.get("path") or "")
        if path:
            return f"open_after_file_edit:{path}"
        return "open_after_file_edit"
    if kind == "CacheMetricsRecorded":
        return "open_after_cache_metrics"
    if kind == "RunCompleted":
        detail = str(last_event.get("error_detail") or "")
        if detail == "empty_input" or detail.startswith("invalid_input:"):
            return "input_validation_exit_without_run_start"
        status = str(last_event.get("status") or "")
        if status == "completed":
            return "completion_without_run_start"
        if status == "error":
            return "run_error_without_start"
        return "run_completion_without_start"
    if kind == "ModelCallCompleted":
        return "model_completion_without_start"
    if kind == "ToolCallCompleted":
        return "open_after_tool_call"
    if kind == "CommandCompleted":
        return "open_after_command"
    if kind:
        return f"open_after_{kind}"
    return "unknown_last_event"


def lifecycle_imbalance_causes(session_dir: Path) -> list[dict[str, Any]]:
    summary = load_json(session_dir / "state" / "summary.json")
    lifecycle = summary.get("state_lifecycle") if isinstance(summary.get("state_lifecycle"), dict) else {}
    existing = lifecycle.get("imbalance_causes")
    if isinstance(existing, list):
        return [row for row in existing if isinstance(row, dict)]

    rows_by_key: dict[tuple[str, str], dict[str, Any]] = {}

    def add(category: str, items: Any) -> None:
        if not isinstance(items, list):
            return
        for item in items:
            if isinstance(item, str):
                run_id = item
                last_event: Any = None
            elif isinstance(item, dict):
                run_id = str(item.get("run_id") or "")
                last_event = item.get("last_event")
            else:
                continue
            cause = lifecycle_cause(last_event)
            key = (category, cause)
            row = rows_by_key.setdefault(
                key,
                {"category": category, "cause": cause, "count": 0, "examples": []},
            )
            row["count"] += 1
            if run_id and len(row["examples"]) < 4:
                row["examples"].append(run_id)

    runs = lifecycle.get("runs") if isinstance(lifecycle.get("runs"), dict) else {}
    model_calls = lifecycle.get("model_calls") if isinstance(lifecycle.get("model_calls"), dict) else {}
    add("run_incomplete", runs.get("incomplete_runs"))
    add("run_unmatched_completed", runs.get("unmatched_completed_details"))
    add("model_call_abnormal_completed", model_calls.get("abnormal_completed_runs"))
    add("model_call_incomplete", model_calls.get("incomplete_runs"))
    add("model_call_unmatched_completed", model_calls.get("unmatched_completed_details"))
    return sorted(
        rows_by_key.values(),
        key=lambda row: (
            -int(row.get("count") or 0),
            str(row.get("category") or ""),
            str(row.get("cause") or ""),
        ),
    )


def lifecycle_cause_summary(session_dir: Path, limit: int = 3) -> str:
    category_labels = {
        "run_incomplete": "state_incomplete",
        "run_unmatched_completed": "state_unmatched",
        "model_call_abnormal_completed": "model_abnormal",
        "model_call_incomplete": "model_incomplete",
        "model_call_unmatched_completed": "model_unmatched",
    }
    parts: list[str] = []
    for row in lifecycle_imbalance_causes(session_dir):
        category = str(row.get("category") or "")
        cause = str(row.get("cause") or "")
        if cause == "input_validation_exit_without_run_start":
            continue
        label = category_labels.get(category, category or "lifecycle")
        cause_label = "open_after_file_edit" if cause.startswith("open_after_file_edit:") else (cause or "unknown")
        try:
            count = int(row.get("count") or 0)
        except (TypeError, ValueError):
            continue
        if count <= 0:
            continue
        parts.append(f"{label}/{cause_label}={count}")
        if len(parts) >= limit:
            break
    return "; ".join(parts)


def corrected_latest_gnomes(session_dir: Path) -> dict[str, Any]:
    """Return graph-facing gnomes corrected from stronger session artifacts.

    State summaries are durable audit artifacts, so old sessions can retain stale
    log-feedback gnomes after the parser learns a better classification. Graph
    pressure feeds future prompts directly; prefer conservative corrections over
    repeating ambiguous old labels.
    """
    gnomes = dict(latest_gnomes(session_dir))
    feedback_metrics = latest_log_feedback_metrics(session_dir)
    if feedback_metrics:
        gnomes.update(feedback_metrics)
    artifact_metrics = task_artifact_verification_metrics(session_dir)
    if artifact_metrics:
        selected = int_metric(artifact_metrics, "selected_task_count")
        task_count = int_metric(artifact_metrics, "task_count")
        verified = int_metric(artifact_metrics, "task_strict_verified_count")
        if selected > 0:
            gnomes["selected_task_count"] = max(int_metric(gnomes, "selected_task_count"), selected)
        if verified > 0 or task_count > 0:
            gnomes["task_strict_verified_count"] = max(
                int_metric(gnomes, "task_strict_verified_count"),
                verified,
            )
            gnomes["task_verified_count"] = max(int_metric(gnomes, "task_verified_count"), verified)
            gnomes["tasks_succeeded"] = max(int_metric(gnomes, "tasks_succeeded"), verified)
        if task_count > 0 and verified == task_count:
            gnomes["task_success_rate"] = 1.0
            gnomes["session_success_rate"] = 1.0
            gnomes["evaluator_unverified_count"] = 0
            gnomes["task_unverified_raw_attempt_count"] = 0
            gnomes["task_unverified_raw_success_count"] = 0
        elif task_count > 0:
            seed_contradictions = int_metric(gnomes, "task_seed_contradiction_count")
            no_edit = max(int_metric(artifact_metrics, "task_no_edit_revert_count") - seed_contradictions, 0)
            explained = (
                max(seed_contradictions, int_metric(artifact_metrics, "task_no_edit_revert_count"))
                + int_metric(artifact_metrics, "task_obsolete_count")
                + int_metric(artifact_metrics, "task_api_error_count")
                + int_metric(artifact_metrics, "protected_file_revert_count")
                + int_metric(artifact_metrics, "task_scope_mismatch_count")
            )
            gnomes["evaluator_unverified_count"] = max(
                int_metric(artifact_metrics, "evaluator_unverified_raw_count")
                - explained
                - int_metric(gnomes, "task_unattempted_count"),
                0,
            )
            gnomes["task_no_edit_revert_count"] = max(
                int_metric(gnomes, "task_no_edit_revert_count"),
                no_edit,
            )
            for key in (
                "task_obsolete_count",
                "task_api_error_count",
                "protected_file_revert_count",
                "task_scope_mismatch_count",
                "task_unlanded_source_count",
            ):
                gnomes[key] = max(int_metric(gnomes, key), int_metric(artifact_metrics, key))

    if resolved_seed_replacement_metrics(gnomes):
        seed_contradictions = int_metric(gnomes, "task_seed_contradiction_count")
        gnomes["task_seed_contradiction_count"] = 0
        gnomes["task_seed_replacement_count"] = max(
            int_metric(gnomes, "task_seed_replacement_count"),
            seed_contradictions,
        )

    if bare_fatal_pattern_only(gnomes):
        gnomes["search_error_count"] = 0

    return gnomes


def compare_sessions(sessions_dir: Path, baseline: str, candidate: str) -> dict[str, Any]:
    sessions = ordered_sessions(sessions_dir)
    by_name = {path.name: path for path in sessions}
    candidate_dir = by_name.get(candidate) if candidate != "latest" else (sessions[-1] if sessions else None)
    if candidate_dir is None:
        raise ValueError(f"candidate session not found: {candidate}")
    if baseline == "previous":
        idx = sessions.index(candidate_dir)
        if idx <= 0:
            raise ValueError("no previous session available for baseline")
        baseline_dir = sessions[idx - 1]
        baseline_ref_commit = None
    else:
        baseline_dir = by_name.get(baseline)
        baseline_ref_commit = None
        if baseline_dir is None:
            baseline_dir, baseline_ref_commit = session_for_git_ref(sessions, baseline)
    if baseline_dir is None:
        raise ValueError(f"baseline session not found: {baseline}")

    base_gnomes = corrected_latest_gnomes(baseline_dir)
    cand_gnomes = corrected_latest_gnomes(candidate_dir)
    deltas: dict[str, Any] = {}
    for key in GNOME_COMPARE_KEYS:
        before = base_gnomes.get(key)
        after = cand_gnomes.get(key)
        if isinstance(before, (int, float)) and not isinstance(before, bool) and isinstance(after, (int, float)) and not isinstance(after, bool):
            deltas[key] = {"before": before, "after": after, "delta": after - before}
        elif before is not None or after is not None:
            deltas[key] = {"before": before, "after": after, "delta": None}
    return {
        "baseline_session": baseline_dir.name,
        "baseline_ref": baseline if baseline_ref_commit else None,
        "baseline_ref_commit": baseline_ref_commit,
        "candidate_session": candidate_dir.name,
        "gnome_deltas": deltas,
        "baseline_tasks": load_json(baseline_dir / "outcome.json").get("tasks_succeeded"),
        "candidate_tasks": load_json(candidate_dir / "outcome.json").get("tasks_succeeded"),
    }


def git_ref_timestamp(ref: str) -> tuple[str, str] | None:
    try:
        result = subprocess.run(
            ["git", "show", "-s", "--format=%H%x00%cI", f"{ref}^{{commit}}"],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0 or "\x00" not in result.stdout:
        return None
    sha, timestamp = result.stdout.strip().split("\x00", 1)
    return sha, timestamp


def parse_ts(value: Any) -> datetime | None:
    if not isinstance(value, str) or not value:
        return None
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None


def session_for_git_ref(sessions: list[Path], ref: str) -> tuple[Path | None, str | None]:
    resolved = git_ref_timestamp(ref)
    if resolved is None:
        return None, None
    sha, timestamp = resolved
    ref_time = parse_ts(timestamp)
    if ref_time is None:
        return None, sha
    candidates: list[tuple[datetime, Path]] = []
    for session in sessions:
        ts = parse_ts(load_json(session / "outcome.json").get("ts"))
        if ts is not None and ts <= ref_time:
            candidates.append((ts, session))
    if not candidates:
        return None, sha
    candidates.sort(key=lambda item: item[0])
    return candidates[-1][1], sha


def evolution_suggestions(session_dir: Path, limit: int = 3) -> list[dict[str, Any]]:
    gnomes = corrected_latest_gnomes(session_dir)
    manifest = task_manifest(session_dir)
    suggestions: list[dict[str, Any]] = []

    def add(kind: str, title: str, reason: str, metric: str, value: Any, priority: int) -> None:
        suggestions.append(
            {
                "kind": kind,
                "title": title,
                "reason": reason,
                "metric": metric,
                "value": value,
                "priority": priority,
            }
        )

    if int(gnomes.get("planner_no_task_count") or 0) > 0 or (manifest and (manifest.get("planner") or {}).get("planning_failed")):
        add("planner", "Make planning failure actionable", "The planner produced no concrete task files.", "planner_no_task_count", gnomes.get("planner_no_task_count"), 100)
    assessment_gap = assessment_artifact_gap(session_dir, manifest)
    if assessment_gap:
        add(
            "planning",
            "Preserve assessment artifacts",
            "Assessment evidence exists but tasks/assessment.md was not preserved, so planner context is degraded for the next DeepSeek task selection.",
            "assessment_artifact_missing_count",
            1,
            89,
        )
    spec_warning_counts = task_spec_warning_counts(manifest)
    spec_warning_total = sum(spec_warning_counts.values())
    spec_warning_detail = task_spec_warning_detail(spec_warning_counts)
    assessment_contradictions = spec_warning_counts.get("assessment_contradiction", 0)
    if assessment_contradictions:
        add(
            "planning",
            "Replace assessment-contradicted task specs",
            f"Selected task specs contradicted fresh assessment evidence ({spec_warning_detail}); replace stale seeds before implementation.",
            "task_manifest_seed_contradiction_count",
            assessment_contradictions,
            93,
        )
    missing_expected_evidence = spec_warning_counts.get("missing_expected_evidence", 0)
    if missing_expected_evidence:
        add(
            "planning",
            "Require task evidence specs",
            f"Selected task specs lacked Expected Evidence ({spec_warning_detail}), so the next implementation target may be hard to verify from task lineage, state events, or gnome movement.",
            "missing_expected_evidence_count",
            missing_expected_evidence,
            82,
        )
    other_spec_warnings = spec_warning_total - missing_expected_evidence - assessment_contradictions
    if other_spec_warnings:
        add(
            "planning",
            "Tighten selected task specs",
            f"Selected task specs had manifest quality warnings ({spec_warning_detail}); require concrete file scope, non-generic objective, and verification evidence before implementation.",
            "task_spec_warning_count",
            other_spec_warnings,
            81,
        )
    spec_quality = gnomes.get("task_spec_quality_score")
    if (
        spec_warning_total == 0
        and isinstance(spec_quality, (int, float))
        and not isinstance(spec_quality, bool)
        and spec_quality < 0.75
    ):
        add(
            "planning",
            "Tighten selected task specs",
            "Task spec quality score fell below the thin-spec threshold, but detailed manifest warnings were unavailable; require concrete file scope, non-generic objective, and expected evidence before implementation.",
            "task_spec_quality_score",
            spec_quality,
            81,
        )
    lifecycle_metrics = (
        ("deepseek_model_call_incomplete_count", "model calls incomplete"),
        ("deepseek_model_call_unmatched_completed_count", "model calls unmatched"),
        ("deepseek_model_call_abnormal_completed_count", "model calls abnormal"),
        ("state_run_incomplete_count", "runs incomplete"),
        ("state_run_unmatched_non_validation_completed_count", "runs unmatched"),
    )
    lifecycle_gaps = [
        (metric, label, int(gnomes.get(metric) or 0))
        for metric, label in lifecycle_metrics
        if int(gnomes.get(metric) or 0) > 0
    ]
    if lifecycle_gaps:
        metric, _label, value = lifecycle_gaps[0]
        detail = "; ".join(f"{label}={count}" for _metric, label, count in lifecycle_gaps)
        cause_detail = lifecycle_cause_summary(session_dir)
        reason = (
            f"Lifecycle causes: {cause_detail}; gaps: {detail}."
            if cause_detail
            else f"Lifecycle gnomes show abnormal or unpaired terminal events: {detail}."
        )
        add(
            "state",
            "Close yyds state and model lifecycle gaps",
            reason,
            metric,
            value,
            96,
        )
    operational_capture = gnomes.get("state_operational_capture_coverage")
    state_capture = gnomes.get("state_capture_coverage")
    if isinstance(operational_capture, (int, float)) and not isinstance(operational_capture, bool) and operational_capture < 1.0:
        add(
            "state",
            "Restore operational state capture",
            "Operational yoagent-state events were missing or incomplete; preserve command/tool/model lifecycle events before trusting gnome movement.",
            "state_operational_capture_coverage",
            operational_capture,
            90,
        )
    elif isinstance(state_capture, (int, float)) and not isinstance(state_capture, bool) and state_capture < 1.0:
        add(
            "state",
            "Restore state event capture",
            "yoagent-state events were missing from session evidence; preserve state/events.jsonl before audit-log session push.",
            "state_capture_coverage",
            state_capture,
            90,
        )
    replay_integrity = gnomes.get("state_replay_integrity_rate")
    if isinstance(replay_integrity, (int, float)) and not isinstance(replay_integrity, bool) and replay_integrity < 1.0:
        add(
            "state",
            "Repair state replay integrity",
            "State replay did not match recorded session artifacts; reconcile state/events.jsonl, state/summary.json, and task artifacts before trusting gnome movement.",
            "state_replay_integrity_rate",
            replay_integrity,
            92,
        )
    state_failures = int_metric(gnomes, "state_failure_count")
    if state_failures > 0:
        add(
            "state",
            "Repair recorded state failure events",
            "State metrics recorded failure events; inspect the failing state/event path and add a targeted harness guard or replay fixture before trusting downstream gnomes.",
            "state_failure_count",
            state_failures,
            86,
        )
    if int(gnomes.get("state_live_baseline_shrink_count") or 0) > 0:
        add(
            "state",
            "Keep live state append-only",
            "The live state log had fewer events than the replay baseline; inspect concurrent state writers before trusting merged session evidence.",
            "state_live_baseline_shrink_count",
            gnomes.get("state_live_baseline_shrink_count"),
            91,
        )
    recurring_failures = int_metric(gnomes, "recurring_failure_count")
    if recurring_failures > 0:
        max_recurrence = int_metric(gnomes, "max_failure_fingerprint_recurrence")
        fingerprints = gnomes.get("failure_fingerprints")
        dominant = ""
        if isinstance(fingerprints, list):
            for item in fingerprints:
                if isinstance(item, dict) and item.get("fingerprint"):
                    dominant = str(item.get("fingerprint") or "")[:90]
                    break
        detail = f" Max recurrence={max_recurrence}." if max_recurrence > 0 else ""
        if dominant:
            detail += f" Dominant fingerprint: {dominant}."
        add(
            "logs",
            "Break recurring log failure fingerprints",
            f"GitHub/action log feedback repeated failure fingerprints across sessions; inspect the dominant phase and add a targeted harness guard or eval fixture.{detail}",
            "recurring_failure_count",
            recurring_failures,
            87,
        )
    json_parse_rate = gnomes.get("json_parse_failure_rate")
    if isinstance(json_parse_rate, (int, float)) and not isinstance(json_parse_rate, bool) and json_parse_rate > 0.0:
        add(
            "deepseek",
            "Reduce DeepSeek JSON parse failures",
            "State/eval metrics recorded JSON parse failures; tighten structured-output prompts, parser recovery, or fixtures before scoring general coding reliability.",
            "json_parse_failure_rate",
            json_parse_rate,
            86,
        )
    malformed_tool_rate = gnomes.get("tool_call_malformed_rate")
    if isinstance(malformed_tool_rate, (int, float)) and not isinstance(malformed_tool_rate, bool) and malformed_tool_rate > 0.0:
        add(
            "deepseek",
            "Reduce malformed tool-call outputs",
            "State/eval metrics recorded malformed tool calls; tighten tool schema instructions, parser recovery, and tool-call fixtures before scoring coding-task reliability.",
            "tool_call_malformed_rate",
            malformed_tool_rate,
            86,
        )
    context_miss_rate = gnomes.get("context_miss_rate")
    if isinstance(context_miss_rate, (int, float)) and not isinstance(context_miss_rate, bool) and context_miss_rate > 0.0:
        add(
            "deepseek",
            "Reduce DeepSeek context misses",
            "State/eval metrics recorded context misses; move stable repo, task, and trajectory evidence into the prompt prefix or retrieval path before selecting broader coding tasks.",
            "context_miss_rate",
            context_miss_rate,
            84,
        )
    repair_loops = int_metric(gnomes, "repair_loop_count")
    if repair_loops > 0:
        add(
            "logs",
            "Reduce repair-loop churn",
            "Coding logs showed repair-loop or retry-after-failure churn; add narrower task context, earlier bounded diagnostics, or targeted fixtures so DeepSeek does not spend turns rediscovering the same failure.",
            "repair_loop_count",
            repair_loops,
            78,
        )
    if int(gnomes.get("evaluator_unverified_count") or 0) > 0:
        add("eval", "Bound evaluator checks so verdicts are not skipped", "Some task evals were unverified or timed out.", "evaluator_unverified_count", gnomes.get("evaluator_unverified_count"), 90)
    if int(gnomes.get("evaluator_timeout_with_verdict_count") or 0) > 0:
        add("eval", "Stop evaluator once verdict evidence exists", "An evaluator wrote a verdict but still timed out, making the verifier evidence ambiguous.", "evaluator_timeout_with_verdict_count", gnomes.get("evaluator_timeout_with_verdict_count"), 92)
    task_success_rate = float_metric(gnomes, "task_success_rate")
    task_success_metric = "task_success_rate"
    if task_success_rate is None:
        task_success_rate = outcome_task_success_rate(session_dir)
        task_success_metric = "outcome_task_success_rate"
    if task_success_rate is not None and task_success_rate < 1.0:
        add(
            "task",
            "Raise verified task success rate",
            "Selected or attempted tasks did not all finish as verified successful outcomes; use task artifacts, action logs, and transcripts to remove the highest-frequency failure class before optimizing secondary gnomes.",
            task_success_metric,
            task_success_rate,
            91,
        )
    session_success_rate = float_metric(gnomes, "session_success_rate")
    if session_success_rate is not None and session_success_rate < 1.0 and (
        task_success_rate is None or task_success_rate >= 1.0
    ):
        add(
            "task",
            "Raise session success rate",
            "The evo session did not complete cleanly even though task success was not the visible bottleneck; inspect build/test/revert status and make the session outcome pass end to end.",
            "session_success_rate",
            session_success_rate,
            89,
        )
    verification_rate = gnomes.get("task_verification_rate")
    if (
        isinstance(verification_rate, (int, float))
        and not isinstance(verification_rate, bool)
        and verification_rate < 1.0
        and int(gnomes.get("evaluator_unverified_count") or 0) <= 0
    ):
        add(
            "eval",
            "Require strict verifier evidence for tasks",
            "Task verification rate was below complete without a counted evaluator-unverified bucket; preserve bounded verifier artifacts before scoring task success.",
            "task_verification_rate",
            verification_rate,
            88,
        )
    mechanical_rate = gnomes.get("task_mechanical_verification_rate")
    if (
        isinstance(mechanical_rate, (int, float))
        and not isinstance(mechanical_rate, bool)
        and mechanical_rate < 1.0
        and (
            not isinstance(verification_rate, (int, float))
            or isinstance(verification_rate, bool)
            or verification_rate >= 1.0
        )
    ):
        add(
            "eval",
            "Preserve mechanical verification artifacts",
            "Mechanical task verification rate was below complete; record deterministic build, test, or eval artifacts for every selected task.",
            "task_mechanical_verification_rate",
            mechanical_rate,
            76,
        )
    if int(gnomes.get("task_unattempted_count") or 0) > 0:
        add(
            "implementation",
            "Preserve budget to start every selected task",
            "The planner selected tasks that the implementation phase never attempted.",
            "task_unattempted_count",
            gnomes.get("task_unattempted_count"),
            94,
        )
    if int(gnomes.get("task_obsolete_count") or 0) > 0:
        add(
            "planning",
            "Replace stale or already-satisfied tasks",
            "Implementation marked selected tasks obsolete or already satisfied; planning should replace stale targets or land small verification/docs improvements.",
            "task_obsolete_count",
            gnomes.get("task_obsolete_count"),
            86,
        )
    if int(gnomes.get("task_seed_contradiction_count") or 0) > 0 and not assessment_contradictions:
        add(
            "planning",
            "Validate seeded tasks against fresh assessment",
            "Seeded tasks were contradicted by assessment evidence; validate seeds before implementation and replace stale targets.",
            "task_seed_contradiction_count",
            gnomes.get("task_seed_contradiction_count"),
            89,
        )
    provider_errors = int_metric(gnomes, "provider_error_count")
    task_api_errors = int_metric(gnomes, "task_api_error_count")
    if provider_errors > 0 and provider_errors > task_api_errors:
        add(
            "deepseek",
            "Recover provider errors before task attempts",
            "DeepSeek/provider API errors appeared outside task-scoped API reverts; preserve the failure evidence and route retry or provider recovery before spending implementation attempts.",
            "provider_error_count",
            provider_errors,
            96,
        )
    if int(gnomes.get("task_api_error_count") or 0) > 0:
        add(
            "implementation",
            "Recover API-error tasks instead of generic reverts",
            "Implementation hit provider/API errors before landed work; preserve the error evidence and retry with provider recovery.",
            "task_api_error_count",
            gnomes.get("task_api_error_count"),
            91,
        )
    if int(gnomes.get("task_no_edit_revert_count") or 0) > 0:
        add(
            "implementation",
            "Force reverted tasks to leave concrete evidence",
            "Implementation tasks reverted without touching files; require an early scoped edit, obsolete note, or concrete blocker instead of analysis-only work.",
            "task_no_edit_revert_count",
            gnomes.get("task_no_edit_revert_count"),
            86,
        )
    if int(gnomes.get("task_unlanded_source_count") or 0) > 0:
        add("commit", "Make source-edit outcomes land or explain reverts", "A task touched source files without a landed source commit.", "task_unlanded_source_count", gnomes.get("task_unlanded_source_count"), 88)
    if int(gnomes.get("evaluator_timeout_count") or 0) > 0:
        add("eval", "Make evaluator timeouts resumable or cheaper", "Evaluator timeout friction still appears in action logs.", "evaluator_timeout_count", gnomes.get("evaluator_timeout_count"), 85)
    if int(gnomes.get("task_scope_mismatch_count") or 0) > 0:
        add(
            "implementation",
            "Align implementation edits with task file scope",
            "Implementation changed files outside the selected task surface; tighten task Files entries and implementation prompts.",
            "task_scope_mismatch_count",
            gnomes.get("task_scope_mismatch_count"),
            84,
        )
    if int(gnomes.get("protected_file_revert_count") or 0) > 0:
        add(
            "implementation",
            "Route protected-file work through explicit approval",
            "Evolution tasks modified protected files and were reverted; route protected workflow/release changes through explicit allowlists or human-owned issues.",
            "protected_file_revert_count",
            gnomes.get("protected_file_revert_count"),
            83,
        )
    if int(gnomes.get("tool_error_count") or 0) > 0:
        add(
            "tooling",
            "Recover failed tool actions before scoring",
            "Failed tool actions were present in session evidence; inspect the dominant tool failure and add prompt/tool guards before trusting the next score.",
            "tool_error_count",
            gnomes.get("tool_error_count"),
            82,
        )
    if int(gnomes.get("prompt_heredoc_expansion_error_count") or 0) > 0:
        add(
            "prompting",
            "Quote generated prompts before execution",
            "Prompt heredocs expanded Markdown code spans before yyds started; render prompts from quoted templates or escape generated backticks.",
            "prompt_heredoc_expansion_error_count",
            gnomes.get("prompt_heredoc_expansion_error_count"),
            81,
        )
    if int(gnomes.get("search_error_count") or 0) > 0:
        add("tooling", "Harden search commands and pattern escaping", "Search/grep errors created avoidable evolution friction.", "search_error_count", gnomes.get("search_error_count"), 80)
    if int(gnomes.get("command_timeout_count") or 0) > 0:
        add("tooling", "Prefer bounded diagnostics before broad commands", "Command timeouts slowed the coding loop.", "command_timeout_count", gnomes.get("command_timeout_count"), 75)
    max_turns = gnomes.get("max_task_turn_count")
    if isinstance(max_turns, (int, float)) and not isinstance(max_turns, bool) and max_turns >= 25:
        if fully_verified_success_metrics(gnomes):
            add(
                "planning",
                "Reduce successful-task turn overhead",
                "A verified task still used many turns, suggesting discovery or verification expanded beyond the scoped task.",
                "max_task_turn_count",
                max_turns,
                70,
            )
        else:
            add("planning", "Split high-turn tasks into narrower plans", "A task used many turns, suggesting the task was too broad or under-specified.", "max_task_turn_count", max_turns, 70)
    if gnomes.get("task_artifact_coverage") == 0:
        add("state", "Restore task artifact coverage", "Task decisions or artifacts were missing from the audit bundle.", "task_artifact_coverage", 0, 95)
    lineage_capture = gnomes.get("task_lineage_capture_coverage")
    if isinstance(lineage_capture, (int, float)) and not isinstance(lineage_capture, bool) and lineage_capture < 1.0:
        add(
            "state",
            "Restore explicit task lineage capture",
            "Task lineage was incomplete; link task_id, touched files, commit SHAs, evaluator verdicts, and gnome deltas directly in yoagent-state.",
            "task_lineage_capture_coverage",
            lineage_capture,
            89,
        )
    if int(gnomes.get("deepseek_cache_ratio_unverified_count") or 0) > 0:
        add(
            "deepseek",
            "Ignore prose-only DeepSeek cache ratios",
            "DeepSeek cache ratios were reported without token-backed cache metrics; use measured cache hit/miss tokens before optimizing prompts.",
            "deepseek_cache_ratio_unverified_count",
            gnomes.get("deepseek_cache_ratio_unverified_count"),
            63,
        )
    if int(gnomes.get("deepseek_cache_metric_missing_count") or 0) > 0:
        add(
            "deepseek",
            "Record token-backed DeepSeek cache metrics",
            "Expected DeepSeek cache metric events were missing, so prompt-cache improvements cannot be trusted yet.",
            "deepseek_cache_metric_missing_count",
            gnomes.get("deepseek_cache_metric_missing_count"),
            62,
        )
    if isinstance(gnomes.get("deepseek_cache_hit_ratio"), (int, float)) and gnomes.get("deepseek_cache_hit_ratio") < 0.5:
        add("deepseek", "Improve stable prompt prefix reuse", "DeepSeek prompt cache hit ratio is low.", "deepseek_cache_hit_ratio", gnomes.get("deepseek_cache_hit_ratio"), 60)

    suggestions.sort(key=lambda item: (-int(item["priority"]), str(item["title"])))
    return suggestions[:limit]


def print_markdown_suggestions(session_dir: Path, limit: int) -> None:
    suggestions = evolution_suggestions(session_dir, limit)
    if not suggestions:
        print("No graph-derived evolution suggestions for this session.")
        return
    print(f"Graph-derived suggestions for {session_dir.name}:")
    for idx, suggestion in enumerate(suggestions, 1):
        print(f"{idx}. {suggestion['title']} ({suggestion['metric']}={suggestion['value']})")
        print(f"   {suggestion['reason']}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    replay_parser = sub.add_parser("replay-check")
    replay_parser.add_argument("--sessions-dir", required=True, type=Path)

    chain_parser = sub.add_parser("chain")
    chain_parser.add_argument("--session-dir", required=True, type=Path)
    chain_parser.add_argument("--task-id")

    compare_parser = sub.add_parser("compare-baseline")
    compare_parser.add_argument("--sessions-dir", required=True, type=Path)
    compare_parser.add_argument("--baseline", default="previous")
    compare_parser.add_argument("--candidate", default="latest")

    suggest_parser = sub.add_parser("suggest")
    suggest_parser.add_argument("--session-dir", required=True, type=Path)
    suggest_parser.add_argument("--limit", type=int, default=3)
    suggest_parser.add_argument("--json", action="store_true")

    args = parser.parse_args()
    try:
        if args.command == "replay-check":
            json.dump(replay_check(args.sessions_dir), sys.stdout, indent=2, sort_keys=True)
            sys.stdout.write("\n")
        elif args.command == "chain":
            rows = build_causal_chains(args.session_dir)
            if args.task_id:
                rows = [row for row in rows if row.get("task_id") == args.task_id]
            json.dump({"session_id": args.session_dir.name, "causal_chains": rows}, sys.stdout, indent=2, sort_keys=True)
            sys.stdout.write("\n")
        elif args.command == "compare-baseline":
            json.dump(compare_sessions(args.sessions_dir, args.baseline, args.candidate), sys.stdout, indent=2, sort_keys=True)
            sys.stdout.write("\n")
        elif args.command == "suggest":
            if args.json:
                json.dump({"session_id": args.session_dir.name, "suggestions": evolution_suggestions(args.session_dir, args.limit)}, sys.stdout, indent=2, sort_keys=True)
                sys.stdout.write("\n")
            else:
                print_markdown_suggestions(args.session_dir, args.limit)
    except ValueError as exc:
        print(f"state_graph_tools: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
