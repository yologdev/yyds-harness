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

    gnomes = state_graph_tools.corrected_latest_gnomes(latest)
    graph_pressure = extract_trajectory.render_graph_suggestions(audit_dir)
    selected = int_metric(gnomes, "selected_task_count")
    attempted = int_metric(gnomes, "tasks_attempted")
    provider_errors = int_metric(gnomes, "provider_error_count")
    task_success = float_metric(gnomes, "task_success_rate")
    verification_rate = float_metric(gnomes, "task_verification_rate")
    artifact_coverage = float_metric(gnomes, "task_artifact_coverage")
    lineage_coverage = float_metric(gnomes, "task_lineage_capture_coverage")
    stale_seed_obsolete = int_metric(gnomes, "task_stale_seed_obsolete_note_count")

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
        if task_success is None:
            issues.append("task success rate missing despite selected or attempted task evidence")
        elif task_success >= 1.0:
            if verification_rate != 1.0:
                issues.append(f"task success is complete but verifier rate is {verification_rate}")
        elif "Dominant task failure:" not in graph_pressure:
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
            "graph_pressure_present": bool(graph_pressure),
            "dominant_task_failure_visible": "Dominant task failure:" in graph_pressure,
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
