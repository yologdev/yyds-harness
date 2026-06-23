#!/usr/bin/env python3
"""Classify gnome metrics and summarize DeepSeek agent fitness."""

from __future__ import annotations

import argparse
import json
from typing import Any


FITNESS_GNOMES = {
    "task_success_rate",
    "task_verification_rate",
    "task_mechanical_verification_rate",
    "coding_log_score",
    "session_success_rate",
    "workflow_success_rate",
    "retry_success_rate",
    "json_parse_failure_rate",
    "tool_call_malformed_rate",
    "context_miss_rate",
    "repair_loop_count",
    "cost_per_successful_task_usd",
    "latency_per_successful_task_ms",
    "deepseek_cache_hit_ratio",
}

DIAGNOSTIC_GNOMES = {
    "planner_no_task_count",
    "provider_error_count",
    "provider_blocked_session_count",
    "evaluator_timeout_count",
    "evaluator_unverified_count",
    "task_artifact_coverage",
    "task_lineage_capture_coverage",
    "state_capture_coverage",
    "state_operational_capture_coverage",
    "audit_capture_coverage",
    "state_replay_integrity_rate",
    "protected_file_revert_count",
    "task_revert_count",
    "task_unattempted_count",
    "task_unlanded_source_count",
    "tool_error_count",
    "command_timeout_count",
}

LOWER_IS_BETTER = {
    "json_parse_failure_rate",
    "tool_call_malformed_rate",
    "context_miss_rate",
    "repair_loop_count",
}

PRIMARY_FITNESS = (
    "task_success_rate",
    "task_verification_rate",
    "coding_log_score",
    "session_success_rate",
)


def numeric(value: Any) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value)
        except ValueError:
            return None
    return None


def classify_gnome(name: str) -> str:
    if name in FITNESS_GNOMES:
        return "fitness"
    if name in DIAGNOSTIC_GNOMES:
        return "diagnostic"
    return "other"


def normalized_fitness_value(name: str, value: Any) -> float | None:
    number = numeric(value)
    if number is None:
        return None
    if name in LOWER_IS_BETTER:
        return max(0.0, min(1.0, 1.0 - number))
    if name.endswith("_rate") or name.endswith("_score") or name == "deepseek_cache_hit_ratio":
        return max(0.0, min(1.0, number))
    return None


def fitness_summary(gnomes: dict[str, Any]) -> dict[str, Any]:
    fitness: dict[str, Any] = {}
    diagnostics: dict[str, Any] = {}
    for key, value in gnomes.items():
        kind = classify_gnome(str(key))
        if kind == "fitness":
            fitness[str(key)] = value
        elif kind == "diagnostic":
            diagnostics[str(key)] = value

    scored: list[float] = []
    for key in PRIMARY_FITNESS:
        if key in fitness:
            value = normalized_fitness_value(key, fitness[key])
            if value is not None:
                scored.append(value)
    if not scored:
        for key, value in fitness.items():
            normalized = normalized_fitness_value(key, value)
            if normalized is not None:
                scored.append(normalized)

    blockers = [
        key
        for key in (
            "planner_no_task_count",
            "provider_error_count",
            "evaluator_timeout_count",
            "task_artifact_coverage",
            "task_lineage_capture_coverage",
        )
        if key in diagnostics
        and (
            (key.endswith("_coverage") and numeric(diagnostics[key]) != 1.0)
            or (not key.endswith("_coverage") and numeric(diagnostics[key]) and numeric(diagnostics[key]) > 0)
        )
    ]

    return {
        "goal": "improve yyds DeepSeek coding/general-agent capability",
        "fitness_gnomes": fitness,
        "diagnostic_gnomes": diagnostics,
        "fitness_score": round(sum(scored) / len(scored), 4) if scored else None,
        "fitness_metric_count": len(fitness),
        "diagnostic_gate_blockers": blockers,
        "primary_fitness": {key: fitness.get(key) for key in PRIMARY_FITNESS if key in fitness},
    }


def render_markdown(summary: dict[str, Any]) -> str:
    fitness = summary.get("fitness_gnomes") if isinstance(summary.get("fitness_gnomes"), dict) else {}
    diagnostics = summary.get("diagnostic_gnomes") if isinstance(summary.get("diagnostic_gnomes"), dict) else {}
    score = summary.get("fitness_score")
    primary = summary.get("primary_fitness") if isinstance(summary.get("primary_fitness"), dict) else {}
    blockers = summary.get("diagnostic_gate_blockers") if isinstance(summary.get("diagnostic_gate_blockers"), list) else []
    parts = []
    for key in PRIMARY_FITNESS:
        if key in primary:
            parts.append(f"{key}={primary[key]}")
    if not parts:
        for key in ("json_parse_failure_rate", "tool_call_malformed_rate", "context_miss_rate"):
            if key in fitness:
                parts.append(f"{key}={fitness[key]}")
    diag_parts = []
    for key in ("planner_no_task_count", "provider_error_count", "evaluator_timeout_count"):
        if key in diagnostics:
            diag_parts.append(f"{key}={diagnostics[key]}")
    lines = ["## Capability fitness feedback"]
    lines.append(f"- goal: {summary.get('goal')}")
    lines.append(f"- fitness_score: {score if score is not None else 'unknown'}")
    if parts:
        lines.append("- primary fitness: " + ", ".join(parts[:5]))
    if diag_parts:
        lines.append("- diagnostic gates: " + ", ".join(diag_parts[:5]))
    if blockers:
        lines.append("- blocker: diagnostic gate(s) still obscure capability fitness: " + ", ".join(str(item) for item in blockers[:4]))
    else:
        lines.append("- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal")
    return "\n".join(lines)


def run_self_tests() -> int:
    gnomes = {
        "task_success_rate": 2 / 3,
        "task_verification_rate": 2 / 3,
        "coding_log_score": 0.8,
        "planner_no_task_count": 0,
        "task_artifact_coverage": 1.0,
    }
    summary = fitness_summary(gnomes)
    assert summary["fitness_metric_count"] == 3, summary
    assert summary["diagnostic_gate_blockers"] == [], summary
    assert summary["fitness_score"] == round(((2 / 3) + (2 / 3) + 0.8) / 3, 4), summary
    rendered = render_markdown(summary)
    assert "Capability fitness feedback" in rendered, rendered
    assert "task_success_rate" in rendered, rendered
    assert classify_gnome("provider_error_count") == "diagnostic"
    assert classify_gnome("tool_call_malformed_rate") == "fitness"
    print("gnome_fitness self-tests passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--test", action="store_true")
    parser.add_argument("metrics", nargs="?", help="JSON object of gnome metrics")
    args = parser.parse_args()
    if args.test:
        return run_self_tests()
    gnomes = json.loads(args.metrics or "{}")
    summary = fitness_summary(gnomes if isinstance(gnomes, dict) else {})
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(render_markdown(summary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

