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
    "session_success_rate",
    "task_success_rate",
    "task_verification_rate",
    "task_mechanical_verification_rate",
    "task_artifact_coverage",
    "task_spec_quality_score",
    "evolution_friction_count",
    "command_timeout_count",
    "evaluator_timeout_count",
    "evaluator_unverified_count",
    "search_error_count",
    "max_task_turn_count",
    "deepseek_cache_hit_ratio",
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


def selected_tasks(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    tasks = manifest.get("selected_tasks") or manifest.get("tasks") or []
    return [task for task in tasks if isinstance(task, dict)]


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

    base_gnomes = latest_gnomes(baseline_dir)
    cand_gnomes = latest_gnomes(candidate_dir)
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
    gnomes = latest_gnomes(session_dir)
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
    if int(gnomes.get("evaluator_unverified_count") or 0) > 0:
        add("eval", "Bound evaluator checks so verdicts are not skipped", "Some task evals were unverified or timed out.", "evaluator_unverified_count", gnomes.get("evaluator_unverified_count"), 90)
    if int(gnomes.get("evaluator_timeout_count") or 0) > 0:
        add("eval", "Make evaluator timeouts resumable or cheaper", "Evaluator timeout friction still appears in action logs.", "evaluator_timeout_count", gnomes.get("evaluator_timeout_count"), 85)
    if int(gnomes.get("search_error_count") or 0) > 0:
        add("tooling", "Harden search commands and pattern escaping", "Search/grep errors created avoidable evolution friction.", "search_error_count", gnomes.get("search_error_count"), 80)
    if int(gnomes.get("command_timeout_count") or 0) > 0:
        add("tooling", "Prefer bounded diagnostics before broad commands", "Command timeouts slowed the coding loop.", "command_timeout_count", gnomes.get("command_timeout_count"), 75)
    max_turns = gnomes.get("max_task_turn_count")
    if isinstance(max_turns, (int, float)) and not isinstance(max_turns, bool) and max_turns >= 25:
        add("planning", "Split high-turn tasks into narrower plans", "A task used many turns, suggesting the task was too broad or under-specified.", "max_task_turn_count", max_turns, 70)
    if gnomes.get("task_artifact_coverage") == 0:
        add("state", "Restore task artifact coverage", "Task decisions or artifacts were missing from the audit bundle.", "task_artifact_coverage", 0, 95)
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
