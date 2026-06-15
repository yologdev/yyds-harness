#!/usr/bin/env python3
"""Verify whether an evo audit session has proof-quality feedback evidence.

This is intentionally narrower than "the whole system will evolve perfectly".
It checks whether the latest available session evidence is clean and actionable
enough to drive the next DeepSeek/yyds evolution step without relying on a
dashboard-only interpretation.
"""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path
from typing import Any

import extract_trajectory
import state_graph_tools


def numeric(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def int_metric(metrics: dict[str, Any], key: str) -> int:
    value = metrics.get(key)
    if numeric(value):
        return int(value)
    if isinstance(value, str):
        try:
            return int(float(value))
        except ValueError:
            return 0
    return 0


def float_metric(metrics: dict[str, Any], key: str) -> float | None:
    value = metrics.get(key)
    if numeric(value):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value)
        except ValueError:
            return None
    return None


def latest_session(audit_dir: Path) -> Path | None:
    sessions = state_graph_tools.ordered_sessions(audit_dir)
    return sessions[-1] if sessions else None


def artifact_readiness_metrics(session_dir: Path) -> dict[str, Any]:
    """Return readiness-critical metrics from durable session artifacts.

    `evolve.sh` writes evo_readiness.json before the asynchronous log-feedback
    job can add log_feedback.json. Do not depend on feedback enrichment for the
    basic task-evidence contract; derive it from outcome/manifest/task artifacts.
    """
    manifest = state_graph_tools.task_manifest(session_dir)
    tasks = state_graph_tools.selected_tasks(manifest)
    artifacts = state_graph_tools.task_artifact_rows(session_dir)
    outcome = state_graph_tools.load_json(session_dir / "outcome.json")
    summary = state_graph_tools.load_json(session_dir / "state" / "summary.json")
    lineage = state_graph_tools.lineage_rows(summary)
    artifact_metrics = state_graph_tools.task_artifact_verification_metrics(session_dir)

    task_ids = [str(task.get("task_id") or "") for task in tasks if task.get("task_id")]
    selected = len(task_ids)
    artifact_hits = sum(1 for task_id in task_ids if task_id in artifacts)
    lineage_hits = sum(1 for task_id in task_ids if task_id in lineage)
    raw_attempt_rows = sum(
        len(row.get("attempts") or [])
        for row in artifacts.values()
        if isinstance(row, dict)
    )
    final_artifact_pairs = [
        (task, artifacts[str(task.get("task_id") or "")])
        for task in tasks
        if isinstance(artifacts.get(str(task.get("task_id") or "")), dict)
    ]
    attempted = max(
        int_metric(outcome, "tasks_attempted"),
        len(final_artifact_pairs),
    )

    artifact_succeeded = 0
    artifact_mechanical_verified = 0
    for task, row in final_artifact_pairs:
        task_outcome = row.get("outcome") if isinstance(row.get("outcome"), dict) else {}
        evals = row.get("evals") if isinstance(row.get("evals"), list) else []
        passed = state_graph_tools.eval_passed(evals)
        if passed:
            artifact_mechanical_verified += 1
        outcome_status = str(task_outcome.get("status") or "").strip().lower()
        revert_reason = str(task_outcome.get("revert_reason") or "").lower()
        planned = [
            str(path)
            for path in (task_outcome.get("planned_files") or task.get("files") or [])
            if path
        ]
        touched = [
            str(path)
            for path in (
                task_outcome.get("touched_files")
                or task_outcome.get("source_files")
                or []
            )
            if path
        ]
        source_touched = [
            str(path)
            for path in (
                (task_outcome.get("source_files") or [])
                + (task_outcome.get("touched_files") or [])
            )
            if state_graph_tools.source_file(str(path))
        ]
        overlap_ok = (
            not planned
            or not touched
            or state_graph_tools.file_overlap(planned, touched)
        )
        landed_ok = not source_touched or bool(
            task_outcome.get("commit_shas") or task_outcome.get("commits")
        )
        blocked = any(
            marker in revert_reason
            for marker in (
                "api error",
                "modified protected files",
                "do not overlap planned",
                "marked obsolete",
            )
        )
        if outcome_status == "completed" and passed and overlap_ok and landed_ok and not blocked:
            artifact_succeeded += 1

    succeeded = max(int_metric(outcome, "tasks_succeeded"), artifact_succeeded)

    result: dict[str, Any] = {
        "task_manifest_available": bool(manifest),
        "selected_task_count": selected,
        "tasks_attempted": attempted,
        "raw_task_attempt_count": raw_attempt_rows,
    }
    if selected > 0:
        result["task_artifact_coverage"] = artifact_hits / selected
        result["task_lineage_capture_coverage"] = lineage_hits / selected
    if attempted > 0:
        result["task_success_rate"] = max(min(succeeded / attempted, 1.0), 0.0)
        result["task_verification_rate"] = max(min(artifact_succeeded / attempted, 1.0), 0.0)
        result["task_mechanical_verification_rate"] = max(
            min(artifact_mechanical_verified / attempted, 1.0),
            0.0,
        )
    for key in (
        "task_stale_seed_obsolete_note_count",
        "task_incomplete_terminal_count",
        "task_terminal_marker_missing_attempt_count",
        "task_no_edit_revert_count",
        "task_obsolete_count",
        "task_api_error_count",
        "protected_file_revert_count",
        "task_scope_mismatch_count",
    ):
        if key in artifact_metrics:
            result[key] = artifact_metrics[key]
    return result


def merge_artifact_metrics(gnomes: dict[str, Any], artifact_metrics: dict[str, Any]) -> dict[str, Any]:
    merged = dict(gnomes)
    if artifact_metrics.get("task_manifest_available"):
        merged["task_manifest_available"] = True
    merged["selected_task_count"] = max(
        int_metric(merged, "selected_task_count"),
        int_metric(artifact_metrics, "selected_task_count"),
    )
    if artifact_metrics.get("task_manifest_available"):
        merged["tasks_attempted"] = int_metric(artifact_metrics, "tasks_attempted")
    else:
        merged["tasks_attempted"] = max(
            int_metric(merged, "tasks_attempted"),
            int_metric(artifact_metrics, "tasks_attempted"),
        )
    if int_metric(artifact_metrics, "raw_task_attempt_count") > 0:
        merged["raw_task_attempt_count"] = int_metric(artifact_metrics, "raw_task_attempt_count")
    for key in (
        "task_success_rate",
        "task_verification_rate",
        "task_mechanical_verification_rate",
        "task_artifact_coverage",
        "task_lineage_capture_coverage",
    ):
        if float_metric(artifact_metrics, key) is not None and (
            float_metric(merged, key) is None or artifact_metrics.get("task_manifest_available")
        ):
            merged[key] = artifact_metrics[key]
    for key in (
        "task_stale_seed_obsolete_note_count",
        "task_incomplete_terminal_count",
        "task_terminal_marker_missing_attempt_count",
        "task_no_edit_revert_count",
        "task_obsolete_count",
        "task_api_error_count",
        "protected_file_revert_count",
        "task_scope_mismatch_count",
    ):
        if artifact_metrics.get("task_manifest_available"):
            merged[key] = int_metric(artifact_metrics, key)
        else:
            merged[key] = max(int_metric(merged, key), int_metric(artifact_metrics, key))
    return merged


def task_success_pressure_visible(graph_pressure: str) -> bool:
    return (
        "Dominant task failure:" in graph_pressure
        or "Raise verified task success rate" in graph_pressure
        or "task_success_rate=" in graph_pressure
        or "outcome_task_success_rate=" in graph_pressure
    )


def workflow_optional_installs_bounded(repo_root: Path) -> list[str]:
    """Return optional workflow install commands that can block evolution indefinitely."""
    offenders: list[str] = []
    for workflow in (
        repo_root / ".github" / "workflows" / "evolve.yml",
        repo_root / ".github" / "workflows" / "skill-evolve.yml",
    ):
        if not workflow.exists():
            offenders.append(f"{workflow}: missing")
            continue
        for line_number, line in enumerate(workflow.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("cargo install ") and "timeout " not in stripped:
                offenders.append(f"{workflow}:{line_number}: {stripped}")
    return offenders


def readiness_report(audit_dir: Path) -> dict[str, Any]:
    latest = latest_session(audit_dir)
    if latest is None:
        return {
            "classification": "not_ready",
            "can_drive_evolution": False,
            "session_id": None,
            "issues": ["no audit sessions found"],
            "evidence": {},
        }

    gnomes = merge_artifact_metrics(
        state_graph_tools.corrected_latest_gnomes(latest),
        artifact_readiness_metrics(latest),
    )
    graph_pressure = extract_trajectory.render_graph_suggestions(audit_dir)
    selected = int_metric(gnomes, "selected_task_count")
    attempted = int_metric(gnomes, "tasks_attempted")
    provider_errors = int_metric(gnomes, "provider_error_count")
    task_success = float_metric(gnomes, "task_success_rate")
    verification_rate = float_metric(gnomes, "task_verification_rate")
    artifact_coverage = float_metric(gnomes, "task_artifact_coverage")
    lineage_coverage = float_metric(gnomes, "task_lineage_capture_coverage")
    stale_seed_obsolete = int_metric(gnomes, "task_stale_seed_obsolete_note_count")
    incomplete_terminal = int_metric(gnomes, "task_incomplete_terminal_count")
    terminal_marker_missing = int_metric(gnomes, "task_terminal_marker_missing_attempt_count")

    issues: list[str] = []
    warnings: list[str] = []
    evidence_expected = selected > 0 or attempted > 0
    provider_blocked = provider_errors > 0 and not evidence_expected

    if provider_blocked:
        issues.append("provider blocked before task selection or task attempts; task success is not measurable")
    elif evidence_expected:
        if not gnomes.get("task_manifest_available"):
            issues.append("task manifest missing despite selected or attempted task evidence")
        if artifact_coverage != 1.0:
            issues.append(f"task artifact coverage incomplete: {artifact_coverage}")
        if lineage_coverage != 1.0:
            issues.append(f"task lineage capture incomplete: {lineage_coverage}")
        if stale_seed_obsolete:
            issues.append(
                f"stale seed-obsolete note contamination present: {stale_seed_obsolete}"
            )
        if incomplete_terminal:
            warnings.append(
                f"task implementation terminal evidence incomplete for {incomplete_terminal} task artifact(s)"
            )
        if terminal_marker_missing and not incomplete_terminal:
            warnings.append(
                f"implementation terminal marker missing on {terminal_marker_missing} attempt(s); mechanical task proof exists"
            )
        if task_success is None:
            issues.append("task success rate missing despite selected or attempted task evidence")
        elif task_success >= 1.0:
            if verification_rate != 1.0:
                issues.append(f"task success is complete but verifier rate is {verification_rate}")
        elif not task_success_pressure_visible(graph_pressure):
            issues.append("low task success lacks prompt-visible dominant failure pressure")
    else:
        issues.append(
            "no selected or attempted task evidence captured; task success is not measurable"
        )

    if graph_pressure and "## Graph-derived next-task pressure" not in graph_pressure:
        issues.append("graph pressure rendered without expected section heading")

    if provider_blocked:
        classification = "provider_blocked"
    elif not evidence_expected:
        classification = "no_task_evidence"
    elif issues:
        classification = "not_ready"
    elif task_success is not None and task_success >= 1.0:
        classification = "verified_success"
    else:
        classification = "actionable"

    return {
        "classification": classification,
        "can_drive_evolution": classification in {"verified_success", "actionable"},
        "session_id": latest.name,
        "issues": issues,
        "warnings": warnings,
        "evidence": {
            "selected_task_count": selected,
            "tasks_attempted": attempted,
            "provider_error_count": provider_errors,
            "task_success_rate": task_success,
            "task_verification_rate": verification_rate,
            "task_artifact_coverage": artifact_coverage,
            "task_lineage_capture_coverage": lineage_coverage,
            "task_stale_seed_obsolete_note_count": stale_seed_obsolete,
            "task_incomplete_terminal_count": incomplete_terminal,
            "task_terminal_marker_missing_attempt_count": terminal_marker_missing,
            "raw_task_attempt_count": int_metric(gnomes, "raw_task_attempt_count"),
            "graph_pressure_present": bool(graph_pressure),
            "dominant_task_failure_visible": task_success_pressure_visible(graph_pressure),
        },
    }


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True), encoding="utf-8")


def run_self_tests() -> int:
    failures: list[str] = []

    def check(name: str, condition: bool, detail: Any = None) -> None:
        if not condition:
            failures.append(f"{name}: {detail!r}")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        session = root / "day-1"
        write_json(
            session / "state/summary.json",
            {"latest_gnomes": {"provider_error_count": 3}},
        )
        report = readiness_report(root)
        check("provider blocked classified", report["classification"] == "provider_blocked", report)
        check("provider blocked not ready", report["can_drive_evolution"] is False, report)

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        session = root / "day-1"
        write_json(
            session / "state/summary.json",
            {
                "latest_gnomes": {
                    "provider_error_count": 0,
                    "selected_task_count": 0,
                    "tasks_attempted": 0,
                }
            },
        )
        report = readiness_report(root)
        check("no task evidence classified", report["classification"] == "no_task_evidence", report)
        check("no task evidence not ready", report["can_drive_evolution"] is False, report)
        check(
            "no task evidence issue named",
            any("no selected or attempted task evidence" in issue for issue in report["issues"]),
            report,
        )

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        session = root / "day-1"
        write_json(
            session / "state/summary.json",
            {
                "latest_gnomes": {
                    "task_manifest_available": True,
                    "selected_task_count": 1,
                    "tasks_attempted": 1,
                    "task_success_rate": 1.0,
                    "task_verification_rate": 1.0,
                    "task_artifact_coverage": 1.0,
                    "task_lineage_capture_coverage": 1.0,
                }
            },
        )
        report = readiness_report(root)
        check("verified success classified", report["classification"] == "verified_success", report)
        check("verified success ready", report["can_drive_evolution"] is True, report)

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        session = root / "day-1"
        write_json(
            session / "state/summary.json",
            {
                "latest_gnomes": {
                    "task_manifest_available": True,
                    "selected_task_count": 1,
                    "tasks_attempted": 1,
                    "task_success_rate": 0.0,
                    "task_verification_rate": 0.0,
                    "task_artifact_coverage": 1.0,
                    "task_lineage_capture_coverage": 1.0,
                    "task_stale_seed_obsolete_note_count": 1,
                }
            },
        )
        report = readiness_report(root)
        check("stale contamination not ready", report["classification"] == "not_ready", report)
        check(
            "stale contamination issue named",
            any("stale seed-obsolete" in issue for issue in report["issues"]),
            report,
        )

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        session = root / "day-1"
        write_json(
            session / "outcome.json",
            {"day": 1, "tasks_attempted": 1, "tasks_succeeded": 0},
        )
        write_json(
            session / "tasks/manifest.json",
            {
                "planner": {"planning_failed": False, "selected_task_count": 1},
                "tasks": [{"task_id": "task_01", "title": "T", "files": ["src/lib.rs"]}],
            },
        )
        write_json(session / "tasks/task_01/outcome.json", {"status": "reverted"})
        write_json(session / "tasks/task_01/decision.json", {"task_id": "task_01"})
        write_json(
            session / "state/summary.json",
            {
                "latest_gnomes": {},
                "task_lineage": [{"task_id": "task_01"}],
            },
        )
        report = readiness_report(root)
        evidence = report["evidence"]
        check("artifact overlay sees selected task", evidence["selected_task_count"] == 1, report)
        check("artifact overlay sees attempted task", evidence["tasks_attempted"] == 1, report)
        check("artifact overlay sees manifest", "task manifest missing" not in "\n".join(report["issues"]), report)
        check("artifact overlay gives task success", evidence["task_success_rate"] == 0.0, report)
        check("artifact overlay gives artifact coverage", evidence["task_artifact_coverage"] == 1.0, report)
        check("artifact overlay gives lineage coverage", evidence["task_lineage_capture_coverage"] == 1.0, report)

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        session = root / "day-1"
        write_json(
            session / "outcome.json",
            {"day": 1, "tasks_attempted": 1, "tasks_succeeded": 1},
        )
        write_json(
            session / "tasks/manifest.json",
            {
                "planner": {"planning_failed": False, "selected_task_count": 1},
                "tasks": [{"task_id": "task_01", "title": "T", "files": ["src/lib.rs"]}],
            },
        )
        write_json(
            session / "tasks/task_01/outcome.json",
            {
                "status": "completed",
                "planned_files": ["src/lib.rs"],
                "touched_files": ["src/lib.rs"],
                "source_files": ["src/lib.rs"],
                "commit_shas": ["abc1234"],
            },
        )
        write_json(session / "tasks/task_01/decision.json", {"task_id": "task_01"})
        write_json(
            session / "tasks/task_01/eval_attempt_1.json",
            {"status": "pass", "verdict": "Verdict: PASS"},
        )
        (session / "tasks/task_01/attempts.jsonl").write_text(
            "\n".join(
                [
                    json.dumps({"phase": "implementation", "status": "incomplete_no_terminal_evidence"}),
                    json.dumps({"phase": "build_fix", "status": "completed"}),
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        write_json(
            session / "state/summary.json",
            {"latest_gnomes": {}, "task_lineage": [{"task_id": "task_01"}]},
        )
        report = readiness_report(root)
        evidence = report["evidence"]
        check("repaired artifact classified verified", report["classification"] == "verified_success", report)
        check("repaired artifact ready", report["can_drive_evolution"] is True, report)
        check("repaired artifact counts final task attempts", evidence["tasks_attempted"] == 1, report)
        check("repaired artifact preserves raw attempts", evidence["raw_task_attempt_count"] == 2, report)
        check("repaired artifact gives final task success", evidence["task_success_rate"] == 1.0, report)
        check("repaired artifact gives final verification", evidence["task_verification_rate"] == 1.0, report)
        check("terminal gap remains warning", bool(report["warnings"]), report)

    check(
        "readiness accepts generic task success pressure",
        task_success_pressure_visible(
            "## Graph-derived next-task pressure\n"
            "- Raise verified task success rate (task_success_rate=0.0): selected tasks failed"
        ),
    )
    check(
        "optional workflow cargo installs are bounded",
        not workflow_optional_installs_bounded(Path(__file__).resolve().parents[1]),
        workflow_optional_installs_bounded(Path(__file__).resolve().parents[1]),
    )

    if failures:
        print("verify_evo_readiness self-tests failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print("verify_evo_readiness self-tests passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--audit-dir", type=Path, default=Path(".yoyo/session_staging"))
    parser.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    parser.add_argument("--test", action="store_true", help="run self-tests")
    args = parser.parse_args()

    if args.test:
        return run_self_tests()

    report = readiness_report(args.audit_dir)
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"classification: {report['classification']}")
        print(f"can_drive_evolution: {str(report['can_drive_evolution']).lower()}")
        print(f"session_id: {report['session_id']}")
        for issue in report["issues"]:
            print(f"issue: {issue}")
        for warning in report["warnings"]:
            print(f"warning: {warning}")
    return 0 if report["can_drive_evolution"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
