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
TASK_FILE_RE = re.compile(r"^task_\d{2}\.md$")
FILE_ANNOTATION_RE = re.compile(
    r"\s+\((?:new|new file|new module|existing|modified|created|generated|optional)[^)]*\)\s*$",
    re.IGNORECASE,
)
FILE_TRAILING_NOTE_RE = re.compile(r"\s+(?:-|--|—)\s+.*$")
FILE_MENTION_RE = re.compile(
    r"(?<![\w./-])((?:src|scripts|tests|docs|eval|memory|skills|tasks|site|journals)/"
    r"[A-Za-z0-9_./+-]*[A-Za-z0-9_+-]\.[A-Za-z0-9_+-]+|"
    r"Cargo\.(?:toml|lock)|README\.md)(?!(?:[\w/-]|\.[A-Za-z0-9_+-]))"
)
PROTECTED_IMPLEMENTATION_SURFACES = (
    ".github/workflows/",
    "IDENTITY.md",
    "PERSONALITY.md",
    "scripts/evolve.sh",
    "scripts/format_issues.py",
    "scripts/build_site.py",
    "skills/self-assess/",
    "skills/evolve/",
    "skills/communicate/",
    "skills/research/",
)
SECTION_STOP_RE = re.compile(
    r"^(?:#+\s*)?(?:title|files|issue|origin|objective|goal|why this matters|"
    r"success criteria|acceptance criteria|verification|test plan|expected evidence)\s*:?\s*",
    re.IGNORECASE,
)


def assessment_alignment(task_text: str, assessment_text: str) -> dict[str, Any]:
    task_lower = task_text.lower()
    assessment_lower = assessment_text.lower()
    contradicted = False
    evidence: list[str] = []

    cold_start_task = "state why last-failure" in task_lower and "no state event found" in task_lower
    cold_start_healthy = "state why last-failure" in assessment_lower and any(
        phrase in assessment_lower
        for phrase in (
            "no failure found",
            "healthy state",
            "shows diagnostic guidance",
            "now properly explains cold-start",
            "no completed failure sessions",
            "correctly reports",
            "returned nothing - meaning no",
            "returned nothing — meaning no",
        )
    )
    if cold_start_task and cold_start_healthy:
        contradicted = True
        evidence.append(
            "task says state why last-failure returns no state event, but assessment reports healthy diagnostic output"
        )

    return {
        "contradicted_by_assessment": contradicted,
        "evidence": evidence,
    }


def normalize_file_entry(value: object) -> str:
    text = " ".join(str(value or "").split()).strip()
    text = text.strip("`'\"")
    text = FILE_ANNOTATION_RE.sub("", text).strip()
    text = FILE_TRAILING_NOTE_RE.sub("", text).strip()
    text = text.strip("`'\" ,")
    if text.endswith(".bak"):
        return ""
    return text


def normalize_file_list(values: list[object]) -> list[str]:
    out: list[str] = []
    for value in values:
        text = normalize_file_entry(value)
        if text and text not in out:
            out.append(text)
    return out


def split_list(value: str) -> list[str]:
    return normalize_file_list(value.replace(";", ",").split(","))


def extract_file_mentions(text: str) -> list[str]:
    return normalize_file_list(match.group(1) for match in FILE_MENTION_RE.finditer(text or ""))


def protected_surface_match(path: str) -> str | None:
    normalized = normalize_file_entry(path).strip("/")
    if not normalized:
        return None
    for protected in PROTECTED_IMPLEMENTATION_SURFACES:
        surface = protected.strip("/")
        if protected.endswith("/"):
            if normalized == surface or normalized.startswith(f"{surface}/"):
                return protected
        elif normalized == surface:
            return protected
    return None


def protected_task_surfaces(task: dict[str, Any]) -> list[str]:
    matches: list[str] = []
    surfaces = task.get("declared_files") or task.get("files") or []
    for path in surfaces:
        if not isinstance(path, str):
            continue
        if protected_surface_match(path) and path not in matches:
            matches.append(path)
    return matches


def read_text(path: Path | None) -> str:
    if path is None or not path.is_file():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def section_summary(text: str, label: str, limit: int = 360) -> str:
    label_re = re.compile(rf"^(?:#+\s*)?{re.escape(label)}\s*:?\s*(.*)$", re.IGNORECASE)
    lines: list[str] = []
    in_section = False
    for raw_line in text.splitlines():
        line = raw_line.strip()
        match = label_re.match(line)
        if match:
            in_section = True
            first = match.group(1).strip()
            if first:
                lines.append(first)
            continue
        if not in_section:
            continue
        if line and SECTION_STOP_RE.match(line):
            break
        if line.startswith("#"):
            break
        if not line:
            continue
        lines.append(re.sub(r"^(?:[-*]|\d+[.)])\s*", "", line))
    return " ".join(" ".join(lines).split())[:limit]


def parse_task(path: Path, task_number: int, assessment_text: str = "") -> dict[str, Any]:
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
    expected_evidence = section_summary(text, "Expected Evidence")
    has_expected_evidence = bool(expected_evidence)
    has_goal = bool(GOAL_RE.search(text))
    generic = (
        fields.get("title", "").strip().lower() == "self-improvement"
        and "identify the most impactful improvement" in lower
    )
    alignment = assessment_alignment(text, assessment_text)
    quality_score = sum([has_success, has_verification, has_expected_evidence, has_goal, not generic]) / 5.0
    if alignment["contradicted_by_assessment"]:
        quality_score = min(quality_score, 0.5)
    title = fields.get("title") or f"Task {task_number}"
    declared_files = normalize_file_list(split_list(fields.get("files", "")))
    files = normalize_file_list(declared_files + extract_file_mentions(text))
    return {
        "task_id": f"task_{task_number:02d}",
        "task_number": task_number,
        "title": title,
        "files": files,
        "declared_files": declared_files,
        "issue": fields.get("issue") or None,
        "origin": fields.get("origin") or "planner",
        "artifact_path": f"tasks/task_{task_number:02d}/task.md",
        "session_plan_path": str(path),
        "expected_evidence": expected_evidence or None,
        "body_preview": " ".join(" ".join(body_lines).split())[:360],
        "quality": {
            "has_goal": has_goal,
            "has_success_criteria": has_success,
            "has_verification": has_verification,
            "has_expected_evidence": has_expected_evidence,
            "generic_self_improvement": generic,
            "assessment_alignment": alignment,
            "score": round(quality_score, 4),
        },
    }


def build_manifest(args: argparse.Namespace) -> dict[str, Any]:
    plan_dir = args.session_plan_dir
    task_paths = (
        sorted(path for path in plan_dir.glob("task_*.md") if TASK_FILE_RE.match(path.name))
        if plan_dir.is_dir()
        else []
    )
    assessment_text = read_text(args.assessment_file)
    tasks = [parse_task(path, index + 1, assessment_text) for index, path in enumerate(task_paths)]
    assessment_missing_text = read_text(getattr(args, "assessment_missing_file", None))
    issue_text = read_text(args.issue_responses_file)
    failure_text = read_text(args.planning_failure_file)
    harness_seeded = [t for t in tasks if isinstance(t.get("origin"), str) and t["origin"] == "harness-seed"]
    planning_failed = bool(
        args.planning_failed
        or failure_text
        or not tasks
        or (tasks and len(harness_seeded) == len(tasks))
    )
    warnings: list[str] = []
    if planning_failed:
        warnings.append("planner_produced_no_task_files")
    if tasks and len(harness_seeded) == len(tasks):
        warnings.append("all_tasks_harness_seeded")
    for task in tasks:
        quality = task.get("quality") if isinstance(task.get("quality"), dict) else {}
        if quality.get("generic_self_improvement"):
            warnings.append(f"{task['task_id']}:generic_self_improvement")
        alignment = quality.get("assessment_alignment") if isinstance(quality, dict) else {}
        if isinstance(alignment, dict) and alignment.get("contradicted_by_assessment"):
            warnings.append(f"{task['task_id']}:assessment_contradiction")
        if not task.get("files"):
            warnings.append(f"{task['task_id']}:missing_files")
        if not quality.get("has_expected_evidence"):
            warnings.append(f"{task['task_id']}:missing_expected_evidence")
        if float(quality.get("score") or 0.0) < 0.75:
            warnings.append(f"{task['task_id']}:thin_task_spec")
        protected = protected_task_surfaces(task)
        if protected:
            task["protected_files"] = protected
            warnings.append(f"{task['task_id']}:protected_files")

    selectable = [task for task in tasks if not task.get("protected_files")]
    selected = selectable[: args.selected_limit]
    if tasks and not selected:
        warnings.append("no_selectable_tasks")

    return {
        "schema_version": 1,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "planner": {
            "planning_failed": planning_failed,
            "task_count": len(tasks),
            "selected_task_count": len(selected),
            "assessment_present": bool(assessment_text.strip()),
            "assessment_missing_present": bool(assessment_missing_text.strip()),
            "issue_responses_present": bool(issue_text.strip()),
            "planning_failure_present": bool(failure_text.strip()),
            "protected_task_count": len(tasks) - len(selectable),
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
    planning_failed = bool(planner.get("planning_failed"))
    task_count = int(planner.get("task_count") or 0)
    selected_task_count = int(planner.get("selected_task_count") or 0)
    no_selectable_tasks = bool(task_count and selected_task_count == 0 and not planning_failed)
    return {
        "phase": "plan",
        "decision_type": "session_plan",
        "decision": "planning_failed" if planning_failed else "no_selectable_tasks" if no_selectable_tasks else "tasks_selected",
        "task_count": task_count,
        "selected_task_count": selected_task_count,
        "assessment_present": bool(planner.get("assessment_present")),
        "planning_failed": planning_failed,
        "reason": (
            "planning phase produced no task files"
            if planning_failed
            else "planning phase produced only protected-file tasks; no implementation task was selected"
            if no_selectable_tasks
            else "planning phase selected implementation tasks for this evolution session"
        ),
        "tasks": [
            {
                "task_id": task.get("task_id"),
                "task_number": task.get("task_number"),
                "task_title": task.get("title"),
                "planned_files": task.get("files") or [],
                "expected_evidence": task.get("expected_evidence"),
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
