#!/usr/bin/env python3
"""Write a per-session DeepSeek capability fitness artifact."""

from __future__ import annotations

import argparse
import json
import tempfile
from pathlib import Path
from typing import Any

import gnome_fitness
import state_graph_tools


def load_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8", errors="replace"))
    except (OSError, json.JSONDecodeError):
        return {}
    return data if isinstance(data, dict) else {}


def session_fitness(session_dir: Path) -> dict[str, Any]:
    summary = load_json(session_dir / "state" / "summary.json")
    gnomes = summary.get("latest_gnomes") if isinstance(summary.get("latest_gnomes"), dict) else {}
    readiness = load_json(session_dir / "evo_readiness.json")
    evidence = readiness.get("evidence") if isinstance(readiness.get("evidence"), dict) else {}
    merged = dict(gnomes)
    for key, value in evidence.items():
        if value is not None:
            merged.setdefault(key, value)
    manifest = state_graph_tools.task_manifest(session_dir)
    tasks = state_graph_tools.selected_tasks(manifest)
    artifact_metrics = state_graph_tools.task_artifact_verification_metrics(session_dir)
    for key, value in artifact_metrics.items():
        if value is not None:
            merged.setdefault(key, value)
    fitness = gnome_fitness.fitness_summary(merged)
    return {
        "schema_version": 1,
        "session_id": session_dir.name,
        "goal": fitness["goal"],
        "selected_task_count": len(tasks),
        "readiness_classification": readiness.get("classification"),
        "can_drive_evolution": readiness.get("can_drive_evolution"),
        "fitness": fitness,
        "next_task_rule": (
            "Prefer tasks that name a fitness gnome, a verifier or held-out eval, "
            "and expected lineage/state/fitness.json evidence. Fix diagnostics only when they block measurement."
        ),
    }


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_self_tests() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        session = Path(tmp) / "day-1"
        write_json(
            session / "state/summary.json",
            {
                "latest_gnomes": {
                    "task_success_rate": 1.0,
                    "task_verification_rate": 1.0,
                    "coding_log_score": 0.9,
                    "planner_no_task_count": 0,
                }
            },
        )
        write_json(
            session / "evo_readiness.json",
            {
                "classification": "verified_success",
                "can_drive_evolution": True,
                "evidence": {"task_artifact_coverage": 1.0},
            },
        )
        write_json(
            session / "tasks/manifest.json",
            {"selected_tasks": [{"task_id": "task_01", "files": ["src/state.rs"]}]},
        )
        report = session_fitness(session)
        assert report["fitness"]["fitness_score"] == 0.9667, report
        assert report["selected_task_count"] == 1, report
    print("deepseek_fitness_eval self-tests passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--audit-dir", type=Path, required=True, help="single session audit artifact directory")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--test", action="store_true")
    args = parser.parse_args()
    if args.test:
        return run_self_tests()
    report = session_fitness(args.audit_dir)
    if args.output:
        write_json(args.output, report)
    if args.json or not args.output:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
