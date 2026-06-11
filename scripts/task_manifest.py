#!/usr/bin/env python3
"""Build durable task decision manifests for evolution audit sessions."""

from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path
from typing import Any


FIELD_RE = re.compile(r"^([A-Za-z][A-Za-z 0-9_-]*):\s*(.*)$")
GOAL_RE = re.compile(r"(?im)^(?:#+\s*)?(?:goal|objective)\s*:?\s*$|^(?:goal|objective)\s*:")


def split_list(value: str) -> list[str]:
    out: list[str] = []
    for part in value.replace(";", ",").split(","):
        text = " ".join(part.split())
        if text and text not in out:
            out.append(text)
    return out


def read_text(path: Path | None) -> str:
    if path is None or not path.is_file():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def parse_task(path: Path, task_number: int) -> dict[str, Any]:
    text = read_text(path)
    fields: dict[str, str] = {}
    body_lines: list[str] = []
    in_fields = True
    for line in text.splitlines():
        match = FIELD_RE.match(line)
        if in_fields and match:
            fields[match.group(1).strip().lower().replace(" ", "_")] = match.group(2).strip()
            continue
        if in_fields and not line.strip():
            continue
        if in_fields and line.lstrip().startswith("#"):
            body_lines.append(line)
            continue
        in_fields = False
        body_lines.append(line)

    lower = text.lower()
    has_success = "success criteria" in lower or "acceptance criteria" in lower
    has_verification = "verification" in lower or "test plan" in lower
    has_goal = bool(GOAL_RE.search(text))
    generic = (
        fields.get("title", "").strip().lower() == "self-improvement"
        and "identify the most impactful improvement" in lower
    )
    quality_score = sum([has_success, has_verification, has_goal, not generic]) / 4.0
    title = fields.get("title") or f"Task {task_number}"
    files = split_list(fields.get("files", ""))
    return {
        "task_id": f"task_{task_number:02d}",
        "task_number": task_number,
        "title": title,
        "files": files,
        "issue": fields.get("issue") or None,
        "origin": fields.get("origin") or "planner",
        "artifact_path": f"tasks/task_{task_number:02d}/task.md",
        "session_plan_path": str(path),
        "body_preview": " ".join(" ".join(body_lines).split())[:360],
        "quality": {
            "has_goal": has_goal,
            "has_success_criteria": has_success,
            "has_verification": has_verification,
            "generic_self_improvement": generic,
            "score": round(quality_score, 4),
        },
    }


def build_manifest(args: argparse.Namespace) -> dict[str, Any]:
    plan_dir = args.session_plan_dir
    task_paths = sorted(plan_dir.glob("task_*.md")) if plan_dir.is_dir() else []
    tasks = [parse_task(path, index + 1) for index, path in enumerate(task_paths)]
    selected = tasks[: args.selected_limit]
    assessment_text = read_text(args.assessment_file)
    assessment_missing_text = read_text(getattr(args, "assessment_missing_file", None))
    issue_text = read_text(args.issue_responses_file)
    failure_text = read_text(args.planning_failure_file)
    planning_failed = bool(args.planning_failed or failure_text or not tasks)
    warnings: list[str] = []
    if planning_failed:
        warnings.append("planner_produced_no_task_files")
    for task in tasks:
        quality = task.get("quality") if isinstance(task.get("quality"), dict) else {}
        if quality.get("generic_self_improvement"):
            warnings.append(f"{task['task_id']}:generic_self_improvement")
        if not task.get("files"):
            warnings.append(f"{task['task_id']}:missing_files")
        if float(quality.get("score") or 0.0) < 0.75:
            warnings.append(f"{task['task_id']}:thin_task_spec")

    return {
        "schema_version": 1,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "planner": {
            "planning_failed": planning_failed,
            "task_count": len(tasks),
            "selected_task_count": min(len(tasks), args.selected_limit),
            "assessment_present": bool(assessment_text.strip()),
            "assessment_missing_present": bool(assessment_missing_text.strip()),
            "issue_responses_present": bool(issue_text.strip()),
            "planning_failure_present": bool(failure_text.strip()),
        },
        "tasks": tasks,
        "selected_tasks": selected,
        "artifacts": {
            "assessment": "tasks/assessment.md" if assessment_text.strip() else None,
            "assessment_missing": "tasks/assessment_missing.md" if assessment_missing_text.strip() else None,
            "issue_responses": "tasks/issue_responses.md" if issue_text.strip() else None,
            "planning_failure": "tasks/planning_failure.md" if failure_text.strip() else None,
            "manifest": "tasks/manifest.json",
        },
        "warnings": list(dict.fromkeys(warnings)),
    }


def decision_payload(manifest: dict[str, Any]) -> dict[str, Any]:
    planner = manifest.get("planner") if isinstance(manifest.get("planner"), dict) else {}
    return {
        "phase": "plan",
        "decision_type": "session_plan",
        "decision": "planning_failed" if planner.get("planning_failed") else "tasks_selected",
        "task_count": int(planner.get("task_count") or 0),
        "selected_task_count": int(planner.get("selected_task_count") or 0),
        "assessment_present": bool(planner.get("assessment_present")),
        "planning_failed": bool(planner.get("planning_failed")),
        "reason": (
            "planning phase produced no task files"
            if planner.get("planning_failed")
            else "planning phase selected implementation tasks for this evolution session"
        ),
        "tasks": [
            {
                "task_id": task.get("task_id"),
                "task_number": task.get("task_number"),
                "task_title": task.get("title"),
                "planned_files": task.get("files") or [],
                "issue": task.get("issue"),
                "origin": task.get("origin"),
                "quality": task.get("quality"),
            }
            for task in (manifest.get("selected_tasks") or [])
            if isinstance(task, dict)
        ],
        "warnings": manifest.get("warnings") or [],
    }


def write_task_decisions(manifest: dict[str, Any], output: Path) -> None:
    tasks_dir = output.parent
    for task in manifest.get("selected_tasks") or []:
        if not isinstance(task, dict):
            continue
        task_id = str(task.get("task_id") or "")
        if not task_id:
            continue
        task_dir = tasks_dir / task_id
        task_dir.mkdir(parents=True, exist_ok=True)
        (task_dir / "decision.json").write_text(
            json.dumps(task, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--session-plan-dir", type=Path, default=Path("session_plan"))
    parser.add_argument("--assessment-file", type=Path)
    parser.add_argument("--assessment-missing-file", type=Path)
    parser.add_argument("--issue-responses-file", type=Path)
    parser.add_argument("--planning-failure-file", type=Path)
    parser.add_argument("--selected-limit", type=int, default=3)
    parser.add_argument("--planning-failed", action="store_true")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--decision-payload", action="store_true")
    parser.add_argument("--write-task-decisions", action="store_true")
    args = parser.parse_args()

    manifest = build_manifest(args)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        if args.write_task_decisions:
            write_task_decisions(manifest, args.output)
    payload = decision_payload(manifest) if args.decision_payload else manifest
    print(json.dumps(payload, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
