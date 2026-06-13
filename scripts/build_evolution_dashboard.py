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
from task_manifest import normalize_file_list, parse_task as parse_task_file


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
NORMAL_MODEL_COMPLETION_STATUSES = {"completed", "success", "ok", "stopped_after_completion_file"}
ASSESSMENT_WRITE_RE = re.compile(r"(?:write|Auto-approved: write:|# Assessment\b).*session_plan/assessment\.md|^\s*\+\s*# Assessment\b")
ASSESSMENT_SYNTHESIS_RE = re.compile(
    r"(enough data to write|prepare the assessment|write the assessment|finalize .*assessment|assessment complete)",
    re.IGNORECASE,
)
ASSESSMENT_NOTE_RE = re.compile(
    r"(key findings|interesting findings|tests pass|tests failed|state tail|state failures|ci run|failure|crash|cache)",
    re.IGNORECASE,
)
SEED_TASK_CONTRADICTION_RE = re.compile(
    r"\bseed(?:ed)? task[\w.-]*\b.*\b(factual error|assessment clearly shows|contradict\w*)\b"
    r"|\b(factual error|assessment clearly shows|contradict\w*)\b.*\bseed(?:ed)? task[\w.-]*\b",
    re.IGNORECASE,
)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def log_feedback_metrics(session_dir: Path) -> dict[str, Any]:
    feedback = load_json(session_dir / "log_feedback.json")
    metrics = feedback.get("metrics") if isinstance(feedback.get("metrics"), dict) else {}
    scalar_metrics = {
        str(key): value
        for key, value in metrics.items()
        if value is None or isinstance(value, (bool, int, float))
    }
    if (
        int(scalar_metrics.get("task_seed_contradiction_count") or 0) == 0
        and log_feedback_has_seed_task_contradiction(feedback)
    ):
        scalar_metrics["task_seed_contradiction_count"] = 1
    failure_fingerprints = metrics.get("failure_fingerprints")
    if isinstance(failure_fingerprints, list):
        active = [
            item
            for item in failure_fingerprints
            if isinstance(item, dict) and not benign_log_failure_text(str(item.get("fingerprint") or ""))
        ]
        scalar_metrics["distinct_failure_count"] = len(active)
        scalar_metrics["failure_count"] = sum(
            int(item.get("count") or 0)
            for item in active
            if isinstance(item.get("count"), (int, float)) and not isinstance(item.get("count"), bool)
        )
    return scalar_metrics


def benign_log_failure_text(text: str) -> bool:
    lower = str(text or "").strip().lower()
    return lower.startswith(
        (
            "**edit ",
            "**edit:",
            "**step ",
            "**step:",
            "**check ",
            "**check:",
            "**verification ",
            "**verification:",
            "**change ",
            "**change:",
            "**implementation ",
            "**implementation:",
            "**result ",
            "**result:",
        )
    )


def seed_task_contradiction_text(text: str) -> bool:
    return bool(SEED_TASK_CONTRADICTION_RE.search(str(text or "")))


def log_feedback_has_seed_task_contradiction(value: Any) -> bool:
    if isinstance(value, str):
        return seed_task_contradiction_text(value)
    if isinstance(value, dict):
        return any(log_feedback_has_seed_task_contradiction(item) for item in value.values())
    if isinstance(value, list):
        return any(log_feedback_has_seed_task_contradiction(item) for item in value)
    return False


def log_feedback_top_lessons(session_dir: Path) -> list[dict[str, Any]]:
    feedback = load_json(session_dir / "log_feedback.json")
    lessons = feedback.get("top_lessons")
    if not isinstance(lessons, list):
        return []
    rows: list[dict[str, Any]] = []
    for lesson in lessons[:6]:
        if not isinstance(lesson, dict):
            continue
        if str(lesson.get("kind") or "") in {"failure", "recurring_failure"} and benign_log_failure_text(
            str(lesson.get("fingerprint") or "")
        ):
            continue
        rows.append(
            {
                "kind": str(lesson.get("kind") or ""),
                "fingerprint": str(lesson.get("fingerprint") or ""),
                "action": str(lesson.get("action") or ""),
                "count": lesson.get("count"),
            }
        )
    return rows


def lesson_key(lesson: dict[str, Any]) -> tuple[str, str]:
    return (
        str(lesson.get("kind") or "").strip().lower(),
        str(lesson.get("fingerprint") or "").strip().lower(),
    )


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


def event_run_id(event: dict[str, Any], payload: dict[str, Any]) -> str | None:
    top_level = event.get("run_id")
    if isinstance(top_level, str) and top_level:
        return top_level
    raw_payload = event.get("payload") if isinstance(event.get("payload"), dict) else {}
    raw_meta = raw_payload.get("_yoyo") if isinstance(raw_payload.get("_yoyo"), dict) else {}
    raw_meta_run = raw_meta.get("run_id")
    if isinstance(raw_meta_run, str) and raw_meta_run:
        return raw_meta_run
    direct = payload.get("run_id")
    if isinstance(direct, str) and direct:
        return direct
    meta = payload.get("_yoyo") if isinstance(payload.get("_yoyo"), dict) else {}
    meta_run = meta.get("run_id")
    if isinstance(meta_run, str) and meta_run:
        return meta_run
    return None


def payload_int(payload: dict[str, Any], *keys: str) -> int | None:
    for key in keys:
        value = payload.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, int):
            return value
        if isinstance(value, str):
            try:
                return int(value.replace(",", ""))
            except ValueError:
                continue
    return None


def deepseek_model_payload(payload: dict[str, Any]) -> bool:
    model = payload.get("model")
    provider = payload.get("provider")
    deepseek_native = payload.get("deepseek_native") is True
    deepseek_model = isinstance(model, str) and model.startswith("deepseek")
    deepseek_provider = provider == "deepseek"
    return deepseek_native or deepseek_model or deepseek_provider


def cache_metrics_expected_from_completion(payload: dict[str, Any]) -> bool:
    if not deepseek_model_payload(payload):
        return False
    input_tokens = payload_int(payload, "input_tokens")
    cache_read_tokens = payload_int(payload, "cache_read_tokens", "prompt_cache_hit_tokens", "cache_hit_tokens")
    return bool((input_tokens or 0) > 0 or (cache_read_tokens or 0) > 0)


def abnormal_model_completion_status(payload: dict[str, Any]) -> str | None:
    status = str(payload.get("status") or "").strip()
    if status and status not in NORMAL_MODEL_COMPLETION_STATUSES:
        return status
    if status == "stopped_after_completion_file":
        return None
    if payload.get("error") or payload.get("error_detail"):
        return status or "error"
    return None


def compact_list(values: list[str], limit: int) -> list[str]:
    out: list[str] = []
    for value in values:
        text = " ".join(str(value).split())
        if text and text not in out:
            out.append(text)
        if len(out) >= limit:
            break
    return out


def compact_count(values: list[str]) -> int:
    out: set[str] = set()
    for value in values:
        text = " ".join(str(value).split())
        if text:
            out.add(text)
    return len(out)


def normalized_set(values: list[str]) -> set[str]:
    return {text for value in values if (text := " ".join(str(value).split()))}


def unique_delta_count(left: list[str], right: list[str]) -> int:
    return len(normalized_set(left) - normalized_set(right))


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
        "failed_tool_summary": failed_tool_pattern_summary(failed_tools),
        "command_count": compact_count(commands),
        "failed_command_count": compact_count(failed_commands),
        "failed_tool_count": compact_count(failed_tools),
        "_commands_all": commands,
        "_failed_commands_all": failed_commands,
        "_failed_tools_all": failed_tools,
        "read_files": compact_list(read_files, 12),
        "edited_files": compact_list(edited_files, 12),
        "read_file_count": compact_count(read_files),
        "edited_file_count": compact_count(edited_files),
        "_read_files_all": read_files,
        "_edited_files_all": edited_files,
    }


def failed_tool_category(label: str) -> str:
    text = str(label or "").strip().lower()
    if text.startswith("search "):
        if "search error:" in text:
            if "unmatched" in text or "invalid content" in text:
                return "search_regex_error"
            if "binary file matches" in text:
                return "search_binary_match"
            return "search_error"
        return "search_tool_error"
    if text.startswith("read "):
        if "no such file" in text or "cannot access" in text:
            return "missing_file_read"
        return "read_error"
    if text.startswith("edit "):
        if "old_text" in text or "did not match" in text or "matches " in text:
            return "edit_context_mismatch"
        return "edit_error"
    if text.startswith("write "):
        return "write_error"
    if text.startswith("bash "):
        return "bash_tool_error"
    return "tool_error"


def failed_tool_pattern_summary(failed_tools: list[str]) -> dict[str, Any]:
    category_counts: dict[str, int] = {}
    examples_by_category: dict[str, list[str]] = {}
    unique_labels: list[str] = []
    seen: set[str] = set()
    for raw in failed_tools:
        label = evidence_text(raw, 500)
        if not label:
            continue
        key = " ".join(label.split())
        if key in seen:
            continue
        seen.add(key)
        unique_labels.append(label)
    for label in unique_labels:
        category = failed_tool_category(label)
        category_counts[category] = category_counts.get(category, 0) + 1
        examples_by_category.setdefault(category, []).append(label)
    top_categories = [
        {
            "category": category,
            "count": count,
            "examples": compact_list(examples_by_category.get(category, []), 3),
        }
        for category, count in sorted(category_counts.items(), key=lambda item: (-item[1], item[0]))
    ]
    return {
        "total_count": len(unique_labels),
        "category_counts": dict(sorted(category_counts.items())),
        "top_categories": top_categories[:8],
    }


def source_file(path: str) -> bool:
    if not path:
        return False
    if path.endswith(".bak"):
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


def protected_file_revert(reason: str) -> bool:
    return "modified protected files" in str(reason or "").lower()


def task_scope_mismatch(row: dict[str, Any]) -> bool:
    problems = row.get("problems") if isinstance(row.get("problems"), list) else []
    return "no_planned_file_overlap" in problems


def task_protected_revert(row: dict[str, Any]) -> bool:
    problems = row.get("problems") if isinstance(row.get("problems"), list) else []
    return bool(row.get("protected_revert") or "modified_protected_files" in problems)


def task_unlanded_source_problem(row: dict[str, Any]) -> bool:
    problems = row.get("problems") if isinstance(row.get("problems"), list) else []
    return (
        ("source_edits_not_landed" in problems or "no_landed_source_commit" in problems)
        and not task_protected_revert(row)
        and not task_scope_mismatch(row)
    )


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


def assessment_transcript_summary(session_dir: Path) -> dict[str, Any]:
    path = session_dir / "transcripts" / "assess.log"
    if not path.is_file():
        return {"present": False, "classification": "missing", "line_count": 0, "evidence_phrases": []}
    try:
        text_value = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return {"present": True, "classification": "unreadable", "line_count": 0, "evidence_phrases": []}
    lines = text_value.splitlines()
    write_evidence = False
    synthesis_evidence = False
    note_evidence = False
    write_phrases: list[str] = []
    synthesis_phrases: list[str] = []
    note_phrases: list[str] = []
    for line in lines:
        clean = line.strip()
        if not clean:
            continue
        if ASSESSMENT_WRITE_RE.search(clean):
            write_evidence = True
            if len(write_phrases) < 3:
                write_phrases.append(clean[:220])
        elif ASSESSMENT_SYNTHESIS_RE.search(clean):
            synthesis_evidence = True
            if len(synthesis_phrases) < 3:
                synthesis_phrases.append(clean[:220])
        elif ASSESSMENT_NOTE_RE.search(clean):
            note_evidence = True
            if len(note_phrases) < 3:
                note_phrases.append(clean[:220])
    if write_evidence:
        classification = "write_evidence"
    elif synthesis_evidence:
        classification = "synthesis_reached"
    elif note_evidence:
        classification = "audit_notes"
    elif lines:
        classification = "transcript_present"
    else:
        classification = "empty_transcript"
    return {
        "present": True,
        "classification": classification,
        "line_count": len(lines),
        "write_evidence": write_evidence,
        "synthesis_evidence": synthesis_evidence,
        "audit_note_evidence": note_evidence,
        "evidence_phrases": (write_phrases + synthesis_phrases + note_phrases)[:5],
    }


def transcript_summary(session_dir: Path) -> dict[str, Any]:
    transcript_dir = session_dir / "transcripts"
    files = sorted(transcript_dir.glob("*.log")) if transcript_dir.is_dir() else []
    phase_counts = {
        "assess": 0,
        "plan": 0,
        "task": 0,
        "build_fix": 0,
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
        elif name.startswith("bfix_"):
            phase = "build_fix"
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
        "files": transcript_rows,
        "assessment": assessment_transcript_summary(session_dir),
    }


def state_pipeline_summary(session_dir: Path) -> dict[str, Any]:
    replay = load_json(session_dir / "state_replay.json")
    merge = load_json(session_dir / "state" / "merge_state_delta.json")
    append_log = session_dir / "state" / "append_state_event.log"
    append_lines = 0
    append_problem_lines = 0
    terminal_attempts = 0
    terminal_completed_model_calls = 0
    terminal_completed_runs = 0
    terminal_noop_attempts = 0
    terminal_fallback_scans = 0
    terminal_examples: list[dict[str, Any]] = []
    if append_log.is_file():
        for line in append_log.read_text(encoding="utf-8", errors="replace").splitlines():
            text = line.strip()
            if not text:
                continue
            append_lines += 1
            lower = text.lower()
            if "failed" in lower or "warning" in lower:
                append_problem_lines += 1
            try:
                row = json.loads(text)
            except json.JSONDecodeError:
                continue
            if not isinstance(row, dict) or (
                "completed_model_calls" not in row and "completed_runs" not in row
            ):
                continue
            terminal_attempts += 1
            completed_model_calls = row.get("completed_model_calls") if isinstance(row.get("completed_model_calls"), list) else []
            completed_runs = row.get("completed_runs") if isinstance(row.get("completed_runs"), list) else []
            terminal_completed_model_calls += len(completed_model_calls)
            terminal_completed_runs += len(completed_runs)
            if not completed_model_calls and not completed_runs:
                terminal_noop_attempts += 1
            diagnostics = row.get("diagnostics") if isinstance(row.get("diagnostics"), dict) else {}
            if diagnostics.get("scope") == "fallback_after_line":
                terminal_fallback_scans += 1
            if len(terminal_examples) < 4:
                terminal_examples.append(
                    {
                        "completed_model_call_count": len(completed_model_calls),
                        "completed_run_count": len(completed_runs),
                        "scope": diagnostics.get("scope"),
                        "open_model_count": diagnostics.get("open_model_count"),
                        "open_run_count": diagnostics.get("open_run_count"),
                    }
                )
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
        "terminal_closure_attempts": terminal_attempts,
        "terminal_closure_completed_model_calls": terminal_completed_model_calls,
        "terminal_closure_completed_runs": terminal_completed_runs,
        "terminal_closure_noop_attempts": terminal_noop_attempts,
        "terminal_closure_fallback_scans": terminal_fallback_scans,
        "terminal_closure_examples": terminal_examples,
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
        obsolete_file = task_dir / "obsolete.md"
        outcome = load_json(task_dir / "outcome.json")
        rows.append(
            {
                "task_id": task_id,
                "has_task_file": task_file.is_file(),
                "has_obsolete_note": obsolete_file.is_file(),
                "obsolete_note_path": f"tasks/{task_id}/obsolete.md" if obsolete_file.is_file() else None,
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


def eval_timed_out_after_passing_verdict(eval_data: dict[str, Any]) -> bool:
    return eval_timed_out_after_verdict(eval_data) and (
        explicit_pass(eval_data.get("status")) or explicit_pass(eval_data.get("verdict"))
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


def eval_attempt_summaries(evals: list[dict[str, Any]]) -> list[dict[str, Any]]:
    summaries: list[dict[str, Any]] = []
    for index, eval_data in enumerate(evals, start=1):
        if not isinstance(eval_data, dict):
            continue
        status = str(eval_data.get("status") or "").strip()
        verdict = str(eval_data.get("verdict") or "").strip()
        normalized_verdict = verdict.removeprefix("Verdict:").strip()
        reason = str(eval_data.get("reason") or "").strip()
        attempt = eval_data.get("attempt")
        if attempt is None:
            attempt = index
        row: dict[str, Any] = {
            "attempt": attempt,
            "status": status or None,
            "exit_code": eval_data.get("exit_code"),
            "verdict": normalized_verdict or verdict or None,
            "reason": reason or None,
            "transcript_path": eval_data.get("transcript_path"),
            "verdict_file": eval_data.get("verdict_file"),
            "timed_out_after_verdict": eval_timed_out_after_verdict(eval_data),
        }
        summaries.append({key: value for key, value in row.items() if value is not None})
    return summaries


def eval_statuses_from_lineage(lineage_eval: Any) -> list[str]:
    if not isinstance(lineage_eval, dict):
        return []
    status = str(lineage_eval.get("status") or "").strip().lower()
    verdict = str(lineage_eval.get("verdict") or "").removeprefix("Verdict:").strip()
    if status:
        return [status]
    if explicit_pass(verdict):
        return ["pass"]
    if explicit_fail(verdict):
        return ["fail"]
    return [verdict.lower()] if verdict else []


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
            for field in (
                "status",
                "revert_reason",
                "source_files",
                "touched_files",
                "commit_shas",
                "has_obsolete_note",
                "obsolete_note_path",
            ):
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
        source_candidates = [
            str(path)
            for path in (lineage.get("source_files") or artifact.get("source_files") or [])
            if path
        ]
        touched = [
            str(path)
            for path in (
                lineage.get("touched_files")
                or artifact.get("touched_files")
                or source_candidates
                or []
            )
            if path
        ]
        source_touched = [path for path in compact_list(source_candidates + touched, 16) if source_file(path)]
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
        eval_attempts = eval_attempt_summaries(artifact_evals)
        timeout_with_verdict = any(
            isinstance(eval_data, dict) and eval_timed_out_after_verdict(eval_data)
            for eval_data in artifact_evals
        )
        timeout_with_passing_verdict = any(
            isinstance(eval_data, dict) and eval_timed_out_after_passing_verdict(eval_data)
            for eval_data in artifact_evals
        )
        artifact_eval_statuses = artifact.get("eval_statuses") or []
        lineage_eval_statuses = eval_statuses_from_lineage(lineage.get("eval"))
        attempt_eval_statuses = [
            str(attempt.get("status"))
            for attempt in eval_attempts
            if isinstance(attempt, dict) and attempt.get("status")
        ]
        eval_statuses = artifact_eval_statuses or attempt_eval_statuses or lineage_eval_statuses
        eval_evidence_source = (
            "task_artifact"
            if artifact_eval_statuses or eval_attempts
            else "state_lineage"
            if lineage_eval_statuses
            else None
        )
        verified = eval_passed(artifact_evals, lineage.get("eval"))
        outcome_status = str(artifact.get("status") or lineage.get("status") or "")
        revert_reason = str(artifact.get("revert_reason") or lineage.get("revert_reason") or "").strip()
        api_error = "api error" in revert_reason.lower()
        protected_revert = protected_file_revert(revert_reason)
        obsolete = bool(artifact.get("has_obsolete_note") or lineage.get("has_obsolete_note"))
        problems: list[str] = []
        if not planned:
            problems.append("missing_planned_files")
        if not touched:
            problems.append("no_touched_files")
        if planned and touched and not overlap:
            problems.append("no_planned_file_overlap")
        if timeout_with_verdict:
            problems.append("evaluator_timed_out_after_verdict")
        if timeout_with_passing_verdict:
            problems.append("timed_out_passing_verdict")
        elif not verified and not obsolete:
            problems.append("no_passing_verifier")
        if obsolete:
            problems.append("task_marked_obsolete")
        if api_error:
            problems.append("implementation_api_error")
        if protected_revert:
            problems.append("modified_protected_files")
        if outcome_status == "reverted" and not touched and not obsolete and not api_error and not protected_revert:
            problems.append("no_edit_revert")
        if source_touched and not landed_commits:
            problems.append("source_edits_not_landed")
        if source_touched and outcome_status == "completed" and not landed_commits:
            problems.append("no_landed_source_commit")
        rows.append(
            {
                "task_id": task_id,
                "title": task.get("title") or lineage.get("task_title"),
                "planned_files": planned,
                "touched_files": touched,
                "source_touched_files": source_touched,
                "overlap": overlap,
                "verified": verified,
                "outcome_status": outcome_status or None,
                "revert_reason": revert_reason or None,
                "api_error": api_error,
                "protected_revert": protected_revert,
                "landed_commit_shas": landed_commits,
                "eval_statuses": eval_statuses,
                "eval_evidence_source": eval_evidence_source,
                "eval_attempt_count": len(eval_attempts),
                "eval_attempts": eval_attempts,
                "latest_eval_attempt": eval_attempts[-1] if eval_attempts else None,
                "problems": problems,
                "obsolete": obsolete,
                "obsolete_note_path": artifact.get("obsolete_note_path") or lineage.get("obsolete_note_path"),
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


def classify_task_state(row: dict[str, Any], attempted: bool | None = None) -> str:
    problems = row.get("problems") if isinstance(row.get("problems"), list) else []
    outcome_status = str(row.get("outcome_status") or "").strip().lower()
    if row.get("strict_success"):
        return "verified_landed"
    if row.get("obsolete") or "task_marked_obsolete" in problems:
        return "obsolete_already_satisfied"
    if attempted is False:
        return "not_attempted"
    if outcome_status == "reverted":
        if row.get("api_error") or "implementation_api_error" in problems:
            return "reverted_api_error"
        if row.get("protected_revert") or "modified_protected_files" in problems:
            return "reverted_protected_file_edits"
        if "no_edit_revert" in problems:
            return "reverted_no_edit"
        if "source_edits_not_landed" in problems or "no_landed_source_commit" in problems:
            return "reverted_unlanded_source_edits"
        if "no_touched_files" in problems:
            return "reverted_no_git_visible_changes"
        if "no_planned_file_overlap" in problems:
            return "reverted_scope_mismatch"
        if "timed_out_passing_verdict" in problems:
            return "reverted_verifier_timed_out_after_pass"
        if "no_passing_verifier" in problems:
            return "reverted_unverified"
        return "reverted"
    if "source_edits_not_landed" in problems or "no_landed_source_commit" in problems:
        return "unlanded_source_edits"
    if "no_touched_files" in problems:
        return "no_git_visible_changes"
    if "no_planned_file_overlap" in problems:
        return "scope_mismatch"
    if "evaluator_timed_out_after_verdict" in problems:
        if "timed_out_passing_verdict" in problems:
            return "verifier_timed_out_after_pass"
        return "verifier_timed_out_after_verdict"
    if "no_passing_verifier" in problems:
        return "verifier_unproven"
    return "unknown"


def task_number_from_id(task_id: str) -> str | None:
    match = re.search(r"(\d+)$", str(task_id or ""))
    if not match:
        return None
    try:
        return str(int(match.group(1)))
    except ValueError:
        return None


def task_transcript_refs(task_id: str, transcript_data: dict[str, Any]) -> dict[str, list[str]]:
    task_number = task_number_from_id(task_id)
    if not task_number:
        return {"implementation": [], "eval": [], "fix": [], "build_fix": [], "all": []}
    padded = f"{int(task_number):02d}"
    patterns = {
        "implementation": [
            re.compile(rf"^task_{re.escape(padded)}_attempt\d+\.log$"),
            re.compile(rf"^task_{re.escape(task_number)}_attempt\d+\.log$"),
        ],
        "eval": [re.compile(rf"^eval_task{re.escape(task_number)}_attempt\d+\.log$")],
        "fix": [re.compile(rf"^fix_task{re.escape(task_number)}_attempt\d+\.log$")],
        "build_fix": [re.compile(rf"^bfix_task{re.escape(task_number)}_attempt\d+\.log$")],
    }
    refs: dict[str, list[str]] = {key: [] for key in patterns}
    for transcript in transcript_data.get("files") if isinstance(transcript_data.get("files"), list) else []:
        if not isinstance(transcript, dict):
            continue
        name = str(transcript.get("name") or "")
        path = str(transcript.get("path") or "")
        if not name or not path:
            continue
        for key, matchers in patterns.items():
            if any(pattern.match(name) for pattern in matchers):
                refs[key].append(path)
    refs["all"] = compact_list(
        refs["implementation"] + refs["eval"] + refs["fix"] + refs["build_fix"],
        16,
    )
    return refs


def structured_task_states(
    task_manifest: dict[str, Any],
    task_artifacts: list[dict[str, Any]],
    task_lineage: list[dict[str, Any]],
    task_verification: dict[str, Any],
    transcript_data: dict[str, Any] | None = None,
) -> dict[str, Any]:
    manifest_tasks = {
        str(row.get("task_id")): row
        for row in (task_manifest.get("tasks") if isinstance(task_manifest.get("tasks"), list) else [])
        if isinstance(row, dict) and row.get("task_id")
    }
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
    rows = task_verification.get("rows") if isinstance(task_verification.get("rows"), list) else []
    states: list[dict[str, Any]] = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        task_id = str(row.get("task_id") or "")
        if not task_id:
            continue
        manifest_task = manifest_tasks.get(task_id, {})
        artifact = artifacts_by_id.get(task_id, {})
        lineage = lineage_by_id.get(task_id, {})
        problems = row.get("problems") if isinstance(row.get("problems"), list) else []
        latest_eval = row.get("latest_eval_attempt") if isinstance(row.get("latest_eval_attempt"), dict) else {}
        transcript_refs = task_transcript_refs(task_id, transcript_data or {})
        implementation_attempt_count = max(
            int(artifact.get("attempt_count") or 0),
            len(artifact.get("attempts") or []),
            len(transcript_refs.get("implementation") or []),
        )
        attempted = bool(
            lineage.get("status")
            or row.get("outcome_status")
            or row.get("touched_files")
            or row.get("eval_attempt_count")
            or implementation_attempt_count
            or transcript_refs.get("eval")
            or transcript_refs.get("fix")
            or transcript_refs.get("build_fix")
        )
        evidence_sources = ["task_manifest"]
        if artifact:
            evidence_sources.append("task_artifacts")
        if lineage:
            evidence_sources.append("state_lineage")
        if row.get("eval_attempt_count"):
            evidence_sources.append("eval_attempts")
        if transcript_refs.get("all"):
            evidence_sources.append("transcripts")
        if row.get("landed_commit_shas"):
            evidence_sources.append("commits")
        failure_reasons = list(problems)
        if row.get("revert_reason"):
            failure_reasons.append(str(row.get("revert_reason")))
        states.append(
            {
                "task_id": task_id,
                "title": row.get("title") or manifest_task.get("title") or lineage.get("task_title"),
                "state": classify_task_state(row, attempted),
                "origin": manifest_task.get("origin"),
                "attempted": attempted,
                "planned_files": row.get("planned_files") or [],
                "touched_files": row.get("touched_files") or [],
                "source_touched_files": row.get("source_touched_files") or [],
                "implementation_status": row.get("outcome_status"),
                "strict_success": bool(row.get("strict_success")),
                "verified": bool(row.get("verified")),
                "overlap": bool(row.get("overlap")),
                "landed_commit_shas": row.get("landed_commit_shas") or [],
                "reverted": row.get("outcome_status") == "reverted" or bool(row.get("revert_reason")),
                "revert_reason": row.get("revert_reason"),
                "api_error": bool(row.get("api_error")),
                "protected_revert": bool(row.get("protected_revert")),
                "obsolete": bool(row.get("obsolete")),
                "obsolete_note_path": row.get("obsolete_note_path"),
                "eval_attempt_count": row.get("eval_attempt_count") or 0,
                "latest_eval_attempt": latest_eval or None,
                "implementation_attempt_count": implementation_attempt_count,
                "implementation_transcripts": transcript_refs.get("implementation") or [],
                "eval_transcripts": transcript_refs.get("eval") or [],
                "fix_transcripts": transcript_refs.get("fix") or [],
                "build_fix_transcripts": transcript_refs.get("build_fix") or [],
                "transcript_paths": transcript_refs.get("all") or [],
                "task_artifact_path": manifest_task.get("artifact_path"),
                "evidence_sources": evidence_sources,
                "failure_reasons": failure_reasons,
            }
        )
    state_counts: dict[str, int] = {}
    for state in states:
        key = str(state.get("state") or "unknown")
        state_counts[key] = state_counts.get(key, 0) + 1
    return {
        "schema_version": 1,
        "task_count": len(states),
        "strict_success_count": sum(1 for state in states if state.get("strict_success")),
        "unverified_count": sum(1 for state in states if not state.get("strict_success")),
        "state_counts": dict(sorted(state_counts.items())),
        "tasks": states,
    }


def refresh_task_state_counts(task_states: dict[str, Any]) -> None:
    rows = task_states.get("tasks") if isinstance(task_states.get("tasks"), list) else []
    state_counts: dict[str, int] = {}
    for row in rows:
        if not isinstance(row, dict):
            continue
        key = str(row.get("state") or "unknown")
        state_counts[key] = state_counts.get(key, 0) + 1
    task_states["state_counts"] = dict(sorted(state_counts.items()))
    task_states["task_count"] = len([row for row in rows if isinstance(row, dict)])
    task_states["strict_success_count"] = sum(
        1 for row in rows if isinstance(row, dict) and row.get("strict_success")
    )
    task_states["unverified_count"] = sum(
        1 for row in rows if isinstance(row, dict) and not row.get("strict_success")
    )


def annotate_seed_contradicted_task_states(work: dict[str, Any], count: int) -> None:
    if count <= 0:
        return
    task_states = work.get("task_states") if isinstance(work.get("task_states"), dict) else {}
    rows = task_states.get("tasks") if isinstance(task_states.get("tasks"), list) else []
    remaining = count
    preferred_states = {
        "reverted_no_edit",
        "reverted_no_git_visible_changes",
        "reverted_unverified",
        "no_git_visible_changes",
    }
    applied = 0
    for prefer_seed_origin in (True, False):
        for row in rows:
            if remaining <= 0:
                break
            if not isinstance(row, dict) or row.get("seed_contradicted"):
                continue
            if prefer_seed_origin and row.get("origin") != "harness-seed":
                continue
            if str(row.get("state") or "") not in preferred_states:
                continue
            row["state"] = "reverted_seed_contradicted" if row.get("reverted") else "seed_contradicted"
            row["seed_contradicted"] = True
            evidence_sources = row.get("evidence_sources") if isinstance(row.get("evidence_sources"), list) else []
            if "log_feedback" not in evidence_sources:
                row["evidence_sources"] = evidence_sources + ["log_feedback"]
            failure_reasons = row.get("failure_reasons") if isinstance(row.get("failure_reasons"), list) else []
            if "seed_task_contradicted_by_assessment" not in failure_reasons:
                row["failure_reasons"] = failure_reasons + ["seed_task_contradicted_by_assessment"]
            remaining -= 1
            applied += 1
        if remaining <= 0:
            break
    refresh_task_state_counts(task_states)
    if applied:
        label = f"{applied} seeded task(s) contradicted by assessment"
        labels = work.get("labels") if isinstance(work.get("labels"), list) else []
        if label not in labels:
            insert_at = 1 if labels else 0
            labels = labels[:insert_at] + [label] + labels[insert_at:]
            work["labels"] = labels
            work["headline"] = "; ".join(str(item) for item in labels[:4])


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
                "files": normalize_file_list(task.get("files") or parsed_task.get("files") or []),
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


def assessment_artifact_state(
    session_dir: Path,
    task_manifest: dict[str, Any],
    transcript_data: dict[str, Any],
) -> dict[str, Any]:
    artifacts = task_manifest.get("artifacts") if isinstance(task_manifest.get("artifacts"), dict) else {}
    assessment_file_present = (session_dir / "tasks" / "assessment.md").is_file()
    assessment_missing_file_present = (session_dir / "tasks" / "assessment_missing.md").is_file()
    artifact_present = bool(task_manifest.get("assessment_present") or assessment_file_present)
    diagnostic_present = bool(
        task_manifest.get("assessment_missing_present")
        or artifacts.get("assessment_missing")
        or assessment_missing_file_present
    )
    transcript_present = bool((transcript_data.get("phase_counts") or {}).get("assess"))
    assessment_transcript = (
        transcript_data.get("assessment")
        if isinstance(transcript_data.get("assessment"), dict)
        else {}
    )
    transcript_classification = str(assessment_transcript.get("classification") or "")
    manifest_present = bool(task_manifest)
    if artifact_present:
        classification = "assessment_present"
        detail = "Assessment artifact is present."
    elif diagnostic_present:
        classification = "missing_with_diagnostic"
        detail = "Assessment artifact is missing, and assessment_missing.md explains the missing output."
    elif transcript_present and transcript_classification == "write_evidence":
        classification = "missing_written_not_preserved"
        detail = (
            "Assessment transcript shows session_plan/assessment.md was written, but neither "
            "assessment.md nor assessment_missing.md was preserved in task artifacts."
        )
    elif transcript_present and transcript_classification == "synthesis_reached":
        classification = "missing_synthesis_only"
        detail = (
            "Assessment transcript reached assessment synthesis, but no assessment artifact "
            "or missing-assessment diagnostic was preserved."
        )
    elif transcript_present and transcript_classification == "audit_notes":
        classification = "missing_recoverable_transcript"
        detail = (
            "Assessment transcript contains useful audit notes, but no structured assessment "
            "artifact or missing-assessment diagnostic was preserved."
        )
    elif transcript_present:
        classification = "missing_transcript_only"
        detail = (
            "Assessment transcript exists, but neither assessment.md nor assessment_missing.md "
            "was preserved."
        )
    elif manifest_present:
        classification = "missing_manifest_only"
        detail = "Task manifest exists, but no assessment artifact, diagnostic, or transcript was preserved."
    else:
        classification = "missing_no_evidence"
        detail = "No assessment artifact, diagnostic, manifest, or transcript was found."
    return {
        "classification": classification,
        "detail": detail,
        "artifact_present": artifact_present,
        "transcript_present": transcript_present,
        "diagnostic_present": diagnostic_present,
        "manifest_present": manifest_present,
        "transcript_classification": transcript_classification or None,
        "transcript_summary": assessment_transcript if assessment_transcript else None,
        "assessment_path": "tasks/assessment.md" if assessment_file_present or artifacts.get("assessment") else None,
        "assessment_missing_path": "tasks/assessment_missing.md"
        if assessment_missing_file_present or artifacts.get("assessment_missing")
        else None,
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
    cache_hit_tokens = 0
    cache_miss_tokens = 0
    deepseek_model_call_started = 0
    deepseek_model_call_completed = 0
    deepseek_model_call_abnormal_completed = 0
    run_started_ids: set[str] = set()
    run_completed_ids: set[str] = set()
    run_session_started_ids: set[str] = set()
    model_call_started_runs: dict[str, dict[str, Any]] = {}
    model_call_completed_runs: set[str] = set()
    model_call_abnormal_completed_runs: list[dict[str, Any]] = []
    model_call_run_errors: dict[str, dict[str, Any]] = {}
    run_last_events: dict[str, dict[str, Any]] = {}
    unkeyed_model_call_starts = 0
    unkeyed_model_call_completions = 0

    for event in events:
        kind = event_kind(event)
        data = event_payload(event)
        run_id = event_run_id(event, data)
        if run_id:
            run_last_events[run_id] = {
                "kind": kind,
                "tool_name": data.get("tool_name"),
                "path": normalize_evidence_path(data.get("path")) if isinstance(data.get("path"), str) else None,
                "command": clean_transcript_action(data.get("command")) if isinstance(data.get("command"), str) else None,
                "status": data.get("status"),
                "error": data.get("error"),
                "error_detail": data.get("error_detail"),
            }
        if kind == "RunStarted" and run_id:
            run_started_ids.add(run_id)
        elif kind == "SessionStarted" and run_id:
            run_session_started_ids.add(run_id)
        elif kind == "RunCompleted" and run_id:
            run_completed_ids.add(run_id)
        if kind == "ModelCallStarted" and deepseek_model_payload(data):
            deepseek_model_call_started += 1
            if run_id:
                model_call_started_runs[run_id] = data
            else:
                unkeyed_model_call_starts += 1
        elif kind == "ModelCallCompleted" and deepseek_model_payload(data):
            deepseek_model_call_completed += 1
            abnormal_status = abnormal_model_completion_status(data)
            if abnormal_status:
                deepseek_model_call_abnormal_completed += 1
                model_call_abnormal_completed_runs.append(
                    {
                        "run_id": run_id,
                        "model": data.get("model"),
                        "status": abnormal_status,
                        "error": data.get("error"),
                        "error_detail": data.get("error_detail"),
                        "last_event": run_last_events.get(run_id) if run_id else None,
                    }
                )
            if run_id:
                model_call_completed_runs.add(run_id)
            else:
                unkeyed_model_call_completions += 1
            if cache_metrics_expected_from_completion(data):
                expected_cache_metrics += 1
        elif kind == "CacheMetricsRecorded":
            cache_metric_events += 1
            hit = payload_int(data, "prompt_cache_hit_tokens", "cache_hit_tokens")
            miss = payload_int(data, "prompt_cache_miss_tokens", "cache_miss_tokens")
            if hit is not None or miss is not None:
                cache_hit_tokens += hit or 0
                cache_miss_tokens += miss or 0
        elif kind == "RunCompleted":
            run_id = event_run_id(event, data)
            if run_id and (data.get("status") == "error" or data.get("error") or data.get("error_detail")):
                model_call_run_errors[run_id] = data
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

    incomplete_run_ids = sorted(run_id for run_id in model_call_started_runs if run_id not in model_call_completed_runs)
    unmatched_completed_run_ids = sorted(run_id for run_id in model_call_completed_runs if run_id not in model_call_started_runs)
    incomplete_runs = [
        {
            "run_id": run_id,
            "model": model_call_started_runs.get(run_id, {}).get("model"),
            "error": model_call_run_errors.get(run_id, {}).get("error"),
            "error_detail": model_call_run_errors.get(run_id, {}).get("error_detail"),
            "status": model_call_run_errors.get(run_id, {}).get("status"),
            "last_event": run_last_events.get(run_id),
        }
        for run_id in incomplete_run_ids[:8]
    ]
    unmatched_completed_runs = [
        {
            "run_id": run_id,
            "last_event": run_last_events.get(run_id),
        }
        for run_id in unmatched_completed_run_ids[:8]
    ]
    incomplete_count = len(incomplete_run_ids) + max(unkeyed_model_call_starts - unkeyed_model_call_completions, 0)
    unmatched_completed_count = len(unmatched_completed_run_ids) + max(unkeyed_model_call_completions - unkeyed_model_call_starts, 0)
    cache_token_total = cache_hit_tokens + cache_miss_tokens
    cache_hit_ratio = round(cache_hit_tokens / cache_token_total, 6) if cache_token_total > 0 else None
    run_incomplete_ids = sorted(run_id for run_id in run_started_ids if run_id not in run_completed_ids)
    run_unmatched_completed_ids = sorted(run_id for run_id in run_completed_ids if run_id not in run_started_ids)
    run_unmatched_completed = [
        {
            "run_id": run_id,
            "last_event": run_last_events.get(run_id),
            "session_started": run_id in run_session_started_ids,
        }
        for run_id in run_unmatched_completed_ids[:8]
    ]
    run_unstarted_input_validation_errors = [
        run
        for run in run_unmatched_completed
        if is_input_validation_completion(run.get("last_event"))
    ]
    run_unstarted_input_validation_error_ids = {
        str(run.get("run_id") or "")
        for run in run_unstarted_input_validation_errors
        if isinstance(run, dict)
    }
    run_unmatched_non_validation_completed = [
        run
        for run in run_unmatched_completed
        if str(run.get("run_id") or "") not in run_unstarted_input_validation_error_ids
    ]
    run_incomplete = [
        {
            "run_id": run_id,
            "last_event": run_last_events.get(run_id),
        }
        for run_id in run_incomplete_ids[:8]
    ]
    state_lifecycle = {
        "schema_version": 1,
        "runs": {
            "started": len(run_started_ids),
            "completed": len(run_completed_ids),
            "incomplete": len(run_incomplete_ids),
            "unmatched_completed": len(run_unmatched_completed_ids),
            "unstarted_input_validation_error": len(run_unstarted_input_validation_errors),
            "unmatched_non_validation_completed": len(run_unmatched_non_validation_completed),
            "incomplete_runs": run_incomplete,
            "unmatched_completed_runs": run_unmatched_completed_ids[:8],
            "unmatched_completed_details": run_unmatched_completed,
            "unstarted_input_validation_error_runs": run_unstarted_input_validation_errors,
            "unmatched_non_validation_completed_details": run_unmatched_non_validation_completed,
        },
        "model_calls": {
            "started": deepseek_model_call_started,
            "completed": deepseek_model_call_completed,
            "abnormal_completed": deepseek_model_call_abnormal_completed,
            "incomplete": incomplete_count,
            "unmatched_completed": unmatched_completed_count,
            "unkeyed_started": unkeyed_model_call_starts,
            "unkeyed_completed": unkeyed_model_call_completions,
            "incomplete_runs": incomplete_runs,
            "abnormal_completed_runs": model_call_abnormal_completed_runs[:8],
            "unmatched_completed_runs": unmatched_completed_run_ids[:8],
            "unmatched_completed_details": unmatched_completed_runs,
        },
    }
    state_lifecycle["strict_balanced"] = (
        len(run_incomplete_ids) == 0
        and len(run_unmatched_completed_ids) == 0
        and incomplete_count == 0
        and unmatched_completed_count == 0
    )
    state_lifecycle["balanced"] = (
        len(run_incomplete_ids) == 0
        and len(run_unmatched_non_validation_completed) == 0
        and incomplete_count == 0
        and unmatched_completed_count == 0
    )
    state_lifecycle["observed"] = bool(
        run_started_ids
        or run_completed_ids
        or deepseek_model_call_started
        or deepseek_model_call_completed
        or unkeyed_model_call_starts
        or unkeyed_model_call_completions
    )
    state_lifecycle["healthy"] = bool(
        state_lifecycle["observed"]
        and state_lifecycle["balanced"]
        and deepseek_model_call_abnormal_completed == 0
    )
    state_lifecycle["imbalance_causes"] = lifecycle_imbalance_causes(state_lifecycle)
    return {
        "edited_files": compact_list(edited_files, 12),
        "read_files": compact_list(read_files, 12),
        "commands": compact_list(commands, 12),
        "failed_commands": compact_list(failed_commands, 8),
        "failed_tools": compact_list(failed_tools, 8),
        "tool_counts": dict(sorted(tool_names.items(), key=lambda item: (-item[1], item[0]))[:8]),
        "command_count": compact_count(commands),
        "failed_command_count": compact_count(failed_commands),
        "failed_tool_count": compact_count(failed_tools),
        "read_file_count": compact_count(read_files),
        "edited_file_count": compact_count(edited_files),
        "_commands_all": commands,
        "_failed_commands_all": failed_commands,
        "_failed_tools_all": failed_tools,
        "_read_files_all": read_files,
        "_edited_files_all": edited_files,
        "deepseek_cache_hit_ratio": cache_hit_ratio,
        "deepseek_cache_hit_tokens": cache_hit_tokens if cache_token_total > 0 else None,
        "deepseek_cache_miss_tokens": cache_miss_tokens if cache_token_total > 0 else None,
        "deepseek_cache_metric_expected_count": expected_cache_metrics,
        "deepseek_cache_metric_event_count": cache_metric_events,
        "deepseek_cache_metric_missing_count": max(expected_cache_metrics - cache_metric_events, 0),
        "deepseek_model_call_started_count": deepseek_model_call_started,
        "deepseek_model_call_completed_count": deepseek_model_call_completed,
        "deepseek_model_call_abnormal_completed_count": deepseek_model_call_abnormal_completed,
        "deepseek_model_call_incomplete_count": incomplete_count,
        "deepseek_model_call_unmatched_completed_count": unmatched_completed_count,
        "deepseek_model_call_incomplete_runs": incomplete_runs,
        "deepseek_model_call_abnormal_completed_runs": model_call_abnormal_completed_runs[:8],
        "deepseek_model_call_unmatched_completed_runs": unmatched_completed_run_ids[:8],
        "deepseek_model_call_unmatched_completed_details": unmatched_completed_runs,
        "state_lifecycle": state_lifecycle,
    }


def is_input_validation_completion(last_event: Any) -> bool:
    if not isinstance(last_event, dict):
        return False
    if last_event.get("kind") != "RunCompleted":
        return False
    if last_event.get("status") != "error":
        return False
    detail = str(last_event.get("error_detail") or "")
    return detail == "empty_input" or detail.startswith("invalid_input:")


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
        if is_input_validation_completion(last_event):
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


def lifecycle_imbalance_causes(state_lifecycle: dict[str, Any]) -> list[dict[str, Any]]:
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
                {
                    "category": category,
                    "cause": cause,
                    "count": 0,
                    "examples": [],
                },
            )
            row["count"] += 1
            if run_id and len(row["examples"]) < 4:
                row["examples"].append(run_id)

    runs = state_lifecycle.get("runs") if isinstance(state_lifecycle.get("runs"), dict) else {}
    model_calls = (
        state_lifecycle.get("model_calls")
        if isinstance(state_lifecycle.get("model_calls"), dict)
        else {}
    )
    add("run_incomplete", runs.get("incomplete_runs"))
    add("run_unmatched_completed", runs.get("unmatched_completed_details"))
    add("model_call_incomplete", model_calls.get("incomplete_runs"))
    add("model_call_unmatched_completed", model_calls.get("unmatched_completed_details"))
    return sorted(
        rows_by_key.values(),
        key=lambda row: (-int(row.get("count") or 0), str(row.get("category") or ""), str(row.get("cause") or "")),
    )


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
    source_file_values = [
        path
        for commit in commits
        for path in (commit.get("source_files") or [])
        if isinstance(path, str)
    ]
    source_files = compact_list(source_file_values, 16)
    command_values = (
        event_data.get("_commands_all", event_data["commands"])
        + transcript_actions.get("_commands_all", transcript_actions["commands"])
    )
    failed_command_values = (
        event_data.get("_failed_commands_all", event_data["failed_commands"])
        + transcript_actions.get("_failed_commands_all", transcript_actions["failed_commands"])
    )
    failed_tool_values = (
        event_data.get("_failed_tools_all", event_data["failed_tools"])
        + transcript_actions.get("_failed_tools_all", transcript_actions["failed_tools"])
    )
    edited_file_values = (
        event_data.get("_edited_files_all", event_data["edited_files"])
        + transcript_actions.get("_edited_files_all", transcript_actions["edited_files"])
    )
    read_file_values = (
        event_data.get("_read_files_all", event_data["read_files"])
        + transcript_actions.get("_read_files_all", transcript_actions["read_files"])
    )
    touched_source_file_values = [path for path in edited_file_values if source_file(path)] + source_file_values
    edited_files = compact_list(edited_file_values, 12)
    touched_source_files = compact_list(touched_source_file_values, 12)
    read_files = compact_list(read_file_values, 12)
    commands = compact_list(command_values, 12)
    failed_commands = compact_list(failed_command_values, 8)
    failed_tools = compact_list(failed_tool_values, 8)
    failed_tool_summary = failed_tool_pattern_summary(failed_tool_values)
    command_count = compact_count(command_values)
    failed_command_count = compact_count(failed_command_values)
    failed_tool_count = compact_count(failed_tool_values)
    read_file_count = compact_count(read_file_values)
    edited_file_count = compact_count(edited_file_values)
    source_changed_file_count = compact_count(source_file_values)
    touched_source_file_count = compact_count(touched_source_file_values)
    public_transcript_actions = {
        key: value for key, value in transcript_actions.items() if not str(key).startswith("_")
    }
    state_failed_tools_all = event_data.get("_failed_tools_all", event_data["failed_tools"])
    transcript_failed_tools_all = transcript_actions.get("_failed_tools_all", transcript_actions["failed_tools"])
    action_evidence = {
        "schema_version": 1,
        "state": {
            "command_count": event_data["command_count"],
            "failed_command_count": event_data["failed_command_count"],
            "failed_tool_count": event_data["failed_tool_count"],
            "read_file_count": event_data["read_file_count"],
            "edited_file_count": event_data["edited_file_count"],
        },
        "transcripts": {
            "command_count": transcript_actions["command_count"],
            "failed_command_count": transcript_actions["failed_command_count"],
            "failed_tool_count": transcript_actions["failed_tool_count"],
            "read_file_count": transcript_actions["read_file_count"],
            "edited_file_count": transcript_actions["edited_file_count"],
        },
        "merged": {
            "command_count": command_count,
            "failed_command_count": failed_command_count,
            "failed_tool_count": failed_tool_count,
            "read_file_count": read_file_count,
            "edited_file_count": edited_file_count,
        },
        "state_only_failed_tool_count": unique_delta_count(state_failed_tools_all, transcript_failed_tools_all),
        "transcript_only_failed_tool_count": unique_delta_count(transcript_failed_tools_all, state_failed_tools_all),
    }
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
    task_states = structured_task_states(
        task_manifest,
        task_artifacts,
        task_lineage,
        task_verification,
        transcript_data,
    )
    task_lineage = annotate_task_lineage_verification(task_lineage, task_verification)
    causal_chains = annotate_task_lineage_verification(causal_chains, task_verification)
    suggestions = augment_evolution_suggestions(suggestions, task_verification)
    source_patch_count = len(source_commits)
    assessment_state = assessment_artifact_state(session_dir, task_manifest, transcript_data)
    assessment_artifact_present = bool(assessment_state.get("artifact_present"))
    assessment_diagnostic_present = bool(assessment_state.get("diagnostic_present"))
    assessment_transcript_present = bool(assessment_state.get("transcript_present"))
    verification_rows = task_verification.get("rows") if isinstance(task_verification.get("rows"), list) else []
    unlanded_source_task_count = sum(
        1
        for row in verification_rows
        if isinstance(row, dict) and task_unlanded_source_problem(row)
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
    if assessment_artifact_present is False and (task_manifest or assessment_transcript_present or assessment_diagnostic_present):
        if assessment_transcript_present:
            labels.append("assessment artifact missing (assess transcript present)")
        else:
            labels.append("assessment artifact missing")
    if source_changed_file_count:
        labels.append(f"{source_changed_file_count} source file(s) changed")
    elif touched_source_file_count:
        labels.append(f"{touched_source_file_count} source file(s) touched")
    elif edited_file_count:
        labels.append(f"{edited_file_count} evidence/bookkeeping file(s) edited")
    if failed_tool_count:
        labels.append(f"{failed_tool_count} failed tool action(s)")
    if failed_command_count > failed_tool_count:
        labels.append(f"{failed_command_count} failed command/check(s)")
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
        "assessment_diagnostic_present": assessment_diagnostic_present,
        "assessment_transcript_present": assessment_transcript_present,
        "assessment_artifact_state": assessment_state,
        "unlanded_source_task_count": unlanded_source_task_count,
        "transcripts": transcript_data,
        "state_pipeline": state_pipeline,
        "task_manifest": task_manifest,
        "task_artifacts": task_artifacts,
        "task_verification": task_verification,
        "task_states": task_states,
        "causal_chains": causal_chains,
        "evolution_suggestions": suggestions,
        "edited_files": edited_files,
        "edited_file_count": edited_file_count,
        "touched_source_files": touched_source_files,
        "touched_source_file_count": touched_source_file_count,
        "source_changed_files": source_files,
        "source_changed_file_count": source_changed_file_count,
        "commits": serialize_commits(commits),
        "source_commits": serialize_commits(source_commits),
        "bookkeeping_commits": serialize_commits(bookkeeping_commits),
        "task_lineage": [task for task in task_lineage if isinstance(task, dict)],
        "read_files": read_files,
        "read_file_count": read_file_count,
        "commands": commands,
        "failed_commands": failed_commands,
        "failed_tools": failed_tools,
        "failed_tool_summary": failed_tool_summary,
        "command_count": command_count,
        "failed_command_count": failed_command_count,
        "failed_tool_count": failed_tool_count,
        "action_evidence": action_evidence,
        "transcript_actions": public_transcript_actions,
        "tool_counts": event_data["tool_counts"],
        "deepseek_cache_hit_ratio": event_data["deepseek_cache_hit_ratio"],
        "deepseek_cache_hit_tokens": event_data["deepseek_cache_hit_tokens"],
        "deepseek_cache_miss_tokens": event_data["deepseek_cache_miss_tokens"],
        "deepseek_cache_metric_expected_count": event_data["deepseek_cache_metric_expected_count"],
        "deepseek_cache_metric_event_count": event_data["deepseek_cache_metric_event_count"],
        "deepseek_cache_metric_missing_count": event_data["deepseek_cache_metric_missing_count"],
        "deepseek_model_call_started_count": event_data["deepseek_model_call_started_count"],
        "deepseek_model_call_completed_count": event_data["deepseek_model_call_completed_count"],
        "deepseek_model_call_abnormal_completed_count": event_data["deepseek_model_call_abnormal_completed_count"],
        "deepseek_model_call_incomplete_count": event_data["deepseek_model_call_incomplete_count"],
        "deepseek_model_call_unmatched_completed_count": event_data["deepseek_model_call_unmatched_completed_count"],
        "deepseek_model_call_incomplete_runs": event_data["deepseek_model_call_incomplete_runs"],
        "deepseek_model_call_abnormal_completed_runs": event_data["deepseek_model_call_abnormal_completed_runs"],
        "deepseek_model_call_unmatched_completed_runs": event_data["deepseek_model_call_unmatched_completed_runs"],
        "deepseek_model_call_unmatched_completed_details": event_data["deepseek_model_call_unmatched_completed_details"],
        "state_lifecycle": event_data["state_lifecycle"],
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
    for key in (
        "deepseek_cache_hit_tokens",
        "deepseek_cache_miss_tokens",
        "deepseek_cache_hit_ratio",
    ):
        value = work.get(key)
        if not isinstance(value, (int, float)) or isinstance(value, bool):
            continue
        current = gnomes.get(key)
        if current is None or not isinstance(current, (int, float)) or isinstance(current, bool):
            gnomes[key] = value
            recalc_score = True
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
            "deepseek_model_call_started_count",
            "deepseek_model_call_completed_count",
            "deepseek_model_call_abnormal_completed_count",
            "deepseek_model_call_incomplete_count",
            "deepseek_model_call_unmatched_completed_count",
        )
    )
    if cache_metric_signal:
        for key in (
            "deepseek_cache_metric_expected_count",
            "deepseek_cache_metric_event_count",
            "deepseek_cache_metric_missing_count",
            "deepseek_model_call_started_count",
            "deepseek_model_call_completed_count",
            "deepseek_model_call_abnormal_completed_count",
            "deepseek_model_call_incomplete_count",
            "deepseek_model_call_unmatched_completed_count",
        ):
            value = work.get(key)
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                continue
            current = gnomes.get(key)
            if not isinstance(current, (int, float)) or isinstance(current, bool) or int(value) != int(current):
                gnomes[key] = int(value)
                recalc_score = True
    state_lifecycle = work.get("state_lifecycle") if isinstance(work.get("state_lifecycle"), dict) else {}
    run_lifecycle = state_lifecycle.get("runs") if isinstance(state_lifecycle.get("runs"), dict) else {}
    run_lifecycle_keys = {
        "state_run_started_count": "started",
        "state_run_completed_count": "completed",
        "state_run_incomplete_count": "incomplete",
        "state_run_unmatched_completed_count": "unmatched_completed",
        "state_run_unstarted_input_validation_error_count": "unstarted_input_validation_error",
    }
    if any(int(run_lifecycle.get(source_key) or 0) > 0 for source_key in run_lifecycle_keys.values()):
        for gnome_key, source_key in run_lifecycle_keys.items():
            value = run_lifecycle.get(source_key)
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                continue
            current = gnomes.get(gnome_key)
            if not isinstance(current, (int, float)) or isinstance(current, bool) or int(value) != int(current):
                gnomes[gnome_key] = int(value)
                recalc_score = True
    failed_tool_count = int(
        work.get("failed_tool_count")
        or (len(work.get("failed_tools") or []) if isinstance(work.get("failed_tools"), list) else 0)
    )
    if failed_tool_count > int(gnomes.get("tool_error_count") or 0):
        gnomes["tool_error_count"] = failed_tool_count
        recalc_score = True
    manifest = work.get("task_manifest") if isinstance(work.get("task_manifest"), dict) else {}
    verification = work.get("task_verification") if isinstance(work.get("task_verification"), dict) else {}
    task_artifacts = work.get("task_artifacts") if isinstance(work.get("task_artifacts"), list) else []
    task_turn_counts = [
        int(row.get("max_turn_count"))
        for row in task_artifacts
        if isinstance(row, dict) and isinstance(row.get("max_turn_count"), int) and int(row.get("max_turn_count")) > 0
    ]
    if task_turn_counts:
        turn_gnomes = {
            "max_task_turn_count": max(task_turn_counts),
            "avg_task_turn_count": round(sum(task_turn_counts) / len(task_turn_counts), 4),
            "total_task_turn_count": sum(task_turn_counts),
        }
        for key, value in turn_gnomes.items():
            current = gnomes.get(key)
            if current is None or (isinstance(current, (int, float)) and not isinstance(current, bool) and value > current):
                gnomes[key] = value
                recalc_score = True
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
            if isinstance(row, dict) and task_unlanded_source_problem(row)
        )
        protected_reverts = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict)
            and (row.get("protected_revert") or "modified_protected_files" in (row.get("problems") or []))
        )
        scope_mismatches = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict) and task_scope_mismatch(row)
        )
        timeout_with_verdict = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict)
            and "evaluator_timed_out_after_verdict" in (row.get("problems") or [])
        )
        obsolete = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict)
            and (row.get("obsolete") or "task_marked_obsolete" in (row.get("problems") or []))
        )
        api_error = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict)
            and (row.get("api_error") or "implementation_api_error" in (row.get("problems") or []))
        )
        no_edit_reverts = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict) and "no_edit_revert" in (row.get("problems") or [])
        )
        seed_contradictions = int(gnomes.get("task_seed_contradiction_count") or 0)
        manifest_seed_contradictions = int(gnomes.get("task_manifest_seed_contradiction_count") or 0)
        succeeded_count = max(
            int(gnomes.get("tasks_succeeded") or 0),
            int(outcome.get("tasks_succeeded") or 0),
        )
        if (
            seed_contradictions
            and manifest_seed_contradictions == 0
            and task_count > 0
            and verified == task_count
            and succeeded_count >= task_count
            and int(gnomes.get("task_revert_count") or 0) == 0
            and obsolete == 0
        ):
            gnomes["task_seed_replacement_count"] = seed_contradictions
            gnomes["task_seed_contradiction_count"] = 0
            seed_contradictions = 0
            recalc_score = True
        no_edit_reverts_after_seed = max(no_edit_reverts - seed_contradictions, 0)
        additional_seed_explanations = max(seed_contradictions - no_edit_reverts, 0)
        unattempted_explanations = int(gnomes.get("task_unattempted_count") or 0)
        gnomes["task_success_rate"] = verified / task_count
        gnomes["session_success_rate"] = 1.0 if verified == task_count else 0.0
        recalc_score = True
        gnomes["task_obsolete_count"] = max(
            int(gnomes.get("task_obsolete_count") or 0),
            obsolete,
        )
        gnomes["task_api_error_count"] = max(
            int(gnomes.get("task_api_error_count") or 0),
            api_error,
        )
        if seed_contradictions:
            gnomes["task_seed_contradiction_count"] = seed_contradictions
        gnomes["task_no_edit_revert_count"] = max(
            int(gnomes.get("task_no_edit_revert_count") or 0),
            no_edit_reverts_after_seed,
        )
        explained_unverified = sum(
            1
            for row in (verification.get("rows") or [])
            if isinstance(row, dict)
            and not row.get("strict_success")
            and (
                row.get("obsolete")
                or "task_marked_obsolete" in (row.get("problems") or [])
                or row.get("api_error")
                or "implementation_api_error" in (row.get("problems") or [])
                or row.get("protected_revert")
                or "modified_protected_files" in (row.get("problems") or [])
                or task_scope_mismatch(row)
                or "no_edit_revert" in (row.get("problems") or [])
            )
        )
        gnomes["evaluator_unverified_count"] = max(
            unverified - explained_unverified - additional_seed_explanations - unattempted_explanations,
            0,
        )
        gnomes["evaluator_timeout_with_verdict_count"] = max(
            int(gnomes.get("evaluator_timeout_with_verdict_count") or 0),
            timeout_with_verdict,
        )
        gnomes["task_unlanded_source_count"] = unlanded
        gnomes["protected_file_revert_count"] = max(
            int(gnomes.get("protected_file_revert_count") or 0),
            protected_reverts,
        )
        gnomes["task_scope_mismatch_count"] = scope_mismatches
    elif attempted:
        succeeded = int(outcome.get("tasks_succeeded") or 0)
        gnomes["task_success_rate"] = None
        gnomes["session_success_rate"] = 0.0
        gnomes["raw_tasks_attempted"] = max(int(gnomes.get("raw_tasks_attempted") or 0), attempted)
        gnomes["raw_tasks_succeeded"] = max(int(gnomes.get("raw_tasks_succeeded") or 0), succeeded)
        gnomes["task_unverified_raw_attempt_count"] = max(
            int(gnomes.get("task_unverified_raw_attempt_count") or 0),
            attempted,
        )
        gnomes["task_unverified_raw_success_count"] = max(
            int(gnomes.get("task_unverified_raw_success_count") or 0),
            succeeded,
        )
        recalc_score = True
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


def metric_int(metrics: dict[str, Any], key: str, default: int = 0) -> int:
    value = metrics.get(key)
    return int(value) if numeric_value(value) else default


def failed_tool_category_lesson(category: str) -> tuple[str, str, str] | None:
    lessons = {
        "search_regex_error": (
            "tool_error_search_regex",
            "search patterns failed because regex punctuation was interpreted by grep",
            "search simple identifiers or use rg --fixed-strings with scoped paths instead of escaped regex snippets",
        ),
        "search_binary_match": (
            "tool_error_search_binary",
            "search scanned binary build artifacts",
            "scope searches to source paths and exclude target/generated artifacts with rg --glob '!target/**'",
        ),
        "missing_file_read": (
            "tool_error_missing_file",
            "agent read or searched paths that did not exist",
            "verify guessed paths with rg --files before reading them, then search owning symbols instead of retrying absent paths",
        ),
        "read_error": (
            "tool_error_read_path",
            "file-read evidence contained path or access errors",
            "verify paths with rg --files and prefer module or symbol discovery when exact files are uncertain",
        ),
        "edit_context_mismatch": (
            "tool_error_edit_context",
            "edit failed because the replacement context was ambiguous or absent",
            "read a tighter surrounding range and use unique old_text context before applying edits",
        ),
        "bash_tool_error": (
            "tool_error_bash",
            "shell tool commands failed during the session",
            "prefer bounded commands with explicit paths and inspect exit output before retrying broader checks",
        ),
    }
    return lessons.get(category)


def failed_tool_category_lessons(work: dict[str, Any] | None) -> list[dict[str, Any]]:
    if not isinstance(work, dict):
        return []
    summary = work.get("failed_tool_summary") if isinstance(work.get("failed_tool_summary"), dict) else {}
    top_categories = summary.get("top_categories") if isinstance(summary.get("top_categories"), list) else []
    lessons: list[dict[str, Any]] = []
    for row in top_categories:
        if not isinstance(row, dict):
            continue
        category = str(row.get("category") or "")
        mapped = failed_tool_category_lesson(category)
        if not mapped:
            continue
        kind, fingerprint, action = mapped
        lessons.append(
            {
                "kind": kind,
                "fingerprint": fingerprint,
                "action": action,
                "count": int(row.get("count") or 0),
                "source": "failed_tool_summary",
                "metric": f"failed_tool_summary.{category}",
                "examples": row.get("examples") if isinstance(row.get("examples"), list) else [],
            }
        )
    return lessons


def corrected_gnome_lessons(
    gnomes: dict[str, Any],
    existing_lessons: list[dict[str, Any]] | None = None,
    work: dict[str, Any] | None = None,
) -> list[dict[str, Any]]:
    existing_keys = {lesson_key(lesson) for lesson in existing_lessons or [] if isinstance(lesson, dict)}
    candidates = [
        (
            "tool_error_count",
            "tool_error",
            "failed tool actions were recovered from transcripts",
            "inspect failed tool calls and add prompt/tool guards for the dominant failure class",
        ),
        (
            "evaluator_unverified_count",
            "evaluator_unverified",
            "tasks lacked strict verifier evidence",
            "require bounded verifier evidence before counting task success",
        ),
        (
            "task_api_error_count",
            "task_api_error",
            "implementation tasks reverted after API errors",
            "preserve API-error evidence and retry with provider recovery instead of treating the task as a no-change revert",
        ),
        (
            "task_seed_contradiction_count",
            "task_seed_contradiction",
            "seeded tasks contradicted the fresh assessment",
            "validate seeded tasks against fresh assessment evidence and replace contradicted seeds before implementation",
        ),
        (
            "task_no_edit_revert_count",
            "task_no_edit_revert",
            "implementation tasks reverted without edits",
            "force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker",
        ),
        (
            "task_scope_mismatch_count",
            "task_scope_mismatch",
            "implementation touched files outside the selected task surface",
            "tighten task files and implementation prompts so planned Files entries match the intended edit surface",
        ),
        (
            "deepseek_model_call_incomplete_count",
            "deepseek_model_call_incomplete",
            "DeepSeek model call lifecycle was incomplete",
            "close model-call lifecycle events on stream errors, timeouts, and abnormal completions",
        ),
        (
            "state_run_incomplete_count",
            "state_run_incomplete",
            "state run lifecycle was incomplete",
            "emit RunCompleted events for every started run, including timeout and API-error exits",
        ),
        (
            "task_unlanded_source_count",
            "task_unlanded_source",
            "task source edits were not landed in source commits",
            "verify task source edits are committed before marking task completion",
        ),
        (
            "task_unverified_raw_success_count",
            "task_unverified_raw_success",
            "raw task success lacked strict task evidence",
            "show raw success as unverified until task artifacts and verifier rows prove it",
        ),
        (
            "planner_no_task_count",
            "planner_no_task",
            "planner produced no usable task",
            "bound discovery and require a selected task artifact before implementation work starts",
        ),
        (
            "state_live_baseline_shrink_count",
            "state_baseline_shrink",
            "live state baseline shrank during merge",
            "preserve live baseline events when projecting session state into dashboard artifacts",
        ),
        (
            "deepseek_cache_ratio_unverified_count",
            "deepseek_cache_unverified",
            "DeepSeek cache ratio was mentioned without token evidence",
            "report cache ratios only from token-backed model-call metrics",
        ),
        (
            "command_timeout_count",
            "command_timeout",
            "commands timed out during the session",
            "prefer bounded targeted checks and record timeout-specific remediation",
        ),
    ]
    lessons: list[dict[str, Any]] = []
    for lesson in failed_tool_category_lessons(work):
        if lesson.get("count", 0) <= 0:
            continue
        key = lesson_key(lesson)
        if key in existing_keys:
            continue
        lessons.append(lesson)
    has_tool_category_lesson = any(
        str(lesson.get("metric") or "").startswith("failed_tool_summary.")
        for lesson in lessons
    )
    for metric_key, kind, fingerprint, action in candidates:
        if metric_key == "tool_error_count" and has_tool_category_lesson:
            continue
        count = metric_int(gnomes, metric_key)
        if count <= 0:
            continue
        lesson = {
            "kind": kind,
            "fingerprint": fingerprint,
            "action": action,
            "count": count,
            "source": "corrected_gnomes",
            "metric": metric_key,
        }
        key = lesson_key(lesson)
        if key in existing_keys:
            continue
        lessons.append(lesson)
    return lessons[:6]


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
            + metric_float(metrics, "protected_file_revert_count") * 2.0
            + metric_float(metrics, "recurring_failure_count") * 2.0
            + metric_float(metrics, "evolution_friction_count")
            + metric_float(metrics, "planner_no_task_count") * 3.0
            + metric_float(metrics, "task_unattempted_count") * 2.0
            + metric_float(metrics, "task_obsolete_count")
            + metric_float(metrics, "task_seed_contradiction_count") * 2.0
            + metric_float(metrics, "task_no_edit_revert_count") * 2.0
            + metric_float(metrics, "task_api_error_count") * 2.0
            + metric_float(metrics, "task_scope_mismatch_count") * 2.0
            + metric_float(metrics, "evaluator_unverified_count")
            + metric_float(metrics, "evaluator_timeout_with_verdict_count") * 2.0
            + metric_float(metrics, "task_unlanded_source_count") * 2.0
            + metric_float(metrics, "task_unverified_raw_success_count")
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


def gnome_correction_source(key: str, corrected_value: Any, feedback_metrics: dict[str, Any]) -> tuple[str, str]:
    if key in feedback_metrics and feedback_metrics.get(key) == corrected_value:
        return ("log_feedback", "log_feedback.json metric overrode or filled the state gnome")
    if key in {
        "tool_error_count",
        "command_timeout_count",
        "search_error_count",
        "failed_command_count",
    }:
        return ("transcripts", "transcript action parsing corrected the gnome")
    if key.startswith("deepseek_model_call_"):
        return ("state_lifecycle.model_calls", "model-call lifecycle events corrected the gnome")
    if key.startswith("state_run_"):
        return ("state_lifecycle.runs", "run lifecycle events corrected the gnome")
    if key.startswith("deepseek_cache_"):
        return ("deepseek_cache_metrics", "token-backed DeepSeek cache/model metrics corrected the gnome")
    if key in {
        "task_success_rate",
        "session_success_rate",
        "evaluator_unverified_count",
        "evaluator_timeout_with_verdict_count",
        "task_obsolete_count",
        "task_seed_contradiction_count",
        "task_no_edit_revert_count",
        "task_api_error_count",
        "task_scope_mismatch_count",
        "task_unlanded_source_count",
        "protected_file_revert_count",
        "task_unverified_raw_attempt_count",
        "task_unverified_raw_success_count",
        "raw_tasks_attempted",
        "raw_tasks_succeeded",
        "task_unattempted_count",
        "task_artifact_coverage",
        "planner_no_task_count",
        "max_task_turn_count",
        "avg_task_turn_count",
        "total_task_turn_count",
    }:
        return ("task_artifacts", "task artifacts, verifier rows, or raw outcome evidence corrected the gnome")
    if key in {
        "state_live_baseline_shrink_count",
        "state_operational_capture_coverage",
        "state_capture_coverage",
        "task_lineage_capture_coverage",
        "task_lineage_event_count",
    }:
        return ("state_pipeline", "state replay, merge, or trace-quality evidence corrected the gnome")
    if key == "coding_log_score":
        return ("derived_score", "coding log score was recomputed from corrected gnome inputs")
    return ("dashboard_correction", "dashboard normalization corrected the gnome")


def state_gnome_audit(
    raw_state_gnomes: dict[str, Any],
    corrected_gnomes: dict[str, Any],
    feedback_metrics: dict[str, Any],
) -> dict[str, Any]:
    corrections = gnome_corrections(raw_state_gnomes, corrected_gnomes)
    rows: list[dict[str, Any]] = []
    source_counts: dict[str, int] = {}
    for key, correction in corrections.items():
        source, reason = gnome_correction_source(key, correction.get("to"), feedback_metrics)
        source_counts[source] = source_counts.get(source, 0) + 1
        rows.append(
            {
                "key": key,
                "from": correction.get("from"),
                "to": correction.get("to"),
                "source": source,
                "reason": reason,
            }
        )
    rows.sort(key=lambda row: (str(row.get("source") or ""), str(row.get("key") or "")))
    return {
        "schema_version": 1,
        "raw_state_gnome_count": len(raw_state_gnomes),
        "corrected_gnome_count": len(corrected_gnomes),
        "correction_count": len(corrections),
        "corrections_by_source": dict(sorted(source_counts.items())),
        "corrections": rows,
    }


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
        feedback_lessons = log_feedback_top_lessons(session_dir)
        raw_state_gnomes = summary.get("latest_gnomes") if isinstance(summary.get("latest_gnomes"), dict) else {}
        latest_gnomes = corrected_gnomes(summary, work, trace, outcome, feedback_metrics)
        annotate_seed_contradicted_task_states(
            work,
            int(latest_gnomes.get("task_seed_contradiction_count") or 0),
        )
        corrected_lessons = corrected_gnome_lessons(latest_gnomes, feedback_lessons, work)
        gnome_audit = state_gnome_audit(raw_state_gnomes, latest_gnomes, feedback_metrics)
        normalize_work_gnome_snapshots(work, latest_gnomes)
        evals, latest_eval, latest_eval_corrections = normalize_latest_eval_gnomes(evals, latest_gnomes)
        if feedback_lessons:
            work["log_feedback_top_lessons"] = feedback_lessons
        if latest_eval and feedback_lessons:
            latest_eval["top_lessons"] = feedback_lessons
        if corrected_lessons:
            work["corrected_gnome_lessons"] = corrected_lessons
        if latest_eval and corrected_lessons:
            latest_eval["corrected_gnome_lessons"] = corrected_lessons
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
            "gnome_corrections": gnome_corrections(raw_state_gnomes, latest_gnomes),
            "state_gnome_audit": gnome_audit,
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
        session["health_reasons"] = run_health_reasons(session)
        sessions.append(session)
    return sessions


def reason_count(value: Any) -> int:
    try:
        return int(value or 0)
    except (TypeError, ValueError):
        return 0


def harness_attention_reasons(work: dict[str, Any]) -> list[str]:
    reasons: list[str] = []
    failed_tools = work.get("failed_tools") if isinstance(work.get("failed_tools"), list) else []
    failed_tool_count = reason_count(work.get("failed_tool_count")) or len(failed_tools)
    if failed_tool_count:
        reasons.append(f"{failed_tool_count} failed tool action(s)")
    failed_command_count = reason_count(work.get("failed_command_count"))
    if failed_command_count > failed_tool_count:
        reasons.append(f"{failed_command_count} failed command/check(s)")
    lifecycle = work.get("state_lifecycle") if isinstance(work.get("state_lifecycle"), dict) else {}
    lifecycle_missing = lifecycle.get("observed") is not True
    lifecycle_unhealthy = lifecycle.get("observed") is True and lifecycle.get("healthy") is False
    if lifecycle_missing:
        reasons.append("state lifecycle not observed")
    elif lifecycle_unhealthy:
        runs = lifecycle.get("runs") if isinstance(lifecycle.get("runs"), dict) else {}
        model_calls = lifecycle.get("model_calls") if isinstance(lifecycle.get("model_calls"), dict) else {}
        parts = [
            f"runs incomplete {reason_count(runs.get('incomplete'))}",
            f"runs unmatched {reason_count(runs.get('unmatched_completed'))}",
            f"model calls incomplete {reason_count(model_calls.get('incomplete'))}",
            f"model calls unmatched {reason_count(model_calls.get('unmatched_completed'))}",
        ]
        reasons.append(f"state lifecycle unhealthy ({'; '.join(parts)})")
    assessment_missing = (
        work.get("assessment_artifact_present") is False
        and (
            work.get("assessment_transcript_present") is True
            or work.get("assessment_diagnostic_present") is True
        )
    )
    if assessment_missing:
        reasons.append("assessment artifact missing")
    return reasons


def harness_attention(work: dict[str, Any]) -> bool:
    return bool(harness_attention_reasons(work))


def seed_contradicted_task_ids(work: dict[str, Any]) -> set[str]:
    task_states = work.get("task_states") if isinstance(work.get("task_states"), dict) else {}
    rows = task_states.get("tasks") if isinstance(task_states.get("tasks"), list) else []
    return {
        str(row.get("task_id"))
        for row in rows
        if isinstance(row, dict)
        and row.get("task_id")
        and (row.get("seed_contradicted") or str(row.get("state") or "") in {"seed_contradicted", "reverted_seed_contradicted"})
    }


def task_state_explanation_reasons(work: dict[str, Any]) -> list[str]:
    task_states = work.get("task_states") if isinstance(work.get("task_states"), dict) else {}
    state_counts = task_states.get("state_counts") if isinstance(task_states.get("state_counts"), dict) else {}
    seed_contradicted = reason_count(state_counts.get("seed_contradicted")) + reason_count(
        state_counts.get("reverted_seed_contradicted")
    )
    reasons: list[str] = []
    if seed_contradicted:
        reasons.append(f"{seed_contradicted} seeded task(s) contradicted by assessment")
    return reasons


def task_verification_problem_reasons(work: dict[str, Any]) -> list[str]:
    verification = work.get("task_verification") if isinstance(work.get("task_verification"), dict) else {}
    rows = verification.get("rows") if isinstance(verification.get("rows"), list) else []
    seed_ids = seed_contradicted_task_ids(work)
    no_passing = 0
    unlanded = 0
    no_overlap = 0
    no_edit = 0
    no_touched = 0
    timeout_after_verdict = 0
    missing_planned = 0
    for row in rows:
        if not isinstance(row, dict):
            continue
        if str(row.get("task_id") or "") in seed_ids:
            continue
        problems = set(row.get("problems") if isinstance(row.get("problems"), list) else [])
        if "no_passing_verifier" in problems:
            no_passing += 1
        if task_unlanded_source_problem(row):
            unlanded += 1
        if "no_planned_file_overlap" in problems:
            no_overlap += 1
        if "no_edit_revert" in problems:
            no_edit += 1
        if "no_touched_files" in problems:
            no_touched += 1
        if "evaluator_timed_out_after_verdict" in problems:
            timeout_after_verdict += 1
        if "missing_planned_files" in problems:
            missing_planned += 1
    reasons: list[str] = []
    if no_passing:
        reasons.append(f"{no_passing} task(s) without passing verifier")
    if unlanded:
        reasons.append(f"{unlanded} unlanded source task(s)")
    if no_overlap:
        reasons.append(f"{no_overlap} task(s) without planned-file overlap")
    if no_edit:
        reasons.append(f"{no_edit} task(s) reverted without edits")
    if no_touched:
        reasons.append(f"{no_touched} task(s) without touched files")
    if timeout_after_verdict:
        reasons.append(f"{timeout_after_verdict} evaluator timeout(s) after verdict")
    if missing_planned:
        reasons.append(f"{missing_planned} task(s) missing planned files")
    return reasons


def run_health_reasons(session: dict[str, Any]) -> list[str]:
    attempted = session.get("tasks_attempted") or 0
    succeeded = session.get("tasks_succeeded") or 0
    work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
    manifest = work.get("task_manifest") if isinstance(work.get("task_manifest"), dict) else {}
    verification = work.get("task_verification") if isinstance(work.get("task_verification"), dict) else {}
    verified_total = int(verification.get("task_count") or 0)
    verified_count = int(verification.get("verified_task_count") or 0)
    if session.get("reverted"):
        return ["session reverted"]
    if manifest.get("planning_failed"):
        return ["planning produced no task files"] + harness_attention_reasons(work)
    if verified_total and verified_count < verified_total:
        return (
            [f"{verified_count}/{verified_total} verified tasks"]
            + task_state_explanation_reasons(work)
            + task_verification_problem_reasons(work)
            + harness_attention_reasons(work)
        )
    if verified_total and verified_count == verified_total:
        if session.get("build_ok") is not True or session.get("test_ok") is not True:
            return ["build/test did not both pass"]
        reasons = harness_attention_reasons(work)
        return reasons or ["verified tasks and clean harness evidence"]
    if attempted:
        return ["raw outcome tasks lack strict verification"] + harness_attention_reasons(work)
    if session.get("build_ok") is True and session.get("test_ok") is True and attempted == succeeded:
        return ["build/test passed with no attempted tasks"]
    if succeeded:
        return [f"{succeeded}/{attempted} raw outcome tasks succeeded"]
    return ["no success evidence captured"]


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
            return "partial" if harness_attention(work) else "passed"
        return "partial"
    if attempted:
        return "attention"
    if session.get("build_ok") is True and session.get("test_ok") is True and attempted == succeeded:
        return "passed"
    if succeeded:
        return "partial"
    return "attention"


def aggregate_gnome_audit(sessions: list[dict[str, Any]]) -> dict[str, Any]:
    correction_count = 0
    corrected_session_count = 0
    raw_state_gnome_count = 0
    corrected_gnome_count = 0
    source_counts: dict[str, int] = {}
    for session in sessions:
        audit = session.get("state_gnome_audit") if isinstance(session.get("state_gnome_audit"), dict) else {}
        session_corrections = int(audit.get("correction_count") or 0)
        correction_count += session_corrections
        if session_corrections:
            corrected_session_count += 1
        raw_state_gnome_count += int(audit.get("raw_state_gnome_count") or 0)
        corrected_gnome_count += int(audit.get("corrected_gnome_count") or 0)
        source_counts_raw = audit.get("corrections_by_source")
        if not isinstance(source_counts_raw, dict):
            continue
        for source, count in source_counts_raw.items():
            source_counts[str(source)] = source_counts.get(str(source), 0) + int(count or 0)
    top_sources = [
        {"source": source, "count": count}
        for source, count in sorted(source_counts.items(), key=lambda row: (-row[1], row[0]))[:8]
    ]
    return {
        "correction_count": correction_count,
        "corrected_session_count": corrected_session_count,
        "raw_state_gnome_count": raw_state_gnome_count,
        "corrected_gnome_count": corrected_gnome_count,
        "corrections_by_source": dict(sorted(source_counts.items())),
        "top_sources": top_sources,
    }


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
    assessment_artifact_state_counts: dict[str, int] = {}
    gnome_audit = aggregate_gnome_audit(sessions)

    for session in sessions:
        evals += 1 if session.get("latest_eval") else 0
        blockers += len(session.get("blockers") or [])
        events += int(session.get("event_count") or 0)
        work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
        assessment_state = (
            work.get("assessment_artifact_state")
            if isinstance(work.get("assessment_artifact_state"), dict)
            else {}
        )
        assessment_classification = str(assessment_state.get("classification") or "unknown")
        assessment_artifact_state_counts[assessment_classification] = (
            assessment_artifact_state_counts.get(assessment_classification, 0) + 1
        )
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
        "assessment_artifact_state_counts": dict(sorted(assessment_artifact_state_counts.items())),
        "event_counts": event_counts,
        "latest_gnomes": latest_gnomes,
        "gnome_keys": gnome_keys,
        "gnome_audit": gnome_audit,
        "gnome_audit_correction_count": gnome_audit["correction_count"],
        "gnome_audit_correction_sessions": gnome_audit["corrected_session_count"],
        "gnome_audit_corrections_by_source": gnome_audit["corrections_by_source"],
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
        claim_detail = detail
    elif expected == 0 and actual_int is None:
        status = "proven"
        claim_detail = f"{detail} No matching evidence was expected; the count metric was absent."
    else:
        status = "conflict"
        claim_detail = detail
    return claim_row(
        name,
        status,
        {"minimum_count": expected},
        {
            "count": actual_int,
            "raw": actual,
            "evidence_count": len(evidence),
        },
        evidence,
        claim_detail,
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


def task_state_count_claim(work: dict[str, Any]) -> dict[str, Any]:
    task_states = work.get("task_states") if isinstance(work.get("task_states"), dict) else {}
    rows = task_states.get("tasks") if isinstance(task_states.get("tasks"), list) else []
    typed_rows = [row for row in rows if isinstance(row, dict)]
    expected_state_counts: dict[str, int] = {}
    for row in typed_rows:
        state = str(row.get("state") or "unknown")
        expected_state_counts[state] = expected_state_counts.get(state, 0) + 1
    expected_strict_success = sum(1 for row in typed_rows if row.get("strict_success") is True)
    expected_unverified = len(typed_rows) - expected_strict_success
    actual = {
        "task_count": task_states.get("task_count"),
        "strict_success_count": task_states.get("strict_success_count"),
        "unverified_count": task_states.get("unverified_count"),
        "state_counts": task_states.get("state_counts") if isinstance(task_states.get("state_counts"), dict) else {},
    }
    status = "proven"
    if not typed_rows and not task_states:
        status = "observed"
    elif (
        int_or_none(actual["task_count"]) != len(typed_rows)
        or int_or_none(actual["strict_success_count"]) != expected_strict_success
        or int_or_none(actual["unverified_count"]) != expected_unverified
        or actual["state_counts"] != dict(sorted(expected_state_counts.items()))
    ):
        status = "conflict"
    return claim_row(
        "task_state_counts_match_rows",
        status,
        {
            "task_count": len(typed_rows),
            "strict_success_count": expected_strict_success,
            "unverified_count": expected_unverified,
            "state_counts": dict(sorted(expected_state_counts.items())),
        },
        actual,
        [str(row.get("task_id")) for row in typed_rows if row.get("task_id")][:8],
        "Structured task-state summary counts should be derived from task state rows.",
        ["work_summary.task_states.tasks", "work_summary.task_states"],
    )


def failed_tool_summary_count_claim(work: dict[str, Any]) -> dict[str, Any]:
    summary = work.get("failed_tool_summary") if isinstance(work.get("failed_tool_summary"), dict) else {}
    category_counts = summary.get("category_counts") if isinstance(summary.get("category_counts"), dict) else {}
    category_total = sum(int(value or 0) for value in category_counts.values())
    failed_tool_count = int_or_none(work.get("failed_tool_count")) or 0
    total_count = int_or_none(summary.get("total_count"))
    status = "proven"
    if total_count != category_total or total_count != failed_tool_count:
        status = "conflict"
    return claim_row(
        "failed_tool_summary_counts_match_failures",
        status,
        {
            "failed_tool_count": failed_tool_count,
            "category_total": category_total,
        },
        {
            "total_count": total_count,
            "failed_tool_count": failed_tool_count,
            "category_counts": category_counts,
        },
        [
            f"{category}:{count}"
            for category, count in sorted(category_counts.items(), key=lambda item: (-int(item[1] or 0), item[0]))
        ][:8],
        "Failed-tool category totals should match the uncapped failed tool count.",
        ["work_summary.failed_tools", "work_summary.failed_tool_summary"],
    )


def assessment_claim(work: dict[str, Any]) -> dict[str, Any]:
    manifest = work.get("task_manifest") if isinstance(work.get("task_manifest"), dict) else {}
    assessment_state = (
        work.get("assessment_artifact_state")
        if isinstance(work.get("assessment_artifact_state"), dict)
        else {}
    )
    artifact_present = work.get("assessment_artifact_present")
    transcript_present = work.get("assessment_transcript_present")
    manifest_artifacts = manifest.get("artifacts") if isinstance(manifest.get("artifacts"), dict) else {}
    diagnostic_present = bool(
        work.get("assessment_diagnostic_present")
        or manifest.get("assessment_missing_present")
        or manifest_artifacts.get("assessment_missing")
    )
    assessment_transcript = (
        assessment_state.get("transcript_summary")
        if isinstance(assessment_state.get("transcript_summary"), dict)
        else {}
    )
    if artifact_present is True:
        status = "proven"
        detail = str(assessment_state.get("detail") or "Assessment artifact is present.")
    elif diagnostic_present and transcript_present:
        status = "observed"
        detail = str(
            assessment_state.get("detail")
            or "Assessment transcript and missing-assessment diagnostic artifact exist, but assessment.md is missing."
        )
    elif transcript_present:
        status = "observed"
        detail = str(
            assessment_state.get("detail")
            or "Assessment phase transcript exists but the assessment artifact is missing."
        )
    elif artifact_present is False:
        status = "missing"
        detail = str(assessment_state.get("detail") or "No assessment artifact or assessment transcript was found.")
    else:
        status = "missing"
        detail = "Assessment artifact state is unknown."
    evidence = []
    if artifact_present is True:
        evidence.append("tasks/assessment.md")
    if diagnostic_present:
        evidence.append("tasks/assessment_missing.md")
    if transcript_present:
        evidence.append("transcripts/assess.log")
    return claim_row(
        "assessment_artifact_and_transcript_state",
        status,
        {"artifact_present": True},
        {
            "artifact_present": artifact_present,
            "transcript_present": transcript_present,
            "diagnostic_present": diagnostic_present,
            "classification": assessment_state.get("classification"),
            "transcript_classification": assessment_state.get("transcript_classification"),
            "transcript_summary": assessment_transcript,
        },
        evidence,
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
        classification = "trusted_ratio_token_backed"
        detail = "Trusted cache ratio is backed by hit/miss token counts."
    elif ratio is not None:
        status = "conflict"
        classification = "trusted_ratio_without_tokens"
        detail = "Trusted cache ratio is present without token evidence."
    elif prose_mentions and unverified >= prose_mentions:
        status = "proven"
        classification = "prose_ratio_marked_unverified"
        detail = "Prose-only cache ratio claims are withheld from trusted KPI and counted as unverified."
    elif prose_mentions:
        status = "conflict"
        classification = "prose_ratio_not_marked_unverified"
        detail = "Prose-only cache ratio claims exist but are not counted as unverified."
    elif expected_events and missing_events:
        status = "missing"
        classification = "expected_metric_events_missing"
        detail = "Completed DeepSeek model calls have token usage but no CacheMetricsRecorded token evidence was captured."
    elif expected_events and metric_events >= expected_events:
        status = "proven"
        classification = "expected_metric_events_present"
        detail = "Expected DeepSeek cache metric events were captured."
    else:
        status = "proven"
        classification = "no_cache_metric_expected"
        detail = "No trusted cache ratio was claimed and no completed DeepSeek model call required cache metrics."
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
            "classification": classification,
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


def model_call_lifecycle_claim(gnomes: dict[str, Any], work: dict[str, Any]) -> dict[str, Any]:
    started = int(gnomes.get("deepseek_model_call_started_count") or 0)
    completed = int(gnomes.get("deepseek_model_call_completed_count") or 0)
    abnormal_completed = int(gnomes.get("deepseek_model_call_abnormal_completed_count") or 0)
    incomplete = int(gnomes.get("deepseek_model_call_incomplete_count") or 0)
    unmatched_completed = int(gnomes.get("deepseek_model_call_unmatched_completed_count") or 0)
    incomplete_runs = work.get("deepseek_model_call_incomplete_runs")
    if not isinstance(incomplete_runs, list):
        incomplete_runs = []
    abnormal_completed_runs = work.get("deepseek_model_call_abnormal_completed_runs")
    if not isinstance(abnormal_completed_runs, list):
        abnormal_completed_runs = []
    unmatched_completed_runs = work.get("deepseek_model_call_unmatched_completed_runs")
    if not isinstance(unmatched_completed_runs, list):
        unmatched_completed_runs = []
    unmatched_completed_details = work.get("deepseek_model_call_unmatched_completed_details")
    if not isinstance(unmatched_completed_details, list):
        unmatched_completed_details = []
    state_lifecycle = work.get("state_lifecycle") if isinstance(work.get("state_lifecycle"), dict) else {}
    imbalance_causes = (
        state_lifecycle.get("imbalance_causes")
        if isinstance(state_lifecycle.get("imbalance_causes"), list)
        else []
    )
    if started == 0 and completed == 0:
        status = "missing"
        detail = "No DeepSeek model call lifecycle events were captured."
    elif incomplete > 0 or unmatched_completed > 0:
        status = "missing"
        detail = "DeepSeek model call starts and completions do not pair by run_id."
    elif abnormal_completed > 0:
        status = "observed"
        detail = "DeepSeek model call starts and completions pair by run_id, but at least one completion ended abnormally."
    else:
        status = "proven"
        detail = "DeepSeek model call starts and completions are paired by run_id and completed normally."
    return claim_row(
        "deepseek_model_call_lifecycle_balanced",
        status,
        {"started_equals_completed": True, "normal_terminal_status": True},
        {
            "started": started,
            "completed": completed,
            "abnormal_completed": abnormal_completed,
            "incomplete": incomplete,
            "unmatched_completed": unmatched_completed,
            "incomplete_runs": incomplete_runs[:8],
            "abnormal_completed_runs": abnormal_completed_runs[:8],
            "unmatched_completed_runs": unmatched_completed_runs[:8],
            "unmatched_completed_details": unmatched_completed_details[:8],
            "imbalance_causes": [
                row for row in imbalance_causes if row.get("category", "").startswith("model_call_")
            ][:8],
        },
        [
            "ModelCallStarted",
            "ModelCallCompleted",
            "deepseek_model_call_started_count",
            "deepseek_model_call_completed_count",
            "deepseek_model_call_abnormal_completed_count",
            "deepseek_model_call_incomplete_count",
            "deepseek_model_call_unmatched_completed_count",
        ],
        detail,
        ["state.events", "latest_gnomes"],
    )


def state_run_lifecycle_claim(gnomes: dict[str, Any], work: dict[str, Any]) -> dict[str, Any]:
    started = int(gnomes.get("state_run_started_count") or 0)
    completed = int(gnomes.get("state_run_completed_count") or 0)
    incomplete = int(gnomes.get("state_run_incomplete_count") or 0)
    unmatched_completed = int(gnomes.get("state_run_unmatched_completed_count") or 0)
    unstarted_input_validation_error = int(gnomes.get("state_run_unstarted_input_validation_error_count") or 0)
    state_lifecycle = work.get("state_lifecycle") if isinstance(work.get("state_lifecycle"), dict) else {}
    runs = state_lifecycle.get("runs") if isinstance(state_lifecycle.get("runs"), dict) else {}
    incomplete_runs = runs.get("incomplete_runs") if isinstance(runs.get("incomplete_runs"), list) else []
    unmatched_runs = runs.get("unmatched_completed_runs") if isinstance(runs.get("unmatched_completed_runs"), list) else []
    unmatched_details = runs.get("unmatched_completed_details") if isinstance(runs.get("unmatched_completed_details"), list) else []
    unmatched_non_validation_completed = int(runs.get("unmatched_non_validation_completed") or 0)
    unmatched_non_validation_completed_details = (
        runs.get("unmatched_non_validation_completed_details")
        if isinstance(runs.get("unmatched_non_validation_completed_details"), list)
        else []
    )
    unstarted_input_validation_error_runs = (
        runs.get("unstarted_input_validation_error_runs")
        if isinstance(runs.get("unstarted_input_validation_error_runs"), list)
        else []
    )
    if started == 0 and completed == 0:
        status = "missing"
        detail = "No yyds run lifecycle events were captured."
    elif incomplete > 0 or unmatched_non_validation_completed > 0:
        status = "missing"
        detail = "RunStarted and RunCompleted events do not pair by run_id."
        if unstarted_input_validation_error > 0:
            detail += f" {unstarted_input_validation_error} unmatched completion(s) are input-validation exits without RunStarted."
    elif unstarted_input_validation_error > 0:
        status = "proven"
        detail = (
            "RunStarted and RunCompleted events are paired for agent runs; "
            f"{unstarted_input_validation_error} pre-agent input-validation exit(s) were explicitly classified."
        )
    else:
        status = "proven"
        detail = "RunStarted and RunCompleted events are paired by run_id."
    return claim_row(
        "state_run_lifecycle_balanced",
        status,
        {
            "agent_started_equals_completed": True,
            "pre_agent_input_validation_exits_classified": True,
        },
        {
            "started": started,
            "completed": completed,
            "incomplete": incomplete,
            "unmatched_completed": unmatched_completed,
            "unstarted_input_validation_error": unstarted_input_validation_error,
            "unmatched_non_validation_completed": unmatched_non_validation_completed,
            "strict_balanced": bool(state_lifecycle.get("strict_balanced")),
            "agent_balanced": bool(state_lifecycle.get("balanced")),
            "incomplete_runs": incomplete_runs[:8],
            "unmatched_completed_runs": unmatched_runs[:8],
            "unmatched_completed_details": unmatched_details[:8],
            "unstarted_input_validation_error_runs": unstarted_input_validation_error_runs[:8],
            "unmatched_non_validation_completed_details": unmatched_non_validation_completed_details[:8],
            "imbalance_causes": [
                row for row in state_lifecycle.get("imbalance_causes", []) if row.get("category", "").startswith("run_")
            ][:8],
        },
        [
            "RunStarted",
            "RunCompleted",
            "state_run_started_count",
            "state_run_completed_count",
            "state_run_incomplete_count",
            "state_run_unmatched_completed_count",
            "state_run_unstarted_input_validation_error_count",
        ],
        detail,
        ["state.events", "work_summary.state_lifecycle", "latest_gnomes"],
    )


def lifecycle_run_id_rows(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        return []
    rows: list[dict[str, Any]] = []
    for row in value:
        if isinstance(row, dict):
            run_id = row.get("run_id")
            if isinstance(run_id, str) and run_id:
                rows.append(row)
        elif isinstance(row, str) and row:
            rows.append({"run_id": row})
    return rows


def lifecycle_run_id_refs(session: dict[str, Any]) -> dict[str, set[str]]:
    work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
    lifecycle = work.get("state_lifecycle") if isinstance(work.get("state_lifecycle"), dict) else {}
    runs = lifecycle.get("runs") if isinstance(lifecycle.get("runs"), dict) else {}
    model_calls = lifecycle.get("model_calls") if isinstance(lifecycle.get("model_calls"), dict) else {}
    refs: dict[str, set[str]] = {}

    def add(rows: Any, surface: str) -> None:
        for row in lifecycle_run_id_rows(rows):
            run_id = str(row.get("run_id") or "")
            if run_id:
                refs.setdefault(run_id, set()).add(surface)

    add(runs.get("incomplete_runs"), "state_run_incomplete")
    add(runs.get("unmatched_completed_details"), "state_run_unmatched_completed")
    add(runs.get("unmatched_completed_runs"), "state_run_unmatched_completed")
    add(model_calls.get("incomplete_runs"), "model_call_incomplete")
    add(model_calls.get("unmatched_completed_runs"), "model_call_unmatched_completed")
    return refs


def annotate_cross_session_lifecycle_reuse(sessions: list[dict[str, Any]]) -> dict[str, Any]:
    refs: dict[str, dict[str, Any]] = {}
    for session in sessions:
        session_id = str(session.get("id") or "")
        if not session_id:
            continue
        for run_id, surfaces in lifecycle_run_id_refs(session).items():
            row = refs.setdefault(run_id, {"sessions": {}, "surfaces": set()})
            row["sessions"][session_id] = sorted(surfaces)
            row["surfaces"].update(surfaces)

    reused = {
        run_id: row
        for run_id, row in refs.items()
        if isinstance(row.get("sessions"), dict) and len(row["sessions"]) > 1
    }
    for session in sessions:
        work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
        lifecycle = work.get("state_lifecycle") if isinstance(work.get("state_lifecycle"), dict) else {}
        session_refs = lifecycle_run_id_refs(session)
        rows = []
        for run_id, surfaces in session_refs.items():
            reuse = reused.get(run_id)
            if not reuse:
                continue
            session_map = reuse.get("sessions") if isinstance(reuse.get("sessions"), dict) else {}
            rows.append(
                {
                    "run_id": run_id,
                    "session_count": len(session_map),
                    "sessions": sorted(session_map.keys())[:8],
                    "surfaces": sorted(surfaces),
                    "all_surfaces": sorted(reuse.get("surfaces") or []),
                }
            )
        rows.sort(key=lambda row: (-int(row.get("session_count") or 0), str(row.get("run_id") or "")))
        if isinstance(lifecycle, dict):
            lifecycle["cross_session_reused_run_id_count"] = len(rows)
            lifecycle["cross_session_reused_run_ids"] = rows[:8]

    top = [
        {
            "run_id": run_id,
            "session_count": len(row["sessions"]),
            "sessions": sorted(row["sessions"].keys())[:8],
            "surfaces": sorted(row.get("surfaces") or []),
        }
        for run_id, row in reused.items()
    ]
    top.sort(key=lambda row: (-int(row.get("session_count") or 0), str(row.get("run_id") or "")))
    return {
        "schema_version": 1,
        "reused_lifecycle_run_id_count": len(reused),
        "top_reused_lifecycle_run_ids": top[:20],
    }


def session_claims(session: dict[str, Any]) -> list[dict[str, Any]]:
    work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
    gnomes = session.get("latest_gnomes") if isinstance(session.get("latest_gnomes"), dict) else {}
    failed_tools = work.get("failed_tools") if isinstance(work.get("failed_tools"), list) else []
    failed_tool_count = reason_count(work.get("failed_tool_count")) or len(failed_tools)
    unlanded = int(work.get("unlanded_source_task_count") or 0)
    return [
        count_claim(
            "failed_tool_actions_match_tool_error_gnome",
            failed_tool_count,
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
        task_state_count_claim(work),
        failed_tool_summary_count_claim(work),
        assessment_claim(work),
        model_call_lifecycle_claim(gnomes, work),
        state_run_lifecycle_claim(gnomes, work),
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
        session_id = session.get("id")
        claims = [
            {**claim, "session_id": session_id}
            for claim in session_claims(session)
            if isinstance(claim, dict)
        ]
        claim_count += len(claims)
        for row in claims:
            status = str(row.get("status") or "unknown")
            status_counts[status] = status_counts.get(status, 0) + 1
        session_rows.append(
            {
                "id": session_id,
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


def build_dashboard_claim_summary(claims_projection: dict[str, Any]) -> dict[str, Any]:
    summary = claims_projection.get("summary") if isinstance(claims_projection, dict) else {}
    status_counts = summary.get("status_counts") if isinstance(summary, dict) else {}
    if not isinstance(status_counts, dict):
        status_counts = {}
    sessions = claims_projection.get("sessions", []) if isinstance(claims_projection, dict) else []
    if not isinstance(sessions, list):
        sessions = []

    def unresolved_example(session: dict[str, Any], claim: dict[str, Any]) -> dict[str, Any]:
        evidence = claim.get("evidence")
        return {
            "session_id": str(session.get("id") or claim.get("session_id") or ""),
            "ts": session.get("ts"),
            "detail": str(claim.get("detail") or ""),
            "evidence": evidence[:3] if isinstance(evidence, list) else [],
        }

    def unresolved_claim_rows(window: list[Any]) -> list[dict[str, Any]]:
        grouped: dict[tuple[str, str], dict[str, Any]] = {}
        for session in window:
            if not isinstance(session, dict):
                continue
            session_id = str(session.get("id") or "")
            session_ts = session.get("ts")
            for claim in session.get("claims", []):
                if not isinstance(claim, dict):
                    continue
                status = str(claim.get("status") or "unknown")
                if status == "proven":
                    continue
                name = str(claim.get("name") or "unknown_claim")
                key = (name, status)
                row = grouped.setdefault(
                    key,
                    {
                        "name": name,
                        "status": status,
                        "count": 0,
                        "examples": [],
                        "latest_examples": [],
                        "first_session_id": None,
                        "first_ts": None,
                        "latest_session_id": None,
                        "latest_ts": None,
                    },
                )
                row["count"] += 1
                if row["first_session_id"] is None:
                    row["first_session_id"] = session_id
                    row["first_ts"] = session_ts
                row["latest_session_id"] = session_id
                row["latest_ts"] = session_ts
                example = unresolved_example(session, claim)
                if len(row["examples"]) < 5:
                    row["examples"].append(example)
                row["latest_examples"].append(example)
                row["latest_examples"] = row["latest_examples"][-5:]
        return sorted(
            grouped.values(),
            key=lambda row: (-int(row.get("count") or 0), str(row.get("name") or ""), str(row.get("status") or "")),
        )

    latest_session = next((session for session in reversed(sessions) if isinstance(session, dict)), {})
    latest_unresolved = []
    if isinstance(latest_session, dict):
        latest_unresolved = [
            {
                "name": str(claim.get("name") or "unknown_claim"),
                "status": str(claim.get("status") or "unknown"),
                **unresolved_example(latest_session, claim),
            }
            for claim in latest_session.get("claims", [])
            if isinstance(claim, dict) and str(claim.get("status") or "unknown") != "proven"
        ]
    recent_window_size = 5
    recent_sessions = sessions[-recent_window_size:]
    top_unresolved = unresolved_claim_rows(sessions)
    recent_top_unresolved = unresolved_claim_rows(recent_sessions)
    recent_unresolved_count = sum(int(row.get("count") or 0) for row in recent_top_unresolved)
    unresolved_count = sum(
        int(value or 0)
        for status, value in status_counts.items()
        if status != "proven"
    )
    return {
        "claim_count": int(summary.get("claim_count") or 0) if isinstance(summary, dict) else 0,
        "status_counts": dict(sorted(status_counts.items())),
        "unresolved_count": unresolved_count,
        "latest_session_id": latest_session.get("id") if isinstance(latest_session, dict) else None,
        "latest_ts": latest_session.get("ts") if isinstance(latest_session, dict) else None,
        "latest_unresolved_count": len(latest_unresolved),
        "latest_unresolved": latest_unresolved,
        "recent_window_size": recent_window_size,
        "recent_unresolved_count": recent_unresolved_count,
        "recent_top_unresolved": recent_top_unresolved,
        "top_unresolved": top_unresolved,
    }


def session_state_projection(session: dict[str, Any]) -> dict[str, Any]:
    work = session.get("work_summary") if isinstance(session.get("work_summary"), dict) else {}
    task_states = work.get("task_states") if isinstance(work.get("task_states"), dict) else {}
    verification = work.get("task_verification") if isinstance(work.get("task_verification"), dict) else {}
    lifecycle = work.get("state_lifecycle") if isinstance(work.get("state_lifecycle"), dict) else {}
    gnome_audit = session.get("state_gnome_audit") if isinstance(session.get("state_gnome_audit"), dict) else {}
    return {
        "id": session.get("id"),
        "ts": session.get("ts"),
        "health": session.get("health"),
        "headline": work.get("headline"),
        "outcome": {
            "tasks_attempted": session.get("tasks_attempted"),
            "tasks_succeeded": session.get("tasks_succeeded"),
            "reverted": session.get("reverted"),
            "build_ok": session.get("build_ok"),
            "test_ok": session.get("test_ok"),
        },
        "task_summary": {
            "task_count": verification.get("task_count"),
            "verified_task_count": verification.get("verified_task_count"),
            "unverified_task_count": verification.get("unverified_task_count"),
            "state_counts": task_states.get("state_counts") or {},
        },
        "assessment": work.get("assessment_artifact_state")
        if isinstance(work.get("assessment_artifact_state"), dict)
        else {},
        "tasks": task_states.get("tasks") if isinstance(task_states.get("tasks"), list) else [],
        "tool_failures": {
            "failed_tool_count": work.get("failed_tool_count"),
            "failed_command_count": work.get("failed_command_count"),
            "summary": work.get("failed_tool_summary") if isinstance(work.get("failed_tool_summary"), dict) else {},
        },
        "lifecycle": {
            "runs": lifecycle.get("runs") if isinstance(lifecycle.get("runs"), dict) else {},
            "model_calls": lifecycle.get("model_calls") if isinstance(lifecycle.get("model_calls"), dict) else {},
            "cross_session_reused_run_id_count": lifecycle.get("cross_session_reused_run_id_count", 0),
            "cross_session_reused_run_ids": lifecycle.get("cross_session_reused_run_ids")
            if isinstance(lifecycle.get("cross_session_reused_run_ids"), list)
            else [],
        },
        "gnome_audit": {
            "raw_state_gnome_count": gnome_audit.get("raw_state_gnome_count"),
            "corrected_gnome_count": gnome_audit.get("corrected_gnome_count"),
            "correction_count": gnome_audit.get("correction_count"),
            "corrections_by_source": gnome_audit.get("corrections_by_source") or {},
        },
    }


def task_state_summary_for_sessions(session_states: list[dict[str, Any]]) -> dict[str, Any]:
    state_counts: dict[str, int] = {}
    task_count = 0
    strict_success_count = 0
    for row in session_states:
        tasks = row.get("tasks") if isinstance(row.get("tasks"), list) else []
        task_count += len(tasks)
        strict_success_count += sum(
            1
            for task in tasks
            if isinstance(task, dict) and task.get("strict_success")
        )
        for state, count in (row.get("task_summary", {}).get("state_counts") or {}).items():
            state_counts[str(state)] = state_counts.get(str(state), 0) + int(count or 0)
    return {
        "session_count": len(session_states),
        "task_count": task_count,
        "strict_success_count": strict_success_count,
        "unverified_count": task_count - strict_success_count,
        "state_counts": dict(sorted(state_counts.items())),
    }


def build_states_projection(
    sessions: list[dict[str, Any]],
    generated_at: str,
    audit_sessions: Path,
) -> dict[str, Any]:
    session_states = [session_state_projection(session) for session in sessions]
    summary = task_state_summary_for_sessions(session_states)
    recent_window_size = 5
    latest_session = session_states[-1] if session_states else {}
    latest_task_summary = task_state_summary_for_sessions([latest_session]) if latest_session else {}
    recent_task_summary = task_state_summary_for_sessions(session_states[-recent_window_size:])
    gnome_audit = aggregate_gnome_audit(sessions)
    summary.update(
        {
            "latest_session_id": latest_session.get("id") if latest_session else None,
            "latest_ts": latest_session.get("ts") if latest_session else None,
            "latest_task_summary": latest_task_summary,
            "recent_window_size": recent_window_size,
            "recent_task_summary": recent_task_summary,
            "gnome_audit": gnome_audit,
        }
    )
    return {
        "schema_version": 1,
        "generated_at": generated_at,
        "source": str(audit_sessions),
        "summary": summary,
        "sessions": session_states,
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

    .dataset-notice {
      display: none;
      border: 1px solid rgba(178, 119, 24, 0.32);
      border-left: 4px solid var(--warning);
      border-radius: 8px;
      background: rgba(255, 247, 232, 0.9);
      padding: 14px 16px;
      color: #4f3920;
    }

    .dataset-notice.visible {
      display: block;
    }

    .dataset-notice strong {
      display: block;
      margin-bottom: 6px;
    }

    .dataset-notice ul {
      margin: 0;
      padding-left: 18px;
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
      <span>Structured artifacts: <a href="data.json">data.json</a> · <a href="claims.json">claims.json</a> · <a href="states.json">states.json</a></span>
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
    <section class="dataset-notice" id="datasetNotice" aria-live="polite"></section>
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
      state_run_started_count: "State runs started",
      state_run_completed_count: "State runs completed",
      state_run_incomplete_count: "Incomplete state runs",
      state_run_unmatched_completed_count: "Unmatched completed state runs",
      state_run_unstarted_input_validation_error_count: "Input-validation completions without starts",
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
      task_obsolete_count: "Obsolete/stale tasks",
      task_seed_contradiction_count: "Contradicted seed tasks",
      task_no_edit_revert_count: "No-edit task reverts",
      task_api_error_count: "Task API-error reverts",
      task_scope_mismatch_count: "Task scope mismatches",
      task_manifest_available: "Task manifest available",
      task_artifact_coverage: "Task artifact coverage",
      task_lineage_capture_coverage: "Task lineage capture",
      task_lineage_event_count: "Task lineage events",
      task_spec_quality_score: "Task spec quality",
      state_replay_integrity_rate: "State replay integrity",
      evaluator_unverified_count: "Evaluator unverified count",
      evaluator_timeout_with_verdict_count: "Evaluator verdict timeouts",
      task_unlanded_source_count: "Unlanded source tasks",
      raw_tasks_attempted: "Raw tasks attempted",
      raw_tasks_succeeded: "Raw tasks succeeded",
      task_unverified_raw_attempt_count: "Unverified raw task attempts",
      task_unverified_raw_success_count: "Unverified raw task successes",
      max_task_turn_count: "Max task turns",
      avg_task_turn_count: "Avg task turns",
      total_task_turn_count: "Total task turns",
      deepseek_cache_hit_ratio: "DeepSeek cache hit ratio",
      deepseek_cache_hit_tokens: "DeepSeek cache hit tokens",
      deepseek_cache_miss_tokens: "DeepSeek cache miss tokens",
      deepseek_cache_metric_event_count: "Cache metric events",
      deepseek_cache_metric_expected_count: "Cache metric expected runs",
      deepseek_cache_metric_missing_count: "Missing cache metric events",
      deepseek_model_call_started_count: "DeepSeek model calls started",
      deepseek_model_call_completed_count: "DeepSeek model calls completed",
      deepseek_model_call_abnormal_completed_count: "Abnormal DeepSeek model completions",
      deepseek_model_call_incomplete_count: "Incomplete DeepSeek model calls",
      deepseek_model_call_unmatched_completed_count: "Unmatched DeepSeek model completions",
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
      "deepseek_model_call_abnormal_completed_count",
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

    function taskStateSummaryForSessions(sessions) {
      const summary = { sessionCount: (sessions || []).length, taskCount: 0, strictSuccessCount: 0, stateCounts: {} };
      (sessions || []).forEach(session => {
        const taskStates = (session.work_summary || {}).task_states || {};
        const tasks = Array.isArray(taskStates.tasks) ? taskStates.tasks : [];
        if (tasks.length) {
          summary.taskCount += tasks.length;
          tasks.forEach(task => {
            if (task.strict_success) summary.strictSuccessCount += 1;
            const key = task.state || "unknown";
            summary.stateCounts[key] = (summary.stateCounts[key] || 0) + 1;
          });
          return;
        }
        const counts = taskStates.state_counts || {};
        Object.entries(counts).forEach(([key, count]) => {
          const value = Number(count || 0);
          summary.taskCount += value;
          summary.stateCounts[key] = (summary.stateCounts[key] || 0) + value;
        });
        summary.strictSuccessCount += Number(taskStates.strict_success_count || 0);
      });
      summary.unverifiedCount = Math.max(0, summary.taskCount - summary.strictSuccessCount);
      return summary;
    }

    function taskStateCountsText(summary) {
      return Object.entries(summary.stateCounts || {})
        .map(([state, count]) => `${state} ${count}`)
        .join(", ");
    }

    function healthOf(session) {
      const storedHealth = String(session.health || "").trim();
      if (["passed", "partial", "attention", "failed", "reverted"].includes(storedHealth)) {
        return storedHealth;
      }
      const attempted = Number(session.tasks_attempted || 0);
      const succeeded = Number(session.tasks_succeeded || 0);
      const work = session.work_summary || {};
      const manifest = work.task_manifest || {};
      const assessmentState = work.assessment_artifact_state || {};
      const verification = work.task_verification || {};
      const strictTotal = Number(verification.task_count || 0);
      const strictVerified = Number(verification.verified_task_count || 0);
      const failedTools = Array.isArray(work.failed_tools) ? work.failed_tools : [];
      const failedToolCount = Number(work.failed_tool_count ?? failedTools.length);
      const failedCommandCount = Number(work.failed_command_count || 0);
      const lifecycle = work.state_lifecycle || {};
      const lifecycleMissing = lifecycle.observed !== true;
      const lifecycleUnhealthy = lifecycle.observed === true && lifecycle.healthy === false;
      const assessmentMissing = work.assessment_artifact_present === false
        && (work.assessment_transcript_present === true || work.assessment_diagnostic_present === true);
      const harnessAttention = failedToolCount > 0 || failedCommandCount > failedToolCount || lifecycleMissing || lifecycleUnhealthy || assessmentMissing;
      if (session.reverted) return "reverted";
      if (manifest.planning_failed) return "attention";
      if (strictTotal && strictVerified < strictTotal) return strictVerified ? "partial" : "attention";
      if (strictTotal && strictVerified === strictTotal) {
        if (session.build_ok !== true || session.test_ok !== true) return "partial";
        return harnessAttention ? "partial" : "passed";
      }
      if (attempted) return "attention";
      if (session.build_ok === true && session.test_ok === true && attempted === succeeded) return "passed";
      if (succeeded > 0) return "partial";
      return "attention";
    }

    function healthReasonsOf(session) {
      if (Array.isArray(session.health_reasons) && session.health_reasons.length) {
        return session.health_reasons;
      }
      const work = session.work_summary || {};
      const verification = work.task_verification || {};
      const strictTotal = Number(verification.task_count || 0);
      const strictVerified = Number(verification.verified_task_count || 0);
      const failedTools = Array.isArray(work.failed_tools) ? work.failed_tools : [];
      const failedToolCount = Number(work.failed_tool_count ?? failedTools.length);
      const failedCommandCount = Number(work.failed_command_count || 0);
      const lifecycle = work.state_lifecycle || {};
      const lifecycleMissing = lifecycle.observed !== true;
      const lifecycleUnhealthy = lifecycle.observed === true && lifecycle.healthy === false;
      const assessmentMissing = work.assessment_artifact_present === false
        && (work.assessment_transcript_present === true || work.assessment_diagnostic_present === true);
      const reasons = [];
      if (strictTotal && strictVerified < strictTotal) reasons.push(`${strictVerified}/${strictTotal} verified tasks`);
      if (strictTotal && strictVerified < strictTotal) {
        const rows = Array.isArray(verification.rows) ? verification.rows : [];
        const taskStateRows = Array.isArray(work.task_states?.tasks) ? work.task_states.tasks : [];
        const seedIds = new Set();
        let seedContradicted = 0;
        taskStateRows.forEach(row => {
          const state = String(row.state || "");
          if (row.seed_contradicted === true || state === "seed_contradicted" || state === "reverted_seed_contradicted") {
            seedContradicted += 1;
            if (row.task_id) seedIds.add(String(row.task_id));
          }
        });
        const counts = {
          noPassing: 0,
          timedOutPassing: 0,
          unlanded: 0,
          noOverlap: 0,
          noEdit: 0,
          noTouched: 0,
          timeoutAfterVerdict: 0,
          missingPlanned: 0
        };
        rows.forEach(row => {
          if (seedIds.has(String(row.task_id || ""))) return;
          const problems = new Set(Array.isArray(row.problems) ? row.problems : []);
          const protectedRevert = row.protected_revert === true || problems.has("modified_protected_files");
          const scopeMismatch = problems.has("no_planned_file_overlap");
          if (problems.has("no_passing_verifier")) counts.noPassing += 1;
          if (problems.has("timed_out_passing_verdict")) counts.timedOutPassing += 1;
          if (!protectedRevert && !scopeMismatch && (problems.has("source_edits_not_landed") || problems.has("no_landed_source_commit"))) counts.unlanded += 1;
          if (problems.has("no_planned_file_overlap")) counts.noOverlap += 1;
          if (problems.has("no_edit_revert")) counts.noEdit += 1;
          if (problems.has("no_touched_files")) counts.noTouched += 1;
          if (problems.has("evaluator_timed_out_after_verdict")) counts.timeoutAfterVerdict += 1;
          if (problems.has("missing_planned_files")) counts.missingPlanned += 1;
        });
        if (seedContradicted) reasons.push(`${seedContradicted} seeded task(s) contradicted by assessment`);
        if (counts.noPassing) reasons.push(`${counts.noPassing} task(s) without passing verifier`);
        if (counts.timedOutPassing) reasons.push(`${counts.timedOutPassing} task(s) with timed-out passing verifier`);
        if (counts.unlanded) reasons.push(`${counts.unlanded} unlanded source task(s)`);
        if (counts.noOverlap) reasons.push(`${counts.noOverlap} task(s) without planned-file overlap`);
        if (counts.noEdit) reasons.push(`${counts.noEdit} task(s) reverted without edits`);
        if (counts.noTouched) reasons.push(`${counts.noTouched} task(s) without touched files`);
        if (counts.timeoutAfterVerdict) reasons.push(`${counts.timeoutAfterVerdict} evaluator timeout(s) after verdict`);
        if (counts.missingPlanned) reasons.push(`${counts.missingPlanned} task(s) missing planned files`);
      }
      if (failedToolCount > 0) reasons.push(`${failedToolCount} failed tool action(s)`);
      if (failedCommandCount > failedToolCount) reasons.push(`${failedCommandCount} failed command/check(s)`);
      if (lifecycleMissing) reasons.push("state lifecycle not observed");
      else if (lifecycleUnhealthy) {
        const runs = lifecycle.runs || {};
        const modelCalls = lifecycle.model_calls || {};
        const parts = [
          `runs incomplete ${Number(runs.incomplete || 0)}`,
          `runs unmatched ${Number(runs.unmatched_completed || 0)}`,
          `model calls incomplete ${Number(modelCalls.incomplete || 0)}`,
          `model calls unmatched ${Number(modelCalls.unmatched_completed || 0)}`
        ];
        reasons.push(`state lifecycle unhealthy (${parts.join("; ")})`);
      }
      if (assessmentMissing) reasons.push("assessment artifact missing");
      return reasons.length ? reasons : ["no success evidence captured"];
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
      let gnomeAuditCorrectionCount = 0;
      let gnomeAuditCorrectionSessions = 0;
      let gnomeAuditRawCount = 0;
      let gnomeAuditCorrectedCount = 0;
      const gnomeAuditSources = {};

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
        const gnomeAudit = session.state_gnome_audit || {};
        const correctionCount = Number(gnomeAudit.correction_count || 0);
        gnomeAuditCorrectionCount += correctionCount;
        if (correctionCount) gnomeAuditCorrectionSessions += 1;
        gnomeAuditRawCount += Number(gnomeAudit.raw_state_gnome_count || 0);
        gnomeAuditCorrectedCount += Number(gnomeAudit.corrected_gnome_count || 0);
        Object.entries(gnomeAudit.corrections_by_source || {}).forEach(([source, count]) => {
          gnomeAuditSources[source] = (gnomeAuditSources[source] || 0) + Number(count || 0);
        });
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
        gnome_keys: gnomeKeys,
        gnome_audit: {
          correction_count: gnomeAuditCorrectionCount,
          corrected_session_count: gnomeAuditCorrectionSessions,
          raw_state_gnome_count: gnomeAuditRawCount,
          corrected_gnome_count: gnomeAuditCorrectedCount,
          corrections_by_source: gnomeAuditSources,
          top_sources: Object.entries(gnomeAuditSources)
            .sort((a, b) => Number(b[1]) - Number(a[1]) || String(a[0]).localeCompare(String(b[0])))
            .slice(0, 8)
            .map(([source, count]) => ({ source, count }))
        },
        gnome_audit_correction_count: gnomeAuditCorrectionCount,
        gnome_audit_correction_sessions: gnomeAuditCorrectionSessions,
        gnome_audit_corrections_by_source: gnomeAuditSources
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
      const claims = state.data?.claims_summary || {};
      const claimCounts = claims.status_counts || {};
      const claimTotal = Number(claims.claim_count || 0);
      const provenClaims = Number(claimCounts.proven || 0);
      const unresolvedClaims = Number(claims.unresolved_count || 0);
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
      const healthReasons = session ? healthReasonsOf(session).map(text).join("; ") : "";
      document.getElementById("heroSummary").innerHTML = `
        <div>
          <span class="pill ${healthClass(health)}">${text(health)}</span>
          ${session ? `<span class="pill soft">${text(trace.label || "unknown trace")}</span>` : ""}
          <h2 class="hero-title">${heroTitle}</h2>
          <div class="hero-kicker">${heroMeta}</div>
          <p class="hero-copy">${heroCopy}</p>
          ${healthReasons ? `<p class="muted">Health reason: ${healthReasons}</p>` : ""}
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
            <div>
              <div class="label">Claim health</div>
              <strong>${claimTotal ? `${text(provenClaims)}/${text(claimTotal)}` : "-"}</strong>
            </div>
            <div>
              <div class="label">Unresolved claims</div>
              <strong>${text(unresolvedClaims)}</strong>
            </div>
          </div>
        </aside>`;
    }

    function datasetWarnings(data) {
      const warnings = [];
      const schema = Number(data?.schema_version || 0);
      if (data?.error) {
        warnings.push(`Could not load data.json: ${data.error}`);
      }
      if (schema && schema < 2) {
        warnings.push(`data.json schema ${schema} is older than the dashboard's schema 2 evidence model.`);
      }
      if (!data?.generated_at) {
        warnings.push("data.json has no generated_at timestamp, so dashboard freshness cannot be verified.");
      }
      if (!data?.claims_summary) {
        warnings.push("claims_summary is missing; claim health and the unresolved evidence queue are incomplete.");
      }
      const sessions = Array.isArray(data?.sessions) ? data.sessions : [];
      const hasCurrentWorkEvidence = sessions.some(session => {
        const work = session.work_summary || {};
        return Boolean(work.task_states || work.state_lifecycle || work.transcript_actions || work.failed_tool_summary);
      });
      if (sessions.length && !hasCurrentWorkEvidence) {
        warnings.push("session work summaries lack current task-state, action-log, transcript-action, and lifecycle evidence fields.");
      }
      return warnings;
    }

    function renderDatasetNotice(data) {
      const target = document.getElementById("datasetNotice");
      const warnings = datasetWarnings(data || {});
      target.classList.toggle("visible", warnings.length > 0);
      target.innerHTML = warnings.length
        ? `<strong>Dataset schema warning</strong><ul>${warnings.map(item => `<li>${text(item)}</li>`).join("")}</ul>`
        : "";
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
        ["Metric backfills", agg.gnome_audit_correction_count || 0, `${text(agg.gnome_audit_correction_sessions || 0)} session(s) recomputed`],
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
          text: `${text(missingCacheMetrics)} completed DeepSeek model call(s) had token usage but did not emit CacheMetricsRecorded events. Cache ratio remains untrusted until hit/miss token events are present.`
        });
      }
      const incompleteModelCalls = Number(gnomes.deepseek_model_call_incomplete_count || 0);
      if (incompleteModelCalls > 0) {
        notes.push({
          kind: "Model calls",
          className: "warn",
          text: `${text(incompleteModelCalls)} DeepSeek model call(s) started without a matching ModelCallCompleted event. Cache metrics are not expected until a model call completes with token usage.`
        });
      }
      const abnormalModelCompletions = Number(gnomes.deepseek_model_call_abnormal_completed_count || 0);
      if (abnormalModelCompletions > 0) {
        notes.push({
          kind: "Model calls",
          className: "warn",
          text: `${text(abnormalModelCompletions)} DeepSeek model completion(s) ended with a non-normal terminal status. Inspect lifecycle evidence before treating the session as clean.`
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
      const gnomeAudit = agg.gnome_audit || {};
      const gnomeCorrectionCount = Number(gnomeAudit.correction_count || agg.gnome_audit_correction_count || 0);
      if (gnomeCorrectionCount > 0) {
        const topSources = Array.isArray(gnomeAudit.top_sources)
          ? gnomeAudit.top_sources
          : Object.entries(agg.gnome_audit_corrections_by_source || {})
            .sort((a, b) => Number(b[1]) - Number(a[1]) || String(a[0]).localeCompare(String(b[0])))
            .slice(0, 3)
            .map(([source, count]) => ({ source, count }));
        const sourceText = topSources.slice(0, 3).map(row => `${text(row.source || "unknown")} ${text(row.count || 0)}`).join(", ");
        notes.push({
          kind: "Metric audit",
          className: "info",
          text: `${text(gnomeCorrectionCount)} gnome metric value(s) were backfilled or corrected from stronger dashboard evidence across ${text(gnomeAudit.corrected_session_count || agg.gnome_audit_correction_sessions || 0)} session(s); this is a mix of historical backfill and real stale-metric fixes, not a raw bug count.${sourceText ? ` Top sources: ${sourceText}.` : ""}`
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

    function sampleCountLabel(values, total) {
      const sampleSize = Math.min((values || []).filter(Boolean).length, 6);
      const count = Number(total ?? sampleSize);
      if (!Number.isFinite(count) || count <= sampleSize) return sampleSize ? ` (${sampleSize})` : "";
      return ` (${sampleSize} of ${fmt.format(count)})`;
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
      if (pipe.terminal_closure_attempts) {
        rows.push(`terminal closure ${text(pipe.terminal_closure_attempts)} attempt(s); ${text(pipe.terminal_closure_completed_model_calls || 0)} model call(s) and ${text(pipe.terminal_closure_completed_runs || 0)} run(s) closed; ${text(pipe.terminal_closure_noop_attempts || 0)} no-op attempt(s); ${text(pipe.terminal_closure_fallback_scans || 0)} fallback scan(s)`);
        (pipe.terminal_closure_examples || []).slice(0, 3).forEach(row => {
          rows.push(`terminal sample: ${text(row.completed_model_call_count || 0)} model / ${text(row.completed_run_count || 0)} run closures${row.scope ? `; scope ${text(row.scope)}` : ""}`);
        });
      }
      return `<ul class="mini-list">${rows.map(row => `<li>${row}</li>`).join("")}</ul>`;
    }

    function lifecycleLastEventLabel(lastEvent) {
      if (!lastEvent || typeof lastEvent !== "object") return "";
      const bits = [lastEvent.kind, lastEvent.tool_name, lastEvent.path, lastEvent.command, lastEvent.status]
        .filter(value => value !== null && value !== undefined && value !== "")
        .map(value => String(value));
      return bits.length ? `; last ${bits.join(" / ")}` : "";
    }

    function renderLifecycleRuns(label, rows) {
      const values = Array.isArray(rows) ? rows.slice(0, 4) : [];
      if (!values.length) return "";
      return values.map(row => {
        if (typeof row === "string") return `${label}: ${text(row)}`;
        const runId = row && typeof row === "object" ? row.run_id : "";
        return `${label}: ${text(runId || "unknown")}${text(lifecycleLastEventLabel(row.last_event))}`;
      });
    }

    function renderLifecycleCauses(rows) {
      const values = Array.isArray(rows) ? rows.slice(0, 6) : [];
      if (!values.length) return [];
      return values.map(row => {
        const examples = Array.isArray(row.examples) && row.examples.length
          ? `; e.g. ${row.examples.slice(0, 2).map(value => text(value)).join(", ")}`
          : "";
        return `cause ${text(row.category || "-")} / ${text(row.cause || "-")}: ${text(row.count || 0)}${examples}`;
      });
    }

    function renderStateLifecycle(work) {
      const lifecycle = work.state_lifecycle || {};
      const runs = lifecycle.runs || {};
      const modelCalls = lifecycle.model_calls || {};
      if (lifecycle.observed !== true) {
        return `<p class="muted">No run/model lifecycle events observed.</p>`;
      }
      const rows = [
        `status ${text(lifecycle.healthy === true ? "healthy" : "unhealthy")} / agent balanced ${text(lifecycle.balanced === true ? "yes" : "no")}${lifecycle.strict_balanced === false && lifecycle.balanced === true ? " / strict balanced no" : ""}`,
        `runs ${text(runs.started || 0)} started, ${text(runs.completed || 0)} completed, ${text(runs.incomplete || 0)} incomplete, ${text(runs.unmatched_completed || 0)} unmatched completed`,
        `model calls ${text(modelCalls.started || 0)} started, ${text(modelCalls.completed || 0)} completed, ${text(modelCalls.incomplete || 0)} incomplete, ${text(modelCalls.unmatched_completed || 0)} unmatched completed`
      ];
      rows.push(...renderLifecycleRuns("incomplete run", runs.incomplete_runs));
      rows.push(...renderLifecycleRuns("unmatched completed run", runs.unmatched_completed_runs));
      rows.push(...renderLifecycleRuns("incomplete model call", modelCalls.incomplete_runs));
      rows.push(...renderLifecycleRuns("unmatched completed model call", modelCalls.unmatched_completed_details || modelCalls.unmatched_completed_runs));
      rows.push(...renderLifecycleCauses(lifecycle.imbalance_causes));
      const reused = Array.isArray(lifecycle.cross_session_reused_run_ids) ? lifecycle.cross_session_reused_run_ids : [];
      if (reused.length) {
        rows.push(`cross-session reused lifecycle run IDs: ${text(lifecycle.cross_session_reused_run_id_count || reused.length)}`);
        reused.slice(0, 4).forEach(row => {
          const sessions = Array.isArray(row.sessions) ? row.sessions.join(", ") : "";
          const surfaces = Array.isArray(row.surfaces) ? row.surfaces.join(", ") : "";
          rows.push(`reused run: ${text(row.run_id || "unknown")} in ${text(row.session_count || 0)} sessions${surfaces ? `; ${text(surfaces)}` : ""}${sessions ? `; examples ${text(sessions)}` : ""}`);
        });
      }
      return `<ul class="mini-list">${rows.map(row => `<li>${row}</li>`).join("")}</ul>`;
    }

    function renderStateGnomeAudit(session) {
      const audit = session.state_gnome_audit || {};
      const rows = Array.isArray(audit.corrections) ? audit.corrections : [];
      const sourceCounts = audit.corrections_by_source || {};
      const sourceBits = Object.entries(sourceCounts)
        .map(([source, count]) => `${source} ${count}`)
        .join(", ");
      const summary = [
        `raw ${text(audit.raw_state_gnome_count || 0)}`,
        `corrected ${text(audit.corrected_gnome_count || 0)}`,
        `changed ${text(audit.correction_count || 0)}`
      ].join(" / ");
      if (!rows.length) {
        return `<p class="muted">${summary}. Raw state gnomes already match dashboard-corrected gnomes.</p>`;
      }
      return `<p class="muted">${summary}${sourceBits ? `; ${text(sourceBits)}` : ""}</p><ul class="mini-list">${rows.slice(0, 8).map(row => {
        const fromValue = metricValue(row.key || "", row.from);
        const toValue = metricValue(row.key || "", row.to);
        return `<li><strong>${text(row.source || "correction")}: ${text(row.key || "")}</strong><br><span class="muted">${fromValue} → ${toValue}; ${text(row.reason || "")}</span></li>`;
      }).join("")}</ul>`;
    }

    function renderTaskArtifacts(session, work) {
      const manifest = work.task_manifest || {};
      const verification = work.task_verification || {};
      const assessmentState = work.assessment_artifact_state || {};
      const rows = (work.task_artifacts || []).slice(0, 6);
      const attempted = Number(session.tasks_attempted || 0);
      const succeeded = Number(session.tasks_succeeded || 0);
      const manifestTaskCount = Number(manifest.task_count ?? ((manifest.tasks || []).length || 0));
      const artifactCount = (work.task_artifacts || []).length;
      const strictTotal = Number(verification.task_count || 0);
      const strictVerified = Number(verification.verified_task_count || 0);
      const assessmentText = work.assessment_artifact_present === true
        ? "assessment present"
        : work.assessment_artifact_present === false
          ? "assessment missing"
          : "assessment unknown";
      const assessmentDetail = assessmentState.detail ? ` ${text(assessmentState.detail)}` : "";
      const transcriptSummary = assessmentState.transcript_summary || {};
      const transcriptEvidence = transcriptSummary.classification
        ? ` Assessment transcript: ${text(transcriptSummary.classification)}${transcriptSummary.line_count ? ` / ${text(transcriptSummary.line_count)} lines` : ""}.`
        : "";
      const evidenceSummary = `<p class="muted">Task evidence: raw outcome ${text(succeeded)}/${text(attempted)}; manifest ${text(manifestTaskCount)} task(s); artifact bundles ${text(artifactCount)}; strict verification ${text(strictVerified)}/${text(strictTotal)}; ${text(assessmentText)}.${assessmentDetail}${transcriptEvidence}</p>`;
      const manifestArtifacts = manifest.artifacts || {};
      const manifestLinks = [
        manifestArtifacts.manifest ? auditLink(session, manifestArtifacts.manifest, "manifest.json") : "",
        manifestArtifacts.assessment ? auditLink(session, manifestArtifacts.assessment, "assessment.md") : "",
        manifestArtifacts.assessment_missing ? auditLink(session, manifestArtifacts.assessment_missing, "assessment_missing.md") : "",
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
        const latestEval = row.latest_eval_attempt || {};
        const evalLabel = latestEval.transcript_path
          ? auditLink(session, latestEval.transcript_path, `eval ${latestEval.attempt || ""}`.trim())
          : (row.eval_attempt_count ? `eval ${text(latestEval.attempt || row.eval_attempt_count)}` : "no eval attempt");
        const evalBits = [];
        if (latestEval.status) evalBits.push(`status ${latestEval.status}`);
        if (latestEval.verdict) evalBits.push(`verdict ${latestEval.verdict}`);
        if (latestEval.exit_code !== undefined && latestEval.exit_code !== null) evalBits.push(`exit ${latestEval.exit_code}`);
        if (latestEval.timed_out_after_verdict) evalBits.push("timed out after verdict");
        const evalText = row.eval_attempt_count ? `${evalLabel}: ${text(evalBits.join(", ") || "recorded")}` : text(evalLabel);
        const reason = row.revert_reason ? `; ${text(row.revert_reason)}` : "";
        return `<li><span class="${klass}">${text(row.task_id || "")}</span> ${text(row.title || "")}<br><span class="muted">planned ${planned} → touched ${touched} → ${text(problems)}; ${evalText}${reason}</span></li>`;
      }).join("");
      const manifestBlock = manifest.task_count !== undefined ? `<div class="task-evidence">
          <strong>Plan decision ${manifest.planning_failed ? "(planning failed)" : ""}</strong>
          ${evidenceSummary}
          ${planningFailure}
          <p class="muted">${text(manifest.selected_task_count || 0)} selected of ${text(manifest.task_count || 0)} task file(s). ${text((manifest.warnings || []).join(", ") || "No manifest warnings.")}</p>
          ${verificationBlock}
          ${verificationRows ? `<ul class="mini-list">${verificationRows}</ul>` : ""}
          ${manifestLinks ? `<p class="muted">${manifestLinks}</p>` : ""}
          ${manifestTasks ? `<ul class="mini-list"><li>${manifestTasks}</li></ul>` : ""}
        </div>` : "";
      if (!rows.length) return manifestBlock || evidenceSummary + `<p class="muted">No per-task artifact bundle recorded yet.</p>`;
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

    function renderTaskStates(session, work) {
      const summary = work.task_states || {};
      const rows = Array.isArray(summary.tasks) ? summary.tasks : [];
      if (!rows.length) return `<p class="muted">No structured task states recorded.</p>`;
      const counts = Object.entries(summary.state_counts || {})
        .map(([state, count]) => `${state} ${count}`)
        .join(", ");
      return `<p class="muted">${text(summary.strict_success_count || 0)}/${text(summary.task_count || rows.length)} strict task state(s) verified${counts ? `; ${text(counts)}` : ""}.</p><ul class="mini-list">${rows.slice(0, 5).map(row => {
        const transcripts = []
          .concat(row.implementation_transcripts || [])
          .concat(row.eval_transcripts || [])
          .slice(0, 3)
          .map(path => auditLink(session, path, path.split("/").pop()))
          .join(" · ");
        const problems = (row.failure_reasons || []).slice(0, 3).join(", ");
        const attempted = row.attempted ? "attempted" : "not attempted";
        const transcriptText = transcripts ? ` / ${transcripts}` : "";
        const problemText = problems ? ` / ${text(problems)}` : "";
        return `<li><strong>${text(row.task_id || "")}: ${text(row.state || "unknown")}</strong><br><span class="muted">${text(attempted)} / impl attempts ${text(row.implementation_attempt_count || 0)} / eval artifacts ${text(row.eval_attempt_count || 0)}${transcriptText}${problemText}</span></li>`;
      }).join("")}</ul>`;
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
        "task_obsolete_count",
        "task_seed_contradiction_count",
        "task_no_edit_revert_count",
        "task_api_error_count",
        "task_scope_mismatch_count",
        "task_unlanded_source_count",
        "max_task_turn_count",
        "state_operational_capture_coverage",
        "task_lineage_capture_coverage",
        "deepseek_cache_hit_ratio",
        "deepseek_model_call_abnormal_completed_count",
        "deepseek_model_call_incomplete_count",
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

    function renderFailedToolSummary(work) {
      const summary = work.failed_tool_summary || {};
      const rows = Array.isArray(summary.top_categories) ? summary.top_categories : [];
      if (!rows.length) return listItems(work.failed_tools, "No failed tool calls recorded.");
      return `<p class="muted">${text(summary.total_count || 0)} failed tool action(s) across ${text(rows.length)} categor${rows.length === 1 ? "y" : "ies"}.</p><ul class="mini-list">${rows.map(row => {
        const examples = (row.examples || []).slice(0, 2).join(" / ");
        return `<li><strong>${text(row.category || "tool_error")} (${text(row.count || 0)}x)</strong><br><span class="muted">${text(examples || "No examples captured.")}</span></li>`;
      }).join("")}</ul>`;
    }

    function renderActionEvidence(work) {
      const evidence = work.action_evidence || {};
      const stateRows = evidence.state || {};
      const transcriptRows = evidence.transcripts || {};
      const mergedRows = evidence.merged || {};
      if (!Object.keys(evidence).length) return `<p class="muted">No action evidence provenance recorded.</p>`;
      const stateOnlyFails = Number(evidence.state_only_failed_tool_count || 0);
      const transcriptOnlyFails = Number(evidence.transcript_only_failed_tool_count || 0);
      const drift = stateOnlyFails || transcriptOnlyFails
        ? `<br><span class="warn">${text(stateOnlyFails)} state-only / ${text(transcriptOnlyFails)} transcript-only failed tool action(s)</span>`
        : "";
      return `<p class="muted">Merged action counts preserve state-event and transcript evidence without hiding provenance.${drift}</p>
        <div class="detail-grid">
          <div class="item"><strong>State events</strong><br><span class="muted">${text(stateRows.command_count || 0)} commands / ${text(stateRows.failed_tool_count || 0)} tool fails / ${text(stateRows.failed_command_count || 0)} failed checks</span></div>
          <div class="item"><strong>Transcripts</strong><br><span class="muted">${text(transcriptRows.command_count || 0)} commands / ${text(transcriptRows.failed_tool_count || 0)} tool fails / ${text(transcriptRows.failed_command_count || 0)} failed checks</span></div>
          <div class="item"><strong>Merged</strong><br><span class="muted">${text(mergedRows.command_count || 0)} commands / ${text(mergedRows.failed_tool_count || 0)} tool fails / ${text(mergedRows.failed_command_count || 0)} failed checks</span></div>
        </div>`;
    }

    function renderLogFeedbackLessons(session, work) {
      const evalData = session.latest_eval || {};
      const rawRows = (work.log_feedback_top_lessons || evalData.top_lessons || []).slice(0, 4);
      const correctedRows = (work.corrected_gnome_lessons || evalData.corrected_gnome_lessons || []).slice(0, 4);
      if (!rawRows.length && !correctedRows.length) return `<p class="muted">No feedback lessons recorded.</p>`;
      const renderRows = (title, rows) => {
        if (!rows.length) return "";
        return `<p class="muted">${text(title)}</p><ul class="mini-list">${rows.map(row => {
          const kind = row.kind ? `${text(row.kind)}: ` : "";
          const count = row.count === undefined || row.count === null ? "" : ` (${text(row.count)}x)`;
          return `<li><strong>${kind}${text(row.fingerprint || "lesson")}${count}</strong><br><span class="muted">${text(row.action || "")}</span></li>`;
        }).join("")}</ul>`;
      };
      return `${renderRows("Raw log-feedback", rawRows)}${renderRows("Corrected gnome pressure", correctedRows)}`;
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
        const sourceFileCount = Number((work.source_changed_file_count || 0) || (work.touched_source_file_count || 0) || sourceFiles.length);
        const evidenceFileCount = Number(work.edited_file_count ?? evidenceFiles.length);
        const readFileCount = Number(work.read_file_count ?? (work.read_files || []).length);
        const failedTools = work.failed_tools || [];
        const failedToolCount = Number(work.failed_tool_count ?? failedTools.length);
        const failedCommandCount = Number(work.failed_command_count || 0);
        const phaseText = Object.entries(transcripts.phase_counts || {}).map(([phase, count]) => `${phase} ${count}`).join(", ");
        const healthReasons = healthReasonsOf(session).map(text).join("; ");
        return `<article class="item work-row">
          <div class="work-meta">
            <span class="pill ${healthClass(healthOf(session))}">${text(healthOf(session))}</span>
            <span class="pill soft">${text(trace.label || "unknown trace")}</span>
            <strong class="work-title">${text(session.id)}</strong>
            <p class="muted">${text(sessionSourceLine(session))}<br>${text(work.headline || "No detailed work signals captured")}<br>Health reason: ${healthReasons}</p>
          </div>
          <div>
            <div class="work-facts">
              <div class="fact"><strong>${text(session.tasks_succeeded || 0)}/${text(session.tasks_attempted || 0)}</strong>tasks</div>
              <div class="fact"><strong>${text(verification.verified_task_count || 0)}/${text(verification.task_count || 0)}</strong>verified</div>
              <div class="fact"><strong>${text(sourceFileCount || 0)}</strong>source files</div>
              <div class="fact"><strong>${text(work.unlanded_source_task_count || 0)}</strong>unlanded source tasks</div>
              <div class="fact"><strong>${text(evidenceFileCount || 0)}</strong>evidence edits</div>
              <div class="fact"><strong>${text(work.eval_count || 0)}</strong>evals</div>
              <div class="fact"><strong>${work.assessment_artifact_present === true ? "yes" : work.assessment_artifact_present === false ? "no" : "-"}</strong>assessment artifact</div>
              <div class="fact"><strong>${text(work.source_commit_count || 0)}</strong>source commits</div>
              <div class="fact"><strong>${text(failedToolCount || 0)}</strong>tool fails</div>
              <div class="fact"><strong>${text(failedCommandCount || 0)}</strong>failed checks</div>
              <div class="fact"><strong>${text(work.decision_count || 0)}</strong>decisions</div>
              <div class="fact"><strong>${text(trace.trace_event_count || 0)}</strong>trace events</div>
            </div>
            <details class="work-details">
              <summary>Open audit evidence</summary>
              <div class="detail-grid">
                <div><strong>Source changes${sampleCountLabel(sourceFiles, sourceFileCount)}</strong>${listItems(sourceFiles, "No source changes recorded.")}</div>
                <div><strong>Source commits</strong>${sourceCommitItems(work)}</div>
                <div><strong>Bookkeeping commits</strong>${bookkeepingCommitItems(work)}</div>
                <div><strong>Task lineage</strong>${renderTaskLineage(work)}</div>
                <div><strong>Causal chains</strong>${renderCausalChains(work)}</div>
                <div><strong>Next-task suggestions</strong>${renderEvolutionSuggestions(work)}</div>
                <div><strong>Feedback lessons</strong>${renderLogFeedbackLessons(session, work)}</div>
                <div><strong>Task states</strong>${renderTaskStates(session, work)}</div>
                <div><strong>Task decision evidence</strong>${renderTaskArtifacts(session, work)}</div>
                <div><strong>Agent transcripts</strong>${renderTranscriptList(session, work)}</div>
                <div><strong>State lifecycle</strong>${renderStateLifecycle(work)}</div>
                <div><strong>State pipeline</strong>${renderStatePipeline(work)}</div>
                <div><strong>Validated</strong>${listItems(work.commands, "No command events recorded.")}</div>
                <div><strong>Read${sampleCountLabel(work.read_files, readFileCount)}</strong>${listItems(work.read_files, "No file reads recorded.")}</div>
                <div><strong>Evidence/bookkeeping edits${sampleCountLabel(evidenceFiles, evidenceFileCount)}</strong>${listItems(evidenceFiles, "No evidence or bookkeeping edits recorded.")}</div>
                <div><strong>Failures</strong>${listItems(work.failed_commands, "No failed commands recorded.")}</div>
                <div><strong>Action evidence</strong>${renderActionEvidence(work)}</div>
                <div><strong>Tool failures</strong>${renderFailedToolSummary(work)}</div>
                <div><strong>State/gnome audit</strong>${renderStateGnomeAudit(session)}</div>
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
        const healthReasonText = healthReasonsOf(session).map(text).join("; ");
        return `<tr>
          <td><strong>${text(session.id)}</strong><div class="muted">Day ${text(session.day)} at ${text(session.session_time)}<br>${text(session.ts)}<br>${text(sessionSourceLine(session))}${session.github_run_id ? `<br>run ${text(session.github_run_id)} attempt ${text(session.github_run_attempt || "-")}` : ""}</div></td>
          <td><span class="pill ${healthClass(health)}">${text(health)}</span><div class="muted">build ${text(session.build_ok)} / test ${text(session.test_ok)}<br>tasks ${text(session.tasks_succeeded)}/${text(session.tasks_attempted)}${verifiedText}<br>${text(work.headline)}<br>Health reason: ${healthReasonText}</div></td>
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
      const claimSummary = state.data?.claims_summary || {};
      const hasClaimSummary = Boolean(state.data?.claims_summary);
      const claimClass = status => status === "missing" ? "warn" : (status === "conflict" ? "bad" : "info");
      const latestUnresolved = hasClaimSummary ? (claimSummary.latest_unresolved || []) : [];
      const latest = latestSession(sessions);
      const latestTaskSummary = latest ? taskStateSummaryForSessions([latest]) : null;
      if (latestTaskSummary && latestTaskSummary.taskCount) {
        items.push({
          kind: "Latest tasks",
          className: latestTaskSummary.unverifiedCount ? "warn" : "good",
          session: latest.id || "latest",
          title: `${latestTaskSummary.strictSuccessCount}/${latestTaskSummary.taskCount} strict task(s) verified`,
          detail: taskStateCountsText(latestTaskSummary) || "No task-state gaps in the latest session."
        });
      }
      const recentTaskSessions = sessions.slice(-5);
      const recentTaskSummary = taskStateSummaryForSessions(recentTaskSessions);
      if (recentTaskSummary.taskCount) {
        items.push({
          kind: "Recent tasks",
          className: recentTaskSummary.unverifiedCount ? "warn" : "good",
          session: `last ${text(recentTaskSessions.length)}`,
          title: `${recentTaskSummary.strictSuccessCount}/${recentTaskSummary.taskCount} strict task(s) verified`,
          detail: taskStateCountsText(recentTaskSummary) || "No task-state gaps in recent sessions."
        });
      }
      if (hasClaimSummary) {
        items.push({
          kind: "Latest claims",
          className: latestUnresolved.length ? "warn" : "good",
          session: claimSummary.latest_session_id || "latest",
          title: latestUnresolved.length
            ? `${latestUnresolved.length} unresolved in latest session`
            : "Latest session claims all proven",
          detail: latestUnresolved.slice(0, 3).map(row => `${row.name} (${row.status})`).join(", ") || "No active claim gaps in the latest session."
        });
        const recentRows = claimSummary.recent_top_unresolved || [];
        if (recentRows.length) {
          items.push({
            kind: "Recent claims",
            className: "warn",
            session: `last ${text(claimSummary.recent_window_size || 5)}`,
            title: `${text(claimSummary.recent_unresolved_count || 0)} unresolved claim(s) in recent sessions`,
            detail: recentRows.slice(0, 3).map(row => `${row.name} ${row.count}`).join(", ")
          });
        }
      }
      (claimSummary.top_unresolved || []).slice(0, 6).forEach(row => {
        const examples = (row.latest_examples || row.examples || [])
          .slice(0, 2)
          .map(example => example.session_id || "")
          .filter(Boolean)
          .join(", ");
        const recency = row.latest_session_id ? `latest: ${row.latest_session_id}` : "";
        const detail = `${row.status || "unknown"}${recency ? ` / ${recency}` : ""}${examples ? ` / examples: ${examples}` : ""}`;
        items.push({
          kind: "Claim",
          className: claimClass(row.status),
          session: `${text(row.count || 0)} session(s)`,
          title: row.name || "unresolved claim",
          detail
        });
      });
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
      renderDatasetNotice(data);
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
    state_integrity = annotate_cross_session_lifecycle_reuse(sessions)
    gnome_history, gnome_numeric_keys = build_gnome_history(sessions)
    generated_at = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    data = {
        "schema_version": 2,
        "generated_at": generated_at,
        "source": str(audit_sessions),
        "aggregate": aggregate(sessions),
        "state_integrity": state_integrity,
        "gnome_history": gnome_history,
        "gnome_numeric_keys": gnome_numeric_keys,
        "sessions": sessions,
    }
    claims = build_claims_projection(sessions, generated_at, audit_sessions)
    data["claims_summary"] = build_dashboard_claim_summary(claims)
    states = build_states_projection(sessions, generated_at, audit_sessions)
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "data.json").write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (output_dir / "claims.json").write_text(json.dumps(claims, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (output_dir / "states.json").write_text(json.dumps(states, indent=2, sort_keys=True) + "\n", encoding="utf-8")
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
