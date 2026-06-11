#!/usr/bin/env python3
"""Build the static harness evolution dashboard from audit-log summaries."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from state_graph_tools import build_causal_chains, evolution_suggestions
from task_manifest import parse_task as parse_task_file


REPO_URL = "https://github.com/yologdev/yyds-harness"
TRANSCRIPT_ACTION_RE = re.compile(r"▶\s+([^▶\n]+)")
TRANSCRIPT_STATUS_RE = re.compile(r"\s+[✓✗]\s*\([^)]*\)\s*$")
TURN_MARKER_RE = re.compile(r"╭─\s*Turn\s+(\d+)\s*─")
WATCH_RE = re.compile(r"([✓✗])\s+Watch\s+(?:passed|failed):\s+`([^`]+)`")
WORKSPACE_PREFIX_RE = re.compile(r"/home/runner/work/yyds-harness/yyds-harness/?")
PSEUDO_ROOT_NAMES = {
    "docs",
    "eval",
    "journals",
    "memory",
    "scripts",
    "session_plan",
    "site",
    "skills",
    "src",
    "tasks",
    "tests",
}


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def log_feedback_metrics(session_dir: Path) -> dict[str, Any]:
    feedback = load_json(session_dir / "log_feedback.json")
    metrics = feedback.get("metrics") if isinstance(feedback.get("metrics"), dict) else {}
    return {
        str(key): value
        for key, value in metrics.items()
        if value is None or isinstance(value, (bool, int, float))
    }


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    if not path.is_file():
        return events
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return events
    for line in lines:
        text = line.strip()
        if not text:
            continue
        try:
            value = json.loads(text)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            events.append(value)
    return events


def event_kind(event: dict[str, Any]) -> str:
    value = event.get("event_type") or event.get("kind")
    if isinstance(value, str):
        return value
    raw_payload = event.get("payload")
    if isinstance(raw_payload, dict):
        meta = raw_payload.get("_yoyo")
        if isinstance(meta, dict) and isinstance(meta.get("event_type"), str):
            return str(meta["event_type"])
    return ""


def event_payload(event: dict[str, Any]) -> dict[str, Any]:
    value = event.get("payload")
    if not isinstance(value, dict):
        return {}
    wrapped = value.get("value")
    if set(value.keys()).issubset({"_yoyo", "value"}) and isinstance(wrapped, dict):
        return wrapped
    return value


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


def compact_list(values: list[str], limit: int) -> list[str]:
    out: list[str] = []
    for value in values:
        text = " ".join(str(value).split())
        if text and text not in out:
            out.append(text)
        if len(out) >= limit:
            break
    return out


def evidence_text(value: Any, max_len: int = 220) -> str:
    text = " ".join(str(value or "").split())
    text = re.sub(r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]+", r"\1[REDACTED]", text)
    text = re.sub(r"\bsk-[A-Za-z0-9_-]{4,}\b", "sk-[REDACTED]", text)
    if len(text) > max_len:
        return text[: max_len - 1].rstrip() + "…"
    return text


def clean_transcript_action(value: str) -> str:
    text = TRANSCRIPT_STATUS_RE.sub("", evidence_text(value, 500)).strip()
    text = WORKSPACE_PREFIX_RE.sub(".", text)
    for root in PSEUDO_ROOT_NAMES:
        text = text.replace(f".{root}/", f"{root}/")
        text = re.sub(rf"(?<!\S)\.{re.escape(root)}(?=$|\s)", root, text)
    return re.sub(r"\bcd\s+\.\s*&&\s*", "", text).strip()


def transcript_action_failed(lines: list[str], index: int, raw: str) -> bool:
    if "✗" in raw:
        return True
    for next_line in lines[index + 1 : index + 12]:
        stripped = next_line.strip()
        if not stripped:
            continue
        if stripped.startswith("▶") or stripped.startswith("╭─") or " Watch " in stripped:
            break
        if re.search(r"✗\s*(?:\([^)]*\))?$", stripped):
            return True
    return False


def transcript_failure_detail(lines: list[str], index: int, raw: str = "") -> str:
    detail_start = index + 1 if "✗" in raw else None
    if detail_start is None:
        for offset, next_line in enumerate(lines[index + 1 : index + 12], start=index + 1):
            stripped = next_line.strip()
            if not stripped:
                continue
            if stripped.startswith("▶") or stripped.startswith("╭─") or " Watch " in stripped:
                break
            if re.search(r"✗\s*(?:\([^)]*\))?$", stripped):
                detail_start = offset + 1
                break
    if detail_start is None:
        detail_start = index + 1
    details: list[str] = []
    for next_line in lines[detail_start : detail_start + 5]:
        stripped = next_line.strip()
        if not stripped:
            continue
        if stripped.startswith("▶") or stripped.startswith("╭─") or " Watch " in stripped:
            break
        cleaned = re.sub(r"^[│|]\s*", "", stripped).strip()
        cleaned = re.sub(r"^[-=]+\s*", "", cleaned).strip()
        cleaned = clean_transcript_action(cleaned)
        if cleaned:
            details.append(cleaned)
        if details:
            break
    return details[0] if details else ""


def transcript_path_token(value: str) -> str:
    text = str(value).strip().strip("`'\"")
    text = text.split()[0] if text.split() else text
    text = text.split(":", 1)[0]
    text = WORKSPACE_PREFIX_RE.sub("", text).strip()
    text = normalize_transcript_path(text)
    return "" if text in {"?", "-", "."} else text


def transcript_tool_label(action: str) -> str:
    for prefix in ("read ", "edit ", "write "):
        if action.startswith(prefix):
            target = transcript_path_token(action.removeprefix(prefix))
            return f"{prefix.strip()} {target}" if target else action
    return action


def exit_code_failure(value: Any) -> bool:
    match = re.search(r"\bExit code:\s*(-?\d+)\b", str(value or ""))
    if not match:
        return False
    try:
        return int(match.group(1)) != 0
    except ValueError:
        return False


def tool_context(tool: str, args: Any) -> str:
    if not isinstance(args, dict):
        return tool
    if tool == "bash":
        command = args.get("command")
        if isinstance(command, str) and command.strip():
            return f"bash {clean_transcript_action(command)}"
        description = args.get("description")
        if isinstance(description, str) and description.strip():
            return f"bash description: {clean_transcript_action(description)}"
        return "bash"
    if tool == "search":
        pattern = args.get("pattern")
        path = args.get("path")
        if isinstance(pattern, str) and isinstance(path, str):
            return f"search {pattern!r} in {normalize_transcript_path(path)}"
    for key in ("path", "file", "target_path"):
        value = args.get(key)
        if isinstance(value, str) and value.strip():
            return f"{tool} {normalize_transcript_path(value)}"
    return tool


def tool_failure_label(tool: str, args: Any, data: dict[str, Any]) -> str:
    context = tool_context(tool, args)
    message = data.get("error") or data.get("message") or data.get("result_preview")
    message_text = evidence_text(message)
    if message_text:
        return f"{context}: {message_text}"
    return context


def normalize_transcript_path(path: str) -> str:
    text = path.strip()
    if text.startswith("..") and not text.startswith("../"):
        text = text[1:]
    if text.startswith("./"):
        text = text[2:]
    pseudo_roots = (
        "Cargo.",
        "README",
        "docs/",
        "eval/",
        "journals/",
        "memory/",
        "scripts/",
        "session_plan/",
        "site/",
        "skills/",
        "src/",
        "tasks/",
        "tests/",
    )
    if text.startswith(".") and any(text[1:].startswith(root) for root in pseudo_roots):
        return text[1:]
    if text.startswith(".") and text[1:] in PSEUDO_ROOT_NAMES:
        return text[1:]
    return text


def normalize_evidence_path(path: str) -> str:
    return normalize_transcript_path(clean_transcript_action(path))


def summarize_transcript_actions(session_dir: Path) -> dict[str, Any]:
    transcript_dir = session_dir / "transcripts"
    files = sorted(transcript_dir.glob("*.log")) if transcript_dir.is_dir() else []
    commands: list[str] = []
    failed_commands: list[str] = []
    failed_tools: list[str] = []
    read_files: list[str] = []
    edited_files: list[str] = []

    for path in files:
        try:
            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError:
            continue
        for index, line in enumerate(lines):
            for marker, command in WATCH_RE.findall(line):
                command_text = clean_transcript_action(command)
                commands.append(command_text)
                if marker == "✗":
                    failed_commands.append(command_text)
            for match in TRANSCRIPT_ACTION_RE.finditer(line):
                raw = match.group(1)
                action = clean_transcript_action(raw)
                if not action or action == "todo":
                    continue
                failed = transcript_action_failed(lines, index, raw)
                if failed and (
                    action.startswith("read ")
                    or action.startswith("edit ")
                    or action.startswith("write ")
                    or action.startswith("search ")
                ):
                    detail = transcript_failure_detail(lines, index, raw)
                    label = transcript_tool_label(action)
                    failed_tools.append(f"{label}: {detail}" if detail else label)
                if action.startswith("$ "):
                    command = action[2:].strip()
                    if command:
                        commands.append(command)
                        if failed:
                            failed_commands.append(command)
                    continue
                if action.startswith("read "):
                    target = transcript_path_token(action.removeprefix("read "))
                    if target:
                        read_files.append(target)
                    continue
                if action.startswith("edit "):
                    target = transcript_path_token(action.removeprefix("edit "))
                    if target:
                        edited_files.append(target)
                    continue
                if action.startswith("write "):
                    target = transcript_path_token(action.removeprefix("write "))
                    if target:
                        edited_files.append(target)
                    continue
                if action.startswith("search ") and " in " in action:
                    target = transcript_path_token(action.rsplit(" in ", 1)[1])
                    if target and target not in {"src", "src/"}:
                        read_files.append(target)
                if failed and action:
                    failed_commands.append(action)

    return {
        "commands": compact_list(commands, 12),
        "failed_commands": compact_list(failed_commands, 8),
        "failed_tools": compact_list(failed_tools, 8),
        "read_files": compact_list(read_files, 12),
        "edited_files": compact_list(edited_files, 12),
    }


def source_file(path: str) -> bool:
    if not path:
        return False
    non_source_prefixes = (
        ".yoyo/",
        "journals/",
        "memory/",
        "session_plan/",
        "sessions/",
        "site/",
    )
    if path.startswith(non_source_prefixes):
        return False
    return path not in {".skill_evolve_counter", "DAY_COUNT", "ISSUES_TODAY.md"}


def path_matches(planned: str, touched: str) -> bool:
    planned = str(planned).strip().strip("/")
    touched = str(touched).strip().strip("/")
    if not planned or not touched:
        return False
    return touched == planned or touched.startswith(f"{planned}/")


def file_overlap(planned: list[str], touched: list[str]) -> bool:
    return any(path_matches(planned_file, touched_file) for planned_file in planned for touched_file in touched)


def trace_quality(summary: dict[str, Any], evals: list[dict[str, Any]]) -> dict[str, Any]:
    counts = summary.get("event_counts") if isinstance(summary.get("event_counts"), dict) else {}
    total = int(summary.get("event_count") or 0)
    patch_evaluated = int(counts.get("PatchEvaluated") or 0)
    trace_events = max(0, total - patch_evaluated)
    operational_events = sum(
        int(counts.get(kind) or 0)
        for kind in (
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
        )
    )
    task_lineage_events = int(counts.get("TaskLineageLinked") or 0)
    feedback_evals = sum(
        1
        for eval_data in evals
        if isinstance(eval_data, dict) and eval_data.get("suite") == "log-feedback"
    )
    if total <= 0:
        status = "missing"
        label = "no state trace"
    elif trace_events <= 0 and (patch_evaluated or feedback_evals):
        status = "feedback_only"
        label = "feedback-only trace"
    elif operational_events <= 0 and task_lineage_events <= 0:
        status = "lifecycle"
        label = "lifecycle-only trace"
    elif operational_events <= 0 and task_lineage_events > 0:
        status = "thin"
        label = "task-lineage trace"
    elif operational_events < 2 or trace_events < 5:
        status = "thin"
        label = "thin state trace"
    else:
        status = "full"
        label = "full state trace"
    return {
        "status": status,
        "label": label,
        "event_count": total,
        "trace_event_count": trace_events,
        "operational_event_count": operational_events,
        "task_lineage_event_count": task_lineage_events,
        "patch_evaluated_count": patch_evaluated,
        "feedback_eval_count": feedback_evals,
        "state_capture_coverage": 1.0 if trace_events > 0 else 0.0,
        "operational_capture_coverage": 1.0 if operational_events > 0 else 0.0,
        "task_lineage_capture_coverage": 1.0 if task_lineage_events > 0 else 0.0,
    }


def session_commit_prefix(outcome: dict[str, Any]) -> str:
    day = outcome.get("day")
    session_time = outcome.get("session_time")
    if day is None or not session_time:
        return ""
    return f"Day {day} ({session_time}):"


def session_commits(outcome: dict[str, Any], repo_root: Path) -> list[dict[str, Any]]:
    prefix = session_commit_prefix(outcome)
    if not prefix:
        return []
    try:
        result = subprocess.run(
            [
                "git",
                "-C",
                str(repo_root),
                "log",
                "--all",
                "--fixed-strings",
                "--grep",
                prefix,
                "--format=%x1e%H%x00%s",
                "--name-only",
            ],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return []
    if result.returncode != 0:
        return []

    commits: list[dict[str, Any]] = []
    for raw_record in result.stdout.split("\x1e"):
        record = raw_record.strip()
        if not record:
            continue
        lines = [line for line in record.splitlines() if line.strip()]
        if not lines or "\x00" not in lines[0]:
            continue
        sha, subject = lines[0].split("\x00", 1)
        files = compact_list(lines[1:], 40)
        commits.append(
            {
                "sha": sha,
                "short_sha": sha[:7],
                "subject": subject,
                "files": files,
                "source_files": [path for path in files if source_file(path)],
            }
        )
    commits.reverse()
    return commits


def serialize_commits(commits: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        {
            "sha": commit.get("sha"),
            "short_sha": commit.get("short_sha"),
            "subject": commit.get("subject"),
            "files": commit.get("files") or [],
            "source_files": commit.get("source_files") or [],
        }
        for commit in commits
    ]


def transcript_summary(session_dir: Path) -> dict[str, Any]:
    transcript_dir = session_dir / "transcripts"
    files = sorted(transcript_dir.glob("*.log")) if transcript_dir.is_dir() else []
    phase_counts = {
        "assess": 0,
        "plan": 0,
        "task": 0,
        "fix": 0,
        "eval": 0,
        "other": 0,
    }
    transcript_rows: list[dict[str, str]] = []
    for path in files:
        name = path.name
        if name.startswith("assess"):
            phase = "assess"
        elif name.startswith("plan"):
            phase = "plan"
        elif name.startswith("task_"):
            phase = "task"
        elif name.startswith("fix_"):
            phase = "fix"
        elif name.startswith("eval_"):
            phase = "eval"
        else:
            phase = "other"
        phase_counts[phase] += 1
        try:
            raw = path.read_bytes()
            line_count = len(raw.splitlines())
            byte_count = len(raw)
        except OSError:
            line_count = 0
            byte_count = 0
        transcript_rows.append(
            {
                "name": name,
                "phase": phase,
                "path": f"transcripts/{name}",
                "line_count": line_count,
                "byte_count": byte_count,
            }
        )
    return {
        "count": len(files),
        "phase_counts": {key: value for key, value in phase_counts.items() if value},
        "files": transcript_rows[:16],
    }


def state_pipeline_summary(session_dir: Path) -> dict[str, Any]:
    replay = load_json(session_dir / "state_replay.json")
    merge = load_json(session_dir / "state" / "merge_state_delta.json")
    append_log = session_dir / "state" / "append_state_event.log"
    append_lines = 0
    append_problem_lines = 0
    if append_log.is_file():
        for line in append_log.read_text(encoding="utf-8", errors="replace").splitlines():
            if not line.strip():
                continue
            append_lines += 1
            lower = line.lower()
            if "failed" in lower or "warning" in lower:
                append_problem_lines += 1
    return {
        "replay_scope": "audit_history" if replay else None,
        "replay_files_read": replay.get("files_read") if isinstance(replay, dict) else None,
        "replay_events_written": replay.get("events_written") if isinstance(replay, dict) else None,
        "replay_duplicates_skipped": replay.get("duplicates_skipped") if isinstance(replay, dict) else None,
        "merge_scope": "live_delta" if merge else None,
        "merge_live_events": merge.get("live_events") if isinstance(merge, dict) else None,
        "merge_base_lines": merge.get("base_lines") if isinstance(merge, dict) else None,
        "merge_effective_base_lines": merge.get("effective_base_lines") if isinstance(merge, dict) else None,
        "merge_baseline_reset": bool(merge.get("baseline_reset")) if isinstance(merge, dict) else False,
        "merge_baseline_shrunk": bool(merge.get("baseline_shrunk")) if isinstance(merge, dict) else False,
        "merge_delta_events": merge.get("delta_events") if isinstance(merge, dict) else None,
        "merge_added_events": merge.get("added") if isinstance(merge, dict) else None,
        "merge_duplicates_skipped": merge.get("skipped_duplicate") if isinstance(merge, dict) else None,
        "session_events_after_merge": merge.get("session_events_after") if isinstance(merge, dict) else None,
        "append_log_lines": append_lines,
        "append_problem_lines": append_problem_lines,
    }


def file_stats(path: Path) -> dict[str, int]:
    try:
        raw = path.read_bytes()
    except OSError:
        return {"line_count": 0, "byte_count": 0}
    return {"line_count": len(raw.splitlines()), "byte_count": len(raw)}


def transcript_turn_count(text: str) -> int:
    turns: list[int] = []
    for match in TURN_MARKER_RE.finditer(text):
        try:
            turns.append(int(match.group(1)))
        except ValueError:
            continue
    if turns:
        return max(turns)
    action_count = len(TRANSCRIPT_ACTION_RE.findall(text))
    if action_count:
        return action_count
    return 1 if text.strip() else 0


def enrich_attempts_with_turns(session_dir: Path, attempts: list[dict[str, Any]]) -> list[dict[str, Any]]:
    enriched: list[dict[str, Any]] = []
    for attempt in attempts:
        if not isinstance(attempt, dict):
            continue
        row = dict(attempt)
        if not isinstance(row.get("turn_count"), int):
            transcript = row.get("transcript_path")
            if isinstance(transcript, str) and transcript:
                path = session_dir / transcript
                try:
                    text_data = path.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    text_data = ""
                row["turn_count"] = transcript_turn_count(text_data)
        enriched.append(row)
    return enriched


def task_artifact_summary(session_dir: Path) -> list[dict[str, Any]]:
    tasks_dir = session_dir / "tasks"
    if not tasks_dir.is_dir():
        return []
    rows: list[dict[str, Any]] = []
    for task_dir in sorted(path for path in tasks_dir.iterdir() if path.is_dir()):
        task_id = task_dir.name
        attempts = enrich_attempts_with_turns(session_dir, load_jsonl(task_dir / "attempts.jsonl"))
        evals: list[dict[str, Any]] = []
        for path in sorted(task_dir.glob("eval_attempt_*.json")):
            eval_data = load_json(path)
            if eval_data:
                evals.append(eval_data)
        artifact_paths: list[dict[str, Any]] = []
        for path in sorted(task_dir.iterdir()):
            if not path.is_file():
                continue
            rel = f"tasks/{task_id}/{path.name}"
            stats = file_stats(path)
            artifact_paths.append(
                {
                    "name": path.name,
                    "path": rel,
                    "line_count": stats["line_count"],
                    "byte_count": stats["byte_count"],
                }
            )
        task_file = task_dir / "task.md"
        outcome = load_json(task_dir / "outcome.json")
        rows.append(
            {
                "task_id": task_id,
                "has_task_file": task_file.is_file(),
                "has_outcome": bool(outcome),
                "task_title": outcome.get("task_title") if isinstance(outcome, dict) else None,
                "status": outcome.get("status") if isinstance(outcome, dict) else None,
                "revert_reason": outcome.get("revert_reason") if isinstance(outcome, dict) else None,
                "source_files": outcome.get("source_files") if isinstance(outcome.get("source_files"), list) else [],
                "touched_files": outcome.get("touched_files") if isinstance(outcome.get("touched_files"), list) else [],
                "commit_shas": outcome.get("commit_shas") if isinstance(outcome.get("commit_shas"), list) else [],
                "attempt_count": len(attempts),
                "attempts": attempts[:8],
                "max_turn_count": max(
                    [
                        int(attempt.get("turn_count"))
                        for attempt in attempts
                        if isinstance(attempt, dict) and isinstance(attempt.get("turn_count"), int)
                    ]
                    or [0]
                ),
                "eval_statuses": [
                    str(eval_data.get("status"))
                    for eval_data in evals
                    if isinstance(eval_data, dict) and eval_data.get("status")
                ],
                "evals": evals[:8],
                "artifacts": artifact_paths,
                "task_line_count": file_stats(task_file)["line_count"] if task_file.is_file() else 0,
            }
        )
    return rows


def explicit_pass(value: Any) -> bool:
    text = str(value or "").strip().lower()
    return text in {"pass", "passed", "ok", "success"} or text.startswith("pass:")


def explicit_fail(value: Any) -> bool:
    text = str(value or "").strip().lower()
    return text in {"fail", "failed", "failure"} or text.startswith("fail:")


def eval_timed_out_after_verdict(eval_data: dict[str, Any]) -> bool:
    if int(eval_data.get("exit_code") or 0) != 124:
        return False
    return (
        explicit_pass(eval_data.get("status"))
        or explicit_pass(eval_data.get("verdict"))
        or explicit_fail(eval_data.get("status"))
        or explicit_fail(eval_data.get("verdict"))
        or bool(eval_data.get("verdict_file"))
    )


def eval_passed(evals: list[dict[str, Any]], lineage_eval: Any) -> bool:
    if evals:
        return any(
            isinstance(eval_data, dict)
            and not eval_timed_out_after_verdict(eval_data)
            and (
                explicit_pass(eval_data.get("status"))
                or explicit_pass(eval_data.get("verdict"))
            )
            for eval_data in evals
        )
    if isinstance(lineage_eval, dict):
        if explicit_pass(lineage_eval.get("verdict")):
            return True
    return False


def eval_summary_from_artifacts(evals: list[dict[str, Any]]) -> dict[str, Any] | None:
    if not evals:
        return None
    latest = next((row for row in reversed(evals) if isinstance(row, dict)), None)
    if not latest:
        return None
    status = str(latest.get("status") or "").strip()
    verdict = str(latest.get("verdict") or "").strip()
    reason = str(latest.get("reason") or "").strip()
    exit_code = latest.get("exit_code")
    normalized = verdict.removeprefix("Verdict:").strip() if verdict else ""
    if not normalized:
        if explicit_pass(status):
            normalized = "PASS"
        elif explicit_fail(status):
            normalized = "FAIL"
        elif status:
            normalized = status.upper()
        elif exit_code is not None:
            normalized = f"EXIT_{exit_code}"
    if not reason:
        if status == "timeout":
            reason = "Evaluator timed out before producing a passing verifier verdict."
        elif status in {"no_verdict", "api_error", "unrecognized"}:
            reason = "Evaluator did not produce a trusted passing verifier verdict."
    summary: dict[str, Any] = {
        "verdict": normalized or None,
        "status": status or None,
        "reason": reason or None,
        "transcript_path": latest.get("transcript_path"),
    }
    return {key: value for key, value in summary.items() if value is not None}


def enrich_task_lineage_with_artifacts(
    task_lineage: list[dict[str, Any]],
    task_artifacts: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    artifacts_by_id = {
        str(row.get("task_id")): row
        for row in task_artifacts
        if isinstance(row, dict) and row.get("task_id")
    }
    enriched: list[dict[str, Any]] = []
    for row in task_lineage:
        if not isinstance(row, dict):
            continue
        next_row = dict(row)
        artifact = artifacts_by_id.get(str(next_row.get("task_id") or ""))
        if artifact:
            if not next_row.get("eval"):
                eval_summary = eval_summary_from_artifacts(artifact.get("evals") or [])
                if eval_summary:
                    next_row["eval"] = eval_summary
            for field in ("status", "revert_reason", "source_files", "touched_files", "commit_shas"):
                if not next_row.get(field) and artifact.get(field):
                    next_row[field] = artifact.get(field)
        enriched.append(next_row)
    return enriched


def task_verification_summary(
    task_manifest: dict[str, Any],
    task_artifacts: list[dict[str, Any]],
    task_lineage: list[dict[str, Any]],
) -> dict[str, Any]:
    selected = task_manifest.get("tasks") if isinstance(task_manifest.get("tasks"), list) else []
    artifacts_by_id = {
        str(row.get("task_id")): row
        for row in task_artifacts
        if isinstance(row, dict) and row.get("task_id")
    }
    lineage_by_id = {
        str(row.get("task_id")): row
        for row in task_lineage
        if isinstance(row, dict) and row.get("task_id")
    }
    if not selected:
        task_ids = list(dict.fromkeys(list(artifacts_by_id) + list(lineage_by_id)))
        selected = [
            {
                "task_id": task_id,
                "title": (
                    artifacts_by_id.get(task_id, {}).get("task_title")
                    or lineage_by_id.get(task_id, {}).get("task_title")
                ),
                "files": (
                    artifacts_by_id.get(task_id, {}).get("planned_files")
                    or lineage_by_id.get(task_id, {}).get("planned_files")
                    or []
                ),
            }
            for task_id in task_ids
        ]
    rows: list[dict[str, Any]] = []
    for task in selected:
        if not isinstance(task, dict):
            continue
        task_id = str(task.get("task_id") or "")
        if not task_id:
            continue
        artifact = artifacts_by_id.get(task_id, {})
        lineage = lineage_by_id.get(task_id, {})
        planned = [str(path) for path in (task.get("files") or lineage.get("planned_files") or []) if path]
        touched = [
            str(path)
            for path in (
                lineage.get("source_files")
                or artifact.get("source_files")
                or lineage.get("touched_files")
                or artifact.get("touched_files")
                or []
            )
            if path
        ]
        landed_commits = [
            str(sha)
            for sha in (
                lineage.get("commit_shas")
                or lineage.get("linked_commit_shas")
                or artifact.get("commit_shas")
                or []
            )
            if sha
        ]
        overlap = file_overlap(planned, touched) if planned and touched else False
        artifact_evals = artifact.get("evals") or []
        timeout_with_verdict = any(
            isinstance(eval_data, dict) and eval_timed_out_after_verdict(eval_data)
            for eval_data in artifact_evals
        )
        verified = eval_passed(artifact_evals, lineage.get("eval"))
        outcome_status = str(artifact.get("status") or lineage.get("status") or "")
        revert_reason = str(artifact.get("revert_reason") or lineage.get("revert_reason") or "").strip()
        problems: list[str] = []
        if not planned:
            problems.append("missing_planned_files")
        if not touched:
            problems.append("no_touched_files")
        if planned and touched and not overlap:
            problems.append("no_planned_file_overlap")
        if timeout_with_verdict:
            problems.append("evaluator_timed_out_after_verdict")
        if not verified:
            problems.append("no_passing_verifier")
        if touched and not landed_commits:
            problems.append("source_edits_not_landed")
        if touched and outcome_status == "completed" and not landed_commits:
            problems.append("no_landed_source_commit")
        rows.append(
            {
                "task_id": task_id,
                "title": task.get("title") or lineage.get("task_title"),
                "planned_files": planned,
                "touched_files": touched,
                "overlap": overlap,
                "verified": verified,
                "outcome_status": outcome_status or None,
                "revert_reason": revert_reason or None,
                "landed_commit_shas": landed_commits,
                "eval_statuses": artifact.get("eval_statuses") or [],
                "problems": problems,
                "strict_success": outcome_status == "completed" and overlap and verified and not problems,
            }
        )
    total = len(rows)
    verified_count = sum(1 for row in rows if row["strict_success"])
    return {
        "task_count": total,
        "verified_task_count": verified_count,
        "unverified_task_count": total - verified_count,
        "all_verified": bool(total > 0 and verified_count == total),
        "rows": rows,
    }


def annotate_task_lineage_verification(
    task_lineage: list[dict[str, Any]],
    task_verification: dict[str, Any],
) -> list[dict[str, Any]]:
    rows_by_id = {
        str(row.get("task_id")): row
        for row in (task_verification.get("rows") or [])
        if isinstance(row, dict) and row.get("task_id")
    }
    annotated: list[dict[str, Any]] = []
    for task in task_lineage:
        if not isinstance(task, dict):
            continue
        next_task = dict(task)
        verification = rows_by_id.get(str(next_task.get("task_id") or ""))
        if verification:
            strict_success = bool(verification.get("strict_success"))
            problems = [
                str(problem)
                for problem in (verification.get("problems") or [])
                if problem
            ]
            next_task["strict_success"] = strict_success
            next_task["verification_status"] = "strict_pass" if strict_success else "strict_failed"
            next_task["verification_problems"] = problems
            next_task["verification_planned_files"] = verification.get("planned_files") or []
            next_task["verification_touched_files"] = verification.get("touched_files") or []
            next_task["landed_commit_shas"] = verification.get("landed_commit_shas") or []
            if verification.get("revert_reason") and not next_task.get("revert_reason"):
                next_task["revert_reason"] = verification.get("revert_reason")
        annotated.append(next_task)
    return annotated


def augment_evolution_suggestions(
    suggestions: list[dict[str, Any]],
    task_verification: dict[str, Any],
) -> list[dict[str, Any]]:
    rows = task_verification.get("rows") if isinstance(task_verification.get("rows"), list) else []
    unlanded = sum(
        1
        for row in rows
        if isinstance(row, dict)
        and (
            "source_edits_not_landed" in (row.get("problems") or [])
            or "no_landed_source_commit" in (row.get("problems") or [])
        )
    )
    timeout_with_verdict = sum(
        1
        for row in rows
        if isinstance(row, dict)
        and "evaluator_timed_out_after_verdict" in (row.get("problems") or [])
    )
    unattempted = sum(
        1
        for row in rows
        if isinstance(row, dict) and row.get("outcome_status") is None
    )
    unverified = int(task_verification.get("unverified_task_count") or 0)
    out = [dict(row) for row in suggestions if isinstance(row, dict)]

    def upsert(kind: str, title: str, reason: str, metric: str, value: Any, priority: int) -> None:
        for row in out:
            if row.get("metric") == metric:
                row["value"] = value
                row["priority"] = max(int(row.get("priority") or 0), priority)
                return
        out.append(
            {
                "kind": kind,
                "title": title,
                "reason": reason,
                "metric": metric,
                "value": value,
                "priority": priority,
            }
        )

    if unverified:
        upsert(
            "eval",
            "Bound evaluator checks so verdicts are not skipped",
            "Some tasks were not strictly verified by evaluator and commit evidence.",
            "evaluator_unverified_count",
            unverified,
            90,
        )
    if unattempted:
        upsert(
            "implementation",
            "Preserve budget to start every selected task",
            "The planner selected tasks that the implementation phase never attempted.",
            "task_unattempted_count",
            unattempted,
            94,
        )
    if timeout_with_verdict:
        upsert(
            "eval",
            "Stop evaluator once verdict evidence exists",
            "An evaluator wrote a verdict but still timed out, making verifier evidence ambiguous.",
            "evaluator_timeout_with_verdict_count",
            timeout_with_verdict,
            92,
        )
    if unlanded:
        upsert(
            "commit",
            "Make source-edit outcomes land or explain reverts",
            "A task touched source files without a landed source commit.",
            "task_unlanded_source_count",
            unlanded,
            88,
        )
    out.sort(key=lambda item: (-int(item.get("priority") or 0), str(item.get("title") or "")))
    return out


def task_manifest_summary(session_dir: Path) -> dict[str, Any]:
    manifest = load_json(session_dir / "tasks" / "manifest.json")
    if not manifest:
        return {}
    planner = manifest.get("planner") if isinstance(manifest.get("planner"), dict) else {}
    artifacts = manifest.get("artifacts") if isinstance(manifest.get("artifacts"), dict) else {}
    selected = manifest.get("selected_tasks") if isinstance(manifest.get("selected_tasks"), list) else []
    tasks: list[dict[str, Any]] = []
    for task in selected[:8]:
        if not isinstance(task, dict):
            continue
        try:
            task_number = int(task.get("task_number") or len(tasks) + 1)
        except (TypeError, ValueError):
            task_number = len(tasks) + 1
        artifact_path = str(task.get("artifact_path") or f"tasks/task_{task_number:02d}/task.md")
        parsed_task = parse_task_file(session_dir / artifact_path, task_number)
        quality = task.get("quality") if isinstance(task.get("quality"), dict) else {}
        parsed_quality = parsed_task.get("quality") if isinstance(parsed_task.get("quality"), dict) else {}
        tasks.append(
            {
                "task_id": task.get("task_id"),
                "task_number": task.get("task_number"),
                "title": task.get("title") or parsed_task.get("title"),
                "files": task.get("files") or parsed_task.get("files") or [],
                "issue": task.get("issue") or parsed_task.get("issue"),
                "origin": task.get("origin") or parsed_task.get("origin"),
                "artifact_path": artifact_path,
                "quality_score": quality.get("score", parsed_quality.get("score")),
                "generic_self_improvement": quality.get(
                    "generic_self_improvement",
                    parsed_quality.get("generic_self_improvement"),
                ),
            }
        )
    return {
        "planning_failed": bool(planner.get("planning_failed")),
        "task_count": int(planner.get("task_count") or 0),
        "selected_task_count": int(planner.get("selected_task_count") or 0),
        "assessment_present": bool(planner.get("assessment_present")),
        "assessment_missing_present": bool(planner.get("assessment_missing_present")),
        "issue_responses_present": bool(planner.get("issue_responses_present")),
        "planning_failure_present": bool(planner.get("planning_failure_present")),
        "tasks": tasks,
        "warnings": manifest.get("warnings") if isinstance(manifest.get("warnings"), list) else [],
        "artifacts": artifacts,
    }


def summarize_events_for_work(events: list[dict[str, Any]]) -> dict[str, Any]:
    edited_files: list[str] = []
    read_files: list[str] = []
    commands: list[str] = []
    failed_commands: list[str] = []
    failed_tools: list[str] = []
    tool_names: dict[str, int] = {}
    tool_starts: dict[str, dict[str, Any]] = {}
    command_starts: dict[str, str] = {}
    expected_cache_metrics = 0
    cache_metric_events = 0

    for event in events:
        kind = event_kind(event)
        data = event_payload(event)
        if kind == "RunStarted" and cache_metrics_expected(data):
            expected_cache_metrics += 1
        elif kind == "CacheMetricsRecorded":
            cache_metric_events += 1
        if kind == "FileEdited":
            path = data.get("path") or data.get("file") or data.get("target_path")
            if isinstance(path, str):
                edited_files.append(normalize_evidence_path(path))
        elif kind == "FileRead":
            path = data.get("path")
            if isinstance(path, str):
                read_files.append(normalize_evidence_path(path))
        elif kind == "CommandStarted":
            command = data.get("command")
            if isinstance(command, str):
                commands.append(clean_transcript_action(command))
                call_id = data.get("tool_call_id")
                if isinstance(call_id, str):
                    command_starts[call_id] = command
        elif kind == "CommandCompleted":
            call_id = data.get("tool_call_id")
            command = data.get("command")
            if not isinstance(command, str) and isinstance(call_id, str):
                command = command_starts.get(call_id)
            if not isinstance(command, str) and isinstance(call_id, str):
                started = tool_starts.get(call_id) or {}
                args = started.get("args") if isinstance(started.get("args"), dict) else {}
                command = args.get("command") if isinstance(args.get("command"), str) else None
            if isinstance(command, str):
                commands.append(clean_transcript_action(command))
            if (data.get("is_error") is True or exit_code_failure(data.get("result_preview"))) and isinstance(command, str):
                failed_commands.append(clean_transcript_action(command))
        if kind in {"ToolCallStarted", "ToolCallCompleted"}:
            tool = data.get("tool_name")
            call_id = data.get("tool_call_id")
            if isinstance(tool, str) and kind == "ToolCallStarted":
                tool_names[tool] = tool_names.get(tool, 0) + 1
                if isinstance(call_id, str):
                    tool_starts[call_id] = data
            if (
                kind == "ToolCallCompleted"
                and isinstance(tool, str)
                and (data.get("is_error") is True or exit_code_failure(data.get("result_preview")))
            ):
                started = tool_starts.get(call_id) if isinstance(call_id, str) else None
                args = started.get("args") if isinstance(started, dict) else None
                failed_tools.append(tool_failure_label(tool, args, data))

    return {
        "edited_files": compact_list(edited_files, 12),
        "read_files": compact_list(read_files, 12),
        "commands": compact_list(commands, 12),
        "failed_commands": compact_list(failed_commands, 8),
        "failed_tools": compact_list(failed_tools, 8),
        "tool_counts": dict(sorted(tool_names.items(), key=lambda item: (-item[1], item[0]))[:8]),
        "command_count": len(commands),
        "deepseek_cache_metric_expected_count": expected_cache_metrics,
        "deepseek_cache_metric_event_count": cache_metric_events,
        "deepseek_cache_metric_missing_count": max(expected_cache_metrics - cache_metric_events, 0),
    }


def work_summary(
    session_dir: Path,
    outcome: dict[str, Any],
    summary: dict[str, Any],
    evals: list[dict[str, Any]],
    blockers: list[dict[str, Any]],
    commits: list[dict[str, Any]],
) -> dict[str, Any]:
    transcript_data = transcript_summary(session_dir)
    state_pipeline = state_pipeline_summary(session_dir)
    task_manifest = task_manifest_summary(session_dir)
    task_artifacts = task_artifact_summary(session_dir)
    causal_chains = build_causal_chains(session_dir)
    suggestions = evolution_suggestions(session_dir)
    event_data = summarize_events_for_work(load_jsonl(session_dir / "state" / "events.jsonl"))
    transcript_actions = summarize_transcript_actions(session_dir)
    source_files = compact_list(
        [
            path
            for commit in commits
            for path in (commit.get("source_files") or [])
            if isinstance(path, str)
        ],
        16,
    )
    edited_files = compact_list(event_data["edited_files"] + transcript_actions["edited_files"], 12)
    touched_source_files = compact_list([path for path in edited_files if source_file(path)] + source_files, 12)
    read_files = compact_list(event_data["read_files"] + transcript_actions["read_files"], 12)
    commands = compact_list(event_data["commands"] + transcript_actions["commands"], 12)
    failed_commands = compact_list(event_data["failed_commands"] + transcript_actions["failed_commands"], 8)
    failed_tools = compact_list(event_data["failed_tools"] + transcript_actions["failed_tools"], 8)
    attempted = int(outcome.get("tasks_attempted") or 0)
    succeeded = int(outcome.get("tasks_succeeded") or 0)
    patches = summary.get("patches", []) if isinstance(summary.get("patches"), list) else []
    decisions = summary.get("decisions", []) if isinstance(summary.get("decisions"), list) else []
    latest_eval = evals[-1] if evals else {}
    source_commits = [commit for commit in commits if commit.get("source_files")]
    bookkeeping_commits = [commit for commit in commits if not commit.get("source_files")]
    task_lineage = summary.get("task_lineage") if isinstance(summary.get("task_lineage"), list) else []
    task_lineage = enrich_task_lineage_with_artifacts(task_lineage, task_artifacts)
    task_verification = task_verification_summary(task_manifest, task_artifacts, task_lineage)
    task_lineage = annotate_task_lineage_verification(task_lineage, task_verification)
    causal_chains = annotate_task_lineage_verification(causal_chains, task_verification)
    suggestions = augment_evolution_suggestions(suggestions, task_verification)
    source_patch_count = len(source_commits)
    assessment_artifact_present = bool(task_manifest.get("assessment_present")) if task_manifest else None
    assessment_transcript_present = bool((transcript_data.get("phase_counts") or {}).get("assess"))
    verification_rows = task_verification.get("rows") if isinstance(task_verification.get("rows"), list) else []
    unlanded_source_task_count = sum(
        1
        for row in verification_rows
        if isinstance(row, dict)
        and (
            "source_edits_not_landed" in (row.get("problems") or [])
            or "no_landed_source_commit" in (row.get("problems") or [])
        )
    )
    labels: list[str] = []
    if attempted:
        verified = int(task_verification.get("verified_task_count") or 0)
        strict_total = int(task_verification.get("task_count") or 0)
        if strict_total:
            labels.append(f"{verified}/{strict_total} verified tasks")
        else:
            labels.append(f"{succeeded}/{attempted} raw outcome task(s)")
            labels.append("missing strict task evidence")
        if strict_total and succeeded != verified:
            labels.append(f"outcome reported {succeeded}/{attempted} tasks")
        if strict_total > attempted:
            labels.append(f"{strict_total - attempted} selected task(s) not attempted")
    elif task_manifest.get("planning_failed"):
        labels.append("planning produced no task files")
    if unlanded_source_task_count:
        labels.append(f"{unlanded_source_task_count} unlanded source task(s)")
    if task_manifest and assessment_artifact_present is False:
        if assessment_transcript_present:
            labels.append("assessment artifact missing (assess transcript present)")
        else:
            labels.append("assessment artifact missing")
    if source_files:
        labels.append(f"{len(source_files)} source file(s) changed")
    elif touched_source_files:
        labels.append(f"{len(touched_source_files)} source file(s) touched")
    elif edited_files:
        labels.append(f"{len(edited_files)} evidence/bookkeeping file(s) edited")
    if failed_tools:
        labels.append(f"{len(failed_tools)} failed tool action(s)")
    command_count = len(commands)
    if command_count:
        labels.append(f"{command_count} command/check signal(s)")
    if evals:
        labels.append(f"{len(evals)} eval record(s)")
    if blockers:
        labels.append(f"{len(blockers)} blocker(s)")
    if not labels:
        labels.append("No detailed work signals captured")

    return {
        "headline": "; ".join(labels[:4]),
        "labels": labels,
        "assessment_artifact_present": assessment_artifact_present,
        "assessment_transcript_present": assessment_transcript_present,
        "unlanded_source_task_count": unlanded_source_task_count,
        "transcripts": transcript_data,
        "state_pipeline": state_pipeline,
        "task_manifest": task_manifest,
        "task_artifacts": task_artifacts,
        "task_verification": task_verification,
        "causal_chains": causal_chains,
        "evolution_suggestions": suggestions,
        "edited_files": edited_files,
        "touched_source_files": touched_source_files,
        "source_changed_files": source_files,
        "commits": serialize_commits(commits),
        "source_commits": serialize_commits(source_commits),
        "bookkeeping_commits": serialize_commits(bookkeeping_commits),
        "task_lineage": [task for task in task_lineage if isinstance(task, dict)],
        "read_files": read_files,
        "commands": commands,
        "failed_commands": failed_commands,
        "failed_tools": failed_tools,
        "transcript_actions": transcript_actions,
        "tool_counts": event_data["tool_counts"],
        "deepseek_cache_metric_expected_count": event_data["deepseek_cache_metric_expected_count"],
        "deepseek_cache_metric_event_count": event_data["deepseek_cache_metric_event_count"],
        "deepseek_cache_metric_missing_count": event_data["deepseek_cache_metric_missing_count"],
        "patch_count": len(patches),
        "state_patch_count": len(patches),
        "source_patch_count": source_patch_count,
        "landed_patch_count": source_patch_count,
        "landed_commit_count": len(commits),
        "source_commit_count": len(source_commits),
        "bookkeeping_commit_count": len(bookkeeping_commits),
        "decision_count": len(decisions),
        "eval_count": len(evals),
        "latest_eval_status": latest_eval.get("status"),
        "latest_eval_score": latest_eval.get("score"),
    }


def corrected_gnomes(
    summary: dict[str, Any],
    work: dict[str, Any],
    trace: dict[str, Any],
    outcome: dict[str, Any],
    feedback_metrics: dict[str, Any] | None = None,
) -> dict[str, Any]:
    gnomes = dict(summary.get("latest_gnomes") if isinstance(summary.get("latest_gnomes"), dict) else {})
    recalc_score = False
    if feedback_metrics:
        gnomes.update(feedback_metrics)
    if (
        "state_operational_capture_coverage" not in gnomes
        and "state_capture_coverage" in gnomes
        and isinstance(trace, dict)
    ):
        value = trace.get("operational_capture_coverage")
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            gnomes["state_operational_capture_coverage"] = value
            recalc_score = True
    if "task_lineage_capture_coverage" not in gnomes and isinstance(trace, dict):
        value = trace.get("task_lineage_capture_coverage")
        if isinstance(value, (int, float)) and not isinstance(value, bool) and value > 0:
            gnomes["task_lineage_capture_coverage"] = value
    if "task_lineage_event_count" not in gnomes and isinstance(trace, dict):
        value = trace.get("task_lineage_event_count")
        if isinstance(value, (int, float)) and not isinstance(value, bool) and value > 0:
            gnomes["task_lineage_event_count"] = value
    cache_hit_tokens = gnomes.get("deepseek_cache_hit_tokens")
    cache_miss_tokens = gnomes.get("deepseek_cache_miss_tokens")
    if gnomes.get("deepseek_cache_hit_ratio") is not None and not (
        isinstance(cache_hit_tokens, (int, float))
        and not isinstance(cache_hit_tokens, bool)
        and isinstance(cache_miss_tokens, (int, float))
        and not isinstance(cache_miss_tokens, bool)
    ):
        gnomes["deepseek_cache_hit_ratio"] = None
        recalc_score = True
        gnomes["deepseek_cache_ratio_unverified_count"] = max(
            int(gnomes.get("deepseek_cache_ratio_unverified_count") or 0),
            1,
        )
    cache_prose_mentions = int(gnomes.get("deepseek_cache_prose_mention_count") or 0)
    if gnomes.get("deepseek_cache_hit_ratio") is None and cache_prose_mentions:
        gnomes["deepseek_cache_ratio_unverified_count"] = max(
            int(gnomes.get("deepseek_cache_ratio_unverified_count") or 0),
            cache_prose_mentions,
        )
    cache_metric_signal = any(
        int(work.get(key) or 0) > 0
        for key in (
            "deepseek_cache_metric_expected_count",
            "deepseek_cache_metric_event_count",
            "deepseek_cache_metric_missing_count",
        )
    )
    if cache_metric_signal:
        for key in (
            "deepseek_cache_metric_expected_count",
            "deepseek_cache_metric_event_count",
            "deepseek_cache_metric_missing_count",
        ):
            value = work.get(key)
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                continue
            current = gnomes.get(key)
            if not isinstance(current, (int, float)) or isinstance(current, bool) or int(value) > int(current):
                gnomes[key] = int(value)
                recalc_score = True
    failed_tool_count = len(work.get("failed_tools") or []) if isinstance(work.get("failed_tools"), list) else 0
    if failed_tool_count > int(gnomes.get("tool_error_count") or 0):
        gnomes["tool_error_count"] = failed_tool_count
        recalc_score = True
    manifest = work.get("task_manifest") if isinstance(work.get("task_manifest"), dict) else {}
    verification = work.get("task_verification") if isinstance(work.get("task_verification"), dict) else {}
    task_artifacts = work.get("task_artifacts") if isinstance(work.get("task_artifacts"), list) else []
    attempted = int(outcome.get("tasks_attempted") or 0)
    selected_count = int(manifest.get("selected_task_count") or 0)
    if selected_count:
        task_unattempted = max(selected_count - attempted, 0)
        if task_unattempted:
            gnomes["task_unattempted_count"] = task_unattempted
            recalc_score = True
        gnomes["task_artifact_coverage"] = min(1.0, len(task_artifacts) / selected_count)
    task_count = int(verification.get("task_count") or 0)
    if task_count:
        verified = int(verification.get("verified_task_count") or 0)
        unverified = int(verification.get("unverified_task_count") or 0)
        unlanded = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict)
            and (
                "source_edits_not_landed" in (row.get("problems") or [])
                or "no_landed_source_commit" in (row.get("problems") or [])
            )
        )
        timeout_with_verdict = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict)
            and "evaluator_timed_out_after_verdict" in (row.get("problems") or [])
        )
        gnomes["task_success_rate"] = verified / task_count
        gnomes["session_success_rate"] = 1.0 if verified == task_count else 0.0
        recalc_score = True
        gnomes["evaluator_unverified_count"] = max(
            int(gnomes.get("evaluator_unverified_count") or 0),
            unverified,
        )
        gnomes["evaluator_timeout_with_verdict_count"] = max(
            int(gnomes.get("evaluator_timeout_with_verdict_count") or 0),
            timeout_with_verdict,
        )
        gnomes["task_unlanded_source_count"] = max(
            int(gnomes.get("task_unlanded_source_count") or 0),
            unlanded,
        )
    if manifest.get("planning_failed"):
        gnomes["planner_no_task_count"] = max(int(gnomes.get("planner_no_task_count") or 0), 1)
        gnomes["session_success_rate"] = 0.0
        gnomes["task_artifact_coverage"] = 0.0
        recalc_score = True
    merge = work.get("state_pipeline") if isinstance(work.get("state_pipeline"), dict) else {}
    if int(gnomes.get("state_live_baseline_shrink_count") or 0) > 0 and merge:
        try:
            legacy_projection_reset = (
                int(merge.get("merge_baseline_reset") or 0) == 1
                or (
                    int(merge.get("merge_effective_base_lines") or 0) == 0
                    and int(merge.get("merge_base_lines") or 0) > int(merge.get("merge_live_events") or 0)
                    and int(merge.get("merge_added_events") or 0) == int(merge.get("merge_live_events") or 0)
                )
            )
        except (TypeError, ValueError):
            legacy_projection_reset = False
        if legacy_projection_reset:
            gnomes["state_live_baseline_shrink_count"] = 0
            recalc_score = True
    if recalc_score:
        score = corrected_coding_log_score(gnomes)
        if score is not None:
            gnomes["coding_log_score"] = score
    return gnomes


def numeric_value(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def metric_float(metrics: dict[str, Any], key: str, default: float = 0.0) -> float:
    value = metrics.get(key)
    return float(value) if numeric_value(value) else default


def corrected_coding_log_score(metrics: dict[str, Any]) -> float | None:
    if not numeric_value(metrics.get("coding_log_score")):
        return None
    if not numeric_value(metrics.get("workflow_success_rate")) or not numeric_value(
        metrics.get("session_success_rate")
    ):
        return None
    task_rate = metrics.get("task_success_rate")
    outcome_parts = [
        metric_float(metrics, "workflow_success_rate"),
        metric_float(metrics, "session_success_rate"),
        float(task_rate) if numeric_value(task_rate) else 0.5,
        0.0 if metrics.get("session_reverted") is True else 1.0,
    ]
    outcome = sum(outcome_parts) / len(outcome_parts)
    failure_pressure = min(
        1.0,
        (
            metric_float(metrics, "distinct_failure_count")
            + metric_float(metrics, "provider_error_count")
            + metric_float(metrics, "json_error_count")
            + metric_float(metrics, "tool_error_count")
            + metric_float(metrics, "recurring_failure_count") * 2.0
            + metric_float(metrics, "evolution_friction_count")
            + metric_float(metrics, "planner_no_task_count") * 3.0
            + metric_float(metrics, "task_unattempted_count") * 2.0
            + metric_float(metrics, "evaluator_unverified_count")
            + metric_float(metrics, "evaluator_timeout_with_verdict_count") * 2.0
            + metric_float(metrics, "task_unlanded_source_count") * 2.0
            + metric_float(metrics, "state_live_baseline_shrink_count") * 2.0
        )
        / 12.0,
    )
    state_capture = metrics.get("state_operational_capture_coverage")
    if not numeric_value(state_capture):
        state_capture = metrics.get("state_capture_coverage")
    capture = (float(state_capture or 0.0) + metric_float(metrics, "audit_capture_coverage")) / 2.0
    reliability = max(0.0, (1.0 - failure_pressure) * 0.75 + capture * 0.25)
    efficiency = 1.0 - min(1.0, metric_float(metrics, "repair_loop_count") / 6.0)
    closed = metrics.get("closed_loop_fix_rate")
    learning = float(closed) if numeric_value(closed) else 0.5
    return round(outcome * 0.40 + reliability * 0.25 + efficiency * 0.20 + learning * 0.15, 4)


def gnome_corrections(raw: dict[str, Any], corrected: dict[str, Any]) -> dict[str, dict[str, Any]]:
    corrections: dict[str, dict[str, Any]] = {}
    for key in sorted(set(raw) | set(corrected)):
        if raw.get(key) != corrected.get(key):
            corrections[str(key)] = {"from": raw.get(key), "to": corrected.get(key)}
    return corrections


def normalize_latest_eval_gnomes(
    evals: list[dict[str, Any]],
    corrected: dict[str, Any],
) -> tuple[list[dict[str, Any]], dict[str, Any], dict[str, dict[str, Any]]]:
    if not evals:
        return [], {}, {}
    normalized = [dict(row) for row in evals]
    latest = dict(normalized[-1])
    raw = latest.get("gnomes") if isinstance(latest.get("gnomes"), dict) else {}
    corrections = gnome_corrections(raw, corrected)
    latest["gnomes"] = dict(corrected)
    score = corrected.get("coding_log_score")
    if numeric_value(score):
        latest["score"] = score
    if corrections:
        latest["gnome_corrections"] = corrections
    normalized[-1] = latest
    return normalized, latest, corrections


def normalize_task_gnome_snapshot(row: dict[str, Any], corrected: dict[str, Any]) -> None:
    if not corrected:
        return
    metrics = row.get("gnome_metrics") if isinstance(row.get("gnome_metrics"), dict) else {}
    deltas = dict(row.get("gnome_deltas") if isinstance(row.get("gnome_deltas"), dict) else {})
    corrections: dict[str, dict[str, Any]] = {}
    for key, corrected_value in corrected.items():
        current_value = metrics.get(key)
        if current_value == corrected_value:
            continue
        corrections[str(key)] = {"from": current_value, "to": corrected_value}
        if numeric_value(current_value) and numeric_value(corrected_value):
            adjustment = float(corrected_value) - float(current_value)
            current_delta = deltas.get(key)
            if numeric_value(current_delta):
                deltas[key] = round(float(current_delta) + adjustment, 6)
            elif current_value is not None:
                deltas[key] = round(adjustment, 6)
    row["gnome_metrics"] = dict(corrected)
    if deltas:
        row["gnome_deltas"] = deltas
    if corrections:
        row["gnome_corrections"] = corrections


def normalize_work_gnome_snapshots(work: dict[str, Any], corrected: dict[str, Any]) -> None:
    for key in ("task_lineage", "causal_chains"):
        rows = work.get(key)
        if not isinstance(rows, list):
            continue
        for row in rows:
            if isinstance(row, dict):
                normalize_task_gnome_snapshot(row, corrected)


def session_sort_key(path: Path) -> tuple[int, str, str]:
    parts = path.name.split("-", 2)
    if len(parts) == 3 and parts[0] == "day":
        try:
            return (int(parts[1]), parts[2], path.name)
        except ValueError:
            pass
    return (-1, path.name, path.name)


def is_real_blocker(blocker: dict[str, Any]) -> bool:
    reason = str(blocker.get("reason") or "").lower()
    if reason.startswith("allowed "):
        return False
    if " via session_always" in reason or " via repo_always" in reason:
        return False
    return True


def eval_dedupe_key(eval_data: dict[str, Any]) -> tuple[str, str]:
    suite = str(eval_data.get("suite") or "")
    eval_id = str(eval_data.get("eval_id") or "")
    if suite == "log-feedback" and eval_id.startswith("log-feedback-"):
        run_key = eval_id.removeprefix("log-feedback-").rsplit("-", 1)[0]
        if run_key:
            return (suite, run_key)
    return ("event", str(eval_data.get("event_id") or eval_id or id(eval_data)))


def dedupe_evals(evals: Any) -> list[dict[str, Any]]:
    if not isinstance(evals, list):
        return []
    order: list[tuple[str, str]] = []
    latest_by_key: dict[tuple[str, str], dict[str, Any]] = {}
    for eval_data in evals:
        if not isinstance(eval_data, dict):
            continue
        key = eval_dedupe_key(eval_data)
        if key not in latest_by_key:
            order.append(key)
        latest_by_key[key] = eval_data
    return [latest_by_key[key] for key in order]


def primary_decision(summary: dict[str, Any]) -> dict[str, Any]:
    decisions = summary.get("decisions") if isinstance(summary.get("decisions"), list) else []
    typed = [row for row in decisions if isinstance(row, dict)]
    if typed:
        for decision in reversed(typed):
            if decision.get("decision_type") != "tool_permission_policy":
                return decision
        return typed[-1]
    latest = summary.get("latest_decision")
    return latest if isinstance(latest, dict) else {}


def load_sessions(audit_sessions: Path, repo_root: Path) -> list[dict[str, Any]]:
    sessions: list[dict[str, Any]] = []
    if not audit_sessions.is_dir():
        return sessions

    for session_dir in sorted(audit_sessions.iterdir(), key=session_sort_key):
        if not session_dir.is_dir():
            continue
        outcome = load_json(session_dir / "outcome.json")
        summary = load_json(session_dir / "state" / "summary.json")
        evals = dedupe_evals(summary.get("evals", []))
        latest_eval = evals[-1] if evals else {}
        latest_decision = primary_decision(summary)
        blockers = [
            blocker
            for blocker in (summary.get("blockers", []) if isinstance(summary.get("blockers"), list) else [])
            if isinstance(blocker, dict) and is_real_blocker(blocker)
        ]
        commits = session_commits(outcome, repo_root)
        trace = trace_quality(summary, evals)
        work = work_summary(session_dir, outcome, summary, evals, blockers, commits)
        feedback_metrics = log_feedback_metrics(session_dir)
        latest_gnomes = corrected_gnomes(summary, work, trace, outcome, feedback_metrics)
        normalize_work_gnome_snapshots(work, latest_gnomes)
        evals, latest_eval, latest_eval_corrections = normalize_latest_eval_gnomes(evals, latest_gnomes)
        if latest_eval:
            work["latest_eval_status"] = latest_eval.get("status")
            work["latest_eval_score"] = latest_eval.get("score")
        session = {
            "id": session_dir.name,
            "day": outcome.get("day"),
            "ts": outcome.get("ts") or summary.get("generated_at"),
            "session_time": outcome.get("session_time"),
            "github_run_id": outcome.get("github_run_id"),
            "github_run_attempt": outcome.get("github_run_attempt"),
            "source_sha": outcome.get("source_sha"),
            "source_ref": outcome.get("source_ref"),
            "github_sha": outcome.get("github_sha"),
            "github_ref": outcome.get("github_ref"),
            "github_ref_name": outcome.get("github_ref_name"),
            "build_ok": outcome.get("build_ok"),
            "test_ok": outcome.get("test_ok"),
            "tasks_attempted": outcome.get("tasks_attempted"),
            "tasks_succeeded": outcome.get("tasks_succeeded"),
            "reverted": outcome.get("reverted"),
            "event_count": summary.get("event_count", 0),
            "event_counts": summary.get("event_counts", {}),
            "trace_quality": trace,
            "latest_gnomes": latest_gnomes,
            "gnome_corrections": gnome_corrections(
                summary.get("latest_gnomes") if isinstance(summary.get("latest_gnomes"), dict) else {},
                latest_gnomes,
            ),
            "gnome_keys": summary.get("gnome_keys", []),
            "evals": evals,
            "latest_eval": latest_eval,
            "latest_eval_gnome_corrections": latest_eval_corrections,
            "latest_decision": latest_decision,
            "patches": summary.get("patches", []),
            "decisions": summary.get("decisions", []),
            "blockers": blockers,
            "code_refs": summary.get("code_refs", []),
            "work_summary": work,
            "audit_url": f"{REPO_URL}/tree/audit-log/sessions/{session_dir.name}",
        }
        session["health"] = run_health(session)
        sessions.append(session)
    return sessions


def run_health(session: dict[str, Any]) -> str:
    attempted = session.get("tasks_attempted") or 0
    succeeded = session.get("tasks_succeeded") or 0
    work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
    manifest = work.get("task_manifest") if isinstance(work.get("task_manifest"), dict) else {}
    verification = work.get("task_verification") if isinstance(work.get("task_verification"), dict) else {}
    verified_total = int(verification.get("task_count") or 0)
    verified_count = int(verification.get("verified_task_count") or 0)
    if session.get("reverted"):
        return "reverted"
    if manifest.get("planning_failed"):
        return "attention"
    if verified_total and verified_count < verified_total:
        return "partial" if verified_count else "attention"
    if verified_total and verified_count == verified_total:
        if session.get("build_ok") is True and session.get("test_ok") is True:
            return "passed"
        return "partial"
    if attempted:
        return "attention"
    if session.get("build_ok") is True and session.get("test_ok") is True and attempted == succeeded:
        return "passed"
    if succeeded:
        return "partial"
    return "attention"


def aggregate(sessions: list[dict[str, Any]]) -> dict[str, Any]:
    promoted = 0
    rejected = 0
    blockers = 0
    evals = 0
    events = 0
    tasks_attempted = 0
    tasks_succeeded = 0
    raw_tasks_attempted = 0
    raw_tasks_succeeded = 0
    unverified_raw_task_attempted = 0
    unverified_raw_task_succeeded = 0
    latest_gnomes: dict[str, Any] = {}
    gnome_keys: list[str] = []
    health = {"passed": 0, "partial": 0, "attention": 0, "reverted": 0}
    event_counts: dict[str, int] = {}
    trace_event_count = 0
    full_trace_sessions = 0
    lifecycle_trace_sessions = 0
    feedback_only_sessions = 0

    for session in sessions:
        evals += 1 if session.get("latest_eval") else 0
        blockers += len(session.get("blockers") or [])
        events += int(session.get("event_count") or 0)
        work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
        verification = work.get("task_verification") if isinstance(work.get("task_verification"), dict) else {}
        strict_total = int(verification.get("task_count") or 0)
        raw_attempted = int(session.get("tasks_attempted") or 0)
        raw_succeeded = int(session.get("tasks_succeeded") or 0)
        raw_tasks_attempted += raw_attempted
        raw_tasks_succeeded += raw_succeeded
        if strict_total:
            tasks_attempted += strict_total
            tasks_succeeded += int(verification.get("verified_task_count") or 0)
        elif raw_attempted:
            unverified_raw_task_attempted += raw_attempted
            unverified_raw_task_succeeded += raw_succeeded
        health[run_health(session)] += 1
        trace = session.get("trace_quality") if isinstance(session.get("trace_quality"), dict) else {}
        trace_event_count += int(trace.get("trace_event_count") or 0)
        if trace.get("status") == "full":
            full_trace_sessions += 1
        if trace.get("status") == "lifecycle":
            lifecycle_trace_sessions += 1
        if trace.get("status") == "feedback_only":
            feedback_only_sessions += 1
        latest_gnomes.update(session.get("latest_gnomes") or {})
        for key in session.get("gnome_keys") or []:
            if isinstance(key, str) and key not in gnome_keys:
                gnome_keys.append(key)
        for key in (session.get("latest_gnomes") or {}).keys():
            if isinstance(key, str) and key not in gnome_keys:
                gnome_keys.append(key)
        for kind, count in (session.get("event_counts") or {}).items():
            if isinstance(count, int):
                event_counts[str(kind)] = event_counts.get(str(kind), 0) + count
        for decision in session.get("decisions") or []:
            decision_text = str(decision.get("decision") or "").lower()
            if decision.get("eligible") is True or "promote" in decision_text:
                promoted += 1
            if decision.get("eligible") is False or "reject" in decision_text:
                rejected += 1

    return {
        "session_count": len(sessions),
        "latest_session_id": sessions[-1].get("id") if sessions else None,
        "eval_count": evals,
        "promoted_decisions": promoted,
        "rejected_decisions": rejected,
        "blocker_count": blockers,
        "event_count": events,
        "trace_event_count": trace_event_count,
        "full_trace_sessions": full_trace_sessions,
        "lifecycle_trace_sessions": lifecycle_trace_sessions,
        "feedback_only_sessions": feedback_only_sessions,
        "tasks_attempted": tasks_attempted,
        "tasks_succeeded": tasks_succeeded,
        "task_success_rate": (tasks_succeeded / tasks_attempted) if tasks_attempted else None,
        "raw_task_outcome_attempted": raw_tasks_attempted,
        "raw_task_outcome_succeeded": raw_tasks_succeeded,
        "unverified_raw_task_outcome_attempted": unverified_raw_task_attempted,
        "unverified_raw_task_outcome_succeeded": unverified_raw_task_succeeded,
        "health": health,
        "event_counts": event_counts,
        "latest_gnomes": latest_gnomes,
        "gnome_keys": gnome_keys,
        "latest_ts": sessions[-1].get("ts") if sessions else None,
    }


def numeric_gnome_values(session: dict[str, Any]) -> dict[str, float]:
    values: dict[str, float] = {}
    for eval_data in session.get("evals") or []:
        gnomes = eval_data.get("gnomes") if isinstance(eval_data, dict) else None
        if not isinstance(gnomes, dict):
            continue
        for key, value in gnomes.items():
            if isinstance(value, bool) or value is None:
                continue
            if isinstance(value, (int, float)):
                values[str(key)] = float(value)
    for key, value in (session.get("latest_gnomes") or {}).items():
        if value is None:
            values.pop(str(key), None)
            continue
        if isinstance(value, bool) or value is None:
            continue
        if isinstance(value, (int, float)):
            values[str(key)] = float(value)
    return values


def build_gnome_history(sessions: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], list[str]]:
    history: list[dict[str, Any]] = []
    keys: list[str] = []
    for session in sessions:
        values = numeric_gnome_values(session)
        for key in values:
            if key not in keys:
                keys.append(key)
        history.append(
            {
                "session_id": session.get("id"),
                "day": session.get("day"),
                "ts": session.get("ts"),
                "health": run_health(session),
                "values": values,
            }
        )
    keys.sort()
    return history, keys


def int_or_none(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return None


def claim_row(
    name: str,
    status: str,
    expected: Any,
    actual: Any,
    evidence: list[str],
    detail: str,
    provenance: list[str],
) -> dict[str, Any]:
    return {
        "name": name,
        "status": status,
        "expected": expected,
        "actual": actual,
        "evidence": evidence,
        "detail": detail,
        "provenance": provenance,
    }


def count_claim(name: str, expected: int, actual: Any, evidence: list[str], detail: str) -> dict[str, Any]:
    actual_int = int_or_none(actual)
    if actual_int is not None and actual_int >= expected:
        status = "proven"
    elif expected == 0 and actual_int is None:
        status = "missing"
    else:
        status = "conflict"
    return claim_row(
        name,
        status,
        expected,
        actual,
        evidence,
        detail,
        ["work_summary", "latest_gnomes"],
    )


def task_verification_count_claim(work: dict[str, Any]) -> dict[str, Any]:
    verification = work.get("task_verification") if isinstance(work.get("task_verification"), dict) else {}
    rows = verification.get("rows") if isinstance(verification.get("rows"), list) else []
    expected_verified = sum(1 for row in rows if isinstance(row, dict) and row.get("strict_success") is True)
    expected_unverified = len([row for row in rows if isinstance(row, dict)]) - expected_verified
    actual = {
        "verified": verification.get("verified_task_count"),
        "unverified": verification.get("unverified_task_count"),
        "task_count": verification.get("task_count"),
    }
    status = "proven"
    if (
        int_or_none(actual["verified"]) != expected_verified
        or int_or_none(actual["unverified"]) != expected_unverified
        or int_or_none(actual["task_count"]) != len([row for row in rows if isinstance(row, dict)])
    ):
        status = "conflict"
    return claim_row(
        "task_verification_counts_match_rows",
        status,
        {
            "verified": expected_verified,
            "unverified": expected_unverified,
            "task_count": len([row for row in rows if isinstance(row, dict)]),
        },
        actual,
        [str(row.get("task_id")) for row in rows if isinstance(row, dict) and row.get("task_id")][:8],
        "Task verification summary counts should be derived from strict_success rows.",
        ["work_summary.task_verification.rows", "work_summary.task_verification"],
    )


def assessment_claim(work: dict[str, Any]) -> dict[str, Any]:
    manifest = work.get("task_manifest") if isinstance(work.get("task_manifest"), dict) else {}
    artifact_present = work.get("assessment_artifact_present")
    transcript_present = work.get("assessment_transcript_present")
    manifest_artifacts = manifest.get("artifacts") if isinstance(manifest.get("artifacts"), dict) else {}
    diagnostic_present = bool(manifest.get("assessment_missing_present") or manifest_artifacts.get("assessment_missing"))
    if not manifest:
        status = "missing"
        detail = "No task manifest is available, so assessment artifact state is unknown."
    elif artifact_present is True:
        status = "proven"
        detail = "Assessment artifact is present."
    elif diagnostic_present and transcript_present:
        status = "observed"
        detail = "Assessment transcript and missing-assessment diagnostic artifact exist, but assessment.md is missing."
    elif transcript_present:
        status = "observed"
        detail = "Assessment phase transcript exists but the assessment artifact is missing."
    else:
        status = "missing"
        detail = "Task manifest exists but no assessment artifact or assessment transcript was found."
    return claim_row(
        "assessment_artifact_and_transcript_state",
        status,
        {"artifact_present": True},
        {
            "artifact_present": artifact_present,
            "transcript_present": transcript_present,
            "diagnostic_present": diagnostic_present,
        },
        ["tasks/assessment.md", "tasks/assessment_missing.md", "transcripts/assess.log"],
        detail,
        ["work_summary.task_manifest", "work_summary.transcripts"],
    )


def cache_claim(gnomes: dict[str, Any]) -> dict[str, Any]:
    ratio = gnomes.get("deepseek_cache_hit_ratio")
    hit_tokens = gnomes.get("deepseek_cache_hit_tokens")
    miss_tokens = gnomes.get("deepseek_cache_miss_tokens")
    prose_mentions = int(gnomes.get("deepseek_cache_prose_mention_count") or 0)
    unverified = int(gnomes.get("deepseek_cache_ratio_unverified_count") or 0)
    expected_events = int(gnomes.get("deepseek_cache_metric_expected_count") or 0)
    metric_events = int(gnomes.get("deepseek_cache_metric_event_count") or 0)
    missing_events = int(gnomes.get("deepseek_cache_metric_missing_count") or 0)
    token_backed = (
        isinstance(hit_tokens, (int, float))
        and not isinstance(hit_tokens, bool)
        and isinstance(miss_tokens, (int, float))
        and not isinstance(miss_tokens, bool)
    )
    if ratio is not None and token_backed:
        status = "proven"
        detail = "Trusted cache ratio is backed by hit/miss token counts."
    elif ratio is not None:
        status = "conflict"
        detail = "Trusted cache ratio is present without token evidence."
    elif prose_mentions and unverified >= prose_mentions:
        status = "proven"
        detail = "Prose-only cache ratio claims are withheld from trusted KPI and counted as unverified."
    elif prose_mentions:
        status = "conflict"
        detail = "Prose-only cache ratio claims exist but are not counted as unverified."
    elif expected_events and missing_events:
        status = "missing"
        detail = "DeepSeek runs advertised cache metric recording but no CacheMetricsRecorded token evidence was captured."
    else:
        status = "missing"
        detail = "No DeepSeek cache ratio evidence was captured."
    return claim_row(
        "deepseek_cache_ratio_is_token_backed_or_marked_unverified",
        status,
        {
            "trusted_ratio_requires_tokens": True,
            "unverified_count_at_least_prose_mentions": prose_mentions,
            "cache_metric_events_at_least_expected": expected_events,
        },
        {
            "ratio": ratio,
            "hit_tokens": hit_tokens,
            "miss_tokens": miss_tokens,
            "prose_mentions": prose_mentions,
            "unverified_count": unverified,
            "expected_metric_events": expected_events,
            "metric_events": metric_events,
            "missing_metric_events": missing_events,
        },
        [
            "deepseek_cache_hit_ratio",
            "deepseek_cache_hit_tokens",
            "deepseek_cache_miss_tokens",
            "deepseek_cache_metric_expected_count",
            "deepseek_cache_metric_event_count",
            "deepseek_cache_metric_missing_count",
        ],
        detail,
        ["latest_gnomes", "log_feedback"],
    )


def session_claims(session: dict[str, Any]) -> list[dict[str, Any]]:
    work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
    gnomes = session.get("latest_gnomes") if isinstance(session.get("latest_gnomes"), dict) else {}
    failed_tools = work.get("failed_tools") if isinstance(work.get("failed_tools"), list) else []
    unlanded = int(work.get("unlanded_source_task_count") or 0)
    return [
        count_claim(
            "failed_tool_actions_match_tool_error_gnome",
            len(failed_tools),
            gnomes.get("tool_error_count"),
            failed_tools[:8],
            "Structured failed tool actions should be reflected in tool_error_count.",
        ),
        count_claim(
            "unlanded_source_tasks_match_gnome",
            unlanded,
            gnomes.get("task_unlanded_source_count"),
            [
                str(row.get("task_id"))
                for row in (
                    (work.get("task_verification") or {}).get("rows")
                    if isinstance(work.get("task_verification"), dict)
                    else []
                )
                if isinstance(row, dict)
                and (
                    "source_edits_not_landed" in (row.get("problems") or [])
                    or "no_landed_source_commit" in (row.get("problems") or [])
                )
            ][:8],
            "Tasks with source edits and no landed source commit should be reflected in task_unlanded_source_count.",
        ),
        task_verification_count_claim(work),
        assessment_claim(work),
        cache_claim(gnomes),
    ]


def build_claims_projection(
    sessions: list[dict[str, Any]],
    generated_at: str,
    audit_sessions: Path,
) -> dict[str, Any]:
    session_rows = []
    status_counts: dict[str, int] = {}
    claim_count = 0
    for session in sessions:
        claims = session_claims(session)
        claim_count += len(claims)
        for row in claims:
            status = str(row.get("status") or "unknown")
            status_counts[status] = status_counts.get(status, 0) + 1
        session_rows.append(
            {
                "id": session.get("id"),
                "ts": session.get("ts"),
                "health": session.get("health"),
                "headline": (session.get("work_summary") or {}).get("headline")
                if isinstance(session.get("work_summary"), dict)
                else None,
                "claims": claims,
            }
        )
    return {
        "schema_version": 1,
        "generated_at": generated_at,
        "source": str(audit_sessions),
        "summary": {
            "session_count": len(sessions),
            "claim_count": claim_count,
            "status_counts": dict(sorted(status_counts.items())),
        },
        "sessions": session_rows,
    }


HTML = r"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Yoyo DeepSeek Harness Evolution</title>
  <style>
    :root {
      color-scheme: light;
      --paper: #f2f5f1;
      --ink: #15140f;
      --muted: #59625d;
      --line: #cbd5cf;
      --panel: #fffdfa;
      --panel-strong: #e6eee8;
      --green: #1b7a58;
      --red: #b23a32;
      --blue: #285c92;
      --gold: #9b7018;
      --violet: #6d4aa2;
      --shadow: 0 18px 44px rgba(22, 36, 29, 0.12);
    }

    * { box-sizing: border-box; }
    body {
      margin: 0;
      background:
        linear-gradient(90deg, rgba(21, 20, 15, 0.045) 1px, transparent 1px),
        linear-gradient(rgba(21, 20, 15, 0.035) 1px, transparent 1px),
        var(--paper);
      background-size: 28px 28px;
      color: var(--ink);
      font: 15px/1.45 ui-monospace, "SFMono-Regular", "Cascadia Mono", "Liberation Mono", monospace;
    }

    header {
      padding: 28px clamp(18px, 4vw, 48px) 18px;
      border-bottom: 1px solid var(--line);
      background: rgba(242, 245, 241, 0.92);
      position: sticky;
      top: 0;
      z-index: 5;
      backdrop-filter: blur(10px);
    }

    h1 {
      margin: 0;
      font-size: clamp(28px, 5vw, 56px);
      line-height: 0.95;
      letter-spacing: 0;
      font-weight: 900;
      max-width: 980px;
    }

    .subhead {
      margin: 12px 0 0;
      max-width: 940px;
      color: var(--muted);
    }

    .note {
      margin-top: 12px;
      display: inline-flex;
      flex-wrap: wrap;
      gap: 8px;
      color: var(--muted);
      font-size: 13px;
    }

    main {
      width: min(1480px, 100%);
      margin: 0 auto;
      padding: 32px clamp(18px, 4vw, 56px) 64px;
      display: grid;
      gap: 32px;
    }

    .toolbar {
      display: grid;
      grid-template-columns: minmax(180px, 1fr) auto auto;
      gap: 12px;
      align-items: center;
    }

    input, select, button {
      border: 1px solid var(--line);
      background: var(--panel);
      color: var(--ink);
      min-height: 42px;
      padding: 0 12px;
      border-radius: 6px;
      font: inherit;
    }

    button {
      cursor: pointer;
      font-weight: 800;
    }

    .hero-report {
      display: grid;
      grid-template-columns: minmax(0, 1.35fr) minmax(260px, 0.65fr);
      gap: 24px;
      padding: clamp(20px, 4vw, 36px);
      border: 1px solid var(--line);
      border-radius: 8px;
      background:
        linear-gradient(135deg, rgba(230, 238, 232, 0.9), rgba(255, 253, 250, 0.92) 56%),
        var(--panel);
      box-shadow: var(--shadow);
    }

    .hero-title {
      margin: 12px 0 0;
      max-width: 920px;
      font-size: 44px;
      line-height: 1.08;
      font-weight: 900;
      letter-spacing: 0;
    }

    .hero-kicker {
      margin-top: 14px;
      display: flex;
      flex-wrap: wrap;
      gap: 8px 12px;
      align-items: center;
      color: var(--muted);
      font-size: 13px;
    }

    .hero-kicker code {
      color: var(--ink);
      background: rgba(230, 238, 232, 0.7);
      border: 1px solid var(--line);
      border-radius: 6px;
      padding: 3px 6px;
      overflow-wrap: anywhere;
    }

    .hero-copy {
      margin: 16px 0 0;
      max-width: 820px;
      color: var(--muted);
      font-size: 16px;
    }

    .hero-side {
      align-self: stretch;
      display: grid;
      align-content: space-between;
      gap: 18px;
      border-left: 1px solid var(--line);
      padding-left: 24px;
    }

    .hero-side .value {
      font-size: 52px;
    }

    .grid,
    .signal-strip {
      display: grid;
      grid-template-columns: repeat(4, minmax(170px, 1fr));
      gap: 14px;
    }

    .metric, .panel {
      border: 1px solid var(--line);
      background: rgba(255, 253, 250, 0.94);
      border-radius: 8px;
      box-shadow: var(--shadow);
    }

    .metric {
      min-height: 132px;
      padding: 18px;
      display: grid;
      align-content: space-between;
    }

    .metric small {
      color: var(--muted);
      display: block;
      margin-top: 8px;
      overflow-wrap: anywhere;
    }

    .label {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
      font-weight: 800;
    }

    .value {
      font-size: clamp(24px, 4vw, 42px);
      font-weight: 900;
      line-height: 1;
      overflow-wrap: anywhere;
    }

    .split {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(320px, 0.42fr);
      gap: 24px;
      align-items: start;
    }

    .chart-dashboard {
      display: grid;
      grid-template-columns: minmax(0, 1fr);
      gap: 24px;
      align-items: start;
    }

    .chart-dashboard.secondary {
      grid-template-columns: minmax(320px, 0.9fr) minmax(420px, 1.1fr);
    }

    .signal-rail {
      display: none;
    }

    .panel.feature .panel-body {
      gap: 22px;
      min-height: 560px;
    }

    .panel.compact .panel-body {
      gap: 16px;
    }

    .panel h2 {
      margin: 0;
      padding: 18px 22px;
      border-bottom: 1px solid var(--line);
      font-size: 17px;
      letter-spacing: 0;
      text-transform: uppercase;
    }

    .panel-body {
      padding: 20px 22px 24px;
      display: grid;
      gap: 20px;
    }

    .explain {
      color: var(--muted);
      margin: 0;
      max-width: 900px;
    }

    .bar-row {
      display: grid;
      gap: 7px;
    }

    .bar-meta {
      display: flex;
      justify-content: space-between;
      gap: 16px;
      color: var(--muted);
      font-size: 13px;
    }

    .bar-track {
      height: 16px;
      border: 1px solid var(--line);
      border-radius: 999px;
      overflow: hidden;
      background: #edf1ec;
      display: flex;
    }

    .bar-fill {
      min-width: 2px;
      height: 100%;
      background: var(--blue);
    }

    .bar-fill.good { background: var(--green); }
    .bar-fill.warn { background: var(--gold); }
    .bar-fill.bad { background: var(--red); }
    .bar-fill.info { background: var(--blue); }
    .bar-fill.violet { background: var(--violet); }

    .legend {
      display: flex;
      flex-wrap: wrap;
      gap: 8px 14px;
      color: var(--muted);
      font-size: 13px;
    }

    .legend span::before {
      content: "";
      display: inline-block;
      width: 10px;
      height: 10px;
      margin-right: 6px;
      border-radius: 2px;
      background: var(--blue);
    }

    .legend .passed::before { background: var(--green); }
    .legend .partial::before { background: var(--gold); }
    .legend .attention::before { background: var(--red); }
    .legend .reverted::before { background: var(--violet); }

    .detail-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 14px;
    }

    .section-head {
      display: flex;
      justify-content: space-between;
      gap: 24px;
      align-items: end;
      margin-bottom: 14px;
    }

    .section-head h2 {
      margin: 0;
      font-size: clamp(22px, 3vw, 34px);
      line-height: 1.05;
      text-transform: uppercase;
    }

    .section-head p {
      margin: 0;
      max-width: 680px;
      color: var(--muted);
    }

    .mini-list {
      margin: 8px 0 0;
      padding: 0;
      list-style: none;
      display: grid;
      gap: 5px;
      color: var(--muted);
      font-size: 12px;
    }

    .mini-list li {
      overflow-wrap: anywhere;
    }

    .work-row {
      display: grid;
      grid-template-columns: minmax(220px, 0.42fr) minmax(0, 1fr);
      gap: 24px;
      align-items: start;
      padding: 18px;
    }

    .work-meta {
      display: grid;
      gap: 10px;
    }

    .work-title {
      font-size: 17px;
      line-height: 1.25;
    }

    .work-facts {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
      gap: 10px;
    }

    .fact {
      border-top: 2px solid var(--line);
      padding-top: 8px;
      color: var(--muted);
      font-size: 12px;
    }

    .fact strong {
      display: block;
      color: var(--ink);
      font-size: 18px;
      line-height: 1.1;
    }

    details.work-details {
      margin-top: 8px;
    }

    details.work-details summary {
      cursor: pointer;
      color: var(--blue);
      font-weight: 800;
    }

    .sparkline {
      width: 100%;
      min-height: 420px;
      border: 1px solid var(--line);
      border-radius: 8px;
      background:
        linear-gradient(rgba(21, 20, 15, 0.05) 1px, transparent 1px),
        linear-gradient(90deg, rgba(21, 20, 15, 0.03) 1px, transparent 1px),
        #fffdf7;
      background-size: 100% 25%, 12.5% 100%;
      overflow: hidden;
    }

    .sparkline svg {
      display: block;
      width: 100%;
      height: 420px;
    }

    .panel.feature .sparkline {
      min-height: 440px;
    }

    .panel.feature .sparkline svg {
      height: 440px;
    }

    .sparkline text {
      fill: var(--muted);
      font-size: 11px;
    }

    .dot {
      display: inline-block;
      width: 9px;
      height: 9px;
      border-radius: 999px;
      margin-right: 6px;
      background: var(--blue);
    }

    .heatmap {
      display: grid;
      gap: 12px;
      overflow-x: auto;
    }

    .heat-row {
      display: grid;
      grid-template-columns: minmax(220px, 0.55fr) minmax(280px, 1fr);
      gap: 16px;
      align-items: center;
      color: var(--muted);
      font-size: 13px;
    }

    .heat-cells {
      display: grid;
      grid-auto-flow: column;
      grid-auto-columns: 18px;
      gap: 6px;
      min-width: max-content;
    }

    .heat-cell {
      width: 18px;
      height: 18px;
      border-radius: 4px;
      border: 1px solid var(--line);
      background: #eef2ed;
    }

    .heat-cell.on {
      background: var(--green);
      border-color: rgba(27, 122, 88, 0.35);
    }

    .table-wrap { overflow-x: auto; }
    table {
      width: 100%;
      border-collapse: collapse;
      min-width: 760px;
    }

    th, td {
      padding: 11px 12px;
      border-bottom: 1px solid var(--line);
      text-align: left;
      vertical-align: top;
    }

    th {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
    }

    tr:hover td { background: rgba(230, 238, 232, 0.58); }
    .pill {
      display: inline-flex;
      align-items: center;
      min-height: 24px;
      padding: 0 8px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: var(--panel-strong);
      font-size: 12px;
      font-weight: 900;
      white-space: nowrap;
    }

    .pill.soft {
      background: transparent;
      color: var(--muted);
    }

    .good { color: var(--green); }
    .bad { color: var(--red); }
    .info { color: var(--blue); }
    .warn { color: var(--gold); }
    .violet { color: var(--violet); }
    .stack {
      display: grid;
      gap: 14px;
      padding: 0;
    }

    .item {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #fffdf7;
      padding: 14px;
    }

    .item strong {
      display: block;
      margin-bottom: 4px;
      overflow-wrap: anywhere;
    }

    .item p {
      margin: 6px 0 0;
    }

    a {
      color: var(--blue);
      text-decoration-thickness: 1px;
      text-underline-offset: 3px;
    }

    .muted { color: var(--muted); }
    .empty {
      padding: 28px;
      color: var(--muted);
      text-align: center;
    }

    @media (max-width: 980px) {
      .hero-report,
      .work-row { grid-template-columns: 1fr; }
      .hero-side {
        border-left: 0;
        border-top: 1px solid var(--line);
        padding-left: 0;
        padding-top: 18px;
      }
      .grid,
      .signal-strip { grid-template-columns: repeat(2, minmax(130px, 1fr)); }
      .chart-dashboard,
      .chart-dashboard.secondary { grid-template-columns: 1fr; }
      .split { grid-template-columns: 1fr; }
      .toolbar { grid-template-columns: 1fr; }
      .work-facts { grid-template-columns: repeat(2, minmax(120px, 1fr)); }
      .hero-title { font-size: 34px; }
      .hero-side .value { font-size: 42px; }
    }

    @media (max-width: 520px) {
      main { gap: 24px; }
      .grid,
      .signal-strip,
      .work-facts { grid-template-columns: 1fr; }
      .detail-grid { grid-template-columns: 1fr; }
      .section-head { display: grid; }
      .hero-title { font-size: 28px; }
      header { position: static; }
    }
  </style>
</head>
<body>
  <header>
    <h1>DeepSeek harness evolution</h1>
    <p class="subhead">A human-readable view of yyds's self-improvement loop: what ran, whether it shipped, which state signals were captured, and where the audit evidence lives.</p>
    <div class="note">
      <span>Source: audit-log branch</span>
      <span>Only sessions with pushed audit evidence appear here.</span>
    </div>
  </header>
  <main>
    <section class="toolbar" aria-label="Dashboard filters">
      <input id="search" placeholder="Filter sessions, decisions, event types, evidence">
      <select id="status">
        <option value="all">All sessions</option>
        <option value="passed">Passed runs</option>
        <option value="attention">Needs attention</option>
        <option value="blocked">Has blockers</option>
        <option value="promoted">Promoted or eligible</option>
        <option value="rejected">Rejected or ineligible</option>
      </select>
      <button id="reset" type="button">Reset</button>
    </section>
    <section class="hero-report" id="heroSummary"></section>
    <section class="signal-strip" id="summary"></section>
    <section class="chart-dashboard">
      <section class="panel feature">
        <h2>Primary Gnome Trend</h2>
        <div class="panel-body">
          <p class="explain">The main longitudinal view. Select one numeric gnome and read the trend without competing charts. Missing values are gaps, not zeroes.</p>
          <select id="gnomeMetric" aria-label="Gnome metric"></select>
          <div id="gnomeTrend"></div>
        </div>
      </section>
    </section>
    <section aria-label="Session work">
      <div class="section-head">
        <h2>Session Work</h2>
        <p>What actually happened behind the metrics: completed tasks, source changes, validations, commits, and audit evidence.</p>
      </div>
      <div class="stack" id="sessionWork"></div>
    </section>
    <section class="panel compact">
      <h2>Feedback Loop Open Points</h2>
      <div class="panel-body">
        <p class="explain">Current weak spots in the yoagent-state feedback loop, derived from the visible audit evidence.</p>
        <div class="stack" id="feedbackLoop"></div>
      </div>
    </section>
    <section class="chart-dashboard secondary">
      <section class="panel compact">
        <h2>Run Signals</h2>
        <div class="panel-body">
          <p class="explain">Compact operational signals for the visible audit window.</p>
          <div id="healthChart"></div>
          <div class="legend">
            <span class="passed">passed</span>
            <span class="partial">partial</span>
            <span class="attention">needs attention</span>
            <span class="reverted">reverted</span>
          </div>
          <div id="taskChart"></div>
          <div id="eventChart"></div>
        </div>
      </section>
      <section class="panel compact">
        <h2>Gnome Evidence</h2>
        <div class="panel-body">
          <p class="explain">Priority health signals first, followed by metric availability. Availability means the session emitted the metric; missing does not mean zero.</p>
          <div id="gnomes"></div>
          <div id="metricNotes"></div>
          <div id="gnomeAvailability"></div>
        </div>
      </section>
    </section>
    <section class="split">
      <section class="panel">
        <h2>Raw Timeline</h2>
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Session</th>
                <th>Outcome</th>
                <th>Decision</th>
                <th>State</th>
                <th>Evidence</th>
              </tr>
            </thead>
            <tbody id="sessions"></tbody>
          </table>
        </div>
      </section>
      <section class="panel compact">
        <h2>Evidence Queue</h2>
        <div class="panel-body">
          <div class="stack" id="evidence"></div>
        </div>
      </section>
    </section>
  </main>
  <script>
    const fmt = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
    const state = { data: null, query: "", status: "all", selectedGnome: "" };
    const gnomeLabels = {
      cost_usd: "Estimated cost",
      cost_per_successful_task_usd: "Cost per successful task",
      latency_ms: "Latency",
      cache_hit_ratio: "Cache hit ratio",
      tool_call_malformed_rate: "Malformed tool calls",
      json_parse_failure_rate: "JSON parse failures",
      context_miss_rate: "Context misses",
      repair_loop_count: "Repair loops",
      state_failure_count: "State failures",
      fixture_agent_attempts: "Fixture agent attempts",
      fixture_agent_mutation_scope_failure_rate: "Mutation scope failures",
      fixture_agent_unexpected_changed_file_count: "Unexpected changed files",
      fim_compile_success_rate: "FIM compile success",
      fim_rollback_rate: "FIM rollback rate",
      fim_token_savings: "FIM token savings",
      deepseek_streaming_protocol_checks: "Streaming protocol checks",
      deepseek_prefix_cache_checks: "Prefix cache checks",
      deepseek_thinking_protocol_checks: "Thinking protocol checks",
      coding_log_score: "Coding log score",
      coding_log_confidence: "Coding log confidence",
      coding_log_available: "Coding log available",
      workflow_success_rate: "Workflow success",
      session_success_rate: "Session success",
      task_success_rate: "Task success",
      retry_success_rate: "Retry success",
      recurring_failure_count: "Recurring failures",
      max_failure_fingerprint_recurrence: "Max failure recurrence",
      state_capture_coverage: "State capture",
      state_operational_capture_coverage: "Operational state capture",
      state_live_baseline_shrink_count: "State baseline shrinks",
      audit_capture_coverage: "Audit capture",
      state_trace_event_count: "Trace events",
      closed_loop_fix_rate: "Closed-loop fix rate",
      evolution_friction_count: "Evolution friction",
      command_timeout_count: "Command timeouts",
      evaluator_timeout_count: "Evaluator timeouts",
      search_error_count: "Search errors",
      protected_file_revert_count: "Protected-file reverts",
      task_revert_count: "Task reverts",
      task_verification_rate: "Task verification",
      task_mechanical_verification_rate: "Mechanical task verification",
      planner_no_task_count: "Planner no-task count",
      task_unattempted_count: "Unattempted selected tasks",
      task_manifest_available: "Task manifest available",
      task_artifact_coverage: "Task artifact coverage",
      task_lineage_capture_coverage: "Task lineage capture",
      task_lineage_event_count: "Task lineage events",
      task_spec_quality_score: "Task spec quality",
      state_replay_integrity_rate: "State replay integrity",
      evaluator_unverified_count: "Evaluator unverified count",
      evaluator_timeout_with_verdict_count: "Evaluator verdict timeouts",
      task_unlanded_source_count: "Unlanded source tasks",
      max_task_turn_count: "Max task turns",
      avg_task_turn_count: "Avg task turns",
      total_task_turn_count: "Total task turns",
      deepseek_cache_hit_ratio: "DeepSeek cache hit ratio",
      deepseek_cache_hit_tokens: "DeepSeek cache hit tokens",
      deepseek_cache_miss_tokens: "DeepSeek cache miss tokens",
      deepseek_cache_metric_event_count: "Cache metric events",
      deepseek_cache_metric_expected_count: "Cache metric expected runs",
      deepseek_cache_metric_missing_count: "Missing cache metric events",
      deepseek_cache_ratio_unverified_count: "Unverified cache ratio reports"
    };
    const priorityGnomes = [
      "coding_log_score",
      "task_success_rate",
      "state_operational_capture_coverage",
      "state_live_baseline_shrink_count",
      "task_lineage_capture_coverage",
      "state_capture_coverage",
      "workflow_success_rate",
      "evolution_friction_count",
      "max_task_turn_count",
      "deepseek_cache_hit_ratio"
    ];

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, char => ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;"
      }[char]));
    }

    function text(value) {
      if (value === null || value === undefined || value === "") return "-";
      if (typeof value === "number") return escapeHtml(fmt.format(value));
      return escapeHtml(value);
    }

    function shortSha(value) {
      const raw = String(value || "").trim();
      return raw ? raw.slice(0, 12) : "";
    }

    function sessionSourceLine(session) {
      const sha = shortSha(session.source_sha || session.github_sha);
      const ref = session.source_ref || session.github_ref_name || session.github_ref || "";
      if (!sha && !ref) return "source revision not recorded";
      if (sha && ref) return `source ${sha} on ${ref}`;
      if (sha) return `source ${sha}`;
      return `source ${ref}`;
    }

    function metricChip(name, value) {
      return `<span class="pill">${text(name)}: ${text(value)}</span>`;
    }

    function percent(value) {
      if (value === null || value === undefined || Number.isNaN(Number(value))) return "-";
      return `${fmt.format(Number(value) * 100)}%`;
    }

    function metricValue(key, value) {
      if (value === null || value === undefined || Number.isNaN(Number(value))) return "-";
      const n = Number(value);
      if (key.endsWith("_rate") || key.endsWith("_ratio") || key.endsWith("_coverage")) return percent(n);
      if (key === "coding_log_score" || key === "coding_log_confidence") return fmt.format(n);
      return text(n);
    }

    function latestSession(sessions) {
      return (sessions || []).slice(-1)[0] || null;
    }

    function latestMetric(agg, key) {
      const value = (agg.latest_gnomes || {})[key];
      return value === undefined ? null : value;
    }

    function healthOf(session) {
      const attempted = Number(session.tasks_attempted || 0);
      const succeeded = Number(session.tasks_succeeded || 0);
      const work = session.work_summary || {};
      const manifest = work.task_manifest || {};
      const verification = work.task_verification || {};
      const strictTotal = Number(verification.task_count || 0);
      const strictVerified = Number(verification.verified_task_count || 0);
      if (session.reverted) return "reverted";
      if (manifest.planning_failed) return "attention";
      if (strictTotal && strictVerified < strictTotal) return strictVerified ? "partial" : "attention";
      if (strictTotal && strictVerified === strictTotal) {
        return session.build_ok === true && session.test_ok === true ? "passed" : "partial";
      }
      if (attempted) return "attention";
      if (session.build_ok === true && session.test_ok === true && attempted === succeeded) return "passed";
      if (succeeded > 0) return "partial";
      return "attention";
    }

    function healthClass(health) {
      if (health === "passed") return "good";
      if (health === "partial") return "warn";
      if (health === "reverted") return "violet";
      return "bad";
    }

    function decisionClass(decision) {
      const d = String(decision?.decision || "").toLowerCase();
      if (decision?.eligible === true || d.includes("promote")) return "good";
      if (decision?.eligible === false || d.includes("reject")) return "bad";
      return "warn";
    }

    function aggregateSessions(sessions, fallback = {}) {
      const health = { passed: 0, partial: 0, attention: 0, reverted: 0 };
      const eventCounts = {};
      const latestGnomes = {};
      const gnomeKeys = [];
      let eventCount = 0;
      let tasksAttempted = 0;
      let tasksSucceeded = 0;
      let rawTasksAttempted = 0;
      let rawTasksSucceeded = 0;
      let unverifiedRawTasksAttempted = 0;
      let unverifiedRawTasksSucceeded = 0;
      let evalCount = 0;
      let blockers = 0;
      let promoted = 0;
      let rejected = 0;
      let traceEventCount = 0;
      let fullTraceSessions = 0;
      let lifecycleTraceSessions = 0;
      let feedbackOnlySessions = 0;

      sessions.forEach(session => {
        const healthKey = healthOf(session);
        health[healthKey] = (health[healthKey] || 0) + 1;
        eventCount += Number(session.event_count || 0);
        const verification = session.work_summary?.task_verification || {};
        const strictTotal = Number(verification.task_count || 0);
        const rawAttempted = Number(session.tasks_attempted || 0);
        const rawSucceeded = Number(session.tasks_succeeded || 0);
        rawTasksAttempted += rawAttempted;
        rawTasksSucceeded += rawSucceeded;
        if (strictTotal) {
          tasksAttempted += strictTotal;
          tasksSucceeded += Number(verification.verified_task_count || 0);
        } else if (rawAttempted) {
          unverifiedRawTasksAttempted += rawAttempted;
          unverifiedRawTasksSucceeded += rawSucceeded;
        }
        blockers += (session.blockers || []).length;
        if (session.latest_eval && Object.keys(session.latest_eval).length) evalCount += 1;
        const trace = session.trace_quality || {};
        traceEventCount += Number(trace.trace_event_count || 0);
        if (trace.status === "full") fullTraceSessions += 1;
        if (trace.status === "lifecycle") lifecycleTraceSessions += 1;
        if (trace.status === "feedback_only") feedbackOnlySessions += 1;
        Object.entries(session.event_counts || {}).forEach(([kind, count]) => {
          eventCounts[kind] = (eventCounts[kind] || 0) + Number(count || 0);
        });
        Object.assign(latestGnomes, session.latest_gnomes || {});
        (session.gnome_keys || []).forEach(key => {
          if (!gnomeKeys.includes(key)) gnomeKeys.push(key);
        });
        (session.decisions || []).forEach(decision => {
          const text = String(decision.decision || "").toLowerCase();
          if (decision.eligible === true || text.includes("promote")) promoted += 1;
          if (decision.eligible === false || text.includes("reject")) rejected += 1;
        });
      });

      return {
        ...fallback,
        session_count: sessions.length,
        latest_session_id: sessions.length ? sessions[sessions.length - 1].id : null,
        latest_ts: sessions.length ? sessions[sessions.length - 1].ts : null,
        event_count: eventCount,
        trace_event_count: traceEventCount,
        full_trace_sessions: fullTraceSessions,
        lifecycle_trace_sessions: lifecycleTraceSessions,
        feedback_only_sessions: feedbackOnlySessions,
        event_counts: eventCounts,
        tasks_attempted: tasksAttempted,
        tasks_succeeded: tasksSucceeded,
        task_success_rate: tasksAttempted ? tasksSucceeded / tasksAttempted : null,
        raw_task_outcome_attempted: rawTasksAttempted,
        raw_task_outcome_succeeded: rawTasksSucceeded,
        unverified_raw_task_outcome_attempted: unverifiedRawTasksAttempted,
        unverified_raw_task_outcome_succeeded: unverifiedRawTasksSucceeded,
        eval_count: evalCount,
        blocker_count: blockers,
        promoted_decisions: promoted,
        rejected_decisions: rejected,
        health,
        latest_gnomes: latestGnomes,
        gnome_keys: gnomeKeys
      };
    }

    function matches(session) {
      const haystack = JSON.stringify(session).toLowerCase();
      if (state.query && !haystack.includes(state.query.toLowerCase())) return false;
      const decisions = session.decisions || [];
      const health = healthOf(session);
      if (state.status === "passed") return health === "passed";
      if (state.status === "attention") return health !== "passed";
      if (state.status === "blocked") return (session.blockers || []).length > 0;
      if (state.status === "promoted") return decisions.some(d => d.eligible === true || String(d.decision || "").toLowerCase().includes("promote"));
      if (state.status === "rejected") return decisions.some(d => d.eligible === false || String(d.decision || "").toLowerCase().includes("reject"));
      return true;
    }

    function barRow(label, value, max, className = "info", detail = "") {
      const safeMax = Math.max(Number(max) || 0, 1);
      const width = Math.max(0, Math.min(100, (Number(value) || 0) / safeMax * 100));
      return `<div class="bar-row">
        <div class="bar-meta"><strong>${text(label)}</strong><span>${text(value)}${detail ? ` ${text(detail)}` : ""}</span></div>
        <div class="bar-track"><div class="bar-fill ${className}" style="width:${width}%"></div></div>
      </div>`;
    }

    function stackedHealth(health) {
      const total = Object.values(health || {}).reduce((sum, value) => sum + Number(value || 0), 0) || 1;
      return `<div class="bar-track" title="Run health">
        ${["passed", "partial", "attention", "reverted"].map(key => {
          const width = Math.max(0, Number(health?.[key] || 0) / total * 100);
          return `<div class="bar-fill ${healthClass(key)}" style="width:${width}%"></div>`;
        }).join("")}
      </div>
      <div class="detail-grid">
        ${["passed", "partial", "attention", "reverted"].map(key => `
          <div class="item"><span class="pill ${healthClass(key)}">${key}</span><strong>${text(health?.[key] || 0)}</strong></div>
        `).join("")}
      </div>`;
    }

    function renderHero(sessions, agg) {
      const session = latestSession(sessions);
      const health = session ? healthOf(session) : "attention";
      const work = session ? (session.work_summary || {}) : {};
      const trace = session ? (session.trace_quality || {}) : {};
      const verification = work.task_verification || {};
      const strictTotal = Number(verification.task_count || 0);
      const strictVerified = Number(verification.verified_task_count || 0);
      const score = latestMetric(agg, "coding_log_score");
      const stateCapture = latestMetric(agg, "state_capture_coverage");
      const heroTitle = session
        ? (strictTotal
            ? `${text(strictVerified)} of ${text(strictTotal)} tasks verified`
            : (Number(session.tasks_attempted || 0)
                ? `${text(session.tasks_succeeded || 0)} of ${text(session.tasks_attempted || 0)} raw outcome tasks`
                : "No task evidence captured"))
        : "No audit-backed evolution sessions yet";
      const heroMeta = session
        ? `Day ${text(session.day)} / ${text(session.session_time || session.ts || "latest")} / <code>${text(session.id)}</code>`
        : "Waiting for the first pushed audit session";
      const heroCopy = session
        ? text(work.headline || "No detailed work signals captured")
        : "Run an evolution session and push audit evidence to populate this report.";
      document.getElementById("heroSummary").innerHTML = `
        <div>
          <span class="pill ${healthClass(health)}">${text(health)}</span>
          ${session ? `<span class="pill soft">${text(trace.label || "unknown trace")}</span>` : ""}
          <h2 class="hero-title">${heroTitle}</h2>
          <div class="hero-kicker">${heroMeta}</div>
          <p class="hero-copy">${heroCopy}</p>
        </div>
        <aside class="hero-side">
          <div>
            <div class="label">Latest coding score</div>
            <div class="value">${score === null ? "-" : metricValue("coding_log_score", score)}</div>
            <p class="muted">Gnome score from the latest log-feedback evidence.</p>
          </div>
          <div class="detail-grid">
            <div>
              <div class="label">State trace</div>
              <strong>${text(trace.trace_event_count || 0)} event(s)</strong>
            </div>
            <div>
              <div class="label">Audit evals</div>
              <strong>${text(agg.eval_count || 0)}</strong>
            </div>
          </div>
        </aside>`;
    }

    function renderSummary(agg) {
      const rate = agg.task_success_rate;
      const unverified = Number(agg.unverified_raw_task_outcome_attempted || 0);
      const rawHint = unverified
        ? `; ${text(unverified)} raw outcome task(s) lacked strict evidence`
        : "";
      const cards = [
        ["Sessions", agg.session_count || 0, "audit-backed runs"],
        ["Strict task success", rate === null || rate === undefined ? "-" : percent(rate), `${text(agg.tasks_succeeded || 0)}/${text(agg.tasks_attempted || 0)} verified tasks${rawHint}`],
        ["Operational traces", agg.full_trace_sessions || 0, `${text(agg.lifecycle_trace_sessions || 0)} lifecycle-only / ${text(agg.feedback_only_sessions || 0)} feedback-only`],
        ["Blockers", agg.blocker_count || 0, "real blocking signals"],
      ];
      document.getElementById("summary").innerHTML = cards.map(([label, value, hint]) => `
        <article class="metric">
          <div class="label">${label}</div>
          <div class="value">${text(value)}</div>
          <small>${text(hint)}</small>
        </article>
      `).join("");
    }

    function renderCharts(agg) {
      document.getElementById("healthChart").innerHTML = stackedHealth(agg.health || {});

      const attempted = Number(agg.tasks_attempted || 0);
      const succeeded = Number(agg.tasks_succeeded || 0);
      const unverified = Number(agg.unverified_raw_task_outcome_attempted || 0);
      document.getElementById("taskChart").innerHTML = attempted
        ? barRow("Strict verified tasks", succeeded, attempted, succeeded === attempted ? "good" : "warn", `of ${attempted}`)
        : `<div class="empty">No strict task verification rows in this filter.</div>`;
      if (unverified) {
        document.getElementById("taskChart").innerHTML += `<p class="explain">${text(unverified)} raw outcome task(s) are shown in session rows but excluded from strict success because task evidence was missing.</p>`;
      }

      const eventRows = Object.entries(agg.event_counts || {})
        .sort((a, b) => Number(b[1]) - Number(a[1]))
        .slice(0, 8);
      const eventMax = Math.max(...eventRows.map(([, value]) => Number(value || 0)), 1);
      document.getElementById("eventChart").innerHTML = eventRows.length
        ? eventRows.map(([kind, count]) => barRow(kind, count, eventMax, "info")).join("")
        : `<div class="empty">No state events captured yet.</div>`;

      const allGnomes = agg.latest_gnomes || {};
      const priorityRows = priorityGnomes
        .filter(key => Object.prototype.hasOwnProperty.call(allGnomes, key))
        .filter(key => allGnomes[key] !== null && allGnomes[key] !== undefined)
        .map(key => [key, allGnomes[key]]);
      const fallbackRows = Object.entries(allGnomes)
        .filter(([key]) => !priorityGnomes.includes(key))
        .filter(([, value]) => value !== null && value !== undefined)
        .slice(0, Math.max(0, 4 - priorityRows.length));
      const gnomeRows = priorityRows.concat(fallbackRows).slice(0, 4);
      document.getElementById("gnomes").innerHTML = gnomeRows.length
        ? `<div class="detail-grid">${gnomeRows.map(([key, value]) => `
          <article class="item">
            <strong>${text(gnomeLabels[key] || key)}</strong>
            <p class="value">${metricValue(key, value)}</p>
            <small class="muted">${text(key)}</small>
          </article>
        `).join("")}</div>`
        : (agg.gnome_keys || []).length
          ? `<div class="stack">${(agg.gnome_keys || []).slice(0, 16).map(key => `<span class="pill soft">${text(gnomeLabels[key] || key)}</span>`).join("")}</div><p class="explain">These signals are configured, but this audit window has not emitted numeric KPI values yet.</p>`
          : `<div class="empty">No gnome KPI values captured yet. This is expected until eval or log-feedback events emit metrics.</div>`;
      renderMetricNotes(agg);
    }

    function renderMetricNotes(agg) {
      const gnomes = agg.latest_gnomes || {};
      const notes = [];
      const unverifiedCache = Number(gnomes.deepseek_cache_ratio_unverified_count || 0);
      if (unverifiedCache > 0 && (gnomes.deepseek_cache_hit_ratio === null || gnomes.deepseek_cache_hit_ratio === undefined)) {
        notes.push({
          kind: "Cache evidence",
          className: "warn",
          text: `${text(unverifiedCache)} DeepSeek cache ratio report(s) were withheld because token evidence was missing. A trusted cache KPI needs CacheMetricsRecorded events with prompt_cache_hit_tokens and prompt_cache_miss_tokens.`
        });
      }
      const missingCacheMetrics = Number(gnomes.deepseek_cache_metric_missing_count || 0);
      if (missingCacheMetrics > 0) {
        notes.push({
          kind: "Cache metrics",
          className: "warn",
          text: `${text(missingCacheMetrics)} DeepSeek run(s) advertised cache metric recording but did not emit CacheMetricsRecorded events. Cache ratio remains untrusted until hit/miss token events are present.`
        });
      }
      const stateCoverage = Number(gnomes.state_capture_coverage ?? NaN);
      const operationalCoverage = Number(gnomes.state_operational_capture_coverage ?? NaN);
      const lineageCoverage = Number(gnomes.task_lineage_capture_coverage ?? NaN);
      if (!Number.isNaN(stateCoverage) && stateCoverage > 0 && !Number.isNaN(operationalCoverage) && operationalCoverage === 0) {
        const lineageText = !Number.isNaN(lineageCoverage) && lineageCoverage > 0
          ? " Task lineage evidence exists, but it is not a substitute for model/tool/cache events."
          : "";
        notes.push({
          kind: "State evidence",
          className: "warn",
          text: `State replay evidence exists, but the latest visible session did not capture yyds/tool operational events.${lineageText} Treat its trace as thin until a fresh run emits model/tool/cache events.`
        });
      }
      document.getElementById("metricNotes").innerHTML = notes.length
        ? `<div class="stack metric-notes">${notes.map(note => `<p class="explain"><span class="pill ${note.className}">${text(note.kind)}</span> ${note.text}</p>`).join("")}</div>`
        : "";
    }

    function gnomeRowsForFilteredSessions(sessions) {
      const visible = new Set(sessions.map(session => session.id));
      return ((state.data && state.data.gnome_history) || []).filter(row => visible.has(row.session_id));
    }

    function availableGnomeKeys(rows) {
      const keys = [];
      rows.forEach(row => {
        Object.entries(row.values || {}).forEach(([key, value]) => {
          if (value === null || value === undefined || Number.isNaN(Number(value))) return;
          if (!keys.includes(key)) keys.push(key);
        });
      });
      keys.sort();
      const preferred = ["coding_log_score", "task_success_rate", "workflow_success_rate", "state_operational_capture_coverage", "state_live_baseline_shrink_count", "task_lineage_capture_coverage", "state_capture_coverage", "evolution_friction_count", "max_task_turn_count", "deepseek_cache_hit_ratio", "cache_hit_ratio"];
      preferred.reverse().forEach(key => {
        const idx = keys.indexOf(key);
        if (idx >= 0) {
          keys.splice(idx, 1);
          keys.unshift(key);
        }
      });
      return keys;
    }

    function renderGnomeMetricSelect(keys) {
      const select = document.getElementById("gnomeMetric");
      if (!keys.length) {
        select.innerHTML = `<option value="">No numeric gnomes</option>`;
        state.selectedGnome = "";
        return;
      }
      if (!state.selectedGnome || !keys.includes(state.selectedGnome)) {
        state.selectedGnome = keys[0];
      }
      select.innerHTML = keys.map(key => `<option value="${escapeHtml(key)}"${key === state.selectedGnome ? " selected" : ""}>${text(gnomeLabels[key] || key)}</option>`).join("");
    }

    function renderSparkline(rows, key) {
      const points = rows
        .map((row, index) => ({ row, index, value: row.values ? row.values[key] : undefined }))
        .filter(point => point.value !== null && point.value !== undefined && !Number.isNaN(Number(point.value)))
        .map(point => ({ ...point, value: Number(point.value) }));
      if (!key || points.length < 2) {
        return `<div class="empty">Need at least two sessions with numeric values for a trend.</div>`;
      }
      const width = 920;
      const height = 300;
      const padX = 54;
      const padY = 36;
      const values = points.map(point => point.value);
      let min = Math.min(...values);
      let max = Math.max(...values);
      if (min === max) {
        min -= 1;
        max += 1;
      }
      const maxIndex = Math.max(rows.length - 1, 1);
      const xy = point => {
        const x = padX + (point.index / maxIndex) * (width - padX * 2);
        const y = height - padY - ((point.value - min) / (max - min)) * (height - padY * 2);
        return [x, y];
      };
      const path = points.map((point, idx) => {
        const [x, y] = xy(point);
        return `${idx === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
      }).join(" ");
      const circles = points.map(point => {
        const [x, y] = xy(point);
        return `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="6"><title>${text(point.row.session_id)}: ${text(point.value)}</title></circle>`;
      }).join("");
      return `<div class="sparkline">
        <svg viewBox="0 0 ${width} ${height}" role="img" aria-label="${text(key)} trend">
          <line x1="${padX}" y1="${padY}" x2="${padX}" y2="${height - padY}" stroke="#cbd5cf" />
          <line x1="${padX}" y1="${height - padY}" x2="${width - padX}" y2="${height - padY}" stroke="#cbd5cf" />
          <text x="${padX}" y="22">${text(max)}</text>
          <text x="${padX}" y="${height - 8}">${text(min)}</text>
          <path d="${path}" fill="none" stroke="#285c92" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" />
          <g fill="#1b7a58" stroke="#fffdfa" stroke-width="3">${circles}</g>
        </svg>
      </div>
      <div class="legend"><span><span class="dot"></span>${text(gnomeLabels[key] || key)}</span><span>${text(points.length)} of ${text(rows.length)} visible sessions emitted this metric</span></div>`;
    }

    function renderGnomeAvailability(rows, keys) {
      const panel = document.getElementById("gnomeAvailability");
      if (!rows.length || !keys.length) {
        panel.innerHTML = `<div class="empty">No gnome history in the current filter.</div>`;
        return;
      }
      const ordered = priorityGnomes
        .filter(key => keys.includes(key))
        .concat(keys.filter(key => !priorityGnomes.includes(key)));
      panel.innerHTML = `<div class="heatmap">${ordered.slice(0, 12).map(key => `
        <div class="heat-row">
          <strong>${text(gnomeLabels[key] || key)}</strong>
          <div class="heat-cells">${rows.map(row => {
            const on = row.values && row.values[key] !== null && row.values[key] !== undefined;
            return `<span class="heat-cell ${on ? "on" : ""}" title="${text(row.session_id)} ${on ? text(row.values[key]) : "missing"}"></span>`;
          }).join("")}</div>
        </div>
      `).join("")}</div><p class="explain">Each cell is one visible session. Filled means the metric was emitted; blank means missing, not zero.</p>`;
    }

    function renderGnomeHistory(sessions) {
      const rows = gnomeRowsForFilteredSessions(sessions);
      const keys = availableGnomeKeys(rows);
      renderGnomeMetricSelect(keys);
      document.getElementById("gnomeTrend").innerHTML = renderSparkline(rows, state.selectedGnome);
      renderGnomeAvailability(rows, keys);
    }

    function listItems(values, emptyText) {
      const rows = (values || []).filter(Boolean).slice(0, 6);
      if (!rows.length) return `<p class="muted">${text(emptyText)}</p>`;
      return `<ul class="mini-list">${rows.map(value => `<li>${text(value)}</li>`).join("")}</ul>`;
    }

    function commitItems(commits) {
      const rows = (commits || []).slice(0, 6);
      if (!rows.length) return `<p class="muted">No landed commits matched this session.</p>`;
      return `<ul class="mini-list">${rows.map(commit => {
        const files = (commit.source_files || commit.files || []).length;
        const count = files ? ` (${text(files)} files)` : "";
        return `<li>${text(commit.short_sha || "")} ${text(commit.subject || "")}${count}</li>`;
      }).join("")}</ul>`;
    }

    function sourceCommitItems(work) {
      const sourceCommits = work.source_commits || [];
      if (sourceCommits.length) return commitItems(sourceCommits);
      return `<p class="muted">No source-changing commits matched this session.</p>`;
    }

    function bookkeepingCommitItems(work) {
      const rows = (work.bookkeeping_commits || []).slice(0, 5);
      if (!rows.length) return `<p class="muted">No bookkeeping commits recorded.</p>`;
      return `<ul class="mini-list">${rows.map(commit => `<li>${text(commit.short_sha || "")} ${text(commit.subject || "")}</li>`).join("")}</ul>`;
    }

    function auditLink(session, path, label) {
      if (!session.audit_url || !path) return text(label || path || "");
      return `<a href="${text(session.audit_url)}/${text(path)}">${text(label || path)}</a>`;
    }

    function renderTranscriptList(session, work) {
      const rows = ((work.transcripts || {}).files || []).slice(0, 10);
      if (!rows.length) return `<p class="muted">No transcript files recorded.</p>`;
      return `<ul class="mini-list">${rows.map(row => {
        const size = `${text(row.line_count || 0)} lines`;
        return `<li>${auditLink(session, row.path, row.name)} <span class="muted">${text(row.phase || "other")} / ${size}</span></li>`;
      }).join("")}</ul>`;
    }

    function renderStatePipeline(work) {
      const pipe = work.state_pipeline || {};
      const hasReplay = pipe.replay_events_written !== undefined && pipe.replay_events_written !== null;
      const hasMerge = pipe.merge_added_events !== undefined && pipe.merge_added_events !== null;
      if (!hasReplay && !hasMerge && !(pipe.append_log_lines || pipe.append_problem_lines)) {
        return `<p class="muted">No state pipeline diagnostics recorded.</p>`;
      }
      const rows = [];
      if (hasReplay) {
        rows.push(`audit replay ${text(pipe.replay_events_written || 0)} event(s) from ${text(pipe.replay_files_read || 0)} session file(s); ${text(pipe.replay_duplicates_skipped || 0)} duplicate(s) skipped`);
      }
      if (hasMerge) {
        rows.push(`live delta ${text(pipe.merge_added_events || 0)} added / ${text(pipe.merge_delta_events || 0)} seen; ${text(pipe.session_events_after_merge || 0)} session event(s) after merge`);
        if (pipe.merge_baseline_shrunk) {
          rows.push(`warning: live state log had fewer events than replay baseline, so merge used ${text(pipe.merge_effective_base_lines || 0)} as the effective baseline`);
        }
      } else if (hasReplay) {
        rows.push(`live delta merge diagnostics not recorded for this session`);
      }
      if (pipe.append_log_lines || pipe.append_problem_lines) {
        rows.push(`append log ${text(pipe.append_log_lines || 0)} line(s); ${text(pipe.append_problem_lines || 0)} problem line(s)`);
      }
      return `<ul class="mini-list">${rows.map(row => `<li>${row}</li>`).join("")}</ul>`;
    }

    function renderTaskArtifacts(session, work) {
      const manifest = work.task_manifest || {};
      const verification = work.task_verification || {};
      const rows = (work.task_artifacts || []).slice(0, 6);
      const manifestArtifacts = manifest.artifacts || {};
      const manifestLinks = [
        manifestArtifacts.manifest ? auditLink(session, manifestArtifacts.manifest, "manifest.json") : "",
        manifestArtifacts.assessment ? auditLink(session, manifestArtifacts.assessment, "assessment.md") : "",
        manifestArtifacts.planning_failure ? auditLink(session, manifestArtifacts.planning_failure, "planning_failure.md") : "",
        manifestArtifacts.issue_responses ? auditLink(session, manifestArtifacts.issue_responses, "issue_responses.md") : ""
      ].filter(Boolean).join(" · ");
      const manifestTasks = (manifest.tasks || []).slice(0, 4).map(task => {
        const quality = task.quality_score === undefined || task.quality_score === null ? "-" : task.quality_score;
        const files = (task.files || []).slice(0, 3).join(", ") || "no files";
        const link = task.artifact_path ? auditLink(session, task.artifact_path, task.task_id || task.title) : text(task.task_id || task.title || "");
        return `${link} <span class="muted">${text(task.title || "")} / ${text(files)} / quality ${text(quality)}</span>`;
      }).join("</li><li>");
      const planningFailure = manifest.planning_failed ? `<p class="bad"><strong>Planning guard failed.</strong> No task files were produced, so implementation was skipped. ${manifestArtifacts.planning_failure ? auditLink(session, manifestArtifacts.planning_failure, "Open planning_failure.md") : ""}</p>` : "";
      const verificationBlock = verification.task_count !== undefined ? `<p class="${verification.unverified_task_count ? "warn" : "muted"}">${text(verification.verified_task_count || 0)}/${text(verification.task_count || 0)} strict task(s) verified. ${text(verification.unverified_task_count || 0)} unverified.</p>` : "";
      const verificationRows = (verification.rows || []).slice(0, 4).map(row => {
        const problems = (row.problems || []).join(", ") || "verified";
        const klass = row.strict_success ? "good" : "warn";
        const planned = text((row.planned_files || []).slice(0, 2).join(", ") || "none");
        const touched = text((row.touched_files || []).slice(0, 2).join(", ") || "none");
        const reason = row.revert_reason ? `; ${text(row.revert_reason)}` : "";
        return `<li><span class="${klass}">${text(row.task_id || "")}</span> ${text(row.title || "")}<br><span class="muted">planned ${planned} → touched ${touched} → ${text(problems)}${reason}</span></li>`;
      }).join("");
      const manifestBlock = manifest.task_count !== undefined ? `<div class="task-evidence">
          <strong>Plan decision ${manifest.planning_failed ? "(planning failed)" : ""}</strong>
          ${planningFailure}
          <p class="muted">${text(manifest.selected_task_count || 0)} selected of ${text(manifest.task_count || 0)} task file(s). ${text((manifest.warnings || []).join(", ") || "No manifest warnings.")}</p>
          ${verificationBlock}
          ${verificationRows ? `<ul class="mini-list">${verificationRows}</ul>` : ""}
          ${manifestLinks ? `<p class="muted">${manifestLinks}</p>` : ""}
          ${manifestTasks ? `<ul class="mini-list"><li>${manifestTasks}</li></ul>` : ""}
        </div>` : "";
      if (!rows.length) return manifestBlock || `<p class="muted">No per-task artifact bundle recorded yet.</p>`;
      return manifestBlock + rows.map(task => {
        const statuses = (task.eval_statuses || []).length ? task.eval_statuses.join(", ") : "no eval artifact";
        const attempts = (task.attempts || []).slice(0, 4).map(attempt => {
          const name = attempt.transcript_path ? auditLink(session, attempt.transcript_path, attempt.stage_name || attempt.phase) : text(attempt.stage_name || attempt.phase || "attempt");
          const turns = attempt.turn_count === undefined || attempt.turn_count === null ? "-" : attempt.turn_count;
          return `${name} <span class="muted">${text(attempt.status || "-")} / ${text(turns)} turns / ${text(attempt.line_count || 0)} lines</span>`;
        }).join("</li><li>");
        const evalRows = (task.evals || []).slice(0, 3).map(evalRow => {
          const label = evalRow.transcript_path ? auditLink(session, evalRow.transcript_path, `eval ${evalRow.attempt || ""}`.trim()) : `eval ${text(evalRow.attempt || "")}`;
          const verdict = evalRow.verdict || evalRow.status || "no verdict";
          const reason = evalRow.reason ? ` — ${String(evalRow.reason).slice(0, 220)}` : "";
          const klass = String(evalRow.status || evalRow.verdict || "").toLowerCase().includes("pass") ? "good" : "warn";
          return `${label} <span class="${klass}">${text(verdict)}</span><span class="muted">${text(reason)}</span>`;
        }).join("</li><li>");
        const artifacts = (task.artifacts || []).slice(0, 6).map(artifact =>
          `${auditLink(session, artifact.path, artifact.name)} <span class="muted">${text(artifact.line_count || 0)} lines</span>`
        ).join("</li><li>");
        return `<div class="task-evidence">
          <strong>${text(task.task_id || "")} ${text(task.status || "")}: ${text(task.task_title || "")}</strong>
          ${task.revert_reason ? `<p class="warn">${text(task.revert_reason)}</p>` : ""}
          <p class="muted">${text(task.attempt_count || 0)} attempt artifact(s); max ${text(task.max_turn_count || 0)} turns; eval ${text(statuses)}; task file ${text(task.task_line_count || 0)} lines</p>
          ${attempts ? `<ul class="mini-list"><li>${attempts}</li></ul>` : ""}
          ${evalRows ? `<ul class="mini-list"><li>${evalRows}</li></ul>` : ""}
          ${artifacts ? `<ul class="mini-list"><li>${artifacts}</li></ul>` : ""}
        </div>`;
      }).join("");
    }

    function metricDeltaValue(key, value) {
      if (value === null || value === undefined || Number.isNaN(Number(value))) return "-";
      const n = Number(value);
      const prefix = n > 0 ? "+" : "";
      return `${prefix}${metricValue(key, n)}`;
    }

    function summarizeGnomeMovement(row) {
      const corrections = row.gnome_corrections || {};
      const deltas = row.gnome_deltas || {};
      const priority = [
        "task_success_rate",
        "session_success_rate",
        "task_verification_rate",
        "evaluator_unverified_count",
        "evaluator_timeout_with_verdict_count",
        "task_unlanded_source_count",
        "max_task_turn_count",
        "state_operational_capture_coverage",
        "task_lineage_capture_coverage",
        "deepseek_cache_hit_ratio",
        "deepseek_cache_metric_missing_count",
        "deepseek_cache_ratio_unverified_count",
        "coding_log_score"
      ];
      const allKeys = Array.from(new Set(Object.keys(corrections).concat(Object.keys(deltas))));
      const ordered = priority.filter(key => allKeys.includes(key)).concat(allKeys.filter(key => !priority.includes(key)));
      const parts = ordered.slice(0, 4).map(key => {
        const label = text(gnomeLabels[key] || key);
        const correction = corrections[key];
        if (correction && typeof correction === "object" && ("from" in correction || "to" in correction)) {
          return `${label} ${metricValue(key, correction.from)}→${metricValue(key, correction.to)}`;
        }
        return `${label} ${metricDeltaValue(key, deltas[key])}`;
      });
      const remaining = Math.max(0, allKeys.length - parts.length);
      return parts.length ? `${parts.join(", ")}${remaining ? ` +${text(remaining)} more` : ""}` : "";
    }

    function renderCausalChains(work) {
      const rows = (work.causal_chains || []).slice(0, 6);
      if (!rows.length) return `<p class="muted">No causal-chain rows recorded yet.</p>`;
      return `<ul class="mini-list">${rows.map(row => {
        const planned = (row.planned_files || []).slice(0, 2).join(", ") || "no planned files";
        const touched = (row.source_files || row.touched_files || []).slice(0, 2).join(", ") || "no touched files";
        const commits = (row.commit_shas || []).map(sha => String(sha).slice(0, 7)).join(", ") || "no commit";
        const evalText = row.eval_verdict || (row.eval_statuses || []).join(", ") || "no eval";
        const strict = row.verification_status || (row.strict_success ? "strict_pass" : "");
        const strictClass = strict === "strict_pass" ? "good" : (strict === "strict_failed" ? "warn" : "muted");
        const strictText = strict ? ` → <span class="${strictClass}">${text(strict.replace("_", " "))}</span>` : "";
        const problems = (row.verification_problems || []).slice(0, 3).join(", ");
        const problemText = problems ? ` (${text(problems)})` : "";
        const deltaCount = Object.keys(row.gnome_deltas || {}).length;
        const correctionCount = Object.keys(row.gnome_corrections || {}).length;
        const correctionText = correctionCount ? ` / ${correctionCount} corrected gnome(s)` : "";
        const movement = summarizeGnomeMovement(row);
        const movementText = movement || `${text(deltaCount)} gnome delta(s)${text(correctionText)}`;
        return `<li>${text(row.task_id || "")}: ${text(row.title || "")}<br><span class="muted">plan ${text(planned)} → touched ${text(touched)} → ${text(commits)} → eval ${text(evalText)}${strictText}${problemText} → ${movementText}</span></li>`;
      }).join("")}</ul>`;
    }

    function renderEvolutionSuggestions(work) {
      const rows = (work.evolution_suggestions || []).slice(0, 4);
      if (!rows.length) return `<p class="muted">No graph-derived next-task suggestions for this session.</p>`;
      return `<ul class="mini-list">${rows.map(row => {
        return `<li><strong>${text(row.title || "")}</strong><br><span class="muted">${text(row.reason || "")} ${text(row.metric || "")}=${text(row.value)}</span></li>`;
      }).join("")}</ul>`;
    }

    function renderSessionWork(sessions) {
      const panel = document.getElementById("sessionWork");
      if (!sessions.length) {
        panel.innerHTML = `<div class="empty">No sessions match the current filter.</div>`;
        return;
      }
      panel.innerHTML = sessions.slice().reverse().slice(0, 12).map(session => {
        const work = session.work_summary || {};
        const trace = session.trace_quality || {};
        const transcripts = work.transcripts || {};
        const verification = work.task_verification || {};
        const manifest = work.task_manifest || {};
        const hasManifest = Object.keys(manifest).length > 0;
        const sourceFiles = (work.source_changed_files || []).length ? work.source_changed_files : (work.touched_source_files || []);
        const evidenceFiles = work.edited_files || [];
        const failedTools = work.failed_tools || [];
        const phaseText = Object.entries(transcripts.phase_counts || {}).map(([phase, count]) => `${phase} ${count}`).join(", ");
        return `<article class="item work-row">
          <div class="work-meta">
            <span class="pill ${healthClass(healthOf(session))}">${text(healthOf(session))}</span>
            <span class="pill soft">${text(trace.label || "unknown trace")}</span>
            <strong class="work-title">${text(session.id)}</strong>
            <p class="muted">${text(sessionSourceLine(session))}<br>${text(work.headline || "No detailed work signals captured")}</p>
          </div>
          <div>
            <div class="work-facts">
              <div class="fact"><strong>${text(session.tasks_succeeded || 0)}/${text(session.tasks_attempted || 0)}</strong>tasks</div>
              <div class="fact"><strong>${text(verification.verified_task_count || 0)}/${text(verification.task_count || 0)}</strong>verified</div>
              <div class="fact"><strong>${text(sourceFiles.length || 0)}</strong>source files</div>
              <div class="fact"><strong>${text(work.unlanded_source_task_count || 0)}</strong>unlanded source tasks</div>
              <div class="fact"><strong>${text(evidenceFiles.length || 0)}</strong>evidence edits</div>
              <div class="fact"><strong>${text(work.eval_count || 0)}</strong>evals</div>
              <div class="fact"><strong>${hasManifest ? text(manifest.assessment_present ? "yes" : "no") : "-"}</strong>assessment artifact</div>
              <div class="fact"><strong>${text(work.source_commit_count || 0)}</strong>source commits</div>
              <div class="fact"><strong>${text(failedTools.length || 0)}</strong>tool fails</div>
              <div class="fact"><strong>${text(work.decision_count || 0)}</strong>decisions</div>
              <div class="fact"><strong>${text(trace.trace_event_count || 0)}</strong>trace events</div>
            </div>
            <details class="work-details">
              <summary>Open audit evidence</summary>
              <div class="detail-grid">
                <div><strong>Source changes</strong>${listItems(sourceFiles, "No source changes recorded.")}</div>
                <div><strong>Source commits</strong>${sourceCommitItems(work)}</div>
                <div><strong>Bookkeeping commits</strong>${bookkeepingCommitItems(work)}</div>
                <div><strong>Task lineage</strong>${renderTaskLineage(work)}</div>
                <div><strong>Causal chains</strong>${renderCausalChains(work)}</div>
                <div><strong>Next-task suggestions</strong>${renderEvolutionSuggestions(work)}</div>
                <div><strong>Task decision evidence</strong>${renderTaskArtifacts(session, work)}</div>
                <div><strong>Agent transcripts</strong>${renderTranscriptList(session, work)}</div>
                <div><strong>State pipeline</strong>${renderStatePipeline(work)}</div>
                <div><strong>Validated</strong>${listItems(work.commands, "No command events recorded.")}</div>
                <div><strong>Read</strong>${listItems(work.read_files, "No file reads recorded.")}</div>
                <div><strong>Evidence/bookkeeping edits</strong>${listItems(evidenceFiles, "No evidence or bookkeeping edits recorded.")}</div>
                <div><strong>Failures</strong>${listItems(work.failed_commands, "No failed commands recorded.")}</div>
                <div><strong>Tool failures</strong>${listItems(work.failed_tools, "No failed tool calls recorded.")}</div>
              </div>
              <p class="muted">Transcript phases: ${text(phaseText || "no transcripts")}. ${text(trace.label || "unknown trace")} from ${text(trace.event_count || 0)} total event(s). Audit files: <a href="${text(session.audit_url)}">open session evidence</a>.</p>
            </details>
          </div>
        </article>`;
      }).join("");
    }

    function renderFeedbackLoop(sessions, agg) {
      const panel = document.getElementById("feedbackLoop");
      if (!sessions.length) {
        panel.innerHTML = `<div class="empty">No sessions match the current filter.</div>`;
        return;
      }
      const feedbackOnly = sessions.filter(session => session.trace_quality?.status === "feedback_only");
      const codeSessions = sessions.filter(session => (session.work_summary?.source_commit_count || 0) > 0);
      const explicitLineage = sessions.filter(session => (session.work_summary?.task_lineage || []).length > 0);
      const wrapupSource = sessions.filter(session =>
        (session.work_summary?.source_commits || []).some(commit => String(commit.subject || "").includes("session wrap-up"))
      );
      const bookkeepingOnly = sessions.filter(session =>
        (session.tasks_attempted || 0) > 0 && !(session.work_summary?.source_commit_count || 0)
      );
      const notes = [
        {
          show: true,
          kind: "Captured",
          className: "good",
          title: `${text(codeSessions.length)} of ${text(sessions.length)} visible sessions have source-changing commits`,
          detail: "Source file lists are derived from all matching session commits, then separated from journals, memory, plans, and .yoyo artifacts."
        },
        {
          show: true,
          kind: explicitLineage.length === sessions.length ? "Captured" : "Open",
          className: explicitLineage.length === sessions.length ? "good" : "warn",
          title: `${text(explicitLineage.length)} of ${text(sessions.length)} visible sessions have explicit task lineage`,
          detail: "New sessions should link task_id, touched files, commit SHAs, evaluator verdicts, and log-feedback gnome deltas directly in yoagent-state."
        },
        {
          show: feedbackOnly.length > 0,
          kind: "Open",
          className: "warn",
          title: `${text(feedbackOnly.length)} session(s) are feedback-only traces`,
          detail: "They have log-feedback evals but no task/tool trace events. New harness lifecycle events prevent future sessions from being completely empty, but older sessions stay historical."
        },
        {
          show: wrapupSource.length > 0,
          kind: "Open",
          className: "warn",
          title: `${text(wrapupSource.length)} session(s) landed source changes in wrap-up commits`,
          detail: "The code changed, but task-level attribution is weak. A stronger loop should attach task_id and commit_sha to state events when each task finishes."
        },
        {
          show: bookkeepingOnly.length > 0,
          kind: "Watch",
          className: "info",
          title: `${text(bookkeepingOnly.length)} session(s) completed tasks without source commits`,
          detail: "This can be legitimate verification work, but repeated bookkeeping-only sessions should trigger a planning-quality review."
        },
        {
          show: explicitLineage.length < sessions.length,
          kind: "Next",
          className: "info",
          title: "Backfill is historical only",
          detail: "Older audit sessions can still be inferred from commits, but only future runs can emit complete task lineage at execution time."
        }
      ].filter(note => note.show);
      panel.innerHTML = notes.map(note => `
        <article class="item">
          <span class="pill ${note.className}">${note.kind}</span>
          <strong>${note.title}</strong>
          <p class="muted">${note.detail}</p>
        </article>
      `).join("");
    }

    function renderSessions(sessions) {
      const body = document.getElementById("sessions");
      if (!sessions.length) {
        body.innerHTML = `<tr><td colspan="5" class="empty">No sessions match the current filter.</td></tr>`;
        return;
      }
      body.innerHTML = sessions.slice().reverse().map(session => {
        const evalData = session.latest_eval || {};
        const decision = session.latest_decision || {};
        const health = healthOf(session);
        const events = session.event_count || 0;
        const work = session.work_summary || {};
        const trace = session.trace_quality || {};
        const verification = work.task_verification || {};
        const verifiedText = verification.task_count ? `<br>verified ${text(verification.verified_task_count || 0)}/${text(verification.task_count || 0)}` : "";
        return `<tr>
          <td><strong>${text(session.id)}</strong><div class="muted">Day ${text(session.day)} at ${text(session.session_time)}<br>${text(session.ts)}<br>${text(sessionSourceLine(session))}${session.github_run_id ? `<br>run ${text(session.github_run_id)} attempt ${text(session.github_run_attempt || "-")}` : ""}</div></td>
          <td><span class="pill ${healthClass(health)}">${text(health)}</span><div class="muted">build ${text(session.build_ok)} / test ${text(session.test_ok)}<br>tasks ${text(session.tasks_succeeded)}/${text(session.tasks_attempted)}${verifiedText}<br>${text(work.headline)}</div></td>
          <td><span class="${decisionClass(decision)}">${text(decision.criterion || decision.decision || decision.decision_type)}</span><div class="muted">${text(decision.reason)}</div></td>
          <td><span class="pill soft">${text(trace.label || "unknown trace")}</span><div class="muted">${text(trace.trace_event_count || 0)} trace events / ${text(events)} total<br>eval ${text(evalData.status)} ${evalData.score === undefined ? "" : `score ${text(evalData.score)}`}</div></td>
          <td><a href="${text(session.audit_url)}">audit files</a><div class="muted">${text((session.blockers || []).length)} blockers / ${text((session.evals || []).length)} evals / ${text(work.source_commit_count || 0)} source commits / ${text(work.bookkeeping_commit_count || 0)} bookkeeping commits / ${text((session.patches || []).length)} state patches</div></td>
        </tr>`;
      }).join("");
    }

    function renderTaskLineage(work) {
      const rows = (work.task_lineage || []).slice(0, 6);
      if (!rows.length) return `<p class="muted">No explicit task lineage events yet.</p>`;
      return `<ul class="mini-list">${rows.map(task => {
        const commitCount = (task.commit_shas || []).length;
        const fileCount = (task.source_files || task.touched_files || []).length;
        const evalVerdict = task.eval && task.eval.verdict ? ` / eval ${task.eval.verdict}` : "";
        const deltaCount = Object.keys(task.gnome_deltas || {}).length;
        const deltaText = deltaCount ? ` / ${deltaCount} gnome delta(s)` : "";
        const correctionCount = Object.keys(task.gnome_corrections || {}).length;
        const correctionText = correctionCount ? ` / ${correctionCount} corrected gnome(s)` : "";
        const method = task.commit_linkage_method ? ` / ${task.commit_linkage_method}` : "";
        const strict = task.verification_status || (task.strict_success ? "strict_pass" : "");
        const strictClass = strict === "strict_pass" ? "good" : (strict === "strict_failed" ? "warn" : "muted");
        const problems = (task.verification_problems || []).slice(0, 3).join(", ");
        const strictText = strict ? ` / ${strict.replace("_", " ")}` : "";
        const problemText = problems ? ` / ${problems}` : "";
        const movement = summarizeGnomeMovement(task);
        const movementText = movement ? ` / ${movement}` : `${deltaText}${correctionText}`;
        return `<li><span class="${strictClass}">${text(task.task_id || "")} ${text(task.status || "-")}</span>: ${text(task.task_title || "")} (${text(fileCount)} files, ${text(commitCount)} commits${method}${evalVerdict}${strictText}${problemText}${movementText})</li>`;
      }).join("")}</ul>`;
    }

    function renderEvidence(sessions) {
      const items = [];
      sessions.slice().reverse().forEach(session => {
        (session.blockers || []).forEach(blocker => {
          items.push({ kind: "Blocker", className: "bad", session: session.id, title: blocker.reason, detail: blocker.patch_id || blocker.event_id });
        });
        (session.code_refs || []).forEach(ref => {
          items.push({ kind: "Code ref", className: "info", session: session.id, title: ref.commit || ref.patch_id || ref.artifact_path, detail: ref.event_type });
        });
        ((session.work_summary || {}).source_commits || []).slice(0, 3).forEach(commit => {
          const files = (commit.source_files || []).length;
          items.push({ kind: "Source commit", className: "good", session: session.id, title: `${commit.short_sha || ""} ${commit.subject || ""}`, detail: `${files} source files: ${(commit.source_files || []).slice(0, 3).join(", ")}` });
        });
        ((session.work_summary || {}).task_lineage || []).slice(0, 3).forEach(task => {
          const files = (task.source_files || task.touched_files || []).length;
          const commits = (task.commit_shas || []).length;
          const deltas = Object.keys(task.gnome_deltas || {}).length;
          const corrections = Object.keys(task.gnome_corrections || {}).length;
          const strictFailed = task.verification_status === "strict_failed";
          const className = strictFailed ? "warn" : (task.verification_status === "strict_pass" || deltas || corrections ? "good" : "info");
          const verification = task.verification_status ? ` / ${task.verification_status.replace("_", " ")}` : "";
          items.push({ kind: "Task link", className, session: session.id, title: `${task.task_id || ""} ${task.task_title || ""}`, detail: `${task.status || "-"}${verification} / ${files} files / ${commits} commits / ${deltas} gnome deltas / ${corrections} corrected gnomes` });
        });
        (session.evals || []).slice(-2).forEach(evalData => {
          items.push({ kind: "Eval", className: evalData.status === "passed" ? "good" : "warn", session: session.id, title: evalData.eval_id || evalData.suite || "evaluation", detail: `${evalData.suite || "-"} ${evalData.status || "-"} score ${evalData.score === undefined ? "-" : evalData.score}` });
        });
        (session.patches || []).slice(-2).forEach(patch => {
          items.push({ kind: "State patch", className: "warn", session: session.id, title: patch.patch_id || patch.intent, detail: `${patch.kind || "-"} risk ${patch.risk_level || "-"}` });
        });
        const work = session.work_summary || {};
        if (!(work.source_commits || []).length) {
          (work.bookkeeping_commits || []).slice(-1).forEach(commit => {
            items.push({ kind: "Bookkeeping", className: "info", session: session.id, title: `${commit.short_sha || ""} ${commit.subject || ""}`, detail: "no source commit in this session" });
          });
        }
      });
      const panel = document.getElementById("evidence");
      if (!items.length) {
        panel.innerHTML = `<div class="empty">No blockers, evals, commits, state patches, or code references yet.</div>`;
        return;
      }
      panel.innerHTML = items.slice(0, 24).map(item => `
        <article class="item">
          <span class="pill ${item.className}">${item.kind}</span>
          <strong>${text(item.title)}</strong>
          <div class="muted">${text(item.session)} / ${text(item.detail)}</div>
        </article>
      `).join("");
    }

    function render() {
      const data = state.data || { sessions: [], aggregate: {} };
      const filtered = (data.sessions || []).filter(matches);
      const visibleAgg = aggregateSessions(filtered, data.aggregate || {});
      renderHero(filtered, visibleAgg);
      renderSummary(visibleAgg);
      renderCharts(visibleAgg);
      renderGnomeHistory(filtered);
      renderSessionWork(filtered);
      renderFeedbackLoop(filtered, visibleAgg);
      renderSessions(filtered);
      renderEvidence(filtered);
    }

    fetch("data.json")
      .then(response => response.ok ? response.json() : Promise.reject(new Error("missing data.json")))
      .then(data => { state.data = data; render(); })
      .catch(error => {
        state.data = { sessions: [], aggregate: {}, error: String(error) };
        render();
      });

    document.getElementById("search").addEventListener("input", event => {
      state.query = event.target.value;
      render();
    });
    document.getElementById("status").addEventListener("change", event => {
      state.status = event.target.value;
      render();
    });
    document.getElementById("gnomeMetric").addEventListener("change", event => {
      state.selectedGnome = event.target.value;
      render();
    });
    document.getElementById("reset").addEventListener("click", () => {
      state.query = "";
      state.status = "all";
      document.getElementById("search").value = "";
      document.getElementById("status").value = "all";
      render();
    });
  </script>
</body>
</html>
"""


def build(audit_sessions: Path, output_dir: Path, repo_root: Path | None = None) -> dict[str, Any]:
    sessions = load_sessions(audit_sessions, repo_root or Path.cwd())
    gnome_history, gnome_numeric_keys = build_gnome_history(sessions)
    generated_at = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    data = {
        "schema_version": 2,
        "generated_at": generated_at,
        "source": str(audit_sessions),
        "aggregate": aggregate(sessions),
        "gnome_history": gnome_history,
        "gnome_numeric_keys": gnome_numeric_keys,
        "sessions": sessions,
    }
    claims = build_claims_projection(sessions, generated_at, audit_sessions)
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "data.json").write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (output_dir / "claims.json").write_text(json.dumps(claims, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (output_dir / "index.html").write_text(HTML, encoding="utf-8")
    return data


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--audit-sessions", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--repo-root", default=Path.cwd(), type=Path)
    parser.add_argument("--copy-to", type=Path, help="Optional second output directory.")
    args = parser.parse_args()

    data = build(args.audit_sessions, args.output_dir, args.repo_root)
    if args.copy_to:
        if args.copy_to.exists():
            shutil.rmtree(args.copy_to)
        shutil.copytree(args.output_dir, args.copy_to)
    print(
        f"Evolution dashboard built: {args.output_dir / 'index.html'} "
        f"({len(data['sessions'])} sessions)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
