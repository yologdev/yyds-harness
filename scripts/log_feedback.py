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

from state_graph_tools import replay_check_session


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
EVALUATOR_UNVERIFIED_RE = re.compile(
    r"Evaluator:\s*(?:timed out|API error|no verdict produced|unrecognized verdict)",
    re.IGNORECASE,
)
SEARCH_ERROR_RE = re.compile(r"\bSearch error:\s*", re.IGNORECASE)
PLANNER_NO_TASK_RE = re.compile(r"Planning agent produced 0 tasks", re.IGNORECASE)
PROTECTED_FILE_RE = re.compile(r"modified protected files(?:\s*:|\s+[—-]\s+reverting)", re.IGNORECASE)
TASK_STARTED_RE = re.compile(r"(?:→|->)\s*Task\s+\d+:", re.IGNORECASE)
TASK_VERIFIED_RE = re.compile(r"\bTask\s+\d+:\s+verified OK\b", re.IGNORECASE)
TASK_REVERT_RE = re.compile(r"\bReverting Task\s+\d+\b", re.IGNORECASE)
CACHE_PERCENT_RE = re.compile(
    r"(?:cache(?: hit)? ratio(?: is)?|Cache:)\D{0,24}(\d+(?:\.\d+)?)\s*%\s*(?:hit ratio)?",
    re.IGNORECASE,
)
CACHE_TOKENS_RE = re.compile(r"([\d,]+)\s+hit tokens,\s*([\d,]+)\s+miss tokens", re.IGNORECASE)
YOYO_USAGE_CACHE_RE = re.compile(
    r"tokens:\s*([\d,]+)\s+in\s*/\s*([\d,]+)\s+out\s*\[cache:\s*([\d,]+)\s+read,\s*([\d,]+)\s+write\]",
    re.IGNORECASE,
)
PROMPT_CACHE_HIT_RE = re.compile(r"\bprompt_cache_hit_tokens\b['\"]?\s*[:=]\s*([\d,]+)", re.IGNORECASE)
PROMPT_CACHE_MISS_RE = re.compile(r"\bprompt_cache_miss_tokens\b['\"]?\s*[:=]\s*([\d,]+)", re.IGNORECASE)
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
OPERATIONAL_STATE_EVENTS = {
    "ToolCallStarted",
    "ToolCallCompleted",
    "CommandStarted",
    "CommandCompleted",
    "FileRead",
    "FileEdited",
    "ModelCallStarted",
    "ModelCallCompleted",
    "CacheMetricsRecorded",
    "FailureObserved",
    "TestStarted",
    "TestCompleted",
}
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
    "state_operational_capture_coverage",
    "state_live_baseline_shrink_count",
    "audit_capture_coverage",
    "closed_loop_fix_rate",
    "evolution_friction_count",
    "command_timeout_count",
    "evaluator_timeout_count",
    "search_error_count",
    "protected_file_revert_count",
    "task_revert_count",
    "task_verification_rate",
    "task_mechanical_verification_rate",
    "planner_no_task_count",
    "task_unattempted_count",
    "task_manifest_available",
    "task_artifact_coverage",
    "task_lineage_capture_coverage",
    "task_lineage_event_count",
    "task_spec_quality_score",
    "state_replay_integrity_rate",
    "evaluator_unverified_count",
    "evaluator_timeout_with_verdict_count",
    "task_unlanded_source_count",
    "max_task_turn_count",
    "avg_task_turn_count",
    "total_task_turn_count",
    "deepseek_cache_hit_ratio",
    "deepseek_cache_hit_tokens",
    "deepseek_cache_miss_tokens",
    "deepseek_cache_ratio_unverified_count",
    "deepseek_cache_metric_event_count",
    "deepseek_cache_metric_expected_count",
    "deepseek_cache_metric_missing_count",
]


def explicit_pass(value: Any) -> bool:
    text = str(value or "").strip().lower()
    return text in {"pass", "passed", "ok", "success"} or text.startswith("pass:")


def explicit_fail(value: Any) -> bool:
    text = str(value or "").strip().lower()
    return text in {"fail", "failed", "failure"} or text.startswith("fail:")


def eval_timed_out_after_verdict(row: dict[str, Any]) -> bool:
    if int(row.get("exit_code") or 0) != 124:
        return False
    return (
        explicit_pass(row.get("status"))
        or explicit_pass(row.get("verdict"))
        or explicit_fail(row.get("status"))
        or explicit_fail(row.get("verdict"))
        or bool(row.get("verdict_file"))
    )


def parse_count(text: str) -> int:
    return int(str(text).replace(",", ""))


def clean_eval_pass(row: dict[str, Any]) -> bool:
    return not eval_timed_out_after_verdict(row) and (
        explicit_pass(row.get("status")) or explicit_pass(row.get("verdict"))
    )


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
    if lower.startswith(">"):
        return True
    if lower.startswith(("**edit ", "**step ", "**check ", "**verification ")):
        return True
    if lower.startswith(('"', "'", "`")):
        return True
    if re.match(
        r"^(let me|interesting!?|actually\b|but wait\b|wait\b|so\b|ok,?\s+so\b|unless\b|i need\b|i can\b|i should\b|looking at\b|there are\b|there's\b|these are\b|that [\"']|the task\b|the test\b|the binary\b|the implementation\b|the crashes?\b|the key issue\b|the most common path\b|the grep showed\b|now\b|good\b|that's expected\b|the evaluator agent needs\b)",
        lower,
    ):
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
    if re.match(
        r"^(?:[a-z_][\w:<>]*!?\s*\(|[a-z_][\w:<>]*::|crate::|self\.|return\b|let\b|if\b|match\b|for\b|while\b|fn\b|pub\b|use\b)",
        lower,
    ):
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


def state_trace_metrics(session_dir: Path) -> dict[str, Any]:
    events = load_events(session_dir / "state" / "events.jsonl")
    operational_count = sum(1 for event in events if event_kind(event) in OPERATIONAL_STATE_EVENTS)
    lineage_count = sum(1 for event in events if event_kind(event) == "TaskLineageLinked")
    return {
        "state_event_count": len(events),
        "state_capture_coverage": 1.0 if events else 0.0,
        "state_operational_event_count": operational_count,
        "state_operational_capture_coverage": 1.0 if operational_count else 0.0,
        "task_lineage_event_count": lineage_count,
        "task_lineage_capture_coverage": 1.0 if lineage_count else 0.0,
    }


def state_pipeline_metrics(session_dir: Path) -> dict[str, Any]:
    merge = load_json(session_dir / "state" / "merge_state_delta.json")
    baseline_shrunk = bool(merge.get("baseline_shrunk")) if merge else False
    baseline_reset = bool(merge.get("baseline_reset")) if merge else False
    if baseline_shrunk and not baseline_reset:
        # Legacy artifacts written before baseline_reset existed have this
        # exact shape when yyds rebuilt the live state projection from replayed
        # audit evidence, then appended only current-session live events.
        try:
            legacy_projection_reset = (
                int(merge.get("effective_base_lines") or 0) == 0
                and int(merge.get("base_lines") or 0) > int(merge.get("live_events") or 0)
                and int(merge.get("added") or 0) == int(merge.get("live_events") or 0)
                and int(merge.get("session_events_before") or 0) <= 10
            )
        except (TypeError, ValueError):
            legacy_projection_reset = False
        if legacy_projection_reset:
            baseline_shrunk = False
    return {
        "state_live_baseline_shrink_count": 1 if baseline_shrunk else 0,
    }


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
    task_mechanical_verified = 0
    task_evaluator_verified = 0
    task_reverts = 0
    planner_no_tasks = 0
    evaluator_unverified = 0
    current_task_eval_infra_failed = False
    cache_ratio: float | None = None
    cache_hit_tokens: int | None = None
    cache_miss_tokens: int | None = None
    cache_prose_mentions = 0
    evidence: list[str] = []

    for raw_line in log_text.splitlines():
        line = redact(strip_ansi(raw_line)).strip()
        if not line:
            continue
        message = log_message(line)
        lower = message.lower()
        noise = is_noise_failure_message(message)
        if not noise and COMMAND_TIMEOUT_RE.search(message):
            command_timeouts += 1
        if not noise and EVALUATOR_TIMEOUT_RE.search(message):
            evaluator_timeouts += 1
        if not noise and EVALUATOR_UNVERIFIED_RE.search(message):
            evaluator_unverified += 1
            current_task_eval_infra_failed = True
        if PLANNER_NO_TASK_RE.search(message):
            planner_no_tasks += 1
        if not noise and SEARCH_ERROR_RE.search(message):
            search_errors += 1
        if not noise and PROTECTED_FILE_RE.search(message):
            protected_file_reverts += 1
        if TASK_STARTED_RE.search(message):
            task_started += 1
            current_task_eval_infra_failed = False
        if TASK_VERIFIED_RE.search(message):
            task_mechanical_verified += 1
            if not current_task_eval_infra_failed:
                task_evaluator_verified += 1
        if TASK_REVERT_RE.search(message):
            task_reverts += 1
        token_match = CACHE_TOKENS_RE.search(message)
        if token_match:
            try:
                cache_hit_tokens = parse_count(token_match.group(1))
                cache_miss_tokens = parse_count(token_match.group(2))
                total = cache_hit_tokens + cache_miss_tokens
                if total > 0:
                    cache_ratio = round(cache_hit_tokens / total, 6)
            except (TypeError, ValueError):
                pass
        prompt_hit_match = PROMPT_CACHE_HIT_RE.search(message)
        prompt_miss_match = PROMPT_CACHE_MISS_RE.search(message)
        if prompt_hit_match and prompt_miss_match:
            try:
                cache_hit_tokens = parse_count(prompt_hit_match.group(1))
                cache_miss_tokens = parse_count(prompt_miss_match.group(1))
                total = cache_hit_tokens + cache_miss_tokens
                if total > 0:
                    cache_ratio = round(cache_hit_tokens / total, 6)
            except (TypeError, ValueError):
                pass
        yoyo_usage_match = YOYO_USAGE_CACHE_RE.search(message)
        if yoyo_usage_match:
            try:
                # yoagent maps DeepSeek prompt_cache_miss_tokens into Usage.input
                # and prompt_cache_hit_tokens into Usage.cache_read.
                cache_miss_tokens = parse_count(yoyo_usage_match.group(1))
                cache_hit_tokens = parse_count(yoyo_usage_match.group(3))
                total = cache_hit_tokens + cache_miss_tokens
                if total > 0:
                    cache_ratio = round(cache_hit_tokens / total, 6)
            except (TypeError, ValueError):
                pass
        cache_match = CACHE_PERCENT_RE.search(message)
        if cache_match and not (token_match or yoyo_usage_match or (prompt_hit_match and prompt_miss_match)):
            cache_prose_mentions += 1
        if "retry after" in lower or "waiting 15 minutes before retry" in lower:
            retry_markers += 1
        if not noise and ("repair loop" in lower or "retrying after failure" in lower):
            repair_loop_count += 1
        if noise:
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
    task_verification_rate = ratio(task_evaluator_verified, task_started) if task_started else None
    task_mechanical_verification_rate = ratio(task_mechanical_verified, task_started) if task_started else None
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
        "task_verified_count": task_evaluator_verified,
        "task_mechanical_verified_count": task_mechanical_verified,
        "task_revert_count": task_reverts,
        "task_verification_rate": task_verification_rate,
        "task_mechanical_verification_rate": task_mechanical_verification_rate,
        "planner_no_task_count": planner_no_tasks,
        "evaluator_unverified_count": evaluator_unverified,
        "deepseek_cache_hit_ratio": cache_ratio,
        "deepseek_cache_hit_tokens": cache_hit_tokens,
        "deepseek_cache_miss_tokens": cache_miss_tokens,
        "deepseek_cache_prose_mention_count": cache_prose_mentions,
        "deepseek_cache_ratio_unverified_count": cache_prose_mentions if cache_prose_mentions and cache_ratio is None else 0,
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


def payload_int(payload: dict[str, Any], *keys: str) -> int | None:
    for key in keys:
        value = payload.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, int):
            return value
        if isinstance(value, str):
            try:
                return parse_count(value)
            except ValueError:
                continue
    return None


def cache_metrics_expected(payload: dict[str, Any]) -> bool:
    genome = payload.get("harness_genome") if isinstance(payload.get("harness_genome"), dict) else {}
    cache_policy = genome.get("cache_policy") if isinstance(genome.get("cache_policy"), dict) else {}
    record_metrics = cache_policy.get("record_metrics") is True
    model = payload.get("model")
    provider = payload.get("provider")
    deepseek_native = payload.get("deepseek_native") is True
    deepseek_model = isinstance(model, str) and model.startswith("deepseek")
    deepseek_provider = provider == "deepseek"
    return record_metrics and (deepseek_native or deepseek_model or deepseek_provider)


def state_cache_metrics(session_dir: Path) -> dict[str, Any]:
    hit_tokens = 0
    miss_tokens = 0
    event_count = 0
    expected_count = 0
    for event in load_events(session_dir / "state" / "events.jsonl"):
        kind = event_kind(event)
        payload = event_payload(event)
        if kind == "RunStarted" and cache_metrics_expected(payload):
            expected_count += 1
        if kind != "CacheMetricsRecorded":
            continue
        hit = payload_int(payload, "prompt_cache_hit_tokens", "cache_hit_tokens")
        miss = payload_int(payload, "prompt_cache_miss_tokens", "cache_miss_tokens")
        if hit is None and miss is None:
            continue
        hit_tokens += hit or 0
        miss_tokens += miss or 0
        event_count += 1
    if event_count == 0 and expected_count == 0:
        return {}
    total = hit_tokens + miss_tokens
    metrics: dict[str, Any] = {
        "deepseek_cache_metric_source": "state",
        "deepseek_cache_metric_event_count": event_count,
        "deepseek_cache_metric_expected_count": expected_count,
        "deepseek_cache_metric_missing_count": max(expected_count - event_count, 0),
    }
    if event_count:
        metrics.update(
            {
                "deepseek_cache_hit_ratio": round(hit_tokens / total, 6) if total > 0 else None,
                "deepseek_cache_hit_tokens": hit_tokens,
                "deepseek_cache_miss_tokens": miss_tokens,
            }
        )
    return metrics


def task_artifact_metrics(session_dir: Path, attempted: int) -> dict[str, Any]:
    manifest = load_json(session_dir / "tasks" / "manifest.json")
    planner = manifest.get("planner") if isinstance(manifest.get("planner"), dict) else {}
    tasks = manifest.get("selected_tasks") if isinstance(manifest.get("selected_tasks"), list) else []
    task_dirs = [
        path
        for path in sorted((session_dir / "tasks").glob("task_*"))
        if path.is_dir()
    ]
    quality_scores: list[float] = []
    for task in tasks:
        if not isinstance(task, dict):
            continue
        quality = task.get("quality") if isinstance(task.get("quality"), dict) else {}
        score = quality.get("score")
        if isinstance(score, (int, float)):
            quality_scores.append(float(score))
    planned_count = int(planner.get("task_count") or len(tasks) or len(task_dirs) or 0)
    selected_count = int(planner.get("selected_task_count") or len(tasks) or len(task_dirs) or 0)
    planning_failed = bool(planner.get("planning_failed"))
    artifact_denominator = selected_count or attempted
    if planning_failed:
        artifact_coverage = 0.0
    elif artifact_denominator:
        artifact_coverage = min(1.0, ratio(len(task_dirs), artifact_denominator))
    else:
        artifact_coverage = None if not task_dirs else 0.0
    task_unattempted = max(selected_count - attempted, 0) if selected_count else 0
    strict_verified = 0
    evaluator_unverified = 0
    evaluator_timeout_with_verdict = 0
    unlanded_source = 0
    for task_dir in task_dirs:
        outcome = load_json(task_dir / "outcome.json")
        evals = [load_json(path) for path in sorted(task_dir.glob("eval_attempt_*.json"))]
        evals = [row for row in evals if row]
        has_pass = any(clean_eval_pass(row) for row in evals)
        has_timeout_with_verdict = any(eval_timed_out_after_verdict(row) for row in evals)
        touched = outcome.get("source_files") or outcome.get("touched_files") or []
        landed = bool(outcome.get("commit_shas") or outcome.get("commits"))
        has_landed_source = not touched or landed
        if outcome.get("status") == "completed" and has_pass and has_landed_source:
            strict_verified += 1
        elif outcome.get("status") == "completed" or evals:
            evaluator_unverified += 1
        if has_timeout_with_verdict:
            evaluator_timeout_with_verdict += 1
        if touched and not landed:
            unlanded_source += 1
    verification_denominator = selected_count or len(task_dirs)
    if verification_denominator:
        evaluator_unverified = max(evaluator_unverified, verification_denominator - strict_verified)
    replay = replay_check_session(session_dir)
    return {
        "task_manifest_available": bool(manifest),
        "planner_no_task_count": 1 if planning_failed else 0,
        "planned_task_count": planned_count,
        "selected_task_count": selected_count,
        "task_unattempted_count": task_unattempted,
        "task_artifact_count": len(task_dirs),
        "task_artifact_coverage": artifact_coverage,
        "task_strict_verified_count": strict_verified,
        "evaluator_unverified_count": evaluator_unverified,
        "evaluator_timeout_with_verdict_count": evaluator_timeout_with_verdict,
        "task_unlanded_source_count": unlanded_source,
        "task_spec_quality_score": round(sum(quality_scores) / len(quality_scores), 4)
        if quality_scores
        else None,
        "state_replay_integrity_rate": 1.0 if replay.get("ok") else 0.0,
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
        status = str(data.get("status") or "")
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
        if kind in {"RunStarted", "TaskLineageLinked"} and status == "started":
            row["started_event_id"] = event.get("event_id")
            for key in ("planned_files", "issue", "base_commit"):
                if data.get(key) is not None:
                    row[key] = data.get(key)
        elif kind in {"RunCompleted", "TaskLineageLinked"} and status != "started":
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
            + float(metrics.get("planner_no_task_count") or 0) * 3.0
            + float(metrics.get("task_unattempted_count") or 0) * 2.0
            + float(metrics.get("evaluator_unverified_count") or 0)
            + float(metrics.get("evaluator_timeout_with_verdict_count") or 0) * 2.0
            + float(metrics.get("task_unlanded_source_count") or 0) * 2.0
            + float(metrics.get("state_live_baseline_shrink_count") or 0) * 2.0
        )
        / 12.0,
    )
    state_capture = metrics.get("state_operational_capture_coverage")
    if not isinstance(state_capture, (int, float)) or isinstance(state_capture, bool):
        state_capture = metrics.get("state_capture_coverage")
    capture = (float(state_capture or 0.0) + float(metrics.get("audit_capture_coverage") or 0.0)) / 2.0
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
    state_metrics = state_trace_metrics(session_dir)
    pipeline_metrics = state_pipeline_metrics(session_dir)
    audit_exists = (session_dir / "audit.jsonl").is_file()
    parsed = parse_log(log_text) if log_available else parse_log("")
    turn_metrics = task_turn_metrics(session_dir)
    cache_metrics = state_cache_metrics(session_dir)
    previous = previous_feedback(session_dir)
    recurrences = recurrence_metrics(parsed["failure_fingerprints"], previous)

    attempted = int(outcome.get("tasks_attempted") or 0)
    succeeded = int(outcome.get("tasks_succeeded") or 0)
    artifact_metrics = task_artifact_metrics(session_dir, attempted)
    artifact_metrics["planner_no_task_count"] = max(
        int(parsed.get("planner_no_task_count") or 0),
        int(artifact_metrics.get("planner_no_task_count") or 0),
    )
    artifact_metrics["evaluator_unverified_count"] = max(
        int(parsed.get("evaluator_unverified_count") or 0),
        int(artifact_metrics.get("evaluator_unverified_count") or 0),
    )
    strict_succeeded = int(artifact_metrics.get("task_strict_verified_count") or 0)
    has_task_evidence = bool(
        artifact_metrics.get("task_manifest_available")
        or int(artifact_metrics.get("task_artifact_count") or 0) > 0
    )
    counted_succeeded = strict_succeeded if attempted and has_task_evidence else succeeded
    task_success_rate = ratio(counted_succeeded, attempted)
    workflow_success = workflow_conclusion.lower() in {"success", "passed"}
    build_ok = bool(outcome.get("build_ok"))
    test_ok = bool(outcome.get("test_ok"))
    reverted = bool(outcome.get("reverted"))
    session_success = bool(
        build_ok
        and test_ok
        and not reverted
        and int(artifact_metrics.get("planner_no_task_count") or 0) == 0
        and (attempted == 0 or counted_succeeded >= attempted)
    )
    retry_success_rate = None
    if parsed["retry_markers"]:
        retry_success_rate = 1.0 if workflow_success else 0.0

    confidence = 0.0
    if log_available:
        confidence += 0.45
    if outcome:
        confidence += 0.20
    if int(state_metrics.get("state_event_count") or 0) > 0:
        confidence += 0.20
    if audit_exists:
        confidence += 0.15

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
        "tasks_succeeded": counted_succeeded,
        "raw_tasks_succeeded": succeeded,
        "retry_success_rate": retry_success_rate,
        "audit_capture_coverage": audit_capture_coverage,
        **state_metrics,
        **pipeline_metrics,
        **parsed,
        **artifact_metrics,
        **turn_metrics,
        **cache_metrics,
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
    if int(metrics.get("planner_no_task_count") or 0) > 0:
        lessons.append(
            {
                "kind": "planner_no_tasks",
                "fingerprint": "planning agent produced no concrete task files",
                "action": "tighten task schema adherence and preserve planner failure evidence instead of running generic fallback work",
            }
        )
    if int(metrics.get("task_unattempted_count") or 0) > 0:
        lessons.append(
            {
                "kind": "task_unattempted",
                "fingerprint": "planner selected tasks that implementation phase never started",
                "action": "reduce selected task count or preserve enough implementation budget to attempt every selected task",
            }
        )
    if int(metrics.get("state_live_baseline_shrink_count") or 0) > 0:
        lessons.append(
            {
                "kind": "state_baseline_shrink",
                "fingerprint": "live state log had fewer events than the replay baseline",
                "action": "inspect concurrent state writers and keep live state append-only before merging session evidence",
            }
        )
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
    if int(metrics.get("evaluator_timeout_with_verdict_count") or 0) > 0:
        lessons.append(
            {
                "kind": "evaluator_timeout_with_verdict",
                "fingerprint": "evaluator wrote a verdict but the process still timed out",
                "action": "make evaluator agents exit immediately after writing verdicts or stop the wrapper once the verdict file exists",
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
                "evolve\tRun evolution session\t2026-06-09T11:52:59.0607692Z Let me look at the test failure. The test at line 6536 panicked, and then at line 6541. Let me read what's there.",
                "evolve\tRun evolution session\t2026-06-09T11:53:09.4054420Z Actually, the output says \"2 test_assertion\" errors at lines 6536 and 6541.",
                "evolve\tRun evolution session\t2026-06-09T11:53:14.8839934Z Wait, the error says the thread panicked at line 6536 and line 6541.",
                "evolve\tRun evolution session\t2026-06-11T04:40:00Z > \"The diagnostic mechanism records failures for later review\"",
                "evolve\tRun evolution session\t2026-06-11T04:40:01Z But wait - looking at the recent events, the prior command failed with exit code 1.",
                "evolve\tRun evolution session\t2026-06-11T04:40:02Z These are quoted evaluator notes about failure evidence, not current command output.",
                "evolve\tRun evolution session\t2026-06-11T08:23:53Z **Edit 1: Spawn failure (line 484-486)** — Wrap the `map_err` closure to stash before constructing the error:",
                "evolve\tRun evolution session\t2026-06-11T08:23:59Z **Edit 2: Timeout (lines ~600-606)** — Stash before returning timeout error:",
                "evolve\tRun evolution session\t2026-06-11T08:23:59Z                         \"Command timed out after {}s\",",
                "evolve\tRun evolution session\t2026-06-11T09:49:37Z Interesting! The `state crashes` command shows that this assessment session had 10 crash attempts before it successfully started.",
                "evolve\tRun evolution session\t2026-06-11T09:52:38Z The crashes have exit code 1 or 2, and no diagnostic - meaning they happened in paths not covered by `stash_diagnostic_error`.",
                "evolve\tRun evolution session\t2026-06-11T09:53:43Z The key issue is that these failure paths exit without telling us WHY.",
                "evolve\tRun evolution session\t2026-06-11T09:54:16Z The most common path: API call fails → `eprintln!(\"error: {e}\")` → `exit_with_state(1)` without stashing the error.",
                "evolve\tRun evolution session\t2026-06-11T09:53:40Z There are many `exit_with_state` calls without prior `stash_diagnostic_error`. The crashes with exit code 1 or 2 could come from any of these.",
                "evolve\tRun evolution session\t2026-06-11T10:26:21Z There's one more error: `build_state_memory_candidates` is used at line 14305 in `commands_state.rs`, but it's private.",
                "evolve\tRun evolution session\t2026-06-11T09:56:16Z                     eprintln!(\"{RED}  error: {e}{RESET}\");",
                "evolve\tRun evolution session\t2026-06-11T10:32:36Z The grep showed 0 for commands_state.rs and the second grep returned exit code 1 which happens with `grep -c` returning 0.",
                "evolve\tRun evolution session\t2026-06-11T12:27:42Z The test timed out. Let me try a narrower test or check the specific failing tests from the trajectory.",
                "evolve\tRun evolution session\t2026-06-11T13:01:09Z The binary timed out on a simple prompt, likely because no real API key is available. Let me just note that.",
            ]
        )
    )
    check("benign action log lines are not failures", noisy["distinct_failure_count"] == 0, noisy["failure_fingerprints"])
    check("benign action log lines do not count command timeouts", noisy["command_timeout_count"] == 0, noisy)
    check("timestamps and retry counts are not provider errors", noisy["provider_error_count"] == 0, noisy["provider_error_count"])
    real_panic = parse_log("evolve\tRun evolution session\t2026-06-09T11:52:59Z thread 'main' panicked at src/state.rs:42:9:")
    check("real rust panic lines are still failures", real_panic["distinct_failure_count"] == 1, real_panic)
    real_compile_error = parse_log("evolve\tRun evolution session\t2026-06-11T10:17:45Z       error[E0277]: the size for values of type `str` cannot be known at compilation time")
    check("real rust compiler errors are still failures", real_compile_error["distinct_failure_count"] == 1, real_compile_error)
    operational = parse_log(
        "\n".join(
            [
                "evolve\tRun evolution session\t2026-06-07T04:26:23Z     │ Command timed out after 60s",
                "evolve\tRun evolution session\t2026-06-07T04:50:22Z ^G    BLOCKED: Task 2 modified protected files: .github/workflows/ci.yml",
                "evolve\tRun evolution session\t2026-06-07T04:50:22Z     Reverting Task 2 (resetting to 041da74)",
                "evolve\tRun evolution session\t2026-06-07T04:24:55Z     │ Search error: grep: ./target/debug/deps/yyds: binary file matches",
                "evolve\tRun evolution session\t2026-06-07T04:24:58Z   Planning agent produced 0 tasks — recording planning failure; no fake task will run.",
                "evolve\tRun evolution session\t2026-06-07T04:24:22Z   → Task 1: First real eval run",
                "evolve\tRun evolution session\t2026-06-07T05:05:46Z    Evaluator: timed out — skipping eval (build+test passed)",
                "evolve\tRun evolution session\t2026-06-07T04:33:47Z     Task 1: verified OK",
                "evolve\tRun evolution session\t2026-06-07T04:09:25Z - Cache: 84.38% hit ratio, 572,800 hit tokens, 106,004 miss tokens",
                "evolve\tRun evolution session\t2026-06-07T04:50:23Z     Build-fix agent modified protected files — reverting",
                "evolve\tRun evolution session\t2026-06-07T04:50:24Z     Fix agent modified protected files - reverting",
            ]
        )
    )
    check("command timeouts counted", operational["command_timeout_count"] == 1, operational)
    check("evaluator timeouts counted", operational["evaluator_timeout_count"] == 1, operational)
    check("protected file reverts counted", operational["protected_file_revert_count"] == 3, operational)
    check("task reverts counted", operational["task_revert_count"] == 1, operational)
    check("search errors counted", operational["search_error_count"] == 1, operational)
    check("planner no-task counted", operational["planner_no_task_count"] == 1, operational)
    check("evaluator unverified counted", operational["evaluator_unverified_count"] == 1, operational)
    check("mechanical verification counted", operational["task_mechanical_verified_count"] == 1, operational)
    check("evaluator timeout blocks verification rate", operational["task_verification_rate"] == 0.0, operational)
    check("cache hit tokens parsed", operational["deepseek_cache_hit_tokens"] == 572800, operational)
    check("cache miss tokens parsed", operational["deepseek_cache_miss_tokens"] == 106004, operational)
    check(
        "cache hit ratio parsed from tokens",
        abs(float(operational["deepseek_cache_hit_ratio"]) - 0.843842) < 0.00001,
        operational,
    )
    yyds_usage = parse_log(
        "evolve\tRun evolution session\t2026-06-09T12:00:00Z "
        "tokens: 106004 in / 2000 out  [cache: 572800 read, 0 write]  "
        "(session: 106004 in / 2000 out)"
    )
    check(
        "yyds usage cache hit tokens parsed",
        yyds_usage["deepseek_cache_hit_tokens"] == 572800,
        yyds_usage,
    )
    check(
        "yyds usage cache miss tokens parsed",
        yyds_usage["deepseek_cache_miss_tokens"] == 106004,
        yyds_usage,
    )
    check(
        "yyds usage cache hit ratio parsed",
        abs(float(yyds_usage["deepseek_cache_hit_ratio"]) - 0.843842) < 0.00001,
        yyds_usage,
    )
    deepseek_usage = parse_log(
        "usage={\"prompt_cache_hit_tokens\": 572800, \"prompt_cache_miss_tokens\": 106004}"
    )
    check(
        "deepseek prompt cache hit tokens parsed",
        deepseek_usage["deepseek_cache_hit_tokens"] == 572800,
        deepseek_usage,
    )
    check(
        "deepseek prompt cache miss tokens parsed",
        deepseek_usage["deepseek_cache_miss_tokens"] == 106004,
        deepseek_usage,
    )
    cache_prose = parse_log("DeepSeek cache: 91% hit ratio - very healthy")
    check("cache prose ratio is not treated as KPI", cache_prose["deepseek_cache_hit_ratio"] is None, cache_prose)
    check("cache prose mention counted", cache_prose["deepseek_cache_prose_mention_count"] == 1, cache_prose)
    check("cache prose ratio marked unverified", cache_prose["deepseek_cache_ratio_unverified_count"] == 1, cache_prose)
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

        task_dir = session / "tasks" / "task_01"
        task_dir.mkdir(parents=True)
        (session / "tasks" / "manifest.json").write_text(
            json.dumps({"planner": {"task_count": 1, "selected_task_count": 1}, "selected_tasks": [{}]}),
            encoding="utf-8",
        )
        (task_dir / "outcome.json").write_text(
            json.dumps(
                {
                    "task_id": "task_01",
                    "status": "completed",
                    "source_files": ["src/state.rs"],
                    "commit_shas": ["abc123"],
                }
            ),
            encoding="utf-8",
        )
        (task_dir / "eval_attempt_1.json").write_text(
            json.dumps(
                {
                    "task_id": "task_01",
                    "status": "pass",
                    "exit_code": 124,
                    "verdict": "Verdict: PASS",
                    "verdict_file": "eval_attempt_1.md",
                }
            ),
            encoding="utf-8",
        )
        artifacts = task_artifact_metrics(session, attempted=1)
        check("timed-out verdict counted", artifacts["evaluator_timeout_with_verdict_count"] == 1, artifacts)
        check("timed-out verdict is not strict verified", artifacts["task_strict_verified_count"] == 0, artifacts)
        check("timed-out verdict is evaluator-unverified", artifacts["evaluator_unverified_count"] == 1, artifacts)

        (task_dir / "eval_attempt_1.json").unlink()
        (task_dir / "outcome.json").write_text(
            json.dumps(
                {
                    "task_id": "task_01",
                    "status": "reverted",
                    "planned_files": ["src/state.rs"],
                    "source_files": [],
                    "commit_shas": [],
                    "revert_reason": "Task scope mismatch: task produced no git-visible file changes",
                }
            ),
            encoding="utf-8",
        )
        reverted_artifacts = task_artifact_metrics(session, attempted=1)
        check(
            "reverted selected task without eval is evaluator-unverified",
            reverted_artifacts["evaluator_unverified_count"] == 1,
            reverted_artifacts,
        )

        (session / "state").mkdir(exist_ok=True)
        (session / "state" / "merge_state_delta.json").write_text(
            json.dumps({"baseline_shrunk": 1}),
            encoding="utf-8",
        )
        pipeline = state_pipeline_metrics(session)
        check("state baseline shrink counted", pipeline["state_live_baseline_shrink_count"] == 1, pipeline)
        shrink_lessons = [lesson["kind"] for lesson in top_lessons(pipeline)]
        check("state baseline shrink lesson emitted", "state_baseline_shrink" in shrink_lessons, shrink_lessons)
        (session / "state" / "merge_state_delta.json").write_text(
            json.dumps({"baseline_shrunk": 0, "baseline_reset": 1}),
            encoding="utf-8",
        )
        reset_pipeline = state_pipeline_metrics(session)
        reset_lessons = [lesson["kind"] for lesson in top_lessons(reset_pipeline)]
        check("state baseline reset not counted as shrink", reset_pipeline["state_live_baseline_shrink_count"] == 0, reset_pipeline)
        check("state baseline reset emits no shrink lesson", "state_baseline_shrink" not in reset_lessons, reset_lessons)
        (session / "state" / "merge_state_delta.json").write_text(
            json.dumps(
                {
                    "baseline_shrunk": 1,
                    "base_lines": 1885,
                    "effective_base_lines": 0,
                    "live_events": 64,
                    "session_events_before": 4,
                    "added": 64,
                }
            ),
            encoding="utf-8",
        )
        legacy_reset_pipeline = state_pipeline_metrics(session)
        check(
            "legacy projection reset not counted as shrink",
            legacy_reset_pipeline["state_live_baseline_shrink_count"] == 0,
            legacy_reset_pipeline,
        )

        planning_session = root / "planning-session"
        (planning_session / "tasks").mkdir(parents=True)
        (planning_session / "tasks" / "manifest.json").write_text(
            json.dumps({"planner": {"planning_failed": True, "task_count": 0, "selected_task_count": 0}}),
            encoding="utf-8",
        )
        planning_artifacts = task_artifact_metrics(planning_session, attempted=0)
        check("planning failure artifact coverage is zero", planning_artifacts["task_artifact_coverage"] == 0.0, planning_artifacts)
        check("planning failure count comes from manifest", planning_artifacts["planner_no_task_count"] == 1, planning_artifacts)

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
