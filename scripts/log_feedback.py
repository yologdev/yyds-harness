#!/usr/bin/env python3
"""Convert GitHub Actions coding logs into yoagent-state-compatible feedback.

This script intentionally writes log feedback as a normal PatchEvaluated eval:

  payload.suite == "log-feedback"
  payload.metrics.state_metrics contains selected gnome/KPI values

Raw logs stay out of prompts and state summaries. The saved assessment keeps
short, normalized evidence only.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


ERROR_LINE_RE = re.compile(
    r"(##\[error\]|::error::|\berror(?:\[[^\]]+\])?:|\berror\[[^\]]+\]|\bfatal:|\bpanicked\b|\bexception\b|\btraceback\b|timed out|exit code [1-9]\d*|process completed with exit code [1-9]\d*|test result: failed)",
    re.IGNORECASE,
)
PROVIDER_ERROR_RE = re.compile(
    r"(provider_error|rate_limit|rate limit|\b(?:http\s*)?(?:429|5\d\d)\b|api error|overloaded)",
    re.IGNORECASE,
)
EXPLICIT_PROVIDER_SIGNAL_RE = re.compile(
    r"(provider_error|rate_limit|rate limit|\b(?:http\s*)?(?:429|5\d\d)\b|overloaded)",
    re.IGNORECASE,
)
JSON_ERROR_RE = re.compile(r"(json|schema|deserialize|parse).*(error|fail)", re.IGNORECASE)
TOOL_ERROR_RE = re.compile(r"(tool call|tool schema|malformed tool|invalid tool)", re.IGNORECASE)
STATE_ERROR_RE = re.compile(r"(state|audit-log|events\.jsonl|state\.sqlite).*(error|fail|missing)", re.IGNORECASE)
COMMAND_TIMEOUT_RE = re.compile(r"Command timed out after (\d+)s|timed out after (\d+)s", re.IGNORECASE)
EVALUATOR_TIMEOUT_RE = re.compile(r"Evaluator:\s*timed out", re.IGNORECASE)
SEARCH_ERROR_RE = re.compile(r"\bSearch error:\s*", re.IGNORECASE)
PROTECTED_FILE_RE = re.compile(r"modified protected files:", re.IGNORECASE)
TASK_STARTED_RE = re.compile(r"(?:→|->)\s*Task\s+\d+:", re.IGNORECASE)
TASK_VERIFIED_RE = re.compile(r"\bTask\s+\d+:\s+verified OK\b", re.IGNORECASE)
TASK_REVERT_RE = re.compile(r"\bReverting Task\s+\d+\b", re.IGNORECASE)
CACHE_PERCENT_RE = re.compile(
    r"(?:cache(?: hit)? ratio(?: is)?|Cache:)\D{0,24}(\d+(?:\.\d+)?)\s*%\s*(?:hit ratio)?",
    re.IGNORECASE,
)
CACHE_TOKENS_RE = re.compile(r"([\d,]+)\s+hit tokens,\s*([\d,]+)\s+miss tokens", re.IGNORECASE)
TASK_TRANSCRIPT_RE = re.compile(r"task_(\d+)_attempt(\d+)\.log$")
TURN_MARKER_RE = re.compile(r"\bTurn\s+(\d+)\b")
ACTION_LINE_RE = re.compile(r"^\s*▶\s+", re.MULTILINE)
AUTHORIZATION_RE = re.compile(r"(?i)\bauthorization\s*[:=]\s*bearer\s+[^'\"\s]+")
BEARER_RE = re.compile(r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{8,}")
SECRET_RE = re.compile(r"(?i)\b(token|secret|password|api[_-]?key)\s*[:=]\s*['\"]?[^'\"\s]+")

MAX_EVIDENCE_LINES = 8
MAX_FINGERPRINTS = 10
LOG_FETCH_TIMEOUT_SECONDS = 45
GITHUB_LOG_TIMESTAMP_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T[\d:.]+Z$")
GNOME_KEYS = [
    "coding_log_score",
    "coding_log_confidence",
    "coding_log_available",
    "workflow_success_rate",
    "session_success_rate",
    "task_success_rate",
    "retry_success_rate",
    "recurring_failure_count",
    "max_failure_fingerprint_recurrence",
    "state_capture_coverage",
    "audit_capture_coverage",
    "closed_loop_fix_rate",
    "evolution_friction_count",
    "command_timeout_count",
    "evaluator_timeout_count",
    "search_error_count",
    "protected_file_revert_count",
    "task_revert_count",
    "task_verification_rate",
    "max_task_turn_count",
    "avg_task_turn_count",
    "total_task_turn_count",
    "deepseek_cache_hit_ratio",
    "deepseek_cache_hit_tokens",
    "deepseek_cache_miss_tokens",
]


def warn(message: str) -> None:
    print(f"log_feedback: WARN: {message}", file=sys.stderr)


def strip_ansi(value: str) -> str:
    value = re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", value)
    return re.sub(r"\^\[\[[0-9;]*[a-zA-Z]", "", value)


def redact(value: str) -> str:
    value = AUTHORIZATION_RE.sub("Authorization: Bearer <redacted>", value)
    value = BEARER_RE.sub("Bearer <redacted>", value)
    value = SECRET_RE.sub(lambda m: f"{m.group(1)}=<redacted>", value)
    value = re.sub(r"gh[psu]_[A-Za-z0-9_]{20,}", "<redacted-token>", value)
    return value


def fingerprint_error_line(line: str) -> str:
    text = redact(strip_ansi(line)).strip()
    text = re.sub(
        r"^(?:[A-Za-z_][\w-]*\s+)*\d{4}-\d{2}-\d{2}T[\d:.]+Z?\s*",
        "",
        text,
    )
    text = re.sub(r"^\d{4}-\d{2}-\d{2}T?[\d:.,Z+ ]*\s*", "", text)
    text = re.sub(r"^[A-Za-z_-]+\s*[\|│]\s*", "", text)
    text = re.sub(r":\d+:\d+", ":N:N", text)
    text = re.sub(r":\d+\b", ":N", text)
    text = re.sub(r"0x[0-9a-fA-F]{4,}", "<HEX>", text)
    text = re.sub(
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        "<UUID>",
        text,
    )
    text = re.sub(r"\b\d{5,}\b", "<N>", text)
    return re.sub(r"\s+", " ", text.lower())[:120]


def log_message(line: str) -> str:
    parts = line.split("\t", 3)
    if len(parts) == 4 and GITHUB_LOG_TIMESTAMP_RE.match(parts[2]):
        return parts[3].strip()
    parts = line.split("\t", 2)
    if len(parts) == 3:
        timestamp, _, message = parts[2].partition(" ")
        if GITHUB_LOG_TIMESTAMP_RE.match(timestamp):
            return message.strip()
    return line.strip()


def is_noise_failure_message(message: str) -> bool:
    lower = message.lower().strip()
    if not lower:
        return True
    if lower.startswith(("##[group]run ", "##[endgroup]", "#", "+", "-")):
        return True
    if lower.startswith(("ok:", "warning:", "compiling ", "checking ", "finished ")):
        return True
    if lower.startswith(("curl ", "echo ", "rtk ", "xurl ", "git ", "python3 ", "cargo ", "chmod ", "./scripts/")):
        return True
    if re.match(r"^\d+\.\s", lower):
        return True
    if "format!(" in lower or re.match(r"^[a-z_][\w:<>]*\s*=", lower):
        return True
    if "|| echo" in lower or "continue-on-error" in lower or "non-fatal" in lower or "fail-soft" in lower:
        return True
    if re.search(r"\b0 (?:failures|failed)\b", lower) or "no failures" in lower:
        return True
    if lower.startswith("test result: ok"):
        return True
    return False


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8", errors="replace"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError) as exc:
        warn(f"could not read {path}: {exc}")
        return {}
    return value if isinstance(value, dict) else {}


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        warn(f"could not read {path}: {exc}")
        return ""


def fetch_run_log(repo: str, run_id: str) -> tuple[bool, str, str]:
    if not repo or not run_id:
        return False, "", "missing repo or run_id"
    cmd = ["gh", "run", "view", run_id, "--repo", repo, "--log"]
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=LOG_FETCH_TIMEOUT_SECONDS,
            start_new_session=True,
        )
    except subprocess.TimeoutExpired:
        return False, "", "gh run view timed out"
    except (FileNotFoundError, OSError) as exc:
        return False, "", str(exc)
    if result.returncode != 0:
        return False, "", (result.stderr or result.stdout or "").strip()[:400]
    return True, result.stdout, ""


def event_count(events_path: Path) -> int:
    if not events_path.is_file():
        return 0
    count = 0
    try:
        with events_path.open(encoding="utf-8", errors="replace") as handle:
            for line in handle:
                if line.strip():
                    count += 1
    except OSError:
        return 0
    return count


def load_events(events_path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not events_path.is_file():
        return rows
    try:
        with events_path.open(encoding="utf-8", errors="replace") as handle:
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
    except OSError:
        return rows
    return rows


def event_kind(event: dict[str, Any]) -> str:
    value = event.get("event_type") or event.get("kind")
    return value if isinstance(value, str) else ""


def event_payload(event: dict[str, Any]) -> dict[str, Any]:
    value = event.get("payload")
    return value if isinstance(value, dict) else {}


def parse_log(log_text: str) -> dict[str, Any]:
    fingerprints: dict[str, dict[str, Any]] = {}
    provider_errors = 0
    json_errors = 0
    tool_errors = 0
    state_errors = 0
    repair_loop_count = 0
    retry_markers = 0
    command_timeouts = 0
    evaluator_timeouts = 0
    search_errors = 0
    protected_file_reverts = 0
    task_started = 0
    task_verified = 0
    task_reverts = 0
    cache_ratio: float | None = None
    cache_hit_tokens: int | None = None
    cache_miss_tokens: int | None = None
    evidence: list[str] = []

    for raw_line in log_text.splitlines():
        line = redact(strip_ansi(raw_line)).strip()
        if not line:
            continue
        message = log_message(line)
        lower = message.lower()
        if COMMAND_TIMEOUT_RE.search(message):
            command_timeouts += 1
        if EVALUATOR_TIMEOUT_RE.search(message):
            evaluator_timeouts += 1
        if SEARCH_ERROR_RE.search(message):
            search_errors += 1
        if PROTECTED_FILE_RE.search(message):
            protected_file_reverts += 1
        if TASK_STARTED_RE.search(message):
            task_started += 1
        if TASK_VERIFIED_RE.search(message):
            task_verified += 1
        if TASK_REVERT_RE.search(message):
            task_reverts += 1
        cache_match = CACHE_PERCENT_RE.search(message)
        if cache_match:
            try:
                cache_ratio = round(float(cache_match.group(1)) / 100.0, 6)
            except (TypeError, ValueError):
                pass
        token_match = CACHE_TOKENS_RE.search(message)
        if token_match:
            try:
                cache_hit_tokens = int(token_match.group(1).replace(",", ""))
                cache_miss_tokens = int(token_match.group(2).replace(",", ""))
                total = cache_hit_tokens + cache_miss_tokens
                if total > 0:
                    cache_ratio = round(cache_hit_tokens / total, 6)
            except (TypeError, ValueError):
                pass
        if "retry after" in lower or "waiting 15 minutes before retry" in lower:
            retry_markers += 1
        if "repair loop" in lower or "retrying after failure" in lower:
            repair_loop_count += 1
        if is_noise_failure_message(message):
            continue
        is_failure = bool(ERROR_LINE_RE.search(message))
        if is_failure and (PROVIDER_ERROR_RE.search(message) or EXPLICIT_PROVIDER_SIGNAL_RE.search(message)):
            provider_errors += 1
        if is_failure and JSON_ERROR_RE.search(message):
            json_errors += 1
        if is_failure and TOOL_ERROR_RE.search(message):
            tool_errors += 1
        if is_failure and STATE_ERROR_RE.search(message):
            state_errors += 1
        if not is_failure:
            continue
        fp = fingerprint_error_line(message)
        if not fp:
            continue
        bucket = fingerprints.setdefault(fp, {"fingerprint": fp, "count": 0, "example": line[:240]})
        bucket["count"] += 1
        if len(evidence) < MAX_EVIDENCE_LINES:
            evidence.append(line[:240])

    ordered = sorted(fingerprints.values(), key=lambda item: (-int(item["count"]), item["fingerprint"]))
    task_verification_rate = ratio(task_verified, task_started) if task_started else None
    evolution_friction_count = (
        command_timeouts
        + evaluator_timeouts
        + search_errors
        + protected_file_reverts
        + task_reverts
    )
    return {
        "failure_fingerprints": ordered[:MAX_FINGERPRINTS],
        "failure_count": sum(int(item["count"]) for item in ordered),
        "distinct_failure_count": len(ordered),
        "provider_error_count": provider_errors,
        "json_error_count": json_errors,
        "tool_error_count": tool_errors,
        "state_error_count": state_errors,
        "repair_loop_count": repair_loop_count,
        "retry_markers": retry_markers,
        "command_timeout_count": command_timeouts,
        "evaluator_timeout_count": evaluator_timeouts,
        "search_error_count": search_errors,
        "protected_file_revert_count": protected_file_reverts,
        "task_started_count": task_started,
        "task_verified_count": task_verified,
        "task_revert_count": task_reverts,
        "task_verification_rate": task_verification_rate,
        "deepseek_cache_hit_ratio": cache_ratio,
        "deepseek_cache_hit_tokens": cache_hit_tokens,
        "deepseek_cache_miss_tokens": cache_miss_tokens,
        "evolution_friction_count": evolution_friction_count,
        "evidence": evidence,
    }


def previous_feedback(session_dir: Path, limit: int = 10) -> list[dict[str, Any]]:
    root = session_dir.parent
    if not root.is_dir():
        return []
    candidates: list[tuple[float, Path]] = []
    for child in root.iterdir():
        if child == session_dir or not child.is_dir():
            continue
        path = child / "log_feedback.json"
        if not path.is_file():
            continue
        try:
            candidates.append((path.stat().st_mtime, path))
        except OSError:
            continue
    candidates.sort(reverse=True)
    return [load_json(path) for _, path in candidates[:limit]]


def find_session_for_run(sessions_dir: Path, run_id: str, run_attempt: str = "") -> str:
    if not sessions_dir.is_dir() or not run_id:
        return ""
    matches: list[tuple[float, Path]] = []
    for child in sessions_dir.iterdir():
        if not child.is_dir():
            continue
        outcome_path = child / "outcome.json"
        outcome = load_json(outcome_path)
        if str(outcome.get("github_run_id") or "") != run_id:
            continue
        if run_attempt and str(outcome.get("github_run_attempt") or "") != run_attempt:
            continue
        try:
            mtime = outcome_path.stat().st_mtime
        except OSError:
            mtime = 0.0
        matches.append((mtime, child))
    if not matches:
        return ""
    matches.sort(key=lambda item: item[0], reverse=True)
    return str(matches[0][1])


def recurrence_metrics(current: list[dict[str, Any]], previous: list[dict[str, Any]]) -> dict[str, Any]:
    current_fps = {
        str(item.get("fingerprint"))
        for item in current
        if isinstance(item, dict) and item.get("fingerprint")
    }
    prior_counts: dict[str, int] = {}
    prior_top: set[str] = set()
    for assessment in previous:
        metrics = assessment.get("metrics")
        if not isinstance(metrics, dict):
            continue
        for item in metrics.get("failure_fingerprints", []) or []:
            if not isinstance(item, dict):
                continue
            fp = item.get("fingerprint")
            if not isinstance(fp, str):
                continue
            prior_counts[fp] = prior_counts.get(fp, 0) + 1
            if len(prior_top) < 5:
                prior_top.add(fp)
    recurring = [fp for fp in current_fps if fp in prior_counts]
    max_recurrence = max((prior_counts[fp] + 1 for fp in recurring), default=0)
    if prior_top:
        closed = len([fp for fp in prior_top if fp not in current_fps])
        closed_loop_fix_rate: float | None = closed / len(prior_top)
    else:
        closed_loop_fix_rate = None
    return {
        "recurring_failure_count": len(recurring),
        "max_failure_fingerprint_recurrence": max_recurrence,
        "closed_loop_fix_rate": closed_loop_fix_rate,
    }


def gnome_values(metrics: dict[str, Any]) -> dict[str, Any]:
    return {key: metrics[key] for key in GNOME_KEYS if key in metrics}


def transcript_turn_count(text: str) -> int:
    turns: list[int] = []
    for match in TURN_MARKER_RE.finditer(text):
        try:
            turns.append(int(match.group(1)))
        except ValueError:
            continue
    if turns:
        return max(turns)
    action_count = len(ACTION_LINE_RE.findall(text))
    if action_count:
        return action_count
    return 1 if text.strip() else 0


def task_turn_metrics(session_dir: Path) -> dict[str, Any]:
    transcript_dir = session_dir / "transcripts"
    if not transcript_dir.is_dir():
        return {
            "task_turn_counts": {},
            "task_turn_attempts": [],
            "max_task_turn_count": None,
            "avg_task_turn_count": None,
            "total_task_turn_count": None,
        }
    per_task: dict[str, int] = {}
    attempts: list[dict[str, Any]] = []
    for path in sorted(transcript_dir.glob("task_*_attempt*.log")):
        match = TASK_TRANSCRIPT_RE.match(path.name)
        if not match:
            continue
        task_number = int(match.group(1))
        attempt = int(match.group(2))
        task_id = f"task_{task_number:02d}"
        turns = transcript_turn_count(read_text(path))
        per_task[task_id] = max(per_task.get(task_id, 0), turns)
        attempts.append(
            {
                "task_id": task_id,
                "task_number": task_number,
                "attempt": attempt,
                "turn_count": turns,
                "transcript": str(path.relative_to(session_dir)),
            }
        )
    counts = list(per_task.values())
    return {
        "task_turn_counts": per_task,
        "task_turn_attempts": attempts,
        "max_task_turn_count": max(counts) if counts else None,
        "avg_task_turn_count": round(sum(counts) / len(counts), 4) if counts else None,
        "total_task_turn_count": sum(counts) if counts else None,
    }


def gnome_deltas(metrics: dict[str, Any], previous: list[dict[str, Any]]) -> dict[str, Any]:
    current = gnome_values(metrics)
    previous_metrics: dict[str, Any] = {}
    for assessment in previous:
        candidate = assessment.get("metrics")
        if isinstance(candidate, dict):
            previous_metrics = gnome_values(candidate)
            break
    deltas: dict[str, Any] = {}
    for key, value in current.items():
        old = previous_metrics.get(key)
        if isinstance(value, bool) or isinstance(old, bool):
            if old is not None and old != value:
                deltas[key] = {"from": old, "to": value}
            continue
        if isinstance(value, (int, float)) and isinstance(old, (int, float)):
            deltas[key] = round(float(value) - float(old), 6)
        elif old is not None and old != value:
            deltas[key] = {"from": old, "to": value}
    return deltas


def task_lineage(session_dir: Path, metrics: dict[str, Any], deltas: dict[str, Any]) -> list[dict[str, Any]]:
    tasks: dict[str, dict[str, Any]] = {}
    for event in load_events(session_dir / "state" / "events.jsonl"):
        kind = event_kind(event)
        data = event_payload(event)
        if data.get("phase") == "task_commit_linkage" and kind in {"DecisionRecorded", "TaskLineageLinked"}:
            for linked_task in data.get("tasks", []) or []:
                if not isinstance(linked_task, dict):
                    continue
                task_id = str(linked_task.get("task_id") or "")
                if not task_id:
                    continue
                row = tasks.setdefault(
                    task_id,
                    {
                        "task_id": task_id,
                        "task_number": linked_task.get("task_number"),
                        "task_title": linked_task.get("task_title"),
                    },
                )
                existing = [str(sha) for sha in (row.get("commit_shas") or []) if sha]
                linked = [str(sha) for sha in (linked_task.get("linked_commit_shas") or []) if sha]
                row["commit_shas"] = list(dict.fromkeys(existing + linked))
                current_commits = row.get("commits") if isinstance(row.get("commits"), list) else []
                linked_commits = (
                    linked_task.get("linked_commits")
                    if isinstance(linked_task.get("linked_commits"), list)
                    else []
                )
                row["commits"] = current_commits + linked_commits
                row["commit_linkage_method"] = linked_task.get("linked_by")
            continue
        if data.get("phase") != "task":
            continue
        task_id = str(data.get("task_id") or "")
        if not task_id and data.get("task_number") is not None:
            try:
                task_id = f"task_{int(data.get('task_number')):02d}"
            except (TypeError, ValueError):
                task_id = str(data.get("task_number"))
        if not task_id:
            continue
        row = tasks.setdefault(
            task_id,
            {
                "task_id": task_id,
                "task_number": data.get("task_number"),
                "task_title": data.get("task_title"),
                "started_event_id": None,
                "completed_event_id": None,
            },
        )
        if kind == "RunStarted":
            row["started_event_id"] = event.get("event_id")
            for key in ("planned_files", "issue", "base_commit"):
                if data.get(key) is not None:
                    row[key] = data.get(key)
        elif kind == "RunCompleted":
            row["completed_event_id"] = event.get("event_id")
            for key in (
                "status",
                "head_commit",
                "touched_files",
                "source_files",
                "commit_shas",
                "commits",
                "eval",
                "revert_reason",
            ):
                value = data.get(key)
                if key in {"touched_files", "source_files", "commit_shas", "commits"} and not value:
                    continue
                if value is not None:
                    row[key] = value
            row["gnome_metrics"] = gnome_values(metrics)
            row["gnome_deltas"] = deltas
    return sorted(
        tasks.values(),
        key=lambda row: (
            row.get("task_number") if isinstance(row.get("task_number"), int) else 999,
            str(row.get("task_id") or ""),
        ),
    )


def ratio(numerator: float, denominator: float) -> float | None:
    if denominator <= 0:
        return None
    return numerator / denominator


def score_assessment(metrics: dict[str, Any]) -> float:
    task_rate = metrics.get("task_success_rate")
    outcome_parts = [
        1.0 if metrics.get("workflow_success") else 0.0,
        1.0 if metrics.get("session_success") else 0.0,
        float(task_rate) if isinstance(task_rate, (int, float)) else 0.5,
        1.0 if not metrics.get("session_reverted") else 0.0,
    ]
    outcome = sum(outcome_parts) / len(outcome_parts)

    failure_pressure = min(
        1.0,
        (
            float(metrics.get("distinct_failure_count") or 0)
            + float(metrics.get("provider_error_count") or 0)
            + float(metrics.get("json_error_count") or 0)
            + float(metrics.get("tool_error_count") or 0)
            + float(metrics.get("recurring_failure_count") or 0) * 2.0
            + float(metrics.get("evolution_friction_count") or 0)
        )
        / 12.0,
    )
    capture = (
        float(metrics.get("state_capture_coverage") or 0.0)
        + float(metrics.get("audit_capture_coverage") or 0.0)
    ) / 2.0
    reliability = max(0.0, (1.0 - failure_pressure) * 0.75 + capture * 0.25)

    repair_pressure = min(1.0, float(metrics.get("repair_loop_count") or 0) / 6.0)
    efficiency = 1.0 - repair_pressure

    closed = metrics.get("closed_loop_fix_rate")
    learning = float(closed) if isinstance(closed, (int, float)) else 0.5

    return round(outcome * 0.40 + reliability * 0.25 + efficiency * 0.20 + learning * 0.15, 4)


def build_assessment(
    session_dir: Path,
    log_available: bool,
    log_error: str,
    log_text: str,
    repo: str,
    run_id: str,
    run_attempt: str,
    workflow_conclusion: str,
) -> dict[str, Any]:
    outcome = load_json(session_dir / "outcome.json")
    state_events = event_count(session_dir / "state" / "events.jsonl")
    audit_exists = (session_dir / "audit.jsonl").is_file()
    parsed = parse_log(log_text) if log_available else parse_log("")
    turn_metrics = task_turn_metrics(session_dir)
    previous = previous_feedback(session_dir)
    recurrences = recurrence_metrics(parsed["failure_fingerprints"], previous)

    attempted = int(outcome.get("tasks_attempted") or 0)
    succeeded = int(outcome.get("tasks_succeeded") or 0)
    task_success_rate = ratio(succeeded, attempted)
    workflow_success = workflow_conclusion.lower() in {"success", "passed"}
    build_ok = bool(outcome.get("build_ok"))
    test_ok = bool(outcome.get("test_ok"))
    reverted = bool(outcome.get("reverted"))
    session_success = bool(
        build_ok
        and test_ok
        and not reverted
        and (attempted == 0 or succeeded >= attempted)
    )
    retry_success_rate = None
    if parsed["retry_markers"]:
        retry_success_rate = 1.0 if workflow_success else 0.0

    confidence = 0.0
    if log_available:
        confidence += 0.45
    if outcome:
        confidence += 0.20
    if state_events > 0:
        confidence += 0.20
    if audit_exists:
        confidence += 0.15

    state_capture_coverage = 1.0 if state_events > 0 else 0.0
    audit_capture_coverage = 1.0 if audit_exists else 0.0
    metrics: dict[str, Any] = {
        "coding_log_available": log_available,
        "coding_log_confidence": round(confidence, 4),
        "workflow_success": workflow_success,
        "workflow_conclusion": workflow_conclusion or "unknown",
        "session_success": session_success,
        "session_reverted": reverted,
        "workflow_success_rate": 1.0 if workflow_success else 0.0,
        "session_success_rate": 1.0 if session_success else 0.0,
        "task_success_rate": task_success_rate,
        "tasks_attempted": attempted,
        "tasks_succeeded": succeeded,
        "retry_success_rate": retry_success_rate,
        "state_capture_coverage": state_capture_coverage,
        "audit_capture_coverage": audit_capture_coverage,
        "state_event_count": state_events,
        **parsed,
        **turn_metrics,
        **recurrences,
    }
    metrics["coding_log_score"] = score_assessment(metrics)
    deltas = gnome_deltas(metrics, previous)

    return {
        "schema_version": 1,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "repo": repo,
        "run_id": run_id,
        "run_attempt": run_attempt,
        "session_id": session_dir.name,
        "log_available": log_available,
        "log_error": log_error,
        "metrics": metrics,
        "gnome_deltas": deltas,
        "top_lessons": top_lessons(metrics),
    }


def top_lessons(metrics: dict[str, Any]) -> list[dict[str, Any]]:
    lessons: list[dict[str, Any]] = []
    if int(metrics.get("protected_file_revert_count") or 0) > 0:
        lessons.append(
            {
                "kind": "protected_file_revert",
                "fingerprint": "evolution task modified protected files and was reverted",
                "action": "route protected workflow/release changes through human-owned issues or explicit allowlists",
            }
        )
    if int(metrics.get("evaluator_timeout_count") or 0) > 0:
        lessons.append(
            {
                "kind": "evaluator_timeout",
                "fingerprint": "task evaluator timed out after build/test passed",
                "action": "make evaluator checks cheaper, bounded, or resumable so quality evidence is not skipped",
            }
        )
    if int(metrics.get("command_timeout_count") or 0) > 0:
        lessons.append(
            {
                "kind": "command_timeout",
                "fingerprint": "agent commands timed out during evolution",
                "action": "prefer bounded diagnostics and targeted commands before broad cargo/state scans",
            }
        )
    if int(metrics.get("search_error_count") or 0) > 0:
        lessons.append(
            {
                "kind": "search_error",
                "fingerprint": "search tool or grep produced an error",
                "action": "escape generated search patterns and avoid binary/.git/target paths in evidence scans",
            }
        )
    max_turns = metrics.get("max_task_turn_count")
    if isinstance(max_turns, (int, float)) and max_turns >= 16:
        lessons.append(
            {
                "kind": "high_task_turn_count",
                "fingerprint": f"max task turn count is high: {int(max_turns)}",
                "action": "split broad tasks earlier or add task-specific context so implementation converges in fewer turns",
            }
        )
    cache_ratio = metrics.get("deepseek_cache_hit_ratio")
    if isinstance(cache_ratio, (int, float)) and cache_ratio < 0.70:
        lessons.append(
            {
                "kind": "deepseek_cache_utilization",
                "fingerprint": f"DeepSeek cache hit ratio below target: {cache_ratio:.3f}",
                "action": "move stable identity, policy, schema, and repo map content earlier in the prompt prefix",
            }
        )
    lessons = lessons[:3]
    for item in metrics.get("failure_fingerprints", []) or []:
        if not isinstance(item, dict):
            continue
        if len(lessons) >= 3:
            break
        fp = str(item.get("fingerprint") or "")
        if not fp:
            continue
        recurring = int(metrics.get("recurring_failure_count") or 0) > 0
        lessons.append(
            {
                "kind": "recurring_failure" if recurring else "failure",
                "fingerprint": fp,
                "count": item.get("count"),
                "action": "inspect the failing phase and add a targeted harness guard or eval fixture",
            }
        )
    if not lessons and not metrics.get("coding_log_available"):
        lessons.append(
            {
                "kind": "missing_log",
                "fingerprint": "github actions log unavailable",
                "action": "check workflow token actions:read permission and gh run view access",
            }
        )
    if len(lessons) < 3 and float(metrics.get("state_capture_coverage") or 0) < 1.0:
        lessons.append(
            {
                "kind": "missing_state",
                "fingerprint": "yoagent-state events missing from session evidence",
                "action": "preserve state/events.jsonl before audit-log session push",
            }
        )
    return lessons[:3]


def append_patch_evaluated(session_dir: Path, assessment: dict[str, Any]) -> Path:
    events_path = session_dir / "state" / "events.jsonl"
    events_path.parent.mkdir(parents=True, exist_ok=True)
    now_ms = int(time.time() * 1000)
    run_id = str(assessment.get("run_id") or "unknown")
    run_attempt = str(assessment.get("run_attempt") or "")
    metrics = assessment["metrics"]
    deltas = assessment.get("gnome_deltas") if isinstance(assessment.get("gnome_deltas"), dict) else {}
    tasks = task_lineage(session_dir, metrics, deltas)
    score = float(metrics["coding_log_score"])
    status = "passed" if score >= 0.75 else "failed"
    failures = int(metrics.get("distinct_failure_count") or 0)
    passed = 1 if bool(metrics.get("workflow_success")) else 0
    eval_payload = {
        "eval_id": f"log-feedback-{run_id}-{now_ms}",
        "harness_version": "github-actions-log-feedback",
        "patch_id": None,
        "suite": "log-feedback",
        "status": status,
        "score": score,
        "passed": passed,
        "failed": failures + (0 if passed else 1),
        "metrics": {
            "state_metrics": metrics,
            "log_feedback": {
                "repo": assessment.get("repo"),
                "run_id": run_id,
                "run_attempt": run_attempt,
                "session_id": assessment.get("session_id"),
                "top_lessons": assessment.get("top_lessons", []),
                "evidence": metrics.get("evidence", []),
                "gnome_deltas": deltas,
                "task_turn_counts": metrics.get("task_turn_counts", {}),
                "task_turn_attempts": metrics.get("task_turn_attempts", []),
                "task_lineage": {"tasks": tasks},
            },
        },
        "failure_event_ids": [],
        "created_at_ms": now_ms,
    }
    event = {
        "event_id": f"evt-log-feedback-{hashlib.sha1(f'{run_id}-{run_attempt}-{now_ms}'.encode()).hexdigest()[:16]}",
        "event_type": "PatchEvaluated",
        "schema_version": 1,
        "timestamp_ms": now_ms,
        "actor": "harness",
        "run_id": f"github-actions-{run_id}",
        "session_id": assessment.get("session_id"),
        "trace_id": f"trace-log-feedback-{run_id}-{run_attempt or 'attempt-unknown'}",
        "parent_event_ids": [],
        "payload": eval_payload,
    }
    with events_path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(event, sort_keys=True) + "\n")
    return events_path


def write_assessment(session_dir: Path, assessment: dict[str, Any], append_state: bool) -> None:
    out_path = session_dir / "log_feedback.json"
    out_path.write_text(json.dumps(assessment, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if append_state:
        append_patch_evaluated(session_dir, assessment)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--session-dir", type=Path)
    parser.add_argument("--sessions-dir", type=Path)
    parser.add_argument("--print-session-for-run", action="store_true")
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    parser.add_argument("--run-id", default=os.environ.get("GITHUB_RUN_ID", ""))
    parser.add_argument("--run-attempt", default=os.environ.get("GITHUB_RUN_ATTEMPT", ""))
    parser.add_argument("--workflow-conclusion", default=os.environ.get("YOYO_WORKFLOW_CONCLUSION", "unknown"))
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--no-fetch", action="store_true")
    parser.add_argument("--no-append-state", action="store_true")
    args = parser.parse_args()

    if args.print_session_for_run:
        print(find_session_for_run(args.sessions_dir or Path("sessions"), args.run_id, args.run_attempt))
        return 0

    session_dir = args.session_dir
    if session_dir is None:
        parser.error("--session-dir is required unless --print-session-for-run is used")
    if not session_dir.is_dir():
        warn(f"session dir does not exist: {session_dir}")
        return 1

    log_available = False
    log_error = ""
    log_text = ""
    if args.log_file:
        log_text = read_text(args.log_file)
        log_available = bool(log_text)
        if not log_available:
            log_error = f"empty or unreadable log file: {args.log_file}"
    elif not args.no_fetch:
        log_available, log_text, log_error = fetch_run_log(args.repo, args.run_id)
        if log_error:
            warn(log_error)
    else:
        log_error = "fetch disabled"

    assessment = build_assessment(
        session_dir=session_dir,
        log_available=log_available,
        log_error=log_error,
        log_text=log_text,
        repo=args.repo,
        run_id=args.run_id,
        run_attempt=args.run_attempt,
        workflow_conclusion=args.workflow_conclusion,
    )
    write_assessment(session_dir, assessment, append_state=not args.no_append_state)
    print(
        "log feedback: "
        f"score={assessment['metrics']['coding_log_score']} "
        f"confidence={assessment['metrics']['coding_log_confidence']} "
        f"session={session_dir.name}"
    )
    return 0


def run_self_tests() -> int:
    failures = 0

    def check(label: str, condition: bool, detail: str = "") -> None:
        nonlocal failures
        if condition:
            print(f"  ok: {label}")
            return
        failures += 1
        print(f"  FAIL: {label} {detail}")

    print("=== log_feedback self-tests ===\n")
    fp_a = fingerprint_error_line("build test 2026-06-06T10:00:00.1Z error[E0308]: src/main.rs:42:9: type mismatch")
    fp_b = fingerprint_error_line("build test 2026-06-07T10:00:00.1Z error[E0308]: src/main.rs:99:3: type mismatch")
    check("fingerprints normalize timestamps and line numbers", fp_a == fp_b, f"{fp_a!r} != {fp_b!r}")

    parsed = parse_log(
        "\n".join(
            [
                "Retry after 15min",
                "2026-06-06T00:00:00Z error: provider_error rate_limit",
                "test | FAILED cargo test exit code 101",
                "warning: unrelated",
            ]
        )
    )
    check("provider errors counted", parsed["provider_error_count"] == 1)
    check("failure fingerprints captured", parsed["distinct_failure_count"] >= 2)
    check("retry markers captured", parsed["retry_markers"] == 1)
    noisy = parse_log(
        "\n".join(
            [
                "evolve\tSetup Rust\t2026-06-06T14:57:25.8102318Z curl --retry 10 https://example.com | sh",
                "evolve\tInstall RTK\t2026-06-06T14:57:39.5657958Z rtk --version || echo \"RTK install failed; agent will use native compressor\"",
                "evolve\tRun evolution session\t2026-06-06T16:29:49.6008331Z   + accumulating evidence -- every tool call, every failure, every eval result",
                "evolve\tRun evolution session\t2026-06-06T15:17:55.6946402Z   Events: 265 total (48 runs, 0 failures)",
                "evolve\tTests\t2026-06-06T15:00:44.7238078Z test result: ok. 0 passed; 0 failed; finished in 0.00s",
                "evolve\tInstall xurl\t2026-06-06T14:57:41.9956725Z Compiling proc-macro-error v1.0.4",
                "evolve\tRun evolution session\t2026-06-06T15:39:56.0227640Z store_schema = format!(\"SQLite integrity OK, schema version error: {e}\");",
                "evolve\tRun evolution session\t2026-06-06T15:10:45.1599289Z 1. The `context explain` timed out - that might be a real bug worth fixing",
            ]
        )
    )
    check("benign action log lines are not failures", noisy["distinct_failure_count"] == 0, noisy["failure_fingerprints"])
    check("timestamps and retry counts are not provider errors", noisy["provider_error_count"] == 0, noisy["provider_error_count"])
    operational = parse_log(
        "\n".join(
            [
                "evolve\tRun evolution session\t2026-06-07T04:26:23Z     │ Command timed out after 60s",
                "evolve\tRun evolution session\t2026-06-07T05:05:46Z    Evaluator: timed out — skipping eval (build+test passed)",
                "evolve\tRun evolution session\t2026-06-07T04:50:22Z ^G    BLOCKED: Task 2 modified protected files: .github/workflows/ci.yml",
                "evolve\tRun evolution session\t2026-06-07T04:50:22Z     Reverting Task 2 (resetting to 041da74)",
                "evolve\tRun evolution session\t2026-06-07T04:24:55Z     │ Search error: grep: ./target/debug/deps/yyds: binary file matches",
                "evolve\tRun evolution session\t2026-06-07T04:24:22Z   → Task 1: First real eval run",
                "evolve\tRun evolution session\t2026-06-07T04:33:47Z     Task 1: verified OK",
                "evolve\tRun evolution session\t2026-06-07T04:09:25Z - Cache: 84.38% hit ratio, 572,800 hit tokens, 106,004 miss tokens",
            ]
        )
    )
    check("command timeouts counted", operational["command_timeout_count"] == 1, operational)
    check("evaluator timeouts counted", operational["evaluator_timeout_count"] == 1, operational)
    check("protected file reverts counted", operational["protected_file_revert_count"] == 1, operational)
    check("task reverts counted", operational["task_revert_count"] == 1, operational)
    check("search errors counted", operational["search_error_count"] == 1, operational)
    check("task verification rate derived", operational["task_verification_rate"] == 1.0, operational)
    check("cache hit tokens parsed", operational["deepseek_cache_hit_tokens"] == 572800, operational)
    check("cache miss tokens parsed", operational["deepseek_cache_miss_tokens"] == 106004, operational)
    check(
        "cache hit ratio parsed from tokens",
        abs(float(operational["deepseek_cache_hit_ratio"]) - 0.843842) < 0.00001,
        operational,
    )
    lesson_kinds = [
        lesson["kind"]
        for lesson in top_lessons({**operational, "coding_log_available": True})
    ]
    check("operational lessons prioritize concrete friction", "protected_file_revert" in lesson_kinds, lesson_kinds)
    check("explicit transcript turn markers counted", transcript_turn_count("╭─ Turn 2 ─╮\n▶ read\n╭─ Turn 15 ─╮\n▶ test\n") == 15)
    check("transcript action fallback counted", transcript_turn_count("▶ read\n▶ search\n") == 2)
    redacted = redact("error: Authorization: Bearer sk-super-secret-token-1234567890 failed")
    check("authorization bearer token redacted", "sk-super-secret" not in redacted, redacted)
    check("authorization prefix preserved", "Authorization: Bearer <redacted>" in redacted, redacted)
    bearer = redact("fatal: upstream said Bearer ghp_abcdefghijklmnopqrstuvwxyz123456")
    check("bare bearer token redacted", "ghp_abcdefghijklmnopqrstuvwxyz" not in bearer, bearer)

    metrics = {
        "workflow_success": True,
        "session_success": True,
        "task_success_rate": 1.0,
        "session_reverted": False,
        "distinct_failure_count": 0,
        "provider_error_count": 0,
        "json_error_count": 0,
        "tool_error_count": 0,
        "recurring_failure_count": 0,
        "state_capture_coverage": 1.0,
        "audit_capture_coverage": 1.0,
        "repair_loop_count": 0,
        "closed_loop_fix_rate": None,
    }
    check("healthy score is high", score_assessment(metrics) > 0.85)
    bad = dict(metrics)
    bad.update({"workflow_success": False, "session_success": False, "distinct_failure_count": 8})
    check("failed score is lower", score_assessment(bad) < score_assessment(metrics))
    check("missing session selection is empty", find_session_for_run(Path("/does/not/exist"), "1") == "")
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        session = root / "session"
        transcript_dir = session / "transcripts"
        transcript_dir.mkdir(parents=True)
        (transcript_dir / "task_01_attempt1.log").write_text(
            "╭─ Turn 2 ─╮\n▶ read\n╭─ Turn 15 ─╮\n▶ cargo test\n",
            encoding="utf-8",
        )
        (transcript_dir / "task_01_attempt2.log").write_text(
            "▶ read\n▶ search\n▶ test\n",
            encoding="utf-8",
        )
        (transcript_dir / "task_02_attempt1.log").write_text(
            "╭─ Turn 4 ─╮\n▶ edit\n",
            encoding="utf-8",
        )
        turns = task_turn_metrics(session)
        check("task turn max per task counted", turns["task_turn_counts"] == {"task_01": 15, "task_02": 4}, turns)
        check("max task turn gnome counted", turns["max_task_turn_count"] == 15, turns)
        check("avg task turn gnome counted", turns["avg_task_turn_count"] == 9.5, turns)
        check("total task turn gnome counted", turns["total_task_turn_count"] == 19, turns)

        attempt1 = root / "attempt-1"
        attempt2 = root / "attempt-2"
        attempt1.mkdir()
        attempt2.mkdir()
        (attempt1 / "outcome.json").write_text(
            json.dumps({"github_run_id": "run-1", "github_run_attempt": "1"}),
            encoding="utf-8",
        )
        (attempt2 / "outcome.json").write_text(
            json.dumps({"github_run_id": "run-1", "github_run_attempt": "2"}),
            encoding="utf-8",
        )
        check(
            "session selection matches run attempt",
            find_session_for_run(root, "run-1", "2") == str(attempt2),
        )
        check(
            "session selection rejects missing attempt",
            find_session_for_run(root, "run-1", "3") == "",
        )

    print(f"\n{'ALL PASSED' if failures == 0 else f'{failures} FAILURE(S)'}")
    return 1 if failures else 0


if __name__ == "__main__":
    if "--test" in sys.argv:
        sys.exit(run_self_tests())
    sys.exit(main())
