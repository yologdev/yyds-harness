#!/usr/bin/env python3
"""Build the static harness evolution dashboard from audit-log summaries."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path
from typing import Any


REPO_URL = "https://github.com/yologdev/yyds-harness"


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


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


def compact_list(values: list[str], limit: int) -> list[str]:
    out: list[str] = []
    for value in values:
        text = " ".join(str(value).split())
        if text and text not in out:
            out.append(text)
        if len(out) >= limit:
            break
    return out


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


def trace_quality(summary: dict[str, Any], evals: list[dict[str, Any]]) -> dict[str, Any]:
    counts = summary.get("event_counts") if isinstance(summary.get("event_counts"), dict) else {}
    total = int(summary.get("event_count") or 0)
    patch_evaluated = int(counts.get("PatchEvaluated") or 0)
    trace_events = max(0, total - patch_evaluated)
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
    elif trace_events < 5:
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
        "patch_evaluated_count": patch_evaluated,
        "feedback_eval_count": feedback_evals,
        "state_capture_coverage": 1.0 if trace_events > 0 else 0.0,
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
        transcript_rows.append({"name": name, "phase": phase, "path": f"transcripts/{name}"})
    return {
        "count": len(files),
        "phase_counts": {key: value for key, value in phase_counts.items() if value},
        "files": transcript_rows[:16],
    }


def summarize_events_for_work(events: list[dict[str, Any]]) -> dict[str, Any]:
    edited_files: list[str] = []
    read_files: list[str] = []
    commands: list[str] = []
    failed_commands: list[str] = []
    tool_names: dict[str, int] = {}

    for event in events:
        kind = event_kind(event)
        data = event_payload(event)
        if kind == "FileEdited":
            path = data.get("path") or data.get("file") or data.get("target_path")
            if isinstance(path, str):
                edited_files.append(path)
        elif kind == "FileRead":
            path = data.get("path")
            if isinstance(path, str):
                read_files.append(path)
        elif kind == "CommandStarted":
            command = data.get("command")
            if isinstance(command, str):
                commands.append(command)
        elif kind == "CommandCompleted":
            command = data.get("command")
            if isinstance(command, str):
                commands.append(command)
            if data.get("is_error") is True and isinstance(command, str):
                failed_commands.append(command)
        if kind in {"ToolCallStarted", "ToolCallCompleted"}:
            tool = data.get("tool_name")
            if isinstance(tool, str) and kind == "ToolCallStarted":
                tool_names[tool] = tool_names.get(tool, 0) + 1

    return {
        "edited_files": compact_list(edited_files, 12),
        "read_files": compact_list(read_files, 12),
        "commands": compact_list(commands, 12),
        "failed_commands": compact_list(failed_commands, 8),
        "tool_counts": dict(sorted(tool_names.items(), key=lambda item: (-item[1], item[0]))[:8]),
        "command_count": len(commands),
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
    event_data = summarize_events_for_work(load_jsonl(session_dir / "state" / "events.jsonl"))
    source_files = compact_list(
        [
            path
            for commit in commits
            for path in (commit.get("source_files") or [])
            if isinstance(path, str)
        ],
        16,
    )
    attempted = int(outcome.get("tasks_attempted") or 0)
    succeeded = int(outcome.get("tasks_succeeded") or 0)
    patches = summary.get("patches", []) if isinstance(summary.get("patches"), list) else []
    decisions = summary.get("decisions", []) if isinstance(summary.get("decisions"), list) else []
    latest_eval = evals[-1] if evals else {}
    source_commits = [commit for commit in commits if commit.get("source_files")]
    bookkeeping_commits = [commit for commit in commits if not commit.get("source_files")]
    task_lineage = summary.get("task_lineage") if isinstance(summary.get("task_lineage"), list) else []
    source_patch_count = len(source_commits)
    labels: list[str] = []
    if attempted:
        labels.append(f"{succeeded}/{attempted} tasks completed")
    if source_files:
        labels.append(f"{len(source_files)} source file(s) changed")
    elif event_data["edited_files"]:
        labels.append(f"{len(event_data['edited_files'])} file(s) edited")
    if event_data["command_count"]:
        labels.append(f"{event_data['command_count']} command event(s)")
    if evals:
        labels.append(f"{len(evals)} eval record(s)")
    if blockers:
        labels.append(f"{len(blockers)} blocker(s)")
    if not labels:
        labels.append("No detailed work signals captured")

    return {
        "headline": "; ".join(labels[:4]),
        "labels": labels,
        "transcripts": transcript_data,
        "edited_files": event_data["edited_files"],
        "source_changed_files": source_files,
        "commits": serialize_commits(commits),
        "source_commits": serialize_commits(source_commits),
        "bookkeeping_commits": serialize_commits(bookkeeping_commits),
        "task_lineage": [task for task in task_lineage if isinstance(task, dict)],
        "read_files": event_data["read_files"],
        "commands": event_data["commands"],
        "failed_commands": event_data["failed_commands"],
        "tool_counts": event_data["tool_counts"],
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
        latest_decision = (
            summary.get("latest_decision") if isinstance(summary.get("latest_decision"), dict) else {}
        )
        blockers = [
            blocker
            for blocker in (summary.get("blockers", []) if isinstance(summary.get("blockers"), list) else [])
            if isinstance(blocker, dict) and is_real_blocker(blocker)
        ]
        commits = session_commits(outcome, repo_root)
        trace = trace_quality(summary, evals)
        work = work_summary(session_dir, outcome, summary, evals, blockers, commits)
        sessions.append(
            {
                "id": session_dir.name,
                "day": outcome.get("day"),
                "ts": outcome.get("ts") or summary.get("generated_at"),
                "session_time": outcome.get("session_time"),
                "github_run_id": outcome.get("github_run_id"),
                "github_run_attempt": outcome.get("github_run_attempt"),
                "build_ok": outcome.get("build_ok"),
                "test_ok": outcome.get("test_ok"),
                "tasks_attempted": outcome.get("tasks_attempted"),
                "tasks_succeeded": outcome.get("tasks_succeeded"),
                "reverted": outcome.get("reverted"),
                "event_count": summary.get("event_count", 0),
                "event_counts": summary.get("event_counts", {}),
                "trace_quality": trace,
                "latest_gnomes": summary.get("latest_gnomes", {}),
                "gnome_keys": summary.get("gnome_keys", []),
                "evals": evals,
                "latest_eval": latest_eval,
                "latest_decision": latest_decision,
                "patches": summary.get("patches", []),
                "decisions": summary.get("decisions", []),
                "blockers": blockers,
                "code_refs": summary.get("code_refs", []),
                "work_summary": work,
                "audit_url": f"{REPO_URL}/tree/audit-log/sessions/{session_dir.name}",
            }
        )
    return sessions


def run_health(session: dict[str, Any]) -> str:
    attempted = session.get("tasks_attempted") or 0
    succeeded = session.get("tasks_succeeded") or 0
    if session.get("reverted"):
        return "reverted"
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
    latest_gnomes: dict[str, Any] = {}
    gnome_keys: list[str] = []
    health = {"passed": 0, "partial": 0, "attention": 0, "reverted": 0}
    event_counts: dict[str, int] = {}
    trace_event_count = 0
    full_trace_sessions = 0
    feedback_only_sessions = 0

    for session in sessions:
        evals += 1 if session.get("latest_eval") else 0
        blockers += len(session.get("blockers") or [])
        events += int(session.get("event_count") or 0)
        tasks_attempted += int(session.get("tasks_attempted") or 0)
        tasks_succeeded += int(session.get("tasks_succeeded") or 0)
        health[run_health(session)] += 1
        trace = session.get("trace_quality") if isinstance(session.get("trace_quality"), dict) else {}
        trace_event_count += int(trace.get("trace_event_count") or 0)
        if trace.get("status") == "full":
            full_trace_sessions += 1
        if trace.get("status") == "feedback_only":
            feedback_only_sessions += 1
        latest_gnomes.update(session.get("latest_gnomes") or {})
        for key in session.get("gnome_keys") or []:
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
        "eval_count": evals,
        "promoted_decisions": promoted,
        "rejected_decisions": rejected,
        "blocker_count": blockers,
        "event_count": events,
        "trace_event_count": trace_event_count,
        "full_trace_sessions": full_trace_sessions,
        "feedback_only_sessions": feedback_only_sessions,
        "tasks_attempted": tasks_attempted,
        "tasks_succeeded": tasks_succeeded,
        "task_success_rate": (tasks_succeeded / tasks_attempted) if tasks_attempted else None,
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
        if isinstance(value, bool) or value is None:
            continue
        if isinstance(value, (int, float)) and key not in values:
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
      max_task_turn_count: "Max task turns",
      avg_task_turn_count: "Avg task turns",
      total_task_turn_count: "Total task turns",
      deepseek_cache_hit_ratio: "DeepSeek cache hit ratio",
      deepseek_cache_hit_tokens: "DeepSeek cache hit tokens",
      deepseek_cache_miss_tokens: "DeepSeek cache miss tokens"
    };
    const priorityGnomes = [
      "coding_log_score",
      "task_success_rate",
      "workflow_success_rate",
      "state_capture_coverage",
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

    function metricChip(name, value) {
      return `<span class="pill">${text(name)}: ${text(value)}</span>`;
    }

    function percent(value) {
      if (value === null || value === undefined || Number.isNaN(Number(value))) return "-";
      return `${fmt.format(Number(value) * 100)}%`;
    }

    function metricValue(key, value) {
      if (value === null || value === undefined || Number.isNaN(Number(value))) return text(value);
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
      if (session.reverted) return "reverted";
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
      let evalCount = 0;
      let blockers = 0;
      let promoted = 0;
      let rejected = 0;
      let traceEventCount = 0;
      let fullTraceSessions = 0;
      let feedbackOnlySessions = 0;

      sessions.forEach(session => {
        const healthKey = healthOf(session);
        health[healthKey] = (health[healthKey] || 0) + 1;
        eventCount += Number(session.event_count || 0);
        tasksAttempted += Number(session.tasks_attempted || 0);
        tasksSucceeded += Number(session.tasks_succeeded || 0);
        blockers += (session.blockers || []).length;
        if (session.latest_eval && Object.keys(session.latest_eval).length) evalCount += 1;
        const trace = session.trace_quality || {};
        traceEventCount += Number(trace.trace_event_count || 0);
        if (trace.status === "full") fullTraceSessions += 1;
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
        event_count: eventCount,
        trace_event_count: traceEventCount,
        full_trace_sessions: fullTraceSessions,
        feedback_only_sessions: feedbackOnlySessions,
        event_counts: eventCounts,
        tasks_attempted: tasksAttempted,
        tasks_succeeded: tasksSucceeded,
        task_success_rate: tasksAttempted ? tasksSucceeded / tasksAttempted : null,
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
      const score = latestMetric(agg, "coding_log_score");
      const stateCapture = latestMetric(agg, "state_capture_coverage");
      const heroTitle = session
        ? `${text(session.tasks_succeeded || 0)} of ${text(session.tasks_attempted || 0)} tasks completed`
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
      const cards = [
        ["Sessions", agg.session_count || 0, "audit-backed runs"],
        ["Task success", rate === null || rate === undefined ? "-" : percent(rate), `${text(agg.tasks_succeeded || 0)}/${text(agg.tasks_attempted || 0)} tasks`],
        ["Full traces", agg.full_trace_sessions || 0, `${text(agg.feedback_only_sessions || 0)} feedback-only`],
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
      document.getElementById("taskChart").innerHTML = attempted
        ? barRow("Successful tasks", succeeded, attempted, succeeded === attempted ? "good" : "warn", `of ${attempted}`)
        : `<div class="empty">No task outcome data yet.</div>`;

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
        .map(key => [key, allGnomes[key]]);
      const fallbackRows = Object.entries(allGnomes)
        .filter(([key]) => !priorityGnomes.includes(key))
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
      const preferred = ["coding_log_score", "task_success_rate", "workflow_success_rate", "state_capture_coverage", "evolution_friction_count", "max_task_turn_count", "deepseek_cache_hit_ratio", "cache_hit_ratio"];
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
        const sourceFiles = (work.source_changed_files || []).length ? work.source_changed_files : work.edited_files;
        const phaseText = Object.entries(transcripts.phase_counts || {}).map(([phase, count]) => `${phase} ${count}`).join(", ");
        return `<article class="item work-row">
          <div class="work-meta">
            <span class="pill ${healthClass(healthOf(session))}">${text(healthOf(session))}</span>
            <span class="pill soft">${text(trace.label || "unknown trace")}</span>
            <strong class="work-title">${text(session.id)}</strong>
            <p class="muted">${text(work.headline || "No detailed work signals captured")}</p>
          </div>
          <div>
            <div class="work-facts">
              <div class="fact"><strong>${text(session.tasks_succeeded || 0)}/${text(session.tasks_attempted || 0)}</strong>tasks</div>
              <div class="fact"><strong>${text(sourceFiles.length || 0)}</strong>source files</div>
              <div class="fact"><strong>${text(work.eval_count || 0)}</strong>evals</div>
              <div class="fact"><strong>${text(work.source_commit_count || 0)}</strong>source commits</div>
              <div class="fact"><strong>${text(work.decision_count || 0)}</strong>decisions</div>
              <div class="fact"><strong>${text(trace.trace_event_count || 0)}</strong>trace events</div>
            </div>
            <details class="work-details">
              <summary>Open audit evidence</summary>
              <div class="detail-grid">
                <div><strong>Changed</strong>${listItems(sourceFiles, "No repo changes recorded.")}</div>
                <div><strong>Source commits</strong>${sourceCommitItems(work)}</div>
                <div><strong>Bookkeeping commits</strong>${bookkeepingCommitItems(work)}</div>
                <div><strong>Task lineage</strong>${renderTaskLineage(work)}</div>
                <div><strong>Validated</strong>${listItems(work.commands, "No command events recorded.")}</div>
                <div><strong>Read</strong>${listItems(work.read_files, "No file reads recorded.")}</div>
                <div><strong>State edits</strong>${listItems(work.edited_files, "No FileEdited events recorded.")}</div>
                <div><strong>Failures</strong>${listItems(work.failed_commands, "No failed commands recorded.")}</div>
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
        return `<tr>
          <td><strong>${text(session.id)}</strong><div class="muted">Day ${text(session.day)} at ${text(session.session_time)}<br>${text(session.ts)}${session.github_run_id ? `<br>run ${text(session.github_run_id)} attempt ${text(session.github_run_attempt || "-")}` : ""}</div></td>
          <td><span class="pill ${healthClass(health)}">${text(health)}</span><div class="muted">build ${text(session.build_ok)} / test ${text(session.test_ok)}<br>tasks ${text(session.tasks_succeeded)}/${text(session.tasks_attempted)}<br>${text(work.headline)}</div></td>
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
        return `<li>${text(task.task_id || "")} ${text(task.status || "-")}: ${text(task.task_title || "")} (${text(fileCount)} files, ${text(commitCount)} commits${evalVerdict}${deltaText})</li>`;
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
          items.push({ kind: "Task link", className: deltas ? "good" : "info", session: session.id, title: `${task.task_id || ""} ${task.task_title || ""}`, detail: `${task.status || "-"} / ${files} files / ${commits} commits / ${deltas} gnome deltas` });
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
    data = {
        "schema_version": 2,
        "source": str(audit_sessions),
        "aggregate": aggregate(sessions),
        "gnome_history": gnome_history,
        "gnome_numeric_keys": gnome_numeric_keys,
        "sessions": sessions,
    }
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "data.json").write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
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
